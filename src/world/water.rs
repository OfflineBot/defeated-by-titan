//! **The river** — `assets/data/water.ron`, one entity per volume, and nothing else.
//!
//! Until 2026-09-01 the canal of Ashgate was a *dry lowered lane*: a 10 m gap between two
//! quays, floor 4 m down, `anchorable: false`, and `maps.ron` said so in its own heading —
//! *"The river, in a game that has no water"*. There was no surface, no colour and no rule for
//! what happens when a body falls in. This file puts the volume there; `player::swim` is the
//! rule, `vector::hookable` is the answer to whether it holds a hook, and `render` is what you
//! see.
//!
//! ## What a water entity is made of, and what it deliberately is not
//!
//! ```text
//!   Name                what a log line and an assertion call it
//!   Transform           the centre of the box
//!   WaterVolume         the half extent, the colour  — shared/, so `player` may read it
//! ```
//!
//! and **no `Block`, no `Body`, no `Collider`, no `RigidBody`.** The whole argument is in
//! [`crate::shared::water`]; the short version is that a collider you can swim through is a
//! `Sensor`, a `Sensor` answers `SpatialQuery::cast_ray` like anything else, and the hook the
//! player fires **from inside the water** to get out would then hit the water at distance 0.
//!
//! ## Why the volumes are not in `maps.ron`
//!
//! Because that file is a list of **cuboids of the city** and every guard around it counts
//! them: `tests/world.rs::f003_the_city_comes_from_the_file_and_not_twice` compares the number
//! of `Block` entities against `world::map::plan_blocks`, and the `SpatialIndex` length is
//! compared against the same plan one test further down. Four supply poles once made that
//! count 2875 against 2871 (`world::supply`), and this is the same shape: a river is not a
//! house.
//!
//! Seen: `docs/images/f003-water.png`, driven with `scripts/f-water.txt`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::WaterVolume;

/// Spawns every body of water the current map has. **Sole writer of [`WaterVolume`].**
///
/// A map with no water is a map with **no key** in `water.ron: volumes` — that is not an
/// error, it is the graybox, and it says so once rather than warning every frame.
pub fn build_water(mut commands: Commands, data: Res<GameData>) {
    let Some(name) = current_map_name(&data) else {
        return;
    };
    let Some(volumes) = data.water.volumes.get(&name) else {
        info!("water: the map {name:?} has no water in water.ron");
        return;
    };
    for volume in volumes {
        let size = volume.size();
        // ⚠️ The file carries the WHOLE edge (the same convention `maps.ron: blocks.size_m`
        // uses and the same one `Collider::cuboid` takes); `WaterVolume` carries the HALF, the
        // way `Body::half_size_m` and `Aabb3d::new` do. The factor of two happens exactly
        // once, here, and `tests/world.rs` measures the built volume against the file.
        commands.spawn((
            Name::new(format!("water_{}", volume.name)),
            Transform::from_translation(volume.center()),
            WaterVolume { half_size_m: size * 0.5, color: volume.color() },
        ));
    }
    info!("water: {} volume(s) on {name}", volumes.len());
}

/// The name of the map being built — `maps.ron: current`, resolved through `GameData` so that
/// a test which repoints `current` gets the water of the map it asked for and not of the one
/// the file names.
fn current_map_name(data: &GameData) -> Option<String> {
    data.current_map().map(|_| data.maps.current.clone())
}
