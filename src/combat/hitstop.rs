//! `F-034` — **the hit stop: the bodies stop, the clock does not.**
//!
//! A husk's cortex is 1.10 m across and a player crosses it at 30 m/s in
//! `1.10 / 30 = 36.7 ms` = **2.2 ticks**. Without a stop the kill happens and the player sees
//! a counter change, not a kill. `gear.ron: feel.hit_stop_cortex_s = 0.12` makes it about 9.4.
//!
//! ## The one-line implementation this file exists to avoid
//!
//! `Time::<Virtual>::set_relative_speed(0.05)` is one line, it looks perfect on screen, and it
//! is wrong: `run_fixed_main_schedule` accumulates `Time<Fixed>`'s overstep out of
//! `Time<Virtual>::delta()` (`bevy_time-0.19.0/src/fixed.rs:243-247`), so slowing virtual time
//! slows **the tick rate itself**. And the tick is not a display quantity —
//! [`Tick`](crate::shared::Tick) is what [`Rng`](crate::shared::Rng) seeds from and what every
//! [`Intent`](crate::shared::Intent) is stamped with. Freezing it stalls the random numbers and
//! drifts the input stamps, per client, which over a wire is a divergence nobody reproduces.
//! avian's `Time<Physics>::set_relative_speed` has exactly the same problem.
//!
//! So the clock keeps running, the tick keeps counting, and a body carrying a
//! [`HitStop`](crate::shared::HitStop) with `ticks_left > 0` simply does not advance this tick.
//! `tests/combat.rs::f034_the_hit_stop_freezes_the_bodies_and_not_the_clock` asserts both
//! halves, and its **first** assertion is the one that goes red on the `Time<Virtual>` version.
//!
//! ## How a body is actually held still
//!
//! With avian's own [`RigidBodyDisabled`], not by zeroing a velocity. Zeroing is not a freeze:
//! gravity keeps accelerating the body inside the step, the position moves by
//! `g·dt² ≈ 0.0056 m` on the first tick, and the criterion is **bit-identical**, not "about the
//! same". `RigidBodyDisabled` takes the body out of the solver, out of the integrator and out
//! of the narrow phase (`avian3d-0.7.0/src/dynamics/rigid_body/mod.rs:329`,
//! `RigidBodyActiveFilter`), so nothing writes `Position` at all — and the velocity the player
//! carried into the cut is still there when the freeze lifts. That is the feel: the impact
//! frame costs time, not momentum.
//!
//! ## `B-004` — a frozen body may not still be holding a joint
//!
//! `RigidBodyDisabled` and a `DistanceJoint` on the same body **corrupt avian's island
//! bookkeeping**, and the process aborts the next time that joint is removed:
//!
//! ```text
//! assertion failed: island.joint_count > 0
//! avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:786
//! ```
//!
//! Read out of avian's source, in the order it happens:
//!
//! 1. `IslandPlugin`'s `On<Insert, (Disabled, RigidBodyDisabled)>` observer strips the body's
//!    `BodyIslandNode` (`islands/mod.rs:126-136`).
//! 2. That component's `on_remove` hook takes the last body out of the island and **removes
//!    the island — while its `joint_count` is still 1** (`islands/mod.rs:1338-1385`). Nothing
//!    in that path looks at the joints; the rope's anchor is `RigidBody::Static` and carries no
//!    island node of its own, so the player was the island's only body.
//! 3. When the freeze lifts, the body gets a fresh `BodyIslandNode`, and `create_island`
//!    recycles exactly that slot — with `joint_count` back at 0.
//! 4. Despawning the joint then decrements a zero (`islands/mod.rs:786`) and the assert fires.
//!
//! That is the whole measured bracket in `scripts/f-flight-cut.txt`: a release **inside** the
//! impact frame was clean because the slot had not been handed out again yet, and every release
//! after it died.
//!
//! **The fix is to take the joint out of the island first, not to stop freezing.** avian has
//! exactly one component for that — [`JointDisabled`] — and its observers do the bookkeeping
//! properly: `Add` removes the joint from the island (`joint_graph/plugin.rs:87`), `Remove`
//! puts it back (`.../plugin.rs:108`). So:
//!
//! - **Freezing:** `JointDisabled` on every joint of the body **is queued before**
//!   `RigidBodyDisabled` on the body. Commands are applied in order, so the island's
//!   `joint_count` is 0 by the time the island is emptied and removed.
//! - **Thawing:** the other way round — the body is enabled first, so that it has an island
//!   again by the time the joint is put back and `add_joint` merges the two ends
//!   (`islands/mod.rs:665-681`; merging an island-less body is the *second* face of the same
//!   bug and panics with "Neither body … is in an island").
//!
//! The three deliberate consequences:
//!
//! - the freeze stays avian's `RigidBodyDisabled`, so `F-034`'s bit-identical `Position` for
//!   exactly 7 ticks is untouched — and it is now bit-identical **with a taut rope**, which it
//!   was not before: the joint used to keep solving through the impact frame;
//! - the rope is not let go of and not re-created: `limits.max`, the anchor entity and
//!   `Rope` all survive the freeze, so `RopeLength` still reads what it read before it;
//! - the joint carries **no** force during the impact frame, which is the same statement the
//!   freeze already makes about gravity — the impact frame costs time, not momentum.
//!
//! Two joints per player at most and a scan only on the tick a hit lands (§11).
//!
//! ## Where in the tick this runs, and why it is not `PostStep`
//!
//! `blades::cut` writes `TitanHit` in `PostStep`. Reading it in the *same* set would be a coin
//! flip at 60 Hz — `SimulationSystems::PostStep` has no order inside it, and the two systems
//! live in different domains, so neither may order the other (that is `src/lib.rs`'s job).
//! So the reaction happens in `Spatial`, the **first** stage of the next tick, which is
//! deterministic and is the same tick `titan::brain::receive_hits` reacts in.

use avian3d::prelude::{DistanceJoint, JointDisabled, RigidBodyDisabled};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{HitStop, HitZone, PlayerId, SimulationSystems, TitanHit, TitanId};

/// How many ticks one hit freezes for. **Seconds in the file, ticks in the code, converted
/// once, at the boundary** — `round(s * simulation_hz)`.
pub fn stop_ticks(zone: HitZone, data: &GameData) -> u32 {
    let seconds = match zone {
        HitZone::Cortex => data.gear.feel.hit_stop_cortex_s,
        // A non-lethal hit stops much less, or every scratch reads like a kill.
        _ => data.gear.feel.hit_stop_normal_s,
    };
    let n = (seconds as f64 * data.game.simulation_hz).round();
    if n.is_finite() && n > 0.0 { n as u32 } else { 0 }
}

/// Reads [`TitanHit`] and freezes the two bodies involved.
///
/// Runs in [`SimulationSystems::Spatial`] — see the module header. **Longest wins**: two hits
/// on the same body in one tick give one freeze of the longer of the two, never the sum. A
/// `u32` that is added to on every tick of an active blade is a body frozen for a quarter of a
/// second per slash, and then for four.
pub fn begin(
    mut commands: Commands,
    data: Res<GameData>,
    mut hits: MessageReader<TitanHit>,
    // `Without` on both sides, or Bevy refuses the system at startup: two queries that both
    // ask for `&mut HitStop` are only disjoint if the filters say so (`B0001`). And they are
    // disjoint by construction — nothing is a player and a titan at once.
    mut players: Query<(Entity, &PlayerId, Option<&mut HitStop>), Without<TitanId>>,
    mut titans: Query<(Entity, &TitanId, Option<&mut HitStop>), Without<PlayerId>>,
    // `B-004`: the joints the frozen body is an end of. Read-only — the joint stays where it
    // is and keeps its length; only avian's own `JointDisabled` marker is written here.
    joints: Query<(Entity, &DistanceJoint)>,
) {
    for hit in hits.read() {
        let ticks = stop_ticks(hit.zone, &data);
        if ticks == 0 {
            continue;
        }
        for (entity, id, stop) in &mut players {
            if *id == hit.by {
                freeze(&mut commands, entity, stop, ticks, &joints);
            }
        }
        for (entity, id, stop) in &mut titans {
            if *id == hit.titan {
                // The titan gets the component but **no `RigidBodyDisabled`**: his position is
                // written by `titan::brain::walk` and not by avian (he is
                // `RigidBody::Kinematic` + `CustomPositionIntegration`), so disabling the body
                // would freeze nothing. What the component does today is gate `combat`'s own
                // systems; the titan's own drive reading it is one line in `titan/`, and that
                // file belongs to another job (`docs/FINDINGS.md`).
                match stop {
                    Some(mut existing) => existing.ticks_left = existing.ticks_left.max(ticks),
                    None => {
                        commands.entity(entity).insert(HitStop::new(ticks));
                    }
                }
            }
        }
    }
}

fn freeze(
    commands: &mut Commands,
    entity: Entity,
    stop: Option<Mut<HitStop>>,
    ticks: u32,
    joints: &Query<(Entity, &DistanceJoint)>,
) {
    match stop {
        Some(mut existing) => existing.ticks_left = existing.ticks_left.max(ticks),
        None => {
            commands.entity(entity).insert(HitStop::new(ticks));
        }
    }
    // `B-004` — **first the joints, then the body, and the order is the fix.** Commands are
    // applied in the order they are queued, so every joint is out of the island before the
    // island is emptied and thrown away. The other way round aborts the process the next time
    // the rope is let go of. See the module header.
    for joint in joints_of(entity, joints) {
        commands.entity(joint).insert(JointDisabled);
    }
    // Idempotent: inserting a marker that is already there costs one archetype check and
    // nothing else, and it saves an `Option<&RigidBodyDisabled>` in the query above.
    commands.entity(entity).insert(RigidBodyDisabled);
}

/// Every joint this body is an end of. At most two per player (`F-004`: two hooks).
fn joints_of(entity: Entity, joints: &Query<(Entity, &DistanceJoint)>) -> Vec<Entity> {
    joints
        .iter()
        .filter(|(_, joint)| joint.body1 == entity || joint.body2 == entity)
        .map(|(e, _)| e)
        .collect()
}

/// One tick off every freeze, and the body moves again at zero.
///
/// Runs in [`SimulationSystems::PostStep`], i.e. **after** the step the body was frozen for.
/// Counting down before the step would give `n` seconds of freeze that are `n − 1` ticks long,
/// and the criterion counts ticks.
pub fn advance(
    mut commands: Commands,
    mut frozen: Query<(Entity, &mut HitStop)>,
    joints: Query<(Entity, &DistanceJoint)>,
) {
    for (entity, mut stop) in &mut frozen {
        if stop.tick() == 0 {
            // `B-004`, and **the mirror image of `freeze`**: the body first, so that it has an
            // island again before the joint is put back into one. A joint whose ends are both
            // island-less panics in `merge_islands` instead.
            commands
                .entity(entity)
                .remove::<HitStop>()
                .remove::<RigidBodyDisabled>();
            for joint in joints_of(entity, &joints) {
                commands.entity(joint).remove::<JointDisabled>();
            }
        }
    }
}

/// The two systems, in the two stages the header argues for. Registered from
/// [`super::CombatPlugin`].
pub fn register(app: &mut App) {
    app.add_systems(FixedUpdate, begin.in_set(SimulationSystems::Spatial))
        .add_systems(FixedUpdate, advance.in_set(SimulationSystems::PostStep));
}
