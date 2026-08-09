//! The guard over the cut — `F-030`, `F-034`, `P5`.
//!
//! **A titan dies only from a fast cut into the cortex.** Five ways of getting that wrong are
//! invisible in a screenshot, and each of them has a test here:
//!
//! 1. **The cut samples positions instead of sweeping.** A blade collider, a `Sensor` plus
//!    `CollisionStart`, an AABB overlap — all three ask "do these overlap *now*", once per
//!    tick. They pass at 8 m/s, they pass the husk at 30 m/s, and they are arithmetically
//!    incapable of passing [`f030_a_pass_at_75_m_s_still_hits_the_weaver`], which measures the
//!    gap at every tick boundary and shows that a sampling test had **nothing to find**.
//! 2. **One unfiltered cast.** It returns the torso every time and never the cortex, because
//!    the cortex sits *inside* the body silhouette and `cast_shape` returns only the closest
//!    hit ([`f030_the_cortex_wins_over_the_body_it_hides_in`] and
//!    [`f030_the_torso_does_not_count_as_the_cortex`], which are the two halves of one claim).
//! 3. **The one-line `Time<Virtual>` hit stop.** It looks perfect on screen and stops the tick
//!    with the bodies ([`f034_the_hit_stop_freezes_the_bodies_and_not_the_clock`], whose
//!    **first** assertion is the one that falls).
//! 4. **A despawned player.** `Downed` is a state with a timer, not a removed entity
//!    ([`p5_a_downed_player_is_a_state_and_not_a_removed_entity`]).
//! 5. **The collider tree's AABBs are one tick old in `PostStep`** and the cut misses moving
//!    targets. That is `docs/PLAN-GAME.md` §11 risk 2 and it is the **first** test in this
//!    file, not the last ([`risk2_a_titan_crossing_at_speed_is_not_missed`]).
//!
//! ## Why most of these run against a fixture and not against the real husk
//!
//! Because the criterion is about **arithmetic**, and the real rig brings an FSM, a pose and a
//! walk with it — three things that move the target while the test is trying to measure a
//! 0.46 m sphere at 1.25 m per tick. The fixture is the same three components the real cortex
//! carries (`Collider::sphere` + `Sensor` + `CollisionLayers::new(LAYER_TITAN_CORTEX, NONE)`,
//! `src/titan/rig.rs:649-666`) inside the same body capsule on `LAYER_TITAN_BODY`, and
//! [`f030_the_cut_kills_the_real_husk`] is the one that meets the real thing.
//!
//! ## Why the passes are flown at y = 60
//!
//! Above every structure in the city: the tallest landmark is the church at 35 m
//! (`scale.ron: architecture.heights_m`). A pass at head height would sweep the blade through
//! a house and measure the house.

use std::time::Instant;

use avian3d::prelude::{
    Collider, CollisionLayers, GravityScale, LayerMask, LinearVelocity, RigidBody,
    RigidBodyDisabled, Sensor, SpatialQuery,
};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::blades::cut::{blade_segment, sweep};
use defeated_by_titan::blades::swing::{BladeTiming, SweptFrom, Swings};
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{
    Cli, Health, HitStop, HitZone, LocalPlayer, MovementState, PlayerId, Side, SpawnTitan, Tick,
    TitanHit, TitanId, Velocity, LAYER_PLAYER, LAYER_TITAN_BODY, LAYER_TITAN_CORTEX,
};
use defeated_by_titan::titan::rig::TitanPart;

// ---------------------------------------------------------------------------
// the harness
// ---------------------------------------------------------------------------

/// Every [`TitanHit`] that was written, with the tick it was written on.
#[derive(Resource, Default)]
struct HitLog(Vec<(u64, TitanHit)>);

fn record_hits(mut log: ResMut<HitLog>, tick: Res<Tick>, mut hits: MessageReader<TitanHit>) {
    for hit in hits.read() {
        log.0.push((tick.0, *hit));
    }
}

/// The **real** app, headless, one simulation step per `update()`.
///
/// Same reasoning as `tests/titan.rs`: avian takes its step size from the generic `Time`, and
/// only `run_fixed_main_schedule` switches that over to `Time<Fixed>`. With
/// `FixedTimesteps(1)` one `update()` is exactly one simulation step, on every machine.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<HitLog>();
    // `Last`, so that a hit written in `PostStep` is logged with the tick it happened on.
    app.add_systems(Last, record_hits);
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

fn player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("the app spawns exactly one local player")
}

fn position(app: &mut App) -> Vec3 {
    let p = player(app);
    app.world().get::<Transform>(p).expect("the player has a Transform").translation
}

fn hits(app: &App) -> Vec<(u64, TitanHit)> {
    app.world().resource::<HitLog>().0.clone()
}

/// Holds the **right** slash. `KeyE` is `SLASH_RIGHT` (`src/net/local.rs`), and the right
/// blade lies on `+X` for a player looking at −Z — the axis every pass in this file is built
/// on. The real key, through the real `Intent` channel: no second, wrong way to play.
fn hold_slash(app: &mut App) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
}

fn swing_tick(app: &mut App) -> Option<u32> {
    let p = player(app);
    app.world().get::<Swings>(p).and_then(|s| s.right.ticks_in_swing)
}

fn timing(app: &mut App) -> BladeTiming {
    let p = player(app);
    *app.world().get::<BladeTiming>(p).expect("the player is equipped with blades")
}

/// Runs until the right blade is on its **first** cutting tick, so that a pass placed
/// afterwards has the whole active window in front of it.
fn until_the_blade_bites(app: &mut App) {
    hold_slash(app);
    let from = timing(app).active_from_tick;
    for _ in 0..300 {
        app.update();
        if swing_tick(app) == Some(from) {
            return;
        }
    }
    panic!("the blade never reached its active window — the swing state machine never started");
}

/// Puts the player at `at_m` with `velocity_m_s` and **no gravity**.
///
/// Gravity off, because the criterion is a horizontal pass at an exact speed and −20 m/s² adds
/// 0.33 m/s per tick to it. The teleport also resets [`SweptFrom`]: without that the jump
/// itself is swept, and a 200 m line from the old position can cut something on the way.
fn place(app: &mut App, at_m: Vec3, velocity_m_s: Vec3) {
    let p = player(app);
    let world = app.world_mut();
    world.entity_mut(p).insert((
        Transform::from_translation(at_m),
        GravityScale(0.0),
        LinearVelocity(velocity_m_s),
    ));
    if let Some(mut from) = world.get_mut::<SweptFrom>(p) {
        from.0 = at_m;
    }
}

// ---------------------------------------------------------------------------
// the fixture — the same three components the real cortex carries
// ---------------------------------------------------------------------------

/// The lane every pass is flown in. Above the church (35 m), the tallest thing in the city.
const LANE_Y: f32 = 60.0;
/// How far to the right of the flight line the target sits. Inside `gear.ron: reach_m` (1.6 m)
/// and not at its very tip, so the test measures the cut and not the last centimetre of reach.
const REACH_X: f32 = 0.8;

/// A target: a body capsule on `LAYER_TITAN_BODY` with a cortex sphere **inside** it on
/// `LAYER_TITAN_CORTEX`, exactly the way `titan::rig::build_rig` assembles the real one.
///
/// The capsule's radius is deliberately larger than the cortex's offset from its axis, so the
/// cortex really is hidden inside the silhouette — that is the whole point of the two casts.
fn spawn_target(app: &mut App, id: u32, cortex_m: Vec3, cortex_r: f32, kinematic: bool) -> Entity {
    let world = app.world_mut();
    let cortex = world
        .spawn((
            Name::new("fixture_cortex"),
            Transform::default(),
            Collider::sphere(cortex_r),
            Sensor,
            CollisionLayers::new(LAYER_TITAN_CORTEX, LayerMask::NONE),
        ))
        .id();
    let root = world
        .spawn((
            Name::new("fixture_titan"),
            TitanId(id),
            Velocity::default(),
            Transform::from_translation(cortex_m),
            if kinematic { RigidBody::Kinematic } else { RigidBody::Static },
            // A body 3 m tall and 2.5 m wide around the cortex — a husk's proportions, and
            // wide enough that every line reaching the cortex crosses the body first. That is
            // the point: it is what makes `f030_the_cortex_wins_over_the_body_it_hides_in`
            // able to catch a single unfiltered cast.
            Collider::capsule_endpoints(1.25, Vec3::new(0.0, -1.6, 0.0), Vec3::new(0.0, 1.4, 0.0)),
            // **Member of the body layer, colliding with nothing.** Queryable — `filter.test`
            // asks memberships only (`avian3d-0.7.0/src/spatial_query/query_filter.rs:97-101`)
            // — but physically intangible, so these tests measure the CAST and not `F-013`'s
            // impact. Measured, not assumed: with a solid body the player at 30 m/s bounces off
            // the fixture at 1.7 m from the cortex (`lv` turned to (-28.4, 0, -13.0)) and no
            // pass ever reaches the target. The real husk IS solid and 2.5 m wide while
            // `gear.ron: reach_m` is 1.60 m — that is a finding about the numbers, not about
            // the cut, and it is reported as one.
            CollisionLayers::new(LAYER_TITAN_BODY, LayerMask::NONE),
        ))
        .id();
    world.entity_mut(root).add_child(cortex);
    ticks(app, 1); // one step, so avian has the colliders in its trees
    root
}

/// Distance from the cortex centre to the blade segment, **minus** the two radii.
///
/// Positive means: at this instant the blade and the cortex do not touch — an overlap test
/// asked at this tick boundary would have found nothing.
fn gap_m(player_m: Vec3, cortex_m: Vec3, cortex_r: f32, data: &GameData) -> f32 {
    let (a, b) = blade_segment(
        player_m,
        Vec3::NEG_Z,
        Side::Right,
        data.game.player.eye_height_m,
        data.gear.blades.reach_m,
    );
    let ab = b - a;
    let t = ((cortex_m - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    (cortex_m - (a + ab * t)).length() - cortex_r - data.gear.blades.thickness_m
}

/// One pass: place the player `lead` ticks before the target and fly `count` ticks past it.
///
/// Returns the gap at every tick boundary, in order. The **hits** land in [`HitLog`].
fn fly_past(
    app: &mut App,
    cortex_m: Vec3,
    cortex_r: f32,
    speed_m_s: f32,
    hand_y_m: f32,
    lead: u32,
    count: u64,
) -> Vec<f32> {
    let d = data(app);
    let step = speed_m_s / d.game.simulation_hz as f32;
    until_the_blade_bites(app);
    // Half a step past the target plus the lead, so that at 75 m/s the two tick boundaries
    // around the crossing land at +0.5 m and −0.75 m — both outside the 0.35 m an overlap
    // test could see. The pass is aimed, not hoped for.
    let start = Vec3::new(
        cortex_m.x - REACH_X,
        hand_y_m - d.game.player.eye_height_m,
        cortex_m.z + 0.5 + step * lead as f32,
    );
    place(app, start, Vec3::new(0.0, 0.0, -speed_m_s));

    let mut gaps = vec![gap_m(start, cortex_m, cortex_r, &d)];
    for _ in 0..count {
        app.update();
        gaps.push(gap_m(position(app), cortex_m, cortex_r, &d));
    }
    gaps
}

/// Places the pass and then flies until the **cortex** is cut, or gives up. Returns the tick.
///
/// A budget and not a fixed count, because the pass is interrupted: the blade meets the body
/// one or more ticks before the nape (every titan is wider than his own neck), and that graze
/// costs `hit_stop_normal_s` of freeze before the flight continues.
fn fly_until_cut(
    app: &mut App,
    cortex_m: Vec3,
    cortex_r: f32,
    speed_m_s: f32,
    hand_y_m: f32,
    budget: u64,
) -> Option<u64> {
    fly_past(app, cortex_m, cortex_r, speed_m_s, hand_y_m, 2, 0);
    for _ in 0..budget {
        app.update();
        if let Some((tick, _)) = hits(app).into_iter().find(|(_, h)| h.zone == HitZone::Cortex) {
            return Some(tick);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Risk 2 — FIRST, not last
// ---------------------------------------------------------------------------

/// ★ **`docs/PLAN-GAME.md` §11, risk 2 — measured, not argued.**
///
/// The cut runs in `PostStep`. The collider tree's AABBs are refreshed in
/// `ColliderTreeSystems::UpdateAabbs`, configured `.in_set(PhysicsStepSystems::BroadPhase)`
/// (`avian3d-0.7.0/src/collider_tree/mod.rs:78-84`) — at the **start** of the step. So in
/// `PostStep` the tree's AABBs are one tick old while `cast_shape`'s narrow phase reads the
/// current `Position`. avian enlarges them by `AABB_MARGIN` (0.05 m,
/// `collider_tree/update.rs:34`) plus a velocity term
/// (`update.rs:756-780`), and *almost certainly* is not a measurement.
///
/// So: the same crossing twice, once against a standing target and once against one crossing
/// at **11 m/s** — the scuttler's speed, the fastest titan in `titan.ron`. If the static case
/// is green and the moving one is red, the enlargement is insufficient and Round 4 changes.
#[test]
fn risk2_a_titan_crossing_at_speed_is_not_missed() {
    /// `None` = the target stands still. Otherwise the velocity it is given, in m/s.
    fn one(target_m_s: Vec3) -> bool {
        let mut app = app();
        let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
        let moving = target_m_s != Vec3::ZERO;
        let root = spawn_target(&mut app, 1, cortex, 0.55, moving);

        // Place the pass FIRST and start the target moving only then. The other way round the
        // target has already travelled the length of `until_the_blade_bites` — measured, it
        // was 1.28 m out of a 1.60 m blade before the pass even began, and the test would have
        // reported "risk 2 is real" about a target that was simply out of reach.
        // 30 m/s is 0.5 m per tick against a 1.10 m cortex: the crossing sits between two
        // samples, which is the whole point of the sweep.
        fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 0);
        if moving {
            // `RigidBody::Kinematic` **without** `CustomPositionIntegration`, so avian moves
            // it: what matters here is that the broad phase enlarges the tree AABB from
            // `LinearVelocity` at the START of the step while the cast in `PostStep` reads the
            // position at the END of it.
            let world = app.world_mut();
            world.entity_mut(root).insert(LinearVelocity(target_m_s));
            if let Some(mut v) = world.get_mut::<Velocity>(root) {
                v.0 = target_m_s;
            }
        }
        for _ in 0..12 {
            app.update();
            if hits(&app).iter().any(|(_, h)| h.zone == HitZone::Cortex) {
                return true;
            }
        }
        false
    }

    let standing = one(Vec3::ZERO);
    // Head-on, at the scuttler's 11 m/s — the fastest titan in `titan.ron`. Head-on and not
    // sideways on purpose: the staleness of the AABB is a displacement of `v · dt` in whatever
    // direction the body moves, and along the axis of the sweep it is worst — 41 m/s of
    // closing speed instead of 30.
    let head_on = one(Vec3::new(0.0, 0.0, 11.0));
    // And across the blade, which is the case `docs/PLAN-GAME.md` §11 writes down.
    let across = one(Vec3::new(11.0, 0.0, 0.0));

    assert!(standing, "the cut misses a STANDING target — nothing else in this file matters");
    assert!(
        head_on && across,
        "RISK 2 IS REAL — standing {standing}, head-on at 11 m/s {head_on}, across at 11 m/s \
         {across}. The collider tree's AABBs are refreshed at the START of the step \
         (avian3d-0.7.0/src/collider_tree/mod.rs:78-84) and the enlargement by AABB_MARGIN \
         (0.05 m) plus the swept term does NOT cover a titan at scuttler speed. Every criterion \
         in Rounds 1 and 2 was signed off against a mechanism that does not work on moving \
         targets, and every one has to be re-run — see docs/PLAN-GAME.md §11 risk 2, \
         'cost if found late'."
    );
    println!(
        "Risk 2 MEASURED: standing {standing} · head-on at 11 m/s {head_on} · across at 11 m/s \
         {across}. The AABB enlargement holds: `default_speculative_margin` is `Scalar::MAX` \
         (avian3d-0.7.0/src/collision/narrow_phase/mod.rs:252), so `update_aabb` sweeps the \
         full `vel * dt` (update.rs:756-772) instead of clamping it, and adds AABB_MARGIN on \
         top."
    );
}

// ---------------------------------------------------------------------------
// F-030 — the cut
// ---------------------------------------------------------------------------

/// The husk pass, at the speed the whole game is designed for.
#[test]
fn f030_a_pass_at_30_m_s_hits_the_cortex() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 6);

    let landed: Vec<_> = hits(&app).into_iter().filter(|(_, h)| h.zone == HitZone::Cortex).collect();
    assert_eq!(landed.len(), 1, "a pass at 30 m/s produced {} cortex hits", landed.len());
    let hit = landed[0].1;
    assert_eq!(hit.titan, TitanId(1));
    assert!(
        (hit.speed_m_s - 30.0).abs() < 0.01,
        "the message carries {} m/s of closing speed, the pass was flown at 30",
        hit.speed_m_s
    );
}

/// ★ **The one that catches the sensor-overlap implementation by name.**
///
/// A weaver's cortex is `2 × 0.23 = 0.46 m` across (`titan.ron:75`) and 75 m/s is
/// `75 / 60 = 1.250 m` per tick: the player is inside the target for **0.37 of a tick**. The
/// assertion is therefore two-sided, and the second half is the teeth:
///
/// 1. the cut lands, **and**
/// 2. at **every tick boundary** of the pass the blade and the cortex are apart.
///
/// A blade collider, a `Sensor` + `CollisionStart` or an AABB overlap all sample positions
/// once per tick — and the second assertion says, in metres, that there was nothing at any
/// sample to find. They cannot pass this test; they pass at 8 m/s and they pass the husk.
#[test]
fn f030_a_pass_at_75_m_s_still_hits_the_weaver() {
    let mut app = app();
    let d = data(&app);
    let weaver = d.titan("weaver").expect("titan.ron has a weaver");
    assert!(
        (weaver.cortex_radius_m - 0.23).abs() < 1e-6,
        "titan.ron weaver.cortex_radius_m = {} — the criterion is written against 0.46 m of \
         diameter",
        weaver.cortex_radius_m
    );
    let r = weaver.cortex_radius_m;
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, r, false);

    let gaps = fly_past(&mut app, cortex, r, 75.0, LANE_Y + 1.6, 2, 5);

    let landed: Vec<_> = hits(&app).into_iter().filter(|(_, h)| h.zone == HitZone::Cortex).collect();
    assert_eq!(
        landed.len(),
        1,
        "a pass at 75 m/s produced {} cortex hits — gaps per tick: {:?}",
        landed.len(),
        gaps
    );

    let closest = gaps.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        closest > 0.0,
        "the blade and the cortex overlapped at a tick boundary ({closest:.3} m) — this pass \
         no longer proves that a position sample would have missed. Re-aim it."
    );
    // 1.250 m per tick, 0.46 m of cortex: 0.37 ticks inside the target.
    let inside_ticks = (2.0 * r) / (75.0 / d.game.simulation_hz as f32);
    println!(
        "F-030 weaver at 75 m/s: hit, closest approach at a tick boundary {closest:.3} m, \
         {inside_ticks:.2} ticks inside the target · gaps {gaps:?}"
    );
}

/// **3 of 3.** The number the criterion asks for, in one place.
#[test]
fn f030_three_of_three_passes_land() {
    let mut d_report = Vec::new();
    for speed in [8.0f32, 30.0, 75.0] {
        let mut app = app();
        let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
        spawn_target(&mut app, 1, cortex, 0.55, false);
        // 8 m/s is `gear.ron: blades.min_speed_m_s` exactly — the slowest cut that is a cut.
        let gaps = fly_past(&mut app, cortex, 0.55, speed, LANE_Y + 1.6, 2, 8);
        let landed: Vec<_> =
            hits(&app).into_iter().filter(|(_, h)| h.zone == HitZone::Cortex).collect();
        assert_eq!(
            landed.len(),
            1,
            "{speed} m/s: {} cortex hits, gaps {gaps:?}",
            landed.len()
        );
        d_report.push(format!("{speed} m/s -> {:.2} m/s closing", landed[0].1.speed_m_s));
    }
    println!("F-030 hits at 8 / 30 / 75 m/s: 3 of 3 — {}", d_report.join(" · "));
}

/// ★ **The one that catches the single unfiltered cast — first half.**
///
/// A pass that crosses the body **and** reaches the cortex must report `Cortex`. One cast
/// without a filter returns the closest hit, which is always the body capsule the cortex is
/// hidden inside; it would report `Torso` here and the game would have no kill in it.
#[test]
fn f030_the_cortex_wins_over_the_body_it_hides_in() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 6);

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(
        zones.contains(&HitZone::Cortex),
        "the pass reached the cortex and reported {zones:?} — one unfiltered cast returns the \
         torso the cortex is hidden inside, and the titan never dies"
    );
    assert_eq!(
        zones.last(),
        Some(&HitZone::Cortex),
        "the cortex was not the LAST word: {zones:?}. A blade that grazes the body and then \
         finds the nape has found the nape."
    );
    println!("F-030 cortex inside the body silhouette: zones in order {zones:?}");
}

/// ★ **The one that catches the single unfiltered cast — second half.**
///
/// A pass 2 m below the cortex crosses the body and nothing else. It must report a **non-cortex
/// zone** — not `Cortex` (a cortex-only cast that reports every body hit as a cortex hit), and
/// not nothing at all (a cast that only ever asks the cortex layer and calls a body pass a
/// miss).
#[test]
fn f030_the_torso_does_not_count_as_the_cortex() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    // The hand 2 m below the cortex: deep inside the body capsule (it spans −1.6..1.4 local
    // with a radius of 1.25), and 2 m from a sphere of radius 0.55 that the blade's own 0.12 m
    // cannot bridge.
    let gaps = fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y - 0.4, 2, 6);
    assert!(
        gaps.iter().cloned().fold(f32::INFINITY, f32::min) > 0.5,
        "the pass came within reach of the cortex after all — it no longer tests the body: {gaps:?}"
    );

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert_eq!(zones.len(), 1, "a pass through the body produced {} hits", zones.len());
    assert_ne!(
        zones[0],
        HitZone::Cortex,
        "a pass 2 m below the cortex was reported as a CORTEX hit — that is a free kill on \
         every body contact"
    );
    println!("F-030 body pass: zone {:?}, and the cortex was {:.2} m away",
        zones[0],
        gaps.iter().cloned().fold(f32::INFINITY, f32::min));
}

/// The active window is not decoration: outside `active_from_s..active_to_s` the blade does
/// not cut, and a swing that has landed does not land seven more times.
#[test]
fn f030_the_blade_does_not_cut_outside_its_active_window() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    let t = timing(&mut app);
    assert!(t.active_ticks() > 0 && t.active_ticks() < t.swing_ticks);

    // Park the blade **inside** the cortex and hold the button for three whole swings. If the
    // window were ignored, every one of those ticks would be a hit.
    until_the_blade_bites(&mut app);
    let hand = cortex - Vec3::new(REACH_X, 0.0, 0.0);
    let feet = hand - Vec3::Y * data(&app).game.player.eye_height_m;
    // A crawl, but above `min_speed_m_s`, so the speed gate is not what is being measured.
    place(&mut app, feet, Vec3::new(0.0, 0.0, -8.0));
    let swings_worth = (t.swing_ticks + t.cooldown_ticks) as u64 * 3;
    ticks(&mut app, swings_worth);

    let landed = hits(&app).len();
    assert!(
        landed <= 3,
        "{landed} hits in {swings_worth} ticks with the blade parked inside the cortex — the \
         active window ({}..{} of {}) or the one-hit-per-swing rule is not being honoured",
        t.active_from_tick,
        t.active_to_tick,
        t.swing_ticks
    );
    assert!(landed >= 1, "the parked blade never cut at all");
    println!(
        "F-030 window: {landed} hits in {swings_worth} ticks ({} swings), window {}..{} of {}",
        3, t.active_from_tick, t.active_to_tick, t.swing_ticks
    );
}

/// **The real thing.** The real rig, the real cortex on the real nape, the real `TitanHit`
/// consumer: a cut kills the husk.
#[test]
fn f030_the_cut_kills_the_real_husk() {
    let mut app = app();
    let d = data(&app);
    // Far away, so he stands still while the blade is brought up to its active window.
    app.world_mut().write_message(SpawnTitan {
        kind: "husk".into(),
        pos_x: 0.0,
        pos_y: 0.0,
        pos_z: -120.0,
    });
    ticks(&mut app, 2);

    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<TitanId>>();
        q.iter(app.world()).next().expect("a husk was spawned")
    };
    let cortex_entity = rig_part(&app, root, TitanPart::Cortex).expect("the husk has a cortex");
    let cortex = app
        .world()
        .get::<GlobalTransform>(cortex_entity)
        .expect("the cortex has a GlobalTransform")
        .translation();
    assert!(
        (cortex.y - 8.9).abs() < 0.05,
        "the husk's cortex sits at {} m, scale.ron says 8.9",
        cortex.y
    );

    // ⚠️ The player is put on his own collision layer for this pass. The husk's body capsule
    // is solid and 2.5 m wide and `gear.ron: reach_m` is 1.60 m — measured, the player at
    // 30 m/s slams into him and is thrown sideways before the blade is anywhere near the nape.
    // That is a **finding about the numbers** (`reach_m` vs. `scale.ron: width_fraction`), and
    // this test is about the cut. Nothing in the repo wears `LAYER_WORLD` today
    // (`src/shared/layers.rs:28-32`), so this makes the player pass through everything.
    let me = player(&mut app);
    app.world_mut()
        .entity_mut(me)
        .insert(CollisionLayers::new(LAYER_PLAYER, LayerMask::NONE));
    let gaps = fly_past(&mut app, cortex, d.titan("husk").expect("husk").cortex_radius_m, 30.0, cortex.y, 2, 10);
    let landed: Vec<_> = hits(&app).into_iter().filter(|(_, h)| h.zone == HitZone::Cortex).collect();
    assert_eq!(landed.len(), 1, "the real husk was not cut — gaps {gaps:?}");

    // And the other domain does its half: `titan::brain::receive_hits` kills by rule.
    ticks(&mut app, 2);
    assert_eq!(
        app.world().get::<defeated_by_titan::shared::TitanState>(root),
        Some(&defeated_by_titan::shared::TitanState::Death),
        "the husk took a cortex cut and is not dying"
    );
    println!("F-030 real husk: cortex at {:.3} m, cut at 30 m/s, state = Death", cortex.y);
}

/// The cost of the mechanism the whole game runs on, against the project's own recorded
/// 0.21 µs for a 112 m ray over 4000 cuboids (`src/world/index.rs:24-25`).
#[test]
fn f030_the_cost_of_one_thousand_casts() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    ticks(&mut app, 2);

    let d = data(&app);
    let (a, b) = blade_segment(
        Vec3::new(0.0, LANE_Y, 0.0),
        Vec3::NEG_Z,
        Side::Right,
        d.game.player.eye_height_m,
        d.gear.blades.reach_m,
    );
    let thickness = d.gear.blades.thickness_m;
    let me = player(&mut app);

    #[derive(Resource, Default)]
    struct Cost(f64, u32);
    app.insert_resource(Cost::default());

    app.world_mut()
        .run_system_once(move |space: SpatialQuery, mut out: ResMut<Cost>| {
            let mut found = 0u32;
            let started = Instant::now();
            for i in 0..1000 {
                // A different sweep each time, so nothing is cached away: the pass walks the
                // blade past the target the way a real one does.
                let z = -2.0 + i as f32 * 0.004;
                let offset = Vec3::new(0.0, 0.0, z);
                if sweep(&space, me, thickness, a + offset, b + offset, Vec3::new(0.0, 0.0, -0.5))
                    .is_some()
                {
                    found += 1;
                }
            }
            out.0 = started.elapsed().as_secs_f64() * 1e6 / 1000.0;
            out.1 = found;
        })
        .expect("the benchmark system runs");

    let cost = app.world().resource::<Cost>();
    assert!(cost.1 > 0, "1000 casts found nothing at all — the benchmark measured an empty world");
    println!(
        "F-030 cost: {:.2} µs per cast over 1000 casts ({} of them hit) [debian]",
        cost.0, cost.1
    );
}

// ---------------------------------------------------------------------------
// F-034 — the hit stop
// ---------------------------------------------------------------------------

/// ★ **The one that catches the one-line `Time<Virtual>` implementation by name.**
///
/// Three assertions, and the **first** is the one that falls on it: `Time<Fixed>` accumulates
/// its overstep out of `Time<Virtual>::delta()` (`bevy_time-0.19.0/src/fixed.rs:243-247`), so
/// `set_relative_speed(0.05)` stops the tick along with the bodies — and `Tick` is what `Rng`
/// seeds from and what every `Intent` is stamped with.
///
/// 1. `Tick` advances by the full count while the player is frozen.
/// 2. The player's `Position` is **bit-identical** for exactly
///    `round(hit_stop_cortex_s × simulation_hz)` ticks.
/// 3. It differs on the next one.
#[test]
fn f034_the_hit_stop_freezes_the_bodies_and_not_the_clock() {
    let mut app = app();
    let d = data(&app);
    let expected = (d.gear.feel.hit_stop_cortex_s as f64 * d.game.simulation_hz).round() as usize;
    assert_eq!(
        expected, 7,
        "gear.ron feel.hit_stop_cortex_s = {} at {} Hz — the criterion is written against 7 ticks",
        d.gear.feel.hit_stop_cortex_s, d.game.simulation_hz
    );

    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    let hit_tick = fly_until_cut(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 20)
        .expect("the pass has to land, or there is nothing to freeze");

    // Sample the position and the tick at the end of every following step.
    let mut samples: Vec<(u64, [u32; 3])> = Vec::new();
    for _ in 0..20 {
        app.update();
        let p = position(&mut app);
        let tick = app.world().resource::<Tick>().0;
        samples.push((tick, [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]));
    }

    // 1. The clock. Twenty steps, twenty ticks — no matter what the bodies did.
    let advanced = samples.last().expect("samples").0 - samples[0].0 + 1;
    assert_eq!(
        advanced, 20,
        "`Tick` advanced by {advanced} over 20 simulation steps. That is the signature of \
         `Time<Virtual>::set_relative_speed` (or avian's `Time<Physics>`): the fixed step \
         accumulates its overstep from `Time<Virtual>::delta()` \
         (bevy_time-0.19.0/src/fixed.rs:243-247), so slowing time slows the TICK — and the \
         tick carries the rng seed and every intent's stamp."
    );

    // 2. and 3. The body. The freeze begins on the tick after the hit (`combat::hitstop`
    // reacts in `Spatial`, the first stage of the next tick), so the run of identical
    // positions starts there.
    let first = samples
        .iter()
        .position(|(t, _)| *t > hit_tick)
        .expect("the samples must reach past the tick of the hit");
    let frozen_at = samples[first].1;
    let held = samples[first..].iter().take_while(|(_, p)| *p == frozen_at).count();
    assert_eq!(
        held, expected,
        "the player's position was bit-identical for {held} ticks, `gear.ron` says {expected} \
         (hit_stop_cortex_s = {})",
        d.gear.feel.hit_stop_cortex_s
    );
    assert_ne!(
        samples[first + held].1,
        frozen_at,
        "the player never started moving again — a hit stop that does not end is a freeze"
    );

    // And the freeze really was avian's, not a zeroed velocity: the momentum survives it.
    println!(
        "F-034 hit stop: Tick advanced {advanced} over 20 steps, position bit-identical for \
         {held} ticks, moving again on the next"
    );
}

/// The number both halves of the criterion ask for: ticks from first cortex contact until the
/// blade is out of the cortex again, **with and without** the stop.
///
/// Without: a husk cortex is 1.10 m across and 30 m/s covers 0.5 m per tick — `1.10 / 30` is
/// 36.7 ms = **2.2 ticks**, and the blade's own 0.12 m widens the window a little. With a
/// 0.12 s stop the player stands still inside it for seven more.
#[test]
fn f034_the_ticks_inside_the_cortex_with_and_without_the_stop() {
    fn measure(stop: bool) -> usize {
        let mut app = app();
        if !stop {
            let mut d = app.world_mut().resource_mut::<GameData>();
            d.gear.feel.hit_stop_cortex_s = 0.0;
        }
        let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
        let r = 0.55;
        spawn_target(&mut app, 1, cortex, r, false);
        let mut gaps = fly_past(&mut app, cortex, r, 30.0, LANE_Y + 1.6, 2, 4);
        let d = data(&app);
        for _ in 0..16 {
            app.update();
            gaps.push(gap_m(position(&mut app), cortex, r, &d));
        }
        // The blade is inside the cortex while the gap is not positive.
        gaps.iter().filter(|g| **g <= 0.0).count()
    }

    let without = measure(false);
    let with = measure(true);
    assert!(
        with > without,
        "the hit stop did not lengthen the contact at all: {with} ticks with, {without} without"
    );
    println!(
        "F-034 ticks inside the husk cortex at 30 m/s: {without} without the stop \
         (1.10 m / 30 m/s = 2.2 ticks of pure geometry), {with} with the 0.12 s stop"
    );
}

/// The kick is presentation and it **returns to zero exactly**, so the image comes back to the
/// direction the aim ray measures — and it is a pure function of the tick, or the
/// `--offscreen` sha256 stops matching and nobody notices (`docs/PLAN-GAME.md` §11 risk 3).
#[test]
fn f034_the_camera_kick_is_a_function_of_the_tick_and_ends_at_exactly_zero() {
    use defeated_by_titan::render::camera::CameraKick;

    let kick = CameraKick { from_tick: 100, ticks: 11, amplitude_rad: -0.061 };
    assert!((kick.pitch_rad(100) - -0.061).abs() < 1e-9, "full amplitude on the tick of the hit");
    assert!(kick.pitch_rad(105).abs() < 0.061, "the kick decays");
    assert_eq!(kick.pitch_rad(111), 0.0, "the kick must reach EXACTLY zero, not almost");
    assert_eq!(kick.pitch_rad(9999), 0.0);
    assert_eq!(CameraKick::default().pitch_rad(0), 0.0, "no hit, no kick");
    // The same tick gives the same angle, always. That is what bit-identity means here.
    assert_eq!(kick.pitch_rad(103), kick.pitch_rad(103));
}

// ---------------------------------------------------------------------------
// P5 — health and being downed
// ---------------------------------------------------------------------------

/// ★ **Red when the entity is despawned.**
///
/// `MovementState::Downed` documents itself as *"out of the fight instead of dead: a state
/// with a timer, not a removed entity"*. A `despawn` looks identical for one frame and then
/// takes the `PlayerId`, the `Gas`, the hooks and the seat a dropped connection holds for
/// 120 s with it.
#[test]
fn p5_a_downed_player_is_a_state_and_not_a_removed_entity() {
    let mut app = app();
    let me = player(&mut app);
    // ⚠️ The test inserts the `Health` itself, because **no RON file in the repository has a
    // player health number** — see `src/combat/health.rs`. That is the finding, not a
    // convenience: `game.ron: player.health` and `titan.ron: <kind>.damage` are missing, and
    // rule 2 forbids inventing either of them in Rust.
    app.world_mut().entity_mut(me).insert(Health::full(100.0));
    ticks(&mut app, 2);
    assert_ne!(
        app.world().get::<MovementState>(me),
        Some(&MovementState::Downed),
        "a player at full health is already downed"
    );

    // Overkill is the normal case, not the edge case: damage comes out of speed.
    app.world_mut().get_mut::<Health>(me).expect("health").damage(9999.0);
    ticks(&mut app, 2);

    assert!(
        app.world().get_entity(me).is_ok(),
        "the player entity was DESPAWNED at zero health. `MovementState::Downed` is a state \
         with a timer, not a removed entity (src/shared/state.rs) — and `squad` has nobody \
         left to revive."
    );
    assert_eq!(
        app.world().get::<MovementState>(me),
        Some(&MovementState::Downed),
        "health reached zero and the player is not `Downed`"
    );
    assert_eq!(
        app.world().get::<PlayerId>(me),
        Some(&PlayerId(1)),
        "the downed player lost his id — the seat a dropped connection holds hangs on it"
    );
    // And he does not swing any more.
    hold_slash(&mut app);
    ticks(&mut app, 60);
    let swings = app.world().get::<Swings>(me).expect("the player is equipped");
    assert_eq!(
        swings.right.ticks_in_swing, None,
        "a downed player is still swinging his blades"
    );
    println!("P5: health 0 -> MovementState::Downed, entity alive, PlayerId kept");
}

/// The freeze is avian's, not a zeroed velocity — and it is removed again.
#[test]
fn f034_the_freeze_is_lifted_and_leaves_nothing_behind() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);
    fly_until_cut(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 20)
        .expect("the pass has to land, or there is nothing to freeze");
    let me = player(&mut app);

    ticks(&mut app, 1);
    assert!(
        app.world().get::<HitStop>(me).is_some(),
        "no `HitStop` on the player one tick after a cortex cut"
    );
    assert!(
        app.world().get::<RigidBodyDisabled>(me).is_some(),
        "the player is frozen without `RigidBodyDisabled` — a zeroed velocity still \
         accelerates under gravity and the position is not bit-identical"
    );

    ticks(&mut app, 30);
    assert!(app.world().get::<HitStop>(me).is_none(), "the `HitStop` was never removed");
    assert!(
        app.world().get::<RigidBodyDisabled>(me).is_none(),
        "the player stayed disabled — every later tick of the game is a frozen player"
    );
}

// ---------------------------------------------------------------------------
// helpers that need the rig
// ---------------------------------------------------------------------------

fn rig_part(app: &App, root: Entity, part: TitanPart) -> Option<Entity> {
    let mut pending = vec![root];
    while let Some(e) = pending.pop() {
        if app.world().get::<TitanPart>(e) == Some(&part) {
            return Some(e);
        }
        if let Some(kids) = app.world().get::<Children>(e) {
            pending.extend(kids.iter());
        }
    }
    None
}


