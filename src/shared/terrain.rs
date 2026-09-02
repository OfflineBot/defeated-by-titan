//! The ground of a district — **a continuous surface: one f32 height per grid corner.**
//!
//! Two rewrites, both his:
//!
//! * 2026-08-29, against the terraces: *„nicht verschiedene hardcoded stufen sondern wirklich
//!   terrain! und deutlich hoeher und niedriger als jetzt!"* — that deleted `levels x step_m`
//!   and built a height field quantised to a 0.25 m rise.
//! * 2026-09-02, against the quantised field: *„ok und die welt hat jetzt harte höhen. aber es
//!   soll smooth sein und mehr elevation! also richtiges terrain!"* — and that deletes the
//!   quantum. There is no `rise_m` any more, no integer `step`, no riser to walk up. The
//!   ground is a **triangle mesh over corner heights**, and the walkability question moved
//!   from "how tall is the riser" (FIND-214: 0.27 m climbs, 0.28 m is a wall) to "how steep
//!   is the slope": [`Terrain::max_grade`](crate) in m/m, shipped at 0.35 ≈ 19°, well under
//!   the ~50° a grounded player can hold.
//!
//! ## THE surface, singular — the contract everything else builds on
//!
//! The field stores heights at the **corners** of an `nx x nz` cell grid — `(nx+1) x (nz+1)`
//! values. Each cell is split into two triangles by the fixed diagonal
//! `(i, j) -> (i+1, j+1)`, and that triangulation **is** the ground:
//!
//! * the render mesh triangulates exactly these corners with exactly this diagonal,
//! * the collider is a trimesh over the same triangles,
//! * and [`TerrainField::height_at_m`] evaluates the same triangulation analytically.
//!
//! One surface from one [`TerrainField::corner_heights`] slice — an oracle that agreed with
//! the picture but not with the collider is the class of bug this contract exists to make
//! impossible.
//!
//! ## The envelope, and why the field cannot break the district
//!
//! The noise on its own is steeper than the grade budget and would bury a door or hang a quay
//! wall in the air. Every **cell** has one [`CellRole`], decided by what stands on it, and the
//! roles constrain the **corners** touching them:
//!
//! * a corner touching a [`CellRole::Pin`] or [`CellRole::Hole`] cell is **pinned**: `h == 0`
//!   exactly (a quay wall meets the water, a door meets its street, the spawn disc stays a
//!   disc);
//! * a corner touching a [`CellRole::Floor`] cell is **floored**: `h >= 0` (paving may be
//!   climbed over, never undercut);
//! * everything else is free, and that is where the relief lives.
//!
//! With `budget = max_grade * cell_m` the two bounds are exact L1 cones:
//!
//! ```text
//! hi(c) =  budget * (L1 distance in corners to the nearest pinned corner)
//! lo(c) = -budget * (L1 distance in corners to the nearest pinned-or-floored corner)
//! ```
//!
//! Both are budget-Lipschitz by construction and `lo <= 0 <= hi`, so the system is always
//! feasible. The noise is clamped between them and then lowered — never raised — by a chamfer
//! descent until **no two corners joined by a grid edge differ by more than `budget`**. That
//! fixed point is the invariant this type promises, and `world::map` asserts it per map
//! rather than trusting it.
//!
//! ## What the shape actually comes from
//!
//! The octave amplitudes are **unitless**; metres enter once, through `elevation_m`: octave
//! `i` contributes `elevation_m * amplitude[i]` metres at wavelength `wavelength_m[i]`. So
//! "more elevation" is ONE number in `maps.ron`, and the octave mix is a shape, not a size.
//! Over most of a dense map the envelope is what binds — the noise decides *where* the high
//! ground is, the budget decides how fast you get there.
//!
//! No Bevy, no `data`, no side effects: `shared` is free for every domain, and a field that
//! can be built without an app is a field `tests/world.rs` can measure corner by corner.

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
    /// No ground at all — the canal. `world::map` cuts these cells out of the trimesh.
    Hole,
}

/// The ground of one map: **f32 heights at the `(nx+1) x (nz+1)` corners** of an `nx x nz`
/// cell grid, in metres over the base plane.
///
/// Built once in `world::map::plan_terrain` and then only read. Corner and cell coordinates
/// are **signed** on the way in and **clamped** to the rim: outside the grid is the nearest
/// corner (or cell) inside it, not zero — the map's own edge carries relief instead of being
/// pulled back to the base plane by a rule about the void.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainField {
    nx: u32,
    nz: u32,
    cell_m: f32,
    /// Row-major over the **corner** grid: `iz * (nx + 1) + ix`, `ix in 0..=nx`,
    /// `iz in 0..=nz`. A `Vec` and not a map: iteration order is part of the city and a
    /// `HashMap` here would be a desync (`world::map`).
    corner: Vec<f32>,
    /// Row-major over the **cell** grid: `iz * nx + ix`.
    hole: Vec<bool>,
}

impl TerrainField {
    /// A provably flat field — every corner at 0, no holes.
    ///
    /// ⚠️ `graybox` is built with this and eight tests in `tests/vector_aiming.rs` plus four
    /// in `tests/player.rs` reason about `y = 0` on it. A map says it wants this by writing
    /// `amplitude: []` and `elevation_m: 0.0`, which is a statement and not a missing key
    /// (`docs/conventions.md` §4).
    pub fn flat(nx: u32, nz: u32, cell_m: f32) -> Self {
        Self {
            nx,
            nz,
            cell_m,
            corner: vec![0.0; ((nx as usize) + 1) * ((nz as usize) + 1)],
            hole: vec![false; (nx as usize) * (nz as usize)],
        }
    }

    /// The whole generator.
    ///
    /// `amplitude[i]` is unitless, `elevation_m * amplitude[i]` is octave `i`'s amplitude in
    /// metres at `wavelength_m[i]`; the shorter wavelengths break up what the longer ones lay
    /// down. `role(ix, iz)` is asked exactly once per **cell**.
    ///
    /// Four passes, and the order is the argument:
    /// 1. the raw noise, in metres, sampled at every **corner** (corners sit on the grid
    ///    lines, so the sample points are `ix * cell_m`, not cell centres);
    /// 2. the corner constraint flags out of the cell roles, and the two L1 distance
    ///    transforms that give `hi` and `lo`;
    /// 3. the clamp into `[lo, hi]` — after which the field respects every pin, every floor
    ///    and every hole, but is still too steep;
    /// 4. the descent `h <- min(h, min(neighbours) + budget)` with
    ///    `budget = max_grade * cell_m`, which only ever lowers and whose fixed point is
    ///    exactly "no grid edge steeper than `max_grade`".
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nx: u32,
        nz: u32,
        cell_m: f32,
        max_grade: f32,
        elevation_m: f32,
        amplitude: &[f32],
        wavelength_m: &[f32],
        rng: &Rng,
        stream: u64,
        role: impl Fn(u32, u32) -> CellRole,
    ) -> Self {
        let cells = (nx as usize) * (nz as usize);
        // `elevation_m: 0.0` and `amplitude: []` are both statements of flatness (`graybox`
        // writes both); a non-positive grade or cell has nothing meaningful to generate.
        if cells == 0
            || cell_m <= 0.0
            || max_grade <= 0.0
            || elevation_m <= 0.0
            || amplitude.is_empty()
        {
            return Self::flat(nx, nz, cell_m);
        }

        let mut roles = Vec::with_capacity(cells);
        for iz in 0..nz {
            for ix in 0..nx {
                roles.push(role(ix, iz));
            }
        }

        let (cw, ch) = (nx as usize + 1, nz as usize + 1); // corner grid
        let corners = cw * ch;
        let budget = max_grade * cell_m;

        // 1 · the raw noise, sampled at every corner. Corners sit on the grid lines, so the
        // lattice coordinate is `ix * cell_m / wavelength` exactly — no half-cell offset.
        let mut h = vec![0.0f32; corners];
        for (octave, (amp, wave)) in amplitude.iter().zip(wavelength_m.iter()).enumerate() {
            let amp_m = elevation_m * *amp;
            if *wave <= 0.0 || amp_m == 0.0 {
                continue;
            }
            for iz in 0..ch {
                for ix in 0..cw {
                    let x = ix as f32 * cell_m / *wave;
                    let z = iz as f32 * cell_m / *wave;
                    h[iz * cw + ix] += amp_m * value_noise(x, z, octave as u64, rng, stream);
                }
            }
        }

        // 2 · which corners are constrained, out of the cell roles. A corner touches up to
        // four cells; one Pin or Hole among them pins it, one Floor among them floors it.
        let cell_role = |ix: usize, iz: usize| roles[iz * nx as usize + ix];
        let mut pinned = vec![false; corners];
        let mut floored = vec![false; corners];
        for iz in 0..ch {
            for ix in 0..cw {
                let mut pin = false;
                let mut floor = false;
                // the cells (ix-1..ix, iz-1..iz) that share this corner, clipped to the grid
                for (cx, cz) in [
                    (ix.wrapping_sub(1), iz.wrapping_sub(1)),
                    (ix, iz.wrapping_sub(1)),
                    (ix.wrapping_sub(1), iz),
                    (ix, iz),
                ] {
                    if cx < nx as usize && cz < nz as usize {
                        match cell_role(cx, cz) {
                            CellRole::Pin | CellRole::Hole => pin = true,
                            CellRole::Floor => floor = true,
                            CellRole::Free => {}
                        }
                    }
                }
                pinned[iz * cw + ix] = pin;
                floored[iz * cw + ix] = floor;
            }
        }

        // The two envelopes. `hi` is a cone rising out of every pinned corner, `lo` a cone
        // falling away from every pinned AND every floored corner — a valley may not undercut
        // a street.
        let to_pin = l1_distance(cw as u32, ch as u32, |i| pinned[i]);
        let to_ground = l1_distance(cw as u32, ch as u32, |i| pinned[i] || floored[i]);

        // 3 · the clamp. `hi` is unbounded on a map with no pin at all, which is why the
        // distance transform reports `None` for "never reached" instead of a large number
        // that would silently become a ceiling.
        for i in 0..corners {
            if let Some(d) = to_pin[i] {
                h[i] = h[i].min(budget * d as f32);
            }
            if let Some(d) = to_ground[i] {
                h[i] = h[i].max(-budget * d as f32);
            }
        }

        // 4 · the descent. Two chamfer sweeps are exact for an L1 min-plus envelope — the
        // forward one carries every constraint down and right, the backward one up and left —
        // and the loop runs twice as a belt over those braces. It never violates `lo`:
        // `lo` is budget-Lipschitz, so `neighbour + budget >= lo(neighbour) + budget >= lo`.
        // The per-map confirmation lives in `world::map`, which can name the map in its
        // message; the in-crate proof (including the doctored-copy control that shows the
        // instrument firing) lives in the tests below.
        let idx = |ix: usize, iz: usize| iz * cw + ix;
        for _ in 0..2 {
            for iz in 0..ch {
                for ix in 0..cw {
                    let i = idx(ix, iz);
                    if ix > 0 {
                        h[i] = h[i].min(h[idx(ix - 1, iz)] + budget);
                    }
                    if iz > 0 {
                        h[i] = h[i].min(h[idx(ix, iz - 1)] + budget);
                    }
                }
            }
            for iz in (0..ch).rev() {
                for ix in (0..cw).rev() {
                    let i = idx(ix, iz);
                    if ix + 1 < cw {
                        h[i] = h[i].min(h[idx(ix + 1, iz)] + budget);
                    }
                    if iz + 1 < ch {
                        h[i] = h[i].min(h[idx(ix, iz + 1)] + budget);
                    }
                }
            }
        }

        let hole = roles.iter().map(|r| *r == CellRole::Hole).collect();
        Self { nx, nz, cell_m, corner: h, hole }
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

    /// The height of one **corner**, in metres — `ix in 0..=nx`, `iz in 0..=nz`, and
    /// **clamped to the rim**, so "the neighbour outside the grid" is an ordinary lookup and
    /// the map's edge is not pulled back to zero.
    pub fn corner_m(&self, ix: i32, iz: i32) -> f32 {
        if self.corner.is_empty() {
            return 0.0;
        }
        let ix = ix.clamp(0, self.nx as i32) as usize;
        let iz = iz.clamp(0, self.nz as i32) as usize;
        self.corner[iz * (self.nx as usize + 1) + ix]
    }

    /// Every corner height, row-major `iz * (nx + 1) + ix` — **the** slice the render mesh
    /// and the collider trimesh are built from. One surface, one source (module doc).
    pub fn corner_heights(&self) -> &[f32] {
        &self.corner
    }

    /// The height of THE surface at a point, in metres — `gx`/`gz` in **grid-local metres**
    /// (corner `(0,0)` sits at `(0.0, 0.0)`, corner `(nx,nz)` at `(nx*cell_m, nz*cell_m)`;
    /// `world::map` owns the world-to-grid origin shift). Points outside are clamped to the
    /// rim, like every other lookup here.
    ///
    /// **Triangle-exact:** each cell is two triangles split by the fixed diagonal
    /// `(i, j) -> (i+1, j+1)`; the lower-right triangle (`fx >= fz`) interpolates corners
    /// `(i,j) (i+1,j) (i+1,j+1)`, the upper-left one `(i,j) (i,j+1) (i+1,j+1)`. Any mesh
    /// built over [`Self::corner_heights`] MUST use the same diagonal, or the picture and
    /// this oracle are two different grounds.
    pub fn height_at_m(&self, gx: f32, gz: f32) -> f32 {
        if self.nx == 0 || self.nz == 0 || self.cell_m <= 0.0 {
            return 0.0;
        }
        let gx = gx.clamp(0.0, self.nx as f32 * self.cell_m);
        let gz = gz.clamp(0.0, self.nz as f32 * self.cell_m);
        let i = ((gx / self.cell_m).floor() as i32).clamp(0, self.nx as i32 - 1);
        let j = ((gz / self.cell_m).floor() as i32).clamp(0, self.nz as i32 - 1);
        let fx = gx / self.cell_m - i as f32;
        let fz = gz / self.cell_m - j as f32;
        let c00 = self.corner_m(i, j);
        let c10 = self.corner_m(i + 1, j);
        let c01 = self.corner_m(i, j + 1);
        let c11 = self.corner_m(i + 1, j + 1);
        if fx >= fz {
            c00 + fx * (c10 - c00) + fz * (c11 - c10)
        } else {
            c00 + fz * (c01 - c00) + fx * (c11 - c01)
        }
    }

    /// The **lowest** point of the surface over a rectangle of **cells**, in metres.
    ///
    /// The surface is piecewise linear over triangles whose vertices are the corners, so its
    /// minimum over the rect is the minimum over the rect's corners — cells `ix0..=ix1` span
    /// corners `ix0..=ix1+1`. Indices rim-clamp like everything else.
    ///
    /// This is what a house is founded on, and `min` rather than `max` is the whole point: a
    /// house set on the lowest ground it covers is cut into the slope on its high side, and a
    /// house set on the highest is a house standing on air on its low side. `FIND-134` §3B
    /// cost a round to the second one.
    pub fn lowest_over_m(&self, ix0: i32, ix1: i32, iz0: i32, iz1: i32) -> f32 {
        let mut lowest = f32::INFINITY;
        for iz in iz0..=iz1 + 1 {
            for ix in ix0..=ix1 + 1 {
                lowest = lowest.min(self.corner_m(ix, iz));
            }
        }
        if lowest == f32::INFINITY { 0.0 } else { lowest }
    }

    /// Is there no ground in this **cell** at all? The canal, and nothing else today.
    pub fn is_hole(&self, ix: i32, iz: i32) -> bool {
        if self.nx == 0 || self.nz == 0 {
            return false;
        }
        let ix = ix.clamp(0, self.nx as i32 - 1) as usize;
        let iz = iz.clamp(0, self.nz as i32 - 1) as usize;
        self.hole[iz * (self.nx as usize) + ix]
    }

    /// The height of the surface at every **cell centre**, in cell order — what a percentile
    /// over the ground is taken from. Every cell is the same size, so cell order **is** an
    /// area-weighted sample. The centre sits on the fixed diagonal, so its height is exactly
    /// the mean of the diagonal's two corners.
    pub fn heights_m(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity((self.nx as usize) * (self.nz as usize));
        for iz in 0..self.nz as i32 {
            for ix in 0..self.nx as i32 {
                out.push((self.corner_m(ix, iz) + self.corner_m(ix + 1, iz + 1)) * 0.5);
            }
        }
        out
    }

}

/// One octave of value noise: a lattice of `[-1, 1]` draws, smoothstepped between.
///
/// Deterministic out of `(seed, lattice point, octave)` — [`Rng`] is stateless, so this is a
/// pure function of the map's seed and nothing accumulates. `x` and `z` are in **lattice
/// units** (grid metres divided by the wavelength) and are always >= 0 here, so the floor
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

/// L1 distance in grid units to the nearest index the predicate holds for — `None` where
/// there is none at all.
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

    /// The shipped Ashgate numbers scaled to a fixture: 24 x 24 cells of 5 m, grade budget
    /// 0.35 * 5 = 1.75 m per edge, elevation 24 m over the unitless [1.0, 0.32] mix — the
    /// smallest grid in which a cone from a pin has room to reach its ceiling.
    const CELL_M: f32 = 5.0;
    const MAX_GRADE: f32 = 0.35;
    const ELEVATION_M: f32 = 24.0;

    fn field(role: impl Fn(u32, u32) -> CellRole) -> TerrainField {
        seeded(3405691582, ELEVATION_M, role)
    }

    fn seeded(seed: u64, elevation_m: f32, role: impl Fn(u32, u32) -> CellRole) -> TerrainField {
        TerrainField::new(
            24,
            24,
            CELL_M,
            MAX_GRADE,
            elevation_m,
            &[1.0, 0.32],
            &[110.0, 40.0],
            &Rng::new(seed),
            0xF003_000E,
            role,
        )
    }

    /// The instrument of the grade tests, factored out so a doctored copy can prove it fires:
    /// the worst `|delta| - budget` over every corner-grid edge, in metres. `<= 0` (up to
    /// float dust) is the invariant.
    fn worst_edge_excess_m(corner: &[f32], nx: usize, nz: usize, budget: f32) -> f32 {
        let (cw, ch) = (nx + 1, nz + 1);
        assert_eq!(corner.len(), cw * ch, "not a corner slice of this grid");
        let mut worst = f32::NEG_INFINITY;
        let mut edges = 0u32;
        for iz in 0..ch {
            for ix in 0..cw {
                let h = corner[iz * cw + ix];
                if ix + 1 < cw {
                    worst = worst.max((h - corner[iz * cw + ix + 1]).abs() - budget);
                    edges += 1;
                }
                if iz + 1 < ch {
                    worst = worst.max((h - corner[(iz + 1) * cw + ix]).abs() - budget);
                    edges += 1;
                }
            }
        }
        // Count what you skip (docs/lessons/fixtures.md): every interior edge visited, none
        // twice — 2 * cw * ch - cw - ch of them.
        assert_eq!(edges as usize, 2 * cw * ch - cw - ch, "the sweep skipped edges");
        worst
    }

    #[test]
    fn f003_no_grid_edge_is_steeper_than_max_grade_and_the_instrument_fires() {
        // ★ The invariant the whole feature stands on: with the quantum gone, `max_grade` is
        // the ONLY thing standing between "terrain" and "a wall with a texture" — red the
        // moment somebody drops the descent.
        //
        // ## What this sweep varies, and what it holds constant
        //
        // Varies: the role layout (four shapes below), and with it every distance transform
        // the code reads. Holds constant: `nx`, `nz`, `cell_m`, `max_grade`, the octaves and
        // the seed — none of which the invariant may depend on, because the descent's fixed
        // point is defined by `budget = max_grade * cell_m` alone. Skipped: nothing — the
        // helper counts its own edges, holes included (a hole cell pins its corners, and a
        // pinned corner is still a corner).
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
        let budget = MAX_GRADE * CELL_M;
        for (name, role) in layouts {
            let f = field(role);
            let excess = worst_edge_excess_m(f.corner_heights(), 24, 24, budget);
            assert!(
                excess <= 1e-4,
                "{name}: an edge exceeds the grade budget by {excess} m — the descent \
                 stopped short of its fixed point"
            );

            // The control: lift ONE corner of a scratch copy past the budget and watch the
            // same instrument fire. A sweep that cannot go red measures nothing
            // (docs/lessons/fixtures.md: delete the thing you are measuring).
            let mut doctored = f.corner_heights().to_vec();
            doctored[12 * 25 + 12] += 2.0 * budget;
            let fired = worst_edge_excess_m(&doctored, 24, 24, budget);
            assert!(
                fired >= budget - 1e-4,
                "{name}: the instrument did not fire on a doctored corner (excess {fired})"
            );
        }
    }

    #[test]
    fn f003_pinned_corners_are_exactly_zero_and_paving_is_never_undercut() {
        // Both halves matter, and they are two different constraints. A generator that
        // honours `Pin` by flattening everything is useless; one that ignores `Floor` hangs a
        // street in the air over a valley it dug underneath. Pins are asserted EXACTLY — the
        // clamp sets `min(hi=0) . max(lo=0)` and the descent provably cannot go below `lo` —
        // because the spawn disc and the titan ring reason about `y = 0`, not `y ~ 0`.
        let f = field(|ix, iz| {
            if ix == 0 {
                CellRole::Pin
            } else if iz == 0 {
                CellRole::Floor
            } else {
                CellRole::Free
            }
        });
        for i in 0..=24i32 {
            // every corner touching the pinned cell column ix == 0: corners ix in {0, 1}
            assert_eq!(f.corner_m(0, i), 0.0, "pinned corner (0,{i}) moved");
            assert_eq!(f.corner_m(1, i), 0.0, "pinned corner (1,{i}) moved");
            // every corner touching the floored cell row iz == 0: corners iz in {0, 1}
            for iz in 0..=1i32 {
                assert!(
                    f.corner_m(i, iz) >= 0.0,
                    "floored corner ({i},{iz}) sank to {} m — a hanging street",
                    f.corner_m(i, iz)
                );
            }
        }
        // And the field is still terrain, not a plate: far from the pin the relief is real.
        let relief = relief_m(&f);
        assert!(relief > 1.0, "relief is only {relief} m — the envelope flattened everything");
    }

    #[test]
    fn f003_two_octaves_disagree_and_deleting_one_moves_the_ground() {
        // The n = 2 case first, with disagreeing elements (docs/lessons/fixtures.md): the
        // second octave must be a real contributor, or `amplitude: [a, b]` is decoration.
        // Deleting the thing we measure: the same field without its short octave differs.
        let one_pin = |ix: u32, iz: u32| {
            if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free }
        };
        let both = TerrainField::new(
            24, 24, CELL_M, MAX_GRADE, ELEVATION_M,
            &[1.0, 0.32], &[110.0, 40.0],
            &Rng::new(3405691582), 0xF003_000E, one_pin,
        );
        let long_only = TerrainField::new(
            24, 24, CELL_M, MAX_GRADE, ELEVATION_M,
            &[1.0], &[110.0],
            &Rng::new(3405691582), 0xF003_000E, one_pin,
        );
        assert_ne!(
            both, long_only,
            "the 40 m octave changed nothing — amplitude[1] is decoration, not terrain"
        );
    }

    #[test]
    fn f003_an_empty_amplitude_is_a_provably_flat_map() {
        // ⚠️ `graybox` is built with `amplitude: []` + `elevation_m: 0.0` and eight
        // `vector_aiming` tests plus four `player` tests reason about `y = 0` on it. If this
        // ever returns anything but zero, those twelve start measuring a world they were
        // never pinned to. Both statements of flatness are tested — either alone must do.
        let empty = TerrainField::new(
            12, 12, CELL_M, MAX_GRADE, ELEVATION_M, &[], &[],
            &Rng::new(7), 0xF003_000E, |_, _| CellRole::Free,
        );
        let zero_elevation = TerrainField::new(
            12, 12, CELL_M, MAX_GRADE, 0.0, &[1.0], &[110.0],
            &Rng::new(7), 0xF003_000E, |_, _| CellRole::Free,
        );
        for f in [&empty, &zero_elevation, &TerrainField::flat(12, 12, CELL_M)] {
            assert!(f.corner_heights().iter().all(|h| *h == 0.0), "a corner left the plane");
            assert_eq!(f.corner_heights().len(), 13 * 13);
            assert_eq!(f.height_at_m(31.4, 27.1), 0.0);
        }
    }

    #[test]
    fn f003_outside_the_grid_is_the_rim_corner_and_not_the_base_plane() {
        // Returning 0 outside would hold the map's whole outer ring at the base plane — 700 m
        // of the district's edge that could carry no relief at all, for a rule about
        // somewhere nobody can stand. Corner indices run 0..=nx, and past either end the rim
        // corner answers.
        let f = field(|ix, iz| if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free });
        assert_eq!(f.corner_m(-1, 4), f.corner_m(0, 4));
        assert_eq!(f.corner_m(25, 4), f.corner_m(24, 4));
        assert_eq!(f.corner_m(4, -7), f.corner_m(4, 0));
        assert_eq!(f.corner_m(4, 99), f.corner_m(4, 24));
        // and the surface oracle clamps the same way
        assert_eq!(f.height_at_m(-3.0, 20.0), f.height_at_m(0.0, 20.0));
        assert_eq!(f.height_at_m(9999.0, 20.0), f.height_at_m(24.0 * CELL_M, 20.0));
    }

    #[test]
    fn f003_the_same_seed_yields_exactly_the_same_ground_and_another_seed_does_not() {
        // Same argument as the city itself: a terrain that differs between two machines is a
        // desync, and it surfaces on the most expensive day there is. Bit-identical, not
        // approximately — `PartialEq` on the struct compares every corner. The second half is
        // what makes `rng` and `stream` real parameters instead of decoration.
        let one = |ix: u32, iz: u32| {
            if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free }
        };
        assert_eq!(seeded(3405691582, ELEVATION_M, one), seeded(3405691582, ELEVATION_M, one));
        assert_ne!(seeded(3405691582, ELEVATION_M, one), seeded(1, ELEVATION_M, one));
    }

    /// max - min over the corners: the one number "mehr elevation" is about.
    fn relief_m(f: &TerrainField) -> f32 {
        let hs = f.corner_heights();
        let hi = hs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let lo = hs.iter().cloned().fold(f32::INFINITY, f32::min);
        hi - lo
    }

    #[test]
    fn f003_doubling_elevation_m_yields_visibly_more_relief() {
        // `elevation_m` is THE knob the user's "mehr elevation!" turns, so it has to move the
        // measured relief, not just a coefficient nobody can see. Strictly more — the
        // envelope may bind part of the way, but a knob the envelope swallows whole would be
        // a lie in the RON.
        let one = |ix: u32, iz: u32| {
            if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free }
        };
        let r1 = relief_m(&seeded(3405691582, ELEVATION_M, one));
        let r2 = relief_m(&seeded(3405691582, 2.0 * ELEVATION_M, one));
        assert!(
            r2 > r1 + 1.0,
            "elevation_m x2 moved relief only {r1} -> {r2} m — the knob is decoration"
        );
        assert!(r1 > 2.0, "relief at elevation_m {ELEVATION_M} is a plate: {r1} m");
    }

    #[test]
    fn f003_height_at_m_evaluates_the_fixed_diagonal_triangulation() {
        // THE surface contract (module doc): the oracle and any mesh over `corner_heights()`
        // are one ground only if both split every cell along (i,j)->(i+1,j+1). This test
        // nails the formula to that diagonal on a cell whose two triangles visibly disagree.
        let f = field(|ix, iz| if ix == 12 && iz == 12 { CellRole::Pin } else { CellRole::Free });

        // find a cell where the two diagonals disagree hard: |c01 + c10 - c00 - c11| large
        let mut cell = None;
        let mut best = 0.25; // metres of disagreement — enough to dwarf float dust
        for iz in 0..24i32 {
            for ix in 0..24i32 {
                let d = (f.corner_m(ix, iz + 1) + f.corner_m(ix + 1, iz)
                    - f.corner_m(ix, iz)
                    - f.corner_m(ix + 1, iz + 1))
                    .abs();
                if d > best {
                    best = d;
                    cell = Some((ix, iz));
                }
            }
        }
        let (i, j) = cell.expect("no cell with disagreeing triangles — fixture too flat to test");
        let (c00, c10, c01, c11) =
            (f.corner_m(i, j), f.corner_m(i + 1, j), f.corner_m(i, j + 1), f.corner_m(i + 1, j + 1));

        // 1 · corners are reproduced (fx = fz = 0 is exact by construction)
        assert_eq!(f.height_at_m(i as f32 * CELL_M, j as f32 * CELL_M), c00);

        // 2 · the cell centre lies ON the diagonal: exactly the mean of ITS two corners,
        //     never of the other pair — this is where a flipped diagonal shows first.
        let centre = f.height_at_m((i as f32 + 0.5) * CELL_M, (j as f32 + 0.5) * CELL_M);
        assert!((centre - (c00 + c11) * 0.5).abs() < 1e-4, "centre off the fixed diagonal");
        assert!(
            (centre - (c01 + c10) * 0.5).abs() > best * 0.5 - 1e-4,
            "centre agrees with the WRONG diagonal — the split is flipped"
        );

        // 3 · one sample deep in each triangle against the plane through its three corners
        let lower = f.height_at_m((i as f32 + 0.7) * CELL_M, (j as f32 + 0.2) * CELL_M);
        assert!((lower - (c00 + 0.7 * (c10 - c00) + 0.2 * (c11 - c10))).abs() < 1e-3);
        let upper = f.height_at_m((i as f32 + 0.2) * CELL_M, (j as f32 + 0.7) * CELL_M);
        assert!((upper - (c00 + 0.7 * (c01 - c00) + 0.2 * (c11 - c01))).abs() < 1e-3);

        // 4 · the boundary itself, at +-1 ULP (docs/lessons/fixtures.md): the surface is
        //     continuous across the diagonal — approaching from either triangle agrees.
        let (gx, gz) = ((i as f32 + 0.5) * CELL_M, (j as f32 + 0.5) * CELL_M);
        let below = f.height_at_m(gx.next_down(), gz);
        let above = f.height_at_m(gx.next_up(), gz);
        assert!((below - centre).abs() < 1e-3 && (above - centre).abs() < 1e-3,
            "the surface tears along the diagonal: {below} | {centre} | {above}");
        // and across a cell boundary
        let edge = (i as f32) * CELL_M;
        assert!(
            (f.height_at_m(edge.next_down(), gz) - f.height_at_m(edge, gz)).abs() < 1e-3,
            "the surface tears at the cell boundary x = {edge}"
        );
    }

    #[test]
    fn f003_a_house_is_founded_on_the_lowest_corner_it_covers() {
        // `lowest_over_m` is what stops a house from standing on air on its downhill corner —
        // `FIND-134` §3B. Piecewise-linear over the corners means the corner minimum IS the
        // surface minimum; the sample sweep behind it is the proof the formula argument holds
        // on a real field.
        let f = field(|ix, iz| if ix == 0 && iz == 0 { CellRole::Pin } else { CellRole::Free });
        let low = f.lowest_over_m(4, 7, 4, 7);
        // the corner minimum, recomputed independently of the method under test
        let mut by_hand = f32::INFINITY;
        for iz in 4..=8i32 {
            for ix in 4..=8i32 {
                by_hand = by_hand.min(f.corner_m(ix, iz));
            }
        }
        assert_eq!(low, by_hand);
        // and no interior sample of the surface dips below it
        for iz in 0..=30 {
            for ix in 0..=30 {
                let gx = (4.0 + 4.0 * ix as f32 / 30.0) * CELL_M;
                let gz = (4.0 + 4.0 * iz as f32 / 30.0) * CELL_M;
                assert!(
                    f.height_at_m(gx, gz) >= low - 1e-4,
                    "surface at ({gx},{gz}) is {} m, below the foundation {low} m",
                    f.height_at_m(gx, gz)
                );
            }
        }
    }

    #[test]
    fn f003_holes_are_cells_and_their_rims_are_pinned_to_the_waterline() {
        let f = field(|ix, _| match ix {
            10 => CellRole::Hole,
            9 | 11 => CellRole::Pin,
            _ => CellRole::Free,
        });
        for iz in 0..24i32 {
            assert!(f.is_hole(10, iz), "the canal cell (10,{iz}) is not a hole");
            assert!(!f.is_hole(9, iz) && !f.is_hole(11, iz), "a quay cell became a hole");
            // every corner of a hole cell is pinned: the ground meets the water at 0 exactly
            for ix in 10..=11i32 {
                for dz in 0..=1i32 {
                    assert_eq!(
                        f.corner_m(ix, iz + dz),
                        0.0,
                        "canal rim corner ({ix},{}) left the waterline",
                        iz + dz
                    );
                }
            }
        }
    }

}
