//! `F-007` Gas boost — the ways it can be wrong that a screenshot never shows.
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
//! 5. **The rope steers the strength instead of the direction** (from 2026-08-10, section 5
//!    below, where its own five failure modes are spelled out). A `lerp` of two unit vectors
//!    is not a unit vector, and that one missing `normalize` makes `boost_m_s2` depend on
//!    where the player happens to be looking.
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
use defeated_by_titan::shared::{
    BodyId, BodyMask, BoostAccel, Buttons, Cli, Gas, GasGrant, Hook, HookArm, HookState,
    IdCounter, IndexEntry, Intent, Side, SpatialIndex,
};
use defeated_by_titan::vector::boost::{boost_direction, rope_dir};

/// Builds the **real** app, headless, one simulation step per `update()`, on the map named
/// here — **not** on whatever `maps.ron: current` happens to say.
///
/// Every test in this file flies a second player 200 m above the city and measures an
/// acceleration, a clamp or a gas cost. None of them is a claim about a district — but a
/// flier only touches nothing as long as nothing is up there, and the free volume over the
/// origin belongs to the level design. On 2026-08-12 `current` moved to `ashgate` and
/// [`f007_the_boost_does_not_outrun_the_top_speed`] measured 0.0000 m/s instead of the clamp,
/// without a line of `vector::boost` having changed. So the map is pinned.
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

// ---------------------------------------------------------------------------------------
// 5. The rope steers the boost (user, 2026-08-10)
// ---------------------------------------------------------------------------------------
//
// *"wenn man boostet soll man in richtung seil und mauszeiger fliegen! also dahin. dass wenn
// man zur hook schaut und gehookt ist und boostet man stark in die richtung fliegt!"*
//
// The five ways this can be wrong, and none of them shows up in a screenshot:
//
// 1. **The blend changes the STRENGTH.** `lerp` of two unit vectors is not a unit vector — it
//    is shorter than 1 everywhere except at the ends, by up to 30 % at 90 degrees. Whoever
//    forgets the `normalize` afterwards has silently made `boost_m_s2` depend on where the
//    player is looking. Every test below asserts the length separately from the direction.
// 2. **The old behaviour is gone.** `boost_rope_fraction: 0.0` has to reproduce the look-only
//    boost **bit for bit** — that is what makes the new key a knob and not a rewrite.
// 3. **NaN.** The player exactly on his anchor, or looking exactly away from it at 0.5: both
//    produce a zero vector somewhere in the middle, and `normalize` turns that into NaN. NaN
//    in a `Transform` is how a player vanishes from the world (§9d).
// 4. **The rope steers while there is no rope.** Unhooked, the fraction may do nothing at all,
//    at any value.
// 5. **Two ropes are not one rope.** With both arms anchored the two directions have to meet
//    in the middle, not "the left one wins because it is index 0".

/// The look direction for a given yaw/pitch in degrees — the same construction the game uses,
/// so a test never hand-writes a direction the `Intent` would not produce.
fn look(yaw_deg: f32, pitch_deg: f32) -> Vec3 {
    Intent { yaw: yaw_deg.to_radians(), pitch: pitch_deg.to_radians(), ..default() }.look_dir()
}

/// An anchored arm whose tip sits at `tip_m`.
fn anchored(tip_m: Vec3, body: u32) -> HookArm {
    HookArm { state: HookState::Anchored { body: BodyId(body), local_m: Vec3::ZERO }, tip_m }
}

#[test]
fn f007_at_zero_the_rope_does_not_move_the_boost_by_a_single_bit() {
    // THE regression guard. `boost_rope_fraction: 0.0` is the behaviour of every `f007_*` test
    // above it, and it is checked for **bit** equality, not for "close": a blend that is
    // mathematically the identity but numerically 0.9999997 turns every exact assertion in
    // this file into a flake, and the four tests above would start failing for a reason nobody
    // could find.
    let l = look(30.0, 20.0);
    let anchor = Some(Vec3::X);
    assert_eq!(
        boost_direction(l, anchor, 0.0),
        l,
        "at boost_rope_fraction = 0 the rope moved the boost direction — then the new key is \
         not a knob, it is a rewrite of F-007"
    );
    // And with no rope at all, trivially the same.
    assert_eq!(boost_direction(l, None, 0.0), l);
}

#[test]
fn f007_looking_at_the_anchor_boosts_along_the_rope_at_full_strength() {
    // Both inputs agree, so the blend cannot change the direction whatever the fraction is —
    // and that is exactly the case in which a missing `normalize` still gives the right
    // direction and the wrong length. Hence the length assertion.
    let l = look(-90.0, 0.0); // +X (docs/conventions.md: yaw 0 looks along -Z)
    for w in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let d = boost_direction(l, Some(Vec3::X), w);
        assert!(
            (d - Vec3::X).length() < 1e-6,
            "looking straight at the anchor at fraction {w} gives {d:?}, not the rope direction"
        );
        assert!(
            (d.length() - 1.0).abs() < 1e-6,
            "the boost direction is {} long at fraction {w} — the blend changed the STRENGTH, \
             and boost_m_s2 now depends on where the player looks",
            d.length()
        );
    }
}

#[test]
fn f007_looking_ninety_degrees_off_the_anchor_lands_between_the_two() {
    // The real case: hooked to the right, looking forward. The result has to lie **between**
    // look and rope — on the rope's side, never past it, never on the far side.
    let l = Vec3::NEG_Z; // forward
    let rope = Vec3::X; // the anchor is off to the right
    let d = boost_direction(l, Some(rope), 0.5);

    assert!(
        (d.length() - 1.0).abs() < 1e-6,
        "the blend is {} long instead of 1 — a lerp of two unit vectors is NOT a unit vector, \
         and that is this feature's one arithmetic trap",
        d.length()
    );
    // Strictly between: positive along both inputs, and past neither of them.
    assert!(d.dot(l) > 0.0 && d.dot(l) < 1.0, "{d:?} does not lie on the look side any more");
    assert!(d.dot(rope) > 0.0 && d.dot(rope) < 1.0, "{d:?} does not lean toward the rope");
    assert!(d.y.abs() < 1e-6, "{d:?} left the plane both inputs live in");
    // At exactly half it is the bisector: 45 degrees off each.
    assert!(
        (d.dot(l) - d.dot(rope)).abs() < 1e-6,
        "at 0.5 the direction is not the bisector: {:.4} along look against {:.4} along rope",
        d.dot(l),
        d.dot(rope)
    );
    // And it really moves with the knob — more fraction, more rope.
    let quarter = boost_direction(l, Some(rope), 0.25);
    assert!(
        quarter.dot(rope) < d.dot(rope),
        "0.25 leans as far toward the rope as 0.5 does — the fraction is not being read"
    );
    // Outside 0..1 it is not a blend any more but an extrapolation, and at 2.0 that points
    // AWAY from the look direction. It is clamped, and that is a decision, not an accident.
    assert_eq!(boost_direction(l, Some(rope), 2.0), rope, "above 1 the rope wins outright");
    assert_eq!(boost_direction(l, Some(rope), -1.0), l, "below 0 the look direction wins");
}

#[test]
fn f007_without_a_hook_the_fraction_does_nothing_at_any_value() {
    let l = look(30.0, 20.0);
    for w in [0.0, 0.5, 1.0, 2.0, -1.0] {
        assert_eq!(
            boost_direction(l, None, w),
            l,
            "unhooked at fraction {w} the boost left the look direction — there is no rope to \
             lean on"
        );
    }
}

#[test]
fn f007_the_two_degenerate_cases_fall_back_to_the_look_direction() {
    let l = Vec3::NEG_Z;

    // (a) The player exactly on his anchor: `rope_dir` has no direction to give. The system
    //     hands over `None` and the boost is the look direction — not NaN.
    let hook = Hook { arms: [anchored(Vec3::new(5.0, 1.0, 5.0), 7), HookArm::default()] };
    assert_eq!(
        rope_dir(Vec3::new(5.0, 1.0, 5.0), &hook),
        None,
        "standing exactly on the anchor produced a direction — normalize(ZERO) is NaN, and NaN \
         in a Transform is how a player vanishes (§9d)"
    );

    // (b) Looking exactly AWAY from the anchor at 0.5: the lerp is the zero vector, and
    //     normalizing it is NaN. This is not a theoretical case — it is "hooked behind you,
    //     boosting forward", which happens in every swing.
    let d = boost_direction(l, Some(-l), 0.5);
    assert!(d.is_finite(), "look opposite rope at 0.5 gave {d:?} — that is a NaN player");
    assert_eq!(d, l, "the fallback for an undecidable blend is the look direction, {d:?} is not");
    assert!((d.length() - 1.0).abs() < 1e-6, "the fallback is {} long, not 1", d.length());

    // Two ropes pointing exactly opposite each other cancel the same way.
    let opposed = Hook {
        arms: [anchored(Vec3::new(10.0, 0.0, 0.0), 7), anchored(Vec3::new(-10.0, 0.0, 0.0), 8)],
    };
    assert_eq!(
        rope_dir(Vec3::ZERO, &opposed),
        None,
        "two exactly opposed ropes have no mean direction, and the answer is None, not NaN"
    );
}

#[test]
fn f007_two_anchors_pull_the_boost_to_the_middle_between_them() {
    // One rope forward-right, one forward-left. The mean of the two **unit** directions is
    // straight forward — the near anchor does not outvote the far one, because a direction has
    // no length. (A mean of the two anchor POINTS would; that is the mistake this pins down.)
    let hook = Hook {
        arms: [anchored(Vec3::new(3.0, 0.0, -3.0), 7), anchored(Vec3::new(-30.0, 0.0, -30.0), 8)],
    };
    let d = rope_dir(Vec3::ZERO, &hook).expect("two anchored arms are a direction");
    assert!(
        (d - Vec3::NEG_Z).length() < 1e-6,
        "the mean of the two ropes is {d:?}, not straight ahead — the 4.2 m rope outvoted the \
         42 m one, so the mean was taken over the points instead of the directions"
    );
    assert!((d.length() - 1.0).abs() < 1e-6, "the mean rope direction is {} long", d.length());

    // One arm anchored, one idle: only the anchored one counts.
    let one = Hook { arms: [anchored(Vec3::new(0.0, 0.0, -8.0), 7), HookArm::default()] };
    assert_eq!(rope_dir(Vec3::ZERO, &one), Some(Vec3::NEG_Z));
    assert_eq!(rope_dir(Vec3::ZERO, &Hook::default()), None, "an idle gear is not a rope");
}

// --- and the same thing through the real app, because a pure function proves nothing about
// --- what the running game actually writes into `BoostAccel`.

/// Anchors the flier's left arm to a body `offset_m` away from his hand, and holds the button
/// that keeps it anchored.
///
/// `update_hooks` (`SimulationSystems::Intent`) runs **before** `gas_boost`
/// (`SimulationSystems::Drive`), re-derives `tip_m` from the spatial index every tick and
/// releases the arm the moment the hook button is not held (`ReleaseReason::Released`) or the
/// carrier is not in the index (`BodyGone`). So both are needed here, and the returned anchor
/// is the one the boost will really see.
fn hang_on(app: &mut App, e: Entity, offset_m: Vec3) -> Vec3 {
    let eye = app.world().resource::<GameData>().game.player.eye_height_m;
    let hand = app.world().get::<Transform>(e).expect("a player has a transform").translation
        + Vec3::Y * eye;
    let anchor = hand + offset_m;
    let body = BodyId(80_007);
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m: anchor,
        half_size_m: Vec3::splat(1.0),
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    let mut hook = app.world_mut().get_mut::<Hook>(e).expect("a player carries both arms");
    hook.arms[Side::Left.index()] =
        HookArm { state: HookState::Anchored { body, local_m: Vec3::ZERO }, tip_m: anchor };
    let mut intent = app.world_mut().get_mut::<Intent>(e).expect("a player has an intent");
    intent.buttons.set(Buttons::HOOK_LEFT, true);
    anchor
}

#[test]
fn f007_in_the_running_game_a_hooked_boost_leans_toward_the_anchor() {
    // Hooked 60 m to the right (+X), looking straight ahead (−Z). With the file's fraction the
    // drive has to leave the look axis and lean toward the rope — and keep its length.
    //
    // Measured `[cachy]` at `boost_rope_fraction: 0.5`: **BoostAccel = (24.0416, 0.0, −24.0416),
    // length exactly 34.0** — the 45-degree bisector of look and rope, at full strength. With
    // the rope ignored it was `(−0.0, 0.0, −34.0)`, which is what the first red of this test
    // reported.
    let mut app = app();
    let d = data(&app);
    let a = d.game.vector.boost_m_s2;
    let w = d.game.vector.boost_rope_fraction;
    let e = flier(&mut app, 0.0);
    hang_on(&mut app, e, Vec3::X * 60.0);

    boost(&mut app, e, 0.0, 0.0); // yaw 0 = look along −Z
    ticks(&mut app, 1);

    let got = drive(&app, e);
    assert!(
        (got.length() - a).abs() < 1e-3,
        "BoostAccel is {} m/s^2 long instead of game.ron's boost_m_s2 = {a} — the blend \
         changed the strength",
        got.length()
    );
    assert!(
        got.x > 0.0,
        "BoostAccel is {got:?}: it does not lean toward the anchor at all, although \
         game.ron: vector.boost_rope_fraction = {w}"
    );
    assert!(got.z < 0.0, "BoostAccel is {got:?}: it gave the look direction up entirely");
    // The exact vector, against the same rule computed by hand from what the game holds.
    let hand = app.world().get::<Transform>(e).expect("transform").translation
        + Vec3::Y * d.game.player.eye_height_m;
    let hook = *app.world().get::<Hook>(e).expect("hooks");
    let want = boost_direction(look(0.0, 0.0), rope_dir(hand, &hook), w) * a;
    assert!(
        (got - want).length() < 0.05,
        "BoostAccel is {got:?}, the rule says {want:?} — the system is not applying the rule \
         the tests above pin down"
    );
}

#[test]
fn f007_in_the_running_game_at_zero_a_hooked_boost_is_the_pure_look_direction() {
    // The regression guard once more, but through the whole app: with the knob at 0 a hooked
    // player boosts exactly where he looks — **bit for bit** the pre-2026-08-10 behaviour.
    let mut app = app();
    let a = app.world().resource::<GameData>().game.vector.boost_m_s2;
    app.world_mut().resource_mut::<GameData>().game.vector.boost_rope_fraction = 0.0;
    let e = flier(&mut app, 0.0);
    hang_on(&mut app, e, Vec3::X * 60.0);

    boost(&mut app, e, 30.0, 20.0);
    ticks(&mut app, 1);

    assert_eq!(
        drive(&app, e),
        look(30.0, 20.0) * a,
        "at boost_rope_fraction = 0 a hooked player no longer boosts where he looks"
    );
}
