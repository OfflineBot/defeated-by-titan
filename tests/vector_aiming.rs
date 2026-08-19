//! `F-002` — free aiming by ray. The guard over `src/vector/aim.rs`.
//!
//! `F-002` demands a ray "from the camera position along the look direction, range = range
//! stat", whose hit point "is checked against a valid anchor surface". Three of the four ways
//! of getting that wrong pass every test that only measures a **distance**:
//!
//! 1. The ray starts at the player's origin instead of at his eye. It still lands on the same
//!    wall *plane*, 1.6 m too low — and the crosshair points somewhere the hook does not go.
//! 2. The ray is cast with a filter for anchorable bodies. It then travels **through** the
//!    untagged wall to the roof behind it, which is exactly what `F-023` forbids
//!    ("line-of-sight check prevents hooking through walls").
//! 3. `hook_range_m` is a comment rather than a limit, or is a number in Rust rather than the
//!    90 m out of `assets/data/game.ron`.
//!
//! Each of the three has a test here that measures the **full three-dimensional hit point**
//! against a coordinate computed from `assets/data/maps.ron` — never a distance alone.
//!
//! The fourth way is the one nothing in the game can see: a ray that hits the player's own
//! capsule. The eye sits at 1.6 m *inside* a capsule spanning 0 .. 1.8 m, so an unexcluded
//! shot reports a hit at zero distance, every tick, for every player.
//!
//! ## Why these tests drive with `app.update()`
//!
//! The same reason as in `tests/player.rs`: avian takes its step size from the *generic*
//! `Time` (`avian3d-0.7.0/src/schedule/mod.rs:238-244`), which only `run_fixed_main_schedule`
//! switches over. Running `FixedMain` by hand measures the machine instead of the game.
//! `TimeUpdateStrategy::FixedTimesteps(1)` makes one `update()` exactly one simulation step.
//!
//! ## Why the test player is never the local one
//!
//! `net::local::read_input` refills the local player's `Intent` out of the keyboard on every
//! tick, and a keyboard knows no absolute look angle. A second player has exactly one source
//! of intents — the inbox — and that is the same channel the network will use
//! (`docs/multiplayer.md`). It is also the honest shape of the rule: there is no such thing
//! as *the* player.
//!
//! ## The fifth way, and it is the one that got through — `B-001`
//!
//! [`AimPoint`] has three fields and this file only ever measured two of them. A hit point
//! that is correct to the centimetre and an `anchorable` that is correct to the block is
//! **worth nothing to the only consumer there is**: `vector::hook::anchor_target` reads
//! `aim.body`, and without it returns `None` — every shot in the running game ended as
//! `ReleaseReason::NoAnchor` while all seven tests below stayed green. `grep -n '\.body'` over
//! this file returned nothing until 2026-08-09.
//!
//! That is the shape of the gap: a test suite that measures what the system *computes*
//! instead of what its consumer *needs*. `f002_the_aim_names_the_body_it_hit` closes it.
//!
//! The picture that belongs to these numbers is `docs/images/f-002-aiming.png`, taken with
//! `scripts/f-002-aiming.txt`.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::GameData;
use defeated_by_titan::net::Inbox;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::Velocity;
use defeated_by_titan::vector::aim::{
    effective_spread_rad, separation_m, settle_distance_m, side_dirs, side_hit_is_coherent,
    slew_spread_rad, wheel_half_rad, AimSpread, SpreadContext,
};
use defeated_by_titan::shared::{
    AimPoint, AnchorSurface, ArmAim, Body, BodyId, BodyMask, Buttons, Cli, Hook, HookState,
    IdCounter, Intent, MovementState, PlayerId, Side, Tick, WarpPlayer,
};

/// Builds the **real** app, headless, one simulation step per `update()`, on the map named
/// here — **not** on whatever `maps.ron: current` happens to say.
///
/// Every fixture in this file is a coordinate: the untagged wall at `z = -33.5`, the 8 m cube
/// at `(-12, 4, -20)`, the brick-red house at `z = -41`. Those are the **graybox**'s, and a
/// test that inherits `current` asserts on them in whatever world the level design last
/// switched to. On 2026-08-12 that was measured: with `current: "ashgate"` six of the nine
/// tests below went red — not one of them about the aim ray, all of them about a wall that is
/// not in that district. So the map is pinned per test.
///
/// `GameData` is inserted by `data::DataPlugin` during `add_plugins` (`src/lib.rs:71` reads it
/// right after), i.e. **before** the first `update()` runs `Startup` — and `world::map::
/// build_map` takes the name out of the resource, not out of the file. That is the seam; it
/// needed nothing new.
fn app_on(map: &str) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.world_mut().resource_mut::<GameData>().maps.current = map.to_string();
    assert!(
        app.world().resource::<GameData>().current_map().is_some(),
        "maps.ron lists no map {map:?} — a typo here builds an empty world and every \
         assertion below turns into `nothing hit`"
    );
    app.update(); // Startup: the city out of `maps.ron` and the local player come into being
    app
}

/// The graybox — the map the coordinates in this file were measured in.
fn app() -> App {
    app_on("graybox")
}

/// Whatever `maps.ron: current` names: the map that actually ships. Only for the tests that
/// make a statement about *the map*, not about a fixture inside one.
fn app_on_current_map() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.update();
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

/// A second player at `pos` — **without** the `LocalPlayer` marker, so nothing but the inbox
/// ever writes his `Intent`.
fn test_player(app: &mut App, pos: Vec3) -> (Entity, PlayerId) {
    let world = app.world_mut();
    let data = world.resource::<GameData>().clone();
    let mut ids = world.resource::<IdCounter>().to_owned();
    let mut commands = world.commands();
    let e = spawn_player(&mut commands, &mut ids, &data, pos, false);
    *world.resource_mut::<IdCounter>() = ids;
    app.update();
    let id = *app.world().get::<PlayerId>(e).expect("a fresh player carries his id");
    (e, id)
}

/// Posts a look direction into the inbox and runs **one** step, so that the aim ray of that
/// step is cast along exactly this direction. Degrees in, radians on the wire
/// (`docs/conventions.md`).
fn look(app: &mut App, id: PlayerId, yaw_deg: f32, pitch_deg: f32) {
    let tick = app.world().resource::<Tick>().0;
    app.world_mut().resource_mut::<Inbox>().push(
        id,
        Intent {
            yaw: yaw_deg.to_radians(),
            pitch: pitch_deg.to_radians(),
            tick,
            ..default()
        },
        tick,
    );
    app.update();
}

/// `warp` through the same message the script driver uses — position exact, velocity zero.
fn warp(app: &mut App, id: PlayerId, pos: Vec3) {
    app.world_mut().write_message(WarpPlayer {
        player: id,
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
    });
    app.update();
}

fn aim_of(app: &App, e: Entity) -> AimPoint {
    *app.world().get::<AimPoint>(e).expect("every player carries an AimPoint from tick 1")
}

fn at(app: &App, e: Entity) -> Vec3 {
    app.world().get::<Transform>(e).expect("the player has a transform").translation
}

/// The eye the ray really starts from — computed here out of `game.ron`, not read out of the
/// system under test.
fn eye(app: &App, e: Entity) -> Vec3 {
    at(app, e) + Vec3::Y * data(app).game.player.eye_height_m
}

// ---------------------------------------------------------------------------------------
// 1. The full coordinate — not "distance to the plane"
// ---------------------------------------------------------------------------------------

#[test]
fn f002_the_aim_point_is_the_whole_coordinate_and_not_just_the_plane() {
    // The target is the untagged wall out of `maps.ron`: center (-30, 5, -34), size
    // (14, 10, 1) — so its near face is the plane z = -33.5, spanning x -37..-23 and
    // y 0..10.
    //
    // A ray from the player's ORIGIN instead of his eye lands on that same plane, at the
    // same z, at almost the same distance. What it does not land on is y = 1.6 — and that
    // is the whole difference between a crosshair and a decoration.
    let mut app = app();
    let d = data(&app);
    let (e, id) = test_player(&mut app, Vec3::new(-30.0, 0.0, -20.0));
    ticks(&mut app, 60); // land and settle: standing is a measured state, not an assumption

    look(&mut app, id, 0.0, 0.0); // yaw 0 = along -Z (the axis contract)

    let eye = eye(&app, e);
    let hit = aim_of(&app, e).point_m.expect("the wall stands 13.5 m in front of him");
    let expected = Vec3::new(eye.x, eye.y, -33.5);

    assert!(
        (hit - expected).length() < 0.02,
        "aim point {hit:?} instead of {expected:?} — eye at {eye:?}"
    );
    // Said once more the way it goes red: the height is the EYE height, not the ground.
    assert!(
        (hit.y - d.game.player.eye_height_m).abs() < 0.02,
        "the ray landed at y = {} instead of at eye height {} — it starts between the feet",
        hit.y,
        d.game.player.eye_height_m
    );
    assert!(
        hit.y > 1.0,
        "y = {} is the player's origin, not his eye (game.ron: player.eye_height_m = {})",
        hit.y,
        d.game.player.eye_height_m
    );
}

// ---------------------------------------------------------------------------------------
// 2. F-023 — first the hit, then the question whether it is anchorable
// ---------------------------------------------------------------------------------------

#[test]
fn f002_an_untagged_wall_in_front_of_a_roof_is_not_hookable_and_not_transparent() {
    // `maps.ron` keeps this pair for exactly this criterion: an untagged wall at z = -33.5
    // and, 7.5 m behind it, the anchorable brick-red house whose near face is z = -41.
    //
    // A ray filtered for anchorable bodies reports the house: same direction, 21 m instead
    // of 13.5 m, `anchorable: true`. That is "hooking through a wall" (`F-023`), and it is
    // invisible in a screenshot because the rope end lies inside the building.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(-30.0, 0.0, -20.0));
    ticks(&mut app, 60);

    look(&mut app, id, 0.0, 0.0);

    let a = aim_of(&app, e);
    let hit = a.point_m.expect("something stands in that direction");
    assert!(
        (hit.z + 33.5).abs() < 0.02,
        "the ray landed at z = {} — the untagged wall stands at z = -33.5, the anchorable \
         house behind it at z = -41. Anything past -34 means the ray went THROUGH the wall",
        hit.z
    );
    assert!(
        !a.anchorable,
        "the untagged wall reports itself as anchorable — then `AnchorSurface` decides nothing"
    );

    // And the roof behind it really is reachable when the line of sight is clear —
    // otherwise this test would also pass in a world where the ray hits nothing at all, and
    // "not hookable" would be a statement about the ray rather than about the wall.
    //
    // From an eye at 16.6 m, 11.5 degrees down: at the wall (13.5 m ahead) the ray is at
    // 13.9 m and clears its 10 m top edge; it comes down on the house roof at y = 11.5,
    // which spans z = -51 .. -41.
    let pitch_deg = -11.5_f32;
    warp(&mut app, id, Vec3::new(-30.0, 15.0, -20.0));
    look(&mut app, id, 0.0, pitch_deg);

    let eye = eye(&app, e);
    let pitch = pitch_deg.to_radians();
    // Distance along the ray down to the roof plane y = 11.5.
    let t = (11.5 - eye.y) / pitch.sin();
    let expected = Vec3::new(eye.x, 11.5, eye.z - t * pitch.cos());

    let over = aim_of(&app, e);
    let roof = over.point_m.expect("over the wall the house roof stands free");
    assert!(
        roof.z < -41.0,
        "the ray landed at z = {} — that is still the wall (z = -33.5), not the roof behind it",
        roof.z
    );
    assert!(
        (roof - expected).length() < 0.05,
        "roof hit {roof:?} instead of {expected:?} (eye {eye:?}, pitch {pitch_deg} deg)"
    );
    assert!(over.anchorable, "the brick-red house is tagged `anchorable: true` in maps.ron");
}

// ---------------------------------------------------------------------------------------
// 3. Free aiming — the hit point is continuous, not one of a handful of placed anchors
// ---------------------------------------------------------------------------------------

#[test]
fn f002_free_aiming_hits_any_point_of_a_tagged_face_not_a_placed_anchor() {
    // The brick-red 8 m cube at (-12, 4, -20): near face z = -16, spanning x -16..-8 and
    // y 0..8. Nine directions, nine different points on that one face — computed from the
    // geometry, not read back out of the system.
    //
    // This is what "free" means in `F-002`: the layer is a ray, not a lookup of anchor
    // points somebody placed. A snap implementation would return the same point nine times.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(-12.0, 0.0, 0.0));
    ticks(&mut app, 60);

    let face_z = -16.0_f32;
    let mut seen: Vec<Vec3> = Vec::new();

    for yaw_deg in [-10.0_f32, 0.0, 10.0] {
        for pitch_deg in [0.0_f32, 8.0, 16.0] {
            look(&mut app, id, yaw_deg, pitch_deg);
            let eye = eye(&app, e);
            let (yaw, pitch) = (yaw_deg.to_radians(), pitch_deg.to_radians());
            // Distance along the ray to the plane z = face_z: the -Z component of the look
            // direction is cos(yaw)*cos(pitch).
            let t = (eye.z - face_z) / (yaw.cos() * pitch.cos());
            let expected = Vec3::new(
                eye.x - t * yaw.sin() * pitch.cos(),
                eye.y + t * pitch.sin(),
                face_z,
            );

            let a = aim_of(&app, e);
            let hit = a.point_m.unwrap_or_else(|| {
                panic!("yaw {yaw_deg}, pitch {pitch_deg}: nothing hit, expected {expected:?}")
            });
            assert!(
                (hit - expected).length() < 0.03,
                "yaw {yaw_deg}, pitch {pitch_deg}: hit {hit:?} instead of {expected:?}"
            );
            assert!(
                a.anchorable,
                "yaw {yaw_deg}, pitch {pitch_deg}: the block is `anchorable: true` in maps.ron"
            );
            seen.push(hit);
        }
    }

    for (i, a) in seen.iter().enumerate() {
        for b in &seen[i + 1..] {
            assert!(
                (*a - *b).length() > 0.5,
                "two directions gave the same point {a:?} — that is a snap, not free aiming"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// 4. The acceptance criterion: EVERY tagged surface is reachable
// ---------------------------------------------------------------------------------------

#[test]
fn f002_every_tagged_surface_in_the_map_is_reachable_by_free_aiming() {
    // `F-002`'s own acceptance: "every tagged surface is reachable by free aiming, even
    // where no anchor point was placed". So: not one example — **all of them**, taken out of
    // the world and not out of a list in this file.
    //
    // Aimed at from 5 m straight above the roof centre, looking down. That is the one
    // direction that is free for every block in this map (a roof has nothing on top of it),
    // and it makes the expected hit point exactly the centre of the top face.
    //
    // ⚠️ **That last sentence is the reason this test is pinned to the graybox** and not run
    // against `current`, however much it would like to be a check on the shipped district.
    // The premise "a roof has nothing on top of it" is a property of the graybox, not of a
    // map: in ashgate 28 of 228 tagged blocks are row houses whose ridge cap — a narrower,
    // `anchorable: false` box — sits exactly on the centre of the top face, so the shot from
    // straight above lands on the cap and reports `anchorable: false` (measured 2026-08-12,
    // `docs/FINDINGS.md` FIND-059). That is a statement about the aiming *method* of this
    // test, not about the ray. Making it map-agnostic needs a free direction per block, and
    // that is a new claim — so it is written down instead of invented here.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(0.0, 4.0, 0.0));
    let eye_height_m = data(&app).game.player.eye_height_m;

    let mut roofs: Vec<(String, Vec3)> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(&Name, &Transform, &Body), With<AnchorSurface>>();
        q.iter(world)
            .map(|(name, t, body)| {
                (
                    name.to_string(),
                    t.translation + Vec3::Y * body.half_size_m.y,
                )
            })
            .collect()
    };
    roofs.sort_by(|a, b| a.0.cmp(&b.0)); // a stable order — a test is a measurement, not a lottery

    assert!(
        roofs.len() > 20,
        "only {} tagged surfaces in the map — is the city built at all?",
        roofs.len()
    );

    let mut unreachable: Vec<String> = Vec::new();
    for (name, roof) in &roofs {
        // Feet 5 m above the roof centre, so the eye is 5 m above it.
        warp(&mut app, id, *roof + Vec3::Y * (5.0 - eye_height_m));
        look(&mut app, id, 0.0, -90.0);

        let a = aim_of(&app, e);
        match a.point_m {
            Some(hit) if a.anchorable && (hit - *roof).length() < 0.05 => {}
            other => unreachable.push(format!(
                "{name}: roof centre {roof:?} -> {other:?} (anchorable: {})",
                a.anchorable
            )),
        }
    }

    assert!(
        unreachable.is_empty(),
        "{} of {} tagged surfaces are not reachable by free aiming: {unreachable:#?}",
        unreachable.len(),
        roofs.len()
    );
}

// ---------------------------------------------------------------------------------------
// 5. The range comes out of the file
// ---------------------------------------------------------------------------------------

#[test]
fn f002_beyond_the_hook_range_from_the_file_there_is_no_hit() {
    // Straight down onto the ground slab, whose top edge lies exactly at y = 0
    // (`maps.ron: blocks[0]`). The **eye** height above it IS the ray length, so the boundary
    // is a number you can read off: one metre inside the range there is a hit, one metre
    // outside there is none.
    //
    // Red the day somebody writes 112 (the old conversion out of 400 studs, Q-002) or an
    // invented 100 into the code instead of taking `vector.hook_range_m` out of the file.
    //
    // 8 m to the side of the origin: the local player stands there, and his capsule blocks
    // the line of sight like any other body would.
    let mut app = app();
    let d = data(&app);
    let range_m = d.game.vector.hook_range_m;
    let feet_for_eye = |eye_m: f32| Vec3::new(8.0, eye_m - d.game.player.eye_height_m, 8.0);
    let (e, id) = test_player(&mut app, feet_for_eye(range_m + 1.0));

    look(&mut app, id, 0.0, -90.0);
    let far = aim_of(&app, e);
    let height = eye(&app, e).y;
    assert!(
        far.point_m.is_none(),
        "with the eye {height:.3} m above the ground the ray found {:?} — the range is \
         {range_m} m (game.ron: vector.hook_range_m)",
        far.point_m
    );

    warp(&mut app, id, feet_for_eye(range_m - 1.0));
    look(&mut app, id, 0.0, -90.0);
    let near = aim_of(&app, e);
    let hit = near
        .point_m
        .unwrap_or_else(|| panic!("at an eye height of {} m the ground is within the {range_m} m range", range_m - 1.0));
    assert!(
        hit.y.abs() < 0.05,
        "the ray landed at y = {} instead of on the ground slab at y = 0",
        hit.y
    );
    assert!(
        !near.anchorable,
        "the ground is `anchorable: false` in maps.ron — otherwise you hook into the pavement"
    );
}

// ---------------------------------------------------------------------------------------
// 6. The ray does not hit the player it belongs to
// ---------------------------------------------------------------------------------------

#[test]
fn f002_the_ray_ignores_the_players_own_capsule() {
    // The eye sits at 1.6 m INSIDE a capsule that spans 0 .. 1.8 m with radius 0.35
    // (`player::spawn_player`). Without the exclusion, `cast_ray(.., solid: true, ..)`
    // returns the origin itself (`avian3d-0.7.0/src/spatial_query/system_param.rs:111-120`)
    // — every player, every tick, at zero distance.
    //
    // And the exclusion works only because collider and body sit on the SAME entity: the
    // filter is tested against `proxy.collider` (`system_param.rs:190`,
    // `query_filter.rs:97`), not against the body.
    let mut app = app();
    let d = data(&app);
    let (e, id) = test_player(&mut app, Vec3::new(-12.0, 0.0, 0.0));
    ticks(&mut app, 60);

    look(&mut app, id, 0.0, 0.0);

    let eye = eye(&app, e);
    let a = aim_of(&app, e);
    let hit = a.point_m.expect("the 8 m cube stands 16 m in front of him");
    let distance = (hit - eye).length();
    assert!(
        distance > d.game.player.height_m,
        "the aim point lies {distance:.4} m from the eye — that is the player's own capsule \
         (radius {} m, height {} m), not the world",
        d.game.player.radius_m,
        d.game.player.height_m
    );
    assert!(
        (distance - 16.0).abs() < 0.05,
        "distance {distance:.4} m instead of the 16 m to the block at z = -16 (maps.ron)"
    );
    assert!(a.anchorable, "and that block is tagged");
}

// ---------------------------------------------------------------------------------------
// 7. Nothing in range is a state, not a stale value
// ---------------------------------------------------------------------------------------

#[test]
fn f002_with_nothing_in_range_the_aim_point_is_empty_and_not_the_last_one() {
    // A crosshair that keeps the last hit when you look at the sky offers the hook a target
    // that is no longer there. `AimPoint` is recomputed every tick; "nothing" is a value.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(-12.0, 0.0, 0.0));
    ticks(&mut app, 60);

    look(&mut app, id, 0.0, 0.0);
    assert!(aim_of(&app, e).point_m.is_some(), "first he is looking at the block");

    look(&mut app, id, 0.0, 89.0); // the pitch limit out of game.ron — nothing is up there
    let a = aim_of(&app, e);
    assert_eq!(
        a,
        AimPoint::default(),
        "looking at the sky the aim point has to be empty, and empty in every field"
    );
}

// ---------------------------------------------------------------------------------------
// 8. The tag and the bit that is derived from it do not drift apart
// ---------------------------------------------------------------------------------------

#[test]
fn f002_the_anchor_tag_and_the_body_mask_say_the_same_thing_about_every_block() {
    // `vector::aim` decides on `BodyMask::ANCHORABLE`, the gizmo in `debug` and `F-003` speak
    // of the marker `AnchorSurface`. Both come out of the same `anchorable:` in `maps.ron`
    // through `world::index::mask_from` — but they are written in two places
    // (`world::map::BlockPlan::spawn`), and if they ever drift the crosshair says one thing
    // and the cyan outline another.
    //
    // **The one test here that is not pinned**, and deliberately: it names no coordinate and
    // no fixture, so it says something about *every* map — including the one that ships. It
    // is what still measures ashgate when the eight fixture tests around it measure the
    // graybox.
    let mut app = app_on_current_map();
    let world = app.world_mut();
    let mut q = world.query::<(&Name, &Body, Has<AnchorSurface>)>();
    let disagreeing: Vec<String> = q
        .iter(world)
        .filter(|(_, body, tagged)| body.mask.contains(BodyMask::ANCHORABLE) != *tagged)
        .map(|(name, body, tagged)| format!("{name}: tag {tagged}, mask {:?}", body.mask))
        .collect();
    assert!(
        disagreeing.is_empty(),
        "{} block(s) whose tag and mask contradict each other: {disagreeing:?}",
        disagreeing.len()
    );

    // The tag-vs-mask agreement above is this test's whole subject and it is unchanged.
    //
    // ⚠️ **What used to stand here — "the map must carry both kinds" — was removed on
    // 2026-08-13, and it is the user's decision, not a weakened assertion.** He wrote:
    // *"es ist extrem wichtig dass man wirklich überall sein seil festmachen kann. also überall!
    // ohne ausnahmen!"* and, minutes later, *"es soll später auch stark vereinzelt dinge geben die
    // man nicht anchorn kann. aber sehr wenig! also kann der check drin bleiben"*. So the
    // **played** map is 100 % anchorable on purpose — measured `2067 of 2067` — and asserting the
    // opposite here would pin the game to a property he asked to have removed.
    //
    // **The criterion stays falsifiable, one file over:** `tests/data.rs::t005_the_graybox_carries_anchorable_and_untagged_surfaces`
    // now names the **graybox** explicitly and requires untagged geometry there (22 blocks), and
    // this file's own eight fixture tests are pinned to that map (`app_on("graybox")`, `FIND-061`).
    // The untagged path is therefore still proven; it is proven on the rig instead of on the city.
    let mut all = world.query::<&Body>();
    let anchorable = all
        .iter(world)
        .filter(|b| b.mask.contains(BodyMask::ANCHORABLE))
        .count();
    let total = all.iter(world).count();
    assert!(
        total > 0 && anchorable == total,
        "{anchorable} of {total} bodies anchorable on the shipped map — the user asked for every \
         surface to be hookable without exception (docs/NEXT.md §1D item 10). A rare deliberate \
         exception is allowed by his follow-up, but it belongs in the map with a comment saying \
         why, and this guard is what makes such an exception a visible decision instead of a leak."
    );
}

// ---------------------------------------------------------------------------------------
// 9. B-001 — the aim names the body, and that is the only field the hook can use
// ---------------------------------------------------------------------------------------

#[test]
fn f002_the_aim_names_the_body_it_hit() {
    // ★ `B-001`. This is the test whose absence let the bug live for a whole round: six tests
    // above measure `point_m` and `anchorable` to the centimetre, and none of them ever looked
    // at `body`. `vector::hook::anchor_target` needs exactly that field —
    //
    //     if !aim.anchorable { return None; }
    //     Some((aim.point_m?, aim.body?))
    //
    // — so `body: None` turns every shot in the running game into `ReleaseReason::NoAnchor`,
    // silently, with an exit code of 0.
    //
    // What is measured is not "some id came back" but **which** one: every tagged surface in
    // the map is aimed at from straight above, and the id in `AimPoint` is compared against
    // the `BodyId` that very entity carries. A constant, a counter or an off-by-one all go
    // red here; `is_some()` alone would not.
    //
    // Pinned to the graybox for two reasons: it shoots from straight above (same premise as
    // test 4, see there), and its last block measures the untagged wall at `z = -33.5`, which
    // exists only in that map.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(0.0, 4.0, 0.0));
    let eye_height_m = data(&app).game.player.eye_height_m;

    // Name, roof centre and id — out of the world, not out of a list in this file.
    let mut targets: Vec<(String, Vec3, Option<BodyId>)> = {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<(&Name, &Transform, &Body, Option<&BodyId>), With<AnchorSurface>>();
        q.iter(world)
            .map(|(name, t, body, id)| {
                (name.to_string(), t.translation + Vec3::Y * body.half_size_m.y, id.copied())
            })
            .collect()
    };
    targets.sort_by(|a, b| a.0.cmp(&b.0)); // a stable order — a test is a measurement
    assert!(
        targets.len() > 20,
        "only {} tagged surfaces in the map — is the city built at all?",
        targets.len()
    );

    // First the world's side of it: without an id on the carrier there is nothing the ray
    // could report, and the failure below would say "aim is broken" about the wrong file.
    let nameless: Vec<&String> =
        targets.iter().filter(|(_, _, id)| id.is_none()).map(|(n, _, _)| n).collect();
    assert!(
        nameless.is_empty(),
        "{} of {} tagged blocks carry no `BodyId` — `world::index::maintain_index` (`T-036a`) \
         hands none out, so `AimPoint.body` cannot be anything but `None`. First: {:?}",
        nameless.len(),
        targets.len(),
        &nameless[..nameless.len().min(3)]
    );

    // The ids are distinct — otherwise "the id matches" would also be satisfied by a constant.
    let mut distinct: Vec<BodyId> = targets.iter().filter_map(|(_, _, id)| *id).collect();
    let handed_out = distinct.len();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), handed_out, "two blocks share one `BodyId`");

    let mut wrong: Vec<String> = Vec::new();
    for (name, roof, expected) in &targets {
        // Feet 5 m above the roof centre, looking straight down: the one direction that is
        // free for every block in this map, and the expected hit is the centre of the top
        // face.
        warp(&mut app, id, *roof + Vec3::Y * (5.0 - eye_height_m));
        look(&mut app, id, 0.0, -90.0);

        let a = aim_of(&app, e);
        let point_ok = a.point_m.is_some_and(|hit| (hit - *roof).length() < 0.05);
        if a.body != *expected || !a.anchorable || !point_ok {
            wrong.push(format!(
                "{name}: roof centre {roof:?} -> body {:?} (expected {expected:?}), \
                 point {:?}, anchorable {}",
                a.body, a.point_m, a.anchorable
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} shots did not name the body they hit — that is `B-001`, and it is what \
         `vector::hook::anchor_target` fails on: {wrong:#?}",
        wrong.len(),
        targets.len()
    );

    // And the converse, so that the field is a statement and not a shortcut through
    // `anchorable`: an **untagged** body has to be named too, with `anchorable: false` beside
    // it. "There is something there, but you cannot hook it" is a state and not a missing hit
    // (`F-023`) — and a `body` that is only filled in when the hit is anchorable would pass
    // everything above and still be wrong here.
    //
    // The target is the untagged wall `maps.ron` keeps for exactly this: centre (-30, 5, -34),
    // near face z = -33.5. It is the same shot as test 1.
    warp(&mut app, id, Vec3::new(-30.0, 0.0, -20.0));
    look(&mut app, id, 0.0, 0.0);
    let wall = aim_of(&app, e);
    let hit = wall.point_m.expect("the untagged wall stands 13.5 m in front of him");
    assert!((hit.z + 33.5).abs() < 0.05, "that is not the wall at z = -33.5 but {:?}", hit);
    assert!(!wall.anchorable, "the wall is `anchorable: false` in maps.ron");
    let named = wall
        .body
        .expect("an untagged body is a body — it has to be named, or `F-023` cannot be seen");
    assert!(
        !distinct.contains(&named),
        "the untagged wall reports body {named:?}, which is one of the tagged blocks"
    );
}

// ---------------------------------------------------------------------------------------
// 10. F-023 — the hemisphere split: Q serves the left set, E the right
// ---------------------------------------------------------------------------------------

/// Posts a look direction **and** buttons and runs one step. Same channel as [`look`] — the
/// inbox — because nobody writes an `Intent` straight onto a player, not even a test.
fn look_and_press(
    app: &mut App,
    id: PlayerId,
    yaw_deg: f32,
    pitch_deg: f32,
    buttons: Buttons,
    spread_deg: f32,
) {
    let tick = app.world().resource::<Tick>().0;
    app.world_mut().resource_mut::<Inbox>().push(
        id,
        Intent {
            yaw: yaw_deg.to_radians(),
            pitch: pitch_deg.to_radians(),
            buttons,
            // The wheel setting travels **in the intent**, absolutely, the same way it will
            // arrive over the wire (`src/shared/intent.rs`). Spelled out in every call instead
            // of defaulted: `Intent::default()` is `0.0`, which the clamp turns into
            // `aim_spread_min_deg` — a perfectly good angle that is not the one that ships.
            aim_spread_deg: spread_deg,
            tick,
            ..default()
        },
        tick,
    );
    app.update();
}

/// Where the arm's shot is actually going — the target frozen into `HookState::Flying` at the
/// moment the trigger was pulled. **Not** an aim point: this is the number the rope flies to.
fn fired_target(app: &App, e: Entity, side: Side) -> Vec3 {
    let hook = *app.world().get::<Hook>(e).expect("every player carries both arms");
    match hook.arm(side).state {
        HookState::Flying { target_m, .. } => target_m,
        other => panic!("the {side:?} arm is {other:?} — no shot left"),
    }
}

#[test]
fn f002_q_and_e_can_target_two_different_points() {
    // The user, 2026-08-12: „und es muss mehr rechts und links spreaden!!" — and the backlog
    // had already specified it: `F-023` splits the candidate set relative to the camera
    // forward axis into a LEFT and a RIGHT hemisphere, "Q bedient ausschliesslich die linke
    // Menge, E ausschliesslich die rechte" (`docs/backlog/gameplay.ron`).
    //
    // Until this test existed `vector::hook` handed **one** `AimPoint` to both arms
    // (`docs/FINDINGS.md` FIND-039), so both ropes flew at the same world point and the two
    // HUD markers described one place. That is the state this measures away.
    //
    // The fixtures are the graybox's three explicit blocks, and the numbers below are
    // computed from `maps.ron`, not read off a run: standing in the clear circle at the
    // origin (`layout.clear_radius_m: 24`, so nothing procedural stands in the way), looking
    // along -Z, the centre ray ends on the small sand-brown cube (0, 2, -12) at z = -10, the
    // left ray on the brick-red cube (-12, 4, -20) and the right one on the stone-gray tower
    // (10, 5.75, -28). All three are `anchorable: true`.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(0.0, 0.0, 0.0));
    ticks(&mut app, 60); // land and settle — the eye has to be a stable number

    let both = Buttons(Buttons::HOOK_LEFT.0 | Buttons::HOOK_RIGHT.0);
    let spread_deg = data(&app).game.vector.aim_spread_deg;
    // The wheel is turned to the shipped notch a few ticks BEFORE the trigger, because the fan
    // is rate-limited (`aim_spread_slew_deg_s`) and 60 ticks of `Intent::default()` left it at
    // the wheel's floor. Firing on the same tick as the wheel move measures the ramp.
    for _ in 0..20 {
        look_and_press(&mut app, id, 0.0, 0.0, Buttons::NONE, spread_deg);
    }
    look_and_press(&mut app, id, 0.0, 0.0, both, spread_deg);

    let left = fired_target(&app, e, Side::Left);
    let right = fired_target(&app, e, Side::Right);
    let apart = (left - right).length();
    println!("f002 two targets at 10 m: {apart:.2} m apart — {left:?} / {right:?}");
    // ⚠️ **This number came down from 15 m on 2026-08-18 and that is the feature, not a
    // relaxation.** The claim of this test is FIND-039's — two points and never one — and it
    // is unchanged. The 15 m was the old fixture's byproduct: a constant 28° half-angle threw
    // the two rays past the small cube 10 m ahead and onto two different blocks 20-28 m away,
    // which is exactly the „zu weit auseinander" the user reported after playing. At 10 m the
    // near field is now governed by the city (`aim_sep_full_reach_m`) and the two hooks
    // straddle 3.4 m of the roof you are actually looking at.
    assert!(
        apart > 3.0,
        "the two ropes flew {apart:.2} m apart — left {left:?}, right {right:?}. At 0.00 m \
         they are one point, which is what `vector::hook` handed both arms before F-023"
    );

    // The hemispheres are not swappable: `Q` (`Side::Left`) is the LEFT one, and left of a
    // player looking along -Z is -X (`docs/conventions.md`, the axis contract).
    assert!(
        left.x < right.x,
        "Q fired at x = {}, E at x = {} — the two hemispheres are swapped",
        left.x,
        right.x
    );

    // §1A requirement 9 — *„und dann muss das seil auch dahin!!"*. What `hud::arm_aim` draws
    // and what the rope flew at is **the same number in the same tick**, not two numbers that
    // agree to within a tolerance: `vector::hook` fires at `ArmAim` and re-casts nothing.
    // Exact equality on purpose — a marker that is 1 ULP off a target is a marker computed a
    // second time somewhere (`docs/FINDINGS.md` FIND-047).
    let arms = *app.world().get::<ArmAim>(e).expect("every player that aims carries an ArmAim");
    assert_eq!(arms.target_of(Side::Left), Some(left), "the Q marker is not where Q flew");
    assert_eq!(arms.target_of(Side::Right), Some(right), "the E marker is not where E flew");
    assert!(
        arms.side(Side::Left).anchorable && arms.side(Side::Right).anchorable,
        "a shot left although the arm's own ray reports a surface that does not hold"
    );

    // And the centre ray is untouched: the crosshair still comes off `AimPoint`, and it still
    // points at the block straight ahead.
    let centre = aim_of(&app, e).point_m.expect("the small cube stands 10 m ahead");
    assert!(
        (centre.z + 10.0).abs() < 0.05 && centre.x.abs() < 0.05,
        "the centre ray moved to {centre:?} — it is the crosshair's source and must not"
    );
}

#[test]
fn f002_a_side_ray_that_finds_nothing_falls_back_to_the_centre_ray() {
    // **The line between a feature and a regression.** The spread is worth nothing if it
    // costs hit rate on every target narrower than the spread itself: aiming at a lone tower
    // has to keep working exactly as well as it did when both arms shared one point.
    //
    // The fixture is built out of the range, not out of the map, so that it does not depend on
    // what stands 40 m away: the small sand-brown cube (0, 2, -12) has its near face 10.00 m
    // ahead, and a side ray sitting `half` off the look direction needs 10 / cos(half) to reach
    // the same face. `hook_range_m` is then set BETWEEN those two numbers, so the centre ray
    // hits and both side rays reach nothing at all. It is read off the resolved fan and not
    // hard-coded to the 11 m that fitted the old 28° half-angle: since the near field became
    // governed (`aim_sep_full_reach_m`) the fan at 10 m is 9.6° and 11 m would let both side
    // rays hit. The number is moved in `GameData` and not in the file, the same way
    // `tests/vector_hooks.rs` moves the flight speed.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(0.0, 0.0, 0.0));
    ticks(&mut app, 60);
    let spread_deg = data(&app).game.vector.aim_spread_deg;
    for _ in 0..20 {
        look_and_press(&mut app, id, 0.0, 0.0, Buttons::NONE, spread_deg);
    }
    let half = app
        .world()
        .get::<AimSpread>(e)
        .and_then(|s| s.half_rad)
        .expect("a player that has aimed for twenty ticks carries a resolved fan");
    let needs_m = 10.0 / half.cos();
    let range_m = 0.5 * (10.0 + needs_m);
    println!(
        "f002 fallback fixture: fan {:.2}°, centre needs 10.00 m, a side ray needs {needs_m:.3} m, \
         range set to {range_m:.3} m",
        half.to_degrees()
    );
    assert!(range_m > 10.0 && range_m < needs_m, "the fixture has no gap to sit in");
    app.world_mut().resource_mut::<GameData>().game.vector.hook_range_m = range_m;

    let both = Buttons(Buttons::HOOK_LEFT.0 | Buttons::HOOK_RIGHT.0);
    look_and_press(&mut app, id, 0.0, 0.0, both, spread_deg);

    let centre = aim_of(&app, e).point_m.expect("the cube stands 10 m ahead, inside 11 m");
    assert!((centre.z + 10.0).abs() < 0.05, "the centre ray landed at {centre:?}");
    for side in Side::ALL {
        assert!(
            (fired_target(&app, e, side) - centre).length() < 1e-4,
            "the {side:?} arm fired at {:?} instead of falling back to the centre {centre:?} \
             — its own ray reaches nothing inside {} m",
            fired_target(&app, e, side),
            data(&app).game.vector.hook_range_m
        );
    }
}

/// Every context in one place, so the table below and the invariants below it cannot drift
/// apart. `ctx(state, speed, Some(d))` is one player, one tick, one thing under the crosshair.
fn ctx(
    wheel_deg: f32,
    v: &defeated_by_titan::data::VectorTuning,
    state: MovementState,
    speed_m_s: f32,
    distance_m: Option<f32>,
) -> SpreadContext {
    let wheel = Intent { aim_spread_deg: wheel_deg, ..default() };
    SpreadContext {
        wheel_rad: wheel.aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg),
        state,
        horizontal_speed_m_s: speed_m_s,
        distance_m,
    }
}

#[test]
fn f023_the_side_ray_sits_at_half_the_wheel_at_every_pitch() {
    // **The half of the old acceptance test that survives 2026-08-18 unchanged, and it has to
    // survive:** it is the only guard on the property that the spread is a SCREEN spread,
    // yawed around the camera's up axis and not around world Y. A yaw around world Y passes at
    // pitch 0 and collapses to nothing at ±90°, which is the failure nobody sees.
    //
    // What is NOT here any more is the acceptance number `apart >= 45 m at 100 m`. That number
    // said the two hooks must be a whole city block apart at range, and it is exactly what the
    // user called too wide („der spread für seile ist zu weit auseinander", 2026-08-18). It is
    // renegotiated in `f023_the_spread_is_a_separation_in_metres_at_every_range`, in the open,
    // against the district's own rulers — not quietly relaxed here.
    let app = app();
    let v = data(&app).game.vector.clone();

    let shipped = Intent { aim_spread_deg: v.aim_spread_deg, ..default() };
    let wheel_rad = shipped.aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg);
    assert!(
        (wheel_rad.to_degrees() - v.aim_spread_deg).abs() < 1e-4,
        "the file's own starting value does not survive its own window"
    );
    // ★ **And this is where the unit of `aim_spread_deg` is asserted, once.** FIND-086 left it
    // open — the file said half-angle (±28° = 56° of fan), `docs/NEXT.md` §1B said full angle
    // (±14°) — and 2026-08-18 decided it for the brief, because the game has now been played:
    // „der spread für seile ist zu weit auseinander". The wheel is the angle BETWEEN the rays.
    // Make `wheel_half_rad` the identity again and this test is the one that goes red.
    let spread_rad = wheel_half_rad(wheel_rad);

    for pitch_deg in [-89.0_f32, -60.0, -30.0, 0.0, 30.0, 60.0, 89.0] {
        for yaw_deg in [-170.0_f32, -45.0, 0.0, 90.0] {
            let intent = Intent {
                yaw: yaw_deg.to_radians(),
                pitch: pitch_deg.to_radians(),
                ..shipped
            };
            let look = intent.look_dir();
            let [left, right] = side_dirs(&intent, spread_rad);
            // Left is left: −X for a player looking along −Z (`docs/conventions.md`).
            let side_of = |d: Vec3| (d - look).dot(Vec3::new(yaw_deg.to_radians().cos(), 0.0, -yaw_deg.to_radians().sin()));
            assert!(
                side_of(left) < 0.0 && side_of(right) > 0.0,
                "yaw {yaw_deg} pitch {pitch_deg}: the hemispheres are swapped"
            );
            let between_deg = left.dot(right).clamp(-1.0, 1.0).acos().to_degrees();
            assert!(
                (between_deg - v.aim_spread_deg).abs() < 0.02,
                "yaw {yaw_deg} pitch {pitch_deg}: the two rays are {between_deg:.3}° apart — \
                 the wheel says {}° and that IS the angle between them",
                v.aim_spread_deg
            );
            for dir in [left, right] {
                assert!(
                    (dir.length() - 1.0).abs() < 1e-5,
                    "yaw {yaw_deg} pitch {pitch_deg}: {dir:?} is not a direction"
                );
                let off_deg = look.dot(dir).clamp(-1.0, 1.0).acos().to_degrees();
                assert!(
                    (off_deg - v.aim_spread_deg / 2.0).abs() < 0.01,
                    "yaw {yaw_deg} pitch {pitch_deg}: the side ray sits {off_deg:.3}° off the \
                     look direction instead of {:.3}° — `aim_spread_deg` is the angle BETWEEN \
                     the two rays, so one ray sits at half of it (FIND-096)",
                    v.aim_spread_deg / 2.0
                );
            }
        }
    }

    // What an `Intent` nobody has wheeled yet means for THIS system. `0.0` is not "no spread"
    // — at 0 both arms fire along one ray again, the state `F-023` exists to end (FIND-039) —
    // so the narrowest the wheel ever gets is the file's floor, and never zero.
    let narrow = Intent::default().aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg);
    assert!(
        (narrow.to_degrees() - v.aim_spread_min_deg).abs() < 1e-4,
        "an intent nobody has wheeled aims at {}° — 0° is one ray, not two",
        narrow.to_degrees()
    );
}

#[test]
fn f023_the_spread_is_a_separation_in_metres_at_every_range() {
    // ★ **THE DELIVERABLE, and it is answered at 10 m and 25 m or it is not answered.** The
    // user, 2026-08-18: „der spread für seile ist zu weit auseinander und sollte mehr dynamisch
    // sein!" — and the ranges he plays at are the near ones: Ashgate's houses are 6.5..11.5 m
    // tall (`maps.ron` ashgate.layout), its streets 6 m wide, so a roof-to-roof hook across a
    // street is **6..20 m** and a hook down the block is 20..45 m. A model that only narrows
    // past 40 m is a measured no-op for the first hook of every flight.
    //
    // BEFORE is what the game shipped until 2026-08-18: `aim_spread_deg` read as a HALF-angle,
    // so a constant 2 · d · sin(28°) that grows without bound — 9.4 m apart at 10 m (wider than
    // an Ashgate house is tall) and 187.8 m at 200 m (four block pitches).
    // AFTER is the model: the wheel is the angle BETWEEN the two rays, the target is a number
    // of METRES that the state and the speed decide, and that budget is only fully available
    // once you are looking `aim_sep_full_reach_m` away — nearer than that it scales with how
    // much city is actually between you and the point.
    let app = app();
    let v = data(&app).game.vector.clone();

    const RANGES: [f32; 9] = [5.0, 10.0, 15.0, 20.0, 25.0, 35.0, 50.0, 100.0, 200.0];
    const NEAR_10: usize = 1;
    const NEAR_25: usize = 4;
    const FAR: [usize; 3] = [6, 7, 8];

    let row = |label: &str, cells: &[f32; 9]| {
        let mut line = format!("{label:<32}");
        for c in cells {
            line.push_str(&format!("{c:>9.2}"));
        }
        println!("{line}");
    };
    let mut head = format!("{:<32}", "context");
    for d in RANGES {
        head.push_str(&format!("{:>9}", format!("{d:.0} m")));
    }
    println!(
        "\nF-023 metric separation of the two landing points, wheel {:.0}° between the rays\n{head}",
        v.aim_spread_deg
    );

    let before = RANGES.map(|d| separation_m(v.aim_spread_deg.to_radians(), d));
    row("BEFORE  28° as a HALF-angle", &before);

    let rows: [(&str, MovementState, f32); 5] = [
        ("AFTER   grounded, standing", MovementState::Grounded, 0.0),
        ("AFTER   airborne, stepped off", MovementState::Airborne, 0.0),
        ("AFTER   airborne, 30 m/s", MovementState::Airborne, 30.0),
        ("AFTER   tethered, swing 19 m/s", MovementState::Tethered, 19.0),
        ("AFTER   tethered, boost 50 m/s", MovementState::Tethered, 50.0),
    ];
    let mut table = [[0.0_f32; 9]; 5];
    for (i, (name, state, speed)) in rows.iter().enumerate() {
        table[i] = RANGES.map(|d| {
            let half = effective_spread_rad(&v, ctx(v.aim_spread_deg, &v, *state, *speed, Some(d)));
            separation_m(half, d)
        });
        row(name, &table[i]);
        for (j, d) in RANGES.iter().enumerate() {
            assert!(
                table[i][j] <= before[j] + 1e-3,
                "{name} at {d} m: {:.2} m apart against the old {:.2} m — the model may never \
                 be WIDER than the game the user already called too wide",
                table[i][j],
                before[j]
            );
        }
    }
    let grounded = table[0];
    let stepped_off = table[1];

    // ★ **THE NEAR FIELD — the two columns the complaint is about.** The band is Ashgate's own
    // rulers and not taste: at 10 m you are looking at the roof across a 6 m street and the two
    // hooks have to land on THAT roof, so a fan wider than a house frontage (12 m) is two
    // different buildings; at 25 m you are looking down the block and half a block face (36 m)
    // is the most that is still one route. 3..5 m and 8..12 m are those two statements in
    // metres, and the old game gave 9.4 m and 23.5 m — a whole house and two thirds of a block.
    for (label, cells) in [("grounded", grounded), ("airborne, stepped off", stepped_off)] {
        assert!(
            (3.0..=5.0).contains(&cells[NEAR_10]),
            "{label} at 10 m: the two ropes land {:.2} m apart — the near field has to sit in \
             3..5 m (one Ashgate house is 6.5..11.5 m tall, its street 6 m wide). The old game \
             gave {:.2} m and the user called it too wide.",
            cells[NEAR_10],
            before[NEAR_10]
        );
        assert!(
            (8.0..=12.0).contains(&cells[NEAR_25]),
            "{label} at 25 m: the two ropes land {:.2} m apart — the near field has to sit in \
             8..12 m (one house frontage is {:.0} m, one block face {:.0} m). The old game gave \
             {:.2} m.",
            cells[NEAR_25],
            v.aim_sep_floor_m,
            v.aim_sep_stand_m,
            before[NEAR_25]
        );
        for idx in [NEAR_10, NEAR_25] {
            assert!(
                cells[idx] <= before[idx] * 0.45,
                "{label} at {} m: {:.2} m against the old {:.2} m is only {:.0} % narrower — \
                 the round that made the fan dynamic was a measured NO-OP here, and that is the \
                 half of „zu weit auseinander\" this test exists for",
                RANGES[idx],
                cells[idx],
                before[idx],
                100.0 * (1.0 - cells[idx] / before[idx])
            );
        }
    }
    // And the invariant the near field is derived FROM, checkable instead of argued: while what
    // you look at is inside your own block face (`aim_sep_stand_m` = 36 m), the two hooks stay
    // inside one house frontage (`aim_sep_floor_m` = 12 m). That is exactly what fixes
    // `aim_sep_full_reach_m` — at d = 36 m the ramp reaches 36 · 36/108 = 12 m and hands over.
    for (j, d) in RANGES.iter().enumerate() {
        if *d <= v.aim_sep_stand_m {
            assert!(
                grounded[j] <= v.aim_sep_floor_m + 1e-3,
                "standing and looking {d} m ahead — inside one block face ({:.0} m) — the two \
                 ropes are {:.2} m apart, wider than the {:.0} m house frontage they are \
                 supposed to straddle",
                v.aim_sep_stand_m,
                grounded[j],
                v.aim_sep_floor_m
            );
        }
    }

    // The far field, unchanged by this round and verified by two adversaries before it: beyond
    // the handover the two ropes land at most ONE BLOCK FACE apart and never less than ONE
    // HOUSE FRONTAGE, instead of 46.9 / 93.9 / 187.8 m.
    for j in FAR {
        assert!(
            grounded[j] <= v.aim_sep_stand_m + 0.05,
            "standing at {} m the two ropes are {:.2} m apart — the block face is {:.1} m",
            RANGES[j],
            grounded[j],
            v.aim_sep_stand_m
        );
        assert!(
            grounded[j] >= v.aim_sep_floor_m,
            "standing at {} m the two ropes are {:.2} m apart — under one house frontage \
             ({:.1} m) both arms are on the same facade, which is FIND-039",
            RANGES[j],
            grounded[j],
            v.aim_sep_floor_m
        );
    }
    assert!(
        grounded[7] < before[7] * 0.5,
        "at 100 m the model gives {:.1} m against the old {:.1} m — that is not narrower \
         enough to be the answer to „zu weit auseinander\"",
        grounded[7],
        before[7]
    );
}

#[test]
fn f023_the_effective_spread_never_exceeds_the_wheel() {
    // **The one-line invariant that makes „too wide" impossible to regress into.** The user's
    // own word decides the reading of the wheel — „wie weit auseinander es gehen DARF"
    // (2026-08-12) — so it is a CEILING in every state, at every distance, at every notch.
    // And the floor is a floor: at 0° both arms fire along one ray again (FIND-039).
    let app = app();
    let v = data(&app).game.vector.clone();
    let floor_rad = v.aim_spread_floor_deg.to_radians();

    let mut wheel_deg = v.aim_spread_min_deg;
    while wheel_deg <= v.aim_spread_max_deg + 1e-4 {
        for state in [
            MovementState::Grounded,
            MovementState::Airborne,
            MovementState::Tethered,
            MovementState::OnWall,
            MovementState::Downed,
        ] {
            for speed in [0.0_f32, 6.0, 19.0, 43.0, 75.0] {
                for d in [None, Some(0.5_f32), Some(3.0), Some(10.0), Some(50.0), Some(500.0)] {
                    let c = ctx(wheel_deg, &v, state, speed, d);
                    let half = effective_spread_rad(&v, c);
                    let ceiling = wheel_half_rad(c.wheel_rad);
                    assert!(
                        half.is_finite(),
                        "{state:?} at {speed} m/s, {d:?} m away, wheel {wheel_deg}°: resolved \
                         {half} — a non-finite angle is a NaN transform two systems later"
                    );
                    assert!(
                        half <= ceiling + 1e-6,
                        "{state:?} at {speed} m/s, {d:?} m away, wheel {wheel_deg}°: resolved \
                         {:.3}° — WIDER than the {:.3}° the player allowed. The wheel is the \
                         angle BETWEEN the rays, so the ceiling on one of them is half of it.",
                        half.to_degrees(),
                        ceiling.to_degrees()
                    );
                    assert!(
                        half >= floor_rad.min(ceiling) - 1e-6,
                        "{state:?} at {speed} m/s, {d:?} m away, wheel {wheel_deg}°: resolved \
                         {:.3}°, under the floor of {:.1}° — at 0° the two arms share one ray \
                         again (FIND-039)",
                        half.to_degrees(),
                        v.aim_spread_floor_deg
                    );
                }
            }
        }
        wheel_deg += v.aim_spread_step_deg;
    }
}

#[test]
fn f023_aiming_further_never_pulls_the_two_hooks_closer_together() {
    // The monotonicity discipline, taken from the losing design that proved its own exponent's
    // range instead of tuning it: `separation(d)` must be non-decreasing in `d`. A model that
    // narrows faster than the distance grows would make the two hooks CONVERGE as you look
    // further away, which reads as the second hook breaking.
    let app = app();
    let v = data(&app).game.vector.clone();
    for state in [MovementState::Grounded, MovementState::Airborne, MovementState::Tethered] {
        for speed in [0.0_f32, 19.0, 43.0] {
            let mut prev = 0.0_f32;
            let mut d = 2.0_f32;
            while d <= 500.0 {
                let half = effective_spread_rad(&v, ctx(v.aim_spread_deg, &v, state, speed, Some(d)));
                let sep = separation_m(half, d);
                assert!(
                    sep >= prev - 1e-3,
                    "{state:?} at {speed} m/s: aiming out to {d:.1} m brought the two hooks \
                     {sep:.2} m apart, CLOSER than the {prev:.2} m one step nearer"
                );
                prev = sep;
                d *= 1.05;
            }
        }
    }
}

#[test]
fn f023_what_you_are_doing_changes_the_spread_at_one_distance() {
    // The second half of his sentence — „und sollte mehr dynamisch sein!". The test of dynamic
    // is not that the number moves with range: it is that the SAME crosshair on the SAME wall
    // gives a different answer depending on what the player is doing.
    let app = app();
    let v = data(&app).game.vector.clone();
    let at = |state, speed| {
        effective_spread_rad(&v, ctx(v.aim_spread_deg, &v, state, speed, Some(100.0))).to_degrees()
    };
    let standing = at(MovementState::Grounded, 0.0);
    let searching = at(MovementState::Airborne, 0.0);
    let swinging = at(MovementState::Tethered, 19.0);
    let boosting = at(MovementState::Tethered, 50.0);
    let blind = effective_spread_rad(&v, ctx(v.aim_spread_deg, &v, MovementState::Grounded, 0.0, None))
        .to_degrees();
    println!(
        "f023 at 100 m: standing {standing:.2}° · searching {searching:.2}° · swinging \
         {swinging:.2}° · boosting {boosting:.2}° · nothing under the crosshair {blind:.2}°"
    );
    assert!(
        searching > standing + 0.5,
        "airborne and untethered you are SEARCHING and may cross a street ({searching:.2}°), \
         standing you are picking a route across one block face ({standing:.2}°)"
    );
    assert!(
        standing > swinging + 1.0,
        "the fan has to close while you are actually swinging: standing {standing:.2}° against \
         {swinging:.2}° on the rope at 19 m/s"
    );
    assert!(
        swinging > boosting,
        "faster is tighter: {swinging:.2}° at 19 m/s against {boosting:.2}° at 50 m/s"
    );
    // Nothing under the crosshair is the ABSENCE of a distance, and then the wheel is the
    // whole answer — half of it off each side, which is the widest the player has allowed.
    // It is no longer *twice* the standing answer, and that is the halving of FIND-096 showing
    // up: 14° against 9.6°, where the old game gave 28° against 27.9°.
    let half_wheel = wheel_half_rad(
        Intent { aim_spread_deg: v.aim_spread_deg, ..default() }
            .aim_spread_rad(v.aim_spread_min_deg, v.aim_spread_max_deg),
    )
    .to_degrees();
    assert!(
        (blind - half_wheel).abs() < 1e-3,
        "with nothing under the crosshair the world has said nothing, so the wheel is the \
         angle: {blind:.2}° instead of {half_wheel:.2}°"
    );
    assert!(
        blind > searching && searching > standing,
        "sky {blind:.2}° · searching {searching:.2}° · standing {standing:.2}° — a crosshair \
         that has found a wall must never open the fan wider than one that has found nothing"
    );

    // Horizontal speed and not total speed: a straight drop must NOT pin the fan shut at the
    // moment a falling player wants the widest sweep.
    let falling = effective_spread_rad(
        &v,
        SpreadContext { horizontal_speed_m_s: 0.0, ..ctx(v.aim_spread_deg, &v, MovementState::Airborne, 0.0, Some(100.0)) },
    );
    assert!(
        (falling.to_degrees() - searching).abs() < 1e-4,
        "a straight fall at terminal speed resolved {:.2}° instead of the searching {searching:.2}° \
         — the speed term is reading the vertical component",
        falling.to_degrees()
    );
}

#[test]
fn f023_a_centre_ray_that_finds_nothing_holds_the_last_distance() {
    // **A miss is the ABSENCE of evidence about distance, not evidence of a far one.** Flying
    // across a roofline the crosshair leaves the roof for the sky several times a second; a
    // model that reset to "far" there would strobe the fan and both HUD markers with it. It
    // also keeps the near-field fan over an edge, which is the one thing the old wide fan was
    // genuinely good at.
    let app = app();
    let v = data(&app).game.vector.clone();
    let dt = 1.0 / 60.0;
    let settle = v.aim_spread_settle_s;

    // Nothing has ever been seen: still nothing.
    assert_eq!(settle_distance_m(None, None, settle, dt, v.min_rope_m, v.hook_range_m), None);
    // The first hit snaps instead of sweeping in from a seed nobody chose.
    let first = settle_distance_m(None, Some(12.0), settle, dt, v.min_rope_m, v.hook_range_m);
    assert_eq!(first, Some(12.0), "the first hit has nothing to average against");
    // The sky holds it.
    assert_eq!(
        settle_distance_m(first, None, settle, dt, v.min_rope_m, v.hook_range_m),
        first,
        "a tick that saw sky moved the aim distance — a miss is not a measurement"
    );
    // And a real depth discontinuity is a slide, not a snap: one tick of 12 m -> 300 m may
    // cover only a fraction of the way, and it arrives inside ~3 tau.
    let one = settle_distance_m(first, Some(300.0), settle, dt, v.min_rope_m, v.hook_range_m)
        .expect("a hit is a distance");
    assert!(
        one > 12.0 && one < 30.0,
        "one tick took the aim distance 12 m -> {one:.1} m; the filter is not filtering"
    );
    let mut d = first;
    for _ in 0..(3.0 * settle / dt) as usize {
        d = settle_distance_m(d, Some(300.0), settle, dt, v.min_rope_m, v.hook_range_m);
    }
    assert!(
        d.expect("still a distance") > 250.0,
        "after 3 tau the aim distance is {:?} — the filter never arrives",
        d
    );
}

#[test]
fn f023_the_spread_keys_are_ordered() {
    // The guards that a comment cannot hold. Each of these is a way to make the whole model
    // silently collapse back to the constant angle, and none of them fails loudly on its own.
    let app = app();
    let v = data(&app).game.vector.clone();
    assert!(
        v.aim_spread_floor_deg > 0.0 && v.aim_spread_floor_deg < v.aim_spread_min_deg,
        "the dynamic floor {}° has to sit strictly between 0 (one shared ray, FIND-039) and \
         the wheel's own floor {}° — otherwise the model can never narrow past the wheel",
        v.aim_spread_floor_deg,
        v.aim_spread_min_deg
    );
    assert!(
        v.aim_sep_floor_m > 0.0
            && v.aim_sep_floor_m <= v.aim_sep_tether_m
            && v.aim_sep_tether_m <= v.aim_sep_stand_m
            && v.aim_sep_stand_m <= v.aim_sep_search_m,
        "the four metre targets are out of order: floor {} · tether {} · stand {} · search {}",
        v.aim_sep_floor_m,
        v.aim_sep_tether_m,
        v.aim_sep_stand_m,
        v.aim_sep_search_m
    );
    // The near-field governor. Above the widest metre target, or the ramp is not a ramp and
    // the whole near field collapses back onto the wheel's ceiling — which is the no-op this
    // key exists to end (FIND-096). And the constant angle it fixes for the near field has to
    // stay inside the wheel's own window at the neutral notch, or the ceiling is what the
    // player feels again.
    assert!(
        v.aim_sep_full_reach_m > v.aim_sep_search_m,
        "aim_sep_full_reach_m = {} is not beyond the widest metre target ({}) — the budget \
         would be fully available before the ramp has done anything",
        v.aim_sep_full_reach_m,
        v.aim_sep_search_m
    );
    let near_deg = (v.aim_sep_search_m / (2.0 * v.aim_sep_full_reach_m)).asin().to_degrees();
    assert!(
        near_deg > v.aim_spread_floor_deg && near_deg < v.aim_sep_neutral_deg / 2.0,
        "the near field resolves to a constant {near_deg:.2}° per side, outside the window \
         between the dynamic floor ({}°) and half the neutral notch ({}°) — below the floor \
         the two arms collapse to one ray (FIND-039), above half the notch the wheel's \
         ceiling is what the player feels and the governor is dead",
        v.aim_spread_floor_deg,
        v.aim_sep_neutral_deg / 2.0
    );
    assert!(
        v.aim_sep_calm_speed_m_s > 0.0 && v.aim_sep_calm_speed_m_s < v.aim_sep_fast_speed_m_s,
        "calm {} m/s has to be under fast {} m/s, or the speed term divides by zero",
        v.aim_sep_calm_speed_m_s,
        v.aim_sep_fast_speed_m_s
    );
    assert!(
        v.aim_sep_neutral_deg >= v.aim_spread_min_deg
            && v.aim_sep_neutral_deg <= v.aim_spread_max_deg,
        "the neutral notch {}° is outside the wheel's own window {}..{}°",
        v.aim_sep_neutral_deg,
        v.aim_spread_min_deg,
        v.aim_spread_max_deg
    );
    assert!(
        v.aim_spread_settle_s > 0.0 && v.aim_spread_slew_deg_s > 0.0,
        "a settle time or a slew rate of zero freezes the fan at whatever it first resolved to"
    );
}

#[test]
fn f023_a_straight_fall_does_not_pin_the_fan_shut() {
    // **The plumbing of the speed term, through the real system**, and the reason it reads the
    // HORIZONTAL speed and not `Velocity::speed_m_s`: a straight 43 m/s drop would otherwise
    // pin the fan to its floor at the exact moment a falling player wants the widest sweep.
    //
    // The fixture is built out of `GameData` and not out of the map, the same way the fallback
    // test moves `hook_range_m`: standing at the origin the centre ray ends 10.00 m ahead, and
    // at the shipped 36 m target the wheel's ceiling binds there — so the target is moved down
    // until the model, and not the ceiling, is what decides the angle.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::ZERO);
    ticks(&mut app, 60);
    {
        let mut d = app.world_mut().resource_mut::<GameData>();
        // 40 / 2 and not 2 / 0.5: since the near-field ramp landed, a target of 2 m at 10 m
        // resolves under `aim_spread_floor_deg` and both halves of this test read the floor.
        // At 40 m of far-field budget the ramp gives 40 · 10/108 = 3.70 m at 10 m = 10.67°,
        // clear of both the 2° floor and the 14° ceiling — so what this measures is the speed
        // term and nothing else.
        d.game.vector.aim_sep_stand_m = 40.0;
        d.game.vector.aim_sep_floor_m = 2.0;
    }
    let wheel_deg = data(&app).game.vector.aim_spread_deg;

    // Terminal velocity straight down. `Velocity` is injected once per tick because
    // `SimulationSystems::Integrate` — the only writer — runs after `World`, where `aim` sits.
    let mut resolved = |app: &mut App, v: Vec3| {
        for _ in 0..20 {
            app.world_mut().entity_mut(e).insert(Velocity(v));
            look_and_press(app, id, 0.0, 0.0, Buttons::NONE, wheel_deg);
        }
        app.world()
            .get::<AimSpread>(e)
            .expect("a player that aims carries the model's memory")
            .half_rad
            .expect("the model resolved an angle")
            .to_degrees()
    };

    let falling = resolved(&mut app, Vec3::new(0.0, -43.0, 0.0));
    let running = resolved(&mut app, Vec3::new(43.0, 0.0, 0.0));
    println!("f023 falling at 43 m/s: {falling:.2}° · moving at 43 m/s: {running:.2}°");
    let reach_m = data(&app).game.vector.aim_sep_full_reach_m;
    let want = (40.0_f32 * (10.0 / reach_m) / 20.0).asin().to_degrees();
    assert!(
        (falling - want).abs() < 0.05,
        "a straight fall resolved {falling:.2}° instead of {want:.2}° — the speed term is \
         reading the vertical component, and the fan shuts on a player who is falling"
    );
    assert!(
        running < falling - 1.0,
        "moving sideways at 43 m/s resolved {running:.2}° against {falling:.2}° falling — the \
         speed term does not reach the running game"
    );
}

#[test]
fn f023_a_wheel_the_player_turns_down_binds_on_the_very_next_tick() {
    // **The slew escape.** `aim` clamps the resolved angle to the wheel inside
    // `effective_spread_rad` and then slews towards it — and the slew was never re-clamped, so
    // for as long as the ramp lasts the fan sits WIDER than the ceiling the player just dialled.
    // The user's own word makes that a contract and not a nicety: „wie weit auseinander es gehen
    // DARF" (2026-08-12). A wheel dropped from the file's ceiling to its floor left the fan at
    // 41/38/35/32/29/26/… degrees for ~11 ticks — about 0.19 s of a game that is doing exactly
    // what he asked it not to.
    let app = app();
    let v = data(&app).game.vector.clone();
    let dt = 1.0 / 60.0;
    let step_rad = v.aim_spread_slew_deg_s.to_radians() * dt;

    // The player was at the widest notch and wheels straight down to the narrowest.
    let was = wheel_half_rad(v.aim_spread_max_deg.to_radians());
    let ceiling = wheel_half_rad(v.aim_spread_min_deg.to_radians());
    let floor = v.aim_spread_floor_deg.to_radians();

    let mut half = was;
    for tick in 1..=30 {
        half = slew_spread_rad(Some(half), ceiling, step_rad, ceiling, floor);
        assert!(
            half <= ceiling + 1e-6,
            "tick {tick} after the wheel went {:.0}° -> {:.0}°: the fan is {:.2}° off centre, \
             wider than the {:.2}° the player allows — the slew escapes the ceiling",
            v.aim_spread_max_deg,
            v.aim_spread_min_deg,
            half.to_degrees(),
            ceiling.to_degrees()
        );
    }
    // …and the clamp is a clamp, not a snap: the ramp is still a ramp in the other direction.
    let opening = slew_spread_rad(Some(floor), wheel_half_rad(v.aim_spread_max_deg.to_radians()), step_rad, wheel_half_rad(v.aim_spread_max_deg.to_radians()), floor);
    assert!(
        (opening - floor - step_rad).abs() < 1e-6,
        "opening up moved {:.3}° in one tick instead of the file's {:.3}° — the re-clamp ate \
         the slew instead of bounding it",
        (opening - floor).to_degrees(),
        step_rad.to_degrees()
    );
    // And the floor still wins over a target under it, after the slew as before it.
    let under = slew_spread_rad(Some(floor), 0.0, step_rad, ceiling, floor);
    assert!(under >= floor - 1e-6, "the slew walked {:.3}° under the floor", under.to_degrees());
}

// ---------------------------------------------------------------------------------------
// 12. `B-008` — a side ray that finds SOMETHING ELSE is a miss, not a target
// ---------------------------------------------------------------------------------------
//
// `F-028`'s fallback asks *"did this side ray find anything anchorable?"* and hands the arm
// the centre ray when the answer is no. In Ashgate the answer is **never** no: the district is
// 100 % anchorable (the user: *„ueberall! ohne ausnahmen!"*) and the ground is always under
// the cone, so a side ray that has left the surface the crosshair stands on carries on and
// bites whatever it meets. Measured from 30 m over the street at (168.19, ., -50.12), looking
// straight down: the crosshair is on the pavement at 30.1 m, the fan asks for **5.85 m** of
// separation — and the two arms landed **11.50 m** and **10.77 m** away, on the two roof caps
// beside the street. From every height over that street the same two roofs win.
//
// So the question is generalised: *"did this side ray find the thing the crosshair is on?"*

#[test]
fn b008_a_side_hit_that_left_the_crosshairs_surface_is_not_a_target() {
    // The predicate on its own, against points typed out by hand (`docs/FINDINGS.md`
    // FIND-103: a test that asks the code under test the same question twice proves nothing).
    // Eye at the origin looking along -Z, the crosshair 30 m out, the fan 11.21° per side —
    // the numbers the run above measured.
    let eye_m = Vec3::ZERO;
    let half = 11.21_f32.to_radians();
    let k = data(&app()).game.vector.aim_side_coherence_k;
    let crosshair = Vec3::new(0.0, 0.0, -30.0);
    let centre = Some((crosshair, BodyId(573)));
    // The fan's own ask at 30 m: 30 * sin(11.21°).
    let asked_m = 30.0 * half.sin();
    assert!((asked_m - 5.83).abs() < 0.05, "the fixture drifted: {asked_m}");

    // 1. On the same body, however far along it — a facade seen at a grazing angle is still
    //    the facade the crosshair is on, and that is the whole point of the spread.
    assert!(
        side_hit_is_coherent(centre, (Vec3::new(0.0, 0.0, -120.0), BodyId(573)), eye_m, half, k),
        "a hit on the very body the crosshair stands on is the thing that was aimed at"
    );
    // 2. Another body, but inside the separation the fan asked for.
    assert!(
        side_hit_is_coherent(
            centre,
            (Vec3::new(asked_m * 1.02, 0.0, -30.0), BodyId(999)),
            eye_m,
            half,
            k
        ),
        "a hit one fan-width off the crosshair is what F-023 asked for"
    );
    // 3. ★ The bug: another body, roughly twice as far off as the fan asked for. That is the
    //    11.50 m the roof cap sat at against the 5.85 m the model wanted.
    assert!(
        !side_hit_is_coherent(
            centre,
            (Vec3::new(asked_m * 1.97, 0.0, -18.0), BodyId(2154)),
            eye_m,
            half,
            k
        ),
        "a hit {:.2} m off a crosshair that asked for {asked_m:.2} m is a different part of \
         town — that is B-008",
        asked_m * 1.97
    );
    // 4. The crosshair on nothing at all. `F-023`'s promise is that the rope and the marker
    //    are one number; a centre ray that found nothing has no number to be, and a side ray
    //    that flies 429 m to a tower the player never saw is exactly what FIND-116 measured.
    assert!(
        !side_hit_is_coherent(None, (Vec3::new(30.0, 100.0, -420.0), BodyId(77)), eye_m, half, k),
        "with nothing under the crosshair there is nothing for a side ray to be coherent with"
    );
}

#[test]
fn b008_a_shot_aimed_straight_down_lands_on_what_the_crosshair_stands_on() {
    // **The bug, in the shipped district and nowhere else** — it exists *because* Ashgate is
    // 100 % anchorable, so it cannot be built in the graybox, where the ground carries no
    // anchor bit at all and the fallback therefore still fires.
    //
    // Gravity off, so the shot is measured from the height it is fired at and not from
    // wherever the player has fallen to by the time the fan has settled.
    use avian3d::prelude::Gravity;
    let mut app = app_on_current_map();
    app.insert_resource(Gravity(Vec3::ZERO));
    let (e, id) = test_player(&mut app, Vec3::new(168.19, 30.0, -50.12));
    let spread_deg = data(&app).game.vector.aim_spread_deg;
    let k = data(&app).game.vector.aim_side_coherence_k;
    // Straight down over the middle of the 4.30 m street of `scripts/f003-ashgate.txt` ACT 5.
    for _ in 0..30 {
        look_and_press(&mut app, id, 0.0, -90.0, Buttons::NONE, spread_deg);
    }

    let eye_m = eye(&app, e);
    let half = app
        .world()
        .get::<AimSpread>(e)
        .and_then(|s| s.half_rad)
        .expect("a player who has aimed for thirty ticks carries a resolved fan");
    let centre = aim_of(&app, e);
    let crosshair = centre.point_m.expect("the street stands 30 m under him");
    assert!(centre.anchorable, "the whole district is anchorable — the pavement included");
    let d_m = (crosshair - eye_m).length();
    let asked_m = d_m * half.sin();
    let arms = *app.world().get::<ArmAim>(e).expect("every player that aims carries an ArmAim");
    println!(
        "b008: fan {:.2}°, crosshair {crosshair:?} at {d_m:.2} m, the fan asks for {asked_m:.2} m",
        half.to_degrees()
    );
    for side in Side::ALL {
        let arm = arms.side(side);
        let point = arm.point_m.expect("every ray in this district ends on something");
        let off_m = (point - crosshair).length();
        println!("  {side:?} -> {point:?} body {:?}, {off_m:.2} m off the crosshair", arm.body);
        // ★ Red before the fix: `Left -> 11.50 m off`, `Right -> 10.77 m off`, both on roof
        // caps (bodies 2154 and 2156) while the crosshair stood on the street (body 573).
        assert!(
            arm.body == centre.body || off_m <= k * asked_m,
            "the {side:?} arm aims at {point:?} on body {:?} — {off_m:.2} m off a crosshair \
             that stands on body {:?} and asked for {asked_m:.2} m. That is B-008: the arm \
             flies at something the player never pointed at.",
            arm.body,
            centre.body
        );
    }
}
