//! save — the save game: profile, gear budget, Traits, Lineage, progress
//!
//! The Bible's requirement holds unchanged, even though ProfileStore means nothing here:
//! **no data loss, no duplication.** Retrofitting it is practically impossible (Bible 6.4) —
//! which is why the domain stands in the tree from day 1, empty as it is.
//!
//! A save game hangs on a [`PlayerId`](crate::shared::PlayerId), not on an `Entity` and not on
//! a connection.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, _app: &mut App) {}
}
