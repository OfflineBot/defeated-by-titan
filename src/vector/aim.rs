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
//! cast with no `cast_ray_predicate` and with the widest mask that is still correct —
//! [`shared::AIM_RAY_SEES`](crate::shared::AIM_RAY_SEES), which is everything except another
//! **player**. Untagged geometry is bit 0 and stays in; only a team mate is dropped, and he is
//! dropped because he is not a surface (`docs/BUGS.md` B-010).
//! Measured on 2026-08-09 with exactly the pair `maps.ron` keeps for it: a filtered
//! ray travels 19.85 m **through** the untagged wall at `z = -33.5` to reach the anchorable
//! roof behind it at 9.85 m. `F-023` forbids that in so many words ("line-of-sight check
//! prevents hooking through walls"), and [`AimPoint`] is built for it: `point_m` and
//! `anchorable` are separate fields, so "there is something there, but you cannot hook it" is
//! a state and not a missing hit.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::data::{GameData, VectorTuning};
use crate::shared::{
    AimPoint, ArmAim, Body, BodyId, BodyMask, Hook, HookState, Intent, LocalPlayer,
    MovementState, PlayerSettings, Side, Velocity, AIM_RAY_SEES,
};

use super::hook::anchor_target;

/// **The one place that says what `aim_spread_deg` means: it is the angle BETWEEN the two
/// side rays, and everything downstream of this line is a HALF-angle.**
///
/// `docs/FINDINGS.md` FIND-086 recorded the contradiction and left it open: `assets/data/
/// game.ron` and `src/data/mod.rs` read the key as a half-angle (`±28°`, 56° of fan), while
/// `docs/NEXT.md` §1B's brief specified *"two side rays at ±`aim_spread_deg`/2"* (28° of fan).
/// The file won on rule 2 and nothing decided between them on merit.
///
/// **Resolved for the brief on 2026-08-18, and the tiebreaker is that the game has now been
/// played:** *„der spread für seile ist zu weit auseinander"* (the user). Under the old reading
/// the shipped wheel opened **56°** of a 91.5° horizontal frustum — the two markers stood 61 %
/// of the screen apart — and the ceiling `aim_spread_max_deg: 44.0` meant 88°, very nearly the
/// whole image. Under this one the wheel is the fan itself: 28° is 31 % of the screen, and the
/// widest a player may dial is now narrower than what the game used to hand him by default.
/// `docs/FINDINGS.md` **FIND-096**.
///
/// Every other reading of the number is derived from this function, so there is exactly one
/// line to flip if the decision is ever reversed — and `tests/vector_aiming.rs::
/// f023_the_side_ray_sits_at_half_the_wheel_at_every_pitch` goes red the moment it is.
pub fn wheel_half_rad(wheel_rad: f32) -> f32 {
    0.5 * wheel_rad
}

/// How far off the look direction one side ray sits, given the angle the model **resolved**
/// this tick. Radians in, radians out (`docs/conventions.md`).
///
/// The argument is already a half-angle — [`effective_spread_rad`] produced it, and it took
/// the wheel's total apart with [`wheel_half_rad`] on the way. This function is the identity
/// and stays here as the seam: rope, HUD marker and ray all reach the world through
/// [`side_dirs`], so there is one number and never two (`F-023`).
pub fn side_angle_rad(half_rad: f32) -> f32 {
    half_rad
}

/// **How wide the two arms aim right now, in radians off the look direction** — the model
/// behind `F-023` since 2026-08-18.
///
/// The user, 2026-08-18: *„der spread für seile ist zu weit auseinander und sollte mehr
/// dynamisch sein!"* Two claims, and one model answers both: **the two ropes aim a number of
/// METRES apart, and what the player is doing decides how many.**
///
/// ## Why metres and not degrees
///
/// A degree is a screen quantity whose world meaning moves by a factor of 20 over a 500 m hook
/// range: the shipped 28° puts the two landing points 9.4 m apart at 10 m and **187.8 m** apart
/// at 200 m. Nothing in Ashgate is 187 m wide — `lot_m` is 36 — so at range the two anchors are
/// in different parts of town, at most one of them is where you are going, and the side ray
/// that missed collapses onto the centre ray at [`aim`]'s fallback. **"Too wide" and "both arms
/// share one point again" (`docs/FINDINGS.md` FIND-039) are the same defect**, and a constant
/// angle causes both.
///
/// ## The five inputs
///
/// 1. **What you are hanging on** — [`MovementState`]. Tethered is a *chain* and stays near
///    your line (`aim_sep_tether_m`, the courtyard); grounded or on a wall you are picking a
///    route across the block face (`aim_sep_stand_m`); airborne and untethered you are
///    searching and may cross a street (`aim_sep_search_m`, the block pitch).
/// 2. **How fast**, on the **horizontal** speed — a straight fall would otherwise pin the fan
///    at the moment a falling player wants the widest sweep. Linear from
///    `aim_sep_calm_speed_m_s` (running) down to the floor at `aim_sep_fast_speed_m_s`
///    (FIND-041's measured chained-swing peak).
/// 3. **How far away what you are looking at is** — the smoothed centre distance. It is free:
///    [`aim`] casts that ray eleven lines before it needs this number.
/// 4. **Whether the crosshair found anything at all.** `None` means the world has told us
///    nothing yet, and then the wheel *is* the fan.
/// 5. **The wheel**, twice: it scales the metre target (`k = wheel / aim_sep_neutral_deg`) and
///    half of it is the hard **ceiling** on the result ([`wheel_half_rad`] — the wheel is the
///    angle *between* the rays). The user's own word decides that reading — *„wie weit
///    auseinander es gehen **darf**"* (2026-08-12).
///
/// ## Why the near field is not the wheel's business
///
/// Inputs 1-3 are all *metres of city*, and metres are the wrong unit at arm's length: a 36 m
/// block-face budget on a point 10 m away asks for ±61°, so the ceiling caught it and handed
/// the wheel straight back. That is why the round that made this model dynamic measured
/// **4 % narrower standing and 1 % airborne** over 10-50 m — a no-op at the ranges Ashgate is
/// actually built at (6 m streets, 6.5-11.5 m houses: the first hook of a flight is 6-20 m).
/// `aim_sep_full_reach_m` is the fix: the metre budget ramps in with distance, so below it the
/// angle is constant per state and the separation grows linearly. `docs/FINDINGS.md` FIND-096.
///
/// ## The invariant that makes the complaint unregressable
///
/// `effective_spread_rad(..) <= wheel_half_rad(ctx.wheel_rad)` in every state, at every
/// distance, at every wheel position — one `clamp`, and `tests/vector_aiming.rs` sweeps it.
/// **This model can never draw a wider fan than the player allows**, and since 2026-08-18 the
/// near field does not reach that ceiling at any wheel position: at the widest notch the two
/// hooks are 5.2 m apart at 10 m against the 9.4 m the game used to give at the *default*.
///
/// Pure: no `Res`, no `World`, no randomness. The angle is recomputed locally from the
/// replicated [`Intent`] plus local simulation state and **never goes on the wire**
/// (`docs/multiplayer.md`).
pub fn effective_spread_rad(v: &VectorTuning, ctx: SpreadContext) -> f32 {
    let floor_rad = v.aim_spread_floor_deg.to_radians();

    // 1. What you are hanging on, as a separation in METRES OF CITY.
    let state_m = match ctx.state {
        MovementState::Tethered => v.aim_sep_tether_m,
        MovementState::Airborne => v.aim_sep_search_m,
        MovementState::Grounded | MovementState::OnWall | MovementState::Downed => {
            v.aim_sep_stand_m
        }
    };

    // 2. The wheel scales that target — so one notch still bites at every distance, which is
    //    the failure mode of a model whose wheel only sets a ceiling.
    let k_wheel = ctx.wheel_rad.to_degrees() / v.aim_sep_neutral_deg.max(f32::MIN_POSITIVE);
    let state_m = (state_m * k_wheel).max(v.aim_sep_floor_m);

    // 3. How fast, on the HORIZONTAL speed: linear collapse towards the floor between running
    //    and FIND-041's measured chained-swing peak.
    let span = (v.aim_sep_fast_speed_m_s - v.aim_sep_calm_speed_m_s).max(f32::MIN_POSITIVE);
    let f_speed = ((ctx.horizontal_speed_m_s - v.aim_sep_calm_speed_m_s) / span).clamp(0.0, 1.0);
    let sep_m = v.aim_sep_floor_m + (state_m - v.aim_sep_floor_m) * (1.0 - f_speed);

    // 4. **The near field is governed by the city and not by the wheel.** A block-scale budget
    //    is a nonsense at arm's length: 36 m of separation on a point 10 m away is ±61°, off
    //    the screen on both sides, so the old model simply hit the ceiling and handed back the
    //    wheel — which is why making the fan dynamic was a measured NO-OP under 38 m and the
    //    user still had 9.4 m of fan on the roof across the street. The budget is therefore
    //    only fully available once you are looking `aim_sep_full_reach_m` away; nearer than
    //    that it scales with how much city actually lies between you and the point.
    let reach_m = v.aim_sep_full_reach_m.max(f32::MIN_POSITIVE);

    // 5. Metres -> angle, at the distance you are actually looking at. `asin` of the half
    //    chord, the inverse of [`separation_m`]. Note what the ramp does to it: below
    //    `reach_m` the `d` cancels, so the near field is a CONSTANT angle per state
    //    (`asin(sep_m / 2 reach_m)` — 9.6° standing, 11.2° searching at the shipped keys) and
    //    the separation grows linearly with range, exactly as a screen-shaped quantity should.
    //    Nothing under the crosshair means the world has said nothing, and then the wheel is
    //    the whole answer.
    let ceiling_rad = wheel_half_rad(ctx.wheel_rad);
    let want_rad = match ctx.distance_m {
        Some(d) if d.is_finite() && d > 0.0 => {
            let budget_m = sep_m * (d / reach_m).min(1.0);
            (budget_m / (2.0 * d)).clamp(0.0, 1.0).asin()
        }
        _ => ceiling_rad,
    };

    // 6. Permission is a CEILING (*„wie weit auseinander es gehen darf"*) and the floor is a
    //    floor. `min` before `max` so a floor above the ceiling — which `game.ron`'s ordering
    //    guard forbids — still yields the ceiling rather than a NaN-shaped surprise.
    want_rad.min(ceiling_rad).max(floor_rad.min(ceiling_rad))
}

/// What [`effective_spread_rad`] needs to know about one player this tick.
///
/// A struct and not six arguments: five of them are `f32` and a call site that swaps two of
/// them compiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpreadContext {
    /// The player's own setting, already clamped into `game.ron`'s window by
    /// [`Intent::aim_spread_rad`] — the **total** angle between the two rays, which is the
    /// unit `aim_spread_deg` carries. The ceiling on the result is half of it
    /// ([`wheel_half_rad`], the one place that says so).
    pub wheel_rad: f32,
    /// What the body hangs on, from the previous tick.
    pub state: MovementState,
    /// Speed in the plane, m/s — **not** [`Velocity::speed_m_s`].
    pub horizontal_speed_m_s: f32,
    /// The smoothed distance to what the crosshair is on, in metres. `None` = the centre ray
    /// has never hit anything since this player spawned.
    pub distance_m: Option<f32>,
}

/// One step of the outer safety clamp on how fast the fan may open or close — **and the
/// re-clamp that keeps the ramp inside the wheel.**
///
/// The rate limit alone is an escape hatch out of the ceiling: [`effective_spread_rad`] clamps
/// its answer to the wheel, and then a slew that starts from last tick's *wider* angle walks
/// towards it from outside. A player who wheels 44° down to 10° used to get 41/38/35/32/29/26…
/// for eleven ticks — 0.19 s of exactly the fan he just said he did not want. The ceiling is
/// his word (*„wie weit auseinander es gehen **darf**"*, 2026-08-12), so it binds on the tick
/// he turns the wheel and not a fifth of a second later.
///
/// `min` before `max`, the same ordering as [`effective_spread_rad`]: a floor above the ceiling
/// — which `game.ron`'s ordering guard forbids — still yields the ceiling and not a surprise.
///
/// Separated from [`aim`] because a system needs a `World` to test and this needs five numbers.
pub fn slew_spread_rad(
    prev_rad: Option<f32>,
    target_rad: f32,
    step_rad: f32,
    ceiling_rad: f32,
    floor_rad: f32,
) -> f32 {
    let slewed = match prev_rad {
        Some(prev) if prev.is_finite() => prev + (target_rad - prev).clamp(-step_rad, step_rad),
        _ => target_rad,
    };
    slewed.min(ceiling_rad).max(floor_rad.min(ceiling_rad))
}

/// The metric separation of the two landing points at `distance_m`, in metres.
///
/// The chord `2 d sin θ`, which is this repo's own convention for the number (`game.ron`
/// derives its window with it, `tests/vector_aiming.rs` asserts against it). It errs *narrow*
/// against the `2 d tan θ` a flat wall would realise, and narrow is the direction asked for.
pub fn separation_m(half_rad: f32, distance_m: f32) -> f32 {
    2.0 * distance_m * half_rad.sin()
}

/// One step of the low-pass on the aim distance, in **log2 metres**.
///
/// Two decisions live here, both taken from the losing designs by the judges' verdict:
///
/// - **Log space.** The angle is a function of `1/d`, so a constant *relative* rate makes a
///   depth discontinuity feel the same at 12 m and at 300 m; a constant metric rate does not.
/// - **Hold on a miss.** A centre ray that finds nothing is the *absence* of evidence about
///   distance, not evidence of a far one. Holding keeps the near-field fan across a roofline
///   sweep, so the side rays still catch the roof — the one thing the old wide fan was good at
///   — and it keeps sky ticks out of the estimate entirely.
///
/// Returns `None` only while nothing has ever been seen. `settle_s <= 0` disables the filter.
pub fn settle_distance_m(
    prev_m: Option<f32>,
    seen_m: Option<f32>,
    settle_s: f32,
    dt_s: f32,
    min_m: f32,
    max_m: f32,
) -> Option<f32> {
    let Some(seen) = seen_m.filter(|d| d.is_finite() && *d > 0.0) else {
        return prev_m; // a miss says nothing about distance
    };
    let target = seen.clamp(min_m.max(f32::MIN_POSITIVE), max_m.max(min_m));
    let (Some(prev), true) = (prev_m.filter(|d| d.is_finite() && *d > 0.0), settle_s > 0.0) else {
        return Some(target); // first evidence, or no filter: snap
    };
    if !(dt_s.is_finite() && dt_s > 0.0) {
        return Some(prev);
    }
    let (l_prev, l_target) = (prev.log2(), target.log2());
    // Settle exactly instead of asymptotically, so a standing player stops writing the
    // component at all and change detection goes quiet (`docs/lessons/performance.md` rule 1).
    if (l_target - l_prev).abs() < SETTLE_EPS_LOG2 {
        return Some(target);
    }
    let alpha = 1.0 - (-dt_s / settle_s).exp();
    Some((l_prev + (l_target - l_prev) * alpha).exp2())
}

/// When the filtered distance is close enough to snap. 0.002 in log2 is 0.14 % of the
/// distance, which is under 0.01° of angle at the widest setting — an epsilon on a float, not
/// a game value, and therefore not a `game.ron` key (rule 2 is about tunables).
const SETTLE_EPS_LOG2: f32 = 0.002;

/// The per-player memory the model needs: the smoothed aim distance and the angle it resolved
/// to last tick.
///
/// **A component and never a `Resource`** — there is no such thing as *the* player
/// (`docs/multiplayer.md` rule 3). `None` is an explicit "nothing seen yet, snap instead of
/// slew" and not a magic `0.0`, so a fresh player, a respawn and a warp-to-a-new-entity all
/// start clean by construction; a warp of an *existing* entity re-converges within `3 τ`.
///
/// **One writer:** [`aim`]. It is inserted there too, so no foreign spawn site has to
/// remember it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct AimSpread {
    pub distance_m: Option<f32>,
    pub half_rad: Option<f32>,
}

/// The two side directions, `Side::Left` first — the look direction yawed by
/// ±[`side_angle_rad`] **around the camera's up axis**, not around world Y.
///
/// `half_rad` is what [`effective_spread_rad`] resolved this tick: a **half**-angle, already
/// under the wheel's own ceiling and already through [`wheel_half_rad`]. The two rays are
/// therefore `2 · half_rad` apart, and that total is the number the wheel carries.
///
/// Around the camera's up axis, because the spread the user asks for is a *screen* spread:
/// looking 60° down, a yaw around world Y rolls the two rays into the ground on one side and
/// into the sky on the other, and the two markers stop being left and right of the crosshair.
///
/// `look` and `right` are orthonormal by construction — `right` is the horizontal
/// `(cos yaw, 0, -sin yaw)` of `docs/conventions.md`'s axis contract, and the look direction
/// has no component along it at any pitch — so `look·cos ∓ right·sin` is a unit vector
/// without a normalize, at every pitch including straight up and straight down.
pub fn side_dirs(intent: &Intent, half_rad: f32) -> [Vec3; 2] {
    let look = intent.look_dir();
    let (sin_yaw, cos_yaw) = intent.yaw.sin_cos();
    let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
    let (sin, cos) = side_angle_rad(half_rad).sin_cos();
    // Index order is `Side::index()`: left = 0, right = 1. Left of a player looking along -Z
    // is -X, so the left ray leans against `right`.
    [look * cos - right * sin, look * cos + right * sin]
}

/// **`B-008` — is what this side ray found the thing the crosshair is standing on?**
///
/// `F-028`'s fallback used to ask *"did this side ray find anything anchorable?"* and hand the
/// arm the centre ray when the answer was no. In Ashgate the answer is **never** no: the
/// district is 100 % anchorable (the user: *„ueberall! ohne ausnahmen!"*) and the ground is
/// always under the cone, so a side ray that has left the surface the crosshair stands on does
/// not come back empty — it carries on and bites whatever it meets next. Two decisions that are
/// individually right, fighting each other.
///
/// Measured 2026-08-19 from 30 m over the street at `(168.19, ., -50.12)`, looking **straight
/// down**: the crosshair stands on the pavement 30.1 m below, the fan asks for 5.85 m of
/// separation — and the two arms landed **11.50 m** and **10.77 m** off it, on the two roof
/// caps beside the street. From every height over that street the same two roofs win, so the
/// pavement under the crosshair is unhookable and nothing says so (`docs/BUGS.md` B-008,
/// `docs/FINDINGS.md` FIND-116).
///
/// So the question is generalised, not replaced: *"did this side ray find the thing the
/// crosshair is on?"*, and it is answered twice over —
///
/// 1. **The same body is always the same thing.** A facade seen at a grazing angle is still the
///    facade the crosshair is on, however far along it the side ray lands, and straddling it is
///    exactly what the spread is for.
/// 2. **Otherwise, within what the fan asked for.** The model resolved `half_rad` this tick and
///    thereby asked for `d * sin(half_rad)` metres per side; `coherence_k` says how many times
///    that a real hit may be off before it is a different part of town
///    (`game.ron: vector.aim_side_coherence_k`, and the geometry that bounds it is written out
///    there).
///
/// **A crosshair on nothing has nothing to be coherent with.** `F-023`'s promise is that the
/// rope and the marker are one number; a centre ray that found no anchor has no number to be,
/// and the arm that flew 429 m to a tower top the player never pointed at is FIND-116's second
/// measured case. It becomes a clean `F-028` miss with a reason instead.
///
/// Pure — five values in, a `bool` out, no `World` and no `Res`, so it is testable against
/// points somebody typed by hand rather than by asking the code under test the same question
/// twice (`docs/FINDINGS.md` FIND-103). Non-finite input answers `false`, i.e. falls back,
/// which is the safe direction.
pub fn side_hit_is_coherent(
    centre: Option<(Vec3, BodyId)>,
    side: (Vec3, BodyId),
    eye_m: Vec3,
    half_rad: f32,
    coherence_k: f32,
) -> bool {
    let Some((crosshair_m, centre_body)) = centre else {
        return false;
    };
    if side.1 == centre_body {
        return true;
    }
    let asked_m = (crosshair_m - eye_m).length() * half_rad.sin();
    (side.0 - crosshair_m).length() <= coherence_k * asked_m
}

// ===========================================================================================
// `F-025` Bewertungsfunktion / `F-024` Snap auf Q und E — **layer 2**, and it never replaces
// layer 1.
//
// > *„es sollte best match sein"* — the user, 2026-08-18.
//
// `F-002` writes the rule this whole section lives under: *"this layer stays ALWAYS active and
// is never replaceable by the snap system"*. It is kept by construction and not by care: with
// [`PlayerSettings::assist_is_on`] false, [`aim`] never calls a single function below, never
// casts a probe ray, and writes exactly the [`ArmAim`] it wrote before this section existed.
// **0 % is not "almost free aim", it is the same code path.**
//
// ## The three things that decide the design
//
// 1. **The candidate query is a ray sweep, not a region query.** `shared::SpatialIndex` cannot
//    answer "what is anchorable along this line" — `aabb_overlaps` is a stub with no callers
//    (`src/world/index.rs`'s own header says so), and `vector` has no edge to `world` to build
//    a new one there. But a region query would be the wrong shape anyway: it returns points
//    behind walls, and every one of them would then need a line-of-sight ray to be usable at
//    all. The sweep asks avian's BVH the same question `F-002` already trusts (0.21 us a ray),
//    and every answer is a real, unoccluded, anchorable surface by construction.
// 2. **The hemisphere split is `F-023`'s and it is checked twice.** The probe directions of a
//    side are generated on that side's half of the sweep, and the winner is then re-tested
//    against `u · right` — an assertion about the *point*, not about the loop that produced it.
// 3. **What the rope flies at and what the marker shows stay ONE number.** The selection
//    happens *inside* [`aim`], before the single write to [`ArmAim`]; `vector::hook` and
//    `hud::arm_aim` both read that one component and neither of them knows an assist exists.
//    A snap that ran at fire time would be the second reading of the same rule that
//    `user-messages.md` already caught once.
// ===========================================================================================

/// One anchorable point the assist may consider (`F-025`).
///
/// A resolved *world* fact — point, carrier, distance — and not a ray: the scoring function
/// must be testable against a set of points somebody typed in, or it can only be checked by
/// asking the code under test the same question twice (`docs/FINDINGS.md` FIND-103).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimCandidate {
    pub point_m: Vec3,
    pub body: BodyId,
    /// Distance from the eye in metres. Carried rather than recomputed, because the caster
    /// already paid for it.
    pub distance_m: f32,
}

/// Everything [`score_candidate`] needs about the player this tick. Pure data, no `World`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreContext {
    pub eye_m: Vec3,
    /// The crosshair — `F-025` measures the angle deviation against **this**, not against the
    /// arm's own side ray. The hemisphere split is what keeps the two arms apart; scoring both
    /// against their own ray would make the wheel's width a second aim assist.
    pub look: Vec3,
    /// The horizontal-plus-vertical velocity, m/s. The momentum term reads its direction.
    pub velocity_m_s: Vec3,
    /// **The catch HALF-WIDTH**, radians left and right of [`Self::look`] along the camera's
    /// horizontal — `PlayerSettings::assist_catch_deg`. It was the radius of a cone until
    /// 2026-08-19; the knob's numbers did not move, the shape did ([`probe_dirs`]).
    pub catch_rad: f32,
    /// The bodies this player's two arms are on right now (anchored or in flight). `F-025`'s
    /// *"Abwertung des zuletzt genutzten Punktes"*, and the reason it needs no memory of its
    /// own: the point you are hanging on IS the last one you used, and devaluing it is exactly
    /// what stops the pair of hooks pendling between two facades.
    pub in_use: [Option<BodyId>; 2],
}

/// The look basis: `[look, right, up]`, orthonormal at every pitch.
///
/// `right` is the horizontal `(cos yaw, 0, -sin yaw)` of `docs/conventions.md` — the same
/// vector [`side_dirs`] leans against, so a candidate's hemisphere and a side ray's hemisphere
/// are decided by one axis and never by two. `up = right × look` closes the frame.
pub fn look_basis(intent: &Intent) -> [Vec3; 3] {
    let look = intent.look_dir();
    let (sin_yaw, cos_yaw) = intent.yaw.sin_cos();
    let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
    [look, right, right.cross(look)]
}

/// Which hemisphere a direction from the eye belongs to — or `None` on the seam.
///
/// The seam is not a rounding problem to be papered over: a point exactly on the vertical
/// plane through the crosshair belongs to neither arm, and handing it to one of them by the
/// sign of a float is how the two arms end up sharing a point again (FIND-039).
pub fn hemisphere(right: Vec3, offset_m: Vec3) -> Option<Side> {
    let lateral = right.dot(offset_m);
    if lateral < 0.0 {
        Some(Side::Left)
    } else if lateral > 0.0 {
        Some(Side::Right)
    } else {
        None
    }
}

/// The angle between the crosshair and a direction from the eye, radians.
pub fn deviation_rad(look: Vec3, offset_m: Vec3) -> f32 {
    let Some(u) = offset_m.try_normalize() else {
        return f32::INFINITY;
    };
    u.dot(look).clamp(-1.0, 1.0).acos()
}

/// The probe directions for **one** hemisphere: `steps` unit vectors on the **screen-horizontal
/// line through the crosshair**, out to [`PlayerSettings::assist_catch_deg`] on that side.
///
/// > *„die seile sollen immer auf der horzontalen fest sein. also wenn das fadenkreuz 0, 0 ist
/// > sollen die seile nur auf der x achse snappen (objekte finden) also seitlich! dann ist es
/// > auch besser einzuschätzen."* — the user, 2026-08-19
///
/// **The sweep used to be a 2D cone** — `rings × probes_per_ring` directions spread over the
/// half-disc around the look direction (`docs/FINDINGS.md` FIND-104) — and it could therefore
/// hand an arm a point **above or below** where the player was looking. Measured over the
/// shipped map before this changed: **9.23° / 3.41 m** of camera-vertical deviation on a
/// published aim point. His reason for wanting it gone is the requirement and not a preference:
/// *„dann ist es auch besser einzuschätzen"* — a snap that moves in two axes cannot be
/// predicted, one that moves along a single named axis can be learned. **Legibility beats a
/// marginally better anchor.**
///
/// ⚠️ **"Horizontal" is the CAMERA's horizontal at every pitch, not the world's.** The axis is
/// `basis[1]`, the same `(cos yaw, 0, -sin yaw)` that [`side_dirs`] leans against, and the plane
/// it spans with `look` is the plane whose camera-space `y` is exactly zero. So every probe —
/// like every side ray and like the centre ray — projects onto the **same screen row as the
/// crosshair**, at every pitch including straight down, and a snap moves the marker sideways and
/// by nothing else. A *world*-horizontal sweep would satisfy the sentence at pitch 0 and violate
/// it everywhere else (`docs/QUESTIONS.md` Q-040 carries the alternative and its rollback point).
///
/// Steps at `catch_rad * (i+1)/steps`, so the outermost probe sits exactly on the catch cone's
/// edge and none of them sits at `theta = 0`, where [`hemisphere`] would refuse it — the
/// crosshair's own direction is the incumbent and is never a candidate. `look` and `right` are
/// orthonormal by construction ([`side_dirs`] says why), so the sum is a unit vector without a
/// normalize and its camera-space `y` is an exact zero rather than a rounded one. Nothing is
/// allocated: the caller casts as it iterates.
pub fn probe_dirs(
    basis: [Vec3; 3],
    catch_rad: f32,
    steps: u32,
    side: Side,
) -> impl Iterator<Item = Vec3> {
    let [look, right, _up] = basis;
    // Left of a player looking along -Z is -X, exactly as in [`side_dirs`].
    let sign = match side {
        Side::Left => -1.0,
        Side::Right => 1.0,
    };
    let steps = steps.max(1);
    (0..steps).map(move |i| {
        let theta = catch_rad * (i + 1) as f32 / steps as f32;
        let (sin_t, cos_t) = theta.sin_cos();
        look * cos_t + right * (sign * sin_t)
    })
}

/// **`F-025`'s weighted score for one candidate.** Higher is better; the reward terms are each
/// normalised to `0..1` and weighted by the file's five keys, and the recency term is
/// subtracted.
///
/// The five factors are the backlog's, verbatim (`docs/backlog/gameplay.ron`, `F-025`):
/// angle deviation to the crosshair **45 %**, momentum preservation **25 %**, height advantage
/// **15 %**, distance in the usable mid-range **10 %**, devaluation of the point last used
/// **5 %**. *"Alle Gewichte liegen in der Config und sind ohne Codeaenderung anpassbar."* —
/// they do: `game.ron: vector.assist_score_*`, and there is no `serde(default)` under any of
/// them, so a missing weight crashes on load instead of quietly scoring zero.
///
/// **Pure.** No `Res`, no `World`, no randomness — so a test can hand it points it typed out
/// itself and check the ordering against arithmetic it did by hand, instead of asking the
/// selection function whether the selection function is right (FIND-103).
pub fn score_candidate(v: &VectorTuning, ctx: &ScoreContext, c: &AimCandidate) -> f32 {
    let offset = c.point_m - ctx.eye_m;
    let Some(u) = offset.try_normalize() else {
        return f32::NEG_INFINITY;
    };

    // 45 % — how far off the crosshair it is, as a fraction of the catch half-width the player has
    // dialled. Straight down the crosshair is 1, at the rim it is 0.
    let deviation = u.dot(ctx.look).clamp(-1.0, 1.0).acos();
    let angle = 1.0 - (deviation / ctx.catch_rad.max(f32::MIN_POSITIVE)).clamp(0.0, 1.0);

    // 25 % — does hooking there CONTINUE the flight or brake it? A rope pulls you towards the
    // anchor, so the useful number is the cosine between the flight direction and the
    // direction to the point. 0.5 is the midpoint of the axis and means "no evidence": below
    // `assist_momentum_min_speed_m_s` there is no trajectory to preserve, and a 0 there would
    // be the claim that standing still makes every anchor a braking one.
    let speed = ctx.velocity_m_s.length();
    let momentum = if speed >= v.assist_momentum_min_speed_m_s && speed > 0.0 {
        0.5 * (1.0 + u.dot(ctx.velocity_m_s / speed))
    } else {
        0.5
    };

    // 15 % — height advantage. Same 0.5 midpoint: level with the eye is neither a gain nor a
    // loss, `assist_height_full_m` above is the full mark, the same below is zero.
    let rise = (c.point_m.y - ctx.eye_m.y) / v.assist_height_full_m.max(f32::MIN_POSITIVE);
    let height = 0.5 + 0.5 * rise.clamp(-1.0, 1.0);

    // 10 % — the usable middle of the range: a triangle centred on `assist_dist_ideal_m`.
    let off_band =
        (c.distance_m - v.assist_dist_ideal_m).abs() / v.assist_dist_span_m.max(f32::MIN_POSITIVE);
    let distance = 1.0 - off_band.clamp(0.0, 1.0);

    // 5 %, subtracted — the point already in use.
    let in_use = ctx.in_use.iter().flatten().any(|b| *b == c.body);

    v.assist_score_angle_w * angle
        + v.assist_score_momentum_w * momentum
        + v.assist_score_height_w * height
        + v.assist_score_distance_w * distance
        - if in_use { v.assist_score_recent_w } else { 0.0 }
}

/// **The margin a candidate has to beat the player's own aim by**, given `F-016`'s strength
/// knob in per cent.
///
/// This one function is `F-024`'s three modes, and it is why there is no fourth enum for the
/// player to dial: at **0 %** the caller never gets here at all
/// ([`PlayerSettings::assist_is_on`]) and the free ray is the whole answer — **FREI**. At
/// **100 %** the margin is 0 and the best candidate always wins — **SNAP**. Everything between
/// is **ASSISTIERT**, `F-024`'s default and the mode where the assist only fires when it has
/// something clearly better to offer.
///
/// Linear in the slider on purpose: he asked for the knobs so he could *report a number back*
/// (*„damit ich testen kann was am besten wäre"*), and a number he reports has to mean one
/// thing.
pub fn required_margin(margin_full: f32, strength_pct: f32) -> f32 {
    margin_full * (1.0 - strength_pct.clamp(0.0, 100.0) / 100.0)
}

/// Pick the best candidate for one hemisphere, or `None` to keep what the player aimed at.
///
/// **Every filter is applied to the CANDIDATE and not to the loop that produced it**: a point
/// is kept only if it really lies in this arm's hemisphere and really lies inside the catch
/// sweep. A probe sweep that generated a direction outside its own side would be caught here
/// rather than silently handing the left arm a point on the right.
///
/// `incumbent` is what free aim found for this arm — `None` when the player is pointing at
/// something that holds nothing (a titan, an untagged wall) or at nothing at all. **That case
/// is exactly `B-007`**: there is no incumbent to beat, so any candidate wins regardless of
/// strength, and the arm reaches the wall beside the body that was blocking it.
pub fn pick_best(
    v: &VectorTuning,
    ctx: &ScoreContext,
    side: Side,
    right: Vec3,
    candidates: &[AimCandidate],
    incumbent: Option<AimCandidate>,
    strength_pct: f32,
) -> Option<AimCandidate> {
    let mut best: Option<(f32, AimCandidate)> = None;
    for c in candidates {
        let offset = c.point_m - ctx.eye_m;
        if hemisphere(right, offset) != Some(side) {
            continue;
        }
        if deviation_rad(ctx.look, offset) > ctx.catch_rad {
            continue;
        }
        let score = score_candidate(v, ctx, c);
        if best.is_none_or(|(top, _)| score > top) {
            best = Some((score, *c));
        }
    }
    let (score, winner) = best?;
    match incumbent {
        // The player is already pointing at something that holds. It keeps the arm unless the
        // candidate is better by the margin his strength slider bought.
        Some(free) => {
            let margin = required_margin(v.assist_margin_full, strength_pct);
            (score >= score_candidate(v, ctx, &free) + margin).then_some(winner)
        }
        // Nothing to beat — free aim found no anchor at all. `F-028`'s fallback used to end
        // here as a silent miss; now it ends on the best point in the hemisphere.
        None => Some(winner),
    }
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
    mut commands: Commands,
    data: Res<GameData>,
    time: Res<Time<Fixed>>,
    settings: Option<Res<PlayerSettings>>,
    space: SpatialQuery,
    bodies: Query<(&Body, Option<&BodyId>)>,
    mut players: Query<(
        Entity,
        &Intent,
        &Transform,
        &Velocity,
        &MovementState,
        &mut AimPoint,
        &mut ArmAim,
        Option<&mut AimSpread>,
        Option<&Hook>,
        Has<LocalPlayer>,
    )>,
) {
    let v = &data.game.vector;
    let range_m = v.hook_range_m;
    let eye_height_m = data.game.player.eye_height_m;
    let dt_s = time.delta_secs();

    for (
        player,
        intent,
        transform,
        velocity,
        state,
        mut point,
        mut arms,
        spread,
        hook,
        is_local,
    ) in &mut players
    {
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
        //
        // Since 2026-08-18 that setting is the **ceiling** and not the angle: how far the two
        // rays really open is solved here, out of the centre ray that was just cast, out of
        // what the body hangs on and out of how fast it is going ([`effective_spread_rad`]).
        // The distance costs nothing — `cast` builds the point as `eye + dir * hit.distance`,
        // so its length back is the hit distance exactly, not an approximation.
        let wheel_rad = intent.aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg);
        let previous = spread.as_deref().copied().unwrap_or_default();
        let seen_m = centre.point_m.map(|p| (p - eye_m).length());
        let distance_m = settle_distance_m(
            previous.distance_m,
            seen_m,
            v.aim_spread_settle_s,
            dt_s,
            v.min_rope_m,
            range_m,
        );
        let target_rad = effective_spread_rad(
            v,
            SpreadContext {
                wheel_rad,
                state: *state,
                        horizontal_speed_m_s: Vec3::new(velocity.0.x, 0.0, velocity.0.z).length(),
                distance_m,
            },
        );
        // The outer safety clamp, on top of the distance filter: one tick may never move the
        // fan more than `aim_spread_slew_deg_s / tick rate`, so a single-tick depth blip is a
        // slide and not a snap. `None` — a player who has never aimed — snaps instead of
        // sweeping up from a stale angle.
        let step_rad = v.aim_spread_slew_deg_s.to_radians() * dt_s.max(0.0);
        let spread_rad = slew_spread_rad(
            previous.half_rad,
            target_rad,
            step_rad,
            wheel_half_rad(wheel_rad),
            v.aim_spread_floor_deg.to_radians(),
        );
        let resolved = AimSpread { distance_m, half_rad: Some(spread_rad) };
        match spread {
            // `set_if_neq` for the same reason as `AimPoint` below: a standing player settles
            // exactly (`SETTLE_EPS_LOG2`) and then stops writing at all.
            Some(mut carried) => {
                carried.set_if_neq(resolved);
            }
            None => {
                commands.entity(player).insert(resolved);
            }
        }
        // **`F-016` / `F-024`: the two knobs, read here and nowhere else.**
        //
        // `Option<Res<..>>` because a test app may run without a settings screen, and
        // `Has<LocalPlayer>` because [`PlayerSettings`] is *this machine's* preference: a
        // remote player's aim may never be bent by the local user's slider
        // (`docs/multiplayer.md` rule 3 — the resource is admissible only as long as nothing
        // in it decides another player's simulation). The day the knobs travel, they travel in
        // [`Intent`] beside `aim_spread_deg`, and this is the one line that changes.
        //
        // ⚠️ **`assist_is_on() == false` is not a shortcut, it is `F-002`'s guarantee.** With
        // either knob at 0 the block below is skipped in full: no probe ray is cast, no score
        // is computed, and `sides` is byte-for-byte what this system produced before `F-024`
        // existed.
        let assist = settings
            .as_deref()
            .filter(|_| is_local)
            .filter(|s| s.assist_is_on())
            .copied();
        let basis = look_basis(intent);
        let score_ctx = assist.map(|s| ScoreContext {
            eye_m,
            look: basis[0],
            velocity_m_s: velocity.0,
            catch_rad: s.assist_catch_deg().to_radians(),
            // What each arm is on right now — `F-025`'s 5 % devaluation of the point last
            // used, with no memory of its own to go stale (see [`ScoreContext::in_use`]).
            in_use: Side::ALL.map(|side| {
                hook.map(|h| h.arm(side).state).and_then(|st| match st {
                    HookState::Anchored { body, .. } | HookState::Flying { body, .. } => Some(body),
                    HookState::Idle | HookState::Retracting => None,
                })
            }),
        });

        let dirs = side_dirs(intent, spread_rad);
        let sides = Side::ALL.map(|side| {
            let found = cast(&space, &bodies, player, eye_m, dirs[side.index()], range_m);
            // **The fallback, and it is the difference between a feature and a regression.**
            // A side ray that does not find the thing the crosshair is on — off the roof edge,
            // into the sky, onto an untagged wall, or **onto whatever it met next** — hands the
            // arm the centre ray instead. Aiming at a lone tower has to keep working exactly as
            // well as it did when both arms shared one point; without this line the spread
            // would cost hit rate on every target narrower than the spread itself.
            //
            // ⚠️ **The test used to be "did it find anything anchorable", and in Ashgate that
            // is always true** — the district is 100 % anchorable and the ground is always
            // under the cone, so an arm aimed straight down bit a roof cap beside the street
            // and never the pavement the crosshair stood on (`B-008`).
            // [`side_hit_is_coherent`] is the same question asked so that a world without holes
            // can still answer it.
            //
            // Resolved HERE and not at fire time: what is written into `ArmAim` is what the
            // rope flies at and what the HUD draws, and a rule applied twice in two files is
            // how a marker and a rope end up in two places (`user-messages.md`, 2026-08-12).
            let free = match anchor_target(&found) {
                Some(hit)
                    if side_hit_is_coherent(
                        anchor_target(&centre),
                        hit,
                        eye_m,
                        spread_rad,
                        v.aim_side_coherence_k,
                    ) =>
                {
                    found
                }
                _ => centre,
            };

            // **Layer 2 (`F-024`/`F-025`), and it only ever runs on top of layer 1.**
            let (Some(s), Some(ctx)) = (assist, score_ctx) else {
                return free;
            };
            let mut candidates = Vec::with_capacity(v.assist_probe_steps as usize);
            for dir in probe_dirs(basis, ctx.catch_rad, v.assist_probe_steps, side) {
                let probe = cast(&space, &bodies, player, eye_m, dir, range_m);
                if let Some((point_m, body)) = anchor_target(&probe) {
                    candidates.push(AimCandidate {
                        point_m,
                        body,
                        distance_m: (point_m - eye_m).length(),
                    });
                }
            }
            let incumbent = anchor_target(&free).map(|(point_m, body)| AimCandidate {
                point_m,
                body,
                distance_m: (point_m - eye_m).length(),
            });
            match pick_best(
                v,
                &ctx,
                side,
                basis[1],
                &candidates,
                incumbent,
                s.assist_strength_pct,
            ) {
                // The winner is published as a full [`AimPoint`] — it came out of
                // [`anchor_target`], so `anchorable` is true by construction and `hud` and
                // `vector::hook` cannot tell it apart from a free-aim hit. That is the point:
                // one number for the rope and the marker (`F-023`).
                Some(best) => AimPoint {
                    point_m: Some(best.point_m),
                    body: Some(best.body),
                    anchorable: true,
                },
                None => free,
            }
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
    // half the map (`system_param.rs:111-120`). The `anchorable` mask is still asked AFTER
    // the hit, never before it (`F-023`) — the one thing filtered up front is **another
    // player**, because he is not a surface and blocking on him reads as a miss
    // (`shared::AIM_RAY_SEES`, `docs/BUGS.md` B-010). Excluding only `player` was enough
    // while two bodies shoved each other apart; since F-163a they stand inside each other
    // and `solid: true` answered the ray at the caster's own eye, distance 0.
    let filter = SpatialQueryFilter::from_excluded_entities([player]).with_mask(AIM_RAY_SEES);
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
