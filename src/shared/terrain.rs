//! The stepped ground of a district — **one integer level per grid cell, and nothing else.**
//!
//! Until 2026-08-13 this game had no terrain at all: `maps.ron` placed one 700 x 700 m slab at
//! `y = -0.1` and every house in the district stood on `y = 0`. The user, the same day:
//!
//! > *„adde verschiedene höhen vom boden her! lass es wie die echte stadt aussehen! aktuell
//! > kann man es noch nicht erkennen!"*
//!
//! ## Why levels and not a height in metres
//!
//! Every body in this world is an **axis-aligned cuboid** (`world::map`: no block is ever
//! rotated, because a rotated `Cuboid` yields the enclosing oversized AABB and the hook
//! visibly catches in mid-air). A continuous height field cannot be built out of those; a
//! **stepped plateau** can. So the ground is an integer level per cell, and a level is worth
//! `step_m` metres. Everything else — where a terrace ends, how many stairs it takes to walk
//! up it — is arithmetic on that integer.
//!
//! ## The one invariant, and it is what makes the terrain walkable
//!
//! **Two cells that share an edge never differ by more than one level**, and a cell on the
//! rim never differs from the ground outside the grid by more than one either. That is not a
//! hope, it is [`TerrainField::new`]'s post-condition: the field is relaxed until it holds.
//! A terrace you cannot walk up is a wall, not terrain — and one level is exactly what
//! `step_m / stair_rise_m` steps of `stair_tread_m` fit into the half-street between two
//! cells (`world::map`, which asserts that arithmetic against the street width).
//!
//! ## The shape comes from the flat cells, not from the noise
//!
//! `new` takes a `flat` predicate: a cell that carries something hand-placed — the canal, the
//! wall, the gate axis, the market square — is pinned to level 0, because a terrace cannot
//! grow through a quay wall or bury a door. The relaxation then turns that set into a
//! **distance transform**: the ground rises step by step as you walk away from the streets and
//! the water into the middle of a quarter, and it drops back down as you approach the next
//! one. That is what a real walled town on a slope does, and it is the reason this file needs
//! almost no rng — the draw only breaks the plateau that the distance transform would
//! otherwise leave on top.
//!
//! **And "almost no rng" is a number, measured 2026-08-18** (`docs/FINDINGS.md` FIND-101).
//! The relaxation's fixed point is `min` over every pinned cell and the outside of the L1
//! distance to it, capped by the cell's own draw `levels - 1 - notch`. So the seed changes a
//! cell **if and only if that cell lies `levels - 1` or more away from every pinned cell and
//! from the rim** — nearer than that the distance transform is already below the draw and
//! erases it. On the shipped Ashgate (16 x 16 cells, 34 % of them pinned, `levels: 6`) that is
//! **one cell out of 256**, and it is the district's only level-5 cell. `rng` and `stream` are
//! therefore real parameters by a hair, not decorative ones — and the sixth level of that map
//! is one cell wide and seed-dependent, which is a property of `maps.ron`, not of this file.
//!
//! No Bevy, no `data`, no side effects: `shared` is free for every domain, and a field that
//! can be built without an app is a field `tests/world.rs` can measure value by value.

use super::Rng;

/// How many different levels a cell may start at before the terraces are carved out of it.
///
/// **Two**, and that is a name rather than a tuning number (`docs/conventions.md` §4): the
/// field's shape comes from the distance to the nearest flat cell, and the draw exists only to
/// stop the interior of a quarter from being one perfectly even plateau. Three or more starting
/// values buy nothing — a cell that starts one level lower pulls all of its neighbours down
/// with it in the relaxation, so the extra spread is eaten again on the way out.
const START_SPREAD: u32 = 2;

/// The level of every cell of one map's terrain grid.
///
/// Built once in `world::map::plan_blocks` and then only read. Cell coordinates are **signed**
/// on the way in ([`Self::level_at`]) so that "the neighbour outside the grid" is an ordinary
/// lookup and not a special case — outside is level 0, which is the ground the district stands
/// on.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainField {
    nx: u32,
    nz: u32,
    step_m: f32,
    /// Row-major, `iz * nx + ix`. A `Vec` and not a map: iteration order is part of the city
    /// and a `HashMap` here would be a desync (`world::map`).
    level: Vec<u32>,
}

impl TerrainField {
    /// The whole generator. `levels` is the **count** (1 = flat), `step_m` the height of one.
    ///
    /// `flat(ix, iz)` is asked exactly once per cell and pins that cell to level 0.
    ///
    /// Three passes, and the order is the argument:
    /// 1. every free cell starts at the ceiling, minus a per-cell notch out of the seed;
    /// 2. every flat cell is pinned to 0;
    /// 3. the field is relaxed — `level <= min(neighbours) + 1`, with everything outside the
    ///    grid counting as 0 — until nothing changes. The relaxation only ever *lowers*, so it
    ///    terminates in at most `levels` sweeps, and its fixed point is precisely the
    ///    invariant this type promises.
    pub fn new(
        nx: u32,
        nz: u32,
        levels: u32,
        step_m: f32,
        rng: &Rng,
        stream: u64,
        flat: impl Fn(u32, u32) -> bool,
    ) -> Self {
        let ceiling = levels.saturating_sub(1);
        let mut level = vec![0u32; (nx as usize) * (nz as usize)];

        for iz in 0..nz {
            for ix in 0..nx {
                let i = (iz as usize) * (nx as usize) + ix as usize;
                if flat(ix, iz) {
                    continue;
                }
                let notch = rng.index(i as u64, stream, START_SPREAD as usize) as u32;
                level[i] = ceiling.saturating_sub(notch);
            }
        }

        let mut field = Self { nx, nz, step_m, level };
        loop {
            let mut changed = false;
            for iz in 0..nz as i32 {
                for ix in 0..nx as i32 {
                    let cap = field
                        .level_at(ix - 1, iz)
                        .min(field.level_at(ix + 1, iz))
                        .min(field.level_at(ix, iz - 1))
                        .min(field.level_at(ix, iz + 1))
                        + 1;
                    let i = (iz as usize) * (nx as usize) + ix as usize;
                    if field.level[i] > cap {
                        field.level[i] = cap;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        field
    }

    pub fn nx(&self) -> u32 {
        self.nx
    }

    pub fn nz(&self) -> u32 {
        self.nz
    }

    pub fn step_m(&self) -> f32 {
        self.step_m
    }

    /// The level of a cell — **0 outside the grid**, which is the ground the district stands
    /// on and not a missing value.
    pub fn level_at(&self, ix: i32, iz: i32) -> u32 {
        if ix < 0 || iz < 0 || ix >= self.nx as i32 || iz >= self.nz as i32 {
            return 0;
        }
        self.level[(iz as usize) * (self.nx as usize) + ix as usize]
    }

    /// How high the ground of a cell stands over the map's own ground plane, in metres.
    pub fn height_at(&self, ix: i32, iz: i32) -> f32 {
        self.level_at(ix, iz) as f32 * self.step_m
    }

    /// How many levels the neighbour in direction `(dx, dz)` lies **below** this cell.
    ///
    /// `0` means level or higher — nothing to build. By the invariant the answer is never
    /// above 1, and `world::map` asserts that instead of trusting it, because the number
    /// decides how far the stairs run into the street.
    pub fn drop_to(&self, ix: i32, iz: i32, dx: i32, dz: i32) -> u32 {
        self.level_at(ix, iz).saturating_sub(self.level_at(ix + dx, iz + dz))
    }

    /// Every level that really occurs, ascending. The measurement behind "≥ 4 distinct
    /// levels" — a field that came out with two is flat whatever its ceiling says.
    pub fn levels_used(&self) -> Vec<u32> {
        let mut all: Vec<u32> = self.level.clone();
        all.sort_unstable();
        all.dedup();
        all
    }

    /// The height of every cell, in cell order — what a percentile over the ground is taken
    /// from. Not the same as sampling the world on a metre grid: every cell is the same size,
    /// so cell order **is** an area-weighted sample.
    pub fn heights_m(&self) -> Vec<f32> {
        self.level.iter().map(|l| *l as f32 * self.step_m).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(levels: u32, flat: impl Fn(u32, u32) -> bool) -> TerrainField {
        seeded(3405691582, levels, flat)
    }

    /// The same fixture with the seed spelled out — the only thing two grounds may differ in.
    fn seeded(seed: u64, levels: u32, flat: impl Fn(u32, u32) -> bool) -> TerrainField {
        TerrainField::new(12, 12, levels, 0.9, &Rng::new(seed), 0xF003_000D, flat)
    }

    #[test]
    fn f003_no_two_neighbours_differ_by_more_than_one_level() {
        // ★ The invariant the stairs are built on. Red the moment somebody drops the
        // relaxation: measured 2026-08-18 on this very fixture, the pre-relaxation field puts a
        // level-4 cell **against the rim and against the pinned column**, a gap of 4 levels =
        // 3.6 m, and a riser like that in a 6 m street is a wall the player walks into and
        // stops at.
        //
        // ⚠️ This comment used to say *"the raw draw alone puts a level-4 cell next to a
        // level-2 one"*, and that was never true: `START_SPREAD` is 2, so the draw only ever
        // hands out `ceiling` or `ceiling - 1` and two **unpinned** neighbours never differ by
        // more than one level. What the relaxation carves is the slope down to the pins and the
        // rim — nothing else (`docs/FINDINGS.md` FIND-101).
        let f = field(5, |ix, iz| ix == 5 || iz % 7 == 3);
        for iz in 0..12i32 {
            for ix in 0..12i32 {
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let here = f.level_at(ix, iz) as i32;
                    let there = f.level_at(ix + dx, iz + dz) as i32;
                    assert!(
                        (here - there).abs() <= 1,
                        "cell ({ix},{iz}) is at level {here}, its neighbour \
                         ({},{}) at {there}",
                        ix + dx,
                        iz + dz
                    );
                }
            }
        }
    }

    #[test]
    fn f003_a_flat_cell_stays_flat_and_the_rest_still_climbs() {
        // Both halves matter. A generator that honours `flat` by flattening everything is
        // useless, and one that ignores it grows a terrace through the canal wall.
        let f = field(5, |ix, iz| ix == 0 || iz == 0);
        for i in 0..12i32 {
            assert_eq!(f.level_at(0, i), 0, "the pinned column climbed");
            assert_eq!(f.level_at(i, 0), 0, "the pinned row climbed");
        }
        assert!(
            f.levels_used().len() >= 4,
            "only {:?} levels in a 12 x 12 field with a ceiling of 4",
            f.levels_used()
        );
    }

    #[test]
    fn f003_one_level_is_a_provably_flat_map() {
        // ⚠️ `graybox` is built with `levels: 1, step_m: 0.0` and eight `vector_aiming` tests
        // plus four `player` tests reason about `y = 0` on it. If this ever returns anything
        // but zero, those twelve start measuring a city they were never pinned to.
        let f = TerrainField::new(12, 12, 1, 0.0, &Rng::new(7), 0xF003_000D, |_, _| false);
        assert_eq!(f.levels_used(), vec![0]);
        for iz in 0..12i32 {
            for ix in 0..12i32 {
                assert_eq!(f.height_at(ix, iz), 0.0);
            }
        }
    }

    #[test]
    fn f003_outside_the_grid_is_ground_and_the_rim_steps_down_to_it() {
        // The rim is not a special case anywhere in `world::map`; it is this lookup. Without
        // it the district would end in a 3.6 m cliff onto the field outside the wall.
        let f = field(5, |_, _| false);
        assert_eq!(f.level_at(-1, 4), 0);
        assert_eq!(f.level_at(12, 4), 0);
        for i in 0..12i32 {
            assert!(f.level_at(0, i) <= 1, "the rim cell ({},{i}) is at {}", 0, f.level_at(0, i));
            assert!(f.level_at(i, 11) <= 1, "the rim cell ({i},11) is at {}", f.level_at(i, 11));
        }
    }

    #[test]
    fn f003_the_same_seed_yields_exactly_the_same_ground() {
        // Same argument as the city itself: a terrain that differs between two machines is a
        // desync, and it surfaces on the most expensive day there is. Both fixtures, because
        // the draw reaches into the second one and is erased in the first (the test below) —
        // reproducibility has to hold on either side of that line, and only one of the two
        // would have caught a `HashMap` iteration order sneaking into the generator.
        assert_eq!(field(5, |ix, _| ix == 5), field(5, |ix, _| ix == 5));
        assert_eq!(field(5, |ix, iz| ix == 0 || iz == 0), field(5, |ix, iz| ix == 0 || iz == 0));
    }

    #[test]
    fn f003_the_draw_reaches_only_cells_the_relaxation_leaves_room_for() {
        // ★ This replaces an `assert_ne!` that was **wrong about this file** and had never been
        // executed: the five tests in this module are only run by `--lib`, and the round that
        // wrote them was commissioned with `--test world --test data --test render`, so the
        // restriction excluded the binary its own tests live in. It asserted that two seeds
        // give two grounds on the `ix == 5` fixture below — they give byte-identical ones, and
        // that is not a bug, it is the mechanism this module is built on.
        //
        // The rule is exact, measured 2026-08-18 (`docs/FINDINGS.md` FIND-101): the fixed point
        // of the relaxation is the L1 distance transform from every pinned cell and from the
        // outside, capped by the cell's own draw `levels - 1 - notch`. So **the seed can change
        // a cell if and only if that cell lies `levels - 1` or more away from every pin and
        // from the rim.** Both halves below are that one sentence, from both sides.
        //
        // ⚠️ On the shipped Ashgate exactly **one cell of 256** clears that bar. The parameters
        // are not decorative, but they are one cell away from it — see FIND-101 before reading
        // any variety into that map's ground.
        let tight = |ix: u32, _: u32| ix == 5;
        assert_eq!(
            seeded(3405691582, 5, tight),
            seeded(1, 5, tight),
            "two seeds gave two grounds although no cell of this fixture is `levels - 1` = 4 \
             away from the pinned column or the rim — either START_SPREAD grew or the \
             relaxation stopped being a distance transform, and the terrain's shape no longer \
             comes from the map's geometry"
        );

        let roomy = |ix: u32, iz: u32| ix == 0 || iz == 0;
        assert_ne!(
            seeded(3405691582, 5, roomy),
            seeded(1, 5, roomy),
            "two seeds gave the same ground on a 12 x 12 field whose middle is 6 cells from \
             any pin, with a ceiling of 4 — then `rng` and `stream` do nothing anywhere and \
             they are a lie in the signature of a public function"
        );
    }
}
