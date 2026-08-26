//! The guard over the titan — `F-050`, `F-056`, `F-064`.
//!
//! A titan has five ways of being wrong that **you cannot see in a screenshot**, and each of
//! them has a test here:
//!
//! 1. **The FSM is decoration.** The enum field is set correctly while the titan walks and
//!    hits at the same time, because nothing gates on it. A "the state changed" assertion
//!    passes that. A *tick count* on `Windup` does not — hence
//!    [`f050_the_husk_winds_up_for_as_long_as_the_file_says`].
//! 2. **The hit zone is placed by a magic number.** `8.9` is written in the docs, in the RON
//!    and in the survey, so the shortest road to a green picture is to type it into Rust. The
//!    assertion is therefore two-sided: the component follows `GameData` **and** `GameData`
//!    follows the file ([`f056_the_cortex_sits_where_scale_ron_says`]).
//! 3. **The kinematic body moves twice per tick.** Nothing errors; the titan is simply twice
//!    as fast as `titan.ron` says, and you go tune `speed_m_s`
//!    ([`f050_the_kinematic_titan_moves_exactly_once_per_tick`]).
//! 4. **The pose is read off the clock.** Nothing errors; the `--offscreen` sha256 just stops
//!    matching, and if nobody looks the reproducibility claim quietly becomes false
//!    ([`f050_the_pose_does_not_depend_on_the_clock`]).
//! 5. **The class cap is a clamp.** `spawn titan bellower` then produces a 14 m titan and a
//!    green run, and nobody finds out that 21 m was never tested
//!    ([`f064_no_kind_spawns_above_the_class_cap`]).
//!
//! Every number these tests compare against comes out of `assets/data/`, never out of this
//! file — except where a literal *is* the claim (the user's 8.9 m, the file's 36 ticks), and
//! there it stands next to the value it pins down.
//!
//! ## Why the tests drive with `app.update()` and `FixedTimesteps(1)`
//!
//! Same reason as `tests/player.rs`: avian takes its step size from the generic `Time`, which
//! only `run_fixed_main_schedule` switches over to `Time<Fixed>`. One `update()` is then
//! exactly one simulation step, on every machine.

use avian3d::prelude::{Collider, CustomPositionIntegration, RigidBody, Sensor};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::DebugOverlay;
use defeated_by_titan::mission::MissionPhase;
use defeated_by_titan::shared::{
    Cli, HitZone, HitZoneOf, ModelAnchors, MovementState, PlayerId, SimulationSystems, SpawnTitan,
    StateClock, TitanHit, TitanId, TitanKindName, TitanState, Velocity, CORTEX_ANCHOR,
};
use defeated_by_titan::titan::perception::{
    hears, loudness_m, period_ticks, sees, Awareness, CrowdSlot, Lod, Senses,
};
use defeated_by_titan::titan::rig::{PartExtent, TitanPart};
use defeated_by_titan::titan::{spawnable, SpawnRefused};

// ---------------------------------------------------------------------------
// the harness
// ---------------------------------------------------------------------------

/// Everything the state recorder has seen, one entry per simulation tick.
#[derive(Resource, Default)]
struct StateLog(Vec<TitanState>);

fn record_state(mut log: ResMut<StateLog>, titans: Query<&TitanState, With<TitanId>>) {
    // The first titan in the world. These tests never put two living ones in it at once,
    // except `f064`, which does not use the log.
    if let Some(state) = titans.iter().next() {
        log.0.push(*state);
    }
}

/// The **real** app, headless, one simulation step per `update()`, with the recorder in
/// `Last` so that what is sampled is the state at the end of the tick.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<StateLog>();
    app.add_systems(Last, record_state);
    app.update(); // Startup: the city, the local player, one step
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

/// Asks for a titan and lets it come into being. **Two ticks**: the spawner reads the message
/// in `PostStep`, so the entity exists at the end of the first one.
fn spawn(app: &mut App, kind: &str, pos: Vec3) {
    app.world_mut().write_message(SpawnTitan {
        kind: kind.to_string(),
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
    });
    ticks(app, 1);
}

fn titan_roots(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<TitanId>>();
    q.iter(app.world()).collect()
}

fn the_titan(app: &mut App) -> Entity {
    let all = titan_roots(app);
    assert_eq!(all.len(), 1, "expected exactly one titan, found {}", all.len());
    all[0]
}

/// Every entity of one titan's rig, root included.
fn rig_entities(app: &App, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(e) = pending.pop() {
        found.push(e);
        if let Some(kids) = app.world().get::<Children>(e) {
            pending.extend(kids.iter());
        }
    }
    found
}

fn part_entity(app: &App, root: Entity, part: TitanPart) -> Option<Entity> {
    rig_entities(app, root)
        .into_iter()
        .find(|e| app.world().get::<TitanPart>(*e) == Some(&part))
}

/// The world-space box the assembled rig occupies, out of `GlobalTransform` × [`PartExtent`].
///
/// Measured on the **built** body, not on the spawner's arithmetic: that is what makes a
/// height assertion an assertion about the titan and not about the function that made it.
fn rig_bounds(app: &App, root: Entity) -> (Vec3, Vec3) {
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for e in rig_entities(app, root) {
        let (Some(half), Some(global)) =
            (app.world().get::<PartExtent>(e), app.world().get::<GlobalTransform>(e))
        else {
            continue;
        };
        let centre = global.translation();
        low = low.min(centre - half.0);
        high = high.max(centre + half.0);
    }
    (low, high)
}

/// Run-length encoding of the state log — the shape the `F-050` claim is written in.
fn runs(log: &[TitanState]) -> Vec<(TitanState, usize)> {
    let mut out: Vec<(TitanState, usize)> = Vec::new();
    for state in log {
        match out.last_mut() {
            Some((s, n)) if s == state => *n += 1,
            _ => out.push((*state, 1)),
        }
    }
    out
}

/// `round(seconds * simulation_hz)`, computed **here** and not with the domain's own helper,
/// so that a wrong conversion in `titan/` cannot make the test agree with it.
fn expected_ticks(seconds: f32, hz: f64) -> usize {
    (seconds as f64 * hz).round() as usize
}

// ---------------------------------------------------------------------------
// F-050 — the state machine
// ---------------------------------------------------------------------------

/// ★ **The one that catches the FSM that is decoration.**
///
/// A husk 25 m from the player runs `Idle → Pursue → Windup → Strike → Recover → Pursue`, and
/// every state lasts the number of ticks `titan.ron` says. Goes red when somebody adds a
/// `Pursue → Strike` edge that skips the wind-up, or drives the timer off `Time::delta_secs()`
/// so the run length wobbles by ±2.
#[test]
fn f050_the_husk_winds_up_for_as_long_as_the_file_says() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let hz = d.game.simulation_hz;

    // +Z, so that the titan's spawn facing (Bevy's forward, −Z) already points at the player
    // at the origin: what is measured here is the walk, not a 180° turn at 50°/s.
    // `spawn` already runs the tick the body comes into being in. The recorder samples in
    // `Last`, so that tick's sample is `Idle`: the titan exists and the FSM has not run on it
    // yet. Throwing that sample away would hide exactly the first state of the sequence.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
    ticks(&mut app, 700);

    let log = app.world().resource::<StateLog>().0.clone();
    let r = runs(&log);
    let shape: Vec<TitanState> = r.iter().take(6).map(|(s, _)| *s).collect();
    assert_eq!(
        shape,
        vec![
            TitanState::Idle,
            TitanState::Pursue,
            TitanState::Windup,
            TitanState::Strike,
            TitanState::Recover,
            TitanState::Pursue,
        ],
        "the state sequence of a husk at 25 m, run-length encoded: {r:?}"
    );

    // **The assertion with the teeth.** Not "it was in Windup" — how many ticks.
    let windup = expected_ticks(husk.windup_s, hz);
    assert_eq!(
        windup, 36,
        "titan.ron husk.windup_s = {} at {hz} Hz — the criterion pins 36 ticks",
        husk.windup_s
    );
    assert_eq!(r[2].1, windup, "Windup ran {} ticks, the file says {windup}", r[2].1);
    assert_eq!(
        r[3].1,
        expected_ticks(husk.strike_s, hz),
        "Strike ran {} ticks, the file says {}",
        r[3].1,
        expected_ticks(husk.strike_s, hz)
    );
    assert_eq!(
        r[4].1,
        expected_ticks(husk.recover_s, hz),
        "Recover ran {} ticks, the file says {}",
        r[4].1,
        expected_ticks(husk.recover_s, hz)
    );

    // The number for the report. `--nocapture` prints it; the assertions above are what
    // holds it.
    println!(
        "F-050 husk at 25 m — Idle {} · Pursue {} · Windup {} · Strike {} · Recover {} · Pursue {}",
        r[0].1, r[1].1, r[2].1, r[3].1, r[4].1, r[5].1
    );
}

/// **The guard for trap 3 and for the whole evidence route.**
///
/// The same number of ticks twice, with different frame pacing: the second app runs an extra
/// zero-step frame between every simulation step, so `Update` runs twice as often while
/// `FixedUpdate` runs exactly as often. A pose driven by `AnimationPlayer` or by any other
/// consumer of `Time` diverges here; one driven by `ticks_in_state` cannot.
#[test]
fn f050_the_pose_does_not_depend_on_the_clock() {
    fn poses(extra_empty_frames: bool) -> (u64, Vec<[u32; 7]>) {
        let mut app = app();
        spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
        for _ in 0..460 {
            if extra_empty_frames {
                // A frame that advances no simulation step at all: `Update` runs, `Time`
                // moves, `Tick` does not.
                app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
                app.update();
                app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
            }
            app.update();
        }
        let tick = app.world().resource::<defeated_by_titan::shared::Tick>().0;
        let root = the_titan(&mut app);
        let mut out = Vec::new();
        for part in TitanPart::ALL {
            let Some(e) = part_entity(&app, root, part) else { continue };
            let t = app.world().get::<Transform>(e).expect("every part has a Transform");
            out.push([
                part as u32,
                t.translation.x.to_bits(),
                t.translation.y.to_bits(),
                t.translation.z.to_bits(),
                t.rotation.x.to_bits(),
                t.rotation.y.to_bits(),
                t.rotation.w.to_bits(),
            ]);
        }
        (tick, out)
    }

    let (tick_a, a) = poses(false);
    let (tick_b, b) = poses(true);
    assert_eq!(tick_a, tick_b, "the two runs did not reach the same tick");
    assert!(!a.is_empty(), "no rig was measured — the comparison would be vacuous");
    assert_eq!(
        a, b,
        "the pose differs between two runs with the same tick count and different frame \
         pacing — something in the pose reads a clock instead of `ticks_in_state`"
    );
}

/// **Trap 1, measured and not argued.**
///
/// `SolverBodyPlugin` gives a `SolverBody` to every dynamic *and kinematic* body
/// (`avian3d-0.7.0/src/dynamics/solver/solver_body/plugin.rs:25-30`) and `integrate_positions`
/// skips only `Without<CustomPositionIntegration>` (`.../integrator/mod.rs:503-504`). Without
/// the marker a titan at `speed_m_s` covers **twice** the ground per tick — no panic, no
/// warning, and the obvious reaction is to halve `speed_m_s` in the RON.
#[test]
fn f050_the_kinematic_titan_moves_exactly_once_per_tick() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");
    let hz = d.game.simulation_hz as f32;

    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
    let root = the_titan(&mut app);

    assert!(
        matches!(app.world().get::<RigidBody>(root), Some(RigidBody::Kinematic)),
        "the titan body is not `RigidBody::Kinematic`"
    );
    assert!(
        app.world().get::<CustomPositionIntegration>(root).is_some(),
        "no `CustomPositionIntegration` on a kinematic body — avian integrates the position a \
         second time and the titan walks at twice `speed_m_s`"
    );

    // Long enough that `accel_m_s2` has run out and the gait sits at `speed_m_s`:
    // 3.0 m/s at 3.0 m/s² is one second.
    let settle = (husk.speed_m_s / husk.accel_m_s2 * hz).ceil() as u64 + 10;
    ticks(&mut app, settle);
    let before = app.world().get::<Transform>(root).expect("transform").translation;
    ticks(&mut app, 60);
    let after = app.world().get::<Transform>(root).expect("transform").translation;

    let travelled = (after - before).length();
    let expected = husk.speed_m_s * 60.0 / hz;
    assert!(
        (travelled - expected).abs() < 0.05,
        "the husk covered {travelled:.3} m in 60 ticks; `titan.ron: speed_m_s` = {} says \
         {expected:.3} m. Twice that is the missing `CustomPositionIntegration`",
        husk.speed_m_s
    );
    println!("F-050 husk gait: {travelled:.3} m in 60 ticks, file says {expected:.3} m");
}

/// ★ **The one that turns `F-050`'s screenshot from a caption into a measurement.**
///
/// The picture criterion (`docs/PLAN-GAME.md` §8) asks for one frame in which the F3 overlay
/// reads `husk#1 Windup 21/36` **and** the arm is visibly up. Three separate claims live in
/// that one line, and each of them has a way of being quietly false:
///
/// - **the kind.** `titan#1` names no row of `titan.ron`, so nobody can check the 36 against a
///   file. Read off [`TitanKindName`] and not off the entity's `Name`, which is a debugging
///   convenience, not an interface.
/// - **the total.** Computed here as `round(windup_s × simulation_hz)` out of [`GameData`], so
///   a `36` typed next to the overlay fails; and asserted against
///   [`StateClock::state_ticks`](StateClock), so a `36` typed into `titan/` fails too.
/// - **the fraction and the pose belong to the same tick.** Everything below is read out of one
///   world without a simulation step in between, and the overlay is driven with a bare
///   `run_schedule(Update)` for exactly that reason. An overlay that lagged the FSM by a tick —
///   or a pose that lagged the number — would look perfect in a still frame and be wrong.
///
/// It goes red when the fraction is faked, when the total is a constant, when the arm is not in
/// the pose that goes with the printed tick, and when the kind disappears from the line.
#[test]
fn f050_the_overlay_agrees_with_the_pose() {
    // Tick 21 of 36 — the frame `docs/PLAN-GAME.md` §8 names, well inside the ramp and nowhere
    // near an edge, so the picture is a slow moment and reproduces.
    const AT_TICK: u32 = 21;

    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let total = expected_ticks(husk.windup_s, d.game.simulation_hz) as u32;
    assert_eq!(
        total, 36,
        "titan.ron husk.windup_s = {} at {} Hz — the criterion pins 36 ticks",
        husk.windup_s, d.game.simulation_hz
    );

    let rig = TitanRig::of(&d, husk).expect("husk rig");
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
    let root = the_titan(&mut app);

    // Up to the named tick, and not one step further.
    let mut reached = false;
    for _ in 0..900 {
        app.update();
        let clock = app.world().get::<StateClock>(root).copied().expect(
            "the husk carries no StateClock — then `ticks_in_state` lives only inside `titan/` \
             and no overlay can print it without an entry in the allow list",
        );
        if app.world().get::<TitanState>(root) == Some(&TitanState::Windup)
            && clock.ticks_in_state == AT_TICK
        {
            reached = true;
            break;
        }
    }
    assert!(reached, "the husk never reached tick {AT_TICK} of his wind-up");

    let clock = app.world().get::<StateClock>(root).copied().expect("StateClock");
    assert_eq!(
        clock.state_ticks, total,
        "the total the overlay is about to print is {} — titan.ron says {total}",
        clock.state_ticks
    );
    let id = app.world().get::<TitanId>(root).expect("TitanId").0;
    let kind = app
        .world()
        .get::<TitanKindName>(root)
        .expect(
            "the husk carries no TitanKindName — then the overlay can say `titan#1` at best, and \
             the picture cannot name the row of titan.ron the 36 came from",
        )
        .0
        .clone();
    assert_eq!(kind, "husk");

    // ---- the pose, in this same tick --------------------------------------------------
    // The arm the fraction claims, out of the pure function, against what is really drawn.
    let angles = PoseAngles {
        windup_arm_deg: d.scale.titan.windup_arm_deg,
        windup_lean_deg: d.scale.titan.windup_lean_deg,
        strike_arm_deg: d.scale.titan.strike_arm_deg,
        roll_lean_deg: d.scale.titan.roll_lean_deg,
    };
    let timing = TitanTiming::of(husk, d.game.simulation_hz);
    let pose = pose_of(TitanState::Windup, clock.ticks_in_state, &timing, &angles);
    let arm = part_entity(&app, root, TitanPart::ArmRight).expect("the rig has a right arm");
    assert_eq!(
        *app.world().get::<Transform>(arm).expect("the arm has a Transform"),
        arm_transform(&rig, true, pose.arm_deg),
        "the drawn arm is not in the pose that belongs to tick {AT_TICK} of the wind-up — the \
         overlay would then be printing a number about a body that is somewhere else"
    );
    // "Visibly raised" as a number and not as an impression: 21 of 36 is 58 % of the ramp.
    assert!(
        pose.arm_deg > 0.5 * d.scale.titan.windup_arm_deg,
        "at tick {AT_TICK} of {total} the arm stands at {:.1} deg of {} deg — that is not a \
         raised arm, and the picture would show a wind-up nobody can see",
        pose.arm_deg,
        d.scale.titan.windup_arm_deg
    );

    // ---- the overlay, in this same tick -----------------------------------------------
    // F3, then ONE `Update` pass and no simulation step. `app.update()` would do two wrong
    // things at once: `ButtonInput` is cleared in `PreUpdate` so the press would be eaten, and
    // the tick would advance — which is precisely the "read a tick apart" this test forbids.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset(KeyCode::F3);
        keys.clear();
        keys.press(KeyCode::F3);
    }
    app.world_mut().run_schedule(Update);

    let text = {
        let mut q = app.world_mut().query_filtered::<&Text, With<DebugOverlay>>();
        q.iter(app.world())
            .next()
            .expect("no entity with `DebugOverlay` — then no screenshot can carry a number")
            .0
            .clone()
    };
    let wanted = format!("{kind}#{id} {:?} {}/{}", TitanState::Windup, AT_TICK, total);
    assert!(
        text.lines().any(|line| line == wanted),
        "the F3 overlay reads:\n{text}\n\n`F-050` needs the line {wanted:?} — kind, state and \
         the tick fraction, so that the word in the picture can be checked against the pose in \
         the same picture"
    );

    // The world really did not move while the overlay was written; otherwise the agreement
    // above would be between two different ticks.
    assert_eq!(
        app.world().get::<StateClock>(root).copied(),
        Some(clock),
        "the simulation advanced while the overlay was being read"
    );

    // ---- and the same line under a state of a DIFFERENT length ------------------------
    // Found by falsification: with only the wind-up checked, a `{}/36` typed into the overlay
    // passes everything above, because the husk's wind-up really is 36 ticks. `Strike` is 12,
    // so a constant — wherever it were typed, in `debug` or in `titan` — cannot survive both.
    // The overlay is already on and `update_overlay` runs in `Update`, i.e. after this tick's
    // `FixedUpdate`: the line below is written from the very tick that is asserted.
    let strike_total = expected_ticks(husk.strike_s, d.game.simulation_hz) as u32;
    assert_ne!(strike_total, total, "Strike and Windup are the same length — pick another state");
    const AT_STRIKE_TICK: u32 = 5;
    let mut struck = false;
    for _ in 0..200 {
        app.update();
        let c = app.world().get::<StateClock>(root).copied().expect("StateClock");
        if app.world().get::<TitanState>(root) == Some(&TitanState::Strike)
            && c.ticks_in_state == AT_STRIKE_TICK
        {
            struck = true;
            break;
        }
    }
    assert!(struck, "the husk never reached tick {AT_STRIKE_TICK} of his strike");
    let text = {
        let mut q = app.world_mut().query_filtered::<&Text, With<DebugOverlay>>();
        q.iter(app.world()).next().expect("DebugOverlay").0.clone()
    };
    let wanted_strike =
        format!("{kind}#{id} {:?} {AT_STRIKE_TICK}/{strike_total}", TitanState::Strike);
    assert!(
        text.lines().any(|line| line == wanted_strike),
        "the F3 overlay reads:\n{text}\n\nexpected {wanted_strike:?} — the total under a titan \
         line has to come out of titan.ron per state, not out of one number that happens to fit \
         the wind-up"
    );

    println!(
        "F-050 overlay: {wanted:?} with the striking arm at {:.1} deg, then {wanted_strike:?}",
        pose.arm_deg
    );
}

// ---------------------------------------------------------------------------
// F-056 — the husk
// ---------------------------------------------------------------------------

/// ★ **The one that catches the hit zone placed by a magic number.**
///
/// Two-sided on purpose: the component follows `GameData`, and `GameData` follows the file.
/// Change `medium` in `scale.ron` to 12.0 and the first half follows while a Rust constant
/// does not; the second half is what stops the whole pair from being satisfied by typing 8.9
/// into both places.
#[test]
fn f056_the_cortex_sits_where_scale_ron_says() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");

    // Far outside `aggro_radius_m`, so the titan stays `Idle`: an upright torso is the pose
    // this height is defined against.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -200.0));
    ticks(&mut app, 2);
    let root = the_titan(&mut app);
    assert_eq!(app.world().get::<TitanState>(root), Some(&TitanState::Idle));

    let cortex = part_entity(&app, root, TitanPart::Cortex).expect("the rig has a cortex");
    let y = app
        .world()
        .get::<GlobalTransform>(cortex)
        .expect("the cortex has a GlobalTransform")
        .translation()
        .y;

    let from_data = d.titan_cortex_height_m(husk).expect("husk cortex height");
    assert!(
        (y - from_data).abs() < 0.01,
        "the cortex sits at {y} m, `GameData::titan_cortex_height_m` says {from_data} m"
    );
    // And the other half: `GameData` has to be reading the file, not a formula and not a
    // constant. 8.9 is the user's figure in metres for the `medium` class.
    assert!(
        (from_data - 8.9).abs() < 0.01,
        "titan_cortex_height_m(husk) = {from_data}, scale.ron says 8.9"
    );

    // **The failure mode the test exists for:** parented to the pelvis it would sit at the
    // right height and stop following the pose. It hangs under the head, or it is wrong.
    let parent = app.world().get::<ChildOf>(cortex).expect("the cortex has a parent").parent();
    assert_eq!(
        app.world().get::<TitanPart>(parent),
        Some(&TitanPart::Head),
        "the cortex hangs under {:?} and not under the head — it will not follow the pose",
        app.world().get::<Name>(parent)
    );

    println!("F-056 cortex height: {y:.3} m (GameData {from_data:.3} m, scale.ron 8.9 m)");
}

/// **Trap 2** — physically intangible, still hittable, and on its own layer.
#[test]
fn f056_the_cortex_is_a_sensor_on_its_own_layer() {
    use avian3d::prelude::CollisionLayers;
    use defeated_by_titan::shared::{LAYER_TITAN_BODY, LAYER_TITAN_CORTEX};

    let mut app = app();
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -200.0));
    ticks(&mut app, 1);
    let root = the_titan(&mut app);
    let cortex = part_entity(&app, root, TitanPart::Cortex).expect("cortex");

    assert!(app.world().get::<Collider>(cortex).is_some(), "the cortex has no collider at all");
    assert!(
        app.world().get::<Sensor>(cortex).is_some(),
        "the cortex is solid — a blade would bounce off the titan's weak point"
    );
    let layers = app.world().get::<CollisionLayers>(cortex).expect("the cortex has layers");
    assert_eq!(
        layers.memberships, LAYER_TITAN_CORTEX,
        "the cortex is not on LAYER_TITAN_CORTEX — a filtered cast never finds it and the cut \
         silently never lands"
    );
    let body = app.world().get::<CollisionLayers>(root).expect("the body has layers");
    assert_eq!(body.memberships, LAYER_TITAN_BODY);
}

/// A cortex hit kills **by rule**, the collider goes on tick one, and the body is gone after
/// `death_s`. Goes red when the corpse keeps its collider — a dead titan is then a wall a
/// player at 30 m/s drives into.
#[test]
fn f056_a_cortex_hit_removes_the_husk() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");

    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -200.0));
    ticks(&mut app, 1);
    let root = the_titan(&mut app);
    let id = *app.world().get::<TitanId>(root).expect("the titan carries a TitanId");
    assert_eq!(titan_roots(&mut app).len(), 1);
    assert!(
        rig_entities(&app, root).iter().any(|e| app.world().get::<Collider>(*e).is_some()),
        "a living titan without a single collider is not a body"
    );

    app.world_mut().write_message(TitanHit {
        titan: id,
        by: PlayerId(1),
        zone: HitZone::Cortex,
        speed_m_s: 30.0,
    });
    ticks(&mut app, 1);

    assert_eq!(
        app.world().get::<TitanState>(root),
        Some(&TitanState::Death),
        "a cortex hit did not kill — the cortex is the only truth (shared/message.rs:21)"
    );
    let still_solid: Vec<Entity> = rig_entities(&app, root)
        .into_iter()
        .filter(|e| app.world().get::<Collider>(*e).is_some())
        .collect();
    assert!(
        still_solid.is_empty(),
        "{} collider(s) left on the corpse one tick after the hit — a corpse is never a wall",
        still_solid.len()
    );

    ticks(&mut app, expected_ticks(husk.death_s, d.game.simulation_hz) as u64);
    assert_eq!(
        titan_roots(&mut app).len(),
        0,
        "the husk is still in the world {} s after the cut",
        husk.death_s
    );
    println!(
        "F-056 death: collider gone on tick 1, body gone after {} ticks ({} s)",
        expected_ticks(husk.death_s, d.game.simulation_hz),
        husk.death_s
    );
}

// ---------------------------------------------------------------------------
// F-064 — the size classes
// ---------------------------------------------------------------------------

/// Every kind either produces a body of the file's height, or refuses with a **named** error.
///
/// Goes red when somebody widens the cap silently, when a kind names a class that does not
/// exist, or when the spawner scales one mesh by a hard-coded factor per kind — which passes
/// for the husk and gives the scuttler a 10 m body.
#[test]
fn f064_no_kind_spawns_above_the_class_cap() {
    let mut app = app();
    let d = data(&app);

    // Far from the player and far apart: every one of them stays `Idle`, so the measured box
    // is the upright rig and not a titan mid-lean.
    for (i, kind) in d.titans.kinds.keys().enumerate() {
        app.world_mut().write_message(SpawnTitan {
            kind: kind.clone(),
            pos_x: i as f32 * 40.0,
            pos_y: 0.0,
            pos_z: 200.0,
        });
    }
    ticks(&mut app, 3);

    let mut spawned = 0usize;
    let mut refused = 0usize;
    for (i, (name, kind)) in d.titans.kinds.iter().enumerate() {
        let class = d.size_class(&kind.size_class).expect("every kind has a class");
        let cap = d
            .size_class(&d.scale.titan.max_spawnable_class)
            .expect("scale.ron: max_spawnable_class names a class that exists");

        match spawnable(&d, name) {
            Err(refusal) => {
                refused += 1;
                assert!(
                    class.height_m > cap.height_m,
                    "{name} was refused although its class fits under the cap: {refusal}"
                );
                assert!(
                    matches!(refusal, SpawnRefused::AboveClassCap { .. }),
                    "{name} was refused, but not by the class cap: {refusal:?}"
                );
                // A refusal is a refusal, not a clamp: nothing of that kind is in the world.
                let found = titan_roots(&mut app).into_iter().any(|e| {
                    app.world()
                        .get::<Name>(e)
                        .is_some_and(|n| n.as_str().starts_with(&format!("titan_{name}_")))
                });
                assert!(!found, "{name} is above the cap and was spawned anyway — clamped, not refused");
            }
            Ok(_) => {
                spawned += 1;
                assert!(class.height_m <= cap.height_m);
                let root = titan_roots(&mut app)
                    .into_iter()
                    .find(|e| {
                        app.world()
                            .get::<Name>(*e)
                            .is_some_and(|n| n.as_str().starts_with(&format!("titan_{name}_")))
                    })
                    .unwrap_or_else(|| panic!("{name} may spawn but no body of it is in the world"));

                let (low, high) = rig_bounds(&app, root);
                let height = high.y - low.y;
                let from_file = d.titan_height_m(kind).expect("class height");
                assert!(
                    (height - from_file).abs() < 0.01,
                    "{name} ({}) stands {height:.3} m tall, scale.ron says {from_file} m",
                    kind.size_class
                );
                assert!(
                    low.y.abs() < 0.01,
                    "{name} does not stand on the ground: its lowest point is at {:.3} m, and a \
                     body's origin lies between its feet (docs/conventions.md)",
                    low.y
                );
                assert!(
                    (low.x - i as f32 * 40.0).abs() < d.titan_height_m(kind).expect("h"),
                    "{name} did not spawn where it was asked to"
                );
                println!("F-064 {name}: class {} -> {height:.3} m", kind.size_class);
            }
        }
    }

    assert!(spawned > 0, "the cap allows nothing at all — nothing above was measured");
    assert!(
        matches!(spawnable(&d, "no_such_titan"), Err(SpawnRefused::UnknownKind { .. })),
        "an unknown kind must be refused by name, not silently ignored"
    );
    println!("F-064 cap `{}`: {spawned} kinds spawn, {refused} are refused", d.scale.titan.max_spawnable_class);
}

/// ★ **The refusal path itself, against a cap this test sets — not against the shipped one.**
///
/// It used to be one assertion inside the test above: *"`spawned > 0 && refused > 0`, a cap that
/// refuses nothing or allows nothing tests nothing"*. That sentence is right and the place was
/// wrong, and `docs/FINDINGS.md` FIND-118 measured what it cost: with the shipped cap raised to
/// `huge` **nothing in `titan.ron` is refused any more** — `boss` is the only class above it and
/// no kind names it — so three assertions in that test went red for a change that had nothing to
/// do with them. A user decision that is one line in a RON file was, in practice, one line plus
/// three assertions in a test file, and the entry that recorded it could not fix that because
/// `tests/titan.rs` belonged to another round.
///
/// So the **mechanism** is measured here, on a cap this test chooses, and the **file** is
/// measured above. Whatever `scale.ron: max_spawnable_class` is set to, exactly one of the two
/// has to move — and it is never this one.
#[test]
fn f064_the_cap_refuses_by_name_whatever_the_shipped_cap_happens_to_be() {
    let app = app();
    let mut d = data(&app);

    // The smallest class in the table, so that everything above it is refused however the file
    // is tuned. Not a made-up class name: an unknown cap is `SpawnRefused::UnknownCap` and a
    // different claim.
    let (smallest, floor) = d
        .scale
        .titan
        .classes
        .iter()
        .min_by(|a, b| a.1.height_m.total_cmp(&b.1.height_m))
        .map(|(name, class)| (name.clone(), class.height_m))
        .expect("scale.ron has size classes");
    d.scale.titan.max_spawnable_class = smallest.clone();

    let mut refused = 0usize;
    let mut allowed = 0usize;
    for (name, kind) in &d.titans.kinds {
        let class = d.size_class(&kind.size_class).expect("every kind names a class");
        match spawnable(&d, name) {
            Err(refusal) => {
                refused += 1;
                assert!(
                    class.height_m > floor,
                    "{name} is {} m under a cap of {floor} m and was refused anyway: {refusal}",
                    class.height_m
                );
                assert!(
                    matches!(refusal, SpawnRefused::AboveClassCap { .. }),
                    "{name} was refused, but not by the class cap: {refusal:?}"
                );
            }
            Ok(_) => {
                allowed += 1;
                assert!(class.height_m <= floor, "{name} is above the cap and was allowed");
            }
        }
    }
    assert!(
        allowed > 0 && refused > 0,
        "cap `{smallest}` ({floor} m) allowed {allowed} and refused {refused} — a cap that \
         refuses nothing or allows nothing tests nothing"
    );
    println!("F-064 refusal path at cap `{smallest}`: {allowed} allowed, {refused} refused");
}

/// **The bellower is blocked on purpose, and this is the one place that says so.**
///
/// `docs/QUESTIONS.md` Q-028 is a user decision taken in his absence and `assets/data/scale.ron`
/// carries the line. What this test adds is that the block is now **really** one line: it is the
/// only assertion in the repository that names the bellower against the cap, so lifting
/// `max_spawnable_class` to `"huge"` means editing `scale.ron` and deleting this function —
/// nothing else moves (FIND-118 measured the version where three other assertions did).
///
/// **Why it is still blocked, argued rather than inherited.** His mechanic is the *call*, and it
/// is real and measured (`tests/mission.rs::f062_a_bellowers_call_reaches_a_husk_that_is_blind_on
/// _his_own`). What he is *for* is the **ear** — `docs/gameplay/enemies.md`: he reacts to the
/// **sound of gas**, and the whole stealth layer of the enemy chapter hangs off that one
/// sentence. `F-051`, the perception model, does not exist. So a spawnable bellower today calls
/// on **sight**, at `aggro_radius_m` 70, and wakes every titan within `call_radius_m` 90 for 25
/// seconds — with no counterplay at all, because the counterplay the design specifies is "spend
/// less gas" and nothing can hear gas.
///
/// ## 🔴 The second reason is GONE, and that is what this test now records
///
/// It used to be worse than hollow: measured on 2026-08-19 by raising the cap and running the
/// suite, **he could not be killed.** `Q-030`'s arithmetic on a 21 m body — `width_fraction`
/// 0.25 gives a radius of 2.625 m, plus the player's 0.35 m is 2.975 m of clearance, against
/// `reach_m` 1.60 + `cortex_radius_m` 0.70 + `thickness_m` 0.12 = 2.42 m of blade. **−0.555 m**
/// (`docs/FINDINGS.md` FIND-124). A kind the player cannot kill is a bug with a body.
///
/// **2026-08-20 closed it from both ends at once**: `cortex_radius_m` 0.70 → 1.16 (the head
/// rule's own ceiling for a 21 m body), `reach_m` 1.60 → 2.00 and `thickness_m` 0.12 → 0.20 give
/// **3.36 m of blade against 2.975 m of clearance = +0.385 m**. So the assertion below is
/// **inverted on purpose**: it no longer says "he is unkillable, which is why he stays out", it
/// says "he is killable now, and the block is a design decision about the **ear** and nothing
/// else". It goes red if anyone shortens the blade back under a `huge` body — which would
/// silently re-open FIND-124 — and the block itself is still asserted one line above.
#[test]
fn f064_the_bellower_stays_blocked_until_the_ear_exists() {
    let app = app();
    let d = data(&app);
    let bellower = &d.titans.kinds["bellower"];
    assert_eq!(
        bellower.size_class, "huge",
        "the bellower changed class — then this block is about something else"
    );
    assert!(
        matches!(spawnable(&d, "bellower"), Err(SpawnRefused::AboveClassCap { .. })),
        "the bellower spawns. If that is intended, `F-051`'s ear exists or the user said so — \
         delete this test and say which (docs/QUESTIONS.md Q-028, docs/FINDINGS.md FIND-118)"
    );

    // The reason, as a number, out of the same three files `Q-030` reads.
    let rig = TitanRig::of(&d, bellower).expect("the bellower has a rig even unspawned");
    let clearance_m = rig.width_m * 0.5 + d.game.player.radius_m;
    let budget_m = d.gear.blades.reach_m + rig.cortex_radius_m + d.gear.blades.thickness_m;
    let margin_m = budget_m - clearance_m;
    assert!(
        margin_m > 0.0,
        "the bellower is unkillable again ({margin_m:+.3} m of margin, {budget_m:.3} m of blade \
         against {clearance_m:.3} m of clearance). FIND-124 is re-opened: somebody shortened \
         gear.ron's reach_m/thickness_m or took back his cortex_radius_m, and the only kind in \
         the game whose nape no arithmetic can reach is back"
    );
    println!(
        "F-064 the bellower ({}) stays out while max_spawnable_class is `{}` — Q-028, and that \
         is now the ONLY reason: his nape is {margin_m:+.3} m INSIDE reach ({budget_m:.3} m of \
         blade against {clearance_m:.3} m of clearance), where FIND-124 measured −0.555 m",
        bellower.size_class, d.scale.titan.max_spawnable_class
    );
}


// ---------------------------------------------------------------------------
// Q-030 — can a player reach the nape of a SOLID titan?
// ---------------------------------------------------------------------------
//
// `docs/QUESTIONS.md` Q-030 says he cannot: `gear.ron: blades.reach_m` is 1.60 m and a husk's
// body radius plus the player's is `1.25 + 0.35 = 1.60 m`, so "there is zero margin", and at
// `large` the same arithmetic is "0.50 m short".
//
// **That arithmetic leaves out two lengths that are in the same two files**, and the tests
// below measure what happens when they are put back:
//
// | | husk (`medium`) | warden (`large`) |
// |---|---|---|
// | clearance the two capsules need | `1.25 + 0.35` = **1.60 m** | `1.75 + 0.35` = **2.10 m** |
// | `gear.ron: blades.reach_m` | 1.60 | 1.60 |
// | ` + titan.ron: cortex_radius_m` | 0.55 | 0.60 |
// | ` + gear.ron: blades.thickness_m` | 0.12 | 0.12 |
// | **how far the blade reaches** | **2.27 m** | **2.32 m** |
// | margin | **+0.67 m** | **+0.22 m** |
//
// The blade does not have to touch the titan's *axis*. It has to touch a sphere of radius
// `cortex_radius_m` that sits `head_m / 2` **behind** the neck (`titan::rig::cortex_in_head`),
// with a swept capsule of radius `thickness_m` — and the cortex protrudes towards the one
// approach the design is built around. Hence the second, sharper assertion in
// [`q030_the_nape_is_cut_from_behind_and_not_from_the_front`]: the same pass flown from the
// front does **not** land, at either size, which is what "the fundamentals of the approach
// angle" (`src/titan/mod.rs`) means in metres.
//
// **Where Q-030's own measurement came from.** It was taken against the fixture in
// `tests/combat.rs`, whose body capsule has radius 1.25 m centred on the cortex's axis, and
// whose `fly_past` starts the player at `cortex.x − REACH_X` with `REACH_X = 0.80`. The player
// is therefore placed **0.80 m inside a 1.25 m body**, 0.80 m short of the 1.60 m the two
// capsules need. `(−28.4, 0, −13.0)` is that placement being ejected; it is a property of the
// fixture, not of the titan.

use avian3d::prelude::{GravityScale, LinearVelocity};
use defeated_by_titan::blades::cut::blade_segment;
use defeated_by_titan::blades::swing::{BladeTiming, SweptFrom, Swings};
use defeated_by_titan::shared::{LocalPlayer, LookOverride, Side, Tick};
use defeated_by_titan::titan::brain::TitanTiming;
use defeated_by_titan::titan::pose::{pose_of, Pose, PoseAngles};
use defeated_by_titan::titan::rig::{arm_transform, torso_transform, TitanRig};

/// Every [`TitanHit`] that was written, with the tick it was written on.
#[derive(Resource, Default)]
struct HitLog(Vec<(u64, TitanHit)>);

fn record_hits(mut log: ResMut<HitLog>, tick: Res<Tick>, mut hits: MessageReader<TitanHit>) {
    for hit in hits.read() {
        log.0.push((tick.0, *hit));
    }
}

/// The same app as [`app`], plus the hit log the passes are read off.
fn app_with_hits() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<StateLog>();
    app.init_resource::<HitLog>();
    app.add_systems(Last, record_state);
    app.add_systems(Last, record_hits);
    app.update();
    app
}

fn the_player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("the app spawns exactly one local player")
}

/// The lane every pass is flown in. Above the church (35 m), the tallest thing in the city — a
/// pass at real nape height would sweep the blade through a house and measure the house.
const LANE_Y: f32 = 60.0;

/// How much air is left between the two capsules. **A player has to be able to fly this**, so
/// it is not the last centimetre of the clearance: 20 cm.
const AIR_M: f32 = 0.20;

/// Whether the titan is allowed to turn towards the player during the pass.
///
/// `Off` zeroes every kind's `turn_deg_per_s` **in the resource**, which is how the four
/// geometry passes below keep measuring geometry after `Q-031` gave a titan the ability to turn
/// inside his own reach. `On` is the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tracking {
    On,
    Off,
}

/// What one pass did.
struct Pass {
    /// The tick a `Cortex` hit was written on, if one was.
    cortex_tick: Option<u64>,
    /// Smallest air between the titan's body capsule and the player's, over the whole pass.
    closest_m: f32,
    /// Smallest distance between the blade's surface and the cortex's, over the whole pass.
    blade_gap_m: f32,
    /// The largest velocity the player ever had **across** his flight line. This is "thrown
    /// off", as a number: the pass was flown at `speed` along one axis and nothing else.
    thrown_off_m_s: f32,
    /// Slowest the player ever got.
    slowest_m_s: f32,
}

/// Flies the real local player past a **real** titan at nape height and reports what happened.
///
/// `dir` is the flight direction; the look is nailed to it through [`LookOverride`] every tick,
/// because the blade hangs from the hand **perpendicular to the look** and not to the velocity
/// (`blades::cut::blade_segment`). Without that the pass measures a blade pointing wherever the
/// camera happened to be left — which is a way to measure nothing at all, quietly.
///
/// The player is parked 300 m away while the swing state machine runs up, so that the titan is
/// still `Idle` when the pass is placed; a titan that has been walking for half a second has
/// moved the target the test is trying to measure.
///
/// `model_cortex` is the `.glb`'s own `cortex` empty, in the root's space, or `None` for the
/// computed rig. It is the one knob that can move the kill zone without moving a single length
/// in `assets/data/`, which is exactly why it needs its own passes.
///
/// `turned_deg` yaws the titan on the spot **before** the pass, so the same flight line can be
/// flown at any bearing off his back. Added 2026-08-20 for `F-030`'s rear gate
/// (`titan.ron: cortex_half_angle_deg`): the approach angle stopped being a matter of
/// centimetres that day and became a matter of degrees, and nothing here could measure a degree.
/// Positive turns him **towards** the side the player passes on, i.e. it raises the bearing.
/// Only meaningful with [`Tracking::Off`] — a titan whose brain is turning will undo it.
fn fly_past_a_titan(
    kind: &str,
    dir: Vec3,
    air_m: f32,
    speed_m_s: f32,
    widen: Option<f32>,
    tracking: Tracking,
    model_cortex: Option<Vec3>,
    turned_deg: f32,
) -> Pass {
    let mut app = app_with_hits();
    if tracking == Tracking::Off {
        // **The brain, turned off in the resource and not in the file** — the same license the
        // `widen` knob below uses, and for the same reason: these four passes measure the four
        // LENGTHS against each other, and a body that is turning while they are measured is a
        // fifth variable in an equation that already has enough.
        //
        // It became necessary on 2026-08-13. Until `Q-031` a titan did not turn inside
        // `attack_range_m` at all, so parking the player 300 m away until the pass was placed
        // was enough to hold him still — which is exactly what the doc comment above this
        // function promises and what it can no longer deliver on its own.
        // [`q031_the_nape_survives_a_titan_who_tracks_you`] is the tracked case, measured with
        // the real number.
        for kind in app.world_mut().resource_mut::<GameData>().titans.kinds.values_mut() {
            kind.turn_deg_per_s = 0.0;
        }
    }
    if let Some(fraction) = widen {
        // The one knob Q-030 proposes to turn, turned **in the resource and not in the file**:
        // `assets/data/scale.ron` holds decisions of the user's and no test may edit it, but a
        // test may ask what would happen if it said something else. `TitanRig::of` reads this
        // field at spawn, so the titan really is built wider.
        app.world_mut().resource_mut::<GameData>().scale.titan.width_fraction = fraction;
    }
    let d = data(&app);
    let k = d.titan(kind).unwrap_or_else(|| panic!("titan.ron has no {kind}"));
    let rig = TitanRig::of(&d, k).expect("the kind has a size class");
    let eye_m = d.game.player.eye_height_m;
    let player_r = d.game.player.radius_m;
    // `look_dir()` is `(−sin yaw · cos pitch, sin pitch, −cos yaw · cos pitch)`, so this is the
    // yaw whose look is exactly `dir` (`shared::intent`).
    let yaw = f32::atan2(-dir.x, -dir.z);
    let step = |app: &mut App| {
        app.world_mut().resource_mut::<LookOverride>().0 = Some((yaw, 0.0));
        app.update();
    };

    let me = the_player(&mut app);
    app.world_mut().entity_mut(me).insert((
        Transform::from_xyz(0.0, LANE_Y, 300.0),
        GravityScale(0.0),
        LinearVelocity(Vec3::ZERO),
    ));
    spawn(&mut app, kind, Vec3::new(0.0, LANE_Y, 0.0));
    ticks(&mut app, 1);
    // **The model bound the way the renderer binds it.** `render::model::read_the_models_anchors`
    // turns the file's frame into the game's and scales it before it writes `ModelAnchors`, so
    // what lands on the entity is a point in the ROOT's own space, metres above the feet, +Z
    // backwards. Putting that same value on by hand is the whole of the binding as far as
    // `titan` is concerned — and it is the only way to fly a pass at a `.glb`'s nape without a
    // renderer, an asset server and an async scene load inside a unit test.
    if let Some(anchor) = model_cortex {
        let root = the_titan(&mut app);
        let mut anchors = ModelAnchors::default();
        anchors.0.insert(CORTEX_ANCHOR.to_string(), anchor);
        app.world_mut().entity_mut(root).insert(anchors);
        ticks(&mut app, 1);
    }

    // Hold the real slash button through the real `Intent` channel, and start the pass on the
    // blade's first cutting tick so the whole active window lies in front of it.
    // ⚠️ Depends on the binding `RMB` -> `SLASH_RIGHT` (`src/net/local.rs::read_input`); it has
    // to be the RIGHT blade because the loop below reads `Swings.right`.
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Right);
    let active_from = app.world().get::<BladeTiming>(me).expect("the player has blades").active_from_tick;
    for _ in 0..300 {
        step(&mut app);
        if app.world().get::<Swings>(me).and_then(|s| s.right.ticks_in_swing) == Some(active_from) {
            break;
        }
    }

    // `right` is the side the blade hangs on (`blade_segment`), so the player is placed on the
    // **other** side of the titan and the blade points inward, at the neck.
    let right = dir.cross(Vec3::Y).normalize();
    let clearance_m = rig.width_m * 0.5 + player_r;
    let offset_m = clearance_m + air_m;
    // **Where the kill zone really is**, and not where this file remembers putting it. Without
    // a model that is the rig's own arithmetic, unchanged, so the four geometry passes measure
    // exactly what they measured before this parameter existed. With one the sensor has been
    // moved by `rig::cortex_from_the_model`, and the pass is aimed at where it moved to — a
    // fixture that keeps aiming at the computed point would report a miss that is its own.
    if turned_deg != 0.0 {
        // On the spot, after the run-up, so `titan::brain` has already written whatever it was
        // going to write this tick and cannot undo it before the pass starts.
        let root = the_titan(&mut app);
        let mut at = app.world_mut().get_mut::<Transform>(root).expect("the titan has a transform");
        at.rotation = Quat::from_rotation_y(turned_deg.to_radians());
        ticks(&mut app, 1);
    }
    let cortex = match (model_cortex, turned_deg != 0.0) {
        (None, false) => Vec3::new(0.0, LANE_Y + rig.cortex_height_m, rig.head_m * 0.5),
        // A turned titan carries his nape round with him, so the arithmetic above stops
        // describing where it is. Read it off the sensor, exactly as the model case does.
        _ => {
            let root = the_titan(&mut app);
            let sensor = part_entity(&app, root, TitanPart::Cortex).expect("the rig has a cortex");
            app.world().get::<GlobalTransform>(sensor).expect("global").translation()
        }
    };
    let tick_m = speed_m_s / d.game.simulation_hz as f32;
    // Half a step past the crossing plus two ticks of lead, the same aiming as
    // `tests/combat.rs::fly_past`.
    let along = cortex.dot(dir) - (0.5 + tick_m * 2.0);
    let start = -right * offset_m + dir * along + Vec3::Y * (LANE_Y + rig.cortex_height_m - eye_m);

    let world = app.world_mut();
    world.entity_mut(me).insert((
        Transform::from_translation(start),
        GravityScale(0.0),
        LinearVelocity(dir * speed_m_s),
    ));
    // Or the teleport itself is swept, and a 300 m line cuts something on the way.
    if let Some(mut from) = world.get_mut::<SweptFrom>(me) {
        from.0 = start;
    }

    let mut out = Pass {
        cortex_tick: None,
        closest_m: f32::INFINITY,
        blade_gap_m: f32::INFINITY,
        thrown_off_m_s: 0.0,
        slowest_m_s: speed_m_s,
    };
    for _ in 0..16 {
        step(&mut app);
        let at = app.world().get::<Transform>(me).expect("transform").translation;
        let v = app.world().get::<LinearVelocity>(me).expect("velocity").0;
        // Air between the two capsules, on the ground plane: both are vertical capsules, so the
        // horizontal distance between the axes is the whole story.
        let axis_m = Vec3::new(at.x, 0.0, at.z).length();
        // Blade surface to cortex surface.
        let (a, b) = blade_segment(at, dir, Side::Right, eye_m, d.gear.blades.reach_m);
        let ab = b - a;
        let u = ((cortex - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        let gap_m = (cortex - (a + ab * u)).length() - rig.cortex_radius_m - d.gear.blades.thickness_m;
        // Everything is measured up to the kill and not past it: the cortex hit freezes the
        // player (`F-034`) and removes the body, so the ticks afterwards describe nothing.
        if out.cortex_tick.is_none() {
            out.closest_m = out.closest_m.min(axis_m - clearance_m);
            out.blade_gap_m = out.blade_gap_m.min(gap_m);
            out.thrown_off_m_s = out.thrown_off_m_s.max((v - dir * v.dot(dir)).length());
            out.slowest_m_s = out.slowest_m_s.min(v.length());
            if let Some((tick, _)) = app
                .world()
                .resource::<HitLog>()
                .0
                .iter()
                .find(|(_, h)| h.zone == HitZone::Cortex)
            {
                out.cortex_tick = Some(*tick);
            }
        }
    }
    out
}

/// ★ **The one the whole job exists for: the kill is a thing a player can DO.**
///
/// A real husk, solid, with the real body collider; the real player, flying past at 30 m/s at
/// nape height with 20 cm of air between the two capsules, swinging the real blade through the
/// real `Intent` channel. He cuts the cortex and **he is not thrown off**.
///
/// Goes red when the titan gets wider, the blade gets shorter, the cortex gets smaller, or the
/// cortex stops sitting on the *back* of the neck — i.e. when any of the four lengths in the
/// table at the top of this section moves against the other three.
/// [`q030_a_titan_wide_enough_really_does_put_the_nape_out_of_reach`] is the same assertion run
/// with one of them broken on purpose, and it is red by construction.
#[test]
fn q030_a_flying_player_reaches_the_nape_of_a_solid_husk() {
    let d = data(&app());
    let husk = d.titan("husk").expect("husk");
    let rig = TitanRig::of(&d, husk).expect("husk rig");
    // The arithmetic of Q-030, spelled out against the files, so that this test says which
    // numbers it is standing on and falls when one of them moves.
    let clearance_m = rig.width_m * 0.5 + d.game.player.radius_m;
    let budget_m = d.gear.blades.reach_m + rig.cortex_radius_m + d.gear.blades.thickness_m;
    assert!(
        (clearance_m - 1.60).abs() < 0.01,
        "a husk plus a player needs {clearance_m:.3} m of clearance, Q-030 is written against 1.60"
    );

    let p = fly_past_a_titan("husk", Vec3::NEG_Z, AIR_M, 30.0, None, Tracking::Off, None, 0.0);
    assert!(
        p.cortex_tick.is_some(),
        "a pass at 30 m/s with {AIR_M:.2} m of air landed NO cortex hit. Closest approach \
         {:.3} m, blade to cortex {:.3} m. Clearance {clearance_m:.3} m, reach budget \
         {budget_m:.3} m",
        p.closest_m,
        p.blade_gap_m
    );
    assert!(
        p.thrown_off_m_s < 1.0 && p.slowest_m_s > 29.0,
        "the player was thrown off: {:.2} m/s across his flight line, slowest {:.2} m/s of 30",
        p.thrown_off_m_s,
        p.slowest_m_s
    );
    assert!(
        p.closest_m > 0.0,
        "the two capsules overlapped by {:.3} m — that is not a pass, it is a collision",
        -p.closest_m
    );
    println!(
        "Q-030 husk: cortex cut on tick {} · closest approach {:.3} m of air · blade to cortex \
         {:+.3} m · thrown off {:.2} m/s across, slowest {:.2} m/s · clearance {clearance_m:.3} m \
         vs reach budget {budget_m:.3} m (margin {:+.3} m)",
        p.cortex_tick.expect("cut"),
        p.closest_m,
        p.blade_gap_m,
        p.thrown_off_m_s,
        p.slowest_m_s,
        budget_m - clearance_m
    );
}

/// The same at `large` — 14 m, where Q-030's arithmetic says the blade is **0.50 m short**.
///
/// It is not short: the 0.50 m is `reach_m` measured against the body radius alone, and the two
/// lengths it leaves out (`cortex_radius_m` 0.77 and `thickness_m` 0.20) are worth 0.97 m.
#[test]
fn q030_the_nape_is_reachable_on_a_large_titan_too() {
    let d = data(&app());
    let player_r = d.game.player.radius_m;

    // The `large` representative the criterion names.
    let warden = d.titan("warden").expect("warden");
    assert_eq!(warden.size_class, "large");
    let rig = TitanRig::of(&d, warden).expect("warden rig");
    let clearance_m = rig.width_m * 0.5 + player_r;
    assert!(
        (clearance_m - 2.10).abs() < 0.01,
        "a `large` titan plus a player needs {clearance_m:.3} m, Q-030 is written against 2.10"
    );
    let p = fly_past_a_titan("warden", Vec3::NEG_Z, AIR_M, 30.0, None, Tracking::Off, None, 0.0);
    assert!(
        p.cortex_tick.is_some(),
        "warden (14 m): no cortex hit. Clearance {clearance_m:.3} m, closest approach {:.3} m, \
         blade to cortex {:.3} m",
        p.closest_m,
        p.blade_gap_m
    );
    assert!(
        p.thrown_off_m_s < 1.0 && p.closest_m > 0.0,
        "warden: thrown off at {:.2} m/s across the flight line, closest {:.3} m",
        p.thrown_off_m_s,
        p.closest_m
    );
    println!(
        "Q-030 warden (14 m, class large): cortex cut on tick {} · {:.3} m of air · blade to \
         cortex {:+.3} m · clearance {clearance_m:.3} m",
        p.cortex_tick.expect("cut"),
        p.closest_m,
        p.blade_gap_m
    );

    // ---- and the same arithmetic for every kind that may spawn --------------------------
    //
    // The margin is what is left of the blade once the two capsules have had their clearance:
    //
    //     reach_m + cortex_radius_m + thickness_m  −  (width_m / 2 + player.radius_m)
    //
    // A margin at or below zero is a kind whose nape **cannot** be cut from a flying pass at
    // any offset, which is the thing Q-030 believes is already true of all of them. It is not
    // true of any of them — but the margins are not comfortable either, and the smallest one
    // is named below by whichever kind owns it.
    let mut margins: Vec<(String, f32, f32)> = Vec::new();
    for (name, kind) in &d.titans.kinds {
        if spawnable(&d, name).is_err() {
            continue;
        }
        let rig = TitanRig::of(&d, kind).expect("rig");
        let clearance_m = rig.width_m * 0.5 + player_r;
        let budget_m = d.gear.blades.reach_m + rig.cortex_radius_m + d.gear.blades.thickness_m;
        margins.push((name.clone(), rig.height_m, budget_m - clearance_m));
    }
    margins.sort_by(|a, b| a.2.total_cmp(&b.2));
    for (name, height_m, margin_m) in &margins {
        assert!(
            *margin_m > 0.0,
            "{name} ({height_m} m) has {margin_m:+.3} m of reach left over its own body: a \
             flying pass cannot cut that nape at ANY offset. `gear.ron: blades.reach_m`, \
             `titan.ron: {name}.cortex_radius_m` and `scale.ron: titan.width_fraction` no \
             longer add up (docs/QUESTIONS.md Q-030)"
        );
    }
    println!(
        "Q-030 reach margin per spawnable kind (reach + cortex_radius + thickness − clearance): {}",
        margins
            .iter()
            .map(|(n, h, m)| format!("{n} ({h} m) {m:+.3} m"))
            .collect::<Vec<_>>()
            .join(" · ")
    );

    // The smallest margin is a corridor a player has to fly down, and it is worth a number.
    let (tightest, _, tightest_margin) = margins[0].clone();
    let mut flyable_air_m = 0.0f32;
    for air_m in [0.30f32, 0.25, 0.20, 0.15, 0.10, 0.05] {
        if fly_past_a_titan(&tightest, Vec3::NEG_Z, air_m, 30.0, None, Tracking::Off, None, 0.0)
            .cortex_tick
            .is_some()
        {
            flyable_air_m = air_m;
            break;
        }
    }
    println!(
        "Q-030 tightest kind: {tightest}, {tightest_margin:+.3} m of margin — the widest gap \
         between the two capsules that still lands a cut is {flyable_air_m:.2} m"
    );
}

/// ★ **The approach angle, as a number.** The nape is cut from **behind**, never from the front.
///
/// The cortex sits half a head's depth behind the neck (`titan::rig::cortex_in_head`), so the
/// same pass, the same speed and the same air produce a kill from one side of the body and
/// nothing at all from the other. That asymmetry is the husk's entire lesson
/// (`src/titan/mod.rs`: *"the fundamentals of the approach angle"*), and without a test it is a
/// sentence in a doc comment.
///
/// Goes red when somebody centres the cortex on the neck axis to make it easier to hit, or
/// raises `reach_m` until the blade is long enough to fish the nape out from the front.
#[test]
fn q030_the_nape_is_cut_from_behind_and_not_from_the_front() {
    for kind in ["husk", "warden"] {
        // Flying along −Z the player comes over the titan's back (a fresh titan faces −Z), and
        // the blade meets the cortex before it meets anything else.
        let behind = fly_past_a_titan(kind, Vec3::NEG_Z, AIR_M, 30.0, None, Tracking::Off, None, 0.0);
        // Flying along +X the player is in **front** of the titan and the blade swings towards
        // his back, through the whole depth of the body.
        let front = fly_past_a_titan(kind, Vec3::X, AIR_M, 30.0, None, Tracking::Off, None, 0.0);
        assert!(behind.cortex_tick.is_some(), "{kind}: the pass from behind did not land");
        assert!(
            front.cortex_tick.is_none(),
            "{kind}: a pass from the FRONT cut the cortex, with the blade {:.3} m from it. The \
             cortex is on the back of the neck and the approach angle is supposed to be the \
             whole lesson",
            front.blade_gap_m
        );
        println!(
            "Q-030 {kind} approach angle: from behind cut on tick {:?} (blade {:+.3} m), from the \
             front no cut (blade {:+.3} m short)",
            behind.cortex_tick, behind.blade_gap_m, front.blade_gap_m
        );
    }
}

/// **Rule 5's second half, for a defect that turned out not to exist.**
///
/// There is no fix to take back out here — the measurement says the nape was reachable all
/// along. So the guard is proved the other way round: the one length Q-030 proposes to change
/// is changed *in the resource*, and the ★ test's assertion has to fall.
///
/// It also puts a number on how much room 0.25 has. `width_fraction` is not on a cliff edge:
/// the husk's nape stays reachable up to about **0.45**, and 0.25 is 44 % below that.
///
/// ⚠️ **The list moved on 2026-08-20 and the reason is the point of the test, not a repair.**
/// It used to run to 0.45 and the cliff sat near 0.33; `gear.ron`'s `reach_m` 1.6 → 2.0 and
/// `thickness_m` 0.12 → 0.20 pushed the same cliff to between 0.41 and 0.49, so the old top of
/// the sweep stopped being a miss. The claim is unchanged — *there is a width at which the nape
/// is unreachable, and `scale.ron`'s 0.25 is well below it* — and the sweep is re-aimed at where
/// that width now is. **The `far_too_wide` assert is what stops this from silently becoming a
/// test that proves nothing** (`docs/FINDINGS.md` FIND-147).
#[test]
fn q030_a_titan_wide_enough_really_does_put_the_nape_out_of_reach() {
    let mut reachable = Vec::new();
    for fraction in [0.25f32, 0.33, 0.41, 0.49, 0.65] {
        let p = fly_past_a_titan("husk", Vec3::NEG_Z, AIR_M, 30.0, Some(fraction), Tracking::Off, None, 0.0);
        reachable.push((fraction, p.cortex_tick.is_some(), p.blade_gap_m));
    }
    let at_file_value = reachable[0];
    let far_too_wide = reachable[reachable.len() - 1];
    assert!(
        at_file_value.1,
        "the husk is unreachable at the value that stands in scale.ron — the ★ test is lying"
    );
    assert!(
        !far_too_wide.1,
        "the husk is still reachable at width_fraction {} — this test proves nothing, because \
         nothing makes it fall",
        far_too_wide.0
    );
    println!(
        "Q-030 width_fraction sweep (husk, 30 m/s, {AIR_M:.2} m of air): {}",
        reachable
            .iter()
            .map(|(f, hit, gap)| format!("{f:.2}{}{:+.2}", if *hit { " cut " } else { " MISS " }, gap))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}

// ---------------------------------------------------------------------------
// F-053 — the telegraph
// ---------------------------------------------------------------------------

/// The vertical focal length of `game.ron: camera.fov_deg`, in pixels of a 1080-line screen.
///
/// Bevy reads `PerspectiveProjection::fov` as the **vertical** field of view — `get_clip_from_view`
/// builds `Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near)`
/// (`bevy_camera-0.19.0/src/projection.rs:284-287`), and `perspective_infinite_reverse_rh`'s
/// first argument is `fov_y_radians`. So 60° is 60° top to bottom, and
/// `f = (1080 / 2) / tan(30°)` = 935.307.
const F_PX: f32 = 935.307;

/// Where the striking hand is, in world space, read off the **assembled** rig.
///
/// The arm box is one entity whose rotation is the hinge, so the hand is the far end of that
/// box: half an arm down its own local −Y (`titan::rig::arm_transform`). Taken through
/// `GlobalTransform`, so the lean of the torso and the yaw of the body are in it and the
/// measurement cannot drift away from what is drawn.
fn hand_world(app: &App, root: Entity, rig: &TitanRig) -> Vec3 {
    let arm = part_entity(app, root, TitanPart::ArmRight).expect("the rig has a right arm");
    let global = app.world().get::<GlobalTransform>(arm).expect("the arm has a GlobalTransform");
    global.transform_point(Vec3::new(0.0, -rig.arm_m * 0.5, 0.0))
}

/// Pinhole projection of a world point onto a 1080-line screen, from a camera `distance_m`
/// away, looking horizontally at `at`.
///
/// The camera stands on the titan's **flank** (+X of the body), so the wind-up — which carries
/// the hand up and backwards, in Y and Z — lies in the image plane and none of it is hidden in
/// the depth axis. A camera in front of him would measure the same telegraph as smaller for a
/// reason that has nothing to do with the pose.
fn to_screen_px(point: Vec3, eye: Vec3) -> Vec2 {
    let forward = Vec3::NEG_X;
    let right = Vec3::NEG_Z;
    let up = Vec3::Y;
    let view = point - eye;
    let depth = view.dot(forward);
    assert!(depth > 0.1, "the point is behind the camera: depth {depth}");
    Vec2::new(F_PX * view.dot(right) / depth, F_PX * view.dot(up) / depth)
}

/// Runs a husk up to his wind-up and returns the hand at the **first** and the **last** tick of
/// it, plus how many ticks that was.
///
/// `angles` overrides `scale.ron`'s three pose angles **in the resource and not in the file** —
/// that is what turns "goes red when the pose angles in RON go to zero" from a sentence in
/// `docs/PLAN-GAME.md` into [`f053_the_telegraph_goes_dark_when_the_pose_angles_go_to_zero`].
fn windup_hand(angles: Option<(f32, f32)>) -> (Vec3, Vec3, usize, TitanRig) {
    let mut app = app();
    if let Some((arm_deg, lean_deg)) = angles {
        let mut d = app.world_mut().resource_mut::<GameData>();
        d.scale.titan.windup_arm_deg = arm_deg;
        d.scale.titan.windup_lean_deg = lean_deg;
    }
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");
    let rig = TitanRig::of(&d, husk).expect("husk rig");

    // +Z, so the titan's spawn facing already points at the player and the approach is a walk
    // instead of a 180° turn — the same placement as `f050`.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
    let root = the_titan(&mut app);

    // Run to the FIRST tick of `Windup` and take the hand there.
    let mut start = None;
    for _ in 0..900 {
        app.update();
        if app.world().get::<TitanState>(root) == Some(&TitanState::Windup) {
            start = Some(hand_world(&app, root, &rig));
            break;
        }
    }
    let start = start.expect("the husk never wound up");
    let planted_at = app.world().get::<Transform>(root).expect("transform").translation;
    let entry_clock = app.world().get::<StateClock>(root).expect("clock").ticks_in_state;
    assert_eq!(entry_clock, 0, "the first Windup sample was taken {entry_clock} ticks in");

    // ... and to the LAST tick of it. Tick `windup_ticks` is already `Strike`, so the sample
    // that has to be held is the one before the edge.
    let mut end = start;
    let mut held = 0;
    for _ in 0..300 {
        app.update();
        if app.world().get::<TitanState>(root) != Some(&TitanState::Windup) {
            break;
        }
        end = hand_world(&app, root, &rig);
        held += 1;
    }
    // `walk` plants a titan that is not in `Pursue`, so every millimetre the hand travels is
    // the pose and none of it is the walk. Without this the measurement below would be a
    // telegraph plus 1.8 m of gait, and it would pass with the arm nailed down.
    let still_at = app.world().get::<Transform>(root).expect("transform").translation;
    assert!(
        (still_at - planted_at).length() < 1e-3,
        "the husk moved {:.3} m during his own wind-up — the hand travel below is a walk, not a \
         telegraph",
        (still_at - planted_at).length()
    );

    let windup_ticks = expected_ticks(husk.windup_s, d.game.simulation_hz);
    assert_eq!(
        held + 1,
        windup_ticks,
        "the wind-up was sampled over {} ticks, titan.ron says {windup_ticks}",
        held + 1
    );
    (start, end, held, rig)
}

/// ★ **F-053 — the wind-up moves the hand far enough to SEE.**
///
/// The hand's `GlobalTransform` at the first tick of `Windup` and at the last one, both
/// projected at `f = 935.3 px`, must be **≥ 150 px** apart at 40 m.
///
/// **A single screenshot cannot catch this.** A still frame of a titan with his arm down is
/// indistinguishable from a titan with no telegraph at all; only the two-sample delta separates
/// them. It goes red when the wind-up is a colour flash, a 5° twitch, or when the three pose
/// angles in `scale.ron` go to zero — and the predicted travel is checked at three distances, so
/// it also goes red when the rig's proportions stop agreeing with the numbers the criterion was
/// written against.
///
/// The sampling is [`windup_hand`].
#[test]
fn f053_the_wind_up_moves_the_hand_far_enough_to_see() {
    let d = data(&app());
    let husk = d.titan("husk").expect("husk");
    let (start, end, held, rig) = windup_hand(None);
    let windup_ticks = expected_ticks(husk.windup_s, d.game.simulation_hz);

    let travel_m = (end - start).length();

    // The pose is a pure function, so the same travel has to come out of it **without an app at
    // all**: the whole chain from the root down, rebuilt here out of `scale.ron`'s fractions and
    // `titan::rig`'s own two transform helpers. If the two disagree, the rig that is drawn is
    // not the rig the pure function describes — and that is exactly the failure a screenshot
    // cannot show. The sample is taken at the tick that really was the last one *inside* the
    // wind-up (`windup_ticks − 1`), where the ramp has not quite arrived at 140 deg yet.
    let pure = {
        let timing = TitanTiming::of(husk, d.game.simulation_hz);
        let angles = PoseAngles {
            windup_arm_deg: d.scale.titan.windup_arm_deg,
            windup_lean_deg: d.scale.titan.windup_lean_deg,
            strike_arm_deg: d.scale.titan.strike_arm_deg,
            roll_lean_deg: d.scale.titan.roll_lean_deg,
        };
        let hand_of = |pose: Pose| {
            let pelvis = Transform::from_xyz(0.0, rig.leg_m, 0.0);
            let hand_in_arm = Vec3::new(0.0, -rig.arm_m * 0.5, 0.0);
            (pelvis
                * torso_transform(&rig, pose.lean_deg)
                * arm_transform(&rig, true, pose.arm_deg))
            .transform_point(hand_in_arm)
        };
        let first = pose_of(TitanState::Windup, 0, &timing, &angles);
        let last = pose_of(TitanState::Windup, held as u32, &timing, &angles);
        assert!(
            last.arm_deg > 130.0 && last.lean_deg > 10.0,
            "the last tick of the wind-up stands at {last:?} — that is not a telegraph"
        );
        (hand_of(last) - hand_of(first)).length()
    };

    let mut measured = Vec::new();
    for (distance_m, predicted_px) in [(20.0f32, 412.0f32), (40.0, 206.0), (100.0, 82.0)] {
        // The camera is placed so that the **midpoint** of the two hand positions is exactly
        // `distance_m` away: the criterion says "at 40 m", and the only unambiguous reading of
        // that is the thing being measured.
        let eye = (start + end) * 0.5 + Vec3::X * distance_m;
        let delta_px = (to_screen_px(end, eye) - to_screen_px(start, eye)).length();
        measured.push((distance_m, delta_px, predicted_px));
    }

    let at_40 = measured[1].1;
    assert!(
        at_40 >= 150.0,
        "the striking hand moves {at_40:.0} px at 40 m during the wind-up; the criterion is \
         150 px. Hand travel {travel_m:.3} m over {windup_ticks} ticks"
    );
    for (distance_m, delta_px, predicted_px) in &measured {
        let off = (delta_px - predicted_px).abs() / predicted_px;
        assert!(
            off < 0.10,
            "at {distance_m} m the hand moves {delta_px:.0} px, `docs/PLAN-GAME.md` §8 F-053 \
             predicts {predicted_px:.0} px — {:.0} % out. The prediction is built on the rig \
             fractions in scale.ron (shoulder {:.2} × h, arm {:.2} × h), so either the pose or \
             those fractions are not what the criterion was written against",
            off * 100.0,
            d.scale.titan.shoulder_height_fraction,
            d.scale.titan.arm_fraction
        );
    }
    assert!(
        (travel_m - pure).abs() < 0.01,
        "the hand travelled {travel_m:.3} m in the world and `pose_of`/`arm_transform` say \
         {pure:.3} m — the rig that is drawn is not the rig the pure function describes"
    );

    println!(
        "F-053 wind-up: hand travels {travel_m:.3} m over {windup_ticks} ticks ({} deg of arm, \
         {} deg of lean) — {}",
        d.scale.titan.windup_arm_deg,
        d.scale.titan.windup_lean_deg,
        measured
            .iter()
            .map(|(m, px, want)| format!("{px:.0} px at {m:.0} m (predicted {want:.0})"))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}

// ---------------------------------------------------------------------------
// F-034 — the impact frame, on the titan's side of it
// ---------------------------------------------------------------------------

/// **The half of the hit stop that was missing: the titan.**
///
/// `combat::hitstop::begin` puts `HitStop` on both bodies and `RigidBodyDisabled` on the player.
/// On a titan that marker does nothing at all — his position is written by `titan::brain::walk`
/// and never by avian (`RigidBody::Kinematic` + `CustomPositionIntegration`) — so before this
/// round a graze froze the player and the titan walked straight on through the impact frame.
/// `src/combat/hitstop.rs` says so in as many words and names this line as the fix.
///
/// Goes red the moment the gate in [`walk`](defeated_by_titan::titan::brain::walk) is taken out
/// again: the titan then covers a full `speed_m_s` worth of ground during a freeze the player
/// spends standing still.
///
/// ⚠️ **The number moved on 2026-08-19 and this test is where it showed** (`F-032`). Until then
/// both bodies were frozen for `gear.ron: feel.hit_stop_normal_s` — 2 ticks, 33 ms, which is why
/// a body cut read as nothing at all. Since the split, `feel.hit_stop_normal_s` is the
/// **player's** impact frame and `titan.ron: <kind>.stagger_s` is what the TITAN loses. This
/// test read the wrong one of the two and went red on "did not pick his walk back up" — the
/// husk was still frozen, for eleven more ticks than it expected.
#[test]
fn f032_a_grazed_titan_holds_still_for_his_whole_stagger() {
    let mut app = app_with_hits();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");
    let hz = d.game.simulation_hz;
    let stop_ticks = expected_ticks(husk.stagger_s, hz) as u64;
    assert!(stop_ticks > 0, "titan.ron: husk.stagger_s rounds to zero ticks");
    assert!(
        stop_ticks > expected_ticks(d.gear.feel.hit_stop_normal_s, hz) as u64,
        "husk.stagger_s is no longer than the player's own impact frame — then a body cut buys \
         the player nothing he can see, which is the hole F-032 was opened for"
    );

    // Far enough that he walks instead of attacking: `attack_range_m` is 6 m, `aggro_radius_m`
    // is 45 m.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 25.0));
    let root = the_titan(&mut app);
    let id = *app.world().get::<TitanId>(root).expect("TitanId");
    // Let the gait run up to `speed_m_s`, or "he did not move" is true for the wrong reason.
    ticks(&mut app, (husk.speed_m_s / husk.accel_m_s2 * hz as f32).ceil() as u64 + 10);
    assert_eq!(app.world().get::<TitanState>(root), Some(&TitanState::Pursue));

    let free_before = app.world().get::<Transform>(root).expect("transform").translation;
    ticks(&mut app, stop_ticks);
    let free_after = app.world().get::<Transform>(root).expect("transform").translation;
    let walked_m = (free_after - free_before).length();

    // A **non-lethal** hit: `Torso`, which `titan::brain::receive_hits` deliberately ignores.
    // The only thing it may do to this body is stop it.
    app.world_mut().write_message(TitanHit {
        titan: id,
        by: PlayerId(1),
        zone: HitZone::Torso,
        speed_m_s: 30.0,
    });
    let frozen_before = app.world().get::<Transform>(root).expect("transform").translation;
    ticks(&mut app, stop_ticks);
    let frozen_after = app.world().get::<Transform>(root).expect("transform").translation;
    let crept_m = (frozen_after - frozen_before).length();

    assert_eq!(
        app.world().get::<TitanState>(root),
        Some(&TitanState::Pursue),
        "a torso hit changed the state — only the cortex kills (shared/message.rs:21)"
    );
    assert!(
        crept_m < walked_m * 0.05,
        "the husk covered {crept_m:.4} m during a {stop_ticks}-tick hit stop and {walked_m:.4} m \
         in the same number of free ticks — the freeze does not reach the titan"
    );
    // And it is a freeze, not a stumble: he leaves it at the speed he had.
    // Two ticks of slack for the end of the freeze — it is inserted by `Commands` and counted
    // down in `PostStep`, and where the sync point falls is Bevy's business — and then the same
    // window again, so that "he is walking" is measured against the same distance "he walked"
    // was. A fixed six ticks is not enough any more: the stagger is 13 and the free window it is
    // compared with grew with it.
    ticks(&mut app, stop_ticks + 2);
    let moving_again = app.world().get::<Transform>(root).expect("transform").translation;
    assert!(
        (moving_again - frozen_after).length() > walked_m * 0.5,
        "the husk did not pick his walk back up after the stagger — a stagger that does not end \
         is a lock, and `titan.ron: husk.stagger_s` = {} s is supposed to be {stop_ticks} ticks",
        husk.stagger_s
    );
    println!(
        "F-032 titan side: {walked_m:.4} m in {stop_ticks} free ticks, {crept_m:.4} m in \
         {stop_ticks} staggered ticks (titan.ron husk.stagger_s = {} s)",
        husk.stagger_s
    );
}

/// **Rule 5's second half for F-053: the pose angles go to zero and the telegraph goes dark.**
///
/// `docs/PLAN-GAME.md` §8 names this failure by name — *"goes red when ... the pose angles in
/// RON go to zero"* — and a criterion whose failure mode has never been produced is a criterion
/// nobody has tested. So it is produced here, in the resource and not in the file: the same
/// husk, the same 36 ticks, the same measurement, with `windup_arm_deg` and `windup_lean_deg`
/// set to 0. The hand has to stop moving, and the pixel delta has to fall under the criterion.
#[test]
fn f053_the_telegraph_goes_dark_when_the_pose_angles_go_to_zero() {
    let (start, end, held, _) = windup_hand(Some((0.0, 0.0)));
    let travel_m = (end - start).length();
    let eye = (start + end) * 0.5 + Vec3::X * 40.0;
    let delta_px = (to_screen_px(end, eye) - to_screen_px(start, eye)).length();
    assert!(
        travel_m < 0.01 && delta_px < 150.0,
        "with the pose angles at zero the hand still travels {travel_m:.3} m over {held} ticks \
         ({delta_px:.0} px at 40 m) — the wind-up is being animated by something other than \
         `scale.ron`, and the \u{2605} test would pass with the file emptied out"
    );
    println!(
        "F-053 red counterpart: pose angles 0/0 -> hand travels {travel_m:.4} m, {delta_px:.1} px \
         at 40 m (criterion 150)"
    );
}

// ---------------------------------------------------------------------------
// F-030 · a swapped model decides where its titan DIES, not only where it renders
//
// `docs/FINDINGS.md` FIND-052 named this the single highest-value follow-up of the model
// registry, and the reason a swap was cosmetic only: `ModelAnchors` was written, tested and
// read by nobody, while `rig::build_rig` kept computing the cortex out of `scale.ron`. A
// swapped model therefore rendered in one place and died in another — invisibly, because both
// places look right on their own.
//
// There is no `.glb` in this repository and there must not be one, so the anchor is put on the
// entity the way `render::model::read_the_models_anchors` puts it there. What that proves is
// the READER; that Blender writes an empty called `cortex` at all is a claim about a file and
// stays ⬜ (`docs/models.md`).
// ---------------------------------------------------------------------------

#[test]
fn f030_a_models_cortex_anchor_beats_the_computed_position() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");
    let computed = d.titan_cortex_height_m(husk).expect("husk cortex height");

    // Far outside `aggro_radius_m`: `Idle` is the upright pose the heights are defined against.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -200.0));
    ticks(&mut app, 2);
    let root = the_titan(&mut app);
    let cortex = part_entity(&app, root, TitanPart::Cortex).expect("the rig has a cortex");

    // **The fallback first**, because it is the one that must not break: no anchors, and the
    // computed position stands exactly as it did before this system existed.
    let before = app.world().get::<GlobalTransform>(cortex).expect("global").translation();
    assert!(
        (before.y - computed).abs() < 0.01,
        "without a model the cortex has to stay where scale.ron puts it ({computed} m), it is \
         at {} m",
        before.y
    );

    // Now the model brings its own. 0.4 m higher and 0.1 m further back than the size table —
    // a plausible modelling decision, and one nothing in the rig can guess.
    // The anchor is given in the MODEL ROOT's own space — metres above the origin between the
    // feet — so the titan's own position in the world is what it has to be read against.
    let origin = app.world().get::<Transform>(root).expect("root transform").translation;
    let anchor = Vec3::new(0.0, computed + 0.4, before.z - origin.z + 0.1);
    let mut anchors = ModelAnchors::default();
    anchors.0.insert(CORTEX_ANCHOR.to_string(), anchor);
    app.world_mut().entity_mut(root).insert(anchors);
    ticks(&mut app, 2);

    let after = app.world().get::<GlobalTransform>(cortex).expect("global").translation();
    assert!(
        (after.y - anchor.y).abs() < 0.01,
        "the model's cortex empty sits at {:.2} m and the kill zone is at {:.2} m — the titan \
         renders where the modeller put it and dies where scale.ron computed it (F-030)",
        anchor.y,
        after.y
    );
    assert!(
        (after.z - origin.z - anchor.z).abs() < 0.01,
        "the anchor's depth was dropped: {:.2} m asked for, {:.2} m reached — the nape is a \
         point, not a height",
        anchor.z,
        after.z - origin.z
    );
    // The sensor moved, not a second one: one cortex, still under the head, still hittable.
    assert_eq!(
        part_entity(&app, root, TitanPart::Cortex),
        Some(cortex),
        "a second cortex was spawned instead of the one being moved"
    );
    assert!(
        app.world().get::<Collider>(cortex).is_some(),
        "the moved cortex lost its collider — it would render in the right place and never be hit"
    );
    println!(
        "F-030 anchor: scale.ron {computed:.2} m -> model {:.2} m, cortex measured at {:.2} m",
        anchor.y, after.y
    );
}

// ---------------------------------------------------------------------------
// F-030 · the model decides the nape's HEIGHT — it does not get to drag it to the front
//
// The 278-file drop of 2026-08-18 carries a `cortex` empty in every full body, and it does not
// sit where the rig puts one. Measured out of the files (node walk, `.glb` JSON chunk):
//
// | file | `cortex` empty, file frame | off the neck axis |
// |---|---|---|
// | `a-042-koerpertyp-a-hager-mittel.glb` | `(+0.010, +8.900, −0.139)` | **0.139 m** |
// | `a-042-koerpertyp-a-hager-gross.glb`  | `(+0.014, +12.460, −0.194)` | 0.194 m |
// | `a-040-titan-basis-rig.glb`           | `(+0.010, +8.900, −0.450)` | **0.450 m** |
// | `a-046-cortex-mesh.glb`               | `(+0.010, +8.900, −0.450)` | 0.450 m |
// | `a-049-lurker-koerper.glb`            | `(+0.014, +8.965, −1.245)` | 1.245 m |
//
// **The drop does not agree with itself**, and that is the first thing to say out loud: the
// pack's own base rig and its dedicated cortex part put the nape 0.45 m behind the neck, while
// the 26 body variants put it at 0.139 m — which is exactly where their `halswulst` mesh sits
// (`(+0.010, +8.443, −0.139)`), i.e. on the **skin of a neck that is about 0.36 m deep**. The
// body files carry the point where the amber would be glued on; the cortex part carries the
// middle of the amber itself (its mesh spans z −0.20 … −0.66).
//
// The rig's box is not that body. `scale.ron: width_fraction 0.25` makes a husk **2.5 m deep**,
// and `titan.ron: cortex_radius_m 0.55` makes the kill zone 1.1 m across. A nape 0.139 m behind
// the axis of a 2.5 m box is not a nape, it is a throat: the sphere then reaches to z −0.41,
// and `gear.ron: blades.reach_m` fishes it out from the **front** at 0.20 m of air. That is the
// husk's whole lesson deleted by a modelling detail
// ([`q030_the_nape_is_cut_from_behind_and_not_from_the_front`]).
//
// So the depth is the rig's and the height is the model's — with one exception that costs
// nothing and is the reason this is a clamp and not a deletion: a model that puts its nape
// **further back** than the rig does (the lurker, 1.245 m) is believed, because that direction
// can only make the approach angle sharper, never softer.
// ---------------------------------------------------------------------------

/// The `cortex` empty of `a-042-koerpertyp-a-hager-mittel.glb`, in the file's own frame.
const DROP_CORTEX_MEDIUM: Vec3 = Vec3::new(0.010, 8.900, -0.139);
/// The same empty in `a-042-koerpertyp-a-hager-gross.glb`.
const DROP_CORTEX_LARGE: Vec3 = Vec3::new(0.014, 12.460, -0.194);
/// `a-049-lurker-koerper.glb` — the one full body that puts its nape a long way back.
const DROP_CORTEX_LURKER: Vec3 = Vec3::new(0.014, 8.965, -1.245);

/// A drop anchor brought into the game the way `render::model::read_the_models_anchors` brings
/// it: scaled by `fit_to_class` (which matches the class's cortex height exactly when the model
/// carries a cortex) and then turned by `MODEL_FACES` — 180° about Y, because the pack is
/// authored facing +Z and this game's forward is −Z. A rotation by π negates x and z.
///
/// It is arithmetic and not a second convention: both steps are `render::model`'s, spelled out
/// here so that this file states where its numbers come from instead of hiding a magic vector.
fn drop_anchor(raw: Vec3, class_cortex_m: f32) -> Vec3 {
    let fit = class_cortex_m / raw.y;
    Vec3::new(-raw.x * fit, class_cortex_m, -raw.z * fit)
}

/// ★ **The drop's own nape, flown at.** A bound model may not make a titan cuttable from the front.
///
/// This is [`q030_the_nape_is_cut_from_behind_and_not_from_the_front`] run against a titan whose
/// kill zone was placed by a `.glb` instead of by `scale.ron` — the case that does not exist yet
/// in `art.ron` and will exist the moment one row says `Gltf(...)`. Red before the clamp: the
/// husk's front pass lands, and the design's central rule is gone from the running game with
/// nothing on fire.
#[test]
fn f030_a_bound_model_cannot_drag_the_nape_round_to_the_front() {
    let d = data(&app());
    for (kind, raw) in [("husk", DROP_CORTEX_MEDIUM), ("warden", DROP_CORTEX_LARGE)] {
        let k = d.titan(kind).unwrap_or_else(|| panic!("titan.ron has no {kind}"));
        let anchor = drop_anchor(raw, d.titan_cortex_height_m(k).expect("cortex height"));
        let behind =
            fly_past_a_titan(kind, Vec3::NEG_Z, AIR_M, 30.0, None, Tracking::Off, Some(anchor), 0.0);
        let front =
            fly_past_a_titan(kind, Vec3::X, AIR_M, 30.0, None, Tracking::Off, Some(anchor), 0.0);
        assert!(
            behind.cortex_tick.is_some(),
            "{kind} with the drop's anchor: the pass from behind did not land (blade {:+.3} m). \
             Binding a model may not cost the cut that F-030 stands on",
            behind.blade_gap_m
        );
        assert!(
            front.cortex_tick.is_none(),
            "{kind} with the drop's anchor bound: a pass from the FRONT cut the cortex, blade \
             {:+.3} m from it. The model's empty sits {:.3} m behind the neck axis and the \
             rig's box is {:.2} m deep — honouring that depth puts the kill zone inside the \
             chest and deletes the approach angle (Q-030)",
            front.blade_gap_m,
            -raw.z,
            d.scale.titan.width_fraction * d.titan_height_m(k).expect("height")
        );
        println!(
            "F-030 bound {kind}: behind cut on tick {:?} (blade {:+.3} m) · front no cut \
             (blade {:+.3} m short)",
            behind.cortex_tick, behind.blade_gap_m, front.blade_gap_m
        );
    }
}

/// The clamp is not a blanket override: a nape the model puts **further back** is believed.
///
/// The lurker's body is the only full body in the drop that does it (1.245 m in its own frame,
/// 1.74 m once it is fitted to `large`), and this is the falsifiable half of the rule above — if
/// [`TitanRig::cortex_in_head_from_model`] ever became "always the rig's depth", this test is
/// what goes red, and it is what keeps a real modelling decision from being thrown away
/// silently.
///
/// ⚠️ **It asserts geometry and not a cut, on purpose.** Measured 2026-08-18: the lurker's nape
/// is not reachable at the 0.20 m of air every other kind is flown at — blade +0.080 m short
/// with the computed nape, +0.060 m with the model's. That is `titan.ron`'s doing
/// (`cortex_radius_m: 0.50` on a `large` body: 1.60 + 0.50 + 0.12 = 2.22 m of blade against
/// 1.75 + 0.35 = 2.10 m of clearance, so 0.12 m of air and not 0.20 m), it is the same before
/// and after this file was touched, and it is reported as a finding rather than smuggled into
/// this test as a number picked to make it pass.
#[test]
fn f030_a_nape_the_model_puts_further_back_is_believed() {
    let d = data(&app());
    let k = d.titan("lurker").expect("titan.ron has no lurker");
    let rig = TitanRig::of(&d, k).expect("the lurker has a size class");
    let anchor = drop_anchor(DROP_CORTEX_LURKER, d.titan_cortex_height_m(k).expect("cortex"));
    let local = rig.cortex_in_head_from_model(anchor);
    assert!(
        anchor.z > rig.head_m * 0.5,
        "the fixture is wrong, not the code: the lurker's anchor at {:.3} m is not behind the \
         rig's own nape at {:.3} m, so this test exercises the clamp instead of the branch \
         around it",
        anchor.z,
        rig.head_m * 0.5
    );
    assert!(
        (local.z - anchor.z).abs() < 1e-4,
        "a nape further back than the rig's has to be taken as it stands: {:.3} m asked for, \
         {:.3} m kept, and the rig's own depth is {:.3} m",
        anchor.z,
        local.z,
        rig.head_m * 0.5
    );
    println!(
        "F-030 lurker: model asks {:.3} m behind the neck, the rig would say {:.3} m, kept {:.3} m",
        anchor.z,
        rig.head_m * 0.5,
        local.z
    );
}

/// The three components of the conversion, one assertion each, with no app around them.
///
/// Height from the model, depth never in front of the rig's, side from the model. Goes red the
/// moment somebody "simplifies" the clamp back to a subtraction.
#[test]
fn f030_the_model_decides_the_napes_height_and_the_rig_its_depth() {
    let d = data(&app());
    let husk = d.titan("husk").expect("husk");
    let rig = TitanRig::of(&d, husk).expect("husk rig");
    let computed = rig.cortex_in_head();
    let anchor = drop_anchor(DROP_CORTEX_MEDIUM, d.titan_cortex_height_m(husk).expect("cortex"));
    let local = rig.cortex_in_head_from_model(anchor);

    // Height: the model's, to the millimetre, and it is the whole point of binding at all.
    assert!(
        (local.y - (anchor.y - rig.head_centre_m())).abs() < 1e-4,
        "the model's cortex height was not taken: {:.3} m in the model, {:.3} m in the head",
        anchor.y,
        local.y
    );
    // Depth: the rig's, because the rig's box is what the blade has to reach past.
    assert!(
        (local.z - computed.z).abs() < 1e-4,
        "the nape is {:.3} m behind the neck and the rig puts it at {:.3} m — a model 0.139 m \
         off the axis of a {:.2} m deep box is a throat, not a nape",
        local.z,
        computed.z,
        rig.width_m
    );
    // Side: the model's. No design rule is about left and right, and the drop's own x is
    // 0.010 m, i.e. authoring noise that nothing should be re-centred for.
    assert!(
        (local.x - anchor.x).abs() < 1e-4,
        "the model's lateral offset was dropped: {:.3} m in the model, {:.3} m in the head",
        anchor.x,
        local.x
    );
    println!(
        "F-030 conversion: model ({:.3}, {:.3}, {:.3}) -> head ({:.3}, {:.3}, {:.3}), rig \
         depth {:.3} m",
        anchor.x, anchor.y, anchor.z, local.x, local.y, local.z, computed.z
    );
}

// ---------------------------------------------------------------------------
// F-072 · the sortie's titans do not outlive the sortie
//
// The hub loop closed on 2026-08-12 (hub → pad → sortie → verdict → hub) and left a hole
// nobody could see from inside `mission`: **a sortie that ends does not take its titans with
// it.** They keep their rig, their `RigidBody::Kinematic` and their brain, so they keep walking
// — through the debrief, through the transition, and into the hub, where the player lands next
// to the enemies of the mission he just finished. A second sortie then starts on a field that
// still holds the first one's.
//
// It is invisible to every test that existed, because all of them either kill every titan they
// spawn (`f070-hub.txt`, `f030-cortex.txt`) or never leave `Active` at all. The field is only
// wrong in the run that *survives* the verdict.
//
// Who clears it is a rule-4 question, not a taste question: `titan` owns titan bodies
// (`docs/architecture.md`, authority table), so `titan` is what ends them. The second test
// below is the falsifiable half — it takes `titan`'s own lifetime marker off one titan and
// demands that the very same transition then leaves him standing. If `mission` ever grows a
// system that reaches into a rig, that test is what goes red.
// ---------------------------------------------------------------------------

/// Sets the mission phase from the outside and lets the `StateTransition` schedule apply it.
///
/// One `update()` is enough: bevy runs `StateTransition` exactly once per frame
/// (`bevy_state-0.19.0/src/app.rs:335`), and `despawn_entities_on_exit_state` runs inside it,
/// so what this returns is the world **after** the transition's despawns have been flushed.
fn set_phase(app: &mut App, phase: MissionPhase) {
    app.world_mut().resource_mut::<NextState<MissionPhase>>().set(phase);
    app.update();
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        phase,
        "the phase did not take — a NextState set from outside is applied in the next frame"
    );
}

/// Every entity that belongs to some titan's rig — root, pelvis, legs, torso, arms, head,
/// cortex. A despawned root that left its limbs behind is nine orphans with colliders in them,
/// and `titan_roots` alone would call that a cleared field.
fn rig_parts(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<TitanPart>>();
    q.iter(app.world()).count()
}

#[test]
fn f072_a_finished_sortie_leaves_no_titans_standing() {
    let mut app = app();
    set_phase(&mut app, MissionPhase::Active);

    // Two, and one of them far outside `aggro_radius_m`: what is under test is the lifetime,
    // and a titan that walks into the player would end for a different reason.
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -60.0));
    spawn(&mut app, "husk", Vec3::new(30.0, 0.0, -200.0));
    ticks(&mut app, 2);
    assert_eq!(titan_roots(&mut app).len(), 2, "the field is supposed to be full first");
    let parts_before = rig_parts(&mut app);
    assert!(parts_before >= 16, "two rigs are at least sixteen parts, found {parts_before}");

    // The verdict falls. `Active` is exited, and with it every body that was fighting in it.
    set_phase(&mut app, MissionPhase::Won);

    assert_eq!(
        titan_roots(&mut app).len(),
        0,
        "the sortie is over and its titans are still standing — they walk through the debrief \
         and into the hub"
    );
    let parts_after = rig_parts(&mut app);
    assert_eq!(
        parts_after, 0,
        "the roots are gone but {parts_after} rig part(s) are left behind — limbs and cortex \
         sensors with no titan on them"
    );
}

#[test]
fn f072_a_lost_sortie_clears_the_field_too() {
    // `Lost` is the arm nobody writes a script for, and it is the one the bug actually bites
    // in: a mission lost on the clock has by definition NOT killed its titans.
    let mut app = app();
    set_phase(&mut app, MissionPhase::Active);
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -60.0));
    ticks(&mut app, 2);
    assert_eq!(titan_roots(&mut app).len(), 1);

    set_phase(&mut app, MissionPhase::Lost);
    assert_eq!(
        titan_roots(&mut app).len(),
        0,
        "a lost sortie leaves its titans standing — the ones that beat you follow you home"
    );
}

#[test]
fn f072_the_field_is_cleared_by_titan_and_by_nothing_else() {
    // ⭐ The rule-4 half, and the only one that cannot be faked. The claim is not "the field is
    // empty after the verdict" — that a `mission` system reaching into a rig would satisfy just
    // as well. The claim is **`titan` is what ends a titan**: the lifetime hangs on the rig
    // root, put there by `titan::spawn_titan`, and taking it off has to be enough to make the
    // very same transition leave the body alone.
    let mut app = app();
    set_phase(&mut app, MissionPhase::Active);
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -60.0));
    ticks(&mut app, 2);
    let root = the_titan(&mut app);

    assert!(
        app.world().get::<DespawnOnExit<MissionPhase>>(root).is_some(),
        "the rig root carries no lifetime at all — nothing scopes a titan to its sortie"
    );
    app.world_mut().entity_mut(root).remove::<DespawnOnExit<MissionPhase>>();

    set_phase(&mut app, MissionPhase::Won);
    assert_eq!(
        titan_roots(&mut app).len(),
        1,
        "a titan without `titan`'s own lifetime marker died at the verdict anyway — something \
         outside `titan` is despawning titan bodies (docs/architecture.md, authority table)"
    );
}

#[test]
fn f072_a_second_sortie_starts_on_an_empty_field() {
    // The acceptance of the round, and the shape the player actually meets: fly one sortie,
    // ride the verdict back to the hub, deploy again. Before this lifetime existed the second
    // sortie's `Active` began with the first sortie's titans already on the ring.
    let mut app = app();

    set_phase(&mut app, MissionPhase::Active);
    for i in 0..3 {
        spawn(&mut app, "husk", Vec3::new(20.0 * i as f32, 0.0, -80.0));
    }
    ticks(&mut app, 2);
    let first_sortie = titan_roots(&mut app);
    assert_eq!(first_sortie.len(), 3, "sortie 1 has three titans in it");

    // Counted by IDENTITY and not by number: standing on a pad in the hub can order a real
    // sortie, and sortie 2's own first wave arriving would make a plain `== 0` green for the
    // wrong reason on the day somebody edits `missions.ron`. What is under test is whether
    // sortie 1's three bodies are still there.
    let survivors = |app: &App| -> usize {
        first_sortie.iter().filter(|e| app.world().get_entity(**e).is_ok()).count()
    };

    set_phase(&mut app, MissionPhase::Won);
    set_phase(&mut app, MissionPhase::Hub);
    assert_eq!(
        survivors(&app),
        0,
        "the player walks into the hub next to the titans of the sortie he just left"
    );

    set_phase(&mut app, MissionPhase::Deploying);
    set_phase(&mut app, MissionPhase::Active);
    assert_eq!(
        survivors(&app),
        0,
        "sortie 2 opens on a field that still holds sortie 1 — the ring is full before the \
         first wave is released"
    );
}

// ---------------------------------------------------------------------------
// Q-031 — a titan turns inside his own reach
// ---------------------------------------------------------------------------

/// Parks the local player at `at_m` with no gravity and no velocity, so that the titan's target
/// stands still for the length of a measurement.
fn park_player(app: &mut App, at_m: Vec3) {
    let p = the_player(app);
    app.world_mut().entity_mut(p).insert((
        Transform::from_translation(at_m),
        GravityScale(0.0),
        LinearVelocity(Vec3::ZERO),
    ));
}

/// The body's yaw in degrees. `brain::walk` is the only writer of it.
fn yaw_deg(app: &App, root: Entity) -> f32 {
    app.world()
        .get::<Transform>(root)
        .expect("a titan root has a Transform")
        .rotation
        .to_euler(EulerRot::YXZ)
        .0
        .to_degrees()
}

/// ★ **The test that finally makes `turn_deg_per_s` a number.**
///
/// Until `Q-031` was answered the turn in [`brain::walk`] ran under
/// `state == Pursue && distance_m > attack_range_m` — so a titan **did not turn inside his own
/// attack range**, in any state (`docs/FINDINGS.md` FIND-012). The husk's `turn_deg_per_s: 50`
/// was the number the user singled out as "the one that decides everything", and it decided
/// nothing: every fight happens inside 6 m.
///
/// The player stands 5 m to the +X of a husk who looks down −Z: **90° off**, and inside
/// `attack_range_m`, so `Pursue → Windup` fires on the second tick and the old guard would never
/// once let him turn. What is asserted is not "he turned" but **how fast** — `turn_deg_per_s`
/// out of the file, times the ticks of the wind-up — and that he gets nowhere near the 90° he
/// wants, because a snap would pass a "he turned" assertion.
#[test]
fn q031_a_titan_turns_while_winding_up() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let hz = d.game.simulation_hz as f32;

    park_player(&mut app, Vec3::new(5.0, 0.5, 0.0));
    assert!(
        5.0 < husk.attack_range_m,
        "the player has to stand INSIDE attack_range_m ({} m) or this measures the old \
         Pursue-only turn",
        husk.attack_range_m
    );
    spawn(&mut app, "husk", Vec3::ZERO);
    let root = the_titan(&mut app);

    // Sample the yaw on the first tick of `Windup` and on the last one, and count the ticks
    // between the two samples — that is exactly how many times `walk` got to turn him.
    let mut first: Option<f32> = None;
    let mut last = 0.0f32;
    let mut steps = 0u32;
    for _ in 0..600 {
        app.update();
        if app.world().get::<TitanState>(root) != Some(&TitanState::Windup) {
            if first.is_some() {
                break; // the wind-up is over
            }
            continue;
        }
        let yaw = yaw_deg(&app, root);
        if first.is_none() {
            first = Some(yaw);
        } else {
            steps += 1;
        }
        last = yaw;
    }
    let first = first.expect("the husk never wound up — nothing is being measured");
    assert!(steps > 0, "the wind-up was one tick long — there is no rate to measure");

    let turned = (last - first).abs();
    let wanted = husk.turn_deg_per_s * steps as f32 / hz;
    assert!(
        turned > 0.1,
        "the husk turned {turned:.3}° over {steps} ticks of his own wind-up while the player \
         stood 90° off his shoulder. `turn_deg_per_s` is {} and it governs nothing \
         (src/titan/brain.rs::walk, docs/FINDINGS.md FIND-012)",
        husk.turn_deg_per_s
    );
    assert!(
        (turned - wanted).abs() < 0.5,
        "the husk turned {turned:.3}° over {steps} ticks; {} °/s out of titan.ron over {steps} \
         ticks at {hz} Hz is {wanted:.3}°",
        husk.turn_deg_per_s
    );
    // He wants 90° and he may not have them: the wind-up is a tracking window, not a snap.
    assert!(
        turned < 89.0,
        "the husk covered {turned:.3}° of the 90° he wanted inside one wind-up — that is a snap, \
         and an approach angle you cannot keep is not an approach angle"
    );
    println!(
        "Q-031 husk wind-up turn: {turned:.3}° over {steps} ticks at {} °/s (wants 90°)",
        husk.turn_deg_per_s
    );
}


/// ★ **Is the approach angle a thing this game has?** — `Q-031`'s own title, re-answered on
/// 2026-08-20 with a different instrument, because the old answer stopped being measurable.
///
/// ## What this test used to assert, and why that is gone
///
/// A **snapshot**: *the warden misses at 0.20 m of air and lands at 0.15*. Two things were
/// wrong with it. `FIND-089` put the margin behind that must-miss at **0.020 m of blade**, and
/// `FIND-123` then showed the number was measured through a contaminated path — the same swing
/// that cut the nape had opened the warden's guard with its own torso graze. A must-miss on a
/// contaminated 2 cm is a tripwire on noise, and `gear.ron`'s `reach_m` 1.6 → 2.0 plus
/// `thickness_m` 0.12 → 0.20 duly tripped it.
///
/// ## What the approach angle IS now, and it is a rule instead of a remainder
///
/// `titan.ron: cortex_half_angle_deg` — the nape may only be cut from inside an arc off the
/// titan's own **backward** vector (`blades::cut::nape_is_exposed`). So the question stopped
/// being *how many centimetres of margin does his turn eat* and became *at what BEARING does
/// his nape shut*, which is a degree and not a length. Measured here by yawing the body on the
/// spot and flying the identical line at it.
///
/// The ideal pass sits about **73°** off a husk's back and **71°** off a warden's, so the file's
/// 115° and 110° leave 42° and 39° of turn before the nape shuts. That is the number a player
/// feels: *you may cut him until he has come about 40° towards you.*
///
/// ⚠️ **The warden's contamination is NOT fixed here.** This fixture still opens his guard with
/// the pass's own torso graze. The clean two-pass version lives in `scripts/f030-hitbox.txt`,
/// where the opening pass is flown across his front, `assert titans == 1` proves it was not the
/// kill, and the nape pass comes from the other side afterwards.
///
/// ⚠️ **And one thing the round cost, recorded rather than hidden:** the turn no longer eats
/// measurable margin. Measured the same day with a 5 cm sweep of `air_m` from 0.00 to 1.60 m, a
/// warden's widest landing pass is **0.95 m with tracking on and 0.95 m with it off** — the
/// 10.7° he covers during a 16-tick pass no longer moves his nape out of a 0.30 m-thick, 2.00 m
/// blade. So the approach angle is carried **entirely by the gate**: a cliff at 110°, not the
/// gradient it used to be. That is a real loss and it belongs to the user's judgement, not to a
/// silent assert — `docs/QUESTIONS.md` Q-047.
#[test]
fn q031_the_nape_survives_a_titan_who_tracks_you() {
    let d = data(&app());
    let mut table = Vec::new();
    for kind in ["husk", "warden"] {
        // The tracked ideal pass still lands at every speed the fixture is flown at. This is the
        // half `F-030` may not lose: a nape that is only reachable at one speed is not reachable.
        for speed in [30.0_f32, 45.0, 60.0] {
            let p = fly_past_a_titan(kind, Vec3::NEG_Z, AIR_M, speed, None, Tracking::On, None, 0.0);
            assert!(
                p.cortex_tick.is_some(),
                "{kind} at {speed} m/s: no cut with {AIR_M} m of air (blade {:+.3} m). The nape \
                 of a tracking titan has stopped being reachable at the criterion's own offset, \
                 which is F-030 traded away for Q-031 and is not an acceptable price",
                p.blade_gap_m
            );
            assert!(p.closest_m > 0.0, "{kind} at {speed} m/s: the capsules touched");
        }

        // ---- and the gate, swept in degrees ------------------------------------------------
        //
        // The body is yawed on the spot before the identical line is flown at it, so the ONLY
        // thing that changes between two runs is the bearing the blade arrives on. Coarse on
        // purpose (5°): what is asserted is that a shut-off exists and sits near the file's own
        // number, not that it sits on a particular degree.
        let gate_deg = d.titan(kind).expect("kind").cortex_half_angle_deg;
        let mut shut_at = None;
        let mut turned = 0.0f32;
        while turned <= 90.0 {
            let p = fly_past_a_titan(kind, Vec3::NEG_Z, AIR_M, 30.0, None, Tracking::Off, None, turned);
            if p.cortex_tick.is_none() {
                // 🔴 The blade has to be INSIDE the cortex when this happens, or the pass was
                // refused by geometry and this sweep is measuring the wrong thing entirely.
                assert!(
                    p.blade_gap_m < 0.0,
                    "{kind}: the pass stopped landing at {turned}° of turn with the blade \
                     {:+.3} m SHORT of the cortex — that is the reach running out, not the gate. \
                     This sweep is not measuring cortex_half_angle_deg",
                    p.blade_gap_m
                );
                shut_at = Some((turned, p.blade_gap_m));
                break;
            }
            turned += 5.0;
        }
        let (shut_deg, gap_m) = shut_at.unwrap_or_else(|| {
            panic!(
                "{kind}: the nape was still cuttable with the body yawed 90° towards the pass. \
                 cortex_half_angle_deg is {gate_deg}° and it is governing nothing — the titan is \
                 a floating bullseye again, which is FIND-012's shape in the other direction"
            )
        });
        assert!(
            shut_deg > 10.0,
            "{kind}: the nape shuts after only {shut_deg}° of turn. A player cannot hold a line \
             that tight and the cut will read as broken"
        );
        table.push((kind, gate_deg, shut_deg, gap_m));
    }

    println!(
        "Q-031 the approach angle, in degrees: {}",
        table
            .iter()
            .map(|(k, gate, shut, gap)| format!(
                "{k} shuts after {shut:.0}° of turn (gate {gate:.0}°, blade {gap:+.2} m inside)"
            ))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}


// ===========================================================================================
// `F-029` Dynamische Ankerpunkte — **a titan holds a rope, and the rope rides him.**
//
// > *„Ankerpunkte an bewegten Objekten: Titanenkörper (Schulter, Arm, Nacken) … Werden pro
// > Frame mit dem Trägerobjekt mitgeführt und beim Tod oder der Zerstörung des Trägers sauber
// > entfernt."*
//
// The acceptance is one sentence: *"Ein Haken an einem Titanen bleibt während dessen Bewegung
// korrekt verankert und löst sich beim Tod des Titanen mit Feedback."*
//
// **This is the half of `B-007` that is geometry and not wording.** `vector::aim` already hit
// the titan — it casts with avian's default filter on purpose, so that a rope can never travel
// *through* a wall — but no titan entity carried a [`Body`], so `hook::anchor_target` returned
// `None` at `aim.body?`, the arm stayed `Idle`, and the titan additionally **blocked** the good
// wall behind him. One component on the rig root closes both.
//
// Nothing here forces an `AimPoint`. The ray is the game's own, the index is the game's own and
// the walk is `titan::brain::walk` — a fixture that wrote the anchor itself would pass with the
// titan deleted, which is the one thing these tests must not do.
// ===========================================================================================

/// The lane the F-029 pair meets in — **the same trick `fly_past_a_titan` uses**: 60 m up, well
/// over Ashgate's tallest roof, so that the aim ray measures the titan and not a church.
const GRAPPLE_LANE_Y: f32 = 60.0;

/// Where the titan stands. His feet are the lane; he is kinematic and `brain::walk` is his only
/// writer, so he does not fall out of it.
const GRAPPLE_TITAN_M: Vec3 = Vec3::new(0.0, GRAPPLE_LANE_Y, 0.0);

/// The player: 14 m to the titan's −X side and 3 m up, which puts his eye inside the capsule's
/// vertical span (feet + 1.6 m … feet + 8.4 m on a 10 m husk). Inside `aggro_radius_m` (45 m),
/// because a hook into a statue proves nothing about a moving carrier.
const GRAPPLE_PLAYER_M: Vec3 = Vec3::new(-14.0, GRAPPLE_LANE_Y + 3.0, 0.0);

/// Parks the local player where he is put, weightless and still, looking at **+X**.
///
/// `look_dir()` is `(−sin yaw · cos pitch, sin pitch, −cos yaw · cos pitch)`, so the yaw whose
/// look is `+X` is `atan2(-1, 0)`. Through [`LookOverride`], which is the same absolute channel
/// the `--script` driver's `look` command uses — a test that wrote `Intent` straight onto the
/// player would be driving the game through a door nothing else uses.
fn hold_the_player(app: &mut App, at_m: Vec3) {
    let player = the_player(app);
    app.world_mut().entity_mut(player).insert((
        Transform::from_translation(at_m),
        GravityScale(0.0),
        LinearVelocity(Vec3::ZERO),
    ));
    app.world_mut().resource_mut::<LookOverride>().0 = Some((f32::atan2(-1.0, 0.0), 0.0));
    app.update();
}

fn arm(app: &App, player: Entity, side: Side) -> HookState {
    app.world().get::<Hook>(player).expect("the player carries a Hook").arm(side).state
}

fn arm_tip(app: &App, player: Entity, side: Side) -> Vec3 {
    app.world().get::<Hook>(player).expect("the player carries a Hook").arm(side).tip_m
}

/// Presses `Q` and runs until the left arm anchors, at most `limit` ticks.
fn grapple(app: &mut App, player: Entity, limit: u64) -> Option<u64> {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    for n in 1..=limit {
        app.update();
        if arm(app, player, Side::Left).is_anchored() {
            return Some(n);
        }
    }
    None
}

/// The pair, set up: a husk in the lane, the player 14 m off him and pointed at his flank.
///
/// 🔴 **The always-on pull is switched off here, and that is the fixture's whole job.**
/// `hold_the_player` says it *parks* the player — weightless and still — and until `FIND-172`
/// that one insert was enough. It is not any more: a hooked player in flight is winched at
/// `vector.drive_idle_speed_m_s` (12 m/s) with no key held, and the anchor of this fixture is
/// the titan himself. Measured before this line existed: the player was hauled from x=−14.0 to
/// **x=−5.675 in two seconds**, i.e. to 4.94 m of a husk whose `attack_range_m` is 6.0 — so the
/// husk stopped walking and stood in `Windup`, and `f029_…_rides_him` read `walked 0.000 m`.
/// He was never blind: the same run measured `saw=true`, `heard=0.853`, `detected=true` at
/// 14 m (the husk's cone is 110° half-angle, so a player abeam is inside the eye, and a
/// tethered player carries 34.8 m of noise against a 35 m ear). **The titan was not asleep;
/// the player had arrived.** Same shape and same fix as the two `F-004` fixtures `FIND-172`
/// turned it off in: `F-029`'s claim is that an anchor rides a moving carrier, and a fixture
/// that flies the player into the carrier measures the winch instead.
fn a_titan_in_the_crosshair(app: &mut App) -> (Entity, Entity) {
    app.world_mut().resource_mut::<GameData>().game.vector.drive_idle_speed_m_s = 0.0;
    spawn(app, "husk", GRAPPLE_TITAN_M);
    let root = the_titan(app);
    hold_the_player(app, GRAPPLE_PLAYER_M);
    // One for `world::index` to hand the new body its `BodyId`, one for the aim ray to find it.
    ticks(app, 2);
    let player = the_player(app);
    (root, player)
}

#[test]
fn f029_a_rope_bites_a_walking_titan_and_rides_him() {
    let mut app = app();
    let (root, player) = a_titan_in_the_crosshair(&mut app);

    // 1. The titan is a carrier at all — `B-007`'s `aim.body?`.
    let anchored_after = grapple(&mut app, player, 60)
        .expect("the rope found no anchor on a titan 30 m away and dead in the crosshair");
    let HookState::Anchored { body, local_m } = arm(&app, player, Side::Left) else {
        unreachable!()
    };
    let carried = app
        .world()
        .get::<BodyId>(root)
        .copied()
        .expect("the rig root carries no BodyId — world::index never took it in");
    assert_eq!(body, carried, "the rope hangs on something that is not the titan");

    // 2. It bit the body and not the air around it: the anchor sits inside the husk's own
    //    silhouette, half a width out at most.
    let rig = *app.world().get::<TitanRig>(root).expect("the root carries its rig");
    // `local_m` is measured from the root's origin, which lies **between the feet**. A point on
    // the body therefore sits inside the rig's own box: half a width to either side, and
    // between the ground and the crown.
    assert!(
        local_m.x.abs() <= rig.width_m * 0.5 + 1e-3
            && local_m.z.abs() <= rig.width_m * 0.5 + 1e-3
            && (0.0..=rig.height_m).contains(&local_m.y),
        "local_m {local_m:?} is outside the husk's own box ({:.2} m wide, {:.2} m tall) — that \
         is not a point on his body",
        rig.width_m,
        rig.height_m
    );

    // 3. **The ride.** He is inside `aggro_radius_m` (45 m) so `brain::walk` really carries him;
    //    the rope has to move by exactly what he moved by, and by nothing else.
    // A second of run-up first: `titan.ron: husk.accel_m_s2` is 3.0, so the first second is
    // spent reaching `speed_m_s` 3.0 — and under acceleration the index's one-tick propagation
    // lag is a real `a · dt · T` term (0.075 m over 1.5 s) that would be measured as drift. At
    // constant speed it is exactly zero, which is the number worth asserting.
    ticks(&mut app, 60);
    let titan_before = app.world().get::<GlobalTransform>(root).unwrap().translation();
    let tip_before = arm_tip(&app, player, Side::Left);
    ticks(&mut app, 60);
    let titan_after = app.world().get::<GlobalTransform>(root).unwrap().translation();
    let tip_after = arm_tip(&app, player, Side::Left);

    let walked = titan_after - titan_before;
    // The control the whole test hangs on: with a titan who did not move, an anchor nailed to
    // the world would pass every line below.
    assert!(
        walked.length() > 2.5,
        "the husk walked {:.3} m in a second at 3.0 m/s — this run proves nothing about a \
         MOVING carrier, and an anchor nailed to the world would pass every line below it",
        walked.length()
    );
    assert!(
        arm(&app, player, Side::Left).is_anchored(),
        "the rope let go while he was walking: {:?}",
        arm(&app, player, Side::Left)
    );
    let drift = (tip_after - tip_before) - walked;
    // **The bound is one tick of the carrier's own travel, and it is not a fudge factor.**
    // `world::index` reads the `GlobalTransform`, which is propagated in `PostUpdate` — after
    // the fixed step that moved the titan. So the hull the rope is read against is exactly one
    // tick old, and while the husk is *turning* towards the player that lag does not cancel: it
    // leaves a residual of one tick of velocity, 3.0 m/s / 60 Hz = 0.05 m. An anchor nailed to
    // the world would miss by the **whole** {walked} — sixty times this bound — so the test
    // still goes red the moment the ride stops working.
    let one_tick_m = walked.length() / 60.0;
    assert!(
        drift.length() <= one_tick_m * 1.2,
        "the anchor drifted {:.4} m off the body over a second of walking, against one tick of \
         his travel ({one_tick_m:.4} m): the tip moved {:?} while the titan moved {:?}",
        drift.length(),
        tip_after - tip_before,
        walked
    );
    println!(
        "F-029: anchored after {anchored_after} ticks at local {local_m:?}; the husk walked \
         {:.3} m in a second and the rope followed to within {:.4} m — one tick of his own \
         travel is {one_tick_m:.4} m",
        walked.length(),
        drift.length()
    );
}

#[test]
fn f029_the_rope_lets_go_when_the_titan_dies_and_says_why() {
    let mut app = app();
    app.init_resource::<ReleaseLog>();
    app.add_systems(Last, record_releases);
    let (root, player) = a_titan_in_the_crosshair(&mut app);
    grapple(&mut app, player, 60).expect("the rope found no anchor on the titan");
    let id = *app.world().get::<TitanId>(root).unwrap();
    app.world_mut().resource_mut::<ReleaseLog>().0.clear();

    // The cortex is cut. `titan::brain::receive_hits` puts him in `Death` in the same tick.
    app.world_mut().write_message(TitanHit {
        titan: id,
        by: PlayerId(1),
        zone: HitZone::Cortex,
        speed_m_s: 30.0,
    });
    // `Death` is decided in `Drive`; the `Body` goes with it, the observer files the removal and
    // `world::index` reports `BodyGone` in the **next** tick's `Spatial`, which is where
    // `vector::hook` reads it. Two ticks is the whole budget.
    ticks(&mut app, 2);

    assert!(
        !arm(&app, player, Side::Left).is_anchored(),
        "the rope is still taut on a dead titan: {:?}",
        arm(&app, player, Side::Left)
    );
    let reasons: Vec<ReleaseReason> =
        app.world().resource::<ReleaseLog>().0.iter().map(|r| r.reason).collect();
    assert!(
        reasons.contains(&ReleaseReason::BodyGone),
        "the release carried {reasons:?} instead of BodyGone — the player is told nothing about \
         why his rope went slack"
    );
    println!("F-029: the corpse released the rope with {reasons:?}");
}

#[test]
fn f029_without_the_titan_the_same_pull_holds_nothing_that_walks() {
    // **The refutation.** Same eye, same look, same trigger, **no titan** — and whatever the
    // 500 m of `hook_range_m` finds out there instead, it does not walk. The pair above claims
    // a moving carrier; this one shows that the movement came from the titan and not from the
    // rope code, which is the difference between measuring a husk and measuring a house.
    let mut app = app();
    hold_the_player(&mut app, GRAPPLE_PLAYER_M);
    ticks(&mut app, 2);

    let player = the_player(&mut app);
    match grapple(&mut app, player, 60) {
        None => {} // nothing out there at all — the strongest form of the same statement
        Some(_) => {
            let before = arm_tip(&app, player, Side::Left);
            ticks(&mut app, 120);
            let moved = (arm_tip(&app, player, Side::Left) - before).length();
            assert!(
                moved < 1e-3,
                "with no titan in the world the anchor still travelled {moved:.3} m in two \
                 seconds — then the ride the test above measures is not the titan's"
            );
        }
    }
    assert!(
        titan_roots(&mut app).is_empty(),
        "this control is only a control while the world really holds no titan"
    );
}

use defeated_by_titan::shared::{BodyId, Hook, HookReleased, HookState, ReleaseReason};

/// Every release the two arms reported, in order. A log of the run, not player state.
#[derive(Resource, Default)]
struct ReleaseLog(Vec<HookReleased>);

fn record_releases(mut log: ResMut<ReleaseLog>, mut released: MessageReader<HookReleased>) {
    log.0.extend(released.read().copied());
}


/// ★ **Rule 6, structurally: `F-032` added four hit zones and not one collider.**
///
/// The perf question this feature owes an answer to is *"limb colliders multiply collider
/// count"*, and the answer is that there are none. A titan carries exactly the two colliders he
/// carried on 2026-08-18 — the root capsule and the cortex sensor — so the broad phase, avian's
/// collider tree, `world::index` and every unfiltered ray in the game (`vector::aim`'s above
/// all) see a body that has not changed at all. The four [`HitZoneOf`] boxes are plain data and
/// are only ever looked at from `blades::cut::limb_zone`, on a tick where a blade already found
/// the body.
///
/// It is also the guard on the version of this feature that was built first and taken back out:
/// a `Sensor` per limb on a layer of its own **broke `F-029`** within the hour, because
/// `vector::aim` casts unfiltered and an arm sticks out of the capsule. This test is what goes
/// red if anybody rebuilds it that way.
#[test]
fn f032_the_limb_zones_are_data_and_the_body_still_carries_two_colliders() {
    let mut app = app();
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, -200.0));
    ticks(&mut app, 2);
    let root = the_titan(&mut app);

    let colliders: Vec<&str> = rig_entities(&app, root)
        .into_iter()
        .filter(|e| app.world().get::<Collider>(*e).is_some())
        .map(|e| app.world().get::<Name>(e).map(|n| n.as_str()).unwrap_or("?"))
        .collect();
    assert_eq!(
        colliders.len(),
        2,
        "the husk carries {} colliders ({colliders:?}). It carried two before F-032 — the root \
         capsule and the cortex sensor — and a limb collider is not a cheaper hit zone, it is a \
         surface `vector::aim`'s unfiltered hook ray runs into (docs/FINDINGS.md, F-029)",
        colliders.len()
    );

    let zones: Vec<HitZone> = rig_entities(&app, root)
        .into_iter()
        .filter_map(|e| app.world().get::<HitZoneOf>(e).map(|z| z.zone))
        .collect();
    assert_eq!(zones.len(), 4, "the husk publishes {} hit zones: {zones:?}", zones.len());
    for wanted in [HitZone::ArmLeft, HitZone::ArmRight, HitZone::LegLeft, HitZone::LegRight] {
        assert!(zones.contains(&wanted), "no {wanted:?} box on the rig: {zones:?}");
    }
    println!("F-032 the husk: colliders {colliders:?}, hit zones {zones:?}");
}

// ---------------------------------------------------------------------------
// F-051 — the perception model: a cone, an ear, and the noise a player makes
// ---------------------------------------------------------------------------

/// **The instrument.** Holds one player at a fixed place and tells the world he is moving at
/// `speed_m_s` in `movement`.
///
/// It runs in [`SimulationSystems::Intent`], which is **before** `Drive` and therefore before
/// `titan::perception::perceive` — and `Velocity` is the player's own domain's in `Integrate`,
/// which is after. So the value the titan's ear reads this tick is the value written here, and
/// the player's own systems get it back untouched on the next one.
///
/// **Why the position is pinned too.** A player really moving at 20 m/s covers 333 m in the
/// 1000 ticks this measurement takes, and then the number that came out would be about the
/// approach and not about the ear. Pinning both is what makes the two runs differ in **exactly
/// one** thing: whether the gas is on.
#[derive(Resource, Clone, Copy)]
struct PinnedPlayer {
    at: Vec3,
    speed_m_s: f32,
    movement: MovementState,
}

fn pin_player(
    pinned: Res<PinnedPlayer>,
    mut players: Query<(&mut Transform, &mut Velocity, &mut MovementState), With<PlayerId>>,
) {
    for (mut transform, mut velocity, mut movement) in &mut players {
        transform.translation = pinned.at;
        // Along +X, which is sideways to every titan in these tests: a speed that pointed at
        // the titan would be an approach, and the ear reads the magnitude anyway.
        velocity.0 = Vec3::new(pinned.speed_m_s, 0.0, 0.0);
        *movement = pinned.movement;
    }
}

/// An app with the instrument above installed.
fn app_with_pinned_player(at: Vec3, speed_m_s: f32, movement: MovementState) -> App {
    let mut app = app();
    app.insert_resource(PinnedPlayer { at, speed_m_s, movement });
    app.add_systems(FixedUpdate, pin_player.in_set(SimulationSystems::Intent));
    app
}

fn awareness_of(app: &mut App, root: Entity) -> Awareness {
    *app.world().get::<Awareness>(root).expect("every titan carries an Awareness")
}

/// Runs until this titan has noticed somebody, and hands back the tick it happened on.
/// `None` means he never did.
fn ticks_until_detected(app: &mut App, root: Entity, limit: u64) -> Option<u64> {
    for t in 0..limit {
        app.update();
        if awareness_of(app, root).detected {
            return Some(t + 1);
        }
    }
    None
}

/// ★ **`F-051`'s acceptance sentence, and it is measured rather than argued.**
///
/// *"Ein leise agierender Spieler wird spaeter entdeckt als ein boostender."* Three runs that
/// differ in **one** thing — the same lurker, the same 40 m, the same player, held in the same
/// spot by the same instrument:
///
/// | the player | noise radius | what the lurker's 75 m ear does |
/// |---|---|---|
/// | 20 m/s **on the rope** | `(8 + 24) × 1.6` = 51.2 m | hears him, and after `n` ticks comes |
/// | 20 m/s **falling** | `8 + 24` = 32.0 m | **nothing** — 32 m of noise does not reach 40 m |
/// | standing still | 8.0 m | nothing |
///
/// The number of ticks is **predicted out of `titan.ron`** and then measured, so a gain typed
/// into `titan/` instead of read from the file fails here. And the second row is the control
/// the habit next to rule 5 asks for: *delete the thing you think you are measuring and check
/// the number moves.* Same speed, same distance, gas off — and the detection disappears
/// entirely instead of merely getting later.
///
/// He is behind the lurker on purpose (`+Z`, and a titan spawns facing `−Z`): the eye is
/// instant and would answer the question before the ear ever got to.
#[test]
fn f051_a_player_under_gas_is_heard_where_the_same_speed_falling_is_not() {
    let at = Vec3::new(0.0, 2.0, 40.0);
    let speed_m_s = 20.0;

    // ---- the prediction, out of the file ------------------------------------------------
    let d = data(&app());
    let feel = d.titans.perception;
    let lurker = d.titan("lurker").expect("titan.ron has a lurker");
    let ear_m = lurker.hearing_radius_m;
    let hz = d.game.simulation_hz as f32;
    let noise_roped = (feel.quiet_m + speed_m_s * feel.noise_per_speed_m) * feel.rope_factor;
    let noise_free = feel.quiet_m + speed_m_s * feel.noise_per_speed_m;
    let reach = noise_roped.min(ear_m);
    let strength = (reach - at.z) / reach;
    let predicted = (1.0 / (feel.hearing_gain_per_s * strength) * hz).ceil() as u64;
    assert!(
        noise_free < at.z && noise_roped > at.z,
        "this measurement is only a measurement while the gas is what crosses the 40 m: \
         roped {noise_roped} m, free {noise_free} m"
    );

    // ---- 1. on the rope -----------------------------------------------------------------
    let mut app = app_with_pinned_player(at, speed_m_s, MovementState::Tethered);
    spawn(&mut app, "lurker", Vec3::ZERO);
    let root = the_titan(&mut app);
    let roped = ticks_until_detected(&mut app, root, 900);
    let roped = roped.expect("a lurker with a 75 m ear never heard a man boosting at 40 m");
    assert!(
        roped.abs_diff(predicted) <= 3,
        "heard him on tick {roped}, the file predicts {predicted} (noise {noise_roped:.1} m, \
         reach {reach:.1} m, strength {strength:.4}, gain {} /s)",
        feel.hearing_gain_per_s
    );
    let heard = awareness_of(&mut app, root);
    assert!(!heard.saw, "the lurker is supposed to have his back to him, not see him");
    assert!((heard.noise_m - noise_roped).abs() < 1e-3, "{} vs {noise_roped}", heard.noise_m);

    // ---- 2. the control: the same speed, gas off ----------------------------------------
    let mut quiet = app_with_pinned_player(at, speed_m_s, MovementState::Airborne);
    spawn(&mut quiet, "lurker", Vec3::ZERO);
    let root = the_titan(&mut quiet);
    let free = ticks_until_detected(&mut quiet, root, 900);
    assert_eq!(
        free, None,
        "the same 20 m/s WITHOUT the gas carries {noise_free:.1} m of noise and must not reach \
         40 m — this is the control run, and a detection here means the ear is reading \
         something other than the noise radius"
    );

    // ---- 3. the floor: standing still ---------------------------------------------------
    let mut still = app_with_pinned_player(at, 0.0, MovementState::Grounded);
    spawn(&mut still, "lurker", Vec3::ZERO);
    let root = the_titan(&mut still);
    assert_eq!(ticks_until_detected(&mut still, root, 300), None, "a motionless player at 40 m");

    println!(
        "F-051 lurker, 40 m, 20 m/s: roped noise {noise_roped:.1} m -> heard on tick {roped} \
         (predicted {predicted}); free noise {noise_free:.1} m -> never; standing 8.0 m -> never"
    );
}

/// ★ **The cone, end to end, in one world.** The eye is instant and the blind spot is real.
///
/// A husk stands at the origin facing `−Z` (a titan's spawn facing). The player stands 15 m
/// **behind** him and does nothing: no sight, and 8 m of noise does not carry 15 m. Then the
/// same player is put 15 m **in front** of him without a single number changing, and the husk
/// is in `Pursue` inside two ticks.
///
/// **What it goes red on.** Until 2026-08-25 `titan::brain::decide` read
/// `distance_m <= aggro_radius_m`, a circle of 45 m with no facing in it — under that rule the
/// first half of this test is `Pursue` too, and the whole of `F-051` is undetectable from
/// outside. Set `sight_half_angle_deg` to 180 in `titan.ron` and the first half goes red again.
#[test]
fn f051_a_husk_is_blind_behind_and_instant_in_front() {
    let behind = Vec3::new(0.0, 2.0, 15.0);
    let in_front = Vec3::new(0.0, 2.0, -15.0);

    let mut app = app_with_pinned_player(behind, 0.0, MovementState::Grounded);
    spawn(&mut app, "husk", Vec3::ZERO);
    let root = the_titan(&mut app);
    ticks(&mut app, 120);

    let blind = awareness_of(&mut app, root);
    let state = *app.world().get::<TitanState>(root).expect("a titan has a state");
    assert!(!blind.saw, "the husk saw a man standing dead behind him");
    assert_eq!(blind.heard, 0.0, "he heard a motionless man at 15 m");
    assert!(!blind.detected, "awareness {} after 120 ticks in the blind spot", blind.level);
    assert_eq!(state, TitanState::Idle, "he is chasing something he has not noticed");

    app.world_mut().resource_mut::<PinnedPlayer>().at = in_front;
    ticks(&mut app, 2);

    let seen = awareness_of(&mut app, root);
    let state = *app.world().get::<TitanState>(root).expect("a titan has a state");
    assert!(seen.saw && seen.detected, "the same man 15 m in front: saw {}, detected {}", seen.saw, seen.detected);
    assert_eq!(seen.level, 1.0, "sight is instant, never an accumulation");
    assert_eq!(state, TitanState::Pursue, "he sees him and stands still");
    println!("F-051 husk: 15 m behind -> Idle, level {:.2}; 15 m in front -> Pursue in 2 ticks", blind.level);
}

/// The two per-kind numbers, in range, on every row. The same guard shape as
/// `tests/combat.rs::every_kind_carries_a_strike_half_angle_in_range`.
///
/// 180 makes the cone a circle again and deletes the feature; below 20 a titan walks past a
/// man standing in front of him.
#[test]
fn f051_every_kind_carries_a_sight_cone_and_an_ear_in_range() {
    let d = data(&app());
    let mut rows = Vec::new();
    for (name, kind) in &d.titans.kinds {
        assert!(
            (20.0..=170.0).contains(&kind.sight_half_angle_deg),
            "{name}: sight_half_angle_deg {} is outside [20, 170] — at 180 the cone is a circle \
             and F-051 is gone",
            kind.sight_half_angle_deg
        );
        assert!(
            (5.0..=300.0).contains(&kind.hearing_radius_m),
            "{name}: hearing_radius_m {} is outside [5, 300]",
            kind.hearing_radius_m
        );
        rows.push(format!(
            "{name} {}/{}",
            kind.sight_half_angle_deg, kind.hearing_radius_m
        ));
    }
    // The bellower is the kind the design hangs the stealth layer off — his ear has to be the
    // widest in the file or `docs/gameplay/enemies.md` is describing somebody else.
    let widest = d
        .titans
        .kinds
        .iter()
        .max_by(|a, b| a.1.hearing_radius_m.total_cmp(&b.1.hearing_radius_m))
        .map(|(name, _)| name.clone())
        .unwrap();
    assert_eq!(widest, "bellower", "the widest ear in titan.ron belongs to {widest}");
    println!("F-051 cones/ears: {}", rows.join(" · "));
}

/// The three pure functions, against numbers computed here and not with the domain's own
/// helper — the same rule [`expected_ticks`] follows.
#[test]
fn f051_the_noise_the_cone_and_the_ear_are_arithmetic() {
    let d = data(&app());
    let feel = d.titans.perception;
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let senses = Senses::of(husk);

    assert_eq!(loudness_m(&feel, 0.0, false), feel.quiet_m);
    let fast = feel.quiet_m + 30.0 * feel.noise_per_speed_m;
    assert!((loudness_m(&feel, 30.0, false) - fast).abs() < 1e-3);
    assert!((loudness_m(&feel, 30.0, true) - fast * feel.rope_factor).abs() < 1e-3);
    assert_eq!(loudness_m(&feel, 10_000.0, true), feel.max_noise_m, "the ceiling is one");

    // The cone, off Bevy's forward (−Z).
    assert!(sees(&senses, Vec3::NEG_Z, Vec3::new(0.0, 0.0, -20.0)));
    assert!(!sees(&senses, Vec3::NEG_Z, Vec3::new(0.0, 0.0, 20.0)), "dead astern");
    assert!(
        !sees(&senses, Vec3::NEG_Z, Vec3::new(0.0, 0.0, -(husk.aggro_radius_m + 1.0))),
        "in the cone, past the range"
    );
    // Height does not blind him: every cone in this game is measured on the ground plane.
    assert!(sees(&senses, Vec3::NEG_Z, Vec3::new(0.0, 60.0, -20.0)));

    // The ear takes the SMALLER of the two radii.
    assert_eq!(hears(&senses, feel.quiet_m, 20.0), 0.0);
    let close = hears(&senses, 300.0, husk.hearing_radius_m / 2.0);
    assert!((close - 0.5).abs() < 1e-3, "a huge noise at half the kind's own ear: {close}");
    println!(
        "F-051 husk: cone {} deg x {} m, ear {} m",
        husk.sight_half_angle_deg, husk.aggro_radius_m, husk.hearing_radius_m
    );
}

// ---------------------------------------------------------------------------
// F-055 — the ring: six titans, six places
// ---------------------------------------------------------------------------

/// The smallest gap between any two living titans, on the ground plane.
fn tightest_pair_m(app: &mut App) -> f32 {
    let roots = titan_roots(app);
    let at: Vec<Vec3> = roots
        .iter()
        .filter_map(|e| app.world().get::<Transform>(*e).map(|t| t.translation))
        .collect();
    let mut tightest = f32::INFINITY;
    for i in 0..at.len() {
        for j in (i + 1)..at.len() {
            let d = at[i] - at[j];
            tightest = tightest.min(Vec3::new(d.x, 0.0, d.z).length());
        }
    }
    tightest
}

/// ★ **`F-055`'s acceptance, with its own control run inside it.**
///
/// *"Bei 6 Titanen auf einen Spieler stehen keine zwei in derselben Position."* Six husks are
/// put on one line 40 m from a player who does not move, and walked for 900 ticks. Then the
/// **same** six are walked again with `crowd.ring_radius_m` set to zero in `GameData` — which
/// is the habit next to rule 5 in one line: *delete the thing you think you are measuring and
/// check the number moves.*
///
/// Without the ring six titans converge on one point and the tightest pair is under a metre.
/// With it they arrive on six bearings and the tightest pair is metres apart. It goes red when
/// the slot stops reaching `brain::aim`, when two titans are handed the same index, and when
/// the fade swallows the offset before anybody has arrived.
#[test]
fn f055_six_titans_on_one_player_stand_in_six_places() {
    fn run(ring_radius_m: Option<f32>) -> (f32, Vec<u32>) {
        let mut app = app_with_pinned_player(Vec3::new(0.0, 2.0, 0.0), 0.0, MovementState::Grounded);
        if let Some(r) = ring_radius_m {
            app.world_mut().resource_mut::<GameData>().titans.crowd.ring_radius_m = r;
        }
        // On a line 40 m out, 2 m apart, on **+Z** — so the spawn facing (Bevy's forward, −Z)
        // already points at the player, exactly as `f050_…` sets its husk up. That is not
        // convenience: since `F-051` a titan acquires nobody he has not perceived, and a
        // motionless player on the ground carries `perception.quiet_m` = 8 m of noise, which
        // no ear in the file reaches at 40 m. Spawned facing away they would stand where they
        // were put for all 900 ticks and the ring would be measured against a field of statues.
        // 40 m is inside the husk's `aggro_radius_m` of 45 and far outside his 6 m reach: they
        // are still walked in, and the spread is the walk's, not the spawn's.
        for i in 0..6 {
            spawn(&mut app, "husk", Vec3::new(i as f32 * 2.0 - 5.0, 0.0, 40.0));
        }
        ticks(&mut app, 900);
        let roots = titan_roots(&mut app);
        let slots: Vec<u32> = roots
            .iter()
            .filter_map(|e| app.world().get::<CrowdSlot>(*e).map(|s| s.index))
            .collect();
        let bearings: Vec<String> = roots
            .iter()
            .filter_map(|e| app.world().get::<Transform>(*e).map(|t| t.translation))
            .map(|p| {
                format!("{:.0}deg@{:.1}m", f32::atan2(p.x, p.z).to_degrees(), Vec3::new(p.x, 0.0, p.z).length())
            })
            .collect();
        let label = match ring_radius_m {
            None => "as shipped".to_string(),
            Some(r) => format!("ring_radius_m forced to {r}"),
        };
        println!("  F-055 {label}: {bearings:?}");
        (tightest_pair_m(&mut app), slots)
    }

    let (with_ring, slots) = run(None);
    let (stacked, _) = run(Some(0.0));

    let mut sorted = slots.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 6, "six titans were handed the slots {slots:?}");

    assert!(
        with_ring > stacked * 2.0 && with_ring > 4.0,
        "the ring bought {with_ring:.2} m between the tightest pair; without it they stand \
         {stacked:.2} m apart. A ring that does not spread them is decoration"
    );
    println!(
        "F-055 six husks after 900 ticks: tightest pair {with_ring:.2} m with the ring, \
         {stacked:.2} m without it, slots {slots:?}"
    );
}

/// ★ **`F-055`'s other half: a lone titan is a place, a crowded one is not.**
///
/// This is the mechanism that took `scripts/game-full.txt` from 24 green asserts to 19 without
/// anybody touching it, and it is the general trap behind every scripted pass in this
/// repository, so it gets a test rather than a comment.
///
/// The fixture is `game-full`'s ACT 3 reduced to its bones: a husk, and a player standing
/// **1.88 m** from him — `|(-1.80, 0.55)|`, the file's own offset from the nape. Nothing else
/// changes between the three runs.
///
/// * **Alone** the husk is planted. `perception`'s `target.distance_m` is *horizontal*
///   (`Vec3::new(to.x, 0.0, to.z)`), so 1.88 m is inside his 6 m `attack_range_m`; `slot.of` is
///   1, `brain::walk`'s `in_position` is therefore true and `pursuing` is false. He may turn on
///   the spot, but the nape stays where it was put — which is the entire reason `q030-reach`
///   and ACTS 2 and 4 of `game-full` work at all.
/// * **With one more husk 38 m away** who has also noticed the player, `claim_slots` hands both
///   of them a ring bearing. `slot.of` is 2, the husk's standing place moves out to
///   `ring_radius_m(9.0, 6.0)` = `min(9.0, 6.0 * 0.9)` = 5.4 m, `at_slot` is false — and
///   `pursuing` is now true **at 1.88 m**. He walks off the coordinate, and a 0.55 m pass aimed
///   at where he was spawned meets empty street.
/// * **The control, and it is the point of the test**: the same crowded run with
///   `crowd.arrive_m` raised so that `at_slot` is true everywhere. The second husk is still
///   there, `slot.of` is still 2 — only the reason to walk is gone, and the displacement
///   collapses. Delete the thing you think you are measuring and check the number moves.
///
/// `detected` is asserted in **all three** runs. Without that this fixture would pass just as
/// well on a husk who never noticed anybody and stood still out of blindness (`FIND-169`), and
/// a green run would mean nothing.
#[test]
fn f055_a_lone_titan_holds_his_ground_and_a_crowded_one_walks_off_it() {
    /// `(how far he drifted, slot.of, detected)`
    fn run(a_second_husk: bool, arrive_m: Option<f32>) -> (f32, u32, bool) {
        // 1.88 m in front of him — `spawn` faces a titan down −Z, so the player is dead ahead
        // and the eye cannot be the thing under test.
        let mut app =
            app_with_pinned_player(Vec3::new(0.0, 2.0, -1.88), 0.0, MovementState::Grounded);
        if let Some(a) = arrive_m {
            app.world_mut().resource_mut::<GameData>().titans.crowd.arrive_m = a;
        }
        let spawned_at = Vec3::ZERO;
        spawn(&mut app, "husk", spawned_at);
        let near = the_titan(&mut app);
        if a_second_husk {
            // Far enough to be no part of the pass and near enough to be part of the crowd:
            // 39.9 m from the player is inside the husk's `aggro_radius_m` of 45, which is the
            // bound `in_the_crowd` uses. On +Z, so his spawn facing already points at the
            // player and he acquires him the same way the near one does.
            spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 38.0));
        }
        // 120 ticks = 2.0 s, a little over the 0.90 s `game-full` leaves between the spawn and
        // the slash — the FSM has to reach `Pursue` before a step is even possible.
        ticks(&mut app, 120);
        let now = app.world().get::<Transform>(near).expect("the husk still exists").translation;
        let d = now - spawned_at;
        let slot = *app.world().get::<CrowdSlot>(near).expect("every titan carries a CrowdSlot");
        (Vec3::new(d.x, 0.0, d.z).length(), slot.of, awareness_of(&mut app, near).detected)
    }

    let (alone, alone_of, alone_saw) = run(false, None);
    let (crowded, crowded_of, crowded_saw) = run(true, None);
    let (control, control_of, control_saw) = run(true, Some(1000.0));

    assert!(
        alone_saw && crowded_saw && control_saw,
        "the husk has to have NOTICED the player in all three runs, or this measures blindness \
         and not the ring — alone {alone_saw}, crowded {crowded_saw}, control {control_saw}"
    );
    assert_eq!(alone_of, 1, "one husk on one player is a crowd of one");
    assert_eq!(crowded_of, 2, "two husks on one player are a crowd of two");
    assert_eq!(control_of, 2, "the control keeps both husks — only `arrive_m` moved");

    assert!(
        alone < 0.25,
        "a husk alone with a player inside his reach must hold his ground; he drifted {alone:.2} m"
    );
    // The bar is not a taste: `cortex_radius_m` is how wide the nape is, so a drift bigger than
    // it is a pass aimed at the spawn point that **cannot** reach the kill zone any more. It
    // comes out of `titan.ron`, never out of this file.
    let nape_m = data(&app_with_pinned_player(Vec3::ZERO, 0.0, MovementState::Grounded))
        .titans
        .kinds["husk"]
        .cortex_radius_m;
    assert!(
        crowded > nape_m && crowded > alone + nape_m,
        "a husk of a crowd walks to his slot even from inside his reach, and far enough that a \
         pass aimed at where he was spawned misses the nape: he moved {crowded:.2} m against the \
         lone husk's {alone:.2} m, and the nape is {nape_m:.2} m wide"
    );
    assert!(
        control < 0.25,
        "with `arrive_m` past his own distance the crowded husk is already in his slot and has \
         nothing to walk to; he moved {control:.2} m"
    );
    println!(
        "F-055 drift off the spawn point after 120 ticks: alone {alone:.2} m (of {alone_of}), \
         crowded {crowded:.2} m (of {crowded_of}), crowded with arrive_m 1000 {control:.2} m \
         (of {control_of}) — the nape is {nape_m:.2} m wide"
    );
}

// ---------------------------------------------------------------------------
// F-054 — the level of detail
// ---------------------------------------------------------------------------

/// ★ **`F-054`: a far titan thinks less often, and the wind-up still lasts as long.**
///
/// Two halves, and the second is the one that matters. The row asks for *"near titans tick at
/// 20 Hz, distant ones at 5 Hz"* and the cheap way to get that number is to skip ticks and let
/// the state clock skip with them — which silently shortens every `windup_s` in the game. The
/// accumulators are stepped by [`Lod::steps`] instead, so the grid gets coarser and the
/// **duration does not move**.
///
/// The control is the same shape as `F-055`'s: `lod.near_m` is pushed past the far titan and
/// the count goes straight back to one run per tick.
#[test]
fn f054_a_far_titan_thinks_less_often_and_winds_up_for_just_as_long() {
    let d = data(&app());
    let hz = d.game.simulation_hz;
    let table = d.titans.lod;
    let near = period_ticks(&table, 10.0, hz);
    let mid = period_ticks(&table, (table.near_m + table.mid_m) / 2.0, hz);
    let far = period_ticks(&table, table.mid_m + 100.0, hz);
    assert_eq!(near, 1, "the near tier is full rate — see the type's doc");
    assert!(far > mid && mid > near, "the tiers do not separate: {near} {mid} {far}");

    // ---- 1. the count -------------------------------------------------------------------
    fn thinking_ticks(at: Vec3, over: u64, near_m: Option<f32>) -> u64 {
        let mut app = app_with_pinned_player(Vec3::new(0.0, 2.0, 0.0), 0.0, MovementState::Grounded);
        if let Some(m) = near_m {
            app.world_mut().resource_mut::<GameData>().titans.lod.near_m = m;
        }
        spawn(&mut app, "husk", at);
        let root = the_titan(&mut app);
        let mut runs = 0;
        for _ in 0..over {
            app.update();
            if app.world().get::<Lod>(root).is_some_and(|l| l.due) {
                runs += 1;
            }
        }
        runs
    }

    let close = thinking_ticks(Vec3::new(0.0, 0.0, -20.0), 240, None);
    let distant = thinking_ticks(Vec3::new(0.0, 0.0, -400.0), 240, None);
    let control = thinking_ticks(Vec3::new(0.0, 0.0, -400.0), 240, Some(10_000.0));
    assert_eq!(close, 240, "a titan 20 m away has to think on every one of 240 ticks");
    assert!(
        distant < 240 / (far as u64) + 2 && distant > 240 / (far as u64) - 2,
        "a titan at 400 m thought {distant} times in 240 ticks; the far tier is 1 in {far}"
    );
    assert_eq!(
        control, 240,
        "with `near_m` pushed past him the same titan at the same 400 m must think every tick \
         — this is the control, and a number that does not move means the tier was never read"
    );

    // ---- 2. the duration, which is what the skipping is allowed to cost --------------
    // `near_m` is pulled in to 5 m so that a husk 20 m away — well inside his own 45 m cone —
    // sits in the MID tier while he attacks. Without this the only titans on a coarse grid are
    // ones too far away to have a wind-up at all, and the half of `F-054` that can be wrong
    // would never be exercised.
    // **+Z, so that "inside his own cone" is true and not just written.** The counting half
    // above is indifferent to facing — `Lod` is chosen from the distance alone — but this half
    // needs him to actually attack, and since `F-051` that needs him to have noticed. A husk
    // spawned on −Z has the player dead astern of his 110° cone and cannot hear 8 m of noise
    // at 20 m, so he would stand still for all 1400 ticks.
    let mut app = app_with_pinned_player(Vec3::new(0.0, 2.0, 0.0), 0.0, MovementState::Grounded);
    app.world_mut().resource_mut::<GameData>().titans.lod.near_m = 5.0;
    spawn(&mut app, "husk", Vec3::new(0.0, 0.0, 20.0));
    let root = the_titan(&mut app);
    let mut windup_ticks = 0;
    let mut seen_windup = false;
    for _ in 0..1400 {
        app.update();
        let state = *app.world().get::<TitanState>(root).expect("a titan has a state");
        if state == TitanState::Windup {
            seen_windup = true;
            windup_ticks += 1;
        } else if seen_windup {
            break;
        }
    }
    let wanted = expected_ticks(d.titan("husk").unwrap().windup_s, hz);
    assert!(seen_windup, "the husk never wound up in 1400 ticks");
    assert!(
        (windup_ticks as i64 - wanted as i64).abs() <= mid as i64,
        "a husk in the mid tier wound up for {windup_ticks} ticks; `windup_s` is {wanted} ticks \
         and the tier's own grid is {mid}. A wind-up that shortens with distance is the bug \
         this half exists for"
    );
    println!(
        "F-054 240 ticks: 20 m -> {close} brain runs, 400 m -> {distant} (1 in {far}), control \
         {control}. Mid-tier wind-up {windup_ticks} ticks against {wanted}"
    );
}
