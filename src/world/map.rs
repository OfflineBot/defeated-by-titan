//! The city out of `assets/data/maps.ron` — **data and a seed, not 200 lines of Rust.**
//!
//! It is built from two sources:
//! 1. `blocks` — explicitly placed cuboids, 1:1 out of the file.
//! 2. `layout` — blocks generated deterministically from `seed` via
//!    [`Rng`](crate::shared::Rng). The same seed yields the same city, on every machine and
//!    in every rollback; `rand::random()` would be a desync here.
//!
//! Every entity gets [`Block`] (that is what `render` sees), [`Body`] (that is what the
//! spatial index sees), the avian pieces [`RigidBody::Static`] and [`Collider::cuboid`], and
//! for `anchorable` an [`AnchorSurface`] on top. **One writer for all four**, so that render
//! shape, index aabb and collision shape cannot drift apart.
//!
//! Since 2026-08-09 the avian components have an **effect**: `PhysicsPlugins` is registered
//! in `src/lib.rs`, and this is the ground the player stands on and the wall he stops at.
//!
//! **`RigidBody::Static` is not optional, even though a bare collider already collides.** A
//! character controller added later filters on `With<ColliderOf>`
//! (`avian3d-0.7.0/.../move_and_slide.rs:82`) and is blind to every collider without a body.
//! Retrofitting that means touching every row of every map — so it stands here from the
//! start, and `tests/player.rs` counts it.
//!
//! ## The trap that does not show up in the picture
//!
//! `Collider::cuboid` takes the **WHOLE edge**, not the half:
//! `avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749` calls
//! `SharedShape::cuboid(x_length * 0.5, ..)` — parry keeps the half internally, avian takes
//! the full one on the outside. [`Body::half_size_m`] and `Aabb3d::new`, by contrast, take
//! the **half** (`bevy_math-0.19.0/src/bounding/bounded3d/mod.rs:66`). A factor of 2 in this
//! spot makes every house twice or half as large without it showing up in the picture —
//! which is why `tests/world.rs::f003_the_colliders_carry_the_half_edge_from_the_file`
//! measures the shape against the file.
//!
//! ## Why the layout does not notice the ground
//!
//! `maps.ron` says: "what is generated leaves room around every placed block". The first
//! placed block is the 400 x 400 m ground slab — a special rule for it would be an
//! `if ground` that nobody ever understands again. Instead [`overlaps`] tests **strictly**
//! (touching does not count): a house stands at y = 0 on the slab whose top edge is at
//! y = 0, and therefore only touches it. Not a special case, just geometry.
//!
//! **No block is ever rotated.** An axis-aligned cuboid is exactly its AABB; a rotated
//! `Cuboid` yields the enclosing, oversized one
//! (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), and the hook
//! visibly catches in mid-air. That is a deliberately deferred limitation
//! (`docs/ROADMAP.md`), not a forgotten one.
//!
//! Seen: `docs/images/f003-city.png`, driven with `scripts/f003-city.txt`.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::data::{GameData, Map, Perimeter};
use crate::shared::{AnchorSurface, Block, Body, Rng};

use super::index::mask_from;

/// Four questions about the same lot, four streams.
///
/// [`Rng`] is stateless and computes out of `(seed, tick, stream)`; **two callers with the
/// same stream get the same number** (`src/shared/rng.rs`). Were the height the same stream
/// as the color, every tall house would have the same color — a pattern you take for intent
/// when you see it in the picture.
///
/// These are **not tuning numbers** but names: they tell callers apart and therefore stand
/// in the code and not in the RON (§4).
const STREAM_BUILT: u64 = 0xF003_0001;
const STREAM_HEIGHT: u64 = 0xF003_0002;
const STREAM_COLOR: u64 = 0xF003_0003;
const STREAM_ANCHORABLE: u64 = 0xF003_0004;

/// How many rng ticks one grid cell owns.
///
/// A closed block draws **per house**, not per cell — otherwise every house in the ring is
/// the same height and the same colour, and the town reads as a stack of identical bars. The
/// tick of house `i` in cell `lot` is `lot * TICKS_PER_LOT + i`, which stays injective as
/// long as a ring never holds more than `TICKS_PER_LOT - 1` houses; [`perimeter_houses`]
/// asserts that instead of silently letting two cells share a draw.
///
/// This is a name, not a tuning number (§4): it says how the rng stream is partitioned.
const TICKS_PER_LOT: u64 = 64;

/// A planned cuboid, **before** it is an entity.
///
/// The plan is separate from the spawning so that `tests/world.rs` can generate the city
/// twice and compare it value by value without building two apps — determinism is the
/// property you lose most cheaply and hunt down most expensively.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockPlan {
    /// `block_<i>` for a placed cuboid, `house_<lot>` for a generated one. The lot is the
    /// **number of the grid cell**, not the order of spawning: a gap in the names is an
    /// unbuilt cell and not a lost entity.
    pub name: String,
    /// World center in meters.
    pub center_m: Vec3,
    /// **Full** edge length in meters, the way `maps.ron` and [`Block`] carry it.
    pub size_m: Vec3,
    pub color: [f32; 3],
    pub anchorable: bool,
    pub solid: bool,
}

impl BlockPlan {
    fn half_size_m(&self) -> Vec3 {
        self.size_m * 0.5
    }

    /// The **only** place where a planned cuboid turns into an entity.
    fn spawn(&self, commands: &mut Commands) {
        let mut e = commands.spawn((
            Name::new(self.name.clone()),
            // What `render` sees: full edge.
            Block { size: self.size_m, color: self.color },
            // What the spatial index sees: half edge.
            Body { half_size_m: self.half_size_m(), mask: mask_from(self.solid, self.anchorable) },
            // What avian sees: the full edge again (see the module header).
            RigidBody::Static,
            Collider::cuboid(self.size_m.x, self.size_m.y, self.size_m.z),
            Transform::from_translation(self.center_m),
        ));
        if self.anchorable {
            e.insert(AnchorSurface);
        }
    }
}

/// Builds the map out of `maps.ron: current` at `Startup`.
///
/// Replaces the blocks that were hard-wired in `world/mod.rs` until 2026-08-09. The first
/// entries in `maps.ron` are exactly those blocks — so that the rebuild is provably
/// **behavior-identical** and not "looks good too".
pub fn build_map(mut commands: Commands, data: Res<GameData>) {
    let Some(map) = data.current_map() else {
        // Loud, not silent: an empty world looks exactly like a render bug (§9d).
        panic!(
            "maps.ron: current = {:?} is not listed under `maps` — there would be no world, \
             and that looks like a render bug",
            data.maps.current
        );
    };

    let plan = plan_blocks(&data, map);
    let anchorable = plan.iter().filter(|r| r.anchorable).count();
    for block in &plan {
        block.spawn(&mut commands);
    }
    info!(
        "map {:?}: {} blocks built ({} placed, {} generated), {anchorable} of them anchorable",
        map.name,
        plan.len(),
        map.blocks.len(),
        plan.len() - map.blocks.len(),
    );
}

/// What is to be built — **without** Bevy, without `Commands`, without side effects.
///
/// Order: first the placed blocks in file order, then the layout in lot order. Both are
/// ordered and neither is a `HashMap` — a city that looks different depending on iteration
/// order is a desync over the network.
pub fn plan_blocks(data: &GameData, map: &Map) -> Vec<BlockPlan> {
    let mut plan: Vec<BlockPlan> = Vec::new();

    for (i, k) in map.blocks.iter().enumerate() {
        plan.push(BlockPlan {
            name: format!("block_{i}"),
            center_m: Vec3::new(k.center_m.0, k.center_m.1, k.center_m.2),
            size_m: Vec3::new(k.size_m.0, k.size_m.1, k.size_m.2),
            color: color_of(data, &k.color),
            anchorable: k.anchorable,
            solid: k.solid,
        });
    }
    let placed = plan.len();

    let r = &map.layout;
    let rng = Rng::new(map.seed);
    let period_m = r.lot_m + r.street_m;
    let nx = lot_count(map.size_m.0, period_m);
    let nz = lot_count(map.size_m.1, period_m);
    // The built-up area is centered on its own extent, not on `nx * period`: no street
    // follows behind the last block, and without this correction the whole city would sit
    // half a street width off center.
    let start_x = -(nx as f32 * period_m - r.street_m) * 0.5;
    let start_z = -(nz as f32 * period_m - r.street_m) * 0.5;

    for iz in 0..nz {
        for ix in 0..nx {
            // The number of the LOT, not of the house. It is the `tick` for the rng:
            // whoever adds a block to `maps.ron` does not thereby shift the heights of
            // every house that follows.
            let lot = (iz * nx + ix) as u64;
            let center_x = start_x + ix as f32 * period_m + r.lot_m * 0.5;
            let center_z = start_z + iz as f32 * period_m + r.lot_m * 0.5;

            // One draw per CELL, not per house: a block is built or it is an open yard.
            // Were this per house, `density` would punch holes into the ring, and a hole in
            // a closed block perimeter is exactly the thing this layout exists to remove.
            if !rng.chance(lot, STREAM_BUILT, r.density) {
                continue;
            }

            // Suburb or old town — the two shapes a cell can have. The `None` arm is
            // deliberately still the old one, box for box and draw for draw, because the
            // graybox is the fixture eight tests in `tests/vector_aiming.rs` are pinned to.
            let footprints: Vec<Footprint> = match &r.perimeter {
                None => vec![Footprint {
                    center_x,
                    center_z,
                    size_x: r.lot_m,
                    size_z: r.lot_m,
                }],
                Some(p) => perimeter_houses(center_x, center_z, r.lot_m, p),
            };
            let single = r.perimeter.is_none();

            for (i, f) in footprints.iter().enumerate() {
                // `lot` itself for the single box, so the graybox draws exactly what it drew
                // before this function learned about rings.
                let tick = if single { lot } else { lot * TICKS_PER_LOT + i as u64 };

                if in_clear_radius(
                    f.center_x,
                    f.center_z,
                    f.size_x * 0.5,
                    f.size_z * 0.5,
                    r.clear_radius_m,
                ) {
                    continue;
                }

                let height_m = rng.range(tick, STREAM_HEIGHT, r.min_height_m, r.max_height_m);
                // A house stands ON the ground: bottom edge y = 0, center at half its height.
                let center_m = Vec3::new(f.center_x, height_m * 0.5, f.center_z);
                let size_m = Vec3::new(f.size_x, height_m, f.size_z);

                // What is explicitly placed wins (`maps.ron`). Only the placed blocks are
                // tested against: two layout houses can never overlap, the ring geometry
                // and the street see to that.
                //
                // ⚠️ The test is **per house, not per cell**. A cell-wide test was what made
                // the aprons cost 43 m of frontage each: the main street's 48 m apron
                // clipped the corner of a block and deleted the whole ring, so the street
                // measured 134 m facade to facade. Per house, an apron deletes exactly the
                // houses standing on it and the rest of the ring keeps the street closed.
                let blocked = plan[..placed]
                    .iter()
                    .any(|g| overlaps(center_m, size_m * 0.5, g.center_m, g.half_size_m()));
                if blocked {
                    continue;
                }

                let colors = &r.colors;
                let color = colors
                    .get(rng.index(tick, STREAM_COLOR, colors.len()))
                    .unwrap_or_else(|| {
                        panic!("maps.ron: layout.colors is empty — every house would be colorless")
                    });

                plan.push(BlockPlan {
                    name: if single { format!("house_{lot}") } else { format!("house_{lot}_{i}") },
                    center_m,
                    size_m,
                    color: color_of(data, color),
                    anchorable: rng.chance(tick, STREAM_ANCHORABLE, r.anchorable_fraction),
                    // A house stops you. That is mechanics and not a tuning question — there
                    // is deliberately no `solid_fraction` in `maps.ron`.
                    solid: true,
                });
            }
        }
    }

    plan
}

/// One house's ground plan inside a cell — everything but its height.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Footprint {
    center_x: f32,
    center_z: f32,
    size_x: f32,
    size_z: f32,
}

/// A closed block: the ring of touching row houses around one cell's courtyard.
///
/// Four runs, and they do **not** overlap: north and south take the full `lot_m` in x and are
/// `wing_depth_m` deep; west and east take only the strip between them. Every run is divided
/// into whole houses of the same width, so two neighbours share a party wall and the gap
/// between them is exactly zero — that is the single change that turns a suburb into a town.
///
/// The courtyard is what is left over, `lot_m - 2 * wing_depth_m` on a side. It is a **gap**,
/// because this world has no subtraction (`docs/FINDINGS.md` FIND-056).
fn perimeter_houses(cx: f32, cz: f32, lot_m: f32, p: &Perimeter) -> Vec<Footprint> {
    let w = p.wing_depth_m;
    let court_m = lot_m - 2.0 * w;
    assert!(
        court_m > 0.0,
        "maps.ron: layout.perimeter.wing_depth_m = {w} leaves no courtyard in a {lot_m} m \
         block — the two wings would grow through each other"
    );

    let mut out = Vec::new();
    // North and south: the street fronts. They carry the run along x.
    let n = runs(lot_m, p.frontage_m);
    let front_m = lot_m / n as f32;
    for k in 0..n {
        let x = cx - lot_m * 0.5 + (k as f32 + 0.5) * front_m;
        out.push(Footprint { center_x: x, center_z: cz - lot_m * 0.5 + w * 0.5, size_x: front_m, size_z: w });
        out.push(Footprint { center_x: x, center_z: cz + lot_m * 0.5 - w * 0.5, size_x: front_m, size_z: w });
    }
    // West and east: only the strip between the two fronts, or the corners would be built
    // twice — and two cuboids in the same place are a z-fight and a doubled collider.
    let m = runs(court_m, p.frontage_m);
    let side_m = court_m / m as f32;
    for k in 0..m {
        let z = cz - court_m * 0.5 + (k as f32 + 0.5) * side_m;
        out.push(Footprint { center_x: cx - lot_m * 0.5 + w * 0.5, center_z: z, size_x: w, size_z: side_m });
        out.push(Footprint { center_x: cx + lot_m * 0.5 - w * 0.5, center_z: z, size_x: w, size_z: side_m });
    }

    assert!(
        (out.len() as u64) < TICKS_PER_LOT,
        "a block of {lot_m} m at a {} m frontage holds {} houses, and only {} rng ticks \
         belong to a cell — two cells would draw the same height",
        p.frontage_m,
        out.len(),
        TICKS_PER_LOT
    );
    out
}

/// How many whole houses a run of `len_m` is divided into. Never zero: a 6 m stub is one
/// narrow house, not a gap in the block.
fn runs(len_m: f32, frontage_m: f32) -> u32 {
    if !(len_m.is_finite() && frontage_m.is_finite()) || frontage_m <= 0.0 {
        return 1;
    }
    ((len_m / frontage_m).round() as i64).max(1) as u32
}

/// How many layout lots fit along one edge. `0` means: no layout.
fn lot_count(edge_m: f32, period_m: f32) -> u32 {
    if !(edge_m.is_finite() && period_m.is_finite()) || period_m <= 0.0 || edge_m <= 0.0 {
        return 0;
    }
    (edge_m / period_m).floor() as u32
}

/// Whether a block comes closer to the origin than `clear_radius_m`.
///
/// Measured from the origin to the **edge** of the block, not to its center: otherwise the
/// clear space depends on the block size, and `clear_radius_m` would stop being a promise.
///
/// The two halves are separate since the ring layout: a row house is 12 m of frontage and
/// 11 m of depth, and one number for both would clear the wrong amount on one of the axes.
fn in_clear_radius(
    center_x: f32,
    center_z: f32,
    half_x_m: f32,
    half_z_m: f32,
    clear_radius_m: f32,
) -> bool {
    let dx = (center_x.abs() - half_x_m).max(0.0);
    let dz = (center_z.abs() - half_z_m).max(0.0);
    dx * dx + dz * dz < clear_radius_m * clear_radius_m
}

/// Strict overlap of two axis-aligned cuboids — **touching does not count.**
///
/// That is exactly what the ground slab slips past: a house with its bottom edge at y = 0
/// and a slab with its top edge at y = 0 have `distance == sum` on the Y axis, and `<` is
/// false. Both sides compute the same sum out of the same floats, so the result is exactly
/// equal and not "nearly".
fn overlaps(a_center: Vec3, a_half: Vec3, b_center: Vec3, b_half: Vec3) -> bool {
    let distance = (a_center - b_center).abs();
    let sum = a_half + b_half;
    distance.x < sum.x && distance.y < sum.y && distance.z < sum.z
}

/// A color out of the one palette — or an abort naming the color that is missing.
///
/// No silent substitute: otherwise one of the three signal colors eventually slips into the
/// scenery (`docs/conventions.md`).
fn color_of(data: &GameData, name: &str) -> [f32; 3] {
    data.color(name).unwrap_or_else(|| {
        panic!("maps.ron: color {name:?} is not listed in `palette`")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f003_a_house_stands_on_the_ground_slab_not_in_it() {
        // The reason the layout builds anything at all: the ground slab is the first placed
        // block and covers the whole map. Were `overlaps` not strict, the city would be
        // empty — and that without a single error message.
        let slab_center = Vec3::new(0.0, -0.1, 0.0);
        let slab_half = Vec3::new(200.0, 0.1, 200.0);
        for height_m in [4.5f32, 7.3, 11.5, 35.0] {
            let house_center = Vec3::new(0.0, height_m * 0.5, 0.0);
            let house_half = Vec3::new(14.0, height_m * 0.5, 14.0);
            assert!(
                !overlaps(house_center, house_half, slab_center, slab_half),
                "a house {height_m} m tall supposedly sits inside the ground slab"
            );
        }
        // And a cellar really is inside it.
        assert!(overlaps(
            Vec3::new(0.0, -0.1, 0.0),
            Vec3::splat(2.0),
            slab_center,
            slab_half
        ));
    }

    #[test]
    fn f003_the_clear_radius_measures_to_the_edge_not_the_center() {
        // A block whose center is 30 m away but whose edge is 16 m: it is in the way.
        assert!(in_clear_radius(30.0, 0.0, 14.0, 14.0, 24.0), "edge at 16 m, radius 24 m");
        assert!(!in_clear_radius(40.0, 0.0, 14.0, 14.0, 24.0), "edge at 26 m, radius 24 m");
        // Diagonally the real distance counts, not the larger of the two axes.
        assert!(!in_clear_radius(35.0, 35.0, 14.0, 14.0, 24.0), "edge diagonally at 29.7 m");
        // A row house is wide and shallow, and the two halves are NOT interchangeable. The
        // same 12 x 11 m footprint, once with its frontage along x and once along z:
        assert!(!in_clear_radius(0.0, 29.5, 6.0, 5.5, 24.0), "edge in z at 24.0 m, radius 24");
        assert!(in_clear_radius(0.0, 29.5, 5.5, 6.0, 24.0), "edge in z at 23.5 m, radius 24");
    }

    #[test]
    fn f003_a_closed_block_is_a_ring_with_no_gap_and_a_courtyard() {
        // The whole point of the layout change. Red the moment a run stops dividing into
        // whole houses — then the block has a slot in it and the street is no longer closed.
        let p = Perimeter { frontage_m: 12.0, wing_depth_m: 11.0 };
        let houses = perimeter_houses(0.0, 0.0, 36.0, &p);
        assert_eq!(houses.len(), 8, "3 + 3 fronts and 1 + 1 flanks");

        // The two street fronts: three houses each, touching, spanning the full 36 m.
        let mut north: Vec<&Footprint> = houses.iter().filter(|f| f.center_z < -6.0).collect();
        north.sort_by(|a, b| a.center_x.total_cmp(&b.center_x));
        assert_eq!(north.len(), 3);
        assert!((north[0].center_x - north[0].size_x * 0.5 + 18.0).abs() < 1e-4);
        for pair in north.windows(2) {
            let gap = (pair[1].center_x - pair[1].size_x * 0.5)
                - (pair[0].center_x + pair[0].size_x * 0.5);
            assert!(gap.abs() < 1e-4, "party wall, not a {gap} m gap");
        }
        assert!((north[2].center_x + north[2].size_x * 0.5 - 18.0).abs() < 1e-4);

        // The courtyard really is empty: nothing overlaps the inner 14 x 14 m.
        for f in &houses {
            let clear = (f.center_x.abs() - f.size_x * 0.5).max(f.center_z.abs() - f.size_z * 0.5);
            assert!(clear >= 7.0 - 1e-4, "{f:?} eats into the 14 m courtyard");
        }

        // And no two houses of a ring overlap — the corners belong to the fronts.
        for (i, a) in houses.iter().enumerate() {
            for b in &houses[i + 1..] {
                let dx = (a.center_x - b.center_x).abs();
                let dz = (a.center_z - b.center_z).abs();
                assert!(
                    !(dx < (a.size_x + b.size_x) * 0.5 - 1e-4
                        && dz < (a.size_z + b.size_z) * 0.5 - 1e-4),
                    "{a:?} and {b:?} stand in the same place"
                );
            }
        }
    }

    #[test]
    fn f003_a_run_is_divided_into_whole_houses() {
        assert_eq!(runs(36.0, 12.0), 3);
        assert_eq!(runs(14.0, 12.0), 1);
        // Never zero: a stub is one narrow house, never a hole in the perimeter.
        assert_eq!(runs(4.0, 12.0), 1);
        assert_eq!(runs(36.0, 0.0), 1);
        assert_eq!(runs(f32::NAN, 12.0), 1);
    }

    #[test]
    fn f003_the_lot_count_drops_the_trailing_street() {
        // 400 m at 28 + 7: eleven blocks are 385 m, twelve would be 420 m.
        assert_eq!(lot_count(400.0, 35.0), 11);
        assert_eq!(lot_count(35.0, 35.0), 1);
        assert_eq!(lot_count(34.0, 35.0), 0);
        // No crash on nonsense, just no layout.
        assert_eq!(lot_count(400.0, 0.0), 0);
        assert_eq!(lot_count(f32::NAN, 35.0), 0);
    }
}
