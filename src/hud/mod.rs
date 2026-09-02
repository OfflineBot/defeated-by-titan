//! hud — gas, blade state, health, objective, crosshair, hit feedback
//!
//! **Reads only.** And reads the state **of the local player** through the
//! [`LocalPlayer`](crate::shared::LocalPlayer) marker — this is the one place in the code that
//! knows who "I" am.
//!
//! PC-only means: more information at once, because no thumb covers half the screen
//! (Bible 3.5).
//!
//! ## Six elements, and one of them still has no producer
//!
//! | element | file | reads | producer |
//! |---|---|---|---|
//! | gas bar, left | [`gas_bar`] | [`Gas`](crate::shared::Gas) | `vector` — **exists** |
//! | blade pips, right | [`blade_pips`] | [`Blades`](crate::shared::Blades) | `blades` — the component exists, the wear does not |
//! | health bar, bottom centre | [`health_bar`] | [`Health`](crate::shared::Health) | job R3-A — **absent today** |
//! | crosshair, centre | [`crosshair`] | [`AimPoint`](crate::shared::AimPoint) | `vector::aim` — **exists** |
//! | ~~arm markers `Q`/`E`~~ | [`arm_aim`] | — | **RETIRED 2026-09-01** (§5E-c, FIND-227): the X is the only centre element |
//! | objective line, top centre | [`objective`] | `KillTally`, `State<MissionPhase>` | `mission` — **exists since 2026-08-10** |
//! | hit mark, above the crosshair | [`hit_mark`] | [`TitanHit`](crate::shared::TitanHit) | `blades::cut` — **exists**; the element is `F-043` and landed 2026-08-19 |
//! | **search band**, level with the crosshair | [`catch_band`] | [`PlayerSettings`](crate::shared::PlayerSettings) | the player's own slider — the element is `F-016` and landed 2026-08-19 |
//!
//! The objective line is the one edge this domain has out of `bevy`, `shared` and `data`:
//! `hud -> mission`, read-only, with its reason on the allow list in `docs/architecture.md`.
//! It is there because `docs/PLAN-GAME.md` §1 counts the counter and the word `WON` as part of
//! "playable", and both are **state** — a message that fired three ticks ago cannot say what
//! the count is in the frame being drawn.
//!
//! **The failure this module is built against has a name: "the bar that is a picture of a
//! bar"** (`docs/PLAN-GAME.md` §8, F-170) — every element of the list present, and three of
//! them showing a hard-coded number because nothing produces the real one yet. So the rule
//! here is: an element whose producer is missing queries it as `Option` and **hides itself**.
//! An empty screen corner is honest; a full bar that means nothing is not, and it is the kind
//! of lie that survives three rounds because it photographs well.
//!
//! ## The colours are not a free choice
//!
//! Cyan, amber and crimson are the three signal colours of `docs/conventions.md` §3, and they
//! appear nowhere else in the game. They are read from `assets/data/maps.ron`'s `signals:`
//! block through [`signal`] — **never written as a literal here**. That is the whole reason
//! they were made data.
//!
//! ## The middle of the screen stays free
//!
//! No node this module spawns may intersect the central 20 % × 20 % of the screen
//! ([`KEEP_OUT_LOW_PCT`]..[`KEEP_OUT_HIGH_PCT`] on both axes). That is what makes the
//! crosshair four ticks around a hole instead of one node — `tests/hud.rs` computes every
//! element's rect out of `ComputedNode` and falls over if one of them creeps inward.
//!
//! **One exception**: the [`catch_band`] — the aim assist's search extent, drawn level with
//! the crosshair because that is where the sweep looks (`docs/FINDINGS.md` FIND-133: a 1D
//! screen-horizontal line, 0.000006° of vertical deviation). Its position **is** an angle, and
//! its whole range lives inside the box: at 1280 × 720 it reaches 227 px from centre at
//! `assist_catch` 100 % but only 88 px at 40 %, against a box edge at 128 px. Held out of the
//! box it would draw a **wider** search than the one running for every setting below about
//! 55 % — FIND-129's lie with a different number on it. It gives up
//! [`arm_aim::SIGHT_CORE_PX`] instead, and it gives it up by **not drawing**: a tick stands on
//! its ray or it is absent, never moved.
//!
//! **There is no second case any more.** Two elements used to be: `anchor_marks` (twelve rings
//! on authored points, deleted 2026-08-28 — *„es soll auf jeglicher oberflqche einhaken. nicht
//! an hardcoded punkten etc!"*, B-011 WONT FIX) and the two [`arm_aim`] markers with a place
//! (FIND-098, FIND-129 — retired whole on 2026-09-01, §5E-c / FIND-227: *„die kreise können
//! ganz weg!"*). The crosshair is measured by its own stronger claim
//! (`tests/hud.rs::the_x_crosshair_hugs_the_centre_and_keeps_the_aim_pixel_free`), and
//! everything else — bars, pips, banner, panels — obeys the full box.
//! `tests/hud.rs::f171_the_centre_carries_nothing_but_the_x` asserts the whole sentence:
//! nothing but crosshair nodes touches the middle band, and no `hud_arm_*` node exists.
//!
//! ## The evidence
//!
//! | what | run → picture |
//! |---|---|
//! | the five elements over a populated frame, tank at 82 % | `scripts/f170-hud.txt` → `docs/images/f170-hud.png` |
//! | the three crosshair states, three crops | `scripts/f171-crosshair.txt` → `docs/images/f171-crosshair.png` |
//! | the objective going `0/3` → `1/3` | `scripts/f170-objective.txt` → `docs/images/f170-objective-before.png`, `docs/images/f170-objective.png` |
//! | the verdict, `WON` over the cleared field | `scripts/game-full.txt` → `docs/images/f071-won.png` |
//! | ~~the arm markers / landing preview~~ | five rows of decoded evidence retired with the element, 2026-09-01 (§5E-c, FIND-227) — in history: `git log -- src/hud/mod.rs` |
//! | **`F-043`, the kill** — `KILL  21.0 m/s` in amber over the husk whose nape was just cut | `scripts/f032-swords.txt --ticks 162` → `docs/images/f043-hit-mark-kill.png` |
//! | the same run's **body cut**, smaller and crimson | `… --ticks 331` → `docs/images/f043-hit-mark-cut.png` |
//! | and the same band 43 ticks after the kill — **empty** | `… --ticks 200` → `docs/images/f043-hit-mark-gone.png` |
//! | **`F-016`, the search band at three settings** — no band, 88 px, 227 px, one stand, one look | `scripts/f016-band.txt --ticks 150 / 240 / 330` → `docs/images/f016-band-0.png`, `-40.png`, `-100.png` |
//!
//! The hit mark was decoded out of the three frames above rather than against a control run:
//! in the band `y 195..250, x 320..960` the kill accounts for **1 401 saturated pixels** in a
//! box of `x 516..763, y 206..232` at a mean sRGB of (237, 202, 92) — `maps.ron`'s amber; the
//! body cut for **698** in `x 555..723, y 204..223` at (224, 106, 115) — its crimson; and the
//! frame at tick 200, 43 ticks after a mark that holds 33, for **0**. Different word, different
//! size, different colour, and it takes itself away. Both boxes end at `y 232` against a
//! keep-out box that starts at `y 288` — 56 px of measured margin.
//!
//! The search band was decoded against its own 0 % frame, which is the control that costs
//! nothing here: the same stand, the same look, the same tick, and the only difference is the
//! knob. At `assist_catch` 100 % it accounts for 1 148 changed pixels running `x 412..867` —
//! 228 px left and 227 px right of a crosshair at 640, against the 226.9 px that 20° through a
//! 60° lens projects to on a 1280 px screen — in exactly **two** connected runs, `412..633` and
//! `646..867`, with the 12 px gap between them being the `SIGHT_CORE_PX` the element gives up.
//! At 40 % the same rows account for `x 551..728` (89 px / 88 px) and the eight ticks a side
//! stand at ±11, 22, 33, 44, 55, 66, 77, 88 px — evenly, because a tangent is linear at 8°;
//! at 100 % they stand at ±27, 55, 82, 110, 138, 167, 197, 227 px — unevenly, because it is not
//! at 20°. The 0 % frame has **zero** changed pixels in the band's rows, which is the
//! *"no search, no band"* half of the claim measured rather than asserted.
//!
//! The objective line was decoded the same way the cyan was, against a control run of the same
//! script **without `--mission`** — no mission, no `KillTally`, no line, everything else in the
//! frame the same. In the top-centre band it accounts for 229 changed pixels in
//! `f170-objective.png` and 1 633 in `f071-won.png`, 668 of those exactly the amber out of
//! `maps.ron` (sRGB 255, 215, 89); the control has **zero** in both. And between the `0/3` and
//! the `1/3` shot of one script exactly **9 × 13 px** change — one glyph cell, the leading
//! digit. That is the sentence in `docs/PLAN-GAME.md` §1, measured.
//!
//!
//! Both were decoded, not assumed: against a control run with this plugin switched off, the
//! HUD accounts for 6 368 changed pixels in `f170-hud.png`, 4 500 of them exactly the cyan out
//! of `maps.ron` (sRGB 63, 237, 249) — and the control has **zero**. The gas bar measures
//! 227 px of a 277.6 px track = 81.8 %, next to an F3 overlay reading `gas 82/100` in the same
//! frame. The three crosshair crops measure 302 × 178, 326 × 202 and 356 × 212 px — the same
//! three tuples `tests/hud.rs` asserts, to the pixel.

pub mod arm_aim;
pub mod blade_pips;
pub mod board;
pub mod career;
pub mod catch_band;
pub mod crosshair;
pub mod gas_bar;
pub mod health_bar;
pub mod hit_mark;
pub mod objective;

use bevy::prelude::*;

use crate::data::GameData;
use crate::menu::Screen;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                gas_bar::spawn_gas_bar,
                blade_pips::spawn_blade_pips,
                health_bar::spawn_health_bar,
                objective::spawn_objective,
                board::spawn_board_panel,
                career::spawn_career_panel,
                crosshair::spawn_crosshair,
                catch_band::spawn_catch_band,
                hit_mark::spawn_hit_mark,
            ),
        )
        .add_systems(
            Update,
            (
                gas_bar::update_gas_bar,
                blade_pips::update_blade_pips,
                health_bar::update_health_bar,
                objective::update_objective,
                // `F-177`. It reads two answers other people already gave — `menu::board::Board`
                // for "is he at the board" and `menu::lobby::chosen` for "which sortie" — and
                // measures nothing itself. That is the corollary of rule 5 and it is the whole
                // reason this element is four lines of decision and no geometry.
                board::update_board_panel,
                // `F-120`/`F-121`/`F-122`. The other half of `menu::debrief`'s plate: a run
                // with no window has no plate at all, so this is the only surface that can
                // ever report a sortie to a screenshot (`FIND-189`). It formats nothing —
                // `progress::ledger` owns the words, exactly as `menu::lobby::entries` owns
                // the mission list the element above draws.
                career::update_career_panel,
                // `F-043`: sense, then show — the same two-step split the crosshair
                // uses, and for the same reason. What a hit *is* (word, size,
                // colour) is then testable against a `HitFlash` set by hand, without a titan,
                // a blade and a swept cast having to be arranged first.
                (hit_mark::sense_hit_mark, hit_mark::show_hit_mark).chain(),

                // Sense, then shape, then paint — **three systems on purpose**: the shape is
                // then testable against a state somebody set by hand, without a titan, a
                // physics world and a look direction having to be arranged first, and the
                // colour can be neutralised in the test without touching the geometry.
                (
                    crosshair::sense_crosshair,
                    crosshair::shape_crosshair,
                    crosshair::paint_crosshair,
                )
                    .chain(),
            ),
        )
        // **The one HUD system that is not in `Update`**, and it has to be here. It projects
        // world points through the camera, so it needs the camera's `GlobalTransform` (written
        // in `TransformSystems::Propagate`) and its viewport size (written in
        // `CameraUpdateSystems`) — both are `PostUpdate`. Placed in `Update` it would draw one
        // frame behind the image: invisible standing still, very visible mid-swing.
        // (Until 2026-09-01 `arm_aim::place_arm_aim` and the `DBT_AIMTRACE` trace ran here
        // too — retired with the markers, §5E-c / FIND-227.)
        .add_systems(
            PostUpdate,
            catch_band::place_catch_band
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::CameraUpdateSystems)
                .before(bevy::ui::UiSystems::Layout),
        )
        // **`PostUpdate`, and that is not a detail.** `menu::toggle_screen` writes `Screen` in
        // `Update`, and two systems in one schedule have no order between them: placed in
        // `Update` this would see the old screen roughly half the time and hide the HUD one
        // frame late — a visible flash of a crosshair across the menu on every `Esc`. Here it
        // runs after every writer of `Screen` and before the frame's visibility is propagated,
        // so the menu and the HUD change in the same image.
        .add_systems(
            PostUpdate,
            hide_while_a_menu_is_up
                .run_if(resource_changed::<Screen>)
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
}

/// **The HUD is the game's overlay, and a menu is not the game.**
///
/// It was measured drawing over all three screens on 2026-08-13 — the crosshair straight down
/// the middle of the pause column, the objective counter, the gas bar, the blade pips and the
/// two arm markers at button height (FIND-092 §4, `docs/images/f175-pause.png`). Freezing
/// `Time<Virtual>` stops the *simulation*; nothing stopped the drawing.
///
/// ## Two rules this obeys, and they are the reason it is four lines
///
/// **It writes `Visibility` and never `Node.display`.** Every element in this domain owns its
/// own `display` — that is how `health_bar`, `blade_pips`, `objective`, `crosshair` and
/// `arm_aim` hide themselves when their producer is missing — and a second writer of that
/// field would be exactly the breach §6 rule 3 forbids. `Visibility` on the **roots** is a
/// field nothing else in this domain touches, it propagates to every child by itself, and it
/// leaves the pixel-exact `F-170`/`F-171` claims about what is drawn *while playing*
/// untouched. `tests/menu.rs::f175_the_hud_is_hidden_while_a_menu_is_up` asserts both halves.
///
/// **It runs on a change and not per frame** (§6 rule 6): `Screen` changes when somebody
/// presses a key or a button, which is a handful of times per session.
///
/// ## And one exception, which is [`ShowWhileTuning`]
///
/// The rule that survives is *"gameplay clutter does not sit over a menu"*. On
/// [`Screen::Settings`] the search band and the crosshair are not clutter — they are the
/// picture of the number the `Aim assist reach` row writes, and a picture you can only look at
/// after closing the screen that changes it is worth nothing (`docs/FINDINGS.md` FIND-136).
/// The pause plate and the lobby get no exception: neither can move the knob.
fn hide_while_a_menu_is_up(
    screen: Res<Screen>,
    mut roots: Query<
        (&mut Visibility, Option<&ShowWhileTuning>),
        (With<HudElement>, Without<ChildOf>),
    >,
) {
    let playing = *screen == Screen::Playing;
    let tuning = *screen == Screen::Settings;
    for (mut visibility, exempt) in &mut roots {
        // The rule and its one exception, on one line: the game's overlay belongs to the game,
        // **except** for the element that IS the number the screen in front of it edits.
        let want = if playing || (tuning && exempt.is_some()) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        // Compared before it is written: `Visibility` is change-detected and a blind write
        // would re-run the propagation over the whole HUD for a value that did not move.
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// **The exception to [`hide_while_a_menu_is_up`], and it is exactly one screen wide.**
///
/// The rule the HUD obeys is *"gameplay clutter does not sit over a menu"* — a crosshair down
/// the middle of the pause column, an objective counter and two bars over the lobby, all
/// measured on 2026-08-13 (FIND-092 §2). This marker names the one case that is not clutter:
/// **the element that IS the setting being edited**. The player opened
/// [`Screen::Settings`](crate::menu::Screen::Settings) to move a number, and an element that
/// answers that number in the tick it moves is worth nothing if it can only be looked at after
/// the screen is closed — which is the whole reason the user asked for it
/// (*„damit man das besser einstellen kann"*, `docs/FINDINGS.md` FIND-135).
///
/// **Two elements carry it and no more**: [`catch_band`] — the search extent, which is the
/// number the `Aim assist reach` row writes — and [`crosshair`], because the band is a ruler
/// **measured from the crosshair** and a ruler with no origin cannot be read. The bars, the
/// pips, the objective line and the hit mark report the *fight*, and there is
/// no fight while a menu is up; they stay hidden.
///
/// It buys nothing on the pause plate or in the lobby, so it is not given there: neither screen
/// can change the number, and a band nobody can move is the clutter this rule exists against.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ShowWhileTuning;

/// The stacking layer of a [`ShowWhileTuning`] element — **above the menu backdrop**.
///
/// Not cosmetic and not a preference: `menu::plate::root` is a full-screen node at the default
/// global z of 0 and it is spawned **after** the HUD, so without this the band would sit under
/// `plate::BACKDROP`'s 0.90 alpha. That is not "dimmer", it is gone: composited in linear light
/// the band's own white falls to sRGB 24 against a background of 14 — **1.05:1**, where over
/// the backdrop it is 10.4:1 (`tests/menu.rs::f016_the_band_reads_over_the_settings_backdrop`).
///
/// Every other HUD element stays at 0. These two never overlap another HUD element — the
/// keep-out box and [`arm_aim::SIGHT_CORE_PX`] are what keep them apart — so lifting them
/// changes nothing about a playing frame, and `F-170`/`F-171`'s pixels are untouched.
pub const TUNING_Z: i32 = 1;

/// On **every** node this domain spawns — containers included.
///
/// `tests/hud.rs` walks exactly this marker to compute the keep-out rectangle. A container
/// without it would be a hole in that test, so the rule is: whatever gets a [`Node`] here
/// gets a `HudElement`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HudElement;

/// The forbidden box in the middle: 20 % of the width by 20 % of the height.
///
/// **Sized to the crosshair's arms, not to the target under them** — it is what makes
/// [`crosshair`] four ticks around a hole. The distinction matters since FIND-098: the one
/// element whose position is an *angle* rather than a *place* is measured against
/// [`arm_aim::SIGHT_CORE_PX`] instead, because the whole angular range it can occupy fits inside
/// this rectangle and being pushed out of it destroyed the reading. Widening this constant is
/// therefore free for chrome and costs nothing for the fan; **narrowing it is what would break
/// the crosshair**, and that is why the fix was an exemption and not a smaller box.
pub const KEEP_OUT_PCT: f32 = 20.0;
/// Left/top edge of the forbidden box, in percent of the screen.
pub const KEEP_OUT_LOW_PCT: f32 = 50.0 - KEEP_OUT_PCT * 0.5;
/// Right/bottom edge of the forbidden box, in percent of the screen.
pub const KEEP_OUT_HIGH_PCT: f32 = 50.0 + KEEP_OUT_PCT * 0.5;

/// A signal colour out of `assets/data/maps.ron`, `signals:`.
///
/// **Panics on a missing key, deliberately** — the same choice `titan::rig` makes for the
/// cortex. A grey stand-in for "amber" would put a HUD on screen that quietly stops obeying
/// `docs/conventions.md` §3, and nothing would ever say so. `serde(default)` is forbidden for
/// game values for the same reason (`CLAUDE.md` rule 2).
///
/// The triples are **linear** RGB, as everywhere else in `maps.ron` — hence
/// `Color::linear_rgb` and not `srgb`. The two differ by a factor of about two in the
/// mid-tones, which is exactly enough to make a cyan look like a different cyan next to the
/// gizmos.
pub fn signal(data: &GameData, name: &str) -> Color {
    let (r, g, b) = data.maps.signals.get(name).copied().unwrap_or_else(|| {
        panic!(
            "maps.ron `signals:` has no key {name:?} — there is nothing to paint the HUD \
             with. The three keys are cyan, amber, crimson (docs/conventions.md §3)"
        )
    });
    Color::linear_rgb(r, g, b)
}

/// The dark plate a bar sits on, and the empty half of every bar.
///
/// **Not a signal colour and deliberately not from `signals:`** — it is the same black-with-
/// alpha the F3 overlay uses (`debug::spawn_overlay`), and its only job is to keep a cyan bar
/// readable over sky as well as over asphalt. Putting it in `maps.ron` would suggest it is a
/// balancing value; it is a legibility constant.
pub const PLATE: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
