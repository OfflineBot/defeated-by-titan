//! `F-008` the dash's **magazine**, and `F-009` the sideways **flip**.
//!
//! ## Why this file exists at all: what bounds a dash
//!
//! Until 2026-08-24 the answer was *the gas price*, and that answer had already stopped being
//! true. Measured (`docs/QUESTIONS.md` Q-046, `docs/FINDINGS.md` FIND-152): `gas_tank` went
//! `300 -> 15000` for testability, so `vector.gas_dodge: 45` went from **6.7 dashes in a
//! sortie to 333**. At 333 the price is a rounding error. The backlog row for `F-008` says
//! *„Doppeltipp erzeugt einen kurzen, harten Impuls **mit eigenem Cooldown**. Anzahl der Dashes
//! ist ein **Stat**"* — and neither half had ever been built (FIND-067 named the cooldown as
//! missing a fortnight ago).
//!
//! So the bound is now two numbers with two jobs:
//!
//! | | key | question it answers |
//! |---|---|---|
//! | magazine | `vector.dodge_charges` (3) | how many **in a row** |
//! | refill | `vector.dodge_recharge_s` (4 s) | how fast they **come back** |
//! | spacing | `vector.dodge_cooldown_s` (0.6 s) | how fast they **leave** |
//!
//! and the gas price rides on top, unchanged, as the thing that makes the dash the *expensive*
//! impulse next to the boost (`vector::boost`, and `tests/vector_boost.rs` holds the ratio).
//!
//! 🔴 **The gate sits in front of the money, not behind it.** `vector::gas::gas_budget` asks
//! [`DodgeCharges::ready`] in the same expression that asks for a direction, so a dash refused
//! for want of a charge costs **nothing**. That ordering is `FIND-152` read the right way round:
//! a check behind the bill debits 45 gas for an impulse that never happens, and a leak you
//! cannot see is worse than one you can.
//!
//! ## `F-009`, and why the flip is not "a dodge sideways"
//!
//! > *„Doppeltipp A/D erzeugt seitlichen Ausweichsprung in der Luft mit kurzen I-Frames."*
//! > — `docs/features.ron`, `F-009`. Acceptance: *„Flip vermeidet einen Titanengriff, wenn im
//! > Fenster ausgeloest."*
//!
//! The acceptance sentence is the whole feature, and it is not about distance: what a flip buys
//! is a **window in which a blow does not land**. It is therefore cheaper and weaker than the
//! dash on purpose, and it costs no charge — you can always flip, you cannot always dash.
//!
//! | | dash (`F-008`) | flip (`F-009`) |
//! |---|---|---|
//! | gesture | double-tap `Space`, or `C` | double-tap `A`, or double-tap `D` |
//! | direction | where **WASD** points, pitch and all | strictly **sideways**, plus a little up |
//! | strength | `dodge_impulse_m_s` 24 | `flip_impulse_m_s` 18 + `flip_up_m_s` 6 |
//! | price | 45 gas **and a charge** | 20 gas, no charge |
//! | i-frames | none | `flip_iframes_s` |
//! | where | anywhere | **air only** — on the ground it is `F-010`'s slide |
//!
//! ## Who writes what
//!
//! - [`DodgeCharges`] — **this file, and nothing else.** `vector::gas` reads it.
//! - [`Invulnerable`] — this file and `player::locomotion::slide`, both through
//!   [`Invulnerable::extend_to`], which is a `max` and therefore commutes: a flip out of a
//!   slide gives the longer of the two windows and neither can shorten the other.
//! - `VelocityIntegrationData::linear_increment` — **contributor, never sole writer**, exactly
//!   as `vector::boost` documents. The flip does *not* write [`BoostAccel`](crate::shared::BoostAccel):
//!   that component has one writer (`vector::boost::gas_boost`) and two systems assigning one
//!   component is the rule-4 breach `docs/architecture.md` §4 is about.

use avian3d::prelude::{Forces, WriteRigidBodyForces};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{DodgeCharges, GasGrant, Intent, Invulnerable, Tick};

/// Where a flip throws you: **strictly sideways off the yaw, with a fixed lift on top.**
///
/// A free function so the rule can be checked without an app — the same reason
/// [`super::boost::dodge_direction`] is one.
///
/// Three decisions, and each one is the backlog row rather than a taste:
///
/// - **The yaw only. Never the pitch.** *„seitlicher Ausweichsprung"* — a flip that tilted with
///   the camera would drive into the ground in exactly the situation it is for, which is the
///   identical argument `dodge_direction` makes for its `A`/`D` half.
/// - **The sign comes from `move_x`, and nothing else.** On the tick `Buttons::FLIP` fires, the
///   key that produced the gesture is down, so `move_x` is `-1` or `+1`. Zero means the player
///   let go between the two taps and there is no flip — `None`, and `vector::gas` reads that as
///   "then it costs nothing" (the cost-follows-the-effect rule, a third time).
/// - **The lift is not normalised in.** The return value is deliberately **not** a unit vector:
///   `flip_impulse_m_s` and `flip_up_m_s` are two independent speeds in the file, and rolling
///   them into one direction would make either of them change the other's meaning.
pub fn flip_velocity_m_s(yaw: f32, move_x: f32, lateral_m_s: f32, up_m_s: f32) -> Option<Vec3> {
    if move_x == 0.0 || !move_x.is_finite() {
        return None;
    }
    let (sin, cos) = yaw.sin_cos();
    // The same `right` every other file in this repo derives from a yaw
    // (`vector::boost::dodge_direction`, `player::locomotion::air_thrust`): `yaw = 0` looks
    // down −Z, so right is +X (`docs/conventions.md`).
    let right = Vec3::new(cos, 0.0, -sin);
    Some(right * move_x.signum() * lateral_m_s + Vec3::Y * up_m_s)
}

/// `F-008` — spends a charge when the dash fires, and gives one back over time.
///
/// **Sole writer of [`DodgeCharges`].** Runs in `SimulationSystems::Drive`, i.e. *after*
/// `vector::gas::gas_budget` (`Intent`) has decided this tick's [`GasGrant`], so the grant it
/// reads is this tick's and not last tick's — which matters, because a dash that was granted
/// and not charged for is a dash that can be repeated forever.
///
/// **The recharge does not run on the tick a dash fires.** Not for the 1/60 s of a charge it
/// would be worth, but so that `left` after a dash is exactly `n - 1` and a test can say so
/// without a tolerance.
pub fn spend_and_recharge(
    time: Res<Time<Fixed>>,
    tick: Res<Tick>,
    data: Res<GameData>,
    mut players: Query<(&GasGrant, &mut DodgeCharges)>,
) {
    let dt = time.delta_secs();
    let recharge_s = data.game.vector.dodge_recharge_s;
    for (grant, mut charges) in &mut players {
        if grant.dodge {
            // `max(0.0)` and not a plain subtraction: `gas_budget` only grants a dash when
            // `ready()` said there was a whole charge, so this branch cannot go negative — and
            // the day something else grants one, a negative magazine would refill *upward*
            // through zero and hand out a free dash later.
            let next = DodgeCharges {
                left: (charges.left - 1.0).max(0.0),
                spent_at_tick: Some(tick.0),
                ..*charges
            };
            charges.set_if_neq(next);
            continue;
        }
        if charges.left >= charges.max || !(recharge_s > 0.0) {
            // A full magazine is the common case, and `set_if_neq` would still be a comparison
            // per player per tick. Nothing to say.
            continue;
        }
        let next =
            DodgeCharges { left: (charges.left + dt / recharge_s).min(charges.max), ..*charges };
        charges.set_if_neq(next);
    }
}

/// `F-009` — the flip: one tick of sideways velocity, and the i-frames that are the point of it.
///
/// **Contributor** to avian's `linear_increment`, sole writer of nothing but the deadline in
/// [`Invulnerable`] (through `extend_to`, which is a `max`).
///
/// `grant.flip` already means *"the double-tap landed in the air, `move_x` is not zero, and the
/// 20 gas is paid"* — the same contract `grant.boost` and `grant.dodge` carry — so there is no
/// second condition here. Asking `Intent` again for the **direction** is exactly what
/// `vector::boost::gas_boost` does and for the same reason: the debit and the thrust have to be
/// one decision, or a player pays for nothing.
///
/// The impulse is divided by the fixed timestep, like `dodge_impulse_m_s`, so that the number in
/// the file stays the same **speed** if `simulation_hz` ever moves (`vector::boost`'s header
/// works the arithmetic through).
pub fn flip(
    time: Res<Time<Fixed>>,
    tick: Res<Tick>,
    data: Res<GameData>,
    mut players: Query<(&Intent, &GasGrant, &mut Invulnerable, Option<Forces>)>,
) {
    let v = &data.game.vector;
    let dt = time.delta_secs();
    if !(dt > 0.0) {
        return;
    }
    let iframe_ticks =
        (v.flip_iframes_s as f64 * data.game.simulation_hz).round().max(0.0) as u64;

    for (intent, grant, mut iframes, forces) in &mut players {
        if !grant.flip {
            continue;
        }
        let Some(delta_m_s) =
            flip_velocity_m_s(intent.yaw, intent.move_x, v.flip_impulse_m_s, v.flip_up_m_s)
        else {
            // `gas_budget` only grants a flip when `move_x != 0.0`, so this cannot happen on a
            // granted tick. It is `unwrap_or`'s honest form: no panic in the simulation, and no
            // silent thrust either.
            continue;
        };
        if let Some(mut forces) = forces {
            forces.apply_linear_acceleration(delta_m_s / dt);
        }
        // **`extend_to` and not an assignment.** A flip fired out of a slide must not cut the
        // slide's window short, and `max` is the only rule under which two moves that grant
        // i-frames cannot take them away from each other.
        iframes.extend_to(tick.0 + iframe_ticks);
        info!(
            "F-009: flip at tick {} — {:.1} m/s sideways, {:.1} up, i-frames to tick {}",
            tick.0,
            v.flip_impulse_m_s,
            v.flip_up_m_s,
            iframes.until_tick,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f009_a_flip_is_sideways_and_never_takes_the_pitch_with_it() {
        // Looking straight down −Z. Right is +X.
        let v = flip_velocity_m_s(0.0, 1.0, 18.0, 6.0).expect("D is a direction");
        assert!((v.x - 18.0).abs() < 1e-4, "right is +X at yaw 0, got {v:?}");
        assert!(v.z.abs() < 1e-4, "a flip has no forward component at all, got {v:?}");
        assert!((v.y - 6.0).abs() < 1e-4, "and the lift is the file's number, got {v:?}");
    }

    #[test]
    fn f009_the_two_sides_are_exact_mirrors() {
        let right = flip_velocity_m_s(1.1, 1.0, 18.0, 6.0).expect("D");
        let left = flip_velocity_m_s(1.1, -1.0, 18.0, 6.0).expect("A");
        assert!((right.x + left.x).abs() < 1e-4, "{right:?} / {left:?}");
        assert!((right.z + left.z).abs() < 1e-4, "{right:?} / {left:?}");
        assert!((right.y - left.y).abs() < 1e-4, "but the lift is the same for both sides");
    }

    #[test]
    fn f009_a_half_pressed_axis_is_still_one_whole_flip() {
        // `signum`, not the raw value: a gamepad stick at 0.4 is a flip, not 0.4 of one — the
        // strength is `flip_impulse_m_s` and nothing else, which is the same rule
        // `boost::dodge_direction` states for the dash.
        let a = flip_velocity_m_s(0.7, 0.4, 18.0, 6.0).expect("a nudged axis");
        let b = flip_velocity_m_s(0.7, 1.0, 18.0, 6.0).expect("a full axis");
        assert!((a - b).length() < 1e-4, "{a:?} vs {b:?}");
    }

    #[test]
    fn f009_no_sideways_input_is_no_flip() {
        assert!(flip_velocity_m_s(0.0, 0.0, 18.0, 6.0).is_none());
        assert!(flip_velocity_m_s(0.0, f32::NAN, 18.0, 6.0).is_none());
    }

    #[test]
    fn f008_a_fresh_magazine_is_ready_and_an_empty_one_is_not() {
        let full = DodgeCharges::new(3.0);
        assert!(full.ready(0, 36), "nothing has been spent, so nothing can refuse");
        let empty = DodgeCharges { left: 0.0, ..full };
        assert!(!empty.ready(1000, 36), "no charge is no dash, whatever the clock says");
        let partial = DodgeCharges { left: 0.99, ..full };
        assert!(!partial.ready(1000, 36), "0.99 of a charge is not a dash");
    }

    #[test]
    fn f008_the_cooldown_refuses_a_second_dash_inside_the_window() {
        let c = DodgeCharges { left: 3.0, max: 3.0, spent_at_tick: Some(100) };
        assert!(!c.ready(135, 36), "35 ticks after the last one, with 36 asked for");
        assert!(c.ready(136, 36), "and exactly 36 later it is allowed — the bound is inclusive");
    }
}
