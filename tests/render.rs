//! The guard over the camera's axis contract — **and over the two ropes.**
//!
//! Two claims live here, and they fail in the same way: the image looks plausible and shows
//! something other than what is measured. The camera half is below from
//! [`f002_look_zero_points_the_camera_at_minus_z`] on; the rope half is at the end of the file
//! and is younger. Until 2026-08-10 `render::rope::draw_ropes` was an empty body, so **no
//! pixel anywhere in this build told a player that a rope was attached** — the gas bar
//! draining was the only proxy, and every screenshot captioned "hook" or "rope" was showing
//! none.
//!
//! The rope tests do **not** stop at the geometry function. They set a hook state on the real
//! player, run the real `Update` and the real `Last`, and then read the gizmo buffer the app
//! actually handed to the renderer (`bevy_gizmos::GizmoAsset::buffer`, filled by
//! `update_gizmo_meshes` in `Last`). That is what makes them a check on the *registration*
//! too: an unregistered system leaves the buffer empty and every one of them goes red.
//!
//! **Image and aim ray have to point the same way.** Until 2026-08-09 they did not:
//! `render::camera::rotate_camera` was an empty body, the camera always looked at −Z, and
//! `intent.look_dir()` went where the player aims. A bug of this kind makes **every image
//! criterion in the project worthless** without ever drawing attention to itself: the image
//! looks plausible, it just shows something other than what is being measured.
//!
//! This file nails that equality down. It falls over when somebody flips a sign, swaps the
//! rotation order, removes the pitch clamp — or empties `rotate_camera` again.
//!
//! **Why the tests run only `Update` and not `app.update()`:** `app.update()` goes through
//! `First`, where `Time<Virtual>` is filled from **wall clock time**. Depending on the
//! machine's mood a fixed step would happen, and `net::local::read_input` would overwrite the
//! `Intent` just set with the look direction of the (nonexistent) mouse. A test whose result
//! depends on the machine's mood that day measures the machine.

use core::any::TypeId;

use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::gizmos::config::DefaultGizmoConfigGroup;
use bevy::gizmos::GizmoHandles;
use bevy::prelude::*;
use bevy::ui::DefaultUiCamera;
use bevy::window::PrimaryWindow;
use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::gizmo::GizmoToggle;
use defeated_by_titan::render::rope::rope_color;
use defeated_by_titan::shared::{BodyId, Cli, Hook, HookArm, HookState, Intent, LocalPlayer, Side};

/// Builds the **real** app, headless — not a second, similar one.
///
/// Two passes: `Commands` only take effect at the end of their run, the player comes into
/// being in `Startup`, and `render::attach_camera` only hangs the camera on him afterwards.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.update();
    app.update();
    app
}

/// Sets the desired look direction on the local player (in **degrees**, as in the script
/// language and in RON) and returns where the camera looks afterwards.
fn look(app: &mut App, yaw_deg: f32, pitch_deg: f32) -> Vec3 {
    let player = local_player(app);
    {
        let mut intent = app
            .world_mut()
            .get_mut::<Intent>(player)
            .expect("the local player has an intent");
        intent.yaw = yaw_deg.to_radians();
        intent.pitch = pitch_deg.to_radians();
    }
    app.world_mut().run_schedule(Update);
    camera_forward(app)
}

fn local_player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world())
        .next()
        .expect("there must be a local player")
}

/// The camera's forward vector.
///
/// The camera's `Transform` is **local** — it hangs off the player as a child. That this is
/// also the vector in world space rests on the player never rotating; that is exactly what
/// [`f002_the_camera_rotates_not_the_player`] checks. Falling back on `GlobalTransform` would
/// not work without running `PostUpdate` too — and with it half of the render preparation.
fn camera_forward(app: &mut App) -> Vec3 {
    let mut q = app.world_mut().query_filtered::<&Transform, With<Camera3d>>();
    let t = q
        .iter(app.world())
        .next()
        .expect("there must be a 3D camera");
    // `Dir3::as_vec3` — bevy_math-0.19.0/src/direction.rs:614.
    t.forward().as_vec3()
}

fn pitch_limit_deg(app: &App) -> f32 {
    // The number lives in assets/data/game.ron, not in the test (rule 2). A test that copies
    // it out is a lie on the day of the first change.
    app.world().resource::<GameData>().game.camera.pitch_limit_deg
}

#[test]
fn f002_look_zero_points_the_camera_at_minus_z() {
    // The axis contract from docs/conventions.md: `yaw = 0, pitch = 0` is −Z. Break it and
    // every model faces the wrong way, and nobody knows why.
    let mut app = app();
    let forward = look(&mut app, 0.0, 0.0);
    assert!(
        (forward - Vec3::NEG_Z).length() < 1e-5,
        "yaw = 0, pitch = 0 must be −Z, but was {forward:?}"
    );
}

#[test]
fn f002_image_and_ray_point_the_same_way() {
    // **The criterion that actually matters.** The aim ray follows `intent.look_dir()`, the
    // image follows where the camera looks. If those are two different directions, every
    // image measures something other than the ray — and you cannot tell by looking at it.
    let mut app = app();

    // Negative angles and values beyond 90 degrees are deliberately in the list: a swapped
    // sign or a stray `abs()` shows up exactly there and nowhere else.
    let pairs = [
        (0.0_f32, 0.0_f32),
        (30.0, -10.0),
        (-45.0, 20.0),
        (135.0, -60.0),
        (200.0, 45.0),
        (-170.0, 89.0),
        (89.9, -89.0),
    ];

    for (yaw_deg, pitch_deg) in pairs {
        let forward = look(&mut app, yaw_deg, pitch_deg);
        let ray = Intent {
            yaw: yaw_deg.to_radians(),
            pitch: pitch_deg.to_radians(),
            ..default()
        }
        .look_dir();
        assert!(
            (forward - ray).length() < 1e-5,
            "look {yaw_deg} {pitch_deg}: the camera points at {forward:?}, \
             the aim ray at {ray:?} — image and measurement drift apart"
        );
    }
}

#[test]
fn f002_pitch_stays_within_the_limit_from_game_ron() {
    // Without a clamp the camera tips over the zenith and the image stands on its head.
    // `net::local` clamps the MOUSE path only; an intent from a script or from the network
    // arrives unclamped, and that is why the camera clamps for itself.
    let mut app = app();
    let limit = pitch_limit_deg(&app);

    for (desired, expected) in [
        (120.0_f32, limit),
        (-120.0, -limit),
        (limit + 1.0, limit),
        (45.0, 45.0),
        (-45.0, -45.0),
    ] {
        let forward = look(&mut app, 0.0, desired);
        // `look_dir().y` is `sin(pitch)` — the pitch can be recovered from the direction.
        let actual = forward.y.asin().to_degrees();
        assert!(
            (actual - expected).abs() < 1e-3,
            "look 0 {desired} should have given {expected} degrees, but gave {actual} — \
             the limit from assets/data/game.ron (camera.pitch_limit_deg = {limit}) \
             is not respected"
        );
    }
}

#[test]
fn f002_the_camera_rotates_not_the_player() {
    // The collision box hangs on the player. If it turns along, the axis-aligned hull is no
    // longer axis-aligned — and collision goes wrong in a way you only notice once somebody
    // gets stuck at an angle against a wall.
    let mut app = app();
    for (yaw, pitch) in [(0.0_f32, 0.0_f32), (137.0, -42.0), (-91.0, 63.0)] {
        look(&mut app, yaw, pitch);
        let player = local_player(&mut app);
        let rotation = app
            .world()
            .get::<Transform>(player)
            .expect("the player has a transform")
            .rotation;
        assert!(
            rotation.angle_between(Quat::IDENTITY) < 1e-6,
            "after look {yaw} {pitch} the player is rotated by {} degrees — the CAMERA \
             rotates, not the player (src/render/camera.rs)",
            rotation.angle_between(Quat::IDENTITY).to_degrees()
        );
    }
}

#[test]
fn f002_rotating_does_not_move_the_eye_height() {
    // `rotate_camera` writes exactly one field. Replace the whole `Transform` by accident and
    // the camera ends up between the feet — and the image merely looks "a bit low".
    let mut app = app();
    let eye_height = app.world().resource::<GameData>().game.player.eye_height_m;

    look(&mut app, 77.0, -33.0);

    let mut q = app.world_mut().query_filtered::<&Transform, With<Camera3d>>();
    let position = q
        .iter(app.world())
        .next()
        .expect("there must be a 3D camera")
        .translation;
    assert!(
        (position - Vec3::new(0.0, eye_height, 0.0)).length() < 1e-6,
        "the camera sits at {position:?} instead of {eye_height} m eye height above the player"
    );
}

#[test]
fn the_camera_is_the_default_ui_camera() {
    // **P1 — the prerequisite the whole evidence route hangs on.**
    //
    // `DefaultUiCamera::get()` asks two questions, in this order
    // (`bevy_ui-0.19.0/src/ui_node.rs:2991-3006`):
    //
    //   1. is there exactly one camera carrying `IsDefaultUiCamera`?  → take it
    //   2. otherwise: which camera renders into the PRIMARY WINDOW?
    //
    // An `--offscreen` run has no window at all — its camera's `RenderTarget` is an `Image`
    // (`src/debug/screenshot.rs:218-222`). Question 2 therefore finds nothing, every UI root
    // keeps `ComputedUiTargetCamera::default()` = `Entity::PLACEHOLDER` and
    // `ComputedUiRenderTargetInfo::physical_size` stays `UVec2::ZERO`
    // (`bevy_ui-0.19.0/src/ui_node.rs:3019-3051`) — the HUD lays out into a zero-sized
    // viewport and **the PNG comes out without a single pixel of UI while the run reports
    // success.** That is the trap: not a crash, an empty picture that looks like evidence
    // (`docs/PLAN-GAME.md` §11, risk 3).
    //
    // So the answer has to come out of question 1, and this test is what keeps it there.
    //
    // **Measured, because the first version of this test was green without the fix and proved
    // nothing:** the fallback's first arm is `RenderTarget::Window(WindowRef::Primary) =>
    // true` — *unconditionally*, it never asks whether a primary window exists (`:2997-3003`).
    // A camera with its default target therefore answers even in a windowless app. Only the
    // swap to `RenderTarget::Image` — which is exactly what `--offscreen` does — drops it into
    // the `_ => false` arm. So the test has to make that swap itself.
    let mut app = app();

    let camera = {
        let mut q = app.world_mut().query_filtered::<Entity, With<Camera3d>>();
        q.iter(app.world()).next().expect("there must be a 3D camera")
    };

    let default_ui_camera = |app: &mut App| {
        app.world_mut()
            .run_system_once(|default: DefaultUiCamera| default.get())
            .expect("the one-shot system runs")
    };

    // The state of affairs a window run shows — and the reason nobody notices the hole:
    // with the default target the fallback answers, with or without the component.
    assert_eq!(
        default_ui_camera(&mut app),
        Some(camera),
        "not even with the default render target does a camera answer for the UI"
    );

    // What `debug::screenshot::attach_offscreen_target` does at `:218-222`, verbatim. Not a
    // real GPU texture — `DefaultUiCamera` looks at the enum variant, not at the pixels.
    let target = app.world_mut().resource_mut::<Assets<Image>>().reserve_handle();
    app.world_mut()
        .entity_mut(camera)
        .insert(RenderTarget::Image(target.into()));

    assert_eq!(
        default_ui_camera(&mut app),
        Some(camera),
        "the camera renders into an `Image`, as under `--offscreen`, and now NO camera answers \
         for the UI. Every UI root then keeps `Entity::PLACEHOLDER` and a physical size of \
         0x0 — every HUD screenshot from here on comes out empty and still exits 0. \
         `IsDefaultUiCamera` belongs on the camera in `render::attach_camera`"
    );

    // And the window fallback must not be what saved it: no window here, no primary camera.
    let windows = {
        let mut q = app.world_mut().query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(app.world()).count()
    };
    assert_eq!(windows, 0, "this app has a window — then the assertion above proves nothing");
}

// ---------------------------------------------------------------------------
// F-004 — the ropes. See the file header for why these go through the buffer.
// ---------------------------------------------------------------------------

/// Everything the app really drew into the default gizmo group this frame, as
/// `(start, end, colour)` — one entry per line.
///
/// The route is the app's own: `Gizmos` in `Update` flushes into
/// `GizmoStorage<DefaultGizmoConfigGroup, ()>`, and `update_gizmo_meshes` in `Last` moves that
/// storage whole into a [`GizmoAsset`] (`bevy_gizmos-0.19.0/src/lib.rs:288-320`). The storage
/// itself is `pub(crate)` and unreadable from out here; the asset's buffer is public, and it
/// is the same bytes. **Only `Update` and `Last` are run, never `app.update()`** — see the
/// header: a fixed step would let `vector::hook`, the only writer of [`Hook`], overwrite the
/// state the test just set, and the test would be measuring the machine's mood.
///
/// `line` writes a **pair into the list buffer**, no strip and no separator
/// (`bevy_gizmos-0.19.0/src/gizmos.rs:412`), so chunks of two are lines and nothing else.
fn drawn_lines(app: &mut App) -> Vec<(Vec3, Vec3, LinearRgba)> {
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Last);

    let handle = app
        .world()
        .resource::<GizmoHandles>()
        .handles()
        .get(&TypeId::of::<DefaultGizmoConfigGroup>())
        .cloned()
        .flatten();
    // `update_gizmo_meshes` puts the slot back to `None` when nothing was drawn — that is the
    // honest empty, not a missing group.
    let Some(handle) = handle else {
        return Vec::new();
    };
    let assets = app.world().resource::<Assets<GizmoAsset>>();
    let view = assets.get(&handle).expect("the gizmo asset the handle names").buffer();
    assert!(
        view.strip_positions.is_empty(),
        "something drew a line STRIP into the default group — this reader only understands \
         lines, and the counts below would be wrong without saying so"
    );
    view.list_positions
        .chunks(2)
        .zip(view.list_colors.chunks(2))
        .map(|(p, c)| (p[0], p[1], c[0]))
        .collect()
}

/// Puts a hook state on the local player and returns his origin — the point a rope starts at.
fn set_hook(app: &mut App, hook: Hook) -> Vec3 {
    // Nobody else may be drawing into the default group, or "nothing was drawn" proves
    // nothing. `debug::gizmo`'s three systems hang behind `gizmos_on`, and `DBT_GIZMOS` is an
    // environment variable — so this is a real possibility and not a formality.
    assert!(
        !app.world().resource::<GizmoToggle>().on,
        "DBT_GIZMOS is set: the debug gizmos are drawing into the same group and every count \
         in this file is meaningless. Run these tests without it."
    );
    let player = local_player(app);
    app.world_mut().entity_mut(player).insert(hook);
    app.world()
        .get::<Transform>(player)
        .expect("the player has a transform")
        .translation
}

/// One arm, anchored, with its tip at `tip_m`.
fn anchored_arm(tip_m: Vec3) -> HookArm {
    HookArm { state: HookState::Anchored { body: BodyId(7), local_m: Vec3::ZERO }, tip_m }
}

#[test]
fn f004_an_anchored_arm_puts_a_line_on_the_screen() {
    // The claim the whole file exists for: a rope is VISIBLE. Before 2026-08-10 this ran green
    // on an empty `draw_ropes` for nobody, because nobody had written it.
    let mut app = app();
    let tip = Vec3::new(24.0, 11.04, -34.0);
    let mut hook = Hook::default();
    hook.arms[Side::Right.index()] = anchored_arm(tip);
    let origin = set_hook(&mut app, hook);

    let lines = drawn_lines(&mut app);
    assert_eq!(lines.len(), 1, "one anchored arm has to be exactly one line, got {lines:?}");
    let (start, end, _) = lines[0];
    assert!(
        (start - origin).length() < 1e-4,
        "the rope starts at {start:?} instead of at the player's origin {origin:?}"
    );
    assert!(
        (end - tip).length() < 1e-4,
        "the rope ends at {end:?} instead of at the anchor {tip:?}"
    );
}

#[test]
fn f004_only_an_anchored_arm_draws_a_rope() {
    // **The half that makes the other half falsifiable.** `tip_m` is live while `Flying` and
    // `Retracting` too — a file that drew those would put a cyan line on screen for a
    // projectile, and "there is a line" would stop meaning "he is attached".
    let mut app = app();
    let tip = Vec3::new(24.0, 11.04, -34.0);

    for state in [
        HookState::Idle,
        HookState::Flying { target_m: tip, body: BodyId(7) },
        HookState::Retracting,
    ] {
        let hook = Hook { arms: [HookArm { state, tip_m: tip }; 2] };
        set_hook(&mut app, hook);
        let lines = drawn_lines(&mut app);
        assert!(lines.is_empty(), "{state:?} drew {} lines and must draw none", lines.len());
    }

    // And the same app, same tick, one field different: now there is a rope. Without this the
    // test above would also be green on an app that draws nothing at all, ever.
    let mut hook = Hook::default();
    hook.arms[Side::Left.index()] = anchored_arm(tip);
    set_hook(&mut app, hook);
    assert_eq!(drawn_lines(&mut app).len(), 1, "an anchored left arm is one rope");
}

#[test]
fn f004_the_two_arms_draw_independently() {
    // `F-001` verbatim: two independently steerable hooks. Left anchored with the right free
    // is one line — not two, not none — and a swapped index cannot pass both halves.
    let mut app = app();
    let left_tip = Vec3::new(-8.0, 9.0, -14.0);
    let right_tip = Vec3::new(8.0, 9.0, -14.0);

    let mut hook = Hook::default();
    hook.arms[Side::Left.index()] = anchored_arm(left_tip);
    set_hook(&mut app, hook);
    let lines = drawn_lines(&mut app);
    assert_eq!(lines.len(), 1, "left anchored, right free is one line");
    assert!((lines[0].1 - left_tip).length() < 1e-4, "and it is the LEFT anchor it reaches");

    let mut hook = Hook::default();
    hook.arms[Side::Right.index()] = anchored_arm(right_tip);
    set_hook(&mut app, hook);
    let lines = drawn_lines(&mut app);
    assert_eq!(lines.len(), 1, "right anchored, left free is one line");
    assert!((lines[0].1 - right_tip).length() < 1e-4, "and it is the RIGHT anchor it reaches");

    let mut hook = Hook::default();
    hook.arms[Side::Left.index()] = anchored_arm(left_tip);
    hook.arms[Side::Right.index()] = anchored_arm(right_tip);
    set_hook(&mut app, hook);
    let lines = drawn_lines(&mut app);
    assert_eq!(lines.len(), 2, "both anchored is two lines");
    let mut ends: Vec<Vec3> = lines.iter().map(|(_, end, _)| *end).collect();
    ends.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert!((ends[0] - left_tip).length() < 1e-4);
    assert!((ends[1] - right_tip).length() < 1e-4);
}

#[test]
fn f004_a_rope_is_the_cyan_out_of_maps_ron() {
    // `docs/conventions.md` §3 reserves cyan for gas, Vector Gear and anchor points, and a
    // rope is Vector Gear. The number is not in the test: it is read out of the same file the
    // game reads, so the day somebody re-tunes the signal colours this stays true instead of
    // going red for the wrong reason (rule 2).
    let mut app = app();
    let mut hook = Hook::default();
    hook.arms[Side::Right.index()] = anchored_arm(Vec3::new(0.0, 9.0, -14.0));
    set_hook(&mut app, hook);

    let expected = {
        let data = app.world().resource::<GameData>();
        rope_color(data).to_linear()
    };
    // The route the value took, spelled out, so the test cannot pass by comparing the
    // function with itself: `signals:` -> `rope_color` -> the gizmo buffer.
    let (r, g, b) = app.world().resource::<GameData>().maps.signals["cyan"];
    assert_eq!(expected, Color::linear_rgb(r, g, b).to_linear(), "rope_color is not maps.ron");

    let lines = drawn_lines(&mut app);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].2, expected, "a rope is drawn in a colour that is not maps.ron's cyan");
}
