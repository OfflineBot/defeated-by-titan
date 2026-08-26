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
use defeated_by_titan::shared::{
    Block, BodyId, Buttons, Cli, Gas, Hook, HookState, IdCounter, Intent, LocalPlayer, MovementState,
    PlayerId, RunAccel, Side, SpatialIndex, Velocity,
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
    // v0 = 6.5 m/s at g = −20: 6.5·0.2 − 10·0.04 = 0.90 m.
    let after_02 = at(&app, e).y;
    assert!(
        (after_02 - 0.9).abs() < 0.05,
        "0.2 s after the jump he is at {after_02} m instead of 0.90 (v0 = {v0}, g = {})",
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
    // Momentum that never ends is not a chain, it is ice. At `-gravity_m_s2` = 20 m/s² a
    // 20 m/s slide needs 20/20 = 1.00 s and 10.0 m — inside `clear_radius_m`.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    let decel = -d.game.gravity_m_s2;
    launch_on_the_ground(&mut app, e, 20.0);

    // It must not be instant — that is exactly the bug this feature is about.
    ticks(&mut app, 2);
    assert!(
        ground_speed(&app, e) > 19.0,
        "two ticks after arriving at 20 m/s he is already at {:.4} m/s",
        ground_speed(&app, e)
    );

    // Halfway: 20 − 20·0.5 = 10 m/s.
    ticks(&mut app, 28); // 30 ticks = 0.5 s in total
    let halfway = ground_speed(&app, e);
    assert!(
        (halfway - 10.0).abs() < 1.0,
        "0.5 s into the slide he is at {halfway:.4} m/s; at {decel} m/s² it has to be 10 m/s"
    );

    // And 20/20 = 1.00 s in he is standing. 90 ticks = 1.5 s leaves half a second of slack.
    ticks(&mut app, 60);
    let end = ground_speed(&app, e);
    assert!(
        end < 0.01,
        "1.5 s after a 20 m/s slide he still moves at {end:.4} m/s — at {decel} m/s² the stop \
         is due after 1.00 s"
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
    // `locomotion::air_control` at `-gravity_m_s2 / 2`. Measured `[offlinebot]`: the legs used
    // to turn this by **22.44°**, the air control turns it by **11.22°**, and with an empty
    // tank by **5.67°** (`f006_above_the_threshold_the_legs_stop_steering_and_the_air_takes_over`
    // is the other half of this pair). **The margin over the 10° below is 1.22°** — whoever
    // lowers the air control below ≈ 0.53·g makes this test go red, and that is the guard
    // working, not a flaky test (`docs/FINDINGS.md` FIND-051).
    let mut app = app();
    let e = me(&mut app);
    launch_on_the_ground(&mut app, e, 30.0);

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
    // And the turn cost him only the ordinary deceleration: 30 − 20·0.5 = 20 m/s.
    let speed = v.length();
    assert!(
        (speed - 20.0).abs() < 1.5,
        "steering left him at {speed:.4} m/s instead of the 20 m/s the plain deceleration \
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
    // The magnitude is derived, not typed: `-gravity_m_s2 / 2` — "the air control is half of
    // gravity, so WASD alone can never hold you up". See `src/player/locomotion.rs`. The day
    // it becomes `game.ron: player.air_accel_m_s2` this line reads that key.
    let a = -d.game.gravity_m_s2 / 2.0;

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
    let d = data(&app);
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
    ticks(&mut app, 12); // 0.2 s: v0·0.2 − 10·0.04 = 0.90 m
    release(&mut app, KeyCode::Space);
    let risen = at(&app, e).y - before;
    assert!(
        (risen - 0.9).abs() < 0.05,
        "0.2 s after a jump with a hook in the wall he has risen {risen:.4} m instead of 0.90 \
         — an anchored hook must not cost the player his jump"
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

fn winch_tuning(d: &GameData) -> WinchTuning {
    WinchTuning {
        speed_m_s: d.game.vector.reel_speed_m_s,
        min_rope_m: d.game.vector.min_rope_m,
        ramp_s: d.game.vector.drive_ramp_s,
        // `Ctrl`'s own value — see `WinchTuning::accel_max_m_s2` for why the key `FIND-159`
        // measured keeps its behaviour and the always-on pull does not.
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
fn f005_under_the_drive_ctrl_winds_in_a_player_who_is_standing_still_on_the_ground() {
    // **The wiring, and it is the one that `scripts/game-full.txt` ACT 1 hangs on.** The act
    // begins with the player standing still — `MovementState::Grounded`, so [`in_flight`] is
    // false — and climbs a 35 m church roof on `Ctrl` alone. Under `Pendulum` that works
    // because the reel moves the joint's `limits.max` and never asks about the player's state;
    // the winch is an acceleration on the **body**, so it had to be lifted out of
    // `air_control`'s flight branch or ACT 1 would have measured `Height 0.300` forever — which
    // is exactly what the pushed build did measure, four asserts red.
    //
    // Red by putting the winch back inside `if in_flight(...)`.
    let mut app = app();
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model =
        defeated_by_titan::data::RopeForceModel::Drive;
    ticks(&mut app, 30); // the player lands and settles on the graybox floor
    let e = me(&mut app);
    let d = data(&app);
    assert_eq!(
        state(&app, e),
        MovementState::Grounded,
        "the fixture is a player STANDING — if he is airborne this test proves nothing"
    );

    let to_anchor = anchor_on_a_body_with_rope_left(&mut app, e, &d);
    let along = to_anchor.normalize();
    let velocity_before = app.world().get::<Velocity>(e).expect("a player carries a Velocity").0;

    // WARNING: the closing speed is read BEFORE the tick, not after it. `air_control` decides
    // on `Velocity`, which `player::integrator::readback` writes at the END of the previous
    // tick — reading `LinearVelocity` after `update()` measures the tick's own result and the
    // number came out 22 m/s2 short. A fixture bug this test made once, kept as a warning.
    let closing = velocity_before.dot(along);
    let due = (d.game.vector.reel_speed_m_s - closing) / d.game.vector.drive_ramp_s;

    hold(&mut app, KeyCode::ControlLeft);
    app.update();

    let a = run_accel(&app, e);
    assert!(
        (a - along * due).length() < 1.0,
        "`Ctrl` on a {:.1} m rope has to wind a STANDING player in at ({} − {closing:.2})/{} = \
         {due:.1} m/s² along it. Measured {a:?}",
        to_anchor.length(),
        d.game.vector.reel_speed_m_s,
        d.game.vector.drive_ramp_s,
    );

    // The control that makes the number mean something: the same tick, the same held key, the
    // hook let go of. If this is not ZERO the test above was measuring gravity, a jump or the
    // look term — not the rope. (`CLAUDE.md` §6 rule 5: delete the thing you think you are
    // measuring and check the number moves.)
    let_go_of_the_left_hook(&mut app, e);
    app.update();
    assert_eq!(
        run_accel(&app, e),
        Vec3::ZERO,
        "with no rope the very same held `Ctrl` must move nothing at all"
    );
}

#[test]
fn f005_under_the_pendulum_ctrl_adds_no_acceleration_at_all() {
    // The model fork. The pendulum's reel is a **length**, carried out by
    // `player::rope::shorten_ropes` on the joint — it must never grow a second, additive
    // acceleration on the body, or the reel would be paid once and delivered twice.
    //
    // Red by deleting the `RopeForceModel::Drive` arm of the winch `match` (making it fire under
    // both models).
    let mut app = app();
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model =
        defeated_by_titan::data::RopeForceModel::Pendulum;
    ticks(&mut app, 30);
    let e = me(&mut app);
    let d = data(&app);
    // The same fixture as the `Drive` test, and it has to be: a rope already at the floor makes
    // this assertion pass no matter what the `match` says.
    anchor_on_a_body_with_rope_left(&mut app, e, &d);
    hold(&mut app, KeyCode::ControlLeft);
    app.update();

    assert_eq!(
        run_accel(&app, e),
        Vec3::ZERO,
        "under `Pendulum` the reel is the joint's business and this function contributes nothing"
    );
}
