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
//! 4. A block silently loses the tag the file gives it, or keeps one the file took away — the
//!    cyan gizmo then outlines a different city than the one the hook catches on, and no
//!    screenshot shows it.
//!
//! ⚠️ Point 4 used to read *"everything is anchorable — then `F-003` checks nothing"*, and it
//! was a census: **not** every surface may be tagged, or the criterion is unfalsifiable. The
//! user overruled that on 2026-08-13 — *„es ist extrem wichtig dass man wirklich überall sein
//! seil festmachen kann. also überall! ohne ausnahmen!"* — and minutes later left the mechanism
//! standing: *„es soll später auch stark vereinzelt dinge geben die man nicht anchorn kann.
//! aber sehr wenig! also kann der check drin bleiben"* (`docs/NEXT.md` §1D item 10 and its
//! clarification). So it is a **default flip**, not a removal, and the census did not die, it
//! moved: on the shipped map an untagged block is now a *listed* exception
//! (`f003_an_unanchorable_block_is_a_listed_exception_and_the_fixture_keeps_both_kinds`), and
//! the untagged path stays falsifiable on the `graybox` fixture, which keeps its untagged
//! blocks for the eight aiming tests pinned to it (`docs/FINDINGS.md` FIND-061).
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
use defeated_by_titan::data::{Model, ModelSource};
use defeated_by_titan::shared::ModelName;
use defeated_by_titan::render::model::{feet_offset_m, fit_to_class};
use defeated_by_titan::world::map::{plan_blocks, BlockPlan, DRESSING, RUBBLE_KIT, RUIN_KIT};
use std::collections::BTreeMap;
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

/// The **walls** of the generated houses — `house_<lot>_<i>`, without their roof caps.
///
/// A generated house has been two cuboids since 2026-08-12 (`world::map`), and the two are
/// different measurements: the wall is what a street is measured between, the ridge is what
/// `d < H` is measured against. A test that lumps them together measures neither.
fn walls(plan: &[BlockPlan]) -> Vec<&BlockPlan> {
    plan.iter().filter(|k| k.name.starts_with("house_")).collect()
}

/// Every roof cap of a wall, lowest first — `roof_<lot>_<i>` and, since `layout.roof_steps`,
/// `roof_<lot>_<i>_<s>` above it.
///
/// ⚠️ The prefix is matched with its separator (`roof_1_2_`) and never bare, or `roof_1_20`
/// would count as a cap of `house_1_2` and every ridge in the district would come out wrong by
/// a whole house.
fn caps_of<'a>(plan: &'a [BlockPlan], wall: &BlockPlan) -> Vec<&'a BlockPlan> {
    let want = wall.name.replacen("house_", "roof_", 1);
    let stepped = format!("{want}_");
    plan.iter().filter(|k| k.name == want || k.name.starts_with(&stepped)).collect()
}

/// Which wall a cap belongs to — `roof_<lot>_<i>[_<s>]` -> `house_<lot>_<i>`.
fn wall_name_of_cap(cap: &BlockPlan) -> String {
    let mut parts = cap.name.split('_');
    parts.next(); // "roof"
    let lot = parts.next().unwrap_or_default();
    let i = parts.next().unwrap_or_default();
    format!("house_{lot}_{i}")
}

/// Where a generated house stands — the top of the terrace under it, `0` on flat ground.
fn base_m(wall: &BlockPlan) -> f32 {
    wall.center_m.y - wall.size_m.y * 0.5
}

/// Total height of a generated house, cap included — the number the hook hangs from.
///
/// ⚠️ Measured **from its own base and not from `y = 0`** since 2026-08-13: a house on a 3.6 m
/// terrace is not a 15 m house, and every proportion in this file (`d < H`, street : ridge, the
/// residential band) is about the building and not about where the ground happens to be.
fn ridge_m(plan: &[BlockPlan], wall: &BlockPlan) -> f32 {
    let top = caps_of(plan, wall)
        .iter()
        .map(|r| r.center_m.y + r.size_m.y * 0.5)
        .fold(wall.center_m.y + wall.size_m.y * 0.5, f32::max);
    top - base_m(wall)
}

/// Which block a generated house belongs to — the `<lot>` of `house_<lot>_<i>`.
///
/// Two houses of the same lot are neighbours in one ring and what lies between them is an
/// alley; two houses of different lots face each other across a street. The distinction is
/// the whole point of the layout, so it is read off the name and not guessed from a width.
fn lot_of(house: &BlockPlan) -> &str {
    // ⚠️ Read as "the second field of the name", not as "what follows `house_`". Since
    // 2026-08-19 a generated building is `house_`, `ruin_` or `rubble_` (`maps.ron:
    // layout.damage`), and the old `trim_start_matches("house_")` returned the *prefix* for
    // the other two — every ruin in the district then counted as one lot called „ruin", and a
    // test that groups by block silently measured three buckets instead of two hundred.
    house.name.split('_').nth(1).unwrap_or("")
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

/// The blocks of the **shipped** map that are deliberately *not* anchorable — name, and the
/// reason a rope may not hold there.
///
/// **Empty on 2026-08-13, and that is a decision rather than an accident.** The whole ashgate
/// section of `maps.ron` was flipped that day: 133 `anchorable: false` became `true`, and the
/// district ships 100 % hookable because the user asked for exactly that. The five reasons the
/// old default was built on — the ground slab, the canal, the gate columns, interior faces and
/// the untagged wall the aim ray was proved against (`docs/NEXT.md` §1D) — all lost to one
/// sentence, and the last of them survives on the `graybox` fixture instead.
///
/// ⚠️ **The list is the mechanism, not a decoration.** He allowed the exception back —
/// *„stark vereinzelt … aber sehr wenig"* — so this must not forbid it; it must make it
/// *visible*. An untagged block that is not on this list fails the test, and putting it on the
/// list means writing down why. That turns a leak into a decision somebody signed.
const SHIPPED_UNANCHORABLE: &[(&str, &str)] = &[];

/// „sehr wenig" as a number, and it is small on purpose.
///
/// An exception is a hand-placed judgement about **one** surface. Eight of them is already a
/// policy wearing a list, and a policy about what may be hooked is the user's call
/// (`docs/QUESTIONS.md`), not a constant somebody bumps on the way past. Whoever needs the
/// ninth has stopped making exceptions and started re-introducing the old default.
const MAX_SHIPPED_UNANCHORABLE: usize = 8;

#[test]
fn f003_every_anchor_tag_in_the_world_comes_from_the_file_and_the_mask_agrees() {
    // This was `f003_not_every_surface_is_anchorable` (K4) until 2026-08-13, and its **name was
    // the premise the user removed** — see the ⚠️ at the top of this file. The census moved to
    // `f003_an_unanchorable_block_is_a_listed_exception_and_the_fixture_keeps_both_kinds`.
    //
    // What is left is the half that never depended on the mix, and it is the half that catches
    // the expensive bug: **the world carries exactly the tags the file hands it, and the marker
    // and the mask are the same state.** A block that quietly loses `AnchorSurface` on the way
    // out of `maps.ron` is invisible — it looks like a house, it collides like a house, and the
    // rope goes through it. Red for that, red for the converse (a tag the file never gave), and
    // red when `AnchorSurface` and `BodyMask::ANCHORABLE` drift apart, which is a cyan gizmo
    // drawn around blocks the hook does not catch.
    let mut app = built_world();
    let built = built_blocks(&mut app);
    let plan = plan();
    let from_file: std::collections::BTreeMap<&str, bool> =
        plan.iter().map(|p| (p.name.as_str(), p.anchorable)).collect();

    assert!(
        built.iter().any(|(_, _, _, a)| *a),
        "not a single anchor surface in the built city — nothing in this district can be hooked"
    );

    let mut drifted: Vec<String> = Vec::new();
    for (name, _, _, marker) in &built {
        let want = *from_file
            .get(name.as_str())
            .unwrap_or_else(|| panic!("{name} stands in the world but not in the plan"));
        if *marker != want {
            drifted.push(format!("{name}: AnchorSurface {marker}, `maps.ron` says {want}"));
        }
    }
    assert!(
        drifted.is_empty(),
        "{} block(s) carry a different tag in the world than in the file: {drifted:#?}",
        drifted.len()
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
fn f003_an_unanchorable_block_is_a_listed_exception_and_the_fixture_keeps_both_kinds() {
    // ★ The guard the map flip needs and did not have. 100 % anchorable is now a **decision**,
    // and a decision that nothing measures decays into a habit: the next hand that writes
    // `anchorable: false` for a reason it does not write down has silently taken a surface away
    // from the player, and the only place that surfaces is a rope that does not catch.
    //
    // Two halves, and neither is a census of the old kind:
    //
    // 1. **On the shipped map an exception has to be named.** Not forbidden — the user allows
    //    it — but listed in `SHIPPED_UNANCHORABLE` with a reason, and capped at
    //    `MAX_SHIPPED_UNANCHORABLE`. That makes a rare exception cheap and a policy expensive,
    //    which is exactly the shape of what he asked for.
    // 2. **The fixture keeps both kinds.** The old K4 worry — "everything tagged, so the
    //    untagged path can no longer be falsified" — is still true, it just does not apply to
    //    the played map any more. It applies to `graybox`, where the eight aiming tests of
    //    `tests/vector_aiming.rs` are pinned (`docs/FINDINGS.md` FIND-061) and where
    //    `f002_an_untagged_wall_in_front_of_a_roof_is_not_hookable_and_not_transparent` needs a
    //    surface that is deliberately untagged to point at. Flatten the fixture too and eight
    //    tests stay green while proving nothing.
    let d = data();
    let plan = plan();

    // ---- 1. the shipped map --------------------------------------------------------------
    let untagged: Vec<&str> =
        plan.iter().filter(|p| !p.anchorable).map(|p| p.name.as_str()).collect();
    let listed: std::collections::BTreeSet<&str> =
        SHIPPED_UNANCHORABLE.iter().map(|(name, _)| *name).collect();

    let unlisted: Vec<&&str> = untagged.iter().filter(|n| !listed.contains(**n)).collect();
    assert!(
        unlisted.is_empty(),
        "{} block(s) of the shipped map {:?} are not anchorable and stand on no list: {unlisted:#?}\n\
         The user, 2026-08-13: „es ist extrem wichtig dass man wirklich überall sein seil \
         festmachen kann. also überall!\" — and „stark vereinzelt … aber sehr wenig! also kann \
         der check drin bleiben\". An exception is allowed; an unexplained one is not. Add it to \
         `SHIPPED_UNANCHORABLE` with the reason a rope may not hold there.",
        unlisted.len(),
        d.maps.current
    );

    // The converse, which is what stops the list from rotting into folklore: an entry that
    // names nothing, or names something that is anchorable again, is a reason nobody has read
    // since it stopped being true.
    let present: std::collections::BTreeSet<&str> =
        plan.iter().map(|p| p.name.as_str()).collect();
    for (name, why) in SHIPPED_UNANCHORABLE {
        assert!(
            present.contains(name),
            "`SHIPPED_UNANCHORABLE` lists {name:?}, which is not in the shipped map at all"
        );
        assert!(
            untagged.contains(name),
            "`SHIPPED_UNANCHORABLE` lists {name:?} as an exception, but it is anchorable — \
             stale entry, and the next reader will believe the reason"
        );
        assert!(!why.trim().is_empty(), "the exception {name:?} carries no reason");
    }

    // And few. A generator-wide regression — `layout.anchorable_fraction` slipping off 1.0 —
    // arrives here as hundreds of unlisted `house_*`, which the check above catches first; this
    // one catches the slower way of losing the same argument, one signed entry at a time.
    assert!(
        untagged.len() <= MAX_SHIPPED_UNANCHORABLE,
        "{} of {} blocks in the shipped map are not anchorable — „sehr wenig\" was the word, \
         and {MAX_SHIPPED_UNANCHORABLE} is where a list stops being a set of exceptions. \
         Raising the cap is a question for the user, not an edit.",
        untagged.len(),
        plan.len()
    );

    // ---- 2. the fixture ------------------------------------------------------------------
    const FIXTURE: &str = "graybox";
    assert_ne!(
        d.maps.current, FIXTURE,
        "the fixture is the shipped map — then half 1 and half 2 of this test contradict each \
         other, and the aiming tests measure the district instead of their fixture"
    );
    let fixture = d
        .maps
        .maps
        .get(FIXTURE)
        .unwrap_or_else(|| panic!("`maps.ron` has no {FIXTURE:?} — the aiming fixture is gone"));
    let tagged = fixture.blocks.iter().filter(|b| b.anchorable).count();
    let bare = fixture.blocks.len() - tagged;
    assert!(
        tagged > 0 && bare > 0,
        "{FIXTURE} carries {tagged} anchorable and {bare} untagged blocks — both kinds have to \
         exist there or `F-003`'s \"no hook on untagged parts\" is unfalsifiable everywhere in \
         this repository, and the eight fixture tests in `tests/vector_aiming.rs` go green \
         without deciding anything"
    );
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
    // `maps.ron` says: the vertical comes from the landmarks, and the residential band stays
    // inside `scale.ron: architecture.heights_m`. Red when somebody pulls the band up —
    // including by adding a roof **on top of** the rolled height instead of cutting it out of
    // it, which is exactly the tempting way to build a roof and would put every ridge 2 to 4
    // metres above the user's own ceiling without a single test noticing.
    let d = data();
    let map = d.current_map().expect("current map");
    let r = &map.layout;
    let plan = plan();
    let houses = walls(&plan);
    assert!(!houses.is_empty(), "no generated house at all");

    // The one height that is deliberately outside the band, and it is a figure of the user's:
    // `scale.ron: architecture.heights_m[layout.tall_height_key]`. `layout.tall_fraction` of
    // the houses are built to it — `Q-036`, answered.
    let tall_m = d.scale.architecture.heights_m[&r.tall_height_key];
    let ceiling_m = map.terrain.step_m * (map.terrain.levels.saturating_sub(1)) as f32;
    let mut tall = 0usize;

    for house in &houses {
        let ridge = ridge_m(&plan, house);
        let in_band = (r.min_height_m..=r.max_height_m).contains(&ridge);
        if (ridge - tall_m).abs() < 1e-3 {
            tall += 1;
        }
        assert!(
            in_band || (ridge - tall_m).abs() < 1e-3,
            "{}: a ridge of {ridge} m is neither in {}..={} nor the tall class ({tall_m} m) \
             (maps.ron: layout)",
            house.name,
            r.min_height_m,
            r.max_height_m
        );
        assert!(house.size_m.y > 0.0, "{}: the roof ate the whole wall", house.name);
        // ⚠️ Until 2026-08-13 this line read "stands on y = 0" — and it had to, there was no
        // terrain. What it means now is **stands on a terrace**: the base is a whole number of
        // `terrain.step_m` and never above the ceiling the file allows. That is stricter, not
        // weaker: a house half a step off its plateau floats or sinks, and neither shows up in
        // the picture.
        let base = base_m(house);
        assert!(base >= 0.0 && base <= ceiling_m + 1e-4, "{}: stands at {base} m", house.name);
        if map.terrain.step_m > 0.0 {
            let levels = base / map.terrain.step_m;
            assert!(
                (levels - levels.round()).abs() < 1e-3,
                "{}: stands at {base} m, which is {levels} steps of {} m — not on a terrace",
                house.name,
                map.terrain.step_m
            );
        } else {
            assert!(base.abs() < 1e-4, "{}: a flat map, and the house stands at {base}", house.name);
        }
        // And the caps really sit ON the wall, each one on the one below it.
        let mut under = house.center_m.y + house.size_m.y * 0.5;
        for cap in caps_of(&plan, house) {
            assert!(
                (cap.center_m.y - cap.size_m.y * 0.5 - under).abs() < 1e-3,
                "{}: the cap starts at {} m, the one below it ends at {under} m",
                cap.name,
                cap.center_m.y - cap.size_m.y * 0.5
            );
            // A **gable**: pulled in across the short side, flush with the gable walls at
            // the ends. Both axes pulled in is a stepped pyramid, and 900 of those are
            // further from a town than the flat lid they replaced (`world::map`, and
            // `docs/images/f003-roofscape.png` is what made the case).
            let short_in = if house.size_m.x >= house.size_m.z {
                cap.size_m.z < house.size_m.z && (cap.size_m.x - house.size_m.x).abs() < 1e-4
            } else {
                cap.size_m.x < house.size_m.x && (cap.size_m.z - house.size_m.z).abs() < 1e-4
            };
            assert!(
                short_in,
                "{}: {:?} on a {:?} house is not a stepped gable — a roof flush with the wall \
                 on both axes is a flat top, and one pulled in on both is a ziggurat",
                cap.name,
                cap.size_m,
                house.size_m
            );
            under = cap.center_m.y + cap.size_m.y * 0.5;
        }
    }

    // The converse: the tall class is really built, and it stays rare. Without this the
    // `||` above would be a licence rather than a measurement — `tall_fraction: 0.0` would
    // pass it, and so would a district where every house is 18 m.
    if r.tall_fraction > 0.0 {
        let want = r.tall_fraction * houses.len() as f32;
        assert!(
            tall as f32 > want * 0.5 && (tall as f32) < want * 2.0,
            "{tall} of {} houses are of the tall class, and the file asks for about {want:.0} \
             (maps.ron: layout.tall_fraction = {})",
            houses.len(),
            r.tall_fraction
        );
    }
}

#[test]
fn f003_the_district_has_a_skyline_and_not_one_flat_top() {
    // ★ The user's verdict of 2026-08-12, as a measurement: *„keine unterschiedliche höhen!"*.
    //
    // The band alone does not catch this. The version he judged rolled every house out of
    // 8.0..11.5 independently — inside the window, spread by any per-house measure, and from
    // twenty metres up a flat mosaic, because white noise averages out over a hundred metres.
    // So what is measured here is **relief at two scales**: neighbours differ, AND blocks
    // differ from blocks. The second one is the one that was missing.
    let d = data();
    let map = d.current_map().expect("current map");
    let r = &map.layout;
    let plan = plan();
    let houses = walls(&plan);

    let ridges: Vec<f32> = houses.iter().map(|h| ridge_m(&plan, h)).collect();
    let lo = ridges.iter().copied().fold(f32::MAX, f32::min);
    let hi = ridges.iter().copied().fold(f32::MIN, f32::max);
    let band = r.max_height_m - r.min_height_m;
    assert!(
        hi - lo > band * 0.8,
        "every ridge in the district is between {lo:.2} and {hi:.2} m — {:.2} m of the \
         {band:.2} m band is unused",
        band - (hi - lo)
    );

    // Scale 2: the mean ridge PER BLOCK. `house_<lot>_<i>` — the lot is the block.
    let mut per_block: std::collections::BTreeMap<&str, (f32, u32)> = Default::default();
    for (h, ridge) in houses.iter().zip(&ridges) {
        let lot = h.name.trim_start_matches("house_").split('_').next().expect("house_<lot>_<i>");
        let e = per_block.entry(lot).or_insert((0.0, 0));
        e.0 += ridge;
        e.1 += 1;
    }
    let mut means: Vec<f32> = per_block.values().map(|(s, n)| s / *n as f32).collect();
    means.sort_by(f32::total_cmp);
    assert!(means.len() > 50, "only {} blocks — is this the district?", means.len());
    let p10 = means[means.len() / 10];
    let p90 = means[means.len() * 9 / 10];
    eprintln!(
        "{} houses in {} blocks · ridge {lo:.2}..{hi:.2} m · block mean p10 {p10:.2} m, \
         p90 {p90:.2} m, relief {:.2} m",
        houses.len(),
        means.len(),
        p90 - p10
    );
    // ⚠️ 1.5 -> 3.0 m on 2026-08-13, and the raise is the acceptance criterion of the round.
    // The user, having looked at the district built to the old number: *„lass es wie die echte
    // stadt aussehen! aktuell kann man es noch nicht erkennen!"*. 1.5 m of relief between the
    // tenth-highest and the tenth-lowest block is half a storey — measurable, and invisible.
    // The band alone cannot reach 3.0 (it is 5.0 m wide and two thirds of it are spent inside
    // one block); what reaches it is `layout.tall_fraction`, and that is the point of the
    // lever.
    assert!(
        p90 - p10 > 3.0,
        "the tenth-highest and the tenth-lowest block of the district differ by \
         {:.2} m — at that relief the town is one flat top from any distance, whatever the \
         individual houses do",
        p90 - p10
    );

    // Scale 1: inside one block, the neighbours are not all the same either.
    let inner = per_block
        .keys()
        .map(|lot| {
            let hs: Vec<f32> = houses
                .iter()
                .zip(&ridges)
                .filter(|(h, _)| h.name.starts_with(&format!("house_{lot}_")))
                .map(|(_, r)| *r)
                .collect();
            hs.iter().copied().fold(f32::MIN, f32::max) - hs.iter().copied().fold(f32::MAX, f32::min)
        })
        .filter(|d| *d > 0.5)
        .count();
    assert!(
        inner * 2 > per_block.len(),
        "only {inner} of {} blocks have houses that differ by more than half a metre",
        per_block.len()
    );
}

#[test]
fn f003_the_ground_is_stepped_and_not_one_flat_slab() {
    // ★ The user, 2026-08-13: *„adde verschiedene höhen vom boden her! lass es wie die echte
    // stadt aussehen! aktuell kann man es noch nicht erkennen!"*. Until that day this game had
    // no terrain at all — `maps.ron` placed one 700 x 700 m slab and every generated house in
    // the district stood on `y = 0`, so the number below was **0.00 m**.
    //
    // Measured on the **base of every generated house**, and that is the honest sample: it is
    // area-weighted over the built district (every house is one vote where houses are), it
    // needs nothing but the plan, and it is exactly the ground the player runs on between the
    // facades. The field's own cell heights are reported next to it and include the pinned
    // rows along the wall and the canal, which nobody walks a district on.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();
    let houses = walls(&plan);
    assert!(houses.len() > 100, "only {} generated houses — is this the district?", houses.len());

    let mut bases: Vec<f32> = houses.iter().map(|h| base_m(h)).collect();
    bases.sort_by(f32::total_cmp);
    let p10 = bases[bases.len() / 10];
    let p90 = bases[bases.len() * 9 / 10];

    let mut distinct: Vec<i64> = bases.iter().map(|b| (b * 1000.0).round() as i64).collect();
    distinct.sort_unstable();
    distinct.dedup();

    let (_, ground) = defeated_by_titan::world::map::terrain_of(&d, map);
    let cells = ground.field.heights_m();

    // The whole field, one digit per cell. It costs sixteen lines and it is the only way to
    // see **why** a number came out the way it did: every 0 is something hand-placed pinning
    // its cell, and the slope away from it is the distance transform (`docs/FINDINGS.md`
    // FIND-090). Reading the histogram instead sent this round down a wrong path twice.
    for iz in 0..ground.field.nz() as i32 {
        let row: String = (0..ground.field.nx() as i32)
            .map(|ix| char::from_digit(ground.field.level_at(ix, iz), 10).unwrap_or('?'))
            .collect();
        eprintln!("terrain row {iz:2}  {row}");
    }
    eprintln!(
        "ground under {} houses: p10 {p10:.2} m, p90 {p90:.2} m, relief {:.2} m · \
         {} distinct levels {distinct:?} · {} terrain cells, levels used {:?} · \
         {} terrace blocks",
        houses.len(),
        p90 - p10,
        distinct.len(),
        cells.len(),
        ground.field.levels_used(),
        ground.pads.len()
    );

    assert!(
        p90 - p10 >= 2.7,
        "the ground under the tenth-highest and the tenth-lowest house of the district \
         differs by {:.2} m — below 2.7 m the terracing is a rounding error and the district \
         is the flat slab it was before 2026-08-13",
        p90 - p10
    );
    assert!(
        distinct.len() >= 4,
        "the whole district stands on {} different ground heights ({distinct:?}) — two or \
         three terraces are a mistake in the map, not a landscape",
        distinct.len()
    );

    // And the invariant the stairs hang from, on the **real** field and not on a fixture:
    // no cell may be more than one level above the one next to it, or the flight that leads
    // up it is short by a riser and the terrace edge is a wall.
    let f = &ground.field;
    for iz in 0..f.nz() as i32 {
        for ix in 0..f.nx() as i32 {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                assert!(
                    f.drop_to(ix, iz, dx, dz) <= 1,
                    "cell ({ix},{iz}) at level {} stands {} levels over ({},{}) — the flight \
                     built for it is one riser deep and the rest is a cliff",
                    f.level_at(ix, iz),
                    f.drop_to(ix, iz, dx, dz),
                    ix + dx,
                    iz + dz
                );
            }
        }
    }

    // Every terrace really carries its stairs: a plateau with a falling neighbour and no
    // flight beside it is the wall this whole design exists to avoid.
    let flight = (map.terrain.step_m / map.terrain.stair_rise_m).round() as u32;
    let mut naked = 0usize;
    for iz in 0..f.nz() as i32 {
        for ix in 0..f.nx() as i32 {
            if f.level_at(ix, iz) == 0 {
                continue;
            }
            for (dx, dz, side) in [(-1, 0, 'w'), (1, 0, 'e'), (0, -1, 'n'), (0, 1, 's')] {
                if f.drop_to(ix, iz, dx, dz) == 0 {
                    continue;
                }
                for k in 1..flight {
                    let want = format!("terrace_{ix}_{iz}_{side}{k}");
                    if !ground.pads.iter().any(|p| p.name == want) {
                        naked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(naked, 0, "{naked} terrace edges fall away without a step built into them");

    // The risers themselves: nothing the player has to climb is taller than one step of the
    // stairs. Measured against the file, so widening `stair_rise_m` past what a 0.35 m capsule
    // can ride up shows here and not only in the run.
    for pad in &ground.pads {
        let top = pad.center_m.y + pad.size_m.y * 0.5;
        let level = top / map.terrain.stair_rise_m;
        assert!(
            (level - level.round()).abs() < 1e-3,
            "{}: its top is at {top} m, which is not a whole number of {} m risers",
            pad.name,
            map.terrain.stair_rise_m
        );
    }
}

#[test]
fn f003_the_terrain_did_not_cost_the_district_its_houses() {
    // ★ The other half of the acceptance, and the one that is easy to lose without noticing:
    // a terrace is a block, `plan_blocks` vetoes a generated house against every hand-placed
    // block, and a house lifted 3.6 m would fly straight over the 0.3 m aprons that exist to
    // delete it. Either mistake shows up as **fewer houses**, and a district that quietly lost
    // a tenth of itself still looks like a district in a screenshot.
    //
    // So the same map is planned twice — once as it ships, once with its terrain switched off
    // by the one number that switches it off — and the two counts are compared.
    let d = data();
    let map = d.current_map().expect("current map");
    let with = plan();

    let mut flat_map = map.clone();
    flat_map.terrain.levels = 1;
    flat_map.terrain.step_m = 0.0;
    let flat = plan_blocks(&d, &flat_map);

    let count = |p: &[BlockPlan]| p.iter().filter(|k| k.name.starts_with("house_")).count();
    let (a, b) = (count(&with), count(&flat));
    eprintln!("{a} houses with terrain, {b} without — {:.4}x", a as f32 / b as f32);
    // ⚠️ **A band and not a floor**, and the upper half is the one that caught something.
    // Tried on 2026-08-13: lift the veto box onto the terrace instead of standing it on
    // `y = 0`, and the count goes **up** — the 0.30 m aprons under the 60 m galleries stick
    // 0.05 m out of the ground, and a house raised onto a terrace flies clean over them. Every
    // roof they exist to delete comes back, and a one-sided `>= 0.98 * b` calls that a success.
    let ratio = a as f32 / b as f32;
    assert!(
        (0.98..=1.02).contains(&ratio),
        "{a} houses stand on the terraced district against {b} on the flat one ({ratio:.4}x) — \
         the ground either ate part of the city or lifted it over the aprons that are supposed \
         to keep it off the galleries"
    );

    // And the terrain really is what is being compared: the flat plan has no terrace in it,
    // the shipped one has many.
    let terraces = |p: &[BlockPlan]| p.iter().filter(|k| k.name.starts_with("terrace_")).count();
    assert_eq!(terraces(&flat), 0, "`levels: 1` still built terraces");
    assert!(terraces(&with) > 50, "only {} terraces in the district", terraces(&with));
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
    // ⚠️ Since 2026-08-12 the sample is split in two, and it has to be: the ring has
    // **alleys** in it now (`maps.ron: layout.perimeter.gap_fraction`), and an alley is 1 to
    // 3 m wide. Pooled into one median they would outnumber the street samples and report a
    // 2 m "street" — a number that passes this test while saying nothing about it. So:
    // 1..4 m is an alley and gets its own count, 4..25 m is a street and carries the
    // assertion, above 25 m is a field and is a hole in the frontage.
    // ⚠️ Since 2026-08-19 the sample is taken over [`facades`] and not over [`walls`], and the
    // **ratio** is taken over intact pairs only. Both halves are the fall of Ashgate arriving
    // in this measurement, and getting either one wrong makes the number a lie:
    //   * a **ruin** keeps its street-facing face exactly where the house's was, so it is
    //     still a facade and the gap to it is still a street width. Left out, every ruined
    //     house turned into a 43 m hole in the frontage — measured on the first run of the
    //     damage round: 201 of 486 samples „look at open ground", against 165 of 507 on
    //     the same district before it fell.
    //   * a **mound** is deliberately pushed past that line into the road, so counting it
    //     would report a street narrower than the one you can fly down.
    //   * the **canyon** (street : ridge) is a statement about the standing town: a 3 m stump
    //     across a 7 m street is a 2.3 : 1 clearing and it is *supposed* to be. Pooled in, the
    //     median would fail this test for the one thing the round was asked to build.
    let plan = plan();
    let houses = facades(&plan);
    assert!(houses.len() > 100, "only {} generated houses — is this the district?", houses.len());

    let span = |c: f32, s: f32| (c - s * 0.5, c + s * 0.5);
    let mounds = rubble(&plan);
    let mut gaps: Vec<f32> = Vec::new();
    let mut ratios: Vec<f32> = Vec::new();
    let mut alleys: Vec<f32> = Vec::new();
    let mut broken = 0usize;
    for a in &houses {
        for axis in 0..2 {
            let (_a_lo, a_hi) = if axis == 0 {
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
            for b in houses.iter().chain(mounds.iter()) {
                let b = *b;
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
            // **A mound is neither a street nor a hole.** It is the nearest thing opposite, so
            // this facade does not look at open ground — but it stands *in* the road
            // (`maps.ron: layout.damage.spill_m`), so the gap to it is not a street width
            // either. Counting it as a street would report a road narrower than the one you
            // can fly down; counting it as a hole would call a collapsed house an empty field.
            if b.name.starts_with("rubble_") {
                continue;
            }
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
            if lot_of(a) == lot_of(b) {
                // An alley between two houses of the SAME block. Not a street — but it is the
                // thing that stopped the ring from reading as one merged mass, so it is
                // counted rather than dropped.
                //
                // Split by **block and not by width**, and that is not a detail: a width
                // threshold moves every narrow street into the alley bucket and the street
                // median comes out flattering by half a metre. Measured 2026-08-12: 9.14 m
                // with a 4 m threshold, 7.62 m by block, on the same city.
                alleys.push(gap);
                continue;
            }
            gaps.push(gap);
            // Against the RIDGE, not against the wall: the roof cap carries the same anchor
            // bit as its house, so `d < H` (FIND-041) hangs from the cap. And only where both
            // sides still stand — see the header.
            if a.name.starts_with("house_") && b.name.starts_with("house_") {
                ratios.push(gap / ridge_m(&plan, a).min(ridge_m(&plan, b)));
            }
        }
    }

    let median = |v: &mut Vec<f32>| {
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    };
    assert!(gaps.len() > 200, "only {} street samples", gaps.len());
    assert!(ratios.len() > 100, "only {} of them stand between two intact houses", ratios.len());
    let n = gaps.len();
    let gap_m = median(&mut gaps);
    let ratio = median(&mut ratios);
    // ★ The other half of the user's verdict — *„häuser sind alle ineinander!"*. Red when
    // somebody takes `gap_fraction` back out: then every neighbour is a party wall again and
    // there is not one gap in the whole district you could see down.
    assert!(
        alleys.len() * 4 > n,
        "{} alleys against {n} streets — with that few gaps between neighbours the ring is \
         one merged mass again, which is exactly what got judged on 2026-08-12",
        alleys.len()
    );
    let alley_m = median(&mut alleys);
    eprintln!(
        "{} generated houses · {n} street samples, median gap {gap_m:.2} m, median \
         street : ridge {ratio:.2} : 1 · {} alleys, median {alley_m:.2} m · {broken} facades \
         look at open ground",
        houses.len(),
        alleys.len()
    );
    // ⚠️ **The ceiling moved from a third to two fifths on 2026-08-19, and only because the
    // district fell.** Measured on the same seed, the same test, three states:
    //
    // | district | facades | street samples | holes |
    // |---|---|---|---|
    // | cuboids, intact | 926 | 507 | 165 (32.5 %) |
    // | dressed, intact | 937 | 514 | 165 (32.1 %) |
    // | dressed, fallen | 898 | 498 | 187 (37.6 %) |
    //
    // Dressing changed nothing here; the fall did, and it had to: 45 houses are a mound now
    // and 290 are a stump narrower than the house it replaced, so the frontage really does
    // have more gaps in it. That is the deliverable, not a regression. What must not happen
    // is the *rest* of the frontage quietly going with it, which is what this still catches —
    // and the number that moved is written down rather than the assertion being widened until
    // it goes green.
    assert!(
        broken * 5 < n * 2,
        "{broken} of {} samples are a facade with no facade opposite — a frontage with that \
         many holes in it is not a street, and the arc has nothing to run between. A fallen \
         ring is allowed 40 % of them (measured 37.6 % on 2026-08-19); above that the ruin \
         has eaten the town instead of standing in it",
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
fn f003_the_hand_placed_blocks_that_have_a_model_in_the_drop_wear_it() {
    // 🔴 `FIND-132`, on the frames the user was shown: *"the placed blocks are still bare
    // cuboids standing among dressed houses — one grey monolith in the middle of the district,
    // a navy box beside a row of houses, the wall as a flat grey mass. Only GENERATED houses
    // are dressed; `maps.ron`'s 215 placed blocks wear nothing, and the mixture is most of
    // what reads as `die map passt nicht`."*
    //
    // `world::map::placed_dress_for` is the hop that was missing. This test is both halves of
    // it: what wears a model wears one that **exists as a file** and **fits its own collider**
    // — and there is at least one, so deleting the table cannot make the test greener.
    let d = data();
    let plan = plan();
    let placed: Vec<&BlockPlan> = plan.iter().filter(|k| k.name.starts_with("block_")).collect();
    assert!(placed.len() > 100, "only {} placed blocks — is this ashgate?", placed.len());

    let dressed: Vec<&BlockPlan> = placed.iter().copied().filter(|k| k.model.is_some()).collect();
    assert!(
        !dressed.is_empty(),
        "not one of the {} hand-placed blocks wears a model, although the drop has a file for \
         the market stalls and one for the gas bottles (world::map::PLACED_DRESSING)",
        placed.len()
    );

    for b in &dressed {
        let name = b.model.expect("filtered");
        let model = d
            .model(name)
            .unwrap_or_else(|| panic!("{}: wears {name:?}, which is not in art.ron", b.name));
        let ModelSource::Gltf(path) = &model.source else {
            panic!(
                "{}: wears {name:?} and that row is still `Primitive` — the cuboid is hidden \
                 under a model that draws nothing, i.e. an invisible solid wall",
                b.name
            )
        };
        assert!(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join(path).is_file(),
            "{}: wears {name:?} -> {path:?}, and that file is not on disk",
            b.name
        );
        // And it fits the box it is standing in. `render::model::fit_to_class` brings the file
        // to `size_m.y`; both footprint axes then have to land inside the same tolerance a
        // dressed house is held to, because a placed block's collider does NOT give way.
        let (_, _, authored) = defeated_by_titan::world::map::PLACED_DRESSING
            .iter()
            .find(|(n, _, _)| *n == name)
            .copied()
            .unwrap_or_else(|| panic!("{name:?} is dressed but not in PLACED_DRESSING"));
        let scale = b.size_m.y / authored[1];
        for (axis, fit, box_m) in [
            ('x', authored[0] * scale, b.size_m.x),
            ('z', authored[2] * scale, b.size_m.z),
        ] {
            let off = (fit - box_m).abs() / box_m;
            assert!(
                off <= 0.25,
                "{}: {name:?} brought to {:.2} m measures {fit:.2} m on {axis} against a \
                 {box_m:.2} m collider — {:.0} % out, and a placed block's box may not move",
                b.name,
                b.size_m.y,
                off * 100.0
            );
        }
    }
    eprintln!("{} of {} hand-placed blocks are dressed", dressed.len(), placed.len());
}

#[test]
fn f003_the_ground_of_this_district_is_a_3_6_percent_grade_and_that_is_the_whole_relief() {
    // \u26a0\ufe0f **The measurement that stopped a fix from being built on 2026-08-19.** `FIND-132`
    // reported the ground as "one flat sand plane — the 3.00 m of terracing is invisible from
    // the air", and the natural reading is that the terrain is not drawn. It IS drawn:
    // `f003_the_terrain_...` counts 1236 terrace blocks over six levels and 7.50 m of relief.
    //
    // What is true instead is arithmetic, and this test is the arithmetic: **one step of
    // `terrain.step_m` per `terrain.cell_m` of ground.** At 1.50 m per 42 m that is a 3.6 %
    // grade, under houses `scale.ron` allows 11.50 m of — so the largest thing the eye could
    // read is 13 % of one roof, and every one of those steps is covered by a five-tread flight
    // in `stair_color` (there is no bare riser in the district). A district that reads as *a
    // mosaic on a table* is therefore not a rendering bug: it is this number, and it is the
    // user's (`docs/FINDINGS.md` FIND-134, `docs/QUESTIONS.md`).
    //
    // The test exists so the number cannot drift silently: change `step_m` or `cell_m` and the
    // grade this file claims has to be re-measured together with it.
    let d = data();
    let map = d.current_map().expect("current map");
    let t = &map.terrain;
    if t.levels <= 1 || t.step_m <= 0.0 {
        return;
    }
    let grade = t.step_m / t.cell_m;
    let relief_m = t.step_m * (t.levels - 1) as f32;
    eprintln!(
        "terrain: {:.2} m step per {:.1} m cell = {:.1} % grade · {} levels = {relief_m:.2} m \
         of relief · tallest house {:.2} m",
        t.step_m,
        t.cell_m,
        grade * 100.0,
        t.levels,
        d.scale.architecture.heights_m.values().copied().fold(0.0, f32::max)
    );
    assert!(
        grade < 0.10,
        "the terrain now climbs {:.1} % — that is no longer the district FIND-134 measured, \
         and the flight geometry (stair_tread_m against street_m) has to be re-checked with it",
        grade * 100.0
    );
    // And the other half: it really is six levels and not one, so "flat" is a statement about
    // the STEP and never about the field being switched off.
    let (_, ground) = defeated_by_titan::world::map::terrain_of(&d, map);
    assert!(
        ground.field.levels_used().len() >= 4,
        "only {:?} of {} terrain levels are used — the field, not the grade, is the problem \
         then, and FIND-134's conclusion does not apply",
        ground.field.levels_used(),
        t.levels
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
    //
    // ## What changed on 2026-08-12, and why it is not a weakening
    //
    // Read literally, "nothing may stand over a tagged roof centre" forbids **pitched roofs**
    // — and that is why the rebuild deleted them, which the user then judged
    // (*„keine unterschiedliche höhen!"*). But the lie FIND-059 describes is not the cap. It
    // is the **answer**: the player aims at the highest thing he sees, the ray hits whatever
    // is on top, and if *that* is untagged the shot dies as `NoAnchor` for no visible reason.
    // A cap that carries the same anchor bit as its wall answers correctly, so the invariant
    // that actually holds — and the one measured here — is: **whatever caps a tagged surface
    // must itself be tagged.** Stricter in one way, too: it no longer stops at the first
    // capping block but reports every untagged one.
    let d = data();
    let map = d.current_map().expect("current map");
    let plan = plan();

    let mut capped: Vec<String> = Vec::new();
    for a in plan.iter().filter(|k| k.anchorable) {
        let a_top = a.center_m.y + a.size_m.y * 0.5;
        for b in plan.iter().filter(|k| !k.anchorable) {
            if std::ptr::eq(a, b) {
                continue;
            }
            let half = b.size_m * 0.5;
            // Straight over the centre of the roof, and higher than it.
            let over = (b.center_m.x - a.center_m.x).abs() < half.x
                && (b.center_m.z - a.center_m.z).abs() < half.z
                && b.center_m.y + half.y > a_top + 1e-3;
            if over {
                capped.push(format!("{} is capped by the untagged {}", a.name, b.name));
                break;
            }
        }
    }
    assert!(
        capped.is_empty(),
        "{} of {} anchorable blocks answer NoAnchor at the highest point the player sees: \
         {capped:#?}",
        capped.len(),
        plan.iter().filter(|k| k.anchorable).count()
    );

    // And the converse, which is what keeps the sentence above from being a licence: the
    // district really does have roof caps, and every single one of them is tagged like the
    // house it sits on. Without this, deleting the roofs again would make the test greener.
    let roofs: Vec<&BlockPlan> = plan.iter().filter(|k| k.name.starts_with("roof_")).collect();
    if map.layout.perimeter.as_ref().and_then(|p| p.roof.as_ref()).is_some() {
        assert!(roofs.len() > 100, "only {} roof caps in the district", roofs.len());
        for cap in &roofs {
            let want = wall_name_of_cap(cap);
            let wall = plan
                .iter()
                .find(|k| k.name == want)
                .unwrap_or_else(|| panic!("{}: a roof cap with no house under it", cap.name));
            assert_eq!(
                cap.anchorable, wall.anchorable,
                "{} and {} disagree about the anchor bit — that is FIND-059",
                cap.name, wall.name
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// F-019 — the garrison headquarters, and the only kind of door a box world can have
// ---------------------------------------------------------------------------------------

/// The headquarters, as it stands in `assets/data/maps.ron: ashgate`. Spelled out here rather
/// than searched for by name, because a *measurement against the file* is the whole point: if
/// somebody moves the hall, these tests have to go red and be re-derived, not quietly follow.
const HQ_DOOR_X_M: f32 = -15.75; // the middle of the 1.5 m facade
const HQ_DOOR_HALF_Z_M: f32 = 3.0; // the opening is 6 m wide, centred on z = 0
const HQ_FLOOR_TOP_M: f32 = 0.15; // the base slab, and the only 0.15 m floor in the map
const HQ_ROOF_TOP_M: f32 = 11.5; // `scale.ron: architecture.heights_m house_large`

/// Whether a point in metres lies **strictly inside** some cuboid of the plan.
///
/// Strict, for the same reason [`plan_blocks`]'s own overlap test is: standing on a floor means
/// touching it, and a test that counts touching as "blocked" would report every doorway solid.
fn solid_at(plan: &[BlockPlan], p: Vec3) -> Option<&BlockPlan> {
    plan.iter().find(|k| {
        let h = k.size_m * 0.5;
        let d = (p - k.center_m).abs();
        d.x < h.x && d.y < h.y && d.z < h.z
    })
}

#[test]
fn f019_the_headquarters_doorway_is_a_gap_a_player_really_fits_through() {
    // ★ The user, 2026-08-12: „in das gebäude muss man rein laufen können".
    //
    // ⚠️ **This world has no subtraction**, so the door is the space between two wall blocks
    // and there is nothing about it that a reader of `maps.ron` can see. `FIND-056` is what
    // happens when that is forgotten: the wall's plinth was emitted across the full run while
    // the courses above it carried the openings, and the gate measured 2 m of solid stone —
    // green file, green tests, and a gate you walk into.
    //
    // So this measures the opening the way the player meets it: a 1.8 m capsule of 0.35 m
    // radius, swept through the whole thickness of the facade.
    let plan = plan();
    let r = 0.35f32; // game.ron: player.radius_m
    let h = 1.8f32; // game.ron: player.height_m

    // 1. The opening is free, over the full capsule and the full wall thickness.
    let mut blocked: Vec<String> = Vec::new();
    for i in 0..=12 {
        let x = HQ_DOOR_X_M - 0.75 + 1.5 * i as f32 / 12.0;
        for j in 0..=8 {
            let z = -(HQ_DOOR_HALF_Z_M - r) + 2.0 * (HQ_DOOR_HALF_Z_M - r) * j as f32 / 8.0;
            for k in 0..=6 {
                let y = HQ_FLOOR_TOP_M + h * k as f32 / 6.0;
                if let Some(b) = solid_at(&plan, Vec3::new(x, y, z)) {
                    blocked.push(format!("{} at ({x:.2}, {y:.2}, {z:.2})", b.name));
                }
            }
        }
    }
    assert!(
        blocked.is_empty(),
        "{} sample points inside the 6 x 4.5 m gate are solid — that is not a door: {blocked:#?}",
        blocked.len()
    );

    // 2. And the facade beside it is NOT free, or "there is a door" would be "there is no
    //    wall". This is the control the plinth story never had.
    for z in [-8.0f32, 8.0] {
        assert!(
            solid_at(&plan, Vec3::new(HQ_DOOR_X_M, 1.0, z)).is_some(),
            "the facade at z = {z} is open too — the hall has no east wall, only a gap"
        );
    }

    // 3. The floor really is a floor, and it is the one the run brackets against. Nothing
    //    else in this map stands at 0.15 m (ground 0.0, aprons 0.05, quays and bridges 0.4),
    //    which is what makes `assert height > 0.10` in `scripts/f019-hq.txt` a position and
    //    not a coincidence.
    let floor = solid_at(&plan, Vec3::new(-31.0, HQ_FLOOR_TOP_M - 0.05, 0.0))
        .expect("no floor slab under the middle of the hall");
    assert!(
        (floor.center_m.y + floor.size_m.y * 0.5 - HQ_FLOOR_TOP_M).abs() < 1e-4,
        "the hall floor tops out at {} m, not at {HQ_FLOOR_TOP_M}",
        floor.center_m.y + floor.size_m.y * 0.5
    );

    // 4. An interior you can walk around in: the aisle from the gate to the back wall is
    //    clear over its whole 29 m, at head height and at knee height.
    for i in 0..=28 {
        let x = -16.5 - i as f32;
        for y in [HQ_FLOOR_TOP_M + 0.5, HQ_FLOOR_TOP_M + 1.7] {
            assert!(
                solid_at(&plan, Vec3::new(x, y, 0.0)).is_none(),
                "the aisle is blocked at ({x}, {y}, 0) — {:?}",
                solid_at(&plan, Vec3::new(x, y, 0.0)).map(|k| k.name.clone())
            );
        }
    }
}

#[test]
fn f019_the_headquarters_roof_is_the_anchor_and_the_tagged_interior_is_reachable_through_the_door() {
    // Two claims, and the second one changed shape on 2026-08-13 without getting weaker.
    //
    // 1. **The roof is hookable**, so arriving by air is the natural way in: 32 x 26 m of
    //    `sand_brown` at 11.5 m. Unchanged.
    // 2. This half used to be *"nothing inside is tagged"*, and it was there for `FIND-059`:
    //    a tagged surface nothing can hook is a **lie in the map**, and an interior wall is
    //    unreachable by construction. The user then made the whole district anchorable
    //    (`docs/NEXT.md` §1D item 10), so the interior is tagged now — all 31 blocks of it —
    //    and the premise "tagged inside ⇒ unreachable" is what died, not the concern.
    //
    //    **The concern is sharper than before.** While most of the map was untagged, an
    //    unreachable tag was one mistake among many; now that everything is tagged it is the
    //    *only* remaining way for the tag to lie, and this hall is the one place in the
    //    district that has an inside at all. So the honest question is no longer "is anything
    //    tagged in here" — it is **"can a rope get to it"**, and that is measurable: the hall
    //    has a 6 x 4.5 m door (`f019_the_headquarters_doorway_is_a_gap_a_player_really_fits_
    //    through` proves the gap is real), so a shot fired from the street through that door
    //    has to land on a tagged surface inside.
    //
    //    Red for both ways of breaking it: seal the door (the first thing every ray meets is
    //    then the facade at x = -15.75, outside the hall), or un-tag what stands at the end of
    //    the aisle (the ray reaches it and it is not anchorable). Which blocks of the hall may
    //    be untagged at all is `f003_an_unanchorable_block_is_a_listed_exception_...`'s
    //    business; this test is about the ones a rope actually meets.
    let plan = plan();

    let roof = plan
        .iter()
        .find(|k| {
            (k.center_m.y + k.size_m.y * 0.5 - HQ_ROOF_TOP_M).abs() < 1e-4
                && (k.center_m.x + 31.0).abs() < 1e-4
                && k.center_m.z.abs() < 1e-4
        })
        .expect("no roof slab over the headquarters");
    assert!(roof.anchorable, "the headquarters roof is not anchorable — you cannot land on it");
    assert!(roof.solid, "the roof is not solid — you would fall through it");

    // Six shots down the aisle: three lines across the 6 m opening, two heights under the
    // 4.5 m lintel. Marched in 5 cm steps, which is finer than the thinnest thing in the hall
    // (the 0.3 m skid), so nothing is stepped over.
    let mut landed: Vec<&str> = Vec::new();
    for z in [-2.5f32, 0.0, 2.5] {
        for y in [1.0f32, 3.5] {
            let mut hit: Option<(f32, &BlockPlan)> = None;
            let mut x = -5.0f32; // on the pavement outside, east of the facade
            while x > -60.0 {
                if let Some(b) = solid_at(&plan, Vec3::new(x, y, z)) {
                    hit = Some((x, b));
                    break;
                }
                x -= 0.05;
            }
            let (at_x, block) = hit.unwrap_or_else(|| {
                panic!(
                    "a shot at (y {y}, z {z}) crosses the whole hall and meets nothing — the \
                     headquarters has no back wall, and a rope fired down the aisle flies out \
                     of the district"
                )
            });
            assert!(
                at_x < -16.5,
                "the shot at (y {y}, z {z}) stops on {} at x = {at_x:.2} — that is the facade, \
                 not the interior. The door is shut, and everything tagged behind it is exactly \
                 the lie FIND-059 is about",
                block.name
            );
            assert!(
                block.anchorable,
                "the shot at (y {y}, z {z}) reaches {} at x = {at_x:.2}, inside the hall, and it \
                 is not anchorable — the interior of the one building in this district you can \
                 walk into refuses the rope",
                block.name
            );
            if !landed.contains(&block.name.as_str()) {
                landed.push(block.name.as_str());
            }
        }
    }

    // And the six shots do not all end on the same slab, or "the interior is reachable" would
    // be one wall's story.
    assert!(
        landed.len() >= 2,
        "all six shots land on {landed:?} — one surface is not an interior"
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

    // The number comes out of the plan and is written nowhere here, because a map change must
    // not turn into a test change.
    let expected_anchorable = plan.iter().filter(|p| p.anchorable).count();
    assert_eq!(anchorable, expected_anchorable, "anchorable bodies in the index vs. in the file");
    assert!(anchorable > 0, "not a single anchorable body in the index");

    // The hull the index carries is the hull of the body, and it comes from the file's half
    // edge. A factor of 2 here is a hook that catches in mid-air.
    for (name, _, id) in &all {
        let entry = index.body(id.expect("id")).expect("in the index");
        let want = by_name[name.as_str()];
        assert_eq!(entry.half_size_m, want.size_m * 0.5, "{name}: half size in the index");
        assert_eq!(entry.center_m, want.center_m, "{name}: center in the index");
    }

    // ⚠️ **The negative case, and it has to be produced now instead of found.**
    //
    // This test used to end on a census — "and not all of them are anchorable, or the mask
    // decides nothing". That was load bearing: every assertion above compares the index against
    // the file, and on a map where the file says `true` everywhere, an index that simply
    // hardcoded `mask_from(true, true)` would satisfy all of them. The census was what made the
    // *derivation* falsifiable, and the map flip of 2026-08-13 took it away — the district is
    // anchorable throughout, on purpose.
    //
    // So the missing half of the map is spawned instead: one body carrying the mask a
    // `anchorable: false` block would get. If the bit is carried through it comes back out of
    // the index unset; if it is invented anywhere between `Body` and `SpatialIndex`, this is
    // the only thing in the file that notices.
    let control = app
        .world_mut()
        .spawn((
            Name::new("t036a_unanchorable_control"),
            Body { half_size_m: Vec3::splat(2.0), mask: mask_from(true, false) },
            Transform::from_translation(Vec3::new(-150.0, 40.0, -150.0)),
        ))
        .id();
    app.update();
    let control_id = *app.world().get::<BodyId>(control).expect("the control body gets an id");
    let entry = app
        .world()
        .resource::<SpatialIndex>()
        .body(control_id)
        .expect("the control body is in the index");
    assert!(
        !entry.mask.contains(BodyMask::ANCHORABLE),
        "a body built from `anchorable: false` comes out of the index ANCHORABLE — the bit is \
         not carried, it is invented, and on a map that is tagged throughout nothing else in \
         this file would ever see it"
    );
    assert!(
        entry.mask.contains(BodyMask::SOLID),
        "and it lost the solid bit on the way, so the index is not carrying the mask at all"
    );
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


// ---------------------------------------------------------------------------------------
// F-019 — A LAMP THAT STANDS OUTDOORS IS A BRIGHTER WORLD, AND NOBODY WOULD SEE IT COMING
// ---------------------------------------------------------------------------------------

#[test]
fn f019_every_interior_lamp_stands_in_a_room_with_a_roof_over_it_and_a_floor_under_it() {
    // `maps.ron: lights` exists to lift ONE room without touching a single exterior pixel
    // (`docs/FINDINGS.md` FIND-078). The containment argument is Lambert — the outer face of
    // the wall a lamp stands behind points away from it — and that argument holds only while
    // the lamp is actually **behind** a wall. A lamp dropped a few metres off, out over the
    // street, breaks nothing that compiles, breaks no other test, and lights the district from
    // a place no measurement in this repository looks at.
    //
    // So: over every lamp there has to be a solid block whose footprint contains it, and under
    // every lamp another one. That is what "indoors" means, and it is checkable from the file.
    let data = data();
    for (id, map) in &data.maps.maps {
        for (i, lamp) in map.lights.iter().enumerate() {
            let (lx, ly, lz) = lamp.center_m;
            let covers = |b: &defeated_by_titan::data::MapBlock| {
                lx >= b.center_m.0 - b.size_m.0 / 2.0
                    && lx <= b.center_m.0 + b.size_m.0 / 2.0
                    && lz >= b.center_m.2 - b.size_m.2 / 2.0
                    && lz <= b.center_m.2 + b.size_m.2 / 2.0
            };
            let roof = map
                .blocks
                .iter()
                .any(|b| b.solid && covers(b) && b.center_m.1 - b.size_m.1 / 2.0 >= ly);
            let floor = map
                .blocks
                .iter()
                .any(|b| b.solid && covers(b) && b.center_m.1 + b.size_m.1 / 2.0 <= ly);
            assert!(
                roof,
                "{id}: lamp {i} at ({lx}, {ly}, {lz}) has no solid block above it — it hangs \
                 in the open air and lights the whole district, and the ONLY way anybody \
                 finds that out is by looking at a screenshot"
            );
            assert!(
                floor,
                "{id}: lamp {i} at ({lx}, {ly}, {lz}) has no solid block under it — a lamp \
                 over a hole lights whatever is at the bottom of it"
            );
        }
    }
}

// =====================================================================================
// The dressing — a generated house that wears a model out of the pack instead of being a
// cuboid (2026-08-18).
//
// Three things are guarded here and they are three different failures:
//   1. the catalogue in `world::map` still describes the files it claims to describe;
//   2. `art.ron` — and nothing else — is the switch that turns dressing on;
//   3. a dressed house is EXACTLY its model, and it never leaves the slot it was drawn in.
// =====================================================================================

/// The full extent of a `.glb`'s own `hit.min`/`hit.max` pair, in metres.
///
/// A hand-rolled read of the glTF JSON chunk: `magic|version|length`, then chunks of
/// `length|type|data`, and the first chunk of type `JSON` is the document. No serde_json in
/// this tree, and pulling one in for eleven numbers would be a dependency for a test.
///
/// ⚠️ `hit.max.z < hit.min.z` on all 278 files of the drop — the two empties are a **corner
/// pair**, not a min/max on every axis — so every extent is taken as an absolute difference.
fn glb_hit_corners_m(file: &str) -> (Vec3, Vec3) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/3d/glb").join(file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(&bytes[0..4], b"glTF", "{file} is not a .glb");
    let mut at = 12usize;
    let mut json = None;
    while at + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap());
        if kind == 0x4E4F_534A {
            json = Some(String::from_utf8_lossy(&bytes[at + 8..at + 8 + len]).into_owned());
            break;
        }
        at += 8 + len;
    }
    let json = json.unwrap_or_else(|| panic!("{file} has no JSON chunk"));

    // One node per element of `"nodes":[ {..}, {..} ]`. Split on the object boundary rather
    // than parsing: a node is flat, and `"name"` and `"translation"` are both on it.
    let nodes = json
        .split_once("\"nodes\":[")
        .unwrap_or_else(|| panic!("{file}: no nodes array"))
        .1;
    let read = |want: &str| -> Vec3 {
        for node in nodes.split("},") {
            if !node.contains(&format!("\"name\":\"{want}\"")) {
                continue;
            }
            let Some(t) = node.split_once("\"translation\":[") else { continue };
            let nums: Vec<f32> = t
                .1
                .split(']')
                .next()
                .unwrap_or("")
                .split(',')
                .filter_map(|k| k.trim().parse::<f32>().ok())
                .collect();
            assert_eq!(nums.len(), 3, "{file}: {want} has {:?}", nums);
            return Vec3::new(nums[0], nums[1], nums[2]);
        }
        panic!("{file}: no node named {want:?} — the pack lost its hit empties");
    };
    (read("hit.min"), read("hit.max"))
}

/// The full extent of that corner pair, in metres — `abs`, because it is a **corner** pair.
fn glb_extent_m(file: &str) -> Vec3 {
    let (lo, hi) = glb_hit_corners_m(file);
    (hi - lo).abs()
}

/// Every class the generator can dress a block with, and the file behind it: the three houses
/// of [`DRESSING`], the eight remnants of [`RUIN_KIT`], the six mounds of [`RUBBLE_KIT`].
///
/// One list, so that a seventeenth class cannot be added without landing in the floor test
/// below — `f003_every_dressed_class_stands_on_the_floor_of_its_own_block` asserts the length
/// against the three tables.
fn every_dressed_file() -> Vec<(&'static str, &'static str)> {
    vec![
        ("house_small", "a-083-fachwerkhaus-klein.glb"),
        ("house_town", "a-083-fachwerkhaus-stadthaus.glb"),
        ("house_large", "a-083-fachwerkhaus-gross.glb"),
        ("ruin_roof_collapsed", "a-089-ruine-dach-eingestuerzt.glb"),
        ("ruin_roof_half", "a-089-ruine-dach-haelfte.glb"),
        ("ruin_gable", "a-089-ruine-giebel.glb"),
        ("ruin_heap", "a-089-ruine-haufen.glb"),
        ("ruin_upper_floor", "a-089-ruine-obergeschoss.glb"),
        ("ruin_pillar", "a-089-ruine-pfeiler.glb"),
        ("ruin_wall_corner", "a-089-ruine-wand-ecke.glb"),
        ("ruin_wall_high", "a-089-ruine-wand-hoch.glb"),
        ("rubble_beams", "a-090-schutt-balken.glb"),
        ("rubble_cover", "a-090-schutt-deckung.glb"),
        ("rubble_flat", "a-090-schutt-flach.glb"),
        ("rubble_heap_large", "a-090-schutt-haufen-gross.glb"),
        ("rubble_high", "a-090-schutt-hoch.glb"),
        ("rubble_wall_piece", "a-090-schutt-wandstueck.glb"),
    ]
}

#[test]
fn f003_every_dressed_class_stands_on_the_floor_of_its_own_block() {
    // ★ **„zudem sind die gebäude nicht auf dem boden sondern in der luft!"** (user,
    // 2026-08-19). A block is positioned by its **centre** — that is the frame the collider,
    // `Body::half_size_m` and the spatial index are written in, and moving it would move the
    // world. Every model in the pack is authored on its **feet**. So the model, hung on that
    // entity, started at the box's middle and the building floated by half its own height.
    //
    // The fix moves the drawing and the model's anchors by one offset
    // (`shared::ModelName::feet_y_m` -> `render::model::feet_offset_m`) and leaves the
    // collider alone. This is the arithmetic of it, for **all seventeen** classes the
    // generator can put down, out of the seventeen files themselves — not out of the
    // catalogue, so that a re-export that lifts a model off its own origin lands here.
    //
    // ⚠️ The scale matters: a remnant is planned at the size its file has, but the fit is a
    // ratio and an offset computed before it would be right at 1.0 and wrong everywhere else
    // (`feet_offset_m`'s own header). The classes below fit at 1.0 today — the second half of
    // the test therefore checks a deliberately *rescaled* block, where an unscaled offset is
    // off by metres.
    let files = every_dressed_file();
    assert_eq!(
        files.len(),
        DRESSING.len() + RUIN_KIT.len() + RUBBLE_KIT.len(),
        "a dressable class was added to a catalogue without a file beside it here — and would \
         then never be checked for standing on its own floor"
    );

    for (name, file) in files {
        let (lo, hi) = glb_hit_corners_m(file);
        let floor_m = lo.y.min(hi.y);
        assert!(
            floor_m.abs() < 0.001,
            "{name}: {file} is authored with its floor at y = {floor_m:.4} m instead of 0. \
             `world::map` tells the renderer the model's floor is `-size.y / 2` below the \
             block's centre, and that is only the box's own floor while the file stands on \
             its origin"
        );

        let mut anchors: BTreeMap<String, Vec3> = BTreeMap::new();
        anchors.insert("hit.min".to_string(), lo);
        anchors.insert("hit.max".to_string(), hi);
        let authored_m = (hi.y - lo.y).abs();

        // A block of exactly this class's size, and one of double it — the second is what
        // separates a scaled offset from an unscaled one.
        for size_y_m in [authored_m, authored_m * 2.0] {
            let scale = fit_to_class(&anchors, Some(size_y_m), None);
            let offset = feet_offset_m(&anchors, scale, Some(-size_y_m * 0.5));
            // Where the mesh's lowest point lands in the block's own frame.
            let drawn_floor_m = offset.y + floor_m * scale;
            assert!(
                (drawn_floor_m + size_y_m * 0.5).abs() < 0.01,
                "{name} in a {size_y_m:.2} m block: the mesh's floor is drawn at \
                 {drawn_floor_m:.3} m in the block's frame and the block's floor is at \
                 {:.3} m — {:.3} m of air under a building",
                -size_y_m * 0.5,
                drawn_floor_m + size_y_m * 0.5
            );
            let drawn_ridge_m = drawn_floor_m + authored_m * scale;
            assert!(
                (drawn_ridge_m - size_y_m * 0.5).abs() < 0.01,
                "{name} in a {size_y_m:.2} m block: the mesh reaches {drawn_ridge_m:.3} m and \
                 the box's lid is at {:.3} m. The collider and the picture are one box \
                 (`BlockPlan::model`), so a roof through the ceiling is the same defect \
                 upside down",
                size_y_m * 0.5
            );
        }
    }
}

#[test]
fn f003_every_dressed_block_in_the_real_map_names_its_own_floor() {
    // The other end of the same wire: the arithmetic above is worth nothing if `BlockPlan`
    // does not actually tell the renderer where the floor is. This is the shipped map, built
    // by the real app, walked entity by entity.
    let mut app = built_world();
    let mut dressed = 0usize;
    let world = app.world_mut();
    let mut q = world.query::<(&Block, &ModelName)>();
    for (block, model) in q.iter(&world) {
        dressed += 1;
        assert_eq!(
            model.feet_y_m,
            Some(-block.size.y * 0.5),
            "the dressed block wearing {:?} is {:.2} m tall and tells the renderer its floor \
             is at {:?}. A block sits at its CENTRE, so the only floor it has is half its \
             height below that — anything else is the building in the air again",
            model.name,
            block.size.y,
            model.feet_y_m
        );
    }
    assert!(
        dressed > 0,
        "not one block on the shipped map wears a model, so this test measured nothing. \
         Either the district lost its dressing or `art.ron` put every row back to `Primitive`"
    );
}


#[test]
fn f003_the_dressing_catalogue_is_what_the_glb_files_really_measure() {
    // ★ `world::map::DRESSING` is eleven numbers copied out of eleven files, and a copied
    // number rots silently: a re-export that makes the town house 20 cm wider leaves the
    // generator dressing every house at the old width, and the mesh then stands a hand's
    // breadth off the collider it is supposed to BE. Nothing about that has a picture.
    //
    // The `y` column is checked twice over: against the file, and against
    // `scale.ron: architecture.heights_m` under the same key. `art.ron` claims in prose that
    // the pack was authored to our own height bands ("matching scale.ron heights_m house_small
    // 4.5 / house_town 8.0 / house_large 11.5 exactly") — this is where the claim is measured.
    let files = [
        ("house_small", "a-083-fachwerkhaus-klein.glb"),
        ("house_town", "a-083-fachwerkhaus-stadthaus.glb"),
        ("house_large", "a-083-fachwerkhaus-gross.glb"),
    ];
    let heights = &data().scale.architecture.heights_m;
    assert_eq!(DRESSING.len(), files.len(), "a class was added without a file beside it");

    for (i, (name, authored_m)) in DRESSING.iter().enumerate() {
        let (want_name, file) = files[i];
        assert_eq!(*name, want_name, "DRESSING row {i} is {name:?}, the file list says {want_name:?}");
        let measured = glb_extent_m(file);
        let claimed = Vec3::new(authored_m[0], authored_m[1], authored_m[2]);
        let off = (measured - claimed).abs();
        assert!(
            off.max_element() < 0.011,
            "{name}: {file} measures {measured:?} m, world::map::DRESSING says {claimed:?} — \
             a dressed house would be built to a size the model does not have"
        );
        let class_m = heights
            .get(*name)
            .unwrap_or_else(|| panic!("scale.ron: architecture.heights_m has no {name:?}"));
        assert!(
            (class_m - authored_m[1]).abs() < 0.011,
            "{name}: scale.ron says the class is {class_m} m tall, {file} is authored at {} m \
             — one of the two is wrong and the district would inherit it",
            authored_m[1]
        );
    }
}

/// `assets/data` with the three house rows switched from `Primitive` to the files behind them
/// — the one line of RON that turns the district into half-timbered houses.
///
/// ⚠️ **`art.ron` has only one of the three rows today** (`house_small`; `house_town` and
/// `house_large` are named in its prose and are not in its map). Those two are added here so
/// the mechanism can be measured at all, and the missing rows are printed rather than
/// swallowed — the day they land in the file this function stops inventing anything and
/// nothing else about the test changes.
fn data_with_house_models() -> GameData {
    let mut d = data();
    let mut invented: Vec<&str> = Vec::new();
    for (name, file) in [
        ("house_small", "3d/glb/a-083-fachwerkhaus-klein.glb"),
        ("house_town", "3d/glb/a-083-fachwerkhaus-stadthaus.glb"),
        ("house_large", "3d/glb/a-083-fachwerkhaus-gross.glb"),
    ] {
        let on_disk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join(file);
        assert!(
            on_disk.is_file(),
            "{}: art.ron would name a file the repository has not got",
            on_disk.display()
        );
        match d.art.models.get_mut(name) {
            Some(row) => row.source = ModelSource::Gltf(file.to_string()),
            None => {
                invented.push(name);
                let template = d
                    .art
                    .models
                    .get("house_small")
                    .expect("art.ron has lost `house_small` as well — the registry is gone")
                    .clone();
                d.art.models.insert(
                    name.to_string(),
                    Model { source: ModelSource::Gltf(file.to_string()), ..template },
                );
            }
        }
    }
    if !invented.is_empty() {
        eprintln!("art.ron has no row for {invented:?} — added for this test only");
    }
    d
}

/// `assets/data` with the three house rows put **back** on `Primitive` — the other direction
/// of the same one-line switch, and the half that says it is a switch and not a wire.
fn data_without_house_models() -> GameData {
    let mut d = data();
    for (name, _) in DRESSING {
        match d.art.models.get_mut(name) {
            Some(row) => row.source = ModelSource::Primitive,
            None => panic!("art.ron has lost its {name:?} row — the house registry is gone"),
        }
    }
    d
}

#[test]
fn f003_art_ron_is_the_only_switch_that_dresses_the_district() {
    // ★ The whole feature has to be one line of RON, and since 2026-08-19 it is **on**.
    //
    // On, because the user asked for it: *„zudem fehlen noch die häuser"* (2026-08-18). The
    // three house rows in `art.ron` name their files and the generator dresses itself with no
    // code touched — that is what `art.ron`'s own header promises ("ONE line to swap it") and
    // this is the only place it is measured on something that is not a titan.
    //
    // ⚠️ Until this day the assertion was the **opposite** one (`dressed == 0`), because the
    // rows were `Primitive` on purpose: `BlockPlan::spawn` did not insert the `ModelName`, so
    // flipping them would have cost every house its cuboid roof in exchange for nothing on
    // screen. That blocker is gone (`src/shared/anchors.rs`), so the guard turns around with
    // it — and the **off** direction is measured below, on data with the rows put back.
    let shipped = plan();
    let dressed_today = shipped.iter().filter(|k| k.model.is_some()).count();
    assert!(
        dressed_today > 300,
        "only {dressed_today} block(s) of the shipped district wear a model — `art.ron` is \
         the switch and it is supposed to be on. „zudem fehlen noch die häuser\""
    );

    // And the converse, which is what keeps it a *switch*: with the three rows back on
    // `Primitive` not one house is dressed, and every one of them has its stepped gable
    // again. Without this half, "one line of RON" is a claim in a comment.
    let off = data_without_house_models();
    let off_map = off.current_map().expect("current map");
    let off_plan = plan_blocks(&off, off_map);
    let still_dressed: Vec<&str> = off_plan
        .iter()
        .filter(|k| k.model.is_some_and(|m| DRESSING.iter().any(|(n, _)| *n == m)))
        .map(|k| k.name.as_str())
        .collect();
    assert!(
        still_dressed.is_empty(),
        "{} house(s) wear a model although all three rows are `Primitive` again: {:?} — then \
         `art.ron` is not the switch and a name with no file behind it takes a cuboid roof \
         away for nothing",
        still_dressed.len(),
        &still_dressed[..still_dressed.len().min(5)]
    );

    let d = data_with_house_models();
    let map = d.current_map().expect("current map");
    let now = plan_blocks(&d, map);

    let houses = walls(&now);
    let dressed: Vec<&BlockPlan> = houses.iter().filter(|k| k.model.is_some()).copied().collect();
    eprintln!(
        "{} of {} houses dressed · {} blocks against {} undressed",
        dressed.len(),
        houses.len(),
        now.len(),
        shipped.len()
    );
    // Measured 2026-08-18: 766 of 926 at `DRESS_TOLERANCE` 0.25. A band and not a floor —
    // half the district is a failure, and all of it means the tolerance stopped being a
    // tolerance and every house is now whatever the model says.
    assert!(
        dressed.len() * 2 > houses.len(),
        "only {} of {} houses could wear a model — the generator's lots and the pack's \
         footprints have drifted apart (world::map::DRESS_TOLERANCE)",
        dressed.len(),
        houses.len()
    );
    for k in &dressed {
        let name = k.model.expect("dressed");
        assert!(
            DRESSING.iter().any(|(n, _)| *n == name),
            "{}: wears {name:?}, which is not a class in world::map::DRESSING",
            k.name
        );
        assert!(
            d.model(name).is_some(),
            "{}: wears {name:?}, which art.ron does not list at all",
            k.name
        );
        // ⚠️ **A dressed house grows no cuboid roof.** The model brings `dach_l`, `dach_r`,
        // `dach_first`, a chimney and a gable of its own; a stack of stone lids through them
        // is FIND-059 in its loudest form.
        let caps = caps_of(&now, k);
        assert!(
            caps.is_empty(),
            "{} wears {name:?} and still carries {} cuboid cap(s) — two roofs on one house",
            k.name,
            caps.len()
        );
    }
    // And the converse: what is NOT dressed still has its stepped gable, or this test would
    // pass just as well on a district that stopped building roofs altogether.
    let bare_with_caps = houses
        .iter()
        .filter(|k| k.model.is_none() && !caps_of(&now, k).is_empty())
        .count();
    assert!(
        bare_with_caps > 20,
        "only {bare_with_caps} undressed house(s) still carry a cap — the roofscape was \
         thrown away rather than replaced"
    );
}

#[test]
fn f003_a_dressed_house_is_exactly_its_model_and_never_leaves_its_slot() {
    // ★ The three ways dressing can go wrong, and none of them has a picture until somebody
    // flies into it:
    //
    //   1. the box is not the model — then the collider, the anchor surface and the mesh are
    //      three different boxes, and the hook catches a metre off the wall;
    //   2. the box grew into its neighbour — a model is scaled uniformly and the box gives
    //      way, so a house CAN come out wider than it was drawn;
    //   3. the facade moved — every proportion in this file is measured facade to facade, and
    //      a district that quietly opens its streets by a metre is FIND-058 coming back.
    let d = data_with_house_models();
    let map = d.current_map().expect("current map");
    let now = plan_blocks(&d, map);
    let bare = plan();

    let by_name = |p: &[BlockPlan], n: &str| p.iter().find(|k| k.name == n).cloned();
    let houses = walls(&now);
    let dressed: Vec<&BlockPlan> = houses.iter().filter(|k| k.model.is_some()).copied().collect();
    assert!(dressed.len() > 100, "only {} dressed houses to measure", dressed.len());

    let mut moved: Vec<f32> = Vec::new();
    let mut newcomers = 0usize;
    for k in &dressed {
        let name = k.model.expect("dressed");
        let (_, authored_m) = DRESSING
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("{}: unknown class {name}", k.name));
        // 1. The box IS the model, uniformly scaled to the ridge the height band drew.
        let ridge = ridge_m(&now, k);
        let scale = ridge / authored_m[1];
        // The model's own frontage is its x, its depth its z (the front is the ±z face).
        // Which world axis each of them lands on is the wing's business — north and south
        // face along z, west and east along x — so both assignments are legal and exactly
        // one of them has to hold to the millimetre.
        let long = authored_m[0] * scale;
        let deep = authored_m[2] * scale;
        let depth_along_x =
            (k.size_m.x - deep).abs() < 1e-3 && (k.size_m.z - long).abs() < 1e-3;
        let depth_along_z =
            (k.size_m.z - deep).abs() < 1e-3 && (k.size_m.x - long).abs() < 1e-3;
        assert!(
            depth_along_x || depth_along_z,
            "{}: wears {name:?} at ridge {ridge:.2} m, so the model is {long:.2} x {deep:.2} m \
             either way round — the box is {:.2} x {:.2}",
            k.name,
            k.size_m.x,
            k.size_m.z
        );

        // 3. The facade stayed where it was drawn — and on the **depth** axis, which is the
        //    one a street is measured across. A house may only give way backwards, into its
        //    own courtyard.
        // A house that is not in the undressed plan is a house the apron veto used to delete
        // and no longer does: the dressed box is smaller and really does clear the paving it
        // used to overlap. Legal, counted, and bounded below — a flood of them would mean the
        // veto had stopped working rather than the houses having got smaller.
        let Some(before) = by_name(&bare, &k.name) else {
            newcomers += 1;
            continue;
        };
        let faces = |b: &BlockPlan| {
            [
                b.center_m.x - b.size_m.x * 0.5,
                b.center_m.x + b.size_m.x * 0.5,
                b.center_m.z - b.size_m.z * 0.5,
                b.center_m.z + b.size_m.z * 0.5,
            ]
        };
        let (a, b) = (faces(&before), faces(k));
        let same = |i: usize| (a[i] - b[i]).abs() < 1e-3;
        let kept_facade = if depth_along_x {
            same(0) || same(1)
        } else {
            same(2) || same(3)
        };
        assert!(
            kept_facade,
            "{}: {a:?} became {b:?} and no face on the depth axis survived — the facade line \
             moved instead of the courtyard side",
            k.name
        );
        moved.push((before.size_m.x - k.size_m.x).abs().max((before.size_m.z - k.size_m.z).abs()));
    }
    moved.sort_by(f32::total_cmp);
    eprintln!(
        "{} dressed houses · footprint moved by a median {:.2} m, at most {:.2} m · \
         {newcomers} house(s) the apron veto no longer deletes",
        dressed.len(),
        moved[moved.len() / 2],
        moved[moved.len() - 1]
    );
    assert!(
        newcomers * 20 < dressed.len(),
        "{newcomers} of {} dressed houses did not exist before — a smaller box may clear an \
         apron it used to overlap, but not that many of them",
        dressed.len()
    );

    // ⚠️ And the one thing the aprons exist for: a house under a gallery. The apron is
    // 0.30 m of paving that deletes whatever would stand beneath the 14 m and 60 m
    // structures, so a house that just squeezed past one has to be checked directly —
    // `f003_no_anchorable_block_has_another_block_sitting_on_its_roof_centre` measures the
    // SHIPPED plan and would never see it.
    let placed_n = map.blocks.len();
    for k in &dressed {
        let top = k.center_m.y + k.size_m.y * 0.5;
        for g in now.iter().take(placed_n) {
            let over = g.center_m.y - g.size_m.y * 0.5 >= top - 1e-3;
            let covers = (g.center_m.x - k.center_m.x).abs() < g.size_m.x * 0.5
                && (g.center_m.z - k.center_m.z).abs() < g.size_m.z * 0.5;
            assert!(
                !(over && covers),
                "{} wears {:?} and now stands under {} — the dressing let a house back in \
                 beneath a gallery the aprons exist to keep clear",
                k.name,
                k.model,
                g.name
            );
        }
    }

    // 2. Nothing overlaps: not two houses of the same ring, and not a hand-placed block.
    let hits = |a: &BlockPlan, b: &BlockPlan| {
        let (ac, ah) = (a.center_m, a.size_m * 0.5);
        let (bc, bh) = (b.center_m, b.size_m * 0.5);
        (ac.x - bc.x).abs() < ah.x + bh.x - 1e-3
            && (ac.y - bc.y).abs() < ah.y + bh.y - 1e-3
            && (ac.z - bc.z).abs() < ah.z + bh.z - 1e-3
    };
    let mut clashes = Vec::new();
    for (i, a) in houses.iter().enumerate() {
        for b in houses.iter().skip(i + 1) {
            if lot_of(a) != lot_of(b) {
                continue;
            }
            if hits(a, b) {
                clashes.push(format!("{} x {}", a.name, b.name));
            }
        }
    }
    assert!(
        clashes.is_empty(),
        "{} pair(s) of houses in one ring stand inside each other after dressing — the \
         tolerance ate the alley between them: {:?}",
        clashes.len(),
        &clashes[..clashes.len().min(6)]
    );
    for a in &dressed {
        for g in now.iter().take(placed_n) {
            assert!(
                !hits(a, g),
                "{} wears {:?} and grew into the hand-placed {} — the veto was taken before \
                 the dressing instead of after it",
                a.name,
                a.model,
                g.name
            );
        }
    }
}

#[test]
fn f003_the_districts_ground_comes_from_the_map_and_barely_from_the_seed() {
    // ★ Written 2026-08-18, after `shared::terrain`'s own `assert_ne!` ("two seeds give two
    // grounds") turned out to be false and to have never been run — its five unit tests live
    // inside `src/shared/terrain.rs` and only `--lib` executes those.
    //
    // The rule behind it is exact (`docs/FINDINGS.md` FIND-101): the relaxation's fixed point
    // is the L1 distance transform from every pinned cell and from the outside of the grid,
    // capped by the cell's own draw `levels - 1 - notch`. So the seed can move a cell **only**
    // where that cell is `levels - 1` or more away from every pin and from the rim. This test
    // measures how much of the shipped district that is, because the answer is what the number
    // "6 levels" is worth: on 2026-08-18 it was **one cell out of 256**, and that one cell is
    // the district's only level-5 cell.
    //
    // Two things are asserted, and the second one is the interesting half:
    //   * the shipped seed reproduces its ground exactly — the desync guard, at map level and
    //     through the whole pin pipeline rather than on a 12 x 12 fixture;
    //   * the seed stays a footnote. If a later `cell_m`, `levels` or a thinned-out set of
    //     hand-placed blocks lets the draw take over the district, this goes red — and it
    //     should, because from that point on the ground is noise and not the town.
    let d = data();
    let map = d.current_map().expect("current map");
    let (_, base) = defeated_by_titan::world::map::terrain_of(&d, map);
    let (nx, nz) = (base.field.nx() as i32, base.field.nz() as i32);
    let cells = (nx * nz) as f32;

    let (_, again) = defeated_by_titan::world::map::terrain_of(&d, map);
    assert_eq!(base.field, again.field, "the same map planned twice gave two grounds");

    let mut worst = 0;
    for seed in [1u64, 2, 7, 12_345, 0xDEAD_BEEF, 999_999_999] {
        let mut m = map.clone();
        m.seed = seed;
        let (_, other) = defeated_by_titan::world::map::terrain_of(&d, &m);
        let moved = (0..nz)
            .flat_map(|iz| (0..nx).map(move |ix| (ix, iz)))
            .filter(|(ix, iz)| base.field.level_at(*ix, *iz) != other.field.level_at(*ix, *iz))
            .count();
        eprintln!(
            "seed {seed}: {moved} of {} cells move, levels used {:?}",
            nx * nz,
            other.field.levels_used()
        );
        worst = worst.max(moved);
    }
    eprintln!(
        "shipped ground: {}x{nz} cells, levels used {:?}, at most {worst} cells ({:.1} %) \
         depend on the seed",
        nx,
        base.field.levels_used(),
        100.0 * worst as f32 / cells
    );
    assert!(
        worst as f32 <= 0.05 * cells,
        "{worst} of {} terrain cells ({:.1} %) change when only the seed changes — the ground \
         of this district is supposed to be the shape of its hand-placed geometry \
         (FIND-090, FIND-101), not a draw. Either the pins thinned out or `levels` grew past \
         the distance transform",
        nx * nz,
        100.0 * worst as f32 / cells
    );
}

// ===========================================================================================
// §1F — Ashgate has fallen. `docs/gameplay/world.md`: *"the war is already lost … Ashgate has
// long since fallen; the Vanguard runs salvage missions into its own ruins"*.
//
// The user, 2026-08-18: *„das ist nicht die echte map!"* — a setting complaint, not a look
// complaint. What was built until this round is an intact, inhabited, tidy market town, and
// `grep -ci 'ruin|rubble|collapse' assets/data/maps.ron` answered **0**.
// ===========================================================================================

/// Every standing remnant of a fallen house — `ruin_<lot>_<i>`.
fn ruins(plan: &[BlockPlan]) -> Vec<&BlockPlan> {
    plan.iter().filter(|k| k.name.starts_with("ruin_")).collect()
}

/// Every collapsed house — `rubble_<lot>_<i>`, the mound that is left where the walls went.
fn rubble(plan: &[BlockPlan]) -> Vec<&BlockPlan> {
    plan.iter().filter(|k| k.name.starts_with("rubble_")).collect()
}

/// What a **street** is measured between: a house that still stands, and a ruin that still
/// stands on the same frontage line.
///
/// ⚠️ Ruins belong in this set and rubble does not, and both halves are load bearing. A
/// half-standing gable keeps its street-facing face exactly where the intact house's was
/// (`world::map`), so a gap measured to it is a real street width; a rubble mound is
/// deliberately pushed **past** that line into the road (`maps.ron: layout.damage.spill_m`),
/// so counting it would report a street that is narrower than the one you can fly down.
fn facades(plan: &[BlockPlan]) -> Vec<&BlockPlan> {
    plan.iter()
        .filter(|k| k.name.starts_with("house_") || k.name.starts_with("ruin_"))
        .collect()
}

#[test]
fn f003_ashgate_has_fallen_and_it_is_not_a_tidy_market_town() {
    // ★ The setting, as a measurement. Red on the district as it shipped until 2026-08-19:
    // 926 intact houses, 0 ruins, 0 rubble — an inhabited walled town in a world whose own
    // design says this ring fell a century ago and is now walked for salvage.
    //
    // Deliberately **not** "some percentage is damaged". Three separate things have to be
    // true, and each of them is a different way of getting a ruin wrong:
    //
    // 1. Ruins and rubble exist at all.
    // 2. Intact stretches survive — a uniformly flattened district has nothing to salvage,
    //    no canyon left to fly down, and reads as a quarry rather than as a town that fell.
    // 3. It is **designed and not sprinkled**: damage comes in stretches. A district where
    //    every third house at random is a stump is noise, and noise reads as a texture, not
    //    as a history.
    let plan = plan();
    let (standing, broken, gone) = (walls(&plan).len(), ruins(&plan).len(), rubble(&plan).len());
    let built = standing + broken + gone;
    assert!(built > 500, "only {built} generated buildings — is this the district?");

    eprintln!(
        "ashgate: {standing} standing, {broken} ruined, {gone} collapsed of {built} \
         ({:.0} % damaged)",
        100.0 * (broken + gone) as f32 / built as f32
    );
    assert!(
        broken > 0 && gone > 0,
        "{broken} ruins and {gone} rubble mounds in the whole district — `docs/gameplay/\
         world.md`: „Ashgate has long since fallen; the Vanguard runs salvage missions into \
         its own ruins\". What stands here is an intact market town, and the user said so: \
         „das ist nicht die echte map!\""
    );
    let damaged = (broken + gone) as f32 / built as f32;
    assert!(
        (0.20..=0.75).contains(&damaged),
        "{:.0} % of the district is damaged. Below 20 % it is a town with a few bad houses; \
         above 75 % there is nothing left to salvage and no frontage left to fly between",
        100.0 * damaged
    );

    // 3. Stretches, not pepper. Every lot is one closed block of row houses, so „a stretch"
    // is measurable per lot: how many lots are wholly intact, and how many are wholly gone.
    // Uniform independent draws at this rate would leave almost none of either.
    let mut per_lot: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for b in walls(&plan).iter().chain(ruins(&plan).iter()).chain(rubble(&plan).iter()) {
        let e = per_lot.entry(lot_of(b).to_string()).or_insert((0, 0));
        e.0 += 1;
        if !b.name.starts_with("house_") {
            e.1 += 1;
        }
    }
    let whole = per_lot.values().filter(|(n, d)| *n >= 4 && *d == 0).count();
    let razed = per_lot.values().filter(|(n, d)| *n >= 4 && d * 2 > *n).count();
    eprintln!(
        "{} lots of {} are wholly intact, {razed} are more than half gone",
        whole,
        per_lot.len()
    );
    assert!(
        whole >= 8 && razed >= 8,
        "{whole} wholly intact blocks and {razed} mostly razed ones out of {} — damage that \
         is neither of those is sprinkled, and sprinkled damage reads as a texture over a \
         market town instead of as a district that fell. It wants intact stretches worth \
         salvaging and collapsed ones that block a route",
        per_lot.len()
    );
}

#[test]
fn f003_the_damage_is_a_gradient_and_the_core_is_what_is_left() {
    // ★ The other half of „design the damage, do not sprinkle it": a fallen ring is not
    // uniformly destroyed. The gradient is the story — the Vanguard held the middle longest
    // and the outer edge is where the wall was breached — and without it the two thresholds
    // above can be met by a coin flip per building.
    //
    // Red when `layout.damage.core_severity` and `edge_severity` are set to the same figure:
    // then the district is damaged everywhere at one rate, and this ratio goes to 1.0.
    let plan = plan();
    let d = data();
    let map = d.current_map().expect("current map");
    let half = map.size_m.0.max(map.size_m.1) * 0.5;

    let (mut near, mut near_bad, mut far, mut far_bad) = (0usize, 0usize, 0usize, 0usize);
    for b in walls(&plan).iter().chain(ruins(&plan).iter()).chain(rubble(&plan).iter()) {
        let r = b.center_m.xz().length() / half;
        let bad = !b.name.starts_with("house_");
        if r < 0.35 {
            near += 1;
            near_bad += bad as usize;
        } else if r > 0.75 {
            far += 1;
            far_bad += bad as usize;
        }
    }
    assert!(near > 40 && far > 40, "{near} buildings near the core, {far} out at the edge");
    let (a, b) = (near_bad as f32 / near as f32, far_bad as f32 / far as f32);
    eprintln!("damaged: {:.0} % near the core, {:.0} % out at the edge", 100.0 * a, 100.0 * b);
    assert!(
        b > a + 0.15,
        "{:.0} % of the buildings near the core are damaged and {:.0} % out at the edge — \
         that is one flat rate over the whole district, not a ring that was breached from \
         outside. The gradient is what makes the damage a history instead of a texture",
        100.0 * a,
        100.0 * b
    );
}

#[test]
fn f003_a_fallen_facade_still_holds_a_rope() {
    // ★ The constraint the ruin round could most easily have broken, and it is the user's
    // own: *„es ist extrem wichtig dass man wirklich überall sein seil festmachen kann. also
    // überall! ohne ausnahmen!"* (2026-08-13). A collapsed wall is still a wall.
    //
    // `f003_an_unanchorable_block_is_a_listed_exception_and_the_fixture_keeps_both_kinds`
    // already forbids an *unlisted* untagged block anywhere on the shipped map. This one says
    // the same thing about the ruins in particular and it says it by name, so that the day
    // somebody adds `anchorable: false` to the damage table with a plausible reason
    // („rubble is loose"), the failure names the rule it broke.
    let plan = plan();
    let broken: Vec<&str> = ruins(&plan)
        .iter()
        .chain(rubble(&plan).iter())
        .filter(|b| !b.anchorable)
        .map(|b| b.name.as_str())
        .collect();
    assert!(!ruins(&plan).is_empty(), "no ruins in the district at all");
    assert!(
        broken.is_empty(),
        "{} of the district's ruins hold no rope: {:?}. „überall! ohne ausnahmen!\" — a \
         collapsed facade is the most interesting thing in a salvage district to hang from",
        broken.len(),
        &broken[..broken.len().min(6)]
    );
}

#[test]
fn f003_the_houses_that_are_left_wear_a_model() {
    // ★ *„zudem fehlen noch die häuser"* (the user, 2026-08-18). The generator has planned a
    // model name per house since that day and **not one entity ever carried it**: `ModelName`
    // lived in `src/render/model.rs`, `world` has no allow-list edge to `render`, and
    // `BlockPlan::spawn` therefore dropped the one field that turns a grey box into a
    // half-timbered house.
    //
    // Red twice over before 2026-08-19: `art.ron` had all three house rows on `Primitive`
    // (so nothing was even planned), and nothing inserted the component (so flipping them
    // would have changed the footprints and shown nothing).
    let plan = plan();
    let dressed = plan.iter().filter(|b| b.model.is_some()).count();
    let houses = walls(&plan).len();
    eprintln!("{dressed} of {} planned blocks wear a model ({houses} houses stand)", plan.len());
    assert!(
        dressed * 4 > houses,
        "only {dressed} of {houses} standing houses are dressed — `art.ron` is the switch \
         (`world::map::dress_for` refuses a row that is not `Gltf(...)`), and a district of \
         grey boxes is what „zudem fehlen noch die häuser\" was about"
    );
}

#[test]
fn f003_the_ruin_catalogue_is_what_the_glb_files_really_measure() {
    // ★ Fourteen files, forty-two numbers copied out of them, and a copied number rots
    // silently: a re-export that makes the gable 20 cm wider leaves the generator building
    // every ruin at the old width, and the mesh then stands a hand's breadth off the collider
    // it is supposed to BE. Nothing about that has a picture — it is the same argument as
    // `f003_the_dressing_catalogue_is_what_the_glb_files_really_measure`, one kit further on.
    let files = [
        ("ruin_roof_collapsed", "a-089-ruine-dach-eingestuerzt.glb"),
        ("ruin_roof_half", "a-089-ruine-dach-haelfte.glb"),
        ("ruin_gable", "a-089-ruine-giebel.glb"),
        ("ruin_heap", "a-089-ruine-haufen.glb"),
        ("ruin_upper_floor", "a-089-ruine-obergeschoss.glb"),
        ("ruin_pillar", "a-089-ruine-pfeiler.glb"),
        ("ruin_wall_corner", "a-089-ruine-wand-ecke.glb"),
        ("ruin_wall_high", "a-089-ruine-wand-hoch.glb"),
        ("rubble_beams", "a-090-schutt-balken.glb"),
        ("rubble_cover", "a-090-schutt-deckung.glb"),
        ("rubble_flat", "a-090-schutt-flach.glb"),
        ("rubble_heap_large", "a-090-schutt-haufen-gross.glb"),
        ("rubble_high", "a-090-schutt-hoch.glb"),
        ("rubble_wall_piece", "a-090-schutt-wandstueck.glb"),
    ];
    let d = data();
    let kit: Vec<(&str, [f32; 3])> =
        RUIN_KIT.iter().chain(RUBBLE_KIT.iter()).map(|(n, e)| (*n, *e)).collect();
    assert_eq!(kit.len(), files.len(), "a remnant was added without a file beside it");

    for (i, (name, authored_m)) in kit.iter().enumerate() {
        let (want_name, file) = files[i];
        assert_eq!(*name, want_name, "kit row {i} is {name:?}, the file list says {want_name:?}");
        let measured = glb_extent_m(file);
        let claimed = Vec3::new(authored_m[0], authored_m[1], authored_m[2]);
        assert!(
            (measured - claimed).abs().max_element() < 0.011,
            "{name}: {file} measures {measured:?} m, world::map says {claimed:?} — a remnant \
             would be built to a size the model does not have"
        );
        // And `art.ron` really binds this name to this file. A kit row whose registry row
        // points somewhere else is a ruin wearing another ruin.
        match d.model(name).map(|m| &m.source) {
            Some(ModelSource::Gltf(path)) => assert!(
                path.ends_with(file),
                "art.ron binds {name:?} to {path:?} and world::map measured {file}"
            ),
            other => panic!("art.ron: {name:?} is {other:?} — the ruin kit is switched off"),
        }
    }

    // ⚠️ The mounds are the half of the kit that a traversal decision hangs on: „rubble takes
    // the ground and leaves the air alone" is only true while nothing in this group is a
    // building. Red the day somebody adds a 9 m ruin to the rubble list.
    for (name, authored_m) in RUBBLE_KIT {
        assert!(
            authored_m[1] <= 3.0,
            "{name} is authored {} m tall — a mound over 3 m in a 6 m street stops being \
             something you can still fly over (maps.ron: layout.damage)",
            authored_m[1]
        );
    }
}

#[test]
fn f003_a_planned_model_reaches_the_entity_that_was_planned_for_it() {
    // ★ *„zudem fehlen noch die häuser"*, on the spawning side. The generator has planned a
    // model per house since 2026-08-18 and `BlockPlan::spawn` dropped it on the floor:
    // `ModelName` lived in `render` and `world` may not reach into `render`. The type moved to
    // `shared/` on 2026-08-19 and this is the test that says the name arrives — without it the
    // plan can be perfect and the district still be grey boxes, which is exactly the state
    // that shipped for a day.
    //
    // It goes through the **real** app, not through `plan_blocks`: what is measured here is
    // the hop from plan to entity and nothing else.
    let mut app = built_world();
    let mut q = app.world_mut().query::<(&Name, &ModelName, &Block)>();
    let worn: Vec<(String, String, f32)> = q
        .iter(app.world())
        .map(|(n, m, b)| (n.to_string(), m.name.clone(), b.size.y))
        .collect();
    let planned = plan().iter().filter(|k| k.model.is_some()).count();
    eprintln!("{} entities carry a model name, {planned} were planned", worn.len());
    assert_eq!(
        worn.len(),
        planned,
        "{} of the {planned} planned models reached an entity — a name that stays in the plan \
         renders nothing at all",
        worn.len()
    );
    assert!(planned > 300, "only {planned} blocks were planned with a model");

    // And it is the **box's own** height that is handed over, not a class figure: that is what
    // `render::model::fit_to_class` scales the file by, so a house whose collider is 9.4 m
    // tall has to ask for 9.4 m or the mesh and the collider are two different buildings.
    let mut q = app.world_mut().query::<(&Name, &ModelName, &Block)>();
    let off: Vec<String> = q
        .iter(app.world())
        .filter(|(_, m, b)| m.height_m.is_none_or(|h| (h - b.size.y).abs() > 1e-4))
        .map(|(n, m, b)| format!("{n}: asks for {:?}, its box is {}", m.height_m, b.size.y))
        .collect();
    assert!(
        off.is_empty(),
        "{} block(s) ask the renderer for a height their collider has not got: {:?}",
        off.len(),
        &off[..off.len().min(4)]
    );
}

#[test]
fn f003_the_rubble_takes_the_ground_and_the_ruin_takes_the_lane() {
    // ★ „Say what your rubble does to a swing lane" — as two measurements instead of a
    // sentence, because this is the half of the fall of Ashgate that is not decoration.
    //
    // 1. **A mound really lies in the road.** It is pushed past its own frontage line
    //    (`maps.ron: layout.damage.spill_m`), so a lane that used to be clear now has
    //    something in it that you have to go over. Red the moment `spill_m` stops being
    //    applied — and a mound that stays politely inside its lot is a decoration.
    // 2. **And it leaves the air alone.** Nothing in the rubble kit reaches the height a rope
    //    swings at; what changes the swing lane is the ruin beside it, which is a stump where
    //    a wall used to hold the rope high.
    let plan = plan();
    let d = data();
    let map = d.current_map().expect("current map");
    let k = map
        .layout
        .damage
        .as_ref()
        .expect("the shipped district is the fallen one — maps.ron: layout.damage");

    let mounds = rubble(&plan);
    let stumps = ruins(&plan);
    assert!(mounds.len() > 20 && stumps.len() > 100, "{} mounds, {} ruins", mounds.len(), stumps.len());

    // 1. Height first, because it is the cheap half.
    let tallest = mounds.iter().map(|b| b.size_m.y).fold(0.0_f32, f32::max);
    assert!(
        tallest <= k.rubble_height_m.1 + 1e-3,
        "the tallest mound is {tallest:.2} m and maps.ron draws them out of {:?} — rubble \
         that reaches the swing lane punishes the one verb this game has",
        k.rubble_height_m
    );

    // 2. The mounds stand lower than the ruins, which stand lower than the houses. That
    // ordering is the whole damage model in one line, and it is what a rope feels.
    let median = |mut v: Vec<f32>| {
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    };
    let top = |b: &BlockPlan| b.size_m.y;
    let (h, r, m) = (
        median(walls(&plan).iter().map(|b| top(b)).collect()),
        median(stumps.iter().map(|b| top(b)).collect()),
        median(mounds.iter().map(|b| top(b)).collect()),
    );
    eprintln!("median height: {h:.2} m standing · {r:.2} m ruined · {m:.2} m collapsed");
    assert!(
        h > r + 1.5 && r > m + 0.5,
        "standing {h:.2} m, ruined {r:.2} m, collapsed {m:.2} m — a ruin that is as tall as \
         the house it was is a re-skin, and the lane over a fallen row is supposed to DIP"
    );

    // 3. And the spill, measured against the same district with the spill turned off —
    // which is the only way to ask "did it move INTO the road" without re-deriving the
    // frontage line the generator drew. Every mound has to have moved by exactly `spill_m`,
    // and it has to have moved **outward**: away from the middle of its own block, which is
    // where the courtyard is and the street is not.
    let mut without = d.clone();
    without
        .maps
        .maps
        .get_mut(&d.maps.current)
        .expect("the shipped map")
        .layout
        .damage
        .as_mut()
        .expect("the shipped district is the fallen one")
        .spill_m = 0.0;
    let unspilled = plan_blocks(&without, without.current_map().expect("current map"));
    let at: std::collections::BTreeMap<&str, Vec3> =
        unspilled.iter().map(|b| (b.name.as_str(), b.center_m)).collect();

    // The middle of a block, out of every building still standing on it.
    let mut sum: std::collections::BTreeMap<&str, (Vec3, f32)> = std::collections::BTreeMap::new();
    for b in &plan {
        if b.name.starts_with("house_") || b.name.starts_with("ruin_") || b.name.starts_with("rubble_")
        {
            let e = sum.entry(lot_of(b)).or_insert((Vec3::ZERO, 0.0));
            e.0 += b.center_m;
            e.1 += 1.0;
        }
    }

    let (mut moved, mut outward) = (0usize, 0usize);
    for b in &mounds {
        let Some(before) = at.get(b.name.as_str()) else {
            panic!("{} is not in the unspilled plan — the two are supposed to be one seed", b.name)
        };
        let step = b.center_m - *before;
        if (step.length() - k.spill_m).abs() < 1e-3 {
            moved += 1;
        }
        let (middle, n) = sum[lot_of(b)];
        if step.xz().dot((b.center_m - middle / n).xz()) > 0.0 {
            outward += 1;
        }
    }
    eprintln!(
        "{moved} of {} mounds moved {} m when the spill was switched on, {outward} of them \
         away from their courtyard",
        mounds.len(),
        k.spill_m
    );
    assert!(
        moved == mounds.len() && outward * 10 > mounds.len() * 9,
        "{moved} of {} mounds moved by `maps.ron: layout.damage.spill_m` = {} m and {outward} \
         of them moved outward — a mound that stays inside its own lot changes nothing about \
         how the district is crossed, and one that falls into the courtyard blocks nothing",
        mounds.len(),
        k.spill_m
    );
}

// ---------------------------------------------------------------------------------------
// `F-019` Nachschub-Stationen — „Statische Punkte auf der Map fuellen Gas und Klingen.
// Begrenzte Nachladungen pro Mission." Acceptance: „Nachladen dauert 1,5 s; Zaehler sichtbar;
// leere Station wird visuell markiert."
// ---------------------------------------------------------------------------------------
//
// The hole these close is measured, not felt (`docs/QUESTIONS.md` Q-044): a tank buys ~16.7 s
// of held boost at the honest `gas_tank: 300` against a **330 s sortie**, and until 2026-08-24
// there was **no refuel anywhere outside the headquarters**. That is why `gas_tank` had to go
// 50x to be testable at all — the tank was papering over a missing world feature.

fn stations(app: &mut App) -> Vec<(Entity, Vec3, defeated_by_titan::shared::SupplyStation)> {
    let mut q = app
        .world_mut()
        .query::<(Entity, &Transform, &defeated_by_titan::shared::SupplyStation)>();
    let mut all: Vec<_> =
        q.iter(app.world()).map(|(e, t, s)| (e, t.translation, *s)).collect();
    all.sort_by(|a, b| a.1.z.total_cmp(&b.1.z));
    all
}

#[test]
fn f019_the_map_that_is_flown_has_supply_stations_standing_in_it() {
    let mut app = built_world();
    let d = data();
    let map = d.current_map().expect("maps.ron: current names a map");
    let found = stations(&mut app);

    assert_eq!(
        found.len(),
        map.supply_stations.len(),
        "maps.ron lists {} stations for {:?} and the world built {}",
        map.supply_stations.len(),
        d.maps.current,
        found.len()
    );
    assert!(
        !found.is_empty(),
        "the map that is actually flown ({:?}) has NO refuel station in it, which is the whole \
         of Q-044 — a 330 s sortie against a tank worth 16.7 s of boost",
        d.maps.current
    );
    for (_, at, s) in &found {
        assert_eq!(s.radius_m, d.gear.resupply.station_radius_m, "at {at:?}");
        assert_eq!(s.refill_s, d.gear.resupply.station_refill_s, "at {at:?}");
        assert!(s.uses_left > 0, "a station that ships empty at {at:?}");
        assert!(!s.running(), "a station idles until somebody stands in it, at {at:?}");
        // One reload is a whole tank over `station_refill_s` — the rate falls out of the two
        // numbers instead of being a third one that can disagree with them.
        let want = d.game.vector.gas_tank / d.gear.resupply.station_refill_s;
        assert!((s.gas_per_s - want).abs() < 1e-3, "{} instead of {want} at {at:?}", s.gas_per_s);
    }
}

#[test]
fn f019_a_station_is_not_a_wall_and_not_an_anchor() {
    // The one property that makes adding four of them to `ashgate` unable to move a single
    // number in any test that already stands: a station carries **no `Collider` and no `Body`**,
    // so it is neither in avian's world nor in the spatial index. It is a place and a marker.
    let mut app = stepped_world();
    let found = stations(&mut app);
    assert!(!found.is_empty());
    for (e, at, _) in found {
        assert!(
            app.world().get::<Collider>(e).is_none(),
            "the station at {at:?} has a collider — it is now something a player bounces off"
        );
        assert!(
            app.world().get::<Body>(e).is_none(),
            "the station at {at:?} is in the spatial index — every hook raycast can now find it"
        );
        assert!(
            app.world().get::<Block>(e).is_none(),
            "the station at {at:?} carries a `Block` — `Block` means A CUBOID OF THE CITY, it \
             is what `f003_the_city_comes_from_the_file_and_not_twice` counts against \
             `plan_blocks`, and four stations wearing one made that count 2875 against 2871. \
             The station is drawn by `render::build_station_meshes` off its own component."
        );
    }
}

#[test]
fn f019_one_reload_fills_a_tank_in_the_time_the_file_says_and_costs_exactly_one_use() {
    use defeated_by_titan::player::spawn_player;
    use defeated_by_titan::shared::{Gas, IdCounter};

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app.update();

    let d = data();
    let (station, at, before) = stations(&mut app).into_iter().next().expect("a station");

    // A second player, standing exactly on it. Not a `Transform` write on the local player: a
    // raw position is synced back off the body, and `spawn_player` is the door every body in
    // this game comes through anyway.
    let e = {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        let e = spawn_player(&mut commands, &mut ids, &data, at, false);
        *world.resource_mut::<IdCounter>() = ids;
        e
    };
    app.update();
    // Empty the tank. `Gas` has one writer in the simulation (`vector::gas`), and this is a
    // test setting a starting condition, not a second authority.
    app.world_mut().get_mut::<Gas>(e).expect("a player has a tank").current = 0.0;

    let refill_ticks = (d.gear.resupply.station_refill_s * d.game.simulation_hz as f32).round() as u64;
    // `+ 3`: the request is written in `PostStep` and applied in the NEXT tick's `Intent`
    // (`vector::gas::apply_refuel_requests`), so the last drop arrives one tick after the pump
    // stops. Two more for the spawn tick and the arithmetic.
    for _ in 0..refill_ticks + 3 {
        app.update();
    }

    let tank = app.world().get::<Gas>(e).expect("a tank").current;
    assert!(
        (tank - d.game.vector.gas_tank).abs() < d.game.vector.gas_tank * 0.02,
        "{:.1} s at a station gave {tank:.0} of {} gas back — one reload is a WHOLE tank",
        d.gear.resupply.station_refill_s,
        d.game.vector.gas_tank
    );

    let after = *app
        .world()
        .get::<defeated_by_titan::shared::SupplyStation>(station)
        .expect("the station is still there");
    assert_eq!(
        after.uses_left,
        before.uses_left - 1,
        "one reload has to cost exactly one use ({} -> {})",
        before.uses_left,
        after.uses_left
    );
    assert!(!after.running(), "and the pump has to have stopped by now");

    // …and the station is still in the world when it is spent. It goes dark, it does not
    // vanish — a station that disappears teaches the player that he misremembered the map.
    assert!(app.world().get_entity(station).is_ok());
}

#[test]
fn f019_a_spent_station_gives_nothing_and_stays_where_it_is() {
    use defeated_by_titan::player::spawn_player;
    use defeated_by_titan::shared::{Gas, IdCounter, SupplyStation};

    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app.update();

    let (station, at, _) = stations(&mut app).into_iter().next().expect("a station");
    // Drain it by hand — 3 x 90 ticks of standing is a slower way of writing the same line.
    app.world_mut().get_mut::<SupplyStation>(station).unwrap().uses_left = 0;

    let e = {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        let e = spawn_player(&mut commands, &mut ids, &data, at, false);
        *world.resource_mut::<IdCounter>() = ids;
        e
    };
    app.update();
    app.world_mut().get_mut::<Gas>(e).unwrap().current = 0.0;
    for _ in 0..200 {
        app.update();
    }

    assert_eq!(
        app.world().get::<Gas>(e).unwrap().current,
        0.0,
        "an empty station still refuelled — `uses_left` is then a number nobody reads"
    );
    assert!(
        app.world().get::<SupplyStation>(station).is_some_and(|s| s.empty()),
        "and it has to report itself empty, because that is what `render::supply` paints"
    );
}

// =====================================================================================
// F-156 — THE HUB IS A PLACE, AND EVERY DOOR IN IT CAN BE WALKED TO
// =====================================================================================
//
// > *„zudem gibt es auch noch keine lobby. mit lobby mein ich auch rumlaufen. also eher eine
// > art hub."* — the user, 2026-08-26, the **second** time he has said the lobby is missing.
//
// It exists. `missions.ron: hub` puts a spawn point, six deployment circles and two supply
// stations on the map and `scripts/f070-hub.txt` walks all of it. What it did not have is
// anything to walk **to**: 2871 blocks in the district and not one of them inside the yard the
// player stands in. The four tests below are the two halves of the answer —
// **there is something there** and **it does not stand in the way**.

/// Half the width of the lane an approach needs, in metres, measured to a block's **edge**.
///
/// ⚠️ **Calibrated against what the map already had, not chosen.** The tightest existing pair
/// is the veteran and elite approach against the two gantry columns at `(±14, 17.5)`: the
/// straight line from the hub spawn point to `(∓9, 16)` passes a column footprint at exactly
/// **1.00 m**, and those columns are the swing spine — they are not moving for a pad. So the
/// floor is set below that and above nothing: `0.90` fails only for geometry that is genuinely
/// *in the way*, and `FIND-162`'s two pads behind the garrison hall's east wall measure **0.00**
/// on this scale. Everything this session placed keeps **2.0 m** by construction; the assert is
/// the guard, not the design rule.
const APPROACH_CLEARANCE_M: f32 = 0.90;

/// A block is **in the way** only if it is tall enough to walk into and low enough to be under
/// your head: top above [`STEP_OVER_M`], bottom below [`HEAD_ROOM_M`].
///
/// Without both halves this test measures the floor. The hall's own depot slab lies across the
/// supply walk (top 0.15 m) and its door lintel hangs over it (bottom 4.5 m) — one is stepped
/// on and the other is walked under, and neither has ever stopped anybody.
const STEP_OVER_M: f32 = 0.50;
const HEAD_ROOM_M: f32 = 1.80;

/// The 2D distance from a segment to an axis-aligned box footprint, in metres. `0.0` means the
/// segment crosses the box — which is what a wall in front of a door measures.
fn segment_to_footprint_m(a: Vec2, b: Vec2, center: Vec2, half: Vec2) -> f32 {
    // 64 samples over a segment that is never longer than the hub is wide (≈20 m) is a
    // resolution of 0.3 m, and the quantity being measured is a metre-scale clearance. An
    // exact segment/AABB routine would be the right thing in the game; in a test that runs
    // eight segments against 215 boxes it would be code nobody checks.
    let mut best = f32::MAX;
    for i in 0..=64 {
        let p = a.lerp(b, i as f32 / 64.0);
        let d = ((p - center).abs() - half).max(Vec2::ZERO);
        best = best.min(d.length());
    }
    best
}

/// The hand-placed blocks that a walking player can collide with, as (name, centre, half) in 2D.
fn walkable_obstacles(plan: &[BlockPlan]) -> Vec<(String, Vec2, Vec2)> {
    plan.iter()
        .filter(|b| b.name.starts_with("block_"))
        .filter(|b| {
            let bottom = b.center_m.y - b.size_m.y * 0.5;
            let top = b.center_m.y + b.size_m.y * 0.5;
            top > STEP_OVER_M && bottom < HEAD_ROOM_M
        })
        .map(|b| {
            (
                b.name.clone(),
                Vec2::new(b.center_m.x, b.center_m.z),
                Vec2::new(b.size_m.x * 0.5, b.size_m.z * 0.5),
            )
        })
        .collect()
}

#[test]
fn f156_every_deployment_pad_can_be_walked_to_from_the_hub_spawn_in_a_straight_line() {
    // 🔴 **`FIND-162`, and it is the reason this test exists at all.** Two of the six pads were
    // placed at `(-18, 0, ±8)` — **behind the garrison hall's east wall**, which `maps.ron`
    // places by hand at `x = -47..-15, |z| <= 13`. A player could not walk to two of the six
    // doors of his own hub, and **nothing said so**: the pads were fine, the mission keys were
    // fine, the `clear_radius_m` check was fine (and irrelevant — a clear radius answers the
    // GENERATOR, never the level designer). The failure was silent until somebody flew there.
    //
    // This is that check, and it is written against the **files** and not against a list of
    // coordinates: every pad in `missions.ron: hub.deployments`, from the spawn point in
    // `missions.ron: hub.spawn_m`, against every hand-placed block in `maps.ron`. A pad added
    // tomorrow is checked the day it is added, and so is a crate put down in front of an old one.
    let d = data();
    let plan = plan();
    let hub = &d.missions.hub;
    let spawn = Vec2::new(hub.spawn_m.0, hub.spawn_m.2);
    let obstacles = walkable_obstacles(&plan);
    assert!(
        obstacles.len() > 50,
        "only {} walkable placed blocks — is this ashgate, and did the filter eat the map?",
        obstacles.len()
    );
    assert!(!hub.deployments.is_empty(), "missions.ron: hub has no deployment pads to walk to");

    for pad in &hub.deployments {
        let target = Vec2::new(pad.center_m.0, pad.center_m.2);
        let (mut worst, mut culprit) = (f32::MAX, String::new());
        for (name, center, half) in &obstacles {
            let d = segment_to_footprint_m(spawn, target, *center, *half);
            if d < worst {
                worst = d;
                culprit = name.clone();
            }
        }
        eprintln!(
            "{}/{} at {:?}: {:.2} m of daylight, tightest against {culprit}",
            pad.mission, pad.difficulty, pad.center_m, worst
        );
        assert!(
            worst >= APPROACH_CLEARANCE_M,
            "the walk from the hub spawn to the {}/{} pad at {:?} passes {culprit} with \
             {worst:.2} m of daylight — below {APPROACH_CLEARANCE_M} m that is a door the \
             player cannot reach on foot, and nothing in the game says so (FIND-162)",
            pad.mission,
            pad.difficulty,
            pad.center_m
        );
    }
}

#[test]
fn f156_the_hub_yard_is_furnished_and_not_an_empty_apron() {
    // 🔴 **The user has said the lobby is missing twice**, and the hub demonstrably exists both
    // times. This test is what the sentence means when it is turned into a number: **how many
    // things are there in the yard he stands in.**
    //
    // Before 2026-08-26 the answer was **zero** — the whole district is 2871 blocks and not one
    // of them was inside 30 m of the hub spawn point except the garrison hall, the two gantry
    // frames and the street under his feet. A spawn point, six circles and two racks is a
    // *configuration*, not a place.
    //
    // ⚠️ It counts **dressed** blocks, not blocks. A cuboid is not furniture: the thing that
    // makes a lantern a lantern is that `world::map::placed_dress_for` found it a file in the
    // drop, and a row that quietly falls back to `Primitive` in `art.ron` puts the count
    // straight back down.
    let plan = plan();
    let d = data();
    let hub = &d.missions.hub;
    let spawn = Vec3::new(hub.spawn_m.0, 0.0, hub.spawn_m.2);
    let yard: Vec<&BlockPlan> = plan
        .iter()
        .filter(|b| b.name.starts_with("block_"))
        .filter(|b| {
            let p = Vec3::new(b.center_m.x, 0.0, b.center_m.z);
            p.distance(spawn) <= 30.0
        })
        .collect();
    let dressed: Vec<&&BlockPlan> = yard.iter().filter(|b| b.model.is_some()).collect();

    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for b in &dressed {
        *kinds.entry(b.model.expect("filtered")).or_default() += 1;
    }
    eprintln!("{} placed blocks within 30 m of the hub spawn, {} of them dressed: {kinds:?}",
        yard.len(), dressed.len());

    assert!(
        dressed.len() >= 20,
        "only {} of the {} hand-placed blocks in the hub yard wear a model — the hub is a spawn \
         point, six circles and two racks again, which is exactly what the user calls a missing \
         lobby (`user-messages.md`, 2026-08-26)",
        dressed.len(),
        yard.len()
    );
    assert!(
        kinds.len() >= 5,
        "the yard is furnished out of only {} different models ({kinds:?}) — a place needs a \
         vocabulary, not one prop repeated",
        kinds.len()
    );
}

#[test]
fn f156_the_placed_dressing_catalogue_is_what_the_glb_files_really_measure() {
    // The same guard `f003_the_dressing_catalogue_...` puts on the three house rows, for the
    // table that dresses **hand-placed** blocks. Every number in `world::map::PLACED_DRESSING`
    // is copied out of a `.glb`'s own `hit.min`/`hit.max` pair, and a copied number rots
    // silently: a re-export that makes the signpost 20 cm wider leaves the map placing a box
    // the model does not fill, and nothing about that has a picture.
    let files = [
        ("market_stall", "a-087-marktstand-zeltdach.glb"),
        ("gas_drum", "a-132-fass-stehend.glb"),
        ("lamp_post", "a-088-laterne-strasse.glb"),
        ("signpost", "a-088-wegweiser.glb"),
        ("banner_long", "a-133-banner-lang.glb"),
        ("hand_cart", "a-131-karren-intakt.glb"),
        ("crate_small", "a-132-kiste-klein.glb"),
        ("sentry", "a-136-npc-vanguard.glb"),
    ];
    let table = defeated_by_titan::world::map::PLACED_DRESSING;
    assert_eq!(table.len(), files.len(), "a PLACED_DRESSING row was added without a file beside it");
    for (i, (name, _color, authored_m)) in table.iter().enumerate() {
        let (want, file) = files[i];
        assert_eq!(*name, want, "PLACED_DRESSING row {i} is {name:?}, the file list says {want:?}");
        let measured = glb_extent_m(file);
        let claimed = Vec3::new(authored_m[0], authored_m[1], authored_m[2]);
        assert!(
            (measured - claimed).abs().max_element() < 0.011,
            "{name}: {file} measures {measured:?} m, PLACED_DRESSING says {claimed:?} — the map \
             would place a box the model does not fill"
        );
    }
}

#[test]
fn f156_no_dressing_row_can_be_mistaken_for_another_one() {
    // `world::map::placed_dress_for` returns the **first** row that matches, so two rows whose
    // (colour, proportion) windows overlap make the table order-dependent — and the box that
    // was drawn for a lantern silently grows a market awning. With two rows that was obvious by
    // inspection; with nine it is not, and this is the assertion that keeps it out.
    //
    // It also fixes what "the box for this row" means: the model brought to its authored height
    // and **rounded to the centimetre**, which is exactly how `maps.ron` writes a size.
    let d = data();
    for (name, color, authored_m) in defeated_by_titan::world::map::PLACED_DRESSING {
        let nominal = Vec3::new(
            (authored_m[0] * 100.0).round() / 100.0,
            (authored_m[1] * 100.0).round() / 100.0,
            (authored_m[2] * 100.0).round() / 100.0,
        );
        let got = defeated_by_titan::world::map::placed_dress_for(&d, nominal, color);
        assert_eq!(
            got,
            Some(name),
            "a {color} box of {nominal:?} m is the {name:?} model's own silhouette and the \
             catalogue answers {got:?} — the rows are not telling each other apart"
        );
    }
}
