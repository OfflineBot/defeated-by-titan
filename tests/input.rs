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
/// The number behind it is in `r7_s_cancels_the_pull_and_only_the_lateral_remains`: a held
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

/// ★ **R7, third meaning, and it is his newest word.** With `S` held **the pull is cancelled**
/// — only the lateral remains.
///
/// The history of this key, each reversal his own sentence, newest wins (`Q-002`):
///
/// 1. 2026-08-12 — *„mit s »spannt« man nur das seil!"* → `S` stopped being a `REEL_IN`
///    binding (56 m of haul in 2 s, the number he reported).
/// 2. 2026-08-27 — `Q-061`, asked directly: *„S spannt nur, bewegt nicht."* → the two-sided
///    bound: `S` moves the player neither in nor out.
/// 3. 2026-09-01 — `docs/NEXT.md` §5D rule 4: *„aber wenn verbunden wird immer rangezogen …
///    aber dennoch AUßER man drückt S dann nur zur seite"* → **`S` is the one input that
///    cancels the always-on pull.** With `S` held and no lateral key, the player holds
///    position on a taut rope; the joint still forbids retreat (§3F, „NICHT das seil
///    verlängern"). `Q-061` is superseded — see its entry in `docs/QUESTIONS.md`.
///
/// The GEOMETRY is unchanged since meaning 1, so all three sentences were measured on the
/// same rope: `look 0 44`, 82.2 m from the hand — 83.32 m body-to-tip, 44.8° of elevation.
/// What CARRIES that geometry moved on 2026-09-02: §5E (`FIND-231`) redistributed Ashgate's
/// terrain and broke the market-square stance's ground, so the fixture now stands on its own
/// slab with a synthetic anchor at the old numbers — the story is on
/// [`taut_rope_then_hold`]. 120 ticks of the old reel closed 56 m here.
///
/// # What the fixture varies and what the code reads (`docs/lessons/fixtures.md` rule 2)
///
/// The fixture varies: the held key set, and `drive_idle_speed_m_s` (the deletion control).
/// It pins: the graybox map, the slab the stance stands on, and the anchor's geometry
/// (44°, 82.2 m — synthetic since 2026-09-02, see [`taut_rope_then_hold`]).
/// The code reads: `move_x`/`move_y` (`pull_scale` — the S-cancel and the lateral ramp),
/// `MovementState` (the winch's `in_flight` gate), the anchor geometry, the
/// `HookAnchored` message (`player::rope::attach_ropes` builds the `DistanceJoint` from it —
/// the fixture has to send it, a forced arm state alone is a winch without a rope), and
/// `drive_steer_pull_fraction`. The fraction is NOT varied here — its ramp is measured in
/// `tests/player.rs::f5d_the_lateral_scales_the_pull_in_a_ramp_not_a_switch`.
///
/// # 🔴 The free pull needs a key that is not a movement key — `Space`
///
/// Until §5D, "the pure always-on pull" was measured by holding `S`: once airborne, `S` was
/// arithmetically no key at all (`FIND-196`'s table). Rule 4 kills that proxy — `S` now
/// cancels the very thing it was standing in for. `Space` replaces it: one jump takes the
/// player off his legs (the winch is `in_flight`-gated, `FIND-172`), and `JUMP` touches
/// neither `move_x` nor `move_y`, so the winch runs at full scale. The jump's own impulse is
/// in the number, which is why the deletion control (`space_alone`) subtracts it out.
///
/// # 🔴 FIND-196 is closed here, and this is the re-measurement
///
/// FIND-196: holding `W` closed **11.270 m** where holding nothing closed **11.937 m** — the
/// look gate `max(0, l̂·r̂)` shrank the drive's target while the winch's floor did not, so the
/// key that flies you AT your anchor was slower than no key. Since §5D the pull is the winch
/// alone (never look-gated, never chased away) and `W` is a look-directed thrust ON TOP of it
/// (`player::locomotion::rope_drive`), so a forward key must close at least the free pull.
///
/// ⚠️ **Until 2026-09-01 the comparison had to happen IN FLIGHT, because the winch was
/// ground-gated** (`FIND-172`'s `in_the_air`, the `Q-055`/`Q-056` decision) — `W` alone
/// WALKED its 8.031 m. **§5E-b overturned that gate, third ruling, newest word wins:**
/// *„wenn cih mich hooke werde ich nicht autmoatisch rangezogen! das fehlt noch!"* — a bite
/// pulls immediately, ground included (`player::locomotion::ground_pull_live`). So `idle`
/// now MEASURES the ground pull (14.714 m closed on this fixture, where the gate held it at
/// 0.000), `W` rides pull + thrust from the bite tick, and the FIND-196 pair `Space`+`W`
/// against `Space` still stands with the same `with_sw <= with_space + 0.05` bound.
#[test]
fn r7_s_cancels_the_pull_and_only_the_lateral_remains() {
    let idle = taut_rope_then_hold(&[]);
    let with_s = taut_rope_then_hold(&[KeyCode::KeyS]);
    let with_a = taut_rope_then_hold(&[KeyCode::KeyA]);
    let with_w = taut_rope_then_hold(&[KeyCode::KeyW]);
    // The free pull: `Space` gets him off his legs without touching a movement axis.
    let with_space = taut_rope_then_hold(&[KeyCode::Space]);
    // And the FIND-196 pair: the same flight WITH the forward key.
    let with_sw = taut_rope_then_hold(&[KeyCode::Space, KeyCode::KeyW]);
    // **The deletion controls**: one key of `game.ron` set to 0 takes `FIND-172`'s always-on
    // pull out of the run and leaves everything else — map, rope, ground, binding — identical.
    let idle_alone = taut_rope_then_hold_with_pull(&[], false);
    let s_alone = taut_rope_then_hold_with_pull(&[KeyCode::KeyS], false);
    let a_alone = taut_rope_then_hold_with_pull(&[KeyCode::KeyA], false);
    let space_alone = taut_rope_then_hold_with_pull(&[KeyCode::Space], false);
    // And the shape of an actual reel, in this same fixture: `Ctrl` is `REEL_IN`.
    let reel = taut_rope_then_hold_with_pull(&[KeyCode::ControlLeft], false);

    // The numbers first: a test that only says "assertion failed" leaves the report without
    // one, and the point of this one IS the number.
    println!(
        "120 ticks on a taut rope — idle {idle:+.3} m · S {with_s:+.3} m · A {with_a:+.3} m · \
         W (pull + thrust, §5E-b) {with_w:+.3} m · Space {with_space:+.3} m · Space+W \
         {with_sw:+.3} m"
    );
    println!(
        "  the always-on pull deleted — S {s_alone:+.3} m · A {a_alone:+.3} m · \
         Space {space_alone:+.3} m · Ctrl (the real reel) {reel:+.3} m"
    );

    // 🔴 **Rule 4, the headline: `S` cancels the pull.** Two-sided and with the pull ON — the
    // stronger statement than Q-061's, because until §5D the pull hauled a taut-roped `S`
    // player 11.9 m through this very window. The bound is derived: `run_speed_m_s` is 6.0, so
    // 120 ticks of walking is 12 m; a tenth of a walk is far below anything a player could
    // feel as movement and an order of magnitude above this fixture's solver settling.
    let a_tenth_of_a_walk = run_speed_m_s() * 2.0 / 10.0;
    assert!(
        with_s.abs() < a_tenth_of_a_walk,
        "§5D rule 4 („AUßER man drückt S dann nur zur seite\"): with the pull ON, S held for \
         120 ticks moved the player {with_s:+.3} m — it must cancel the pull and hold position \
         within {a_tenth_of_a_walk:.3} m"
    );
    // And with the pull deleted the same key reads the same ~nothing — S owns no movement on
    // this axis in either world. If these two differ, something besides the winch moves him.
    assert!(
        s_alone.abs() < a_tenth_of_a_walk,
        "with the always-on pull deleted, S still moved {s_alone:+.3} m in 120 ticks — S is \
         neither a reel nor a walk on this axis (docs/NEXT.md §1A req 7, §5D rule 4)"
    );
    // 🔴 **The deletion control has to move a number, and since rule 4 that number is
    // `Space`'s, not `S`'s.** One jump with the pull on hands him to the winch; the same jump
    // with the pull deleted comes straight back down to his legs.
    assert!(
        with_space < space_alone - 5.0,
        "deleting `drive_idle_speed_m_s` changed the held-Space run from {with_space:+.3} m to \
         {space_alone:+.3} m — if those two are the same number, the always-on pull is not what \
         closes this rope and this test is measuring the wrong thing"
    );
    // 🔴 **FIND-196, closed: the forward key in flight closes at least the free pull.**
    // 0.05 m of margin against a defect that measured 0.667 m. Under the old model this pair
    // is exactly the strangle: the drive's look-gated chase replaced the winch's closing with
    // something slower the moment `W` went down.
    assert!(
        with_sw <= with_space + 0.05,
        "Space+W closed {:.3} m while the free pull (Space alone) closed {:.3} m — the look is \
         gating the pull again (FIND-196, §5D rule 1)",
        -with_sw,
        -with_space
    );
    // `W` alone, since §5E-b: the bite hands the body to the rope on the spot, and `W` is
    // thrust plus drive ON TOP of the pull — so from a standing start it has to beat the
    // free pull by a wide margin, not merely walk. Measured on this fixture: −53.813 m
    // against the free pull's −14.714 — a 39 m gap; 10 m is the fat-margin floor under it.
    assert!(
        with_w < with_space - 10.0,
        "W alone closed {:.3} m against the free pull's {:.3} m — thrust on top of the \
         ground pull is supposed to beat the pull alone by ≥ 10 m in this stance (§5E-b + \
         §5D rule 2)",
        -with_w,
        -with_space
    );
    // 🔴 **And the fixture has to be able to SEE a reel**, or none of the above is evidence:
    // `Ctrl`, under the same deleted pull, is `REEL_IN` and closes tens of metres.
    assert!(
        reel < -20.0,
        "`Ctrl` closed only {:.3} m in this fixture — if the real reel is invisible here, the \
         fact that S is invisible proves nothing",
        -reel
    );
    // `A`, since §5E-b, hands the body to the rope too (the ramp never reaches zero) — and
    // the DISTANCE still has to stay flat in this stance, for a measured reason, not a gate:
    // at 44.8° of elevation gravity's outward radial component (32·sin 44.8° ≈ 22.5 m/s²)
    // beats the ramped pull's whole ceiling (0.35 · 34.29 = 12 m/s²), so the taut joint
    // holds the radius while the lateral drive swings him across it. The angle where that
    // argument dies is asin(12/32) = 22.0° — this stance carries double it. And the ground
    // under the whole swing arc is the fixture's own FLAT slab: the lateral drive carries
    // the body 31–46 m across it in these 120 ticks (`drive_lateral_m_s: 24`), and on
    // 2026-09-02 exactly that arc is what §5E's redistributed terrain broke under the old
    // market stance — a hill ridden up INSIDE the sphere closed 1.310 m with the pull
    // deleted. Measured on the slab: −0.072 m with the pull on, −0.008 m without it. If `A`
    // ever starts closing tens of metres here, the ramp (`pull_scale`) has fallen off and
    // the full winch is running under a lateral key.
    let reel_would_have = reel_speed_m_s() * 2.0;
    assert!(
        with_a > -reel_would_have / 100.0 && a_alone > -reel_would_have / 100.0,
        "A closed {:+.3} m with the pull and {a_alone:+.3} m without it; the key that never \
         tautens the rope must not close it either",
        with_a
    );
    // 🔴 **§5E-b (2026-09-01), the third ruling on the ground gate, and this assert is its
    // re-derivation.** Until today `idle` was asserted to be 0.000 — the `Q-055`/`Q-056`
    // design where a hooked player standing still kept his legs. The user overturned it:
    // *„wenn cih mich hooke werde ich nicht autmoatisch rangezogen! das fehlt noch!"* — so
    // the key that takes him off his legs is now NO key, and a flat `idle` is the BUG this
    // line guards against. Measured on this fixture: −14.714 m (the 44.8° anchor is below the
    // 69° clean-lift line, so this closing is the DRAG — `player::locomotion::ground_pull_live`
    // and the elevation sweep in `tests/player.rs::f176_the_contact_break_threshold_*`).
    // −5 m is a third of the measurement and 60× the old residual noise.
    assert!(
        idle < -5.0,
        "a hooked player who pressed nothing closed only {:.3} m in 120 ticks — §5E-b says \
         the bite pulls immediately, ground included; a flat idle is the old `in_the_air` \
         gate (FIND-172/Q-056) coming back",
        -idle
    );
    // And the deletion control for exactly that claim: the same no-key stance with
    // `drive_idle_speed_m_s: 0` has to stand still — otherwise something other than the
    // always-on pull moved him and the number above is not the pull's.
    assert!(
        idle_alone.abs() < a_tenth_of_a_walk,
        "with the always-on pull deleted the no-key player still drifted {idle_alone:+.3} m \
         — the idle measurement is not measuring the winch"
    );
    println!("idle: {idle:+.3} m pulled · {idle_alone:+.3} m with the pull deleted");
}

/// `player.run_speed_m_s` out of the app's own `game.ron` — what walking would look like on
/// this axis, and therefore the scale `Q-061`'s two-sided bound on `S` is derived from. Read
/// from the file and not written as `6.0`, so a tuning change moves the test with the game.
fn run_speed_m_s() -> f32 {
    let app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let v = app.world().resource::<GameData>().game.player.run_speed_m_s;
    v
}

/// `vector.reel_speed_m_s` out of the app's own `game.ron` — the speed `S` used to have.
fn reel_speed_m_s() -> f32 {
    let app = defeated_by_titan::app(Cli { headless: true, ..default() });
    let v = app.world().resource::<GameData>().game.vector.reel_speed_m_s;
    v
}

/// Puts the player on a taut 82.2 m rope at 44° of elevation, then holds `keys` for
/// **120 ticks** and returns how much the distance to the anchor changed, in metres.
/// Negative is towards it.
///
/// # The stance is SYNTHETIC since 2026-09-02, and that is the fix for a real break
///
/// Until §5E (`FIND-231`) this manoeuvre was `scripts/f003-ashgate.txt` ACT 4 leg 1 flown
/// from Ashgate's market square: warp to `(75, 2, -30)`, `look 0 44`, and the ray put the
/// anchor on the wall gallery — measured on the last green run: 58.35 m away, 57.95 m up,
/// **44.8° of elevation, 82.24 m of rope**. §5E redistributed the terrain, and what broke
/// was NOT the stance or the angle (the ray re-hit the gallery at the same 44.8°): it was
/// the ground under the `A` key's swing ARC. The lateral drive carries the body 31–46 m
/// across the square in 120 ticks (`drive_lateral_m_s: 24`), the redistributed field rises
/// to y ≈ +3.3 m out there, and a body ridden up a hill INSIDE the rope sphere closes the
/// straight-line distance — 1.31 m with the pull deleted, against a 0.56 m bound that had
/// measured 0.008 m on flat pads.
///
/// So the fixture now owns its ground the way it already owned its keys: the **graybox**
/// (pinned for `tests/player.rs`'s reason — nothing here is a claim about a district), a
/// **flat 300 × 300 m slab whose top is exactly y = 100** — above every graybox roof, the
/// church (35 m) included, so no house can wander into the swing arc either — and the
/// anchor placed **synthetically at the OLD geometry**: `look 0 44`'s direction, 82.2 m
/// from the hand, on a phantom index body (the `tests/player.rs::stand_and_bite` pattern —
/// `world::index::maintain_index` only strikes out what an observer removed, so the entry
/// persists and `update_hooks` keeps the tip in place). Same elevation, same rope, same
/// physics premise; the ground can no longer move because no generator owns it.
///
/// `reel_speed_m_s` is 28, so 120 ticks of reeling are **56 m**, and that is exactly what
/// this returned while `S` was a second binding for `REEL_IN`.
fn taut_rope_then_hold(keys: &[KeyCode]) -> f32 {
    taut_rope_then_hold_with_pull(keys, true)
}

fn taut_rope_then_hold_with_pull(keys: &[KeyCode], pull: bool) -> f32 {
    use avian3d::prelude::{Collider, RigidBody};
    use defeated_by_titan::shared::{
        BodyId, BodyMask, Hook, HookAnchored, HookState, IndexEntry, LookOverride, PlayerId,
        Side, SpatialIndex, WarpPlayer,
    };

    // The old market-square ray, kept as numbers: `look 0 44` (yaw 0 is -Z, pitch up) and
    // 82.2 m of rope hand-to-tip. The elevation carries the physics premise — see `r7_*`'s
    // radius-holding argument — so whoever changes it re-derives that comment.
    const PITCH_DEG: f32 = 44.0;
    const ROPE_M: f32 = 82.2;
    const SLAB_TOP_Y_M: f32 = 100.0;

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // The graybox, not whatever `maps.ron: current` says — the same pinning argument as
    // `tests/player.rs`: this file's claim is about keys and the rope force model, not about
    // a district, and a level designer must not be able to move it.
    app.world_mut().resource_mut::<GameData>().maps.current = "graybox".to_string();
    assert!(
        app.world().resource::<GameData>().current_map().is_some(),
        "maps.ron lists no map \"graybox\" — the fixture would build an empty world"
    );
    if !pull {
        app.world_mut().resource_mut::<GameData>().game.vector.drive_idle_speed_m_s = 0.0;
    }
    // One `update()` is one tick here, by construction — the same reason `tap_script` does it:
    // "120 ticks" has to be 120 ticks and not the mood of the machine (`B-002`).
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();

    // The proving ground: one static slab, top at y = 100, big enough that the whole `A`
    // swing circle (radius ROPE_M · cos 44° ≈ 59 m around under-anchor, z ∈ [-118, 0]) and
    // the idle drag path stay on it with margin. `RigidBody::Static` + `Collider` is the
    // exact component pair `world::map` gives a block's physics.
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(300.0, 1.0, 300.0),
        Transform::from_xyz(0.0, SLAB_TOP_Y_M - 0.5, -30.0),
    ));

    let me = *app
        .world_mut()
        .query_filtered::<&PlayerId, With<LocalPlayer>>()
        .iter(app.world())
        .next()
        .expect("the local player must exist after startup");

    app.world_mut().write_message(WarpPlayer {
        player: me,
        pos_x: 0.0,
        pos_y: SLAB_TOP_Y_M + 2.0,
        pos_z: 0.0,
    });
    app.world_mut().resource_mut::<LookOverride>().0 = Some((0.0, PITCH_DEG.to_radians()));
    for _ in 0..90 {
        app.update();
    }
    // Fixture health: he has to be STANDING ON THE SLAB, or every number below is a
    // free-fall measurement wearing a stance's name.
    let stand_m = player_pos(&mut app);
    assert!(
        (stand_m.y - SLAB_TOP_Y_M).abs() < 0.5,
        "the player settled at y = {:.3} instead of on the slab top at {SLAB_TOP_Y_M} — the \
         fixture's ground is not under his feet and nothing below measures a stance",
        stand_m.y
    );

    // The rope: `Q` is `HOOK_LEFT` and it stays **held** — releasing the key releases the
    // anchor (`src/vector/hook.rs`, `ReleaseReason::Released`). The anchor itself is forced,
    // not flown: a phantom body in the spatial index carries the tip at exactly the old
    // ray's geometry, so the bite is tick-exact and the 120-tick window starts taut.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    let eye_m = app.world().resource::<GameData>().game.player.eye_height_m;
    let (sin, cos) = PITCH_DEG.to_radians().sin_cos();
    let anchor_m = stand_m + Vec3::Y * eye_m + Vec3::new(0.0, sin, -cos) * ROPE_M;
    {
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .expect("the local player must exist");
        let body = BodyId(90_001);
        let center_m = anchor_m + Vec3::Y * 2.0;
        app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
            id: body,
            center_m,
            half_size_m: Vec3::splat(2.0),
            mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
        });
        let mut hook = app.world_mut().get_mut::<Hook>(entity).expect("the player has a Hook");
        let arm = &mut hook.arms[Side::Left.index()];
        arm.state = HookState::Anchored { body, local_m: anchor_m - center_m };
        arm.tip_m = anchor_m;
        // And the MESSAGE a real bite sends: `player::rope::attach_ropes` builds the
        // `DistanceJoint` out of `HookAnchored`, not out of the arm's state — without it the
        // fixture has a winch but no rope, `S` walks 9 m past the radius and `Ctrl` reels
        // nothing (measured on this fixture's first run, 2026-09-02).
        let tick = app.world().resource::<Tick>().0;
        app.world_mut().write_message(HookAnchored {
            player: me,
            side: Side::Left,
            body,
            point_x: anchor_m.x,
            point_y: anchor_m.y,
            point_z: anchor_m.z,
            tick,
        });
    }
    let before = distance_to_anchor(&mut app)
        .expect("the forced anchor did not register — this run measures nothing about S");

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

/// The local player's position — the fixture's own measuring points.
fn player_pos(app: &mut App) -> Vec3 {
    let mut players = app.world_mut().query_filtered::<&Transform, With<LocalPlayer>>();
    players.iter(app.world()).next().expect("the local player exists").translation
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

// ---------------------------------------------------------------------------------------
// `F-009` — the flip's gesture: „Doppeltipp A/D".
// ---------------------------------------------------------------------------------------
//
// 🔴 **This block exists because of `FIND-152`**: a whole-app test that never reached the code
// it was testing. `tests/vector_boost.rs` proves what a granted flip DOES by writing
// `Buttons::FLIP` into an `Intent` by hand — which proves nothing at all about whether a human
// pressing `A` twice ever produces that bit. This is the half that closes the loop, and it is
// the same shape as the `f008_*` block above it.

/// Which of `[FLIP, move_x]` each tick of `script` produced, off the **real** input path.
fn flip_script(script: &[&[KeyCode]]) -> Vec<(bool, f32)> {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();

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
        let i = players.iter(app.world()).next().expect("the local player exists");
        out.push((i.buttons.contains(Buttons::FLIP), i.move_x));
    }
    out
}

const A: &[KeyCode] = &[KeyCode::KeyA];
const D: &[KeyCode] = &[KeyCode::KeyD];

#[test]
fn f009_two_a_taps_inside_the_window_are_one_flip_and_the_side_rides_on_move_x() {
    let ticks = flip_script(&[A, NOTHING, A, NOTHING, NOTHING]);
    let fired: Vec<usize> = ticks
        .iter()
        .enumerate()
        .filter(|(_, (flip, _))| *flip)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(fired, vec![2], "the flip lands on the SECOND press and on no other: {ticks:?}");
    // The whole reason `Buttons::FLIP` carries no side: on the tick it fires, the key that
    // produced it is down, so `move_x` already says which way. Two bits saying one thing is
    // two bits that can disagree.
    assert_eq!(ticks[2].1, -1.0, "`A` is -1 on the strafe axis: {ticks:?}");
}

#[test]
fn f009_two_d_taps_are_a_flip_the_other_way() {
    let ticks = flip_script(&[D, NOTHING, D, NOTHING]);
    assert!(ticks[2].0, "two D presses have to flip as well: {ticks:?}");
    assert_eq!(ticks[2].1, 1.0, "`D` is +1: {ticks:?}");
}

#[test]
fn f009_a_left_then_a_right_is_not_a_flip() {
    // The reason `net::local` keeps TWO arming states and not one. `A`-then-`D` is a player
    // changing direction, not asking for anything — with one shared tick it would fire a flip
    // (and bill 20 gas) every time somebody strafed back and forth.
    let ticks = flip_script(&[A, NOTHING, D, NOTHING, A, NOTHING, D, NOTHING]);
    assert!(
        ticks.iter().all(|(flip, _)| !*flip),
        "changing direction fired a flip: {ticks:?}"
    );
}

#[test]
fn f009_a_held_a_is_strafing_and_never_a_flip() {
    // The same property `f008_a_held_space_is_a_jump_and_never_a_dodge` holds for the dash, and
    // it matters more here: `A` is held for most of every swing (`docs/NEXT.md` §1a — *„das a d
    // sorgt dafür dass man nicht immer direkt zum seil gezogen wird"*), so a held key that
    // fired would bill `gas_flip` sixty times a second for ordinary steering.
    let ticks = flip_script(&[A, A, A, A, A, A, A, A]);
    assert_eq!(
        ticks.iter().filter(|(flip, _)| *flip).count(),
        0,
        "a held A produced a flip: {ticks:?}"
    );
    assert!(ticks.iter().all(|(_, x)| *x == -1.0), "…while still strafing the whole time");
}

#[test]
fn f009_a_second_tap_after_the_window_is_only_a_strafe() {
    let window = window_ticks() as usize;
    let mut script: Vec<&[KeyCode]> = vec![A];
    script.resize(window + 3, NOTHING);
    script.push(A);
    let ticks = flip_script(&script);
    assert_eq!(
        ticks.iter().filter(|(flip, _)| *flip).count(),
        0,
        "a tap {} ticks after the first still flipped — the window is {window}: {ticks:?}",
        window + 2
    );
}

// ---------------------------------------------------------------------------
// The rope trigger is a toggle at the seam (2026-09-01), and the keys are binds
// ---------------------------------------------------------------------------

/// > *„mach dass q und e toggle sind und nicht hold (oder in einstellungen einstellbar)"*
///
/// The whole toggle lives in `net::local::HookLatch`, on the keyboard side of the `Intent` —
/// this test drives the REAL app through `ButtonInput` and reads the bit off the `Intent`,
/// exactly the seam a script or a network client would see. A tap goes down for two ticks and
/// comes up; under the default `HookFire::Toggle` the `HOOK_LEFT` bit has to STAY up — the
/// pre-2026-09-01 code dropped it with the key. The running-game half of the claim (tap,
/// anchored, tap, released, and a long hold still releasing on key-up) is
/// `scripts/f172-hook-toggle.txt`, captured red against the old binary first:
/// `line 22: assert Rope == 1 — measured 0.000`.
#[test]
fn the_hook_bit_stays_latched_after_a_tap_and_hold_mode_still_exists() {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();
    let step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));

    let bit = |app: &mut App| {
        let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
        players
            .iter(app.world())
            .next()
            .expect("the local player must exist")
            .buttons
            .contains(Buttons::HOOK_LEFT)
    };

    // The same stance as `scripts/f172-hook-toggle.txt`: 12 m over the boulevard, looking
    // straight down — an anchor that has never missed. From the spawn on the street the
    // pavement is under `vector.min_rope_m` (3.0) and a tap at nothing clears itself off the
    // idle arm (`net::local::HookLatch`), which would make this test measure the miss path
    // instead of the latch. Warp and look take the same routes a script's verbs take.
    let me = {
        let mut players = app
            .world_mut()
            .query_filtered::<&defeated_by_titan::shared::PlayerId, With<LocalPlayer>>();
        *players.iter(app.world()).next().expect("the local player must exist")
    };
    app.world_mut().write_message(defeated_by_titan::shared::WarpPlayer {
        player: me,
        pos_x: 168.19,
        pos_y: 12.0,
        pos_z: -50.12,
    });
    app.world_mut().resource_mut::<defeated_by_titan::shared::LookOverride>().0 =
        Some((0.0, (-89.0f32).to_radians()));
    app.update();

    // Tap: two ticks down, then up.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();
    app.update();
    assert!(bit(&mut app), "the tap itself raises the bit — that edge is the fire");
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    for _ in 0..5 {
        app.update();
    }
    // ⚠️ In this app the aim ray from the spawn stance finds an anchorable surface, the arm
    // anchors, and the latch therefore holds. (If it missed, the latch would clear off the
    // idle arm — `net::local::HookLatch` — and this assert is what would notice.)
    assert!(
        bit(&mut app),
        "Toggle is the default and a tap has to LATCH: the bit fell with the key, which is \
         the Hold behaviour he asked to be rid of"
    );

    // The second tap drops it.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();
    app.update();
    assert!(!bit(&mut app), "the second tap has to release the latch");

    // And Hold is one setting away, bit-for-bit the old behaviour.
    app.world_mut()
        .resource_mut::<defeated_by_titan::shared::PlayerSettings>()
        .hook_fire = defeated_by_titan::shared::settings::HookFire::Hold;
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();
    assert!(bit(&mut app), "Hold: down is down");
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    app.update();
    assert!(!bit(&mut app), "Hold: up is up, the moment the key comes back");
}

/// `F-172` at the same seam: `read_input` fires the arm off `PlayerSettings::binds`, not off
/// a literal `KeyCode` — rebind the left hook to `M` and `M` is what raises the bit while `Q`
/// raises nothing.
#[test]
fn f172_a_rebound_key_fires_the_arm_and_the_old_key_is_dead() {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();
    {
        let mut s = app.world_mut().resource_mut::<defeated_by_titan::shared::PlayerSettings>();
        s.binds.set(
            defeated_by_titan::shared::settings::BindAction::HookLeft,
            KeyCode::KeyM,
        );
    }
    let step = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));

    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
    app.update();
    app.update();
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let buttons = players.iter(app.world()).next().expect("player").buttons;
    assert!(
        !buttons.contains(Buttons::HOOK_LEFT),
        "`Q` still fires the left arm although the bind moved to `M`"
    );

    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyM);
    app.update();
    let mut players = app.world_mut().query_filtered::<&Intent, With<LocalPlayer>>();
    let buttons = players.iter(app.world()).next().expect("player").buttons;
    assert!(buttons.contains(Buttons::HOOK_LEFT), "the bound key has to fire the arm");
}
