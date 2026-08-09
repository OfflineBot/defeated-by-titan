//! The guard over the camera's axis contract.
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

use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::ui::DefaultUiCamera;
use bevy::window::PrimaryWindow;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{Intent, LocalPlayer, Cli};

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
