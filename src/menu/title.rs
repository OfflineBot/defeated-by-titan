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
//! spawn. The first of the four is called *Play* in the backlog and it is called *Play* here.
//!
//! ## Where *Play* goes, and why it is not the hub
//!
//! ⚠️ **Changed on 2026-08-24, and it closed a routing hole.** *New Game* used to hand the
//! screen straight to the game, which is the hub — and the **lobby was then unreachable from a
//! cold start**. It existed, it worked, it listed every mission in `missions.ron`, and the only
//! door into it was the pause screen *inside a game that was already running*: a player who
//! never pressed `Esc` never saw that this game has a mission list at all
//! (the user, 2026-08-23: *„es fehlt die lobby"* — it did not, the way to it did).
//!
//! So *Play* opens [`Screen::Lobby`](super::Screen::Lobby). The hub is still standing behind
//! it, stopped, exactly as before; the lobby's *Back* is the hub floor, one click further.
//! Nothing about the second door had to be answered twice, because the lobby's *Back* was
//! already "hand the screen to the game" and that is the same sentence from either route.
//! `F-175`'s acceptance — *every screen in at most two clicks* — is met by the shorter path,
//! not the longer one: mission list 1, hub 2, settings 1.
//!
//! ## What is deliberately NOT on it
//!
//! - **No *Continue* and no *New Game* next to each other.** There is one profile per player and
//!   `save` loads it by itself (`save::load_profiles`), so today both rows would do the same
//!   thing and one of them would be a lie. See [`rows`] for the two things that have to exist
//!   before they split.
//! - **No *Load*, no news feed, no social links.** `UI-001` asks for the last two and there is
//!   nothing behind either; do not add a row nothing can spawn.
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
    /// Into the game — and the first thing the game asks is *which sortie*, so this opens the
    /// **lobby**. The hub is already standing behind the plate and is one *Back* further.
    ///
    /// ⚠️ The variant kept the name `NewGame` on purpose while the label became *Play*: the day
    /// there is a profile to continue, `NewGame` is the row that starts a **fresh** one and
    /// `Continue` is the row that resumes — and a variant renamed twice is a variant nobody can
    /// grep for through the history.
    NewGame,
    /// The options — the second route into [`Screen::Settings`], and the reason
    /// [`super::SettingsFrom`] exists.
    Settings,
    /// To desktop. The one entry a launcher screen must never be missing.
    Quit,
}

/// The rows, top to bottom, and the one place they are decided.
///
/// Three, and every one of them does something today:
///
/// | row | what it does | why it is here |
/// |---|---|---|
/// | *Play* | opens the lobby | the mission list was unreachable without it |
/// | *Settings* | opens the options | the second route into them, and the reason `SettingsFrom` exists |
/// | *Quit* | `AppExit` | the one entry a launcher screen must never be missing |
///
/// ⚠️ **The label is *Play* and not *New Game*, and that is the honest word today.**
/// `save::load_profiles` gives every player his `saves/player-<id>.ron` at spawn and writes it
/// back after every decided sortie (`F-200`), so a row called *New Game* would silently
/// **continue** a career instead of starting one. A row that says the opposite of what it does
/// is worse than a row that is missing.
///
/// ⚠️ **Where *Continue* goes when there is something to continue** — first in this list,
/// `(TitleAction::Continue, "Continue")`, with *New Game* keeping second place. It needs
/// exactly two things that do not exist today, and neither of them is in this file:
///
/// 1. **A way to ask whether there is anything to continue.** `save::Profile` already holds the
///    answer (`sorties_flown > 0`, `cleared` — `src/save/profile.rs`), but it is a component
///    **per player** and this screen stands before any player has been seated, so the question
///    has to be answered by `save` about the *file* and not about an entity. That is a
///    `save`-side read plus one line on the allow list in `docs/architecture.md`
///    (`menu -> save`, read-only), and it arrives here the way `in_a_sortie` arrives at the
///    pause screen: read once in [`super::spawn_menu`] and passed in as a `bool`.
/// 2. **A way to start a fresh one.** `save::load_profiles` reads the file unconditionally, so
///    there is nothing a *New Game* row could ask for that would not be the same career again.
///    Until `save` can be told "begin a new profile", the two rows are one row.
fn rows() -> Vec<(TitleAction, &'static str)> {
    vec![
        (TitleAction::NewGame, "Play"),
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
/// *Play* writes nothing but the screen. **The world behind the plate is already there** —
/// a flagless run loads the hub and then stops the clock (`menu::apply_screen`), so starting is
/// letting go rather than building: no second boot path, no `DeployRequest`, and `mission` keeps
/// being the only writer of the phase. The screen it hands over to is the **lobby**, which does
/// not let go either: the clock stays stopped until something on that plate is pressed.
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
                info!("title: play - into the lobby");
                *screen = Screen::Lobby;
            }
            TitleAction::Settings => *screen = Screen::Settings,
            TitleAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
