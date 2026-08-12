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
//! ## And ground contact is not the same as the ground moving him (`F-004`)
//!
//! From 2026-08-10 this system also writes [`MovementState::Tethered`], which was declared on
//! day one and written by nobody until then. The rule, its one sentence of justification and
//! the tick it was measured on are in [`movement_state`]; what matters here is only that this
//! stays the **sole writer** of the component and that `Tethered` is inside the set of states
//! it may overwrite, or a tethered player never gets his legs back.
//!
//! No collision events are needed for any of this, and none would arrive:
//! `CollisionEventsEnabled` on the *body* does nothing, it is baked into the BVH proxy when
//! the *collider* spawns.

use avian3d::prelude::{Collisions, LinearVelocity};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Hook, MovementState, PlayerId, Velocity};

/// Which of the three derived states a body is in, **as a function of nothing but its
/// arguments** — so that the rule can be checked without an `App`
/// (`tests/player.rs::f004_a_rope_takes_the_body_over_only_when_the_ground_is_not_moving_it`).
///
/// `ground_speed_m_s` is the **horizontal** speed, `|v.xz|`. The only three states this
/// function can return are the three it may write; who is `Downed` or `OnWall` is somebody
/// else's decision (see [`readback`]).
///
/// ## The whole rule is one line, and the argument for it is one sentence
///
/// **The legs cannot produce more than the ground's top speed.** So a body with a hook in
/// something that is moving faster than that is being moved by the *rope*, whatever the ground
/// contact says — and a body with a hook in something that is standing on a roof at walking
/// pace is being moved by his *legs*, whatever the hook says.
///
/// That second half is why `anchored` alone is not the rule, and it is not a detail: a player
/// standing on a roof with a hook in the wall has to be able to walk and jump like anybody
/// else. `tests/player.rs::f004_a_hook_in_the_wall_does_not_glue_the_player` is the guard, and
/// it goes red for both of the obvious versions of this feature ("write `Tethered` whenever an
/// arm holds", "skip a player who carries an anchored hook").
///
/// ## `ground_top_speed_m_s` is `run_speed_m_s` **plus one tick of the ground's own step**
///
/// Not a tolerance somebody typed. A walking player does not sit exactly on `run_speed_m_s`:
/// measured over 60 ticks of held `W` on `[offlinebot]`, `|v.xz|` alternates between
/// **5.999977112 and 6.000022888** — avian's solver returns the assignment `ground_locomotion`
/// made ±2.3e-5 m/s. On a bare `> run_speed_m_s` that is enough to flip a walking player to
/// `Tethered` mid-stride, and `f004_a_hook_in_the_wall_does_not_glue_the_player` caught exactly
/// that on the first green run.
///
/// So the caller passes `run_speed_m_s + (-gravity_m_s2)/simulation_hz` — the run speed plus
/// **one tick of the deceleration [`super::locomotion::ground_step`] works in**, which is that
/// file's `μ·g` at `μ = 1.0` over one tick, 0.3333 m/s at the file's numbers. It is the
/// resolution of the very system this decision protects, and it is made of two numbers that are
/// already in `game.ron`.
///
/// **There is no number in the middle for this choice to be wrong about**: the noise is
/// 2.3e-5 m/s, the margin is 0.3333 m/s, and the rope hands the body over at 22.138 m/s — four
/// orders of magnitude on either side.
///
/// ## Why the speed and not the ground contact decides it
///
/// Because the ground contact is **one step old** and the velocity is not. The narrow phase
/// runs before the solver, so a body that the rope lifted off the floor during step *n* still
/// reports a contact when [`readback`] looks at the end of step *n* — the same lag the jump
/// guard in [`super::locomotion`] pays for with `velocity.y <= 0.0`.
///
/// **Measured, `scripts/f-001-hooks.txt`, `[offlinebot]`:** the reel starts at t=199 and hands
/// the player to t=200 at `v = (0.000, 17.143, −22.138)` — 28.0 m/s straight along the rope,
/// which is `vector.reel_speed_m_s` to the digit. On that one tick, and on that one tick only,
/// `MovementState` read `Grounded` at 22.138 m/s of horizontal speed. It was worth the whole
/// headline number: `ground_locomotion` deleted the −22.138 (no key was held), which left
/// `(0, 17.143, 0)` — almost pure **tangent** to the rope, and a reel multiplies the tangent by
/// `length_prev/length_new` (`shared::rope::rope_reel_in`). ACT 1 then reported 46.414 m/s off
/// the `vector.max_speed_m_s` clamp. Keeping the −22.138 leaves 28 m/s pointing almost straight
/// **at** the anchor, which a reel cannot amplify at all.
/// **The one speed that splits the legs from the air**, out of `game.ron` and out of nowhere
/// else: `run_speed_m_s` plus one tick of the ground's own deceleration.
///
/// It answers two questions with one number, and that is the point of it being a function
/// instead of two expressions: [`movement_state`] asks *"is the rope carrying him or are his
/// legs?"* and [`super::locomotion::in_flight`] asks *"is this input running or is it thrust?"*.
/// Those are the same question — **the legs cannot produce more than the ground's top speed** —
/// and two copies of the arithmetic would be two answers the day somebody changes one of them.
///
/// Why the extra tick and not a bare `run_speed_m_s` is in [`movement_state`]'s doc: held `W`
/// comes back as 6.000022888 m/s, not as 6.0.
pub fn ground_top_speed_m_s(data: &GameData) -> f32 {
    data.game.player.run_speed_m_s - data.game.gravity_m_s2 / data.game.simulation_hz as f32
}

pub fn movement_state(
    anchored: bool,
    grounded: bool,
    ground_speed_m_s: f32,
    ground_top_speed_m_s: f32,
) -> MovementState {
    if anchored && (!grounded || ground_speed_m_s > ground_top_speed_m_s) {
        return MovementState::Tethered;
    }
    if grounded { MovementState::Grounded } else { MovementState::Airborne }
}

/// Reads the physics step back into the two components the rest of the game speaks.
///
/// **Sole writer of [`Velocity`] and of [`MovementState`]** on a player.
pub fn readback(
    data: Res<GameData>,
    collisions: Collisions,
    mut players: Query<
        (Entity, &LinearVelocity, &Hook, &mut Velocity, &mut MovementState),
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

    // The fastest a tick of ground can hand a body back — see [`ground_top_speed_m_s`] and
    // [`movement_state`] for why it is not just `run_speed_m_s`.
    let ground_top_speed_m_s = ground_top_speed_m_s(&data);

    for (entity, physics_velocity, hook, mut velocity, mut state) in &mut players {
        // `set_if_neq` and not an assignment — here and on the state below. A component that
        // reports itself changed on all sixty ticks makes every `Changed<T>` filter that
        // comes after it worthless, and a standing player really does not change.
        velocity.set_if_neq(Velocity(physics_velocity.0));

        // Being downed and running a wall are states of other domains — `combat::down_at_zero`
        // writes the one, `F-006` will write the other, and neither may be undone here. The
        // three this system decides between are the three [`movement_state`] returns, and
        // `Tethered` **has to be in this list**: a state that can be entered and not left is
        // a player who never gets his legs back.
        if !matches!(
            *state,
            MovementState::Grounded | MovementState::Airborne | MovementState::Tethered
        ) {
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

        // `Hook` is only READ here — `vector::hook::update_hooks` stays its one writer. What is
        // asked of it is the one thing that cannot be seen on the body itself: whether anything
        // is holding on. The rest of the rule is in [`movement_state`].
        let now = movement_state(
            hook.anchored_count() > 0,
            grounded,
            physics_velocity.0.xz().length(),
            ground_top_speed_m_s,
        );
        state.set_if_neq(now);
    }
}
