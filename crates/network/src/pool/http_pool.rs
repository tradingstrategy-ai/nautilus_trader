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

//! Round-robin pool of [`HttpClient`]s, each bound to a different source IP.

use std::{
    net::IpAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{HttpClientTemplate, PickHint, PoolError};
use crate::http::HttpClient;

/// A fan of [`HttpClient`]s sharing the same template, each bound to a
/// distinct local source IP. Stateless REST is round-robined across slots
/// to amortise per-IP rate limits.
#[derive(Debug)]
pub struct HttpPool {
    slots: Vec<HttpClient>,
    rr_counter: AtomicUsize,
}

impl HttpPool {
    /// Build one [`HttpClient`] per address using the same template.
    ///
    /// Adapters typically build the template once with their default headers
    /// and REST quota, then call this to fan it out across every configured IP.
    ///
    /// # Errors
    ///
    /// - [`PoolError::Empty`] if `addresses` is empty.
    /// - [`PoolError::BindFailed`] if any
    ///   [`HttpClient::new_with_local_addr`] call fails (e.g. the kernel
    ///   refuses the bind because the IP isn't assigned to any NIC).
    pub fn new(template: &HttpClientTemplate, addresses: Vec<IpAddr>) -> Result<Self, PoolError> {
        if addresses.is_empty() {
            return Err(PoolError::Empty);
        }
        let slots = addresses
            .into_iter()
            .map(|addr| {
                HttpClient::new_with_local_addr(
                    template.headers.clone(),
                    template.header_keys.clone(),
                    template.keyed_quotas.clone(),
                    template.default_quota,
                    template.timeout_secs,
                    template.proxy_url.clone(),
                    Some(addr),
                )
                .map_err(|e| PoolError::BindFailed {
                    address: addr,
                    cause: e.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            slots,
            rr_counter: AtomicUsize::new(0),
        })
    }

    /// Build a 1-slot pool with no `local_address` set on the underlying
    /// [`HttpClient`] — preserves the kernel-default source-IP behaviour for
    /// callers that haven't opted into IP pinning.
    ///
    /// Returned pool has `len() == 1` and dispatch is a no-op round-robin
    /// (always picks slot 0).
    ///
    /// # Errors
    ///
    /// - [`PoolError::BindFailed`] if the underlying
    ///   [`HttpClient::new_with_local_addr`] call fails.
    pub fn new_no_bind(template: HttpClientTemplate) -> Result<Self, PoolError> {
        let client = HttpClient::new_with_local_addr(
            template.headers,
            template.header_keys,
            template.keyed_quotas,
            template.default_quota,
            template.timeout_secs,
            template.proxy_url,
            None, // explicit: no source-IP bind
        )
        .map_err(|e| PoolError::BindFailed {
            // Use UNSPECIFIED as a sentinel — we never actually bound anywhere.
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            cause: e.to_string(),
        })?;
        Ok(Self {
            slots: vec![client],
            rr_counter: AtomicUsize::new(0),
        })
    }

    /// Number of slots in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the pool has no slots.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Pick a slot and return a reference to the underlying [`HttpClient`].
    ///
    /// The `hint` parameter must be [`PickHint::Stateless`] — `HttpPool` only
    /// serves stateless REST; the wallet-private slot-0 pin lives in `WsPool`.
    /// A non-`Stateless` hint trips a `debug_assert!` so a routing bug
    /// (e.g. accidentally feeding a wallet-private hint into HTTP code) fails
    /// loudly in debug rather than silently round-robining a request that
    /// should have been pinned.
    ///
    /// This method is preferred over `dispatch_async` at call sites that need
    /// to `.await` the client directly (avoids async-closure lifetime issues).
    pub fn pick_client(&self, hint: PickHint<'_>) -> (usize, &HttpClient) {
        debug_assert!(
            matches!(hint, PickHint::Stateless),
            "HttpPool::pick_client got non-Stateless hint"
        );
        // Singleton fast-path: skip the atomic for single-IP pools so
        // single-IP users pay zero overhead vs the pre-pool direct
        // `HttpClient` call.
        if self.slots.len() == 1 {
            return (0, &self.slots[0]);
        }
        let slot = self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.slots.len();
        (slot, &self.slots[slot])
    }

    /// Round-robin dispatch. Returns `(slot_index, closure_result)`.
    ///
    /// # Safety-critical non-feature: no automatic retry on a different slot
    ///
    /// If the picked slot's underlying [`HttpClient`] errors (e.g.
    /// `EADDRNOTAVAIL` because the IP isn't bound to a NIC), the error
    /// propagates verbatim. **DO NOT** add a "try next slot on error" loop
    /// here. That would mask operator misconfiguration as transient
    /// failures. The caller can retry, which advances the round-robin
    /// counter to a different slot naturally.
    pub async fn dispatch_async<F, Fut, R>(&self, hint: PickHint<'_>, f: F) -> (usize, R)
    where
        F: FnOnce(&HttpClient) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        debug_assert!(
            matches!(hint, PickHint::Stateless),
            "HttpPool got non-Stateless hint"
        );
        // Singleton fast-path: skip the atomic for single-IP pools.
        if self.slots.len() == 1 {
            return (0, f(&self.slots[0]).await);
        }
        let slot = self.rr_counter.fetch_add(1, Ordering::Relaxed) % self.slots.len();
        let result = f(&self.slots[slot]).await;
        (slot, result)
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn loopback_template() -> HttpClientTemplate {
        HttpClientTemplate::default()
    }

    #[test]
    fn empty_input_errors() {
        let err = HttpPool::new(&loopback_template(), vec![]).unwrap_err();
        assert!(matches!(err, PoolError::Empty));
    }

    #[test]
    fn loopback_constructs() {
        let pool = HttpPool::new(&loopback_template(), vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
            .unwrap();
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn new_no_bind_constructs_single_slot_pool() {
        let pool = HttpPool::new_no_bind(loopback_template()).unwrap();
        assert_eq!(pool.len(), 1);
    }

    #[tokio::test]
    async fn new_no_bind_dispatch_always_returns_slot_0() {
        let pool = HttpPool::new_no_bind(loopback_template()).unwrap();
        for _ in 0..5 {
            let (slot, ()) = pool.dispatch_async(PickHint::Stateless, |_| async {}).await;
            assert_eq!(slot, 0);
        }
    }

    #[tokio::test]
    async fn dispatch_returns_advancing_slot_index() {
        // Round-robin observed via the returned slot index, not via a
        // useless counter check. Six dispatches against 3 slots → slot
        // sequence 0, 1, 2, 0, 1, 2.
        let pool = HttpPool::new(
            &loopback_template(),
            vec![
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V4(Ipv4Addr::LOCALHOST),
            ],
        )
        .unwrap();
        let mut slots = Vec::new();
        for _ in 0..6 {
            let (slot, _) = pool
                .dispatch_async(PickHint::Stateless, |_c| async { 42 })
                .await;
            slots.push(slot);
        }
        assert_eq!(slots, vec![0, 1, 2, 0, 1, 2]);
    }

    #[tokio::test]
    async fn closure_result_propagates() {
        let pool = HttpPool::new(&loopback_template(), vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
            .unwrap();
        let (slot, result) = pool
            .dispatch_async(PickHint::Stateless, |_| async { "hello" })
            .await;
        assert_eq!(slot, 0);
        assert_eq!(result, "hello");
    }

    #[test]
    fn pick_client_singleton_returns_slot_0() {
        let pool = HttpPool::new(&loopback_template(), vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
            .unwrap();
        for _ in 0..5 {
            let (slot, _client) = pool.pick_client(PickHint::Stateless);
            assert_eq!(slot, 0);
        }
    }

    #[test]
    fn pick_client_round_robins_across_slots() {
        let pool = HttpPool::new(
            &loopback_template(),
            vec![
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V4(Ipv4Addr::LOCALHOST),
            ],
        )
        .unwrap();
        let slots: Vec<usize> = (0..6)
            .map(|_| pool.pick_client(PickHint::Stateless).0)
            .collect();
        assert_eq!(slots, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "HttpPool::pick_client got non-Stateless hint")]
    fn pick_client_rejects_non_stateless_hint_in_debug() {
        let pool = HttpPool::new(&loopback_template(), vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
            .unwrap();
        let _ = pool.pick_client(PickHint::WalletPrivateChannel {
            channel: "userFills",
        });
    }
}
