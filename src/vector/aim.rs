//! `F-002` Free aiming by ray — **layer 1 of the aiming system.**
//!
//! `F-002` verbatim: „Raycast aus Kameraposition in Blickrichtung, Reichweite = Range-Stat.
//! Trefferpunkt wird gegen eine gueltige Ankerflaeche geprueft. **Diese Ebene bleibt IMMER
//! aktiv und ist niemals durch das Snap-System ersetzbar.**"
//!
//! ## Two traps, both with evidence
//!
//! 1. **`bevy::picking::MeshRayCast` is forbidden.** `features = ["picking"]` puts it within
//!    reach (`Cargo.toml`, expanded in `bevy-0.19.0/Cargo.toml:2820-2825`) and it iterates
//!    over **all** visible mesh entities — the source says it word for word:
//!    "Check all entities" (`bevy_picking-0.19.0/src/mesh_picking/ray_cast/mod.rs:224`,
//!    `culling_query.par_iter()` at `:228`). That is exactly the §11 breach that works in the
//!    graybox and shows up at a thousand houses. What we ask is `SpatialIndex::cast_ray`.
//! 2. **The ray starts at eye height, not at the player origin.** The origin sits between the
//!    feet (`docs/conventions.md`); `player.eye_height_m` is the same number `render` hangs
//!    the camera on. That is how `vector` gets to the eye point **without knowing the
//!    camera** — no query on `Camera3d`, no edge.
//!
//! And: **hit first, then check anchorable.** The index delivers the nearest solid hit
//! together with its mask; here it is decided whether it is anchorable. A ray that skips
//! untagged bodies hooks through walls — `F-023` forbids that in so many words.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Intent, SpatialIndex, AimPoint};

/// Writes [`AimPoint`] for every player, once per fixed step.
// filled in by job Z — F-002, F-003
pub fn aim(
    _daten: Res<GameData>,
    _index: Res<SpatialIndex>,
    mut _spieler: Query<(&Intent, &Transform, &mut AimPoint)>,
) {
}
