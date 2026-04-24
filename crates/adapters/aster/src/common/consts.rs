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

//! Aster venue constants and API endpoints.

use std::{num::NonZeroU32, sync::LazyLock};

use nautilus_model::identifiers::Venue;
use nautilus_network::ratelimiter::quota::Quota;
use ustr::Ustr;

use super::enums::{AsterRateLimitInterval, AsterRateLimitType};

/// The Aster venue identifier string.
pub const ASTER: &str = "ASTER";

/// Static venue instance for Aster.
pub static ASTER_VENUE: LazyLock<Venue> = LazyLock::new(|| Venue::new(ASTER));

/// Aster Link and Trade broker ID for Spot.
///
/// <https://developers.asterdex.com/docs/aster_link/link-and-trade>
pub const ASTER_NAUTILUS_SPOT_BROKER_ID: &str = "TD67BGP9";

/// Aster Link and Trade broker ID for Futures.
///
/// <https://developers.asterdex.com/docs/aster_link/link-and-trade>
pub const ASTER_NAUTILUS_FUTURES_BROKER_ID: &str = "aHRE4BCj";

/// Aster Spot API base URL (mainnet).
pub const ASTER_SPOT_HTTP_URL: &str = "https://api.asterdex.com";

/// Aster USD-M Futures API base URL (mainnet).
pub const ASTER_FUTURES_USD_HTTP_URL: &str = "https://fapi.asterdex.com";

/// Aster COIN-M Futures API base URL (mainnet).
pub const ASTER_FUTURES_COIN_HTTP_URL: &str = "https://dapi.asterdex.com";

/// Aster European Options API base URL (mainnet).
pub const ASTER_OPTIONS_HTTP_URL: &str = "https://eapi.asterdex.com";

/// Aster Spot API base URL (testnet).
pub const ASTER_SPOT_TESTNET_HTTP_URL: &str = "https://testnet.asterdex.com";

/// Aster USD-M Futures API base URL (testnet).
pub const ASTER_FUTURES_USD_TESTNET_HTTP_URL: &str = "https://demo-fapi.asterdex.com";

/// Aster COIN-M Futures API base URL (testnet).
pub const ASTER_FUTURES_COIN_TESTNET_HTTP_URL: &str = "https://testnet.asterdex.com";

/// Aster Spot API base URL (demo).
pub const ASTER_SPOT_DEMO_HTTP_URL: &str = "https://demo-api.asterdex.com";

/// Aster USD-M Futures API base URL (demo).
pub const ASTER_FUTURES_USD_DEMO_HTTP_URL: &str = "https://demo-fapi.asterdex.com";

/// Aster COIN-M Futures API base URL (demo, same as COIN-M testnet).
pub const ASTER_FUTURES_COIN_DEMO_HTTP_URL: &str = "https://testnet.asterdex.com";

/// Aster Spot WebSocket base URL (mainnet).
pub const ASTER_SPOT_WS_URL: &str = "wss://stream.asterdex.com:9443/ws";

/// Aster USD-M Futures WebSocket base URL (mainnet).
pub const ASTER_FUTURES_USD_WS_URL: &str = "wss://fstream.asterdex.com/ws";

/// Aster COIN-M Futures WebSocket base URL (mainnet).
pub const ASTER_FUTURES_COIN_WS_URL: &str = "wss://dstream.asterdex.com/ws";

/// Aster European Options WebSocket base URL (mainnet).
pub const ASTER_OPTIONS_WS_URL: &str = "wss://nbstream.asterdex.com/eoptions";

/// Aster Spot SBE WebSocket stream URL (mainnet).
pub const ASTER_SPOT_SBE_WS_URL: &str = "wss://stream-sbe.asterdex.com/ws";

/// Aster Spot SBE WebSocket API URL (mainnet).
pub const ASTER_SPOT_SBE_WS_API_URL: &str =
    "wss://ws-api.asterdex.com:443/ws-api/v3?responseFormat=sbe&sbeSchemaId=3&sbeSchemaVersion=3";

/// Aster USD-M Futures WebSocket Trading API URL (mainnet).
pub const ASTER_FUTURES_USD_WS_API_URL: &str = "wss://ws-fapi.asterdex.com/ws-fapi/v1";

/// Aster USD-M Futures WebSocket Trading API URL (testnet).
pub const ASTER_FUTURES_USD_WS_API_TESTNET_URL: &str =
    "wss://testnet.asterdex.com/ws-fapi/v1";

/// Aster Spot SBE WebSocket API URL (testnet).
pub const ASTER_SPOT_SBE_WS_API_TESTNET_URL: &str = "wss://ws-api.testnet.asterdex.com/ws-api/v3?responseFormat=sbe&sbeSchemaId=3&sbeSchemaVersion=3";

/// Aster Spot SBE WebSocket API URL (demo).
pub const ASTER_SPOT_SBE_WS_API_DEMO_URL: &str =
    "wss://demo-ws-api.asterdex.com/ws-api/v3?responseFormat=sbe&sbeSchemaId=3&sbeSchemaVersion=3";

/// Aster Spot WebSocket base URL (testnet).
pub const ASTER_SPOT_TESTNET_WS_URL: &str = "wss://stream.testnet.asterdex.com/ws";

/// Aster Spot WebSocket base URL (demo).
pub const ASTER_SPOT_DEMO_WS_URL: &str = "wss://demo-stream.asterdex.com/ws";

/// Aster USD-M Futures WebSocket base URL (testnet).
pub const ASTER_FUTURES_USD_TESTNET_WS_URL: &str = "wss://fstream-testnet.asterdex.com/ws";

/// Aster COIN-M Futures WebSocket base URL (testnet).
pub const ASTER_FUTURES_COIN_TESTNET_WS_URL: &str = "wss://dstream-testnet.asterdex.com/ws";

/// HTTP header name for the Aster API key.
pub const ASTER_API_KEY_HEADER: &str = "X-MBX-APIKEY";

/// Aster Spot API version path.
pub const ASTER_SPOT_API_PATH: &str = "/api/v3";

/// Aster USD-M Futures API version path.
pub const ASTER_FAPI_PATH: &str = "/fapi/v1";

/// Aster COIN-M Futures API version path.
pub const ASTER_DAPI_PATH: &str = "/dapi/v1";

/// Aster European Options API version path.
pub const ASTER_EAPI_PATH: &str = "/eapi/v1";

/// Describes a static rate limit quota for a product type.
#[derive(Clone, Copy, Debug)]
pub struct AsterRateLimitQuota {
    /// Rate limit type.
    pub rate_limit_type: AsterRateLimitType,
    /// Time interval unit.
    pub interval: AsterRateLimitInterval,
    /// Number of intervals.
    pub interval_num: u32,
    /// Maximum allowed requests for the interval.
    pub limit: u32,
}

/// Spot & margin REST limits (default IP weights).
///
/// References:
/// - <https://developers.asterdex.com/docs/aster-spot-api-docs/limits>
pub const ASTER_SPOT_RATE_LIMITS: &[AsterRateLimitQuota] = &[
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::RequestWeight,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 1_200,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Second,
        interval_num: 1,
        limit: 10,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Day,
        interval_num: 1,
        limit: 100_000,
    },
];

/// USD-M Futures REST limits (default IP weights).
///
/// References:
/// - <https://developers.asterdex.com/docs/derivatives/usds-margined-futures/general-info#limits>
pub const ASTER_FAPI_RATE_LIMITS: &[AsterRateLimitQuota] = &[
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::RequestWeight,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 2_400,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Second,
        interval_num: 1,
        limit: 50,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 1_200,
    },
];

/// COIN-M Futures REST limits (default IP weights).
///
/// References:
/// - <https://developers.asterdex.com/docs/derivatives/coin-margined-futures/general-info#limits>
pub const ASTER_DAPI_RATE_LIMITS: &[AsterRateLimitQuota] = &[
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::RequestWeight,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 1_200,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Second,
        interval_num: 1,
        limit: 20,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 1_200,
    },
];

/// Options REST limits (default IP weights).
///
/// References:
/// - <https://developers.asterdex.com/docs/derivatives/european-options/general-info#limits>
pub const ASTER_EAPI_RATE_LIMITS: &[AsterRateLimitQuota] = &[
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::RequestWeight,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 3_000,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Second,
        interval_num: 1,
        limit: 5,
    },
    AsterRateLimitQuota {
        rate_limit_type: AsterRateLimitType::Orders,
        interval: AsterRateLimitInterval::Minute,
        interval_num: 1,
        limit: 200,
    },
];

/// WebSocket subscription rate limit: 5 messages per second.
///
/// Aster limits incoming WebSocket messages (subscribe/unsubscribe) to 5 per second.
pub static ASTER_WS_SUBSCRIPTION_QUOTA: LazyLock<Quota> = LazyLock::new(|| {
    Quota::per_second(NonZeroU32::new(5).expect("non-zero")).expect("valid constant")
});

/// WebSocket connection rate limit: 1 per second (conservative).
///
/// Aster limits connections to 300 per 5 minutes per IP. This conservative quota
/// of 1 per second helps avoid hitting the connection limit during reconnection storms.
pub static ASTER_WS_CONNECTION_QUOTA: LazyLock<Quota> = LazyLock::new(|| {
    Quota::per_second(NonZeroU32::new(1).expect("non-zero")).expect("valid constant")
});

/// Pre-interned rate limit key for WebSocket subscription operations.
pub static ASTER_RATE_LIMIT_KEY_SUBSCRIPTION: LazyLock<[Ustr; 1]> =
    LazyLock::new(|| [Ustr::from("subscription")]);

/// Aster error code for GTX (post-only) order rejection.
///
/// Returned when a GTX order would immediately match as taker.
pub const ASTER_GTX_ORDER_REJECT_CODE: i64 = -5022;

/// Aster error code for new order rejected.
///
/// For spot LIMIT_MAKER orders, this code is returned with the message
/// "Order would immediately match and take." to indicate a post-only rejection.
pub const ASTER_NEW_ORDER_REJECTED_CODE: i64 = -2010;

/// Aster Spot LIMIT_MAKER rejection message.
///
/// This message is specific to post-only (LIMIT_MAKER) orders that would match immediately.
pub const ASTER_SPOT_POST_ONLY_REJECT_MSG: &str = "Order would immediately match and take.";

/// Valid order book depth levels for Aster.
pub const ASTER_BOOK_DEPTHS: [u32; 7] = [5, 10, 20, 50, 100, 500, 1000];
