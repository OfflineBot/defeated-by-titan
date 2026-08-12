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
//! ## This file only ever subtracts (`docs/QUESTIONS.md` Q-033)
//!
//! **There is no refill here, and its absence is a decision.** The user answered it on
//! 2026-08-12: *„gas refillt nur im main gebäude an bestimmten stationen/objekten"* — gas
//! comes back **at a place you go to**, never on a timer.
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
//! 16.67 s at the numbers of 2026-08-12. The tank is what got bigger for that.
//!
//! `Changed<Gas>` stays a signal because of it. A tick in which nobody wants gas writes
//! nothing at all to the tank, so the HUD is not woken sixty times a second by a number that
//! did not move (`tests/vector_gas.rs::f018_an_idle_tank_never_refills_on_its_own`).
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

    for (intent, hook, mut gas, mut grant) in &mut players {
        let wants_boost = intent.pressed(Buttons::BOOST);
        let wants_reel_in = intent.pressed(Buttons::REEL_IN) && hook.anchored_count() > 0;

        if !wants_boost && !wants_reel_in {
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
            wants_boost,
            wants_reel_in,
            boost_cost,
            reel_cost,
            &mut tank,
        );
        gas.set_if_neq(tank);
        grant.set_if_neq(booked);
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
                    wants_boost,
                    wants_reel,
                    BOOST,
                    REEL,
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
