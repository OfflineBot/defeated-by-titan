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
    MovementState, LookOverride, IntentSystems, Gas, LocalPlayer, Mark, PlayerId,
    WarpPlayer, Cli, Velocity, Tick, TitanId, SpawnTitan,
};
use script::{Instruction, ScriptCommand, Metric};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        let start = app.world().get_resource::<Cli>().cloned().unwrap_or_default();

        app.init_resource::<ScriptRun>()
            .add_systems(FixedPreUpdate, run_script.in_set(IntentSystems::Source))
            .add_systems(FixedPostUpdate, nan_guard);

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
        (&'static PlayerId, &'static Transform, &'static Gas, &'static Velocity),
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
                if let Some((id, _, _, _)) = world.players.iter().next() {
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
        _ => {
            let (_, transform, gas, tempo) = world.players.iter().next()?;
            Some(match metric {
                Metric::Height => transform.translation.y,
                Metric::Gas => gas.current,
                Metric::Speed => tempo.speed_m_s(),
                Metric::Titans | Metric::Tick => unreachable!("high behandelt"),
            })
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

/// The F3 overlay: position, look, speed, gas, state, tick — **in the image**.
///
/// That makes every report reproducible: the user sends a coordinate, and `warp` puts you
/// exactly there (§12c). Without a window there is no overlay — then the log does the job.
#[derive(Component)]
pub struct DebugOverlay;

pub fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        DebugOverlay,
        Text::new("F3"),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    ));
}

pub fn update_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    tick: Res<Tick>,
    players: Query<(&Transform, &Gas, &MovementState), With<LocalPlayer>>,
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
        let content = match players.iter().next() {
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
        **text = content;
    }
}
