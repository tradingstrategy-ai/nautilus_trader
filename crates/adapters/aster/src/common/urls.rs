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

//! URL resolution helpers for Aster API endpoints.

use super::{
    consts::{
        ASTER_FUTURES_COIN_DEMO_HTTP_URL, ASTER_FUTURES_COIN_HTTP_URL,
        ASTER_FUTURES_COIN_TESTNET_HTTP_URL, ASTER_FUTURES_COIN_TESTNET_WS_URL,
        ASTER_FUTURES_COIN_WS_URL, ASTER_FUTURES_USD_DEMO_HTTP_URL,
        ASTER_FUTURES_USD_HTTP_URL, ASTER_FUTURES_USD_TESTNET_HTTP_URL,
        ASTER_FUTURES_USD_TESTNET_WS_URL, ASTER_FUTURES_USD_WS_URL, ASTER_OPTIONS_HTTP_URL,
        ASTER_OPTIONS_WS_URL, ASTER_SPOT_DEMO_HTTP_URL, ASTER_SPOT_DEMO_WS_URL,
        ASTER_SPOT_HTTP_URL, ASTER_SPOT_TESTNET_HTTP_URL, ASTER_SPOT_TESTNET_WS_URL,
        ASTER_SPOT_WS_URL,
    },
    enums::{AsterEnvironment, AsterProductType},
};

/// Returns the HTTP base URL for the given product type and environment.
#[must_use]
pub fn get_http_base_url(
    product_type: AsterProductType,
    environment: AsterEnvironment,
) -> &'static str {
    match (product_type, environment) {
        // Mainnet
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Mainnet) => {
            ASTER_SPOT_HTTP_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Mainnet) => ASTER_FUTURES_USD_HTTP_URL,
        (AsterProductType::CoinM, AsterEnvironment::Mainnet) => ASTER_FUTURES_COIN_HTTP_URL,
        (AsterProductType::Options, AsterEnvironment::Mainnet) => ASTER_OPTIONS_HTTP_URL,

        // Testnet
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Testnet) => {
            ASTER_SPOT_TESTNET_HTTP_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Testnet) => {
            ASTER_FUTURES_USD_TESTNET_HTTP_URL
        }
        (AsterProductType::CoinM, AsterEnvironment::Testnet) => {
            ASTER_FUTURES_COIN_TESTNET_HTTP_URL
        }
        (AsterProductType::Options, AsterEnvironment::Testnet) => ASTER_OPTIONS_HTTP_URL,

        // Demo
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Demo) => {
            ASTER_SPOT_DEMO_HTTP_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Demo) => ASTER_FUTURES_USD_DEMO_HTTP_URL,
        (AsterProductType::CoinM, AsterEnvironment::Demo) => ASTER_FUTURES_COIN_DEMO_HTTP_URL,
        (AsterProductType::Options, AsterEnvironment::Demo) => ASTER_OPTIONS_HTTP_URL,
    }
}

/// Returns the WebSocket base URL for the given product type and environment.
#[must_use]
pub fn get_ws_base_url(
    product_type: AsterProductType,
    environment: AsterEnvironment,
) -> &'static str {
    match (product_type, environment) {
        // Mainnet
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Mainnet) => {
            ASTER_SPOT_WS_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Mainnet) => ASTER_FUTURES_USD_WS_URL,
        (AsterProductType::CoinM, AsterEnvironment::Mainnet) => ASTER_FUTURES_COIN_WS_URL,
        (AsterProductType::Options, AsterEnvironment::Mainnet) => ASTER_OPTIONS_WS_URL,

        // Testnet
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Testnet) => {
            ASTER_SPOT_TESTNET_WS_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Testnet) => {
            ASTER_FUTURES_USD_TESTNET_WS_URL
        }
        (AsterProductType::CoinM, AsterEnvironment::Testnet) => {
            ASTER_FUTURES_COIN_TESTNET_WS_URL
        }
        (AsterProductType::Options, AsterEnvironment::Testnet) => ASTER_OPTIONS_WS_URL,

        // Demo (futures demo uses same WS URLs as futures testnet)
        (AsterProductType::Spot | AsterProductType::Margin, AsterEnvironment::Demo) => {
            ASTER_SPOT_DEMO_WS_URL
        }
        (AsterProductType::UsdM, AsterEnvironment::Demo) => ASTER_FUTURES_USD_TESTNET_WS_URL,
        (AsterProductType::CoinM, AsterEnvironment::Demo) => {
            ASTER_FUTURES_COIN_TESTNET_WS_URL
        }
        (AsterProductType::Options, AsterEnvironment::Demo) => ASTER_OPTIONS_WS_URL,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_http_url_spot_mainnet() {
        let url = get_http_base_url(AsterProductType::Spot, AsterEnvironment::Mainnet);
        assert_eq!(url, "https://api.asterdex.com");
    }

    #[rstest]
    fn test_http_url_spot_testnet() {
        let url = get_http_base_url(AsterProductType::Spot, AsterEnvironment::Testnet);
        assert_eq!(url, "https://testnet.asterdex.com");
    }

    #[rstest]
    fn test_http_url_spot_demo() {
        let url = get_http_base_url(AsterProductType::Spot, AsterEnvironment::Demo);
        assert_eq!(url, "https://demo-api.asterdex.com");
    }

    #[rstest]
    fn test_http_url_usdm_mainnet() {
        let url = get_http_base_url(AsterProductType::UsdM, AsterEnvironment::Mainnet);
        assert_eq!(url, "https://fapi.asterdex.com");
    }

    #[rstest]
    fn test_http_url_usdm_testnet() {
        let url = get_http_base_url(AsterProductType::UsdM, AsterEnvironment::Testnet);
        assert_eq!(url, "https://demo-fapi.asterdex.com");
    }

    #[rstest]
    fn test_http_url_coinm_mainnet() {
        let url = get_http_base_url(AsterProductType::CoinM, AsterEnvironment::Mainnet);
        assert_eq!(url, "https://dapi.asterdex.com");
    }

    #[rstest]
    fn test_http_url_usdm_demo() {
        let url = get_http_base_url(AsterProductType::UsdM, AsterEnvironment::Demo);
        assert_eq!(url, "https://demo-fapi.asterdex.com");
    }

    #[rstest]
    fn test_http_url_coinm_demo() {
        let url = get_http_base_url(AsterProductType::CoinM, AsterEnvironment::Demo);
        assert_eq!(url, "https://testnet.asterdex.com");
    }

    #[rstest]
    fn test_ws_url_spot_mainnet() {
        let url = get_ws_base_url(AsterProductType::Spot, AsterEnvironment::Mainnet);
        assert_eq!(url, "wss://stream.asterdex.com:9443/ws");
    }

    #[rstest]
    fn test_ws_url_spot_demo() {
        let url = get_ws_base_url(AsterProductType::Spot, AsterEnvironment::Demo);
        assert_eq!(url, "wss://demo-stream.asterdex.com/ws");
    }

    #[rstest]
    fn test_ws_url_usdm_mainnet() {
        let url = get_ws_base_url(AsterProductType::UsdM, AsterEnvironment::Mainnet);
        assert_eq!(url, "wss://fstream.asterdex.com/ws");
    }

    #[rstest]
    fn test_ws_url_usdm_testnet() {
        let url = get_ws_base_url(AsterProductType::UsdM, AsterEnvironment::Testnet);
        assert_eq!(url, "wss://fstream-testnet.asterdex.com/ws");
    }
}
