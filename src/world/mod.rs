//! world — the maps, anchor points, collision, the spatial index.
//!
//! **The spatial index belongs here** (grid cells -> entities, maintained through `Added`
//! and `RemovedComponents` so that it cannot go stale). Hook impacts, blade hits, collision
//! and titan target search **all** go through it: a city has thousands of houses, and
//! nothing may walk every entity to answer a question about the ten meters in front of your
//! nose (`prompts/init.md` §11).
//!
//! **Where this stands:** [`map::build_map`] really builds the city since 2026-08-09 — out
//! of `assets/data/maps.ron`, placed blocks 1:1 and the layout deterministically from the
//! map's seed. That retires the four hard-wired placeholder blocks in [`spawn_ground`]; they
//! are the first entries in the file and can be checked there as **behavior-identical**
//! (`tests/world.rs`).
//!
//! ⚠️ What is left in [`spawn_ground`] is **the ground marker and nothing else**. The
//! visible ground slab now comes from the file (`maps.ron: blocks[0]`) — spawned twice it
//! would be a flicker between two coincident surfaces. [`Ground`] itself has **no reader
//! left** today; it dies together with the hard-coded `ground_y = 0.0` in
//! `src/player/mod.rs`, as soon as `player::integrator::step` is filled — not before, or the
//! player falls for 600 ticks and `scripts/t007-first-run.txt` falls with him.
//!
//! What gets spawned is **data**, not meshes: `render` turns it into triangles without
//! knowing this domain (`shared::Block`).

pub mod index;
pub mod map;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Ground, SpatialIndex, SimulationSystems};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // The index needs three numbers out of the RON and **no `Default`**: `cell_m = 0.0`
        // would be a division by zero in the DDA, and `init_resource` is the most obvious
        // line in the world. Hence only `insert_resource(SpatialIndex::new(..))`.
        let w = &app.world().resource::<GameData>().game.world;
        let raum = SpatialIndex::new(w.cell_m, w.half_extent_m, w.large_body_cells);
        app.insert_resource(raum);

        // The observer instead of `RemovedComponents` — reasoning in `world::index`.
        app.add_observer(index::on_body_removed);

        app.add_systems(Startup, (spawn_ground, map::build_map))
            .add_systems(FixedUpdate, index::maintain_index.in_set(SimulationSystems::Spatial));
    }
}

/// The ground marker — **and nothing else any more.**
///
/// The blocks stood here in the code until 2026-08-09; they now stand in
/// `assets/data/maps.ron` and are built by [`map::build_map`]. The visible ground slab comes
/// from there too: this marker carries **no** [`Block`](crate::shared::Block) any more, or
/// two coincident 400 m surfaces would lie on top of each other.
fn spawn_ground(mut commands: Commands) {
    commands.spawn((
        Name::new("ground"),
        Ground { height_m: 0.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
