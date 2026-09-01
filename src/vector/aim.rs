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
    AimPoint, ArmAim, Body, BodyId, Hook, HookState, Intent, LocalPlayer,
    PlayerSettings, Side, Velocity, AIM_RAY_SEES,
};

use super::hook::anchor_target;
use super::hookable::{is_hookable, HookableSurfaces};

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
// 2. **The sweep is one screen-horizontal LINE through the crosshair, and there is one winner
//    on it.** It used to be two — one per hemisphere, the two ends of `F-023`'s fan — until the
//    user retired the fan on 2026-08-23 (`docs/QUESTIONS.md` Q-048). The line itself is his and
//    it stays (`docs/FINDINGS.md` FIND-133): a snap that moves in two axes cannot be predicted.
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
    /// The crosshair — `F-025` measures the angle deviation against **this**, and since the
    /// fan was retired there is nothing else to measure it against: it is the one direction the
    /// player aimed and both arms answer to it.
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
/// `right` is the horizontal `(cos yaw, 0, -sin yaw)` of `docs/conventions.md` — the axis the
/// assist's probe sweep steps along, so "sideways" is one axis and never two. `up = right ×
/// look` closes the frame.
pub fn look_basis(intent: &Intent) -> [Vec3; 3] {
    let look = intent.look_dir();
    let (sin_yaw, cos_yaw) = intent.yaw.sin_cos();
    let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
    [look, right, right.cross(look)]
}

/// The angle between the crosshair and a direction from the eye, radians.
pub fn deviation_rad(look: Vec3, offset_m: Vec3) -> f32 {
    let Some(u) = offset_m.try_normalize() else {
        return f32::INFINITY;
    };
    u.dot(look).clamp(-1.0, 1.0).acos()
}

/// The probe directions for **one side** of the crosshair: `steps` unit vectors on the **screen-horizontal
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
/// `basis[1]`, the `(cos yaw, 0, -sin yaw)` of `docs/conventions.md`'s axis contract, and the plane
/// it spans with `look` is the plane whose camera-space `y` is exactly zero. So every probe —
/// like every side ray and like the centre ray — projects onto the **same screen row as the
/// crosshair**, at every pitch including straight down, and a snap moves the marker sideways and
/// by nothing else. A *world*-horizontal sweep would satisfy the sentence at pitch 0 and violate
/// it everywhere else (`docs/QUESTIONS.md` Q-040 carries the alternative and its rollback point).
///
/// Steps at `catch_rad * (i+1)/steps`, so the outermost probe sits exactly on the catch cone's
/// edge and none of them sits at `theta = 0` — the crosshair's own direction is the incumbent
/// and is never a candidate. `look` and `right` are orthonormal by construction — `right` has no
/// component along `look` at any pitch — so the sum is a unit vector without a
/// normalize and its camera-space `y` is an exact zero rather than a rounded one. Nothing is
/// allocated: the caller casts as it iterates.
pub fn probe_dirs(
    basis: [Vec3; 3],
    catch_rad: f32,
    steps: u32,
    side: Side,
) -> impl Iterator<Item = Vec3> {
    let [look, right, _up] = basis;
    // Left of a player looking along -Z is -X (`docs/conventions.md`, the axis contract).
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

/// Pick the best candidate on the crosshair's row, or `None` to keep what the player aimed at.
///
/// **One winner for both arms.** Until 2026-08-23 this ran twice, once per hemisphere, and the
/// two answers were the two ends of `F-023`'s fan. The user retired the fan — *„einfach da wo
/// ich hinschau (also fadenkreuz) geht das seil hin"* — so the candidate set is now the whole
/// screen-horizontal sweep, left and right of the crosshair together, and the point that wins
/// it is the point both ropes fly to (`docs/QUESTIONS.md` Q-048).
///
/// **The filter is still applied to the CANDIDATE and not to the loop that produced it**: a
/// point is kept only if it really lies inside the catch sweep, so a probe sweep that generated
/// a direction outside the width the player dialled is caught here rather than trusted.
///
/// `incumbent` is what free aim found — `None` when the player is pointing at something that
/// holds nothing (a titan, an untagged wall) or at nothing at all. **That case is exactly
/// `B-007`**: there is no incumbent to beat, so any candidate wins regardless of strength, and
/// the arms reach the wall beside the body that was blocking them.
pub fn pick_best(
    v: &VectorTuning,
    ctx: &ScoreContext,
    candidates: &[AimCandidate],
    incumbent: Option<AimCandidate>,
    strength_pct: f32,
) -> Option<AimCandidate> {
    let mut best: Option<(f32, AimCandidate)> = None;
    for c in candidates {
        let offset = c.point_m - ctx.eye_m;
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
        // The player is already pointing at something that holds. It keeps the arms unless the
        // candidate is better by the margin his strength slider bought.
        Some(free) => {
            let margin = required_margin(v.assist_margin_full, strength_pct);
            (score >= score_candidate(v, ctx, &free) + margin).then_some(winner)
        }
        // Nothing to beat — free aim found no anchor at all. `F-028`'s fallback used to end
        // here as a silent miss; now it ends on the best point on the crosshair's row.
        None => Some(winner),
    }
}

/// Writes [`AimPoint`] and [`ArmAim`] for every player, once per fixed step.
///
/// Runs in `SimulationSystems::PostStep` — **after** `Integrate`, and that is not a detail.
///
/// The ray starts at [`eye`], which is `translation + Y·eye_height_m`, and
/// `render::attach_camera` hangs the camera on the player at exactly that offset. So the ray's
/// origin **is** the camera's position and its direction **is** the camera's forward — but only
/// if both are read at the same instant. In `World`, before `Integrate`, they were one step of
/// eye travel apart, the HUD drew the answer through a camera that had already moved, and the
/// error was an angle that diverged as the player closed on the surface: median 14 px and up to
/// **420 px** over one boost (`docs/FINDINGS.md` FIND-217, `docs/BUGS.md` B-029).
///
/// `vector::hook` reads [`ArmAim`] in `Intent`, i.e. one stage after this system ran at the end
/// of the previous tick — from a `Transform` no system has touched in between, because
/// `Integrate` is the only writer of it and it does not sit between those two points. The rope
/// therefore fires at the very point the frame the player was looking at had drawn.
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
    settings: Option<Res<PlayerSettings>>,
    // `Q-078`'s switch. `Res` and not `Option<Res>`: `VectorPlugin` inits it, and unlike
    // `PlayerSettings` it is not a preference a test app may sensibly run without — a missing
    // one would silently mean "hookable by default" and hide the day somebody forgets to
    // register it.
    hookable: Res<HookableSurfaces>,
    space: SpatialQuery,
    bodies: Query<(&Body, Option<&BodyId>)>,
    mut players: Query<(
        Entity,
        &Intent,
        &Transform,
        &Velocity,
        &mut AimPoint,
        &mut ArmAim,
        Option<&Hook>,
        Has<LocalPlayer>,
    )>,
) {
    let v = &data.game.vector;
    let range_m = v.hook_range_m;
    let eye_height_m = data.game.player.eye_height_m;

    for (player, intent, transform, velocity, mut point, mut arms, hook, is_local) in &mut players
    {
        let eye_m = eye(transform.translation, eye_height_m);

        // **One ray, and both arms get its answer.** The user, 2026-08-23: *„dann das
        // auseinander mit q und e kann weg. einfach da wo ich hinschau (also fadenkreuz) geht
        // das seil hin."* Until that sentence this cast three rays — a centre one for the
        // crosshair and two side rays at ±`effective_spread_rad` that were what Q and E flew
        // at (`F-023`). The fan is gone with all sixteen of its keys; the crosshair's own hit
        // is the whole answer for both arms (`docs/QUESTIONS.md` Q-048).
        //
        // One `cast_ray` per player per tick: measured 0.21 us against 4000 blocks (module
        // header). It is a BVH walk, not an iteration over the world (§11).
        let centre =
            cast(&space, &bodies, *hookable, player, eye_m, intent.look_dir(), range_m);

        // **`F-016` / `F-024`: the two knobs, read here and nowhere else.**
        //
        // `Option<Res<..>>` because a test app may run without a settings screen, and
        // `Has<LocalPlayer>` because [`PlayerSettings`] is *this machine's* preference: a
        // remote player's aim may never be bent by the local user's slider
        // (`docs/multiplayer.md` rule 3 — the resource is admissible only as long as nothing
        // in it decides another player's simulation). The day the knobs travel, they travel in
        // [`Intent`] and this is the one line that changes.
        //
        // ⚠️ **`assist_is_on() == false` is not a shortcut, it is `F-002`'s guarantee.** With
        // either knob at 0 the block below is skipped in full: no probe ray is cast, no score
        // is computed, and both arms carry exactly the centre ray.
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

        // **Layer 2 (`F-024`/`F-025`), and it only ever runs on top of layer 1** — one sweep,
        // one winner, one point for both arms. `probe_dirs` still takes a [`Side`]: it is the
        // sign of the step along the camera's horizontal, and casting it for both sides is the
        // *whole* screen-horizontal line through the crosshair. That line is the user's other
        // standing requirement and it survives the fan (`docs/FINDINGS.md` FIND-133: *„die
        // seile sollen immer auf der horzontalen fest sein"*) — **the assist stays, the fan
        // goes.** Same 2 x `assist_probe_steps` rays as before, two fewer than the fan cast.
        let resolved = match (assist, score_ctx) {
            (Some(s), Some(ctx)) => {
                let mut candidates =
                    Vec::with_capacity(2 * v.assist_probe_steps as usize);
                for side in Side::ALL {
                    for dir in probe_dirs(basis, ctx.catch_rad, v.assist_probe_steps, side) {
                        let probe =
                            cast(&space, &bodies, *hookable, player, eye_m, dir, range_m);
                        if let Some((point_m, body)) = anchor_target(&probe) {
                            candidates.push(AimCandidate {
                                point_m,
                                body,
                                distance_m: (point_m - eye_m).length(),
                            });
                        }
                    }
                }
                let incumbent = anchor_target(&centre).map(|(point_m, body)| AimCandidate {
                    point_m,
                    body,
                    distance_m: (point_m - eye_m).length(),
                });
                match pick_best(v, &ctx, &candidates, incumbent, s.assist_strength_pct) {
                    // The winner is published as a full [`AimPoint`] — it came out of
                    // [`anchor_target`], so `anchorable` is true by construction and `hud` and
                    // `vector::hook` cannot tell it apart from a free-aim hit. That is the
                    // point: one number for the ropes and the markers.
                    Some(best) => AimPoint {
                        point_m: Some(best.point_m),
                        body: Some(best.body),
                        anchorable: true,
                    },
                    None => centre,
                }
            }
            _ => centre,
        };

        // `set_if_neq` and not a plain assignment: a standing player aims at the same point
        // every tick, and a `Mut` that is written unconditionally triggers change detection
        // 60 times a second for nothing (`docs/lessons/performance.md`, rule 1).
        //
        // ⚠️ **The crosshair keeps the RAW centre ray and the arms carry the resolved one.**
        // They are the same value with the assist off; with it on the crosshair still says
        // where the player is pointing while the markers and the ropes say where the hooks
        // go, and a snap that moved the crosshair would be the game aiming for him.
        point.set_if_neq(centre);
        arms.set_if_neq(ArmAim { arms: [resolved, resolved] });
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
    hookable: HookableSurfaces,
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

    // **`Q-078`: the category is asked, not the tag.** Until 2026-08-27 this line read
    // `body.is_some_and(|b| b.mask.contains(BodyMask::ANCHORABLE))` — i.e. `F-003`, "no hook
    // on an untagged part". The user cancelled that rule and asked for the switch instead
    // (`vector::hookable`), so the field keeps its meaning — *may a hook take this* — and only
    // the answer changed. Every kind says yes today.
    AimPoint {
        point_m: Some(eye_m + direction * hit.distance),
        body: id,
        anchorable: is_hookable(hookable, body),
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
