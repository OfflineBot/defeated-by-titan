//! titan — the titans: rig, limbs, cortex, AI
//!
//! **At least half of all enemy kinds carry an anti-autopilot property** (bible 4) — otherwise
//! the fight degenerates into clicking on targets. The husk is the one that does not: his role
//! in the bible is *"the fundamentals of the approach angle"*, which is exactly why he is the
//! kind this round builds.
//!
//! **Every attack has a wind-up of at least 0.4 s** and the cortex is the only lethal point
//! (bible 2, pillar P4: readability before realism). The player should never have to ask why
//! he died.
//!
//! Reads [`TitanHit`](crate::shared::TitanHit) and decides for itself what a hit means for its
//! body — `combat` does not know how a titan is built.
//!
//! ## What stands here since 2026-08-09 — `F-050`, `F-056`, `F-064`
//!
//! | file | what |
//! |---|---|
//! | [`rig`] | the nine entities, every length out of `scale.ron` |
//! | [`brain`] | the reduced FSM, the straight-line walk, the death |
//! | [`pose`] | the arm and the lean, as a pure function of `(state, ticks_in_state)` |
//!
//! ## The four traps that are paid for here
//!
//! 1. **`RigidBody::Kinematic` needs `CustomPositionIntegration`** — otherwise the titan moves
//!    twice per tick. `SolverBodyPlugin` gives a `SolverBody` to every dynamic *and kinematic*
//!    body (`avian3d-0.7.0/src/dynamics/solver/solver_body/plugin.rs:25-30`, inserted at
//!    `:147-150`) and `integrate_positions` filters only `Without<CustomPositionIntegration>`
//!    (`.../dynamics/integrator/mod.rs:503-504`). See [`rig::build_rig`].
//! 2. **The cortex is a child `Sensor` on `LAYER_TITAN_CORTEX`, under the head.** Physically
//!    intangible, still hittable: `SpatialQuery`'s collider query carries **no**
//!    `Without<Sensor>` (`.../spatial_query/system_param.rs:59-64`), unlike `MoveAndSlide`'s
//!    (`.../character_controller/move_and_slide.rs:82`). Under the head, so it follows the
//!    pose through `GlobalTransform` and nobody has to remember to move it.
//! 3. **The pose never reads a clock.** `AnimationPlayer` is available and must not be used —
//!    see [`pose`].
//! 4. **The class cap is a refusal, not a clamp.** See [`spawnable`].
//!
//! ## The evidence
//!
//! | what | how |
//! |---|---|
//! | the numbers and the red tests | `tests/titan.rs`, `cargo test --test titan` |
//! | the husk and his cortex | `scripts/f056-husk.txt` → `docs/images/f056-husk.png` |
//! | the wind-up at tick 21 of 36 | `scripts/f050-states.txt` → `docs/images/f050-states.png` |
//!
//! Both scripts are `--offscreen` runs and both PNGs come out bit-identical on two runs. The
//! header of each script says why its camera stands where it stands — a framing that was
//! arrived at by looking at the image, not by reasoning about it.
//!
//! ## What is deliberately not built
//!
//! No navigation (`F-052`, Round 2), no perception model (`F-051` — one number,
//! `aggro_radius_m`, stands in for it), no walk cycle, no second animated arm, no damage from
//! anything but the cortex. `Alerted` and `Stagger` are missing from `TitanState` on purpose.

pub mod brain;
pub mod pose;
pub mod rig;

use avian3d::prelude::PhysicsSystems;
use bevy::prelude::*;

use crate::data::{GameData, TitanKind};
use crate::shared::{IdCounter, SimulationSystems, SpawnTitan, TitanId};

use brain::{TitanClock, TitanGait, TitanTarget, TitanTiming, TitanTuning};
use pose::PoseAngles;
use rig::TitanRig;

pub struct TitanPlugin;

impl Plugin for TitanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                // `Drive`: read the messages of the last tick, count the accumulator up,
                // decide the edge, then put the body into the pose that goes with it.
                // `.chain()`, because the pose of tick *n* has to be the pose of the state
                // the FSM just decided on — not of the one before it.
                (brain::receive_hits, brain::advance, pose::apply_pose)
                    .chain()
                    .in_set(SimulationSystems::Drive),
                // `Integrate`, and **before every avian system**: what is written here is the
                // input to the step (the same reasoning as `player::apply_warps`).
                brain::walk
                    .before(PhysicsSystems::First)
                    .in_set(SimulationSystems::Integrate),
                // `PostStep`: a titan asked for this tick enters the world at the **end** of
                // it and takes its first step in the next one. That is what makes `Idle` a
                // state you can observe rather than a value nobody ever sees.
                (spawn_titans, brain::dissolve)
                    .chain()
                    .in_set(SimulationSystems::PostStep),
            ),
        );
    }
}

/// Why a spawn did not happen. **A named refusal, never a silent clamp.**
///
/// A clamp is the shortcut that makes `spawn titan bellower` produce a 14 m titan and a green
/// run — and then nobody finds out that the 21 m class was never tested. Every arm of this
/// enum names the file the answer lives in.
#[derive(Debug, Clone, PartialEq)]
pub enum SpawnRefused {
    /// No such key in `titan.ron`.
    UnknownKind { kind: String },
    /// The kind names a size class that is not in `scale.ron: titan.classes`.
    UnknownClass { kind: String, class: String },
    /// `scale.ron: titan.max_spawnable_class` names a class that does not exist.
    UnknownCap { cap: String },
    /// The kind's class is taller than the cap. **This is a user decision made in his
    /// absence** (`docs/QUESTIONS.md` Q-028) and taking it back is one line in `scale.ron`.
    AboveClassCap {
        kind: String,
        class: String,
        height_m: f32,
        cap: String,
        cap_height_m: f32,
    },
}

impl std::fmt::Display for SpawnRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnRefused::UnknownKind { kind } => write!(
                f,
                "spawn refused: titan kind {kind:?} is not in assets/data/titan.ron"
            ),
            SpawnRefused::UnknownClass { kind, class } => write!(
                f,
                "spawn refused: kind {kind:?} names size class {class:?}, which is not in \
                 assets/data/scale.ron titan.classes"
            ),
            SpawnRefused::UnknownCap { cap } => write!(
                f,
                "spawn refused: scale.ron titan.max_spawnable_class = {cap:?} is not one of \
                 titan.classes"
            ),
            SpawnRefused::AboveClassCap { kind, class, height_m, cap, cap_height_m } => write!(
                f,
                "spawn refused: {kind:?} is class {class:?} at {height_m} m, above the cap \
                 {cap:?} at {cap_height_m} m (scale.ron titan.max_spawnable_class, \
                 docs/QUESTIONS.md Q-028). Not clamped — raise the cap or pick another kind"
            ),
        }
    }
}

impl std::error::Error for SpawnRefused {}

/// May this kind spawn at all? **The class cap of `F-064`, as a `Result`.**
///
/// The rank of a class is its `height_m` — the file gives the five classes no order of their
/// own, and height is the only thing "larger than" can honestly mean here. `huge` (21 m) and
/// `boss` (28 m) are above `large` (14 m) today, so the bellower is unspawnable, and that is
/// intended: at `width_fraction` 0.25 a `huge` titan is 5.25 m wide in a 7.0 m street and a
/// `boss` is exactly 7.00 m, which is not a tight alley but a wall.
pub fn spawnable<'a>(data: &'a GameData, kind_name: &str) -> Result<&'a TitanKind, SpawnRefused> {
    let kind = data
        .titan(kind_name)
        .ok_or_else(|| SpawnRefused::UnknownKind { kind: kind_name.to_string() })?;
    let class = data.size_class(&kind.size_class).ok_or_else(|| SpawnRefused::UnknownClass {
        kind: kind_name.to_string(),
        class: kind.size_class.clone(),
    })?;
    let cap_name = &data.scale.titan.max_spawnable_class;
    let cap = data
        .size_class(cap_name)
        .ok_or_else(|| SpawnRefused::UnknownCap { cap: cap_name.clone() })?;
    if class.height_m > cap.height_m {
        return Err(SpawnRefused::AboveClassCap {
            kind: kind_name.to_string(),
            class: kind.size_class.clone(),
            height_m: class.height_m,
            cap: cap_name.clone(),
            cap_height_m: cap.height_m,
        });
    }
    Ok(kind)
}

/// Builds one titan, or says why not.
///
/// Separate from the system so that a wave in `mission` and a `spawn` in a script go through
/// **one** door — two spawners are two rigs a month later.
pub fn spawn_titan(
    commands: &mut Commands,
    ids: &mut IdCounter,
    data: &GameData,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind_name: &str,
    pos: Vec3,
) -> Result<(Entity, TitanId), SpawnRefused> {
    let kind = spawnable(data, kind_name)?;
    let rig = TitanRig::of(data, kind).ok_or_else(|| SpawnRefused::UnknownClass {
        kind: kind_name.to_string(),
        class: kind.size_class.clone(),
    })?;
    let id = ids.next_titan();
    let root = rig::build_rig(commands, meshes, materials, data, kind_name, kind, &rig, id, pos);
    let s = &data.scale.titan;
    commands.entity(root).insert((
        TitanTiming::of(kind, data.game.simulation_hz),
        TitanTuning::of(kind),
        TitanClock::default(),
        TitanTarget::default(),
        TitanGait::default(),
        PoseAngles {
            windup_arm_deg: s.windup_arm_deg,
            windup_lean_deg: s.windup_lean_deg,
            strike_arm_deg: s.strike_arm_deg,
        },
    ));
    Ok((root, id))
}

/// Consumes [`SpawnTitan`] — the message the script driver and `mission` write.
///
/// Until today it had **no reader**: `spawn titan husk 0 0 -40` in a script wrote a message
/// into the void and `assert titans > 0` measured 0 with every test green.
fn spawn_titans(
    mut commands: Commands,
    mut messages: MessageReader<SpawnTitan>,
    mut ids: ResMut<IdCounter>,
    data: Res<GameData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for message in messages.read() {
        match spawn_titan(
            &mut commands,
            &mut ids,
            &data,
            &mut meshes,
            &mut materials,
            &message.kind,
            message.pos(),
        ) {
            Ok((_, id)) => info!(
                "titan {} ({}) at {:?}",
                id.0,
                message.kind,
                message.pos()
            ),
            // Loud, and it is not a panic: a script that names a kind wrong should say so and
            // keep running, so that the rest of the run still produces its numbers.
            Err(refusal) => error!("{refusal}"),
        }
    }
}
