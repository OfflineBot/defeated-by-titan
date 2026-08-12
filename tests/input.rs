//! The guard over the **mouse**: what the device moved is what the view turns.
//!
//! `P3` in `docs/PLAN-GAME.md` §8 — a row the backlog has no `F-ID` for. The claim it pins is
//! one sentence: **applied yaw over a run equals the raw device motion, at any frame rate.**
//!
//! ## Why this test exists and why it is not obvious
//!
//! `net::local::read_input` runs in `FixedPreUpdate` (`src/net/mod.rs`). Bevy **assigns**
//! `AccumulatedMouseMotion.delta` once per **frame**, in `PreUpdate`
//! (`bevy_input-0.19.0/src/mouse.rs:257-267` — an `=`, not a `+=`), while the fixed loop runs
//! the whole `FixedMain` schedule **0..n times per frame**
//! (`bevy_time-0.19.0/src/fixed.rs:249-255`, `while ...expend()`). So a resource that is
//! refreshed per frame is read by a schedule that runs a different number of times.
//!
//! **At exactly 60 fps the bug is invisible** — one frame, one tick, and the two rates cancel.
//! That is the whole reason it survived: the only rate anybody ever ran is the one rate at
//! which it is correct. Above 60 fps most frames' motion is thrown away; below it, one frame's
//! motion is applied two or three times.
//!
//! ## How the frame rate is set here
//!
//! `TimeUpdateStrategy::ManualDuration` (`bevy_time-0.19.0/src/lib.rs:118-119`) makes one
//! `app.update()` advance `Time<Real>` by exactly one named duration. That, and nothing else,
//! is what makes "144 fps" a number in this test instead of the mood of the machine that day.
//! `tests/multiplayer.rs`'s `ticks()` helper runs `FixedMain` **directly** and therefore cannot
//! see this bug at all: it never runs a frame.

use std::time::Duration;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{Buttons, Cli, Intent, LocalPlayer, Tick};

/// One measured run.
#[derive(Debug)]
struct Run {
    /// What the device reported, in radians of yaw.
    raw: f32,
    /// What arrived on the player's `Intent`, in radians of yaw.
    applied: f32,
    /// How many `app.update()` calls, i.e. rendered frames.
    frames: u32,
    /// How many fixed simulation steps happened inside them.
    ticks: u64,
}

impl Run {
    /// Applied divided by raw. `1.0` is right; `< 1` is motion thrown away, `> 1` is motion
    /// applied more than once.
    fn ratio(&self) -> f32 {
        self.applied / self.raw
    }

    fn report(&self, name: &str) -> String {
        format!(
            "{name}: {} frames, {} ticks — raw {:.4} rad, applied {:.4} rad, \
             ratio {:.3} ({:+.1} %)",
            self.frames,
            self.ticks,
            self.raw,
            self.applied,
            self.ratio(),
            (self.ratio() - 1.0) * 100.0
        )
    }
}

/// Drives the **real** app for one frame per entry in `frame_dts`, moving the mouse by
/// `dx_px` in every one of them.
///
/// Not a second, similar app: `defeated_by_titan::app` is what is actually played, and the
/// registration of `read_input` is half of what is under test here.
fn drive(frame_dts: &[Duration], dx_px: f32) -> Run {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });

    // The startup frame must not smuggle a fixed step in: `Time<Real>`'s first delta is wall
    // clock time since the app was built, and that is the machine's mood, not the test's.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    let deg_per_px = app.world().resource::<GameData>().game.camera.mouse_deg_per_px;

    for dt in frame_dts {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(*dt));
        // The device does not write the resource, it writes messages — and Bevy's own
        // `accumulate_mouse_motion_system` turns them into the resource in `PreUpdate`. Writing
        // `AccumulatedMouseMotion` by hand would step over exactly the system whose behaviour
        // is the bug.
        app.world_mut().write_message(MouseMotion { delta: Vec2::new(dx_px, 0.0) });
        app.update();
    }

    // **One frame of settling, with the mouse still.** A run can end in the middle of a
    // frame, with motion that is buffered and whose tick is simply not due yet — that is not
    // lost motion, and counting it as lost would be measuring the length of the run instead of
    // the bug. Two timesteps' worth of time and no motion: whatever is pending arrives, and
    // nothing new comes in. It hides nothing — with the defect in place the 144 fps run is
    // still 58 % short after it, because one extra tick cannot return 176 frames.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        2 * app.world().resource::<Time<Fixed>>().timestep(),
    ));
    app.update();

    let ticks = app.world().resource::<Tick>().0;
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let intent = *players
        .iter(app.world())
        .next()
        .expect("the local player must exist after startup");

    Run {
        // `read_input` turns a positive `delta.x` into a negative yaw (turning right).
        // The sign is not what is being measured here — the magnitude is.
        raw: -(dx_px * frame_dts.len() as f32) * deg_per_px.to_radians(),
        applied: intent.yaw,
        frames: frame_dts.len() as u32,
        ticks,
    }
}

fn fps(n: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(n))
}

/// ★ **The bug.** A run whose frames contain **no** fixed step and a run whose frames contain
/// **three** — and the total applied yaw has to come out the same as the device's.
///
/// Goes red when `read_input` reads `AccumulatedMouseMotion` straight out of `FixedPreUpdate`:
/// the four short frames' motion is dropped, and the one long frame's motion is applied three
/// times over.
#[test]
fn p3_the_applied_yaw_equals_the_device_motion() {
    // Four 4 ms frames (250 fps) and then one 40 ms frame (25 fps). At a 16.67 ms timestep
    // that is 0-0-0-0-3 fixed steps — both failure directions in one run, which is the point:
    // a test that only ran fast would be passed by a fix that only counts frames.
    let mut pattern = Vec::new();
    for _ in 0..12 {
        pattern.extend([
            Duration::from_millis(4),
            Duration::from_millis(4),
            Duration::from_millis(4),
            Duration::from_millis(4),
            Duration::from_millis(40),
        ]);
    }

    let mixed = drive(&pattern, 3.0);
    let fast = drive(&[fps(144); 300], 3.0);
    let slow = drive(&[fps(60); 300], 3.0);

    // Printed before the asserts on purpose: the number is the evidence, and a test that only
    // says "assertion failed" leaves the bug report without one.
    println!("{}", mixed.report("mixed 250/25 fps"));
    println!("{}", fast.report("144 fps"));
    println!("{}", slow.report("60 fps"));

    for run in [&mixed, &fast, &slow] {
        assert!(
            (run.ratio() - 1.0).abs() <= 0.01,
            "applied yaw is off by more than 1 % — {run:?}, ratio {:.3}",
            run.ratio()
        );
    }
}

/// The device motion of a frame in which **no** fixed step happens must not fall on the floor —
/// it belongs to the next tick.
///
/// Split out from the star test so that a failure names *which* of the two directions broke.
#[test]
fn p3_a_frame_without_a_fixed_step_keeps_its_motion() {
    // 2 ms frames: it takes nine of them before one 16.67 ms step is due, so eight frames in
    // nine have no tick at all.
    let run = drive(&[Duration::from_millis(2); 250], 3.0);
    println!("{}", run.report("500 fps"));
    assert!(run.ticks < u64::from(run.frames), "the run has to have frames without a tick");
    assert!(
        (run.ratio() - 1.0).abs() <= 0.01,
        "{:.1} % of the mouse motion was thrown away — {run:?}",
        (1.0 - run.ratio()) * 100.0
    );
}

/// A frame that carries **two or more** fixed steps must not apply its motion twice.
#[test]
fn p3_a_catch_up_frame_does_not_apply_its_motion_twice() {
    // 50 ms frames at a 16.67 ms timestep: three steps in every frame.
    let run = drive(&[Duration::from_millis(50); 60], 3.0);
    println!("{}", run.report("20 fps"));
    assert!(run.ticks > u64::from(run.frames), "the run has to have catch-up frames");
    assert!(
        (run.ratio() - 1.0).abs() <= 0.01,
        "{:.1} % more yaw was applied than the mouse moved — {run:?}",
        (run.ratio() - 1.0) * 100.0
    );
}

/// The absolute look of a script (`look 0 -10`) still wins over the mouse, and it is **taken
/// out**, not copied.
///
/// This is the regression guard on the fix: whoever buffers the mouse motion must not go and
/// buffer it **past** an override, or every `--script` run silently drifts.
#[test]
fn p3_a_script_look_still_overrides_the_mouse() {
    use defeated_by_titan::shared::LookOverride;

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    // A frame with mouse motion in it AND an override: the override decides.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(fps(60)));
    app.world_mut().write_message(MouseMotion { delta: Vec2::new(500.0, 0.0) });
    app.world_mut().resource_mut::<LookOverride>().0 = Some((0.25, -0.1));
    app.update();

    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let after_override = *players.iter(app.world()).next().expect("local player");
    assert!(
        (after_override.yaw - 0.25).abs() < 1e-5,
        "the script's absolute look must win — yaw was {}",
        after_override.yaw
    );

    // And the next frame the mouse has the wheel back.
    app.world_mut().write_message(MouseMotion { delta: Vec2::new(10.0, 0.0) });
    app.update();
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let after_mouse = *players.iter(app.world()).next().expect("local player");
    assert!(
        (after_mouse.yaw - 0.25).abs() > 1e-6,
        "after an override the mouse has to move the view again — yaw stayed {}",
        after_mouse.yaw
    );
}

// ---------------------------------------------------------------------------
// The **key mapping** — `src/net/local.rs` is the only place in the game that knows what a
// key is, and this is the only place that says what the keys mean.
//
// The user played the game on 2026-08-10, the first time a human ever did, and asked for the
// ropes on `Q` and `E` so they can be steered while the blades stay on the mouse. That is a
// swap, not an addition: whoever moves `HOOK_LEFT` onto `Q` also has to move `MARK` off it,
// and has to take `HOOK_LEFT` off the left mouse button. A test that only checked the new
// binding would stay green with the key bound **twice** — so every case below asserts the
// button that must be set **and** the button that must not be.
// ---------------------------------------------------------------------------

/// Presses keys and mouse buttons in the **real** app and returns the [`Buttons`] that arrive
/// on the local player's `Intent` one tick later.
///
/// Goes through `ButtonInput` and the whole `FixedPreUpdate` chain, exactly like the script
/// driver does (`src/debug/mod.rs:217-224`) — writing the `Intent` by hand would test nothing
/// but the test.
fn buttons_from(keys: &[KeyCode], mouse: &[MouseButton]) -> Buttons {
    intent_from(keys, mouse).buttons
}

/// The same, but the whole [`Intent`] — the movement axes matter as soon as one key means two
/// things (`S` is `REEL_IN` **and** walking backwards, `docs/NEXT.md` §1a).
fn intent_from(keys: &[KeyCode], mouse: &[MouseButton]) -> Intent {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });

    // Startup frame with a zero delta, so no fixed step is smuggled in before the press.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    for key in keys {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(*key);
    }
    for button in mouse {
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(*button);
    }

    // Two timesteps' worth of time: at least one fixed step is due, and a held button is still
    // held in the second one.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        2 * app.world().resource::<Time<Fixed>>().timestep(),
    ));
    app.update();

    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    players
        .iter(app.world())
        .next()
        .expect("the local player must exist after startup")
        .to_owned()
}

/// `S` **tensions the rope** — and it keeps walking backwards on the ground.
///
/// The user, 2026-08-12 (`docs/NEXT.md` §1a): *„wenn man w drückt und verbunden ist bekommt man
/// schon movement! bei a und d movement zur seite. **mit s »spannt« man nur das seil!**"* — so
/// `S` is the one WASD key that is not a thrust (`player::locomotion::air_thrust` clamps the
/// forward axis at 0) and instead does what `Ctrl` does.
///
/// **A second binding, not a move.** `Ctrl` keeps `REEL_IN`: the reel has to stay reachable
/// with a hand that is holding `W`, and `scripts/f-flight-cut.txt` presses `Ctrl` for it.
#[test]
fn bindings_s_tensions_the_rope_and_still_walks_backwards() {
    let i = intent_from(&[KeyCode::KeyS], &[]);
    assert!(
        i.buttons.contains(Buttons::REEL_IN),
        "S must set REEL_IN — got {:?}",
        i.buttons
    );
    assert!(
        (i.move_y + 1.0).abs() < 1e-6,
        "S must still be backwards on the ground — move_y is {}",
        i.move_y
    );
    assert!(
        intent_from(&[KeyCode::ControlLeft], &[]).buttons.contains(Buttons::REEL_IN),
        "the reel lost its own key — S is a SECOND binding, not a move"
    );
}

/// `Q` is the **left rope**, and it is no longer the mark.
#[test]
fn bindings_q_is_hook_left_and_not_mark() {
    let t = buttons_from(&[KeyCode::KeyQ], &[]);
    assert!(t.contains(Buttons::HOOK_LEFT), "Q must set HOOK_LEFT — got {t:?}");
    assert!(!t.contains(Buttons::MARK), "Q must not set MARK any more — got {t:?}");
}

/// `E` is the **right rope**, and it is no longer the right blade.
#[test]
fn bindings_e_is_hook_right_and_not_slash_right() {
    let t = buttons_from(&[KeyCode::KeyE], &[]);
    assert!(t.contains(Buttons::HOOK_RIGHT), "E must set HOOK_RIGHT — got {t:?}");
    assert!(
        !t.contains(Buttons::SLASH_RIGHT),
        "E must not set SLASH_RIGHT any more — got {t:?}"
    );
}

/// The left mouse button is the **left blade**, and it is no longer the left rope.
#[test]
fn bindings_left_mouse_is_slash_left_and_not_hook_left() {
    let t = buttons_from(&[], &[MouseButton::Left]);
    assert!(t.contains(Buttons::SLASH_LEFT), "LMB must set SLASH_LEFT — got {t:?}");
    assert!(
        !t.contains(Buttons::HOOK_LEFT),
        "LMB must not set HOOK_LEFT any more — got {t:?}"
    );
}

/// The right mouse button is the **right blade**, and it is no longer the right rope.
#[test]
fn bindings_right_mouse_is_slash_right_and_not_hook_right() {
    let t = buttons_from(&[], &[MouseButton::Right]);
    assert!(t.contains(Buttons::SLASH_RIGHT), "RMB must set SLASH_RIGHT — got {t:?}");
    assert!(
        !t.contains(Buttons::HOOK_RIGHT),
        "RMB must not set HOOK_RIGHT any more — got {t:?}"
    );
}

/// `MARK` had to go somewhere when `Q` became a rope: `Tab`, a key nothing else uses.
#[test]
fn bindings_tab_is_mark() {
    let t = buttons_from(&[KeyCode::Tab], &[]);
    assert!(t.contains(Buttons::MARK), "Tab must set MARK — got {t:?}");
    assert!(
        !t.contains(Buttons::HOOK_LEFT) && !t.contains(Buttons::HOOK_RIGHT),
        "Tab must not touch a rope — got {t:?}"
    );
}

/// The two hooks stay **independent** — the point of putting them on two keys is that both
/// ropes can be out at once, and a hand can hold `Q` and `E` where it cannot hold two mouse
/// buttons and still aim.
#[test]
fn bindings_q_and_e_hold_both_ropes_at_once() {
    let t = buttons_from(&[KeyCode::KeyQ, KeyCode::KeyE], &[]);
    assert!(
        t.contains(Buttons::HOOK_LEFT) && t.contains(Buttons::HOOK_RIGHT),
        "Q+E must set both hooks — got {t:?}"
    );
}

/// What the rebinding was **not** allowed to touch: boost, reel-in, jump and dodge sit where
/// they sat, and `F` keeps working as the second binding for the left blade (the keyboard's
/// only route to it, and the one `parse_key` can reach).
#[test]
fn bindings_boost_reel_jump_dodge_and_f_are_unchanged() {
    assert!(
        buttons_from(&[KeyCode::ShiftLeft], &[]).contains(Buttons::BOOST),
        "Shift must still be BOOST"
    );
    assert!(
        buttons_from(&[KeyCode::ControlLeft], &[]).contains(Buttons::REEL_IN),
        "Ctrl must still be REEL_IN"
    );
    assert!(buttons_from(&[KeyCode::Space], &[]).contains(Buttons::JUMP), "Space must still be JUMP");
    assert!(buttons_from(&[KeyCode::KeyC], &[]).contains(Buttons::DODGE), "C must still be DODGE");
    assert!(
        buttons_from(&[KeyCode::KeyF], &[]).contains(Buttons::SLASH_LEFT),
        "F must still be SLASH_LEFT"
    );
}
