//! Math helpers — **nothing changes per frame, everything per second.**
//!
//! `* dt` alone is not enough. The three traps cost half a day each, all of them look like
//! "the game feels different on my machine", and **over a wire they are desync**
//! (`prompts/init.md` §11, §6 rule 4):
//!
//! 1. **Never round to integers.** `(damage * dt).ceil()` turns the frame rate into the
//!    damage number. Carry the fractions along.
//! 2. **Exponential smoothing is per frame.** `x += (target - x) * 0.1` hangs on the frame
//!    rate — use [`smooth`].
//! 3. **Noise scales with `sqrt(dt)`**, not with `dt` — [`noise_scale`].
//!
//! There is **one** helper per case, and only that one gets used. Two forms for the same
//! thing means no form.

use bevy::prelude::*;

/// Upper bound for one time step.
///
/// A frame can take 0.5 s (a reload, a moved window, Blender in the background). Unclamped,
/// exactly that frame pushes the player through the wall and produces NaN in the rope
/// forces — the bug that looks like "the player is gone" (§9d).
pub const DT_MAX_S: f32 = 0.1;

pub fn clamped_dt_s(dt_s: f32) -> f32 {
    if dt_s.is_finite() { dt_s.clamp(0.0, DT_MAX_S) } else { 0.0 }
}

/// Exponential smoothing over a **half life**, independent of the frame rate.
///
/// `half_life_s` is the time after which half of the difference is gone — a number you can
/// write into a RON and check in your head. `0` means immediately.
pub fn smooth(current: f32, target: f32, half_life_s: f32, dt_s: f32) -> f32 {
    if half_life_s <= 0.0 {
        return target;
    }
    let fraction = 1.0 - (-core::f32::consts::LN_2 * clamped_dt_s(dt_s) / half_life_s).exp();
    current + (target - current) * fraction
}

pub fn smooth_vec3(current: Vec3, target: Vec3, half_life_s: f32, dt_s: f32) -> Vec3 {
    Vec3::new(
        smooth(current.x, target.x, half_life_s, dt_s),
        smooth(current.y, target.y, half_life_s, dt_s),
        smooth(current.z, target.z, half_life_s, dt_s),
    )
}

/// Scaling for **noise** (camera shake, spread): `sqrt(dt)`, not `dt`.
///
/// Noise is a random walk; its standard deviation grows with the square root of time. Scaled
/// with `dt` it becomes invisible at a high frame rate and an earthquake at a low one.
pub fn noise_scale(dt_s: f32) -> f32 {
    clamped_dt_s(dt_s).sqrt()
}

/// Safe normalization. `None` means "that direction does not exist" — and the caller has to
/// make up its mind instead of passing a NaN along.
///
/// `Vec3::normalize` on a zero vector yields NaN, and a NaN in the `Transform` is the bug
/// you only see three systems later (§9d).
pub fn direction(v: Vec3) -> Option<Vec3> {
    let l = v.length();
    if l.is_finite() && l > 1e-6 { Some(v / l) } else { None }
}

/// Whether a value can be a position at all. `debug/` warns **once** if it cannot.
pub fn is_finite(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_is_frame_rate_independent() {
        // The actual purpose: the same span of time must produce the same result, no matter
        // how many steps it is walked in. That is exactly what `x += (target-x)*0.1`
        // CANNOT do — and exactly that is a desync over a wire.
        //
        // Both runs cover **one second**, and both stay below DT_MAX_S with their step size
        // — otherwise the test measures the clamp instead of the smoothing (see the test
        // below).
        let target = 10.0;
        let hz = 0.25;

        let mut coarse = 0.0;
        for _ in 0..12 {
            coarse = smooth(coarse, target, hz, 1.0 / 12.0);
        }
        let mut fine = 0.0;
        for _ in 0..60 {
            fine = smooth(fine, target, hz, 1.0 / 60.0);
        }
        assert!(
            (coarse - fine).abs() < 1e-3,
            "12 steps gave {coarse}, 60 steps {fine} — the same second must produce \
             the same result"
        );
        // Four half lives in one second: 1 - 1/16 = 0.9375 of 10.
        assert!((fine - 9.375).abs() < 1e-3, "expected 9.375, was {fine}");
    }

    #[test]
    fn a_hitch_is_clamped_not_caught_up() {
        // Beyond DT_MAX_S the frame rate independence is violated **on purpose**: a
        // 0.5 s frame must not execute 0.5 s of movement in one go, otherwise exactly that
        // frame pushes the player through the wall (§9d). The smoothing catches up
        // afterwards, it does not jump.
        //
        // This line stands here so that the difference is a DECISION and does not get
        // reported as a bug one day.
        let one_hitch = smooth(0.0, 10.0, 0.25, 0.5);
        let clamped = smooth(0.0, 10.0, 0.25, DT_MAX_S);
        assert_eq!(
            one_hitch, clamped,
            "a half-second frame must act like DT_MAX_S, not like half a second"
        );
        assert!(one_hitch < 3.0, "and it must not jump almost all the way to the target");
    }

    #[test]
    fn smoothing_keeps_its_half_life() {
        let x = smooth(0.0, 1.0, 0.1, 0.1);
        assert!((x - 0.5).abs() < 1e-5, "after one half life expected 0.5, was {x}");
    }

    #[test]
    fn dt_is_clamped_and_nan_becomes_zero() {
        assert_eq!(clamped_dt_s(0.5), DT_MAX_S);
        assert_eq!(clamped_dt_s(-1.0), 0.0);
        assert_eq!(clamped_dt_s(f32::NAN), 0.0);
        assert_eq!(clamped_dt_s(f32::INFINITY), 0.0);
    }

    #[test]
    fn smoothing_never_produces_nan() {
        // The edge case, not the normal case: a frame of 0.5 s and a half life of 0 are
        // exactly the values that really do turn up in the game.
        for hz in [0.0_f32, 1e-9, 0.5, 1e9] {
            for dt in [0.0_f32, 1.0 / 60.0, 0.5, f32::NAN] {
                let x = smooth(1.0, 2.0, hz, dt);
                assert!(x.is_finite(), "hz {hz}, dt {dt} gave {x}");
            }
        }
    }

    #[test]
    fn direction_rejects_the_zero_vector() {
        assert!(direction(Vec3::ZERO).is_none());
        assert!(direction(Vec3::splat(f32::NAN)).is_none());
        assert!(direction(Vec3::new(1e-9, 0.0, 0.0)).is_none());
        let d = direction(Vec3::new(0.0, 0.0, -3.0)).expect("3 m is a direction");
        assert!((d - Vec3::NEG_Z).length() < 1e-6);
    }

    #[test]
    fn noise_scales_with_the_square_root() {
        // A step four times as long means twice as much noise, not four times.
        let a = noise_scale(1.0 / 240.0);
        let b = noise_scale(4.0 / 240.0);
        assert!((b / a - 2.0).abs() < 1e-4, "ratio was {}", b / a);
    }
}
