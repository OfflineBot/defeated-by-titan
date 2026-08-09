//! **Defeated by Titan** — a 3D Titan-fighting game in Bevy.
//!
//! One domain = one folder = one plugin = standalone. What a domain may and may not do stands
//! in `docs/architecture.md`; `tests/domains.rs` falls over when somebody does not hold to it.
//!
//! ## Why the plugin list stands here and not in `main.rs`
//!
//! `prompts/init.md` §5 puts it in `main.rs`. It stands here because `tests/multiplayer.rs`
//! and `tests/domains.rs` have to build the **same** app that is actually played — otherwise
//! they check a second, similar app and prove nothing about the real one. `main.rs` stays what
//! §5 wants: read flags, have the app built, start it. **A named deviation**, because it
//! serves the purpose of the rule (one seam, one writer) instead of contradicting it.

pub mod blades;
pub mod combat;
pub mod data;
pub mod debug;
pub mod hud;
pub mod menu;
pub mod mission;
pub mod net;
pub mod player;
pub mod progress;
pub mod render;
pub mod save;
pub mod shared;
pub mod sound;
pub mod squad;
pub mod titan;
pub mod vector;
pub mod world;

use avian3d::prelude::{Gravity, PhysicsPlugins, PhysicsSystems, SubstepCount};
use bevy::app::{PluginsState, ScheduleRunnerPlugin};
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{ExitCondition, PresentMode};

use shared::{
    Impact, HookReleased, HookAnchored, IdCounter, BodyGone, Mark, SimulationSystems,
    WarpPlayer, Cli, Tick, TitanHit, SpawnTitan, Rng,
};

/// The window title. Stands in **exactly one** place — `docs/conventions.md` names the three
/// spellings of the project name and where each of them lives.
pub const WINDOW_TITLE: &str = "Defeated by Titan";

/// Builds the app that is played **and** tested.
pub fn app(start: Cli) -> App {
    let mut app = App::new();

    if !start.unknown.is_empty() {
        // Loud, not silent: a mistyped flag that gets ignored costs an hour of debugging at
        // the wrong end.
        eprintln!(
            "unknown launch arguments: {}\nknown are: --headless --offscreen --sandbox \
             --novsync --reexport --no-export --mission <name> --script <file> --lag <ms> \
             --ticks <n> --screenshot <file>",
            start.unknown.join(", ")
        );
    }

    app.insert_resource(start.clone());
    app.add_plugins(base_plugins(&start));

    // data runs BEFORE everything else: it loads the RON and crashes at startup when a value
    // is missing — instead of quietly running on a zero in the middle of the game (§4).
    app.add_plugins(data::DataPlugin);

    let game = app.world().resource::<data::GameData>().game.clone();
    app.insert_resource(Time::<Fixed>::from_hz(game.simulation_hz));

    app.init_resource::<Tick>()
        .init_resource::<IdCounter>()
        .init_resource::<Rng>()
        .add_message::<TitanHit>()
        .add_message::<SpawnTitan>()
        .add_message::<WarpPlayer>()
        .add_message::<Mark>()
        // A `MessageWriter<T>` without `add_message::<T>()` is a RUNTIME error at system init,
        // not a compile error — it only shows up once somebody writes the system, and then it
        // knocks over every test of the round. That is why all four stand here, before the
        // first sender exists (docs/interface.md, "the seam first").
        .add_message::<HookAnchored>()
        .add_message::<HookReleased>()
        .add_message::<Impact>()
        .add_message::<BodyGone>();

    // The six stages of one simulation step, configured in EXACTLY ONE place.
    //
    // Not inside a plugin: `world`, `vector` and `player` are all three members of it, and a
    // domain that fixes the order of another one is a hidden edge that walks past the
    // allow-list. `src/lib.rs` is the seam that has already been named.
    //
    // The order is the answer to "who wins": the index is current before anybody asks it;
    // asking happens before anything moves; wanting happens before forces come into being;
    // and moving happens last, done by exactly one system.
    app.configure_sets(
        FixedUpdate,
        (
            SimulationSystems::Spatial,
            SimulationSystems::World,
            SimulationSystems::Intent,
            SimulationSystems::Drive,
            SimulationSystems::Integrate,
            SimulationSystems::PostStep,
        )
            .chain(),
    );

    // ---- The physics world -------------------------------------------------------------
    //
    // avian runs in `FixedUpdate`, not in its own default `FixedPostUpdate`: the physics IS
    // the simulation step, and everything the six stages above say about "who wins" is only
    // true if the physics stands **inside** that chain and not behind it.
    app.add_plugins(PhysicsPlugins::new(FixedUpdate));

    // **All five stages, or none.** avian chains `First → Prepare → StepSimulation →
    // Writeback → Last` itself (avian3d-0.7.0/src/schedule/mod.rs:73-83), but that chain
    // says nothing about where the five sit inside OUR chain. Putting only one of them in
    // `Integrate` does **not** panic and does not warn — it silently leaves `Prepare` and
    // `Writeback` outside, and then the drive systems write into a velocity that was read
    // one stage too early. Measured, not feared.
    app.configure_sets(
        FixedUpdate,
        (
            PhysicsSystems::First,
            PhysicsSystems::Prepare,
            PhysicsSystems::StepSimulation,
            PhysicsSystems::Writeback,
            PhysicsSystems::Last,
        )
            .in_set(SimulationSystems::Integrate),
    );

    // Gravity and substeps come out of `game.ron`, not out of avian's defaults (−9.81 and 6).
    // Both are game values, and a game value that lives in two places is wrong in one of
    // them by next week (§4).
    app.insert_resource(Gravity(Vec3::Y * game.gravity_m_s2));
    app.insert_resource(SubstepCount(game.substeps));

    // The order IS the dependency order (docs/architecture.md).
    // Nested, because `add_plugins` takes at most ~15 elements per tuple and above that
    // strikes as an unreadable trait error (docs/lessons/bevy.md).
    app.add_plugins((
        (
            save::SavePlugin,
            net::NetPlugin,
            world::WorldPlugin,
            render::RenderPlugin,
            player::PlayerPlugin,
            vector::VectorPlugin,
        ),
        (
            blades::BladesPlugin,
            titan::TitanPlugin,
            combat::CombatPlugin,
            mission::MissionPlugin,
            progress::ProgressPlugin,
            squad::SquadPlugin,
        ),
        (
            hud::HudPlugin,
            sound::SoundPlugin,
            menu::MenuPlugin,
            debug::DebugPlugin,
        ),
    ));

    // With `--screenshot` the ending belongs to `debug::screenshot`: `exit_after_ticks` would
    // write `AppExit` the moment `tick >= ticks` — that is, exactly when the screenshot is
    // triggered and BEFORE anybody has read it back off the GPU. The run ended green and
    // without a file.
    if start.ticks > 0 && start.image.is_none() {
        app.add_systems(Last, exit_after_ticks);
    }

    // ---- `Plugin::finish` — and why it stands HERE ---------------------------------------
    //
    // `App::update()` does **not** run `Plugin::finish` (`bevy_app-0.19.0/src/app.rs:165-171`;
    // `finish` is at :268 and only a runner calls it). avian creates `SolverDiagnostics` in
    // `Plugin::finish` and several of its systems take that resource as a plain, non-optional
    // `ResMut` — so without these two lines **every** test that builds this app panics with
    // "Resource does not exist" the moment avian is registered. `cargo run` was never
    // affected; `cargo test` was affected always.
    //
    // The wait loop is not decoration. `RenderPlugin::finish` does
    // `future_render_resources.0.lock().unwrap().take().unwrap()`
    // (`bevy_render-0.19.0/src/lib.rs:452-465`) — call it while the wgpu adapter request is
    // still in flight and it panics on a `None`. So we wait exactly the way
    // `ScheduleRunnerPlugin`'s runner waits (`bevy_app-0.19.0/src/schedule_runner.rs:77-85`).
    //
    // Doing it twice is not a risk: every runner in play checks `plugins_state() != Cleaned`
    // first and skips (schedule_runner.rs:78, `bevy_winit-0.19.0/src/state.rs:148`).
    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();

    app
}

/// Bevy's base kit, set up for **this** machine.
fn base_plugins(start: &Cli) -> bevy::app::PluginGroupBuilder {
    let fenster = start.wants_window().then(|| Window {
        title: WINDOW_TITLE.into(),
        // Under vsync every frame time is 16.6 ms — so "what does this cost?" measures the
        // same ceiling six times over (§11).
        present_mode: if start.novsync { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync },
        ..default()
    });

    let mut plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: fenster,
        // Without a window `OnAllClosed` would shut down immediately: zero windows are all
        // windows (docs/lessons/bevy.md).
        exit_condition: if start.wants_window() {
            ExitCondition::OnPrimaryClosed
        } else {
            ExitCondition::DontExit
        },
        ..default()
    });

    // **Two separate questions, and that is exactly the difference between `--headless` and
    // `--offscreen`:** "is there a window?" and "is there a GPU?". Until today they were the
    // same question — which made a screenshot without a window impossible on principle,
    // because `backends: None` never even looks for an adapter (`docs/QUESTIONS.md` Q-009).
    if !start.wants_window() {
        // Without a window there is no event loop driving the app.
        plugins = plugins.add(ScheduleRunnerPlugin::run_loop(
            core::time::Duration::from_secs_f64(1.0 / 240.0),
        ));
        #[cfg(any(feature = "x11", feature = "wayland"))]
        {
            // WinitPlugin builds an event loop at startup and panics without a display.
            plugins = plugins.disable::<bevy::winit::WinitPlugin>();
        }
    }

    if !start.has_gpu() {
        // `backends: None` means: wgpu does not look for an adapter at all. Without it,
        // startup falls over deep inside wgpu on a machine with no GPU driver.
        //
        // Having no window, by contrast, is NO reason to switch the adapter off: when the
        // renderer is built the window is an `Option` and is simply absent
        // (`bevy_render-0.19.0/src/lib.rs:501-506`, `compatible_surface: None`).
        plugins = plugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                backends: None,
                ..default()
            })),
            ..default()
        });
    }

    plugins
}

/// `--ticks n`: exit after n simulation steps.
///
/// That gives a run without a window an ending **always** — even when a script hangs in a loop
/// or none was given at all. A test run that never comes back is worse than one that fails.
fn exit_after_ticks(tick: Res<Tick>, start: Res<Cli>, mut exit: MessageWriter<AppExit>) {
    if tick.0 >= start.ticks {
        info!("--ticks {} reached, exiting", start.ticks);
        exit.write(AppExit::Success);
    }
}
