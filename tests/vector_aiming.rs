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
use defeated_by_titan::shared::{
    AimPoint, AnchorSurface, ArmAim, Body, BodyId, BodyMask, Buttons, Cli, Hook, HookState,
    IdCounter, Intent, PlayerId, Side, Tick, WarpPlayer,
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
    // 🔴 **The tolerance is DERIVED, and it had to be: it was `0.05` and gravity broke it.**
    // The ray is cast inside the tick by `vector::aim`; `eye()` here reads the transform after
    // `app.update()` has returned. The player is warped to `y = 15.0` **in the air**, so between
    // the cast and the read he is still falling — the two are one to two simulation steps apart
    // by construction, and no amount of settling can fix that without moving him off the stand
    // the test needs.
    //
    // A vertical error `δ` at the eye becomes `δ / tan(pitch)` on the roof plane — here a
    // **4.9x** lever. At `gravity_m_s2` −20 that put the error at 0.04 m against a 0.05 m
    // literal, i.e. marginal and nobody noticed; at −32 it is 0.066 m and the literal broke.
    // **Measured on two binaries one control run apart: identical to five decimals with and
    // without the rope joint**, so this is the gravity change and not the rope (`FIND-196`).
    //
    // So the bound comes out of the file instead: two steps of free fall at the *current*
    // gravity, projected along the *current* pitch. It moves when the tuning moves, and it is
    // still two orders of magnitude below the 3.5 m that separates this roof from the wall in
    // front of it — the thing the test is actually about.
    let dt = 1.0_f32 / data(&app).game.simulation_hz as f32;
    let g = data(&app).game.gravity_m_s2.abs();
    let slack = 2.0 * g * dt * dt / pitch.tan().abs();
    assert!(
        (roof - expected).length() < slack,
        "roof hit {roof:?} instead of {expected:?} (eye {eye:?}, pitch {pitch_deg} deg, \
         slack {slack:.4} m = two ticks of free fall at g = {g})"
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
// 10. F-023 retired, 2026-08-23 — **both ropes fly at the crosshair**
//
// > *„dann das auseinander mit q und e kann weg. einfach da wo ich hinschau (also fadenkreuz)
// > geht das seil hin."* — the user, after playing the reference beside this game.
//
// This section used to hold the fan: the hemisphere split, the metre model of
// `effective_spread_rad`, the wheel's ceiling and `B-008`'s coherence guard. Thirteen tests
// went with the sixteen keys. `docs/QUESTIONS.md` Q-048 · `git show 83f09da` for the model.
// ---------------------------------------------------------------------------------------

/// Posts a look direction **and** buttons and runs one step. Same channel as [`look`] — the
/// inbox — because nobody writes an `Intent` straight onto a player, not even a test.
fn look_and_press(
    app: &mut App,
    id: PlayerId,
    yaw_deg: f32,
    pitch_deg: f32,
    buttons: Buttons,
) {
    let tick = app.world().resource::<Tick>().0;
    app.world_mut().resource_mut::<Inbox>().push(
        id,
        Intent {
            yaw: yaw_deg.to_radians(),
            pitch: pitch_deg.to_radians(),
            buttons,
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
fn f023_both_arms_fire_at_the_point_under_the_crosshair() {
    // The user, 2026-08-23, after playing the reference beside this game: „dann das
    // auseinander mit q und e kann weg. einfach da wo ich hinschau (also fadenkreuz) geht das
    // seil hin." **One aim point, two ropes.** It overrides his own 2026-08-12 sentence that
    // built the fan (`docs/QUESTIONS.md` Q-048), and the standing rule says it may
    // (`CLAUDE.md`: his instruction beats his own earlier number).
    //
    // Same fixture as the fan test it replaces: standing in the clear circle at the origin
    // (`layout.clear_radius_m: 24`), looking along -Z, the centre ray ends on the small
    // sand-brown cube (0, 2, -12) at z = -10. Under the fan the left arm flew to the brick-red
    // cube (-12, 4, -20) and the right one to the stone-gray tower (10, 5.75, -28); now both
    // fly at the cube the crosshair is on.
    let mut app = app();
    let (e, id) = test_player(&mut app, Vec3::new(0.0, 0.0, 0.0));
    ticks(&mut app, 60); // land and settle — the eye has to be a stable number

    let both = Buttons(Buttons::HOOK_LEFT.0 | Buttons::HOOK_RIGHT.0);
    for _ in 0..20 {
        look_and_press(&mut app, id, 0.0, 0.0, Buttons::NONE);
    }
    look_and_press(&mut app, id, 0.0, 0.0, both);

    let centre = aim_of(&app, e).point_m.expect("the small cube stands 10 m ahead");
    let left = fired_target(&app, e, Side::Left);
    let right = fired_target(&app, e, Side::Right);
    println!("f023 one point: centre {centre:?} · Q {left:?} · E {right:?}");
    // Exact equality and not a tolerance: `vector::hook` fires at `ArmAim` and re-casts
    // nothing, so a rope that is 1 ULP off the crosshair's hit is a second computation of the
    // same number somewhere (`docs/FINDINGS.md` FIND-047).
    assert_eq!(left, centre, "Q flew at {left:?} instead of the crosshair's point {centre:?}");
    assert_eq!(right, centre, "E flew at {right:?} instead of the crosshair's point {centre:?}");
    assert_eq!(left, right, "the two ropes went to two different points — the fan is back");

    // And `F-023`'s surviving half: what the HUD draws is what the rope flew at, one number in
    // one tick (`docs/FINDINGS.md` FIND-129).
    let arms = *app.world().get::<ArmAim>(e).expect("every player that aims carries an ArmAim");
    assert_eq!(arms.target_of(Side::Left), Some(left), "the Q marker is not where Q flew");
    assert_eq!(arms.target_of(Side::Right), Some(right), "the E marker is not where E flew");
    assert!(
        arms.side(Side::Left).anchorable && arms.side(Side::Right).anchorable,
        "a shot left although the arm's own point reports a surface that does not hold"
    );
}

#[test]
fn b008_a_shot_aimed_straight_down_lands_on_what_the_crosshair_stands_on() {
    // **`B-008`, and the whole class of bug it names, is gone by construction.** It existed
    // *because* Ashgate is 100 % anchorable: an arm's own side ray, cast `half_rad` off the
    // look direction, left the pavement the crosshair stood on and bit a roof cap beside the
    // street instead — measured 2026-08-19 at **11.50 m and 10.77 m** off the crosshair from
    // 30 m over the street at `(168.19, ., -50.12)`. `side_hit_is_coherent` was the guard.
    // Since 2026-08-23 there is no side ray to be incoherent: both arms carry the crosshair's
    // own hit. The test stays because the *claim* stays — a shot goes where the player points,
    // straight down included — and it would catch a second ray creeping back in.
    //
    // Gravity off, so the shot is measured from the height it is fired at.
    use avian3d::prelude::Gravity;
    let mut app = app_on_current_map();
    app.insert_resource(Gravity(Vec3::ZERO));
    let (e, id) = test_player(&mut app, Vec3::new(168.19, 30.0, -50.12));
    // Straight down over the middle of the 4.30 m street of `scripts/f003-ashgate.txt` ACT 5.
    for _ in 0..30 {
        look_and_press(&mut app, id, 0.0, -90.0, Buttons::NONE);
    }

    let eye_m = eye(&app, e);
    let centre = aim_of(&app, e);
    let crosshair = centre.point_m.expect("the street stands 30 m under him");
    assert!(centre.anchorable, "the whole district is anchorable — the pavement included");
    let arms = *app.world().get::<ArmAim>(e).expect("every player that aims carries an ArmAim");
    println!("b008: crosshair {crosshair:?} at {:.2} m", (crosshair - eye_m).length());
    for side in Side::ALL {
        let arm = arms.side(side);
        let point = arm.point_m.expect("every ray in this district ends on something");
        println!("  {side:?} -> {point:?} body {:?}", arm.body);
        assert_eq!(
            point, crosshair,
            "the {side:?} arm aims at {point:?} on body {:?} while the crosshair stands on \
             {crosshair:?} on body {:?} — a second ray has come back",
            arm.body, centre.body
        );
    }
}
