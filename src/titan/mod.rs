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
//! ## What stands here since 2026-08-10 — `F-053`, the titan's half of `F-034`, and Q-030
//!
//! - **`F-053` is measured, not asserted.** The wind-up carries the striking hand **8.221 m**
//!   over its 36 ticks, which is **192 px at 40 m** on a 1080-line screen against a criterion of
//!   150 (`tests/titan.rs::f053_the_wind_up_moves_the_hand_far_enough_to_see`,
//!   `docs/images/f053-windup.png`). The pose itself is unchanged — it was already a pure
//!   function of `(state, ticks_in_state)` — what was missing was the number and the two-frame
//!   picture, and a still frame cannot carry either.
//! - **The wind-up can now be READ OFF the picture, not just believed.** `ticks_in_state` and
//!   the length of the state it counts against left [`brain::TitanClock`] for
//!   [`StateClock`](crate::shared::StateClock) in `shared/`, and the kind key left the entity's
//!   `Name` for [`TitanKindName`](crate::shared::TitanKindName), written by [`rig::build_rig`].
//!   The F3 overlay therefore reads `husk#1 Windup 21/36` (`docs/images/f050-states.png`,
//!   tick 433) **without a new edge in the allow list** — that was the whole reason both types
//!   went to `shared/` rather than `debug` being allowed to read `titan/` for one line of text.
//!   Moved and not mirrored: two components holding the same counter are two components that
//!   disagree the first time somebody adds a state edge.
//!   `tests/titan.rs::f050_the_overlay_agrees_with_the_pose` holds the word, the fraction and
//!   the drawn arm to one and the same tick.
//! - **The hit stop reaches the titan.** [`brain::walk`] and [`brain::dissolve`] read
//!   [`HitStop`](crate::shared::HitStop). `combat::hitstop::begin` freezes a titan by putting
//!   the component on him and `RigidBodyDisabled` next to it — and that marker does *nothing*
//!   to a body avian never integrates, which is trap 1 again from the other side. Measured
//!   before the gate: 0.1000 m of walking during a 2-tick freeze the player spent standing
//!   still.
//! - **Q-030 is answered, and the answer is that the question's arithmetic is wrong.** The nape
//!   of a **solid** husk is reachable and always was: the blade does not have to touch the
//!   titan's axis, it has to touch a sphere of radius `cortex_radius_m` with a swept capsule of
//!   radius `thickness_m`. That is `1.60 + 0.55 + 0.12 = 2.27 m` of reach against `1.25 + 0.35
//!   = 1.60 m` of clearance — **0.67 m of margin**, not "zero". No length was changed anywhere;
//!   what changed is that there is now a test that flies the pass
//!   ([`tests/titan.rs::q030_a_flying_player_reaches_the_nape_of_a_solid_husk`]) and a script
//!   that shows it (`scripts/q030-reach.txt`). The head of that section in `tests/titan.rs`
//!   carries the whole table, per class, and names where Q-030's own measurement came from.
//!
//! ## How long a titan lives — 2026-08-12, the hole the hub loop opened
//!
//! **A titan's life is his sortie's `Active` phase.** [`spawn_titan`] hangs
//! `DespawnOnExit(MissionPhase::Active)` on the rig root, which is the same lifetime the
//! mission's own [`WaveSchedule`](crate::mission::WaveSchedule) entity already carries — so at
//! the verdict the pending waves and the standing bodies stop existing in one and the same
//! transition, and the debrief happens on an empty field.
//!
//! Before that line existed the hub loop closed over a field it never cleared: a sortie that
//! ended `Won` or `Lost` left every titan it had spawned walking, with his brain, his
//! kinematic body and his cortex sensor intact. He walked through the 3.0 s debrief, through
//! the transition, and stood in the hub next to the player who had just come home — and the
//! **second** sortie of the session opened on a ring that still held the first one's. No test
//! saw it, because every script we had either kills every titan it spawns or never leaves
//! `Active` at all: the field is only wrong in the run that *survives* the verdict
//! (`tests/titan.rs`, the `f072_` block; `docs/FINDINGS.md` FIND-068).
//!
//! **Why `titan` and not `mission`:** titan bodies have one writer and it is this domain
//! (`docs/architecture.md`, authority table). `mission` says *the sortie is over* — by leaving
//! `Active`, which it is the only writer of — and `titan` decides what that means for a body,
//! exactly the way it decides what a [`TitanHit`](crate::shared::TitanHit) means for one.
//! `tests/titan.rs::f072_the_field_is_cleared_by_titan_and_by_nothing_else` is the falsifiable
//! half: it takes the marker off one titan and demands that the same transition then leave him
//! standing, so a despawn that ever grows inside `mission` goes red here.
//!
//! A titan asked for **outside** a sortie (a `spawn titan` in a `--sandbox` script, in
//! `Briefing`, in the hub) carries the same marker and is untouched by it: there is no `Active`
//! for him to be exited from. One door, no branch — `spawn_titan` is the only place a rig is
//! built, and a second rule for "was there a mission at the time" would be a second lifetime to
//! keep in agreement.
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
//! | the telegraph, in **two** frames | `scripts/f053-windup.txt` → `docs/images/f053-windup.png` |
//! | the cut, against a **solid** husk | `scripts/q030-reach.txt` → `docs/images/f030-solid-husk.png` |
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
use crate::mission::MissionPhase;
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
                // …and, in the same set, the one line that makes a swapped model die where it
                // renders: a `cortex` empty out of the `.glb` beats the position the rig
                // computes out of `scale.ron`. `Changed<ModelAnchors>`-gated, so it costs
                // nothing on the 99.99 % of ticks in which no scene instance became ready.
                (spawn_titans, brain::dissolve, rig::cortex_from_the_model)
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
        // **A titan lives as long as the sortie he was spawned into, and not one tick
        // longer.** See the module head, "How long a titan lives".
        //
        // On the ROOT, so the whole rig goes with him: `despawn_entities_on_exit_state` calls
        // `try_despawn`, and a despawn in bevy 0.19 takes the entity's descendants with it —
        // the pelvis, the four limbs, the torso, the head and the cortex sensor are all
        // `add_child`ren of this entity (`rig::build_rig`). Despawning them one by one from a
        // list would be the version that leaves a collider behind the day somebody adds a
        // tenth part.
        DespawnOnExit(MissionPhase::Active),
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
