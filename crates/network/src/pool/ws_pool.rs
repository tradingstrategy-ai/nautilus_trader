// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Pool of [`WebSocketClient`]s, each bound to a different source IP.
//!
//! Wallet-private channels are hard-pinned to slot 0 — see design rationale
//! below. Market-data channels are sharded per [`ShardMode`] (default
//! [`ShardMode::Instrument`]).

use std::{
    collections::HashSet,
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::Mutex;

use super::{PickHint, PoolError, ShardMode, slot_for_instrument};
use crate::{
    ratelimiter::quota::Quota,
    transport::TransportError,
    websocket::{
        WebSocketClient, WebSocketConfig,
        types::{MessageHandler, PingHandler},
    },
};

/// Bundle of the 5 non-config arguments to [`WebSocketClient::connect`].
///
/// All fields are `Clone` (the handlers are `Arc<dyn Fn>`), so the
/// pool can replicate them per slot cheaply.
#[derive(Clone)]
pub struct WsConnectArgs {
    /// Optional handler invoked for each incoming message.
    pub message_handler: Option<MessageHandler>,
    /// Optional handler invoked for each incoming ping frame.
    pub ping_handler: Option<PingHandler>,
    /// Optional callback fired after a successful reconnect.
    pub post_reconnection: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Per-endpoint rate-limit quotas.
    pub keyed_quotas: Vec<(String, Quota)>,
    /// Default rate-limit quota.
    pub default_quota: Option<Quota>,
}

impl std::fmt::Debug for WsConnectArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsConnectArgs")
            .field("message_handler", &self.message_handler.is_some())
            .field("ping_handler", &self.ping_handler.is_some())
            .field("post_reconnection", &self.post_reconnection.is_some())
            .field("keyed_quotas_count", &self.keyed_quotas.len())
            .field("default_quota", &self.default_quota.is_some())
            .finish()
    }
}

/// A fan of [`WebSocketClient`]s sharing the same template, each bound to
/// a distinct local source IP.
///
/// Wallet-private channels (e.g. HL `userFills`, `webData2`) are
/// hard-pinned to slot 0 to prevent duplicate event delivery across IPs.
/// If a wallet-private channel were spread across multiple slots, the
/// venue would deliver the same event N times — once per IP that
/// subscribed. Operators need exactly-once semantics for wallet events.
#[derive(Debug)]
pub struct WsPool {
    slots: Vec<WebSocketClient>,
    shard_mode: ShardMode,
    rr_counter: AtomicUsize,
    /// Per-slot subscription tracking for observability. Not consulted
    /// during dispatch.
    sub_counts: Vec<Mutex<HashSet<String>>>,
}

impl WsPool {
    /// Build one [`WebSocketClient`] per address, each bound to a distinct
    /// local source IP, using the same template.
    ///
    /// Async because [`WebSocketClient::connect`] is async (the WS handshake
    /// happens at construction). Clones the template per slot, sets
    /// `local_addr = Some(addr)` on each clone, and awaits `connect`. If any
    /// slot's connect fails, the whole pool construction fails (no partial
    /// pools).
    ///
    /// # Errors
    ///
    /// - Returns [`PoolError::Empty`] if `addresses` is empty.
    /// - Returns [`PoolError::BindFailed`] if any slot's
    ///   [`WebSocketClient::connect`] fails.
    pub async fn new(
        template: WebSocketConfig,
        args: WsConnectArgs,
        addresses: Vec<IpAddr>,
        shard_mode: ShardMode,
    ) -> Result<Self, PoolError> {
        if addresses.is_empty() {
            return Err(PoolError::Empty);
        }
        let mut slots = Vec::with_capacity(addresses.len());
        let mut sub_counts = Vec::with_capacity(addresses.len());
        for addr in addresses {
            let mut config = template.clone();
            config.local_addr = Some(addr);
            let client = WebSocketClient::connect(
                config,
                args.message_handler.clone(),
                args.ping_handler.clone(),
                args.post_reconnection.clone(),
                args.keyed_quotas.clone(),
                args.default_quota,
            )
            .await
            .map_err(|e: TransportError| PoolError::BindFailed {
                address: addr,
                cause: e.to_string(),
            })?;
            slots.push(client);
            sub_counts.push(Mutex::new(HashSet::new()));
        }
        Ok(Self {
            slots,
            shard_mode,
            rr_counter: AtomicUsize::new(0),
            sub_counts,
        })
    }

    /// Number of slots in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the pool has no slots.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Pure slot selection — no I/O, no async, no `&self` mutation other
    /// than the atomic counter. Public for testing.
    ///
    /// [`PickHint::Stateless`] is invalid here ([`WsPool`] is only for
    /// stateful subscriptions); it trips a `debug_assert!` and falls back
    /// to slot 0 in release builds.
    #[must_use]
    pub fn pick_slot(&self, hint: PickHint<'_>) -> usize {
        match hint {
            PickHint::Stateless => {
                debug_assert!(false, "WsPool got Stateless hint");
                0
            }
            PickHint::WalletPrivateChannel { .. } => 0,
            PickHint::MarketDataChannel { instrument, .. } => match self.shard_mode {
                ShardMode::Instrument => slot_for_instrument(instrument, self.slots.len()),
                ShardMode::RoundRobin => {
                    self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.slots.len()
                }
            },
        }
    }

    /// Async dispatch. Returns `(slot, R)`.
    ///
    /// # Safety-critical non-feature: no auto-failover for slot 0
    ///
    /// If slot 0 dies, all wallet-private subscriptions die. **DO NOT** add
    /// a fall-through to slot 1 — that would risk subscribing the same
    /// wallet-private channel from two IPs and getting duplicate event
    /// delivery (the very scenario the pin exists to prevent).
    pub async fn dispatch_async<F, Fut, R>(&self, hint: PickHint<'_>, f: F) -> (usize, R)
    where
        F: FnOnce(&WebSocketClient) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        debug_assert!(
            !matches!(hint, PickHint::Stateless),
            "WsPool got Stateless hint"
        );
        let slot = self.pick_slot(hint);
        let result = f(&self.slots[slot]).await;
        (slot, result)
    }

    /// Bookkeeping: record a successful subscription on a slot.
    /// Caller is responsible for calling this AFTER a successful dispatch.
    /// Idempotent (`HashSet` semantics).
    pub async fn record_subscription(&self, slot: usize, channel: String) {
        if slot < self.sub_counts.len() {
            self.sub_counts[slot].lock().await.insert(channel);
        }
    }

    /// How many distinct channels are subscribed on `slot`.
    pub async fn slot_load(&self, slot: usize) -> usize {
        if slot >= self.sub_counts.len() {
            return 0;
        }
        self.sub_counts[slot].lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic pool that mirrors `WsPool::pick_slot`'s logic but with an
    /// explicit slot count (rather than reading from a real
    /// `Vec<WebSocketClient>`, which we can't construct in a unit test
    /// without a real WS server). Used exclusively for picker tests below.
    struct SyntheticPool {
        n: usize,
        shard_mode: ShardMode,
        rr_counter: AtomicUsize,
    }

    impl SyntheticPool {
        fn pick_slot(&self, hint: PickHint<'_>) -> usize {
            match hint {
                PickHint::Stateless | PickHint::WalletPrivateChannel { .. } => 0,
                PickHint::MarketDataChannel { instrument, .. } => match self.shard_mode {
                    ShardMode::Instrument => slot_for_instrument(instrument, self.n),
                    ShardMode::RoundRobin => {
                        self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.n
                    }
                },
            }
        }
    }

    fn synthetic_pool(n: usize, shard_mode: ShardMode) -> SyntheticPool {
        SyntheticPool {
            n,
            shard_mode,
            rr_counter: AtomicUsize::new(0),
        }
    }

    #[test]
    fn wallet_private_pins_slot_0_for_all_known_channels() {
        for n in [1, 2, 3, 5, 10] {
            for mode in [ShardMode::Instrument, ShardMode::RoundRobin] {
                let pool = synthetic_pool(n, mode);
                for chan in ["userFills", "userFundings", "webData2", "orderUpdates"] {
                    let s = pool.pick_slot(PickHint::WalletPrivateChannel { channel: chan });
                    assert_eq!(s, 0, "{chan} must pin to slot 0 (n={n}, mode={mode:?})");
                }
            }
        }
    }

    #[test]
    fn instrument_mode_same_symbol_same_slot() {
        let pool = synthetic_pool(3, ShardMode::Instrument);
        let s1 = pool.pick_slot(PickHint::MarketDataChannel {
            instrument: "BTC",
            channel: "l2Book@BTC",
        });
        let s2 = pool.pick_slot(PickHint::MarketDataChannel {
            instrument: "BTC",
            channel: "trades@BTC",
        });
        let s3 = pool.pick_slot(PickHint::MarketDataChannel {
            instrument: "BTC",
            channel: "candle@BTC/4h",
        });
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }

    #[test]
    fn round_robin_mode_advances_counter() {
        let pool = synthetic_pool(3, ShardMode::RoundRobin);
        let slots: Vec<_> = (0..9)
            .map(|_| {
                pool.pick_slot(PickHint::MarketDataChannel {
                    instrument: "BTC",
                    channel: "l2Book@BTC",
                })
            })
            .collect();
        assert_eq!(slots, vec![0, 1, 2, 0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn instrument_mode_reasonable_distribution_over_100_symbols() {
        let pool = synthetic_pool(3, ShardMode::Instrument);
        let mut counts = [0usize; 3];
        for i in 0..100 {
            let name = format!("SYM{i}");
            let s = pool.pick_slot(PickHint::MarketDataChannel {
                instrument: &name,
                channel: "l2Book",
            });
            counts[s] += 1;
        }
        let max = *counts.iter().max().unwrap();
        assert!(
            max < 67,
            "max slot load {max} > 2x average: counts {counts:?}"
        );
    }
}
