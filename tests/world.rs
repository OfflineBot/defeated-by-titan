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
//!
//! ## And a fifth way, which is `B-001` — `T-036a`
//!
//! The city can be right in every one of those four ways and still be **unreachable**: a body
//! without a [`BodyId`] cannot be hooked. A hook stores the stable id of its carrier and never
//! an `Entity` (`docs/multiplayer.md` rule 5), and the only place that hands those ids out is
//! `world::index::maintain_index`. While its body was empty, `vector::aim` reported
//! `body: None` for every hit in the running game and every shot ended as
//! `ReleaseReason::NoAnchor` — with all six tests above green. The four `t036a_*` tests at the
//! bottom measure the index itself: id, count, mask, and the removal report.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{
    AnchorSurface, Block, Body, BodyGone, BodyId, BodyMask, Cli, IdCounter, SpatialIndex,
};
use defeated_by_titan::world::index::mask_from;
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

/// The same real app, but with **exactly one fixed step per `update()`** — from the second
/// `update()` on.
///
/// Two things, both measured and neither obvious:
///
/// 1. [`built_world`] takes its step size from the wall clock. `FixedUpdate` — and with it
///    `world::index::maintain_index` — then runs zero times or five, depending on how busy
///    the machine is. Whoever measures the index without `TimeUpdateStrategy` measures the
///    clock (same reasoning as in `tests/vector_aiming.rs`).
/// 2. **The first `update()` runs no fixed step at all**, whatever the strategy says. The
///    fixed loop lags the frame loop by one: bevy's own test spells it out —
///    "Frame 0 / Fixed update should not have run yet"
///    (`bevy_time-0.19.0/src/lib.rs:262-268`), and the counter there is still 0 after frame 1.
///    So `Startup` alone leaves the index empty, and a test that measured after one `update()`
///    would report "no body has an id" **whether or not the maintainer works**. Hence two.
fn stepped_world() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<GoneLog>();
    app.add_systems(Last, collect_gone);
    app.update(); // Startup + PostStartup — and no fixed step yet
    app.update(); // the first simulation step: the index takes the city in
    app
}

/// Every [`BodyGone`] of the run, in order. Not state — a log.
///
/// A `MessageReader` in a test function would have its own cursor and find the buffer already
/// consumed; the same pattern stands in `tests/vector_hooks.rs`.
#[derive(Resource, Default)]
struct GoneLog(Vec<BodyGone>);

fn collect_gone(mut log: ResMut<GoneLog>, mut gone: MessageReader<BodyGone>) {
    log.0.extend(gone.read().copied());
}

/// Every body of the world as `(name, entity, id)`, sorted by name — a stable order, because
/// a test is a measurement and not a lottery.
fn bodies_with_id(app: &mut App) -> Vec<(String, Entity, Option<BodyId>)> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Name, Entity, Option<&BodyId>), With<Body>>();
    let mut all: Vec<(String, Entity, Option<BodyId>)> = q
        .iter(world)
        .map(|(n, e, id)| (n.to_string(), e, id.copied()))
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
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

#[test]
fn f003_the_street_is_narrower_than_the_houses_are_tall() {
    // ★ The one proportion the district lives or dies by, and the reason `layout.perimeter`
    // exists. The survey of the real walled town this district is modelled on gives
    // **street : ridge = 8.1 : 13 = 0.62 : 1** — the house is 1.6x as tall as the street is
    // wide, and that is what makes `d < H` (FIND-041) hold from one roof to the next.
    //
    // Before the closed block, this map put one 21 m box in the middle of a 28 m cell and
    // one cell in four stayed empty: the median gap from facade to facade was **21 m**
    // against an 8 m house — 2.6 : 1, a business park, and the reason the gantry lane had to
    // be invented (FIND-058).
    //
    // Measured here, not asserted from a comment: for every generated house, the gap to the
    // next generated house along +x and along +z whose other axis overlaps. That is exactly
    // what a street sample is.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();
    let houses = &plan[map.blocks.len()..];
    assert!(houses.len() > 100, "only {} generated houses — is this the district?", houses.len());

    let span = |c: f32, s: f32| (c - s * 0.5, c + s * 0.5);
    let mut gaps: Vec<f32> = Vec::new();
    let mut ratios: Vec<f32> = Vec::new();
    let mut broken = 0usize;
    for a in houses {
        for axis in 0..2 {
            let (a_lo, a_hi) = if axis == 0 {
                span(a.center_m.x, a.size_m.x)
            } else {
                span(a.center_m.z, a.size_m.z)
            };
            let (a_olo, a_ohi) = if axis == 0 {
                span(a.center_m.z, a.size_m.z)
            } else {
                span(a.center_m.x, a.size_m.x)
            };
            let mut best: Option<&BlockPlan> = None;
            for b in houses {
                let (b_lo, _) = if axis == 0 {
                    span(b.center_m.x, b.size_m.x)
                } else {
                    span(b.center_m.z, b.size_m.z)
                };
                let (b_olo, b_ohi) = if axis == 0 {
                    span(b.center_m.z, b.size_m.z)
                } else {
                    span(b.center_m.x, b.size_m.x)
                };
                // Facing each other means: the other axis really overlaps. Two houses that
                // share a corner are not a street.
                if b_lo < a_hi - 1e-3 || a_olo >= b_ohi - 1e-3 || b_olo >= a_ohi - 1e-3 {
                    continue;
                }
                let take = match best {
                    None => true,
                    Some(k) => {
                        let (k_lo, _) = if axis == 0 {
                            span(k.center_m.x, k.size_m.x)
                        } else {
                            span(k.center_m.z, k.size_m.z)
                        };
                        b_lo < k_lo
                    }
                };
                if take {
                    best = Some(b);
                }
            }
            let Some(b) = best else { continue };
            let (b_lo, _) = if axis == 0 {
                span(b.center_m.x, b.size_m.x)
            } else {
                span(b.center_m.z, b.size_m.z)
            };
            let gap = b_lo - a_hi;
            // A gap wider than the hook can swing over is not a street, it is a field: the
            // wall boulevard, the field outside, the market square. Those are deliberate
            // (see the two dead zones below) and they must not be averaged into the street.
            // Below 1 m it is a **party wall**, not a street — two row houses of the same
            // run touch, and counting that as a 0 m street would make the median meaningless
            // in the flattering direction. Above 25 m it is not a street either but a field:
            // the wall boulevard, the market square, the open country outside the gate.
            // Those are deliberate and they must not be averaged into the canyon.
            if gap > 25.0 {
                // Not a street but a field, and every one of these is a **hole in the
                // frontage**: the house looks out at open ground instead of at the house
                // opposite. Counted, because this — and not the width of the street — is
                // what the old layout really got wrong.
                broken += 1;
                continue;
            }
            if gap < 1.0 {
                // A party wall, not a street. Counting two touching row houses as a 0 m
                // street would make the median meaningless in the flattering direction.
                continue;
            }
            gaps.push(gap);
            ratios.push(gap / a.size_m.y.min(b.size_m.y));
        }
    }

    let median = |v: &mut Vec<f32>| {
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    };
    assert!(gaps.len() > 200, "only {} street samples", gaps.len());
    let n = gaps.len();
    let gap_m = median(&mut gaps);
    let ratio = median(&mut ratios);
    eprintln!(
        "{} generated houses · {n} street samples, median gap {gap_m:.2} m, median \
         street : ridge {ratio:.2} : 1 · {broken} facades look at open ground",
        houses.len()
    );
    assert!(
        broken * 3 < n,
        "{broken} of {} samples are a facade with no facade opposite — a frontage with that \
         many holes in it is not a street, and the arc has nothing to run between",
        broken + n
    );

    assert!(
        gap_m <= 9.0,
        "median gap from facade to facade is {gap_m:.2} m — the survey says 8.1"
    );
    assert!(
        ratio < 1.0,
        "median street : ridge is {ratio:.2} : 1 — above 1.0 the rope has no arc between two \
         houses, and the map needs scaffolding again (FIND-058)"
    );
}

#[test]
fn f003_no_anchorable_block_has_another_block_sitting_on_its_roof_centre() {
    // ★ `FIND-059`: 28 of ashgate's tagged row houses were built as body + a **narrower,
    // untagged ridge cap** standing exactly on the centre of the body's roof. The highest
    // point a player sees and aims at therefore answered `NoAnchor`, and
    // `tests/vector_aiming.rs::f002_every_tagged_surface_in_the_map_is_reachable_by_free_aiming`
    // had to be pinned to the graybox because of it.
    //
    // That test measures with a real ray and can only run where its premise holds. This one
    // is the premise, as geometry, over whatever map `current` names: **a tagged surface
    // nothing can reach from above is a lie in the map**, and a lie you can only find by
    // playing is one nobody finds.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();

    let mut capped: Vec<String> = Vec::new();
    for a in plan.iter().filter(|k| k.anchorable) {
        let a_top = a.center_m.y + a.size_m.y * 0.5;
        for b in &plan {
            if std::ptr::eq(a, b) {
                continue;
            }
            let half = b.size_m * 0.5;
            // Straight over the centre of the roof, and higher than it.
            let over = (b.center_m.x - a.center_m.x).abs() < half.x
                && (b.center_m.z - a.center_m.z).abs() < half.z
                && b.center_m.y + half.y > a_top + 1e-3;
            if over {
                capped.push(format!("{} is capped by {}", a.name, b.name));
                break;
            }
        }
    }
    assert!(
        capped.is_empty(),
        "{} of {} anchorable blocks cannot be hooked at their roof centre: {capped:#?}",
        capped.len(),
        plan.iter().filter(|k| k.anchorable).count()
    );
}

// ---------------------------------------------------------------------------------------
// T-036a / B-001 — the index, and the ids without which nothing can be hooked
// ---------------------------------------------------------------------------------------

#[test]
fn t036a_every_body_gets_exactly_one_id() {
    // ★ `B-001`. Red for exactly the state the game shipped in: `maintain_index` had an empty
    // body, so **not one entity in the world carried a `BodyId`** — 0 instead of 79 — and
    // `AimPoint.body` was `None` on every single hit.
    //
    // Everything here is read out of the plan and out of `IdCounter`, never out of a literal:
    // the number of blocks is a question for `maps.ron`.
    let planned = plan().len();
    let mut app = stepped_world();

    let all = bodies_with_id(&mut app);
    assert_eq!(all.len(), planned, "{} bodies in the world, {planned} planned", all.len());

    let missing: Vec<&String> = all.iter().filter(|(_, _, id)| id.is_none()).map(|(n, _, _)| n).collect();
    assert!(
        missing.is_empty(),
        "{} of {planned} bodies carry no `BodyId`. `world::index::maintain_index` is the only \
         place that hands them out, and a hook hangs on an id and never on an `Entity` — with \
         none, every shot ends as `ReleaseReason::NoAnchor`. First: {:?}",
        missing.len(),
        &missing[..missing.len().min(3)]
    );

    // Consecutive out of the counter, not random: two machines have to arrive at the same
    // numbering (`docs/multiplayer.md` rule 5), and a gap is a body that got its id twice.
    let mut ids: Vec<u32> = all.iter().filter_map(|(_, _, id)| id.map(|b| b.0)).collect();
    ids.sort_unstable();
    let handed_out = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), handed_out, "two bodies share one `BodyId`");
    assert_eq!(
        ids,
        (1..=planned as u32).collect::<Vec<u32>>(),
        "the ids are not the consecutive 1..={planned} out of `IdCounter`"
    );
    assert_eq!(
        app.world().resource::<IdCounter>().body,
        planned as u32,
        "`IdCounter.body` and the number of bodies disagree"
    );

    // And the index knows exactly those bodies — not fewer (a body that nothing can find) and
    // not more (an id inserted twice under two entries).
    assert_eq!(
        app.world().resource::<SpatialIndex>().len(),
        planned,
        "the index holds {} of {planned} bodies",
        app.world().resource::<SpatialIndex>().len()
    );

    // Letting it keep running hands out nothing a second time. Red the day the `Without<BodyId>`
    // filter or the `Commands` insert is dropped: then the counter climbs by 79 per tick.
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().resource::<IdCounter>().body, planned as u32, "ids are handed out again every tick");
    assert_eq!(app.world().resource::<SpatialIndex>().len(), planned, "the index grows every tick");
}

#[test]
fn t036a_the_index_carries_the_anchorable_bit_from_the_file() {
    // The hook asks the mask and nothing else (`vector::aim`: `mask.contains(ANCHORABLE)`).
    // If the bit in the index does not come out of the same `anchorable:` in `maps.ron` that
    // `AnchorSurface` comes out of, the cyan gizmo outlines one set of blocks and the hook
    // catches on another — and you cannot see that in a screenshot.
    //
    // `mask_from` is used, not a second translation: one place, or one of the two goes stale.
    let plan = plan();
    let by_name: std::collections::BTreeMap<&str, &BlockPlan> =
        plan.iter().map(|p| (p.name.as_str(), p)).collect();
    let mut app = stepped_world();

    let all = bodies_with_id(&mut app);
    let index = app.world().resource::<SpatialIndex>();

    let mut wrong: Vec<String> = Vec::new();
    let mut anchorable = 0usize;
    for (name, _, id) in &all {
        let id = id.unwrap_or_else(|| panic!("{name} carries no BodyId — see t036a_every_body_gets_exactly_one_id"));
        let entry = index
            .body(id)
            .unwrap_or_else(|| panic!("{name} (id {}) is not in the index", id.0));
        let want = by_name
            .get(name.as_str())
            .unwrap_or_else(|| panic!("{name} stands in the world but not in the plan"));
        let expected = mask_from(want.solid, want.anchorable);
        if entry.mask != expected {
            wrong.push(format!("{name}: mask {:?} in the index, {expected:?} from the file", entry.mask));
        }
        if entry.mask.contains(BodyMask::ANCHORABLE) {
            anchorable += 1;
            assert!(want.anchorable, "{name} is anchorable in the index and not in the file");
        } else {
            assert!(!want.anchorable, "{name} is anchorable in the file and not in the index");
        }
    }
    assert!(wrong.is_empty(), "{} block(s) with the wrong mask: {wrong:#?}", wrong.len());

    // Both kinds, and the numbers come out of the plan. In the graybox that is 63 of 79 —
    // written nowhere here, because a map change must not turn into a test change.
    let expected_anchorable = plan.iter().filter(|p| p.anchorable).count();
    assert_eq!(anchorable, expected_anchorable, "anchorable bodies in the index vs. in the file");
    assert!(anchorable > 0, "not a single anchorable body in the index");
    assert!(
        anchorable < all.len(),
        "all {} bodies are anchorable — then the mask decides nothing",
        all.len()
    );

    // The hull the index carries is the hull of the body, and it comes from the file's half
    // edge. A factor of 2 here is a hook that catches in mid-air.
    for (name, _, id) in &all {
        let entry = index.body(id.expect("id")).expect("in the index");
        let want = by_name[name.as_str()];
        assert_eq!(entry.half_size_m, want.size_m * 0.5, "{name}: half size in the index");
        assert_eq!(entry.center_m, want.center_m, "{name}: center in the index");
    }
}

#[test]
fn t036a_a_removed_body_is_struck_out_and_reported() {
    // `vector::hook` releases every hook hanging on the carrier when `BodyGone` arrives, and
    // falls back on `index.body(id) == None` if the message was lost. Both have to hold, or a
    // rope stays taut on a house that no longer exists.
    //
    // Red when `on_body_removed` is empty (nothing lands in the mailbox) AND when
    // `maintain_index` never collects it (the mailbox fills up and nobody hears).
    let mut app = stepped_world();
    let all = bodies_with_id(&mut app);
    let (name, entity, id) = all.first().cloned().expect("the city has bodies");
    let id = id.expect("every body carries an id — see t036a_every_body_gets_exactly_one_id");

    let before = app.world().resource::<SpatialIndex>().len();
    assert!(app.world().resource::<SpatialIndex>().body(id).is_some(), "{name} is not in the index");
    app.world_mut().resource_mut::<GoneLog>().0.clear();

    app.world_mut().entity_mut(entity).despawn();
    app.update(); // the next fixed step: the maintainer empties the mailbox

    let reported: Vec<BodyId> = app.world().resource::<GoneLog>().0.iter().map(|m| m.body).collect();
    assert!(
        reported.contains(&id),
        "{name} (id {}) was despawned, and `BodyGone` reported {reported:?} — the hooks \
         hanging on it never let go",
        id.0
    );
    assert_eq!(
        app.world().resource::<SpatialIndex>().body(id),
        None,
        "{name} was despawned and is still in the index — a hook would keep anchoring on it"
    );
    assert_eq!(
        app.world().resource::<SpatialIndex>().len(),
        before - 1,
        "one body gone, {} still in the index (was {before})",
        app.world().resource::<SpatialIndex>().len()
    );

    // And it is reported **once**, not once per tick from here on.
    let after_first = app.world().resource::<GoneLog>().0.len();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<GoneLog>().0.len(),
        after_first,
        "`BodyGone` is repeated every tick — the mailbox is not emptied"
    );
}

#[test]
fn t036a_a_body_spawned_late_is_taken_in_and_stands_right_one_tick_later() {
    // The city is spawned in `Startup` and its `GlobalTransform` is propagated in
    // `PostStartup` (`bevy_transform-0.19.0/src/plugins.rs:27-28`), so it is in the index at
    // its true position from the first tick — `t036a_the_index_carries_the_anchorable_bit...`
    // measures that centre by centre.
    //
    // A body spawned **later** is a different case, and this is the one `F-029` will walk
    // into. `maintain_index` reads the `GlobalTransform` (it has to: for a child body the
    // `Transform` is local), and that is only propagated in `PostUpdate` — one stage AFTER
    // `RunFixedMainLoop`. So a body that comes into being between two updates is taken into
    // the index in the next fixed step **at the origin**, and moves to its real place one tick
    // later, when `Changed<GlobalTransform>` catches it.
    //
    // That is a measurement, not an excuse: whoever hangs an anchor on a limb spawned this
    // frame has it at (0,0,0) for one tick. Written down here so the next person finds it as a
    // number instead of as a hook that catches in mid-air.
    let mut app = stepped_world();
    let before = app.world().resource::<SpatialIndex>().len();
    let place = Vec3::new(120.0, 6.0, 120.0);

    let e = app
        .world_mut()
        .spawn((
            Name::new("t036a_late_block"),
            Body { half_size_m: Vec3::splat(3.0), mask: mask_from(true, true) },
            Transform::from_translation(place),
        ))
        .id();
    app.update();

    // Taken in immediately, with an id and with its mask — that part is not deferred.
    let id = *app.world().get::<BodyId>(e).expect("a body spawned late also gets an id");
    let first = app
        .world()
        .resource::<SpatialIndex>()
        .body(id)
        .expect("and it is in the index in the very step it appears");
    assert!(first.mask.contains(BodyMask::ANCHORABLE), "the mask is right from the first tick");
    assert_eq!(app.world().resource::<SpatialIndex>().len(), before + 1, "exactly one body more");

    // And at the latest one tick later it stands where it was spawned. Red the day the
    // `Changed<GlobalTransform>` loop is dropped: then it stays at the origin forever.
    app.update();
    let settled = app.world().resource::<SpatialIndex>().body(id).expect("still there");
    assert_eq!(settled.center_m, place, "the index never caught up with the world position");
    assert_eq!(settled.half_size_m, Vec3::splat(3.0));
    assert_eq!(app.world().resource::<SpatialIndex>().len(), before + 1, "and not inserted twice");
}

