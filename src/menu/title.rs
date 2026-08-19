//! The title screen — **the front door of the whole game.**
//!
//! > *„gibt es ein hauptmenü?"* — the user, 2026-08-19. There was not one: a flagless
//! > `cargo run` dropped him into the hub as a 3D place, with no game name, no *New Game* and
//! > no *Quit* anywhere before the first frame of play.
//!
//! `docs/backlog/ui.ron` `UI-001` ("Startbildschirm") has specified this row the whole time —
//! *"Erster Bildschirm nach dem Laden"*, elements *"Play, Neuigkeiten, Einstellungen,
//! Sozial-Links"*. Two of those four exist today and two do not, and **only the two that exist
//! are on the plate**: there is no news feed to show and no social link to open, and a row that
//! cannot do anything is the registry rule of §4 in menu form — do not add a row nothing can
//! spawn.
//!
//! ## What is deliberately NOT on it
//!
//! - **No *Continue* and no *Load*.** Nothing can be continued: `save` is being built in a
//!   parallel round and there is no save file to read. The moment there is one, *Continue*
//!   becomes the **first** row of [`rows`] and nothing else on this screen has to move — see
//!   the note there for the one condition it needs.
//! - **No *Mission select*.** [`Screen::Lobby`](super::Screen::Lobby) keeps its single route in
//!   (the pause screen), the same way the settings screen kept its own until today. The hub is
//!   one click away through *New Game* and the lobby one `Esc` after that, so `F-175`'s *"every
//!   screen in at most two clicks"* is unaffected — and a second door into the lobby would have
//!   needed a second answer to "where does its Back button go".
//!
//! ## The pointer here is a case nothing else in this domain has
//!
//! Every other screen is reached **out of** the game, so the pointer was captured and gets
//! handed back. The title stands there before anything has been played: nothing has grabbed it
//! yet, and nothing may. That is why [`super::Screen`] is decided by `FromWorld` out of
//! [`Cli`](crate::shared::Cli) rather than by a `Startup` system — a startup system would leave
//! the screen at `Playing` for the first frame's `apply_screen`, which would lock the pointer to
//! the compositor and release it one frame later. `tests/menu.rs` holds both halves: the plate
//! is up with a free cursor, and **not one tick of simulation** has run underneath it.

use bevy::prelude::*;

use super::{plate, Screen};

/// What a button on the title screen does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleAction {
    /// Into the game: the hub, which is already standing behind this plate.
    NewGame,
    /// The options — the second route into [`Screen::Settings`], and the reason
    /// [`super::SettingsFrom`] exists.
    Settings,
    /// To desktop. The one entry a launcher screen must never be missing.
    Quit,
}

/// The rows, top to bottom, and the one place they are decided.
///
/// ⚠️ **Where *Continue* goes when there is something to continue:** first in this list,
/// `(TitleAction::Continue, "Continue")`, guarded by whatever `save` ends up exposing as "there
/// is a save" — a `Res<...>` read here and passed in by [`super::spawn_menu`], exactly the way
/// `in_a_sortie` is passed into the pause screen. Nothing else on this screen changes; the
/// button below it keeps its label and its meaning.
fn rows() -> Vec<(TitleAction, &'static str)> {
    vec![
        (TitleAction::NewGame, "New Game"),
        (TitleAction::Settings, "Settings"),
        (TitleAction::Quit, "Quit"),
    ]
}

/// Builds the plate — the game's name and the entries that actually work.
///
/// The name comes from [`crate::WINDOW_TITLE`] and not from a string here:
/// `docs/conventions.md` names exactly one place for each spelling of the project's name, and
/// the window bar and the title screen have to say the same thing.
pub fn spawn_title_screen(commands: &mut Commands) {
    commands.spawn(plate::root(Screen::Title, "title")).with_children(|screen| {
        screen.spawn(plate::title(crate::WINDOW_TITLE));
        screen.spawn(plate::note("Two hooks, two tanks, two blades."));

        for (action, label) in rows() {
            screen
                .spawn((
                    Name::new(format!("title_{action:?}")),
                    action,
                    plate::button(plate::BUTTON_W, false),
                ))
                .with_child(plate::label(label));
        }
    });
}

/// What the buttons do.
///
/// *New Game* writes nothing but the screen. **The world behind the plate is already there** —
/// a flagless run loads the hub and then stops the clock (`menu::apply_screen`), so starting is
/// letting go rather than building: no second boot path, no `DeployRequest`, and `mission` keeps
/// being the only writer of the phase.
pub fn title_buttons(
    buttons: Query<(&Interaction, &TitleAction)>,
    mut screen: ResMut<Screen>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            TitleAction::NewGame => {
                info!("title: new game - into the hub");
                *screen = Screen::Playing;
            }
            TitleAction::Settings => *screen = Screen::Settings,
            TitleAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
