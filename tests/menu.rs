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
use bevy::camera::RenderTarget;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiStack;
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
use defeated_by_titan::hud::catch_band::{CatchTick, END_H_PX};
use defeated_by_titan::hud::{HudElement, ShowWhileTuning};
use defeated_by_titan::menu::board::Board;
use defeated_by_titan::menu::lobby::{chosen, entries, LobbyAction, LobbyChoice};
use defeated_by_titan::menu::settings::{Nudge, SettingsAction};
use defeated_by_titan::mission::{MissionPhase, Sortie};
use defeated_by_titan::net::local::MouseSinceTick;
use defeated_by_titan::shared::{Intent, LocalPlayer, LookOverride, PlayerSettings};

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
    let s = *app.world().resource::<PlayerSettings>();

    assert_eq!(s.mouse_deg_per_px, camera.mouse_deg_per_px);
    assert_eq!(s.fov_deg, camera.fov_deg);
    assert_eq!(s.pitch_limit_deg, camera.pitch_limit_deg);
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

/// The roots that are exempt from the rule above — the band and the crosshair.
fn tuning_roots(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, (With<ShowWhileTuning>, Without<ChildOf>)>();
    let mut roots: Vec<Entity> = q.iter(app.world()).collect();
    roots.sort();
    roots
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

    // ⚠️ **The one exception, and it is one screen wide.** The settings screen is where the
    // aim-assist reach is set, and the search band is the picture of that number — hiding it
    // there means the only way to tune the knob is set → close → look → reopen, which is
    // exactly what the user asked the band to end (`docs/FINDINGS.md` FIND-135, FIND-136).
    // So on `Screen::Settings` the elements carrying `hud::ShowWhileTuning` — the band and the
    // crosshair it is measured from — stay up, and **nothing else does**: the bars, the pips,
    // the objective counter, the hit mark and the arm markers report a fight, and there is no
    // fight while a menu is up. That is FIND-092 §2's rule, narrowed rather than weakened.
    press(&mut app, &PauseAction::Settings);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Settings);
    let tuning = tuning_roots(&mut app);
    assert!(
        tuning.len() >= 3,
        "the exception has to have something in it — found {} tuning roots",
        tuning.len()
    );
    for &e in &roots {
        let want = if tuning.contains(&e) { Visibility::Inherited } else { Visibility::Hidden };
        assert_eq!(
            visibility(&app, e),
            want,
            "{e} on the settings screen: the band and the crosshair stay, everything else goes"
        );
    }
    assert_eq!(
        hud_drawing(&mut app),
        while_playing,
        "the exception must not change WHICH elements the HUD would draw (F-170, F-171)"
    );

    // Back out of the options, and the exception ends with the screen it belongs to.
    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
    for &e in &roots {
        assert_eq!(
            visibility(&app, e),
            Visibility::Hidden,
            "{e} kept the settings screen's exemption on the pause plate"
        );
    }

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
    assert_eq!(rows.len(), 5, "the settings screen has five adjustable rows");
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

// ---------------------------------------------------------------------------------------
// The front door: the title screen (`UI-001`, 2026-08-19)
//
// > *„gibt es ein hauptmenü?"* — the user. There was not one: a flagless `cargo run` put him
// > straight into the hub, with the game's name nowhere on screen and no *New Game* and no
// > *Quit* before the first frame of play.
//
// Everything here needs a window entity for the same reason the rest of the file does — the
// domain is gated on `With<PrimaryWindow>`. What a `--headless` run can still prove is which
// screen the launch *decided* on, and `menu::announce_the_first_screen` says that out loud.
// ---------------------------------------------------------------------------------------

use defeated_by_titan::menu::title::TitleAction;

/// A launch that named **no door at all** — `--headless` is not a door, it is how anything runs
/// on this machine — plus a window entity, so this domain draws.
///
/// ⚠️ **The clock is set to manual BEFORE the first `update`.** That is what makes
/// [`f175_the_title_lets_no_frame_of_the_game_run`] able to see a single frame of `Playing`: at
/// 100 ms a step, one unpaused frame is roughly six fixed steps, and the tick counter shows it.
/// With real time the very first frame's delta is ~0 and a flicker would hide inside it.
fn app_at_the_front_door() -> (App, Entity) {
    let mut app = defeated_by_titan::app(Cli::from_args(["--headless".to_string()]));
    let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    app.update();
    (app, window)
}

/// Which plates are on screen, by the screen they were built for.
fn plates(app: &mut App) -> Vec<Screen> {
    let mut q = app.world_mut().query::<&MenuRoot>();
    q.iter(app.world()).map(|root| root.0).collect()
}

/// ★ **The first thing a launch shows.**
///
/// Goes red the moment the boot flow forgets the front door: without it the run lands in
/// `Screen::Playing`, no plate is built, and the first assert names the screen that is up.
#[test]
fn f175_a_flagless_launch_opens_the_title_screen_first() {
    let (mut app, window) = app_at_the_front_door();

    assert_eq!(
        plates(&mut app),
        vec![Screen::Title],
        "a launch that named no door has to open the title screen, and exactly one plate"
    );

    let text = plate_text(&mut app);
    assert!(
        text.iter().any(|t| t == defeated_by_titan::WINDOW_TITLE),
        "the title screen does not say the game's name: {text:?}"
    );
    for entry in ["Play", "Settings", "Quit"] {
        assert!(text.iter().any(|t| t == entry), "no {entry:?} on the title: {text:?}");
    }

    // ⚠️ **The pointer case nothing else in this domain has.** Every other screen hands a
    // captured pointer back; here nothing has ever taken it, and nothing may.
    let c = cursor(&app, window);
    assert_eq!(
        c.grab_mode,
        CursorGrabMode::None,
        "the title screen grabbed the pointer — there is nothing to look around at yet"
    );
    assert!(c.visible, "a title screen you cannot click is a wall, not a door");

    // The hub is loaded behind the plate and stopped, which is why *New Game* is a release and
    // not a second boot path.
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "the world behind the title is not the hub — then *New Game* would have to build one"
    );
    assert!(
        app.world().resource::<Time<Virtual>>().is_paused(),
        "the game ran underneath the title screen"
    );
}

/// ★ **Not one frame of the game runs before the player asks for it.**
///
/// The half a screenshot could not show, and the half the test above does not cover: a plate
/// can be on screen over a world that is quietly running underneath it. `Time<Virtual>` is what
/// `run_fixed_main_schedule` feeds on, so **the tick counter is the honest question** — and it
/// answers it in numbers: 0 while the title is up, 29 over the same five frames once it is gone
/// (measured 2026-08-19, 100 ms a frame at 60 Hz).
///
/// ⚠️ It does **not** prove anything about the very first frame: the fixed loop of frame one
/// runs before `apply_screen` ever pauses the clock, and it stayed at 0 only because a first
/// frame's delta is ~0. That is measured, not designed.
#[test]
fn f175_the_title_lets_no_frame_of_the_game_run() {
    use defeated_by_titan::shared::Tick;

    let (mut app, window) = app_at_the_front_door();
    assert_eq!(
        app.world().resource::<Tick>().0,
        0,
        "the simulation stepped before the title screen was even up"
    );

    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Tick>().0,
        0,
        "five frames at 100 ms went into the simulation while nobody had pressed anything"
    );
    assert_eq!(
        app.world().resource::<Time<Virtual>>().elapsed(),
        std::time::Duration::ZERO,
        "the virtual clock moved under the title screen"
    );
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::None, "the pointer was taken");
}

/// *Play* puts the player **in the hub, on his feet** — pointer taken, clock running, no plate.
///
/// 🔴 **Rewritten twice, and the second rewrite undoes the first.** On 2026-08-24 this test was
/// changed to assert that *Play* opens the mission list, on the premise that the list was
/// otherwise unreachable from a cold start. **That premise was false** — `menu::pause` has
/// offered *Mission select* outside a sortie since 2026-08-18, one day before the title screen
/// existed, so the cold-start route was always `Play → hub → Esc → Mission select`
/// (`f175_the_mission_list_is_still_reachable_from_the_hub` is that route as an assertion).
/// What the change actually did was take the walking away: *„mit lobby mein ich auch rumlaufen.
/// also eher eine art hub"* (the user, 2026-08-26). `docs/FINDINGS.md` FIND-173.
#[test]
fn f175_play_puts_the_player_in_the_hub_and_not_behind_a_plate() {
    use defeated_by_titan::shared::Tick;

    let (mut app, window) = app_at_the_front_door();
    press(&mut app, &TitleAction::NewGame);
    app.update();

    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "*Play* has to hand the player the world he stands in, not another plate"
    );
    assert!(plates(&mut app).is_empty(), "a plate stayed up over the running game");

    let c = cursor(&app, window);
    assert_eq!(c.grab_mode, CursorGrabMode::Locked, "the game has to take the pointer");
    assert!(!c.visible, "a system cursor in the middle of the crosshair is nobody's design");
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());

    let started = app.world().resource::<Tick>().0;
    app.update();
    assert!(app.world().resource::<Tick>().0 > started, "the simulation never started");
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "walking out of the mission list has to leave you standing in the hub, not in a sortie"
    );
}

/// `Esc` is a way **back**, and behind the title there is nothing.
///
/// It must not start the game (a reflex press would skip the menu) and it must not quit it (a
/// reflex press would end the run). Both are one button away and neither is a key.
#[test]
fn f175_escape_is_no_way_out_of_the_title() {
    let (mut app, window) = app_at_the_front_door();
    press_esc(&mut app, window);

    assert_eq!(*app.world().resource::<Screen>(), Screen::Title, "Esc walked out of the title");
    assert_eq!(plates(&mut app), vec![Screen::Title], "the title plate went away on Esc");
    assert!(app.should_exit().is_none(), "Esc on the title ended the run");
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::None);
}

/// ★ **The invariant the title screen could have broken.**
///
/// *"Settings is reached from the pause screen and from nowhere else, so there is exactly one
/// route in and `Esc` always knows where back is"* — the title is the second route. The way back
/// is therefore **recorded** (`menu::SettingsFrom`) instead of assumed, and this test drives
/// both routes in one run, because a constant answer is right on exactly one of them.
#[test]
fn f175_the_settings_come_back_to_the_screen_that_opened_them() {
    // Route one, the new one: title → settings → title, by key and by button.
    let (mut app, window) = app_at_the_front_door();
    press(&mut app, &TitleAction::Settings);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Settings);

    press_esc(&mut app, window);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Title,
        "Esc out of the settings landed on a screen the player never came from"
    );

    press(&mut app, &TitleAction::Settings);
    app.update();
    press(&mut app, &SettingsAction::Back);
    app.update();
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Title,
        "the Back button and Esc have to agree — they are the same door"
    );

    // Route two, the old one, unchanged: pause → settings → pause.
    let (mut app, window) = app_with_window();
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Settings);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Settings);
    press_esc(&mut app, window);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Paused,
        "the pause route lost its way back when the title route was added"
    );
}

/// **Nothing on this plate is a row that cannot do anything.**
///
/// `UI-001` asks for *"Play, Neuigkeiten, Einstellungen, Sozial-Links"* and two of those four
/// have nothing behind them today; `save` is being built in a parallel round, so there is also
/// nothing to *Continue*. The registry rule of §4 applies to a menu the same way it applies to a
/// spawn table: do not add a row nothing can spawn.
#[test]
fn f175_the_title_offers_only_what_the_game_can_actually_do() {
    let (mut app, _window) = app_at_the_front_door();

    assert_eq!(
        in_screen_order::<TitleAction>(&mut app),
        vec![TitleAction::NewGame, TitleAction::Settings, TitleAction::Quit],
        "the title's rows are not the three that work, in the order they are read"
    );

    let text = plate_text(&mut app);
    assert!(
        text.iter().any(|t| t == "Play"),
        "the first row says what it does, and today it cannot say *New Game*: `save` loads the \
         one profile there is by itself, so a fresh career is not a thing this row could start"
    );
    for empty in ["Continue", "Load", "News", "Credits", "Multiplayer"] {
        assert!(
            !text.iter().any(|t| t.contains(empty)),
            "the title offers {empty:?} and nothing in this build can do it: {text:?}"
        );
    }
}

/// **`--hub` is the way past the front door**, and it is the flag every hub script already uses.
///
/// The Cli half of this — that `--script`, `--mission`, `--sandbox` and `--no-hub` walk past the
/// title too — is a table in `src/shared/cli.rs`; this is the half that proves the app acts on
/// it, which is the thing the 35 scripts in `scripts/` depend on.
#[test]
fn f175_a_named_door_walks_straight_past_the_title() {
    let start = Cli::from_args(["--headless".to_string(), "--hub".to_string()]);
    assert!(!start.title, "--hub names a door, so it has already answered the title's question");

    let (mut app, window) = windowed(start);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "--hub stopped at the title screen — every hub script would now start in a menu"
    );
    assert!(plates(&mut app).is_empty(), "a plate was built for a run that asked for the hub");
    assert_eq!(cursor(&app, window).grab_mode, CursorGrabMode::Locked);
}

// ---------------------------------------------------------------------------
// The lobby's squad section — the multiplayer front door (2026-08-19)
// ---------------------------------------------------------------------------

/// Opens the lobby the way a player does: `Esc`, then *Mission select*.
fn open_the_lobby(start: Cli) -> (App, Entity) {
    let (mut app, window) = windowed(start);
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Lobby);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Lobby);
    (app, window)
}

/// ★ **Who is here** — and it is the roster's answer, not a sentence in the menu.
#[test]
fn f176_the_lobby_lists_every_seat_in_the_session() {
    use defeated_by_titan::net::{Roster, SeatKind};

    let (mut app, _window) = open_the_lobby(Cli { headless: true, port: Some(0), ..default() });

    let seats = app.world().resource::<Roster>().len();
    assert_eq!(seats, 1, "a run with no peers has exactly one seat, not {seats}");
    let lines = plate_text(&mut app);
    assert!(
        lines.iter().any(|l| l.starts_with("Squad") && l.contains("1 in this session")),
        "the lobby has to say how many are in the session: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("you")),
        "the player has to find himself on the list: {lines:?}"
    );

    // Somebody joins — and the list is the roster's, so it grows without the menu knowing how.
    let addr = "127.0.0.1:40404".parse().expect("a literal address");
    app.world_mut().resource_mut::<Roster>().seat(
        defeated_by_titan::shared::PlayerId(9),
        SeatKind::Remote(addr),
        addr.to_string(),
        0,
    );
    app.update();
    let lines = plate_text(&mut app);
    assert!(
        lines.iter().any(|l| l.contains("2 in this session")),
        "the squad list did not follow the roster: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("127.0.0.1:40404")),
        "a seat that is on the roster has to be on the screen: {lines:?}"
    );
}

/// ★ **The *Host* row really opens a port, and the label says which one the OS gave.**
///
/// There is no *Join* row next to it and that is checked here too: a row that took an address
/// and then showed nothing would be the row `title.rs` refuses to draw.
#[test]
fn f176_the_host_row_opens_a_real_port_and_shows_it() {
    use defeated_by_titan::net::Host;

    // `port: Some(0)` — the OS picks a free one, so this test does not fight the game or a
    // second test run for 34197.
    let (mut app, _window) = open_the_lobby(Cli { headless: true, port: Some(0), ..default() });
    assert!(!app.world().resource::<Host>().is_open(), "nothing is hosting before the click");

    press(&mut app, &LobbyAction::Host(true));
    app.update();

    let port = app
        .world()
        .resource::<Host>()
        .port()
        .expect("pressing Host has to leave a bound port behind");
    let lines = plate_text(&mut app);
    println!("bound port {port}, plate says {lines:?}");
    assert!(
        lines.iter().any(|l| l.contains(&port.to_string())),
        "the row has to show the port the OS really gave, {port}: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("Input only")),
        "the screen has to say what the link is and is not: {lines:?}"
    );
    // Clicking it again closes it — the same row, the other way.
    press(&mut app, &LobbyAction::Host(false));
    app.update();
    assert!(!app.world().resource::<Host>().is_open(), "the door did not close again");
}


// ---------------------------------------------------------------------------
// `F-016` — the band is only worth something if it can be seen WHILE the knob moves
// ---------------------------------------------------------------------------

/// Gives the 3D camera a 1280 x 720 image to draw into.
///
/// **Without it the band has no pixels to stand on**: `place_catch_band` projects a direction
/// through `Camera::world_to_viewport`, and a headless run's camera has no target size to
/// project into — so every tick hides itself and a test about where the band lands would be
/// asserting against an empty set. The same helper, for the same reason, sits in `tests/hud.rs`
/// (`attach_screen`); the size is `debug::screenshot`'s own, which is also `Window::default()`'s,
/// so the menu's layout is the one the pictures are taken at.
fn attach_screen(app: &mut App) {
    let handle = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        images.add(Image::new_target_texture(
            1280,
            720,
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        ))
    };
    let camera = {
        let mut q = app.world_mut().query_filtered::<Entity, With<Camera3d>>();
        q.iter(app.world()).next().expect("there must be a 3D camera")
    };
    app.world_mut().entity_mut(camera).insert(RenderTarget::Image(handle.into()));
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);
}

/// Every drawn tick of the search band, as a screen rectangle `(x0, y0, x1, y1)`.
///
/// Read out of the layout (`ComputedNode` + `UiGlobalTransform`) and not out of the `Node`'s
/// own `left`/`top`, because what is being asserted is where the thing **lands**, and a
/// rectangle inside a menu's flex column has no `left` of its own to compare against.
fn band_rects(app: &mut App) -> Vec<(f32, f32, f32, f32)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Node, &ComputedNode, &UiGlobalTransform), With<CatchTick>>();
    q.iter(app.world())
        .filter(|(node, ..)| node.display != Display::None)
        .map(|(_, computed, at)| {
            let (s, c) = (computed.size(), at.translation);
            (c.x - s.x / 2.0, c.y - s.y / 2.0, c.x + s.x / 2.0, c.y + s.y / 2.0)
        })
        .collect()
}

/// Turns the aim-assist reach up by `n` clicks of the `+` arrow, one frame each.
///
/// ⚠️ The **strength** goes up first, once, because `PlayerSettings::assist_is_on` is
/// `catch > 0 && strength > 0` and the band is gated on exactly that predicate — the same one
/// `vector::aim` filters on, so *no probe cast* and *no band* stay one decision (FIND-135). A
/// run that only turned the reach up would draw nothing and prove nothing.
fn nudge_reach(app: &mut App, n: usize) {
    if app.world().resource::<PlayerSettings>().assist_strength_pct == 0.0 {
        press(app, &SettingsAction::AssistStrength(Nudge::Up));
        app.update();
    }
    for _ in 0..n {
        press(app, &SettingsAction::AssistCatch(Nudge::Up));
        app.update();
    }
}

/// ★ **The band answers the slider while the slider is on screen.**
///
/// The user asked for the band for one stated reason — *„es soll in der ui angezeigt werden von
/// wo bis wo gesearched wird **damit man das besser einstellen kann**!"* — and it landed hidden
/// behind `hud::hide_while_a_menu_is_up`, so tuning it was set → close → look → reopen. This is
/// that sentence as a test: the settings screen is up, the `Aim assist reach` row is pressed,
/// and the band on screen gets wider in the same frame.
#[test]
fn f016_the_band_widens_under_the_slider_while_the_settings_screen_is_up() {
    let (mut app, window) = app_with_window();
    attach_screen(&mut app);
    open_settings(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Settings);
    assert_eq!(
        app.world().resource::<PlayerSettings>().assist_catch_pct,
        0.0,
        "the run has to start at free aim for 'no search, no band' to mean anything"
    );
    assert!(band_rects(&mut app).is_empty(), "0 % is free aim and draws no band");

    nudge_reach(&mut app, 1);
    let one = band_rects(&mut app);
    assert!(
        !one.is_empty(),
        "one click of `Aim assist reach` has to put the band on screen without closing the menu"
    );
    // Laid out is not the same as **on screen**: `hud::hide_while_a_menu_is_up` writes
    // `Visibility` on the roots and leaves `Node.display` alone, so a band that was hidden
    // would still have every rectangle above. This is the half the user asked for.
    let band: Vec<Entity> = {
        let mut q = app.world_mut().query_filtered::<Entity, With<CatchTick>>();
        q.iter(app.world()).collect()
    };
    for e in band {
        assert_ne!(
            visibility(&app, e),
            Visibility::Hidden,
            "the band is laid out but hidden — tuning it is still set, close, look, reopen"
        );
    }

    nudge_reach(&mut app, 7);
    let eight = band_rects(&mut app);
    let reach = |r: &[(f32, f32, f32, f32)]| {
        r.iter().fold(0.0_f32, |w, (x0, _, x1, _)| w.max((x1 - 640.0).abs().max((640.0 - x0).abs())))
    };
    assert!(
        reach(&eight) > reach(&one) * 3.0,
        "eight clicks have to be visibly wider than one: {:.1} px against {:.1} px",
        reach(&eight),
        reach(&one)
    );
    println!("BAND one click {:.1} px · eight clicks {:.1} px", reach(&one), reach(&eight));
}

/// ★ **A ruler under an almost-opaque backdrop is not a dimmer ruler, it is no ruler.**
///
/// `plate::root` is a full-screen node at `BACKDROP`'s 0.90 alpha and it is spawned **after**
/// the HUD, so by default it sits on top of it. Both halves are asserted here: the band is
/// above the plate's backdrop in the UI stack, and the contrast it then has against the dimmed
/// world clears WCAG 1.4.11's 3:1 on **any** frame — the same bar FIND-093 held the plate's
/// edge to, and with the same arithmetic. The backdrop itself is not touched: FIND-093 raised
/// it to 0.90 for the edge's 9.94:1 and this fix costs that nothing.
#[test]
fn f016_the_band_reads_over_the_settings_backdrop() {
    let (mut app, window) = app_with_window();
    attach_screen(&mut app);
    open_settings(&mut app, window);
    nudge_reach(&mut app, 20);
    app.update();

    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<MenuRoot>>();
        q.iter(app.world()).next().expect("the settings plate has to be on screen")
    };
    let band: Vec<Entity> = {
        let mut q = app.world_mut().query_filtered::<Entity, With<CatchTick>>();
        q.iter(app.world()).collect()
    };
    let stack = app.world().resource::<UiStack>();
    let at = |e: Entity| {
        stack.uinodes.iter().position(|x| *x == e).expect("a laid-out node is in the UI stack")
    };
    let backdrop = at(root);
    for &e in &band {
        assert!(
            at(e) > backdrop,
            "the band is buried under the backdrop: tick at {} against the plate at {backdrop}",
            at(e)
        );
    }

    // The contrast the player then gets, on the darkest and the brightest frame the game can
    // put behind the menu. `NEUTRAL` is white at 0.75 alpha, so it composites over whatever the
    // backdrop left of the world.
    let neutral = luminance(defeated_by_titan::hud::crosshair::NEUTRAL);
    let alpha = defeated_by_titan::hud::crosshair::NEUTRAL.to_srgba().alpha;
    let mut worst = f32::MAX;
    for step in 0..=10 {
        let world = step as f32 / 10.0;
        let behind = behind_the_menu(world);
        let over = alpha * neutral + (1.0 - alpha) * behind;
        // What it would have been buried: the band paints on the world, the backdrop paints on
        // both, and the two ends of the comparison move together.
        let buried = behind_the_menu(alpha * neutral + (1.0 - alpha) * world);
        println!(
            "BAND world {world:.1} · over the backdrop {:.2}:1 · buried under it {:.2}:1",
            contrast(over, behind),
            contrast(buried, behind)
        );
        worst = worst.min(contrast(over, behind));
    }
    assert!(
        worst >= 3.0,
        "the band has to clear WCAG 1.4.11's 3:1 on any frame — worst {worst:.2}:1"
    );
}

/// ★ **The settings screen leaves the crosshair's row empty.**
///
/// The band's position **is** an angle: it is drawn level with the crosshair because that is
/// where the sweep looks, and it cannot be moved out of the way without lying about where the
/// search is (`docs/FINDINGS.md` FIND-133, FIND-135). So it is the menu that gets out of the
/// way — `plate::CENTRE_LANE_PX` — and this is the test that says so. It walks every drawn node
/// of the settings screen except the backdrop itself and asserts none of them touches a drawn
/// tick.
#[test]
fn f016_the_settings_screen_leaves_the_bands_lane_empty() {
    let (mut app, window) = app_with_window();
    attach_screen(&mut app);
    open_settings(&mut app, window);
    // The widest band there is: every settings row has to clear the worst case, not the one
    // this run happens to be at.
    nudge_reach(&mut app, 20);
    app.update();
    assert_eq!(app.world().resource::<PlayerSettings>().assist_catch_pct, 100.0);

    let band = band_rects(&mut app);
    assert!(!band.is_empty(), "there has to be a band for this test to prove anything");
    let plate: Vec<(f32, f32, f32, f32, String)> = {
        let mut q = app.world_mut().query_filtered::<
            (&ComputedNode, &UiGlobalTransform, Option<&Text>),
            (With<PauseElement>, Without<MenuRoot>, Without<plate::CentreLane>),
        >();
        q.iter(app.world())
            .map(|(computed, at, text)| {
                let (s, c) = (computed.size(), at.translation);
                (
                    c.x - s.x / 2.0,
                    c.y - s.y / 2.0,
                    c.x + s.x / 2.0,
                    c.y + s.y / 2.0,
                    text.map(|t| t.0.clone()).unwrap_or_default(),
                )
            })
            .collect()
    };
    assert!(plate.len() > 10, "the settings screen has to be built for this to prove anything");

    let lane = band.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |a, b| {
        (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
    });
    println!(
        "BAND lane x {:.1}..{:.1} y {:.1}..{:.1}, {} ticks",
        lane.0,
        lane.2,
        lane.1,
        lane.3,
        band.len()
    );
    for (x0, y0, x1, y1, text) in &plate {
        let clear = *x1 <= lane.0 || *x0 >= lane.2 || *y1 <= lane.1 || *y0 >= lane.3;
        assert!(
            clear,
            "a settings node sits in the band's lane: x {x0:.1}..{x1:.1} y {y0:.1}..{y1:.1} \
             against the lane y {:.1}..{:.1} — {text:?}",
            lane.1, lane.3
        );
    }
    // And the lane is wide enough for the tallest tick with room on both sides, so a row that
    // grows by a pixel does not silently start clipping it.
    let (above, below) = plate.iter().fold((f32::MIN, f32::MAX), |(a, b), (_, y0, _, y1, _)| {
        (if *y1 <= lane.1 { a.max(*y1) } else { a }, if *y0 >= lane.3 { b.min(*y0) } else { b })
    });
    println!("BAND free lane {above:.1}..{below:.1} = {:.1} px", below - above);
    assert!(
        below - above >= END_H_PX + 8.0,
        "the lane is {:.1} px for a {END_H_PX:.0} px tick — it needs {:.0}",
        below - above,
        END_H_PX + 8.0
    );
}

// ---------------------------------------------------------------------------
// F-175 — the whole loop: title → lobby → sortie → debrief → lobby (2026-08-24)
// ---------------------------------------------------------------------------
//
// > *„es fehlt die lobby … erstelle die ganze gameloop mit lobby und main menu etc."*
// > — the user, 2026-08-23.
//
// Two holes, and one test each:
//
// 1. **The lobby was built and unreachable.** `Screen::Lobby` works, and the only route into it
//    was the pause screen *inside a running game*. From a cold start *New Game* dropped the
//    player into the hub and the mission list existed only for somebody who already knew that
//    `Esc` was hiding it.
// 2. **The loop had no end.** `Won` and `Lost` only logged a line; three seconds later the hub
//    took over. Nothing was ever shown to the player about the sortie he had just flown, and
//    there was no way back to the lobby except `Esc` again.

use defeated_by_titan::menu::debrief::DebriefAction;
use defeated_by_titan::shared::{HitZone, PlayerId, TitanHit, TitanId};

/// A windowed run standing in the hub, ticking 100 ms a frame so a sortie can be flown inside a
/// test without waiting for a wall clock.
fn a_windowed_hub() -> (App, Entity) {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    (app, window)
}

/// Frames of 100 ms until `seconds` of them have gone by. **Frames and not ticks**: a screen
/// stops `Time<Virtual>` and therefore the ticks, which is exactly what a debrief the player is
/// still reading has to do.
fn run_for(app: &mut App, seconds: f32) {
    for _ in 0..((seconds * 10.0).round() as u32) {
        app.update();
    }
}

/// Stands every player on the hub's first deployment pad — the door `missions.ron` calls *"the
/// one you find without looking for it"*. No `.single()`: every player is one of many.
fn onto_the_first_pad(app: &mut App) {
    let pad = {
        let missions = &app.world().resource::<GameData>().missions;
        Vec3::from(missions.hub.deployments.first().expect("a deployment pad").center_m)
    };
    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerId>>();
    let players: Vec<Entity> = q.iter(app.world()).collect();
    assert!(!players.is_empty(), "no player to put on a pad");
    for p in players {
        app.world_mut().entity_mut(p).insert(Transform::from_translation(pad));
    }
}

/// The running sortie's kill target, out of the mission entity — never a literal here.
fn kill_target(app: &mut App) -> u32 {
    let mut q = app.world_mut().query::<&defeated_by_titan::mission::KillTally>();
    q.iter(app.world()).next().expect("no mission is running").target
}

/// Wins the running sortie with the same `TitanHit` messages a blade writes.
fn win_it(app: &mut App) {
    let player = {
        let mut q = app.world_mut().query::<&PlayerId>();
        *q.iter(app.world()).next().expect("no player in the world")
    };
    let target = kill_target(app);
    for i in 0..target {
        app.world_mut().write_message(TitanHit {
            titan: TitanId(700 + i),
            by: player,
            zone: HitZone::Cortex,
            speed_m_s: 30.0,
        });
        app.update();
    }
}

/// ★ **The half that must not be lost twice**: from a cold start, the mission list is still
/// two presses away — and the way to it never went through the title screen.
///
/// 🔴 This test used to assert the opposite (`f175_the_front_door_leads_to_the_mission_list`,
/// 2026-08-24) on a premise it did not check: that *Play* was the only door into
/// `Screen::Lobby`. It was not. `menu::pause` pushes *Mission select* in its **not in a sortie**
/// branch — i.e. exactly when the player is standing in the hub — and has done since
/// 2026-08-18, one day before the title screen was built. Measured, not remembered:
/// `git show 9e51c16:src/menu/pause.rs`.
///
/// So the walk below is the whole cold start: `Play` (1 click) → hub → `Esc` (1 key) →
/// *Mission select* (2nd click). `F-175`'s *"every screen in at most two clicks"* holds.
#[test]
fn f175_the_mission_list_is_still_reachable_from_the_hub() {
    let start = Cli::from_args(["--headless".to_string()]);
    assert!(start.title, "a run that names no door has to open on the title screen");

    let (mut app, window) = windowed(start);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Title);

    press(&mut app, &TitleAction::NewGame);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "*Play* has to put him in the hub — that is the place he says is missing"
    );

    press_esc(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Paused);
    let mut actions = app.world_mut().query::<&PauseAction>();
    let rows: Vec<PauseAction> = actions.iter(app.world()).copied().collect();
    assert!(
        rows.contains(&PauseAction::Lobby),
        "the pause screen in the hub has to offer the mission list, and offers {rows:?}"
    );

    press(&mut app, &PauseAction::Lobby);
    app.update();
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Lobby,
        "two clicks and a key from a cold start have to reach the mission list"
    );
    let lines = plate_text(&mut app);
    assert!(
        lines.iter().any(|l| l.contains("Pick a sortie")),
        "the plate that came up is not the mission list: {lines:?}"
    );
}

/// ★★ **The loop has an end, and it is a screen the player reads.**
///
/// Not a log line: `Won` and `Lost` only ever called `announce`, and three seconds later the
/// hub took over. What a sortie did was never on screen at all.
#[test]
fn f175_the_debrief_is_a_screen_and_it_waits_for_the_player() {
    let (mut app, _window) = a_windowed_hub();
    onto_the_first_pad(&mut app);
    run_for(&mut app, 0.5);
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Active,
        "the pad did not deploy — the rest of this test would measure nothing"
    );
    let target = kill_target(&mut app);
    win_it(&mut app);
    run_for(&mut app, 0.3);
    assert_eq!(*app.world().resource::<State<MissionPhase>>().get(), MissionPhase::Won);

    // Well past every hold in `missions.ron: hub`. The debrief is a screen, so the clock stops
    // under it and it cannot be waited out.
    run_for(&mut app, 6.0);
    assert_ne!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "six seconds after the verdict the game is simply running again — the player was \
         never shown what his sortie did, and a loop that cannot report is not a loop"
    );
    let text = plate_text(&mut app).join(" | ");
    println!("debrief plate: {text}");
    assert!(
        text.contains(&format!("{target} / {target}")),
        "the debrief has to say the kills against the target: {text:?}"
    );
    assert!(text.contains("WON"), "the debrief has to say how it ended: {text:?}");
    assert!(
        text.contains("Ashgate Skirmish"),
        "and which sortie it was reporting on: {text:?}"
    );
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Debrief,
        "the screen is up and the phase moved on underneath it — then it is not the debrief, \
         it is a picture of one"
    );

    // ★★ **And the loop closes.** The way out of the report is the mission list, and the
    // mission list starts the next sortie — which is the whole ring in one run:
    // hub → sortie → verdict → debrief → lobby → sortie.
    press(&mut app, &DebriefAction::Lobby);
    app.update();
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Lobby,
        "the debrief has no way back to the mission list"
    );

    press(&mut app, &LobbyAction::Deploy);
    run_for(&mut app, 1.0);
    assert!(
        app.world().resource::<State<MissionPhase>>().get().is_running(),
        "the lobby did not start a second sortie after a debrief — the phase is {:?}",
        app.world().resource::<State<MissionPhase>>().get()
    );
    let mut q = app.world_mut().query::<&defeated_by_titan::mission::KillTally>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "the second sortie is counting kills on two tallies — the finished mission was never \
         cleared away, which is what routing a deploy through the hub is for"
    );
}

/// ★ **The second door out of the report, and the one that can quietly ruin the next sortie.**
///
/// *Redeploy* means *the same one again* — the same template **and the same difficulty**, out of
/// `mission::Sortie` and not out of `LobbyChoice`, which may hold something the player last
/// touched three sorties ago. And it has to go **through the hub**: deploying straight out of a
/// finished sortie leaves the old mission entity standing, and then two `KillTally`s count the
/// next run's kills (`mission::take_orders_from_the_menu`).
#[test]
fn f175_redeploy_flies_the_same_sortie_again_and_through_the_hub() {
    let (mut app, _window) = a_windowed_hub();
    onto_the_first_pad(&mut app);
    run_for(&mut app, 0.5);
    let flown = app.world().resource::<Sortie>().0.clone().expect("the pad set an order");
    win_it(&mut app);
    run_for(&mut app, 6.0);
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Debrief,
        "the debrief is not up — the rest of this test would press a button that is not there"
    );

    press(&mut app, &DebriefAction::Redeploy);
    run_for(&mut app, 1.5);

    let again = app.world().resource::<Sortie>().0.clone().expect("Redeploy set no order");
    assert_eq!(again.template, flown.template, "Redeploy flew a different mission");
    assert_eq!(
        again.difficulty, flown.difficulty,
        "Redeploy flew the same mission at a different level — *again* has to mean again"
    );
    assert!(
        app.world().resource::<State<MissionPhase>>().get().is_running(),
        "Redeploy never started anything — the phase is {:?}",
        app.world().resource::<State<MissionPhase>>().get()
    );
    let mut q = app.world_mut().query::<&defeated_by_titan::mission::KillTally>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "two kill counters are running — the finished mission was never cleared, so Redeploy \
         went over the top of the hub instead of through it"
    );
}

// ===========================================================================================
// F-177 — THE MISSION BOARD. The signpost in the muster yard is a door into the overview.
// ===========================================================================================
//
// > *„wenn man in der hub auf ein board drueckt (F) dann kommt man in eine mission uebersciht
// > in der man auswaehlen kann was man machen will!"* — the user, 2026-08-27 (`Q-059`).
//
// The RED test. It uses **only API that exists before the feature** — an app in the hub, the
// player's `Transform`, a real `KeyboardInput` for `F`, and `menu::Screen` — so that the first
// run of it measures the game and not a compile error.

/// Presses and releases one key through the real message path, exactly as `press_esc` does.
/// `window` may be any entity: `bevy_input::keyboard_input_system` never looks at it.
fn tap_key(app: &mut App, window: Entity, code: KeyCode, key: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: key.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

/// Puts the local player exactly there. A test may write a `Transform` the game normally warps.
fn stand_at(app: &mut App, at: Vec3) {
    let mut q = app.world_mut().query_filtered::<&mut Transform, With<LocalPlayer>>();
    for mut t in q.iter_mut(app.world_mut()) {
        t.translation = at;
    }
}

/// The real app in the hub with **no window at all** — `--headless`, which is how every run
/// on this machine goes and the only mode a script can drive. The returned entity is a bare
/// id for [`KeyboardInput::window`]; `bevy_input::keyboard_input_system` never looks at it.
fn in_the_hub_no_window() -> (App, Entity) {
    let mut app = defeated_by_titan::app(Cli { headless: true, hub: true, ..default() });
    let fake = app.world_mut().spawn_empty().id();
    for _ in 0..4 {
        app.update();
    }
    (app, fake)
}

/// Holds a key down for `secs` of **real** time, updating the app all the while — the hold the
/// board measures is on `Time<Real>`, so a test that faked the clock would not be measuring it.
fn hold_key(app: &mut App, window: Entity, code: KeyCode, key: Key, secs: f32) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: key.clone(),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f32() < secs {
        app.update();
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: key,
        state: ButtonState::Released,
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

fn tap_f(app: &mut App, window: Entity) {
    tap_key(app, window, KeyCode::KeyF, Key::Character("f".into()));
}

fn hold_f(app: &mut App, window: Entity, secs: f32) {
    hold_key(app, window, KeyCode::KeyF, Key::Character("f".into()), secs);
}

/// Where the board stands and how far it reaches, out of the loaded file.
fn board_post(app: &App) -> (Vec3, f32, f32) {
    let post = &app.world().resource::<GameData>().missions.hub.board;
    (Vec3::from(post.center_m), post.radius_m, post.hold_s)
}

/// Puts the player at the board, one metre inside its circle on the +Z side.
fn stand_at_the_board(app: &mut App) {
    let (centre, radius, _) = board_post(app);
    stand_at(app, centre + Vec3::new(0.0, 0.2, radius - 1.0));
    app.update();
}

#[test]
fn f177_f_at_the_board_opens_the_mission_overview() {
    // The signpost of `maps.ron: ashgate`, the one the survey photographed standing right of
    // the spawn view. A literal here **on purpose**: this is the red test, and it may not
    // depend on the file field the fix is going to add.
    let signpost = Vec3::new(3.6, 1.8, -4.2);

    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    stand_at(&mut app, signpost + Vec3::new(0.0, 0.2, 2.0));
    app.update();

    tap_key(&mut app, window, KeyCode::KeyF, Key::Character("f".into()));

    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Lobby,
        "F at the mission board did not open the mission overview"
    );
}

/// ⭐ **The provenance leg: the position under test is one the GAME produced.**
///
/// `docs/FINDINGS.md` FIND-190's shape — *two functions asked about the same invented point
/// cannot disagree about which point it is* — is why this test does not warp anybody. It sets
/// the look direction, holds `W`, and lets `net::local` → `player` walk the capsule out of the
/// hub's own spawn point until the board says it is in reach. Nothing here writes a
/// `Transform`.
#[test]
fn f177_he_walks_to_the_board_from_the_spawn_point_and_it_comes_into_reach() {
    let (mut app, window) = in_the_hub_no_window();
    // One simulation tick per `app.update()`. Without it a test frame advances the game by the
    // microseconds it took to run, and four seconds of held `W` move the player 0.2 mm.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_nanos(16_666_667),
    ));
    let (centre, radius, _) = board_post(&app);
    let spawn = Vec3::from(app.world().resource::<GameData>().missions.hub.spawn_m);

    let far = spawn.distance(centre);
    assert!(
        far > radius,
        "the board is inside the spawn circle ({far:.2} m against {radius:.2} m) — nobody \
         would ever have to walk to it, and this test would pass on tick 1"
    );
    assert!(!app.world().resource::<Board>().in_range, "in reach before he moved");

    // The bearing from the spawn point to the signpost. `look 0` faces -Z and `look 90` faces
    // -X (`scripts/f070-hub.txt` ACT 1 walks the hall that way), i.e. forward is
    // (-sin yaw, ., -cos yaw).
    // ⚠️ **Radians.** `net::local::read_input` keeps `Look` in radians and the `--script`
    // driver converts `look <deg>` on the way in; a degree number here walks him 194° off.
    let to = centre - spawn;
    let yaw = (-to.x).atan2(-to.z);
    app.world_mut().insert_resource(LookOverride(Some((yaw, 0.0))));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyW,
        logical_key: Key::Character("w".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });

    let mut arrived = None;
    for frame in 0..240 {
        app.update();
        if app.world().resource::<Board>().in_range {
            arrived = Some(frame);
            break;
        }
    }
    let frame = arrived.expect(
        "he held W straight at the signpost for four seconds and the board never came into \
         reach — the trigger is not on the prop, or the prop is not walkable to",
    );

    // And the answer agrees with where the game actually put him — read out of the world, not
    // out of anything this test wrote.
    let mut q = app.world_mut().query_filtered::<&Transform, With<LocalPlayer>>();
    let at = q.iter(app.world()).next().expect("a local player").translation;
    assert!(
        at.distance(centre) <= radius,
        "the board says in reach at frame {frame}, but the player the game is drawing stands \
         {:.2} m from it against a radius of {radius:.2}",
        at.distance(centre)
    );
    assert!(at.distance(spawn) > 0.5, "he never actually walked: {at:?}");
}

/// ★ **F away from the board does nothing at all** — it stays the left blade, and the mission
/// list stays shut. This is the half the user will notice first if it is wrong.
#[test]
fn f177_f_away_from_the_board_does_nothing_at_all() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    let (centre, radius, hold_s) = board_post(&app);
    // Just outside, and then far outside: the near miss is the one a wrong comparison passes.
    for out in [radius + 0.05, radius + 30.0] {
        stand_at(&mut app, centre + Vec3::new(0.0, 0.0, out));
        app.update();
        let before = app.world().resource::<LobbyChoice>().clone();

        tap_f(&mut app, window);
        tap_f(&mut app, window);
        hold_f(&mut app, window, hold_s * 2.0);

        assert_eq!(
            *app.world().resource::<Screen>(),
            Screen::Playing,
            "{out:.2} m from the board and F opened something"
        );
        let board = *app.world().resource::<Board>();
        assert!(!board.open, "{out:.2} m from the board and the overview is open");
        assert!(!board.in_range, "{out:.2} m from the board and it says in reach");
        assert_eq!(
            *app.world().resource::<LobbyChoice>(),
            before,
            "{out:.2} m from the board and F moved the choice"
        );
        assert_eq!(
            *app.world().resource::<State<MissionPhase>>().get(),
            MissionPhase::Hub,
            "{out:.2} m from the board and a hold on F deployed a sortie"
        );
    }
}

/// ★ **F during a sortie opens nothing**, and it opens nothing because the board is not there:
/// `DespawnOnExit(MissionPhase::Hub)` is the whole mechanism, held by
/// `tests/mission.rs::f177_the_board_is_a_hub_thing_and_does_not_follow_you_into_a_sortie`.
#[test]
fn f177_f_inside_a_sortie_opens_nothing() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    let (centre, _, hold_s) = board_post(&app);

    // Deploy through the door that already existed, so this test cannot be passed by breaking
    // the board's own deploy.
    press_esc(&mut app, window);
    press(&mut app, &PauseAction::Lobby);
    app.update();
    press(&mut app, &LobbyAction::Deploy);
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().resource::<State<MissionPhase>>().get().is_running(),
        "the fixture never got into a sortie"
    );

    // Standing exactly where the board would be, if there were one.
    stand_at(&mut app, centre);
    app.update();
    tap_f(&mut app, window);
    hold_f(&mut app, window, hold_s * 2.0);

    let board = *app.world().resource::<Board>();
    assert!(!board.in_range, "the board is in reach in the middle of a fight");
    assert!(!board.open, "F opened the mission list in the middle of a fight");
    assert_eq!(
        *app.world().resource::<Screen>(),
        Screen::Playing,
        "F put a menu over a running sortie"
    );
}

/// The press that opens may not also choose, and the next one must.
#[test]
fn f177_the_press_that_opens_chooses_nothing_and_the_next_tap_steps_one_on() {
    let (mut app, window) = in_the_hub_no_window();
    stand_at_the_board(&mut app);

    let list = entries(app.world().resource::<GameData>());
    assert!(list.len() >= 3, "missions.ron offers {} sorties — too few to step", list.len());
    let start = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    )
    .expect("a default sortie");

    tap_f(&mut app, window);
    assert!(app.world().resource::<Board>().open, "the first F did not open the board");
    let after_open = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    )
    .expect("a sortie");
    assert_eq!(after_open, start, "the press that opens the board also moved the choice");

    tap_f(&mut app, window);
    let after_step = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    )
    .expect("a sortie");
    let at = list.iter().position(|e| *e == start).expect("the default is in the list");
    assert_eq!(
        after_step,
        list[(at + 1) % list.len()],
        "a tap did not step to the next sortie in missions.ron's own order"
    );
}

/// ⭐ **THE SWEEP — every sortie in the file, and back to where it started.**
///
/// ## What the code under test reads (`menu::board::step`, `menu::lobby::{entries, chosen}`)
///
/// `missions.ron: templates` (every key and every difficulty key, in file order),
/// `missions.ron: hub.deployments` (the default door `chosen` falls back to), and
/// `LobbyChoice { mission, difficulty }`.
///
/// ## What this sweep varies
///
/// **The whole choice space**: every one of the `entries()` positions, reached by pressing `F`
/// the way a player reaches it, one press at a time, twice round the list. Nothing else in the
/// list above is held constant that the rule depends on — the file is the file, and the
/// fallback door is exercised by the first lap starting from the default.
///
/// ## What it skips: **nothing.** `stepped` is counted and asserted against `2 * len`, so a
/// press that quietly did nothing shows up as a short lap rather than as a passing `0 of N`.
#[test]
fn f177_the_board_walks_every_sortie_in_the_file_and_comes_back() {
    let (mut app, window) = in_the_hub_no_window();
    stand_at_the_board(&mut app);

    let list = entries(app.world().resource::<GameData>());
    let laps = 2;
    tap_f(&mut app, window); // open, and this press chooses nothing

    let mut seen: Vec<(String, Option<String>)> = Vec::new();
    let mut stepped = 0usize;
    for _ in 0..(list.len() * laps) {
        tap_f(&mut app, window);
        stepped += 1;
        seen.push(
            chosen(app.world().resource::<GameData>(), app.world().resource::<LobbyChoice>())
                .expect("a sortie after every press"),
        );
    }
    assert_eq!(stepped, list.len() * laps, "presses were skipped, and a skip is invisible");

    let one_lap: Vec<(String, Option<String>)> = seen[..list.len()].to_vec();
    let mut sorted = one_lap.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        list.len(),
        "one lap of {} presses landed on {} distinct sorties — the cycle repeats or skips: {:?}",
        list.len(),
        sorted.len(),
        one_lap
    );
    let mut want = list.clone();
    want.sort();
    assert_eq!(sorted, want, "the board does not offer exactly what missions.ron does");
    assert_eq!(
        seen[list.len()..].to_vec(),
        one_lap,
        "the second lap is not the first one — the cycle does not wrap the same way twice"
    );
}

/// ★★ **The claim the board exists for: a hold deploys the sortie it is showing, and it is a
/// sortie the default would NOT have flown.**
#[test]
fn f177_a_hold_deploys_the_sortie_the_board_is_showing() {
    let (mut app, window) = in_the_hub_no_window();
    stand_at_the_board(&mut app);
    let (_, _, hold_s) = board_post(&app);

    let default_door = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    )
    .expect("a default sortie");

    tap_f(&mut app, window); // open
    tap_f(&mut app, window); // step off the default, so the assert below cannot pass on it
    let showing = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    )
    .expect("a sortie");
    assert_ne!(showing, default_door, "the fixture never left the default door");

    hold_f(&mut app, window, hold_s + 0.15);
    for _ in 0..8 {
        app.update();
    }

    let order = app
        .world()
        .resource::<Sortie>()
        .0
        .clone()
        .expect("holding F at the board started no sortie");
    assert_eq!(order.template, showing.0, "the board flew a mission it was not showing");
    assert_eq!(order.difficulty, showing.1, "the board flew a level it was not showing");
    assert!(order.from_hub, "a sortie out of the board has to find its way back to the hub");
    assert!(
        app.world().resource::<State<MissionPhase>>().get().is_running(),
        "the order was written but nothing started — the phase is {:?}",
        app.world().resource::<State<MissionPhase>>().get()
    );
    assert!(!app.world().resource::<Board>().open, "the board stayed open over the sortie");
}

/// A hold that is **too short** is a tap: it steps, it does not fly. The two halves of one key
/// have to be told apart by the length of the press and by nothing else.
#[test]
fn f177_a_press_shorter_than_the_hold_steps_and_never_deploys() {
    let (mut app, window) = in_the_hub_no_window();
    stand_at_the_board(&mut app);
    let (_, _, hold_s) = board_post(&app);

    tap_f(&mut app, window); // open
    let before = chosen(
        app.world().resource::<GameData>(),
        app.world().resource::<LobbyChoice>(),
    );
    hold_f(&mut app, window, hold_s * 0.4);
    for _ in 0..8 {
        app.update();
    }

    assert!(
        app.world().resource::<Sortie>().0.is_none(),
        "a press of {:.2} s against a hold of {hold_s:.2} s deployed a sortie",
        hold_s * 0.4
    );
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Hub,
        "a short press left the hub"
    );
    assert_ne!(
        chosen(app.world().resource::<GameData>(), app.world().resource::<LobbyChoice>()),
        before,
        "a short press did not step the choice either — it did nothing at all"
    );
}

/// Walking away shuts the overview. Only a run with no window can do it — with one, the plate
/// stops the clock and he is not walking anywhere — and it is what keeps the board a place
/// instead of a modal you are stuck in.
#[test]
fn f177_walking_away_from_the_board_shuts_it() {
    let (mut app, window) = in_the_hub_no_window();
    stand_at_the_board(&mut app);
    tap_f(&mut app, window);
    assert!(app.world().resource::<Board>().open);

    let (centre, radius, _) = board_post(&app);
    stand_at(&mut app, centre + Vec3::new(0.0, 0.0, radius + 6.0));
    app.update();
    app.update();
    let board = *app.world().resource::<Board>();
    assert!(!board.in_range, "still in reach six metres outside the circle");
    assert!(!board.open, "he walked away and the mission overview followed him");
}

/// With a window the board **is** `Screen::Lobby`, so `Esc` and *Back* shut it — and the board
/// has to notice, or the next `F` would open something that is already open.
#[test]
fn f177_esc_shuts_the_plate_and_the_board_follows_it() {
    let (mut app, window) = windowed(Cli { headless: true, hub: true, ..default() });
    stand_at_the_board(&mut app);
    tap_f(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Lobby);
    assert!(app.world().resource::<Board>().open);

    press_esc(&mut app, window);
    app.update();
    assert_eq!(*app.world().resource::<Screen>(), Screen::Playing, "Esc did not shut the plate");
    assert!(
        !app.world().resource::<Board>().open,
        "the plate is gone and the board still thinks it is open — the next F would do nothing"
    );

    // And it opens again, which is the half a stuck flag would fail.
    tap_f(&mut app, window);
    assert_eq!(*app.world().resource::<Screen>(), Screen::Lobby, "the board would not reopen");
}

/// ⭐⭐ **THE SWEEP THAT HAS TO BE THREE-DIMENSIONAL.**
///
/// `f177_no_stance_in_the_hub_names_a_door_and_opens_another` swept **2 361 960** stances and
/// reported **0** lies while the shipped game lied at `(0, 2, 0)` yaw 140, photographed. It
/// varied `x`, `z` and `yaw` and took its **height** from one `stand()` call — and height was
/// the one axis the rule depended on. A million samples of the wrong slice measure the slice.
///
/// ## Every variable `menu::board::work_the_board` reads to answer *"is he in reach"*
///
/// 1. every `LocalPlayer`'s `Transform.translation` — **x, y and z**
/// 2. every `MissionBoard`'s `Transform.translation` — x, y, z
/// 3. that board's `radius_m`
/// 4. how many boards there are, and how many local players
///
/// It reads **nothing else** for this question: no yaw, no pitch, no velocity, no camera, no
/// ray, no walk model. That is the whole difference between this element and the retired
/// `hud::hub_prompt`, and it is why the list above is four lines long.
///
/// ## What this sweep varies
///
/// 1. `x`, **`y`** and `z` of the player, over a 3D grid centred on the board and reaching
///    twice its radius on every axis — **`y` at the same resolution as the other two**, which
///    is the correction FIND-181 bought
/// 2. the board's own centre and radius: read out of the world, never assumed
/// 3. window / no window (both, in the outer loop) — the one axis that decides whether
///    `Screen` is written at all
///
/// Held constant, and named because a variable the code reads that the sweep does not vary is
/// the bug: **the number of boards (1) and of local players (1)**. Both are what the hub has;
/// the two-board case is `mission::hub`'s "nearest wins" rule and has no second board to be
/// wrong about yet, and a second *local* player cannot exist — there is one keyboard.
///
/// ## What it skips
///
/// **Nothing, and the count is asserted.** There is no `continue` in the loop: every sample is
/// judged and `checked` has to come out at exactly `sides * n^3`. A `0 of N` whose N is
/// arithmetic about the wrong set is what this assertion exists against.
///
/// ## Where the position comes from
///
/// The oracle is computed from the `Transform` **the world holds after the frame** and not from
/// the vector this test wrote: the fixed step runs before `Update`, so a sample is judged
/// against the position `work_the_board` actually saw. That is FIND-190's correction — the
/// defect it missed was a *stale* position, and a test that asks the oracle about its own
/// literal cannot see one. The fully game-driven leg is
/// `f177_he_walks_to_the_board_from_the_spawn_point_and_it_comes_into_reach` and
/// `scripts/f177-board.txt`, which walk there with `W`.
#[test]
fn f177_the_board_reaches_the_same_distance_on_every_axis_including_height() {
    // 15 steps per axis over ±2 radius, and the same 15 on the vertical.
    const STEPS: i32 = 15;
    let mut checked = 0usize;
    let mut in_reach = 0usize;

    for windowed_run in [false, true] {
        let mut app = if windowed_run {
            windowed(Cli { headless: true, hub: true, ..default() }).0
        } else {
            in_the_hub_no_window().0
        };
        let (centre, radius, _) = board_post(&app);
        // Read out of the world, not out of the file: if the spawn ever stops copying the
        // file's number onto the component, this sweep measures the component.
        let (post_at, post_radius) = {
            let mut q = app
                .world_mut()
                .query::<(&defeated_by_titan::mission::hub::MissionBoard, &Transform)>();
            let (post, at) = q.iter(app.world()).next().expect("a mission board in the hub");
            (at.translation, post.radius_m)
        };
        assert_eq!(post_at, centre);
        assert_eq!(post_radius, radius);

        let span = radius * 2.0;
        for ix in 0..STEPS {
            for iy in 0..STEPS {
                for iz in 0..STEPS {
                    let f = |i: i32| -span + 2.0 * span * (i as f32) / ((STEPS - 1) as f32);
                    stand_at(&mut app, post_at + Vec3::new(f(ix), f(iy), f(iz)));
                    app.update();

                    // The position the game holds — which is the one the system read, because
                    // the fixed step runs before `Update` and nothing moves him after it.
                    let at = {
                        let mut q =
                            app.world_mut().query_filtered::<&Transform, With<LocalPlayer>>();
                        q.iter(app.world()).next().expect("a local player").translation
                    };
                    let want = at.distance(post_at) <= post_radius;
                    let said = app.world().resource::<Board>().in_range;
                    checked += 1;
                    if want {
                        in_reach += 1;
                    }
                    assert_eq!(
                        said, want,
                        "windowed = {windowed_run}, standing at {at:?}: {:.3} m from a board \
                         that reaches {post_radius:.3} m, and the board says in_range = {said}",
                        at.distance(post_at)
                    );
                }
            }
        }
    }

    let want = 2 * (STEPS as usize).pow(3);
    assert_eq!(checked, want, "the sweep skipped samples, and a skip is invisible in a 0 of N");
    // A sweep that never reached the inside of the circle would pass every assert above by
    // agreeing that nothing is ever in reach. Both answers have to occur.
    assert!(in_reach > 0, "not one sample of {checked} was inside the board's circle");
    assert!(in_reach < checked, "every sample of {checked} was inside — the grid is too small");
}

// ---------------------------------------------------------------------------
// F-120 / F-122 — the debrief reports the CAREER, not just the sortie (2026-09-01)
// ---------------------------------------------------------------------------
//
// > *„Ganz ausbauen"* — the user, 2026-09-01, choosing to build the progression out
// > (`docs/QUESTIONS.md`, the Q-062 override).
//
// `menu::debrief::DebriefLedger` was spawned as a **named, documented, empty node** on
// 2026-08-24 and stayed empty. Meanwhile `progress::Career` computed the level, the rank, the
// gear budget and what each sortie earned, every sortie, and wrote them to `info!` and nowhere
// else. Measured 2026-09-01 (`docs/FINDINGS.md` FIND-222): the shipped save had flown **419
// sorties** and spent **0 of 122** gear points, because no surface in the game had ever
// mentioned that a gear point existed.

/// ★★ **What the sortie earned lands on the screen the player reads.**
///
/// ⚠️ **The two halves are asserted separately on purpose**, and that split IS the finding: the
/// first half passed for a week before this test existed — `Career::last_sortie_xp` was right
/// all along — and only the second half was ever broken. A single combined assert would have
/// reported "the debrief is wrong" for a number that was already correct, and pointed the fix
/// at the wrong domain (`docs/lessons/fixtures.md`: name what the code reads, name what the
/// fixture varies, the difference is the bug).
#[test]
fn f120_the_debrief_says_what_the_sortie_earned() {
    use defeated_by_titan::progress::Career;
    use defeated_by_titan::shared::LocalPlayer;

    let (mut app, _window) = a_windowed_hub();
    onto_the_first_pad(&mut app);
    run_for(&mut app, 0.5);
    assert_eq!(
        *app.world().resource::<State<MissionPhase>>().get(),
        MissionPhase::Active,
        "the pad did not deploy — the rest of this test would measure nothing"
    );
    win_it(&mut app);
    run_for(&mut app, 0.3);
    assert_eq!(*app.world().resource::<State<MissionPhase>>().get(), MissionPhase::Won);
    run_for(&mut app, 6.0);

    // ---- half one: the number EXISTS. This half has always passed. -------------------
    let mut q = app.world_mut().query_filtered::<&Career, With<LocalPlayer>>();
    let career: Career = q.iter(app.world()).next().cloned().expect(
        "the local player has no Career at all — then this test is measuring the absence of a \
         component and not the absence of a line on a screen",
    );
    println!("career after the sortie: {}", career.one_line());
    assert!(
        career.last_sortie_xp > 0,
        "the sortie earned nothing, so the debrief has nothing to report and this test cannot \
         tell a missing LINE from a missing NUMBER: {career:?}"
    );
    assert!(career.gear_points > 0, "a level-1 career still has a budget: {career:?}");

    // ---- half two: the number is ON THE SCREEN. This half was red. -------------------
    let text = plate_text(&mut app).join(" | ");
    println!("debrief plate: {text}");
    assert!(
        text.contains(&format!("+{} XP", career.last_sortie_xp)),
        "the debrief does not say what the sortie earned. The number exists ({} xp) and the \
         screen that exists to report it is silent: {text:?}",
        career.last_sortie_xp
    );
    assert!(
        text.contains(&format!("LEVEL {}", career.level)),
        "the debrief does not say what level he is: {text:?}"
    );
    assert!(
        text.contains(&format!("RANK {}", career.rank)),
        "the debrief does not say what rank he is: {text:?}"
    );
    assert!(
        text.contains(&format!("{} GEAR POINTS", career.gear_points)),
        "the debrief does not say there is a budget to spend — which is why 122 of them sat \
         unspent for 419 sorties: {text:?}"
    );
}
