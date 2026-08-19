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
use defeated_by_titan::menu::{plate, MenuRoot, PauseAction, PauseElement, Screen};
use defeated_by_titan::shared::Cli;

/// The real app, headless, plus **one window entity** — the state a windowed run is in as far
/// as the ECS is concerned.
fn app_with_window() -> (App, Entity) {
    windowed(Cli { headless: true, ..default() })
}

/// The same, for a run that was started differently — in the hub, or straight into a sortie.
fn windowed(start: Cli) -> (App, Entity) {
    let mut app = defeated_by_titan::app(start);
    // `Window` requires `CursorOptions` (`bevy_window-0.19.0/src/window.rs:163`), so the
    // component arrives at its default — `grab_mode: None`, `visible: true` (`:782-789`).
    let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
    app.update();
    (app, window)
}

/// The one button carrying this action. Panics with the action's name when it is not on
/// screen, because "no such button" is the failure this file exists to catch.
fn button<A: Component + PartialEq + std::fmt::Debug>(app: &mut App, want: &A) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &A)>();
    q.iter(app.world())
        .find(|(_, action)| *action == want)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no button for {want:?} is on screen"))
}

/// Presses the button carrying `want`. The plates rebuild on every change, so a button entity
/// from before a click is a dangling one — this looks it up again every time.
fn press<A: Component + PartialEq + std::fmt::Debug>(app: &mut App, want: &A) {
    let e = button(app, want);
    click(app, e);
}

/// Every line of text on the screen, in no particular order.
///
/// A plain query over `Text` and not a walk down the plate's children: what is being asserted
/// is "the player can read this number", and where in the tree it hangs is not part of that
/// claim.
fn plate_text(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
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

/// Four toggles in a row, checked after **every** one of them.
///
/// The two tests above check one flip each and would both still pass if the second `Esc` were
/// the last one that worked. Outside the process, on the real screen, four toggles were driven
/// with `ydotool` and the cursor came and went four times; this is the in-process twin of that
/// run — and the only one of the two that a `cargo test` can defend.
#[test]
fn p4_the_pointer_follows_the_screen_through_four_toggles() {
    let (mut app, window) = app_with_window();

    for round in 1..=4 {
        press_esc(&mut app, window);
        // Odd press = paused, even press = playing again.
        let (want_screen, want_grab) = if round % 2 == 1 {
            (Screen::Paused, CursorGrabMode::None)
        } else {
            (Screen::Playing, CursorGrabMode::Locked)
        };

        assert_eq!(
            *app.world().resource::<Screen>(),
            want_screen,
            "after Esc number {round} the screen has to be {want_screen:?}"
        );
        let c = cursor(&app, window);
        assert_eq!(c.grab_mode, want_grab, "Esc number {round}: the pointer follows the screen");
        assert_eq!(
            c.visible,
            want_screen == Screen::Paused,
            "Esc number {round}: a paused pointer is visible, a playing one is not"
        );
    }
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
    // ⚠️ **This list grew on 2026-08-13** — *„menu (also bei escape)"*. Until that morning it
    // was `[Quit, Resume]` and F-175 was 🟨 for exactly that reason. `Abandon` is **not** in it
    // here and that is the claim of the next test: this run has no sortie to abandon.
    assert_eq!(
        on_screen,
        vec![PauseAction::Lobby, PauseAction::Quit, PauseAction::Resume, PauseAction::Settings],
        "the ways out of a pause screen, and F-175 says at most two clicks to any of them"
    );

    press_esc(&mut app, window);
    let mut overlay = app.world_mut().query::<&PauseElement>();
    assert_eq!(overlay.iter(app.world()).count(), 0, "Resume has to clear the screen again");
}

/// The overlay is on screen for as long as the pause lasts — and it is there **once**.
///
/// `spawn_pause_screen` is self-healing and not message-driven: its condition is *"paused and
/// nothing on screen"* (`src/menu/pause.rs:27-30`). That is the right shape, and it is exactly
/// the shape that fails silently — a missing guard spawns a fresh overlay **every frame** and
/// nobody sees it, because the copies sit on top of each other and the screen looks correct.
/// So the assertion is not "something is on screen" but "the same number of things".
#[test]
fn f175_the_pause_screen_is_on_screen_once_and_stays_once() {
    let (mut app, window) = app_with_window();

    press_esc(&mut app, window);
    let after_one_pause = {
        let mut overlay = app.world_mut().query::<&PauseElement>();
        overlay.iter(app.world()).count()
    };
    assert!(after_one_pause > 0, "a paused game has to show its pause screen");

    // Five frames of standing still in the pause. Nothing has changed, so nothing may be spawned.
    for _ in 0..5 {
        app.update();
    }
    let mut overlay = app.world_mut().query::<&PauseElement>();
    assert_eq!(
        overlay.iter(app.world()).count(),
        after_one_pause,
        "holding the pause for five frames must not spawn a second overlay per frame"
    );

    // …and through a whole second cycle: Resume, pause again.
    press_esc(&mut app, window);
    press_esc(&mut app, window);
    let mut overlay = app.world_mut().query::<&PauseElement>();
    assert_eq!(
        overlay.iter(app.world()).count(),
        after_one_pause,
        "the second pause screen has to be the same size as the first one, not twice it"
    );
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
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

// ---------------------------------------------------------------------------
// The hub is not a screen (2026-08-12)
// ---------------------------------------------------------------------------

/// ★ **The decision this file exists to hold: the hub did not become a third `Screen`.**
///
/// When the hub was built (`mission::hub`), `Screen` was the obvious place for it — it is
/// `Playing | Paused` and a hub looks like a third mode. It is not. In the hub the pointer stays
/// locked, `Time<Virtual>` keeps running and the player **walks**: that is `Screen::Playing` in
/// every respect this domain can observe. A third variant would have made "the game is paused"
/// and "the player is in the hub" two answers to one question, and `apply_screen` would then
/// have had to decide which of them owns the cursor.
///
/// So the hub is a phase of `mission::MissionPhase` and this test is the guard: if somebody
/// widens `Screen` later, they have to come past this and say why.
#[test]
fn f072_the_hub_is_a_place_and_not_a_screen() {
    use defeated_by_titan::mission::MissionPhase;

    let mut app = defeated_by_titan::app(Cli { headless: true, hub: true, ..default() });
    let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "`--hub` did not put the session in the hub — the rest of this test would prove nothing"
    );
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "the hub arrived as a screen state. It is a place: you walk in it, and the mouse still \
         looks around"
    );

    // The two things a screen decides, and the hub changes neither of them.
    let c = cursor(&app, window);
    assert_eq!(c.grab_mode, CursorGrabMode::Locked, "the pointer was let go in the hub");
    assert!(!c.visible, "a system cursor appeared in the hub");
    assert!(
        !app.world().resource::<Time<Virtual>>().is_paused(),
        "time stopped in the hub — nobody could walk anywhere"
    );

    // And `Esc` still works there, which is the one thing a place must not take away.
    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::None, "Esc has to give the pointer back in the hub too");
}

// ---------------------------------------------------------------------------------------
// Settings, the lobby, and the front door (2026-08-13)
//
// > *„zudem fehlen settings. menu (also bei escape) und eine main lobby in der man die mission
// > starten kann"* — the user (`docs/NEXT.md` §1D, reqs 6-8).
//
// **Everything below needs a window entity**, and that is not a detail of the test setup: the
// whole domain is gated on `With<PrimaryWindow>` (`menu::there_is_a_window`), so a `--headless`
// run has no menu at all and can never be evidence for one. What a headless run *can* prove is
// where the game starts, and `f175_a_run_that_names_no_door_starts_in_the_hub` is that half.
// ---------------------------------------------------------------------------------------

use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::hud::HudElement;
use defeated_by_titan::menu::lobby::{LobbyAction, LobbyChoice};
use defeated_by_titan::menu::settings::{Nudge, SettingsAction};
use defeated_by_titan::mission::{MissionPhase, Sortie};
use defeated_by_titan::net::local::MouseSinceTick;
use defeated_by_titan::shared::{Intent, LocalPlayer, PlayerSettings};

/// A run that went straight into a fight, the way `--mission tutorial` does.
fn app_in_a_sortie() -> (App, Entity) {
    let (app, window) = windowed(Cli {
        headless: true,
        mission: Some("tutorial".to_string()),
        ..default()
    });
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Active,
        "--mission tutorial did not start a sortie — the rest of this test would prove nothing"
    );
    (app, window)
}

/// ★ **Abandon is the one button that depends on where you are.** Offering "give up" in the
/// hub is a button that cannot do anything; not offering it in a fight is a fight you cannot
/// leave.
#[test]
fn f175_abandon_is_only_offered_inside_a_sortie() {
    let (mut app, window) = app_in_a_sortie();
    press_esc(&mut app, window);
    let mut actions = app.world_mut().query::<&PauseAction>();
    let on_screen: Vec<PauseAction> = actions.iter(app.world()).copied().collect();
    assert!(
        on_screen.contains(&PauseAction::Abandon),
        "a running sortie has to offer a way out of it: {on_screen:?}"
    );

    let (mut hub, hub_window) = windowed(Cli { headless: true, hub: true, ..default() });
    press_esc(&mut hub, hub_window);
    let mut actions = hub.world_mut().query::<&PauseAction>();
    let in_the_hub: Vec<PauseAction> = actions.iter(hub.world()).copied().collect();
    assert!(
        !in_the_hub.contains(&PauseAction::Abandon),
        "there is nothing to abandon in the hub: {in_the_hub:?}"
    );
}

/// **Abandon ends the sortie and writes no verdict.** Not `Lost` — nobody lost, the run did
/// not happen.
#[test]
fn f175_abandon_puts_you_back_in_the_hub_without_a_verdict() {
    let (mut app, window) = app_in_a_sortie();
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Abandon);

    // Abandon hands the screen straight back to the game, so the clock runs again and
    // `mission::take_orders_from_the_menu` may act — see its header for why it refuses to act
    // on a stopped clock.
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "abandoning has to land in the hub"
    );
}

/// ★ **The pointer is free on every screen, not only on the pause screen.**
///
/// This is the guarantee `P4` bought and the one a second and third screen could quietly have
/// broken: `apply_screen` decides on `!= Screen::Playing`, so a screen added tomorrow cannot
/// forget to hand the mouse back — and this test is what says so.
#[test]
fn p4_the_pointer_is_free_on_the_settings_and_the_lobby_screen() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    press_esc(&mut app, window);

    for (open, expected) in [
        (PauseAction::Settings, Screen::Settings),
        (PauseAction::Lobby, Screen::Lobby),
    ] {
        // Back to the pause screen first — both doors start there.
        if *app.world().resource::<Screen>() != Screen::Paused {
            press_esc(&mut app, window);
        }
        press(&mut app, &open);
        app.update();
        assert_eq!(*app.world().resource::<Screen>(), expected);

        let c = cursor(&app, window);
        assert_eq!(c.grab_mode, CursorGrabMode::None, "{expected:?} took the pointer away");
        assert!(c.visible, "{expected:?} left the pointer invisible — nobody can click that");
        assert!(
            app.world().resource::<Time<Virtual>>().is_paused(),
            "{expected:?} let the simulation run underneath the cursor"
        );
    }

    // …and out again: `Esc` from the lobby is the game, and the pointer comes back to it.
    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::Locked);
}

/// **Every setting is seeded out of `game.ron`** — no number in `src/` invented one of them.
#[test]
fn f175_the_settings_are_seeded_out_of_the_file() {
    let app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let data = app.world().resource::<GameData>();
    let camera = data.game.camera.clone();
    let spread = data.game.vector.aim_spread_deg;
    let s = *app.world().resource::<PlayerSettings>();

    assert_eq!(s.mouse_deg_per_px, camera.mouse_deg_per_px);
    assert_eq!(s.fov_deg, camera.fov_deg);
    assert_eq!(s.pitch_limit_deg, camera.pitch_limit_deg);
    assert_eq!(s.aim_spread_deg, spread);
    assert!(!s.invert_y, "inverted has to be off until somebody asks for it");
    // `F-016`, and this is the guarantee that lets the two knobs ship before the scoring that
    // will read them: **0 % is exactly today's pure free aim**, so a fresh game aims as it did
    // yesterday. `F-002` says the free ray "stays ALWAYS active and is never replaceable by
    // the snap system", and until he moves one of these it is the whole answer.
    assert_eq!(s.assist_catch_pct, 0.0, "the assist may not be on before he turns it on");
    assert_eq!(s.assist_strength_pct, 0.0);
}

/// `F-016` — **the two knobs he asked for, live, with no restart, and readable back.**
///
/// > *„und seinstellen können wie weit ca es sein sollte und wie aggressive (damit ich testen
/// > kann was am besten wäre mach debug einstellungen dafür)"* — the user, 2026-08-18.
///
/// The three properties that make them worth anything to him: a click moves the number, the
/// number is on the screen so he can read it back, and it is still there when he leaves the
/// menu — `F-024`'s own acceptance, *"Moduswechsel ist ohne Neustart wirksam"*.
#[test]
fn f016_the_two_assist_knobs_are_live_and_readable() {
    use defeated_by_titan::shared::settings::{
        ASSIST_CATCH_MAX_DEG, ASSIST_MIN_PCT, ASSIST_STEP_PCT,
    };

    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Settings);
    app.update();

    // One click each, and the effect is immediate — no apply button, no restart.
    press(&mut app, &SettingsAction::AssistCatch(Nudge::Up));
    app.update();
    press(&mut app, &SettingsAction::AssistStrength(Nudge::Up));
    app.update();
    let s = *app.world().resource::<PlayerSettings>();
    assert_eq!(s.assist_catch_pct, ASSIST_MIN_PCT + ASSIST_STEP_PCT);
    assert_eq!(s.assist_strength_pct, ASSIST_MIN_PCT + ASSIST_STEP_PCT);

    // **He can read the number off the screen**, which is what makes "tell us what felt best"
    // possible at all. The value sits in the row, and the degrees it means sit under it.
    let shown = plate_text(&mut app);
    assert!(
        shown.iter().any(|t| t.contains("Aim assist reach")),
        "the reach row is not on the screen: {shown:?}"
    );
    assert!(
        shown.iter().any(|t| t.contains("Aim assist strength")),
        "the strength row is not on the screen: {shown:?}"
    );
    let wanted = format!("{:.0} %", s.assist_catch_pct);
    assert!(
        shown.iter().any(|t| *t == wanted),
        "the screen does not show the value {wanted:?} it just set: {shown:?}"
    );
    // The degrees are computed **here** and not read back out of `assist_catch_deg`: a test
    // that asks the screen and the function the same question in the same words passes even
    // when both are wrong together, which is exactly what happened when this line was first
    // written (rule 5, and it took breaking the function to notice).
    let expected_deg = ASSIST_STEP_PCT / 100.0 * ASSIST_CATCH_MAX_DEG;
    assert!(
        shown.iter().any(|t| t.contains(&format!("{expected_deg:.1} deg"))),
        "one notch is {expected_deg:.1} deg off the crosshair, and the row does not say so: \
         {shown:?}"
    );

    // And it survives leaving the menu — the setting is the player's, not the screen's.
    press_esc(&mut app, window);
    press_esc(&mut app, window);
    app.update();
    let after = *app.world().resource::<PlayerSettings>();
    assert_eq!(after.assist_catch_pct, s.assist_catch_pct, "the knob was lost on the way out");
    assert_eq!(after.assist_strength_pct, s.assist_strength_pct);
}

/// Drives one tick with a mouse push and reports the `Intent` the simulation would have read.
///
/// `FixedTimesteps(1)`: one `update()` is one tick, by construction — the same idiom
/// `tests/input.rs` uses, and for the same reason (`B-002`: a fixed step per frame is 0..n).
fn look_after_a_push(app: &mut App, dx: f32, dy: f32) -> (f32, f32) {
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app.world_mut().resource_mut::<MouseSinceTick>().delta = Vec2::new(dx, dy);
    app.update();
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let intent = players.iter(app.world()).next().expect("a local player after startup");
    (intent.yaw, intent.pitch)
}

/// ★ **A setting that does not take effect is not a setting.**
///
/// Half the sensitivity, half the turn — measured through the real `read_input`, not by
/// reading the resource back.
#[test]
fn f175_the_mouse_sensitivity_setting_changes_how_far_a_look_turns() {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let (fast_yaw, _) = look_after_a_push(&mut app, 100.0, 0.0);

    let mut slow = defeated_by_titan::app(Cli { headless: true, ..default() });
    {
        let mut s = slow.world_mut().resource_mut::<PlayerSettings>();
        s.mouse_deg_per_px /= 2.0;
    }
    let (slow_yaw, _) = look_after_a_push(&mut slow, 100.0, 0.0);

    println!("100 px: {fast_yaw} rad at the file's sensitivity, {slow_yaw} rad at half of it");
    assert!(fast_yaw.abs() > 0.0, "the mouse turned nothing at all — this test proves nothing");
    assert!(
        (slow_yaw - fast_yaw / 2.0).abs() < 1e-6,
        "half the sensitivity has to be half the turn: {fast_yaw} -> {slow_yaw}"
    );
}

/// **Invert Y flips the pitch and touches nothing else.** The yaw of an inverted player is the
/// yaw of everybody else's.
#[test]
fn f175_invert_y_flips_the_pitch_and_leaves_the_yaw_alone() {
    let mut plain = defeated_by_titan::app(Cli { headless: true, ..default() });
    let (yaw, pitch) = look_after_a_push(&mut plain, 40.0, 30.0);

    let mut inverted = defeated_by_titan::app(Cli { headless: true, ..default() });
    inverted.world_mut().resource_mut::<PlayerSettings>().invert_y = true;
    let (inverted_yaw, inverted_pitch) = look_after_a_push(&mut inverted, 40.0, 30.0);

    assert!(pitch.abs() > 0.0, "the mouse pitched nothing at all — this test proves nothing");
    assert!(
        (inverted_pitch + pitch).abs() < 1e-6,
        "inverted has to be the opposite pitch: {pitch} vs {inverted_pitch}"
    );
    assert!((inverted_yaw - yaw).abs() < 1e-6, "invert Y is not invert X: {yaw} vs {inverted_yaw}");
}

/// **The field of view reaches the camera** — `render` stays the one writer of `Projection`,
/// and it follows the setting within a frame.
#[test]
fn f175_the_field_of_view_setting_reaches_the_camera() {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.update();
    app.update();

    let fov_of = |app: &mut App| -> f32 {
        let mut q = app.world_mut().query_filtered::<&Projection, With<Camera3d>>();
        match q.iter(app.world()).next().expect("render::attach_camera builds one camera") {
            Projection::Perspective(p) => p.fov,
            other => panic!("the game camera stopped being perspective: {other:?}"),
        }
    };
    let seeded = fov_of(&mut app);
    let want = app.world().resource::<PlayerSettings>().fov_deg;
    assert!((seeded - want.to_radians()).abs() < 1e-6, "the camera was built off the settings");

    app.world_mut().resource_mut::<PlayerSettings>().nudge_fov(2.0);
    let asked = app.world().resource::<PlayerSettings>().fov_deg;
    app.update();
    let after = fov_of(&mut app);
    println!("fov {want} -> {asked} deg, camera {seeded} -> {after} rad");
    assert!(asked > want, "two steps up the slider have to move the number");
    assert!((after - asked.to_radians()).abs() < 1e-6, "the camera did not follow the setting");
}

/// ★★ **One field, two devices.** The wheel and the settings row write the *same*
/// `aim_spread_deg`, and the `Intent` carries whichever moved it last.
///
/// This is the test the brief asked for by name: `F-023` put the spread on the mouse wheel the
/// day before the settings screen existed, and a screen with a slider of its own would have
/// made two numbers out of one — turn the wheel, open the settings, and see the value the wheel
/// started from.
#[test]
fn f175_the_wheel_and_the_settings_row_write_one_aim_spread() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    let step = app.world().resource::<GameData>().game.vector.aim_spread_step_deg;
    let start = app.world().resource::<PlayerSettings>().aim_spread_deg;

    // One notch of the wheel, through the real input path.
    app.world_mut().resource_mut::<MouseSinceTick>().scroll_notches = 1.0;
    app.update();
    let after_wheel = app.world().resource::<PlayerSettings>().aim_spread_deg;
    assert!(
        (after_wheel - (start + step)).abs() < 1e-4,
        "the wheel no longer writes the settings: {start} -> {after_wheel}"
    );

    // …and the screen sees exactly that number and moves it on from there.
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Settings);
    app.update();
    press(&mut app, &SettingsAction::Spread(Nudge::Up));
    let after_click = app.world().resource::<PlayerSettings>().aim_spread_deg;
    assert!(
        (after_click - (after_wheel + step)).abs() < 1e-4,
        "the row started from its own copy instead of the wheel's value: \
         {after_wheel} -> {after_click}"
    );

    // And what the simulation is handed is that same number — the one thing that travels.
    press_esc(&mut app, window); // back to the pause screen
    press_esc(&mut app, window); // and into the game, so the clock runs again
    app.update();
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let shipped = players.iter(app.world()).next().expect("a local player").aim_spread_deg;
    assert!(
        (shipped - after_click).abs() < 1e-4,
        "the Intent carried {shipped} while the setting said {after_click}"
    );
}

/// **The lobby is `missions.ron`, drawn.** One button per template, one per difficulty of the
/// chosen one — and no list in `src/menu/lobby.rs`.
#[test]
fn f175_the_lobby_lists_what_the_file_says() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Lobby);
    app.update();

    let missions = app.world().resource::<GameData>().missions.clone();
    let mut on_screen: Vec<String> = {
        let mut q = app.world_mut().query::<&LobbyAction>();
        q.iter(app.world())
            .filter_map(|a| match a {
                LobbyAction::PickMission(key) => Some(key.clone()),
                _ => None,
            })
            .collect()
    };
    on_screen.sort();
    let mut expected: Vec<String> = missions.templates.keys().cloned().collect();
    expected.sort();
    assert_eq!(on_screen, expected, "the lobby has to offer every mission in the file");

    // The default choice is the hub's first pad — `missions.ron` calls it the door you find
    // without looking for it — and its difficulties are the ones on screen.
    let pad = missions.hub.deployments.first().expect("the hub has a deployment pad");
    let mut levels: Vec<String> = {
        let mut q = app.world_mut().query::<&LobbyAction>();
        q.iter(app.world())
            .filter_map(|a| match a {
                LobbyAction::PickDifficulty(key) => Some(key.clone()),
                _ => None,
            })
            .collect()
    };
    levels.sort();
    let mut want: Vec<String> =
        missions.templates[&pad.mission].difficulties.keys().cloned().collect();
    want.sort();
    assert_eq!(levels, want, "the difficulty row has to be that mission's own levels");
}

/// ★★ **The claim the whole screen exists for: the lobby starts a sortie.**
///
/// Through the same door a deployment pad uses — `mission::take_orders_from_the_menu` sets
/// `Sortie` and the phase, and the lobby only asked. No `--mission` on the command line, no
/// walking onto a circle.
#[test]
fn f175_the_lobby_deploys_the_sortie_it_shows() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Lobby);
    app.update();

    // Pick a level that is **not** the default one, so the assert below cannot pass on a
    // screen that ignored the click.
    let (picked, fallback) = {
        let missions = &app.world().resource::<GameData>().missions;
        let pad = missions.hub.deployments.first().expect("a deployment pad").clone();
        let levels = &missions.templates[&pad.mission].difficulties;
        (
            levels.keys().last().cloned().expect("the hub's mission has difficulty levels"),
            pad.difficulty.clone(),
        )
    };
    assert_ne!(picked, fallback, "the test has to pick something the default is not");
    press(&mut app, &LobbyAction::PickDifficulty(picked.clone()));
    app.update();
    assert_eq!(
        app.world().resource::<LobbyChoice>().difficulty.as_deref(),
        Some(picked.as_str())
    );

    press(&mut app, &LobbyAction::Deploy);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "Deploy has to hand the screen back — the sortie is not flown with a cursor on it"
    );
    for _ in 0..6 {
        app.update();
    }

    let order = app.world().resource::<Sortie>().0.clone().expect("the lobby set the order");
    assert_eq!(order.difficulty.as_deref(), Some(picked.as_str()));
    assert!(order.from_hub, "a sortie out of the front door has to find its way back to it");
    let phase = *app.world().resource::<State<MissionPhase>>().get();
    assert!(
        phase.is_running(),
        "the lobby did not start the sortie — the phase is {phase:?}"
    );
}

/// ★ **The front door, and it is the flagless one.**
///
/// `--hub` was opt-in for one day (`docs/FINDINGS.md` FIND-057 §5). A run that names no other
/// door now starts where the missions are chosen — and `--headless` is not a door, it is how
/// this machine runs anything at all.
#[test]
fn f175_a_run_that_names_no_door_starts_in_the_hub() {
    let start = Cli::from_args(["--headless".to_string()]);
    assert!(start.hub, "--headless alone has to leave the front door in place");

    let mut app = defeated_by_titan::app(start);
    app.update();
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "a flagless start has to land in the hub"
    );
}

// ---------------------------------------------------------------------------
// The screens read the file in the file's order (FIND-092 §4)
// ---------------------------------------------------------------------------

/// Every component of type `A` under the menu root that is on screen, **in the order the
/// screen spawned them** — depth-first, children left to right.
///
/// A plain `Query` cannot answer this: iteration order is the archetype's, not the spawn
/// order, and "which button is leftmost" is exactly what FIND-092 measured wrong. So this
/// walks `Children`, which *is* the layout order.
fn in_screen_order<A: Component + Clone>(app: &mut App) -> Vec<A> {
    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<MenuRoot>>();
        q.iter(app.world()).next().expect("a menu plate has to be on screen")
    };
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(e) = pending.pop() {
        if let Some(a) = app.world().get::<A>(e) {
            found.push(a.clone());
        }
        if let Some(kids) = app.world().get::<Children>(e) {
            // Pushed in reverse so the stack pops them left to right.
            let mut kids: Vec<Entity> = kids.iter().collect();
            kids.reverse();
            pending.extend(kids);
        }
    }
    found
}

/// ★ **The easiest difficulty is the first button, because the file says so.**
///
/// The lobby showed `Elite | Recruit | Veteran` — the hardest level first, the easiest one in
/// the middle — and the mission row put the tutorial `First Ride` behind `Ashgate Skirmish`
/// (FIND-092 §4, `docs/images/f175-lobby.png`). The cause was the container, not the screen:
/// a `BTreeMap` sorts by key and `missions.ron`'s deliberate order never reached the UI.
///
/// This asserts the **rendered** order and not the map's, so it stays red if somebody fixes
/// the container and the screen re-sorts anyway.
#[test]
fn f175_the_lobby_offers_the_difficulties_in_the_order_the_file_lists_them() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Lobby);
    app.update();

    let file = app.world().resource::<GameData>().missions.clone();
    let pad = file.hub.deployments.first().expect("the hub has a deployment pad").clone();

    let shown: Vec<String> = in_screen_order::<LobbyAction>(&mut app)
        .into_iter()
        .filter_map(|a| match a {
            LobbyAction::PickDifficulty(key) => Some(key),
            _ => None,
        })
        .collect();
    let wanted: Vec<String> =
        file.templates[&pad.mission].difficulties.keys().cloned().collect();
    println!("difficulty row: {shown:?}");
    assert_eq!(shown, wanted, "the difficulty row has to run in the file's order");
    assert_eq!(
        shown.first().map(String::as_str),
        Some("recruit"),
        "and the file's first level is the easiest one — that is the point of the ordering"
    );

    let missions: Vec<String> = in_screen_order::<LobbyAction>(&mut app)
        .into_iter()
        .filter_map(|a| match a {
            LobbyAction::PickMission(key) => Some(key),
            _ => None,
        })
        .collect();
    let wanted: Vec<String> = file.templates.keys().cloned().collect();
    println!("mission row: {missions:?}");
    assert_eq!(missions, wanted, "the mission row has to run in the file's order");
    assert_eq!(
        missions.first().map(String::as_str),
        Some("tutorial"),
        "the tutorial is written first and belongs first"
    );
}

// ---------------------------------------------------------------------------
// The HUD belongs to the game, not to the menu (FIND-092 §4)
// ---------------------------------------------------------------------------

/// The top node of every HUD element — the ones the whole overlay hangs off.
fn hud_roots(app: &mut App) -> Vec<Entity> {
    let mut q =
        app.world_mut().query_filtered::<Entity, (With<HudElement>, Without<ChildOf>)>();
    let mut roots: Vec<Entity> = q.iter(app.world()).collect();
    roots.sort();
    roots
}

/// What every HUD node is currently **drawing** — its `Node.display`, which is how
/// `F-170`/`F-171` decide whether an element is on screen at all. The pair of claims below
/// hangs on this being untouched by the menu.
fn hud_drawing(app: &mut App) -> Vec<(Entity, Display)> {
    let mut q = app.world_mut().query_filtered::<(Entity, &Node), With<HudElement>>();
    let mut all: Vec<(Entity, Display)> =
        q.iter(app.world()).map(|(e, node)| (e, node.display)).collect();
    all.sort_by_key(|(e, _)| *e);
    all
}

fn visibility(app: &App, e: Entity) -> Visibility {
    *app.world().get::<Visibility>(e).expect("a Node always carries Visibility")
}

/// ★ **The crosshair does not run down the middle of the pause screen.**
///
/// Measured on all three shipped screens: ~8 600–9 200 cyan HUD px over the plate, the amber
/// objective counter, the gas bar, the blade pips, and the crosshair on rows 540–899 straight
/// through the menu column (FIND-092 §4, `docs/images/f175-pause.png`). Nothing hid the HUD
/// when `Screen != Playing` — freezing `Time<Virtual>` stops the simulation, not the drawing.
///
/// ⚠️ The second half is the one that guards the two 🟧 rows: `F-170` and `F-171` are pixel-
/// exact claims about what the HUD draws **while playing**, and a fix that hid the HUD would
/// be worthless if it also changed that. So the `Node.display` of every HUD node is snapshotted
/// while playing and compared again after a full pause-and-resume.
#[test]
fn f175_the_hud_is_hidden_while_a_menu_is_up() {
    let (mut app, window) = app_with_window();
    let roots = hud_roots(&mut app);
    assert!(
        roots.len() >= 5,
        "the HUD has to be on screen for this test to prove anything — found {} roots",
        roots.len()
    );
    // The HUD settles over the first few frames — an element whose producer only exists after
    // startup flips `Display::None -> Flex` once. The baseline is taken **after** that and is
    // then checked for being a baseline at all: comparing against a value that moves on its
    // own would prove nothing about the menu.
    for _ in 0..8 {
        app.update();
    }
    let while_playing = hud_drawing(&mut app);
    app.update();
    assert_eq!(
        hud_drawing(&mut app),
        while_playing,
        "the HUD has to be settled before it can be used as a baseline"
    );
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
    for &e in &roots {
        assert_ne!(visibility(&app, e), Visibility::Hidden, "the HUD is the game's, {e} hid it");
    }

    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
    for &e in &roots {
        assert_eq!(
            visibility(&app, e),
            Visibility::Hidden,
            "{e} is still drawing over the pause screen"
        );
    }
    assert_eq!(
        hud_drawing(&mut app),
        while_playing,
        "hiding the HUD must not change WHICH elements the HUD would draw (F-170, F-171)"
    );

    // The lobby is a screen too, and the rule is one line rather than a list of screens.
    press(&mut app, &PauseAction::Lobby);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Lobby);
    for &e in &roots {
        assert_eq!(visibility(&app, e), Visibility::Hidden, "{e} draws over the lobby");
    }

    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing);
    for &e in &roots {
        assert_ne!(visibility(&app, e), Visibility::Hidden, "{e} never came back");
    }
    assert_eq!(
        hud_drawing(&mut app),
        while_playing,
        "a pause and a resume have to leave the HUD exactly as they found it"
    );
}

// ---------------------------------------------------------------------------
// The settings screen is one grid, and its hint tells the truth (FIND-092 §4)
// ---------------------------------------------------------------------------

/// The direct children of the menu plate that is on screen, in layout order.
fn plate_children(app: &mut App) -> Vec<Entity> {
    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<MenuRoot>>();
        q.iter(app.world()).next().expect("a menu plate has to be on screen")
    };
    app.world().get::<Children>(root).map(|kids| kids.iter().collect()).unwrap_or_default()
}

/// Every laid-out row of the screen: its width in px, and the widths it is made of.
///
/// A row is a child of the plate with more than one child of its own — the four settings rows
/// and nothing else on this screen (the title, the hint lines and the Back button all have one
/// child or none). The width is the arithmetic the layout does: the children's own widths plus
/// the gap between them.
fn settings_rows(app: &mut App) -> Vec<(f32, Vec<f32>)> {
    let mut rows = Vec::new();
    for parent in plate_children(app) {
        let kids: Vec<Entity> = match app.world().get::<Children>(parent) {
            Some(kids) if kids.len() > 1 => kids.iter().collect(),
            _ => continue,
        };
        let node = app.world().get::<Node>(parent).expect("a row is a node");
        let gap = match node.column_gap {
            Val::Px(px) => px,
            other => panic!("a settings row's column_gap is {other:?} and not px"),
        };
        let widths: Vec<f32> = kids
            .iter()
            .map(|&k| match app.world().get::<Node>(k).expect("a row's child is a node").width {
                Val::Px(px) => px,
                other => panic!("a settings row's child is {other:?} wide and not px"),
            })
            .collect();
        let total = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
        rows.push((total, widths));
    }
    rows
}

/// The hint line under the `Invert Y` row — the child of the plate that follows the row with
/// the toggle in it.
fn invert_hint(app: &mut App) -> String {
    let children = plate_children(app);
    let toggle_row = children
        .iter()
        .position(|&e| {
            app.world().get::<Children>(e).is_some_and(|kids| {
                kids.iter().any(|k| {
                    app.world().get::<SettingsAction>(k) == Some(&SettingsAction::InvertY)
                })
            })
        })
        .expect("the Invert Y row has to be on the settings screen");
    let note = children[toggle_row + 1];
    app.world().get::<Text>(note).expect("a hint is a text line").0.clone()
}

fn open_settings(app: &mut App, window: Entity) {
    press_esc(app, window);
    press(app, &PauseAction::Settings);
    app.update();
}

/// ★ **Four rows, one grid.**
///
/// The `Invert Y` row measured 406 px against the other three at 452, and because every row is
/// centred on its own that pushed its label 24 px to the right and left its 208 px toggle
/// lining up with neither the `-` column nor the `+` column (FIND-092 §4,
/// `docs/images/f175-settings.png`). Four rows, no shared grid.
#[test]
fn f175_every_settings_row_is_the_same_width() {
    let (mut app, window) = app_with_window();
    open_settings(&mut app, window);

    let rows = settings_rows(&mut app);
    println!("settings rows: {rows:?}");
    assert_eq!(rows.len(), 6, "the settings screen has six adjustable rows");
    let (first, _) = rows[0];
    for (total, widths) in &rows {
        assert!(
            (total - first).abs() < 0.01,
            "every row has to be as wide as the others: {total} px against {first} px, {widths:?}"
        );
    }

    // And the toggle spans exactly the `-` value `+` block, so its edges sit on the same two
    // columns the other rows' arrows do.
    let (_, spread) = &rows[0];
    let arrows_and_value: f32 = spread[1..].iter().sum::<f32>() + 8.0 * 2.0;
    let (_, toggle) = &rows[1];
    assert_eq!(
        toggle.len(),
        2,
        "the Invert Y row is a label and one toggle: {toggle:?}"
    );
    assert!(
        (toggle[1] - arrows_and_value).abs() < 0.01,
        "the toggle has to span the whole `- value +` block: {} px against {arrows_and_value} px",
        toggle[1]
    );
}

/// ★ **A hint that describes the state you are not in is worse than no hint.**
///
/// `Invert Y: off` was captioned *"mouse forward looks down"* — the behaviour of the setting
/// when it is **on** (FIND-092 §4). Pushing the mouse forward is `d.y < 0`, and with
/// `invert_y` off `net::local::read_input` raises the pitch, i.e. looks **up**.
#[test]
fn f175_the_invert_y_hint_says_what_the_setting_currently_does() {
    let (mut app, window) = app_with_window();
    open_settings(&mut app, window);

    assert!(!app.world().resource::<PlayerSettings>().invert_y, "the file's default is off");
    let off = invert_hint(&mut app);
    println!("invert_y off: {off:?}");
    assert!(
        off.contains("up"),
        "with invert off, a mouse pushed forward looks UP — the hint says {off:?}"
    );

    press(&mut app, &SettingsAction::InvertY);
    app.update();
    assert!(app.world().resource::<PlayerSettings>().invert_y, "the click has to have landed");
    let on = invert_hint(&mut app);
    println!("invert_y on: {on:?}");
    assert!(
        on.contains("down"),
        "with invert on, a mouse pushed forward looks DOWN — the hint says {on:?}"
    );
    assert_ne!(off, on, "a hint that does not follow the value is a label, not a hint");
}

// ---------------------------------------------------------------------------
// The plate is legible on any frame, not only on the two that were photographed
// ---------------------------------------------------------------------------

/// One sRGB channel, decoded to linear light.
fn linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance of a colour, ignoring its alpha.
fn luminance(color: Color) -> f32 {
    let c = color.to_srgba();
    0.2126 * linear(c.red) + 0.7152 * linear(c.green) + 0.0722 * linear(c.blue)
}

/// WCAG contrast between two luminances.
fn contrast(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

/// What the backdrop leaves of a world of luminance `world`.
///
/// **Composited in linear light**, which is not an assumption: FIND-092 §3 measured the
/// backdrop turning a grey of 166 into 92 at `a = 0.72`, and this arithmetic reproduces that
/// to the integer. Luminance is linear in the linear channels, so compositing the luminance is
/// the same as compositing the three channels and then weighing them.
fn behind_the_menu(world: f32) -> f32 {
    let a = plate::BACKDROP.to_srgba().alpha;
    a * luminance(plate::BACKDROP) + (1.0 - a) * world
}

/// ★ **The button has to be findable on a frame nobody has photographed yet.**
///
/// Measured on the two that were: the plate is `1.10:1` against the night city of a sortie and
/// `2.44:1` against the daylight hub (FIND-092 §4) — both under WCAG 1.4.11's 3:1 for a user
/// interface component. It was legible in those two frames and would not have been on a bright
/// one, which is a defect that ships and then bites.
///
/// **A plate colour alone cannot fix it, and that is arithmetic rather than taste.** The
/// requirement pulls in two directions at once: against a dark frame the plate has to be
/// *lighter*, against a bright one *darker*, and the near-white label on it needs 4.5:1 of its
/// own, which caps the plate at a luminance of 0.148. Solve the three together and the backdrop
/// has to be almost opaque before any single plate colour satisfies them — which would throw
/// away the property the backdrop was built for (*"the game behind it stays readable, so a
/// screenshot of a paused frame still shows what was paused"*).
///
/// So the component is identified by its **edge**, which is what 1.4.11 asks for, and the
/// backdrop is deepened until that edge clears 3:1 against **any** world at all — a black frame
/// and a white one, not merely the two we happen to own.
#[test]
fn f175_the_menu_plate_is_legible_on_any_frame() {
    let plate_l = luminance(plate::PLATE);
    let chosen_l = luminance(plate::PLATE_CHOSEN);
    let edge_l = luminance(plate::PLATE_EDGE);
    let ink_l = luminance(plate::INK);
    let dim_l = luminance(plate::INK_DIM);

    // The two worlds that were photographed, back-computed from the shipped images
    // (FIND-092 §3/§4): the night city of a sortie and the daylight hub.
    let sortie = behind_the_menu(0.021016);
    let hub = behind_the_menu(0.381264);
    // And the two that bound every frame there can ever be.
    let darkest = behind_the_menu(0.0);
    let brightest = behind_the_menu(1.0);

    println!(
        "backdrop alpha {:.2} · plate vs background: sortie {:.2}:1, hub {:.2}:1",
        plate::BACKDROP.to_srgba().alpha,
        contrast(plate_l, sortie),
        contrast(plate_l, hub)
    );
    println!(
        "edge vs background: sortie {:.2}:1, hub {:.2}:1, black frame {:.2}:1, white frame \
         {:.2}:1 · edge vs plate {:.2}:1, vs chosen plate {:.2}:1",
        contrast(edge_l, sortie),
        contrast(edge_l, hub),
        contrast(edge_l, darkest),
        contrast(edge_l, brightest),
        contrast(edge_l, plate_l),
        contrast(edge_l, chosen_l)
    );
    println!(
        "ink vs plate {:.2}:1 · dim ink vs plate {:.2}:1",
        contrast(ink_l, plate_l),
        contrast(dim_l, plate_l)
    );

    // 1.4.11, the component: its boundary against **both** neighbours, on any frame.
    for (what, background) in
        [("a black frame", darkest), ("a white frame", brightest), ("the hub", hub), ("a sortie", sortie)]
    {
        let ratio = contrast(edge_l, background);
        assert!(
            ratio >= 3.0,
            "the button's edge is {ratio:.2}:1 against {what} — WCAG 1.4.11 wants 3:1"
        );
    }
    for (what, fill) in [("the plate", plate_l), ("the chosen plate", chosen_l)] {
        let ratio = contrast(edge_l, fill);
        assert!(ratio >= 3.0, "the button's edge is {ratio:.2}:1 against {what}, wanted 3:1");
    }
    assert!(plate::EDGE_PX >= 2.0, "a one-pixel edge disappears on a scaled display");

    // 1.4.3, the label: the thing this fix must not trade away.
    assert!(
        contrast(ink_l, plate_l) >= 4.5,
        "the label is {:.2}:1 on its plate — WCAG AA wants 4.5:1 for text this size",
        contrast(ink_l, plate_l)
    );
    assert!(
        contrast(dim_l, plate_l) >= 4.5,
        "the hint line is {:.2}:1 on the plate",
        contrast(dim_l, plate_l)
    );
    // And the backdrop still has to leave the paused world visible, which is what it is for.
    assert!(
        behind_the_menu(0.381264) > 0.005,
        "the backdrop has gone opaque — a paused screenshot no longer shows what was paused"
    );
}
