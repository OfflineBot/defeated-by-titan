//! combat — hits, damage out of speed, amputation, steam, death
//!
//! **Damage comes out of speed.** A slash from standing scratches, the same slash at 30 m/s
//! kills — and the formula belongs in the RON, not in the code (`prompts/init.md` §1, §4).
//!
//! **The cortex is the only truth:** a cortex hit kills, no matter how full the titan is.
//! Everything else is preparation.
//!
//! No splatter: titans evaporate, wounds vent steam (Bible 3.3). That had two independent
//! reasons anyway, and it stays as a style rule even without platform moderation.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, _app: &mut App) {}
}
