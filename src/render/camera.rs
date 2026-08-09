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
use crate::shared::{Intent, LocalPlayer};

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
    players: Query<&Intent, With<LocalPlayer>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
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
