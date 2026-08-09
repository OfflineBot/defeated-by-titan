//! `F-007` Gas boost — an acceleration along the look direction for as long as the tank pays.
//!
//! ## It is an acceleration, not a force — and that is a decision about the RON file
//!
//! avian offers both. A force would drag the **mass** into every game value: the player's
//! capsule (`radius_m` 0.35, `height_m` 1.8) has a `ComputedMass` of **0.6029 kg** at avian's
//! default density (measured `[offlinebot]`), so a "boost force" of 20 would mean 33 m/s²
//! today and something else the day somebody gives the player a density. `boost_m_s2` in
//! `game.ron` is a number you can check against gravity in your head — 34 against 20 means
//! "the boost beats gravity by 70 %" on every machine, at every mass. So:
//! [`apply_linear_acceleration`], documented in so many words as *"ignoring mass"*
//! (`avian3d-0.7.0/src/dynamics/rigid_body/forces/query_data.rs:475-487`). Measured against
//! the alternative: with `apply_force` a 10 kg player reaches −7.68 m/s where a 0.6 kg one
//! reaches −112.79; with the acceleration both reach **−68.002785, bit for bit**
//! (`tests/vector_boost.rs`).
//!
//! ## Who owns the field this writes into
//!
//! ⚠️ One thing to know before reading the authority table: the [`Forces`] **query data**
//! declares `Write<LinearVelocity>` (`query_data.rs:105-121`), so on paper this system holds
//! mutable access to it. It never writes it — an acceleration goes nowhere near the velocity —
//! but the scheduler counts that access, and `player::locomotion::ground_locomotion` is the
//! one that really does assign it. The two never overlap: `Drive` runs before `Integrate`.
//!
//! [`Forces`] does **not** touch `LinearVelocity` for an acceleration. It adds into
//! `VelocityIntegrationData::linear_increment` (`.../integrator/mod.rs:235-239`), which is an
//! **accumulator**: gravity lands in the same field (`.../integrator/mod.rs:297-298`), it is
//! multiplied by the substep delta once per step, and avian clears it itself after the substep
//! loop (`clear_velocity_increments`, `.../integrator/mod.rs:316-327`). **Nothing is reset by
//! hand here** — a hand-written reset would fight the engine for the one field it already owns.
//!
//! That is also why this system may sit in `SimulationSystems::Drive` next to
//! `reel::reel_in` without a `.chain()`: two contributors adding into an accumulator commute,
//! and the one field each of them *assigns* ([`BoostAccel`] here) is its own.
//!
//! ## Why the gas decides, and the button does not
//!
//! Read here is [`GasGrant`], and **only** that. `Gas::try_spend` is never called from this
//! file — `vector::gas` (`F-018`) books once per tick and publishes the result, so that a
//! nearly empty tank does not let the system order decide who pays (`shared::gear`,
//! `docs/QUESTIONS.md` Q-017).
//!
//! It would be tempting to also ask `Intent::pressed(Buttons::BOOST)` here. That is exactly
//! what this file does not do: **the debit and the thrust have to be one decision.** If the
//! grant said yes and this system said no, the player would have paid gas for nothing — and a
//! leak you cannot see is worse than one you can. [`Intent`] is read for the **direction**
//! only. `vector::gas` states the other half of the same contract in its own header:
//! `GasGrant.boost == true` means the button is held **and** this tick's gas is paid.
//!
//! Without a grant the drive holds `Vec3::ZERO` and the acceleration is exactly zero, not a
//! fraction (`F-018`: at 0 there is no more flying).
//!
//! `F-006` Swerve and `F-008` Dash dock on here later: one system more, not one type more.
//!
//! Seen: `scripts/f-007-boost.txt` · `docs/images/f-007-boost.png` ·
//! measured in `tests/vector_boost.rs`.

use avian3d::prelude::{Forces, WriteRigidBodyForces};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{BoostAccel, GasGrant, Intent};

/// Writes [`BoostAccel`] = look direction * `vector.boost_m_s2`, or `ZERO`, and hands the same
/// vector to avian as a linear acceleration.
///
/// **Sole writer of [`BoostAccel`].** Contributor — never sole writer — of
/// `VelocityIntegrationData::linear_increment`, which belongs to avian.
///
/// `Option<Forces>` and not a plain `Forces`: the physics components arrive with avian's own
/// prepare step, and a player in his very first tick has none of them yet. With a plain
/// `Forces` the whole row would drop out of the query and [`BoostAccel`] would silently keep
/// the value of the tick before — the one thing its "written every tick, even when it is zero"
/// contract exists to prevent (`shared::gear`).
pub fn gas_boost(
    data: Res<GameData>,
    mut players: Query<(&Intent, &GasGrant, &mut BoostAccel, Option<Forces>)>,
) {
    let strength_m_s2 = data.game.vector.boost_m_s2;

    for (intent, grant, mut drive, forces) in &mut players {
        // `look_dir()` is a unit vector by construction (`shared::intent`, checked there over
        // the whole yaw/pitch range), so no `normalize` is needed and no NaN can come out of
        // one: normalizing a zero-length vector is the classic way to make a player vanish
        // from the world (§9d).
        let wanted = if grant.boost { intent.look_dir() * strength_m_s2 } else { Vec3::ZERO };

        // `set_if_neq`: a component that reports itself changed on all sixty ticks makes every
        // `Changed<BoostAccel>` filter behind it worthless — and a player who is not boosting
        // really does not change.
        drive.set_if_neq(BoostAccel(wanted));

        if let Some(mut forces) = forces {
            // avian itself skips a zero vector (`query_data.rs:483`), so the `ZERO` case costs
            // nothing and needs no branch of its own here.
            forces.apply_linear_acceleration(wanted);
        }
    }
}
