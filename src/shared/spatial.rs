//! The spatial index — **nothing walks over all entities to answer a question about the ten
//! meters in front of your nose** (`prompts/init.md` §11, `T-036a`).
//!
//! Hook impact (`F-002`), collision (`F-013`), blade hits and titan target search go through
//! here, **all of them**. It is maintained by `world::index`; the type lives in `shared/` so
//! that `vector` and `player` can use it **without an edge to `world`** — exactly the pattern
//! `shared::geometry` already holds for `Block`.
//!
//! Admissible as a `Resource`, because it is **world state** and not player state: there is
//! not a single authoritative per-player field in this type (`docs/multiplayer.md` rule 3
//! forbids only the latter).
//!
//! ## Three decisions you do not reopen without a measurement
//!
//! 1. **A grid over X and Z only.** A city measures 560 m horizontally (Ashgate) and rarely
//!    40 m vertically. A third axis triples the cells and separates almost nothing.
//! 2. **Large bodies do NOT go into the grid.** The ground is a cuboid of 400 x 400 m; with
//!    8 m cells it would lie in 2500 cells and every horizontal ray would test it once per
//!    visited cell. Whatever occupies more than `large_body_cells` cells lands in a linear
//!    list that every ray checks **exactly once**.
//! 3. **No `Default`.** `cell_m = 0.0` would be a division by zero in the DDA, and
//!    `app.init_resource::<SpatialIndex>()` is the most obvious line in the world. There is
//!    only [`SpatialIndex::new`].
//!
//! ## Why the directory
//!
//! `remove(id)` without a position would have to search the whole grid — on 840 m maps that
//! is over 70 000 cells **per despawn**, and `F-029` (anchors on titan limbs) and `T-020`
//! (streaming) despawn all the time. The `BTreeMap` directory costs one entry per body and
//! turns that into an access to the cells the body really occupies. `BTreeMap` and not
//! `HashMap`: the order of an iteration is part of the determinism.

use std::collections::BTreeMap;

use bevy::math::Dir3;
use bevy::prelude::*;

use super::geometry::BodyMask;
use super::ids::BodyId;

/// A body the way the index carries it: axis-aligned hull plus mask.
///
/// A copy, not a reference — the index is read while the world moves, and an `Entity`
/// belongs in nothing that gets saved or sent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexEntry {
    pub id: BodyId,
    /// Center of the hull in world coordinates.
    pub center_m: Vec3,
    /// Half edge length. `Aabb3d::new` takes exactly this form.
    pub half_size_m: Vec3,
    pub mask: BodyMask,
}

impl IndexEntry {
    pub fn min_m(&self) -> Vec3 {
        self.center_m - self.half_size_m
    }

    pub fn max_m(&self) -> Vec3 {
        self.center_m + self.half_size_m
    }
}

/// The nearest solid hit of a ray.
///
/// **The mask is delivered along, not filtered out beforehand** (`F-023`: "line-of-sight
/// checking prevents hooking through walls"). A ray that skips untagged bodies hooks through
/// walls; the caller decides whether the hit is anchorable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub body: BodyId,
    pub point_m: Vec3,
    /// Surface normal at the hit point, unit length.
    pub normal_m: Vec3,
    /// Distance from the ray origin in meters.
    pub distance_m: f32,
    pub mask: BodyMask,
}

/// Result of a ray query.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RayResult {
    pub hit: Option<RayHit>,
    /// How many hulls were really checked.
    ///
    /// **Diagnostics, not evidence.** A linear implementation that reports `1` here would
    /// pass any counter bound — the cost is measured from the outside (`docs/interface.md`,
    /// criterion `T-036a`).
    pub checked: u32,
}

/// Grid cells over XZ plus one list for large bodies.
#[derive(Resource, Debug)]
pub struct SpatialIndex {
    cell_m: f32,
    half_extent_m: f32,
    columns: usize,
    large_body_cells: u32,
    /// `columns * columns` cells, allocated once at startup, never per tick.
    cells: Vec<Vec<IndexEntry>>,
    /// Bodies that occupy too many cells. Every ray checks them exactly once.
    large: Vec<IndexEntry>,
    /// Where every body stands — for `remove` and for the anchor lookup.
    directory: BTreeMap<BodyId, IndexEntry>,
    /// Mailbox of the `on_remove` observer (see [`SpatialIndex::queue_removal`]).
    pending_removals: Vec<BodyId>,
}

impl SpatialIndex {
    /// The **only** constructor. All three numbers come from `assets/data/game.ron`
    /// (`world.cell_m`, `world.half_extent_m`, `world.large_body_cells`).
    ///
    /// Aborts if the cell size is not positive: a zero would be a division by zero in the
    /// DDA, three systems later and with no hint at the file. `tests/data.rs` catches the
    /// same case as a red test instead of a crashed game.
    pub fn new(cell_m: f32, half_extent_m: f32, large_body_cells: u32) -> Self {
        assert!(
            cell_m.is_finite() && cell_m > 0.0,
            "world.cell_m = {cell_m} — must be finite and > 0 (assets/data/game.ron)"
        );
        assert!(
            half_extent_m.is_finite() && half_extent_m > 0.0,
            "world.half_extent_m = {half_extent_m} — must be finite and > 0"
        );
        let columns = ((2.0 * half_extent_m / cell_m).ceil() as usize).max(1);
        SpatialIndex {
            cell_m,
            half_extent_m,
            columns,
            large_body_cells: large_body_cells.max(1),
            cells: vec![Vec::new(); columns * columns],
            large: Vec::new(),
            directory: BTreeMap::new(),
            pending_removals: Vec::new(),
        }
    }

    pub fn cell_m(&self) -> f32 {
        self.cell_m
    }

    pub fn half_extent_m(&self) -> f32 {
        self.half_extent_m
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    /// How many bodies the index knows — grid and large bodies together.
    pub fn len(&self) -> usize {
        self.directory.len()
    }

    /// How many bodies lie in the large-body list.
    pub fn large_len(&self) -> usize {
        self.large.len()
    }

    /// A body by its id — the lookup a hook computes its anchor point in world coordinates
    /// from (`Body` center + `local_m`). `None` means: **the carrier is gone**, and the hook
    /// releases with `ReleaseReason::BodyGone`.
    pub fn body(&self, id: BodyId) -> Option<IndexEntry> {
        self.directory.get(&id).copied()
    }

    /// Take a body in, or replace its hull.
    pub fn insert(&mut self, entry: IndexEntry) {
        self.remove(entry.id);
        match self.cell_range(entry.center_m, entry.half_size_m) {
            Some(range) if range.cell_count() <= self.large_body_cells as usize => {
                for z in range.iz0..=range.iz1 {
                    for x in range.ix0..=range.ix1 {
                        self.cells[z * self.columns + x].push(entry);
                    }
                }
            }
            _ => self.large.push(entry),
        }
        self.directory.insert(entry.id, entry);
    }

    /// Take a body out of the index. `true` if it was in there.
    pub fn remove(&mut self, id: BodyId) -> bool {
        let Some(old) = self.directory.remove(&id) else {
            return false;
        };
        match self.cell_range(old.center_m, old.half_size_m) {
            Some(range) if range.cell_count() <= self.large_body_cells as usize => {
                for z in range.iz0..=range.iz1 {
                    for x in range.ix0..=range.ix1 {
                        self.cells[z * self.columns + x].retain(|e| e.id != id);
                    }
                }
            }
            _ => self.large.retain(|e| e.id != id),
        }
        true
    }

    /// **The mailbox.** The `on_remove` observer in `world::index` pushes the id of a
    /// disappearing body in here; the maintainer collects it in the next fixed step.
    ///
    /// Why not `RemovedComponents`: its buffers are swapped in `World::clear_trackers`, and
    /// that runs **once per `App::update`** (`bevy_app-0.19.0/src/sub_app.rs:149`), while
    /// `FixedMain` runs 0..n times per frame (`bevy_time-0.19.0/src/fixed.rs:37-39`).
    /// Headless drives 240 Hz against 60 Hz fixed — **three out of four frames lose the
    /// report**, and the index goes stale exactly where the docs say it cannot.
    pub fn queue_removal(&mut self, id: BodyId) {
        self.pending_removals.push(id);
    }

    /// Collect the pending removals and empty the mailbox.
    pub fn take_removals(&mut self) -> Vec<BodyId> {
        std::mem::take(&mut self.pending_removals)
    }

    /// The nearest **solid** hit of a ray, mask included (`E14`: hit first, then check
    /// anchorable).
    ///
    /// 2D DDA over XZ; per visited cell `RayCast3d::aabb_intersection_at`
    /// (`bevy_math-0.19.0/src/bounding/raycast3d.rs:49`), stopping as soon as the nearest hit
    /// lies closer than the exit of the current cell. The large-body list is checked
    /// **once** up front.
    ///
    /// Careful when filling this in: `aabb_intersection_at` clamps `tmin` to 0
    /// (`raycast3d.rs:64`) — an origin **inside** the box yields `Some(0.0)`.
    // filled in by job R — T-036a
    pub fn cast_ray(&self, _origin_m: Vec3, _richtung: Dir3, _weite_m: f32) -> RayResult {
        RayResult::default()
    }

    /// Every body whose hull could touch the box. `out` is cleared and filled anew — the
    /// caller holds the buffer, so that no system allocates per tick.
    // filled in by job R — T-036a
    pub fn aabb_overlaps(&self, _mitte_m: Vec3, _halb_m: Vec3, out: &mut Vec<IndexEntry>) {
        out.clear();
    }

    /// The cell range a hull touches — clamped to the grid. `None` means: completely
    /// outside.
    fn cell_range(&self, center_m: Vec3, half_size_m: Vec3) -> Option<CellRange> {
        if !(center_m.is_finite() && half_size_m.is_finite()) {
            return None;
        }
        let min = center_m - half_size_m;
        let max = center_m + half_size_m;
        let limit = self.half_extent_m;
        if max.x < -limit || min.x > limit || max.z < -limit || min.z > limit {
            return None;
        }
        Some(CellRange {
            ix0: self.column(min.x),
            ix1: self.column(max.x),
            iz0: self.column(min.z),
            iz1: self.column(max.z),
        })
    }

    /// World coordinate -> column index, clamped to the edge. A body just outside does not
    /// vanish with this, it lands in the edge cell.
    fn column(&self, value_m: f32) -> usize {
        let raw = (value_m + self.half_extent_m) / self.cell_m;
        if !raw.is_finite() || raw < 0.0 {
            return 0;
        }
        (raw as usize).min(self.columns - 1)
    }
}

#[derive(Clone, Copy, Debug)]
struct CellRange {
    ix0: usize,
    ix1: usize,
    iz0: usize,
    iz1: usize,
}

impl CellRange {
    fn cell_count(&self) -> usize {
        (self.ix1 - self.ix0 + 1) * (self.iz1 - self.iz0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SpatialIndex {
        SpatialIndex::new(8.0, 320.0, 64)
    }

    fn block(id: u32, center: Vec3, half: Vec3) -> IndexEntry {
        IndexEntry { id: BodyId(id), center_m: center, half_size_m: half, mask: BodyMask::SOLID }
    }

    #[test]
    fn t036a_the_grid_covers_the_whole_map() {
        let i = index();
        assert_eq!(i.columns(), 80, "2 * 320 m / 8 m = 80 columns");
        assert_eq!(i.len(), 0);
    }

    #[test]
    fn t036a_a_body_is_found_and_removed_again() {
        let mut i = index();
        i.insert(block(1, Vec3::new(10.0, 6.0, -20.0), Vec3::splat(5.0)));
        assert_eq!(i.len(), 1);
        assert_eq!(i.body(BodyId(1)).map(|e| e.center_m), Some(Vec3::new(10.0, 6.0, -20.0)));
        assert!(i.remove(BodyId(1)));
        assert_eq!(i.len(), 0);
        assert_eq!(i.body(BodyId(1)), None);
        assert!(!i.remove(BodyId(1)), "removing twice is not an error, but not a hit either");
    }

    #[test]
    fn t036a_insert_replaces_instead_of_duplicating() {
        let mut i = index();
        i.insert(block(7, Vec3::new(0.0, 0.0, 0.0), Vec3::splat(2.0)));
        i.insert(block(7, Vec3::new(100.0, 0.0, 100.0), Vec3::splat(2.0)));
        assert_eq!(i.len(), 1);
        assert_eq!(i.body(BodyId(7)).map(|e| e.center_m), Some(Vec3::new(100.0, 0.0, 100.0)));
    }

    #[test]
    fn t036a_the_ground_lands_in_the_large_body_list() {
        // 400 x 400 m at 8 m cells is 2500 cells — the ground does NOT belong in the grid.
        let mut i = index();
        i.insert(block(1, Vec3::new(0.0, -0.1, 0.0), Vec3::new(200.0, 0.1, 200.0)));
        i.insert(block(2, Vec3::new(12.0, 8.0, -30.0), Vec3::new(6.0, 8.0, 6.0)));
        assert_eq!(i.large_len(), 1, "exactly the ground");
        assert_eq!(i.len(), 2);
        assert!(i.remove(BodyId(1)));
        assert_eq!(i.large_len(), 0);
    }

    #[test]
    fn t036a_a_body_outside_the_grid_is_not_lost() {
        let mut i = index();
        // Far outside: falls into no cell range and lands in the large list.
        i.insert(block(3, Vec3::new(9000.0, 0.0, 9000.0), Vec3::splat(1.0)));
        assert_eq!(i.len(), 1);
        assert!(i.body(BodyId(3)).is_some());
        // Just outside: gets clamped into the edge cell and stays findable.
        i.insert(block(4, Vec3::new(319.0, 0.0, -319.0), Vec3::splat(1.0)));
        assert_eq!(i.len(), 2);
        assert!(i.remove(BodyId(4)));
    }

    #[test]
    fn t036a_the_mailbox_survives_any_number_of_frames() {
        // Exactly the case `RemovedComponents` fails at: several reports, collected only
        // later.
        let mut i = index();
        i.queue_removal(BodyId(1));
        i.queue_removal(BodyId(2));
        i.queue_removal(BodyId(1));
        let pending = i.take_removals();
        assert_eq!(pending, vec![BodyId(1), BodyId(2), BodyId(1)]);
        assert!(i.take_removals().is_empty(), "the mailbox is emptied when it is collected");
    }

    #[test]
    #[should_panic(expected = "world.cell_m")]
    fn t036a_a_cell_size_of_zero_panics_at_construction() {
        // Without this abort it would be a division by zero in the DDA, three systems later
        // and with no hint at the file.
        let _ = SpatialIndex::new(0.0, 320.0, 64);
    }
}
