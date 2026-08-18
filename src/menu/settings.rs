//! The settings screen — **four things a person may change about his own game.**
//!
//! > *„zudem fehlen settings."* — the user, 2026-08-13 (`docs/NEXT.md` §1D req 6).
//!
//! | row | field | window | seeded from |
//! |---|---|---|---|
//! | Mouse sensitivity | `mouse_deg_per_px` | 0.01 – 0.60 °/px, step 0.01 | `game.ron: camera.mouse_deg_per_px` (0.08) |
//! | Invert Y | `invert_y` | on / off | nothing — it is a preference, not a game value |
//! | Field of view | `fov_deg` | 55 – 110°, step 5 | `game.ron: camera.fov_deg` (60) |
//! | Aim spread | `aim_spread_deg` | `game.ron: vector.aim_spread_min_deg … _max_deg` | `game.ron: vector.aim_spread_deg` (28) |
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

use super::{plate, PauseElement, Screen};
use crate::data::GameData;
use crate::shared::settings::{
    FOV_MAX_DEG, FOV_MIN_DEG, MOUSE_MAX_DEG_PER_PX, MOUSE_MIN_DEG_PER_PX,
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
    /// Back to the pause screen — the same place `Esc` goes.
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
        row(
            screen,
            "Aim spread",
            &format!("{:.1} deg", s.aim_spread_deg),
            // Named, because a player who has already found the wheel has to see that this is
            // the same number and not a second one.
            &format!(
                "{:.0} - {:.0}, the mouse wheel sets it too",
                v.aim_spread_min_deg, v.aim_spread_max_deg
            ),
            SettingsAction::Spread,
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
            SettingsAction::Back => *screen = Screen::Paused,
        }
    }
}
