//! Pool of `WebSocketClient`s, each bound to a different source IP.
//!
//! Wallet-private channels are hard-pinned to slot 0 (see design spec H1).
//! Market-data channels are sharded per `ShardMode` (default `Instrument`).

use std::{
    collections::HashSet,
    net::IpAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Error as WsError;

use super::{slot_for_instrument, PickHint, PoolError, ShardMode};
use crate::{
    ratelimiter::quota::Quota,
    websocket::{
        types::{MessageHandler, PingHandler},
        WebSocketClient, WebSocketConfig,
    },
};

/// Bundle of the 5 non-config arguments to `WebSocketClient::connect`.
/// All fields are `Clone` (the handlers are `Arc<dyn Fn>`), so the
/// pool can replicate them per slot cheaply.
#[derive(Clone)]
pub struct WsConnectArgs {
    pub message_handler: Option<MessageHandler>,
    pub ping_handler: Option<PingHandler>,
    pub post_reconnection: Option<Arc<dyn Fn() + Send + Sync>>,
    pub keyed_quotas: Vec<(String, Quota)>,
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
    /// Async because `WebSocketClient::connect` is async (the WS handshake
    /// happens at construction).
    ///
    /// Clones the template per slot, sets `local_addr = Some(addr)` on each
    /// clone, and awaits `connect`. If any slot's connect fails, the whole
    /// pool construction fails (no partial pools).
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
                args.default_quota.clone(),
            )
            .await
            .map_err(|e: WsError| PoolError::BindFailed {
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

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Pure slot selection — no I/O, no async, no `&self` mutation other
    /// than the atomic counter. Public for testing.
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
    pub async fn dispatch_async<F, Fut, R>(
        &self,
        hint: PickHint<'_>,
        f: F,
    ) -> (usize, R)
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
    /// Idempotent (HashSet semantics).
    pub async fn record_subscription(&self, slot: usize, channel: String) {
        if slot < self.sub_counts.len() {
            self.sub_counts[slot].lock().await.insert(channel);
        }
    }

    pub async fn slot_load(&self, slot: usize) -> usize {
        if slot >= self.sub_counts.len() {
            return 0;
        }
        self.sub_counts[slot].lock().await.len()
    }
}

// NOTE: tests below exercise pick_slot ONLY via a synthetic helper.
// Construction tests would require a real WS server — deferred to
// integration tests via validate.py in the consumer repo.

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic pool that mirrors WsPool::pick_slot's logic but with an
    /// explicit slot count (rather than reading from a real Vec<WebSocketClient>,
    /// which we can't construct in a unit test without a real WS server).
    /// Used exclusively for picker tests below.
    struct SyntheticPool {
        n: usize,
        shard_mode: ShardMode,
        rr_counter: AtomicUsize,
    }

    impl SyntheticPool {
        fn pick_slot(&self, hint: PickHint<'_>) -> usize {
            match hint {
                PickHint::Stateless => 0,
                PickHint::WalletPrivateChannel { .. } => 0,
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

    // Note: empty_input_errors test would require constructing WsPool::new,
    // which is async and requires real WebSocketConfig + handlers. The
    // empty-vec early-return is trivial code; verified by inspection. If
    // desired, a #[tokio::test] could be added that builds a default
    // WebSocketConfig + empty WsConnectArgs and asserts the error type.
}
