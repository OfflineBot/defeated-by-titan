//! The pause screen — **the menu the user asked for**, and the hub of the other two.
//!
//! > *„menu (also bei escape)"* — the user, 2026-08-13. Until that morning `Esc` offered
//! *Resume* and *Quit* and nothing else; `F-175` ("every screen in at most two clicks") was
//! 🟨 for exactly that reason (`docs/PLAN-GAME.md` §6, R3-A).
//!
//! Five ways out now, and one of them is conditional:
//!
//! | button | what it does | when |
//! |---|---|---|
//! | Resume | back into the game | always — and `Esc` is the same thing |
//! | Settings | [`Screen::Settings`](super::Screen::Settings) | always |
//! | Abandon sortie | `shared::AbandonSortie` → back to the hub, **no verdict** | only in a sortie |
//! | Quit to lobby / Mission select | [`Screen::Lobby`](super::Screen::Lobby), giving up a running sortie on the way | always |
//! | Quit to desktop | `AppExit` | always |
//!
//! Every one of them is also reachable **without a mouse** at least as far as `Esc` goes: `Esc`
//! is Resume. In this environment that is not a nicety, it is the only way anybody here can
//! operate this screen at all — nobody has ever run this game in a window on machine A.
//!
//! ⚠️ **This screen writes no mission state.** *Abandon* and *Quit to lobby* send
//! `shared::AbandonSortie` and `mission` decides what that means; the phase keeps exactly one
//! writer (`docs/architecture.md`, authority table). The edge `menu -> mission` in the allow
//! list is **read-only** and buys one thing: knowing whether there is a sortie to abandon.

use bevy::prelude::*;

use super::{plate, PauseAction, Screen};
use crate::shared::AbandonSortie;

/// **The row that opens the mission list**, and the HUD quotes this constant rather than
/// spelling the words a second time.
///
/// Anything that tells the player where the mission list is quotes this constant rather than
/// spelling the words a second time — the pause column itself, and since 2026-08-28 the
/// **mission board in the hub** the user asked for on 2026-08-27 (*„wenn man in der hub auf ein
/// board drückt (F) dann kommt man in eine mission übersciht"*, `Q-062`): `hud::board` draws this
/// exact string while you stand at the signpost, so a renamed row renames the prompt with it.
/// ⚠️ This paragraph pointed at `Q-059` until then, which is about the **retired hub line** and
/// not about the board — `F-177`'s question is `Q-062`.
///
/// A screen that carries its own copy of a label is exactly how a renamed button ends up being
/// promised somewhere it no longer exists — `FIND-178`, where the HUD lettered `Q`/`E` on a snap
/// that did not exist. **One spelling, one place.**
pub const MISSION_SELECT_ROW: &str = "Mission select";

/// Builds the plate. Called by `menu::spawn_menu`, which owns the "is one already there"
/// question for all five screens.
///
/// `in_a_sortie` decides one button and one label, and it is passed in rather than read here so
/// that this function stays a pure builder — the phase is read in exactly one place
/// (`menu::in_a_sortie`).
pub fn spawn_pause_screen(commands: &mut Commands, in_a_sortie: bool) {
    commands.spawn(plate::root(Screen::Paused, "pause")).with_children(|screen| {
        screen.spawn(plate::title("Paused"));

        // A `Vec` and not an array: the sortie-only button is the whole reason this screen
        // needed to know the phase.
        let mut buttons = vec![
            (PauseAction::Resume, "Resume  (Esc)".to_string()),
            (PauseAction::Settings, "Settings".to_string()),
        ];
        if in_a_sortie {
            buttons.push((PauseAction::Abandon, "Abandon sortie".to_string()));
            buttons.push((PauseAction::Lobby, "Quit to lobby".to_string()));
        } else {
            // In the hub there is nothing to quit *from* — the same door is simply the way to
            // the mission list.
            buttons.push((PauseAction::Lobby, MISSION_SELECT_ROW.to_string()));
        }
        buttons.push((PauseAction::Quit, "Quit to desktop".to_string()));

        for (action, label) in buttons {
            screen
                .spawn((
                    Name::new(format!("pause_{action:?}")),
                    action,
                    plate::button(plate::BUTTON_W, false),
                ))
                .with_child(plate::label(label));
        }
    });
}

/// What the buttons do.
///
/// Quit writes `AppExit` and does **not** despawn anything: ending the run belongs to the app,
/// and a menu that tears its own world down first leaves nothing to shut down cleanly.
///
/// *Abandon* and *Quit to lobby* both write [`AbandonSortie`] unconditionally. That is not
/// sloppiness — `mission` checks the phase before it acts on the message, and putting the same
/// check here as well would give the answer two owners for the sake of one message that is
/// already ignored when it means nothing.
pub fn pause_buttons(
    buttons: Query<(&Interaction, &PauseAction)>,
    mut screen: ResMut<Screen>,
    mut abandon: MessageWriter<AbandonSortie>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseAction::Resume => *screen = Screen::Playing,
            PauseAction::Settings => *screen = Screen::Settings,
            PauseAction::Abandon => {
                abandon.write(AbandonSortie);
                // Straight back to the game: what he wanted was the hub floor, not a menu.
                *screen = Screen::Playing;
            }
            PauseAction::Lobby => {
                abandon.write(AbandonSortie);
                *screen = Screen::Lobby;
            }
            PauseAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
