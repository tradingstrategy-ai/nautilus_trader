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

//! HL WebSocket channel taxonomy.
//!
//! HL splits its WS channels into two categories:
//!
//! - **Wallet-private channels** (`userFills`, `userFundings`, `webData2`,
//!   `orderUpdates`) — keyed by wallet address. Subscribing from multiple
//!   IPs risks duplicate delivery; the WsPool hard-pins these to slot 0.
//!
//! - **Public market-data channels** — form `<channel_type>@<instrument>`
//!   or `<channel_type>@<instrument>/<extra>`. Sharded across pool slots.
//!
//! # Safety: do NOT add auto-failover for slot 0
//!
//! The slot-0 pin is a correctness property. A future "smart failover"
//! patch would risk the very double-delivery scenario the pin prevents.
//! See the design spec in the nautilus-strategies repo at
//! `docs/superpowers/specs/2026-05-21-multi-ip-rest-ws-pool-design.md`.

use nautilus_model::identifiers::InstrumentId;

pub const WALLET_PRIVATE_CHANNELS: &[&str] = &[
    "userFills",
    "userFundings",
    "webData2",
    "orderUpdates",
];

/// Extract the raw HL symbol from an NT `InstrumentId`.
///
/// Examples:
///
/// | Input                              | Output    |
/// |------------------------------------|-----------|
/// | `"BTC-USD-PERP.HYPERLIQUID"`       | `"BTC"`   |
/// | `"ETH-USD-PERP.HYPERLIQUID"`       | `"ETH"`   |
/// | `"@107-USD-SPOT.HYPERLIQUID"`      | `"@107"`  |
///
/// The pool hashes on this raw symbol (NOT the NT instrument ID) so that
/// NT version bumps (which change the ID format) don't silently re-shard.
pub fn extract_hl_symbol(instrument_id: &InstrumentId) -> String {
    let s = instrument_id.symbol.as_str();
    s.split('-').next().unwrap_or(s).to_string()
}

/// Parse an HL WS channel string into `(channel_type, instrument)`.
///
/// Used by `validate.py` and any callers that work in raw HL strings.
/// The HL WS adapter itself uses typed methods that don't need this.
///
/// Examples:
///
/// | Input              | Output                       |
/// |--------------------|------------------------------|
/// | `"l2Book@BTC"`     | `("l2Book", Some("BTC"))`    |
/// | `"candle@SOL/4h"`  | `("candle", Some("SOL"))`    |
/// | `"userFills"`      | `("userFills", None)`        |
/// | `"trades@@ETH"`    | `("trades", Some("@ETH"))`   |
/// | `""`               | `("", None)`                 |
///
/// `candle@<sym>/<interval>` strips the `/<interval>` suffix. Double-`@`
/// splits on the first `@` (HL server rejects malformed channels at
/// protocol level, so the pool just hashes the bytes uniformly and
/// returns *some* slot deterministically). Empty strings are the caller's
/// responsibility to reject.
#[must_use]
pub fn parse_hl_channel(channel: &str) -> (&str, Option<&str>) {
    match channel.split_once('@') {
        None => (channel, None),
        Some((channel_type, rest)) => {
            let instrument = rest.split('/').next().unwrap_or(rest);
            (channel_type, Some(instrument))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_private_constant_has_four() {
        // Regression: if HL adds a new wallet-private channel, this
        // constant must grow to match. Update test + audit pool behavior.
        assert_eq!(WALLET_PRIVATE_CHANNELS.len(), 4);
    }

    #[test]
    fn parses_l2book() {
        assert_eq!(parse_hl_channel("l2Book@BTC"), ("l2Book", Some("BTC")));
    }

    #[test]
    fn parses_candle_strips_interval() {
        assert_eq!(parse_hl_channel("candle@SOL/4h"), ("candle", Some("SOL")));
    }

    #[test]
    fn parses_wallet_private_no_at() {
        for c in WALLET_PRIVATE_CHANNELS {
            assert_eq!(parse_hl_channel(c), (*c, None));
        }
    }

    #[test]
    fn parses_double_at_splits_on_first() {
        assert_eq!(parse_hl_channel("trades@@ETH"), ("trades", Some("@ETH")));
    }

    #[test]
    fn parses_empty_string() {
        assert_eq!(parse_hl_channel(""), ("", None));
    }

    #[test]
    fn extract_hl_symbol_strips_suffix() {
        let id = InstrumentId::from("BTC-USD-PERP.HYPERLIQUID");
        assert_eq!(extract_hl_symbol(&id), "BTC");
    }

    #[test]
    fn extract_hl_symbol_handles_spot_at_prefix() {
        let id = InstrumentId::from("@107-USD-SPOT.HYPERLIQUID");
        assert_eq!(extract_hl_symbol(&id), "@107");
    }
}
