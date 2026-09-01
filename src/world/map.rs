//! The city out of `assets/data/maps.ron` — **data and a seed, not 200 lines of Rust.**
//!
//! It is built from two sources:
//! 1. `blocks` — explicitly placed cuboids, 1:1 out of the file.
//! 2. `layout` — blocks generated deterministically from `seed` via
//!    [`Rng`](crate::shared::Rng). The same seed yields the same city, on every machine and
//!    in every rollback; `rand::random()` would be a desync here.
//!
//! Every entity gets [`Block`] (that is what `render` sees), [`Body`] (that is what the
//! spatial index sees), the avian pieces [`RigidBody::Static`] and [`Collider::cuboid`], and
//! for `anchorable` an [`AnchorSurface`] on top. **One writer for all four**, so that render
//! shape, index aabb and collision shape cannot drift apart.
//!
//! Since 2026-08-09 the avian components have an **effect**: `PhysicsPlugins` is registered
//! in `src/lib.rs`, and this is the ground the player stands on and the wall he stops at.
//!
//! **`RigidBody::Static` is not optional, even though a bare collider already collides.** A
//! character controller added later filters on `With<ColliderOf>`
//! (`avian3d-0.7.0/.../move_and_slide.rs:82`) and is blind to every collider without a body.
//! Retrofitting that means touching every row of every map — so it stands here from the
//! start, and `tests/player.rs` counts it.
//!
//! ## The trap that does not show up in the picture
//!
//! `Collider::cuboid` takes the **WHOLE edge**, not the half:
//! `avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749` calls
//! `SharedShape::cuboid(x_length * 0.5, ..)` — parry keeps the half internally, avian takes
//! the full one on the outside. [`Body::half_size_m`] and `Aabb3d::new`, by contrast, take
//! the **half** (`bevy_math-0.19.0/src/bounding/bounded3d/mod.rs:66`). A factor of 2 in this
//! spot makes every house twice or half as large without it showing up in the picture —
//! which is why `tests/world.rs::f003_the_colliders_carry_the_half_edge_from_the_file`
//! measures the shape against the file.
//!
//! ## Why the layout does not notice the ground
//!
//! `maps.ron` says: "what is generated leaves room around every placed block". The first
//! placed block is the 400 x 400 m ground slab — a special rule for it would be an
//! `if ground` that nobody ever understands again. Instead [`overlaps`] tests **strictly**
//! (touching does not count): a house stands at y = 0 on the slab whose top edge is at
//! y = 0, and therefore only touches it. Not a special case, just geometry.
//!
//! ## A house is two cuboids, and the second one is the reason you can see the first
//!
//! Since 2026-08-12 a generated house is a **body plus a roof cap** — a smaller cuboid on the
//! eaves, in its own colour. The user had just judged the version without it:
//!
//! > *„häuser sind alle ineinander! keine unterschiedliche höhen! es sieht überhaupt nicht
//! > aus wie eine attack on titan map! viel zu kompakt!"*
//!
//! The cap does **not** make the district taller: the rolled height is the ridge and the roof
//! is cut out of it downward (`data::Roof`). What it does is give every house a silhouette,
//! and — with `Perimeter`'s per-house draws — stop eight equal boxes in a square from reading
//! as one object. Both blocks carry the **same** `anchorable` bit, so the highest thing the
//! player aims at answers the same way the wall below it does (`FIND-059`, from the other
//! side).
//!
//! **No block is ever rotated.** An axis-aligned cuboid is exactly its AABB; a rotated
//! `Cuboid` yields the enclosing, oversized one
//! (`bevy_math-0.19.0/src/bounding/bounded3d/primitive_impls.rs:100-115`), and the hook
//! visibly catches in mid-air. That is a deliberately deferred limitation
//! (`docs/ROADMAP.md`), not a forgotten one.
//!
//! ## And since 2026-08-13 the ground has a height, and the roof has a ridge
//!
//! The user: *„adde verschiedene höhen vom boden her! lass es wie die echte stadt aussehen!
//! aktuell kann man es noch nicht erkennen!"*. Two answers, both data:
//! [`plan_terrain`] steps the ground into terraces with a flight of stairs on every falling
//! edge, and `layout.roof_steps` / `layout.tall_fraction` turn a roofscape of equal flat lids
//! into stepped gables with a handful of 18 m houses among them.
//!
//! Seen: `docs/images/f003-city.png`, driven with `scripts/f003-city.txt`.
//! The terrain: `docs/images/f003-terrain.png` (from the street) and
//! `docs/images/f003-roofscape.png` (from the air), driven with
//! `scripts/w2-terrain-walk.txt`.

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::data::{Damage, GameData, Map, ModelSource, Perimeter};
use crate::shared::{AnchorSurface, Block, Body, CellRole, ModelName, Rng, TerrainField};

use super::index::mask_from;

/// Every question about the same lot gets its **own** stream.
///
/// [`Rng`] is stateless and computes out of `(seed, tick, stream)`; **two callers with the
/// same stream get the same number** (`src/shared/rng.rs`). Were the height the same stream
/// as the color, every tall house would have the same color — a pattern you take for intent
/// when you see it in the picture.
///
/// These are **not tuning numbers** but names: they tell callers apart and therefore stand
/// in the code and not in the RON (§4).
const STREAM_BUILT: u64 = 0xF003_0001;
const STREAM_HEIGHT: u64 = 0xF003_0002;
const STREAM_COLOR: u64 = 0xF003_0003;
const STREAM_ANCHORABLE: u64 = 0xF003_0004;
/// Per **cell**: which height band this whole block sits in, and how far the ring is shifted
/// off its grid position. The three reasons the district stopped being a checkerboard.
const STREAM_LEVEL: u64 = 0xF003_0005;
const STREAM_JITTER_X: u64 = 0xF003_0006;
const STREAM_JITTER_Z: u64 = 0xF003_0007;
/// Per **wing**: how wide the houses of this one run are.
const STREAM_FRONTAGE: u64 = 0xF003_0008;
/// Per **house**: alley, setback, depth, roof pitch.
const STREAM_GAP: u64 = 0xF003_0009;
const STREAM_SETBACK: u64 = 0xF003_000A;
const STREAM_DEPTH: u64 = 0xF003_000B;
const STREAM_RISE: u64 = 0xF003_000C;
/// Per **house**: is this one built to the tall class instead of out of the band?
const STREAM_TALL: u64 = 0xF003_000D;
/// Per **terrain cell**: the notch [`TerrainField`] starts that cell at.
const STREAM_TERRAIN: u64 = 0xF003_000E;
/// Per **cell** and per **house**: how far this block, and this house in it, sit off the
/// damage gradient (`maps.ron: layout.damage`). Two streams and not one, because the whole
/// point of the block draw is that a street front falls as a piece.
const STREAM_DAMAGE_BLOCK: u64 = 0xF003_000F;
const STREAM_DAMAGE_HOUSE: u64 = 0xF003_0010;
/// Per **house**: which remnant of the kit this fallen house wears, and how tall its mound is.
const STREAM_REMNANT: u64 = 0xF003_0011;
const STREAM_MOUND: u64 = 0xF003_0012;

/// How many rng ticks one grid cell owns.
///
/// A closed block draws **per house**, not per cell — otherwise every house in the ring is
/// the same height and the same colour, and the town reads as a stack of identical bars. The
/// tick of house `i` in cell `lot` is `lot * TICKS_PER_LOT + i`, which stays injective as
/// long as a ring never holds more than `TICKS_PER_LOT - 1` houses; [`perimeter_houses`]
/// asserts that instead of silently letting two cells share a draw.
///
/// This is a name, not a tuning number (§4): it says how the rng stream is partitioned.
const TICKS_PER_LOT: u64 = 64;

/// **The pack's architecture, as a shape — the catalogue a generated house is dressed from.**
///
/// One row per logical name in `art.ron` that a row house can wear, with the size the shipped
/// `.glb` behind that name is authored at: the full extent of its own `hit.min`/`hit.max`
/// pair, in metres, x / y / z. The model's front is its **±z face** (`fenster_..._vorn`,
/// `tuer_blatt`, `giebel_v` all sit on it), so `z` is the house's depth and `x` its frontage.
///
/// ⚠️ **These are measurements of a file, not tuning numbers** (§4). Every one of them is
/// verified against the file it claims to describe by
/// `tests/world.rs::f003_the_dressing_catalogue_is_what_the_glb_files_really_measure`, which
/// parses `assets/3d/glb/a-083-fachwerkhaus-*.glb` and falls over on a re-export that moves
/// them. The `y` column is additionally checked against `scale.ron: architecture.heights_m`
/// under the **same key** — that the pack was authored to our own height bands is a claim
/// `art.ron` makes in prose, and this is where it becomes a test.
///
/// They stand in Rust and not in RON for one reason and it is not a good one: the schema of a
/// map lives in `src/data/`, which this round did not own. `art.ron` is where they belong.
pub const DRESSING: [(&str, [f32; 3]); 3] = [
    // a-083-fachwerkhaus-klein
    ("house_small", [6.56, 4.50, 8.32]),
    // a-083-fachwerkhaus-stadthaus
    ("house_town", [9.10, 8.00, 7.90]),
    // a-083-fachwerkhaus-gross
    ("house_large", [8.30, 11.50, 9.90]),
];

/// **The fallen ring, as a shape** — the eight remnants a house that did not survive can wear.
///
/// Same form as [`DRESSING`] and read the same way: one row per logical name in `art.ron`,
/// with the full extent of that file's own `hit.min`/`hit.max` pair in metres, x / y / z. The
/// front is the ±z face, so `x` is the frontage and `z` the depth.
///
/// ⚠️ **Measurements of files, not tuning numbers** (§4) — which is why they stand here and
/// the *distribution* stands in `maps.ron: layout.damage`.
/// `tests/world.rs::f003_the_ruin_catalogue_is_what_the_glb_files_really_measure` parses all
/// fourteen files and falls over on a re-export that moves them. ⚠️ The `hit` pair is a
/// **corner pair** and not a min/max pair (`hit.max.z < hit.min.z` on all 278 files of the
/// drop), so the extent is taken with `abs`.
///
/// The order is the order they are drawn from and it is not meaningful; what a house wears is
/// `rng.index(.., STREAM_REMNANT, ..)` over everything that fits its lot.
pub const RUIN_KIT: [(&str, [f32; 3]); 8] = [
    // a-089-ruine-dach-eingestuerzt
    ("ruin_roof_collapsed", [7.04, 2.62, 5.16]),
    // a-089-ruine-dach-haelfte
    ("ruin_roof_half", [6.72, 4.74, 4.93]),
    // a-089-ruine-giebel — the tallest of the kit
    ("ruin_gable", [6.47, 8.49, 4.01]),
    // a-089-ruine-haufen
    ("ruin_heap", [7.49, 2.40, 5.81]),
    // a-089-ruine-obergeschoss
    ("ruin_upper_floor", [6.95, 5.55, 4.93]),
    // a-089-ruine-pfeiler
    ("ruin_pillar", [5.86, 9.00, 4.87]),
    // a-089-ruine-wand-ecke
    ("ruin_wall_corner", [6.47, 5.60, 6.80]),
    // a-089-ruine-wand-hoch
    ("ruin_wall_high", [6.22, 6.94, 3.96]),
];

/// **The mounds** — what is left where the walls went. Same contract as [`RUIN_KIT`].
///
/// Nothing in this group is over 3 m authored, and that is the design and not the pack's
/// accident: a mound takes the **ground** away and leaves the swing lane alone.
pub const RUBBLE_KIT: [(&str, [f32; 3]); 6] = [
    // a-090-schutt-balken
    ("rubble_beams", [4.10, 2.10, 3.70]),
    // a-090-schutt-deckung
    ("rubble_cover", [3.70, 1.20, 3.31]),
    // a-090-schutt-flach
    ("rubble_flat", [3.94, 0.90, 2.95]),
    // a-090-schutt-haufen-gross
    ("rubble_heap_large", [6.20, 3.00, 4.80]),
    // a-090-schutt-hoch
    ("rubble_high", [4.20, 1.80, 3.50]),
    // a-090-schutt-wandstueck
    ("rubble_wall_piece", [4.33, 2.40, 2.80]),
];

/// How far a house's drawn footprint may be moved to make it **be** the model that dresses it.
///
/// A model is scaled **uniformly** — a half-timbered house stretched on one axis has fat
/// timbers and reads as a mistake — so it has one degree of freedom against a box that has
/// three, and the box is the side that gives way. This is how much it may give.
///
/// Measured on the shipped district, 926 generated houses, three policies (2026-08-18):
///
/// | rule | dressed | fill (model area / box area) | median street |
/// |---|---|---|---|
/// | the model must fit **inside** the box | 780 | **0.66** | 8.55 m |
/// | box may move ±12 % | 291 | 0.97 | 7.43 m |
/// | box may move ±25 % | **766** | 0.96 | **7.39 m** |
///
/// The first rule is the tempting one and it is the wrong one: a model that merely *fits* is
/// on average a third smaller than the collider it stands in, so the hook catches on air and
/// the street widens by 1.2 m. `0.25` keeps the box within a quarter of what was drawn, and
/// the district's median street moves by **0.01 m** — the base is 7.38 m against a ceiling of
/// 9.0 (`tests/world.rs::f003_the_street_is_narrower_than_the_houses_are_tall`).
///
/// It is also what keeps the graybox fixture out: a 28 m lot is nowhere near a quarter of a
/// 9 m house, so no fixture box is ever dressed and the eight aim tests pinned to it do not
/// move.
const DRESS_TOLERANCE: f32 = 0.25;

/// **What a HAND-PLACED cuboid may wear** — one row per `art.ron` name, with the palette key
/// the block has to carry and the full extent of that file's own `hit.min`/`hit.max` pair in
/// metres (x / y / z, `abs`, because the pair is a corner pair — see [`REMNANTS`]).
///
/// ## 🔴 It was two rows long, and that was a measurement — but only of ONE direction
///
/// ⚠️ **Read this before adding a row, and read it before deleting one.** The 2026-08-19
/// paragraph below is still true and it answers exactly one question: *given the 215 boxes
/// `maps.ron` already draws, which of them has a file in the drop?* Two. That is a fact about
/// the wall, the gatehouses and the gantries, and it has not changed.
///
/// **It says nothing about the other direction**, which is the one the hub yard needed on
/// 2026-08-26: *given a file in the drop, draw the box that IS it.* A lantern is not a box
/// somebody happened to place that turned out to be lantern-shaped; it is a box drawn at
/// `0.64 x 4.20 x 0.64` **because** that is what `a-088-laterne-strasse` measures, so the
/// collider, the anchor surface and the mesh are one thing by construction and the fit is
/// exact rather than inside a tolerance. Six rows came in that way, and every one of them is
/// a prop in `maps.ron: ashgate` that did not exist the day before.
///
/// ⚠️ **A new row must not be able to claim an OLD box.** `placed_dress_for` returns the first
/// match, so two rows whose (colour, proportion) windows overlap make the table
/// order-dependent — a crate would grow a market awning, or a barrel a uniform. That is a
/// test and not a habit: `tests/world.rs::f156_no_dressing_row_can_be_mistaken_for_another_one`
/// walks every row's own silhouette back through this function, and the eight rows were
/// checked against all 215 placed blocks before they landed (0 newly claimed). It is also why
/// the sack stack (`a-132-sackstapel`, 0.90 x 0.72 x 0.70) is **not** here: at 1.2x scale the
/// crate covers it on both footprint axes, and the two would have swapped depending on row
/// order.
///
/// ## The 2026-08-19 measurement, unchanged
///
/// The user, twice: *„die map passt aber immernoch nicht."* `FIND-132` measured that only
/// *generated* houses are dressed and that the **215 hand-placed blocks wear nothing** — the
/// grey monolith mid-district, the navy box beside a house row, the wall as a flat grey mass.
/// This table is the hop that was missing. What it is **not** is a way to dress the wall, and
/// that was measured before it was written (`docs/FINDINGS.md` FIND-134):
///
/// * the 215 placed blocks fall into **80 distinct size classes**;
/// * matching every one of them against all 279 files of the drop at a uniform scale, inside
///   [`DRESS_TOLERANCE`], leaves **two** classes where the fit is also the thing the block
///   actually is. The rest either have no candidate at all (the 700 x 15 x 43.9 m wall bands,
///   the 44 x 4 x 8 m gantry beams, the 36 x 1 x 8 m bridges) or find one that is absurd —
///   the 8 x 35 m bell tower's best fit in the whole drop is a **gas canister** at 4 %, the
///   20 x 120 x 55 m gatehouse's is a **severed arm** at 8 %. Proportion is not meaning.
/// * and the reason is structural rather than an oversight: the pack's wall vocabulary
///   (`a-095-mauersegment-*`, `a-096-mauerkrone-*`, `a-101-bresche-*`) is a **tile set**
///   authored at one module — 11.20 m wide, 120.00 m tall, 46.8..69.0 m deep — while
///   `maps.ron` builds Ashgate's wall as monolithic bands 700 / 336 / 285 m wide and 15 m
///   tall. `render::model::fit_to_class` scales a model **uniformly**: it can
///   fit one tile to one box, it cannot repeat one along it. Dressing the wall therefore means
///   re-cutting it into 11.20 m tiles in `maps.ron` — every collider in the district's
///   silhouette, `scripts/f003-ashgate.txt`'s 40 asserts and the whole anchor ladder — and
///   that is a round of its own.
///
/// ⚠️ **A placed block's box does NOT give way.** [`dress_for`] may re-plan a house's footprint
/// to the model, because a generated house is only ever a box the layout invented; a placed
/// block is gameplay geometry that the aprons, the terrain pins and `f003-ashgate.txt` are all
/// measured against. So the model is fitted to the box and never the other way round, and a
/// row is only allowed here when the overhang that leaves is something the object may
/// physically have (a market awning) or is nothing (1.5 % on a barrel).
pub const PLACED_DRESSING: [(&str, &str, [f32; 3]); 8] = [
    // ---- the two rows of 2026-08-19: models found for boxes that were already there -------
    // a-087-marktstand-zeltdach — the eight stalls of the market square, 3.0 x 2.5 x 3.0 m.
    ("market_stall", "brick_red", [4.20, 2.91, 3.64]),
    // a-132-fass-stehend — the four gas bottles in the headquarters' bay, 1.3 x 1.8 x 1.3 m.
    ("gas_drum", "olive_green", [0.66, 0.90, 0.66]),
    // ---- the six rows of 2026-08-26: boxes drawn FOR a model, in the hub yard -------------
    // a-088-laterne-strasse — a street lantern, 0.64 x 4.20 x 0.64 m, `roof_slate` iron.
    ("lamp_post", "roof_slate", [0.64, 4.20, 0.64]),
    // a-088-wegweiser — a signpost, 2.26 x 3.60 x 1.63 m. The one prop in the yard that is
    // about *information*: it stands where a player has to decide which door he is walking to.
    ("signpost", "sand_brown", [2.26, 3.60, 1.63]),
    // a-133-banner-lang — a long banner, 1.24 x 4.20 x 0.68 m. Hung flat on the garrison
    // facade, so the 1.24 m of collider it brings is against a wall nobody walks into.
    ("banner_long", "brick_red", [1.24, 4.20, 0.68]),
    // a-131-karren-intakt — a handcart, 2.40 x 1.80 x 4.80 m.
    ("hand_cart", "sand_brown", [2.40, 1.80, 4.80]),
    // a-132-kiste-klein — a crate, 0.60 m cubed.
    ("crate_small", "sand_brown", [0.60, 0.60, 0.60]),
    // a-136-npc-vanguard — a soldier, 0.68 x 1.81 x 0.62 m. **He stands and does nothing**;
    // see the `sentry` row in `art.ron` for why that is the whole specification.
    ("sentry", "olive_green", [0.68, 1.81, 0.62]),
];

/// **Which model dresses this hand-placed cuboid** — or `None`, which is the answer for 203 of
/// the 215 (see [`PLACED_DRESSING`] for why, and `docs/FINDINGS.md` FIND-134 for the numbers).
///
/// Three conditions, the same three [`dress_for`] uses minus the one that cannot apply:
///
/// 1. **`art.ron` has to say the name comes out of a file.** A `Primitive` row is a name with
///    no model behind it, and dressing against one would hide the cuboid
///    (`render::model::hide_the_primitive_under_a_model`) and draw nothing in its place — an
///    invisible solid wall. This is what keeps the whole feature *one line of RON*.
/// 2. **The block has to carry the row's palette key.** Size alone is not identity: a 3.0 x
///    2.5 x 3.0 m box is a market stall here because it is `brick_red` on the market square,
///    and the day somebody places a stone one of the same size it should not silently grow a
///    canvas awning.
/// 3. **The model, scaled uniformly to the box's height, has to fit both footprint axes
///    within [`DRESS_TOLERANCE`]** — and unlike a house, the box does not move to meet it.
pub fn placed_dress_for(data: &GameData, size_m: Vec3, color_key: &str) -> Option<&'static str> {
    for (name, wants_color, authored_m) in PLACED_DRESSING {
        if wants_color != color_key || authored_m[1] <= f32::EPSILON || size_m.y <= 0.0 {
            continue;
        }
        if !matches!(data.model(name).map(|m| &m.source), Some(ModelSource::Gltf(_))) {
            continue;
        }
        let scale = size_m.y / authored_m[1];
        let (fit_x, fit_z) = (authored_m[0] * scale, authored_m[2] * scale);
        if (fit_x - size_m.x).abs() > DRESS_TOLERANCE * size_m.x
            || (fit_z - size_m.z).abs() > DRESS_TOLERANCE * size_m.z
        {
            continue;
        }
        return Some(name);
    }
    None
}

/// A planned cuboid, **before** it is an entity.
///
/// The plan is separate from the spawning so that `tests/world.rs` can generate the city
/// twice and compare it value by value without building two apps — determinism is the
/// property you lose most cheaply and hunt down most expensively.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockPlan {
    /// `block_<i>` for a placed cuboid, `house_<lot>` for a generated one. The lot is the
    /// **number of the grid cell**, not the order of spawning: a gap in the names is an
    /// unbuilt cell and not a lost entity.
    pub name: String,
    /// World center in meters.
    pub center_m: Vec3,
    /// **Full** edge length in meters, the way `maps.ron` and [`Block`] carry it.
    pub size_m: Vec3,
    pub color: [f32; 3],
    pub anchorable: bool,
    pub solid: bool,
    /// **Which logical model dresses this cuboid** — a key into `art.ron: models`, or `None`
    /// for a block that is nothing but its box.
    ///
    /// The plan carries the *name* and never a file (`art.ron` is the only place a file name
    /// is written down), and it carries it because there is otherwise no entity to hang one
    /// on: six of the eight registry rows have a model in the pack and nothing in the game
    /// that spawns them. `size_m` of a dressed block **is** the model's own silhouette at the
    /// scale it is drawn at, so the collider, the anchor surface and the mesh are one box and
    /// not three.
    pub model: Option<&'static str>,
}

impl BlockPlan {
    fn half_size_m(&self) -> Vec3 {
        self.size_m * 0.5
    }

    /// The **only** place where a planned cuboid turns into an entity.
    fn spawn(&self, commands: &mut Commands) {
        let mut e = commands.spawn((
            Name::new(self.name.clone()),
            // What `render` sees: full edge.
            Block { size: self.size_m, color: self.color },
            // What the spatial index sees: half edge.
            Body { half_size_m: self.half_size_m(), mask: mask_from(self.solid, self.anchorable) },
            // What avian sees: the full edge again (see the module header).
            RigidBody::Static,
            Collider::cuboid(self.size_m.x, self.size_m.y, self.size_m.z),
            Transform::from_translation(self.center_m),
        ));
        if self.anchorable {
            e.insert(AnchorSurface);
        }
        // **And the model, if this box wears one** — the hop the district waited a day for.
        //
        // `ModelName` lives in `shared/` since 2026-08-19 for exactly this: `render` reads it
        // and spawns the glTF scene, `world` writes it, and neither needs an edge to the
        // other (`docs/architecture.md`, and `src/shared/anchors.rs` carries the argument).
        //
        // `height_m` is the box's **own** height and not a class figure out of `scale.ron`:
        // a house is planned at the size its model has (`dress_for`), a remnant is planned at
        // the size its remnant has (`remnant_for`), so `render::model::fit_to_class` brings
        // the file to exactly the collider it is standing in. `cortex_height_m` stays `None`
        // — a house does not die.
        if let Some(model) = self.model {
            e.insert(ModelName {
                name: model.to_string(),
                cortex_height_m: None,
                height_m: Some(self.size_m.y),
                // **And the floor of the box, because the box is positioned by its CENTRE.**
                // Every model in the pack stands on its own origin (`hit.min.y = 0` on all
                // eleven house files and all fourteen remnants), so hanging one on an entity
                // that sits at `center_m` put the building's feet on the box's middle and it
                // floated by half its own height — 5.75 m for `a-083-fachwerkhaus-gross`, which
                // is exactly what the user reported on 2026-08-19. The collider does NOT move:
                // it is what `world::index` and every raycast in the game read, and moving it
                // would move the world. What moves is the drawing *and its anchors together*
                // (`shared::ModelName::feet_y_m`).
                feet_y_m: Some(-self.size_m.y * 0.5),
            });
        }
    }
}

/// Builds the map out of `maps.ron: current` at `Startup`.
///
/// Replaces the blocks that were hard-wired in `world/mod.rs` until 2026-08-09. The first
/// entries in `maps.ron` are exactly those blocks — so that the rebuild is provably
/// **behavior-identical** and not "looks good too".
pub fn build_map(mut commands: Commands, data: Res<GameData>) {
    let Some(map) = data.current_map() else {
        // Loud, not silent: an empty world looks exactly like a render bug (§9d).
        panic!(
            "maps.ron: current = {:?} is not listed under `maps` — there would be no world, \
             and that looks like a render bug",
            data.maps.current
        );
    };

    let plan = plan_blocks(&data, map);
    let anchorable = plan.iter().filter(|r| r.anchorable).count();
    for block in plan.iter() {
        block.spawn(&mut commands);
    }

    info!(
        "map {:?}: {} blocks built ({} placed, {} generated), {anchorable} of them anchorable",
        map.name,
        plan.len(),
        map.blocks.len(),
        plan.len() - map.blocks.len(),
    );
}

/// What is to be built — **without** Bevy, without `Commands`, without side effects.
///
/// Order: first the placed blocks in file order, then the layout in lot order. Both are
/// ordered and neither is a `HashMap` — a city that looks different depending on iteration
/// order is a desync over the network.
pub fn plan_blocks(data: &GameData, map: &Map) -> Vec<BlockPlan> {
    let mut plan: Vec<BlockPlan> = placed_blocks(data, map);
    let placed = plan.len();

    let r = &map.layout;
    let rng = Rng::new(map.seed);
    let g = LayoutGrid::of(map);
    let (period_m, nx, nz, start_x, start_z) = (g.period_m, g.nx, g.nz, g.start_x, g.start_z);

    // The ground the district stands on, and the stairs that lead up it. It has to be planned
    // **before** the houses: every house is raised onto the terrace of its own cell, and the
    // veto against the placed blocks is taken over the raised box.
    let ground = plan_terrain(data, map, &plan[..placed], &rng, &g);
    plan.extend(ground.pads);
    let field = ground.field;

    // The tall class — the answer to `Q-036`, and the one height in this generator that does
    // not come out of `layout`'s own band. It is looked up **once**, and loudly: a key that is
    // not in `scale.ron` is a district that silently loses its skyline.
    let tall_height_m = *data
        .scale
        .architecture
        .heights_m
        .get(&r.tall_height_key)
        .unwrap_or_else(|| {
            panic!(
                "maps.ron: layout.tall_height_key = {:?} is not in scale.ron: \
                 architecture.heights_m",
                r.tall_height_key
            )
        });

    for iz in 0..nz {
        for ix in 0..nx {
            // The number of the LOT, not of the house. It is the `tick` for the rng:
            // whoever adds a block to `maps.ron` does not thereby shift the heights of
            // every house that follows.
            let lot = (iz * nx + ix) as u64;
            // The ring is shifted off its grid position, and that is the only tool an
            // unrotatable world has against a checkerboard: the street between two cells is
            // `street_m + jitter(right) - jitter(left)` and therefore a different width at
            // every crossing. `2 * cell_jitter_m < street_m` keeps it from closing.
            let (jitter_x, jitter_z) = match &r.perimeter {
                None => (0.0, 0.0),
                Some(p) => (
                    rng.range(lot, STREAM_JITTER_X, -p.cell_jitter_m, p.cell_jitter_m),
                    rng.range(lot, STREAM_JITTER_Z, -p.cell_jitter_m, p.cell_jitter_m),
                ),
            };
            let center_x = start_x + ix as f32 * period_m + r.lot_m * 0.5 + jitter_x;
            let center_z = start_z + iz as f32 * period_m + r.lot_m * 0.5 + jitter_z;

            // One draw per CELL, not per house: a block is built or it is an open yard.
            // Were this per house, `density` would punch holes into the ring, and a hole in
            // a closed block perimeter is exactly the thing this layout exists to remove.
            if !rng.chance(lot, STREAM_BUILT, r.density) {
                continue;
            }

            // Suburb or old town — the two shapes a cell can have. The `None` arm is
            // deliberately still the old one, box for box and draw for draw, because the
            // graybox is the fixture eight tests in `tests/vector_aiming.rs` are pinned to.
            // **One base per CELL, not per house**, and that is a decision the field forced.
            // A ring is eight to twelve row houses with party walls; founded individually on a
            // continuous field they step against each other by a rise at every join, and a
            // party wall with a 0.25 m lip in it is not a party wall. So the whole ring — the
            // lot plus the half street its ring may jitter into — is founded on the **lowest**
            // ground it covers, exactly as it used to be founded on the one terrace of its
            // cell. Measured 2026-08-29: per house it cost the district 19 more broken
            // frontages (37.6 % -> 40.3 %) and `f003_the_street_is_narrower_than_the_houses_
            // are_tall` says so.
            let cell_base_m = ground_under(
                &field,
                map,
                Rect {
                    x0: center_x - period_m * 0.5,
                    x1: center_x + period_m * 0.5,
                    z0: center_z - period_m * 0.5,
                    z1: center_z + period_m * 0.5,
                },
            );

            let footprints: Vec<Footprint> = match &r.perimeter {
                None => vec![Footprint {
                    center_x,
                    center_z,
                    size_x: r.lot_m,
                    size_z: r.lot_m,
                    // A graybox lot is not a row house: it is one box in the middle of its
                    // cell with street on all four sides. `-z` is named as its front so the
                    // field has a value, and nothing hangs on it — a 28 m box is nowhere near
                    // a quarter of a 9 m model, so `dress_for` never dresses one.
                    frontage_along_x: true,
                    facade_dir: -1.0,
                    slot_m: r.lot_m,
                    depth_room_m: r.lot_m,
                }],
                Some(p) => perimeter_houses(center_x, center_z, r.lot_m, p, &rng, lot),
            };
            let single = r.perimeter.is_none();

            // **The block's own level**, one draw for the whole cell. The house then only
            // varies inside `house_spread_m` around it. One draw per house over the whole
            // window is white noise, and white noise averages out over a hundred metres —
            // which is exactly how the first version came out flat.
            //
            // With `perimeter: None` this collapses, term for term, to the single draw the
            // graybox made before: `min + fraction * (max - min)`.
            let band_m = r.max_height_m - r.min_height_m;
            let (level_m, spread_m) = match &r.perimeter {
                None => (r.min_height_m, band_m),
                Some(p) => {
                    let spread = p.house_spread_m.min(band_m);
                    (
                        rng.range(lot, STREAM_LEVEL, r.min_height_m, r.max_height_m - spread),
                        spread,
                    )
                }
            };
            let roof = r.perimeter.as_ref().and_then(|p| p.roof.as_ref());
            // How far this district has fallen. `None` is a district that still stands.
            let damage = r.damage.as_ref();

            for (i, f) in footprints.iter().enumerate() {
                // `lot` itself for the single box, so the graybox draws exactly what it drew
                // before this function learned about rings.
                let tick = if single { lot } else { lot * TICKS_PER_LOT + i as u64 };

                if in_clear_radius(
                    f.center_x,
                    f.center_z,
                    f.size_x * 0.5,
                    f.size_z * 0.5,
                    r.clear_radius_m,
                ) {
                    continue;
                }

                // The **ridge** — the whole house, the number the height window is about. The
                // roof is then cut out of it downward, so a district that grows roofs does
                // not thereby grow taller than `scale.ron` allows it to be.
                //
                // ⚠️ Unless the house is drawn into the **tall class**, in which case the
                // ridge is one figure out of `scale.ron` and the band does not apply. That is
                // `Q-036`'s gap, filled: without it the whole district lives in 5 m of spread
                // and reads as one flat top from anywhere (`maps.ron: layout.tall_fraction`).
                let ridge_m = if rng.chance(tick, STREAM_TALL, r.tall_fraction) {
                    tall_height_m
                } else {
                    level_m + rng.range(tick, STREAM_HEIGHT, 0.0, spread_m)
                };
                // **Is there a model for this house, and does it fit?** The answer decides
                // the box, not the other way round: a dressed house *is* its model's
                // silhouette, and it grows no cuboid roof because the model brings a roof,
                // a chimney and a gable of its own (`dach_first`, `schornstein_kopf`,
                // `giebel_v` — the node list of `a-083-fachwerkhaus-*`).
                //
                // `None` here is the normal answer today and not a failure: `art.ron` ships
                // every row as `Primitive`, and [`dress_for`] refuses to dress a name that
                // resolves to no file. Flipping those three rows to `Gltf(...)` is what
                // turns this district into half-timbered houses — one line each, no code.
                // **Did this house survive the fall of Ashgate?**
                //
                // Asked BEFORE the dressing, and that order is the whole reason the ruin and
                // the house landed in one round: a fallen house wears a different kit, keeps
                // a different footprint and grows no roof, so dressing it first would be work
                // thrown away twice (`docs/NEXT.md` §2C).
                //
                // `None` — from `maps.ron: layout.damage` — is a district that still stands,
                // and it is what the graybox fixture says.
                let remnant =
                    damage.and_then(|k| remnant_for(data, k, map, f, ridge_m, &rng, lot, tick));
                let dress = if remnant.is_some() { None } else { dress_for(data, f, ridge_m) };
                let rise_m = match dress {
                    // The model is the whole house, ridge included, so nothing is cut out of
                    // it downward — `fit_to_class` scales the file to `ridge_m` and its own
                    // roof lands where the cuboid roof would have ended.
                    Some(_) => 0.0,
                    None => roof.map_or(0.0, |k| {
                        ridge_m
                            * rng.range(tick, STREAM_RISE, k.min_rise_fraction, k.max_rise_fraction)
                    }),
                };
                let height_m = match &remnant {
                    // A remnant is what is LEFT of the house, so its height is its own and
                    // never the ridge the intact house would have had.
                    Some(k) => k.height_m,
                    None => ridge_m - rise_m,
                };
                // The footprint the block finally gets. Undressed it is what was drawn;
                // dressed it is the model's own, **and the street-facing face stays where it
                // was** — the frontage line is what two rounds of work went into, and a house
                // that gives back depth gives it back to its courtyard.
                let (size_x, size_z, center_x, center_z) = match &remnant {
                    Some(k) => (k.size_x, k.size_z, k.center_x, k.center_z),
                    None => match dress {
                    None => (f.size_x, f.size_z, f.center_x, f.center_z),
                    Some((_, front_m, depth_m)) => {
                        let (drawn_depth, center_depth) = if f.frontage_along_x {
                            (f.size_z, f.center_z)
                        } else {
                            (f.size_x, f.center_x)
                        };
                        let facade = center_depth + f.facade_dir * drawn_depth * 0.5;
                        let moved = facade - f.facade_dir * depth_m * 0.5;
                        if f.frontage_along_x {
                            (front_m, depth_m, f.center_x, moved)
                        } else {
                            (depth_m, front_m, moved, f.center_z)
                        }
                    }
                    },
                };
                // A house stands ON the ground, and since 2026-08-29 the ground is a
                // continuous field: **the lowest cell its whole cell covers** (see
                // `cell_base_m` above), so the uphill corner is cut into the slope and the
                // downhill one never stands on air (`FIND-134` §3B). `base_m` is the only line
                // in this loop the terrain touches; everything below it is measured from there.
                let base_m = cell_base_m;
                let center_m = Vec3::new(center_x, base_m + height_m * 0.5, center_z);
                let size_m = Vec3::new(size_x, height_m, size_z);

                // What is explicitly placed wins (`maps.ron`). Only the placed blocks are
                // tested against: two layout houses can never overlap, the ring geometry
                // and the street see to that.
                //
                // ⚠️ The test is **per house, not per cell**. A cell-wide test was what made
                // the aprons cost 43 m of frontage each: the main street's 48 m apron
                // clipped the corner of a block and deleted the whole ring, so the street
                // measured 134 m facade to facade. Per house, an apron deletes exactly the
                // houses standing on it and the rest of the ring keeps the street closed.
                //
                // ⚠️ And it is tested against the house **including its roof**: the aprons
                // exist because a 14 m gallery hangs over the ground, and a roof cap that
                // slipped under one would be a tagged surface with stone over it
                // (`tests/world.rs::f003_no_anchorable_block_has_another_block_sitting_on_its_roof_centre`).
                //
                // ⚠️ And it is taken over the box from **y = 0 to the raised ridge**, not over
                // the raised box. Both halves are load bearing: the aprons are 0.3 m of paving
                // whose top edge sticks 0.05 m out of the ground, and a house lifted onto a
                // terrace would fly straight over them — the galleries would get their roofs
                // back. Reaching up to the raised ridge is the other direction, and it costs
                // nothing today because everything a house could grow into up there (the 56 m
                // gantry beams, the 60 m gallery) starts far above 11.5 + 3.6 m.
                //
                // ⚠️ And it is taken over the **dressed** footprint since 2026-08-18, not over
                // the drawn one. A dressing may move a wall out by up to `DRESS_TOLERANCE`,
                // and a veto taken before that would let a house grow into an apron it was
                // just measured clear of.
                // ⚠️ And over the remnant's own height when the house has fallen: a 3 m
                // stump vetoed against the 11 m box it used to be would delete a mound the
                // aprons never touch.
                let veto_m = base_m + if remnant.is_some() { height_m } else { ridge_m };
                let ridge_center = Vec3::new(center_x, veto_m * 0.5, center_z);
                let ridge_half = Vec3::new(size_x * 0.5, veto_m * 0.5, size_z * 0.5);
                // ⚠️ And a placed block whose **top is below the ground** vetoes by its
                // FOOTPRINT alone, at any height. That is the channel, and it is the one case
                // the y test above cannot see: a house box runs from `y = 0` upward, the
                // channel floor's top is at **y = -4.00**, so the floor slab has never vetoed
                // anything and only the quays — 0.4 m proud of the paving — ever did. That
                // worked while `maps.ron` could argue *"a 12 x 11 m row house does not fit into
                // a 10 m gap"*, and it stopped working the moment the user got the river he
                // asked for (*„das wasser ist auch VIEL zu klein"*): at 30 m of channel,
                // **34 of 456 houses** were generated standing in mid-air over the water
                // (`docs/FINDINGS.md` FIND-220). The rule that does not depend on the width is
                // the one `CellRole::Hole` already states for the ground beside it: **a house
                // may not stand where there is no ground.**
                let blocked = plan[..placed].iter().any(|g| {
                    let half = g.half_size_m();
                    if g.center_m.y + half.y < 0.0 {
                        let d = (ridge_center - g.center_m).abs();
                        return d.x < ridge_half.x + half.x && d.z < ridge_half.z + half.z;
                    }
                    overlaps(ridge_center, ridge_half, g.center_m, half)
                });
                if blocked {
                    continue;
                }

                let colors = &r.colors;
                let color = match &remnant {
                    // A remnant is ash, not brick. Only ever seen where the ruin kit is
                    // switched off in `art.ron` — the mesh covers the box otherwise — but
                    // that is exactly the build a model-less screenshot shows.
                    Some(k) => &k.color,
                    None => colors
                        .get(rng.index(tick, STREAM_COLOR, colors.len()))
                        .unwrap_or_else(|| {
                            panic!(
                                "maps.ron: layout.colors is empty — every house would be \
                                 colorless"
                            )
                        }),
                };
                // **The roof carries the same bit as its house.** Two different answers for
                // one building is the FIND-059 bug from the other side: the player aims at
                // the highest thing he sees, and that is the cap.
                let anchorable = rng.chance(tick, STREAM_ANCHORABLE, r.anchorable_fraction);

                plan.push(BlockPlan {
                    // **The name says what it is**, and six tests in `tests/world.rs` read
                    // it: the residential band, the skyline, the street width and the roof
                    // rule are all about houses that still stand, and a stump measured as a
                    // house would fail every one of them for the right reason at the wrong
                    // address.
                    name: match &remnant {
                        Some(k) => format!("{}_{lot}_{i}", k.prefix),
                        None if single => format!("house_{lot}"),
                        None => format!("house_{lot}_{i}"),
                    },
                    center_m,
                    size_m,
                    color: color_of(data, color),
                    anchorable,
                    // A house stops you. That is mechanics and not a tuning question — there
                    // is deliberately no `solid_fraction` in `maps.ron`.
                    solid: true,
                    model: match &remnant {
                        Some(k) => k.model,
                        None => dress.map(|(name, _, _)| name),
                    },
                });

                // **A dressed house grows no cap.** Two roofs on one building is the FIND-059
                // bug in its loudest form: the model's own ridge and a stack of stone lids
                // through it.
                if let (Some(k), None, None) = (roof, dress, &remnant) {
                    // The cap is pulled in on all four sides; what is left over is the ledge
                    // the roof reads by and the strip you can still stand on.
                    //
                    // Since 2026-08-13 it is `roof_steps` caps and not one, each pulled in one
                    // notch further than the one below it. A flat lid on every house is a
                    // roofscape of equal rectangles, and *„aktuell kann man es noch nicht
                    // erkennen"* is as much about that as about the ground: from the air a
                    // stepped pitch has a ridge line and a lid has none. The rise is still cut
                    // **out of** the house — the steps divide it, they do not add to it.
                    let steps = r.roof_steps.max(1);
                    let keep_top = 1.0 - 2.0 * k.inset_fraction * steps as f32;
                    assert!(
                        keep_top > 0.0,
                        "maps.ron: layout.perimeter.roof.inset_fraction = {} over \
                         layout.roof_steps = {steps} leaves the top cap no extent — a roof \
                         with a negative edge is an invisible collider",
                        k.inset_fraction
                    );
                    let each_m = rise_m / steps as f32;
                    // ⚠️ **The steps are pulled in across the SHORT axis only, and that is what
                    // makes them a roof.** Pulled in on all four sides they are a stepped
                    // pyramid — measured on `docs/images/f003-roofscape.png` on 2026-08-13:
                    // 900 square ziggurats, which is further from *„wie die echte stadt"* than
                    // the flat lid they replaced. A real roof is a **ridge**: flush with the
                    // gable walls at its ends, stepping in over the eaves. So the long side of
                    // the footprint keeps its full extent and the ridge runs along it, which on
                    // a row house is along the street front — exactly where a gable belongs.
                    let ridge_along_x = f.size_x >= f.size_z;
                    for s in 0..steps {
                        let keep = 1.0 - 2.0 * k.inset_fraction * (s + 1) as f32;
                        let (keep_x, keep_z) =
                            if ridge_along_x { (1.0, keep) } else { (keep, 1.0) };
                        plan.push(BlockPlan {
                            // The bottom step keeps the old name, so that every reader of
                            // `roof_<lot>_<i>` — and every image caption — still finds the
                            // same block; the steps above it are suffixed.
                            name: if s == 0 {
                                format!("roof_{lot}_{i}")
                            } else {
                                format!("roof_{lot}_{i}_{s}")
                            },
                            center_m: Vec3::new(
                                f.center_x,
                                base_m + height_m + each_m * (s as f32 + 0.5),
                                f.center_z,
                            ),
                            size_m: Vec3::new(f.size_x * keep_x, each_m, f.size_z * keep_z),
                            color: color_of(data, &k.color),
                            anchorable,
                            solid: true,
                            model: None,
                        });
                    }
                }
            }
        }
    }

    plan
}

/// The cuboids `maps.ron` places by hand, 1:1 and in file order.
fn placed_blocks(data: &GameData, map: &Map) -> Vec<BlockPlan> {
    map.blocks
        .iter()
        .enumerate()
        .map(|(i, k)| BlockPlan {
            name: format!("block_{i}"),
            center_m: Vec3::new(k.center_m.0, k.center_m.1, k.center_m.2),
            size_m: Vec3::new(k.size_m.0, k.size_m.1, k.size_m.2),
            color: color_of(data, &k.color),
            anchorable: k.anchorable,
            solid: k.solid,
            // **And since 2026-08-19 a hand-placed cuboid may wear one too.** The name still
            // does not stand in `maps.ron` — `blocks` has no `model:` field and `src/data/`
            // was not this round's to change — so the match is by the two things the file
            // already says about a block, its size and its palette key
            // ([`placed_dress_for`]). 12 of the 215 come out dressed and 203 do not, and the
            // reason the number is that small is measured rather than shrugged at:
            // [`PLACED_DRESSING`].
            model: placed_dress_for(
                data,
                Vec3::new(k.size_m.0, k.size_m.1, k.size_m.2),
                &k.color,
            ),
        })
        .collect()
}

/// The ground of a map, planned on its own — the same call [`plan_blocks`] makes, without the
/// city on top of it. `tests/world.rs` measures the relief on this.
pub fn terrain_of(data: &GameData, map: &Map) -> (LayoutGrid, PlannedGround) {
    let g = LayoutGrid::of(map);
    let placed = placed_blocks(data, map);
    let ground = plan_terrain(data, map, &placed, &Rng::new(map.seed), &g);
    (g, ground)
}

/// The grid every generated thing is placed on — lots, and the terrain cells over them.
///
/// One struct rather than five loose locals, because `tests/world.rs` has to be able to ask
/// **which cell a house belongs to** without re-deriving the arithmetic. A test that
/// re-derives it is a test that agrees with itself.
#[derive(Clone, Copy, Debug)]
pub struct LayoutGrid {
    /// Block pitch: `lot_m + street_m`.
    pub period_m: f32,
    pub nx: u32,
    pub nz: u32,
    /// World coordinate of the near edge of lot 0.
    pub start_x: f32,
    pub start_z: f32,
}

impl LayoutGrid {
    pub fn of(map: &Map) -> Self {
        let r = &map.layout;
        let period_m = r.lot_m + r.street_m;
        let nx = lot_count(map.size_m.0, period_m);
        let nz = lot_count(map.size_m.1, period_m);
        Self {
            period_m,
            nx,
            nz,
            // The built-up area is centered on its own extent, not on `nx * period`: no
            // street follows behind the last block, and without this correction the whole
            // city would sit half a street width off center.
            start_x: -(nx as f32 * period_m - r.street_m) * 0.5,
            start_z: -(nz as f32 * period_m - r.street_m) * 0.5,
        }
    }

}

/// The stepped ground and the field it was cut from.
pub struct PlannedGround {
    /// The levels, so that the houses can be raised onto the terrace of their own cell.
    pub field: TerrainField,
    /// The terrace tops and their flights of stairs, in cell order.
    pub pads: Vec<BlockPlan>,
}

/// The lowest ground a footprint stands on, in metres — what a house is founded on.
///
/// The cell grid is the map's own, not the layout's: at `cell_m` 5.0 against a block pitch of
/// 42 a house covers three cells and a ring covers nine, so "the cell this lot belongs to" is
/// no longer a question with an answer. `min` and not `max` is the decision — see
/// [`TerrainField::lowest_over`].
fn ground_under(field: &TerrainField, map: &Map, foot: Rect) -> f32 {
    let cell_m = map.terrain.cell_m;
    if cell_m <= 0.0 || field.nx() == 0 {
        return 0.0;
    }
    let (origin_x, origin_z) = (-map.size_m.0 * 0.5, -map.size_m.1 * 0.5);
    let ix0 = ((foot.x0 - origin_x) / cell_m).floor() as i32;
    let ix1 = ((foot.x1 - origin_x) / cell_m).ceil() as i32 - 1;
    let iz0 = ((foot.z0 - origin_z) / cell_m).floor() as i32;
    let iz1 = ((foot.z1 - origin_z) / cell_m).ceil() as i32 - 1;
    field.lowest_over(ix0, ix1.max(ix0), iz0, iz1.max(iz0))
}

/// A footprint, looking down. The terrain question is always a footprint question.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x0: f32,
    x1: f32,
    z0: f32,
    z1: f32,
}

impl Rect {
    fn of(center: Vec3, half: Vec3) -> Self {
        Self {
            x0: center.x - half.x,
            x1: center.x + half.x,
            z0: center.z - half.z,
            z1: center.z + half.z,
        }
    }

    fn grown(self, by_m: f32) -> Self {
        Self {
            x0: self.x0 - by_m,
            x1: self.x1 + by_m,
            z0: self.z0 - by_m,
            z1: self.z1 + by_m,
        }
    }

    fn hits(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.z0 < other.z1 && other.z0 < self.z1
    }

    /// A slab thinner than a centimetre is a z-fighting sliver, not a terrace.
    fn real(&self) -> bool {
        self.x1 - self.x0 > 0.01 && self.z1 - self.z0 > 0.01
    }
}

/// `a` without `b`, as up to four rectangles.
///
/// **This world has no subtraction** (`docs/FINDINGS.md` FIND-056) — what it has is leaving
/// pieces out, and that is exactly what this does. It is the reason a bell tower or a tree does
/// not have to flatten the whole 42 m cell it happens to stand in: the terrace is cut around
/// its foot, and the pillar itself fills the hole, because it is solid from the ground up.
fn without(a: Rect, b: Rect) -> Vec<Rect> {
    if !a.hits(&b) {
        return vec![a];
    }
    let (z0, z1) = (b.z0.max(a.z0), b.z1.min(a.z1));
    [
        Rect { z1: b.z0.min(a.z1), ..a },
        Rect { z0: b.z1.max(a.z0), ..a },
        Rect { x1: b.x0.min(a.x1), z0, z1, ..a },
        Rect { x0: b.x1.max(a.x0), z0, z1, ..a },
    ]
    .into_iter()
    .filter(Rect::real)
    .collect()
}

/// What is left of one slab once every pillar has been cut out of it.
fn cut(rect: Rect, pillars: &[Rect]) -> Vec<Rect> {
    let mut out = vec![rect];
    for p in pillars {
        out = out.into_iter().flat_map(|r| without(r, *p)).collect();
    }
    out
}

/// The ground the district stands on — **the whole of it, in one place.**
///
/// The user, 2026-08-29, having played the terraced version this replaced: *„auch die
/// verschiedenen hoehen passen nicht! das soll grass sein und nicht so wie jetzt! und nicht
/// verschiedene hardcoded stufen sondern wirklich terrain! und deutlich hoeher und niedriger
/// als jetzt!"*
///
/// ## What went, and what it took with it
///
/// The terraces were `levels x step_m` on a 42 m cell: six plateaus, a 1.50 m cliff between
/// two of them, and a flight of `step_m / stair_rise_m` risers cut into every falling edge to
/// make the cliff walkable. Every one of those numbers existed **for the cliff** — and there
/// is no cliff any more, so `levels`, `step_m`, `stair_rise_m`, `stair_tread_m`, `stair_color`
/// and `pillar_gap_m` are gone with it, along with the assert that `step_m` be a whole
/// multiple of the riser and the one that `cell_m` be a whole multiple of the block pitch.
///
/// **What survives is `FIND-091`'s actual finding, in its new form:** whatever the slope, the
/// player has to be able to walk up it. For terraces that was a flight of stairs; for a field
/// it is one number, `rise_m`, and it is measured (`shared::terrain`, and FIND-214).
///
/// ## Three decisions, and each is why a whole class of bug cannot happen
///
/// 1. **The grid covers the whole map, not the layout.** `size_m` is asserted to be a whole
///    multiple of `cell_m`. The ground *is* the terrain now — the two 700 m slabs that used to
///    be Ashgate's floor are deleted from `maps.ron`, because a slab at `y = -0.1` over a
///    valley at `-10` is a lid on it.
/// 2. **Every hand-placed block is one of four things to the ground, and it never moves.**
///    All 240 of them stand exactly where the file puts them; what changes is what the ground
///    under them is allowed to do (`shared::CellRole`). That is the whole answer to "what
///    happens to the hand-placed blocks": nothing.
///    * **sky** — bottom already above `max_rise_m + door_height_m` (the 56 m gantry beams,
///      the 60 m gallery, the crown at 120 m). The ground can never meet it. Ignored.
///    * **hole** — top below the base plane: the canal floor. No ground at all is emitted
///      over it, which is how the channel stays a channel in a world that cannot subtract.
///    * **floor** — top at or below `paving_top_m`: the paving, the streets, the market
///      square. The ground may climb over it, exactly as a terrace always could, but may not
///      sink away underneath and leave it in the air.
///    * **pin** — everything else that stands on the ground: the quay walls, the wall
///      courses, the gate towers, the stalls, the church, the trees. Its cell is 0.
/// 3. **One block per merged rectangle, not one per cell.** 19 600 cells of 5 m would be
///    19 600 draw calls; greedily merging neighbours of equal height into the largest
///    axis-aligned rectangle that fits brings Ashgate to ~6 300. That is the only reason a
///    5 m cell is affordable at all, and `docs/lessons/performance.md` rule 6 is why it is
///    measured in `tests/world.rs` and not hoped for.
pub fn plan_terrain(
    data: &GameData,
    map: &Map,
    placed: &[BlockPlan],
    rng: &Rng,
    _g: &LayoutGrid,
) -> PlannedGround {
    let t = &map.terrain;
    let r = &map.layout;
    let flat_map = t.amplitude_m.is_empty() || t.rise_m <= 0.0 || t.cell_m <= 0.0;
    let (ncx, ncz) = if flat_map {
        (0, 0)
    } else {
        (
            (map.size_m.0 / t.cell_m).round().max(0.0) as u32,
            (map.size_m.1 / t.cell_m).round().max(0.0) as u32,
        )
    };
    let (origin_x, origin_z) = (-map.size_m.0 * 0.5, -map.size_m.1 * 0.5);

    // The footprint of one cell, boundary to boundary.
    let rect = |cx: u32, cz: u32| Rect {
        x0: origin_x + cx as f32 * t.cell_m,
        x1: origin_x + (cx + 1) as f32 * t.cell_m,
        z0: origin_z + cz as f32 * t.cell_m,
        z1: origin_z + (cz + 1) as f32 * t.cell_m,
    };

    // The four classes. `sky_m` is the line above which nothing the ground does is visible,
    // and it comes out of `max_rise_m` plus the user's own door height from `scale.ron` — not
    // out of a margin invented here.
    let sky_m = t.max_rise_m + data.scale.reference.door_height_m;
    let mut holes: Vec<Rect> = Vec::new();
    let mut floors: Vec<Rect> = Vec::new();
    let mut pins: Vec<Rect> = Vec::new();
    for b in placed {
        let half = b.half_size_m();
        let (bottom, top) = (b.center_m.y - half.y, b.center_m.y + half.y);
        let foot = Rect::of(b.center_m, half);
        if top < 0.0 {
            holes.push(foot);
        } else if top <= t.paving_top_m {
            floors.push(foot);
        } else if bottom >= sky_m {
            continue;
        } else {
            pins.push(foot);
        }
    }

    let field = TerrainField::new(
        ncx,
        ncz,
        t.cell_m,
        t.rise_m,
        &t.amplitude_m,
        &t.wavelength_m,
        rng,
        STREAM_TERRAIN,
        |cx, cz| {
            let cell = rect(cx, cz);
            if holes.iter().any(|h| cell.hits(h)) {
                return CellRole::Hole;
            }
            // The spawn. Measured to the **edge** of the cell, like `clear_radius_m`, and for
            // the same reason: otherwise the clear space would depend on the cell size.
            let dx = cell.x0.max(-cell.x1).max(0.0);
            let dz = cell.z0.max(-cell.z1).max(0.0);
            if dx * dx + dz * dz < t.flat_radius_m * t.flat_radius_m {
                return CellRole::Pin;
            }
            if pins.iter().any(|p| cell.hits(p)) {
                return CellRole::Pin;
            }
            if floors.iter().any(|f| cell.hits(f)) {
                return CellRole::Floor;
            }
            CellRole::Free
        },
    );

    let mut pads: Vec<BlockPlan> = Vec::new();
    if flat_map {
        return PlannedGround { field, pads };
    }

    assert!(
        (ncx as f32 * t.cell_m - map.size_m.0).abs() < 1e-3
            && (ncz as f32 * t.cell_m - map.size_m.1).abs() < 1e-3,
        "maps.ron: size_m = {:?} is not a whole multiple of terrain.cell_m = {} — then the \
         last row of the map has no ground under it at all",
        map.size_m,
        t.cell_m
    );
    assert!(
        t.flat_radius_m >= r.clear_radius_m,
        "maps.ron: terrain.flat_radius_m = {} is below layout.clear_radius_m = {} — then the \
         ground moves inside the space the layout keeps free around the spawn",
        t.flat_radius_m,
        r.clear_radius_m
    );
    assert_eq!(
        t.amplitude_m.len(),
        t.wavelength_m.len(),
        "maps.ron: terrain has {} amplitudes and {} wavelengths — an octave is a pair",
        t.amplitude_m.len(),
        t.wavelength_m.len()
    );
    assert!(
        !t.colors.is_empty(),
        "maps.ron: terrain.colors is empty — the ground would have no colour at all"
    );

    // ## The invariant, asserted and not trusted
    //
    // One rise is the tallest riser a player can walk over, and the field promises that no two
    // neighbours differ by more than one. That promise is the difference between terrain and a
    // wall with a texture, so it is checked here where the map can be named — `docs/BUGS.md`
    // B-018 is what it looks like when it fails silently.
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for cz in 0..ncz as i32 {
        for cx in 0..ncx as i32 {
            let h = field.height_at(cx, cz);
            lo = lo.min(h);
            hi = hi.max(h);
            for (dx, dz) in [(1, 0), (0, 1)] {
                let d = (field.step_at(cx, cz) - field.step_at(cx + dx, cz + dz)).abs();
                assert!(
                    d <= 1,
                    "map {:?}: cell ({cx},{cz}) stands {:.2} m over its neighbour \
                     ({},{}) — one rise is {} m and that is already the whole walking budget",
                    map.name,
                    d as f32 * t.rise_m,
                    cx + dx,
                    cz + dz,
                    t.rise_m
                );
            }
        }
    }
    assert!(
        hi <= t.max_rise_m + 1e-3,
        "map {:?}: the field reached {hi:.2} m but terrain.max_rise_m says {} — every block \
         between the two is sky to the classifier and would have been ignored",
        map.name,
        t.max_rise_m
    );
    assert!(
        t.base_m < lo - 1e-3,
        "maps.ron: terrain.base_m = {} is not below the lowest cell ({lo:.2} m) — a ground \
         block would be inside out",
        t.base_m
    );

    // ## The merge, and why it is greedy and not clever
    //
    // Row by row: take the first cell nobody has claimed, run right while the height is the
    // same, then run down while the whole span still matches. It is the standard greedy mesh
    // and it is exact about what it emits — every cell that is not a hole ends up in exactly
    // one rectangle, which is what `tests/world.rs` counts. A smarter partition would win a
    // few per cent and cost the property that makes it checkable.
    let colors: Vec<[f32; 3]> = t.colors.iter().map(|k| color_of(data, k)).collect();
    let span = (hi - lo).max(1e-3);
    let mut claimed = vec![false; (ncx as usize) * (ncz as usize)];
    let at = |cx: u32, cz: u32| (cz as usize) * (ncx as usize) + cx as usize;
    for cz in 0..ncz {
        let mut cx = 0;
        while cx < ncx {
            if claimed[at(cx, cz)] || field.is_hole(cx as i32, cz as i32) {
                cx += 1;
                continue;
            }
            let step = field.step_at(cx as i32, cz as i32);
            let mut cx1 = cx;
            while cx1 + 1 < ncx
                && !claimed[at(cx1 + 1, cz)]
                && !field.is_hole((cx1 + 1) as i32, cz as i32)
                && field.step_at((cx1 + 1) as i32, cz as i32) == step
            {
                cx1 += 1;
            }
            let mut cz1 = cz;
            while cz1 + 1 < ncz
                && (cx..=cx1).all(|c| {
                    !claimed[at(c, cz1 + 1)]
                        && !field.is_hole(c as i32, (cz1 + 1) as i32)
                        && field.step_at(c as i32, (cz1 + 1) as i32) == step
                })
            {
                cz1 += 1;
            }
            for z in cz..=cz1 {
                for x in cx..=cx1 {
                    claimed[at(x, z)] = true;
                }
            }
            let top_m = step as f32 * t.rise_m;
            let band = (((top_m - lo) / span) * colors.len() as f32) as usize;
            let piece = Rect {
                x0: origin_x + cx as f32 * t.cell_m,
                x1: origin_x + (cx1 + 1) as f32 * t.cell_m,
                z0: origin_z + cz as f32 * t.cell_m,
                z1: origin_z + (cz1 + 1) as f32 * t.cell_m,
            };
            pads.push(BlockPlan {
                name: format!("ground_{cx}_{cz}"),
                center_m: Vec3::new(
                    (piece.x0 + piece.x1) * 0.5,
                    (t.base_m + top_m) * 0.5,
                    (piece.z0 + piece.z1) * 0.5,
                ),
                size_m: Vec3::new(
                    piece.x1 - piece.x0,
                    top_m - t.base_m,
                    piece.z1 - piece.z0,
                ),
                color: colors[band.min(colors.len() - 1)],
                // Ground, and since 2026-08-12 the ground of this district holds a hook —
                // *„man soll überall seinen haken inmachen können! auch an den boden"*.
                // Anything else here would be an unlisted exception and
                // `tests/world.rs::f003_an_unanchorable_block_is_a_listed_exception...`
                // would say so.
                anchorable: true,
                solid: true,
                // The ground wears no model. The pack has street and stair pieces
                // (`a-085-strasse-*`), and a hillside is not one of them.
                model: None,
            });
            cx = cx1 + 1;
        }
    }

    PlannedGround { field, pads }
}

/// One house's ground plan inside a cell — everything but its height.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Footprint {
    center_x: f32,
    center_z: f32,
    size_x: f32,
    size_z: f32,
    /// Does the **frontage** run along x? Then z is the depth. The four wings of a ring
    /// alternate, and a model has a front — so this is not derivable from `size_x >= size_z`,
    /// which is false for every house in a 9 m slot that is 11 m deep.
    frontage_along_x: bool,
    /// Which way the **street** lies along the depth axis, `-1.0` or `+1.0`. The facade is
    /// `center + facade_dir * size * 0.5` on that axis, and it is the one edge of a house that
    /// may not move: everything about a street is measured facade to facade.
    facade_dir: f32,
    /// The house's own share of its run, across the frontage axis — gap included.
    ///
    /// **The envelope a dressing may never leave.** Two neighbours in a run are centred in
    /// adjacent slots, so a box inside its own slot cannot reach into the next one however
    /// wide the model turns out to be. Without this the ±`DRESS_TOLERANCE` window would let a
    /// house eat the alley beside it and stand inside its neighbour.
    slot_m: f32,
    /// How far back from the facade the house may reach before it is in the courtyard.
    depth_room_m: f32,
}

/// **Which model dresses this house, and at what footprint** — or `None`, which is the normal
/// answer.
///
/// Three things have to be true at once, and each of them removes a whole class of wrong:
///
/// 1. **`art.ron` has to say the name comes out of a file.** A row that is `Primitive` is a
///    name with no model behind it, and dressing a house against one would cost it its cuboid
///    roof and rewrite its footprint in exchange for nothing. This is what makes the whole
///    feature *one line of RON*: flip `house_small`/`house_town`/`house_large` to `Gltf(...)`
///    and the district dresses itself; flip them back and it is cuboids again, no code moved.
/// 2. **The model may only be scaled uniformly**, so the box gives way instead — but by at
///    most `DRESS_TOLERANCE`. The alternative, "take the model that fits inside the box", was
///    measured and is worse in the way that matters: the model then averages 0.66 of the
///    collider it stands in, the hook catches on air a metre off the wall, and the district's
///    median street opens from 7.38 m to 8.55.
/// 3. **The result has to stay inside the house's own slot and its own depth**, so no dressing
///    can reach a neighbour or cross the frontage line.
///
/// Among everything that passes, the **largest** is taken: two classes that both fit differ
/// only in how much of the lot the town actually covers.
fn dress_for(data: &GameData, f: &Footprint, ridge_m: f32) -> Option<(&'static str, f32, f32)> {
    let (front_m, depth_m) = if f.frontage_along_x {
        (f.size_x, f.size_z)
    } else {
        (f.size_z, f.size_x)
    };
    let mut best: Option<(&'static str, f32, f32)> = None;
    for (name, authored_m) in DRESSING {
        if authored_m[1] <= f32::EPSILON || ridge_m <= 0.0 {
            continue;
        }
        if !matches!(data.model(name).map(|m| &m.source), Some(ModelSource::Gltf(_))) {
            continue;
        }
        let scale = ridge_m / authored_m[1];
        let (fit_front_m, fit_depth_m) = (authored_m[0] * scale, authored_m[2] * scale);
        if (fit_front_m - front_m).abs() > DRESS_TOLERANCE * front_m
            || (fit_depth_m - depth_m).abs() > DRESS_TOLERANCE * depth_m
        {
            continue;
        }
        if fit_front_m > f.slot_m || fit_depth_m > f.depth_room_m {
            continue;
        }
        if best.is_none_or(|(_, bf, bd)| fit_front_m * fit_depth_m > bf * bd) {
            best = Some((name, fit_front_m, fit_depth_m));
        }
    }
    best
}

/// **What is left of a house that did not survive** — the plan of one remnant, before it is a
/// [`BlockPlan`].
///
/// It is a separate type and not four loose returns because all five numbers have to be
/// decided together: the model is picked by what fits the lot, and the box that is finally
/// built **is** that model's silhouette (the same invariant a dressed house has — collider,
/// anchor surface and mesh are one box and not three).
#[derive(Clone, Debug, PartialEq)]
struct Remnant {
    /// `ruin` or `rubble` — the first half of the block's name, and what six tests in
    /// `tests/world.rs` tell a stump from a house by.
    prefix: &'static str,
    size_x: f32,
    size_z: f32,
    center_x: f32,
    center_z: f32,
    height_m: f32,
    model: Option<&'static str>,
    color: String,
}

/// **How far gone this one house is**, 0..1 — the gradient plus two draws
/// (`maps.ron: layout.damage`, and the argument for the shape is written there).
///
/// The radial term is taken from the map's own half extent, so a district that is made bigger
/// keeps its core and its edge rather than becoming uniformly ruined.
fn severity_at(k: &Damage, map: &Map, rng: &Rng, lot: u64, tick: u64, x: f32, z: f32) -> f32 {
    let half = map.size_m.0.max(map.size_m.1) * 0.5;
    let radial = if half > f32::EPSILON {
        ((x * x + z * z).sqrt() / half).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gradient = k.core_severity + (k.edge_severity - k.core_severity) * radial;
    let block = rng.range(lot, STREAM_DAMAGE_BLOCK, -k.block_spread, k.block_spread);
    let house = rng.range(tick, STREAM_DAMAGE_HOUSE, -k.house_spread, k.house_spread);
    (gradient + block + house).clamp(0.0, 1.0)
}

/// **Which remnant this house wears, and at what size** — or `None`, which means it is still
/// standing.
///
/// Three rules, and they are deliberately *not* [`dress_for`]'s three:
///
/// 1. A remnant may be **smaller** than the house it was, and usually is. `dress_for` refuses
///    a model that is more than [`DRESS_TOLERANCE`] off the drawn box because an intact house
///    has to keep the frontage line closed; a ruin that only covers half its lot is the
///    picture. So the model is scaled to the **largest** it can be inside the lot instead.
/// 2. It is never **taller** than the house was — `damage.ruin_height_fraction` of the ridge,
///    at most. Nothing in a fallen district got taller by falling.
/// 3. **The street-facing face stays where it was**, exactly as a dressed house's does, so a
///    ruined stretch still measures as the street it is. A mound is the one exception and it
///    is the point of `damage.spill_m`: it is pushed past that line into the road.
///
/// Among everything that fits, one is **drawn** rather than the largest taken (`dress_for`
/// takes the largest): eight remnants that all look like the biggest fragment that fits is a
/// row of identical stumps, and this kit exists to be irregular.
///
/// `None` for the model — with `art.ron` on `Primitive` — is a legal answer and not a
/// failure: the box is still built, at the remnant's height and in ash, and the district is
/// still fallen. That is what keeps the whole kit one line of RON per row.
#[allow(clippy::too_many_arguments)]
fn remnant_for(
    data: &GameData,
    k: &Damage,
    map: &Map,
    f: &Footprint,
    ridge_m: f32,
    rng: &Rng,
    lot: u64,
    tick: u64,
) -> Option<Remnant> {
    let severity = severity_at(k, map, rng, lot, tick, f.center_x, f.center_z);
    if severity < k.ruin_at {
        return None;
    }
    let collapsed = severity >= k.rubble_at;

    let (front_m, depth_m) = if f.frontage_along_x {
        (f.size_x, f.size_z)
    } else {
        (f.size_z, f.size_x)
    };
    // What the remnant is allowed to be. A ruin is capped by the ridge it was; a mound is
    // capped by the window `maps.ron` draws mounds out of.
    let ceiling_m = if collapsed {
        rng.range(tick, STREAM_MOUND, k.rubble_height_m.0, k.rubble_height_m.1)
    } else {
        ridge_m * k.ruin_height_fraction
    };
    let floor_m = if collapsed { 0.0 } else { k.ruin_min_height_m };
    let kit: &[(&'static str, [f32; 3])] = if collapsed { &RUBBLE_KIT } else { &RUIN_KIT };

    // Everything in the kit that fits this lot at a size worth building.
    let mut fits: Vec<(&'static str, f32, f32, f32)> = Vec::new();
    for (name, authored_m) in kit {
        let [ax, ay, az] = *authored_m;
        if ax <= f32::EPSILON || ay <= f32::EPSILON || az <= f32::EPSILON {
            continue;
        }
        if !matches!(data.model(name).map(|m| &m.source), Some(ModelSource::Gltf(_))) {
            continue;
        }
        // **Uniform, always** — a ruin stretched on one axis has fat timbers exactly the way
        // a stretched house does, and the ruin kit is the same pack.
        let scale = (front_m / ax).min(depth_m / az).min(ceiling_m / ay);
        let height_m = ay * scale;
        if height_m < floor_m || height_m <= 0.0 {
            continue;
        }
        fits.push((name, ax * scale, az * scale, height_m));
    }

    // Nothing in the kit fits, or `art.ron` has the kit switched off. The house has still
    // fallen — it is a box in ash at the remnant's height, and it grows no roof.
    let (model, front, depth, height_m) = if fits.is_empty() {
        let height_m = if collapsed {
            ceiling_m
        } else {
            // Half of what it was, floor respected: a stump, not a house.
            (ridge_m * k.ruin_height_fraction * 0.5).max(k.ruin_min_height_m).min(ridge_m)
        };
        (None, front_m, depth_m, height_m)
    } else {
        let (name, front, depth, height_m) = fits[rng.index(tick, STREAM_REMNANT, fits.len())];
        (Some(name), front, depth, height_m)
    };

    // The facade line, and what a mound does to it.
    let (drawn_depth, center_depth) = if f.frontage_along_x {
        (f.size_z, f.center_z)
    } else {
        (f.size_x, f.center_x)
    };
    let facade = center_depth + f.facade_dir * drawn_depth * 0.5;
    let spill = if collapsed { k.spill_m } else { 0.0 };
    let moved = facade - f.facade_dir * depth * 0.5 + f.facade_dir * spill;
    let (size_x, size_z, center_x, center_z) = if f.frontage_along_x {
        (front, depth, f.center_x, moved)
    } else {
        (depth, front, moved, f.center_z)
    };

    Some(Remnant {
        prefix: if collapsed { "rubble" } else { "ruin" },
        size_x,
        size_z,
        center_x,
        center_z,
        height_m,
        model,
        color: if collapsed { k.rubble_color.clone() } else { k.ruin_color.clone() },
    })
}

/// A closed block: the ring of houses around one cell's courtyard — **and no two of them the
/// same**.
///
/// Four runs, and they do **not** overlap: north and south take the full `lot_m` in x and
/// start at the street edge; west and east take only the strip between them. That much is
/// unchanged. What changed on 2026-08-12 is everything about the individual house, and it
/// changed because the version without it was judged:
///
/// > *„häuser sind alle ineinander! keine unterschiedliche höhen! [...] viel zu kompakt!"*
///
/// The first ring divided every run into houses of **exactly** the same width, gave each of
/// them **exactly** `wing_depth_m` of depth, and set them flush against the street edge with
/// a gap of **exactly** zero. Party walls are what the survey measured, and they were built
/// correctly — but eight identical boxes in a square with no gap and no relief are, from ten
/// metres up, one object. So now, out of the seed:
///
/// * the **frontage** is drawn per run, so the house widths of two facing rows differ;
/// * a **gap** is taken off each house as a fraction of its slot — sometimes 0 (the party
///   wall the survey wants), sometimes the full `gap_fraction` (an alley you can drop into);
/// * a **setback** moves the facade off the street edge, so the frontage line is ragged;
/// * the **depth** is cut behind that, so the courtyard side is ragged too.
///
/// `setback_max_m + depth_spread_m < wing_depth_m` is asserted rather than clamped: a clamp
/// would silently give every house the same minimum depth again, which is the failure this
/// function exists to avoid.
///
/// The courtyard is what is left over, `lot_m - 2 * wing_depth_m` on a side. It is a **gap**,
/// because this world has no subtraction (`docs/FINDINGS.md` FIND-056), and no setback or
/// depth cut can eat into it — a house only ever grows *shallower*, never deeper.
fn perimeter_houses(
    cx: f32,
    cz: f32,
    lot_m: f32,
    p: &Perimeter,
    rng: &Rng,
    lot: u64,
) -> Vec<Footprint> {
    let w = p.wing_depth_m;
    let court_m = lot_m - 2.0 * w;
    assert!(
        court_m > 0.0,
        "maps.ron: layout.perimeter.wing_depth_m = {w} leaves no courtyard in a {lot_m} m \
         block — the two wings would grow through each other"
    );
    assert!(
        p.setback_max_m + p.depth_spread_m < w,
        "maps.ron: layout.perimeter setback_max_m {} + depth_spread_m {} is not less than \
         wing_depth_m {w} — a house could come out with no depth at all",
        p.setback_max_m,
        p.depth_spread_m
    );
    assert!(
        (0.0..1.0).contains(&p.gap_fraction),
        "maps.ron: layout.perimeter.gap_fraction = {} is not in 0..1 — at 1.0 the house is \
         the alley",
        p.gap_fraction
    );

    let mut out: Vec<Footprint> = Vec::new();
    // The wings in a fixed order, and the order is part of the seed: house `i` of a cell is
    // the `i`-th footprint out of this loop, and that is the tick its height is drawn on.
    // 0 north, 1 south (the runs along x), 2 west, 3 east (the strip between them).
    for wing in 0..4u64 {
        let along_x = wing < 2;
        let run_m = if along_x { lot_m } else { court_m };
        // One frontage per run, not per house: a row of houses is built by one builder in one
        // decade, and the row opposite is not.
        let frontage_m = p.frontage_m
            + rng.range(
                lot * TICKS_PER_LOT + wing,
                STREAM_FRONTAGE,
                -p.frontage_spread_m * 0.5,
                p.frontage_spread_m * 0.5,
            );
        let n = runs(run_m, frontage_m);
        let slot_m = run_m / n as f32;
        for k in 0..n {
            // The tick of the house that is about to be pushed — the same formula
            // `plan_blocks` uses for its height, color and roof, and it only stays in step
            // because `out.len()` is the index this footprint will have.
            let tick = lot * TICKS_PER_LOT + out.len() as u64;
            // Centred in its slot, so half the gap falls on either side: two neighbours are
            // then `(gap_k + gap_k+1) / 2` apart and the block still ends at its own edge.
            let gap_m = slot_m * rng.range(tick, STREAM_GAP, 0.0, p.gap_fraction);
            let setback_m = rng.range(tick, STREAM_SETBACK, 0.0, p.setback_max_m);
            let depth_m = w - setback_m - rng.range(tick, STREAM_DEPTH, 0.0, p.depth_spread_m);
            let width_m = slot_m - gap_m;
            let along = (k as f32 + 0.5) * slot_m - run_m * 0.5;
            // Measured from the street edge inward, never from the courtyard outward.
            let inset = lot_m * 0.5 - setback_m - depth_m * 0.5;
            // `frontage_along_x` and `facade_dir` are what a model needs and a cuboid did
            // not: which axis the street front runs along, and which side of the box it is.
            // Wing 0 looks north (-z), 1 south (+z), 2 west (-x), 3 east (+x) — the street is
            // always outward, the courtyard always inward.
            let room_m = w - setback_m;
            out.push(match wing {
                0 => Footprint { center_x: cx + along, center_z: cz - inset, size_x: width_m, size_z: depth_m,
                                 frontage_along_x: true, facade_dir: -1.0, slot_m, depth_room_m: room_m },
                1 => Footprint { center_x: cx + along, center_z: cz + inset, size_x: width_m, size_z: depth_m,
                                 frontage_along_x: true, facade_dir: 1.0, slot_m, depth_room_m: room_m },
                2 => Footprint { center_x: cx - inset, center_z: cz + along, size_x: depth_m, size_z: width_m,
                                 frontage_along_x: false, facade_dir: -1.0, slot_m, depth_room_m: room_m },
                _ => Footprint { center_x: cx + inset, center_z: cz + along, size_x: depth_m, size_z: width_m,
                                 frontage_along_x: false, facade_dir: 1.0, slot_m, depth_room_m: room_m },
            });
        }
    }

    assert!(
        (out.len() as u64) < TICKS_PER_LOT,
        "a block of {lot_m} m at a {} m frontage holds {} houses, and only {} rng ticks \
         belong to a cell — two cells would draw the same height",
        p.frontage_m,
        out.len(),
        TICKS_PER_LOT
    );
    out
}

/// How many whole houses a run of `len_m` is divided into. Never zero: a 6 m stub is one
/// narrow house, not a gap in the block.
fn runs(len_m: f32, frontage_m: f32) -> u32 {
    if !(len_m.is_finite() && frontage_m.is_finite()) || frontage_m <= 0.0 {
        return 1;
    }
    ((len_m / frontage_m).round() as i64).max(1) as u32
}

/// How many layout lots fit along one edge. `0` means: no layout.
fn lot_count(edge_m: f32, period_m: f32) -> u32 {
    if !(edge_m.is_finite() && period_m.is_finite()) || period_m <= 0.0 || edge_m <= 0.0 {
        return 0;
    }
    (edge_m / period_m).floor() as u32
}

/// Whether a block comes closer to the origin than `clear_radius_m`.
///
/// Measured from the origin to the **edge** of the block, not to its center: otherwise the
/// clear space depends on the block size, and `clear_radius_m` would stop being a promise.
///
/// The two halves are separate since the ring layout: a row house is 12 m of frontage and
/// 11 m of depth, and one number for both would clear the wrong amount on one of the axes.
fn in_clear_radius(
    center_x: f32,
    center_z: f32,
    half_x_m: f32,
    half_z_m: f32,
    clear_radius_m: f32,
) -> bool {
    let dx = (center_x.abs() - half_x_m).max(0.0);
    let dz = (center_z.abs() - half_z_m).max(0.0);
    dx * dx + dz * dz < clear_radius_m * clear_radius_m
}

/// Strict overlap of two axis-aligned cuboids — **touching does not count.**
///
/// That is exactly what the ground slab slips past: a house with its bottom edge at y = 0
/// and a slab with its top edge at y = 0 have `distance == sum` on the Y axis, and `<` is
/// false. Both sides compute the same sum out of the same floats, so the result is exactly
/// equal and not "nearly".
fn overlaps(a_center: Vec3, a_half: Vec3, b_center: Vec3, b_half: Vec3) -> bool {
    let distance = (a_center - b_center).abs();
    let sum = a_half + b_half;
    distance.x < sum.x && distance.y < sum.y && distance.z < sum.z
}

/// A color out of the one palette — or an abort naming the color that is missing.
///
/// No silent substitute: otherwise one of the three signal colors eventually slips into the
/// scenery (`docs/conventions.md`).
fn color_of(data: &GameData, name: &str) -> [f32; 3] {
    data.color(name).unwrap_or_else(|| {
        panic!("maps.ron: color {name:?} is not listed in `palette`")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f003_a_house_stands_on_the_ground_slab_not_in_it() {
        // The reason the layout builds anything at all: the ground slab is the first placed
        // block and covers the whole map. Were `overlaps` not strict, the city would be
        // empty — and that without a single error message.
        let slab_center = Vec3::new(0.0, -0.1, 0.0);
        let slab_half = Vec3::new(200.0, 0.1, 200.0);
        for height_m in [4.5f32, 7.3, 11.5, 35.0] {
            let house_center = Vec3::new(0.0, height_m * 0.5, 0.0);
            let house_half = Vec3::new(14.0, height_m * 0.5, 14.0);
            assert!(
                !overlaps(house_center, house_half, slab_center, slab_half),
                "a house {height_m} m tall supposedly sits inside the ground slab"
            );
        }
        // And a cellar really is inside it.
        assert!(overlaps(
            Vec3::new(0.0, -0.1, 0.0),
            Vec3::splat(2.0),
            slab_center,
            slab_half
        ));
    }

    #[test]
    fn f003_the_clear_radius_measures_to_the_edge_not_the_center() {
        // A block whose center is 30 m away but whose edge is 16 m: it is in the way.
        assert!(in_clear_radius(30.0, 0.0, 14.0, 14.0, 24.0), "edge at 16 m, radius 24 m");
        assert!(!in_clear_radius(40.0, 0.0, 14.0, 14.0, 24.0), "edge at 26 m, radius 24 m");
        // Diagonally the real distance counts, not the larger of the two axes.
        assert!(!in_clear_radius(35.0, 35.0, 14.0, 14.0, 24.0), "edge diagonally at 29.7 m");
        // A row house is wide and shallow, and the two halves are NOT interchangeable. The
        // same 12 x 11 m footprint, once with its frontage along x and once along z:
        assert!(!in_clear_radius(0.0, 29.5, 6.0, 5.5, 24.0), "edge in z at 24.0 m, radius 24");
        assert!(in_clear_radius(0.0, 29.5, 5.5, 6.0, 24.0), "edge in z at 23.5 m, radius 24");
    }

    /// The ring the district is built from, as of 2026-08-12.
    fn ashgate_ring() -> Perimeter {
        Perimeter {
            frontage_m: 12.0,
            wing_depth_m: 11.0,
            frontage_spread_m: 5.0,
            gap_fraction: 0.22,
            setback_max_m: 1.6,
            depth_spread_m: 3.2,
            cell_jitter_m: 2.0,
            house_spread_m: 2.4,
            roof: None,
        }
    }

    #[test]
    fn f003_a_closed_block_stays_inside_its_lot_and_keeps_its_courtyard() {
        // The two invariants no amount of irregularity may break: nothing leaves the cell (or
        // it grows into the street and there is no street left), and nothing reaches into the
        // courtyard (or the two facing wings meet in the middle of the block).
        let p = ashgate_ring();
        let rng = Rng::new(3405691582);
        let court_half = 36.0 * 0.5 - p.wing_depth_m; // 7.0

        for lot in 0..64u64 {
            let houses = perimeter_houses(0.0, 0.0, 36.0, &p, &rng, lot);
            assert!(houses.len() >= 6, "lot {lot}: {} houses in a ring", houses.len());
            for f in &houses {
                let out_x = f.center_x.abs() + f.size_x * 0.5;
                let out_z = f.center_z.abs() + f.size_z * 0.5;
                assert!(
                    out_x <= 18.0 + 1e-4 && out_z <= 18.0 + 1e-4,
                    "lot {lot}: {f:?} sticks out of its 36 m cell"
                );
                let clear =
                    (f.center_x.abs() - f.size_x * 0.5).max(f.center_z.abs() - f.size_z * 0.5);
                assert!(clear >= court_half - 1e-4, "lot {lot}: {f:?} eats into the courtyard");
            }
            // And no two houses of a ring stand in the same place — two cuboids in one spot
            // are a z-fight and a doubled collider, and neither shows up in the picture.
            for (i, a) in houses.iter().enumerate() {
                for b in &houses[i + 1..] {
                    let dx = (a.center_x - b.center_x).abs();
                    let dz = (a.center_z - b.center_z).abs();
                    assert!(
                        !(dx < (a.size_x + b.size_x) * 0.5 - 1e-4
                            && dz < (a.size_z + b.size_z) * 0.5 - 1e-4),
                        "lot {lot}: {a:?} and {b:?} stand in the same place"
                    );
                }
            }
        }
    }

    #[test]
    fn f003_no_two_houses_of_a_ring_are_the_same_box() {
        // ★ The user's verdict, as a test: *„häuser sind alle ineinander! keine
        // unterschiedliche höhen!"*. Red again the moment somebody takes the per-house draws
        // back out — then every footprint of a run is the same rectangle, which is precisely
        // what one merged mass with a flat top is made of.
        let p = ashgate_ring();
        let rng = Rng::new(3405691582);
        let houses = perimeter_houses(0.0, 0.0, 36.0, &p, &rng, 7);

        let widths: Vec<f32> = houses.iter().map(|f| f.size_x.max(f.size_z)).collect();
        let depths: Vec<f32> = houses.iter().map(|f| f.size_x.min(f.size_z)).collect();
        let spread = |v: &[f32]| {
            v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min)
        };
        assert!(spread(&widths) > 0.5, "every house is {:.2} m wide", widths[0]);
        assert!(spread(&depths) > 0.5, "every house is {:.2} m deep", depths[0]);

        // And there is at least one real alley in the ring — a gap you can see from the
        // street, not a party wall.
        let mut north: Vec<&Footprint> = houses.iter().filter(|f| f.center_z < -7.0).collect();
        north.sort_by(|a, b| a.center_x.total_cmp(&b.center_x));
        let widest = north
            .windows(2)
            .map(|w| (w[1].center_x - w[1].size_x * 0.5) - (w[0].center_x + w[0].size_x * 0.5))
            .fold(0.0f32, f32::max);
        assert!(widest > 0.3, "the widest gap in the street front is {widest:.2} m");
    }

    #[test]
    fn f003_a_run_is_divided_into_whole_houses() {
        assert_eq!(runs(36.0, 12.0), 3);
        assert_eq!(runs(14.0, 12.0), 1);
        // Never zero: a stub is one narrow house, never a hole in the perimeter.
        assert_eq!(runs(4.0, 12.0), 1);
        assert_eq!(runs(36.0, 0.0), 1);
        assert_eq!(runs(f32::NAN, 12.0), 1);
    }

    #[test]
    fn f003_the_lot_count_drops_the_trailing_street() {
        // 400 m at 28 + 7: eleven blocks are 385 m, twelve would be 420 m.
        assert_eq!(lot_count(400.0, 35.0), 11);
        assert_eq!(lot_count(35.0, 35.0), 1);
        assert_eq!(lot_count(34.0, 35.0), 0);
        // No crash on nonsense, just no layout.
        assert_eq!(lot_count(400.0, 0.0), 0);
        assert_eq!(lot_count(f32::NAN, 35.0), 0);
    }
}
