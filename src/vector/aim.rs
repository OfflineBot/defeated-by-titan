//! `F-002` Free aiming by ray — **layer 1 of the aiming system.**
//!
//! `F-002` verbatim: "Raycast from the camera position along the look direction, range =
//! range stat. The hit point is checked against a valid anchor surface. **This layer stays
//! ALWAYS active and is never replaceable by the snap system.**"
//!
//! ## Three traps, all three with evidence
//!
//! 1. **`bevy::picking::MeshRayCast` is forbidden.** `features = ["picking"]` puts it within
//!    reach (`Cargo.toml`, expanded in `bevy-0.19.0/Cargo.toml:2820-2825`) and it iterates
//!    over **all** visible mesh entities — the source says it word for word:
//!    "Check all entities" (`bevy_picking-0.19.0/src/mesh_picking/ray_cast/mod.rs:224`,
//!    `culling_query.par_iter()` at `:228`). That is exactly the §11 breach that works in the
//!    graybox and shows up at a thousand houses. What we ask is avian's [`SpatialQuery`],
//!    which walks a BVH (`avian3d-0.7.0/src/spatial_query/system_param.rs:176-200`,
//!    `tree.ray_traverse_closest`). Measured on 2026-08-09: **0.21 us** for a 112 m ray
//!    against 4000 blocks.
//! 2. **The ray starts at eye height, not at the player origin.** The origin sits between the
//!    feet (`docs/conventions.md`); `player.eye_height_m` is the same number `render` hangs
//!    the camera on. That is how `vector` gets to the eye point **without knowing the
//!    camera** — no query on `Camera3d`, no edge. It is not cosmetic: a ray from the origin
//!    still lands on the same wall *plane*, so every test that only measures a distance
//!    passes while the aim point sits 1.6 m too low.
//! 3. **A ray hits the player's own capsule**, and the eye at 1.6 m lies *inside* it
//!    (0 .. 1.8 m, radius 0.35). Without an exclusion, every shot reports a hit at zero
//!    distance. [`SpatialQueryFilter::with_excluded_entities`] matches the **collider**
//!    entity, not the body (`query_filter.rs:97`, called with `proxy.collider` in
//!    `system_param.rs:190`) — it only works because `player::spawn_player` puts the collider
//!    on the **same** entity as the body. Whoever moves it into a child re-breaks this.
//!
//! And the rule the whole feature hangs on: **hit first, then check anchorable.** The ray is
//! cast with the default filter — no layer mask, no `cast_ray_predicate` that skips untagged
//! bodies. Measured on 2026-08-09 with exactly the pair `maps.ron` keeps for it: a filtered
//! ray travels 19.85 m **through** the untagged wall at `z = -33.5` to reach the anchorable
//! roof behind it at 9.85 m. `F-023` forbids that in so many words ("line-of-sight check
//! prevents hooking through walls"), and [`AimPoint`] is built for it: `point_m` and
//! `anchorable` are separate fields, so "there is something there, but you cannot hook it" is
//! a state and not a missing hit.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{AimPoint, ArmAim, Body, BodyId, BodyMask, Intent, Side};

use super::hook::anchor_target;

/// How far off the look direction one side ray sits, given the player's spread setting.
/// Radians in, radians out (`docs/conventions.md`).
///
/// ⚠️ **`aim_spread_deg` is a HALF-angle**, and that is a decision with two sources pulling
/// against each other:
///
/// - `assets/data/game.ron` says so and does the arithmetic in its own comment: *"At 100 m of
///   aim distance 28° puts the two side points 2 · 100 · sin(28°) = **93.9 m** apart"*, and
///   its ceiling `aim_spread_max_deg: 44.0` is justified as *"1.75° to spare"* against the
///   45.75° half of the 91.5° horizontal frustum. Both numbers are only true for a half-angle.
/// - `docs/NEXT.md` §1B's `W3` brief says *"two side rays at ±`aim_spread_deg`/2"*.
///
/// **The file wins** (rule 2: the number and its meaning live in the RON), and the acceptance
/// criterion holds either way — the brief asks for ≥ 45 m at 28°/100 m and this gives 93.9.
/// The contradiction is written down in `docs/FINDINGS.md` FIND-083; flipping the reading is
/// this one function.
pub fn side_angle_rad(spread_rad: f32) -> f32 {
    spread_rad
}

/// The two side directions, `Side::Left` first — the look direction yawed by
/// ±[`side_angle_rad`] **around the camera's up axis**, not around world Y.
///
/// `spread_rad` is what [`Intent::aim_spread_rad`] hands over: the player's own wheel setting,
/// already clamped into the window from `game.ron`. That clamp lives on the `Intent` and not
/// here, so that the wheel, the HUD and this ray can never disagree about what 0° means.
///
/// Around the camera's up axis, because the spread the user asks for is a *screen* spread:
/// looking 60° down, a yaw around world Y rolls the two rays into the ground on one side and
/// into the sky on the other, and the two markers stop being left and right of the crosshair.
///
/// `look` and `right` are orthonormal by construction — `right` is the horizontal
/// `(cos yaw, 0, -sin yaw)` of `docs/conventions.md`'s axis contract, and the look direction
/// has no component along it at any pitch — so `look·cos ∓ right·sin` is a unit vector
/// without a normalize, at every pitch including straight up and straight down.
pub fn side_dirs(intent: &Intent, spread_rad: f32) -> [Vec3; 2] {
    let look = intent.look_dir();
    let (sin_yaw, cos_yaw) = intent.yaw.sin_cos();
    let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
    let (sin, cos) = side_angle_rad(spread_rad).sin_cos();
    // Index order is `Side::index()`: left = 0, right = 1. Left of a player looking along -Z
    // is -X, so the left ray leans against `right`.
    [look * cos - right * sin, look * cos + right * sin]
}

/// Writes [`AimPoint`] and [`ArmAim`] for every player, once per fixed step.
///
/// Runs in `SimulationSystems::World`: a question to the world, asked **before** anything
/// moves, so that every system in that stage sees the same world (`shared::schedule`).
///
/// **One writer:** [`AimPoint`] and [`ArmAim`] are written here and nowhere else. `hud` reads
/// the centre point for the crosshair; `vector::hook` reads **only** [`ArmAim`], so what the
/// rope flies at and what the marker shows is one number and not two (`F-023`).
///
/// ## What this system does not see, and why that is safe today
///
/// avian keeps the broad-phase BVH per body type and rebuilds the **dynamic** trees inside
/// the physics step (`avian3d-0.7.0/src/collider_tree/mod.rs:8-11`), which runs one stage
/// later than this one (`SimulationSystems::Integrate`). So a ray asked here sees a moving
/// body at its position from the end of the previous tick. Every body in the world is
/// **static** today — houses do not move — and the only dynamic bodies are the players, whose
/// own capsule is excluded anyway. The day a titan limb becomes an anchor (`F-029`) this
/// becomes a real one-tick lag and has to be measured, not argued about.
pub fn aim(
    data: Res<GameData>,
    space: SpatialQuery,
    bodies: Query<(&Body, Option<&BodyId>)>,
    mut players: Query<(Entity, &Intent, &Transform, &mut AimPoint, &mut ArmAim)>,
) {
    let v = &data.game.vector;
    let range_m = v.hook_range_m;
    let eye_height_m = data.game.player.eye_height_m;

    for (player, intent, transform, mut point, mut arms) in &mut players {
        let eye_m = eye(transform.translation, eye_height_m);
        let centre = cast(&space, &bodies, player, eye_m, intent.look_dir(), range_m);

        // Three rays, not one. The centre stays the crosshair's source and is cast from the
        // look direction alone; the two side rays are what Q and E fly at (`F-023`).
        //
        // Three `cast_ray` instead of one is the whole added cost: measured 0.21 us per ray
        // against 4000 blocks (module header), so 0.63 us per player per tick against a
        // 16 666 us budget. It is a BVH walk, not an iteration over the world (§11).
        // The player's own wheel setting (`W2`), absolute and never a delta, clamped into the
        // file's window by the `Intent` itself — a per-player number, not a resource, because
        // there is no such thing as *the* player (`docs/multiplayer.md` rule 3).
        let spread_rad = intent.aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg);
        let dirs = side_dirs(intent, spread_rad);
        let sides = Side::ALL.map(|side| {
            let found = cast(&space, &bodies, player, eye_m, dirs[side.index()], range_m);
            // **The fallback, and it is the difference between a feature and a regression.**
            // A side ray that finds nothing to hook — off the roof edge, into the sky, or
            // onto an untagged wall — hands the arm the centre ray instead of nothing. Aiming
            // at a lone tower has to keep working exactly as well as it did when both arms
            // shared one point; without this line the spread would cost hit rate on every
            // target narrower than the spread itself.
            //
            // Resolved HERE and not at fire time: what is written into `ArmAim` is what the
            // rope flies at and what the HUD draws, and a rule applied twice in two files is
            // how a marker and a rope end up in two places (`user-messages.md`, 2026-08-12).
            if anchor_target(&found).is_some() { found } else { centre }
        });

        // `set_if_neq` and not a plain assignment: a standing player aims at the same point
        // every tick, and a `Mut` that is written unconditionally triggers change detection
        // 60 times a second for nothing (`docs/lessons/performance.md`, rule 1).
        point.set_if_neq(centre);
        arms.set_if_neq(ArmAim { arms: sides });
    }
}

/// The eye point: the origin sits **between the feet**, the eye `eye_height_m` above it.
///
/// One line, one place — `render` hangs the camera on the same number, and two spellings of
/// the same offset are how a crosshair and a hook end up pointing at different things.
pub fn eye(translation_m: Vec3, eye_height_m: f32) -> Vec3 {
    translation_m + Vec3::Y * eye_height_m
}

/// One shot. Separated from the system so that it takes no `Res` and can be measured.
///
/// Returns [`AimPoint::default()`] — nothing hit — for a look direction that is not a
/// direction (NaN out of a broken `Intent`) and for an origin that is not finite. A `NaN` in
/// the aim point becomes a `NaN` target for the hook and then a `NaN` in the `Transform`, and
/// that looks like "the player has vanished" (`prompts/init.md` §9d).
fn cast(
    space: &SpatialQuery,
    bodies: &Query<(&Body, Option<&BodyId>)>,
    player: Entity,
    eye_m: Vec3,
    look: Vec3,
    range_m: f32,
) -> AimPoint {
    if !eye_m.is_finite() || !(range_m.is_finite() && range_m > 0.0) {
        return AimPoint::default();
    }
    let Ok(direction) = Dir3::new(look) else {
        return AimPoint::default();
    };

    // `solid: true` — an eye that has ended up inside a body reports **that** body at zero
    // distance instead of shooting out through its far side and offering an anchor across
    // half the map (`system_param.rs:111-120`). No filter and no predicate: the mask is
    // asked AFTER the hit, never before it (`F-023`).
    let filter = SpatialQueryFilter::from_excluded_entities([player]);
    let Some(hit) = space.cast_ray(eye_m, direction, range_m, true, &filter) else {
        return AimPoint::default();
    };

    // The hit entity is the **collider**, not the body. Today the two are the same entity for
    // a house (`world::map`) and for a player (`player::spawn_player`); for a child collider
    // they would not be, and then this lookup returns `Err` and the surface counts as not
    // anchorable — visibly wrong instead of silently hookable.
    let (body, id) = match bodies.get(hit.entity) {
        Ok((body, id)) => (Some(body), id.copied()),
        Err(_) => (None, None),
    };

    AimPoint {
        point_m: Some(eye_m + direction * hit.distance),
        body: id,
        anchorable: body.is_some_and(|b| b.mask.contains(BodyMask::ANCHORABLE)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f002_the_ray_starts_at_the_eye_and_not_between_the_feet() {
        // The whole point of the eye offset. A ray from the origin lands on the same wall
        // PLANE and therefore passes every test that only measures a distance — while the
        // aim point sits a whole body height too low.
        let feet = Vec3::new(3.0, 0.0, -4.0);
        assert_eq!(eye(feet, 1.6), Vec3::new(3.0, 1.6, -4.0));
        // And it is the number out of the file, not the body height: 1.6, not 1.8 and not
        // the 1.65 that was estimated from the body height until 2026-08-09.
        assert_ne!(eye(feet, 1.6), eye(feet, 1.8));
    }
}
