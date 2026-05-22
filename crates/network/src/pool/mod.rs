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

//! Multi-IP source-binding pools for HTTP and WebSocket clients.
//!
//! Provides the building blocks for a fan of clients each bound to a
//! different local IP. This module currently exports the deterministic
//! instrument-to-slot hash; the actual pool types are layered on top in
//! follow-up commits.

use std::hash::{Hash, Hasher};

use siphasher::sip::SipHasher13;

/// Returns the slot index for an instrument, deterministic across process restarts.
///
/// Uses fixed-key SipHash-1-3 (key `(0, 0)`). The reason for the fixed key
/// (not `DefaultHasher`) is **determinism across process restarts**: the same
/// instrument MUST land on the same slot every time the bot starts, so
/// operators can reason about "BTC sits on IP X" and the slot-0 wallet-private
/// pin remains stable. `DefaultHasher` uses a per-process random seed and
/// would re-shuffle the instrument-to-IP mapping on every restart.
///
/// We are not protecting against hash-collision attacks here (no adversarial
/// input); the `(0, 0)` key is the conventional "no secret" sentinel for
/// deterministic SipHash.
///
/// # Panics
///
/// Panics if `n_slots == 0`. Callers MUST validate the pool is non-empty
/// before calling.
#[must_use]
pub fn slot_for_instrument(instrument: &str, n_slots: usize) -> usize {
    assert!(n_slots > 0, "slot_for_instrument called with n_slots=0");
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    instrument.as_bytes().hash(&mut hasher);
    (hasher.finish() as usize) % n_slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_for_instrument_deterministic() {
        let a = slot_for_instrument("BTC", 3);
        let b = slot_for_instrument("BTC", 3);
        assert_eq!(a, b);
    }

    #[test]
    fn slot_for_instrument_known_values() {
        // Hardcoded so a future hash-function change fails this test loudly.
        // If you genuinely need to change the hash, update these AND bump
        // a major version since all operators' instrument-to-IP mapping shifts.
        assert_eq!(slot_for_instrument("BTC", 3), 2);
        assert_eq!(slot_for_instrument("ETH", 3), 1);
        assert_eq!(slot_for_instrument("SOL", 3), 0);
        assert_eq!(slot_for_instrument("BTC", 1), 0);
    }

    #[test]
    #[should_panic(expected = "n_slots=0")]
    fn slot_for_instrument_panics_on_zero_slots() {
        let _ = slot_for_instrument("BTC", 0);
    }
}
