//! State on the player — **components, never a `Resource`.**
//!
//! Gas, blades and movement state hang on *one* player. As a `Resource` they would be
//! global, and with that the game would be a single-player game you only notice as one when
//! multiplayer comes around (`prompts/init.md` §6 rule 3).
//!
//! They live in `shared/` although `vector` and `blades` write them, because `hud` and
//! `sound` have to **read** them — and opening an edge between domains for that would be the
//! beginning of the end of the domain rule. **Who writes stands in the authority table in
//! `docs/architecture.md`, not in the type.**

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Gas is finite, and **spending gas is loud** — the Bellower reacts to the noise (bible 4).
/// That couples the resource to the risk instead of making it a plain timer.
///
/// The numbers (tank size, drain per second, boost cost) stand in `assets/data/gear.ron`,
/// **not here** (§4).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gas {
    pub current: f32,
    pub max: f32,
    /// `--sandbox` sets this: infinite gas, for looking around (§12a).
    pub unlimited: bool,
}

impl Gas {
    pub fn full(max: f32) -> Self {
        Gas { current: max, max, unlimited: false }
    }

    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 { (self.current / self.max).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Tries to spend `amount`. `false` means: there was not enough, **and nothing was
    /// deducted**.
    ///
    /// No partial spend: "gas exactly zero at the moment of the boost" is one of the edge
    /// cases that belong in a test (§8) — half a boost would be harder to explain than none.
    pub fn try_spend(&mut self, amount: f32) -> bool {
        if self.unlimited {
            return true;
        }
        if !amount.is_finite() || amount < 0.0 {
            return false;
        }
        if self.current + 1e-6 < amount {
            return false;
        }
        self.current = (self.current - amount).max(0.0);
        true
    }

    pub fn refill(&mut self, amount: f32) {
        if amount.is_finite() && amount > 0.0 {
            self.current = (self.current + amount).min(self.max);
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.unlimited && self.current <= 0.0
    }
}

/// Blades go blunt and break. **Economy instead of cooldowns** (`prompts/init.md` §1):
/// you reload at supply points, from the horse, or off fallen comrades.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blades {
    /// How many pairs are left in the harness.
    pub pairs_left: u8,
    /// Condition of the pair in use, 1.0 = fresh, 0.0 = broken.
    pub sharpness: f32,
}

impl Blades {
    pub fn fresh(pairs: u8) -> Self {
        Blades { pairs_left: pairs, sharpness: 1.0 }
    }

    pub fn is_broken(&self) -> bool {
        self.sharpness <= 0.0
    }

    /// Put in a fresh pair. `false` means: none left.
    pub fn swap_pair(&mut self) -> bool {
        if self.pairs_left == 0 {
            return false;
        }
        self.pairs_left -= 1;
        self.sharpness = 1.0;
        true
    }
}

/// Velocity in m/s.
///
/// A component of its own and not a value derived from the `Transform`: **damage comes out
/// of speed** (`prompts/init.md` §1), and a quantity that damage comes out of must not
/// depend on how much time passed between two frames. It is also exactly the number
/// `assert speed > 25` measures (§12b).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Velocity(pub Vec3);

impl Velocity {
    pub fn speed_m_s(&self) -> f32 {
        self.0.length()
    }
}

/// What the player's body currently hangs on.
///
/// **It decides who is allowed to write the `Transform`.** `player` writes it on the ground
/// and in free fall, `vector` on the rope — never both at once. Two writers on the same
/// field are not a design, they are a coin flip at 60 Hz (§5 rule 4).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementState {
    #[default]
    Grounded,
    Airborne,
    /// At least one hook holds — from here on the `Transform` belongs to `vector`.
    Tethered,
    OnWall,
    /// Out of the fight instead of dead: a state with a timer, not a removed entity.
    /// Revived by team mates (bible 3.6, `squad/`).
    Downed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_only_spends_what_is_there() {
        let mut g = Gas::full(100.0);
        assert!(g.try_spend(30.0));
        assert!((g.current - 70.0).abs() < 1e-6);
        assert!(!g.try_spend(80.0), "80 out of 70 must not succeed");
        assert!((g.current - 70.0).abs() < 1e-6, "a failed boost costs nothing");
    }

    #[test]
    fn gas_exactly_zero_at_the_moment_of_the_boost() {
        // Exactly the edge case from prompts/init.md §8 — the normal case works almost by
        // itself, the bugs sit at the edges.
        let mut g = Gas::full(10.0);
        assert!(g.try_spend(10.0));
        assert!(g.is_empty());
        assert!(!g.try_spend(0.001));
        assert_eq!(g.fraction(), 0.0);
    }

    #[test]
    fn gas_rejects_nonsensical_amounts() {
        let mut g = Gas::full(10.0);
        assert!(!g.try_spend(-5.0), "a negative spend would be a refill");
        assert!(!g.try_spend(f32::NAN));
        assert!((g.current - 10.0).abs() < 1e-6);
    }

    #[test]
    fn sandbox_gas_never_runs_out() {
        let mut g = Gas { unlimited: true, ..Gas::full(1.0) };
        assert!(g.try_spend(1000.0));
        assert!(!g.is_empty());
    }

    #[test]
    fn gas_does_not_overflow_on_refill() {
        let mut g = Gas::full(50.0);
        g.try_spend(10.0);
        g.refill(999.0);
        assert!((g.current - 50.0).abs() < 1e-6);
    }

    #[test]
    fn blades_swap_until_the_belt_is_empty() {
        let mut k = Blades::fresh(2);
        k.sharpness = 0.0;
        assert!(k.is_broken());
        assert!(k.swap_pair());
        assert!(!k.is_broken());
        assert!(k.swap_pair());
        assert!(!k.swap_pair(), "an empty harness yields no more pairs");
        assert_eq!(k.pairs_left, 0);
    }
}
