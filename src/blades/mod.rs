//! blades — the blades: swing, wear, breakage, swapping, resupply
//!
//! **Economy instead of cooldowns.** Blades go blunt and break; you reload at supply points,
//! from the horse, or on fallen comrades.
//!
//! Writes [`Blades`](crate::shared::Blades).
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct BladesPlugin;

impl Plugin for BladesPlugin {
    fn build(&self, _app: &mut App) {}
}
