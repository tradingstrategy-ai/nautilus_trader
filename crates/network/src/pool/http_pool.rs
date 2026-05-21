//! Round-robin pool of `HttpClient`s, each bound to a different source IP.

use std::{
    net::IpAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{HttpClientTemplate, PickHint, PoolError};
use crate::http::HttpClient;

#[derive(Debug)]
pub struct HttpPool {
    slots: Vec<HttpClient>,
    rr_counter: AtomicUsize,
}

impl HttpPool {
    /// Build one `HttpClient` per address using the same template
    /// (HL adapter passes its default headers + REST quota via template).
    ///
    /// # Errors
    /// - `PoolError::Empty` if `addresses` is empty.
    /// - `PoolError::BindFailed` if any `HttpClient::new_with_local_addr` call fails.
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
                .map_err(|e| PoolError::BindFailed { address: addr, cause: e.to_string() })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { slots, rr_counter: AtomicUsize::new(0) })
    }

    /// Build a 1-slot pool with no `local_address` set on the underlying
    /// `HttpClient` — preserves PR #4's "no bind, kernel default" path.
    /// Used by `HyperliquidRawHttpClient::new()` callers (no IP specified).
    ///
    /// Returned pool has `len() == 1` and `dispatch_async` is a no-op
    /// round-robin (always picks slot 0).
    ///
    /// # Errors
    /// - `PoolError::BindFailed` if the underlying `HttpClient::new_with_local_addr` call fails.
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
            // Use UNSPECIFIED as a sentinel — we never actually bound anywhere
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            cause: e.to_string(),
        })?;
        Ok(Self {
            slots: vec![client],
            rr_counter: AtomicUsize::new(0),
        })
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Pick a slot and return a reference to the underlying [`HttpClient`].
    ///
    /// The `hint` parameter is accepted for API symmetry with `dispatch_async`
    /// but `HttpPool` always round-robins stateless REST requests regardless
    /// of hint.
    ///
    /// This method is preferred over `dispatch_async` at call sites that need
    /// to `.await` the client directly (avoids async-closure lifetime issues).
    pub fn pick_client(&self, _hint: PickHint<'_>) -> (usize, &HttpClient) {
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
    /// If the picked slot's underlying `HttpClient` errors (e.g.
    /// `EADDRNOTAVAIL` because the IP isn't bound to a NIC), the error
    /// propagates verbatim. **DO NOT** add a "try next slot on error" loop
    /// here. That would mask operator misconfiguration as transient
    /// failures. The caller can retry, which advances the round-robin
    /// counter to a different slot naturally.
    pub async fn dispatch_async<F, Fut, R>(
        &self,
        hint: PickHint<'_>,
        f: F,
    ) -> (usize, R)
    where
        F: FnOnce(&HttpClient) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        debug_assert!(
            matches!(hint, PickHint::Stateless),
            "HttpPool got non-Stateless hint"
        );
        // Fast path: singleton pool (e.g. `HyperliquidRawHttpClient::new()` via
        // `new_no_bind`, or any operator who set `HL_LOCAL_ADDR=<single-ip>`)
        // skips the atomic. Preserves PR #4's overhead profile exactly for
        // single-IP users.
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
    use super::*;
    use std::net::Ipv4Addr;

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
        let pool = HttpPool::new(
            &loopback_template(),
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        )
        .unwrap();
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn new_no_bind_constructs_single_slot_pool() {
        // The "no IP specified, kernel default" path. Returns a 1-slot
        // pool. dispatch_async on it is a no-op round-robin.
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
            let (slot, _) = pool.dispatch_async(PickHint::Stateless, |_c| async { 42 }).await;
            slots.push(slot);
        }
        assert_eq!(slots, vec![0, 1, 2, 0, 1, 2]);
    }

    #[tokio::test]
    async fn closure_result_propagates() {
        let pool = HttpPool::new(&loopback_template(), vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]).unwrap();
        let (slot, result) = pool.dispatch_async(PickHint::Stateless, |_| async { "hello" }).await;
        assert_eq!(slot, 0);
        assert_eq!(result, "hello");
    }
}
