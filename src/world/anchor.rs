//! **The anchor points** — `F-021`, `F-022`, `F-023`, `F-031a`.
//!
//! ## Why this file had to exist
//!
//! `F-003` gave every box in the district an [`AnchorSurface`](crate::shared::AnchorSurface)
//! and the whole district is tagged (*„überall! ohne ausnahmen!"*). That is **level one** of
//! the design's anchor system: *which surfaces hold a rope at all*. It is a boolean per
//! cuboid, and a boolean per cuboid cannot answer any of the questions the design asks next:
//!
//! * *which* point on that 700 m wall does `Q` take me to (`F-023`),
//! * what does the HUD draw a ring on (`F-026`),
//! * which twelve of the four hundred in view survive the density cap (`F-027`),
//! * and is this map's coverage even complete (`F-031a`).
//!
//! **Level two is a list of discrete, individually addressable points with metadata**, and
//! that is [`AnchorField`]. It is built once, at `Startup`, out of the same
//! [`BlockPlan`](super::map::BlockPlan) list that spawns the city — so a point cannot describe
//! a building that was not built, which is the one way a hand-authored point list always goes
//! wrong.
//!
//! ## Two sources, one list
//!
//! 1. **Procedural, out of the geometry** ([`generate_points`], `F-022`). Roof corners, roof
//!    edges, and — for anything taller than [`COURSE_MIN_HEIGHT_M`] — a **ladder of facade
//!    courses** every [`COURSE_RISE_M`] up the outside. That last one is the important one:
//!    it is the `hook.gesims_15..105` ladder the pack authors into its wall tiles
//!    (`docs/FINDINGS.md` FIND-134), delivered **without** re-cutting a single collider.
//!    Ashgate's wall is monolithic 700 m bands and a tile cannot be repeated along one; a
//!    generated point does not care how the box was cut.
//! 2. **Named, out of the models** ([`AnchorField::adopt_named`], `F-021`). Everything a
//!    `.glb` carries under `hook.<anything>` (`shared::anchors::HOOK_PREFIX`) — `hook.traufe`,
//!    `hook.first`, `hook.krone`, `hook.spitze`. 439 of them across 144 files were dropped at
//!    load until 2026-08-18 and have been read but unused ever since. Here they land in the
//!    same list as the generated ones, carrying their name, so a consumer never has to know
//!    which of the two made a point.
//!
//! ## ⚠️ The numbers in this file stand in Rust, and that is not where they belong
//!
//! Rule 2 says a game value lives in RON. These do not, for the same reason
//! [`DRESSING`](super::map::DRESSING) does not: the schema of a map lives in `src/data/`, and
//! this round did not own that file. **The patch is reported, not applied** — the target is a
//! `layout.anchors: (...)` block in `assets/data/maps.ron` with these seven keys and no
//! `serde(default)` on any of them. Until then they are constants with this warning on them.
//!
//! ## Rule 6, and why the field is a column grid
//!
//! The hook reaches 200 m. A cubic grid at the index's 8 m cell would make one candidate
//! query walk 51³ = 132 651 cells to answer a question about what is in front of your nose —
//! the exact thing rule 6 forbids. The world is 700 × 700 m and **130 m tall**: it is flat.
//! So the field buckets by **column** (x, z) at [`FIELD_CELL_M`], 22 × 22 = 484 buckets for
//! Ashgate, and a 200 m query touches 169 of them. Height falls out of the distance test.

use bevy::prelude::*;

use super::map::BlockPlan;
use crate::shared::{ModelAnchors, HOOK_PREFIX};

/// **Which planned block this entity is** — the bridge from an entity back into
/// [`AnchorField::blocks`].
///
/// It carries a `u32` and not an `Entity` on purpose (`docs/multiplayer.md` rule 5): the plan
/// index is the same number on every machine that loads the same map, an `Entity` is not.
/// Written by [`super::map::build_map`], read only inside this domain.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorBlock(pub u32);

/// How far in from the true corner a corner point sits, in metres.
///
/// A point exactly on the edge is half over the void: the rope bites, the hook slides off the
/// chamfer of the model that dresses the box, and the player reads it as a miss. Half a metre
/// in is on the roof.
pub const CORNER_INSET_M: f32 = 0.5;

/// Along a roof edge, one point every this many metres, in metres.
///
/// A generated house has ~12 m of frontage (`maps.ron: layout`, survey median 13.1 m), so on
/// the common case this is exactly the two corners and nothing between them; it only starts
/// producing points on the long stuff — quays, gantries, the wall.
pub const EDGE_SPACING_M: f32 = 12.0;

/// The vertical pitch of the facade ladder, in metres.
///
/// **15 m is not a taste**: Ashgate's wall is built one course per 15 m (`maps.ron`, eight
/// bands from y = 0 to y = 120), and the pack names its cornices `hook.gesims_15` through
/// `hook.gesims_105` — a cornice every 15 m up a 120 m wall (`src/shared/anchors.rs`). The
/// generated ladder and the authored one therefore land on the same rungs.
pub const COURSE_RISE_M: f32 = 15.0;

/// Only a facade at least this tall gets a ladder, in metres.
///
/// Below it the roof is inside one rope-length of the ground and the ladder buys nothing but
/// markers.
pub const COURSE_MIN_HEIGHT_M: f32 = 30.0;

/// Along a facade course, one point every this many metres, in metres.
///
/// Wider than [`EDGE_SPACING_M`] on purpose: a facade rung is a fallback for a rope that is
/// *under* the roofline, not a place you aim at, and 700 m of wall at 12 m would be 58 points
/// per rung per band.
pub const COURSE_SPACING_M: f32 = 24.0;

/// `F-022`'s **Mindestabstand**: two points closer than this are one point, in metres.
///
/// Applied per block in generation order, so which of the two survives is deterministic and
/// does not depend on iteration order (`docs/multiplayer.md`: a city that differs by
/// iteration order is a desync).
pub const MIN_SPACING_M: f32 = 3.0;

/// A plate thinner than this is a floor, not a feature, in metres.
///
/// The ground slab is 0.2 m and the streets are 0.3 m: 700 × 700 m of tagged surface that
/// would otherwise generate tens of thousands of edge points along the map border. A gantry
/// is 2–3 m and keeps its points.
pub const MIN_THICKNESS_M: f32 = 0.5;

/// The upper bound on what one block may contribute.
///
/// The wall's bands are 700 × 15 × 40 m. Without a cap one band alone is ~230 course points
/// times eight rungs, and the field stops being a list of places you aim at.
pub const MAX_POINTS_PER_BLOCK: usize = 48;

/// Edge length of one bucket of the column grid, in metres. See the module header.
pub const FIELD_CELL_M: f32 = 32.0;

/// What kind of feature a point sits on — `F-021`'s *Typ*.
///
/// The order is the **quality order**, best first, and [`AnchorKind::quality`] reads off it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorKind {
    /// The model brought it under `hook.<name>` — a modeller put it there on purpose, so it
    /// outranks anything computed.
    Named,
    /// A roof corner, inset by [`CORNER_INSET_M`]. Two edges meet, so a rope that overshoots
    /// one still lands on the other.
    Corner,
    /// A point along a roof edge between the corners.
    Edge,
    /// A rung of the facade ladder — vertical face, not a roof.
    Course,
}

impl AnchorKind {
    /// `F-021`'s **Guete-Basiswert**, 0..1, before any distance or angle weighting.
    ///
    /// ⚠️ Tuning numbers in Rust — see the module header.
    pub fn quality(self) -> f32 {
        match self {
            AnchorKind::Named => 1.0,
            AnchorKind::Corner => 0.85,
            AnchorKind::Edge => 0.7,
            AnchorKind::Course => 0.5,
        }
    }
}

/// One discrete anchor point — `F-021`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorPoint {
    /// Where it is, in world metres.
    pub position_m: Vec3,
    /// Which way the surface it sits on faces. `+Y` for a roof, `±X`/`±Z` for a facade rung.
    /// A hook approaching against the normal is coming from inside the building.
    pub normal: Vec3,
    pub kind: AnchorKind,
    /// [`AnchorKind::quality`], carried on the point so a consumer never has to re-derive it
    /// and a later per-point override has somewhere to live.
    pub quality: f32,
    /// Which block it belongs to — an index into [`AnchorField::blocks`], **not** an
    /// `Entity` (`docs/multiplayer.md` rule 5).
    pub block: u32,
    /// The `hook.*` name the model gave it, without the prefix — `traufe`, `first`,
    /// `gesims_45`. `None` for everything [`generate_points`] made up.
    pub name: Option<String>,
    /// `F-021`'s *dynamisch*: does this point ride a moving carrier (a titan's shoulder, a
    /// cart, a wall lift)? Everything in this field today is static; `F-029` already rides
    /// titans through `HookState::Anchored { body, local_m }` and does not go through here.
    pub dynamic: bool,
}

/// Which side of the camera axis a candidate is on — `F-023`.
///
/// *"Q bedient ausschliesslich die linke Menge, E ausschliesslich die rechte."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere {
    Left,
    Right,
}

/// One point that survived the cone — `F-023`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Index into [`AnchorField::points`].
    pub index: u32,
    pub distance_m: f32,
    pub side: Hemisphere,
    /// The ranking number, high is better. Base quality, falling off with distance and with
    /// the angle off the camera axis — `F-027`'s *Bewertungsfunktion*.
    pub score: f32,
}

/// `F-031a`'s report — what a map's point list is wrong about.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnchorReport {
    pub points: usize,
    /// Points that ended up inside a solid block that is not their own. A rope that reaches
    /// one is a rope through a wall.
    pub buried: usize,
    /// Pairs closer together than [`MIN_SPACING_M`] that survived the per-block thinning —
    /// i.e. two *different* blocks putting a point in the same spot.
    pub clustered: usize,
    /// Points whose normal is not a unit axis. A normal that is not a direction is a bug in
    /// generation, not in the map.
    pub bad_normal: usize,
    /// Buckets of the column grid that hold no point although a block stands in them —
    /// `F-031a`'s *Loecher in der Abdeckung*.
    pub holes: usize,
}

impl AnchorReport {
    /// A map ships when this is true — `F-031a`: *„Kein Release einer Map ohne fehlerfreien
    /// Validierungsbericht."*
    pub fn is_clean(&self) -> bool {
        self.points > 0 && self.buried == 0 && self.clustered == 0 && self.bad_normal == 0
    }
}

/// **The map's anchor point list** — `F-021`'s *vollstaendige Ankerpunktliste*.
///
/// One writer: [`super::map::build_map`] inserts it, [`AnchorField::adopt_named`] adds the
/// model-borne points to it as the models finish loading. Both live in this domain and
/// nothing outside `world` writes it (`docs/architecture.md`).
#[derive(Resource, Debug, Clone, Default)]
pub struct AnchorField {
    points: Vec<AnchorPoint>,
    /// The name of every block that has an index in [`AnchorPoint::block`], in plan order.
    blocks: Vec<String>,
    /// Half-extents of those blocks, parallel to `blocks` — kept so [`AnchorField::validate`]
    /// can ask "is this point inside somebody else's box" without a second pass over the plan.
    boxes: Vec<(Vec3, Vec3)>,
    /// Column buckets, `columns_x * columns_z` of them, row-major in z.
    cells: Vec<Vec<u32>>,
    /// The same buckets for **blocks** — every column a block's footprint touches lists it.
    ///
    /// This is what makes [`AnchorField::validate`]'s "is this point inside somebody else's
    /// box" question rule-6 shaped: without it the check is 25 000 points x 2 871 blocks, and
    /// a validation pass that takes a minute is a validation pass nobody runs.
    block_cells: Vec<Vec<u32>>,
    origin_m: Vec2,
    columns_x: usize,
    columns_z: usize,
}

impl AnchorField {
    /// Builds the field out of the plan the city was built from — `F-021` + `F-022`.
    pub fn from_plan(plan: &[BlockPlan], size_m: (f32, f32)) -> Self {
        let mut field = AnchorField {
            origin_m: Vec2::new(-size_m.0 * 0.5, -size_m.1 * 0.5),
            columns_x: (size_m.0 / FIELD_CELL_M).ceil() as usize + 1,
            columns_z: (size_m.1 / FIELD_CELL_M).ceil() as usize + 1,
            ..default()
        };
        field.cells = vec![Vec::new(); field.columns_x * field.columns_z];
        field.block_cells = vec![Vec::new(); field.columns_x * field.columns_z];
        field.blocks = plan.iter().map(|b| b.name.clone()).collect();
        field.boxes = plan.iter().map(|b| (b.center_m, b.size_m * 0.5)).collect();
        for (i, block) in plan.iter().enumerate() {
            let half = block.size_m * 0.5;
            for cell in field.footprint_cells(block.center_m, half) {
                field.block_cells[cell].push(i as u32);
            }
        }
        for (i, block) in plan.iter().enumerate() {
            for point in generate_points(block, i as u32) {
                // **A point inside somebody else's box is unreachable and never enters the
                // field.** `F-031a` calls these *unerreichbare Punkte* and asks for a report;
                // the report is the second line of defence, this is the first. Ashgate had
                // **416** of them on 2026-08-25 — a roof that a taller neighbour's collider
                // swallows, a ruin standing in a rubble mound — and every one of them would
                // have been a marker on a wall with a rope that goes through it.
                //
                // ⚠️ It is checked against the *blocks*, which is why `block_cells` is filled
                // before this loop and not after it.
                if field.inside_another_block(point.position_m, point.block) {
                    continue;
                }
                field.push(point);
            }
        }
        field
    }

    /// Is this position strictly inside a block that is not `own`?
    ///
    /// Rule 6: only the blocks of the position's own column are asked.
    fn inside_another_block(&self, position_m: Vec3, own: u32) -> bool {
        let Some(cell) = self.cell_of(position_m) else {
            return false;
        };
        self.block_cells[cell].iter().any(|&b| {
            if b == own {
                return false;
            }
            let (center, half) = self.boxes[b as usize];
            let d = (position_m - center).abs();
            d.x < half.x - 0.01 && d.y < half.y - 0.01 && d.z < half.z - 0.01
        })
    }

    /// The one way a point enters the field — **and the one place `F-022`'s Mindestabstand is
    /// enforced across block boundaries.**
    ///
    /// Two houses that share a party wall put their corner points a metre apart, and a marker
    /// pair a metre apart is one marker with a shadow. So the rule is global, not per block.
    ///
    /// ⚠️ **A [`AnchorKind::Named`] point is exempt.** A modeller put `hook.traufe` where it is
    /// on purpose, and dropping an authored point because a computed one got there first would
    /// make the pack's 439 anchors lose to geometry that was invented to stand in for them.
    fn push(&mut self, point: AnchorPoint) {
        if point.kind != AnchorKind::Named && !self.near(point.position_m, MIN_SPACING_M).is_empty()
        {
            return;
        }
        let index = self.points.len() as u32;
        if let Some(cell) = self.cell_of(point.position_m) {
            self.cells[cell].push(index);
        }
        self.points.push(point);
    }

    /// Every column bucket a box's footprint touches.
    fn footprint_cells(&self, center_m: Vec3, half_m: Vec3) -> Vec<usize> {
        let lo_x = ((center_m.x - half_m.x - self.origin_m.x) / FIELD_CELL_M).floor().max(0.0);
        let lo_z = ((center_m.z - half_m.z - self.origin_m.y) / FIELD_CELL_M).floor().max(0.0);
        let hi_x = ((center_m.x + half_m.x - self.origin_m.x) / FIELD_CELL_M).floor().max(0.0);
        let hi_z = ((center_m.z + half_m.z - self.origin_m.y) / FIELD_CELL_M).floor().max(0.0);
        let mut out = Vec::new();
        if lo_x as usize >= self.columns_x || lo_z as usize >= self.columns_z {
            return out;
        }
        let hi_x = (hi_x as usize).min(self.columns_x - 1);
        let hi_z = (hi_z as usize).min(self.columns_z - 1);
        for z in lo_z as usize..=hi_z {
            for x in lo_x as usize..=hi_x {
                out.push(z * self.columns_x + x);
            }
        }
        out
    }

    fn cell_of(&self, p: Vec3) -> Option<usize> {
        let x = ((p.x - self.origin_m.x) / FIELD_CELL_M).floor();
        let z = ((p.z - self.origin_m.y) / FIELD_CELL_M).floor();
        if x < 0.0 || z < 0.0 {
            return None;
        }
        let (x, z) = (x as usize, z as usize);
        (x < self.columns_x && z < self.columns_z).then(|| z * self.columns_x + x)
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// How many of the field's points a **model** brought, not [`generate_points`].
    ///
    /// The pack ships 439 `hook.*` points across 144 files and the loader has read them since
    /// 2026-08-18 without anything consuming them (`docs/FINDINGS.md` FIND-116). This is the
    /// number that says how many of them the game actually holds.
    pub fn named(&self) -> usize {
        self.points.iter().filter(|p| p.kind == AnchorKind::Named).count()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// `F-021`'s *einzeln inspizierbar* — one point, by its address.
    pub fn get(&self, index: u32) -> Option<&AnchorPoint> {
        self.points.get(index as usize)
    }

    /// The name of the block a point belongs to.
    pub fn block_name(&self, point: &AnchorPoint) -> Option<&str> {
        self.blocks.get(point.block as usize).map(String::as_str)
    }

    pub fn points(&self) -> &[AnchorPoint] {
        &self.points
    }

    pub fn blocks(&self) -> &[String] {
        &self.blocks
    }

    /// How many buckets of the column grid hold at least one point. Used by the coverage half
    /// of [`AnchorField::validate`] and by the perf argument in the module header.
    pub fn filled_cells(&self) -> usize {
        self.cells.iter().filter(|c| !c.is_empty()).count()
    }

    /// Every point within `radius_m` of `p` — **without** walking the list.
    ///
    /// Rule 6: only the column buckets the radius actually touches are read.
    pub fn near(&self, p: Vec3, radius_m: f32) -> Vec<u32> {
        let mut out = Vec::new();
        let r2 = radius_m * radius_m;
        for cell in self.cells_around(p, radius_m) {
            for &i in &self.cells[cell] {
                if self.points[i as usize].position_m.distance_squared(p) <= r2 {
                    out.push(i);
                }
            }
        }
        out
    }

    fn cells_around(&self, p: Vec3, radius_m: f32) -> Vec<usize> {
        let lo_x = ((p.x - radius_m - self.origin_m.x) / FIELD_CELL_M).floor().max(0.0) as usize;
        let lo_z = ((p.z - radius_m - self.origin_m.y) / FIELD_CELL_M).floor().max(0.0) as usize;
        let hi_x = (((p.x + radius_m - self.origin_m.x) / FIELD_CELL_M).floor().max(0.0) as usize)
            .min(self.columns_x.saturating_sub(1));
        let hi_z = (((p.z + radius_m - self.origin_m.y) / FIELD_CELL_M).floor().max(0.0) as usize)
            .min(self.columns_z.saturating_sub(1));
        let mut out = Vec::new();
        if lo_x >= self.columns_x || lo_z >= self.columns_z {
            return out;
        }
        for z in lo_z..=hi_z {
            for x in lo_x..=hi_x {
                out.push(z * self.columns_x + x);
            }
        }
        out
    }

    /// `F-023` — every point that is in range, in the view cone, and on a side.
    ///
    /// `forward` and `up` are the camera's, `range_m` is the gear's reach. The cone is the
    /// design's: 130° horizontal, 90° vertical by default, passed in so the Range stat and
    /// the settings can move it without touching this file.
    ///
    /// ⚠️ **No line of sight.** The design asks for one (*„freie Sichtlinie zum Spieler"*) and
    /// it is not here: the occlusion test needs avian's `SpatialQuery`, which is a `SystemParam`
    /// and not something a pure function over a `Vec` can hold. The caller filters. That is
    /// written down rather than faked, because a candidate list that *claims* line of sight and
    /// does not have one is `FIND-103` waiting to happen.
    pub fn candidates(
        &self,
        eye_m: Vec3,
        forward: Vec3,
        up: Vec3,
        range_m: f32,
        cone_h_deg: f32,
        cone_v_deg: f32,
    ) -> Vec<Candidate> {
        let forward = forward.normalize_or_zero();
        let up = up.normalize_or_zero();
        if forward == Vec3::ZERO || up == Vec3::ZERO {
            return Vec::new();
        }
        let right = forward.cross(up).normalize_or_zero();
        if right == Vec3::ZERO {
            return Vec::new();
        }
        let real_up = right.cross(forward);
        let half_h = (cone_h_deg * 0.5).to_radians();
        let half_v = (cone_v_deg * 0.5).to_radians();

        let mut out = Vec::new();
        for i in self.near(eye_m, range_m) {
            let point = &self.points[i as usize];
            let to = point.position_m - eye_m;
            let distance_m = to.length();
            if distance_m <= f32::EPSILON {
                continue;
            }
            let along = to.dot(forward);
            if along <= 0.0 {
                continue; // behind the camera
            }
            let lateral = to.dot(right);
            let vertical = to.dot(real_up);
            if lateral.atan2(along).abs() > half_h || vertical.atan2(along).abs() > half_v {
                continue;
            }
            // **The split is the sign of the lateral component and nothing else.** A point
            // exactly on the axis (`lateral == 0.0`) goes right, so that the two sets are a
            // partition and never share a member — `F-023` says Q never delivers a point
            // right of the axis, which is only checkable if "on the axis" has one answer.
            let side = if lateral < 0.0 { Hemisphere::Left } else { Hemisphere::Right };
            let angle = (along / distance_m).clamp(-1.0, 1.0);
            let score = point.quality * (1.0 - distance_m / range_m).max(0.0) * angle;
            out.push(Candidate { index: i, distance_m, side, score });
        }
        out.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then(a.index.cmp(&b.index))
        });
        out
    }

    /// The best `n` candidates of one side, best first — `F-027`'s *Bewertungsfunktion* and
    /// the cap, computed here so the HUD only has to draw.
    pub fn best_of(&self, candidates: &[Candidate], side: Hemisphere, n: usize) -> Vec<Candidate> {
        candidates.iter().filter(|c| c.side == side).take(n).copied().collect()
    }

    /// `F-021` — the points a loaded model brought with it, in world space.
    ///
    /// `anchors` is in the model root's own frame and `to_world` is that root's transform, so
    /// this is where `hook.traufe` stops being a number in a `.glb` and becomes a place in the
    /// district. Idempotent by name: adopting the same block twice does not double its points.
    pub fn adopt_named(&mut self, block: u32, to_world: &GlobalTransform, anchors: &ModelAnchors) {
        let mut added = 0usize;
        for (name, local) in anchors.0.iter() {
            let Some(short) = name.strip_prefix(HOOK_PREFIX) else {
                continue;
            };
            if self
                .points
                .iter()
                .any(|p| p.block == block && p.name.as_deref() == Some(short))
            {
                continue;
            }
            let position_m = to_world.transform_point(*local);
            self.push(AnchorPoint {
                position_m,
                normal: Vec3::Y,
                kind: AnchorKind::Named,
                quality: AnchorKind::Named.quality(),
                block,
                name: Some(short.to_string()),
                dynamic: false,
            });
            added += 1;
        }
        if added > 0 {
            debug!("anchor field: adopted {added} named points from block {block}");
        }
    }

    /// `F-031a` — the validation report for this map.
    pub fn validate(&self) -> AnchorReport {
        let mut report = AnchorReport { points: self.points.len(), ..default() };
        for (i, point) in self.points.iter().enumerate() {
            if (point.normal.length() - 1.0).abs() > 1e-3 {
                report.bad_normal += 1;
            }
            // Buried: inside a box that is not the one it was generated from. The check runs
            // over the blocks of the point's own column, not over all 2871 — rule 6 again.
            if let Some(cell) = self.cell_of(point.position_m) {
                for &b in &self.block_cells[cell] {
                    if b == point.block {
                        continue;
                    }
                    let (center, half) = self.boxes[b as usize];
                    let d = (point.position_m - center).abs();
                    if d.x < half.x - 0.01 && d.y < half.y - 0.01 && d.z < half.z - 0.01 {
                        report.buried += 1;
                        break;
                    }
                }
            }
            // Clustered: a neighbour from a *different* block closer than the minimum. Only
            // forward pairs, so a pair counts once.
            for j in self.near(point.position_m, MIN_SPACING_M) {
                if j as usize > i && self.points[j as usize].block != point.block {
                    report.clustered += 1;
                }
            }
        }
        for (cell, points) in self.cells.iter().enumerate() {
            if points.is_empty() && self.cell_holds_a_block(cell) {
                report.holes += 1;
            }
        }
        report
    }

    fn cell_holds_a_block(&self, cell: usize) -> bool {
        self.block_cells[cell]
            .iter()
            .any(|&b| self.boxes[b as usize].1.y * 2.0 >= MIN_THICKNESS_M)
    }
}

/// The `Startup` half of `F-021`: hangs the field on the app once the city is planned.
///
/// It runs **inside** [`super::map::build_map`] rather than after it, because the plan is the
/// input and re-planning 2871 blocks to read them a second time is work for nothing.
///
/// `F-022`'s acceptance is *„Eine komplette Stadtmap wird in unter 5 Minuten mit Punkten
/// befuellt"*. Measured on Ashgate: see the `info!` line this emits.
pub fn log_field(field: &AnchorField, map: &str, micros: u128) {
    let report = field.validate();
    info!(
        "anchors {map}: {} points over {} blocks in {micros} us ({} filled cells, \
         buried {} clustered {} bad-normal {} holes {})",
        field.len(),
        field.blocks().len(),
        field.filled_cells(),
        report.buried,
        report.clustered,
        report.bad_normal,
        report.holes,
    );
}

/// `F-021` — the named `hook.*` points of a model, taken into the field as it finishes loading.
///
/// **`Changed<ModelAnchors>` and nothing per tick** (rule 6). `render::model` writes the
/// component exactly once per entity, so this runs once per dressed block over the first few
/// hundred frames and then never again.
pub fn adopt_model_anchors(
    mut field: ResMut<AnchorField>,
    loaded: Query<(&AnchorBlock, &GlobalTransform, &ModelAnchors), Changed<ModelAnchors>>,
) {
    let before = field.named();
    for (block, to_world, anchors) in &loaded {
        if anchors.is_empty() {
            continue;
        }
        field.adopt_named(block.0, to_world, anchors);
    }
    // ⚠️ **This log is the only place the pack's authored anchors are countable at runtime**,
    // and it is here because "the loader reads 439 `hook.*` points" and "the game uses 439
    // `hook.*` points" are two different claims (`docs/FINDINGS.md` FIND-116). `log_field`
    // runs at `Startup`, before a single `.glb` has finished loading, so its count is the
    // generated half and nothing else. Rate-limited to *a batch that actually changed
    // something*, so a dressed district prints a handful of lines and then goes quiet.
    let after = field.named();
    if after != before {
        info!("anchors: {} named hook.* points adopted, {} in the field", after, field.len());
    }
}

/// `F-022` — the points one block contributes, out of its geometry alone.
///
/// Pure: no `Commands`, no `World`, no rng. The same block always yields the same points in
/// the same order, which is what makes the field checkable in a unit test and identical on
/// two machines.
pub fn generate_points(block: &BlockPlan, index: u32) -> Vec<AnchorPoint> {
    // Two ways a box contributes nothing, and both are the map speaking rather than a taste:
    // `F-003` took the tag away (a listed exception), or the box is a plate you cannot stand
    // on the edge of. The ground slab and every street in Ashgate are the second kind.
    if !block.anchorable || block.size_m.y < MIN_THICKNESS_M {
        return Vec::new();
    }

    let half = block.size_m * 0.5;
    let roof_y = block.center_m.y + half.y;
    let floor_y = block.center_m.y - half.y;

    // A box narrower than four insets has no frame left, so the inset shrinks with it rather
    // than turning the roof inside out. (Ashgate has 1 m parapets; without this their corner
    // points would cross over each other.)
    let ex = (half.x - CORNER_INSET_M.min(half.x * 0.5)).max(0.0);
    let ez = (half.z - CORNER_INSET_M.min(half.z * 0.5)).max(0.0);
    let (cx, cz) = (block.center_m.x, block.center_m.z);

    let mut out: Vec<AnchorPoint> = Vec::new();
    let mut keep = |out: &mut Vec<AnchorPoint>, position_m: Vec3, normal: Vec3, kind: AnchorKind| {
        if out.len() >= MAX_POINTS_PER_BLOCK {
            return;
        }
        if out.iter().any(|p| p.position_m.distance_squared(position_m) < MIN_SPACING_M.powi(2)) {
            return;
        }
        out.push(AnchorPoint {
            position_m,
            normal,
            kind,
            quality: kind.quality(),
            block: index,
            name: None,
            dynamic: false,
        });
    };

    // ## 0. Where the ladder's rungs go — computed **before** anything is placed, because
    // the ladder's share of [`MAX_POINTS_PER_BLOCK`] is reserved out of the budget and the
    // roof edges get what is left over.
    //
    // 🔴 **This ordering is the whole of the wall ladder.** Until 2026-08-26 the ladder was
    // generated last and simply ran out of budget. Ashgate's gate towers are 120 m in one
    // piece (`maps.ron`, `(±24, 60, -120)`, 20 × 120 × 55 m): four corners and ten roof-edge
    // points, then six points per rung, and the 48 run out **five and a half rungs up**. So
    // the rungs at 90 m and 105 m — the top of the climb, in front of the gate, the one
    // place in this map a player is guaranteed to be — were the two that got dropped, and
    // `f022_the_wall_stacks_into_a_ladder_of_rungs_every_fifteen_metres` was red on exactly
    // y = 105. **A cap that truncates a ladder from the top is a ladder you cannot finish.**
    let mut rung_heights: Vec<f32> = Vec::new();
    if block.size_m.y >= COURSE_MIN_HEIGHT_M {
        let mut rung_y = floor_y + COURSE_RISE_M;
        while rung_y < roof_y - 1.0 {
            rung_heights.push(rung_y);
            rung_y += COURSE_RISE_M;
        }
    }
    // One point on each of the four faces of each rung is what *a complete ladder* costs.
    // Capped so the corners and a handful of roof edges always survive it.
    let ladder_reserve = (rung_heights.len() * 4).min(MAX_POINTS_PER_BLOCK.saturating_sub(8));

    // ## 1. The four roof corners. Best points on the box: two edges meet, so a rope that
    // overshoots one still lands on the other.
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        keep(
            &mut out,
            Vec3::new(cx + sx * ex, roof_y, cz + sz * ez),
            Vec3::Y,
            AnchorKind::Corner,
        );
    }

    // ## 2. The roof edges between them.
    //
    // The spacing is [`EDGE_SPACING_M`] **or whatever keeps this box inside its budget**,
    // whichever is coarser. A 700 m wall course at a flat 12 m would be 116 points on one
    // block and the field would stop being a list of places you aim at; at the adaptive
    // spacing it is a rung, evenly spread, with a point in the middle whenever the count is
    // odd. That last property is not decoration: the gate stands in the middle of Ashgate's
    // wall, so the middle of a course is exactly where a player is climbing.
    let perimeter_m = 4.0 * (ex + ez);
    let edge_budget = (MAX_POINTS_PER_BLOCK - 4 - ladder_reserve) as f32 * 0.6;
    let spacing_m = EDGE_SPACING_M.max(perimeter_m / edge_budget.max(1.0));
    for (along_x, sign) in [(true, -1.0f32), (true, 1.0), (false, -1.0), (false, 1.0)] {
        let (len_m, fixed) = if along_x { (ex * 2.0, cz + sign * ez) } else { (ez * 2.0, cx + sign * ex) };
        let n = (len_m / spacing_m).floor() as i32;
        for i in 0..n {
            let t = -len_m * 0.5 + len_m * (i as f32 + 0.5) / n as f32;
            let position_m = if along_x {
                Vec3::new(cx + t, roof_y, fixed)
            } else {
                Vec3::new(fixed, roof_y, cz + t)
            };
            keep(&mut out, position_m, Vec3::Y, AnchorKind::Edge);
        }
    }

    // ## 3. The facade ladder — `hook.gesims_15..105`, computed.
    //
    // Only for a box taller than [`COURSE_MIN_HEIGHT_M`]: below that the roof is inside one
    // rope-length of the ground and a rung buys nothing but a marker. Ashgate's gate towers
    // are 120 m in one piece and its church is 35 m; the wall band itself needs none of this,
    // because it is already built one 15 m course at a time and every course top is a roof.
    //
    // **Pass A — every rung gets its four face centres.** This is the guaranteed half, and
    // `ladder_reserve` above is what pays for it. Completeness before density: a rung you can
    // reach on all four sides beats two extra points on a rung nobody got to.
    for &rung_y in &rung_heights {
        for (normal, at) in facade_faces(cx, cz, half, rung_y) {
            keep(&mut out, at, normal, AnchorKind::Course);
        }
    }
    // **Pass B — the extra points along a long face**, at [`COURSE_SPACING_M`], out of
    // whatever the block has left. This half *is* allowed to run out, and when it does it
    // thins the ladder rather than cutting its top off: pass A already put a point on every
    // rung of every face. (Where `n` is odd the middle point is the face centre again and
    // [`MIN_SPACING_M`] drops it, which is why this pass is a fill and not a duplicate.)
    for &rung_y in &rung_heights {
        for (normal, at) in facade_faces(cx, cz, half, rung_y) {
            let along_x = normal.x == 0.0;
            let len_m = if along_x { ex * 2.0 } else { ez * 2.0 };
            let n = (len_m / COURSE_SPACING_M).floor() as i32;
            for i in 0..n {
                let t = -len_m * 0.5 + len_m * (i as f32 + 0.5) / n as f32;
                let offset = if along_x { Vec3::X * t } else { Vec3::Z * t };
                keep(&mut out, at + offset, normal, AnchorKind::Course);
            }
        }
    }
    out
}

/// The centre of each of a box's four vertical faces at one height, with its outward normal.
///
/// Both passes of the facade ladder walk the same four faces in the same order, and that
/// order is part of `F-021`'s determinism contract (`docs/multiplayer.md`: a city that
/// differs by iteration order is a desync).
fn facade_faces(cx: f32, cz: f32, half: Vec3, rung_y: f32) -> [(Vec3, Vec3); 4] {
    [
        (Vec3::X, Vec3::new(cx + half.x, rung_y, cz)),
        (Vec3::NEG_X, Vec3::new(cx - half.x, rung_y, cz)),
        (Vec3::Z, Vec3::new(cx, rung_y, cz + half.z)),
        (Vec3::NEG_Z, Vec3::new(cx, rung_y, cz - half.z)),
    ]
}
