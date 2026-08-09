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

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Gas, GasGrant, Hook, Intent};

/// Debits this tick's gas and writes [`GasGrant`].
// filled in by job G — F-018
pub fn gas_budget(
    _zeit: Res<Time<Fixed>>,
    _daten: Res<GameData>,
    mut _spieler: Query<(&Intent, &Hook, &mut Gas, &mut GasGrant)>,
) {
}
