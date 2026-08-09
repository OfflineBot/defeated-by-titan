//! The two ropes in the picture — **without them no screenshot is evidence for `F-001`.**
//!
//! Drawn from the player's shoulder to the hook tip
//! ([`HookArm::tip_m`](crate::shared::HookArm)), in **cyan**: the signal color of the Vector
//! Gear (`docs/conventions.md`, cyan = gas / Vector Gear / anchors). Amber and crimson are
//! forbidden here, placeholders included.
//!
//! `render` **reads only**. The state comes out of [`Hook`](crate::shared::Hook); this
//! module writes no field of the simulation.

use bevy::prelude::*;

use crate::shared::Hook;

/// Draws one line per anchored or flying arm.
// to be filled by assignment S — F-001, screenshot required
pub fn draw_ropes(_spieler: Query<(&Hook, &Transform)>, mut _gizmos: Gizmos) {}
