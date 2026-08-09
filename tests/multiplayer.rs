//! The guard over "there is no such thing as **the** player".
//!
//! It spawns **two** players and lets the simulation run for a few ticks. It falls over the
//! second somebody writes `.single()` on a player query or puts player state into a
//! `Resource` (`prompts/init.md` §6, `docs/multiplayer.md`).
//!
//! **Without it the whole multiplayer section rots quietly** — and you notice only once
//! multiplayer is up, that is, after months of work you then have to go back into.

use bevy::app::FixedMain;
use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    MovementState, Gas, IdCounter, Intent, PlayerId, Cli, Buttons, Velocity,
};

/// Builds the **real** app, headless. Not a second, similar one — otherwise the test proves
/// nothing about the game that is actually played.
fn app() -> App {
    defeated_by_titan::app(Cli { headless: true, ..default() })
}

/// Runs the fixed simulation for **exactly** `n` ticks.
///
/// Not through `app.update()`: `run_fixed_main_schedule` feeds on `Time<Virtual>.delta()`,
/// and that is overwritten in `First` from **wall clock time**. Between two `update()` calls
/// in a test only microseconds pass — so a fixed step would almost never happen, and how
/// many there were would depend on the machine's mood that day. A test whose step count
/// depends on the machine measures the machine.
///
/// So `Time<Fixed>` is advanced directly and `FixedMain` is run directly: `n` steps are `n`
/// steps, on every box.
fn ticks(app: &mut App, n: u64) {
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    for _ in 0..n {
        app.world_mut().resource_mut::<Time<Fixed>>().advance_by(timestep);
        app.world_mut().run_schedule(FixedMain);
    }
}

#[test]
fn multiplayer_two_players_simulate_independently() {
    let mut app = app();
    app.update(); // Startup: the world and the local player come into being

    // A second player, without the LocalPlayer marker — exactly the way a team mate will
    // later arrive over the network.
    let second = {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        let e = spawn_player(
            &mut commands,
            &mut ids,
            &data,
            Vec3::new(20.0, 2.0, 0.0),
            false,
        );
        *world.resource_mut::<IdCounter>() = ids;
        e
    };
    app.update();

    let mut ids: Vec<PlayerId> = app
        .world_mut()
        .query::<&PlayerId>()
        .iter(app.world())
        .copied()
        .collect();
    ids.sort_by_key(|p| p.0);
    assert_eq!(ids.len(), 2, "there must be two players in the world, not {}", ids.len());
    assert_ne!(ids[0], ids[1], "two players with the same PlayerId");

    // Only the SECOND one gets a movement request. If both have moved afterwards, they share
    // state — which is exactly what must not happen.
    {
        let mut intent = app
            .world_mut()
            .get_mut::<Intent>(second)
            .expect("the second player has an intent");
        intent.move_y = 1.0;
    }

    let before_local = position(&mut app, ids[0]);
    let before_second = app.world().get::<Transform>(second).unwrap().translation;

    ticks(&mut app, 30);

    let after_local = position(&mut app, ids[0]);
    let after_second = app.world().get::<Transform>(second).unwrap().translation;

    assert!(
        (after_second - before_second).length() > 0.5,
        "the second player should have moved: {before_second:?} -> {after_second:?}"
    );
    assert!(
        (after_local.xz() - before_local.xz()).length() < 1e-3,
        "the local player moved too, although only the second one had a request \
         ({before_local:?} -> {after_local:?}) — player state is global somewhere"
    );
}

fn position(app: &mut App, who: PlayerId) -> Vec3 {
    let mut q = app.world_mut().query::<(&PlayerId, &Transform)>();
    q.iter(app.world())
        .find(|(id, _)| **id == who)
        .map(|(_, t)| t.translation)
        .expect("the player must exist")
}

#[test]
fn multiplayer_player_state_lives_on_the_player_not_the_world() {
    // Gas, Intent, Velocity and MovementState are **components**. As a `Resource` they would
    // be global — and the game would be a single-player game that only shows it in month 12
    // (§6 rule 3).
    //
    // These lines cannot be written as a runtime check: `world.get_resource::<Gas>()`
    // **does not even compile** as long as `Gas` is not a `Resource`. Here the compiler is
    // the sharper guard than any `assert` — so the check that remains is the reverse one:
    // does the state really hang on every single player?
    let mut app = app();
    app.update();

    let with_state = app
        .world_mut()
        .query::<(&PlayerId, &Gas, &Intent, &Velocity, &MovementState)>()
        .iter(app.world())
        .count();
    let players = app.world_mut().query::<&PlayerId>().iter(app.world()).count();
    assert_eq!(
        with_state, players,
        "{players} players, but only {with_state} with their own state — something went \
         global somewhere"
    );
}

#[test]
fn multiplayer_no_player_state_is_stored_as_a_resource() {
    // The compiler catches `get_resource::<Gas>()` — but not the day somebody writes
    // `#[derive(Resource)]` onto `Gas`. That is what this check guards against, and it falls
    // over while the line is being written instead of in month 12.
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = crate_root.join("src/shared/state.rs");
    let text = std::fs::read_to_string(&state).expect("src/shared/state.rs must exist");

    for (no, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        assert!(
            !line.contains("Resource"),
            "{}:{} — shared/state.rs contains {line:?}. Gas, Blades, Velocity and \
             MovementState belong on the PLAYER, not in the world (init.md §6 rule 3)",
            state.display(),
            no + 1
        );
    }
}

#[test]
fn multiplayer_no_single_on_player_queries_in_source() {
    // The test above only falls over once `.single()` REALLY panics (that is, with two
    // players). This check falls over while the line is being written — which is cheaper.
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    let mut pending = vec![crate_root.join("src")];
    while let Some(p) = pending.pop() {
        for entry in std::fs::read_dir(&p).expect("lesbar") {
            let path = entry.expect("IndexEntry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("lesbar");
            let knows_players = text.contains("PlayerId") || text.contains("LocalPlayer");
            for (no, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                if knows_players && (code.contains(".single()") || code.contains(".single_mut()"))
                {
                    hits.push(format!(
                        "{}:{}",
                        path.strip_prefix(&crate_root).unwrap_or(&path).display(),
                        no + 1
                    ));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "`.single()` in files that know about players: {hits:?}\n\
         every player is one of many — iterate the query, and when \"me\" is really \
         meant, go through the LocalPlayer marker (init.md §6 rule 3)"
    );
}

#[test]
fn t019_lag_delays_delivery_and_loses_nothing() {
    // Bible T-019: every movement feature is checked at 200 ms of simulated latency AS WELL.
    // So the switch has to exist and to do something — not just sit in the help text.
    let mut app = defeated_by_titan::app(Cli { headless: true, lag_ms: 200, ..default() });
    app.update();

    let inbox = app.world().resource::<defeated_by_titan::net::Inbox>();
    assert_eq!(
        inbox.lag_ticks, 12,
        "200 ms at 60 Hz are 12 ticks, not {}",
        inbox.lag_ticks
    );
}

#[test]
fn multiplayer_buttons_survive_the_trip_through_the_inbox() {
    use defeated_by_titan::net::Inbox;

    let mut inbox = Inbox::with_lag(5);
    let mut buttons = Buttons::NONE;
    buttons.set(Buttons::HOOK_LEFT, true);
    buttons.set(Buttons::BOOST, true);
    inbox.push(PlayerId(7), Intent { buttons, tick: 3, ..default() }, 3);

    let out = inbox.drain_due(8);
    assert_eq!(out.len(), 1);
    assert!(out[0].1.pressed(Buttons::HOOK_LEFT));
    assert!(out[0].1.pressed(Buttons::BOOST));
    assert!(!out[0].1.pressed(Buttons::JUMP));
    assert_eq!(out[0].1.tick, 3, "the intent carries the tick it was meant for");
}

#[test]
fn multiplayer_velocity_is_a_component_on_the_player() {
    let mut app = app();
    app.update();
    let n = app.world_mut().query::<(&PlayerId, &Velocity)>().iter(app.world()).count();
    assert_eq!(n, 1, "velocity belongs on the player, not in the world");
}
