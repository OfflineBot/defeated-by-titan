//! What stands in the world — as **data**, not as a mesh.
//!
//! `world` spawns blocks, `render` turns them into meshes. That way `render` does not have
//! to know the domain `world`, and `world` does not have to understand anything about
//! rendering: the two talk through a component, not through a call (`docs/architecture.md`).
//!
//! And it is the same split multiplayer will need later: the simulation knows blocks and
//! anchor surfaces, the presentation knows triangles (§6 rule 1).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A cuboid in the world. Size in **meters**, origin at its center.
///
/// Low poly, flat color surfaces — the bible's style holds for placeholders too
/// (`docs/conventions.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub size: Vec3,
    /// Base color as linear RGB. **None of the three signal colors** (cyan, amber, crimson)
    /// — those are reserved for gameplay and nothing else.
    pub color: [f32; 3],
}

/// This surface is **anchorable**.
///
/// The translated `CollectionService` tag `AnchorSurface` of the reference work
/// (`docs/architecture.md`). Only tagged surfaces are anchorable (`F-003`) — that prevents
/// physics exploits, keeps level design steerable, and defines a map's traversal difficulty
/// through the density of those surfaces.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnchorSurface;

/// The ground. As long as there is no spatial index, it is the only collision.
///
/// ⚠️ **Transitional.** As soon as `world::map` builds the map from `assets/data/maps.ron`,
/// the ground is a [`Body`] like any other and this type goes away — together with
/// `ground_y = 0.0` in `src/player/mod.rs`. Both die in the same commit as the filled index,
/// not before: with no index and no ground the player falls for 600 ticks.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    pub height_m: f32,
}

/// What a body counts as — a bit pattern like [`Buttons`](super::intent::Buttons).
///
/// Fixed size, `serde` without an extra feature, fits over a wire. One `bool` per purpose
/// would be the same thing with more fields and without the option of checking several
/// purposes in a single query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyMask(pub u32);

impl BodyMask {
    pub const NONE: BodyMask = BodyMask(0);
    /// Stops a body — collision (stage 2, `F-013`).
    pub const SOLID: BodyMask = BodyMask(1 << 0);
    /// Anchorable (`F-003`), derived from the marker [`AnchorSurface`].
    pub const ANCHORABLE: BodyMask = BodyMask(1 << 1);
    /// Takes blade hits (`blades`, `combat`).
    pub const SLICEABLE: BodyMask = BodyMask(1 << 2);

    pub fn contains(self, other: BodyMask) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    pub fn with(self, other: BodyMask) -> BodyMask {
        BodyMask(self.0 | other.0)
    }
}

/// The **axis-aligned hull** of a body in the world — house, roof, ground, later a titan's
/// shoulder.
///
/// The center is the entity's world position; `world::index` reads it from the
/// `GlobalTransform`, so that a child body (from `F-029` on) does not write its local
/// position into the index. Today the two are identical.
///
/// **Immutable after spawning** — an immutable component has no write conflict. An
/// axis-aligned cuboid **is** exactly its AABB; a rotated `Cuboid`, by contrast, yields the
/// enclosing hull, which is too large
/// (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), and then the hook
/// visibly catches in mid-air. **Blocks are not rotated.**
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Body {
    /// Half edge length in meters — the same form `Aabb3d::new(center, half_size)` takes
    /// (`bevy_math-0.19.0/src/bounding/bounded3d/mod.rs:66`).
    pub half_size_m: Vec3,
    pub mask: BodyMask,
}

/// The player's hull: **center and half edge length**, from height and position.
///
/// A model's origin sits **between the feet** (`docs/conventions.md`) — so the obvious line
/// `Aabb3d::new(translation, (r, height/2, r))` sinks the player 0.9 m into the ground, and
/// `scripts/t007-first-run.txt` (`assert height < 0.5`) fails. That is why these two lines
/// stand **once** here and not in three systems.
pub fn player_aabb(translation_m: Vec3, height_m: f32, radius_m: f32) -> (Vec3, Vec3) {
    let half_height = height_m * 0.5;
    (
        translation_m + Vec3::Y * half_height,
        Vec3::new(radius_m, half_height, radius_m),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f003_a_mask_keeps_several_purposes_apart() {
        let m = BodyMask::SOLID.with(BodyMask::ANCHORABLE);
        assert!(m.contains(BodyMask::SOLID));
        assert!(m.contains(BodyMask::ANCHORABLE));
        assert!(!m.contains(BodyMask::SLICEABLE));
        // "nothing" never holds — otherwise every query for the empty mask would come back
        // true.
        assert!(!m.contains(BodyMask::NONE));
        assert!(!BodyMask::NONE.contains(BodyMask::SOLID));
    }

    #[test]
    fn stage2_the_player_hull_stands_on_the_ground_not_in_it() {
        // Origin between the feet: the bottom edge of the hull sits exactly at y = 0 when
        // the player stands at y = 0.
        let (center, half) = player_aabb(Vec3::new(3.0, 0.0, -4.0), 1.8, 0.35);
        assert!((center.y - 0.9).abs() < 1e-6, "center {center:?}");
        assert!((half - Vec3::new(0.35, 0.9, 0.35)).length() < 1e-6, "half {half:?}");
        let bottom = center.y - half.y;
        assert!(bottom.abs() < 1e-6, "bottom edge at {bottom} instead of 0");
        assert!((center.x - 3.0).abs() < 1e-6 && (center.z + 4.0).abs() < 1e-6);
    }
}
