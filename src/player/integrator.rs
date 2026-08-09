//! **The integrator is avian.** What is left in this file is the readback.
//!
//! Until 2026-08-09 a hand-written step sequence was planned here: integrate by hand, clamp
//! by hand, substep by hand, resolve the rope by hand, collide against `SpatialIndex` by
//! hand, and a referee on top to decide who wins between wall and rope. **Three rounds of
//! measurement retired all of it**, and each line was retired by a number:
//!
//! - **The rope** is an avian `DistanceJoint` with `limits = (0, L)`. It only pulls, never
//!   pushes (`avian3d-0.7.0/src/dynamics/joints/mod.rs:329-343` corrects only when the
//!   distance exceeds the maximum). Reeling in **through the joint** reaches 58.23 m/s from
//!   v0 = 20 because angular momentum is preserved; the hand-written clamp gives exactly
//!   20.000 — it eats the reel-in, and the reel-in *is* the feeling.
//! - **The referee** is gone. With the speed clamp, 24 substeps and per-substep reeling the
//!   worst wall penetration across 18 measured cases is −0.0043 m of the −0.01 m allowed.
//!   There is nothing left for a referee to arbitrate.
//! - **The substeps** are 24 and come out of `game.ron`, not out of a `ceil()` in Rust.
//!
//! So a player's `Position`, `Rotation` and `LinearVelocity` are written by **avian**, and the
//! `Transform` by avian's writeback. That is the row in the authority table now.
//!
//! ## What still has to happen here, and why it is one system
//!
//! Two project-owned components describe the same body and are read by everybody who must not
//! know avian: [`Velocity`] (that is what `assert speed > 25` measures, and what damage will
//! be computed from) and [`MovementState`]. Both are **derived**, both are derived from the
//! same physics step, and deriving them twice would mean two systems fighting over
//! `ContactGraph`. Hence: one system, run after `PhysicsSystems::Writeback`, one writer each.
//!
//! ## Ground contact comes out of the collider, not out of `y <= 0`
//!
//! [`Collisions`] gives the touching contact pairs of this step
//! (`avian3d-0.7.0/src/collision/contact_types/system_param.rs:53-58`). A manifold normal
//! points **from the first shape to the second**
//! (`.../contact_types/mod.rs:354-357`) — so the normal facing the player is the manifold
//! normal flipped exactly when the player is `collider1`. Whoever forgets that flip has a
//! player who counts as grounded against a ceiling.
//!
//! **And `is_touching()` alone is not ground contact.** avian works with speculative
//! contacts: a pair counts as touching while the shapes are still *approaching*, and
//! `ContactPoint::penetration` is then negative (`.../contact_types/mod.rs:617-623`). Taking
//! that at face value costs exactly what was measured on 2026-08-09: a jumping player stayed
//! `Grounded` for about three more ticks, `locomotion::ground_locomotion` set his jump
//! velocity again on each of them, and the apex came out at **1.2642 m instead of the
//! 1.0562 m** that `jump_speed_m_s²/2g` allows. Holding the button was worth 20 % of extra
//! height — that is not a feature, it is a leak. So a contact counts as ground only from
//! `penetration > -world.collision_margin_m`, i.e. from the distance at which a body comes to
//! a stop in front of a surface anyway.
//!
//! No collision events are needed for any of this, and none would arrive:
//! `CollisionEventsEnabled` on the *body* does nothing, it is baked into the BVH proxy when
//! the *collider* spawns.

use avian3d::prelude::{Collisions, LinearVelocity};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{MovementState, PlayerId, Velocity};

/// Reads the physics step back into the two components the rest of the game speaks.
///
/// **Sole writer of [`Velocity`] and of [`MovementState`]** on a player.
pub fn readback(
    data: Res<GameData>,
    collisions: Collisions,
    mut players: Query<
        (Entity, &LinearVelocity, &mut Velocity, &mut MovementState),
        With<PlayerId>,
    >,
) {
    // Degrees in the file, radians in the code — the conversion happens exactly at the
    // boundary (`docs/conventions.md`). `cos` because a normal is a unit vector: the steeper
    // the surface, the smaller its Y component.
    let min_normal_y = data.game.player.max_ground_slope_deg.to_radians().cos();
    // How far a contact may still be apart and count as ground. The same number the world
    // uses to stop a body in front of a surface — not a second one invented here.
    let min_penetration = -data.game.world.collision_margin_m;

    for (entity, physics_velocity, mut velocity, mut state) in &mut players {
        // `set_if_neq` and not an assignment — here and on the state below. A component that
        // reports itself changed on all sixty ticks makes every `Changed<T>` filter that
        // comes after it worthless, and a standing player really does not change.
        velocity.set_if_neq(Velocity(physics_velocity.0));

        // The rope, being downed and running a wall are states of their own domains. This
        // system only ever decides between standing and falling.
        if !matches!(*state, MovementState::Grounded | MovementState::Airborne) {
            continue;
        }

        // The collider sits on the SAME entity as the body (see `super::spawn_player`), so
        // the collider key and the player entity are the same key.
        let grounded = collisions.collisions_with(entity).any(|pair| {
            let flip = if pair.collider1 == entity { -1.0 } else { 1.0 };
            pair.manifolds.iter().any(|manifold| {
                manifold.normal.y * flip >= min_normal_y
                    && manifold.points.iter().any(|p| p.penetration > min_penetration)
            })
        });

        let now = if grounded { MovementState::Grounded } else { MovementState::Airborne };
        state.set_if_neq(now);
    }
}
