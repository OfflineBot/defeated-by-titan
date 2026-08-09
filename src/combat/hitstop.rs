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
//! ## Where in the tick this runs, and why it is not `PostStep`
//!
//! `blades::cut` writes `TitanHit` in `PostStep`. Reading it in the *same* set would be a coin
//! flip at 60 Hz — `SimulationSystems::PostStep` has no order inside it, and the two systems
//! live in different domains, so neither may order the other (that is `src/lib.rs`'s job).
//! So the reaction happens in `Spatial`, the **first** stage of the next tick, which is
//! deterministic and is the same tick `titan::brain::receive_hits` reacts in.

use avian3d::prelude::RigidBodyDisabled;
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
) {
    for hit in hits.read() {
        let ticks = stop_ticks(hit.zone, &data);
        if ticks == 0 {
            continue;
        }
        for (entity, id, stop) in &mut players {
            if *id == hit.by {
                freeze(&mut commands, entity, stop, ticks);
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

fn freeze(commands: &mut Commands, entity: Entity, stop: Option<Mut<HitStop>>, ticks: u32) {
    match stop {
        Some(mut existing) => existing.ticks_left = existing.ticks_left.max(ticks),
        None => {
            commands.entity(entity).insert(HitStop::new(ticks));
        }
    }
    // Idempotent: inserting a marker that is already there costs one archetype check and
    // nothing else, and it saves an `Option<&RigidBodyDisabled>` in the query above.
    commands.entity(entity).insert(RigidBodyDisabled);
}

/// One tick off every freeze, and the body moves again at zero.
///
/// Runs in [`SimulationSystems::PostStep`], i.e. **after** the step the body was frozen for.
/// Counting down before the step would give `n` seconds of freeze that are `n − 1` ticks long,
/// and the criterion counts ticks.
pub fn advance(mut commands: Commands, mut frozen: Query<(Entity, &mut HitStop)>) {
    for (entity, mut stop) in &mut frozen {
        if stop.tick() == 0 {
            commands
                .entity(entity)
                .remove::<HitStop>()
                .remove::<RigidBodyDisabled>();
        }
    }
}

/// The two systems, in the two stages the header argues for. Registered from
/// [`super::CombatPlugin`].
pub fn register(app: &mut App) {
    app.add_systems(FixedUpdate, begin.in_set(SimulationSystems::Spatial))
        .add_systems(FixedUpdate, advance.in_set(SimulationSystems::PostStep));
}
