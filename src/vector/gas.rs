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
//! ## The refill (`docs/QUESTIONS.md` Q-033)
//!
//! Until 2026-08-10 this file only ever subtracted, and nothing anywhere added: the tank was
//! full at spawn and then fell to zero for the rest of the mission. The user played it and
//! said *„der boost hält nicht lang genug"*, which was true and too kind.
//!
//! It now refills — **while neither boosting nor reeling, and only after a pause**
//! (`vector.gas_regen_per_s`, `vector.gas_regen_delay_s`). Two consequences worth stating
//! before somebody reads a net drain into them:
//!
//! - **A held boost still costs the full `gas_boost_per_s`.** The refill and the debit never
//!   run in the same tick, so there is no net rate. How long one boost lasts is
//!   `gas_tank / gas_boost_per_s` and nothing else — the tank got bigger for that, the refill
//!   did not.
//! - **The pause follows the *want*, not the spend.** A tick in which the button asks for gas
//!   arms it, even when the tank was too empty to pay: otherwise a player who ran dry would
//!   be refilling while still holding the button down.
//!
//! `Changed<Gas>` stays a signal through all of it. A full tank that nobody is using is not
//! written, so the HUD is not woken (`tests/vector_gas.rs`, "stops reporting a change"). What
//! *does* report a change is the pause counting down — 30 ticks after the last burn — and it
//! reports one because the tank really is changing: the countdown is part of it.
//!
//! Cost, evidence and image of this file: `tests/vector_gas.rs`, `scripts/f-018-gas.txt`,
//! `docs/images/f-018-gas.png`.

use bevy::prelude::*;

use crate::data::{GameData, GasConsumer};
use crate::shared::{Buttons, Gas, GasGrant, Hook, Intent};

/// Debits this tick's gas and writes [`GasGrant`].
///
/// The **only** writer of `Gas` in the simulation (`docs/architecture.md`, authority table).
pub fn gas_budget(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    mut players: Query<(&Intent, &Hook, &mut Gas, &mut GasGrant)>,
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
    let refill = vector.gas_regen_per_s * dt;

    for (intent, hook, mut gas, mut grant) in &mut players {
        let wants_boost = intent.pressed(Buttons::BOOST);
        let wants_reel_in = intent.pressed(Buttons::REEL_IN) && hook.anchored_count() > 0;

        // On a copy, so that `Gas` is only marked changed when something about the tank
        // really is different — and so that [`book`] and [`refill_tank`] stay plain functions
        // a test can drive without an app.
        let mut tank = *gas;

        if !wants_boost && !wants_reel_in {
            // Nobody wants anything: nothing is debited, and the tank fills back up if its
            // pause has run out. On a full tank with the pause at zero this writes the
            // identical struct and `set_if_neq` keeps quiet — `Changed<Gas>` is a signal the
            // HUD and one day the wire read, and a tank that reports a change every tick
            // without changing is a lie in that signal.
            grant.set_if_neq(GasGrant::default());
            refill_tank(&mut tank, refill, dt);
            gas.set_if_neq(tank);
            continue;
        }

        let booked = book(
            &vector.gas_priority,
            wants_boost,
            wants_reel_in,
            boost_cost,
            reel_cost,
            &mut tank,
        );
        // The pause starts over on every tick that **wants** gas, whether or not there was
        // any to give. Arming it on the spend instead would refill a player who is holding
        // the button down on an empty tank.
        arm_pause(&mut tank, vector.gas_regen_delay_s);
        gas.set_if_neq(tank);
        grant.set_if_neq(booked);
    }
}

/// One tick of refill for **one** tank: counts the pause down, or puts `amount` back.
///
/// **Never both in the same tick.** The pause is over when it reaches zero, and the tick that
/// brings it there is still a tick of waiting — one tick either way is 17 ms, and a branch
/// that does two things at once is the one somebody later reads as a net rate.
///
/// An unlimited tank (`--sandbox`) is left alone entirely: there is nothing to put back, and
/// counting a pause down on it would mark `Gas` as changed for half a second after every
/// boost while nothing about it changed.
pub fn refill_tank(gas: &mut Gas, amount: f32, dt: f32) {
    if gas.unlimited {
        return;
    }
    if gas.regen_delay_left_s > 0.0 {
        gas.regen_delay_left_s = (gas.regen_delay_left_s - dt).max(0.0);
        return;
    }
    // `Gas::refill` clamps at `max` and ignores a nonsensical amount, so a full tank takes
    // nothing and stays byte-identical.
    gas.refill(amount);
}

/// Holds the refill off for `delay_s` seconds from now. See [`refill_tank`].
pub fn arm_pause(gas: &mut Gas, delay_s: f32) {
    if gas.unlimited || !delay_s.is_finite() || delay_s < 0.0 {
        return;
    }
    gas.regen_delay_left_s = delay_s;
}

/// Books one tick for **one** tank, in the order the file names.
///
/// Pure on purpose: the order is a game value, and a game value has to be testable in both
/// directions without editing `assets/data/game.ron` (which belongs to the main head).
///
/// Every consumer is served **at most once** per tick, whatever the list says. A duplicate
/// entry in `gas_priority` is a data error — `tests/vector_gas.rs` names it — but it must not
/// turn into a double debit in the meantime.
pub fn book(
    priority: &[GasConsumer],
    wants_boost: bool,
    wants_reel_in: bool,
    boost_cost: f32,
    reel_cost: f32,
    gas: &mut Gas,
) -> GasGrant {
    let mut grant = GasGrant::default();
    // **Exhaustive `match`, no `_` arm.** The day `GasConsumer` gets a third variant
    // (`F-008` dash is already written into `docs/features.ron`), this file has to stop
    // compiling. A catch-all would instead silently hand the new consumer nothing.
    for consumer in priority {
        match consumer {
            GasConsumer::Boost => {
                if wants_boost && !grant.boost {
                    grant.boost = gas.try_spend(boost_cost);
                }
            }
            GasConsumer::ReelIn => {
                if wants_reel_in && !grant.reel_in {
                    grant.reel_in = gas.try_spend(reel_cost);
                }
            }
        }
    }
    grant
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 60 Hz, the numbers from `game.ron` as of 2026-08-09: 18/s and 6/s.
    const BOOST: f32 = 18.0 / 60.0; // 0.3
    const REEL: f32 = 6.0 / 60.0; //  0.1

    #[test]
    fn f018_the_file_decides_who_gets_the_last_drop() {
        // A tank that covers each of them alone but not both: 0.35 out of 0.3 + 0.1. This is
        // the case in which, without the booking, the system order would decide — and the
        // system order is not a design.
        let mut first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], true, true, BOOST, REEL, &mut first);
        assert!(g.boost, "the file names Boost first, so Boost gets the drop");
        assert!(!g.reel_in, "and there is nothing left over for the second one");
        assert!((first.current - 0.05).abs() < 1e-6, "0.35 - 0.3 = 0.05, got {}", first.current);

        // The same tank, the other order — and the other one wins. If this holds, the order
        // really is a value from the file and not an `if` in the code.
        let mut second = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::ReelIn, GasConsumer::Boost], true, true, BOOST, REEL, &mut second);
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
            let g = book(&order, true, true, BOOST, REEL, &mut gas);
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
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], true, false, BOOST, REEL, &mut gas);
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
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], false, false, BOOST, REEL, &mut gas);
        assert_eq!(g, GasGrant::default());
        assert!((gas.current - 100.0).abs() < 1e-6, "tank at {}", gas.current);
    }

    #[test]
    fn f018_the_sandbox_tank_grants_everything_and_stays_full() {
        // `--sandbox`: infinite gas, for looking around (§12a).
        let mut gas = Gas { unlimited: true, ..Gas::full(1.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], true, true, BOOST, REEL, &mut gas);
        assert!(g.boost && g.reel_in, "in the sandbox both get fuel: {g:?}");
        assert!((gas.current - 1.0).abs() < 1e-6, "and nothing leaves the tank");
        assert!(!gas.is_empty());
    }

    /// One tick at 60 Hz, and one tick of a 10/s refill.
    const DT: f32 = 1.0 / 60.0;
    const REGEN: f32 = 10.0 / 60.0;

    #[test]
    fn f018_the_pause_runs_out_before_the_first_drop_goes_in() {
        // 0.5 s at 60 Hz is 30 ticks of waiting, and not one of them puts gas back.
        let mut gas = Gas { current: 0.0, ..Gas::full(300.0) };
        arm_pause(&mut gas, 0.5);
        for tick in 0..30 {
            refill_tank(&mut gas, REGEN, DT);
            assert_eq!(gas.current, 0.0, "tick {tick} of the pause put gas back");
        }
        assert!(gas.regen_delay_left_s <= 0.0, "after 30 ticks the pause is over");
        refill_tank(&mut gas, REGEN, DT);
        assert!((gas.current - REGEN).abs() < 1e-6, "and then it refills: {}", gas.current);
    }

    #[test]
    fn f018_the_refill_stops_at_the_top_and_changes_nothing_there() {
        let mut gas = Gas::full(300.0);
        let before = gas;
        for _ in 0..600 {
            refill_tank(&mut gas, REGEN, DT);
        }
        assert_eq!(gas, before, "a full tank must come out of the refill byte-identical");
    }

    #[test]
    fn f018_an_unlimited_tank_is_left_alone_by_the_refill() {
        // `--sandbox`: nothing to put back, and nothing to wait for either. A pause counting
        // down on it would mark `Gas` as changed while nothing about it changed.
        let mut gas = Gas { unlimited: true, ..Gas::full(1.0) };
        let before = gas;
        arm_pause(&mut gas, 0.5);
        assert_eq!(gas, before, "an unlimited tank does not arm a pause");
        refill_tank(&mut gas, REGEN, DT);
        assert_eq!(gas, before, "and it does not refill");
    }

    #[test]
    fn f018_a_doubled_entry_in_the_file_does_not_debit_twice() {
        // A broken `gas_priority` is a data error and has its own test in
        // `tests/vector_gas.rs`. Until somebody sees it, it must not quietly cost double.
        let mut gas = Gas::full(100.0);
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Boost, GasConsumer::ReelIn],
            true,
            false,
            BOOST,
            REEL,
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
