//! The pose — **a pure function of `(TitanState, ticks_in_state)`, never of a clock.**
//!
//! ## Why `AnimationPlayer` is not used, although it is right there
//!
//! `bevy_animation` is in `DefaultPlugins` (`bevy_internal-0.19.0/src/default_plugins.rs:85`)
//! and would do this in three lines. It advances on `Time`, not on `Time<Fixed>` — so the arm
//! angle in frame *n* would depend on how long frame *n−1* took. Everything this project
//! calls evidence hangs off one property: an `--offscreen` run with the same script produces a
//! **bit-identical** PNG (`docs/ACCEPTANCE.md`). A pose read off the wall clock breaks that,
//! and it breaks it silently: nothing errors, the `sha256` simply stops matching. It is also
//! the same argument as [`HitStop`](crate::shared::HitStop)'s — a tick counter, never a clock.
//!
//! So: the pose takes an integer, `ticks_in_state`, and returns two angles. Two runs at
//! different frame rates that reach tick *n* have the same pose, to the bit.
//! `tests/titan.rs::f050_the_pose_does_not_depend_on_the_clock` is the guard.
//!
//! ## What is deliberately not in here
//!
//! No walk cycle. The legs are two rigid boxes and the feet slide — `docs/PLAN-GAME.md` §12
//! says so in as many words, and inventing a stride length would be inventing a number that
//! belongs in `scale.ron`.

use bevy::prelude::*;

use crate::shared::TitanState;

use super::brain::TitanTiming;
use super::rig::{arm_transform, torso_transform, TitanPart, TitanRig};

/// The three pose angles out of `scale.ron: titan`, in **degrees**, baked onto the body at
/// spawn.
///
/// They stand in the file and not here because "raise the arm 140°" in Rust is an angle
/// nobody tunes after the first person plays it (rule 2). They are properties of the **rig**,
/// which is why there is one set for all kinds and not twenty-seven.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PoseAngles {
    pub windup_arm_deg: f32,
    pub windup_lean_deg: f32,
    pub strike_arm_deg: f32,
}

/// What the rig looks like this tick: one arm angle, one lean angle, both in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    /// Hinge angle of the **striking** (right) arm.
    pub arm_deg: f32,
    /// Lean of the torso about the hip. Positive tips the shoulders back.
    pub lean_deg: f32,
}

/// **The whole feature, in one pure function.**
///
/// `ticks_in_state` is the number of ticks *already completed* in `state`, so the entry tick
/// of a state is 0 and the pose starts at the beginning of its ramp.
pub fn pose_of(state: TitanState, ticks_in_state: u32, t: &TitanTiming, a: &PoseAngles) -> Pose {
    match state {
        // Nothing to telegraph. `Death` too: the body dissolves by scale, and a corpse that
        // keeps swinging its arm is worse than one that does not.
        TitanState::Idle | TitanState::Pursue | TitanState::Death => Pose::default(),
        TitanState::Windup => Pose {
            arm_deg: lerp(0.0, a.windup_arm_deg, fraction(ticks_in_state, t.windup_ticks)),
            lean_deg: lerp(0.0, a.windup_lean_deg, fraction(ticks_in_state, t.windup_ticks)),
        },
        TitanState::Strike => Pose {
            arm_deg: lerp(
                a.windup_arm_deg,
                a.strike_arm_deg,
                fraction(ticks_in_state, t.strike_ticks),
            ),
            lean_deg: lerp(a.windup_lean_deg, 0.0, fraction(ticks_in_state, t.strike_ticks)),
        },
        TitanState::Recover => Pose {
            arm_deg: lerp(
                a.strike_arm_deg,
                0.0,
                fraction(ticks_in_state, t.recover_ticks),
            ),
            lean_deg: 0.0,
        },
    }
}

/// How far through a state we are, as a fraction. A state of zero ticks is over at once
/// instead of dividing by zero.
fn fraction(ticks_in_state: u32, duration_ticks: u32) -> f32 {
    if duration_ticks == 0 {
        return 1.0;
    }
    (ticks_in_state as f32 / duration_ticks as f32).clamp(0.0, 1.0)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// Writes the pose onto the torso and the two arms.
///
/// Only the **right** arm swings; the left hangs. One striking arm is what the three angles in
/// `scale.ron` describe, and a second animated arm would need a second set of numbers that
/// nobody has written.
///
/// It walks **down** from each body instead of up from each box, so that one titan's numbers
/// are read once instead of once per box — and so that a stray `TitanPart` without a body over
/// it is simply not posed rather than posed from the wrong titan.
pub(super) fn apply_pose(
    bodies: Query<
        (
            Entity,
            &TitanState,
            &super::brain::TitanClock,
            &TitanTiming,
            &PoseAngles,
            &TitanRig,
        ),
        With<super::rig::TitanBody>,
    >,
    children: Query<&Children>,
    mut parts: Query<(&TitanPart, &mut Transform)>,
) {
    for (root, state, clock, timing, angles, rig) in &bodies {
        let pose = pose_of(*state, clock.ticks_in_state, timing, angles);
        let mut pending = vec![root];
        while let Some(entity) = pending.pop() {
            if let Ok(kids) = children.get(entity) {
                pending.extend(kids.iter());
            }
            let Ok((part, mut transform)) = parts.get_mut(entity) else {
                continue;
            };
            let wanted = match part {
                TitanPart::Torso => torso_transform(rig, pose.lean_deg),
                TitanPart::ArmRight => arm_transform(rig, true, pose.arm_deg),
                TitanPart::ArmLeft => arm_transform(rig, false, 0.0),
                // Pelvis, legs, head and cortex hold still relative to their parent — the
                // lean and the swing reach them through the hierarchy, which is the whole
                // reason the cortex hangs under the head.
                _ => continue,
            };
            if *transform != wanted {
                *transform = wanted;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> TitanTiming {
        TitanTiming { windup_ticks: 36, strike_ticks: 12, recover_ticks: 24, cooldown_ticks: 90, death_ticks: 60 }
    }

    fn angles() -> PoseAngles {
        PoseAngles { windup_arm_deg: 140.0, windup_lean_deg: 12.0, strike_arm_deg: -30.0 }
    }

    #[test]
    fn the_windup_starts_at_rest_and_ends_at_the_files_angle() {
        let (t, a) = (timing(), angles());
        assert_eq!(pose_of(TitanState::Windup, 0, &t, &a).arm_deg, 0.0);
        // Tick 36 is already `Strike`; the last tick INSIDE the windup is 35, and the ramp
        // has to have arrived by the time the state changes, or the arm jumps.
        let last = pose_of(TitanState::Windup, 35, &t, &a).arm_deg;
        assert!(last > 130.0 && last < 140.0, "arm at tick 35 of 36: {last}");
        assert_eq!(pose_of(TitanState::Windup, 36, &t, &a).arm_deg, 140.0);
    }

    #[test]
    fn the_strike_carries_the_arm_past_the_rest_pose() {
        let (t, a) = (timing(), angles());
        assert_eq!(pose_of(TitanState::Strike, 0, &t, &a).arm_deg, 140.0);
        assert_eq!(pose_of(TitanState::Strike, 12, &t, &a).arm_deg, -30.0);
        // It really passes through the hanging pose — that is what makes it a swing and not
        // a fade between two positions.
        assert!(pose_of(TitanState::Strike, 8, &t, &a).arm_deg < 30.0);
    }

    #[test]
    fn the_pose_is_the_same_for_the_same_tick() {
        // The property the whole evidence route rests on, at the level of the function.
        let (t, a) = (timing(), angles());
        for tick in 0..40 {
            assert_eq!(
                pose_of(TitanState::Windup, tick, &t, &a),
                pose_of(TitanState::Windup, tick, &t, &a)
            );
        }
    }

    #[test]
    fn a_state_of_zero_ticks_does_not_divide_by_zero() {
        let a = angles();
        let t = TitanTiming { windup_ticks: 0, ..timing() };
        let p = pose_of(TitanState::Windup, 0, &t, &a);
        assert!(p.arm_deg.is_finite() && p.lean_deg.is_finite());
    }

    #[test]
    fn idle_pursue_and_death_carry_no_pose() {
        let (t, a) = (timing(), angles());
        for state in [TitanState::Idle, TitanState::Pursue, TitanState::Death] {
            assert_eq!(pose_of(state, 17, &t, &a), Pose::default(), "{state:?}");
        }
    }
}
