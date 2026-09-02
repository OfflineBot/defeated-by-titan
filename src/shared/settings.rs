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
    /// Whether a rope trigger is a hold or a toggle. [`HookFire::Toggle`] since 2026-09-01 —
    /// the user's words, and the settings row is the „oder in einstellungen einstellbar" half.
    pub hook_fire: HookFire,
    /// The rebindable keys (`F-172`, partial — see [`KeyBinds`] for what is honestly not in it).
    pub binds: KeyBinds,
    /// Crosshair size, 50..200 % of the base the `game.ron: hud.crosshair` numbers draw.
    /// 100 % is the „mittel bis klein" he asked for on 2026-09-01.
    pub crosshair_size_pct: f32,
    /// Index into [`CROSSHAIR_COLOURS`] — the colour of the crosshair's **Free** state.
    /// `Anchor`/`Cortex` keep cyan/amber: those two states are signals and a signal colour
    /// is not a preference (`docs/conventions.md` §3).
    pub crosshair_colour: usize,
    /// Which settings page the plate shows. **View state, not a preference** — it is here
    /// because the plate rebuilds off this one resource, and it is never persisted.
    pub page: SettingsPage,
    /// The bind row waiting for a key, or `None`. View state like [`Self::page`]; leaving the
    /// settings screen clears it (`menu::settings::settings_buttons`). Never persisted.
    pub rebinding: Option<BindAction>,
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
        let seeded = Self {
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
            // Toggle because he asked for toggle in so many words; Hold is one row away.
            hook_fire: HookFire::Toggle,
            binds: KeyBinds::DEFAULT,
            // 100 % IS the requested size: the base numbers in `game.ron: hud.crosshair` were
            // written as the „mittel bis klein" of 2026-09-01, so the slider starts at them.
            crosshair_size_pct: 100.0,
            crosshair_colour: 0,
            page: SettingsPage::Main,
            rebinding: None,
        };
        // **What he set last time beats what the files seed** — `saves/settings.ron`, written
        // by `menu::settings` on every change. Absent file: the seeds above. Broken file: the
        // seeds above, with a warning — a preference is cheap enough to lose, unlike a career
        // (`save::file` keeps broken profiles; this deliberately does not).
        load_settings(seeded)
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
            hook_fire: HookFire::Toggle,
            binds: KeyBinds::DEFAULT,
            crosshair_size_pct: 100.0,
            crosshair_colour: 0,
            page: SettingsPage::Main,
            rebinding: None,
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
            hook_fire: HookFire::Toggle,
            binds: KeyBinds::DEFAULT,
            crosshair_size_pct: 100.0,
            crosshair_colour: 0,
            page: SettingsPage::Main,
            rebinding: None,
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

// ---------------------------------------------------------------------------
// How a rope trigger behaves, the rebindable keys, and the crosshair's own two
// knobs (2026-09-01) — plus the file all of it survives a restart in.
// ---------------------------------------------------------------------------

/// What `Q`/`E` mean while an arm is out (user, 2026-09-01: *„mach dass q und e toggle sind
/// und nicht hold (oder in einstellungen einstellbar)"*).
///
/// [`HookFire::Toggle`] is the default because he asked for it in those words; Hold stays a
/// settings row away. Under Toggle a **tap** fires and the next tap releases — and a press
/// held longer than `net::local::HOOK_TAP_MAX_TICKS` still releases on key-up, so a held key
/// behaves the way every evidence script and every Hold-trained hand expects. The latch
/// itself lives in `net::local::HookLatch`, on the keyboard side of the `Intent`:
/// `vector::hook` keeps reading a plain held bit and never learns which mode produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookFire {
    /// The rope holds while the key is down — the behaviour before 2026-09-01.
    Hold,
    /// A tap fires, the next tap releases; a long press still releases on key-up.
    Toggle,
}

impl HookFire {
    /// The word the settings row and the settings file spell it with.
    pub fn word(self) -> &'static str {
        match self {
            HookFire::Hold => "hold",
            HookFire::Toggle => "toggle",
        }
    }

    pub fn from_word(word: &str) -> Option<HookFire> {
        match word {
            "hold" => Some(HookFire::Hold),
            "toggle" => Some(HookFire::Toggle),
            _ => None,
        }
    }
}

/// The rebindable keyboard bindings (`F-172` — *„es wird zeit einstellungen für keybinds zu
/// adden"*, user 2026-09-01).
///
/// **This is the model; `net::local::read_input` is the one consumer.** The mouse buttons
/// (both blades) and the movement axes (`WASD`) are deliberately not in here yet — the blades
/// because a mouse button is not a `KeyCode` and the capture below is a keyboard capture, the
/// axes because rebinding them touches the flip detector's two sides (`F-009`) and nobody has
/// asked. `F-172`'s full claim ("Keine Aktion ist fest verdrahtet", presets) is therefore NOT
/// met by this struct and is not claimed anywhere as met.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinds {
    pub hook_left: KeyCode,
    pub hook_right: KeyCode,
    pub dodge: KeyCode,
    pub mark: KeyCode,
    /// The keyboard route to the left blade — `F` next to the left mouse button, and the only
    /// way a script reaches `SLASH_LEFT` (`debug::script::parse_key` cannot press a mouse).
    pub slash_left: KeyCode,
    pub boost: KeyCode,
    pub reel_in: KeyCode,
    pub jump: KeyCode,
}

impl KeyBinds {
    /// The scheme the user settled on 2026-08-10, unchanged — rebinding starts from here.
    pub const DEFAULT: KeyBinds = KeyBinds {
        hook_left: KeyCode::KeyQ,
        hook_right: KeyCode::KeyE,
        dodge: KeyCode::KeyC,
        mark: KeyCode::Tab,
        slash_left: KeyCode::KeyF,
        boost: KeyCode::ShiftLeft,
        reel_in: KeyCode::ControlLeft,
        jump: KeyCode::Space,
    };

    pub fn get(&self, action: BindAction) -> KeyCode {
        match action {
            BindAction::HookLeft => self.hook_left,
            BindAction::HookRight => self.hook_right,
            BindAction::Dodge => self.dodge,
            BindAction::Mark => self.mark,
            BindAction::SlashLeft => self.slash_left,
            BindAction::Boost => self.boost,
            BindAction::ReelIn => self.reel_in,
            BindAction::Jump => self.jump,
        }
    }

    fn set_raw(&mut self, action: BindAction, key: KeyCode) {
        match action {
            BindAction::HookLeft => self.hook_left = key,
            BindAction::HookRight => self.hook_right = key,
            BindAction::Dodge => self.dodge = key,
            BindAction::Mark => self.mark = key,
            BindAction::SlashLeft => self.slash_left = key,
            BindAction::Boost => self.boost = key,
            BindAction::ReelIn => self.reel_in = key,
            BindAction::Jump => self.jump = key,
        }
    }

    /// Binds `key` to `action` — and **a conflict swaps instead of refusing**: the action that
    /// held `key` until now takes `action`'s old key. That is the cheapest rule that keeps the
    /// invariant a keyboard needs — *no two actions on one key, no action without a key* — and
    /// it is `F-172`'s „Konflikterkennung" in its smallest honest form: the conflict is
    /// detected and resolved, never silently created.
    pub fn set(&mut self, action: BindAction, key: KeyCode) {
        let old = self.get(action);
        for other in BindAction::ALL {
            if other != action && self.get(other) == key {
                self.set_raw(other, old);
            }
        }
        self.set_raw(action, key);
    }
}

/// One rebindable action — a row on the keybinds page, a field of [`KeyBinds`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindAction {
    HookLeft,
    HookRight,
    Dodge,
    Mark,
    SlashLeft,
    Boost,
    ReelIn,
    Jump,
}

impl BindAction {
    pub const ALL: [BindAction; 8] = [
        BindAction::HookLeft,
        BindAction::HookRight,
        BindAction::Dodge,
        BindAction::Mark,
        BindAction::SlashLeft,
        BindAction::Boost,
        BindAction::ReelIn,
        BindAction::Jump,
    ];

    /// The label on the settings row — what the action does, not what key it is.
    pub fn label(self) -> &'static str {
        match self {
            BindAction::HookLeft => "Hook left",
            BindAction::HookRight => "Hook right",
            BindAction::Dodge => "Dodge",
            BindAction::Mark => "Mark titan",
            BindAction::SlashLeft => "Slash (keyboard)",
            BindAction::Boost => "Boost",
            BindAction::ReelIn => "Reel in",
            BindAction::Jump => "Jump",
        }
    }

    /// The field name in `settings.ron` — stable, English, never the display label.
    fn file_key(self) -> &'static str {
        match self {
            BindAction::HookLeft => "hook_left",
            BindAction::HookRight => "hook_right",
            BindAction::Dodge => "dodge",
            BindAction::Mark => "mark",
            BindAction::SlashLeft => "slash_left",
            BindAction::Boost => "boost",
            BindAction::ReelIn => "reel_in",
            BindAction::Jump => "jump",
        }
    }
}

/// Every key the capture accepts, with the name it is written to `settings.ron` under.
///
/// A closed table and not `format!("{key:?}")`, because the file has to be **parsed back**:
/// what is not in this table cannot be stored, so it cannot be captured either. The names are
/// the `KeyCode` variant names, so a hand-edited file and a debug log spell keys identically.
pub const REBINDABLE_KEYS: &[(&str, KeyCode)] = &[
    ("KeyA", KeyCode::KeyA), ("KeyB", KeyCode::KeyB), ("KeyC", KeyCode::KeyC),
    ("KeyD", KeyCode::KeyD), ("KeyE", KeyCode::KeyE), ("KeyF", KeyCode::KeyF),
    ("KeyG", KeyCode::KeyG), ("KeyH", KeyCode::KeyH), ("KeyI", KeyCode::KeyI),
    ("KeyJ", KeyCode::KeyJ), ("KeyK", KeyCode::KeyK), ("KeyL", KeyCode::KeyL),
    ("KeyM", KeyCode::KeyM), ("KeyN", KeyCode::KeyN), ("KeyO", KeyCode::KeyO),
    ("KeyP", KeyCode::KeyP), ("KeyQ", KeyCode::KeyQ), ("KeyR", KeyCode::KeyR),
    ("KeyS", KeyCode::KeyS), ("KeyT", KeyCode::KeyT), ("KeyU", KeyCode::KeyU),
    ("KeyV", KeyCode::KeyV), ("KeyW", KeyCode::KeyW), ("KeyX", KeyCode::KeyX),
    ("KeyY", KeyCode::KeyY), ("KeyZ", KeyCode::KeyZ),
    ("Digit1", KeyCode::Digit1), ("Digit2", KeyCode::Digit2), ("Digit3", KeyCode::Digit3),
    ("Digit4", KeyCode::Digit4), ("Digit5", KeyCode::Digit5), ("Digit6", KeyCode::Digit6),
    ("Digit7", KeyCode::Digit7), ("Digit8", KeyCode::Digit8), ("Digit9", KeyCode::Digit9),
    ("Digit0", KeyCode::Digit0),
    ("Space", KeyCode::Space), ("Tab", KeyCode::Tab),
    ("ShiftLeft", KeyCode::ShiftLeft), ("ShiftRight", KeyCode::ShiftRight),
    ("ControlLeft", KeyCode::ControlLeft), ("ControlRight", KeyCode::ControlRight),
    ("AltLeft", KeyCode::AltLeft), ("AltRight", KeyCode::AltRight),
    ("ArrowUp", KeyCode::ArrowUp), ("ArrowDown", KeyCode::ArrowDown),
    ("ArrowLeft", KeyCode::ArrowLeft), ("ArrowRight", KeyCode::ArrowRight),
    ("Comma", KeyCode::Comma), ("Period", KeyCode::Period), ("Slash", KeyCode::Slash),
    ("Semicolon", KeyCode::Semicolon), ("Quote", KeyCode::Quote),
    ("Backquote", KeyCode::Backquote), ("Minus", KeyCode::Minus), ("Equal", KeyCode::Equal),
    ("BracketLeft", KeyCode::BracketLeft), ("BracketRight", KeyCode::BracketRight),
    ("Backslash", KeyCode::Backslash), ("Enter", KeyCode::Enter),
    ("Home", KeyCode::Home), ("End", KeyCode::End),
    ("PageUp", KeyCode::PageUp), ("PageDown", KeyCode::PageDown),
];

/// The file name of one key, or `None` for a key the capture must refuse.
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    REBINDABLE_KEYS.iter().find(|(_, k)| *k == key).map(|(name, _)| *name)
}

/// The inverse — how `settings.ron` comes back in.
pub fn key_from_name(name: &str) -> Option<KeyCode> {
    REBINDABLE_KEYS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

/// The short label a button shows — `KeyQ` is written `Q`, `Digit1` is `1`.
pub fn key_label(key: KeyCode) -> String {
    let name = key_name(key).unwrap_or("?");
    name.strip_prefix("Key").or_else(|| name.strip_prefix("Digit")).unwrap_or(name).to_string()
}

/// Which page of the settings screen is up. View state of this machine's one screen — it
/// lives here because `menu::spawn_menu` rebuilds the plate off exactly this resource, so a
/// page flip is a settings change like any other and needs no second rebuild mechanism.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Main,
    Keybinds,
    Crosshair,
}

/// The crosshair colours a player may pick (user, 2026-09-01: *„größe einstellbar und farbe
/// auch!"*). **None of the three signal colours** — cyan, amber and crimson keep their one
/// meaning each (`docs/conventions.md` §3), and the crosshair's `Anchor`/`Cortex` states keep
/// painting over this choice with exactly those two, because those states ARE the signals.
pub const CROSSHAIR_COLOURS: &[(&str, Color)] = &[
    ("white", Color::srgba(1.0, 1.0, 1.0, 0.9)),
    ("green", Color::srgba(0.35, 0.95, 0.45, 0.9)),
    ("magenta", Color::srgba(0.95, 0.35, 0.85, 0.9)),
    ("black", Color::srgba(0.0, 0.0, 0.0, 0.9)),
];

/// The smallest crosshair. Below half size the strokes are shorter than their own gap floor.
pub const CROSSHAIR_MIN_PCT: f32 = 50.0;
/// The biggest — twice the base, which is already „mittel bis groß" territory.
pub const CROSSHAIR_MAX_PCT: f32 = 200.0;
/// One click. Quarters are steps he can name back.
pub const CROSSHAIR_STEP_PCT: f32 = 25.0;

impl PlayerSettings {
    /// `+1` or `-1` for the crosshair size.
    pub fn nudge_crosshair_size(&mut self, steps: f32) {
        self.crosshair_size_pct = clamp_step(
            self.crosshair_size_pct,
            steps,
            CROSSHAIR_STEP_PCT,
            CROSSHAIR_MIN_PCT,
            CROSSHAIR_MAX_PCT,
        );
    }

    /// `+1` or `-1` through [`CROSSHAIR_COLOURS`], wrapping — four colours are a cycle, not a
    /// slider with end stops.
    pub fn cycle_crosshair_colour(&mut self, steps: i32) {
        let n = CROSSHAIR_COLOURS.len() as i32;
        self.crosshair_colour =
            (self.crosshair_colour as i32 + steps).rem_euclid(n) as usize;
    }

    /// The chosen colour, defensively: an index a hand-edited file pushed out of the table
    /// falls back to entry 0 instead of a panic in the draw path.
    pub fn crosshair_colour(&self) -> Color {
        CROSSHAIR_COLOURS
            .get(self.crosshair_colour)
            .unwrap_or(&CROSSHAIR_COLOURS[0])
            .1
    }

    /// The chosen colour's name, same fallback.
    pub fn crosshair_colour_name(&self) -> &'static str {
        CROSSHAIR_COLOURS
            .get(self.crosshair_colour)
            .unwrap_or(&CROSSHAIR_COLOURS[0])
            .0
    }
}

// ---------------------------------------------------------------------------
// `saves/settings.ron` — what he set survives quitting.
// ---------------------------------------------------------------------------
//
// **`menu` writes it, this file spells it** — the one writer of `PlayerSettings` stays the one
// writer of its file (`src/save/mod.rs` says in its own header why `save` must not do this).
// The serialisation and the path rule live HERE so that `FromWorld` above can read the file
// back without `shared` reaching up into `save` — which the domain order forbids.
//
// ⚠️ The path rule deliberately repeats `save::file::SaveDir::discover`'s three-step contract
// (env `DBT_SAVE_DIR` — off/empty is off; a test binary is off; else `<root>/saves/`), because
// `shared` cannot import it. Two spellings of one contract is exactly what rule 5's corollary
// warns about — recorded in `docs/FINDINGS.md` FIND-224 with the unification that would fix it
// (move the discovery into `shared`, let `save` read it).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The file's schema. Bumped when a field changes meaning; a mismatch reads as "no file".
const SETTINGS_SCHEMA: u32 = 1;

/// What goes to disk. A separate struct and not `PlayerSettings` itself, so that view state
/// (`page`, `rebinding`) can never leak into the file and a `KeyCode` is stored as the name
/// [`REBINDABLE_KEYS`] can parse back.
#[derive(Serialize, Deserialize)]
struct StoredSettings {
    schema: u32,
    mouse_deg_per_px: f32,
    invert_y: bool,
    fov_deg: f32,
    assist_catch_pct: f32,
    assist_strength_pct: f32,
    speed_fov_pct: f32,
    hook_fire: String,
    crosshair_size_pct: f32,
    crosshair_colour: String,
    binds: Vec<(String, String)>,
}

/// `PlayerSettings` as its file text.
pub fn render_settings(s: &PlayerSettings) -> String {
    let out = StoredSettings {
        schema: SETTINGS_SCHEMA,
        mouse_deg_per_px: s.mouse_deg_per_px,
        invert_y: s.invert_y,
        fov_deg: s.fov_deg,
        assist_catch_pct: s.assist_catch_pct,
        assist_strength_pct: s.assist_strength_pct,
        speed_fov_pct: s.speed_fov_pct,
        hook_fire: s.hook_fire.word().to_string(),
        crosshair_size_pct: s.crosshair_size_pct,
        crosshair_colour: s.crosshair_colour_name().to_string(),
        binds: BindAction::ALL
            .iter()
            .map(|a| {
                (
                    a.file_key().to_string(),
                    key_name(s.binds.get(*a)).unwrap_or("?").to_string(),
                )
            })
            .collect(),
    };
    let pretty = ron::ser::PrettyConfig::new().struct_names(false);
    ron::ser::to_string_pretty(&out, pretty).expect("StoredSettings has no unserialisable field")
}

/// The file text laid over a seeded `PlayerSettings` — **every number lands clamped into its
/// own slider window**, so a hand-edited `fov_deg: 500` comes back as [`FOV_MAX_DEG`] and not
/// as a fisheye nobody can navigate out of (the menu itself could never write one).
pub fn parse_settings(text: &str, seeded: PlayerSettings) -> Result<PlayerSettings, String> {
    let stored: StoredSettings = ron::de::from_str(text).map_err(|e| e.to_string())?;
    if stored.schema != SETTINGS_SCHEMA {
        return Err(format!(
            "settings schema {} but this build writes {SETTINGS_SCHEMA}",
            stored.schema
        ));
    }
    let mut s = seeded;
    s.mouse_deg_per_px =
        stored.mouse_deg_per_px.clamp(MOUSE_MIN_DEG_PER_PX, MOUSE_MAX_DEG_PER_PX);
    s.invert_y = stored.invert_y;
    s.fov_deg = stored.fov_deg.clamp(FOV_MIN_DEG, FOV_MAX_DEG);
    s.assist_catch_pct = stored.assist_catch_pct.clamp(ASSIST_MIN_PCT, ASSIST_MAX_PCT);
    s.assist_strength_pct = stored.assist_strength_pct.clamp(ASSIST_MIN_PCT, ASSIST_MAX_PCT);
    s.speed_fov_pct = stored.speed_fov_pct.clamp(0.0, 100.0);
    s.hook_fire = HookFire::from_word(&stored.hook_fire)
        .ok_or_else(|| format!("hook_fire {:?} is neither hold nor toggle", stored.hook_fire))?;
    s.crosshair_size_pct =
        stored.crosshair_size_pct.clamp(CROSSHAIR_MIN_PCT, CROSSHAIR_MAX_PCT);
    s.crosshair_colour = CROSSHAIR_COLOURS
        .iter()
        .position(|(name, _)| *name == stored.crosshair_colour)
        .ok_or_else(|| format!("crosshair colour {:?} is not in the table", stored.crosshair_colour))?;
    for (action_key, key) in &stored.binds {
        let action = BindAction::ALL
            .iter()
            .copied()
            .find(|a| a.file_key() == action_key)
            .ok_or_else(|| format!("{action_key:?} is not a bindable action"))?;
        let key = key_from_name(key).ok_or_else(|| format!("{key:?} is not a bindable key"))?;
        // Through `set`, never `set_raw`: the swap rule holds against the file too, so a
        // hand-edited duplicate cannot produce two actions on one key.
        s.binds.set(action, key);
    }
    Ok(s)
}

/// Where `settings.ron` lives — or `None`, and then nothing is read or written.
///
/// The same three-step contract as `save::file::SaveDir::discover` (see the module note above
/// for why it is spelled twice): `DBT_SAVE_DIR` decides first (`off`/empty is off), a test
/// binary (`target/*/deps/`) is off, otherwise `<crate root>/saves/settings.ron`.
pub fn settings_path() -> Option<PathBuf> {
    if let Ok(from_env) = std::env::var("DBT_SAVE_DIR") {
        let trimmed = from_env.trim();
        if trimmed.is_empty() || trimmed == "off" {
            return None;
        }
        return Some(PathBuf::from(trimmed).join("settings.ron"));
    }
    let in_a_test_binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.ends_with("deps")))
        .unwrap_or(false);
    if in_a_test_binary {
        return None;
    }
    let built_from = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if built_from.is_dir() {
        return Some(built_from.join("saves").join("settings.ron"));
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .map(|dir| dir.join("saves").join("settings.ron"))
}

/// The seeded settings with the stored ones laid over them — called by `from_world`, once.
pub fn load_settings(seeded: PlayerSettings) -> PlayerSettings {
    let Some(path) = settings_path() else {
        return seeded;
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        // Absent is the normal first run; anything else is worth one line.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return seeded,
        Err(e) => {
            warn!("settings file {} is unreadable ({e}) — using the seeds", path.display());
            return seeded;
        }
    };
    match parse_settings(&text, seeded) {
        Ok(s) => {
            info!("settings loaded from {}", path.display());
            s
        }
        Err(why) => {
            warn!(
                "settings file {} does not parse ({why}) — using the seeds; the next \
                 settings click overwrites it",
                path.display()
            );
            seeded
        }
    }
}

/// Writes the file — temp-then-rename like `save::file`, so a power cut mid-write leaves the
/// old file and never half of a new one. Failure is one log line, not a panic: a read-only
/// disk must not take the settings screen down with it.
pub fn store_settings(s: &PlayerSettings) {
    let Some(path) = settings_path() else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    let text = render_settings(s);
    let tmp = path.with_extension("ron.tmp");
    let written = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(&tmp, &text))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if let Err(e) = written {
        warn!("settings could not be written to {} ({e})", path.display());
    }
}
