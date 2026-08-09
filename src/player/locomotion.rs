//! Ground run and air control as a **contribution**, not as finished movement.
//!
//! Writes exactly one component ([`RunAccel`], in m/s²) and nothing else. Three separate
//! drive components instead of one with three fields: that way "exactly one writer per
//! component" holds literally and is checkable with `grep`, and three systems with disjoint
//! `&mut` really do run in parallel in Bevy instead of serializing on each other.
//!
//! **Assignment, never `+=`.** A contributor that wants nothing writes `Vec3::ZERO`. That
//! leaves no empty system, no order dependency inside `SimulationSystems::Drive` and no state
//! that lives one tick too long.
//!
//! Reads [`MovementState`] from the **end of the previous tick** — one tick of lag is
//! deterministic and cheaper than an order dependency on the integrator.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{RunAccel, MovementState, Intent};

/// Writes [`RunAccel`] from `Intent.movement()` and `player.run_speed_m_s`.
// filled in by job V — stage 2
pub fn ground_run(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &MovementState, &mut RunAccel)>,
) {
}
