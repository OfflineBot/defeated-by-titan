//! hold — **the blade as a thing in the world, in the player's hands.**
//!
//! The user, after playing on 2026-08-19: *„attack fehlt aber noch (mit schwertern..)"*. Two
//! rounds then measured that the attack itself is not what is missing — the swing fires, the
//! cast lands, `TitanHit` is written, a body cut staggers. What was missing is **steel**:
//! `FIND-120` found that `super::cut::blade_segment` builds two `Vec3` per tick, casts a
//! capsule between them and throws both away in the same tick. There was no entity, no mesh,
//! no transform — and therefore nothing a [`ModelName`] could sit on, which is the whole reason
//! `assets/data/art.ron: "blade"` had to stay `Primitive` while the pair
//! (`a-024-klingen-paar-neu`), the empty grips and ten finishes sat unused in the drop.
//!
//! This file is the missing object. One entity per player, parented to him, carrying the
//! logical name `blade` — and `render::model::spawn_models` dresses it out of the registry
//! exactly like a titan, without `render` ever learning that a domain `blades` exists.
//!
//! ## It draws what the cast already computes — and that is one expression, not two
//!
//! [`super::cut::blade_right`] is the direction the cutting capsule lies on. This file points
//! the pair at **that vector**, out of the same function, so the two cannot drift:
//! [`held_pose`] at full sweep puts the model's left blade on `-blade_right(look)` and its
//! right blade on `+blade_right(look)`, which is precisely
//! `blade_segment(.., Side::Left/Right, ..)`. A second notion of where the blade is would make
//! `FIND-113`'s lesson — *the camera is part of the shot* — a rendering bug as well.
//!
//! ## 🔴 Where the hand is — and on 2026-08-19 it was the EYE, which is why the user saw a spike
//!
//! Until 2026-08-19 this file lifted the pair by `eye_height_m − (its own `hand.l`/`hand.r`
//! empty)` — i.e. it put the model's hands **on the camera**, so that the drawing and
//! `blade_segment`'s cast would share one hand. The argument was that a blade *stands across*
//! the view ray where a rope lies along it. **At full sweep that is true. At rest it is exactly
//! false**, and rest is what a player looks at nearly all the time:
//!
//! * the pair is authored 2.08 m from tip to tip (`hit.min`/`hit.max`, `art.ron`'s `"blade"`
//!   row), because it is a PAIR held fore-and-aft — 0.93 m of steel per hand;
//! * at sweep 0 that 2.08 m lies **along the look direction**, centred on the hands;
//! * with the hands on the camera, one blade therefore ran from behind the eye to **1.17 m in
//!   front of it**, through Bevy's 0.1 m near plane, dead centre of the frame.
//!
//! That is the pale spike across `docs/images/f003-map-before-aerial.png` and
//! `-street.png` — and it was never a scale problem. **The pair is drawn at scale 1.0000 and
//! that is its correct size.**
//!
//! **So the pair hangs where the drop authored it, and nothing lifts it.** `a-024` is a
//! costume part in the Vanguard rig's own space — the same class of file as `a-012` cloaks at
//! y 0.15 and `a-045` heads at y 8.9 (`FIND-132`), and its `hand.l`/`hand.r` sit at
//! **y ≈ 0.8376 in a 1.80 m rig** (`a-001-basis-rig-vanguard`: 0.64 × **1.80** × 0.54).
//! `game.ron: player.height_m` is **1.8** and `scale.ron: reference.human_height_m` is the same
//! figure — so a `Transform::default()` on a player whose origin is between his feet
//! (`docs/conventions.md`) puts the file's hands exactly where a 1.80 m body's hands are.
//! **No number is invented here; two files that already agree are simply believed.**
//!
//! ⚠️ **What that costs, honestly:** the drawn hand is now 0.76 m below the eye, and
//! `blades::cut::blade_segment` still casts from the eye. The picture and the capsule are
//! therefore 0.76 m apart on y — on top of the 0.67 m of reach `FIND-127` already measured
//! them apart on length. **Both are one defect** (the cut is cast from a camera, not from a
//! hand) and both belong to `F-030`'s round, with `scripts/f030-cortex.txt` re-measured. What
//! is NOT acceptable is the previous answer to it: agreeing with the cast by drawing the steel
//! inside the camera.
//!
//! `render::rope`'s header settles the same question the other way round for a rope, and the
//! two are now consistent rather than opposite: **neither is drawn from the eye.**
//!
//! ## ⚠️ What this is NOT: an animation
//!
//! **The drop ships zero animation clips** — measured over all 278 files, twice
//! (`assets/data/art.ron`'s header). So there is no swing *animation* and `animations: {}`
//! stays honest. What a player gets is a blade that **exists, is held, and turns through the
//! arc that ends on the cast**: [`swing_sweep`] is `1.0` on exactly the ticks
//! [`Swing::is_active`] is true and below `1.0` on every other tick, so the drawn pair is on
//! the capsule's line for the whole cutting window and travels in and out around it.
//!
//! ## What it costs
//!
//! One entity per player, and one `Transform` write per player per frame — no query over the
//! world, no allocation, nothing per titan (`docs/lessons/performance.md` rule 6).
//! [`equip_blades`]'s query is `Without<BladeHand>`, so it is empty from the second frame on.

use bevy::prelude::*;

use crate::shared::{Intent, ModelAnchors, ModelName, PlayerId};

use super::cut::blade_right;
use super::swing::{BladeTiming, Swing, Swings};

/// The key into `assets/data/art.ron: models`. **One place**, so the row and the spawner
/// cannot disagree about the name — an unknown name renders nothing and warns once.
pub const BLADE_MODEL: &str = "blade";

/// How far the pair turns out of its authored ready pose over one swing: a quarter turn.
///
/// Not a taste number. The drop authors the pair fore-and-aft — `hand.l`'s blade forward,
/// `hand.r`'s back — and the cut lies along `±blade_right(look)`, which is the forward axis
/// turned by exactly 90° about Y. So this constant is the angle between the file's pose and
/// the simulation's, and [`held_pose`] is a rotation from the one to the other.
pub const SWEEP_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// The entity that carries the pair. Marker only — everything about it is derived.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldBlades;

/// On the **player**: he already has a pair, and which entity it is.
///
/// An `Entity` and not a stable id on purpose: this is a parent/child link inside one process,
/// exactly like `ChildOf` itself, and it never travels over a wire
/// (`docs/multiplayer.md` rule 4 is about simulation state, and this is neither).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BladeHand(pub Entity);

/// Where the pair's own hands sit, out of the model's `hand.l` / `hand.r` empties.
///
/// The mean of the two, in metres above the model's origin — `None` when the file names
/// neither, which is the honest answer for a row that is still `Primitive` and for any model
/// that is not a pair of blades. **Never a substituted number.**
///
/// ⚠️ **Since 2026-08-19 this is a yardstick and no longer a lift.** [`held_pose`] does not
/// move the pair at all (see the module header), so what this function is for is the check
/// that the swapped-in model is authored for a body of the game's size:
/// `tests/render.rs::f033_the_pairs_own_hands_land_on_a_1_80_m_bodys_hands` reads it out of the
/// real `.glb` and holds it against `game.ron: player.height_m`. A pair authored for a 2.4 m
/// rig would otherwise hang in the air with nothing saying so.
pub fn hand_height_m(anchors: &ModelAnchors) -> Option<f32> {
    let hands: Vec<f32> = ["hand.l", "hand.r"]
        .iter()
        .filter_map(|a| anchors.get(a))
        .map(|v| v.y)
        .collect();
    if hands.is_empty() {
        return None;
    }
    Some(hands.iter().sum::<f32>() / hands.len() as f32)
}

/// How far through its arc one blade is: `0.0` at rest, `1.0` **exactly on the ticks it cuts**.
///
/// The claim that makes this file falsifiable, and it is a claim about the simulation and not
/// about a look: the pair is on the cast's own line for the whole of `active_from..active_to`
/// and nowhere else. Wind-up walks it out, the recovery walks it back — both linear, both over
/// the ticks `gear.ron` already pays for, so the arc costs the swing nothing it did not have.
pub fn swing_sweep(swing: &Swing, timing: &BladeTiming) -> f32 {
    let Some(t) = swing.ticks_in_swing else {
        return 0.0;
    };
    if t < timing.active_from_tick {
        // A window that opens on tick 0 has no wind-up to walk through — then the blade is
        // out from the first tick, which is what `active_from_tick: 0` asks for.
        if timing.active_from_tick == 0 {
            return 1.0;
        }
        return t as f32 / timing.active_from_tick as f32;
    }
    if t < timing.active_to_tick {
        return 1.0;
    }
    let recovery = timing.swing_ticks.saturating_sub(timing.active_to_tick);
    if recovery == 0 {
        return 0.0;
    }
    // `+ 1`, so that the first tick after the window is already below 1.0 — otherwise the
    // drawn blade would claim to be cutting on a tick on which it does not.
    (1.0 - (t - timing.active_to_tick + 1) as f32 / recovery as f32).max(0.0)
}

/// The pair's transform **in the player's own frame**, at a given look and sweep.
///
/// A pure function of two numbers, so "the drawn blade ends on the cast" is a claim a test
/// puts a number on without an app, a camera or a GPU.
///
/// * **rotation** — a single turn about Y. `blade_right(look)` is where the cut lies; the
///   model's blades lie fore-and-aft in its own frame, and `render::model::MODEL_FACES` has
///   already turned the file's `+Z` into the game's `−Z` on the scene child below this one. So
///   the yaw that puts the model's forward on the game's, plus [`SWEEP_RAD`] × sweep, lands the
///   left blade on `−blade_right` and the right one on `+blade_right`.
/// * **translation** — `Vec3::ZERO`, always. The pair is a part authored in the Vanguard rig's
///   own space and the player is that rig's height, so "draw it where the file puts it" IS
///   "draw it in his hands" (module header). It is the same sentence
///   [`ModelName::feet_y_m`](crate::shared::ModelName) says one level down for a rig part, and
///   the same reason `ModelName::new` leaves that field `None` for a blade.
pub fn held_pose(look: Vec3, sweep: f32) -> Transform {
    let right = blade_right(look);
    // The horizontal forward that goes with that right. `Y × right`, not `right × Y`: for
    // `right = +X` this is `−Z`, which is the game's forward (`docs/conventions.md`).
    let forward = Vec3::Y.cross(right);
    // The yaw that maps the model's own forward (`−Z`, after MODEL_FACES) onto it.
    let yaw = f32::atan2(-forward.x, -forward.z);
    Transform {
        translation: Vec3::ZERO,
        rotation: Quat::from_rotation_y(yaw + SWEEP_RAD * sweep.clamp(0.0, 1.0)),
        scale: Vec3::ONE,
    }
}

/// Hangs one pair on every player who has none yet.
///
/// **A child of the player and not a root entity**: the pair then follows him at 75 m/s
/// without a second notion of where he is, and it costs no position write at all — only the
/// rotation in [`hold_the_blades`] changes. No `.single()` and no `With<LocalPlayer>`: every
/// player carries blades, and the day a second one exists he carries his own
/// (`docs/multiplayer.md` rule 3).
///
/// ⚠️ `Visibility` is inserted deliberately. The player carries none, and Bevy's propagation
/// falls back to *visible* for a child whose parent lacks the components
/// (`bevy_camera-0.19.0/src/visibility/mod.rs:655`) — but only for a child that has an
/// `InheritedVisibility` of its own to write into, which is what `Visibility` requires.
pub fn equip_blades(
    mut commands: Commands,
    fresh: Query<Entity, (With<PlayerId>, Without<BladeHand>)>,
) {
    for player in &fresh {
        let hand = commands
            .spawn((
                Name::new("blades"),
                HeldBlades,
                // ⚠️ **`ModelName::new` — and both of its `None`s are the right answer here,
                // checked on 2026-08-19 rather than inherited.**
                //
                // * `feet_y_m: None`. `FIND-132` moves a DRESSED BLOCK's model down onto the
                //   box's floor because a block's entity sits at the box's centre. A blade is
                //   the other class of file in that same finding: a **part authored in its
                //   parent rig's space** (`a-024`'s own floor is y 0.45, not 0), and moving one
                //   of those is the bug the field was written to avoid. `None` = "draw it where
                //   the rig authored it", which is exactly what a held blade wants.
                // * `height_m: None`. `render::model::fit_to_class` scales by the model's
                //   **`hit` height**, and the pair's `hit` height is its 0.87 m THICKNESS — its
                //   length is 2.08 m on z. So a `height_m` here would be a yardstick laid
                //   across the blade instead of along it, and any number in it would be
                //   arbitrary. The pair is authored 1:1 for a 1.80 m rig and the game's player
                //   is 1.80 m, so scale 1.0000 is not a fallback, it is the measurement.
                //   (The 0.67 m gap to `gear.ron: blades.reach_m` that `FIND-127` measured is
                //   real and is NOT closed by scaling — see the module header.)
                ModelName::new(BLADE_MODEL),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(player).insert(BladeHand(hand)).add_child(hand);
    }
}

/// One frame of the pair: where he looks, how far through the swing he is.
///
/// **`Update` and not `FixedUpdate`**, and that is not a break with rule 4: nothing here is
/// simulation — no field of the game state is read back from it, and the cut does not consult
/// it. It is drawn beside `render::camera::rotate_camera`, off the same `Intent`, because a
/// pair that turned at 60 Hz while the camera turns at the frame rate would visibly lag the
/// view it is held in.
pub fn hold_the_blades(
    players: Query<(&Intent, &Swings, &BladeTiming)>,
    mut held: Query<(&ChildOf, &mut Transform), With<HeldBlades>>,
) {
    for (parent, mut transform) in &mut held {
        let Ok((intent, swings, timing)) = players.get(parent.parent()) else {
            continue;
        };
        // The pair is **one** mesh carrying both blades (`a-024-klingen-paar-neu`: one merged
        // primitive, `hand.l` and `hand.r`), so the two sides cannot be drawn apart. The
        // further of the two swings wins — a blade that is out has to look out, and the drop
        // ships no single-blade file to do better with.
        let sweep =
            swing_sweep(&swings.left, timing).max(swing_sweep(&swings.right, timing));
        let pose = held_pose(intent.look_dir(), sweep);
        // Compared before it is written: a `DerefMut` marks the transform changed and the
        // change travels into propagation and into the render world every frame it happens
        // (`docs/lessons/performance.md` rule 6, the same reason `apply_field_of_view` reads
        // through `&*` first).
        if *transform != pose {
            *transform = pose;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Side;

    /// The numbers `gear.ron` gives at 60 Hz: 0.35 / 0.08 / 0.22 / 0.30 s.
    fn timing() -> BladeTiming {
        BladeTiming {
            swing_ticks: 21,
            active_from_tick: 5,
            active_to_tick: 13,
            cooldown_ticks: 18,
        }
    }

    fn looks() -> Vec<Vec3> {
        let mut out = Vec::new();
        for yaw in [-3.0_f32, -1.6, -0.4, 0.0, 0.7, 2.9] {
            for pitch in [-1.2_f32, -0.3, 0.0, 0.5, 1.4] {
                let (sy, cy) = yaw.sin_cos();
                let (sp, cp) = pitch.sin_cos();
                out.push(Vec3::new(-sy * cp, sp, -cy * cp));
            }
        }
        out
    }

    #[test]
    fn f033_the_drawn_blade_ends_exactly_on_the_cast_it_is_drawn_for() {
        // **The one test this whole file exists to make possible.** At full sweep the model's
        // left blade (its local −Z, after MODEL_FACES) has to lie on the same line
        // `blades::cut` casts its capsule along, for every look direction there is. Break the
        // `+ SWEEP_RAD * sweep` in `held_pose` and this goes red at 90° on all thirty.
        for look in looks() {
            let pose = held_pose(look, 1.0);
            let left_blade = pose.rotation * -Vec3::Z;
            let right_blade = pose.rotation * Vec3::Z;
            let (a, b) = super::super::cut::blade_segment(Vec3::ZERO, look, Side::Left, 1.6, 1.6);
            let cast = (b - a).normalize();
            assert!(
                (left_blade - cast).length() < 1e-5,
                "look {look:?}: the drawn left blade points {left_blade:?} and the cut is cast \
                 along {cast:?} — the picture and the capsule are two different blades"
            );
            assert!(
                (right_blade + cast).length() < 1e-5,
                "look {look:?}: the two drawn blades are not each other's opposite"
            );
        }
    }

    #[test]
    fn f033_at_rest_the_pair_faces_where_he_looks() {
        // Sweep 0 is the file's own ready pose, and it has to face forward and not sideways —
        // otherwise the player holds his blades across his chest while standing still.
        for look in looks() {
            let pose = held_pose(look, 0.0);
            let forward = pose.rotation * -Vec3::Z;
            let want = Vec3::new(look.x, 0.0, look.z).normalize_or_zero();
            // Straight up and straight down have no horizontal look at all; `blade_right`
            // falls back to the world X axis there and the pair goes with it.
            let want = if want.length_squared() > 0.0 { want } else { -Vec3::Z };
            assert!(
                (forward - want).length() < 1e-4,
                "look {look:?}: the resting pair faces {forward:?}, he faces {want:?}"
            );
        }
    }

    #[test]
    fn f033_the_pair_is_out_on_exactly_the_ticks_that_cut() {
        // The claim that ties the drawing to the simulation instead of to a timer of its own.
        let t = timing();
        let mut s = Swing::default();
        s.start();
        let mut ticks_out = 0;
        for tick in 0..t.swing_ticks {
            let sweep = swing_sweep(&s, &t);
            assert!((0.0..=1.0).contains(&sweep), "tick {tick}: sweep {sweep} is off the arc");
            if s.is_active(&t) {
                assert_eq!(
                    sweep, 1.0,
                    "tick {tick} cuts and the drawn pair is only {sweep} of the way out — the \
                     picture would show a blade somewhere the capsule is not"
                );
                ticks_out += 1;
            } else {
                assert!(
                    sweep < 1.0,
                    "tick {tick} does not cut and the pair is fully out — a blade that looks \
                     like it is cutting and is not is worse than no blade"
                );
            }
            s.advance(&t);
        }
        assert_eq!(ticks_out, t.active_ticks(), "gear.ron gives 8 cutting ticks out of 21");
        assert_eq!(swing_sweep(&s, &t), 0.0, "the arm is back at rest and the pair is not");
    }

    #[test]
    fn f033_the_arc_is_walked_and_not_jumped() {
        // Without this the "arc" could be a two-state flip and every assertion above would
        // still hold. A quarter turn in one tick is not a swing, it is a teleporting sword.
        let t = timing();
        let mut s = Swing::default();
        s.start();
        let mut previous = swing_sweep(&s, &t);
        let mut steps = 0;
        for _ in 0..t.swing_ticks {
            s.advance(&t);
            let now = swing_sweep(&s, &t);
            let step = (now - previous).abs();
            assert!(
                step <= 0.5,
                "the pair moved {step} of its whole arc in one tick — that is {:.0}° at 60 Hz",
                step * SWEEP_RAD.to_degrees()
            );
            if step > 0.0 {
                steps += 1;
            }
            previous = now;
        }
        assert!(steps >= 8, "the pair moved on only {steps} ticks of the swing");
    }

    /// The pair's own corners out of `a-024-klingen-paar-neu.glb`, in the RIG's frame — the
    /// same two `art.ron`'s `"blade"` row writes down: `hit.min` (-0.345, **0.4518**, 1.170),
    /// `hit.max` (0.286, **1.3210**, -0.908). Measured out of the file, not chosen.
    const AUTHORED_TOP_M: f32 = 1.3210;

    /// Bevy's own default near plane — `bevy_camera-0.19.0/src/projection.rs:421`. Nothing in
    /// this project overrides it, so geometry closer than this to the eye is CUT, and what is
    /// left of it fills the frame from the middle outwards.
    const NEAR_M: f32 = 0.1;

    #[test]
    fn f033_the_steel_hangs_at_the_hands_and_never_across_the_camera() {
        // 🔴 **The picture the user was shown on 2026-08-19.** `held_pose` used to lift the
        // pair by `eye_height_m - hand`, i.e. put the model's hands ON the camera. The pair is
        // 2.08 m long from tip to tip in its own frame, so one blade then ran from behind the
        // eye to 1.17 m in front of it, straight through the 0.1 m near plane, dead centre —
        // the pale spike across `docs/images/f003-map-before-aerial.png`.
        //
        // The claim now: **nothing the pair is made of comes nearer the eye than the near
        // plane.** Put the eye lift back and this goes red by half a metre.
        let eye = 1.6;
        let top = held_pose(-Vec3::Z, 0.0).translation.y + AUTHORED_TOP_M;
        assert!(
            eye - top > NEAR_M,
            "the highest steel is drawn at y = {top:.3} m and the eye is at {eye:.3} m — \
             {:.3} m apart, and the near plane is {NEAR_M} m. The pair is inside the camera.",
            eye - top
        );
    }

    #[test]
    fn f033_the_pair_is_drawn_where_its_rig_authored_it_and_never_moved_by_a_guess() {
        // `hand_height_m` is `None` for every model that is not a pair of blades, and the
        // answer to "I do not know where its hand is" is to leave it where it was authored —
        // never a substituted number (`shared::ModelAnchors::get`'s own rule). Since
        // 2026-08-19 that is the answer for a model that DOES name its hands as well: the pair
        // is a rig part and the rig is the player.
        assert_eq!(hand_height_m(&ModelAnchors::default()), None);
        for sweep in [0.0, 0.4, 1.0] {
            assert_eq!(
                held_pose(-Vec3::Z, sweep).translation,
                Vec3::ZERO,
                "sweep {sweep}: the pair was moved out of the frame its file was authored in"
            );
        }

        let mut anchors = ModelAnchors::default();
        anchors.0.insert("hand.l".into(), Vec3::new(-0.203, 0.8444, -0.236));
        anchors.0.insert("hand.r".into(), Vec3::new(0.220, 0.8307, 0.0256));
        let hand = hand_height_m(&anchors).expect("both hands are named");
        assert!((hand - 0.83755).abs() < 1e-4, "the mean of the two hands is {hand}");
        // And unmoved, those hands are a 1.80 m body's hands — 46.5 % of his height, which is
        // where a hanging arm ends. That is the whole claim the lift used to be needed for.
        let share = hand / 1.8;
        assert!(
            (0.40..0.55).contains(&share),
            "the pair's hands sit at {share:.3} of a 1.80 m body — that is not a hand height, \
             so this file is not authored for the game's rig"
        );
    }
}
