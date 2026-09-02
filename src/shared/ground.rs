//! The one terrain entity's render contract — **the corner grid, carried as a component.**
//!
//! §5E replaced ~6 300 quantised ground pads with ONE entity per map: a static trimesh
//! collider over the corner heights of [`super::TerrainField`], and this sheet beside it so
//! `render` can draw **the same surface** without an edge into `world` (the same argument as
//! [`super::Block`]: `world` writes data, `render` turns it into triangles).
//!
//! ## The contract, and it is the module doc of `shared::terrain` restated
//!
//! Every consumer of this sheet MUST triangulate each cell along the fixed diagonal
//! `(i, j) -> (i+1, j+1)` and MUST leave hole cells out. The collider `world::map` builds is
//! exactly that mesh; `TerrainField::height_at_m` evaluates exactly that mesh; a drawing with
//! a flipped diagonal or a filled hole is a picture of a different ground than the one the
//! player stands on.
//!
//! Coordinates are **world space**: corner `(ix, iz)` sits at
//! `(origin_m.x + ix * cell_m, corners_m[iz * (nx + 1) + ix], origin_m.y + iz * cell_m)`.
//! ⚠️ The entity carrying this sheet does NOT sit at the world origin — its `Transform` is
//! the centre of its own AABB, because that is what `world::index` reads as the body centre.
//! A mesh built from these values belongs at `Transform::IDENTITY` (or subtract the carrier's
//! translation before attaching it to the carrier).

use bevy::prelude::{Component, Vec2};

/// The ground of one map, ready to draw. Written once by `world::map::build_map`, read by
/// `render`. One writer, like every other field in this game.
#[derive(Component, Clone, Debug)]
pub struct TerrainSheet {
    /// World x/z of corner `(0, 0)` — the map's `-size/2` corner.
    pub origin_m: Vec2,
    /// Edge of one cell, metres.
    pub cell_m: f32,
    /// Cell counts. Corners are `(nx + 1) x (nz + 1)`.
    pub nx: u32,
    pub nz: u32,
    /// Corner heights in metres over the base plane, row-major `iz * (nx + 1) + ix` — a copy
    /// of [`super::TerrainField::corner_heights`], carried so `render` needs no re-generation
    /// and no `world` edge.
    pub corners_m: Vec<f32>,
    /// Per **cell**, row-major `iz * nx + ix`: no ground here at all (the canal). Hole cells
    /// are not part of the surface — not drawn, not collided.
    pub hole: Vec<bool>,
    /// The ground colours, lowest band to highest, already resolved out of the palette
    /// (`maps.ron: terrain.colors` — *„das soll grass sein"*). The field's own height range
    /// is cut into `colors.len()` equal bands.
    pub colors: Vec<[f32; 3]>,
}
