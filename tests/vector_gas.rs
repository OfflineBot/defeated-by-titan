//! `F-018` — the gas budget, measured against `assets/data/game.ron`.
//!
//! The unit tests of the booking itself sit in `src/vector/gas.rs`; they drive `book()`
//! directly and can flip the priority order, which a test may not do to the real RON file.
//! **What is checked here is the other half:** that the system really runs, in the real app,
//! once per tick, off the real numbers, and driven by the real keys a human presses.
//!
//! Five ways this can be wrong that no unit test gets hold of:
//!
//! 1. The cost is multiplied by `dt` twice (or not at all). The tank then drains 60 times too
//!    slowly or 60 times too fast, and both look plausible while playing.
//! 2. `gas_budget` is not registered in `FixedUpdate` at all, or lands outside the simulation
//!    set — nothing panics, the tank simply never empties.
//! 3. Two consumers each debit the full amount because the grant is ignored.
//! 4. `--sandbox` runs out of gas after 5.6 s, and the mode meant for looking around becomes
//!    unusable in exactly the situation it exists for.
//! 5. The gas hangs on `LocalPlayer` and the second player flies on somebody else's tank.
//!
//! ## Why these tests drive with `app.update()`
//!
//! The same reason as in `tests/player.rs`: `Time<Fixed>` only becomes the source of the
//! generic `Time` inside `run_fixed_main_schedule`, and running `FixedMain` by hand skips
//! that. `TimeUpdateStrategy::FixedTimesteps(1)` advances real time by exactly one timestep
//! per `App::update()` (`bevy_time-0.19.0/src/lib.rs:181-183`), so one `update()` is exactly
//! one simulation step — and the tick count is a number you can compute the gas from.
//!
//! The image belonging to these numbers is `docs/images/f-018-gas.png`, taken with
//! `scripts/f-018-gas.txt`. **It does not show the gas level** — `hud::gas_bar` is an empty
//! stub, and until it is filled the tank is nowhere on screen (`src/hud/mod.rs`). What the
//! run does deliver is the same measurement as here, with its own exit code.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::{GameData, GasConsumer};
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    BodyId, Buttons, Cli, Gas, GasGrant, Hook, HookState, IdCounter, Intent, LocalPlayer, Side,
};

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app_with(start: Cli) -> App {
    let mut app = defeated_by_titan::app(start);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update(); // Startup: the city and the local player come into being
    app
}

fn app() -> App {
    app_with(Cli { headless: true, ..default() })
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

fn gas(app: &App, e: Entity) -> Gas {
    *app.world().get::<Gas>(e).expect("every player carries a tank")
}

fn grant(app: &App, e: Entity) -> GasGrant {
    *app.world().get::<GasGrant>(e).expect("every player carries this tick's booking")
}

fn set_tank(app: &mut App, e: Entity, current: f32) {
    app.world_mut().get_mut::<Gas>(e).expect("player has a tank").current = current;
}

/// Presses a real key — the same input a human triggers and the same one `--script` uses.
/// Writing into `Intent` would not work for the local player: `net::local::read_input`
/// rebuilds it from the keyboard every tick.
fn hold(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
}

fn release(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(key);
}

/// `ShiftLeft` is boost, `ControlLeft` is reel-in (`src/net/local.rs:57-58`). Named here so
/// that a rebinding breaks one line and not six tests.
const BOOST_KEY: KeyCode = KeyCode::ShiftLeft;
const REEL_KEY: KeyCode = KeyCode::ControlLeft;

/// Hangs the left hook on a body. **Set again before every tick** in the loops below:
/// `vector::hook::update_hooks` is being filled by another job right now, and a test that
/// silently stops measuring reel-in the day that lands is worse than no test.
fn anchor_left(app: &mut App, e: Entity) {
    let mut hook = app.world_mut().get_mut::<Hook>(e).expect("every player carries two hooks");
    hook.arms[Side::Left.index()].state =
        HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO };
}

/// A second player, without the `LocalPlayer` marker — the way a team mate arrives later.
fn second_player(app: &mut App, pos: Vec3) -> Entity {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, pos, false);
    *world.resource_mut::<IdCounter>() = ids;
    app.update();
    e
}

// ---------------------------------------------------------------------------------------
// 1. The consumption per second is the number in the file — measured against the tick count
// ---------------------------------------------------------------------------------------

#[test]
fn f018_a_second_of_boost_costs_exactly_the_value_from_the_file() {
    // THE test against "multiplied by dt twice". At 18/s and 60 Hz one tick costs 0.3; 60
    // ticks cost 18. Multiplying by dt a second time gives 0.005 per second, which nobody
    // notices while playing and everybody notices at balancing time.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60); // land first; nothing is pressed, so nothing is spent

    let before = gas(&app, e);
    assert!(
        (before.current - d.game.vector.gas_tank).abs() < 1e-6,
        "the tank starts full: {} instead of {}",
        before.current,
        d.game.vector.gas_tank
    );

    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 60); // exactly one second
    release(&mut app, BOOST_KEY);

    let spent = before.current - gas(&app, e).current;
    let expected = d.game.vector.gas_boost_per_s;
    assert!(
        (spent - expected).abs() < 0.01,
        "60 ticks of boost cost {spent:.4} gas; game.ron says gas_boost_per_s = {expected}"
    );
    assert!(
        grant(&app, e).boost,
        "the tank is full and the button held — the grant has to say so"
    );
}

#[test]
fn f018_boost_and_reel_in_together_cost_the_sum_and_not_one_of_them() {
    // The case the whole booking exists for: both consumers in the same tick. One debit per
    // consumer, exactly once — not twice, and not one of the two silently dropped because the
    // other one ran first.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    let before = gas(&app, e).current;
    hold(&mut app, BOOST_KEY);
    hold(&mut app, REEL_KEY);
    for _ in 0..60 {
        anchor_left(&mut app, e);
        app.update();
    }
    release(&mut app, BOOST_KEY);
    release(&mut app, REEL_KEY);

    let spent = before - gas(&app, e).current;
    let expected = d.game.vector.gas_boost_per_s + d.game.vector.gas_reel_per_s;
    assert!(
        (spent - expected).abs() < 0.01,
        "one second of boost AND reel-in cost {spent:.4}; game.ron says {} + {} = {expected}",
        d.game.vector.gas_boost_per_s,
        d.game.vector.gas_reel_per_s
    );
    let g = grant(&app, e);
    assert!(g.boost && g.reel_in, "on a full tank both are served: {g:?}");
}

#[test]
fn f018_reeling_in_without_an_anchored_hook_costs_nothing() {
    // The cost follows the effect, not the button: pressing reel-in in free fall pulls on
    // nothing. Without this the tank empties while falling and nobody sees why.
    let mut app = app();
    let e = me(&mut app);
    ticks(&mut app, 60);

    let before = gas(&app, e).current;
    hold(&mut app, REEL_KEY);
    ticks(&mut app, 60);
    release(&mut app, REEL_KEY);

    assert!(
        (before - gas(&app, e).current).abs() < 1e-6,
        "a second of reeling in nothing cost {:.4} gas",
        before - gas(&app, e).current
    );
    assert!(!grant(&app, e).reel_in, "nothing is being reeled in, so nothing is granted");
}

// ---------------------------------------------------------------------------------------
// 2. A tank that covers only one of them — and the file decides who
// ---------------------------------------------------------------------------------------

#[test]
fn f018_a_tank_for_one_consumer_follows_the_order_in_the_file() {
    // Deliberately **not** "Boost wins": the winner is read out of `gas_priority`, so that
    // this test still measures the right thing after the user has flipped the two lines.
    let mut app = app();
    let d = data(&app);
    let v = &d.game.vector;
    let e = me(&mut app);
    ticks(&mut app, 60);

    let dt = 1.0 / d.game.simulation_hz as f32;
    let boost_cost = v.gas_boost_per_s * dt;
    let reel_cost = v.gas_reel_per_s * dt;
    // Enough for each of them on its own, not enough for both.
    let tank = boost_cost.max(reel_cost) + 0.5 * boost_cost.min(reel_cost);
    set_tank(&mut app, e, tank);

    hold(&mut app, BOOST_KEY);
    hold(&mut app, REEL_KEY);
    anchor_left(&mut app, e);
    app.update();
    release(&mut app, BOOST_KEY);
    release(&mut app, REEL_KEY);

    let g = grant(&app, e);
    assert_eq!(
        u8::from(g.boost) + u8::from(g.reel_in),
        1,
        "a tank of {tank:.4} covers exactly one of {boost_cost:.4} / {reel_cost:.4} — \
         served was {g:?}"
    );
    let winner = if g.boost { GasConsumer::Boost } else { GasConsumer::ReelIn };
    assert_eq!(
        winner, v.gas_priority[0],
        "game.ron names {:?} first, the drop went to {winner:?}",
        v.gas_priority[0]
    );
}

#[test]
fn f018_an_empty_tank_yields_no_half_boost() {
    // `F-018` in its own words: "at 0 no more flying, only ground movement". Half a boost
    // would be harder to explain than none.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    let dt = 1.0 / d.game.simulation_hz as f32;
    let third = d.game.vector.gas_boost_per_s * dt / 3.0;
    set_tank(&mut app, e, third);

    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 10);
    release(&mut app, BOOST_KEY);

    assert!(!grant(&app, e).boost, "a third of a tick's cost is not a boost");
    assert!(
        (gas(&app, e).current - third).abs() < 1e-6,
        "a refused boost costs nothing — the tank holds {} instead of {third}",
        gas(&app, e).current
    );
}

#[test]
fn f018_the_tank_runs_dry_and_stays_at_zero_instead_of_going_negative() {
    // 100 gas at 18/s is 5.55 s of boost. Ten seconds of holding must not push the tank
    // below zero, and must not flip the grant back on.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 600); // ten seconds
    release(&mut app, BOOST_KEY);

    let dt = 1.0 / d.game.simulation_hz as f32;
    let one_tick = d.game.vector.gas_boost_per_s * dt;
    let left = gas(&app, e).current;
    assert!(
        (0.0..one_tick).contains(&left),
        "after ten seconds of boost the tank holds {left}, expected 0 .. {one_tick} \
         (a refused boost leaves the last, insufficient drop lying)"
    );
    assert!(!grant(&app, e).boost, "an empty tank grants no boost");
}

#[test]
fn f018_the_grant_does_not_outlive_the_button_by_one_tick() {
    // A grant that stays standing is a free boost. Written every tick, for everybody, by
    // assignment — that is why there is no clearing system.
    let mut app = app();
    let e = me(&mut app);
    ticks(&mut app, 60);

    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 2);
    assert!(grant(&app, e).boost, "the button is held, so the grant stands");

    release(&mut app, BOOST_KEY);
    ticks(&mut app, 1);
    assert!(!grant(&app, e).boost, "the button was released — the grant has to be gone");
}

// ---------------------------------------------------------------------------------------
// 3. `--sandbox`
// ---------------------------------------------------------------------------------------

#[test]
fn f018_the_sandbox_tank_never_runs_out() {
    // `--sandbox` exists for looking around (§12a). A tank of 100 at 24/s would be empty
    // after 4.2 s, and exactly the mode meant for looking would be the one you cannot fly in.
    let mut app = app_with(Cli { headless: true, sandbox: true, ..default() });
    let d = data(&app);
    let e = me(&mut app);
    assert!(gas(&app, e).unlimited, "--sandbox gives an unlimited tank");
    ticks(&mut app, 60);

    hold(&mut app, BOOST_KEY);
    hold(&mut app, REEL_KEY);
    for _ in 0..600 {
        anchor_left(&mut app, e);
        app.update();
    }
    release(&mut app, BOOST_KEY);
    release(&mut app, REEL_KEY);

    let tank = gas(&app, e);
    assert!(
        (tank.current - d.game.vector.gas_tank).abs() < 1e-6,
        "ten seconds of boost and reel-in in the sandbox left {} of {}",
        tank.current,
        d.game.vector.gas_tank
    );
    assert!(!tank.is_empty());
    let g = grant(&app, e);
    assert!(g.boost && g.reel_in, "in the sandbox everybody is served: {g:?}");
}

// ---------------------------------------------------------------------------------------
// 4. There is no such thing as THE tank
// ---------------------------------------------------------------------------------------

#[test]
fn f018_every_player_pays_out_of_his_own_tank() {
    // Gas that hangs on `LocalPlayer` would be a single-player game you notice as one in
    // month twelve (§6 rule 3, docs/multiplayer.md).
    let mut app = app();
    let d = data(&app);
    let mine = me(&mut app);
    // 8 m to the side, inside `maps.ron: layout.clear_radius_m` — no house there.
    let other = second_player(&mut app, Vec3::new(8.0, 2.0, 0.0));
    ticks(&mut app, 60);

    // The second player is not driven by the keyboard: `net::deliver_intents` only writes to
    // players who have mail, and he has none. So his `Intent` stands until somebody changes
    // it — which is exactly how the network will feed him later.
    let mut intent = app.world_mut().get_mut::<Intent>(other).expect("he has an intent");
    intent.buttons.set(Buttons::BOOST, true);

    let mine_before = gas(&app, mine).current;
    let other_before = gas(&app, other).current;
    ticks(&mut app, 60);

    let spent_other = other_before - gas(&app, other).current;
    assert!(
        (spent_other - d.game.vector.gas_boost_per_s).abs() < 0.01,
        "the second player boosted for a second and paid {spent_other:.4}"
    );
    assert!(
        (mine_before - gas(&app, mine).current).abs() < 1e-6,
        "I pressed nothing and still lost {:.4} gas",
        mine_before - gas(&app, mine).current
    );
    assert!(grant(&app, other).boost, "his booking, not mine");
    assert!(!grant(&app, mine).boost, "and mine stays empty");
}

// ---------------------------------------------------------------------------------------
// 5. The file itself
// ---------------------------------------------------------------------------------------

#[test]
fn f018_the_priority_list_names_every_consumer_exactly_once() {
    // `gas_priority` is an ORDER, so it has to be complete. A consumer missing from the list
    // would silently never get fuel — the quiet failure this project has no time for. And a
    // doubled entry is a data error even though `vector::gas::book` refuses to debit twice.
    let d = data(&app());
    let list = &d.game.vector.gas_priority;
    for consumer in [GasConsumer::Boost, GasConsumer::ReelIn] {
        let n = list.iter().filter(|c| **c == consumer).count();
        assert_eq!(n, 1, "game.ron: vector.gas_priority names {consumer:?} {n} times: {list:?}");
    }
    assert_eq!(list.len(), 2, "and nothing else: {list:?}");
}

#[test]
fn f018_the_costs_in_the_file_are_positive_and_the_tank_outlasts_a_swing() {
    // Not balancing — a guard against a zero that would make the whole feature invisible
    // rather than broken.
    let d = data(&app());
    let v = &d.game.vector;
    assert!(v.gas_boost_per_s > 0.0, "a boost that costs nothing is not a resource");
    assert!(v.gas_reel_per_s > 0.0, "reel-in that costs nothing is not a resource");
    assert!(v.gas_tank > 0.0);
    let seconds = v.gas_tank / (v.gas_boost_per_s + v.gas_reel_per_s);
    assert!(
        seconds > 2.0,
        "a full tank lasts {seconds:.2} s of boost plus reel-in — below two seconds nobody \
         crosses a street, and the numbers in game.ron would be a different game"
    );
}
