//! mission — the sortie: objectives, phases, spawn waves, victory and defeat
//!
//! **One mission arc runs 5–7 minutes** and is a complete arc with guaranteed, noticeable
//! progress (Bible 5, change 10). The reference has a session length of about 21 minutes —
//! that is 2–4 missions.
//!
//! Mission templates live in `assets/data/missions.ron`: objectives, phases, spawn waves,
//! reward. A new mission is file work, not Rust.
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct MissionPlugin;

impl Plugin for MissionPlugin {
    fn build(&self, _app: &mut App) {}
}
