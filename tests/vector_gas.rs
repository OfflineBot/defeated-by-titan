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
    BodyId, Buttons, Cli, Gas, GasGrant, Hook, HookState, IdCounter, Intent, LocalPlayer,
    PlayerId, RefuelRequest, Side,
};

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app_with(start: Cli) -> App {
    let mut app = defeated_by_titan::app(start);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<GasChanges>();
    // `FixedLast` and not `Last`: it runs in the same fixed step as `gas_budget`, so one
    // counted change is one simulation tick and not one frame that may have held none or two.
    app.add_systems(FixedLast, count_gas_changes);
    app.update(); // Startup: the city and the local player come into being
    app
}

/// How often a `Gas` was marked `Changed` — **the signal, not the value.**
///
/// `vector::gas` refuses to touch a tank nobody is using, because `Changed<Gas>` is what the
/// HUD reads and a tank that reports a change every tick without changing is a lie in that
/// signal. A refill that runs on a full tank forever would be exactly that lie, and it is
/// invisible in every assert on `Gas::current` — which is why it gets counted here.
#[derive(Resource, Default)]
struct GasChanges(u32);

fn count_gas_changes(mut n: ResMut<GasChanges>, changed: Query<(), Changed<Gas>>) {
    n.0 += changed.iter().count() as u32;
}

fn gas_changes(app: &App) -> u32 {
    app.world().resource::<GasChanges>().0
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

fn hz(d: &GameData) -> f32 {
    d.game.simulation_hz as f32
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
    // How long the tank lasts is a number from the file, so the length of this run is too:
    // `gas_tank / gas_boost_per_s` seconds, and half as long again on top. (It was a literal
    // `600` — ten seconds — while `gas_tank` was 100; at 300 that run stops while the tank is
    // still two thirds full and the test stops measuring what it is named after.)
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    let dry_ticks = (d.game.vector.gas_tank / d.game.vector.gas_boost_per_s * hz(&d)) as u64;
    hold(&mut app, BOOST_KEY);
    ticks(&mut app, dry_ticks + dry_ticks / 2);
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
    for consumer in
        [GasConsumer::Boost, GasConsumer::Steer, GasConsumer::ReelIn, GasConsumer::Dodge]
    {
        let n = list.iter().filter(|c| **c == consumer).count();
        assert_eq!(n, 1, "game.ron: vector.gas_priority names {consumer:?} {n} times: {list:?}");
    }
    assert_eq!(list.len(), 4, "and nothing else: {list:?}");
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

// ---------------------------------------------------------------------------------------
// 6. Nothing refills the tank (`docs/QUESTIONS.md` Q-033, answered 2026-08-12)
//
// The user: *"gas refillt nur im main gebäude an bestimmten stationen/objekten"*. Refuelling
// is **a place you go to**, never a rate — so the simulation has one writer that lowers the
// tank and none that raises it, and the tests below are what keeps it that way. They are also
// the **station rules in advance**: when the refuel stations of `docs/NEXT.md` §1d get built,
// "nothing refills while spending" and "never above `max`" are exactly the two claims they
// will have to keep, and the first test is the one that will then need a station standing next
// to the player before it may be relaxed.
//
// Between 2026-08-10 and 2026-08-12 this section measured a 10/s regeneration behind a 0.5 s
// pause — an assumption made while the question was open. Four tests went with it.
// ---------------------------------------------------------------------------------------

#[test]
fn f018_an_idle_tank_never_refills_on_its_own() {
    // ★ **The new truth, and it goes red against the regeneration.** The user answered Q-033
    // on 2026-08-12: *"gas refillt nur im main gebäude an bestimmten stationen/objekten"* —
    // gas comes back **at a place you go to**, never on a timer. So a tank left alone holds
    // its number for as long as you leave it alone, and no pause, no rate and no idle branch
    // may put a single drop back.
    //
    // Twenty seconds of standing still is deliberately long: `gas_regen_delay_s` was 0.5 s
    // and `gas_regen_per_s` 10/s, so a regeneration of any shape has 19.5 s to show itself
    // here — 195 gas' worth, two thirds of the tank.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    // Burn one tick first, exactly the way the game burns it: a timer-shaped refill is armed
    // off the last tick that wanted gas, so this is the state it was built to fire from.
    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 1);
    release(&mut app, BOOST_KEY);
    let half = d.game.vector.gas_tank / 2.0;
    set_tank(&mut app, e, half);

    // One tick before the counter is read: `set_tank` above writes `Gas` by hand, and Bevy's
    // change detection reports that write — mine, not the game's. Measuring from here on means
    // every change counted below came out of the simulation.
    app.update();
    let idle_ticks = (20.0 * hz(&d)) as u64;
    let before = gas_changes(&app);
    ticks(&mut app, idle_ticks);

    let now = gas(&app, e).current;
    assert!(
        (now - half).abs() < 1e-3,
        "{:.1} s of touching nothing moved the tank from {half} to {now} — gas refills only \
         at a station in the main building (docs/QUESTIONS.md Q-033), and nothing in the \
         simulation may put a drop back on its own",
        idle_ticks as f32 / hz(&d)
    );
    // And the same claim in the signal the HUD hangs on: a tank nobody is using is not a tank
    // that changes, so it must not be written at all.
    assert_eq!(
        gas_changes(&app) - before,
        0,
        "an idle tank reported {} changes in {idle_ticks} ticks — `Changed<Gas>` is a signal, \
         and something is still writing to the tank every tick",
        gas_changes(&app) - before
    );
}

#[test]
fn f018_nothing_refills_while_the_gas_is_being_spent() {
    // **The rule the refuel stations will have to keep, written before they exist.** A held
    // boost costs its full price: one second of it costs `gas_boost_per_s`, not
    // `gas_boost_per_s` minus something that ran alongside. Anything that puts gas back during
    // the burn turns the drain into a net one, and the tank then carries a boost far longer
    // than the file says it does — which is how the length of a flight stops being readable
    // out of `gas_tank / gas_boost_per_s`.
    //
    // **Strengthened on 2026-08-12 (Q-033):** the sum at the end is not enough on its own — a
    // refill that gave back exactly what it took would pass it. So the tank is read **every
    // tick** and may never be higher than it was the tick before.
    let mut app = app();
    let d = data(&app);
    let v = &d.game.vector;
    let e = me(&mut app);
    ticks(&mut app, 60);
    let half = v.gas_tank / 2.0;

    set_tank(&mut app, e, half);
    hold(&mut app, BOOST_KEY);
    let mut previous = half;
    for tick in 0..60 {
        app.update();
        let now = gas(&app, e).current;
        assert!(
            now <= previous + 1e-6,
            "tick {tick} of a held boost took the tank from {previous} UP to {now} — \
             nothing may put gas back while it is being spent"
        );
        previous = now;
    }
    release(&mut app, BOOST_KEY);
    let spent = half - gas(&app, e).current;
    assert!(
        (spent - v.gas_boost_per_s).abs() < 0.01,
        "a second of boost cost {spent:.4}; game.ron says gas_boost_per_s = {}",
        v.gas_boost_per_s
    );

    // The same for a rope that is actually being reeled in. The pressed button alone is not
    // enough — the cost follows the effect.
    set_tank(&mut app, e, half);
    hold(&mut app, REEL_KEY);
    let mut previous = half;
    for tick in 0..60 {
        anchor_left(&mut app, e);
        app.update();
        let now = gas(&app, e).current;
        assert!(
            now <= previous + 1e-6,
            "tick {tick} of reeling in took the tank from {previous} UP to {now}"
        );
        previous = now;
    }
    release(&mut app, REEL_KEY);
    let spent = half - gas(&app, e).current;
    assert!(
        (spent - v.gas_reel_per_s).abs() < 0.01,
        "a second of reeling in cost {spent:.4}; game.ron says gas_reel_per_s = {}",
        v.gas_reel_per_s
    );
}

#[test]
fn f018_the_tank_never_climbs_above_max_and_a_full_tank_reports_no_change() {
    // ★ **The one with teeth, and the second station rule.** `Changed<Gas>` is what the HUD
    // hangs on: anything that writes the tank every tick without moving it is invisible in
    // every assert on `Gas::current` and still wakes the bar sixty times a second for the rest
    // of the run. And `max` is a ceiling a station will have to respect too — a tank at 300 of
    // 300 that is topped up once more is how a refuel point invents fuel.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    // Burn, release, then hand the tank back one tick short of full: the state in which a
    // mechanism that tops off would show itself.
    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 1);
    release(&mut app, BOOST_KEY);
    let one_tick = d.game.vector.gas_boost_per_s / hz(&d);
    let short = d.game.vector.gas_tank - one_tick;
    set_tank(&mut app, e, short);

    // One tick first: `set_tank` is my own write and change detection reports it as one.
    app.update();
    let before = gas_changes(&app);
    ticks(&mut app, 300); // five seconds of standing there
    let tank = gas(&app, e);
    assert!(
        tank.current <= tank.max + 1e-6,
        "the tank climbed past the top: {} of {}",
        tank.current,
        tank.max
    );
    assert!(
        (tank.current - short).abs() < 1e-3,
        "five seconds of nothing topped the tank up from {short} to {} — gas comes back at a \
         station, not by standing still (docs/QUESTIONS.md Q-033)",
        tank.current
    );
    assert_eq!(
        gas_changes(&app) - before,
        0,
        "a tank that nobody is using reported {} changes in 300 ticks — `Changed<Gas>` is a \
         signal, and that is a lie in it",
        gas_changes(&app) - before
    );
}

#[test]
fn f018_the_sandbox_tank_is_not_written_while_it_idles() {
    // `--sandbox` is unlimited: nothing leaves it and nothing goes into it, so nothing about
    // it may be written — a mechanism that ticked some countdown on it would mark `Gas` as
    // changed while the tank stood still.
    let mut app = app_with(Cli { headless: true, sandbox: true, ..default() });
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60);

    hold(&mut app, BOOST_KEY);
    ticks(&mut app, 30);
    release(&mut app, BOOST_KEY);

    let before = gas_changes(&app);
    ticks(&mut app, 300);
    let tank = gas(&app, e);
    assert!(
        (tank.current - d.game.vector.gas_tank).abs() < 1e-6,
        "the sandbox tank holds {} of {}",
        tank.current,
        d.game.vector.gas_tank
    );
    assert_eq!(
        gas_changes(&app) - before,
        0,
        "the unlimited tank reported {} changes while nothing about it changed",
        gas_changes(&app) - before
    );
}

// ---------------------------------------------------------------------------------------
// 7. The arithmetic the user will feel
// ---------------------------------------------------------------------------------------

#[test]
fn f018_a_full_tank_carries_a_boost_that_lasts() {
    // ★ **The user's complaint, as a number.** He flew a 100 tank at 18/s: 5.6 s, and then a
    // dead Vector Gear for the rest of a 330 s mission. The length of one held boost is
    // `gas_tank / gas_boost_per_s` and comes out of the tank alone — there is no refill
    // anywhere (Q-033) — so this test measures the tank against the file and against the
    // sentence "der boost hält nicht lang genug". The tripled tank is the **whole** answer to
    // it, which is why this test matters more since the regeneration came back out.
    let mut app = app();
    let d = data(&app);
    let v = &d.game.vector;
    let e = me(&mut app);
    ticks(&mut app, 60);
    assert!((gas(&app, e).current - v.gas_tank).abs() < 1e-6, "the tank starts full");

    let expected_s = v.gas_tank / v.gas_boost_per_s;
    assert!(
        expected_s > 10.0,
        "game.ron carries {} gas at {}/s = {expected_s:.2} s of held boost. The user flew \
         5.6 s of it and said it does not last long enough; anything in that neighbourhood \
         ships the same complaint again",
        v.gas_tank,
        v.gas_boost_per_s
    );

    let cap = (expected_s * hz(&d) * 2.0) as u64;
    hold(&mut app, BOOST_KEY);
    let mut burned = 0u64;
    while burned < cap {
        app.update();
        burned += 1;
        if !grant(&app, e).boost {
            break;
        }
    }
    release(&mut app, BOOST_KEY);

    let measured_s = burned as f32 / hz(&d);
    assert!(
        (measured_s - expected_s).abs() < 0.1,
        "held boost ran dry after {measured_s:.2} s; {} gas at {}/s is {expected_s:.2} s",
        v.gas_tank,
        v.gas_boost_per_s
    );
}

#[test]
fn f018_a_tank_is_a_whole_mission_of_flying_because_nothing_refills_it() {
    // The other end of the same arithmetic, and it replaces the refill-rate test that stood
    // here until 2026-08-12. With no regeneration anywhere, `gas_tank` is not "how long one
    // boost lasts" — it is **the entire supply of a run** until the refuel stations exist
    // (`docs/NEXT.md` §1d). So the file has to carry enough of it that a mission is flyable at
    // all, and that is a claim about the numbers, not about the mechanism.
    let d = data(&app());
    let v = &d.game.vector;

    // A reel-in of a full rope costs a few per cent of the tank. Not balancing: a guard
    // against a tank so large that the other consumer stops existing, and against one so small
    // that a single reel-in ends the run.
    let reel_s = (d.scale.vector.anchor_range_m - v.min_rope_m) / v.reel_speed_m_s;
    let share = v.gas_reel_per_s * reel_s / v.gas_tank;
    assert!(
        (0.001..0.5).contains(&share),
        "reeling in a full {} m rope takes {reel_s:.2} s and costs {:.2} gas — {:.1} % of a \
         {} tank",
        d.scale.vector.anchor_range_m,
        v.gas_reel_per_s * reel_s,
        share * 100.0,
        v.gas_tank
    );

    // Boost is bought in bursts, not held for sixteen seconds. Half a second is a burst; the
    // tank has to hold enough of them that flying is a rhythm rather than a countdown.
    let bursts = v.gas_tank / (v.gas_boost_per_s * 0.5);
    assert!(
        bursts > 20.0,
        "a {} tank at {}/s is only {bursts:.1} half-second boosts, and nothing refills it \
         (docs/QUESTIONS.md Q-033) — that is a run that ends before the mission does",
        v.gas_tank,
        v.gas_boost_per_s
    );
}

// ---------------------------------------------------------------------------------------
// 8. The refill — `vector` is the ONE writer of `Gas`, and a station only asks (FIND-063)
// ---------------------------------------------------------------------------------------

#[test]
fn f018_a_refuel_request_is_the_only_thing_that_ever_raises_a_tank() {
    // The reader's half of the rule-4 repair of 2026-08-12. `mission::hub`'s station used to
    // call `Gas::refill` itself; now it sends `RefuelRequest` and **this** domain applies it.
    // What is measured here is the seam, not the station: a real app with no hub in it at all.
    let mut app = app();
    let me = me(&mut app);
    let d = data(&app);
    let max = d.game.vector.gas_tank;
    set_tank(&mut app, me, 0.0);

    // 1. Nobody asks: nothing comes back. Q-033 from this side — the applier must not be a
    //    regeneration with an extra step.
    ticks(&mut app, 30);
    assert_eq!(gas(&app, me).current, 0.0, "the tank refilled itself without a request");

    // 2. One request, one tick, exactly the amount asked for.
    let id = *app.world().get::<PlayerId>(me).expect("a player carries an id");
    app.world_mut().write_message(RefuelRequest { player: id, amount: 25.0 });
    ticks(&mut app, 1);
    assert!(
        (gas(&app, me).current - 25.0).abs() < 1e-3,
        "25 gas were asked for and the tank holds {}",
        gas(&app, me).current
    );

    // 3. A request for somebody else's tank is not mine. The message carries a `PlayerId` for
    //    this reason and for no other (`docs/multiplayer.md` rule 2).
    let mate = second_player(&mut app, Vec3::new(30.0, 2.0, 0.0));
    set_tank(&mut app, mate, 0.0);
    let mate_id = *app.world().get::<PlayerId>(mate).expect("a player carries an id");
    assert_ne!(mate_id, id, "two players, two ids");
    app.world_mut().write_message(RefuelRequest { player: mate_id, amount: 10.0 });
    ticks(&mut app, 1);
    assert!(
        (gas(&app, me).current - 25.0).abs() < 1e-3,
        "somebody else's refuel landed in my tank: {}",
        gas(&app, me).current
    );
    assert!(
        (gas(&app, mate).current - 10.0).abs() < 1e-3,
        "the request named him and his tank holds {}",
        gas(&app, mate).current
    );

    // 4. And never above the tank, however much is asked for.
    app.world_mut().write_message(RefuelRequest { player: id, amount: max * 10.0 });
    ticks(&mut app, 1);
    assert!(
        (gas(&app, me).current - max).abs() < 1e-3,
        "a station overfilled the tank: {} of {max}",
        gas(&app, me).current
    );

    // 5. A full tank that keeps being asked writes NOTHING — the `Changed<Gas>` signal the HUD
    //    reads must not tick sixty times a second for a number that does not move (§6 rule 6).
    let before = gas_changes(&app);
    for _ in 0..30 {
        app.world_mut().write_message(RefuelRequest { player: id, amount: 5.0 });
        ticks(&mut app, 1);
    }
    assert_eq!(
        gas_changes(&app) - before,
        0,
        "30 requests against a full tank woke `Changed<Gas>` {} times",
        gas_changes(&app) - before
    );
}

// ---------------------------------------------------------------------------------------
// 6. F-006 rope steering — the new thrust is NOT free (`docs/NEXT.md` §1B, FIND-082)
//
// All nine judges of the §1B plan named the same biggest flaw independently: the mixing rule
// added the strongest thrust in the game and charged nothing for it. `GasConsumer::Steer` is
// the answer, priced at the boost's own gas per m/s of speed bought — 16/30 against 18/34.
//
// These two are the whole-app half of the claim; the booking itself is unit-tested in
// `src/vector/gas.rs`, and what the thrust then *does* is `tests/player.rs` §3bc.
// ---------------------------------------------------------------------------------------

/// `W` — `Buttons::MOVE` comes out of the movement keys in `src/net/local.rs`.
const FORWARD_KEY: KeyCode = KeyCode::KeyW;

#[test]
fn f006_a_second_of_rope_steering_costs_what_the_file_says() {
    // The same shape as `f018_a_second_of_boost_costs_exactly_the_value_from_the_file`, and
    // against the same failure: multiplied by `dt` twice it costs 0.0044/s, which is invisible
    // while playing and wrong at balancing time.
    let mut app = app();
    let d = data(&app);
    let e = me(&mut app);
    ticks(&mut app, 60); // land first; nothing is pressed, so nothing is spent
    let before = gas(&app, e).current;

    hold(&mut app, FORWARD_KEY);
    // Re-anchored before every tick for the reason `anchor_left` gives: `update_hooks` is the
    // real writer and lets go of a hook nobody is holding the trigger for.
    for _ in 0..60 {
        anchor_left(&mut app, e);
        ticks(&mut app, 1);
    }
    release(&mut app, FORWARD_KEY);

    let spent = before - gas(&app, e).current;
    let expected = d.game.vector.gas_steer_per_s;
    assert!(
        (spent - expected).abs() < 0.01,
        "60 ticks of W on an anchored rope cost {spent:.4} gas; game.ron says \
         gas_steer_per_s = {expected}. A rope thrust that costs nothing is the flaw every one \
         of the nine judges of docs/NEXT.md §1B named"
    );
    assert!(grant(&app, e).steer, "and the grant that gates the thrust was actually issued");
}

#[test]
fn f006_a_rope_with_no_key_and_a_key_with_no_rope_both_cost_nothing() {
    // *The cost follows the effect, not the button* — the same rule reel-in and the dodge
    // already live by. Neither half of the want alone produces any thrust in
    // `player::locomotion::air_control`, so neither may bill.
    let mut app = app();
    let e = me(&mut app);
    ticks(&mut app, 60);

    // A rope and no key at all.
    let before = gas(&app, e).current;
    for _ in 0..60 {
        anchor_left(&mut app, e);
        ticks(&mut app, 1);
    }
    let after_idle = gas(&app, e).current;
    assert!(
        (before - after_idle).abs() < 1e-6,
        "an anchored rope with no movement key held cost {:.4} gas — hanging still is free",
        before - after_idle
    );
    assert!(!grant(&app, e).steer, "and no grant was issued for it");

    // A key and no rope. `S` is deliberately part of this: it is `w⁺ = 0`
    // („mit s »spannt« man nur das seil"), so it buys nothing and pays nothing even WITH a rope.
    for key in [FORWARD_KEY, KeyCode::KeyD, KeyCode::KeyS] {
        let before = gas(&app, e).current;
        hold(&mut app, key);
        ticks(&mut app, 60);
        release(&mut app, key);
        let spent = before - gas(&app, e).current;
        assert!(
            spent.abs() < 1e-6,
            "{key:?} held for a second without a hook cost {spent:.4} gas — the free-air \
             control is not gated on gas and never bills for it"
        );
    }

    // And `S` with a rope: still nothing, because the pull term is `max(0, move_y)`.
    let before = gas(&app, e).current;
    hold(&mut app, KeyCode::KeyS);
    for _ in 0..60 {
        anchor_left(&mut app, e);
        ticks(&mut app, 1);
    }
    release(&mut app, KeyCode::KeyS);
    let spent = before - gas(&app, e).current;
    assert!(
        spent.abs() < 1e-6,
        "S on a taut rope cost {spent:.4} gas — requirement 7 of docs/NEXT.md §1A is that S \
         never hauls you at the rope, and what does not haul does not bill"
    );
}
