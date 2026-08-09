//! debug — the `--script` driver, `--screenshot`, the gizmos, the F3 overlay and the NaN
//! guard.
//!
//! **The tools come before the features** (`prompts/init.md` §12). Without them everything
//! is built and nothing is seen, because every feature sits behind mouse and keyboard and
//! nobody is at the keyboard.
//!
//! The driver writes into **real** `ButtonInput` resources; `net::local` then reads them
//! exactly the way it would read a human's keyboard. The order is guaranteed by
//! [`IntentSystems`](crate::shared::IntentSystems) — not by the accident of system
//! ordering.

pub mod screenshot;
pub mod gizmo;
pub mod script;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::FontSize;

use crate::shared::{
    MovementState, LookOverride, IntentSystems, Gas, Health, LocalPlayer, Mark, PlayerId,
    WarpPlayer, Cli, Velocity, Tick, TitanId, TitanState, SpawnTitan,
};
use script::{Instruction, ScriptCommand, Metric};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        let start = app.world().get_resource::<Cli>().cloned().unwrap_or_default();

        app.init_resource::<ScriptRun>()
            .add_systems(FixedPreUpdate, run_script.in_set(IntentSystems::Source))
            .add_systems(FixedPostUpdate, nan_guard);

        // The F3 overlay. It is registered **always**, exactly like the gizmos below and for
        // the same reason: it draws nothing until F3 is pressed, and a system that is not
        // registered cannot be checked at all. It compiled for weeks and hung in no schedule
        // — which is the quietest kind of dead code there is, because everything about it
        // looks finished (`tests/debug.rs::the_overlay_is_spawned_exactly_once`).
        //
        // `Update` and not `FixedUpdate`: this is presentation. It reads, it never writes a
        // simulation value.
        app.add_systems(Startup, spawn_overlay).add_systems(Update, update_overlay);

        // Gizmos are presentation, so `Update` and not the fixed step.
        //
        // They are registered **always** and drawn only when the toggle is on: a `run_if`
        // costs nothing, whereas a system that is not registered cannot be checked at all
        // — and a drawing system that is there or not depending on the launch mode is
        // exactly the kind of fake switch you first miss in the image
        // (`docs/lessons/bevy.md`).
        gizmo::install(app);
        app.configure_sets(Update, gizmo::GizmoSystems.run_if(gizmo::gizmos_on));
        app.add_systems(
            Update,
            (
                gizmo::toggle_gizmos,
                (gizmo::draw_anchors, gizmo::draw_reference, gizmo::draw_players)
                    .in_set(gizmo::GizmoSystems),
            )
                .chain(),
        );

        if let Some(path) = start.script.clone() {
            let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                // Aborting loudly at startup is the right behavior here: a run that
                // cannot find its script would otherwise end green without having done
                // anything (§9).
                panic!("--script {}: cannot be read — {e}", path.display())
            });
            let plan = script::parse(&content).unwrap_or_else(|errors| {
                let list: Vec<String> = errors.iter().map(|f| f.to_string()).collect();
                panic!(
                    "--script {}: {} line(s) not understood:\n  {}",
                    path.display(),
                    errors.len(),
                    list.join("\n  ")
                )
            });
            info!("script run {}: {} instructions", path.display(), plan.len());
            app.insert_resource(ScriptRun { plan, ..default() });
        }

        // `--screenshot`: without the flag nothing is installed here at all
        // (`debug::screenshot`).
        screenshot::install(app, &start);
    }
}

/// The state of a script run.
#[derive(Resource, Debug, Default)]
pub struct ScriptRun {
    pub plan: Vec<Instruction>,
    /// Next instruction.
    pub at: usize,
    /// Time left on the running `wait`/`key` instruction, in seconds.
    pub wait_s: f32,
    /// Keys that are held until their time runs out.
    held: Vec<(Held, f32)>,
    pub failures: Vec<String>,
    pub checked: u32,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl ScriptRun {
    pub fn is_running(&self) -> bool {
        !self.plan.is_empty() && !self.done
    }
}

/// Bundles what the driver is allowed to touch. A system takes at most ~16 parameters, and
/// beyond that it hits you as an unreadable trait error (`docs/lessons/bevy.md`).
#[derive(SystemParam)]
pub struct DriverWorld<'w, 's> {
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    look: ResMut<'w, LookOverride>,
    spawn_titan: MessageWriter<'w, SpawnTitan>,
    warp: MessageWriter<'w, WarpPlayer>,
    marks: MessageWriter<'w, Mark>,
    exit: MessageWriter<'w, AppExit>,
    players: Query<
        'w,
        's,
        (
            &'static PlayerId,
            &'static Transform,
            &'static Gas,
            &'static Velocity,
            // `Option`, because nothing spawns player health yet (`P5`). Without the `Option`
            // a missing `Health` would silently drop the player out of THIS query — and
            // `warp`, `assert speed` and `assert gas` would all stop working at once, for a
            // reason nobody would look for here.
            Option<&'static Health>,
        ),
        With<LocalPlayer>,
    >,
    titans: Query<'w, 's, &'static TitanId>,
}

/// Runs the script — one instruction per tick, except at `wait`.
fn run_script(mut run: ResMut<ScriptRun>, tick: Res<Tick>, time: Res<Time<Fixed>>, mut world: DriverWorld) {
    if !run.is_running() {
        return;
    }
    let dt = time.delta_secs();

    // Let held keys expire before new ones are added.
    run.held.retain_mut(|(command, rest)| {
        *rest -= dt;
        if *rest > 0.0 {
            return true;
        }
        match command {
            Held::Key(k) => world.keys.release(*k),
            Held::Mouse(m) => world.mouse.release(*m),
        }
        false
    });

    if run.wait_s > 0.0 {
        run.wait_s -= dt;
        return;
    }

    while run.at < run.plan.len() {
        let instruction = run.plan[run.at].clone();
        run.at += 1;
        match instruction.command {
            ScriptCommand::SpawnTitan { kind, pos } => {
                world.spawn_titan.write(SpawnTitan {
                    kind,
                    pos_x: pos.x,
                    pos_y: pos.y,
                    pos_z: pos.z,
                });
            }
            ScriptCommand::Warp(pos) => {
                if let Some((id, _, _, _, _)) = world.players.iter().next() {
                    world.warp.write(WarpPlayer {
                        player: *id,
                        pos_x: pos.x,
                        pos_y: pos.y,
                        pos_z: pos.z,
                    });
                }
            }
            ScriptCommand::Look { yaw_deg, pitch_deg } => {
                world.look.0 = Some((yaw_deg.to_radians(), pitch_deg.to_radians()));
            }
            ScriptCommand::Key { code, duration_s } => {
                world.keys.press(code);
                run.held.push((Held::Key(code), duration_s));
            }
            ScriptCommand::Hook { right, duration_s } => {
                let m = if right { MouseButton::Right } else { MouseButton::Left };
                world.mouse.press(m);
                run.held.push((Held::Mouse(m), duration_s));
            }
            ScriptCommand::Wait(s) => {
                // Commands are deferred: whatever is spawned this tick exists only at the
                // end of the tick. Without `wait` you photograph an empty field (§3).
                run.wait_s = s;
                return;
            }
            ScriptCommand::Mark(text) => {
                info!("MARK t={} {}", tick.0, text);
                world.marks.write(Mark { text, tick: tick.0 });
            }
            ScriptCommand::Assert { metric, comparison, value } => {
                let actual = measure(metric, &world, tick.0);
                run.checked += 1;
                let holds = actual.is_some_and(|i| comparison.holds(i, value));
                if !holds {
                    let message = format!(
                        "line {}: assert {metric:?} {} {value} — measured {}",
                        instruction.line,
                        comparison.symbol(),
                        actual.map_or("nothing (no player found)".to_string(), |i| format!("{i:.3}")),
                    );
                    error!("{message}");
                    run.failures.push(message);
                }
            }
            ScriptCommand::End => {
                run.at = run.plan.len();
            }
        }
    }

    if run.at >= run.plan.len() && run.held.is_empty() {
        run.done = true;
        let n = run.failures.len();
        if n == 0 {
            info!(
                "script run finished: {} asserts held, {} ticks",
                run.checked, tick.0
            );
            world.exit.write(AppExit::Success);
        } else {
            error!("script run finished: {n} of {} asserts failed", run.checked);
            for m in &run.failures {
                error!("  {m}");
            }
            world.exit.write(AppExit::error());
        }
    }
}

/// What an `assert` can measure. `None` means "not measurable" and **counts as failed** —
/// a check that found nothing is not a check that passed (§9).
fn measure(metric: Metric, world: &DriverWorld, tick: u64) -> Option<f32> {
    match metric {
        Metric::Titans => Some(world.titans.iter().count() as f32),
        Metric::Tick => Some(tick as f32),
        // ⚠️ **The one line the mission job replaces** (`F-070`/`F-071`, `docs/PLAN-GAME.md`
        // §5): the kill counter becomes a component on the `Mission` entity with per-
        // `PlayerId` counts, so read it off there and hand back that player's number. Until
        // then this is an honest zero — `assert kills >= 3` therefore fails, which is the
        // direction that cannot lie. **The parser does not have to be touched for it.**
        Metric::Kills => Some(0.0),
        Metric::Speed | Metric::Height | Metric::Gas | Metric::Health => {
            let (_, transform, gas, tempo, health) = world.players.iter().next()?;
            match metric {
                Metric::Height => Some(transform.translation.y),
                Metric::Gas => Some(gas.current),
                Metric::Speed => Some(tempo.speed_m_s()),
                // `None` and not `0.0`: a player with no `Health` component is not a player
                // at zero health, it is a player nobody has measured.
                Metric::Health => health.map(|h| h.current),
                Metric::Titans | Metric::Tick | Metric::Kills => unreachable!("handled above"),
            }
        }
    }
}

/// Warns **once** when a position is not finite.
///
/// NaN in the `Transform` is the bug that looks like "the player has vanished" — and
/// without this guard you hunt for it three systems too late (`prompts/init.md` §9d).
fn nan_guard(
    positions: Query<(Entity, &Transform), Or<(With<PlayerId>, With<TitanId>)>>,
    mut warned: Local<bool>,
) {
    if *warned {
        return;
    }
    for (e, t) in &positions {
        if !crate::shared::math::is_finite(t.translation) {
            error!(
                "position of {e:?} is not finite: {:?} — somewhere a division by zero happened \
                 or a zero vector was normalized (docs/BUGS.md §9d)",
                t.translation
            );
            *warned = true;
            return;
        }
    }
}

/// The F3 overlay: tick, position, gas, movement state — and one line per living titan.
///
/// That makes every report reproducible: the user sends a coordinate, and `warp` puts you
/// exactly there (§12c). And it is what turns a screenshot into a **measurement**: the number
/// stands next to the thing it describes, in the same PNG, at the same tick.
///
/// It hangs on the camera that carries [`IsDefaultUiCamera`](bevy::ui::IsDefaultUiCamera)
/// (`render::attach_camera`) — without that it is invisible in exactly the `--offscreen`
/// images it exists for.
#[derive(Component)]
pub struct DebugOverlay;

/// No font asset and no `Camera2d`.
///
/// `default_font` is on in `Cargo.toml`, so `FontSource::Handle(Handle::default())` — the
/// default of [`TextFont`] — resolves to the built-in `FiraMono-subset.ttf`
/// (`bevy_text-0.19.0/src/text.rs:284-291`). Two field types changed in 0.19 and will bite
/// anyone writing this from memory: `font_size` is a [`FontSize`] (`:392`, enum `:487-500`)
/// and `font` is a `FontSource` (`:383`, enum `:282-307`).
pub fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        DebugOverlay,
        Text::new("F3"),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        // White on a dark plate. Not decoration: the overlay stands over sky, over asphalt
        // and over a grey titan, and white-on-white is a number nobody can read off the
        // image — which makes the image stop being evidence.
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
    ));
}

/// Writes the overlay's text — **reads only**, one line per frame, never per entity.
///
/// The titan lines are sorted by [`TitanId`]: query iteration follows archetype order, and an
/// image whose lines swap places between two runs breaks the bit-identity that is this
/// project's only evidence route (`docs/ACCEPTANCE.md`).
pub fn update_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    tick: Res<Tick>,
    players: Query<(&Transform, &Gas, &MovementState), With<LocalPlayer>>,
    titans: Query<(&TitanId, Option<&TitanState>)>,
    mut lines: Query<(&mut Text, &mut Node), With<DebugOverlay>>,
    mut visible: Local<bool>,
) {
    if keys.just_pressed(KeyCode::F3) {
        *visible = !*visible;
    }
    for (mut text, mut node) in &mut lines {
        node.display = if *visible { Display::Flex } else { Display::None };
        if !*visible {
            continue;
        }
        let mut content = match players.iter().next() {
            Some((t, gas, state)) => format!(
                "t={}  pos {:.1} {:.1} {:.1}  gas {:.0}/{:.0}  {:?}",
                tick.0,
                t.translation.x,
                t.translation.y,
                t.translation.z,
                gas.current,
                gas.max,
                state
            ),
            None => format!("t={}  (no local player)", tick.0),
        };

        let mut bodies: Vec<(u32, Option<TitanState>)> =
            titans.iter().map(|(id, state)| (id.0, state.copied())).collect();
        bodies.sort_unstable_by_key(|(id, _)| *id);
        for (id, state) in bodies {
            // `None` is printed and not skipped: a titan without a `TitanState` is a hole in
            // the FSM (`F-050`), and a line that quietly disappears is the reason nobody
            // finds it.
            content.push_str(&match state {
                Some(s) => format!("\ntitan#{id} {s:?}"),
                None => format!("\ntitan#{id} (no state)"),
            });
        }

        **text = content;
    }
}
