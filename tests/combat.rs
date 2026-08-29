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
//!    ([`p5_a_downed_player_is_a_state_and_not_a_removed_entity`]) — and a strike that lands
//!    sixty times in one second, which is what [`p5_one_strike_subtracts_once_and_not_once_per_tick`]
//!    exists to make impossible.
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
    Collider, CollisionLayers, DistanceJoint, GravityScale, LayerMask, LinearVelocity, RigidBody,
    RigidBodyDisabled, Sensor, SpatialQuery,
};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::blades::cut::{blade_segment, limb_zone, sweep};
use defeated_by_titan::blades::swing::{BladeTiming, SweptFrom, Swings};
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{
    BladeRestockRequest, Blades, BodyId, Cli, Health, HitStop, HitZone, HookAnchored, HookReleased,
    HitZoneOf, LocalPlayer, MovementState, PlayerId, ReleaseReason, Side, SpawnTitan, Tick,
    TitanHit, TitanId, TitanState, Velocity, LAYER_PLAYER, LAYER_TITAN_BODY, LAYER_TITAN_CORTEX,
};
use defeated_by_titan::combat::combo::Combo;
use defeated_by_titan::combat::damage::{damage_of, ticks_of, CollapseGuard};
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
    build(Cli { headless: true, ..default() })
}

/// The same app **with a mission running**, so that `mission::decide` is switched on.
///
/// `P5`'s last claim is not about combat at all: the second way to lose already exists in
/// `src/mission/` and was inert because nothing produced a [`Health`]. This app is what proves
/// it fires — the branch is not rebuilt here, it is fed.
fn mission_app() -> App {
    build(Cli { headless: true, mission: Some("tutorial".into()), ..default() })
}

fn build(start: Cli) -> App {
    let mut app = defeated_by_titan::app(start);
    // 🔴 **`Pendulum`, PINNED** — the same line `tests/vector_rope.rs::app` carries and for the
    // same reason. The five `b004_*` tests here are about `combat::hitstop` and a REAL
    // `DistanceJoint`, and `RopeForceModel::Drive` builds none at all
    // (`src/player/locomotion.rs`, `FIND-152`), so they read whichever way `game.ron` happened
    // to be set and all five went red when the shipped default moved on 2026-08-23. Nothing
    // else in this file fires a hook, so nothing else can tell the two models apart.
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model =
        defeated_by_titan::data::RopeForceModel::Pendulum;
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

/// Holds the **right** slash. The right blade lies on `+X` for a player looking at −Z — the
/// axis every pass in this file is built on. The real button, through the real `Intent`
/// channel: no second, wrong way to play.
///
/// ⚠️ Depends on the binding `RMB` → `SLASH_RIGHT` (`src/net/local.rs::read_input`). It has to
/// be the right blade, not the left, because every assertion below reads `Swings.right`. Until
/// 2026-08-10 this pressed `KeyE`; `E` is `HOOK_RIGHT` now and no blade ever swung.
fn hold_slash(app: &mut App) {
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Right);
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

/// The player's [`MovementState`] **as the game wrote it**, not as a test wished it.
fn movement(app: &mut App) -> MovementState {
    let p = player(app);
    *app.world().get::<MovementState>(p).expect("every player carries a MovementState")
}

/// **Puts the player genuinely in the air** at `at_m`, and checks the game agrees.
///
/// 🔴 **Why this is not `insert(MovementState::Airborne)`, which is what three tests here did
/// until 2026-08-26 and what left them red.** `player::integrator::readback` is the *sole*
/// writer of that component and it runs in [`SimulationSystems::Integrate`] — which sits
/// **between the two sets that read it**: `combat::combo::bank` in `Spatial` and
/// `combat::combo::decay`, `combat::strike::land` and `blades::cut` in `PostStep`. So a state
/// inserted from a test is honoured by the first reader and **overwritten before the second**.
/// `f041_hits_in_the_air_raise_the_multiplier_and_a_landing_ends_it` measured exactly that: the
/// two `bank` assertions passed off the inserted `Airborne` and the landing never happened,
/// because `readback` had already put the real state back.
///
/// That is `docs/FINDINGS.md` FIND-103 with the halves swapped — a test that tells the code the
/// answer instead of asking the game for it. **The state has to be produced and then read
/// back**, which is what this and [`stand`] do.
///
/// Gravity off, so that "in the air" stays true for as long as the test needs it. `at_m` has to
/// clear the ground by more than the player capsule's half height (0.9 m,
/// `game.ron: player.height_m`) or he rests on it and the assert below says so.
fn hover(app: &mut App, at_m: Vec3) {
    place(app, at_m, Vec3::ZERO);
    // Two ticks: `readback` writes the state during `Integrate` of the next one.
    ticks(app, 2);
    assert_eq!(
        movement(app),
        MovementState::Airborne,
        "the player was put at {at_m} and the game does not call that airborne — a test that \
         cannot produce the state it is about measures nothing"
    );
}

/// **Puts the player genuinely on his feet** at `at_m` and returns how many ticks the drop took.
///
/// The other half of [`hover`]'s argument: a landing is not a component you can write, it is a
/// contact `player::integrator::readback` finds. Gravity back on, and then the ground does it.
///
/// Panics rather than returning, if he never arrives — a test whose player is still falling is
/// measuring the air.
fn stand(app: &mut App, at_m: Vec3) -> u32 {
    let p = player(app);
    let world = app.world_mut();
    world.entity_mut(p).insert((
        Transform::from_translation(at_m),
        // 1.0 is avian's own default — `place` is what took it away, so this gives it back
        // rather than inventing a value.
        GravityScale(1.0),
        LinearVelocity(Vec3::ZERO),
    ));
    if let Some(mut from) = world.get_mut::<SweptFrom>(p) {
        from.0 = at_m;
    }
    for n in 0..240 {
        app.update();
        if movement(app) == MovementState::Grounded {
            return n + 1;
        }
    }
    panic!("the player never reached the ground from {at_m} in 240 ticks — nothing was measured");
}

// ---------------------------------------------------------------------------
// P5 — the helpers: a real husk, a pinned player, health per tick
// ---------------------------------------------------------------------------

/// The **real** husk out of `titan.ron`, at `at_m`, and the tick it takes to appear.
///
/// Not the fixture below: `combat::strike` resolves a titan's kind off the rig's `Name`, and a
/// fixture has no kind — so these tests measure the same body the game builds, on purpose. If
/// `titan::rig` ever renames a titan, `p5_a_husk_needs_exactly_three_strikes` is what says so.
fn spawn_husk(app: &mut App, at_m: Vec3) -> Entity {
    let before: Vec<Entity> = titans(app);
    app.world_mut().write_message(SpawnTitan {
        kind: "husk".into(),
        pos_x: at_m.x,
        pos_y: at_m.y,
        pos_z: at_m.z,
    });
    // `titan::spawn_titans` runs in `PostStep`, so the body stands at the end of the next tick.
    ticks(app, 2);
    let after = titans(app);
    *after
        .iter()
        .find(|e| !before.contains(e))
        .expect("the husk of titan.ron has to be in the world after two ticks")
}

/// A standing spot `m` metres **in front of** a husk spawned at `husk_at`, at chest height.
///
/// `SpawnTitan` carries no facing (`docs/FINDINGS.md` FIND-012) and `titan::rig` builds the body
/// with an identity rotation, so a fresh husk looks down **−Z**. Every P5 test below used to
/// stand the player at `husk_at.z + 5`, which is his *back* — and that was fine while
/// `combat::strike::reaches` was a cylinder, because the back and the front booked the same 34.
/// Since `Q-031` they do not, so the fixtures say which side they mean.
///
/// The tests that had to move are the ones whose claim is *not* about facing —
/// "three strikes down a player", "one strike subtracts once", "the damage is in the file",
/// "every player down loses the mission". `a_strike_from_behind_books_no_damage` is the one that
/// stands on the other side on purpose.
fn in_his_face(husk_at: Vec3, m: f32) -> Vec3 {
    Vec3::new(husk_at.x, 0.5, husk_at.z - m)
}

fn titans(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<TitanId>>();
    q.iter(app.world()).collect()
}

/// `None` means: this player has no [`Health`] **component**, which is not the same as zero.
fn health(app: &App, who: Entity) -> Option<f32> {
    app.world().get::<Health>(who).map(|h| h.current)
}

fn now(app: &App) -> u64 {
    app.world().resource::<Tick>().0
}

fn any_titan_is_striking(app: &mut App) -> bool {
    let mut q = app.world_mut().query::<&TitanState>();
    q.iter(app.world()).any(|s| *s == TitanState::Strike)
}

/// What one run of `n` ticks did to the local player.
#[derive(Debug)]
struct Watch {
    /// `(tick, health afterwards)` for every tick the number changed — **one entry per landed
    /// strike**, which is the whole claim of `P5`.
    drops: Vec<(u64, f32)>,
    /// The first tick the player was [`MovementState::Downed`].
    downed_at: Option<u64>,
    /// How many strikes were *begun* in this run — the guard against a test that proves
    /// nothing because the titan never swung at all.
    strikes: u32,
}

fn watch(app: &mut App, n: u64) -> Watch {
    let me = player(app);
    let mut w = Watch { drops: Vec::new(), downed_at: None, strikes: 0 };
    let mut last = health(app, me);
    let mut was_striking = any_titan_is_striking(app);
    for _ in 0..n {
        app.update();
        let t = now(app);
        let current = health(app, me);
        if let (Some(before), Some(after)) = (last, current)
            && (before - after).abs() > 1e-4
        {
            w.drops.push((t, after));
        }
        last = current;
        let striking = any_titan_is_striking(app);
        if striking && !was_striking {
            w.strikes += 1;
        }
        was_striking = striking;
        if w.downed_at.is_none()
            && app.world().get::<MovementState>(me) == Some(&MovementState::Downed)
        {
            w.downed_at = Some(t);
        }
    }
    w
}

// ---------------------------------------------------------------------------
// the fixture — the same three components the real cortex carries
// ---------------------------------------------------------------------------

/// The lane every pass is flown in. Above the church (35 m), the tallest thing in the city.
const LANE_Y: f32 = 60.0;
/// How far to the right of the flight line the target sits: **half of `gear.ron: reach_m`**, so
/// the blade straddles it and the test measures the cut and not the last centimetre of reach.
///
/// ⚠️ 0.8 → 1.0 on 2026-08-20 with `reach_m` 1.6 → 2.0, and it is not cosmetic. At 0.8 the hand
/// sat at x −0.8 and a 2.0 m blade reached to x +1.2, plus `thickness_m` to **+1.45** — and a
/// husk's arm box spans `w/2 .. 3w/4` = **1.25 .. 1.875** (`titan::rig`'s header). So every
/// "chest pass" in this file silently started clipping the right arm: an extra `HitZoneOf`
/// refinement, an extra zone in the mask, an extra 0.06 of sharpness and a second stagger.
/// Three tests went red on it at once and all three were right to.
/// `f032_the_chest_is_still_the_torso_after_the_limbs_got_their_own_zones` asserts the tie to
/// `reach_m`, so the next person to move that number gets a red test and not a mystery.
const REACH_X: f32 = 1.0;

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
            // `gear.ron: reach_m` is 2.00 m — that is a finding about the numbers, not about
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
    // is solid and 2.5 m wide and `gear.ron: reach_m` is 2.00 m — measured, the player at
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
                // `|_, _| true` — the gate of `F-030`'s rear cone is not what this benchmark
                // measures, and a rejected cortex would fall through to a SECOND cast and make
                // the µs figure describe two casts on some iterations and one on others.
                if sweep(
                    &space,
                    me,
                    thickness,
                    a + offset,
                    b + offset,
                    Vec3::new(0.0, 0.0, -0.5),
                    |_, _| true,
                )
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
// F-033 — the blade wears out, which is the half that makes the rack mean anything
// ---------------------------------------------------------------------------

/// Reads the local player's harness.
fn harness(app: &mut App) -> Blades {
    let me = player(app);
    *app.world().get::<Blades>(me).expect("`player::spawn_player` gives every player a harness")
}

/// ★ **The one that falls on a monotone harness.**
///
/// Until 2026-08-13 `gear.ron: blades.wear_per_hit` had **no reader anywhere in `src/`**
/// (`docs/FINDINGS.md` FIND-075). [`Blades`] only ever grew — at a rack — so *economy instead of
/// cooldowns* had a way back and nothing to come back from: `scripts/f070-hub.txt`'s
/// `assert blades == 5` was a tautology written down as one, and the five HUD pips of `F-170`
/// were a constant dressed as a gauge.
///
/// This flies the pass [`f030_the_cortex_wins_over_the_body_it_hides_in`] flies and then asks
/// the harness. **Nothing here is compared against a literal**: the floor is `wear_per_hit` read
/// out of the file, so retuning the number moves the test with it.
#[test]
fn f033_a_cut_costs_the_blade_that_made_it() {
    let mut app = app();
    let d = data(&app);
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);

    let before = harness(&mut app);
    assert_eq!(before.sharpness, 1.0, "the pass has to start on a fresh pair, got {before:?}");

    fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 6);
    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(
        zones.contains(&HitZone::Cortex),
        "the pass never reached the nape, so there is nothing to charge for: {zones:?}"
    );

    let after = harness(&mut app);
    assert!(
        after.sharpness < before.sharpness,
        "the blade cut {zones:?} and the harness is untouched at {after:?}. \
         `gear.ron: blades.wear_per_hit` ({}) has no reader, `Blades` is monotone upward, and a \
         rack that hands back what was never taken is a no-op with a floor pad under it",
        d.gear.blades.wear_per_hit
    );
    // **The exact arithmetic, re-derived from the file rather than from `wear_of`.** A wear
    // number typed into Rust does not pass this, and neither does a system that charges the
    // right amount for the wrong zone.
    let b = &d.gear.blades;
    let expected: f32 = zones
        .iter()
        .map(|z| if *z == HitZone::Cortex { b.wear_per_hit } else { b.wear_per_hit * b.wear_torso_factor })
        .sum();
    assert!(
        (before.sharpness - after.sharpness - expected).abs() < 1e-5,
        "the pass cut {zones:?} and cost {} sharpness; gear.ron says {expected}",
        before.sharpness - after.sharpness
    );
    println!(
        "F-033 one pass: zones {zones:?}, sharpness {:.3} -> {:.3}, pairs {} -> {}",
        before.sharpness, after.sharpness, before.pairs_left, after.pairs_left
    );
}

/// ★ **The design claim as a test, against the file and not against a fixture.**
///
/// The unit tests in `src/blades/cut.rs` spell `gear.ron`'s numbers out in a `tuning()` fixture,
/// which is the house style — and it means they stay green when the **file** changes. This one
/// is the guard on the file itself, and it exists because flipping `wear_torso_factor` from 0.5
/// to 2.0 was measured to break nothing else in the suite.
///
/// The claim: **a graze must cost less than a cut.** A pass that ends in a nape reports
/// `[Torso, Cortex]` on every titan in the game, because every titan is wider than his own neck
/// ([`f030_the_cortex_wins_over_the_body_it_hides_in`]) — so a factor above 1.0 charges the
/// player more for the shape of the titan's shoulders than for the nape he actually hit, and the
/// harness becomes a tax on winning. That is the cooldown-shaped design `docs/gameplay/` rejects.
#[test]
fn f033_the_file_charges_a_graze_less_than_a_cut() {
    let app = app();
    let b = data(&app).gear.blades;
    assert!(
        b.wear_torso_factor > 0.0,
        "gear.ron: blades.wear_torso_factor is {} — a free graze means a player can flail at \
         bodies all mission long at no cost",
        b.wear_torso_factor
    );
    assert!(
        b.wear_torso_factor < 1.0,
        "gear.ron: blades.wear_torso_factor is {} — at or above 1.0 the graze every successful \
         kill pays on the way in costs at least as much as the cut itself, and the harness \
         becomes a tax on winning instead of on imprecision",
        b.wear_torso_factor
    );
    // And the budget the whole feature is judged on, stated out loud so it cannot drift silently.
    let per_kill = b.wear_per_hit * (1.0 + b.wear_torso_factor);
    let kills = (1.0 / per_kill) * (f32::from(b.start_pairs) + 1.0);
    assert!(
        (4.0..=80.0).contains(&kills),
        "a full harness is worth {kills:.1} kills at {per_kill:.3} sharpness each — outside the \
         range in which walking to a rack is a decision rather than a formality or a chore"
    );
    println!(
        "F-033 budget: {per_kill:.3} sharpness a kill, {kills:.1} kills out of a full harness \
         ({} spares + the pair in hand)",
        b.start_pairs
    );
}

/// ★ **The one that says "out of blades" is a place to go and not a dead end.**
///
/// Two claims in one pass, and both of them are `F-033`'s reason to exist:
///
/// 1. **A dry harness cuts nothing.** Not "cuts for less" — `titan::brain::receive_hits` kills on
///    `Cortex` by rule and never looks at the speed, so a broken blade that still wrote a
///    `TitanHit` would be a free kill with no steel behind it.
/// 2. **And the rack gives it back.** Which is the sentence the whole hub was built for, and the
///    one that could not be tested in a running game until something took a blade away
///    (`docs/FINDINGS.md` FIND-075 §2).
///
/// The **walk** to the hall is `tests/mission.rs`'s claim, not this file's — so the request the
/// rack sends is written straight into the world here.
#[test]
fn f033_a_dry_harness_cannot_cut_and_a_rack_is_the_way_back() {
    let mut app = app();
    let cortex = Vec3::new(REACH_X, LANE_Y + 1.6, 0.0);
    spawn_target(&mut app, 1, cortex, 0.55, false);

    // The pair in his hands is spent and there is no spare behind it.
    let me = player(&mut app);
    app.world_mut().entity_mut(me).insert(Blades { pairs_left: 0, sharpness: 0.0 });
    assert!(harness(&mut app).is_broken(), "the fixture did not start dry");

    fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 6);
    let on_empty: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(
        on_empty.is_empty(),
        "a broken blade reported {on_empty:?}. `titan::brain::receive_hits` kills on Cortex by \
         rule, so that is a kill with no steel behind it"
    );

    // The way back. One second of the rack: `sharpen_per_s: 2.0` makes the pair in his hands
    // fresh, `blade_pairs_per_s: 1.5` puts a spare behind it.
    let id = *app.world().get::<PlayerId>(me).expect("the player carries his id");
    app.world_mut().write_message(BladeRestockRequest { player: id, seconds: 1.0 });
    ticks(&mut app, 2);
    let back = harness(&mut app);
    assert!(!back.is_broken(), "a second at a rack left the harness broken: {back:?}");
    assert_eq!(back.sharpness, 1.0, "the pair in his hands was not honed: {back:?}");
    assert_eq!(back.pairs_left, 1, "a second at 1.5 pairs/s owes exactly one spare: {back:?}");

    // Same pass, same geometry, and now it lands.
    fly_past(&mut app, cortex, 0.55, 30.0, LANE_Y + 1.6, 2, 6);
    let restocked: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(
        restocked.contains(&HitZone::Cortex),
        "restocked at a rack and the same pass still cuts nothing: {restocked:?}"
    );
    let after = harness(&mut app);
    assert!(after.sharpness < 1.0, "the restocked pair cut and was not charged: {after:?}");
    println!(
        "F-033 dry -> rack -> cutting again: {on_empty:?} then {restocked:?}, harness {after:?}"
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
    // The test no longer inserts the `Health` itself: `game.ron: player.health` exists since
    // 2026-08-09 and `combat::health::grant` puts the component on the player. That is what
    // turns this test from a statement about a component the game never had into a statement
    // about the running game.
    ticks(&mut app, 2);
    assert_eq!(
        health(&app, me),
        Some(data(&app).game.player.health),
        "the player does not carry `game.ron: player.health` — nothing produces health, and \
         every assertion below measures a component the test put there itself"
    );
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

/// The producer, on its own: a player who has never been touched carries the file's number.
///
/// Red when nothing installs [`Health`] — and that was the state of the repository until this
/// job: the HUD's crimson bar hid itself, `assert health > 0` measured nothing, and
/// `mission::decide`'s second loss branch queried an empty set.
#[test]
fn p5_the_player_carries_the_health_the_file_gives_him() {
    let mut app = app();
    let d = data(&app);
    let me = player(&mut app);
    // One tick, not zero: the player is spawned in `Startup` through `Commands` and `grant`
    // runs in `FixedUpdate`, so the component exists at the end of the tick the player does —
    // measured, not assumed.
    ticks(&mut app, 1);
    let h = app.world().get::<Health>(me).copied().expect(
        "the local player has no `Health` — `game.ron: player.health` exists, so somebody has \
         to install it (src/combat/health.rs::grant)",
    );
    assert_eq!(h.max, d.game.player.health, "the maximum is not the number in game.ron");
    assert_eq!(h.current, h.max, "a player starts a sortie hurt");
    assert!(h.max > 0.0, "a player with 0 max health is downed before the mission starts");
    // And it is the local player's, not a global: twenty players, twenty numbers.
    let mut q = app.world_mut().query_filtered::<&Health, With<PlayerId>>();
    assert_eq!(q.iter(app.world()).count(), 1, "one player, one health component");
    println!("P5: player health {} / {} out of game.ron", h.current, h.max);
}

/// ★ **The one the whole feature is calibrated for.**
///
/// `game.ron: player.health` (100) against `titan.ron: husk.damage` (34) is **three strikes**,
/// and the quotient is computed out of the two files here — a `3` written into this test would
/// survive any change to either number.
///
/// It goes red on both of the failure modes that matter:
///
/// 1. **A literal instead of the file.** `p5_the_damage_comes_out_of_the_file_and_not_out_of_rust`
///    is the inversion of that one, in-process: it changes the number and the count moves.
/// 2. **A subtraction per tick instead of per strike.** ⚠️ Counting the subtractions is *not*
///    enough for that one and the first version of this test was wrong about it: three
///    per-tick subtractions also read 100 → 66 → 32 → 0. What separates the two is **when**
///    they land — one blow per `attack_cooldown_s`, 90 ticks apart, against three ticks in a
///    row inside a single 12-tick `Strike`. Measured: the inversion produces
///    `[(39, 66), (40, 32), (41, 0)]`, so the gap is what this test asserts.
#[test]
fn p5_a_husk_needs_exactly_three_strikes() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let start = d.game.player.health;
    let damage = husk.damage;
    assert!(damage > 0.0, "titan.ron husk.damage = {damage} — a strike that costs nothing");
    let needed = (start / damage).ceil() as u32;
    assert_eq!(
        needed, 3,
        "game.ron player.health = {start} against titan.ron husk.damage = {damage} is {needed} \
         strikes. The criterion of this session is written against THREE — either the files were \
         retuned (then this number is the new truth and the line above is what has to move) or \
         one of the two is wrong."
    );

    // Five metres out: inside `attack_range_m` (6.0) so the husk commits, outside his own body
    // (2.5 m wide) so avian never touches the player and the distance stays what it was set to.
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);

    // 90 ticks per attack (`attack_cooldown_s` 1.5 s, longer than wind-up + strike + recover),
    // so three of them fit inside 400 with room to spare.
    let w = watch(&mut app, 400);

    assert_eq!(
        w.drops.len(),
        3,
        "the husk landed {} subtractions in 400 ticks, not 3 ({} strikes were begun): {:?}. \
         Twelve in a row is the per-tick failure; zero is a strike that reaches nobody.",
        w.drops.len(),
        w.strikes,
        w.drops
    );
    for (i, (tick, left)) in w.drops.iter().enumerate() {
        let expected = (start - damage * (i + 1) as f32).max(0.0);
        assert!(
            (left - expected).abs() < 1e-3,
            "after strike {} at tick {tick} the player has {left}, {start} − {} × {damage} is \
             {expected}",
            i + 1,
            i + 1
        );
    }

    // ★ One blow per attack, not three inside one. `attack_cooldown_s` is measured from one
    // wind-up to the next and is longer than wind-up + strike + recover, so it *is* the period.
    let cooldown = (husk.attack_cooldown_s as f64 * d.game.simulation_hz).round() as u64;
    assert_eq!(cooldown, 90, "titan.ron husk.attack_cooldown_s = {}", husk.attack_cooldown_s);
    for pair in w.drops.windows(2) {
        let gap = pair[1].0 - pair[0].0;
        assert_eq!(
            gap, cooldown,
            "two subtractions {gap} ticks apart, `attack_cooldown_s` is {cooldown} ticks: {:?}. \
             A gap of one is three hits inside ONE 12-tick strike — the player never got to see \
             a second wind-up, and the telegraph he was supposed to read never mattered.",
            w.drops
        );
    }
    assert!(w.strikes >= 3, "only {} strikes were begun in 400 ticks", w.strikes);

    let downed = w.downed_at.expect("three strikes and the player is still on his feet");
    assert!(
        downed >= w.drops[1].0 + cooldown,
        "the player was `Downed` at tick {downed}, only {} ticks after the SECOND strike at \
         tick {} which left him {} health. He has to survive a whole attack cycle in between.",
        downed - w.drops[1].0,
        w.drops[1].0,
        w.drops[1].1
    );
    assert!(
        downed >= w.drops[2].0 && downed <= w.drops[2].0 + 2,
        "the third strike landed at tick {} and the player went down at {downed} — \
         `down_at_zero` runs in `Drive`, one tick after `strike::land` in `PostStep`, so the \
         gap is 1 and never 40",
        w.drops[2].0
    );
    println!(
        "P5 three strikes: {start} -> {} -> {} -> {} · strikes at ticks {:?} · Downed at tick \
         {downed}",
        w.drops[0].1, w.drops[1].1, w.drops[2].1,
        w.drops.iter().map(|(t, _)| *t).collect::<Vec<_>>()
    );
}

/// ★ **The one that makes "sixty hits a second" impossible.**
///
/// A husk's `Strike` lasts `round(strike_s × simulation_hz)` = 12 ticks. Over that whole window
/// there is **exactly one** subtraction. The obvious implementation — subtract while the state
/// is `Strike` — passes every "the player takes damage" test there is and turns the impact
/// frame into a damage multiplier.
#[test]
fn p5_one_strike_subtracts_once_and_not_once_per_tick() {
    let mut app = app();
    let d = data(&app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let start = d.game.player.health;
    let strike_ticks = (husk.strike_s as f64 * d.game.simulation_hz).round() as u64;
    assert_eq!(
        strike_ticks, 12,
        "titan.ron husk.strike_s = {} at {} Hz — the criterion is written against 12 ticks",
        husk.strike_s, d.game.simulation_hz
    );

    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);

    // Only as far as the first blow plus its own length: a second attack is 90 ticks away, so
    // anything counted inside this window belongs to strike number one.
    let w = watch(&mut app, 40 + strike_ticks + 2);

    assert_eq!(w.strikes, 1, "{} strikes were begun, the window is written for one", w.strikes);
    assert_eq!(
        w.drops.len(),
        1,
        "one strike of {strike_ticks} ticks produced {} subtractions: {:?}. That is the whole \
         failure mode — the state is `Strike` on every one of those ticks and a system that \
         reads the state instead of booking the blow subtracts on every one of them.",
        w.drops.len(),
        w.drops
    );
    let me = player(&mut app);
    assert!(
        (health(&app, me).expect("health") - (start - husk.damage)).abs() < 1e-3,
        "after one strike the player has {:?}, {start} − {} is {}",
        health(&app, me),
        husk.damage,
        start - husk.damage
    );
    println!(
        "P5 one strike: {strike_ticks} ticks of `Strike`, {} subtraction at tick {} — \
         {start} -> {}",
        w.drops.len(),
        w.drops[0].0,
        w.drops[0].1
    );
}

/// ★ **The inversion of "the number is a literal", run in-process.**
///
/// The same husk, the same code, one number changed: at 50 damage a player with 100 health goes
/// down on the **second** strike. A `34.0` compiled into Rust keeps needing three, and this test
/// is what says so out loud — without touching `assets/data/`, which belongs to the main head.
#[test]
fn p5_the_damage_comes_out_of_the_file_and_not_out_of_rust() {
    let mut app = app();
    let start = data(&app).game.player.health;
    let retuned = start / 2.0;
    {
        let mut d = app.world_mut().resource_mut::<GameData>();
        d.titans.kinds.get_mut("husk").expect("titan.ron has a husk").damage = retuned;
    }

    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);
    let w = watch(&mut app, 300);

    assert_eq!(
        w.drops.len(),
        2,
        "with husk.damage retuned to {retuned} against {start} health the player takes 2 \
         strikes, not {}: {:?}. A literal in Rust does not move when the file does.",
        w.drops.len(),
        w.drops
    );
    assert!((w.drops[0].1 - retuned).abs() < 1e-3, "first strike left {}", w.drops[0].1);
    assert_eq!(w.drops[1].1, 0.0, "second strike left {}", w.drops[1].1);
    assert!(w.downed_at.is_some(), "two strikes at half health and nobody is down");
    println!(
        "P5 the number is the file's: husk.damage {retuned} -> {} strikes instead of 3",
        w.drops.len()
    );
}

/// Out of reach is out of reach — on the ground plane **and** upwards.
///
/// Both halves let the husk really swing (`strikes >= 1`), or the test would prove nothing: a
/// titan that never attacked also never took any health, and that passes.
///
/// * **Horizontal**: the player steps out to `attack_range_m + 2` while the wind-up is running.
///   The titan is committed, he swings, and he hits air — a wind-up that could not be walked
///   out of would make `windup_s` a countdown to a guaranteed hit instead of a telegraph.
/// * **Vertical**: 60 m straight up is 0 m away on the ground plane. A reach that is only a
///   ground distance hits a player who is flying over the titan's head.
#[test]
fn p5_a_strike_out_of_range_takes_nothing() {
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    let reach = {
        let app = app();
        data(&app).titan("husk").expect("titan.ron has a husk").attack_range_m
    };

    /// One half: put the husk into his wind-up with the player in reach, then move the player
    /// to `escape_to` and watch the committed blow land on nothing.
    fn swing_at(husk_at: Vec3, escape_to: Vec3) -> (Watch, f32) {
        let mut app = app();
        let start = data(&app).game.player.health;
        let body = spawn_husk(&mut app, husk_at);
        // In his FACE, not his back — since `Q-031` a blow into the back books nothing anyway,
        // and a test that would pass with `reach_m: 0.0` measures nothing (`in_his_face`).
        place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);
        // Run until he is committed: `Windup` is the point of no return (`titan::brain::decide`
        // has no edge out of it except through `Strike`).
        let mut committed = false;
        for _ in 0..200 {
            app.update();
            if app.world().get::<TitanState>(body) == Some(&TitanState::Windup) {
                committed = true;
                break;
            }
        }
        assert!(committed, "the husk never wound up — nothing is being measured");
        // He does not walk during `Windup` or `Strike` (`titan::brain::walk` moves nothing that
        // is not in `Pursue`), so the player stays where he is put. He does now *turn* during
        // `Windup` (`Q-031`), but the escape below is straight down his forward vector, so the
        // yaw he wants is the yaw he already has and the turn is 0°.
        place(&mut app, escape_to, Vec3::ZERO);
        let w = watch(&mut app, 60);
        let me = player(&mut app);
        let left = health(&app, me).expect("the player has health");
        assert!(w.strikes >= 1, "the committed wind-up never became a strike");
        assert_eq!(left, start, "the player was hit: {:?}", w.drops);
        (w, start)
    }

    // ---- horizontal: two metres past `attack_range_m`, still well inside `aggro_radius_m`
    let (w, _) = swing_at(husk_at, in_his_face(husk_at, reach + 2.0));
    assert!(
        w.drops.is_empty(),
        "a strike with a reach of {reach} m took health off a player standing at {} m: {:?}",
        reach + 2.0,
        w.drops
    );

    // ---- vertical: straight over his head — 0 m away on the ground plane, 60 m up
    let (w, _) = swing_at(husk_at, Vec3::new(0.0, LANE_Y, husk_at.z));
    assert!(
        w.drops.is_empty(),
        "the husk hit a player {LANE_Y} m over his head: {:?}. The reach is a ground distance \
         with a ceiling, not a cylinder to the sky (src/combat/strike.rs).",
        w.drops
    );
    println!(
        "P5 out of reach: attack_range_m {reach} — nothing at {} m on the ground, nothing at \
         {LANE_Y} m up",
        reach + 2.0
    );
}

/// ★ **The second way to lose, end to end — and none of it is built here.**
///
/// `mission::decide` has carried "every player down ⇒ `Lost`" since `F-070` and it was inert,
/// because the query it reads was empty: nothing in the running game produced a [`Health`].
/// This test does not rebuild that branch, it **feeds** it — which is the only honest way to
/// find out whether a placeholder was ever right.
#[test]
fn p5_the_mission_is_lost_when_every_player_is_down() {
    use defeated_by_titan::mission::MissionPhase;

    let mut app = mission_app();
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Active,
        "`--mission tutorial` has to be running, or `decide` never gets to run at all"
    );

    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);
    // ⚠️ **1200 and not 400, and the reason is not this test.** This is the only test in the
    // file that runs [`mission_app`], so it is the only one where the player is the target of a
    // *crowd*: the tutorial queues four titans on a 24 m ring on top of the husk spawned above.
    // `titan::perception::claim_slots` gained a `titan.ron: crowd.arrive_m` gate on 2026-08-26
    // — *"a titan of a crowd HOLDS HIS ATTACK until he is inside `arrive_m` of his slot"* — and
    // a titan that starts inside his own reach now walks to a ring slot first. Measured with a
    // throwaway probe on the same fixture: the three blows moved from 449 / 539 / 629 to
    // **718 / 808 / 898**, `Downed` from 630 to **899**. Nothing was starved and no assert below
    // changed; the run needs 4.5 s more of clock. `docs/FINDINGS.md` FIND-166.
    let w = watch(&mut app, 1200);

    let downed = w.downed_at.expect("the player never went down — there is no loss to check");
    let phase = *app.world().resource::<State<MissionPhase>>().get();
    assert_eq!(
        phase,
        MissionPhase::Lost,
        "the player is down at tick {downed} and the mission says {}. The tutorial's clock is \
         330 s = 19 800 ticks, so this cannot be the timeout — it is the second way to lose",
        phase.label()
    );
    // And it is not the clock and not a win: the verdict was spoken at the tick the player fell.
    let mut q = app.world_mut().query::<&defeated_by_titan::mission::MissionClock>();
    let clock = *q.iter(app.world()).next().expect("a running mission has a clock");
    let decided = clock.decided_at_tick.expect("a decided mission records its tick");
    assert!(
        decided < clock.deadline_tick(),
        "decided at {decided}, deadline at {} — this run measured the timeout, not the player",
        clock.deadline_tick()
    );
    assert!(
        decided.abs_diff(downed) <= 2,
        "the player went down at tick {downed} and the mission was decided at {decided}"
    );
    println!(
        "P5 second loss path: player down at tick {downed}, mission LOST at tick {decided} \
         (deadline {}), drops {:?}",
        clock.deadline_tick(),
        w.drops
    );
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
// B-004 — the hit stop meets the rope
// ---------------------------------------------------------------------------

/// Hangs the player on a real [`DistanceJoint`], built by the system that owns it.
///
/// The message and not the hook FSM: `player::rope::attach_ropes` reads [`HookAnchored`], and
/// what this file has to bring together is a **joint on the player's body** and a **hit stop**
/// — not `vector::hook`, which has its own tests. The anchor is straight above the player and
/// gravity is off (`place`), so the rope hangs slack and moves nobody.
fn hang_a_rope(app: &mut App, above_m: f32) {
    let me = player(app);
    let at_m = position(app);
    let id = *app.world().get::<PlayerId>(me).expect("the player carries his id");
    let tick = now(app);
    app.world_mut().write_message(HookAnchored {
        player: id,
        side: Side::Right,
        body: BodyId(80_004),
        point_x: at_m.x,
        point_y: at_m.y + above_m,
        point_z: at_m.z,
        tick,
    });
    // `attach_ropes` runs in `Drive`, so the joint stands at the end of the next tick.
    ticks(app, 2);
    assert_eq!(joints(app), 1, "no rope came into being — the rest of this test measures nothing");
}

/// Lets the rope go the way every release in the game does: through [`HookReleased`], which
/// `player::rope::detach_ropes` carries out by despawning the joint entity.
fn let_the_rope_go(app: &mut App) {
    let me = player(app);
    let id = *app.world().get::<PlayerId>(me).expect("the player carries his id");
    let tick = now(app);
    app.world_mut().write_message(HookReleased {
        player: id,
        side: Side::Right,
        reason: ReleaseReason::Released,
        tick,
    });
    ticks(app, 2);
}

fn joints(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&DistanceJoint>();
    q.iter(app.world()).count()
}

/// Lands a cortex hit on the player without flying a pass — [`TitanHit`] is the message
/// `combat::hitstop::begin` reacts to, and the blade that writes it has its own tests above.
fn land_a_cortex_hit(app: &mut App) {
    let me = player(app);
    let id = *app.world().get::<PlayerId>(me).expect("the player carries his id");
    app.world_mut().write_message(TitanHit {
        titan: TitanId(1),
        by: id,
        zone: HitZone::Cortex,
        speed_m_s: 30.0,
    });
}

/// ★ **`B-004` — the game's core loop: cut a titan while a rope is on you, then let go.**
///
/// Before the fix this test does not fail, it **aborts the process**:
///
/// ```text
/// thread 'main' panicked at avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:786:9:
/// assertion failed: island.joint_count > 0
/// ```
///
/// The chain, read out of avian's own source: `RigidBodyDisabled` on the player makes
/// `IslandPlugin`'s `On<Insert, (Disabled, RigidBodyDisabled)>` observer strip his
/// `BodyIslandNode` (`islands/mod.rs:126-136`); that component's `on_remove` hook takes the
/// last body out of the island and **removes the island while its `joint_count` is still 1**
/// (`islands/mod.rs:1338-1385`). When the freeze lifts, the body gets a fresh
/// `BodyIslandNode`, which recycles exactly that island slot — with `joint_count` back at 0.
/// The despawn of the joint then decrements it (`islands/mod.rs:786`) and the assert fires.
///
/// That is also why the release **inside** the impact frame was clean in
/// `scripts/f-flight-cut.txt`: the slot had not been handed out again yet.
#[test]
fn b004_a_cut_landed_on_a_rope_survives_letting_the_rope_go() {
    let mut app = app();
    place(&mut app, Vec3::new(0.0, LANE_Y, 0.0), Vec3::ZERO);
    app.update();
    hang_a_rope(&mut app, 9.0);

    let me = player(&mut app);
    land_a_cortex_hit(&mut app);
    ticks(&mut app, 1);
    assert!(
        app.world().get::<HitStop>(me).is_some(),
        "the hit did not freeze the player, so this test never met the bug"
    );
    assert_eq!(joints(&mut app), 1, "the rope has to still be there during the freeze");

    // Past the freeze — this is where the island slot is handed out a second time.
    ticks(&mut app, 12);
    assert!(app.world().get::<HitStop>(me).is_none(), "the freeze never ended");

    let_the_rope_go(&mut app);
    assert_eq!(joints(&mut app), 0, "the rope was not removed");
    ticks(&mut app, 5);
}

/// The other half of the same corruption: a hook that **bites** while the player is frozen.
///
/// `add_joint_to_graph` merges the islands of both ends (`joint_graph/plugin.rs:143-152`), and
/// a disabled body has no island at all — `merge_islands` panics with *"Neither body … nor …
/// is in an island"* (`islands/mod.rs:814-830`). Same cause, one tick earlier.
#[test]
fn b004_a_hook_that_bites_during_the_freeze_does_not_abort_the_process() {
    let mut app = app();
    place(&mut app, Vec3::new(0.0, LANE_Y, 0.0), Vec3::ZERO);
    app.update();

    let me = player(&mut app);
    land_a_cortex_hit(&mut app);
    ticks(&mut app, 2);
    assert!(app.world().get::<RigidBodyDisabled>(me).is_some(), "the player is not frozen");

    hang_a_rope(&mut app, 9.0);
    ticks(&mut app, 12);
    assert!(app.world().get::<HitStop>(me).is_none(), "the freeze never ended");
    let_the_rope_go(&mut app);
    assert_eq!(joints(&mut app), 0, "the rope was not removed");
    ticks(&mut app, 5);
}

/// `F-034`'s proven behaviour, **with a rope on the player** — the fix must not buy the crash
/// with a freeze that no longer freezes.
///
/// Same criterion as `f034_the_hit_stop_freezes_the_bodies_and_not_the_clock`: the position is
/// bit-identical for exactly `round(hit_stop_cortex_s × simulation_hz)` = 7 ticks, and it
/// moves again on the next one. A joint that is still solved through the impact frame moves
/// the body by millimetres, and millimetres are not bit-identical.
#[test]
fn b004_the_freeze_is_still_bit_identical_with_a_rope_attached() {
    let mut app = app();
    let d = data(&app);
    let expected = (d.gear.feel.hit_stop_cortex_s as f64 * d.game.simulation_hz).round() as usize;
    assert_eq!(expected, 7, "the criterion is written against 7 ticks");

    // Gravity **on** and the anchor to one side, so the rope is really under load: a slack
    // rope would hold still even without a fix.
    place(&mut app, Vec3::new(0.0, LANE_Y, 0.0), Vec3::new(0.0, 0.0, -20.0));
    let me = player(&mut app);
    app.world_mut().entity_mut(me).insert(GravityScale(1.0));
    app.update();
    hang_a_rope(&mut app, 6.0);
    ticks(&mut app, 6); // let him fall into the rope, so the joint is taut

    land_a_cortex_hit(&mut app);
    let mut samples: Vec<[u32; 3]> = Vec::new();
    for _ in 0..20 {
        app.update();
        let p = position(&mut app);
        samples.push([p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]);
    }
    let frozen_at = samples[0];
    let held = samples.iter().take_while(|p| **p == frozen_at).count();
    assert_eq!(
        held, expected,
        "the player on a rope was bit-identical for {held} ticks instead of {expected} — \
         the joint was solved through the impact frame"
    );
    assert_ne!(samples[held], frozen_at, "the freeze never ended");

    let_the_rope_go(&mut app);
    ticks(&mut app, 5);
    println!("B-004: 7 frozen ticks with a taut rope, and the rope let go afterwards");
}

// ---------------------------------------------------------------------------
// B-004 — the sweep. This is the test the two previous "fixes" would have failed.
// ---------------------------------------------------------------------------

/// What one release attempt did. A [`Release::Aborted`] is a **process abort**, not a failed
/// assertion — avian's island bookkeeping panics, and the panic is caught here on purpose so
/// that the sweep can report the whole bracket instead of stopping at its first tick.
#[derive(Debug)]
enum Release {
    Clean,
    Leaked(usize),
    Aborted(String),
}

fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<panic with a non-string payload>".to_string()
    }
}

/// Builds a fresh app, lands a cortex hit, waits `release_after` ticks and lets the rope go.
///
/// `attach_after`: `None` hangs the rope **before** the hit (the reported repro — a player who
/// was already swinging when he cut). `Some(n)` hangs it `n` ticks **after** the hit, i.e.
/// inside the impact frame, which is the joint that is born carrying `JointDisabled`.
///
/// A fresh `App` per tick and not one app swept in place: an aborted island bookkeeping is
/// **world state**, so a second release in the same world would measure the first one's wreck.
fn release_at(attach_after: Option<u64>, release_after: u64) -> Release {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut app = app();
        place(&mut app, Vec3::new(0.0, LANE_Y, 0.0), Vec3::ZERO);
        app.update();
        if attach_after.is_none() {
            hang_a_rope(&mut app, 9.0);
        }
        land_a_cortex_hit(&mut app);
        match attach_after {
            None => ticks(&mut app, release_after),
            Some(n) => {
                // `hang_a_rope` costs 2 ticks of its own; the sweep counts from the hit.
                ticks(&mut app, n);
                hang_a_rope(&mut app, 9.0);
                ticks(&mut app, release_after.saturating_sub(n + 2));
            }
        }
        let_the_rope_go(&mut app);
        // Past the end of any freeze, so that a thaw that trips over a joint that is already
        // gone is counted here too — the mirror of the abort this sweep is named after.
        ticks(&mut app, 20);
        joints(&mut app)
    }));
    match outcome {
        Ok(0) => Release::Clean,
        Ok(n) => Release::Leaked(n),
        Err(payload) => Release::Aborted(panic_text(payload)),
    }
}

/// Runs one sweep and returns the ticks that did not survive, with what killed them.
fn sweep_releases(attach_after: Option<u64>, range: std::ops::RangeInclusive<u64>) -> Vec<(u64, String)> {
    // avian's panic is expected on the red side of this test and prints a full backtrace line
    // per tick. The hook is silenced for the sweep and restored right after it, so a genuine
    // assertion failure inside `release_at` still arrives — through `Release::Aborted`.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut dead = Vec::new();
    for t in range {
        match release_at(attach_after, t) {
            Release::Clean => {}
            Release::Leaked(n) => dead.push((t, format!("{n} joint(s) survived the release"))),
            Release::Aborted(why) => dead.push((t, why)),
        }
    }
    std::panic::set_hook(previous);
    dead
}

/// ★ **`B-004`, and the reason it was closed wrong twice: the bracket, not a point.**
///
/// The player action is one thing — *cut a cortex while roped, then let go* — and the tick he
/// lets go on is **not his choice**, it is whenever his thumb comes off the button. The impact
/// frame is `round(hit_stop_cortex_s × simulation_hz)` = 7 ticks = an eighth of a second, so
/// every release tick from the cut to well past the thaw is the same single action.
///
/// Both previous fixes passed a test that released on **one** side of that window:
///
/// | release | before fix 1 | after fix 1 | after fix 2 |
/// |---|---|---|---|
/// | inside the impact frame | clean | clean | **abort, `islands/mod.rs:820`** |
/// | after the impact frame | **abort, `islands/mod.rs:786`** | clean | clean |
///
/// A point test cannot tell those three columns apart. This one sweeps `t = 0..=20` ticks after
/// the hit and asserts that **all** of them survive, so a fix that moves the failure instead of
/// removing it fails here by construction.
#[test]
fn b004_the_rope_may_be_let_go_on_any_tick_across_the_impact_frame() {
    let dead = sweep_releases(None, 0..=20);
    assert!(
        dead.is_empty(),
        "the release is not survivable on every tick of the bracket — {} of 21 ticks died:\n{}",
        dead.len(),
        dead.iter()
            .map(|(t, why)| format!("  t+{t:<2} {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The same sweep for the rope that is **born inside the impact frame** — a hook that bites
/// while the player is frozen and is let go of again before he thaws.
///
/// That joint never reaches avian's joint graph at all (`attach_ropes` spawns it carrying
/// `JointDisabled`), so it is a different row of the matrix than the sweep above and it is the
/// one that is easiest to fix on one side only.
#[test]
fn b004_a_rope_born_inside_the_impact_frame_may_also_be_let_go_of_at_once() {
    let dead = sweep_releases(Some(1), 3..=20);
    assert!(
        dead.is_empty(),
        "a rope hung during the freeze cannot be let go of on every tick — {} died:\n{}",
        dead.len(),
        dead.iter()
            .map(|(t, why)| format!("  t+{t:<2} {why}"))
            .collect::<Vec<_>>()
            .join("\n")
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



// ---------------------------------------------------------------------------
// Q-031 — the strike is a cone, not a cylinder
// ---------------------------------------------------------------------------

/// ★ **The approach angle, as a number: 34 from the front, 0 from behind.**
///
/// Before `Q-031` was answered, `StrikeTuning::reaches` was
/// `ground_m <= reach_m && to.y <= top_m && to.y >= -reach_m` — a cylinder around the titan's
/// axis with no dot product in it (`docs/FINDINGS.md` FIND-012). A player standing in the
/// husk's back took the identical 34 as one standing in his face, `turn_deg_per_s` governed
/// nothing, and the cortex-on-the-nape design had no mechanical meaning at all.
///
/// Both halves of this test use the **same** distance, the **same** height and the **same**
/// husk out of `titan.ron`. The only thing that differs between them is which side of him the
/// player stands on, so the difference in the number is the facing and nothing else.
///
/// Goes red when the cone comes out of `reaches`, and it goes red the *other* way — with the
/// front booking nothing — when the half-angle in `titan.ron` is turned down far enough to miss
/// a player standing directly in front.
#[test]
fn a_strike_from_behind_books_no_damage() {
    /// One husk at the origin looking down −Z, one player at `stand_m`, run until his **first**
    /// blow is over. Returns what the player lost and how many blows were begun.
    fn one_blow(stand_m: Vec3) -> (f32, u32) {
        let mut app = app();
        let start = data(&app).game.player.health;
        // The player first, then the husk: a titan that spends his two spawn ticks aiming at
        // the player's default position is a titan who has already turned before the
        // measurement starts.
        place(&mut app, stand_m, Vec3::ZERO);
        let body = spawn_husk(&mut app, Vec3::ZERO);
        place(&mut app, stand_m, Vec3::ZERO);

        let mut begun = 0u32;
        let mut was_striking = false;
        for _ in 0..400 {
            app.update();
            let striking = app.world().get::<TitanState>(body) == Some(&TitanState::Strike);
            if striking && !was_striking {
                begun += 1;
            }
            was_striking = striking;
            // Stop the moment the first blow is over, or the husk lands his second and third
            // and the measurement becomes "how many strikes fit in 400 ticks".
            if begun >= 1 && !striking {
                break;
            }
        }
        let me = player(&mut app);
        let left = health(&app, me).expect("the player has health");
        (start - left, begun)
    }

    let reach = {
        let app = app();
        data(&app).titan("husk").expect("titan.ron has a husk").attack_range_m
    };
    let damage = {
        let app = app();
        data(&app).titan("husk").expect("titan.ron has a husk").damage
    };
    // Inside `attack_range_m` on both sides, so the husk commits to the blow either way and the
    // cylinder of FIND-012 would book `damage` twice.
    let stand_m = reach - 1.0;

    let (front, front_blows) = one_blow(Vec3::new(0.0, 0.5, -stand_m));
    let (rear, rear_blows) = one_blow(Vec3::new(0.0, 0.5, stand_m));

    assert_eq!(front_blows, 1, "the husk never struck at the player in front of him");
    assert_eq!(rear_blows, 1, "the husk never struck at the player behind him");
    assert_eq!(
        front, damage,
        "a blow into the husk's own face took {front} instead of {damage} — the cone is too \
         narrow to hit a player standing directly in front (titan.ron: husk.strike_half_angle_deg)"
    );
    assert_eq!(
        rear, 0.0,
        "a blow booked {rear} against a player standing {stand_m} m behind the husk. The strike \
         is a cone around the titan's forward vector, not a cylinder around his axis \
         (src/combat/strike.rs::reaches, docs/FINDINGS.md FIND-012)"
    );
    println!(
        "Q-031 husk at {stand_m} m: front {front} · rear {rear} (damage {damage}, reach {reach})"
    );
}

/// The range guard on the new key. **Not `serde(default)`-able** — a kind without a
/// `strike_half_angle_deg` fails to load, which is rule 2; this is the other half, the value
/// being a sane one.
///
/// Under 30° a titan whiffs at a player standing straight in front of him and the attack system
/// reads as broken; at 90° the cone is a half-space, everything in front lands, and the
/// approach angle is back to meaning nothing — which is exactly the hole Q-031 closed.
#[test]
fn every_kind_carries_a_strike_half_angle_in_range() {
    let app = app();
    let d = data(&app);
    assert!(!d.titans.kinds.is_empty(), "titan.ron has no kinds — the loop below is vacuous");
    for (key, kind) in &d.titans.kinds {
        let deg = kind.strike_half_angle_deg;
        assert!(
            (30.0..=90.0).contains(&deg),
            "titan.ron: {key}.strike_half_angle_deg is {deg}, outside [30, 90]"
        );
    }
    println!(
        "Q-031 half-angles: {}",
        d.titans
            .kinds
            .iter()
            .map(|(k, v)| format!("{k} {}", v.strike_half_angle_deg))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}

/// The range guard on `cortex_half_angle_deg`, the twin of the one above it — and it also
/// checks the **derivation** the file claims for its eight values, because a rule that only
/// lives in a comment is a rule that drifts.
///
/// `titan.ron` says every value is `90 + turn_deg_per_s × 0.15 s + 15°`, rounded to the nearest
/// 5, where the 0.15 s is the swing's own press-to-contact time out of `gear.ron`. That is not
/// decoration: **90° would be the literal rear hemisphere and it is unplayable**, because a
/// titan turns toward you while the blade is in the air and a bearing the player pressed at is
/// always smaller than the one the cut lands at. The margin is what he turns, plus a tick and a
/// half.
#[test]
fn every_kind_carries_a_cortex_half_angle_in_range() {
    let app = app();
    let d = data(&app);
    assert!(!d.titans.kinds.is_empty(), "titan.ron has no kinds — the loop below is vacuous");
    let blades = &d.gear.blades;
    let contact_s = (blades.active_from_s + blades.active_to_s) * 0.5;
    for (key, kind) in &d.titans.kinds {
        let deg = kind.cortex_half_angle_deg;
        assert!(
            (45.0..=180.0).contains(&deg),
            "titan.ron: {key}.cortex_half_angle_deg is {deg}, outside [45, 180]"
        );
        // The gate must give back at least what this kind's own turn takes during one swing,
        // or a correctly aimed press lands as a torso graze and reads as a broken hitbox.
        let owed = 90.0 + kind.turn_deg_per_s * contact_s;
        assert!(
            deg >= owed,
            "titan.ron: {key}.cortex_half_angle_deg is {deg}, but he turns {}°/s and the swing \
             takes {contact_s:.2} s from press to contact — a player who presses at 90° is at \
             {owed:.1}° when the blade lands, and this gate refuses him",
            kind.turn_deg_per_s
        );
    }
    println!(
        "F-030 cortex gates ({:.2} s press-to-contact): {}",
        contact_s,
        d.titans
            .kinds
            .iter()
            .map(|(k, v)| format!("{k} {}° (turns {}°/s)", v.cortex_half_angle_deg, v.turn_deg_per_s))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}

// ---------------------------------------------------------------------------
// F-032 — a blade in the body is not a kill. It is a stagger.
// ---------------------------------------------------------------------------

/// The husk, the tick count `titan.ron: husk.stagger_s` asks for, and the one the player's own
/// impact frame gives. Spelled out here so a change to either file shows up as a red test and
/// not as a silently different feel.
fn stagger_and_normal_ticks(app: &App) -> (u32, u32) {
    let d = data(app);
    let husk = d.titan("husk").expect("titan.ron has a husk");
    let hz = d.game.simulation_hz;
    (
        (husk.stagger_s as f64 * hz).round() as u32,
        (d.gear.feel.hit_stop_normal_s as f64 * hz).round() as u32,
    )
}

/// Spawns a real husk 120 m away — out of `aggro_radius_m: 45`, so he stands still while the
/// blade is brought up to its active window — and returns `(root, cortex position)`.
fn a_standing_husk(app: &mut App) -> (Entity, Vec3) {
    app.world_mut().write_message(SpawnTitan {
        kind: "husk".into(),
        pos_x: 0.0,
        pos_y: 0.0,
        pos_z: -120.0,
    });
    ticks(app, 2);
    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<TitanId>>();
        q.iter(app.world()).next().expect("a husk was spawned")
    };
    let cortex = rig_part(app, root, TitanPart::Cortex).expect("the husk has a cortex");
    let at = app
        .world()
        .get::<GlobalTransform>(cortex)
        .expect("the cortex has a GlobalTransform")
        .translation();
    // ⚠️ The player passes through the solid husk for these tests, exactly as
    // `f030_the_cut_kills_the_real_husk` does and for the same measured reason: the body
    // capsule is 2.5 m wide and `gear.ron: reach_m` is 2.00 m, so a 30 m/s pass slams into him
    // and is thrown sideways before the blade is anywhere near. That is a finding about the
    // numbers; these tests are about what a landed cut MEANS.
    let me = player(app);
    app.world_mut()
        .entity_mut(me)
        .insert(CollisionLayers::new(LAYER_PLAYER, LayerMask::NONE));
    (root, at)
}

/// How many consecutive ticks `who` carries a [`HitStop`], sampled at the end of every step.
fn ticks_held_still(app: &mut App, who: Entity, budget: u64) -> u32 {
    let mut held = 0;
    let mut seen = false;
    for _ in 0..budget {
        app.update();
        match app.world().get::<HitStop>(who).is_some() {
            true => {
                seen = true;
                held += 1;
            }
            false if seen => break,
            false => {}
        }
    }
    held
}

/// ★ **`F-032` — the first consequence a non-lethal cut has ever had in this game.**
///
/// The backlog's own words for this feature: *"Kein Kill, sondern Stagger, Bewegungs-Debuff
/// oder Blendung."* Measured on 2026-08-19 with `scripts/f032-swords.txt`: a blade through the
/// husk's chest, his arm and his leg all wrote `TitanHit { zone: Torso }` at 20.67 m/s — and
/// **nothing in the repository read it.** `titan::brain::receive_hits` drops every non-cortex
/// hit on the floor, `render::camera` kicks on `Cortex` only, and the single reaction left was
/// `gear.ron: feel.hit_stop_normal_s` = 2 ticks = 33 ms. The mechanism was there; the meaning
/// was not.
///
/// So: a body cut takes `titan.ron: <kind>.stagger_s` off the titan's **advance**, and no
/// number of them ever kills him.
#[test]
fn f032_a_body_cut_staggers_the_titan_and_never_kills_him() {
    let mut app = app();
    let (expected, normal) = stagger_and_normal_ticks(&app);
    assert!(
        expected > normal,
        "titan.ron: husk.stagger_s resolves to {expected} ticks and the player's own impact \
         frame to {normal} — a stagger that is no longer than the frame it comes with is not a \
         stagger, and a body cut still reads as nothing"
    );
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);

    // 2.4 m below the nape: inside the torso box (`leg_m` 4.80 .. `cortex` 8.90 for a 10 m
    // husk) and far outside a cortex sphere of 0.55 m that the blade's own 0.12 m cannot
    // bridge. `fly_past` with `count: 0` only PLACES the pass — the flying is done below, so
    // that the freeze can be sampled tick by tick.
    let gaps = fly_past(&mut app, cortex, husk_r, 30.0, cortex.y - 2.4, 2, 0);
    let held = ticks_held_still(&mut app, root, 40);

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(!zones.is_empty(), "the pass through the husk's chest cut nothing at all: {gaps:?}");
    assert!(
        !zones.contains(&HitZone::Cortex),
        "a pass 2.4 m below the nape reported {zones:?} — this test no longer measures a BODY cut"
    );

    // 1. The stagger itself. One tick of slack in either direction: the freeze is inserted by
    //    `Commands` in `Spatial` and counted down in `PostStep`, and where exactly the sync
    //    point falls is Bevy's business, not this feature's.
    assert!(
        held.abs_diff(expected) <= 1,
        "the husk carried a HitStop for {held} ticks, titan.ron: husk.stagger_s asks for \
         {expected}. Before F-032 this number was {normal} — the player's own impact frame — \
         because every non-cortex hit fell through to `feel.hit_stop_normal_s`."
    );

    // 2. **And it is never a kill.** Only the cortex kills, by rule
    //    (`titan::brain::receive_hits`), and a stagger that could finish a titan would throw
    //    the whole nape design away.
    ticks(&mut app, 400);
    assert_ne!(
        app.world().get::<TitanState>(root),
        Some(&TitanState::Death),
        "a body cut killed the husk — only the Cortex may do that"
    );
    assert_eq!(titans(&mut app).len(), 1, "the husk is gone after a body cut");
    println!("F-032 body cut: zones {zones:?}, husk held still for {held} ticks (file: {expected})");
}

/// **The control run**, and it is the half that makes the test above worth anything.
///
/// `docs/FINDINGS.md` FIND-103: a test that asks the screen and the function the same question
/// passes when both are wrong. The mirror of that here is a stagger test that would pass with
/// no blade in it at all — so this is the same husk, the same speed, the same tick budget, with
/// the pass moved out of `reach_m`. No hit, no freeze, and the number above therefore came out
/// of the blade and not out of gravity.
#[test]
fn f032_a_pass_out_of_reach_staggers_nothing() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);

    // `fly_past` puts the hand at `cortex.x - REACH_X`; 6 m further out is well past
    // `reach_m` 1.60 m plus the body capsule's 1.25 m.
    let far = cortex + Vec3::new(-6.0, 0.0, 0.0);
    fly_past(&mut app, far, husk_r, 30.0, cortex.y - 2.4, 2, 0);
    let held = ticks_held_still(&mut app, root, 40);

    assert!(hits(&app).is_empty(), "a pass 6 m wide of the husk cut him: {:?}", hits(&app));
    assert_eq!(held, 0, "the husk was staggered by a blade that never touched him");
    println!("F-032 control: no hit, no stagger — the {held} ticks are the blade's, not gravity's");
}

/// **`F-034` may not be paid for by `F-032`.** A cortex hit is a kill, not a stagger.
///
/// Every successful pass reports `[Torso, Cortex]` — every titan is wider than his own neck
/// (`f030_the_cortex_wins_over_the_body_it_hides_in`), so the graze lands first and the stagger
/// with it. **And it lands EARLIER, not in the same tick:** the run of `scripts/f032-swords.txt`
/// on 2026-08-19 measured `Torso` on tick 154 and `Cortex` on tick 157 of the same fall, and the
/// re-derived run of 2026-08-29 (`gravity_m_s2` -20 -> -32) measures 130 and 133. **The absolute
/// ticks are a function of gravity and the THREE between them is not** — that gap is what this
/// test is about, and it has now survived a 60 % change in the constant that moves the pass. If the
/// kill then merely took the LONGER of the two freezes, the corpse would stand still for what
/// was left of `stagger_s` instead of `feel.hit_stop_cortex_s`, and the dissolve of
/// `scripts/f034-hitstop.txt` — a 🟧 row whose evidence is two photographed ticks 0.983 and
/// 0.883 of the way through — would be a different length.
///
/// The two hits are written as messages three ticks apart, exactly as that run produced them.
/// A flown pass cannot be used here: at 30 m/s the graze and the nape land on the **same** tick,
/// both `Commands` inserts race, and the test measures the order of two inserts instead of the
/// rule (measured — the flown version passed identically with `max` and with `assign`).
#[test]
fn f032_a_cortex_hit_assigns_the_kill_frame_over_any_stagger() {
    let mut app = app();
    let d = data(&app);
    let cortex_ticks = (d.gear.feel.hit_stop_cortex_s as f64 * d.game.simulation_hz).round() as u32;
    let (stagger, _) = stagger_and_normal_ticks(&app);
    assert!(
        stagger > cortex_ticks + 3,
        "the husk's stagger ({stagger} ticks) is not more than three ticks longer than the kill \
         frame ({cortex_ticks}) — then `max` and `assign` cannot be told apart three ticks after \
         the graze and this test proves nothing"
    );
    let (root, _) = a_standing_husk(&mut app);
    let husk_id = *app.world().get::<TitanId>(root).expect("the husk has a TitanId");
    let entity = player(&mut app);
    let me = *app.world().get::<PlayerId>(entity).expect("the player has an id");

    let graze = TitanHit { titan: husk_id, by: me, zone: HitZone::Torso, speed_m_s: 20.67 };
    app.world_mut().write_message(graze);
    ticks(&mut app, 3);
    assert!(
        app.world().get::<HitStop>(root).is_some(),
        "the graze's stagger was already over after three ticks — it is {stagger} ticks long, \
         so there is nothing for the kill to have to override"
    );

    app.world_mut().write_message(TitanHit { zone: HitZone::Cortex, ..graze });
    app.update();
    let left = app.world().get::<HitStop>(root).map(|s| s.ticks_left);
    assert!(
        left.is_some_and(|n| n <= cortex_ticks),
        "three ticks after a graze the nape was cut, and the husk carries {left:?} ticks of \
         freeze against a kill frame of {cortex_ticks} (the graze's stagger is {stagger}) — the \
         kill took `max(stagger, kill)` instead of assigning its own frame, so the corpse \
         dissolves on the wrong schedule and F-034's two photographed ticks move"
    );
    println!(
        "F-032/F-034: graze +3 ticks, then the nape — freeze {left:?}, kill frame {cortex_ticks}, \
         stagger {stagger}"
    );
}

/// **The bound that is arithmetic and not taste.** One player's two blades land a hit every
/// `(swing_s + cooldown_s) / 2`; a `stagger_s` at or above that number is a titan who never
/// gets a tick to move in — a permanent lock, and the design's *"Kein Kill"* turned into a kill
/// by another name.
///
/// It reads both numbers out of the two files instead of repeating them, so tuning either one
/// is what moves this test.
#[test]
fn f032_no_kind_can_be_tuned_into_a_permanent_stagger_lock() {
    let app = app();
    let d = data(&app);
    let b = &d.gear.blades;
    let cadence_s = (b.swing_s + b.cooldown_s) / 2.0;
    assert!(!d.titans.kinds.is_empty(), "titan.ron has no kinds — the loop below is vacuous");
    for (key, kind) in &d.titans.kinds {
        assert!(
            kind.stagger_s > 0.0,
            "titan.ron: {key}.stagger_s is {} — a cut into this kind's body means nothing at \
             all, which is the hole F-032 was opened for",
            kind.stagger_s
        );
        assert!(
            kind.stagger_s < cadence_s,
            "titan.ron: {key}.stagger_s is {} against a blade cadence of {cadence_s:.3} s \
             (gear.ron: (swing_s {} + cooldown_s {}) / 2) — one player alone locks this kind in \
             place forever",
            kind.stagger_s,
            b.swing_s,
            b.cooldown_s
        );
    }
    println!(
        "F-032 stagger_s vs a {cadence_s:.3} s cadence: {}",
        d.titans
            .kinds
            .iter()
            .map(|(k, v)| format!("{k} {}", v.stagger_s))
            .collect::<Vec<_>>()
            .join(" · ")
    );
}

/// ★ **`F-032` — an arm hit is its own zone, and it stopped being the torso today.**
///
/// Until 2026-08-19 a titan carried **one** collider, the root capsule (`docs/FINDINGS.md`
/// FIND-109), so `blades::cut::sweep` could only ever answer `Cortex` or the honest catch-all
/// `Torso` — a blade through the chest, the arm and the leg all wrote the identical message.
/// `HitZone::ArmLeft`, `ArmRight`, `LegLeft` and `LegRight` had never been produced by anything
/// in this game.
///
/// The pass below is the f032 chest pass moved **1.75 m to the husk's right**: the hand sits at
/// `x + 0.95` and `gear.ron: reach_m` carries the blade to `x + 2.55`, across an arm box that
/// spans `1.25 .. 1.875` (`w/2 + w/8 ± w/8` for `w = 2.5 m`). The blade is inside the root
/// capsule (radius 1.25 m) for the whole of it, which is exactly why this is a test about
/// PRECEDENCE and not about geometry: the nearest surface is the capsule, and the answer has to
/// be the arm anyway.
///
/// **Red when:** the limb tier is taken out of `sweep`, or the limb colliders out of
/// `titan::rig::build_rig` — the zone is `Torso` again in both cases.
#[test]
fn f032_a_cut_through_the_arm_is_an_arm_hit_and_never_the_torso() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);

    // The rig really did grow the four boxes, and they are where this pass expects them.
    // Without this line a missing collider reads as "the zone table is wrong" further down.
    {
        let mut q = app.world_mut().query::<(&HitZoneOf, &GlobalTransform)>();
        let boxes: Vec<String> = q
            .iter(app.world())
            .map(|(z, t)| format!("{:?} at {:.2?}", z.zone, t.translation()))
            .collect();
        assert_eq!(boxes.len(), 4, "the husk carries {} limb zones: {boxes:?}", boxes.len());
        println!("F-032 the husk's limb zones: {}", boxes.join(" · "));
    }

    // 1.75 m to his right and 2.4 m below the nape: through the right arm, far from the
    // cortex sphere the blade's own 0.12 m cannot bridge.
    let over_the_arm = cortex + Vec3::new(1.75, 0.0, 0.0);
    let gaps = fly_past(&mut app, over_the_arm, husk_r, 30.0, cortex.y - 2.4, 2, 8);

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(!zones.is_empty(), "the pass through the husk's right arm cut nothing: {gaps:?}");
    assert!(
        zones.contains(&HitZone::ArmRight),
        "a blade through the arm box reported {zones:?} — the limb colliders or the limb tier \
         of `sweep` are missing, and every body cut is a `Torso` again (FIND-109)"
    );
    assert!(
        !zones.contains(&HitZone::Cortex),
        "a pass 2.4 m below the nape and 1.75 m to the side reported {zones:?} — a limb hit is \
         never a kill, and this test no longer measures a limb"
    );

    // And the nape rule is untouched: an arm is preparation, not damage.
    ticks(&mut app, 400);
    assert_ne!(
        app.world().get::<TitanState>(root),
        Some(&TitanState::Death),
        "an arm cut killed the husk — only the Cortex may do that"
    );
    println!("F-032 arm pass: zones {zones:?}");
}

/// ★ **`F-032` — and the leg, which is the one that needed the per-zone graze rule.**
///
/// The arm boxes break the surface of the root capsule (`w/2 .. 3w/4` against a radius of
/// `w/2`); the leg boxes do **not** — they span `0 .. w/2` and sit wholly inside it. So a blade
/// flown at a leg meets the silhouette one or more ticks before the leg, and under the old
/// one-bit rule (`super::swing::Swing::has_grazed`) that first `Torso` swallowed everything
/// behind it. `blades::cut::GrazedZones` is what this test is really about: **each zone once**,
/// so the pass books the body it entered through *and* the leg it went on to cut.
///
/// **Red when:** the mask goes back to one bit — the zones are then `[Torso]` and a leg hit
/// cannot be produced at any speed.
#[test]
fn f032_a_cut_through_the_leg_is_a_leg_hit_and_the_body_it_entered_through_is_still_reported() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);

    // Knee height on a 10 m husk (`leg_fraction` 0.48 → the legs run 0 .. 4.80 m) and 1.3 m to
    // his right, so the blade lies across the right leg box (`0 .. w/2` = 0 .. 1.25 m).
    let over_the_leg = Vec3::new(cortex.x + 1.3, cortex.y, cortex.z);
    let gaps = fly_past(&mut app, over_the_leg, husk_r, 30.0, 2.4, 2, 8);

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(!zones.is_empty(), "the pass through the husk's right leg cut nothing: {gaps:?}");
    assert!(
        zones.contains(&HitZone::LegRight),
        "a blade through the leg box reported {zones:?} — the leg sits INSIDE the root capsule,          so without the per-zone graze mask the silhouette answers first and the leg never does"
    );
    assert!(
        !zones.contains(&HitZone::Cortex),
        "a pass at knee height reported {zones:?} — a limb hit is never a kill"
    );
    // Each zone once, however many ticks the blade spends inside it. A mask that never closed
    // would book a hit on every tick of the active window and empty the harness in one pass.
    for zone in [HitZone::Torso, HitZone::LegRight] {
        let n = zones.iter().filter(|z| **z == zone).count();
        assert!(n <= 1, "{zone:?} was booked {n} times in one swing: {zones:?}");
    }

    ticks(&mut app, 400);
    assert_ne!(
        app.world().get::<TitanState>(root),
        Some(&TitanState::Death),
        "a leg cut killed the husk — only the Cortex may do that"
    );
    println!("F-032 leg pass: zones {zones:?}");
}

/// **The control**, and it is the half that keeps the test above from being a rename.
///
/// The same husk, the same speed, the same tick budget, with the pass back on the chest line
/// where `f032_a_body_cut_staggers_the_titan_and_never_kills_him` flies it. The blade spans
/// `x −0.8 .. +0.8` there and touches no limb box at all, so the answer has to stay `Torso` —
/// a limb tier that swallowed the body would pass the arm test and break every graze in the
/// game.
#[test]
fn f032_the_chest_is_still_the_torso_after_the_limbs_got_their_own_zones() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    // 🔴 The fixture's own tie to the file. The blade has to STRADDLE the chest line, or its
    // tip runs into the arm box at `w/2` and this test measures the arm — which is exactly what
    // happened on 2026-08-20 when `reach_m` grew and `REACH_X` did not. See [`REACH_X`].
    assert!(
        (REACH_X - d.gear.blades.reach_m * 0.5).abs() < 1e-6,
        "REACH_X is {REACH_X} but gear.ron: reach_m is {} — the pass is no longer centred on the \
         chest and the zones below describe a different line than the one this test is named for",
        d.gear.blades.reach_m
    );
    let (_, cortex) = a_standing_husk(&mut app);

    let gaps = fly_past(&mut app, cortex, husk_r, 30.0, cortex.y - 2.4, 2, 8);
    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(!zones.is_empty(), "the pass through the husk's chest cut nothing: {gaps:?}");
    assert!(
        zones.iter().all(|z| *z == HitZone::Torso),
        "a pass down the middle of the body reported {zones:?} — `Torso` is the honest name for \
         \"the blade found the body\" and nothing on the chest line is a limb"
    );
    println!("F-032 chest pass: zones {zones:?}");
}

/// **Rule 6, with a number: what the limb refinement costs per landed hit.**
///
/// The sibling of [`f030_the_cost_of_one_thousand_casts`], and the reason it exists is that
/// `F-032`'s cost cannot be seen in a frame budget: [`limb_zone`] runs **only** on a tick where
/// a blade already found a body, and only over that one titan's four boxes. A whole-frame A/B
/// measures the noise of whatever else is on the machine; this measures the work.
///
/// It runs against the **real husk**, so the four boxes are the rig's own and not a fixture's.
#[test]
fn f032_the_cost_of_one_thousand_limb_refinements() {
    let mut app = app();
    let (root, cortex) = a_standing_husk(&mut app);
    let d = data(&app);
    let thickness = d.gear.blades.thickness_m;

    #[derive(Resource, Default)]
    struct Cost(f64, u32);
    app.insert_resource(Cost::default());

    app.world_mut()
        .run_system_once(
            move |children: Query<&Children>,
                  limbs: Query<(&HitZoneOf, &GlobalTransform)>,
                  mut out: ResMut<Cost>| {
                let mut found = 0u32;
                let started = Instant::now();
                for i in 0..1000 {
                    // A different blade every time, so nothing is cached: the hand walks down
                    // the husk's right flank from the shoulder to the knee, which is the band
                    // an arm zone and a leg zone share.
                    let y = cortex.y - 1.0 - i as f32 * 0.006;
                    let a = Vec3::new(cortex.x + 0.9, y, cortex.z);
                    let b = a + Vec3::new(1.6, 0.0, 0.0);
                    let blade = Collider::capsule_endpoints(thickness, a, b);
                    if limb_zone(&children, &limbs, root, &blade, Vec3::new(0.0, -0.5, 0.0))
                        .is_some()
                    {
                        found += 1;
                    }
                }
                out.0 = started.elapsed().as_secs_f64() * 1e6 / 1000.0;
                out.1 = found;
            },
        )
        .expect("the benchmark system runs");

    let cost = app.world().resource::<Cost>();
    assert!(cost.1 > 0, "1000 refinements found no limb at all — the benchmark measured nothing");
    println!(
        "F-032 cost: {:.2} µs per refinement over 1000 calls ({} of them found a limb) [debian]",
        cost.0, cost.1
    );
}

// ---------------------------------------------------------------------------------------
// `F-009` / `F-010` — the i-frames, and the only place in this game where a player's damage
// is refused.
// ---------------------------------------------------------------------------------------
//
// Both backlog rows are written as an *avoidance*, not as a movement:
//
//   F-009  „Flip vermeidet einen Titanengriff, wenn im Fenster ausgeloest."
//   F-010  „Slide vermeidet Stomp-Angriff; geht fliessend in Sprint ueber."
//
// so the acceptance of both is a blow that does not land. This pair of tests is that sentence
// and its control: the same husk, the same distance, the same 400 ticks — once with the window
// open and once without. The control is the point (`FIND-103`): a test that only asserts
// "no damage" passes just as well when the husk never swung.

#[test]
fn f009_a_player_inside_the_i_frame_window_takes_nothing_at_all() {
    let mut app = app();
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);

    // A window that covers the whole run. `F-009`'s real one is 21 ticks and `F-010`'s 18 —
    // what is being tested here is the RULE in `combat::strike::land`, not the length, which is
    // `tests/vector_boost.rs` and `tests/player.rs` respectively.
    let me = player(&mut app);
    app.world_mut()
        .entity_mut(me)
        .insert(defeated_by_titan::shared::Invulnerable { until_tick: u64::MAX });

    let w = watch(&mut app, 400);
    assert!(
        w.strikes >= 3,
        "the husk began only {} strikes in 400 ticks — without swings this test proves nothing \
         at all, which is exactly the shape FIND-103 warns about",
        w.strikes
    );
    assert!(
        w.drops.is_empty(),
        "an invulnerable player still lost health {} time(s): {:?}",
        w.drops.len(),
        w.drops
    );
    assert!(w.downed_at.is_none(), "and he certainly must not go down");
}

#[test]
fn f009_the_control_the_same_husk_without_the_window_lands_three_blows() {
    // The control run, and it is the half that makes the test above mean something. Identical
    // in every line except the component.
    let mut app = app();
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);

    let w = watch(&mut app, 400);
    assert_eq!(
        w.drops.len(),
        3,
        "the same setup WITHOUT i-frames has to land three blows (P5's own claim); it landed \
         {}: {:?}",
        w.drops.len(),
        w.drops
    );
}

#[test]
fn f009_an_expired_window_is_not_a_window() {
    // The deadline is a tick, not a flag, and nothing removes the component when it passes
    // (`shared::Invulnerable`). So a stale `Invulnerable` sitting on a player must behave
    // exactly like no component at all — otherwise every player who ever flipped once would be
    // immortal from his second flip onwards.
    let mut app = app();
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    spawn_husk(&mut app, husk_at);
    place(&mut app, in_his_face(husk_at, 5.0), Vec3::ZERO);

    let me = player(&mut app);
    app.world_mut()
        .entity_mut(me)
        .insert(defeated_by_titan::shared::Invulnerable { until_tick: 5 });

    let w = watch(&mut app, 400);
    assert_eq!(
        w.drops.len(),
        3,
        "a window that ended at tick 5 still stopped {} of 3 blows over 400 ticks",
        3 - w.drops.len()
    );
}

// ---------------------------------------------------------------------------
// F-031 · F-041 · F-044 — the damage formula, the combo and the ground attack
//
// 🔴 **Why most of these drive `TitanHit` directly instead of flying a pass.**
// `combat` reads the message and knows nothing about how a blade is swung — that is the domain
// split `src/blades/mod.rs` argues out, and it is also what makes the claim testable at all:
// a pass books between one and five zones depending on where the limb boxes fall, so "one cut
// at 30 m/s" is not a thing a flight can hold still. The two tests that DO fly
// (`f031_a_body_cut_drains_a_pool_that_nothing_ever_wrote`, `f044_*`) are the ones whose claim
// is about the producer, and they are the control on the rest: without them this block would
// measure a function against itself, which is `docs/FINDINGS.md` FIND-103 exactly.
// ---------------------------------------------------------------------------

fn titan_id(app: &App, who: Entity) -> TitanId {
    *app.world().get::<TitanId>(who).expect("a titan carries a TitanId")
}

fn local_id(app: &mut App) -> PlayerId {
    let p = player(app);
    *app.world().get::<PlayerId>(p).expect("the local player carries a PlayerId")
}

/// The wound pool of a titan. Panics rather than returning `None`: `titan::rig` gives every
/// body a `Health`, and a body without one is a rig bug, not a measurement.
fn pool(app: &App, who: Entity) -> f32 {
    health(app, who).expect("titan::rig hangs Health::full(titan.ron: <kind>.health) on every body")
}

/// Writes one [`TitanHit`] straight onto the bus — the exact message `blades::cut` writes.
fn send_hit(app: &mut App, titan: TitanId, by: PlayerId, zone: HitZone, speed_m_s: f32) {
    app.world_mut().write_message(TitanHit { titan, by, zone, speed_m_s });
}

fn combo(app: &mut App) -> Combo {
    let p = player(app);
    app.world().get::<Combo>(p).copied().unwrap_or(Combo::NONE)
}


/// ★ **`F-031` — a body cut drains a pool that nothing in this game had ever written.**
///
/// `titan::rig` has hung `Health::full(titan.ron: <kind>.health)` on every body since the rig
/// existed, and until 2026-08-25 **no line in `src/` ever touched that component** — the other
/// half of the same hole `gear.ron: blades.damage_per_m_s` sat in (`docs/FINDINGS.md` FIND-075
/// is the same shape for `wear_per_hit`). This is the flight that proves the join: a real cut,
/// out of a real swing, moves a real pool.
///
/// **Red when:** `combat::damage::apply` is taken out of `CombatPlugin`, or its `TitanHit`
/// reader stops matching `hit.titan` — the pool is `max` in both cases.
#[test]
fn f031_a_body_cut_drains_a_pool_that_nothing_ever_wrote() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);
    let full = pool(&app, root);
    assert!(full > 0.0, "titan.ron: husk.health is not a positive number");

    // The same chest pass `f032_a_body_cut_staggers_the_titan_and_never_kills_him` flies: 2.4 m
    // below the nape, inside the torso box and far outside a 0.55 m cortex sphere.
    fly_past(&mut app, cortex, husk_r, 30.0, cortex.y - 2.4, 2, 0);
    ticks(&mut app, 40);

    let zones: Vec<HitZone> = hits(&app).into_iter().map(|(_, h)| h.zone).collect();
    assert!(!zones.is_empty(), "the pass cut nothing at all — this test measures nothing");
    assert!(
        !zones.contains(&HitZone::Cortex),
        "a pass 2.4 m below the nape reported {zones:?} — that is not a BODY cut any more"
    );
    let left = pool(&app, root);
    assert!(
        left < full,
        "the husk took {zones:?} and his wound pool is still {left}/{full}. That is the state \
         this whole feature was opened for: a component with a number in it that nothing writes"
    );
    println!("F-031 pass: zones {zones:?}, pool {full} -> {left}");
}

/// **The control, and it is what makes the test above worth anything** (`FIND-103`).
///
/// The same husk, the same speed, the same tick budget, the pass moved out of `reach_m`. No
/// cut, no drain — so the number above came out of the blade and not out of the app starting.
#[test]
fn f031_a_pass_out_of_reach_drains_nothing() {
    let mut app = app();
    let d = data(&app);
    let husk_r = d.titan("husk").expect("husk").cortex_radius_m;
    let (root, cortex) = a_standing_husk(&mut app);
    let full = pool(&app, root);

    // 6 m to the side: `reach_m` is 2.0 and the body capsule is 1.25, so the blade ends 2.75 m
    // short of the hide.
    fly_past(&mut app, cortex + Vec3::new(6.0, 0.0, 0.0), husk_r, 30.0, cortex.y - 2.4, 2, 0);
    ticks(&mut app, 40);

    assert!(hits(&app).is_empty(), "a pass 6 m wide of the husk still cut him: {:?}", hits(&app));
    assert_eq!(
        pool(&app, root),
        full,
        "the pool moved without a cut — whatever drains it, it is not the blade"
    );
}

/// ★ **`F-031`'s acceptance, in the running game:** *"Ein Schnitt bei doppelter Geschwindigkeit
/// erzeugt mindestens 60 Prozent mehr Schaden."*
///
/// Two identical messages, one at `v` and one at `2v`, against two identical husks. Identical
/// in everything the formula can see — same zone, same multiplier, same kind — so the only free
/// variable is the speed the acceptance sentence names.
///
/// **Red when:** the speed term is dropped out of `damage_of`, or `damage_per_m_s` stops being
/// read; both make the two numbers equal.
#[test]
fn f031_a_cut_at_double_the_speed_takes_at_least_sixty_percent_more() {
    let mut app = app();
    let by = local_id(&mut app);
    let slow_body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let fast_body = spawn_husk(&mut app, Vec3::new(40.0, 0.0, -150.0));
    let (slow_id, fast_id) = (titan_id(&app, slow_body), titan_id(&app, fast_body));
    let full = pool(&app, slow_body);

    send_hit(&mut app, slow_id, by, HitZone::Torso, 15.0);
    send_hit(&mut app, fast_id, by, HitZone::Torso, 30.0);
    ticks(&mut app, 3);

    let slow = full - pool(&app, slow_body);
    let fast = full - pool(&app, fast_body);
    assert!(slow > 0.0, "a 15 m/s chest cut took nothing at all");
    assert!(
        fast >= slow * 1.6,
        "F-031: {slow:.1} at 15 m/s against {fast:.1} at 30 m/s is only {:.0} % more. The row \
         asks for 60, and a formula with no speed term in it reports 0",
        (fast / slow - 1.0) * 100.0
    );
    println!("F-031 acceptance: 15 m/s -> {slow:.1}, 30 m/s -> {fast:.1}");
}

/// 🔴 ★ **The cortex books no wound damage, ever.**
///
/// *A titan dies only from a fast cut into the cortex* — and the moment the lethal zone also
/// carries a damage factor, a tuning pass can move the thing the whole game is built on without
/// anybody noticing. `combat::damage::zone_factor` answers `0.0` for it and this is the guard.
///
/// **Red when:** `HitZone::Cortex` is given any factor at all in `zone_factor`.
#[test]
fn f031_the_cortex_books_no_wound_damage_at_all() {
    let mut app = app();
    let by = local_id(&mut app);
    let body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let id = titan_id(&app, body);
    let full = pool(&app, body);

    send_hit(&mut app, id, by, HitZone::Cortex, 75.0);
    ticks(&mut app, 2);

    assert_eq!(
        pool(&app, body),
        full,
        "a cortex hit at 75 m/s took wound damage. The nape is decided by rule in \
         titan::brain::receive_hits; a formula that can also reach it is a second way to kill"
    );
    // And the rule itself still fired, so this is not measuring a message that went nowhere.
    assert_eq!(
        app.world().get::<TitanState>(body),
        Some(&TitanState::Death),
        "the cortex hit did not kill either — this test is measuring nothing at all"
    );
}

/// ★ **`F-031` — emptying the pool puts the titan on the floor and never kills him.**
///
/// `docs/gameplay/enemies.md`: *"every other hit zone is preparation, not damage"*. This is
/// what "preparation" was made into — a window in which the nape stands still.
///
/// **Red when:** the collapse branch is removed (no long freeze), or when it is allowed to kill
/// (the `TitanState::Death` assertion), or when the pool is not refilled (a titan permanently
/// at zero is a state nobody else in the repository knows how to read).
#[test]
fn f031_an_emptied_pool_floors_the_titan_and_never_kills_him() {
    let mut app = app();
    let d = data(&app);
    let by = local_id(&mut app);
    let body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let id = titan_id(&app, body);
    let full = pool(&app, body);
    let floor = ticks_of(d.gear.damage.collapse_s, d.game.simulation_hz);
    let stagger = ticks_of(d.titan("husk").expect("husk").stagger_s, d.game.simulation_hz);
    assert!(
        floor > stagger,
        "gear.ron: damage.collapse_s is {floor} ticks against a husk's own stagger of \
         {stagger} — a collapse that is no longer than the stagger it comes with is not a \
         collapse, it is a longer stagger nobody notices"
    );

    // One message that is worth the whole pool. Deliberately ONE and not a burst: a burst
    // would also be measuring `hitstop::begin`'s `.max()` and the two would be indivisible.
    let speed = full / (d.gear.blades.damage_per_m_s * d.gear.damage.zone_torso_factor) + 1.0;
    send_hit(&mut app, id, by, HitZone::Torso, speed);
    ticks(&mut app, 2);

    let held = app.world().get::<HitStop>(body).map(|s| s.ticks_left).unwrap_or(0);
    assert!(
        held + 2 >= floor,
        "the husk was floored for {held} ticks and gear.ron: damage.collapse_s asks for {floor}. \
         Before this feature a body cut bought {stagger} — the kind's own stagger"
    );
    assert!(
        app.world().get::<CollapseGuard>(body).is_some(),
        "no refractory guard after a collapse — he can be put straight back on the floor"
    );
    assert_eq!(
        pool(&app, body),
        full,
        "the wound pool was left at zero. It has to steam shut on the collapse, or every \
         further cut collapses him again and the refractory guard is the only thing between \
         the player and a stun lock"
    );

    ticks(&mut app, 400);
    assert_ne!(
        app.world().get::<TitanState>(body),
        Some(&TitanState::Death),
        "emptying the wound pool killed the husk — only the Cortex may do that"
    );
    println!("F-031 collapse: pool {full}, one hit at {speed:.1} m/s, {held} ticks on the floor");
}

/// 🔴 ★ **`F-031` — a titan cannot be kept on the floor.**
///
/// The reason `gear.ron: damage.collapse_refractory_s` exists at all. The arithmetic version of
/// this guarantee — *"collapse_s is shorter than the time it takes to empty the pool again"* —
/// is already **false** for the scuttler at the shipped numbers (60 points of pool against
/// 1.4 x 30 x 2.0 = 84 for one capped chest cut), so the claim is made by a refractory
/// component instead and this is what measures it.
///
/// The flood below is far worse than anything a player can produce: one pool-emptying hit
/// **every tick** for four refractory windows.
///
/// **Red when:** `CollapseGuard` is not inserted, or `advance_guards` removes it early, or the
/// collapse branch stops checking it.
#[test]
fn f031_a_titan_cannot_be_kept_on_the_floor() {
    let mut app = app();
    let d = data(&app);
    let by = local_id(&mut app);
    let body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let id = titan_id(&app, body);
    let full = pool(&app, body);
    let refractory = ticks_of(d.gear.damage.collapse_refractory_s, d.game.simulation_hz);
    let speed = full / (d.gear.blades.damage_per_m_s * d.gear.damage.zone_torso_factor) + 1.0;

    let window = refractory * 4;
    let mut collapses = 0;
    let mut guarded_before = false;
    for _ in 0..window {
        send_hit(&mut app, id, by, HitZone::Torso, speed);
        app.update();
        let guarded = app.world().get::<CollapseGuard>(body).is_some();
        if guarded && !guarded_before {
            collapses += 1;
        }
        guarded_before = guarded;
    }
    let ceiling = window / refractory + 1;
    assert!(
        collapses <= ceiling,
        "{collapses} collapses in {window} ticks against a refractory window of {refractory} — \
         at most {ceiling} are possible if the guard works. A pool-emptying hit was written on \
         every one of those ticks, so this is the stun lock, measured"
    );
    assert!(collapses >= 1, "not one collapse in {window} ticks of pool-emptying hits");
    println!("F-031 lock guard: {collapses} collapses in {window} ticks (ceiling {ceiling})");
}

/// ★ **`F-041` — consecutive hits without ground contact raise a multiplier.**
///
/// The row, verbatim: *"Aufeinanderfolgende Treffer ohne Bodenkontakt erhoehen einen
/// Multiplikator, der bei Treffer oder Landung zurueckgesetzt wird."* This is the first half;
/// the landing is the second half of the same test.
///
/// **Red when:** `combo::bank` is removed, the `is_airborne` gate is inverted, or `combo::decay`
/// stops clearing the chain on the ground.
#[test]
fn f041_hits_in_the_air_raise_the_multiplier_and_a_landing_ends_it() {
    let mut app = app();
    let d = data(&app);
    let by = local_id(&mut app);
    let body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let id = titan_id(&app, body);
    let step = d.gear.damage.combo_step;
    let window = ticks_of(d.gear.damage.combo_window_s, d.game.simulation_hz);

    // 60 m up and no gravity: airborne because he IS, not because a test said so ([`hover`]).
    hover(&mut app, Vec3::new(0.0, 60.0, 0.0));
    send_hit(&mut app, id, by, HitZone::Torso, 20.0);
    ticks(&mut app, 1);
    assert_eq!(combo(&mut app).hits, 1);
    assert_eq!(
        combo(&mut app).multiplier,
        1.0,
        "the FIRST hit of a chain paid a bonus — a chain of one is not a combo"
    );

    send_hit(&mut app, id, by, HitZone::Torso, 20.0);
    ticks(&mut app, 1);
    let after_two = combo(&mut app);
    assert_eq!(after_two.hits, 2);
    assert!(
        (after_two.multiplier - (1.0 + step)).abs() < 1e-5,
        "two hits gave x{:.3}, gear.ron: damage.combo_step {step} asks for x{:.3}",
        after_two.multiplier,
        1.0 + step
    );

    // ★ **The landing, and it is a real one.** Gravity back on, 0.3 m of drop, and
    // `player::integrator::readback` reports `Grounded` on the tick the contact appears —
    // the same tick `combo::decay` reads it in `PostStep`.
    let fell = stand(&mut app, Vec3::new(0.0, 1.2, 0.0));
    // 🔴 The control against the sibling claim: a chain that had simply run out of window would
    // look identical from here. It cannot have — the drop is a fraction of the window.
    assert!(
        fell * 4 < window,
        "the drop took {fell} ticks of a {window}-tick window, so this is measuring the timeout \
         and not the landing"
    );
    assert_eq!(
        combo(&mut app),
        Combo::NONE,
        "the chain survived a landing — 'ohne Bodenkontakt' is the whole rule"
    );
    println!("F-041: two hits -> x{:.2}, landed after {fell} ticks -> {:?}", after_two.multiplier, combo(&mut app));
}

/// ★ **`F-041` — the chain lapses on its own.** `gear.ron: damage.combo_window_s`.
///
/// Without this a player who cut one titan and then flew for a minute would still be carrying
/// a multiplier, and the row's *"bricht korrekt ab"* would mean only "on the ground".
#[test]
fn f041_a_chain_lapses_after_the_window_without_a_hit() {
    let mut app = app();
    let d = data(&app);
    let by = local_id(&mut app);
    let body = spawn_husk(&mut app, Vec3::new(0.0, 0.0, -150.0));
    let id = titan_id(&app, body);
    let window = ticks_of(d.gear.damage.combo_window_s, d.game.simulation_hz);
    assert!(window > 4, "gear.ron: damage.combo_window_s is {window} ticks — nothing to measure");

    hover(&mut app, Vec3::new(0.0, 60.0, 0.0));
    send_hit(&mut app, id, by, HitZone::Torso, 20.0);
    ticks(&mut app, 1);
    assert!(combo(&mut app).is_running());

    // Half the window: still there. Whoever deletes the countdown passes the line below and
    // falls over the one after it, which is the point of measuring both.
    for _ in 0..window / 2 {
        app.update();
    }
    // 🔴 The control, and it is the whole reason this test was red: if he is on the ground the
    // chain is gone for a reason that has nothing to do with the window, and the assert below
    // would be reading `combo::decay`'s landing branch while claiming to read its clock.
    assert_eq!(
        movement(&mut app),
        MovementState::Airborne,
        "the player came down mid-measurement — this run is about the window, not the landing"
    );
    assert!(combo(&mut app).is_running(), "the chain lapsed inside half its own window");

    for _ in 0..window {
        app.update();
    }
    assert_eq!(movement(&mut app), MovementState::Airborne, "the player came down");
    assert_eq!(combo(&mut app), Combo::NONE, "the chain outlived {window} ticks of silence");
}

/// ★ **`F-041` — a titan that connects breaks the chain.** The *"bei Treffer"* of the row, and
/// it is the titan's hit on you, not yours on him.
///
/// **Red when:** the reset is taken out of `combat::strike::land`.
#[test]
fn f041_a_titan_that_connects_breaks_the_chain() {
    let mut app = app();
    let by = local_id(&mut app);
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    let body = spawn_husk(&mut app, husk_at);
    let id = titan_id(&app, body);
    // In his face and clear of the ground: 2.0 m of altitude puts the capsule's feet 1.1 m up,
    // and `to.y` 2.0 is far inside `StrikeTuning::top_m`, so he is airborne AND reachable.
    hover(&mut app, Vec3::new(husk_at.x, 2.0, husk_at.z - 5.0));

    send_hit(&mut app, id, by, HitZone::Torso, 20.0);
    ticks(&mut app, 1);
    send_hit(&mut app, id, by, HitZone::Torso, 20.0);
    ticks(&mut app, 1);
    assert!(combo(&mut app).multiplier > 1.0, "the chain never got going");

    // The husk needs ~90 ticks to wind up and strike.
    let p = player(&mut app);
    let start = health(&app, p).expect("the player carries a Health");
    let mut broken_at = None;
    // 🔴 **The control against the window** (`gear.ron: damage.combo_window_s` = 120 ticks
    // against a wind-up of ~90). A chain that had lapsed on its own two ticks before the blow
    // would leave `Combo::NONE` behind exactly like the blow does, and this test would pass
    // with `combat::strike::land`'s reset deleted. So the chain is read on **every** tick and
    // the state immediately before the blow is what the assert is allowed to rest on.
    let mut before = combo(&mut app);
    for _ in 0..400 {
        assert!(
            before.is_running(),
            "the chain lapsed on its own before the husk ever connected — this run would have \
             measured `combo::decay`'s clock and called it `strike::land`'s reset"
        );
        app.update();
        if health(&app, p).unwrap_or(start) < start {
            broken_at = Some(combo(&mut app));
            break;
        }
        before = combo(&mut app);
    }
    assert_eq!(
        movement(&mut app),
        MovementState::Airborne,
        "the player landed during the wind-up — then the landing broke the chain, not the blow"
    );
    let after = broken_at.expect("the husk never landed a blow in 400 ticks — nothing measured");
    assert_eq!(
        after,
        Combo::NONE,
        "the husk connected and the chain survived it: {after:?}. The row says the multiplier \
         is reset 'bei Treffer'"
    );
}

/// ★ **`F-044` — a ground attack exists where a scratch used to be refused.**
///
/// `game.ron: player.run_speed_m_s` is 6.0 against a `gear.ron: blades.min_speed_m_s` of 8.0,
/// so before this a player on his feet could not touch a titan **at any speed his legs can
/// produce**. The row: *"Grundlegender Bodenangriff fuer Situationen ohne Gas."*
///
/// **Red when:** `blades::cut::ground_attack` is removed — the touch is refused again and no
/// message is written at all.
#[test]
fn f044_a_ground_attack_lands_where_a_scratch_used_to_be_refused() {
    let mut app = app();
    let d = data(&app);
    let floor = d.gear.blades.min_speed_m_s;
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    let body = spawn_husk(&mut app, husk_at);
    let full = pool(&app, body);

    // Standing still, 1.75 m off the husk's axis — the stand-off `scripts/f030-cortex.txt`
    // flies, at chest height instead of at the nape. `place` pins him with no velocity; the
    // `MovementState` is what the feature reads, and it is set here rather than waited for
    // because a settle would also be measuring `player::locomotion`.
    // 🔴 **He has to actually be standing.** `blades::cut` reads `MovementState` in `PostStep`
    // and `player::integrator::readback` writes it in `Integrate` of the same tick, so an
    // inserted `Grounded` is gone before the cut ever sees it — which is why this test was red.
    // [`stand`] drops him the last 0.3 m and waits for the game to report the contact.
    let fell = stand(&mut app, Vec3::new(husk_at.x - 1.75, 1.2, husk_at.z));
    hold_slash(&mut app);
    ticks(&mut app, 120);

    let landed = hits(&app);
    assert_eq!(
        movement(&mut app),
        MovementState::Grounded,
        "the player left his feet during the swing — then this is not a ground attack"
    );
    assert!(
        !landed.is_empty(),
        "a standing player settled in {fell} ticks, swung at a husk 1.75 m away for two seconds \
         and cut nothing"
    );
    for (_, hit) in &landed {
        assert!(
            hit.speed_m_s < floor,
            "a standing player produced {:.2} m/s, over min_speed_m_s {floor} — this test is \
             measuring an airborne cut",
            hit.speed_m_s
        );
        assert_ne!(
            hit.zone,
            HitZone::Cortex,
            "a ground attack reported the CORTEX. titan::brain::receive_hits kills on that by \
             rule and never looks at the speed, so this is a free kill from standing"
        );
    }
    assert!(pool(&app, body) < full, "the ground attack was worth nothing at all");
    println!(
        "F-044: {} ground hits, pool {full} -> {}, zones {:?}",
        landed.len(),
        pool(&app, body),
        landed.iter().map(|(_, h)| h.zone).collect::<Vec<_>>()
    );
}

/// **The control for `F-044`, and it is the whole reason the feature is not just "delete the
/// speed floor"** (`FIND-103`): the identical touch **in the air** still writes nothing.
#[test]
fn f044_the_same_slow_touch_in_the_air_is_still_a_scratch() {
    let mut app = app();
    let husk_at = Vec3::new(0.0, 0.0, -10.0);
    let body = spawn_husk(&mut app, husk_at);
    let full = pool(&app, body);

    // The same spot and the same swing as the test above — [`hover`] instead of [`stand`] is
    // the ONLY difference between the two runs, which is what makes this a control and not a
    // second test.
    hover(&mut app, Vec3::new(husk_at.x - 1.75, 1.2, husk_at.z));
    hold_slash(&mut app);
    ticks(&mut app, 120);
    assert_eq!(movement(&mut app), MovementState::Airborne, "he found the ground after all");

    assert!(
        hits(&app).is_empty(),
        "a slow touch in mid-air wrote {:?} — the speed floor is gone, not conditioned",
        hits(&app)
    );
    assert_eq!(pool(&app, body), full, "the husk lost pool to a mid-air scratch");
}

/// **`F-044`'s acceptance, as a comparison and not a number:** *"Bodenangriff existiert, ist
/// aber niemals die effizientere Wahl."*
///
/// The cheapest cut a flying player can book is one at exactly `min_speed_m_s`; the ground
/// attack has to stay under it, and it does so by construction because it has no speed term at
/// all. Against the shipped file: 5.0 against 11.2.
#[test]
fn f044_a_ground_attack_is_never_the_better_choice() {
    let app = app();
    let d = data(&app);
    let floor = d.gear.blades.min_speed_m_s;
    let ground = damage_of(&d.gear, HitZone::Torso, floor - 0.01, 1.0);
    let cheapest_airborne = damage_of(&d.gear, HitZone::Torso, floor, 1.0);
    assert!(ground > 0.0, "F-044: the ground attack is worth nothing — the row says it exists");
    assert!(
        ground < cheapest_airborne,
        "F-044: a ground attack books {ground:.1} against {cheapest_airborne:.1} for the \
         slowest cut a flying player can make. The row says it may never be the better choice"
    );
    // And the combo cannot rescue it either: a grounded player has no chain at all.
    assert!(
        !defeated_by_titan::combat::combo::is_airborne(MovementState::Grounded),
        "a grounded player can carry a combo — then the ground attack CAN be the better choice"
    );
    println!("F-044: ground {ground:.1} vs cheapest airborne {cheapest_airborne:.1}");
}

