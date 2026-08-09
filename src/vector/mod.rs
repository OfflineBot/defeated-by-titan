//! vector — THE CORE: hooks, rope, momentum, gas, boost, wallrun
//!
//! **The game lives and dies by this feel — not by the Titan AI.**
//! A player who flies elegantly through the city without killing a single Titan has to be
//! having fun. If that does not work, nothing works (bible 2, pillar P1).
//!
//! Hence the hard gate: **no meta system before the movement convinces.** And hence every
//! number here lives in `assets/data/game.ron` and none in the code.
//!
//! Two **independently** steerable hooks (`F-001`), pendulum physics with two hooks set
//! (`F-004`), reel-in (`F-005`), swerve (`F-006`). Rope forces need guards: normalizing a
//! zero-length vector produces NaN, and NaN in the `Transform` looks like
//! "the player has vanished" (§9d).
//!
//! **Status:** the seam is in place, the math is missing. Five modules, five files, five
//! jobs — **the registration here is already complete**, so that no agent has to touch this
//! file later and no two jobs end up fighting over it
//! (`docs/interface.md`, file ownership).
//!
//! | File | F-ID | writes |
//! |---|---|---|
//! | `aim.rs` | `F-002`, `F-003` | `AimPoint` |
//! | `gas.rs` | `F-018` | `Gas`, `GasGrant` |
//! | `hook.rs` | `F-001` | `Hook`, `PrevButtons` |
//! | `boost.rs` | `F-007` | `BoostAccel` |
//! | `reel.rs` | `F-005` | `ReelSpeed` |
//!
//! What is **not** here: `Velocity`, `Transform`, `RopeLength`. Those are written by the
//! integrator in `player::integrator` — one writer, not two.

pub mod reel;
pub mod gas;
pub mod hook;
pub mod boost;
pub mod aim;

use bevy::prelude::*;

use crate::shared::SimulationSystems;

pub struct VectorPlugin;

impl Plugin for VectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, aim::aim.in_set(SimulationSystems::World))
            // `.chain()`: the gas budget is booked BEFORE the hook switches — otherwise it
            // hangs on system order whether a freshly set hook already costs gas in the same
            // tick.
            .add_systems(
                FixedUpdate,
                (gas::gas_budget, hook::update_hooks).chain().in_set(SimulationSystems::Intent),
            )
            // **Deliberately without `.chain()`**: both write their own component by
            // assignment, the `&mut` sets are disjoint, so the order is provably
            // irrelevant — and Bevy really does run them in parallel.
            .add_systems(FixedUpdate, (boost::gas_boost, reel::reel_in).in_set(SimulationSystems::Drive))
            .add_systems(
                FixedUpdate,
                hook::store_prev_buttons.in_set(SimulationSystems::PostStep),
            );
    }
}
