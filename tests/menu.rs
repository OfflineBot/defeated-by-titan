//! The guard over the **pointer**: it is captured, and it can be got back.
//!
//! `P4` in `docs/PLAN-GAME.md` §8, plus the pause screen half of `F-175`. Two claims:
//!
//! 1. **`Esc` gives the pointer back.** This test comes first in the file and was written
//!    first, on purpose: a locked pointer with no release key is a game you have to `pkill`.
//! 2. **The pointer is only captured when there is a window.** Every run on this machine is
//!    `--headless` or `--offscreen`; a grab in one of those would be a grab on a pointer that
//!    is not there, in a run nobody is watching.
//!
//! ## Why these tests spawn a window entity by hand
//!
//! `src/lib.rs:207-225` builds `primary_window: None` whenever `Cli::wants_window()` is false —
//! so a headless app has **no window entity at all**, and there is nothing whose
//! `CursorOptions` could be looked at. The entity spawned here is a plain ECS entity: winit is
//! disabled in this mode (`src/lib.rs:236-240`), so nothing opens on screen. What is under
//! test is the game's decision — *"lock it / let it go"* — not the compositor's execution of
//! it, and this file says so rather than pretending otherwise.
//!
//! **Nobody has ever run this game in a window on this machine.** The visual half of `P4` —
//! seeing a pointer disappear — is not proven here and is not claimed.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use defeated_by_titan::menu::{PauseAction, PauseElement, Screen};
use defeated_by_titan::shared::Cli;

/// The real app, headless, plus **one window entity** — the state a windowed run is in as far
/// as the ECS is concerned.
fn app_with_window() -> (App, Entity) {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // `Window` requires `CursorOptions` (`bevy_window-0.19.0/src/window.rs:163`), so the
    // component arrives at its default — `grab_mode: None`, `visible: true` (`:782-789`).
    let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
    app.update();
    (app, window)
}

fn cursor(app: &App, window: Entity) -> CursorOptions {
    app.world()
        .get::<CursorOptions>(window)
        .expect("a Window entity always carries CursorOptions")
        .clone()
}

/// Presses a pause-screen button.
///
/// `Update` is run **directly** instead of a whole frame, and that is not a shortcut: Bevy's
/// `ui_focus_system` runs in `PreUpdate` and resets every `Interaction` it does not find a
/// pointer over (`bevy_ui-0.19.0/src/focus.rs:165-171`, registered at `lib.rs:175`). A real
/// click is *set* by that system and read in the same frame's `Update`; an `Interaction`
/// written from a test between two frames would be wiped before anything saw it. There is no
/// pointer in a headless run to hover with, so the frame is entered where a real click
/// leaves it.
fn click(app: &mut App, button: Entity) {
    *app.world_mut().get_mut::<Interaction>(button).expect("buttons are interactive") =
        Interaction::Pressed;
    app.world_mut().run_schedule(Update);
}

/// One `Esc`, pressed and released again, with a frame for each.
///
/// The release matters: `just_pressed` is what the toggle reads, and a key that is never
/// released would make the second press invisible.
fn press_esc(app: &mut App, window: Entity) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

/// ★ **The way out.** Written and run before the capture existed.
///
/// Goes red the moment `Esc` is not wired: the pointer stays locked and invisible, and the
/// only way out of the game is another terminal.
#[test]
fn p4_esc_releases_the_pointer() {
    let (mut app, window) = app_with_window();

    // Locked **by hand**, not by the capture system. This test has to be able to fail for one
    // reason only — that there is no way out — and that means it must not depend on the
    // feature it is the safety net for. It was written and run before the capture existed.
    {
        let mut c = app.world_mut().get_mut::<CursorOptions>(window).expect("cursor options");
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
    }

    press_esc(&mut app, window);

    let c = cursor(&app, window);
    assert_eq!(c.grab_mode, CursorGrabMode::None, "Esc has to give the pointer back");
    assert!(c.visible, "and it has to be visible again — an invisible free pointer is worse");
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
}

/// And back in again: the same key returns the pointer to the game.
#[test]
fn p4_esc_twice_gives_the_pointer_back_to_the_game() {
    let (mut app, window) = app_with_window();
    press_esc(&mut app, window);
    press_esc(&mut app, window);

    let c = cursor(&app, window);
    assert_eq!(c.grab_mode, CursorGrabMode::Locked);
    assert!(!c.visible);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
}

/// The capture itself — and that it happens **because there is a window**.
#[test]
fn p4_the_pointer_is_captured_when_there_is_a_window() {
    let (app, window) = app_with_window();
    let c = cursor(&app, window);
    assert_eq!(
        c.grab_mode,
        CursorGrabMode::Locked,
        "a mouse-look game that leaves the pointer free lets you turn until the screen edge"
    );
    assert!(!c.visible, "a system cursor in the middle of the crosshair is nobody's design");
}

/// …and **not** without one. Every run on this machine is one of these two.
#[test]
fn p4_a_run_without_a_window_grabs_nothing() {
    for start in [
        Cli { headless: true, ..default() },
        // `--offscreen` is the mode that renders without a window. It is not built here
        // (that would ask this machine for a Vulkan adapter inside a unit test); what is
        // checked is the flag that decides, and `src/lib.rs:207` reads exactly this one.
        Cli { offscreen: true, ..default() },
    ] {
        assert!(!start.wants_window(), "{start:?} must not want a window");
    }

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.update();
    app.update();

    let mut windows = app.world_mut().query::<&Window>();
    assert_eq!(
        windows.iter(app.world()).count(),
        0,
        "a headless run has no window entity — so there is nothing to grab, and nothing did"
    );
    let mut grabbed = app.world_mut().query::<&CursorOptions>();
    assert!(
        grabbed
            .iter(app.world())
            .all(|c| c.grab_mode == CursorGrabMode::None),
        "nothing in a windowless run may be captured"
    );
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
}

/// The pause screen exists, has exactly the two ways on the plan, and is **gone** again
/// afterwards — a paused overlay that survives Resume covers the game.
#[test]
fn f175_the_pause_screen_offers_resume_and_quit() {
    let (mut app, window) = app_with_window();

    let mut overlay = app.world_mut().query::<&PauseElement>();
    assert_eq!(overlay.iter(app.world()).count(), 0, "nothing is on screen while playing");

    press_esc(&mut app, window);

    let mut actions = app.world_mut().query::<&PauseAction>();
    let mut on_screen: Vec<PauseAction> = actions.iter(app.world()).copied().collect();
    on_screen.sort_by_key(|a| format!("{a:?}"));
    assert_eq!(
        on_screen,
        vec![PauseAction::Quit, PauseAction::Resume],
        "two ways out of a pause screen, and F-175 says at most two clicks to either"
    );

    press_esc(&mut app, window);
    let mut overlay = app.world_mut().query::<&PauseElement>();
    assert_eq!(overlay.iter(app.world()).count(), 0, "Resume has to clear the screen again");
}

/// Resume is a button as well as a key — because a key is the only one of the two anybody in
/// this environment can press, and a button is the only one a player expects.
#[test]
fn f175_the_resume_button_does_what_the_key_does() {
    let (mut app, window) = app_with_window();
    press_esc(&mut app, window);

    let resume = {
        let mut q = app.world_mut().query::<(Entity, &PauseAction)>();
        q.iter(app.world())
            .find(|(_, a)| **a == PauseAction::Resume)
            .map(|(e, _)| e)
            .expect("a Resume button")
    };
    click(&mut app, resume);

    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::Locked);
}

/// Quit ends the run. Not `despawn`, not a flag somebody polls: `AppExit`.
#[test]
fn f175_the_quit_button_ends_the_run() {
    let (mut app, window) = app_with_window();
    press_esc(&mut app, window);

    let quit = {
        let mut q = app.world_mut().query::<(Entity, &PauseAction)>();
        q.iter(app.world())
            .find(|(_, a)| **a == PauseAction::Quit)
            .map(|(e, _)| e)
            .expect("a Quit button")
    };
    assert!(app.should_exit().is_none(), "nothing has asked to exit yet");

    click(&mut app, quit);

    assert!(app.should_exit().is_some(), "Quit has to write AppExit");
}

/// Pausing stops the simulation, and Resume starts it again.
///
/// Through `Time<Virtual>`, which is what feeds `run_fixed_main_schedule`
/// (`bevy_time-0.19.0/src/fixed.rs:244-247`) — so it is **one** switch for every domain
/// instead of a `Paused` check somebody has to remember to add to each new system.
#[test]
fn f175_a_paused_game_does_not_simulate() {
    use defeated_by_titan::shared::Tick;
    use std::time::Duration;

    let (mut app, window) = app_with_window();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    app.update();
    let running = app.world().resource::<Tick>().0;
    app.update();
    assert!(app.world().resource::<Tick>().0 > running, "unpaused, the tick has to move");

    press_esc(&mut app, window);
    let paused_at = app.world().resource::<Tick>().0;
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Tick>().0,
        paused_at,
        "a paused game must not simulate five more frames' worth of titan"
    );

    press_esc(&mut app, window);
    app.update();
    assert!(
        app.world().resource::<Tick>().0 > paused_at,
        "and it has to go on again afterwards"
    );
}
