//! `F-001` — the guard over the two hooks.
//!
//! The acceptance from `docs/features.ron` reads: "Beide Haken lassen sich unabhaengig setzen
//! und loesen; Zustaende sind im HUD sichtbar." The **second half is not testable here** —
//! `hud/` draws nothing yet and `render::rope::draw_ropes` is an empty stub. What this file
//! covers is the first half, and every number it measures comes out of `assets/data/game.ron`.
//!
//! ## Two things the tests do that need a reason
//!
//! 1. **They write `AimPoint` themselves, through a system of their own.** `vector::aim::aim`
//!    (`F-002`) is built and its ray does hit — but **no body in this world carries a
//!    [`BodyId`] yet**, because `world::index::maintain_index` (`T-036a`) is still a stub.
//!    Measured on 2026-08-09 `[offlinebot]` after 181 ticks: the real `AimPoint` of a standing
//!    player is `point_m: Some((0.0, 1.5999689, -10.0))`, `anchorable: true`, **`body: None`**,
//!    and exactly 0 entities in the world carry a `BodyId`. A hook cannot hang on a carrier
//!    that has no stable id, so in the real game every shot ends as `NoAnchor` today.
//!    Writing the component from outside between two `update()` calls would not survive: `aim`
//!    runs in `SimulationSystems::World`, that is *before* `SimulationSystems::Intent`. So the
//!    injector is registered `.after(aim)` inside the same set — it stands in for the finished
//!    `F-002` + `T-036a` and delivers exactly what the `AimPoint` interface promises: a point,
//!    a carrier, and whether it holds.
//! 2. **The flight-time test moves the number in `GameData`.** A test that only measures
//!    against the file's own value stays green when somebody hard-codes exactly that value in
//!    Rust. Tripling the speed at run time and measuring again is what makes it red.
//!
//! ## Why these tests drive with `app.update()`
//!
//! The same reason as in `tests/player.rs`: `tests/multiplayer.rs` advances `Time<Fixed>` by
//! hand and runs `FixedMain` directly, and avian's step size then comes from the *generic*
//! `Time` (`avian3d-0.7.0/src/schedule/mod.rs:238-244`). `TimeUpdateStrategy::FixedTimesteps(1)`
//! makes one `App::update()` exactly one simulation step on every machine.
//!
//! The picture and the script that belong to `F-001`: `scripts/f-001-hooks.txt` and
//! `docs/images/f-001-hooks.png`.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::net::Inbox;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{
    AimPoint, ArmAim, BodyGone, BodyId, BodyMask, Buttons, Cli, Hook, HookAnchored, HookReleased,
    HookState, IdCounter, IndexEntry, Intent, LocalPlayer, MissReason, PlayerId, PrevButtons,
    ReleaseReason, RopeLength, Side, SimulationSystems, SpatialIndex, Tick,
};
use defeated_by_titan::vector::aim::aim;

// ---------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------

/// What `vector::aim` would write if it were built. See the module header.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ForcedAim(AimPoint);

/// Writes the forced point into **both** carriers: the centre ray the crosshair reads
/// ([`AimPoint`]) and the per-arm ray the hook fires at ([`ArmAim`], `F-023`).
///
/// Both, with the same value, because that is exactly what the real `vector::aim` produces
/// for a target the whole spread covers: a side ray that finds nothing anchorable falls back
/// to the centre ray. The tests in this file are about the **state machine**, not about the
/// hemisphere split — that one has its own fixture in `tests/vector_aiming.rs`, in a map with
/// real geometry. Forcing only one of the two would test which component the hook happens to
/// read instead of what it does with what it reads.
fn force_aim(mut players: Query<(&ForcedAim, &mut AimPoint, &mut ArmAim)>) {
    for (forced, mut point, mut arms) in &mut players {
        point.set_if_neq(forced.0);
        arms.set_if_neq(ArmAim { arms: [forced.0; 2] });
    }
}

/// Every message the two hooks sent, in order. Not player state — a log of the run.
#[derive(Resource, Default)]
struct HookLog {
    anchored: Vec<HookAnchored>,
    released: Vec<HookReleased>,
}

fn collect_messages(
    mut log: ResMut<HookLog>,
    mut anchored: MessageReader<HookAnchored>,
    mut released: MessageReader<HookReleased>,
) {
    log.anchored.extend(anchored.read().copied());
    log.released.extend(released.read().copied());
}

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<HookLog>();
    app.add_systems(
        FixedUpdate,
        force_aim.in_set(SimulationSystems::World).after(aim),
    );
    app.add_systems(Last, collect_messages);
    app.update(); // Startup: the city and the local player come into being
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

/// The one local player. Not `.single()` — every player is one of many (§6 rule 3).
fn me(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

fn player_id(app: &App, e: Entity) -> PlayerId {
    *app.world().get::<PlayerId>(e).expect("every player has a stable id")
}

fn hook(app: &App, e: Entity) -> Hook {
    *app.world().get::<Hook>(e).expect("every player carries both arms")
}

fn arm_state(app: &App, e: Entity, side: Side) -> HookState {
    hook(app, e).arm(side).state
}

fn tip(app: &App, e: Entity, side: Side) -> Vec3 {
    hook(app, e).arm(side).tip_m
}

/// Where the tip starts and comes home — `player.eye_height_m` above the origin between the
/// feet, the same point `vector::aim` shoots its ray from.
fn hand(app: &App, e: Entity) -> Vec3 {
    let eye = data(app).game.player.eye_height_m;
    app.world().get::<Transform>(e).expect("the player has a transform").translation
        + Vec3::Y * eye
}

/// Puts an anchorable body into the spatial index and returns its stable id.
///
/// Directly, not through `world::index::maintain_index` — that one is a stub belonging to
/// `T-036a`. High ids so that nothing collides with the city once it is filled.
fn put_body(app: &mut App, id: u32, center_m: Vec3, half_size_m: Vec3) -> BodyId {
    let body = BodyId(id);
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m,
        half_size_m,
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    body
}

/// Aims the player at a point on a body. `anchorable` is the `F-003` tag.
fn aim_at(app: &mut App, e: Entity, point_m: Vec3, body: BodyId, anchorable: bool) {
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(point_m),
        body: Some(body),
        anchorable,
    }));
}

fn aim_at_nothing(app: &mut App, e: Entity) {
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint::default()));
}

/// The key that fires the hook on `side` — the **one** place in this file that knows a binding.
///
/// ⚠️ Depends on `Q` -> `HOOK_LEFT` and `E` -> `HOOK_RIGHT` (`src/net/local.rs::read_input`).
/// Until 2026-08-10 the hooks sat on the mouse; `LMB`/`RMB` are the **blades** now, so pressing
/// them fired no hook at all. Rebind there, change these two lines, and the file follows.
fn hook_key(side: Side) -> KeyCode {
    match side {
        Side::Left => KeyCode::KeyQ,
        Side::Right => KeyCode::KeyE,
    }
}

/// Presses a **real** key — the same input a human triggers and the same one the `--script`
/// driver uses (`src/net/local.rs` maps `Q`/`E` onto the two arms).
fn press(app: &mut App, side: Side) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(hook_key(side));
}

fn let_go(app: &mut App, side: Side) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(hook_key(side));
}

/// A second player, without the `LocalPlayer` marker — the way a team mate arrives later.
fn second_player(app: &mut App, pos: Vec3) -> Entity {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, pos, false);
    *world.resource_mut::<IdCounter>() = ids;
    app.update();
    e
}

/// Posts an intent for a **remote** player through the one channel. Nobody writes an `Intent`
/// straight onto a player (`src/net/mod.rs`) — not even a test.
fn post(app: &mut App, id: PlayerId, buttons: Buttons) {
    let tick = app.world().resource::<Tick>().0;
    app.world_mut()
        .resource_mut::<Inbox>()
        .push(id, Intent { buttons, tick, ..default() }, tick);
}

/// Runs until the arm reaches `Anchored`, at most `limit` ticks. Returns the tick count.
fn ticks_until_anchored(app: &mut App, e: Entity, side: Side, limit: u64) -> u64 {
    for n in 1..=limit {
        app.update();
        if arm_state(app, e, side).is_anchored() {
            return n;
        }
    }
    panic!("the hook did not anchor within {limit} ticks — it is {:?}", arm_state(app, e, side));
}

/// Runs until the arm is back at `Idle`, at most `limit` ticks. Returns the tick count.
fn ticks_until_idle(app: &mut App, e: Entity, side: Side, limit: u64) -> u64 {
    for n in 1..=limit {
        app.update();
        if arm_state(app, e, side) == HookState::Idle {
            return n;
        }
    }
    panic!("the tip did not come home within {limit} ticks — it is {:?}", arm_state(app, e, side));
}

/// Lets the player land and come to rest, so that `hand()` is a stable number.
fn settle(app: &mut App) {
    ticks(app, 180);
}

// ---------------------------------------------------------------------------------------
// 1. The trigger — an edge, not a hold
// ---------------------------------------------------------------------------------------

#[test]
fn f001_a_press_fires_once_and_holding_does_not_fire_again() {
    // Autofire is the difference between a hook and a machine gun. The button is held for a
    // full second here; exactly one shot may come out of that.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_001, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);

    press(&mut app, Side::Left);
    ticks(&mut app, 60); // one second of holding

    let log = app.world().resource::<HookLog>();
    assert_eq!(
        log.anchored.len(),
        1,
        "one press, {} anchor messages — the edge detection is not working",
        log.anchored.len()
    );
    assert!(
        log.released.is_empty(),
        "nothing was let go, and yet: {:?}",
        log.released
    );
    assert!(
        arm_state(&app, e, Side::Left).is_anchored(),
        "after a second of holding the left arm is {:?}",
        arm_state(&app, e, Side::Left)
    );
    assert_eq!(
        arm_state(&app, e, Side::Right),
        HookState::Idle,
        "the right arm was never touched"
    );
}

#[test]
fn f001_the_edge_belongs_to_the_player_and_not_to_the_system() {
    // THE test against `Local<Buttons>`. Player 1 holds his button from the start; player 2
    // presses the same button five ticks later. With one previous state shared by the system,
    // player 2's press falls into a `just_pressed` that already contains the button — and he
    // never fires. Each of the two has to fire exactly once, on his own edge.
    let mut app = app();
    let first = me(&mut app);
    let second = second_player(&mut app, Vec3::new(20.0, 2.0, 0.0));
    let second_id = player_id(&app, second);
    settle(&mut app);

    let body_a = put_body(&mut app, 90_002, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    let body_b = put_body(&mut app, 90_003, Vec3::new(20.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, first, Vec3::new(0.0, 1.6, -28.0), body_a, true);
    aim_at(&mut app, second, Vec3::new(20.0, 1.6, -28.0), body_b, true);

    press(&mut app, Side::Left);
    ticks(&mut app, 5);
    post(&mut app, second_id, Buttons::HOOK_LEFT);
    ticks(&mut app, 40);

    assert!(
        arm_state(&app, first, Side::Left).is_anchored(),
        "player 1 is {:?}",
        arm_state(&app, first, Side::Left)
    );
    assert!(
        arm_state(&app, second, Side::Left).is_anchored(),
        "player 2 is {:?} — his edge was eaten by player 1's held button",
        arm_state(&app, second, Side::Left)
    );

    let log = app.world().resource::<HookLog>();
    assert_eq!(log.anchored.len(), 2, "two players, two shots: {:?}", log.anchored);

    // And the previous state is really a component of each player's own.
    let first_prev = *app.world().get::<PrevButtons>(first).expect("player 1 has PrevButtons");
    let second_prev = *app.world().get::<PrevButtons>(second).expect("player 2 has PrevButtons");
    assert!(first_prev.0.contains(Buttons::HOOK_LEFT));
    assert!(second_prev.0.contains(Buttons::HOOK_LEFT));

    // Let player 2 go and check that player 1 keeps holding — one player's release must not
    // be another's.
    post(&mut app, second_id, Buttons::NONE);
    ticks(&mut app, 2);
    assert!(
        arm_state(&app, first, Side::Left).is_anchored(),
        "player 1 lost his hook when player 2 let go"
    );
    assert!(
        !arm_state(&app, second, Side::Left).is_anchored(),
        "player 2 let go and is still hanging"
    );
}

// ---------------------------------------------------------------------------------------
// 2. The flight — the number comes out of the file
// ---------------------------------------------------------------------------------------

/// Fires once at a body 28 m away and returns the number of ticks up to `Anchored`.
fn measure_flight(speed_m_s: Option<f32>) -> (u64, f32, f32) {
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    if let Some(speed) = speed_m_s {
        app.world_mut().resource_mut::<GameData>().game.vector.hook_speed_m_s = speed;
    }
    let speed = data(&app).game.vector.hook_speed_m_s;
    let hz = data(&app).game.simulation_hz as f32;

    let target = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_004, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);

    // The distance really flown: from the hand, not from the origin between the feet.
    let distance_m = (target - hand(&app, e)).length();

    press(&mut app, Side::Left);
    // The first update fires; from the second on the tip moves.
    app.update();
    assert!(
        matches!(arm_state(&app, e, Side::Left), HookState::Flying { .. }),
        "the shot did not leave: {:?}",
        arm_state(&app, e, Side::Left)
    );
    let flown = ticks_until_anchored(&mut app, e, Side::Left, 600);

    (flown, distance_m, distance_m / speed * hz)
}

#[test]
fn f001_the_flight_time_comes_out_of_the_file_and_not_out_of_the_code() {
    // The criterion verbatim: flight time = distance / hook_speed_m_s * 60, within one tick.
    // The second half of the test is what makes it red when somebody writes the 90 into the
    // Rust: at a third of the speed the flight has to take three times as long.
    let (at_file_value, distance_m, expected) = measure_flight(None);
    assert!(
        (at_file_value as f32 - expected).abs() <= 1.0,
        "{distance_m:.3} m took {at_file_value} ticks, the file allows {expected:.2} ± 1 \
         (game.ron: vector.hook_speed_m_s)"
    );

    let third = defeated_by_titan::data::GameData::load(std::path::Path::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data"),
    ))
    .game
    .vector
    .hook_speed_m_s
        / 3.0;
    let (slowed, _, expected_slow) = measure_flight(Some(third));
    assert!(
        (slowed as f32 - expected_slow).abs() <= 1.0,
        "at {third} m/s the flight took {slowed} ticks instead of {expected_slow:.2} — \
         the speed is hard-coded somewhere instead of being read from the file"
    );
    assert!(
        slowed > at_file_value * 2,
        "a third of the speed took {slowed} ticks against {at_file_value} — the number is \
         not being read at all"
    );
}

#[test]
fn f001_the_tip_starts_in_the_hand_and_flies_towards_the_anchor() {
    // Without this the flight time above would be right for a tip that teleports.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // Far enough out that the five full steps below all fit inside the flight. The **last**
    // step of any flight is a partial one — the tip stops on the anchor instead of overshooting
    // it — so a target that is not a whole number of steps away turns this into a measurement
    // of the arrival. It was: at `hook_speed_m_s: 160` the old fixed 28 m was 10.5 steps, and
    // `W1`'s 160 -> 500 m/s made it 3.4. Derived from the file now, so the next speed change
    // cannot do it again.
    let per_tick_m = data(&app).game.vector.hook_speed_m_s / data(&app).game.simulation_hz as f32;
    let target = Vec3::new(0.0, 1.6, -per_tick_m * 8.0);
    let body =
        put_body(&mut app, 90_005, Vec3::new(0.0, 1.6, -per_tick_m * 8.0 - 4.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);

    let start = hand(&app, e);
    press(&mut app, Side::Left);
    app.update();
    assert!(
        (tip(&app, e, Side::Left) - start).length() < 1e-3,
        "the tip starts at {:?} instead of in the hand at {start:?}",
        tip(&app, e, Side::Left)
    );

    let mut last = tip(&app, e, Side::Left);
    for _ in 0..5 {
        app.update();
        let now = tip(&app, e, Side::Left);
        let step = (now - last).length();
        assert!(
            (step - per_tick_m).abs() < 1e-3,
            "the tip moved {step:.4} m in one tick, the file says {per_tick_m:.4} m"
        );
        assert!(
            (target - now).length() < (target - last).length(),
            "the tip is not getting closer to the anchor"
        );
        last = now;
    }
}

// ---------------------------------------------------------------------------------------
// 3. Two arms, independently
// ---------------------------------------------------------------------------------------

#[test]
fn f001_both_hooks_anchor_and_release_independently() {
    // The acceptance of `F-001` in one test: "beide Haken lassen sich unabhaengig setzen und
    // loesen". Two different carriers, so that "independent" is not just two entries in the
    // same array.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let left_body = put_body(&mut app, 90_006, Vec3::new(-20.0, 1.6, -20.0), Vec3::splat(4.0));
    let right_body = put_body(&mut app, 90_007, Vec3::new(20.0, 1.6, -20.0), Vec3::splat(4.0));

    let left_point = Vec3::new(-16.0, 1.6, -20.0);
    aim_at(&mut app, e, left_point, left_body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    let right_point = Vec3::new(16.0, 1.6, -20.0);
    aim_at(&mut app, e, right_point, right_body, true);
    press(&mut app, Side::Right);
    ticks_until_anchored(&mut app, e, Side::Right, 300);

    assert_eq!(hook(&app, e).anchored_count(), 2, "both arms hold");
    match arm_state(&app, e, Side::Left) {
        HookState::Anchored { body, .. } => assert_eq!(body, left_body, "left hangs on the left"),
        other => panic!("the left arm is {other:?}"),
    }
    match arm_state(&app, e, Side::Right) {
        HookState::Anchored { body, .. } => {
            assert_eq!(body, right_body, "right hangs on the right")
        }
        other => panic!("the right arm is {other:?}"),
    }

    // Let go of the LEFT one only.
    let_go(&mut app, Side::Left);
    app.update();
    assert!(
        !arm_state(&app, e, Side::Left).is_anchored(),
        "the left arm did not let go"
    );
    assert!(
        arm_state(&app, e, Side::Right).is_anchored(),
        "letting the left one go took the right one with it — that is one state for two arms"
    );
    assert_eq!(hook(&app, e).anchored_count(), 1);

    let released: Vec<_> = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "exactly one release: {released:?}");
    assert_eq!(released[0].side, Side::Left);
    assert_eq!(released[0].reason, ReleaseReason::Released);

    // And now the right one, on its own.
    let_go(&mut app, Side::Right);
    app.update();
    assert_eq!(hook(&app, e).anchored_count(), 0);
    assert_eq!(app.world().resource::<HookLog>().released.len(), 2);
}

#[test]
fn f001_letting_go_brings_the_tip_home_at_the_retract_speed_from_the_file() {
    // The fourth state is not decoration: the arm is not ready again until the tip is back.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let target = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_008, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, target, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    let distance_m = (tip(&app, e, Side::Left) - hand(&app, e)).length();
    let speed = data(&app).game.vector.hook_retract_speed_m_s;
    let hz = data(&app).game.simulation_hz as f32;
    let expected = distance_m / speed * hz;

    let_go(&mut app, Side::Left);
    app.update(); // the release itself: Anchored -> Retracting
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);

    let home = ticks_until_idle(&mut app, e, Side::Left, 600);
    assert!(
        (home as f32 - expected).abs() <= 1.0,
        "the tip needed {home} ticks for {distance_m:.2} m, the file allows {expected:.2} ± 1 \
         (game.ron: vector.hook_retract_speed_m_s)"
    );
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() < 1e-3,
        "the tip came to rest at {:?} instead of in the hand",
        tip(&app, e, Side::Left)
    );
}

// ---------------------------------------------------------------------------------------
// 4. The three reasons a hook lets go without being asked
// ---------------------------------------------------------------------------------------

#[test]
fn f001_a_hook_whose_carrier_disappears_releases() {
    // `F-029`: "releases with feedback when the titan dies", `T-020`: an unloaded area. A hook
    // that keeps hanging on a body that no longer exists is a rope into nothing — and the
    // reason a hook stores a `BodyId` and never an `Entity`.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_009, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    // The carrier goes, exactly the way `world::index` reports it.
    let tick = app.world().resource::<Tick>().0;
    app.world_mut().resource_mut::<SpatialIndex>().remove(body);
    app.world_mut().write_message(BodyGone { body, tick });
    app.update();

    assert!(
        !arm_state(&app, e, Side::Left).is_anchored(),
        "the carrier is gone and the hook is still hanging on it"
    );
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    assert_eq!(
        released[0].reason,
        ReleaseReason::BodyGone,
        "the reason is not log prose — `hud` and `sound` tell from it whether the rope tore"
    );
}

#[test]
fn f001_an_overextended_rope_releases_one_tick_later() {
    // The rope is never fought against a wall: the integrator pays it out and sets the flag,
    // this system reads the flag in the NEXT tick and lets go. One tick of lag is the price
    // for `Hook` having exactly one writer.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let body = put_body(&mut app, 90_010, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    app.world_mut().get_mut::<RopeLength>(e).expect("the player has a rope").overextended
        [Side::Left.index()] = true;
    app.update();

    assert!(!arm_state(&app, e, Side::Left).is_anchored(), "the wall won and the hook held");
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    assert_eq!(released[0].reason, ReleaseReason::Overextended);
    assert_eq!(released[0].side, Side::Left);
}

#[test]
fn f001_a_shot_at_nothing_anchorable_reports_no_anchor() {
    // `F-023` in so many words: no hooking through walls. A ray that ends on an untagged wall
    // is a hit and not an anchor — and the trigger has to say so, or nobody knows why nothing
    // happened.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // 1. Nothing in range at all.
    aim_at_nothing(&mut app, e);
    press(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Idle, "a miss must not fly");
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 1, "{released:?}");
    // Since `F-028` the reason carries **why**; which of the four it is has its own test
    // below, and this one stays about the state machine.
    assert!(
        matches!(released[0].reason, ReleaseReason::NoAnchor(_)),
        "{:?}",
        released[0].reason
    );

    // 2. A hit on a body that is not tagged as an anchor surface.
    let_go(&mut app, Side::Left);
    let wall = put_body(&mut app, 90_011, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), wall, false);
    ticks(&mut app, 2);
    press(&mut app, Side::Right);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Right), HookState::Idle);
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), 2, "{released:?}");
    assert_eq!(
        released[1].reason,
        ReleaseReason::NoAnchor(MissReason::SurfaceHoldsNothing),
        "an untagged wall is a hit that holds nothing, and the player has to be told which"
    );
    assert_eq!(released[1].side, Side::Right);
    assert!(
        app.world().resource::<HookLog>().anchored.is_empty(),
        "something anchored although nothing was anchorable"
    );
}

// ---------------------------------------------------------------------------------------
// 4b. §1A requirement 1 — "festhaken … was instant sein soll"
// ---------------------------------------------------------------------------------------

#[test]
fn f002_a_refire_during_retract_flies_again_within_one_tick() {
    // The user, 2026-08-12: „wenn ich mit seilen festhake (was instant sein soll)".
    //
    // Until this test existed a shot left **only** from `Idle`, so every release opened a
    // lockout of `rope_length / vector.hook_retract_speed_m_s` in which the trigger did
    // nothing at all. Worse than the wait: the trigger pull was **swallowed**. Pressing
    // during the retract consumed the edge, and by the time the arm reached `Idle` the button
    // was already held — `just_pressed` is an edge, so the arm sat there ready and did not
    // fire until the player let go and pressed a *second* time.
    //
    // What is measured is that number: how many ticks pass between the trigger going down
    // and a shot leaving. The acceptance is **≤ 1**.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    // A long rope on purpose. At 500 m/s and 60 Hz the tip travels 8.33 m per tick, so a
    // short rope would hide the lockout inside the rounding — the bug only shows at the
    // ranges `game.ron` now allows (`hook_range_m: 500`).
    let far = Vec3::new(0.0, 1.6, -180.0);
    let body = put_body(&mut app, 90_020, Vec3::new(0.0, 1.6, -200.0), Vec3::splat(20.0));
    aim_at(&mut app, e, far, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 600);

    let distance_m = (tip(&app, e, Side::Left) - hand(&app, e)).length();
    let d = data(&app);
    let lockout_ticks =
        distance_m / d.game.vector.hook_retract_speed_m_s * d.game.simulation_hz as f32;
    assert!(
        lockout_ticks > 10.0,
        "this test measures a lockout of {lockout_ticks:.1} ticks — too short to prove \
         anything. Move the anchor further out."
    );

    // The release itself: one tick, `Anchored -> Retracting`. That tick is not the lockout,
    // it is the release, and it is the one tick the acceptance allows.
    let_go(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() > 20.0,
        "the tip is already home — then there is no retract to cancel and the test proves \
         nothing"
    );

    // A fresh target for the second shot, so that "it flew again" cannot be confused with
    // "it never let go".
    let near = Vec3::new(0.0, 1.6, -40.0);
    let second = put_body(&mut app, 90_021, Vec3::new(0.0, 1.6, -50.0), Vec3::splat(10.0));
    aim_at(&mut app, e, near, second, true);

    // The trigger goes down on the very next tick and stays down — the way a player holds it.
    press(&mut app, Side::Left);
    let mut blocked = 0u64;
    let mut flying = None;
    for _ in 0..(lockout_ticks as u64 + 30) {
        app.update();
        if let HookState::Flying { target_m, body } = arm_state(&app, e, Side::Left) {
            flying = Some((target_m, body));
            break;
        }
        if arm_state(&app, e, Side::Left).is_anchored() {
            panic!("the arm skipped `Flying` — a shot that never flies is not a shot");
        }
        blocked += 1;
    }
    let (target_m, on) = flying.unwrap_or_else(|| {
        panic!(
            "the trigger was held for {blocked} ticks and no shot left. The lockout is \
             {lockout_ticks:.1} ticks long and it swallows the edge: the arm reaches `Idle` \
             with the button already down, and `just_pressed` never comes back"
        )
    });
    assert!(
        blocked <= 1,
        "{blocked} ticks passed between the trigger going down and the shot leaving \
         (lockout: {lockout_ticks:.1} ticks). §1A requirement 1 allows 1"
    );

    // `B-001` — the state that flies still carries its carrier. A `Flying` without a `BodyId`
    // is how a miss became a flight and cost this project a day.
    assert_eq!(on, second, "the second shot flies at the second body");
    assert!(
        (target_m - near).length() < 1e-3,
        "the refire kept the OLD target {target_m:?} instead of {near:?}"
    );
    // The retract was cancelled, not queued: the new flight starts in the hand, so its flight
    // time is `distance / hook_speed_m_s` and nothing else.
    assert!(
        (tip(&app, e, Side::Left) - hand(&app, e)).length() <= d.game.vector.hook_speed_m_s
            / d.game.simulation_hz as f32
            + 1e-3,
        "the second shot started at {:?}, which is not the hand {:?}",
        tip(&app, e, Side::Left),
        hand(&app, e)
    );

    // And the other half of the same rule: a refire that finds nothing anchorable must not
    // strand the arm. It reports `NoAnchor` and keeps coming home.
    ticks_until_anchored(&mut app, e, Side::Left, 600);
    let_go(&mut app, Side::Left);
    app.update();
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Retracting);
    let before = app.world().resource::<HookLog>().released.len();
    aim_at_nothing(&mut app, e);
    press(&mut app, Side::Left);
    app.update();
    assert_eq!(
        arm_state(&app, e, Side::Left),
        HookState::Retracting,
        "a refire at nothing turned the retract into something else"
    );
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(released.len(), before + 1, "{released:?}");
    assert!(
        matches!(released[before].reason, ReleaseReason::NoAnchor(_)),
        "{:?}",
        released[before].reason
    );
}

/// `F-028` — **the four failures are four different answers, in the running game.**
///
/// The user, 2026-08-18: *„teilweise kann man gar nicht usen weil keine ahnung wieso."*
/// `B-007` is why: a titan carries no `Body`, so a ray that ends on him is a hit that holds
/// nothing — and because he is solid, the wall behind him is unreachable too. Both used to
/// arrive as the same silent `NoAnchor` with the arm sitting in `Idle`.
///
/// The three cases the acceptance names are driven here through the real message path, and
/// the pair that had no way of being told apart at all — *open sky* against *too far* — is
/// driven with a real avian collider 900 m out, against a `hook_range_m` of 500.
#[test]
fn f028_a_failed_pull_says_which_of_the_four_it_was() {
    use avian3d::prelude::{Collider, RigidBody};
    use defeated_by_titan::shared::Body;

    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);
    let range_m = data(&app).game.vector.hook_range_m;

    // ---- 1. A hit on a surface that carries no anchor. `B-007`'s titan, and every untagged
    // wall. The probe is deliberately irrelevant here: a near hit may never be talked over by
    // something further out, or "aim past him" turns into "come closer".
    let wall = put_body(&mut app, 91_001, Vec3::new(0.0, 1.6, -32.0), Vec3::splat(4.0));
    aim_at(&mut app, e, Vec3::new(0.0, 1.6, -28.0), wall, false);
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::SurfaceHoldsNothing)),
        "{released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 2. A surface that holds, with no carrier to hang it on. `B-001` was exactly this,
    // and it is a world fault, not a player error — so it may not read as one.
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(Vec3::new(0.0, 1.6, -28.0)),
        body: None,
        anchorable: true,
    }));
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::NoCarrier)),
        "{released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 3. Nothing at all on that line. High over the district, so the city is not under
    // the probe: the answer has to be "turn", not "come closer".
    app.world_mut().entity_mut(e).get_mut::<Transform>().expect("a player has a transform")
        .translation = Vec3::new(0.0, 400.0, 0.0);
    aim_at_nothing(&mut app, e);
    ticks(&mut app, 2);
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::NothingInRange)),
        "400 m over Ashgate with nothing on the line the answer has to be open sky: {released:?}"
    );
    let_go(&mut app, Side::Left);

    // ---- 4. The same empty aim, with a real anchorable wall 900 m out — beyond the file's
    // own `hook_range_m`. **This is the pair that had no way of being told apart**, and it is
    // the difference between "turn around" and "get closer".
    assert!(range_m < 900.0, "hook_range_m is {range_m} m — this test needs it under 900");
    let dir = app.world().get::<Intent>(e).expect("a player has an intent").look_dir();
    let far_m = hand(&app, e) + dir * 900.0;
    app.world_mut().spawn((
        Name::new("f028_far_wall"),
        Body { half_size_m: Vec3::splat(30.0), mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE) },
        BodyId(91_002),
        RigidBody::Static,
        Collider::cuboid(60.0, 60.0, 60.0),
        Transform::from_translation(far_m),
    ));
    ticks(&mut app, 4); // avian rebuilds its static tree inside the physics step
    press(&mut app, Side::Left);
    app.update();
    let released = app.world().resource::<HookLog>().released.clone();
    assert_eq!(
        released.last().map(|m| m.reason),
        Some(ReleaseReason::NoAnchor(MissReason::OutOfReach)),
        "an anchorable wall 900 m out against a {range_m} m reach has to read as OUT OF \
         REACH, not as open sky: {released:?}"
    );

    // And through all four the arm never left `Idle` — the feedback is the whole change, the
    // state machine is untouched (`F-029` is what would make a titan hold, and it is unbuilt).
    assert_eq!(arm_state(&app, e, Side::Left), HookState::Idle);
    assert!(
        app.world().resource::<HookLog>().anchored.is_empty(),
        "something anchored although nothing was anchorable"
    );
}

// ---------------------------------------------------------------------------------------
// 5. The anchor is stored in the carrier's frame, not in the world's
// ---------------------------------------------------------------------------------------

#[test]
fn f001_the_anchor_rides_along_when_its_carrier_moves() {
    // `HookState::Anchored` keeps `local_m` and a `BodyId`, not a world position and not an
    // `Entity`. From `F-029` on (anchors on titan limbs) that is the difference between a hook
    // that holds and one that hangs where the shoulder used to be.
    let mut app = app();
    let e = me(&mut app);
    settle(&mut app);

    let center = Vec3::new(0.0, 1.6, -32.0);
    let point = Vec3::new(0.0, 1.6, -28.0);
    let body = put_body(&mut app, 90_012, center, Vec3::splat(4.0));
    aim_at(&mut app, e, point, body, true);
    press(&mut app, Side::Left);
    ticks_until_anchored(&mut app, e, Side::Left, 300);

    match arm_state(&app, e, Side::Left) {
        HookState::Anchored { local_m, body: on } => {
            assert_eq!(on, body);
            assert!(
                (local_m - (point - center)).length() < 1e-3,
                "local_m is {local_m:?} instead of {:?}",
                point - center
            );
        }
        other => panic!("the arm is {other:?}"),
    }
    assert!((tip(&app, e, Side::Left) - point).length() < 1e-3);

    // The carrier moves five meters to the side.
    let moved = center + Vec3::X * 5.0;
    put_body(&mut app, 90_012, moved, Vec3::splat(4.0));
    app.update();
    assert!(
        (tip(&app, e, Side::Left) - (point + Vec3::X * 5.0)).length() < 1e-3,
        "the tip stayed at {:?} while its carrier moved",
        tip(&app, e, Side::Left)
    );
    assert!(arm_state(&app, e, Side::Left).is_anchored(), "and it is still holding");
}

