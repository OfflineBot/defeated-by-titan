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

/// ★ **The aim spread travels as an ABSOLUTE angle, and that is what makes a lost packet
/// harmless.**
///
/// `F-023`'s wheel (the user, 2026-08-12: *„mit mausrad soll man einstellen können wie weit
/// auseinander es gehen darf!"*) is the first input in this game that has **state**. A wheel
/// notch is a delta, and a delta over a wire is a desync that never re-converges: drop one
/// packet and the two machines stay one notch apart until somebody restarts the game.
///
/// So the accumulation happens on the sending side (`net::local::Spread`) and the wire carries
/// the result. This test drops the middle packet of three and shows both answers: the absolute
/// one lands on the sender's angle, the delta one is a notch short **forever**.
#[test]
fn f023_a_dropped_packet_does_not_desync_the_aim_spread() {
    use defeated_by_titan::data::GameData;
    use defeated_by_titan::net::local::Spread;
    use defeated_by_titan::net::Inbox;

    let app = app();
    let v = &app.world().resource::<GameData>().game.vector;
    let (start, step, min, max) =
        (v.aim_spread_deg, v.aim_spread_step_deg, v.aim_spread_min_deg, v.aim_spread_max_deg);

    // The sender turns the wheel one notch per tick, three times, and posts an intent each
    // time — but the middle one never makes it into the inbox.
    let mut wheel = Spread::default();
    let mut inbox = Inbox::default();
    let mut sent = Vec::new();
    for tick in 0..3u64 {
        let absolute = wheel.turn(1.0, start, step, min, max);
        sent.push(absolute);
        if tick == 1 {
            continue; // the lost packet
        }
        inbox.push(PlayerId(1), Intent { aim_spread_deg: absolute, tick, ..default() }, tick);
    }

    let delivered = inbox.drain_due(3);
    let received = delivered.last().expect("two of the three arrived").1.aim_spread_deg;
    // ⚠️ Derived from `game.ron`, **not** from `sent`: three notches up from the file's
    // starting value. Comparing the receiver against the sender's own output would be a
    // tautology — it would stay green even if `Spread` stopped accumulating altogether.
    let want = (start + 3.0 * step).clamp(min, max);
    // What a delta scheme would have made of the same two packets: one notch missing, and no
    // later packet ever repairs it.
    let as_deltas = start + 2.0 * step;

    println!(
        "sent {sent:?} — absolute delivers {received}°, a delta scheme would deliver \
         {as_deltas}° and stay {}° short forever",
        want - as_deltas
    );
    assert!(
        (received - want).abs() < 1e-6,
        "the receiver has to end on the sender's {want}°, it ended on {received}°"
    );
    assert!(
        (as_deltas - want).abs() > 1e-6,
        "this test proves nothing unless the delta scheme really would have differed"
    );
}

#[test]
fn multiplayer_velocity_is_a_component_on_the_player() {
    let mut app = app();
    app.update();
    let n = app.world_mut().query::<(&PlayerId, &Velocity)>().iter(app.world()).count();
    assert_eq!(n, 1, "velocity belongs on the player, not in the world");
}

// ───────────────────────────────────────────────────────────────────────────────────────
// The bible's four ground rules for players among players (3.6, `docs/multiplayer.md`).
// ───────────────────────────────────────────────────────────────────────────────────────

/// ★ **No collision between players** (F-163a) — *"at this speed the single biggest source of
/// frustration there is"* (`src/squad/mod.rs`).
///
/// Two bodies standing in the same spot. avian resolves an overlap by pushing both out of it,
/// so without a collision filter this is a shove — and at 75 m/s it is a shove that ends a
/// flight.
#[test]
fn f163a_two_players_in_the_same_spot_do_not_push_each_other() {
    let mut app = app();
    app.update();

    let second = {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        // 0.1 m apart, i.e. deep inside each other: the player capsule's radius is ~0.4 m.
        let e = spawn_player(&mut commands, &mut ids, &data, Vec3::new(0.1, 2.0, 0.0), false);
        *world.resource_mut::<IdCounter>() = ids;
        e
    };
    app.update();

    let ids: Vec<PlayerId> =
        app.world_mut().query::<&PlayerId>().iter(app.world()).copied().collect();
    let first = ids.iter().copied().min_by_key(|p| p.0).expect("a local player");

    let before_a = position(&mut app, first).xz();
    let before_b = app.world().get::<Transform>(second).unwrap().translation.xz();
    ticks(&mut app, 60);
    let after_a = position(&mut app, first).xz();
    let after_b = app.world().get::<Transform>(second).unwrap().translation.xz();

    let moved_a = (after_a - before_a).length();
    let moved_b = (after_b - before_b).length();
    println!(
        "a {before_a:?} -> {after_a:?} ({moved_a:.3} m), b {before_b:?} -> {after_b:?} \
         ({moved_b:.3} m)"
    );
    assert!(
        moved_a < 0.01 && moved_b < 0.01,
        "nobody asked either player to move, and they still travelled {moved_a:.3} m / \
         {moved_b:.3} m sideways — they are shoving each other (F-163a)"
    );
}

/// ★ **No damage between players** (F-162a), and it is checked at the layer and not at the
/// outcome.
///
/// `blades::cut::sweep` casts against `LAYER_TITAN_CORTEX` and `LAYER_TITAN_BODY` and against
/// nothing else. So the rule holds exactly as long as a player's collider is a member of
/// neither mask — which is a property of the player, checkable without a blade, a titan or a
/// swing.
#[test]
fn f162a_a_player_is_not_a_member_of_any_mask_a_blade_cuts() {
    use avian3d::prelude::CollisionLayers;
    use defeated_by_titan::shared::{LAYER_TITAN_BODY, LAYER_TITAN_CORTEX};

    let mut app = app();
    app.update();

    let mut q = app.world_mut().query::<(&PlayerId, &CollisionLayers)>();
    let seen: Vec<(PlayerId, u32)> =
        q.iter(app.world()).map(|(id, l)| (*id, l.memberships.0 as u32)).collect();
    assert!(!seen.is_empty(), "a player has to carry CollisionLayers for this rule to be readable");
    for (id, memberships) in seen {
        let cuttable = memberships & (LAYER_TITAN_BODY.0 as u32 | LAYER_TITAN_CORTEX.0 as u32);
        assert_eq!(
            cuttable, 0,
            "player {} is on a mask a blade casts against ({memberships:#b}) — a blade could \
             hit him (F-162a)",
            id.0
        );
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────
// The wire. `FIND-103`: a two-player test that drives both players through the same local
// code proves nothing about a network — so this half goes through a real socket.
// ───────────────────────────────────────────────────────────────────────────────────────

/// ★ **A second player, driven from outside the process, over UDP.**
///
/// Everything about this test is deliberately the long way round: the intent is **encoded to
/// bytes**, handed to the **operating system**, sent to a **port**, and read back by the game
/// through `recv_from`. Nothing in it calls a function the local player's input also calls.
/// That is the point — the older two-player test spawns its second player with
/// `player::spawn_player` and pokes his `Intent` component, which would stay green if the wire
/// did not exist at all.
///
/// It checks three things at once, and the third is the only security property this transport
/// has:
///
/// 1. an unknown address gets a **seat and a body**;
/// 2. his intents move **him** and not the player at this keyboard;
/// 3. the `PlayerId` **in** the datagram is ignored — he sends `99` and does not get it.
#[test]
fn net_a_peer_on_a_real_socket_drives_his_own_body() {
    use defeated_by_titan::net::wire::{self, Frame};
    use defeated_by_titan::net::{Host, Roster, SeatKind};
    use std::net::UdpSocket;

    // `port: Some(0)` — the OS picks a free one. A fixed port would make this test fail
    // whenever the game is running beside it, which on this machine it often is.
    let mut app =
        defeated_by_titan::app(Cli { headless: true, host: true, port: Some(0), ..default() });
    app.update(); // Startup opens the door, the first Update binds it

    let port = app
        .world()
        .resource::<Host>()
        .port()
        .expect("--host has to have bound a port by the end of the first frame");
    println!("the game is listening on 127.0.0.1:{port}");

    let local = *app
        .world_mut()
        .query_filtered::<&PlayerId, With<defeated_by_titan::shared::LocalPlayer>>()
        .iter(app.world())
        .next()
        .expect("this machine has a player");
    let local_before = position(&mut app, local);

    let peer = UdpSocket::bind("127.0.0.1:0").expect("a socket of our own");
    // ⚠️ `PlayerId(99)` is a lie the sender tells. The seat comes from the address.
    let running = Frame {
        player: PlayerId(99),
        intent: Intent { move_y: 1.0, tick: 0, ..default() },
    };

    // Send and step, forty times. UDP on loopback is quick but it is not synchronous, and a
    // test that sends once and steps once measures the kernel's mood.
    for tick in 0..40u64 {
        let mut frame = running;
        frame.intent.tick = tick;
        peer.send_to(&wire::encode(&frame), ("127.0.0.1", port))
            .expect("loopback must accept a datagram");
        ticks(&mut app, 1); // FixedPreUpdate: the socket is read here
        app.update(); // Update: `player::seat_players` builds the body
    }

    let ids: Vec<PlayerId> =
        app.world_mut().query::<&PlayerId>().iter(app.world()).copied().collect();
    assert_eq!(
        ids.len(),
        2,
        "a peer sent 40 frames and the world has {} player(s): {ids:?}",
        ids.len()
    );
    let remote = *ids.iter().find(|id| **id != local).expect("somebody who is not me");
    assert_ne!(
        remote,
        PlayerId(99),
        "the seat was taken from the datagram — a peer can claim to be anybody"
    );

    let roster = app.world().resource::<Roster>();
    assert_eq!(roster.len(), 2, "both seats belong in the roster");
    assert!(roster.get(local).expect("my seat").kind.is_local());
    assert!(
        matches!(roster.get(remote).expect("his seat").kind, SeatKind::Remote(_)),
        "the peer's seat has to remember where he is"
    );

    let remote_at = position(&mut app, remote);
    let local_after = position(&mut app, local);
    println!(
        "local {:?} -> {:?}, remote at {remote_at:?} (seat {})",
        local_before, local_after, remote.0
    );
    assert!(
        remote_at.z < -0.5,
        "he pressed forward for 40 ticks and stands at {remote_at:?} — nothing came off the wire"
    );
    assert!(
        (local_after.xz() - local_before.xz()).length() < 1e-3,
        "the player at this keyboard moved too ({local_before:?} -> {local_after:?}) — a \
         remote intent is landing on the wrong body"
    );
}

/// A UDP port is reachable by anything on the machine. Rubbish must cost a log line and not
/// the process.
#[test]
fn net_a_hostile_datagram_does_not_take_the_game_down() {
    use defeated_by_titan::net::Host;
    use std::net::UdpSocket;

    let mut app =
        defeated_by_titan::app(Cli { headless: true, host: true, port: Some(0), ..default() });
    app.update();
    let port = app.world().resource::<Host>().port().expect("bound");

    let peer = UdpSocket::bind("127.0.0.1:0").expect("a socket of our own");
    for junk in [vec![], vec![0u8], vec![0xffu8; 37], vec![1u8; 2000], vec![1u8; 36]] {
        let _ = peer.send_to(&junk, ("127.0.0.1", port));
    }
    for _ in 0..20 {
        ticks(&mut app, 1);
        app.update();
    }

    let players = app.world_mut().query::<&PlayerId>().iter(app.world()).count();
    assert_eq!(players, 1, "junk on the port must not seat anybody: {players} players");
}
