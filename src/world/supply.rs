//! `F-019` **Nachschub-Stationen** — the refuel points that stand out in the field.
//!
//! ## The hole this closes, in numbers
//!
//! `docs/QUESTIONS.md` Q-044, measured: at the honest tank (`gas_tank: 300`) a sortie buys
//! **16.7 s of held boost against a 330 s mission**, and until this file existed there was no
//! refuel anywhere outside the headquarters. `mission::hub::refuel_at_stations` serves the two
//! racks *inside the hall* — and the hall is what you leave when the sortie starts. The user's
//! own answer to Q-033 says refills happen away from base; the stations were never built.
//!
//! That is also why `gas_tank` had to go **50x** (300 → 15000) to be testable at all: the tank
//! was papering over a missing world feature. This is the world feature.
//!
//! ## Why it is not `mission::hub`'s code with a different constant
//!
//! Three differences, and each one is the reason the hub's pair of systems could not simply be
//! pointed at a second set of coordinates:
//!
//! | | hub rack (`mission::hub`) | field station (here) |
//! |---|---|---|
//! | supply | **infinite** — the base is the base | **finite**, `gear.ron: resupply.station_uses` |
//! | rate | a trickle you stand in (`gas_per_s: 40`) | a **pump with a duration** (1.5 s, a whole tank) |
//! | who owns it | `mission`, and it lives and dies with the hub | `world`, and it is part of the map |
//!
//! The middle row is the acceptance sentence — *„Nachladen dauert 1,5 s"* — and it is a
//! different mechanism, not a different number: a trickle has no beginning and no end, so it
//! cannot cost a use and cannot be counted.
//!
//! ## Who writes what
//!
//! **This file never touches `Gas` and never touches `Blades`.** It writes
//! [`RefuelRequest`] and [`BladeRestockRequest`], exactly as `mission::hub` does, and
//! `vector::gas::apply_refuel_requests` / `blades::resupply::apply_restock_requests` remain the
//! only writers of those two fields (`docs/architecture.md` authority table, `FIND-063`). The
//! one field this file owns is [`SupplyStation`] itself.
//!
//! No domain edge is bought: the component lives in `shared/`, the messages live in `shared/`,
//! and `render::supply` reads the component without knowing this module exists.
//!
//! **Evidence:** `tests/world.rs::f019_*` · `scripts/f019-supply.txt`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    BladeRestockRequest, PlayerId, RefuelRequest, SupplyStation, Tick,
};

/// Spawns one entity per `maps.ron: <current>.supply_stations` row, at `Startup`.
///
/// **No `Collider`, no `Body`, and — measured — no `Block` either.** A station is a place, not
/// an obstacle: a collider would put a box in the middle of the swing spine and a `Body` would
/// put it in the spatial index where every hook raycast would find it.
///
/// `Block` was the obvious way to make it visible and it is **wrong**, which two guard tests
/// said within a minute of it being tried: `tests/world.rs::f003_the_city_comes_from_the_file_
/// and_not_twice` counts `Block` entities against `plan_blocks` (2875 against 2871 — the four
/// stations) and `f003_every_anchor_tag_in_the_world_comes_from_the_file_and_the_mask_agrees`
/// panicked with *"supply_station_0 stands in the world but not in the plan"*. Both are right:
/// **`Block` means "a cuboid of the city", and a station is not part of the city.** So the
/// station carries nothing but its component and its transform, and `render::build_station_
/// meshes` draws it off `SupplyStation` — which is also why adding four of them to `ashgate`
/// cannot move a single anchor, a single ray or a single collision in any test that stands.
///
/// (This is the trap the round was warned about in so many words: *a map that grew
/// 2048 -> 2059 blocks between an A and a B run*. Here it was four.)
pub fn build_stations(mut commands: Commands, data: Res<GameData>) {
    let Some(map) = data.maps.maps.get(&data.maps.current) else {
        // `world::map::build_map` reports the missing map already; a second identical error
        // per startup teaches nobody anything.
        return;
    };
    let r = &data.gear.resupply;
    // One reload is a **whole tank**, delivered over `station_refill_s`. The rate therefore
    // falls out of the two numbers instead of being a third one that can disagree with them.
    let gas_per_s = data.game.vector.gas_tank / r.station_refill_s;

    for (i, point) in map.supply_stations.iter().enumerate() {
        let center = Vec3::from(point.center_m);
        commands.spawn((
            Name::new(format!("supply_station_{i}")),
            SupplyStation {
                radius_m: r.station_radius_m,
                uses_left: point.uses,
                charge_s: 0.0,
                refill_s: r.station_refill_s,
                gas_per_s,
                served_this_visit: false,
            },
            Transform::from_translation(center),
        ));
    }
    if !map.supply_stations.is_empty() {
        info!(
            "F-019: {} supply stations on {} — {} reloads each, {:.1} s per reload, {:.0} gas/s, {:.1} m reach",
            map.supply_stations.len(),
            map.name,
            r.station_uses,
            r.station_refill_s,
            gas_per_s,
            r.station_radius_m,
        );
    }
}

/// The pump. Runs in `SimulationSystems::PostStep`, exactly where `mission::hub` runs its own,
/// so the position it judges is the one this tick's integration produced.
///
/// Three decisions, and the first one is the whole design:
///
/// - **A use is spent on the tick the pump starts.** See [`SupplyStation`] for why the two
///   obvious alternatives are an exploit and an invisible rule respectively.
/// - **The pump runs to the end whether he stays or not.** A player who dives out at 1.0 s has
///   spent a whole use for two thirds of a tank, and that is a mistake he can see.
///   Everyone standing in the circle drinks from the same running pump — one station is a
///   squad's decision, not a queue (`docs/multiplayer.md`).
/// - **`uses_left` is not decremented to a despawn.** An empty station stays in the world and
///   goes dark (`render::supply`), because a player has to be able to learn that this one is
///   spent — a station that vanishes teaches him that he misremembered the map.
pub fn run_the_pumps(
    time: Res<Time<Fixed>>,
    tick: Res<Tick>,
    players: Query<(&PlayerId, &Transform)>,
    mut stations: Query<(&mut SupplyStation, &Transform, &Name)>,
    mut gas: MessageWriter<RefuelRequest>,
    mut blades: MessageWriter<BladeRestockRequest>,
) {
    let dt = time.delta_secs();
    for (mut station, at, name) in &mut stations {
        // Who is in the circle **this** tick. Collected once and reused for both the trigger
        // and the payout, so a player cannot be "inside" for one and "outside" for the other.
        let mut inside: Vec<PlayerId> = Vec::new();
        for (id, player_at) in &players {
            if player_at.translation.distance(at.translation) <= station.radius_m {
                inside.push(*id);
            }
        }

        // **The visit latch, and it exists because a test found the rule missing.**
        // `tests/world.rs::f019_one_reload_fills_a_tank...` read `3 -> 1` where it wanted
        // `3 -> 2`: a player who simply stood on the station drained all three reloads in 4.5 s
        // without pressing anything, because the tick after one pump stopped, the next one
        // started. That is not *„begrenzte Nachladungen"*, it is a station that empties itself.
        //
        // So a station serves **one reload per visit**: the latch closes when a pump starts and
        // only opens again when the circle is empty. Three uses are three *visits*, which is
        // what makes the counter a thing a player can plan around — and it is also what the
        // acceptance sentence means by a reload *taking* 1.5 s rather than being a rate.
        if inside.is_empty() {
            station.served_this_visit = false;
        }
        if !inside.is_empty()
            && !station.served_this_visit
            && !station.running()
            && station.uses_left > 0
        {
            station.uses_left -= 1;
            station.served_this_visit = true;
            station.charge_s = station.refill_s;
            info!(
                "F-019: {} starts a reload at tick {} for {} player(s) — {} use(s) left after this one",
                name.as_str(),
                tick.0,
                inside.len(),
                station.uses_left,
            );
        }

        if !station.running() {
            continue;
        }

        // Pay out this tick's share to everybody in the circle. `RefuelRequest` carries the
        // amount because gas is one scalar with one rate; `BladeRestockRequest` carries the
        // seconds because a harness is three numbers and their owner is `blades`
        // (`shared::message`, the asymmetry is documented there).
        for id in &inside {
            gas.write(RefuelRequest { player: *id, amount: station.gas_per_s * dt });
            blades.write(BladeRestockRequest { player: *id, seconds: dt });
        }

        // Counted down after the payout, so a `refill_s` of exactly one tick still pays once.
        station.charge_s = (station.charge_s - dt).max(0.0);
        if !station.running() && station.uses_left == 0 {
            info!("F-019: {} is empty at tick {}", name.as_str(), tick.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station(uses: u32) -> SupplyStation {
        SupplyStation {
            radius_m: 6.0,
            uses_left: uses,
            charge_s: 0.0,
            refill_s: 1.5,
            gas_per_s: 10000.0,
            served_this_visit: false,
        }
    }

    #[test]
    fn f019_a_station_with_nothing_left_and_nothing_running_is_empty() {
        let mut s = station(0);
        assert!(s.empty(), "no uses and no pump is what empty means");
        s.charge_s = 0.5;
        assert!(
            !s.empty(),
            "the LAST reload is still running — a station that goes dark while it is still \
             paying out is a lie in the one signal the player has"
        );
    }

    #[test]
    fn f019_a_full_station_is_neither_empty_nor_running() {
        let s = station(3);
        assert!(!s.empty());
        assert!(!s.running(), "a station idles until somebody stands in it");
    }
}
