//! sound — gas hiss, hook impact, blade cut, titan footstep
//!
//! **Every kind of hit has a sound of its own** (Bible 2, pillar P4). And spending gas is
//! loud: the Bellower reacts to it, which couples the resource to the risk.
//!
//! ⚠️ Bevy's audio hangs on ALSA and is therefore hidden behind the `audio` feature — on
//! machine A there is no `alsa.pc` (`docs/environment.md`). Without the feature this domain
//! loads nothing and plays nothing; it says so **once** at startup instead of silently
//! staying silent.
//!
//! Sounds are **measured instead of listened to**: length, fundamental frequency, envelope,
//! peak level, whether it loops. Only original or licensed music (Bible 6.4).
//!
//! **Still empty.** The plugin stands in the tree so that the order in `lib.rs` is right from
//! the start and a fan-out across domains is possible without five agents creating the same
//! folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, _app: &mut App) {}
}
