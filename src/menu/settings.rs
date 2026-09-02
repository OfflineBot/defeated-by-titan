//! The settings screen — **three pages of things a person may change about his own game.**
//!
//! > *„zudem fehlen settings."* — the user, 2026-08-13 (`docs/NEXT.md` §1D req 6).
//! > *„es wird zeit einstellungen für keybinds zu adden. damit man mehr einstellen kann!"* —
//! > the user, 2026-09-01, and the keybinds page is `F-172` in its first honest cut.
//!
//! | page | row | field | window |
//! |---|---|---|---|
//! | Main | Mouse sensitivity | `mouse_deg_per_px` | 0.01 – 0.60 °/px, step 0.01 |
//! | Main | Invert Y | `invert_y` | on / off |
//! | Main | Field of view | `fov_deg` | 55 – 110°, step 5 |
//! | Main | Aim assist reach | `assist_catch_pct` | 0 – 100 %, step 5 |
//! | Main | Aim assist strength | `assist_strength_pct` | 0 – 100 %, step 5 |
//! | Keybinds | Hook fire | `hook_fire` | Hold / Toggle |
//! | Keybinds | eight bind rows | `binds` | any key in `REBINDABLE_KEYS` |
//! | Crosshair | Crosshair size | `crosshair_size_pct` | 50 – 200 %, step 25 |
//! | Crosshair | Crosshair colour | `crosshair_colour` | `CROSSHAIR_COLOURS`, cycling |
//!
//! ## Rebinding is a two-click capture
//!
//! Click a key button and the row arms ([`PlayerSettings::rebinding`]); the next key that goes
//! down and is in `shared::settings::REBINDABLE_KEYS` becomes the bind. A key another action
//! holds **swaps** (`KeyBinds::set` — `F-172`'s „Konflikterkennung" in its smallest honest
//! form). Leaving the screen cancels the capture; there is no cancel key, because `Esc` is
//! already "leave the screen" one level up (`menu::keys`).
//!
//! ## Everything on these pages survives a restart — `saves/settings.ron`
//!
//! Every arm below that changes a persisted field ends the frame with
//! `shared::settings::store_settings` — **`menu` stays the one writer of `PlayerSettings` and
//! of its file** (`src/save/mod.rs`'s own header says why `save` must not do this), and
//! `PlayerSettings::from_world` reads the file back before anything else runs. The two view
//! fields (`page`, `rebinding`) never reach the file.
//!
//! ## Why every row rebuilds the whole plate
//!
//! A click writes `PlayerSettings`, `menu::despawn_menu` sees `is_changed()` and takes the
//! plate down, `menu::spawn_menu` builds it again out of the new values. One place decides what
//! this screen shows — its `spawn` — instead of two that drift apart the first time a row is
//! added. It costs a rebuild **per click**, never per frame (§6 rule 6), and it has a second
//! effect that is worth more than the tidiness: a held mouse button cannot ramp a slider,
//! because the button it is holding no longer exists. A page flip is the same mechanism with
//! no new machinery, which is why the page lives in `PlayerSettings` at all.

use bevy::prelude::*;

use super::{plate, PauseElement, Screen, SettingsFrom};
use crate::shared::settings::{
    key_label, key_name, store_settings, BindAction, HookFire, SettingsPage,
    ASSIST_CATCH_MAX_DEG, ASSIST_MAX_PCT, ASSIST_MIN_PCT, CROSSHAIR_MAX_PCT, CROSSHAIR_MIN_PCT,
    FOV_MAX_DEG, FOV_MIN_DEG, MOUSE_MAX_DEG_PER_PX, MOUSE_MIN_DEG_PER_PX,
};
use crate::shared::PlayerSettings;

/// Which way an arrow points. A sign, so the arithmetic below is one line per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    Down,
    Up,
}

impl Nudge {
    fn steps(self) -> f32 {
        match self {
            Nudge::Down => -1.0,
            Nudge::Up => 1.0,
        }
    }

    fn arrow(self) -> &'static str {
        match self {
            Nudge::Down => "-",
            Nudge::Up => "+",
        }
    }
}

/// What a button on the settings screen does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    Mouse(Nudge),
    InvertY,
    Fov(Nudge),
    /// `F-016` — how far off the crosshair the assist may catch. 0 % is free aim.
    AssistCatch(Nudge),
    /// `F-016` / `F-024` — how hard it pulls once it has a candidate. 0 % is free aim.
    AssistStrength(Nudge),
    /// Hold ↔ Toggle for the two rope triggers (user, 2026-09-01: *„oder in einstellungen
    /// einstellbar"* — this row is that clause).
    HookFire,
    /// The X's size, 50–200 % (*„größe einstellbar"*).
    CrosshairSize(Nudge),
    /// The X's Free-state colour, cycling through the table (*„und farbe auch!"*).
    CrosshairColour(Nudge),
    /// Arm the capture for one action's key (`F-172`).
    Bind(BindAction),
    /// Show another page of this same screen.
    Page(SettingsPage),
    /// One step out: a sub-page goes back to Main, Main goes back to the screen the options
    /// were opened from — the same place `Esc` goes ([`SettingsFrom`]).
    Back,
}

/// Builds the plate out of the **current** values. Called by `menu::spawn_menu`.
pub fn spawn_settings_screen(commands: &mut Commands, s: &PlayerSettings) {
    match s.page {
        SettingsPage::Main => spawn_main_page(commands, s),
        SettingsPage::Keybinds => spawn_keybinds_page(commands, s),
        SettingsPage::Crosshair => spawn_crosshair_page(commands, s),
    }
}

fn spawn_main_page(commands: &mut Commands, s: &PlayerSettings) {
    commands.spawn(plate::root(Screen::Settings, "settings")).with_children(|screen| {
        screen.spawn(plate::title("Settings"));

        row(
            screen,
            "Mouse sensitivity",
            &format!("{:.2} deg/px", s.mouse_deg_per_px),
            &format!("{MOUSE_MIN_DEG_PER_PX:.2} - {MOUSE_MAX_DEG_PER_PX:.2}"),
            SettingsAction::Mouse,
        );
        toggle_row(screen, "Invert Y", s.invert_y);
        row(
            screen,
            "Field of view",
            &format!("{:.0} deg", s.fov_deg),
            &format!("{FOV_MIN_DEG:.0} - {FOV_MAX_DEG:.0}, vertical"),
            SettingsAction::Fov,
        );

        // **The middle of the screen, kept empty.** The `hud` draws the aim assist's search
        // extent level with the crosshair — the picture of the very number two rows below —
        // and it stays on screen while this plate is up (`hud::ShowWhileTuning`). It cannot be
        // moved aside, so the plate is what makes room. It sits **here** and not anywhere else
        // because here is the middle of this column, and a centred column puts its middle on
        // the middle of the screen.
        screen.spawn(plate::centre_lane());

        // `F-016`, the two the user asked for. Their hint lines carry two things a slider
        // cannot: what 0 means, and — for the reach — what the percentage is in degrees, so
        // the number he reports back to us is one we can act on.
        row(
            screen,
            "Aim assist reach",
            &format!("{:.0} %", s.assist_catch_pct),
            &format!(
                "{ASSIST_MIN_PCT:.0} - {ASSIST_MAX_PCT:.0}, 0 = free aim — now {:.1} deg off \
                 the crosshair (max {ASSIST_CATCH_MAX_DEG:.0})",
                s.assist_catch_deg()
            ),
            SettingsAction::AssistCatch,
        );
        row(
            screen,
            "Aim assist strength",
            &format!("{:.0} %", s.assist_strength_pct),
            // It says "not wired up yet" in so many words. An honest empty corner beats a
            // control that pretends to do something — the same rule `hud` is built on.
            &format!(
                "{ASSIST_MIN_PCT:.0} - {ASSIST_MAX_PCT:.0}, 0 = free aim — how hard it snaps \
                 (F-025 is not built yet, so this is off)"
            ),
            SettingsAction::AssistStrength,
        );

        // The two sub-pages, side by side in ONE row: a second full-height row here would
        // outgrow the 720 px budget this column already fills, and the counterweight below has
        // only 64 px to give.
        screen.spawn(plate::row()).with_children(|line| {
            for (page, label) in
                [(SettingsPage::Keybinds, "Keybinds"), (SettingsPage::Crosshair, "Crosshair")]
            {
                line.spawn((
                    Name::new(format!("settings_Page_{page:?}")),
                    SettingsAction::Page(page),
                    plate::button((plate::ROW_W - plate::ROW_GAP) * 0.5, false),
                ))
                .with_child(plate::label(format!("{label}  >")));
            }
        });

        screen
            .spawn((
                Name::new("settings_Back"),
                SettingsAction::Back,
                plate::button(plate::BUTTON_W, false),
            ))
            .with_child(plate::label("Back  (Esc)"));

        // **The counterweight, and it is not decoration.** `plate::root` centres this column on
        // the screen, so `plate::centre_lane` only lands on the *screen's* middle while the
        // column above it and the column below it are the same height. Retiring the aim-spread
        // row (Q-048) once slid the plate down by half a row; the pages row above took another
        // 58 px of the same budget on 2026-09-01, so what is left here is the difference.
        //
        // The number is pinned from the outside by
        // `tests/menu.rs::f016_the_settings_screen_leaves_the_bands_lane_empty`, which measures
        // the band and every plate node and falls over the moment they touch — so a row added
        // or removed here cannot silently move the hole off the crosshair again.
        screen.spawn((
            Name::new("settings_counterweight"),
            PauseElement,
            Node { height: Val::Px(ROW_COUNTERWEIGHT_PX), width: Val::Px(plate::ROW_W), ..default() },
        ));
    });
}

/// `F-172` — the keybinds page: the hook-fire mode and the eight rebindable actions, two per
/// row so the column fits a 720 px screen with the lane still in its middle.
fn spawn_keybinds_page(commands: &mut Commands, s: &PlayerSettings) {
    commands.spawn(plate::root(Screen::Settings, "settings")).with_children(|screen| {
        screen.spawn(plate::title("Keybinds"));
        screen.spawn(plate::note(
            "click a key, then press the new one — a key already in use swaps",
        ));

        // Hold | Toggle. On the keybinds page because it is a statement about the same two
        // keys the first bind row names.
        screen.spawn(plate::row()).with_children(|line| {
            line.spawn((PauseElement, Node { width: Val::Px(plate::LABEL_W), ..default() }))
                .with_child(plate::label("Hook fire"));
            line.spawn((
                Name::new("settings_HookFire"),
                SettingsAction::HookFire,
                plate::button(plate::SPAN_W, s.hook_fire == HookFire::Toggle),
            ))
            .with_child(plate::label(match s.hook_fire {
                HookFire::Toggle => "toggle — tap fires, tap releases",
                HookFire::Hold => "hold — release lets go",
            }));
        });

        bind_pair(screen, s, BindAction::HookLeft, BindAction::HookRight);
        bind_pair(screen, s, BindAction::Dodge, BindAction::Mark);

        // The same hole as the main page: the band and the crosshair stay up over every page
        // of this screen, so every page keeps the lane.
        screen.spawn(plate::centre_lane());

        bind_pair(screen, s, BindAction::SlashLeft, BindAction::Boost);
        bind_pair(screen, s, BindAction::ReelIn, BindAction::Jump);

        screen
            .spawn((
                Name::new("settings_Back"),
                SettingsAction::Back,
                plate::button(plate::BUTTON_W, false),
            ))
            .with_child(plate::label("Back  (Esc)"));

        // The upper half carries the title, the note and the mode row; the lower half only two
        // pair rows and the button. This makes up the difference, same contract as the main
        // page's counterweight.
        screen.spawn((
            Name::new("settings_counterweight"),
            PauseElement,
            Node { height: Val::Px(KEYBINDS_COUNTERWEIGHT_PX), width: Val::Px(plate::ROW_W), ..default() },
        ));
    });
}

/// The crosshair page — size and colour (*„größe einstellbar und farbe auch!"*).
fn spawn_crosshair_page(commands: &mut Commands, s: &PlayerSettings) {
    commands.spawn(plate::root(Screen::Settings, "settings")).with_children(|screen| {
        screen.spawn(plate::title("Crosshair"));

        row(
            screen,
            "Crosshair size",
            &format!("{:.0} %", s.crosshair_size_pct),
            &format!(
                "{CROSSHAIR_MIN_PCT:.0} - {CROSSHAIR_MAX_PCT:.0} % of the base X — the \
                 middle stays empty at every size"
            ),
            SettingsAction::CrosshairSize,
        );

        screen.spawn(plate::centre_lane());

        row(
            screen,
            "Crosshair colour",
            s.crosshair_colour_name(),
            "free aim only — anchor stays cyan, cortex stays amber (they are signals)",
            SettingsAction::CrosshairColour,
        );

        screen
            .spawn((
                Name::new("settings_Back"),
                SettingsAction::Back,
                plate::button(plate::BUTTON_W, false),
            ))
            .with_child(plate::label("Back  (Esc)"));
    });
}

/// One settings row plus one `plate::root` row gap — the height the lower half of the main
/// column lost when the aim-spread row was retired (`Q-048`), minus the 58 px the pages row
/// put back on 2026-09-01. See the counterweight comment above.
const ROW_COUNTERWEIGHT_PX: f32 = 6.0;

/// What the keybinds page's lower half is short by: the title, the hint note and the mode row
/// stand above the lane against one button below it.
const KEYBINDS_COUNTERWEIGHT_PX: f32 = 86.0;

/// The label cell of one bind, narrower than [`plate::LABEL_W`] because two of them share a
/// row with their buttons.
const BIND_LABEL_W: f32 = 120.0;
/// The key button of one bind.
const BIND_KEY_W: f32 = 98.0;

/// Two binds side by side: `label [key]   label [key]`.
///
/// The armed cell says so in its own plate — it is the `chosen` state, the same lighter plate
/// every screen uses for "this one", and its label is the instruction.
fn bind_pair(
    screen: &mut ChildSpawnerCommands,
    s: &PlayerSettings,
    left: BindAction,
    right: BindAction,
) {
    screen.spawn(plate::row()).with_children(|line| {
        for action in [left, right] {
            let armed = s.rebinding == Some(action);
            line.spawn((PauseElement, Node { width: Val::Px(BIND_LABEL_W), ..default() }))
                .with_child(plate::label(action.label()));
            line.spawn((
                Name::new(format!("settings_Bind_{action:?}")),
                SettingsAction::Bind(action),
                plate::button(BIND_KEY_W, armed),
            ))
            .with_child(plate::label(if armed {
                "press...".to_string()
            } else {
                key_label(s.binds.get(action))
            }));
        }
    });
}

/// One adjustable row: `label   [-]  value  [+]`, and a dim line under it saying what the
/// window is. The window is written down because a slider that silently stops is a slider the
/// player thinks is broken.
fn row(
    screen: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    hint: &str,
    make: fn(Nudge) -> SettingsAction,
) {
    screen.spawn(plate::row()).with_children(|line| {
        line.spawn((PauseElement, Node { width: Val::Px(plate::LABEL_W), ..default() }))
            .with_child(plate::label(label.to_string()));
        // `- value +`, in that order: the value sits **between** its two arrows, so the eye
        // reads the row as one thing instead of three.
        line.spawn((
            Name::new(format!("settings_{:?}", make(Nudge::Down))),
            make(Nudge::Down),
            plate::button(plate::ARROW_W, false),
        ))
        .with_child(plate::label(Nudge::Down.arrow()));
        line.spawn((
            PauseElement,
            Node {
                width: Val::Px(plate::VALUE_W),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child(plate::label(value.to_string()));
        line.spawn((
            Name::new(format!("settings_{:?}", make(Nudge::Up))),
            make(Nudge::Up),
            plate::button(plate::ARROW_W, false),
        ))
        .with_child(plate::label(Nudge::Up.arrow()));
    });
    screen.spawn(plate::note(hint.to_string()));
}

/// The on/off row. One button that says what it currently is.
///
/// **It spans [`plate::SPAN_W`] and not a width of its own.** At 208 px the row came out 406 px
/// wide against the others' 452, and because every row is centred independently that pushed
/// this label 24 px to the right and left the toggle's edges lining up with neither the `-`
/// column nor the `+` column — four rows and no grid (FIND-092 §4,
/// `docs/images/f175-settings.png`).
///
/// **And the hint follows the value.** It used to be the constant string
/// *"mouse forward looks down"*, shown under `Invert Y: off` — which is what the setting does
/// when it is **on**. A caption that describes the state you are not in is worse than none:
/// pushing the mouse forward is `d.y < 0`, and `net::local::read_input` raises the pitch by
/// `-pitch_sign() * d.y`, so with `invert_y` off a forward push looks **up**.
fn toggle_row(screen: &mut ChildSpawnerCommands, label: &str, on: bool) {
    screen.spawn(plate::row()).with_children(|line| {
        line.spawn((PauseElement, Node { width: Val::Px(plate::LABEL_W), ..default() }))
            .with_child(plate::label(label.to_string()));
        line.spawn((
            Name::new("settings_InvertY"),
            SettingsAction::InvertY,
            plate::button(plate::SPAN_W, on),
        ))
        .with_child(plate::label(if on { "on" } else { "off" }));
    });
    screen.spawn(plate::note(if on {
        "mouse forward looks down"
    } else {
        "mouse forward looks up"
    }));
}

/// What the buttons do — **the only place a setting is written by a click** — plus the key
/// capture while a bind row is armed.
///
/// `PlayerSettings` is taken as `ResMut` and touched only when something really changes: a
/// `DerefMut` on a resource marks it changed for every reader, this system runs every frame
/// (§6 rule 6), and a changed resource is a full plate rebuild here.
///
/// Every persisted change ends with one `store_settings` — the click IS the save point, so
/// there is no "apply" button to forget and no dirty state to flush on exit.
pub fn settings_buttons(
    buttons: Query<(&Interaction, &SettingsAction)>,
    keys: Res<ButtonInput<KeyCode>>,
    back: Res<SettingsFrom>,
    mut settings: ResMut<PlayerSettings>,
    mut screen: ResMut<Screen>,
) {
    // Leaving the screen (`Esc`, or any route) forgets the view state: a capture must not
    // stay armed into the next visit, and the next visit starts on the first page. Guarded
    // reads first — `settings` may only be dereferenced mutably when something changes.
    if *screen != Screen::Settings {
        if settings.rebinding.is_some() || settings.page != SettingsPage::Main {
            settings.rebinding = None;
            settings.page = SettingsPage::Main;
        }
        return;
    }

    let mut persist = false;

    // The capture. `get_just_pressed` and not `pressed`: the arming click's own frame cannot
    // bind anything (a mouse click is not a key), and a held key binds once, not per frame.
    if let Some(action) = settings.rebinding {
        let captured = keys.get_just_pressed().find(|key| key_name(**key).is_some()).copied();
        if let Some(key) = captured {
            settings.binds.set(action, key);
            settings.rebinding = None;
            persist = true;
            info!("keybind {} = {}", action.label(), key_label(key));
        }
    }

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            SettingsAction::Mouse(n) => {
                settings.nudge_mouse(n.steps());
                persist = true;
            }
            SettingsAction::InvertY => {
                let inverted = settings.invert_y;
                settings.invert_y = !inverted;
                persist = true;
            }
            SettingsAction::Fov(n) => {
                settings.nudge_fov(n.steps());
                persist = true;
            }
            // ⚠️ **Both print.** `F-024`'s acceptance is that a change is live without a
            // restart, and the user's own reason for asking is that he wants to *test* and
            // tell us what felt best — so the value goes into the log the moment it moves.
            // One line per click, never per frame: this branch only runs on `Pressed`.
            SettingsAction::AssistCatch(n) => {
                settings.nudge_assist_catch(n.steps());
                persist = true;
                info!(
                    "aim assist reach = {:.0} % ({:.1} deg off the crosshair)",
                    settings.assist_catch_pct,
                    settings.assist_catch_deg()
                );
            }
            SettingsAction::AssistStrength(n) => {
                settings.nudge_assist_strength(n.steps());
                persist = true;
                info!("aim assist strength = {:.0} %", settings.assist_strength_pct);
            }
            SettingsAction::HookFire => {
                settings.hook_fire = match settings.hook_fire {
                    HookFire::Hold => HookFire::Toggle,
                    HookFire::Toggle => HookFire::Hold,
                };
                persist = true;
                info!("hook fire = {}", settings.hook_fire.word());
            }
            SettingsAction::CrosshairSize(n) => {
                settings.nudge_crosshair_size(n.steps());
                persist = true;
                info!("crosshair size = {:.0} %", settings.crosshair_size_pct);
            }
            SettingsAction::CrosshairColour(n) => {
                settings.cycle_crosshair_colour(n.steps() as i32);
                persist = true;
                info!("crosshair colour = {}", settings.crosshair_colour_name());
            }
            SettingsAction::Bind(bind) => {
                // Clicking the armed row again disarms it; clicking another row moves the
                // capture there. View state only — nothing to persist yet.
                settings.rebinding =
                    if settings.rebinding == Some(*bind) { None } else { Some(*bind) };
            }
            SettingsAction::Page(page) => {
                settings.page = *page;
                settings.rebinding = None;
            }
            SettingsAction::Back => {
                if settings.page != SettingsPage::Main {
                    settings.page = SettingsPage::Main;
                    settings.rebinding = None;
                } else {
                    *screen = back.0;
                }
            }
        }
    }

    if persist {
        store_settings(&settings);
    }
}
