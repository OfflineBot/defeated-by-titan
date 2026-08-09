//! The guard over the city — `F-003`.
//!
//! A city that comes out of a file has four ways of being wrong, and **you see none of them
//! in the image**:
//!
//! 1. It is not built at all (or built twice) — in the image both just look like "houses".
//! 2. It is not deterministic — that surfaces only over the network, on the most expensive
//!    day there is.
//! 3. The collision shape is off by a factor of 2 against the render shape — the hook catches
//!    in mid-air, and the image still shows a house.
//! 4. Everything is anchorable — then "no hook on untagged surfaces" (`F-003`) checks nothing.
//!
//! So this test measures **against `assets/data/maps.ron`**, not against itself.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{AnchorSurface, Block, Body, BodyMask, Cli};
use defeated_by_titan::world::map::{plan_blocks, BlockPlan};
use std::path::PathBuf;

/// Builds the **real** app, headless, and runs `Startup` once.
///
/// Not a second, similar app: otherwise the test proves nothing about the game that is
/// actually played (the same argument as in `tests/multiplayer.rs`).
fn built_world() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.update();
    app
}

fn data() -> GameData {
    GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

/// The plan `build_map` works through — the same function, but without an app.
fn plan() -> Vec<BlockPlan> {
    let d = data();
    let map = d.current_map().expect("maps.ron: current must exist");
    plan_blocks(&d, map)
}

/// Every built cuboid as `(name, center, full size, anchorable)`, sorted by name.
fn built_blocks(app: &mut App) -> Vec<(String, Vec3, Vec3, bool)> {
    let mut q = app
        .world_mut()
        .query::<(&Name, &Block, &Transform, Option<&AnchorSurface>)>();
    let mut all: Vec<(String, Vec3, Vec3, bool)> = q
        .iter(app.world())
        .map(|(n, k, t, a)| (n.to_string(), t.translation, k.size, a.is_some()))
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

#[test]
fn f003_the_city_comes_from_the_file_and_not_twice() {
    // K1. Red when `build_map` is an empty body (0 instead of ~90), when it spawns twice
    // (2x), or when the layout generates nothing (then it would be exactly the placed blocks
    // from the file).
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();
    let placed = map.blocks.len();
    let generated = plan.len() - placed;

    assert!(
        generated > 40,
        "the layout generated {generated} houses — a city with fewer is a stub, \
         not a district (maps.ron: layout)"
    );

    let mut app = built_world();
    let built = built_blocks(&mut app);
    assert_eq!(
        built.len(),
        plan.len(),
        "{} entities with Block, but {} planned cuboids ({placed} placed + {generated} \
         generated). Zero means: nothing built. Double means: two writers",
        built.len(),
        plan.len()
    );

    // And letting it keep running changes nothing about that: the city belongs in `Startup`,
    // not in `Update` — otherwise it grows by one city every frame.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(built_blocks(&mut app).len(), plan.len(), "the city grows every frame");

    // Independent of the planning function: every block placed in the file stands in the
    // world exactly once, at exactly its center and at exactly its size.
    for (i, k) in map.blocks.iter().enumerate() {
        let center = Vec3::new(k.center_m.0, k.center_m.1, k.center_m.2);
        let size = Vec3::new(k.size_m.0, k.size_m.1, k.size_m.2);
        let hits: Vec<_> = built
            .iter()
            .filter(|(_, m, g, _)| *m == center && *g == size)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "maps.ron: blocks[{i}] (center {center:?}, size {size:?}) is in the world {}x \
             instead of exactly once",
            hits.len()
        );
        assert_eq!(
            hits[0].3, k.anchorable,
            "maps.ron: blocks[{i}] anchorable = {} in the file, {} in the world",
            k.anchorable, hits[0].3
        );
    }
}

#[test]
fn f003_the_same_seed_yields_exactly_the_same_city() {
    // K2. Red the second somebody wires in `rand::random()`, a `HashMap` iteration or a clock
    // reading. Every value is compared, not the count: a city with the same number of houses
    // standing in different places is the same bug over the network.
    let first = plan();
    let second = plan();
    assert_eq!(first.len(), second.len(), "two runs, two city sizes");
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a, b, "the same seed, two different cuboids:\n  {a:?}\n  {b:?}");
    }

    // And the same city comes out of the real app too — not just out of the planning
    // function. Two separate apps, value by value.
    let mut app_a = built_world();
    let mut app_b = built_world();
    assert_eq!(built_blocks(&mut app_a), built_blocks(&mut app_b));

    // A different seed yields a different city — otherwise the comparison above only checks
    // that the function does the same thing twice, which it would do without an rng at all.
    let d = data();
    let mut map = d.current_map().expect("current map").clone();
    map.seed = map.seed.wrapping_add(1);
    let other = plan_blocks(&d, &map);
    assert_ne!(other, first, "a different seed produced exactly the same city");
}

#[test]
fn f003_the_colliders_carry_the_half_edge_from_the_file() {
    // K3. `Collider::cuboid` takes the FULL edge and halves it internally
    // (avian3d-0.7.0/src/collision/collider/parry/mod.rs:747-749), while `Body::half_size_m`
    // and parry's `Cuboid::half_extents` carry the HALF one. A factor of 2 does not show up
    // in the image — it does here.
    //
    // What is measured is the SHAPE SOURCE (`collider.shape()`), not `ColliderAabb`: that
    // hull grows by a measured 0.01 m per axis, and by the sweep once something moves.
    let d = data();
    let map = d.current_map().expect("current map");
    let mut app = built_world();

    let mut q = app.world_mut().query::<(&Transform, &Block, &Body, &Collider)>();
    let all: Vec<(Vec3, Vec3, Vec3, Vec3)> = q
        .iter(app.world())
        .map(|(t, b, k, c)| {
            let form = c
                .shape()
                .as_cuboid()
                .expect("every block is a cuboid — nothing here is rotated");
            let h = form.half_extents;
            (t.translation, b.size, k.half_size_m, Vec3::new(h.x, h.y, h.z))
        })
        .collect();

    assert!(all.len() > 40, "only {} blocks with a collider", all.len());

    let mut measured = 0;
    for (i, k) in map.blocks.iter().enumerate() {
        let center = Vec3::new(k.center_m.0, k.center_m.1, k.center_m.2);
        let full = Vec3::new(k.size_m.0, k.size_m.1, k.size_m.2);
        let (_, render, aabb, collider) = all
            .iter()
            .find(|(m, _, _, _)| *m == center)
            .unwrap_or_else(|| panic!("maps.ron: blocks[{i}] is not at {center:?}"));
        assert_eq!(*render, full, "blocks[{i}]: render shape deviates from the file");
        assert_eq!(
            *collider,
            full * 0.5,
            "blocks[{i}]: collider half size {collider:?}, expected {:?} — \
             factor {:.2} against the file",
            full * 0.5,
            collider.x / (full.x * 0.5)
        );
        assert_eq!(*aabb, full * 0.5, "blocks[{i}]: Body::half_size_m deviates");
        measured += 1;
    }
    assert!(measured >= 3, "only {measured} blocks checked against the file, at least 3 required");

    // And for EVERY block, the generated ones included: render shape and collision shape are
    // the same shape. That is exactly why one writer sets both.
    for (center, render, aabb, collider) in &all {
        assert_eq!(*collider, *render * 0.5, "block at {center:?}: render {render:?}");
        assert_eq!(*aabb, *render * 0.5, "block at {center:?}: aabb {aabb:?}");
    }
}

#[test]
fn f003_not_every_surface_is_anchorable() {
    // K4. Red the moment somebody tags everything wholesale — then "no hook on untagged
    // surfaces" (F-003) can no longer be falsified, and the criterion checks nothing.
    let mut app = built_world();
    let built = built_blocks(&mut app);
    let anchorable = built.iter().filter(|(_, _, _, a)| *a).count();
    let untagged = built.len() - anchorable;
    assert!(anchorable > 0, "not a single anchor surface in the built city");
    assert!(
        untagged > 0,
        "all {} blocks carry an anchor surface — then F-003 checks nothing",
        built.len()
    );

    // The marker and the mask are the same state, written in two places by one writer. If
    // they drift apart, the hook catches somewhere other than where the gizmo says.
    let mut q = app.world_mut().query::<(&Body, Option<&AnchorSurface>)>();
    for (body, anchors) in q.iter(app.world()) {
        assert_eq!(
            body.mask.contains(BodyMask::ANCHORABLE),
            anchors.is_some(),
            "BodyMask {:?} and AnchorSurface {:?} contradict each other",
            body.mask,
            anchors.is_some()
        );
    }
}

#[test]
fn f003_no_grid_house_stands_inside_a_placed_block() {
    // `maps.ron`: "What is placed explicitly wins: the generated stuff leaves room around
    // every placed block." Red when the overlap check is missing — then a 28 m block grows
    // straight through the watchtower.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();
    let (placed, generated) = plan.split_at(map.blocks.len());

    for house in generated {
        for block in placed {
            let distance = (house.center_m - block.center_m).abs();
            let sum = house.size_m * 0.5 + block.size_m * 0.5;
            assert!(
                !(distance.x < sum.x && distance.y < sum.y && distance.z < sum.z),
                "{} sits inside {}: distance {distance:?}, sum {sum:?}",
                house.name,
                block.name
            );
        }
    }

    // The converse, which is what turns all of this into a claim: the ground slab covers the
    // whole map, and the city still stands. If the check were not strict, everything here
    // would be empty.
    assert!(!generated.is_empty(), "the ground slab swallowed the whole city");
}

#[test]
fn f003_the_space_around_the_origin_stays_clear() {
    // That is where the player starts, and `scripts/t007-first-run.txt` runs 6 m toward -Z.
    // Red when `clear_radius_m` is ignored — then a house stands on top of the player.
    let d = data();
    let map = d.current_map().expect("current map");
    let radius = map.layout.clear_radius_m;
    let plan = plan();

    for house in &plan[map.blocks.len()..] {
        let half = house.size_m * 0.5;
        let dx = (house.center_m.x.abs() - half.x).max(0.0);
        let dz = (house.center_m.z.abs() - half.z).max(0.0);
        let distance = (dx * dx + dz * dz).sqrt();
        assert!(
            distance >= radius,
            "{} comes within {distance:.2} m of the origin, clear_radius_m = {radius}",
            house.name
        );
    }
}

#[test]
fn f003_the_grid_houses_stay_in_the_height_window_from_the_file() {
    // `maps.ron` says: the city is flat, the vertical comes from the landmarks. Red when
    // somebody pulls the residential band back up — in the image that looks like a skyline,
    // not like a bug.
    let d = data();
    let map = d.current_map().expect("current map");
    let r = &map.layout;
    let plan = plan();
    let houses = &plan[map.blocks.len()..];

    let mut tall = 0;
    for house in houses {
        let h = house.size_m.y;
        assert!(
            (r.min_height_m..=r.max_height_m).contains(&h),
            "{}: {h} m is not in {}..={} (maps.ron: layout)",
            house.name,
            r.min_height_m,
            r.max_height_m
        );
        assert!(house.center_m.y > 0.0, "{}: center below the ground", house.name);
        assert!(
            (house.center_m.y - h * 0.5).abs() < 1e-4,
            "{}: does not stand on y = 0, but at {}",
            house.name,
            house.center_m.y - h * 0.5
        );
        if h > (r.min_height_m + r.max_height_m) * 0.5 {
            tall += 1;
        }
    }
    // Not all the same height: a city of nothing but 8 m blocks would be an arithmetic bug
    // that no height window catches.
    assert!(
        tall > 0 && tall < houses.len(),
        "{tall} of {} houses in the upper half of the window — the heights do not spread",
        houses.len()
    );
}
