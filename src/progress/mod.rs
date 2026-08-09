//! progress — XP, Mark/Sigil, gear budget, Traits, Lineage, Ascension
//!
//! ⚠️ **Comes only after the Vector Gear gate.** The skill tree, the economy and Lineages are
//! not started as long as the movement does not feel convincing (Bible 6.1) — the graveyard of
//! this genre is made of games that did it the other way round.
//!
//! **Skill beats numbers** (pillar P2): stat growth opens new content, it does not replace an
//! ability. And **no progress without a guarantee** (P3): every goal is reachable on a
//! deterministic path, and every probability is visible inside the game.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, _app: &mut App) {}
}
