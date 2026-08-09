//! `F-004` pendulum and `F-005` reel-in — the guard over the rope.
//!
//! The rope is an avian `DistanceJoint` with `limits = (0, L)`. That is not a taste
//! decision, it is `docs/measurements/rope-decision.md`, and these tests are the criteria
//! that decision was made against, written down before the code was:
//!
//! - reeling in **gains** speed (58.23 m/s out of `v0 = 20` in the measurement, against
//!   exactly 20.000 for the hand-written clamp that was retired),
//! - the shortening happens **per substep**, not per tick (per tick injects
//!   `rate x SubstepCount` = 677 m/s and drives the player through walls),
//! - the swing loses little speed per second (4.26 %/s measured at 24 substeps),
//! - the rope pulls and never pushes,
//! - it never gets shorter than `vector.min_rope_m`,
//! - and letting go really removes the joint.
//!
//! ## Every number comes out of `GameData`
//!
//! Not one literal from `game.ron` stands in this file. A test that measures against a value
//! it hard-codes itself stays green on the day somebody hard-codes the same value in the Rust
//! — which is exactly the failure `tests/vector_hooks.rs` describes for the hook speed.
//!
//! ## Two things these tests do that need a reason
//!
//! 1. **They write `AimPoint` themselves**, through a system of their own, and put the
//!    carrier into `SpatialIndex` by hand. Same reason as in `tests/vector_hooks.rs`:
//!    `world::index::maintain_index` (`T-036a`) is a stub, so no body in the real world
//!    carries a `BodyId` yet and every real shot ends as `NoAnchor`.
//! 2. **The swing tests run with `Gravity(ZERO)`.** That is how the measurement was taken
//!    (`examples/probe_avian.rs::schwung_fahren`: gravity 0, anchor `L` above the player,
//!    `v0` sideways) and it is the only way the number means anything: with gravity on, a
//!    pendulum's speed swings by ±100 % from height alone, and what you would be measuring is
//!    `g`, not the solver. `Gravity` is a resource, so the test says so out loud instead of
//!    the code having a switch for it. The reel-in test keeps gravity **on** — there the
//!    number has to hold in the real world.

use avian3d::prelude::{DistanceJoint, Gravity, LinearVelocity, Position};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{
    AimPoint, BodyId, BodyMask, Cli, HookState, IndexEntry, LocalPlayer, PlayerId, RopeLength,
    Side, SimulationSystems, SpatialIndex, WarpPlayer,
};
use defeated_by_titan::vector::aim::aim;

// ---------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------

/// What `vector::aim` would write if `T-036a` were built. See the module header.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ForcedAim(AimPoint);

fn force_aim(mut players: Query<(&ForcedAim, &mut AimPoint)>) {
    for (forced, mut point) in &mut players {
        if *point != forced.0 {
            *point = forced.0;
        }
    }
}

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.add_systems(FixedUpdate, force_aim.in_set(SimulationSystems::World).after(aim));
    app.update(); // Startup: the city and the local player come into being
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

/// The one local player. Not `.single()` — every player is one of many (§6 rule 3).
fn me(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

fn player_id(app: &App, e: Entity) -> PlayerId {
    *app.world().get::<PlayerId>(e).expect("every player has a stable id")
}

fn position(app: &App, e: Entity) -> Vec3 {
    app.world().get::<Position>(e).expect("the player is a physics body").0
}

fn velocity(app: &App, e: Entity) -> Vec3 {
    app.world().get::<LinearVelocity>(e).expect("the player is a physics body").0
}

fn set_velocity(app: &mut App, e: Entity, v: Vec3) {
    app.world_mut().get_mut::<LinearVelocity>(e).expect("the player is a physics body").0 = v;
}

fn rope_length(app: &App, e: Entity, side: Side) -> f32 {
    app.world()
        .get::<RopeLength>(e)
        .expect("every player carries a RopeLength")
        .length_m(side)
}

/// How many rope joints exist in the whole world.
fn joint_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&DistanceJoint>();
    q.iter(app.world()).count()
}

/// Warps the player exactly there and stops him dead. The sanctioned path (§12c) — and the
/// only one that does not fight avian over `Position`.
fn warp(app: &mut App, e: Entity, to: Vec3) {
    let id = player_id(app, e);
    app.world_mut().write_message(WarpPlayer {
        player: id,
        pos_x: to.x,
        pos_y: to.y,
        pos_z: to.z,
    });
}

/// Presses and holds the reel-in key. `src/net/local.rs` maps `ControlLeft` onto
/// `Buttons::REEL_IN` — the test presses the same key a human does.
fn hold_reel_in(app: &mut App) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ControlLeft);
}

/// Hangs the player on a rope of about `nominal_length_m`, at `player_pos`, with the anchor
/// straight **above** him. Returns the length the joint really got.
///
/// The player is warped back onto his spot in **every** tick of the hook's flight, so that
/// the length the rope is born with is the one this function names and not one plus however
/// far he fell in the meantime. The warp stops the moment the hook bites.
fn hang(app: &mut App, e: Entity, player_pos: Vec3, nominal_length_m: f32) -> f32 {
    let anchor = player_pos + Vec3::Y * nominal_length_m;
    let body = BodyId(80_001);
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m: anchor + Vec3::Y * 2.0,
        half_size_m: Vec3::splat(2.0),
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(anchor),
        body: Some(body),
        anchorable: true,
    }));

    warp(app, e, player_pos);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);

    for _ in 0..600 {
        warp(app, e, player_pos);
        app.update();
        let anchored = app
            .world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .expect("every player carries both arms")
            .arm(Side::Left)
            .state
            .is_anchored();
        if anchored {
            // One more tick without a warp, so `sync_rope_length` has published a length that
            // belongs to a player who really stands where he stands.
            app.update();
            let l = rope_length(app, e, Side::Left);
            assert!(l > 0.0, "the hook bit and no rope came into being");
            return l;
        }
    }
    panic!(
        "the hook did not bite within 600 ticks — it is {:?}",
        app.world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .map(|h| h.arm(Side::Left).state)
            .unwrap_or(HookState::Idle)
    );
}

fn kill_gravity(app: &mut App) {
    app.insert_resource(Gravity(Vec3::ZERO));
}

/// The anchor of the one rope that exists. Read out of the world, not remembered by the test
/// — that is what makes the "the joint is really gone" assertion mean something.
fn anchor_point(app: &mut App) -> Option<Vec3> {
    let mut q = app.world_mut().query::<&DistanceJoint>();
    let anchor = q.iter(app.world()).next().map(|j| j.body1)?;
    app.world().get::<Position>(anchor).map(|p| p.0)
}

// ---------------------------------------------------------------------------------------
// `F-005` — the reel-in, and the two numbers the whole round hangs on
// ---------------------------------------------------------------------------------------

#[test]
fn f005_reeling_in_gains_speed_beyond_the_start() {
    // **The criterion the round hangs on.** Reeling in through the joint preserves angular
    // momentum: `v * L` stays put, so a third of the length is three times the speed. The
    // measurement got 58.23 m/s out of `v0 = 20`; the hand-written solver that was retired
    // got exactly 20.000 — it ate the reel-in, and the reel-in is the feel of the gear.
    //
    // Gravity stays ON here. This number has to hold in the real world, not in a vacuum.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let min_rope_m = d.game.vector.min_rope_m;
    let max_speed_m_s = d.game.vector.max_speed_m_s;

    // Three times the floor, so that "about a third of the start length" is exactly the floor
    // and the test does not have to guess where to stop.
    let start_length_m = min_rope_m * 3.0;
    let v0 = 20.0;

    // High above the city: this test measures the rope, not a roof.
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), start_length_m);
    set_velocity(&mut app, e, Vec3::new(v0, 0.0, 0.0));

    // Measured **at the end of the reel-in**, not some fixed number of ticks later. Once the
    // rope is at its floor the pendulum starts trading the speed back for height against
    // gravity — that is physics, not the rope, and a test that waits measures `g`. (For the
    // record: 120 ticks later the same run reads 41.55 m/s.)
    hold_reel_in(&mut app);
    let mut peak = 0.0f32;
    let mut speed = 0.0f32;
    let mut end_length_m = l0;
    for _ in 0..120 {
        app.update();
        peak = peak.max(velocity(&app, e).length());
        speed = velocity(&app, e).length();
        end_length_m = rope_length(&app, e, Side::Left);
        if end_length_m <= min_rope_m + 1e-4 {
            break;
        }
    }

    assert!(
        (end_length_m - min_rope_m).abs() < 0.01,
        "the rope ran from {l0:.3} m to {end_length_m:.3} m, expected the floor at \
         {min_rope_m} m (game.ron: vector.min_rope_m) — the reel-in did not run"
    );
    assert!(
        speed >= 45.0,
        "from v0 = {v0} m/s at {l0:.2} m down to {end_length_m:.2} m the player reached only \
         {speed:.2} m/s (peak {peak:.2}). The joint has to preserve angular momentum — the \
         measurement got 58.23 m/s. A clamp that eats the reel-in gives back exactly v0."
    );
    assert!(
        peak <= max_speed_m_s + 0.1,
        "the player reached {peak:.2} m/s, the file allows {max_speed_m_s} \
         (game.ron: vector.max_speed_m_s) — MaxLinearSpeed is not on the body"
    );
}

#[test]
fn f005_shortening_happens_per_substep_not_per_tick() {
    // **The test that goes red when somebody "simplifies" the substep system into
    // `FixedUpdate`.** One tick of reeling shortens the rope by `reel_speed / simulation_hz`
    // — once, not once per substep. Shortening per tick instead injects
    // `rate x SubstepCount` and the measurement watched the player reach 677.66 m/s and go
    // 2.53 m through a wall.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as f32;
    let substeps = d.game.substeps as f32;
    let per_tick_m = d.game.vector.reel_speed_m_s / hz;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 4.0);

    hold_reel_in(&mut app);
    app.update();
    let after_one_tick = rope_length(&app, e, Side::Left);
    let shortened = l0 - after_one_tick;

    assert!(
        (shortened - per_tick_m).abs() <= per_tick_m * 0.01,
        "one tick of reeling took {shortened:.5} m off the rope; the file says \
         {per_tick_m:.5} m (vector.reel_speed_m_s / simulation_hz) ± 1 %. \
         {:.5} m would be once per substep applied {substeps} times over — that is the \
         per-tick bug the measurement clocked at 677.66 m/s.",
        per_tick_m * substeps
    );
}

#[test]
fn f005_the_rope_never_gets_shorter_than_the_file_says() {
    // `vector.min_rope_m`: any closer would drag the camera into the wall. Read from
    // `GameData`, never from a literal — and reeled at for far longer than it takes to get
    // there, so a missing clamp runs the length negative and the joint starts pushing.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let min_rope_m = d.game.vector.min_rope_m;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), min_rope_m * 4.0);
    // `unlimited` gas, so that an empty tank is not what stops the reel-in.
    app.world_mut()
        .get_mut::<defeated_by_titan::shared::Gas>(e)
        .expect("every player carries a tank")
        .unlimited = true;

    hold_reel_in(&mut app);
    let mut shortest = l0;
    for _ in 0..300 {
        app.update();
        shortest = shortest.min(rope_length(&app, e, Side::Left));
    }

    assert!(
        shortest >= min_rope_m - 1e-4,
        "the rope got down to {shortest:.5} m, the file allows {min_rope_m} m \
         (game.ron: vector.min_rope_m)"
    );
    assert!(
        (shortest - min_rope_m).abs() < 0.01,
        "the rope stopped at {shortest:.5} m instead of at the floor {min_rope_m} m — it is \
         not the file that stopped it"
    );
}

// ---------------------------------------------------------------------------------------
// `F-004` — the pendulum
// ---------------------------------------------------------------------------------------

#[test]
fn f004_a_swing_loses_little_speed_per_second() {
    // Without gravity a pendulum is a pure circle and **every** loss of speed is solver
    // damping. That is how `examples/probe_avian.rs::schwung_fahren` measured it: 4.26 %/s at
    // 24 substeps. The criterion is 6 %/s. For scale: the pure radial projection that was
    // considered and rejected loses 99.2 %/s at `L = 3 m, v = 75 m/s`.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as u64;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 8.0);

    let v0 = 20.0;
    set_velocity(&mut app, e, Vec3::new(v0, 0.0, 0.0));
    ticks(&mut app, hz); // exactly one second

    let end = velocity(&app, e).length();
    let loss_per_s = (1.0 - end / v0) * 100.0;
    assert!(
        loss_per_s <= 6.0,
        "over one second on a {l0:.2} m rope the swing fell from {v0:.2} to {end:.2} m/s — \
         {loss_per_s:.2} %/s, the criterion is 6 %/s (measured 4.26 %/s at \
         {} substeps)",
        d.game.substeps
    );
    // And the other direction: a rope that gains speed out of nothing is not a rope.
    assert!(
        end <= v0 * 1.01,
        "the swing ended at {end:.2} m/s having started at {v0:.2} — a constraint that adds \
         energy is not a constraint"
    );
}

#[test]
fn f004_the_rope_pulls_but_does_not_push() {
    // `limits.min = 0.0` is what says so: `DistanceLimit::compute_correction` corrects only
    // above the maximum. A player who is closer to his anchor than `L` has to be able to stay
    // there — a rope is not a rod, and pushing is what a spring would do.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 4.0);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    // Straight at the anchor, at half his rope's length, and then let go of everything.
    let half = l0 * 0.5;
    warp(&mut app, e, anchor - Vec3::Y * half);
    app.update();
    set_velocity(&mut app, e, Vec3::ZERO);

    let start = (position(&app, e) - anchor).length();
    ticks(&mut app, 60);
    let end = (position(&app, e) - anchor).length();

    assert!(
        end <= start + 0.05,
        "the player sat {start:.3} m under an anchor whose rope is {l0:.3} m long and was \
         pushed out to {end:.3} m. A rope pulls; it does not push."
    );
    assert!(
        end < l0 - 0.5,
        "he ended up at {end:.3} m of {l0:.3} m — something drove him to the full length"
    );
}

#[test]
fn f004_releasing_the_hook_removes_the_joint() {
    // Every release reason goes through `HookReleased`. A joint that outlives its hook is an
    // invisible rope: the player hangs on nothing anybody can see, and no message will ever
    // come to free him.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 3.0);
    assert_eq!(joint_count(&mut app), 1, "one hook, one joint");
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    // Let go of the mouse button — the same input a human gives.
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().release(MouseButton::Left);
    set_velocity(&mut app, e, Vec3::new(20.0, 0.0, 0.0));
    app.update();

    assert!(
        !app.world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .expect("every player carries both arms")
            .arm(Side::Left)
            .state
            .is_anchored(),
        "the hook did not let go"
    );
    assert_eq!(
        joint_count(&mut app),
        0,
        "the hook let go and {} joint(s) are still holding the player",
        joint_count(&mut app)
    );
    assert_eq!(
        rope_length(&app, e, Side::Left),
        0.0,
        "`RopeLength` still claims a constraint that no longer exists — 0.0 means \
         'no constraint' (src/shared/gear.rs)"
    );

    // And he really flies free: 60 ticks at 20 m/s put him far beyond his old rope's length.
    ticks(&mut app, 60);
    let distance = (position(&app, e) - anchor).length();
    assert!(
        distance > l0 * 2.0,
        "one second after letting go he is {distance:.2} m from the anchor, his rope was \
         {l0:.2} m — something is still holding him"
    );
}
