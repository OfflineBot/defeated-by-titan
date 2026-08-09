//! Randomness that comes out the same on two machines.
//!
//! **Determinism where it is cheap** (`prompts/init.md` §6 rule 5). A titan that turns a
//! different way on two machines is a bug you only see over a wire — that is, on the most
//! expensive day.
//!
//! That is why [`Rng`] is **stateless**: it does not draw from a running stream, it computes
//! a value out of `(seed, tick, stream)`. That is the decisive difference: a running
//! generator delivers different numbers as soon as two systems run in a different order —
//! and Bevy's systems run in parallel and in no fixed order. A generator with state would be
//! a coin flip at 60 Hz here.
//!
//! `stream` tells the users apart: `titan.0` for one titan's decision, a fixed hash for a
//! loot roll site. Two users with the same `stream` in the same tick get the same number —
//! that is not a bug, that is the rule: whoever wants a number of their own takes a stream
//! of their own.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The seed is **part of the state** — it gets saved and one day sent.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    pub seed: u64,
}

impl Default for Rng {
    fn default() -> Self {
        // A fixed default instead of the clock: a run has to be reproducible without anyone
        // doing anything (§17: the command stands next to it, seed included).
        Rng { seed: 0x0DEF_EA7E_D0B7_1743 }
    }
}

/// SplitMix64 — short, fast, good enough, and **the same everywhere**. No `rand` crate: its
/// output is allowed to change between versions, and exactly that would be a desync nobody
/// recognizes as one.
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { seed }
    }

    /// A raw value out of `(seed, tick, stream)`.
    pub fn raw(&self, tick: u64, stream: u64) -> u64 {
        splitmix64(
            self.seed
                ^ splitmix64(tick.wrapping_mul(0xD1B5_4A32_D192_ED03))
                ^ splitmix64(stream.wrapping_mul(0xA24B_AED4_963E_E407)),
        )
    }

    /// Uniform in `[0, 1)`.
    pub fn fraction(&self, tick: u64, stream: u64) -> f32 {
        // 24 bits are enough for an f32 and keep rounding from landing on exactly 1.0.
        (self.raw(tick, stream) >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in `[min, max)`.
    pub fn range(&self, tick: u64, stream: u64, min: f32, max: f32) -> f32 {
        min + self.fraction(tick, stream) * (max - min)
    }

    /// An index in `0..n`. `n == 0` yields `0` — the caller checks for emptiness itself.
    pub fn index(&self, tick: u64, stream: u64, n: usize) -> usize {
        if n == 0 { 0 } else { (self.raw(tick, stream) % n as u64) as usize }
    }

    /// Whether something happens with probability `p`.
    pub fn chance(&self, tick: u64, stream: u64, p: f32) -> bool {
        self.fraction(tick, stream) < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_number() {
        // That is the whole purpose: two machines, the same tick, the same number.
        let w = Rng::new(42);
        assert_eq!(w.raw(7, 3), w.raw(7, 3));
        assert_eq!(w.fraction(7, 3), w.fraction(7, 3));
    }

    #[test]
    fn system_order_does_not_matter() {
        // Bevy's systems run in parallel. A generator with state would have different values
        // here depending on the run order — this one does not.
        let w = Rng::new(42);
        let forward: Vec<u64> = (0..5).map(|s| w.raw(9, s)).collect();
        let backward: Vec<u64> = (0..5).rev().map(|s| w.raw(9, s)).collect();
        assert_eq!(forward, backward.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn different_seeds_different_numbers() {
        assert_ne!(Rng::new(1).raw(0, 0), Rng::new(2).raw(0, 0));
        let w = Rng::new(1);
        assert_ne!(w.raw(0, 0), w.raw(1, 0));
        assert_ne!(w.raw(0, 0), w.raw(0, 1));
    }

    #[test]
    fn fraction_stays_in_the_half_open_unit_interval() {
        // Exactly 1.0 would be the error that runs an index off the end of an array.
        let w = Rng::new(0xABCD);
        for tick in 0..2000u64 {
            let a = w.fraction(tick, tick % 7);
            assert!((0.0..1.0).contains(&a), "tick {tick} gave {a}");
        }
    }

    #[test]
    fn index_stays_in_range_even_for_zero() {
        let w = Rng::new(5);
        assert_eq!(w.index(1, 1, 0), 0);
        for n in 1..20usize {
            for tick in 0..50u64 {
                assert!(w.index(tick, 0, n) < n);
            }
        }
    }

    #[test]
    fn the_distribution_is_not_obviously_skewed() {
        // Not a quality test, only a guard: a broken mixer stands out here immediately.
        let w = Rng::new(7);
        let mut bins = [0u32; 10];
        for tick in 0..10_000u64 {
            bins[(w.fraction(tick, 0) * 10.0) as usize] += 1;
        }
        for (i, n) in bins.iter().enumerate() {
            assert!((800..1200).contains(n), "bin {i} held {n} of ~1000 each");
        }
    }
}
