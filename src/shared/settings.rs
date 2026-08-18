//! The options a **person** sets — not the numbers a game is balanced with.
//!
//! > *„zudem fehlen settings."* — the user, 2026-08-13 (`docs/NEXT.md` §1D, req 6).
//!
//! ## Why this is not a rule-2 violation, and where the numbers actually come from
//!
//! §6 rule 2 says a game value lives in RON and never in Rust, and that no value may be
//! defaulted. Both hold here: **every field below is seeded out of `game.ron`** by
//! [`PlayerSettings::from_world`], there is **no `Default` impl**, and no new RON key was
//! invented for any of it. `mouse_deg_per_px`, `fov_deg`, `pitch_limit_deg` and
//! `vector.aim_spread_deg` are exactly the four values the game already shipped with; what is
//! new is that the person at the keyboard may move them afterwards.
//!
//! The **windows** the sliders run in ([`MOUSE_MIN_DEG_PER_PX`] and friends) are deliberately
//! *not* in `game.ron`, for the same reason `net::local`'s `PIXELS_PER_NOTCH` is not: they are
//! properties of a device and of a control, not of the game, and a tuning file that carries a
//! slider's end stop invites somebody to balance the game with it. `aim_spread_deg` is the one
//! exception and it proves the rule — its window *is* a game value
//! (`game.ron: vector.aim_spread_min_deg/-max_deg/-step_deg`), because the wheel could already
//! reach it before this file existed, so the settings screen reads that window rather than
//! carrying a second one.
//!
//! ## A `Resource`, and rule 3 is not being bent
//!
//! `docs/multiplayer.md` rule 3 forbids **player** state in a resource, because there are many
//! players and a resource holds one of anything. These are not that. They are the state of
//! *this machine's* input and display — the same class as `net::local::MouseSinceTick` ("the
//! state of a device"), `net::local::Look`, and `menu::Screen`. A second player on a second
//! machine has his own, and **nothing here ever travels over a wire**: the one value the
//! simulation is allowed to see, `aim_spread_deg`, reaches it through `Intent::aim_spread_deg`
//! and through nothing else, exactly as it did when only the wheel could move it.
//!
//! ## One field, one writer — with two input devices
//!
//! `aim_spread_deg` is written by `net::local::read_input` (the mouse wheel, `F-023`) **and**
//! by `menu::settings` (the slider). That is one writer in the sense the rule means — *the
//! local player changing his own setting* — reached by two devices, the way `Buttons::DODGE` is
//! reached by `C` and by a double-tapped `Space`. What must never happen, and does not, is a
//! **second copy**: `net::local::Spread` used to hold the live angle in a `Local` and the
//! settings screen would have been unable to see it. The accumulator now lives in this one
//! field and [`step_spread`] is the arithmetic both of them go through.

use bevy::prelude::*;

use crate::data::GameData;

/// What the person at this machine has set. Seeded from `game.ron`, then his.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PlayerSettings {
    /// Degrees of yaw per pixel of mouse motion. `game.ron: camera.mouse_deg_per_px`.
    pub mouse_deg_per_px: f32,
    /// Whether pushing the mouse forward looks **down**. Off out of the file — there is no key
    /// for it in `game.ron` and there must not be one: nobody balances a game with it.
    pub invert_y: bool,
    /// Vertical field of view. `game.ron: camera.fov_deg`, and read as vertical because that is
    /// what `PerspectiveProjection.fov` is (`docs/QUESTIONS.md` Q-021).
    pub fov_deg: f32,
    /// **The live aim spread**, `F-023`. The wheel writes it, the settings screen writes it,
    /// `Intent::aim_spread_deg` carries it. Half-angle (`vector::aim`).
    pub aim_spread_deg: f32,
    /// How far up and down the local mouse path may look. Clamped to the file's value at all
    /// times — `render::camera` clamps the *incoming intent* against `game.ron` as well, and
    /// that one is a safety bound against a network peer, not a preference.
    pub pitch_limit_deg: f32,
}

/// The slowest a mouse may be set to. Below this a 180° turn takes more desk than anybody has.
pub const MOUSE_MIN_DEG_PER_PX: f32 = 0.01;
/// The fastest. Above this one pixel of desk noise is a visible flick.
pub const MOUSE_MAX_DEG_PER_PX: f32 = 0.60;
/// One click of the slider. 0.01 is finer than the 0.08 the file ships with is coarse — a
/// player who wants "a little slower" has to be able to ask for a little.
pub const MOUSE_STEP_DEG_PER_PX: f32 = 0.01;

/// The narrowest field of view. `scale.ron: camera.min_ground_fov_deg` is 55 and this is the
/// same number by intent, not by accident: below it ground combat stops showing the titan.
pub const FOV_MIN_DEG: f32 = 55.0;
/// The widest. `game.ron: camera.fov_max_speed_deg` is 90 and `F-017` will drive the image
/// there at full speed; a player who wants that view standing still may have it, plus the
/// headroom the wide-screen crowd asks for.
pub const FOV_MAX_DEG: f32 = 110.0;
/// One click of the FOV slider.
pub const FOV_STEP_DEG: f32 = 5.0;

impl FromWorld for PlayerSettings {
    /// **The only way this type comes into being.** There is no `Default`, so no number here
    /// can be invented — a missing key crashes in `data::` at startup, where it belongs.
    fn from_world(world: &mut World) -> Self {
        let data = world.resource::<GameData>();
        let camera = &data.game.camera;
        Self {
            mouse_deg_per_px: camera.mouse_deg_per_px,
            // Not in the file, and deliberately: it is a preference with no game meaning, and
            // "off" is what every game means by not inverted.
            invert_y: false,
            fov_deg: camera.fov_deg,
            aim_spread_deg: data.game.vector.aim_spread_deg,
            pitch_limit_deg: camera.pitch_limit_deg,
        }
    }
}

impl PlayerSettings {
    /// `+1` or `-1` for the mouse, clamped into the slider's window.
    pub fn nudge_mouse(&mut self, steps: f32) {
        self.mouse_deg_per_px = clamp_step(
            self.mouse_deg_per_px,
            steps,
            MOUSE_STEP_DEG_PER_PX,
            MOUSE_MIN_DEG_PER_PX,
            MOUSE_MAX_DEG_PER_PX,
        );
    }

    /// `+1` or `-1` for the field of view.
    pub fn nudge_fov(&mut self, steps: f32) {
        self.fov_deg = clamp_step(self.fov_deg, steps, FOV_STEP_DEG, FOV_MIN_DEG, FOV_MAX_DEG);
    }

    /// `+1` or `-1` for the aim spread — **through the same arithmetic the wheel uses**, and
    /// with the window out of `game.ron` rather than a second one of this file's own.
    pub fn nudge_spread(&mut self, steps: f32, step_deg: f32, min_deg: f32, max_deg: f32) {
        self.aim_spread_deg =
            step_spread(self.aim_spread_deg, steps, step_deg, min_deg, max_deg);
    }

    /// Which way the mouse pitches. `+1` normal, `-1` inverted — a factor and not an `if`, so
    /// the input path stays one line.
    pub fn pitch_sign(&self) -> f32 {
        if self.invert_y {
            -1.0
        } else {
            1.0
        }
    }
}

/// One turn of the aim-spread wheel (or one click of its slider): **absolute** degrees out,
/// clamped into the window.
///
/// A free function and not a method, because two callers with two shapes go through it: the
/// wheel in `net::local::read_input` and the slider in `menu::settings`. Clamping and not
/// wrapping is what makes the value converge — two players who turned differently and then ran
/// into an end stop end on the same number, and a wrap would keep them apart forever
/// (`docs/multiplayer.md`; `net::local::Spread` carries the long version of the argument).
pub fn step_spread(current_deg: f32, steps: f32, step_deg: f32, min_deg: f32, max_deg: f32) -> f32 {
    (current_deg + steps * step_deg).clamp(min_deg, max_deg)
}

/// The same arithmetic for a slider whose window is a UI constant rather than a game value.
fn clamp_step(current: f32, steps: f32, step: f32, min: f32, max: f32) -> f32 {
    (current + steps * step).clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A setting that leaves its window is a setting that breaks the game it belongs to — a
    /// mouse at 0 does not turn at all, and an FOV at 5 is a telescope.
    #[test]
    fn a_slider_cannot_be_pushed_out_of_its_window() {
        let mut s = PlayerSettings {
            mouse_deg_per_px: 0.08,
            invert_y: false,
            fov_deg: 60.0,
            aim_spread_deg: 28.0,
            pitch_limit_deg: 89.0,
        };
        for _ in 0..500 {
            s.nudge_mouse(-1.0);
            s.nudge_fov(-1.0);
        }
        assert_eq!(s.mouse_deg_per_px, MOUSE_MIN_DEG_PER_PX);
        assert_eq!(s.fov_deg, FOV_MIN_DEG);
        for _ in 0..500 {
            s.nudge_mouse(1.0);
            s.nudge_fov(1.0);
        }
        assert_eq!(s.mouse_deg_per_px, MOUSE_MAX_DEG_PER_PX);
        assert_eq!(s.fov_deg, FOV_MAX_DEG);
    }

    /// The wheel and the slider are the same arithmetic — one field, one behaviour.
    #[test]
    fn the_slider_and_the_wheel_agree_on_one_notch() {
        let (start, step, min, max) = (28.0, 2.0, 4.0, 60.0);
        let mut s = PlayerSettings {
            mouse_deg_per_px: 0.08,
            invert_y: false,
            fov_deg: 60.0,
            aim_spread_deg: start,
            pitch_limit_deg: 89.0,
        };
        s.nudge_spread(1.0, step, min, max);
        assert_eq!(s.aim_spread_deg, step_spread(start, 1.0, step, min, max));
        assert_eq!(s.aim_spread_deg, start + step);
    }
}
