//! The guard over the gizmos — **the strokes without which an image is not evidence.**
//!
//! `docs/ACCEPTANCE.md` demands an image for 🟧 on which you actually **recognize**
//! something. On `docs/images/t006-world-far.png` you see blocks, but not which of them is
//! anchorable. `src/debug/gizmo.rs` draws exactly that — and this file is what keeps the rule
//! from rotting quietly:
//!
//! - **They are registered and they run.** Take the `add_systems` line out of
//!   `src/debug/mod.rs` and something falls over here, instead of the next job losing its
//!   image.
//! - **They draw only what is tagged.** A gizmo on a block without an
//!   [`AnchorSurface`](defeated_by_titan::shared::AnchorSurface) would claim something the
//!   game does not do — and `F-003` ("no hook on untagged surfaces") would no longer be
//!   checkable in the image.
//! - **They draw nothing at all while the toggle is off.**
//!
//! Whatever can be checked **without** an app — colors, edge counts, sizes, the toggle —
//! lives as a unit test in `src/debug/gizmo.rs`. Only what needs a real app lives here.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::gizmo::{GizmoToggle, GizmoCounts, GizmoSystems};
use defeated_by_titan::debug::script::parse;
use defeated_by_titan::debug::{DebugOverlay, ScriptRun};
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    AnchorSurface, Block, BodyId, Health, Hook, HookState, IdCounter, LocalPlayer, MovementState,
    Side, StateClock, TitanId, TitanKindName, TitanState, Velocity, Cli,
};

/// Builds the **real** app, headless — not a second, similar one.
///
/// The toggle is set explicitly instead of being read from the environment: a test that
/// flips `DBT_GIZMOS` checks the process instead of the rule, and it disturbs every other
/// test running in parallel in the same process.
fn app(gizmos_on: bool) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(GizmoToggle { on: gizmos_on });
    app
}

fn counts(app: &App) -> GizmoCounts {
    *app.world().resource::<GizmoCounts>()
}

/// A cuboid shaped like a house, so the test does not hang on some special shape. Whether it
/// is anchorable is the only thing that differs between the cases.
fn block() -> Block {
    Block { size: Vec3::new(6.0, 9.0, 6.0), color: [0.42, 0.43, 0.40] }
}

#[test]
fn the_gizmo_systems_are_registered_in_the_update_schedule() {
    // The literal-minded half: the three systems are registered. The test next door checks
    // that they also do something — together the pair falls over whether somebody removes
    // the registration or leaves it standing and empties the body.
    //
    // Checked through the SET and not through system names: without `bevy_utils/debug` every
    // system here is called, verbatim, "<Enable the debug feature to see the name>"
    // (measured, see `src/debug/gizmo.rs::GizmoSystems`) — a name test would be green while
    // knowing nothing at all.
    let mut app = app(false);
    app.update(); // without one pass the schedule is not initialized

    let schedule = app.get_schedule(Update).expect("Update-Schedule");
    let systems = schedule
        .graph()
        .systems_in_set(GizmoSystems.intern())
        .expect("the GizmoSystems set is not in the Update schedule");

    assert_eq!(
        systems.len(),
        3,
        "there should be three drawing systems in the set (anchors, reference, players) — \
         without them the next job has no image on which anything is recognizable \
         (docs/ABNAHME.md)"
    );
}

#[test]
fn the_gizmos_run_and_outline_the_anchor_surfaces_of_the_map() {
    let mut app = app(true);
    app.update(); // Startup builds the map, Update draws it

    let drawn = counts(&app).anchors;
    let present = anchor_surfaces(&mut app);
    assert!(present > 0, "the map has not a single anchor surface — the test measures nothing");
    assert_eq!(
        drawn, present,
        "{present} anchor surfaces in the world, but {drawn} outlined"
    );
}

#[test]
fn a_block_without_an_anchor_surface_gets_no_gizmo() {
    // **This is the claim the image makes.** If every block were outlined, "outlined" would
    // only mean "is a block" — and `F-003` would no longer be checkable on any screenshot.
    let mut app = app(true);
    app.update();
    let prev = counts(&app).anchors;

    app.world_mut().spawn((Name::new("probe_untagged"), block(), Transform::from_xyz(80.0, 4.5, 0.0)));
    app.update();
    assert_eq!(
        counts(&app).anchors,
        prev,
        "a block without an anchor surface was outlined — the image would claim something \
         the game does not do"
    );

    app.world_mut().spawn((
        Name::new("probe_tagged"),
        block(),
        AnchorSurface,
        Transform::from_xyz(80.0, 4.5, 20.0),
    ));
    app.update();
    assert_eq!(
        counts(&app).anchors,
        prev + 1,
        "a new anchor surface stayed invisible — then the image shows an old state"
    );
}

#[test]
fn with_the_toggle_off_nothing_is_drawn() {
    // Gizmos must not run all the time: compute time, and on an in-game image they get in
    // the way.
    let mut app = app(false);
    app.update();
    app.update();
    assert_eq!(
        counts(&app),
        GizmoCounts::default(),
        "the toggle is off and it was drawn anyway"
    );
    assert!(anchor_surfaces(&mut app) > 0, "there would have been something to draw");
}

#[test]
fn the_hull_holding_the_camera_stays_empty_a_team_mate_does_not() {
    // The camera hangs off the local player as a child and sits inside his hull. Draw it and
    // your own hull lies over the whole image as a wireframe — 0.35 m in front of a 60 degree
    // lens, one edge fills the frame.
    let mut app = app(true);
    for _ in 0..3 {
        app.update(); // player, then camera (commands are deferred), then propagation
    }
    assert_eq!(
        counts(&app).players,
        0,
        "your own hull was drawn — it covers every first-person image"
    );

    // A team mate, exactly the way one will later arrive over the network: no LocalPlayer,
    // no camera. He MUST be marked, otherwise he is invisible in a long-range shot.
    {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        spawn_player(&mut commands, &mut ids, &data, Vec3::new(60.0, 2.0, 0.0), false);
    }
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(
        counts(&app).players,
        1,
        "a team mate without a camera stayed unmarked — exactly the case the marker \
         exists for (docs/multiplayer.md rule 3)"
    );
}

// ---------------------------------------------------------------------------------------
// P2 — the F3 overlay. Two systems that compiled for weeks and were registered nowhere.
// ---------------------------------------------------------------------------------------

/// The app after the deferred commands of `Startup` have really landed.
fn running_app() -> App {
    let mut app = app(false);
    // Player in `Startup`, camera one pass later (`render::attach_camera`), overlay entity at
    // the end of the `Startup` in which it was spawned.
    for _ in 0..3 {
        app.update();
    }
    app
}

/// The overlay entity's text and whether it is displayed at all.
fn overlay(app: &mut App) -> (String, Display) {
    let mut q = app.world_mut().query_filtered::<(&Text, &Node), With<DebugOverlay>>();
    let (text, node) = q
        .iter(app.world())
        .next()
        .expect(
            "no entity with `DebugOverlay` — `debug::spawn_overlay` is not registered in \
             `DebugPlugin`, and then no screenshot in this project can ever show a number",
        );
    (text.0.clone(), node.display)
}

#[test]
fn the_overlay_is_spawned_exactly_once() {
    // The literal half: `spawn_overlay` runs. Take the `add_systems(Startup, ...)` line out of
    // `src/debug/mod.rs` and this falls over — instead of the next job noticing three rounds
    // later that its HUD screenshots are all empty.
    let mut app = running_app();
    let mut q = app.world_mut().query_filtered::<Entity, With<DebugOverlay>>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "there must be exactly one overlay — none means it is not registered, two mean it is \
         in `Update` instead of `Startup` and grows by one per frame"
    );
}

/// One press of F3, the way a keyboard delivers it — released first, then pressed.
///
/// Not `press()` on its own: `ButtonInput::press` only arms `just_pressed` when the key was
/// **not** held before (`bevy_input-0.19.0/src/button_input.rs`). A second `press()` without a
/// release in between does nothing at all — which is also why `key F3` twice in a script
/// really does toggle twice: the driver releases the key after its duration.
fn tap_f3(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset(KeyCode::F3);
    keys.clear();
    keys.press(KeyCode::F3);
}

#[test]
fn the_overlay_is_off_until_f3_and_then_carries_the_numbers() {
    // The other half: registered AND doing something. `update_overlay` writes the text; the
    // pair falls over whether somebody removes the registration or leaves it standing and
    // empties the body.
    //
    // `run_schedule(Update)` and not `app.update()`: `ButtonInput` is cleared in `PreUpdate`
    // (`bevy_input-0.19.0/src/keyboard.rs`), so a press followed by `app.update()` would be
    // gone before `update_overlay` ever sees it.
    let mut app = running_app();

    let (_, display) = overlay(&mut app);
    assert_eq!(
        display,
        Display::None,
        "the overlay stands in the picture unasked — it lies over every in-game screenshot"
    );

    tap_f3(&mut app);
    app.world_mut().run_schedule(Update);

    let (text, display) = overlay(&mut app);
    assert_eq!(display, Display::Flex, "F3 did not switch the overlay on");
    // Not "is not empty": an overlay that prints its placeholder "F3" forever would pass that.
    // What has to be in there is the tick — the number that makes a screenshot line up with a
    // log line at all (`prompts/init.md` §12c).
    assert!(
        text.contains("t=") && text.contains("gas "),
        "the overlay shows {text:?} — tick and gas are what a report is reconstructed from"
    );

    tap_f3(&mut app);
    app.world_mut().run_schedule(Update);
    assert_eq!(
        overlay(&mut app).1,
        Display::None,
        "F3 is a toggle — a switch that only goes on is a switch you have to restart the game \
         to undo"
    );
}

#[test]
fn the_overlay_says_flight_for_a_skidding_player_and_not_for_a_walking_one() {
    // `FIND-050`, the sentence it left open: *"the F3 overlay still prints `Grounded` for a
    // player skidding at 30 m/s in flight mode — that is a lie in the debug overlay."* It is a
    // lie that costs evidence, not comfort: the overlay is what turns a screenshot into a
    // measurement, and a picture that says `Grounded` while `air_control` is steering the body
    // proves the opposite of what is happening.
    //
    // The honest print is the **predicate** (`player::locomotion::in_flight`, over
    // `ground_top_speed_m_s` = 6.3333 m/s at the file's numbers), not the raw variant — and
    // both halves are here, because a word that is always printed is not a verdict.
    let mut app = running_app();
    let player = {
        let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
        q.iter(app.world()).next().expect("no local player — `player::spawn_player` did not run")
    };

    // Feet on the floor, 30 m/s across it: `Grounded` is what `integrator::readback` writes, and
    // it is still true. It is just not the whole truth.
    app.world_mut()
        .entity_mut(player)
        .insert((MovementState::Grounded, Velocity(Vec3::new(30.0, 0.0, 0.0))));
    tap_f3(&mut app);
    app.world_mut().run_schedule(Update);
    let (text, _) = overlay(&mut app);
    let line = text.lines().next().unwrap_or_default().to_string();
    assert!(
        line.contains("Grounded FLIGHT") && line.contains("spd 30.0"),
        "the overlay printed {line:?} — a player skidding at 30 m/s is in flight by the game's \
         own rule, and the speed has to stand next to the verdict or it cannot be read off the \
         image"
    );

    // Slow again, same tick after next: the legs have him back, and the word has to go away.
    // `clear()` because `just_pressed` survives `run_schedule(Update)` — `ButtonInput` is
    // cleared in `PreUpdate`, so without this the second frame would toggle the overlay off.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    app.world_mut().entity_mut(player).insert(Velocity(Vec3::ZERO));
    app.world_mut().run_schedule(Update);
    let (text, _) = overlay(&mut app);
    let line = text.lines().next().unwrap_or_default().to_string();
    assert!(
        line.contains("Grounded") && !line.contains("FLIGHT"),
        "the overlay printed {line:?} — a standing player is not flying, and a verdict that is \
         always printed says nothing"
    );
}

#[test]
fn every_living_titan_gets_a_line_in_a_stable_order() {
    // `docs/PLAN-GAME.md` §4: one line per titan, so that `F-050`'s state machine can be READ
    // OFF a screenshot instead of being believed.
    //
    // The fixture is built out of `shared` types only and does not wait for `titan/` — the
    // overlay's job is to print what it is given, and that is what is checked here.
    //
    // The **order** is the part that is easy to lose: query iteration follows archetype order,
    // so two runs of the same script could print the same titans in a different order and the
    // `sha256` of the screenshot would stop matching — the one property `docs/ACCEPTANCE.md`
    // rests on, broken with nothing erroring.
    let mut app = running_app();
    for (id, state) in [(7u32, TitanState::Windup), (2, TitanState::Idle), (5, TitanState::Death)] {
        app.world_mut().spawn((TitanId(id), state));
    }
    // One without a state: the FSM has a hole, and a line that quietly vanishes is the reason
    // nobody finds it.
    app.world_mut().spawn(TitanId(9));
    // The full line `F-050`'s picture criterion asks for: kind, state and the tick fraction.
    // Assembled out of `shared` components alone, exactly like the four above — what is
    // checked here is that the overlay prints what it is handed, in the shape the criterion
    // names. That the numbers on a **real** husk are the file's is
    // `tests/titan.rs::f050_the_overlay_agrees_with_the_pose`.
    app.world_mut().spawn((
        TitanId(3),
        TitanKindName::new("husk"),
        TitanState::Windup,
        StateClock { ticks_in_state: 21, state_ticks: 36 },
    ));
    // And one whose state has no length at all: `Pursue` ends when the world ends it, so
    // `4/0` would be a fraction nobody can read.
    app.world_mut().spawn((
        TitanId(4),
        TitanKindName::new("husk"),
        TitanState::Pursue,
        StateClock { ticks_in_state: 4, state_ticks: 0 },
    ));

    tap_f3(&mut app);
    app.world_mut().run_schedule(Update);

    let (text, _) = overlay(&mut app);
    // Two header lines since `F-070`: the player line, then the mission line. The titan block
    // is what this test is about, and it comes after both.
    let lines: Vec<&str> = text.lines().skip(2).collect();
    assert_eq!(
        lines,
        vec![
            "titan#2 Idle",
            "husk#3 Windup 21/36",
            "husk#4 Pursue",
            "titan#5 Death",
            "titan#7 Windup",
            "titan#9 (no state)",
        ],
        "the overlay printed {text:?}"
    );
    assert_eq!(
        text.lines().nth(1),
        Some("mission BRIEFING"),
        "the mission line is the second one, and without --mission it reads Briefing: {text:?}"
    );
}

// ---------------------------------------------------------------------------------------
// The script vocabulary — `assert health` and `assert kills`.
// ---------------------------------------------------------------------------------------

/// Puts a single script line into the running app and lets the **real** driver execute it.
///
/// Returns `(how many asserts were checked, which of them failed)`.
fn run_line(app: &mut App, line: &str) -> (u32, Vec<String>) {
    {
        let plan = parse(line).unwrap_or_else(|e| {
            let list: Vec<String> = e.iter().map(|f| f.to_string()).collect();
            panic!("{line:?} does not parse: {}", list.join("; "))
        });
        let mut run = app.world_mut().resource_mut::<ScriptRun>();
        run.plan = plan;
        run.at = 0;
        run.done = false;
        run.checked = 0;
        run.failures.clear();
    }
    app.world_mut().run_schedule(FixedPreUpdate);
    let run = app.world().resource::<ScriptRun>();
    (run.checked, run.failures.clone())
}

fn local_player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

#[test]
fn assert_health_reads_the_players_health_component() {
    // The metric the mission rounds write their criteria in. It reads `shared::Health`, and
    // it reads it off the LOCAL PLAYER — not off a resource, not off `.single()`
    // (`docs/multiplayer.md` rule 3).
    let mut app = running_app();
    let player = local_player(&mut app);
    app.world_mut().entity_mut(player).insert(Health::full(100.0));

    let (checked, failures) = run_line(&mut app, "assert health > 0");
    assert_eq!(checked, 1, "the assert did not run at all");
    assert!(failures.is_empty(), "100 of 100 health is not > 0? {failures:?}");

    let (_, failures) = run_line(&mut app, "assert health == 100");
    assert!(failures.is_empty(), "{failures:?}");

    app.world_mut().entity_mut(player).get_mut::<Health>().unwrap().damage(40.0);
    let (_, failures) = run_line(&mut app, "assert health == 60");
    assert!(failures.is_empty(), "the metric does not follow the component: {failures:?}");
}

#[test]
fn assert_health_without_a_health_component_fails_loudly() {
    // **The half that matters.** A metric that answered `0.0` for "there is nothing to measure"
    // would turn `assert health > 0` into a silent lie the day somebody forgets the component —
    // and `measure()` documents exactly this: not measurable counts as failed (§9).
    //
    // Until `P5` nothing spawned player health and the case built itself. Since
    // `combat::health::grant` hangs `Health::full(game.ron: player.health)` on every player, the
    // test has to build it: the component is taken **off** again, and that is the same claim as
    // before — "a player nobody has measured" is what the un-measurable case *is*, and it is
    // the case that arrives the day a player comes over the wire without one.
    //
    // The removal sits directly in front of the measured line and nothing runs in between.
    // `grant` is registered in `FixedUpdate` (`SimulationSystems::Intent`), so a single
    // `app.update()` here would put the component straight back and this test would be green
    // for the wrong reason. `run_line` runs `FixedPreUpdate` and nothing else.
    //
    // `TimeUpdateStrategy::FixedTimesteps(1)` for the one step that hands out the health, and it
    // is not decoration: `running_app()` calls `app.update()` three times with the **automatic**
    // clock, so whether a fixed step runs at all depends on how much wall time those three
    // frames happened to take. Measured on this machine: under load (13 tests in the binary at
    // once) `grant` had run and the player carried `Health`, alone it had not. The premise
    // below would have been a coin toss.
    let mut app = running_app();
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    let player = local_player(&mut app);
    assert!(
        app.world().get::<Health>(player).is_some(),
        "the premise is inverted: since P5 a player HAS health, and this test takes it away — \
         if `combat::health::grant` no longer grants any, the loud one to fix is that"
    );
    app.world_mut().entity_mut(player).remove::<Health>();

    let (checked, failures) = run_line(&mut app, "assert health > 0");
    assert_eq!(checked, 1);
    assert_eq!(failures.len(), 1, "a player without health must not pass a health check");
    assert!(
        failures[0].contains("nothing"),
        "the message has to say that nothing was measured, not print a number: {:?}",
        failures[0]
    );
}

#[test]
fn assert_kills_without_a_mission_measures_nothing_and_fails() {
    // Same reasoning as `assert health` above, and the answer changed with `F-071`: the counter
    // is a component on the mission entity, so a run without `--mission` has nothing to read.
    // `Some(0.0)` would let `assert kills == 0` pass on a run in which the counter was never
    // wired up at all — a check that found nothing is not a check that passed (§9).
    let mut app = running_app();
    let (checked, failures) = run_line(&mut app, "assert kills == 0");
    assert_eq!(checked, 1);
    assert_eq!(failures.len(), 1, "there is no mission to count kills in");
    assert!(
        failures[0].contains("nothing"),
        "the message has to say that nothing was measured: {:?}",
        failures[0]
    );

    let (_, failures) = run_line(&mut app, "assert kills >= 3");
    assert_eq!(failures.len(), 1, "nobody has killed anything — this must not pass");
}

#[test]
fn assert_phase_without_a_mission_reads_briefing() {
    // The state is registered in **every** launch mode (`MissionPlugin`), so this one is
    // always measurable — and `Briefing` (0) is the honest reading of "no mission was
    // started", not a missing measurement.
    let mut app = running_app();
    let (checked, failures) = run_line(&mut app, "assert phase == 0");
    assert_eq!(checked, 1);
    assert!(failures.is_empty(), "{failures:?}");

    let (_, failures) = run_line(&mut app, "assert phase == 2");
    assert_eq!(failures.len(), 1, "nothing is Active without --mission");
}

/// Anchors one arm of the local player's [`Hook`], the way `vector::hook` would.
///
/// The state is written directly and not shot at a wall: what is under test here is the
/// **metric**, and a fixture that has to hit a block first would go red for aiming.
/// That a real shot really does anchor is `scripts/b001-anchor.txt`.
fn anchor(app: &mut App, player: Entity, side: Side) {
    let mut hook = app
        .world_mut()
        .get_mut::<Hook>(player)
        .expect("a player carries a `Hook` from tick 1 (`player::spawn_player`)");
    hook.arms[side.index()].state =
        HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO };
}

#[test]
fn assert_rope_counts_the_anchored_arms_and_is_not_a_constant() {
    // **The metric `scripts/f-flight-cut.txt` had to argue around.** That run claims a cortex
    // cut landed under rope momentum, and the only rope observation the vocabulary offered was
    // the GAS LEDGER — gas is debited only while `REEL_IN` is held and an arm is anchored
    // (`src/vector/gas.rs`), so a falling tank *implies* a rope. That is a proxy with about
    // five ticks of resolution, and it is not the sentence "a rope was anchored on this tick".
    //
    // The transition is the claim: zero with nothing anchored, one per anchored arm. A metric
    // that answered a constant would pass half of this test and prove nothing at all.
    let mut app = running_app();
    let player = local_player(&mut app);

    let (checked, failures) = run_line(&mut app, "assert rope == 0");
    assert_eq!(checked, 1, "the assert did not run at all");
    assert!(failures.is_empty(), "nothing was ever fired, so nothing is anchored: {failures:?}");

    let (_, failures) = run_line(&mut app, "assert rope >= 1");
    assert_eq!(
        failures.len(),
        1,
        "no hook has been fired — a rope check must not pass on an empty world"
    );

    anchor(&mut app, player, Side::Left);
    let (_, failures) = run_line(&mut app, "assert rope >= 1");
    assert!(failures.is_empty(), "one arm is anchored and the metric denies it: {failures:?}");
    let (_, failures) = run_line(&mut app, "assert rope == 1");
    assert!(failures.is_empty(), "one anchored arm must read as exactly one: {failures:?}");

    // Two arms, because `F-001`'s hooks are independent: a metric that saturated at one would
    // be green above and still be unable to tell one rope from two.
    anchor(&mut app, player, Side::Right);
    let (_, failures) = run_line(&mut app, "assert rope == 2");
    assert!(failures.is_empty(), "both arms are anchored: {failures:?}");
}

#[test]
fn assert_rope_without_a_player_measures_nothing_and_fails() {
    // Same direction as `assert health` and `assert kills`: not measurable counts as failed
    // (§9). `0.0` for "there is nobody to read a hook off" would turn `assert rope == 0` into a
    // green line in a run that had no player in it at all.
    //
    // The marker is taken off rather than the entity despawned: `bevy_render`'s
    // `sync_component` panics on a despawn in this headless fixture (measured), and the driver
    // asks `With<LocalPlayer>` anyway — no local player is no local player.
    let mut app = running_app();
    let player = local_player(&mut app);
    app.world_mut().entity_mut(player).remove::<LocalPlayer>();

    let (checked, failures) = run_line(&mut app, "assert rope == 0");
    assert_eq!(checked, 1);
    assert_eq!(failures.len(), 1, "there is no player whose hooks could be counted");
    assert!(
        failures[0].contains("nothing"),
        "the message has to say that nothing was measured, not print a number: {:?}",
        failures[0]
    );
}

#[test]
fn a_metric_that_does_not_exist_is_an_error_and_not_a_zero() {
    // A parser that silently accepted an unknown word would hand back a green run that
    // measured nothing. `phase` was refused this way until `F-070` built the state machine it
    // reads; the word that stands in for it here is one that measures nothing at all.
    let f = parse("assert cloud == 1\n").expect_err("`cloud` is not measurable");
    assert!(f[0].reason.contains("not measurable"), "{:?}", f[0].reason);
    assert!(
        f[0].reason.contains("health")
            && f[0].reason.contains("kills")
            && f[0].reason.contains("phase")
            && f[0].reason.contains("rope"),
        "the error message has to list what IS known: {:?}",
        f[0].reason
    );
}

// ---------------------------------------------------------------------------------------
// `--ticks` and the verdict. **The quietest way to lose a test in this repository.**
// ---------------------------------------------------------------------------------------

/// Runs the **real** app with a real `--ticks` limit and hands back the exit the process
/// would have.
///
/// [`App::should_exit`] and not a hand-rolled read of `Messages<AppExit>`: that is the exact
/// function `ScheduleRunnerPlugin`'s runner asks (`bevy_app-0.19.0/src/app.rs:1429`), errors
/// included — so what comes back here is the exit code, not a guess at it.
///
/// The plan is injected instead of a file being written: `DebugPlugin` reads `--script` at
/// **build** time, and a test that has to lay a file on disk first checks the file system
/// rather than the rule. Everything under test — the registered `exit_after_ticks`, the real
/// `ScriptRun`, `debug::cutoff_verdict` — is the app's own.
fn exit_of_a_run(ticks: u64, script: &str) -> AppExit {
    let mut app = defeated_by_titan::app(Cli { headless: true, ticks, ..default() });
    // Without this the fixed step depends on how much wall time a frame happened to take,
    // and `--ticks` would be reached after a number of frames nobody can predict.
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    {
        let plan = parse(script).unwrap_or_else(|e| {
            let list: Vec<String> = e.iter().map(|f| f.to_string()).collect();
            panic!("{script:?} does not parse: {}", list.join("; "))
        });
        app.world_mut().resource_mut::<ScriptRun>().plan = plan;
    }
    // Generously more frames than ticks: the exit has to come from `--ticks`, and a loop that
    // stops exactly at the limit could not tell "exited" from "ran out of frames".
    for _ in 0..(ticks * 2 + 20) {
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
    panic!("the run never ended — `--ticks {ticks}` did not exit at all");
}

#[test]
fn a_failed_assert_survives_the_tick_limit_that_cuts_the_script_off() {
    // **The bug this test exists for.** `exit_after_ticks` wrote `AppExit::Success` the moment
    // `tick >= ticks`, without ever looking at the script. So a limit that fell before the
    // script's end ended the run green **with red asserts in its own log**:
    //
    //   --headless --script scripts/f-001-hooks.txt --ticks 400   -> exit 0, 2 asserts red
    //   --headless --script scripts/f-001-hooks.txt --ticks 2000  -> exit 1, "2 of 14 failed"
    //
    // That is `docs/HANDOVER.md` §2.2 again with a different cause — a run that reports
    // success for something it did not show — and every `--ticks` number in a script header is
    // a potential instance of it.
    //
    // `assert height > 1000` cannot hold: the player is dropped from ~2 m. `wait 10` is 600
    // ticks, so the limit of 4 lands in the middle of it and `mark` never runs.
    let exit = exit_of_a_run(4, "assert height > 1000\nwait 10\nmark never-reached\n");
    assert!(
        exit.is_error(),
        "a run whose asserts went red ended at exit 0 because --ticks cut it off — the one \
         invariant is that a failed assert is never green, under any flag combination"
    );
}

#[test]
fn a_script_cut_off_before_its_end_is_not_a_green_run_either() {
    // The second, weaker half — and the reason the message is a distinct one. Every assert
    // that ran held, but the script never reached its end, so nothing it claims further down
    // was ever checked and its summary line was never printed. A run that stops in the middle
    // has not demonstrated what the file says it demonstrates; the fix is a bigger `--ticks`,
    // not a change to the game.
    let exit = exit_of_a_run(4, "assert height > 0\nwait 10\nmark never-reached\n");
    assert!(
        exit.is_error(),
        "the script never got to its end and the run still reported success — that is exactly \
         the reading a truncated `--ticks` must not be allowed to produce"
    );
}

#[test]
fn a_run_without_a_script_and_a_finished_script_both_stay_green() {
    // **The half that keeps the fix from being worse than the bug.** `--ticks` has legitimate
    // uses that have nothing to do with a verdict: a plain simulation run without any script,
    // and a script that really is through. Neither may start failing.
    assert!(
        !exit_of_a_run(4, "").is_error(),
        "`--ticks` without `--script` is a simulation run, not a failed test"
    );
    // `end` closes the script inside the limit; `wait 0.02` is one tick, so the run is really
    // finished and not merely out of instructions.
    assert!(
        !exit_of_a_run(6, "assert height > 0\nwait 0.02\nend\n").is_error(),
        "a script that reached its end with every assert holding must stay green"
    );
}

/// How many entities the world currently lists as anchorable.
fn anchor_surfaces(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, With<AnchorSurface>>();
    query.iter(app.world()).count()
}
