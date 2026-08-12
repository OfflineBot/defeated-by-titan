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
    let mut q = app.world_mut().query_filtered::<&mut Gas, With<PlayerId>>();
    for mut gas in q.iter_mut(app.world_mut()) {
        gas.current = 0.0;
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

    // 3. And never above the tank.
    ticks(&mut app, 1200);
    let full = gas_left(&mut app);
    let max = data(&app).game.vector.gas_tank;
    assert!((full - max).abs() < 1e-3, "the station overfilled the tank: {full} of {max}");
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
