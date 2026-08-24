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
use avian3d::prelude::LinearVelocity;

use crate::shared::{Block, LocalPlayer, PlayerSettings, SupplyStation};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<model::ModelAssets>()
            .add_systems(
                Startup,
                (
                    light::setup_sun,
                    light::setup_sky,
                    light::setup_interior_lights,
                    model::load_configured_models,
                ),
            )
            .add_observer(model::read_the_models_anchors)
            .add_systems(
                Update,
                (
                    attach_camera,
                    apply_field_of_view,
                    build_block_meshes,
                    // `.chain()`: the material has to exist before the colour is written into
                    // it, and Bevy's sync point between two chained systems is what makes
                    // that true in the same frame.
                    (build_station_meshes, mark_supply_stations).chain(),
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
    settings: Res<PlayerSettings>,
    // 🔴 **`Without<Children>` stood here until 2026-08-19, and it was a mine.** It said "a
    // player who has children already has his camera" — which was true for exactly as long as
    // the camera was the only thing anybody ever hung on a player. `blades::hold::equip_blades`
    // now hangs a pair of blades on him, and whichever of the two lands first would have taken
    // the other's place: a game with a sword and no camera, black screen, exit code 0, no
    // warning. The `existing` guard above already says "there is a camera" and it says it about
    // the whole world, which is the question this system actually asks (§6 rule 3: there is one
    // camera, and it is mine).
    new_players: Query<Entity, With<LocalPlayer>>,
    existing: Query<(), With<Camera3d>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(player) = new_players.iter().next() else {
        return;
    };
    let camera = commands
        .spawn((
            Name::new("camera"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                // `PlayerSettings` and not `k.fov_deg` since 2026-08-13 — and it is the **same
                // number** on a fresh run: `shared::settings` seeds the resource out of
                // `game.ron: camera.fov_deg`. What it buys is that a camera built after the
                // player changed his FOV is built with the FOV he chose, instead of snapping
                // to it one frame later in [`apply_field_of_view`].
                fov: settings.fov_deg.to_radians(),
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
/// The field of view the player set — **`render` stays the one writer of `Projection`.**
///
/// `menu` owns the settings screen and writes `shared::PlayerSettings`; it does not touch the
/// camera. That keeps the authority table true (`docs/architecture.md`: the camera is
/// `render`'s) and it is the seam `F-017` will need — speed-driven FOV interpolates between
/// this base and `game.ron: camera.fov_max_speed_deg`, and it has to interpolate from the
/// player's number, not from the file's.
///
/// Runs on a changed resource and compares before it writes: `Projection` is read by the render
/// world every frame, and re-marking it as changed for a value that did not move is exactly what
/// §6 rule 6 is about.
fn apply_field_of_view(
    data: Res<GameData>,
    settings: Res<PlayerSettings>,
    players: Query<&LinearVelocity, With<LocalPlayer>>,
    mut cameras: Query<&mut Projection, With<Camera3d>>,
) {
    // **`F-017` is why the early-out is gone.** Until 2026-08-24 this system returned unless
    // `PlayerSettings` had changed, which was right when the field of view was a preference and
    // nothing else. It is now a function of the player's speed as well, and speed changes on
    // every tick without anybody touching a setting — an early-out on `is_changed()` would have
    // meant the curve fires once, on the frame the slider moves, and never again. The write is
    // still guarded, one line down, by comparing the value: that is the guard that matters
    // (§6 rule 6), and it is the one that was doing the work all along.
    let speed_m_s = players.iter().next().map_or(0.0, |v| v.0.length());
    let want = speed_fov_deg(
        settings.fov_deg,
        data.game.camera.fov_max_speed_deg,
        data.game.camera.fov_speed_from_m_s,
        data.game.vector.max_speed_m_s,
        speed_m_s,
        settings.speed_fov_pct,
    )
    .to_radians();
    for mut projection in &mut cameras {
        // Read through `&*` first: a `DerefMut` on a `Mut<Projection>` marks it changed even
        // when nothing is written, and the change would travel into the render world.
        let Projection::Perspective(current) = &*projection else {
            continue;
        };
        if (current.fov - want).abs() <= 1e-6 {
            continue;
        }
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.fov = want;
        }
    }
}

/// `F-017` **Geschwindigkeits-Feedback** — the field of view as a function of speed, and it is
/// a free function so it can be checked without an app, a camera or a window.
///
/// > *„Speedlines, Kamera-FOV-Kurve, Windrauschen und Vignette skalieren mit Velocity. Verkauft
/// > Tempo ohne echte Physikaenderung."* — the backlog row. **This is the FOV half**, which is
/// the one the design calls the *biggest lever* (`game.ron: camera`) and the only one of the
/// four that needs no new asset.
///
/// ```text
///   fov
///    |                              ______ fov_max_speed_deg (90)
///    |                        _____/
///    |  ______________ ______/
///    |                 base = settings.fov_deg (60, and the player's, not the file's)
///    +--------------|--------------|------> |v|
///                  from           max_speed
///                (22 m/s)         (75 m/s)
/// ```
///
/// Four things that are decisions and not arithmetic:
///
/// - **It interpolates from the PLAYER's base, not the file's.** `settings.fov_deg` is what he
///   set in the options; a curve anchored at `game.ron: camera.fov_deg` would silently undo his
///   slider the moment he moved.
/// - **`pct` scales the WIDENING, not the result.** At 0 % the return value is `base` exactly —
///   the same `f32`, not a value that rounds to it — which is what makes
///   *„abschaltbar fuer Motion Sickness"* a real off switch and not a quieter version of the
///   effect. Multiplying the whole result would move the resting field of view instead.
/// - **It is stepless.** *„FOV und Audio reagieren stufenlos"* is the acceptance sentence, so
///   this is `lerp` and never a set of thresholds.
/// - **Above `max_speed_m_s` it saturates.** That is not a guard against a bug, it is the
///   physics: `vector.max_speed_m_s` is an avian `MaxLinearSpeed` on the body, so `|v|` cannot
///   exceed it — the `clamp` is what keeps the function honest if it ever does.
///
/// Degenerate spans (`from >= max`, a `pct` outside 0..100) collapse to the base rather than to
/// a NaN: a NaN `fov` is a black screen, and a black screen is the one failure a player cannot
/// report usefully.
pub fn speed_fov_deg(
    base_deg: f32,
    max_speed_deg: f32,
    from_m_s: f32,
    max_speed_m_s: f32,
    speed_m_s: f32,
    pct: f32,
) -> f32 {
    let span = max_speed_m_s - from_m_s;
    if !(span > 0.0) || !speed_m_s.is_finite() {
        return base_deg;
    }
    let t = ((speed_m_s - from_m_s) / span).clamp(0.0, 1.0);
    // ⚠️ **`clamp` does not clean a NaN** — `f32::clamp` returns the NaN it was given
    // (`core`: "if self is NaN, returns NaN"), so a non-finite `speed_fov_pct` came straight
    // through here and made the projection's `fov` a NaN, which is a black screen. Found by
    // `tests/render.rs::f017_a_degenerate_span_or_a_nan_speed_falls_back_to_the_base` on the
    // first run; the `speed` guard above was already written this way and the `pct` one was not.
    let strength = if pct.is_finite() { (pct / 100.0).clamp(0.0, 1.0) } else { 0.0 };
    base_deg + (max_speed_deg - base_deg) * t * strength
}

/// `F-019` — **the empty station is marked, and the running one is too.**
///
/// The acceptance sentence asks only for *„leere Station wird visuell markiert"*; the middle
/// state is here because without it a player cannot tell a station that is pumping *for him*
/// from one that is simply there, and the 1.5 s he is being asked to stand still for is exactly
/// the time in which he needs to know.
///
/// | state | colour | what it says |
/// |---|---|---|
/// | reloads left, idle | **cyan** | come here |
/// | pumping | **amber** | it is running, stay |
/// | empty | `ash_dark` | this one is spent, fly on |
///
/// Cyan and amber are two of the three signal colours, and `docs/conventions.md` §3 reserves
/// them for gameplay and forbids them everywhere else — so a cyan pole in a grey street is by
/// construction a thing that does something for you. Empty leaves the signal set entirely and
/// drops into the palette, which is the point: it is no longer gameplay.
///
/// **It compares before it writes.** `Assets::get_mut` marks the material changed and the change
/// travels into the render world; a station pumps for 90 ticks and its `SupplyStation` reports
/// itself changed on every one of them (`charge_s` really does move), so writing unguarded
/// would push a material through the pipeline ninety times to say the same amber.
/// `F-019` — **draws the station**, because nothing else does any more.
///
/// It got its mesh from [`build_block_meshes`] via a `Block` component for about a minute, and
/// two guard tests said no in one run: `Block` means *a cuboid of the city*, `tests/world.rs`
/// counts them against `world::map::plan_blocks`, and four stations made the count 2875 against
/// 2871 (`src/world/supply.rs` carries the whole note). So the station carries its own marker
/// and this is its own builder — one more system, and the city's guard stays a guard.
///
/// The pole is deliberately **small and tall** (1.5 x 4 x 1.5 m): visible over a 7 m street,
/// standing inside a 6 m trigger sphere, and far too thin to be the thing you land on.
fn build_station_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    fresh: Query<Entity, (With<SupplyStation>, Without<Mesh3d>)>,
) {
    for e in &fresh {
        let mesh = meshes.add(Cuboid::new(1.5, 4.0, 1.5));
        // The colour is left to `mark_supply_stations`, which runs in the same schedule and
        // knows the three states. Seeding it here as well would be two writers of one field for
        // the sake of one frame.
        let material = materials.add(StandardMaterial {
            metallic: 0.0,
            perceptual_roughness: 0.95,
            ..default()
        });
        commands.entity(e).insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

fn mark_supply_stations(
    data: Res<GameData>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stations: Query<(&SupplyStation, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (station, handle) in &stations {
        let key = if station.empty() {
            None
        } else if station.running() {
            Some("amber")
        } else {
            Some("cyan")
        };
        let want = match key {
            Some(signal) => data.maps.signals.get(signal).map(|(r, g, b)| [*r, *g, *b]),
            None => data.maps.palette.get("ash_dark").map(|(r, g, b)| [*r, *g, *b]),
        };
        let Some([r, g, b]) = want else {
            continue;
        };
        let want = Color::linear_rgb(r, g, b);
        // Read first. A `get_mut` that writes the colour it already holds is a change signal
        // for nothing, sixty times a second, for every station on the map.
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        if material.base_color == want {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.base_color = want;
        }
    }
}

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
