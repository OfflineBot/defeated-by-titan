//! The rope constraint — the math that `F-004` and `F-005` hang on.
//!
//! **Pure functions: no Bevy except `bevy_math`, no system, no schedule, no `dt`.** The core
//! of the whole thing has to be checkable without an `App` — a pendulum you can only measure
//! in the running game is a pendulum nobody measures.
//!
//! `docs/architecture.md` translates `RopeConstraint` explicitly into "our own rope math
//! against `Time<Fixed>`, no engine constraint". The math core lives here in `shared/`
//! because it is not a system and its only caller sits in `player` — that saves one edge in
//! the allow list.
//!
//! ## Why exactly this way, and not otherwise
//!
//! Two obvious methods have been **worked through and rejected** (`docs/interface.md`,
//! section "Why the solver looks the way it does"):
//!
//! - **Pure radial projection** (clamp the position, strike the outward-pointing radial
//!   part) loses the factor `1/sqrt(1 + (v*dt/L)^2)` per tick. At `L = 3 m` and `75 m/s`
//!   that is **99.2 % of the speed per second** — exactly in the interesting range (short
//!   rope, high speed) the pendulum falls asleep. Guard: [`tests`]
//!   `f004_a_short_rope_at_high_speed_barely_loses_momentum`.
//! - **Spring/penalty** (acceleration out of the distance violation) needs, for ±1 cm at
//!   `L = 3 m, v = 75 m/s`, a stiffness of `k/m ≈ 189 500 s^-2`; symplectic Euler is
//!   unstable there at `omega*dt = 7.25` (gain −50.6 per tick) and runs into NaN within
//!   **0.41 s**.
//!
//! What stands here: **clamp the position onto the sphere, ROTATE the velocity along with
//! it.** The rotation preserves `|v|` exactly (a rotation is length-preserving); afterwards
//! the outward-pointing radial part is struck, because **a rope pulls, it does not push**.
//! That jolt is the physically real moment of going taut, and it disappears as soon as the
//! rope stays taut.

use bevy::math::{Quat, Vec3};

use super::math::direction;

/// Two hooks, therefore two possible ropes — left and right.
pub const SIDES: usize = 2;

/// A **taut** rope: where it hangs and how long it is allowed to be.
///
/// The length is an upper bound, not a target: the body may be closer to the anchor at any
/// time (then the rope is slack and this constraint does nothing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopeConstraint {
    /// Anchor point in **world coordinates**. Whoever carries the anchor on a moving body
    /// computes it beforehand (`Body` center + `local_m`).
    pub anchor_m: Vec3,
    /// Maximum distance to the anchor in meters. `<= 0` means "no constraint".
    pub length_m: f32,
}

/// What one constraint step made of position and velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstraintResult {
    pub pos_m: Vec3,
    pub velocity_m_s: Vec3,
    /// Which side really went taut in this step. Indexed like `constraints`: `0 = left`,
    /// `1 = right`. This is the quantity the caller derives `MovementState::Tethered` from —
    /// not "a hook holds".
    pub taut: [bool; SIDES],
}

/// Keeps `pos_free_m` inside every taut rope sphere and rotates the velocity along.
///
/// The caller integrates itself (`pos_free_m = pos_prev_m + velocity * dt`) and passes
/// **both** positions: `pos_prev_m` is the reference point for the rotation, `pos_free_m` the
/// unhindered result of the step.
///
/// `iterations` is the number of Gauss-Seidel passes over both constraints; it comes from
/// `assets/data/game.ron` (`vector.rope_iterations`), **not from the code**. With one pass
/// the second constraint violates the first again.
///
/// **The order is fixed** (left, then right): the same input yields bit-identical results, on
/// every machine, in every rollback.
///
/// Well-behaved against nonsense: non-finite inputs, an anchor exactly on the position, a
/// length `<= 0`, and the degenerate case "the body fell through the anchor within one tick"
/// produce no NaN.
pub fn rope_step(
    pos_prev_m: Vec3,
    pos_free_m: Vec3,
    velocity_m_s: Vec3,
    constraints: [Option<RopeConstraint>; SIDES],
    iterations: u32,
) -> ConstraintResult {
    let unchanged = ConstraintResult {
        pos_m: pos_free_m,
        velocity_m_s,
        taut: [false; SIDES],
    };
    if !(pos_prev_m.is_finite() && pos_free_m.is_finite() && velocity_m_s.is_finite()) {
        return unchanged;
    }

    let mut pos = pos_free_m;
    let mut taut = [false; SIDES];

    // At least one pass, otherwise a `0` in the RON would be a rope switched off.
    for _ in 0..iterations.max(1) {
        for (i, constraint) in constraints.iter().enumerate() {
            let Some(z) = constraint else { continue };
            if !is_valid(z) {
                continue;
            }
            let d = pos - z.anchor_m;
            if d.length() > z.length_m {
                let Some(dir) = direction(d) else { continue };
                pos = z.anchor_m + dir * z.length_m;
                taut[i] = true;
            }
        }
    }

    // The velocity is dragged along ONCE per taut rope, after the last pass. Inside the
    // passes it would be a repeated rotation by the same angle — visible as a slingshot.
    let mut tempo = velocity_m_s;
    for (i, constraint) in constraints.iter().enumerate() {
        if !taut[i] {
            continue;
        }
        let Some(z) = constraint else { continue };
        let (Some(dir_prev), Some(dir_new)) =
            (direction(pos_prev_m - z.anchor_m), direction(pos - z.anchor_m))
        else {
            continue;
        };

        // At `dir_prev ≈ -dir_new`, `Quat::from_rotation_arc` picks an ARBITRARY axis of
        // rotation (`from_axis_angle(from.any_orthonormal_vector(), PI)`,
        // glam-0.32.1/src/f32/sse2/quat.rs:337-340). Deterministic, but physically
        // meaningless: the body would have fallen through the anchor within one tick. In
        // that case nothing is rotated, only the radial part is struck.
        if dir_prev.dot(dir_new) > -0.999 {
            tempo = Quat::from_rotation_arc(dir_prev, dir_new) * tempo;
        }

        // A rope pulls, it does not push.
        let outward = tempo.dot(dir_new);
        if outward > 0.0 {
            tempo -= outward * dir_new;
        }
    }

    if !tempo.is_finite() || !pos.is_finite() {
        return unchanged;
    }

    ConstraintResult { pos_m: pos, velocity_m_s: tempo, taut }
}

/// Rope shortening that **does work** (`F-005`).
///
/// Reeling a rope in preserves angular momentum: the **tangential** velocity scales with
/// `length_prev / length_new`. Without that the player gains height but no speed — and that
/// speed is exactly the feeling the whole game hangs on (`F-005`: "the player can gain height
/// out of the low point", bible P1).
///
/// The only cap is `max_speed_m_s` (`F-012`, from `assets/data/game.ron`) — and
/// `vector.min_rope_m`, which the caller applies when carrying the length forward. None of
/// those numbers stand in the code.
///
/// The radial component is left untouched: the pulling-in itself happens through the shorter
/// length in [`rope_step`], not through a second velocity.
pub fn rope_reel_in(
    anchor_m: Vec3,
    pos_m: Vec3,
    velocity_m_s: Vec3,
    length_prev_m: f32,
    length_new_m: f32,
    max_speed_m_s: f32,
) -> Vec3 {
    if !(anchor_m.is_finite() && pos_m.is_finite() && velocity_m_s.is_finite()) {
        return velocity_m_s;
    }
    if !(length_prev_m.is_finite() && length_new_m.is_finite())
        || length_prev_m <= 0.0
        || length_new_m <= 0.0
    {
        return velocity_m_s;
    }
    let Some(dir) = direction(pos_m - anchor_m) else {
        return velocity_m_s;
    };

    let radial = velocity_m_s.dot(dir) * dir;
    let tangential = velocity_m_s - radial;
    let new_velocity = radial + tangential * (length_prev_m / length_new_m);

    if !new_velocity.is_finite() {
        return velocity_m_s;
    }
    if max_speed_m_s.is_finite() && max_speed_m_s > 0.0 {
        new_velocity.clamp_length_max(max_speed_m_s)
    } else {
        new_velocity
    }
}

fn is_valid(z: &RopeConstraint) -> bool {
    z.anchor_m.is_finite() && z.length_m.is_finite() && z.length_m > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn one(anchor_m: Vec3, length_m: f32) -> [Option<RopeConstraint>; SIDES] {
        [Some(RopeConstraint { anchor_m, length_m }), None]
    }

    /// One tick: the caller integrates, the constraint corrects.
    fn one_tick(
        pos: Vec3,
        tempo: Vec3,
        constraints: [Option<RopeConstraint>; SIDES],
        iterations: u32,
    ) -> ConstraintResult {
        rope_step(pos, pos + tempo * DT, tempo, constraints, iterations)
    }

    #[test]
    fn f004_without_a_constraint_the_step_is_the_identity() {
        let r = rope_step(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0), Vec3::X, [None, None], 2);
        assert_eq!(r.pos_m, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(r.velocity_m_s, Vec3::X);
        assert_eq!(r.taut, [false, false]);
    }

    #[test]
    fn f004_a_slack_rope_changes_nothing() {
        // The body stays inside the sphere — then no jolt may arise.
        let anchors = Vec3::new(0.0, 20.0, 0.0);
        let r = one_tick(Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0), one(anchors, 30.0), 2);
        assert_eq!(r.taut, [false, false]);
        assert_eq!(r.velocity_m_s, Vec3::new(0.0, 5.0, 0.0));
    }

    #[test]
    fn f004_the_distance_stays_on_the_sphere() {
        let anchors = Vec3::ZERO;
        let l = 12.0;
        let r = one_tick(Vec3::new(l, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), one(anchors, l), 2);
        assert!(r.taut[0], "a rope that had to go taut did not");
        let distance = (r.pos_m - anchors).length();
        assert!((distance - l).abs() < 1e-4, "distance {distance} instead of {l}");
    }

    #[test]
    fn f004_a_rope_pulls_and_does_not_push() {
        // Purely radial and outward: after going taut, nothing of it is left.
        let anchors = Vec3::ZERO;
        let l = 10.0;
        let r = one_tick(Vec3::new(l, 0.0, 0.0), Vec3::new(40.0, 0.0, 0.0), one(anchors, l), 2);
        assert!(r.velocity_m_s.length() < 1e-3, "remaining velocity {}", r.velocity_m_s);
    }

    #[test]
    fn f004_a_short_rope_at_high_speed_barely_loses_momentum() {
        // **The guard against pure radial projection.** That one loses 99.2 % per second at
        // L = 3 m and 75 m/s; rotating along preserves |v| exactly. Without gravity, so
        // that only the solver is measured.
        let anchors = Vec3::ZERO;
        let l = 3.0;
        let mut pos = Vec3::new(l, 0.0, 0.0);
        let mut tempo = Vec3::new(0.0, 0.0, 75.0);
        for _ in 0..60 {
            let r = one_tick(pos, tempo, one(anchors, l), 2);
            pos = r.pos_m;
            tempo = r.velocity_m_s;
        }
        let magnitude = tempo.length();
        assert!(
            magnitude > 75.0 * 0.99,
            "after one second still {magnitude} m/s instead of ~75 — the solver eats momentum"
        );
        let distance = (pos - anchors).length();
        assert!((distance - l).abs() < 1e-3, "distance {distance} instead of {l}");
    }

    #[test]
    fn f004_two_anchors_hold_both_spheres() {
        // The case F-004 is named after. Two roofs, the body hangs below them and is pulled
        // between the two.
        let a = Vec3::new(-8.0, 14.0, 0.0);
        let b = Vec3::new(8.0, 14.0, 0.0);
        let constraints = [
            Some(RopeConstraint { anchor_m: a, length_m: 18.0 }),
            Some(RopeConstraint { anchor_m: b, length_m: 18.0 }),
        ];
        let r = one_tick(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, -25.0, 0.0), constraints, 8);
        assert_eq!(r.taut, [true, true]);
        assert!((r.pos_m - a).length() <= 18.0 + 1e-3, "left rope overextended");
        assert!((r.pos_m - b).length() <= 18.0 + 1e-3, "right rope overextended");
        assert!(r.velocity_m_s.is_finite());
    }

    #[test]
    fn f004_the_degenerate_case_produces_no_nan() {
        // A `warp` can put the body through the anchor in one step; then
        // `dir_prev ≈ -dir_new` and `from_rotation_arc` picks an arbitrary axis.
        let anchors = Vec3::ZERO;
        let l = 4.0;
        let r = rope_step(
            Vec3::new(l, 0.0, 0.0),
            Vec3::new(-40.0, 0.0, 0.0),
            Vec3::new(-30.0, 0.0, 0.0),
            one(anchors, l),
            2,
        );
        assert!(r.pos_m.is_finite() && r.velocity_m_s.is_finite());
        assert!(((r.pos_m - anchors).length() - l).abs() < 1e-4);
    }

    #[test]
    fn f004_nonsensical_input_comes_back_unchanged() {
        let kaputt = Vec3::new(f32::NAN, 0.0, 0.0);
        let r = rope_step(kaputt, Vec3::ZERO, Vec3::X, one(Vec3::ZERO, 5.0), 2);
        assert_eq!(r.velocity_m_s, Vec3::X);
        assert_eq!(r.taut, [false, false]);

        // Length 0 is "no constraint", not a division by zero.
        let r = one_tick(Vec3::new(5.0, 0.0, 0.0), Vec3::X * 10.0, one(Vec3::ZERO, 0.0), 2);
        assert_eq!(r.taut, [false, false]);
        assert!(r.pos_m.is_finite());
    }

    #[test]
    fn f005_reel_in_accelerates_tangentially() {
        // Conservation of angular momentum: half the length, twice the tangential speed.
        let anchors = Vec3::ZERO;
        let pos = Vec3::new(30.0, 0.0, 0.0);
        let new_velocity = rope_reel_in(anchors, pos, Vec3::new(0.0, 0.0, 20.0), 30.0, 15.0, 75.0);
        assert!((new_velocity.z - 40.0).abs() < 1e-4, "tangential {new_velocity:?}, expected 40 m/s");
        assert!(new_velocity.x.abs() < 1e-6, "the radial component must not change");
    }

    #[test]
    fn f005_reel_in_leaves_the_radial_component_alone() {
        let anchors = Vec3::ZERO;
        let pos = Vec3::new(20.0, 0.0, 0.0);
        let new_velocity = rope_reel_in(anchors, pos, Vec3::new(-6.0, 0.0, 10.0), 20.0, 10.0, 75.0);
        assert!((new_velocity.x + 6.0).abs() < 1e-4, "radial {new_velocity:?}");
        assert!((new_velocity.z - 20.0).abs() < 1e-4, "tangential {new_velocity:?}");
    }

    #[test]
    fn f005_reel_in_caps_at_max_speed() {
        let new_velocity = rope_reel_in(
            Vec3::ZERO,
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 60.0),
            40.0,
            4.0,
            75.0,
        );
        assert!(new_velocity.length() <= 75.0 + 1e-3, "magnitude {}", new_velocity.length());
    }

    #[test]
    fn f005_without_a_length_change_reel_in_changes_nothing() {
        let t = Vec3::new(3.0, -4.0, 12.0);
        let new_velocity = rope_reel_in(Vec3::ZERO, Vec3::new(9.0, 0.0, 0.0), t, 9.0, 9.0, 75.0);
        assert!((new_velocity - t).length() < 1e-4, "{new_velocity:?} instead of {t:?}");
    }

    #[test]
    fn f005_reel_in_with_nonsense_returns_the_velocity_unchanged() {
        let t = Vec3::new(1.0, 2.0, 3.0);
        // Length zero: no division by zero.
        assert_eq!(rope_reel_in(Vec3::ZERO, Vec3::X * 5.0, t, 10.0, 0.0, 75.0), t);
        // Anchor exactly on the position: no direction, so no math.
        assert_eq!(rope_reel_in(Vec3::ZERO, Vec3::ZERO, t, 10.0, 5.0, 75.0), t);
        // No NaN comes out if none goes in.
        assert_eq!(rope_reel_in(Vec3::ZERO, Vec3::X * 5.0, t, f32::NAN, 5.0, 75.0), t);
    }
}
