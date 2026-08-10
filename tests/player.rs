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
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    Block, BodyId, Cli, Hook, HookState, IdCounter, LocalPlayer, MovementState, PlayerId, Side,
    SpatialIndex, Velocity,
};

/// Builds the **real** app, headless, one simulation step per `update()`.
///
/// Not a second, similar one — otherwise the test proves nothing about the game that is
/// actually played (the same argument as in `tests/multiplayer.rs`).
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update(); // Startup: the city and the local player come into being
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
    let mut app = app();
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
