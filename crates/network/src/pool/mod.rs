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

//! Multi-IP source-binding pools for HTTP and WebSocket clients.
//!
//! Provides the building blocks for a fan of clients each bound to a
//! different local IP. This module exports the deterministic
//! instrument-to-slot hash and the pick-hint enums that pool implementations
//! consume.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use siphasher::sip::SipHasher13;

use crate::ratelimiter::quota::Quota;

pub mod http_pool;
pub mod ws_pool;

pub use http_pool::HttpPool;
pub use ws_pool::{WsConnectArgs, WsPool};

/// Hint that callers pass into a pool to influence which slot is picked.
///
/// `Stateless` requests can use any slot — round-robin gives the best
/// rate-limit amortisation. `WalletPrivateChannel` subscriptions are
/// pinned to slot 0 so the WS broker doesn't fan authentication across
/// every slot. `MarketDataChannel` is sharded per [`ShardMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickHint<'a> {
    /// Stateless REST — round-robin via internal counter.
    Stateless,
    /// HL channels keyed by wallet address (`userFills`, `userFundings`,
    /// `webData2`, `orderUpdates`). Pin to slot 0 to avoid duplicate
    /// delivery across IPs.
    WalletPrivateChannel {
        /// Channel name carried for logging/metrics.
        channel: &'a str,
    },
    /// Public market-data channels. Sharded per [`ShardMode`].
    ///
    /// `instrument` MUST be the raw venue symbol (e.g. HL `"BTC"`), NOT
    /// the NT instrument ID (e.g. `"BTC-USD-PERP.HYPERLIQUID"`) — the NT
    /// format has changed across NT versions and would silently re-shard.
    MarketDataChannel {
        /// Raw venue symbol.
        instrument: &'a str,
        /// Channel name (e.g. `"l2Book@BTC"`).
        channel: &'a str,
    },
}

/// How a pool decides which slot to use for a market-data request when no
/// explicit pin is in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardMode {
    /// Default. Per-instrument bucketing via [`slot_for_instrument`].
    /// Keeps all of an instrument's channels co-located.
    Instrument,
    /// `AtomicUsize` counter. No locality. Escape hatch.
    RoundRobin,
}

/// Errors returned by pool construction and pick paths.
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// The pool was constructed with no slots.
    #[error("pool constructed with zero IPs")]
    Empty,
    /// A configured IP string could not be parsed.
    #[error("invalid IP address '{input}': {reason}")]
    InvalidAddress {
        /// The malformed input.
        input: String,
        /// The parse error.
        reason: String,
    },
    /// Building a client bound to the requested local IP failed.
    #[error("failed to bind to {address}: {cause}")]
    BindFailed {
        /// The local IP that failed to bind.
        address: std::net::IpAddr,
        /// Underlying error from the HTTP client builder.
        cause: String,
    },
}

/// Template carrying every argument `HttpClient::new_with_local_addr` takes
/// except `local_addr` itself, which is filled per-slot by the pool.
///
/// Adapters typically pre-build a template once with their default headers
/// and rate-limit quota, then hand it to the pool which fans it out across
/// every configured IP.
#[derive(Debug, Clone, Default)]
pub struct HttpClientTemplate {
    /// Static default headers (e.g. `User-Agent`).
    pub headers: HashMap<String, String>,
    /// Header keys to pre-intern (for rate-limit response parsing).
    pub header_keys: Vec<String>,
    /// Per-endpoint rate-limit quotas keyed by endpoint name.
    pub keyed_quotas: Vec<(String, Quota)>,
    /// Default quota applied when no keyed quota matches.
    pub default_quota: Option<Quota>,
    /// Optional overall request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Optional forward HTTP proxy URL.
    pub proxy_url: Option<String>,
}

/// Returns the slot index for an instrument, deterministic across process restarts.
///
/// Uses fixed-key SipHash-1-3 (key `(0, 0)`). The reason for the fixed key
/// (not `DefaultHasher`) is **determinism across process restarts**: the same
/// instrument MUST land on the same slot every time the bot starts, so
/// operators can reason about "BTC sits on IP X" and the slot-0 wallet-private
/// pin remains stable. `DefaultHasher` uses a per-process random seed and
/// would re-shuffle the instrument-to-IP mapping on every restart.
///
/// We are not protecting against hash-collision attacks here (no adversarial
/// input); the `(0, 0)` key is the conventional "no secret" sentinel for
/// deterministic `SipHash`.
///
/// # Panics
///
/// Panics if `n_slots == 0`. Callers MUST validate the pool is non-empty
/// before calling.
#[must_use]
pub fn slot_for_instrument(instrument: &str, n_slots: usize) -> usize {
    assert!(n_slots > 0, "slot_for_instrument called with n_slots=0");
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    instrument.as_bytes().hash(&mut hasher);
    (hasher.finish() as usize) % n_slots
}

#[cfg(test)]
mod type_tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    #[test]
    fn pickhint_variants_construct() {
        let _ = PickHint::Stateless;
        let _ = PickHint::WalletPrivateChannel {
            channel: "userFills",
        };
        let _ = PickHint::MarketDataChannel {
            instrument: "BTC",
            channel: "l2Book@BTC",
        };
    }

    #[test]
    fn shardmode_variants_construct() {
        let _ = ShardMode::Instrument;
        let _ = ShardMode::RoundRobin;
    }

    #[test]
    fn pool_error_variants_construct() {
        let _ = PoolError::Empty;
        let _ = PoolError::InvalidAddress {
            input: "x".into(),
            reason: "y".into(),
        };
        let _ = PoolError::BindFailed {
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            cause: "z".into(),
        };
    }

    #[test]
    fn http_client_template_default() {
        let _ = HttpClientTemplate::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_for_instrument_deterministic() {
        let a = slot_for_instrument("BTC", 3);
        let b = slot_for_instrument("BTC", 3);
        assert_eq!(a, b);
    }

    #[test]
    fn slot_for_instrument_known_values() {
        // Hardcoded so a future hash-function change fails this test loudly.
        // If you genuinely need to change the hash, update these AND bump
        // a major version since all operators' instrument-to-IP mapping shifts.
        assert_eq!(slot_for_instrument("BTC", 3), 2);
        assert_eq!(slot_for_instrument("ETH", 3), 1);
        assert_eq!(slot_for_instrument("SOL", 3), 0);
        assert_eq!(slot_for_instrument("BTC", 1), 0);
    }

    #[test]
    #[should_panic(expected = "n_slots=0")]
    fn slot_for_instrument_panics_on_zero_slots() {
        let _ = slot_for_instrument("BTC", 0);
    }
}
