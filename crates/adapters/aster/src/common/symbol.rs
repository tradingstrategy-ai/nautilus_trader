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

//! Aster symbol conversion utilities.

use nautilus_model::identifiers::InstrumentId;
use ustr::Ustr;

use super::{consts::ASTER_VENUE, enums::AsterProductType};

/// Converts a Aster symbol to a Nautilus instrument ID.
///
/// For USD-M futures, appends "-PERP" suffix to match Nautilus symbology.
/// For COIN-M futures, keeps the symbol as-is (uses "_PERP" format).
///
/// # Examples
///
/// - ("BTCUSDT", UsdM) → "BTCUSDT-PERP.ASTER"
/// - ("ETHUSD_PERP", CoinM) → "ETHUSD_PERP.ASTER"
#[must_use]
pub fn format_instrument_id(symbol: &Ustr, product_type: AsterProductType) -> InstrumentId {
    let nautilus_symbol = match product_type {
        AsterProductType::UsdM => {
            // USD-M symbols don't have _PERP suffix from Aster, we add -PERP
            format!("{symbol}-PERP")
        }
        AsterProductType::CoinM => {
            // COIN-M symbols already have _PERP suffix from Aster
            symbol.to_string()
        }
        _ => symbol.to_string(),
    };
    InstrumentId::new(nautilus_symbol.into(), *ASTER_VENUE)
}

/// Converts a Nautilus instrument ID to a Aster-compatible symbol.
///
/// This function strips common suffixes like "-PERP" that Nautilus uses for
/// internal symbology but Aster doesn't recognize.
///
/// # Examples
///
/// - "BTCUSDT-PERP" → "BTCUSDT"
/// - "ETHUSD_PERP" → "ETHUSD_PERP" (COIN-M format, kept as-is)
/// - "BTCUSDT" → "BTCUSDT"
#[must_use]
pub fn format_aster_symbol(instrument_id: &InstrumentId) -> String {
    let symbol = instrument_id.symbol.as_str();

    if symbol.ends_with("-PERP") {
        symbol.trim_end_matches("-PERP").to_string()
    } else {
        symbol.to_string()
    }
}

/// Converts a Nautilus instrument ID to a lowercase Aster WebSocket stream symbol.
///
/// This is used for constructing WebSocket stream names which require lowercase symbols.
#[must_use]
pub fn format_aster_stream_symbol(instrument_id: &InstrumentId) -> String {
    format_aster_symbol(instrument_id).to_lowercase()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("BTCUSDT-PERP.ASTER", "BTCUSDT")]
    #[case("ETHUSDT-PERP.ASTER", "ETHUSDT")]
    #[case("BTCUSD_PERP.ASTER", "BTCUSD_PERP")]
    #[case("BTCUSDT.ASTER", "BTCUSDT")]
    #[case("ETHBTC.ASTER", "ETHBTC")]
    fn test_format_aster_symbol(#[case] input: &str, #[case] expected: &str) {
        let instrument_id = InstrumentId::from(input);
        assert_eq!(format_aster_symbol(&instrument_id), expected);
    }

    #[rstest]
    #[case("BTCUSDT-PERP.ASTER", "btcusdt")]
    #[case("ETHUSDT-PERP.ASTER", "ethusdt")]
    #[case("BTCUSD_PERP.ASTER", "btcusd_perp")]
    fn test_format_aster_stream_symbol(#[case] input: &str, #[case] expected: &str) {
        let instrument_id = InstrumentId::from(input);
        assert_eq!(format_aster_stream_symbol(&instrument_id), expected);
    }
}
