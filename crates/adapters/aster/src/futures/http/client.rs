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

//! Aster Futures HTTP client for USD-M and COIN-M markets.

use std::{collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};

use ahash::AHashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use nautilus_core::{
    consts::NAUTILUS_USER_AGENT, datetime::SECONDS_IN_DAY, nanos::UnixNanos, time::AtomicTime,
};
use nautilus_model::{
    data::{Bar, BarType, TradeTick},
    enums::{
        AggregationSource, AggressorSide, BarAggregation, MarketStatusAction, OrderSide, OrderType,
        TimeInForce,
    },
    events::AccountState,
    identifiers::{AccountId, ClientOrderId, InstrumentId, TradeId, VenueOrderId},
    instruments::any::InstrumentAny,
    reports::{FillReport, OrderStatusReport},
    types::{Currency, Price, Quantity},
};
use nautilus_network::{
    http::{HttpClient, HttpResponse, Method},
    ratelimiter::quota::Quota,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use ustr::Ustr;

use super::{
    error::{AsterFuturesHttpError, AsterFuturesHttpResult},
    models::{
        BatchOrderResult, AsterBookTicker, AsterCancelAllOrdersResponse, AsterFundingRate,
        AsterFuturesAccountInfo, AsterFuturesAlgoOrder, AsterFuturesAlgoOrderCancelResponse,
        AsterFuturesCoinExchangeInfo, AsterFuturesCoinSymbol, AsterFuturesKline,
        AsterFuturesMarkPrice, AsterFuturesOrder, AsterFuturesTicker24hr,
        AsterFuturesTrade, AsterFuturesUsdExchangeInfo, AsterFuturesUsdSymbol,
        AsterHedgeModeResponse, AsterLeverageResponse, AsterOpenInterest, AsterOrderBook,
        AsterPositionRisk, AsterPriceTicker, AsterServerTime, AsterUserTrade,
        ListenKeyResponse,
    },
    query::{
        BatchCancelItem, BatchModifyItem, BatchOrderItem, AsterAlgoOrderQueryParams,
        AsterAllAlgoOrdersParams, AsterAllOrdersParams, AsterBookTickerParams,
        AsterCancelAllAlgoOrdersParams, AsterCancelAllOrdersParams, AsterCancelOrderParams,
        AsterDepthParams, AsterFundingRateParams, AsterKlinesParams, AsterMarkPriceParams,
        AsterModifyOrderParams, AsterNewAlgoOrderParams, AsterNewOrderParams,
        AsterOpenAlgoOrdersParams, AsterOpenInterestParams, AsterOpenOrdersParams,
        AsterOrderQueryParams, AsterPositionRiskParams, AsterSetLeverageParams,
        AsterSetMarginTypeParams, AsterTicker24hrParams, AsterTradesParams,
        AsterUserTradesParams, ListenKeyParams,
    },
};
use crate::common::{
    consts::{
        ASTER_API_KEY_HEADER, ASTER_DAPI_PATH, ASTER_DAPI_RATE_LIMITS, ASTER_FAPI_PATH,
        ASTER_FAPI_RATE_LIMITS, ASTER_NAUTILUS_FUTURES_BROKER_ID, AsterRateLimitQuota,
    },
    credential::SigningCredential,
    encoder::encode_broker_id,
    enums::{
        AsterAlgoType, AsterEnvironment, AsterFuturesOrderType, AsterPositionSide,
        AsterPriceMatch, AsterProductType, AsterRateLimitInterval, AsterRateLimitType,
        AsterSide, AsterTimeInForce, AsterWorkingType,
    },
    models::AsterErrorResponse,
    parse::{parse_coinm_instrument, parse_usdm_instrument},
    symbol::{format_aster_symbol, format_instrument_id},
    urls::get_http_base_url,
};

const ASTER_GLOBAL_RATE_KEY: &str = "aster:global";
const ASTER_ORDERS_RATE_KEY: &str = "aster:orders";

/// Raw HTTP client for Aster Futures REST API.
#[derive(Debug, Clone)]
pub struct AsterRawFuturesHttpClient {
    client: HttpClient,
    base_url: String,
    api_path: &'static str,
    credential: Option<SigningCredential>,
    recv_window: Option<u64>,
    order_rate_keys: Vec<String>,
}

impl AsterRawFuturesHttpClient {
    /// Returns a reference to the underlying HTTP client.
    #[must_use]
    pub fn http_client(&self) -> &HttpClient {
        &self.client
    }

    /// Creates a new Aster raw futures HTTP client.
    ///
    /// # Errors
    ///
    /// Returns an error if credentials are incomplete or the HTTP client fails to build.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        product_type: AsterProductType,
        environment: AsterEnvironment,
        api_key: Option<String>,
        api_secret: Option<String>,
        base_url_override: Option<String>,
        recv_window: Option<u64>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
    ) -> AsterFuturesHttpResult<Self> {
        Self::new_with_local_addr(
            product_type,
            environment,
            api_key,
            api_secret,
            base_url_override,
            recv_window,
            timeout_secs,
            proxy_url,
            None,
        )
    }

    /// Creates a new [`AsterRawFuturesHttpClient`] with an optional `local_addr` for
    /// source-IP pinning. See [`HttpClient::new_with_local_addr`] for semantics.
    ///
    /// # Errors
    ///
    /// Returns an error if credentials are incomplete or the HTTP client fails to build.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_local_addr(
        product_type: AsterProductType,
        environment: AsterEnvironment,
        api_key: Option<String>,
        api_secret: Option<String>,
        base_url_override: Option<String>,
        recv_window: Option<u64>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
        local_addr: Option<std::net::IpAddr>,
    ) -> AsterFuturesHttpResult<Self> {
        let RateLimitConfig {
            default_quota,
            keyed_quotas,
            order_keys,
        } = Self::rate_limit_config(product_type);

        let credential = match (api_key, api_secret) {
            (Some(key), Some(secret)) => Some(SigningCredential::new(key, secret)),
            (None, None) => None,
            _ => return Err(AsterFuturesHttpError::MissingCredentials),
        };

        let base_url = base_url_override
            .unwrap_or_else(|| get_http_base_url(product_type, environment).to_string());

        let api_path = Self::resolve_api_path(product_type);
        let headers = Self::default_headers(&credential);

        let client = HttpClient::new_with_local_addr(
            headers,
            vec![ASTER_API_KEY_HEADER.to_string()],
            keyed_quotas,
            default_quota,
            timeout_secs,
            proxy_url,
            local_addr,
        )?;

        Ok(Self {
            client,
            base_url,
            api_path,
            credential,
            recv_window,
            order_rate_keys: order_keys,
        })
    }

    /// Performs a GET request and deserializes the response body.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or response deserialization fails.
    pub async fn get<P, T>(
        &self,
        path: &str,
        params: Option<&P>,
        signed: bool,
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.request(Method::GET, path, params, signed, use_order_quota, None)
            .await
    }

    /// Performs a POST request with optional body and signed query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or response deserialization fails.
    pub async fn post<P, T>(
        &self,
        path: &str,
        params: Option<&P>,
        body: Option<Vec<u8>>,
        signed: bool,
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.request(Method::POST, path, params, signed, use_order_quota, body)
            .await
    }

    /// Performs a PUT request with signed query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or response deserialization fails.
    pub async fn request_put<P, T>(
        &self,
        path: &str,
        params: Option<&P>,
        signed: bool,
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.request(Method::PUT, path, params, signed, use_order_quota, None)
            .await
    }

    /// Performs a DELETE request with signed query.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or response deserialization fails.
    pub async fn request_delete<P, T>(
        &self,
        path: &str,
        params: Option<&P>,
        signed: bool,
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.request(Method::DELETE, path, params, signed, use_order_quota, None)
            .await
    }

    /// Performs a batch POST request with batchOrders parameter.
    ///
    /// # Errors
    ///
    /// Returns an error if credentials are missing, the request fails, or JSON parsing fails.
    pub async fn batch_request<T: Serialize>(
        &self,
        path: &str,
        items: &[T],
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.batch_request_method(Method::POST, path, items, use_order_quota)
            .await
    }

    /// Performs a batch DELETE request with batchOrders parameter.
    ///
    /// # Errors
    ///
    /// Returns an error if credentials are missing, the request fails, or JSON parsing fails.
    pub async fn batch_request_delete<T: Serialize>(
        &self,
        path: &str,
        items: &[T],
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.batch_request_method(Method::DELETE, path, items, use_order_quota)
            .await
    }

    /// Performs a batch PUT request with batchOrders parameter.
    ///
    /// # Errors
    ///
    /// Returns an error if credentials are missing, the request fails, or JSON parsing fails.
    pub async fn batch_request_put<T: Serialize>(
        &self,
        path: &str,
        items: &[T],
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.batch_request_method(Method::PUT, path, items, use_order_quota)
            .await
    }

    async fn batch_request_method<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        items: &[T],
        use_order_quota: bool,
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        let cred = self
            .credential
            .as_ref()
            .ok_or(AsterFuturesHttpError::MissingCredentials)?;

        let batch_json = serde_json::to_string(items)
            .map_err(|e| AsterFuturesHttpError::ValidationError(e.to_string()))?;

        let encoded_batch = Self::percent_encode(&batch_json);
        let timestamp = Utc::now().timestamp_millis();
        let mut query = format!("batchOrders={encoded_batch}&timestamp={timestamp}");

        if let Some(recv_window) = self.recv_window {
            query.push_str(&format!("&recvWindow={recv_window}"));
        }

        let signature = cred.sign(&query);
        query.push_str(&format!("&signature={signature}"));

        let url = self.build_url(path, &query);

        let mut headers = HashMap::new();
        headers.insert(
            ASTER_API_KEY_HEADER.to_string(),
            cred.api_key().to_string(),
        );

        let keys = self.rate_limit_keys(use_order_quota);

        let response = self
            .client
            .request(
                method,
                url,
                None::<&HashMap<String, Vec<String>>>,
                Some(headers),
                None,
                None,
                Some(keys),
            )
            .await?;

        if !response.status.is_success() {
            return self.parse_error_response(&response);
        }

        serde_json::from_slice(&response.body)
            .map_err(|e| AsterFuturesHttpError::JsonError(e.to_string()))
    }

    /// Percent-encodes a string for use in URL query parameters.
    fn percent_encode(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 3);
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{byte:02X}"));
                }
            }
        }
        result
    }

    async fn request<P, T>(
        &self,
        method: Method,
        path: &str,
        params: Option<&P>,
        signed: bool,
        use_order_quota: bool,
        body: Option<Vec<u8>>,
    ) -> AsterFuturesHttpResult<T>
    where
        P: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let mut query = params
            .map(serde_urlencoded::to_string)
            .transpose()
            .map_err(|e| AsterFuturesHttpError::ValidationError(e.to_string()))?
            .unwrap_or_default();

        let mut headers = HashMap::new();

        if signed {
            let cred = self
                .credential
                .as_ref()
                .ok_or(AsterFuturesHttpError::MissingCredentials)?;

            if !query.is_empty() {
                query.push('&');
            }

            let timestamp = Utc::now().timestamp_millis();
            query.push_str(&format!("timestamp={timestamp}"));

            if let Some(recv_window) = self.recv_window {
                query.push_str(&format!("&recvWindow={recv_window}"));
            }

            let signature = cred.sign(&query);
            query.push_str(&format!("&signature={signature}"));
            headers.insert(
                ASTER_API_KEY_HEADER.to_string(),
                cred.api_key().to_string(),
            );
        }

        let url = self.build_url(path, &query);
        let keys = self.rate_limit_keys(use_order_quota);

        let response = self
            .client
            .request(
                method,
                url,
                None::<&HashMap<String, Vec<String>>>,
                Some(headers),
                body,
                None,
                Some(keys),
            )
            .await?;

        if !response.status.is_success() {
            return self.parse_error_response(&response);
        }

        serde_json::from_slice::<T>(&response.body)
            .map_err(|e| AsterFuturesHttpError::JsonError(e.to_string()))
    }

    fn build_url(&self, path: &str, query: &str) -> String {
        // Full API paths (e.g., /fapi/v2/account) bypass the default api_path
        let url_path = if path.starts_with("/fapi/") || path.starts_with("/dapi/") {
            path.to_string()
        } else if path.starts_with('/') {
            format!("{}{}", self.api_path, path)
        } else {
            format!("{}/{}", self.api_path, path)
        };

        let mut url = format!("{}{}", self.base_url, url_path);

        if !query.is_empty() {
            url.push('?');
            url.push_str(query);
        }
        url
    }

    fn rate_limit_keys(&self, use_orders: bool) -> Vec<String> {
        if use_orders {
            let mut keys = Vec::with_capacity(1 + self.order_rate_keys.len());
            keys.push(ASTER_GLOBAL_RATE_KEY.to_string());
            keys.extend(self.order_rate_keys.iter().cloned());
            keys
        } else {
            vec![ASTER_GLOBAL_RATE_KEY.to_string()]
        }
    }

    fn parse_error_response<T>(&self, response: &HttpResponse) -> AsterFuturesHttpResult<T> {
        let status = response.status.as_u16();
        let body = String::from_utf8_lossy(&response.body).to_string();

        if let Ok(err) = serde_json::from_str::<AsterErrorResponse>(&body) {
            return Err(AsterFuturesHttpError::AsterError {
                code: err.code,
                message: err.msg,
            });
        }

        Err(AsterFuturesHttpError::UnexpectedStatus { status, body })
    }

    fn default_headers(credential: &Option<SigningCredential>) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), NAUTILUS_USER_AGENT.to_string());

        if let Some(cred) = credential {
            headers.insert(
                ASTER_API_KEY_HEADER.to_string(),
                cred.api_key().to_string(),
            );
        }
        headers
    }

    fn resolve_api_path(product_type: AsterProductType) -> &'static str {
        match product_type {
            AsterProductType::UsdM => ASTER_FAPI_PATH,
            AsterProductType::CoinM => ASTER_DAPI_PATH,
            _ => ASTER_FAPI_PATH, // Default to USD-M
        }
    }

    fn rate_limit_config(product_type: AsterProductType) -> RateLimitConfig {
        let quotas = match product_type {
            AsterProductType::UsdM => ASTER_FAPI_RATE_LIMITS,
            AsterProductType::CoinM => ASTER_DAPI_RATE_LIMITS,
            _ => ASTER_FAPI_RATE_LIMITS,
        };

        let mut keyed = Vec::new();
        let mut order_keys = Vec::new();
        let mut default = None;

        for quota in quotas {
            if let Some(q) = Self::quota_from(quota) {
                match quota.rate_limit_type {
                    AsterRateLimitType::RequestWeight if default.is_none() => {
                        default = Some(q);
                    }
                    AsterRateLimitType::Orders => {
                        let key = format!("{}:{:?}", ASTER_ORDERS_RATE_KEY, quota.interval);
                        order_keys.push(key.clone());
                        keyed.push((key, q));
                    }
                    _ => {}
                }
            }
        }

        let default_quota = default.unwrap_or_else(|| {
            Quota::per_second(NonZeroU32::new(10).expect("non-zero")).expect("valid constant")
        });

        keyed.push((ASTER_GLOBAL_RATE_KEY.to_string(), default_quota));

        RateLimitConfig {
            default_quota: Some(default_quota),
            keyed_quotas: keyed,
            order_keys,
        }
    }

    fn quota_from(quota: &AsterRateLimitQuota) -> Option<Quota> {
        let burst = NonZeroU32::new(quota.limit)?;
        match quota.interval {
            AsterRateLimitInterval::Second => Quota::per_second(burst),
            AsterRateLimitInterval::Minute => Some(Quota::per_minute(burst)),
            AsterRateLimitInterval::Day => {
                Quota::with_period(Duration::from_secs(SECONDS_IN_DAY))
                    .map(|q| q.allow_burst(burst))
            }
            AsterRateLimitInterval::Unknown => None,
        }
    }

    /// Fetches 24hr ticker statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn ticker_24h(
        &self,
        params: &AsterTicker24hrParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesTicker24hr>> {
        self.get("ticker/24hr", Some(params), false, false).await
    }

    /// Fetches best bid/ask prices.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn book_ticker(
        &self,
        params: &AsterBookTickerParams,
    ) -> AsterFuturesHttpResult<Vec<AsterBookTicker>> {
        self.get("ticker/bookTicker", Some(params), false, false)
            .await
    }

    /// Fetches price ticker.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn price_ticker(
        &self,
        symbol: Option<&str>,
    ) -> AsterFuturesHttpResult<Vec<AsterPriceTicker>> {
        #[derive(Serialize)]
        struct Params<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            symbol: Option<&'a str>,
        }
        self.get("ticker/price", Some(&Params { symbol }), false, false)
            .await
    }

    /// Fetches order book depth.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn depth(
        &self,
        params: &AsterDepthParams,
    ) -> AsterFuturesHttpResult<AsterOrderBook> {
        self.get("depth", Some(params), false, false).await
    }

    /// Fetches mark price and funding rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn mark_price(
        &self,
        params: &AsterMarkPriceParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesMarkPrice>> {
        let response: MarkPriceResponse =
            self.get("premiumIndex", Some(params), false, false).await?;
        Ok(response.into())
    }

    /// Fetches funding rate history.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn funding_rate(
        &self,
        params: &AsterFundingRateParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFundingRate>> {
        self.get("fundingRate", Some(params), false, false).await
    }

    /// Fetches current open interest for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn open_interest(
        &self,
        params: &AsterOpenInterestParams,
    ) -> AsterFuturesHttpResult<AsterOpenInterest> {
        self.get("openInterest", Some(params), false, false).await
    }

    /// Fetches recent public trades for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn trades(
        &self,
        params: &AsterTradesParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesTrade>> {
        self.get("trades", Some(params), false, false).await
    }

    /// Fetches kline/candlestick data for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn klines(
        &self,
        params: &AsterKlinesParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesKline>> {
        self.get("klines", Some(params), false, false).await
    }

    /// Sets leverage for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn set_leverage(
        &self,
        params: &AsterSetLeverageParams,
    ) -> AsterFuturesHttpResult<AsterLeverageResponse> {
        self.post("leverage", Some(params), None, true, false).await
    }

    /// Sets margin type for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn set_margin_type(
        &self,
        params: &AsterSetMarginTypeParams,
    ) -> AsterFuturesHttpResult<serde_json::Value> {
        self.post("marginType", Some(params), None, true, false)
            .await
    }

    /// Queries hedge mode (dual side position) setting.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_hedge_mode(&self) -> AsterFuturesHttpResult<AsterHedgeModeResponse> {
        self.get::<(), _>("positionSide/dual", None, true, false)
            .await
    }

    /// Creates a listen key for user data stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn create_listen_key(&self) -> AsterFuturesHttpResult<ListenKeyResponse> {
        self.post::<(), ListenKeyResponse>("listenKey", None, None, true, false)
            .await
    }

    /// Keeps alive an existing listen key.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn keepalive_listen_key(&self, listen_key: &str) -> AsterFuturesHttpResult<()> {
        let params = ListenKeyParams {
            listen_key: listen_key.to_string(),
        };
        let _: serde_json::Value = self
            .request_put("listenKey", Some(&params), true, false)
            .await?;
        Ok(())
    }

    /// Closes an existing listen key.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn close_listen_key(&self, listen_key: &str) -> AsterFuturesHttpResult<()> {
        let params = ListenKeyParams {
            listen_key: listen_key.to_string(),
        };
        let _: serde_json::Value = self
            .request_delete("listenKey", Some(&params), true, false)
            .await?;
        Ok(())
    }

    /// Fetches account information including balances and positions.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_account(&self) -> AsterFuturesHttpResult<AsterFuturesAccountInfo> {
        // USD-M uses /fapi/v2/account, COIN-M uses /dapi/v1/account
        let path = if self.api_path.starts_with("/fapi") {
            "/fapi/v2/account"
        } else {
            "/dapi/v1/account"
        };
        self.get::<(), _>(path, None, true, false).await
    }

    /// Fetches position risk information.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_positions(
        &self,
        params: &AsterPositionRiskParams,
    ) -> AsterFuturesHttpResult<Vec<AsterPositionRisk>> {
        // USD-M uses /fapi/v2/positionRisk, COIN-M uses /dapi/v1/positionRisk
        let path = if self.api_path.starts_with("/fapi") {
            "/fapi/v2/positionRisk"
        } else {
            "/dapi/v1/positionRisk"
        };
        self.get(path, Some(params), true, false).await
    }

    /// Fetches user trades for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_user_trades(
        &self,
        params: &AsterUserTradesParams,
    ) -> AsterFuturesHttpResult<Vec<AsterUserTrade>> {
        self.get("userTrades", Some(params), true, false).await
    }

    /// Queries a single order by order ID or client order ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_order(
        &self,
        params: &AsterOrderQueryParams,
    ) -> AsterFuturesHttpResult<AsterFuturesOrder> {
        self.get("order", Some(params), true, false).await
    }

    /// Queries all open orders.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_open_orders(
        &self,
        params: &AsterOpenOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesOrder>> {
        self.get("openOrders", Some(params), true, false).await
    }

    /// Queries all orders (including historical).
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_all_orders(
        &self,
        params: &AsterAllOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesOrder>> {
        self.get("allOrders", Some(params), true, false).await
    }

    /// Submits a new order.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn submit_order(
        &self,
        params: &AsterNewOrderParams,
    ) -> AsterFuturesHttpResult<AsterFuturesOrder> {
        self.post("order", Some(params), None, true, true).await
    }

    /// Submits multiple orders in a single request (up to 5 orders).
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn submit_order_list(
        &self,
        orders: &[BatchOrderItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        if orders.is_empty() {
            return Ok(Vec::new());
        }

        if orders.len() > 5 {
            return Err(AsterFuturesHttpError::ValidationError(
                "Batch order limit is 5 orders maximum".to_string(),
            ));
        }

        self.batch_request("batchOrders", orders, true).await
    }

    /// Modifies an existing order (price and quantity only).
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn modify_order(
        &self,
        params: &AsterModifyOrderParams,
    ) -> AsterFuturesHttpResult<AsterFuturesOrder> {
        self.request_put("order", Some(params), true, true).await
    }

    /// Modifies multiple orders in a single request (up to 5 orders).
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn batch_modify_orders(
        &self,
        modifies: &[BatchModifyItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        if modifies.is_empty() {
            return Ok(Vec::new());
        }

        if modifies.len() > 5 {
            return Err(AsterFuturesHttpError::ValidationError(
                "Batch modify limit is 5 orders maximum".to_string(),
            ));
        }

        self.batch_request_put("batchOrders", modifies, true).await
    }

    /// Cancels an existing order.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_order(
        &self,
        params: &AsterCancelOrderParams,
    ) -> AsterFuturesHttpResult<AsterFuturesOrder> {
        self.request_delete("order", Some(params), true, true).await
    }

    /// Cancels all open orders for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_all_orders(
        &self,
        params: &AsterCancelAllOrdersParams,
    ) -> AsterFuturesHttpResult<AsterCancelAllOrdersResponse> {
        self.request_delete("allOpenOrders", Some(params), true, true)
            .await
    }

    /// Cancels multiple orders in a single request (up to 5 orders).
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn batch_cancel_orders(
        &self,
        cancels: &[BatchCancelItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        if cancels.is_empty() {
            return Ok(Vec::new());
        }

        if cancels.len() > 5 {
            return Err(AsterFuturesHttpError::ValidationError(
                "Batch cancel limit is 5 orders maximum".to_string(),
            ));
        }

        self.batch_request_delete("batchOrders", cancels, true)
            .await
    }

    /// Submits a new algo order (conditional order).
    ///
    /// Algo orders include STOP_MARKET, STOP (stop-limit), TAKE_PROFIT, TAKE_PROFIT_MARKET,
    /// and TRAILING_STOP_MARKET order types.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn submit_algo_order(
        &self,
        params: &AsterNewAlgoOrderParams,
    ) -> AsterFuturesHttpResult<AsterFuturesAlgoOrder> {
        self.post("algoOrder", Some(params), None, true, true).await
    }

    /// Cancels an algo order.
    ///
    /// Must provide either `algo_id` or `client_algo_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_algo_order(
        &self,
        params: &AsterAlgoOrderQueryParams,
    ) -> AsterFuturesHttpResult<AsterFuturesAlgoOrderCancelResponse> {
        self.request_delete("algoOrder", Some(params), true, true)
            .await
    }

    /// Queries a single algo order.
    ///
    /// Must provide either `algo_id` or `client_algo_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_algo_order(
        &self,
        params: &AsterAlgoOrderQueryParams,
    ) -> AsterFuturesHttpResult<AsterFuturesAlgoOrder> {
        self.get("algoOrder", Some(params), true, false).await
    }

    /// Queries all open algo orders.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_open_algo_orders(
        &self,
        params: &AsterOpenAlgoOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesAlgoOrder>> {
        self.get("openAlgoOrders", Some(params), true, false).await
    }

    /// Queries all algo orders including historical (7-day limit).
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_all_algo_orders(
        &self,
        params: &AsterAllAlgoOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesAlgoOrder>> {
        self.get("allAlgoOrders", Some(params), true, false).await
    }

    /// Cancels all open algo orders for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_all_algo_orders(
        &self,
        params: &AsterCancelAllAlgoOrdersParams,
    ) -> AsterFuturesHttpResult<AsterCancelAllOrdersResponse> {
        self.request_delete("algoOpenOrders", Some(params), true, true)
            .await
    }
}

/// Response wrapper for mark price endpoint.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MarkPriceResponse {
    Single(AsterFuturesMarkPrice),
    Multiple(Vec<AsterFuturesMarkPrice>),
}

impl From<MarkPriceResponse> for Vec<AsterFuturesMarkPrice> {
    fn from(response: MarkPriceResponse) -> Self {
        match response {
            MarkPriceResponse::Single(price) => vec![price],
            MarkPriceResponse::Multiple(prices) => prices,
        }
    }
}

struct RateLimitConfig {
    default_quota: Option<Quota>,
    keyed_quotas: Vec<(String, Quota)>,
    order_keys: Vec<String>,
}

/// In-memory cache entry for Aster Futures instruments.
#[derive(Clone, Debug)]
pub enum AsterFuturesInstrument {
    /// USD-M futures symbol.
    UsdM(AsterFuturesUsdSymbol),
    /// COIN-M futures symbol.
    CoinM(AsterFuturesCoinSymbol),
}

impl AsterFuturesInstrument {
    /// Returns the symbol name for the instrument.
    #[must_use]
    pub const fn symbol(&self) -> Ustr {
        match self {
            Self::UsdM(s) => s.symbol,
            Self::CoinM(s) => s.symbol,
        }
    }

    /// Returns the price precision for the instrument.
    #[must_use]
    pub const fn price_precision(&self) -> i32 {
        match self {
            Self::UsdM(s) => s.price_precision,
            Self::CoinM(s) => s.price_precision,
        }
    }

    /// Returns the quantity precision for the instrument.
    #[must_use]
    pub const fn quantity_precision(&self) -> i32 {
        match self {
            Self::UsdM(s) => s.quantity_precision,
            Self::CoinM(s) => s.quantity_precision,
        }
    }

    /// Returns the Nautilus-formatted instrument ID.
    #[must_use]
    pub fn id(&self) -> InstrumentId {
        match self {
            Self::UsdM(s) => format_instrument_id(&s.symbol, AsterProductType::UsdM),
            Self::CoinM(s) => format_instrument_id(&s.symbol, AsterProductType::CoinM),
        }
    }

    /// Returns the quote currency for the instrument.
    #[must_use]
    pub fn quote_currency(&self) -> Currency {
        let quote_asset = match self {
            Self::UsdM(s) => &s.quote_asset,
            Self::CoinM(s) => &s.quote_asset,
        };
        Currency::from(quote_asset.as_str())
    }
}

/// Aster Futures HTTP client for USD-M and COIN-M perpetuals.
#[derive(Debug, Clone)]
pub struct AsterFuturesHttpClient {
    inner: Arc<AsterRawFuturesHttpClient>,
    product_type: AsterProductType,
    clock: &'static AtomicTime,
    instruments: Arc<DashMap<Ustr, AsterFuturesInstrument>>,
    treat_expired_as_canceled: bool,
}

impl AsterFuturesHttpClient {
    /// Creates a new [`AsterFuturesHttpClient`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the product type is invalid or HTTP client creation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        product_type: AsterProductType,
        environment: AsterEnvironment,
        clock: &'static AtomicTime,
        api_key: Option<String>,
        api_secret: Option<String>,
        base_url_override: Option<String>,
        recv_window: Option<u64>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
        treat_expired_as_canceled: bool,
    ) -> AsterFuturesHttpResult<Self> {
        Self::new_with_local_addr(
            product_type,
            environment,
            clock,
            api_key,
            api_secret,
            base_url_override,
            recv_window,
            timeout_secs,
            proxy_url,
            treat_expired_as_canceled,
            None,
        )
    }

    /// Like [`new`](Self::new) but accepts an optional `local_addr` to pin outbound
    /// TCP connections to a specific source IP.
    ///
    /// # Errors
    ///
    /// Returns an error if the product type is invalid or HTTP client creation fails.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_local_addr(
        product_type: AsterProductType,
        environment: AsterEnvironment,
        clock: &'static AtomicTime,
        api_key: Option<String>,
        api_secret: Option<String>,
        base_url_override: Option<String>,
        recv_window: Option<u64>,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
        treat_expired_as_canceled: bool,
        local_addr: Option<std::net::IpAddr>,
    ) -> AsterFuturesHttpResult<Self> {
        match product_type {
            AsterProductType::UsdM | AsterProductType::CoinM => {}
            _ => {
                return Err(AsterFuturesHttpError::ValidationError(format!(
                    "AsterFuturesHttpClient requires UsdM or CoinM product type, was {product_type:?}"
                )));
            }
        }

        let raw = AsterRawFuturesHttpClient::new_with_local_addr(
            product_type,
            environment,
            api_key,
            api_secret,
            base_url_override,
            recv_window,
            timeout_secs,
            proxy_url,
            local_addr,
        )?;

        Ok(Self {
            inner: Arc::new(raw),
            product_type,
            clock,
            instruments: Arc::new(DashMap::new()),
            treat_expired_as_canceled,
        })
    }

    /// Returns the product type (UsdM or CoinM).
    #[must_use]
    pub const fn product_type(&self) -> AsterProductType {
        self.product_type
    }

    /// Returns a reference to the inner raw HTTP client.
    #[must_use]
    pub fn inner(&self) -> &AsterRawFuturesHttpClient {
        &self.inner
    }

    /// Returns a clone of the instruments cache Arc.
    #[must_use]
    pub fn instruments_cache(&self) -> Arc<DashMap<Ustr, AsterFuturesInstrument>> {
        Arc::clone(&self.instruments)
    }

    /// Returns server time.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn server_time(&self) -> AsterFuturesHttpResult<AsterServerTime> {
        self.inner
            .get::<_, AsterServerTime>("time", None::<&()>, false, false)
            .await
    }

    /// Sets leverage for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn set_leverage(
        &self,
        params: &AsterSetLeverageParams,
    ) -> AsterFuturesHttpResult<AsterLeverageResponse> {
        self.inner.set_leverage(params).await
    }

    /// Sets margin type for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn set_margin_type(
        &self,
        params: &AsterSetMarginTypeParams,
    ) -> AsterFuturesHttpResult<serde_json::Value> {
        self.inner.set_margin_type(params).await
    }

    /// Queries hedge mode (dual side position) setting.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_hedge_mode(&self) -> AsterFuturesHttpResult<AsterHedgeModeResponse> {
        self.inner.query_hedge_mode().await
    }

    /// Creates a listen key for user data stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn create_listen_key(&self) -> AsterFuturesHttpResult<ListenKeyResponse> {
        self.inner.create_listen_key().await
    }

    /// Keeps alive an existing listen key.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn keepalive_listen_key(&self, listen_key: &str) -> AsterFuturesHttpResult<()> {
        self.inner.keepalive_listen_key(listen_key).await
    }

    /// Closes an existing listen key.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn close_listen_key(&self, listen_key: &str) -> AsterFuturesHttpResult<()> {
        self.inner.close_listen_key(listen_key).await
    }

    /// Fetches exchange information and populates the instrument cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the product type is invalid.
    pub async fn exchange_info(&self) -> AsterFuturesHttpResult<()> {
        match self.product_type {
            AsterProductType::UsdM => {
                let info: AsterFuturesUsdExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;
                for symbol in info.symbols {
                    self.instruments
                        .insert(symbol.symbol, AsterFuturesInstrument::UsdM(symbol));
                }
            }
            AsterProductType::CoinM => {
                let info: AsterFuturesCoinExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;
                for symbol in info.symbols {
                    self.instruments
                        .insert(symbol.symbol, AsterFuturesInstrument::CoinM(symbol));
                }
            }
            _ => {
                return Err(AsterFuturesHttpError::ValidationError(
                    "Invalid product type for futures".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Fetches exchange info and returns the current status of each symbol.
    ///
    /// Builds a fresh status snapshot from the response without disturbing the
    /// shared instruments cache, so a transient failure does not break other
    /// HTTP operations that depend on cached precision data.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the product type is invalid.
    pub async fn request_symbol_statuses(
        &self,
    ) -> AsterFuturesHttpResult<AHashMap<Ustr, MarketStatusAction>> {
        let mut statuses = AHashMap::new();

        match self.product_type {
            AsterProductType::UsdM => {
                let info: AsterFuturesUsdExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;
                for symbol in &info.symbols {
                    statuses.insert(symbol.symbol, MarketStatusAction::from(symbol.status));
                }
            }
            AsterProductType::CoinM => {
                let info: AsterFuturesCoinExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;
                for symbol in &info.symbols {
                    let action = symbol
                        .contract_status
                        .map_or(MarketStatusAction::NotAvailableForTrading, Into::into);
                    statuses.insert(symbol.symbol, action);
                }
            }
            _ => {
                return Err(AsterFuturesHttpError::ValidationError(
                    "Invalid product type for futures".to_string(),
                ));
            }
        }

        Ok(statuses)
    }

    /// Fetches exchange information and returns parsed Nautilus instruments.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the product type is invalid.
    pub async fn request_instruments(&self) -> AsterFuturesHttpResult<Vec<InstrumentAny>> {
        let ts_init = UnixNanos::default();

        let instruments = match self.product_type {
            AsterProductType::UsdM => {
                let info: AsterFuturesUsdExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;

                let mut instruments = Vec::with_capacity(info.symbols.len());

                for symbol in info.symbols {
                    // Cache symbol for precision lookups
                    self.instruments.insert(
                        symbol.symbol,
                        AsterFuturesInstrument::UsdM(symbol.clone()),
                    );

                    match parse_usdm_instrument(&symbol, ts_init, ts_init) {
                        Ok(instrument) => instruments.push(instrument),
                        Err(e) => {
                            log::debug!(
                                "Skipping symbol during instrument parsing: symbol={}, error={e}",
                                symbol.symbol
                            );
                        }
                    }
                }

                log::info!(
                    "Loaded USD-M perpetual instruments: count={}",
                    instruments.len()
                );
                instruments
            }
            AsterProductType::CoinM => {
                let info: AsterFuturesCoinExchangeInfo = self
                    .inner
                    .get("exchangeInfo", None::<&()>, false, false)
                    .await?;

                let mut instruments = Vec::with_capacity(info.symbols.len());
                for symbol in info.symbols {
                    // Cache symbol for precision lookups
                    self.instruments.insert(
                        symbol.symbol,
                        AsterFuturesInstrument::CoinM(symbol.clone()),
                    );

                    match parse_coinm_instrument(&symbol, ts_init, ts_init) {
                        Ok(instrument) => instruments.push(instrument),
                        Err(e) => {
                            log::debug!(
                                "Skipping symbol during instrument parsing: symbol={}, error={e}",
                                symbol.symbol
                            );
                        }
                    }
                }

                log::info!(
                    "Loaded COIN-M perpetual instruments: count={}",
                    instruments.len()
                );
                instruments
            }
            _ => {
                return Err(AsterFuturesHttpError::ValidationError(
                    "Invalid product type for futures".to_string(),
                ));
            }
        };

        Ok(instruments)
    }

    /// Fetches 24hr ticker statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn ticker_24h(
        &self,
        params: &AsterTicker24hrParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesTicker24hr>> {
        self.inner.ticker_24h(params).await
    }

    /// Fetches best bid/ask prices.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn book_ticker(
        &self,
        params: &AsterBookTickerParams,
    ) -> AsterFuturesHttpResult<Vec<AsterBookTicker>> {
        self.inner.book_ticker(params).await
    }

    /// Fetches price ticker.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn price_ticker(
        &self,
        symbol: Option<&str>,
    ) -> AsterFuturesHttpResult<Vec<AsterPriceTicker>> {
        self.inner.price_ticker(symbol).await
    }

    /// Fetches order book depth.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn depth(
        &self,
        params: &AsterDepthParams,
    ) -> AsterFuturesHttpResult<AsterOrderBook> {
        self.inner.depth(params).await
    }

    /// Fetches mark price and funding rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn mark_price(
        &self,
        params: &AsterMarkPriceParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesMarkPrice>> {
        self.inner.mark_price(params).await
    }

    /// Fetches funding rate history.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn funding_rate(
        &self,
        params: &AsterFundingRateParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFundingRate>> {
        self.inner.funding_rate(params).await
    }

    /// Fetches current open interest for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn open_interest(
        &self,
        params: &AsterOpenInterestParams,
    ) -> AsterFuturesHttpResult<AsterOpenInterest> {
        self.inner.open_interest(params).await
    }

    /// Queries a single order by order ID or client order ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_order(
        &self,
        params: &AsterOrderQueryParams,
    ) -> AsterFuturesHttpResult<AsterFuturesOrder> {
        self.inner.query_order(params).await
    }

    /// Queries all open orders.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_open_orders(
        &self,
        params: &AsterOpenOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesOrder>> {
        self.inner.query_open_orders(params).await
    }

    /// Queries all orders (including historical).
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_all_orders(
        &self,
        params: &AsterAllOrdersParams,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesOrder>> {
        self.inner.query_all_orders(params).await
    }

    /// Fetches account information including balances and positions.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_account(&self) -> AsterFuturesHttpResult<AsterFuturesAccountInfo> {
        self.inner.query_account().await
    }

    /// Fetches position risk information.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_positions(
        &self,
        params: &AsterPositionRiskParams,
    ) -> AsterFuturesHttpResult<Vec<AsterPositionRisk>> {
        self.inner.query_positions(params).await
    }

    /// Fetches user trades for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_user_trades(
        &self,
        params: &AsterUserTradesParams,
    ) -> AsterFuturesHttpResult<Vec<AsterUserTrade>> {
        self.inner.query_user_trades(params).await
    }

    /// Submits a new order.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The instrument is not cached.
    /// - The order type or time-in-force is unsupported.
    /// - Stop orders are submitted without a trigger price.
    /// - The request fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_order(
        &self,
        account_id: AccountId,
        instrument_id: InstrumentId,
        client_order_id: ClientOrderId,
        order_side: OrderSide,
        order_type: OrderType,
        quantity: Quantity,
        time_in_force: TimeInForce,
        price: Option<Price>,
        trigger_price: Option<Price>,
        reduce_only: bool,
        post_only: bool,
        position_side: Option<AsterPositionSide>,
        price_match: Option<AsterPriceMatch>,
        // Aster routes trailing stop params through regular order API (no algo endpoint)
        activation_price: Option<Price>,
        callback_rate: Option<String>,
        working_type: Option<AsterWorkingType>,
    ) -> anyhow::Result<OrderStatusReport> {
        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;

        let aster_side = AsterSide::try_from(order_side)?;
        let aster_order_type = order_type_to_aster_futures(order_type)?;
        let aster_tif = if post_only {
            AsterTimeInForce::Gtx
        } else {
            AsterTimeInForce::try_from(time_in_force)?
        };

        let requires_trigger_price = matches!(
            order_type,
            OrderType::StopMarket
                | OrderType::StopLimit
                | OrderType::TrailingStopMarket
                | OrderType::MarketIfTouched
                | OrderType::LimitIfTouched
        );

        if requires_trigger_price && trigger_price.is_none() {
            anyhow::bail!("Order type {order_type:?} requires a trigger price");
        }

        // MARKET and STOP_MARKET orders don't accept timeInForce
        let requires_time_in_force = matches!(
            order_type,
            OrderType::Limit | OrderType::StopLimit | OrderType::LimitIfTouched
        );

        let qty_str = quantity.to_string();
        let price_str = if price_match.is_some() {
            None
        } else {
            price.map(|p| p.to_string())
        };
        let stop_price_str = trigger_price.map(|p| p.to_string());
        let client_id_str = encode_broker_id(&client_order_id, ASTER_NAUTILUS_FUTURES_BROKER_ID);

        let params = AsterNewOrderParams {
            symbol,
            side: aster_side,
            order_type: aster_order_type,
            time_in_force: if requires_time_in_force {
                Some(aster_tif)
            } else {
                None
            },
            quantity: Some(qty_str),
            price: price_str,
            new_client_order_id: Some(client_id_str),
            stop_price: stop_price_str,
            reduce_only: if reduce_only { Some(true) } else { None },
            position_side,
            close_position: None,
            activation_price: activation_price.map(|p| p.to_string()),
            callback_rate,
            working_type,
            price_protect: None,
            new_order_resp_type: None,
            good_till_date: None,
            recv_window: None,
            price_match,
            self_trade_prevention_mode: None,
        };

        let order = self.inner.submit_order(&params).await?;
        let ts_init = self.clock.get_time_ns();
        order.to_order_status_report(
            account_id,
            instrument_id,
            size_precision,
            self.treat_expired_as_canceled,
            ts_init,
        )
    }

    /// Submits an algo order (conditional order) to the Aster Algo Service.
    ///
    /// As of 2025-12-09, Aster migrated conditional order types to the Algo Service API.
    /// This method handles StopMarket, StopLimit, MarketIfTouched, LimitIfTouched,
    /// and TrailingStopMarket orders.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The order type requires a trigger price but none is provided.
    /// - The instrument is not cached.
    /// - The request fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_algo_order(
        &self,
        account_id: AccountId,
        instrument_id: InstrumentId,
        client_order_id: ClientOrderId,
        order_side: OrderSide,
        order_type: OrderType,
        quantity: Quantity,
        time_in_force: TimeInForce,
        price: Option<Price>,
        trigger_price: Option<Price>,
        reduce_only: bool,
        close_position: bool,
        position_side: Option<AsterPositionSide>,
        activation_price: Option<Price>,
        callback_rate: Option<String>,
        working_type: Option<AsterWorkingType>,
    ) -> anyhow::Result<OrderStatusReport> {
        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;

        let aster_side = AsterSide::try_from(order_side)?;
        let aster_order_type = order_type_to_aster_futures(order_type)?;
        let aster_tif = AsterTimeInForce::try_from(time_in_force)?;

        anyhow::ensure!(
            trigger_price.is_some(),
            "Algo order type {order_type:?} requires a trigger price"
        );

        // Limit orders require time in force
        let requires_time_in_force =
            matches!(order_type, OrderType::StopLimit | OrderType::LimitIfTouched);

        let price_str = price.map(|p| p.to_string());
        let trigger_price_str = trigger_price.map(|p| p.to_string());
        let client_id_str = encode_broker_id(&client_order_id, ASTER_NAUTILUS_FUTURES_BROKER_ID);

        // closePosition is mutually exclusive with quantity and reduceOnly
        let params = if close_position {
            AsterNewAlgoOrderParams {
                symbol,
                side: aster_side,
                order_type: aster_order_type,
                algo_type: AsterAlgoType::Conditional,
                position_side,
                quantity: None,
                price: price_str,
                trigger_price: trigger_price_str,
                time_in_force: if requires_time_in_force {
                    Some(aster_tif)
                } else {
                    None
                },
                working_type,
                close_position: Some(true),
                price_protect: None,
                reduce_only: None,
                activation_price: activation_price.map(|p| p.to_string()),
                callback_rate,
                client_algo_id: Some(client_id_str),
                good_till_date: None,
                recv_window: None,
            }
        } else {
            let qty_str = quantity.to_string();
            AsterNewAlgoOrderParams {
                symbol,
                side: aster_side,
                order_type: aster_order_type,
                algo_type: AsterAlgoType::Conditional,
                position_side,
                quantity: Some(qty_str),
                price: price_str,
                trigger_price: trigger_price_str,
                time_in_force: if requires_time_in_force {
                    Some(aster_tif)
                } else {
                    None
                },
                working_type,
                close_position: None,
                price_protect: None,
                reduce_only: if reduce_only { Some(true) } else { None },
                activation_price: activation_price.map(|p| p.to_string()),
                callback_rate,
                client_algo_id: Some(client_id_str),
                good_till_date: None,
                recv_window: None,
            }
        };

        let order = self.inner.submit_algo_order(&params).await?;
        let ts_init = self.clock.get_time_ns();
        order.to_order_status_report(account_id, instrument_id, size_precision, ts_init)
    }

    /// Submits multiple orders in a single request (up to 5 orders).
    ///
    /// Each order in the batch is processed independently. The response contains
    /// the result for each order, which can be either a success or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn submit_order_list(
        &self,
        orders: &[BatchOrderItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.inner.submit_order_list(orders).await
    }

    /// Modifies an existing order (price and quantity only).
    ///
    /// Either `venue_order_id` or `client_order_id` must be provided.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Neither venue_order_id nor client_order_id is provided.
    /// - The instrument is not cached.
    /// - The request fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn modify_order(
        &self,
        account_id: AccountId,
        instrument_id: InstrumentId,
        venue_order_id: Option<VenueOrderId>,
        client_order_id: Option<ClientOrderId>,
        order_side: OrderSide,
        quantity: Quantity,
        price: Price,
    ) -> anyhow::Result<OrderStatusReport> {
        anyhow::ensure!(
            venue_order_id.is_some() || client_order_id.is_some(),
            "Either venue_order_id or client_order_id must be provided"
        );

        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;

        let aster_side = AsterSide::try_from(order_side)?;

        let order_id = venue_order_id
            .map(|id| id.inner().parse::<i64>())
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid venue order ID"))?;

        let params = AsterModifyOrderParams {
            symbol,
            order_id,
            orig_client_order_id: client_order_id
                .map(|id| encode_broker_id(&id, ASTER_NAUTILUS_FUTURES_BROKER_ID)),
            side: aster_side,
            quantity: quantity.to_string(),
            price: price.to_string(),
            recv_window: None,
        };

        let order = self.inner.modify_order(&params).await?;
        let ts_init = self.clock.get_time_ns();
        order.to_order_status_report(
            account_id,
            instrument_id,
            size_precision,
            self.treat_expired_as_canceled,
            ts_init,
        )
    }

    /// Modifies multiple orders in a single request (up to 5 orders).
    ///
    /// Each modify in the batch is processed independently. The response contains
    /// the result for each modify, which can be either a success or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn batch_modify_orders(
        &self,
        modifies: &[BatchModifyItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.inner.batch_modify_orders(modifies).await
    }

    /// Cancels an order by venue order ID or client order ID.
    ///
    /// Either `venue_order_id` or `client_order_id` must be provided.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Neither venue_order_id nor client_order_id is provided.
    /// - The request fails.
    pub async fn cancel_order(
        &self,
        instrument_id: InstrumentId,
        venue_order_id: Option<VenueOrderId>,
        client_order_id: Option<ClientOrderId>,
    ) -> anyhow::Result<VenueOrderId> {
        anyhow::ensure!(
            venue_order_id.is_some() || client_order_id.is_some(),
            "Either venue_order_id or client_order_id must be provided"
        );

        let symbol = format_aster_symbol(&instrument_id);

        let order_id = venue_order_id
            .map(|id| id.inner().parse::<i64>())
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid venue order ID"))?;

        let params = AsterCancelOrderParams {
            symbol,
            order_id,
            orig_client_order_id: client_order_id
                .map(|id| encode_broker_id(&id, ASTER_NAUTILUS_FUTURES_BROKER_ID)),
            recv_window: None,
        };

        let order = self.inner.cancel_order(&params).await?;
        Ok(VenueOrderId::new(order.order_id.to_string()))
    }

    /// Cancels an algo order (conditional order) via the Aster Algo Service.
    ///
    /// Use the `client_algo_id` which corresponds to the `client_order_id` used
    /// when submitting the algo order.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_algo_order(&self, client_order_id: ClientOrderId) -> anyhow::Result<()> {
        let params = AsterAlgoOrderQueryParams {
            algo_id: None,
            client_algo_id: Some(encode_broker_id(
                &client_order_id,
                ASTER_NAUTILUS_FUTURES_BROKER_ID,
            )),
            recv_window: None,
        };

        let response = self.inner.cancel_algo_order(&params).await?;
        if response.code.parse::<i32>().unwrap_or(0) == 200 {
            Ok(())
        } else {
            anyhow::bail!(
                "Cancel algo order failed: code={}, msg={}",
                response.code,
                response.msg
            )
        }
    }

    /// Cancels all open orders for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_all_orders(
        &self,
        instrument_id: InstrumentId,
    ) -> anyhow::Result<Vec<VenueOrderId>> {
        let symbol = format_aster_symbol(&instrument_id);

        let params = AsterCancelAllOrdersParams {
            symbol,
            recv_window: None,
        };

        let response = self.inner.cancel_all_orders(&params).await?;
        if response.code == 200 {
            Ok(vec![])
        } else {
            anyhow::bail!("Cancel all orders failed: {}", response.msg);
        }
    }

    /// Cancels all open algo orders for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn cancel_all_algo_orders(&self, instrument_id: InstrumentId) -> anyhow::Result<()> {
        let symbol = format_aster_symbol(&instrument_id);

        let params = AsterCancelAllAlgoOrdersParams {
            symbol,
            recv_window: None,
        };

        let response = self.inner.cancel_all_algo_orders(&params).await?;
        if response.code == 200 {
            Ok(())
        } else {
            anyhow::bail!("Cancel all algo orders failed: {}", response.msg);
        }
    }

    /// Cancels multiple orders in a single request (up to 5 orders).
    ///
    /// Each cancel in the batch is processed independently. The response contains
    /// the result for each cancel, which can be either a success or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch exceeds 5 orders or the request fails.
    pub async fn batch_cancel_orders(
        &self,
        cancels: &[BatchCancelItem],
    ) -> AsterFuturesHttpResult<Vec<BatchOrderResult>> {
        self.inner.batch_cancel_orders(cancels).await
    }

    /// Queries open algo orders (conditional orders).
    ///
    /// Returns all open algo orders, optionally filtered by symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_open_algo_orders(
        &self,
        instrument_id: Option<InstrumentId>,
    ) -> AsterFuturesHttpResult<Vec<AsterFuturesAlgoOrder>> {
        let symbol = instrument_id.map(|id| format_aster_symbol(&id));

        let params = AsterOpenAlgoOrdersParams {
            symbol,
            recv_window: None,
        };

        self.inner.query_open_algo_orders(&params).await
    }

    /// Queries a single algo order by client_order_id.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn query_algo_order(
        &self,
        client_order_id: ClientOrderId,
    ) -> AsterFuturesHttpResult<AsterFuturesAlgoOrder> {
        let params = AsterAlgoOrderQueryParams {
            algo_id: None,
            client_algo_id: Some(encode_broker_id(
                &client_order_id,
                ASTER_NAUTILUS_FUTURES_BROKER_ID,
            )),
            recv_window: None,
        };

        self.inner.query_algo_order(&params).await
    }

    /// Returns the size precision for an instrument from the cache.
    fn get_size_precision(&self, symbol: &str) -> anyhow::Result<u8> {
        let instrument = self
            .instruments
            .get(&Ustr::from(symbol))
            .ok_or_else(|| anyhow::anyhow!("Instrument not found in cache: {symbol}"))?;

        let precision = match instrument.value() {
            AsterFuturesInstrument::UsdM(s) => s.quantity_precision,
            AsterFuturesInstrument::CoinM(s) => s.quantity_precision,
        };

        Ok(precision as u8)
    }

    /// Returns the price precision for an instrument from the cache.
    fn get_price_precision(&self, symbol: &str) -> anyhow::Result<u8> {
        let instrument = self
            .instruments
            .get(&Ustr::from(symbol))
            .ok_or_else(|| anyhow::anyhow!("Instrument not found in cache: {symbol}"))?;

        let precision = match instrument.value() {
            AsterFuturesInstrument::UsdM(s) => s.price_precision,
            AsterFuturesInstrument::CoinM(s) => s.price_precision,
        };

        Ok(precision as u8)
    }

    /// Requests the current account state.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or parsing fails.
    pub async fn request_account_state(
        &self,
        account_id: AccountId,
    ) -> anyhow::Result<AccountState> {
        let ts_init = UnixNanos::default();
        let account_info = self.inner.query_account().await?;
        account_info.to_account_state(account_id, ts_init)
    }

    /// Requests a single order status report.
    ///
    /// Either `venue_order_id` or `client_order_id` must be provided.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or parsing fails.
    pub async fn request_order_status_report(
        &self,
        account_id: AccountId,
        instrument_id: InstrumentId,
        venue_order_id: Option<VenueOrderId>,
        client_order_id: Option<ClientOrderId>,
    ) -> anyhow::Result<OrderStatusReport> {
        anyhow::ensure!(
            venue_order_id.is_some() || client_order_id.is_some(),
            "Either venue_order_id or client_order_id must be provided"
        );

        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;

        let order_id = venue_order_id
            .map(|id| id.inner().parse::<i64>())
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid venue order ID"))?;

        let orig_client_order_id =
            client_order_id.map(|id| encode_broker_id(&id, ASTER_NAUTILUS_FUTURES_BROKER_ID));

        let params = AsterOrderQueryParams {
            symbol,
            order_id,
            orig_client_order_id,
            recv_window: None,
        };

        let order = self.inner.query_order(&params).await?;
        let ts_init = self.clock.get_time_ns();
        order.to_order_status_report(
            account_id,
            instrument_id,
            size_precision,
            self.treat_expired_as_canceled,
            ts_init,
        )
    }

    /// Requests order status reports for open orders.
    ///
    /// If `instrument_id` is None, returns all open orders.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or parsing fails.
    pub async fn request_order_status_reports(
        &self,
        account_id: AccountId,
        instrument_id: Option<InstrumentId>,
        open_only: bool,
    ) -> anyhow::Result<Vec<OrderStatusReport>> {
        let symbol = instrument_id.map(|id| format_aster_symbol(&id));

        let orders = if open_only {
            let params = AsterOpenOrdersParams {
                symbol: symbol.clone(),
                recv_window: None,
            };
            self.inner.query_open_orders(&params).await?
        } else {
            // For historical orders, symbol is required
            let symbol = symbol.ok_or_else(|| {
                anyhow::anyhow!("instrument_id is required for historical orders")
            })?;
            let params = AsterAllOrdersParams {
                symbol,
                order_id: None,
                start_time: None,
                end_time: None,
                limit: None,
                recv_window: None,
            };
            self.inner.query_all_orders(&params).await?
        };

        let ts_init = self.clock.get_time_ns();
        let mut reports = Vec::with_capacity(orders.len());

        for order in orders {
            let order_instrument_id = instrument_id.unwrap_or_else(|| {
                // Build instrument ID from order symbol
                let suffix = self.product_type.suffix();
                InstrumentId::from(format!("{}{}.ASTER", order.symbol, suffix))
            });

            let size_precision = self.get_size_precision(&order.symbol).unwrap_or(8); // Default precision if not in cache

            match order.to_order_status_report(
                account_id,
                order_instrument_id,
                size_precision,
                self.treat_expired_as_canceled,
                ts_init,
            ) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    log::warn!("Failed to parse order status report: {e}");
                }
            }
        }

        Ok(reports)
    }

    /// Requests fill reports for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or parsing fails.
    pub async fn request_fill_reports(
        &self,
        account_id: AccountId,
        instrument_id: InstrumentId,
        venue_order_id: Option<VenueOrderId>,
        start: Option<i64>,
        end: Option<i64>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<FillReport>> {
        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;
        let price_precision = self.get_price_precision(&symbol)?;

        let order_id = venue_order_id
            .map(|id| id.inner().parse::<i64>())
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid venue order ID"))?;

        let params = AsterUserTradesParams {
            symbol,
            order_id,
            start_time: start,
            end_time: end,
            from_id: None,
            limit,
            recv_window: None,
        };

        let trades = self.inner.query_user_trades(&params).await?;

        let ts_init = self.clock.get_time_ns();
        let mut reports = Vec::with_capacity(trades.len());

        for trade in trades {
            match trade.to_fill_report(
                account_id,
                instrument_id,
                price_precision,
                size_precision,
                ts_init,
            ) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    log::warn!("Failed to parse fill report: {e}");
                }
            }
        }

        Ok(reports)
    }

    /// Requests recent public trades for an instrument.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails, instrument is not cached, or parsing fails.
    pub async fn request_trades(
        &self,
        instrument_id: InstrumentId,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<TradeTick>> {
        let symbol = format_aster_symbol(&instrument_id);
        let size_precision = self.get_size_precision(&symbol)?;
        let price_precision = self.get_price_precision(&symbol)?;

        let params = AsterTradesParams { symbol, limit };

        let trades = self.inner.trades(&params).await?;
        let ts_init = UnixNanos::default();

        let mut result = Vec::with_capacity(trades.len());
        for trade in trades {
            let price: f64 = trade.price.parse().unwrap_or(0.0);
            let size: f64 = trade.qty.parse().unwrap_or(0.0);
            let ts_event = UnixNanos::from_millis(trade.time as u64);

            let aggressor_side = if trade.is_buyer_maker {
                AggressorSide::Seller
            } else {
                AggressorSide::Buyer
            };

            let tick = TradeTick::new(
                instrument_id,
                Price::new(price, price_precision),
                Quantity::new(size, size_precision),
                aggressor_side,
                TradeId::new(trade.id.to_string()),
                ts_event,
                ts_init,
            );
            result.push(tick);
        }

        Ok(result)
    }

    /// Requests bar (kline/candlestick) data for an instrument.
    ///
    /// # Errors
    ///
    /// Returns an error if the bar type is not supported, instrument is not cached,
    /// or the request fails.
    pub async fn request_bars(
        &self,
        bar_type: BarType,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<Bar>> {
        anyhow::ensure!(
            bar_type.aggregation_source() == AggregationSource::External,
            "Only EXTERNAL aggregation is supported"
        );

        let spec = bar_type.spec();
        let step = spec.step.get();
        let interval = match spec.aggregation {
            BarAggregation::Second => {
                anyhow::bail!("Aster Futures does not support second-level kline intervals")
            }
            BarAggregation::Minute => format!("{step}m"),
            BarAggregation::Hour => format!("{step}h"),
            BarAggregation::Day => format!("{step}d"),
            BarAggregation::Week => format!("{step}w"),
            BarAggregation::Month => format!("{step}M"),
            a => anyhow::bail!("Aster Futures does not support {a:?} aggregation"),
        };

        let symbol = format_aster_symbol(&bar_type.instrument_id());
        let price_precision = self.get_price_precision(&symbol)?;
        let size_precision = self.get_size_precision(&symbol)?;

        let params = AsterKlinesParams {
            symbol,
            interval,
            start_time: start.map(|dt| dt.timestamp_millis()),
            end_time: end.map(|dt| dt.timestamp_millis()),
            limit,
        };

        let klines = self.inner.klines(&params).await?;
        let ts_init = UnixNanos::default();

        let mut result = Vec::with_capacity(klines.len());
        for kline in klines {
            let open: f64 = kline.open.parse().unwrap_or(0.0);
            let high: f64 = kline.high.parse().unwrap_or(0.0);
            let low: f64 = kline.low.parse().unwrap_or(0.0);
            let close: f64 = kline.close.parse().unwrap_or(0.0);
            let volume: f64 = kline.volume.parse().unwrap_or(0.0);

            // close_time is end of interval, add 1ms for next bar's open
            let ts_event = UnixNanos::from_millis(kline.close_time as u64);

            let bar = Bar::new(
                bar_type,
                Price::new(open, price_precision),
                Price::new(high, price_precision),
                Price::new(low, price_precision),
                Price::new(close, price_precision),
                Quantity::new(volume, size_precision),
                ts_event,
                ts_init,
            );
            result.push(bar);
        }

        Ok(result)
    }
}

/// Checks if an order type requires the Algo Service API.
///
/// Aster does NOT support the `/fapi/v1/algo` endpoint (returns 404).
/// All conditional orders (stop, take-profit, trailing stop) go through
/// the regular `/fapi/v1/order` endpoint. Always returns `false`.
#[must_use]
pub fn is_algo_order_type(_order_type: OrderType) -> bool {
    false
}

/// Converts a Nautilus order type to a Aster Futures order type.
pub(crate) fn order_type_to_aster_futures(
    order_type: OrderType,
) -> anyhow::Result<AsterFuturesOrderType> {
    match order_type {
        OrderType::Market => Ok(AsterFuturesOrderType::Market),
        OrderType::Limit => Ok(AsterFuturesOrderType::Limit),
        OrderType::StopMarket => Ok(AsterFuturesOrderType::StopMarket),
        OrderType::StopLimit => Ok(AsterFuturesOrderType::Stop),
        OrderType::MarketIfTouched => Ok(AsterFuturesOrderType::TakeProfitMarket),
        OrderType::LimitIfTouched => Ok(AsterFuturesOrderType::TakeProfit),
        OrderType::TrailingStopMarket => Ok(AsterFuturesOrderType::TrailingStopMarket),
        _ => anyhow::bail!("Unsupported order type for Aster Futures: {order_type:?}"),
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::time::get_atomic_clock_realtime;
    use nautilus_network::http::{HttpStatus, StatusCode};
    use rstest::rstest;
    use tokio_util::bytes::Bytes;

    use super::*;

    #[rstest]
    fn test_rate_limit_config_usdm_has_request_weight_and_orders() {
        let config = AsterRawFuturesHttpClient::rate_limit_config(AsterProductType::UsdM);

        assert!(config.default_quota.is_some());
        assert_eq!(config.order_keys.len(), 2);
        assert!(config.order_keys.iter().any(|k| k.contains("Second")));
        assert!(config.order_keys.iter().any(|k| k.contains("Minute")));
    }

    #[rstest]
    fn test_rate_limit_config_coinm_has_request_weight_and_orders() {
        let config = AsterRawFuturesHttpClient::rate_limit_config(AsterProductType::CoinM);

        assert!(config.default_quota.is_some());
        assert_eq!(config.order_keys.len(), 2);
    }

    #[rstest]
    fn test_quota_from_unknown_interval_returns_none() {
        let quota = AsterRateLimitQuota {
            rate_limit_type: AsterRateLimitType::Orders,
            interval: AsterRateLimitInterval::Unknown,
            interval_num: 1,
            limit: 10,
        };

        assert!(AsterRawFuturesHttpClient::quota_from(&quota).is_none());
    }

    #[rstest]
    fn test_create_client_rejects_spot_product_type() {
        let result = AsterFuturesHttpClient::new(
            AsterProductType::Spot,
            AsterEnvironment::Mainnet,
            get_atomic_clock_realtime(),
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );

        assert!(result.is_err());
    }

    fn create_test_raw_client() -> AsterRawFuturesHttpClient {
        AsterRawFuturesHttpClient::new(
            AsterProductType::UsdM,
            AsterEnvironment::Mainnet,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("Failed to create test client")
    }

    #[rstest]
    fn test_parse_error_response_aster_error() {
        let client = create_test_raw_client();
        let response = HttpResponse {
            status: HttpStatus::new(StatusCode::BAD_REQUEST),
            headers: HashMap::new(),
            body: Bytes::from(r#"{"code":-1121,"msg":"Invalid symbol."}"#),
        };

        let result: AsterFuturesHttpResult<()> = client.parse_error_response(&response);

        match result {
            Err(AsterFuturesHttpError::AsterError { code, message }) => {
                assert_eq!(code, -1121);
                assert_eq!(message, "Invalid symbol.");
            }
            other => panic!("Expected AsterError, was {other:?}"),
        }
    }

    // ---------- local_addr plumbing ----------

    #[rstest]
    fn test_aster_raw_http_client_local_addr_none_equivalent_to_new() {
        let a = AsterRawFuturesHttpClient::new(
            AsterProductType::UsdM,
            AsterEnvironment::Mainnet,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let b = AsterRawFuturesHttpClient::new_with_local_addr(
            AsterProductType::UsdM,
            AsterEnvironment::Mainnet,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(a.is_ok());
        assert!(b.is_ok());
    }

    #[rstest]
    fn test_aster_raw_http_client_local_addr_loopback_builds() {
        let result = AsterRawFuturesHttpClient::new_with_local_addr(
            AsterProductType::UsdM,
            AsterEnvironment::Mainnet,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        );
        assert!(
            result.is_ok(),
            "expected client to build with local_addr=127.0.0.1, was {result:?}"
        );
    }
}
