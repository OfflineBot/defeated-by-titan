//! render — camera, light, building meshes.
//!
//! **Reads only.** Rendering is presentation, not simulation — a system that spawns a mesh
//! straight out of a mouse click is the beginning of the end, because that very click has to
//! be confirmed by the server later (`prompts/init.md` §6 rule 1).
//!
//! The style is fixed: low poly, soft normals, flat color surfaces, aggressive distance fog
//! (it does double duty: atmosphere and culling). The three signal colors are for gameplay
//! and nothing else (`docs/conventions.md`).
//!
//! ⚠️ None of this has ever been **seen** — machine A has no window
//! (`docs/environment.md`). Everything here stays 🟨 until somebody on machine B looks at it.

//! **Where the seam stands:** [`camera::rotate_camera`] has been filled since 2026-08-09 —
//! image and aim ray point the same way, nailed down in `tests/render.rs` and seen in
//! `docs/images/f002-look.png` / `docs/images/f002-look-turned.png`.
//! [`rope::draw_ropes`] has been filled since 2026-08-10 and is registered below — a cyan line
//! per **anchored** arm, and nothing at all for a released one. Before that day no pixel
//! anywhere in this build told a player a rope was attached. Seen in
//! `docs/images/f004-rope.png` against `docs/images/f004-rope-released.png`, which is the same
//! scene 30 ticks later with the arm let go.

//! **The look, since 2026-08-12:** [`light`] holds the sun, the sky dome, the fog and the
//! exposure, and every number of it is in `assets/data/art.ron: lighting`. It exists because
//! the same complaint came in twice — *„alles sehr flat (auch farben, licht etc)"* — and the
//! second time it was measured: a wall face and the ground beside it read luminance 183.2 and
//! 183.3, because the old sun clipped every face with `NdotL > 0.73` to white
//! (`docs/FINDINGS.md` FIND-071).

pub mod camera;
pub mod light;
pub mod model;
pub mod rope;

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Block, LocalPlayer};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<model::ModelAssets>()
            .add_systems(
                Startup,
                (light::setup_sun, light::setup_sky, model::load_configured_models),
            )
            .add_observer(model::read_the_models_anchors)
            .add_systems(
                Update,
                (
                    attach_camera,
                    build_block_meshes,
                    camera::rotate_camera,
                    light::follow_the_eye,
                    log_frame_time.run_if(|| std::env::var_os("DBT_FRAMETIME").is_some()),
                    rope::draw_ropes,
                    // The model chain, in its own order: a file becomes handles, an entity
                    // becomes a name, a name becomes a body. `chain()` and not three separate
                    // registrations, because a titan that appears and gets its scene one frame
                    // later is exactly the case `attach_late_scenes` exists for — and a random
                    // order would make that "sometimes".
                    (
                        model::resolve_animation_clips,
                        model::name_the_titans_model,
                        model::spawn_models,
                        model::attach_late_scenes,
                        // …and the two that finish the seam: the game state picks the clip,
                        // and the cuboid rig gets out of the way of the model that replaced
                        // it — or comes back when that model cannot show the state.
                        model::drive_animations,
                        model::hide_the_primitive_under_a_model,
                    )
                        .chain(),
                ),
            );
    }
}

/// `DBT_FRAMETIME=1` — **the real frame time, and why wall clock cannot give it.**
///
/// `docs/lessons/performance.md` lists "no number for what shadows cost" as a gap and rule 5
/// forbids switching them on without one. Timing a run does not answer it: a `--script` run
/// lasts as long as its `wait` lines say, because `Time<Virtual>` tracks the wall clock — a
/// 21.25 s script takes 21.25 s whether the renderer needs 2 ms or 12 ms a frame. Measured:
/// 1275 ticks = 22.18 s wall, with and without shadows, to the second.
///
/// What does answer it is **how many frames fitted into those seconds**. Nothing paces the
/// `Update` loop in an `--offscreen` run (no window, no vsync), so `frames / real seconds` is
/// the true frame time — and that is exactly Bevy's own vsync warning in
/// `docs/lessons/performance.md`: only measured without the ceiling does the number say
/// anything.
///
/// Off unless the variable is set, like `DBT_GIZMOS` (`debug::gizmo`) — a diagnostic that runs
/// when nobody asked for it is a cost, not a measurement.
fn log_frame_time(
    time: Res<Time<Real>>,
    mut frames: Local<u32>,
    mut since_s: Local<f32>,
    mut window: Local<u32>,
) {
    *frames += 1;
    *since_s += time.delta_secs();
    // Every 2 real seconds, so a 20 s run gives ten samples and the first one (pipeline
    // compilation, shader specialization, the first shadow map) can be thrown away.
    if *since_s < 2.0 {
        return;
    }
    *window += 1;
    info!(
        "FRAMETIME window {} — {} frames in {:.3} s = {:.3} ms/frame ({:.0} fps)",
        *window,
        *frames,
        *since_s,
        *since_s * 1000.0 / *frames as f32,
        *frames as f32 / *since_s
    );
    *frames = 0;
    *since_s = 0.0;
}

/// Hangs the camera on the local player.
///
/// **This is the only place where "I" get a camera.** Every other player is one of many and
/// has none (§6 rule 3).
///
/// In Bevy 0.19 `AmbientLight` hangs on the **camera**, not on the world — it is a component
/// with `#[require(Camera)]` and no longer a `Resource` (`docs/lessons/bevy.md`).
///
/// # Why [`IsDefaultUiCamera`] hangs here — the whole evidence route depends on it
///
/// This is one component, and without it **every HUD screenshot in this project comes out
/// empty while the run exits 0** (`docs/PLAN-GAME.md` §11, risk 3). Read from the installed
/// source, not from memory:
///
/// `DefaultUiCamera::get()` (`bevy_ui-0.19.0/src/ui_node.rs:2990-3009`) asks two questions.
/// First `default_cameras.single()` — the query over `With<IsDefaultUiCamera>`, `:2991`.
/// Only if that finds nothing does it fall back to filtering by render target, `:2997-3003`:
///
/// ```text
/// RenderTarget::Window(WindowRef::Primary)   => true      // ← unconditional
/// RenderTarget::Window(WindowRef::Entity(w)) => w is the primary window
/// _                                         => false     // ← Image lands here
/// ```
///
/// Under `--offscreen` the camera's target is an `Image`
/// (`debug::screenshot::attach_offscreen_target`) — so the fallback answers `None`, every UI
/// root keeps `ComputedUiTargetCamera::default()` = `Entity::PLACEHOLDER` and
/// `ComputedUiRenderTargetInfo::physical_size` = `UVec2::ZERO` (`:3019-3051`). The UI lays
/// out into a zero-sized viewport: no crash, no warning, just no pixels.
///
/// **And it is invisible in a window run**, because there the first arm of the fallback fires
/// — which is exactly why this is worth a paragraph instead of a word.
/// `tests/render.rs::the_camera_is_the_default_ui_camera` swaps the target the way
/// `--offscreen` does and falls over when the component goes.
///
/// **Measured `[debian]`, not reasoned** (`scripts/p1-overlay.txt`, 1280x720, tick 140):
///
/// | run | UI camera | UI root size | PNG |
/// |---|---|---|---|
/// | with the component | `Some(camera)` | 1280x720 | `docs/images/p1-overlay.png`, 625 728 B |
/// | component taken out | `None` | **0x0** | 617 554 B, **bit-identical to the run in which the overlay was never switched on** (`docs/images/p1-no-overlay.png`) |
///
/// Both runs exited 0 and both wrote a perfectly good picture of the city. That is the whole
/// point: the failure has no symptom except a missing overlay.
fn attach_camera(
    mut commands: Commands,
    data: Res<GameData>,
    new_players: Query<Entity, (With<LocalPlayer>, Without<Children>)>,
    existing: Query<(), With<Camera3d>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(player) = new_players.iter().next() else {
        return;
    };
    let k = &data.game.camera;
    let camera = commands
        .spawn((
            Name::new("camera"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: k.fov_deg.to_radians(),
                ..default()
            }),
            // Ambient fill, distance fog and exposure — all three are per-view in Bevy 0.19,
            // all three come out of `art.ron: lighting`, and they are only meaningful against
            // each other (`render::light`).
            light::camera_light_settings(&data.art.lighting),
            // The one component that keeps the UI in an `--offscreen` image — see above.
            IsDefaultUiCamera,
            // Eye height above the player's origin — which lies between the feet
            // (docs/conventions.md).
            Transform::from_xyz(0.0, data.game.player.eye_height_m, 0.0),
        ))
        .id();
    commands.entity(player).add_child(camera);
}

/// Turns [`Block`] data into triangles — **once per entity**.
///
/// `render` does not know `world` for that: it asks for a component, not for a function
/// (`docs/architecture.md`).
fn build_block_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    without_mesh: Query<(Entity, &Block), Without<Mesh3d>>,
) {
    for (e, block) in &without_mesh {
        let mesh = meshes.add(Cuboid::new(block.size.x, block.size.y, block.size.z));
        let material = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(block.color[0], block.color[1], block.color[2]),
            // A missing metallicFactor means 1.0, i.e. fully metallic — a diffuse material
            // without the value looks like chrome in the game (docs/models.md, glTF trap 2).
            // The same holds here.
            metallic: 0.0,
            perceptual_roughness: 0.95,
            ..default()
        });
        commands.entity(e).insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}
