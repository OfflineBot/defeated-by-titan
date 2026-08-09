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
use defeated_by_titan::titan::brain::TitanClock;
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
fn fly_past_a_titan(kind: &str, dir: Vec3, air_m: f32, speed_m_s: f32, widen: Option<f32>) -> Pass {
    let mut app = app_with_hits();
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

    // Hold the real slash key through the real `Intent` channel, and start the pass on the
    // blade's first cutting tick so the whole active window lies in front of it.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
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
    let cortex = Vec3::new(0.0, LANE_Y + rig.cortex_height_m, rig.head_m * 0.5);
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

    let p = fly_past_a_titan("husk", Vec3::NEG_Z, AIR_M, 30.0, None);
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
/// lengths it leaves out (`cortex_radius_m` 0.60 and `thickness_m` 0.12) are worth 0.72 m.
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
    let p = fly_past_a_titan("warden", Vec3::NEG_Z, AIR_M, 30.0, None);
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
        if fly_past_a_titan(&tightest, Vec3::NEG_Z, air_m, 30.0, None).cortex_tick.is_some() {
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
        let behind = fly_past_a_titan(kind, Vec3::NEG_Z, AIR_M, 30.0, None);
        // Flying along +X the player is in **front** of the titan and the blade swings towards
        // his back, through the whole depth of the body.
        let front = fly_past_a_titan(kind, Vec3::X, AIR_M, 30.0, None);
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
/// the husk's nape stays reachable up to about **0.33**, and 0.25 is 32 % below that.
#[test]
fn q030_a_titan_wide_enough_really_does_put_the_nape_out_of_reach() {
    let mut reachable = Vec::new();
    for fraction in [0.25f32, 0.29, 0.33, 0.37, 0.45] {
        let p = fly_past_a_titan("husk", Vec3::NEG_Z, AIR_M, 30.0, Some(fraction));
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
    let entry_clock = app.world().get::<TitanClock>(root).expect("clock").ticks_in_state;
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
#[test]
fn f034_a_grazed_titan_holds_still_for_the_impact_frame() {
    let mut app = app_with_hits();
    let d = data(&app);
    let husk = d.titan("husk").expect("husk");
    let hz = d.game.simulation_hz;
    let stop_ticks = expected_ticks(d.gear.feel.hit_stop_normal_s, hz) as u64;
    assert!(stop_ticks > 0, "gear.ron: feel.hit_stop_normal_s rounds to zero ticks");

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
    ticks(&mut app, 4);
    let moving_again = app.world().get::<Transform>(root).expect("transform").translation;
    assert!(
        (moving_again - frozen_after).length() > walked_m * 0.5,
        "the husk did not pick his walk back up after the hit stop"
    );
    println!(
        "F-034 titan side: {walked_m:.4} m in {stop_ticks} free ticks, {crept_m:.4} m in \
         {stop_ticks} frozen ticks (gear.ron feel.hit_stop_normal_s = {} s)",
        d.gear.feel.hit_stop_normal_s
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
