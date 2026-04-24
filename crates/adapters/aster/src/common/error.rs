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

//! Adapter-level error types aggregating HTTP and WebSocket errors.

use std::fmt::Display;

/// Aster WebSocket streams error type shared by spot and futures clients.
#[derive(Debug)]
pub enum AsterWsError {
    /// General client error.
    ClientError(String),
    /// Authentication failed.
    AuthenticationError(String),
    /// Message parsing error.
    ParseError(String),
    /// Network or connection error.
    NetworkError(String),
    /// Operation timed out.
    Timeout(String),
}

impl Display for AsterWsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientError(msg) => write!(f, "Client error: {msg}"),
            Self::AuthenticationError(msg) => write!(f, "Authentication error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::Timeout(msg) => write!(f, "Timeout: {msg}"),
        }
    }
}

impl std::error::Error for AsterWsError {}

/// Result type for Aster WebSocket stream operations.
pub type AsterWsResult<T> = Result<T, AsterWsError>;

/// Adapter-level error aggregating HTTP and WebSocket errors.
#[derive(Debug, thiserror::Error)]
pub enum AsterError {
    /// A Futures HTTP API error.
    #[error("Futures HTTP error: {0}")]
    FuturesHttp(#[from] crate::futures::http::error::AsterFuturesHttpError),

    /// A WebSocket streams error.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] AsterWsError),

    /// A Futures WebSocket Trading API error.
    #[error("Futures WS API error: {0}")]
    FuturesWsApi(#[from] crate::futures::websocket::trading::error::AsterFuturesWsApiError),

    /// A configuration or build error.
    #[error("Config error: {0}")]
    Config(String),
}

/// Aster error codes indicating authentication or permission failures.
const ASTER_AUTH_ERROR_CODES: [i64; 3] = [
    -2015, // Invalid API-key, IP, or permissions for action
    -2014, // API-key format invalid
    -1022, // Signature for this request is not valid
];

/// Aster error codes indicating rate limiting or throttling.
const ASTER_RATE_LIMIT_ERROR_CODES: [i64; 2] = [
    -1003, // Too many requests; WAF limit violated
    -1015, // Too many new orders; rate limit violated
];

impl AsterError {
    /// Returns `true` if the error is likely transient and the operation can be retried.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::FuturesHttp(e) => match e {
                crate::futures::http::error::AsterFuturesHttpError::NetworkError(_)
                | crate::futures::http::error::AsterFuturesHttpError::Timeout(_) => true,
                crate::futures::http::error::AsterFuturesHttpError::AsterError {
                    code, ..
                } => ASTER_RATE_LIMIT_ERROR_CODES.contains(code),
                crate::futures::http::error::AsterFuturesHttpError::UnexpectedStatus {
                    status,
                    ..
                } => *status == 429 || *status >= 500,
                _ => false,
            },
            Self::WebSocket(e) => matches!(
                e,
                AsterWsError::NetworkError(_) | AsterWsError::Timeout(_)
            ),
            Self::FuturesWsApi(e) => matches!(
                e,
                crate::futures::websocket::trading::error::AsterFuturesWsApiError::ConnectionError(_)
            ),
            Self::Config(_) => false,
        }
    }

    /// Returns `true` if the error is fatal and requires intervention.
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::FuturesHttp(e) => match e {
                crate::futures::http::error::AsterFuturesHttpError::MissingCredentials => true,
                crate::futures::http::error::AsterFuturesHttpError::AsterError {
                    code, ..
                } => ASTER_AUTH_ERROR_CODES.contains(code),
                crate::futures::http::error::AsterFuturesHttpError::UnexpectedStatus {
                    status,
                    ..
                } => *status == 401 || *status == 403,
                _ => false,
            },
            Self::WebSocket(e) => {
                matches!(e, AsterWsError::AuthenticationError(_))
            }
            Self::FuturesWsApi(_) => false,
            Self::Config(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::futures::http::error::AsterFuturesHttpError;

    #[rstest]
    fn test_futures_http_network_error_is_retryable() {
        let err = AsterError::FuturesHttp(AsterFuturesHttpError::NetworkError(
            "connection refused".to_string(),
        ));
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[rstest]
    fn test_futures_http_missing_credentials_is_fatal() {
        let err = AsterError::FuturesHttp(AsterFuturesHttpError::MissingCredentials);
        assert!(err.is_fatal());
        assert!(!err.is_retryable());
    }

    #[rstest]
    fn test_ws_auth_error_is_fatal() {
        let err = AsterError::WebSocket(AsterWsError::AuthenticationError(
            "invalid key".to_string(),
        ));
        assert!(err.is_fatal());
        assert!(!err.is_retryable());
    }

    #[rstest]
    fn test_ws_network_error_is_retryable() {
        let err =
            AsterError::WebSocket(AsterWsError::NetworkError("connection lost".to_string()));
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[rstest]
    fn test_config_error_is_fatal() {
        let err = AsterError::Config("invalid product type".to_string());
        assert!(err.is_fatal());
        assert!(!err.is_retryable());
    }

    #[rstest]
    fn test_futures_http_auth_error_code_is_fatal() {
        let err = AsterError::FuturesHttp(AsterFuturesHttpError::AsterError {
            code: -2015,
            message: "Invalid API-key".to_string(),
        });
        assert!(err.is_fatal());
        assert!(!err.is_retryable());
    }

    #[rstest]
    fn test_futures_http_rate_limit_is_retryable() {
        let err = AsterError::FuturesHttp(AsterFuturesHttpError::AsterError {
            code: -1003,
            message: "Too many requests".to_string(),
        });
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[rstest]
    fn test_display_formatting() {
        let err = AsterError::FuturesHttp(AsterFuturesHttpError::AsterError {
            code: -1100,
            message: "Illegal characters found".to_string(),
        });
        let msg = err.to_string();
        assert!(msg.contains("Futures HTTP error"));
        assert!(msg.contains("-1100"));
    }
}
