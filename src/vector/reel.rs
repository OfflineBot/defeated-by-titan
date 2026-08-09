//! `F-005` Reel-in — **a change of length, not a pulling force.**
//!
//! Build reel-in as a force toward the anchor and you get the „lineares Ziehen" that `F-004`
//! explicitly rules out. As a change of length the acceleration falls out as a side effect of
//! the rope constraint — and `shared::rope::rope_reel_in` scales the **tangential** velocity
//! by `L_prev / L_new` while it does so. That is the difference between "the player gains
//! height" and "the player gains speed", and the speed is the feel the whole game hangs on.
//!
//! This module writes only the **desired value** ([`ReelSpeed`], in m/s per side); it is the
//! integrator that carries it out, keeps the length and clamps it to `vector.min_rope_m`.
//! One field, one writer.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{ReelSpeed, GasGrant, Hook, Intent};

/// Writes [`ReelSpeed`] per side: `vector.reel_speed_m_s` or 0.
// filled in by job E — F-005
pub fn reel_in(
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Hook, &GasGrant, &mut ReelSpeed)>,
) {
}
