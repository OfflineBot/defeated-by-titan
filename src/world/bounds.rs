//! `F-012` — **the fence.** Four invisible boxes at the map's own edge, so that normal play
//! cannot walk or fly out of the world.
//!
//! The user, 2026-08-27: *„und man kann an der seite einfach runterfallen!"* — and asked what
//! should be there he named **both** halves:
//! *„unsichtbare wand + wenn man runterfaellt wegen bug teleport man zurueck!"*
//!
//! ⚠️ **This file is only the first half, and it is the half that cannot be trusted alone.**
//! The second one lives in [`crate::player::recovery`] and it exists for the case where *this*
//! has already failed — a warp, a seam, a body somebody put somewhere by hand. Built as "the
//! wall, and then the wall again", the feature does nothing at all for the case it was asked
//! for. `docs/BUGS.md` B-015 measured the difference: a body put 2 m under the world by hand
//! stayed there, and no fence in any position would have changed that.
//!
//! ## What the cause actually was, and it was the boring one of the four
//!
//! Not a plate smaller than the playable area, not a seam, not tunnelling: **past the plate
//! there is nothing at all**. Ashgate's two ground slabs cover `x ∈ [-350, 350]`,
//! `z ∈ [-350, 350]` exactly, which is its declared `size_m` to the metre — and one metre
//! further out there is no collider at any height, ever. Twelve stances measured, twelve fell.
//!
//! ## What this half CANNOT do, and it took a refutation round to say it out loud
//!
//! A fence is a horizontal answer. It is what normal play runs into — walking at the edge,
//! swinging at the edge, arriving at `vector.max_speed_m_s` — and every one of those is
//! measured in `tests/player.rs::f012_*`. It answers **nothing** about a player who goes over
//! it, and on 2026-08-28 that was two held keys and seven seconds away. Whoever reads this file
//! looking for "why can I leave the map" has to read [`crate::player::recovery`] as well: the
//! two halves are not the same mechanism twice, and the second one is the one that closes the
//! world.
//!
//! ## Why a collider and not a position clamp
//!
//! A clamp would be a second writer on a player's `Transform`, against avian
//! (`CLAUDE.md` rule 4, and the authority table in `src/player/mod.rs`). A static box is the
//! same kind of thing as every other wall in the district: the solver already knows how to
//! stop a body at 75 m/s in front of one, the rope already knows how to swing against one, and
//! nothing new gets a vote on where the player is.
//!
//! ## What the fence deliberately is NOT
//!
//! - **Not drawn.** No [`Block`](crate::shared::Block), so `render` never sees it. That is the
//!   whole of "invisible" — there is no transparency involved anywhere.
//! - **Not a body of the world.** No [`Body`](crate::shared::Body), so it never enters the
//!   [`SpatialIndex`](crate::shared::SpatialIndex), never gets a `BodyId`, and can therefore
//!   never be an anchor: `vector::hook` asks `bodies.get(hit.entity)` and a fence panel is not
//!   in that query at all, so a rope that reaches one reports `SurfaceHoldsNothing` and comes
//!   home. It also keeps the fence out of `tests/world.rs::t036a_*`, whose whole subject is
//!   that the number of bodies equals the number of **planned blocks**.
//! - **Not infinitely tall, and 🔴 not a ceiling either.** `bounds.fence_top_m` stops it at
//!   200 m, and one reason is measured: an aim ray sees a collider whether or not it is drawn,
//!   and `tests/vector_hooks.rs::f028_a_failed_pull_says_which_of_the_four_it_was` fires a
//!   level ray from 400 m up and needs "open sky" back. A fence to the sky would have turned
//!   that into "the surface holds nothing" — the same class of collateral as `B-010`.
//!
//!   The other reason used to be *"above anything the gear reaches on gas alone"*, and **that
//!   was false**: measured 2026-08-28, `W` + `Shift` from a standing start clears 200 m in
//!   under seven seconds for 0.72 % of one tank, and the climb has no apex at all — held long
//!   enough the body sits at `vector.max_speed_m_s` going up, 4.163 m per unit of gas, 62 km
//!   on a full tank. **No number here would have been enough**, and a bigger one buys a bigger
//!   version of the next problem.
//!
//! - 🔴 **Its top face is a floor, and that is not fixable here.** A cuboid has six faces; the
//!   upper one is a solid, invisible, standable ring `fence_thickness_m` wide at
//!   `fence_top_m`, running the whole way round the district **outside the map's footprint**.
//!   A body put on it at `(355, 210, 0)` rested at exactly y = 200.000 and was still there
//!   14 s later — recorded by nothing (`record_safe_ground` correctly refuses a stance outside
//!   the map) and recovered by nothing (500 m above the plane). Every fence height has this
//!   ring; it just moves.
//!
//!   So the ring is closed by two things and neither is enough alone.
//!   [`crate::player::recovery::out_of_the_world`] says **outside the map's own footprint is
//!   out of the world at any height** — and it says it with a **strict** `>`, which is why the
//!   fence may not touch the boundary. `maps.ron: bounds.fence_margin_m` stands it 0.18 m
//!   further out, bracketed by two measurements.
//!
//!   🔴 **`fence_margin_m: 0.0` shipped for two days and it was the whole bug** (`B-017`): the
//!   inner lip stood exactly on `hx`, `|x| > hx` is strict, and the line |x| = 350.000 was a
//!   solid invisible standable floor the rule called *in the world*. `warp 350 201 0` rested
//!   at 200.000 after ten seconds; `record_safe_ground` made it home; one nudge outward warped
//!   the player 1501 times in 25 s. And *"move the fence out by any amount"* is not the fix
//!   either — a capsule rests on its bottom sphere, which reaches `player.radius_m` over the
//!   lip, so what has to be cleared is the **parking reach** (`bounds.fence_rest_reach_m`) and
//!   not the float grid (`FIND-205`).
//!
//!   `tests/world.rs::f012_the_whole_top_face_of_the_fence_lies_outside_the_map_and_is_
//!   recovered_from` sweeps every panel's top face — **every sample of it, with no `continue`
//!   in the function**, which is the other half of `B-017`: the old version skipped the 64 of
//!   648 samples on the inner line and called them "a body cannot rest on a line".

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::data::{GameData, Map};

/// One panel of the fence, as a plan — **without** Bevy, without `Commands`, without side
/// effects, exactly like [`crate::world::map::plan_blocks`]. A test can then check where the
/// fence stands without building an app, and the builder below has nothing left to get wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct FencePanel {
    pub name: &'static str,
    /// World centre in meters.
    pub center_m: Vec3,
    /// **Full** edge length in meters, the way `maps.ron` and `Collider::cuboid` carry it.
    pub size_m: Vec3,
}

/// Where the four panels stand, out of the map's own `size_m` and its `bounds`.
///
/// The two panels on `x` are made long enough to cover the corners (`+/- (hz + thickness)`),
/// so the four boxes overlap in pairs and there is no diagonal gap. A corner is exactly where
/// a body arriving at 45° would find one.
pub fn plan_fence(map: &Map) -> [FencePanel; 4] {
    let b = &map.bounds;
    assert!(
        b.fence_top_m > b.fence_bottom_m,
        "maps.ron: {}: fence_top_m {} is not above fence_bottom_m {} — the fence would be \
         inside out",
        map.name,
        b.fence_top_m,
        b.fence_bottom_m
    );
    assert!(
        b.fence_thickness_m > 0.0,
        "maps.ron: {}: fence_thickness_m {} — a fence of no thickness is not a fence",
        map.name,
        b.fence_thickness_m
    );

    let t = b.fence_thickness_m;
    let hx = map.size_m.0 * 0.5 + b.fence_margin_m;
    let hz = map.size_m.1 * 0.5 + b.fence_margin_m;
    let height = b.fence_top_m - b.fence_bottom_m;
    let y = (b.fence_top_m + b.fence_bottom_m) * 0.5;

    [
        FencePanel {
            name: "fence_x_plus",
            center_m: Vec3::new(hx + t * 0.5, y, 0.0),
            size_m: Vec3::new(t, height, 2.0 * (hz + t)),
        },
        FencePanel {
            name: "fence_x_minus",
            center_m: Vec3::new(-hx - t * 0.5, y, 0.0),
            size_m: Vec3::new(t, height, 2.0 * (hz + t)),
        },
        FencePanel {
            name: "fence_z_plus",
            center_m: Vec3::new(0.0, y, hz + t * 0.5),
            size_m: Vec3::new(2.0 * hx, height, t),
        },
        FencePanel {
            name: "fence_z_minus",
            center_m: Vec3::new(0.0, y, -hz - t * 0.5),
            size_m: Vec3::new(2.0 * hx, height, t),
        },
    ]
}

/// Builds the fence of the map that is being played. `Startup`, beside `map::build_map`.
pub fn build_bounds(mut commands: Commands, data: Res<GameData>) {
    let Some(map) = data.current_map() else {
        // `map::build_map` panics on the same condition one system earlier and says why; a
        // second panic here would only bury it.
        return;
    };

    let panels = plan_fence(map);
    for panel in &panels {
        commands.spawn((
            Name::new(panel.name),
            // No `Block` (nothing draws it) and no `Body` (nothing hooks it) — see the module
            // header. `RigidBody::Static` is not optional: `tests/player.rs::
            // t007_every_world_collider_carries_a_rigid_body` is the guard for every collider
            // in the game, and a character controller added later would be blind without it.
            RigidBody::Static,
            Collider::cuboid(panel.size_m.x, panel.size_m.y, panel.size_m.z),
            Transform::from_translation(panel.center_m),
        ));
    }

    info!(
        "map {:?}: fenced at +-({:.1}, {:.1}) m, {:.0} m tall, recovery plane at {} m",
        map.name,
        map.size_m.0 * 0.5 + map.bounds.fence_margin_m,
        map.size_m.1 * 0.5 + map.bounds.fence_margin_m,
        map.bounds.fence_top_m - map.bounds.fence_bottom_m,
        map.bounds.recovery_plane_y_m,
    );
}
