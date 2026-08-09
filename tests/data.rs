//! The guard over the RON files.
//!
//! **Numbers belong in RON, not in Rust** (`prompts/init.md` §4) — and that is exactly why
//! the RON needs a test. A number in code is caught by the compiler; a number in a file is
//! caught by nobody, except here.
//!
//! What is checked is not "is it pretty", but **what the bible lays down as binding** and
//! **what has to be consistent with itself** (every reference points at something that
//! exists).

use defeated_by_titan::data::{GameData, GasConsumer};
use std::path::PathBuf;

fn data() -> GameData {
    GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

#[test]
fn t005_all_ron_files_load() {
    // No `serde(default)` for game values: a missing value is meant to crash. This test is
    // the place where that crash is not an aborted game but a red test.
    let d = data();
    assert!(!d.titans.kinds.is_empty(), "titan.ron without a single kind");
    assert!(!d.art.models.is_empty(), "art.ron without a single model");
    assert!(!d.maps.maps.is_empty(), "maps.ron without a single map");
    assert!(!d.maps.palette.is_empty(), "maps.ron without a single color");
}

#[test]
fn t005_every_titan_winds_up_at_least_four_tenths_of_a_second() {
    // Bible, pillar P4 (readability before realism): "every titan attack has a windup of at
    // least 0.4 seconds". That is not a recommendation — the player must never have to ask
    // why he died.
    for (name, kind) in &data().titans.kinds {
        assert!(
            kind.windup_s >= 0.4,
            "{name}: windup_s = {} — the bible demands at least 0.4 s",
            kind.windup_s
        );
    }
}

#[test]
fn t005_every_titan_points_at_a_model_that_exists() {
    // A reference into nothing is a bug of the same class as a dead link in the docs (§10).
    // Otherwise it only surfaces when exactly this titan gets spawned.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        assert!(
            d.art.models.contains_key(&kind.model),
            "{name} points at model {:?}, which is not in art.ron",
            kind.model
        );
    }
}

#[test]
fn t005_every_wave_names_a_titan_kind_that_exists() {
    let d = data();
    for (mission, template) in &d.missions.templates {
        for wave in &template.waves {
            assert!(
                d.titans.kinds.contains_key(&wave.kind),
                "mission {mission:?}: wave at {}s wants {:?}, which is not in titan.ron",
                wave.at_s,
                wave.kind
            );
            assert!(wave.count > 0, "mission {mission:?}: a wave with zero titans");
        }
    }
}

#[test]
fn t005_a_mission_arc_lasts_five_to_seven_minutes() {
    // Bible 5, change 10: average mission length 5–7 min. Every mission has to be a complete
    // arc with progress you can feel.
    for (name, v) in &data().missions.templates {
        assert!(
            (300.0..=420.0).contains(&v.target_duration_s),
            "{name}: target_duration_s = {} — the bible wants 5–7 min (300–420 s)",
            v.target_duration_s
        );
        assert!(
            v.waves.iter().all(|w| w.at_s <= v.target_duration_s),
            "{name}: a wave spawns after the end of the mission"
        );
    }
}

#[test]
fn t005_the_simulation_runs_at_sixty_hertz() {
    // §6 rule 4: a fixed simulation step. Over the network a frame-dependent result is not a
    // comfort problem but a desync — and by then it is too late to change it.
    let hz = data().game.simulation_hz;
    assert!((hz - 60.0).abs() < 1e-9, "simulation_hz = {hz}, expected 60");
}

#[test]
fn t005_hook_range_stays_in_the_design_window() {
    // init.md §1 names 60–120 m. If this test falls over, somebody tuned the range without
    // carrying its origin along. The tighter guard sits directly below.
    let r = data().game.vector.hook_range_m;
    assert!((60.0..=120.0).contains(&r), "hook_range_m = {r} — init.md §1 names 60–120 m");
}

#[test]
fn t005_no_value_is_zero_negative_or_nan() {
    // The edge case, not the normal one: a zero in a tank size is a division by zero three
    // systems later (§9d).
    let d = data();
    let v = &d.game.vector;
    let positive = [
        ("gas_tank", v.gas_tank),
        ("hook_speed_m_s", v.hook_speed_m_s),
        ("hook_retract_speed_m_s", v.hook_retract_speed_m_s),
        ("reel_speed_m_s", v.reel_speed_m_s),
        ("min_rope_m", v.min_rope_m),
        ("boost_m_s2", v.boost_m_s2),
        ("max_speed_m_s", v.max_speed_m_s),
        ("player.height_m", d.game.player.height_m),
        ("player.run_speed_m_s", d.game.player.run_speed_m_s),
        ("player.max_substep_m", d.game.player.max_substep_m),
        ("camera.fov_deg", d.game.camera.fov_deg),
        ("world.cell_m", d.game.world.cell_m),
        ("world.half_extent_m", d.game.world.half_extent_m),
        ("world.min_wall_m", d.game.world.min_wall_m),
        ("world.collision_margin_m", d.game.world.collision_margin_m),
    ];
    for (name, value) in positive {
        assert!(value.is_finite() && value > 0.0, "{name} = {value} — must be finite and > 0");
    }
    assert!(
        d.game.gravity_m_s2 < 0.0,
        "gravity_m_s2 = {} — downwards means negative, +Y is up",
        d.game.gravity_m_s2
    );
    for (name, kind) in &d.titans.kinds {
        let height = d
            .titan_height_m(kind)
            .unwrap_or_else(|| panic!("{name}: size class {:?} does not exist", kind.size_class));
        assert!(height > 0.0, "{name}: height_m = {height}");
        assert!(
            kind.cortex_radius_m > 0.0,
            "{name}: cortex_radius_m = 0 — a cortex that is a point feels like \
             a broken game (docs/models.md)",

        );
    }
}

#[test]
fn t005_eye_height_sits_inside_the_body() {
    let s = &data().game.player;
    assert!(
        s.eye_height_m > 0.0 && s.eye_height_m < s.height_m,
        "eye_height_m = {} does not fit height_m = {}",
        s.eye_height_m,
        s.height_m
    );
}

#[test]
fn t005_a_substep_is_smaller_than_the_thinnest_wall() {
    // The core of the tunneling safeguard. At max_speed_m_s = 75 a 60 Hz tick covers 1.25 m;
    // without substeps the player drives through every wall — and only sometimes, which is
    // the worst kind of bug there is (F-012).
    let d = data();
    let substep = d.game.player.max_substep_m;
    let wall = d.game.world.min_wall_m;
    assert!(
        substep < wall,
        "max_substep_m = {substep} >= min_wall_m = {wall} — every substep can skip a wall"
    );
    assert!(
        d.game.world.collision_margin_m < substep,
        "collision_margin_m = {} must be smaller than one substep",
        d.game.world.collision_margin_m
    );
    // And the number of substeps at top speed has to stay finite and small.
    let substeps = (d.game.vector.max_speed_m_s / 60.0 / substep).ceil();
    assert!(
        (1.0..=32.0).contains(&substeps),
        "{substeps} substeps per tick at top speed — too many or none at all"
    );
}

#[test]
fn t005_the_grid_covers_map_and_hook_range() {
    // An anchor outside the grid lands in the border cell and is therefore in the wrong
    // place. The grid has to carry half the map PLUS one full hook range (T-036a).
    let d = data();
    let w = &d.game.world;
    assert!(w.cell_m > 0.0, "world.cell_m = {} — division by zero in the DDA", w.cell_m);
    assert!(w.large_body_cells >= 1, "large_body_cells = 0 puts every body into the linear list — that is exactly \
         the iteration §11 forbids");
    for (name, map) in &d.maps.maps {
        let needed = map.size_m.0.max(map.size_m.1) * 0.5
            + d.game.vector.hook_range_m;
        assert!(
            w.half_extent_m >= needed,
            "{name}: half_extent_m = {} does not cover {needed} m",
            w.half_extent_m
        );
    }
    // A cell bigger than the map would be a grid with one cell in it.
    assert!(w.cell_m < w.half_extent_m, "world.cell_m = {} is not a grid", w.cell_m);
}

#[test]
fn t005_gas_priority_names_every_consumer_exactly_once() {
    // Who pays when the tank runs low is a game-value decision (docs/QUESTIONS.md Q-017).
    // If a consumer is missing it never gets gas, and nobody goes looking for it in the RON.
    let r = &data().game.vector.gas_priority;
    for who in [GasConsumer::Boost, GasConsumer::ReelIn] {
        assert_eq!(
            r.iter().filter(|x| **x == who).count(),
            1,
            "gas_priority = {r:?} — {who:?} must appear exactly once"
        );
    }
    assert_eq!(r.len(), 2, "gas_priority = {r:?} — exactly two consumers, no more");
}

#[test]
fn t005_the_rope_solver_runs_at_least_twice() {
    // Zero passes would be a rope shut down; with one pass and two anchors the second
    // constraint violates the first one again.
    let n = data().game.vector.rope_iterations;
    assert!((1..=16).contains(&n), "rope_iterations = {n} — expected 1..16");
    assert!(n >= 2, "rope_iterations = {n} — with one pass the two-hook case (F-004) is violated \
         after a single tick");
}

#[test]
fn t005_the_current_map_is_in_maps_ron() {
    let d = data();
    assert!(
        d.current_map().is_some(),
        "maps.ron: current = {:?}, but that map is not listed under `maps`",
        d.maps.current
    );
}

#[test]
fn t005_every_block_names_a_color_from_the_palette() {
    // A reference into nothing is a bug of the same class as a dead link in the docs.
    // Without this test it only surfaces when exactly this block gets built.
    let d = data();
    for (id, map) in &d.maps.maps {
        for (i, k) in map.blocks.iter().enumerate() {
            assert!(
                d.color(&k.color).is_some(),
                "{id}: block {i} names color {:?}, which is not in the palette",
                k.color
            );
        }
        for color in &map.layout.colors {
            assert!(
                d.color(color).is_some(),
                "{id}: layout.colors names {color:?}, which is not in the palette"
            );
        }
        assert!(!map.layout.colors.is_empty(), "{id}: layout without a single color");
    }
}

#[test]
fn t005_every_map_can_be_built() {
    // The numbers `world` generates the city from: a zero or a swapped height range would be
    // an empty or an infinite city.
    let d = data();
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        assert!(map.size_m.0 > 0.0 && map.size_m.1 > 0.0, "{id}: size_m = {:?}",
                map.size_m);
        assert!(r.lot_m > 0.0 && r.street_m > 0.0, "{id}: lot_m/street_m must be > 0");
        assert!(
            r.min_height_m > 0.0 && r.min_height_m < r.max_height_m,
            "{id}: min_height_m = {} / max_height_m = {}", r.min_height_m, r.max_height_m
        );
        assert!((0.0..=1.0).contains(&r.density), "{id}: density = {} is not in 0..1", r.density);
        assert!(
            (0.0..=1.0).contains(&r.anchorable_fraction),
            "{id}: anchorable_fraction = {} is not in 0..1", r.anchorable_fraction
        );
        assert!(r.clear_radius_m > 0.0, "{id}: clear_radius_m = 0 builds a house on top of the player");
        // A street wider than a lot is not a city, it is a field with blocks on it.
        assert!(r.street_m < r.lot_m, "{id}: street_m {} >= lot_m {}", r.street_m, r.lot_m);

        for (i, k) in map.blocks.iter().enumerate() {
            let g = k.size_m;
            assert!(
                g.0 > 0.0 && g.1 > 0.0 && g.2 > 0.0,
                "{id}: block {i} has size_m = {g:?} — a cuboid without extent is not an \
                 obstacle but a division by zero"
            );
            assert!(
                k.solid || k.anchorable,
                "{id}: block {i} is neither solid nor anchorable — it would be invisible to \
                 every system that asks the index"
            );
        }
    }
}

#[test]
fn t005_the_graybox_carries_anchorable_and_untagged_surfaces() {
    // Only this makes "no hook possible on untagged parts" (F-003) runnable at all: a map on
    // which everything is anchorable cannot falsify the criterion.
    let d = data();
    let map = d.current_map().expect("current map");
    let anchorable = map.blocks.iter().filter(|k| k.anchorable).count();
    let untagged = map.blocks.iter().filter(|k| !k.anchorable).count();
    assert!(anchorable > 0, "not a single anchorable surface on the start map");
    assert!(
        untagged > 0,
        "everything is anchorable — then F-003 checks nothing (docs/FRAGEN.md Q-010)"
    );
}

#[test]
fn t005_every_third_party_asset_carries_its_attribution() {
    // §7: without an `attribution` a third-party asset is a zombie — the user cannot find it
    // later in order to replace it. As long as everything is a placeholder the list is
    // empty; that is exactly when this test still has to exist, so the first foreign model
    // gets noticed.
    for (name, model) in &data().art.models {
        if let Some(h) = &model.attribution {
            assert!(
                h.contains("http") && h.contains("20"),
                "{name}: attribution {h:?} — expected URL · date · license · what it replaces"
            );
        }
        assert!(model.scale > 0.0, "{name}: scale = {}", model.scale);
    }
}

// ===========================================================================
// The scale — assets/data/scale.ron is the ONE truth about sizes
// ===========================================================================
//
// The numbers below were laid down by the **user** (2026-08-09). They are not untuned, they
// are decided. A value in a file that nobody checks drifts away — and sizes drift especially
// quietly, because a house that is too tall does not crash, it merely looks wrong. So behind
// every size here stands a test that goes RED.
//
// The precedence rule these tests enforce: **a direct figure in meters from the user beats
// every derivation** — including the conversion out of the backlog (docs/QUESTIONS.md Q-002).

#[test]
fn t005_the_player_capsule_is_exactly_the_human_reference() {
    // Catches: a player capsule that deviates from the 1.80 m reference — the user writes
    // "check the capsule exactly!" in so many words.
    // Without this test: every size comparison in the image would be off by that same factor.
    // A 1.9 m capsule makes a 21 m titan look like 20 m and the wall like 114 m — and you
    // would go tune the titan while the error sits on the player. Exactly the kind of bug
    // you only find once somebody writes it down.
    let d = data();
    assert_eq!(
        d.game.player.height_m, d.scale.reference.human_height_m,
        "player.height_m = {} deviates from the human reference = {} (scale.ron)",
        d.game.player.height_m, d.scale.reference.human_height_m
    );
}

#[test]
fn t005_eye_height_is_the_scales_camera_height() {
    // Catches: an eye height that is not the prescribed camera height (1.60 m), and an eye
    // height above the crown of the head.
    // Without this test: the old 1.65 m would simply have stayed. They were estimated from
    // body height, and an estimate next to a prescription does not stand out — it is
    // "roughly right", after all. At a 55–65 degree field of view, five centimeters of camera
    // height are exactly the difference between "I am standing in front of it" and "I am
    // floating in front of it".
    let d = data();
    let s = &d.game.player;
    assert_eq!(
        s.eye_height_m, d.scale.camera.height_m,
        "player.eye_height_m = {} != scale.ron camera.height_m = {}",
        s.eye_height_m, d.scale.camera.height_m
    );
    assert!(
        s.eye_height_m < s.height_m,
        "eye_height_m = {} is not below height_m = {} — the camera floats above the head",
        s.eye_height_m, s.height_m
    );
}

#[test]
fn t005_hook_range_is_the_users_anchor_range() {
    // Catches: every relapse to the derived 112 m (400 studs × 0.28, Q-002). The user states
    // 90 m directly, and a direct figure in meters beats every derivation.
    // Without this test: the 112 would come back with the next "let me recompute that from
    // the backlog" — with a rationale and a source, and therefore especially convincing. The
    // range decides whether the wall is reachable in two moves and how large the spatial grid
    // has to be; changing it quietly by 24 % moves both.
    let d = data();
    assert_eq!(
        d.game.vector.hook_range_m, d.scale.vector.anchor_range_m,
        "vector.hook_range_m = {} != scale.ron vector.anchor_range_m = {}",
        d.game.vector.hook_range_m, d.scale.vector.anchor_range_m
    );
}

#[test]
fn t005_ground_fov_stays_in_the_users_window() {
    // Catches: a field of view outside 55–65 degrees. The user calls it the "biggest lever",
    // and he explicitly names GROUND COMBAT — 60 degrees is the baseline, not the ceiling.
    // Without this test: the old 90 degrees would come back the moment somebody feels "too
    // cramped". 90 degrees make every titan small and every meter short; that is exactly the
    // perception this game must not have. The second assert pins down that the speed FOV
    // (F-017) goes UP — a smaller value would be a camera that zooms in at top speed instead
    // of opening up, that is, the opposite of a sense of speed.
    let d = data();
    let k = &d.game.camera;
    let m = &d.scale.camera;
    assert!(
        (m.min_ground_fov_deg..=m.max_ground_fov_deg).contains(&k.fov_deg),
        "camera.fov_deg = {} is not in {}..={} (scale.ron, ground-combat FOV)",
        k.fov_deg, m.min_ground_fov_deg, m.max_ground_fov_deg
    );
    assert!(
        k.fov_max_speed_deg >= k.fov_deg,
        "fov_max_speed_deg = {} < fov_deg = {} — F-017 opens the image up with speed, \
         it does not close it",
        k.fov_max_speed_deg, k.fov_deg
    );
    assert!(
        k.fov_max_speed_deg < 180.0,
        "fov_max_speed_deg = {} — from 180 degrees on the projection matrix degenerates",
        k.fov_max_speed_deg
    );
}

#[test]
fn t005_every_titan_kind_carries_exactly_one_size_class() {
    // Catches: a kind that points at a size class which does not exist in scale.ron (typo,
    // deleted class, invented class) — F-064.
    // Without this test: `titan_height_m()` returns `None`, and the first caller who writes
    // `unwrap_or(0.0)` spawns a titan of height zero. That one stands in the ground, its
    // cortex sits at 0 m, and you go looking for the bug in the collision code instead of in
    // one letter in one file.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).unwrap_or_else(|| {
            panic!(
                "{name}: size_class = {:?} is not in scale.ron titan.classes — \
                 known are {:?}",
                kind.size_class,
                d.scale.titan.classes.keys().collect::<Vec<_>>()
            )
        });
        // And the class really has to be a size: taller than a human, shorter than the
        // Ashwalker. Anything else is not a titan, it is a typo with a decimal point in it.
        assert!(
            height > d.scale.reference.human_height_m
                && height <= d.scale.titan.ashwalker_height_m,
            "{name}: class {:?} is {height} m — that is not a titan",
            kind.size_class
        );
    }
}

#[test]
fn t005_the_cortex_sits_at_eighty_nine_percent_of_the_height() {
    // Catches: a cortex height that no longer sits at ~89 % (F-030). The user gives five
    // pairs, all of which come out at 0.881–0.893; "~89 %" is his rounding, 88–90 % is the
    // window.
    //
    // ⚠️ Since 2026-08-09 this test checks a **ratio between two of the user's meter figures**
    // instead of a number against its own formula. Before that, `cortex_height_m` was
    // computed from `height_m * cortex_fraction` — and then this assert can never go red,
    // because it recomputes exactly the calculation that produced the value. A guard over a
    // derivation is decoration. Now both numbers stand in the RON, and the fraction is what
    // it is for the user: the rule by which you notice that one of the two has drifted.
    //
    // ALL classes are checked, not just the ones in use: `boss` (28 m) has no representative
    // in titan.ron today, and that is exactly why a mistake there would go unnoticed.
    let d = data();
    let m = &d.scale.titan;
    for (name, k) in &m.classes {
        let fraction = k.cortex_height_m / k.height_m;
        assert!(
            (0.88..=0.90).contains(&fraction),
            "class {name}: cortex {} m of {} m = {fraction} — the user prescribes ~89 % \
             (3.7/4.2 · 8.9/10 · 12.5/14 · 18.7/21 · 24.9/28)",
            k.cortex_height_m, k.height_m
        );
        // The cortex sits on the body, not above it: otherwise the marker points at thin air.
        assert!(
            k.cortex_height_m < k.height_m,
            "class {name}: cortex at {} m above the crown {} m", k.cortex_height_m, k.height_m
        );
        // And the number has to be a STATED figure, not a calculation. The user names his
        // cortex heights to the decimeter (3.7 · 8.9 · 12.5 · 18.7 · 24.9); whatever falls
        // out of `height_m * 0.89` has three decimal places (4.2 × 0.89 = 3.738). Without
        // this assert the relapse into the derivation would be invisible — 3.738 sits right
        // in the middle of the window above, and that is exactly why the window cannot
        // catch it.
        let decimeter = (k.cortex_height_m * 10.0).round() / 10.0;
        assert!(
            (k.cortex_height_m - decimeter).abs() < 1e-4,
            "class {name}: cortex_height_m = {} is not stated to a decimeter — it looks like \
             `height_m * cortex_fraction`. The five cortex heights are the users meter \
             values, not a derivation (docs/models.md)",
            k.cortex_height_m
        );
    }
    // And the fraction itself has to be the center of that window — otherwise the rule checks
    // something other than what it claims.
    assert!(
        (m.cortex_fraction - 0.89).abs() < 0.005,
        "cortex_fraction = {} — the user writes \"cortex at ~89 %\"", m.cortex_fraction
    );
    // Every kind reaches its cortex height through its class, with no detour via a formula.
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).expect("size class");
        let cortex = d.titan_cortex_height_m(kind).expect("cortex height");
        assert!(cortex > 0.0 && cortex < height, "{name}: cortex {cortex} m / height {height} m");
    }
}

#[test]
fn t005_the_cortex_fits_under_the_titans_head() {
    // Catches: a hit zone larger than the titan's entire head.
    // Without this test: that was exactly the state of things, and nobody could see it,
    // because until 2026-08-09 the user's head rule stood nowhere as a number. `scuttler` had
    // cortex_radius_m 0.40 — so 0.80 m of diameter — on a 4.2 m body whose head, by the
    // 1/9..1/10 rule, measures only 0.42..0.47 m. The cortex was almost twice the size of the
    // head; `weaver` (0.90 m) even more so. Geometrically impossible, and in the image the
    // small titan wears a bullseye where its neck should be.
    //
    // The test is deliberately only an UPPER BOUND. Whether the radius should grow with body
    // size is an open question for the user (docs/QUESTIONS.md Q-019) — that it must not
    // exceed the head is not.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).expect("size class");
        let head = d.titan_max_head_height_m(kind).expect("head height");
        let diameter = 2.0 * kind.cortex_radius_m;
        assert!(
            diameter <= head,
            "{name}: cortex_radius_m = {} ⇒ {diameter} m diameter on a {height} m \
             body whose head is at most {head} m tall (scale.ron \
             titan.max_head_fraction). A hit zone larger than the head cannot be a \
             neck joint",
            kind.cortex_radius_m
        );
    }
}

#[test]
fn t005_the_streets_stay_as_narrow_as_the_user_wants() {
    // Catches: a street width outside 6–8 m. The user writes "keep them narrow".
    // Without this test: the old 9 m would come back the moment somebody turns a knob on the
    // layout, and wider never stands out — it looks tidier, after all. But narrow streets are
    // the reason speed feels like speed: the wall flying past has to be close.
    let d = data();
    let r = &d.scale.reference;
    for (id, map) in &d.maps.maps {
        assert!(
            (r.min_street_m..=r.max_street_m).contains(&map.layout.street_m),
            "{id}: layout.street_m = {} is not in {}..={} (scale.ron)",
            map.layout.street_m, r.min_street_m, r.max_street_m
        );
    }
}

#[test]
fn t005_the_generated_city_stays_residential() {
    // Catches: generated houses outside the residential band (4.5 m small house to 11.5 m
    // large house). This used to say 8–34 m; 34 m is the size of a church.
    // Without this test: the city creeps back up, because "more anchor points up high" sounds
    // reasonable in every single case. **The city is MEANT to be flat.** The vertical comes
    // from the wall (120 m), the church (35 m), the watchtower and the trees — and those only
    // work as long as the residential band does not catch up with them. A skyline would not
    // be a balancing mistake, it would be a different game.
    let d = data();
    let h = &d.scale.architecture.heights_m;
    let low = h["house_small"];
    let high = h["house_large"];
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        assert!(
            r.min_height_m >= low,
            "{id}: min_height_m = {} is below the small house ({low} m)", r.min_height_m
        );
        assert!(
            r.max_height_m <= high,
            "{id}: max_height_m = {} is above the large house ({high} m) — that is landmark \
             height (church {} m) and belongs in `blocks` as a placed entry, \
             not rolled",
            r.max_height_m, h["church"]
        );
    }
}

#[test]
fn t005_placed_blocks_stay_residential_too() {
    // Catches: an explicitly placed cuboid above 11.5 m that does not declare itself a
    // landmark.
    // Without this test: half the city stands outside the rule that is supposed to keep it
    // flat. That was exactly the state of things — the guard above read only `layout`, while
    // `blocks` carried ridge heights of 12.0 / 14.0 / 18.0 m. 18 m is precisely the height
    // that the same test forbids the layout as "landmark height", and those three were not
    // landmarks, they were gray cubes.
    //
    // Only an UPPER BOUND, no lower one: a parapet, a wall panel and the ground slab may be
    // flatter than a small house. Upwards, `landmark` alone decides, and that is the point —
    // the exception has to name itself.
    let d = data();
    let high = d.scale.architecture.heights_m["house_large"];
    for (id, map) in &d.maps.maps {
        for (i, k) in map.blocks.iter().enumerate() {
            if k.landmark {
                continue;
            }
            let ridge = k.center_m.1 + k.size_m.1 * 0.5;
            assert!(
                ridge <= high,
                "{id}: block {i} has ridge height {ridge} m and thus stands above the \
                 residential band ({high} m). Either it is a structure from \
                 scale.ron:architecture.heights_m — then `landmark: true` — or it gets \
                 shortened. A gray cube is no reason to raise the city"
            );
        }
    }
}

#[test]
fn t005_every_size_class_has_a_structure_above_its_cortex() {
    // Catches: a size class whose cortex sits above every structure the scale knows about —
    // that is, a titan you cannot approach from above in this world.
    // Without this test: the size table created three classes (14 / 21 / 28 m) whose cortex
    // sits at 12.5 / 18.7 / 24.9 m, while the tallest residential house measures 11.5 m. Roof
    // height + min_rope_m gives an **anchor ceiling**; above it no rope holds. Every approach
    // would be ballistic — let go, fly, one pass with no correction — and the blame would go
    // to the rope release, the boost and the camera, because the cause is two numbers in two
    // other files that nobody holds up against each other.
    // The user's numbers stay; what changes is the composition of the city (church 35 m,
    // watchtower 12 m, tree 12 m). docs/QUESTIONS.md Q-022.
    let d = data();
    let m = &d.scale;
    let (tallest, height) = m
        .architecture
        .heights_m
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("scale.ron without a single structure");
    for (name, k) in &m.titan.classes {
        assert!(
            *height >= k.cortex_height_m,
            "class {name}: cortex at {} m, but the tallest structure is {tallest} with \
             {height} m — there is no anchor above that cortex",
            k.cortex_height_m
        );
    }
}

#[test]
fn t005_the_start_map_really_carries_the_vertical() {
    // Catches: a map that places nothing but residential buildings while the docs claim the
    // vertical comes from landmarks.
    // Without this test: exactly that gap. `maps.ron` promised itself "church, watchtower and
    // wall are placed as `blocks`" — and not one map contained any of them. The statement was
    // an intention, not a state, and the arithmetic behind it (highest anchor 11.5 m +
    // min_rope_m 3.0 m = a 14.5 m anchor ceiling) hit exactly the three size classes the size
    // table had just created.
    //
    // What is checked is the map that actually gets built — a number in `scale.ron` is not an
    // anchor, a block in the map is one.
    let d = data();
    let map = d.current_map().expect("current map");
    let high = d.scale.architecture.heights_m["house_large"];
    let highest_anchor = map
        .blocks
        .iter()
        .filter(|k| k.anchorable)
        .map(|k| k.center_m.1 + k.size_m.1 * 0.5)
        .fold(0.0f32, f32::max);
    let ceiling = highest_anchor + d.game.vector.min_rope_m;
    assert!(
        highest_anchor > high,
        "the highest ANCHORABLE point of the start map is at {highest_anchor} m — not \
         above the residential band ({high} m). Without an anchorable landmark the \
         vertical is a claim"
    );
    // And it has to carry the largest class titan.ron actually uses.
    let highest_cortex = d
        .titans
        .kinds
        .values()
        .filter_map(|a| d.titan_cortex_height_m(a))
        .fold(0.0f32, f32::max);
    assert!(
        ceiling >= highest_cortex,
        "anchor ceiling {ceiling} m, highest cortex of a used titan kind {highest_cortex} m \
         — every approach to that target would be ballistic (docs/FRAGEN.md Q-022)"
    );
}

#[test]
fn t005_every_map_carries_its_own_layout() {
    // Catches: a map smaller than the layout that is supposed to stand on it — and a clear
    // area around the origin that eats half the map.
    // Without this test: the city would have one or zero complete blocks, the layout would
    // generate almost nothing, and you would look for the bug in the generator instead of in
    // two numbers.
    let d = data();
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        let period = r.lot_m + r.street_m;
        let shortest_edge = map.size_m.0.min(map.size_m.1);
        assert!(
            shortest_edge >= 4.0 * period,
            "{id}: a map of {shortest_edge} m does not carry four grid periods of {period} m",
        );
        assert!(
            2.0 * r.clear_radius_m < shortest_edge,
            "{id}: clear_radius_m = {} eats the map ({shortest_edge} m)",
            r.clear_radius_m
        );
    }
}

#[test]
fn t005_the_scale_factors_stay_unequal() {
    // Catches: the "tidier-upper" who levels the three factors to 1.0 because unequal scales
    // look like a mistake.
    // Without this test: exactly that happens, with a clear conscience and in a one-liner.
    // Titans exaggerated by 1.4 and walls by 2.4 are the VISUAL LANGUAGE of the reference
    // work: the human small, the threat out of all proportion, the wall a horizon. A uniform
    // scale would be technically clean and artistically dead.
    let m = &data().scale;
    for (name, f) in [
        ("architecture_factor", m.architecture_factor),
        ("titan_factor", m.titan_factor),
        ("wall_factor", m.wall_factor),
    ] {
        assert!(f.is_finite() && f > 0.0, "{name} = {f} — must be finite and > 0");
    }
    assert!(
        m.titan_factor > m.architecture_factor,
        "titan_factor = {} is not greater than architecture_factor = {} — then titans are \
         no longer exaggerated",
        m.titan_factor, m.architecture_factor
    );
    assert!(
        m.wall_factor > m.titan_factor,
        "wall_factor = {} is not greater than titan_factor = {} — then the wall is no \
         horizon any more, just a big wall",
        m.wall_factor, m.titan_factor
    );
}

#[test]
fn t005_the_ashwalker_rises_thirty_meters_above_the_wall() {
    // Catches: every change to wall height or Ashwalker height that breaks the user's own
    // check — "150 m, 30 m above the wall".
    // Without this test: somebody lowers the wall to 100 m because it feels "unreachable",
    // and the boss's entrance loses exactly the image it is 150 m tall for. The relation
    // between the two numbers is the claim, not their magnitude.
    let m = &data().scale;
    let overhang = m.titan.ashwalker_height_m - m.wall.height_m;
    assert!(
        (overhang - 30.0).abs() < 0.01,
        "Ashwalker {} m − wall {} m = {overhang} m, the user prescribes 30 m",
        m.titan.ashwalker_height_m, m.wall.height_m
    );
}

#[test]
fn t005_the_wall_is_reachable_in_two_moves() {
    // Catches: a wall whose crown (120 m) or whose intermediate platform (60 m) is no longer
    // reachable within the anchor range (90 m).
    // Without this test: the three numbers wander independently — range down for balancing,
    // wall up for effect — and at some point the wall can no longer be climbed from below.
    // That only surfaces when somebody tries it in game, and then the guess is "the controls
    // are broken", not "three RON numbers do not fit together".
    let d = data();
    let m = &d.scale.wall;
    let range = d.scale.vector.anchor_range_m;
    assert!(
        m.platform_height_m > 0.0 && m.platform_height_m < m.height_m,
        "platform_height_m = {} is not between the ground and the crown ({} m)",
        m.platform_height_m, m.height_m
    );
    assert!(
        m.platform_height_m <= range,
        "ground -> platform is {} m, the anchor range is {range} m",
        m.platform_height_m
    );
    assert!(
        m.height_m - m.platform_height_m <= range,
        "platform -> crown is {} m, the anchor range is {range} m",
        m.height_m - m.platform_height_m
    );
    // Battered means: thicker at the base than at the top. The other way round would be an
    // overhang no hook holds on to and under which every titan would find cover.
    assert!(
        m.base_thickness_m > m.top_thickness_m,
        "base_thickness_m = {} <= top_thickness_m = {} — the wall overhangs",
        m.base_thickness_m, m.top_thickness_m
    );
}

#[test]
fn t005_the_walls_scale_ladder_stays_readable() {
    // Catches: a stone course or a banding too coarse to read size off.
    // Without this test: both numbers look like decoration and get cut in the first
    // performance conversation ("less geometry on the wall"). Then the 120 m wall is a gray
    // surface, the eye has no ladder, and up close the wall reads like a 12 m wall. The stone
    // course has to be clearly smaller than a human — otherwise it is not a scale, just a
    // pattern.
    let m = &data().scale;
    assert!(
        m.wall.stone_course_m > 0.0 && m.wall.stone_course_m < m.reference.human_height_m * 0.5,
        "stone_course_m = {} — as a scale ladder a course must sit well below half a \
         human height ({} m)",
        m.wall.stone_course_m, m.reference.human_height_m * 0.5
    );
    assert!(
        m.wall.band_spacing_m > m.wall.stone_course_m && m.wall.band_spacing_m < m.wall.height_m,
        "band_spacing_m = {} must be coarser than a stone course and finer than the wall",
        m.wall.band_spacing_m
    );
    // Enough bands for the ladder to be a ladder at all.
    let bands = (m.wall.height_m / m.wall.band_spacing_m).floor();
    assert!(bands >= 4.0, "only {bands} bands on {} m of wall", m.wall.height_m);
}

#[test]
fn t005_every_eaves_belongs_to_a_structure_that_exists() {
    // Catches: an eaves height without a total height (typo in the key) and eaves that sit
    // above the ridge.
    // Without this test: the modeler builds a house with 6 m eaves and 4.5 m total height —
    // a roof that points downwards. A reference into nothing is the same bug as a dead link
    // in the docs, except that nobody clicks this one.
    let a = &data().scale.architecture;
    for (name, eaves) in &a.eaves_m {
        let height = a.heights_m.get(name).unwrap_or_else(|| {
            panic!("eaves_m names {name:?}, which is not in heights_m")
        });
        assert!(
            *eaves > 0.0 && eaves < height,
            "{name}: eaves {eaves} m do not fit under the total height {height} m"
        );
    }
    for (name, height) in &a.heights_m {
        assert!(*height > 0.0, "architecture.heights_m[{name}] = {height}");
    }
    assert!(!a.heights_m.is_empty(), "scale.ron without a single structure");
}

#[test]
fn t005_the_titan_head_stays_smaller_than_the_human_head() {
    // Catches: a head-size rule that makes the titan head relatively as large as the human
    // head (1/7.5).
    // Without this test: the first person to make a titan model "more proportional" raises
    // exactly this number — and from then on the model looks like a human standing too close
    // instead of a 21 m body. The relatively small head IS the impression of size; together
    // with cortex_fraction it decides whether the cortex is readable at 100 m (F-030).
    let m = &data().scale;
    let t = &m.titan;
    assert!(
        t.min_head_fraction > 0.0 && t.min_head_fraction < t.max_head_fraction,
        "min_head_fraction = {} / max_head_fraction = {} — min must be strictly smaller",
        t.min_head_fraction, t.max_head_fraction
    );
    assert!(
        t.max_head_fraction < m.reference.human_head_fraction,
        "max_head_fraction = {} is not smaller than the human fraction {} (1/7.5) — then \
         the titan reads as a nearby human, not as a big body",
        t.max_head_fraction, m.reference.human_head_fraction
    );
    // The user writes "1/9 - 1/10". The window has to really CONTAIN both fractions: at
    // 0.1111 a model built exactly on 1/9 = 0.111111… falls out of its own prescription by
    // one ten-thousandth. That is not a rounding detail, it is an upper bound that later gets
    // checked against.
    assert!(
        t.min_head_fraction <= 1.0 / 10.0 && t.max_head_fraction >= 1.0 / 9.0,
        "head fraction {}..{} does not include 1/10 = {} and 1/9 = {}",
        t.min_head_fraction, t.max_head_fraction, 1.0 / 10.0, 1.0 / 9.0_f32
    );
}

#[test]
fn t005_the_grid_carries_the_worlds_height_too() {
    // Catches: a world that grows taller than the grid indexing it.
    // Without this test: `half_extent_m` was computed against the map in the PLANE
    // (400/2 + 90 = 290) and never against height. In Y the wall (120 m) and the Ashwalker
    // (150 m) stand on top of each other — 270 m, so 30 m of margin. Whether the grid is
    // three-dimensional at all is open (docs/QUESTIONS.md Q-014); that a grid must not be
    // smaller than the world it holds is not. A body outside it lands in the border cell and
    // is therefore in the wrong place — and that looks like a bug in the ray cast.
    let d = data();
    let m = &d.scale;
    let highest_point = m.wall.height_m + m.titan.ashwalker_height_m;
    assert!(
        d.game.world.half_extent_m >= highest_point,
        "world.half_extent_m = {} does not cover wall ({} m) + Ashwalker ({} m) = \
         {highest_point} m",
        d.game.world.half_extent_m, m.wall.height_m, m.titan.ashwalker_height_m
    );
}

/// A number the way `docs/models.md` writes it: decimal point, no trailing zero.
fn doc_number(value: f32) -> String {
    format!("{value}")
}

#[test]
fn t005_the_size_table_in_the_docs_shows_the_same_numbers() {
    // Catches: every number in `scale.ron` that no longer has its row in `docs/models.md` —
    // and every new structure missing from the doc.
    // Without this test: `docs/models.md` is a second, complete and entirely unguarded copy
    // of the same ~30 numbers. Today the file secures that with the sentence "both are
    // changed together or not at all" — which is a request, not a guard. And it is the
    // version the **modeler** reads: a doc that quietly deviates from the data is worse than
    // no doc, because people build to it.
    //
    // What is checked is the cell form "| <number> m", not the bare number — otherwise `8 m`
    // would find itself inside `18 m` and the test would be green while knowing nothing.
    let d = data();
    let m = &d.scale;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/models.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));

    let mut expected: Vec<(String, String)> = vec![
        ("reference.human_height_m".into(), format!("| {} m", doc_number(m.reference.human_height_m))),
        ("reference.door_height_m".into(), format!("| {} m", doc_number(m.reference.door_height_m))),
        ("wall.height_m".into(), format!("| {} m", doc_number(m.wall.height_m))),
        ("wall.top_thickness_m".into(), format!("| {} m", doc_number(m.wall.top_thickness_m))),
        ("wall.base_thickness_m".into(), format!("| {} m", doc_number(m.wall.base_thickness_m))),
        ("wall.platform_height_m".into(), format!("| {} m", doc_number(m.wall.platform_height_m))),
        ("wall.stone_course_m".into(), format!("| {} m", doc_number(m.wall.stone_course_m))),
        ("wall.band_spacing_m".into(), format!("| {} m", doc_number(m.wall.band_spacing_m))),
        ("titan.ashwalker_height_m".into(), format!("| {} m", doc_number(m.titan.ashwalker_height_m))),
        ("camera.height_m".into(), format!("| {} m", doc_number(m.camera.height_m))),
        ("vector.anchor_range_m".into(), format!("| {} m", doc_number(m.vector.anchor_range_m))),
    ];
    for (name, height) in &m.architecture.heights_m {
        expected.push((format!("architecture.heights_m[{name}]"), format!("| {} m", doc_number(*height))));
    }
    for (name, class) in &m.titan.classes {
        expected.push((format!("titan.classes[{name}].height_m"),
                   format!("| {} m", doc_number(class.height_m))));
        expected.push((format!("titan.classes[{name}].cortex_height_m"),
                   format!("Cortex {} m", doc_number(class.cortex_height_m))));
    }

    for (source, wanted) in expected {
        assert!(
            text.contains(&wanted),
            "docs/models.md does not contain {wanted:?} — the doc deviates at {source} from \
             assets/data/scale.ron. Both change together or not at all"
        );
    }
}
