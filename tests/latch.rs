//! `docs/NEXT.md` §5E-c (user, 2026-09-01): *„wenn ich E drücke und toggelt. und später
//! nochmal e drücke soll das seil weg "neu" raus gehen und toggeln!"*
//!
//! The claim under test, at the seam a human or a script really drives (`ButtonInput` →
//! `net::local::read_input` → `Intent` → `vector::hook`):
//!
//! - **A tap on an ANCHORED arm re-fires.** Its PRESS edge releases the old rope — *„das
//!   seil weg"*, in the press's own tick — and its key-up inside
//!   `net::local::HOOK_TAP_MAX_TICKS` (18 ticks = 0.3 s) is a fresh fire at the CURRENT aim:
//!   *„neu raus"*. Release and fresh flight sit one tap apart, one gesture.
//! - **A press held PAST the boundary is the PURE release**: the rope went at the press, the
//!   late key-up adds nothing, the arm goes home and stays home.
//!
//! ⚠️ **Why the release is on the press and not on the key-up** (`docs/QUESTIONS.md` Q-095):
//! the press cannot yet know whether it is a tap or a hold, and holding the OLD rope through
//! that decision would mean a long press re-fires the rope it was asked to drop — and it
//! would contradict `tests/input.rs::the_hook_bit_stays_latched_after_a_tap_and_hold_mode_
//! still_exists`, which pins the bit LOW two held ticks after the second press. His words
//! order the verbs the same way: the rope goes away first, the new one follows.
//!
//! The latch's own truth table is in `src/net/local.rs`'s unit tests (`--lib`); what THIS
//! file pins is the handoff across the two schedules: the falling edge really sends
//! `Anchored -> Retracting`, and the rising edge really leaves `Retracting` as a fresh
//! `Flying` in the key-up's own tick — decision 1 of `vector::hook` (`F-002`: the retract is
//! not a lockout).
//!
//! ## What the fixture varies, and what the rule reads (`docs/lessons/fixtures.md` rule 2)
//!
//! The rule (`HookLatch::feed` + `vector::hook::update_hooks`) reads: the key edges per
//! tick, the press duration against `HOOK_TAP_MAX_TICKS`, last tick's `HookState` per side,
//! and this arm's own `ArmAim` at the fire tick. The fixture varies: the press duration
//! (3 ticks against 24), the aim between the two shots (straight down against 60° of yaw at
//! a shallower dive), and the verb (tap against hold). It holds constant: the stance
//! (`scripts/f172-hook-toggle.txt`'s pavement anchor, 12 m over the boulevard — an anchor
//! that has never missed), the side (left), and `HookFire::Toggle` (the shipping default).
//! The right arm rides the identical code path through `hook_latches[1]`; `tests/input.rs`
//! pins the per-side wiring.
//!
//! **The red half (rule 5)** is the running game against the pinned pre-§5E-c binary:
//! `scripts/f172-hook-toggle.txt` ACT 2 — `assert rope == 1 — measured 0.000`, because the
//! old second tap was a plain toggle-off (`docs/FINDINGS.md` FIND-227).

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use defeated_by_titan::net::local::HOOK_TAP_MAX_TICKS;
use defeated_by_titan::shared::{
    Cli, Hook, HookArm, HookState, LocalPlayer, LookOverride, PlayerId, Side, WarpPlayer,
};

/// The real app, headless, stepped one fixed tick per `app.update()`, warped to the
/// boulevard stance with the look already straight down.
fn app_over_the_boulevard() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();
    let step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));

    let me = {
        let mut players = app.world_mut().query_filtered::<&PlayerId, With<LocalPlayer>>();
        *players.iter(app.world()).next().expect("the local player must exist")
    };
    app.world_mut().write_message(WarpPlayer {
        player: me,
        pos_x: 168.19,
        pos_y: 12.0,
        pos_z: -50.12,
    });
    look(&mut app, 0.0, -89.0);
    app.update();
    app
}

/// Sets the look through the same override a script's `look` verb uses (§12b).
fn look(app: &mut App, yaw_deg: f32, pitch_deg: f32) {
    app.world_mut().resource_mut::<LookOverride>().0 =
        Some((yaw_deg.to_radians(), pitch_deg.to_radians()));
}

fn key_q(app: &mut App, down: bool) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    if down {
        keys.press(KeyCode::KeyQ);
    } else {
        keys.release(KeyCode::KeyQ);
    }
}

fn left_arm(app: &mut App) -> HookArm {
    let mut q = app.world_mut().query_filtered::<&Hook, With<LocalPlayer>>();
    *q.iter(app.world()).next().expect("the local player must exist").arm(Side::Left)
}

fn tick(app: &App) -> u64 {
    app.world().resource::<defeated_by_titan::shared::Tick>().0
}

/// Tap `Q` (`ticks_down` ticks, under the boundary) and run until the arm anchors.
/// Returns the anchor's world point (`tip_m` — the pixel `render::rope` draws to).
fn tap_and_anchor(app: &mut App, ticks_down: usize) -> Vec3 {
    key_q(app, true);
    for _ in 0..ticks_down {
        app.update();
    }
    key_q(app, false);
    for _ in 0..90 {
        app.update();
        if matches!(left_arm(app).state, HookState::Anchored { .. }) {
            return left_arm(app).tip_m;
        }
    }
    panic!("the tap never anchored — the pavement stance is the one that never misses");
}

/// §5E-c, the whole gesture: anchored on A, aim moved, ONE tap — the tap's own tick window
/// carries the release of A and the fresh flight, and the arm ends anchored on B.
#[test]
fn a_tap_on_an_anchored_arm_releases_a_and_flies_at_b_in_the_same_tick_window() {
    let mut app = app_over_the_boulevard();
    let anchor_a = tap_and_anchor(&mut app, 3);

    // Aim somewhere else: 60° of yaw, a shallower dive — still boulevard, metres away.
    // The rope holds A meanwhile: that is the toggle holding.
    look(&mut app, 60.0, -35.0);
    app.update();
    assert!(
        matches!(left_arm(&mut app).state, HookState::Anchored { .. }),
        "moving the aim alone must not move the rope"
    );

    // THE TAP. Its press edge is the release of A — in that very tick.
    key_q(&mut app, true);
    app.update();
    let released_at = tick(&app);
    assert!(
        matches!(left_arm(&mut app).state, HookState::Retracting),
        "the re-tap's press has to release the old rope in its own tick — measured: {:?}",
        left_arm(&mut app).state
    );
    app.update(); // still held: nothing fires while the key is down
    assert!(
        matches!(left_arm(&mut app).state, HookState::Retracting | HookState::Idle),
        "held past the press, the arm only retracts — the fire waits on the key-up"
    );

    // The key-up inside the boundary is the fresh fire, in the key-up's own tick.
    key_q(&mut app, false);
    app.update();
    let refired_at = tick(&app);
    let state = left_arm(&mut app).state;
    let HookState::Flying { target_m, .. } = state else {
        panic!("the short key-up has to fire fresh at the current aim — measured: {state:?}");
    };
    assert!(
        (target_m - anchor_a).length() > 2.0,
        "the fresh shot flies at the NEW aim, not back at A: target {target_m} against A \
         {anchor_a}"
    );
    assert!(
        refired_at - released_at <= 3,
        "release of A (t={released_at}) and flight to B (t={refired_at}) have to sit inside \
         one tap's tick window — this tap was 3 ticks long"
    );

    // And it toggles: the new rope anchors and STAYS, key long up — a first tap again.
    for _ in 0..90 {
        app.update();
        if matches!(left_arm(&mut app).state, HookState::Anchored { .. }) {
            break;
        }
    }
    assert!(
        matches!(left_arm(&mut app).state, HookState::Anchored { .. }),
        "the re-fired rope has to anchor and stay"
    );
    let anchor_b = left_arm(&mut app).tip_m;
    assert!(
        (anchor_b - anchor_a).length() > 2.0,
        "anchored on B, metres from A: B {anchor_b} against A {anchor_a}"
    );
}

/// §5E-c, the second verb on the same key: a press held past `HOOK_TAP_MAX_TICKS` is the
/// PURE release — the rope goes at the press, the late key-up fires nothing, ever.
#[test]
fn a_long_press_on_an_anchored_arm_releases_and_nothing_refires() {
    let mut app = app_over_the_boulevard();
    tap_and_anchor(&mut app, 3);

    key_q(&mut app, true);
    app.update();
    assert!(
        matches!(left_arm(&mut app).state, HookState::Retracting),
        "the press releases the rope immediately, hold and tap alike — measured: {:?}",
        left_arm(&mut app).state
    );
    // Hold well past the boundary: nothing may fire while the key is down.
    for _ in 0..(HOOK_TAP_MAX_TICKS + 6) {
        app.update();
        assert!(
            !matches!(left_arm(&mut app).state, HookState::Flying { .. }),
            "nothing fires while the re-tap is held"
        );
    }
    key_q(&mut app, false);
    // A key-up past the boundary adds nothing: no flight, no anchor, ever again.
    for _ in 0..40 {
        app.update();
        let state = left_arm(&mut app).state;
        assert!(
            matches!(state, HookState::Idle | HookState::Retracting),
            "a long press is the PURE release — the key-up must not fire fresh, measured: \
             {state:?}"
        );
    }
}
