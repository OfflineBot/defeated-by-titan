//! The camera turns — **or every image shows something other than what the ray measures.**
//!
//! Until 2026-08-09 `src/` contained exactly one rotation: the sun. The camera hangs on the
//! player as a child with `Transform::from_xyz(0, eye_height_m, 0)`, i.e. rotated by the
//! identity, and nobody ever wrote `Intent.yaw/pitch` into a `Transform`. It therefore
//! **always** looked at −Z while the aim ray goes by `intent.look_dir()`. If a script said
//! `look 30 -10`, the ray aimed somewhere other than the image — and every screenshot
//! criterion would have been worthless without anyone noticing.
//!
//! **What turns is the CAMERA, not the player.** The collision box hangs on the player; if
//! it turns along, the axis-aligned aabb is no longer an axis-aligned aabb.
//!
//! Row in the authority table: `Transform of the camera | render`.
//!
//! **No interpolation between simulation steps.** It would need a second writer on the
//! player `Transform` or a presentation entity of its own — both are a design of their own
//! and stand in `docs/ROADMAP.md`, not in this assignment.
//!
//! **No smoothing either.** `game.ron: camera.smoothing_half_life_s` stays unread here:
//! smoothed, the image shows a **different** direction than `intent.look_dir()` throughout
//! every turn, and precisely that equality is the acceptance criterion (`tests/render.rs`).
//! Whoever wants smoothing smooths the look angle in the `Intent` first — then image and ray
//! stay together. So the value belongs to nobody yet.
//!
//! **Evidence:** `tests/render.rs` · `docs/images/f002-look.png` and
//! `docs/images/f002-look-turned.png` out of `scripts/f002-look.txt` and
//! `scripts/f002-look-turned.txt` respectively.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{HitZone, Intent, LocalPlayer, Tick, TitanHit};

/// `F-034`'s camera kick — **purely visual, and a pure function of the tick.**
///
/// Two things it is deliberately not. It is not a simulation value: nothing reads it back, and
/// the aim ray keeps going by `intent.look_dir()` while the image is kicked, exactly the way
/// `smoothing_half_life_s` is left unread above. And it does **not decay off a clock**: this
/// system runs in `Update`, so a `Time`-driven decay would put a different pitch in the image
/// on every frame rate — and with it a different `--offscreen` sha256, which is the one thing
/// the whole evidence route rests on (`docs/PLAN-GAME.md` §11, risk 3). It decays over
/// `round(camera_kick_s × simulation_hz)` **ticks**.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraKick {
    /// The tick the hit landed on.
    pub from_tick: u64,
    /// How many ticks the kick lasts. Zero means there is no kick.
    pub ticks: u32,
    /// Full amplitude in radians, at `from_tick`.
    pub amplitude_rad: f32,
}

impl CameraKick {
    /// The extra pitch this tick, in radians. Linear from full to nothing and then exactly
    /// zero — not an exponential that is "almost" zero forever, because "almost zero" means
    /// the camera never returns to the angle the ray measures.
    pub fn pitch_rad(&self, tick: u64) -> f32 {
        if self.ticks == 0 {
            return 0.0;
        }
        let elapsed = tick.saturating_sub(self.from_tick);
        if elapsed >= self.ticks as u64 {
            return 0.0;
        }
        let left = 1.0 - elapsed as f32 / self.ticks as f32;
        self.amplitude_rad * left
    }
}

/// The yaw of a direction in [`Intent::look_dir`]'s own convention — the yaw at which a
/// player with pitch 0 would look exactly along `dir`'s horizontal part.
///
/// `look_dir = (−sin y · cos p, sin p, −cos y · cos p)`, so the horizontal part is
/// `(−sin y, −cos y)` and the inverse is `atan2(−x, −z)`. `None` for a vertical or zero
/// direction — a rope straight above the player constrains no yaw at all.
pub fn direction_yaw(dir: Vec3) -> Option<f32> {
    if dir.x.abs() < 1e-6 && dir.z.abs() < 1e-6 {
        return None;
    }
    Some((-dir.x).atan2(-dir.z))
}

/// §5C's look clamp — *„wenn man hoocked ist soll man auch nicht zu stark über 80deg
/// links/rechts schauen. also schon einiges aber nicht zu viel drehen!"* — as a pure function.
///
/// ```text
/// Δ = wrap_to_±π(yaw − rope_yaw)
/// |Δ| ≤ limit          → yaw, untouched
/// |Δ| > limit          → rope_yaw ± (limit + soft · tanh((|Δ| − limit) / soft))
/// ```
///
/// **The edge is a RUBBER BAND, and the curve says exactly how.** Inside ±`limit` the clamp
/// does not exist. Past it the excess is compressed through `tanh`: the slope is 1 at the
/// limit (no kink to feel — resistance grows, it does not begin with a wall) and the yaw
/// saturates asymptotically at `limit + soft`, which the view can therefore never pass. Hard
/// stop and slow-return were both considered and rejected: a hard stop is a wall at exactly
/// the angle he asked to still be usable („also schon einiges … drehen"), and a slow-return
/// needs per-tick state plus a rate — two more numbers — where the band needs none.
///
/// **Stateless on purpose, and that decides where it must be wired.** Applied to the look
/// ACCUMULATOR (`net::local::read_input`'s `Look`, the honest owner of yaw — see
/// `docs/QUESTIONS.md` Q-091), the band holds while hooked and releasing the rope simply
/// stops the clamping: the accumulator already IS inside the band, so nothing snaps. Applied
/// to the camera instead it would make the image disagree with `intent.look_dir()` — the one
/// equality this module's header defends (`tests/render.rs`) — which is why `rotate_camera`
/// deliberately does NOT call it.
///
/// ⚠️ The 80° and 10° are game values and belong in `game.ron: camera` (rule 2); the keys do
/// not exist yet because `src/data/mod.rs` is another stream's file today — Q-091 names them.
/// This function takes radians and no defaults, so the wiring cannot forget the file.
#[must_use]
pub fn hooked_yaw_soft_clamp(yaw: f32, rope_yaw: f32, limit_rad: f32, soft_rad: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    // The offset from the rope, wrapped to ±π: yaw accumulates freely across turns
    // (`net::local::Look` has no modulus), and an unwrapped subtraction would clamp a player
    // whose yaw and rope only LOOK far apart by a winding.
    let delta = (yaw - rope_yaw + PI).rem_euclid(TAU) - PI;
    let excess = delta.abs() - limit_rad;
    if excess <= 0.0 || !(soft_rad > 0.0) {
        return yaw;
    }
    let allowed = limit_rad + soft_rad * (excess / soft_rad).tanh();
    // Returned in the caller's own winding: shift by what the wrap took out, so the result is
    // continuous in `yaw` even at the ±π seam.
    yaw - delta + delta.signum() * allowed
}

/// Puts `yaw` and `pitch` out of the local player's [`Intent`] onto the camera.
///
/// Runs in `Update` and not in the fixed step: presentation is not simulation, and a camera
/// that only follows 60 times a second feels wrong on a 144 Hz screen.
///
/// The rotation is `Ry(yaw) * Rx(pitch)` and **no roll**. In that order
/// `Transform::forward() == Intent::look_dir()` holds exactly — both are worked through and
/// nailed down in `tests/render.rs`:
///
/// - `bevy_transform-0.19.0/src/components/transform.rs:317-326` — `forward()` is
///   `-(rotation * Vec3::Z)`, i.e. `rotation * NEG_Z`.
/// - `glam-0.32.1/src/f32/sse2/quat.rs:170-181` — `from_rotation_x`/`from_rotation_y` are
///   the ordinary right-handed axis rotations.
/// - `Rx(pitch) * NEG_Z = (0, sin p, -cos p)`, and `Ry(yaw)` on top of that gives
///   `(-sin y · cos p, sin p, -cos y · cos p)` — sign for sign
///   `Intent::look_dir()` (`src/shared/intent.rs:42`).
pub fn rotate_camera(
    data: Res<GameData>,
    tick: Res<Tick>,
    mut hits: MessageReader<TitanHit>,
    mut kick: Local<CameraKick>,
    players: Query<&Intent, With<LocalPlayer>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    // `F-034`'s kick. A `Local` and not a component: it belongs to this system and to nobody
    // else, nothing reads it back, and there is exactly one 3D camera (`attach_camera`).
    // The message is drained even when there is no camera yet, so a hit cannot arrive twice.
    for hit in hits.read() {
        // Only the kill kicks. A kick on every scratch is a camera nobody can aim through,
        // and `hit_stop_normal_s` already says a non-lethal hit is the small event.
        if hit.zone != HitZone::Cortex {
            continue;
        }
        let k = &data.gear.feel;
        let ticks = (k.camera_kick_s as f64 * data.game.simulation_hz).round();
        *kick = CameraKick {
            from_tick: tick.0,
            ticks: if ticks.is_finite() && ticks > 0.0 { ticks as u32 } else { 0 },
            // Degrees in the file, radians in the code, converted at the boundary
            // (`docs/conventions.md`). Downward, so the image dips into the cut.
            amplitude_rad: -k.camera_kick_deg.to_radians(),
        };
    }

    // There is no "the player" — but exactly one who is ME (§6 rule 3). If he does not exist
    // (yet), that is not an error: the world is only just being built.
    let Some(intent) = players.iter().next() else {
        return;
    };

    // The limit is a tuning number and stands in `assets/data/game.ron`, not here (rule 2).
    // Clamping happens **here too**, not only in `net::local`: over there only the mouse
    // path is clamped, and an `Intent` can later come out of the network as well. A camera
    // that tips over the zenith stands on its head.
    let limit = data.game.camera.pitch_limit_deg.to_radians();
    let pitch = intent.pitch.clamp(-limit, limit);

    // Ry(yaw) * Rx(pitch), in exactly that order and without roll. Not via
    // `Quat::from_euler`: its axis order would have to be looked up, this one is written
    // out.
    // The kick rides on the pitch and is clamped with it: a kick past the zenith would stand
    // the image on its head, and the clamp is the same one the mouse path obeys.
    let pitch = (pitch + kick.pitch_rad(tick.0)).clamp(-limit, limit);

    let rotation = Quat::from_rotation_y(intent.yaw) * Quat::from_rotation_x(pitch);

    // `With<Camera3d>` without a further filter is enough because there is **at most one**
    // 3D camera: `render::attach_camera` bails out as soon as one exists. If that guarantee
    // falls, it falls one file further up, not here.
    for mut t in &mut camera {
        // Only write when something really changes — otherwise change detection reports a
        // rotation every frame, and transform propagation works through the camera branch
        // again for every image (rule 6).
        if t.rotation != rotation {
            t.rotation = rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // §5C's look clamp, pure math. What the fixture varies: the yaw offset from the rope, the
    // limit and the softness. What the code reads: exactly those three plus the rope yaw. The
    // WIRING — who owns the yaw and where the clamp acts on the real accumulator — is
    // `docs/QUESTIONS.md` Q-091; these tests hold the curve itself.

    const LIMIT: f32 = 80.0_f32 * std::f32::consts::PI / 180.0;
    const SOFT: f32 = 10.0_f32 * std::f32::consts::PI / 180.0;

    fn deg(d: f32) -> f32 {
        d.to_radians()
    }

    #[test]
    fn f5c_inside_the_limit_the_yaw_is_untouched() {
        // „also schon einiges … drehen" — up to the limit the clamp must be invisible, and
        // exactly at the limit too (the band starts past it, not on it).
        for off in [0.0, 30.0, -60.0, 79.9, -80.0] {
            let yaw = deg(off);
            let out = hooked_yaw_soft_clamp(yaw, 0.0, LIMIT, SOFT);
            assert!(
                (out - yaw).abs() < 1e-6,
                "{off}° off the rope was moved to {:.3}° — inside ±80° the clamp must not exist",
                out.to_degrees()
            );
        }
    }

    #[test]
    fn f5c_past_the_limit_the_band_resists_and_saturates() {
        // „aber nicht zu viel drehen!" — the edge is SOFT: a rubber band, not a wall. Past the
        // limit the excess is compressed through tanh, so the felt resistance grows smoothly
        // (slope 1 at the limit — no kink to feel) and the yaw can NEVER pass limit + soft.
        let at_100 = hooked_yaw_soft_clamp(deg(100.0), 0.0, LIMIT, SOFT);
        let expected = LIMIT + SOFT * (deg(20.0) / SOFT).tanh();
        assert!(
            (at_100 - expected).abs() < 1e-5,
            "100° off the rope came out {:.3}° — the band is limit + soft·tanh(excess/soft), \
             which is {:.3}°",
            at_100.to_degrees(),
            expected.to_degrees()
        );
        // Monotone: pushing harder still moves the view, only ever less — a band, not a stop.
        let at_120 = hooked_yaw_soft_clamp(deg(120.0), 0.0, LIMIT, SOFT);
        assert!(
            at_120 > at_100 && at_120 < LIMIT + SOFT,
            "120° gave {:.3}° against {:.3}° at 100° — past the limit the band must keep \
             giving ground, and never past limit + soft = {:.3}°",
            at_120.to_degrees(),
            at_100.to_degrees(),
            (LIMIT + SOFT).to_degrees()
        );
        // And it is symmetric: the left edge is the right edge mirrored.
        let left = hooked_yaw_soft_clamp(deg(-100.0), 0.0, LIMIT, SOFT);
        assert!(
            (left + at_100).abs() < 1e-5,
            "−100° gave {:.3}° but +100° gave {:.3}° — the two edges are one curve",
            left.to_degrees(),
            at_100.to_degrees()
        );
    }

    #[test]
    fn f5c_the_clamp_is_relative_to_the_rope_not_to_north() {
        // The user's 80° is measured FROM THE ROPE. Same 100° excess, rope at 90°: the answer
        // is the rope's yaw plus the band, in the yaw's own winding.
        let rope = deg(90.0);
        let out = hooked_yaw_soft_clamp(rope + deg(100.0), rope, LIMIT, SOFT);
        let expected = rope + LIMIT + SOFT * (deg(20.0) / SOFT).tanh();
        assert!(
            (out - expected).abs() < 1e-5,
            "with the rope at 90° a look at 190° came out {:.3}° instead of {:.3}°",
            out.to_degrees(),
            expected.to_degrees()
        );
        // And the delta wraps: a look at −170° relative to a rope at 170° is 20° AWAY, not
        // 340° — an unwrapped subtraction would clamp a player who is nearly aligned.
        let near = hooked_yaw_soft_clamp(deg(-170.0), deg(170.0), LIMIT, SOFT);
        assert!(
            (near - deg(-170.0)).abs() < 1e-6,
            "−170° against a rope at 170° is 20° of offset and was still moved to {:.3}°",
            near.to_degrees()
        );
    }

    #[test]
    fn f5c_a_vertical_rope_constrains_no_yaw() {
        // Straight up (or down), every yaw looks equally away from the rope — there is no
        // angle to clamp against, and `direction_yaw` says so instead of inventing one.
        assert_eq!(direction_yaw(Vec3::Y), None);
        assert_eq!(direction_yaw(Vec3::new(0.0, -3.0, 0.0)), None);
        // And the convention round-trips: a direction built from a yaw yields that yaw.
        for y in [0.0_f32, 1.0, -2.5] {
            let dir = Vec3::new(-y.sin(), 0.0, -y.cos());
            let back = direction_yaw(dir).expect("horizontal");
            assert!(
                (back - y).abs() < 1e-5,
                "yaw {y} round-tripped to {back} through direction_yaw"
            );
        }
    }
}
