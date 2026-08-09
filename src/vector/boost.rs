//! `F-007` Gas boost — impulse along the look direction for as long as the button is held.
//!
//! Writes **only** [`BoostAccel`], never `Velocity` and never `Transform`: the collection
//! point belongs to the integrator. That way the boost acts in the air **and** on the rope at
//! the same time, without two systems fighting over the same `Transform` — exactly the case
//! the old state split "`player` on the ground, `vector` on the rope" broke on.
//!
//! Does **not** call `Gas::try_spend`: the account belongs to `vector::gas`. All that is read
//! here is [`GasGrant`]. Without a grant the drive holds `Vec3::ZERO` — no half boost
//! (`F-018`: „Bei 0 kein Fliegen mehr").
//!
//! `F-006` Swerve and `F-008` Dash dock on here later: one system more, not one type more.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{BoostAccel, GasGrant, Intent};

/// Writes [`BoostAccel`] = look direction * `vector.boost_m_s2`, or `ZERO`.
// filled in by job B — F-007
pub fn gas_boost(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &GasGrant, &mut BoostAccel)>,
) {
}
