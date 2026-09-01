//! `F-001` — the guard over the two hooks.
//!
//! The acceptance from `docs/features.ron` reads: "Beide Haken lassen sich unabhaengig setzen
//! und loesen; Zustaende sind im HUD sichtbar." The **second half is not testable here** —
//! `hud/` draws nothing yet and `render::rope::draw_ropes` is an empty stub. What this file
//! covers is the first half, and every number it measures comes out of `assets/data/game.ron`.
//!
//! ## Two things the tests do that need a reason
//!
//! 1. **They write `AimPoint` themselves, through a system of their own.** `vector::aim::aim`
//!    (`F-002`) is built and its ray does hit — but **no body in this world carries a
//!    [`BodyId`] yet**, because `world::index::maintain_index` (`T-036a`) is still a stub.
//!    Measured on 2026-08-09 `[offlinebot]` after 181 ticks: the real `AimPoint` of a standing
//!    player is `point_m: Some((0.0, 1.5999689, -10.0))`, `anchorable: true`, **`body: None`**,
//!    and exactly 0 entities in the world carry a `BodyId`. A hook cannot hang on a carrier
//!    that has no stable id, so in the real game every shot ends as `NoAnchor` today.
//!    Writing the component from outside between two `update()` calls would not survive, so the
//!    injector is a system in `SimulationSystems::World` — the stage before
//!    `SimulationSystems::Intent`, where the hooks read. It stands in for the finished
//!    `F-002` + `T-036a` and delivers exactly what the `AimPoint` interface promises: a point,
//!    a carrier, and whether it holds.
//!
//!    🔴 **It carries no `.after(aim)`, and that is load-bearing, not tidying.** `aim` used to
//!    sit in `World` too, so ordering after it was the only way to be the last writer before
//!    `Intent`. `FIND-217`/`B-029` moved `aim` to `SimulationSystems::PostStep`, and the
//!    six stages are `.chain()`ed (`src/lib.rs`) — so `World -> ... -> PostStep -> World`
//!    closed a **dependency cycle** in `FixedUpdate`. Bevy answers a cycle by enumerating
//!    every simple cycle in the component and formatting all of them into one `String`:
//!    2 290 028 cycles here, a single `realloc` of **4.63 GB**, and on 2026-09-01 an
//!    OOM kill that took the user's tmux session with it (`B-030`, `FIND-218`).
//!    With `aim` last in the tick, `World` at the start of the next tick already **is** the
//!    last writer before `Intent`. The order needs no edge; the edge only needed a cycle.
//! 2. **The flight-time test moves the number in `GameData`.** A test that only measures
//!    against the file's own value stays green when somebody hard-codes exactly that value in
//!    Rust. Tripling the speed at run time and measuring again is what makes it red.
//!
//! ## Why these tests drive with `app.update()`
//!
//! The same reason as in `tests/player.rs`: `tests/multiplayer.rs` advances `Time<Fixed>` by
//! hand and runs `FixedMain` directly, and avian's step size then comes from the *generic*
//! `Time` (`avian3d-0.7.0/src/schedule/mod.rs:238-244`). `TimeUpdateStrategy::FixedTimesteps(1)`
//! makes one `App::update()` exactly one simulation step on every machine.
//!
//! The picture and the script that belong to `F-001`: `scripts/f-001-hooks.txt` and
//! `docs/images/f-001-hooks.png`.

use std::fmt::Write as _;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::net::Inbox;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    AimPoint, ArmAim, BodyGone, BodyId, BodyMask, Buttons, Cli, Hook, HookAnchored, HookReleased,
    HookState, IdCounter, IndexEntry, Intent, LocalPlayer, MissReason, PlayerId, PrevButtons,
    ReleaseReason, RopeLength, Side, SimulationSystems, SpatialIndex, Tick,
};
use defeated_by_titan::shared::{LookOverride, PlayerSettings, Velocity};
use defeated_by_titan::data::VectorTuning;
use defeated_by_titan::vector::aim::{
    deviation_rad, look_basis, pick_best, probe_dirs, required_margin,
    score_candidate, AimCandidate, ScoreContext,
};

// ---------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------

/// What `vector::aim` would write if it were built. See the module header.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ForcedAim(AimPoint);

/// Writes the forced point into **both** carriers: the centre ray the crosshair reads
/// ([`AimPoint`]) and the per-arm ray the hook fires at ([`ArmAim`], `F-023`).
///
/// Both, with the same value, because that is exactly what the real `vector::aim` produces
/// for a target the whole spread covers: a side ray that finds nothing anchorable falls back
/// to the centre ray. The tests in this file are about the **state machine**, not about the
/// hemisphere split — that one has its own fixture in `tests/vector_aiming.rs`, in a map with
/// real geometry. Forcing only one of the two would test which component the hook happens to
/// read instead of what it does with what it reads.
fn force_aim(mut players: Query<(&ForcedAim, &mut AimPoint, &mut ArmAim)>) {
    for (forced, mut point, mut arms) in &mut players {
        point.set_if_neq(forced.0);
        arms.set_if_neq(ArmAim { arms: [forced.0; 2] });
    }
}

/// Every message the two hooks sent, in order. Not player state — a log of the run.
#[derive(Resource, Default)]
struct HookLog {
    anchored: Vec<HookAnchored>,
    released: Vec<HookReleased>,
}

fn collect_messages(
    mut log: ResMut<HookLog>,
    mut anchored: MessageReader<HookAnchored>,
    mut released: MessageReader<HookReleased>,
) {
    log.anchored.extend(anchored.read().copied());
    log.released.extend(released.read().copied());
}

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<HookLog>();
    // No `.after(aim)` — see the module header, `B-030`. `aim` runs in `PostStep`.
    app.add_systems(FixedUpdate, force_aim.in_set(SimulationSystems::World));
    app.add_systems(Last, collect_messages);
    schedules_build_or_explain(&mut app);
    app.update(); // Startup: the city and the local player come into being
    app
}

/// At most this many cycles are named, and at most this many nodes of each.
const MAX_CYCLES_SHOWN: usize = 3;
const MAX_NODES_PER_CYCLE: usize = 12;

/// **The guard that stands between a schedule cycle and the user's desktop** — `B-030`.
///
/// Builds every schedule in the app **before** the first `update()` and, if one does not
/// build, panics with a message of a **fixed maximum size**.
///
/// Why it has to exist at all: `Schedule::run` answers a build error by calling
/// `ScheduleBuildError::to_string`, and for a dependency cycle that is
/// `dependency_cycle_to_string` (`bevy_ecs-0.19.0/src/schedule/error.rs:174-206`), which
/// writes **one block per simple cycle** into a single `String`. The number of simple cycles
/// in a strongly connected component is not linear in anything you can look at — the one
/// measured on 2026-09-01 was **2 290 028** cycles over ~10 nodes, the `String` doubled its
/// way to a **4 966 055 936-byte** `realloc`, and the kernel killed the test binary at
/// 25 GB anon-rss. **The cycle is a one-line mistake; the 25 GB is bevy's report about it.**
///
/// So this reads the same error and prints at most [`MAX_CYCLES_SHOWN`] cycles of at most
/// [`MAX_NODES_PER_CYCLE`] nodes: bounded by a constant, whatever the graph does. It is not a
/// smaller constant on an unbounded thing — the unbounded thing never runs.
///
/// It is cheap in the green case: `initialize` is idempotent and `app.update()` would call it
/// one line later anyway.
fn schedules_build_or_explain(app: &mut App) {
    use bevy::ecs::schedule::graph::DiGraphToposortError;
    use bevy::ecs::schedule::{ScheduleBuildError, Schedules};

    let labels: Vec<_> =
        app.world().resource::<Schedules>().iter().map(|(_, s)| s.label()).collect();
    for label in labels {
        app.world_mut().schedule_scope(label, |world, schedule| {
            let Err(error) = schedule.initialize(world) else {
                return;
            };
            let graph = schedule.graph();
            let mut lines = format!("schedule {label:?} does not build");
            match &error {
                ScheduleBuildError::DependencySort(DiGraphToposortError::Cycle(cycles)) => {
                    let _ = write!(lines, " — {} before/after cycle(s):", cycles.len());
                    for cycle in cycles.iter().take(MAX_CYCLES_SHOWN) {
                        let names: Vec<_> = cycle
                            .iter()
                            .take(MAX_NODES_PER_CYCLE)
                            .map(|n| graph.get_node_name(n))
                            .collect();
                        let _ = write!(lines, "\n  len {}: {names:?}", cycle.len());
                    }
                }
                ScheduleBuildError::FlatDependencySort(DiGraphToposortError::Cycle(cycles)) => {
                    let _ = write!(lines, " — {} flat before/after cycle(s):", cycles.len());
                    for cycle in cycles.iter().take(MAX_CYCLES_SHOWN) {
                        let names: Vec<_> = cycle
                            .iter()
                            .take(MAX_NODES_PER_CYCLE)
                            .map(|n| graph.get_node_name(&(*n).into()))
                            .collect();
                        let _ = write!(lines, "\n  len {}: {names:?}", cycle.len());
                    }
                }
                // Every other build error formats a bounded message on its own.
                other => {
                    let _ = write!(lines, " — {other}");
                }
            }
            panic!("{lines}\n(truncated on purpose — see `schedules_build_or_explain`, B-030)");
        });
    }
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

/// The one local player. Not `.single()` — every player is one of many (§6 rule 3).
fn me(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

fn player_id(app: &App, e: Entity) -> PlayerId {
    *app.world().get::<PlayerId>(e).expect("every player has a stable id")
}

fn hook(app: &App, e: Entity) -> Hook {
    *app.world().get::<Hook>(e).expect("every player carries both arms")
}

fn arm_state(app: &App, e: Entity, side: Side) -> HookState {
    hook(app, e).arm(side).state
}

fn tip(app: &App, e: Entity, side: Side) -> Vec3 {
    hook(app, e).arm(side).tip_m
}

/// Where the tip starts and comes home — `player.eye_height_m` above the origin between the
/// feet, the same point `vector::aim` shoots its ray from.
fn hand(app: &App, e: Entity) -> Vec3 {
    let eye = data(app).game.player.eye_height_m;
    app.world().get::<Transform>(e).expect("the player has a transform").translation
        + Vec3::Y * eye
}

/// Puts an anchorable body into the spatial index and returns its stable id.
///
/// Directly, not through `world::index::maintain_index` — that one is a stub belonging to
/// `T-036a`. High ids so that nothing collides with the city once it is filled.
fn put_body(app: &mut App, id: u32, center_m: Vec3, half_size_m: Vec3) -> BodyId {
    let body = BodyId(id);
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m,
        half_size_m,
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    body
}

/// Aims the player at a point on a body. `anchorable` is the `F-003` tag.
fn aim_at(app: &mut App, e: Entity, point_m: Vec3, body: BodyId, anchorable: bool) {
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(point_m),
        body: Some(body),
        anchorable,
    }));
}

fn aim_at_nothing(app: &mut App, e: Entity) {
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint::default()));
}

/// The key that fires the hook on `side` — the **one** place in this file that knows a binding.
///
/// ⚠️ Depends on `Q` -> `HOOK_LEFT` and `E` -> `HOOK_RIGHT` (`src/net/local.rs::read_input`).
/// Until 2026-08-10 the hooks sat on the mouse; `LMB`/`RMB` are the **blades** now, so pressing
/// them fired no hook at all. Rebind there, change these two lines, and the file follows.
fn hook_key(side: Side) -> KeyCode {
    match side {
        Side::Left => KeyCode::KeyQ,
        Side::Right => KeyCode::KeyE,
    }
}

/// Presses a **real** key — the same input a human triggers and the same one the `--script`
/// driver uses (`src/net/local.rs` maps `Q`/`E` onto the two arms).
fn press(app: &mut App, side: Side) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(hook_key(side));
}

fn let_go(app: &mut App, side: Side) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(hook_key(side));
}

/// A second player, without the `LocalPlayer` marker — the way a team mate arrives later.
fn second_player(app: &mut App, pos: Vec3) -> Entity {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, pos, false);
    *world.resource_mut::<IdCounter>() = ids;
    app.update();
    e
}

/// Posts an intent for a **remote** player through the one channel. Nobody writes an `Intent`
/// straight onto a player (`src/net/mod.rs`) — not even a test.
fn post(app: &mut App, id: PlayerId, buttons: Buttons) {
    let tick = app.world().resource::<Tick>().0;
    app.world_mut()
        .resource_mut::<Inbox>()
        .push(id, Intent { buttons, tick, ..default() }, tick);
}

/// Runs until the arm reaches `Anchored`, at most `limit` ticks. Returns the tick count.
fn ticks_until_anchored(app: &mut App, e: Entity, side: Side, limit: u64) -> u64 {
    for n in 1..=limit {
        app.update();
        if arm_state(app, e, side).is_anchored() {
            return n;
        }
    }
    panic!("the hook did not anchor within {limit} ticks — it is {:?}", arm_state(app, e, side));
}

/// Runs until the arm is back at `Idle`, at most `limit` ticks. Returns the tick count.
fn ticks_until_idle(app: &mut App, e: Entity, side: Side, limit: u64) -> u64 {
    for n in 1..=limit {
        app.update();
        if arm_state(app, e, side) == HookState::Idle {
            return n;
        }
    }
    panic!("the tip did not come home within {limit} ticks — it is {:?}", arm_state(app, e, side));
}

/// Lets the player land and come to rest, so that `hand()` is a stable number.
fn settle(app: &mut App) {
    ticks(app, 180);
}

// ---------------------------------------------------------------------------------------
// 1. The trigger — an edge, not a hold
// ---------------------------------------------------------------------------------------

#[test]
fn f001_a_press_fires_once_and_holding_does_not_fire_again() {
    // Autofire is the difference between a hook and a machine gun. The button is held for a
    // full second here; exactly one shot may come out of that.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_001, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);

    press(&mut app, Side::Left);
    ticks(&mut app, 60); // one second of holding

    let log = app.world().resource::<HookLog>();
    assert_eq!(
        log.anchored.len(),
        1,
        "one press, {} anchor messages — the edge detection is not working",
        log.anchored.len()
    );
    assert!(
        log.released.is_empty(),
        "nothing was let go, and yet: {:?}",
        log.released
    );
    assert!(
        arm_state(&app, e, Side::Left).is_anchored(),
        "after a second of holding the left arm is {:?}",
        arm_state(&app, e, Side::Left)
    );
    assert_eq!(
        arm_state(&app, e, Side::Right),
        HookState::Idle,
        "the right arm was never touched"
    );
}

#[test]
fn f001_the_edge_belongs_to_the_player_and_not_to_the_system() {
    // THE test against `Local<Buttons>`. Player 1 holds his button from the start; player 2
    // presses the same button five ticks later. With one previous state shared by the system,
    // player 2's press falls into a `just_pressed` that already contains the button — and he
    // never fires. Each of the two has to fire exactly once, on his own edge.
    let mut app = app();
    let first = me(&mut app);
    let second = second_player(&mut app, Vec3::new(20.0, 2.0, 0.0));
    let second_id = player_id(&app, second);
    settle(&mut app);

    let body_a = put_body(&mut app, 90_002, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    let body_b = put_body(&mut app, 90_003, Vec3::new(20.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, first, Vec3::new(0.0, 1.6, -28.0), body_a, true);
    aim_at(&mut app, second, Vec3::new(20.0, 1.6, -28.0), body_b, true);

    press(&mut app, Side::Left);
    ticks(&mut app, 5);
    post(&mut app, second_id, Buttons::HOOK_LEFT);
    ticks(&mut app, 40);

    assert!(
        arm_state(&app, first, Side::Left).is_anchored(),
        "player 1 is {:?}",
        arm_state(&app, first, Side::Left)
    );
    assert!(
        arm_state(&app, second, Side::Left).is_anchored(),
        "player 2 is {:?} — his edge was eaten by player 1's held button",
        arm_state(&app, second, Side::Left)
    );

    let log = app.world().resource::<HookLog>();
    assert_eq!(log.anchored.len(), 2, "two players, two shots: {:?}", log.anchored);

    // And the previous state is really a component of each player's own.
    let first_prev = *app.world().get::<PrevButtons>(first).expect("player 1 has PrevButtons");
    let second_prev = *app.world().get::<PrevButtons>(second).expect("player 2 has PrevButtons");
    assert!(first_prev.0.contains(Buttons::HOOK_LEFT));
    assert!(second_prev.0.contains(Buttons::HOOK_LEFT));

    // Let player 2 go and check that player 1 keeps holding — one player's release must not
    // be another's.
    post(&mut app, second_id, Buttons::NONE);
    ticks(&mut app, 2);
    assert!(
        arm_state(&app, first, Side::Left).is_anchored(),
        "player 1 lost his hook when player 2 let go"
    );
    assert!(
        !arm_state(&app, second, Side::Left).is_anchored(),
        "player 2 let go and is still hanging"
    );
}

// ---------------------------------------------------------------------------------------
// 2. The flight — the number comes out of the file
// ---------------------------------------------------------------------------------------

/// Fires once at a body 28 m away and returns the number of ticks up to `Anchored`.
fn measure_flight(speed_m_s: Option<f32>, flight_max_s: Option<f32>) -> (u64, f32, f32) {
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    if let Some(speed) = speed_m_s {
        app.world_mut().resource_mut::<GameData>().game.vector.hook_speed_m_s = speed;
    }
    // ⚠️ Since 2026-08-20 there are TWO keys in a flight time and `hook_flight_max_s` is the
    // one that wins on a long shot. Every claim below is about `hook_speed_m_s`, so the caller
    // pushes the ceiling out of the way rather than pretending it is not there.
    if let Some(max_s) = flight_max_s {
        app.world_mut().resource_mut::<GameData>().game.vector.hook_flight_max_s = max_s;
    }
    let speed = data(&app).game.vector.hook_speed_m_s;
    let hz = data(&app).game.simulation_hz as f32;

    let target = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_004, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);

    // The distance really flown: from the hand, not from the origin between the feet.
    let distance_m = (target - hand(&app, e)).length();

    press(&mut app, Side::Left);
    // The first update fires; from the second on the tip moves.
    app.update();
    assert!(
        matches!(arm_state(&app, e, Side::Left), HookState::Flying { .. }),
        "the shot did not leave: {:?}",
        arm_state(&app, e, Side::Left)
    );
    let flown = ticks_until_anchored(&mut app, e, Side::Left, 600);

    (flown, distance_m, distance_m / speed * hz)
}

#[test]
fn f001_the_flight_time_comes_out_of_the_file_and_not_out_of_the_code() {
    // The criterion verbatim: flight time = distance / hook_speed_m_s * 60, within one tick.
    // The second half of the test is what makes it red when somebody writes the 90 into the
    // Rust: at a third of the speed the flight has to take three times as long.
    let (at_file_value, distance_m, expected) = measure_flight(None, Some(1000.0));
    assert!(
        (at_file_value as f32 - expected).abs() <= 1.0,
        "{distance_m:.3} m took {at_file_value} ticks, the file allows {expected:.2} ± 1 \
         (game.ron: vector.hook_speed_m_s)"
    );

    let third = defeated_by_titan::data::GameData::load(std::path::Path::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data"),
    ))
    .game
    .vector
    .hook_speed_m_s
        / 3.0;
    let (slowed, _, expected_slow) = measure_flight(Some(third), Some(1000.0));
    assert!(
        (slowed as f32 - expected_slow).abs() <= 1.0,
        "at {third} m/s the flight took {slowed} ticks instead of {expected_slow:.2} — \
         the speed is hard-coded somewhere instead of being read from the file"
    );
    assert!(
        slowed > at_file_value * 2,
        "a third of the speed took {slowed} ticks against {at_file_value} — the number is \
         not being read at all"
    );
}

#[test]
fn f001_the_tip_starts_in_the_hand_and_flies_towards_the_anchor() {
    // Without this the flight time above would be right for a tip that teleports.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // Far enough out that the five full steps below all fit inside the flight. The **last**
    // step of any flight is a partial one — the tip stops on the anchor instead of overshooting
    // it — so a target that is not a whole number of steps away turns this into a measurement
    // of the arrival. It was: at `hook_speed_m_s: 160` the old fixed 28 m was 10.5 steps, and
    // `W1`'s 160 -> 500 m/s made it 3.4. Derived from the file now, so the next speed change
    // cannot do it again.
    //
    // ⚠️ And **not further out than `hook_speed_m_s * hook_flight_max_s`** (2026-08-20): past
    // that distance the ceiling decides the step and the tip moves more than the speed key
    // says, which is the ceiling working and not a bug. 50 m today — eight steps would have
    // been 66.7 m and this test would have been measuring the wrong key.
    let d = data(&app);
    let dt = 1.0 / d.game.simulation_hz as f32;
    let distance_m =
        (d.game.vector.hook_speed_m_s * dt * 8.0).min(d.game.vector.hook_speed_m_s * d.game.vector.hook_flight_max_s);
    let per_tick_m = defeated_by_titan::vector::hook::flight_per_tick_m(&d.game.vector, distance_m, dt);
    let steps = ((distance_m / per_tick_m).floor() as usize).saturating_sub(1).min(5);
    let target = Vec3::new(0.0, 1.6, -distance_m);
    let body =
        put_body(&mut app, 90_005, Vec3::new(0.0, 1.6, -distance_m - 4.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);

    let start = hand(&app, e);
    press(&mut app, Side::Left);
    app.update();
    assert!(
        (tip(&app, e, Side::Left) - start).length() < 1e-3,
        "the tip starts at {:?} instead of in the hand at {start:?}",
        tip(&app, e, Side::Left)
    );

    let mut last = tip(&app, e, Side::Left);
    assert!(steps >= 3, "only {steps} full steps fit into this flight — it proves nothing");
    for _ in 0..steps {
        app.update();
        let now = tip(&app, e, Side::Left);
        let step = (now - last).length();
        assert!(
            (step - per_tick_m).abs() < 1e-3,
            "the tip moved {step:.4} m in one tick, the file says {per_tick_m:.4} m"
        );
        assert!(
            (target - now).length() < (target - last).length(),
            "the tip is not getting closer to the anchor"
        );
        last = now;
    }
}

// ---------------------------------------------------------------------------------------
// 3. Two arms, independently
// ---------------------------------------------------------------------------------------

#[test]
fn f001_both_hooks_anchor_and_release_independently() {
    // The acceptance of `F-001` in one test: "beide Haken lassen sich unabhaengig setzen und
    // loesen". Two different carriers, so that "independent" is not just two entries in the
    // same array.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let left_body = put_body(&mut app, 90_006, Vec3::new(-20.0, 1.6, -20.0), Vec3::splat(4.0));
    let right_body = put_body(&mut app, 90_007, Vec3::new(20.0, 1.6, -20.0), Vec3::splat(4.0));

    let left_point = Vec3::new(-16.0, 1.6, -20.0);
    aim_at(&mut app, e, left_point, left_body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    let right_point = Vec3::new(16.0, 1.6, -20.0);
    aim_at(&mut app, e, right_point, right_body, true);
    press(&mut app, Side::Right);
    ticks_until_anchored(&mut app, e, Side::Right, 300);

    assert_eq!(hook(&app, e).anchored_count(), 2, "both arms hold");
    match arm_state(&app, e, Side::Left) {
        HookState::Anchored { body, .. } => assert_eq!(body, left_body, "left hangs on the left"),
        other => panic!("the left arm is {other:?}"),
    }
    match arm_state(&app, e, Side::Right) {
        HookState::Anchored { body, .. } => {
            assert_eq!(body, right_body, "right hangs on the right")
        }
        other => panic!("the right arm is {other:?}"),
    }

    // Let go of the LEFT one only.
    let_go(&mut app, Side::Left);
    app.update();
    assert!(
        !arm_state(&app, e, Side::Left).is_anchored(),
        "the left arm did not let go"
    );
    assert!(
        arm_state(&app, e, Side::Right).is_anchored(),
        "letting the left one go took the right one with it — that is one state for two arms"
    );
    assert_eq!(hook(&app, e).anchored_count(), 1);

    let released: Vec<_> = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "exactly one release: {released:?}");
    assert_eq!(released[0].side, Side::Left);
    assert_eq!(released[0].reason, ReleaseReason::Released);

    // And now the right one, on its own.
    let_go(&mut app, Side::Right);
    app.update();
    assert_eq!(hook(&app, e).anchored_count(), 0);
    assert_eq!(app.world().resource::<HookLog>().released.len(), 2);
}

#[test]
fn f001_letting_go_brings_the_tip_home_at_the_retract_speed_from_the_file() {
    // The fourth state is not decoration: the arm is not ready again until the tip is back.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let target = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_008, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    let distance_m = (tip(&app, e, Side::Left) - hand(&app, e)).length();
    let speed = data(&app).game.vector.hook_retract_speed_m_s;
    let hz = data(&app).game.simulation_hz as f32;
    let expected = distance_m / speed * hz;

    let_go(&mut app, Side::Left);
    app.update(); // the release itself: Anchored -> Retracting
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);

    let home = ticks_until_idle(&mut app, e, Side::Left, 600);
    assert!(
        (home as f32 - expected).abs() <= 1.0,
        "the tip needed {home} ticks for {distance_m:.2} m, the file allows {expected:.2} ± 1 \
         (game.ron: vector.hook_retract_speed_m_s)"
    );
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() < 1e-3,
        "the tip came to rest at {:?} instead of in the hand",
        tip(&app, e, Side::Left)
    );
}

// ---------------------------------------------------------------------------------------
// 4. The three reasons a hook lets go without being asked
// ---------------------------------------------------------------------------------------

#[test]
fn f001_a_hook_whose_carrier_disappears_releases() {
    // `F-029`: "releases with feedback when the titan dies", `T-020`: an unloaded area. A hook
    // that keeps hanging on a body that no longer exists is a rope into nothing — and the
    // reason a hook stores a `BodyId` and never an `Entity`.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_009, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    // The carrier goes, exactly the way `world::index` reports it.
    let tick = app.world().resource::<Tick>().0;
    app.world_mut().resource_mut::<SpatialIndex>().remove(body);
    app.world_mut().write_message(BodyGone { body, tick });
    app.update();

    assert!(
        !arm_state(&app, e, Side::Left).is_anchored(),
        "the carrier is gone and the hook is still hanging on it"
    );
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    assert_eq!(
        released[0].reason,
        ReleaseReason::BodyGone,
        "the reason is not log prose — `hud` and `sound` tell from it whether the rope tore"
    );
}

#[test]
fn f001_an_overextended_rope_releases_one_tick_later() {
    // The rope is never fought against a wall: the integrator pays it out and sets the flag,
    // this system reads the flag in the NEXT tick and lets go. One tick of lag is the price
    // for `Hook` having exactly one writer.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_010, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    app.world_mut().get_mut::<RopeLength>(e).expect("the player has a rope").overextended
        [Side::Left.index()] = true;
    app.update();

    assert!(!arm_state(&app, e, Side::Left).is_anchored(), "the wall won and the hook held");
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    assert_eq!(released[0].reason, ReleaseReason::Overextended);
    assert_eq!(released[0].side, Side::Left);
}

#[test]
fn f001_a_shot_at_nothing_anchorable_reports_no_anchor() {
    // `F-023` in so many words: no hooking through walls. A ray that ends on an untagged wall
    // is a hit and not an anchor — and the trigger has to say so, or nobody knows why nothing
    // happened.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // 1. Nothing in range at all.
    aim_at_nothing(&mut app, e);
    press(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Idle, "a miss must not fly");
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    // Since `F-028` the reason carries **why**; which of the four it is has its own test
    // below, and this one stays about the state machine.
    assert!(
        matches!(released[0].reason, ReleaseReason::NoAnchor(_)),
        "{:?}",
        released[0].reason
    );

    // 2. A hit on a body that is not tagged as an anchor surface.
    let_go(&mut app, Side::Left);
    let wall = put_body(&mut app, 90_011, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), wall, false);
    ticks(&mut app, 2);
    press(&mut app, Side::Right);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Right), HookState::Idle);
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 2, "{released:?}");
    assert_eq!(
        released[1].reason,
        ReleaseReason::NoAnchor(MissReason::SurfaceHoldsNothing),
        "an untagged wall is a hit that holds nothing, and the player has to be told which"
    );
    assert_eq!(released[1].side, Side::Right);
    assert!(
        app.world().resource::<HookLog>().anchored.is_empty(),
        "something anchored although nothing was anchorable"
    );
}

// ---------------------------------------------------------------------------------------
// 4b. §1A requirement 1 — "festhaken … was instant sein soll"
// ---------------------------------------------------------------------------------------

#[test]
fn f002_a_refire_during_retract_flies_again_within_one_tick() {
    // The user, 2026-08-12: „wenn ich mit seilen festhake (was instant sein soll)".
    //
    // Until this test existed a shot left **only** from `Idle`, so every release opened a
    // lockout of `rope_length / vector.hook_retract_speed_m_s` in which the trigger did
    // nothing at all. Worse than the wait: the trigger pull was **swallowed**. Pressing
    // during the retract consumed the edge, and by the time the arm reached `Idle` the button
    // was already held — `just_pressed` is an edge, so the arm sat there ready and did not
    // fire until the player let go and pressed a *second* time.
    //
    // What is measured is that number: how many ticks pass between the trigger going down
    // and a shot leaving. The acceptance is **≤ 1**.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // A long rope on purpose. At 500 m/s and 60 Hz the tip travels 8.33 m per tick, so a
    // short rope would hide the lockout inside the rounding — the bug only shows at the
    // ranges `game.ron` now allows (`hook_range_m: 500`).
    let far = Vec3::new(0.0, 1.6, -180.0);
    let body = put_body(&mut app, 90_020, Vec3::new(0.0, 1.6, -200.0), Vec3::splat(20.0));
    aim_at(&mut app, e, far, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 600);

    let distance_m = (tip(&app, e, Side::Left) - hand(&app, e)).length();
    let d = data(&app);
    let lockout_ticks =
        distance_m / d.game.vector.hook_retract_speed_m_s * d.game.simulation_hz as f32;
    assert!(
        lockout_ticks > 10.0,
        "this test measures a lockout of {lockout_ticks:.1} ticks — too short to prove \
         anything. Move the anchor further out."
    );

    // The release itself: one tick, `Anchored -> Retracting`. That tick is not the lockout,
    // it is the release, and it is the one tick the acceptance allows.
    let_go(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() > 20.0,
        "the tip is already home — then there is no retract to cancel and the test proves \
         nothing"
    );

    // A fresh target for the second shot, so that "it flew again" cannot be confused with
    // "it never let go".
    let near = Vec3::new(0.0, 1.6, -40.0);
    let second = put_body(&mut app, 90_021, Vec3::new(0.0, 1.6, -50.0), Vec3::splat(10.0));
    aim_at(&mut app, e, near, second, true);

    // The trigger goes down on the very next tick and stays down — the way a player holds it.
    press(&mut app, Side::Left);
    let mut blocked = 0u64;
    let mut flying = None;
    for _ in 0..(lockout_ticks as u64 + 30) {
        app.update();
        if let HookState::Flying { target_m, body } = arm_state(&app, e, Side::Left) {
            flying = Some((target_m, body));
            break;
        }
        if arm_state(&app, e, Side::Left).is_anchored() {
            panic!("the arm skipped `Flying` — a shot that never flies is not a shot");
        }
        blocked += 1;
    }
    let (target_m, on) = flying.unwrap_or_else(|| {
        panic!(
            "the trigger was held for {blocked} ticks and no shot left. The lockout is \
             {lockout_ticks:.1} ticks long and it swallows the edge: the arm reaches `Idle` \
             with the button already down, and `just_pressed` never comes back"
        )
    });
    assert!(
        blocked <= 1,
        "{blocked} ticks passed between the trigger going down and the shot leaving \
         (lockout: {lockout_ticks:.1} ticks). §1A requirement 1 allows 1"
    );

    // `B-001` — the state that flies still carries its carrier. A `Flying` without a `BodyId`
    // is how a miss became a flight and cost this project a day.
    assert_eq!(on, second, "the second shot flies at the second body");
    assert!(
        (target_m - near).length() < 1e-3,
        "the refire kept the OLD target {target_m:?} instead of {near:?}"
    );
    // The retract was cancelled, not queued: the new flight starts in the hand, so its flight
    // time is `distance / hook_speed_m_s` and nothing else.
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() <= d.game.vector.hook_speed_m_s
            / d.game.simulation_hz as f32
            + 1e-3,
        "the second shot started at {:?}, which is not the hand {:?}",
        tip(&app, e, Side::Left),
        hand(&app, e)
    );

    // And the other half of the same rule: a refire that finds nothing anchorable must not
    // strand the arm. It reports `NoAnchor` and keeps coming home.
    ticks_until_anchored(&mut app, e, Side::Left, 600);
    let_go(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);
    let before = app.world().resource::<HookLog>().released.len();
    aim_at_nothing(&mut app, e);
    press(&mut app, Side::Left);
    app.update();
    assert_eq!(
        arm_state(&app, e, Side::Left),
        HookState::Retracting,
        "a refire at nothing turned the retract into something else"
    );
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), before + 1, "{released:?}");
    assert!(
        matches!(released[before].reason, ReleaseReason::NoAnchor(_)),
        "{:?}",
        released[before].reason
    );
}

/// `F-028` — **the four failures are four different answers, in the running game.**
///
/// The user, 2026-08-18: *„teilweise kann man gar nicht usen weil keine ahnung wieso."*
/// `B-007` is why: a titan carries no `Body`, so a ray that ends on him is a hit that holds
/// nothing — and because he is solid, the wall behind him is unreachable too. Both used to
/// arrive as the same silent `NoAnchor` with the arm sitting in `Idle`.
///
/// The three cases the acceptance names are driven here through the real message path, and
/// the pair that had no way of being told apart at all — *open sky* against *too far* — is
/// driven with a real avian collider 900 m out, against a `hook_range_m` of 500.
#[test]
fn f028_a_failed_pull_says_which_of_the_four_it_was() {
    use avian3d::prelude::{Collider, RigidBody};
    use defeated_by_titan::shared::Body;

    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);
    let range_m = data(&app).game.vector.hook_range_m;

    // ---- 1. A hit on a surface that carries no anchor. `B-007`'s titan, and every untagged
    // wall. The probe is deliberately irrelevant here: a near hit may never be talked over by
    // something further out, or "aim past him" turns into "come closer".
    let wall = put_body(&mut app, 91_001, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), wall, false);
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::SurfaceHoldsNothing)),
        "{released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 2. A surface that holds, with no carrier to hang it on. `B-001` was exactly this,
    // and it is a world fault, not a player error — so it may not read as one.
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(Vec3::new(0.0, 1.6, -28.0)),
        body: None,
        anchorable: true,
    }));
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::NoCarrier)),
        "{released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 3. Nothing at all on that line. High over the district, so the city is not under
    // the probe: the answer has to be "turn", not "come closer".
    app.world_mut().entity_mut(e).get_mut::<Transform>().expect("a player has a transform")
        .translation = Vec3::new(0.0, 400.0, 0.0);
    aim_at_nothing(&mut app, e);
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::NothingInRange)),
        "400 m over Ashgate with nothing on the line the answer has to be open sky: {released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 4. The same empty aim, with a real anchorable wall 900 m out — beyond the file's
    // own `hook_range_m`. **This is the pair that had no way of being told apart**, and it is
    // the difference between "turn around" and "get closer".
    assert!(range_m < 900.0, "hook_range_m is {range_m} m — this test needs it under 900");
    let dir = app.world().get::<Intent>(e).expect("a player has an intent").look_dir();
    let far_m = hand(&app, e) + dir * 900.0;
    app.world_mut().spawn((
        Name::new("f028_far_wall"),
        Body { half_size_m: Vec3::splat(30.0), mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE) },
        BodyId(91_002),
        RigidBody::Static,
        Collider::cuboid(60.0, 60.0, 60.0),
        Transform::from_translation(far_m),
    ));
    ticks(&mut app, 4); // avian rebuilds its static tree inside the physics step
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::OutOfReach)),
        "an anchorable wall 900 m out against a {range_m} m reach has to read as OUT OF \
         REACH, not as open sky: {released:?}"
    );

    // And through all four the arm never left `Idle` — the feedback is the whole change, the
    // state machine is untouched (`F-029` is what would make a titan hold, and it is unbuilt).
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Idle);
    assert!(
        app.world().resource::<HookLog>().anchored.is_empty(),
        "something anchored although nothing was anchorable"
    );
}

// ---------------------------------------------------------------------------------------
// 5. The anchor is stored in the carrier's frame, not in the world's
// ---------------------------------------------------------------------------------------

#[test]
fn f001_the_anchor_rides_along_when_its_carrier_moves() {
    // `HookState::Anchored` keeps `local_m` and a `BodyId`, not a world position and not an
    // `Entity`. From `F-029` on (anchors on titan limbs) that is the difference between a hook
    // that holds and one that hangs where the shoulder used to be.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let center = Vec3::new(0.0, 1.6, -32.0);
    let point = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_012, center, Vec3::splat(4.0));
    aim_at(&mut app, e, point, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    match arm_state(&app, e, Side::Left) {
        HookState::Anchored { local_m, body: on } => {
            assert_eq!(on, body);
            assert!(
                (local_m - (point - center)).length() < 1e-3,
                "local_m is {local_m:?} instead of {:?}",
                point - center
            );
        }
        other => panic!("the arm is {other:?}"),
    }
    assert!((tip(&app, e, Side::Left) - point).length() < 1e-3);

    // The carrier moves five meters to the side.
    let moved = center + Vec3::X * 5.0;
    put_body(&mut app, 90_012, moved, Vec3::splat(4.0));
    app.update();
    assert!(
        (tip(&app, e, Side::Left) - (point + Vec3::X * 5.0)).length() < 1e-3,
        "the tip stayed at {:?} while its carrier moved",
        tip(&app, e, Side::Left)
    );
    assert!(arm_state(&app, e, Side::Left).is_anchored(), "and it is still holding");
}


// ===========================================================================================
// `F-025` Bewertungsfunktion / `F-024` Snap auf Q und E
//
// > *„es sollte best match sein"* — the user, 2026-08-18.
//
// **Every assertion below computes its expected value out of the backlog's own weights, by
// hand, and never asks the scoring function what the scoring function thinks.** That is the
// lesson of `docs/FINDINGS.md` FIND-103: a test that puts the same question to the screen and
// to the code passes while both are wrong. So the fixtures here are candidates typed out as
// coordinates, arranged so that four of the five terms are provably equal between them and
// the score difference is one weight times one number a reader can check on paper.
// ===========================================================================================

/// The tuning straight out of `assets/data/game.ron` — no app, no plugins.
fn vector_tuning() -> VectorTuning {
    GameData::load(std::path::Path::new("assets/data")).game.vector
}

/// A context with nothing switched on: no motion, no hooks out, a 20° cone.
fn calm_ctx(look: Vec3) -> ScoreContext {
    ScoreContext {
        eye_m: Vec3::ZERO,
        look,
        velocity_m_s: Vec3::ZERO,
        catch_rad: 20.0_f32.to_radians(),
        in_use: [None, None],
    }
}

fn candidate(point_m: Vec3, id: u32) -> AimCandidate {
    AimCandidate { point_m, body: BodyId(id), distance_m: point_m.length() }
}

/// The five weights are `F-025`'s own numbers, and they are in the file rather than in Rust.
///
/// > *"Faktoren: Winkelabweichung zum Fadenkreuz (Hauptgewicht 45 Prozent), Momentum-Erhalt
/// > (25 Prozent …), Hoehenvorteil relativ zur Bewegungsrichtung (15 Prozent), Distanz im
/// > nutzbaren Mittelbereich (10 Prozent), Abwertung des zuletzt genutzten Punktes (5 Prozent
/// > …). Alle Gewichte liegen in der Config und sind ohne Codeaenderung anpassbar."*
/// > — `docs/backlog/gameplay.ron`, `F-025`
///
/// The numbers here are typed from that line and not read back out of the code (§6 rule 2).
#[test]
fn f025_the_five_weights_are_the_backlogs_own_numbers_and_they_live_in_the_file() {
    let v = vector_tuning();
    assert_eq!(v.assist_score_angle_w, 0.45, "angle deviation to the crosshair");
    assert_eq!(v.assist_score_momentum_w, 0.25, "momentum preservation");
    assert_eq!(v.assist_score_height_w, 0.15, "height advantage");
    assert_eq!(v.assist_score_distance_w, 0.10, "distance in the usable mid-range");
    assert_eq!(v.assist_score_recent_w, 0.05, "devaluation of the point last used");
    // 45 + 25 + 15 + 10 + 5 = 100. The fifth is a **penalty**, so the four rewards are 95 %
    // and the whole budget is only closed once the devaluation is counted in — which is what
    // the backlog's list actually adds up to, and the first version of this assertion got it
    // wrong (it demanded 1.0 of the four and went red on 0.95).
    let budget = v.assist_score_angle_w
        + v.assist_score_momentum_w
        + v.assist_score_height_w
        + v.assist_score_distance_w
        + v.assist_score_recent_w;
    assert!((budget - 1.0).abs() < 1e-6, "the five factors are 100 %, not {budget}");
}

/// 45 %: the point on the crosshair beats the one at the rim of the catch cone **by exactly
/// `assist_score_angle_w`**, with the other four terms held equal by construction.
///
/// Both points sit at the same distance, both at the eye's own height (so the height term is
/// the midpoint for both), and the player stands still (so the momentum term is the midpoint
/// for both). Nothing but the angle can move the number.
#[test]
fn f025_the_angle_term_is_worth_exactly_the_files_forty_five_percent() {
    let v = vector_tuning();
    let ctx = calm_ctx(Vec3::NEG_Z);
    let d = 30.0_f32;
    let on_the_crosshair = candidate(Vec3::new(0.0, 0.0, -d), 1);
    let (sin_c, cos_c) = ctx.catch_rad.sin_cos();
    let at_the_rim = candidate(Vec3::new(d * sin_c, 0.0, -d * cos_c), 2);

    let gap = score_candidate(&v, &ctx, &on_the_crosshair) - score_candidate(&v, &ctx, &at_the_rim);
    assert!(
        (gap - v.assist_score_angle_w).abs() < 1e-5,
        "on the crosshair beats the rim by {gap}, and the file says {}",
        v.assist_score_angle_w
    );
}

/// 25 %: *"bevorzugt Punkte, die die aktuelle Flugbahn fortsetzen statt sie zu bremsen"*.
///
/// Two candidates 90° apart, both 45° off the crosshair, both level with the eye, both at the
/// same distance. The player flies straight at the first one. The momentum term runs
/// `0.5 (1 + cos)`, so the gap is `w · (1.0 − 0.5) = w/2` and nothing else.
#[test]
fn f025_a_point_that_continues_the_flight_beats_one_that_brakes_it() {
    let v = vector_tuning();
    let ctx_look = Vec3::X;
    let d = 30.0_f32;
    let k = std::f32::consts::FRAC_1_SQRT_2;
    let ahead = candidate(Vec3::new(d * k, 0.0, -d * k), 1);
    let across = candidate(Vec3::new(d * k, 0.0, d * k), 2);
    let mut ctx = calm_ctx(ctx_look);
    // Faster than `assist_momentum_min_speed_m_s`, straight at `ahead`.
    ctx.velocity_m_s = (ahead.point_m - ctx.eye_m).normalize() * 25.0;
    ctx.catch_rad = 60.0_f32.to_radians(); // both inside the cone, so the angle term is equal

    let gap = score_candidate(&v, &ctx, &ahead) - score_candidate(&v, &ctx, &across);
    let expected = v.assist_score_momentum_w * 0.5;
    assert!(
        (gap - expected).abs() < 1e-5,
        "continuing the flight is worth {gap}, expected {expected} from the file's {}",
        v.assist_score_momentum_w
    );

    // And a player who is not moving has no trajectory to preserve: the term says nothing and
    // the two points come out level.
    ctx.velocity_m_s = Vec3::ZERO;
    let level = score_candidate(&v, &ctx, &ahead) - score_candidate(&v, &ctx, &across);
    assert!(level.abs() < 1e-5, "standing still, the two points differ by {level}");
}

/// 15 %: the height advantage, and it is symmetric about the eye.
#[test]
fn f025_the_height_term_is_worth_exactly_the_files_fifteen_percent() {
    let v = vector_tuning();
    let ctx = calm_ctx(Vec3::X);
    let d = 20.0_f32;
    let k = std::f32::consts::FRAC_1_SQRT_2;
    let above = candidate(Vec3::new(d * k, d * k, 0.0), 1);
    let below = candidate(Vec3::new(d * k, -d * k, 0.0), 2);
    let mut ctx = ctx;
    ctx.catch_rad = 60.0_f32.to_radians();

    let rise = d * k / v.assist_height_full_m;
    assert!(rise < 1.0, "the fixture has to stay inside the term's saturation");
    let gap = score_candidate(&v, &ctx, &above) - score_candidate(&v, &ctx, &below);
    let expected = v.assist_score_height_w * rise;
    assert!(
        (gap - expected).abs() < 1e-5,
        "up beats down by {gap}, and 0.15 · {rise} is {expected}"
    );
}

/// 5 %, subtracted: *"Abwertung des zuletzt genutzten Punktes … verhindert Pendeln zwischen
/// zwei Punkten"*. The same point, scored twice — once while an arm is hanging on it.
#[test]
fn f025_the_point_you_are_hanging_on_is_devalued_by_exactly_five_percent() {
    let v = vector_tuning();
    let ctx = calm_ctx(Vec3::NEG_Z);
    let held = candidate(Vec3::new(0.0, 0.0, -30.0), 77);

    let mut busy = ctx;
    busy.in_use = [Some(BodyId(77)), None];
    let gap = score_candidate(&v, &ctx, &held) - score_candidate(&v, &busy, &held);
    assert!(
        (gap - v.assist_score_recent_w).abs() < 1e-6,
        "hanging on it costs {gap}, and the file says {}",
        v.assist_score_recent_w
    );

    // A body no arm is on is not devalued — the penalty is about *this* point, not about
    // having a rope out at all.
    let other = candidate(Vec3::new(0.0, 0.0, -30.0), 78);
    assert_eq!(score_candidate(&v, &ctx, &other), score_candidate(&v, &busy, &other));
}

/// `F-024`'s acceptance, first half: *"das System waehlt **nie** einen Punkt hinter dem
/// Spieler, wenn ein brauchbarer vor ihm liegt."*
///
/// And the stronger statement the catch cone buys: it does not choose one **even when nothing
/// usable lies ahead**, because a point outside the cone is not a candidate at all. The
/// forward direction is checked with a dot product the test computes itself.
#[test]
fn f024_never_a_point_behind_the_player() {
    let v = vector_tuning();
    let look = Vec3::NEG_Z;
    let ctx = calm_ctx(look);
    // Behind, and very tempting on every other term: high up and right in the sweet spot of
    // the distance band.
    let behind = candidate(Vec3::new(2.0, 18.0, 30.0), 1);
    let ahead = candidate(Vec3::new(1.0, 0.0, -30.0), 2);

    let picked = pick_best(&v, &ctx, &[behind, ahead], None, 100.0)
        .expect("a usable point lies ahead");
    assert_eq!(picked.body, BodyId(2), "it took the point behind the player");
    assert!(
        (picked.point_m - ctx.eye_m).normalize().dot(look) > 0.0,
        "the chosen point is not in front of the crosshair"
    );

    // Alone, the point behind is still not chosen — the arm keeps free aim instead.
    assert_eq!(
        pick_best(&v, &ctx, &[behind], None, 100.0),
        None,
        "a point 150 deg off the crosshair was accepted as a candidate"
    );
}

/// `F-024`'s three modes are one number, and the number is linear so that he can report it
/// back (*„damit ich testen kann was am besten waere"*).
#[test]
fn f024_the_three_modes_come_out_of_the_strength_knob_alone() {
    let v = vector_tuning();
    let full = v.assist_margin_full;
    // SNAP: the best candidate always wins.
    assert_eq!(required_margin(full, 100.0), 0.0);
    // ASSISTIERT: it has to be better, and how much better is the slider.
    assert!((required_margin(full, 50.0) - full / 2.0).abs() < 1e-6);
    // The lowest the slider can go while still being on at all.
    assert!(required_margin(full, 5.0) > required_margin(full, 95.0));

    // And what the margin *does*: a candidate that is better, but not better enough, loses.
    let ctx = calm_ctx(Vec3::NEG_Z);
    let aimed_at = candidate(Vec3::new(3.0, 0.0, -30.0), 1);
    let slightly_better = candidate(Vec3::new(1.0, 0.0, -30.0), 2);
    let gap = score_candidate(&v, &ctx, &slightly_better) - score_candidate(&v, &ctx, &aimed_at);
    assert!(gap > 0.0 && gap < full, "the fixture needs a small but real improvement, got {gap}");
    assert_eq!(
        pick_best(&v, &ctx, &[slightly_better], Some(aimed_at), 5.0),
        None,
        "a marginal improvement snapped at 5 % strength"
    );
    assert_eq!(
        pick_best(&v, &ctx, &[slightly_better], Some(aimed_at), 100.0)
            .map(|c| c.body),
        Some(BodyId(2)),
        "SNAP has to take the better point"
    );
}

/// `B-007`, and it is the half of the user's complaint that costs him the city: he aims at a
/// wall, a solid body that holds nothing stands in the way, and free aim comes back empty.
///
/// With no incumbent to beat there is nothing to weigh the candidate against, so **any** valid
/// point in the hemisphere wins at **any** non-zero strength. That is the mechanism; the
/// measurement in the real map is `f024_the_assist_reaches_anchors_free_aim_cannot`.
#[test]
fn b007_with_nothing_to_beat_the_assist_wins_at_the_lowest_strength_there_is() {
    let v = vector_tuning();
    let ctx = calm_ctx(Vec3::NEG_Z);
    let beside_the_blocker = candidate(Vec3::new(4.0, 2.0, -25.0), 9);
    assert_eq!(
        pick_best(&v, &ctx, &[beside_the_blocker], None, 5.0)
            .map(|c| c.body),
        Some(BodyId(9))
    );
}

/// The probe fan is the candidate query, and two things have to be true of every ray it casts
/// at every pitch: it stays **inside the catch cone**, and it stays **on its own side**.
///
/// Both are checked with arithmetic this test does itself — `Intent::look_dir` and the
/// horizontal right vector of `docs/conventions.md` — and not by asking `deviation_rad`, which
/// is the function the system uses.
///
/// ⚠️ **"Its own side" is about the SWEEP, not about the two arms.** `F-023`'s fan was retired
/// on 2026-08-23 and `pick_best` no longer filters by hemisphere: both sides are cast, both
/// answers go into one candidate list, and one winner is published to both arms. What this test
/// still holds is that the sweep really covers left *and* right of the crosshair and never
/// leaves the row (`docs/FINDINGS.md` FIND-133).
#[test]
fn f024_every_probe_stays_inside_the_catch_cone_and_on_its_own_side() {
    let v = vector_tuning();
    for catch_deg in [5.0_f32, 12.5, 20.0] {
        let catch_rad = catch_deg.to_radians();
        for yaw_deg in [-170.0_f32, -35.0, 0.0, 91.0, 179.0] {
            for pitch_deg in [-89.0_f32, -45.0, 0.0, 45.0, 89.0] {
                let intent = Intent {
                    yaw: yaw_deg.to_radians(),
                    pitch: pitch_deg.to_radians(),
                    ..default()
                };
                let look = intent.look_dir();
                let right =
                    Vec3::new(intent.yaw.cos(), 0.0, -intent.yaw.sin());
                for side in Side::ALL {
                    let mut seen = 0;
                    for dir in probe_dirs(
                        look_basis(&intent),
                        catch_rad,
                        v.assist_probe_steps,
                        side,
                    ) {
                        seen += 1;
                        let dev = dir.dot(look).clamp(-1.0, 1.0).acos();
                        assert!(
                            dev <= catch_rad + 1e-4,
                            "a probe at yaw {yaw_deg} pitch {pitch_deg} sits {} deg off the \
                             crosshair, the cone is {catch_deg}",
                            dev.to_degrees()
                        );
                        let lateral = right.dot(dir);
                        let wanted = match side {
                            Side::Left => lateral < 0.0,
                            Side::Right => lateral > 0.0,
                        };
                        assert!(
                            wanted,
                            "a {side:?} probe has lateral {lateral} at yaw {yaw_deg} \
                             pitch {pitch_deg}"
                        );
                        assert!(dir.is_finite() && (dir.length() - 1.0).abs() < 1e-4);
                    }
                    assert_eq!(
                        seen, v.assist_probe_steps as usize,
                        "the sweep lost probes"
                    );
                }
            }
        }
    }
    assert!(deviation_rad(Vec3::NEG_Z, Vec3::ZERO).is_infinite());
}

// ---------------------------------------------------------------------------------------
// The same two features in the running game — the settings resource, the real map, the real
// rays. These are the ones that answer "is the knob wired to anything".
// ---------------------------------------------------------------------------------------

/// Both assist knobs at once. A plain resource write — which is itself half of `F-024`'s
/// acceptance (*"Moduswechsel ist ohne Neustart wirksam"*): there is nothing to restart.
fn set_assist(app: &mut App, catch_pct: f32, strength_pct: f32) {
    let mut s = app.world_mut().resource_mut::<PlayerSettings>();
    s.assist_catch_pct = catch_pct;
    s.assist_strength_pct = strength_pct;
}

/// Points the **local** player, through the same absolute override the `--script` driver's
/// `look` command uses. Degrees in, radians on the resource (`docs/conventions.md`).
fn look_at(app: &mut App, yaw_deg: f32, pitch_deg: f32) {
    *app.world_mut().resource_mut::<LookOverride>() =
        LookOverride(Some((yaw_deg.to_radians(), pitch_deg.to_radians())));
    app.update();
}

fn arms_of(app: &App, e: Entity) -> [AimPoint; 2] {
    app.world().get::<ArmAim>(e).expect("every player carries an ArmAim from tick 1").arms
}

/// `F-016`'s own definition, and the safest regression guard this round has: **at 0 % the game
/// aims exactly as it did before the assist existed.**
///
/// > *"Bei 0 verhaelt sich das System wie reines freies Zielen."* — `docs/backlog/gameplay.ron`,
/// > `F-016`
///
/// Not "within a tolerance": the two knobs are `AND`ed by `PlayerSettings::assist_is_on`, so
/// with either one at zero `vector::aim` never casts a probe ray and never calls a scoring
/// function. The assertion is therefore `assert_eq!` on the whole [`AimPoint`], bit for bit.
#[test]
fn f016_at_zero_percent_the_aim_is_bit_for_bit_the_one_the_game_had_before() {
    let mut app = app();
    settle(&mut app);
    let e = me(&mut app);
    set_assist(&mut app, 0.0, 0.0);
    look_at(&mut app, 25.0, -8.0);
    ticks(&mut app, 40);

    let baseline = arms_of(&app, e);
    ticks(&mut app, 10);
    assert_eq!(
        arms_of(&app, e),
        baseline,
        "the fixture is not settled — a standing player's aim still moves, so nothing below \
         would mean anything"
    );

    // Full reach, no strength: `F-024`'s FREI. Nothing may move.
    set_assist(&mut app, 100.0, 0.0);
    ticks(&mut app, 10);
    assert_eq!(arms_of(&app, e), baseline, "reach at 100 % moved the aim while strength was 0");

    // Full strength, no reach: the catch cone is 0 deg wide, which is free aim again.
    set_assist(&mut app, 0.0, 100.0);
    ticks(&mut app, 10);
    assert_eq!(arms_of(&app, e), baseline, "strength at 100 % moved the aim while reach was 0");

    // And one click of each is enough to be a different feature — otherwise the two tests
    // above would pass on a system that is simply never called.
    set_assist(&mut app, 100.0, 100.0);
    ticks(&mut app, 10);
    let snapped = arms_of(&app, e);
    assert!(
        snapped != baseline || snapped.iter().all(|a| a.anchorable),
        "SNAP changed nothing anywhere, and free aim was not already on an anchor — the \
         assist is not wired to the rays at all"
    );
}

/// `F-024`'s acceptance, second half: *"Moduswechsel ist ohne Neustart wirksam"* — and
/// `B-007`'s measurement, because the two are the same run.
///
/// The sweep looks for a direction in the **shipped** map where free aim leaves an arm on
/// something that holds nothing — a solid body without `ANCHORABLE`, which is exactly the
/// shape a titan has (`docs/BUGS.md` B-007: no `shared::Body`, and he blocks the ray). Then it
/// switches to SNAP **in the middle of the run** and gives the game **one** tick.
///
/// The numbers it prints are the finding, not the assertion: how many of the swept directions
/// leave an arm with no anchor under free aim, and how many of those the assist rescues.
#[test]
fn f024_the_mode_switch_bites_within_one_tick_and_reaches_what_free_aim_cannot() {
    let mut app = app();
    settle(&mut app);
    let e = me(&mut app);

    let mut swept = 0;
    let mut dead_free = 0;
    let mut rescued = 0;
    let mut first_rescue: Option<(f32, f32, Side)> = None;

    for yaw_deg in (0..360).step_by(15) {
        for pitch_deg in [-20.0_f32, -5.0, 10.0, 25.0] {
            set_assist(&mut app, 0.0, 0.0);
            look_at(&mut app, yaw_deg as f32, pitch_deg);
            ticks(&mut app, 6);
            let free = arms_of(&app, e);

            set_assist(&mut app, 100.0, 100.0);
            app.update(); // ONE tick, no restart
            let snap = arms_of(&app, e);

            for side in Side::ALL {
                swept += 1;
                let i = side.index();
                if !free[i].anchorable {
                    dead_free += 1;
                    if snap[i].anchorable {
                        rescued += 1;
                        first_rescue.get_or_insert((yaw_deg as f32, pitch_deg, side));
                    }
                }
                // Whatever it chose, it is a point in front of the player and it holds.
                if snap[i].anchorable {
                    assert!(snap[i].point_m.is_some() && snap[i].body.is_some());
                }
            }
        }
    }

    println!(
        "B-007 / F-024 sweep: {swept} arm-directions, {dead_free} with no anchor under free \
         aim, {rescued} of those rescued by SNAP within one tick ({:.1} %), first at {:?}",
        100.0 * rescued as f32 / dead_free.max(1) as f32,
        first_rescue
    );
    assert!(dead_free > 0, "the shipped map offers no direction where free aim comes back empty");
    assert!(
        rescued > 0,
        "SNAP rescued none of the {dead_free} directions where free aim found no anchor — \
         the mode switch is not effective without a restart"
    );
}

/// `F-025`'s acceptance is about a **chain**: *"Ein geuebter Spieler kann bei aktivem Snap eine
/// Bahn ueber 5 Wechsel beschleunigen."* The half of it this test can measure honestly is the
/// mechanism underneath: with the assist on, the point an arm is sent to **continues the
/// flight** more often than the point free aim would have found.
///
/// Measured in the real map, on a moving player, over a sweep of directions — and the
/// comparison is `u · v̂` computed here out of the published aim point and the player's own
/// velocity, not out of anything the scoring function returns.
#[test]
fn f025_the_assist_picks_points_that_carry_the_flight_further_than_free_aim() {
    let mut app = app();
    settle(&mut app);
    let e = me(&mut app);
    let eye_height = data(&app).game.player.eye_height_m;

    let mut free_sum = 0.0_f64;
    let mut snap_sum = 0.0_f64;
    let mut pairs = 0_u32;

    for yaw_deg in (0..360).step_by(20) {
        for pitch_deg in [-10.0_f32, 5.0, 20.0] {
            // A flight, straight along the look direction and fast enough for the momentum
            // term to have anything to say.
            let flight = Intent {
                yaw: (yaw_deg as f32).to_radians(),
                pitch: pitch_deg.to_radians(),
                ..default()
            }
            .look_dir()
                * 30.0;

            set_assist(&mut app, 0.0, 0.0);
            look_at(&mut app, yaw_deg as f32, pitch_deg);
            app.world_mut().entity_mut(e).insert(Velocity(flight));
            ticks(&mut app, 4);
            let free = arms_of(&app, e);

            set_assist(&mut app, 100.0, 100.0);
            app.world_mut().entity_mut(e).insert(Velocity(flight));
            app.update();
            let snap = arms_of(&app, e);

            let eye = app.world().get::<Transform>(e).expect("a transform").translation
                + Vec3::Y * eye_height;
            let along = flight.normalize();
            for side in Side::ALL {
                let i = side.index();
                let (Some(f), Some(s)) = (free[i].point_m, snap[i].point_m) else {
                    continue;
                };
                if !free[i].anchorable || !snap[i].anchorable {
                    continue;
                }
                free_sum += (f - eye).normalize_or_zero().dot(along) as f64;
                snap_sum += (s - eye).normalize_or_zero().dot(along) as f64;
                pairs += 1;
            }
        }
    }

    assert!(pairs > 20, "only {pairs} usable pairs — the fixture found almost no anchors");
    let (free_mean, snap_mean) = (free_sum / pairs as f64, snap_sum / pairs as f64);
    println!(
        "F-025 momentum: mean cos(aim, flight) over {pairs} pairs — free {free_mean:.4}, \
         SNAP {snap_mean:.4}"
    );
    assert!(
        snap_mean >= free_mean,
        "the assist chose points that brake the flight more than free aim did: \
         {snap_mean:.4} < {free_mean:.4}"
    );
}


// ---------------------------------------------------------------------------------------
// The `settings` verb — the driver's route to the two assist knobs (`F-016`/`F-024`/`F-025`).
// ---------------------------------------------------------------------------------------

/// A `settings` line in a script really moves the running game's `PlayerSettings`.
///
/// **End to end and through the real app**, on purpose: `src/debug/script.rs`'s own unit tests
/// cover the parse and cover `Setting::apply`, and both of them would still pass if the driver
/// never dispatched the command at all — which is exactly how a verb ends up parsed, tested and
/// dead (`docs/FINDINGS.md` FIND-103: a test that asks the code and the code the same question).
/// So this one asks the **resource** after the app has run, and it asks it through
/// `assist_is_on()` and `assist_catch_deg()`, which are what `vector::aim` itself reads.
#[test]
fn f025_a_settings_line_moves_the_running_games_assist_knobs() {
    use defeated_by_titan::debug::script::parse;
    use defeated_by_titan::debug::ScriptRun;

    let mut app = defeated_by_titan::app(Cli { headless: true, ticks: 200, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));

    // Before: the shipped default is pure free aim — `F-016` defines 0 % as the absence of the
    // feature, and `f024_the_three_modes_come_out_of_the_strength_knob_alone` depends on it.
    for _ in 0..3 {
        app.update();
    }
    let before = *app.world().resource::<PlayerSettings>();
    assert!(!before.assist_is_on(), "the game must still start with the assist off");

    let plan = parse("settings assist_catch 100\nsettings assist_strength 100\nmark knobs-set\n")
        .expect("the `settings` verb has to parse");
    app.world_mut().resource_mut::<ScriptRun>().plan = plan;
    for _ in 0..10 {
        app.update();
    }

    let after = *app.world().resource::<PlayerSettings>();
    assert!(
        after.assist_is_on(),
        "a script set both knobs to 100 % and the running game is still in free aim — the \
         verb parses but the driver never dispatched it"
    );
    assert_eq!(after.assist_catch_pct, 100.0);
    assert_eq!(after.assist_strength_pct, 100.0);
    // 100 % catch is the 20 deg end stop. Read through the accessor `vector::aim` uses, so a
    // changed end stop moves this test with it instead of leaving it green on a stale number.
    assert!(
        (after.assist_catch_deg() - 20.0).abs() < 1e-4,
        "100 % has to mean the full catch cone, measured {}",
        after.assist_catch_deg()
    );
    // And nothing else moved: the verb names one field per line.
    assert_eq!(after.fov_deg, before.fov_deg);
    assert_eq!(after.pitch_limit_deg, before.pitch_limit_deg);
    assert_eq!(after.mouse_deg_per_px, before.mouse_deg_per_px);
}

// ---------------------------------------------------------------------------------------
// `F-024` — **the candidate search is a LINE, and the line is horizontal on the SCREEN.**
//
// > *„ok von snapping. die seile sollen immer auf der horzontalen fest sein. also wenn das
// > fadenkreuz 0, 0 ist sollen die seile nur auf der x achse snappen (objekte finden) also
// > seitlich! dann ist es auch besser einzuschätzen."* — the user, 2026-08-19
//
// The reason is the requirement: **einzuschätzen**. A snap that can move in two axes is
// guesswork; one that moves along a single, named axis can be learned. So the assist may search
// left and right of the crosshair and **never up or down** — and "up" is the *camera's* up at
// every pitch, because that is the axis the player reads (`docs/QUESTIONS.md` Q-040).
// ---------------------------------------------------------------------------------------

/// The camera's up axis, **derived here by hand** and not taken from `vector::aim::look_basis`.
///
/// `docs/conventions.md`: `look = (-sin y · cos p, sin p, -cos y · cos p)` and the horizontal
/// right is `(cos y, 0, -sin y)`. Their cross product is `(sin y · sin p, cos p, cos y · sin p)`,
/// and that closed form is what stands here — `FIND-103`: a test that asks the function under
/// test for its own frame cannot see the frame being wrong.
fn camera_up(yaw_rad: f32, pitch_rad: f32) -> Vec3 {
    let (sy, cy) = yaw_rad.sin_cos();
    let (sp, cp) = pitch_rad.sin_cos();
    Vec3::new(sy * sp, cp, cy * sp)
}

/// Every probe the candidate sweep casts lies on the **screen-horizontal line through the
/// crosshair**: its component along the camera's up axis is zero at every yaw and every pitch.
///
/// This is the whole feature in one assertion. A probe with any vertical component is a
/// candidate the snap could take that sits above or below where the player is looking, and that
/// is the thing he asked to be rid of.
#[test]
fn f024_every_probe_sits_on_the_crosshairs_own_row_and_never_above_or_below_it() {
    let v = vector_tuning();
    let mut worst_rad = 0.0_f32;
    for catch_deg in [5.0_f32, 12.5, 20.0] {
        let catch_rad = catch_deg.to_radians();
        for yaw_deg in [-170.0_f32, -35.0, 0.0, 91.0, 179.0] {
            for pitch_deg in [-89.0_f32, -60.0, -45.0, 0.0, 45.0, 89.0] {
                let intent = Intent {
                    yaw: yaw_deg.to_radians(),
                    pitch: pitch_deg.to_radians(),
                    ..default()
                };
                let up = camera_up(intent.yaw, intent.pitch);
                for side in Side::ALL {
                    for dir in probe_dirs(
                        look_basis(&intent),
                        catch_rad,
                        v.assist_probe_steps,
                        side,
                    ) {
                        let vertical = up.dot(dir).abs();
                        worst_rad = worst_rad.max(vertical.asin());
                        assert!(
                            vertical < 1e-5,
                            "a probe at yaw {yaw_deg} pitch {pitch_deg} sits {} deg above or \
                             below the crosshair — the sweep is still a cone",
                            vertical.asin().to_degrees()
                        );
                    }
                }
            }
        }
    }
    println!(
        "F-024 sideways-only: worst probe elevation off the crosshair over the whole sweep = \
         {:.6} deg",
        worst_rad.to_degrees()
    );
}

/// **In the running game, on the shipped map, with real rays**: whatever the assist publishes
/// into `ArmAim`, the direction from the eye to it has no vertical component in the camera's
/// frame. Measured over a yaw × pitch sweep at both end stops of the knobs.
///
/// The probe test above is arithmetic about directions; this one is about the **points the game
/// actually hands the rope and the HUD**, which is the thing the player sees. The number it
/// prints is the answer to "how far up or down can a snap still move a rope": in degrees off the
/// crosshair's row, and in metres of camera-vertical offset at the range it landed.
#[test]
fn f024_a_published_snap_point_never_sits_above_or_below_the_crosshair_in_the_running_game() {
    let mut app = app();
    settle(&mut app);
    let e = me(&mut app);
    let eye_height = data(&app).game.player.eye_height_m;

    let mut worst_deg = 0.0_f32;
    let mut worst_m = 0.0_f32;
    let mut worst_at = (0.0_f32, 0.0_f32);
    let mut seen = 0_u32;

    for (catch, strength) in [(0.0_f32, 0.0_f32), (50.0, 50.0), (100.0, 100.0)] {
        for yaw_deg in (0..360).step_by(30) {
            for pitch_deg in [-60.0_f32, -25.0, 0.0, 25.0, 60.0] {
                set_assist(&mut app, catch, strength);
                look_at(&mut app, yaw_deg as f32, pitch_deg);
                ticks(&mut app, 4);
                let arms = arms_of(&app, e);
                let eye = app.world().get::<Transform>(e).expect("a transform").translation
                    + Vec3::Y * eye_height;
                let up = camera_up((yaw_deg as f32).to_radians(), pitch_deg.to_radians());
                for side in Side::ALL {
                    let Some(p) = arms[side.index()].point_m else { continue };
                    seen += 1;
                    let off = p - eye;
                    let vertical_m = up.dot(off).abs();
                    let vertical_deg = (vertical_m / off.length().max(1e-6))
                        .clamp(-1.0, 1.0)
                        .asin()
                        .to_degrees();
                    if vertical_deg > worst_deg {
                        worst_deg = vertical_deg;
                        worst_m = vertical_m;
                        worst_at = (yaw_deg as f32, pitch_deg);
                    }
                }
            }
        }
    }

    println!(
        "F-024 sideways-only, {seen} published arm points: worst camera-vertical deviation \
         {worst_deg:.6} deg / {worst_m:.4} m, at yaw {} pitch {}",
        worst_at.0, worst_at.1
    );
    assert!(seen > 100, "only {seen} points — the sweep found almost nothing to aim at");
    assert!(
        worst_deg < 0.01,
        "a published aim point sits {worst_deg:.4} deg ({worst_m:.3} m) above or below the \
         crosshair at yaw {} pitch {} — the snap is still allowed to search vertically",
        worst_at.0,
        worst_at.1
    );
}

/// **What the collapse to a line costs `F-025`'s height weight, measured and not argued.**
///
/// *"Hoehenvorteil relativ zur Bewegungsrichtung (15 Prozent)"* scores `point.y - eye.y`. On a
/// **screen-horizontal** sweep every probe of a hemisphere leaves the eye at the same camera
/// elevation, so its world elevation is `asin(sin pitch · cos α)` — and at **pitch 0 that is
/// exactly 0 for every α**: every candidate is at eye height, `height` is 0.5 for all of them,
/// and the term contributes the same 0.075 to every score. It **cannot separate two candidates
/// while the player looks level**, which is the common case. Off level it comes back, because
/// the candidates then sit at different distances along rays of different world elevation.
///
/// The measurement is the honest one: run the same sweep twice, once with the file's weights and
/// once with `assist_score_height_w` set to 0 at run time, and count the arm-directions whose
/// published point moves. A term that separates nothing changes nothing when it is deleted.
///
/// **This test does not retune anything** — the five weights are the backlog's numbers and they
/// are the user's to judge (`docs/FINDINGS.md` FIND-131). It exists so the judgement has a
/// number under it.
#[test]
fn f025_the_height_term_stops_separating_candidates_when_the_player_looks_level() {
    let mut app = app();
    settle(&mut app);
    let e = me(&mut app);
    let pitches = [-60.0_f32, -25.0, 0.0, 25.0, 60.0];

    let sweep = |app: &mut App| {
        let mut out: Vec<Option<Vec3>> = Vec::new();
        for pitch_deg in pitches {
            for yaw_deg in (0..360).step_by(30) {
                set_assist(app, 100.0, 100.0);
                look_at(app, yaw_deg as f32, pitch_deg);
                ticks(app, 4);
                let arms = arms_of(app, e);
                out.push(arms[0].point_m);
                out.push(arms[1].point_m);
            }
        }
        out
    };

    let with_height = sweep(&mut app);
    app.world_mut().resource_mut::<GameData>().game.vector.assist_score_height_w = 0.0;
    let without = sweep(&mut app);

    assert_eq!(with_height.len(), without.len());
    let per_pitch = 12 * 2; // 12 yaws, two arms
    let mut moved_level = 0;
    let mut moved_off_level = 0;
    let mut line = String::new();
    for (p, pitch_deg) in pitches.iter().enumerate() {
        let lo = p * per_pitch;
        let moved = (lo..lo + per_pitch).filter(|i| with_height[*i] != without[*i]).count();
        line.push_str(&format!(" pitch {pitch_deg:+.0}: {moved}/{per_pitch};"));
        if *pitch_deg == 0.0 {
            moved_level += moved;
        } else {
            moved_off_level += moved;
        }
    }
    println!(
        "F-025 height term on a horizontal sweep — arm-directions whose point moves when the \
         15 % height weight is deleted:{line} (level {moved_level}, off level {moved_off_level})"
    );
    assert_eq!(
        moved_level, 0,
        "looking level, deleting the height weight moved {moved_level} points — then the term \
         does separate candidates on a horizontal sweep and the arithmetic above is wrong"
    );
}

// ---------------------------------------------------------------------------------------
// 8. TIME TO HOOK — the user, 2026-08-20
//
//   „und time to hook also e drücken zum connecten geht zu lang! das muss schneller gehen."
//
// Two separate sentences hide in that one, and both are measured here:
//   a) the LEDGER — how many ticks lie between the trigger going down and `Anchored`, at the
//      ranges `vector.hook_range_m` allows;
//   b) the HOLD — how long the button has to stay down for the shot to survive at all.
// ---------------------------------------------------------------------------------------

/// Fires once at a body `distance_m` straight ahead and returns
/// `(ticks_to_flying, ticks_to_anchored)` counted from the tick the trigger goes down.
///
/// The **button is held for the whole run**, so this measures the flight and nothing else.
fn ledger(app: &mut App, e: Entity, id: u32, distance_m: f32) -> (u64, u64) {
    let hand = hand(app, e);
    let target = hand + Vec3::NEG_Z * distance_m;
    let body = put_body(app, id, target + Vec3::NEG_Z * 4.0, Vec3::splat(4.0));
    aim_at(app, e, target, body, true);

    press(app, Side::Left);
    let mut flying = None;
    for n in 1..=4000 {
        app.update();
        if flying.is_none() && matches!(arm_state(app, e, Side::Left), HookState::Flying { .. }) {
            flying = Some(n);
        }
        if arm_state(app, e, Side::Left).is_anchored() {
            return (flying.unwrap_or(n), n);
        }
    }
    panic!("no anchor within 4000 ticks at {distance_m} m");
}

#[test]
fn f005_the_time_from_the_trigger_to_the_anchor_is_capped_by_the_file() {
    // The acceptance is the file's own ceiling: **press -> `Anchored` in at most
    // `1 + hook_flight_max_s * simulation_hz` ticks, at every distance in `hook_range_m`.**
    // The `1` is the fire tick and it is irreducible — the trigger is an edge, and an edge
    // is seen in the tick it happens in and acted on in the same one.
    //
    // ⚠️ RED before `hook_flight_max_s` was read: measured 3 / 7 / 13 / 25 / 49 / 61 ticks at
    // 18 / 50 / 100 / 200 / 400 / 500 m, i.e. the far half of the range cost between 0.4 s and
    // 1.02 s of a game that does nothing the player can act on.
    let d = defeated_by_titan::data::GameData::load(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/data"
    )));
    let hz = d.game.simulation_hz as f32;
    let cap_ticks = (d.game.vector.hook_flight_max_s * hz).ceil() as u64 + 1;
    let uncapped = |m: f32| (m / (d.game.vector.hook_speed_m_s / hz)).ceil() as u64 + 1;

    let mut line = String::new();
    let mut worst = 0;
    for (i, distance_m) in [18.0_f32, 50.0, 100.0, 200.0, 400.0, 500.0].iter().enumerate() {
        let mut app = app();
        let e = me(&mut app);
        settle(&mut app);
        let (flying, anchored) = ledger(&mut app, e, 91_100 + i as u32, *distance_m);
        line.push_str(&format!(
            " {distance_m:.0} m: fly {flying}, anchor {anchored} ({:.1} ms, was {});",
            anchored as f32 * 1000.0 / hz,
            uncapped(*distance_m)
        ));
        assert_eq!(flying, 1, "the shot did not leave in the tick the trigger went down");
        worst = worst.max(anchored);
    }
    println!("F-005 press -> anchored:{line} cap {cap_ticks} ticks");
    assert!(
        worst <= cap_ticks,
        "the slowest shot took {worst} ticks; game.ron: vector.hook_flight_max_s allows \
         {cap_ticks} (fire tick included).{line}"
    );
}

#[test]
fn f005_letting_go_in_mid_flight_no_longer_swallows_the_shot() {
    // ⚠️ RED before this round, and it is `F-028`'s rule broken in the one place nobody
    // looked: while `Flying`, `!held` sent the arm to `Retracting` and wrote a `Released`
    // message **with no log line at all**. So the button had to stay down for the whole
    // `1 + ceil(d / (hook_speed_m_s / hz))` ticks or the press did nothing and said nothing —
    // 4 ticks at 18 m and 26 ticks (0.43 s) at 200 m, which is longer than a human taps.
    // Measured in the real game the same day: `scripts/f005-feel.txt` ACT 2, a 50 ms tap at
    // 18.06 m, `assert Rope > 0 — measured 0.000`, and not one line in the log about it.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let hand = hand(&app, e);
    let target = hand + Vec3::NEG_Z * 60.0;
    let body = put_body(&mut app, 91_200, target + Vec3::NEG_Z * 4.0, Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);

    press(&mut app, Side::Left);
    app.update(); // the shot leaves
    assert!(matches!(arm_state(&app, e, Side::Left), HookState::Flying { .. }));
    app.update(); // one tick of flight — the tip is nowhere near 60 m
    let_go(&mut app, Side::Left);

    // The flight is a commitment: it lands.
    let landed = ticks_until_anchored(&mut app, e, Side::Left, 600);
    assert!(
        landed > 0,
        "a shot that has left has to reach its anchor even when the trigger comes back up"
    );
    // And then the rope obeys the button again, which is the behaviour that was never in
    // question: a hook is held while `Q`/`E` is held.
    app.update();
    assert_eq!(
        arm_state(&app, e, Side::Left),
        HookState::Retracting,
        "the arm anchored with the button already up and then did not let go"
    );
}

#[test]
fn f005_the_flight_ceiling_is_a_number_that_binds_and_a_number_a_player_would_wait() {
    // The bounds on `vector.hook_flight_max_s` itself. The ledger test above measures the game
    // **against the file**, so the file could answer any complaint by raising its own ceiling —
    // it was written that way first and the control run proved it: setting the key to 10.0 left
    // that test green at 601 ticks. This is the half that cannot be moved from inside.
    //
    // ⚠️ This lives here and not in `tests/data.rs`: the key is `vector::hook`'s and so is its
    // meaning, and a second file that also knows what „instant" is here is a second definition.
    let v = defeated_by_titan::data::GameData::load(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/data"
    )))
    .game
    .vector;

    // 1. It has to BIND. A ceiling above what `hook_speed_m_s` already delivers at maximum
    //    range is a key nobody can tell is broken — and the whole reason it exists is that the
    //    far half of `hook_range_m` cost up to 1.02 s.
    let uncapped_worst_s = v.hook_range_m / v.hook_speed_m_s;
    assert!(
        v.hook_flight_max_s < uncapped_worst_s,
        "hook_flight_max_s {} s never binds: hook_range_m / hook_speed_m_s is already \
         {uncapped_worst_s:.3} s",
        v.hook_flight_max_s
    );

    // 2. And it has to be a time a player does not experience as waiting. 0.15 s is the ceiling
    //    on the ceiling: it is one and a half times this file's own „reads as instant" band
    //    (`aim_spread_settle_s`, 0.10 s), it is half of the 0.30 s a human takes to react at
    //    all, and it is under the shortest window the game already measures as a deliberate
    //    gesture (`dodge_double_tap_window_ticks`, 18 ticks = 0.300 s). Above it, „e drücken
    //    zum connecten geht zu lang" is true again by arithmetic.
    assert!(
        v.hook_flight_max_s <= 0.15,
        "hook_flight_max_s is {} s — a press the player waits {} ms for is the complaint the \
         key was added to answer (the user, 2026-08-20)",
        v.hook_flight_max_s,
        v.hook_flight_max_s * 1000.0
    );
    // 3. Strictly positive: 0.0 is not „no ceiling", it is a division this code refuses to do.
    assert!(v.hook_flight_max_s > 0.0, "hook_flight_max_s has to be a positive time");
}
