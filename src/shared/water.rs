//! Water — **the volume, and the one number that says how deep you are in it.**
//!
//! The user, asked on 2026-08-29 what happens when a body meets the river, answered three
//! things and this file carries two of them:
//!
//! > *„Man schwimmt / wird langsam."* — water is **terrain with a cost**, not a killing plane
//! > and not a wall. You fall in, you lose speed and gas, and you work your way out.
//!
//! > *„Nein — Wasser haelt keinen Haken."* — and that half does **not** live here. It lives in
//! > [`crate::vector::hookable`], as the first [`SurfaceKind`](crate::vector::hookable::SurfaceKind)
//! > the `Q-078` switch turns off. There is deliberately no second hookability flag in this
//! > file: one question, one mechanism.
//!
//! ## Why these two types are here and not in `world`
//!
//! `world` spawns the water and `player` has to read it, and `player -> world` is not an edge
//! in the allow list of `docs/architecture.md` — nor should it be for this. The same split
//! `shared::Block` already makes: the domain that *builds* a thing and the domain that *acts*
//! on it talk through a component (`docs/architecture.md`, and `shared/geometry.rs`'s header
//! makes the identical argument for `render`).
//!
//! ⚠️ **[`WaterVolume`] carries no `Body`, no `Block`, no `Collider` and no `RigidBody`, and
//! all four absences are decisions:**
//!
//! * **no `Collider`** — a collider you can pass through is a `Sensor`, and a sensor is
//!   returned by `SpatialQuery::cast_ray` like anything else. The hook ray would then stop at
//!   the surface, and a hook fired **from inside** the water would answer at distance 0
//!   (avian clamps `tmin` to 0 for an origin inside a shape,
//!   `bevy_math-0.19.0/src/bounding/raycast3d.rs:64`) — i.e. the one shot the player needs to
//!   get out would be the one shot that cannot work. Water is transparent to the rope ray
//!   because it is not there at all.
//! * **no `RigidBody`** — nothing to attach it to, and `tests/player.rs::
//!   t007_every_world_collider_carries_a_rigid_body` asks about colliders, of which there are
//!   none here.
//! * **no `Block`** — `Block` means *a cuboid of the city*, and `tests/world.rs::
//!   f003_the_city_comes_from_the_file_and_not_twice` counts them against
//!   `world::map::plan_blocks`. Four supply stations already made that count 2875 against 2871
//!   once (`src/world/supply.rs`); the river would make it worse. So water carries its own
//!   marker and `render` gives it its own builder — the precedent is exactly one file old.
//! * **no `Body`** — `world::index::maintain_index` would give it a `BodyId` and put it in the
//!   `SpatialIndex`, whose length `tests/world.rs::f003_every_body_lands_in_the_index` compares
//!   against the same plan. And it would buy nothing today: `SpatialIndex::cast_ray` and
//!   `::aabb_overlaps` are both **still stubs** that answer nothing
//!   (`src/shared/spatial.rs`, "filled in by job R"), so a water body in the index is a body
//!   nobody can find. `docs/FINDINGS.md` FIND-216.

use bevy::prelude::*;

/// **A body of water.** An axis-aligned box; the entity's `Transform` is its centre.
///
/// Its top face is the surface. Everything about being *in* it is decided against this box
/// and nothing else, so that one shape answers "is he wet", "how deep" and "where is the
/// surface" — three questions that must not be allowed to disagree.
///
/// The colour rides along because `render::build_water_meshes` needs it and has no other way
/// to ask: `render` may not know `world`, and a palette key would put the river's hue in
/// `maps.ron`, which is a different file with a different owner.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WaterVolume {
    /// Half edge length in metres — the form `Aabb3d::new(centre, half)` takes, and the same
    /// form `shared::Body` uses, so that nobody has to remember which of the two this is.
    pub half_size_m: Vec3,
    /// Base colour as linear RGB. **None of the three signal colours** (cyan, amber, crimson)
    /// — those are gameplay and nothing else (`docs/conventions.md` §3).
    pub color: [f32; 3],
}

impl WaterVolume {
    /// The y of the surface, given the volume's centre.
    pub fn surface_y_m(&self, centre_m: Vec3) -> f32 {
        centre_m.y + self.half_size_m.y
    }

    /// **How deep a point is in this water**, in metres, or `None` when the point is not in it
    /// at all.
    ///
    /// The point is the body's **origin**, which sits between the feet
    /// (`docs/conventions.md`) — so "in the water" means *his feet are under the surface*,
    /// which is what makes a body float with his head out instead of his eyes.
    ///
    /// ⚠️ **The test is strict on every one of the six faces**, and the surface is one of
    /// them: a point at exactly `surface_y_m` is **out**, depth `None`. That is a real
    /// boundary and not a rounding question — `tests/player.rs` samples it on purpose, because
    /// a fence test that skips the samples lying on its own line is the shape that hid a bug
    /// for four rounds (`CLAUDE.md` rule 5). It costs nothing here: the equilibrium a floating
    /// body settles at is `gravity * surface_band_m / buoyancy_m_s2` **below** the surface, not
    /// on it, so the line is crossed and never rested on.
    pub fn depth_m(&self, centre_m: Vec3, point_m: Vec3) -> Option<f32> {
        let min = centre_m - self.half_size_m;
        let max = centre_m + self.half_size_m;
        let inside = point_m.x > min.x
            && point_m.x < max.x
            && point_m.z > min.z
            && point_m.z < max.z
            && point_m.y > min.y
            && point_m.y < max.y;
        inside.then(|| max.y - point_m.y)
    }
}

/// **How deep this body is in water right now** — 0.0 means dry.
///
/// One writer: `player::swim::swim_in_water`. Readers: `vector::gas` (the gear costs more
/// while it is working under water) and, one day, `hud` and `sound`.
///
/// It is a component on the player and never a resource, for the reason
/// `docs/multiplayer.md` rule 3 gives: two players in one session are wet independently, and
/// a resource would make one of them decide for the other.
///
/// ⚠️ **Every player carries it from tick 1**, dry (`player::spawn_player_with_id`). A system
/// that filters on a missing component silently skips the player, and a player who is skipped
/// by the gas multiplier is a player for whom water is free.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Submerged {
    /// Metres between the surface and the body's origin. `0.0` is dry — **not** "exactly at
    /// the surface", which is the same state and is written the same way.
    pub depth_m: f32,
}

impl Default for Submerged {
    fn default() -> Self {
        Self { depth_m: 0.0 }
    }
}

impl Submerged {
    /// Is this body in the water at all? The one predicate; nobody compares `depth_m` to zero
    /// themselves, so that "wet" has one spelling.
    pub fn wet(&self) -> bool {
        self.depth_m > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> (WaterVolume, Vec3) {
        // 10 m wide, 4 m deep, 100 m long, surface at y = -0.6 — the shape of the Ashgate
        // channel, at a hundredth of its length.
        (
            WaterVolume { half_size_m: Vec3::new(5.0, 1.7, 50.0), color: [0.0, 0.2, 0.3] },
            Vec3::new(-70.0, -2.3, 0.0),
        )
    }

    #[test]
    fn f003_the_surface_is_the_top_of_the_box_and_nothing_else() {
        // ⚠️ A tolerance and not `assert_eq`, and the first version of this test was the
        // reason: `-2.3 + 1.7` is `-0.5999999` in f32 and the literal `-0.6` is a different
        // number. Comparing a computed surface against a typed one measures the float format,
        // not the rule — and every test below therefore asks the code where its own surface is
        // instead of assuming it knows.
        let (water, centre) = pool();
        assert!((water.surface_y_m(centre) - -0.6).abs() < 1e-5);
    }

    #[test]
    fn f003_a_point_on_the_surface_is_out_of_the_water_and_a_hair_under_it_is_in() {
        // The boundary, sampled from both sides and ON the line — the class a fence test is
        // most tempted to `continue` past (`CLAUDE.md` rule 5, the 2026-08-29 shape). The line
        // is taken from `surface_y_m`, i.e. from the same arithmetic `depth_m` uses, so that
        // "on the line" really is on the line and not one ULP off it.
        let (water, centre) = pool();
        let surface = water.surface_y_m(centre);
        assert_eq!(
            water.depth_m(centre, Vec3::new(-70.0, surface, 0.0)),
            None,
            "a point ON the surface counts as in"
        );
        let under = Vec3::new(-70.0, surface - 1e-4, 0.0);
        let depth = water.depth_m(centre, under).expect("one tenth of a millimetre under");
        assert!(depth > 0.0 && depth < 1e-3, "depth {depth} at 0.1 mm of submersion");
    }

    #[test]
    fn f003_all_six_faces_are_strict_so_the_bank_is_not_water() {
        let (water, centre) = pool();
        let min = centre - water.half_size_m;
        let max = centre + water.half_size_m;
        // The two quay faces, the bed, the two ends and the surface — each taken from the
        // box's own arithmetic, for the reason the test above spells out.
        for out in [
            Vec3::new(min.x, centre.y, centre.z),
            Vec3::new(max.x, centre.y, centre.z),
            Vec3::new(centre.x, min.y, centre.z),
            Vec3::new(centre.x, centre.y, min.z),
            Vec3::new(centre.x, centre.y, max.z),
            Vec3::new(centre.x, max.y, centre.z),
        ] {
            assert_eq!(water.depth_m(centre, out), None, "{out:?} is on a face and counts as in");
        }
        // And the middle really is in, or the six lines above prove nothing.
        let middle = water.depth_m(centre, centre).expect("the centre of the pool is water");
        assert!((middle - water.half_size_m.y).abs() < 1e-5, "depth at the centre is {middle}");
    }

    #[test]
    fn f003_dry_is_the_default_and_wet_is_one_spelling() {
        assert_eq!(Submerged::default().depth_m, 0.0);
        assert!(!Submerged::default().wet());
        assert!(!Submerged { depth_m: 0.0 }.wet(), "exactly at the surface is dry");
        assert!(Submerged { depth_m: 0.01 }.wet());
    }
}
