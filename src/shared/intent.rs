//! `Intent` — **input is a datum, not a key press.**
//!
//! There is exactly one struct, and the simulation reads **only** that. Who fills it is none
//! of its business: the local keyboard, the `--script` driver, or later the network. That
//! channel is precisely the one multiplayer needs — and it gets built anyway, because in
//! this environment nobody can click. **One effort, two problems solved**
//! (`prompts/init.md` §6 rule 2, §12).
//!
//! Deliberately **no `Vec2`/`Vec3` fields**: this type goes over a wire one day and gets
//! saved. Bare `f32` are what `serde` can do without an extra feature, and they say exactly
//! how many bytes it is (§6 rule 8).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// What a player wants in **one** simulation tick.
///
/// Hangs as a component on the player — not as a `Resource`, because there is no such thing
/// as "the player" (§6 rule 3).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    /// Movement in the plane, player-local: `x` to the right, `y` forward, each -1..1.
    pub move_x: f32,
    pub move_y: f32,
    /// Look direction in **radians**. `yaw = 0` means looking towards −Z
    /// (`docs/conventions.md`).
    pub yaw: f32,
    /// Positive is up, clamped to ±89°.
    pub pitch: f32,
    /// Pressed buttons as a bit pattern.
    pub buttons: Buttons,
    /// Which simulation tick. The server will later discard anything too old.
    pub tick: u64,
}

impl Intent {
    pub fn movement(&self) -> Vec2 {
        Vec2::new(self.move_x, self.move_y)
    }

    /// Look direction as a unit vector. `yaw = 0, pitch = 0` yields −Z.
    pub fn look_dir(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    pub fn pressed(&self, button: Buttons) -> bool {
        self.buttons.contains(button)
    }
}

/// The buttons as a bit pattern — a `u32` instead of a `HashSet`, so that an `Intent` has a
/// fixed size and fits over a wire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Buttons(pub u32);

impl Buttons {
    pub const NONE: Buttons = Buttons(0);
    pub const JUMP: Buttons = Buttons(1 << 0);
    /// Hook left / right — two **independently** steerable hooks (`F-001`).
    pub const HOOK_LEFT: Buttons = Buttons(1 << 1);
    pub const HOOK_RIGHT: Buttons = Buttons(1 << 2);
    /// Reel the rope in (`F-005`) — costs gas.
    pub const REEL_IN: Buttons = Buttons(1 << 3);
    /// Gas boost. Spending gas is loud — the Bellower reacts to it (bible 4).
    pub const BOOST: Buttons = Buttons(1 << 4);
    pub const SLASH_LEFT: Buttons = Buttons(1 << 5);
    pub const SLASH_RIGHT: Buttons = Buttons(1 << 6);
    pub const DODGE: Buttons = Buttons(1 << 7);
    pub const MARK: Buttons = Buttons(1 << 8);

    pub fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    pub fn set(&mut self, other: Buttons, pressed: bool) {
        if pressed {
            self.0 |= other.0;
        } else {
            self.0 &= !other.0;
        }
    }

    /// Which buttons are pressed in `self` that were not yet pressed in `prev`. The
    /// difference between "is holding" and "has just pressed" is the difference between
    /// autofire and a single shot.
    pub fn just_pressed(self, prev: Buttons) -> Buttons {
        Buttons(self.0 & !prev.0)
    }
}

/// An absolute look angle dictated from outside (`look 0 -10` in a script).
///
/// **Taken out** on read, not copied: an override applies once and then hands the wheel back
/// to the mouse. Without that, a script could nail the view down by accident, and nobody
/// would see why the camera stopped moving.
#[derive(Resource, Debug, Default)]
pub struct LookOverride(pub Option<(f32, f32)>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_press_and_release() {
        let mut t = Buttons::NONE;
        assert!(!t.contains(Buttons::BOOST));
        t.set(Buttons::BOOST, true);
        t.set(Buttons::HOOK_LEFT, true);
        assert!(t.contains(Buttons::BOOST));
        assert!(t.contains(Buttons::HOOK_LEFT));
        assert!(!t.contains(Buttons::HOOK_RIGHT));
        t.set(Buttons::BOOST, false);
        assert!(!t.contains(Buttons::BOOST));
        assert!(t.contains(Buttons::HOOK_LEFT));
    }

    #[test]
    fn the_empty_button_set_is_never_pressed() {
        // Otherwise `contains(NONE)` would always be true and every query for "nothing
        // pressed" would quietly fire every frame.
        assert!(!Buttons::NONE.contains(Buttons::NONE));
        assert!(!Buttons(0xffff_ffff).contains(Buttons::NONE));
    }

    #[test]
    fn just_pressed_reports_only_the_transition() {
        let prev = Buttons::BOOST;
        let current = Buttons(Buttons::BOOST.0 | Buttons::JUMP.0);
        assert!(current.just_pressed(prev).contains(Buttons::JUMP));
        assert!(!current.just_pressed(prev).contains(Buttons::BOOST));
    }

    #[test]
    fn look_zero_points_at_minus_z() {
        // The axis contract from docs/conventions.md. If it falls, every model stands the
        // wrong way round and nobody knows why.
        let i = Intent::default();
        let b = i.look_dir();
        assert!((b - Vec3::NEG_Z).length() < 1e-6, "look_dir was {b:?}");
    }

    #[test]
    fn look_is_always_a_unit_vector() {
        for yaw in [-3.0_f32, -1.0, 0.0, 0.7, 2.9] {
            for pitch in [-1.5_f32, -0.3, 0.0, 0.3, 1.5] {
                let i = Intent { yaw, pitch, ..default() };
                let l = i.look_dir().length();
                assert!((l - 1.0).abs() < 1e-5, "yaw {yaw} pitch {pitch} gave length {l}");
            }
        }
    }
}
