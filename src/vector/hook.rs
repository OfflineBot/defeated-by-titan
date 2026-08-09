//! `F-001` The double hook — the state machine of both arms.
//!
//! `Idle -> Flying -> Anchored -> Retracting -> Idle`, per side independently (`F-001`
//! verbatim: „Zwei unabhaengig steuerbare Enterhaken (links/rechts), einzeln abfeuerbar und
//! loesbar").
//!
//! **It fires on the edge, not on the hold**: `Buttons::just_pressed` against
//! [`PrevButtons`] — holding is not firing. The previous state is a **component on the
//! player**, not a `Local<Buttons>`: a `Local` belongs to the system and is shared by all
//! players (player 2 fires when player 1 lets go), and it is invisible in the snapshot, so it
//! survives no rollback.
//!
//! This module is the **only writer of [`Hook`]** and the only sender of
//! `HookAnchored`/`HookReleased`. Two reasons to release come from outside and are merely
//! carried out here:
//! - `BodyGone` (the carrier is gone) — in the same tick, because
//!   `SimulationSystems::Spatial` runs before `SimulationSystems::Intent`.
//! - `RopeLength::overextended` (the wall has won) — **one tick later**, because the
//!   integrator only sets it in `SimulationSystems::Integrate`. One tick of lag is the price
//!   for `Hook` having exactly one writer.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    Hook, HookReleased, HookAnchored, Intent, BodyGone, PlayerId, SpatialIndex, RopeLength,
    Tick, PrevButtons, AimPoint,
};

/// Drives both arms through their states and reports every change.
// filled in by job H — F-001
pub fn update_hooks(
    _tick: Res<Tick>,
    _zeit: Res<Time<Fixed>>,
    _daten: Res<GameData>,
    _index: Res<SpatialIndex>,
    mut _weg: MessageReader<BodyGone>,
    mut _gesetzt: MessageWriter<HookAnchored>,
    mut _geloest: MessageWriter<HookReleased>,
    mut _spieler: Query<(
        &PlayerId,
        &Intent,
        &PrevButtons,
        &AimPoint,
        &RopeLength,
        &mut Hook,
    )>,
) {
}

/// Stores this tick's buttons for the edge detection in the next one.
///
/// Runs at the end of the step (`SimulationSystems::PostStep`) — until then every reader has
/// seen the edge. **The only writer of [`PrevButtons`]**; anyone else who needs an edge reads
/// this component instead of keeping a second previous state of his own.
// filled in by job H — F-001
pub fn store_prev_buttons(mut _spieler: Query<(&Intent, &mut PrevButtons)>) {}
