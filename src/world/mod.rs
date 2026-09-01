//! world — the maps, collision, the spatial index.
//!
//! ⚠️ **There is no authored anchor point list here, and there must not be one again.** The
//! user, 2026-08-27: *„es soll auf jeglicher oberflqche einhaken. nicht an hardcoded punkten
//! etc!"* — hooking is a property of a **surface**, not of a curated list of places. What
//! decides whether a rope bites is [`crate::shared::AnchorSurface`] (and its
//! `BodyMask::ANCHORABLE`, written by [`map`]), and where it bites is `vector::hook`'s
//! raycast. `world::anchor` — an `AnchorField` of generated corner, edge and course points
//! plus the models' adopted `hook.*` empties, and the `hud::anchor_marks` field that drew
//! them — was deleted on 2026-08-28 for exactly that reason: it was the wrong idea, not an
//! unfinished one (`docs/QUESTIONS.md` Q-067, `docs/BUGS.md` B-011 WONT FIX,
//! `docs/PLAN.md` §0, `docs/FINDINGS.md` FIND-197).
//!
//! **The spatial index belongs here** (grid cells -> entities, kept current so that it cannot
//! go stale). Hook impacts, blade hits, collision and titan target search **all** go through
//! it: a city has thousands of houses, and nothing may walk every entity to answer a question
//! about the ten meters in front of your nose (`prompts/init.md` §11).
//!
//! ⚠️ Not through `Added` and **not** through `RemovedComponents`, as this line claimed until
//! 2026-08-09: a new body is found with `Without<BodyId>` (the id and the index entry are
//! written in the same breath, so the filter is exactly "not taken in yet"), and a
//! disappearing one through an **observer** on `Remove`. `RemovedComponents` loses three out
//! of four reports at 240 Hz against 60 Hz fixed — the evidence, with file and line, stands in
//! [`index`].
//!
//! **Where this stands:** [`map::build_map`] really builds the city since 2026-08-09 — out
//! of `assets/data/maps.ron`, placed blocks 1:1 and the layout deterministically from the
//! map's seed. Every block it spawns carries `RigidBody::Static` and a `Collider`, so the
//! ground under the player and the wall in front of him are the same kind of thing as
//! everything else in the file.
//!
//! ⚠️ **`spawn_ground` is gone, and with it the last user of `shared::Ground`.** It was the
//! stand-in for "there is no collision yet": a marker at `y = 0` that the hand-written player
//! integrator clamped against. Since the player is an avian body and the visible ground slab
//! comes out of `maps.ron: blocks[0]` (top edge exactly at y = 0), ground contact comes from
//! the collider. The type `shared::Ground` now has **no reader and no writer left anywhere**
//! — it belongs to `shared/` and is reported for deletion, not deleted here.
//!
//! **Every world collider must carry a `RigidBody`, and `Static` is enough.** Not because
//! anything today needs it — a collider without a body already collides — but because a
//! character controller added later filters on `With<ColliderOf>`
//! (`avian3d-0.7.0/.../move_and_slide.rs:82`) and would be blind to exactly the bodies that
//! carry no `RigidBody`. That is not a bug you find, it is a bug you walk through. [`map`]
//! sets both from one writer.
//!
//! What gets spawned is **data**, not meshes: `render` turns it into triangles without
//! knowing this domain (`shared::Block`).

pub mod bounds;
pub mod index;
pub mod map;
pub mod supply;
pub mod water;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{SimulationSystems, SpatialIndex};

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

        // `F-012` — the fence. Beside `build_map` and not inside it: it is not a block of the
        // district, it is the district's limit, and it deliberately carries neither `Block`
        // nor `Body` (`src/world/bounds.rs`).
        app.add_systems(
            Startup,
            (
                map::build_map,
                supply::build_stations,
                bounds::build_bounds,
                // The river. Beside `build_map` and not inside it, for the reason
                // `bounds::build_bounds` is beside it too: it is not a block of the district.
                // It carries no `Block`, no `Body` and no collider at all (`world::water`).
                water::build_water,
            ),
        )
            .add_systems(
                FixedUpdate,
                (
                    index::maintain_index.in_set(SimulationSystems::Spatial),
                    // `F-019` — in `PostStep`, exactly where `mission::hub` runs its own
                    // stations, so the position it judges is the one this tick's integration
                    // produced. It writes only `RefuelRequest`/`BladeRestockRequest`; the
                    // owning domains apply them next tick (`src/world/supply.rs`).
                    supply::run_the_pumps.in_set(SimulationSystems::PostStep),
                ),
            );
    }
}
