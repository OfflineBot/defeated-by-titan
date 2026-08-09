//! The guard over the gizmos — **the strokes without which an image is not evidence.**
//!
//! `docs/ACCEPTANCE.md` demands an image for 🟧 on which you actually **recognize**
//! something. On `docs/images/t006-world-far.png` you see blocks, but not which of them is
//! anchorable. `src/debug/gizmo.rs` draws exactly that — and this file is what keeps the rule
//! from rotting quietly:
//!
//! - **They are registered and they run.** Take the `add_systems` line out of
//!   `src/debug/mod.rs` and something falls over here, instead of the next job losing its
//!   image.
//! - **They draw only what is tagged.** A gizmo on a block without an
//!   [`AnchorSurface`](defeated_by_titan::shared::AnchorSurface) would claim something the
//!   game does not do — and `F-003` ("no hook on untagged surfaces") would no longer be
//!   checkable in the image.
//! - **They draw nothing at all while the toggle is off.**
//!
//! Whatever can be checked **without** an app — colors, edge counts, sizes, the toggle —
//! lives as a unit test in `src/debug/gizmo.rs`. Only what needs a real app lives here.

use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::gizmo::{GizmoToggle, GizmoCounts, GizmoSystems};
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::shared::{AnchorSurface, Block, IdCounter, Cli};

/// Builds the **real** app, headless — not a second, similar one.
///
/// The toggle is set explicitly instead of being read from the environment: a test that
/// flips `DBT_GIZMOS` checks the process instead of the rule, and it disturbs every other
/// test running in parallel in the same process.
fn app(gizmos_on: bool) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(GizmoToggle { on: gizmos_on });
    app
}

fn counts(app: &App) -> GizmoCounts {
    *app.world().resource::<GizmoCounts>()
}

/// A cuboid shaped like a house, so the test does not hang on some special shape. Whether it
/// is anchorable is the only thing that differs between the cases.
fn block() -> Block {
    Block { size: Vec3::new(6.0, 9.0, 6.0), color: [0.42, 0.43, 0.40] }
}

#[test]
fn the_gizmo_systems_are_registered_in_the_update_schedule() {
    // The literal-minded half: the three systems are registered. The test next door checks
    // that they also do something — together the pair falls over whether somebody removes
    // the registration or leaves it standing and empties the body.
    //
    // Checked through the SET and not through system names: without `bevy_utils/debug` every
    // system here is called, verbatim, "<Enable the debug feature to see the name>"
    // (measured, see `src/debug/gizmo.rs::GizmoSystems`) — a name test would be green while
    // knowing nothing at all.
    let mut app = app(false);
    app.update(); // without one pass the schedule is not initialized

    let schedule = app.get_schedule(Update).expect("Update-Schedule");
    let systems = schedule
        .graph()
        .systems_in_set(GizmoSystems.intern())
        .expect("the GizmoSystems set is not in the Update schedule");

    assert_eq!(
        systems.len(),
        3,
        "there should be three drawing systems in the set (anchors, reference, players) — \
         without them the next job has no image on which anything is recognizable \
         (docs/ABNAHME.md)"
    );
}

#[test]
fn the_gizmos_run_and_outline_the_anchor_surfaces_of_the_map() {
    let mut app = app(true);
    app.update(); // Startup builds the map, Update draws it

    let drawn = counts(&app).anchors;
    let present = anchor_surfaces(&mut app);
    assert!(present > 0, "the map has not a single anchor surface — the test measures nothing");
    assert_eq!(
        drawn, present,
        "{present} anchor surfaces in the world, but {drawn} outlined"
    );
}

#[test]
fn a_block_without_an_anchor_surface_gets_no_gizmo() {
    // **This is the claim the image makes.** If every block were outlined, "outlined" would
    // only mean "is a block" — and `F-003` would no longer be checkable on any screenshot.
    let mut app = app(true);
    app.update();
    let prev = counts(&app).anchors;

    app.world_mut().spawn((Name::new("probe_untagged"), block(), Transform::from_xyz(80.0, 4.5, 0.0)));
    app.update();
    assert_eq!(
        counts(&app).anchors,
        prev,
        "a block without an anchor surface was outlined — the image would claim something \
         the game does not do"
    );

    app.world_mut().spawn((
        Name::new("probe_tagged"),
        block(),
        AnchorSurface,
        Transform::from_xyz(80.0, 4.5, 20.0),
    ));
    app.update();
    assert_eq!(
        counts(&app).anchors,
        prev + 1,
        "a new anchor surface stayed invisible — then the image shows an old state"
    );
}

#[test]
fn with_the_toggle_off_nothing_is_drawn() {
    // Gizmos must not run all the time: compute time, and on an in-game image they get in
    // the way.
    let mut app = app(false);
    app.update();
    app.update();
    assert_eq!(
        counts(&app),
        GizmoCounts::default(),
        "the toggle is off and it was drawn anyway"
    );
    assert!(anchor_surfaces(&mut app) > 0, "there would have been something to draw");
}

#[test]
fn the_hull_holding_the_camera_stays_empty_a_team_mate_does_not() {
    // The camera hangs off the local player as a child and sits inside his hull. Draw it and
    // your own hull lies over the whole image as a wireframe — 0.35 m in front of a 60 degree
    // lens, one edge fills the frame.
    let mut app = app(true);
    for _ in 0..3 {
        app.update(); // player, then camera (commands are deferred), then propagation
    }
    assert_eq!(
        counts(&app).players,
        0,
        "your own hull was drawn — it covers every first-person image"
    );

    // A team mate, exactly the way one will later arrive over the network: no LocalPlayer,
    // no camera. He MUST be marked, otherwise he is invisible in a long-range shot.
    {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        spawn_player(&mut commands, &mut ids, &data, Vec3::new(60.0, 2.0, 0.0), false);
    }
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(
        counts(&app).players,
        1,
        "a team mate without a camera stayed unmarked — exactly the case the marker \
         exists for (docs/multiplayer.md rule 3)"
    );
}

/// How many entities the world currently lists as anchorable.
fn anchor_surfaces(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, With<AnchorSurface>>();
    query.iter(app.world()).count()
}
