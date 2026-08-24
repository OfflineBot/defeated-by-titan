//! menu — the screens, the pointer, and the way back out.
//!
//! > *„zudem fehlen settings. menu (also bei escape) und eine main lobby in der man die mission
//! > starten kann"* — the user, 2026-08-13 (`docs/NEXT.md` §1D, reqs 6–8).
//!
//! Five screens since then, and one key that walks between them:
//!
//! ```text
//!   Title ──Play──► Lobby ──Deploy──► Playing ──Esc──► Paused ──Settings──► Settings
//!     │              │  ▲                │                │                    │
//!     └─Settings─────┘  │             (a sortie ends)     └─Mission select─────┤
//!                       │                │                                     │
//!                       └─Esc/To the lobby──── Debrief ◄──hub.verdict_s────────┘
//! ```
//!
//! **The loop closes at the debrief**, and that is what was missing until 2026-08-24: the
//! verdict was a log line, the hub took over three seconds later, and the mission list could
//! only be reached from inside a running game (`title.rs`, `debrief.rs`).
//!
//! ## The title screen is the door, and it is the **first** thing a launch shows
//!
//! > *„gibt es ein hauptmenü?"* — the user, 2026-08-19. Until then a flagless `cargo run`
//! > walked straight into the hub.
//!
//! Which screen a run opens on is decided **once**, by `FromWorld for Screen` out of
//! [`Cli::title`](crate::shared::Cli::title) — see the impl for why it is not a `Startup`
//! system. A command line that named any door at all (`--hub`, `--mission`, `--sandbox`,
//! `--no-hub`, `--script`) goes straight past it, which is what keeps all 35 scripts in
//! `scripts/` running exactly as they did.
//!
//! ## What this domain owns: the pointer
//!
//! A mouse-look game that leaves the system cursor free lets you turn until the screen edge
//! and then stop. So the pointer is **locked and hidden** the moment there is a window, and
//! every screen that is not [`Screen::Playing`] gives it back (`docs/PLAN-GAME.md` §8, `P4`).
//!
//! **The release was built before the capture.** A locked pointer with no release key is a
//! game you have to `pkill` — and on a machine where nobody has ever seen this game in a
//! window, that failure would have been found by somebody else, later, without a terminal
//! open. `tests/menu.rs` holds that guarantee across all five screens — including the
//! title, which is the one case where the pointer was **never taken in the first place**.
//!
//! ## What decides: the window entity, not the flag
//!
//! Every system here is gated on `With<PrimaryWindow>`. `src/lib.rs` builds
//! `primary_window: None` whenever `Cli::wants_window()` is false, so `--headless` and
//! `--offscreen` have **no window entity** and therefore grab nothing — and draw no menu —
//! without this file having to ask `Cli` at all. One condition instead of two that can drift
//! apart, and it is the *true* one: whether there is a pointer to take.
//!
//! ⚠️ **The consequence for evidence:** a `--headless` run can never exercise a screen in this
//! domain, because there is no window and therefore no menu. The tests in `tests/menu.rs` spawn
//! a window **entity** by hand (winit is disabled in that mode, so nothing opens) — that is the
//! only way anything here is checkable on this machine, and it is checked that way deliberately
//! rather than claimed.
//!
//! ## The hub is still a **place**, and the lobby is a **screen**
//!
//! `tests/menu.rs::f072_the_hub_is_a_place_and_not_a_screen` holds the older half of this: in
//! the hub you walk, the pointer stays locked and time runs, so the hub is `Screen::Playing`
//! and a phase of `mission::MissionPhase`. [`Screen::Lobby`] does **not** change that. It is the
//! front door to the same deployment the pads already do — a screen you open, pick a mission on
//! and leave again — and the walk-in pads keep working with no knowledge of it
//! (`scripts/f070-hub.txt`, 35 asserts, untouched).

pub mod debrief;
pub mod lobby;
pub mod pause;
pub mod plate;
pub mod settings;
pub mod title;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::data::GameData;
use crate::mission::{KillTally, Mission, MissionClock, MissionPhase, Verdict};
use crate::shared::PlayerSettings;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Screen>()
            // **Seeded here, and this is the one place it happens.** `PlayerSettings` has no
            // `Default` — it is built by `FromWorld` out of `GameData`, which `DataPlugin` has
            // already put down (`src/lib.rs` adds it before every other plugin). Registered
            // outside the `there_is_a_window` gate on purpose: `net::local::read_input` reads
            // the mouse sensitivity in **every** run, window or not.
            .init_resource::<PlayerSettings>()
            .init_resource::<lobby::LobbyChoice>()
            .init_resource::<SettingsFrom>()
            // **Says out loud which door this run came through**, once, at startup — and
            // outside the window gate on purpose. A `--headless` run draws no menu and can
            // therefore never show a screen; the one thing it *can* still prove is where the
            // launch landed, and this line is that evidence
            // (`tests/menu.rs` holds the in-process half).
            .add_systems(Startup, announce_the_first_screen)
            // **The one screen nobody asks for.** It comes up with the phase and it goes away
            // with it — `OnEnter`/`OnExit` and not a per-frame condition, because a condition
            // re-opens the plate the frame after the player left it and deadlocks the session
            // (`debrief::open_the_debrief`). Behind the window gate like everything else here:
            // a `--headless` run has no menu, and `missions.ron: hub.debrief_s` is what it
            // waits instead.
            .add_systems(
                OnEnter(MissionPhase::Debrief),
                debrief::open_the_debrief.run_if(there_is_a_window),
            )
            .add_systems(
                OnExit(MissionPhase::Debrief),
                debrief::close_the_debrief_screen.run_if(there_is_a_window),
            )
            .add_systems(
                Update,
                (
                    // The order is the answer to "what does one `Esc` do": it flips the
                    // screen, and everything downstream follows the screen. A button that ran
                    // after the pointer had already been applied would leave the mouse one
                    // frame behind the state it belongs to — and one frame of a free pointer
                    // over a running game is one frame in which a click lands outside the
                    // window.
                    (
                        toggle_screen,
                        title::title_buttons,
                        pause::pause_buttons,
                        settings::settings_buttons,
                        lobby::lobby_buttons,
                        debrief::debrief_buttons,
                    ),
                    // And it ends the sortie on the way out, whichever door was used.
                    debrief::close_the_debrief,
                    // After the buttons and before anything looks at the screen: the way back
                    // out of the options is recorded from where they were opened, and both
                    // openers get it without knowing about it.
                    remember_the_way_into_settings,
                    apply_screen,
                    // Clear first, build second, and `.chain()` between them: a screen that
                    // changed has to lose its old plate in the same frame it gains the new
                    // one, or the two overlap for a frame and the player clicks the wrong one.
                    despawn_menu,
                    spawn_menu,
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

/// `Esc` — the one key this domain knows.
///
/// ⚠️ `src/net/local.rs` calls itself *"the only place in the game that knows what a key
/// is"*, and that stays true of every **gameplay** binding: `Esc` produces no `Intent`, it
/// never reaches the simulation, and it does not travel over a wire. Pausing is a thing this
/// screen does to this machine. Once `F-172` moves the bindings into a file, this one goes
/// with them.
///
/// **It walks back one step, not all the way out.** From the settings screen `Esc` returns to
/// **the screen the settings were opened from** — [`SettingsFrom`], written by exactly one
/// system, because there are two openers since the title screen exists and an assumed answer
/// would be wrong on one of them. From the lobby it returns to the game, because the lobby's
/// "back" is the hub floor you are standing on either way.
///
/// **From the title `Esc` does nothing at all.** The title is where the chain of "back" ends:
/// there is nothing behind it, and the two things `Esc` could plausibly mean there — start the
/// game, or quit it — are both a button press away and neither should happen by reflex.
fn toggle_screen(
    keys: Res<ButtonInput<KeyCode>>,
    back: Res<SettingsFrom>,
    mut screen: ResMut<Screen>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let next = match *screen {
        Screen::Title => Screen::Title,
        Screen::Playing => Screen::Paused,
        Screen::Paused => Screen::Playing,
        Screen::Settings => back.0,
        Screen::Lobby => Screen::Playing,
        // **The one screen `Esc` may not simply dismiss.** Behind the debrief stands a finished
        // sortie and no game to go back to, so "back" is forward: the mission list, which is
        // where the loop starts over. `menu::debrief::close_the_debrief` is what ends the
        // sortie, and it does not care which of the three doors was used.
        Screen::Debrief => Screen::Lobby,
    };
    // Compared before it is written: `Screen` is change-detected and the HUD rebuilds its
    // visibility on `resource_changed::<Screen>` (`hud::hide_while_a_menu_is_up`). An `Esc` on
    // the title that wrote `Title` over `Title` would run that pass for nothing, every press.
    if *screen != next {
        *screen = next;
    }
}

/// **Where `Back` and `Esc` go from [`Screen::Settings`]** — the screen the options were opened
/// from.
///
/// Until 2026-08-19 the answer was the constant `Screen::Paused`, and it was right because the
/// pause screen was the only route in. The title screen is the second one, and *"`Esc` always
/// knows where back is"* is only still true if the way back is **recorded** instead of assumed.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsFrom(pub Screen);

impl Default for SettingsFrom {
    /// The pause screen, because that is the route that existed first and the one a run with no
    /// window would have taken. It is overwritten on the frame before the options open.
    fn default() -> Self {
        Self(Screen::Paused)
    }
}

/// Keeps [`SettingsFrom`] pointing at the last screen that was **not** the settings.
///
/// **One writer, and it is this system** — not the two buttons that open the options. A third
/// opener added later cannot forget to set it, which is exactly the failure mode "every opener
/// writes it itself" has: the button compiles, the screen opens, and `Esc` walks out the wrong
/// door on one route only.
fn remember_the_way_into_settings(screen: Res<Screen>, mut from: ResMut<SettingsFrom>) {
    if *screen != Screen::Settings && from.0 != *screen {
        from.0 = *screen;
    }
}

/// One line at startup saying which door the run came through.
///
/// It is the only evidence a `--headless` run can give about this domain: with no window there
/// is no menu at all (see the module docs), so "the launch reached the title" cannot be seen —
/// it can only be **said**, once, by the state that decided it.
fn announce_the_first_screen(screen: Res<Screen>) {
    info!("menu: the first screen of this run is {:?}", *screen);
}

/// Makes the world match [`Screen`]: the pointer, and whether time runs.
///
/// Compares before it writes. Not to save the cycles — there is one window — but because
/// `bevy_winit` reacts to `Changed<CursorOptions>`
/// (`bevy_winit-0.19.0/src/system.rs:609-612`) and would otherwise re-issue a grab to the
/// compositor **every frame** for a value that never changed (§6 rule 6).
///
/// **Every screen that is not [`Screen::Playing`] frees the pointer**, and that is one line
/// rather than a list — a fifth screen added tomorrow cannot forget to hand the mouse back.
/// It was, on 2026-08-24 ([`Screen::Debrief`]), and this function needed no edit at all.
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
    //
    // ⚠️ It is also the reason `mission` reads `DeployRequest` in `Update`: a message a menu
    // sends can never be answered inside a simulation that the menu itself has stopped.
    if captured == time.is_paused() {
        if captured {
            time.unpause();
        } else {
            time.pause();
        }
    }
}

/// Takes the plate down when it no longer shows the truth.
///
/// Three ways it stops being true, and all three are one query away:
///
/// - the **screen** changed — Paused → Settings, Settings → Playing, anything;
/// - a **setting** changed while the settings screen is up, so a number on it is stale;
/// - the **choice** changed while the lobby is up, so the highlighted mission is the old one;
/// - the **session** changed while the lobby is up — the door opened, or somebody joined or
///   left, so the squad list is the old one. ⚠️ Both resources are written by systems that run
///   every tick, so both take care not to mark themselves changed for nothing; the argument is
///   on `net::session::seat_the_local_player` and `net::socket::sweep_peers`.
///
/// Rebuilding the whole plate instead of patching one `Text` is deliberate: it happens on a
/// click and never per frame (§6 rule 6), and it means every screen has exactly one place where
/// what it shows is decided — its `spawn`. A patch path would be a second one, and the two go
/// out of step the first time somebody adds a row.
fn despawn_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    settings: Res<PlayerSettings>,
    choice: Res<lobby::LobbyChoice>,
    roster: Res<crate::net::Roster>,
    host: Res<crate::net::Host>,
    roots: Query<&MenuRoot>,
    elements: Query<Entity, With<PauseElement>>,
) {
    if roots.is_empty() {
        return;
    }
    let stale = roots.iter().any(|root| root.0 != *screen)
        || (*screen == Screen::Settings && settings.is_changed())
        || (*screen == Screen::Lobby
            && (choice.is_changed() || roster.is_changed() || host.is_changed()));
    if !stale {
        return;
    }
    for e in &elements {
        // The children carry `PauseElement` too, so `despawn` is called on entities that a
        // parent may already have taken with it. `try_despawn` is the difference between
        // "the screen is gone" and a panic in a menu.
        commands.entity(e).try_despawn();
    }
}

/// Builds the overlay the current [`Screen`] wants, when there is none.
///
/// Self-healing rather than message-driven: the condition is *"this screen wants a plate and
/// there is none"*, so a menu can never be missing and can never be there twice — no matter in
/// which order the toggle and the spawn happen to run.
fn spawn_menu(
    mut commands: Commands,
    screen: Res<Screen>,
    data: Res<GameData>,
    settings: Res<PlayerSettings>,
    choice: Res<lobby::LobbyChoice>,
    roster: Res<crate::net::Roster>,
    host: Res<crate::net::Host>,
    start: Res<crate::shared::Cli>,
    phase: Res<State<MissionPhase>>,
    sortie: Query<(&Mission, &MissionClock, &KillTally, Option<&Verdict>)>,
    roots: Query<&MenuRoot>,
) {
    if roots.iter().any(|root| root.0 == *screen) {
        return;
    }
    match *screen {
        Screen::Playing => {}
        Screen::Title => title::spawn_title_screen(&mut commands),
        Screen::Paused => pause::spawn_pause_screen(&mut commands, in_a_sortie(&phase)),
        Screen::Settings => settings::spawn_settings_screen(&mut commands, &settings),
        Screen::Lobby => {
            lobby::spawn_lobby_screen(&mut commands, &data, &choice, &roster, &host, &start)
        }
        // The finished sortie is still standing — `mission::hub::open_hub` despawns it on the
        // way into the hub and not a tick earlier, precisely so this plate has something to
        // read. Read-only, over the `menu -> mission` edge that already exists for
        // `in_a_sortie` (`docs/architecture.md`).
        Screen::Debrief => debrief::spawn_debrief_screen(&mut commands, &data, sortie.iter().next()),
    }
}

/// Whether there is a sortie to abandon: everything except the hub and a run that never
/// started one.
///
/// **`menu` reads the phase and never writes it** (`docs/architecture.md`, allow list). The
/// button that acts on it sends `shared::AbandonSortie`; `mission` stays the one writer.
pub fn in_a_sortie(phase: &State<MissionPhase>) -> bool {
    let phase = *phase.get();
    phase.is_running() || phase.is_decided()
}

/// Playing, or looking at one of the four screens.
///
/// A `Resource` and not a component: this is the state of *this session's screen*, not of a
/// player. §6 rule 3 forbids putting **player** state in a resource — and it forbids it for a
/// reason that does not apply here: a second player does not get a second pause screen, he
/// gets his own `Intent`. ⚠️ The day this game is played over a network, pausing stops being
/// a thing one machine may do to the simulation — see the note on [`Screen::Paused`].
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// **The front door.** The game's name, *New Game*, *Settings*, *Quit* — the first thing a
    /// launch that named no other door shows, before a frame of the game has been simulated.
    ///
    /// ⚠️ It is a `Screen` and not a `MissionPhase`, and that is the exact opposite of the
    /// decision the hub got (`f072_the_hub_is_a_place_and_not_a_screen`) — for the same reason.
    /// The hub is a **place**: you walk in it, the pointer is captured, the clock runs. The
    /// title is a **plate over a stopped world**: nothing is walked, nothing is simulated, the
    /// pointer is free. Everything `apply_screen` decides, it decides right here without a
    /// single new branch.
    Title,
    Playing,
    /// ⚠️ **Local-only.** Pausing works by stopping `Time<Virtual>`, i.e. the whole
    /// simulation. Over a network that is not available to a client, and the pause screen
    /// there becomes an overlay that does not stop anything. Written down here rather than
    /// discovered later.
    Paused,
    /// The options. Reached from the pause screen and from nowhere else, so there is exactly
    /// one route in and `Esc` always knows where "back" is.
    Settings,
    /// **The main lobby** — pick a mission, pick a difficulty, deploy. The screen the walk-in
    /// deployment pads never had, and it starts the same sortie they do.
    ///
    /// Since 2026-08-24 it is also where the **title screen** leads: it was built, it worked,
    /// and the only route into it was the pause menu of a game that was already running
    /// (`title.rs`).
    Lobby,
    /// **The debrief** — what the sortie just did, and the way back to the lobby.
    ///
    /// It is the one screen nobody opens: it comes up with
    /// [`MissionPhase::Debrief`](crate::mission::MissionPhase::Debrief) and holds that phase
    /// open for as long as it is up, because a stopped `Time<Virtual>` runs no `FixedUpdate`
    /// and the timer that would end it lives there. See [`debrief`].
    Debrief,
}

/// **Where a run begins, decided once, out of the command line.**
///
/// `Screen` has **no `Default`** on purpose since the title screen exists. A `Default` would
/// have to be one of two answers — `Title` breaks every one of the several hundred tests that
/// build their `Cli` with `..default()`, `Playing` silently swallows the front door — and
/// `init_resource` would take it without anybody noticing which. `FromWorld` has the one thing
/// a `Default` cannot have: the run's own [`Cli`], which `src/lib.rs` inserts before the first
/// plugin is added.
///
/// ⚠️ **Here and not in a `Startup` system**, and the reason is stated narrowly because the
/// wide version is not true: `Startup` runs inside the first frame *before* `Update`, so
/// `apply_screen` would not have been late and the pointer would not have flickered. What a
/// startup writer would be late for is everything that runs **before** `Update` in that first
/// frame — `First`, `PreUpdate`, `StateTransition`, the fixed loop — and every other `Startup`
/// system, which has no ordering against it at all: [`announce_the_first_screen`] is one of
/// those, and it would have printed whatever it happened to see. `FromWorld` runs while the
/// plugin is built, so there is no frame and no schedule in which the answer is not yet there.
///
/// ⚠️ **What is NOT claimed:** `apply_screen` stops the clock in `Update`, and the first
/// frame's fixed loop runs before it. It steps that frame's delta, which at startup is ~0 —
/// `tests/menu.rs::f175_the_title_lets_no_frame_of_the_game_run` measured 0 ticks there and 29
/// over five frames once the screen was gone, so the pause is what holds, not the ordering.
impl FromWorld for Screen {
    fn from_world(world: &mut World) -> Self {
        // `expect` and not a fallback: `MenuPlugin` is only ever added by `crate::app`, which
        // inserts `Cli` first (`src/lib.rs`). A quiet default here would be a second, wrong
        // answer to "where does this run start" in exactly the case nobody tests.
        let start = world.get_resource::<crate::shared::Cli>().expect(
            "menu::MenuPlugin needs Cli — it is inserted in crate::app before any plugin",
        );
        if start.title {
            Screen::Title
        } else {
            Screen::Playing
        }
    }
}

/// On **every** node this domain spawns, containers included — the whole overlay is despawned
/// by this marker, whichever screen built it.
///
/// The name is older than the second and third screens and is kept on purpose: `tests/menu.rs`
/// is the guard over the pointer and renaming a marker in it would have meant touching tests
/// that must be able to say they did not move.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PauseElement;

/// On the **root** node only: which screen this plate was built for.
///
/// It is what makes [`spawn_menu`] and [`despawn_menu`] a pair instead of six systems — "is
/// what is on screen still the screen we are on" is one comparison, and a screen added later
/// answers it without touching either of them.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuRoot(pub Screen);

/// What a button on the pause screen does.
///
/// `Quit` is quit **to desktop** and keeps its old name: it is the one action that was here
/// before the other four and `tests/menu.rs::f175_the_quit_button_ends_the_run` names it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseAction {
    Resume,
    /// Open [`Screen::Settings`].
    Settings,
    /// **Give the sortie up** and stand in the hub again. Only on screen inside a sortie —
    /// there is nothing to abandon in the hub.
    Abandon,
    /// Open [`Screen::Lobby`]. Inside a sortie that means giving it up first — the label says
    /// so ("Quit to lobby"), and the sortie is ended by the same message *Abandon* sends.
    Lobby,
    /// To desktop.
    Quit,
}
