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
use defeated_by_titan::hud::hit_mark::{HitFlash, HitMark};
use defeated_by_titan::hud::objective::ObjectiveLine;
use defeated_by_titan::hud::{HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use defeated_by_titan::mission::{KillTally, MissionPhase};
use defeated_by_titan::shared::{
    BodyId, Blades, Cli, Gas, Health, HitZone, Hook, HookReleased, HookState, LocalPlayer,
    MissReason, PlayerId, PlayerSettings, ReleaseReason, Side, TitanHit, TitanId,
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
    //
    // **The two documented exceptions are not in this app**: an arm marker that carries a place —
    // a fan preview (FIND-098) or a tip, an anchor, a fallback (FIND-129) — stands on that place
    // and is held out of the aim pixel instead. Nothing here has an `ArmAim`, so both markers are
    // badges in their side slots and every node below is under the full rule. The `F-016` search
    // band is the second, and it is absent here for a reason and not by luck: both assist knobs
    // ship at 0 (`PlayerSettings::from_world`), so no probe is cast and no tick is drawn. What
    // holds it to the sight core instead is
    // `f016_the_band_keeps_the_sight_core_clear`, over the whole slider — this test would let
    // the band through, and that is exactly why the other one exists.
    //
    // **The `F-026` anchor marks are the third case of the FIRST exception, and they are
    // skipped here by name.** Every one of them carries a *place* — an anchor point, or the
    // point a rope is on — which is exactly the class that exception names, so they are held
    // out of `arm_aim::SIGHT_CORE_PX` and not out of the 20 % box. Skipping them here is not a
    // hole: `f026_no_anchor_mark_ever_touches_the_sight_core` measures them over 24 looks and
    // `f026_every_anchor_mark_stands_on_its_own_projected_pixel` holds each one to 0.6 px of
    // its own projection, which is a stricter rule than this test could ever be.
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
        // The `F-026` field — see the header of this test. Skipped by the name its own spawner
        // gives it, so a node that stops being an anchor mark stops being skipped.
        if name.as_str().starts_with("hud_anchor_") {
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
fn f171_the_arm_markers_read_the_aim_point() {
    // The other half of the wiring: with both arms idle the shape is decided by
    // `anchorable` — **per arm**, out of `ArmAim`, since W3. Goes red when somebody leaves the
    // system registered and empties its body, and red as well if the two arms are ever fed one
    // shared answer again: the third row asks for `Q` ready and `E` free at the same time,
    // which the shared `AimPoint` could not express at all.
    use defeated_by_titan::shared::{AimPoint, ArmAim};
    let mut app = app();
    let player = local_player(&mut app);

    let point = |anchorable: bool| AimPoint {
        point_m: Some(Vec3::new(0.0, 1.6, -10.0)),
        body: Some(BodyId(1)),
        anchorable,
    };
    for (left_ok, right_ok) in [(false, false), (true, true), (true, false), (false, true)] {
        app.world_mut()
            .entity_mut(player)
            .insert(ArmAim { arms: [point(left_ok), point(right_ok)] });
        app.world_mut()
            .run_system_once(arm_aim::sense_arm_aim)
            .expect("the one-shot system runs");
        for (side, ok) in [(Side::Left, left_ok), (Side::Right, right_ok)] {
            let expected = if ok { ArmAimState::Ready } else { ArmAimState::Free };
            assert_eq!(
                arm_state(&mut app, side),
                expected,
                "with `anchorable: {ok}` on that arm the {side:?} marker has to be {expected:?} \
                 (the pair was left={left_ok}, right={right_ok})"
            );
        }
    }
}

#[test]
fn f170_the_arm_markers_stay_out_of_the_middle_in_every_state() {
    // `f170_nothing_covers_the_middle_of_the_screen` sees this pair only in the state a fresh
    // player is in. The pair changes size with its state, so it gets its own loop over all four
    // — including the widest glyph and the one with the tether.
    //
    // **Scope, since FIND-098 and FIND-129:** this is the *badge* claim, and the bare app is
    // exactly that case — no `ArmAim` data, so neither arm has a point of its own and both
    // markers park in their side slots. A marker that carries a place is deliberately allowed
    // inside the box and may only not cover the aim pixel; that is swept by
    // `f023_the_drawn_marker_is_strictly_monotone_in_the_resolved_fan` for the fan and by
    // `f023_the_drawn_pixel_is_the_projection_of_the_point_the_rope_flies_to` for the rest.
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
    // The projection is in here too. It is the only one of the four that touches the camera, so
    // it is the only one that could quietly start costing something.
    let place = app.world_mut().register_system(arm_aim::place_arm_aim);

    // Warm up: the first call builds the system state, and that cost is paid once ever.
    for _ in 0..100 {
        app.world_mut().run_system(sense).expect("the system runs");
        app.world_mut().run_system(shape).expect("the system runs");
        app.world_mut().run_system(place).expect("the system runs");
        app.world_mut().run_system(paint).expect("the system runs");
    }
    let rounds = 2_000;
    let start = std::time::Instant::now();
    for _ in 0..rounds {
        app.world_mut().run_system(sense).expect("the system runs");
        app.world_mut().run_system(shape).expect("the system runs");
        app.world_mut().run_system(place).expect("the system runs");
        app.world_mut().run_system(paint).expect("the system runs");
    }
    let per_frame_us = start.elapsed().as_secs_f64() * 1e6 / f64::from(rounds);
    println!("f171 arm markers: {per_frame_us:.3} us per frame for all four systems");
    assert!(
        per_frame_us < 50.0,
        "the four arm-marker systems cost {per_frame_us:.3} us per frame — that is the order of \
         a spatial query, and this element is not allowed to cast one"
    );
}

// ---------------------------------------------------------------------------------------
// F-171 — the landing preview: the marker stands on a WORLD point, and it MOVES
//
// The user, after playing on 2026-08-12:
//   *"es soll previewd werden wo der aktuelle haken landen würde! also sollte richtig
//    angezeigt werden. nicht nur am fadenkreuz. weil das stimmt auch nicht."*
//   *"zudem sollen diese weiter auseinander sein. also weiter rechts und links!"*
//
// `FINDINGS.md` FIND-047 measured why he is right: the two markers were pinned at fixed screen
// percentages and photographed at **the same pixels across four runs with four different
// aims**. The tests below are the ones that could not have passed then.
// ---------------------------------------------------------------------------------------

/// Where the local player is standing and looking, in one call.
///
/// `Intent` and not the camera `Transform`: `render::camera::rotate_camera` is what turns the
/// camera, out of exactly this `Intent`, and a test that turned the camera itself would be
/// measuring its own arithmetic instead of the game's (`tests/render.rs`).
fn stand_and_look(app: &mut App, at_m: Vec3, yaw_deg: f32, pitch_deg: f32) {
    use defeated_by_titan::shared::Intent;
    let player = local_player(app);
    {
        let mut e = app.world_mut().entity_mut(player);
        let mut t = e.get_mut::<Transform>().expect("the player has a `Transform`");
        t.translation = at_m;
    }
    {
        let mut e = app.world_mut().entity_mut(player);
        let mut intent = e.get_mut::<Intent>().expect("the player has an `Intent`");
        intent.yaw = yaw_deg.to_radians();
        intent.pitch = pitch_deg.to_radians();
    }
}

/// Anchors one arm at a **world** point and tells the marker about it.
fn anchor_arm_at(app: &mut App, side: Side, world_m: Vec3) {
    let player = local_player(app);
    let mut hook = app
        .world_mut()
        .entity_mut(player)
        .get::<Hook>()
        .copied()
        .expect("the player carries a `Hook`");
    hook.arms[side.index()].state =
        HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO };
    hook.arms[side.index()].tip_m = world_m;
    app.world_mut().entity_mut(player).insert(hook);
}

/// Runs the whole HUD the way the game runs it, then lays the UI out.
///
/// `Update` and `PostUpdate` by hand and never `app.update()` — the reason is the file header:
/// `First` fills `Time<Virtual>` off the wall clock, a fixed step then falls inside the frame
/// depending on the machine's mood, and `vector::aim` writes a real raycast into `AimPoint`
/// underneath a test that just set it.
fn run_hud(app: &mut App) {
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);
}

/// The centre of one arm's **glyph** on screen, in physical pixels.
fn glyph_centre(app: &mut App, side: Side) -> Vec2 {
    use defeated_by_titan::hud::arm_aim::MarkerPart;
    let mut q = app
        .world_mut()
        .query::<(&ArmMarker, &Node, &ComputedNode, &UiGlobalTransform)>();
    for (marker, node, computed, at) in q.iter(app.world()) {
        if marker.side != side
            || marker.part != MarkerPart::Glyph
            || node.display == Display::None
        {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = rect(computed, at);
        return Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    }
    panic!("the {side:?} glyph is not on screen");
}

#[test]
fn f171_a_free_aim_point_projects_onto_the_crosshair() {
    // ★ **The measurement the whole design hangs on, and it is not a wish.**
    //
    // `vector::aim` casts from `translation + Y·eye_height_m` along `intent.look_dir()`.
    // `render::attach_camera` hangs the camera on the player at exactly
    // `Transform::from_xyz(0, eye_height_m, 0)`, and `rotate_camera` gives it
    // `Ry(yaw)·Rx(pitch)` — so the ray's origin **is** the camera's position and its direction
    // **is** the camera's forward. Every point on that ray therefore projects onto the same
    // pixel, and that pixel is the middle of the screen.
    //
    // That is why "preview where the free hook would land" cannot be drawn anywhere but on the
    // crosshair, and why two DIFFERENT idle points need `F-023`'s hemispheres first. This test
    // measures it instead of arguing it: two points at 8 m and at 90 m along the look ray, at
    // two different look angles, all four on the centre pixel.
    use defeated_by_titan::shared::Intent;
    let mut app = app();
    attach_screen(&mut app);

    let eye_height_m = app.world().resource::<GameData>().game.player.eye_height_m;
    let (w, h) = screen(&mut app);
    let centre = Vec2::new(w * 0.5, h * 0.5);

    for (yaw_deg, pitch_deg) in [(0.0_f32, 0.0_f32), (37.0, -12.0), (-140.0, 25.0)] {
        stand_and_look(&mut app, Vec3::new(4.0, 0.0, -9.0), yaw_deg, pitch_deg);
        run_hud(&mut app);

        let player = local_player(&mut app);
        let (intent, transform) = {
            let e = app.world().entity(player);
            (*e.get::<Intent>().unwrap(), *e.get::<Transform>().unwrap())
        };
        let eye = transform.translation + Vec3::Y * eye_height_m;

        let (camera, cam_at) = {
            let mut q = app
                .world_mut()
                .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>();
            let (c, t) = q.iter(app.world()).next().expect("there is a 3D camera");
            (c.clone(), *t)
        };

        for distance_m in [8.0_f32, 90.0] {
            let point = eye + intent.look_dir() * distance_m;
            let px = camera
                .world_to_viewport(&cam_at, point)
                .expect("a point in front of the camera projects");
            let off = (px * (w / camera.logical_viewport_size().unwrap().x) - centre).length();
            println!(
                "f171 free aim: yaw {yaw_deg}, pitch {pitch_deg}, {distance_m} m -> \
                 {px:?}, {off:.3} px off centre"
            );
            assert!(
                off < 1.5,
                "the free aim point at {distance_m} m projects {off:.1} px away from the \
                 crosshair. If that is really true, a per-arm landing preview for an IDLE arm \
                 is drawable without F-023 — and the whole argument in `hud::arm_aim` for why \
                 it is not has to be rewritten"
            );
        }
    }
}

#[test]
fn f171_an_anchored_marker_follows_its_anchor_across_the_screen() {
    // ★ **The user's sentence, as a test.** *"nicht nur am fadenkreuz. weil das stimmt auch
    // nicht."* FIND-047 measured the markers at the same pixels in four runs with four
    // different aims, because `node_for` pinned them to `top 65 %` / `left|right 52 %`.
    //
    // An anchored arm is holding a real point in the world — the one `render::rope` draws to.
    // Turn the head and that point has to travel across the screen. Goes red against a marker
    // that is a badge in a fixed slot.
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::new(0.0, 0.0, 0.0), 0.0, 0.0);

    // 30 m ahead, 18 m up: comfortably inside the frustum at every yaw the loop uses.
    let anchor = Vec3::new(0.0, 18.0, -30.0);
    anchor_arm_at(&mut app, Side::Left, anchor);
    anchor_arm_at(&mut app, Side::Right, anchor);

    let mut seen: Vec<(f32, Vec2)> = Vec::new();
    for yaw_deg in [-14.0_f32, -7.0, 0.0, 7.0, 14.0] {
        stand_and_look(&mut app, Vec3::ZERO, yaw_deg, 0.0);
        run_hud(&mut app);
        let at = glyph_centre(&mut app, Side::Left);
        println!("f171 anchor follow: yaw {yaw_deg} -> glyph at {at:?}");
        seen.push((yaw_deg, at));
    }

    // **The direction is asserted, not just the movement.** `yaw = 0` looks towards −Z and
    // `Intent::look_dir` is `(-sin y·cos p, sin p, -cos y·cos p)`, so a rising yaw turns the head
    // to the LEFT — and a fixed point in front of you then travels to the RIGHT across the
    // screen, monotonically. A marker that moved the wrong way would pass a test that only asked
    // whether it moved at all, and it would be unusable in exactly the way FIND-047 describes.
    for pair in seen.windows(2) {
        let (yaw_a, a) = pair[0];
        let (yaw_b, b) = pair[1];
        assert!(
            b.x > a.x + 10.0,
            "yaw went {yaw_a} -> {yaw_b} and the anchored marker moved from {a:?} to {b:?}. \
             A marker that does not travel with its anchor is a badge in a fixed slot, and that \
             is exactly what the player called out (FIND-047)"
        );
    }
    let travel = seen.last().unwrap().1.x - seen.first().unwrap().1.x;
    println!("f171 anchor follow: 28 deg of yaw moved the marker {travel:.1} px");
    assert!(travel > 100.0, "28 deg of yaw only moved the marker {travel:.1} px");
}

#[test]
fn f171_two_anchors_put_the_two_markers_on_two_different_points() {
    // ★ *"zudem sollen diese weiter auseinander sein. also weiter rechts und links!"*
    //
    // Two arms holding two different buildings are two different world points — the one case in
    // which the pair is genuinely two points today, and the case FIND-039 says free aiming
    // cannot produce. The gap between the two markers has to be the gap between the two
    // anchors, and it has to CHANGE when the anchors do. A fixed pair of slots gives a constant.
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::ZERO, 0.0, 0.0);

    let mut gaps = Vec::new();
    for spread_m in [10.0_f32, 22.0, 40.0] {
        anchor_arm_at(&mut app, Side::Left, Vec3::new(-spread_m, 14.0, -34.0));
        anchor_arm_at(&mut app, Side::Right, Vec3::new(spread_m, 14.0, -34.0));
        run_hud(&mut app);
        let left = glyph_centre(&mut app, Side::Left);
        let right = glyph_centre(&mut app, Side::Right);
        assert!(
            left.x < right.x,
            "the anchor at -{spread_m} m drew right of the anchor at +{spread_m} m \
             ({left:?} against {right:?}) — the two markers are swapped"
        );
        let gap = right.x - left.x;
        println!("f171 two anchors: +-{spread_m} m -> gap {gap:.1} px ({left:?} {right:?})");
        gaps.push(gap);
    }
    for pair in gaps.windows(2) {
        assert!(
            pair[1] > pair[0] + 20.0,
            "the anchors moved apart and the gap went {:.1} -> {:.1} px. The markers are not \
             standing on the anchors",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn f170_an_anchor_dead_ahead_stands_on_the_anchor_and_off_the_sight_core() {
    // ★ **The trap, sprung on purpose** — and since FIND-129 it catches the opposite lie.
    //
    // A marker that follows a world point will sooner or later be aimed straight at; that is not
    // an edge case, it is what a player does before he shoots. This test used to demand that
    // `F-170`'s box win that collision, and the box winning it is what drew the marker
    // **146 px** from an anchor 30 m dead ahead. The user, 2026-08-19: *„wichtig wäre nur dass
    // diese auch genau da sind visuell wo das seil auch landen würde!"*
    //
    // So the claim is now the pair of things that can both be true: the glyph stands on the
    // anchor's own pixel, and it is off the [`SIGHT_CORE_PX`] square the player is cutting.
    use defeated_by_titan::hud::arm_aim::SIGHT_CORE_PX;
    let mut app = app();
    attach_screen(&mut app);
    let eye_height_m = app.world().resource::<GameData>().game.player.eye_height_m;
    stand_and_look(&mut app, Vec3::ZERO, 0.0, 0.0);

    // Straight ahead at eye height: this projects onto the exact centre pixel.
    let dead_ahead = Vec3::new(0.0, eye_height_m, -30.0);
    anchor_arm_at(&mut app, Side::Left, dead_ahead);
    anchor_arm_at(&mut app, Side::Right, dead_ahead);
    run_hud(&mut app);

    let (w, h) = screen(&mut app);
    let (camera, camera_at) = camera_of(&mut app);
    let want = camera
        .world_to_viewport(&camera_at, dead_ahead)
        .expect("an anchor 30 m dead ahead projects onto the screen");
    assert!(
        (want - Vec2::new(w * 0.5, h * 0.5)).length() < 1.0,
        "the anchor was supposed to be on the centre pixel and projects to {want:?}"
    );

    let mut seen = 0;
    let mut q = app
        .world_mut()
        .query_filtered::<(&Name, &Node, &ComputedNode, &UiGlobalTransform), Or<(With<ArmMarker>, With<ArmMarkerLabel>)>>();
    for (name, node, computed, at) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = rect(computed, at);
        if max_x - min_x <= 0.0 || max_y - min_y <= 0.0 {
            continue;
        }
        seen += 1;
        let covers_core = min_x < want.x + SIGHT_CORE_PX
            && max_x > want.x - SIGHT_CORE_PX
            && min_y < want.y + SIGHT_CORE_PX
            && max_y > want.y - SIGHT_CORE_PX;
        assert!(
            !covers_core,
            "{name} sits at x {min_x:.1}..{max_x:.1}, y {min_y:.1}..{max_y:.1} with both arms \
             anchored dead ahead — on the {SIGHT_CORE_PX} px square around {want:?} that the \
             player is cutting. The marker may stand on its anchor; it may not stand on the \
             blade's own pixels"
        );
    }
    assert!(seen >= 4, "only {seen} arm nodes were measured — the test is looking at nothing");

    // The glyphs are on the anchor's x, and the vertical step out of the core is all that moved.
    let left = glyph_centre(&mut app, Side::Left);
    let right = glyph_centre(&mut app, Side::Right);
    println!("f170 dead ahead: {seen} nodes, anchor at {want:?}, Q at {left:?}, E at {right:?}");
    for (side, at) in [(Side::Left, left), (Side::Right, right)] {
        assert!(
            (at.x - want.x).abs() < 1.0,
            "{side:?} is anchored on a point that projects to {want:?} and its glyph was drawn \
             at {at:?} — the x of a marker holding a place is that place"
        );
    }
    // One place, two ropes: the two glyphs coincide, and the letters keep them apart.
    assert!((left - right).length() < 1.0, "Q and E hold the same point and were drawn apart");
}

// ---------------------------------------------------------------------------------------
// F-026 — the marker stands where the rope lands
//
// > *"und da wo das seil am ende auch landet soll die markierung hin vom seil, dass man direkt
// > sieht wo man sich connected! **das ist wichtig**. und dann muss das seil auch dahin!!"*
// > — the user, 2026-08-12 (`docs/NEXT.md` §1A, requirement 9)
//
// Two sentences, two tests. The first says the marker reads the arm's own firing point; the
// second fires the hook and compares what left the hand against what the marker stood on.
// **`assert_eq!` on the `Vec3` and no tolerance** — a metre of "close enough" is exactly the
// gap FIND-047 lived in.
// ---------------------------------------------------------------------------------------

/// Runs **one whole simulation step**: `FixedUpdate`, the six stages of
/// `src/shared/schedule.rs`, so `vector::aim` casts its three real rays and `vector::hook`
/// reads the result in the same call.
///
/// **`FixedPreUpdate` deliberately stays out.** That is where `net::local::read_input` turns
/// the keyboard into an `Intent` (`src/net/mod.rs:58`), and it would overwrite the one the test
/// just set with "no keys are down". Nothing inside `FixedUpdate` writes `Intent`, which is why
/// a hand-set intent survives the step and the test is still looking at the real systems.
fn sim_step(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

/// The local player's `Hook` and `ArmAim` as one snapshot, out of the same tick.
fn aim_snapshot(app: &mut App) -> (Hook, defeated_by_titan::shared::ArmAim) {
    use defeated_by_titan::shared::ArmAim;
    let player = local_player(app);
    let e = app.world().entity(player);
    (
        *e.get::<Hook>().expect("the player carries a `Hook`"),
        *e.get::<ArmAim>().expect("`AimPoint` requires `ArmAim`, so the player carries one"),
    )
}

#[test]
fn f026_the_marker_stands_exactly_where_that_arm_fires() {
    // ★ **The acceptance number of `docs/NEXT.md` §1B W5: `|marker target − fired target| == 0`.**
    //
    // Not two values that agree, ONE value read twice: `vector::aim` resolves the side ray
    // (fallback to the centre ray included) into `ArmAim`, `hud::arm_aim::target_of` draws that
    // and `vector::hook::anchor_target` fires at it. The test runs a real fixed step on the
    // real map so the points are raycast, not invented, and then compares with `assert_eq!`.
    use defeated_by_titan::vector::hook::anchor_target;
    let mut app = app();

    let mut anchorable_seen = 0;
    for (yaw_deg, pitch_deg) in [(0.0_f32, 0.0_f32), (90.0, 10.0), (180.0, -5.0), (37.0, 20.0)] {
        stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), yaw_deg, pitch_deg);
        sim_step(&mut app);
        let (hook, arm_aim) = aim_snapshot(&mut app);

        for side in Side::ALL {
            let marker = arm_aim::target_of(&hook, &arm_aim, side);
            let fired = anchor_target(arm_aim.side(side)).map(|(point_m, _)| point_m);
            if arm_aim.side(side).anchorable {
                anchorable_seen += 1;
                assert_eq!(
                    marker, fired,
                    "yaw {yaw_deg} pitch {pitch_deg}, {side:?}: the marker stands on {marker:?} \
                     and the rope would fly to {fired:?}. That is FIND-047 again — the picture \
                     promising something the simulation does not do"
                );
                println!("f026 same value: yaw {yaw_deg} {side:?} -> {:?}", marker.unwrap());
            } else {
                assert_eq!(
                    fired, None,
                    "yaw {yaw_deg}, {side:?}: `anchorable` is false and yet a shot has a target"
                );
            }
        }
    }
    assert!(
        anchorable_seen > 0,
        "not one of the eight (yaw, side) pairs found anchorable geometry — the test compared \
         two `None`s eight times and proved nothing"
    );
}

#[test]
fn f026_the_rope_flies_at_the_point_the_marker_stood_on() {
    // ★ *"und dann muss das seil auch dahin!!"* — the second sentence, and the only one that
    // can be proven by firing. The marker's rule is applied to this tick's `ArmAim`, the hook
    // key is pressed in the same tick, and `HookState::Flying { target_m }` — the value
    // `vector::hook` walks the tip along and `render::rope` draws to — has to be that same
    // `Vec3`, bit for bit.
    use defeated_by_titan::shared::{Buttons, Intent, PrevButtons};
    let mut app = app();

    // A stand where at least one arm catches: the market-square church, the same aim
    // `scripts/f-001-hooks.txt` and `scripts/f171-crosshair.txt` both anchor from.
    stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), 0.0, 34.0);
    sim_step(&mut app);

    let player = local_player(&mut app);
    {
        let mut e = app.world_mut().entity_mut(player);
        let mut intent = e.get_mut::<Intent>().expect("the player has an `Intent`");
        intent.buttons.set(Buttons::HOOK_LEFT, true);
        intent.buttons.set(Buttons::HOOK_RIGHT, true);
    }
    // Both keys have to be *fresh* this tick: `vector::hook` fires on the edge, never on the
    // held button.
    app.world_mut().entity_mut(player).insert(PrevButtons(Buttons::NONE));

    // ⚠️ **The preview is taken from the tick the shot happens in, not from the one before it**,
    // and that is not pedantry: measured while writing this test, the aim point moved
    // 11.0431 → 11.0615 m between two ticks because the player is still settling onto the
    // ground. Any test with a tolerance loose enough to swallow those 18 mm is loose enough to
    // swallow a real mismatch, which is why this one has none.
    //
    // Within the step the order is fixed by `SimulationSystems`: `vector::aim` writes `ArmAim`
    // in `World`, `vector::hook` reads it in `Intent`. So the `ArmAim` standing after the step
    // **is** the one the shot used, and it is the one `hud::arm_aim` draws in the same frame.
    // `Hook::default()` is the arm as the marker saw it at the instant the key went down: idle.
    let before = aim_snapshot(&mut app).1;
    sim_step(&mut app);
    let (hook, after) = aim_snapshot(&mut app);
    let previewed = Side::ALL.map(|side| arm_aim::target_of(&Hook::default(), &after, side));
    println!("f026 aim drift between two ticks: {before:?} -> {after:?}");

    let mut fired = 0;
    for side in Side::ALL {
        if let HookState::Flying { target_m, .. } = hook.arm(side).state {
            fired += 1;
            assert_eq!(
                Some(target_m),
                previewed[side.index()],
                "{side:?}: the marker stood on {:?} and the rope left for {target_m:?}",
                previewed[side.index()]
            );
            println!("f026 rope: {side:?} flies at {target_m:?}, the marker's own point");
        }
    }
    assert!(fired > 0, "neither arm left the hand — the test proved nothing about the rope");
}

#[test]
fn f026_an_anchor_behind_you_goes_to_the_edge_on_its_own_side() {
    // ★ **The hard case, decided rather than dodged.** `Camera::world_to_viewport` refuses a
    // point behind the near plane, and half of every swing is spent with the anchor behind the
    // player. The choice (module header of `src/hud/arm_aim.rs`): clamp to the screen edge on
    // the side the anchor really is, do not hide. This test is what makes "the right side" a
    // fact — a marker that answered (0, 0) or jumped to its own slot would pass a test that
    // only asked whether it was on screen.
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::ZERO, 0.0, 0.0);
    let (w, _h) = screen(&mut app);

    // yaw 0 looks towards −Z, so +Z is behind. One anchor behind-and-left, one behind-and-right.
    anchor_arm_at(&mut app, Side::Left, Vec3::new(-25.0, 6.0, 18.0));
    anchor_arm_at(&mut app, Side::Right, Vec3::new(25.0, 6.0, 18.0));
    run_hud(&mut app);
    let left = glyph_centre(&mut app, Side::Left);
    let right = glyph_centre(&mut app, Side::Right);
    println!("f026 behind: Q at {left:?}, E at {right:?} on a {w:.0} px screen");
    assert!(left.x < w * 0.15, "the anchor behind-LEFT drew the Q marker at {left:?}");
    assert!(right.x > w * 0.85, "the anchor behind-RIGHT drew the E marker at {right:?}");

    // Now swap the two anchors over: the markers have to swap with them, which a pair parked
    // in fixed side slots would never do.
    anchor_arm_at(&mut app, Side::Left, Vec3::new(25.0, 6.0, 18.0));
    anchor_arm_at(&mut app, Side::Right, Vec3::new(-25.0, 6.0, 18.0));
    run_hud(&mut app);
    let left = glyph_centre(&mut app, Side::Left);
    let right = glyph_centre(&mut app, Side::Right);
    println!("f026 behind, swapped: Q at {left:?}, E at {right:?}");
    assert!(left.x > w * 0.85, "Q's anchor moved behind-RIGHT and the marker stayed at {left:?}");
    assert!(right.x < w * 0.15, "E's anchor moved behind-LEFT and the marker stayed at {right:?}");
}

// ---------------------------------------------------------------------------------------
// F-026 — a fired arm previews where it LANDS, not the hook in its hand
// ---------------------------------------------------------------------------------------

/// Sends one arm out at a world point exactly the way `vector::hook::fire` does.
///
/// **Including its decision 5: the tip starts in the hand**, i.e. at the eye, i.e. inside the
/// camera. That detail is the whole point of the test below — a marker that reads the tip has
/// to project a point sitting on the camera's own near plane.
fn fire_arm_at(app: &mut App, side: Side, target_m: Vec3) {
    let eye_height_m = app.world().resource::<GameData>().game.player.eye_height_m;
    let player = local_player(app);
    let hand_m = app
        .world()
        .entity(player)
        .get::<Transform>()
        .expect("the player has a `Transform`")
        .translation
        + Vec3::Y * eye_height_m;
    let mut hook = app
        .world()
        .entity(player)
        .get::<Hook>()
        .copied()
        .expect("the player carries a `Hook`");
    hook.arms[side.index()].state = HookState::Flying { target_m, body: BodyId(1) };
    hook.arms[side.index()].tip_m = hand_m;
    app.world_mut().entity_mut(player).insert(hook);
}

/// Puts **one** aim point `d_m` dead ahead into both halves of `ArmAim` and lays the HUD out —
/// the shape `vector::aim` publishes since the fan was retired (`docs/QUESTIONS.md` Q-048).
///
/// Returns the same point twice, indexed by [`Side`], so a caller can fire each arm at what its
/// own marker was standing on.
fn plant_one_aim_point(app: &mut App, d_m: f32) -> [Vec3; 2] {
    use defeated_by_titan::shared::{AimPoint, ArmAim, Intent};
    let eye_height_m = app.world().resource::<GameData>().game.player.eye_height_m;
    let player = local_player(app);
    let intent = *app.world().entity(player).get::<Intent>().expect("the player has an `Intent`");
    let point = Vec3::Y * eye_height_m + intent.look_dir() * d_m;
    let one = AimPoint { point_m: Some(point), body: Some(BodyId(1)), anchorable: true };
    app.world_mut().entity_mut(player).insert(ArmAim { arms: [one, one] });
    run_hud(app);
    [point, point]
}

/// How far from the middle of the screen a marker parked in its **side slot** stands, for one
/// shape. The furthest out `layout_for` will ever put a marker on purpose — read out of the
/// layout itself rather than spelled again here, so the number cannot drift.
fn slot_offset_px(side: Side, state: ArmAimState, screen: Vec2) -> f32 {
    use defeated_by_titan::hud::arm_aim::{layout_for, shape_of};
    let shape = shape_of(state);
    let laid = layout_for(side, shape, None, screen);
    (laid.glyph.x + shape.glyph_w_px * 0.5 - screen.x * 0.5).abs()
}

#[test]
fn f026_a_fired_arm_previews_where_it_lands_and_not_the_hook_in_its_hand() {
    // ★ **The teleport, and it fires on every single shot.** The player's requirement is
    // *"und da wo das seil am ende auch landet soll die markierung hin ... dass man direkt
    // sieht wo man landet"* — where the rope ENDS UP. `vector::hook::fire` freezes that place
    // into `HookState::Flying { target_m }` and puts the tip **in the hand**; the tip is then
    // one metre from the camera for the first few ticks, `Camera::world_to_viewport` refuses
    // it, `edge_pixel` gives it a bearing, and the layout clamps it to the edge of the screen.
    //
    // So a marker reading `tip_m` answers "your target is off the right-hand edge" about a
    // point 40 m dead ahead, and then crawls back inwards over the flight as if the target
    // were moving. Measured here: it went to **608 px** off centre on a 1280 px screen, from
    // 105 px, in one frame — and the target had not moved at all.
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::ZERO, 0.0, 0.0);
    let (w, h) = screen(&mut app);
    let centre_x = w * 0.5;

    // Both arms home first, so each previews the shared aim point and not a stale `target_m`.
    // The point is planted rather than raycast: this test is about what happens when an arm is
    // **fired**, and a fixture that depends on what stands 40 m ahead in the map would make it
    // about the map instead. 40 m dead ahead, one body, both arms — which is what `vector::aim`
    // publishes since `F-023` was retired: one point in both halves of `ArmAim`.
    let points = plant_one_aim_point(&mut app, 40.0);
    for side in Side::ALL {
        set_arm(&mut app, side, HookState::Idle);
    }
    run_hud(&mut app);
    let before = Side::ALL.map(|side| glyph_centre(&mut app, side));

    // Fire both arms at **exactly** the two points their markers were standing on. Nothing
    // about the world changed; the only new fact is that the arms are committed.
    for side in Side::ALL {
        fire_arm_at(&mut app, side, points[side.index()]);
    }
    run_hud(&mut app);

    for side in Side::ALL {
        let after = glyph_centre(&mut app, side);
        let (off_before, off_after) =
            ((before[side.index()].x - centre_x).abs(), (after.x - centre_x).abs());
        let slot = slot_offset_px(side, ArmAimState::Busy, Vec2::new(w, h));
        println!(
            "f026 fire {side:?}: {off_before:.1} px -> {off_after:.1} px off centre \
             (side slot is {slot:.1} px)"
        );
        // Either it did not move at all — the honest answer, and the one a target outside the
        // box gets — or `F-170`'s box parked it in its own side slot. Never further: past the
        // slot is the projection running away, which is what reading the hook in the player's
        // hand produces.
        let stayed = (off_after - off_before).abs() <= 1.0;
        let parked = (off_after - slot).abs() <= 1.0;
        assert!(
            stayed || parked,
            "{side:?} fired at {:?} — the point its own marker was standing on \
             {off_before:.1} px off the centre of a {w:.0} px screen — and the marker jumped \
             to {off_after:.1} px, which is neither where it stood nor the {slot:.1} px side \
             slot. It is reading the hook in the player's hand instead of the place the hook \
             is flying to, so it says the target is off the edge of the screen while the \
             target is 40 m dead ahead",
            points[side.index()]
        );
    }
}

// ---------------------------------------------------------------------------------------
// F-043 — a landed blade says so, and a miss says nothing
// ---------------------------------------------------------------------------------------
//
// The user, after playing on 2026-08-19: *„attack fehlt aber noch (mit schwertern..)"*. The
// round that measured it (`scripts/f032-swords.txt`, the table in that file's header) found the
// swing firing, the cast landing and `TitanHit` written on all four acts — and nothing on screen
// changing on three of them, because `F-034`'s hit-stop and camera kick fire on a Cortex kill
// only. These tests are what stops that from coming back.

/// Writes the hit the way `blades::cut` writes it — the **real** message, not a stand-in.
fn land_a_hit(app: &mut App, by: PlayerId, zone: HitZone, speed_m_s: f32) {
    app.world_mut().write_message(TitanHit { titan: TitanId(1), by, zone, speed_m_s });
    app.world_mut().run_schedule(Update);
}

/// What the hit mark reads: `(text, font px, colour, drawn at all)`.
fn hit_mark(app: &mut App) -> (String, f32, Color, bool) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Text, &TextFont, &TextColor, &Node), With<HitMark>>();
    let (text, font, colour, node) = q.iter(app.world()).next().expect(
        "no node with `HitMark` — `hud::hit_mark::spawn_hit_mark` is not registered",
    );
    let px = match font.font_size {
        bevy::text::FontSize::Px(px) => px,
        other => panic!("the hit mark's font size is {other:?} and not a pixel size"),
    };
    (text.0.clone(), px, colour.0, node.display != Display::None)
}

fn me(app: &mut App) -> PlayerId {
    let player = local_player(app);
    *app.world().entity(player).get::<PlayerId>().expect("the local player carries a PlayerId")
}

#[test]
fn f043_a_landed_blade_puts_a_line_on_screen_and_nothing_else_does() {
    // ★ **The one with teeth.** This is the user's sentence turned into an assert: before
    // today every one of the four acts of `scripts/f032-swords.txt` looked identical on screen.
    // It goes red the moment the element is wired to something that is not the hit — a timer, a
    // constant, or the swing rather than its result.
    let mut app = app();
    let mine = me(&mut app);

    // 1. Nothing has happened. An empty screen is the honest state, and it is what makes the
    //    appearance below mean something.
    let (_, _, _, drawn) = hit_mark(&mut app);
    assert!(!drawn, "the hit mark is on screen before any blade has landed");

    // 2. A body cut at the speed the measured pass actually produced.
    land_a_hit(&mut app, mine, HitZone::Torso, 20.67);
    let (text, _, _, drawn) = hit_mark(&mut app);
    assert!(drawn, "a blade landed in the titan's body and the screen said nothing");
    assert!(
        text.contains("CUT") && text.contains("20.7"),
        "the line reads {text:?} — it has to name what landed and how fast"
    );

    // 3. The kill is a different line, not a louder version of the same one.
    land_a_hit(&mut app, mine, HitZone::Cortex, 21.0);
    let (kill_text, _, _, drawn) = hit_mark(&mut app);
    assert!(drawn);
    assert!(
        kill_text.contains("KILL"),
        "the cortex kill reads {kill_text:?} — the one hit the whole game is built around is \
         not being told apart from a scratch"
    );

    // 4. **The miss.** Nothing landed, so nothing may be said — and the previous mark has to be
    //    gone, or the line would keep asserting a hit that is over. The countdown itself is
    //    `hit_mark::step_flash`'s unit test; here the component is cleared the way the clock
    //    clears it and the screen has to follow.
    {
        let mut q = app.world_mut().query_filtered::<&mut HitFlash, With<HitMark>>();
        for mut flash in q.iter_mut(app.world_mut()) {
            *flash = HitFlash::default();
        }
    }
    app.world_mut().run_schedule(Update);
    let (text, _, _, drawn) = hit_mark(&mut app);
    assert!(!drawn, "the line is still drawn after the mark ran out: {text:?}");
    assert!(text.is_empty(), "the line still reads {text:?} with nothing to report");
}

#[test]
fn f043_a_team_mates_blade_does_not_flash_on_my_screen() {
    // `docs/multiplayer.md` rule 1. `TitanHit` carries `by`, every player's cuts travel on the
    // same channel, and a HUD that reads "the only hit there is" is a HUD that lies the day a
    // second player exists. Cheap to hold now, expensive to find later.
    let mut app = app();
    let mine = me(&mut app);
    let somebody_else = PlayerId(mine.0.wrapping_add(1));
    assert_ne!(somebody_else, mine);

    land_a_hit(&mut app, somebody_else, HitZone::Cortex, 30.0);
    let (text, _, _, drawn) = hit_mark(&mut app);
    assert!(
        !drawn,
        "another player's kill flashed on my screen and read {text:?}"
    );
}

#[test]
fn f043_the_three_hit_kinds_differ_in_word_size_and_colour() {
    // `F-171`'s rule about the arm markers, applied here: they may not differ **only** in
    // colour. Three signals per kind — word, size, colour — so a player who cannot tell amber
    // from crimson still reads three different lines.
    //
    // And the two colours are checked against `maps.ron`'s `signals:` block, not against a
    // literal: `docs/conventions.md` §3 makes amber the cortex and crimson the damage, and a
    // hard-coded red here would be exactly the drift that table exists to prevent.
    let mut app = app();
    let mine = me(&mut app);
    let (amber, crimson) = {
        let data = app.world().resource::<GameData>();
        (
            data.maps.signals.get("amber").copied().expect("maps.ron has an amber"),
            data.maps.signals.get("crimson").copied().expect("maps.ron has a crimson"),
        )
    };
    let strong = app.world().resource::<GameData>().gear.feel.strong_hit_m_s;

    let mut readings = Vec::new();
    for (label, zone, speed, want) in [
        ("kill", HitZone::Cortex, strong + 3.0, amber),
        ("cut", HitZone::Torso, strong + 3.0, crimson),
        ("graze", HitZone::Torso, strong - 3.0, crimson),
    ] {
        land_a_hit(&mut app, mine, zone, speed);
        let (text, px, colour, drawn) = hit_mark(&mut app);
        assert!(drawn, "the {label} was not drawn at all");
        let want = Color::linear_rgb(want.0, want.1, want.2);
        assert_eq!(
            colour.to_linear().to_vec3(),
            want.to_linear().to_vec3(),
            "the {label} is painted {colour:?} and `maps.ron` says {want:?}"
        );
        readings.push((label, text, px));
    }

    for (i, (a_name, a_text, a_px)) in readings.iter().enumerate() {
        for (b_name, b_text, b_px) in &readings[i + 1..] {
            let a_word = a_text.split_whitespace().next().unwrap_or("");
            let b_word = b_text.split_whitespace().next().unwrap_or("");
            assert_ne!(a_word, b_word, "{a_name} and {b_name} say the same word");
            assert_ne!(a_px, b_px, "{a_name} and {b_name} are drawn at the same size");
        }
    }
}

#[test]
fn f043_a_hold_of_zero_seconds_switches_the_whole_element_off() {
    // `F-043`'s row: *"Vollstaendig abschaltbar"*. One RON value, no settings screen, and **no
    // frame at all** — not "it flashes once and then stops", which is what a naive off-switch
    // that only shortens the countdown would produce.
    let mut app = app();
    let mine = me(&mut app);

    // The file ships it **on** — an off-switch nobody can tell from a broken feature is worth
    // nothing, so the shipped value is read first and has to be positive.
    let shipped = app.world().resource::<GameData>().gear.feel.hit_mark_s;
    assert!(
        shipped > 0.0,
        "`gear.ron: feel.hit_mark_s` ships at {shipped}, so the element is off in the game"
    );

    app.world_mut().resource_mut::<GameData>().gear.feel.hit_mark_s = 0.0;

    land_a_hit(&mut app, mine, HitZone::Cortex, 30.0);
    let (text, _, _, drawn) = hit_mark(&mut app);
    assert!(!drawn, "hit_mark_s = 0.0 still drew {text:?}");

}

#[test]
fn f043_the_hit_mark_stays_out_of_the_middle_in_every_kind() {
    // `F-170`: no node this domain spawns may intersect the central 20 % x 20 %. The hit mark
    // is 50 % of the screen wide, so it overlaps the box in x by construction and has to clear
    // it in y — at the **kill**'s font size, which is the biggest the element can be.
    //
    // `f170_nothing_covers_the_middle_of_the_screen` cannot see this element: it is
    // `Display::None` until something lands, and a hidden node covers nothing.
    let mut app = app();
    attach_screen(&mut app);
    let mine = me(&mut app);
    let (w, h) = screen(&mut app);
    assert!(w > 0.0 && h > 0.0, "the UI laid out into a {w} x {h} viewport");

    let box_min_x = w * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = w * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = h * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = h * KEEP_OUT_HIGH_PCT / 100.0;

    for (label, zone) in [("kill", HitZone::Cortex), ("cut", HitZone::Torso)] {
        land_a_hit(&mut app, mine, zone, 40.0);
        app.world_mut().run_schedule(PostUpdate);
        app.world_mut().run_schedule(PostUpdate);

        let mut q = app
            .world_mut()
            .query_filtered::<(&Node, &ComputedNode, &UiGlobalTransform), With<HitMark>>();
        let (node, computed, at) =
            q.iter(app.world()).next().expect("the hit mark node must exist");
        assert_ne!(node.display, Display::None, "the {label} was not laid out at all");
        let (min_x, min_y, max_x, max_y) = rect(computed, at);
        assert!(
            max_x - min_x > 0.0 && max_y - min_y > 0.0,
            "the {label}'s rect is empty — this test just checked nothing"
        );
        let overlaps =
            min_x < box_max_x && max_x > box_min_x && min_y < box_max_y && max_y > box_min_y;
        assert!(
            !overlaps,
            "the {label} covers the middle of the screen: ({min_x:.1}, {min_y:.1})..\
             ({max_x:.1}, {max_y:.1}) against the box ({box_min_x:.1}, {box_min_y:.1})..\
             ({box_max_x:.1}, {box_max_y:.1})"
        );
    }
}

// ---------------------------------------------------------------------------------------
// F-029 — the rope that is torn off a dying titan
// ---------------------------------------------------------------------------------------

/// What one arm's letter reads, and in what colour.
fn arm_letter(app: &mut App, side: Side) -> (String, Color) {
    let mut q = app.world_mut().query::<(&ArmMarkerLabel, &Text, &TextColor)>();
    for (label, text, colour) in q.iter(app.world()) {
        if label.0 == side {
            return (text.0.clone(), colour.0);
        }
    }
    panic!("no `ArmMarkerLabel` for {side:?}");
}

#[test]
fn f029_a_rope_torn_off_a_dying_titan_says_so_on_the_marker() {
    // `F-029`'s acceptance ends *"und loest sich beim Tod des Titanen **mit Feedback**"*. The
    // release itself has been right since 2026-08-18 (`tests/titan.rs::f029_the_rope_lets_go_
    // when_the_titan_dies_and_says_why`) — `vector::hook` writes `ReleaseReason::BodyGone` in
    // the tick the body leaves the index. What was missing is the second half: `sense_arm_miss`
    // matched **only** `ReleaseReason::NoAnchor(_)`, so the rope went slack in complete silence
    // and the player was left to guess whether he had let go himself.
    //
    // This uses `F-028`'s channel and `F-028`'s crimson label — not a second mechanism. Two
    // ways of saying "your arm has nothing" would drift apart within a week.
    let mut app = app();
    let mine = me(&mut app);
    let crimson = {
        let c = app.world().resource::<GameData>().maps.signals.get("crimson").copied().unwrap();
        Color::linear_rgb(c.0, c.1, c.2)
    };

    let (idle, _) = arm_letter(&mut app, Side::Left);
    assert_eq!(idle, "Q", "the resting letter is the key and nothing else");

    app.world_mut().write_message(HookReleased {
        player: mine,
        side: Side::Left,
        reason: ReleaseReason::BodyGone,
        tick: 1,
    });
    app.world_mut().run_schedule(Update);

    let (torn, ink) = arm_letter(&mut app, Side::Left);
    assert_ne!(
        torn, "Q",
        "the titan died under the hook and the marker said nothing — `F-029`'s acceptance is \
         \"loest sich beim Tod des Titanen mit Feedback\", and silence is not feedback"
    );
    assert_eq!(
        ink.to_linear().to_vec3(),
        crimson.to_linear().to_vec3(),
        "the torn rope is not painted in `F-028`'s crimson — a second colour for the same \
         kind of event is a second mechanism"
    );

    // The other arm is untouched: one rope tore, not two.
    let (other, _) = arm_letter(&mut app, Side::Right);
    assert_eq!(other, "E", "the right marker reacted to the left arm's rope");
}

#[test]
fn f029_a_torn_rope_and_an_empty_pull_do_not_say_the_same_thing() {
    // Two different situations that both leave the arm empty: *you* found nothing, versus *the
    // thing you were holding* died. They ask the player for different moves — aim again, or
    // look for the next anchor while you are already falling — so they may not read the same.
    let mut app = app();
    let mine = me(&mut app);

    app.world_mut().write_message(HookReleased {
        player: mine,
        side: Side::Left,
        reason: ReleaseReason::NoAnchor(MissReason::NothingInRange),
        tick: 1,
    });
    app.world_mut().run_schedule(Update);
    let (missed, _) = arm_letter(&mut app, Side::Left);

    app.world_mut().write_message(HookReleased {
        player: mine,
        side: Side::Left,
        reason: ReleaseReason::BodyGone,
        tick: 2,
    });
    app.world_mut().run_schedule(Update);
    let (torn, _) = arm_letter(&mut app, Side::Left);

    assert_ne!(
        missed, torn,
        "a pull that found nothing and a rope torn off a dead titan both read {missed:?}"
    );
}

#[test]
fn f029_a_release_i_asked_for_says_nothing() {
    // The control, and it is the one that keeps the two above honest: letting go of the button
    // is not a failure and must not flash anything. A marker that shouted on every release
    // would pass both tests above and be worse than silence.
    let mut app = app();
    let mine = me(&mut app);

    for reason in [ReleaseReason::Released, ReleaseReason::Overextended] {
        app.world_mut().write_message(HookReleased {
            player: mine,
            side: Side::Left,
            reason,
            tick: 1,
        });
        app.world_mut().run_schedule(Update);
        let (text, _) = arm_letter(&mut app, Side::Left);
        assert_eq!(text, "Q", "{reason:?} put {text:?} under the marker");
    }
}

// ---------------------------------------------------------------------------------------
// F-023 — the guard: the DRAWN pixel against the FIRED world point, in the running app
// ---------------------------------------------------------------------------------------

/// The 3D camera and its transform, so a test can project a world point **itself**.
///
/// `hud::arm_aim::place_arm_aim` uses exactly this pair; reading it here is not comparing a
/// function to itself (`docs/FINDINGS.md` FIND-103), because what is compared is the laid-out
/// **rectangle** on one side and a raw `Camera::world_to_viewport` on the other — the whole of
/// `target_of`, `bearing_of` and `layout_for` sits between the two and none of it is consulted.
fn camera_of(app: &mut App) -> (Camera, GlobalTransform) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>();
    let (camera, at) = q.iter(app.world()).next().expect("there must be a 3D camera");
    (camera.clone(), *at)
}

/// **Where this arm's rope really is or really goes**, read without asking `hud` anything.
///
/// Four states, four sources, and every one of them is a `vector` value:
/// - `Idle` — [`vector::hook::anchor_target`] on this arm's own [`AimPoint`]: the exact `Vec3`
///   `vector::hook::fire` would freeze into `Flying { target_m }` if the key went down now.
/// - `Flying` — that frozen `target_m`, which is where the rope **ends up** (FIND-099).
/// - `Anchored` / `Retracting` — `tip_m`, which is where `render::rope` really draws to.
fn rope_point(
    hook: &Hook,
    arms: &defeated_by_titan::shared::ArmAim,
    side: Side,
) -> Option<Vec3> {
    use defeated_by_titan::vector::hook::anchor_target;
    match hook.arm(side).state {
        HookState::Idle => anchor_target(arms.side(side)).map(|(point_m, _)| point_m),
        HookState::Flying { target_m, .. } => Some(target_m),
        HookState::Anchored { .. } | HookState::Retracting => Some(hook.arm(side).tip_m),
    }
}

/// Moves the two aim-assist knobs the way the settings screen moves them (`F-024` / `F-025`).
fn set_assist(app: &mut App, catch_pct: f32, strength_pct: f32) {
    let mut s = app.world_mut().resource_mut::<PlayerSettings>();
    s.assist_catch_pct = catch_pct;
    s.assist_strength_pct = strength_pct;
}

/// Puts one arm into a state **around a real world point**, the way the simulation would.
fn put_arm_on(app: &mut App, side: Side, state: &str, target_m: Vec3) {
    let eye_height_m = app.world().resource::<GameData>().game.player.eye_height_m;
    let player = local_player(app);
    let hand_m = app
        .world()
        .entity(player)
        .get::<Transform>()
        .expect("the player has a `Transform`")
        .translation
        + Vec3::Y * eye_height_m;
    let mut hook = app
        .world()
        .entity(player)
        .get::<Hook>()
        .copied()
        .expect("the player carries a `Hook`");
    let arm = &mut hook.arms[side.index()];
    match state {
        "idle" => {
            arm.state = HookState::Idle;
            arm.tip_m = hand_m;
        }
        "flying" => {
            arm.state = HookState::Flying { target_m, body: BodyId(1) };
            arm.tip_m = hand_m;
        }
        "anchored" => {
            arm.state = HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO };
            arm.tip_m = target_m;
        }
        "retracting" => {
            arm.state = HookState::Retracting;
            arm.tip_m = target_m;
        }
        other => panic!("no arm state called {other:?}"),
    }
    app.world_mut().entity_mut(player).insert(hook);
}

/// The four arm states the sweep below walks, by the name [`put_arm_on`] answers to.
const ROPE_STATES: [&str; 4] = ["idle", "flying", "anchored", "retracting"];

#[test]
fn f023_the_drawn_pixel_is_the_projection_of_the_point_the_rope_flies_to() {
    // ★ **The user, 2026-08-19:** *„wichtig wäre nur dass diese auch genau da sind visuell wo
    // das seil auch landen würde!"* — and this is the only test in the repository that can see
    // it, because it is the only one that compares the **laid-out rectangle** against a world
    // point projected by hand.
    //
    // Everything before it compared one world value with another (`f026_the_marker_stands_
    // exactly_where_that_arm_fires`) or one screen value against an angle (`f023_the_drawn_
    // marker_stands_at_the_resolved_fan_angle`, which only ever runs on idle arms). The gap
    // between the two is `layout_for`, and **both** of the last two lies lived exactly there:
    // FIND-098's fixed slot and FIND-099's 608 px teleport. So the guard has to straddle it.
    //
    // ## What is swept, and why each axis is in
    //
    // - **both assist knobs** at 0 / 25 / 50 / 75 / 100 (`F-024`/`F-025`, FIND-104): with the
    //   snap on, `ArmAim` is a *candidate* off the probe cone and not the ray's own hit, so the
    //   drawn pixel has to follow somewhere the fan angle cannot predict;
    // - **pitch from +89 to −89**, because `B-008` (FIND-121) lived at straight down;
    // - **three stands**, one of them 60 m up, so the near field, the far field past
    //   `aim_sep_full_reach_m` and the sky lane are all in;
    // - **all four arm states**, because FIND-099's lie was in exactly one of them and was
    //   invisible from the other three.
    //
    // ## The one allowance, and it is bounded
    //
    // A marker whose rectangle would sit on the pixels the player is cutting steps out of
    // `SIGHT_CORE_PX` — the 6 px square inside the crosshair. That is the only place the drawn
    // pixel may leave the projection, it is at most half a glyph plus six pixels, and the test
    // counts how often it fires so the allowance can never quietly become the rule.
    use defeated_by_titan::hud::arm_aim::{shape_of, SIGHT_CORE_PX};

    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    let centre = Vec2::new(w * 0.5, h * 0.5);

    let stands = [
        // The market square, where `scripts/f-001-hooks.txt` anchors from.
        Vec3::new(51.0, 0.0, 13.0),
        // FIND-121's own stand, the street `B-008` was measured over.
        Vec3::new(168.19, 0.0, -50.12),
        // 60 m up: the roofs are below, the towers past 108 m are ahead.
        Vec3::new(51.0, 60.0, 13.0),
    ];
    let looks: [(f32, f32); 9] = [
        (0.0, 0.0),
        (0.0, 89.0),
        (0.0, -89.0),
        (37.0, -60.0),
        (90.0, 10.0),
        (180.0, -5.0),
        (200.0, 45.0),
        (270.0, -30.0),
        (120.0, 3.0),
    ];
    let assists: [(f32, f32); 5] =
        [(0.0, 0.0), (25.0, 25.0), (50.0, 50.0), (75.0, 75.0), (100.0, 100.0)];

    let mut checked = 0;
    let mut strict = 0;
    let mut dodged = 0;
    let mut worst = (0.0_f32, String::new());
    let mut x_worst = (0.0_f32, String::new());

    for (catch_pct, strength_pct) in assists {
        set_assist(&mut app, catch_pct, strength_pct);
        for stand in stands {
            for (yaw_deg, pitch_deg) in looks {
                stand_and_look(&mut app, stand, yaw_deg, pitch_deg);
                // A real fixed step on the real map: `ArmAim` is raycast, coherence-tested and
                // snapped by the shipped systems, never invented here.
                sim_step(&mut app);
                let fired = {
                    use defeated_by_titan::vector::hook::anchor_target;
                    let (_, arms) = aim_snapshot(&mut app);
                    Side::ALL.map(|s| anchor_target(arms.side(s)).map(|(p, _)| p))
                };

                for state in ROPE_STATES {
                    for side in Side::ALL {
                        if let Some(target_m) = fired[side.index()] {
                            put_arm_on(&mut app, side, state, target_m);
                        }
                    }
                    run_hud(&mut app);
                    let (hook, arms) = aim_snapshot(&mut app);
                    let (camera, camera_at) = camera_of(&mut app);

                    for side in Side::ALL {
                        let Some(point_m) = rope_point(&hook, &arms, side) else {
                            continue;
                        };
                        let Ok(want) = camera.world_to_viewport(&camera_at, point_m) else {
                            continue;
                        };
                        let shape = shape_of(arm_state(&mut app, side));
                        let full_h =
                            shape.glyph_h_px + shape.tether_px.map_or(0.0, |t| 4.0 + t);
                        // A point whose glyph would not fit on the screen is legitimately
                        // clamped inwards (`layout_for` step 2) — that is a courtesy, not a
                        // claim, and it is not what this test is about.
                        let margin = Vec2::new(shape.glyph_w_px + 32.0, full_h + 32.0);
                        if want.x < margin.x
                            || want.y < margin.y
                            || want.x > w - margin.x
                            || want.y > h - margin.y
                        {
                            continue;
                        }

                        let drew = glyph_centre(&mut app, side);
                        let off = (drew - want).length();
                        checked += 1;

                        // Would the honest rectangle have sat on the pixels being cut? The
                        // rectangle is **not** centred on the point: `layout_for` centres the
                        // *glyph* on it and the tether hangs below, so `full_h` runs downwards
                        // from `want.y - glyph_h/2`. Spelling that out is the difference
                        // between an exemption that matches the rule and one that is a guess.
                        let top = want.y - shape.glyph_h_px * 0.5;
                        let over_the_core = (want.x - centre.x).abs()
                            < shape.glyph_w_px * 0.5 + SIGHT_CORE_PX
                            && top < centre.y + SIGHT_CORE_PX
                            && top + full_h > centre.y - SIGHT_CORE_PX;
                        let allowed = if over_the_core {
                            dodged += 1;
                            // The step goes to the NEARER of the two edges of the core, and the
                            // two are `full_h + 2 * SIGHT_CORE_PX` apart, so half of that is
                            // the most it can ever move.
                            full_h * 0.5 + SIGHT_CORE_PX + 1.0
                        } else {
                            strict += 1;
                            1.5
                        };
                        let label = || {
                            format!(
                                "assist {catch_pct:.0}/{strength_pct:.0}, stand {stand:?}, \
                                 yaw {yaw_deg} pitch {pitch_deg}, {state} {side:?}"
                            )
                        };
                        if off > worst.0 {
                            worst = (off, label());
                        }
                        let off_x = (drew.x - want.x).abs();
                        if off_x > x_worst.0 {
                            x_worst = (off_x, label());
                        }
                        assert!(
                            off <= allowed,
                            "assist {catch_pct:.0}/{strength_pct:.0}, standing at {stand:?} \
                             looking yaw {yaw_deg} pitch {pitch_deg}, the {side:?} arm is \
                             {state} on {point_m:?}. That point projects to {want:?} and the \
                             glyph was drawn at {drew:?} — {off:.1} px away, {allowed:.1} px \
                             allowed. The user's requirement is that the marker is exactly \
                             where the rope lands (2026-08-19); a marker parked somewhere \
                             else is the HUD promising a place the shot does not take"
                        );
                    }
                }
            }
        }
    }

    println!(
        "f023 drawn-vs-fired: {checked} samples, {strict} exact, {dodged} out of the sight \
         core, worst {:.1} px ({}), worst x {:.2} px ({})",
        worst.0, worst.1, x_worst.0, x_worst.1
    );
    // The sweep has to have really looked at all four states in both keep-out regimes, or a
    // green run means nothing. 600-ish samples came out of the shipped map on 2026-08-19.
    assert!(
        checked >= 400,
        "only {checked} (state, side) pairs had a rope point on screen at all — the sweep \
         stopped reaching the geometry it was written for"
    );
    // ⚠️ **This used to demand `strict >= 300` and it cannot any more, for a reason that is
    // geometry and not a relaxation.** Until 2026-08-23 the two arms previewed two *different*
    // points — `F-023`'s fan — so at most one of them was ever over the sight core. Both arms
    // now carry the **same** point, and the place a player aims at is the middle of his screen
    // by construction, so the pair dodges together: 164 of 752 exact against 406 of 708 before,
    // with the worst step unchanged at 20.4 px against 21.5 px. The count moved because the two
    // markers coincide, not because anything is drawn further from its rope.
    //
    // What replaces it has more teeth, not less: **the x is exact in every single sample**. The
    // dodge is allowed to move the y and nothing else (the letter hangs outboard and rides with
    // it), and x is the axis `F-024`'s sideways-only sweep puts the whole message on
    // (`docs/FINDINGS.md` FIND-133). `x_worst` is asserted at the pixel below.
    assert!(
        strict > 0,
        "not one of {checked} samples was held to the exact projection — the sweep never left \
         the sight core and the strict branch is untested"
    );
    assert!(
        x_worst.0 <= 1.5,
        "the drawn x left the projected x by {:.2} px ({}) — the sight-core dodge may move the \
         y and nothing else, because x is where the point really is sideways",
        x_worst.0, x_worst.1
    );
    assert!(
        dodged > 0,
        "the sight-core allowance never fired in {checked} samples — it is dead code and the \
         test is not proving that the dodge is bounded"
    );
}

/// **`F-024`, the user's own criterion, measured on the screen: a snap moves the marker
/// SIDEWAYS and by nothing else.**
///
/// > *„die seile sollen immer auf der horzontalen fest sein. also wenn das fadenkreuz 0, 0 ist
/// > sollen die seile nur auf der x achse snappen (objekte finden) also seitlich! dann ist es
/// > auch besser einzuschätzen."* — 2026-08-19
///
/// `tests/vector_hooks.rs::f024_a_published_snap_point_never_sits_above_or_below_the_crosshair_in_the_running_game`
/// proves the same thing about the **point**, in the camera's frame, out of arithmetic. It is
/// not the whole proof: what the player judges is the **pixel**, and the pixel comes from the
/// render camera, which is not the transform `vector::aim` cast its rays from (`aim` runs in
/// `FixedUpdate`, the camera rig one stage later). If those two frames disagree, two points on
/// the aim's row project to two different screen rows and the promise is broken on screen while
/// holding in the world. That gap is exactly where FIND-098, FIND-099 and FIND-129 all lived.
///
/// So this measures the difference the user would see: the same arm, the same stand, the same
/// look, at five assist settings — and the projected **y** of the point the rope flies to may
/// not move. The x may move as much as it likes; that is the feature.
#[test]
fn f024_a_snap_moves_the_marker_sideways_on_the_screen_and_never_up_or_down() {
    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);

    let stands = [
        Vec3::new(51.0, 0.0, 13.0),
        Vec3::new(168.19, 0.0, -50.12),
        Vec3::new(51.0, 60.0, 13.0),
    ];
    let looks: [(f32, f32); 7] = [
        (0.0, 0.0),
        (0.0, -89.0),
        (37.0, -60.0),
        (90.0, 10.0),
        (180.0, -5.0),
        (200.0, 45.0),
        (270.0, -30.0),
    ];
    let assists: [(f32, f32); 5] =
        [(0.0, 0.0), (25.0, 25.0), (50.0, 50.0), (75.0, 75.0), (100.0, 100.0)];

    let mut worst_dy = 0.0_f32;
    let mut worst_dx = 0.0_f32;
    let mut worst_at = String::new();
    let mut compared = 0;
    let mut moved_sideways = 0;

    for stand in stands {
        for (yaw_deg, pitch_deg) in looks {
            // Column per arm: the projected point at each assist setting, or `None`.
            let mut seen: [Vec<(f32, Vec2)>; 2] = [Vec::new(), Vec::new()];
            for (catch_pct, strength_pct) in assists {
                set_assist(&mut app, catch_pct, strength_pct);
                stand_and_look(&mut app, stand, yaw_deg, pitch_deg);
                // ⚠️ **At rest, and the fixture has to say so.** `vector::aim` casts from the
                // eye *before* avian integrates and the camera renders from the eye *after*,
                // so a moving player carries one step of parallax between the ray and the
                // picture — 0.5 m at 30 m/s. That parallax is distance-dependent, so it moves
                // a near point and a far point by different amounts and would show up here as
                // a vertical jump the snap did not cause. Left uncontrolled it was 8.6 px, and
                // all of it was a 60 m stand in free fall accelerating across the sweep.
                // The parallax is real and it is `docs/FINDINGS.md` FIND-133's second half;
                // it is not the axis this test is about.
                let me = local_player(&mut app);
                app.world_mut()
                    .entity_mut(me)
                    .insert(avian3d::prelude::LinearVelocity(Vec3::ZERO));
                sim_step(&mut app);
                // Put the eye back exactly where `vector::aim` cast from, before the camera is
                // asked where the point projects. Without it the two are one integration step
                // apart and the difference is **distance-dependent**, so it moves a near point
                // and a far point by different amounts and reads as a vertical jump the snap did
                // not cause: 8.6 px uncontrolled (a 60 m stand in free fall), 1.4 px with the
                // velocity zeroed but the step still taken. The parallax is real in the running
                // game and it is `docs/FINDINGS.md` FIND-133's last section; it is not the axis
                // this test is about, and a guard that measures both cannot tell them apart.
                stand_and_look(&mut app, stand, yaw_deg, pitch_deg);
                run_hud(&mut app);
                let (hook, arms) = aim_snapshot(&mut app);
                let (camera, camera_at) = camera_of(&mut app);
                for side in Side::ALL {
                    let Some(point_m) = rope_point(&hook, &arms, side) else { continue };
                    let Ok(at) = camera.world_to_viewport(&camera_at, point_m) else { continue };
                    // Off-screen projections carry no readable row.
                    if at.x < 0.0 || at.y < 0.0 || at.x > w || at.y > h {
                        continue;
                    }
                    seen[side.index()].push((catch_pct, at));
                }
            }
            for side in Side::ALL {
                let column = &seen[side.index()];
                let Some((_, base)) = column.first().copied() else { continue };
                for (catch_pct, at) in column.iter().skip(1) {
                    compared += 1;
                    let dy = (at.y - base.y).abs();
                    let dx = (at.x - base.x).abs();
                    if dx > 1.0 {
                        moved_sideways += 1;
                    }
                    if dy > worst_dy {
                        worst_dy = dy;
                        worst_dx = dx;
                        worst_at = format!(
                            "stand {stand:?} yaw {yaw_deg} pitch {pitch_deg} {side:?} at \
                             {catch_pct:.0} %: free {base:?} -> snapped {at:?}"
                        );
                    }
                }
            }
        }
    }

    println!(
        "F-024 on the screen: {compared} free-vs-snapped pairs, {moved_sideways} of them moved \
         the marker sideways by more than 1 px; worst VERTICAL movement {worst_dy:.3} px \
         (with {worst_dx:.1} px of sideways at the same sample) — {worst_at}"
    );
    assert!(compared > 50, "only {compared} pairs — the sweep found almost nothing to aim at");
    assert!(
        moved_sideways > 0,
        "no snap moved any marker at all, so this test proves nothing about the axis"
    );
    assert!(
        worst_dy <= 1.0,
        "a snap moved the marker {worst_dy:.2} px UP OR DOWN on the screen — {worst_at}. The \
         user asked for the search to be locked to the horizontal so that it can be judged; a \
         vertical jump is exactly what he asked to be rid of"
    );
}

// ---------------------------------------------------------------------------------------
// F-016 — the search band: from where to where the assist is looking
//
// > *„es soll in der ui angezeigt werden von wo bis wo gesearched wird damit man das besser
// > einstellen kann!"* — the user, 2026-08-19
//
// The last clause is the acceptance criterion: the band exists to be READ WHILE THE NUMBER IS
// BEING CHANGED. So the five tests below ask, in order: does every mark stand on a ray the
// search really casts · does the extent match an angle this file projects with its own
// arithmetic · is it gone when no search is running · does it answer the knob in the tick it
// moves · and does it leave the pixels the player is cutting alone.
// ---------------------------------------------------------------------------------------

/// The drawn centre of every band tick, by `(side index, step)`. A tick that is not drawn is
/// simply absent, which is what three of the tests below are actually about.
fn band_ticks(app: &mut App) -> std::collections::BTreeMap<(usize, u32), Vec2> {
    use defeated_by_titan::hud::catch_band::CatchTick;
    let mut out = std::collections::BTreeMap::new();
    let mut q = app
        .world_mut()
        .query::<(&CatchTick, &Node, &ComputedNode, &UiGlobalTransform)>();
    for (tick, node, computed, at) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        let (min_x, min_y, max_x, max_y) = rect(computed, at);
        out.insert(
            (tick.side.index(), tick.step),
            Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5),
        );
    }
    out
}

/// The drawn rectangle of one span rule, or `None` when it is not drawn.
fn band_rule(app: &mut App, side: Side) -> Option<(f32, f32, f32, f32)> {
    use defeated_by_titan::hud::catch_band::CatchRule;
    let mut q = app
        .world_mut()
        .query::<(&CatchRule, &Node, &ComputedNode, &UiGlobalTransform)>();
    for (rule, node, computed, at) in q.iter(app.world()) {
        if rule.0 != side || node.display == Display::None {
            continue;
        }
        return Some(rect(computed, at));
    }
    None
}

/// Every drawn band node's rectangle, ticks and rules together.
fn band_rects(app: &mut App) -> Vec<(String, (f32, f32, f32, f32))> {
    use defeated_by_titan::hud::catch_band::{CatchRule, CatchTick};
    let mut out = Vec::new();
    let mut q = app
        .world_mut()
        .query_filtered::<(&Name, &Node, &ComputedNode, &UiGlobalTransform), Or<(With<CatchTick>, With<CatchRule>)>>();
    for (name, node, computed, at) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        out.push((name.to_string(), rect(computed, at)));
    }
    out
}

/// The crosshair's own pixel, out of the render camera — not `viewport / 2`.
fn crosshair_px(app: &mut App) -> Vec2 {
    let (camera, at) = camera_of(app);
    camera
        .world_to_viewport(&at, at.translation() + *at.forward())
        .expect("the direction the camera is looking projects onto its own viewport")
}

fn probe_steps(app: &App) -> u32 {
    app.world().resource::<GameData>().game.vector.assist_probe_steps
}

/// The catch half-width in radians — through [`PlayerSettings::assist_catch_deg`], the same
/// accessor `vector::aim` fills `ScoreContext::catch_rad` with. There is one mapping from
/// percent to degrees in this repository and this test does not add a second.
fn catch_rad(app: &App) -> f32 {
    app.world().resource::<PlayerSettings>().assist_catch_deg().to_radians()
}

/// **The load-bearing one: every mark of the band stands on a ray the sweep really casts.**
///
/// ⚠️ `docs/FINDINGS.md` FIND-103 — *a test that asks the screen and the function the same
/// question passes when both are wrong.* It is not that here, and the reason is that `hud`
/// **cannot** call the sweep: there is no `hud -> vector` line on the allow list in
/// `docs/architecture.md`, so `hud::catch_band::probe_theta_rad` is a second spelling of
/// `vector::aim::probe_dirs`' own `theta`, exactly as `hud::crosshair::eye` is a second
/// spelling of `vector::aim::eye`. This test is what pins the two together: it takes the
/// `Vec3` directions `probe_dirs` **itself returns**, projects them, and asserts the laid-out
/// rectangles stand on them. The day the sweep changes shape, this goes red before the picture
/// starts lying — which is the whole failure mode of FIND-098, FIND-099 and FIND-129.
#[test]
fn f016_the_band_stands_on_the_probe_rays_the_search_really_casts() {
    use defeated_by_titan::hud::arm_aim::SIGHT_CORE_PX;
    use defeated_by_titan::shared::Intent;
    use defeated_by_titan::vector::aim::{look_basis, probe_dirs};

    let mut app = app();
    attach_screen(&mut app);
    let steps = probe_steps(&app);

    // Six look angles including two steep ones, because "horizontal" is the CAMERA's
    // horizontal at every pitch (FIND-133) — a band built out of a world axis would pass at
    // pitch 0 and fail at −85°.
    let looks: [(f32, f32); 6] =
        [(0.0, 0.0), (37.0, -60.0), (90.0, 10.0), (180.0, -5.0), (270.0, 45.0), (0.0, -85.0)];
    // The slider's own step is 5 %, so the narrowest setting a player can dial is in here.
    let catches = [5.0, 25.0, 40.0, 75.0, 100.0];

    let mut checked = 0;
    let mut dropped = 0;
    let mut worst = 0.0_f32;
    let mut worst_at = String::new();

    for (yaw_deg, pitch_deg) in looks {
        for catch_pct in catches {
            set_assist(&mut app, catch_pct, 100.0);
            stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), yaw_deg, pitch_deg);
            run_hud(&mut app);

            let drawn = band_ticks(&mut app);
            let centre = crosshair_px(&mut app);
            let (camera, camera_at) = camera_of(&mut app);
            let player = local_player(&mut app);
            let intent = *app
                .world()
                .entity(player)
                .get::<Intent>()
                .expect("the player carries an `Intent`");
            let basis = look_basis(&intent);
            let rad = catch_rad(&app);

            for side in Side::ALL {
                for (i, dir) in probe_dirs(basis, rad, steps, side).enumerate() {
                    let want = camera
                        .world_to_viewport(&camera_at, camera_at.translation() + dir)
                        .expect("a probe inside the catch projects onto the viewport");
                    let key = (side.index(), i as u32);
                    // The one thing the band gives up is the sight core, and it gives it up by
                    // NOT DRAWING — never by moving a mark somewhere else.
                    if (want.x - centre.x).abs() < SIGHT_CORE_PX {
                        dropped += 1;
                        assert!(
                            !drawn.contains_key(&key),
                            "the {side:?} tick {i} at {want:?} is inside the {SIGHT_CORE_PX} px \
                             sight core around {centre:?} and was drawn anyway"
                        );
                        continue;
                    }
                    let drew = drawn.get(&key).copied().unwrap_or_else(|| {
                        panic!(
                            "catch {catch_pct} %, yaw {yaw_deg} pitch {pitch_deg}: the sweep \
                             casts a {side:?} probe {i} that projects to {want:?}, and no band \
                             tick is drawn for it. The user asked to see from where to where \
                             the search runs; a missing ray is a search he cannot see"
                        )
                    });
                    checked += 1;
                    let off = (drew - want).length();
                    if off > worst {
                        worst = off;
                        worst_at = format!(
                            "catch {catch_pct} %, yaw {yaw_deg} pitch {pitch_deg}, {side:?} \
                             probe {i}: ray projects to {want:?}, tick drawn at {drew:?}"
                        );
                    }
                    assert!(
                        off <= 1.0,
                        "{worst_at} — {off:.2} px apart. The band has to stand on the rays the \
                         search casts, or it is a picture of a search and not the search"
                    );
                }
            }
        }
    }

    println!(
        "f016 band-vs-sweep: {checked} probe rays compared against their tick, {dropped} \
         dropped into the sight core, worst {worst:.3} px ({worst_at})"
    );
    assert!(
        checked >= 400,
        "only {checked} rays were compared — the sweep or the band stopped producing marks"
    );
}

/// **The extent, against arithmetic this file does itself.**
///
/// The test above compares the band to `probe_dirs`; this one compares it to a pinhole camera
/// worked out by hand from `fov_deg` and the aspect ratio, with nothing of the game's in it.
/// Two independent checks, because a band that agreed with `probe_dirs` and disagreed with the
/// lens would still be in the wrong place on the screen.
///
/// It also prints the number the whole feature is for: **how wide the band is at 0 / 40 /
/// 100 %.**
#[test]
fn f016_the_band_ends_where_the_catch_angle_says_in_hand_projected_pixels() {
    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    let steps = probe_steps(&app);
    let fov_deg = app.world().resource::<PlayerSettings>().fov_deg;
    // `PerspectiveProjection.fov` is the VERTICAL field of view (Q-021), so the horizontal
    // half-angle comes out of it through the aspect ratio and nothing else.
    let tan_half_h = (fov_deg * 0.5).to_radians().tan() * (w / h);

    for catch_pct in [0.0_f32, 5.0, 40.0, 100.0] {
        set_assist(&mut app, catch_pct, 100.0);
        stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), 0.0, 0.0);
        run_hud(&mut app);

        let ticks = band_ticks(&mut app);
        let left = ticks.get(&(Side::Left.index(), steps - 1)).copied();
        let right = ticks.get(&(Side::Right.index(), steps - 1)).copied();
        if catch_pct == 0.0 {
            assert!(
                left.is_none() && right.is_none(),
                "at 0 % no search runs and there is nothing to draw the extent of, but the end \
                 marks are at {left:?} / {right:?}"
            );
            println!("f016 band width at   0 %: no band");
            continue;
        }

        let (left, right) = (
            left.expect("the left end mark has to be drawn while a search runs"),
            right.expect("the right end mark has to be drawn while a search runs"),
        );
        let centre = crosshair_px(&mut app);
        // The pinhole, by hand: x_ndc = tan(theta) / (tan(fov/2) * aspect).
        let want_dx = (w * 0.5) * catch_rad(&app).tan() / tan_half_h;
        println!(
            "f016 band width at {catch_pct:3.0} %: {:.1} px half-width ({:.1} px end to end), \
             hand-projected {want_dx:.1} px",
            (right.x - left.x) * 0.5,
            right.x - left.x
        );
        for (side, at) in [("left", left), ("right", right)] {
            let dx = (at.x - centre.x).abs();
            assert!(
                (dx - want_dx).abs() <= 1.0,
                "at {catch_pct} % the {side} end mark stands {dx:.1} px from the crosshair, and \
                 {:.2}° through a {fov_deg}° lens on a {w} x {h} screen is {want_dx:.1} px. The \
                 band would be telling him a different number than the one he is setting",
                catch_rad(&app).to_degrees()
            );
            assert!(
                (at.y - centre.y).abs() <= 1.0,
                "the {side} end mark is {:.1} px off the crosshair's own row. The search is a \
                 LINE (FIND-133, 0.000006° of vertical deviation); a band that is not level is \
                 drawing a shape the sweep does not have",
                (at.y - centre.y).abs()
            );
        }
    }
}

/// **No reach, no band** — and above 0 % reach there is always one, however the second knob
/// stands.
///
/// `Q-042`, 2026-08-20: the gate used to be [`PlayerSettings::assist_is_on`] —
/// `catch > 0 && strength > 0` — and **both ship at 0**, so the element existed for a moment it
/// was never present in: the player opens `Settings`, turns *Aim assist reach* up, and sees
/// nothing at all. The picture is the **reach's** picture, so the reach is what draws it.
///
/// ⚠️ **The probe is still `assist_is_on`'s and nothing here touches that.** Drawing and
/// searching are two predicates now, deliberately, and the difference is said out loud in the
/// band's colour —
/// `f016_the_reach_alone_draws_the_band_and_the_colour_says_whether_it_searches`.
/// `tests/vector_hooks.rs::f016_at_zero_percent_the_aim_is_bit_for_bit_the_one_the_game_had_before`
/// is the invariant that would have caught it if it had.
#[test]
fn f016_there_is_no_band_when_there_is_no_reach() {
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), 0.0, 0.0);

    for (catch_pct, strength_pct) in [(0.0, 0.0), (0.0, 100.0)] {
        set_assist(&mut app, catch_pct, strength_pct);
        run_hud(&mut app);
        let drawn = band_rects(&mut app);
        assert!(
            drawn.is_empty(),
            "reach {catch_pct} % is free aim — there is no extent, and {} band nodes are on \
             screen: {:?}",
            drawn.len(),
            drawn.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
        );
    }

    for (catch_pct, strength_pct) in [(100.0, 0.0), (100.0, 100.0)] {
        set_assist(&mut app, catch_pct, strength_pct);
        run_hud(&mut app);
        let drawn = band_rects(&mut app);
        assert_eq!(
            drawn.len(),
            2 * probe_steps(&app) as usize + 2,
            "reach {catch_pct} % / strength {strength_pct} %: every probe and both span rules \
             have to be drawn — the reach row is the row that draws this picture, and it may \
             not need a second row's permission"
        );
    }
}

/// ★ **`Q-042` — the reach draws the band on its own, and the colour says whether anything is
/// actually searching.**
///
/// The user asked for the band for one stated reason: *„es soll in der ui angezeigt werden von
/// wo bis wo gesearched wird **damit man das besser einstellen kann**!"* A band that appears
/// only once a second, differently-named knob is also non-zero fails that sentence — and
/// `assist_strength_pct` ships at 0, so it failed it on every fresh run.
///
/// ⚠️ **Drawing and searching are two predicates and this test is what keeps that honest.**
/// The geometry is identical in both states — the same ticks on the same rays, tested to
/// 0.691 px by `f016_the_band_stands_on_the_probe_rays_the_search_really_casts` — because the
/// geometry answers *"how far does the reach go"*, which is true whether or not a probe is in
/// flight. The one thing that differs is the colour, which is the only claim the two states
/// make differently: **is a ray being cast right now.** That is why this is not FIND-098 /
/// FIND-099 / FIND-127 / FIND-129 with a new shape: nothing is drawn in a place the thing is
/// not, and the state that would be a lie if it were silent is not silent.
///
/// Both colours have to clear WCAG 1.4.11's 3:1 over the settings backdrop in its worst case
/// (FIND-136 §2), because the band is a ruler in **both** states — and with `F-025` unbuilt the
/// idle one is the only state a player can be in today.
#[test]
fn f016_the_reach_alone_draws_the_band_and_the_colour_says_whether_it_searches() {
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), 0.0, 0.0);

    set_assist(&mut app, 40.0, 100.0);
    run_hud(&mut app);
    let searching_ticks = band_ticks(&mut app);
    let searching_rules = [band_rule(&mut app, Side::Left), band_rule(&mut app, Side::Right)];
    let searching_colour = one_band_colour(&mut app);

    set_assist(&mut app, 40.0, 0.0);
    run_hud(&mut app);
    let idle_ticks = band_ticks(&mut app);
    let idle_rules = [band_rule(&mut app, Side::Left), band_rule(&mut app, Side::Right)];
    let idle_colour = one_band_colour(&mut app);

    assert!(
        !idle_ticks.is_empty(),
        "reach 40 % with the strength knob at 0 draws nothing — that is `Q-042`: the player \
         turns the row he was told to turn and the feature he asked for stays absent"
    );
    assert_eq!(
        idle_ticks, searching_ticks,
        "the band's geometry is the REACH and nothing else — the same ticks on the same probe \
         rays. A band that moved when the strength knob moved would be drawing a second number"
    );
    assert_eq!(idle_rules, searching_rules, "and the span rules with them");

    let searching_colour = searching_colour.expect("the band is drawn at 40 % / 100 %");
    let idle_colour = idle_colour.expect("the band is drawn at 40 % / 0 %");
    assert_ne!(
        idle_colour, searching_colour,
        "no probe ray is cast at 0 % strength (`PlayerSettings::assist_is_on`), so a band drawn \
         exactly like the live one would be claiming a search that is not running"
    );
    for (what, colour) in [("searching", searching_colour), ("idle", idle_colour)] {
        for world in [0.0_f32, 1.0] {
            let ratio = band_contrast_over_the_backdrop(colour, world);
            assert!(
                ratio >= 3.0,
                "the {what} band is {ratio:.2}:1 over the settings backdrop on a world at \
                 luminance {world} — under WCAG 1.4.11's 3:1 it is not a ruler he can read \
                 (FIND-136 §2)"
            );
        }
    }
    println!(
        "BAND Q-042: searching {:.2}:1 · idle {:.2}:1 over the backdrop on white; the two \
         states differ by {:.2}:1",
        band_contrast_over_the_backdrop(searching_colour, 1.0),
        band_contrast_over_the_backdrop(idle_colour, 1.0),
        state_contrast(searching_colour, idle_colour, 1.0),
    );
}

/// The one colour every drawn band node carries, or `None` when nothing is drawn — and it
/// falls over if the nodes disagree, because a band in two colours says two things.
fn one_band_colour(app: &mut App) -> Option<Color> {
    use defeated_by_titan::hud::catch_band::{CatchRule, CatchTick};
    let mut out: Option<Color> = None;
    let mut q = app
        .world_mut()
        .query_filtered::<(&Name, &Node, &BackgroundColor), Or<(With<CatchTick>, With<CatchRule>)>>(
        );
    for (name, node, colour) in q.iter(app.world()) {
        if node.display == Display::None {
            continue;
        }
        match out {
            None => out = Some(colour.0),
            Some(seen) => assert_eq!(
                seen, colour.0,
                "{name} is a different colour from the rest of the band"
            ),
        }
    }
    out
}

/// Relative luminance of a colour in linear light — the arithmetic FIND-093 and FIND-136 use.
fn linear_luminance(c: Color) -> f32 {
    let l = c.to_linear();
    0.2126 * l.red + 0.7152 * l.green + 0.0722 * l.blue
}

/// What one band colour reads at over `menu::plate::BACKDROP` over a world frame of luminance
/// `world` — 0.0 is black, 1.0 is the worst case the game can put behind the menu.
fn band_contrast_over_the_backdrop(colour: Color, world: f32) -> f32 {
    use defeated_by_titan::menu::plate::BACKDROP;
    let a = BACKDROP.to_linear().alpha;
    let back = a * linear_luminance(BACKDROP) + (1.0 - a) * world;
    let a = colour.to_linear().alpha;
    let front = a * linear_luminance(colour) + (1.0 - a) * back;
    (front.max(back) + 0.05) / (front.min(back) + 0.05)
}

/// And what the two states read at against **each other**, over the same background.
fn state_contrast(one: Color, other: Color, world: f32) -> f32 {
    use defeated_by_titan::menu::plate::BACKDROP;
    let a = BACKDROP.to_linear().alpha;
    let back = a * linear_luminance(BACKDROP) + (1.0 - a) * world;
    let over = |c: Color| {
        let a = Color::to_linear(&c).alpha;
        a * linear_luminance(c) + (1.0 - a) * back
    };
    let (x, y) = (over(one), over(other));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// **It answers the knob in the tick the knob moves** — no restart, no respawn.
///
/// That is the requirement, not a nicety: the element exists so the number can be set by eye,
/// and a band that needed a reload would be read against the *previous* setting. The knob is
/// moved here exactly the way `menu`'s slider and `debug`'s `settings assist_catch <n>` move
/// it — by writing `PlayerSettings` — and the band is measured after one HUD pass.
#[test]
fn f016_the_band_answers_the_knob_in_the_tick_it_moves() {
    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), 0.0, 0.0);
    let steps = probe_steps(&app);

    let half_width = |app: &mut App, catch_pct: f32| -> f32 {
        set_assist(app, catch_pct, 100.0);
        run_hud(app);
        let ticks = band_ticks(app);
        let centre = crosshair_px(app);
        ticks
            .get(&(Side::Right.index(), steps - 1))
            .unwrap_or_else(|| panic!("no end mark at {catch_pct} %"))
            .x
            - centre.x
    };

    let narrow = half_width(&mut app, 20.0);
    let wide = half_width(&mut app, 100.0);
    let back = half_width(&mut app, 20.0);

    // The angles are 4° and 20°, and a pinhole maps them by their tangents — so the ratio is
    // fixed arithmetic and not a "it got bigger" that a stuck band could also pass.
    let want = 20.0_f32.to_radians().tan() / 4.0_f32.to_radians().tan();
    let got = wide / narrow;
    println!(
        "f016 knob: 20 % -> {narrow:.1} px, 100 % -> {wide:.1} px, back to 20 % -> {back:.1} px \
         (ratio {got:.3}, tan 20°/tan 4° = {want:.3})"
    );
    assert!(
        (got - want).abs() <= 0.05,
        "moving the knob from 20 % to 100 % took the band from {narrow:.1} px to {wide:.1} px, \
         a ratio of {got:.3} where the two angles say {want:.3}"
    );
    assert!(
        (back - narrow).abs() <= 0.5,
        "coming back to 20 % left the band at {back:.1} px instead of the {narrow:.1} px it \
         stood at before — the band is remembering a setting instead of reading one"
    );
}

/// **The band is level with the crosshair, so it runs through the one region `F-170`
/// protects.** This is the second documented exemption from the keep-out box, and what it
/// gives up instead is [`SIGHT_CORE_PX`] — the pixels the player is cutting.
///
/// The exemption is FIND-098's own argument and no new one: the band's position IS an angle,
/// its whole range lives inside the box (the box's edge is 128 px from centre and the band
/// reaches 227 px at 100 % but only 88 px at 40 %), so pushing it out would draw a band wider
/// than the search for every setting below about 55 %. That is FIND-129's lie with a different
/// number on it.
#[test]
fn f016_the_band_keeps_the_sight_core_clear() {
    use defeated_by_titan::hud::arm_aim::SIGHT_CORE_PX;
    let mut app = app();
    attach_screen(&mut app);
    let steps = probe_steps(&app);
    let mut seen = 0;

    for catch_pct in [5.0_f32, 20.0, 40.0, 60.0, 80.0, 100.0] {
        for (yaw_deg, pitch_deg) in [(0.0, 0.0), (123.0, -70.0)] {
            set_assist(&mut app, catch_pct, 100.0);
            stand_and_look(&mut app, Vec3::new(51.0, 0.0, 13.0), yaw_deg, pitch_deg);
            run_hud(&mut app);
            let centre = crosshair_px(&mut app);
            for (name, (min_x, min_y, max_x, max_y)) in band_rects(&mut app) {
                seen += 1;
                let covers = min_x < centre.x + SIGHT_CORE_PX
                    && max_x > centre.x - SIGHT_CORE_PX
                    && min_y < centre.y + SIGHT_CORE_PX
                    && max_y > centre.y - SIGHT_CORE_PX;
                assert!(
                    !covers,
                    "at catch {catch_pct} % `{name}` stands at ({min_x:.1}, {min_y:.1})..\
                     ({max_x:.1}, {max_y:.1}) and covers the {SIGHT_CORE_PX} px sight core at \
                     {centre:?} — the pixels the player is aiming with"
                );
            }
            // And the span rule really spans: from the edge of the core out to the end mark,
            // on both sides. A band whose rule stopped short would read as a narrower search
            // than the ticks say, and the two would be telling him different numbers.
            let ticks = band_ticks(&mut app);
            for side in Side::ALL {
                let (min_x, _, max_x, _) = band_rule(&mut app, side)
                    .unwrap_or_else(|| panic!("at catch {catch_pct} % the {side:?} span rule is not drawn"));
                let end = ticks
                    .get(&(side.index(), steps - 1))
                    .unwrap_or_else(|| panic!("at catch {catch_pct} % the {side:?} end mark is not drawn"))
                    .x;
                let (outer, inner) = match side {
                    Side::Left => (min_x, max_x),
                    Side::Right => (max_x, min_x),
                };
                assert!(
                    (outer - end).abs() <= 1.5,
                    "at catch {catch_pct} % the {side:?} span rule stops at {outer:.1} px and                      its end mark stands at {end:.1} px — the rule and the ticks are drawing                      two different searches"
                );
                assert!(
                    (inner - centre.x).abs() <= SIGHT_CORE_PX + 1.5,
                    "at catch {catch_pct} % the {side:?} span rule starts {:.1} px from the                      crosshair, and all it is allowed to give up is the {SIGHT_CORE_PX} px                      sight core",
                    (inner - centre.x).abs()
                );
            }
            // And the mark that carries the number he is tuning is never the one dropped.
            for side in Side::ALL {
                assert!(
                    ticks.contains_key(&(side.index(), steps - 1)),
                    "at catch {catch_pct} % the {side:?} END MARK is not drawn. The narrowest \
                     the slider can dial is 5 % = 1°, which projects 10.9 px out on a 1280 px \
                     screen — past the {SIGHT_CORE_PX} px core. A dropped end mark means the \
                     band has stopped showing the extent at all"
                );
            }
        }
    }
    assert!(seen >= 100, "only {seen} band nodes were looked at over the whole slider");
}

// ---------------------------------------------------------------------------------------
// F-026 + F-027 + F-030a — the anchor field on screen
//
// **What these tests exist against has a name and a number.** `docs/FINDINGS.md` FIND-160:
// 1520 authored `hook.*` points loaded, and `grep` for a reader of `AnchorField` outside
// `src/world/` came back empty — a whole anchor system with no caller. The first failure mode
// of giving it one is therefore not "the marker is wrong", it is **"the marker is not there and
// the test passed anyway"** (FIND-152). So every test below counts what it looked at and falls
// over on a count that is too small, and the two that matter most compare the drawn rectangle
// against a world point projected by hand — the only comparison that can see FIND-129's class
// of lie.
// ---------------------------------------------------------------------------------------

/// Forces the 10 Hz set-refresh, so that "the set was searched again after I moved" is a fact
/// and not a coincidence.
///
/// ⚠️ **It is needed for determinism, not because time stands still.** `First` never runs in
/// these tests, so `Time`'s delta is frozen at whatever the last real `app.update()` left in it
/// — which is a good fraction of a second while the world is being built, i.e. usually *more*
/// than the 10 Hz period. Whether a plain `run_hud` refreshes the set therefore depends on the
/// machine's mood, which is exactly what a test may not depend on. Where a test needs the
/// opposite — the set **held** across a frame — the delta has to be set to zero explicitly, see
/// `f030a_the_set_refreshes_at_ten_hertz_and_the_pixel_every_frame`.
fn force_mark_refresh(app: &mut App) {
    use defeated_by_titan::hud::anchor_marks::MarkClock;
    let mut clock = app.world_mut().resource_mut::<MarkClock>();
    clock.since_s = 10.0;
}

/// Every anchor mark that is really laid out: `(slot, state, world point, screen rect)`.
fn anchor_marks(
    app: &mut App,
) -> Vec<(usize, defeated_by_titan::hud::anchor_marks::MarkState, Vec3, (f32, f32, f32, f32))> {
    use defeated_by_titan::hud::anchor_marks::{AnchorMark, MarkAt, MarkState};
    let mut out = Vec::new();
    let mut q = app
        .world_mut()
        .query::<(&AnchorMark, &MarkState, &MarkAt, &Node, &ComputedNode, &UiGlobalTransform)>();
    for (mark, state, at, node, computed, ui_at) in q.iter(app.world()) {
        if node.display == Display::None || *state == MarkState::Off {
            continue;
        }
        let Some(point_m) = at.point_m else { continue };
        out.push((mark.slot, *state, point_m, rect(computed, ui_at)));
    }
    out.sort_by_key(|(slot, ..)| *slot);
    out
}

/// Puts the player on a roof in the middle of the district and looks somewhere — the same
/// helper the arm-marker sweep uses, plus the refresh the field needs.
fn stand_look_and_mark(app: &mut App, at_m: Vec3, yaw_deg: f32, pitch_deg: f32) {
    stand_and_look(app, at_m, yaw_deg, pitch_deg);
    force_mark_refresh(app);
    run_hud(app);
}

#[test]
fn f026_the_drawn_states_differ_in_form_not_only_in_colour() {
    // `F-026` verbatim: *„Vier Zustaende, jeweils durch FORM UND Farbe unterschieden
    // (Farbenblindheit)"*. A table with two rows that read the same is a table that explains
    // nothing, and it is exactly what a colour-blind player is left holding.
    //
    // ⚠️ **Two rows, where the spec names four.** `BestLeft` and `BestRight` were withdrawn on
    // 2026-08-26 because they carried `Q` and `E` for a key that does not obey them (`F-024`
    // unbuilt — `src/hud/anchor_marks.rs` header, `docs/BUGS.md` B-011). The parenthesis the
    // spec cares about is *still* satisfied, and with more room than before: the two survivors
    // differ in size, in filled-vs-hollow AND in the tether, so three independent
    // non-colour channels tell them apart.
    use defeated_by_titan::hud::anchor_marks::{form_of, MarkState};

    for a in MarkState::DRAWN {
        assert!(form_of(a).size_px > 0.0, "{a:?} has no size — it cannot be seen at all");
        for b in MarkState::DRAWN {
            if a == b {
                continue;
            }
            assert_ne!(
                form_of(a),
                form_of(b),
                "{a:?} and {b:?} are drawn as the same rectangle. Then the only thing telling \
                 them apart is colour, and `F-026`'s parenthesis says that is not allowed"
            );
        }
    }
    // And the differences are the ones `F-026` names, not two arbitrary sizes: one small
    // hollow ring, one FILLED symbol with a rope connection.
    assert_eq!(MarkState::DRAWN.len(), 2, "the drawn states are the candidate and the rope");
    assert!(form_of(MarkState::Anchored).tether, "`F-026`: „gefuelltes Symbol mit Seilverbindung\"");
    assert_eq!(
        form_of(MarkState::Anchored).border_px,
        0.0,
        "the anchored symbol is FILLED — a border makes it a ring like the other three"
    );
    assert!(form_of(MarkState::Candidate).border_px > 0.0, "`F-026`: „Kandidat (kleiner Ring)\"");
    assert!(
        !form_of(MarkState::Candidate).tether,
        "only the rope has a rope connection — a candidate with a tether claims a rope"
    );
}

#[test]
fn f026_every_anchor_mark_stands_on_its_own_projected_pixel() {
    // ★ **The promise the user asked for twice, for the new element.** FIND-098, FIND-099,
    // FIND-127 and FIND-129 were four separate cases of a HUD drawing a thing where the thing
    // is not. `arm_aim` allows itself 20 px of vertical dodge around the sight core; this
    // element allows **zero** — a mark stands on its projection or it is not drawn.
    //
    // The comparison straddles the layout, which is the only place the lie can live: the
    // world point comes off the mark's own `MarkAt`, it is projected here by hand through the
    // camera, and it is compared against the rectangle `ComputedNode` + `UiGlobalTransform`
    // actually laid out.
    let mut app = app();
    attach_screen(&mut app);

    let stands = [Vec3::new(0.0, 12.0, 0.0), Vec3::new(40.0, 30.0, -25.0), Vec3::new(-60.0, 8.0, 55.0)];
    let looks = [(0.0_f32, 0.0_f32), (90.0, -10.0), (200.0, 20.0), (300.0, -30.0)];

    let mut checked = 0;
    let mut worst = 0.0_f32;
    for stand in stands {
        for (yaw_deg, pitch_deg) in looks {
            stand_look_and_mark(&mut app, stand, yaw_deg, pitch_deg);
            let (camera, camera_at) = camera_of(&mut app);
            for (slot, state, point_m, (min_x, min_y, max_x, max_y)) in anchor_marks(&mut app) {
                let want = camera
                    .world_to_viewport(&camera_at, point_m)
                    .expect("a mark is only drawn for a point the camera can project");
                let drew = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
                let off = (drew - want).abs();
                worst = worst.max(off.max_element());
                checked += 1;
                // **Half a pixel per axis, and that is the UI's rounding and not this
                // element's.** `place_anchor_marks` writes `Camera::world_to_viewport`'s answer
                // unmodified; taffy then rounds every laid-out edge to a whole pixel, and every
                // mark's width is already an integer, so the centre can only be off by the
                // rounding of its left/top edge. Measured worst case over the sweep: 0.5 px.
                // Anything above that is the element moving a mark, which it may not do.
                assert!(
                    off.x <= 0.51 && off.y <= 0.51,
                    "slot {slot} ({state:?}) marks {point_m:?}, which projects to {want:?}, and \
                     the ring was laid out centred on {drew:?} — {off:?} px away, and the UI's \
                     own rounding can only account for 0.5. This element has no licence to move \
                     a mark at all (FIND-129); if it cannot be drawn on its own pixel it may \
                     only not be drawn"
                );
            }
        }
    }
    assert!(
        checked >= 24,
        "only {checked} marks were looked at over 12 stand/look pairs — the field found almost \
         nothing and this test just measured an empty screen (FIND-152)"
    );
    println!("f026 projection: {checked} marks checked, worst offset {worst:.3} px");
}

#[test]
fn f026_no_anchor_mark_ever_touches_the_sight_core() {
    // `F-170`'s box, resolved the way `src/hud/mod.rs` says: a mark carries a *place*, so it is
    // held out of `arm_aim::SIGHT_CORE_PX` — the pixels the player is cutting — and it gives
    // that up by **standing down**, never by stepping aside. So the rectangle of every drawn
    // mark has to miss the core, and the check below is the whole reason the element is allowed
    // inside the 20 % box at all.
    use defeated_by_titan::hud::arm_aim::SIGHT_CORE_PX;
    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    let core = Vec2::new(w * 0.5, h * 0.5);

    let mut checked = 0;
    let mut nearest = f32::INFINITY;
    for yaw_deg in [0.0_f32, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
        for pitch_deg in [-25.0_f32, -5.0, 10.0] {
            stand_look_and_mark(&mut app, Vec3::new(0.0, 12.0, 0.0), yaw_deg, pitch_deg);
            for (slot, state, _, (min_x, min_y, max_x, max_y)) in anchor_marks(&mut app) {
                checked += 1;
                let over = min_x < core.x + SIGHT_CORE_PX
                    && max_x > core.x - SIGHT_CORE_PX
                    && min_y < core.y + SIGHT_CORE_PX
                    && max_y > core.y - SIGHT_CORE_PX;
                let dx = (core.x - (min_x + max_x) * 0.5).abs() - (max_x - min_x) * 0.5;
                let dy = (core.y - (min_y + max_y) * 0.5).abs() - (max_y - min_y) * 0.5;
                nearest = nearest.min(dx.max(dy));
                assert!(
                    !over,
                    "at yaw {yaw_deg} pitch {pitch_deg}, slot {slot} ({state:?}) covers the \
                     sight core: ({min_x:.1}, {min_y:.1})..({max_x:.1}, {max_y:.1}) against a \
                     core of {SIGHT_CORE_PX} px around {core:?}"
                );
            }
        }
    }
    assert!(checked >= 48, "only {checked} marks were looked at over 24 looks (FIND-152)");
    println!("f026 sight core: {checked} marks checked, nearest edge {nearest:.1} px out");
}

#[test]
fn f026_the_field_marks_name_no_key() {
    // 🔴 **The structural half of the rule the app-level test measures on screen.**
    //
    // This element used to letter its two best rings `Q` and `E`. `F-024` — the feature that
    // makes the hook fire at `AnchorField`'s best candidate — is unbuilt, so `vector::hook`
    // took `vector::aim`'s raycast and the letters captioned a point the key never flew to.
    // Measured before they came off: the game's log put the fired anchor at `(51.00, 1.65,
    // -1.00)`, which projects to `(640.00, 357.77)`, and this element drew its `Q` at
    // `(445.93, 352.07)` — **194.15 px, 17.30 deg away** — while `arm_aim`'s `Q`, the one the
    // key obeys, was within 0.5 px in x.
    //
    // **Never draw a promise the game does not keep; a disclosed lie is still a lie on
    // screen.** The header disclosed this one and shipped it, which is why the guard is a test
    // and not a paragraph.
    //
    // It reads the source because that is the only place a letter can be re-introduced without
    // a running app noticing on the day it happens — the same technique
    // `f171_the_marker_letters_are_the_keys_that_fire_the_arms` uses on `src/net/local.rs`.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hud/anchor_marks.rs"),
    )
    .expect("src/hud/anchor_marks.rs must be readable");
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for side in Side::ALL {
        let letter = arm_aim::key_label(side);
        assert!(
            !code.contains(&format!("\"{letter}\"")),
            "`src/hud/anchor_marks.rs` spells the string \"{letter}\" — that is the key label \
             for the {side:?} arm. The anchor field may say \"there is an anchor here\"; it may \
             not say \"{letter} goes here\", because until `F-024` lands it does not. \
             `docs/BUGS.md` B-011"
        );
    }
    // And the state table has no room for one either: a `glyph` field is how the letters got
    // on screen the first time.
    assert!(
        !code.contains("glyph"),
        "`src/hud/anchor_marks.rs` has a glyph again — the field marks caption no key"
    );
}

#[test]
fn f026_an_anchored_arm_marks_its_rope_with_a_filled_symbol_and_a_tether() {
    // `F-026`'s fourth state, and it is the one that does **not** come out of the field: an
    // anchor you are hanging on is worth drawing whether or not it is still a candidate, and
    // after half a swing it usually is not.
    use defeated_by_titan::hud::anchor_marks::{AnchorMarkTether, MarkState};
    let mut app = app();
    attach_screen(&mut app);

    // A point that is really in front of the player, so it projects.
    let anchor_m = Vec3::new(0.0, 18.0, -30.0);
    stand_and_look(&mut app, Vec3::new(0.0, 12.0, 0.0), 0.0, 0.0);
    anchor_arm_at(&mut app, Side::Left, anchor_m);
    force_mark_refresh(&mut app);
    run_hud(&mut app);

    let marks = anchor_marks(&mut app);
    let anchored: Vec<_> = marks.iter().filter(|(_, s, ..)| *s == MarkState::Anchored).collect();
    assert_eq!(
        anchored.len(),
        1,
        "one arm is anchored, so exactly one mark says so — found {} in {marks:?}",
        anchored.len()
    );
    assert_eq!(
        anchored[0].2, anchor_m,
        "the anchored mark has to sit on the rope's own anchor and nowhere else"
    );
    // Nothing else may claim the same point: a weak ring under its own disc would be the field
    // saying the point is free while the rope is on it.
    assert_eq!(
        marks.iter().filter(|(_, _, p, _)| *p == anchor_m).count(),
        1,
        "the anchored point is marked twice"
    );
    // And the *Seilverbindung* is really laid out — the half of `F-026`'s fourth state that a
    // state enum alone cannot promise.
    let mut q = app.world_mut().query::<(&AnchorMarkTether, &Node)>();
    let shown = q.iter(app.world()).filter(|(_, node)| node.display != Display::None).count();
    assert_eq!(shown, 1, "the anchored mark's rope connection is not drawn");
}

#[test]
fn f027_the_marks_are_capped_thinned_and_stay_far_under_a_seventh_of_the_screen() {
    // `F-027`'s acceptance, verbatim: *„In der dichtesten Map ueberdecken Marker nie mehr als
    // 15 Prozent der Bildflaeche"* — plus the cap („maximal 12") and the thinning
    // („bei hoher Punktdichte ausgeduennt"), all three measured on the real map.
    //
    // The area is summed over bounding boxes **without** subtracting overlaps, i.e. an upper
    // bound: if the sum is under the limit the union certainly is.
    let data_max = {
        let data = app().world().resource::<GameData>().clone();
        data.game.hud.marker_max as usize
    };
    assert_eq!(data_max, 12, "`F-027` says twelve — `game.ron: game.hud.marker_max` says {data_max}");

    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    let gap_px = app.world().resource::<GameData>().game.hud.marker_min_gap_px;

    let mut worst_pct = 0.0_f32;
    let mut worst_count = 0usize;
    let mut looks = 0;
    for yaw_deg in [0.0_f32, 40.0, 80.0, 120.0, 160.0, 200.0, 240.0, 280.0, 320.0] {
        for pitch_deg in [-30.0_f32, -10.0, 5.0] {
            stand_look_and_mark(&mut app, Vec3::new(0.0, 14.0, 0.0), yaw_deg, pitch_deg);
            let marks = anchor_marks(&mut app);
            looks += 1;
            // **The cap has to be asked where it is applied, not where it happens to hold.**
            // Counting the drawn nodes cannot see it at all: `spawn_anchor_marks` puts down
            // exactly `marker_max` slots, so the screen is under the cap even with the cap
            // deleted — measured 2026-08-26 by deleting it and watching this test stay green
            // (`CLAUDE.md` rule 5: delete the thing you are measuring and check the number
            // moves). So `pick` is called here directly, on the real field and the real camera.
            {
                use defeated_by_titan::hud::anchor_marks::pick;
                use defeated_by_titan::world::anchor::AnchorField;
                let (camera, camera_at) = camera_of(&mut app);
                let data = app.world().resource::<GameData>().clone();
                let field = app.world().resource::<AnchorField>().clone();
                let eye = camera_at.translation();
                let candidates = field.candidates(
                    eye,
                    camera_at.forward().as_vec3(),
                    camera_at.up().as_vec3(),
                    data.game.vector.hook_range_m,
                    data.game.hud.marker_cone_h_deg,
                    data.game.hud.marker_cone_v_deg,
                );
                let picks = pick(&field, &candidates, [None, None], eye, &data.game.hud, |p| {
                    camera.world_to_viewport(&camera_at, p).ok()
                });
                assert!(
                    picks.len() <= data_max,
                    "at yaw {yaw_deg} pitch {pitch_deg} the selection returned {} picks out of                      {} candidates — `F-027`'s cap is {data_max}",
                    picks.len(),
                    candidates.len()
                );
            }
            assert!(
                marks.len() <= data_max,
                "at yaw {yaw_deg} pitch {pitch_deg} {} marks are on screen — the cap is \
                 {data_max} (`F-027`)",
                marks.len()
            );
            let area: f32 = marks
                .iter()
                .map(|(_, _, _, (a, b, c, d))| (c - a).max(0.0) * (d - b).max(0.0))
                .sum();
            let pct = 100.0 * area / (w * h);
            worst_pct = worst_pct.max(pct);
            worst_count = worst_count.max(marks.len());
            assert!(
                pct <= 15.0,
                "at yaw {yaw_deg} pitch {pitch_deg} the marks cover {pct:.2} % of the screen — \
                 `F-027` allows 15 %"
            );
            // The thinning, measured rather than assumed: no two marks are closer than the gap
            // the file names. Two rings one on top of the other is the clutter this exists
            // against, and a cap alone does not prevent it.
            for i in 0..marks.len() {
                for j in (i + 1)..marks.len() {
                    let (_, _, _, a) = marks[i];
                    let (_, _, _, b) = marks[j];
                    let ca = Vec2::new((a.0 + a.2) * 0.5, (a.1 + a.3) * 0.5);
                    let cb = Vec2::new((b.0 + b.2) * 0.5, (b.1 + b.3) * 0.5);
                    assert!(
                        ca.distance(cb) >= gap_px - 1.0,
                        "at yaw {yaw_deg} pitch {pitch_deg} two marks stand {:.1} px apart, \
                         under the {gap_px} px `game.ron: game.hud.marker_min_gap_px` promises: \
                         {:?} and {:?}",
                        ca.distance(cb),
                        marks[i],
                        marks[j]
                    );
                }
            }
        }
    }
    assert!(
        worst_count >= 4 && looks == 27,
        "the densest look produced only {worst_count} marks over {looks} looks — this test \
         measured an almost empty screen and would have passed on any cap (FIND-152)"
    );
    println!(
        "f027: worst {worst_count} marks, {worst_pct:.2} % of {w}x{h} covered over {looks} looks"
    );
}

#[test]
fn f027_switching_the_markers_off_draws_nothing_and_searches_nothing() {
    // `F-027`: *„inklusive vollstaendiger Abschaltung"*. Two halves, and the second is the one
    // that is easy to fake: an element that still runs the search and only hides the rings
    // gives the player back nothing at all. So this asserts the clock never even ticks.
    use defeated_by_titan::hud::anchor_marks::{AnchorMark, MarkClock, MarkState};
    let mut app = app();
    attach_screen(&mut app);

    stand_look_and_mark(&mut app, Vec3::new(0.0, 14.0, 0.0), 0.0, -10.0);
    let on = anchor_marks(&mut app).len();
    assert!(on > 0, "with the element on there has to be something to switch off (FIND-152)");

    {
        let mut data = app.world_mut().resource_mut::<GameData>();
        data.game.hud.marker_max = 0;
    }
    app.world_mut().resource_mut::<MarkClock>().refreshes = 0;
    let before = app.world().resource::<MarkClock>().refreshes;
    force_mark_refresh(&mut app);
    run_hud(&mut app);

    assert_eq!(anchor_marks(&mut app).len(), 0, "{on} marks were on screen and the switch is off");
    assert_eq!(
        app.world().resource::<MarkClock>().refreshes,
        before,
        "the search ran although the element is switched off — then the switch costs the \
         player his rings and gives him back no frame time"
    );
    // And the states really went back to `Off`, not just the nodes to `Display::None`: a mark
    // that keeps its state is a mark that comes back the moment anything else touches a node.
    let mut q = app.world_mut().query_filtered::<&MarkState, With<AnchorMark>>();
    let live = q.iter(app.world()).filter(|s| **s != MarkState::Off).count();
    assert_eq!(live, 0, "{live} slots still hold a state with the element off");

    // The control this whole test needs (`CLAUDE.md` rule 5): put the switch back and check the
    // number moves. Without it, "0 marks" is also what a broken search prints.
    {
        let mut data = app.world_mut().resource_mut::<GameData>();
        data.game.hud.marker_max = 12;
    }
    force_mark_refresh(&mut app);
    run_hud(&mut app);
    assert!(
        anchor_marks(&mut app).len() > 0,
        "switching the element back on produced nothing — then the `0` above measured a broken \
         search and not a working switch"
    );
    assert!(
        app.world().resource::<MarkClock>().refreshes > before,
        "the clock never ticked even with the element on"
    );
}

#[test]
fn f030a_the_set_refreshes_at_ten_hertz_and_the_pixel_every_frame() {
    // `F-030a` verbatim: *„mit 10 Hz Aktualisierungsrate und Interpolation der
    // Markerpositionen dazwischen"*. Read literally the second half asks for an interpolated
    // screen position, which is the FIND-129 lie with a new name on it. It is implemented on
    // the reading that costs nothing and lies about nothing — the SET is refreshed at 10 Hz,
    // the PROJECTION of every chosen point is recomputed every frame — and this is the test
    // that pins both halves apart.
    use defeated_by_titan::hud::anchor_marks::MarkClock;
    let mut app = app();
    attach_screen(&mut app);

    // Half of the whole claim: the cadence itself, as arithmetic, without a wall clock.
    let mut clock = MarkClock::default();
    assert!(clock.due(0.0, 10.0), "the first frame always searches");
    let mut ticks = 0;
    for _ in 0..60 {
        if clock.due(1.0 / 60.0, 10.0) {
            ticks += 1;
        }
    }
    assert_eq!(ticks, 10, "one second of 60 fps has to produce exactly ten searches, not {ticks}");

    // The other half, in the running HUD: turn the camera **without** letting the set refresh.
    // Every mark keeps its world point and every ring still lands on that point's new pixel.
    stand_look_and_mark(&mut app, Vec3::new(0.0, 14.0, 0.0), 0.0, -10.0);
    let before = anchor_marks(&mut app);
    assert!(before.len() >= 3, "only {} marks to follow (FIND-152)", before.len());
    let refreshes = app.world().resource::<MarkClock>().refreshes;

    stand_and_look(&mut app, Vec3::new(0.0, 14.0, 0.0), 6.0, -10.0);
    // **No time passes.** `First` is never run in these tests, so `Time`'s delta is frozen at
    // whatever the last real `app.update()` left in it — which is a good fraction of a second
    // while the world is being built, enough to roll the 10 Hz gate over on its own. Setting it
    // to zero is the only way to ask the question this test is asking: *the set stayed put, and
    // the pixels still followed*.
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::ZERO);
    run_hud(&mut app); // no `force_mark_refresh`: the SET must stay put
    assert_eq!(
        app.world().resource::<MarkClock>().refreshes,
        refreshes,
        "the set refreshed although no time passed — then the 10 Hz gate is not a gate"
    );

    let after = anchor_marks(&mut app);
    let (camera, camera_at) = camera_of(&mut app);
    let mut moved = 0;
    for (slot, _, point_m, (min_x, min_y, max_x, max_y)) in &after {
        assert!(
            before.iter().any(|(s, _, p, _)| s == slot && p == point_m),
            "slot {slot} changed its point without a refresh"
        );
        let want = camera.world_to_viewport(&camera_at, *point_m).expect("still projectable");
        let drew = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        assert!(
            (drew - want).abs().max_element() <= 0.51,
            "slot {slot} did not follow its own point across 6 deg of yaw: drawn at {drew:?}, \
             the point projects to {want:?}"
        );
        if let Some((_, _, _, old)) = before.iter().find(|(s, ..)| s == slot) {
            if ((min_x + max_x) * 0.5 - (old.0 + old.2) * 0.5).abs() > 2.0 {
                moved += 1;
            }
        }
    }
    // The control: if nothing moved, the paragraph above proved nothing at all.
    assert!(
        moved >= 3,
        "only {moved} marks moved over 6 deg of yaw — the camera did not really turn and this \
         test compared a frame with itself"
    );
}

#[test]
fn f030a_the_candidate_search_stays_inside_its_frame_budget() {
    // `F-030a`'s acceptance: *„Die Suche kostet unter 0,8 ms pro Frame … auch bei 4000
    // Ankerpunkten in der Map."* Measured against the real field of the real map, through the
    // real function, and at the real cone the HUD asks for.
    use defeated_by_titan::hud::anchor_marks::pick;
    use defeated_by_titan::world::anchor::AnchorField;

    let mut app = app();
    attach_screen(&mut app);
    stand_and_look(&mut app, Vec3::new(0.0, 14.0, 0.0), 0.0, -10.0);
    run_hud(&mut app);

    let (camera, camera_at) = camera_of(&mut app);
    let data = app.world().resource::<GameData>().clone();
    let field = app.world().resource::<AnchorField>().clone();
    assert!(
        field.len() >= 1000,
        "the field holds only {} points — a budget measured against an empty map is not a \
         budget (FIND-152)",
        field.len()
    );

    let eye = camera_at.translation();
    let forward = camera_at.forward().as_vec3();
    let up = camera_at.up().as_vec3();
    let tuning = &data.game.hud;

    let runs = 200;
    let started = std::time::Instant::now();
    let mut total = 0usize;
    for _ in 0..runs {
        let candidates = field.candidates(
            eye,
            forward,
            up,
            data.game.vector.hook_range_m,
            tuning.marker_cone_h_deg,
            tuning.marker_cone_v_deg,
        );
        let picks = pick(&field, &candidates, [None, None], eye, tuning, |p| {
            camera.world_to_viewport(&camera_at, p).ok()
        });
        total += picks.len();
    }
    let per_search_ms = started.elapsed().as_secs_f64() * 1000.0 / runs as f64;
    // The search runs at `marker_refresh_hz`, not per frame — that is what the 10 Hz gate buys
    // — so the per-FRAME cost at 60 fps is what `F-030a` is about.
    let per_frame_ms = per_search_ms * (tuning.marker_refresh_hz as f64 / 60.0);
    println!(
        "f030a: {} points, {:.0} picks/search, {per_search_ms:.3} ms per search, \
         {per_frame_ms:.3} ms per frame at 60 fps",
        field.len(),
        total as f64 / runs as f64
    );
    assert!(
        total > 0,
        "the search found nothing over {runs} runs — then the timing below measured an early \
         return and not a search"
    );
    assert!(
        per_frame_ms < 0.8,
        "the candidate search costs {per_frame_ms:.3} ms per frame ({per_search_ms:.3} ms per \
         search at {} Hz) — `F-030a` allows 0.8 ms",
        tuning.marker_refresh_hz
    );
}

/// 🔴 **The one the round of 2026-08-26 shipped a lie against.**
///
/// `F-026`'s two best rings carried the letters `Q` and `E`, and `F-024` — the feature that
/// makes `Q` fire at [`AnchorField`](defeated_by_titan::world::anchor::AnchorField)'s best
/// candidate — is **unbuilt**. `vector::hook::fire` takes `vector::aim`'s raycast target, which
/// has never heard of the field. So the screen carried **two** `Q` glyphs on two different
/// points, 194.15 px apart at the element's own stand, and only one of them was the point the
/// key obeys.
///
/// The rule this test is the guard for, in the words it deserves: **never draw a promise the
/// game does not keep; a disclosed lie is still a lie on screen.** The lie was written down in
/// the module header and shipped anyway, which is what makes it a rule rather than an accident.
///
/// So: **one `Q` and one `E` on the whole screen**, and they are `arm_aim`'s — the marker that
/// is measured at 0.0 px against the point the rope flies to (`FIND-129`). The field element
/// may say *"there is an anchor here"*; it may not say *"Q goes here"*.
#[test]
fn f026_exactly_one_q_and_one_e_are_on_the_screen_and_they_are_the_arms() {
    let mut app = app();
    attach_screen(&mut app);

    let mut most_marks = 0usize;
    let mut looks = 0usize;
    for (at_m, yaw_deg, pitch_deg) in [
        (Vec3::new(0.0, 12.0, 0.0), 0.0_f32, -8.0_f32),
        (Vec3::new(0.0, 12.0, 0.0), 75.0, -8.0),
        (Vec3::new(0.0, 12.0, 0.0), 210.0, -8.0),
        (Vec3::new(0.0, 60.0, 0.0), 35.0, -25.0),
        (Vec3::new(51.0, 2.0, 13.0), 0.0, 0.0),
    ] {
        stand_look_and_mark(&mut app, at_m, yaw_deg, pitch_deg);
        let marks = anchor_marks(&mut app);
        most_marks = most_marks.max(marks.len());
        looks += 1;

        // Every VISIBLE node on the whole HUD whose text is one of the two key labels — no
        // filter by component, because the question is what the player's eye finds, not what
        // one element believes it drew.
        let mut letters: Vec<(String, Vec2, Option<Side>)> = Vec::new();
        let mut q =
            app.world_mut().query::<(Entity, &Text, &Node, &ComputedNode, &UiGlobalTransform)>();
        let found: Vec<(Entity, String, Vec2)> = q
            .iter(app.world())
            .filter(|(_, _, node, ..)| node.display != Display::None)
            .filter_map(|(e, text, _, computed, ui_at)| {
                let body = text.0.trim().to_string();
                (body == arm_aim::key_label(Side::Left)
                    || body == arm_aim::key_label(Side::Right))
                .then(|| {
                    let (min_x, min_y, max_x, max_y) = rect(computed, ui_at);
                    (e, body, Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
                })
            })
            .collect();
        for (entity, body, at) in found {
            let owner = app.world().get::<ArmMarkerLabel>(entity).map(|l| l.0);
            letters.push((body, at, owner));
        }
        println!(
            "f026 letters: yaw {yaw_deg} pitch {pitch_deg} — {} marks, letters {letters:?}",
            marks.len()
        );

        for side in Side::ALL {
            let want = arm_aim::key_label(side);
            let drawn: Vec<&(String, Vec2, Option<Side>)> =
                letters.iter().filter(|(b, ..)| b == want).collect();
            assert_eq!(
                drawn.len(),
                1,
                "at yaw {yaw_deg} pitch {pitch_deg} the screen carries {} `{want}` glyphs at \
                 {drawn:?}. Exactly one thing on this screen may say `{want}`: the arm marker, \
                 which stands on the point the key really fires at. A second `{want}` is a \
                 promise the game does not keep",
                drawn.len()
            );
            // And the survivor is the ARM's own label — the marker measured at 0.0 px against
            // the point the rope flies to (`FIND-129`) — and not a field ring that happens to
            // be the only one wearing the letter this frame.
            assert_eq!(
                drawn[0].2,
                Some(side),
                "the only `{want}` on screen is at {:?} and it is not the {side:?} arm's own \
                 label — so the letter is captioning something the key does not obey",
                drawn[0].1
            );
        }
    }
    assert!(
        most_marks >= 3 && looks == 5,
        "the anchor field drew at most {most_marks} marks over {looks} looks — this test \
         watched an element that was not running (FIND-152)"
    );
}
