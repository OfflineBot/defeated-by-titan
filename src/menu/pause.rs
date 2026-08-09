//! The pause screen — one screen, two ways out.
//!
//! `F-175` wants *every* screen in at most two clicks. This is **one** of them; main menu,
//! options, loadout and debrief do not exist. Reported as 🟨 for exactly that reason
//! (`docs/PLAN-GAME.md` §6, R3-A).
//!
//! Both buttons are also reachable **without a mouse**: `Esc` is Resume. In this environment
//! that is not a nicety, it is the only way anybody here can operate this screen at all
//! (`prompts/init.md` §12a).

use bevy::prelude::*;
use bevy::text::FontSize;

use super::{PauseAction, PauseElement, Screen};

/// Backdrop of the pause screen. Dark, not black: the game behind it stays readable, so a
/// screenshot of a paused frame still shows what was paused.
const BACKDROP: Color = Color::srgba(0.02, 0.03, 0.05, 0.72);
const PLATE: Color = Color::srgb(0.10, 0.12, 0.15);
const INK: Color = Color::srgb(0.90, 0.93, 0.96);

/// Builds the overlay when the screen turns to [`Screen::Paused`] and there is none yet.
///
/// Self-healing rather than message-driven: the condition is *"paused and nothing on
/// screen"*, so a pause screen can never be missing and can never be there twice — no matter
/// in which order the toggle and the spawn happen to run.
pub fn spawn_pause_screen(mut commands: Commands, present: Query<Entity, With<PauseElement>>) {
    if !present.is_empty() {
        return;
    }
    commands
        .spawn((
            Name::new("pause_backdrop"),
            PauseElement,
            BackgroundColor(BACKDROP),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|screen| {
            screen.spawn((
                Name::new("pause_title"),
                PauseElement,
                Text::new("Paused"),
                TextFont { font_size: FontSize::Px(34.0), ..default() },
                TextColor(INK),
            ));
            for (action, label) in [
                (PauseAction::Resume, "Resume  (Esc)"),
                (PauseAction::Quit, "Quit"),
            ] {
                screen
                    .spawn((
                        Name::new(format!("pause_{action:?}")),
                        PauseElement,
                        action,
                        Button,
                        BackgroundColor(PLATE),
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_child((
                        PauseElement,
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(18.0), ..default() },
                        TextColor(INK),
                    ));
            }
        });
}

/// Clears the overlay again. A pause screen that survives Resume covers the game.
pub fn despawn_pause_screen(mut commands: Commands, present: Query<Entity, With<PauseElement>>) {
    for e in &present {
        // The children carry `PauseElement` too, so `despawn` is called on entities that a
        // parent may already have taken with it. `try_despawn` is the difference between
        // "the screen is gone" and a panic in a menu.
        commands.entity(e).try_despawn();
    }
}

/// What the two buttons do.
///
/// Quit writes `AppExit` and does **not** despawn anything: ending the run belongs to the app,
/// and a menu that tears its own world down first leaves nothing to shut down cleanly.
pub fn pause_buttons(
    buttons: Query<(&Interaction, &PauseAction)>,
    mut screen: ResMut<Screen>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseAction::Resume => *screen = Screen::Playing,
            PauseAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
