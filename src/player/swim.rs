//! **Swimming** — what water does to a body, and the whole of it is one pure function.
//!
//! The user was asked on 2026-08-29 what should happen when a body meets the river, and
//! answered:
//!
//! > *„Man schwimmt / wird langsam."*
//!
//! So water is **terrain with a cost**: not a killing plane, not a wall, not a bounce. You
//! fall in, the water takes your speed, it holds you at the surface, your legs move you at a
//! walking pace divided by two, and you get out with the gear — which costs more gas while it
//! is working under water (`vector::gas`, `water.ron: swim.gas_cost_factor`).
//!
//! ## The four things [`swim_step`] does, in this order, and why the order is the rule
//!
//! ```text
//!   1  not in the water        -> return the velocity untouched. NOTHING below runs.
//!   2  drag        v *= exp(-drag_per_s * dt)          isotropic: down, sideways and along
//!                                                      a rope all the same
//!   3  buoyancy    v.y += buoyancy_m_s2 * f * dt       f = depth / surface_band_m, 0..1
//!   4  the legs    horizontal, toward swim_speed_m_s, at most swim_accel_m_s2 * dt
//! ```
//!
//! **Drag before buoyancy**, or the lift would be damped by the same tick that produced it and
//! the equilibrium depth would depend on `dt`. **Buoyancy before the legs**, because the legs
//! are horizontal and must not be able to cancel the lift.
//!
//! ⚠️ **`buoyancy_m_s2` is GROSS.** avian adds `game.ron: gravity_m_s2` (−32.0) in the very
//! same step, *after* this system, so a fully submerged body feels `44 − 32 = +12 m/s²`. That
//! is the reason this file may not "just apply the net value": the net value is not a number
//! anybody owns, it is the difference between two files.
//!
//! ## Why this writes `LinearVelocity` and how it does not fight `ground_locomotion`
//!
//! It is chained **after** [`super::locomotion::ground_locomotion`] and before every avian
//! system, and it touches a player only while [`Submerged::wet`] — which is exactly the state
//! in which the ground has nothing to say, because there is no ground under him. The same
//! split-by-state the authority table already makes between `player` and `vector`
//! (`docs/architecture.md`): one writer per field **per state**, never two at once.
//!
//! ## Why the water is a query and not the spatial index
//!
//! Because `SpatialIndex::aabb_overlaps` **is a stub** — it clears the output buffer and
//! returns (`src/shared/spatial.rs`, "filled in by job R"). Building the swim rule on it would
//! be building on 🟨, and it would answer "no water anywhere" without a single test going red.
//! What this queries instead is the [`WaterVolume`] archetype, which has **one entity in the
//! shipped map** and is not "every entity in the world" — the thing `CLAUDE.md` rule 6
//! forbids. The day the index answers, this is where it changes.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use crate::data::{GameData, SwimTuning};
use crate::shared::{Intent, PlayerId, Submerged, WaterVolume};

/// **The swim rule**, as a function of nothing but its arguments, so that it can be checked
/// without an `App`, without avian and without a map (`tests/player.rs`).
///
/// * `velocity_m_s` — the body's velocity at the start of the tick, after the legs have had
///   their say on land.
/// * `depth_m` — metres between the surface and the body's **origin**, which sits between the
///   feet (`docs/conventions.md`). `0.0` or less is dry, and dry returns the input.
/// * `wish_dir` — where the player wants to go, in world space, horizontal, **not**
///   normalised (a half-held stick is half a wish). Its Y is ignored: you steer a swim with
///   your legs and you climb out with the gear.
/// * `dt_s` — one fixed tick.
///
/// The returned velocity is what avian integrates **before** it adds gravity.
pub fn swim_step(
    velocity_m_s: Vec3,
    depth_m: f32,
    wish_dir: Vec3,
    tuning: &SwimTuning,
    dt_s: f32,
) -> Vec3 {
    if depth_m <= 0.0 {
        return velocity_m_s;
    }

    // 2 — drag. Exponential and not a subtraction: a subtraction has to be clamped at zero,
    // and a clamp is a second rule about the same number. Isotropic on purpose — the water
    // does not care which way you are going, and a horizontal-only drag would let a dive keep
    // all of its speed, which is the one entry the player actually notices.
    let damped = velocity_m_s * (-tuning.drag_per_s * dt_s).exp();

    // 3 — buoyancy, ramped over the top `surface_band_m` so that a body FLOATS instead of
    // being switched between sinking and shooting out. `surface_band_m <= 0` would be a
    // division by zero; it means "no ramp", i.e. full lift from the first millimetre.
    let submersion = if tuning.surface_band_m > 0.0 {
        (depth_m / tuning.surface_band_m).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let lifted = damped + Vec3::Y * (tuning.buoyancy_m_s2 * submersion * dt_s);

    // 4 — the legs. Horizontal only, accelerating toward the wish at a rate water allows.
    // ⚠️ With **no** key held the wish is zero and this brakes as well, which is deliberate:
    // water is what you push against, and a body that keeps drifting after you let go reads as
    // ice. It is bounded by `swim_accel_m_s2 * dt` like every other case, so it can never
    // reverse the velocity in one tick.
    let want = wish_dir.clamp_length_max(1.0) * tuning.swim_speed_m_s;
    let have = Vec3::new(lifted.x, 0.0, lifted.z);
    let step = (Vec3::new(want.x, 0.0, want.z) - have).clamp_length_max(tuning.swim_accel_m_s2 * dt_s);
    lifted + step
}

/// **How deep a point is in the deepest water it is in**, or 0.0 for dry.
///
/// `max` and not `first`: two overlapping volumes are one body of water, and taking whichever
/// the query happened to yield first would make the depth depend on spawn order. It is also
/// the one place the whole rule reads the world, which is why it is a function of a slice and
/// not of a `Query` — `tests/player.rs` hands it **two** volumes, because a rule that has only
/// ever been given one is a rule about a different function (`CLAUDE.md` rule 5).
pub fn depth_in(waters: &[(WaterVolume, Vec3)], point_m: Vec3) -> f32 {
    waters
        .iter()
        .filter_map(|(water, centre)| water.depth_m(*centre, point_m))
        .fold(0.0f32, f32::max)
}

/// The player's wish direction in world space, out of his own [`Intent`].
///
/// The same two basis vectors `locomotion::ground_locomotion` builds, and deliberately the
/// same shape: movement is player-local and gets rotated into the world before it is used.
pub fn wish_dir(intent: &Intent) -> Vec3 {
    let (sin, cos) = intent.yaw.sin_cos();
    let forward = Vec3::new(-sin, 0.0, -cos);
    let right = Vec3::new(cos, 0.0, -sin);
    forward * intent.move_y + right * intent.move_x
}

/// Applies [`swim_step`] to every player, and is the **sole writer of [`Submerged`]**.
///
/// It writes `Submerged` for **every** player on every tick, wet or dry — a component that is
/// only updated while it is interesting is a component that stays true after the interesting
/// part ended, and `vector::gas` would go on charging water prices on the quay.
pub fn swim_in_water(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    waters: Query<(&WaterVolume, &Transform)>,
    mut players: Query<(&Intent, &Transform, &mut LinearVelocity, &mut Submerged), With<PlayerId>>,
) {
    // `Transform` and not `GlobalTransform` for the water: a volume has no parent, so the two
    // are the same value, and `GlobalTransform` is only propagated at the end of a frame —
    // taking it here would make the first tick's answer depend on the schedule.
    let mut volumes: Vec<(WaterVolume, Vec3)> = Vec::new();
    for (water, at) in &waters {
        volumes.push((*water, at.translation));
    }

    let dt = time.delta_secs();
    let tuning = &data.water.swim;

    for (intent, at, mut velocity, mut submerged) in &mut players {
        let depth = depth_in(&volumes, at.translation);
        // `set_if_neq`: a dry player's depth is 0.0 on all sixty ticks, and a component that
        // reports itself changed every tick makes every `Changed<T>` filter after it worthless.
        submerged.set_if_neq(Submerged { depth_m: depth });
        if depth <= 0.0 {
            continue;
        }
        let next = swim_step(velocity.0, depth, wish_dir(intent), tuning, dt);
        if next != velocity.0 {
            velocity.0 = next;
        }
    }
}
