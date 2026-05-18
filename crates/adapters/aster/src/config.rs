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

//! Aster adapter configuration structures.

use std::{any::Any, collections::HashMap};

use nautilus_model::identifiers::{AccountId, TraderId};
use nautilus_system::factories::ClientConfig;
use rust_decimal::Decimal;

use crate::common::enums::{AsterEnvironment, AsterMarginType, AsterProductType};

/// Configuration for Aster data client.
///
/// Ed25519 API keys are required for SBE WebSocket streams.
#[derive(Clone, Debug, bon::Builder)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.aster", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.aster")
)]
pub struct AsterDataClientConfig {
    /// Product types to subscribe to.
    #[builder(default = vec![AsterProductType::UsdM])]  // Aster is futures-only
    pub product_types: Vec<AsterProductType>,
    /// Environment (mainnet or testnet).
    #[builder(default = AsterEnvironment::Mainnet)]
    pub environment: AsterEnvironment,
    /// Optional base URL override for HTTP API.
    pub base_url_http: Option<String>,
    /// Optional base URL override for WebSocket.
    pub base_url_ws: Option<String>,
    /// API key (Ed25519).
    pub api_key: Option<String>,
    /// API secret (Ed25519 base64-encoded or PEM).
    pub api_secret: Option<String>,
    /// Interval in seconds for polling exchange info to detect instrument status
    /// changes (e.g. Trading -> Halt). Set to 0 to disable. Defaults to 3600 (60 minutes).
    #[builder(default = 3600)]
    pub instrument_status_poll_secs: u64,
    /// Optional local source IP to bind outbound TCP connections to (REST + WS).
    /// When `Some(ip)`, the client pins every outbound socket to this address;
    /// when `None` (default), the kernel selects the source IP from the routing table.
    pub local_addr: Option<std::net::IpAddr>,
}

impl Default for AsterDataClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ClientConfig for AsterDataClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Configuration for Aster execution client.
///
/// Ed25519 API keys are required for execution clients. Aster deprecated
/// listenKey-based user data streams in favor of WebSocket API authentication,
/// which only supports Ed25519.
#[derive(Clone, Debug, bon::Builder)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.aster", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.aster")
)]
pub struct AsterExecClientConfig {
    /// Trader ID for the client.
    #[builder(default = TraderId::from("TRADER-001"))]
    pub trader_id: TraderId,
    /// Account ID for the client.
    #[builder(default = AccountId::from("ASTER-001"))]
    pub account_id: AccountId,
    /// Product types to trade.
    #[builder(default = vec![AsterProductType::UsdM])]  // Aster is futures-only
    pub product_types: Vec<AsterProductType>,
    /// Environment (mainnet or testnet).
    #[builder(default = AsterEnvironment::Mainnet)]
    pub environment: AsterEnvironment,
    /// Optional base URL override for HTTP API.
    pub base_url_http: Option<String>,
    /// Optional base URL override for WebSocket user data stream.
    pub base_url_ws: Option<String>,
    /// Optional base URL override for WebSocket trading API (Spot and USD-M Futures).
    pub base_url_ws_trading: Option<String>,
    /// Whether to use the WebSocket trading API for order operations (Spot and USD-M Futures).
    #[builder(default = true)]
    pub use_ws_trading: bool,
    /// Whether to use Aster Futures hedging position IDs.
    ///
    /// When true, fill reports include a `venue_position_id` derived from
    /// the instrument and position side (e.g. `ETHUSDT-PERP.ASTER-LONG`).
    /// When false, `venue_position_id` is None, allowing virtual positions
    /// with `OmsType::Hedging`.
    #[builder(default = true)]
    pub use_position_ids: bool,
    /// Default taker fee rate for commission estimation.
    ///
    /// Used as a fallback when the venue omits commission fields in
    /// exchange-generated fills (liquidation, ADL, settlement).
    /// Standard Aster Futures taker fee is 0.0004 (0.04%).
    #[builder(default = Decimal::new(4, 4))]
    pub default_taker_fee: Decimal,
    /// API key (Ed25519 required, uses env var if not provided).
    pub api_key: Option<String>,
    /// API secret (Ed25519 base64-encoded, required, uses env var if not provided).
    pub api_secret: Option<String>,
    /// Initial leverage per Aster symbol (e.g. BTCUSDT -> 20), applied during connect.
    pub futures_leverages: Option<HashMap<String, u32>>,
    /// Margin type per Aster symbol (e.g. BTCUSDT -> Cross), applied during connect.
    pub futures_margin_types: Option<HashMap<String, AsterMarginType>>,
    /// If true, the EXPIRED execution type emits `OrderCanceled` instead of `OrderExpired`.
    ///
    /// Aster uses EXPIRED for certain cancel scenarios depending on order type
    /// and time-in-force combination.
    #[builder(default = false)]
    pub treat_expired_as_canceled: bool,
    /// Optional local source IP to bind outbound TCP connections to (REST + WS).
    /// See [`AsterDataClientConfig::local_addr`].
    pub local_addr: Option<std::net::IpAddr>,
}

impl Default for AsterExecClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ClientConfig for AsterExecClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
