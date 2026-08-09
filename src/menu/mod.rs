//! menu — pause screen, the pointer, and the way back out
//!
//! For somebody who cannot click, a main menu is a wall without a door — which is why
//! `--sandbox`, `--mission` and `--script` exist and walk straight past it
//! (`prompts/init.md` §12a).
//!
//! Rebindable keys, color-blind modes, a screenshake slider and reduced motion are
//! requirements, not decoration (Bible 3.5). **None of them is here.**
//!
//! ## What this domain owns: the pointer
//!
//! A mouse-look game that leaves the system cursor free lets you turn until the screen edge
//! and then stop. So the pointer is **locked and hidden** the moment there is a window, and
//! `Esc` gives it back (`docs/PLAN-GAME.md` §8, `P4`).
//!
//! **The release was built before the capture.** A locked pointer with no release key is a
//! game you have to `pkill` — and on a machine where nobody has ever seen this game in a
//! window, that failure would have been found by somebody else, later, without a terminal
//! open.
//!
//! ## What decides: the window entity, not the flag
//!
//! Every system here is gated on `With<PrimaryWindow>`. `src/lib.rs` builds
//! `primary_window: None` whenever `Cli::wants_window()` is false, so `--headless` and
//! `--offscreen` have **no window entity** and therefore grab nothing — without this file
//! having to ask `Cli` at all. One condition instead of two that can drift apart, and it is
//! the *true* one: whether there is a pointer to take.
//!
//! ## `F-175` is **not** claimed here
//!
//! F-175 is "every screen in at most two clicks". This is **one** screen — pause, with Resume
//! and Quit. Main menu, options, loadout and the debrief do not exist. The row stays 🟨
//! (`docs/PLAN-GAME.md` §6, R3-A).

pub mod pause;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screen>().add_systems(
            Update,
            (
                // The order is the answer to "what does one `Esc` do": it flips the screen,
                // and everything downstream follows the screen. A button that ran after the
                // pointer had already been applied would leave the mouse one frame behind
                // the state it belongs to — and one frame of a free pointer over a running
                // game is one frame in which a click lands outside the window.
                (toggle_pause, pause::pause_buttons),
                apply_screen,
                pause::spawn_pause_screen.run_if(paused),
                pause::despawn_pause_screen.run_if(not(paused)),
            )
                .chain()
                .run_if(there_is_a_window),
        );
    }
}

/// Whether this run has a pointer to take at all.
///
/// **This, and not `Cli`, is the condition.** `src/lib.rs` builds `primary_window: None`
/// whenever `Cli::wants_window()` is false, so `--headless` and `--offscreen` have no window
/// entity — and asking the world is asking the truth, while asking the flag is asking
/// somebody's intention.
fn there_is_a_window(windows: Query<(), With<PrimaryWindow>>) -> bool {
    !windows.is_empty()
}

fn paused(screen: Res<Screen>) -> bool {
    *screen == Screen::Paused
}

/// `Esc` — the one key this domain knows.
///
/// ⚠️ `src/net/local.rs` calls itself *"the only place in the game that knows what a key
/// is"*, and that stays true of every **gameplay** binding: `Esc` produces no `Intent`, it
/// never reaches the simulation, and it does not travel over a wire. Pausing is a thing this
/// screen does to this machine. Once `F-172` moves the bindings into a file, this one goes
/// with them.
fn toggle_pause(keys: Res<ButtonInput<KeyCode>>, mut screen: ResMut<Screen>) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    *screen = match *screen {
        Screen::Playing => Screen::Paused,
        Screen::Paused => Screen::Playing,
    };
}

/// Makes the world match [`Screen`]: the pointer, and whether time runs.
///
/// Compares before it writes. Not to save the cycles — there is one window — but because
/// `bevy_winit` reacts to `Changed<CursorOptions>`
/// (`bevy_winit-0.19.0/src/system.rs:609-612`) and would otherwise re-issue a grab to the
/// compositor **every frame** for a value that never changed (§6 rule 6).
fn apply_screen(
    screen: Res<Screen>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
) {
    let captured = *screen == Screen::Playing;
    let want = if captured {
        // `Locked` and not `Confined`: mouse-look wants relative motion with no edge to run
        // into. X11 does not support it and **Bevy falls back to `Confined` by itself**
        // (`bevy_window-0.19.0/src/window.rs:768-771`) — so this line is right on both, and
        // asking here which display server we are on would only be a second, staler answer.
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };

    for mut cursor in &mut cursors {
        if cursor.grab_mode != want {
            cursor.grab_mode = want;
        }
        if cursor.visible == captured {
            cursor.visible = !captured;
        }
    }

    // One switch for the whole simulation instead of a `paused` check in every domain's
    // systems: `run_fixed_main_schedule` feeds on `Time<Virtual>::delta()`
    // (`bevy_time-0.19.0/src/fixed.rs:244-247`), so a paused virtual clock means no fixed
    // step happens — and nothing that anybody writes later can forget to honour it.
    if captured == time.is_paused() {
        if captured {
            time.unpause();
        } else {
            time.pause();
        }
    }
}

/// Playing, or looking at the pause screen. There is no third state today.
///
/// A `Resource` and not a component: this is the state of *this session's screen*, not of a
/// player. §6 rule 3 forbids putting **player** state in a resource — and it forbids it for a
/// reason that does not apply here: a second player does not get a second pause screen, he
/// gets his own `Intent`. ⚠️ The day this game is played over a network, pausing stops being
/// a thing one machine may do to the simulation — see the note on [`Screen::Paused`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Playing,
    /// ⚠️ **Local-only.** Pausing works by stopping `Time<Virtual>`, i.e. the whole
    /// simulation. Over a network that is not available to a client, and the pause screen
    /// there becomes an overlay that does not stop anything. Written down here rather than
    /// discovered later.
    Paused,
}

/// On **every** node this domain spawns, containers included — the whole overlay is despawned
/// by this marker.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PauseElement;

/// What a button on the pause screen does. Two of them, and that is the whole screen.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseAction {
    Resume,
    Quit,
}
