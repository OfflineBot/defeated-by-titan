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
