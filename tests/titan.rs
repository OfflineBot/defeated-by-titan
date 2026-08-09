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
use defeated_by_titan::shared::{Cli, HitZone, PlayerId, SpawnTitan, TitanHit, TitanId, TitanState};
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

    assert!(spawned > 0 && refused > 0, "{spawned} spawned, {refused} refused — a cap that \
         refuses nothing or allows nothing tests nothing");
    // The bellower is `huge` and therefore unspawnable this session. That is intended
    // (docs/QUESTIONS.md Q-028) and it is the one row this test pins down by name.
    assert!(
        matches!(spawnable(&d, "bellower"), Err(SpawnRefused::AboveClassCap { .. })),
        "the bellower is `huge` and must not spawn while the cap is `large`"
    );
    assert!(
        matches!(spawnable(&d, "no_such_titan"), Err(SpawnRefused::UnknownKind { .. })),
        "an unknown kind must be refused by name, not silently ignored"
    );
}
