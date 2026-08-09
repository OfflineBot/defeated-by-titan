//! player — the body: running, jumping, gravity, ground.
//!
//! Reads [`Intent`], **never the keyboard**. Who filled that intent — a human, a script or
//! one day the network — is none of this domain's business, and that is exactly the point
//! (`prompts/init.md` §6 rule 2).
//!
//! **The player's `Transform` has exactly one writer**, and it will be called
//! [`integrator::step`]. The old split "`player` on the ground, `vector` on the rope, kept
//! apart by `MovementState`" does not hold: a gas boost acts in the air **and** on the rope
//! at the same time, so there is no state that separates the two writers
//! (`docs/architecture.md`, authority table).
//!
//! **Status:** the seam is in place. [`integrator::step`] is registered as a stub in
//! `SimulationSystems::Integrate` and does nothing; today's movement still runs in
//! [`move_players`] — WASD, gravity, a ground plane at y = 0, no real collision.
//! **[`move_players`] and the hardcoded `ground_y = 0.0` die in the commit that fills
//! `integrator::step`**, together with `shared::Ground`. Not before: otherwise the player
//! falls for 600 ticks and takes `scripts/t007-first-run.txt` down with him.

pub mod integrator;
pub mod locomotion;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    ReelSpeed, RunAccel, BoostAccel, MovementState, Gas, GasGrant, Hook,
    IdCounter, Intent, Blades, LocalPlayer, PlayerId, SimulationSystems, RopeLength, WarpPlayer,
    Cli, Velocity, PrevButtons, AimPoint,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_local_player)
            .add_systems(FixedUpdate, locomotion::ground_run.in_set(SimulationSystems::Drive))
            .add_systems(
                FixedUpdate,
                (apply_warps, move_players, integrator::step)
                    .chain()
                    .in_set(SimulationSystems::Integrate),
            );
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

fn apply_warps(
    mut messages: MessageReader<WarpPlayer>,
    mut players: Query<(&PlayerId, &mut Transform, &mut Velocity)>,
) {
    for w in messages.read() {
        for (id, mut transform, mut tempo) in &mut players {
            if *id == w.player {
                transform.translation = Vec3::new(w.pos_x, w.pos_y, w.pos_z);
                // Without this the player carries his old velocity along and keeps
                // falling the moment he arrives — a `warp` that does not stop is
                // worthless as a debugging tool (§12c).
                tempo.0 = Vec3::ZERO;
            }
        }
    }
}

/// Running and falling. **Everything per second**, nothing per frame (§11).
fn move_players(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    mut players: Query<(&Intent, &mut Transform, &mut Velocity, &mut MovementState)>,
) {
    let dt = crate::shared::math::clamped_dt_s(time.delta_secs());
    let s = &data.game.player;
    let ground_y = 0.0;

    for (intent, mut transform, mut tempo, mut state) in &mut players {
        if *state == MovementState::Tethered {
            // On the rope the transform belongs to `vector`. This domain does not touch it.
            continue;
        }

        // Movement is player-local: rotate into world coordinates first, then apply.
        let (sin, cos) = intent.yaw.sin_cos();
        let forward = Vec3::new(-sin, 0.0, -cos);
        let right = Vec3::new(cos, 0.0, -sin);
        let desired = (forward * intent.move_y + right * intent.move_x)
            .clamp_length_max(1.0)
            * s.run_speed_m_s;

        let grounded = transform.translation.y <= ground_y + 1e-3;
        if grounded {
            tempo.0.x = desired.x;
            tempo.0.z = desired.z;
            if intent.pressed(crate::shared::Buttons::JUMP) {
                tempo.0.y = s.jump_speed_m_s;
                *state = MovementState::Airborne;
            } else {
                tempo.0.y = 0.0;
                *state = MovementState::Grounded;
            }
        } else {
            tempo.0.y += data.game.gravity_m_s2 * dt;
            *state = MovementState::Airborne;
        }

        transform.translation += tempo.0 * dt;
        if transform.translation.y < ground_y {
            transform.translation.y = ground_y;
            tempo.0.y = 0.0;
        }
    }
}
