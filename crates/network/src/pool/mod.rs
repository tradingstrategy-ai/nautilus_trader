//! Multi-IP source-binding pools for HTTP and WebSocket clients.
//!
//! See `docs/superpowers/specs/2026-05-21-multi-ip-rest-ws-pool-design.md`
//! in the nautilus-strategies repo for the design rationale.

pub mod http_pool;
pub use http_pool::HttpPool;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use siphasher::sip::SipHasher13;

use crate::ratelimiter::quota::Quota;

#[derive(Debug, Clone, Copy)]
pub enum PickHint<'a> {
    /// Stateless REST — round-robin via internal counter.
    Stateless,
    /// HL channels keyed by wallet address (userFills, userFundings,
    /// webData2, orderUpdates). Pin to slot 0 to avoid duplicate delivery
    /// across IPs (see spec Open Question H1).
    WalletPrivateChannel { channel: &'a str },
    /// Public market-data channels. Sharded per `ShardMode`.
    ///
    /// `instrument` MUST be the raw HL symbol ("BTC"), NOT the NT
    /// instrument ID ("BTC-USD-PERP.HYPERLIQUID") — the NT format has
    /// changed across NT versions and would silently re-shard.
    MarketDataChannel { instrument: &'a str, channel: &'a str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardMode {
    /// Default. Per-instrument bucketing via slot_for_instrument.
    /// Keeps all of an instrument's channels co-located.
    Instrument,
    /// AtomicUsize counter. No locality. Escape hatch.
    RoundRobin,
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("pool constructed with zero IPs")]
    Empty,
    #[error("invalid IP address '{input}': {reason}")]
    InvalidAddress { input: String, reason: String },
    #[error("failed to bind to {address}: {cause}")]
    BindFailed { address: std::net::IpAddr, cause: String },
}

/// Template carrying everything `HttpClient::new_with_local_addr` takes
/// except `local_addr` itself, which is filled per-slot by `HttpPool::new`.
/// HL adapter passes its `Self::default_headers()` and `HYPERLIQUID_REST_QUOTA`
/// via this template to preserve HL-specific behavior across all pool slots.
#[derive(Debug, Clone, Default)]
pub struct HttpClientTemplate {
    pub headers: HashMap<String, String>,
    pub header_keys: Vec<String>,
    pub keyed_quotas: Vec<(String, Quota)>,
    pub default_quota: Option<Quota>,
    pub timeout_secs: Option<u64>,
    pub proxy_url: Option<String>,
}

/// Returns the slot index for an instrument, deterministic across process restarts.
///
/// Uses fixed-key SipHash-1-3 (key (0, 0)). The reason for the fixed key (not
/// `DefaultHasher`) is **determinism across process restarts**: the same
/// instrument MUST land on the same slot every time the bot starts, so
/// operators can reason about "BTC sits on IP X" and the slot-0 wallet-private
/// pin remains stable. `DefaultHasher` uses a per-process random seed and
/// would re-shuffle the instrument-to-IP mapping on every restart.
///
/// We are not protecting against hash-collision attacks here (no adversarial
/// input); the (0, 0) key is the conventional "no secret" sentinel for
/// deterministic SipHash.
///
/// # Panics
///
/// Panics if `n_slots == 0`. Callers MUST validate the pool is non-empty
/// before calling.
pub fn slot_for_instrument(instrument: &str, n_slots: usize) -> usize {
    assert!(n_slots > 0, "slot_for_instrument called with n_slots=0");
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    instrument.as_bytes().hash(&mut hasher);
    (hasher.finish() as usize) % n_slots
}

#[cfg(test)]
mod type_tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn pickhint_variants_construct() {
        let _h1 = PickHint::Stateless;
        let _h2 = PickHint::WalletPrivateChannel { channel: "userFills" };
        let _h3 = PickHint::MarketDataChannel { instrument: "BTC", channel: "l2Book@BTC" };
    }

    #[test]
    fn shardmode_variants_construct() {
        let _ = ShardMode::Instrument;
        let _ = ShardMode::RoundRobin;
    }

    #[test]
    fn pool_error_variants_construct() {
        let _e1 = PoolError::Empty;
        let _e2 = PoolError::InvalidAddress { input: "x".into(), reason: "y".into() };
        let _e3 = PoolError::BindFailed {
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            cause: "z".into(),
        };
    }

    #[test]
    fn http_client_template_default() {
        let _t = HttpClientTemplate::default();
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
        // Values discovered on first run; fill in after Step 2.
        assert_eq!(slot_for_instrument("BTC", 3), 2);
        assert_eq!(slot_for_instrument("ETH", 3), 1);
        assert_eq!(slot_for_instrument("SOL", 3), 0);
        assert_eq!(slot_for_instrument("BTC", 1), 0);
    }

    #[test]
    #[should_panic(expected = "n_slots=0")]
    fn slot_for_instrument_panics_on_zero_slots() {
        slot_for_instrument("BTC", 0);
    }
}
