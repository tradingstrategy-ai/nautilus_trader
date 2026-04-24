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

//! Aster Futures adapter components.
//!
//! This module provides HTTP and WebSocket clients for Aster Futures:
//!
//! - **USD-M Futures** (`fapi.asterdex.com`) - USDT-margined perpetual contracts
//! - **COIN-M Futures** (`dapi.asterdex.com`) - Coin-margined perpetual contracts
//!
//! ## WebSocket Streams
//!
//! Unlike Spot which uses SBE binary encoding, Futures uses standard JSON WebSocket streams:
//!
//! - `<symbol>@trade` - Real-time trade data
//! - `<symbol>@depth` - Order book updates (diff)
//! - `<symbol>@depth@100ms` - Order book updates (100ms frequency)
//! - `<symbol>@markPrice` - Mark price updates
//! - `<symbol>@kline_<interval>` - Kline/candlestick updates
//!
//! ## Authentication
//!
//! - Public streams: No authentication required
//! - User data streams: Requires listen key (obtained via REST API)

pub mod data;
pub mod execution;
pub mod http;
pub mod websocket;

pub use data::AsterFuturesDataClient;
pub use execution::AsterFuturesExecutionClient;
pub use http::client::AsterFuturesHttpClient;
pub use websocket::{
    streams::client::AsterFuturesWebSocketClient, trading::AsterFuturesWsTradingClient,
};
