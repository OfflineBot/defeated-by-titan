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

use avian3d::prelude::Collider;
use bevy::animation::graph::AnimationGraph;
use bevy::animation::{AnimationClip, AnimationPlayer, RepeatAnimation};
use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::gizmos::config::DefaultGizmoConfigGroup;
use bevy::gizmos::GizmoHandles;
use bevy::prelude::*;
use bevy::ui::DefaultUiCamera;
use bevy::window::PrimaryWindow;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use defeated_by_titan::data::{GameData, Model, ModelSource};
use defeated_by_titan::debug::gizmo::GizmoToggle;
use bevy::camera::Exposure;
use bevy::light::NotShadowCaster;
use bevy::mesh::VertexAttributeValues;
use defeated_by_titan::render::light::{to_sun, InteriorLamp, SkyDome};
use defeated_by_titan::render::model::{
    load_configured_models, ModelAnchors, ModelAssets, ModelBody, ModelName, PendingScene,
    PrimitiveFallback, CORTEX_ANCHOR,
};
use defeated_by_titan::render::rope::rope_color;
use defeated_by_titan::shared::{
    BodyId, Cli, Hook, HookArm, HookState, Intent, LocalPlayer, Side, TitanKindName, TitanState,
};
use std::collections::BTreeMap;

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

// ---------------------------------------------------------------------------
// The model registry — `assets/data/art.ron` decides, and BOTH answers are normal.
//
// The user, 2026-08-12: *„mach zudem, dass ich später einfach die 3d modelle austauschen kann
// + eigene animationen adden kann!"* These tests are the half of that sentence that can be
// checked without a single `.glb` in the repository — and "without a single `.glb`" is not a
// limitation of the tests, it is the **requirement**: every visible thing in this game is a
// primitive cuboid today, and it has to stay that way until somebody puts a file in.
//
// **Why they mutate `GameData` instead of adding a line to `art.ron`:** the swap has to be
// provable while every shipped entry is still a placeholder. A test entry in the real registry
// would need a file that does not exist, and would make `cargo run` print an error for ever.
// The types are the real ones, the systems are the real ones, the app is the real one — only
// the row is the test's own.
// ---------------------------------------------------------------------------

/// Both halves of the switch spawn into the same app, so a green result cannot come from two
/// differently built worlds.
fn art_entry(source: ModelSource, animations: &[(&str, &str)]) -> Model {
    Model {
        source,
        scale: 1.0,
        attribution: None,
        animations: animations
            .iter()
            .map(|(state, clip)| ((*state).to_string(), (*clip).to_string()))
            .collect(),
    }
}

/// Puts a row into the registry the way `art.ron` would, then does what `Startup` does with it.
fn register(app: &mut App, name: &str, model: Model) {
    app.world_mut()
        .resource_mut::<GameData>()
        .art
        .models
        .insert(name.to_string(), model);
    // `load_configured_models` is a `Startup` system and has already run. Running it again by
    // hand is what makes this a test of the REAL loader rather than of a handle the test made
    // up itself.
    app.world_mut()
        .run_system_once(load_configured_models)
        .expect("the loader runs");
}

#[test]
fn f030_a_model_without_a_file_stays_the_primitive_it_is_today() {
    // **The non-negotiable.** `titan_husk` is `source: Primitive` in the shipped `art.ron`, and
    // every titan in the game points at it. If this goes red the game has started demanding a
    // file that is not in the repository — and the symptom is an empty world, not an error.
    let mut app = app();
    let entity = app.world_mut().spawn(ModelName::new("titan_husk")).id();
    app.world_mut().run_schedule(Update);

    assert_eq!(
        app.world().get::<ModelBody>(entity),
        Some(&ModelBody::Primitive),
        "a `source: Primitive` row must leave the entity alone — whatever cuboids are already \
         standing on it are the model"
    );
    assert!(
        app.world().get::<Children>(entity).is_none(),
        "nothing may be spawned under a primitive: the rig owns those children"
    );
    assert_eq!(
        app.world().get::<ModelAnchors>(entity).map(ModelAnchors::is_empty),
        Some(true),
        "a primitive has no anchors OUT OF A FILE — an empty map is how a reader is told to \
         ask the rig instead of being handed a Vec3::ZERO kill zone"
    );
}

#[test]
fn f030_a_configured_model_spawns_a_scene_instead_of_the_primitive() {
    // The other half, and the one that makes the half above falsifiable: same app, same tick,
    // one line of RON different — and now something is spawned.
    let mut app = app();
    register(&mut app, "swapped", art_entry(ModelSource::Gltf("3d/glb/swapped.glb".into()), &[]));

    let entity = app.world_mut().spawn(ModelName::new("swapped")).id();
    app.world_mut().run_schedule(Update);

    let body = app
        .world()
        .get::<ModelBody>(entity)
        .copied()
        .expect("the registry has to have decided something");
    let ModelBody::Scene(scene) = body else {
        panic!(
            "art.ron says `source: Gltf(..)` and the entity still came out as {body:?} — \
             then no swap is possible and the switch is decoration"
        );
    };
    assert_eq!(
        app.world().get::<ChildOf>(scene).map(ChildOf::parent),
        Some(entity),
        "the scene has to hang UNDER the entity: the simulation owns the entity's transform, \
         the model owns only its own scale (docs/architecture.md)"
    );
    // The file does not exist, so what the child carries is the handle in flight, not a
    // finished scene. That is the honest assertion — and `attach_late_scenes` is what turns it
    // into a `WorldAssetRoot` on the frame the file arrives.
    assert!(
        app.world().get::<PendingScene>(scene).is_some()
            || app.world().get::<WorldAssetRoot>(scene).is_some(),
        "the child carries neither a pending glTF handle nor a scene root — nothing will ever \
         be rendered under it"
    );
}

#[test]
fn f030_an_unknown_model_name_never_takes_the_geometry_away() {
    // The direction that costs a session: a typo in `titan.ron` must not leave a titan
    // invisible. Falling back to the primitive is the only safe answer, and it is said out
    // loud once.
    let mut app = app();
    let entity = app.world_mut().spawn(ModelName::new("no_such_model")).id();
    app.world_mut().run_schedule(Update);

    assert_eq!(
        app.world().get::<ModelBody>(entity),
        Some(&ModelBody::Primitive),
        "an unknown logical name must fall back to the primitive, not to nothing"
    );
}

#[test]
fn f030_the_repository_runs_with_not_a_single_glb() {
    // **The requirement, checked on the file that ships** — not on a fixture. The day somebody
    // sets a row to `Gltf(..)` without committing the file, this is what says so.
    let app = app();
    let data = app.world().resource::<GameData>();
    let configured: Vec<&String> = data
        .art
        .models
        .iter()
        .filter(|(_, m)| !matches!(m.source, ModelSource::Primitive))
        .map(|(name, _)| name)
        .collect();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for name in &configured {
        let ModelSource::Gltf(path) = &data.art.models[*name].source else {
            continue;
        };
        assert!(
            root.join(path).is_file(),
            "art.ron: model {name:?} points at {path:?}, and assets/{path} is not there. \
             Either commit the file (assets/3d/glb/ is TRACKED, docs/models.md) or put the row \
             back to `source: Primitive`"
        );
    }
    assert_eq!(
        app.world().resource::<ModelAssets>().gltf.len(),
        configured.len(),
        "the loader asked for a different number of files than art.ron configures"
    );
}

#[test]
fn f030_a_titan_gets_its_model_and_its_cortex_yardstick_out_of_the_ron() {
    // The wiring, and it costs no domain edge: `TitanKindName` is `shared`, `titan.ron` and
    // `scale.ron` are `data` — both free for every domain. `render` never learns that a domain
    // `titan` exists (docs/architecture.md).
    let mut app = app();
    let entity = app.world_mut().spawn(TitanKindName::new("husk")).id();
    app.world_mut().run_schedule(Update);

    let named = app.world().get::<ModelName>(entity).cloned().expect("a titan gets a model name");
    let data = app.world().resource::<GameData>();
    let kind = data.titan("husk").expect("husk stands in titan.ron");
    assert_eq!(named.name, kind.model, "the model name has to come out of titan.ron");
    assert_eq!(
        named.cortex_height_m,
        data.titan_cortex_height_m(kind),
        "the cortex yardstick has to come out of scale.ron — a swapped model is checked \
         against it, and a number copied into Rust is a lie on the day of the first change"
    );
}

#[test]
fn f030_a_named_clip_that_is_not_in_the_file_leaves_the_state_without_one() {
    // **The animation seam, and its only promise: silence is not an option, and a wrong name
    // is not fatal.** `windup` here names a clip that cannot possibly resolve, because there
    // is no file at all. The state must simply have no clip — a fabricated fallback clip
    // would be the trap `docs/models.md` warns about, where the model animates the wrong
    // thing and nothing says so.
    let mut app = app();
    register(
        &mut app,
        "animated",
        art_entry(
            ModelSource::Gltf("3d/glb/animated.glb".into()),
            &[("idle", "Idle"), ("windup", "NoSuchClip")],
        ),
    );

    for _ in 0..20 {
        app.update();
    }

    let assets = app.world().resource::<ModelAssets>();
    let resolved = assets.clips.get("animated");
    assert!(
        resolved.is_none_or(|c| !c.contains_key("windup")),
        "a clip name that is not in the file must leave that state EMPTY, and the loader must \
         say so — it resolved {resolved:?} instead"
    );
    assert!(
        app.world().get_resource::<GameData>().is_some(),
        "the app has to still be alive: a missing model is a warning, never a crash"
    );
}

// ---------------------------------------------------------------------------
// F-030 · the swap, end to end — **on a synthetic scene, because there is no `.glb`**
//
// `assets/3d/glb/` is empty on purpose and must stay empty (`f030_the_repository_runs_with_not
// _a_single_glb` above). So these tests build the thing a `.glb` would have become: a
// `WorldAsset` with named empties in it, handed to the registry the way the loader would hand
// it over. The types are the real ones, the spawner is Bevy's own, the observer is the real
// one — what is synthetic is the file, and only the file.
//
// **What that does and does not prove** is worth being exact about: it proves the anchor walk,
// the `WorldAssetRoot` handoff, the visibility switch and the clip driver on a REAL spawned
// instance. It does not prove that Blender's exporter writes the empties under those names,
// that the model arrives upright, or that a glTF's own `AnimationPlayer` sits where this code
// looks for it. Those need a file and a screen (`docs/models.md`).
// ---------------------------------------------------------------------------

/// The stand-in for an exported `.glb`: a world with one root and one named empty per anchor.
fn synthetic_model(app: &mut App, anchors: &[(&str, Vec3)]) -> Handle<WorldAsset> {
    let mut world = World::new();
    let root = world.spawn((Name::new("model_root"), Transform::default())).id();
    for (name, at) in anchors {
        let empty = world
            .spawn((Name::new((*name).to_string()), Transform::from_translation(*at)))
            .id();
        world.entity_mut(root).add_child(empty);
    }
    app.world_mut()
        .resource_mut::<Assets<WorldAsset>>()
        .add(WorldAsset::new(world))
}

/// Registers a row **and** hands the registry a finished scene for it, the way
/// `resolve_animation_clips` would once the file is in memory.
fn register_loaded(
    app: &mut App,
    name: &str,
    animations: &[(&str, &str)],
    anchors: &[(&str, Vec3)],
    clips: &[&str],
) {
    register(
        app,
        name,
        art_entry(ModelSource::Gltf(format!("3d/glb/{name}.glb")), animations),
    );
    let scene = synthetic_model(app, anchors);
    let resolved: BTreeMap<String, Handle<AnimationClip>> = clips
        .iter()
        .map(|state| {
            let clip = app
                .world_mut()
                .resource_mut::<Assets<AnimationClip>>()
                .add(AnimationClip::default());
            ((*state).to_string(), clip)
        })
        .collect();
    let mut assets = app.world_mut().resource_mut::<ModelAssets>();
    assets.scenes.insert(name.to_string(), scene);
    assets.clips.insert(name.to_string(), resolved);
}

/// An entity that looks like the titan rig from the outside: a body with one cuboid child that
/// carries a collider — the two things that must **not** share a fate.
fn body_with_a_cuboid(app: &mut App, name: &str) -> (Entity, Entity) {
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::new(1.0, 1.0, 1.0));
    let part = app
        .world_mut()
        .spawn((
            Name::new("primitive_part"),
            Mesh3d(mesh),
            Collider::sphere(0.5),
            Transform::from_xyz(0.0, 2.0, 0.0),
        ))
        .id();
    let body = app.world_mut().spawn((ModelName::new(name), Transform::default())).id();
    app.world_mut().entity_mut(body).add_child(part);
    (body, part)
}

fn visibility(app: &App, entity: Entity) -> Visibility {
    app.world().get::<Visibility>(entity).copied().unwrap_or(Visibility::Inherited)
}

#[test]
fn f030_a_model_that_arrived_hides_the_cuboid_it_replaces() {
    // **Gap 1.** Until 2026-08-12 the scene spawned BESIDE the rig and both were visible — for
    // a titan that is two bodies standing in one place, and it made every swap look broken.
    // The other half of the assertion is the one that matters more: the collider does not go
    // with the picture. A hidden cortex still kills.
    let mut app = app();
    register_loaded(&mut app, "swapped", &[], &[], &[]);
    let (_body, part) = body_with_a_cuboid(&mut app, "swapped");
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        visibility(&app, part),
        Visibility::Hidden,
        "the model arrived and the cuboid it replaces is still drawn — that is two bodies in \
         one place, which is what a swap looked like before this line existed"
    );
    assert!(
        app.world().get::<Collider>(part).is_some(),
        "hiding the picture took the collider with it — the titan would render right and be \
         unhittable, which is worse than the doubled silhouette it replaced"
    );
    let placed = app
        .world()
        .get::<GlobalTransform>(part)
        .expect("the part keeps a global transform")
        .translation();
    assert_eq!(
        placed,
        Vec3::new(0.0, 2.0, 0.0),
        "hiding must not move anything: the cortex sits where scale.ron put it, visible or not"
    );

    // And the other direction, in the same app: a primitive row leaves its cuboids alone.
    let (_, untouched) = body_with_a_cuboid(&mut app, "titan_husk");
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        visibility(&app, untouched),
        Visibility::Inherited,
        "a `source: Primitive` row must never hide anything — the cuboid IS the model there"
    );
}

#[test]
fn f030_a_file_that_never_loads_leaves_the_cuboid_standing() {
    // The safe direction, and the one `docs/models.md` promises for a wrong path: the entity
    // keeps its cuboid. The trigger for hiding is therefore the scene that ARRIVED, never the
    // row that was configured — `swapped_missing` points at a file that does not exist.
    let mut app = app();
    register(
        &mut app,
        "swapped_missing",
        art_entry(ModelSource::Gltf("3d/glb/swapped_missing.glb".into()), &[]),
    );
    let (_, part) = body_with_a_cuboid(&mut app, "swapped_missing");
    for _ in 0..8 {
        app.update();
    }

    assert_eq!(
        visibility(&app, part),
        Visibility::Inherited,
        "the file never loaded, so there is nothing to see except the cuboid — hiding it would \
         make a typo in art.ron into an invisible titan"
    );
}

#[test]
fn f030_the_game_state_plays_the_clip_that_is_mapped_to_it() {
    // **Gap 2.** Until 2026-08-12 the clips were resolved by state name and NOTHING played
    // them: no `AnimationPlayer`, no graph. `TitanState` is the honest source — it is what the
    // FSM already decides and what the F3 overlay prints, so an animation cannot disagree with
    // the game.
    let mut app = app();
    register_loaded(
        &mut app,
        "animated_husk",
        &[("idle", "Idle"), ("windup", "Windup")],
        &[],
        &["idle", "windup"],
    );
    let (body, _) = body_with_a_cuboid(&mut app, "animated_husk");
    app.world_mut().entity_mut(body).insert(TitanState::Idle);
    for _ in 0..6 {
        app.update();
    }

    let ModelBody::Scene(scene) = *app
        .world()
        .get::<ModelBody>(body)
        .expect("the registry decided something")
    else {
        panic!("a configured model has to produce a scene");
    };
    let nodes = app.world().resource::<ModelAssets>().graphs["animated_husk"].nodes.clone();
    let player = app
        .world()
        .get::<AnimationPlayer>(scene)
        .expect("nothing plays the clips — the animation seam is a lookup table and no player");
    assert!(
        player.is_playing_animation(nodes["idle"]),
        "the titan is Idle and the idle clip is not playing"
    );
    assert_eq!(
        player.animation(nodes["idle"]).map(|a| a.repeat_mode()),
        Some(RepeatAnimation::Forever),
        "idle has to loop — a one-shot idle is a body that animates once and then stands still"
    );

    // The edge: one state in, one clip out, and a wind-up plays ONCE.
    *app.world_mut().get_mut::<TitanState>(body).expect("the state is there") = TitanState::Windup;
    for _ in 0..3 {
        app.update();
    }
    let player = app.world().get::<AnimationPlayer>(scene).expect("the player stays");
    assert!(
        player.is_playing_animation(nodes["windup"]),
        "the state changed and the clip did not — that is the seam being decoration again"
    );
    assert!(
        !player.is_playing_animation(nodes["idle"]),
        "two clips at once: this seam plays one state, and blending is a decision nobody made"
    );
    assert_eq!(
        player.animation(nodes["windup"]).map(|a| a.repeat_mode()),
        Some(RepeatAnimation::Never),
        "a looping wind-up is a titan that telegraphs for ever (F-053)"
    );
    assert!(
        !app.world().resource::<Assets<AnimationGraph>>().is_empty(),
        "the graph asset has to exist for the player to read a node out of"
    );
}

#[test]
fn f030_a_state_without_a_clip_brings_the_cuboid_back() {
    // **The fourth glTF trap** (`docs/FINDINGS.md` FIND-053) and the worst one: the model
    // spawns, renders, is the right size and stands perfectly still. There is no visual symptom
    // — so this makes one. A model that DECLARES animations and cannot show the state the game
    // is in gets its cuboid rig back, and the rig is the thing `titan::pose` actually animates.
    let mut app = app();
    register_loaded(
        &mut app,
        "half_animated",
        &[("idle", "Idle"), ("windup", "Windup")],
        &[],
        &["idle"], // `Windup` is not in the file
    );
    let (body, part) = body_with_a_cuboid(&mut app, "half_animated");
    app.world_mut().entity_mut(body).insert(TitanState::Idle);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        visibility(&app, part),
        Visibility::Hidden,
        "idle resolved, so the model can show what the game is doing and the cuboid goes"
    );

    *app.world_mut().get_mut::<TitanState>(body).expect("the state is there") = TitanState::Windup;
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().get::<PrimitiveFallback>(body),
        Some(&PrimitiveFallback(true)),
        "a state the model has no clip for has to be MARKED, not swallowed"
    );
    assert_eq!(
        visibility(&app, part),
        Visibility::Inherited,
        "the model cannot show the wind-up and nothing came back — that is a titan that \
         telegraphs invisibly, and the player dies without being told why (F-053)"
    );

    // …and a model that says `animations: {}` is not broken, it is static. It keeps its cuboid
    // hidden, because that is what the user asked for in one line of RON.
    let mut app = self::app();
    register_loaded(&mut app, "static_model", &[], &[], &[]);
    let (body, part) = body_with_a_cuboid(&mut app, "static_model");
    app.world_mut().entity_mut(body).insert(TitanState::Windup);
    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        app.world().get::<PrimitiveFallback>(body).copied(),
        Some(PrimitiveFallback(false)),
        "`animations: {{}}` is a legal answer — a static model must not be reported as broken"
    );
    assert_eq!(visibility(&app, part), Visibility::Hidden);
}

#[test]
fn f030_the_anchors_come_out_of_a_spawned_instance() {
    // The anchor walk had never had an instance under it: `read_the_models_anchors` was
    // compiled and reasoned about, not observed (`docs/FINDINGS.md` FIND-052). This spawns a
    // real `WorldAsset` through Bevy's own spawner and reads what lands on the entity.
    let mut app = app();
    register_loaded(
        &mut app,
        "anchored",
        &[],
        &[(CORTEX_ANCHOR, Vec3::new(0.0, 9.4, 0.5)), ("hook.l", Vec3::new(-0.8, 6.0, 0.0))],
        &[],
    );
    let (body, _) = body_with_a_cuboid(&mut app, "anchored");
    for _ in 0..8 {
        app.update();
    }

    let anchors = app
        .world()
        .get::<ModelAnchors>(body)
        .expect("every entity with a ModelName carries anchors")
        .clone();
    assert_eq!(
        anchors.get(CORTEX_ANCHOR),
        Some(Vec3::new(0.0, 9.4, 0.5)),
        "the `cortex` empty out of the file did not arrive — then a swapped model renders in \
         one place and dies in another (F-030)"
    );
    assert_eq!(anchors.get("hook.l"), Some(Vec3::new(-0.8, 6.0, 0.0)));
    assert_eq!(
        anchors.get("hand.r"),
        None,
        "an empty the model does not carry must be ABSENT, never Vec3::ZERO — that would be a \
         kill zone between the feet"
    );
}

// ---------------------------------------------------------------------------
// FIND-071 — the light. **"alles sehr flat"**, and it was arithmetic.
//
// The user said it twice, once in the first window session and again on 2026-08-12. The second
// time it was measured out of `docs/images/f003-light-before.png`: a vertical wall face read
// luminance 183.2 and the ground beside it 183.3 — the same value on two surfaces at right
// angles, because `illuminance 10_000` against Bevy's default `Exposure::BLENDER` (ev100 9.7)
// puts every face with NdotL > 0.73 **over 1.0**, and a clipped face has neither colour nor
// orientation left.
//
// These tests do not check that the picture is pretty. They check the three things that decide
// whether it *can* be: the sun points where the file says, the four faces of a box get four
// different amounts of light and none of them clips, and the sky is more than one colour.
// ---------------------------------------------------------------------------

/// Albedo of `stone_gray` — the most common surface in the district (`maps.ron: palette`).
/// The reference surface every luminance below is computed for.
const STONE_GRAY_G: f32 = 0.43;

/// Lambert: what a face with this normal gets, in **linear output units after exposure**.
/// 1.0 is the clip. This is Bevy's own path, shortened to the diffuse term:
/// `albedo/pi * illuminance * NdotL * exposure` (`bevy_pbr .. pbr_functions.wgsl:863`).
fn lit(sun_dir: Vec3, normal: Vec3, illuminance: f32, exposure: f32) -> f32 {
    let n_dot_l = (-sun_dir).dot(normal).max(0.0);
    STONE_GRAY_G / core::f32::consts::PI * illuminance * n_dot_l * exposure
}

/// The one `DirectionalLight` and the transform it was aimed with.
fn the_sun(app: &mut App) -> (DirectionalLight, GlobalTransform) {
    let mut q = app.world_mut().query::<(&DirectionalLight, &GlobalTransform)>();
    let found: Vec<_> = q.iter(app.world()).map(|(l, t)| (l.clone(), *t)).collect();
    assert_eq!(found.len(), 1, "there is exactly one sun, spawned by render::light::setup_sun");
    found.into_iter().next().unwrap()
}

#[test]
fn f071_the_sun_stands_where_art_ron_says() {
    // `to_sun` is the whole aiming convention in one function, and it is the game's convention
    // and not Bevy's: yaw 0 = -Z, +90 = +X (docs/conventions.md). A flipped sign here lights the
    // district from below and nobody notices, because a uniformly lit box looks the same as a
    // uniformly unlit one once the exposure clips.
    let up = to_sun(0.0, 90.0);
    assert!(up.abs_diff_eq(Vec3::Y, 1e-5), "elevation 90 is straight up, got {up}");
    let north = to_sun(0.0, 0.0);
    assert!(north.abs_diff_eq(Vec3::NEG_Z, 1e-5), "azimuth 0 is -Z, got {north}");
    let east = to_sun(90.0, 0.0);
    assert!(east.abs_diff_eq(Vec3::X, 1e-5), "azimuth 90 is +X, got {east}");

    let mut app = app();
    let k = app.world().resource::<GameData>().art.lighting.sun.clone();
    let (_, transform) = the_sun(&mut app);

    // The light travels the OTHER way from the direction the sun stands in.
    let want = -to_sun(k.azimuth_deg, k.elevation_deg);
    let got = transform.forward().as_vec3();
    assert!(
        got.abs_diff_eq(want, 1e-4),
        "the sun points {got}, art.ron says {want} (azimuth {} deg, elevation {} deg) — a sun \
         aimed anywhere else makes every NdotL below a different number",
        k.azimuth_deg,
        k.elevation_deg
    );
    assert!(
        got.y < 0.0,
        "the light has to travel DOWNWARD; it travels {got} and the district is lit from below"
    );
}

#[test]
fn f071_a_roof_a_sunlit_wall_and_a_shaded_wall_are_three_different_values() {
    // **This is the test the user's sentence turns into.** Four faces of one box, four numbers,
    // and the gap between them is what "not flat" means. It falls over for the old settings
    // (illuminance 10_000, ev100 9.7 -> roof and sunlit wall both clip at 1.0 and become one
    // value), and it falls over for a sun straight overhead (all four walls identical).
    let mut app = app();
    let (light, transform) = the_sun(&mut app);
    let exposure = {
        let mut q = app.world_mut().query_filtered::<&Exposure, With<Camera3d>>();
        *q.iter(app.world()).next().expect("the camera carries an Exposure out of art.ron")
    };
    let dir = transform.forward().as_vec3();
    let e = exposure.exposure();

    let roof = lit(dir, Vec3::Y, light.illuminance, e);
    let east = lit(dir, Vec3::X, light.illuminance, e);
    let south = lit(dir, Vec3::Z, light.illuminance, e);
    let west = lit(dir, Vec3::NEG_X, light.illuminance, e);
    let north = lit(dir, Vec3::NEG_Z, light.illuminance, e);

    // 1. Nothing clips. This is the actual bug: with the old pair the brightest face landed at
    //    1.10 and every face above NdotL 0.73 came out the same white.
    let brightest = roof.max(east).max(south);
    assert!(
        brightest < 1.0,
        "the brightest stone_gray face is at {brightest:.3} of the clip — over 1.0 it has no \
         colour and no orientation left, which is exactly the state FIND-071 measured \
         (illuminance {}, ev100 {})",
        light.illuminance,
        exposure.ev100
    );

    // 2. Three lit faces, three values, and the gaps are big enough to see. 0.08 in linear
    //    output is roughly 20 sRGB steps at these levels — a visible step, not dithering.
    for (a, an, b, bn) in
        [(roof, "roof", east, "+X wall"), (roof, "roof", south, "+Z wall"), (east, "+X wall", south, "+Z wall")]
    {
        assert!(
            (a - b).abs() > 0.08,
            "{an} is at {a:.3} and {bn} at {b:.3} — that is one value, and one value is what \
             the user calls flat"
        );
    }

    // 3. Two faces get no sun at all and live on the fill alone. Without them there is no
    //    "wall in shade" to be a third value.
    assert_eq!((west, north), (0.0, 0.0), "every wall of a box gets sun — then none of them reads as shaded");

    // 4. And the fill is a real fraction of the sun, not a floor and not a second sun.
    let ambient = {
        let mut q = app.world_mut().query_filtered::<&AmbientLight, With<Camera3d>>();
        q.iter(app.world()).next().expect("the camera carries an AmbientLight").clone()
    };
    let fill = STONE_GRAY_G * ambient.color.to_linear().green * ambient.brightness * e;
    let ratio = fill / brightest;
    assert!(
        (0.04..=0.20).contains(&ratio),
        "the shaded side sits at {:.1} % of the sunlit side ({fill:.3} vs {brightest:.3}) — \
         under 4 % a shadow is a hole you cannot read a roof out of at 30 m/s, over 20 % it is \
         not a shadow",
        ratio * 100.0
    );
    // And it is COOL against a warm sun — that split is half of what makes a face read as
    // shaded rather than as dark grey.
    let fill_rgb = ambient.color.to_linear();
    let sun_rgb = light.color.to_linear();
    assert!(
        fill_rgb.blue > fill_rgb.red && sun_rgb.red >= sun_rgb.blue,
        "fill {fill_rgb:?} against sun {sun_rgb:?} — the fill has to be the cooler of the two"
    );
}

#[test]
fn f019_a_roofed_room_lives_on_its_lamp_and_the_fill_is_the_second_term() {
    // **Retired and rewritten on 2026-08-13, and the premise is why.** Until today this test was
    // called `f019_the_fill_lifts_the_darkest_material_inside_a_roofed_room` and it floored
    // `art.ron: lighting.ambient.brightness` at 3830, on the argument that *under a roof there is
    // no sunlit surface anywhere in the frame, so the only contrast left is between two
    // ambient-lit materials*. That argument was true when it was written (`FIND-078`) and it
    // stopped being true the moment the hall got a lamp of its own (`FIND-080`,
    // `maps.ron: ashgate.lights`): the room is now lit BY something, and the fill is no longer
    // the only term in it.
    //
    // ⚠️ Lowering the number without moving this test would have left a red suite; deleting this
    // test without replacing it would have thrown a guard away for nothing. So the claim moves
    // instead of disappearing, and it moves onto the failure that is now the real one:
    //
    //   **somebody raises the world's fill instead of lighting the room.**
    //
    // That is not hypothetical — it is exactly what happened here on 2026-08-13 06:00, when the
    // fill went 2400 -> 4200 as a documented mitigation and the exterior paid for it 1:1
    // (shadow-against-sun 29.1 % -> 38.0 % in sRGB, measured; `FIND-071` is the contrast that
    // buys). `FIND-078 §3` had already measured the ceiling on that lever: at **12000** the racks
    // were *still* flat rectangles, because a directionless fill gives nothing a lit face and a
    // shaded one. No fill is ever the answer to an unreadable room.
    //
    // The two bounds below, both computed from the files and neither one a literal:
    //
    //   brightness       lamp/fill    brick_red on the fill, as % of the lamplit floor
    //          0             inf        0.00 %   <- fails (2): the shaded flank is black
    //       1459           12.09        5.00 %   <- the floor
    //       2400            7.35        8.23 %   <- what stands in art.ron
    //       3528            5.00       12.09 %   <- the ceiling
    //       4200            4.20       14.40 %   <- fails (1): the mitigation this replaced
    //      12000            1.47       41.14 %   <- fails (1), hard
    let mut app = app();
    let exposure = {
        let mut q = app.world_mut().query_filtered::<&Exposure, With<Camera3d>>();
        *q.iter(app.world()).next().expect("the camera carries an Exposure out of art.ron")
    };
    let ambient = {
        let mut q = app.world_mut().query_filtered::<&AmbientLight, With<Camera3d>>();
        q.iter(app.world()).next().expect("the camera carries an AmbientLight").clone()
    };
    let e = exposure.exposure();

    // 1. The fill is the FILE's, not a literal and not Bevy's own default. `AmbientLight` is a
    //    component with `#[require(Camera)]` in 0.19: drop it and the scene silently falls back
    //    to `GlobalAmbientLight` (brightness 80, white) — a term 30x too small and the wrong
    //    colour, with no error and no missing entity to find.
    let data = app.world().resource::<GameData>().clone();
    let file = &data.art.lighting.ambient;
    assert_eq!(
        (ambient.brightness, ambient.color.to_linear().to_f32_array_no_alpha()),
        (file.brightness, [file.color.0, file.color.1, file.color.2]),
        "the camera's fill is not the one in art.ron: lighting.ambient — a literal in Rust is a \
         game number in the wrong file (rule 2)"
    );
    assert_ne!(
        ambient.brightness, 80.0,
        "80 is Bevy's GlobalAmbientLight default — the camera's own AmbientLight has gone missing"
    );

    // 2. ⭐ The premise, made structural. Everything below reads "the room has a light of its
    //    own"; delete the lamps and this test has to say so instead of quietly measuring air.
    let map = data.current_map().expect("current map");
    assert!(
        !map.lights.is_empty(),
        "the start map declares no interior lamp — then the old premise is back (the fill IS the \
         room's only light) and so is the floor of 3830 this test used to carry"
    );

    // The strongest lamp in the map, and the surface it hangs over: the top of the highest solid
    // block whose footprint contains it and that lies below it. Derived, so it moves with the
    // hall's own floor instead of pinning 0.15 m here.
    let lit_by_the_lamp = map
        .lights
        .iter()
        .map(|lamp| {
            let (lx, ly, lz) = lamp.center_m;
            let floor_y = map
                .blocks
                .iter()
                .filter(|b| {
                    b.solid
                        && lx >= b.center_m.0 - b.size_m.0 / 2.0
                        && lx <= b.center_m.0 + b.size_m.0 / 2.0
                        && lz >= b.center_m.2 - b.size_m.2 / 2.0
                        && lz <= b.center_m.2 + b.size_m.2 / 2.0
                        && b.center_m.1 + b.size_m.1 / 2.0 <= ly
                })
                .map(|b| b.center_m.1 + b.size_m.1 / 2.0)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                floor_y.is_finite(),
                "a lamp at ({lx}, {ly}, {lz}) has no floor under it — \
                 tests/world.rs::f019_every_interior_lamp_stands_in_a_room_with_a_roof_over_it_and_a_floor_under_it \
                 is the one that should have caught that first"
            );
            // Bevy's own point-light falloff, straight down onto an up-facing face (NdotL = 1):
            // lumens/4pi candela, `(1 - (d/r)^4)^2 / d^2` attenuation, and that attenuation is a
            // HARD zero at d = r (`bevy_pbr .. pbr_functions.wgsl`, `bevy_pbr/src/render/light.rs`).
            let d = ly - floor_y;
            let intensity = lamp.intensity_lm / (4.0 * core::f32::consts::PI);
            let factor = (d * d) / (lamp.range_m * lamp.range_m);
            let smooth = (1.0 - factor * factor).max(0.0);
            let lux = intensity * smooth * smooth / (d * d).max(1e-4);
            STONE_GRAY_G / core::f32::consts::PI * lux * e
        })
        .fold(0.0f32, f32::max);

    // 3. ⭐ THE CEILING, and it is the one that bites. The lamp has to stay the room's light: on
    //    the very surface it hangs over, it must deliver at least five times what the world's
    //    fill does. A fill that creeps up on the lamp is a fill that is paying for the interior
    //    out of the exterior's shadows, and it buys nothing a lamp has not already bought —
    //    `FIND-080 §4`: going 4200 -> 2400 cost 6.7 sRGB levels on the rack and handed back the
    //    whole 29.1 % contrast outside.
    let fill_on_the_same_face = STONE_GRAY_G * ambient.color.to_linear().green * ambient.brightness * e;
    let dominance = lit_by_the_lamp / fill_on_the_same_face;
    assert!(
        dominance >= 5.0,
        "the lamp delivers {lit_by_the_lamp:.4} on the floor it hangs over and the ambient fill \
         {fill_on_the_same_face:.4} — only {dominance:.2}x. Under 5x the room is being lit by the \
         WORLD and not by its own lamp, and every shadow in the district is paying for it \
         (brightness {}, ev100 {})",
        ambient.brightness,
        exposure.ev100
    );

    // 4. ⭐ THE FLOOR. The fill is the second term, not no term: the outboard flank of a rack, the
    //    corners of the hall and every exterior shadow face away from every lamp (`NdotL <= 0`),
    //    so they live on the fill alone and nothing else in this file bounds them from below in a
    //    ROOM. `brick_red` is the darkest thing the palette puts in here — it is the blade rack's
    //    back panel (`maps.ron: ashgate`) — and it has to stay a visible fraction of the floor it
    //    stands on. Read from the file, because an albedo is a game number too.
    let darkest = data.color("brick_red").expect("brick_red stands in maps.ron: palette")[1];
    let on_the_fill_alone = darkest * ambient.color.to_linear().green * ambient.brightness * e;
    let against_the_floor = on_the_fill_alone / lit_by_the_lamp;
    assert!(
        against_the_floor >= 0.05,
        "the darkest material in the room sits at {:.2} % of the lamplit floor it stands on \
         ({on_the_fill_alone:.4} vs {lit_by_the_lamp:.4}) — a face no lamp reaches is then a hole, \
         and at 0 the fill has stopped existing",
        against_the_floor * 100.0
    );
}

#[test]
fn f071_the_sky_is_a_gradient_and_not_one_colour() {
    // The `ClearColor` the district used to stand against was a single flat value; the user
    // named it in the same breath as the light. The dome carries its gradient in vertex
    // colours, so this reads the mesh the app actually built.
    let mut app = app();
    let k = app.world().resource::<GameData>().art.lighting.sky.clone();

    let handle = {
        let mut q = app.world_mut().query_filtered::<&Mesh3d, With<SkyDome>>();
        q.iter(app.world()).next().expect("render::light::setup_sky spawns exactly one dome").0.clone()
    };
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes.get(&handle).expect("the dome's mesh is loaded");
    let colors = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(c)) => c.clone(),
        other => panic!("the dome has no per-vertex colour ({other:?}) — then it IS one colour"),
    };

    // One colour per ring, `rings + 1` rings. Fewer distinct values than rings means somebody
    // collapsed the gradient.
    let mut distinct: Vec<[u32; 3]> =
        colors.iter().map(|c| [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()]).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(
        distinct.len() as u32 >= k.rings,
        "the sky has {} distinct colours over {} rings — that is a flat sky with extra triangles",
        distinct.len(),
        k.rings
    );

    // The stops land where they are named: first vertex at the zenith, last at the nadir.
    let first = colors.first().expect("the dome has vertices");
    let last = colors.last().unwrap();
    assert!(
        (first[0] - k.zenith.0).abs() < 1e-5 && (last[0] - k.nadir.0).abs() < 1e-5,
        "top vertex {first:?} / bottom vertex {last:?} do not match art.ron's zenith {:?} and \
         nadir {:?} — the gradient is upside down",
        k.zenith,
        k.nadir
    );
    assert!(
        k.zenith.2 > k.zenith.0 && k.horizon.0 > k.zenith.0,
        "the sky has to be darker and bluer at the top than at the horizon, or it reads as a wall"
    );
}

#[test]
fn f071_the_sky_casts_no_shadow_and_the_fog_meets_it_at_the_horizon() {
    // Two failures that are invisible in the code and catastrophic on screen.
    //
    // A 820 m sphere inside the shadow cascade puts the WHOLE district in permanent night, and
    // the symptom ("everything went dark") points at the sun, not at the sky.
    let mut app = app();
    let dome = {
        let mut q = app.world_mut().query_filtered::<Entity, With<SkyDome>>();
        q.iter(app.world()).next().expect("there is a dome")
    };
    assert!(
        app.world().get::<NotShadowCaster>(dome).is_some(),
        "the sky dome casts a shadow — it encloses the district, so the district is in the dark"
    );

    // And the fog colour has to BE the horizon stop. If it is not, the horizon is a visible
    // seam: geometry fades to one colour and the sky behind it is another.
    let k = app.world().resource::<GameData>().art.lighting.clone();
    assert_eq!(
        (k.fog.color.0, k.fog.color.1, k.fog.color.2),
        (k.sky.horizon.0, k.sky.horizon.1, k.sky.horizon.2),
        "fog colour and sky horizon differ — that line is a seam across the whole screen"
    );

    let fog = {
        let mut q = app.world_mut().query_filtered::<&DistanceFog, With<Camera3d>>();
        q.iter(app.world()).next().expect("the camera carries the fog out of art.ron").clone()
    };
    match fog.falloff {
        FogFalloff::Linear { start, end } => {
            assert_eq!((start, end), (k.fog.start_m, k.fog.end_m));
            assert!(
                end < k.sky.radius_m,
                "fog ends at {end} m and the dome stands at {} m — the dome would be inside the \
                 fog and the sky would go back to one colour",
                k.sky.radius_m
            );
        }
        other => panic!("the fog is {other:?}, and only Linear has two numbers you can walk out"),
    }
}

#[test]
fn f071_the_sky_is_wound_so_you_see_it_from_the_inside() {
    // **The failure this catches has no symptom.** The first build wound the dome the other
    // way round, so `cull_mode: Some(Face::Front)` threw away the side facing the eye. There
    // was no warning, no error, no missing entity and no missing mesh — the sky simply stayed
    // the default `ClearColor` (43, 44, 47) pixel for pixel, and the four tests above were all
    // green while nothing was on screen. It cost a build and a render to find.
    //
    // So: take a real triangle off the equator and check which way its face normal points.
    // wgpu's front face is counter-clockwise, so `(v1-v0) x (v2-v0)` has to point AWAY from
    // the centre — then the front face is the outside and culling it leaves the inside.
    let mut app = app();
    let handle = {
        let mut q = app.world_mut().query_filtered::<&Mesh3d, With<SkyDome>>();
        q.iter(app.world()).next().expect("there is a dome").0.clone()
    };
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes.get(&handle).expect("the dome's mesh is loaded");

    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(p)) => p.clone(),
        other => panic!("the dome has no positions ({other:?})"),
    };
    let indices: Vec<u32> = mesh.indices().expect("the dome is indexed").iter().map(|i| i as u32).collect();

    // Every triangle, not just one: a sphere that is right at the equator and wrong at the
    // poles is still a hole in the sky.
    let mut outward = 0;
    for t in indices.chunks_exact(3) {
        let v: Vec<Vec3> = t.iter().map(|&i| Vec3::from(positions[i as usize])).collect();
        let normal = (v[1] - v[0]).cross(v[2] - v[0]);
        // The centroid is the outward direction at that spot — the dome is centred on origin.
        let centroid = (v[0] + v[1] + v[2]) / 3.0;
        if normal.dot(centroid) > 0.0 {
            outward += 1;
        }
    }
    let total = indices.len() / 3;
    // The two polar rings are degenerate (all three vertices of the pole coincide), so their
    // normal is zero and they count neither way. Everything else has to be outward.
    let degenerate = 2 * 32; // segments, at the north and the south pole
    assert!(
        outward >= total - degenerate,
        "{outward} of {total} triangles wind outward — the rest face inward, and wgpu's \
         `Face::Front` cull then throws away the side you stand on. The sky is invisible and \
         nothing reports it"
    );
}

// ---------------------------------------------------------------------------------------
// F-019 — THE LAMPS IN THE HALL. `maps.ron: lights` -> `PointLight`
// ---------------------------------------------------------------------------------------
//
// `docs/FINDINGS.md` FIND-078 measured the room and named the fix: a light INSIDE the
// building, because ambient has no direction and therefore cannot make shape. These two tests
// are the seam between the file and the world — a lamp declared in the map has to be a light
// entity, and a map that declares none has to have none. The second half is not decoration:
// the failure it catches is a lamp that is spawned from somewhere other than the map, which
// would brighten every map in the game and be invisible in the map data.

/// Every [`InteriorLamp`] in the world, with what the renderer will actually use.
fn the_lamps(app: &mut App) -> Vec<(PointLight, Vec3)> {
    let mut q = app.world_mut().query_filtered::<(&PointLight, &Transform), With<InteriorLamp>>();
    q.iter(app.world()).map(|(l, t)| (l.clone(), t.translation)).collect()
}

#[test]
fn f019_a_lamp_in_the_map_is_a_light_in_the_world() {
    let mut app = app_on("ashgate");
    let map = app.world().resource::<GameData>().map("ashgate").expect("ashgate is a map").clone();
    let palette_free = app.world().resource::<GameData>().clone();

    assert!(
        !map.lights.is_empty(),
        "ashgate declares no interior lamps — then the garrison hall is back to the state \
         FIND-078 measured: a back wall at 51.9 against an EXTERIOR wall in shadow at 51.5"
    );

    let lamps = the_lamps(&mut app);
    assert_eq!(
        lamps.len(),
        map.lights.len(),
        "maps.ron declares {} lamps and {} are in the world",
        map.lights.len(),
        lamps.len()
    );

    // Value by value against the file — the number, not "a light is there". An intensity that
    // silently became Bevy's default (1 000 000 lm) is 35x too dark at this exposure and looks
    // in the picture exactly like the unlit room this whole feature exists to end.
    for (i, want) in map.lights.iter().enumerate() {
        let (got, at) = &lamps[i];
        let c = palette_free.color(&want.color).expect("the lamp's colour is in maps.ron: palette");
        assert_eq!(
            *at,
            Vec3::new(want.center_m.0, want.center_m.1, want.center_m.2),
            "lamp {i} hangs somewhere other than where maps.ron puts it"
        );
        assert_eq!(got.intensity, want.intensity_lm, "lamp {i}: intensity is not the file's");
        assert_eq!(got.range, want.range_m, "lamp {i}: range is not the file's — and range is \
             the ONE number that keeps a lamp's effect inside its own building");
        assert_eq!(got.shadow_maps_enabled, want.shadows, "lamp {i}: the expensive switch is \
             not the file's (a shadowed point light is a CUBE map, six passes a frame)");
        let rgb = got.color.to_linear();
        assert!(
            (rgb.red - c[0]).abs() < 1e-5 && (rgb.green - c[1]).abs() < 1e-5 && (rgb.blue - c[2]).abs() < 1e-5,
            "lamp {i} burns {rgb:?} and maps.ron: palette {:?} is {c:?}",
            want.color
        );
    }

    // And no lamp may name one of the three signal colours. That rule is the reason the colour
    // is a palette KEY and not a triple on the light — as a triple it would be a sentence in a
    // document again, and an amber lantern is the exact thing it was written against.
    for (i, lamp) in map.lights.iter().enumerate() {
        assert!(
            !palette_free.maps.signals.contains_key(&lamp.color),
            "lamp {i} burns the signal colour {:?} — cyan, amber and crimson are gameplay and \
             nothing else (docs/conventions.md §3)",
            lamp.color
        );
    }
}

#[test]
fn f019_a_map_without_lamps_has_not_a_single_light_in_it() {
    // The graybox has no interiors — every box in it is solid. If a lamp turns up here, it did
    // not come out of `maps.ron: lights`, and whatever did spawn it is lighting every map in
    // the game from a place no map data mentions.
    let mut app = app_on("graybox");
    let declared = app.world().resource::<GameData>().map("graybox").expect("graybox is a map").lights.len();
    assert_eq!(declared, 0, "this test measures the empty case; graybox now declares {declared} lamps");

    assert!(the_lamps(&mut app).is_empty(), "a map that declares no lamp got one anyway");

    let mut any = app.world_mut().query::<&PointLight>();
    let all: Vec<_> = any.iter(app.world()).collect();
    assert!(
        all.is_empty(),
        "{} point light(s) stand in a map that declares none — the district is being lit from \
         outside its own data, and no file says so",
        all.len()
    );
}

/// The real app on a **named** map instead of whatever `maps.ron: current` happens to say.
///
/// The same pattern as `tests/vector_aiming.rs::app_on`: the override lands on `GameData`
/// before the first `update()`, which is when `Startup` — and with it
/// `render::light::setup_interior_lights` — runs. A test about a map has to name that map, or
/// it silently becomes a test about the day's shipping map.
fn app_on(map: &str) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.world_mut().resource_mut::<GameData>().maps.current = map.to_string();
    assert!(
        app.world().resource::<GameData>().current_map().is_some(),
        "maps.ron lists no map {map:?} — a typo here builds an empty world and every assertion \
         below turns into `nothing is there`"
    );
    app.update();
    app.update();
    app
}
