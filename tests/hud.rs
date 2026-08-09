//! The guard over the HUD — `F-170` (base layout) and `F-171` (dynamic crosshair).
//!
//! **What this file exists to make impossible has a name.** `docs/PLAN-GAME.md` §8 calls it
//! *"the bar that is a picture of a bar"*: every element of F-170's list present, and three of
//! them showing a hard-coded number because their producer does not exist yet. Such a HUD
//! photographs perfectly. It survives a round. It is discovered two rounds later, by somebody
//! wiring up the real value and finding that nothing changes.
//!
//! So the tests here do not ask "is there a bar". They ask "does the bar move when the number
//! moves, and does it disappear when the number does not exist".
//!
//! **Why the tests run schedules by hand instead of `app.update()`** — the same reason
//! `tests/render.rs` gives: `app.update()` goes through `First`, where `Time<Virtual>` is
//! filled from the wall clock, and depending on the machine's mood a fixed step happens.
//! `vector` would then write `Gas` and `AimPoint` underneath a test that just set them, and
//! the result would measure the machine. `Update` alone is deterministic; `PostUpdate` alone
//! is what lays the UI out.

use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::{ComputedNode, UiGlobalTransform};

use defeated_by_titan::data::GameData;
use defeated_by_titan::debug::screenshot::{OFFSCREEN_HEIGHT, OFFSCREEN_WIDTH};
use defeated_by_titan::hud::blade_pips::{BladeCluster, BladePip, SharpnessBar};
use defeated_by_titan::hud::crosshair::{
    self, CrosshairPart, CrosshairShape, CrosshairState,
};
use defeated_by_titan::hud::gas_bar::GasBar;
use defeated_by_titan::hud::health_bar::{HealthBar, HealthTrack};
use defeated_by_titan::hud::objective::ObjectiveLine;
use defeated_by_titan::hud::{HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use defeated_by_titan::shared::{Blades, Cli, Gas, Health, LocalPlayer};

/// Builds the **real** app, headless — not a second, similar one.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // Two passes: `Commands` only take effect at the end of their run, the player comes into
    // being in `Startup`, and `render::attach_camera` only hangs the camera on him afterwards.
    app.update();
    app.update();
    app
}

fn local_player(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

/// Gives the camera a render target of a known size — **exactly what `--offscreen` does**.
///
/// Without this the UI lays out into a zero-sized viewport: a headless app has no window, so
/// `Camera::physical_target_size()` has nothing to report, `ComputedUiRenderTargetInfo` stays
/// `UVec2::ZERO`, and every rectangle in the keep-out test would come out 0 × 0 and pass. That
/// is the same trap `tests/render.rs::the_camera_is_the_default_ui_camera` guards from the
/// other side, and the reason this helper uses the picture's own resolution constants and not
/// two numbers of its own.
fn attach_screen(app: &mut App) {
    let handle = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        images.add(Image::new_target_texture(
            OFFSCREEN_WIDTH,
            OFFSCREEN_HEIGHT,
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        ))
    };
    let camera = {
        let mut q = app.world_mut().query_filtered::<Entity, With<Camera3d>>();
        q.iter(app.world()).next().expect("there must be a 3D camera")
    };
    app.world_mut()
        .entity_mut(camera)
        .insert(RenderTarget::Image(handle.into()));
    // Camera info is computed in `PostUpdate`, the UI target info reads it there, and the
    // layout reads that — three stages of one schedule, so twice is enough and once is not.
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);
}

/// The screen rectangle of one node, in physical pixels: `(min_x, min_y, max_x, max_y)`.
fn rect(node: &ComputedNode, at: &UiGlobalTransform) -> (f32, f32, f32, f32) {
    let half = node.size() * 0.5;
    let c = at.translation;
    (c.x - half.x, c.y - half.y, c.x + half.x, c.y + half.y)
}

fn percent(node: &Node) -> f32 {
    match node.width {
        Val::Percent(p) => p,
        other => panic!(
            "the bar's width is {other:?} and not a percentage — the reading has to be \
             `Val::Percent`, or nothing outside this file can check it"
        ),
    }
}

// ---------------------------------------------------------------------------------------
// F-170 — the bars follow their number, or they are not there
// ---------------------------------------------------------------------------------------

#[test]
fn f170_the_gas_bar_follows_the_gas_and_not_the_clock() {
    // ★ **The one with teeth.** Goes red the moment somebody wires the bar to a constant, to
    // a timer, or to `Gas::max` — all three of which produce a bar that looks right in every
    // screenshot ever taken of a full tank.
    let mut app = app();
    let player = local_player(&mut app);

    for (current, expected) in [(30.0_f32, 30.0_f32), (0.0, 0.0), (100.0, 100.0), (55.5, 55.5)] {
        app.world_mut()
            .entity_mut(player)
            .insert(Gas { current, max: 100.0, unlimited: false });
        app.world_mut().run_schedule(Update);

        let mut q = app.world_mut().query_filtered::<&Node, With<GasBar>>();
        let node = q
            .iter(app.world())
            .next()
            .expect("no node with `GasBar` — `hud::gas_bar::spawn_gas_bar` is not registered");
        let got = percent(node);
        assert!(
            (got - expected).abs() < 0.1,
            "gas {current} of 100 must give a bar of {expected} %, but the node is {got} % wide"
        );
    }
}

#[test]
fn f170_the_health_bar_follows_the_health_and_hides_while_nobody_produces_it() {
    // ★ The same criterion for the second bar — **and its other half**, which is the one that
    // matters today: job R3-A has not landed, nothing in the running game puts a `Health` on
    // the player, and a bar sitting at 100 % would be a picture of a bar.
    //
    // The arithmetic can still be checked in full, because `Health` lives in `shared`: the
    // test inserts the **real** component, not a stand-in. When R3-A lands, only the
    // "and it hides" half becomes obsolete.
    let mut app = app();
    let player = local_player(&mut app);

    app.world_mut().entity_mut(player).remove::<Health>();
    app.world_mut().run_schedule(Update);
    assert_eq!(
        health_track_display(&mut app),
        Display::None,
        "there is no `Health` on the local player, and the health bar is on screen anyway — \
         that is a bar showing a number nobody produced (docs/PLAN-GAME.md §8, F-170)"
    );

    for (current, expected) in [(30.0_f32, 30.0_f32), (0.0, 0.0), (100.0, 100.0)] {
        app.world_mut().entity_mut(player).insert(Health { current, max: 100.0 });
        app.world_mut().run_schedule(Update);
        assert_eq!(
            health_track_display(&mut app),
            Display::Flex,
            "with a `Health` on the player the bar has to be visible"
        );
        let mut q = app.world_mut().query_filtered::<&Node, With<HealthBar>>();
        let node = q.iter(app.world()).next().expect("no node with `HealthBar`");
        let got = percent(node);
        assert!(
            (got - expected).abs() < 0.1,
            "health {current} of 100 must give a bar of {expected} %, but it is {got} % wide"
        );
    }
}

fn health_track_display(app: &mut App) -> Display {
    let mut q = app.world_mut().query_filtered::<&Node, With<HealthTrack>>();
    q.iter(app.world())
        .next()
        .expect("no node with `HealthTrack` — the health bar is not registered")
        .display
}

#[test]
fn f170_the_blade_pips_follow_the_pairs_and_the_sharpness() {
    // The third readout. `Blades` has a producer — `player::spawn_player` inserts it — but its
    // *wear* does not, so in a real run the value never moves. That is exactly the situation
    // in which a hard-coded pip row is invisible, so it gets the same treatment.
    let mut app = app();
    let player = local_player(&mut app);
    let pairs = app.world().resource::<GameData>().gear.blades.start_pairs;
    assert!(pairs >= 2, "gear.ron start_pairs = {pairs} — this test needs at least two pips");

    for left in [pairs, 1, 0] {
        app.world_mut()
            .entity_mut(player)
            .insert(Blades { pairs_left: left, sharpness: 0.5 });
        app.world_mut().run_schedule(Update);

        let lit = lit_pips(&mut app);
        assert_eq!(
            lit,
            usize::from(left),
            "{left} pairs left must light {left} pips, not {lit}"
        );

        let mut q = app.world_mut().query_filtered::<&Node, With<SharpnessBar>>();
        let got = percent(q.iter(app.world()).next().expect("no `SharpnessBar` node"));
        assert!(
            (got - 50.0).abs() < 0.1,
            "sharpness 0.5 must give a bar of 50 %, but it is {got} %"
        );
    }

    app.world_mut().entity_mut(player).remove::<Blades>();
    app.world_mut().run_schedule(Update);
    let mut q = app.world_mut().query_filtered::<&Node, With<BladeCluster>>();
    assert_eq!(
        q.iter(app.world()).next().expect("no `BladeCluster` node").display,
        Display::None,
        "without a `Blades` component the pips must not be on screen"
    );
}

/// How many pips are lit — a pip is lit when it does not carry the dark plate colour.
fn lit_pips(app: &mut App) -> usize {
    let plate = defeated_by_titan::hud::PLATE;
    let mut q = app.world_mut().query_filtered::<&BackgroundColor, With<BladePip>>();
    q.iter(app.world()).filter(|c| c.0 != plate).count()
}

#[test]
fn f170_the_objective_line_stays_empty_until_a_producer_exists() {
    // `mission` is a stub, `hud → mission` is not on the allow list in `docs/architecture.md`,
    // and there is therefore nothing that produces an objective. The honest state is: the node
    // exists, hidden, empty.
    //
    // **This test is what stops the shortcut.** Write `"Kill 5 titans"` into
    // `hud::objective::update_objective` to make the screenshot look finished, and it goes red.
    let mut app = app();
    app.world_mut().run_schedule(Update);

    let mut q = app.world_mut().query_filtered::<(&Text, &Node), With<ObjectiveLine>>();
    let (text, node) = q.iter(app.world()).next().expect("no node with `ObjectiveLine`");
    assert!(
        text.0.is_empty(),
        "the objective line reads {:?} — but nothing in this build produces an objective, \
         so that string was invented by the HUD (docs/PLAN-GAME.md §8, F-170)",
        text.0
    );
    assert_eq!(
        node.display,
        Display::None,
        "an objective line with no objective has to be hidden, not empty-but-drawn"
    );
}

// ---------------------------------------------------------------------------------------
// F-170 — nothing covers the middle of the screen
// ---------------------------------------------------------------------------------------

#[test]
fn f170_nothing_covers_the_middle_of_the_screen() {
    // Every `HudElement`, in its **worst case**: health present so its bar is laid out, the
    // objective forced visible although nothing produces it, and the crosshair in `Cortex` —
    // the state with eight nodes and the widest reach.
    let mut app = app();
    attach_screen(&mut app);
    let player = local_player(&mut app);
    app.world_mut().entity_mut(player).insert(Health::full(100.0));
    app.world_mut().run_schedule(Update);

    // The objective would be hidden (no producer, see the test above), and a hidden node
    // covers nothing — which would make this test blind to exactly the element most likely to
    // be widened later. So the test shows it itself and asks the hypothetical question.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<(&mut Text, &mut Node), With<ObjectiveLine>>();
        for (mut text, mut node) in q.iter_mut(app.world_mut()) {
            text.0 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into();
            node.display = Display::Flex;
        }
    }
    set_crosshair(&mut app, CrosshairState::Cortex);
    app.world_mut()
        .run_system_once(crosshair::shape_crosshair)
        .expect("the one-shot system runs");
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);

    let (w, h) = screen(&mut app);
    assert!(
        w > 0.0 && h > 0.0,
        "the UI laid out into a {w} x {h} viewport — every rectangle below would be empty and \
         this test would pass without looking at anything. `IsDefaultUiCamera` belongs on the \
         camera (tests/render.rs::the_camera_is_the_default_ui_camera)"
    );

    let box_min_x = w * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = w * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = h * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = h * KEEP_OUT_HIGH_PCT / 100.0;

    let mut seen = 0;
    let mut q = app
        .world_mut()
        .query_filtered::<(&Name, &Node, &ComputedNode, &UiGlobalTransform), With<HudElement>>();
    for (name, node, computed, at) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = rect(computed, at);
        if max_x - min_x <= 0.0 || max_y - min_y <= 0.0 {
            continue;
        }
        seen += 1;
        let overlaps = min_x < box_max_x && max_x > box_min_x && min_y < box_max_y && max_y > box_min_y;
        assert!(
            !overlaps,
            "`{name}` covers the middle of the screen: it stands at \
             ({min_x:.1}, {min_y:.1})..({max_x:.1}, {max_y:.1}) and the central \
             20 % x 20 % box is ({box_min_x:.1}, {box_min_y:.1})..({box_max_x:.1}, \
             {box_max_y:.1}) on a {w} x {h} screen"
        );
    }
    assert!(
        seen >= 12,
        "only {seen} HUD nodes were laid out — with five elements and an eight-node crosshair \
         there have to be more, so this test just checked almost nothing"
    );
}

/// The viewport the UI was laid out into, in physical pixels.
///
/// Taken off a HUD node's own render target info and not off the camera: what matters is the
/// size the **layout** used, and those two are the same number only when the trap in
/// `tests/render.rs` is not open.
fn screen(app: &mut App) -> (f32, f32) {
    let mut q = app
        .world_mut()
        .query_filtered::<&bevy::ui::ComputedUiRenderTargetInfo, With<HudElement>>();
    let info = q.iter(app.world()).next().expect("a HUD node must have render target info");
    let size = info.physical_size();
    (size.x as f32, size.y as f32)
}

// ---------------------------------------------------------------------------------------
// F-171 — the three states
// ---------------------------------------------------------------------------------------

fn set_crosshair(app: &mut App, state: CrosshairState) {
    let mut q = app
        .world_mut()
        .query_filtered::<&mut CrosshairState, With<CrosshairPart>>();
    for mut s in q.iter_mut(app.world_mut()) {
        *s = state;
    }
}

/// `(visible node count, bounding width px, bounding height px)` — the tuple `F-171` compares.
fn crosshair_geometry(app: &mut App) -> (usize, f32, f32) {
    app.world_mut()
        .run_system_once(crosshair::shape_crosshair)
        .expect("the one-shot system runs");
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);

    let mut count = 0;
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    let mut q = app
        .world_mut()
        .query_filtered::<(&Node, &ComputedNode, &UiGlobalTransform), With<CrosshairPart>>();
    for (node, computed, at) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        let (a, b, c, d) = rect(computed, at);
        count += 1;
        min_x = min_x.min(a);
        min_y = min_y.min(b);
        max_x = max_x.max(c);
        max_y = max_y.max(d);
    }
    (count, max_x - min_x, max_y - min_y)
}

#[test]
fn f171_the_three_states_differ_in_shape_not_only_in_colour() {
    // ★ **The criterion of the row.** Its acceptance is "the states are distinguishable under
    // colour blindness", and the only way to make that falsifiable is to take the colour away:
    // every `BackgroundColor` is forced to one and the same value, and the three states still
    // have to come out as three different `(node_count, width, height)` tuples.
    //
    // Goes red when the states are three colours on one node — which is what a crosshair
    // becomes the first time somebody "simplifies" it.
    let mut app = app();
    attach_screen(&mut app);

    let mut measured = Vec::new();
    for state in [CrosshairState::Free, CrosshairState::Anchor, CrosshairState::Cortex] {
        set_crosshair(&mut app, state);
        {
            let mut q = app
                .world_mut()
                .query_filtered::<&mut BackgroundColor, With<CrosshairPart>>();
            for mut colour in q.iter_mut(app.world_mut()) {
                colour.0 = Color::WHITE;
            }
        }
        measured.push((state, crosshair_geometry(&mut app)));
    }

    // The colours really are all equal now — otherwise the assertion below would be free.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<CrosshairPart>>();
        let all: Vec<Color> = q.iter(app.world()).map(|c| c.0).collect();
        assert!(
            all.iter().all(|c| *c == Color::WHITE),
            "the test did not manage to neutralise the colour — then it proves nothing about \
             shape"
        );
    }

    for (state, (count, w, h)) in &measured {
        assert!(
            *count > 0 && *w > 0.0 && *h > 0.0,
            "{state:?} draws nothing at all: {count} nodes, {w} x {h} px"
        );
    }
    for (i, (a_state, a)) in measured.iter().enumerate() {
        for (b_state, b) in measured.iter().skip(i + 1) {
            let differs = a.0 != b.0 || (a.1 - b.1).abs() > 0.5 || (a.2 - b.2).abs() > 0.5;
            assert!(
                differs,
                "{a_state:?} and {b_state:?} are the same shape — {a:?} against {b:?}. \
                 With the colour taken away a player cannot tell them apart, and `F-171`'s \
                 acceptance is exactly that he can"
            );
        }
    }

    // The numbers this row has to report, printed so the run itself is the measurement.
    for (state, (count, w, h)) in &measured {
        println!("f171 {state:?}: {count} nodes, {w:.1} x {h:.1} px");
    }
}

#[test]
fn f171_the_state_rule_puts_the_cortex_first() {
    // A pure rule, so it is tested as one. Cortex beats anchor: a titan's nape is often over a
    // roof, both are true at once, and the crosshair has to say the lethal one.
    assert_eq!(crosshair::state_for(false, false), CrosshairState::Free);
    assert_eq!(crosshair::state_for(true, false), CrosshairState::Anchor);
    assert_eq!(crosshair::state_for(false, true), CrosshairState::Cortex);
    assert_eq!(crosshair::state_for(true, true), CrosshairState::Cortex);
}

#[test]
fn f171_the_crosshair_follows_the_aim_point() {
    // The wiring, not the shape: `sense_crosshair` reads `AimPoint::anchorable` off the local
    // player. Goes red when somebody leaves the system registered and empties its body — the
    // failure mode this repo has already had once (`docs/lessons/`, the sham switch).
    use defeated_by_titan::shared::AimPoint;
    let mut app = app();
    let player = local_player(&mut app);

    for (anchorable, expected) in
        [(false, CrosshairState::Free), (true, CrosshairState::Anchor), (false, CrosshairState::Free)]
    {
        app.world_mut().entity_mut(player).insert(AimPoint {
            point_m: Some(Vec3::new(0.0, 1.6, -10.0)),
            body: None,
            anchorable,
        });
        app.world_mut()
            .run_system_once(crosshair::sense_crosshair)
            .expect("the one-shot system runs");
        let mut q = app
            .world_mut()
            .query_filtered::<&CrosshairState, With<CrosshairPart>>();
        let got = *q.iter(app.world()).next().expect("no crosshair parts");
        assert_eq!(
            got, expected,
            "with `anchorable: {anchorable}` the crosshair has to be {expected:?}, not {got:?}"
        );
    }
}

#[test]
fn f171_the_crosshair_eye_is_the_aim_eye() {
    // `hud` may not reach into `vector` (the allow list in `docs/architecture.md` is empty), so
    // `hud::crosshair::eye` is the second spelling of `vector::aim::eye`. Two spellings of one
    // offset are how a crosshair and a hook end up pointing at different things — the file
    // that owns the first spelling says so itself. This is the test that keeps them equal.
    //
    // The test may cross the domain line the code may not: `tests/domains.rs` reads `src/`.
    for translation in [Vec3::ZERO, Vec3::new(3.0, -2.0, 7.5), Vec3::new(-100.0, 70.0, 130.0)] {
        for height in [0.0_f32, 1.6, 2.75] {
            assert_eq!(
                crosshair::eye(translation, height),
                defeated_by_titan::vector::aim::eye(translation, height),
                "the crosshair's eye and the aim ray's eye have drifted apart at \
                 {translation:?} / {height} m"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// The colours come out of the file
// ---------------------------------------------------------------------------------------

#[test]
fn f170_the_signal_colours_come_out_of_maps_ron() {
    // Nothing is changed here — the point is that a literal cannot creep back in. The three
    // signal colours were made data this session precisely so that this assertion exists:
    // `docs/conventions.md` §3 says cyan, amber and crimson appear nowhere else, and a rule
    // that is only prose is a rule that is broken in three weeks.
    //
    // The expected values are read straight out of `GameData`, **not** through `hud::signal` —
    // that would be the function under test checking itself.
    let mut app = app();
    let player = local_player(&mut app);
    app.world_mut().entity_mut(player).insert(Health::full(100.0));
    set_crosshair(&mut app, CrosshairState::Cortex);
    app.world_mut()
        .run_system_once(crosshair::paint_crosshair)
        .expect("the one-shot system runs");
    app.world_mut().run_schedule(Update);
    // `Update` re-senses the crosshair, so the amber has to be re-established afterwards.
    set_crosshair(&mut app, CrosshairState::Cortex);
    app.world_mut()
        .run_system_once(crosshair::paint_crosshair)
        .expect("the one-shot system runs");

    let expect = |name: &str, app: &App| {
        let (r, g, b) = *app
            .world()
            .resource::<GameData>()
            .maps
            .signals
            .get(name)
            .unwrap_or_else(|| panic!("maps.ron has no signal {name:?}"));
        Color::linear_rgb(r, g, b)
    };
    let cyan = expect("cyan", &app);
    let amber = expect("amber", &app);
    let crimson = expect("crimson", &app);

    assert_eq!(
        background::<GasBar>(&mut app),
        cyan,
        "the gas bar is not the cyan out of maps.ron — a literal has crept back in"
    );
    assert_eq!(
        background::<HealthBar>(&mut app),
        crimson,
        "the health bar is not the crimson out of maps.ron"
    );
    let mut q = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<CrosshairPart>>();
    let got = q.iter(app.world()).next().expect("no crosshair parts").0;
    assert_eq!(got, amber, "the cortex crosshair is not the amber out of maps.ron");

    let mut q = app.world_mut().query_filtered::<&TextColor, With<ObjectiveLine>>();
    let got = q.iter(app.world()).next().expect("no objective line").0;
    assert_eq!(got, amber, "the objective line is not the amber out of maps.ron");
}

fn background<C: Component>(app: &mut App) -> Color {
    let mut q = app.world_mut().query_filtered::<&BackgroundColor, With<C>>();
    q.iter(app.world()).next().expect("the element must exist").0
}

#[test]
fn f170_every_hud_node_carries_the_marker() {
    // The keep-out test walks `HudElement`. A node without it would be invisible to that test
    // — and the middle of the screen would be covered by the one element nobody checked.
    let mut app = app();
    app.world_mut().run_schedule(Update);

    let hud_names: Vec<String> = {
        let mut q = app.world_mut().query_filtered::<&Name, With<Node>>();
        q.iter(app.world())
            .map(|n| n.as_str().to_owned())
            .filter(|n| n.starts_with("hud_"))
            .collect()
    };
    let marked: Vec<String> = {
        let mut q = app.world_mut().query_filtered::<&Name, With<HudElement>>();
        q.iter(app.world()).map(|n| n.as_str().to_owned()).collect()
    };
    for name in &hud_names {
        assert!(
            marked.contains(name),
            "`{name}` is a HUD node without `HudElement` — \
             `f170_nothing_covers_the_middle_of_the_screen` cannot see it"
        );
    }
    assert!(hud_names.len() >= 12, "only {} HUD nodes found", hud_names.len());
}

#[test]
fn f171_the_shape_table_is_the_only_place_the_numbers_live() {
    // Cheap, and it catches the copy: if `shape_of` and `node_count` ever disagree, the test
    // above measures one thing and the picture shows another.
    for state in [CrosshairState::Free, CrosshairState::Anchor, CrosshairState::Cortex] {
        let shape: CrosshairShape = crosshair::shape_of(state);
        let expected = if shape.corner.is_some() { 8 } else { 4 };
        assert_eq!(
            crosshair::node_count(state),
            expected,
            "{state:?}: `node_count` and `shape_of` disagree"
        );
    }
}
