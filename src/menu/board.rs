//! **The mission board** — the signpost in the muster yard, and the door into the overview
//! that is a *key* instead of a *place*.
//!
//! > *„wenn man in der hub auf ein board drueckt (F) dann kommt man in eine mission uebersciht
//! > in der man auswaehlen kann was man machen will!"* — the user, 2026-08-27
//! > (`docs/QUESTIONS.md` Q-059).
//!
//! ```text
//!   walk up to the signpost ──F──► the overview ──F──► the next sortie
//!                                        │  hold F
//!                                        └────────────► DeployRequest ──► mission
//! ```
//!
//! ## What this is NOT, and each line of it was paid for
//!
//! - **It is not a second mission list.** [`Screen::Lobby`] already lists `missions.ron` and
//!   already deploys what it shows (`tests/menu.rs::f175_the_lobby_deploys_the_sortie_it_shows`).
//!   This module opens it, moves its [`LobbyChoice`] and presses its *Deploy* — through the
//!   same `shared::DeployRequest` the button writes, so `mission` still has exactly one route
//!   in. Give the mechanism a front door; do not build a second mechanism.
//! - **It is not a second mission list on the floor either.** The six walk-on pads stay
//!   (`mission::hub::deploy_on_contact`, 35 asserts). The user said of the pads behind his
//!   back: *„Lass es, ich dreh mich um."* — so nothing turns him round, and this is the door
//!   he **finds**, 5.5 m in front of the spawn view.
//! - **It is not a prediction.** The retired `hud::hub_prompt` tried to say which door you
//!   were walking towards and was refuted four times. There is no ray here, no bearing rule and
//!   no walk model: the only question this module asks is *"is the local player inside the
//!   board's circle"*, which is a fact with a number, measured the same way a refuel station
//!   measures its own.
//!
//! ## `F`, and why one key is enough
//!
//! | you are | `F` does |
//! |---|---|
//! | not in the board's circle | **nothing** — `F` stays the left blade and this module returns before it looks at the keyboard |
//! | in the circle, board shut | opens the overview. **This press cannot deploy** — see [`Board::armed`] |
//! | in the circle, board open, released quickly | steps the choice on by one entry of `menu::lobby::entries` |
//! | in the circle, board open, held past `hub.board.hold_s` | deploys what the board is showing, and shuts |
//! | in a sortie | **nothing** — the board is `DespawnOnExit(MissionPhase::Hub)`, so the query is empty |
//!
//! **A hold and not a double-tap.** `src/net/local.rs` argues against gestures for the dodge on
//! accessibility grounds, and that argument is about a *window* you have to hit — two taps
//! inside 0.3 s. A hold has a floor and no ceiling: whoever is slow holds longer. And it is
//! never the only route — with a window, every entry on the plate is a mouse button and
//! *Deploy* is a mouse button.
//!
//! **The press that opens cannot also deploy.** Otherwise walking up and leaning on `F` would
//! fly whatever the board happened to be showing before it was on screen. You always deploy
//! something you have been shown.
//!
//! ## Real time, and it has to be
//!
//! The hold is measured on [`Time<Real>`]. With a window the overview is `Screen::Lobby`,
//! `menu::apply_screen` stops `Time<Virtual>` and therefore every `FixedUpdate` tick, so a hold
//! counted in ticks or in virtual seconds would never finish in exactly the run a player is
//! actually looking at. `Time<Real>` runs in all four launch modes.
//!
//! ## `Screen` is the plate's state, and a run with no plate does not get its screen moved
//!
//! `src/lib.rs` builds `primary_window: None` for `--headless` and `--offscreen`, `menu`'s whole
//! `Update` chain is gated on `there_is_a_window`, and `hud::hide_while_a_menu_is_up` hides the
//! HUD whenever [`Screen`] is not `Playing`. So writing `Screen::Lobby` in a windowless run
//! would blank the only surface such a run has and show nothing in its place. This module
//! therefore writes `Screen` **only where there is a window to draw it on**, and keeps its own
//! [`Board`] either way — which is also what lets `scripts/f177-board.txt` see the thing at all
//! (`FIND-189`: no script can press `Esc` or click a menu).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::lobby::{chosen, entries, LobbyChoice};
use super::Screen;
use crate::data::GameData;
use crate::mission::hub::MissionBoard;
use crate::shared::{DeployRequest, LocalPlayer};

/// **What the board is doing right now.** One writer — [`work_the_board`] — and everything
/// else reads it, including `hud::board`, which may not ask the question a second time
/// (`CLAUDE.md` rule 5, the corollary: one writer decides, everyone else reads the answer).
///
/// A `Resource` for the same reason [`Screen`] and [`LobbyChoice`] are: it is the state of
/// *this session's screen*, not of a player. `F` comes off this machine's keyboard, so the
/// player it measures is the local one — there is one keyboard here the way there is one mouse.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct Board {
    /// Whether the local player stands inside the board's circle. Written every frame; the HUD
    /// draws its prompt off this and never measures the distance itself.
    pub in_range: bool,
    /// Whether the overview is up.
    pub open: bool,
    /// How long `F` has been down **on a press that found the board already open**, in real
    /// seconds. `None` means this press cannot deploy — which is exactly the state the press
    /// that opened the board leaves behind.
    pub armed: Option<f32>,
    /// This press has already done its work (it opened the board, or it deployed). Cleared on
    /// release, so one press is one action and a long hold cannot also count as a tap.
    pub spent: bool,
}

/// The board, the key, and the deploy. `Update`, and **outside** `menu`'s window gate.
///
/// It runs in every launch mode on purpose: a `--headless` or `--offscreen` run has no window,
/// therefore no menu, therefore no way to press a button — and this is the only interaction in
/// the domain that a script can reach at all. Everything it writes is either its own
/// ([`Board`]), shared with the plate ([`LobbyChoice`], and the plate rebuilds on
/// `choice.is_changed()`), or a message somebody else acts on (`DeployRequest`).
#[allow(clippy::too_many_arguments)]
pub fn work_the_board(
    keys: Res<ButtonInput<KeyCode>>,
    real: Res<Time<Real>>,
    data: Res<GameData>,
    windows: Query<(), With<PrimaryWindow>>,
    boards: Query<(&MissionBoard, &Transform)>,
    players: Query<&Transform, With<LocalPlayer>>,
    mut board: ResMut<Board>,
    mut choice: ResMut<LobbyChoice>,
    mut screen: ResMut<Screen>,
    mut deploy: MessageWriter<DeployRequest>,
) {
    let windowed = !windows.is_empty();

    // **The nearest board wins**, not the first the query hands back — the same rule
    // `mission::hub::deploy_on_contact` follows, and for the same reason: archetype order is
    // not an answer a player can predict. Today there is one board; the rule costs nothing.
    // ⚠️ `players` is iterated and never `.single()`d (`docs/multiplayer.md` rule 3), and it is
    // filtered to `LocalPlayer` because a key on this keyboard is a fact about this machine.
    let mut nearest: Option<(f32, &MissionBoard)> = None;
    for player in &players {
        for (post, at) in &boards {
            let d = player.translation.distance(at.translation);
            if d > post.radius_m {
                continue;
            }
            if nearest.is_none_or(|(closest, _)| d < closest) {
                nearest = Some((d, post));
            }
        }
    }

    let in_range = nearest.is_some();
    if board.in_range != in_range {
        board.in_range = in_range;
    }

    // **Two ways the overview stops being up, and neither of them is a key.**
    //
    // 1. He walked away. Only a windowless run can do that — with a window `Screen::Lobby`
    //    stops the clock and he is standing still — and it is what makes the board a place
    //    rather than a modal you are stuck in.
    // 2. The plate went away: *Back*, *Deploy*, `Esc`. `Screen` is the plate's state and this
    //    module is not its only writer, so the board follows it rather than fighting it.
    //    One frame late, because `toggle_screen` may run after this system — invisible, and
    //    cheaper than an ordering that would make the open flicker.
    if board.open && (!in_range || (windowed && *screen != Screen::Lobby)) {
        shut(&mut board, &mut screen, windowed);
    }

    let Some((_, post)) = nearest else {
        // Out of reach, the key means nothing here — and `net::local` still reads it as the
        // left blade, exactly as it did before this file existed.
        if board.armed.is_some() {
            board.armed = None;
        }
        if board.spent {
            board.spent = false;
        }
        return;
    };

    if keys.just_pressed(KeyCode::KeyF) {
        if board.open {
            // Armed: this press may still become a hold.
            board.armed = Some(0.0);
            board.spent = false;
        } else {
            board.open = true;
            // **Spent, and that is the guard.** The press that opens the board may neither
            // step nor deploy: both are behind `!spent`. Measured 2026-08-28 — flipping only
            // the `armed` line below leaves every f177 test green, and flipping `spent` turns
            // `f177_the_press_that_opens_chooses_nothing_and_the_next_tap_steps_one_on` red
            // with `left: veteran, right: recruit`. So `armed = None` is belt: it says the
            // same thing a second way and it is what makes the hold accumulator not even
            // start, but the brace is this next line.
            board.armed = None;
            board.spent = true;
            if windowed && *screen != Screen::Lobby {
                *screen = Screen::Lobby;
            }
            info!(
                "board: the mission overview is open — {} sorties to choose from",
                entries(&data).len()
            );
        }
    }

    // ⚠️ **The release is read BEFORE the accumulator, and that order is the whole tap.** The
    // frame a key comes up in has `just_released` true and `pressed` false, so an accumulator
    // block that ran first would disarm the press and the release would then find nothing to
    // step — measured 2026-08-28: every tap did exactly nothing, twelve presses landed on one
    // sortie, and the four tests that say so all went red at once.
    if keys.just_released(KeyCode::KeyF) {
        if board.armed.is_some() && !board.spent {
            step(&data, &mut choice);
        }
        board.armed = None;
        board.spent = false;
    } else if let Some(held) = board.armed.as_mut() {
        if keys.pressed(KeyCode::KeyF) {
            *held += real.delta_secs();
            if *held >= post.hold_s && !board.spent {
                board.spent = true;
                board.armed = None;
                fly_it(&data, &choice, &mut deploy);
                shut(&mut board, &mut screen, windowed);
            }
        } else {
            // The key came up in a frame this system never saw `just_released` in — the input
            // was cleared by something else, or a frame was dropped. Disarm rather than let a
            // stale accumulator finish a hold nobody is still making.
            board.armed = None;
        }
    }
}

/// Puts the overview away, and takes the plate with it where there is one.
fn shut(board: &mut Board, screen: &mut Screen, windowed: bool) {
    board.open = false;
    board.armed = None;
    if windowed && *screen == Screen::Lobby {
        *screen = Screen::Playing;
    }
}

/// One entry on. Wraps, because a list you can only walk off the end of is a list you have to
/// close and reopen.
///
/// The current position is re-derived through [`chosen`] and never remembered as an index: a
/// stored index would point at a different sortie the moment `missions.ron` gains a line, and
/// the whole reason [`LobbyChoice`] holds keys instead of numbers is that a stale key falls back
/// to the default door while a stale index deploys somebody else's mission.
fn step(data: &GameData, choice: &mut LobbyChoice) {
    let list = entries(data);
    if list.is_empty() {
        error!("the mission board has nothing to show — assets/data/missions.ron has no templates");
        return;
    }
    let here = chosen(data, choice);
    let at = here.as_ref().and_then(|now| list.iter().position(|e| e == now)).unwrap_or(0);
    let (mission, difficulty) = list[(at + 1) % list.len()].clone();
    info!("board: {mission:?} {difficulty:?} — entry {} of {}", (at + 1) % list.len() + 1, list.len());
    choice.mission = Some(mission);
    choice.difficulty = difficulty;
}

/// **Deploy — and it asks, exactly as the *Deploy* button does.**
///
/// `mission::take_orders_from_the_menu` reads the message in `Update` and sets `Sortie` and the
/// phase. This module holds no mission state and never touches the phase
/// (`docs/architecture.md`: the `menu -> mission` edge is read-only).
fn fly_it(data: &GameData, choice: &LobbyChoice, deploy: &mut MessageWriter<DeployRequest>) {
    let Some((template, difficulty)) = chosen(data, choice) else {
        error!("the board was held with nothing to fly — assets/data/missions.ron has no templates");
        return;
    };
    info!("board: deploying {template:?} at {difficulty:?}");
    deploy.write(DeployRequest { template, difficulty });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of the state the opening press leaves behind. **The live guard is `spent`**
    /// and it is held by `tests/menu.rs::f177_the_press_that_opens_chooses_nothing_and_the_
    /// next_tap_steps_one_on`, which goes red on it; this one only names the two flags so that
    /// nobody reading the struct has to guess which of them is doing the work.
    #[test]
    fn f177_the_press_that_opens_the_board_is_spent_and_not_armed() {
        let opened = Board { in_range: true, open: true, armed: None, spent: true };
        assert!(opened.spent, "the opening press has done its one thing — this is the guard");
        assert!(opened.armed.is_none(), "and the accumulator never even starts");
    }
}
