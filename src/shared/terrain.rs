//! The ground of a district — **a continuous height field, one height per cell, in metres.**
//!
//! Until 2026-08-29 this file built *terraces*: an integer level per 42 m cell, `levels`
//! of them, `step_m` apart, with a flight of stairs cut into every falling edge. The user
//! played that and rejected it in one sentence:
//!
//! > *„auch die verschiedenen hoehen passen nicht! das soll grass sein und nicht so wie jetzt!
//! > und nicht verschiedene hardcoded stufen sondern wirklich terrain! und deutlich hoeher und
//! > niedriger als jetzt!"*
//!
//! Three demands, and the first one is this file: **not `levels x step_m`.** What replaced it
//! is a height field sampled on a small cell (4-6 m instead of 42), quantised to a rise the
//! player can walk over, and shaped by noise inside an envelope that the geometry of the map
//! dictates.
//!
//! ## Why it is still quantised, and why the quantum is 0.25 m
//!
//! Every body in this world is an **axis-aligned cuboid** (`world::map`: no block is ever
//! rotated, because a rotated `Cuboid` yields the enclosing oversized AABB and the hook
//! visibly catches in mid-air). A cuboid has a flat top, so two neighbouring cells at
//! different heights always meet in a **riser** — there is no such thing as a ramp here. The
//! only question a cuboid world gets to answer is *how tall may that riser be*, and that is
//! not a taste but a measurement:
//!
//! | riser, walking `W` from flat at `gravity_m_s2: -32`, tread 3.0 m | climbed |
//! |---|---|
//! | 0.10 · 0.15 · 0.20 · 0.25 · 0.26 · 0.27 m | **all ten steps** |
//! | 0.28 m | one step, then stuck |
//! | 0.29 · 0.30 · 0.35 · 0.40 · 0.50 · 0.60 · 0.80 m | **never leaves the ground** |
//!
//! (measured 2026-08-29 on the shipped binary, `docs/FINDINGS.md` FIND-214; treads of 1.0,
//! 2.0 and 3.0 m make no difference — the riser is the whole discriminator, which is also
//! `docs/BUGS.md` B-018 from the other side.) So `rise_m` is bounded above by **0.27 m** at
//! the provisional `-32`, and `maps.ron` spends 0.25 of it.
//!
//! **That one number fixes the steepest ground in the game:** `rise_m / cell_m`. At 0.25 over
//! 5.0 m that is a **5 % grade**, against the 3.6 % of the terraces — and, far more
//! importantly, it is a grade that holds *everywhere* instead of a 1.5 m cliff every 42 m.
//!
//! ## The three-part envelope, and why the field cannot break the district
//!
//! The noise on its own is far steeper than 5 % and would bury a door or hang a quay wall in
//! the air. It is therefore clamped into an envelope built out of the map's own hand-placed
//! geometry, and every cell has exactly one [`CellRole`]:
//!
//! * [`CellRole::Pin`] — something stands here that the ground may neither lift nor drop: a
//!   quay wall, a gate tower, a market stall, the wall itself, the spawn. `h == 0`.
//! * [`CellRole::Floor`] — a piece of paving lies here, top at or below `paving_top_m`. The
//!   ground may climb over it (that is what a terrace always did) but may **not** sink away
//!   under it and leave it hanging. `h >= 0`.
//! * [`CellRole::Hole`] — the canal. No ground at all; `world::map` emits no block.
//! * [`CellRole::Free`] — everything else, and this is where the relief lives.
//!
//! From those the two bounds are exact and need no iteration to be *correct*:
//!
//! ```text
//! hi(c) =  rise_m * (L1 distance in cells to the nearest Pin)
//! lo(c) = -rise_m * (L1 distance in cells to the nearest Pin or Floor)
//! ```
//!
//! Both are `rise_m`-Lipschitz by construction and `lo <= 0 <= hi`, so the system is always
//! feasible. The noise is clamped between them and then lowered — never raised — until no
//! two neighbours differ by more than one rise. **The fixed point of that descent is the
//! invariant this type promises**, and `world::map` asserts it rather than trusting it.
//!
//! ## What the shape actually comes from
//!
//! Mostly from the envelope, not from the noise — exactly as the terraces did. `maps.ron`
//! asks for amplitudes far above what a 5 % grade can carry, so over most of the map the
//! clamp is what is binding and the ground is a smooth ramp away from the streets, the water
//! and the wall. The noise is what stops the interior of a quarter from being a perfect cone:
//! it decides *where* the high ground is, and the envelope decides how fast you get there.
//!
//! No Bevy, no `data`, no side effects: `shared` is free for every domain, and a field that
//! can be built without an app is a field `tests/world.rs` can measure cell by cell.

use super::Rng;

/// What one cell of the grid is allowed to do, decided by what stands on it.
///
/// Not a bitfield and not an `Option<f32>`: the four cases have four different constraints
/// and naming them is what makes the envelope readable in one line each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRole {
    /// Free ground. The relief lives here.
    Free,
    /// Paving lies here: the ground may rise over it, never fall away under it. `h >= 0`.
    Floor,
    /// Something stands here that the ground may not move. `h == 0`.
    Pin,
    /// No ground at all — the canal. `world::map` emits no block for it.
    Hole,
}

/// The height of every cell of one map's ground, in whole multiples of `rise_m`.
///
/// Built once in `world::map::plan_terrain` and then only read. Cell coordinates are
/// **signed** on the way in ([`Self::step_at`]) and **clamped** to the rim: outside the grid
/// is the nearest cell inside it, not zero. That is the one behavioural difference to the
/// terraced field this replaced, and it is what lets the map's own edge carry relief instead
/// of being pulled back to the base plane by a rule about the void.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainField {
    nx: u32,
    nz: u32,
    cell_m: f32,
    rise_m: f32,
    /// Row-major, `iz * nx + ix`. A `Vec` and not a map: iteration order is part of the city
    /// and a `HashMap` here would be a desync (`world::map`).
    step: Vec<i32>,
    hole: Vec<bool>,
}

impl TerrainField {
    /// A provably flat field — every cell at 0, no holes.
    ///
    /// ⚠️ `graybox` is built with this and eight tests in `tests/vector_aiming.rs` plus four
    /// in `tests/player.rs` reason about `y = 0` on it. A map says it wants this by writing
    /// `amplitude_m: []`, which is a statement and not a missing key (`docs/conventions.md`
    /// §4).
    pub fn flat(nx: u32, nz: u32, cell_m: f32) -> Self {
        Self {
            nx,
            nz,
            cell_m,
            rise_m: 0.0,
            step: vec![0; (nx as usize) * (nz as usize)],
            hole: vec![false; (nx as usize) * (nz as usize)],
        }
    }

    /// The whole generator.
    ///
    /// `amplitude_m[i]` and `wavelength_m[i]` are one octave of value noise each; the shorter
    /// wavelengths break up what the longer ones lay down. `role(ix, iz)` is asked exactly
    /// once per cell.
    ///
    /// Four passes, and the order is the argument:
    /// 1. the raw noise, in metres;
    /// 2. the two distance transforms that give `hi` and `lo`;
    /// 3. the clamp into `[lo, hi]` — after which the field respects every pin, every floor
    ///    and every hole, but is still far too steep;
    /// 4. the descent `h <- min(h, min(neighbours) + rise_m)`, which only ever lowers and
    ///    whose fixed point is exactly "no two neighbours differ by more than one rise".
    pub fn new(
        nx: u32,
        nz: u32,
        cell_m: f32,
        rise_m: f32,
        amplitude_m: &[f32],
        wavelength_m: &[f32],
        rng: &Rng,
        stream: u64,
        role: impl Fn(u32, u32) -> CellRole,
    ) -> Self {
        let n = (nx as usize) * (nz as usize);
        if n == 0 || rise_m <= 0.0 || cell_m <= 0.0 || amplitude_m.is_empty() {
            return Self::flat(nx, nz, cell_m);
        }

        let mut roles = Vec::with_capacity(n);
        for iz in 0..nz {
            for ix in 0..nx {
                roles.push(role(ix, iz));
            }
        }

        // 1 · the raw noise, sampled at the centre of every cell.
        let mut h = vec![0.0f32; n];
        for (octave, (amp, wave)) in amplitude_m.iter().zip(wavelength_m.iter()).enumerate() {
            if *wave <= 0.0 || *amp == 0.0 {
                continue;
            }
            for iz in 0..nz {
                for ix in 0..nx {
                    let x = (ix as f32 + 0.5) * cell_m / *wave;
                    let z = (iz as f32 + 0.5) * cell_m / *wave;
                    h[(iz as usize) * (nx as usize) + ix as usize] +=
                        *amp * value_noise(x, z, octave as u64, rng, stream);
                }
            }
        }

        // 2 · the two envelopes. `hi` is a cone rising out of every pin, `lo` a cone falling
        // away from every pin AND every piece of paving — a valley may not undercut a street.
        let to_pin = l1_distance(nx, nz, |i| matches!(roles[i], CellRole::Pin | CellRole::Hole));
        let to_ground = l1_distance(nx, nz, |i| {
            matches!(roles[i], CellRole::Pin | CellRole::Hole | CellRole::Floor)
        });

        // 3 · the clamp. `hi` is unbounded on a map with no pin at all, which is why the
        // distance transform reports `None` for "never reached" instead of a large number
        // that would silently become a ceiling.
        for i in 0..n {
            let hi = to_pin[i].map(|d| rise_m * d as f32);
            let lo = to_ground[i].map(|d| -rise_m * d as f32);
            if let Some(hi) = hi {
                h[i] = h[i].min(hi);
            }
            if let Some(lo) = lo {
                h[i] = h[i].max(lo);
            }
        }

        // 4 · the descent. Two chamfer sweeps are exact for an L1 min-plus envelope — the
        // forward one carries every constraint down and right, the backward one up and left —
        // and a third pass over the whole grid confirms the fixed point instead of assuming
        // it (the assert lives in `world::map`, which can name the map in its message).
        let idx = |ix: usize, iz: usize| iz * (nx as usize) + ix;
        for _ in 0..2 {
            for iz in 0..nz as usize {
                for ix in 0..nx as usize {
                    let i = idx(ix, iz);
                    if ix > 0 {
                        h[i] = h[i].min(h[idx(ix - 1, iz)] + rise_m);
                    }
                    if iz > 0 {
                        h[i] = h[i].min(h[idx(ix, iz - 1)] + rise_m);
                    }
                }
            }
            for iz in (0..nz as usize).rev() {
                for ix in (0..nx as usize).rev() {
                    let i = idx(ix, iz);
                    if ix + 1 < nx as usize {
                        h[i] = h[i].min(h[idx(ix + 1, iz)] + rise_m);
                    }
                    if iz + 1 < nz as usize {
                        h[i] = h[i].min(h[idx(ix, iz + 1)] + rise_m);
                    }
                }
            }
        }

        let step: Vec<i32> = h.iter().map(|v| (v / rise_m).round() as i32).collect();
        let hole = roles.iter().map(|r| *r == CellRole::Hole).collect();
        Self { nx, nz, cell_m, rise_m, step, hole }
    }

    pub fn nx(&self) -> u32 {
        self.nx
    }

    pub fn nz(&self) -> u32 {
        self.nz
    }

    pub fn cell_m(&self) -> f32 {
        self.cell_m
    }

    /// The quantum of the field, in metres — and the tallest riser a player ever meets on the
    /// ground. Bounded above by what he can walk over; see the table at the top of this file.
    pub fn rise_m(&self) -> f32 {
        self.rise_m
    }

    /// The height of a cell in whole rises — **clamped to the rim**, so "the neighbour outside
    /// the grid" is an ordinary lookup and the map's edge is not pulled back to zero.
    pub fn step_at(&self, ix: i32, iz: i32) -> i32 {
        if self.nx == 0 || self.nz == 0 {
            return 0;
        }
        let ix = ix.clamp(0, self.nx as i32 - 1) as usize;
        let iz = iz.clamp(0, self.nz as i32 - 1) as usize;
        self.step[iz * (self.nx as usize) + ix]
    }

    /// How high the ground of a cell stands over the map's own base plane, in metres.
    pub fn height_at(&self, ix: i32, iz: i32) -> f32 {
        self.step_at(ix, iz) as f32 * self.rise_m
    }

    /// Is there no ground here at all? The canal, and nothing else today.
    pub fn is_hole(&self, ix: i32, iz: i32) -> bool {
        if self.nx == 0 || self.nz == 0 {
            return false;
        }
        let ix = ix.clamp(0, self.nx as i32 - 1) as usize;
        let iz = iz.clamp(0, self.nz as i32 - 1) as usize;
        self.hole[iz * (self.nx as usize) + ix]
    }

    /// The **lowest** ground over a rectangle of cells, in metres.
    ///
    /// This is what a house is founded on, and `min` rather than `max` is the whole point: a
    /// house set on the lowest ground it covers is cut into the slope on its high side, and a
    /// house set on the highest is a house standing on air on its low side. `FIND-134` §3B
    /// cost a round to the second one.
    pub fn lowest_over(&self, ix0: i32, ix1: i32, iz0: i32, iz1: i32) -> f32 {
        let mut lowest = i32::MAX;
        for iz in iz0..=iz1 {
            for ix in ix0..=ix1 {
                lowest = lowest.min(self.step_at(ix, iz));
            }
        }
        if lowest == i32::MAX { 0.0 } else { lowest as f32 * self.rise_m }
    }

    /// Every height that really occurs, ascending. A field that comes out with six of them is
    /// a flight of terraces whatever its parameters say.
    pub fn steps_used(&self) -> Vec<i32> {
        let mut all = self.step.clone();
        all.sort_unstable();
        all.dedup();
        all
    }

    /// The height of every cell, in cell order — what a percentile over the ground is taken
    /// from. Every cell is the same size, so cell order **is** an area-weighted sample.
    pub fn heights_m(&self) -> Vec<f32> {
        self.step.iter().map(|s| *s as f32 * self.rise_m).collect()
    }
}

/// One octave of value noise: a lattice of `[-1, 1]` draws, smoothstepped between.
///
/// Deterministic out of `(seed, lattice point, octave)` — [`Rng`] is stateless, so this is a
/// pure function of the map's seed and nothing accumulates. `x` and `z` are in **lattice
/// units** (world metres divided by the wavelength) and are always >= 0 here, so the floor
/// never has to be corrected for negative operands.
fn value_noise(x: f32, z: f32, octave: u64, rng: &Rng, stream: u64) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (fx, fz) = (x - xi, z - zi);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let at = |dx: i64, dz: i64| {
        let lx = xi as i64 + dx;
        let lz = zi as i64 + dz;
        // One tick per lattice point per octave. The shifts are wide enough that a 700 m map
        // at a 100 m wavelength cannot alias one lattice point onto another.
        let tick = ((lx as u64) << 40) ^ ((lz as u64) << 8) ^ octave;
        rng.range(tick, stream, -1.0, 1.0)
    };
    let a = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let b = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    a + (b - a) * sz
}

/// L1 distance in cells to the nearest cell the predicate holds for — `None` where there is
/// none at all.
///
/// `None` and not `u32::MAX`: a map with no pin has no ceiling, and a sentinel here would
/// quietly become one at whatever value it happened to have.
fn l1_distance(nx: u32, nz: u32, seed: impl Fn(usize) -> bool) -> Vec<Option<u32>> {
    let (w, hgt) = (nx as usize, nz as usize);
    let far = u32::MAX / 4;
    let mut d: Vec<u32> = (0..w * hgt).map(|i| if seed(i) { 0 } else { far }).collect();
    for iz in 0..hgt {
        for ix in 0..w {
            let i = iz * w + ix;
            if ix > 0 {
                d[i] = d[i].min(d[i - 1] + 1);
            }
            if iz > 0 {
                d[i] = d[i].min(d[i - w] + 1);
            }
        }
    }
    for iz in (0..hgt).rev() {
        for ix in (0..w).rev() {
            let i = iz * w + ix;
            if ix + 1 < w {
                d[i] = d[i].min(d[i + 1] + 1);
            }
            if iz + 1 < hgt {
                d[i] = d[i].min(d[i + w] + 1);
            }
        }
    }
    d.into_iter().map(|v| if v >= far { None } else { Some(v) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 24 x 24 cells of 5 m, the shipped rise, one long octave — the smallest fixture in
    /// which a cone from a pin has room to reach its ceiling.
    fn field(role: impl Fn(u32, u32) -> CellRole) -> TerrainField {
        seeded(3405691582, role)
    }

    fn seeded(seed: u64, role: impl Fn(u32, u32) -> CellRole) -> TerrainField {
        TerrainField::new(
            24,
            24,
            5.0,
            0.25,
            &[9.0, 3.0],
            &[110.0, 40.0],
            &Rng::new(seed),
            0xF003_000E,
            role,
        )
    }

    #[test]
    fn f003_no_two_neighbouring_cells_differ_by_more_than_one_rise() {
        // ★ The invariant the whole feature stands on: one rise is 0.25 m and a 0.25 m riser
        // is the tallest thing a player can walk over at `gravity_m_s2: -32` (0.28 m is
        // already a wall — measured, see the table at the top of this file). Red the moment
        // somebody drops the descent, and red for a reason a screenshot cannot show.
        //
        // ## What this sweep varies, and what it holds constant
        //
        // Varies: the role layout (four shapes below), and with it every distance transform
        // the code reads. Holds constant: `nx`, `nz`, `cell_m`, `rise_m`, the octaves and the
        // seed — none of which the invariant can depend on, because the descent's fixed point
        // is defined by `rise_m` alone and `rise_m` cancels out of `|delta step| <= 1`.
        // Skipped: nothing. Every cell of every layout is visited, holes included — a hole is
        // still a height, it just carries no block.
        let layouts: [(&str, fn(u32, u32) -> CellRole); 4] = [
            ("one pin in the middle", |ix, iz| {
                if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free }
            }),
            ("a wall across", |_, iz| if iz == 6 { CellRole::Pin } else { CellRole::Free }),
            ("paving only, no pin at all", |ix, iz| {
                if ix == 3 && iz == 3 { CellRole::Floor } else { CellRole::Free }
            }),
            ("a canal between two quays", |ix, _| match ix {
                10 => CellRole::Hole,
                9 | 11 => CellRole::Pin,
                _ => CellRole::Free,
            }),
        ];
        for (name, role) in layouts {
            let f = field(role);
            for iz in 0..24i32 {
                for ix in 0..24i32 {
                    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let a = f.step_at(ix, iz);
                        let b = f.step_at(ix + dx, iz + dz);
                        assert!(
                            (a - b).abs() <= 1,
                            "{name}: cell ({ix},{iz}) is at step {a}, its neighbour \
                             ({},{}) at {b} — that is a {:.2} m riser and 0.28 m is already \
                             a wall",
                            ix + dx,
                            iz + dz,
                            (a - b).abs() as f32 * f.rise_m()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn f003_a_pin_stays_at_zero_and_paving_is_never_undercut() {
        // Both halves matter, and they are two different constraints. A generator that honours
        // `Pin` by flattening everything is useless; one that ignores `Floor` hangs a street
        // in the air over a valley it dug underneath.
        let f = field(|ix, iz| {
            if ix == 0 {
                CellRole::Pin
            } else if iz == 0 {
                CellRole::Floor
            } else {
                CellRole::Free
            }
        });
        for i in 0..24i32 {
            assert_eq!(f.step_at(0, i), 0, "the pinned column moved");
            assert!(f.step_at(i, 0) >= 0, "the paved row sank to {}", f.height_at(i, 0));
        }
        assert!(
            f.steps_used().len() >= 20,
            "only {} distinct heights in a 24 x 24 field — that is a terrace, not terrain",
            f.steps_used().len()
        );
    }

    #[test]
    fn f003_an_empty_amplitude_is_a_provably_flat_map() {
        // ⚠️ `graybox` is built with `amplitude_m: []` and eight `vector_aiming` tests plus
        // four `player` tests reason about `y = 0` on it. If this ever returns anything but
        // zero, those twelve start measuring a world they were never pinned to.
        let f = TerrainField::new(
            12,
            12,
            5.0,
            0.25,
            &[],
            &[],
            &Rng::new(7),
            0xF003_000E,
            |_, _| CellRole::Free,
        );
        assert_eq!(f.steps_used(), vec![0]);
        for iz in 0..12i32 {
            for ix in 0..12i32 {
                assert_eq!(f.height_at(ix, iz), 0.0);
            }
        }
    }

    #[test]
    fn f003_outside_the_grid_is_the_rim_cell_and_not_the_base_plane() {
        // The one behavioural change against the terraced field this replaced. Returning 0
        // outside meant every rim cell was one rise from the void, so the map's whole outer
        // ring was held at the base plane — 700 m of the district's edge that could carry no
        // relief at all, for a rule about somewhere nobody can stand.
        let f = field(|ix, iz| if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free });
        assert_eq!(f.step_at(-1, 4), f.step_at(0, 4));
        assert_eq!(f.step_at(24, 4), f.step_at(23, 4));
        assert_eq!(f.step_at(4, -7), f.step_at(4, 0));
    }

    #[test]
    fn f003_the_same_seed_yields_exactly_the_same_ground_and_another_seed_does_not() {
        // Same argument as the city itself: a terrain that differs between two machines is a
        // desync, and it surfaces on the most expensive day there is. The second half is what
        // makes `rng` and `stream` real parameters instead of decoration — and unlike the
        // terraced field, where exactly one cell of 256 cleared that bar (`FIND-101`), the
        // noise here reaches every free cell.
        let one = |ix: u32, iz: u32| if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free };
        assert_eq!(seeded(3405691582, one), seeded(3405691582, one));
        assert_ne!(seeded(3405691582, one), seeded(1, one));
    }

    #[test]
    fn f003_a_house_is_founded_on_the_lowest_ground_it_covers() {
        // `lowest_over` is what stops a house from standing on air on its downhill corner —
        // `FIND-134` §3B. The field is a slope here by construction: one pin in the corner and
        // nothing else, so height grows with the L1 distance from it.
        let f = field(|ix, iz| if ix == 0 && iz == 0 { CellRole::Pin } else { CellRole::Free });
        let low = f.lowest_over(4, 7, 4, 7);
        for iz in 4..=7 {
            for ix in 4..=7 {
                assert!(
                    f.height_at(ix, iz) >= low - 1e-6,
                    "cell ({ix},{iz}) at {:.2} m is below the foundation at {low:.2} m",
                    f.height_at(ix, iz)
                );
            }
        }
        assert_eq!(low, f.height_at(4, 4).min(f.height_at(7, 7)).min(low));
    }
}
