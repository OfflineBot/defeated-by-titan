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
    let mut app = defeated_by_titan::app(Cli {
        headless: true,
        mission: mission.map(|s| s.to_string()),
        ..default()
    });
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
    let highest = *titan_ids(&mut app).last().expect("bodies are still standing");
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
