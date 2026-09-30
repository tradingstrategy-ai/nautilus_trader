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

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

use crate::{
    common::enums::HyperliquidInfoRequestType,
    http::{
        models::HyperliquidExecAction,
        query::{ExchangeAction, ExchangeActionParams, InfoRequest},
    },
};

#[derive(Debug)]
pub struct WeightedLimiter {
    capacity: f64,       // tokens per minute (e.g., 1200)
    refill_per_sec: f64, // capacity / 60
    state: tokio::sync::Mutex<State>,
}

#[derive(Debug)]
struct State {
    tokens: f64,
    last_refill: Instant,
}

impl WeightedLimiter {
    pub fn per_minute(capacity: u32) -> Self {
        let cap = capacity as f64;
        Self {
            capacity: cap,
            refill_per_sec: cap / 60.0,
            state: tokio::sync::Mutex::new(State {
                tokens: cap,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Acquire `weight` tokens, sleeping until available.
    pub async fn acquire(&self, weight: u32) {
        let need = weight as f64;

        loop {
            let mut st = self.state.lock().await;
            Self::refill_locked(&mut st, self.refill_per_sec, self.capacity);

            if st.tokens >= need {
                st.tokens -= need;
                return;
            }
            let deficit = need - st.tokens;
            let secs = deficit / self.refill_per_sec;
            drop(st);
            tokio::time::sleep(Duration::from_secs_f64(secs.max(0.01))).await;
        }
    }

    /// Post-response debit for per-items adders (can temporarily clamp to 0).
    pub async fn debit_extra(&self, extra: u32) {
        if extra == 0 {
            return;
        }
        let mut st = self.state.lock().await;
        Self::refill_locked(&mut st, self.refill_per_sec, self.capacity);
        st.tokens = (st.tokens - extra as f64).max(0.0);
    }

    pub async fn snapshot(&self) -> RateLimitSnapshot {
        let mut st = self.state.lock().await;
        Self::refill_locked(&mut st, self.refill_per_sec, self.capacity);
        RateLimitSnapshot {
            capacity: self.capacity as u32,
            tokens: st.tokens.max(0.0) as u32,
        }
    }

    fn refill_locked(st: &mut State, per_sec: f64, cap: f64) {
        let dt = Instant::now().duration_since(st.last_refill).as_secs_f64();
        if dt > 0.0 {
            st.tokens = (st.tokens + dt * per_sec).min(cap);
            st.last_refill = Instant::now();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitSnapshot {
    pub capacity: u32,
    pub tokens: u32,
}

/// One [`WeightedLimiter`] per `HttpPool` slot.
///
/// Hyperliquid enforces its REST rate limit **per source IP**, so a client
/// fanning requests across an `N`-address pool gets `N` independent 1200/min
/// buckets rather than one bucket shared by the whole client. A 1-slot
/// instance (the historical single-IP case) behaves identically to a single
/// [`WeightedLimiter`] — same capacity, same wait for the same call pattern.
///
/// Callers must `acquire`/`debit_extra` on the SAME slot index that
/// `HttpPool::pick_client` returned for that request, so the credit for a
/// completed request lands on the bucket it actually drew from.
#[derive(Debug, Clone)]
pub struct PerSlotLimiter {
    slots: Arc<Vec<WeightedLimiter>>,
}

impl PerSlotLimiter {
    /// Build one [`WeightedLimiter::per_minute(capacity)`] per slot.
    ///
    /// # Panics
    ///
    /// Panics if `n_slots == 0` — a limiter fan must cover at least the
    /// pool's single no-bind slot.
    pub fn new(n_slots: usize, capacity: u32) -> Self {
        assert!(n_slots > 0, "PerSlotLimiter::new called with n_slots=0");
        let slots = (0..n_slots)
            .map(|_| WeightedLimiter::per_minute(capacity))
            .collect();
        Self {
            slots: Arc::new(slots),
        }
    }

    /// Number of slots (buckets) in this limiter.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether this limiter has no slots (never true for an instance built
    /// via [`Self::new`], which asserts `n_slots > 0`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Acquire `weight` tokens from the bucket at `slot`, sleeping until available.
    pub async fn acquire(&self, slot: usize, weight: u32) {
        self.slots[slot].acquire(weight).await;
    }

    /// Post-response debit for per-item adders, applied to the bucket at `slot`.
    pub async fn debit_extra(&self, slot: usize, extra: u32) {
        self.slots[slot].debit_extra(extra).await;
    }

    /// Snapshot the bucket at `slot`.
    pub async fn snapshot(&self, slot: usize) -> RateLimitSnapshot {
        self.slots[slot].snapshot().await
    }

    /// Snapshot every slot's bucket, in slot order.
    pub async fn snapshot_all(&self) -> Vec<RateLimitSnapshot> {
        let mut out = Vec::with_capacity(self.slots.len());
        for limiter in self.slots.iter() {
            out.push(limiter.snapshot().await);
        }
        out
    }
}

pub fn backoff_full_jitter(attempt: u32, base: Duration, cap: Duration) -> Duration {
    let mut hasher = DefaultHasher::new();
    attempt.hash(&mut hasher);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos.hash(&mut hasher);
    let hash = hasher.finish();

    let max = (base.as_millis() as u64)
        .saturating_mul(1u64 << attempt.min(16))
        .min(cap.as_millis() as u64)
        .max(base.as_millis() as u64);

    // Floor at 1ms to prevent zero-duration backoff
    Duration::from_millis((hash % max).max(1))
}

/// Classify Info requests into weight classes based on request type.
pub fn info_base_weight(req: &InfoRequest) -> u32 {
    match req.request_type {
        HyperliquidInfoRequestType::L2Book
        | HyperliquidInfoRequestType::AllMids
        | HyperliquidInfoRequestType::RecentTrades
        | HyperliquidInfoRequestType::ClearinghouseState
        | HyperliquidInfoRequestType::OrderStatus
        | HyperliquidInfoRequestType::SpotClearinghouseState
        | HyperliquidInfoRequestType::ExchangeStatus
        | HyperliquidInfoRequestType::UserFees => 2,
        HyperliquidInfoRequestType::UserRole => 60,
        _ => 20,
    }
}

/// Extra weight for heavy Info endpoints: +1 per 20 (most), +1 per 60 for candleSnapshot.
/// We count the largest array in the response (robust to schema variants).
pub fn info_extra_weight(req: &InfoRequest, json: &Value) -> u32 {
    let items = match json {
        Value::Array(a) => a.len(),
        Value::Object(m) => m
            .values()
            .filter_map(|v| v.as_array().map(|a| a.len()))
            .max()
            .unwrap_or(0),
        _ => 0,
    };

    let unit = match req.request_type {
        HyperliquidInfoRequestType::CandleSnapshot => 60usize,
        HyperliquidInfoRequestType::HistoricalOrders
        | HyperliquidInfoRequestType::UserFills
        | HyperliquidInfoRequestType::UserFillsByTime
        | HyperliquidInfoRequestType::FundingHistory
        | HyperliquidInfoRequestType::UserFunding
        | HyperliquidInfoRequestType::NonUserFundingUpdates
        | HyperliquidInfoRequestType::TwapHistory
        | HyperliquidInfoRequestType::UserTwapSliceFills
        | HyperliquidInfoRequestType::UserTwapSliceFillsByTime
        | HyperliquidInfoRequestType::DelegatorHistory
        | HyperliquidInfoRequestType::DelegatorRewards
        | HyperliquidInfoRequestType::ValidatorStats => 20usize,
        _ => return 0,
    };
    (items / unit) as u32
}

/// Exchange: 1 + floor(batch_len / 40)
pub fn exchange_weight(action: &ExchangeAction) -> u32 {
    // Extract batch size from typed params
    let batch_size = match &action.params {
        ExchangeActionParams::Order(params) => params.orders.len(),
        ExchangeActionParams::Cancel(params) => params.cancels.len(),
        ExchangeActionParams::Modify(_) => {
            // Modify is for a single order
            1
        }
        ExchangeActionParams::UpdateLeverage(_) | ExchangeActionParams::UpdateIsolatedMargin(_) => {
            0
        }
    };
    1 + (batch_size as u32 / 40)
}

/// Exchange weight for the canonical typed execution action model.
pub fn exec_action_weight(action: &HyperliquidExecAction) -> u32 {
    let batch_size = match action {
        HyperliquidExecAction::Order { orders, .. } => orders.len(),
        HyperliquidExecAction::Cancel { cancels, .. } => cancels.len(),
        HyperliquidExecAction::CancelByCloid { cancels, .. } => cancels.len(),
        HyperliquidExecAction::Modify { .. } => 1,
        HyperliquidExecAction::BatchModify { modifies } => modifies.len(),
        HyperliquidExecAction::UpdateLeverage { .. }
        | HyperliquidExecAction::UpdateIsolatedMargin { .. }
        | HyperliquidExecAction::ScheduleCancel { .. }
        | HyperliquidExecAction::UsdClassTransfer { .. }
        | HyperliquidExecAction::UserOutcome { .. }
        | HyperliquidExecAction::TwapPlace { .. }
        | HyperliquidExecAction::TwapCancel { .. }
        | HyperliquidExecAction::Noop => 0,
    };
    1 + (batch_size as u32 / 40)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use rust_decimal::Decimal;

    use super::{
        super::models::{
            Cloid, HyperliquidExecAction, HyperliquidExecCancelByCloidRequest,
            HyperliquidExecCancelOrderRequest, HyperliquidExecGrouping, HyperliquidExecLimitParams,
            HyperliquidExecModifyOrderRequest, HyperliquidExecOrderKind,
            HyperliquidExecPlaceOrderRequest, HyperliquidExecTif,
        },
        *,
    };
    use crate::http::query::{
        CancelParams, ExchangeAction, ExchangeActionParams, ExchangeActionType, OrderParams,
        UpdateLeverageParams,
    };

    fn exec_order() -> HyperliquidExecPlaceOrderRequest {
        HyperliquidExecPlaceOrderRequest {
            asset: 0,
            is_buy: true,
            price: Decimal::new(50000, 0),
            size: Decimal::new(1, 0),
            reduce_only: false,
            kind: HyperliquidExecOrderKind::Limit {
                limit: HyperliquidExecLimitParams {
                    tif: HyperliquidExecTif::Gtc,
                },
            },
            cloid: Some(Cloid::from_hex("0x00000000000000000000000000000000").unwrap()),
        }
    }

    fn exec_modify() -> HyperliquidExecModifyOrderRequest {
        HyperliquidExecModifyOrderRequest {
            oid: 12345.into(),
            order: exec_order(),
        }
    }

    fn exec_cancel_by_cloid() -> HyperliquidExecCancelByCloidRequest {
        HyperliquidExecCancelByCloidRequest {
            asset: 0,
            cloid: Cloid::from_hex("0x00000000000000000000000000000000").unwrap(),
        }
    }

    #[rstest]
    #[case(1, 1)]
    #[case(39, 1)]
    #[case(40, 2)]
    #[case(79, 2)]
    #[case(80, 3)]
    fn test_exchange_weight_order_steps_every_40(
        #[case] array_len: usize,
        #[case] expected_weight: u32,
    ) {
        let orders: Vec<HyperliquidExecPlaceOrderRequest> =
            (0..array_len).map(|_| exec_order()).collect();

        let action = ExchangeAction {
            action_type: ExchangeActionType::Order,
            params: ExchangeActionParams::Order(OrderParams {
                orders,
                grouping: HyperliquidExecGrouping::Na,
                builder: None,
            }),
        };
        assert_eq!(exchange_weight(&action), expected_weight);
    }

    #[rstest]
    #[case(1, 1)]
    #[case(39, 1)]
    #[case(40, 2)]
    #[case(79, 2)]
    #[case(80, 3)]
    fn test_exec_action_weight_order_steps_every_40(
        #[case] array_len: usize,
        #[case] expected_weight: u32,
    ) {
        let action = HyperliquidExecAction::Order {
            orders: (0..array_len).map(|_| exec_order()).collect(),
            grouping: HyperliquidExecGrouping::Na,
            builder: None,
        };

        assert_eq!(exec_action_weight(&action), expected_weight);
    }

    #[rstest]
    #[case(1, 1)]
    #[case(39, 1)]
    #[case(40, 2)]
    #[case(79, 2)]
    #[case(80, 3)]
    fn test_exec_action_weight_cancel_by_oid_steps_every_40(
        #[case] array_len: usize,
        #[case] expected_weight: u32,
    ) {
        let action = HyperliquidExecAction::Cancel {
            cancels: (0..array_len)
                .map(|i| HyperliquidExecCancelOrderRequest {
                    asset: 0,
                    oid: i as u64,
                })
                .collect(),
            fast: None,
        };

        assert_eq!(exec_action_weight(&action), expected_weight);
    }

    #[rstest]
    #[case(1, 1)]
    #[case(39, 1)]
    #[case(40, 2)]
    #[case(79, 2)]
    #[case(80, 3)]
    fn test_exec_action_weight_cancel_by_cloid_steps_every_40(
        #[case] array_len: usize,
        #[case] expected_weight: u32,
    ) {
        let action = HyperliquidExecAction::CancelByCloid {
            cancels: (0..array_len).map(|_| exec_cancel_by_cloid()).collect(),
            fast: None,
        };

        assert_eq!(exec_action_weight(&action), expected_weight);
    }

    #[rstest]
    #[case(1, 1)]
    #[case(39, 1)]
    #[case(40, 2)]
    #[case(79, 2)]
    #[case(80, 3)]
    fn test_exec_action_weight_batch_modify_steps_every_40(
        #[case] array_len: usize,
        #[case] expected_weight: u32,
    ) {
        let action = HyperliquidExecAction::BatchModify {
            modifies: (0..array_len).map(|_| exec_modify()).collect(),
        };

        assert_eq!(exec_action_weight(&action), expected_weight);
    }

    #[rstest]
    fn test_exec_action_weight_modify() {
        let action = HyperliquidExecAction::Modify {
            modify: exec_modify(),
        };

        assert_eq!(exec_action_weight(&action), 1);
    }

    #[rstest]
    fn test_exec_action_weight_non_batch_action() {
        let action = HyperliquidExecAction::UpdateLeverage {
            asset: 1,
            is_cross: true,
            leverage: 10,
        };

        assert_eq!(exec_action_weight(&action), 1);
    }

    #[rstest]
    fn test_exchange_weight_cancel() {
        let cancels: Vec<HyperliquidExecCancelByCloidRequest> =
            (0..40).map(|_| exec_cancel_by_cloid()).collect();

        let action = ExchangeAction {
            action_type: ExchangeActionType::Cancel,
            params: ExchangeActionParams::Cancel(CancelParams {
                cancels,
                fast: None,
            }),
        };
        assert_eq!(exchange_weight(&action), 2);
    }

    #[rstest]
    fn test_exchange_weight_non_batch_action() {
        let update_leverage = ExchangeAction {
            action_type: ExchangeActionType::UpdateLeverage,
            params: ExchangeActionParams::UpdateLeverage(UpdateLeverageParams {
                asset: 1,
                is_cross: true,
                leverage: 10,
            }),
        };
        assert_eq!(exchange_weight(&update_leverage), 1);
    }

    #[tokio::test]
    async fn test_limiter_roughly_caps_to_capacity() {
        let limiter = WeightedLimiter::per_minute(1200);

        // Consume ~1200 in quick succession
        for _ in 0..60 {
            limiter.acquire(20).await; // 60 * 20 = 1200
        }

        // The next acquire should take time for tokens to refill
        let t0 = std::time::Instant::now();
        limiter.acquire(20).await;
        let elapsed = t0.elapsed();

        // Should take at least some time to refill (allow some jitter/timing variance)
        assert!(
            elapsed.as_millis() >= 500,
            "Expected significant delay, was {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_limiter_debit_extra_works() {
        let limiter = WeightedLimiter::per_minute(100);

        // Start with full bucket
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.capacity, 100);
        assert_eq!(snapshot.tokens, 100);

        // Acquire some tokens
        limiter.acquire(30).await;
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.tokens, 70);

        // Debit extra
        limiter.debit_extra(20).await;
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.tokens, 50);

        // Debit more than available (should clamp to 0)
        limiter.debit_extra(100).await;
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.tokens, 0);
    }

    #[rstest]
    #[case(0, 100)]
    #[case(1, 200)]
    #[case(2, 400)]
    fn test_backoff_full_jitter_increases(#[case] attempt: u32, #[case] max_expected_ms: u64) {
        let base = Duration::from_millis(100);
        let cap = Duration::from_secs(5);

        let delay = backoff_full_jitter(attempt, base, cap);

        assert!(delay.as_millis() >= 1);
        assert!(delay.as_millis() <= max_expected_ms as u128);
    }

    #[rstest]
    fn test_backoff_full_jitter_respects_cap() {
        let base = Duration::from_millis(100);
        let cap = Duration::from_secs(5);

        let delay_high = backoff_full_jitter(10, base, cap);
        assert!(delay_high.as_millis() <= cap.as_millis());
    }

    // Per-slot limiter parallelism tests.
    //
    // `wait_per_call = 60 / free_calls` for a `WeightedLimiter::per_minute(capacity)`
    // drained one token (weight 1) at a time past its `free_calls = capacity` budget.
    // With `free_calls = 200`, `wait_per_call = 0.3s`; 4 calls beyond the free budget
    // force a deterministic ~1.2s wait per slot. These constants keep the test fast
    // (~1-2s) while still proving the parallelism/regression properties.
    const TEST_CAPACITY: u32 = 200;
    const TEST_FREE_CALLS: u32 = TEST_CAPACITY; // weight 1 per acquire
    const TEST_EXTRA_CALLS: u32 = 4;
    const TEST_CALLS_PER_SLOT: u32 = TEST_FREE_CALLS + TEST_EXTRA_CALLS;

    #[tokio::test]
    async fn test_per_slot_limiter_4_slots_run_in_parallel() {
        // 4 slots, each drained by TEST_CALLS_PER_SLOT weight-1 acquires. If each
        // slot has its own bucket, all 4 slots' waits overlap and total wall time
        // is ~one slot's wait (~1.2s), not ~4x that (~4.8s) as it would be with a
        // single shared bucket serving all 4 slots' worth of load.
        let limiter = PerSlotLimiter::new(4, TEST_CAPACITY);

        let t0 = std::time::Instant::now();
        let mut handles = Vec::new();
        for slot in 0..4 {
            let limiter = limiter.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..TEST_CALLS_PER_SLOT {
                    limiter.acquire(slot, 1).await;
                }
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let elapsed = t0.elapsed();

        assert!(
            elapsed.as_millis() < 2_500,
            "Expected parallel per-slot drain to finish well under 2.5s, took {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_per_slot_limiter_1_slot_matches_single_limiter_timing() {
        // Regression guard: a 1-slot pool must behave byte-for-byte like the old
        // single shared `WeightedLimiter` — same capacity, same wait for the same
        // call pattern used by exactly one slot above.
        let limiter = PerSlotLimiter::new(1, TEST_CAPACITY);

        let t0 = std::time::Instant::now();
        for _ in 0..TEST_CALLS_PER_SLOT {
            limiter.acquire(0, 1).await;
        }
        let elapsed = t0.elapsed();

        assert!(
            elapsed.as_millis() >= 900,
            "Expected the single-slot limiter to pay the full refill wait (~1.2s), was {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_per_slot_limiter_debit_extra_lands_on_same_slot() {
        // debit_extra(slot, ..) must only affect the slot it targets, never a
        // different slot or an aggregate shared across slots.
        let limiter = PerSlotLimiter::new(2, 100);

        limiter.acquire(0, 30).await;
        limiter.debit_extra(0, 20).await;

        let snap0 = limiter.snapshot(0).await;
        let snap1 = limiter.snapshot(1).await;

        assert_eq!(snap0.tokens, 50); // 100 - 30 - 20
        assert_eq!(snap1.tokens, 100); // untouched
    }

    #[tokio::test]
    async fn test_per_slot_limiter_snapshot_all_returns_one_per_slot() {
        let limiter = PerSlotLimiter::new(3, 100);
        limiter.acquire(1, 40).await;

        let snapshots = limiter.snapshot_all().await;

        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[0].tokens, 100);
        assert_eq!(snapshots[1].tokens, 60);
        assert_eq!(snapshots[2].tokens, 100);
    }
}
