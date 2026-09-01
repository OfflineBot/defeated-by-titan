//! The guard over the seam between the game and the physics engine.
//!
//! Since 2026-08-09 the player is an avian body instead of a `translation += velocity * dt`
//! with a hard-coded ground plane at `y = 0.0`. That swap has six ways of being wrong that
//! **you cannot see in a screenshot**:
//!
//! 1. avian is registered but its five stages sit outside the simulation set — nothing
//!    panics, the order is simply wrong.
//! 2. The capsule is built with `Collider::capsule` instead of the endpoints, and the player
//!    stands half a body height inside the ground. In the picture he just looks short.
//! 3. Gravity or the substep count silently comes from avian's default (−9.81 / 6) instead of
//!    out of `game.ron`.
//! 4. `MaxLinearSpeed` is missing — `F-012` is then a comment, not a clamp, and you find out
//!    the day somebody flings themselves out of the map.
//! 5. A world collider carries no `RigidBody`, and a character controller added later is
//!    blind to exactly that body.
//! 6. The physics hangs on `LocalPlayer` and the second player is a ghost.
//!
//! Each of the six has a test here, and each measures against `assets/data/game.ron`.
//!
//! ## Why these tests drive with `app.update()` and not with `run_schedule(FixedMain)`
//!
//! `tests/multiplayer.rs` advances `Time<Fixed>` by hand and runs `FixedMain` directly. That
//! was right as long as the movement read `Time<Fixed>` itself. **avian does not:**
//! `run_physics_schedule` takes its step size from the *generic* `Time`
//! (`avian3d-0.7.0/src/schedule/mod.rs:238-244`), and that one is only switched over to
//! `Time<Fixed>` by `run_fixed_main_schedule` — which running `FixedMain` by hand skips. The
//! physics then steps with the last wall-clock delta: **measured on `[offlinebot]`, the
//! player fell at 8.28 m/s² instead of 20**, and the number differed from run to run.
//!
//! `TimeUpdateStrategy::FixedTimesteps(1)` advances real time by exactly one timestep per
//! `App::update()` (`bevy_time-0.19.0/src/lib.rs:181-183`), so one `update()` is exactly one
//! simulation step — on every machine. Measured against it: −0.33333 m/s per tick, that is
//! exactly the −20 m/s² from the file.
//!
//! The picture that belongs to these numbers is `docs/images/t007-physics.png`, taken with
//! `scripts/t007-physics.txt`.

use avian3d::prelude::{
    Collider, Gravity, LinearVelocity, LockedAxes, MaxLinearSpeed, RigidBody, SleepingDisabled,
    SubstepCount,
};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::player::integrator::movement_state;
use defeated_by_titan::player::locomotion::{
    DriveTuning, SteerTuning, WinchTuning, air_thrust, rope_drive, rope_steer, rope_winch,
};
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::player::swim::{depth_in, swim_step};
use defeated_by_titan::data::SwimTuning;
use defeated_by_titan::shared::{
    Block, BodyId, Buttons, Cli, Gas, Hook, HookState, IdCounter, Intent, LocalPlayer, MovementState,
    LookOverride, PlayerId, RunAccel, Side, SpatialIndex, Submerged, Velocity, WaterVolume,
};

/// Builds the **real** app, headless, one simulation step per `update()`, on the map named
/// here — **not** on whatever `maps.ron: current` happens to say.
///
/// Not a second, similar one — otherwise the test proves nothing about the game that is
/// actually played (the same argument as in `tests/multiplayer.rs`).
///
/// The map is pinned for the same reason as in `tests/vector_aiming.rs`: nothing in this file
/// is a claim about a district. It is the integrator, the jump height, the run speed, the
/// ground contact — measured against `maps.ron: graybox`, whose ground block has its top edge
/// exactly at y = 0. On 2026-08-12 `current` moved to `ashgate`, whose ground at the origin
/// stands 0.05 m proud, and four tests here went red without a single line of physics having
/// changed. A test about the integrator must not change its answer because a level designer
/// moved a building.
///
/// `GameData` is inserted by `data::DataPlugin` during `add_plugins`, i.e. **before** the
/// first `update()` runs `Startup` — and `world::map::build_map` takes the name out of the
/// resource, not out of the file. That is the seam; it needed nothing new.
fn app_on(map: &str) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.world_mut().resource_mut::<GameData>().maps.current = map.to_string();
    assert!(
        app.world().resource::<GameData>().current_map().is_some(),
        "maps.ron lists no map {map:?} — a typo here builds an empty world and every \
         assertion below turns into `nothing hit`"
    );
    app.update(); // Startup: the city and the local player come into being
    app
}

/// The graybox — the map every number in this file was measured in.
fn app() -> App {
    app_on("graybox")
}

/// Whatever `maps.ron: current` names: the map that actually ships. Only for the tests that
/// make a statement about *the map*, not about a fixture inside one.
fn app_on_current_map() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app
}

/// Runs **exactly** `n` simulation steps.
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

fn at(app: &App, e: Entity) -> Vec3 {
    app.world().get::<Transform>(e).expect("the player has a transform").translation
}

fn state(app: &App, e: Entity) -> MovementState {
    *app.world().get::<MovementState>(e).expect("the player has a movement state")
}

/// Presses a real key — the same input a human triggers, and the same one the `--script`
/// driver uses. Writing into `Intent` directly would not work for the local player anyway:
/// `net::read_input` refills it out of the keyboard on every tick.
fn hold(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
}

fn release(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(key);
}

/// A second player, without the `LocalPlayer` marker — the way a team mate arrives later.
fn second_player(app: &mut App, pos: Vec3) -> Entity {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, pos, false);
    *world.resource_mut::<IdCounter>() = ids;
    let e = e;
    app.update();
    e
}

// ---------------------------------------------------------------------------------------
// 1. The physics really runs, in the right schedule, with the numbers from the file
// ---------------------------------------------------------------------------------------

#[test]
fn t007_the_player_falls_with_the_gravity_from_the_file() {
    // The whole seam in one number. Red when `PhysicsPlugins` is not registered (the player
    // then hangs at 2 m forever, because nothing integrates him), red when the five stages
    // land in the wrong place, and red when gravity comes from somewhere other than the file.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    assert!((at(&app, e).y - 2.0).abs() < 1e-6, "the player starts at 2 m, not at {}", at(&app, e).y);

    ticks(&mut app, 1);
    let v = app.world().get::<LinearVelocity>(e).unwrap().0.y;
    let expected = d.game.gravity_m_s2 / d.game.simulation_hz as f32;
    assert!(
        (v - expected).abs() < 1e-4,
        "after one tick he falls at {v} m/s; at {} m/s^2 and {} Hz it has to be {expected}",
        d.game.gravity_m_s2,
        d.game.simulation_hz
    );
}

#[test]
fn t007_gravity_and_substeps_come_out_of_the_file_not_out_of_avian() {
    // avian's defaults are −9.81 (integrator/mod.rs:156-162) and 6 substeps
    // (solver/schedule.rs:185-191). Both are wrong for this game, and both are wrong SILENTLY.
    let app = app();
    let d = data(&app);

    let g = app.world().resource::<Gravity>().0;
    assert_eq!(
        g,
        Vec3::new(0.0, d.game.gravity_m_s2, 0.0),
        "gravity is {g:?}, game.ron says {} m/s^2 downwards",
        d.game.gravity_m_s2
    );
    assert!(
        g.y < -9.9,
        "−9.81 would be avian's default, not the {} out of the file",
        d.game.gravity_m_s2
    );

    let n = app.world().resource::<SubstepCount>().0;
    assert_eq!(n, d.game.substeps, "SubstepCount is {n}, game.ron says {}", d.game.substeps);
    assert_ne!(n, 6, "6 is avian's default — the value has to come out of the file");
}

// ---------------------------------------------------------------------------------------
// 2. The capsule, and where the player comes to rest
// ---------------------------------------------------------------------------------------

#[test]
fn t007_the_capsule_stands_on_the_origin_and_not_around_it() {
    // `Collider::capsule(r, l)` puts the endpoints at ±l/2 (parry/mod.rs:790-797) — with that
    // the player would sink 0.9 m into the ground, because his origin lies between his feet
    // (docs/conventions.md). The endpoint form spans exactly 0 .. height_m.
    let mut app = app();
    let d = data(&app);
    let s = &d.game.player;
    let e = me(&mut app);

    let collider = app.world().get::<Collider>(e).expect("the player has a collider");
    let capsule = collider
        .shape()
        .as_capsule()
        .expect("the player's collider is a capsule, not something else");

    assert!(
        (capsule.radius - s.radius_m).abs() < 1e-6,
        "capsule radius {} instead of {} from the file",
        capsule.radius,
        s.radius_m
    );
    let bottom = capsule.segment.a.y.min(capsule.segment.b.y) - capsule.radius;
    let top = capsule.segment.a.y.max(capsule.segment.b.y) + capsule.radius;
    assert!(bottom.abs() < 1e-6, "the capsule starts at {bottom} m instead of at the feet (0)");
    assert!(
        (top - s.height_m).abs() < 1e-6,
        "the capsule ends at {top} m instead of at {} m",
        s.height_m
    );
}

#[test]
fn t007_the_player_carries_the_four_components_without_which_the_body_is_wrong() {
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    let w = app.world();

    assert_eq!(w.get::<RigidBody>(e), Some(&RigidBody::Dynamic), "the player is a dynamic body");
    assert!(
        w.get::<SleepingDisabled>(e).is_some(),
        "without SleepingDisabled a player hanging still on a rope falls asleep after 0.5 s \
         (avian3d-0.7.0/src/dynamics/rigid_body/sleeping.rs:103-107)"
    );
    let locked = w.get::<LockedAxes>(e).expect("the player's rotation is locked").to_bits();
    assert_eq!(
        locked,
        LockedAxes::ROTATION_LOCKED.to_bits(),
        "an unlocked capsule tips over and takes the camera child and the axis-aligned hull \
         with it (tests/render.rs::f002_the_camera_rotates_not_the_player)"
    );
    let clamp = w.get::<MaxLinearSpeed>(e).expect("F-012: the clamp is there from day one");
    assert!(
        (clamp.0 - d.game.vector.max_speed_m_s).abs() < 1e-6,
        "MaxLinearSpeed is {} instead of the {} out of game.ron",
        clamp.0,
        d.game.vector.max_speed_m_s
    );
}

#[test]
fn t007_the_player_comes_to_rest_on_the_ground_and_does_not_sink_into_it() {
    // The ground is `maps.ron: blocks[0]` — top edge exactly at y = 0. There is no ground
    // plane left in the code that could catch him, and no `Ground` marker either.
    let mut app = app();
    let e = me(&mut app);
    ticks(&mut app, 180); // 3 s: falls the 2 m in 0.45 s, the rest is settling

    let y = at(&app, e).y;
    assert!(
        y.abs() < 0.01,
        "the player rests at y = {y} m; expected 0 ± 0.01 — he is floating or sinking in"
    );
    assert_eq!(
        state(&app, e),
        MovementState::Grounded,
        "he is standing on the ground and does not know it — ground contact comes out of the \
         collider now, not out of `y <= 0`"
    );
    let v = app.world().get::<LinearVelocity>(e).unwrap().0;
    assert!(v.length() < 1e-3, "he is standing still and yet moves at {v:?}");
}

// ---------------------------------------------------------------------------------------
// 3. Walking and jumping — through avian, driven by real keys
// ---------------------------------------------------------------------------------------

#[test]
fn t007_walking_reaches_the_run_speed_from_the_file() {
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 120); // land first

    let start = at(&app, e);
    hold(&mut app, KeyCode::KeyW);
    ticks(&mut app, 60); // one second
    release(&mut app, KeyCode::KeyW);

    let end = at(&app, e);
    let walked = (end - start).xz().length();
    let expected = d.game.player.run_speed_m_s;
    assert!(
        (walked - expected).abs() < 0.25,
        "walked {walked:.3} m in one second, expected {expected} m (game.ron: run_speed_m_s)"
    );
    // `yaw = 0` means looking along −Z (docs/conventions.md). Walking sideways would be a
    // rotation error that nobody sees from a distance.
    assert!(end.z < start.z - 5.0, "he walked to {end:?} instead of along −Z");
    assert!(end.y.abs() < 0.01, "he walked at y = {} instead of on the ground", end.y);

    // And the velocity the rest of the game reads is the one avian computed.
    let physics = app.world().get::<LinearVelocity>(e).unwrap().0;
    let mirror = app.world().get::<Velocity>(e).unwrap().0;
    assert!(
        (physics - mirror).length() < 1e-6,
        "Velocity ({mirror:?}) is not the readback of LinearVelocity ({physics:?})"
    );
}

#[test]
fn t007_a_jump_reaches_exactly_the_height_the_file_allows() {
    // `jump_speed_m_s` is a number you can compute a height from in your head: v²/2g. If the
    // game does not deliver that height, the number in `game.ron` is not a promise but a
    // suggestion — and every balancing decision built on it is guesswork.
    //
    // **This test holds the button the whole time on purpose.** Before the two guards in
    // `player::integrator::readback` (contact only from `penetration > -collision_margin_m`)
    // and `player::locomotion::ground_locomotion` (`velocity.y <= 0.0`), holding was worth
    // 1.2642 m instead of 1.0562 m — 20 % of free jump height for pressing longer.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 120);
    assert_eq!(state(&app, e), MovementState::Grounded, "he has to be standing before the jump");

    let v0 = d.game.player.jump_speed_m_s;
    let apex_theory = v0 * v0 / (2.0 * -d.game.gravity_m_s2);

    hold(&mut app, KeyCode::Space);
    ticks(&mut app, 12); // 0.2 s
    // 🔴 **RE-DERIVED 2026-08-27, and derived from the file rather than typed.** This line read
    // a literal `0.90` — `6.5·0.2 − 10·0.04` at `jump_speed_m_s` 6.5 and `gravity_m_s2` −20.
    // Both moved on 2026-08-27 (8.2 against −32) and the same formula now gives
    // `8.2·0.2 − 16·0.04` = **1.000 m**; the run reads 0.9977 (discrete Euler over 12 ticks).
    // The number is computed here so the next constant change moves it by itself — a literal
    // is what made this test a stale assert instead of a guard (`docs/NEXT.md` §3G rule 1:
    // re-derive, do not widen — the band stays ±0.05 m).
    let rise_02 = v0 * 0.2 - 0.5 * -d.game.gravity_m_s2 * 0.2 * 0.2;
    let after_02 = at(&app, e).y;
    assert!(
        (after_02 - rise_02).abs() < 0.05,
        "0.2 s after the jump he is at {after_02} m instead of {rise_02:.4} (v0 = {v0}, g = {})",
        d.game.gravity_m_s2
    );
    assert_eq!(state(&app, e), MovementState::Airborne, "in the air he is not grounded");

    let mut apex = after_02;
    for _ in 0..60 {
        ticks(&mut app, 1);
        apex = apex.max(at(&app, e).y);
    }
    release(&mut app, KeyCode::Space);
    assert!(
        (apex - apex_theory).abs() < 0.03,
        "apex {apex:.4} m against the {apex_theory:.4} m that v0²/2g allows — holding the \
         button must not be worth extra height"
    );

    ticks(&mut app, 90); // he is long back down
    let landed = at(&app, e).y;
    assert!(landed.abs() < 0.01, "after landing he is at {landed} m instead of at 0");
    assert_eq!(state(&app, e), MovementState::Grounded);
}

// ---------------------------------------------------------------------------------------
// 3b. F-014 — the ground stopped deleting horizontal momentum
//
// The user's verdict after his first session was "seile ohne boost bringen gar nichts", and
// the largest single cause was here: `ground_locomotion` **assigned** `velocity.x/z` on every
// grounded tick. Measured [cachy], 27 headless runs: released at the bottom of a pendulum arc
// the player lands at 39.717 m/s and is at 0.000 m/s two ticks later with no key held, at
// 6.000 m/s with W held. A clean swing that covers 48.02 m in 2.83 s — 2.83× running speed —
// ends at walking pace the moment a toe touches the ground.
//
// **All six tests below drive along +Z on purpose.** Every explicitly placed block of
// `maps.ron: graybox` sits at negative Z, and `layout.clear_radius_m` is 24 m, so a slide of
// up to ~22 m along +Z from the origin meets nothing. A test that measures deceleration must
// not accidentally be measuring a wall.
// ---------------------------------------------------------------------------------------

/// Puts the landed local player at `speed` m/s along +Z, the way a swing hands him over.
fn launch_on_the_ground(app: &mut App, e: Entity, speed_m_s: f32) {
    ticks(app, 120); // land first — `MovementState::Grounded` comes out of real contacts
    assert_eq!(state(app, e), MovementState::Grounded, "he has to be standing before the launch");
    app.world_mut().get_mut::<LinearVelocity>(e).unwrap().0 = Vec3::new(0.0, 0.0, speed_m_s);
}

fn ground_speed(app: &App, e: Entity) -> f32 {
    app.world().get::<LinearVelocity>(e).unwrap().0.xz().length()
}

#[test]
fn f014_a_landing_at_speed_keeps_its_momentum() {
    // THE test. Before F-014 this reported 0.0000 m/s: the assignment with an empty intent.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);

    ticks(&mut app, 1);
    let v = ground_speed(&app, e);
    // One tick of ground deceleration is `-gravity_m_s2 / simulation_hz` = 0.333 m/s.
    let expected = 30.0 + d.game.gravity_m_s2 / d.game.simulation_hz as f32;
    assert!(
        (v - expected).abs() < 0.05,
        "he arrived at 30 m/s and one tick later he is at {v:.4} m/s; expected {expected:.4} \
         — the ground is deleting momentum instead of chaining it (F-014)"
    );
}

#[test]
fn f014_a_held_key_does_not_pull_a_landing_back_to_the_run_speed() {
    // The A/B whose only difference is the held key: before F-014 the no-key case reported
    // 0.0000 and this one reported exactly `run_speed_m_s`, because W is worth an assignment
    // of 6 m/s no matter what the player brought with him.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);

    // W is forward, that is −Z; the momentum runs along +Z. Whether the key agrees with the
    // direction must not decide whether the speed survives.
    hold(&mut app, KeyCode::KeyW);
    ticks(&mut app, 1);
    release(&mut app, KeyCode::KeyW);

    let v = ground_speed(&app, e);
    assert!(
        v > 29.0,
        "with W held he is at {v:.4} m/s one tick after arriving at 30 — {} m/s would be the \
         old assignment (game.ron: run_speed_m_s)",
        d.game.player.run_speed_m_s
    );
}

#[test]
fn f014_a_slide_comes_to_a_full_stop_without_input() {
    // Momentum that never ends is not a chain, it is ice. The deceleration IS `-gravity_m_s2`
    // (`player::locomotion::ground_step`), so a `v0` slide needs `v0/decel` seconds.
    //
    // 🔴 **RE-DERIVED 2026-08-27.** Every number in this test used to be a literal written for
    // `gravity_m_s2` = −20: *"20/20 = 1.00 s"*, *"halfway 10 m/s"*, *"two ticks and he is still
    // over 19"*. At −32 the run reads 18.9333 / 4.0 / 0.0 and the first two literals went red —
    // the behaviour never changed, the constant did. They are computed from `decel` now, so the
    // next constant change moves them by itself (`docs/NEXT.md` §3G rule 1: re-derive, do not
    // widen — every band below is the one it always was).
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    let decel = -d.game.gravity_m_s2;
    let hz = d.game.simulation_hz as f32;
    let v0 = 20.0f32;
    launch_on_the_ground(&mut app, e, v0);

    // It must not be instant — that is exactly the bug this feature is about. Two ticks of
    // `decel` and not one metre more: at −20 that was 19.33 m/s, at −32 it is 18.93.
    ticks(&mut app, 2);
    let after_2 = v0 - decel * 2.0 / hz;
    assert!(
        ground_speed(&app, e) > after_2 - 0.1,
        "two ticks after arriving at {v0} m/s he is already at {:.4} m/s, and {decel} m/s² of \
         deceleration owes {after_2:.4}",
        ground_speed(&app, e)
    );

    // 🔴 **AND THE RAMP HAS AN END THAT IS NOT ZERO, which is what a literal hid for a year.**
    // `ground_step` returns the desired velocity *directly* below `run_speed_m_s`, so the linear
    // brake runs only down to `run_speed_m_s + decel/hz` and the last 6.5 m/s go in one tick.
    // At −20 that snap sat at 0.683 s, i.e. **after** the 0.5 s this line used to sample, and
    // `20 − 20·0.5 = 10` was a fair mid-ramp reading. At −32 the snap is at 0.421 s and the same
    // sample reads **0.0000** — the ramp was over. The sample time is derived from the snap now,
    // and the guard below is what says so out loud instead of letting the next constant change
    // measure a standstill and call it a ramp.
    let snap_at = d.game.player.run_speed_m_s + decel / hz;
    let sample_s = 0.25;
    let due = v0 - decel * sample_s;
    assert!(
        due > snap_at,
        "at {decel} m/s² the slide is already under the {snap_at:.4} m/s snap after \
         {sample_s} s, so this line measures a standstill and not a ramp — move the sample \
         earlier"
    );
    ticks(&mut app, 13); // 15 ticks = 0.25 s in total
    let halfway = ground_speed(&app, e);
    assert!(
        (halfway - due).abs() < 1.0,
        "{sample_s} s into the slide he is at {halfway:.4} m/s; at {decel} m/s² it has to be \
         {due:.4}"
    );

    // And the stop is due at `(v0 − snap_at)/decel` — 0.421 s at −32, 0.683 s at −20. The wait
    // below leaves at least half a second of slack over it, exactly as it always did.
    let stop_s = (v0 - snap_at) / decel;
    assert!(stop_s < 0.75, "a {stop_s:.3} s stop does not fit in the 1.25 s this test waits");
    ticks(&mut app, 60);
    let end = ground_speed(&app, e);
    assert!(
        end < 0.01,
        "1.25 s after a {v0} m/s slide he still moves at {end:.4} m/s — at {decel} m/s² the \
         stop is due after {stop_s:.3} s"
    );
}

#[test]
fn f014_from_rest_the_run_speed_is_still_reached_at_once() {
    // The floor must not have cost the ground its snap. From a standstill, one tick of W is
    // still the whole run speed — no ramp, no acceleration number, exactly as before.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 120);
    assert!(ground_speed(&app, e) < 1e-3, "he has to be standing still before this");

    hold(&mut app, KeyCode::KeyW);
    ticks(&mut app, 1);
    release(&mut app, KeyCode::KeyW);

    let v = ground_speed(&app, e);
    assert!(
        (v - d.game.player.run_speed_m_s).abs() < 1e-3,
        "one tick of W from a standstill gives {v:.4} m/s instead of {} (game.ron: \
         run_speed_m_s)",
        d.game.player.run_speed_m_s
    );
}

#[test]
fn f014_below_the_run_speed_the_ground_is_still_an_assignment() {
    // **A deliberate decision, written down as a test.** Arriving at 2 m/s and holding W
    // *does* become 6 m/s in one tick. Below `run_speed_m_s` the rule is unchanged: the
    // ground is a target, ground combat stays crisp, and there is no second acceleration
    // number to tune. The floor only ever reaches upward from the run speed, never below it —
    // so this is not "the landing conjures speed", it is "walking is walking".
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 2.0);

    hold(&mut app, KeyCode::KeyW);
    ticks(&mut app, 1);
    release(&mut app, KeyCode::KeyW);

    let v = ground_speed(&app, e);
    assert!(
        (v - d.game.player.run_speed_m_s).abs() < 1e-3,
        "arriving at 2 m/s and holding W gives {v:.4} m/s; below the run speed the ground is \
         still an assignment and that is {}",
        d.game.player.run_speed_m_s
    );
}

#[test]
fn f014_the_input_still_steers_the_carried_momentum() {
    // Carrying momentum must not mean being a passenger. A player who arrives fast and holds
    // A curves; he does not keep the old vector forever, and he does not stop dead either.
    //
    // ⚠️ **Since F-006 (2026-08-12) this number is the AIR CONTROL's, not the legs'.** Above
    // `run_speed_m_s + (-gravity_m_s2)/simulation_hz` `ground_locomotion` passes `Vec2::ZERO`
    // as `desired`, so `ground_step` only brakes; what bends the line is
    // `locomotion::air_control` at `game.ron: player.air_accel_m_s2`. Measured `[offlinebot]`:
    // the legs used to turn this by **22.44°**, the air control turns it by **11.22°**, and
    // with an empty tank by **5.67°**
    // (`f006_above_the_threshold_the_legs_stop_steering_and_the_air_takes_over` is the other
    // half of this pair). **The margin over the 10° below is 1.22°** — whoever lowers
    // `air_accel_m_s2` makes this test go red, and that is the guard working, not a flaky test
    // (`docs/FINDINGS.md` FIND-051).
    // ⚠️ **That sentence used to say `-gravity_m_s2 / 2`, and it was true when the two were 10
    // and −20.** `air_accel_m_s2` has been its own key for a while and gravity moved to −32 on
    // 2026-08-27 without it; the derivation was carried here by hand and nobody noticed.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    let v0 = 30.0f32;
    launch_on_the_ground(&mut app, e, v0);

    hold(&mut app, KeyCode::KeyA); // yaw = 0 ⇒ A is −X, perpendicular to the +Z momentum
    ticks(&mut app, 30); // 0.5 s
    release(&mut app, KeyCode::KeyA);

    let v = app.world().get::<LinearVelocity>(e).unwrap().0.xz();
    let turned_deg = v.x.atan2(v.y).to_degrees().abs(); // Vec3::xz() ⇒ (x, z)
    assert!(
        turned_deg > 10.0,
        "half a second of A turned the momentum by {turned_deg:.2}° — the input does not steer \
         at speed, the player is a passenger"
    );
    assert!(
        v.y > 5.0,
        "after half a second of A he moves at {:?}; A is meant to curve the +Z momentum, not \
         to brake it into nothing",
        v
    );
    // And the turn cost him only the ordinary deceleration: `v0 − (-gravity_m_s2)·0.5`.
    // 🔴 **RE-DERIVED 2026-08-27** — the literal `20.0` here was `30 − 20·0.5` and the file now
    // says −32, i.e. 14.0 m/s. The run reads 14.4922, the band is the ±1.5 it always was, and
    // the claim (*"turning is not a second brake"*) is unchanged.
    let due = v0 - -d.game.gravity_m_s2 * 0.5;
    let speed = v.length();
    assert!(
        (speed - due).abs() < 1.5,
        "steering left him at {speed:.4} m/s instead of the {due:.4} m/s the plain deceleration \
         allows — turning must not be a second brake"
    );
}

// ---------------------------------------------------------------------------------------
// 3bb. F-006 Swerve — WASD is the air control, and touching the ground does not take it away
//
// The user, 2026-08-12, after playing it (`docs/NEXT.md` §1, quoted verbatim there):
// *„wenn man w drückt und verbunden ist bekommt man schon movement! bei a und d movement zur
// seite. mit s »spannt« man nur das seil! … das a d sorgt dafür dass man nicht immer direkt
// zum seil gezogen wird!"* and *„nur weil man den boden berührt ist man nicht direkt aus
// flugmodus raus, erst wenn man langsam genug ist läuft man wieder"* and *„ohne gas kann man
// immernoch w a d nutzen um etwas movement aufzubauen (aber hälfte ca)"*.
//
// That is `F-006` out of `docs/backlog/gameplay.ron` word for word — *"Richtungseingabe
// waehrend des Einzugs moduliert die Flugbahn seitlich, nach oben und unten. **Kein binaeres
// Ziel-Anfliegen**"*, acceptance *"Vier Swerve-Richtungen aendern die Bahn messbar ohne Haken
// zu loesen"*. Until 2026-08-12 `ground_locomotion` read the input **only** while
// `MovementState::Grounded`, so airborne WASD did nothing whatever.
//
// The threshold is not a new one: `run_speed_m_s + (-gravity_m_s2)/simulation_hz` = 6.3333 is
// the same number `movement_state` splits `Grounded` from `Tethered` on (`FIND-037`).
// ---------------------------------------------------------------------------------------

fn run_accel(app: &App, e: Entity) -> Vec3 {
    app.world().get::<RunAccel>(e).expect("a player carries the run drive").0
}

/// Bodies 200 m over the city, 20 m apart — air control measured with no ground, no rope and
/// no contact anywhere in the picture.
///
/// **Second** players, because they carry no `LocalPlayer`: `net::deliver_intents` only writes
/// an `Intent` for a `PlayerId` that has mail, and nobody posts mail for them, so what
/// [`fly`] writes survives every tick (`src/net/mod.rs`).
///
/// All of them are spawned **before** any intent is set, because `second_player` runs a tick of
/// its own — spawning them one at a time gave the first flyer three ticks of thrust more than
/// the last and measured 10.5002 m/s where 10 was due.
fn flyers(app: &mut App, n: usize) -> Vec<Entity> {
    (0..n).map(|i| second_player(app, Vec3::new(i as f32 * 20.0, 200.0, 0.0))).collect()
}

fn fly(app: &mut App, e: Entity, intent: Intent) {
    *app.world_mut().get_mut::<Intent>(e).expect("a player carries an intent") = intent;
}

fn horizontal(app: &App, e: Entity) -> Vec2 {
    app.world().get::<LinearVelocity>(e).unwrap().0.xz()
}

#[test]
fn f006_w_flies_where_you_look_a_and_d_go_sideways_and_s_never_thrusts() {
    // Four bodies, one second of held key, one app. Before F-006 all four reported
    // **0.0000 m/s** of horizontal speed: `ground_locomotion` skipped every one of them.
    let mut app = app();
    let d = data(&app);
    // The magnitude is derived, not typed — and 🔴 **since 2026-08-27 it is derived from the
    // RIGHT key.** It read `-gravity_m_s2 / 2` under the sentence *"the air control is half of
    // gravity, so WASD alone can never hold you up"*, which was true only while the two were
    // 10 and −20. `player.air_accel_m_s2` has been its own key in `game.ron` for a while
    // (`src/player/locomotion.rs:1037` reads it and nothing else), and when the user moved
    // gravity to −32 this line started demanding 16 m/s from a 10 m/s² accelerator: measured
    // 10.0002, i.e. the air control working exactly as its key says.
    // ⚠️ **The sentence it stood for is now `air_accel_m_s2 < -gravity_m_s2`, and that is
    // `tests/data.rs`' business, not this test's** — here the claim is only that a held key in
    // the air is worth its own key's acceleration.
    let a = d.game.player.air_accel_m_s2;

    let bodies = flyers(&mut app, 4);
    let (forward, sideways, backwards, idle) = (bodies[0], bodies[1], bodies[2], bodies[3]);
    fly(&mut app, forward, Intent { move_y: 1.0, ..default() });
    // Looking 60° down and holding D: the sideways thrust has to stay HORIZONTAL, or every
    // strafe in a fast swing — where you are looking at the street — pushes you into it.
    fly(&mut app, sideways, Intent { move_x: 1.0, pitch: -60.0_f32.to_radians(), ..default() });
    fly(&mut app, backwards, Intent { move_y: -1.0, ..default() });
    fly(&mut app, idle, Intent::default());

    let fall_before = app.world().get::<LinearVelocity>(sideways).unwrap().0.y;
    ticks(&mut app, 60); // one second
    let fall_after = app.world().get::<LinearVelocity>(sideways).unwrap().0.y;

    // W: `yaw = 0` looks along −Z (`docs/conventions.md`), so one second of W is `a` m/s of −Z.
    let w = horizontal(&app, forward);
    assert!(
        (w.length() - a).abs() < 0.5 && w.y < -1.0,
        "one second of W in the air gave {w:?} ({:.4} m/s); {a} m/s along −Z is what an air \
         control of {a} m/s² buys — airborne input is being thrown away (F-006)",
        w.length()
    );

    // A/D: the same magnitude, +X for D, and nothing added to the fall.
    let s = horizontal(&app, sideways);
    assert!(
        (s.length() - a).abs() < 0.5 && s.x > 1.0,
        "one second of D in the air gave {s:?} — `A`/`D` is the steering the rope never had \
         (\"das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird\")"
    );
    let fallen = fall_after - fall_before;
    let gravity_only = d.game.gravity_m_s2; // one second of it
    assert!(
        (fallen - gravity_only).abs() < 0.5,
        "a second of D while looking 60° down changed the vertical velocity by {fallen:.4} m/s \
         instead of the {gravity_only} m/s gravity alone owes — the strafe is tilting with the \
         pitch instead of staying horizontal"
    );

    // S: the rope's tension key, and **not** a thrust.
    let b = horizontal(&app, backwards);
    assert!(
        b.length() < 0.01,
        "one second of S in the air gave {b:?} — S tensions the rope (\"mit s »spannt« man nur \
         das seil!\") and must never push the body"
    );
    assert!(horizontal(&app, idle).length() < 0.01, "a body with no key held thrust anyway");
}

#[test]
fn f006_an_empty_tank_leaves_half_the_air_control() {
    // *„ohne gas kann man immernoch w a d nutzen um etwas movement aufzubauen (aber hälfte
    // ca)"* — so the air control is **not gated** on gas, it is halved without it. That is
    // what stops an empty tank from being the dead end it is today.
    let mut app = app();
    let bodies = flyers(&mut app, 2);
    let (full, dry) = (bodies[0], bodies[1]);
    fly(&mut app, full, Intent { move_y: 1.0, ..default() });
    fly(&mut app, dry, Intent { move_y: 1.0, ..default() });

    // Re-emptied every tick: `vector.gas_regen_per_s` would put the tank back after
    // `gas_regen_delay_s`, and then this would be measuring a full tank again.
    for _ in 0..60 {
        app.world_mut().get_mut::<Gas>(dry).unwrap().current = 0.0;
        ticks(&mut app, 1);
    }

    let with_gas = horizontal(&app, full).length();
    let without = horizontal(&app, dry).length();
    assert!(with_gas > 1.0, "the full tank produced {with_gas:.4} m/s — nothing to halve");
    assert!(
        (without - with_gas * 0.5).abs() < 0.5,
        "an empty tank gave {without:.4} m/s against the {with_gas:.4} m/s of a full one; the \
         user's word is \"hälfte ca\" - half, not zero and not all of it"
    );
}

#[test]
fn f006_touching_the_ground_at_speed_does_not_end_the_air_control() {
    // *„nur weil man den boden berührt ist man nicht direkt aus flugmodus raus, erst wenn man
    // langsam genug ist läuft man wieder"* — a **speed** threshold, not a contact test. His
    // feet are on the floor in both halves of this test; only the speed differs.
    let mut app = app();
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);

    hold(&mut app, KeyCode::KeyD);
    ticks(&mut app, 2);
    assert_eq!(
        state(&app, e),
        MovementState::Grounded,
        "his feet have to be down for this test to say anything"
    );
    let skidding = run_accel(&app, e);
    assert!(
        skidding.length() > 1.0,
        "skidding across the ground at 30 m/s his air control is {skidding:?} — one toe on the \
         floor took the whole flight mode away (F-006, docs/NEXT.md §1b)"
    );

    // And the other end of the same rule: once he is slow, the legs have him back and the air
    // control is silent — or walking would carry a thrust on top of its assignment.
    release(&mut app, KeyCode::KeyD);
    ticks(&mut app, 150); // 30 m/s at 20 m/s² is standing after 1.5 s
    hold(&mut app, KeyCode::KeyD);
    ticks(&mut app, 2);
    let walking = run_accel(&app, e);
    assert_eq!(
        walking,
        Vec3::ZERO,
        "a walking player carries an air control of {walking:?} — below the run speed the \
         ground is an assignment and nothing may push on top of it"
    );
}

#[test]
fn f006_the_swerve_bends_the_flight_without_letting_go_of_the_hook() {
    // **`F-006`'s own acceptance sentence**, out of `docs/backlog/gameplay.ron`: *"Vier
    // Swerve-Richtungen aendern die Bahn messbar **ohne Haken zu loesen**"* — and the user's
    // reason for it, *„das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird"*.
    // A rope that can only drag you at your anchor is a leash; one you can lean out of is a
    // swing.
    let mut app = app();
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);
    anchor_the_left_hook(&mut app, e);
    ticks(&mut app, 1);
    assert_eq!(state(&app, e), MovementState::Tethered, "the rope has to be carrying him");

    hold(&mut app, KeyCode::KeyD); // yaw = 0 ⇒ D is +X, across the +Z the rope handed him
    ticks(&mut app, 30);
    release(&mut app, KeyCode::KeyD);

    let v = horizontal(&app, e);
    assert!(
        v.x > 1.0,
        "half a second of D on the rope moved him {:.4} m/s sideways — the rope is still the \
         only thing steering and the player is a passenger on it (F-006)",
        v.x
    );
    let hook = app.world().get::<Hook>(e).expect("the player carries a Hook");
    assert!(
        matches!(hook.arm(Side::Left).state, HookState::Anchored { .. }),
        "the swerve let go of the hook — F-006 is a course change WITHOUT releasing it"
    );
}

#[test]
fn f006_above_the_threshold_the_legs_stop_steering_and_the_air_takes_over() {
    // The other half of `f014_the_input_still_steers_the_carried_momentum`, and the two are a
    // **pair**: with a tank the steering is there (that test), with an empty one it is half
    // and the legs add nothing of their own (this one). Before F-006 the legs steered a
    // 30 m/s slide by **22.44°** in half a second whatever the tank said, because
    // `ground_step`'s direction term does not know about gas.
    let mut app = app();
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);

    hold(&mut app, KeyCode::KeyA); // yaw = 0 ⇒ A is −X, perpendicular to the +Z momentum
    for _ in 0..30 {
        app.world_mut().get_mut::<Gas>(e).unwrap().current = 0.0;
        ticks(&mut app, 1);
    }
    release(&mut app, KeyCode::KeyA);

    let v = horizontal(&app, e);
    let turned_deg = v.x.atan2(v.y).to_degrees().abs();
    assert!(
        turned_deg < 10.0,
        "half a second of A on an empty tank turned a 30 m/s slide by {turned_deg:.2}° — that \
         is the legs steering, and above the run speed the legs are not driving any more"
    );
    assert!(
        turned_deg > 2.0,
        "{turned_deg:.2}° is no steering at all — half of the air control still has to bend \
         the line, or an empty tank is the dead end it was before"
    );
}

// ---------------------------------------------------------------------------------------
// 3c. F-004 — `MovementState::Tethered`: whose velocity is it while a hook holds?
//
// `MovementState::Tethered` was declared in `src/shared/state.rs` on the first day and written
// by NOBODY until 2026-08-10. What that cost is one tick, and it is measurable to the digit:
// in `scripts/f-001-hooks.txt` the reel starts at t=199 and hands the player over to t=200 at
// v = (0.000, 17.143, −22.138) — 28 m/s straight along the rope, which is
// `vector.reel_speed_m_s` exactly. On that one tick `MovementState` still reads `Grounded`,
// because contact data is one step old (the narrow phase runs before the solver,
// `player::integrator`), and so `ground_locomotion` wrote his horizontal velocity while the
// rope was carrying him.
//
// It was worth the whole headline number of this project. Deleting the −22.138 left
// (0, 17.143, 0), which is almost pure TANGENT to the rope — and `shared::rope::rope_reel_in`
// multiplies the tangent by `length_prev/length_new`, so the reel whipped it up to the 75 m/s
// clamp and ACT 1 reported 46.414 m/s. Keeping it leaves 28 m/s pointing almost straight AT
// the anchor, which a reel cannot amplify at all.
//
// ⚠️ **Anchored is not the same as off the ground.** A player standing on a roof with a hook
// in it walks and jumps like anybody else — `f004_a_hook_in_the_wall_does_not_glue_the_player`
// below is the guard, and it goes red for the obvious version of this feature ("skip a player
// who carries an anchored hook").
// ---------------------------------------------------------------------------------------

/// Puts an anchored hook on the left arm **without** flying the tip there first.
///
/// A test may write `Hook` directly; a system may not (`vector::hook::update_hooks` is its one
/// writer). What is under test here is the reader, not the state machine — but the writer keeps
/// running, so the trigger has to stay held (`Q` = `Buttons::HOOK_LEFT`, `src/net/local.rs`) or
/// the arm lets go with `ReleaseReason::Released` on the very next tick, and the carrier has to
/// be a body that really stands in the `SpatialIndex` or it lets go with `BodyGone`.
/// Switches the **always-on pull** off for one fixture (`FIND-172`).
///
/// It is not a way of dodging an inconvenient result: it is the same move `kill_gravity` is in
/// `tests/vector_rope.rs`. `F-004`'s claim is about the STATE MACHINE — that `ground_locomotion`
/// stops writing the velocity of a body the rope has taken over, and that an anchored hook does
/// not glue a standing player to the floor. Since 2026-08-26 the rope also applies a force of
/// its own to every hooked player in flight, and a fixture that anchors on `a_real_body` and
/// then runs 15 m away from it measures that force instead of the claim: measured 29.67 m/s →
/// **0.00 m/s** with the pull left on. The pull's own bound is
/// `f172_the_always_on_pull_never_hauls_harder_than_it_does_from_a_standing_start`.
fn without_the_always_on_pull(app: &mut App) {
    app.world_mut().resource_mut::<GameData>().game.vector.drive_idle_speed_m_s = 0.0;
}

fn anchor_the_left_hook(app: &mut App, e: Entity) {
    let body = a_real_body(app);
    hold(app, KeyCode::KeyQ);
    let mut hook = app.world_mut().get_mut::<Hook>(e).expect("the player carries a Hook");
    hook.arms[Side::Left.index()].state = HookState::Anchored { body, local_m: Vec3::ZERO };
}

fn let_go_of_the_left_hook(app: &mut App, e: Entity) {
    release(app, KeyCode::KeyQ);
    let mut hook = app.world_mut().get_mut::<Hook>(e).expect("the player carries a Hook");
    hook.arms[Side::Left.index()].state = HookState::Idle;
}

/// A `BodyId` the spatial index really knows — an arm anchored on nothing is let go of as
/// `BodyGone` in the next tick, and the test would then be measuring an unhooked player.
fn a_real_body(app: &mut App) -> BodyId {
    let index = app.world().resource::<SpatialIndex>();
    let body = (1..200)
        .map(BodyId)
        .find(|id| index.body(*id).is_some())
        .expect("the map has to put at least one body into the index");
    body
}

#[test]
fn f004_a_rope_takes_the_body_over_only_when_the_ground_is_not_moving_it() {
    // The rule as a function of nothing but its arguments. The whole argument is one sentence:
    // the legs cannot produce more than the ground's top speed, so a roped body that is faster
    // than that is being moved by the rope and not by the ground.
    //
    // `top` is `run_speed_m_s` + one tick of `-gravity_m_s2` — `6.0 + 20.0/60`, both out of
    // `game.ron`. Why it is not plain 6.0 is the fourth block below.
    let top = 6.0 + 20.0 / 60.0;

    // No rope: the ground and the air, exactly as before.
    assert_eq!(movement_state(false, true, 0.0, top), MovementState::Grounded);
    assert_eq!(movement_state(false, true, 30.0, top), MovementState::Grounded);
    assert_eq!(movement_state(false, false, 30.0, top), MovementState::Airborne);

    // A rope and no ground under the feet — the swing. It read `Airborne` until today, which
    // is what the F3 overlay printed while the player hung on a rope.
    assert_eq!(movement_state(true, false, 0.0, top), MovementState::Tethered);
    assert_eq!(movement_state(true, false, 40.0, top), MovementState::Tethered);

    // A rope AND the ground, at walking pace: he is standing there. The ground keeps him, and
    // that is the half of the rule that lets a player walk on a roof with a hook in the wall.
    assert_eq!(movement_state(true, true, 0.0, top), MovementState::Grounded);
    assert_eq!(movement_state(true, true, 6.0, top), MovementState::Grounded);

    // **The measured knife edge.** Held `W` does not come back as exactly 6.0: over 60 ticks
    // on `[offlinebot]` it alternates between 5.999977112 and 6.000022888. On a bare
    // `> run_speed_m_s` the second of those flips a walking player to `Tethered` mid-stride.
    assert_eq!(movement_state(true, true, 6.000022888, top), MovementState::Grounded);
    assert_eq!(movement_state(true, true, 5.999977112, top), MovementState::Grounded);

    // A rope AND the ground, faster than any leg: it is the rope's. This is t=200 of
    // `scripts/f-001-hooks.txt`, where the reel hands the body over at 28 m/s along the rope.
    assert_eq!(movement_state(true, true, 22.138, top), MovementState::Tethered);
}

#[test]
fn f004_the_ground_does_not_write_the_velocity_of_a_player_the_rope_drags() {
    // The behavioural half of the same claim, in the real app. `game.ron: player.friction` is
    // 0.0, so `ground_locomotion` is the ONLY thing that can brake a horizontal velocity on
    // the ground — whatever is left after 30 ticks is its doing and nobody else's.
    let mut app = app();
    without_the_always_on_pull(&mut app); // see the helper — this measures the GROUND
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);
    anchor_the_left_hook(&mut app, e);

    // Tick 1 still runs under the state of the tick before (`MovementState` is derived after
    // the physics step and read at the start of the next one) — one tick of ground is the
    // price of that lag and it is deterministic.
    ticks(&mut app, 1);
    let handover = ground_speed(&app, e);

    ticks(&mut app, 29);
    let v = ground_speed(&app, e);
    assert!(
        (v - handover).abs() < 0.05,
        "half a second later the roped player is at {v:.4} m/s instead of the {handover:.4} \
         he was handed over at — the ground is still writing the horizontal velocity of a \
         body the rope is carrying (F-004, MovementState::Tethered)"
    );
    // And the counter-check, so that this test cannot pass by measuring nothing: without the
    // hook the same 30 m/s is down to 30 − 20·0.5 = 20 m/s after the same half second.
    assert!(
        v > 25.0,
        "{v:.4} m/s after half a second is the ordinary ground deceleration — the hook \
         changed nothing"
    );
}

#[test]
fn f004_tethered_is_written_when_an_arm_anchors_and_cleared_when_it_lets_go() {
    let mut app = app();
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);
    assert_eq!(state(&app, e), MovementState::Grounded);

    anchor_the_left_hook(&mut app, e);
    ticks(&mut app, 1);
    assert_eq!(
        state(&app, e),
        MovementState::Tethered,
        "an arm is anchored and the body is moving at rope speed, and nobody wrote \
         MovementState::Tethered — the variant is declared in src/shared/state.rs and dead"
    );

    let_go_of_the_left_hook(&mut app, e);
    ticks(&mut app, 2);
    assert_ne!(
        state(&app, e),
        MovementState::Tethered,
        "the hook let go and the body is still tethered — a state that is entered and never \
         left is worse than one that is never entered"
    );
}

#[test]
fn f004_a_hook_in_the_wall_does_not_glue_the_player() {
    // **The guard against the obvious version of this feature.** "Skip a player who carries an
    // anchored hook" and "write `Tethered` whenever an arm holds" both go red here: being
    // anchored is not the same as being off the ground, and a player standing on a roof with a
    // hook in it walks and jumps like anybody else.
    let mut app = app();
    without_the_always_on_pull(&mut app); // see the helper — this measures the STATE MACHINE
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 120); // land
    anchor_the_left_hook(&mut app, e);
    ticks(&mut app, 2);
    assert_eq!(
        state(&app, e),
        MovementState::Grounded,
        "he is standing on the ground with a hook in a wall — that is standing, not hanging"
    );

    // Walking.
    hold(&mut app, KeyCode::KeyW);
    ticks(&mut app, 1);
    let v = ground_speed(&app, e);
    assert!(
        (v - d.game.player.run_speed_m_s).abs() < 1e-3,
        "one tick of W with a hook in the wall gives {v:.4} m/s instead of {} — the rope took \
         his legs away (game.ron: run_speed_m_s)",
        d.game.player.run_speed_m_s
    );
    ticks(&mut app, 59);
    release(&mut app, KeyCode::KeyW);
    assert_eq!(state(&app, e), MovementState::Grounded, "a second of walking is not a swing");

    // And jumping.
    ticks(&mut app, 60); // come to a stop again
    let before = at(&app, e).y;
    hold(&mut app, KeyCode::Space);
    ticks(&mut app, 12); // 0.2 s
    release(&mut app, KeyCode::Space);
    // Same re-derivation as `t007_a_jump_reaches_exactly_the_height_the_file_allows`, and it
    // has to be the same expression: this test's whole claim is *"the hook changes nothing"*,
    // so its number is the free jump's number and never one of its own. `v0·t − ½gt²` with the
    // 2026-08-27 pair (8.2, −32) is 1.000 m where the literal `0.90` this line carried was
    // (6.5, −20).
    let rise_02 =
        d.game.player.jump_speed_m_s * 0.2 - 0.5 * -d.game.gravity_m_s2 * 0.2 * 0.2;
    let risen = at(&app, e).y - before;
    assert!(
        (risen - rise_02).abs() < 0.05,
        "0.2 s after a jump with a hook in the wall he has risen {risen:.4} m instead of \
         {rise_02:.4} — an anchored hook must not cost the player his jump"
    );
}

// ---------------------------------------------------------------------------------------
// 4. F-012 — the clamp. THE test that goes red when `MaxLinearSpeed` disappears.
// ---------------------------------------------------------------------------------------

#[test]
fn f012_the_top_speed_is_clamped_and_not_merely_documented() {
    // The clamp exists from day one and is not retrofitted (bible 6.4, fling exploits).
    // Counter-check driven: without `MaxLinearSpeed` in `player::spawn_player` this test
    // reports 200.0000 m/s instead of 75.0000.
    let mut app = app();
    let max = data(&app).game.vector.max_speed_m_s;

    // High above the city and fast: on the ground his horizontal velocity belongs to
    // `locomotion::ground_locomotion` and would be overwritten in the same tick.
    let flung = second_player(&mut app, Vec3::new(0.0, 200.0, 0.0));
    app.world_mut().get_mut::<LinearVelocity>(flung).unwrap().0 = Vec3::new(200.0, 0.0, 0.0);

    ticks(&mut app, 30);

    let speed = app.world().get::<Velocity>(flung).unwrap().speed_m_s();
    assert!(
        speed <= max + 1e-3,
        "the player flies at {speed:.4} m/s, game.ron: vector.max_speed_m_s = {max}"
    );
    assert!(
        speed > max - 1.0,
        "{speed:.4} m/s is far below the clamp — then this test is measuring something else"
    );
}

// ---------------------------------------------------------------------------------------
// 5. Every world collider carries a body
// ---------------------------------------------------------------------------------------

#[test]
fn t007_every_world_collider_carries_a_rigid_body() {
    // A collider without a `RigidBody` collides today and is invisible tomorrow: avian's
    // character controller filters on `With<ColliderOf>` (.../move_and_slide.rs:82), and
    // `ColliderOf` only comes into being for a collider that belongs to a body. Retrofitting
    // that means touching every row of every map — so it is checked from the start.
    //
    // **The one test here that is not pinned**, and deliberately: it names no coordinate and
    // no fixture, so it says something about *every* map — including the one that ships.
    let mut app = app_on_current_map();
    let mut q = app.world_mut().query::<(&Name, &Collider, Option<&RigidBody>)>();
    let bodyless: Vec<String> = q
        .iter(app.world())
        .filter(|(_, _, body)| body.is_none())
        .map(|(name, _, _)| name.to_string())
        .collect();
    assert!(
        bodyless.is_empty(),
        "{} collider(s) without a RigidBody: {bodyless:?}",
        bodyless.len()
    );

    // And the check is worth something only if there is anything to check.
    let mut blocks = app.world_mut().query_filtered::<&RigidBody, With<Block>>();
    let statics = blocks.iter(app.world()).filter(|b| **b == RigidBody::Static).count();
    assert!(statics > 40, "only {statics} static world blocks — is the city built at all?");
}

// ---------------------------------------------------------------------------------------
// 6. There is no such thing as THE player
// ---------------------------------------------------------------------------------------

#[test]
fn t007_a_second_player_is_a_body_of_his_own() {
    // Physics that hangs on `LocalPlayer` would be a single-player game you notice as one in
    // month twelve (§6 rule 3, docs/multiplayer.md).
    let mut app = app();
    // 8 m to the side, inside `maps.ron: layout.clear_radius_m` — there is no house there,
    // and no player either.
    let second = second_player(&mut app, Vec3::new(8.0, 2.0, 0.0));
    ticks(&mut app, 180);

    let y = at(&app, second).y;
    assert!(y.abs() < 0.01, "the second player rests at y = {y} instead of on the ground");
    assert_eq!(state(&app, second), MovementState::Grounded);

    let mut q = app.world_mut().query::<(&PlayerId, &RigidBody, &Collider, &MaxLinearSpeed)>();
    assert_eq!(
        q.iter(app.world()).count(),
        2,
        "both players carry the full physics body, not just the local one"
    );
}

// ---------------------------------------------------------------------------------------
// 3bc. F-006 the MIXING RULE — what an anchored rope adds to WASD (`docs/NEXT.md` §1B)
//
// The user, 2026-08-12 (`docs/NEXT.md` §1A, verbatim): *„wenn ich mit seilen festhake (was
// instant sein soll) und w in die richtung drücke will ich dass man deutlich mehr geboosted
// wird. also dass man dort richtig hingezogen wird. wenn man aber a oder d drückt wird nach
// links/rechts geboostet! wenn man zur seite schaut soll die steuerung mitdrehen. also wenn ich
// 45 grad nach links und w drücke dann etwas eingezogen aber auch boost zur seite."*
//
// The plan was designed three ways and judged nine times; what came out is one line of
// arithmetic, and these five tests are its five acceptance numbers. They drive
// `locomotion::rope_steer` and `locomotion::air_thrust` **directly**, without an `App`: an
// acceleration is a function of its arguments, and measuring it through a physics step would
// mean measuring gravity, the solver and the ground contact at the same time. The whole-app
// half of the claim is `f006_the_swerve_bends_the_flight_without_letting_go_of_the_hook` above
// and `tests/vector_gas.rs::f006_a_second_of_rope_steering_costs_what_the_file_says`.
//
// Every number below is READ OUT OF `game.ron`, never typed: the asserts are the *shape* of the
// rule (10 + 30 at 0°, the cosine split at 45°, nothing at 90°), not a snapshot of today's
// tuning — all three keys are ⚠️ UNTUNED and are meant to move.
// ---------------------------------------------------------------------------------------

/// The RON, without an `App` — the same loader `tests/data.rs` uses.
fn game_data() -> GameData {
    GameData::load(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

fn steer_tuning(d: &GameData) -> SteerTuning {
    SteerTuning {
        pull_m_s2: d.game.player.air_pull_m_s2,
        lateral_m_s2: d.game.player.air_lateral_m_s2,
        fade_m: d.game.player.air_pull_fade_m,
        min_rope_m: d.game.vector.min_rope_m,
        lift_m_s2: -d.game.gravity_m_s2 * d.game.player.air_pull_lift_fraction,
    }
}

/// An anchor `length_m` away, in the horizontal direction `yaw` looks at — so `yaw = 0` puts it
/// straight ahead along −Z and the look/rope angle is exactly the yaw difference.
fn anchor_at(yaw: f32, length_m: f32) -> Vec3 {
    let (sin, cos) = yaw.sin_cos();
    Vec3::new(-sin, 0.0, -cos) * length_m
}

/// `look + rope` for one anchor — the full mixing rule, the way `air_control` assembles it with
/// a full tank and the steer grant paid.
fn mixed(d: &GameData, look_yaw: f32, anchor: Vec3, move_x: f32, move_y: f32) -> Vec3 {
    let look_dir = Intent { yaw: look_yaw, ..default() }.look_dir();
    let look = air_thrust(look_dir, look_yaw, move_x, move_y, d.game.player.air_accel_m_s2);
    look + rope_steer(&[anchor], look_dir, look_yaw, move_x, move_y, steer_tuning(d))
}

#[test]
fn f006_looking_straight_at_the_anchor_w_hauls_at_the_sum_of_both_numbers() {
    // *„dass man dort richtig hingezogen wird"* — and it is a SUM, not a blend: the pull sits
    // outside the `clamp_length_max(1.0)` that makes W+D one thrust. 10 + 30 = 40 m/s², four
    // times what a rope-less player gets and 88 % of `boost_m_s2`, so Shift is still the
    // strong option.
    let d = game_data();
    let due = d.game.player.air_accel_m_s2 + d.game.player.air_pull_m_s2;

    // 60 m of rope: far past `min_rope_m + air_pull_fade_m`, so the fade is 1 and out of the way.
    let anchor = anchor_at(0.0, 60.0);
    let a = mixed(&d, 0.0, anchor, 0.0, 1.0);

    let towards = anchor.normalize();
    let closing = a.dot(towards);
    assert!(
        (closing - due).abs() < 0.5,
        "W straight at the anchor closes at {closing:.4} m/s² instead of the \
         air_accel_m_s2 {} + air_pull_m_s2 {} = {due} that game.ron owes — before the mixing \
         rule this was {} and the user's word for it was that a rope „bringt gar nichts\"",
        d.game.player.air_accel_m_s2,
        d.game.player.air_pull_m_s2,
        d.game.player.air_accel_m_s2
    );
    // And nothing sideways **except the weight the aligned pull takes off** (2026-08-20):
    // `air_pull_lift_fraction` puts `-gravity_m_s2 * fraction` on `ŷ`, gated by the same
    // cosine, and on a horizontal rope `ŷ` is across the rope by construction. So the check is
    // not "nothing off the line" any more, it is "nothing off the line that is not exactly the
    // gravity relief" — which is the stronger sentence: a stray tangential term of any other
    // size fails it just as it did before.
    let lift_due = -d.game.gravity_m_s2 * d.game.player.air_pull_lift_fraction;
    let sideways = (a - towards * closing - Vec3::Y * lift_due).length();
    assert!(
        sideways < 0.05,
        "{sideways:.4} m/s² across a rope the player is looking straight down, over and above \
         the {lift_due:.2} m/s² of gravity relief game.ron owes at full alignment"
    );
}

#[test]
fn f006_at_forty_five_degrees_w_is_part_haul_and_part_side_boost() {
    // *„wenn ich 45 grad nach links und w drücke dann etwas eingezogen aber auch boost zur
    // seite"* — one sentence, two numbers, and they are the cosine projection's:
    //   radial     = air_accel·cos45 + air_pull·cos45 = 7.071 + 21.213 = 28.284
    //   tangential = air_accel·sin45                  =                   7.071
    // ⚠️ **This is the test that dies if anybody puts `nlerp` back** (FIND-046): a slerp/nlerp
    // between look and rope keeps the magnitude and turns the direction, so it cannot produce
    // 28.28 radial *and* 7.07 tangential out of a 10 m/s² look term at all.
    let d = game_data();
    let root_half = std::f32::consts::FRAC_1_SQRT_2;
    let radial_due = (d.game.player.air_accel_m_s2 + d.game.player.air_pull_m_s2) * root_half;
    let tangential_due = d.game.player.air_accel_m_s2 * root_half;

    let anchor = anchor_at(0.0, 60.0); // the rope stays on −Z
    let a = mixed(&d, std::f32::consts::FRAC_PI_4, anchor, 0.0, 1.0); // and the head turns 45°

    let towards = anchor.normalize();
    let radial = a.dot(towards);
    // Minus the gravity relief, which is on `ŷ` and rides the same cos45 the haul does — see
    // the sister test above. What is left is the *„boost zur seite"* this test is about.
    let lift_due = -d.game.gravity_m_s2 * d.game.player.air_pull_lift_fraction * root_half;
    let tangential = (a - towards * radial - Vec3::Y * lift_due).length();
    assert!(
        (radial - radial_due).abs() < 0.5,
        "45° off the rope the haul is {radial:.4} m/s², not the {radial_due:.4} the cosine \
         projection owes — {a:?}"
    );
    assert!(
        (tangential - tangential_due).abs() < 0.3,
        "and {tangential:.4} m/s² across it instead of {tangential_due:.4}: „etwas eingezogen \
         aber auch boost zur seite\" is BOTH, and this half is the „zur seite\""
    );
}

#[test]
fn f006_across_the_rope_there_is_no_haul_left_at_all() {
    // *„das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird"* as arithmetic:
    // at 90° `max(0, l̂·r̂)` is exactly zero, so the rope adds NOTHING towards the anchor and
    // what is left is the free-air control, pure swing steer. Behind 90° it stays zero — which
    // is also requirement 7 („aktuell wenn ich seil spanne und s drücke werde ich stark zum
    // seil gezogen! das soll nicht sein!") from the other side: looking away can never haul.
    let d = game_data();
    let anchor = anchor_at(0.0, 60.0);

    for (name, look_yaw) in [
        ("90° across", std::f32::consts::FRAC_PI_2),
        ("135° behind", 3.0 * std::f32::consts::FRAC_PI_4),
        ("180° away", std::f32::consts::PI),
    ] {
        let a = mixed(&d, look_yaw, anchor, 0.0, 1.0);
        let radial = a.dot(anchor.normalize());
        assert!(
            radial < 0.05,
            "{name} from the rope, W held, the player is still hauled at {radial:.4} m/s² — \
             the projection has to be max(0, dot) and nothing else ({a:?})"
        );
        // ...and the free-air control is untouched: the whole 10 m/s² is still there.
        assert!(
            (a.length() - d.game.player.air_accel_m_s2).abs() < 0.05,
            "{name}: {:.4} m/s² of thrust instead of the plain air control {} — the rope must \
             subtract nothing either",
            a.length(),
            d.game.player.air_accel_m_s2
        );
    }
}

#[test]
fn f006_without_a_rope_the_air_control_is_bit_identical_to_before() {
    // The regression guard for everything else in this block: an unhooked player must come out
    // of the new code with the **same bits**, not the same number to five places. `assert_eq!`
    // on the `Vec3` and no epsilon anywhere — a `+ Vec3::ZERO` that rounds a −0.0 into a +0.0
    // is exactly the kind of change that looks free and is not.
    let d = game_data();
    for (yaw, pitch, move_x, move_y) in [
        (0.0, 0.0, 0.0, 1.0),
        (0.7, -0.9, 1.0, 0.0),
        (-2.4, 0.4, -1.0, 1.0),
        (1.1, 0.0, 0.3, -1.0), // S, which is never a thrust
        (3.0, 1.2, 0.0, 0.0),
    ] {
        let look_dir = Intent { yaw, pitch, ..default() }.look_dir();
        let before = air_thrust(look_dir, yaw, move_x, move_y, d.game.player.air_accel_m_s2);
        let with_no_anchors =
            before + rope_steer(&[], look_dir, yaw, move_x, move_y, steer_tuning(&d));
        assert_eq!(
            with_no_anchors, before,
            "yaw {yaw}, pitch {pitch}, ({move_x}, {move_y}): a player with no rope must be \
             untouched by the mixing rule, and `rope_steer` on an empty slice is the whole of \
             that promise"
        );
    }
}

#[test]
fn f006_the_pull_lets_go_before_the_short_rope_cliff() {
    // **FIND-035 is the reason this key exists**: at `min_rope_m` the length constraint takes
    // 17 m/s out of the player in ONE tick, and W straight at an anchor 3 m away feeds exactly
    // that. So the pull is **exactly zero at `min_rope_m`** and climbs to full over
    // `air_pull_fade_m` above it — zero at 3 m, full at 15 m with the numbers of 2026-08-13.
    //
    // ⚠️ The lateral term is deliberately NOT faded: `A`/`D` next to an anchor pushes you AWAY
    // from the cliff, which is the one thing you want close in.
    //
    // ⚠️ **The number read here is the component ALONG the rope and not the length of the
    // vector** — since 2026-08-20 `rope_steer` also puts `air_pull_lift_fraction` of gravity
    // on `ŷ`, which on a horizontal rope is perpendicular, so `.length()` would be
    // `hypot(30, 14) = 33.1` and this test would fail on a term it is not about. The relief
    // rides the same fade, so every claim below is unchanged by it.
    let d = game_data();
    let t = steer_tuning(&d);
    let full = d.game.player.air_pull_m_s2;

    for length_m in [0.5, 1.0, t.min_rope_m] {
        let anchor = anchor_at(0.0, length_m);
        let pull = rope_steer(&[anchor], anchor.normalize(), 0.0, 0.0, 1.0, t).length();
        assert!(
            pull < 1e-4,
            "a rope {length_m} m long (min_rope_m = {}) still hauls at {pull:.4} m/s² — that \
             thrust runs straight into FIND-035's 17 m/s cliff",
            t.min_rope_m
        );
    }

    // The band in between rises, and nowhere in it is the pull already full.
    let mut previous = 0.0;
    for step in 1..12 {
        let length_m = t.min_rope_m + t.fade_m * step as f32 / 12.0;
        let anchor = anchor_at(0.0, length_m);
        let along = anchor.normalize();
        let pull = rope_steer(&[anchor], along, 0.0, 0.0, 1.0, t).dot(along);
        assert!(pull > previous, "the fade is not monotone at {length_m:.2} m: {pull} <= {previous}");
        assert!(pull < full, "at {length_m:.2} m the pull is already the full {full}");
        previous = pull;
    }

    // And at the top of the band it is the whole number, with no fade left to pay.
    let anchor = anchor_at(0.0, t.min_rope_m + t.fade_m);
    let along = anchor.normalize();
    let pull = rope_steer(&[anchor], along, 0.0, 0.0, 1.0, t).dot(along);
    assert!(
        (pull - full).abs() < 1e-3,
        "at min_rope_m + air_pull_fade_m = {} m the pull is {pull:.4} instead of the full {full}",
        t.min_rope_m + t.fade_m
    );
}

#[test]
fn f006_two_opposed_ropes_do_not_average_themselves_away() {
    // Judge-forced detail 2, and it is the reason `rope_steer` takes a SLICE and not a mean
    // direction: `unit(r̂₁ + r̂₂)` for two anchors 180° apart is the zero vector, so the
    // strongest place in the game — hanging between two buildings — would be the one where W
    // does nothing at all. Each arm projects on its own; only the forces are averaged.
    let d = game_data();
    let t = steer_tuning(&d);
    let look_dir = Intent::default().look_dir(); // −Z
    let ahead = anchor_at(0.0, 60.0);
    let behind = -ahead;

    let both = rope_steer(&[ahead, behind], look_dir, 0.0, 0.0, 1.0, t);
    let one = rope_steer(&[ahead], look_dir, 0.0, 0.0, 1.0, t);
    assert!(
        both.length() > 0.4 * d.game.player.air_pull_m_s2,
        "two anchors 180° apart pull at {:.4} m/s² — a mean direction would make this zero \
         ({both:?})",
        both.length()
    );
    // The one behind him contributes nothing (its projection is 0), so the budget it does own
    // is what the halving costs: exactly half of the single-rope pull.
    assert!(
        (both.length() - one.length() * 0.5).abs() < 0.05,
        "the pull budget is shared, so the anchor he is looking at should give {:.4} of the \
         {:.4} a single rope gives — got {:.4}",
        one.length() * 0.5,
        one.length(),
        both.length()
    );
}

#[test]
fn f006_the_strafe_rides_the_look_right_and_never_the_rope_tangent() {
    // Judge-forced detail 3. A rope tangent **flips sign** the moment the anchor passes beside
    // the player, which inverts `A`/`D` in the middle of a swing — the one moment he is
    // committed and cannot correct. So `D` is +X at `yaw = 0` whatever the rope is doing, and
    // it stays +X while the anchor walks all the way round him.
    let d = game_data();
    let t = steer_tuning(&d);
    let look_dir = Intent::default().look_dir();
    for step in 0..12 {
        let anchor = anchor_at(std::f32::consts::TAU * step as f32 / 12.0, 40.0);
        let a = rope_steer(&[anchor], look_dir, 0.0, 1.0, 0.0, t); // D alone, no W
        assert!(
            (a.x - d.game.player.air_lateral_m_s2).abs() < 1e-3 && a.z.abs() < 1e-3,
            "with the anchor at step {step}/12 around him, D gave {a:?} instead of \
             +X · air_lateral_m_s2 {} — the strafe is riding the rope instead of the look",
            d.game.player.air_lateral_m_s2
        );
        assert!(a.y.abs() < 1e-6, "and it stays horizontal: {a:?}");
    }
}

#[test]
fn f006_in_the_real_app_the_rope_pull_is_there_and_an_empty_tank_gets_none_of_it() {
    // **The whole-app half of the 40 m/s² headline**, and the one claim the pure-function tests
    // above cannot make: that `air_control` really assembles `look + rope`, off the real
    // `Hook`, the real `Transform` and the real `GasGrant`, in the real schedule. Two bodies,
    // one rope each, one tick — the only difference between them is the tank.
    //
    // *„ohne gas kann man immernoch w a d nutzen um etwas movement aufzubauen (aber hälfte
    // ca)"* — so the LOOK term halves. **The rope term does not halve, it disappears**: it is
    // gated on `GasGrant::steer`, and half a rope pull for no gas would be exactly the free
    // thrust all nine judges of `docs/NEXT.md` §1B refused.
    let mut app = app();
    // 🔴 **`Pendulum`, PINNED** — this test is about `rope_steer` and `air_pull_m_s2`, i.e. about
    // ONE of the two force models, and it read whichever way `game.ron` happened to be set. It
    // went red the day the shipped default moved to `Drive` (2026-08-23) without a single line
    // of its subject having changed. Same line, same reason, as `tests/vector_rope.rs::app`.
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model =
        defeated_by_titan::data::RopeForceModel::Pendulum;
    let d = data(&app);
    ticks(&mut app, 2); // the spatial index is filled in the first steps, not in `Startup`
    let body = a_real_body(&mut app);
    // 50 m up, not the 200 m of `flyers`: the rope has to stay well inside `hook_range_m` or
    // `update_hooks` lets go of it as `Overextended` and this measures an unhooked player.
    let full = second_player(&mut app, Vec3::new(0.0, 50.0, 0.0));
    let dry = second_player(&mut app, Vec3::new(24.0, 50.0, 0.0));

    let hang = |app: &mut App, e: Entity| {
        let mut hook = app.world_mut().get_mut::<Hook>(e).expect("a player carries two hooks");
        hook.arms[Side::Left.index()].state =
            HookState::Anchored { body, local_m: Vec3::ZERO };
    };
    // `Buttons::HOOK_LEFT` held, or `update_hooks` releases the arm as `Released` next tick —
    // these two carry no `LocalPlayer`, so their `Intent` is whatever `fly` last wrote.
    let held = |yaw: f32, pitch: f32| Intent {
        move_y: 1.0,
        yaw,
        pitch,
        buttons: Buttons::HOOK_LEFT,
        ..default()
    };
    for e in [full, dry] {
        hang(&mut app, e);
        fly(&mut app, e, held(0.0, 0.0));
    }
    app.update(); // `update_hooks` puts `tip_m` onto the body

    // Now look straight down each rope, so the projection is 1 and the number is the sum.
    for e in [full, dry] {
        let tip = app.world().get::<Hook>(e).unwrap().arm(Side::Left).tip_m;
        let hand = at(&app, e) + Vec3::Y * d.game.player.eye_height_m;
        let along = (tip - hand).normalize();
        fly(&mut app, e, held((-along.x).atan2(-along.z), along.y.asin()));
        hang(&mut app, e);
    }
    app.world_mut().get_mut::<Gas>(dry).unwrap().current = 0.0;
    app.update();

    let due = d.game.player.air_accel_m_s2 + d.game.player.air_pull_m_s2;
    // **Minus the gravity relief first** (2026-08-20): `air_pull_lift_fraction` puts
    // `-gravity_m_s2 * fraction` on `ŷ` at full alignment, and this test is about the two
    // numbers the sum is made of, not about the third. The look here runs straight down the
    // rope, so what is left is `air_accel_m_s2 + air_pull_m_s2` along it and nothing else.
    let lift_due = -d.game.gravity_m_s2 * d.game.player.air_pull_lift_fraction;
    let hauling = (run_accel(&app, full) - Vec3::Y * lift_due).length();
    assert!(
        (hauling - due).abs() < 1.0,
        "W straight down a real rope in the real app gives {hauling:.4} m/s² instead of \
         air_accel_m_s2 {} + air_pull_m_s2 {} = {due} — the pure function is right and the \
         wiring is not",
        d.game.player.air_accel_m_s2,
        d.game.player.air_pull_m_s2
    );

    let empty = run_accel(&app, dry).length();
    let half_look = d.game.player.air_accel_m_s2 * d.game.player.air_accel_empty_fraction;
    assert!(
        (empty - half_look).abs() < 0.2,
        "the same rope on an empty tank gives {empty:.4} m/s²; the look term halved is \
         {half_look} and the rope term is ZERO, so that is the whole of it. {:.4} would be \
         half the rope pull smuggled in for free",
        half_look + d.game.player.air_pull_m_s2 * 0.5
    );
}

// ---------------------------------------------------------------------------------------
// THE LOOK-PULL — the user, 2026-08-20
//
//   „man muss wenn man sich hookt und in die richtung gehen stärker in die richtung gehen!
//    also wenn man da hin schaut dass nicht alle physics also gravitiy so stark sind. dass man
//    gerader hingezogen wird … aber wenn man nicht hinschaut man auch gut kreise schwingen kann"
//
// Two ends of one trade, and the two tests below are the two ends. What is measured in both is
// the **net** acceleration — the thrust plus `gravity_m_s2` — because a straight line is a
// property of what the world does to you and not of what you push with. The pull was never the
// problem: it is 40 m/s² along the rope and always was.
// ---------------------------------------------------------------------------------------

/// The angle between the net acceleration and the rope, in degrees. **The straightness
/// number.** 0° is "the game takes you exactly where you are aiming".
fn droop_deg(net: Vec3, towards: Vec3) -> f32 {
    net.normalize().dot(towards.normalize()).clamp(-1.0, 1.0).acos().to_degrees()
}

#[test]
fn f005_looking_at_the_anchor_the_pull_is_not_eaten_by_gravity() {
    // ⚠️ RED before `air_pull_lift_fraction` existed, and the red number is arithmetic anybody
    // can redo: a horizontal rope, the look straight down it, `W` held. Thrust is
    // `air_accel_m_s2 + air_pull_m_s2` = 40 m/s² along the rope; gravity is 20 m/s² across it;
    // `atan(20 / 40)` = **26.57° below the line he is aiming at**. In the real game that came
    // out as `scripts/f005-feel.txt` ACT 3: four seconds of `W` at an anchor 9.5 m above him
    // and `assert Height > 8 — measured 1.996`.
    //
    // The acceptance is his sentence, made into a number: **at full alignment the net has to
    // point within 15° of the rope** — i.e. the straight haul has to be recognisably straight.
    // 15 and not 0 because gravity does not go away (see the sister test): the file's `< 1.0`
    // bound on the fraction is the same requirement seen from the other side.
    let d = game_data();
    let anchor = anchor_at(0.0, 60.0);
    let net = mixed(&d, 0.0, anchor, 0.0, 1.0) + Vec3::Y * d.game.gravity_m_s2;
    let droop = droop_deg(net, anchor);

    let without = mixed(&d, 0.0, anchor, 0.0, 1.0)
        - Vec3::Y * (-d.game.gravity_m_s2 * d.game.player.air_pull_lift_fraction)
        + Vec3::Y * d.game.gravity_m_s2;
    println!(
        "F-005 straightness at full alignment: {droop:.2}° off the rope, \
         {:.2}° with air_pull_lift_fraction deleted",
        droop_deg(without, anchor)
    );

    assert!(
        droop < 15.0,
        "looking straight down a 60 m rope with W held, the game hauls the player {droop:.2}° \
         off the line he is aiming at — game.ron: player.air_pull_lift_fraction is {} and the \
         acceptance is 15°",
        d.game.player.air_pull_lift_fraction
    );
    // The control: delete the term and the number has to move back to the 26.57° above. A test
    // whose number does not move when the thing it measures is removed measures nothing.
    assert!(
        droop_deg(without, anchor) > droop + 10.0,
        "deleting the lift moved the droop from {droop:.2}° to {:.2}° — under 10° of \
         difference means this test is not measuring the lift at all",
        droop_deg(without, anchor)
    );
}

#[test]
fn f005_looking_away_from_the_rope_the_swing_is_bit_identical_to_before() {
    // The other half of the same sentence: *„aber wenn man nicht hinschaut man auch gut kreise
    // schwingen kann"*. The relief rides `cᵢ = max(0, l̂ · r̂ᵢ)`, so from 90° on it is gone and
    // a swing is exactly the state in which the look is not down the rope.
    //
    // ⚠️ **Not `assert_eq!`, and the tolerance is measured rather than picked.** The first
    // version of this test did use `assert_eq!` and it went red at 270°: `l̂ · r̂` at a right
    // angle is `+1.2e-8` and not `0.0`, because 270° in f32 radians is not exactly `3π/2`, and
    // `max(0, ·)` keeps that sign. The whole term is then **1.7e-7 m/s²** — 8 parts in a
    // billion of the pull, a millimetre per second after an hour of swinging. `1e-5` is two
    // orders above that and still 3e-7 of `air_pull_m_s2`, so a term that really reappeared
    // could not hide under it.
    let d = game_data();
    let anchor = anchor_at(0.0, 60.0);
    let mut zeroed = steer_tuning(&d);
    zeroed.lift_m_s2 = 0.0;

    for turn_deg in [90.0_f32, 120.0, 180.0, 270.0] {
        let yaw = turn_deg.to_radians();
        let look_dir = Intent { yaw, ..default() }.look_dir();
        let with = rope_steer(&[anchor], look_dir, yaw, 0.0, 1.0, steer_tuning(&d));
        let without = rope_steer(&[anchor], look_dir, yaw, 0.0, 1.0, zeroed);
        let leak = (with - without).length();
        assert!(
            leak < 1e-5,
            "{turn_deg}° off the rope the gravity relief still contributes {leak:e} m/s² — the \
             arc the player swings in is not the arc it was"
        );
    }

    // And the swing still has something to swing with: at 90° the look term is the whole of
    // `air_accel_m_s2`, across the rope, which is what feeds a circle.
    let yaw = std::f32::consts::FRAC_PI_2;
    let a = mixed(&d, yaw, anchor, 0.0, 1.0);
    let towards = anchor.normalize();
    let tangential = (a - towards * a.dot(towards)).length();
    println!("F-005 circle at 90° off the rope: {tangential:.3} m/s² across it, 0 along it");
    assert!(
        (tangential - d.game.player.air_accel_m_s2).abs() < 0.05,
        "{tangential:.3} m/s² across the rope instead of air_accel_m_s2 {} — the swing lost \
         the thrust that draws the circle",
        d.game.player.air_accel_m_s2
    );
}

#[test]
fn f005_the_gravity_relief_is_a_fraction_between_the_droop_and_weightlessness() {
    // The bounds on `player.air_pull_lift_fraction` itself, and both ends are the user's own
    // sentence rather than taste. ⚠️ Here and not in `tests/data.rs` for the same reason the
    // hook ceiling's bounds live next to the hook: the meaning is `rope_steer`'s.
    let d = game_data();
    let f = d.game.player.air_pull_lift_fraction;

    // Below 0.5 the aligned haul still loses more than half of what gravity takes. The measured
    // floor, not a felt one: at 0.0 the net points 26.57° below the line the player is aiming
    // at (`f005_looking_at_the_anchor_the_pull_is_not_eaten_by_gravity`), and at 0.5 it is
    // 14.04° — already outside that test's 15° acceptance would be at 0.49.
    assert!(
        f > 0.5,
        "air_pull_lift_fraction is {f} — under half of gravity relieved and „dass man gerader \
         hingezogen wird\" is still not true"
    );
    // At 1.0 the player is weightless while looking at his own anchor, and then there is no arc
    // to fall back into when he looks away — which is the other half of the same sentence,
    // „aber wenn man nicht hinschaut man auch gut kreise schwingen kann".
    assert!(
        f < 1.0,
        "air_pull_lift_fraction is {f} — at 1.0 a player looking at his anchor with W held has \
         no weight at all, and a swing is a thing that falls"
    );
    // And what is left over is a real force and not a rounding: at 0.7 it is 6.0 m/s², which is
    // `player.run_speed_m_s` worth of acceleration still pulling him down every second.
    let left_over = -d.game.gravity_m_s2 * (1.0 - f);
    assert!(
        left_over >= 1.0,
        "{left_over:.2} m/s² of weight left at full alignment is not a world to swing in"
    );
}


// ---------------------------------------------------------------------------------------
// `FIND-149` — the DRIVE. `locomotion::rope_drive` directly, without an `App`.
//
// The user played the reference beside this game on 2026-08-23: *„wenn ich mich hooke: dann
// werde ich direkt rangezogen wenn ich ran gehe. mit a und d kann man zur seite gehen. aber
// sonst wird man direkt hingezogen! **wenn ich nichts drucke dann wird auch nicht
// rangezogen!**"* … *„aber es ist ein etwas smoother übergang! aber recht schnell!"*
//
// The whole-app half of these claims is `tests/vector_rope.rs::f149_*` and
// `scripts/f006-drive.txt`. Every number below is read out of `game.ron`.
// ---------------------------------------------------------------------------------------

fn drive_tuning(d: &GameData) -> DriveTuning {
    DriveTuning {
        speed_m_s: d.game.vector.drive_speed_m_s,
        lateral_m_s: d.game.vector.drive_lateral_m_s,
        ramp_s: d.game.vector.drive_ramp_s,
        accel_max_m_s2: d.game.vector.drive_accel_max_m_s2,
        steer_pull_fraction: d.game.vector.drive_steer_pull_fraction,
    }
}

#[test]
fn f149_a_hooked_player_who_holds_nothing_is_not_driven_at_all() {
    // **The load-bearing sentence, as a function call.** A rope 40 m ahead, the player looking
    // straight at it — the most "obviously about to be pulled" geometry there is — and no key.
    let d = game_data();
    let t = drive_tuning(&d);
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();
    // Falling at 20 m/s, so that a wrong sign or a missing early return would show up as a
    // brake instead of hiding in a zero velocity.
    let falling = Vec3::new(0.0, -20.0, 0.0);

    let nothing = rope_drive(&[ahead], look, 0.0, 0.0, 0.0, falling, t);
    assert_eq!(
        nothing,
        Vec3::ZERO,
        "a hooked player with no key held was driven at {nothing:?} — „wenn ich nichts drucke \
         dann wird auch nicht rangezogen\" is the whole of this model"
    );
    // `S` is the rope's tension key and never a thrust (`docs/NEXT.md` §1A requirement 7).
    let s_key = rope_drive(&[ahead], look, 0.0, 0.0, -1.0, falling, t);
    assert_eq!(s_key, Vec3::ZERO, "`S` drove the player at {s_key:?} — it tensions, it never hauls");
    // And looking away from the anchor is the same answer, with `W` held: the look gate is
    // `max(0, l̂·r̂)`, so there is no angle at which a rope behind you drags you backwards.
    let behind = rope_drive(&[anchor_at(std::f32::consts::PI, 40.0)], look, 0.0, 0.0, 1.0, falling, t);
    assert_eq!(behind, Vec3::ZERO, "a rope 180° behind the look drove the player at {behind:?}");
}

#[test]
fn f149_the_drive_chases_a_speed_instead_of_building_one() {
    // *„es ist ein etwas smoother übergang! aber recht schnell!"* — an exponential with the
    // file's time constant, and an acceleration that dies as the speed arrives. That last
    // property is what makes it a drive and not a thrust: `rope_steer` at the same geometry
    // pushes just as hard at 50 m/s as it does at 0.
    let d = game_data();
    let t = drive_tuning(&d);
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();
    let target = Vec3::NEG_Z * t.speed_m_s;

    // From rest: the full gap over the ramp, along the rope — **under the ceiling**
    // (`FIND-172`). At `(52, 0.08, 250)` the unbounded term is 650 m/s² and the file's weight
    // takes 250 of it; the direction is the rope's either way.
    let from_rest = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, Vec3::ZERO, t);
    let due = (t.speed_m_s / t.ramp_s).min(t.accel_max_m_s2);
    assert!(
        (from_rest - Vec3::NEG_Z * due).length() < 1e-3,
        "from rest the drive gave {from_rest:?}; {due:.2} m/s² along −Z is \
         `min(drive_speed_m_s / drive_ramp_s, drive_accel_max_m_s2)` and nothing else"
    );
    // **At the target it is exactly zero** — the cap is the construction, not a clamp.
    let arrived = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, target, t);
    assert!(
        arrived.length() < 1e-4,
        "a player already travelling at the drive speed was still accelerated at {arrived:?} — \
         then it is a thrust and `drive_speed_m_s` means nothing"
    );
    // Past the target it BRAKES, which is the same rule read from the other side.
    let too_fast = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, target * 1.5, t);
    assert!(
        too_fast.dot(Vec3::NEG_Z) < 0.0,
        "at 1.5x the drive speed the drive still pushed forward ({too_fast:?})"
    );
    // One ramp of explicit integration closes 1 − 1/e of the gap, and that is the number the
    // user is being asked to feel. **The band is wide because the tick is coarse against a
    // short ramp**: at `drive_ramp_s: 0.25` one constant is 15 ticks and the discrete sum comes
    // out a little UNDER the analytic 63.2 % (64.4 %, and the old band was `0.55..0.68`); at
    // `0.08` it is 4.8 ticks, `dt/ramp` = 0.21, and the same sum lands a little OVER it
    // (68.9 %). Both are the exponential; only the sampling moved. The assert is the shape.
    //
    // ⚠️ **It starts inside the ceiling's linear regime and not from rest, since `FIND-172`.**
    // Above `drive_accel_max_m_s2 · drive_ramp_s` of gap the drive is a straight line and not an
    // exponential at all — that is the weight, and measuring the ramp through it would measure
    // the cap. The gap here is a fifth of that threshold.
    let dt = 1.0 / d.game.simulation_hz as f32;
    let gap0 = 0.2 * t.accel_max_m_s2 * t.ramp_s;
    let start = Vec3::NEG_Z * (t.speed_m_s - gap0);
    let mut v = start;
    for _ in 0..(t.ramp_s / dt).round() as u32 {
        v += rope_drive(&[ahead], look, 0.0, 0.0, 1.0, v, t) * dt;
    }
    let closed = (v.length() - start.length()) / gap0;
    assert!(
        (0.55..0.75).contains(&closed),
        "after one time constant the drive had closed {:.1} % of the gap, not the ~63 % an \
         exponential owes — `drive_ramp_s` is not the ramp it claims to be",
        closed * 100.0
    );
}

#[test]
fn f149_a_and_d_hold_a_line_off_the_anchor_and_stay_horizontal() {
    // *„mit a und d kann man zur seite gehen"* / *„das a d sorgt dafür dass man nicht immer
    // direkt zum seil gezogen wird"*. The lateral rides the horizontal look-right, so a strafe
    // in a fast swing — where the player is looking at the street — does not drive him into it.
    let d = game_data();
    let t = drive_tuning(&d);
    let below = Vec3::new(0.0, -30.0, 0.0); // the anchor under him: pitch cannot help here
    let look = Intent { pitch: -60.0_f32.to_radians(), ..default() }.look_dir();

    let strafe = rope_drive(&[below], look, 0.0, 1.0, 0.0, Vec3::ZERO, t);
    let due = (t.lateral_m_s / t.ramp_s).min(t.accel_max_m_s2);
    assert!(
        (strafe.x - due).abs() < 1e-3,
        "`D` alone gave {strafe:?}; {due:.2} m/s² along +X is \
         `min(drive_lateral_m_s / drive_ramp_s, drive_accel_max_m_s2)` — the ceiling is the same \
         one the forward axis pays (`FIND-172`)"
    );
    assert!(
        strafe.y.abs() < 1e-4 && strafe.z.abs() < 1e-4,
        "`D` while looking 60° down drove {strafe:?} — the strafe is tilting with the pitch"
    );
    // And it works with **no** forward key at all, which is the difference between steering and
    // being hauled: `A`/`D` are the player's own thrust across the rope.
    assert!(strafe.length() > 0.0, "`D` on an anchored rope did nothing");
}

#[test]
fn f149_the_two_force_models_are_not_the_same_thing() {
    // The regression guard the whole switch exists for. Same geometry, same keys, same file —
    // and if these two ever answer the same, one of the branches in `air_control` is dead and
    // the user's A/B is measuring one model twice.
    let d = game_data();
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();
    let fast = Vec3::NEG_Z * d.game.vector.drive_speed_m_s;

    let steer = rope_steer(&[ahead], look, 0.0, 0.0, 1.0, steer_tuning(&d));
    let drive = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, Vec3::ZERO, drive_tuning(&d));
    assert!(
        (steer - drive).length() > 1.0,
        "the pendulum's pull and the drive answered the same thing ({steer:?} vs {drive:?}) — \
         `game.ron: vector.rope_force_model` would then be a switch between one model and itself"
    );
    // The sharpest difference, and the one the player feels: at the drive's own top speed the
    // drive is spent and the pendulum's pull is not.
    let steer_fast = rope_steer(&[ahead], look, 0.0, 0.0, 1.0, steer_tuning(&d));
    let drive_fast = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, fast, drive_tuning(&d));
    assert!(
        steer_fast.length() > 1.0 && drive_fast.length() < 1e-3,
        "at {} m/s the pendulum pulled {:.2} m/s² and the drive {:.2} m/s² — the pendulum BUILDS \
         speed and the drive CHASES one; a test that cannot tell those apart is not a test",
        d.game.vector.drive_speed_m_s,
        steer_fast.length(),
        drive_fast.length()
    );
}

#[test]
fn f149_the_drive_numbers_are_the_ones_the_file_can_defend() {
    // ⚠️ Here and not in `tests/data.rs` for the reason the hook ceiling's bounds are: the
    // meaning of these three is `rope_drive`'s. All three are UNTUNED and are meant to move —
    // what is guarded is the shape, not the value.
    let d = game_data();
    let t = drive_tuning(&d);
    let dt = 1.0 / d.game.simulation_hz as f32;

    // A pure swing on the pendulum runs 17–21 m/s (`Q-018`); a drive at or under that would be
    // a downgrade the player feels in the first second.
    assert!(t.speed_m_s > 21.0, "drive_speed_m_s is {} — slower than the swing it replaces", t.speed_m_s);
    // And `vector.max_speed_m_s` is a CLAMP, not a speed anybody chose. A drive that reaches it
    // would make the clamp the thing the player feels.
    assert!(
        t.speed_m_s <= d.game.vector.max_speed_m_s,
        "drive_speed_m_s is {} against a max_speed_m_s of {}",
        t.speed_m_s,
        d.game.vector.max_speed_m_s
    );
    // The ramp is integrated explicitly, once per tick. Under two ticks it overshoots instead
    // of ramping, and „ein etwas smoother übergang" is gone either way.
    assert!(
        t.ramp_s > 2.0 * dt,
        "drive_ramp_s is {} s against a tick of {dt:.4} s — that is a snap, not a ramp",
        t.ramp_s
    );
    assert!(t.ramp_s < 1.0, "drive_ramp_s is {} s — „recht schnell\" it is not", t.ramp_s);
    // `A`/`D` share the drive's own cap, so a lateral above it could never be reached — which
    // is exactly why *„staerker zur seite als rangezogen"* is bought with
    // `drive_steer_pull_fraction` and not by raising this key past the speed (`FIND-172`).
    assert!(
        t.lateral_m_s > 0.0 && t.lateral_m_s <= t.speed_m_s,
        "drive_lateral_m_s is {} against a drive_speed_m_s of {}",
        t.lateral_m_s,
        t.speed_m_s
    );
    // `FIND-172`. The ceiling is the weight: below `speed/ramp` it is doing something at all,
    // and it still has to leave the drive able to reach its own speed inside a flight.
    assert!(
        t.accel_max_m_s2 > 0.0 && t.accel_max_m_s2 < t.speed_m_s / t.ramp_s,
        "drive_accel_max_m_s2 is {} against a `drive_speed_m_s / drive_ramp_s` of {:.0} — at or          above that the ceiling never binds and the player has no weight again",
        t.accel_max_m_s2,
        t.speed_m_s / t.ramp_s
    );
    // A steer that keeps the whole radial pull is the behaviour he asked to be changed; one
    // that keeps none of it turns `W`+`D` into a pure strafe.
    assert!(
        (0.0..1.0).contains(&t.steer_pull_fraction),
        "drive_steer_pull_fraction is {} — 1.0 is the pre-`FIND-172` behaviour the user rejected",
        t.steer_pull_fraction
    );
    // The always-on pull. Above the winch it would make `Ctrl` pointless, and it has to stay
    // under the drive so that `W` is still the thing that flies.
    let idle = d.game.vector.drive_idle_speed_m_s;
    assert!(
        idle > 0.0 && idle < d.game.vector.reel_speed_m_s,
        "drive_idle_speed_m_s is {idle} against a reel_speed_m_s of {} — a free pull at or above          the winch retires `Ctrl` (`F-005`)",
        d.game.vector.reel_speed_m_s
    );
    // ⚠️ The bound that is a DESIGN statement and not a range: on a vertical rope the idle pull
    // settles at `idle − |g|·ramp` m/s of climb, and hanging still must not out-climb walking.
    let climb = idle + d.game.gravity_m_s2 * d.game.vector.drive_idle_ramp_s;
    assert!(
        climb < d.game.player.run_speed_m_s,
        "hanging on a vertical rope and pressing NOTHING climbs at {climb:.2} m/s against a          run_speed_m_s of {} — „es soll immer ranziehen\" is not „es soll dich hochreissen\"",
        d.game.player.run_speed_m_s
    );
    assert!(
        d.game.vector.drive_idle_ramp_s > t.ramp_s,
        "drive_idle_ramp_s is {} and drive_ramp_s is {} — nobody presses the idle pull, so it          must not arrive faster than the key does",
        d.game.vector.drive_idle_ramp_s,
        t.ramp_s
    );
}

// ---------------------------------------------------------------------------------------
// `FIND-172` — **it always pulls, `A`/`D` beat the pull, and the drive got a weight.** The
// user, 2026-08-26, after playing the drive:
//
// > *„es ist zu aggressiv. also man wird zu sehr rangezogen. und folgendes soll verändert
// > werden. ich will dass es immer ranzieht. nicht nur wenn ich w drücke! nur wenn ich a oder d
// > drücke dass es stärker zur seite geht als rangezogen!"*
//
// and, one minute later:
//
// > *„zudem fühlt sich die gravitation nicht richtig an. oder die masse von dem character. es
// > fühlt sich zu leicht an."*
//
// The always-on pull is wiring and is measured in `tests/vector_rope.rs::f172_*`; what a pure
// function can hold is the ceiling and the steer.
// ---------------------------------------------------------------------------------------

#[test]
fn f172_the_drive_can_never_yank_harder_than_the_files_own_ceiling() {
    let d = game_data();
    let t = drive_tuning(&d);
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();

    // The worst case there is: at rest, looking straight at the anchor, `W` down.
    let yank = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, Vec3::ZERO, t).length();
    assert!(
        (yank - t.accel_max_m_s2).abs() < 1e-2,
        "the drive's hardest pull is {yank:.1} m/s² against a drive_accel_max_m_s2 of {} —          „zu aggressiv\" is this number",
        t.accel_max_m_s2
    );
    // **The control that makes it a measurement**: the same call with the ceiling lifted. If
    // these two ever answer the same, the clamp is not in the path.
    let uncapped =
        rope_drive(&[ahead], look, 0.0, 0.0, 1.0, Vec3::ZERO, DriveTuning { accel_max_m_s2: 1e9, ..t })
            .length();
    assert!(
        uncapped > yank * 2.0,
        "with the ceiling lifted the same geometry gave {uncapped:.1} m/s² against {yank:.1} —          `drive_accel_max_m_s2` is not being read"
    );
    // And a SMALL correction is untouched by it, which is why „man merkt es direkt"
    // (`FIND-153`) survives the weight: near the target the ramp alone governs.
    let nearly = Vec3::NEG_Z * (t.speed_m_s - 4.0);
    let small = rope_drive(&[ahead], look, 0.0, 0.0, 1.0, nearly, t).length();
    assert!(
        (small - 4.0 / t.ramp_s).abs() < 1e-2,
        "a 4 m/s gap was answered with {small:.2} m/s² instead of `4 / drive_ramp_s` = {:.2} —          the ceiling is binding where it must not",
        4.0 / t.ramp_s
    );
}

#[test]
fn f172_reversing_a_fast_flight_costs_more_time_than_starting_one_and_that_is_the_weight() {
    // *„es fühlt sich zu leicht an."* A velocity drive has no inertia by construction: without
    // a ceiling `(v* − v)/τ` replaces the whole velocity in the same ~3τ **whatever the speed
    // was**, so nothing in the game resists a direction change and mass is decoration
    // (`Forces::apply_linear_acceleration` ignores it). The ceiling is what gives that back.
    let d = game_data();
    let t = drive_tuning(&d);
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();
    let dt = 1.0 / d.game.simulation_hz as f32;

    // Ticks until the velocity along the rope reaches 90 % of the drive speed, from `start`.
    let ticks_to_speed = |start: Vec3, t: DriveTuning| {
        let mut v = start;
        for n in 1..600 {
            v += rope_drive(&[ahead], look, 0.0, 0.0, 1.0, v, t) * dt;
            if v.dot(Vec3::NEG_Z) >= 0.9 * t.speed_m_s {
                return n;
            }
        }
        panic!("the drive never reached 90 % of {} m/s", t.speed_m_s);
    };

    let from_rest = ticks_to_speed(Vec3::ZERO, t);
    // Flying away from the anchor at the drive's own top speed: twice the gap.
    let reversal = ticks_to_speed(Vec3::Z * t.speed_m_s, t);
    println!(
        "f172 weight: {} ticks ({:.0} ms) from rest · {} ticks ({:.0} ms) to reverse a {} m/s flight",
        from_rest,
        from_rest as f32 * dt * 1000.0,
        reversal,
        reversal as f32 * dt * 1000.0,
        t.speed_m_s
    );
    assert!(
        reversal as f32 > 1.6 * from_rest as f32,
        "starting from rest took {from_rest} ticks and reversing a {} m/s flight {reversal} — a          body that turns a full flight around in the time it takes to start one has no mass",
        t.speed_m_s
    );
    // **The control.** With the ceiling lifted the two collapse onto each other, because that
    // is exactly what a pure exponential does: the time constant does not know how big the gap
    // is. This is the measurement of weightlessness, and it is the game before `FIND-172`.
    let free = DriveTuning { accel_max_m_s2: 1e9, ..t };
    let (free_rest, free_reversal) = (ticks_to_speed(Vec3::ZERO, free), ticks_to_speed(Vec3::Z * t.speed_m_s, free));
    println!("f172 weight, ceiling lifted: {free_rest} ticks from rest · {free_reversal} to reverse");
    assert!(
        (free_reversal as f32) < 1.5 * free_rest as f32,
        "with the ceiling lifted, rest took {free_rest} ticks and the reversal {free_reversal} —          if those already differ, this test is measuring something other than the ceiling"
    );
}

#[test]
fn f172_a_or_d_turns_the_drive_further_sideways_than_the_rope_pulls_it_in() {
    // *„nur wenn ich a oder d drücke dass es stärker zur seite geht als rangezogen!"* — measured
    // as the two components of the velocity the drive is chasing: across the rope against along
    // it. Flown, not read off the target, so the cap and the ramp are both in the number.
    let d = game_data();
    let t = drive_tuning(&d);
    let dt = 1.0 / d.game.simulation_hz as f32;
    // The anchor straight ahead along −Z at the look, so „sideways" is +X and „inward" is −Z
    // and neither needs a projection to be read.
    let ahead = anchor_at(0.0, 40.0);
    let look = Intent::default().look_dir();

    let fly = |t: DriveTuning, move_x: f32| {
        let mut v = Vec3::ZERO;
        for _ in 0..30 {
            v += rope_drive(&[ahead], look, 0.0, move_x, 1.0, v, t) * dt;
        }
        v
    };

    let straight = fly(t, 0.0);
    let steered = fly(t, 1.0);
    let sideways = steered.x;
    let inward = steered.dot(Vec3::NEG_Z);
    println!(
        "f172 W+D after 0.5 s: {sideways:.2} m/s sideways vs {inward:.2} m/s inward          ({:.1}° off the rope) · W alone {:.1}°",
        steered.angle_between(Vec3::NEG_Z).to_degrees(),
        straight.angle_between(Vec3::NEG_Z).to_degrees()
    );
    assert!(
        sideways > inward,
        "`W`+`D` drove {sideways:.2} m/s across the rope against {inward:.2} m/s along it —          „stärker zur seite als rangezogen\" is the instruction and it is this comparison"
    );
    // **The control, and it is the one line that used to be the whole behaviour.** With the
    // fraction back at 1.0 the radial wins, which is what he was complaining about.
    let old = fly(DriveTuning { steer_pull_fraction: 1.0, ..t }, 1.0);
    assert!(
        old.dot(Vec3::NEG_Z) > old.x,
        "with `drive_steer_pull_fraction` at 1.0 the drive came out {:.2} m/s sideways and {:.2}          inward — the radial is supposed to win there, or this test is not measuring the key",
        old.x,
        old.dot(Vec3::NEG_Z)
    );
    // And the steer must not be a brake (`Q-050`): turning costs direction, never speed.
    assert!(
        steered.length() > 0.9 * straight.length(),
        "`W`+`D` reached {:.2} m/s against `W` alone at {:.2} — steering is supposed to turn the          drive, not slow it",
        steered.length(),
        straight.length()
    );
}

// ---------------------------------------------------------------------------------------
// `FIND-153` — **straighter, stricter, immediate.** The user, 2026-08-23, after playing the
// `Drive` model for the first time:
//
// > *„wenn ich mich hooke und w drücke oder generell booste dann soll ich erstmal ziemlich
// > direkt daran gezogen werden. also ziemlich gerade. außer ich move nach links (a oder rechts
// > d). **es darf „strenger" sein. also nicht so physics accurate aber mehr haptisch. also man
// > macht was und man merkt es auch direkt!**"*
//
// That is a design instruction and it outranks physical plausibility. These three tests are the
// three halves of it that a pure function can hold: **immediate** (the ramp), **straight** (the
// two ropes do not dilute each other), and **`A`/`D` steer instead of braking** (`Q-050`).
// The angle a whole flight actually makes with its rope is
// `tests/vector_rope.rs::f153_under_drive_w_pulls_the_flight_onto_the_rope_line`.
// ---------------------------------------------------------------------------------------

#[test]
fn f153_a_and_d_alone_steer_the_flight_instead_of_braking_it() {
    // `Q-050`, and it was never anybody's decision: under the full velocity chase a released
    // `W` reads as "chase 18 m/s sideways", and `scripts/f006-drive.txt` measured that as
    // 52.9 → 20.9 m/s in a single second. *„außer ich move nach links"* asks `A`/`D` to bend
    // the line, not to end the flight.
    // **A whole second of it, integrated the way `air_control` integrates it** — because that is
    // the shape `Q-050` measured (*"52.9 → 20.9 m/s in a second"*), and a single tick cannot tell
    // "steers" from "brakes slowly".
    let d = game_data();
    let t = drive_tuning(&d);
    let dt = 1.0 / d.game.simulation_hz as f32;
    let ahead = anchor_at(0.0, 60.0);
    let look = Intent::default().look_dir();

    let started_at = Vec3::NEG_Z * t.speed_m_s;
    let mut v = started_at;
    for _ in 0..d.game.simulation_hz.round() as u32 {
        v += rope_drive(&[ahead], look, 0.0, 1.0, 0.0, v, t) * dt;
    }
    assert!(
        v.length() > 0.95 * started_at.length(),
        "a second of `D` with `W` released took the flight from {:.1} m/s to {:.1} m/s — an air \
         brake nobody asked for (`Q-050`). `A`/`D` steer, they do not stop the player",
        started_at.length(),
        v.length()
    );
    let turned = v.angle_between(started_at).to_degrees();
    assert!(
        turned > 15.0,
        "a second of `D` turned the flight {turned:.1}° — *„außer ich move nach links (a oder \
         rechts d)\"* asks the key to take the player OFF the anchor line"
    );
    assert!(
        v.x > 0.0 && (v.y - started_at.y).abs() < 1e-3,
        "`D` drove the player to {v:?} — the strafe rides the horizontal look-right and never \
         the vertical"
    );
}

#[test]
fn f153_a_second_rope_does_not_halve_the_drive() {
    // The `1/n` is `rope_steer`'s **force budget** — one pull shared between two arms. A target
    // VELOCITY has no budget to share, and carrying the division over made a second hook a
    // 34 % penalty. Direction still blends between the arms; only the strength stops being an
    // average.
    let d = game_data();
    let t = drive_tuning(&d);
    let look = Intent::default().look_dir();
    let straight_ahead = anchor_at(0.0, 60.0);
    let off_to_the_right = anchor_at(-60.0_f32.to_radians(), 60.0);

    let single = rope_drive(&[straight_ahead], look, 0.0, 0.0, 1.0, Vec3::ZERO, t);
    let pair = rope_drive(&[straight_ahead, off_to_the_right], look, 0.0, 0.0, 1.0, Vec3::ZERO, t);
    assert!(
        pair.length() > 0.95 * single.length(),
        "one rope drove at {:.1} m/s² and two at {:.1} m/s² — hooking a second roof must never \
         be the slower option",
        single.length(),
        pair.length()
    );
    assert!(
        pair.x > 0.0 && pair.z < 0.0,
        "two anchors 60° apart drove the player at {pair:?} instead of between them"
    );
}

#[test]
fn f153_the_drive_is_felt_inside_a_quarter_of_a_second() {
    // *„man macht was und man merkt es auch direkt"*, as a number: **how long until 90 % of the
    // target speed**. Integrated exactly the way `air_control` integrates it — once per tick,
    // explicitly — so this is the number the player's hand actually waits for and not the
    // analytic `τ·ln 10`.
    let d = game_data();
    let t = drive_tuning(&d);
    let dt = 1.0 / d.game.simulation_hz as f32;
    let ahead = anchor_at(0.0, 60.0);
    let look = Intent::default().look_dir();

    let mut v = Vec3::ZERO;
    let mut ticks = 0u32;
    while v.length() < 0.9 * t.speed_m_s && ticks < 600 {
        v += rope_drive(&[ahead], look, 0.0, 0.0, 1.0, v, t) * dt;
        ticks += 1;
    }
    let ms = ticks as f32 * dt * 1000.0;
    // See `tests/vector_rope.rs::f153_under_drive_w_pulls_the_flight_onto_the_rope_line`: the
    // two numbers `FIND-153` answers the user with are printed, not only asserted.
    println!("f153 ms to 90 % of {:.0} m/s: {ms:.0} ms ({ticks} ticks)", t.speed_m_s);
    // ⚠️ **The band was 200 ms until `FIND-172` and it is 250 ms now, and that is a trade the
    // user asked for, not a slipped number.** `drive_accel_max_m_s2` is what makes a *large*
    // change of velocity take time — the weight behind *„es fühlt sich zu leicht an"* — and the
    // start of a flight is the largest change there is. Measured: 167 ms at `(70, 0.08, ∞)`,
    // **233 ms** at `(52, 0.08, 250)`. What „man merkt es direkt" is really about survives
    // untouched: the first 50 ms still deliver 12.5 m/s, and a small correction never touches
    // the ceiling at all (`f172_the_drive_can_never_yank_harder_than_the_files_own_ceiling`).
    assert!(
        ms <= 250.0,
        "the drive needed {ms:.0} ms to reach 90 % of {:.0} m/s — *„es darf strenger sein … man \
         macht was und man merkt es auch direkt\"*. `drive_ramp_s: 0.25` cost 567 ms of it",
        t.speed_m_s
    );
}

// ---------------------------------------------------------------------------------------
// `F-010` Slide-Dodge am Boden — „Gleit-Ausweichmanoever am Boden mit I-Frames und
// Momentum-Erhalt. Slide vermeidet Stomp-Angriff; geht fliessend in Sprint ueber."
// ---------------------------------------------------------------------------------------
//
// The three claims of the row, and each of them is a way the move can be built wrong:
//
// 1. **Momentum-Erhalt** — a slide out of a fast landing must not slow the player down. Built
//    as "set the velocity to `slide_speed_m_s`" it would be a BRAKE at every speed above 12,
//    which is every speed a swing hands over (17-21 m/s, `Q-018`).
// 2. **I-Frames** — and shorter than the slide, or the move is a dodge nobody has to time.
// 3. **geht fliessend in Sprint ueber** — when the deadline passes nothing is reset. Built as
//    "restore the velocity afterwards" it would eat the very momentum claim 1 protects.
//
// `C` and not a double-tapped `Space`: both routes reach `Buttons::DODGE`
// (`net::local::DodgeTap`), and the single key is the one a test can hold without a gesture.

fn slide_of(app: &App, e: Entity) -> defeated_by_titan::shared::Slide {
    *app.world().get::<defeated_by_titan::shared::Slide>(e).expect("a player carries a slide")
}

fn iframes_of(app: &App, e: Entity) -> u64 {
    app.world()
        .get::<defeated_by_titan::shared::Invulnerable>(e)
        .expect("a player carries an i-frame deadline")
        .until_tick
}

#[test]
fn f010_a_slide_keeps_the_speed_it_started_with_instead_of_setting_it() {
    let mut app = app();
    let e = me(&mut app);
    // 30 m/s is well above `slide_speed_m_s` (12) and is what a real swing hands over.
    launch_on_the_ground(&mut app, e, 30.0);
    let before = ground_speed(&app, e);

    hold(&mut app, KeyCode::KeyC);
    app.update();
    assert!(slide_of(&app, e).active(tick_now(&app)), "C on the ground has to start a slide");

    let during = ground_speed(&app, e);
    assert!(
        during >= before - 0.5,
        "the slide BRAKED him from {before:.2} to {during:.2} m/s — `Momentum-Erhalt` means the \
         larger of the two, and `player.slide_speed_m_s` is a FLOOR, not a speed"
    );
}

#[test]
fn f010_a_slide_below_the_floor_is_lifted_to_it_and_holds_its_direction() {
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    // Below `slide_speed_m_s` (12) and above `slide_min_speed_m_s` (3): the floor has to lift.
    launch_on_the_ground(&mut app, e, 6.0);

    hold(&mut app, KeyCode::KeyC);
    app.update();
    let during = ground_speed(&app, e);
    assert!(
        (during - d.game.player.slide_speed_m_s).abs() < 0.5,
        "a 6 m/s slide came out at {during:.2}; the floor is {}",
        d.game.player.slide_speed_m_s
    );
    // And it goes where he was going (+Z), not where the camera or the stick points.
    let v = app.world().get::<LinearVelocity>(e).unwrap().0;
    assert!(v.z > 0.0 && v.x.abs() < 0.5, "the slide left its own direction: {v:?}");
}

#[test]
fn f010_you_cannot_slide_from_standing() {
    let mut app = app();
    let e = me(&mut app);
    ticks(&mut app, 120); // land and come to rest
    assert_eq!(state(&app, e), MovementState::Grounded);
    assert!(ground_speed(&app, e) < 1.0, "he has to be still for this test to mean anything");

    hold(&mut app, KeyCode::KeyC);
    ticks(&mut app, 5);
    assert!(
        !slide_of(&app, e).active(tick_now(&app)),
        "a slide is momentum redirected, and there is nothing to redirect from standing \
         (`player.slide_min_speed_m_s`)"
    );
    assert_eq!(iframes_of(&app, e), 0, "and it must not hand out free i-frames either");
}

#[test]
fn f010_the_i_frames_are_shorter_than_the_slide_and_both_come_out_of_the_file() {
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    launch_on_the_ground(&mut app, e, 20.0);

    hold(&mut app, KeyCode::KeyC);
    app.update();
    let started = tick_now(&app);
    let s = slide_of(&app, e);
    let hz = d.game.simulation_hz as f32;
    let want_slide = (d.game.player.slide_duration_s * hz).round() as u64;
    let want_iframes = (d.game.player.slide_iframes_s * hz).round() as u64;

    assert!(
        s.until_tick.abs_diff(started + want_slide) <= 1,
        "the slide ends at {} and `slide_duration_s` says {}",
        s.until_tick,
        started + want_slide
    );
    assert!(
        iframes_of(&app, e).abs_diff(started + want_iframes) <= 1,
        "the i-frames end at {} and `slide_iframes_s` says {}",
        iframes_of(&app, e),
        started + want_iframes
    );
    assert!(
        iframes_of(&app, e) < s.until_tick,
        "the i-frames ({}) must end BEFORE the slide ({}) — the tail is what carries him out \
         from under the foot, and a window as long as the movement is a dodge nobody has to time",
        iframes_of(&app, e),
        s.until_tick
    );
}

#[test]
fn f010_a_slide_flows_into_the_run_instead_of_being_reset() {
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    launch_on_the_ground(&mut app, e, 25.0);

    hold(&mut app, KeyCode::KeyC);
    app.update();
    release(&mut app, KeyCode::KeyC);
    let slide_ticks = (d.game.player.slide_duration_s * d.game.simulation_hz as f32).round() as u64;
    ticks(&mut app, slide_ticks + 2);

    assert!(
        !slide_of(&app, e).active(tick_now(&app)),
        "the slide has to be over by now"
    );
    // Nothing is reset when it ends: `ground_step`'s μg brake takes over from wherever the
    // slide left him, and one tick of that is 20/60 = 0.33 m/s. Anything near zero would mean
    // the velocity was thrown away at the end of the move.
    let after = ground_speed(&app, e);
    assert!(
        after > d.game.player.run_speed_m_s,
        "he came out of the slide at {after:.2} m/s — a slide that ends by resetting the \
         velocity eats the very momentum `Momentum-Erhalt` is about"
    );
}

#[test]
fn f010_the_cooldown_is_measured_from_the_start_and_includes_the_slide() {
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    // 8 m/s and not 25: at 25 the slide covers 13.75 m and the graybox has buildings in it,
    // so the second half of this test would be measuring what he ran into rather than the
    // clock. The cooldown does not care how fast the slide was.
    launch_on_the_ground(&mut app, e, 8.0);

    // ⚠️ **A held `C` fires ONCE.** `net::local::DodgeTap::feed` makes both routes to
    // `Buttons::DODGE` an EDGE — a held key that produced sixty dodges a second would empty a
    // tank in seven ticks, and the rate limit is that edge. So this test cannot "hold C and
    // wait"; it has to let go and press again, which is what a player does anyway.
    hold(&mut app, KeyCode::KeyC);
    // **`W` as well, and it is not decoration.** Without it the first run of this test was red
    // for a reason that has nothing to do with the cooldown: after the slide ends, `ground_step`
    // brakes an unsteered player at μg = 20 m/s², so 25 m/s is gone in 75 ticks — and by the
    // time the 54-tick cooldown is up he is below `slide_min_speed_m_s` (3) and could not have
    // slid again whatever the cooldown said. Holding `W` keeps him at `run_speed_m_s`, which is
    // above the floor, so the only thing left that can refuse the second slide is the clock.
    hold(&mut app, KeyCode::KeyW);
    app.update();
    let first = slide_of(&app, e).started_at_tick.expect("the first slide started");

    // One whole slide plus a tick: the move is over, the cooldown is not — and the player
    // presses again, properly, with a release in between so the edge really fires.
    let slide_ticks = (d.game.player.slide_duration_s * d.game.simulation_hz as f32).round() as u64;
    release(&mut app, KeyCode::KeyC);
    ticks(&mut app, slide_ticks);
    hold(&mut app, KeyCode::KeyC);
    ticks(&mut app, 2);
    assert_eq!(
        slide_of(&app, e).started_at_tick,
        Some(first),
        "a second press re-entered the slide the tick it ended — `slide_cooldown_s` ({} s) is \
         measured from the START and therefore includes the {} s of sliding",
        d.game.player.slide_cooldown_s,
        d.game.player.slide_duration_s,
    );

    // …and past the cooldown it may start again — on a fresh press, because the button is an
    // edge (see above).
    let cooldown_ticks =
        (d.game.player.slide_cooldown_s * d.game.simulation_hz as f32).round() as u64;
    release(&mut app, KeyCode::KeyC);
    ticks(&mut app, cooldown_ticks);
    hold(&mut app, KeyCode::KeyC);
    ticks(&mut app, 2);
    assert!(
        slide_of(&app, e).started_at_tick.is_some_and(|t| t > first),
        "past the cooldown a held C has to slide again, and it did not: state={:?} speed={:.2}          tick={} first={first} cooldown={cooldown_ticks}",
        state(&app, e),
        ground_speed(&app, e),
        tick_now(&app),
    );
}

fn tick_now(app: &App) -> u64 {
    app.world().resource::<defeated_by_titan::shared::Tick>().0
}

// ---------------------------------------------------------------------------------------
// `F-005` UNDER THE DRIVE — the winch. `Q-050`'s dead key, given a job (2026-08-25)
//
// `Drive` builds no `DistanceJoint` (`FIND-152`), so `player::rope::shorten_ropes` never sees
// the rope: `Ctrl` moved nobody while `vector::gas` billed `gas_reel_per_s` for it. What it
// does now is [`rope_winch`] — closing speed along the rope, no look gate, stops at
// `min_rope_m`. The four tests below are its four properties, and the fifth is the wiring.
// ---------------------------------------------------------------------------------------

/// Anchors the left arm on a body that is **further away than `min_rope_m`**, and hands back
/// the real hand-to-tip vector after `vector::hook::update_hooks` has written `tip_m`.
///
/// 🔴 **Not `a_real_body`, and that is a fixture bug both winch tests made.** The first body in
/// the index is the graybox **ground**, whose centre sits **1.70 m** from the hand — inside
/// `vector.min_rope_m`, so the winch correctly refuses and the test measures its own fixture.
/// `f005_under_the_pendulum_ctrl_adds_no_acceleration_at_all` was **green with the model fork
/// deleted** because of exactly that (Rule 5, 2026-08-25): it was asserting `ZERO` against a
/// rope that was already at the floor. Writing `tip_m` by hand does not help either —
/// `update_hooks` is its one writer and overwrites it inside the same tick, one system set
/// ahead of `air_control`.
fn anchor_on_a_body_with_rope_left(app: &mut App, e: Entity, d: &GameData) -> Vec3 {
    let body = {
        let hand = at(app, e) + Vec3::Y * d.game.player.eye_height_m;
        let index = app.world().resource::<SpatialIndex>();
        (1..200)
            .map(BodyId)
            .filter_map(|id| index.body(id).map(|entry| (id, entry.center_m)))
            .map(|(id, c)| (id, (c - hand).length()))
            .filter(|(_, len)| {
                *len > d.game.vector.min_rope_m + 5.0 && *len < d.game.vector.hook_range_m
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("the graybox has to hold one body between 8 m and a hook range away")
            .0
    };
    hold(app, KeyCode::KeyQ); // or `update_hooks` lets go of the arm on the next tick
    app.world_mut().get_mut::<Hook>(e).expect("two hooks").arms[Side::Left.index()].state =
        HookState::Anchored { body, local_m: Vec3::ZERO };
    app.update(); // `update_hooks` is the one writer of `tip_m` — it puts it on the body

    let hand_m = at(app, e) + Vec3::Y * d.game.player.eye_height_m;
    let tip_m = app.world().get::<Hook>(e).unwrap().arm(Side::Left).tip_m;
    let to_anchor = tip_m - hand_m;
    assert!(
        to_anchor.length() > d.game.vector.min_rope_m + 1.0,
        "the fixture needs a rope with room left in it, not one already at the floor: {:.2} m",
        to_anchor.length()
    );
    to_anchor
}

/// A **strong, unbounded** winch — the shape the four property tests below are about.
///
/// ⚠️ **This is no longer a tuning the game ever passes.** Until `Q-058` it was `Ctrl`'s
/// (`reel_speed_m_s`, `drive_ramp_s`, no ceiling); `Ctrl` shortens the joint now
/// (`player::locomotion::rope_winch`'s header), so the only caller in the game is the always-on
/// pull and its numbers are [`idle_pull_tuning`]'s. The values are kept because the four
/// properties — never brakes, rope axis only, stops at `min_rope_m`, no look gate — are
/// statements about the **function**, and an unbounded, fast winch is where each of them is
/// easiest to falsify.
fn winch_tuning(d: &GameData) -> WinchTuning {
    WinchTuning {
        speed_m_s: d.game.vector.reel_speed_m_s,
        min_rope_m: d.game.vector.min_rope_m,
        ramp_s: d.game.vector.drive_ramp_s,
        accel_max_m_s2: f32::INFINITY,
    }
}

/// The **always-on** pull's tuning (`FIND-172`) — the same function, a lower speed, a longer
/// ramp and a ceiling that is derived from the two of them.
fn idle_pull_tuning(d: &GameData) -> WinchTuning {
    WinchTuning {
        speed_m_s: d.game.vector.drive_idle_speed_m_s,
        min_rope_m: d.game.vector.min_rope_m,
        ramp_s: d.game.vector.drive_idle_ramp_s,
        accel_max_m_s2: d.game.vector.drive_idle_speed_m_s / d.game.vector.drive_idle_ramp_s,
    }
}

#[test]
fn f172_the_always_on_pull_never_hauls_harder_than_it_does_from_a_standing_start() {
    // 🔴 **The trap this round actually fell into.** `rope_winch`'s property 1 says the winch
    // can never brake — and that is a statement about the CLOSING SPEED, not about the
    // acceleration. A player flying *away* from his anchor is a gap of `speed + |v|`, and
    // divided by a ramp that is a haul nobody sized.
    //
    // Measured before the ceiling existed: `f004_the_ground_does_not_write_the_velocity_of_a_
    // player_the_rope_drags` handed a player to the rope at **29.67 m/s** and found him at
    // **0.00 m/s** half a second later — the always-on pull had reversed him.
    let d = game_data();
    let t = idle_pull_tuning(&d);
    let to_anchor = Vec3::NEG_Z * 40.0;

    let from_rest = rope_winch(&[to_anchor], Vec3::ZERO, t).length();
    let outbound = rope_winch(&[to_anchor], Vec3::Z * 30.0, t).length();
    assert!(
        (outbound - from_rest).abs() < 1e-3,
        "flying away at 30 m/s the always-on pull hauled at {outbound:.1} m/s² against the          {from_rest:.1} m/s² it uses from rest — „es ist zu aggressiv\" is what that is"
    );
    // **The control**: with the ceiling lifted the two come apart, which is the game before
    // this ceiling and the shape of the measured 29.67 → 0.00.
    let free = rope_winch(&[to_anchor], Vec3::Z * 30.0, WinchTuning { accel_max_m_s2: 1e9, ..t })
        .length();
    assert!(
        free > from_rest * 2.0,
        "with the ceiling lifted, flying away at 30 m/s gave {free:.1} m/s² against {from_rest:.1}          from rest — if those match, `accel_max_m_s2` is not in the path"
    );
    // And it still does its job: from rest it is exactly `idle / ramp` along the rope.
    assert!(
        (from_rest - t.speed_m_s / t.ramp_s).abs() < 1e-3,
        "from rest the always-on pull gave {from_rest:.2} m/s² instead of          `drive_idle_speed_m_s / drive_idle_ramp_s` = {:.2}",
        t.speed_m_s / t.ramp_s
    );
}

#[test]
fn f005_the_winch_can_never_brake_a_flight_that_is_already_closing_faster() {
    // Property 1, and it is the one that keeps `Q-050`'s other half from coming back in a new
    // key: the released-`W` chase read a 52.9 m/s flight as an error and killed it (`FIND-153`).
    // A winch that *sets* the closing speed would do the same at 28.
    //
    // Red in one line: drop the `if closing_m_s >= t.speed_m_s { return ZERO }` guard — the term
    // then comes out **negative** along the rope, i.e. a brake of (28 − 40)/0.08 = −150 m/s².
    let d = game_data();
    let t = winch_tuning(&d);
    let along = Vec3::new(0.0, 1.0, 0.0);
    let to_anchor = along * 40.0;

    let fast = along * (t.speed_m_s + 12.0);
    assert_eq!(
        rope_winch(&[to_anchor], fast, t),
        Vec3::ZERO,
        "a player already closing at {} m/s must get nothing from the winch, not a brake",
        t.speed_m_s + 12.0
    );

    // And the control, so the assertion above can tell the two apart: half that speed and the
    // winch really does push.
    let slow = along * (t.speed_m_s * 0.5);
    let a = rope_winch(&[to_anchor], slow, t);
    let due = (t.speed_m_s * 0.5) / t.ramp_s;
    assert!(
        (a - along * due).length() < 1e-3,
        "at half the winch speed the term has to be {due:.1} m/s² along the rope, not {a:?}"
    );
}

#[test]
fn f005_the_winch_touches_the_rope_axis_and_leaves_the_crossing_momentum_alone() {
    // Property 2 — the half of the pendulum's reel that survives without a constraint.
    // `shared::rope::rope_reel_in` scales the **tangential** velocity while it shortens; the
    // winch cannot scale it, but it must not eat it either, or reeling in the middle of a swing
    // would be a brake wearing a reel's name.
    //
    // Red by returning `(along * t.speed_m_s - velocity_m_s) / t.ramp_s` — the whole-velocity
    // chase — which puts 750 m/s² across the rope to kill the 60 m/s of crossing flight.
    let d = game_data();
    let t = winch_tuning(&d);
    let along = Vec3::new(0.0, 1.0, 0.0);
    let across = Vec3::new(0.0, 0.0, -1.0);
    let v = across * 60.0; // pure crossing momentum, nothing along the rope at all

    let a = rope_winch(&[along * 40.0], v, t);

    assert!(
        a.dot(across).abs() < 1e-4,
        "the winch put {:.4} m/s² across the rope — it may only ever act along it: {a:?}",
        a.dot(across)
    );
    let due = t.speed_m_s / t.ramp_s;
    assert!(
        (a.dot(along) - due).abs() < 1e-3,
        "and along it exactly (28 − 0)/ramp = {due:.1} m/s², measured {:.1}",
        a.dot(along)
    );
}

#[test]
fn f005_the_winch_stops_at_min_rope_m_and_the_arm_inside_it_contributes_nothing() {
    // Property 3 — the same floor `player::rope::shorten_ropes` clamps `limits.max` to, so the
    // two models stop at the same distance. Without it a player who has arrived is driven into
    // the wall he is hanging on, and `try_normalize` eventually hands back a direction built out
    // of numerical noise.
    //
    // Red by deleting the `if to_anchor.length() <= t.min_rope_m { continue }`.
    let d = game_data();
    let t = winch_tuning(&d);
    let up = Vec3::Y;

    let inside = up * (t.min_rope_m - 0.5);
    assert_eq!(
        rope_winch(&[inside], Vec3::ZERO, t),
        Vec3::ZERO,
        "an arm {:.1} m from its anchor is at the floor and winds in nothing",
        t.min_rope_m - 0.5
    );

    let outside = up * (t.min_rope_m + 0.5);
    assert_ne!(
        rope_winch(&[outside], Vec3::ZERO, t),
        Vec3::ZERO,
        "half a metre further out it has to pull again — otherwise this test passes because \
         the winch is broken, not because the floor works"
    );

    // Two arms, one of each: the far one still winds in. That is the rule the gas gate copies.
    let both = rope_winch(&[inside, outside], Vec3::ZERO, t);
    assert!(
        (both - rope_winch(&[outside], Vec3::ZERO, t)).length() < 1e-4,
        "with one arm at the floor and one out past it, the far arm alone decides: {both:?}"
    );
}

#[test]
fn f005_ctrl_never_adds_an_acceleration_to_the_body_under_either_model() {
    // 🔴 **`Q-058`, 2026-08-27 — this test replaces two, and one of them asserted the
    // opposite.** Until this day `RopeForceModel::Drive` built no `DistanceJoint`, so
    // `player::rope::shorten_ropes` never saw a `Drive` rope, `Ctrl` was a dead key that
    // `vector::gas` still billed (`Q-050`), and `player::locomotion::rope_winch` was given the
    // job as a body acceleration at `reel_speed_m_s`. The old test measured exactly that:
    // `f005_under_the_drive_ctrl_winds_in_a_player_who_is_standing_still_on_the_ground`,
    // `(reel_speed − closing)/drive_ramp_s` along the rope.
    //
    // **A `Drive` rope has a joint now**, so `Ctrl` is the joint's reel under both models —
    // one key, one mechanism, with the `L_prev/L_new` amplification that is the whole feel of
    // the Vector Gear (`player::rope`'s header: 58.23 m/s out of `v0 = 20`, against 20.000 for
    // an acceleration). Keeping both would pay the reel once and deliver it twice, which is the
    // sentence the surviving half of this test has always made about `Pendulum`.
    //
    // **What `Ctrl` DOES do now is measured where a real joint exists**:
    // `tests/vector_rope.rs::q058_under_drive_ctrl_shortens_the_joint_exactly_as_under_pendulum`.
    // This fixture forces `HookState::Anchored` by hand and writes no `HookAnchored`, so
    // `attach_ropes` never runs and there is no joint here to measure — which is precisely why
    // the claim this test can still make is a claim about `air_control` and nothing else.
    //
    // Red by putting `grant.reel_in` back into the winch's `match` in `air_control`.
    for model in [
        defeated_by_titan::data::RopeForceModel::Drive,
        defeated_by_titan::data::RopeForceModel::Pendulum,
    ] {
        let mut app = app();
        app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model = model;
        ticks(&mut app, 30); // the player lands and settles on the graybox floor
        let e = me(&mut app);
        let d = data(&app);
        assert_eq!(
            state(&app, e),
            MovementState::Grounded,
            "{model:?}: the fixture is a player STANDING — if he is airborne this proves nothing"
        );
        // ⚠️ **A rope with room left in it.** A rope already at `min_rope_m` would make the
        // assertion pass whatever the `match` says — the helper asserts that itself.
        let to_anchor = anchor_on_a_body_with_rope_left(&mut app, e, &d);
        hold(&mut app, KeyCode::ControlLeft);
        app.update();

        assert_eq!(
            run_accel(&app, e),
            Vec3::ZERO,
            "{model:?}: `Ctrl` on a {:.1} m rope put an acceleration on the BODY. The reel is a \
             change of `limits.max` and `player::rope::shorten_ropes` is its one writer — a \
             second, additive haul here pays it once and delivers it twice",
            to_anchor.length()
        );
    }
}

// ---------------------------------------------------------------------------------------
// F-012 — THE MAP HAS AN EDGE. The user, 2026-08-27: *„und man kann an der seite einfach
// runterfallen!"*, and asked what should be there he named BOTH mechanisms:
// *„unsichtbare wand + wenn man runterfaellt wegen bug teleport man zurueck!"*
//
// The two are not one thing twice. The fence stops normal play from leaving; the recovery
// works **after the fence has already failed** — a warp, a seam, a tunnel at 75 m/s. The
// control below proves the recovery cannot fire on a legitimate 120 m dive.
// ---------------------------------------------------------------------------------------

/// Where the world's floor is, measured out of the map that is actually built, and never
/// typed as a literal: the lowest underside of any block `world::map` planned.
fn deepest_floor_m(app: &App) -> f32 {
    let d = data(app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    defeated_by_titan::world::map::plan_blocks(&d, &map)
        .iter()
        .map(|b| b.center_m.y - b.size_m.y * 0.5)
        .fold(f32::INFINITY, f32::min)
}

/// Puts the body exactly there, the way a bug does — **not** through `WarpPlayer`, so that a
/// test of the recovery cannot be answered by the warp path the recovery itself uses.
fn put_body_at(app: &mut App, e: Entity, pos: Vec3) {
    app.world_mut()
        .entity_mut(e)
        .get_mut::<Transform>()
        .expect("a player has a transform")
        .translation = pos;
    app.world_mut()
        .entity_mut(e)
        .get_mut::<LinearVelocity>()
        .expect("a player has a velocity")
        .0 = Vec3::ZERO;
}

/// How many `WarpPlayer` messages the last tick produced. The recovery's only visible act.
fn warps_sent(app: &App) -> usize {
    app.world()
        .resource::<Messages<defeated_by_titan::shared::WarpPlayer>>()
        .iter_current_update_messages()
        .count()
}

#[test]
fn f012_a_body_dropped_past_the_edge_of_the_map_is_brought_back_into_it() {
    // ## The repro of `B-015`, and the case the fence provably cannot answer.
    //
    // The body is **put** past the edge, not driven there — the fence is already behind it,
    // which is exactly the situation the user asked the second mechanism for
    // (*„wenn man runterfaellt wegen bug"*). Nothing here may depend on the fence at all.
    //
    // **In its first form this test asserted only that nothing catches him within 2 s, and it
    // read 12 of 12 at `y = -44.0` — free fall at `gravity_m_s2 = -32`, nothing hit.** That is
    // the measurement in `docs/BUGS.md` B-015, and it identified the cause out of the four
    // candidates: not a plate smaller than the playable area, not a seam and not tunnelling —
    // **past the plate there is no collider at any height.** The horizon is now long enough to
    // reach the recovery plane, and what is asserted is that he comes back.
    //
    // What the answer depends on: `bounds.recovery_plane_y_m`, `recovery_lift_m`, `SafeGround`,
    // `gravity_m_s2` (how long the fall takes) and the map's `size_m`.
    // What this sweep varies: the sign and the axis of the offset (4 directions) x the distance
    // past the edge (1, 5, 25 m) = 12 samples. **None is skipped** — every one is asserted, and
    // the failure list below carries the count so a silent exclusion cannot hide in it.
    // What it holds constant, and why: the height he is dropped from (20 m — the fall to the
    // plane dominates it by a factor of sixteen) and the map (`current`, the district that
    // ships, because the claim is about the shipped world).
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let floor = deepest_floor_m(&app);
    let plane = map.bounds.recovery_plane_y_m;

    // He has to have stood somewhere first, or this measures the spawn point instead of the
    // recorded ground.
    put_body_at(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 60);
    assert_eq!(state(&app, e), MovementState::Grounded, "the fixture needs a recorded ground");
    let safe = at(&app, e);

    // 20 m up, falling to -300 m at 32 m/s^2 is sqrt(2 * 320 / 32) = 4.47 s = 269 ticks.
    let horizon = 400;
    let mut lost = Vec::new();
    for (label, dir) in [("+x", Vec3::X), ("-x", Vec3::NEG_X), ("+z", Vec3::Z), ("-z", Vec3::NEG_Z)]
    {
        for past_m in [1.0_f32, 5.0, 25.0] {
            let edge = Vec3::new(hx * dir.x, 20.0, hz * dir.z);
            put_body_at(&mut app, e, edge + dir * past_m);
            let mut deepest = f32::INFINITY;
            for _ in 0..horizon {
                app.update();
                deepest = deepest.min(at(&app, e).y);
            }
            let p = at(&app, e);
            if p.y < floor - 5.0 || p.x.abs() > hx + 1.0 || p.z.abs() > hz + 1.0 {
                lost.push(format!(
                    "{label} +{past_m} m -> {p:?} (deepest {deepest:.1} m, plane {plane} m)"
                ));
            }
        }
    }

    assert!(
        lost.is_empty(),
        "{} of 12 bodies put one to twenty-five metres past the {} x {} m map were still out          of the world after {horizon} ticks. The ground they should have come back to is          {safe:?}, the deepest block underside is {floor:.2} m: {lost:?}",
        lost.len(),
        map.size_m.0,
        map.size_m.1,
    );
}

#[test]
fn f012_walking_at_the_edge_of_the_map_does_not_walk_you_off_it() {
    // Walking speed, the four edges. What the fence depends on: `bounds` out of `maps.ron`
    // and the map's `size_m`. What this varies: which of the four edges, and therefore both
    // signs of both horizontal axes. Held constant, and named: the speed (the legs' own
    // `run_speed_m_s` — the speed case is the next test) and the map.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let floor = deepest_floor_m(&app);

    for (label, key, from, dir) in [
        ("+x", KeyCode::KeyD, Vec3::new(hx - 12.0, 2.0, 0.0), Vec3::X),
        ("-x", KeyCode::KeyA, Vec3::new(-hx + 12.0, 2.0, 0.0), Vec3::NEG_X),
        ("-z", KeyCode::KeyW, Vec3::new(0.0, 2.0, -hz + 12.0), Vec3::NEG_Z),
        ("+z", KeyCode::KeyS, Vec3::new(0.0, 2.0, hz - 12.0), Vec3::Z),
    ] {
        put_body_at(&mut app, e, from);
        ticks(&mut app, 10);
        hold(&mut app, key);
        ticks(&mut app, 240); // 4 s — more than the 12 m at 6 m/s needs
        release(&mut app, key);
        ticks(&mut app, 10);

        let p = at(&app, e);
        let out = p.dot(dir);
        assert!(
            p.y > floor - 5.0,
            "{label}: walking into the edge dropped the player to y = {:.2} (world floor \
             {floor:.2} m)",
            p.y
        );
        assert!(
            out <= hx.max(hz) + 1.0,
            "{label}: walking carried the player {out:.2} m out along {dir:?}, past the \
             {:.0} m half-extent of the map",
            hx.max(hz)
        );
    }
}

#[test]
fn f012_flying_at_the_edge_at_top_speed_does_not_tunnel_through_it() {
    // The tunnelling case, and it is the one a collider can lose: `vector.max_speed_m_s`
    // straight at the fence. What the answer depends on: `game.ron: substeps` and
    // `player.max_speed_m_s`, and the fence's own thickness out of `maps.ron: bounds`.
    // What this varies: the four edges and the sign of the vertical component (level, and
    // 30 deg downwards — a diagonal is what a rope release actually produces).
    // Held constant and named: the launch height (40 m, above every roof at the edge so
    // that a house cannot do the fence's work for it).
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let v_max = d.game.vector.max_speed_m_s;
    let floor = deepest_floor_m(&app);

    let mut escaped = Vec::new();
    for (label, from, dir) in [
        ("+x", Vec3::new(hx - 60.0, 40.0, 0.0), Vec3::X),
        ("-x", Vec3::new(-hx + 60.0, 40.0, 0.0), Vec3::NEG_X),
        ("+z", Vec3::new(0.0, 40.0, hz - 60.0), Vec3::Z),
        ("-z", Vec3::new(0.0, 40.0, -hz + 60.0), Vec3::NEG_Z),
    ] {
        for (down_label, down) in [("level", 0.0_f32), ("30deg-down", -0.5)] {
            put_body_at(&mut app, e, from);
            app.update();
            let v = (dir + Vec3::Y * down).normalize() * v_max;
            app.world_mut().entity_mut(e).get_mut::<LinearVelocity>().expect("velocity").0 = v;
            ticks(&mut app, 60); // 1 s at 75 m/s is 75 m — well past the 60 m of run-up
            let p = at(&app, e);
            let out = p.dot(dir);
            if out > hx.max(hz) + 2.0 || p.y < floor - 5.0 {
                escaped.push(format!("{label}/{down_label} -> {out:.2} m out, y = {:.2}", p.y));
            }
        }
    }

    assert!(
        escaped.is_empty(),
        "{} of 8 launches at {v_max} m/s went through the fence: {escaped:?}",
        escaped.len()
    );
}

#[test]
fn f012_a_player_below_the_world_comes_back_to_where_he_last_stood() {
    // The RECOVERY, and it is deliberately driven with the fence already defeated: the body
    // is put below the plane by hand, exactly the way a seam or a bad warp would.
    // What the answer depends on: `bounds.recovery_plane_y_m`, the recorded safe ground, and
    // that `apply_warps` really moves the body. What this varies: the horizontal place he
    // last stood (three of them) and how far below the plane he starts (2 m and 90 m).
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let plane = map.bounds.recovery_plane_y_m;

    for stood_at in [Vec3::new(0.0, 2.0, 0.0), Vec3::new(40.0, 2.0, 40.0), Vec3::new(-30.0, 2.0, 20.0)]
    {
        put_body_at(&mut app, e, stood_at);
        ticks(&mut app, 60); // he lands and is GROUNDED — that is what gets recorded
        assert_eq!(
            state(&app, e),
            MovementState::Grounded,
            "the fixture needs him STANDING at {stood_at:?} — otherwise nothing is recorded \
             and the assertion below is about the spawn point"
        );
        let safe = at(&app, e);

        for below_m in [2.0_f32, 90.0] {
            put_body_at(&mut app, e, Vec3::new(safe.x, plane - below_m, safe.z));
            ticks(&mut app, 8);
            let p = at(&app, e);
            assert!(
                (p.xz() - safe.xz()).length() < 2.0 && p.y > plane + 100.0,
                "dropped {below_m} m under the recovery plane ({plane} m) he came back to \
                 {p:?}, not to the ground he last stood on ({safe:?})"
            );
        }
    }
}

#[test]
fn f012_a_dive_from_the_top_of_the_wall_is_not_a_fall_out_of_the_world() {
    // ## THE CONTROL. A legitimate 120 m dive must NOT be recovered.
    //
    // The rule that tells the two apart is **depth, not fall distance and not speed**: the
    // recovery plane lies far below the deepest block in the map, and no legitimate surface
    // reaches it. This test proves the discriminator holds for the longest drop the district
    // has — off the coping of the 120 m wall.
    //
    // What the answer depends on: `bounds.recovery_plane_y_m` and the height of the wall.
    // What this varies: nothing — it is one dive, and it is the deepest one Ashgate offers.
    // What would make it vacuous, and is therefore asserted: that he really fell (>100 m of
    // it) and really landed (Grounded at the end). An assertion satisfied by a body that
    // never moved is not an assertion.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let plane = map.bounds.recovery_plane_y_m;

    // Stand him on the ground first, so there IS a recorded safe spot to be wrongly sent to.
    put_body_at(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 60);
    let ground = at(&app, e);
    assert_eq!(state(&app, e), MovementState::Grounded, "the control needs a recorded ground");

    // Off the coping: `maps.ron` puts it at y = 119 +- 1, so 121 m is one step above it.
    let top = Vec3::new(0.0, 121.0, -60.0);
    put_body_at(&mut app, e, top);
    let mut warps = 0usize;
    let mut lowest = f32::INFINITY;
    for _ in 0..300 {
        app.update();
        warps += warps_sent(&app);
        lowest = lowest.min(at(&app, e).y);
    }
    let p = at(&app, e);

    assert!(top.y - lowest > 100.0, "the dive has to BE a dive: it fell {:.1} m", top.y - lowest);
    assert!(lowest > plane, "a 120 m dive reached {lowest:.1} m, under the plane at {plane} m");
    assert_eq!(warps, 0, "a legitimate dive sent {warps} recovery warps, and must send none");
    assert!(
        (p.xz() - Vec2::new(top.x, top.z)).length() < 15.0,
        "he landed at {p:?} — a recovery would have put him back at {ground:?}"
    );
    assert_eq!(state(&app, e), MovementState::Grounded, "after 5 s the dive has to be over");
}

/// The eight bearings a rectangle can be left by: four flats and four corners. The corner is
/// the seam between two fence panels and the flat is the middle of one, and a rule that reads
/// `x` and `z` separately can be right on one and wrong on the other.
fn eight_ways_out(hx: f32, hz: f32) -> [(&'static str, Vec3); 8] {
    [
        ("+x", Vec3::new(hx, 0.0, 0.0)),
        ("-x", Vec3::new(-hx, 0.0, 0.0)),
        ("+z", Vec3::new(0.0, 0.0, hz)),
        ("-z", Vec3::new(0.0, 0.0, -hz)),
        ("+x+z", Vec3::new(hx, 0.0, hz)),
        ("+x-z", Vec3::new(hx, 0.0, -hz)),
        ("-x+z", Vec3::new(-hx, 0.0, hz)),
        ("-x-z", Vec3::new(-hx, 0.0, -hz)),
    ]
}

/// Is this body inside the map's own footprint, and above the world's floor? The one question
/// every test below asks about a final position, and it is asked about `map.size_m` alone —
/// never about the fence, whose margin is exactly the latent bug in `record_safe_ground`.
fn back_in_the_world(p: Vec3, hx: f32, hz: f32, floor_m: f32) -> bool {
    p.x.abs() <= hx + 1.0 && p.z.abs() <= hz + 1.0 && p.y > floor_m - 5.0
}

#[test]
fn f012_the_top_of_the_fence_is_not_a_ring_you_can_stand_on_outside_the_map() {
    // ## The repro, measured 2026-08-28 in the round that refuted the first build
    //
    // A body put at `(355, 210, 0)` — 5 m past the 350 m edge, 10 m above `fence_top_m` — came
    // to rest at **exactly y = 200.000** and was still there 14 s later. The same at the corner
    // `(355, 210, 355)`, and 8 s of held `W` along it left him at 200.000.
    //
    // ## The two lists, because a sweep's size is not its coverage
    //
    // What the rule reads: the player's `Transform` (**all three** components),
    // `map.size_m` (both axes), `bounds.recovery_plane_y_m`, and — this is the defect — nothing
    // at all about how high above the plane he is.
    // ## 🔴 AND THE AXIS IT HELD CONSTANT WAS THE DEFECT — the fifth time in this project.
    //
    // Until 2026-08-29 the three distances below were `0.5`, `t/3` and `2t/3` metres, **all
    // strictly outward**, under a comment that said *"What it skips: nothing."* The one
    // distance it never sampled was **zero** — and `out_of_the_world` tests `|x| > hx`,
    // STRICTly, while `maps.ron` shipped `fence_margin_m: 0.0`, so the fence's inner lip stood
    // exactly ON `hx`. `warp 350 201 0` rested at 200.000 m after 3 s and after 10 s and after
    // six seconds of held `W`: a solid, invisible, standable floor the rule called IN THE
    // WORLD. The bug lived in the single sample the fixture's own `across` list could not
    // produce.
    //
    // What this sweep varies: 8 bearings out of the map (4 flats **and** 4 corners, the seam
    // between two panels), and **8 distances across the top face**, derived and not typed:
    // one millimetre INSIDE the boundary, one ULP inside, **exactly zero**, one ULP outside,
    // one millimetre outside, then half a metre, a third of the way across the face and two
    // thirds. 64 samples. The three in the middle are the ones that were missing, and one ULP
    // at 350 m is 2^-15 = 30.5 micrometres — the smallest step the boundary can be crossed by.
    // What it holds constant, and why: the drop height, at `fence_top_m + 10 m` — the point of
    // this test is the *resting* place, and every drop lands on the same face. The height axis
    // itself is swept in `f012_the_map_footprint_is_the_world_at_every_height`.
    // What it skips: nothing, and there is no `continue` in it. `checked` is counted and
    // asserted against the sweep's own size, so a skipped class cannot hide in the denominator.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let b = map.bounds.clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let floor = deepest_floor_m(&app);
    let t = b.fence_thickness_m;

    let mut parked = Vec::new();
    let mut checked = 0usize;
    for (label, on_edge) in eight_ways_out(hx, hz) {
        let dir = Vec3::new(on_edge.x.signum() * on_edge.x.abs().min(1.0), 0.0, on_edge.z.signum() * on_edge.z.abs().min(1.0))
            .normalize();
        // One ULP at the boundary itself, read out of the float and never typed: at 350 m
        // that is 2^-15 m, and it is exactly the step that separates "on the line" from
        // "past it" for the `>` in `out_of_the_world`.
        let ulp = f32::from_bits(hx.to_bits() + 1) - hx;
        for across in [-0.001_f32, -ulp, 0.0, ulp, 0.001, 0.5, t / 3.0, 2.0 * t / 3.0] {
            checked += 1;
            let from = Vec3::new(on_edge.x, b.fence_top_m + 10.0, on_edge.z) + dir * across;
            put_body_at(&mut app, e, from);
            ticks(&mut app, 240); // 4 s: 0.8 s of falling, then 3.2 s of standing still
            let p = at(&app, e);
            // 🔴 TWO conditions, and the second one was missing until the control run of
            // 2026-08-29 found it. `back_in_the_world` allows `hx + 1.0` of slack — it has to,
            // it is the shared helper every F-012 fixture judges a final position with — and a
            // body parked on the fence's inner lip at exactly `hx` satisfies it. So with
            // `fence_margin_m` put back to 0.0 this test stayed **green** on a build where
            // `warp 350 201 0` demonstrably parked at 200.000 forever. The face's own height is
            // the discriminator that slack cannot swallow.
            let on_the_face = (p.y - b.fence_top_m).abs() < 1.0;
            if !back_in_the_world(p, hx, hz, floor) || on_the_face {
                parked.push(format!(
                    "{label} +{across:.2} m from {from:?} -> {p:?}{}",
                    if on_the_face { " — STILL ON THE FENCE'S TOP FACE" } else { "" }
                ));
            }
        }
    }

    assert_eq!(checked, 64, "the sweep skipped samples");
    assert!(
        parked.is_empty(),
        "{} of {checked} bodies dropped onto the top face of the fence are still outside the \
         {} x {} m map after 4 s. The fence top is a solid ring at y = {} m and it lies OUTSIDE \
         the footprint, so nothing records it and nothing recovers from it: {parked:?}",
        parked.len(),
        map.size_m.0,
        map.size_m.1,
        b.fence_top_m,
    );
}

#[test]
fn f012_nothing_that_can_rest_on_the_fence_rests_inside_the_map() {
    // ## 🔴 THE MEASUREMENT `fence_margin_m` STANDS ON, and it refuted the obvious fix twice.
    //
    // "Stand the fence outside the footprint and the strict `>` in `out_of_the_world` covers
    // all of it" is **wrong**: a capsule does not rest on the point under its origin, it rests
    // on its bottom sphere, and that sphere reaches over the lip. With `fence_margin_m: 0.0` a
    // body put at x = **349.999**, a millimetre INSIDE the map, came to rest at **349.938,
    // y 199.994** on the fence's top face — `Grounded`, in the world, recorded as home. 48 of
    // 48 stances did. So the number to clear is not the float grid; moving the fence by one
    // ULP would have changed nothing.
    //
    // What has to be cleared is **how far back over the lip a body can PARK** — come to rest
    // and stay. Past that distance the contact normal tilts too far for friction and he slides
    // off, which is the good case: he falls into the map and lands on real ground.
    //
    // ## Why the fence is built 50 m INSIDE the map for this, and it is the point of the test
    //
    // Because with the shipped fence the recovery reaches every one of these bodies and warps
    // them away before they settle, so the measurement would report "nobody parks here" and
    // mean "nobody was allowed to try". At `fence_margin_m = -50` the whole top face lies
    // inside the footprint, `out_of_the_world` never fires, and what is left is a property of
    // the capsule, the box edge and avian's friction alone — which is what it is, and why it
    // transfers to the shipped fence 50 m further out.
    //
    // What the answer depends on: `game.ron: player.radius_m` and the capsule's endpoints, the
    // solver's friction and contact tolerance, `gravity_m_s2`, `bounds.fence_top_m`.
    // What this varies: **11 offsets from the lip**, every one a fraction of `radius_m` and
    // none of them typed as a length, from 0.6 `radius_m` inside the lip out to 0.2 outside and
    // concentrated in the last hundredth before it — the axis the whole fix lives on — x 2 ways
    // of arriving (set down 5 cm over the face, dropped 2 m onto it) x 2 bearings. 44 stances,
    // each given 10 s.
    // What it holds constant, and why: the other two bearings, because the four were measured
    // symmetric to four decimals on 2026-08-29 (+x 0.0702 against -x 0.0701); and the height of
    // the face, which IS the face.
    // What it skips: nothing. `checked` is counted and asserted, and `parked > 0` is what stops
    // "everything slid off" from passing this by measuring nothing.
    let d = data(&app_on_current_map());
    let shipped = d.current_map().expect("maps.ron: `current` names a map").clone();
    let b = shipped.bounds.clone();
    let r = d.game.player.radius_m;

    // The fence, deliberately inside the map, so the recovery cannot flatter the measurement.
    let inward_m = -50.0_f32;
    let mut app = app_with_fence_margin(inward_m);
    let e = me(&mut app);
    let (lx, lz) = (shipped.size_m.0 * 0.5 + inward_m, shipped.size_m.1 * 0.5 + inward_m);

    let mut deepest = f32::NEG_INFINITY;
    let mut deepest_at = String::new();
    let (mut checked, mut parked) = (0usize, 0usize);
    for (label, lip) in [("+x", Vec3::new(lx, 0.0, 0.0)), ("+z", Vec3::new(0.0, 0.0, lz))] {
        // ⚠️ `f32::signum(0.0)` is **1.0**, not 0.0 — written the naive way this vector comes
        // out diagonal for the flat bearings and the whole sweep measures a corner instead
        // (it did, and read 0.3721 m). The `abs().min(1.0)` is what zeroes the axis that is
        // not the bearing's, and it is the form the rest of this file's F-012 fixtures use.
        let out = Vec3::new(
            lip.x.signum() * lip.x.abs().min(1.0),
            0.0,
            lip.z.signum() * lip.z.abs().min(1.0),
        );
        for f in [
            -0.6_f32, -0.3, -0.15, -0.08, -0.04, -0.02, -0.01, -0.005, 0.0, 0.05, 0.2,
        ] {
            let from_lip = f * r;
            for (how, over) in [("set down", 0.05_f32), ("dropped", 2.0)] {
                checked += 1;
                let from = lip + out * from_lip + Vec3::Y * (b.fence_top_m + over);
                put_body_at(&mut app, e, from);
                // 10 s, and the question is asked at the END: a body that clings for two
                // seconds and then goes has not parked. An earlier draft asked after 2.5 s and
                // counted a body 1.5 m below the face and still falling (0.4073 m).
                ticks(&mut app, 600);
                let p = at(&app, e);
                // Still ON the face after ten seconds? A capsule standing on it sits 6 mm into
                // it; anything lower has slid off and landed in the map — the good case.
                if (p.y - b.fence_top_m).abs() > 0.5 {
                    continue;
                }
                parked += 1;
                // How far INSIDE the lip he parked, along the bearing's own axis.
                let inside = lip.dot(out) - p.dot(out);
                if inside > deepest {
                    deepest = inside;
                    deepest_at = format!("{label} {how} {from_lip:+.4} m from the lip -> {p:?}");
                }
            }
        }
    }

    println!(
        "F-012 fence top face: {checked} stances, {parked} parked, deepest park {deepest:.4} m \
         inside the lip ({deepest_at}); pinned fence_rest_reach_m {}, fence_margin_m {}, \
         radius_m {r}",
        b.fence_rest_reach_m, b.fence_margin_m
    );
    assert_eq!(checked, 44, "the sweep skipped samples");
    assert!(
        parked > 0,
        "not one of {checked} bodies was still on the fence's top face after 10 s — then this \
         test measures nothing and the margin it pays for is unpaid"
    );
    assert!(
        deepest <= b.fence_rest_reach_m,
        "a body parks {deepest:.4} m back over the fence's lip ({deepest_at}), and \
         `maps.ron: bounds.fence_rest_reach_m` says {}. That number is a MEASUREMENT: re-measure \
         it, write the new one into every map, and re-derive `fence_margin_m` — it has to stay \
         strictly between the new reach and `player.radius_m` = {r} m, or the fence's top face \
         is a ring a body can park on INSIDE the map's own footprint, which is the bug of \
         2026-08-29 with a smaller radius.",
        b.fence_rest_reach_m
    );
    // And the bracket, stated where the measurement is and not only in `tests/data.rs`.
    assert!(
        b.fence_rest_reach_m < b.fence_margin_m && b.fence_margin_m < r,
        "the bracket is broken: fence_rest_reach_m {} < fence_margin_m {} < radius_m {r} is what \
         makes every place a body can park on the fence lie outside the map, while a body \
         pressed against the fence's inner face stays inside it",
        b.fence_rest_reach_m,
        b.fence_margin_m
    );
}

#[test]
fn f012_the_ground_he_is_sent_back_to_is_never_ground_he_can_be_sent_back_from() {
    // ## THE POISONED HOME. `recovery.rs`'s own header claims this invariant, and on
    // ## 2026-08-29 the shipped game falsified it in four seconds of standing still.
    //
    // The header says: *"the place you get sent back to can never itself be a place you get
    // sent back from"*, and it says it because `record_safe_ground` and `recover_the_fallen`
    // call the same `out_of_the_world`. **That is not enough.** `out_of_the_world` tests
    // `|x| > hx` STRICTly and the fence's inner lip stood exactly ON `hx`, so a body parked at
    // `(350, 201, 0)` was `Grounded`, was "in the world", and got RECORDED. The shipped log:
    //   "back to Vec3(350.0, 200.49994, 0.0), the ground he stood on at tick 426"
    // and every later recovery from any cause — a seam, a bad warp, the plane — delivered the
    // player onto that 200 m ledge for the rest of the session. Reproduced under `--hub` too.
    //
    // 🔴 **And the oracle may not be `out_of_the_world` again** (`CLAUDE.md` rule 5, the fourth
    // shape): the first draft of this test asked that function about the recorded point, which
    // is the same function `record_safe_ground` had just asked about the same point, so it
    // passed on the broken build. Two implementations of one question cannot disagree. The
    // oracle here is the map's own **planned geometry**: a home has to be inside the footprint
    // and at or under the tallest thing `world::map` planned — the fence is not in that plan
    // (`bounds::plan_fence` is a different function), so its 200 m face cannot satisfy it.
    //
    // What the rule reads: the player's `Transform`, `MovementState`, `map.size_m`,
    // `bounds.recovery_plane_y_m`, `bounds.recovery_lift_m`.
    // What this varies: 8 bearings out of the map x **6 distances across the band where a body
    // can be grounded on the fence's lip while his origin is still inside the footprint** x
    // 2 drop heights over the face. 96 stances, each parked 5 s so the recorder really sees it.
    //   That band is `[hx - (radius_m - fence_margin_m), hx]` — 0.17 m wide as the numbers
    //   ship — and it is where the second half of the fix lives: the geometry cannot close it,
    //   because a capsule reaches `radius_m` over the lip and `fence_margin_m` has to stay
    //   under `radius_m` for the other half of the bracket. The distances are fractions of that
    //   band and not typed lengths, so they follow both numbers when either moves, and they
    //   include **exactly zero** — the map's own edge, which is the one-character bug.
    // What it holds constant: the map, and the parking time.
    // What it skips: nothing. There is no `continue`; `checked` is counted and asserted.
    use defeated_by_titan::player::recovery::SafeGround;

    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    // The oracle, out of the planned world and never typed: the highest surface `world::map`
    // built. The fence is not in it.
    let roof_m = defeated_by_titan::world::map::plan_blocks(&d, &map)
        .iter()
        .map(|b| b.center_m.y + b.size_m.y * 0.5)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        roof_m + 1.0 < map.bounds.fence_top_m,
        "the fixture needs the fence's top face ({} m) to be above everything the map plans \
         ({roof_m} m), or the oracle below cannot tell the ledge from a roof",
        map.bounds.fence_top_m
    );

    let mut poisoned = Vec::new();
    let mut checked = 0usize;
    for (label, on_edge) in eight_ways_out(hx, hz) {
        let dir = Vec3::new(on_edge.x.signum() * on_edge.x.abs().min(1.0), 0.0, on_edge.z.signum() * on_edge.z.abs().min(1.0))
            .normalize();
        // The band a body can be grounded on the lip in while his origin is still inside the
        // footprint, and the samples are fractions of it — `-1.0` is where his bottom sphere
        // only just reaches the lip, `0.0` is the map's own edge, `+0.01` is a hair outside it.
        let band_m = d.game.player.radius_m - map.bounds.fence_margin_m;
        for f in [-1.0_f32, -0.6, -0.3, -0.01, 0.0, 0.01] {
            let across = f * band_m;
            for over in [1.0_f32, 2.0] {
                checked += 1;
                let from = Vec3::new(on_edge.x, map.bounds.fence_top_m + over, on_edge.z)
                    + dir * across;
                put_body_at(&mut app, e, from);
                // 5 s: a 200 m fall to the ground inside the map takes 3.5 s at
                // `gravity_m_s2`, and the recorder needs him standing for one tick after it.
                ticks(&mut app, 300);
                let safe = app
                    .world()
                    .entity(e)
                    .get::<SafeGround>()
                    .copied()
                    .expect("a player carries his own SafeGround");
                let h = safe.pos_m;
                if h.x.abs() > hx || h.z.abs() > hz || h.y > roof_m + 1.0 {
                    poisoned.push(format!(
                        "{label} {across:+.4} m ({f:+.2} of the band), {over} m over the face, \
                         from {from:?}: home is {h:?} at tick {}",
                        safe.tick
                    ));
                }
            }
        }
    }

    assert_eq!(checked, 96, "the sweep skipped samples");
    assert!(
        poisoned.is_empty(),
        "{} of {checked} stances left `SafeGround` pointing at a place that is not on anything \
         the map planned (footprint {} x {} m, highest planned surface {roof_m:.2} m) — the home \
         is poisoned and every later fall from any cause ends there: {:?}",
        poisoned.len(),
        map.size_m.0,
        map.size_m.1,
        &poisoned[..poisoned.len().min(3)]
    );

    // ## And the driven half, because the sweep above judges a recorded point and not a body.
    //
    // Park exactly on the lip, then fall under the plane the way a seam does it, and see where
    // the recovery actually delivers him. On the broken build this read y = 200.000 — the
    // ledge — and he was still there ten seconds later.
    put_body_at(&mut app, e, Vec3::new(hx, map.bounds.fence_top_m + 1.0, 0.0));
    ticks(&mut app, 300);
    put_body_at(&mut app, e, Vec3::new(0.0, map.bounds.recovery_plane_y_m - 20.0, 0.0));
    ticks(&mut app, 420);
    let p = at(&app, e);
    assert!(
        p.x.abs() <= hx && p.z.abs() <= hz && p.y <= roof_m + 1.0,
        "after parking on the fence's inner lip at ({hx}, {}, 0) and then falling under the \
         plane, the recovery put him at {p:?} — outside the {} x {} m footprint or above the \
         highest thing the map plans ({roof_m:.2} m). That is the 200 m ledge, and it is where \
         every later recovery of the session would have gone.",
        map.bounds.fence_top_m,
        map.size_m.0,
        map.size_m.1
    );
}

#[test]
fn f012_at_the_smallest_legal_fence_margin_the_recorder_still_keeps_nothing_on_the_fence() {
    // ## 🔴 THE TEST THAT PAYS FOR THE OTHER HALF OF THE FIX, and it exists because the
    // ## control run of 2026-08-29 caught it being unpaid for.
    //
    // `record_safe_ground` does two things beyond asking `out_of_the_world`: it judges the
    // **whole body** (`+ radius_m` outward on each axis, at the place he would be PUT DOWN),
    // and it refuses a stance he is **falling past** rather than standing on
    // (`|vy| > gravity_m_s2 / simulation_hz`). Both were written against a measurement — a body
    // sliding off the fence's lip was recorded at `(-299.74, 199.886, 0)`, 200 m up with
    // nothing under it — and then **neither of them could be made to go red**: at the shipped
    // `fence_margin_m` of 0.18 the lip is far enough out that the recovery removes a body
    // before he can creep inward at all. Two fixes with no failing test are two guesses
    // (`CLAUDE.md` rule 5), so this is the configuration that reaches them.
    //
    // ## The configuration, and it is a LEGAL one
    //
    // `tests/data.rs::f012_the_fence_stands_within_one_body_radius_of_the_map_edge` accepts any
    // margin in `(fence_rest_reach_m, player.radius_m)`. This runs the **smallest** one a
    // millimetre above the floor of that window. The fence's top face then begins only
    // `margin` outside the map edge, its inner slope is `margin / radius_m` — gentle enough
    // that friction holds a body dropped onto it — and a body can be `Grounded`, still, and
    // **inside the footprint** on a surface that is not part of the world. The origin alone
    // cannot tell that stance from standing on the map's own ground. The body can.
    //
    // What the rule reads: `Transform`, `MovementState`, `Velocity`, `map.size_m`,
    // `bounds.recovery_lift_m`, `bounds.recovery_plane_y_m`, `game.player.radius_m`,
    // `gravity_m_s2`, `simulation_hz`.
    // What this varies: 4 bearings x 5 places across the band `[lip - radius_m, hx]`, dropped
    // onto the fence's top face. 20 stances.
    // What it holds constant, and why: the margin (that IS the configuration under test) and
    // the drop height.
    // What it skips: nothing. `checked` is counted and asserted, and `on_the_face > 0` is what
    // stops "nobody ever landed on it" from passing this by measuring nothing.
    use defeated_by_titan::player::recovery::SafeGround;

    let shipped = data(&app_on_current_map());
    let map0 = shipped.current_map().expect("maps.ron: `current` names a map").clone();
    let r = shipped.game.player.radius_m;
    // A millimetre above the floor of the window `tests/data.rs` allows.
    let margin_m = map0.bounds.fence_rest_reach_m + 0.001;
    assert!(margin_m < r, "the window `(fence_rest_reach_m, radius_m)` is empty — nothing to test");

    let mut app = app_with_fence_margin(margin_m);
    let e = me(&mut app);
    let (hx, hz) = (map0.size_m.0 * 0.5, map0.size_m.1 * 0.5);
    let top = map0.bounds.fence_top_m;
    let roof_m = defeated_by_titan::world::map::plan_blocks(&shipped, &map0)
        .iter()
        .map(|b| b.center_m.y + b.size_m.y * 0.5)
        .fold(f32::NEG_INFINITY, f32::max);

    let mut poisoned = Vec::new();
    let (mut checked, mut on_the_face) = (0usize, 0usize);
    for (label, on_edge) in eight_ways_out(hx, hz) {
        if on_edge.x != 0.0 && on_edge.z != 0.0 {
            continue; // the four corners are the same face twice; counted below, not skipped
        }
        let dir = Vec3::new(on_edge.x.signum() * on_edge.x.abs().min(1.0), 0.0, on_edge.z.signum() * on_edge.z.abs().min(1.0))
            .normalize();
        // From where the bottom sphere only just reaches the lip, up to the map's own edge.
        for f in [-1.0_f32, -0.7, -0.4, -0.15, 0.0] {
            checked += 1;
            let from = Vec3::new(on_edge.x, top + 1.0, on_edge.z) + dir * (f * (r - margin_m));
            app.world_mut()
                .entity_mut(e)
                .insert(SafeGround { pos_m: Vec3::new(0.0, -1000.0, 0.0), tick: 0 });
            put_body_at(&mut app, e, from);
            let mut landed = false;
            for _ in 0..300 {
                ticks(&mut app, 1);
                if (at(&app, e).y - top).abs() < 0.5 {
                    landed = true;
                }
                let home = app
                    .world()
                    .entity(e)
                    .get::<SafeGround>()
                    .copied()
                    .expect("a player carries his own SafeGround")
                    .pos_m;
                if home.y > roof_m + 1.0 {
                    poisoned.push(format!(
                        "{label} {:+.3} m from the map edge, from {from:?}: home is {home:?}, \
                         above everything the map plans ({roof_m:.2} m)",
                        f * (r - margin_m)
                    ));
                    break;
                }
            }
            if landed {
                on_the_face += 1;
            }
        }
    }

    assert_eq!(checked, 20, "the sweep skipped samples — the four corners are counted out on \
         purpose (the same face twice) and the four flats are all of it");
    assert!(
        on_the_face > 0,
        "not one of {checked} bodies ever touched the fence's top face at margin {margin_m} — \
         then this fixture measures nothing and both halves of the recorder are unpaid for"
    );
    assert!(
        poisoned.is_empty(),
        "{} of {checked} stances got a home 200 m up on the fence's top face, inside the \
         footprint, with nothing under it. `record_safe_ground` has to judge the WHOLE BODY at \
         the place he would be put down, and refuse a stance he is falling past: {:?}",
        poisoned.len(),
        &poisoned[..poisoned.len().min(3)]
    );
}

#[test]
fn f012_a_recovery_whose_destination_does_not_hold_is_not_repeated_every_tick() {
    // ## THE LOOP, and it is a defect of its own even with the ring gone.
    //
    // Measured on the shipped binary, 2026-08-29: from the fence's inner lip any lateral input
    // nudged `x` from 350.000 to 350.1, `PastTheEdge` fired, the warp put him back at
    // `safe + lift` = (350.0, 200.49994, 0) — **on the lip** — he dropped the half metre onto
    // it and drifted out again. **1501 warp lines in one run, one per tick, 25.0 s of wall
    // clock, every one with the identical destination**, plus 60 `warn!` lines per second.
    //
    // If the destination does not hold, warping to it again is not a fix. So the rule under
    // test is not "the ring is gone" — that is the geometry's job and
    // `f012_the_top_of_the_fence_is_not_a_ring_you_can_stand_on_outside_the_map` measures it.
    // It is: **a recovery that fails is escalated and then given up on, and it is bounded.**
    //
    // The poison is injected by hand, exactly the way `put_body_at` injects a position: the
    // whole point is that this has to hold for a `SafeGround` that no code path can produce
    // any more, because the next bug will produce one the same way this one did.
    //
    // What the rule reads: `Transform`, `SafeGround`, `map.size_m`,
    // `bounds.recovery_plane_y_m`, `bounds.recovery_lift_m`.
    // What this varies: two poisoned homes (past the edge, and under the plane) x two places
    // to be stranded in. 4 cases, 300 ticks (5 s) each.
    // What it holds constant: the map.
    // What it skips: nothing — every case asserts, and each asserts that the FIRST tick did
    // warp, so a recovery that never fires cannot pass by doing nothing.
    use defeated_by_titan::player::recovery::SafeGround;

    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let plane = map.bounds.recovery_plane_y_m;

    let mut runaway = Vec::new();
    let mut never_fired = Vec::new();
    let mut checked = 0usize;
    for (home_label, home) in [
        ("past the edge", Vec3::new(hx + 50.0, 5.0, 0.0)),
        ("under the plane", Vec3::new(0.0, plane - 10.0, 0.0)),
    ] {
        for (where_label, stranded) in [
            ("on the fence", Vec3::new(hx + 5.0, map.bounds.fence_top_m + 1.0, 0.0)),
            ("over the void", Vec3::new(0.0, 40.0, hz + 80.0)),
        ] {
            checked += 1;
            app.world_mut().entity_mut(e).insert(SafeGround { pos_m: home, tick: 0 });
            put_body_at(&mut app, e, stranded);
            let mut warps = 0usize;
            let mut first = 0usize;
            for tick in 0..300 {
                app.update();
                let n = warps_sent(&app);
                warps += n;
                if tick == 0 {
                    first = n;
                }
            }
            let case = format!("home {home_label}, stranded {where_label}");
            if first == 0 {
                never_fired.push(case.clone());
            }
            // Two: one to the recorded ground, one to the fallback when that did not hold.
            // Anything above that is a recovery repeating itself.
            if warps > 2 {
                runaway.push(format!("{case}: {warps} warps in 300 ticks"));
            }
        }
    }

    assert_eq!(checked, 4, "the sweep skipped samples");
    assert!(
        never_fired.is_empty(),
        "the recovery never fired at all for {never_fired:?} — this fixture measures a BOUND on \
         a mechanism, and a mechanism that does nothing satisfies every bound"
    );
    assert!(
        runaway.is_empty(),
        "{} of {checked} strandings warped the player more than twice — a destination that does \
         not hold is not fixed by warping to it again, and the shipped game did it 1501 times \
         in 25 s with 60 warn lines a second: {runaway:?}",
        runaway.len()
    );
}

#[test]
fn f012_the_map_footprint_is_the_world_at_every_height() {
    // ## THE HEIGHT AXIS. It is the one every fixture of the first round held constant, and it
    // ## is the one the rule depended on.
    //
    // What the rule reads: `Transform.translation.x`, `.y`, `.z`; `map.size_m` (both axes);
    // `bounds.recovery_plane_y_m`; `SafeGround`.
    // What this sweep varies: 8 bearings out of the map x **11 heights**, derived from the
    // map's own numbers and spanning everything a body can be at — below the fence's foot, on
    // the ground, on the coping of the 120 m wall, just under and just over `fence_top_m`, and
    // up at the two ceilings the gear was measured to reach (657 m from the ground, 900 m from
    // the wall). 88 samples.
    // What it holds constant: the distance past the edge (2 m — far enough that no rounding
    // decides it, close enough that a body 2 m out is exactly the case a swing produces) and
    // the map (`current`, because the claim is about the district that ships).
    // What it skips: nothing; `checked` is asserted against the sweep's size.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let b = map.bounds.clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let floor = deepest_floor_m(&app);

    let heights = [
        b.fence_bottom_m + 5.0,
        floor,
        2.0,
        30.0,
        121.0,
        b.fence_top_m * 0.5,
        b.fence_top_m - 1.0,
        b.fence_top_m + 0.5,
        b.fence_top_m + 60.0,
        657.0,
        902.0,
    ];

    let mut lost = Vec::new();
    let mut checked = 0usize;
    for (label, on_edge) in eight_ways_out(hx, hz) {
        let dir = Vec3::new(on_edge.x.signum() * on_edge.x.abs().min(1.0), 0.0, on_edge.z.signum() * on_edge.z.abs().min(1.0))
            .normalize();
        for y in heights {
            checked += 1;
            let from = Vec3::new(on_edge.x, y, on_edge.z) + dir * 2.0;
            put_body_at(&mut app, e, from);
            ticks(&mut app, 90); // 1.5 s — a recovery costs ONE tick, and then he settles
            let p = at(&app, e);
            if !back_in_the_world(p, hx, hz, floor) {
                lost.push(format!("{label} at y = {y:.1} -> {p:?}"));
            }
        }
    }

    assert_eq!(checked, 8 * heights.len(), "the sweep skipped samples");
    assert!(
        lost.is_empty(),
        "{} of {checked} bodies two metres outside the {} x {} m map were still out of the \
         world 1.5 s later. Every one of them is out whatever his height — the ones above \
         {} m are the ones the first build could not see: {lost:?}",
        lost.len(),
        map.size_m.0,
        map.size_m.1,
        b.fence_top_m,
    );
}

/// Flies `n` ticks with the look nailed at an absolute angle and **samples every tick**:
/// how many ticks he spent outside the map's footprint in total, the longest unbroken run of
/// them, and the lowest `y` he reached. A final position cannot see any of the three — the
/// bug being measured is a body that is outside for ten seconds and then lands somewhere
/// ordinary.
fn fly_looking(
    app: &mut App,
    e: Entity,
    yaw_deg: f32,
    pitch_deg: f32,
    n: u64,
    hx: f32,
    hz: f32,
) -> (u64, u64, f32) {
    let (mut total, mut streak, mut longest, mut lowest) = (0u64, 0u64, 0u64, f32::INFINITY);
    for _ in 0..n {
        app.world_mut().resource_mut::<LookOverride>().0 =
            Some((yaw_deg.to_radians(), pitch_deg.to_radians()));
        app.update();
        let p = at(app, e);
        lowest = lowest.min(p.y);
        if p.x.abs() > hx || p.z.abs() > hz {
            total += 1;
            streak += 1;
            longest = longest.max(streak);
        } else {
            streak = 0;
        }
    }
    (total, longest, lowest)
}

#[test]
fn f012_two_held_keys_from_the_spawn_point_do_not_carry_you_out_of_the_world() {
    // ## The keyboard-only escape, driven exactly as it was measured on 2026-08-28
    //
    // No warp anywhere but onto the spawn point, no velocity written by hand, no hook: `W` and
    // `ShiftLeft` held, and the look angle turned twice. The measured run read
    // **243.143 m** after 7 s of climbing at pitch 89, then **284.175 m and outside the map**
    // after 9 s at pitch 20, then **-233.201 m and still falling** eight seconds later —
    // roughly **ten seconds of falling out of the world** before the plane at -300 m caught
    // him. The fence is 200 m tall and did nothing about any of it.
    //
    // ## What is asserted, and why it is a STREAK and not a final position
    //
    // After the fix he still flies out — of course he does, he is holding `W` at the edge —
    // and he is put back on the tick after he crosses. So the honest measure is **how long he
    // is out**, sampled every tick: one crossing is not a bug, ten seconds of falling is. The
    // recovery costs one tick to write the message and one for `apply_warps` to move the body,
    // so anything up to a handful of ticks is the mechanism working.
    //
    // What the answer depends on: `player.air_accel_m_s2`, `vector.boost_m_s2`,
    // `game.gravity_m_s2`, `vector.gas_tank`, `map.size_m`, `bounds.*`, `SafeGround`.
    // What this varies: the two look angles (up, then out), and it samples **every one of the
    // 1080 ticks** rather than the three the old fixture looked at.
    // What it holds constant: one flight path — it is a repro of a reported escape, not a
    // sweep; the sweep over heights is `f012_the_map_footprint_is_the_world_at_every_height`.
    // What it skips: nothing; `total` counts the ticks it saw him out and the assertion below
    // prints it, so a run in which he never left says so instead of passing vacuously.
    // ⚠️ It asserts no height anywhere, so a retune of the provisional `gravity_m_s2` /
    // `boost_m_s2` changes how far he gets and not whether this passes.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let floor = deepest_floor_m(&app);

    put_body_at(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 60);
    assert_eq!(state(&app, e), MovementState::Grounded, "the escape starts from a STANDING player");

    hold(&mut app, KeyCode::KeyW);
    hold(&mut app, KeyCode::ShiftLeft);
    // 7 s straight up. Measured apex of this leg on 2026-08-28: 241.9 m — over a 200 m fence.
    let (up_total, _, _) = fly_looking(&mut app, e, 0.0, 89.0, 420, hx, hz);
    let apex = at(&app, e);
    // 9 s outward. `yaw = -90` looks along +X (`docs/conventions.md`), `pitch 20` keeps him
    // climbing while he travels — the exact pair that read 284 m and outside the map.
    let (out_total, longest, lowest) = fly_looking(&mut app, e, -90.0, 20.0, 540, hx, hz);
    release(&mut app, KeyCode::KeyW);
    release(&mut app, KeyCode::ShiftLeft);
    let (tail_total, tail_longest, tail_lowest) = fly_looking(&mut app, e, -90.0, 20.0, 480, hx, hz);

    // The fixture has to BE the escape, or every assertion below is satisfied by a player who
    // never went anywhere (`CLAUDE.md` rule 5).
    assert!(
        apex.y > 120.0,
        "7 s of W+Shift looking up reached only {:.1} m — that is not over the 200 m fence and \
         the run below is not the escape that was reported",
        apex.y
    );
    assert_eq!(up_total, 0, "he left the footprint while flying straight UP from the spawn point");
    assert!(
        out_total > 0,
        "9 s of flying outward from {apex:?} never crossed the {} x {} m edge at all — the \
         escape did not happen and this test proves nothing",
        map.size_m.0,
        map.size_m.1
    );

    // 6 ticks: one to write `WarpPlayer`, one for `apply_warps` to move the body, and slack for
    // the tick he is still travelling on. Ten seconds is 600.
    let cap = 6;
    assert!(
        longest.max(tail_longest) <= cap,
        "he was outside the {} x {} m map for {} ticks in a row on two held keys ({out_total} \
         + {tail_total} ticks out of 1020 in total). Anything over {cap} is a player falling \
         through nothing: the escape reached {apex:?} and the lowest he got was {:.1} m",
        map.size_m.0,
        map.size_m.1,
        longest.max(tail_longest),
        lowest.min(tail_lowest)
    );
    let p = at(&app, e);
    assert!(
        back_in_the_world(p, hx, hz, floor),
        "after the escape and 8 s more he is at {p:?}, out of the world (floor {floor:.2} m)"
    );
    assert!(
        lowest.min(tail_lowest) > map.bounds.recovery_plane_y_m,
        "the footprint rule is supposed to catch him BEFORE the plane does, and he still \
         reached {:.1} m against a plane at {} m",
        lowest.min(tail_lowest),
        map.bounds.recovery_plane_y_m
    );
}

#[test]
fn f012_a_body_driven_into_the_fence_at_top_speed_is_never_recovered() {
    // ## THE CONTROL THAT PAYS FOR THE ZERO GRACE, and it is a measurement first.
    //
    // `player::recovery::out_of_the_world` allows **no tolerance** at the map's edge, because
    // any tolerance of `g` metres is a standable ring `g` metres wide on top of the fence. That
    // is only affordable if the solver never puts a legitimate body past the edge — so this
    // test drives one into the fence at `vector.max_speed_m_s` and reads the largest excursion
    // it can produce, instead of arguing from the capsule radius.
    //
    // What the answer depends on: `player.radius_m`, `bounds.fence_margin_m`,
    // `fence_thickness_m`, `game.ron: substeps`, `vector.max_speed_m_s`, `map.size_m`.
    // What this varies: the four flats **x six heights** spanning the fence from its foot to
    // its top face — including 199 m and 200.5 m, the two either side of `fence_top_m`, which
    // is where a body that is going over rather than into it separates. 24 launches.
    // What it holds constant, and why: the direction is straight at the panel (the diagonal
    // case is `f012_flying_at_the_edge_at_top_speed_does_not_tunnel_through_it`) and the map.
    // What it skips: the two heights above `fence_top_m` are **expected** to leave the map, so
    // they are counted separately and not asserted as excursions — the count is printed.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let b = map.bounds.clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let v_max = d.game.vector.max_speed_m_s;

    let mut worst = f32::NEG_INFINITY;
    let mut worst_at = String::new();
    let mut warped_below_the_top = Vec::new();
    let mut over_the_top = 0usize;
    let mut checked = 0usize;

    for (label, from, dir) in [
        ("+x", Vec3::new(hx - 60.0, 0.0, 0.0), Vec3::X),
        ("-x", Vec3::new(-hx + 60.0, 0.0, 0.0), Vec3::NEG_X),
        ("+z", Vec3::new(0.0, 0.0, hz - 60.0), Vec3::Z),
        ("-z", Vec3::new(0.0, 0.0, -hz + 60.0), Vec3::NEG_Z),
    ] {
        for y in [b.fence_bottom_m + 10.0, 3.0, 60.0, 121.0, b.fence_top_m - 1.0, b.fence_top_m + 0.5]
        {
            checked += 1;
            let above_the_fence = y > b.fence_top_m;
            put_body_at(&mut app, e, from + Vec3::Y * y);
            app.update();
            app.world_mut().entity_mut(e).get_mut::<LinearVelocity>().expect("velocity").0 =
                dir * v_max;
            let mut warps = 0usize;
            let mut out_by = f32::NEG_INFINITY;
            for _ in 0..90 {
                app.update();
                warps += warps_sent(&app);
                let p = at(&app, e);
                out_by = out_by.max((p.x.abs() - hx).max(p.z.abs() - hz));
            }
            if above_the_fence {
                over_the_top += 1;
                continue;
            }
            if out_by > worst {
                worst = out_by;
                worst_at = format!("{label} at y = {y:.1}");
            }
            if warps > 0 {
                warped_below_the_top.push(format!("{label} at y = {y:.1}: {warps} warp(s)"));
            }
        }
    }

    println!(
        "F-012 fence at speed: worst excursion {worst:+.4} m relative to the map edge \
         ({worst_at}), fence_margin_m {}, radius_m {}",
        b.fence_margin_m,
        d.game.player.radius_m
    );
    assert_eq!(checked, 24, "the sweep skipped samples");
    assert_eq!(over_the_top, 4, "4 of the 24 launches start above the fence and are expected to \
         leave the map — they are counted, not asserted, and this is the count");
    assert!(
        warped_below_the_top.is_empty(),
        "a body flying INTO the fence at {v_max} m/s was recovered {} time(s) — the zero grace \
         at the edge is eating legitimate play: {warped_below_the_top:?}",
        warped_below_the_top.len()
    );
    // ## The number the upper half of the bracket stands on, and it is DERIVED, not typed.
    //
    // The solver keeps a capsule's origin `player.radius_m` inside any solid face, so a body
    // pressed against the fence's inner face — which stands `fence_margin_m` outside the map's
    // edge — sits `radius_m - fence_margin_m` INSIDE that edge. Measured 2026-08-29 at
    // `fence_margin_m: 0.0`: **-0.3500 m**, `radius_m` to four decimals, so the solver's
    // penetration at the clamp is zero. The threshold is half of the derived clearance: a real
    // regression halves it, and a rounding does not.
    let clearance_m = d.game.player.radius_m - b.fence_margin_m;
    assert!(
        clearance_m > 0.0,
        "maps.ron: fence_margin_m {} is not below player.radius_m {} — a body pressed against \
         the fence is then outside the map by construction and this measurement is meaningless \
         (`tests/data.rs::f012_the_fence_stands_within_one_body_radius_of_the_map_edge`)",
        b.fence_margin_m,
        d.game.player.radius_m
    );
    assert!(
        worst < -0.5 * clearance_m,
        "the solver let a body at {v_max} m/s reach {worst:.4} m past the {} x {} m edge \
         ({worst_at}). `radius_m` {} minus `fence_margin_m` {} is {clearance_m:.4} m of \
         clearance, and the zero grace in `out_of_the_world` needs the fence to keep a body at \
         least half of it inside the map.",
        map.size_m.0,
        map.size_m.1,
        d.game.player.radius_m,
        b.fence_margin_m
    );
}

#[test]
fn f012_a_legitimate_fall_inside_the_map_is_never_recovered_at_any_height() {
    // ## THE CONTROLS the user's own play consists of. Not one of these may be recovered.
    //
    //   a tower dive of 120 m · a fall into a courtyard · a hook that drops you 60 m ·
    //   a rope swing along the OUTSIDE of the 120 m wall, at the far end of the district
    //
    // The rule that tells them from a fall out of the world is `map.size_m` and
    // `recovery_plane_y_m` — never the fall distance, never the speed, and (since 2026-08-28)
    // never the height either, which is exactly why this control has to run at several.
    //
    // What the answer depends on: `map.size_m`, `bounds.recovery_plane_y_m`, `SafeGround`,
    // `gravity_m_s2`.
    // What this varies: 4 kinds of legitimate fall x the place in the district, and for the
    // drops the height (30, 60 and 121 m). 11 cases.
    // What it holds constant: the map, because the claim is about the district that ships.
    // What it skips: nothing — every case asserts, and each one also asserts that it really
    // fell or really moved, so a body that never left its start cannot satisfy it.
    let mut app = app_on_current_map();
    let e = me(&mut app);
    let d = data(&app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let (hx, hz) = (map.size_m.0 * 0.5, map.size_m.1 * 0.5);
    let v_max = d.game.vector.max_speed_m_s;

    // Give him a ground to be wrongly sent back to, or a recovery would be invisible.
    put_body_at(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 60);
    assert_eq!(state(&app, e), MovementState::Grounded, "the controls need a recorded ground");

    let mut recovered = Vec::new();
    // Free falls: a courtyard, three 60 m drops of the kind a released hook produces, and two
    // dives off the coping — one in the middle of the wall and one 10 m from the map's corner,
    // which is the case a footprint rule can eat.
    for (label, from) in [
        ("courtyard 30 m", Vec3::new(0.0, 30.0, 0.0)),
        ("hook drop 60 m, mid-district", Vec3::new(200.0, 60.0, 200.0)),
        ("hook drop 60 m, west", Vec3::new(-260.0, 60.0, 100.0)),
        ("hook drop 60 m, 10 m from the corner", Vec3::new(hx - 10.0, 60.0, hz - 10.0)),
        ("tower dive 121 m, mid-wall", Vec3::new(0.0, 121.0, -60.0)),
        ("tower dive 121 m, 10 m from the edge", Vec3::new(hx - 10.0, 121.0, -60.0)),
    ] {
        put_body_at(&mut app, e, from);
        let mut warps = 0usize;
        let mut lowest = f32::INFINITY;
        for _ in 0..300 {
            app.update();
            warps += warps_sent(&app);
            lowest = lowest.min(at(&app, e).y);
        }
        assert!(
            from.y - lowest > 20.0,
            "{label}: it has to BE a fall — it dropped {:.1} m",
            from.y - lowest
        );
        if warps > 0 {
            recovered.push(format!("{label} from {from:?}: {warps} warp(s), lowest {lowest:.1} m"));
        }
    }

    // The swing along the OUTSIDE of the wall. Ashgate's wall is `(0, 119, -120) x (700, 2, 28)`
    // — it runs the full width of the map, so its outer face is at z = -134 and everything
    // beyond it to z = -350 is still the district. A rope carries a player along there at the
    // clamp; the fence is what he meets at the end of it, and he must not be teleported for it.
    for (label, from, dir) in [
        ("swing +x outside the wall", Vec3::new(-hx + 20.0, 60.0, -140.0), Vec3::X),
        ("swing -x outside the wall", Vec3::new(hx - 20.0, 60.0, -140.0), Vec3::NEG_X),
        ("swing -z into the corner, over the wall", Vec3::new(hx - 20.0, 140.0, -170.0), Vec3::NEG_Z),
        ("swing +x along the far edge", Vec3::new(-hx + 20.0, 25.0, hz - 6.0), Vec3::X),
        ("swing -z along the far edge", Vec3::new(hx - 6.0, 25.0, hz - 20.0), Vec3::NEG_Z),
    ] {
        put_body_at(&mut app, e, from);
        app.update();
        app.world_mut().entity_mut(e).get_mut::<LinearVelocity>().expect("velocity").0 =
            dir * v_max;
        let mut warps = 0usize;
        let mut travelled = 0.0_f32;
        for _ in 0..420 {
            app.update();
            warps += warps_sent(&app);
            travelled = travelled.max((at(&app, e) - from).dot(dir));
        }
        assert!(
            travelled > 50.0,
            "{label}: it has to BE a swing — he covered {travelled:.1} m along {dir:?}"
        );
        if warps > 0 {
            recovered.push(format!("{label} from {from:?}: {warps} warp(s) after {travelled:.1} m"));
        }
    }

    assert!(
        recovered.is_empty(),
        "{} of 11 legitimate falls and swings inside the {} x {} m district were recovered. \
         The footprint rule is eating play it must not touch: {recovered:?}",
        recovered.len(),
        map.size_m.0,
        map.size_m.1
    );
}

/// The same app, but the fence is planned `margin_m` outside the map's own edge — the latent
/// bug of 2026-08-28, in the one form that makes it visible. Mutated **before** the first
/// `update()`, so `world::bounds::build_bounds` really builds the fence out there.
fn app_with_fence_margin(margin_m: f32) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    {
        let mut d = app.world_mut().resource_mut::<GameData>();
        let key = d.maps.current.clone();
        d.maps.maps.get_mut(&key).expect("current names a map").bounds.fence_margin_m = margin_m;
    }
    app.update();
    app
}

#[test]
fn f012_a_fence_far_outside_the_map_is_not_a_bigger_map() {
    // ## The latent one, and it was one RON number away from shipping.
    //
    // `record_safe_ground` gated the footprint on `map.size_m * 0.5 + bounds.fence_margin_m` —
    // the **fence's** footprint, not the map's — inside a file whose own header says it knows
    // nothing about `world::bounds`. With `fence_margin_m = 0` the two coincide and nothing
    // shipped wrong; with `500` the recovery put the player back at `(350.10, 120.50, -120.0)`,
    // 0.1 m past the end of the coping with nothing under it, and he fell out and had to be
    // recovered twice.
    //
    // What the rule reads: `map.size_m` and `bounds.recovery_plane_y_m` — and it must read
    // **nothing else**, which is what this measures by changing the one thing it must not read.
    // What this varies: `fence_margin_m` (0 and 500 m), x 8 bearings x 5 heights for the pure
    // half, and one driven body in the 500 m gap between the map's edge and the fence.
    // What it skips: nothing.
    use defeated_by_titan::player::recovery::out_of_the_world;

    let d = data(&app_on_current_map());
    let mut wide = d.current_map().expect("maps.ron: `current` names a map").clone();
    let narrow = wide.clone();
    wide.bounds.fence_margin_m = 500.0;
    let (hx, hz) = (narrow.size_m.0 * 0.5, narrow.size_m.1 * 0.5);

    let mut disagreed = Vec::new();
    let mut checked = 0usize;
    for (label, on_edge) in eight_ways_out(hx, hz) {
        let dir = Vec3::new(on_edge.x.signum() * on_edge.x.abs().min(1.0), 0.0, on_edge.z.signum() * on_edge.z.abs().min(1.0))
            .normalize();
        for past in [-5.0_f32, -0.5, 0.5, 5.0, 400.0] {
            for y in [-40.0_f32, 2.0, 121.0, 205.0, 900.0] {
                checked += 1;
                let p = Vec3::new(on_edge.x, y, on_edge.z) + dir * past;
                if out_of_the_world(&narrow, p) != out_of_the_world(&wide, p) {
                    disagreed.push(format!("{label} {past:+} m at y = {y}: {p:?}"));
                }
            }
        }
    }
    assert_eq!(checked, 8 * 5 * 5, "the sweep skipped samples");
    assert!(
        disagreed.is_empty(),
        "{} of {checked} points got a different answer when the FENCE moved 500 m — \
         `out_of_the_world` reads the map's footprint and must not read the fence's: \
         {disagreed:?}",
        disagreed.len()
    );

    // And the driven half, because a pure sweep over invented `Vec3`s is arithmetic
    // (`CLAUDE.md` rule 5, the fourth shape): a real body, in the real 500 m gap, with a real
    // fence built out at 850 m.
    let mut app = app_with_fence_margin(500.0);
    let e = me(&mut app);
    let floor = deepest_floor_m(&app);
    put_body_at(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
    ticks(&mut app, 60);
    let safe = at(&app, e);
    assert!(safe.x.abs() <= hx, "the fixture needs him standing INSIDE the map: {safe:?}");

    put_body_at(&mut app, e, Vec3::new(hx + 150.0, 20.0, 0.0));
    ticks(&mut app, 90);
    let p = at(&app, e);
    assert!(
        back_in_the_world(p, hx, hz, floor),
        "a body 150 m past the {} x {} m map, inside a fence planned 500 m further out, is at \
         {p:?} 1.5 s later. The gap between the map and the fence is floorless — it is not a \
         bigger map.",
        narrow.size_m.0,
        narrow.size_m.1
    );
}

/// Holds `W` + `ShiftLeft` straight up from `from` and reports the **steady** climb: metres
/// gained and gas spent between second 5 and second 15, by which time the body is at
/// `vector.max_speed_m_s` and every transient is behind it.
///
/// No hook, no warp after the start, no velocity written by hand — two keys, exactly the two
/// the escape of 2026-08-28 was flown on.
fn steady_climb(app: &mut App, e: Entity, from: Vec3) -> (f32, f32) {
    let gas = |app: &App| app.world().get::<Gas>(e).expect("the player has gas").current;
    put_body_at(app, e, from);
    ticks(app, 30);
    hold(app, KeyCode::KeyW);
    hold(app, KeyCode::ShiftLeft);
    let up = |app: &mut App, n: u64| {
        for _ in 0..n {
            app.world_mut().resource_mut::<LookOverride>().0 =
                Some((0.0, 89.0_f32.to_radians()));
            app.update();
        }
    };
    up(app, 300); // 5 s of spin-up, thrown away
    let (y0, g0) = (at(app, e).y, gas(app));
    up(app, 600); // 10 s of steady state, measured
    let (y1, g1) = (at(app, e).y, gas(app));
    release(app, KeyCode::KeyW);
    release(app, KeyCode::ShiftLeft);
    (y1 - y0, g0 - g1)
}

/// The highest surface of this map you can stand on, and the point above its middle — derived
/// out of the planned blocks, never typed. For Ashgate that is the coping of the 120 m wall.
fn highest_standing_start(app: &App) -> Vec3 {
    let d = data(app);
    let map = d.current_map().expect("maps.ron: `current` names a map").clone();
    let top = defeated_by_titan::world::map::plan_blocks(&d, &map)
        .into_iter()
        .filter(|b| b.solid)
        .max_by(|a, b| {
            (a.center_m.y + a.size_m.y * 0.5)
                .partial_cmp(&(b.center_m.y + b.size_m.y * 0.5))
                .expect("no NaN in a planned block")
        })
        .expect("a map has blocks");
    top.center_m + Vec3::Y * (top.size_m.y * 0.5 + 2.0)
}

#[test]
fn f012_the_gear_climbs_higher_than_the_fence_and_the_number_is_pinned() {
    // ## The measurement the first build asserted in prose and never took.
    //
    // `fence_top_m` stood at 200 m under the sentence *"far above anything the gear reaches on
    // gas alone"*. Measured 2026-08-28 from a standing start at (0, 2, 0), `W` + `Shift`,
    // looking up: **12.2 · 49.9 · 54.6 · 71.7 · 114.5 · 182.5 · 259.3 · 336.3 m** after one to
    // eight seconds, and the gas for the first six was 15000.000 -> 14891.771 — **0.72 % of one
    // tank**. The sentence was wrong by 3.3x in height and irrelevant in gas.
    //
    // 🔴 **And it is worse than 3.3x, because there is no apex at all.** The 657 m that round
    // reported was where its script ran out of ticks. Held long enough the body simply sits at
    // `vector.max_speed_m_s` going up: this fixture measured **3992 m in 60 s** on `graybox`.
    // The climb is bounded by the **tank**, not by the sky — so the honest ceiling is
    // `metres per unit of gas x vector.gas_tank`, and it is measured here in a ten-second
    // steady-state window instead of flown for the fourteen minutes it would really take.
    //
    // 🔴 **This test exists to go red on a retune.** It does NOT demand that the fence be
    // taller than the gear — it must not, because a taller fence is not the fix (it would still
    // have a top face to stand on) and `player::recovery::out_of_the_world` is. What it pins is
    // the MEASUREMENT, in `maps.ron: bounds.gear_ceiling_m`, so the next person who lowers
    // `vector.gas_tank` or raises `vector.boost_m_s2` finds out here rather than in the sky.
    //
    // What the answer depends on: `vector.boost_m_s2`, `player.air_accel_m_s2`,
    // `game.gravity_m_s2`, `vector.max_speed_m_s`, the gas cost of a held boost, and
    // `vector.gas_tank`.
    // What this varies: the launch point — the ground **and** the highest standable surface of
    // the map, derived from the planned blocks and not typed — and the map: `graybox` (35 m
    // skyline) and `ashgate` (a 120 m wall), whose numbers must not be the same.
    // What it holds constant, and why: the look angle (89 deg, the maximum an `Intent` allows)
    // and the two keys, because the claim is about the cheapest escape and not the best one.
    // What it skips: nothing; both maps and both launch points are measured and asserted.
    for map_key in ["graybox", "ashgate"] {
        let mut app = app_on(map_key);
        let e = me(&mut app);
        let d = data(&app);
        let map = d.current_map().expect("a map").clone();
        let tank = d.game.vector.gas_tank;
        let start = highest_standing_start(&app);

        let (climbed_g, spent_g) = steady_climb(&mut app, e, Vec3::new(0.0, 2.0, 0.0));
        let (climbed_r, spent_r) = steady_climb(&mut app, e, start);
        assert!(
            spent_g > 0.0 && spent_r > 0.0,
            "{map_key}: ten seconds of held boost spent {spent_g} / {spent_r} gas — a climb              that costs nothing is not the flight this test is about"
        );
        assert!(
            climbed_g > 100.0 && climbed_r > 100.0,
            "{map_key}: ten seconds of W+Shift straight up gained {climbed_g:.1} /              {climbed_r:.1} m — that is not a climb and the pin below would be noise"
        );

        // The ceiling one full tank buys, from the better of the two starts.
        let per_gas = (climbed_g / spent_g).max(climbed_r / spent_r);
        let base = start.y.max(2.0);
        let measured = base + per_gas * tank;
        let declared = map.bounds.gear_ceiling_m;

        // 5 %: a retune of gas, boost, gravity or the speed clamp moves this by far more, and
        // the noise of a ten-second window by far less. It is the width of "nobody touched the
        // gear", not a taste.
        assert!(
            (measured - declared).abs() <= declared * 0.05,
            "{map_key}: one full tank of {tank} lifts the gear {measured:.0} m              ({per_gas:.3} m per unit of gas, {climbed_g:.1} m / {spent_g:.2} gas from the              ground and {climbed_r:.1} m / {spent_r:.2} gas from {start:?}), and `maps.ron:              bounds.gear_ceiling_m` says {declared:.0} m. Somebody changed the gear. Re-fly it,              write {measured:.0} into `maps.ron` — and then read `fence_top_m` ({} m) again              with the new number in front of you.",
            map.bounds.fence_top_m
        );
        // And the sentence that used to stand in `maps.ron` in place of a measurement, now
        // asserted instead of believed.
        assert!(
            measured > map.bounds.fence_top_m,
            "{map_key}: the gear now stops at {measured:.0} m, BELOW the {} m fence. That would              be a real change and a welcome one — but `player::recovery::out_of_the_world` is              what holds the world together now, and this line is here so the claim in              `maps.ron` and the claim in `recovery.rs` cannot quietly drift apart.",
            map.bounds.fence_top_m
        );
    }
}

// ---------------------------------------------------------------------------------------
// 8. Water — `src/player/swim.rs`, `assets/data/water.ron`
//
// The user, 2026-08-29, asked what happens when a body meets the river:
//
//   > *„Man schwimmt / wird langsam."*
//
// So water is terrain with a cost: not lethal, not a wall. You fall in, it takes your speed,
// it holds you at the surface, and you work your way out with the gear — which costs more gas
// while it is under water (`vector::gas`, and its own tests in `src/vector/gas.rs`).
//
// The volume itself and its four deliberate absences are `tests/world.rs` §12; whether a hook
// may bite it is `src/vector/hookable.rs`. What is measured here is the RULE.
// ---------------------------------------------------------------------------------------

/// The tuning the game ships with, out of `water.ron` — never a literal in this file, so that
/// a number moving in the file moves these tests with it (`CLAUDE.md` rule 2).
fn swim_tuning() -> SwimTuning {
    GameData::load(&defeated_by_titan::data::assets_dir().join("data")).water.swim
}

/// A pool 10 m wide, 4 m deep, 100 m long with its surface at y = -0.6 — the Ashgate channel
/// at a hundredth of its length. `(volume, centre)`, the pair `depth_in` takes.
fn pool(centre_x: f32) -> (WaterVolume, Vec3) {
    (
        WaterVolume { half_size_m: Vec3::new(5.0, 1.7, 50.0), color: [0.06, 0.16, 0.22] },
        Vec3::new(centre_x, -2.275, 0.0),
    )
}

#[test]
fn f003_a_dry_body_is_not_touched_by_the_swim_rule_at_all() {
    // The first line of `swim_step`, and the one that decides whether this feature is a
    // feature or a global drag on the whole game. Both sides of the boundary AND the boundary
    // itself: `depth_m == 0.0` is dry, and it is the value the classifier writes for every
    // player on the quay on every one of the sixty ticks.
    let t = swim_tuning();
    let v = Vec3::new(12.0, -30.0, 4.0);
    for depth in [-5.0f32, -1e-6, 0.0] {
        assert_eq!(
            swim_step(v, depth, Vec3::X, &t, 1.0 / 60.0),
            v,
            "depth {depth} moved a dry body"
        );
    }
    // And one micron under the surface it is NOT untouched — without this the line above
    // would also pass for a `swim_step` that does nothing at all.
    let wet = swim_step(v, 1e-6, Vec3::X, &t, 1.0 / 60.0);
    assert_ne!(wet, v, "one micron of submersion changed nothing — is the rule wired in?");
}

#[test]
fn f003_water_takes_a_thirty_metre_dive_down_to_walking_pace_inside_a_second() {
    // *„wird langsam"*, as a number. Drag is exponential, so the claim is exact rather than a
    // feeling: `v * exp(-drag_per_s * t)`, and at `drag_per_s` 6.0 half a second is `e^-3`.
    //
    // ⚠️ The body is held at a FIXED depth here on purpose — this measures the drag and
    // nothing else. What happens when the depth is allowed to move is the float test below,
    // and lumping the two together would leave neither measured.
    let t = swim_tuning();
    let dt = 1.0 / 60.0;
    let mut v = Vec3::new(0.0, -30.0, 0.0);
    for _ in 0..30 {
        v = swim_step(v, 1.7, Vec3::ZERO, &t, dt);
        // Gravity is avian's and is added after this function every tick, so a test of the
        // rule alone has to add it too or it is measuring half a step.
        v.y += -32.0 * dt;
    }
    let speed = v.length();
    assert!(
        speed < 6.0,
        "half a second under water and the body still moves at {speed:.2} m/s — that is faster \
         than `game.ron: player.run_speed_m_s` and nothing about it says water"
    );
    // And it is the water that did it, not the arithmetic: the same half second with
    // `drag_per_s` at zero.
    let mut control = SwimTuning { drag_per_s: 0.0, ..t };
    control.buoyancy_m_s2 = 0.0;
    let mut w = Vec3::new(0.0, -30.0, 0.0);
    for _ in 0..30 {
        w = swim_step(w, 1.7, Vec3::ZERO, &control, dt);
        w.y += -32.0 * dt;
    }
    assert!(
        w.length() > 40.0,
        "the control run without drag ends at {:.2} m/s — if it does not accelerate, the number \
         above is not about water (`CLAUDE.md` rule 5: delete the thing you are measuring and \
         check the number moves)",
        w.length()
    );
}

#[test]
fn f003_a_body_dropped_into_water_floats_where_the_two_files_say_it_floats() {
    // The equilibrium is arithmetic between two files and nothing else:
    //
    //     depth = -gravity_m_s2 * surface_band_m / buoyancy_m_s2 = 32 * 1.0 / 44 = 0.727 m
    //
    // Below it the buoyancy wins and the body rises, above it gravity wins and it sinks. What
    // this integrates is that loop, with avian's gravity added by hand exactly where avian
    // adds it — after the rule, once per tick.
    let t = swim_tuning();
    let dt = 1.0 / 60.0;
    let predicted = 32.0 * t.surface_band_m / t.buoyancy_m_s2;

    let mut y = -6.0f32; // starting well under: dropped in, on the bed
    let mut v = Vec3::ZERO;
    let surface = -0.6f32;
    for _ in 0..600 {
        let depth = (surface - y).max(0.0);
        v = swim_step(v, depth, Vec3::ZERO, &t, dt);
        v.y += -32.0 * dt;
        y += v.y * dt;
    }
    let settled = surface - y;
    assert!(
        (settled - predicted).abs() < 0.05,
        "the body settled {settled:.3} m under the surface, the two files predict \
         {predicted:.3} m (gravity 32 x surface_band_m {} / buoyancy_m_s2 {})",
        t.surface_band_m,
        t.buoyancy_m_s2
    );
    // He floats — head out, not eyes under. The origin is between the feet, so a 1.8 m body
    // with 0.73 m submerged has a metre of himself in the air.
    assert!(settled < 1.0, "a body that floats {settled:.2} m under water is a body that drowns");
    assert!(settled > 0.05, "a body sitting ON the surface is not floating, it is standing");
}

#[test]
fn f003_the_swim_rule_reads_the_deepest_of_two_overlapping_waters_and_not_the_first() {
    // 🔴 **The n = 2 case, and the elements DISAGREE** — `CLAUDE.md` rule 5: a fixture that
    // passes ONE element cannot see a TWO-element bug, and `max` over a collection is exactly
    // where a per-element promise goes to die, invisible at n = 1 because there the aggregate
    // IS the element.
    //
    // Two pools whose footprints overlap at x = -70 and whose surfaces differ: the point is
    // 1.7 m under the first and 0.2 m under the second. `depth_in` must answer 1.7 — taking
    // whichever the query yielded first would make a body's buoyancy depend on spawn order.
    let deep = pool(-70.0);
    let shallow = (
        WaterVolume { half_size_m: Vec3::new(5.0, 0.2, 50.0), color: [0.0, 0.0, 0.0] },
        Vec3::new(-70.0, -2.075, 0.0),
    );
    let point = Vec3::new(-70.0, -2.275, 0.0);
    let a = depth_in(&[deep, shallow], point);
    let b = depth_in(&[shallow, deep], point);
    assert_eq!(a, b, "the answer depends on the order the two volumes arrived in");
    assert!((a - 1.7).abs() < 1e-4, "two overlapping waters answered {a} instead of 1.7");
    // And with only ONE of them the number is different — or the assertion above would hold
    // for an implementation that ignores the second volume entirely.
    assert!((depth_in(&[shallow], point) - 0.0).abs() < 1e-4);
    assert!((depth_in(&[deep], point) - 1.7).abs() < 1e-4);
    // Two pools that do NOT overlap: a body in one is not in the other.
    let far = pool(100.0);
    assert_eq!(depth_in(&[far], point), 0.0, "a pool 170 m away made the body wet");
}

#[test]
fn f003_the_swim_rule_answers_the_same_way_over_the_whole_channel_and_skips_nothing() {
    // ## The sweep, and what it holds constant (`CLAUDE.md` rule 5, the fourth shape)
    //
    // **What `swim_step` reads:** `velocity_m_s` (x, y, z), `depth_m`, `wish_dir` (x, z; y is
    // ignored by contract), `dt_s`, and five numbers out of `water.ron`.
    // **What this sweep varies:** `depth_m` (13 values, from -0.5 through exactly 0.0 to past
    // `surface_band_m` and down to the bed), `velocity_m_s` (7, including zero, a 75 m/s dive
    // at `vector.max_speed_m_s`, and a pure sideways run), `wish_dir` (5, including zero and a
    // half-held stick).
    // **What it holds constant, and why:** `dt_s` — the fixed step is the one thing the game
    // never varies (`game.ron: simulation_hz`), and a rule that depended on it would be a bug
    // this test could not name anyway; and the five tuning numbers, which are the file's and
    // are covered by the four tests above that move them one at a time.
    // **What it skips: nothing.** 455 of 455 samples are asserted, and the count is printed in
    // the failure message so that a `continue` added later cannot hide in the denominator.
    let t = swim_tuning();
    let dt = 1.0 / 60.0;
    let depths = [
        -0.5, -1e-6, 0.0, 1e-6, 0.01, 0.2, t.surface_band_m * 0.5, t.surface_band_m,
        t.surface_band_m + 1e-6, 1.0, 1.7, 2.5, 3.35,
    ];
    let velocities = [
        Vec3::ZERO,
        Vec3::new(0.0, -75.0, 0.0),
        Vec3::new(0.0, -30.0, 0.0),
        Vec3::new(30.0, 0.0, 0.0),
        Vec3::new(0.0, 12.0, 0.0),
        Vec3::new(-8.0, -8.0, -8.0),
        Vec3::new(0.0, 0.0, 45.0),
    ];
    let wishes = [
        Vec3::ZERO,
        Vec3::X,
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.5, 0.0, 0.0),
        Vec3::new(0.7, 0.0, 0.7),
    ];

    let mut checked = 0usize;
    let mut wet_samples = 0usize;
    for depth in depths {
        for v in velocities {
            for wish in wishes {
                let out = swim_step(v, depth, wish, &t, dt);
                checked += 1;
                assert!(out.is_finite(), "depth {depth} v {v:?} wish {wish:?} -> {out:?}");
                if depth <= 0.0 {
                    assert_eq!(out, v, "a dry sample at depth {depth} was changed");
                    continue;
                }
                wet_samples += 1;
                // The one promise that has to hold at EVERY depth and for EVERY velocity:
                // water never makes a body faster than it was, sideways.
                let before = Vec2::new(v.x, v.z).length();
                let after = Vec2::new(out.x, out.z).length();
                let allowed = before + t.swim_accel_m_s2 * dt + 1e-4;
                assert!(
                    after <= allowed,
                    "depth {depth} v {v:?} wish {wish:?}: horizontal {before:.4} -> {after:.4}, \
                     more than the legs may add ({:.4})",
                    t.swim_accel_m_s2 * dt
                );
                // And the lift is never downward: buoyancy may be zero at the surface, never
                // negative — gravity is avian's job and is not in this function.
                let damped_y = v.y * (-t.drag_per_s * dt).exp();
                assert!(
                    out.y >= damped_y - 1e-4,
                    "depth {depth} v {v:?}: the rule pushed the body DOWN ({:.4} -> {:.4})",
                    damped_y,
                    out.y
                );
            }
        }
    }
    assert_eq!(checked, depths.len() * velocities.len() * wishes.len());
    assert_eq!(checked, 455, "the sweep changed size — say so in the comment above");
    assert_eq!(
        wet_samples,
        10 * velocities.len() * wishes.len(),
        "10 of the 13 depths are wet by construction; {wet_samples} samples reached the water"
    );
}

#[test]
fn f003_a_body_dropped_into_the_canal_slows_down_floats_and_does_not_drown() {
    // 🟧 **The real game, on the map that ships** — not a fixture pool and not a hand-written
    // `Vec3`. The body is dropped 20 m over the channel between the bridges at z = 20 and
    // z = 110, and everything measured below comes out of the app: the water was spawned by
    // `world::water::build_water` out of `water.ron`, the fall is avian's, and the depth is
    // whatever `player::swim` computed from the volume it found.
    //
    // That is the FIND-215 shape avoided on purpose (`CLAUDE.md` rule 5): a test that hands
    // the pure function a point it invented, and an oracle fed the same invented point, cannot
    // disagree about which point it is.
    let mut app = app_on_current_map();
    if data(&app).maps.current != "ashgate" {
        return;
    }
    let me = me(&mut app);
    put_body_at(&mut app, me, Vec3::new(-70.0, 20.0, 60.0));

    // Fall. 20 m at 32 m/s² is 1.12 s; give it two seconds and watch the whole thing.
    let mut peak_speed = 0.0f32;
    let mut wettest = 0.0f32;
    let mut lowest = f32::INFINITY;
    for _ in 0..120 {
        app.update();
        let v = app.world().get::<Velocity>(me).expect("velocity").0.length();
        let depth = app.world().get::<Submerged>(me).expect("submerged").depth_m;
        let y = at(&app, me).y;
        if depth == 0.0 && y > -0.6 {
            peak_speed = peak_speed.max(v);
        }
        wettest = wettest.max(depth);
        lowest = lowest.min(y);
    }
    assert!(
        peak_speed > 15.0,
        "the body only reached {peak_speed:.2} m/s on the way down — it never fell into \
         anything and the numbers below are about standing still"
    );
    assert!(wettest > 0.5, "the deepest this body ever got was {wettest:.3} m — it missed the \
         water, or `build_water` put none there");

    // Two more seconds to settle.
    for _ in 0..120 {
        app.update();
    }
    let settled_depth = app.world().get::<Submerged>(me).expect("submerged").depth_m;
    let settled_speed = app.world().get::<Velocity>(me).expect("velocity").0.length();
    let y = at(&app, me).y;

    assert!(
        settled_speed < 2.0,
        "four seconds in the river and the body still moves at {settled_speed:.2} m/s"
    );
    assert!(
        settled_depth > 0.05 && settled_depth < 1.5,
        "the body sits {settled_depth:.3} m under the surface — floating is neither 0 nor the \
         bed of the channel"
    );
    assert!(
        y > -3.9,
        "the body is at y = {y:.2} and the channel floor is at -4.00: it sank to the bottom"
    );
    // Not lethal: *„Man schwimmt"*, and nothing in this game drowns yet.
    assert_eq!(
        state(&app, me),
        MovementState::Airborne,
        "a floating body is not `Downed` and not `Grounded`"
    );
}

#[test]
fn f003_a_hook_fired_from_inside_the_water_still_finds_the_quay_above_it() {
    // *„Nein — Wasser haelt keinen Haken"* must not turn into *"no hook works in water"*: the
    // river is escapable, and the shot that escapes it is fired from inside it. This is the
    // test that would have caught the `Sensor` version of the water — avian clamps `tmin` to
    // 0 for a ray whose origin is inside a shape (`raycast3d.rs:64`), so a collider here would
    // answer every escape shot at distance 0, with the crosshair on the quay.
    let mut app = app_on_current_map();
    if data(&app).maps.current != "ashgate" {
        return;
    }
    let me = me(&mut app);
    put_body_at(&mut app, me, Vec3::new(-70.0, 20.0, 60.0));
    for _ in 0..180 {
        app.update();
    }
    let depth = app.world().get::<Submerged>(me).expect("submerged").depth_m;
    assert!(depth > 0.05, "the body is not in the water ({depth:.3} m) — nothing below is a \
         statement about a shot from the water");

    // Look at the east quay, 5 m away and 1.0 m above the surface, and fire. `yaw = -90 deg`
    // is +X (`shared::Intent::look_dir`: yaw 0 is -Z), pitch level — the eye of a floating
    // body sits at 0.27 m and the quay face spans -4.00 .. +0.40, so a level ray meets stone.
    hold(&mut app, KeyCode::KeyQ);
    for _ in 0..12 {
        app.world_mut().resource_mut::<LookOverride>().0 =
            Some((-std::f32::consts::FRAC_PI_2, 0.0));
        app.update();
    }
    let hook = app.world().get::<Hook>(me).expect("the player has a hook");
    assert!(
        hook.anchored_count() > 0,
        "a hook fired from inside the water anchored nothing — the river has become a wall the \
         rope cannot get through"
    );
}
