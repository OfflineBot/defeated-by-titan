//! **The integrator** — the only system that writes a player's `Transform`, `Velocity`,
//! `MovementState` and `RopeLength`.
//!
//! The old split "`player` on the ground, `vector` on the rope, kept apart by
//! `MovementState`" does not hold: a gas boost acts in the air **and** on the rope at the
//! same time, so there is no state that separates the two writers. Instead of two writers
//! with a switch there is now **one** writer and several collection points.
//!
//! ## The step sequence, numbered — it is the interface
//!
//! ```text
//! (a) dt  = clamped_dt_s(Time<Fixed>::delta_secs())
//! (b) a   = RunAccel + BoostAccel + (0, gravity_m_s2, 0)
//! (c) velocity += a * dt
//! (d) velocity = clamp_length_max(vector.max_speed_m_s)        F-012, BEFORE the move
//! (e) advance the rope lengths:
//!       freshly anchored -> length = |pos - anchor|
//!       ReelSpeed        -> length -= desired * dt, clamped to vector.min_rope_m
//!                           velocity = rope_reel_in(..)  (angular momentum, F-005)
//!       not anchored     -> length = 0
//! (f) N   = ceil(|velocity| * dt / player.max_substep_m), at least 1
//! (g) per substep, in THIS order:
//!       (g1) pos_free = pos + velocity * dt/N
//!       (g2) rope_step(pos, pos_free, velocity, constraints, vector.rope_iterations)
//!       (g3) collision against SpatialIndex::aabb_overlaps, margin = world.collision_margin_m
//!       (g4) if the collision pushed us out of the rope sphere:
//!              length = |pos - anchor|   (follow it, never fight the wall)
//!              length > vector.hook_range_m -> overextended = true
//! (h) derive the MovementState: Tethered > Grounded > Airborne
//! (i) report an Impact when (g3) took speed away
//! ```
//!
//! ## The referee: **the wall wins, the rope gives**
//!
//! Without that ruling two systems decide independently about the same position, and the
//! result is a 30 Hz jitter on the `Transform` the camera hangs off as a child
//! (`src/render/mod.rs`) — which does not look like "two systems without a referee", it looks
//! like "the physics is broken". That is why the order in (g) is **fixed** and the rope
//! length gives, not the position.
//!
//! ## No `Local` as a scratch buffer
//!
//! The buffer for `SpatialIndex::aabb_overlaps` is allocated per tick, not held in a
//! `Local<Vec<_>>`. A guard that forbids `Local<T>` in simulation systems
//! (`docs/multiplayer.md`) cannot tell state from a scratch buffer, and an exemption list is
//! the beginning of the end of the rule. Should the allocation turn out **measurably**
//! expensive, the buffer becomes a field next to the index — not a `Local`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    ReelSpeed, RunAccel, BoostAccel, Impact, MovementState, Hook, PlayerId,
    SpatialIndex, RopeLength, Velocity, Tick,
};

/// One fixed simulation step for every player.
// filled in by job V — F-004, F-005, F-012, F-013, stage 2
pub fn step(
    _zeit: Res<Time<Fixed>>,
    _tick: Res<Tick>,
    _daten: Res<GameData>,
    _index: Res<SpatialIndex>,
    mut _aufprall: MessageWriter<Impact>,
    mut _spieler: Query<(
        &PlayerId,
        &RunAccel,
        &BoostAccel,
        &ReelSpeed,
        &Hook,
        &mut Transform,
        &mut Velocity,
        &mut MovementState,
        &mut RopeLength,
    )>,
) {
}
