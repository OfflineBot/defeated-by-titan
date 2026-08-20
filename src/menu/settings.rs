//! The settings screen — **six things a person may change about his own game.**
//!
//! > *„zudem fehlen settings."* — the user, 2026-08-13 (`docs/NEXT.md` §1D req 6).
//!
//! | row | field | window | seeded from |
//! |---|---|---|---|
//! | Mouse sensitivity | `mouse_deg_per_px` | 0.01 – 0.60 °/px, step 0.01 | `game.ron: camera.mouse_deg_per_px` (0.08) |
//! | Invert Y | `invert_y` | on / off | nothing — it is a preference, not a game value |
//! | Field of view | `fov_deg` | 55 – 110°, step 5 | `game.ron: camera.fov_deg` (60) |
//! | Aim spread | `aim_spread_deg` | `game.ron: vector.aim_spread_min_deg … _max_deg` | `game.ron: vector.aim_spread_deg` (28) |
//! | Aim assist reach | `assist_catch_pct` | 0 – 100 %, step 5 | nothing — `F-016` defines 0 % as free aim |
//! | Aim assist strength | `assist_strength_pct` | 0 – 100 %, step 5 | the same |
//!
//! ## The last two rows are the ones he asked for by name
//!
//! > *„die accuracy von anzeige zu wo seil landet ist nicht immer korrekt … es sollte best match
//! > sein. und seinstellen können wie weit ca es sein sollte und wie aggressive (damit ich testen
//! > kann was am besten wäre mach debug einstellungen dafür)"* — the user, 2026-08-18.
//!
//! `F-016` is the feature and it specifies the shape exactly: a **stepless 0–100 % snap catch
//! angle where 0 % is today's pure free aim**. So both rows start at 0, and at 0 the game aims
//! precisely as it did before they existed — which is what makes them safe to ship a round
//! before `F-024`/`F-025` build the candidate scoring that will read them.
//!
//! ⚠️ **Today they are knobs with no consumer yet, and the screen says so** rather than
//! implying an effect that is not there. What they already do is the thing he actually asked
//! for: they are live, they need no restart (`F-024`), and every change **prints its own value
//! into the log** so he can tell us the number he liked. A knob whose setting he cannot report
//! back is half a knob.
//!
//! ## The aim-spread row is the one that matters, and it is the one that could have been a bug
//!
//! `F-023` put the aim spread on the **mouse wheel** the day before this screen existed
//! (`net::local::read_input`, *„mit mausrad soll man einstellen können wie weit auseinander es
//! gehen darf"*). A settings screen with a slider of its own would have made two numbers out of
//! one: turn the wheel, open the settings, and the screen shows the value the wheel started
//! from. So the wheel's accumulator was **moved into**
//! [`PlayerSettings::aim_spread_deg`](crate::shared::PlayerSettings) and this row edits that
//! same field. One field, one writer — reached by two devices
//! (`src/shared/settings.rs` carries the long form of the argument).
//!
//! ## Why every row rebuilds the whole plate
//!
//! A click writes `PlayerSettings`, `menu::despawn_menu` sees `is_changed()` and takes the
//! plate down, `menu::spawn_menu` builds it again out of the new values. One place decides what
//! this screen shows — its `spawn` — instead of two that drift apart the first time a row is
//! added. It costs a rebuild **per click**, never per frame (§6 rule 6), and it has a second
//! effect that is worth more than the tidiness: a held mouse button cannot ramp a slider,
//! because the button it is holding no longer exists.

use bevy::prelude::*;

use super::{plate, PauseElement, Screen, SettingsFrom};
use crate::data::GameData;
use crate::shared::settings::{
    ASSIST_CATCH_MAX_DEG, ASSIST_MAX_PCT, ASSIST_MIN_PCT, FOV_MAX_DEG, FOV_MIN_DEG,
    MOUSE_MAX_DEG_PER_PX, MOUSE_MIN_DEG_PER_PX,
};
use crate::shared::PlayerSettings;

/// Which way an arrow points. A sign, so the arithmetic below is one line per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    Down,
    Up,
}

impl Nudge {
    fn steps(self) -> f32 {
        match self {
            Nudge::Down => -1.0,
            Nudge::Up => 1.0,
        }
    }

    fn arrow(self) -> &'static str {
        match self {
            Nudge::Down => "-",
            Nudge::Up => "+",
        }
    }
}

/// What a button on the settings screen does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    Mouse(Nudge),
    InvertY,
    Fov(Nudge),
    Spread(Nudge),
    /// `F-016` — how far off the crosshair the assist may catch. 0 % is free aim.
    AssistCatch(Nudge),
    /// `F-016` / `F-024` — how hard it pulls once it has a candidate. 0 % is free aim.
    AssistStrength(Nudge),
    /// Back to the screen the options were opened from — the same place `Esc` goes, and
    /// since 2026-08-19 that is a recorded answer ([`SettingsFrom`]) rather than a constant:
    /// the title screen opens the options too.
    Back,
}

/// Builds the plate out of the **current** values. Called by `menu::spawn_menu`.
pub fn spawn_settings_screen(commands: &mut Commands, data: &GameData, s: &PlayerSettings) {
    let v = &data.game.vector;
    commands.spawn(plate::root(Screen::Settings, "settings")).with_children(|screen| {
        screen.spawn(plate::title("Settings"));

        row(
            screen,
            "Mouse sensitivity",
            &format!("{:.2} deg/px", s.mouse_deg_per_px),
            &format!("{MOUSE_MIN_DEG_PER_PX:.2} - {MOUSE_MAX_DEG_PER_PX:.2}"),
            SettingsAction::Mouse,
        );
        toggle_row(screen, "Invert Y", s.invert_y);
        row(
            screen,
            "Field of view",
            &format!("{:.0} deg", s.fov_deg),
            &format!("{FOV_MIN_DEG:.0} - {FOV_MAX_DEG:.0}, vertical"),
            SettingsAction::Fov,
        );

        // **The middle of the screen, kept empty.** The `hud` draws the aim assist's search
        // extent level with the crosshair — the picture of the very number two rows below —
        // and it stays on screen while this plate is up (`hud::ShowWhileTuning`). It cannot be
        // moved aside, so the plate is what makes room. It sits **here** and not anywhere else
        // because here is the middle of this column, and a centred column puts its middle on
        // the middle of the screen.
        screen.spawn(plate::centre_lane());

        row(
            screen,
            "Aim spread",
            // ⚠️ "deg apart max" and not "deg max": since 2026-08-18 the number is the angle
            // **between** the two rays and it is a ceiling, not the angle they take
            // (`vector::aim::wheel_half_rad`, FIND-096). The old caption read as a per-ray
            // half-angle and was off by a factor of two (`docs/NEXT.md` §2D).
            &format!("{:.1} deg apart max", s.aim_spread_deg),
            // Named, because a player who has already found the wheel has to see that this is
            // the same number and not a second one — and since 2026-08-18 it is a CEILING, so
            // the row says what the ropes actually do at that setting instead of a degree the
            // game only obeys when you point at the sky. The metres are the standing target
            // scaled by this notch (`src/vector/aim.rs::effective_spread_rad`, step 2).
            &format!(
                "{:.0} - {:.0}, the mouse wheel sets it too — up to {:.0} m apart",
                v.aim_spread_min_deg,
                v.aim_spread_max_deg,
                (v.aim_sep_stand_m * s.aim_spread_deg / v.aim_sep_neutral_deg)
                    .max(v.aim_sep_floor_m)
            ),
            SettingsAction::Spread,
        );
        // `F-016`, the two the user asked for. Their hint lines carry two things a slider
        // cannot: what 0 means, and — for the reach — what the percentage is in degrees, so
        // the number he reports back to us is one we can act on.
        row(
            screen,
            "Aim assist reach",
            &format!("{:.0} %", s.assist_catch_pct),
            &format!(
                "{ASSIST_MIN_PCT:.0} - {ASSIST_MAX_PCT:.0}, 0 = free aim — now {:.1} deg off \
                 the crosshair (max {ASSIST_CATCH_MAX_DEG:.0})",
                s.assist_catch_deg()
            ),
            SettingsAction::AssistCatch,
        );
        row(
            screen,
            "Aim assist strength",
            &format!("{:.0} %", s.assist_strength_pct),
            // It says "not wired up yet" in so many words. An honest empty corner beats a
            // control that pretends to do something — the same rule `hud` is built on.
            &format!(
                "{ASSIST_MIN_PCT:.0} - {ASSIST_MAX_PCT:.0}, 0 = free aim — how hard it snaps \
                 (F-025 is not built yet, so this is off)"
            ),
            SettingsAction::AssistStrength,
        );

        screen
            .spawn((
                Name::new("settings_Back"),
                SettingsAction::Back,
                plate::button(plate::BUTTON_W, false),
            ))
            .with_child(plate::label("Back  (Esc)"));
    });
}

/// One adjustable row: `label   [-]  value  [+]`, and a dim line under it saying what the
/// window is. The window is written down because a slider that silently stops is a slider the
/// player thinks is broken.
fn row(
    screen: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    hint: &str,
    make: fn(Nudge) -> SettingsAction,
) {
    screen.spawn(plate::row()).with_children(|line| {
        line.spawn((PauseElement, Node { width: Val::Px(plate::LABEL_W), ..default() }))
            .with_child(plate::label(label.to_string()));
        // `- value +`, in that order: the value sits **between** its two arrows, so the eye
        // reads the row as one thing instead of three.
        line.spawn((
            Name::new(format!("settings_{:?}", make(Nudge::Down))),
            make(Nudge::Down),
            plate::button(plate::ARROW_W, false),
        ))
        .with_child(plate::label(Nudge::Down.arrow()));
        line.spawn((
            PauseElement,
            Node {
                width: Val::Px(plate::VALUE_W),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child(plate::label(value.to_string()));
        line.spawn((
            Name::new(format!("settings_{:?}", make(Nudge::Up))),
            make(Nudge::Up),
            plate::button(plate::ARROW_W, false),
        ))
        .with_child(plate::label(Nudge::Up.arrow()));
    });
    screen.spawn(plate::note(hint.to_string()));
}

/// The on/off row. One button that says what it currently is.
///
/// **It spans [`plate::SPAN_W`] and not a width of its own.** At 208 px the row came out 406 px
/// wide against the others' 452, and because every row is centred independently that pushed
/// this label 24 px to the right and left the toggle's edges lining up with neither the `-`
/// column nor the `+` column — four rows and no grid (FIND-092 §4,
/// `docs/images/f175-settings.png`).
///
/// **And the hint follows the value.** It used to be the constant string
/// *"mouse forward looks down"*, shown under `Invert Y: off` — which is what the setting does
/// when it is **on**. A caption that describes the state you are not in is worse than none:
/// pushing the mouse forward is `d.y < 0`, and `net::local::read_input` raises the pitch by
/// `-pitch_sign() * d.y`, so with `invert_y` off a forward push looks **up**.
fn toggle_row(screen: &mut ChildSpawnerCommands, label: &str, on: bool) {
    screen.spawn(plate::row()).with_children(|line| {
        line.spawn((PauseElement, Node { width: Val::Px(plate::LABEL_W), ..default() }))
            .with_child(plate::label(label.to_string()));
        line.spawn((
            Name::new("settings_InvertY"),
            SettingsAction::InvertY,
            plate::button(plate::SPAN_W, on),
        ))
        .with_child(plate::label(if on { "on" } else { "off" }));
    });
    screen.spawn(plate::note(if on {
        "mouse forward looks down"
    } else {
        "mouse forward looks up"
    }));
}

/// What the buttons do — **the only place a setting is written by a click.**
///
/// `PlayerSettings` is taken as `ResMut` and touched only inside the pressed branch: a
/// `DerefMut` on a resource marks it changed for every reader, and this system runs every
/// frame (§6 rule 6). The one write per click is what makes the plate rebuild.
///
/// The aim-spread window comes out of `game.ron` and not out of a constant here — it is the
/// same window the wheel obeys, and a second copy of it would be a second answer.
pub fn settings_buttons(
    buttons: Query<(&Interaction, &SettingsAction)>,
    data: Res<GameData>,
    back: Res<SettingsFrom>,
    mut settings: ResMut<PlayerSettings>,
    mut screen: ResMut<Screen>,
) {
    let v = &data.game.vector;
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            SettingsAction::Mouse(n) => settings.nudge_mouse(n.steps()),
            SettingsAction::InvertY => {
                let inverted = settings.invert_y;
                settings.invert_y = !inverted;
            }
            SettingsAction::Fov(n) => settings.nudge_fov(n.steps()),
            SettingsAction::Spread(n) => settings.nudge_spread(
                n.steps(),
                v.aim_spread_step_deg,
                v.aim_spread_min_deg,
                v.aim_spread_max_deg,
            ),
            // ⚠️ **Both print.** `F-024`'s acceptance is that a change is live without a
            // restart, and the user's own reason for asking is that he wants to *test* and
            // tell us what felt best — so the value goes into the log the moment it moves.
            // One line per click, never per frame: this branch only runs on `Pressed`.
            SettingsAction::AssistCatch(n) => {
                settings.nudge_assist_catch(n.steps());
                info!(
                    "aim assist reach = {:.0} % ({:.1} deg off the crosshair)",
                    settings.assist_catch_pct,
                    settings.assist_catch_deg()
                );
            }
            SettingsAction::AssistStrength(n) => {
                settings.nudge_assist_strength(n.steps());
                info!("aim assist strength = {:.0} %", settings.assist_strength_pct);
            }
            SettingsAction::Back => *screen = back.0,
        }
    }
}
