//! titan — the titans: rig, limbs, cortex, AI
//!
//! **At least half of all enemy kinds carry an anti-autopilot property** (Bible 4) —
//! otherwise the fight degenerates into clicking on targets. Husk, Errant, Scuttler, Weaver,
//! Warden, Lurker, Bellower, Chorus, plus four raid bosses.
//!
//! **Every attack has a windup of at least 0.4 s** and the cortex is readable from 100 m
//! (Bible 2, pillar P4: readability before realism). The player should never have to ask why
//! he died.
//!
//! Reads [`TitanHit`](crate::shared::TitanHit) and decides for itself what a hit means for its
//! body — `combat` does not know how a titan is built.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct TitanPlugin;

impl Plugin for TitanPlugin {
    fn build(&self, _app: &mut App) {}
}
