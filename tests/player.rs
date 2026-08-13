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
use defeated_by_titan::player::locomotion::{SteerTuning, air_thrust, rope_steer};
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
    // And nothing sideways: at 0° there is no tangent to spend anything on.
    let sideways = (a - towards * closing).length();
    assert!(sideways < 0.05, "{sideways:.4} m/s² across a rope the player is looking straight down");
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
    let tangential = (a - towards * radial).length();
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
        let pull = rope_steer(&[anchor], anchor.normalize(), 0.0, 0.0, 1.0, t).length();
        assert!(pull > previous, "the fade is not monotone at {length_m:.2} m: {pull} <= {previous}");
        assert!(pull < full, "at {length_m:.2} m the pull is already the full {full}");
        previous = pull;
    }

    // And at the top of the band it is the whole number, with no fade left to pay.
    let anchor = anchor_at(0.0, t.min_rope_m + t.fade_m);
    let pull = rope_steer(&[anchor], anchor.normalize(), 0.0, 0.0, 1.0, t).length();
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
    let hauling = run_accel(&app, full).length();
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
