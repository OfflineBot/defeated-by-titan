//! render::light — the sun, the sky dome, the fog and the exposure.
//!
//! # Why this file exists, and it is one measurement and not an opinion
//!
//! The user, twice, once in the very first window session and again on 2026-08-12:
//!
//! > *„aktuell sieht man nicht so viel unterschiede. alles sehr flat (auch farben, licht etc)"*
//!
//! It was written down the first time and nothing was done. This is the second time, and the
//! cause is arithmetic, not taste — measured out of `docs/images/f003-light-before.png`
//! (`docs/FINDINGS.md` FIND-071):
//!
//! | patch | mean RGB | luminance |
//! |---|---|---|
//! | a vertical wall face | 182.6 / 183.8 / 179.5 | **183.2** |
//! | the ground beside it | 182.7 / 183.8 / 179.6 | **183.3** |
//!
//! Two surfaces at right angles to each other, one number apart. The reason: the old light was
//! `illuminance: 10_000` against Bevy's default exposure of `ev100 = 9.7`, and
//! `0.42/pi * 10000 * exposure(9.7) = 1.10` — **every face with `NdotL > 0.73` clipped to
//! white**, and a clipped face has no colour and no orientation left. Mean saturation over
//! those patches was 0.023.
//!
//! So the fix is not "add lights". It is: put the sunlit side back under the clip (exposure and
//! illuminance become one solved pair), give the fill a colour so the unlit side is *cool* and
//! not merely dark, put the boxes' own shadows back on the ground, and replace the single-colour
//! `ClearColor` with a dome and a fog that agree on their horizon.
//!
//! # The four things it does
//!
//! 1. **[`setup_sun`]** — one `DirectionalLight`, aimed from an azimuth/elevation pair rather
//!    than from a point in metres, with cascaded shadows sized for a 400 m district.
//! 2. **[`setup_sky`]** — a dome with a three-stop vertical gradient in its **vertex colours**,
//!    `unlit`, `fog_enabled: false`, front faces culled (we are inside it), and — this one is
//!    load-bearing — [`NotShadowCaster`]: a 820 m sphere that casts a shadow puts the whole
//!    district in the dark.
//! 3. **[`follow_the_eye`]** — the dome is pinned to the camera's *translation only*. Rotation
//!    must not be inherited or the gradient turns with the head, which is why it is a system and
//!    not a child entity.
//! 4. **[`camera_light_settings`]** — `AmbientLight`, `DistanceFog` and `Exposure` all hang on
//!    the **camera** in Bevy 0.19 (`docs/lessons/bevy.md`), so they are attached where the
//!    camera is born, in [`super::attach_camera`].
//!
//! **Every number is in `assets/data/art.ron: lighting`** (rule 2) — including `shadows: bool`,
//! and that is deliberate: `docs/lessons/performance.md` rule 5 says shadows are the most
//! expensive switch in the game and demands a number for them. A `bool` in the file is how the
//! number gets measured with two runs of the *same binary* and the *same scene*.

use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::{
    CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap, NotShadowCaster,
    NotShadowReceiver,
};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::camera::visibility::NoFrustumCulling;

use crate::data::{GameData, Lighting};

/// Marks the sky dome, so [`follow_the_eye`] can find it without a name lookup.
#[derive(Component)]
pub struct SkyDome;

/// A linear-RGB triple out of the RON as a Bevy [`Color`].
///
/// **`linear_rgb`, never `srgb`.** The palette in `maps.ron` is documented as linear and
/// `world::map` reads it that way; a sky that used the sRGB constructor would be a different
/// colour from the walls it sits behind for no visible reason.
fn linear(c: (f32, f32, f32)) -> Color {
    Color::linear_rgb(c.0, c.1, c.2)
}

/// Where the sun is, as a unit vector **pointing at it** from the world origin.
///
/// The convention is the game's own and not Bevy's: yaw 0 looks along -Z, +90 along +X
/// (`docs/conventions.md`, and `scripts/f003-ashgate.txt` warps by it). Elevation is degrees
/// above the horizon.
///
/// Kept as a free function because it is the one piece of this file that is arithmetic and can
/// therefore be checked without a GPU — `tests/render.rs::f071_the_sun_stands_where_art_ron_says`.
pub fn to_sun(azimuth_deg: f32, elevation_deg: f32) -> Vec3 {
    let (az, el) = (azimuth_deg.to_radians(), elevation_deg.to_radians());
    Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos())
}

/// The sun. One directional light, and the cascade split that decides how far its shadows reach.
pub fn setup_sun(mut commands: Commands, data: Res<GameData>) {
    let k = &data.art.lighting;
    let sun = &k.sun;

    // The shadow map size is a Resource in Bevy, not a field on the light. Inserted (not
    // mutated) so the value is the file's even in an app that never had the default.
    commands.insert_resource(DirectionalLightShadowMap { size: sun.shadow_map_size });

    let cascades: CascadeShadowConfig = CascadeShadowConfigBuilder {
        num_cascades: sun.cascades,
        minimum_distance: 0.1,
        maximum_distance: sun.shadow_distance_m,
        first_cascade_far_bound: sun.first_cascade_far_bound_m,
        overlap_proportion: sun.cascade_overlap,
    }
    .into();

    // 500 m out along the sun vector. A directional light has no position — only the rotation
    // of its transform is read — but `looking_at` needs somewhere to look from, and a point
    // that far out keeps the gizmo (`DBT_GIZMOS=1`) out of the district.
    let eye = to_sun(sun.azimuth_deg, sun.elevation_deg) * 500.0;

    commands.spawn((
        Name::new("sun"),
        DirectionalLight {
            color: linear(sun.color),
            illuminance: sun.illuminance_lux,
            // `contact_shadows_enabled` is deliberately NOT set: on its own it does nothing at
            // all — contact shadows additionally need a `ContactShadows` component on the
            // camera. A field you take for a switch when it is none is worse than no field.
            shadow_maps_enabled: sun.shadows,
            shadow_depth_bias: sun.shadow_depth_bias,
            shadow_normal_bias: sun.shadow_normal_bias,
            ..default()
        },
        cascades,
        Transform::from_translation(eye).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// The sky dome.
///
/// A `ClearColor` is one colour and reads as a wall; this is a sphere seen from the inside whose
/// **vertex colours** run zenith -> horizon -> nadir. Three reasons it is geometry and not a
/// shader: it costs one draw call of 512 quads, it needs no material plugin of our own, and its
/// gradient is the same three numbers the fog is built out of, so the two cannot drift apart.
pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    data: Res<GameData>,
) {
    let sky = &data.art.lighting.sky;
    let mesh = meshes.add(dome_mesh(sky));

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        // The gradient is the whole point — no light may touch it.
        unlit: true,
        // Without this the dome is fogged to the fog colour and we are back to one colour.
        fog_enabled: false,
        // We stand inside the sphere, so it is the FRONT faces that have to go. Cheaper than
        // `cull_mode: None`, which would draw every triangle twice.
        cull_mode: Some(Face::Front),
        ..default()
    });

    commands.spawn((
        Name::new("sky"),
        SkyDome,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        // ⚠️ Load-bearing. A 820 m sphere in the shadow cascade puts the entire district in
        // permanent night, and the symptom ("everything is dark") looks nothing like the cause.
        NotShadowCaster,
        NotShadowReceiver,
        // ⚠️ Also load-bearing, and it cost a run to find: the dome ENCLOSES the camera, so
        // its `Aabb` contains the near plane and Bevy's `check_visibility` throws it out —
        // the first build rendered a perfect gradient nobody could see and the sky stayed the
        // default `ClearColor` (43,44,47) pixel for pixel. A thing that is always around you
        // is exactly what this component is for.
        NoFrustumCulling,
        Transform::default(),
    ));
}

/// The dome geometry: a UV sphere with one colour per vertex.
///
/// Only positions, indices and colours — the material is `unlit`, so normals and UVs would be
/// bytes nobody reads. Wound so that the outside is the front face; [`setup_sky`] culls that
/// side away.
fn dome_mesh(sky: &crate::data::Sky) -> Mesh {
    let (segments, rings) = (sky.segments.max(3), sky.rings.max(2));
    let r = sky.radius_m;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();

    for ring in 0..=rings {
        // theta from 0 (north pole) to pi (south pole).
        let theta = core::f32::consts::PI * ring as f32 / rings as f32;
        let (st, ct) = theta.sin_cos();
        // ct = +1 at the zenith, 0 at the horizon, -1 at the nadir — which is exactly the
        // parameter the three stops are keyed on.
        let c = if ct >= 0.0 {
            mix(sky.horizon, sky.zenith, ct)
        } else {
            mix(sky.horizon, sky.nadir, -ct)
        };
        for seg in 0..=segments {
            let phi = core::f32::consts::TAU * seg as f32 / segments as f32;
            let (sp, cp) = phi.sin_cos();
            positions.push([r * st * cp, r * ct, r * st * sp]);
            colors.push([c.0, c.1, c.2, 1.0]);
        }
    }

    let stride = segments + 1;
    let mut indices: Vec<u32> = Vec::new();
    for ring in 0..rings {
        for seg in 0..segments {
            let a = ring * stride + seg;
            let b = a + stride;
            // ⚠️ The order is the whole visibility of the sky. `(a, a+1, b)` winds the
            // **outside** counter-clockwise, which is wgpu's front face — so `Face::Front`
            // culling in [`setup_sky`] throws the outside away and leaves the inside, which is
            // the only side we ever stand on. The first build had `(a, b, a+1)`, which winds it
            // the other way: the dome was then culled *inside out*, rendered a perfect gradient
            // that faced away from the eye, and the sky stayed the default `ClearColor`
            // (43, 44, 47) pixel for pixel — with no warning, no error and no missing entity.
            // `tests/render.rs::f071_the_sky_is_wound_so_you_see_it_from_the_inside`.
            indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
}

/// Linear interpolation between two RON colour triples. `t = 0` is `a`, `t = 1` is `b`.
fn mix(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// Pins the dome to the eye — **translation only**.
///
/// A child entity would inherit the camera's rotation and the gradient would turn with the head:
/// look up and the horizon would follow you. One entity moved per frame is not the kind of
/// per-frame work rule 6 is about; it is a single `Vec3` write.
///
/// It follows the camera and not the player because the camera is what the frustum belongs to,
/// and the dome's only job is to stay inside the far plane.
pub fn follow_the_eye(
    camera: Query<&GlobalTransform, (With<Camera3d>, Without<SkyDome>)>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
) {
    let Some(eye) = camera.iter().next() else {
        return;
    };
    let eye = eye.translation();
    for mut t in &mut dome {
        if t.translation != eye {
            t.translation = eye;
        }
    }
}

/// The three components that hang on the **camera** and not on the world.
///
/// Called by [`super::attach_camera`] at the moment the camera is spawned — `AmbientLight`
/// carries `#[require(Camera)]` in Bevy 0.19 and `DistanceFog` and `Exposure` are per-view by
/// nature. Returned as a tuple rather than inserted here so that there is exactly one place in
/// the code that spawns the camera.
pub fn camera_light_settings(k: &Lighting) -> (AmbientLight, DistanceFog, Exposure) {
    (
        AmbientLight { color: linear(k.ambient.color), brightness: k.ambient.brightness, ..default() },
        DistanceFog {
            color: linear(k.fog.color),
            // No sun glow: it is a lens effect, and this scene has no lens. `Color::NONE`
            // switches the whole term off (`bevy_pbr-0.19.0/src/fog.rs`).
            directional_light_color: Color::NONE,
            directional_light_exponent: 1.0,
            falloff: FogFalloff::Linear { start: k.fog.start_m, end: k.fog.end_m },
        },
        Exposure { ev100: k.exposure_ev100 },
    )
}
