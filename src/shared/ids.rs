//! Stable ids — **never** Bevy's `Entity` for anything that gets saved or sent.
//!
//! `Entity` is a local index with a generation. On another machine the same number means
//! something else, and after a restart it does too. Ids of our own cost one line today and
//! later save the netcode **and** the save game (`prompts/init.md` §6 rule 7).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Who a player is — across sessions, dropped connections and machines.
///
/// A dropped connection reserves the seat for 120 s (bible 3.6): the state hangs on this id,
/// not on a connection and not on an `Entity`.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct PlayerId(pub u32);

/// Who a titan is. Same reasoning as [`PlayerId`].
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TitanId(pub u32);

/// Who a **body** in the world is — house, roof, ground, later a titan's shoulder.
///
/// A hook remembers this id and **not** the position and **not** the `Entity`: positions
/// move (`F-029`), and `Entity` means something else on another machine. When the carrier
/// disappears — a titan dies, an area is unloaded (`T-020`) — the spatial index reports that
/// through `BodyGone` and the hook releases with `ReleaseReason::BodyGone`.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct BodyId(pub u32);

/// **The only place in the code that knows which player is "me".**
///
/// The camera hangs on it, the HUD hangs on it — and nothing else. Every system that writes
/// `.single()` on a player query instead turns the game into a single-player game, and
/// nobody notices until multiplayer comes around (`prompts/init.md` §6 rule 3, checked by
/// `tests/multiplayer.rs`).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct LocalPlayer;

/// Hands out consecutive ids. Part of the state, so that two machines get the same order —
/// hence a resource with a counter and not randomness.
#[derive(Resource, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct IdCounter {
    pub player: u32,
    pub titan: u32,
    pub body: u32,
}

impl IdCounter {
    pub fn next_player(&mut self) -> PlayerId {
        self.player += 1;
        PlayerId(self.player)
    }

    pub fn next_titan(&mut self) -> TitanId {
        self.titan += 1;
        TitanId(self.titan)
    }

    pub fn next_body(&mut self) -> BodyId {
        self.body += 1;
        BodyId(self.body)
    }
}
