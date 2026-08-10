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
use defeated_by_titan::hud::arm_aim::{self, ArmAimState, ArmMarker, ArmMarkerLabel};
use defeated_by_titan::hud::gas_bar::GasBar;
use defeated_by_titan::hud::health_bar::{HealthBar, HealthTrack};
use defeated_by_titan::hud::objective::ObjectiveLine;
use defeated_by_titan::hud::{HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use defeated_by_titan::mission::{KillTally, MissionPhase};
use defeated_by_titan::shared::{
    BodyId, Blades, Cli, Gas, Health, Hook, HookState, LocalPlayer, PlayerId, Side, TitanId,
};

/// Builds the **real** app, headless — not a second, similar one.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // Two passes: `Commands` only take effect at the end of their run, the player comes into
    // being in `Startup`, and `render::attach_camera` only hangs the camera on him afterwards.
    app.update();
    app.update();
    app
}

/// The same app **with a mission running** — `--mission tutorial`, the way `main.rs` builds it.
///
/// Not a hand-made mission entity: `mission::begin_mission` walks `Briefing → Deploying →
/// Active` in `Startup` and `deploy` puts the [`KillTally`] down with the `kill_target` **out of
/// `missions.ron`**. A test that spawned its own tally with a `3` in it would be checking its own
/// literal against the HUD's.
fn mission_app() -> App {
    let mut app = defeated_by_titan::app(Cli {
        headless: true,
        mission: Some("tutorial".to_string()),
        ..default()
    });
    app.update();
    app.update();
    app
}

/// `kill_target` of the tutorial **out of the file** — the number the objective counts to.
fn kill_target(app: &App) -> u32 {
    app.world()
        .resource::<GameData>()
        .missions
        .templates
        .get("tutorial")
        .expect("missions.ron must know the tutorial template")
        .kill_target
}

/// What the objective line reads, and whether it is drawn at all.
fn objective(app: &mut App) -> (String, Display) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Text, &Node), With<ObjectiveLine>>();
    let (text, node) = q
        .iter(app.world())
        .next()
        .expect("no node with `ObjectiveLine` — `hud::objective::spawn_objective` is not registered");
    (text.0.clone(), node.display)
}

/// Books one kill on the **real** counter, through the mission's own API.
fn credit(app: &mut App, player: PlayerId, titan: TitanId) {
    let mut q = app.world_mut().query::<&mut KillTally>();
    let mut tally = q
        .iter_mut(app.world_mut())
        .next()
        .expect("a mission entity with a `KillTally` — `--mission tutorial` did not deploy");
    assert!(tally.credit(player, titan), "titan {titan:?} was already credited");
}

/// Moves the mission phase the way `mission::decide` does, transition included.
fn set_phase(app: &mut App, phase: MissionPhase) {
    app.world_mut().resource_mut::<NextState<MissionPhase>>().set(phase);
    app.world_mut().run_schedule(StateTransition);
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
            .insert(Gas { current, ..Gas::full(100.0) });
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
fn f170_the_objective_line_stays_empty_without_a_mission() {
    // **This test is what stops the shortcut**, and it is the one that was here before the
    // producer landed: write `"Kill 5 titans"` into `hud::objective::update_objective` to make
    // the screenshot look finished, and it goes red.
    //
    // It has grown a second half. Hiding the line unconditionally used to pass the first half,
    // and now cannot: with `--mission tutorial` the line has to say something.
    let mut app = app();
    app.world_mut().run_schedule(Update);

    let (text, display) = objective(&mut app);
    assert!(
        text.is_empty(),
        "the objective line reads {text:?} in a game with no mission — nothing produced that \
         string, so the HUD invented it (docs/PLAN-GAME.md §8, F-170)"
    );
    assert_eq!(
        display,
        Display::None,
        "an objective line with no objective has to be hidden, not empty-but-drawn"
    );

    let mut app = mission_app();
    app.world_mut().run_schedule(Update);
    let (text, display) = objective(&mut app);
    assert!(
        !text.is_empty() && display == Display::Flex,
        "with `--mission tutorial` running the objective line reads {text:?} and is {display:?} \
         — a line that hides in every case passes the half of this test above without ever \
         showing a player anything"
    );
}

#[test]
fn f170_the_objective_counts_the_real_kills() {
    // ★ **The one with teeth for this element.** `docs/PLAN-GAME.md` §1 asks for a counter that
    // "goes from `0/3` to `1/3`", and the named failure of the row is the element that is
    // present and wired to a constant.
    //
    // Neither number in the assertion below is written here: the target comes out of
    // `missions.ron` through `GameData`, the count out of the mission's own `KillTally`. Put a
    // literal `3` in the HUD and change `kill_target` in the file, and this goes red.
    //
    // **The kills are booked on two different players on purpose.** The mission is won by the
    // squad (`mission::run::KillTally::total`), so the line counts the squad. A HUD that showed
    // `KillTally::of(local_player)` would read 2/3 next to a `WON` in the first co-op session —
    // that swap turns this test red at the second kill.
    let mut app = mission_app();
    let target = kill_target(&app);
    assert!(target >= 3, "missions.ron: tutorial kill_target = {target} — this test needs 3+");

    app.world_mut().run_schedule(Update);
    assert_eq!(
        objective(&mut app).0,
        format!("0/{target}"),
        "a mission that has just started stands at 0 kills of {target}"
    );

    for n in 1..=target {
        // player 1, 2, 1, 2 … — the total is the squad's, not one player's.
        let player = PlayerId(1 + n % 2);
        credit(&mut app, player, TitanId(100 + n));
        app.world_mut().run_schedule(Update);
        assert_eq!(
            objective(&mut app).0,
            format!("{n}/{target}"),
            "after {n} credited kill(s) the line has to read {n}/{target}"
        );
    }

    // And now the half a literal survives. Everything above is also true of
    // `format!("{}/3", …)`, because `missions.ron` happens to say 3 — so the target is moved
    // out from under the HUD. `KillTally::target` is `kill_target` and nothing else, so this is
    // the same move as editing the file, without touching a file another job owns.
    let moved = target + 4;
    {
        let mut q = app.world_mut().query::<&mut KillTally>();
        let mut tally = q.iter_mut(app.world_mut()).next().expect("the mission's KillTally");
        tally.target = moved;
    }
    app.world_mut().run_schedule(Update);
    assert_eq!(
        objective(&mut app).0,
        format!("{target}/{moved}"),
        "the target moved from {target} to {moved} and the line did not follow — that number is \
         a literal in the HUD, and `missions.ron` no longer decides what the mission counts to \
         (CLAUDE.md rule 2)"
    );
}

#[test]
fn f170_the_screen_says_what_the_mission_decided() {
    // ★ `docs/PLAN-GAME.md` §1: "the screen says **LOST**" / "it says **WON**". Not the F3
    // overlay — the screen.
    //
    // The word is `MissionPhase::label()`'s and the HUD does not get its own copy of it. Two
    // halves, and the second is the one that keeps them from drifting: the phase says `WON`, the
    // HUD is asked, and then `src/hud/objective.rs` is read to check that the word is not
    // *also* written down in there.
    let mut app = mission_app();

    for phase in [MissionPhase::Won, MissionPhase::Lost] {
        set_phase(&mut app, phase);
        app.world_mut().run_schedule(Update);
        let (text, display) = objective(&mut app);
        assert_eq!(
            text,
            phase.label(),
            "the mission decided {phase:?} and the screen says {text:?} — a player only ever \
             sees the HUD, so this is what the verdict IS (docs/PLAN-GAME.md §1)"
        );
        assert_eq!(display, Display::Flex, "the verdict has to be drawn, not just stored");
    }

    // The verdict is bigger than the counter — "large enough to be the thing you notice".
    set_phase(&mut app, MissionPhase::Won);
    app.world_mut().run_schedule(Update);
    let verdict_px = objective_font_px(&mut app);
    set_phase(&mut app, MissionPhase::Active);
    app.world_mut().run_schedule(Update);
    let count_px = objective_font_px(&mut app);
    assert!(
        verdict_px > count_px * 1.5,
        "the verdict is {verdict_px} px and the counter {count_px} px — the word the whole \
         mission was about has to be the thing you notice"
    );

    // And the HUD does not keep a second copy of the wording.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hud/objective.rs"),
    )
    .expect("src/hud/objective.rs must be readable");
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for phase in [
        MissionPhase::Briefing,
        MissionPhase::Deploying,
        MissionPhase::Active,
        MissionPhase::Won,
        MissionPhase::Lost,
    ] {
        let literal = format!("{:?}", phase.label());
        assert!(
            !code.contains(&literal),
            "src/hud/objective.rs contains the literal {literal} — the words belong to \
             `MissionPhase::label()`, and two spellings of one word drift apart the first time \
             somebody renames a phase"
        );
    }
    assert!(
        code.contains("label()"),
        "src/hud/objective.rs never calls `MissionPhase::label()` — then the words on the \
         screen came from somewhere else"
    );
}

/// The objective line's font size in logical pixels.
fn objective_font_px(app: &mut App) -> f32 {
    use bevy::text::FontSize;
    let mut q = app
        .world_mut()
        .query_filtered::<&TextFont, With<ObjectiveLine>>();
    let font = q.iter(app.world()).next().expect("no node with `ObjectiveLine`");
    match font.font_size {
        FontSize::Px(px) => px,
        other => panic!("the objective line's font size is {other:?} and not `Px` — nothing \
                         outside this file can then compare it"),
    }
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

    // The objective is hidden in this app (no mission), and a hidden node covers nothing —
    // which would make this test blind to exactly the element most likely to be widened later.
    // So the test shows it itself, in a **worse** state than the game can produce: forty
    // characters at the verdict's font size, where the longest real line is `LOST` and the
    // longest counter `99/99`. If this fits above the box, everything the element can say fits.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<(&mut Text, &mut Node, &mut TextFont), With<ObjectiveLine>>();
        for (mut text, mut node, mut font) in q.iter_mut(app.world_mut()) {
            text.0 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into();
            font.font_size = bevy::text::FontSize::Px(defeated_by_titan::hud::objective::VERDICT_PX);
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

// ---------------------------------------------------------------------------------------
// F-171 — the two arm markers, `Q` and `E`
// ---------------------------------------------------------------------------------------
//
// The user's sentence after playing on 2026-08-10: *"und es muss auch visuell immer 2 punkte
// angezeigt werden so der e und q haken hingehen würden!"* Two markers, always, one per arm.
//
// The trap this block exists to catch is the same one the whole file is built against, in its
// nastiest form for this element: **two markers that stand somewhere plausible and mean
// nothing**. A pair of dots symmetric around the crosshair photographs beautifully whether or
// not it is wired to the arms at all. So nothing below asserts that "an entity exists": every
// test here moves an arm and demands that the geometry moves with it.

const ARM_STATES: [ArmAimState; 4] = [
    ArmAimState::Free,
    ArmAimState::Ready,
    ArmAimState::Busy,
    ArmAimState::Anchored,
];

fn set_arm_state(app: &mut App, state: ArmAimState) {
    let mut q = app
        .world_mut()
        .query_filtered::<&mut ArmAimState, With<ArmMarker>>();
    for mut s in q.iter_mut(app.world_mut()) {
        *s = state;
    }
}

/// `(visible node count, bounding width px, bounding height px)` for **one** arm.
///
/// Per side, not for the pair: the whole claim of this element is that the two sides can say
/// different things, and a tuple over both of them together could not see that.
fn arm_geometry(app: &mut App, side: Side) -> (usize, f32, f32) {
    app.world_mut()
        .run_system_once(arm_aim::shape_arm_aim)
        .expect("the one-shot system runs");
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);

    let mut count = 0;
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    let mut q = app
        .world_mut()
        .query::<(&ArmMarker, &Node, &ComputedNode, &UiGlobalTransform)>();
    for (marker, node, computed, at) in q.iter(app.world()) {
        if marker.side != side || node.display == Display::None {
            continue;
        }
        let (a, b, c, d) = rect(computed, at);
        if c - a <= 0.0 || d - b <= 0.0 {
            continue;
        }
        count += 1;
        min_x = min_x.min(a);
        min_y = min_y.min(b);
        max_x = max_x.max(c);
        max_y = max_y.max(d);
    }
    (count, max_x - min_x, max_y - min_y)
}

/// Puts one hook state on one arm of the local player, leaving the other arm alone.
fn set_arm(app: &mut App, side: Side, state: HookState) {
    let player = local_player(app);
    let mut hook = app
        .world_mut()
        .entity_mut(player)
        .get::<Hook>()
        .copied()
        .expect("the player carries a `Hook` — `player::spawn_player` inserts it");
    hook.arms[side.index()].state = state;
    app.world_mut().entity_mut(player).insert(hook);
}

/// What one arm's marker currently says.
fn arm_state(app: &mut App, side: Side) -> ArmAimState {
    let mut q = app.world_mut().query::<(&ArmMarker, &ArmAimState)>();
    q.iter(app.world())
        .find(|(m, _)| m.side == side)
        .map(|(_, s)| *s)
        .expect("both arms must have a marker")
}

#[test]
fn f171_the_two_arm_markers_differ_in_shape_not_only_in_colour() {
    // ★ **The criterion.** `F-026`'s acceptance is that a player can say without thinking where
    // `Q` and `E` would take him, and `F-171`'s is that he can do it colour-blind. The only way
    // to make either falsifiable is to take the colour away: every `BackgroundColor` and every
    // `BorderColor` on the markers is forced to one and the same value, and the four states
    // still have to come out as four different `(node count, width, height)` tuples.
    //
    // Goes red the moment the four states become four colours on one dot.
    let mut app = app();
    attach_screen(&mut app);

    let mut measured = Vec::new();
    for state in ARM_STATES {
        set_arm_state(&mut app, state);
        {
            let mut q = app
                .world_mut()
                .query_filtered::<(&mut BackgroundColor, &mut BorderColor), With<ArmMarker>>();
            for (mut fill, mut border) in q.iter_mut(app.world_mut()) {
                fill.0 = Color::WHITE;
                border.set_all(Color::WHITE);
            }
        }
        measured.push((state, arm_geometry(&mut app, Side::Left)));
    }

    // The colour really is gone — otherwise every assertion below would be free.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<(&BackgroundColor, &BorderColor), With<ArmMarker>>();
        assert!(
            q.iter(app.world()).all(|(f, b)| f.0 == Color::WHITE && b.top == Color::WHITE),
            "the test did not manage to neutralise the colour — then it proves nothing about \
             shape"
        );
    }

    for (state, (count, w, h)) in &measured {
        assert!(
            *count > 0 && *w > 0.0 && *h > 0.0,
            "{state:?} draws nothing at all: {count} nodes, {w} x {h} px — and requirement 1 is \
             that BOTH markers are visible in EVERY state, because a marker that vanishes tells \
             the player nothing about why"
        );
        assert_eq!(
            *count,
            arm_aim::node_count(*state),
            "{state:?}: the shape table promises {} nodes and {count} are drawn",
            arm_aim::node_count(*state)
        );
    }
    for (i, (a_state, a)) in measured.iter().enumerate() {
        for (b_state, b) in measured.iter().skip(i + 1) {
            let differs = a.0 != b.0 || (a.1 - b.1).abs() > 0.5 || (a.2 - b.2).abs() > 0.5;
            assert!(
                differs,
                "{a_state:?} and {b_state:?} are the same shape — {a:?} against {b:?}. With the \
                 colour taken away a player cannot tell them apart, and both `F-171`'s and \
                 `F-026`'s acceptance is exactly that he can"
            );
        }
    }

    // The mirror: the right arm is the left arm's shape, on the other side. A right marker that
    // silently drew something else would pass every assertion above.
    for state in ARM_STATES {
        set_arm_state(&mut app, state);
        let left = arm_geometry(&mut app, Side::Left);
        let right = arm_geometry(&mut app, Side::Right);
        assert_eq!(
            left, right,
            "{state:?}: the two arms draw different shapes — {left:?} against {right:?}. The \
             side is carried by WHERE the marker stands, never by what it looks like"
        );
    }

    for (state, (count, w, h)) in &measured {
        println!("f171 arm {state:?}: {count} nodes, {w:.1} x {h:.1} px");
    }
}

#[test]
fn f171_the_two_markers_come_apart_when_the_two_arms_do() {
    // ★ **The one with teeth.** Today both hooks fly at the one `AimPoint` the camera ray
    // produced — `vector::hook::update_hooks` reads it once and gives it to both arms — so a
    // preview that put them on two different world points would be drawing a mechanic the
    // simulation does not have (`docs/backlog/gameplay.ron` F-023's hemispheres are ⬜).
    //
    // What is true is that the two arms have their own STATE. This test is the proof that the
    // markers are wired to that state per arm and not to one shared value: one arm anchors, the
    // other stays idle, and the two markers have to say two different things.
    let mut app = app();
    attach_screen(&mut app);
    // The aim answer is pinned, and it has to be: `app()` runs two `app.update()`s, a fixed step
    // can fall inside them depending on how busy the machine is, and then `vector::aim` writes a
    // real raycast into `AimPoint` underneath the test. Measured on 2026-08-10: this test passes
    // alone and fails in a full `--test hud` run without this line. What is under test here is
    // the arm STATE, so the shared aim answer is held still.
    {
        use defeated_by_titan::shared::AimPoint;
        let player = local_player(&mut app);
        app.world_mut()
            .entity_mut(player)
            .insert(AimPoint { point_m: None, body: None, anchorable: false });
    }

    set_arm(&mut app, Side::Left, HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO });
    set_arm(&mut app, Side::Right, HookState::Idle);
    app.world_mut()
        .run_system_once(arm_aim::sense_arm_aim)
        .expect("the one-shot system runs");

    assert_eq!(arm_state(&mut app, Side::Left), ArmAimState::Anchored);
    assert_ne!(
        arm_state(&mut app, Side::Left),
        arm_state(&mut app, Side::Right),
        "one arm is anchored and the other is idle, and both markers say the same thing — then \
         the pair is one marker drawn twice"
    );
    let anchored = arm_geometry(&mut app, Side::Left);
    let idle = arm_geometry(&mut app, Side::Right);
    assert_ne!(
        anchored, idle,
        "the two states reach the screen as the same geometry {anchored:?} — the difference has \
         to be visible, not only in a component"
    );

    // And the other way round, so a marker hard-wired to `Side::Left` cannot pass.
    set_arm(&mut app, Side::Left, HookState::Idle);
    set_arm(&mut app, Side::Right, HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO });
    app.world_mut()
        .run_system_once(arm_aim::sense_arm_aim)
        .expect("the one-shot system runs");
    assert_eq!(arm_state(&mut app, Side::Right), ArmAimState::Anchored);
    assert_eq!(arm_state(&mut app, Side::Left), ArmAimState::Free);

    // A retracting arm is not a ready one: `vector::hook` only fires from `Idle`, so a `Ready`
    // ring over an arm that cannot shoot is a promise the simulation does not keep.
    set_arm(&mut app, Side::Left, HookState::Retracting);
    app.world_mut()
        .run_system_once(arm_aim::sense_arm_aim)
        .expect("the one-shot system runs");
    assert_eq!(arm_state(&mut app, Side::Left), ArmAimState::Busy);
}

#[test]
fn f171_the_arm_markers_read_the_aim_point() {
    // The other half of the wiring: with both arms idle the shape is decided by
    // `AimPoint::anchorable`, which `vector::aim` writes. Goes red when somebody leaves the
    // system registered and empties its body — the failure this repo has already had once.
    use defeated_by_titan::shared::AimPoint;
    let mut app = app();
    let player = local_player(&mut app);

    for (anchorable, expected) in
        [(false, ArmAimState::Free), (true, ArmAimState::Ready), (false, ArmAimState::Free)]
    {
        app.world_mut().entity_mut(player).insert(AimPoint {
            point_m: Some(Vec3::new(0.0, 1.6, -10.0)),
            body: Some(BodyId(1)),
            anchorable,
        });
        app.world_mut()
            .run_system_once(arm_aim::sense_arm_aim)
            .expect("the one-shot system runs");
        for side in Side::ALL {
            assert_eq!(
                arm_state(&mut app, side),
                expected,
                "with `anchorable: {anchorable}` the {side:?} marker has to be {expected:?}"
            );
        }
    }
}

#[test]
fn f170_the_arm_markers_stay_out_of_the_middle_in_every_state() {
    // `f170_nothing_covers_the_middle_of_the_screen` sees this pair only in the state a fresh
    // player is in. The pair changes size with its state, so it gets its own loop over all four
    // — including the widest glyph and the one with the tether.
    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    assert!(w > 0.0 && h > 0.0, "the UI laid out into a {w} x {h} viewport");

    let box_min_x = w * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = w * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = h * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = h * KEEP_OUT_HIGH_PCT / 100.0;

    for state in ARM_STATES {
        set_arm_state(&mut app, state);
        app.world_mut()
            .run_system_once(arm_aim::shape_arm_aim)
            .expect("the one-shot system runs");
        app.world_mut().run_schedule(PostUpdate);
        app.world_mut().run_schedule(PostUpdate);

        let mut q = app
            .world_mut()
            .query_filtered::<(&Name, &Node, &ComputedNode, &UiGlobalTransform), Or<(With<ArmMarker>, With<ArmMarkerLabel>)>>();
        let mut seen = 0;
        for (name, node, computed, at) in q.iter(app.world()) {
            if node.display == Display::None {
                continue;
            }
            let (min_x, min_y, max_x, max_y) = rect(computed, at);
            if max_x - min_x <= 0.0 || max_y - min_y <= 0.0 {
                continue;
            }
            seen += 1;
            let overlaps =
                min_x < box_max_x && max_x > box_min_x && min_y < box_max_y && max_y > box_min_y;
            assert!(
                !overlaps,
                "in {state:?} `{name}` covers the middle of the screen: \
                 ({min_x:.1}, {min_y:.1})..({max_x:.1}, {max_y:.1}) against the box \
                 ({box_min_x:.1}, {box_min_y:.1})..({box_max_x:.1}, {box_max_y:.1})"
            );
        }
        // Two glyphs and two letters at the very least, plus the tethers where a state has them.
        let expected = 2 * arm_aim::node_count(state) + 2;
        assert_eq!(
            seen, expected,
            "in {state:?} only {seen} of the expected {expected} marker nodes were laid out — \
             the test just checked almost nothing, and requirement 1 is that both markers are \
             visible ALWAYS"
        );
    }
}

#[test]
fn f171_the_marker_letters_are_the_keys_that_fire_the_arms() {
    // `hud` may not reach into `net`, so the letters `Q` and `E` are written a second time in
    // `src/hud/arm_aim.rs`. This is the test that keeps the two spellings equal: it reads the
    // binding out of `src/net/local.rs` and falls over the day a hook is rebound without the
    // label following. A label that names the wrong key is worse than no label.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/net/local.rs"),
    )
    .expect("src/net/local.rs must be readable");
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for (side, button) in [(Side::Left, "HOOK_LEFT"), (Side::Right, "HOOK_RIGHT")] {
        let letter = arm_aim::key_label(side);
        let expected = format!("Buttons::{button}, keys.pressed(KeyCode::Key{letter})");
        assert!(
            code.contains(&expected),
            "the {side:?} marker is labelled `{letter}`, but `src/net/local.rs` does not \
             contain `{expected}` — the HUD is naming a key that does not fire this arm"
        );
    }
    assert_ne!(arm_aim::key_label(Side::Left), arm_aim::key_label(Side::Right));
}

#[test]
fn f171_the_arm_markers_cost_no_ray_and_no_sweep() {
    // `CLAUDE.md` rule 6: nothing runs over all entities to answer a question about the ten
    // metres in front of your nose. The crosshair next door pays for one cortex-filtered ray per
    // frame and says so in its header; this element pays for **none** — it reads `Hook` and
    // `AimPoint`, which `vector` has already written this tick.
    //
    // The bound is deliberately loose. What it is here to catch is not a microsecond, it is the
    // day somebody "improves" the preview by casting a probe ray per arm: two `SpatialQuery`
    // casts against a hundred blocks plus the system overhead do not fit into it, and a pair of
    // component reads does with two orders of magnitude to spare.
    let mut app = app();
    attach_screen(&mut app);
    let sense = app.world_mut().register_system(arm_aim::sense_arm_aim);
    let shape = app.world_mut().register_system(arm_aim::shape_arm_aim);
    let paint = app.world_mut().register_system(arm_aim::paint_arm_aim);

    // Warm up: the first call builds the system state, and that cost is paid once ever.
    for _ in 0..100 {
        app.world_mut().run_system(sense).expect("the system runs");
        app.world_mut().run_system(shape).expect("the system runs");
        app.world_mut().run_system(paint).expect("the system runs");
    }
    let rounds = 2_000;
    let start = std::time::Instant::now();
    for _ in 0..rounds {
        app.world_mut().run_system(sense).expect("the system runs");
        app.world_mut().run_system(shape).expect("the system runs");
        app.world_mut().run_system(paint).expect("the system runs");
    }
    let per_frame_us = start.elapsed().as_secs_f64() * 1e6 / f64::from(rounds);
    println!("f171 arm markers: {per_frame_us:.3} us per frame for all three systems");
    assert!(
        per_frame_us < 50.0,
        "the three arm-marker systems cost {per_frame_us:.3} us per frame — that is the order of \
         a spatial query, and this element is not allowed to cast one"
    );
}
