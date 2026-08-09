//! data — load the RON files. **Runs before everything else.**
//!
//! > **Numbers belong in RON, not in Rust.** A new Titan kind, a blade tier, a gas cost
//! > figure: file work, not Rust. Only *units* and *mechanics* stand in the code
//! > (`prompts/init.md` §4).
//!
//! Why that is not optional: **balancing is the work that happens most often.** If it needs a
//! rebuild, it does not happen. And another agent can change one RON line without
//! understanding this code.
//!
//! **No `serde(default)` for game values.** A missing value is meant to crash at load time,
//! not to quietly slip a zero in — otherwise you hunt the bug in the code while it sits in the
//! file. That is also why loading happens **synchronously during setup** and not through the
//! `AssetServer`: an error is meant to be loud **at startup**, with file name and line, and
//! not three systems later as an empty screen (§9d).
//!
//! **And `#[serde(deny_unknown_fields)]` on every type in here.** Leaving a field out always
//! crashed; *adding* one did not — serde reads past an unknown field without a word. Measured
//! on 2026-08-09: an `erfunden_m: 42.0` in `scale.ron` and a `gewicht_kg: 70.0` under
//! `game.ron: player` both loaded without a word. That is exactly the trap for the file the
//! user's numbers are going to be added to from now on: an addition at the wrong nesting level
//! vanishes silently, and the number you typed in is not in the game.
//!
//! **This is the only place that knows file names.** Everybody else asks for the logical name;
//! `tools/norms.py` falls over when a path stands in the code anywhere else.

use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameData::load(&assets_root()));
    }
}

/// Where `assets/` lives.
///
/// **`cargo run`, never the bare binary**: the binary looks for `assets/` relative to the
/// working directory and finds nothing — empty world, no error message, looks exactly like a
/// render bug (`prompts/init.md` §3). So that a `cargo test` out of `tests/` finds it anyway,
/// the crate directory is checked as well.
fn assets_root() -> PathBuf {
    let here = PathBuf::from("assets/data");
    if here.is_dir() {
        return here;
    }
    let at_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
    if at_crate.is_dir() {
        return at_crate;
    }
    panic!(
        "assets/data/ not found — neither at {:?} nor at {:?}.\n\
         Start with `cargo run`, not with the bare binary from target/debug/ \
         (prompts/init.md §3).",
        here.canonicalize().unwrap_or(here.clone()),
        at_crate
    );
}

/// Everything that comes out of `assets/data/`. One resource, many readers, **no writer**.
#[derive(Resource, Debug, Clone)]
pub struct GameData {
    pub game: Game,
    pub gear: Gear,
    pub titans: Titans,
    pub art: Art,
    pub missions: Missions,
    pub traits: Traits,
    pub maps: Maps,
    /// The **one truth about sizes**, laid down by the user. Every other file only mirrors it;
    /// `tests/data.rs` falls over the moment one of them deviates from this.
    pub scale: Scale,
}

impl GameData {
    pub fn load(dir: &Path) -> Self {
        GameData {
            game: load_ron(dir, "game.ron"),
            gear: load_ron(dir, "gear.ron"),
            titans: load_ron(dir, "titan.ron"),
            art: load_ron(dir, "art.ron"),
            missions: load_ron(dir, "missions.ron"),
            traits: load_ron(dir, "traits.ron"),
            maps: load_ron(dir, "maps.ron"),
            scale: load_ron(dir, "scale.ron"),
        }
    }

    /// A map by its logical name. `None` means: it is not in `maps.ron` — and the caller
    /// reports that loudly instead of building an empty world.
    pub fn map(&self, id: &str) -> Option<&Map> {
        self.maps.maps.get(id)
    }

    /// The map that is built at startup (`maps.ron: current`).
    /// `tests/data.rs` pins down that it exists.
    pub fn current_map(&self) -> Option<&Map> {
        self.map(&self.maps.current)
    }

    /// A color out of the one palette. `None` means: the key does not stand in `palette` — no
    /// silent substitute, or sooner or later a signal color slips in.
    pub fn color(&self, name: &str) -> Option<[f32; 3]> {
        self.maps.palette.get(name).map(|(r, g, b)| [*r, *g, *b])
    }

    /// A Titan kind by its logical name. `None` means: it is not in the RON — and the caller
    /// reports that loudly instead of inventing a stand-in Titan.
    pub fn titan(&self, kind: &str) -> Option<&TitanKind> {
        self.titans.kinds.get(kind)
    }

    pub fn model(&self, name: &str) -> Option<&Model> {
        self.art.models.get(name)
    }

    /// A size class by its logical name (`scale.ron: titan.classes`). `None` means: the class
    /// does not stand in the RON — `tests/data.rs` catches that before a Titan of height 0
    /// stands in the ground.
    pub fn size_class(&self, name: &str) -> Option<&SizeClass> {
        self.scale.titan.classes.get(name)
    }

    /// The height of a Titan kind. It does **not** stand in `titan.ron` — only the class does,
    /// and the height comes out of `scale.ron`. One number, one place.
    pub fn titan_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.size_class(&kind.size_class).map(|k| k.height_m)
    }

    /// Where the Cortex sits — **the user's figure in meters**, not `height_m * 0.89`.
    ///
    /// **The only lethal weak point** (`F-030`). Until 2026-08-09 it was computed here out of
    /// the fraction; that was convenient and wrong: the user names five Cortex heights in
    /// meters, and *a direct figure in meters beats every derivation*. For the small class the
    /// calculation was off by 4 cm. The fraction is now what it is for the user — a rule that
    /// `tests/data.rs` checks the five numbers against.
    pub fn titan_cortex_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.size_class(&kind.size_class).map(|k| k.cortex_height_m)
    }

    /// The largest head height the user's head rule allows for this kind
    /// (`height_m * scale.titan.max_head_fraction`, that is 1/9 of the body height).
    ///
    /// It is the **geometric upper bound for `cortex_radius_m`**: a hit zone whose diameter is
    /// larger than the whole head cannot be the base of a neck. Before this guard, `scuttler`
    /// (0.80 m Cortex on a 0.47 m head) and `weaver` (0.90 m) carried exactly that.
    pub fn titan_max_head_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.titan_height_m(kind).map(|h| h * self.scale.titan.max_head_fraction)
    }
}

/// Reads a RON file, or aborts with a message that points at the error **in the file** instead
/// of in the code.
fn load_ron<T: for<'a> Deserialize<'a>>(dir: &Path, file: &str) -> T {
    let path = dir.join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("{}: cannot be read — {e}", path.display())
    });
    ron::de::from_str(&text).unwrap_or_else(|e| {
        // ron names line and column; those are exactly what you want to see.
        panic!("{}: not valid RON — {e}", path.display())
    })
}

// ---------------------------------------------------------------------------
// game.ron — tuning: Vector Gear, camera, physics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub simulation_hz: f64,
    /// Solver substeps per simulation step — avian's [`SubstepCount`](avian3d::prelude::SubstepCount),
    /// whose own default is 6. **Measured, not guessed:** 24 is the smallest value that holds
    /// both the swing loss (4.26 %/s against 8.97 %/s at 6) and the wall. It belongs here and
    /// not in Rust, because it is the price of the simulation and thus a number you tune (§4).
    pub substeps: u32,
    pub gravity_m_s2: f32,
    pub player: PlayerTuning,
    pub vector: VectorTuning,
    pub camera: CameraTuning,
    pub world: WorldTuning,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerTuning {
    pub height_m: f32,
    pub radius_m: f32,
    pub run_speed_m_s: f32,
    pub jump_speed_m_s: f32,
    pub eye_height_m: f32,
    /// Largest distance per substep of the integrator. Has to be strictly smaller than
    /// [`WorldTuning::min_wall_m`], or the player tunnels through the thinnest wall.
    pub max_substep_m: f32,
    /// Coefficient of friction of the player's capsule against everything else.
    ///
    /// Combined with the surface's own value by `CoefficientCombine::Min`, so that a wall
    /// cannot brake the player — a measurement showed a combined 0.65 eating 75 % of the
    /// speed per second when a swinging player grazes a wall in a 7 m alley.
    pub friction: f32,
    /// Coefficient of restitution of the player's capsule. `0.0` = no bounce.
    pub restitution: f32,
    /// How steep a surface may be and still count as ground. Degrees in the RON, radians in
    /// the code (`docs/conventions.md`). Decides whether [`MovementState::Grounded`] holds —
    /// and with it whether you may jump off it.
    ///
    /// [`MovementState::Grounded`]: crate::shared::MovementState::Grounded
    pub max_ground_slope_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorTuning {
    pub hook_range_m: f32,
    pub hook_speed_m_s: f32,
    pub hook_retract_speed_m_s: f32,
    pub reel_speed_m_s: f32,
    pub min_rope_m: f32,
    /// Gauss-Seidel iterations over both rope constraints (`shared::rope::rope_step`).
    pub rope_iterations: u32,
    pub gas_tank: f32,
    pub gas_boost_per_s: f32,
    pub gas_reel_per_s: f32,
    /// Who pays first when the tank does not cover both. **A game-value decision**, which is
    /// why it stands here and not as an `if` in `vector/gas.rs`.
    pub gas_priority: Vec<GasConsumer>,
    pub boost_m_s2: f32,
    pub max_speed_m_s: f32,
}

/// Who spends gas. Its own type instead of `String`, so that a typo in the RON **crashes at
/// load time** instead of silently losing a consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum GasConsumer {
    /// `F-007` gas boost.
    Boost,
    /// `F-005` reel-in.
    ReelIn,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraTuning {
    /// Field of view in **ground combat** — the base, not the ceiling. Has to lie inside the
    /// window `scale.ron: camera.min_ground_fov_deg ..= max_ground_fov_deg`.
    pub fov_deg: f32,
    /// Field of view at `vector.max_speed_m_s`. `F-017` will interpolate between the two
    /// later; the **curve** belongs in the code, the two **ends** belong in the RON.
    pub fov_max_speed_deg: f32,
    pub mouse_deg_per_px: f32,
    pub pitch_limit_deg: f32,
    pub smoothing_half_life_s: f32,
}

/// What the world itself costs: spatial index and collision.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTuning {
    /// Edge length of one grid cell (`T-036a`). **Unmeasured** —
    /// `docs/lessons/performance.md`, `docs/QUESTIONS.md` Q-014. (Stood here as Q-013 until
    /// 2026-08-09; Q-013 is the question about the maximum rope length.)
    pub cell_m: f32,
    /// Half the edge length of the grid. Has to cover the map **and** the hook range.
    pub half_extent_m: f32,
    /// How many occupied cells it takes before a body goes into the linear large-body list.
    pub large_body_cells: u32,
    /// The thinnest wall allowed. Calibrates [`PlayerTuning::max_substep_m`].
    pub min_wall_m: f32,
    /// Collision margin against jitter in contact.
    pub collision_margin_m: f32,
}

// ---------------------------------------------------------------------------
// maps.ron — the city as data (E13, F-003, Q-010)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maps {
    /// Logical name of the map that is built at startup.
    pub current: String,
    /// The one palette, linear RGB. No RGB triple per block — or sooner or later a signal
    /// color slips in (`docs/conventions.md`).
    pub palette: BTreeMap<String, (f32, f32, f32)>,
    pub maps: BTreeMap<String, Map>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    pub name: String,
    /// Edge length in X and Z, in meters.
    pub size_m: (f32, f32),
    /// Seed for `shared::Rng`. Part of the state, **never** `rand::random()`.
    pub seed: u64,
    pub layout: Layout,
    /// Explicitly placed boxes. They beat the generated layout.
    pub blocks: Vec<MapBlock>,
}

/// The rule `world` deterministically generates buildings from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub lot_m: f32,
    pub street_m: f32,
    pub min_height_m: f32,
    pub max_height_m: f32,
    /// Fraction of lots that get built on, 0..1.
    pub density: f32,
    /// Fraction of anchorable buildings, 0..1.
    pub anchorable_fraction: f32,
    /// Radius around the origin that stays clear.
    pub clear_radius_m: f32,
    /// Allowed color keys out of [`Maps::palette`].
    pub colors: Vec<String>,
}

/// One explicitly placed box. Origin at the center, like `shared::Block`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapBlock {
    pub center_m: (f32, f32, f32),
    /// Full edge length. The index keeps the half internally.
    pub size_m: (f32, f32, f32),
    pub color: String,
    /// Gets `shared::AnchorSurface` and `BodyMask::ANCHORABLE` (`F-003`).
    pub anchorable: bool,
    /// Stops a body — `BodyMask::SOLID`.
    pub solid: bool,
    /// `false` = housing, and it has to stay below `scale.ron: architecture.heights_m`
    /// `house_large` (11.5 m). `true` = church, watchtower, tree, wall — **they carry the
    /// vertical** and are allowed above it.
    ///
    /// Without this field the flatness rule only held for `layout`, and the explicitly placed
    /// boxes of the graybox stood at 12, 14 and 18 m ridge height outside the guard that is
    /// supposed to keep the city flat.
    pub landmark: bool,
}

// ---------------------------------------------------------------------------
// gear.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    pub blades: BladeTuning,
    pub resupply: ResupplyTuning,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BladeTuning {
    pub start_pairs: u8,
    pub wear_per_hit: f32,
    pub damage_per_m_s: f32,
    pub min_speed_m_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResupplyTuning {
    pub gas_per_s: f32,
    pub range_m: f32,
}

// ---------------------------------------------------------------------------
// titan.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titans {
    pub kinds: BTreeMap<String, TitanKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanKind {
    /// Key in `scale.ron: titan.classes` (`F-064`). **No height per kind** — height and Cortex
    /// height come out of the scale table through [`GameData::titan_height_m`] and
    /// [`GameData::titan_cortex_height_m`], so that the two cannot drift apart.
    ///
    /// Was called `groesse` until 2026-08-09; a field name ending in "-size" reads like a
    /// length (`docs/conventions.md` §5), but what stands here is a key.
    pub size_class: String,
    pub speed_m_s: f32,
    pub cortex_radius_m: f32,
    pub regen_per_s: f32,
    /// Windup of every attack. **At least 0.4 s** — Bible, pillar P4 (readability before
    /// realism). `tests/data.rs` falls over when a kind drops below it.
    pub windup_s: f32,
    pub model: String,
}

// ---------------------------------------------------------------------------
// scale.ron — the one truth about sizes (laid down by the user, 2026-08-09)
// ---------------------------------------------------------------------------

/// The user's size table as a type.
///
/// **These numbers are not untuned, they are laid down.** Everything else in `assets/data/` is
/// anybody's to change; whoever changes something here changes a decision of the user's. The
/// precedence rule that goes with it: **a direct figure in meters from the user beats every
/// derivation** — including the conversion out of the backlog (`docs/QUESTIONS.md` Q-002).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    /// The world is **deliberately not scaled uniformly**: architecture 1.0, Titans 1.4, walls
    /// 2.4. A Titan reads bigger than "realistic", a wall monumental — that is the visual
    /// language, not sloppiness. Whoever levels the three destroys it.
    pub architecture_factor: f32,
    pub titan_factor: f32,
    pub wall_factor: f32,
    pub reference: Reference,
    pub architecture: ArchitectureSizes,
    pub titan: TitanScale,
    pub wall: WallSizes,
    pub camera: CameraScale,
    pub vector: VectorScale,
}

/// What the eye measures everything else against.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// "Check the capsule exactly!" — `game.ron: player.height_m` has to be **exactly** this.
    pub human_height_m: f32,
    pub door_height_m: f32,
    /// Window for `maps.ron: layout.street_m`. "Keep them tight."
    pub min_street_m: f32,
    pub max_street_m: f32,
    /// 1/7.5. The **yardstick** for [`TitanScale::min_head_fraction`]: the Titan head is
    /// relatively smaller, and that is exactly what makes the eye read "huge" instead of
    /// "close".
    pub human_head_fraction: f32,
}

/// Building heights. The housing is deliberately flat; the vertical comes from the wall, the
/// church, the watchtower and the trees.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureSizes {
    /// Logical name -> total height in meters.
    pub heights_m: BTreeMap<String, f32>,
    /// Logical name -> eaves height (top edge of the outer wall, where the roof starts).
    /// **Filled in only where the user named a number** — every key has to stand in
    /// [`Self::heights_m`] as well, and `tests/data.rs` checks that.
    pub eaves_m: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanScale {
    /// The fraction of the body height the Cortex sits at (0.89) — **a check rule, not a
    /// source.** The five Cortex heights stand as figures in meters in
    /// [`SizeClass::cortex_height_m`]; this value is what the user wrote next to them, and
    /// `tests/data.rs` uses it to hold the five inside the window 0.88..0.90.
    ///
    /// It serves as a source only where the user named no Cortex height — today exactly once:
    /// for the Ashwalker (150 m ⇒ 133.5 m).
    pub cortex_fraction: f32,
    /// Head height as a fraction of the body height: 1/10 to 1/9.
    pub min_head_fraction: f32,
    pub max_head_fraction: f32,
    /// The five size classes (`F-064`). `titan.ron` points here with `size_class`.
    pub classes: BTreeMap<String, SizeClass>,
    /// The 150 m boss. **Outside the classes**: not a scaled enemy kind but a structure with a
    /// face. The user's own check: 150 − 120 = 30 m above the wall.
    pub ashwalker_height_m: f32,
}

/// One size class. Its own type instead of a bare `f32`, so that later per-class values (range,
/// hit points, stride length) land here instead of being duplicated per kind.
///
/// **Both fields are figures in meters from the user** — `cortex_height_m` is not computed from
/// `height_m`. The fraction ([`TitanScale::cortex_fraction`]) checks them instead of producing
/// them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeClass {
    pub height_m: f32,
    /// Height of the Cortex above the ground. The **only lethal weak point** (`F-030`).
    pub cortex_height_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallSizes {
    pub height_m: f32,
    pub top_thickness_m: f32,
    /// Larger than [`Self::top_thickness_m`]: the wall tapers toward the top.
    pub base_thickness_m: f32,
    /// A staging point at half height. Without it the crown of the wall cannot be reached from
    /// below with a 90 m anchor range (120 > 90).
    pub platform_height_m: f32,
    /// **The ladder of scale.** A 0.6 m stone course and a band every 15 m are the reason a
    /// 120 m wall looks big instead of gray.
    pub stone_course_m: f32,
    pub band_spacing_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraScale {
    /// Eye height. `game.ron: player.eye_height_m` has to be this.
    pub height_m: f32,
    /// Window for `game.ron: camera.fov_deg` — **ground combat**, "the biggest lever".
    pub min_ground_fov_deg: f32,
    pub max_ground_fov_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorScale {
    /// 90 m, straight from the user. `game.ron: vector.hook_range_m` has to be this — and
    /// **not** the 112 m out of the conversion (Q-002).
    pub anchor_range_m: f32,
    /// "x1.5 vs. standard". **Is not applied to anything** as long as the reference is missing
    /// (`docs/QUESTIONS.md` Q-018).
    pub speed_factor: f32,
}

// ---------------------------------------------------------------------------
// art.ron — the registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Art {
    pub models: BTreeMap<String, Model>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Name of the `.blend` without the extension. The auto-export turns it into the `.glb`
    /// (§7).
    pub blend: String,
    /// `false` ⇒ the **placeholder path** out of Bevy primitives. Both paths have to run at any
    /// time and have the same size, hit zone and scale — otherwise switching is not a switch
    /// but a rebuild.
    pub use_blend: bool,
    pub scale: f32,
    /// Set only on third-party material: URL · date · license · what it is meant to replace.
    /// That makes the replacement list a `grep` (§7).
    pub attribution: Option<String>,
}

// ---------------------------------------------------------------------------
// missions.ron / traits.ron — still nearly empty, but present and loaded
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Missions {
    pub templates: BTreeMap<String, MissionTemplate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionTemplate {
    pub name: String,
    pub map: String,
    /// The mission arc runs 5–7 min (Bible 5, change 10).
    pub target_duration_s: f32,
    pub waves: Vec<Wave>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub at_s: f32,
    pub kind: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Traits {
    pub entries: BTreeMap<String, TraitDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitDef {
    pub name: String,
    pub cost: u32,
    pub description: String,
}
