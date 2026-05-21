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

//! Configuration for WebSocket client connections.
//!
//! # Reconnection Strategy
//!
//! The default configuration uses unlimited reconnection attempts (`reconnect_max_attempts: None`).
//! This is intentional for trading systems because:
//! - Venues may be down for extended periods but eventually recover.
//! - Exponential backoff already prevents resource waste.
//! - Automatic recovery can be useful when manual intervention is not desirable.
//!
//! Use `Some(n)` primarily for testing, development, or non-critical connections.

use std::{fmt::Debug, net::IpAddr};

/// Configuration for WebSocket client connections.
///
/// This struct contains only static configuration settings. Runtime callbacks
/// (message handler, ping handler) are passed separately to `connect()`.
///
/// # Connection Modes
///
/// ## Handler Mode
///
/// - Use with [`crate::websocket::WebSocketClient::connect`].
/// - Pass a message handler to `connect()` to receive messages via callback.
/// - Client spawns internal task to read messages and call handler.
/// - Supports automatic reconnection with exponential backoff.
/// - Reconnection config fields (`reconnect_*`) are active.
/// - Best for long-lived connections, Python bindings, callback-based APIs.
///
/// ## Stream Mode
///
/// - Use with [`crate::websocket::WebSocketClient::connect_stream`].
/// - Returns a [`MessageReader`](super::types::MessageReader) stream for the caller to read from.
/// - **Does NOT support automatic reconnection** (reader owned by caller).
/// - Reconnection config fields are ignored.
/// - On disconnect, client transitions to CLOSED state and caller must manually reconnect.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.network", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.network")
)]
#[derive(Clone, Debug, bon::Builder)]
pub struct WebSocketConfig {
    /// The URL to connect to.
    pub url: String,
    /// The default headers.
    #[builder(default)]
    pub headers: Vec<(String, String)>,
    /// The optional heartbeat interval (seconds).
    pub heartbeat: Option<u64>,
    /// The optional heartbeat message.
    pub heartbeat_msg: Option<String>,
    /// The timeout (milliseconds) for reconnection attempts.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub reconnect_timeout_ms: Option<u64>,
    /// The initial reconnection delay (milliseconds) for reconnects.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub reconnect_delay_initial_ms: Option<u64>,
    /// The maximum reconnect delay (milliseconds) for exponential backoff.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub reconnect_delay_max_ms: Option<u64>,
    /// The exponential backoff factor for reconnection delays.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub reconnect_backoff_factor: Option<f64>,
    /// The maximum jitter (milliseconds) added to reconnection delays.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub reconnect_jitter_ms: Option<u64>,
    /// The maximum number of reconnection attempts before giving up.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    /// - `None`: Unlimited reconnection attempts (default, recommended for production).
    /// - `Some(n)`: After n failed attempts, transition to CLOSED state.
    pub reconnect_max_attempts: Option<u32>,
    /// The idle timeout (milliseconds) for the read task.
    /// When set, the read task will break and trigger reconnection if no data
    /// is received within this duration. Useful for detecting silently dead
    /// connections where the server stops sending without closing.
    /// **Note**: Only applies to handler mode. Ignored in stream mode.
    pub idle_timeout_ms: Option<u64>,
    /// Optional local IP address to bind outbound TCP connections to.
    ///
    /// When `Some(ip)`, the TCP socket is explicitly bound to this address before
    /// connecting — used to pin a single process to a specific source IP, e.g. when
    /// a venue rate-limits by source IP and the host has multiple IPs available.
    /// When `None` (default), the kernel selects the source IP from the routing table
    /// and the standard `connect_async_with_config` fast path is used.
    ///
    /// **Note**: For the `turmoil` test feature this field is accepted but ignored —
    /// the simulator routes via a virtual network without real bind semantics.
    pub local_addr: Option<IpAddr>,
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_local_addr_defaults_to_none_via_builder() {
        // Builder omits local_addr → should be None (Option default).
        let cfg = WebSocketConfig::builder()
            .url("ws://example.test".to_string())
            .build();
        assert!(cfg.local_addr.is_none());
    }

    #[rstest]
    fn test_local_addr_round_trips_via_builder() {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let cfg = WebSocketConfig::builder()
            .url("wss://example.test".to_string())
            .local_addr(ip)
            .build();
        assert_eq!(cfg.local_addr, Some(ip));
    }

    #[rstest]
    fn test_local_addr_survives_clone() {
        // WebSocketClientInner stores config and reads config.local_addr from
        // every reconnect call. Cloning the config (which happens implicitly
        // when state machines pass the config around) must preserve local_addr.
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let cfg = WebSocketConfig::builder()
            .url("wss://example.test".to_string())
            .local_addr(ip)
            .build();
        let cloned = cfg.clone();
        assert_eq!(cloned.local_addr, Some(ip));
    }

    #[rstest]
    fn test_local_addr_v6_round_trip() {
        // Verify IPv6 addresses round-trip correctly (separate code path on
        // the connect side: TcpSocket::new_v6 vs ::new_v4).
        let ip = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);
        let cfg = WebSocketConfig::builder()
            .url("wss://example.test".to_string())
            .local_addr(ip)
            .build();
        assert_eq!(cfg.local_addr, Some(ip));
    }
}
