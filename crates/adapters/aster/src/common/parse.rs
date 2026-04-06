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

//! Parsing utilities for Aster API responses.
//!
//! Provides conversion functions to transform raw Aster exchange data
//! into Nautilus domain objects such as instruments and market data.

use std::str::FromStr;

use anyhow::Context;
use nautilus_core::nanos::UnixNanos;
use nautilus_model::{
    data::BarSpecification,
    enums::BarAggregation,
    identifiers::{InstrumentId, Symbol, Venue},
    instruments::{
        any::InstrumentAny, crypto_perpetual::CryptoPerpetual,
    },
    types::{Currency, Price, Quantity},
};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::{
    common::{
        consts::ASTER,
        enums::{AsterContractStatus, AsterKlineInterval, AsterTradingStatus},
    },
    futures::http::models::{AsterFuturesCoinSymbol, AsterFuturesUsdSymbol},
};
const CONTRACT_TYPE_PERPETUAL: &str = "PERPETUAL";

/// Returns a currency from the internal map or creates a new crypto currency.
pub fn get_currency(code: &str) -> Currency {
    Currency::get_or_create_crypto(code)
}

/// Extracts filter values from Aster symbol filters array.
fn get_filter<'a>(filters: &'a [Value], filter_type: &str) -> Option<&'a Value> {
    filters.iter().find(|f| {
        f.get("filterType")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t == filter_type)
    })
}

/// Parses a string field from a JSON value.
fn parse_filter_string(filter: &Value, field: &str) -> anyhow::Result<String> {
    filter
        .get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("Missing field '{field}' in filter"))
}

/// Parses a Price from a filter field.
fn parse_filter_price(filter: &Value, field: &str) -> anyhow::Result<Price> {
    let value = parse_filter_string(filter, field)?;
    Price::from_str(&value).map_err(|e| anyhow::anyhow!("Failed to parse {field}='{value}': {e}"))
}

/// Parses a Quantity from a filter field.
fn parse_filter_quantity(filter: &Value, field: &str) -> anyhow::Result<Quantity> {
    let value = parse_filter_string(filter, field)?;
    Quantity::from_str(&value)
        .map_err(|e| anyhow::anyhow!("Failed to parse {field}='{value}': {e}"))
}

/// Parses a USD-M Futures symbol definition into a Nautilus CryptoPerpetual instrument.
///
/// # Errors
///
/// Returns an error if:
/// - Required filter values are missing (PRICE_FILTER, LOT_SIZE).
/// - Price or quantity values cannot be parsed.
/// - The contract type is not PERPETUAL.
pub fn parse_usdm_instrument(
    symbol: &AsterFuturesUsdSymbol,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
) -> anyhow::Result<InstrumentAny> {
    // Only handle perpetual contracts for now
    if symbol.contract_type != CONTRACT_TYPE_PERPETUAL {
        anyhow::bail!(
            "Unsupported contract type '{}' for symbol '{}', expected '{}'",
            symbol.contract_type,
            symbol.symbol,
            CONTRACT_TYPE_PERPETUAL
        );
    }

    if symbol.status != AsterTradingStatus::Trading {
        anyhow::bail!(
            "Symbol '{}' is not trading (status: {:?})",
            symbol.symbol,
            symbol.status
        );
    }

    let base_currency = get_currency(symbol.base_asset.as_str());
    let quote_currency = get_currency(symbol.quote_asset.as_str());
    let settlement_currency = get_currency(symbol.margin_asset.as_str());

    let instrument_id = InstrumentId::new(
        Symbol::from_str_unchecked(format!("{}-PERP", symbol.symbol)),
        Venue::new(ASTER),
    );
    let raw_symbol = Symbol::new(symbol.symbol.as_str());

    let price_filter = get_filter(&symbol.filters, "PRICE_FILTER")
        .context("Missing PRICE_FILTER in symbol filters")?;

    let tick_size = parse_filter_price(price_filter, "tickSize")?;
    if tick_size.is_zero() {
        anyhow::bail!(
            "Invalid tickSize of 0 for symbol '{}', cannot create instrument",
            symbol.symbol,
        );
    }
    let max_price = parse_filter_price(price_filter, "maxPrice").ok();
    let min_price = parse_filter_price(price_filter, "minPrice").ok();

    let lot_filter =
        get_filter(&symbol.filters, "LOT_SIZE").context("Missing LOT_SIZE in symbol filters")?;

    let step_size = parse_filter_quantity(lot_filter, "stepSize")?;
    let max_quantity = parse_filter_quantity(lot_filter, "maxQty").ok();
    let min_quantity = parse_filter_quantity(lot_filter, "minQty").ok();

    // Default margin (0.1 = 10x leverage)
    let default_margin = Decimal::new(1, 1);

    let instrument = CryptoPerpetual::new(
        instrument_id,
        raw_symbol,
        base_currency,
        quote_currency,
        settlement_currency,
        false, // is_inverse
        tick_size.precision,
        step_size.precision,
        tick_size,
        step_size,
        None, // multiplier
        Some(step_size),
        max_quantity,
        min_quantity,
        None, // max_notional
        None, // min_notional
        max_price,
        min_price,
        Some(default_margin),
        Some(default_margin),
        None, // maker_fee
        None, // taker_fee
        None, // info
        ts_event,
        ts_init,
    );

    Ok(InstrumentAny::CryptoPerpetual(instrument))
}

/// Parses a COIN-M Futures symbol definition into a Nautilus CryptoPerpetual instrument.
///
/// COIN-M perpetuals are inverse contracts settled in base currency (e.g., BTC).
///
/// # Errors
///
/// Returns an error if:
/// - Required filter values are missing (PRICE_FILTER, LOT_SIZE).
/// - Price or quantity values cannot be parsed.
/// - The contract type is not PERPETUAL.
/// - The contract is not in TRADING status.
pub fn parse_coinm_instrument(
    symbol: &AsterFuturesCoinSymbol,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
) -> anyhow::Result<InstrumentAny> {
    if symbol.contract_type != CONTRACT_TYPE_PERPETUAL {
        anyhow::bail!(
            "Unsupported contract type '{}' for symbol '{}', expected '{}'",
            symbol.contract_type,
            symbol.symbol,
            CONTRACT_TYPE_PERPETUAL
        );
    }

    if symbol.contract_status != Some(AsterContractStatus::Trading) {
        anyhow::bail!(
            "Symbol '{}' is not trading (status: {:?})",
            symbol.symbol,
            symbol.contract_status
        );
    }

    let base_currency = get_currency(symbol.base_asset.as_str());
    let quote_currency = get_currency(symbol.quote_asset.as_str());

    // COIN-M contracts are settled in the base currency (inverse)
    let settlement_currency = get_currency(symbol.margin_asset.as_str());

    let instrument_id = InstrumentId::new(
        Symbol::from_str_unchecked(format!("{}-PERP", symbol.symbol)),
        Venue::new(ASTER),
    );
    let raw_symbol = Symbol::new(symbol.symbol.as_str());

    let price_filter = get_filter(&symbol.filters, "PRICE_FILTER")
        .context("Missing PRICE_FILTER in symbol filters")?;

    let tick_size = parse_filter_price(price_filter, "tickSize")?;
    if tick_size.is_zero() {
        anyhow::bail!(
            "Invalid tickSize of 0 for symbol '{}', cannot create instrument",
            symbol.symbol,
        );
    }
    let max_price = parse_filter_price(price_filter, "maxPrice").ok();
    let min_price = parse_filter_price(price_filter, "minPrice").ok();

    let lot_filter =
        get_filter(&symbol.filters, "LOT_SIZE").context("Missing LOT_SIZE in symbol filters")?;

    let step_size = parse_filter_quantity(lot_filter, "stepSize")?;
    let max_quantity = parse_filter_quantity(lot_filter, "maxQty").ok();
    let min_quantity = parse_filter_quantity(lot_filter, "minQty").ok();

    // COIN-M has contract_size as the multiplier
    let multiplier = Quantity::new(symbol.contract_size as f64, 0);

    // Default margin (0.1 = 10x leverage)
    let default_margin = Decimal::new(1, 1);

    let instrument = CryptoPerpetual::new(
        instrument_id,
        raw_symbol,
        base_currency,
        quote_currency,
        settlement_currency,
        true, // is_inverse (COIN-M contracts are inverse)
        tick_size.precision,
        step_size.precision,
        tick_size,
        step_size,
        Some(multiplier),
        Some(step_size),
        max_quantity,
        min_quantity,
        None, // max_notional
        None, // min_notional
        max_price,
        min_price,
        Some(default_margin),
        Some(default_margin),
        None, // maker_fee
        None, // taker_fee
        None, // info
        ts_event,
        ts_init,
    );

    Ok(InstrumentAny::CryptoPerpetual(instrument))
}

/// Converts a Nautilus bar specification to a Aster kline interval.
///
/// # Errors
///
/// Returns an error if the bar specification does not map to a supported
/// Aster kline interval.
pub fn bar_spec_to_aster_interval(
    bar_spec: BarSpecification,
) -> anyhow::Result<AsterKlineInterval> {
    let step = bar_spec.step.get();
    let interval = match bar_spec.aggregation {
        BarAggregation::Second => {
            anyhow::bail!("Aster Spot does not support second-level kline intervals")
        }
        BarAggregation::Minute => match step {
            1 => AsterKlineInterval::Minute1,
            3 => AsterKlineInterval::Minute3,
            5 => AsterKlineInterval::Minute5,
            15 => AsterKlineInterval::Minute15,
            30 => AsterKlineInterval::Minute30,
            _ => anyhow::bail!("Unsupported minute interval: {step}m"),
        },
        BarAggregation::Hour => match step {
            1 => AsterKlineInterval::Hour1,
            2 => AsterKlineInterval::Hour2,
            4 => AsterKlineInterval::Hour4,
            6 => AsterKlineInterval::Hour6,
            8 => AsterKlineInterval::Hour8,
            12 => AsterKlineInterval::Hour12,
            _ => anyhow::bail!("Unsupported hour interval: {step}h"),
        },
        BarAggregation::Day => match step {
            1 => AsterKlineInterval::Day1,
            3 => AsterKlineInterval::Day3,
            _ => anyhow::bail!("Unsupported day interval: {step}d"),
        },
        BarAggregation::Week => match step {
            1 => AsterKlineInterval::Week1,
            _ => anyhow::bail!("Unsupported week interval: {step}w"),
        },
        BarAggregation::Month => match step {
            1 => AsterKlineInterval::Month1,
            _ => anyhow::bail!("Unsupported month interval: {step}M"),
        },
        agg => anyhow::bail!("Unsupported bar aggregation for Aster: {agg:?}"),
    };

    Ok(interval)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;
    use ustr::Ustr;

    use super::*;
    use crate::common::enums::{AsterContractStatus, AsterTradingStatus};

    fn sample_usdm_symbol() -> AsterFuturesUsdSymbol {
        AsterFuturesUsdSymbol {
            symbol: Ustr::from("BTCUSDT"),
            pair: Ustr::from("BTCUSDT"),
            contract_type: "PERPETUAL".to_string(),
            delivery_date: 4133404800000,
            onboard_date: 1569398400000,
            status: AsterTradingStatus::Trading,
            maint_margin_percent: "2.5000".to_string(),
            required_margin_percent: "5.0000".to_string(),
            base_asset: Ustr::from("BTC"),
            quote_asset: Ustr::from("USDT"),
            margin_asset: Ustr::from("USDT"),
            price_precision: 2,
            quantity_precision: 3,
            base_asset_precision: 8,
            quote_precision: 8,
            underlying_type: Some("COIN".to_string()),
            underlying_sub_type: vec!["PoW".to_string()],
            settle_plan: None,
            trigger_protect: Some("0.0500".to_string()),
            liquidation_fee: Some("0.012500".to_string()),
            market_take_bound: Some("0.05".to_string()),
            order_types: vec!["LIMIT".to_string(), "MARKET".to_string()],
            time_in_force: vec!["GTC".to_string(), "IOC".to_string()],
            filters: vec![
                json!({
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.10",
                    "maxPrice": "4529764",
                    "minPrice": "556.80"
                }),
                json!({
                    "filterType": "LOT_SIZE",
                    "stepSize": "0.001",
                    "maxQty": "1000",
                    "minQty": "0.001"
                }),
            ],
        }
    }

    fn sample_coinm_symbol() -> AsterFuturesCoinSymbol {
        AsterFuturesCoinSymbol {
            symbol: Ustr::from("BTCUSD_PERP"),
            pair: Ustr::from("BTCUSD"),
            contract_type: "PERPETUAL".to_string(),
            delivery_date: 4_133_404_800_000,
            onboard_date: 1_569_398_400_000,
            contract_status: Some(AsterContractStatus::Trading),
            contract_size: 100,
            maint_margin_percent: "2.5000".to_string(),
            required_margin_percent: "5.0000".to_string(),
            base_asset: Ustr::from("BTC"),
            quote_asset: Ustr::from("USD"),
            margin_asset: Ustr::from("BTC"),
            price_precision: 1,
            quantity_precision: 0,
            base_asset_precision: 8,
            quote_precision: 8,
            equal_qty_precision: None,
            trigger_protect: Some("0.0500".to_string()),
            liquidation_fee: Some("0.012500".to_string()),
            market_take_bound: Some("0.05".to_string()),
            order_types: vec!["LIMIT".to_string(), "MARKET".to_string()],
            time_in_force: vec!["GTC".to_string(), "IOC".to_string()],
            filters: vec![
                json!({
                    "filterType": "PRICE_FILTER",
                    "tickSize": "0.10",
                    "maxPrice": "1000000",
                    "minPrice": "0.10"
                }),
                json!({
                    "filterType": "LOT_SIZE",
                    "stepSize": "1",
                    "maxQty": "1000",
                    "minQty": "1"
                }),
            ],
        }
    }

    #[rstest]
    fn test_parse_usdm_perpetual() {
        let symbol = sample_usdm_symbol();
        let ts = UnixNanos::from(1_700_000_000_000_000_000u64);

        let result = parse_usdm_instrument(&symbol, ts, ts);
        assert!(result.is_ok(), "Failed: {:?}", result.err());

        let instrument = result.unwrap();
        match instrument {
            InstrumentAny::CryptoPerpetual(perp) => {
                assert_eq!(perp.id.to_string(), "BTCUSDT-PERP.ASTER");
                assert_eq!(perp.raw_symbol.to_string(), "BTCUSDT");
                assert_eq!(perp.base_currency.code.as_str(), "BTC");
                assert_eq!(perp.quote_currency.code.as_str(), "USDT");
                assert_eq!(perp.settlement_currency.code.as_str(), "USDT");
                assert!(!perp.is_inverse);
                assert_eq!(perp.price_increment, Price::from_str("0.10").unwrap());
                assert_eq!(perp.size_increment, Quantity::from_str("0.001").unwrap());
            }
            other => panic!("Expected CryptoPerpetual, was {other:?}"),
        }
    }

    #[rstest]
    fn test_parse_non_perpetual_fails() {
        let mut symbol = sample_usdm_symbol();
        symbol.contract_type = "CURRENT_QUARTER".to_string();
        let ts = UnixNanos::from(1_700_000_000_000_000_000u64);

        let result = parse_usdm_instrument(&symbol, ts, ts);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported contract type")
        );
    }

    #[rstest]
    fn test_parse_missing_price_filter_fails() {
        let mut symbol = sample_usdm_symbol();
        symbol.filters = vec![json!({
            "filterType": "LOT_SIZE",
            "stepSize": "0.001",
            "maxQty": "1000",
            "minQty": "0.001"
        })];
        let ts = UnixNanos::from(1_700_000_000_000_000_000u64);

        let result = parse_usdm_instrument(&symbol, ts, ts);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing PRICE_FILTER")
        );
    }

    #[rstest]
    fn test_parse_coinm_perpetual() {
        let symbol = sample_coinm_symbol();
        let ts = UnixNanos::from(1_700_000_000_000_000_000u64);

        let result = parse_coinm_instrument(&symbol, ts, ts).unwrap();

        match result {
            InstrumentAny::CryptoPerpetual(perp) => {
                assert_eq!(perp.id.to_string(), "BTCUSD_PERP-PERP.ASTER");
                assert_eq!(perp.raw_symbol.to_string(), "BTCUSD_PERP");
                assert_eq!(perp.base_currency.code.as_str(), "BTC");
                assert_eq!(perp.quote_currency.code.as_str(), "USD");
                assert_eq!(perp.settlement_currency.code.as_str(), "BTC");
                assert!(perp.is_inverse);
                assert_eq!(perp.price_increment, Price::from_str("0.10").unwrap());
                assert_eq!(perp.size_increment, Quantity::from_str("1").unwrap());
            }
            other => panic!("Expected CryptoPerpetual, was {other:?}"),
        }
    }

    mod bar_spec_tests {
        use std::num::NonZeroUsize;

        use nautilus_model::{
            data::BarSpecification,
            enums::{BarAggregation, PriceType},
        };

        use super::*;
        use crate::common::enums::AsterKlineInterval;

        fn make_bar_spec(step: usize, aggregation: BarAggregation) -> BarSpecification {
            BarSpecification {
                step: NonZeroUsize::new(step).unwrap(),
                aggregation,
                price_type: PriceType::Last,
            }
        }

        #[rstest]
        #[case(1, BarAggregation::Minute, AsterKlineInterval::Minute1)]
        #[case(3, BarAggregation::Minute, AsterKlineInterval::Minute3)]
        #[case(5, BarAggregation::Minute, AsterKlineInterval::Minute5)]
        #[case(15, BarAggregation::Minute, AsterKlineInterval::Minute15)]
        #[case(30, BarAggregation::Minute, AsterKlineInterval::Minute30)]
        #[case(1, BarAggregation::Hour, AsterKlineInterval::Hour1)]
        #[case(2, BarAggregation::Hour, AsterKlineInterval::Hour2)]
        #[case(4, BarAggregation::Hour, AsterKlineInterval::Hour4)]
        #[case(6, BarAggregation::Hour, AsterKlineInterval::Hour6)]
        #[case(8, BarAggregation::Hour, AsterKlineInterval::Hour8)]
        #[case(12, BarAggregation::Hour, AsterKlineInterval::Hour12)]
        #[case(1, BarAggregation::Day, AsterKlineInterval::Day1)]
        #[case(3, BarAggregation::Day, AsterKlineInterval::Day3)]
        #[case(1, BarAggregation::Week, AsterKlineInterval::Week1)]
        #[case(1, BarAggregation::Month, AsterKlineInterval::Month1)]
        fn test_bar_spec_to_aster_interval(
            #[case] step: usize,
            #[case] aggregation: BarAggregation,
            #[case] expected: AsterKlineInterval,
        ) {
            let bar_spec = make_bar_spec(step, aggregation);
            let result = bar_spec_to_aster_interval(bar_spec).unwrap();
            assert_eq!(result, expected);
        }

        #[rstest]
        fn test_unsupported_second_interval() {
            let bar_spec = make_bar_spec(1, BarAggregation::Second);
            let result = bar_spec_to_aster_interval(bar_spec);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("does not support second-level")
            );
        }

        #[rstest]
        fn test_unsupported_minute_interval() {
            let bar_spec = make_bar_spec(7, BarAggregation::Minute);
            let result = bar_spec_to_aster_interval(bar_spec);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Unsupported minute interval")
            );
        }

        #[rstest]
        fn test_unsupported_aggregation() {
            let bar_spec = make_bar_spec(100, BarAggregation::Tick);
            let result = bar_spec_to_aster_interval(bar_spec);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Unsupported bar aggregation")
            );
        }
    }

}
