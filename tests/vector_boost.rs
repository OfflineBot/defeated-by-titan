//! `F-007` Gas boost — the four ways it can be wrong that a screenshot never shows.
//!
//! The acceptance of `F-007` reads "boost produces noticeable acceleration". "Noticeable" is
//! not a criterion, so these tests measure against `assets/data/game.ron` instead:
//!
//! 1. **A factor of `dt` too many or too few.** `boost_m_s2 = 34` has to be 34 m/s², not
//!    34 m/s and not 0.567 m/s². Applied once per tick instead of continuously it is 60 times
//!    too small; written as a velocity it is 60 times too large. Both look "kind of fast" in a
//!    picture, and both make every number in the file a suggestion.
//! 2. **A force instead of an acceleration.** The capsule's `ComputedMass` is **0.6029 kg**
//!    (measured `[offlinebot]`, avian's default density), so a force number would silently
//!    mean something else for every body that ever gets a mass of its own. Two players of
//!    different mass boosting side by side is the test that separates the two.
//! 3. **The clamp does not hold.** `F-012` exists from day one against fling exploits — a
//!    boost that outruns it is exactly the exploit.
//! 4. **Half a boost on an empty tank.** `F-018` says: at 0 there is no more flying. Not
//!    "less flying".
//!
//! ## Why these tests drive with `app.update()`
//!
//! Same reason as `tests/player.rs`: avian takes its step size from the *generic* `Time`
//! (`avian3d-0.7.0/src/schedule/mod.rs:238-244`), which only `run_fixed_main_schedule` switches
//! over to `Time<Fixed>`. Running `FixedMain` by hand therefore steps the physics with the last
//! wall-clock delta and measures the machine instead of the game.
//! `TimeUpdateStrategy::FixedTimesteps(1)` makes one `App::update()` exactly one simulation
//! step (`bevy_time-0.19.0/src/lib.rs:181-183`).
//!
//! ## Why every test here flies a SECOND player, high above the city
//!
//! Two reasons, both measured rather than assumed:
//!
//! - **On the ground the horizontal velocity is not the boost's.**
//!   `player::locomotion::ground_locomotion` **assigns** `LinearVelocity.x/z` on every tick a
//!   player is `Grounded`. The boost still lands (it is an acceleration, applied inside the
//!   step), but it is thrown away again at the start of the next tick, so a grounded player
//!   tops out at `run_speed_m_s + boost_m_s2/60` and never accumulates. That is a real
//!   interaction and it is reported, not tested around — but it is not what `F-007` is about.
//! - **The local player's `Intent` is refilled from the keyboard on every tick**
//!   (`net::local::read_input`), so it cannot be dictated. A second player gets no mail
//!   (`net::deliver_intents` only writes players with a due intent), which makes his `Intent`
//!   the one thing in this app you can set once and rely on.
//!
//! The picture that belongs to these numbers is `docs/images/f-007-boost.png`, taken with
//! `scripts/f-007-boost.txt`.

use avian3d::prelude::{LinearVelocity, Mass, MaxLinearSpeed, NoAutoMass};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{BoostAccel, Buttons, Cli, Gas, GasGrant, IdCounter, Intent};

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
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

/// A second player, 200 m above the city — far above the tallest block (18 m) and far enough
/// that four seconds of falling touch nothing.
///
/// `x_m` is not decoration. Two players spawned on the same spot are two overlapping dynamic
/// capsules, and avian pushes them apart: measured, that cost **1.10 m/s of the 68** after two
/// seconds, and the difference looked exactly like a physics bug in the boost. Whoever needs
/// two fliers gives them different `x_m`.
fn flier(app: &mut App, x_m: f32) -> Entity {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, Vec3::new(x_m, 200.0, 0.0), false);
    *world.resource_mut::<IdCounter>() = ids;
    app.update();
    e
}

/// Hold the boost button and grant the gas for it.
///
/// **Both**, and on purpose. `vector::gas::gas_budget` (`F-018`) recomputes [`GasGrant`] from
/// the button and the tank on every tick, so the grant written here is only the seed for the
/// first one; from the second tick on it is `F-018`'s. Pressing the button as well is what
/// makes it stay `true` — the tank is full (`spawn_player` gives `Gas::full(gas_tank)` = 100,
/// which pays for 5.5 s at `gas_boost_per_s = 18`), so nothing in these tests runs dry.
/// Written this way the tests also held while `gas_budget` was still an empty stub, which is
/// what made them runnable before `F-018` landed.
fn boost(app: &mut App, e: Entity, yaw_deg: f32, pitch_deg: f32) {
    let mut intent = app.world_mut().get_mut::<Intent>(e).expect("a player has an intent");
    intent.yaw = yaw_deg.to_radians();
    intent.pitch = pitch_deg.to_radians();
    intent.buttons.set(Buttons::BOOST, true);
    let mut grant = app.world_mut().get_mut::<GasGrant>(e).expect("a player has a gas grant");
    grant.boost = true;
}

fn velocity(app: &App, e: Entity) -> Vec3 {
    app.world().get::<LinearVelocity>(e).expect("a player is a physics body").0
}

fn drive(app: &App, e: Entity) -> Vec3 {
    app.world().get::<BoostAccel>(e).expect("a player carries the boost drive").0
}

// ---------------------------------------------------------------------------------------
// 1. The number in the file is m/s², and it arrives as m/s²
// ---------------------------------------------------------------------------------------

#[test]
fn f007_two_seconds_of_boost_are_worth_exactly_two_seconds_of_boost() {
    // THE test of this feature. `MaxLinearSpeed` is taken off on purpose: with the clamp in
    // place this measures `F-012` and not `F-007`.
    //
    // Measured along the look axis alone, because gravity owns Y. At `yaw = 0, pitch = 0` the
    // look direction is −Z (docs/conventions.md), so the whole boost sits in `v.z`.
    //
    // Counter-check driven `[offlinebot]`: swap `apply_linear_acceleration` for `apply_force`
    // in `vector::boost` and this reports **112.7853 m/s instead of 68** — 34 newtons on a
    // 0.6 kg capsule. Restored, green again at 68.0028.
    let mut app = app();
    let d = data(&app);
    let a = d.game.vector.boost_m_s2;
    let e = flier(&mut app, 0.0);
    app.world_mut().entity_mut(e).remove::<MaxLinearSpeed>();

    boost(&mut app, e, 0.0, 0.0);
    ticks(&mut app, 120); // 2 s at 60 Hz

    let v = velocity(&app, e);
    let along_look = -v.z;
    let expected = a * 2.0;
    assert!(
        (along_look - expected).abs() <= expected * 0.01,
        "after 2 s of boost he flies at {along_look:.4} m/s along the look axis; \
         game.ron: vector.boost_m_s2 = {a} m/s^2, so it has to be {expected} ± 1 %"
    );
    // And it really is the look axis, not "somewhere fast".
    assert!(v.x.abs() < 1e-3, "the boost drifts sideways: v.x = {}", v.x);
    assert!(v.z < 0.0, "the boost points along +Z; yaw = 0 means −Z (docs/conventions.md)");
    // Gravity is untouched by all of this — the boost adds to the world, it does not replace
    // it. 2 s at −20 m/s^2 are −40 m/s.
    let fall = d.game.gravity_m_s2 * 2.0;
    assert!(
        (v.y - fall).abs() <= fall.abs() * 0.01,
        "he falls at {} m/s instead of {fall} — a boost must not switch gravity off",
        v.y
    );
}

#[test]
fn f007_the_drive_is_the_look_direction_times_the_number_from_the_file() {
    // The drive is what `hud`, `sound` and `F-006`/`F-008` will read. It is an acceleration in
    // m/s^2 — **not** already multiplied by anything. A `* dt` anywhere in this chain shows up
    // here as a length of 0.567 instead of 34.
    let mut app = app();
    let d = data(&app);
    let a = d.game.vector.boost_m_s2;
    let e = flier(&mut app, 0.0);

    boost(&mut app, e, 30.0, 20.0);
    ticks(&mut app, 1);

    let want = Intent { yaw: 30f32.to_radians(), pitch: 20f32.to_radians(), ..default() }
        .look_dir()
        * a;
    let got = drive(&app, e);
    assert!(
        (got - want).length() < 1e-4,
        "BoostAccel is {got:?}, expected {want:?} (look direction * vector.boost_m_s2 = {a})"
    );
    assert!(
        (got.length() - a).abs() < 1e-4,
        "the drive is {} m/s^2 long instead of {a} — something multiplied a delta into it",
        got.length()
    );
}

// ---------------------------------------------------------------------------------------
// 2. Acceleration, not force: mass may not appear in the game values
// ---------------------------------------------------------------------------------------

#[test]
fn f007_a_heavy_player_boosts_exactly_as_fast_as_a_light_one() {
    // The one test that separates `apply_linear_acceleration` from `apply_force`. With a force
    // the ten-times-heavier body would come out ten times slower — and `boost_m_s2` in
    // `game.ron` would silently be a number about the capsule's density instead of about the
    // game.
    //
    // Counter-check driven `[offlinebot]`: with `apply_force` the two report **−112.7853 and
    // −7.6834 m/s**. With the acceleration they are not merely close, they are **bit-identical**
    // (−68.002785 both) — which is what "ignoring mass" has to mean.
    let mut app = app();
    let light = flier(&mut app, 0.0);
    let heavy = flier(&mut app, 30.0);
    app.world_mut().entity_mut(light).remove::<MaxLinearSpeed>();
    app.world_mut()
        .entity_mut(heavy)
        .remove::<MaxLinearSpeed>()
        // `NoAutoMass` next to `Mass`: without it the collider's own mass is added on top
        // (avian3d-0.7.0/src/dynamics/rigid_body/mass_properties/components/mod.rs:110-131).
        .insert((Mass(10.0), NoAutoMass));

    boost(&mut app, light, 0.0, 0.0);
    boost(&mut app, heavy, 0.0, 0.0);
    ticks(&mut app, 120);

    let a = velocity(&app, light).z;
    let b = velocity(&app, heavy).z;
    assert!(
        (a - b).abs() < 1e-3,
        "the light player reaches {a:.4} m/s and the heavy one {b:.4} m/s — the boost is a \
         force, not an acceleration, and every number in game.ron now depends on the mass"
    );
}

// ---------------------------------------------------------------------------------------
// 3. F-012 — the clamp wins over the boost
// ---------------------------------------------------------------------------------------

#[test]
fn f007_the_boost_does_not_outrun_the_top_speed() {
    // 34 m/s^2 alongside 20 m/s^2 of gravity reach 75 m/s after 1.9 s. Four seconds are far
    // past that, and still below the 5.5 s of gas a full tank pays for — so this measures the
    // clamp and not an empty tank.
    let mut app = app();
    let d = data(&app);
    let max = d.game.vector.max_speed_m_s;
    let e = flier(&mut app, 0.0);

    boost(&mut app, e, 0.0, 0.0);
    ticks(&mut app, 240); // 4 s

    let speed = velocity(&app, e).length();
    assert!(
        speed <= max + 1e-3,
        "the boost carries him to {speed:.4} m/s, game.ron: vector.max_speed_m_s = {max} \
         (bible 6.4: the clamp exists from day one, against fling exploits)"
    );
    assert!(
        speed > max - 0.01,
        "{speed:.4} m/s is below the clamp — then this test is measuring something other than \
         the clamp, and F-007 is not producing the acceleration it promises"
    );
}

// ---------------------------------------------------------------------------------------
// 4. F-018 — at 0 there is no more flying. Not less flying.
// ---------------------------------------------------------------------------------------

#[test]
fn f007_a_held_button_without_a_grant_produces_exactly_zero() {
    // The edge case, and it is checked for **exactly** zero, not for "small": half a boost on
    // an empty tank is harder to explain than none, and a fraction is what you get when a
    // system multiplies the drive by a remaining-gas ratio somewhere.
    //
    // Counter-check driven `[offlinebot]`: replace the `if grant.boost` in `vector::boost` by a
    // factor of 0.1 for a missing grant — a "little bit of reserve" is exactly the plausible
    // mistake — and this reports `BoostAccel = Vec3(-0.0, 0.0, -3.4)` instead of `ZERO`.
    let mut app = app();
    let e = flier(&mut app, 0.0);
    app.world_mut().entity_mut(e).remove::<MaxLinearSpeed>();

    // Button held, tank empty, no grant — all three, so that this stays the same test after
    // `F-018` fills `vector::gas::gas_budget`.
    let mut intent = app.world_mut().get_mut::<Intent>(e).expect("a player has an intent");
    intent.buttons.set(Buttons::BOOST, true);
    let mut gas = app.world_mut().get_mut::<Gas>(e).expect("a player has a tank");
    gas.current = 0.0;
    let mut grant = app.world_mut().get_mut::<GasGrant>(e).expect("a player has a gas grant");
    grant.boost = false;

    ticks(&mut app, 120);

    assert_eq!(
        drive(&app, e),
        Vec3::ZERO,
        "the drive holds {:?} without a grant — F-018: at 0 there is no more flying",
        drive(&app, e)
    );
    let v = velocity(&app, e);
    assert_eq!(v.x, 0.0, "he drifts at {} m/s in x without a single grant of gas", v.x);
    assert_eq!(v.z, 0.0, "he drifts at {} m/s in z without a single grant of gas", v.z);
}
