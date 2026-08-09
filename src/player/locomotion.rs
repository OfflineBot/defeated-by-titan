//! Ground locomotion — **the one system that writes the player's horizontal velocity.**
//!
//! ## Why a direct velocity write and not a force
//!
//! Everything else on this player is an acceleration: gravity comes from avian, the gas boost
//! will come from [`BoostAccel`](crate::shared::BoostAccel) through `Forces`
//! (`apply_linear_acceleration`, "ignoring mass"), so mass never becomes a tuning number.
//! Running on the ground is the exception, and deliberately so: a run speed is a **target**,
//! not a push. `run_speed_m_s = 6.0` has to mean 6 m/s on the first tick and after ten
//! seconds, uphill and against a wall — an acceleration plus a drag term would turn one honest
//! number in `game.ron` into two dishonest ones, and the value you actually reach would depend
//! on the surface. So: assignment, on exactly one component, in exactly one place.
//!
//! **That makes this system a writer of `LinearVelocity`** — avian is the other one, and the
//! two never run at the same time: this one is ordered `.before(PhysicsSystems::First)`, so
//! the solver reads what was written here and nobody reads a half-written velocity. The row
//! is in the authority table of `docs/architecture.md`.
//!
//! ## Only on the ground
//!
//! In the air this system writes nothing at all — no air control, exactly as before the
//! physics arrived. Whoever wants air control adds it as a **contribution** in
//! `SimulationSystems::Drive` (that is what [`RunAccel`](crate::shared::RunAccel) is there
//! for) and not as a second assignment here, or the two fight over the same field.
//!
//! Reads [`MovementState`] from the **end of the previous tick** —
//! [`super::integrator::readback`] derives it out of the real contacts after the physics step.
//! One tick of lag is deterministic and cheaper than an order dependency inside the step.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Buttons, Intent, MovementState};

/// Running and jumping — the only direct writer of a player's [`LinearVelocity`].
///
/// Runs in `SimulationSystems::Integrate`, before **every** avian system.
pub fn ground_locomotion(
    data: Res<GameData>,
    mut players: Query<(&Intent, &MovementState, &mut LinearVelocity)>,
) {
    let s = &data.game.player;

    for (intent, state, mut velocity) in &mut players {
        // On the rope the body belongs to `vector`; downed and on the wall are states of
        // their own. This system speaks only for a player standing on the ground.
        if *state != MovementState::Grounded {
            continue;
        }

        // Movement is player-local: rotate into world coordinates first, then apply.
        let (sin, cos) = intent.yaw.sin_cos();
        let forward = Vec3::new(-sin, 0.0, -cos);
        let right = Vec3::new(cos, 0.0, -sin);
        let desired = (forward * intent.move_y + right * intent.move_x).clamp_length_max(1.0)
            * s.run_speed_m_s;

        velocity.x = desired.x;
        velocity.z = desired.z;

        // **`velocity.y <= 0.0` is not decoration — it is worth 10 % of jump height.**
        // Contact data is one tick old (the narrow phase runs before the solver), so a player
        // who has already taken off still counts as `Grounded` for one more tick. Without
        // this guard, holding the button set the jump speed a second time on that tick and
        // the apex came out at 1.1588 m instead of 1.0562 m — measured on 2026-08-09. You
        // push OFF the ground; you cannot push off it while already leaving it.
        // Measured against a false positive: at rest `velocity.y` is exactly 0.0 — over 300
        // ticks it was never once greater.
        if intent.pressed(Buttons::JUMP) && velocity.y <= 0.0 {
            // Speed, not an impulse: `jump_speed_m_s` is a height you can compute in your
            // head (v²/2g), and it stays that number no matter what the capsule weighs.
            velocity.y = s.jump_speed_m_s;
        }
    }
}
