//! The two ropes in the picture — **without them no screenshot is evidence for `F-001`.**
//!
//! Until 2026-08-10 this file was one empty line and every image in the repository captioned
//! "hook" or "rope" showed **no rope at all**: the evidence PNG of a cortex cut under rope
//! momentum contained exactly eleven cyan connected components and every one of them was an
//! axis-aligned HUD rectangle (gas fill 274x12, pip underline 86x6, five pips 14x18, four
//! crosshair ticks). The gas bar draining was the only thing on screen that said a rope was
//! attached. That is what this file repairs.
//!
//! Drawn in **cyan**: the signal colour of the Vector Gear (`docs/conventions.md` §3, "cyan —
//! gas, Vector Gear, anchor points"). The three numbers come out of `assets/data/maps.ron`'s
//! `signals:` block through [`rope_color`], never as a literal — the same route
//! `hud::signal` and `titan::rig` take, and for the same reason: a colour that is written
//! twice is wrong in one of the two places by next week.
//!
//! `render` **reads only**. The state comes out of [`Hook`]; this module writes no field of
//! the simulation.
//!
//! ## Where a rope starts, and why it is not the hand
//!
//! The simulation's hand is the **eye**: `vector::hook` starts and retracts every tip at
//! `transform.translation + Y · player.eye_height_m` (`src/vector/hook.rs`, decision 3), and
//! its own header says why — the flight distance has to be the distance the aim ray measured.
//! Its last sentence is the one that matters here: *"A real shoulder socket is a number the
//! RON does not have yet."*
//!
//! **That point may not be used for drawing.** `render::attach_camera` hangs the camera on the
//! player as a child at exactly `Transform::from_xyz(0, eye_height_m, 0)`, and the player never
//! rotates — so the simulation's hand **is** the camera's own position, to the millimetre. A
//! segment that starts at the camera and ends at the anchor lies **along a view ray**: every
//! one of its points projects onto the same pixel, so it renders as a single dot and the image
//! is exactly as empty as it was before. Correct, invisible, and impossible to notice from the
//! outside — the worst of the three.
//!
//! So the rope is drawn from the **player's origin**, which lies between his feet
//! (`docs/conventions.md`): the one point on the body that already exists, is 1.6 m off the
//! camera, and needs no number that the RON does not have. It is a stand-in for a shoulder
//! socket, not a claim to be one — the day `game.ron` grows a hand offset, exactly one
//! expression in [`draw_ropes`] changes.
//!
//! ## Only an **anchored** arm draws
//!
//! [`HookArm::tip_m`] is also live while `Flying` and `Retracting` — `vector::hook` walks the
//! tip along in both. Neither of those is a rope: a tip on its way out is a projectile, a tip
//! on its way home is nothing at all, and drawing them would make "there is a cyan line"
//! stop meaning "he is attached". That is the half of this file that makes it falsifiable, and
//! `tests/render.rs::f004_only_an_anchored_arm_draws_a_rope` is what keeps it there.

use bevy::gizmos::config::GizmoConfigGroup;
use bevy::gizmos::gizmos::GizmoBuffer;
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Hook, HookArm, Side};

/// The key in `assets/data/maps.ron`'s `signals:` block that a rope is painted with.
///
/// `docs/conventions.md` §3 reserves cyan for "gas, Vector Gear, anchor points". A rope is
/// Vector Gear, so the colour is prescribed here and not chosen.
pub const SIGNAL: &str = "cyan";

/// The rope colour out of `assets/data/maps.ron`.
///
/// **Panics on a missing key, deliberately** — the same choice `hud::signal` and
/// `titan::rig::cortex_in_head` make. A grey stand-in would put a rope on screen that quietly
/// stops obeying `docs/conventions.md` §3, and nothing would ever say so (`CLAUDE.md` rule 2:
/// no `serde(default)` for game values, for exactly this reason).
///
/// The triples in `maps.ron` are **linear** RGB, hence `Color::linear_rgb` and not `srgb`:
/// the two differ by about a factor of two in the mid-tones, which is enough to make this
/// cyan a different cyan from the one `debug::gizmo` outlines an anchor surface with.
pub fn rope_color(data: &GameData) -> Color {
    let (r, g, b) = data.maps.signals.get(SIGNAL).copied().unwrap_or_else(|| {
        panic!(
            "maps.ron `signals:` has no key {SIGNAL:?} — there is nothing to draw a rope \
             with. The three keys are cyan, amber, crimson (docs/conventions.md §3)"
        )
    });
    Color::linear_rgb(r, g, b)
}

/// The one segment an arm contributes — or `None`, which means it draws nothing.
///
/// A pure function of the arm and one point, so that "left anchored and right free draws
/// exactly one line" is a claim a test can put a number on without an app, a camera or a GPU.
pub fn rope_segment(arm: &HookArm, origin_m: Vec3) -> Option<(Vec3, Vec3)> {
    // **The one expression the whole file turns on.** Replace it with `None` — the state this
    // file was in until 2026-08-10 — and three tests in this module plus four in
    // `tests/render.rs` go red, the last of them by reading the gizmo buffer the app really
    // handed to the renderer. That is the check that this is drawn and not merely computed.
    arm.state.is_anchored().then_some((origin_m, arm.tip_m))
}

/// Both arms at once, indexed the way [`Side::index`] indexes: `0 = left`, `1 = right`.
///
/// An array and not a `Vec`: at most two ropes hang on a player, this runs once per player per
/// frame, and an allocation per frame is the shape `docs/lessons/performance.md` rule 6 exists
/// to forbid. It also makes the left/right independence a property of the **type** — a test
/// reads slot 0 and slot 1, it does not count entries and hope.
pub fn rope_segments(hook: &Hook, origin_m: Vec3) -> [Option<(Vec3, Vec3)>; 2] {
    Side::ALL.map(|side| rope_segment(hook.arm(side), origin_m))
}

/// Puts one player's ropes into a gizmo buffer.
///
/// Generic over the gizmo group, the same shape as `debug::gizmo::outline_anchor`, so a test
/// can draw into a [`GizmoBuffer`] of its own and count the points without building an app.
pub fn draw_rope_lines<C, K>(
    gizmos: &mut GizmoBuffer<C, K>,
    hook: &Hook,
    origin_m: Vec3,
    color: Color,
) where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    for (start_m, end_m) in rope_segments(hook, origin_m).into_iter().flatten() {
        gizmos.line(start_m, end_m, color);
    }
}

/// Draws one line per **anchored** arm, for every player.
///
/// No `.single()` and no `With<LocalPlayer>`: every player is one of many and every one of them
/// has two ropes worth seeing (`docs/multiplayer.md` rule 3). `With<Hook>` falls out of the
/// query itself, so this touches the players and not the world (`docs/lessons/performance.md`
/// rule 1).
///
/// `&Transform` and not `&GlobalTransform`: a player is a root entity (the camera is *his*
/// child, not the other way round), so the two are the same value — and `Transform` is the one
/// avian's writeback wrote **this** tick, while `GlobalTransform` is propagated in `PostUpdate`
/// and is therefore one frame old. A rope that lags the body by a frame at 75 m/s is a rope
/// that starts a metre behind the player.
pub fn draw_ropes(data: Res<GameData>, players: Query<(&Hook, &Transform)>, mut gizmos: Gizmos) {
    let color = rope_color(&data);
    for (hook, transform) in &players {
        // The stand-in for the shoulder socket — see the module header for why this is the
        // origin and not `+ Y · eye_height_m`. When the RON grows a hand offset, this
        // expression is the whole change.
        draw_rope_lines(&mut gizmos, hook, transform.translation, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{BodyId, HookState};

    /// A buffer to draw into without an app running — `Gizmos` is only a `SystemParam` around
    /// exactly this type (`bevy_gizmos-0.19.0/src/gizmos.rs:143-175`).
    fn buffer() -> GizmoBuffer<DefaultGizmoConfigGroup, ()> {
        GizmoBuffer::new()
    }

    fn anchored(tip_m: Vec3) -> HookArm {
        HookArm {
            state: HookState::Anchored { body: BodyId(7), local_m: Vec3::ZERO },
            tip_m,
        }
    }

    #[test]
    fn f004_an_anchored_arm_draws_from_the_body_to_the_tip() {
        let arm = anchored(Vec3::new(10.0, 12.0, -34.0));
        let origin = Vec3::new(24.0, 0.0, -20.0);
        assert_eq!(
            rope_segment(&arm, origin),
            Some((origin, Vec3::new(10.0, 12.0, -34.0))),
            "a rope runs from the player to the anchor, and in that direction"
        );
    }

    #[test]
    fn f004_an_idle_flying_or_retracting_arm_draws_nothing() {
        // `tip_m` is live in three of the four states. Only one of them is a rope: drawing a
        // tip in flight would make "there is a cyan line" stop meaning "he is attached".
        let tip = Vec3::new(1.0, 2.0, 3.0);
        for state in [
            HookState::Idle,
            HookState::Flying { target_m: tip, body: BodyId(7) },
            HookState::Retracting,
        ] {
            let arm = HookArm { state, tip_m: tip };
            assert_eq!(rope_segment(&arm, Vec3::ZERO), None, "{state:?} is not a rope");
        }
    }

    #[test]
    fn f004_the_two_arms_are_independent() {
        // The half that makes the whole file falsifiable: left anchored and right free is
        // exactly one line, and it is the LEFT slot that carries it.
        let mut hook = Hook::default();
        hook.arms[Side::Left.index()] = anchored(Vec3::new(0.0, 9.0, 0.0));
        let segments = rope_segments(&hook, Vec3::ZERO);
        assert_eq!(segments[Side::Left.index()], Some((Vec3::ZERO, Vec3::new(0.0, 9.0, 0.0))));
        assert_eq!(segments[Side::Right.index()], None, "a free right arm draws nothing");

        // And the mirror image, so a swapped index cannot pass both.
        let mut hook = Hook::default();
        hook.arms[Side::Right.index()] = anchored(Vec3::new(0.0, 5.0, 0.0));
        let segments = rope_segments(&hook, Vec3::ZERO);
        assert_eq!(segments[Side::Left.index()], None, "a free left arm draws nothing");
        assert_eq!(segments[Side::Right.index()], Some((Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0))));
    }

    #[test]
    fn f004_a_fresh_hook_puts_not_one_point_in_the_buffer() {
        let mut b = buffer();
        draw_rope_lines(&mut b, &Hook::default(), Vec3::ZERO, Color::WHITE);
        assert!(b.list_positions.is_empty(), "an idle gear drew {} points", b.list_positions.len());
        assert!(b.strip_positions.is_empty());
    }

    #[test]
    fn f004_two_anchored_arms_are_two_lines_and_four_points() {
        let mut hook = Hook::default();
        hook.arms[Side::Left.index()] = anchored(Vec3::new(-4.0, 8.0, 0.0));
        hook.arms[Side::Right.index()] = anchored(Vec3::new(4.0, 8.0, 0.0));
        let mut b = buffer();
        draw_rope_lines(&mut b, &hook, Vec3::ZERO, Color::WHITE);
        // `line` writes a pair into the LIST buffer, no separator and no strip
        // (`bevy_gizmos-0.19.0/src/gizmos.rs:412`).
        assert_eq!(b.list_positions.len(), 4, "two lines are four endpoints");
        assert!(b.strip_positions.is_empty(), "a rope is a line, not a strip");
    }
}
