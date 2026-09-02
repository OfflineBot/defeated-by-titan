//! dressing — the drawn model, its collider and its anchors are ONE house (B-039).
//!
//! **The sentence this binary exists for** (user, 2026-09-01, at the controller):
//! *„zudem sind die anchor points bei häusern in der luft! das passt nicht."* The measuring
//! round behind it (1584 bites, 15 dressed houses, offset to the nearest drawn surface median
//! 1.07 m / worst 2.84 m) split the cause three ways, and each test here holds one of them
//! shut:
//!
//! 1. **the quarter turn** — a house fronting along z transposes its *box* when it is planned
//!    (`world::map`, the `(depth_m, front_m, ..)` arm) and until 2026-09-01 the *drawing*
//!    did not turn with it: two visible walls 1.6–1.8 m inside the collider, the mesh 0.3 m
//!    out through the other two, on 5 of 15 houses;
//! 2. **the envelope slack** — the authored `hit` pair sits 0.23–0.30 m outside the visible
//!    walls on every side of every a-083 file;
//! 3. **the roof shape** — one cuboid at full width to the ridge, under a drawn roof that
//!    slopes in above ~70 % of the height (roofline bites 1.3–2.8 m in the air).
//!
//! Fixture notes (docs/lessons/fixtures.md #2 — name both lists):
//! * the code under test reads: `BlockPlan { size_m, model, yaw_rad }`, `art.ron: hulls`,
//!   the DRESSING/RUIN/RUBBLE catalogues (themselves pinned to the glb files by
//!   `tests/world.rs::f003_the_dressing_catalogue_is_what_the_glb_files_really_measure`);
//! * the fixture varies: house orientation (plain AND swapped — the n = 2 rule, #4), the
//!   dressed class, ray height (wall band, roof slope, ridge), ray side;
//! * held constant: the shipped map (`maps.ron: current`) and its seed — this binary asks
//!   about the city the player is in, not an invented one.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::MODEL_FACES;
use defeated_by_titan::world::map::{
    plan_blocks, BlockPlan, DRESSING, PLACED_DRESSING, RUBBLE_KIT, RUIN_KIT,
};
use std::f32::consts::FRAC_PI_2;
use std::path::PathBuf;

fn data() -> GameData {
    GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

/// The authored full extent of a dressed class, out of the same catalogues the planner reads.
fn authored_of(model: &str) -> Option<[f32; 3]> {
    DRESSING
        .iter()
        .chain(RUIN_KIT.iter())
        .chain(RUBBLE_KIT.iter())
        .find(|(name, _)| *name == model)
        .map(|(_, size)| *size)
}

/// Does the total turn (`MODEL_FACES + yaw`) put the authored x extent on world z?
fn drawn_transposed(yaw_rad: f32) -> bool {
    let quarters = ((MODEL_FACES + yaw_rad) / FRAC_PI_2).round() as i64;
    quarters.rem_euclid(2) == 1
}

/// The drawn footprint (x, z) of a dressed plan: the authored extents at the plan's scale,
/// through the same rotation the mesh gets. This is the arithmetic of `render::model`'s
/// `model_transform` reduced to extents — the claim under test is that it lands on `size_m`.
fn drawn_footprint_m(plan: &BlockPlan, authored_m: [f32; 3]) -> (f32, f32) {
    let scale = plan.size_m.y / authored_m[1];
    if drawn_transposed(plan.yaw_rad) {
        (authored_m[2] * scale, authored_m[0] * scale)
    } else {
        (authored_m[0] * scale, authored_m[2] * scale)
    }
}

/// Which way the BOX says the house fronts — measured on the box itself, **independent of
/// `yaw_rad`** (the thing under test must not classify its own fixture, fixtures.md #5): the
/// planner writes either `(front, depth)` or `(depth, front)` into `size_m`, exactly.
fn box_transposed(plan: &BlockPlan, authored_m: [f32; 3]) -> bool {
    let scale = plan.size_m.y / authored_m[1];
    let plain = (plan.size_m.x - authored_m[0] * scale).abs();
    let swapped = (plan.size_m.x - authored_m[2] * scale).abs();
    swapped < plain
}

#[test]
fn b039_a_house_fronting_along_z_draws_its_walls_on_its_collider() {
    // ★ Cause 1, the transpose. Before the quarter turn this fails on every swapped house
    // with a miss of |front − depth| · scale ≈ 1.2–1.6 m — the drawn wall standing that far
    // inside (one side) and outside (the other side) of the collider that the rope bites.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan_blocks(&d, map);

    let (mut plain, mut swapped, mut skipped) = (0usize, 0usize, 0usize);
    for b in plan.iter().filter(|b| b.name.starts_with("house")) {
        let Some(model) = b.model else {
            skipped += 1; // undressed house: nothing is drawn, nothing can be off
            continue;
        };
        let authored = authored_of(model)
            .unwrap_or_else(|| panic!("{}: model {model:?} is in no catalogue", b.name));
        if box_transposed(b, authored) {
            swapped += 1;
        } else {
            plain += 1;
        }
        let (dx, dz) = drawn_footprint_m(b, authored);
        assert!(
            (dx - b.size_m.x).abs() < 0.02 && (dz - b.size_m.z).abs() < 0.02,
            "{} ({model}, yaw {:.2}): the drawn mesh covers {dx:.2} x {dz:.2} m but the \
             collider is {:.2} x {:.2} m — the visible wall stands {:.2} m off the surface \
             the rope bites",
            b.name,
            b.yaw_rad,
            b.size_m.x,
            b.size_m.z,
            ((dx - b.size_m.x).abs().max((dz - b.size_m.z).abs())) / 2.0,
        );
    }
    // The n = 2 rule: a district with only one orientation cannot see the transpose at all.
    assert!(
        plain >= 1 && swapped >= 1,
        "fixture too thin: {plain} plain / {swapped} swapped dressed houses \
         ({skipped} undressed skipped) — both orientations must exist for this to prove anything"
    );
}

#[test]
fn b039_a_fallen_house_fronting_along_z_draws_its_remnant_on_its_collider() {
    // The same seam through the other arm: `remnant_for` swaps its footprint by the same
    // `frontage_along_x` and its model is drawn by the same `model_transform`.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan_blocks(&d, map);

    let (mut checked, mut skipped) = (0usize, 0usize);
    for b in plan
        .iter()
        .filter(|b| b.name.starts_with("ruin") || b.name.starts_with("rubble"))
    {
        let Some(model) = b.model else {
            skipped += 1; // a remnant with the kit switched off is a bare box in ash
            continue;
        };
        let authored = authored_of(model)
            .unwrap_or_else(|| panic!("{}: model {model:?} is in no catalogue", b.name));
        let (dx, dz) = drawn_footprint_m(b, authored);
        // A remnant's box is the model silhouette shrunk by `min(...)` fits — the drawn
        // footprint must cover the box exactly on the axis the scale came from and never be
        // smaller than the box anywhere (the same "nothing visible outside the collider is
        // promised, nothing hookable beyond the mesh is" trade as the houses).
        assert!(
            (dx - b.size_m.x).abs() < 0.02 && (dz - b.size_m.z).abs() < 0.02,
            "{} ({model}, yaw {:.2}): drawn {dx:.2} x {dz:.2} m, collider {:.2} x {:.2} m",
            b.name,
            b.yaw_rad,
            b.size_m.x,
            b.size_m.z,
        );
        checked += 1;
    }
    // Count what we skip (fixtures.md #3). Zero dressed remnants would make this test
    // vacuous — say so instead of passing silently.
    assert!(
        checked >= 1,
        "no dressed remnant in the shipped map ({skipped} bare ones skipped) — this test \
         measured nothing; if the damage kit went away on purpose, delete it with reasons"
    );
}

#[test]
fn b039_every_house_hull_stays_inside_the_authored_envelope() {
    // ★ Cause 2. The compound must round INWARD: every wall box and roof rectangle inside
    // the authored hit envelope by at least the margin the mesh itself keeps (measured
    // 0.23–0.30 m of slack per side; 0.10 m asserted, so an honest re-derivation cannot trip
    // it while an envelope-copied number always will).
    let d = data();
    for (model, authored) in DRESSING {
        let hull = d
            .art
            .hulls
            .get(model)
            .unwrap_or_else(|| panic!("art.ron: hulls has no {model:?} — the whole compound \
                 feature silently reverts to the envelope cuboid for this class"));
        let (hx, hy, hz) = (authored[0] * 0.5, authored[1], authored[2] * 0.5);
        for w in &hull.walls {
            assert!(
                w.min_m.0 > -hx + 0.10 && w.max_m.0 < hx - 0.10,
                "{model}: wall x [{:.2}, {:.2}] is not inside the ±{hx:.2} envelope by 10 cm",
                w.min_m.0,
                w.max_m.0
            );
            assert!(
                w.min_m.2 > -hz + 0.10 && w.max_m.2 < hz - 0.10,
                "{model}: wall z [{:.2}, {:.2}] is not inside the ±{hz:.2} envelope by 10 cm",
                w.min_m.2,
                w.max_m.2
            );
            assert!(w.min_m.1 >= 0.0 && w.max_m.1 <= hy, "{model}: wall leaves 0..{hy}");
        }
        assert!(
            hull.roof_rects.len() >= 2,
            "{model}: fewer than two roof rectangles is not a wedge, it is a plane"
        );
        let mut last_y = f32::NEG_INFINITY;
        for r in &hull.roof_rects {
            assert!(r.y_m > last_y, "{model}: roof_rects must ascend");
            last_y = r.y_m;
            assert!(
                r.min_m.0 >= -hx && r.max_m.0 <= hx && r.min_m.1 >= -hz && r.max_m.1 <= hz,
                "{model}: roof rect at y {:.2} leaves the envelope",
                r.y_m
            );
            assert!(r.y_m <= hy + 0.01, "{model}: roof above the authored height");
        }
        // Continuity: the wedge starts where the tallest wall box ends, or a ray at the
        // eaves line passes between the two and the fascia is unhookable.
        let wall_top =
            hull.walls.iter().map(|w| w.max_m.1).fold(f32::NEG_INFINITY, f32::max);
        let wedge_bottom = hull.roof_rects[0].y_m;
        assert!(
            (wall_top - wedge_bottom).abs() < 0.01,
            "{model}: walls end at {wall_top:.2} m and the wedge starts at {wedge_bottom:.2} m"
        );
    }
}

/// A swapped town house at the origin, exactly as `plan_blocks` shapes one at scale 1.0:
/// authored 9.10 x 8.00 x 7.90, fronting along z, so the box carries (depth, height, front).
fn swapped_town_house() -> BlockPlan {
    BlockPlan {
        name: "house_fixture".into(),
        center_m: Vec3::ZERO,
        size_m: Vec3::new(7.90, 8.00, 9.10),
        color: [0.5, 0.5, 0.5],
        anchorable: true,
        solid: true,
        model: Some("house_town"),
        yaw_rad: FRAC_PI_2,
    }
}

#[test]
fn b039_the_wall_collider_stands_on_the_drawn_wall_plane_not_the_hit_envelope() {
    // ★ Causes 1+2 together, on the collider avian actually raycasts. The authored wall
    // planes of house_town are z −3.21 / +3.25 (area-weighted dominant planes, FIND-225);
    // under MODEL_FACES + FRAC_PI_2 (three quarter turns: (x, z) → (−z, x)) they land on
    // world x +3.21 / −3.25. The old envelope face was at ±3.95.
    let d = data();
    let plan = swapped_town_house();
    let c: Collider = plan.collider(&d);

    // A horizontal ray into the wall band, from +x. Entity-local y −2.0 is 2.0 m over the floor.
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(6.0, -2.0, 0.0), Vec3::NEG_X, 12.0, true)
        .expect("the wall must stop the ray");
    let x = 6.0 - hit.0;
    assert!(
        (x - 3.21).abs() < 0.05,
        "the +x wall bites at x = {x:.2} m — the drawn plane is 3.21, the old envelope 3.95"
    );
    // And from −x: the OTHER authored plane, which only lands here if the turn is a real
    // rotation and not a mirror (the two planes differ by 4 cm on purpose).
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-6.0, -2.0, 0.0), Vec3::X, 12.0, true)
        .expect("the −x wall must stop the ray");
    let x = -6.0 + hit.0;
    assert!(
        (x + 3.25).abs() < 0.05,
        "the −x wall bites at x = {x:.2} m — the drawn plane is −3.25"
    );
    // The frontage axis keeps the authored x planes (±3.8) on world z.
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, -2.0, 6.0), Vec3::NEG_Z, 12.0, true)
        .expect("the gable wall must stop the ray");
    let z = 6.0 - hit.0;
    assert!(
        (z - 3.81).abs() < 0.06,
        "the +z gable bites at z = {z:.2} m — the drawn plane is 3.81, the old envelope 4.55"
    );
}

#[test]
fn b039_a_roofline_ray_bites_the_drawn_slope_not_the_old_box_corner() {
    // ★ Cause 3, the one the player aims at. At authored y 7.6 the drawn roof of house_town
    // is 0.42 m wide (slope 3.72 → ~0 over 5.68..7.84); the old cuboid was still 7.90 m wide
    // up there, so a roofline bite hung metres off the slope. Swapped, the slope shows on
    // world x.
    let d = data();
    let c = swapped_town_house().collider(&d);
    let y = -4.0 + 7.6; // entity-local: floor is at −4.0
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(6.0, y, 0.0), Vec3::NEG_X, 12.0, true)
        .expect("the wedge must still stop a roofline ray — nothing visible may be unhookable");
    let x = 6.0 - hit.0;
    assert!(
        x < 0.9,
        "a ray at the 7.6 m roofline bites at x = {x:.2} m off axis — the drawn slope is \
         0.42 m wide there, the old envelope face was at 3.95 (a 2.8 m air bite)"
    );
    // And the gable end at the same height keeps its FULL width — the wedge narrows only
    // across the ridge, or the whole gable wall would become unhookable (conservative side).
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, y, 6.0), Vec3::NEG_Z, 12.0, true)
        .expect("the gable must stop the ray at roof height");
    let z = 6.0 - hit.0;
    assert!(
        (z - 4.25).abs() < 0.06,
        "the gable at roof height bites at z = {z:.2} m — the drawn gable reaches 4.25"
    );
}

#[test]
fn b039_the_ridge_of_a_dressed_house_is_still_hookable() {
    // The conservative half stated as its own claim: shrinking the roof must never take the
    // ridge line away. Straight down onto the ridge of a PLAIN town house (yaw 0 — the n = 2
    // rule cuts both ways: the wedge has to stand in both orientations).
    let d = data();
    let mut plan = swapped_town_house();
    plan.size_m = Vec3::new(9.10, 8.00, 7.90);
    plan.yaw_rad = 0.0;
    let c = plan.collider(&d);
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(0.0, 8.0, 0.0), Vec3::NEG_Y, 12.0, true)
        .expect("the ridge is gone — the most-aimed-at line of the house lost its collider");
    let y = 8.0 - hit.0;
    assert!(
        (y - 3.84).abs() < 0.05,
        "the ridge sits at entity y = {y:.2} m — authored 7.84 over a floor at −4.0"
    );
    // And nothing of the compound reaches above the drawn ridge: a bite on top of the old
    // envelope (y = +4.0, full footprint) was exactly the „in der luft" symptom.
    assert!(
        !c.contains_point(Vec3::ZERO, Quat::IDENTITY, Vec3::new(3.0, 3.95, 0.0)),
        "the old envelope's top corner is still solid — roofline bites would float again"
    );
}

/// The authored full extent over ALL four catalogues — [`authored_of`] plus the placed
/// props, which grew hull rows with B-043 and are in none of the three generated kits.
fn authored_of_any(model: &str) -> Option<[f32; 3]> {
    authored_of(model).or_else(|| {
        PLACED_DRESSING
            .iter()
            .find(|(name, _, _)| *name == model)
            .map(|(_, _, size)| *size)
    })
}

#[test]
fn b043_every_hull_stays_inside_its_class_envelope() {
    // ★ The conservative-index invariant B-039 relies on, stated over EVERY hulls row and
    // not just the three houses: the spatial index (`Body`) stays the plan envelope, so a
    // compound that reaches OUTSIDE the envelope is a surface the index cannot see — a
    // hookable wall that a range query says is not there. Rounding is INWARD, always,
    // even where the drawn mesh itself pokes out of its own hit pair (ruin_wall_corner's
    // wall is drawn to x 4.42 against a ±3.24 envelope; its hull row is clamped, B-043).
    //
    // Fixture notes (fixtures.md #2): the code under test reads `art.ron: hulls` and the
    // four catalogues; this test varies the model (every row that exists) and holds the
    // envelope definition constant. It fails on any new row that copies mesh extents
    // without clamping them.
    let d = data();
    assert!(
        d.art.hulls.len() >= 17,
        "only {} hulls rows — the B-043 set (3 houses + 14 remnant classes) is gone",
        d.art.hulls.len()
    );
    for (model, hull) in &d.art.hulls {
        let authored = authored_of_any(model)
            .unwrap_or_else(|| panic!("art.ron: hulls[{model:?}] is in no catalogue"));
        let (hx, hy, hz) = (authored[0] * 0.5, authored[1], authored[2] * 0.5);
        let eps = 1e-3;
        for w in &hull.walls {
            assert!(
                w.min_m.0 >= -hx - eps
                    && w.max_m.0 <= hx + eps
                    && w.min_m.1 >= -eps
                    && w.max_m.1 <= hy + eps
                    && w.min_m.2 >= -hz - eps
                    && w.max_m.2 <= hz + eps,
                "{model}: wall ({:?}..{:?}) leaves the ±{hx:.2} x 0..{hy:.2} x ±{hz:.2} \
                 envelope — the spatial index would lose this surface",
                w.min_m,
                w.max_m
            );
            assert!(
                w.min_m.0 < w.max_m.0 && w.min_m.1 < w.max_m.1 && w.min_m.2 < w.max_m.2,
                "{model}: degenerate wall box {:?}..{:?}",
                w.min_m,
                w.max_m
            );
        }
        for r in &hull.roof_rects {
            assert!(
                r.min_m.0 >= -hx - eps
                    && r.max_m.0 <= hx + eps
                    && r.min_m.1 >= -hz - eps
                    && r.max_m.1 <= hz + eps
                    && r.y_m >= -eps
                    && r.y_m <= hy + eps,
                "{model}: roof rect at y {:.2} leaves the envelope",
                r.y_m
            );
        }
        assert!(
            !hull.walls.is_empty() || hull.roof_rects.len() >= 2,
            "{model}: neither walls nor a wedge — an empty hull is a class nothing can hit"
        );
    }
}

/// A `ruin_wall_corner` at the origin exactly as `remnant_for` shapes one at scale 1.0:
/// the box is the file's own hit extent (6.47 x 5.60 x 6.80), yaw 0, so the compound is
/// turned by `MODEL_FACES` (pi) alone — authored (x, z) land on entity-local (-x, -z).
fn corner_ruin_fixture() -> BlockPlan {
    BlockPlan {
        name: "ruin_fixture".into(),
        center_m: Vec3::ZERO,
        size_m: Vec3::new(6.47, 5.60, 6.80),
        color: [0.5, 0.5, 0.5],
        anchorable: true,
        solid: true,
        model: Some("ruin_wall_corner"),
        yaw_rad: 0.0,
    }
}

#[test]
fn b043_a_remnant_ray_bites_the_drawn_wall_not_the_envelope() {
    // ★ B-043 in miniature, on the collider avian actually raycasts. The drawn a-089
    // L-wall's long face sits at authored z -0.22 (measured, 0.20 m voxel sweep of the
    // glb, 2026-09-02); under the pi turn it lands at entity-local z +0.22. The old
    // envelope face was at z 3.40 — 3.18 m of air, which is the fleet's "median 2.88 m"
    // on this class and the reason `ruin_161_3` could anchor 4.23 m from anything drawn.
    let d = data();
    let c: Collider = corner_ruin_fixture().collider(&d);

    // The wall band: entity-local y -2.0 is authored 0.8..3.6 territory (floor at -2.8).
    // Authored x 0.35..3.22 lands at local x -3.22..-0.35; aim down the middle.
    let hit = c
        .cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.8, -1.0, 6.0), Vec3::NEG_Z, 12.0, true)
        .expect("the drawn wall must stop the ray");
    let z = 6.0 - hit.0;
    assert!(
        (z - 0.22).abs() < 0.05,
        "the wall bites at local z = {z:.2} m — the drawn plane is 0.22, the envelope 3.40"
    );
}

#[test]
fn b043_the_phantom_band_of_the_corner_ruin_is_gone() {
    // ★ Leg A of scripts/b043-air-anchor.txt as a unit test: authored x -1.85..-0.25 holds
    // NO drawn triangle above the base spill (y 0.93+), yet the envelope was solid there —
    // the in-game measurement put a bite on that face 4.23 m from any drawn surface. Under
    // the pi turn the band is entity-local x 0.25..1.85; a ray down it must now fly through.
    //
    // The n = 2 companion (fixtures.md #1: delete the thing you measure and watch the
    // number move): the same ray one metre further at local x -1.0 crosses the REAL wall
    // and must still bite — "collide what is drawn", not "stop colliding".
    let d = data();
    let c: Collider = corner_ruin_fixture().collider(&d);
    let phantom =
        c.cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(1.0, -1.0, 6.0), Vec3::NEG_Z, 12.0, true);
    assert!(
        phantom.is_none(),
        "the empty -x band still collides at local x 1.0 (hit {phantom:?}) — the rope would \
         anchor in open air exactly as B-043 reported"
    );
    let real =
        c.cast_ray(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, -1.0, 6.0), Vec3::NEG_Z, 12.0, true);
    assert!(
        real.is_some(),
        "the drawn wall stopped colliding — the fix overshot from phantom into unhookable"
    );
}

