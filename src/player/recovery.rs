//! `F-012` — **the recovery.** The half that gets forgotten, and the only half that helps once
//! something has already gone wrong.
//!
//! The user, 2026-08-27: *„unsichtbare wand + wenn man runterfaellt wegen bug teleport man
//! zurueck!"* — **wegen bug**. He asked for this one for the case the fence does not hold.
//!
//! So this file knows nothing about [`crate::world::bounds`], and that is a stricter statement
//! than it looks: it reads `Map::size_m` and two numbers out of `maps.ron: bounds`, and it
//! never asks where a fence panel stands, how thick it is, how tall it is, or whether one was
//! built at all. **A map with `bounds::build_bounds` deleted out of the plugin still recovers
//! everybody** — which is the whole point of the second mechanism, and it is what
//! `tests/player.rs::f012_a_fence_far_outside_the_map_is_not_a_bigger_map` measures by moving
//! the fence 500 m and getting the same answers back.
//!
//! ## What "out of the world" is, and it is TWO questions and not one
//!
//! 🔴 **The first build asked only about depth, and leaving is not only downward.** Measured
//! 2026-08-28, all three of them one hole:
//!
//! - `fence_top_m` is 200 m; the gear climbs to **657 m** from the ground and **901 m** from
//!   the wall, on **0.72 % of one tank**. You fly over the fence.
//! - Two held keys from the spawn point — `W` + `Shift`, look up, then look out — put the
//!   player 284 m up and **outside** the map, and roughly **ten seconds of falling through
//!   nothing** followed before the plane at -300 m caught him.
//! - The fence's own top face is a solid, invisible, standable ring 10 m wide at y = 200 that
//!   runs the whole way round the district. A body put on it rested at exactly 200.000 m and
//!   was still there 14 s later; the depth question never fires 500 m above the plane. Parked
//!   outside the world, indefinitely. ⚠️ And the sentence that used to stand here — *"and
//!   `record_safe_ground` correctly refuses to record him"* — was **false at the lip**, which
//!   is the whole of the bug below: with `fence_margin_m: 0.0` the inner metre of that ring lay
//!   inside the footprint and got recorded as home.
//!
//! So [`out_of_the_world`] asks both, and it is **one function** that both systems call —
//! never the same question implemented twice in two schedules (`CLAUDE.md` rule 5's
//! corollary; that shape cost this project `FIND-103` and the HUD's stale stance):
//!
//! 1. **Under the plane.** `bounds.recovery_plane_y_m` is -300 m and the deepest block
//!    underside in the district is -4.2 m, so nothing legitimate crosses it.
//! 2. **Past the edge.** Outside `map.size_m` horizontally, **at any height**. The ground
//!    stops exactly at `size_m / 2` — one metre further out there is no collider at any height
//!    ever — so a body whose origin is past it is over nothing whether he is at -45 m or at
//!    900 m.
//!
//! ### The grace at the edge is ZERO, and the FENCE is what pays for it
//!
//! There is no tolerance in [`out_of_the_world`], because any tolerance of `g` metres is a
//! standable ring `g` metres wide somewhere. Zero is affordable only because the fence stands
//! **outside** the footprint and holds a legitimate body inside it, and that is a geometry
//! problem with two measured bounds — `maps.ron: bounds.fence_margin_m` carries the derivation:
//!
//! ```text
//!   fence_rest_reach_m 0.10  <  fence_margin_m 0.18  <  player.radius_m 0.35
//! ```
//!
//! 🔴 **And "just stand the fence outside the footprint" is NOT enough, which is what the
//! second refutation of 2026-08-29 measured.** A capsule does not rest on the point under its
//! origin; it rests on its bottom sphere, and that sphere reaches over the lip. With
//! `fence_margin_m: 0.0` the fence's inner lip stood exactly on `hx`, and `|x| > hx` is
//! **strict** — so a body put at `(350, 201, 0)` rested at `(349.93, 199.99, 0)`, `Grounded`,
//! *in the world*, and got recorded as home. 48 of 48 stances did. Moving the fence out by one
//! ULP would have changed nothing: the number that has to be cleared is how far a capsule
//! slides back over the lip before friction holds it, and that is **0.0892 m**, not 3e-5.
//!
//! `tests/player.rs::f012_nothing_that_can_rest_on_the_fence_rests_inside_the_map` measures the
//! lower bound, `f012_a_body_driven_into_the_fence_at_top_speed_is_never_recovered` the upper
//! one (`-0.3500 m` at the clamp, to four decimals), and
//! `tests/data.rs::f012_the_fence_stands_within_one_body_radius_of_the_map_edge` guards the
//! chain when somebody edits `maps.ron`.
//!
//! ### And therefore the fence needs no great height
//!
//! Raising `fence_top_m` is **not** the fix and never was: a taller fence still has a top face
//! to stand on, and the gear reaches 901 m, so every number is a number somebody beats. The
//! fence's job is horizontal — it is what normal play runs into. Everything that gets over it
//! is caught by question 2 above, at any height, and that is the whole of the second half the
//! user asked for.
//!
//! ## What "last safe" means, and where it is stored
//!
//! [`SafeGround`] is a **component on the player** — never a `Resource`, never reached through
//! `.single()` (`CLAUDE.md` rule 4, `docs/multiplayer.md`): every player carries his own, and
//! it is written and read per entity so that the day two of them fall nothing has to be
//! disentangled.
//!
//! It records the player's own `Transform.translation` on **the last tick on which both** held
//! at once:
//!
//! 1. [`MovementState::Grounded`] — his feet were on something the solver agreed with. Not
//!    "he was slow", not "he had a rope": the ground contact comes out of the collider
//!    (`super::integrator::readback`), and this file re-derives none of it — it reads the
//!    answer that domain already gave (`CLAUDE.md` rule 5's corollary).
//!    ⚠️ There is deliberately **no** "and he was not falling past it" condition here, and
//!    that is a decision the control run took out of my hands. A velocity gate of one tick of
//!    gravity was written first, against the measurement that a body sliding off the fence's
//!    top face stays `Grounded` the whole way down (recorded at 0.2811 m back over the lip).
//!    Then it could not be made to go red in any legal configuration — the body condition
//!    below already covers every stance it would have refused — and a fix without a failing
//!    test is a guess (`CLAUDE.md` rule 5). It was removed rather than shipped.
//! 2. 🔴 **The place he would be PUT DOWN is not [`out_of_the_world`], and it is asked about
//!    his WHOLE BODY** — not about the place his origin is standing. That is [`recovery_destination`], the one function that knows what a recovery
//!    does with a recorded point, and asking about *it* is what makes the invariant this
//!    header claims an invariant the code **enforces** instead of one the geometry happens to
//!    satisfy. It did not, once: the header said "the place you get sent back to can never
//!    itself be a place you get sent back from" while the shipped game logged
//!    *"back to Vec3(350.0, 200.49994, 0.0)"* — the top of the fence — and delivered every
//!    later recovery of the session there.
//!
//!    ⚠️ And the guard for it may **not** be a second sweep of [`out_of_the_world`]: that is
//!    the same function this system already asked about the same point, and two implementations
//!    of one question cannot disagree (`CLAUDE.md` rule 5, the fourth shape — the first draft
//!    of `tests/player.rs::f012_the_ground_he_is_sent_back_to_is_never_ground_he_can_be_sent_
//!    back_from` did exactly that and passed on the broken build). The oracle there is the
//!    map's own **planned geometry**: a home has to be inside the footprint and at or under
//!    the highest thing `world::map` planned, and the fence is not in that plan.
//!
//!    The body half is the other measurement of that day. A capsule standing within
//!    `player.radius_m` of the map's edge may be standing on the ground, or on the **inner lip
//!    of the fence's top face** a `fence_margin_m` further out — and from a position alone the
//!    two cannot be told apart, because the capsule that could be resting on either is the
//!    same size. A body sliding off that lip was recorded at `(-299.74, 199.886, 0)`, 200 m up
//!    with nothing under it. So the question is asked at the far corner of the body's own
//!    footprint, and both are refused. Nothing is lost: a player pressed against the fence is
//!    recorded a body-radius further in, one tick earlier, and that stance is real ground.
//!
//! 🔴 **And it gates on `map.size_m` ALONE.** It read `map.size_m * 0.5 + fence_margin_m` —
//! the FENCE's footprint, not the map's — under a header that claimed to know nothing about
//! `world::bounds`. With `fence_margin_m = 0` the two coincide and nothing shipped wrong; with
//! `500` the recovery put the player back at `(350.10, 120.50, -120.0)`, **0.1 m past the end
//! of the coping with nothing under it**, and he fell out again and had to be recovered twice.
//! `tests/player.rs::f012_a_fence_far_outside_the_map_is_not_a_bigger_map` is the repro.
//!
//! Its start value is the spawn position (`super::spawn_player_with_id`), so a player who
//! falls before he has ever landed still has somewhere to go.
//!
//! ## How a fall out of the world is told from a 120 m dive
//!
//! **Downwards, by depth.** Not by fall distance, not by speed, not by time in the air — a
//! dive off the coping of Ashgate's wall is 120 m of falling at up to 60 m/s and is the most
//! normal thing in this game. `bounds.recovery_plane_y_m` is **-300 m**, and the deepest block
//! in the whole district has its underside at **-4.2 m**: there is no surface in the map that
//! can be reached from under the plane, so nothing legitimate ever crosses it.
//! **Sideways, by the footprint** — and there the discriminator is that the map's ground ends
//! where `size_m` says it does, so there is nothing legitimate to do out there either.
//! `tests/player.rs::f012_a_dive_from_the_top_of_the_wall_is_not_a_fall_out_of_the_world` and
//! `f012_a_legitimate_fall_inside_the_map_is_never_recovered_at_any_height` are the controls,
//! and the first was **green before this file existed** — which is what makes it a control and
//! not a second copy of the feature.
//!
//! ## 🔴 A recovery that does not hold is not repeated — [`Recoveries`]
//!
//! Geometry is not a guarantee, and the ring proved it twice over: from the lip, one nudge
//! outward warped the player to `safe + lift`, which was **on the lip**, so he dropped the half
//! metre back onto it and drifted out again. Counted on the shipped binary: **1501 warp lines
//! in one run, one per tick, 25.0 s of wall clock, every one with the identical destination**,
//! plus sixty `warn!` lines a second.
//!
//! **If the destination does not hold, warping to it again is not a fix.** So the recovery
//! counts its own consecutive failures per player and escalates instead of repeating:
//!
//! 1. the ground he last stood on ([`SafeGround`]) — this is the normal case and it is over
//!    in one tick;
//! 2. still out of the world on the next tick? Then that ground is not to be trusted, whatever
//!    recorded it: **the spawn point**, which is the one place in the map nothing in play can
//!    have poisoned;
//! 3. still out after that? **Stop.** One `error!`, and then silence until he is in the world
//!    again. A player standing still outside the world is a bug somebody can see and report;
//!    a player teleported sixty times a second is a bug that hides itself behind its own noise.
//!
//! What moves him down the ladder is **whether the last recovery held**, asked by looking and
//! not by a clock: he is out of the world again, within a second of the warp
//! (`game.ron: simulation_hz`), and still within one body height
//! (`game.ron: player.height_m`) of where that warp put him down. A fall 400 m away eight
//! ticks later is a different fall and gets the full ladder again — a purely time-based rule
//! sent exactly that case to the spawn point and turned
//! `f012_a_player_below_the_world_comes_back_to_where_he_last_stood` red, which is what the
//! test is for. So the bound is **two warps per episode**, and an episode needs a destination
//! that fails twice. `tests/player.rs::f012_a_recovery_whose_destination_does_not_hold_is_not_repeated_
//! every_tick` injects a poisoned home by hand — because the next bug will produce one the same
//! way this one did — and counts.
//!
//! ## Why the warp goes through a message
//!
//! `Transform` on a player has exactly one non-avian writer, `super::apply_warps`, and it stays
//! that way. Sending [`WarpPlayer`] also buys the rope release for free:
//! `super::rope::detach_ropes` reads the same message one stage earlier, which is the whole
//! reason `B-003` is not open — a `DistanceJoint` that survives a teleport dragged the body
//! 47.93 m back in one tick.
//!
//! The message is written in `PostStep` and read in the next tick's `Drive`/`Integrate`, so a
//! recovery costs **one tick** — about half a metre more of falling, four hundred metres
//! below anything anybody can see.

use bevy::prelude::*;

use crate::data::{GameData, Map};
use crate::shared::{MovementState, PlayerId, Tick, WarpPlayer};

/// The last place this player stood that it is safe to put him back on.
///
/// Per player, on the player. See the module header for the three conditions and for why none
/// of them is "he was inside the fence".
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SafeGround {
    /// World position of his feet — the body's own origin (`docs/conventions.md`).
    pub pos_m: Vec3,
    /// The tick it was recorded on. Read by the log line a recovery writes: "back to where he
    /// stood 340 ticks ago" is a different bug report from "back to where he stood last tick".
    pub tick: u64,
}

impl SafeGround {
    /// A player who has never landed still has somewhere to go: where he came into the world.
    pub fn at_spawn(pos_m: Vec3) -> Self {
        Self { pos_m, tick: 0 }
    }
}

/// 🔴 **The one place in the game that decides where the world ends** — and one function, not
/// two, because two implementations of one question drift and no sweep finds it (`CLAUDE.md`
/// rule 5's corollary). [`record_safe_ground`] and [`recover_the_fallen`] both call this.
///
/// Pure: no `Commands`, no world, no `Res` — exactly like [`crate::world::bounds::plan_fence`],
/// so a sweep can ask it a hundred thousand times without building an app. ⚠️ **And a sweep of
/// it is arithmetic and not evidence** (`CLAUDE.md` rule 5, the fourth shape): every position
/// a pure sweep hands it is a `Vec3` the test invented, and — measured 2026-08-29 — a test that
/// asks *this* function whether a recorded home is safe learns nothing, because
/// [`record_safe_ground`] asked the same function about the same point one tick earlier. The
/// tests that count drive the real body and judge it against the map's planned geometry:
/// `f012_nothing_that_can_rest_on_the_fence_rests_inside_the_map` and
/// `f012_the_ground_he_is_sent_back_to_is_never_ground_he_can_be_sent_back_from`.
///
/// `None` means he is in the world. See the module header for both questions, and for the two
/// measurements that let the horizontal grace be exactly zero.
pub fn out_of_the_world(map: &Map, p_m: Vec3) -> Option<OutOfWorld> {
    // `size_m` ALONE. Not `+ fence_margin_m` — see the header: the fence's footprint is not
    // the map's, and the day they differ the recovery would put a player down 0.1 m past the
    // end of the coping with nothing under it.
    let hx = map.size_m.0 * 0.5;
    let hz = map.size_m.1 * 0.5;
    if p_m.y <= map.bounds.recovery_plane_y_m {
        Some(OutOfWorld::UnderThePlane)
    } else if p_m.x.abs() > hx || p_m.z.abs() > hz {
        // No tolerance, and the zero is derived — a grace of `g` metres is a standable ring
        // `g` metres wide somewhere. What pays for it is `bounds.fence_margin_m`, which is
        // bracketed by two measurements; the header carries the derivation.
        Some(OutOfWorld::PastTheEdge)
    } else {
        None
    }
}

/// Where a recovery **puts down** a body that last stood at `stood_m`.
///
/// 🔴 One function, because the invariant this file's header claims — *"the place you get sent
/// back to can never itself be a place you get sent back from"* — is only an invariant if the
/// question [`record_safe_ground`] asks and the point [`recover_the_fallen`] warps to are the
/// **same point**. They were not: the recorder judged where the player was *standing* and the
/// recovery warped him to that plus [`Bounds::recovery_lift_m`](crate::data::Bounds), and the
/// day the two differ is the day the guard is checking the wrong place. It costs nothing to
/// make it structural, and the header was already wrong once about it being free.
pub fn recovery_destination(map: &Map, stood_m: Vec3) -> Vec3 {
    stood_m + Vec3::Y * map.bounds.recovery_lift_m
}

/// Which of the two questions said he is out. Carried into the `warn!` because
/// *"he went over the side at 240 m"* and *"he is under the world at -304 m"* are different
/// bug reports, and a recovery that does not say which is a bug that hides itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutOfWorld {
    /// Below `bounds.recovery_plane_y_m`. He fell through something.
    UnderThePlane,
    /// Outside `Map::size_m` horizontally, **at any height** — over the fence, through it, or
    /// standing on top of it.
    PastTheEdge,
}

impl OutOfWorld {
    /// One clause for the log line.
    pub fn why(self) -> &'static str {
        match self {
            OutOfWorld::UnderThePlane => "under the world",
            OutOfWorld::PastTheEdge => "outside the map footprint",
        }
    }
}

/// How the recovery itself is going, per player — **the thing that keeps it from becoming the
/// bug it is supposed to fix.**
///
/// Measured on the shipped binary 2026-08-29: from the fence's inner lip, one nudge outward
/// produced **1501 warps in 25 s, one per tick, every one with the identical destination**, and
/// sixty `warn!` lines a second. A destination that does not hold is not fixed by warping to it
/// again. See the module header for the three steps.
///
/// On the player, never a `Resource` and never reached through `.single()` (`CLAUDE.md` rule 4).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Recoveries {
    /// Where this player came into the world. **Written once, at spawn, and never again** — it
    /// is the destination of last resort precisely because nothing that happens in play can
    /// reach it. [`SafeGround`] can be poisoned; this cannot.
    pub spawn_m: Vec3,
    /// How many recoveries in a row have failed to put him back in the world. `0` means the
    /// last one held (or there has not been one).
    pub in_a_row: u32,
    /// The tick the last warp was sent on, and where it was sent. Together they answer the one
    /// question the escalation turns on — **did the last recovery hold?** — and they answer it
    /// by looking, not by a clock: he is out of the world again, within a second, still
    /// essentially where he was put down. A fall somewhere else a moment later is a new fall
    /// and gets the full ladder again; that is what `tests/player.rs::f012_a_player_below_the_
    /// world_comes_back_to_where_he_last_stood` measures, and a purely time-based rule sent its
    /// second fall to the spawn point.
    pub last_warp_tick: u64,
    /// Where the last warp put him. See [`Self::last_warp_tick`].
    pub last_warp_to_m: Vec3,
}

impl Recoveries {
    /// A fresh player: never recovered, and his own spawn point as the fallback.
    pub fn at_spawn(pos_m: Vec3) -> Self {
        Self { spawn_m: pos_m, in_a_row: 0, last_warp_tick: 0, last_warp_to_m: pos_m }
    }
}

/// **The one writer of [`SafeGround`].**
pub fn record_safe_ground(
    data: Res<GameData>,
    tick: Res<Tick>,
    mut players: Query<(&Transform, &MovementState, &mut SafeGround), With<PlayerId>>,
) {
    let Some(map) = data.current_map() else {
        return;
    };
    let r = data.game.player.radius_m;

    for (transform, state, mut safe) in &mut players {
        if *state != MovementState::Grounded {
            continue;
        }
        let p = transform.translation;
        // 🔴 THE INVARIANT, and it asks about **the place he would be PUT DOWN**, not the
        // place he is standing: `recovery_destination` is the one function that knows what a
        // recovery does with a recorded point, so the question this system asks and the point
        // `recover_the_fallen` warps to cannot drift apart.
        //
        // 🔴 And it asks about **the whole body**, not about its origin. A capsule standing
        // within `player.radius_m` of the map's edge may be standing on the ground — or on the
        // inner lip of the fence's top face, `fence_margin_m` further out, which is not part of
        // the world at all. **From a position alone the two cannot be told apart**, because the
        // capsule that could be resting on either is the same size: measured 2026-08-29, a body
        // grounded on that lip was recorded at `(-299.74, 199.886, 0)` — 200 m up with nothing
        // under it — and a later fall would have been warped there. So the stance is judged at
        // the far corner of the body's own footprint and both are refused. Nothing is lost: a
        // player pressed against the fence is recorded a body-radius further in, one tick
        // earlier, and that stance is real ground.
        let body = Vec3::new(r * p.x.signum(), 0.0, r * p.z.signum());
        if out_of_the_world(map, recovery_destination(map, p) + body).is_some() {
            continue;
        }
        // `set_if_neq` for the same reason as in `integrator::readback`: a standing player
        // really does not change, and a component that reports itself changed on all sixty
        // ticks makes every `Changed<T>` after it worthless.
        safe.set_if_neq(SafeGround { pos_m: p, tick: tick.0 });
    }
}

/// Sends everybody who is out of the world back to the ground he last stood on — **once**, then
/// to his spawn point, then not at all.
///
/// Writes **only** the message and its own [`Recoveries`]. `super::apply_warps` moves the body,
/// `super::rope::detach_ropes` lets the arms go — one writer each, and neither of them learns a
/// new rule for this.
pub fn recover_the_fallen(
    data: Res<GameData>,
    tick: Res<Tick>,
    mut warp: MessageWriter<WarpPlayer>,
    mut players: Query<(&PlayerId, &Transform, &SafeGround, &mut Recoveries)>,
) {
    let Some(map) = data.current_map() else {
        return;
    };
    // "A whole second", out of the file and not as a literal (`CLAUDE.md` rule 2), and "within
    // a body's own height", likewise. Together they say what "the last recovery did not hold"
    // means, and they say it by LOOKING at where he is rather than by counting ticks: a fall
    // 400 m away eight ticks later is a new fall, and the escalation must not eat it.
    let settle_ticks = data.game.simulation_hz.round().max(1.0) as u64;
    let same_place_m = data.game.player.height_m;

    for (id, transform, safe, mut tries) in &mut players {
        let p = transform.translation;
        let Some(why) = out_of_the_world(map, p) else {
            continue;
        };

        // 🔴 Did the last one hold? He is out of the world again, within a second of the warp,
        // and still essentially where that warp put him down — then the destination is the
        // problem and sending him there again is not a fix. Anywhere else, or later, is a new
        // fall and starts the ladder over.
        let same_failure = tick.0.saturating_sub(tries.last_warp_tick) < settle_ticks
            && p.distance(tries.last_warp_to_m) < same_place_m;
        let attempt = if same_failure { tries.in_a_row + 1 } else { 1 };
        tries.in_a_row = attempt;

        // Step 3 of the header: stop. `error!` exactly on the tick the decision is taken, and
        // then silence — a player standing still outside the world is a bug somebody can see
        // and report, a player teleported sixty times a second is a bug hiding behind its own
        // noise, and the second one is what shipped.
        if attempt > 2 {
            if attempt == 3 {
                error!(
                    "player {} is at {:.1?} m, {}, and is STILL there after a recovery to the \
                     ground he stood on at tick {} and one to his spawn point {:.1?}. Giving up: \
                     no further warp and no further line until he is back inside the {} x {} m \
                     map, because warping to a destination that does not hold is not a fix — it \
                     is what produced 1501 warps in 25 s on 2026-08-29.",
                    id.0,
                    p,
                    why.why(),
                    safe.tick,
                    tries.spawn_m,
                    map.size_m.0,
                    map.size_m.1
                );
            }
            continue;
        }

        // Step 1 is the recorded ground; step 2 says that ground is not to be trusted, whatever
        // recorded it, and falls back to the one place in the map that play cannot poison.
        let (back, from_where) = if attempt == 1 {
            (recovery_destination(map, safe.pos_m), format!("the ground he stood on at tick {}", safe.tick))
        } else {
            (
                recovery_destination(map, tries.spawn_m),
                format!(
                    "his spawn point — the ground he stood on at tick {} did not hold and is \
                     not to be trusted",
                    safe.tick
                ),
            )
        };
        // `warn!` and not `info!`: reaching this line means something already went wrong — the
        // fence is what normal play meets, and nothing normal gets here. A silent recovery is a
        // bug that hides itself.
        warn!(
            "player {} was at {:.1?} m, {} ({} x {} m, plane {} m): back to {:?}, {from_where}",
            id.0,
            p,
            why.why(),
            map.size_m.0,
            map.size_m.1,
            map.bounds.recovery_plane_y_m,
            back,
        );
        tries.last_warp_tick = tick.0;
        tries.last_warp_to_m = back;
        warp.write(WarpPlayer { player: *id, pos_x: back.x, pos_y: back.y, pos_z: back.z });
    }
}
