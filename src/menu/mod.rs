//! menu — main menu, pause, options
//!
//! For somebody who cannot click, a main menu is a wall without a door — which is why
//! `--sandbox`, `--mission` and `--script` exist and walk straight past it
//! (`prompts/init.md` §12a).
//!
//! Rebindable keys, color-blind modes, a screenshake slider and reduced motion are
//! requirements, not decoration (Bible 3.5).
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, _app: &mut App) {}
}
