//! `water.ron` — **the river, as data.** The volumes, and what being in one does to a body.
//!
//! Its own file and not a block in `maps.ron`, for two reasons and the second one is the one
//! that will still be true next month:
//!
//! 1. `maps.ron` describes **cuboids of the city** — things with a collider that stop you.
//!    Water stops nothing; it is a region, and mixing it into `blocks` would mean every guard
//!    that counts blocks (`tests/world.rs`, and the `SpatialIndex` length beside it) has to
//!    learn about a row that is not a block.
//! 2. The **swim tuning is not per map.** How fast water slows a body is a property of water,
//!    the way `game.ron: gravity_m_s2` is a property of the world — and a per-map copy of it
//!    is four copies of one number waiting to disagree.
//!
//! ⚠️ **No `serde(default)` anywhere in this file** (`CLAUDE.md` rule 2): a missing key has to
//! crash on load. Two of the numbers below are load-bearing in a way that a silent `0.0` would
//! hide completely — a `drag_per_s` of 0 is water that does not slow you, and a
//! `buoyancy_m_s2` of 0 is water you sink to the bottom of and never leave.

use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Everything in `water.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterData {
    pub swim: SwimTuning,
    /// Per map name, exactly as `maps.ron: maps` spells it. A map with no water is a map with
    /// **no key here** — not an empty list and not a missing file
    /// (`world::water::build_water` treats both the same and says so).
    pub volumes: BTreeMap<String, Vec<WaterVolumeSpec>>,
}

/// What water does to a body in it. **All of it per second or per second squared** — nothing
/// here is per tick, and nothing here may grow a `_per_tick` (`docs/conventions.md`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwimTuning {
    /// The exponential rate at which water eats velocity: `v *= exp(-drag_per_s * dt)`.
    ///
    /// Exponential and not a subtraction, because a subtraction has to be clamped at zero and
    /// a clamp is a second rule about the same number. It also gives the two figures that make
    /// this feel like water rather than like treacle: a **terminal sink speed** of
    /// `-gravity_m_s2 / drag_per_s` and a **half-life** of `ln 2 / drag_per_s`.
    pub drag_per_s: f32,
    /// Upward acceleration on a **fully** submerged body, before gravity.
    ///
    /// ⚠️ **Gross, not net.** avian adds `game.ron: gravity_m_s2` in the same step and after
    /// this, so what a floating body actually feels is `buoyancy_m_s2 + gravity_m_s2` — the
    /// value here has to be bigger than 32 or the river is a hole you drown in.
    pub buoyancy_m_s2: f32,
    /// Over what depth the buoyancy ramps from nothing (at the surface) to all of it.
    ///
    /// It is what makes a body **float** instead of shooting out: the equilibrium sits at
    /// `-gravity_m_s2 * surface_band_m / buoyancy_m_s2` below the surface, which is where he
    /// bobs. Set it to 0 and the body would be a switch, not a float.
    pub surface_band_m: f32,
    /// How fast the legs move a body through water. Against `game.ron: player.run_speed_m_s`
    /// this is the whole of *„wird langsam"*.
    pub swim_speed_m_s: f32,
    /// How hard the legs may push to reach [`Self::swim_speed_m_s`]. Small: water is what you
    /// push against, and it pushes back.
    pub swim_accel_m_s2: f32,
    /// **What the gear costs while it is working under water**, as a factor on every gas rate
    /// and every gas impulse (`vector::gas::gas_budget`).
    ///
    /// The other half of *„wird langsam"*: getting out of the river is a hook and a reel, and
    /// this is what that hook and that reel cost extra. `1.0` would make water free and is a
    /// legal value for exactly that experiment.
    pub gas_cost_factor: f32,
}

/// One body of water: an axis-aligned box, in world metres.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterVolumeSpec {
    /// For `Name`, for a log line and for an assertion message.
    pub name: String,
    /// Centre in world metres — **not** a corner, the same convention `maps.ron: blocks` uses.
    pub center_m: (f32, f32, f32),
    /// The **whole** edge length, like `maps.ron: blocks.size_m` and like `Collider::cuboid`.
    /// `shared::WaterVolume` carries the half; the conversion happens once, in
    /// `world::water::build_water`.
    pub size_m: (f32, f32, f32),
    /// Linear RGB. **Not a palette key**, and that is deliberate: the palette lives in
    /// `maps.ron` and this is a different file. A river is one hue in the whole game.
    pub color: (f32, f32, f32),
}

impl WaterVolumeSpec {
    pub fn center(&self) -> Vec3 {
        Vec3::new(self.center_m.0, self.center_m.1, self.center_m.2)
    }

    pub fn size(&self) -> Vec3 {
        Vec3::new(self.size_m.0, self.size_m.1, self.size_m.2)
    }

    pub fn color(&self) -> [f32; 3] {
        [self.color.0, self.color.1, self.color.2]
    }

    /// The y of the surface — the top of the box.
    pub fn surface_y_m(&self) -> f32 {
        self.center_m.1 + self.size_m.1 * 0.5
    }

    /// The y of the bed — the bottom of the box.
    pub fn bed_y_m(&self) -> f32 {
        self.center_m.1 - self.size_m.1 * 0.5
    }
}
