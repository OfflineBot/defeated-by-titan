//! The options a **person** sets — not the numbers a game is balanced with.
//!
//! > *„zudem fehlen settings."* — the user, 2026-08-13 (`docs/NEXT.md` §1D, req 6).
//!
//! ## Why this is not a rule-2 violation, and where the numbers actually come from
//!
//! §6 rule 2 says a game value lives in RON and never in Rust, and that no value may be
//! defaulted. Both hold here: **every field below is seeded out of `game.ron`** by
//! [`PlayerSettings::from_world`], there is **no `Default` impl**, and no new RON key was
//! invented for any of it. `mouse_deg_per_px`, `fov_deg` and `pitch_limit_deg` are exactly the
//! three values the game already shipped with; what is new is that the person at the keyboard
//! may move them afterwards.
//!
//! ⚠️ **Two fields are seeded at zero and not out of the file, and that is not a defaulted game
//! value.** [`PlayerSettings::assist_catch_pct`] and [`PlayerSettings::assist_strength_pct`]
//! (`F-016`, added 2026-08-19 because the user asked for the knobs themselves: *„und
//! seinstellen können wie weit ca es sein sollte und wie aggressive"*) have **0 % defined as the
//! absence of the feature** — `F-016`'s own words, *"0 % = pure free aim"* — so seeding them at
//! zero is the statement that nothing about aiming changed, not a number somebody invented. The
//! same footing as `invert_y`, and the reason `game.ron` got no new key for either.
//!
//! The **windows** the sliders run in ([`MOUSE_MIN_DEG_PER_PX`] and friends) are deliberately
//! *not* in `game.ron`, for the same reason `net::local`'s `PIXELS_PER_NOTCH` is not: they are
//! properties of a device and of a control, not of the game, and a tuning file that carries a
//! slider's end stop invites somebody to balance the game with it. `aim_spread_deg` used to be
//! the exception that proved the rule — its window *was* a game value, because the mouse wheel
//! could reach it before this file existed. That field is gone with `F-023` (below), and every
//! window here is now a UI constant.
//!
//! ## A `Resource`, and rule 3 is not being bent
//!
//! `docs/multiplayer.md` rule 3 forbids **player** state in a resource, because there are many
//! players and a resource holds one of anything. These are not that. They are the state of
//! *this machine's* input and display — the same class as `net::local::MouseSinceTick` ("the
//! state of a device"), `net::local::Look`, and `menu::Screen`. A second player on a second
//! machine has his own, and **nothing here travels over a wire at all** since `aim_spread_deg`
//! was retired: the assist knobs bend only the local player's aim (`vector::aim` filters on
//! `Has<LocalPlayer>`), which is what keeps rule 3 satisfied.
//!
//! ## The field that had two input devices is gone
//!
//! Between 2026-08-13 and 2026-08-23 `aim_spread_deg` was written by `net::local::read_input`
//! (the mouse wheel, `F-023`) **and** by `menu::settings` (the slider) — one field reached by
//! two devices, with `step_spread` as the arithmetic both went through. The user retired the
//! fan on 2026-08-23 (*„dann das auseinander mit q und e kann weg. einfach da wo ich hinschau
//! (also fadenkreuz) geht das seil hin."*): both ropes fly at the crosshair, so there is no
//! angle to allow, and the field, the wheel, the row and `step_spread` went with it
//! (`docs/QUESTIONS.md` Q-048).

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
    /// How far up and down the local mouse path may look. Clamped to the file's value at all
    /// times — `render::camera` clamps the *incoming intent* against `game.ron` as well, and
    /// that one is a safety bound against a network peer, not a preference.
    pub pitch_limit_deg: f32,
    /// **How far from where you point the hook may still catch**, 0..100 % (`F-016`).
    ///
    /// > *„und seinstellen können wie weit ca es sein sollte"* — the user, 2026-08-18.
    ///
    /// `F-016` specifies exactly this shape: *"a 0–100 % stepless snap catch angle"*, where
    /// **0 % is today's pure free aim** — the ray goes where the crosshair goes and nothing
    /// else is considered. 100 % is [`ASSIST_CATCH_MAX_DEG`] off the look direction, which is
    /// the widest a catch can be without reaching behind the shoulder.
    ///
    /// **Named for what it means to the player and not for today's raycast**, on purpose: it
    /// is *how far the game may look on your behalf*, and `F-025`'s weighted scoring reads it
    /// as the radius of the candidate set — the gate that decides which anchors are scored at
    /// all, before the 45 % angle-deviation term ranks them. Nothing in the name has to change
    /// when the raycast becomes a candidate sweep.
    pub assist_catch_pct: f32,
    /// **How hard it pulls once it has found one**, 0..100 % (`F-016` / `F-024`).
    ///
    /// > *„und wie aggressive (damit ich testen kann was am besten wäre mach debug
    /// > einstellungen dafür)"* — the user, 2026-08-18.
    ///
    /// The second axis, and it is genuinely a different question from [`Self::assist_catch_pct`]:
    /// *whether* a better anchor is a candidate, against *how much of the way* the game is
    /// allowed to move your aim towards it. 0 % is again exactly free aim — a candidate may be
    /// found and highlighted and the rope still flies where you pointed, which is `F-024`'s
    /// **FREI** mode. 100 % is its full-snap mode; in between is **ASSISTIERT**.
    ///
    /// `F-025` reads it as **how much better a candidate has to be than the point you are
    /// really aiming at** before it is allowed to take the arm:
    /// `vector::aim::required_margin` is `game.ron: vector.assist_margin_full * (1 - pct/100)`.
    /// One number, three modes, no fourth enum nobody can dial — 0 % never gets there at all
    /// (FREI), 100 % needs no margin (SNAP), between is ASSISTIERT.
    ///
    /// **Wired since 2026-08-19** (`docs/FINDINGS.md` FIND-104): the consumer is
    /// `vector::aim::aim`, and it is the only one.
    pub assist_strength_pct: f32,
    /// **How much of `F-017`'s speed effect this machine wants**, 0..100 %.
    ///
    /// The backlog row asks for it in so many words: *„abschaltbar fuer Motion Sickness"*. A
    /// widening lens is the single most common trigger for simulator sickness, and a player who
    /// cannot switch it off cannot play the game at all — so this is an accessibility control,
    /// not a taste slider, and it lives beside the other two 0..100 knobs.
    ///
    /// **0 % is exactly the behaviour that shipped before `F-017`**: the field of view is
    /// [`Self::fov_deg`] at every speed, bit for bit, because `render::speed_fov` returns the
    /// base without touching the arithmetic at all. 100 % is the full curve to
    /// `game.ron: camera.fov_max_speed_deg`. In between it is a fraction of the widening, which
    /// is the useful shape — most people who cannot take the full effect can take a third of it.
    ///
    /// It seeds at 100: the effect is the feature, and a feature that ships off is a feature
    /// nobody sees.
    pub speed_fov_pct: f32,
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

/// Both assist knobs are percentages of themselves — a slider that runs 0..100 needs no window
/// out of a file, and `F-016` writes the range down itself: *"0–100 %, stepless"*.
pub const ASSIST_MIN_PCT: f32 = 0.0;
/// The other end. 100 % is "as much help as this game will ever give".
pub const ASSIST_MAX_PCT: f32 = 100.0;
/// One click. 5 % is twenty distinguishable settings across the whole range — fine enough that
/// he can find a feel, coarse enough that he can **tell us the number back** (*„damit ich
/// testen kann was am besten wäre"*), which a 0.01 slider makes impossible.
pub const ASSIST_STEP_PCT: f32 = 5.0;

/// What [`PlayerSettings::assist_catch_pct`] = 100 % means in degrees off the look direction.
///
/// **A UI end stop and not a game value**, the same class as [`MOUSE_MAX_DEG_PER_PX`]: it is a
/// property of the control. `F-025`'s candidate sweep, built on 2026-08-19, reads the
/// *percentage* through [`PlayerSettings::assist_catch_deg`] and never this constant. 20° is a little under half the horizontal half-frustum at
/// the file's 60° vertical FOV — past that the "assist" is picking targets the player is not
/// looking at, which is the failure `F-024` names (*"waehlt nie einen Punkt hinter dem
/// Spieler"*).
pub const ASSIST_CATCH_MAX_DEG: f32 = 20.0;

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
            pitch_limit_deg: camera.pitch_limit_deg,
            // **Zero is not a defaulted game value, it is the absence of a feature.** `F-016`
            // defines 0 % as "exactly today's pure free aim", so seeding both at 0 is the
            // statement that the game behaves precisely as it did before this row existed —
            // the same argument `invert_y: false` makes above, and the reason no key was
            // invented in `game.ron` for either of them (§6 rule 2).
            assist_catch_pct: 0.0,
            assist_strength_pct: 0.0,
            // `F-017` ships **on**, and that is the one of these three that is not zero. The
            // two above are 0 because 0 is the absence of an aim assist and the assist has to
            // be asked for; this one is 100 because the speed effect IS the feature, and a
            // feature that ships off is a feature nobody sees. Off is one slider away, and
            // that is what `abschaltbar` asks for.
            speed_fov_pct: 100.0,
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

    /// `+1` or `-1` for *how far the assist looks*.
    pub fn nudge_assist_catch(&mut self, steps: f32) {
        self.assist_catch_pct = clamp_step(
            self.assist_catch_pct,
            steps,
            ASSIST_STEP_PCT,
            ASSIST_MIN_PCT,
            ASSIST_MAX_PCT,
        );
    }

    /// `+1` or `-1` for *how hard it pulls*.
    pub fn nudge_assist_strength(&mut self, steps: f32) {
        self.assist_strength_pct = clamp_step(
            self.assist_strength_pct,
            steps,
            ASSIST_STEP_PCT,
            ASSIST_MIN_PCT,
            ASSIST_MAX_PCT,
        );
    }

    /// The catch percentage as the angle it means, in **degrees** off the look direction.
    ///
    /// The one place the percentage becomes a geometry, so that `F-024`/`F-025` cannot end up
    /// with a second reading of the same slider — and so that a HUD line and a candidate sweep
    /// always quote the same number.
    pub fn assist_catch_deg(&self) -> f32 {
        self.assist_catch_pct / 100.0 * ASSIST_CATCH_MAX_DEG
    }

    /// **Is any assist on at all?** `false` is the guarantee `F-002` demands — *"this layer
    /// stays ALWAYS active and is never replaceable by the snap system"* — read from the
    /// player's own two knobs: with either one at zero there is nothing to snap to or nothing
    /// to snap with, and the free ray is the whole answer.
    pub fn assist_is_on(&self) -> bool {
        self.assist_catch_pct > 0.0 && self.assist_strength_pct > 0.0
    }

    /// **Is there a reach to draw?** — [`assist_is_on`](Self::assist_is_on)'s first half, on
    /// its own, and it exists because those two halves answer two different questions.
    ///
    /// ⚠️ **This is the DRAWING predicate. [`assist_is_on`](Self::assist_is_on) is the
    /// SEARCHING one, and only that one may gate a probe ray** — it is `F-002`'s guarantee and
    /// `tests/vector_hooks.rs::f016_at_zero_percent_the_aim_is_bit_for_bit_the_one_the_game_had_before`
    /// is what holds it. `vector::aim` must never call this function.
    ///
    /// They deliberately differ, and `Q-042` is why: both knobs ship at 0, so a HUD element
    /// gated on `assist_is_on` was invisible in the exact moment it exists for — the player
    /// who opens the settings screen and turns *Aim assist reach* up sees nothing happen, at
    /// any value, because a second and differently-named row is the master switch. The reach
    /// **is** a number with a picture (`hud::catch_band`: from where to where the search would
    /// look), the strength is whether a find is taken, and the picture belongs to the row that
    /// owns it. What the picture may not do is *claim a search is running*, so the band says
    /// that in its colour instead — [`hud::catch_band::IDLE`](crate::hud::catch_band::IDLE).
    pub fn assist_has_reach(&self) -> bool {
        self.assist_catch_pct > 0.0
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
            pitch_limit_deg: 89.0,
            assist_catch_pct: 0.0,
            assist_strength_pct: 0.0,
            speed_fov_pct: 100.0,
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

    /// `F-016` — the two knobs the user asked for, and the one property that makes them safe
    /// to ship before `F-024`/`F-025` exist: **at zero the game aims exactly as it does
    /// today.**
    ///
    /// > *„es sollte best match sein. und seinstellen können wie weit ca es sein sollte und wie
    /// > aggressive (damit ich testen kann was am besten wäre mach debug einstellungen dafür)"*
    /// > — the user, 2026-08-18.
    ///
    /// `F-002` is the rule this protects: *"this layer stays ALWAYS active and is never
    /// replaceable by the snap system"*. Both knobs start at 0 and 0 is off, so the free ray
    /// keeps the whole answer until he moves one.
    #[test]
    fn f016_the_two_assist_knobs_start_off_and_stay_inside_nought_to_a_hundred() {
        let mut s = PlayerSettings {
            mouse_deg_per_px: 0.08,
            invert_y: false,
            fov_deg: 60.0,
            pitch_limit_deg: 89.0,
            assist_catch_pct: 0.0,
            assist_strength_pct: 0.0,
            speed_fov_pct: 100.0,
        };
        // Off is off, and it is off in the geometry too — not "almost zero degrees".
        assert!(!s.assist_is_on());
        assert_eq!(s.assist_catch_deg(), 0.0);

        // One click of each is a change he can feel and a number he can read back to us.
        s.nudge_assist_catch(1.0);
        s.nudge_assist_strength(1.0);
        assert_eq!(s.assist_catch_pct, ASSIST_STEP_PCT);
        assert_eq!(s.assist_strength_pct, ASSIST_STEP_PCT);
        assert!(s.assist_is_on());

        // Neither can be pushed out of `F-016`'s stated 0..100 %.
        for _ in 0..200 {
            s.nudge_assist_catch(-1.0);
            s.nudge_assist_strength(-1.0);
        }
        assert_eq!(s.assist_catch_pct, ASSIST_MIN_PCT);
        assert_eq!(s.assist_strength_pct, ASSIST_MIN_PCT);
        assert!(!s.assist_is_on(), "0 % has to be exactly free aim again");
        for _ in 0..200 {
            s.nudge_assist_catch(1.0);
            s.nudge_assist_strength(1.0);
        }
        assert_eq!(s.assist_catch_pct, ASSIST_MAX_PCT);
        assert_eq!(s.assist_strength_pct, ASSIST_MAX_PCT);
        // And the percentage means one angle and only one — `F-025` reads this, not a second
        // reading of its own.
        assert_eq!(s.assist_catch_deg(), ASSIST_CATCH_MAX_DEG);
        // Half the slider is half the angle: stepless and linear, so a number he reports back
        // ("40 felt right") maps onto exactly one geometry.
        s.assist_catch_pct = 50.0;
        assert!((s.assist_catch_deg() - ASSIST_CATCH_MAX_DEG / 2.0).abs() < 1e-6);
        // Either knob at zero is free aim: a catch radius with no pull, or a pull with no
        // radius, are both "the rope goes where you point".
        s.assist_strength_pct = 0.0;
        assert!(!s.assist_is_on());
    }
}
