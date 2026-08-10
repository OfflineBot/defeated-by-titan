//! player — the body: running, jumping, gravity, ground.
//!
//! Reads [`Intent`], **never the keyboard**. Who filled that intent — a human, a script or
//! one day the network — is none of this domain's business, and that is exactly the point
//! (`prompts/init.md` §6 rule 2).
//!
//! ## Since 2026-08-09 the player is a physics body
//!
//! The hand-written `translation += velocity * dt` with a hard-coded ground plane at
//! `y = 0.0` is **gone**, and with it `move_players` and the reader of `shared::Ground`. What
//! moves the player now is avian, registered in `src/lib.rs`. Three components of this domain
//! are therefore written by three different hands, and each has exactly one:
//!
//! | field | writer |
//! |---|---|
//! | `Position`, `Rotation`, `Transform` | **avian** (integrator and writeback) |
//! | `LinearVelocity` | **avian**, plus [`locomotion::ground_locomotion`] before every avian system |
//! | `Velocity`, `MovementState` | [`integrator::readback`], after `PhysicsSystems::Writeback` |
//! | `RopeLength` | [`rope::sync_rope_length`], after `PhysicsSystems::Writeback` |
//!
//! The rope itself (`F-004`, `F-005`) is an avian `DistanceJoint` and lives in [`rope`] —
//! `vector::reel` only says how fast it should get shorter.
//!
//! ## The three traps that are already paid for here
//!
//! - **The collider sits on the SAME entity as the body**, as
//!   `Collider::capsule_endpoints(r, (0, r, 0), (0, h − r, 0))`
//!   (`avian3d-0.7.0/src/collision/collider/parry/mod.rs:800-802`). Not as a child: a ray
//!   would hit the player's own collider, and `with_excluded_entities([player])` does **not**
//!   help, because the filter matches the collider entity and not the body. On the same
//!   entity the exclusion works, the center of mass lands at h/2, and the resting position is
//!   y = 0 — which is what the origin between the feet requires (`docs/conventions.md`).
//! - **`LockedAxes::ROTATION_LOCKED`.** The camera hangs off the player as a child and the
//!   hull is axis-aligned. A capsule that tips over takes both with it —
//!   `tests/render.rs::f002_the_camera_rotates_not_the_player` is the guard.
//! - **`SleepingDisabled`.** avian puts a body to sleep after 0.5 s below 0.15 m/s
//!   (`avian3d-0.7.0/src/dynamics/rigid_body/sleeping.rs:103-107`). A player hanging still on
//!   a rope would fall asleep, and a sleeping player takes no input.

pub mod integrator;
pub mod locomotion;
pub mod rope;

use avian3d::prelude::{
    CoefficientCombine, Collider, Friction, LinearVelocity, LockedAxes, MaxLinearSpeed,
    PhysicsSystems, Restitution, RigidBody, SleepingDisabled,
};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    AimPoint, Blades, BoostAccel, Cli, Gas, GasGrant, Hook, IdCounter, Intent, LocalPlayer,
    MovementState, PlayerId, PrevButtons, ReelSpeed, RopeLength, RunAccel, SimulationSystems,
    Velocity, WarpPlayer,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_local_player)
            .add_systems(
                FixedUpdate,
                (
                    // Before EVERY avian system, not just before `Prepare`: what is written
                    // here is the input to the step, and `PhysicsSystems::First` already
                    // carries avian's own `assert_components_finite` in a debug build.
                    (apply_warps, locomotion::ground_locomotion)
                        .chain()
                        .before(PhysicsSystems::First),
                    // After the writeback, so that what is read back is this step's result
                    // and not the one before it.
                    (integrator::readback, rope::sync_rope_length)
                        .after(PhysicsSystems::Writeback),
                )
                    .in_set(SimulationSystems::Integrate),
            )
            // `F-004`. In `Drive` and not in `Integrate`: `HookAnchored` and `HookReleased`
            // are written one stage earlier in `Intent`, and the chain over the six stages
            // (`src/lib.rs`) puts a command sync point between `Drive` and `Integrate` — so
            // the joint really exists before avian's first system of this same tick looks for
            // it. `.chain()` because a release and a fresh anchor on one side in one tick
            // must not depend on which of the two Bevy runs first.
            .add_systems(
                FixedUpdate,
                (rope::detach_ropes, rope::attach_ropes)
                    .chain()
                    .in_set(SimulationSystems::Drive),
            );
        // The per-substep reel-in. Its own function because it hangs in avian's
        // `SubstepSchedule` and not in `FixedUpdate` — see `rope.rs`, decision 1.
        rope::register(app);
    }
}

/// Spawns **one** player and marks him as the local one.
///
/// Deliberately separate: everyone has a `PlayerId`, exactly one has `LocalPlayer`. A second
/// player (a test, later the network) gets the same components without the marker —
/// `tests/multiplayer.rs` does exactly that.
pub fn spawn_player(
    commands: &mut Commands,
    ids: &mut IdCounter,
    data: &GameData,
    pos: Vec3,
    local: bool,
) -> Entity {
    let id = ids.next_player();
    let s = &data.game.player;
    // Nested, because a tuple in `spawn` takes only so many elements and beyond that
    // hits you as an unreadable trait error (`docs/lessons/bevy.md`).
    let mut e = commands.spawn((
        Name::new(format!("player_{}", id.0)),
        id,
        Intent::default(),
        Velocity::default(),
        MovementState::default(),
        Gas::full(data.game.vector.gas_tank),
        Blades::fresh(data.gear.blades.start_pairs),
        Transform::from_translation(pos),
        // The Vector Gear hangs on the player, not on the world: every player has his own
        // (`docs/multiplayer.md` rule 3). All eight are present from tick 1 on, so that no
        // system filters on a missing component and silently skips the player.
        (
            Hook::default(),
            RopeLength::default(),
            AimPoint::default(),
            GasGrant::default(),
            RunAccel::default(),
            BoostAccel::default(),
            ReelSpeed::default(),
            PrevButtons::default(),
        ),
        // The physics body. See the module header for why each of these is here.
        (
            RigidBody::Dynamic,
            // Endpoints, not `Collider::capsule`: that one centers the capsule on the origin
            // (parry/mod.rs:790-797) and would sink the player half his height into the
            // ground, because a body's origin lies between his feet.
            Collider::capsule_endpoints(
                s.radius_m,
                Vec3::new(0.0, s.radius_m, 0.0),
                Vec3::new(0.0, s.height_m - s.radius_m, 0.0),
            ),
            LockedAxes::ROTATION_LOCKED,
            SleepingDisabled,
            // `F-012` — the clamp exists from day one and is not retrofitted (bible 6.4,
            // fling exploits). Measured: it holds at exactly 75.0000 m/s.
            MaxLinearSpeed(data.game.vector.max_speed_m_s),
            // `Min` and not avian's `Average`: a wall must not be able to put its own
            // friction on the player. See `game.ron: player.friction`.
            Friction::new(s.friction).with_combine_rule(CoefficientCombine::Min),
            Restitution::new(s.restitution).with_combine_rule(CoefficientCombine::Min),
        ),
    ));
    if local {
        e.insert(LocalPlayer);
    }
    e.id()
}

fn spawn_local_player(
    mut commands: Commands,
    mut ids: ResMut<IdCounter>,
    data: Res<GameData>,
    start: Res<Cli>,
) {
    let e = spawn_player(&mut commands, &mut ids, &data, Vec3::new(0.0, 2.0, 0.0), true);
    if start.sandbox {
        // `--sandbox`: an empty field, unlimited gas — just to look at it (§12a).
        commands.entity(e).insert(Gas {
            unlimited: true,
            ..Gas::full(data.game.vector.gas_tank)
        });
    }
}

/// `warp x y z` — the player stands exactly there afterwards (§12c).
///
/// Writes the `Transform`, not `Position`: avian takes the transform over in
/// `PhysicsSystems::Prepare` as long as nothing has written `Position` since the last physics
/// tick (`avian3d-0.7.0/src/physics_transform/mod.rs:215-223`), and this system is ordered
/// before every avian system. `Velocity` is deliberately **not** written — it is derived, and
/// [`integrator::readback`] derives it from the `LinearVelocity` zeroed here.
///
/// ## The ropes are **not** released here — and that is not an oversight (`B-003`)
///
/// A `DistanceJoint` that survives a teleport pulls the player straight back: measured at
/// **47.93 m of drag in one tick** out of a 55.73 m warp. But the release cannot happen in this
/// system: its `Commands` land at the next sync point, and that is behind
/// `PhysicsSystems::StepSimulation` — avian would step once with a teleported body still tied
/// to an anchor. So [`rope::detach_ropes`] reads the same `WarpPlayer` one stage earlier, in
/// `SimulationSystems::Drive`, and [`rope::sync_rope_length`] raises
/// `RopeLength::overextended` so `vector::hook` — the only writer of `Hook` — lets the arm go
/// on its own. Both live in [`rope`]; this system stays what its name says.
fn apply_warps(
    mut messages: MessageReader<WarpPlayer>,
    mut players: Query<(&PlayerId, &mut Transform, &mut LinearVelocity)>,
) {
    for w in messages.read() {
        for (id, mut transform, mut velocity) in &mut players {
            if *id == w.player {
                transform.translation = Vec3::new(w.pos_x, w.pos_y, w.pos_z);
                // Without this the player carries his old velocity along and keeps
                // falling the moment he arrives — a `warp` that does not stop is
                // worthless as a debugging tool (§12c).
                velocity.0 = Vec3::ZERO;
            }
        }
    }
}
