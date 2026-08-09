//! squad — fellow players and escort: going down, reviving, marking
//!
//! The Bible's four ground rules (3.6) are **not negotiable** and stand here in the code: no
//! damage between players, **no collision** between players (at this speed the single biggest
//! source of frustration there is), separate loot per player, no exclusion in public
//! instances.
//!
//! **Downed instead of dead**: "dead" is a state with a timer, not a removal of the entity.
//! That produces the most valuable moment in co-op design — somebody has to decide whether to
//! land in the middle of titan fire to pull another player back up.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, _app: &mut App) {}
}
