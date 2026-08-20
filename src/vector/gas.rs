//! `F-018` The gas budget — **the only place that debits `Gas`.**
//!
//! Without this detour `F-007` (boost) and `F-005` (reel-in) would both call
//! `Gas::try_spend`. That method is deliberately atomic and without partial spending
//! (`shared::state`), so on a tight tank the **system order** would decide who pays — the
//! coin toss at 60 Hz that `docs/architecture.md` forbids, and on the network a desync
//! nobody reproduces.
//!
//! Here it is booked **once per tick** and the result published as [`GasGrant`]. Whoever
//! reads `false` there writes zero into his drive.
//!
//! The **priority** on a tight tank lives in `assets/data/game.ron`
//! (`vector.gas_priority`), not as an `if` here: "what runs out first?" is a balancing
//! decision (`docs/QUESTIONS.md` Q-017).
//!
//! ## The contract the drive systems may rely on
//!
//! `GasGrant.boost == true` means **both** at once: the button is held *and* this tick's gas
//! has been paid. `vector::boost` therefore needs no second condition of its own — one
//! `if grant.boost` is the whole check. The same for `GasGrant.reel_in`. Were the want
//! condition to live in two places, one of them would be wrong by next week, and it would
//! be wrong as a **free** boost.
//!
//! `GasGrant.steer` is the same contract for the rope half of the mixing rule
//! (`docs/NEXT.md` §1B, `player::locomotion::rope_steer`) — and it is the one all nine judges of
//! that plan asked for independently, because without it the strongest thrust in the game would
//! have been the only free one.
//!
//! Written **every tick, for every player**, and by assignment — there is no clearing
//! system and no grant that lives one tick too long.
//!
//! ## Three decisions that are not obvious from the code
//!
//! - **Reel-in only wants gas when a hook holds.** Pressing the button in free fall pulls
//!   on nothing, so it costs nothing. The cost follows the effect, not the button.
//! - **The grant is per player, not per side.** [`GasGrant`] carries one `reel_in`, and
//!   `game.ron` carries one `gas_reel_per_s` — so two taut ropes reeled in at once cost
//!   exactly as much as one. That is a game-value question, not a mechanism, and it is
//!   listed in the report of this job rather than answered here.
//! - **The `Hook` read here is one tick old.** `vector/mod.rs` chains `gas_budget` **before**
//!   `hook::update_hooks` on purpose. A hook that anchors in this tick therefore starts
//!   costing gas in the next one — one tick of delay in exchange for an order that does not
//!   depend on which system Bevy happens to run first.
//!
//! ## The budget only ever subtracts; gas comes back only when a station asks (Q-033)
//!
//! **[`gas_budget`] has no refill in it, and its absence is a decision.** The user answered it
//! on 2026-08-12: *„gas refillt nur im main gebäude an bestimmten stationen/objekten"* — gas
//! comes back **at a place you go to**, never on a timer.
//!
//! That place is `mission::hub`'s refuel station, and since the same day it **asks**:
//! [`apply_refuel_requests`] takes a `RefuelRequest` and is the only thing in the game that
//! ever raises a tank. It sits in this file because `Gas` has **one** writer and it is this
//! file — a station that called `Gas::refill` itself was the rule-4 violation this repair took
//! back out (`docs/FINDINGS.md` FIND-063). Nothing about the answer to Q-033 changes with it:
//! no rate, no timer, no idle branch — a tank rises only while somebody stands in a station.
//!
//! Between 2026-08-10 and then this file did regenerate the tank while nobody was spending,
//! after a `gas_regen_delay_s` pause. That was an assumption made under the autonomous rule
//! while the question was open, and the user picked none of the three shapes it offered. The
//! whole mechanism came back out: `refill_tank`, `arm_pause`, `Gas::regen_delay_left_s` and
//! both RON keys. **Do not put a rate back in when the tank feels tight** — the answer is the
//! stations (queued in `docs/NEXT.md` §1d), and the reason is in the bible: burning gas is
//! loud and a Bellower answers it, so the resource is coupled to *risk*, which a tank that
//! quietly fills itself while you hang around is not.
//!
//! How long one held boost lasts is therefore `gas_tank / gas_boost_per_s` and nothing else —
//! **833.3 s at the numbers of 2026-08-20** (it was 16.67 s while `gas_tank` was 300). The tank
//! is what got bigger for that, three times now: 100 -> 300 -> 15000, each time because the user
//! said so after playing. The rule is the division; the seconds are whatever the file currently
//! says. See `assets/data/game.ron: vector.gas_tank` and `docs/QUESTIONS.md` Q-046.
//!
//! `Changed<Gas>` stays a signal because of it. A tick in which nobody wants gas writes
//! nothing at all to the tank, so the HUD is not woken sixty times a second by a number that
//! did not move (`tests/vector_gas.rs::f018_an_idle_tank_never_refills_on_its_own`).
//!
//! Cost, evidence and image of this file: `tests/vector_gas.rs`, `scripts/f-018-gas.txt`,
//! `docs/images/f-018-gas.png`.

use bevy::prelude::*;

use crate::data::{GameData, GasConsumer};
use crate::shared::{Buttons, Gas, GasGrant, Hook, Intent, PlayerId, RefuelRequest};

/// Puts gas back — **the only thing in the game that ever raises a tank** (Q-033).
///
/// The gas comes back at a place you walk to, and the place is `mission::hub`'s refuel station.
/// But `Gas` has one writer and it is this file (`docs/architecture.md`, authority table), so
/// the station **asks** with a [`RefuelRequest`] and this applies it. Until 2026-08-12 the
/// station called `Gas::refill` itself; that was a second writer on one field, disjoint from
/// this one only *by phase*, and "disjoint by phase" is the argument that stops being true over
/// a wire (`docs/FINDINGS.md` FIND-063).
///
/// Three properties this has to keep, and each one is a test:
///
/// - **A request names a player and fills that player's tank.** Not `LocalPlayer`, not
///   `.single()` — `docs/multiplayer.md` rule 2. Two players in one station are two requests.
/// - **It never raises a tank above `Gas::max`**: `Gas::refill` caps, and a station that keeps
///   asking after the tank is full changes nothing.
/// - **`set_if_neq`, exactly like [`gas_budget`]**: a full tank in a station must not report
///   `Changed<Gas>` sixty times a second for a number that did not move (§6 rule 6).
///
/// **One tick late, deliberately.** The station sees the player in `PostStep` and this runs in
/// the next tick's `Intent`. Ordering it into the same tick would mean a `vector` system
/// ordered against a `mission` system — a hidden edge past the allow list — and the whole point
/// of the repair was not to buy an edge. It is the same trade the `Hook` read above makes, and
/// at `gear.ron: resupply.gas_per_s` of 40 one tick is 0.67 gas.
pub fn apply_refuel_requests(
    mut requests: MessageReader<RefuelRequest>,
    mut players: Query<(&PlayerId, &mut Gas)>,
) {
    for request in requests.read() {
        for (id, mut gas) in &mut players {
            if *id != request.player {
                continue;
            }
            let mut tank = *gas;
            tank.refill(request.amount);
            gas.set_if_neq(tank);
        }
    }
}

/// **Does the rope steer this tick move the player at all?** The `Steer` half of the budget's
/// want condition, as a pure function so that a test can hold it against the thrust it is
/// supposed to be paying for.
///
/// The arguments are `player::locomotion::rope_steer`'s own, minus the two magnitudes and the
/// yaw — because the question is whether that function returns `Vec3::ZERO`, and neither
/// `pull_m_s2` nor the yaw can decide that.
///
/// `to_anchors_m` is one `tip − hand` per **anchored** arm, unnormalised, exactly as
/// `rope_steer` takes it.
pub fn steer_has_effect(
    to_anchors_m: &[Vec3],
    look_dir: Vec3,
    move_x: f32,
    move_y: f32,
    min_rope_m: f32,
    fade_m: f32,
) -> bool {
    if to_anchors_m.is_empty() {
        return false;
    }
    // `A`/`D` are the player's own thrust across the rope and nothing scales them down, so a
    // lateral key on an anchored rope always moves him and always pays. This is the half of the
    // steer that was never in doubt.
    if move_x != 0.0 {
        return true;
    }
    // `S` is not a haul (`docs/NEXT.md` §1A requirement 7) — the same `.max(0.0)` as the thrust.
    if move_y.max(0.0) <= 0.0 {
        return false;
    }
    // And the half that was: **the pull is `max(0, l̂ · r̂)` times `clamp((L − min)/fade)`, and
    // both of those are zero over most of a swing.** A player hangs *under* his anchor and looks
    // where he is going, so `l̂ · r̂` is negative and the pull is exactly `Vec3::ZERO` — measured
    // over `scripts/f018-budget.txt`: mean delivered pull 0.0012 of `air_pull_m_s2` across 99
    // sampled steer ticks, while 16/s was charged for every one of them. 48.3 % of the tank.
    to_anchors_m.iter().any(|to_anchor| {
        let Some(direction) = to_anchor.try_normalize() else {
            return false;
        };
        look_dir.dot(direction) > 0.0 && to_anchor.length() > min_rope_m && fade_m > 0.0
    })
}

/// **Diagnostics only, and off unless `DBT_GAS_LEDGER=1` stands in the environment.**
///
/// The tank is one number, so a player who says *„gas ist VIEL zu schnell weg"* cannot be
/// answered from it: 300 gone tells you nothing about WHICH verb spent it. This splits the
/// same debit four ways as it happens — the amount, and how many ticks each consumer *wanted*
/// versus how many it was *granted* — so a sortie can be read as a ledger instead of a slope.
///
/// It is not a game value and it is not state anybody may read: nothing in the simulation
/// looks at it, it lives in a `Local` of [`gas_budget`], and with the variable unset the whole
/// thing is four adds and no output. Kept because the next tuning round needs the same number
/// (`docs/FINDINGS.md` FIND-139).
#[derive(Default)]
pub struct Ledger {
    tick: u64,
    /// Gas actually debited, per consumer, in the order `[boost, steer, reel_in, dodge]`.
    spent: [f32; 4],
    /// Ticks in which the consumer *wanted* gas.
    wanted: [u32; 4],
    /// Ticks in which it was *granted* — below `wanted` only on a tank that ran short.
    granted: [u32; 4],
    /// What really left the tank, summed straight off `Gas::current`. The four `spent` entries
    /// have to add up to this; if they do not, something bills gas that this file does not see.
    debited: f32,
}

/// Is the ledger switched on? Read once, not sixty times a second.
fn ledger_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("DBT_GAS_LEDGER").is_ok_and(|v| v != "0"))
}

/// Debits this tick's gas and writes [`GasGrant`].
///
/// The **only** writer of `Gas` in the simulation (`docs/architecture.md`, authority table).
pub fn gas_budget(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    mut ledger: Local<Ledger>,
    mut players: Query<(&Intent, &Hook, &mut Gas, &mut GasGrant, &Transform)>,
) {
    let vector = &data.game.vector;
    // `Time<Fixed>` and not `1.0 / simulation_hz`: the timestep is set from the very same
    // number (`src/lib.rs`), and a value derived twice is a value that drifts once.
    let dt = time.delta_secs();
    // Per **tick**, not per second. `gas_boost_per_s` is 18/s; at 60 Hz that is 0.3 per tick,
    // and 60 ticks of it are exactly the 18 from the file. Multiplying by `dt` a second time
    // somewhere further along would cost 0.005/s — nothing anybody notices while playing, and
    // `tests/vector_gas.rs` goes red on it.
    let boost_cost = vector.gas_boost_per_s * dt;
    let reel_cost = vector.gas_reel_per_s * dt;
    // A rate like the two above, and billed per tick like them: 16/s is 0.2667 per tick at
    // 60 Hz. It buys `player.air_pull_m_s2` / `air_lateral_m_s2` of thrust in
    // `player::locomotion::air_control`, and the price is the boost's own price per m/s of
    // speed bought — 16/30 against 18/34 (`assets/data/game.ron`, `tests/data.rs` holds it).
    let steer_cost = vector.gas_steer_per_s * dt;
    // **Not multiplied by `dt`, and that is the point of `F-008`.** The other two are rates and
    // are billed per tick; a dodge is one impulse and is billed once, on the tick the double-tap
    // lands. `gas_dodge` therefore has no `_per_s` in its name and must not grow one.
    let dodge_cost = vector.gas_dodge;

    for (intent, hook, mut gas, mut grant, transform) in &mut players {
        let wants_boost = intent.pressed(Buttons::BOOST);
        let wants_reel_in = intent.pressed(Buttons::REEL_IN) && hook.anchored_count() > 0;
        // The same shape as the line above it, and for the same reason: **the cost follows the
        // effect, not the button.** A double-tap with no movement key held has no direction to
        // throw the player in (`boost::dodge_direction` answers `None`), so `vector::boost`
        // would write zero — and billing 15 % of a tank for zero thrust is the invisible leak
        // that whole detour exists to prevent. One rule, one function, two callers.
        let wants_dodge =
            intent.pressed(Buttons::DODGE) && super::boost::dodge_direction(intent).is_some();
        // **`docs/NEXT.md` §1B, and the same rule a third time: the cost follows the effect.**
        // The rope term of the mixing rule is `n > 0 && (w⁺ > 0 || mx ≠ 0)` — no anchored hook
        // means there is no rope direction to push along and no lateral boost to add, and no
        // movement key means both halves of it are multiplied by zero. `S` alone is `w⁺ = 0`
        // (*„mit s »spannt« man nur das seil"*) and buys nothing, so it pays nothing.
        // **The one that was a bill and not a price** (`docs/FINDINGS.md` FIND-139). Until
        // 2026-08-20 this line read `anchored_count() > 0 && (move_y.max(0.0) > 0.0 ||
        // move_x != 0.0)` — the BUTTON — and `player::locomotion::rope_steer` then delivered
        // `Vec3::ZERO` for most of every swing, because its pull carries a `max(0, l̂ · r̂)` and
        // a player hangs *under* his anchor while looking where he is going. Measured over
        // `scripts/f018-budget.txt`: 144.8 of 300 gas — **48.3 % of the tank, the largest line
        // item in the game** — bought a mean thrust of 0.0012 of `air_pull_m_s2`.
        //
        // [`steer_has_effect`] is that condition read off the geometry instead, and
        // `tests/vector_gas.rs::f006_the_steer_is_billed_exactly_when_the_rope_really_thrusts`
        // holds the two against each other over 750 geometries, so the copy here cannot drift
        // away from the formula it is paying for.
        let hand_m = transform.translation + Vec3::Y * data.game.player.eye_height_m;
        let mut to_anchors_m = [Vec3::ZERO; 2];
        let mut anchored = 0;
        for arm in &hook.arms {
            if arm.state.is_anchored() {
                to_anchors_m[anchored] = arm.tip_m - hand_m;
                anchored += 1;
            }
        }
        let wants_steer = steer_has_effect(
            &to_anchors_m[..anchored],
            intent.look_dir(),
            intent.move_x,
            intent.move_y,
            vector.min_rope_m,
            data.game.player.air_pull_fade_m,
        );

        if !wants_boost && !wants_reel_in && !wants_dodge && !wants_steer {
            // Nobody wants anything, so **the tank is not touched at all** — not even to
            // write the same number back. `Changed<Gas>` is a signal the HUD and one day the
            // wire read, and a tank that reports a change every tick without changing is a
            // lie in that signal. Refilling belongs to the stations, not here (Q-033).
            grant.set_if_neq(GasGrant::default());
            continue;
        }

        // On a copy, so that `Gas` is only marked changed when something about the tank really
        // is different — and so that [`book`] stays a plain function a test can drive without
        // an app.
        let mut tank = *gas;
        let booked = book(
            &vector.gas_priority,
            Wants {
                boost: wants_boost,
                reel_in: wants_reel_in,
                steer: wants_steer,
                dodge: wants_dodge,
            },
            Costs {
                boost: boost_cost,
                reel_in: reel_cost,
                steer: steer_cost,
                dodge: dodge_cost,
            },
            &mut tank,
        );
        if ledger_enabled() {
            let wants = [wants_boost, wants_steer, wants_reel_in, wants_dodge];
            let grants = [booked.boost, booked.steer, booked.reel_in, booked.dodge];
            let costs = [boost_cost, steer_cost, reel_cost, dodge_cost];
            for i in 0..4 {
                ledger.wanted[i] += u32::from(wants[i]);
                ledger.granted[i] += u32::from(grants[i]);
                if grants[i] {
                    ledger.spent[i] += costs[i];
                }
            }
            // The control on the four adds above, and the reason the ledger is worth reading:
            // what it claims left the tank is compared against what really left it. A consumer
            // billed twice, or billed somewhere outside this file, shows up here and nowhere
            // else.
            ledger.debited += gas.current - tank.current;
        }
        gas.set_if_neq(tank);
        grant.set_if_neq(booked);
    }

    if ledger_enabled() {
        ledger.tick += 1;
        if ledger.tick % 60 == 0 {
            let total: f32 = ledger.spent.iter().sum();
            let tank = data.game.vector.gas_tank;
            info!(
                "gas ledger t={t} spent={total:.1} of {debited:.1} debited ({pct:.2}% of tank) | boost={b:.1} steer={s:.1} reel={r:.1} dodge={d:.1} | wanted_ticks boost={wb} steer={ws} reel={wr} dodge={wd} | granted_ticks boost={gb} steer={gs} reel={gr} dodge={gd}",
                t = ledger.tick,
                debited = ledger.debited,
                // ⚠️ `.2`, not `.0`. At `gas_tank: 15000` a whole ordinary sortie spends
                // ~223 gas = 1.49 % of the tank, so `{:.0}` printed the total as "1%" and
                // rounded all four line items to 0 % or 1 % — erasing exactly the split this
                // ledger exists to show (it is the instrument FIND-139 was found with, and
                // the one `scripts/f018-budget.txt` is written around). The absolute gas
                // figures beside it are the primary reading; the percentage is the scale.
                pct = 100.0 * total / tank,
                b = ledger.spent[0],
                s = ledger.spent[1],
                r = ledger.spent[2],
                d = ledger.spent[3],
                wb = ledger.wanted[0],
                ws = ledger.wanted[1],
                wr = ledger.wanted[2],
                wd = ledger.wanted[3],
                gb = ledger.granted[0],
                gs = ledger.granted[1],
                gr = ledger.granted[2],
                gd = ledger.granted[3],
            );
        }
    }
}

/// Books one tick for **one** tank, in the order the file names.
///
/// Pure on purpose: the order is a game value, and a game value has to be testable in both
/// directions without editing `assets/data/game.ron` (which belongs to the main head).
///
/// Every consumer is served **at most once** per tick, whatever the list says. A duplicate
/// entry in `gas_priority` is a data error — `tests/vector_gas.rs` names it — but it must not
/// turn into a double debit in the meantime.
pub fn book(priority: &[GasConsumer], wants: Wants, costs: Costs, gas: &mut Gas) -> GasGrant {
    let mut grant = GasGrant::default();
    // **Exhaustive `match`, no `_` arm.** The day `GasConsumer` gets a third variant
    // (`F-008` dash is already written into `docs/features.ron`), this file has to stop
    // compiling. A catch-all would instead silently hand the new consumer nothing.
    for consumer in priority {
        match consumer {
            GasConsumer::Boost => {
                if wants.boost && !grant.boost {
                    grant.boost = gas.try_spend(costs.boost);
                }
            }
            GasConsumer::ReelIn => {
                if wants.reel_in && !grant.reel_in {
                    grant.reel_in = gas.try_spend(costs.reel_in);
                }
            }
            GasConsumer::Steer => {
                if wants.steer && !grant.steer {
                    grant.steer = gas.try_spend(costs.steer);
                }
            }
            GasConsumer::Dodge => {
                if wants.dodge && !grant.dodge {
                    grant.dodge = gas.try_spend(costs.dodge);
                }
            }
        }
    }
    grant
}

/// Who wants gas this tick. **A struct and not three `bool` arguments** — [`book`] would
/// otherwise take three `bool`s and three `f32`s in a row, and the day somebody swaps two of
/// them the compiler says nothing and a dodge is billed a reel-in's price.
///
/// Every field is the *want*, already filtered by whether it can have an effect:
/// `reel_in` is false without an anchored hook, `dodge` is false without a movement direction
/// (`vector::boost::dodge_direction`). **The cost follows the effect, not the button.**
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Wants {
    pub boost: bool,
    pub reel_in: bool,
    /// `F-006` rope steering: `anchored_count() > 0` **and** a movement key that is not only
    /// `S`. Both halves are the effect, not the button.
    pub steer: bool,
    pub dodge: bool,
}

/// What one tick of each consumer costs, in gas.
///
/// ⚠️ **`boost`, `reel_in` and `steer` are per-tick amounts** — the rate out of the file already
/// multiplied by `dt` — **and `dodge` is not.** A dodge is one impulse and is billed whole, on
/// the one tick its grant is true. Multiplying it by `dt` as well would make it 60 times
/// cheaper and nobody would see why.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Costs {
    pub boost: f32,
    pub reel_in: f32,
    pub steer: f32,
    pub dodge: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 60 Hz, the numbers from `game.ron` as of 2026-08-09: 18/s and 6/s.
    const BOOST: f32 = 18.0 / 60.0; // 0.3
    const REEL: f32 = 6.0 / 60.0; //  0.1
    /// `F-006` rope steering, 16/s from `game.ron` as of 2026-08-13.
    const STEER: f32 = 16.0 / 60.0; // 0.26667
    /// `F-008`, and **flat** — not divided by 60. The dodge is billed once, not per second.
    const DODGE: f32 = 45.0;

    /// The two oldest continuous consumers as every test below spells them; `F-006` and `F-008`
    /// are off unless a test says otherwise. A helper and not four literals per call, so that
    /// adding a fifth consumer one day touches one line rather than nine — which is exactly what
    /// `Steer` did on 2026-08-13.
    fn wants(boost: bool, reel_in: bool) -> Wants {
        Wants { boost, reel_in, steer: false, dodge: false }
    }

    fn costs() -> Costs {
        Costs { boost: BOOST, reel_in: REEL, steer: STEER, dodge: DODGE }
    }

    #[test]
    fn f018_the_file_decides_who_gets_the_last_drop() {
        // A tank that covers each of them alone but not both: 0.35 out of 0.3 + 0.1. This is
        // the case in which, without the booking, the system order would decide — and the
        // system order is not a design.
        let mut first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, true), costs(), &mut first);
        assert!(g.boost, "the file names Boost first, so Boost gets the drop");
        assert!(!g.reel_in, "and there is nothing left over for the second one");
        assert!((first.current - 0.05).abs() < 1e-6, "0.35 - 0.3 = 0.05, got {}", first.current);

        // The same tank, the other order — and the other one wins. If this holds, the order
        // really is a value from the file and not an `if` in the code.
        let mut second = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::ReelIn, GasConsumer::Boost], wants(true, true), costs(), &mut second);
        assert!(g.reel_in, "ReelIn stands first here");
        assert!(!g.boost, "0.35 - 0.1 = 0.25 is not enough for a boost costing 0.3");
        assert!((second.current - 0.25).abs() < 1e-6, "got {}", second.current);
    }

    #[test]
    fn f018_exactly_one_of_the_two_is_served_when_the_tank_is_short() {
        // The claim as a claim, independent of which one it is: on a tight tank one and only
        // one gets fuel. Half of each would be the answer nobody can explain.
        for order in [
            [GasConsumer::Boost, GasConsumer::ReelIn],
            [GasConsumer::ReelIn, GasConsumer::Boost],
        ] {
            let mut gas = Gas { current: 0.35, ..Gas::full(100.0) };
            let g = book(&order, wants(true, true), costs(), &mut gas);
            assert_eq!(
                u8::from(g.boost) + u8::from(g.reel_in),
                1,
                "order {order:?} served {g:?}"
            );
        }
    }

    #[test]
    fn f018_an_empty_tank_pays_for_no_half_boost() {
        // `F-018` in its own words: "at 0 no more flying, only ground movement".
        let mut gas = Gas { current: 0.1, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, false), costs(), &mut gas);
        assert!(!g.boost, "0.1 does not cover a boost costing 0.3");
        assert!(
            (gas.current - 0.1).abs() < 1e-6,
            "a refused boost costs nothing — the tank holds {} instead of 0.1",
            gas.current
        );
    }

    #[test]
    fn f018_whoever_does_not_want_gas_does_not_pay() {
        let mut gas = Gas::full(100.0);
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(false, false), costs(), &mut gas);
        assert_eq!(g, GasGrant::default());
        assert!((gas.current - 100.0).abs() < 1e-6, "tank at {}", gas.current);
    }

    #[test]
    fn f018_the_sandbox_tank_grants_everything_and_stays_full() {
        // `--sandbox`: infinite gas, for looking around (§12a).
        let mut gas = Gas { unlimited: true, ..Gas::full(1.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, true), costs(), &mut gas);
        assert!(g.boost && g.reel_in, "in the sandbox both get fuel: {g:?}");
        assert!((gas.current - 1.0).abs() < 1e-6, "and nothing leaves the tank");
        assert!(!gas.is_empty());
    }

    #[test]
    fn f018_booking_never_puts_a_drop_back_in() {
        // The station rule (Q-033) from the booking's side: `book` is the only thing this file
        // does to a tank, and it is monotone downwards. Nine hundred ticks of every
        // combination of the two buttons, and the number may not rise once — a refill smuggled
        // in here would look exactly like the regeneration that was taken out on 2026-08-12.
        for (wants_boost, wants_reel) in [(true, true), (true, false), (false, true), (false, false)]
        {
            let mut gas = Gas { current: 5.0, ..Gas::full(300.0) };
            let mut previous = gas.current;
            for tick in 0..900 {
                book(
                    &[GasConsumer::Boost, GasConsumer::ReelIn],
                    wants(wants_boost, wants_reel),
                    costs(),
                    &mut gas,
                );
                assert!(
                    gas.current <= previous + 1e-9,
                    "tick {tick} of ({wants_boost}, {wants_reel}) took the tank from {previous} \
                     up to {} — gas comes back only at a station (docs/QUESTIONS.md Q-033)",
                    gas.current
                );
                assert!(gas.current <= gas.max, "and never above max: {} of {}", gas.current, gas.max);
                previous = gas.current;
            }
        }
    }

    /// The fourth arm exists and it debits. Without this the `Steer` entry in `gas_priority`
    /// would be a consumer that is named and never served — which is exactly the state
    /// `FIND-082` describes and this file was left un-compiling to prevent.
    #[test]
    fn f006_the_rope_steer_is_billed_and_is_not_free() {
        let mut gas = Gas::full(100.0);
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Steer, GasConsumer::ReelIn],
            Wants { steer: true, ..wants(false, false) },
            costs(),
            &mut gas,
        );
        assert!(g.steer, "an anchored rope with W held got no grant: {g:?}");
        assert!(
            (gas.current - (100.0 - STEER)).abs() < 1e-6,
            "one tick of rope steering took {} instead of {STEER} — all nine judges of \
             docs/NEXT.md §1B named a free rope thrust as its biggest flaw",
            100.0 - gas.current
        );
        assert!(!g.boost && !g.reel_in, "and it paid for nobody else: {g:?}");
    }

    /// `Steer` stands **second** in `game.ron`, so the deliberate press wins the last drop over
    /// the one that is held all flight long (`docs/QUESTIONS.md` Q-037). Both directions, so the
    /// claim is about the file and not about an `if`.
    #[test]
    fn f006_the_file_decides_whether_the_last_drop_boosts_or_steers() {
        let mut boost_first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Steer],
            Wants { steer: true, ..wants(true, false) },
            costs(),
            &mut boost_first,
        );
        assert!(g.boost && !g.steer, "Boost stands first: {g:?}");

        let mut steer_first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(
            &[GasConsumer::Steer, GasConsumer::Boost],
            Wants { steer: true, ..wants(true, false) },
            costs(),
            &mut steer_first,
        );
        assert!(g.steer, "Steer stands first here: {g:?}");
        assert!(!g.boost, "0.35 - 0.2667 = 0.0833 does not cover a boost costing 0.3");
    }

    #[test]
    fn f018_a_doubled_entry_in_the_file_does_not_debit_twice() {
        // A broken `gas_priority` is a data error and has its own test in
        // `tests/vector_gas.rs`. Until somebody sees it, it must not quietly cost double.
        let mut gas = Gas::full(100.0);
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Boost, GasConsumer::ReelIn],
            wants(true, false),
            costs(),
            &mut gas,
        );
        assert!(g.boost);
        assert!(
            (gas.current - (100.0 - BOOST)).abs() < 1e-6,
            "one boost costs one boost — the tank holds {}",
            gas.current
        );
    }
}
