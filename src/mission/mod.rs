//! mission — the sortie: objectives, phases, spawn waves, victory and defeat
//!
//! **This is what turns a sandbox into a game.** Before it, a titan standing in a grey city was
//! a demo; with it there is one mission, one way to win and a clock that decides against you
//! (`docs/PLAN-GAME.md` §1).
//!
//! **One mission arc runs 5–7 minutes** and is a complete arc with noticeable progress
//! (Bible 5, change 10). Mission templates live in `assets/data/missions.ron`: name, map,
//! duration, `kill_target`, spawn waves. **A new mission is file work, not Rust.**
//!
//! ## What stands here since 2026-08-09 — `F-070`, `F-071`
//!
//! | file | what |
//! |---|---|
//! | [`phase`] | the six phases, and why a `Resource` is right here and not a breach of rule 4 |
//! | [`run`] | the clock in ticks, the per-player counter, the wave list, template ⊕ difficulty |
//! | [`hub`] | **the place you play out of** (2026-08-12): deployment pads, refuel stations, the way back |
//!
//! ```text
//! Startup
//!    │
//!    ├─ --hub ──► Hub ──(a player stands on a deployment pad)──┐
//!    │             ▲                                           │
//!    │             │  hub.debrief_s after the verdict          │
//!    │             │  (only for a sortie that came from here)  ▼
//!    ├─ --mission <name> ────────────────► Deploying ──► Active ──┬─ kill_target ──► Won
//!    │                                     (entity,      (waves,  │
//!    │                                      clock,        kills)  └─ deadline, or every
//!    │                                      counter)                 player down ──► Lost
//!    └─ neither: stays in Briefing, and nothing in this domain ever runs
//! ```
//!
//! **A difficulty is data, not a branch** — `missions.ron: templates.<m>.difficulties` holds
//! kill target, deadline and waves per level, and [`run::resolve`] is the one place that puts
//! template and level together (`F-071`, the user 2026-08-12).
//!
//! ## The four traps that are paid for here
//!
//! 1. **The wall-clock timer.** A timeout out of `Time::delta_secs()` fires at a tick that
//!    depends on the frame rate — every `--script` run in the repository becomes flaky, and
//!    the code looks right in review. Everything here counts ticks ([`run::MissionClock`]).
//! 2. **`titans == 0` as the win condition.** It looks right and is an instant, silent win at
//!    tick 0, before a single wave has spawned — and it reads as a bug in the spawner. The win
//!    is counted from cortex kills, never from an empty field
//!    (`tests/mission.rs::f071_an_empty_field_before_the_first_wave_is_not_a_win`).
//! 3. **Counting `TitanHit` messages.** A torso hit would then win the mission. Only
//!    `HitZone::Cortex` counts, and a titan is paid for **once**: a dissolving body keeps its
//!    `TitanId` for `death_s` (see [`run::KillTally`]).
//! 4. **A wave that spawns into a finished mission.** The schedule entity carries
//!    `DespawnOnExit(MissionPhase::Active)` — note the name, `StateScoped` was renamed at
//!    `bevy_state-0.19.0/src/state_scoped.rs:149`.
//!
//! ## Where the numbers come from
//!
//! `target_duration_s`, `kill_target` and the waves come out of `missions.ron`; the tick rate
//! out of `game.ron: simulation_hz`; the spawn ring out of `maps.ron:
//! layout.clear_radius_m`. **Not one of them is a literal in this domain.** Set
//! `target_duration_s: 10.0` in the file and the mission is lost at tick 600 without a rebuild
//! — `tests/mission.rs::f070_the_deadline_follows_the_file_and_not_a_literal` does exactly that.
//!
//! ## What is deliberately not built
//!
//! No briefing screen, no extraction (`F-073`), no restart (`F-074`), no reward booking
//! (`progress`), no objective *kinds* beyond "kill n" — `F-071` is the skirmish and nothing
//! else. And **the civilian clause of `F-071`'s description ("without too many NPC civilians
//! dying") is dropped**: there are no NPCs in this game and no `F-ID` for them anywhere near.
//! That is why `F-071` cannot go above 🟨 (`docs/PLAN-GAME.md` §8).

pub mod hub;
pub mod phase;
pub mod run;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    Cli, Health, HitZone, PlayerId, SimulationSystems, SpawnTitan, Tick, TitanHit,
};

pub use hub::{DeploymentPoint, RefuelStation, ReturnToHub, Sortie, SortieOrder};
pub use phase::MissionPhase;
pub use run::{resolve, KillTally, Mission, MissionClock, SortieNumbers, WaveSchedule};

pub struct MissionPlugin;

impl Plugin for MissionPlugin {
    fn build(&self, app: &mut App) {
        // Registered **always**, exactly like the F3 overlay and for the same reason: a state
        // that only exists under `--mission` cannot be read by `hud`, `debug` or a test in any
        // other launch mode, and a switch that is there or not depending on the flags is the
        // kind of fake switch you first notice in the image (`docs/lessons/bevy.md`).
        // Without `--mission` it simply stays `Briefing` forever.
        app.init_state::<MissionPhase>();
        // The order of the next sortie. Registered always and empty by default: `deploy` reads
        // it, and a resource that only exists under one launch mode is a system that panics
        // under the others.
        app.init_resource::<Sortie>();

        app.add_systems(Startup, begin_mission)
            .add_systems(OnEnter(MissionPhase::Deploying), deploy)
            .add_systems(OnEnter(MissionPhase::Active), open_the_field)
            .add_systems(OnEnter(MissionPhase::Won), announce)
            .add_systems(OnEnter(MissionPhase::Lost), announce)
            // The hub is furnished on entry and cleared on exit — the clearing is
            // `DespawnOnExit(MissionPhase::Hub)` on every entity spawned there, so there is no
            // second system that has to remember to run.
            .add_systems(OnEnter(MissionPhase::Hub), hub::open_hub);

        // ---- the hub loop ------------------------------------------------------------------
        //
        // All three in `PostStep`, and that is the whole ordering story: both triggers have to
        // read the position **this** tick's integration produced, not the one the player had
        // before he walked. The station only *asks* — it writes `shared::RefuelRequest`, and
        // `vector::gas::apply_refuel_requests` (`Intent`) applies it in the next tick, because
        // `Gas` has exactly one writer and it is not this domain (`docs/architecture.md`,
        // authority table; `FINDINGS.md` FIND-063).
        app.add_systems(
            FixedUpdate,
            (hub::deploy_on_contact, hub::refuel_at_stations)
                .in_set(SimulationSystems::PostStep)
                .run_if(in_state(MissionPhase::Hub)),
        )
        .add_systems(
            FixedUpdate,
            hub::return_to_hub
                .in_set(SimulationSystems::PostStep)
                .run_if(a_verdict_has_fallen),
        )
        // `Deploying → Active` needs one system, because the hub sets `Deploying` from inside
        // the running game and a `NextState` set in `OnEnter` is applied a frame later anyway.
        // At startup this never fires: `begin_mission` walks the chain itself, so the game is
        // already `Active` before the first tick (`F-070`).
        .add_systems(
            FixedUpdate,
            open_the_gate
                .in_set(SimulationSystems::PostStep)
                .run_if(in_state(MissionPhase::Deploying)),
        );

        app.add_systems(
            FixedUpdate,
            (
                // `Intent`: the mission **wants** a titan. It has to stand before
                // `SimulationSystems::PostStep`, where `titan::spawn_titans` reads the message
                // — inside one set the order is not fixed, and a wave that lands one tick
                // later on some machines is a divergence nobody reproduces.
                release_due_waves.in_set(SimulationSystems::Intent),
                // `PostStep`: consequences of the step. Count first, then judge, `.chain()` —
                // the kill of the last tick has to be able to win the mission in that same
                // tick, not in the next one.
                (count_kills, decide).chain().in_set(SimulationSystems::PostStep),
            )
                .run_if(in_state(MissionPhase::Active)),
        );
    }
}

/// Reads `--mission <name>` and walks `Briefing → Deploying → Active` **before the first
/// tick**.
///
/// An exclusive system, because it runs the `StateTransition` schedule itself. Bevy applies a
/// `NextState` in exactly one place per frame (`bevy_state-0.19.0/src/app.rs:335`,
/// `insert_after(PreUpdate, StateTransition)`), so a chain of three phases through `NextState`
/// alone would need three frames — the mission would be `Active` only from tick 3 on, and
/// `F-071`'s "still `Active` at tick 1" would be a lie about the deploy, not about the win
/// check. Running the schedule twice here costs two passes over five variants at startup and
/// gives every phase a real `OnEnter`.
///
/// **Without `--mission` and without `--hub` this does nothing at all** and the game stays in
/// `Briefing`.
///
/// `--hub` wins over `--mission`: the hub is where a mission is *chosen*, so naming both is a
/// contradiction, and the loud half of it is the one the player asked for.
fn begin_mission(world: &mut World) {
    let start = world.resource::<Cli>().clone();

    if start.hub {
        if start.mission.is_some() {
            warn!(
                "--hub and --mission {:?} were both given — the hub decides which sortie is \
                 flown, so --mission is ignored",
                start.mission
            );
        }
        world.resource_mut::<NextState<MissionPhase>>().set(MissionPhase::Hub);
        world.run_schedule(StateTransition);
        return;
    }

    let Some(name) = start.mission.clone() else {
        return;
    };

    {
        let data = world.resource::<GameData>();
        let Some(template) = data.missions.templates.get(&name) else {
            // Loud and not a panic: a mistyped mission name should say so and leave a playable
            // sandbox standing, exactly like a mistyped titan kind in `titan::spawn_titans`.
            let known: Vec<&str> = data.missions.templates.keys().map(|k| k.as_str()).collect();
            error!(
                "--mission {name:?} is not in assets/data/missions.ron — known: {known:?}. \
                 No mission is started; the phase stays Briefing."
            );
            return;
        };
        // The template names its map, but the world builds `maps.ron: current`. Today that is
        // "graybox" and the tutorial says "ashgate", which does not exist yet. Say so once
        // instead of letting somebody wonder later which city they are looking at.
        if data.map(&template.map).is_none() {
            warn!(
                "mission {name:?} names map {:?}, which is not in assets/data/maps.ron — the \
                 world builds {:?} instead (maps.ron: current)",
                template.map, data.maps.current
            );
        }
    }

    // The direct drop-in: no difficulty (the template's own numbers) and no way back — see
    // `hub::ReturnToHub`.
    world.resource_mut::<Sortie>().0 = Some(SortieOrder {
        template: name,
        difficulty: None,
        from_hub: false,
    });

    for next in [MissionPhase::Deploying, MissionPhase::Active] {
        world.resource_mut::<NextState<MissionPhase>>().set(next);
        world.run_schedule(StateTransition);
    }
}

/// Whether the sortie has been decided — the condition [`hub::return_to_hub`] hangs on.
///
/// A named function and not `in_state(Won).or(in_state(Lost))`: the predicate is
/// `MissionPhase::is_decided`, it is written down once next to the enum, and a third phase that
/// counts as a verdict one day changes one line instead of two call sites.
fn a_verdict_has_fallen(phase: Res<State<MissionPhase>>) -> bool {
    phase.get().is_decided()
}

/// `Deploying → Active`, one tick after the hub started a sortie.
fn open_the_gate(mut next: ResMut<NextState<MissionPhase>>) {
    next.set(MissionPhase::Active);
}

/// The mission comes into being: one entity with its clock and its counter.
fn deploy(mut commands: Commands, sortie: Res<Sortie>, data: Res<GameData>, tick: Res<Tick>) {
    let Some((order, numbers)) = chosen(&sortie, &data) else {
        return;
    };
    let clock = MissionClock::new(tick.0, numbers.target_duration_s, data.game.simulation_hz);
    info!(
        "mission {:?} ({}) deployed at tick {} — {} kills in {} ticks ({} s){}",
        order.template,
        numbers.name,
        tick.0,
        numbers.kill_target,
        clock.duration_ticks,
        numbers.target_duration_s,
        if order.from_hub { ", out of the hub" } else { "" }
    );
    let mission = commands
        .spawn((
            Mission { template: order.template.clone(), name: numbers.name.clone() },
            clock,
            KillTally::with_target(numbers.kill_target),
        ))
        .id();
    if order.from_hub {
        // The way back, carried by the sortie itself and not by a global flag: `--mission
        // <name>` came from nowhere and stays on its verdict.
        commands.entity(mission).insert(ReturnToHub);
    }
}

/// The field opens: the waves get their ticks and their places.
///
/// `DespawnOnExit(MissionPhase::Active)` is the point of the separate entity — when the verdict
/// falls, the pending waves stop existing in the same transition, and no titan walks into a
/// mission that is already over.
fn open_the_field(mut commands: Commands, sortie: Res<Sortie>, data: Res<GameData>, tick: Res<Tick>) {
    let Some((_, numbers)) = chosen(&sortie, &data) else {
        return;
    };
    // The circle the city generator keeps free, so nothing spawns inside a house. Loud when
    // there is no map: a silent 0.0 would stack every titan of every wave on the origin, on
    // top of the player, and that reads as a bug in the spawner rather than a missing map.
    let ring_m = match data.current_map() {
        Some(map) => map.layout.clear_radius_m,
        None => {
            error!(
                "maps.ron: current = {:?} is not in `maps` — the wave ring has no radius and \
                 every titan would spawn on the origin",
                data.maps.current
            );
            return;
        }
    };
    let schedule = WaveSchedule::of(numbers.waves, tick.0, data.game.simulation_hz, ring_m);
    info!(
        "mission active at tick {}: {} titan(s) queued on a {:.1} m ring",
        tick.0,
        schedule.pending.len(),
        ring_m
    );
    commands.spawn((schedule, DespawnOnExit(MissionPhase::Active)));
}

/// One line in the log when the verdict falls. It is what a `--script` run leaves behind for a
/// human to line the screenshot up against.
fn announce(
    phase: Res<State<MissionPhase>>,
    tick: Res<Tick>,
    missions: Query<(&MissionClock, &KillTally)>,
) {
    for (clock, tally) in &missions {
        info!(
            "MISSION {} at tick {} (decided at {:?}) — {}/{} kills",
            phase.get().label(),
            tick.0,
            clock.decided_at_tick,
            tally.total(),
            tally.target
        );
    }
}

/// The order that is being flown, and the numbers it flies.
///
/// `None` means: nothing was ordered, or the file does not know the template or the difficulty
/// — [`begin_mission`] and [`hub::deploy_on_contact`] have both already said so loudly, and a
/// half-built mission with a stand-in duration would be worse than none.
fn chosen<'a>(
    sortie: &'a Sortie,
    data: &'a GameData,
) -> Option<(&'a SortieOrder, SortieNumbers<'a>)> {
    let order = sortie.0.as_ref()?;
    let template = data.missions.templates.get(&order.template)?;
    let numbers = resolve(template, order.difficulty.as_deref())?;
    Some((order, numbers))
}

/// Writes a [`SpawnTitan`] for every titan whose tick has come.
fn release_due_waves(
    tick: Res<Tick>,
    mut schedules: Query<&mut WaveSchedule>,
    mut spawn: MessageWriter<SpawnTitan>,
) {
    for mut schedule in &mut schedules {
        for titan in schedule.take_due(tick.0) {
            info!("wave: {} at {:?} (tick {})", titan.kind, titan.pos, tick.0);
            spawn.write(SpawnTitan {
                kind: titan.kind,
                pos_x: titan.pos.x,
                pos_y: titan.pos.y,
                pos_z: titan.pos.z,
            });
        }
    }
}

/// Books cortex kills onto the players who made them.
///
/// **Only `HitZone::Cortex`.** Counting `TitanHit` messages as such would let a torso hit win
/// the mission, and that is exactly the bug `tests/mission.rs::
/// f071_the_last_kill_and_not_the_first_wins_the_mission` goes red on. And **only once per
/// titan**: a body that is already dissolving keeps its `TitanId` for `death_s`.
fn count_kills(mut hits: MessageReader<TitanHit>, mut tallies: Query<&mut KillTally>) {
    for hit in hits.read() {
        if hit.zone != HitZone::Cortex {
            continue;
        }
        for mut tally in &mut tallies {
            if tally.credit(hit.by, hit.titan) {
                info!(
                    "kill: player {} cut titan {} — {}/{}",
                    hit.by.0,
                    hit.titan.0,
                    tally.total(),
                    tally.target
                );
            }
        }
    }
}

/// The verdict. **The win is checked before the loss**, so a kill on the deadline tick still
/// wins — the other order would take a mission away from somebody who earned it by one tick.
///
/// The second way to lose reads [`Health`] on the players. Nothing writes that component yet
/// (`P5`, `docs/PLAN-GAME.md` §5), so the query is empty today and this branch is inert — it is
/// written now because `docs/PLAN-GAME.md` §1 says "two ways to lose", and an empty query is a
/// far more honest placeholder than a `todo!()` nobody removes.
fn decide(
    tick: Res<Tick>,
    mut next: ResMut<NextState<MissionPhase>>,
    mut missions: Query<(&mut MissionClock, &KillTally)>,
    players: Query<&Health, With<PlayerId>>,
) {
    for (mut clock, tally) in &mut missions {
        if clock.decided_at_tick.is_some() {
            continue;
        }
        let everybody_down = !players.is_empty() && players.iter().all(|h| h.current <= 0.0);
        let verdict = if tally.reached() {
            MissionPhase::Won
        } else if clock.expired(tick.0) || everybody_down {
            MissionPhase::Lost
        } else {
            continue;
        };
        clock.decided_at_tick = Some(tick.0);
        next.set(verdict);
    }
}
