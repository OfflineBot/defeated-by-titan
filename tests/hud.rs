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
//!
//! The arm-aim markers are GONE since 2026-09-01 (docs/NEXT.md §5E-c: *„die kreise
//! können ganz weg! also in der mitte“*, docs/FINDINGS.md FIND-227). The tests that measured
//! them — the shape table, the landing preview, the letters, the miss words — went with the
//! element; what replaced them is the INVERSION, `f171_the_centre_carries_nothing_but_the_x`:
//! the middle of the screen belongs to the X crosshair and to nothing else this module spawns.

use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::{ComputedNode, UiGlobalTransform};

use defeated_by_titan::data::{assets_dir, GameData};
use defeated_by_titan::debug::screenshot::{OFFSCREEN_HEIGHT, OFFSCREEN_WIDTH};
use defeated_by_titan::hud::blade_pips::{BladeCluster, BladePip, SharpnessBar};
use defeated_by_titan::hud::crosshair::{
    self, CrosshairPart, CrosshairShape, CrosshairState,
};
use defeated_by_titan::hud::board;
use defeated_by_titan::hud::gas_bar::GasBar;
use defeated_by_titan::hud::health_bar::{HealthBar, HealthTrack};
use defeated_by_titan::hud::hit_mark::{HitFlash, HitMark};
use defeated_by_titan::hud::objective::ObjectiveLine;
use defeated_by_titan::hud::{HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use defeated_by_titan::menu::board::Board;
use defeated_by_titan::menu::lobby::{chosen, entries, LobbyChoice};
use defeated_by_titan::mission::{KillTally, MissionPhase};
use defeated_by_titan::progress::Career;
use defeated_by_titan::shared::{
    Blades, Cli, Gas, Health, HitZone, LocalPlayer, PlayerId, PlayerSettings, Side, TitanHit,
    TitanId,
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
    // **The one documented exception is not in this app**: the `F-016` search band is absent
    // here for a reason and not by luck — both assist knobs ship at 0
    // (`PlayerSettings::from_world`), so no probe is cast and no tick is drawn. What holds it
    // to the sight core instead is `f016_the_band_keeps_the_sight_core_clear`, over the whole
    // slider — this test would let the band through, and that is exactly why the other one
    // exists. (The arm markers were the other exception until 2026-09-01. They are GONE —
    // §5E-c, FIND-227 — and `f171_the_centre_carries_nothing_but_the_x` asserts the absence.)
    //
    // **There is no third exception, and there is no longer a skip list here.** The `F-026`
    // anchor marks used to be one — twelve rings on points out of `world::anchor::AnchorField`,
    // skipped by the `hud_anchor_` name their spawner gave them. The field was deleted on
    // 2026-08-28 (the user, 2026-08-27: *„es soll auf jeglicher oberflqche einhaken. nicht an
    // hardcoded punkten etc!"*), so every node this test now sees is under the full box rule
    // with no exception at all.
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
    // ⭐ `F-177`, and the same argument as the objective above: this app has no hub and no
    // board, so the panel would sit at `display: None` and this test would be blind to the one
    // element that draws a whole column of text. So it is shown here in its **widest** state —
    // the real list out of `missions.ron`, which is what the open board says — rather than
    // being skipped as a hidden node. An assertion satisfied by an empty screen is not an
    // assertion.
    {
        let data = app.world().resource::<GameData>().clone();
        let list = entries(&data);
        let full = a_career_that_has_cleared_everything(&data);
        let widest =
            board::board_text(true, true, &list, list.first(), &data.progress, Some(&full))
                .expect("in range and open has to say something");
        let mut q = app
            .world_mut()
            .query_filtered::<(&mut Text, &mut Node), With<board::BoardPanel>>();
        for (mut text, mut node) in q.iter_mut(app.world_mut()) {
            text.0 = widest.clone();
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
    let mut crosshair_seen = 0;
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
        // 🔴 **The crosshair is exempt from the box since 2026-09-01, and it is the ONLY
        // named exemption** — the placed arm markers were the other one until §5E-c retired
        // the element the same day (FIND-227). The box was 20 % BECAUSE it was defined as
        // the old crosshair's own reach; the user replaced that crosshair with a
        // mittel-klein X that lives at the centre by his own words (*„4 striche wo in der
        // mitte nichts"*). The claim that replaces the box for these eight nodes is STRONGER
        // where it matters and lives in
        // `the_x_crosshair_hugs_the_centre_and_keeps_the_aim_pixel_free`: within 60 px of the
        // centre, whole sight core empty, every state, 1 px sampling. **Not a silent skip:**
        // counted below and asserted.
        if name.as_str().starts_with("hud_crosshair_") {
            crosshair_seen += 1;
            continue;
        }
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
    println!(
        "f170: {seen} nodes checked against the box, {crosshair_seen} crosshair nodes under \
         the X test's own claim"
    );
    // The crosshair really went through its exemption — 8, because the state above is Cortex.
    // A `0` would mean the names changed and the box claim was quietly carrying the element;
    // the stronger claim would still hold, but the count here is what keeps the exemption a
    // decision instead of a leftover.
    assert_eq!(
        crosshair_seen, 8,
        "{crosshair_seen} crosshair nodes went through the exemption instead of 8 — the name \
         prefix or the Cortex node count changed and this test is no longer seeing the element"
    );
    // And the board panel really was one of them, laid out with real pixels. Without this the
    // block above could have been shown into a zero-width node and skipped by the `<= 0.0`
    // guard, and the whole `F-177` half of this test would be arithmetic about nothing.
    let mut q = app
        .world_mut()
        .query_filtered::<(&Node, &ComputedNode), With<board::BoardPanel>>();
    let (node, computed) = q.iter(app.world()).next().expect("the board panel is spawned");
    assert_eq!(node.display, Display::Flex, "the board panel was not shown for this test");
    assert!(
        computed.size().x > 1.0 && computed.size().y > 1.0,
        "the board panel laid out into {:?} — the keep-out check above never saw it",
        computed.size()
    );
}

/// §5E-c, the INVERSION of everything the retired marker block used to assert (user,
/// 2026-09-01: *„die kreise können ganz weg! also in der mitte"* — `docs/FINDINGS.md`
/// FIND-227, superseding his own 2026-08-19 *„wichtig wäre nur dass diese auch genau da sind
/// visuell wo das seil auch landen würde"* — newest word wins, both dated in the finding).
///
/// Two halves, and the second is the acceptance sentence measured in the layout:
/// 1. the arm-marker assembly is GONE FROM THE TREE — not hidden, not parked, absent: no
///    node whose name starts with `hud_arm_` exists at all;
/// 2. every laid-out HUD rectangle that touches the central 20 % x 20 % box belongs to the
///    X crosshair — the centre carries the X and NOTHING else.
///
/// Captured RED against the pre-removal tree first (rule 5): six `hud_arm_*` nodes.
/// The pixel half of the same claim is the decoded `--offscreen` frame in FIND-227.
#[test]
fn f171_the_centre_carries_nothing_but_the_x() {
    let mut app = app();
    attach_screen(&mut app);
    let player = local_player(&mut app);
    app.world_mut().entity_mut(player).insert(Health::full(100.0));
    app.world_mut().run_schedule(Update);
    set_crosshair(&mut app, CrosshairState::Cortex);
    app.world_mut()
        .run_system_once(crosshair::shape_crosshair)
        .expect("the one-shot system runs");
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);

    // 1. Absent, not hidden. A `Display::None` node or a parked one would pass a pixel test
    //    and still be one relayout away from coming back.
    let arm_nodes: Vec<String> = {
        let mut q = app.world_mut().query_filtered::<&Name, With<Node>>();
        q.iter(app.world())
            .map(|n| n.as_str().to_owned())
            .filter(|n| n.starts_with("hud_arm_"))
            .collect()
    };
    assert!(
        arm_nodes.is_empty(),
        "the marker assembly is still in the tree: {arm_nodes:?} — §5E-c says the circles go \
         ENTIRELY (FIND-227)"
    );

    // 2. The centre belongs to the X alone.
    let (w, h) = screen(&mut app);
    assert!(w > 0.0 && h > 0.0, "no viewport — every rectangle below would be empty");
    let box_min_x = w * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = w * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = h * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = h * KEEP_OUT_HIGH_PCT / 100.0;
    let mut x_nodes = 0;
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
        let touches =
            min_x < box_max_x && max_x > box_min_x && min_y < box_max_y && max_y > box_min_y;
        if !touches {
            continue;
        }
        assert!(
            name.as_str().starts_with("hud_crosshair_"),
            "`{name}` stands in the centre band ({min_x:.1}, {min_y:.1})..({max_x:.1}, \
             {max_y:.1}) — the centre carries the X and NOTHING else"
        );
        x_nodes += 1;
    }
    // Cortex is the widest state: eight nodes. A zero would mean the X was not laid out and
    // half of this test measured an empty band.
    assert_eq!(x_nodes, 8, "{x_nodes} crosshair nodes in the centre band instead of 8");
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

// ---------------------------------------------------------------------------------------
// The arm markers stood here until 2026-09-01 — shape table, landing preview, letters,
// miss words. Retired whole under §5E-c (*„die kreise können ganz weg“*, FIND-227); the
// inversion lives in `f171_the_centre_carries_nothing_but_the_x`. The helpers that survive
// (`stand_and_look`, `run_hud`, `sim_step`, `camera_of`) serve the band and crosshair tests.
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

// ---------------------------------------------------------------------------------------
// F-026 — a fired arm previews where it LANDS, not the hook in its hand
// ---------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------
// F-023 — the guard: the DRAWN pixel against the FIRED world point, in the running app
// ---------------------------------------------------------------------------------------

/// The 3D camera and its transform, so a test can project a world point **itself** — the band
/// tests read it to recompute `catch_band`'s projection from the outside.
fn camera_of(app: &mut App) -> (Camera, GlobalTransform) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>();
    let (camera, at) = q.iter(app.world()).next().expect("there must be a 3D camera");
    (camera.clone(), *at)
}

/// Moves the two aim-assist knobs the way the settings screen moves them (`F-024` / `F-025`).
fn set_assist(app: &mut App, catch_pct: f32, strength_pct: f32) {
    let mut s = app.world_mut().resource_mut::<PlayerSettings>();
    s.assist_catch_pct = catch_pct;
    s.assist_strength_pct = strength_pct;
}

/// The four arm states the sweep below walks, by the name [`put_arm_on`] answers to.
const ROPE_STATES: [&str; 4] = ["idle", "flying", "anchored", "retracting"];

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

// ===========================================================================================
// F-177 — the board's panel says what the file says, and nothing it made up
// ===========================================================================================

/// ★ **The panel is `missions.ron`, not a list this file wrote.**
///
/// It drives the real game: `--hub`, stand at the board, press `F` through the real
/// `KeyboardInput` path, and read the `Text` back out. Every sortie in the file has to be on
/// it, the cursor has to stand on the one `menu::lobby::chosen` names, and nothing may be on
/// it that the file does not have.
///
/// ⚠️ **The provenance is the game's**: the highlight is compared against `chosen()`'s answer
/// for the resource the game holds, not against a pair this test invented — and the "which
/// sortie" question is asked exactly once, in `menu::lobby`, which is the corollary that cost
/// this project a round (`CLAUDE.md` rule 5).
#[test]
fn f177_the_board_panel_lists_exactly_what_missions_ron_offers() {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonState;

    let mut app = defeated_by_titan::app(Cli { headless: true, hub: true, ..default() });
    let fake_window = app.world_mut().spawn_empty().id();
    for _ in 0..4 {
        app.update();
    }

    // Stand at the board — one metre inside its circle.
    let (centre, radius) = {
        let post = &app.world().resource::<GameData>().missions.hub.board;
        (Vec3::from(post.center_m), post.radius_m)
    };
    {
        let mut q = app.world_mut().query_filtered::<&mut Transform, With<LocalPlayer>>();
        for mut t in q.iter_mut(app.world_mut()) {
            t.translation = centre + Vec3::new(0.0, 0.2, radius - 1.0);
        }
    }
    app.update();

    let panel = |app: &mut App| -> String {
        let mut q = app.world_mut().query_filtered::<&Text, With<board::BoardPanel>>();
        q.iter(app.world()).next().expect("the board panel is spawned").0.clone()
    };

    // Shut: the prompt, and no sortie names on it.
    assert!(app.world().resource::<Board>().in_range, "the fixture is not at the board");
    let shut = panel(&mut app);
    assert!(shut.contains(board::HEADING), "the prompt does not name the board: {shut:?}");

    // Open it the way a player does.
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyF,
            logical_key: Key::Character("f".into()),
            state,
            text: None,
            repeat: false,
            window: fake_window,
        });
        app.update();
    }
    assert!(app.world().resource::<Board>().open, "F did not open the board");

    let data = app.world().resource::<GameData>().clone();
    let list = entries(&data);
    let said = panel(&mut app);
    assert!(list.len() >= 5, "missions.ron offers only {} sorties", list.len());

    // Every sortie in the file is on the panel, as its own line.
    let lines: Vec<&str> = said.lines().collect();
    let mut counted = 0usize;
    for entry in &list {
        let want = match &entry.1 {
            Some(level) => format!("{}  {level}", entry.0),
            None => entry.0.clone(),
        };
        let hits = lines.iter().filter(|l| l.trim_start_matches("> ").trim() == want).count();
        assert_eq!(hits, 1, "{want:?} appears {hits} times on the panel:\n{said}");
        counted += 1;
    }
    assert_eq!(counted, list.len(), "sorties were skipped, and a skip is invisible");

    // And nothing is on it that the file does not have: as many sortie rows as entries.
    let rows = lines
        .iter()
        .filter(|l| l.starts_with(board::CURSOR) || l.starts_with(board::NO_CURSOR))
        .count();
    assert_eq!(
        rows,
        list.len(),
        "the panel draws {rows} sortie rows against {} in missions.ron:\n{said}",
        list.len()
    );

    // The cursor stands on the one that would actually fly — `menu::lobby::chosen`'s answer,
    // asked once and read here rather than re-derived.
    let picked = chosen(&data, app.world().resource::<LobbyChoice>()).expect("a sortie");
    let marked: Vec<&&str> = lines.iter().filter(|l| l.starts_with(board::CURSOR)).collect();
    assert_eq!(marked.len(), 1, "the panel marks {} sorties:\n{said}", marked.len());
    let want = match &picked.1 {
        Some(level) => format!("{}  {level}", picked.0),
        None => picked.0.clone(),
    };
    assert_eq!(
        marked[0].trim_start_matches("> ").trim(),
        want,
        "the cursor stands on a sortie that is not the one a hold would deploy"
    );
}

/// ★ **The panel keeps out of the middle of the screen, on its own.**
///
/// `f170_nothing_covers_the_middle_of_the_screen` covers every element at once and is the
/// standing guard — but it panics on the first offender, so an element added today can hide
/// behind one that is already red (it is, `docs/FINDINGS.md`: `hud_arm_marker_Left` stands in
/// the box in the tree this landed in, and it is not this element's doing). This one asks the
/// 🔴 **The widest the mission board can ever be**, and the career is what makes it so.
///
/// Since 2026-09-01 each row can carry a ladder marker — `hud::board::CLEARED` for a rung this
/// career has won (`docs/FINDINGS.md` FIND-222). A `None` career draws **no markers at all**, so
/// the two keep-out tests below were measuring a panel narrower than the one a real player sees:
/// exactly the blind spot `docs/lessons/fixtures.md` is about — *name what the code reads, name
/// what the fixture varies, the difference is the bug.* This career clears **every** entry the
/// file offers, which is the widest state reachable with the shipped `progress.ron`
/// (`gates: {}`, so no row can say `LOCKED`, which is longer still — and
/// `src/hud/board.rs::f121_the_shipped_ladder_takes_no_playable_content_away` is what goes red
/// the day that stops being true).
fn a_career_that_has_cleared_everything(data: &GameData) -> Career {
    let cleared = entries(data)
        .into_iter()
        .map(|(template, level)| match level {
            Some(l) => format!("{template}/{l}"),
            None => template,
        })
        .collect();
    Career {
        level: 100,
        xp: 999_999,
        xp_into_level: 0,
        xp_for_the_next_level: None,
        skill_points: 99,
        gear_points: 204,
        gear_points_spent: 0,
        cleared,
        gear: std::collections::BTreeMap::new(),
        rank: "S".to_string(),
        last_sortie_xp: 0,
        levelled_up_to: None,
    }
}

/// question about `F-177` alone and cannot be masked by anybody else's node.
#[test]
fn f177_the_board_panel_stays_out_of_the_middle_of_the_screen() {
    let mut app = app();
    attach_screen(&mut app);
    app.world_mut().run_schedule(Update);

    // The widest thing it can ever say: the whole file, open, with a cursor on it.
    {
        let data = app.world().resource::<GameData>().clone();
        let list = entries(&data);
        let full = a_career_that_has_cleared_everything(&data);
        let widest =
            board::board_text(true, true, &list, list.first(), &data.progress, Some(&full))
                .expect("open says something");
        let mut q = app
            .world_mut()
            .query_filtered::<(&mut Text, &mut Node), With<board::BoardPanel>>();
        for (mut text, mut node) in q.iter_mut(app.world_mut()) {
            text.0 = widest.clone();
            node.display = Display::Flex;
        }
    }
    app.world_mut().run_schedule(PostUpdate);
    app.world_mut().run_schedule(PostUpdate);

    let (w, h) = screen(&mut app);
    assert!(w > 0.0 && h > 0.0, "the UI laid out into {w} x {h} — nothing below is measured");
    let (box_min_x, box_max_x) = (w * KEEP_OUT_LOW_PCT / 100.0, w * KEEP_OUT_HIGH_PCT / 100.0);
    let (box_min_y, box_max_y) = (h * KEEP_OUT_LOW_PCT / 100.0, h * KEEP_OUT_HIGH_PCT / 100.0);

    let mut q = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<board::BoardPanel>>();
    let (computed, at) = q.iter(app.world()).next().expect("the board panel is spawned");
    let (min_x, min_y, max_x, max_y) = rect(computed, at);
    assert!(
        max_x - min_x > 1.0 && max_y - min_y > 1.0,
        "the panel laid out into ({min_x}, {min_y})..({max_x}, {max_y}) — an assertion \
         satisfied by an empty rectangle is not an assertion"
    );
    assert!(
        !(min_x < box_max_x && max_x > box_min_x && min_y < box_max_y && max_y > box_min_y),
        "the board panel covers the middle: ({min_x:.1}, {min_y:.1})..({max_x:.1}, {max_y:.1}) \
         against a box of ({box_min_x:.1}, {box_min_y:.1})..({box_max_x:.1}, {box_max_y:.1})"
    );
}

// ---------------------------------------------------------------------------------------
// F-026 — the marker the PLAYER looks at, and not the projection a test computes
// ---------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The X crosshair (2026-09-01) — „4 striche wo in der mitte nichts und 45deg rotiert"
// ---------------------------------------------------------------------------

/// Whether `point` (physical px) lies inside this node's rotated rectangle.
fn obb_contains(computed: &ComputedNode, at: &UiGlobalTransform, point: Vec2) -> bool {
    let Some(inverse) = at.try_inverse() else {
        return false;
    };
    let local = inverse.transform_point2(point);
    let half = computed.size() * 0.5;
    local.x.abs() <= half.x && local.y.abs() <= half.y
}

/// The user, 2026-09-01, in one sentence: *„mach zudem die verbindung zu einem einfachen
/// crosshair und nicht so gkreise mit seiten strichen etc. sollen 4 striche wo in der mitte
/// nichts und 45deg rotiert und gröse eher mittel bis klein. aktuell ist mittel bis groß."*
///
/// Three claims, each half of his sentence:
/// - **mittel bis klein**: every crosshair pixel stays within [`X_REACH_MAX_PX`] of the
///   centre — the old ticks stood 128 px out at 1280 x 720 and that is the „mittel bis groß"
///   he is done with;
/// - **in der mitte nichts**: the whole sight core (`arm_aim::SIGHT_CORE_PX` on each side of
///   the aim pixel) contains not one crosshair pixel, in any state, sampled at 1 px pitch —
///   this is also the claim that replaces the 20 % keep-out box for this element, the way
///   FIND-098/FIND-129 replaced it for the arm markers;
/// - **45deg rotiert**: every visible node is rotated to the diagonals, so the element is an
///   X and never a `+`.
#[test]
fn the_x_crosshair_hugs_the_centre_and_keeps_the_aim_pixel_free() {
    const X_REACH_MAX_PX: f32 = 60.0;
    let mut app = app();
    attach_screen(&mut app);
    let (w, h) = screen(&mut app);
    assert!(w > 0.0 && h > 0.0, "no viewport, nothing measured");
    let centre = Vec2::new(w * 0.5, h * 0.5);

    for state in [CrosshairState::Free, CrosshairState::Anchor, CrosshairState::Cortex] {
        set_crosshair(&mut app, state);
        app.world_mut()
            .run_system_once(crosshair::shape_crosshair)
            .expect("the one-shot system runs");
        app.world_mut().run_schedule(PostUpdate);
        app.world_mut().run_schedule(PostUpdate);

        let mut seen = 0;
        let mut q = app.world_mut().query_filtered::<(
            &Name,
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
        ), With<CrosshairPart>>();
        for (name, node, computed, at) in q.iter(app.world()) {
            if node.display == Display::None || computed.size().min_element() <= 0.0 {
                continue;
            }
            seen += 1;
            // mittel bis klein: the farthest corner of the rotated rectangle.
            let half = computed.size() * 0.5;
            for corner in [
                Vec2::new(-half.x, -half.y),
                Vec2::new(half.x, -half.y),
                Vec2::new(-half.x, half.y),
                Vec2::new(half.x, half.y),
            ] {
                let reach = (at.transform_point2(corner) - centre).length();
                assert!(
                    reach <= X_REACH_MAX_PX,
                    "{state:?}: `{name}` reaches {reach:.1} px from the centre — the X is \
                     supposed to be mittel bis klein ({X_REACH_MAX_PX:.0} px), the old ticks' \
                     128 px is the mittel bis groß he is done with"
                );
            }
            // 45deg rotiert: the node's local x axis lies on a diagonal.
            let axis = at.matrix2.x_axis.normalize();
            assert!(
                (axis.x.abs() - axis.y.abs()).abs() < 1e-3,
                "{state:?}: `{name}` is axis-aligned (x axis {axis:?}) — the element has to \
                 be an X, never a +"
            );
        }
        assert!(seen >= 4, "{state:?}: only {seen} crosshair nodes laid out");

        // in der mitte nichts: the whole sight core, at 1 px pitch.
        let core = defeated_by_titan::hud::arm_aim::SIGHT_CORE_PX;
        let steps = core as i32;
        for dx in -steps..=steps {
            for dy in -steps..=steps {
                let point = centre + Vec2::new(dx as f32, dy as f32);
                let mut q = app.world_mut().query_filtered::<(
                    &Name,
                    &Node,
                    &ComputedNode,
                    &UiGlobalTransform,
                ), With<CrosshairPart>>();
                for (name, node, computed, at) in q.iter(app.world()) {
                    if node.display == Display::None {
                        continue;
                    }
                    assert!(
                        !obb_contains(computed, at, point),
                        "{state:?}: `{name}` covers ({dx:+}, {dy:+}) px off the aim pixel — \
                         in der mitte NICHTS, and the sight core is the middle"
                    );
                }
            }
        }
    }
}
