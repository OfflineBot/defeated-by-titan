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

// `debug -> mission`: the F3 overlay and `assert phase|kills` read the mission's phase and its
// kill counter. There is no message to read them from — they are *state*, not an event, and a
// tool that has to see the state of a running game cannot be served by a message that fired
// three ticks ago. The edge has its line with this reason in `docs/architecture.md`.
use crate::data::GameData;
use crate::mission::{KillTally, MissionPhase};
// `debug -> player`: the F3 overlay has to print whether the player is FLYING, and that is not
// a component anywhere — it is `player::locomotion::in_flight` over
// `player::integrator::ground_top_speed_m_s` (`FIND-050`). A message cannot serve it (it is a
// predicate over the current state, not an event), and mirroring the answer into a `shared`
// component would give the overlay a second writer of the player's state for no reason other
// than a text line. Read-only, and the derivation stays `player`'s. See `docs/architecture.md`.
use crate::player::{integrator::ground_top_speed_m_s, locomotion::in_flight};
use crate::shared::{
    MovementState, LookOverride, IntentSystems, Blades, Gas, Health, Hook, LocalPlayer, Mark,
    PlayerId, PlayerSettings,
    StateClock, WarpPlayer, Cli, Velocity, Tick, TitanId, TitanKindName, TitanState, SpawnTitan,
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

    /// Whether a script was given at all. A run without one cannot be cut off in the middle
    /// of anything — `--headless --ticks 600` with no `--script` is a plain simulation run.
    pub fn has_script(&self) -> bool {
        !self.plan.is_empty()
    }

    /// Instructions that were never executed.
    ///
    /// Zero does **not** mean the script finished: the last instruction may be a `wait` that
    /// is still counting down, and until [`ScriptRun::done`] is set no summary has been
    /// printed and no verdict exists. That is why [`cutoff_verdict`] asks `done` and uses
    /// this number only for the message.
    pub fn left(&self) -> usize {
        self.plan.len().saturating_sub(self.at)
    }
}

/// **The verdict of a run that `--ticks` is cutting off right now** — the one place that
/// decides whether `--ticks n` ends green.
///
/// It exists because `--ticks` used to write [`AppExit::Success`] unconditionally
/// (`src/lib.rs::exit_after_ticks`), so a run whose tick limit fell **before** the script
/// reached its end reported success with red asserts in its own log:
///
/// ```text
/// --headless --script scripts/f-001-hooks.txt --ticks 400   -> exit 0, 2 asserts red
/// --headless --script scripts/f-001-hooks.txt --ticks 2000  -> exit 1, "2 of 14 asserts failed"
/// ```
///
/// That is `docs/HANDOVER.md` §2.2 with a different cause: a script that reports success for
/// something it did not show. Two facts decide, and they are **not** the same fact:
///
/// 1. **An assert failed.** Then the run is red, always, under every flag combination. This
///    is the invariant — nothing below may soften it.
/// 2. **The script did not reach its end.** Then the run has not *demonstrated* what the
///    script claims, whatever the asserts that did run said, and the summary line that a
///    reader looks for was never printed. So it is red too — but with its own, distinct
///    message, because the fix is a bigger `--ticks` and not a bug in the game.
///
/// **A screenshot run is not affected and must not be.** `--screenshot` cuts a script short
/// at a chosen tick on purpose, and `src/lib.rs` does not even register the `--ticks` exit
/// then — the ending belongs to `debug::screenshot`, which waits for the PNG. (That path has
/// a hole of its own; it is written up as a finding, not fixed here.)
pub fn cutoff_verdict(run: &ScriptRun, tick: u64) -> AppExit {
    if !run.has_script() {
        info!("--ticks {tick} reached, exiting");
        return AppExit::Success;
    }

    if run.done {
        // `run_script` printed the summary and wrote the exit in the tick it finished. This
        // only repeats its verdict, so that a limit landing on the same tick cannot turn it.
        return if run.failures.is_empty() { AppExit::Success } else { AppExit::error() };
    }

    let failed = run.failures.len();
    // Two different ways to be unfinished, and the difference is what a reader needs: either
    // whole instructions were never reached, or the last one is still running (a `wait`, or a
    // key still held down). Both mean the summary line was never printed.
    let how = match run.left() {
        0 => format!(
            "instruction {} of {} is still running",
            run.at.min(run.plan.len()),
            run.plan.len()
        ),
        n => format!("{n} of {} instructions never ran", run.plan.len()),
    };
    error!(
        "script did not finish: cut off at tick {tick} — {how}, {} asserts checked, \
         {failed} failed. This run has NOT shown what the script claims; raise --ticks.",
        run.checked,
    );
    for m in &run.failures {
        error!("  {m}");
    }
    AppExit::error()
}

/// Bundles what the driver is allowed to touch. A system takes at most ~16 parameters, and
/// beyond that it hits you as an unreadable trait error (`docs/lessons/bevy.md`).
#[derive(SystemParam)]
pub struct DriverWorld<'w, 's> {
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    look: ResMut<'w, LookOverride>,
    /// This machine's preferences — what `settings <key> <value>` moves.
    ///
    /// `ResMut` and not `Option<ResMut>`: `menu::MenuPlugin` calls `init_resource` outside its
    /// window gate, so the resource exists in **every** launch mode including `--headless`. An
    /// `Option` here would turn a missing resource into a `settings` line that quietly does
    /// nothing, which is the one failure this driver is built to refuse.
    settings: ResMut<'w, PlayerSettings>,
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
            // Not an `Option`, and that is the same judgement as `Gas` and `Velocity` above:
            // `player::spawn_player` hangs all eight pieces of the Vector Gear on every player
            // from tick 1 on, exactly so that nothing filters on a missing one. `Health` below
            // is the documented exception, not the pattern.
            &'static Hook,
            // `Option`, because nothing spawns player health yet (`P5`). Without the `Option`
            // a missing `Health` would silently drop the player out of THIS query — and
            // `warp`, `assert speed` and `assert gas` would all stop working at once, for a
            // reason nobody would look for here.
            Option<&'static Health>,
            // Not an `Option`, for the same reason `Gas` above is not: `player::spawn_player`
            // equips the harness on every player from tick 1. `shared::Blades` and not
            // anything out of `blades/` — `debug -> blades` is not an edge the allow list of
            // `docs/architecture.md` has, and this metric does not need one.
            &'static Blades,
        ),
        With<LocalPlayer>,
    >,
    titans: Query<'w, 's, &'static TitanId>,
    /// The mission's kill counter. A `Query` and not a resource, because the counter is a
    /// component on the mission entity with one number per player (`F-096` wants that later).
    /// Empty when no mission is running — and then `assert kills` measures **nothing**, which
    /// counts as failed. That is the direction that cannot lie.
    tally: Query<'w, 's, &'static KillTally>,
    /// The mission phase. This one **always** exists: `MissionPlugin` registers the state in
    /// every launch mode, and without `--mission` it reads `Briefing` forever.
    phase: Res<'w, State<MissionPhase>>,
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
                if let Some((id, ..)) = world.players.iter().next() {
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
            // ⚠️ The ropes are on `Q`/`E` and the blades on the mouse since 2026-08-10
            // (`src/net/local.rs`, the user's scheme after the first human play session).
            // Until then `hook` pressed a mouse button — which is now a BLADE, so every script
            // in the repository was swinging a sword where it said it fired a rope, and no
            // assert could see it. The verb keeps its name; only the device under it moved.
            ScriptCommand::Hook { right, duration_s } => {
                let k = if right { KeyCode::KeyE } else { KeyCode::KeyQ };
                world.keys.press(k);
                run.held.push((Held::Key(k), duration_s));
            }
            ScriptCommand::Slash { right, duration_s } => {
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
                let holds = actual.is_ok_and(|i| comparison.holds(i, value));
                if !holds {
                    let message = format!(
                        "line {}: assert {metric:?} {} {value} — measured {}",
                        instruction.line,
                        comparison.symbol(),
                        actual.map_or_else(
                            |why| format!("nothing ({why})"),
                            |i| format!("{i:.3}")
                        ),
                    );
                    error!("{message}");
                    run.failures.push(message);
                }
            }
            // ⚠️ Applied **immediately**, not deferred: `run_script` is in `FixedPreUpdate`
            // and `vector::aim::pre_fire_aim` reads the settings in
            // `SimulationSystems::World` of the same tick, so a `settings` line bites on the
            // tick after the line — the same one-tick
            // latency `look` has. It is logged because a knob nobody can see in the run log is
            // a knob nobody can tell was set.
            ScriptCommand::Settings { key, value } => {
                key.apply(&mut world.settings, value);
                info!("settings t={} {} = {value}", tick.0, key.key());
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

/// Why a metric could not be measured. **Each missing link names itself.**
///
/// `assert kills` used to report `no player found` when it was the mission tally that was
/// missing — the `?` chain has two links and the message named the first one whatever went
/// wrong (`FIND-074`). A wrong error message is worse than none: it sends the reader to the
/// player spawn for an hour when the answer is that `--mission` was not passed.
const NO_PLAYER: &str = "no local player found";
const NO_TALLY: &str = "no mission kill tally — is this run missing --mission?";
const NO_HEALTH: &str = "the local player has no Health component";

/// What an `assert` can measure. `Err` means "not measurable" and **counts as failed** —
/// a check that found nothing is not a check that passed (§9) — and it carries the reason,
/// which is printed instead of the number.
fn measure(metric: Metric, world: &DriverWorld, tick: u64) -> Result<f32, &'static str> {
    match metric {
        Metric::Titans => Ok(world.titans.iter().count() as f32),
        Metric::Tick => Ok(tick as f32),
        // The line the mission job was left to fill in (`F-070`/`F-071`): the counter is a
        // component on the mission entity with per-`PlayerId` counts, so this hands back the
        // **local player's** number. Two ways to fail and two different messages: no player,
        // or no mission running. A check that found nothing is not a check that passed.
        Metric::Kills => {
            let (id, ..) = world.players.iter().next().ok_or(NO_PLAYER)?;
            let tally = world.tally.iter().next().ok_or(NO_TALLY)?;
            Ok(tally.of(*id) as f32)
        }
        // Always measurable: the state is registered in every launch mode, and "no mission" is
        // the honest answer `Briefing` (0), not a missing one.
        Metric::Phase => Ok(world.phase.get().code() as f32),
        Metric::Speed
        | Metric::Height
        | Metric::Gas
        | Metric::Health
        | Metric::Rope
        | Metric::Blades
        | Metric::Sharpness => {
            let (_, transform, gas, tempo, hook, health, blades) =
                world.players.iter().next().ok_or(NO_PLAYER)?;
            match metric {
                Metric::Height => Ok(transform.translation.y),
                Metric::Gas => Ok(gas.current),
                Metric::Speed => Ok(tempo.speed_m_s()),
                // The one metric that reads the Vector Gear itself. `Hook::anchored_count` and
                // not a count of its own: `vector::hook` is the single writer of `Hook`, and a
                // second definition of "anchored" living in a debugging tool is exactly the
                // kind of drift that makes a green run mean nothing.
                Metric::Rope => Ok(hook.anchored_count() as f32),
                // Its own reason and not `0.0`: a player with no `Health` component is not a
                // player at zero health, it is a player nobody has measured — and it is not a
                // missing player either, which is what this used to say.
                Metric::Health => health.map(|h| h.current).ok_or(NO_HEALTH),
                // The harness, straight off `shared::Blades` — the same component `hud` draws
                // its pips from, and not a second count kept here. Nothing lowers either number
                // yet (`docs/FINDINGS.md` FIND-075), which is a fact about the game and not
                // about the metric: what a script writes with it goes red the day the rack
                // stops working, and that is what an evidence line is for.
                Metric::Blades => Ok(f32::from(blades.pairs_left)),
                Metric::Sharpness => Ok(blades.sharpness),
                Metric::Titans | Metric::Tick | Metric::Kills | Metric::Phase => {
                    unreachable!("handled above")
                }
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

/// The F3 overlay: tick, position, gas, movement state, **the mission line** — and one line per
/// living titan.
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
///
/// ## Why a titan line is `husk#1 Windup 21/36` and not `titan#1 Windup`
///
/// Because `titan#1 Windup` is equally true on tick 1 and on tick 35 of a wind-up, and `F-050`'s
/// picture criterion (`docs/PLAN-GAME.md` §8) asks the image to prove that the wind-up lasts as
/// long as `titan.ron` says **while the arm is up in the same frame**. The word alone cannot
/// carry that; the fraction can, and the kind is what says which row of `titan.ron` the 36 is
/// supposed to have come from.
///
/// Both come off components in `shared/`
/// ([`TitanKindName`], [`StateClock`]) — **not** out of `titan/`, which `debug` may not read,
/// and **not** out of the entity's `Name`, which is a debugging convenience and not an
/// interface. Nothing here computes a total of its own: `titan::brain` writes both numbers on
/// the same line as the state edge, so a fraction printed here cannot be a tick out of step
/// with the pose that was built from it.
pub fn update_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    tick: Res<Tick>,
    data: Res<GameData>,
    players: Query<(&Transform, &Gas, &MovementState, &Velocity), With<LocalPlayer>>,
    titans: Query<(&TitanId, Option<&TitanKindName>, Option<&TitanState>, Option<&StateClock>)>,
    // The mission phase and its counter. Until `hud` (`F-170`) draws the objective line and the
    // word `WON`/`LOST`, **this is the only place a screenshot can show what the mission is
    // doing** — and a picture without the verdict in it is not evidence for `F-070`.
    phase: Res<State<MissionPhase>>,
    tallies: Query<&KillTally>,
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
            Some((t, gas, state, velocity)) => {
                // **The state alone is a lie and `FIND-050` says so in as many words:** a player
                // skidding across the floor at 30 m/s is `Grounded` and is, by the game's own
                // rule, in flight — his legs steer nothing, `player::locomotion::air_control`
                // does. So the variant is printed AND the verdict, because they are two
                // different facts and the overlay owns neither: `Grounded` is still what
                // `movement_state` wrote, `FLIGHT` is what `in_flight` answers about it.
                //
                // The speed stands next to them for the same reason a titan line carries
                // `21/36` instead of `Windup`: with the number in the same PNG the verdict can
                // be **read off** the image against the threshold instead of believed.
                let speed_m_s = velocity.0.xz().length();
                let flying = in_flight(*state, speed_m_s, ground_top_speed_m_s(&data));
                format!(
                    "t={}  pos {:.1} {:.1} {:.1}  gas {:.0}/{:.0}  {:?}{}  spd {speed_m_s:.1}",
                    tick.0,
                    t.translation.x,
                    t.translation.y,
                    t.translation.z,
                    gas.current,
                    gas.max,
                    state,
                    if flying { " FLIGHT" } else { "" }
                )
            }
            None => format!("t={}  (no local player)", tick.0),
        };

        // The mission line. The counter is only there while a mission runs; the phase always
        // is, and `BRIEFING` is the honest reading of "no mission was started".
        content.push_str(&match tallies.iter().next() {
            Some(tally) => format!(
                "\nmission {}  kills {}/{}",
                phase.get().label(),
                tally.total(),
                tally.target
            ),
            None => format!("\nmission {}", phase.get().label()),
        });

        // Borrowed, not cloned: this runs every frame the overlay is on, and a `String` per
        // titan per frame is exactly the kind of allocation §6 rule 6 is about.
        let mut bodies: Vec<(u32, &str, Option<TitanState>, Option<StateClock>)> = titans
            .iter()
            .map(|(id, kind, state, clock)| {
                // `titan` is the honest fallback for a body that carries a `TitanId` and no
                // kind — a test fixture, or a spawner that forgot. Printing a guessed kind
                // would be worse than printing none.
                (id.0, kind.map_or("titan", TitanKindName::as_str), state.copied(), clock.copied())
            })
            .collect();
        bodies.sort_unstable_by_key(|(id, ..)| *id);
        for (id, kind, state, clock) in bodies {
            // `None` is printed and not skipped: a titan without a `TitanState` is a hole in
            // the FSM (`F-050`), and a line that quietly disappears is the reason nobody
            // finds it.
            content.push_str(&match (state, clock) {
                // `Idle` and `Pursue` have no length, and `21/0` is not a reading — so the
                // fraction is left off rather than faked.
                (Some(s), Some(c)) if c.is_timed() => {
                    format!("\n{kind}#{id} {s:?} {}/{}", c.ticks_in_state, c.state_ticks)
                }
                (Some(s), _) => format!("\n{kind}#{id} {s:?}"),
                (None, _) => format!("\n{kind}#{id} (no state)"),
            });
        }

        **text = content;
    }
}
