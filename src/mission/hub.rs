//! The hub — **the place the game is played out of**, and the loop back into it.
//!
//! > *„dann fehlt auch noch eine hub! bei der man rum laufen kann und missionen starten kann.
//! > das game ist dann eine mission (mit schwierigkeitsleveln)"* — the user, 2026-08-12.
//!
//! Before this, `--mission tutorial` dropped you straight into a fight and `Won`/`Lost` were
//! the end of the run. What is here is the ring that closes it:
//!
//! ```text
//!   Hub ──(a player stands on a pad)──► Deploying ──► Active ──┬─► Won ──┐
//!    ▲                                                         └─► Lost ─┤
//!    │                                       hub.verdict_s later         ▼
//!    └──── hub.debrief_s later, or one click, and only for a sortie ── Debrief
//!          that came from here
//! ```
//!
//! ## Three things, and the reason each one is a **place** and not a menu
//!
//! | thing | what it is | why not a button |
//! |---|---|---|
//! | deployment pad | a circle you stand in ([`DeploymentPoint`]) | the game has mouse-look and **no cursor while playing** (`menu`, `P4`) — a click is not available |
//! | difficulty | one pad per level, out of `missions.ron: hub.deployments` | the choice is a door you walk to, so it costs no UI that does not exist |
//! | refuel station | a circle that fills your tank ([`RefuelStation`]) | *„gas refillt nur im main gebäude an bestimmten stationen/objekten"* (`docs/QUESTIONS.md` Q-033) |
//! | blade rack | a circle that fills your harness ([`BladeRack`]) | the other half of *„das main gebäude in dem der gas und schwert nachschub ist"* — and both of them stand **inside** the hall since 2026-08-12 evening, at the two racks of `maps.ron: ashgate` |
//!
//! ## What this module does **not** do
//!
//! - **It builds no building.** The hub is laid out on whatever map `maps.ron: current` builds;
//!   only its pads are placed, out of `missions.ron: hub`. `maps.ron` belongs to another job —
//!   including the two racks the supply stations now stand on. What this module guarantees is
//!   that the *coordinates* come out of the file; that they are inside the hall is
//!   `tests/mission.rs::f019_every_supply_station_stands_on_the_depot_floor_of_the_main_building`,
//!   which finds the floor by its height and never by a literal.
//! - **It does not touch `Gas` outside a station**, and it never adds a rate anywhere else. The
//!   whole point of Q-033 is that gas is a place you go to.
//! - **It has no lobby, no loadout, no NPCs, no persistence.** A hub in this build is three
//!   amber circles, three cyan ones, and the way back.
//!
//! ## The authority question this module raised, and how it was closed
//!
//! `docs/architecture.md`'s authority table says `Gas` is written by **`vector`**. The first
//! version of [`refuel_at_stations`] (2026-08-12, morning) called `Gas::refill` itself and was
//! therefore a **second writer** — bounded by argument (fixed system order, only ever adding,
//! hub phase only) but a second writer all the same, and the arguments that bounded it were
//! all of the form *"the two never meet"*. That is exactly what stops holding over a wire,
//! where "never meet" becomes two machines holding two numbers for one tank.
//!
//! **Since the same day it asks instead of writes:** a station sends
//! [`RefuelRequest`](crate::shared::RefuelRequest) and `vector::gas::apply_refuel_requests`
//! applies it. `mission` holds no `&mut Gas` anywhere, and the message type lives in `shared`,
//! so no domain edge was bought for it. The seam is the one `WarpPlayer` already uses in
//! [`open_hub`]: the hub decides *what should happen to a player*, the domain that owns the
//! field does it. Guarded by
//! `tests/mission.rs::f072_a_station_asks_for_gas_and_never_writes_the_tank_itself`, which
//! runs the station system with **no `vector` in the app at all** — the one shape the whole-app
//! tests cannot distinguish (`FINDINGS.md` FIND-063).
//!
//! **The harness went the same way on the same day.** [`restock_at_stations`] sends
//! [`BladeRestockRequest`](crate::shared::BladeRestockRequest) and
//! `blades::resupply::apply_restock_requests` is the only caller of `restock` — `mission` holds
//! no `&mut Blades` either. It was built as the seam from the first line rather than repaired
//! into one, which is the whole value of FIND-063 having been written down.
//! Guarded by `tests/mission.rs::f033_a_rack_asks_for_blades_and_never_writes_the_harness_itself`.
//!
//! **The cost, and it is one tick:** the request is written in `PostStep` and applied in the
//! next tick's `Intent`, because a same-tick application would mean ordering a `vector` system
//! against a `mission` system, which no domain may do. 16 ms of latency on a refuel at 40 gas/s
//! is 0.67 gas — `vector::gas` already carries the same trade for its `Hook` read.
//!
//! ⚠️ **Measured on 2026-08-12: the `run_if(in_state(MissionPhase::Hub))` on the station
//! systems is carried twice, and taking it away alone changes nothing observable.** The
//! stations are `DespawnOnExit(MissionPhase::Hub)`, so outside the hub the query is empty
//! anyway; `tests/mission.rs::f072_a_station_is_a_hub_thing_and_does_not_follow_you_into_a_sortie`
//! only goes red when **both** are removed. The `run_if` stays as the guard for the day a
//! station stops being state-scoped — but it is belt, and the lifetime is the braces.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{BladeRestockRequest, Block, PlayerId, RefuelRequest, Tick, WarpPlayer};

use super::phase::MissionPhase;
use super::run::{to_ticks, MissionClock, Mission};

/// A door out of the hub: **stand in this circle and the sortie starts.**
///
/// The trigger is a circle and not a button, and the mission plus the difficulty are the
/// pad's, not a global setting — that is what makes "three difficulties" a level-design
/// question instead of a menu.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct DeploymentPoint {
    /// Key in `missions.ron: templates`.
    pub mission: String,
    /// Key in that template's `difficulties`.
    pub difficulty: String,
    pub radius_m: f32,
}

/// Where gas comes back — **the only thing in the game that ever refills a tank** (Q-033).
///
/// Both numbers are copied onto the component at spawn time out of `gear.ron: resupply`, so
/// that the system that ticks it is a function of what is in the world and does not have to
/// read `GameData` per player per tick.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RefuelStation {
    pub radius_m: f32,
    pub gas_per_s: f32,
}

/// Where **blades** come back — the other half of the supply, and the second half of the user's
/// sentence *„auch das main gebäude in dem der gas und schwert nachschub ist"*.
///
/// **A separate component and not two more fields on [`RefuelStation`]**, although today every
/// station carries both. The map already draws two racks and labels one of them gas and the
/// other blades; the reason they are not split yet is that `data::StationPad` is a bare
/// `center_m` with no way to say which, and `src/data/mod.rs` was not this hand's to widen
/// (`docs/FINDINGS.md` FIND-066). The moment it gains a `kind`, a gas-only station is one that
/// carries `RefuelStation` and no `BladeRack` — the component split is already the right shape,
/// and only [`open_hub`]'s spawn has to change.
///
/// **It carries no rate.** `gear.ron: resupply.blade_pairs_per_s` and `sharpen_per_s` are
/// `blades`' numbers and are read there; this rack knows only how far it reaches and sends the
/// seconds. See [`BladeRestockRequest`](crate::shared::BladeRestockRequest) for why that is the
/// one asymmetry against gas.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BladeRack {
    pub radius_m: f32,
}

/// On the mission entity: **this sortie was deployed from the hub, so it goes back there.**
///
/// A component on the mission and not a flag in a resource, because it is a property of *this*
/// sortie. A mission started with `--mission <name>` does not carry it and stays where it
/// lands — it came from nowhere and there is nowhere to return to (`scripts/f070-lost.txt`
/// reads the verdict 120 ticks after it falls, and `tests/combat.rs` 250 ticks after).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReturnToHub;

/// What the next `Deploying` will fly. Written by [`deploy_on_contact`] and by
/// `mission::begin_mission`, read by `mission::deploy` and `mission::open_the_field`.
///
/// **A `Resource`, and rule 4 is not being bent.** `docs/multiplayer.md` rule 3 forbids putting
/// *player* state in a resource, for the arithmetic reason that there are many players and a
/// resource holds one of anything. There is exactly **one sortie**, and every player in the hub
/// deploys into the same one — a squad in which one player flies the elite waves while his
/// mates fly the recruit ones would not be a feature, it would be the bug. This is the same
/// argument `phase.rs` makes for `State<MissionPhase>`, and it is per session, not per player.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Sortie(pub Option<SortieOrder>);

/// One order: which template, which difficulty, and whether there is a hub to go back to.
#[derive(Clone, Debug, PartialEq)]
pub struct SortieOrder {
    pub template: String,
    /// `None` is the direct drop-in `--mission <name>`: the template's own numbers
    /// (`mission::run::resolve`).
    pub difficulty: Option<String>,
    pub from_hub: bool,
}

/// Half the height of a pad slab. It is flush with the ground on purpose: a pad you can trip
/// over is a pad that changes how the hub walks.
const PAD_HALF_HEIGHT_M: f32 = 0.1;

/// `OnEnter(Hub)`: clear away the sortie that just ended, lay the pads out, put the players
/// back on their feet.
///
/// The order inside the function is the order it reads in: the old mission stops existing
/// **before** the new hub is furnished, so no tick ever sees two `KillTally`s. Everything
/// spawned here carries `DespawnOnExit(MissionPhase::Hub)`, so leaving the hub cleans it up
/// with no system of its own.
pub fn open_hub(
    mut commands: Commands,
    data: Res<GameData>,
    finished: Query<Entity, With<Mission>>,
    players: Query<&PlayerId>,
    mut warp: MessageWriter<WarpPlayer>,
) {
    // The mission entity is not state-scoped: it has to survive `Won`/`Lost` so the debrief can
    // read its clock and its counter. So it is despawned here, at the one moment when nothing
    // needs it any more — and a second sortie therefore starts with one counter, not two.
    let mut cleared = 0;
    for e in &finished {
        commands.entity(e).despawn();
        cleared += 1;
    }

    let hub = &data.missions.hub;
    let amber = signal(&data, "amber");
    let cyan = signal(&data, "cyan");

    for pad in &hub.deployments {
        let center = Vec3::from(pad.center_m);
        commands.spawn((
            Name::new(format!("deployment_{}_{}", pad.mission, pad.difficulty)),
            DeploymentPoint {
                mission: pad.mission.clone(),
                difficulty: pad.difficulty.clone(),
                radius_m: pad.radius_m,
            },
            // Amber, and that is the rule and not a taste: `docs/conventions.md` §3 reserves
            // amber for "cortex, weak points, **objectives**". A deployment pad is the
            // objective of the hub. **No collider** — you walk onto it, you do not climb it.
            Block { size: pad_slab(pad.radius_m), color: amber },
            Transform::from_translation(center),
            DespawnOnExit(MissionPhase::Hub),
        ));
    }

    // Range and rates out of `gear.ron: resupply`. `missions.ron` says *where* the supply is,
    // `gear.ron` says what it gives back; two files, one number each, and no third truth.
    //
    // **Both components on one entity**: a supply point of the main building hands back gas and
    // blades, because `data::StationPad` has no way to say which rack is which (FIND-066) and
    // because `gear.ron: resupply` is one block with one `range_m` for all four numbers. The
    // day the file can say it, this loop inserts one component or the other.
    let resupply = &data.gear.resupply;
    for station in &hub.refuel_stations {
        commands.spawn((
            Name::new("supply_station"),
            RefuelStation { radius_m: resupply.range_m, gas_per_s: resupply.gas_per_s },
            BladeRack { radius_m: resupply.range_m },
            // Cyan: "gas, Vector Gear, anchor points" (`docs/conventions.md` §3).
            Block { size: pad_slab(resupply.range_m), color: cyan },
            Transform::from_translation(Vec3::from(station.center_m)),
            DespawnOnExit(MissionPhase::Hub),
        ));
    }

    // Everybody lands at the hub's spawn point. Through `WarpPlayer` and not by writing the
    // `Transform` here: `player` is the writer of a player's position, the message is the seam
    // that already exists for it, and it zeroes the velocity — a player who came back out of a
    // 60 m/s dive would otherwise arrive in the hub still falling.
    //
    // At startup this writes nothing at all: `spawn_local_player` has not run yet when
    // `begin_mission` walks into the hub, so the query is empty — and the spawn point is the
    // same one anyway.
    let mut sent = 0;
    let landing = Vec3::from(hub.spawn_m);
    for id in &players {
        warp.write(WarpPlayer {
            player: *id,
            pos_x: landing.x,
            pos_y: landing.y,
            pos_z: landing.z,
        });
        sent += 1;
    }

    info!(
        "hub {:?}: {} deployment pad(s), {} supply station(s), {} player(s) landed, \
         {} finished mission(s) cleared",
        hub.name,
        hub.deployments.len(),
        hub.refuel_stations.len(),
        sent,
        cleared
    );
}

/// A pad's slab: as wide as the circle it marks, and 0.2 m thick.
fn pad_slab(radius_m: f32) -> Vec3 {
    Vec3::new(radius_m * 2.0, PAD_HALF_HEIGHT_M * 2.0, radius_m * 2.0)
}

/// One of the three signal colors out of `maps.ron: signals`.
///
/// Loud and grey when the key is missing, never a guessed color: a pad painted in a color
/// nobody chose is a pad that breaks the one rule that lets a player at full speed tell what
/// matters (`docs/conventions.md` §3).
fn signal(data: &GameData, key: &str) -> [f32; 3] {
    match data.maps.signals.get(key) {
        Some((r, g, b)) => [*r, *g, *b],
        None => {
            error!("maps.ron: signals has no {key:?} — the hub pads are painted grey instead");
            [0.5, 0.5, 0.5]
        }
    }
}

/// **The trigger.** A player standing on a deployment pad starts that pad's sortie.
///
/// Runs in `PostStep`, so the position it reads is the one this tick's integration produced —
/// asking in `Intent` would judge the player by where he was before he walked.
///
/// Three decisions that are not obvious:
///
/// - **The nearest pad wins, not the first one the query hands back.** Query iteration follows
///   archetype order; with three pads in one archetype that is spawn order, and "whichever
///   comes first in the file" is not an answer a player can predict when two circles overlap.
/// - **Any player deploys the squad.** Twenty players in a hub and one of them steps onto the
///   pad: the sortie starts for everyone, because there is one sortie
///   (`docs/multiplayer.md`). It is the same reason the phase is a state and not a component.
/// - **The distance is 3D.** A player flying 40 m above the pad is not standing on it — and in
///   the hub, with the Vector Gear idle, that is exactly the case this excludes.
pub fn deploy_on_contact(
    players: Query<&Transform, With<PlayerId>>,
    pads: Query<(&DeploymentPoint, &Transform)>,
    data: Res<GameData>,
    mut sortie: ResMut<Sortie>,
    mut next: ResMut<NextState<MissionPhase>>,
) {
    let mut best: Option<(f32, &DeploymentPoint)> = None;
    for player in &players {
        for (pad, at) in &pads {
            let d = player.translation.distance(at.translation);
            if d > pad.radius_m {
                continue;
            }
            if best.is_none_or(|(closest, _)| d < closest) {
                best = Some((d, pad));
            }
        }
    }
    let Some((distance_m, pad)) = best else {
        return;
    };

    // Checked here and not in `deploy`, because here there is still somewhere to refuse *to*:
    // the player keeps standing in the hub instead of walking into a phase that has no numbers.
    let known = data
        .missions
        .templates
        .get(&pad.mission)
        .is_some_and(|t| t.difficulties.contains_key(&pad.difficulty));
    if !known {
        error!(
            "deployment pad names mission {:?} at difficulty {:?}, which is not in \
             assets/data/missions.ron — no sortie is started",
            pad.mission, pad.difficulty
        );
        return;
    }

    info!(
        "deployment: {:?} at {:?} — a player is {distance_m:.1} m from the pad",
        pad.mission, pad.difficulty
    );
    sortie.0 = Some(SortieOrder {
        template: pad.mission.clone(),
        difficulty: Some(pad.difficulty.clone()),
        from_hub: true,
    });
    next.set(MissionPhase::Deploying);
}

/// **The only refill of a tank in this game** (`docs/QUESTIONS.md` Q-033) — and it *asks*.
///
/// Per tick and per player, one [`RefuelRequest`] over `gas_per_s * dt` while he stands inside
/// a station's circle. **This function does not touch `Gas`**, and it holds no `&mut Gas` at
/// all: `vector::gas::apply_refuel_requests` is the only thing that ever writes a tank
/// (`docs/architecture.md`, authority table; `FINDINGS.md` FIND-063).
///
/// Three things that look like they belong here and deliberately do not:
///
/// - **No check whether the tank is already full.** The station cannot see the tank any more,
///   and it does not need to: `Gas::refill` caps at `Gas::max` and the applier writes through
///   `set_if_neq`, so a request for a full tank changes nothing and wakes no `Changed<Gas>`.
/// - **No accumulating.** One tick, one request, per player in reach. A player standing in two
///   overlapping stations gets one request, exactly as he got one refill before — the `find`
///   is what says "a station", not "every station".
/// - **The rate stays a station's property.** It was copied out of `gear.ron: resupply` at
///   spawn ([`RefuelStation`]); the message carries the tick's amount, so the receiver needs no
///   `GameData` and no knowledge of what a station is.
pub fn refuel_at_stations(
    time: Res<Time<Fixed>>,
    stations: Query<(&RefuelStation, &Transform)>,
    players: Query<(&PlayerId, &Transform)>,
    mut requests: MessageWriter<RefuelRequest>,
) {
    let dt = time.delta_secs();
    for (id, at) in &players {
        let Some((station, _)) = stations
            .iter()
            .find(|(s, pad)| at.translation.distance(pad.translation) <= s.radius_m)
        else {
            continue;
        };
        requests.write(RefuelRequest { player: *id, amount: station.gas_per_s * dt });
    }
}

/// **The only restock of a harness in this game** — and it *asks*, exactly like
/// [`refuel_at_stations`].
///
/// Per tick and per player, one [`BladeRestockRequest`] over the tick's seconds while he stands
/// inside a rack's circle. **This function does not touch `Blades`**, and it holds no
/// `&mut Blades` at all: `blades::resupply::apply_restock_requests` is the only thing that ever
/// calls `restock` (`docs/architecture.md`, authority table; `FINDINGS.md` FIND-066).
///
/// **A second system and not two messages out of one**, although both query the same players
/// against the same positions. Gas and blades are two fields with two owners, and a station
/// that stops giving one of them back one day is then a component that comes off the entity
/// rather than a branch inside a loop — which is the whole point of the component split (see
/// [`BladeRack`]). The cost is one extra pass over at most a handful of racks.
pub fn restock_at_stations(
    time: Res<Time<Fixed>>,
    racks: Query<(&BladeRack, &Transform)>,
    players: Query<(&PlayerId, &Transform)>,
    mut requests: MessageWriter<BladeRestockRequest>,
) {
    let dt = time.delta_secs();
    for (id, at) in &players {
        if !racks
            .iter()
            .any(|(rack, pad)| at.translation.distance(pad.translation) <= rack.radius_m)
        {
            continue;
        }
        // One request per player per tick, never one per rack he is standing in — the same
        // rule `refuel_at_stations` follows with its `find`. Two overlapping racks are still
        // one supply point as far as the harness is concerned.
        requests.write(BladeRestockRequest { player: *id, seconds: dt });
    }
}

/// **The way home, and it has two stops.** A sortie that came out of the hub goes
/// `Won`/`Lost` → [`MissionPhase::Debrief`] → `Hub`, and both legs are counted off the one
/// tick the verdict was decided on.
///
/// ```text
///   Won | Lost  ──hub.verdict_s──►  Debrief  ──hub.debrief_s──►  Hub
/// ```
///
/// It reads [`MissionClock::decided_at_tick`] rather than starting a timer of its own: that
/// tick is already written, by the system that spoke the verdict, and a second clock counting
/// the same thing is a second answer to "when did this end". Both legs measure from it, so the
/// debrief cannot drift by the tick the transition itself costs.
///
/// ⚠️ **In a run with a window the second leg is never spent.** `menu` opens
/// `Screen::Debrief` when this phase is entered, every screen that is not `Playing` stops
/// `Time<Virtual>`, and `FixedUpdate` — where this system lives — does not run on a stopped
/// clock. So the player reads the report for as long as he wants and the button he presses is
/// what ends it (`shared::AbandonSortie`, through `take_orders_from_the_menu`). The number is
/// what a `--headless` or `--script` run waits instead: those have no window, therefore no
/// menu, therefore nothing that could hold the phase open.
///
/// **One system and not two**, because the two legs are one question — *how far along the way
/// home is this sortie* — and splitting it would mean two readers of `decided_at_tick` that can
/// disagree about which leg is running.
pub fn walk_the_way_home(
    tick: Res<Tick>,
    data: Res<GameData>,
    phase: Res<State<MissionPhase>>,
    missions: Query<&MissionClock, With<ReturnToHub>>,
    mut next: ResMut<NextState<MissionPhase>>,
) {
    let hub = &data.missions.hub;
    let verdict = to_ticks(hub.verdict_s, data.game.simulation_hz);
    let debrief = to_ticks(hub.debrief_s, data.game.simulation_hz);
    for clock in &missions {
        let Some(decided) = clock.decided_at_tick else {
            continue;
        };
        let since = tick.0.saturating_sub(decided);
        match *phase.get() {
            MissionPhase::Debrief if since >= verdict.saturating_add(debrief) => {
                info!("debrief over at tick {} — back to the hub", tick.0);
                next.set(MissionPhase::Hub);
            }
            MissionPhase::Won | MissionPhase::Lost if since >= verdict => {
                info!("the verdict has stood {since} ticks — the debrief");
                next.set(MissionPhase::Debrief);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pad_is_as_wide_as_the_circle_it_marks() {
        // A marker that is not the size of its own trigger is the reason a player stands next
        // to a pad that looks like he is on it.
        let slab = pad_slab(3.0);
        assert_eq!(slab.x, 6.0);
        assert_eq!(slab.z, 6.0);
        assert!(slab.y < 0.25, "a pad you can trip over changes how the hub walks: {slab:?}");
    }
}
