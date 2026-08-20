//! The guard over the mission — `F-070`, `F-071`.
//!
//! A mission state machine has four ways of being wrong that **look right in review and pass a
//! loose test**, and each of them has a test here:
//!
//! 1. **The timer is a wall clock.** `Time::delta_secs()` accumulated into a float fires at a
//!    tick that depends on the frame rate. "The mission eventually ends" still passes; ±1 on a
//!    named tick does not ([`f070_the_timeout_loses_the_mission_at_the_tick_the_file_says`]).
//! 2. **The duration is a Rust constant.** Nothing errors, the mission just stops listening to
//!    `missions.ron` ([`f070_the_deadline_follows_the_file_and_not_a_literal`], which puts
//!    10 s into the loaded data and demands 600 ticks).
//! 3. **The win check asks `titans == 0`.** This is the dangerous one: it *looks* right, and it
//!    produces an instant, silent win at tick 0 — before a single wave has spawned — which
//!    then reads as a bug in the spawner
//!    ([`f071_an_empty_field_before_the_first_wave_is_not_a_win`]).
//! 4. **The win check counts `TitanHit` messages.** A torso hit then wins the mission
//!    ([`f071_the_last_kill_and_not_the_first_wins_the_mission`], which sends four non-cortex
//!    hits through the same run — without them the test would prove nothing).
//!
//! Every number these tests compare against comes out of `assets/data/`, never out of this
//! file — except where a literal *is* the claim (19 800 = 330 s × 60 Hz), and there it stands
//! next to the value it is derived from.
//!
//! ## Why the recorder sits in `SimulationSystems::Drive`
//!
//! It samples the phase **before** `PostStep`, that is before the wave spawner and before the
//! verdict of this tick. That is exactly what
//! [`f071_an_empty_field_before_the_first_wave_is_not_a_win`] needs: at tick 1 the mission is
//! already `Active` and the field is still empty, because the first wave's titan comes into
//! being at the end of that very tick.
//!
//! ## The one-tick offset, and why the criterion says ±1
//!
//! A `NextState` set in `FixedUpdate` is applied by the `StateTransition` schedule, which runs
//! once per frame after `PreUpdate` (`bevy_state-0.19.0/src/app.rs:335`). So a verdict decided
//! at tick *n* is readable as `State<MissionPhase>` from tick *n+1* on. The decision tick is
//! pinned down exactly (`MissionClock::decided_at_tick`), the observation within ±1.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::mission::{KillTally, Mission, MissionClock, MissionPhase, WaveSchedule};
use defeated_by_titan::shared::{
    Cli, Health, HitZone, PlayerId, SimulationSystems, SpawnTitan, Tick, TitanHit, TitanId,
};

// ---------------------------------------------------------------------------
// the harness
// ---------------------------------------------------------------------------

/// One sample per simulation tick, taken before the consequences of that tick.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    tick: u64,
    phase: MissionPhase,
    titans: usize,
    kills: u32,
}

#[derive(Resource, Default)]
struct Log(Vec<Sample>);

fn record(
    mut log: ResMut<Log>,
    tick: Res<Tick>,
    phase: Res<State<MissionPhase>>,
    titans: Query<&TitanId>,
    tallies: Query<&KillTally>,
) {
    log.0.push(Sample {
        tick: tick.0,
        phase: *phase.get(),
        titans: titans.iter().count(),
        kills: tallies.iter().next().map_or(0, |t| t.total()),
    });
}

/// The **real** app, headless, one simulation step per `update()`. Not started yet: the caller
/// may still change `GameData` before `Startup` reads it.
fn built(mission: Option<&str>) -> App {
    built_from(Cli {
        headless: true,
        mission: mission.map(|s| s.to_string()),
        ..default()
    })
}

fn built_from(start: Cli) -> App {
    let mut app = defeated_by_titan::app(start);
    // avian takes its step size from the generic `Time`, which only `run_fixed_main_schedule`
    // switches over to `Time<Fixed>`. One `update()` is then exactly one simulation step, on
    // every machine — the same reasoning as in `tests/titan.rs`.
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<Log>();
    app.add_systems(FixedUpdate, record.in_set(SimulationSystems::Drive));
    app
}

/// A started app: `Startup` has run, the mission stands, **and tick 1 has been recorded**.
///
/// The loop is not decoration. The first `update()` is the `Startup` frame and `Time` carries
/// no delta into it, so `RunFixedMainLoop` runs zero fixed steps — measured, not assumed. A
/// hard `app.update()` here would leave the log empty and every "at tick 1" assertion below
/// would fail on an off-by-one that has nothing to do with the mission.
fn started(mission: Option<&str>) -> App {
    let mut app = built(mission);
    for _ in 0..4 {
        app.update();
        if !app.world().resource::<Log>().0.is_empty() {
            return app;
        }
    }
    panic!("four frames and not one simulation tick — the fixed step is not running");
}

/// A started app that is standing **in the hub** — `--hub`, and the first tick recorded.
fn in_the_hub() -> App {
    let mut app = built_from(Cli { headless: true, hub: true, ..default() });
    for _ in 0..4 {
        app.update();
        if !app.world().resource::<Log>().0.is_empty() {
            return app;
        }
    }
    panic!("four frames and not one simulation tick — the fixed step is not running");
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

fn phase(app: &App) -> MissionPhase {
    *app.world().resource::<State<MissionPhase>>().get()
}

fn log(app: &App) -> Vec<Sample> {
    app.world().resource::<Log>().0.clone()
}

/// The tick at which the recorder first saw a phase other than the given one.
fn first_tick_leaving(app: &App, from: MissionPhase) -> Option<Sample> {
    log(app).into_iter().find(|s| s.phase != from)
}

fn clock(app: &mut App) -> MissionClock {
    let mut q = app.world_mut().query::<&MissionClock>();
    *q.iter(app.world()).next().expect("no mission entity — `--mission` did not deploy anything")
}

fn tally(app: &mut App) -> KillTally {
    let mut q = app.world_mut().query::<&KillTally>();
    q.iter(app.world()).next().expect("no kill counter on the mission").clone()
}

fn titan_ids(app: &mut App) -> Vec<TitanId> {
    let mut q = app.world_mut().query::<&TitanId>();
    let mut ids: Vec<TitanId> = q.iter(app.world()).copied().collect();
    ids.sort_unstable();
    ids
}

/// The local player's id. Never `.single()` on a player query — `tests/multiplayer.rs` is the
/// guard, and this file is not going to be the exception.
fn a_player(app: &mut App) -> PlayerId {
    let mut q = app.world_mut().query::<&PlayerId>();
    *q.iter(app.world()).next().expect("no player in the world")
}

/// Asks for a titan and lets it come into being. Two ticks: the spawner reads the message in
/// `PostStep`, so the entity exists at the end of the first one.
fn spawn_titan(app: &mut App, kind: &str, pos: Vec3) {
    app.world_mut().write_message(SpawnTitan {
        kind: kind.to_string(),
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
    });
    ticks(app, 2);
}

/// Puts every player so far outside every titan's `aggro_radius_m` that no wave can walk to
/// him, and hands back the distance it used.
///
/// **Why the long tests need this.** Since `P5` a husk's `Strike` really does subtract
/// `titan.ron: husk.damage` (34) from `game.ron: player.health` (100), and `mission::decide`'s
/// second loss path ("every player down ⇒ `Lost`") is no longer inert. A test player who stands
/// still in the middle of the spawn ring is down after three blows, and every test whose
/// subject is a **tick far in the future** — the 19 800 of the deadline, the 5 400 of the second
/// wave — would then measure the walking speed of a husk instead of the thing it was written
/// for. Parking him is not a weakening: the criterion below is unchanged, it is the *premise*
/// ("nothing else decides this mission") that is being made true instead of assumed.
///
/// Every number here comes out of `assets/data/`: the spawn ring is `maps.ron:
/// layout.clear_radius_m` (the same one `mission::open_the_field` reads), and the reach of the
/// widest pair of eyes in the game is the largest `aggro_radius_m` of `titan.ron` — the maximum
/// over **all** kinds and not over the tutorial's two, so that a new wave in `missions.ron`
/// cannot quietly bring a titan that sees further. `titan::brain::decide` sends a titan outside
/// that radius to `Idle`, and `titan::brain::walk` moves nothing that is not in `Pursue`, so a
/// parked player is not chased at all — he is not merely out of reach for a while.
fn park_players_out_of_aggro(app: &mut App) -> f32 {
    let d = data(app);
    let ring_m = d
        .current_map()
        .expect("maps.ron: `current` names a map — the wave ring has no radius without it")
        .layout
        .clear_radius_m;
    let widest_aggro_m =
        d.titans.kinds.values().map(|k| k.aggro_radius_m).fold(0.0f32, f32::max);
    // Twenty metres of air on top of the largest aggro radius in the file, measured from the
    // far side of the spawn ring.
    let park_m = ring_m + widest_aggro_m + 20.0;
    // Still on the 400 x 400 m ground slab of `maps.ron`, or the player would fall out of the
    // world for 330 s and this would stop being "parked" and start being "falling".
    assert!(park_m < 200.0, "{park_m} m is off the ground slab — this is no longer a parking spot");

    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerId>>();
    let players: Vec<Entity> = q.iter(app.world()).collect();
    assert!(!players.is_empty(), "no player to park");
    for player in players {
        // The `Transform`, not `Position`: avian's `transform_to_position` is on by default
        // (`avian3d-0.7.0/src/physics_transform/mod.rs:161`) and takes the teleport over in
        // `PhysicsSystems::Prepare` — the same move `tests/combat.rs::place` makes.
        app.world_mut().entity_mut(player).insert(Transform::from_xyz(0.0, 2.0, park_m));
    }
    park_m
}

/// What every player has left of `game.ron: player.health`, lowest first. Empty when nobody
/// carries the component — which is a different answer from "nobody was hit".
fn healths(app: &mut App) -> Vec<f32> {
    let mut q = app.world_mut().query_filtered::<&Health, With<PlayerId>>();
    let mut left: Vec<f32> = q.iter(app.world()).map(|h| h.current).collect();
    left.sort_by(f32::total_cmp);
    left
}

fn hit(app: &mut App, titan: TitanId, by: PlayerId, zone: HitZone) {
    app.world_mut().write_message(TitanHit { titan, by, zone, speed_m_s: 30.0 });
    app.update();
}

// ---------------------------------------------------------------------------
// F-070 — the state machine
// ---------------------------------------------------------------------------

#[test]
fn f070_without_a_mission_flag_nothing_in_this_domain_happens() {
    // The state is registered in every launch mode, so `hud` and `debug` can always read it.
    // What must NOT happen is a mission running that nobody asked for.
    let mut app = started(None);
    ticks(&mut app, 5);
    assert_eq!(phase(&app), MissionPhase::Briefing);

    let mut missions = app.world_mut().query::<&Mission>();
    assert_eq!(missions.iter(app.world()).count(), 0, "a mission deployed without --mission");
    let mut waves = app.world_mut().query::<&WaveSchedule>();
    assert_eq!(waves.iter(app.world()).count(), 0, "waves were queued without --mission");
}

#[test]
fn f070_a_mission_name_the_file_does_not_know_starts_nothing() {
    // Loud in the log, and no half-built mission: a typo must not leave a clock running
    // against a template that does not exist.
    let mut app = started(Some("tutrial"));
    ticks(&mut app, 3);
    assert_eq!(phase(&app), MissionPhase::Briefing);
    let mut missions = app.world_mut().query::<&Mission>();
    assert_eq!(missions.iter(app.world()).count(), 0);
}

#[test]
fn f070_the_mission_is_active_before_the_first_tick() {
    // `Briefing → Deploying → Active` has to be done by tick 1. If the phases were walked one
    // `NextState` per frame, the mission would be `Active` only from tick 3 on — and every
    // criterion that names a tick would be off by an amount nobody wrote down.
    let mut app = started(Some("tutorial"));
    let d = data(&app);
    let template = &d.missions.templates["tutorial"];

    assert_eq!(phase(&app), MissionPhase::Active);
    assert_eq!(log(&app)[0].tick, 1);
    assert_eq!(log(&app)[0].phase, MissionPhase::Active, "not Active at the first tick");

    let clock = clock(&mut app);
    assert_eq!(clock.started_at_tick, 0, "the mission clock starts with the simulation");
    assert_eq!(
        clock.duration_ticks,
        (template.target_duration_s as f64 * d.game.simulation_hz).round() as u64
    );
    assert_eq!(tally(&mut app).target, template.kill_target, "the target comes out of the file");
}

#[test]
fn f070_the_deadline_follows_the_file_and_not_a_literal() {
    // ⭐ The half of `F-070` that catches a Rust constant. `missions.ron` says 330 s; this test
    // puts **10 s** into the loaded data before `Startup` reads it and demands 600 ticks. A
    // literal in the code keeps waiting for 19 800 and this goes red — which is exactly what
    // it is for.
    //
    // It does **not** catch the wall clock, and that was measured, not assumed: with a timer
    // that accumulates `Time::delta_secs()` this test still passes at 600 ticks, because f32
    // drift over 600 steps is smaller than one tick. Only the long test below catches it (it
    // lands at 19 804 instead of 19 800). That is the whole reason the expensive one exists.
    let mut app = built(Some("tutorial"));
    app.world_mut()
        .resource_mut::<GameData>()
        .missions
        .templates
        .get_mut("tutorial")
        .expect("the tutorial template")
        .target_duration_s = 10.0;

    let hz = data(&app).game.simulation_hz;
    let expected = (10.0 * hz).round() as u64;
    assert_eq!(expected, 600);

    ticks(&mut app, expected + 10);

    let clock = clock(&mut app);
    assert_eq!(clock.duration_ticks, expected, "the duration did not come out of the data");
    assert_eq!(clock.decided_at_tick, Some(expected), "decided at the wrong tick");
    assert_eq!(phase(&app), MissionPhase::Lost);

    // And the ±1 of the big test is **exactly one**, always, in this direction: the verdict is
    // decided in `FixedUpdate` at tick n and applied by the `StateTransition` schedule of the
    // next frame, so the recorder sees it at n+1. Pinned down here, where it costs a second,
    // so that the window in the 19 800 test is a known offset and not a tolerance for drift.
    let left = first_tick_leaving(&app, MissionPhase::Active).expect("it left Active");
    assert_eq!(left.tick, expected + 1, "the StateTransition offset is not one tick");
}

#[test]
fn f070_the_timeout_loses_the_mission_at_the_tick_the_file_says() {
    // ⭐ The criterion of `F-070`, at the real number. 0 kills, and the phase becomes `Lost` at
    // tick `target_duration_s × simulation_hz` = 19 800, ±1.
    //
    // **This is what catches the wall-clock timer** — measured, not hoped for: replace
    // `MissionClock::expired` with a `Time::delta_secs()` accumulator and this test reports
    // tick **19 804**, four ticks late, from f32 drift alone, even in a harness that feeds it
    // exactly one fixed step per frame. In a real run, where frames outnumber ticks four to
    // one, it is off by a great deal more. A loose "the mission eventually ends" test passes
    // either version and the code looks right in review; this one does not.
    //
    // The window is one tick wide and no wider, and it is an offset rather than a tolerance:
    // `NextState` is applied by the `StateTransition` schedule of the next frame — pinned down
    // exactly in the 600-tick test above.
    //
    // The player is parked outside every aggro radius first (`park_players_out_of_aggro`).
    // Since `P5` the tutorial's first husk walks over and downs a standing test player in three
    // strikes, and the mission is then lost at tick 630 — by the *other* loss path. That is the
    // second way to lose working, not the clock: its own test is
    // `f070_every_player_out_of_the_fight_is_the_second_way_to_lose`. The criterion here is
    // untouched, only its premise is now established instead of assumed — and the assertion
    // that nobody lost a single point of health says so out loud.
    let mut app = started(Some("tutorial"));
    let d = data(&app);
    let deadline =
        (d.missions.templates["tutorial"].target_duration_s as f64 * d.game.simulation_hz).round()
            as u64;
    assert_eq!(deadline, 19_800, "330 s at 60 Hz — if this moved, the file moved");
    park_players_out_of_aggro(&mut app);

    ticks(&mut app, deadline + 4);

    assert_eq!(
        healths(&mut app),
        vec![d.game.player.health],
        "somebody was hit — the parking failed and this run measures a husk, not the clock"
    );

    let left = first_tick_leaving(&app, MissionPhase::Active)
        .expect("the mission never left Active — the clock does not run");
    assert_eq!(left.phase, MissionPhase::Lost, "a mission without kills must not be won");
    assert_eq!(left.kills, 0, "nobody cut anything in this run");
    assert!(
        left.tick.abs_diff(deadline) <= 1,
        "the phase became Lost at tick {}, not at {deadline} ±1",
        left.tick
    );
    assert_eq!(
        clock(&mut app).decided_at_tick,
        Some(deadline),
        "the verdict was decided on the wrong tick"
    );
    assert_eq!(phase(&app), MissionPhase::Lost);
}

#[test]
fn f070_every_player_out_of_the_fight_is_the_second_way_to_lose() {
    // `docs/PLAN-GAME.md` §1: one way to win, **two** ways to lose. The test writes the
    // component itself and takes it to zero in one step, so that the branch is measured on its
    // own rather than through a husk's three strikes — `tests/combat.rs::
    // p5_the_mission_is_lost_when_every_player_is_down` is the end-to-end half. (Until `P5`
    // nothing wrote player `Health` at all and this was the only test the branch had.)
    let mut app = started(Some("tutorial"));
    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerId>>();
    let players: Vec<Entity> = q.iter(app.world()).collect();
    assert!(!players.is_empty(), "no player to knock down");

    for player in &players {
        app.world_mut().entity_mut(*player).insert(Health::full(100.0));
    }
    ticks(&mut app, 2);
    assert_eq!(phase(&app), MissionPhase::Active, "a healthy player must not lose the mission");

    for player in &players {
        app.world_mut().entity_mut(*player).insert(Health { current: 0.0, max: 100.0 });
    }
    ticks(&mut app, 3);
    assert_eq!(phase(&app), MissionPhase::Lost);
    assert!(
        clock(&mut app).decided_at_tick.expect("decided") < 19_800,
        "this must not be the timeout"
    );
}

// ---------------------------------------------------------------------------
// F-071 — the skirmish
// ---------------------------------------------------------------------------

#[test]
fn f071_an_empty_field_before_the_first_wave_is_not_a_win() {
    // ⭐ **The more dangerous of the two criteria, because it looks right.** A win check that
    // asks `titans == 0` is true at tick 0 — before a single wave has spawned — and hands out
    // an instant, silent victory that then reads as a bug in the spawner.
    //
    // The recorder sits in `SimulationSystems::Drive`, so it samples before `PostStep`: at
    // tick 1 the mission is `Active` and the field is provably empty, because the first wave's
    // titan comes into being at the end of that tick.
    let mut app = started(Some("tutorial"));
    let first = log(&app)[0];
    assert_eq!(first.tick, 1);
    assert_eq!(first.titans, 0, "the premise of this test: nothing has spawned yet");
    assert_eq!(first.phase, MissionPhase::Active, "an empty field won the mission at tick 1");
    assert_eq!(first.kills, 0);

    // And it stays that way while the field fills up: nothing here is won by counting bodies.
    //
    // The window matters. A verdict decided at tick 1 is only *readable* from tick 2 on
    // (`StateTransition` runs once per frame), so "Active at tick 1" alone cannot catch a win
    // at tick 1 — it would sample one tick too early. What catches it is that **no sample in
    // the whole opening window is a verdict**, and that is asserted over every one of them.
    ticks(&mut app, 30);
    assert_eq!(phase(&app), MissionPhase::Active);
    assert_eq!(tally(&mut app).total(), 0);
    let decided: Vec<Sample> = log(&app).into_iter().filter(|s| s.phase.is_decided()).collect();
    assert!(
        decided.is_empty(),
        "the mission was decided while nobody had cut anything: {decided:?}"
    );
}

#[test]
fn f071_the_last_kill_and_not_the_first_wins_the_mission() {
    // ⭐ 3 titans, `kill_target` out of the file. `Active` after kills 1 and 2, `Won` on kill 3,
    // and the counter reads 1, 2, 3 on the way.
    //
    // **Four non-cortex hits go through the same run.** Without them this test would prove
    // nothing: a win check that counted `TitanHit` messages instead of cortex kills would pass
    // a run that only ever sends cortex hits.
    let mut app = started(Some("tutorial"));
    let target = tally(&mut app).target;
    assert_eq!(target, 3, "missions.ron: tutorial.kill_target");
    let player = a_player(&mut app);

    // Three of our own, on top of whatever the first wave brought. The two ticks are what lets
    // the 0 s wave arrive first — otherwise its husk lands in `mine` and this test kills four
    // titans while claiming to kill three.
    ticks(&mut app, 2);
    let before = titan_ids(&mut app);
    assert!(!before.is_empty(), "the 0 s wave has not arrived, so `before` proves nothing");
    for i in 0..3 {
        spawn_titan(&mut app, "husk", Vec3::new(30.0 + i as f32 * 10.0, 0.0, -40.0));
    }
    let mine: Vec<TitanId> =
        titan_ids(&mut app).into_iter().filter(|id| !before.contains(id)).collect();
    assert_eq!(mine.len(), 3, "three titans of our own were asked for");

    // Every zone but the cortex. A win check counting messages is won right here.
    let mut non_cortex = 0;
    for zone in [HitZone::Torso, HitZone::ArmLeft, HitZone::LegRight, HitZone::Head] {
        hit(&mut app, mine[0], player, zone);
        non_cortex += 1;
    }
    assert_eq!(non_cortex, 4, "the number that makes the assertion below mean something");
    assert_eq!(tally(&mut app).total(), 0, "a torso hit is not a kill");
    assert_eq!(phase(&app), MissionPhase::Active, "a torso hit won the mission");

    // And now the cortex, one at a time.
    for (i, titan) in mine.iter().enumerate() {
        hit(&mut app, *titan, player, HitZone::Cortex);
        let counted = i as u32 + 1;
        assert_eq!(tally(&mut app).total(), counted, "kill {counted} was not counted");
        assert_eq!(tally(&mut app).of(player), counted, "the credit went to nobody");
        if counted < target {
            assert_eq!(
                phase(&app),
                MissionPhase::Active,
                "the mission was won on kill {counted} of {target}"
            );
        }
    }

    ticks(&mut app, 2);
    assert_eq!(phase(&app), MissionPhase::Won);
    assert_eq!(tally(&mut app).total(), 3);
    let decided = clock(&mut app).decided_at_tick.expect("a won mission has a decision tick");
    assert!(decided < 19_800, "this must not be the timeout dressed up as a win");
}

#[test]
fn f071_a_dissolving_titan_is_not_a_second_kill() {
    // ⚠️ Measured this session: a titan **keeps its `TitanId` for `death_s`** (1.0 s for a
    // husk). So a second cortex hit on a body that is already dying can arrive, and a counter
    // that took it would report 4/3 kills off three titans — or win a mission off one.
    let mut app = started(Some("tutorial"));
    let player = a_player(&mut app);
    let before = titan_ids(&mut app);
    spawn_titan(&mut app, "husk", Vec3::new(30.0, 0.0, -40.0));
    let mine = *titan_ids(&mut app)
        .iter()
        .find(|id| !before.contains(id))
        .expect("the titan we just asked for");

    hit(&mut app, mine, player, HitZone::Cortex);
    assert_eq!(tally(&mut app).total(), 1);
    // The body is still there — that is the trap.
    assert!(titan_ids(&mut app).contains(&mine), "the id is gone, so the trap is not reproduced");
    for _ in 0..3 {
        hit(&mut app, mine, player, HitZone::Cortex);
    }
    assert_eq!(tally(&mut app).total(), 1, "the same titan was paid for more than once");
    assert_eq!(phase(&app), MissionPhase::Active);
}

#[test]
fn f071_no_wave_walks_into_a_decided_mission() {
    // What `DespawnOnExit(MissionPhase::Active)` is for. The tutorial queues waves at 0 s, 90 s
    // and 210 s; a mission won at 3 s must not get a titan at 90 s. Without the scoped entity
    // the schedule survives the verdict and the spawn reads as a bug in `titan`.
    let mut app = started(Some("tutorial"));
    let player = a_player(&mut app);
    ticks(&mut app, 2);
    let before = titan_ids(&mut app);
    for i in 0..3 {
        spawn_titan(&mut app, "husk", Vec3::new(30.0 + i as f32 * 10.0, 0.0, -40.0));
    }
    let mine: Vec<TitanId> =
        titan_ids(&mut app).into_iter().filter(|id| !before.contains(id)).collect();
    assert_eq!(mine.len(), 3);
    for titan in &mine {
        hit(&mut app, *titan, player, HitZone::Cortex);
    }
    ticks(&mut app, 2);
    assert_eq!(phase(&app), MissionPhase::Won);

    let mut waves = app.world_mut().query::<&WaveSchedule>();
    assert_eq!(
        waves.iter(app.world()).count(),
        0,
        "the wave schedule survived the verdict — DespawnOnExit is not doing its job"
    );

    // Past the 90 s wave (5 400 ticks), and nothing NEW arrives. Counting bodies would be the
    // wrong measure: the three we cut dissolve away in the meantime, so the number goes down on
    // its own. `IdCounter` hands out ids in order, so "no id above the highest one we have
    // seen" is the question that actually means "nothing spawned".
    //
    // ⚠️ The watermark is taken from the ids this test **created**, not from the bodies still
    // standing after the verdict. It used to read `titan_ids(...).last()`, which quietly assumed
    // a titan outlives his own sortie — and on 2026-08-12 `titan::spawn_titan` gained
    // `DespawnOnExit(MissionPhase::Active)`, the field emptied at the verdict, and this test
    // panicked with "bodies are still standing" on a claim that has nothing to do with waves
    // (`docs/FINDINGS.md` FIND-066 §3). `mine` holds the three highest ids in the game either
    // way, so the question survives whichever lifetime a body ends up having.
    let highest = *mine.iter().max().expect("this test spawned three titans");
    ticks(&mut app, 5_500);
    let late: Vec<TitanId> =
        titan_ids(&mut app).into_iter().filter(|id| *id > highest).collect();
    assert!(late.is_empty(), "a wave spawned into a mission that was already over: {late:?}");
}

#[test]
fn f071_the_waves_come_out_of_the_file_at_the_ticks_the_file_says() {
    // The waves are file work: three rows in `missions.ron` become titans at 0 s, 90 s and
    // 210 s. Only the first two are checked here — 210 s is 12 600 ticks, and this test does
    // not need to be four minutes long to show that the conversion is real.
    //
    // The player is parked outside every aggro radius, for the same reason as in the deadline
    // test above and with a sharper consequence here: since `P5` the first husk downs a
    // standing test player at tick 630, the mission is `Lost`, and `WaveSchedule` carries
    // `DespawnOnExit(MissionPhase::Active)` — so the 5 400-tick wave has no schedule left to
    // come out of and this test would report "the second wave did not come" for a reason that
    // has nothing to do with the file. That the schedule may not survive a verdict is a claim
    // of its own and keeps its own test (`f071_no_wave_walks_into_a_decided_mission`).
    let mut app = started(Some("tutorial"));
    park_players_out_of_aggro(&mut app);
    let d = data(&app);
    let template = &d.missions.templates["tutorial"];
    let hz = d.game.simulation_hz;

    let first: u32 = template.waves.iter().filter(|w| w.at_s == 0.0).map(|w| w.count).sum();
    ticks(&mut app, 2);
    assert_eq!(titan_ids(&mut app).len(), first as usize, "the 0 s wave did not come");

    let second_at = (template.waves[1].at_s as f64 * hz).round() as u64;
    assert_eq!(second_at, 5_400, "90 s at 60 Hz");
    ticks(&mut app, second_at - 10);
    let before = titan_ids(&mut app).len();
    assert_eq!(before, first as usize, "the second wave came early");
    // The premise of the second half: the mission is still running, so there is still a
    // schedule to release from. Without it a decided mission would read as "the wave did not
    // come out of the file" three assertions later.
    assert_eq!(
        phase(&app),
        MissionPhase::Active,
        "the mission was decided before its second wave — this test then measures nothing"
    );
    ticks(&mut app, 20);
    assert_eq!(
        titan_ids(&mut app).len(),
        (first + template.waves[1].count) as usize,
        "the second wave did not come at its tick"
    );
}

// ---------------------------------------------------------------------------
// The hub — the loop the game did not have (user, 2026-08-12)
// ---------------------------------------------------------------------------
//
// Four ways this feature can be wrong while every one of the tests above stays green, and one
// test here for each:
//
// 1. **The pad triggers on nothing.** A trigger volume that fires from anywhere (or from
//    nowhere) is the difference between a door and a teleport
//    (`f072_a_player_beside_the_pad_starts_nothing`).
// 2. **The difficulty is decoration.** The pad names `elite` and the sortie flies the
//    template's own three numbers — everything *looks* right, the mission is simply the wrong
//    one (`f072_the_difficulty_and_not_the_template_decides_the_numbers`).
// 3. **The way back leaks into the drop-in.** `Won`/`Lost` returning to the hub for a mission
//    that was started with `--mission <name>` silently breaks `scripts/f070-lost.txt`,
//    `scripts/game-full.txt` and `tests/combat.rs`, which all read the verdict long after it
//    fell (`f072_a_sortie_that_came_from_nowhere_stays_on_its_verdict`).
// 4. **The refill is a timer again.** Gas that comes back anywhere but at a station is exactly
//    the assumption the user threw out on 2026-08-12 (`docs/QUESTIONS.md` Q-033,
//    `f072_gas_comes_back_at_a_station_and_nowhere_else`).

use defeated_by_titan::mission::{DeploymentPoint, RefuelStation, ReturnToHub};
use defeated_by_titan::shared::{BladeRestockRequest, Blades, Gas, RefuelRequest};

/// The hub's layout out of `missions.ron` — never a literal in this file.
fn hub_layout(app: &App) -> defeated_by_titan::data::HubLayout {
    app.world().resource::<GameData>().missions.hub.clone()
}

fn pads(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&DeploymentPoint>();
    q.iter(app.world()).count()
}

fn stations(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&RefuelStation>();
    q.iter(app.world()).count()
}

/// Puts every player exactly there. The `Transform`, not `Position` — avian takes the teleport
/// over in `PhysicsSystems::Prepare`, the same move `park_players_out_of_aggro` makes.
fn place_players(app: &mut App, at: Vec3) {
    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerId>>();
    let players: Vec<Entity> = q.iter(app.world()).collect();
    assert!(!players.is_empty(), "no player to place");
    for player in players {
        app.world_mut().entity_mut(player).insert(Transform::from_translation(at));
    }
}

/// Asserts that a spot is inside **no** trigger of the hub — no pad, no station — and hands it
/// back. The layout is data and it moves; a test that says "away from everything" and means a
/// coordinate somebody typed in once is a test that goes green for the wrong reason.
fn nowhere_in_particular(app: &App, at: Vec3) -> Vec3 {
    let hub = hub_layout(app);
    let range = data(app).gear.resupply.range_m;
    for pad in &hub.deployments {
        let d = at.distance(Vec3::from(pad.center_m));
        assert!(
            d > pad.radius_m,
            "{at:?} is {d:.1} m from the {:?} pad of radius {} — that is not 'outside'",
            pad.difficulty,
            pad.radius_m
        );
    }
    for station in &hub.refuel_stations {
        let d = at.distance(Vec3::from(station.center_m));
        assert!(d > range, "{at:?} is {d:.1} m from a station of range {range}");
    }
    at
}

/// The center of the pad that offers this difficulty, out of the file.
fn pad_center(app: &App, difficulty: &str) -> Vec3 {
    let hub = hub_layout(app);
    let pad = hub
        .deployments
        .iter()
        .find(|p| p.difficulty == difficulty)
        .unwrap_or_else(|| panic!("missions.ron: hub has no {difficulty:?} pad"));
    Vec3::from(pad.center_m)
}

fn player_pos(app: &mut App) -> Vec3 {
    let mut q = app.world_mut().query_filtered::<&Transform, With<PlayerId>>();
    q.iter(app.world()).next().expect("no player").translation
}

fn gas_left(app: &mut App) -> f32 {
    let mut q = app.world_mut().query_filtered::<&Gas, With<PlayerId>>();
    q.iter(app.world()).next().expect("no player carries a tank").current
}

/// Empties every tank, so that a refill is visible as a rise and not as "still full".
fn drain_tanks(app: &mut App) {
    set_tanks(app, 0.0);
}

/// Puts every tank at `current`. Used to make a **small** refill deficit: since `gas_tank` is
/// 15000 (Q-046) a test that wants to watch a tank reach the top cannot afford to start it at
/// zero — 40 gas/s takes 375 s to cross that — and the clamp it wants to measure does not care
/// how deep the hole was.
fn set_tanks(app: &mut App, current: f32) {
    let mut q = app.world_mut().query_filtered::<&mut Gas, With<PlayerId>>();
    for mut gas in q.iter_mut(app.world_mut()) {
        gas.current = current;
    }
}

#[test]
fn f072_the_hub_is_a_place_that_stands_there_before_the_first_tick() {
    // `--hub` has to be as immediate as `--mission`: pads in the world at tick 1, or the first
    // thing a player does is walk through a door that is not built yet.
    let mut app = in_the_hub();
    assert_eq!(phase(&app), MissionPhase::Hub);
    assert_eq!(log(&app)[0].phase, MissionPhase::Hub, "not in the hub at the first tick");

    let hub = hub_layout(&app);
    assert_eq!(pads(&mut app), hub.deployments.len(), "the doors come out of missions.ron");
    assert_eq!(stations(&mut app), hub.refuel_stations.len(), "and so do the stations");
    assert!(hub.deployments.len() >= 3, "the user asked for difficulty levels, and there are {} doors", hub.deployments.len());

    // And no sortie: standing in the hub is not being in a mission.
    let mut missions = app.world_mut().query::<&Mission>();
    assert_eq!(missions.iter(app.world()).count(), 0, "a mission deployed in the hub");
}

#[test]
fn f072_a_player_beside_the_pad_starts_nothing() {
    // ⭐ The half that makes the other half mean something. Without it "walking onto the pad
    // deploys" is satisfied by a system that deploys on tick 1 from anywhere in the world.
    let mut app = in_the_hub();
    let center = pad_center(&app, "recruit");
    let hub = hub_layout(&app);
    let radius = hub.deployments[0].radius_m;

    // One metre outside the circle, and outside every other trigger in the hub — the tightest
    // miss the layout allows, because "he was thirty metres away" is a claim about nothing.
    let beside = nowhere_in_particular(&app, center - Vec3::Z * (radius + 1.0));
    place_players(&mut app, beside);
    ticks(&mut app, 30);
    assert_eq!(
        phase(&app),
        MissionPhase::Hub,
        "a player 1 m outside a pad of radius {radius} m started a sortie"
    );
    let mut missions = app.world_mut().query::<&Mission>();
    assert_eq!(missions.iter(app.world()).count(), 0);
}

#[test]
fn f072_walking_onto_a_pad_deploys_that_pads_sortie() {
    let mut app = in_the_hub();
    let center = pad_center(&app, "recruit");
    place_players(&mut app, center);
    ticks(&mut app, 10);

    assert_eq!(phase(&app), MissionPhase::Active, "the pad did not start a sortie");
    let mut q = app.world_mut().query::<&Mission>();
    let mission = q.iter(app.world()).next().expect("no mission entity").clone();
    assert_eq!(mission.template, "skirmish", "the pad's mission, not somebody else's");

    // The numbers are the difficulty's, read back out of the file rather than written here.
    let d = data(&app);
    let level = &d.missions.templates["skirmish"].difficulties["recruit"];
    assert_eq!(tally(&mut app).target, level.kill_target);
    assert_eq!(
        clock(&mut app).duration_ticks,
        (level.target_duration_s as f64 * d.game.simulation_hz).round() as u64
    );
    assert!(mission.name.contains(&level.name), "the name says which level is being flown: {:?}", mission.name);

    // The pads are gone with the hub — a deployment pad standing in the middle of a fight would
    // deploy you again on the way past it.
    assert_eq!(pads(&mut app), 0, "the hub's furniture survived the deployment");
    assert_eq!(stations(&mut app), 0, "a refuel station survived into the mission — Q-033");
}

#[test]
fn f072_the_difficulty_and_not_the_template_decides_the_numbers() {
    // ⭐ The one that catches a difficulty that is only a label. `elite` and the template must
    // disagree in the file, or this test proves nothing — which is asserted first.
    let mut app = in_the_hub();
    let d = data(&app);
    let template = &d.missions.templates["skirmish"];
    let elite = &template.difficulties["elite"];
    assert_ne!(
        elite.kill_target, template.kill_target,
        "missions.ron: `elite` and the template's own kill_target are the same number — this \
         test cannot tell the two apart"
    );

    let elite_pad = pad_center(&app, "elite");
    place_players(&mut app, elite_pad);
    ticks(&mut app, 10);

    assert_eq!(tally(&mut app).target, elite.kill_target, "the template's target was flown");
    assert_eq!(
        clock(&mut app).duration_ticks,
        (elite.target_duration_s as f64 * d.game.simulation_hz).round() as u64,
        "the template's deadline was flown"
    );
}

#[test]
fn f072_a_won_sortie_lands_back_in_the_hub_at_the_spawn_point() {
    // The whole ring in one test: hub → pad → Active → kills → Won → hub.
    let mut app = in_the_hub();
    let pad = pad_center(&app, "recruit");
    place_players(&mut app, pad);
    ticks(&mut app, 10);
    assert_eq!(phase(&app), MissionPhase::Active);

    let player = a_player(&mut app);
    let target = tally(&mut app).target;
    for i in 0..target {
        hit(&mut app, TitanId(100 + i as u32), player, HitZone::Cortex);
    }
    ticks(&mut app, 3);
    assert_eq!(phase(&app), MissionPhase::Won, "{target} cortex kills did not win it");

    // The verdict stays up for `hub.debrief_s` and not one tick less: a WON that is gone before
    // it can be read is a WON nobody saw.
    let hub = hub_layout(&app);
    let debrief = (hub.debrief_s as f64 * data(&app).game.simulation_hz).round() as u64;
    ticks(&mut app, debrief / 2);
    assert_eq!(phase(&app), MissionPhase::Won, "the debrief was cut short");

    ticks(&mut app, debrief / 2 + 5);
    assert_eq!(phase(&app), MissionPhase::Hub, "the sortie never came back to the hub");

    // And the hub is a hub again: furniture back, no mission entity left over, and the player
    // standing at the spawn point instead of on the pad he deployed from — which is what stops
    // the next tick from deploying him all over again.
    assert_eq!(pads(&mut app), hub.deployments.len());
    let mut missions = app.world_mut().query::<&Mission>();
    assert_eq!(
        missions.iter(app.world()).count(),
        0,
        "the finished mission is still there — a second sortie would run two kill counters"
    );
    ticks(&mut app, 5);
    let at = player_pos(&mut app);
    let landing = Vec3::from(hub.spawn_m);
    assert!(
        (at.x - landing.x).abs() < 2.0 && (at.z - landing.z).abs() < 2.0,
        "the player came back at {at:?} instead of at the hub's spawn point {landing:?}"
    );
    assert_eq!(phase(&app), MissionPhase::Hub, "he redeployed himself by standing still");
}

#[test]
fn f072_a_lost_sortie_comes_back_too() {
    // The second way out of a sortie. `P5`'s loss path (every player down) is used because the
    // recruit clock is 420 s = 25 200 ticks and a test that waits for it would measure the
    // walking speed of a husk instead of the way back.
    let mut app = in_the_hub();
    let pad = pad_center(&app, "recruit");
    place_players(&mut app, pad);
    ticks(&mut app, 10);
    assert_eq!(phase(&app), MissionPhase::Active);

    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerId>>();
    let players: Vec<Entity> = q.iter(app.world()).collect();
    for p in players {
        app.world_mut().entity_mut(p).insert(Health { current: 0.0, max: 100.0 });
    }
    ticks(&mut app, 3);
    assert_eq!(phase(&app), MissionPhase::Lost, "a squad that is down has not lost");

    let debrief = (hub_layout(&app).debrief_s as f64 * data(&app).game.simulation_hz).round() as u64;
    ticks(&mut app, debrief + 5);
    assert_eq!(phase(&app), MissionPhase::Hub, "a lost sortie is a dead end");
}

#[test]
fn f072_a_sortie_that_came_from_nowhere_stays_on_its_verdict() {
    // ⭐ The regression guard for three scripts and two tests that are not this job's to run:
    // `scripts/f070-lost.txt` reads the verdict 120 ticks after it falls and
    // `tests/combat.rs::p5_the_mission_is_lost_when_every_player_is_down` up to 250 ticks
    // after. A `--mission <name>` run that quietly walked into a hub would turn every one of
    // them red for a reason that has nothing to do with what they measure.
    let mut app = started(Some("tutorial"));
    let player = a_player(&mut app);
    let target = tally(&mut app).target;
    for i in 0..target {
        hit(&mut app, TitanId(200 + i as u32), player, HitZone::Cortex);
    }
    ticks(&mut app, 3);
    assert_eq!(phase(&app), MissionPhase::Won);

    let mut q = app.world_mut().query::<&ReturnToHub>();
    assert_eq!(q.iter(app.world()).count(), 0, "a drop-in mission was marked to return");

    let debrief = (hub_layout(&app).debrief_s as f64 * data(&app).game.simulation_hz).round() as u64;
    ticks(&mut app, debrief * 4);
    assert_eq!(
        phase(&app),
        MissionPhase::Won,
        "`--mission tutorial` walked into the hub — every script that reads the verdict after \
         the fact is now measuring the hub"
    );
}

#[test]
fn f072_gas_comes_back_at_a_station_and_nowhere_else() {
    // `docs/QUESTIONS.md` Q-033 as a test. Three claims in one run, because the third is the
    // only one that makes the first two more than "some system adds gas somewhere".
    let mut app = in_the_hub();
    let station = Vec3::from(hub_layout(&app).refuel_stations[0].center_m);

    // 1. Far away from every station: nothing comes back, ever. This is the assumption the user
    //    threw out — a tank that fills itself while you stand around.
    let outside = nowhere_in_particular(&app, Vec3::new(0.0, 2.0, -6.0));
    place_players(&mut app, outside);
    drain_tanks(&mut app);
    ticks(&mut app, 60);
    assert_eq!(gas_left(&mut app), 0.0, "the tank refilled itself away from any station");

    // 2. Standing in one: it comes back at `gear.ron: resupply.gas_per_s`.
    place_players(&mut app, station);
    ticks(&mut app, 30);
    let after = gas_left(&mut app);
    let expected = data(&app).gear.resupply.gas_per_s * 30.0 / data(&app).game.simulation_hz as f32;
    assert!(
        (after - expected).abs() < expected * 0.2,
        "30 ticks in a station gave {after} of the expected {expected}"
    );

    // 3. And never above the tank — the CLAMP in the refuel path, which is the claim that makes
    //    the first two more than "some system adds gas somewhere".
    //
    // ⚠️ **RE-CUT 2026-08-20 (Q-046), because filling a 15000 tank from empty is not a test.**
    // This used to drain the tank and then stand in the station for 1200 ticks, relying on
    // 40 gas/s x 20 s = 800 gas being more than the whole tank. That was only ever true because
    // the tank was 300; at 15000 it filled 5 % of it and the assert went red on a mechanism that
    // is perfectly fine. Topping a tank up at 40/s would need 22 500 ticks — 375 s of simulation
    // inside the round gate, to measure a `min`.
    //
    // So the deficit is made small instead of the run long: park the tank ten gas below full and
    // stand in the station far longer than the 0.25 s that costs. **Same claim, and a sharper
    // one** — it now fails if the station overshoots by more than 1e-3, at any tank size,
    // including after a rollback to `gas_tank: 300.0`.
    let max = data(&app).game.vector.gas_tank;
    set_tanks(&mut app, max - 10.0);
    ticks(&mut app, 120); // 2 s at 40 gas/s = 80 gas offered against a 10 gas hole
    let full = gas_left(&mut app);
    assert!(
        (full - max).abs() < 1e-3,
        "the station overfilled the tank: {full} of {max} (it was offered 80 gas into a 10 gas \
         hole and had to stop at the top)"
    );
}

#[test]
fn f072_a_station_is_a_hub_thing_and_does_not_follow_you_into_a_sortie() {
    // The other half of Q-033: refuelling is not "wherever a station used to be". Same spot,
    // same player, a running mission — and nothing comes back.
    //
    // ⚠️ Measured while breaking it: this is carried by **two** mechanisms — the `run_if` on
    // the system and `DespawnOnExit(MissionPhase::Hub)` on the station itself — and it only
    // goes red when both are taken away. Written down rather than left as a test that looks
    // sharper than it is.
    let mut app = in_the_hub();
    let station = Vec3::from(hub_layout(&app).refuel_stations[0].center_m);
    let pad = pad_center(&app, "recruit");
    place_players(&mut app, pad);
    ticks(&mut app, 10);
    assert_eq!(phase(&app), MissionPhase::Active, "this test needs a running sortie");

    place_players(&mut app, station);
    drain_tanks(&mut app);
    ticks(&mut app, 60);
    assert_eq!(
        gas_left(&mut app),
        0.0,
        "the tank refilled itself inside a mission, at the place a station stands in the hub"
    );
}

#[test]
fn f072_every_door_names_a_mission_and_a_difficulty_the_file_knows() {
    // A pad pointing at nothing is a door that does not open, and nothing in the game says so
    // until somebody walks into it. Plus the shape the user asked for: three levels, and every
    // one of them a real set of numbers.
    let d = data(&started(None));
    let hub = &d.missions.hub;
    for pad in &hub.deployments {
        let template = d
            .missions
            .templates
            .get(&pad.mission)
            .unwrap_or_else(|| panic!("hub pad names mission {:?}, which is not a template", pad.mission));
        let level = template.difficulties.get(&pad.difficulty).unwrap_or_else(|| {
            panic!("hub pad names difficulty {:?} of {:?}, which is not in the file", pad.difficulty, pad.mission)
        });
        assert!(pad.radius_m > 0.0, "a pad of radius {} cannot be walked into", pad.radius_m);
        assert!(level.kill_target > 0, "{:?} is won before it starts", pad.difficulty);
        assert!(
            (300.0..=420.0).contains(&level.target_duration_s),
            "{:?}: {} s — the bible wants a 5–7 min arc out of every level, not only out of the template",
            pad.difficulty,
            level.target_duration_s
        );
        for wave in &level.waves {
            assert!(
                d.titans.kinds.contains_key(&wave.kind),
                "{:?} wants {:?}, which is not in titan.ron",
                pad.difficulty,
                wave.kind
            );
            assert!(wave.count > 0, "{:?}: a wave with zero titans", pad.difficulty);
            assert!(
                wave.at_s <= level.target_duration_s,
                "{:?}: a wave at {} s in a {} s sortie",
                pad.difficulty,
                wave.at_s,
                level.target_duration_s
            );
        }
    }
    let levels: Vec<&String> = hub.deployments.iter().map(|p| &p.difficulty).collect();
    assert!(levels.len() >= 3, "the user asked for difficulty levels; the hub offers {levels:?}");
}

// ---------------------------------------------------------------------------
// One writer of `Gas` — the rule 4 repair of 2026-08-12 (FIND-063)
// ---------------------------------------------------------------------------

/// A bare app with **only the hub's station system in it** — no `vector` anywhere.
///
/// The point of building it by hand instead of using [`in_the_hub`] is that the real app
/// carries both halves, so it cannot tell "the hub asks and vector fills" from "the hub fills
/// itself". Here the applier is simply absent, and a tank that still rises can only have been
/// written by `mission`.
fn a_station_and_a_player(with_vector: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.add_message::<RefuelRequest>();
    app.add_systems(FixedUpdate, defeated_by_titan::mission::hub::refuel_at_stations);
    if with_vector {
        // Ordered by hand, because here there is no `SimulationSystems` to hang it on: in the
        // real app the applier runs in the NEXT tick's `Intent`, and this test is about who
        // writes the tank, not about when.
        app.add_systems(
            FixedUpdate,
            defeated_by_titan::vector::gas::apply_refuel_requests
                .after(defeated_by_titan::mission::hub::refuel_at_stations),
        );
    }

    app.world_mut().spawn((
        RefuelStation { radius_m: 4.0, gas_per_s: 40.0 },
        Transform::from_translation(Vec3::ZERO),
    ));
    let player = app
        .world_mut()
        .spawn((
            PlayerId(1),
            Transform::from_translation(Vec3::ZERO),
            // ⚠️ A DELIBERATELY ARBITRARY tank, not the shipped one. This fixture exists to
            // watch who WRITES `Gas`, not how big it is, so it must not be read as a mirror of
            // `game.ron: vector.gas_tank` (which is 15000 since 2026-08-20 — Q-046). Left as a
            // literal on purpose; the number is meaningless here and pulling the RON in would
            // couple a writer test to a tuning value for nothing.
            Gas { current: 0.0, ..Gas::full(300.0) },
        ))
        .id();
    (app, player)
}

fn tank(app: &App, player: Entity) -> f32 {
    app.world().entity(player).get::<Gas>().expect("the player carries a tank").current
}

#[test]
fn f072_a_station_asks_for_gas_and_never_writes_the_tank_itself() {
    // ⭐ The rule-4 test. `docs/architecture.md`'s authority table says `Gas` is written by
    // `vector` and by nothing else. So a player standing in a station of a game **without**
    // `vector::gas` must come out with the tank he went in with — the hub's job is to ask.
    //
    // Without this, "one writer" is a sentence in a doc: the second writer of 2026-08-12 was
    // invisible to every other test in this file, because they all run the whole app.
    let (mut app, player) = a_station_and_a_player(false);
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        tank(&app, player),
        0.0,
        "a second of standing in a station filled the tank with no `vector::gas` in the app — \
         `mission` is writing `Gas` itself (docs/architecture.md, authority table; rule 4)"
    );

    // The other half, or the first one proves only that nothing works: the SAME station and
    // the SAME player, with the applier added, do fill the tank. `40.0 gas/s` is the station's
    // own rate, 60 ticks are one second at the 60 Hz this app is built with.
    let (mut app, player) = a_station_and_a_player(true);
    for _ in 0..60 {
        app.update();
    }
    let after = tank(&app, player);
    assert!(
        (after - 40.0).abs() < 1.0,
        "a second in the station gave {after} of the station's own 40.0 gas/s — the request is \
         written but nothing applies it"
    );
}

// ---------------------------------------------------------------------------
// F-019 — the supply is INSIDE the building
// ---------------------------------------------------------------------------
//
// The user, 2026-08-12: „auch das main gebäude in dem der gas und schwert nachschub ist muss da
// sein (in das gebäude muss man rein laufen können. drinnen sind die nachschübe)". The building
// landed first and the stations stayed out on the pavement at y = 0.0, which is a hub that has a
// headquarters *and* a supply dump in front of it. The two tests below are what "drinnen" means
// as a number.

/// Every explicitly placed **solid** block of `maps.ron: current`, as `(min, max)` in meters.
///
/// The generated layout is not in here and does not need to be: the depot floor is an apron, and
/// `world` drops a generated house as soon as a placed block overlaps its lot (`maps.ron`, the
/// base slab's second job).
fn solid_boxes(d: &GameData) -> Vec<(Vec3, Vec3)> {
    let map = d.current_map().expect("maps.ron: current names a map that is not in the file");
    map.blocks
        .iter()
        .filter(|b| b.solid)
        .map(|b| {
            let c = Vec3::from(b.center_m);
            let half = Vec3::from(b.size_m) * 0.5;
            (c - half, c + half)
        })
        .collect()
}

/// The floor the supply stations stand on, as `(min, max)`.
///
/// Found by its **height and nothing else**: the one block of the map whose top face is exactly
/// at the stations' `center_m.1`. That is what makes "inside the building" a testable
/// coordinate instead of a description — `maps.ron` puts the depot floor of the garrison
/// headquarters at 0.15 m and gives no other block in the district that height (ground 0.0,
/// aprons 0.05, quays and bridges 0.4), so a station at that height stands on that floor and
/// nowhere else. `scripts/f019-hq.txt` reads the same number from the other side, as
/// `assert height > 0.10`.
fn floor_under_the_stations(d: &GameData) -> (Vec3, Vec3) {
    let stations = &d.missions.hub.refuel_stations;
    assert!(!stations.is_empty(), "the hub has no supply station at all");
    let y = stations[0].center_m.1;
    for station in stations {
        assert_eq!(
            station.center_m.1, y,
            "two supply stations at two heights — {:?} against {y}; they stand on one floor \
             or 'inside' is not one place",
            station.center_m
        );
    }
    let floors: Vec<(Vec3, Vec3)> = solid_boxes(d)
        .into_iter()
        .filter(|(_, max)| (max.y - y).abs() < 1e-4)
        .collect();
    assert_eq!(
        floors.len(),
        1,
        "the stations stand at y = {y}, and {} block(s) of maps.ron have their top face there. \
         Exactly one has to, or the height proves nothing about which building you are in",
        floors.len()
    );
    floors[0]
}

/// Can a 1.8 m player on a 0.35 m capsule stand here without being inside geometry?
///
/// The body is sampled from just above the floor to the top of the head, so the floor slab
/// itself is not what makes every spot in the hall unstandable.
fn standable(at: Vec3, d: &GameData, solids: &[(Vec3, Vec3)]) -> bool {
    let r = d.game.player.radius_m;
    let min = Vec3::new(at.x - r, at.y + 0.05, at.z - r);
    let max = Vec3::new(at.x + r, at.y + d.game.player.height_m, at.z + r);
    !solids
        .iter()
        .any(|(lo, hi)| min.x < hi.x && max.x > lo.x && min.y < hi.y && max.y > lo.y && min.z < hi.z && max.z > lo.z)
}

#[test]
fn f019_every_supply_station_stands_on_the_depot_floor_of_the_main_building() {
    // ⭐ „drinnen sind die nachschübe" as an assert. Before 2026-08-12 evening the three
    // stations stood at y = 0.0 on the street; the building had been standing since that
    // morning. Nothing in the game said so — the hub worked, the gas came back, and the one
    // sentence the user wrote was not true.
    let d = data(&started(None));
    let (floor_min, floor_max) = floor_under_the_stations(&d);
    let inset = d.game.player.radius_m;

    for station in &d.missions.hub.refuel_stations {
        let at = Vec3::from(station.center_m);
        assert!(
            at.x > floor_min.x + inset && at.x < floor_max.x - inset,
            "station {at:?} is not over the depot floor in x ({} .. {})",
            floor_min.x,
            floor_max.x
        );
        assert!(
            at.z > floor_min.z + inset && at.z < floor_max.z - inset,
            "station {at:?} is not over the depot floor in z ({} .. {})",
            floor_min.z,
            floor_max.z
        );
    }
}

/// The grid step the standing-room scan uses. A cell is 0.0625 m², so 16 of them are a square
/// metre — a player standing still with room to turn.
const APPROACH_STEP_M: f32 = 0.25;

/// Every spot on the depot floor a player can **stand** on and still be inside a station's
/// reach, nearest first.
///
/// This is the question a coordinate check cannot ask. A station's trigger is a 3D distance to
/// its centre, and its centre is the middle of a 5 x 9 m solid rack — so "inside the building"
/// and "usable by a human being" are two different claims, and only the second one is the
/// feature. The scan is a grid and not arithmetic because the obstacles are the map's: the rack
/// itself, the four roof posts, the back wall.
fn approaches_to(station: Vec3, d: &GameData) -> Vec<Vec3> {
    let (floor_min, floor_max) = floor_under_the_stations(d);
    let solids = solid_boxes(d);
    let range = d.gear.resupply.range_m;
    let inset = d.game.player.radius_m;
    let steps = (range / APPROACH_STEP_M).ceil() as i32;
    let mut found: Vec<Vec3> = Vec::new();
    for ix in -steps..=steps {
        for iz in -steps..=steps {
            let spot = station
                + Vec3::new(ix as f32 * APPROACH_STEP_M, 0.0, iz as f32 * APPROACH_STEP_M);
            if spot.distance(station) > range {
                continue;
            }
            let on_the_floor = spot.x > floor_min.x + inset
                && spot.x < floor_max.x - inset
                && spot.z > floor_min.z + inset
                && spot.z < floor_max.z - inset;
            if on_the_floor && standable(spot, d, &solids) {
                found.push(spot);
            }
        }
    }
    found.sort_by(|a, b| {
        a.distance(station).partial_cmp(&b.distance(station)).expect("a NaN coordinate")
    });
    found
}

#[test]
fn f019_a_supply_station_has_floor_you_can_actually_stand_on_inside_its_reach() {
    // ⭐ The half that makes the one above worth having. A station whose centre sits in the
    // middle of a solid rack is "inside the building" by every coordinate check and cannot be
    // used by anybody: the trigger is a 3D distance to the centre, the rack is 5 x 9 m of
    // collider, and a player who walks up to it stands 5 m from the number that decides.
    let d = data(&started(None));
    const NEEDED: usize = 16;

    for station in &d.missions.hub.refuel_stations {
        let at = Vec3::from(station.center_m);
        let room = approaches_to(at, &d);
        assert!(
            room.len() >= NEEDED,
            "the station at {at:?} has {} standable 0.25 m cells inside its {} m reach \
             ({:.2} m\u{b2} of floor) — a player cannot walk up to it",
            room.len(),
            d.gear.resupply.range_m,
            room.len() as f32 * APPROACH_STEP_M * APPROACH_STEP_M
        );
    }
}

/// Where a player who walks **straight in and keeps walking** ends up: the westernmost spot on
/// the aisle centre line he can still stand on, which is the back wall of the hall.
///
/// Derived and never a literal — it is the map's own geometry read from the far side. Scanning
/// from the back of the depot floor eastwards, the first standable cell *is* the stop, because
/// everything west of it is the wall.
fn end_of_the_aisle(d: &GameData) -> Vec3 {
    let (floor_min, floor_max) = floor_under_the_stations(d);
    let solids = solid_boxes(d);
    let y = d.missions.hub.refuel_stations[0].center_m.1;
    let mut x = floor_min.x;
    while x < floor_max.x {
        let spot = Vec3::new(x, y, 0.0);
        if standable(spot, d, &solids) {
            return spot;
        }
        x += 0.05;
    }
    panic!("no standable spot anywhere on the aisle centre line — the hall has no interior");
}

#[test]
fn f019_walking_straight_down_the_aisle_puts_you_in_reach_of_every_rack() {
    // ⭐ The difference between "a player *can* reach the supply" and "a player who walks in
    // finds it". The test above only asks whether standing room exists inside the reach — and
    // it passed with the stations at the **centre** of the two 5 x 9 m racks, where the only
    // standing room was a 1.15 m strip between the rack's east face and a roof post, plus a
    // 1.0 m slot behind the rack against the back wall. 57 cells, all of them threaded
    // (`docs/FINDINGS.md` FIND-075).
    //
    // What the user asked for is a place you walk into: „in das gebäude muss man rein laufen
    // können. drinnen sind die nachschübe". So the criterion is the walk itself — the door is
    // on the aisle centre line, the aisle is 29 m of clear floor, and a player who holds W from
    // his own spawn point is stopped by the back wall. **From that spot every rack has to be in
    // reach**, or the supply is a puzzle.
    let d = data(&started(None));
    let stop = end_of_the_aisle(&d);
    let range = d.gear.resupply.range_m;

    for station in &d.missions.hub.refuel_stations {
        let at = Vec3::from(station.center_m);
        let distance = stop.distance(at);
        assert!(
            distance <= range,
            "a player who walks straight in stops at {stop:?} and the station at {at:?} is \
             {distance:.2} m away — outside the {range} m of gear.ron: resupply.range_m. \
             The supply of the main building has to be where the walk ends"
        );
    }
}

// ---------------------------------------------------------------------------
// One writer of `Blades` — the same shape, built in rather than repaired in
// ---------------------------------------------------------------------------

/// A bare app with **only the hub's rack system in it** — no `blades` anywhere.
///
/// Same construction as [`a_station_and_a_player`] and for the same reason: the real app carries
/// both halves, so it cannot tell "the rack asks and blades restocks" from "the rack restocks
/// itself". Here the applier is simply absent, and a harness that still grows can only have been
/// written by `mission`.
///
/// The player carries an **empty** harness (`pairs_left: 0`, `sharpness: 0.0`), because a full
/// one cannot grow and would make both halves of the test pass for nothing.
fn a_rack_and_a_player(with_blades: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.add_message::<BladeRestockRequest>();
    app.add_systems(FixedUpdate, defeated_by_titan::mission::hub::restock_at_stations);
    if with_blades {
        // `GameData` only for the half that actually applies: the rack does not read it, and a
        // test that hands it to both halves would hide a rack that started reading the tuning.
        app.insert_resource(GameData::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/data"
        ))));
        app.add_systems(
            FixedUpdate,
            defeated_by_titan::blades::resupply::apply_restock_requests
                .after(defeated_by_titan::mission::hub::restock_at_stations),
        );
    }

    app.world_mut().spawn((
        defeated_by_titan::mission::BladeRack { radius_m: 4.0 },
        Transform::from_translation(Vec3::ZERO),
    ));
    let player = app
        .world_mut()
        .spawn((
            PlayerId(1),
            Transform::from_translation(Vec3::ZERO),
            Blades { pairs_left: 0, sharpness: 0.0 },
        ))
        .id();
    (app, player)
}

fn harness(app: &App, player: Entity) -> Blades {
    *app.world().entity(player).get::<Blades>().expect("the player carries a harness")
}

#[test]
fn f033_a_rack_asks_for_blades_and_never_writes_the_harness_itself() {
    // ⭐ The rule-4 test for the other field. `docs/architecture.md`'s authority table says
    // `Blades` is written by `blades` and by nothing else. So a player standing at a rack of a
    // game **without** `blades::resupply` must come out with the harness he went in with.
    //
    // This is the one shape every whole-app test is blind to — it is what let a second writer of
    // `Gas` stand for a day (`docs/FINDINGS.md` FIND-063).
    let (mut app, player) = a_rack_and_a_player(false);
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        harness(&app, player),
        Blades { pairs_left: 0, sharpness: 0.0 },
        "a second at a rack restocked the harness with no `blades` in the app — `mission` is \
         writing `Blades` itself (docs/architecture.md, authority table; rule 4)"
    );

    // The other half, or the first one proves only that nothing works: the SAME rack and the
    // SAME player, with the applier added, do get restocked. Both numbers come out of
    // `gear.ron: resupply` — 1.5 pairs/s and 2.0 sharpness/s over one second.
    let (mut app, player) = a_rack_and_a_player(true);
    let tuning = app.world().resource::<GameData>().gear.resupply.clone();
    for _ in 0..60 {
        app.update();
    }
    let after = harness(&app, player);
    assert_eq!(
        after.pairs_left,
        tuning.blade_pairs_per_s.floor() as u8,
        "a second at the rack gave {} pair(s) of the file's {} per second — the request is \
         written but nothing applies it",
        after.pairs_left,
        tuning.blade_pairs_per_s
    );
    assert_eq!(after.sharpness, 1.0, "the pair in the harness was not honed: {after:?}");
}

#[test]
fn f033_a_rack_is_a_hub_thing_and_the_harness_does_not_refill_in_the_field() {
    // The blade half of Q-033's rule. Standing where a rack stands, in a running sortie, gives
    // nothing back — carried by `DespawnOnExit(MissionPhase::Hub)` on the station and by the
    // `run_if` on the system, the same two mechanisms the gas half has.
    let mut app = in_the_hub();
    let rack = Vec3::from(hub_layout(&app).refuel_stations[0].center_m);
    let pad = pad_center(&app, "recruit");
    place_players(&mut app, pad);
    ticks(&mut app, 10);
    assert_eq!(phase(&app), MissionPhase::Active, "this test needs a running sortie");

    place_players(&mut app, rack);
    let mut q = app.world_mut().query_filtered::<&mut Blades, With<PlayerId>>();
    for mut blades in q.iter_mut(app.world_mut()) {
        *blades = Blades { pairs_left: 0, sharpness: 0.0 };
    }
    ticks(&mut app, 60);
    let mut q = app.world_mut().query_filtered::<&Blades, With<PlayerId>>();
    let after = *q.iter(app.world()).next().expect("no player carries a harness");
    assert_eq!(
        after,
        Blades { pairs_left: 0, sharpness: 0.0 },
        "the harness refilled itself inside a mission, at the place a rack stands in the hub"
    );
}

#[test]
fn f033_a_player_at_a_rack_of_the_hub_walks_away_restocked() {
    // The positive one, in the real app and at the real coordinate out of `missions.ron` — the
    // one that says the wiring is connected end to end and not only in a hand-built app.
    let mut app = in_the_hub();
    let rack = Vec3::from(hub_layout(&app).refuel_stations[0].center_m);
    let capacity = data(&app).gear.blades.start_pairs;

    // Away from every rack first: an empty harness stays empty. Without this the test is
    // satisfied by a restock that runs everywhere, which is exactly the cooldown the design
    // replaced with an economy.
    let outside = nowhere_in_particular(&app, Vec3::new(0.0, 2.0, -6.0));
    place_players(&mut app, outside);
    let mut q = app.world_mut().query_filtered::<&mut Blades, With<PlayerId>>();
    for mut blades in q.iter_mut(app.world_mut()) {
        *blades = Blades { pairs_left: 0, sharpness: 0.0 };
    }
    ticks(&mut app, 60);
    let mut q = app.world_mut().query_filtered::<&Blades, With<PlayerId>>();
    let away = *q.iter(app.world()).next().expect("no player carries a harness");
    assert_eq!(
        away,
        Blades { pairs_left: 0, sharpness: 0.0 },
        "the harness restocked itself away from any rack — economy instead of cooldowns"
    );

    // And at the rack the harness comes back, up to the cap and no further. **Not at the
    // station's centre**, which is the middle of a solid rack — at the nearest spot on the
    // depot floor a player could actually be standing (`approaches_to`). That is the difference
    // between "the trigger works" and "you can use it", and it is also where
    // `scripts/f070-hub.txt` has to put the player.
    let approach = approaches_to(rack, &data(&app))[0];
    assert!(
        approach.distance(rack) <= data(&app).gear.resupply.range_m,
        "the approach {approach:?} is outside the reach it was scanned in"
    );
    place_players(&mut app, approach);
    ticks(&mut app, 600);
    let mut q = app.world_mut().query_filtered::<&Blades, With<PlayerId>>();
    let full = *q.iter(app.world()).next().expect("no player carries a harness");
    assert_eq!(full.pairs_left, capacity, "ten seconds at the rack gave {full:?}");
    assert_eq!(full.sharpness, 1.0);
}

// ---------------------------------------------------------------------------
// F-057 .. F-063 — **the roster fights differently, and the difference is mechanical**
// ---------------------------------------------------------------------------
//
// Until 2026-08-19 `titan.ron` carried eight kinds and the game had one enemy: every kind ran
// `titan::brain` with different numbers, so the player's answer to all eight was the same
// answer. `docs/gameplay/enemies.md` is explicit that this is the failure mode of the whole
// combat system — *"of its enemy types, exactly one demands real timing … everything else is
// mobile feed"* — and it asks for the opposite: **at least half of all kinds carry an
// anti-autopilot property.**
//
// Every test below is a PAIR: the kind that has the property and the husk, which has none, in
// the same app, under the same tick count. A single-kind assertion ("the errant swerves") is
// worth nothing here — it cannot tell a behaviour from a coordinate. The husk is the control,
// and each test names the one line in `assets/data/titan.ron` that has to be zeroed for it to
// go red.

use avian3d::prelude::ColliderDisabled;
use defeated_by_titan::shared::{TitanKindName, TitanState};
use defeated_by_titan::shared::StateClock;
use defeated_by_titan::titan::brain::{Guard, TitanTiming};

/// Every titan of one kind: its root entity, where it stands, and what it is doing.
fn bodies_of(app: &mut App, kind: &str) -> Vec<(Entity, Vec3, TitanState)> {
    let mut q = app
        .world_mut()
        .query::<(Entity, &TitanKindName, &Transform, &TitanState)>();
    let mut found: Vec<(Entity, Vec3, TitanState)> = q
        .iter(app.world())
        .filter(|(_, name, _, _)| name.as_str() == kind)
        .map(|(e, _, t, s)| (e, t.translation, *s))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// The one body of a kind, or a message that says which kind was missing.
fn one_body(app: &mut App, kind: &str) -> (Entity, Vec3, TitanState) {
    let found = bodies_of(app, kind);
    assert_eq!(found.len(), 1, "expected exactly one {kind} in the world, found {}", found.len());
    found[0]
}

/// **Is this titan's nape out of the world right now?**
///
/// Counted over the whole rig rather than looked up by `TitanPart`, on purpose: the cortex is
/// the only part `titan::brain::guard_the_cortex` ever disables, and a test that names a part
/// type would have to be rewritten the day the rig grows limb zones. What is under test is the
/// *effect* — a blade cast finds nothing where the nape is.
fn nape_is_covered(app: &mut App, root: Entity) -> bool {
    let mut pending = vec![root];
    let mut covered = false;
    while let Some(entity) = pending.pop() {
        if let Some(kids) = app.world().get::<Children>(entity) {
            pending.extend(kids.iter());
        }
        if app.world().get::<ColliderDisabled>(entity).is_some() {
            covered = true;
        }
    }
    covered
}

/// A started app with the player standing at the origin and nothing else going on.
fn a_field() -> App {
    let mut app = started(None);
    place_players(&mut app, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 2);
    app
}

/// Turns every titan in the world to face the player it can see, on the spot.
///
/// A titan spawns facing −Z and turns at `turn_deg_per_s` — 50°/s for a husk. A body that has
/// to come about 177° therefore spends **three and a half seconds walking an arc**, and any
/// measurement of where two of them go is measuring that arc and not their behaviour.
fn face_the_player(app: &mut App) {
    let mut players = app.world_mut().query_filtered::<&Transform, With<PlayerId>>();
    let at = players
        .iter(app.world())
        .next()
        .expect("no player to face")
        .translation;
    let mut q = app.world_mut().query_filtered::<Entity, With<TitanId>>();
    let bodies: Vec<Entity> = q.iter(app.world()).collect();
    for body in bodies {
        let from = app.world().get::<Transform>(body).expect("a titan has a Transform").translation;
        let to = Vec3::new(at.x - from.x, 0.0, at.z - from.z);
        let yaw = f32::atan2(-to.x, -to.z);
        app.world_mut()
            .entity_mut(body)
            .insert(Transform::from_translation(from).with_rotation(Quat::from_rotation_y(yaw)));
    }
}

/// How far a point lies off the straight line from `spawn` to `goal`, on the ground plane.
fn off_the_line(spawn: Vec3, goal: Vec3, at: Vec3) -> f32 {
    let line = Vec3::new(goal.x - spawn.x, 0.0, goal.z - spawn.z).normalize();
    let d = Vec3::new(at.x - spawn.x, 0.0, at.z - spawn.z);
    (d - line * d.dot(line)).length()
}

/// **`F-057` — the errant is never on the line you aimed at, and the husk always is.**
///
/// What it costs the player: a shot has to be led. A husk walks the straight line from where it
/// stands to where you stand, so the hook you fire at it lands where you fired it; an errant
/// swings `behaviour.swerve_deg` (35°) to each side, flipping every `swerve_period_s` (0.9 s),
/// and is metres off that line by the time anything you threw arrives.
///
/// **Red when:** `assets/data/titan.ron` gives the errant `swerve_deg: 0.0`. Measured — with
/// that one edit the errant's excursion falls from 3.15 m to 0.00 m and the assert below fails.
#[test]
fn f057_the_errant_leaves_the_line_the_husk_walks() {
    let mut app = a_field();
    let goal = Vec3::ZERO;
    let husk_at = Vec3::new(0.0, 0.0, 40.0);
    let errant_at = Vec3::new(40.0, 0.0, 0.0);
    spawn_titan(&mut app, "husk", husk_at);
    spawn_titan(&mut app, "errant", errant_at);

    // Both are inside their own `aggro_radius_m` (45 and 50) at 40 m, and neither is inside the
    // other's — nothing here is a group effect.
    let mut husk_worst = 0.0f32;
    let mut errant_worst = 0.0f32;
    for _ in 0..240 {
        ticks(&mut app, 1);
        husk_worst = husk_worst.max(off_the_line(husk_at, goal, one_body(&mut app, "husk").1));
        errant_worst =
            errant_worst.max(off_the_line(errant_at, goal, one_body(&mut app, "errant").1));
    }

    assert!(
        husk_worst < 0.5,
        "the husk wandered {husk_worst:.2} m off its own line — the control is not a control"
    );
    assert!(
        errant_worst > 2.0,
        "the errant stayed {errant_worst:.2} m off the line; with swerve_deg 35 and 0.9 s of \
         half-period at 6.5 m/s it has to leave it by metres, or `behaviour.swerve_deg` is not \
         reaching `titan::brain::aim`"
    );
    println!("F-057 excursion: husk {husk_worst:.2} m · errant {errant_worst:.2} m");
}

/// **`F-058` — the scuttler's blow carries his body; the husk's does not.**
///
/// What it costs the player: sideways is no longer far enough. A husk strikes from a planted
/// stance, so stepping out of a 6 m reach is the whole answer; a scuttler covers
/// `behaviour.lunge_m_s` × `strike_s` = 14.0 × 0.2 = **2.8 m** while the blow is already in the
/// air, on top of a 2.5 m reach. The answer the design asks for is altitude
/// (`docs/gameplay/enemies.md`: *"vertical evasion"*), and that is what this measures the room
/// for.
///
/// **Red when:** `lunge_m_s: 0.0` on the scuttler — then both numbers are 0.00 m.
#[test]
fn f058_the_scuttler_travels_through_his_own_strike_and_the_husk_stands_still() {
    let mut app = a_field();
    // Each inside its own `attack_range_m` (2.5 and 6.0) from the start, so both go straight to
    // `Windup` without a walk that would pollute the measurement.
    spawn_titan(&mut app, "scuttler", Vec3::new(2.0, 0.0, 0.0));
    spawn_titan(&mut app, "husk", Vec3::new(0.0, 0.0, 5.0));

    let mut travelled = |app: &mut App, kind: &str, last: &mut Vec3| -> f32 {
        let (_, at, state) = one_body(app, kind);
        let step = if state == TitanState::Strike { at.distance(*last) } else { 0.0 };
        *last = at;
        step
    };
    let mut scuttler_last = one_body(&mut app, "scuttler").1;
    let mut husk_last = one_body(&mut app, "husk").1;
    let (mut scuttler_m, mut husk_m) = (0.0f32, 0.0f32);
    for _ in 0..180 {
        ticks(&mut app, 1);
        scuttler_m += travelled(&mut app, "scuttler", &mut scuttler_last);
        husk_m += travelled(&mut app, "husk", &mut husk_last);
    }

    assert!(husk_m < 0.01, "the husk moved {husk_m:.3} m inside his own Strike — he must not");
    assert!(
        scuttler_m > 2.0,
        "the scuttler covered {scuttler_m:.3} m inside his Strike; 14.0 m/s × 0.2 s is 2.8 m, so \
         `behaviour.lunge_m_s` is not reaching `titan::brain::walk`"
    );
    println!("F-058 ground covered inside Strike: husk {husk_m:.3} m · scuttler {scuttler_m:.3} m");
}

/// **`F-059` — a weaver's nape is not a target until he commits.**
///
/// What it costs the player: the approach cannot be spammed. On a husk the nape is always
/// there, so the optimal play is to fly at it whenever the gas allows; on a weaver the cortex
/// sensor is **out of the world** while he is `Idle` or `Pursue` and comes back only for the
/// 0.50 + 0.18 + 0.37 = 1.05 s he is inside his own attack. Bait the blow, cut in the window —
/// which is the *"timing instead of spam"* the design asks the weaver to teach.
///
/// It is the collider and not the message that is taken away, and that is the load-bearing
/// half: a `TitanHit { zone: Cortex }` dropped inside `titan/` would still have been counted a
/// kill by `mission::count_kills`, which reads the message and not the corpse.
///
/// **Red when:** the weaver's `cortex_guard` is set to `Always`.
#[test]
fn f059_the_weavers_nape_is_out_of_the_world_until_he_commits() {
    let mut app = a_field();
    spawn_titan(&mut app, "weaver", Vec3::new(0.0, 0.0, 20.0));
    spawn_titan(&mut app, "husk", Vec3::new(0.0, 0.0, -20.0));
    ticks(&mut app, 4);

    let weaver = one_body(&mut app, "weaver").0;
    let husk = one_body(&mut app, "husk").0;
    assert_eq!(one_body(&mut app, "weaver").2, TitanState::Pursue, "the weaver has to be coming");
    assert!(nape_is_covered(&mut app, weaver), "a weaver in Pursue must not offer his nape");
    assert!(!nape_is_covered(&mut app, husk), "the husk's nape is always open — that is the husk");

    // He walks 7 m/s into a 2.5 m reach from 20 m: the wind-up is inside three seconds.
    let mut committed = None;
    for tick in 0..300 {
        ticks(&mut app, 1);
        if matches!(
            one_body(&mut app, "weaver").2,
            TitanState::Windup | TitanState::Strike | TitanState::Recover
        ) {
            committed = Some(tick);
            break;
        }
    }
    let at = committed.expect("the weaver never committed in five seconds — the test measured nothing");
    // One tick of grace: `guard_the_cortex` runs after `advance` in the same tick, but the
    // `Commands` it queues are applied at the next flush.
    ticks(&mut app, 2);
    assert!(
        !nape_is_covered(&mut app, weaver),
        "the weaver committed at tick {at} and his nape is still covered — then there is no \
         window and the kind is unkillable, not hard"
    );
    println!("F-059 the weaver's nape opened at tick {at} of his approach");
}

/// **`F-060` — the warden's hand is on his nape until a body cut knocks it off.**
///
/// What it costs the player: a two-stage attack. One pass is not enough — the first blade goes
/// into the body (which also staggers him for `stagger_s` 0.14 s, `F-032`), and only then is
/// the cortex a target, for `cortex_guard: WhenOpened(3.0)` seconds. That is
/// `docs/gameplay/enemies.md`'s *"arms first, then the cortex"*, and it is the one kind in the
/// roster whose fight has two steps.
///
/// ⚠️ **"Arms first" is still the design and still not the code.** `F-060`'s acceptance names
/// the arms — *"Frontalangriff auf Arme oeffnet den Cortex"* — and since `F-032` gave the limbs
/// their own zones the arm hit exists to be read. It is not read yet: narrowing the opener to
/// the two arm zones reddens four 🟧 reach rows that reach the warden's cortex only because
/// their own pass grazes his torso first. The second half of this test is the half that is new:
/// **an arm cut opens him too**, so the day the narrowing lands, this line already holds.
/// `titan::brain::receive_hits` carries the one-line diff and the reason.
///
/// **Red when:** the warden's `cortex_guard` is set to `Always`.
#[test]
fn f060_a_body_cut_opens_the_wardens_nape_and_time_closes_it_again() {
    let mut app = a_field();
    let park = park_players_out_of_aggro(&mut app);
    assert!(park > 0.0);
    spawn_titan(&mut app, "warden", Vec3::new(0.0, 0.0, 0.0));
    spawn_titan(&mut app, "husk", Vec3::new(30.0, 0.0, 0.0));
    ticks(&mut app, 4);

    let (warden, _, _) = one_body(&mut app, "warden");
    let husk = one_body(&mut app, "husk").0;
    assert!(nape_is_covered(&mut app, warden), "a warden starts with his hand on his nape");

    let open_ticks = {
        let d = data(&app);
        let guard = Guard::of(&d.titans.kinds["warden"], d.game.simulation_hz);
        guard.open_ticks
    };
    assert!(open_ticks > 0, "titan.ron: the warden's cortex_guard has to be WhenOpened(s)");

    let warden_id = *app.world().get::<TitanId>(warden).expect("a warden carries a TitanId");
    let husk_id = *app.world().get::<TitanId>(husk).expect("a husk carries a TitanId");
    let by = a_player(&mut app);

    // The same message a blade through the chest writes (`scripts/f032-swords.txt` act B).
    for titan in [warden_id, husk_id] {
        app.world_mut().write_message(TitanHit {
            titan,
            by,
            zone: HitZone::Torso,
            speed_m_s: 20.67,
        });
    }
    ticks(&mut app, 3);
    assert!(!nape_is_covered(&mut app, warden), "a cut into the body has to open the nape");
    assert!(!nape_is_covered(&mut app, husk), "a husk has no guard and a torso hit changes nothing");

    // …and it closes again. Otherwise the first graze of the sortie makes him a husk forever.
    ticks(&mut app, open_ticks as u64 + 4);
    assert!(
        nape_is_covered(&mut app, warden),
        "the warden's nape stayed open past his own {open_ticks} ticks — the window is not a window"
    );
    // **And the arm opens him too** — the zone `blades::cut::limb_zone` produces for a blade
    // across the arm box, which did not exist before `F-032`
    // (`tests/combat.rs::f032_a_cut_through_the_arm_is_an_arm_hit_and_never_the_torso`). Today
    // that is one zone among several; the day `receive_hits` narrows the opener to the arms it
    // is the only one, and this assertion is what carries over unchanged.
    app.world_mut().write_message(TitanHit {
        titan: warden_id,
        by,
        zone: HitZone::ArmRight,
        speed_m_s: 20.67,
    });
    ticks(&mut app, 3);
    assert!(
        !nape_is_covered(&mut app, warden),
        "a cut into the warden's ARM did not open his nape — that is F-060's own acceptance \
         sentence, and it is the one zone that must never stop working"
    );
    println!(
        "F-060 the warden's nape opened on a torso hit and on an arm hit, and closed again \
         after {open_ticks} ticks"
    );
}

/// **`F-061` — the lurker never takes a step, and that is the whole kind.**
///
/// What it costs the player: attention instead of reflex. He cannot be kited, because there is
/// nothing to kite; a lurker you have seen is free ground, a lurker you have not seen costs 48
/// health the moment you pass inside his 8 m reach. `behaviour.ambush` removes `Idle → Pursue`
/// from his state machine entirely and sends `Recover` back to `Idle` instead of on to
/// `Pursue`.
///
/// **Red when:** `ambush: false` on the lurker — he then walks the 20 m like anything else.
#[test]
fn f061_the_lurker_holds_his_ground_while_the_husk_comes_for_you() {
    let mut app = a_field();
    let lurker_at = Vec3::new(0.0, 0.0, 20.0);
    let husk_at = Vec3::new(30.0, 0.0, 20.0);
    spawn_titan(&mut app, "lurker", lurker_at);
    spawn_titan(&mut app, "husk", husk_at);
    ticks(&mut app, 600);

    let (_, lurker_now, lurker_state) = one_body(&mut app, "lurker");
    let (_, husk_now, _) = one_body(&mut app, "husk");
    let lurker_m = Vec3::new(lurker_now.x - lurker_at.x, 0.0, lurker_now.z - lurker_at.z).length();
    let husk_m = Vec3::new(husk_now.x - husk_at.x, 0.0, husk_now.z - husk_at.z).length();

    assert_eq!(lurker_state, TitanState::Idle, "a lurker at 20 m has nothing to do but wait");
    assert!(lurker_m < 0.05, "the lurker walked {lurker_m:.2} m — an ambusher does not walk");
    assert!(
        husk_m > 20.0,
        "the husk covered {husk_m:.2} m in ten seconds at 3 m/s — the control did not run"
    );
    println!("F-061 in ten seconds: lurker {lurker_m:.2} m · husk {husk_m:.2} m");
}

/// **`F-063` — two chorus arrive from two sides.**
///
/// What it costs the player: target prioritization. Two husks walk down the same line and stay
/// in one silhouette, so facing one faces both; two chorus aim `behaviour.flank_offset_m` (9 m)
/// to opposite sides of you — the sign comes off the titan's own id — and close from there.
/// Whichever one you turn to, the other is behind you, and `combat::strike`'s cone (chorus: 55°)
/// is what makes that cost something.
///
/// **The two pairs run in two apps, from the same spawn, facing the player from tick one.**
/// Both of those are the test's own history: a pair 80 m behind the other pair is inside
/// nobody's aggro but a pair that has to turn 177° first spends four seconds walking an arc,
/// and the husk control then "separated" by 7.96 m without any behaviour at all. What is under
/// test is the walk, not the turn.
///
/// **Red when:** `flank_offset_m: 0.0` on the chorus — the separation collapses onto the husks'.
#[test]
fn f063_a_chorus_pair_splits_where_a_husk_pair_stacks() {
    let widest = |kind: &str| -> f32 {
        let mut app = a_field();
        spawn_titan(&mut app, kind, Vec3::new(-2.0, 0.0, 40.0));
        spawn_titan(&mut app, kind, Vec3::new(2.0, 0.0, 40.0));
        face_the_player(&mut app);
        let mut worst = 0.0f32;
        for _ in 0..600 {
            ticks(&mut app, 1);
            let pair = bodies_of(&mut app, kind);
            assert_eq!(pair.len(), 2, "a pair is two bodies");
            let (a, b) = (pair[0].1, pair[1].1);
            worst = worst.max(Vec3::new(a.x - b.x, 0.0, a.z - b.z).length());
        }
        worst
    };
    let chorus_m = widest("chorus");
    let husk_m = widest("husk");

    assert!(
        husk_m < 4.5,
        "the husks came within {husk_m:.2} m of each other's line — they start 4 m apart and \
         walk at the same point, so they may only converge; the control is broken"
    );
    assert!(
        chorus_m > husk_m * 2.5,
        "the chorus pair reached {chorus_m:.2} m of separation against the husks' {husk_m:.2} m \
         — with 9 m of flank offset to each side that has to be a different fight"
    );
    println!("F-063 widest separation on the approach: chorus {chorus_m:.2} m · husks {husk_m:.2} m");
}

/// **`F-062` — one bellower's call wakes a titan that cannot see you.**
///
/// What it costs the player: he cannot fight one at a time. A husk 140 m away is blind — his
/// `aggro_radius_m` is 45 — and stays `Idle` all sortie. Put a bellower between you and him and
/// the husk comes: `call_radius_m` 90 m, `call_hold_s` 25 s, and `titan::brain::decide` ignores
/// the husk's own eyes while the alert holds.
///
/// ## ⚠️ Two honest holes, and this test names both
///
/// 1. **The bellower cannot be spawned in the game.** He is class `huge` (21 m) and
///    `scale.ron: max_spawnable_class` is `large` (`docs/QUESTIONS.md` Q-028) — so this test
///    raises the cap **in its own copy of the data** and nowhere else. That is deliberate: it
///    proves the call is built and that exactly one line of RON stands between the player and
///    the eighth kind. See `docs/FINDINGS.md` FIND-118.
/// 2. **The ear is missing.** The design has him react to the *sound of gas* (`F-051`); there
///    is no perception model, so he calls on sight. The stealth layer is not built.
///
/// **Red when:** `call_radius_m: 0.0` on the bellower — the husk then stays `Idle`, which is
/// what the second half of this test asserts with no bellower in the world at all.
#[test]
fn f062_a_bellowers_call_reaches_a_husk_that_is_blind_on_his_own() {
    let far = Vec3::new(0.0, 0.0, 140.0);

    // ---- with the bellower ------------------------------------------------------------
    let mut app = built(None);
    // The one line of `scale.ron` this kind is waiting for, raised here and only here.
    app.world_mut().resource_mut::<GameData>().scale.titan.max_spawnable_class = "huge".to_string();
    for _ in 0..4 {
        app.update();
        if !app.world().resource::<Log>().0.is_empty() {
            break;
        }
    }
    place_players(&mut app, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 2);
    spawn_titan(&mut app, "bellower", Vec3::new(0.0, 0.0, 60.0));
    spawn_titan(&mut app, "husk", far);
    assert_eq!(
        bodies_of(&mut app, "bellower").len(),
        1,
        "the cap was raised in this app's data and the bellower still did not spawn"
    );
    ticks(&mut app, 30);
    let called = one_body(&mut app, "husk").2;

    // ---- and without him ---------------------------------------------------------------
    let mut alone = a_field();
    spawn_titan(&mut alone, "husk", far);
    ticks(&mut alone, 30);
    let blind = one_body(&mut alone, "husk").2;

    assert_eq!(
        blind,
        TitanState::Idle,
        "a husk 140 m from the player sees nothing (aggro_radius_m 45) — without that the test \
         above proves nothing"
    );
    assert_eq!(
        called,
        TitanState::Pursue,
        "the bellower stood 80 m from the husk, inside his 90 m call, and the husk did not come"
    );
    println!("F-062 the same husk at 140 m: {blind:?} alone · {called:?} with a bellower in earshot");
}

/// **Every wave of every difficulty names a kind that may actually spawn.**
///
/// `tests/data.rs::t005_every_wave_names_a_titan_kind_that_exists` checks the templates' own
/// waves and stops there — the `difficulties` are where the mixes live, and a kind that is in
/// `titan.ron` but above `scale.ron: max_spawnable_class` would be refused at spawn and the
/// wave would simply not arrive. That is a mission short one titan and a log line nobody reads.
#[test]
fn f065_every_wave_of_every_difficulty_asks_for_a_kind_that_may_spawn() {
    let app = built(None);
    let d = data(&app);
    let mut seen = 0;
    for (mission, template) in &d.missions.templates {
        let levels = template.difficulties.iter().map(|(n, l)| (n.as_str(), &l.waves));
        let own = std::iter::once(("(the template's own)", &template.waves));
        for (level, waves) in own.chain(levels) {
            for wave in waves {
                seen += 1;
                assert!(
                    defeated_by_titan::titan::spawnable(&d, &wave.kind).is_ok(),
                    "{mission}/{level}: the wave at {} s asks for {:?}, which cannot spawn: {}",
                    wave.at_s,
                    wave.kind,
                    defeated_by_titan::titan::spawnable(&d, &wave.kind).unwrap_err()
                );
            }
        }
    }
    assert!(seen > 10, "only {seen} waves were checked — missions.ron got smaller, not the test");
}

/// ★ **`F-059` — the roll, and this is the half `shared/state.rs` was needed for.**
///
/// The roster round built the weaver's *lesson* (`cortex_guard: WhenCommitted` — his nape is
/// out of the world unless he is committed to his own attack) and said so plainly: the roll
/// itself needs a [`TitanState`] arm, and that file was another hand's. This is the arm.
///
/// The backlog's acceptance sentence is *"Rolle hat lesbares Startup, danach garantierte
/// Unverwundbarkeit fuer definierte Dauer"* (`F-059`), and the state is cut along exactly that
/// line — so this test asks for exactly those three things:
///
/// 1. his attack **ends in a roll** and not in a walk back to `Pursue`;
/// 2. the roll's `roll_startup_s` is a window in which the nape is **still a target** — the tell
///    is readable *and* punishable, which is what makes it a startup and not a cheat;
/// 3. after it, and until the roll is over, the cortex is **out of the world** — the guaranteed
///    invulnerability, with a number on it;
/// 4. and the body has moved **backwards** by the end of it, so the roll costs the player
///    position and not only time.
///
/// **Red when:** `titan.ron: weaver.behaviour.roll_s` goes to 0 (he never enters the state), or
/// `roll_startup_s` is raised to `roll_s` (no i-frames), or `Guard::open` stops making the
/// startup an exception.
#[test]
fn f059_the_weavers_attack_ends_in_a_roll_that_is_readable_first_and_untouchable_after() {
    let mut app = a_field();
    spawn_titan(&mut app, "weaver", Vec3::new(0.0, 0.0, 12.0));
    ticks(&mut app, 4);
    let weaver = one_body(&mut app, "weaver").0;

    let (roll_ticks, startup_ticks) = {
        let d = data(&app);
        let t = TitanTiming::of(&d.titans.kinds["weaver"], d.game.simulation_hz);
        (t.roll_ticks, t.roll_startup_ticks)
    };
    assert!(
        roll_ticks > startup_ticks && startup_ticks > 0,
        "titan.ron: the weaver's roll is {roll_ticks} ticks with a {startup_ticks}-tick startup \
         — a roll with no startup is invulnerability with no tell, and a roll that is all \
         startup has no invulnerability in it at all"
    );

    // He walks 7 m/s into a 2.5 m reach from 12 m, so the whole cycle is inside four seconds.
    // Sampled every tick, because what is under test is WHERE inside the state things change.
    let mut seen: Vec<(u32, TitanState, bool, f32)> = Vec::new();
    let mut rolled = false;
    for _ in 0..600 {
        ticks(&mut app, 1);
        let state = *app.world().get::<TitanState>(weaver).expect("the weaver has a state");
        let at = app.world().get::<Transform>(weaver).expect("a body").translation;
        let n = app.world().get::<StateClock>(weaver).expect("a clock").ticks_in_state;
        if state == TitanState::Roll {
            rolled = true;
            seen.push((n, state, nape_is_covered(&mut app, weaver), at.z));
        } else if rolled {
            break;
        }
    }
    assert!(rolled, "the weaver never rolled in ten seconds — his attack still ends in Pursue");
    assert!(seen.len() as u32 >= roll_ticks - 1, "the roll lasted {} ticks, not {roll_ticks}", seen.len());

    // 2. The startup is open. One tick of grace at the seam: `guard_the_cortex` queues its
    //    `Commands` and Bevy applies them at the next flush.
    let open_in_startup = seen.iter().filter(|(n, _, covered, _)| *n + 1 < startup_ticks && !covered).count();
    assert!(
        open_in_startup > 0,
        "the weaver's nape was covered for every one of the {startup_ticks} startup ticks: \
         {seen:?}. Then the tell is not a window and the roll is a free escape"
    );

    // 3. …and the rest of the roll is not. This is the acceptance sentence.
    let late: Vec<_> = seen.iter().filter(|(n, _, _, _)| *n > startup_ticks + 1).collect();
    assert!(!late.is_empty(), "the roll has no ticks after its own startup: {seen:?}");
    assert!(
        late.iter().all(|(_, _, covered, _)| *covered),
        "the weaver was cuttable after his startup: {late:?} — F-059 asks for GUARANTEED \
         invulnerability for a defined duration, and a guarantee with a hole is not one"
    );

    // 4. And he really left. He stands at z = +12 and the player at the origin, so a retreat is
    //    an increase in z.
    let (_, _, _, first) = seen.first().copied().expect("the roll had a first tick");
    let (_, _, _, last) = seen.last().copied().expect("the roll had a last tick");
    assert!(
        last - first > 1.0,
        "the weaver rolled {:.2} m backwards — a roll that does not move is a timer",
        last - first
    );
    println!(
        "F-059 the weaver's roll: {} ticks ({startup_ticks} of startup), nape open for {} of \
         them, {:.2} m of retreat",
        seen.len(),
        seen.iter().filter(|(_, _, c, _)| !c).count(),
        last - first
    );
}

/// **The control: seven of eight kinds never enter the state at all.**
///
/// `TitanState`'s own doc calls a variant nothing enters or leaves "decoration"; the mirror of
/// that failure is a variant *everything* enters, which would turn the whole roster into
/// weavers. `roll_s: 0.0` is what says "this kind does not roll", and this is the test that it
/// is read.
#[test]
fn f059_only_the_weaver_rolls_and_the_file_is_what_says_so() {
    let app = a_field();
    let d = data(&app);
    let hz = d.game.simulation_hz;
    let rollers: Vec<&String> =
        d.titans.kinds.iter().filter(|(_, k)| k.behaviour.roll_s > 0.0).map(|(n, _)| n).collect();
    assert_eq!(
        rollers,
        vec!["weaver"],
        "titan.ron: {rollers:?} roll. The roll is the weaver's identity (docs/gameplay/enemies.md) \
         and a second kind with it is a reskin of him"
    );
    for (name, kind) in &d.titans.kinds {
        let t = TitanTiming::of(kind, hz);
        if kind.behaviour.roll_s <= 0.0 {
            assert_eq!(t.roll_ticks, 0, "{name} has roll_s 0 and still resolves {} roll ticks", t.roll_ticks);
            continue;
        }
        assert!(
            t.roll_startup_ticks > 0 && t.roll_startup_ticks < t.roll_ticks,
            "{name}: {} startup ticks out of {} — see F-059's acceptance",
            t.roll_startup_ticks,
            t.roll_ticks
        );
    }
    println!("F-059 rollers in titan.ron: {rollers:?}");
}
