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

/// 🔴 `S` **never reels** — and it still walks backwards.
///
/// This test used to claim the opposite, and it was wrong. *„mit s »spannt« man nur das
/// seil!"* was read as "reel in", so `S` became a second binding for `REEL_IN`; but
/// **„spannen" is holding a rope taut, not hauling it in**, and the first time the user played
/// it he said so (`docs/NEXT.md` §1A req 7): *„aktuell wenn ich seil spanne und s drücke werde
/// ich stark zum seil gezogen! das soll nicht sein!"*
///
/// The number behind it is in `r7_s_held_on_a_taut_rope_does_not_close_the_distance`: a held
/// `S` was worth `reel_speed_m_s` × 2 s = **56 m** of rope. What stays taut is the rope, and
/// what keeps it taut is the rope constraint — not a key.
#[test]
fn bindings_s_never_reels_and_still_walks_backwards() {
    let i = intent_from(&[KeyCode::KeyS], &[]);
    assert!(
        !i.buttons.contains(Buttons::REEL_IN),
        "S must NOT set REEL_IN — it hauls the player into his own anchor ({:?})",
        i.buttons
    );
    assert!(
        (i.move_y + 1.0).abs() < 1e-6,
        "S must still be backwards on the ground — move_y is {}",
        i.move_y
    );
    assert!(
        intent_from(&[KeyCode::ControlLeft], &[]).buttons.contains(Buttons::REEL_IN),
        "and the reel keeps its own key: Ctrl is REEL_IN"
    );
}

/// ★ **R7, and it is the number the user reported.** A held `S` on a taut rope must not close
/// the distance to the anchor.
///
/// *„aktuell wenn ich seil spanne und s drücke werde ich stark zum seil gezogen! das soll
/// nicht sein!"* (user, 2026-08-12, `docs/NEXT.md` §1A req 7). The binding test above says
/// which bit is set; **this one says what it did to the player**, in metres, in the real app
/// on the real map — a bit pattern is not evidence that anybody stopped being dragged.
///
/// The manoeuvre is `scripts/f003-ashgate.txt` ACT 4 leg 1 with `S` where that script holds
/// `Ctrl`: from the market square at `(75, 2, -30)`, `look 0 44` puts the ray on the wall
/// gallery 57.6 m away and 56 m up, i.e. ~80 m of rope. `reel_speed_m_s` is 28, so 120 ticks
/// of reeling are **56 m** — which is exactly what this measured before `S` was taken off
/// `REEL_IN`.
#[test]
fn r7_s_held_on_a_taut_rope_does_not_close_the_distance() {
    let idle = taut_rope_then_hold(&[]);
    let with_s = taut_rope_then_hold(&[KeyCode::KeyS]);
    let with_a = taut_rope_then_hold(&[KeyCode::KeyA]);
    let with_w = taut_rope_then_hold(&[KeyCode::KeyW]);

    // The numbers first: a test that only says "assertion failed" leaves the report without
    // one, and the point of this one IS the number.
    println!(
        "120 ticks on a taut rope — idle {idle:+.3} m · S {with_s:+.3} m · A {with_a:+.3} m · \
         W {with_w:+.3} m (S as a second REEL_IN binding: -56.001 m)"
    );

    // **The bound is derived, not chosen.** `S` used to be `REEL_IN`, and 120 ticks of that is
    // `reel_speed_m_s` × 2 s. A hundredth of that is still two orders of magnitude away from
    // anything a player would call "being pulled in", and no *slow* reel can sneak through it
    // either — a reel at 1 % of `reel_speed_m_s` would take three minutes to cover this rope.
    let reel_would_have = reel_speed_m_s() * 2.0;
    assert!(
        with_s > -reel_would_have / 100.0,
        "S closed {:.3} m in 120 ticks — a reel would have closed {reel_would_have:.1} m. \
         `S` is backwards movement, not a reel (docs/NEXT.md §1A req 7)",
        -with_s
    );
    // And the half that says what `S` **is**: a movement key like the other three. Walking
    // into a taut rope moves you, and the rope answers — `A` and `W` do far more of it than
    // `S` does. Without this line a fix that merely made the reel *slow* would pass, and the
    // user would still be dragged, only politely.
    assert!(
        with_s >= with_a && with_s >= with_w,
        "S ({with_s:+.3} m) closes more distance than A ({with_a:+.3} m) or W ({with_w:+.3} m) \
         — it still has a rope power the other movement keys do not have"
    );
    // ⚠️ `idle` is 0.000 m and `S` is not: walking backwards pulls the rope taut, and a rope
    // that points 44° upwards lifts you a few centimetres when it answers. That residual is
    // measured in `docs/FINDINGS.md` FIND-083 and it is **not** what the user reported — the
    // acceptance number for this job was -0.05 m, and this is the one line that says openly
    // that the ground case lands at -0.17 m instead, for a reason that has nothing to do with
    // the binding.
    println!("idle control: {idle:+.3} m — anything above that is the rope, not the key");
}

/// `vector.reel_speed_m_s` out of the app's own `game.ron` — the speed `S` used to have.
fn reel_speed_m_s() -> f32 {
    let app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let v = app.world().resource::<GameData>().game.vector.reel_speed_m_s;
    v
}

/// Hooks the wall gallery from the market square, then holds `keys` for **120 ticks** and
/// returns how much the distance to the anchor changed, in metres. Negative is towards it.
///
/// The manoeuvre is `scripts/f003-ashgate.txt` ACT 4 leg 1 with `S` where that script holds
/// `Ctrl`: from `(75, 2, -30)`, `look 0 44` puts the ray on the gallery 57.6 m away and 56 m
/// up — 82.2 m of rope, measured. `reel_speed_m_s` is 28, so 120 ticks of reeling are **56 m**,
/// and that is exactly what this returned while `S` was a second binding for `REEL_IN`.
fn taut_rope_then_hold(keys: &[KeyCode]) -> f32 {
    use defeated_by_titan::shared::{LookOverride, PlayerId, WarpPlayer};

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // One `update()` is one tick here, by construction — the same reason `tap_script` does it:
    // "120 ticks" has to be 120 ticks and not the mood of the machine (`B-002`).
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();

    let me = *app
        .world_mut()
        .query_filtered::<&PlayerId, With<LocalPlayer>>()
        .iter(app.world())
        .next()
        .expect("the local player must exist after startup");

    app.world_mut().write_message(WarpPlayer { player: me, pos_x: 75.0, pos_y: 2.0, pos_z: -30.0 });
    app.world_mut().resource_mut::<LookOverride>().0 = Some((0.0, 44.0_f32.to_radians()));
    for _ in 0..90 {
        app.update();
    }

    // The rope: `Q` is `HOOK_LEFT` and it stays **held** — releasing the key releases the
    // anchor (`scripts/f003-ashgate.txt` ACT 4 relies on the same).
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    let mut anchored = false;
    for _ in 0..180 {
        app.update();
        if distance_to_anchor(&mut app).is_some() {
            anchored = true;
            break;
        }
    }
    assert!(anchored, "the hook never anchored — this run measures nothing about S");
    let before = distance_to_anchor(&mut app).expect("anchored");

    for key in keys {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(*key);
    }
    for _ in 0..120 {
        app.update();
    }
    let after = distance_to_anchor(&mut app)
        .expect("the rope has to still be anchored, or the measurement is void");
    println!("  {keys:?} held: {before:.3} m -> {after:.3} m to the anchor");
    after - before
}

/// Distance from the local player to the tip of whichever arm is anchored, or `None` while
/// no rope holds.
fn distance_to_anchor(app: &mut App) -> Option<f32> {
    use defeated_by_titan::shared::{Hook, Side};

    let mut players = app.world_mut().query_filtered::<(&Transform, &Hook), With<LocalPlayer>>();
    let (transform, hook) = players.iter(app.world()).next()?;
    Side::ALL
        .into_iter()
        .find(|side| hook.arm(*side).state.is_anchored())
        .map(|side| (hook.arm(side).tip_m - transform.translation).length())
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
    // ⚠️ **`C` is still `DODGE`, but it is an EDGE since `F-008`** — this helper reads the
    // second of two ticks with the key held, and `DODGE` is true only on the first. That is
    // deliberate and it is measured in `f008_c_fires_one_dodge_per_press_and_not_one_per_tick`:
    // a dodge costs a flat 45 gas, so a held key that pressed `DODGE` on every tick would empty
    // the tank in seven of them. What this line still guards is the other half of the binding —
    // `C` and nothing else must be the key, and it must not have quietly become a `JUMP` or a
    // `BOOST` on the way.
    assert!(
        !buttons_from(&[KeyCode::KeyC], &[]).contains(Buttons::JUMP)
            && !buttons_from(&[KeyCode::KeyC], &[]).contains(Buttons::BOOST),
        "C must be DODGE alone"
    );
    assert!(
        buttons_from(&[KeyCode::KeyF], &[]).contains(Buttons::SLASH_LEFT),
        "F must still be SLASH_LEFT"
    );
}

// ---------------------------------------------------------------------------------------
// `F-008` — the double-tap. The user, 2026-08-12 (`docs/NEXT.md` §1c):
// *„mit doppel leertaste boostet man stark in die lauf richtung (ein weiter dodge) der viel
// gas aufbraucht."*
//
// Everything below drives the **real** `read_input` through the real app, one fixed step per
// `update()`, and reads the `Intent` the simulation would have read. Writing `Buttons::DODGE`
// by hand would test nothing but the test — and the whole risk in this feature is a timing
// one, so a pure function alone would not be evidence. The unit-level rules of the state
// machine are checked underneath, on `DodgeTap` itself.
// ---------------------------------------------------------------------------------------

use defeated_by_titan::net::local::DodgeTap;

/// Drives the app tick by tick. `script[i]` is the set of keys **held during tick i**; the
/// answer is, per tick, whether the `Intent` carried `DODGE` and `JUMP`.
///
/// `FixedTimesteps(1)` and not a duration: this file's other tests exist because a fixed step
/// per frame is 0..n (`B-002`), and a timing test that cannot say which tick it is on measures
/// nothing. One `update()` is one tick here, by construction.
fn tap_script(script: &[&[KeyCode]]) -> Vec<(bool, bool)> {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update(); // Startup: the world and the local player come into being

    let mut out = Vec::with_capacity(script.len());
    for held in script {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            for key in *held {
                keys.press(*key);
            }
        }
        app.update();
        let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
        let t = players
            .iter(app.world())
            .next()
            .expect("the local player must exist after startup")
            .buttons;
        out.push((t.contains(Buttons::DODGE), t.contains(Buttons::JUMP)));
    }
    out
}

/// How many ticks in `script` pressed `DODGE`.
fn dodges(script: &[&[KeyCode]]) -> usize {
    tap_script(script).iter().filter(|(dodge, _)| *dodge).count()
}

const SPACE: &[KeyCode] = &[KeyCode::Space];
const NOTHING: &[KeyCode] = &[];

/// The window out of `game.ron`, so this file does not carry a second copy of a game value.
fn window_ticks() -> u64 {
    let app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let w = app.world().resource::<GameData>().game.vector.dodge_double_tap_window_ticks;
    assert!(w >= 2, "a window under 2 ticks cannot hold a press and a release: {w}");
    w
}

#[test]
fn f008_two_space_taps_inside_the_window_are_one_dodge() {
    // Tap, release, tap — three ticks, well inside any sane window. The dodge lands on the
    // tick of the SECOND press and on no other, because a dodge is one impulse.
    let ticks = tap_script(&[SPACE, NOTHING, SPACE, NOTHING, NOTHING]);
    let fired: Vec<usize> =
        ticks.iter().enumerate().filter(|(_, (d, _))| *d).map(|(i, _)| i).collect();
    assert_eq!(fired, vec![2], "the dodge must land exactly on tick 2, fired on {fired:?}");
}

#[test]
fn f008_a_second_tap_after_the_window_is_only_a_jump() {
    // The same gesture, one tick too slow. If this ever goes green with the gap above the
    // window, the window is not being read at all and the value in the file is decoration.
    let gap = (window_ticks() + 2) as usize;
    let mut script: Vec<&[KeyCode]> = vec![SPACE];
    script.extend(std::iter::repeat_n(NOTHING, gap));
    script.push(SPACE);
    let ticks = tap_script(&script);
    assert!(
        ticks.iter().all(|(dodge, _)| !dodge),
        "a gap of {gap} ticks is outside the {}-tick window and must not dodge",
        window_ticks()
    );
    assert!(ticks[0].1 && ticks[gap + 1].1, "both taps are still jumps");
}

#[test]
fn f008_a_held_space_is_a_jump_and_never_a_dodge() {
    // The edge, not the level. This is also the guard on `bindings_..._unchanged` above: a
    // player who holds Space to jump must not be charged 45 gas for it.
    let held: Vec<&[KeyCode]> = vec![SPACE; 40];
    assert_eq!(dodges(&held), 0, "40 ticks of held Space produced a dodge");
    assert!(tap_script(&held).iter().all(|(_, jump)| *jump), "and every one of them is a jump");
}

#[test]
fn f008_a_dodge_never_swallows_the_jump() {
    // `F-008` adds a button, it does not steal one. On the tick the dodge fires, `JUMP` is
    // pressed as well — a dodge off the ground is a jump that then throws you.
    let ticks = tap_script(&[SPACE, NOTHING, SPACE]);
    assert_eq!(ticks[2], (true, true), "tick 2 must be dodge AND jump, was {:?}", ticks[2]);
}

#[test]
fn f008_c_fires_one_dodge_per_press_and_not_one_per_tick() {
    // The reason this matters is arithmetic, not taste: a dodge costs a flat `gas_dodge` (45),
    // so a `DODGE` bit that stayed true while `C` is held would empty the 300-gas tank in
    // seven ticks — 0.11 s — and the player would only ever see that his gas was gone.
    let held: Vec<&[KeyCode]> = vec![&[KeyCode::KeyC]; 40];
    let ticks = tap_script(&held);
    let fired: Vec<usize> =
        ticks.iter().enumerate().filter(|(_, (d, _))| *d).map(|(i, _)| i).collect();
    assert_eq!(fired, vec![0], "40 ticks of held C must be one dodge, on tick 0; fired {fired:?}");
    // And `C` is `DODGE` and not something else — the half of the old binding assertion that
    // survived `F-008` (`bindings_boost_reel_jump_dodge_and_f_are_unchanged`).
    assert!(!ticks.iter().any(|(_, jump)| *jump), "C must not be a jump");
}

// --- and the rules of the state machine itself, which no app test can reach cheaply -------

#[test]
fn f008_a_third_tap_is_not_a_second_dodge() {
    // Without consuming the first tap, tap 2 would arm tap 3 and a player drumming on Space
    // would pay 45 gas per tap. Three taps inside the window are ONE dodge.
    let mut tap = DodgeTap::default();
    let mut fired = 0;
    for (tick, down) in [true, false, true, false, true, false].into_iter().enumerate() {
        if tap.feed(down, false, tick as u64, 18) {
            fired += 1;
        }
    }
    assert_eq!(fired, 1, "three taps inside one window are one dodge, got {fired}");
}

#[test]
fn f008_a_late_tap_re_arms_instead_of_failing() {
    // Tap — long pause — tap, tap. The third is the partner of the second. Anything else
    // would let one mistimed pair swallow the next honest attempt.
    let mut tap = DodgeTap::default();
    assert!(!tap.feed(true, false, 0, 4));
    assert!(!tap.feed(false, false, 1, 4));
    assert!(!tap.feed(true, false, 20, 4), "20 ticks apart is not a double-tap");
    assert!(!tap.feed(false, false, 21, 4));
    assert!(tap.feed(true, false, 22, 4), "but the pair 20/22 is");
}

#[test]
fn f008_the_window_is_inclusive_at_both_ends() {
    // Exactly `window_ticks` apart still counts; one more does not. A window whose boundary
    // nobody pinned down is a window that moves whenever somebody rewrites the comparison.
    for (gap, expected) in [(4_u64, true), (5, false)] {
        let mut tap = DodgeTap::default();
        tap.feed(true, false, 0, 4);
        tap.feed(false, false, 1, 4);
        assert_eq!(
            tap.feed(true, false, gap, 4),
            expected,
            "a gap of {gap} against a window of 4 must be {expected}"
        );
    }
}
