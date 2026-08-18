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
    house.name.trim_start_matches("house_").split('_').next().unwrap_or("")
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
    let plan = plan();
    let houses = walls(&plan);
    assert!(houses.len() > 100, "only {} generated houses — is this the district?", houses.len());

    let span = |c: f32, s: f32| (c - s * 0.5, c + s * 0.5);
    let mut gaps: Vec<f32> = Vec::new();
    let mut ratios: Vec<f32> = Vec::new();
    let mut alleys: Vec<f32> = Vec::new();
    let mut broken = 0usize;
    for a in &houses {
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
            for b in &houses {
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
            // bit as its house, so `d < H` (FIND-041) hangs from the cap.
            ratios.push(gap / ridge_m(&plan, a).min(ridge_m(&plan, b)));
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
