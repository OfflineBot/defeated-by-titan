//! gizmo — the lines that turn an image into **evidence**.
//!
//! `docs/ACCEPTANCE.md` says "No image, no 🟧 — no exceptions". An image on its own is not
//! enough for that though: `docs/images/t006-world-far.png` shows ground and blocks and
//! nothing else — **which block is anchorable is not in the image.** Whoever has to carry
//! an image line for `F-002`, `F-003` or `F-004` next cannot carry it without this file.
//! The counterpart with the gizmos switched on is `docs/images/f003-anchors.png`; the
//! difference between the two **is** the evidence.
//!
//! Three things are drawn, and each of them answers a question an image otherwise leaves
//! open:
//!
//! | drawn | color | answers |
//! |---|---|---|
//! | outline of every [`AnchorSurface`] | **cyan** | what is anchorable? |
//! | axis cross at the origin, ground grid | neutral | how big, how far? |
//! | hull and mast of every player | white | who stands where? |
//!
//! ## The colors are not free
//!
//! `docs/conventions.md` §3 reserves **cyan** for "gas, Vector Gear, **anchor points**",
//! **amber** for targets and weak points, **crimson** for danger. For an anchor surface
//! cyan is therefore **prescribed**, not chosen. Everything else here is none of the three
//! and stays **neutral** for that reason.
//!
//! That affects the axis cross above all: the usual X=red / Y=green / Z=blue assignment
//! would be a violation, because red belongs to danger. So **X is magenta** instead of red
//! (full blue channel, hence far away from crimson), **Y is light gray** instead of green,
//! and **Z stays blue** — blue is not cyan as long as the green channel is low. The player
//! is white; the axis cross stands at the origin and nowhere else, so the two cannot be
//! mistaken for one another.
//!
//! ## The toggle — and why it is an environment variable
//!
//! Gizmos must not run all the time: they cost compute, and on a gameplay image they get
//! in the way. The obvious toggle would be a launch flag in the style of the nine others
//! ([`Cli`](crate::shared::Cli)) — **but `src/shared/cli.rs` does not belong to this
//! commission**, and a silent fix in someone else's file is an invisible merge conflict
//! with the agent working right next to you. A `--gizmos` fished out of
//! `std::env::args()` by hand is out too: [`Cli::from_args`](crate::shared::Cli::from_args)
//! files every unknown argument in `unknown`, and the launch would then loudly report a
//! flag as unknown that did in fact take effect.
//!
//! That leaves the environment variable [`ENV_VAR`]. It gets by without foreign files and
//! hangs onto the same screenshot command:
//!
//! ```text
//! env DBT_GIZMOS=1 cargo run --features wayland,audio -- --offscreen \
//!     --script scripts/t006-shot-far.txt --ticks 110 --screenshot docs/images/f003-anchors.png
//! ```
//!
//! With a window **F4** toggles it (F3 is the overlay). The right place is still a
//! `--gizmos` flag in `src/shared/cli.rs` — that is recorded as a finding in the report.
//!
//! ## Why the numbers live here and not in a RON file
//!
//! Rule 2 ("numbers belong in RON") means game values: titan kinds, blade tiers, gas
//! costs. A line width in pixels and a grid pitch are the edge lengths of a **test tool**,
//! exactly like [`OFFSCREEN_WIDTH`](super::screenshot::OFFSCREEN_WIDTH) in
//! `src/debug/screenshot.rs`. What does come from the RON here is everything that
//! describes the player: his height and his radius are in `game.ron` and are not rebuilt.

use bevy::gizmos::gizmos::GizmoBuffer;
use bevy::prelude::*;
use core::f32::consts::FRAC_PI_2;

use crate::data::GameData;
use crate::shared::{player_aabb, AnchorSurface, Block, Body, PlayerId};

// ---------------------------------------------------------------------------
// Colors — linear RGB, the same form as `Block::color`
// ---------------------------------------------------------------------------

/// **Cyan.** `docs/conventions.md` §3: "cyan — gas, Vector Gear, anchor points." An anchor
/// surface is exactly that; here the color is regulation, not taste.
const CYAN: [f32; 3] = [0.0, 0.85, 1.0];

/// +X. Magenta instead of the usual red: crimson belongs to danger.
const AXIS_X: [f32; 3] = [1.0, 0.05, 0.75];
/// +Y. Light gray instead of the usual green — neutral, and "up" is unambiguous anyway.
const AXIS_Y: [f32; 3] = [0.85, 0.85, 0.85];
/// +Z. Blue may stay: it is none of the three signal colors as long as the green channel
/// is low enough that it does not pass for cyan.
const AXIS_Z: [f32; 3] = [0.10, 0.25, 1.0];

/// The ground grid. Almost black, because the ground is bright — and neutral, because a
/// grid makes no statement about gameplay.
const GRID: [f32; 3] = [0.02, 0.02, 0.03];

/// The player. White, neutral.
const PLAYER: [f32; 3] = [1.0, 1.0, 1.0];

/// How much the negative half of an axis is dimmed against the positive one. That way you
/// see at a glance where +X ends and −X begins, without spending a second color on it.
const DIM_FACTOR: f32 = 0.35;

// ---------------------------------------------------------------------------
// Dimensions of the test tool
// ---------------------------------------------------------------------------

/// Edge length of one grid cell. Ten meters is the number a human estimates distances in,
/// and a multiple of the player height (1.8 m) would not be.
const GRID_CELL_M: f32 = 10.0;
/// Cells per axis. 20 x 10 m = 200 m of edge length, centered on the origin — half the
/// graybox (`maps.ron: size_m = 400 x 400`).
const GRID_CELLS: u32 = 20;
/// How far the grid floats above the top of the ground. The ground slab ends at y = 0
/// (`maps.ron: blocks[0]`); without that gap grid and slab flicker against each other.
/// Ten centimeters are a tenth of a pixel at 45 m and change no reading.
const GRID_HEIGHT_M: f32 = 0.1;

/// Arm length of the axis cross — **exactly one grid cell**. That makes the cross the
/// scale for the grid as well, instead of a second, competing length.
const AXIS_M: f32 = GRID_CELL_M;

/// How far the mast sticks out above a player's head. Without it a 1.8 m capsule at 80 m
/// is a few pixels tall and cannot be found in the image.
const MAST_M: f32 = 3.0;

/// Line width of the statements (anchors, players, axes) in pixels. Bevy's default is
/// 2.0 — at 1280x720 an outline is there with it, but not *clear*.
const LINE_PX: f32 = 3.0;
/// Line width of the grid. Thinner than the statements: the grid is background and must
/// not shout the outlines down.
const GRID_PX: f32 = 1.0;

/// How far gizmos are shifted toward the camera (−1 to 1, 0 = not at all,
/// `bevy_gizmos-0.19.0/src/config.rs:215-227`).
///
/// An outline lies **exactly** on the surface it outlines — without a bias every edge
/// flickers against its own wall.
///
/// **The value is measured, not guessed.** The scale is the depth buffer's and therefore
/// strongly nonlinear: `−0.05` sounded small and was not — at 45 to 150 m it drew every
/// outline *through* its own house, all twelve edges visible, and outlines further back
/// lay over the houses in front of them (comparison image from the same run). `−0.001`
/// keeps the occlusion intact and still does not flicker:
/// `docs/images/f003-anchors.png` was taken with it.
const DEPTH_BIAS: f32 = -0.001;

// ---------------------------------------------------------------------------
// The toggle
// ---------------------------------------------------------------------------

/// The environment variable that switches the gizmos on at startup. See the module header
/// for why it is not a launch flag.
pub const ENV_VAR: &str = "DBT_GIZMOS";

/// Whether anything is drawn. **Not player state** — a display setting of this process,
/// which is why it may be a `Resource` (`docs/multiplayer.md` forbids player *state* as a
/// resource, not every resource).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GizmoToggle {
    pub on: bool,
}

impl GizmoToggle {
    /// Reads [`ENV_VAR`] exactly once, while the app is being built.
    pub fn from_env() -> Self {
        Self { on: enabled_from_text(std::env::var(ENV_VAR).ok().as_deref()) }
    }
}

/// Whether a text switches the toggle on.
///
/// A function of its own, so the rule can be checked **without environment variables** —
/// the same pattern as `shared::cli::script_forces_headless`: a test that sets
/// `DBT_GIZMOS` checks the process and not the rule, and disturbs every test running in
/// parallel in the same process on the side.
pub fn enabled_from_text(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), Some("1" | "on" | "yes" | "true"))
}

/// The condition under which the three drawing systems run at all.
pub fn gizmos_on(toggle: Res<GizmoToggle>) -> bool {
    toggle.on
}

/// **F4** toggles — F3 belongs to the overlay (`super::update_overlay`).
pub fn toggle_gizmos(keys: Res<ButtonInput<KeyCode>>, mut toggle: ResMut<GizmoToggle>) {
    if keys.just_pressed(KeyCode::F4) {
        toggle.on = !toggle.on;
        info!("gizmos {}", if toggle.on { "on" } else { "off" });
    }
}

/// What was really drawn in the last round.
///
/// Not decoration: `docs/lessons/performance.md` demands that nobody walks all entities to
/// answer a local question — these two numbers are the **measurement** for that, and they
/// stand in the log instead of being guessed. One field, one writer: `anchors` is written
/// by [`draw_anchors`], `players` by [`draw_players`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GizmoCounts {
    pub anchors: usize,
    pub players: usize,
}

/// Installs the toggle, the counters, the grid group and the line widths into the app.
///
/// Called by [`super::DebugPlugin`] — the same shape as [`super::screenshot::install`], so
/// that `debug/mod.rs` stays thin.
pub fn install(app: &mut App) {
    app.insert_resource(GizmoToggle::from_env())
        .init_resource::<GizmoCounts>()
        // A group of its own just for the grid: the line width hangs on the group, not on
        // the individual call. Without it the grid would be just as fat as the outlines
        // and would drown them out in the image.
        .init_gizmo_group::<GridGizmos>();

    let mut store = app.world_mut().resource_mut::<GizmoConfigStore>();
    let (statement, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    statement.line.width = LINE_PX;
    statement.depth_bias = DEPTH_BIAS;
    let (grid, _) = store.config_mut::<GridGizmos>();
    grid.line.width = GRID_PX;
    grid.depth_bias = DEPTH_BIAS;
}

/// The gizmo group of the ground grid — thinner than anything that makes a statement.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct GridGizmos;

/// The three drawing systems as **one named set**.
///
/// That is not cosmetics but the only way to check that they are registered at all: this
/// crate does not enable `bevy_utils/debug`, and without that feature `System::name()`
/// returns literally `"<Enable the debug feature to see the name>"`
/// (`bevy_utils-0.19.0/src/debug_info.rs:10-21`) — so a test over system names is blind
/// here on principle. Through a named set it works without names:
/// `schedule.graph().systems_in_set(GizmoSystems.intern())` counts them
/// (`bevy_ecs-0.19.0/src/schedule/schedule.rs:964-980`, `tests/debug.rs`).
///
/// And it carries the condition: [`gizmos_on`] is evaluated **once** per run and not three
/// times.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GizmoSystems;

// ---------------------------------------------------------------------------
// 1. Anchor surfaces — what is anchorable?
// ---------------------------------------------------------------------------

/// Outlines **every** [`AnchorSurface`] in cyan.
///
/// The query carries `With<AnchorSurface>` and therefore touches only the tagged entities
/// and not the world: the ground, the untagged wall from `maps.ron` and every
/// non-anchorable grid house drop out in the ECS already and cost not one comparison
/// (`docs/lessons/performance.md`, rule 1). How many there are stands in [`GizmoCounts`]
/// and in the log.
pub fn draw_anchors(
    mut gizmos: Gizmos,
    mut counts: ResMut<GizmoCounts>,
    surfaces: Query<(&GlobalTransform, Option<&Body>, Option<&Block>), With<AnchorSurface>>,
) {
    let mut drawn = 0usize;
    for (transform, body, block) in &surfaces {
        let Some(half_size_m) = anchor_half_size(body, block) else {
            continue;
        };
        outline_anchor(&mut gizmos, transform.translation(), transform.rotation(), half_size_m);
        drawn += 1;
    }
    // Write only on change: a `ResMut` that receives the same value every frame triggers
    // change detection every frame (§11 "nothing changes per frame").
    if counts.anchors != drawn {
        info!("gizmos: {drawn} anchor surfaces outlined");
        counts.anchors = drawn;
    }
}

/// The **half** edge length that gets outlined — or `None` when the entity has no shape.
///
/// [`Body`] wins over [`Block`], and that is deliberate: `Body::half_size_m` is the hull
/// the hook really tests against, `Block::size` only the one `render` turns into
/// triangles. Should the two ever drift apart, the image is to show the **hook truth** and
/// not the prettier shape. `Block` carries the **whole** edge, `Body` the half
/// (`src/shared/geometry.rs`) — the factor of 2 sits exactly here.
pub fn anchor_half_size(body: Option<&Body>, block: Option<&Block>) -> Option<Vec3> {
    body
        .map(|k| k.half_size_m)
        .or_else(|| block.map(|b| b.size * 0.5))
}

/// Draws the twelve edges of an anchor surface.
///
/// Generic over the gizmo group, so a test can draw the lines into a [`GizmoBuffer`] of
/// its own and **count them**, without building an app.
pub fn outline_anchor<C, K>(
    gizmos: &mut GizmoBuffer<C, K>,
    center_m: Vec3,
    rotation: Quat,
    half_size_m: Vec3,
) where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    // `cube` scales a unit cube — so the WHOLE edge goes into the scale.
    gizmos.cube(
        Transform { translation: center_m, rotation: rotation, scale: half_size_m * 2.0 },
        color(CYAN),
    );
}

// ---------------------------------------------------------------------------
// 2. Scale — how big, how far?
// ---------------------------------------------------------------------------

/// Ground grid and axis cross at the origin.
///
/// Touches **not a single entity**: both hang on the world origin alone.
pub fn draw_reference(mut grid: Gizmos<GridGizmos>, mut cross: Gizmos) {
    ground_grid(&mut grid);
    axis_cross(&mut cross);
}

/// The grid in the XZ plane.
pub fn ground_grid<C, K>(gizmos: &mut GizmoBuffer<C, K>)
where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    // `grid` puts its lattice in the XY plane (`bevy_gizmos-0.19.0/src/grid.rs:199-201`);
    // the quarter turn around X tips it onto the ground.
    gizmos
        .grid(
            Isometry3d::new(Vec3::Y * GRID_HEIGHT_M, Quat::from_rotation_x(-FRAC_PI_2)),
            UVec2::splat(GRID_CELLS),
            Vec2::splat(GRID_CELL_M),
            color(GRID),
        )
        .outer_edges();
}

/// Three axes through the origin: the positive half with a tip, the negative one dimmed.
pub fn axis_cross<C, K>(gizmos: &mut GizmoBuffer<C, K>)
where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    for (direction, tone) in
        [(Vec3::X, AXIS_X), (Vec3::Y, AXIS_Y), (Vec3::Z, AXIS_Z)]
    {
        let arm = direction * AXIS_M;
        // The negative half first and darker — it says "it carries on over here" without
        // pulling the eye away from the sign. For −Z that is the camera's view direction
        // (`docs/conventions.md`: yaw = 0 means looking toward −Z), so exactly the line
        // you read depth off in an image.
        gizmos.line(-arm, Vec3::ZERO, dimmed(tone, DIM_FACTOR));
        gizmos.arrow(Vec3::ZERO, arm, color(tone));
    }
}

// ---------------------------------------------------------------------------
// 3. Players — who stands where?
// ---------------------------------------------------------------------------

/// Marks **every** player: hull, foot cross, mast.
///
/// No `.single()` and no `With<LocalPlayer>` — every player is one of many
/// (`docs/multiplayer.md` rule 3). The only one skipped is whoever's hull the camera sits
/// in: see [`aabb_contains`].
pub fn draw_players(
    mut gizmos: Gizmos,
    mut counts: ResMut<GizmoCounts>,
    data: Res<GameData>,
    players: Query<&GlobalTransform, With<PlayerId>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let s = &data.game.player;
    let mut drawn = 0usize;
    for transform in &players {
        let feet_m = transform.translation();
        let (center_m, half_size_m) = player_aabb(feet_m, s.height_m, s.radius_m);
        if cameras.iter().any(|k| aabb_contains(center_m, half_size_m, k.translation())) {
            continue;
        }
        mark_player(&mut gizmos, feet_m, center_m, half_size_m);
        drawn += 1;
    }
    if counts.players != drawn {
        info!("gizmos: {drawn} players marked");
        counts.players = drawn;
    }
}

/// Whether a point lies inside an axis-aligned box.
///
/// The camera hangs as a child of the local player today and therefore sits **inside** his
/// capsule (`src/render/mod.rs::attach_camera`, eye height 1.6 m at a body height of
/// 1.8 m). Drawing it anyway would lay your own hull as a lattice over the whole image:
/// at 0.35 m in front of a 60-degree lens one edge fills the frame.
///
/// What is checked is **the geometry and not the `LocalPlayer` marker**. The marker would
/// be the same answer today and the wrong one tomorrow: as soon as there is a third-person
/// or a free camera, your own capsule belongs in the image again — and then this rule
/// still holds, without anybody having to think of it.
pub fn aabb_contains(center_m: Vec3, half_size_m: Vec3, point_m: Vec3) -> bool {
    let distance = (point_m - center_m).abs();
    distance.x <= half_size_m.x && distance.y <= half_size_m.y && distance.z <= half_size_m.z
}

/// Hull, foot cross and mast of a player.
pub fn mark_player<C, K>(
    gizmos: &mut GizmoBuffer<C, K>,
    feet_m: Vec3,
    center_m: Vec3,
    half_size_m: Vec3,
) where
    C: GizmoConfigGroup,
    K: 'static + Send + Sync,
{
    let tone = color(PLAYER);
    gizmos.cube(
        Transform { translation: center_m, rotation: Quat::IDENTITY, scale: half_size_m * 2.0 },
        tone,
    );
    // The foot cross sits at the model's origin, and that lies between the feet
    // (`docs/conventions.md`) — so it marks the point `warp` sets and the one an
    // `assert height` measures.
    gizmos.cross(Isometry3d::from_translation(feet_m), half_size_m.x, tone);
    gizmos.line(feet_m, feet_m + Vec3::Y * (half_size_m.y * 2.0 + MAST_M), tone);
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn color(rgb: [f32; 3]) -> Color {
    Color::linear_rgb(rgb[0], rgb[1], rgb[2])
}

fn dimmed(rgb: [f32; 3], fraction: f32) -> Color {
    Color::linear_rgb(rgb[0] * fraction, rgb[1] * fraction, rgb[2] * fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer to draw into without an app running — `Gizmos` is only a `SystemParam`
    /// around exactly this type (`bevy_gizmos-0.19.0/src/gizmos.rs:155-175`).
    fn buffer() -> GizmoBuffer<DefaultGizmoConfigGroup, ()> {
        GizmoBuffer::new()
    }

    /// All **real** points of a buffer.
    ///
    /// Bevy terminates every polyline with a `Vec3::NAN` and colors it with
    /// `LinearRgba::NAN` — that is the separator between two strips, not a point
    /// (`bevy_gizmos-0.19.0/src/gizmos.rs:939-942` and `:501-515`). Whoever counts it in
    /// counts one corner too many and compares a color that is none.
    fn points(b: &GizmoBuffer<DefaultGizmoConfigGroup, ()>) -> Vec<Vec3> {
        b.list_positions
            .iter()
            .chain(b.strip_positions.iter())
            .copied()
            .filter(|p| p.is_finite())
            .collect()
    }

    fn highest_point(b: &GizmoBuffer<DefaultGizmoConfigGroup, ()>) -> f32 {
        points(b).iter().fold(f32::MIN, |m, p| m.max(p.y))
    }

    #[test]
    fn an_anchor_outline_is_cyan_and_has_twelve_edges() {
        let mut b = buffer();
        outline_anchor(&mut b, Vec3::new(10.0, 5.75, -28.0), Quat::IDENTITY, Vec3::splat(5.0));

        // A cuboid has twelve edges: eight as two rings of five points each in the strip
        // (plus the NAN separator), four as three pairs in the list.
        assert_eq!(b.strip_positions.len(), 11, "two rings of five points plus the separator");
        assert_eq!(b.list_positions.len(), 6, "three connections of two points each");
        assert_eq!(points(&b).len(), 16, "sixteen real corners");

        // The color is not a matter of taste (docs/conventions.md §3).
        let cyan = color(CYAN).to_linear();
        let colors: Vec<_> = b
            .strip_colors
            .iter()
            .chain(b.list_colors.iter())
            .filter(|c| c.red.is_finite())
            .collect();
        assert!(!colors.is_empty(), "nothing was colored at all");
        for c in colors {
            assert_eq!(*c, cyan, "an anchor surface is cyan, nothing else");
        }
    }

    #[test]
    fn the_outline_takes_the_full_edge_not_the_half() {
        // The factor of 2 between `Body::half_size_m` and `Block::size` is the trap you do
        // not notice in the image (`src/world/map.rs`): an outline twice the size looks
        // like a generous frame and is a lie.
        let mut b = buffer();
        outline_anchor(&mut b, Vec3::ZERO, Quat::IDENTITY, Vec3::new(4.0, 6.0, 4.0));
        let top = highest_point(&b);
        assert!((top - 6.0).abs() < 1e-5, "top edge at {top} instead of 6.0 m");
    }

    #[test]
    fn body_wins_over_block_and_both_over_nothing() {
        let body = Body { half_size_m: Vec3::splat(2.0), mask: crate::shared::BodyMask::ANCHORABLE };
        let block = Block { size: Vec3::splat(10.0), color: [0.4, 0.4, 0.4] };
        assert_eq!(anchor_half_size(Some(&body), Some(&block)), Some(Vec3::splat(2.0)));
        assert_eq!(anchor_half_size(None, Some(&block)), Some(Vec3::splat(5.0)));
        assert_eq!(anchor_half_size(Some(&body), None), Some(Vec3::splat(2.0)));
        // An anchor surface without any shape is not guessed at, it is left out.
        assert_eq!(anchor_half_size(None, None), None);
    }

    #[test]
    fn the_axis_cross_uses_none_of_the_three_signal_colors() {
        // Cyan, amber and crimson are reserved for gameplay exclusively
        // (docs/conventions.md §3). The test checks the property, not the numeric value:
        // a color is suspect when it comes close to one of the three tones.
        for tone in [AXIS_X, AXIS_Y, AXIS_Z, GRID, PLAYER] {
            let [r, g, b] = tone;
            assert!(!(r < 0.4 && g > 0.5 && b > 0.5), "{tone:?} is cyan");
            assert!(!(r > 0.6 && g > 0.3 && g < 0.75 && b < 0.3), "{tone:?} is amber");
            assert!(!(r > 0.5 && g < 0.25 && b < 0.25), "{tone:?} is crimson");
        }
        // And cyan stays reserved for the anchor surface.
        assert!(CYAN[0] < 0.4 && CYAN[1] > 0.5 && CYAN[2] > 0.5);
    }

    #[test]
    fn the_ground_grid_lies_flat_and_not_upright() {
        // `grid` draws in the XY plane; without the quarter turn the grid would stand in
        // front of the camera like a wall — and in a way that looks deliberate in an
        // image.
        let mut b = buffer();
        ground_grid(&mut b);
        let points = points(&b);
        assert!(!points.is_empty(), "the grid drew not a single line");
        let half = GRID_CELLS as f32 * GRID_CELL_M * 0.5;
        for p in points {
            assert!((p.y - GRID_HEIGHT_M).abs() < 1e-4, "grid point at y = {}", p.y);
            assert!(p.x.abs() <= half + 1e-3 && p.z.abs() <= half + 1e-3, "{p:?} lies outside");
        }
    }

    #[test]
    fn the_toggle_reads_only_clear_yes_words() {
        for yes in ["1", "on", "yes", "true", " 1 "] {
            assert!(enabled_from_text(Some(yes)), "{yes:?} should switch it on");
        }
        for no in ["0", "", "off", "no", "false", "maybe"] {
            assert!(!enabled_from_text(Some(no)), "{no:?} should not switch it on");
        }
        assert!(!enabled_from_text(None), "without the variable it stays off");
    }

    #[test]
    fn the_camera_sits_in_the_hull_of_the_player_carrying_it() {
        // The numbers are the ones from `game.ron`: body 1.8 m, radius 0.35 m, eye 1.6 m.
        let (center, half) = player_aabb(Vec3::new(6.0, 20.0, 45.0), 1.8, 0.35);
        let eye = Vec3::new(6.0, 20.0 + 1.6, 45.0);
        assert!(aabb_contains(center, half, eye), "your own eye sits inside the hull");

        // Another player two meters away is not the same capsule.
        let (center2, half2) = player_aabb(Vec3::new(8.0, 20.0, 45.0), 1.8, 0.35);
        assert!(!aabb_contains(center2, half2, eye), "the neighbor is drawn");
        // And neither is a free camera above the player.
        assert!(!aabb_contains(center, half, Vec3::new(6.0, 30.0, 45.0)));
    }

    #[test]
    fn a_player_gets_hull_cross_and_mast() {
        let mut b = buffer();
        let feet = Vec3::new(0.0, 2.0, 0.0);
        let (center, half) = player_aabb(feet, 1.8, 0.35);
        mark_player(&mut b, feet, center, half);
        assert!(points(&b).len() > 16, "hull, cross and mast, not just one line");
        // The mast sticks out MAST_M above the head: 2.0 + 1.8 + 3.0.
        let top = highest_point(&b);
        assert!((top - (2.0 + 1.8 + MAST_M)).abs() < 1e-4, "mast tip at {top}");
    }
}
