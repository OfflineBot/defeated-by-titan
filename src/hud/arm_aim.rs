//! **The landing preview: one marker per arm, `Q` left and `E` right, standing on the point in
//! the world that arm is aimed at.**
//!
//! The user asked for the element on 2026-08-10 (*"und es muss auch visuell immer 2 punkte
//! angezeigt werden so der e und q haken hingehen würden!"*) and then rejected the first
//! answer on 2026-08-12, correctly:
//!
//! > *"es soll previewd werden wo der aktuelle haken landen würde! also sollte richtig angezeigt
//! > werden. nicht nur am fadenkreuz. weil das stimmt auch nicht."*
//! > *"zudem sollen diese weiter auseinander sein. also weiter rechts und links!"*
//!
//! He was describing `FINDINGS.md` FIND-047 from the outside: the two markers were pinned at
//! `top 65 %` / `left|right 52 %` and were photographed **at the same pixels across four runs
//! with four different aims**. They were state badges wearing a location's clothes.
//!
//! # Where a marker stands, and why it cannot be anywhere else
//!
//! [`place_arm_aim`] projects **that arm's own** landing point through the real camera
//! (`Camera::world_to_viewport`) and puts the marker there. The point comes out of
//! [`ArmAim`](crate::shared::ArmAim), which `vector::aim` fills from that arm's own side ray and
//! `vector::hook` fires at **without re-casting** — so the marker and the rope read the same
//! `Vec3` in the same tick. `tests/hud.rs::f026_the_marker_stands_exactly_where_that_arm_fires`
//! asserts that with `assert_eq!` and no tolerance, and
//! `f026_the_rope_flies_at_the_point_the_marker_stood_on` fires the hook and compares
//! `HookState::Flying { target_m }` against the same value. That pair is the user's
//! *„und dann muss das seil auch dahin!!"* (2026-08-12) as a test, and it is the only reason to
//! believe the sentence.
//!
//! An arm whose tip is out (`Flying`, `Retracting`, `Anchored`) previews **its tip** instead —
//! the point `render::rope` draws to, so marker and rope cannot disagree there either.
//!
//! ## What an idle marker says, and what it cannot say
//!
//! A side ray is a **fixed direction relative to the camera** (`vector::aim::side_dirs` yaws the
//! look direction by ±`aim_spread_deg` around the camera's up axis), and a fixed direction
//! projects to a fixed pixel: `aim_spread_deg` off the crosshair, at the crosshair's height. Two
//! consequences, both measured rather than argued:
//!
//! - **The distance to the hit is not in the picture.** A projection has no depth; a wall at 6 m
//!   and a roof at 300 m along the same ray are the same pixel. What the marker promises is the
//!   *bearing* the rope takes, and that promise is exact.
//! - **A side ray that finds nothing anchorable falls back to the centre ray** (`vector::aim`),
//!   which lands on the crosshair —
//!   `tests/hud.rs::f171_a_free_aim_point_projects_onto_the_crosshair` measures 0.000 px, because
//!   `vector::aim` starts at `translation + Y·eye_height_m` and `render::attach_camera` hangs the
//!   camera on exactly that offset. So a pair that collapses onto the middle is not a bug, it is
//!   the reading *"neither side found anything of its own"*.
//!
//! # Off the screen, and behind you
//!
//! `Camera::world_to_viewport` hands back a **usable** pixel for a point beside the viewport: NDC
//! x/y outside ±1 is not an error, only z outside `0..1` is
//! (`bevy_camera-0.19.0/src/camera.rs:546-556`). The ordinary off-screen case therefore needs
//! nothing but step 2 of [`layout_for`]. What it refuses is a point **behind the near plane**, and
//! that is not an edge case here: a swing spends half of its arc with the anchor behind the
//! player.
//!
//! Such a marker is **clamped to the screen edge on the side the point really is**
//! ([`edge_pixel`]), not hidden. Hiding it would blink the pair out exactly during the manoeuvre
//! it exists for, and *which side the rope pulls from* is what the player needs mid-swing. The
//! price of that choice, written down instead of hidden: an edge marker says *"that arm's point
//! is over there"* and does **not** separate "just off the edge, in front" from "behind you".
//! Distinguishing them would need a fifth glyph, and the fifth glyph would have to carry
//! `F-026`'s colour-blindness clause too (`docs/FINDINGS.md` FIND-087 §3).
//!
//! # Why the middle of the screen survives a world-tracked marker
//!
//! `F-170` keeps the central 20 % × 20 % free ([`KEEP_OUT_LOW_PCT`]..[`KEEP_OUT_HIGH_PCT`]) and
//! `tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen` is a proven 🟧 claim. A marker
//! that follows a world point **will** eventually be aimed straight at, so the guard cannot be
//! left to luck. [`layout_for`] keeps that marker on its own pixel and steps it out of
//! [`SIGHT_CORE_PX`] instead (FIND-129, below); a marker with **no** point at all is pushed
//! whole — glyph, tether and letter — out of the box towards the side it belongs to, so it never
//! claims the wrong half of the screen. Ties (a point dead on the axis) go to the arm's own side.
//! Step 3 is applied **last**, after the screen clamp: on a viewport small enough for the two to
//! disagree, the proven claim wins and a few pixels of the marker leave the screen instead.
//!
//! ## …and why an idle arm's marker is exempt from it (`docs/FINDINGS.md` FIND-098)
//!
//! **That rule, applied to the fan, made the HUD lie.** On 2026-08-18 `vector::aim` learned to
//! resolve the spread from what the player is doing instead of handing back the wheel, and the
//! resolved half-angle is small: at the shipped wheel 28 it is at most **9.594°** grounded,
//! **11.212°** airborne, **3.716°** tethered. On the design aspect that is |NDC_x| 0.165 / 0.193
//! / 0.063, and the box edge is 0.2 — so *every* fan, in *every* state, at *every* distance and
//! *every* wheel notch below the top, touched the box and was pushed to the same fixed slot at
//! 146 px. The player paid for a 65 % narrower fan and the picture did not move a pixel. `F-023`'s
//! whole claim is that the rope and the marker are **one number**; a marker that cannot move is
//! not that number.
//!
//! So step 3 now asks **what the marker's x means** ([`Bearing`]) rather than where it landed:
//!
//! - [`Bearing::World`] — a tip in flight, an anchor being held, or an arm that fell back to the
//!   centre ray. Its glyph stands on that point's own pixel and steps out of [`SIGHT_CORE_PX`]
//!   **vertically, towards the nearer edge**, because both of its axes carry a place.
//! - [`Bearing::Fan`] — an idle arm on its **own** side ray. The glyph's x *is* the resolved
//!   half-angle. It keeps that angle exactly — **at every field of view** — and gives up only
//!   [`SIGHT_CORE_PX`], the little square the player is actually cutting, by stepping *down* out
//!   of it. It may stand inside the box; it may not stand on the target.
//!
//! A marker with **no** point at all — the side ray found nothing and the centre ray it falls
//! back to found nothing either — is the one node here that is still chrome: it claims nothing
//! about the world, it parks in its side slot, and it obeys the full box.
//!
//! ## …and the FIND-098 argument had a second half that was wrong (`docs/FINDINGS.md` FIND-129)
//!
//! FIND-098 exempted the fan and left [`Bearing::World`] inside the box, on the argument that
//! `render::rope` is already drawing the rope to that point so the box costs nothing, and that a
//! fallback marker is a *state badge* with no position of its own (FIND-087 §2). **Swept in the
//! running app on 2026-08-19 that cost 150.0 px and 47.7 m.** The place a player aims at is the
//! middle of his screen by construction, so the box fired on the common case and not on the
//! corner one: over three stands, nine look angles, five assist settings and all four arm states,
//! **400 of 469 world-bearing samples were drawn somewhere the rope does not go** — and 16 of
//! them were idle fallbacks, where no rope is on screen at all and the glyph was the player's
//! only reading of where the hook would land.
//!
//! The user, 2026-08-19: *„wichtig wäre nur dass diese auch genau da sind visuell wo das seil
//! auch landen würde!"* So the box lost that case. `F-170`'s claim is unchanged for everything it
//! was written against — bars, pips, banner, letters, the crosshair and a marker with nothing to
//! say — and what a marker carrying a place keeps clear is [`SIGHT_CORE_PX`], the pixels the
//! blade is aimed at.
//!
//! **What the box loses, measured rather than argued:** an idle glyph is 20 px wide, and the
//! narrowest angle the model can reach (`aim_spread_floor_deg` 2°) projects 21.8 px off centre on
//! a 1280 px screen at the file's 60° FOV — so the glyph's inner edge is 11.8 px clear of the aim
//! pixel at the *worst* angle. The crosshair, the bars, the pips, the banner and the letters are
//! all still held out of the full box; nothing that F-170 was written against moved.
//!
//! **And the FOV is a live setting** (`shared::PlayerSettings`, 55..110°, `docs/FINDINGS.md`
//! FIND-099): at 110° that same 2° projects **8.8 px**, so a guard that clamped the *x* would
//! turn the whole band below 3.63° back into a fixed slot for any player who widened his view.
//! That is why the sight-core guard moves the **y**, which for a fan marker is the crosshair's y
//! at every angle and carries nothing, and never the x, which carries everything.
//!
//! **For a [`Bearing::World`] marker the rule does not bend and the marker does not fade**, and
//! the reason is that the price is bounded and the case is degenerate. It is a *slide to the box
//! edge*, never a jump: the displacement is at most half the box (128 px of 1280) and it grows
//! continuously as the point walks in, so nothing teleports.
//!
//! ⚠️ **The one case where that costs truth is written down and not hidden**: an arm whose side
//! ray found nothing falls back to the centre ray, and the centre ray projects into the box — so
//! the marker parks at the edge while the rope will fly at the crosshair. In that one case the
//! marker is a *state badge*, not a location, and the reading is "this arm has no point of its
//! own". `docs/FINDINGS.md` FIND-087 §2 has the measurement, the photograph and the three
//! alternatives that were weighed against it. And a point inside the box is, for an idle arm, exactly the
//! fallback case above — that arm found nothing of its own and is firing at the centre ray, which
//! the crosshair is already standing on. Fading the marker there would take away the state
//! (`Ready` against `Free`) at the moment the player is about to press the key; the whole element
//! is `F-026` *"immer sichtbar"*, and `tests/hud.rs::f170_the_arm_markers_stay_out_of_the_middle_in_every_state`
//! counts the nodes so a "fade" cannot quietly become a disappearance.
//!
//! # Four shapes, and the colour carries nothing
//!
//! | [`ArmAimState`] | what it means | glyph | tether | nodes |
//! |---|---|---|---|---|
//! | [`Free`](ArmAimState::Free) | idle, and the ray finds nothing hookable | flat dash | — | 1 |
//! | [`Ready`](ArmAimState::Ready) | idle, and this arm would catch | ring | — | 1 |
//! | [`Busy`](ArmAimState::Busy) | the tip is out: flying or coming home | wide ring | — | 1 |
//! | [`Anchored`](ArmAimState::Anchored) | this arm is holding | filled disc | yes | 2 |
//!
//! The four bounding boxes are four different `(node count, width, height)` tuples, so
//! `tests/hud.rs::f171_the_two_arm_markers_differ_in_shape_not_only_in_colour` can force every
//! `BackgroundColor` **and** every `BorderColor` to one value and still tell them apart. That is
//! `F-171`'s rule and `F-026`'s acceptance, and it is the only way either is falsifiable.
//!
//! # One field, one writer — inside a single component
//!
//! `Node` is now written by two systems, and they are split **by field**, not by entity:
//! [`shape_arm_aim`] owns `width`, `height`, `border`, `border_radius` and `display`;
//! [`place_arm_aim`] owns `left`, `top`, `right`, `bottom` and `margin`. Neither ever reads back
//! what the other wrote. Written as two whole-`Node` assignments they would overwrite each other
//! every frame and change detection would fire 60 times a second on a standing player — the exact
//! failure `docs/lessons/performance.md` rule 1 is about.
//!
//! # What it costs per frame
//!
//! **No ray and no spatial query.** [`sense_arm_aim`] reads [`Hook`] and
//! [`ArmAim`](crate::shared::ArmAim) off the local player — both already written this tick by
//! `vector`, and the three rays behind them are cast once there, not again here — and
//! [`place_arm_aim`] adds two
//! matrix multiplications, one per arm. Every write is guarded by a comparison, so a standing
//! player with a still camera produces zero writes (`CLAUDE.md` rule 6).

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::crosshair::NEUTRAL;
use crate::hud::{signal, HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use crate::shared::{
    AimPoint, ArmAim, Hook, HookReleased, HookState, LocalPlayer, MissReason, PlayerId,
    ReleaseReason, Side,
};

/// Which arm this node belongs to, and which of its two nodes it is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmMarker {
    pub side: Side,
    pub part: MarkerPart,
}

/// The glyph is the marker; the tether is the short stem that only the anchored state shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerPart {
    Glyph,
    Tether,
}

/// What one arm would do if you pressed its key right now.
///
/// Lives on the marker nodes and not on the player and not in a `Resource`: it is view state of
/// the local client, it has exactly one writer, and player state in a resource is the first step
/// this project forbids (`docs/multiplayer.md` rule 2). Same choice as
/// [`CrosshairState`](crate::hud::crosshair::CrosshairState), for the same reason.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArmAimState {
    /// The arm is idle and the aim ray ends somewhere no hook can take.
    #[default]
    Free,
    /// The arm is idle and would catch. Press the key and it flies.
    Ready,
    /// The tip is not in the hand — flying out or being reeled back. **The key does nothing**
    /// until it is home again (`vector::hook`, decision 1: a shot only leaves from `Idle`).
    ///
    /// Flying and retracting are one state on purpose: what the player has to know here is that
    /// this arm is not available, and the rope in the world says which of the two it is.
    Busy,
    /// The arm is holding.
    Anchored,
}

/// The pure rule: one arm's hook state plus "is the shared aim point hookable" → one shape.
///
/// A free function, so the rule is testable without a physics world, a camera and a look
/// direction having to be arranged first.
pub fn state_for(hook: &HookState, anchorable: bool) -> ArmAimState {
    match hook {
        HookState::Idle => {
            if anchorable {
                ArmAimState::Ready
            } else {
                ArmAimState::Free
            }
        }
        HookState::Flying { .. } | HookState::Retracting => ArmAimState::Busy,
        HookState::Anchored { .. } => ArmAimState::Anchored,
    }
}

/// **Why the last pull of this arm found nothing, while it is still worth saying** (`F-028`).
///
/// The user, 2026-08-18: *„teilweise kann man gar nicht usen weil keine ahnung wieso."* Before
/// today a pull that found nothing wrote one `info!` line into a log he never reads and left the
/// marker exactly as it was — the same dash, the same grey, no sound. `F-028`'s acceptance is
/// his sentence turned around: *"Kein Tastendruck bleibt ohne Rueckmeldung."*
///
/// **A countdown and not a flag**, because the thing being reported is an *event* and the marker
/// is a *state*: the word has to appear at the moment the key goes down and then get out of the
/// way, or the HUD ends up asserting something about the world that stopped being true. It lives
/// on all three nodes of an arm so that [`paint_arm_aim`] can colour the glyph and
/// [`show_arm_miss`] can write the letter without either of them reading the other's field.
///
/// **One writer:** [`sense_arm_miss`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmMiss {
    /// `None` = this arm has nothing to report, and the marker says what it always said.
    pub reason: Option<ArmHint>,
    /// Seconds of hint left. Counted down on the **generic** `Time`, not `Time<Fixed>`: this is
    /// view state on a frame-rate clock, and a hint that lived in fixed ticks would flicker at a
    /// frame rate the simulation does not share.
    pub left_s: f32,
}

/// How long the word stays. A legibility constant and not a game value — the same argument
/// [`ArmShape`] makes: `CLAUDE.md` rule 2 names "a titan kind, a blade tier, a gas cost", and
/// how long a caption is readable is none of the three.
///
/// 1.6 s is two comfortable reads of two words at arm's length, and it is under the time a
/// player takes to line up a second shot — so the hint never outlives the situation it
/// describes.
pub const MISS_HINT_S: f32 = 1.6;

/// **The four words**, one per [`MissReason`], and each of them tells the player to do a
/// different thing.
///
/// Short on purpose: this sits next to a 20 px glyph in the corner of his eye while he is
/// falling, and a sentence there is a sentence nobody reads. The long form is in the log
/// ([`MissReason::explains`]) and that is where a run's evidence comes from.
///
/// | word | what he should do |
/// |---|---|
/// | `NO TARGET` | turn — that line is empty all the way out |
/// | `TOO FAR` | come closer — there is something, past your reach |
/// | `WONT HOLD` | aim past it — including past a titan (`B-007`) |
/// | `NO ANCHOR` | nothing he can do; the world is at fault (`B-001`) |
/// | `ROPE TORN` | nothing he can do either — but for the opposite reason: he had a good anchor and it died under him (`F-029`) |
pub const fn miss_label(reason: ArmHint) -> &'static str {
    match reason {
        ArmHint::Miss(MissReason::NothingInRange) => "NO TARGET",
        ArmHint::Miss(MissReason::OutOfReach) => "TOO FAR",
        ArmHint::Miss(MissReason::SurfaceHoldsNothing) => "WONT HOLD",
        ArmHint::Miss(MissReason::NoCarrier) => "NO ANCHOR",
        ArmHint::CarrierGone => "ROPE TORN",
    }
}

/// **Everything one arm can have to tell the player, in one type.**
///
/// Until 2026-08-19 this was a bare [`MissReason`], and that shape was the bug: `F-028` built
/// the whole channel — the countdown, the crimson, the word under the letter — for the *pull*
/// that finds nothing, and [`sense_arm_miss`] matched `ReleaseReason::NoAnchor(_)` and nothing
/// else. So [`ReleaseReason::BodyGone`] went past in complete silence: the titan you were
/// hanging on dies, `vector::hook` drops the rope in that same tick
/// (`tests/titan.rs::f029_the_rope_lets_go_when_the_titan_dies_and_says_why`), and the marker
/// went back to grey as if you had let go of the button yourself. `F-029`'s acceptance is
/// explicit that this is half the feature — *"loest sich beim Tod des Titanen **mit
/// Feedback**"*.
///
/// **One type and not a second channel.** The alternative was another component with another
/// countdown and another colour, and two ways of saying *"this arm has nothing"* drift apart
/// within a week — which is the argument [`MissReason`] itself makes for riding on
/// [`ReleaseReason`] rather than travelling on a message of its own.
///
/// The two are kept apart as *variants* rather than merged into a fifth [`MissReason`] because
/// they are not the same kind of fact. A `MissReason` is about the shot you just took; this is
/// about a rope you were already holding. `MissReason` is `shared`'s, it goes over a wire, and
/// `vector::hook` produces it — none of which is true of a HUD's word for "your anchor died".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArmHint {
    /// A trigger pull that found nothing (`F-028`). The four reasons of [`MissReason`].
    Miss(MissReason),
    /// **The thing you were holding is gone** (`F-029`): the titan died, or the area unloaded.
    /// Not your mistake, and the word says so — the rope tore, you did not miss.
    CarrierGone,
}

/// **The world point this arm's marker stands on**, or `None` when it has none.
///
/// An idle arm stands on [`ArmAim::target_of`](crate::shared::ArmAim::target_of) — **this arm's
/// own side ray**, the exact value `vector::hook::fire` turns into
/// `HookState::Flying { target_m }` when the key is pressed. Not the shared centre
/// [`AimPoint`](crate::shared::AimPoint): that one is the crosshair's, and a marker fed from it
/// would be drawing a promise the rope does not keep (`docs/FINDINGS.md` FIND-047 is the day that
/// promise was measured and found broken).
///
/// The three states with a tip out take [`HookArm::tip_m`](crate::shared::HookArm), which
/// `vector::hook` walks along on every tick and `render::rope` already draws to — so the marker
/// and the rope cannot disagree about where the arm is holding either.
///
/// `None` means this arm has nothing at all: the side ray found nothing and the centre ray it
/// falls back to found nothing either. Then the marker goes to its side slot.
pub fn target_of(hook: &Hook, aim: &ArmAim, side: Side) -> Option<Vec3> {
    let arm = hook.arm(side);
    match arm.state {
        HookState::Idle => aim.target_of(side),
        // **Where it lands, not where it is** (`docs/FINDINGS.md` FIND-099). `vector::hook::fire`
        // freezes the destination into `Flying { target_m }` and starts the tip **in the hand**
        // (its decision 5) — so for the first ticks of every shot the tip sits on the camera's
        // own near plane, `Camera::world_to_viewport` refuses it, and [`edge_pixel`] answers with
        // a bearing that the clamp turns into the edge of the screen. Measured: a marker
        // standing 155 px off centre jumped to **608 px** the frame the key went down, about a
        // target 40 m dead ahead that had not moved, and then crawled back in over the flight.
        //
        // `target_m` is the exact `Vec3` the idle marker was already standing on
        // (`tests/hud.rs::f026_the_rope_flies_at_the_point_the_marker_stood_on`), so this is
        // also the only reading under which firing does not move the preview at all — which is
        // the requirement: *"dass man direkt sieht wo man landet"*.
        HookState::Flying { target_m, .. } => Some(target_m),
        // Retracting and anchored keep the **tip**: there `render::rope` really is drawing to
        // that point, and a retracting arm has no destination to show.
        HookState::Retracting | HookState::Anchored { .. } => Some(arm.tip_m),
    }
}

/// The geometry of one state, in logical pixels.
///
/// **Shape constants, not balancing values** — the same argument
/// [`CrosshairShape`](crate::hud::crosshair::CrosshairShape) makes for the crosshair: `CLAUDE.md`
/// rule 2 names "a titan kind, a blade tier, a gas cost", and the fact that a marker is a ring is
/// none of the three. What would belong in RON is a UI scale factor, and there is none yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmShape {
    pub glyph_w_px: f32,
    pub glyph_h_px: f32,
    /// Border width. `0` means the glyph is filled, anything else means it is an outline — and
    /// outline against filled is a shape difference, not a colour one.
    pub border_px: f32,
    /// Whether the corners are rounded all the way (a ring or a disc) or not at all (a dash).
    pub round: bool,
    /// Length of the stem under the glyph, or `None` when this state has no second node.
    pub tether_px: Option<f32>,
}

/// The one table. Change a number here and the picture and the test move together.
pub const fn shape_of(state: ArmAimState) -> ArmShape {
    match state {
        ArmAimState::Free => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 4.0,
            border_px: 0.0,
            round: false,
            tether_px: None,
        },
        ArmAimState::Ready => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 20.0,
            border_px: 3.0,
            round: true,
            tether_px: None,
        },
        ArmAimState::Busy => ArmShape {
            glyph_w_px: 28.0,
            glyph_h_px: 28.0,
            border_px: 3.0,
            round: true,
            tether_px: None,
        },
        ArmAimState::Anchored => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 20.0,
            border_px: 0.0,
            round: true,
            tether_px: Some(16.0),
        },
    }
}

/// How many nodes a state shows — the first element of the tuple the test compares.
pub const fn node_count(state: ArmAimState) -> usize {
    match shape_of(state).tether_px {
        Some(_) => 2,
        None => 1,
    }
}

/// The key that fires this arm.
///
/// **A second spelling of `src/net/local.rs:84-85`**, where `Buttons::HOOK_LEFT` is bound to
/// `KeyCode::KeyQ` and `HOOK_RIGHT` to `KeyCode::KeyE`. `hud` may not reach into `net` (the
/// allow list in `docs/architecture.md` has no such line), so the letter is written twice — and
/// `tests/hud.rs::f171_the_marker_letters_are_the_keys_that_fire_the_arms` reads that source file
/// back and goes red the day somebody rebinds a hook without coming here. A label that names the
/// wrong key is worse than no label.
pub const fn key_label(side: Side) -> &'static str {
    match side {
        Side::Left => "Q",
        Side::Right => "E",
    }
}

/// Gap between the glyph and its tether.
const TETHER_GAP_PX: f32 = 4.0;
/// Width of the tether stem.
const TETHER_W_PX: f32 = 4.0;
/// The widest glyph any state draws. Used for the one-frame placeholder at spawn, where there is
/// no viewport to measure against yet.
pub const GLYPH_MAX_PX: f32 = 28.0;
/// Gap between the glyph and its letter.
const LABEL_GAP_PX: f32 = 6.0;
/// The letter. Small: it names the key, it is not the readout.
pub const LABEL_PX: f32 = 15.0;
/// How wide one letter is taken to be when the cluster is kept out of the middle.
///
/// A **deliberate over-estimate** and not a measurement: the real width comes out of text layout
/// one stage later, and the keep-out arithmetic has to run before it. `Q` at 15 px measures about
/// 10 px, so 12 leaves room and errs outward — outward is the safe direction, because the box is
/// inward. `tests/hud.rs` measures the real rects afterwards and falls over if this is ever too
/// small.
const LABEL_W_PX: f32 = 12.0;
/// How far clear of `F-170`'s keep-out box a pushed marker stands.
///
/// **Not zero, and the reason is measured.** Flush against the box, the widest glyph puts its
/// centre 142 px from the middle of a 1280 px screen and the narrowest 138 —
/// `docs/NEXT.md` §1B asks W5 for **145**, and a glyph whose edge lies exactly on the boundary
/// is already touching the rectangle `F-170` exists to keep readable. 8 px clears every state
/// (146 px for a 20 px glyph, 150 for the 28 px one) and costs nothing else: the push only ever
/// runs for a point that is inside the box anyway.
const KEEP_OUT_GAP_PX: f32 = 8.0;
/// **The pixels the player is actually aiming at**, and the only part of the middle a fan
/// marker may not cover.
///
/// `F-170`'s keep-out box is 20 % of the width — 128 px either side at 1280 — and that number
/// exists to make the crosshair *four ticks around a hole* ([`crate::hud::crosshair`]): it is
/// sized to the crosshair's arms, not to the thing under them. A marker whose x **is** the
/// resolved fan angle ([`Bearing::Fan`]) is measured against this instead, because the fan lives
/// entirely inside the box: 22° — the widest half-angle the wheel can permit — projects to
/// 252 px, and 2° (`aim_spread_floor_deg`) to 21.8 px, both of them well inside 128.
///
/// **6 px on both axes — a little square, not a margin.** It is the hole the crosshair's four
/// ticks stand around, and a fan marker clears it by stepping **down**, never sideways
/// (`docs/FINDINGS.md` FIND-099). Sideways was the first shape of this guard and it was a fixed
/// slot in miniature: it binds whenever the fan projects under `SIGHT_CORE_PX + glyph_w/2` =
/// 16 px, which on a 1280 px viewport is every angle below **3.63° at 110° FOV** and below 2.5°
/// already at 80° — and the FOV is a live setting the player has a slider for. Down costs
/// nothing: `vector::aim::side_dirs` yaws the two rays around the **camera's** up axis, so their
/// camera-space y is exactly 0 and their projected y is the middle of the screen at every angle
/// and every pitch. The x is the entire message and it is never touched.
///
/// `tests/hud.rs::f023_the_drawn_marker_is_strictly_monotone_in_the_resolved_fan` sweeps the
/// whole reachable band at 55, 60, 90 and 110° and would see a clamp on the x as a flat step.
pub const SIGHT_CORE_PX: f32 = 6.0;

/// **What a marker's x MEANS this frame** — and therefore which rule keeps the sight line clear.
///
/// The keep-out box was written against *chrome*: a gas bar, a banner, a letter creeping inward
/// over the thing the player is about to cut. Two of the six nodes this module draws are not
/// chrome at all, and treating them as such is what
/// [`FIND-098`](../../../docs/FINDINGS.md) is about — see the module header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bearing {
    /// **An idle arm previewing its own side ray.** The glyph's x is the resolved fan half-angle
    /// and nothing else — it carries no distance, no world position, only *how wide the two
    /// ropes are aiming right now*. Exempt from `F-170`'s box, held out of [`SIGHT_CORE_PX`].
    Fan,
    /// **A position in the world, which can be anywhere**: a tip in flight, an anchor being held,
    /// or the shared centre ray an arm with nothing of its own falls back to. Pushed out of
    /// `F-170`'s box, exactly as before.
    World,
}

/// Which of the two rules this arm's marker obeys this tick.
///
/// Three ways to land on [`Bearing::World`], and each of them is a case where the box costs
/// nothing the player cannot get elsewhere:
///
/// - **the tip is out** — `render::rope` is drawing the rope to that exact point in the world, so
///   the glyph is a second telling of something already on screen;
/// - **the arm fell back to the centre ray** (`vector::aim`: a side ray that finds nothing is
///   handed `centre`) — then the marker is a *state badge* and has no position of its own to be
///   honest about, which is `docs/FINDINGS.md` FIND-087 §2's decision, kept;
/// - **the arm has nothing at all** — the side slot, as before.
///
/// The fallback is detected by comparing this arm's point with the shared
/// [`AimPoint`](crate::shared::AimPoint) **by value and with no tolerance**: `vector::aim` assigns
/// the very same `AimPoint` into the arm, so the two are bit-identical when and only when the
/// fallback ran. Same choice, same reason as
/// `tests/hud.rs::f026_the_marker_stands_exactly_where_that_arm_fires`.
pub fn bearing_of(hook: &Hook, aim: &ArmAim, centre_m: Option<Vec3>, side: Side) -> Bearing {
    if !matches!(hook.arm(side).state, HookState::Idle) {
        return Bearing::World;
    }
    match (aim.target_of(side), centre_m) {
        (Some(own), Some(shared)) if own == shared => Bearing::World,
        (Some(_), _) => Bearing::Fan,
        (None, _) => Bearing::World,
    }
}

/// Where one arm's three nodes stand this frame, in logical UI pixels (top-left corners).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmLayout {
    pub glyph: Vec2,
    pub tether: Vec2,
    pub label: Vec2,
    /// Whether the letter sits to the **right** of the glyph. It always sits on the side away
    /// from the middle of the screen, so it can never be the thing that creeps into the box.
    pub label_right: bool,
}

/// **The whole placement rule, as one pure function** — no camera, no `World`, no `Node`.
///
/// `at` is the arm's world target already projected into logical viewport pixels — a real pixel
/// from `Camera::world_to_viewport`, or [`edge_pixel`]'s bearing when the point is behind the near
/// plane. `None` means the arm has **no target at all**: neither its side ray nor the centre ray
/// it falls back to found anything.
///
/// Three steps, in this order and the order is the design:
/// 1. put the glyph's centre on the projected point, or in the arm's side slot if there is none;
/// 2. clamp the cluster into the viewport, so a marker never leaves the screen entirely;
/// 3. keep the sight line clear — and **which** sight line depends on whether this marker
///    carries a place: a marker with a projected point is held out of [`SIGHT_CORE_PX`] only,
///    a marker with none (`at` is `None`) is pushed out of `F-170`'s whole keep-out box towards
///    the side it belongs to. **Both [`Bearing`]s step to the nearer edge** since `F-024`'s
///    sweep was locked to the horizontal on 2026-08-19 — a fan marker's point projects on the
///    crosshair's row, so the two edges are equidistant and the tie falls downwards exactly as
///    FIND-099 measured, while a *snapped* point a few pixels off the row now takes the short
///    way out instead of the long one.
///
/// Step 3 is last on purpose: it is the proven 🟧 claim
/// (`tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen`) and step 2 is a courtesy, so on
/// a viewport small enough for the two to fight, the claim wins.
///
/// **Why nothing with a point in it is pushed** (`docs/FINDINGS.md` FIND-098, FIND-129): the
/// whole reachable fan lives inside the box — 2°..22° of half-angle is 21.8..252 px of a 1280 px
/// screen against a 128 px box edge — so a box push turns *every* fan into the same fixed slot
/// and the wheel stops reaching the screen; and the place a player aims at is the middle of his
/// screen by construction, so the same push threw a **flying tip and a held anchor 150 px / 47.7 m**
/// off the rope. `F-023`'s claim is that the rope and the marker are one number; a marker that
/// cannot move is not that number, and neither is one parked beside the point it names. The box
/// loses nothing it was built to protect: [`SIGHT_CORE_PX`] keeps the pixels the player is
/// cutting uncovered in every case, and a marker with nothing to say still parks in its slot.
pub fn layout_for(
    side: Side,
    shape: ArmShape,
    at: Option<Vec2>,
    viewport: Vec2,
    bearing: Bearing,
) -> ArmLayout {
    let vw = viewport.x.max(1.0);
    let vh = viewport.y.max(1.0);
    let box_min_x = vw * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = vw * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = vh * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = vh * KEEP_OUT_HIGH_PCT / 100.0;

    let full_h = shape.glyph_h_px + shape.tether_px.map_or(0.0, |t| TETHER_GAP_PX + t);
    let label_out = LABEL_GAP_PX + LABEL_W_PX;

    // The slot an arm falls back to, and the place a marker is pushed to when it would cover
    // the middle: [`KEEP_OUT_GAP_PX`] clear of its own side of the keep-out box, at eye level.
    // That is the "weiter rechts und links" the player asked for, and the number is the box
    // rather than a taste.
    let slot_x = |right: bool| {
        if right {
            box_max_x + KEEP_OUT_GAP_PX
        } else {
            box_min_x - KEEP_OUT_GAP_PX - shape.glyph_w_px
        }
    };

    let (mut x, mut y, label_right) = match at {
        Some(p) if p.is_finite() => {
            // Which half is this point in? The letter and the keep-out push both follow that,
            // so a marker never points at the wrong side of the screen. Dead on the axis is the
            // idle case, and there the arm's own side decides — `Q` left, `E` right.
            let lean_right = if (p.x - vw * 0.5).abs() < 0.5 {
                matches!(side, Side::Right)
            } else {
                p.x > vw * 0.5
            };
            (p.x - shape.glyph_w_px * 0.5, p.y - shape.glyph_h_px * 0.5, lean_right)
        }
        _ => {
            let right = matches!(side, Side::Right);
            (slot_x(right), vh * 0.5 - shape.glyph_h_px * 0.5, right)
        }
    };

    // 2. into the viewport.
    let (lo_extra, hi_extra) = if label_right { (0.0, label_out) } else { (label_out, 0.0) };
    let min_x = lo_extra;
    let max_x = (vw - shape.glyph_w_px - hi_extra).max(min_x);
    x = x.clamp(min_x, max_x);
    y = y.clamp(0.0, (vh - full_h).max(0.0));

    // 3. out of the middle. The cluster is the glyph plus its letter; the tether hangs inside
    // the glyph's own width, so it adds nothing horizontally and its length is already in
    // `full_h`.
    let core = Vec2::new(vw * 0.5, vh * 0.5);
    let over_the_core = |x: f32, y: f32| {
        x < core.x + SIGHT_CORE_PX
            && x + shape.glyph_w_px > core.x - SIGHT_CORE_PX
            && y < core.y + SIGHT_CORE_PX
            && y + full_h > core.y - SIGHT_CORE_PX
    };
    match bearing {
        // **A marker that carries a place is not chrome either** (`docs/FINDINGS.md` FIND-129).
        //
        // FIND-098 exempted the *fan* from the box and left this arm inside it, on the argument
        // that `render::rope` is already drawing the rope to the point so the box costs nothing.
        // Swept in the running app on 2026-08-19 that argument cost **150.0 px / 47.7 m**: the
        // place a player aims at is by construction the middle of his screen, so the box fires on
        // the common case and not on the corner one — 400 of 469 samples over three stands, nine
        // look angles, five assist settings and all four arm states were drawn somewhere the rope
        // does not go. **And 16 of those had no rope on screen at all**: an idle arm that fell
        // back to the centre ray is a shot that has not happened yet, `render::rope` is drawing
        // nothing, and the marker was the player's only reading of where the hook would land.
        //
        // The user, 2026-08-19: *„wichtig wäre nur dass diese auch genau da sind visuell wo das
        // seil auch landen würde!"* — so the box loses this case, and keeps the one it was
        // written for: a marker with **no** place of its own (`at` is `None`) is a badge, it
        // claims nothing about the world, and it parks in its side slot as before.
        //
        // What the sight line keeps is [`SIGHT_CORE_PX`] — the pixels the player is cutting. A
        // world marker steps out of that square **vertically, towards the nearer edge**: the
        // letter sits outboard of the glyph and moves with `y`, so a vertical step can never put
        // the label on the core (a horizontal one can — escaping left with the letter on the
        // right lands it exactly on the middle), and going to the *nearer* edge keeps the glyph
        // on the side of the sight line its point really is on. [`Bearing::Fan`] does the same
        // since 2026-08-19; see the note on that arm for why it used to differ and why the
        // difference stopped paying for itself.
        Bearing::World => match at {
            Some(p) if p.is_finite() => {
                if over_the_core(x, y) {
                    let down = core.y + SIGHT_CORE_PX;
                    let up = core.y - SIGHT_CORE_PX - full_h;
                    y = if up >= 0.0 && (y - up).abs() < (down - y).abs() { up } else { down };
                }
            }
            _ => {
                let hits_box = x - lo_extra < box_max_x
                    && x + shape.glyph_w_px + hi_extra > box_min_x
                    && y < box_max_y
                    && y + full_h > box_min_y;
                if hits_box {
                    x = slot_x(label_right);
                }
            }
        },
        // **The fan keeps its exact angle at every field of view and gives up the axis that
        // carries nothing.** A fan marker's y is the crosshair's y at every angle — `side_dirs`
        // yaws the two rays around the *camera's* up axis, so their camera-space y is exactly
        // zero and their projected y is exactly the middle of the screen, whatever the pitch.
        // Its x is the whole message. So when the two would meet over the sight core the glyph
        // steps out of it **vertically** and holds its x: the marker stays honest and the
        // pixels the player is cutting stay uncovered.
        //
        // The clamp this replaces was an `x.max(centre + SIGHT_CORE_PX)`, and it was a fixed
        // slot in miniature: it binds whenever the fan projects under
        // `SIGHT_CORE_PX + glyph_w/2` = 16 px, which at 1280 logical px is **every angle below
        // 3.63° once the player sets his FOV to 110** — measured, and 80° is already enough
        // (`docs/FINDINGS.md` FIND-099). The FOV is a live setting since 2026-08-18
        // (`shared::PlayerSettings`, 55..110), so that band is not a corner case.
        //
        // The letter sits outboard by construction and moves with `y`, and the tether hangs
        // inside the glyph's width — the glyph's own rectangle is the whole rule.
        Bearing::Fan => {
            // **The same nearer-edge step as [`Bearing::World`], and it is a no-op on the case
            // FIND-099 decided.** A fan marker whose point projects *exactly* on the crosshair's
            // row finds the two edges of the core equidistant, the tie resolves downwards, and
            // the picture FIND-099 measured is unchanged.
            //
            // It stops being a no-op where `F-024`'s sideways-only sweep put an idle arm since
            // 2026-08-19: the arm's point is then a **snapped world place** at a real distance,
            // and the render camera is one stage behind the transform `vector::aim` cast from,
            // so the projection lands a few pixels off the row rather than on it. Measured on
            // the shipped map at `(168.19, 0, -50.12)`, yaw 90 pitch 10, assist 25/25: the point
            // projects to `(633.0, 356.8)`, the old always-down step drew the glyph at
            // `(633.0, 376.0)` — **19.2 px** from its own point — and stepping to the nearer
            // edge draws it at `(633.0, 344.0)`, **12.8 px**. The bound is now the same half a
            // glyph plus six pixels that `tests/hud.rs::f023_the_drawn_pixel_is_the_projection_\
            // of_the_point_the_rope_flies_to` allows for a world marker, and that test's
            // allowance was always written for this rule — it was simply never applied here.
            if over_the_core(x, y) {
                let down = core.y + SIGHT_CORE_PX;
                let up = core.y - SIGHT_CORE_PX - full_h;
                y = if up >= 0.0 && (y - up).abs() < (down - y).abs() { up } else { down };
            }
        }
    }

    ArmLayout {
        glyph: Vec2::new(x, y),
        tether: Vec2::new(
            x + (shape.glyph_w_px - TETHER_W_PX) * 0.5,
            y + shape.glyph_h_px + TETHER_GAP_PX,
        ),
        label: Vec2::new(
            if label_right {
                x + shape.glyph_w_px + LABEL_GAP_PX
            } else {
                x - LABEL_GAP_PX - LABEL_W_PX
            },
            y + (shape.glyph_h_px - LABEL_PX) * 0.5,
        ),
        label_right,
    }
}

/// **The screen point for a target the camera refuses to project**: a pseudo-pixel far outside
/// the viewport, on the bearing the target really lies.
///
/// `camera_space_m` is the target in the camera's own frame — `+X` right, `+Y` up, `-Z` forward
/// (Bevy's view convention; `docs/conventions.md`'s axes are the world's). Only the two lateral
/// components are read: past the near plane there is no pixel, but there is still a side.
///
/// [`layout_for`] clamps whatever comes out of here into the viewport, so the marker lands on the
/// edge that points at the target, and the handover is continuous — a point drifting towards the
/// camera plane projects further and further out on the same side, and this keeps it there.
///
/// A target **dead** behind the camera has no bearing at all (`x` and `y` both ~0). Then the arm's
/// own side decides, which is the same tie-break [`layout_for`] uses on the screen axis.
pub fn edge_pixel(camera_space_m: Vec3, viewport: Vec2, side: Side) -> Vec2 {
    // Screen y grows downwards and the camera's y upwards — hence the one sign.
    let mut dir = Vec2::new(camera_space_m.x, -camera_space_m.y);
    if !dir.is_finite() || dir.length_squared() < 1e-6 {
        dir = Vec2::new(if matches!(side, Side::Right) { 1.0 } else { -1.0 }, 0.0);
    }
    // Longer than the diagonal, so the clamp really reaches an edge whichever way it points.
    viewport * 0.5 + dir.normalize() * (viewport.length() + 1.0)
}

/// Six nodes: a glyph, a tether and a letter per arm.
///
/// The positions written here are a **one-frame placeholder** — there is no viewport at
/// `Startup`, so they are percentages that stand clear of the keep-out box, and
/// [`place_arm_aim`] overwrites them in pixels as soon as a camera reports a size.
pub fn spawn_arm_aim(mut commands: Commands, data: Res<GameData>) {
    for side in Side::ALL {
        let shape = shape_of(ArmAimState::default());
        let right = matches!(side, Side::Right);
        // Outside the box on the arm's own side: 34 % / 64 % of the width against a box that
        // runs 40 %..60 %.
        let x = if right {
            Val::Percent(KEEP_OUT_HIGH_PCT + 4.0)
        } else {
            Val::Percent(KEEP_OUT_LOW_PCT - 6.0)
        };
        commands.spawn((
            Name::new(format!("hud_arm_marker_{side:?}")),
            ArmMarker { side, part: MarkerPart::Glyph },
            ArmAimState::default(),
            ArmMiss::default(),
            HudElement,
            BackgroundColor(NEUTRAL),
            BorderColor::all(Color::NONE),
            placeholder(shape_node(MarkerPart::Glyph, shape), x),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_tether_{side:?}")),
            ArmMarker { side, part: MarkerPart::Tether },
            ArmAimState::default(),
            ArmMiss::default(),
            HudElement,
            BackgroundColor(NEUTRAL),
            BorderColor::all(Color::NONE),
            placeholder(shape_node(MarkerPart::Tether, shape), x),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_label_{side:?}")),
            ArmMarkerLabel(side),
            ArmMiss::default(),
            HudElement,
            Text::new(key_label(side)),
            TextFont { font_size: FontSize::Px(LABEL_PX), ..default() },
            TextColor(signal(&data, "cyan")),
            placeholder(
                Node { position_type: PositionType::Absolute, ..default() },
                if right {
                    Val::Percent(KEEP_OUT_HIGH_PCT + 4.0 + GLYPH_MAX_PX / 10.0)
                } else {
                    Val::Percent(KEEP_OUT_LOW_PCT - 9.0)
                },
            ),
        ));
    }
}

fn placeholder(mut node: Node, x: Val) -> Node {
    node.left = x;
    node.top = Val::Percent(50.0);
    node
}

/// The `Q` / `E` letter.
///
/// It names a key binding, and until `F-028` its **text** was written once at startup and never
/// again. It is now also the one place a *failed* pull can say something in words
/// ([`show_arm_miss`]): for [`MISS_HINT_S`] seconds after a trigger that found nothing the
/// letter is followed by [`miss_label`], and then it goes back to being the key and nothing
/// else. Its position always follows the glyph.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmMarkerLabel(pub Side);

/// The size half of a node: everything [`shape_arm_aim`] owns, and nothing else.
fn shape_node(part: MarkerPart, shape: ArmShape) -> Node {
    let mut node = Node { position_type: PositionType::Absolute, ..default() };
    match part {
        MarkerPart::Glyph => {
            node.width = Val::Px(shape.glyph_w_px);
            node.height = Val::Px(shape.glyph_h_px);
            node.border = UiRect::all(Val::Px(shape.border_px));
            // `border_radius` is a field of `Node` in bevy 0.19 (`ui_node.rs:738`), not a
            // component of its own — so the ring and the dash are one write, not two.
            node.border_radius =
                if shape.round { BorderRadius::MAX } else { BorderRadius::ZERO };
        }
        MarkerPart::Tether => match shape.tether_px {
            Some(length_px) => {
                node.width = Val::Px(TETHER_W_PX);
                node.height = Val::Px(length_px);
            }
            // `Display::None` and not a despawn — an entity that comes and goes changes the
            // archetype every time an arm catches, and would make the node count depend on when
            // you look. The crosshair's corner marks are switched off the same way.
            None => node.display = Display::None,
        },
    }
    node
}

/// Reads the local player's two arms, writes [`ArmAimState`] and nothing else.
///
/// `anchorable` is **this arm's own** answer, out of [`ArmAim`] — `F-023`'s hemispheres landed on
/// 2026-08-13, so the left ring and the right dash can now disagree, and that is the point: `Q`
/// catching while `E` does not is a fact about the world that the pair may not average away.
/// `vector::hook::update_hooks` decides the shot from exactly the same field
/// (`anchor_target(arm_aim.side(side))`), so a `Ready` ring is a promise the simulation keeps.
pub fn sense_arm_aim(
    players: Query<(&Hook, &ArmAim), With<LocalPlayer>>,
    mut markers: Query<(&ArmMarker, &mut ArmAimState)>,
) {
    let Some((hook, aim)) = players.iter().next() else {
        return;
    };
    for (marker, mut state) in &mut markers {
        let side = marker.side;
        state.set_if_neq(state_for(&hook.arm(side).state, aim.side(side).anchorable));
    }
}

/// **`F-028` — reads every failed pull of the local player and starts the hint.**
///
/// The message is [`HookReleased`] with [`ReleaseReason::NoAnchor`], which `vector::hook` has
/// written in the same tick the trigger went down. No new channel: the reason travels **on** the
/// message every consumer already reads, so there is no way for the HUD and the log to disagree
/// about why a shot did not leave.
///
/// **Filtered by [`PlayerId`]** and not by "the only player there is": in a session with a team
/// mate his misses arrive here too, and a marker that flashed for someone else's trigger would
/// be a lie about the local player's own gear (`docs/multiplayer.md` rule 1).
///
/// The countdown runs even in a frame with no message, so the hint clears itself — and once it
/// has cleared, the `set_if_neq` stops writing at all and change detection goes quiet
/// (`docs/lessons/performance.md` rule 1).
///
/// **One writer of [`ArmMiss`].**
pub fn sense_arm_miss(
    time: Res<Time>,
    mut released: MessageReader<HookReleased>,
    players: Query<&PlayerId, With<LocalPlayer>>,
    mut glyphs: Query<(&ArmMarker, &mut ArmMiss), Without<ArmMarkerLabel>>,
    mut labels: Query<(&ArmMarkerLabel, &mut ArmMiss), Without<ArmMarker>>,
) {
    let dt = time.delta_secs();
    // The freshest miss per side this frame. An array and not a lookup: two sides, and the
    // second pull of the same arm in one frame is the one the player is asking about.
    let mut fresh: [Option<ArmHint>; 2] = [None; 2];
    if let Some(me) = players.iter().next() {
        for message in released.read() {
            if message.player != *me {
                continue;
            }
            // **Two of the four reasons say something and two do not.** `Released` is the
            // player letting go — an intention, not a failure — and `Overextended` is the rope
            // running out against a wall, which he sees happen. A marker that flashed on every
            // release would be noise on the one channel that has to mean something.
            let hint = match message.reason {
                ReleaseReason::NoAnchor(why) => Some(ArmHint::Miss(why)),
                ReleaseReason::BodyGone => Some(ArmHint::CarrierGone),
                ReleaseReason::Released | ReleaseReason::Overextended => None,
            };
            if let Some(hint) = hint {
                fresh[message.side.index()] = Some(hint);
            }
        }
    } else {
        // No local player (the menu before a sortie): drain the cursor anyway, or the first
        // frame after a spawn replays a backlog of somebody else's misses.
        released.clear();
    }

    for (side, mut miss) in glyphs
        .iter_mut()
        .map(|(m, miss)| (m.side, miss))
        .chain(labels.iter_mut().map(|(l, miss)| (l.0, miss)))
    {
        miss.set_if_neq(step_miss(*miss, fresh[side.index()], dt));
    }
}

/// One step of the hint: a fresh miss restarts it, anything else lets it run out.
///
/// A free function so the rule is testable without a message writer, a local player and a
/// clock having to be arranged first — the same reason [`state_for`] is one.
///
/// **A fresh miss always wins, including over a hint that is already running**: the second
/// press is the one the player is asking about, and a hint that kept showing the first reason
/// would answer the wrong question. It resets even when the reason is *the same*, so the word
/// visibly reappears instead of quietly ageing out mid-press.
pub fn step_miss(miss: ArmMiss, fresh: Option<ArmHint>, dt_s: f32) -> ArmMiss {
    match fresh {
        Some(why) => ArmMiss { reason: Some(why), left_s: MISS_HINT_S },
        None => {
            let left_s = miss.left_s - dt_s.max(0.0);
            // Cleared to the exact default and not to a small remainder: `set_if_neq` then
            // stops writing entirely once the hint is over (§6 rule 6).
            if left_s > 0.0 && miss.reason.is_some() {
                ArmMiss { reason: miss.reason, left_s }
            } else {
                ArmMiss::default()
            }
        }
    }
}

/// **`F-028` — puts the word under the marker**, and takes it away again.
///
/// Writes `Text` and `TextColor` on the two label nodes and nothing else; the letter itself
/// still comes from [`key_label`], so a rebind cannot leave a stale key on screen
/// (`tests/hud.rs::f171_the_marker_letters_are_the_keys_that_fire_the_arms`).
///
/// Both writes are guarded by a comparison: outside a hint this system does nothing at all, and
/// a standing player produces no write (§6 rule 6).
pub fn show_arm_miss(
    data: Res<GameData>,
    mut labels: Query<(&ArmMarkerLabel, &ArmMiss, &mut Text, &mut TextColor)>,
) {
    let cyan = signal(&data, "cyan");
    let crimson = signal(&data, "crimson");
    for (label, miss, mut text, mut colour) in &mut labels {
        let (wanted, ink) = match miss.reason {
            Some(why) => (format!("{}  {}", key_label(label.0), miss_label(why)), crimson),
            None => (key_label(label.0).to_string(), cyan),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
        if colour.0 != ink {
            colour.0 = ink;
        }
    }
}

/// Writes the **size** fields of `Node` and no others — see the module header on why the split
/// is by field and not by entity.
pub fn shape_arm_aim(mut markers: Query<(&ArmMarker, &ArmAimState, &mut Node)>) {
    for (marker, state, mut node) in &mut markers {
        let wanted = shape_node(marker.part, shape_of(*state));
        if node.width != wanted.width
            || node.height != wanted.height
            || node.border != wanted.border
            || node.border_radius != wanted.border_radius
            || node.display != wanted.display
        {
            node.width = wanted.width;
            node.height = wanted.height;
            node.border = wanted.border;
            node.border_radius = wanted.border_radius;
            node.display = wanted.display;
        }
    }
}

/// Writes the **position** fields of `Node` and no others.
fn put(node: &mut Node, at: Vec2) {
    let left = Val::Px(at.x);
    let top = Val::Px(at.y);
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    // The placeholder at spawn only ever sets `left`/`top`, but a `right` left over from an
    // older layout would fight this one silently, so both are pinned to `Auto` here.
    if node.right != Val::Auto {
        node.right = Val::Auto;
    }
    if node.bottom != Val::Auto {
        node.bottom = Val::Auto;
    }
}

/// **The preview itself:** projects each arm's world target and puts its three nodes there.
///
/// Runs in `PostUpdate`, after `TransformSystems::Propagate` and `CameraUpdateSystems` and
/// before `UiSystems::Layout`. Not in `Update`: the camera's `GlobalTransform` and its viewport
/// size are both computed in `PostUpdate`, so a marker placed in `Update` would be one frame
/// behind the image — which is invisible standing still and very visible mid-swing, exactly when
/// the element has to be trusted.
///
/// `Camera3d` without a further filter is enough because there is at most one 3D camera:
/// `render::attach_camera` bails out as soon as one exists.
pub fn place_arm_aim(
    players: Query<(&Hook, &ArmAim, Option<&AimPoint>), With<LocalPlayer>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut markers: Query<(&ArmMarker, &ArmAimState, &mut Node), Without<ArmMarkerLabel>>,
    mut labels: Query<(&ArmMarkerLabel, &mut Node), Without<ArmMarker>>,
) {
    let Some((camera, camera_at)) = cameras.iter().next() else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };

    // Read the two shapes first, so both `&mut Node` loops below can be write-only.
    let mut shapes = [shape_of(ArmAimState::default()); 2];
    for (marker, state, _) in markers.iter() {
        shapes[marker.side.index()] = shape_of(*state);
    }

    let aim = players.iter().next();
    // The shared centre ray, if the player has one at all: `bearing_of` needs it to tell an arm
    // aiming down its own side ray from an arm that found nothing and fell back to the middle.
    let centre_m = aim.and_then(|(_, _, centre)| centre.and_then(|c| c.point_m));
    let mut layout = [ArmLayout::default(); 2];
    for side in Side::ALL {
        let world = aim.and_then(|(hook, point, _)| target_of(hook, point, side));
        let bearing = aim.map_or(Bearing::World, |(hook, point, _)| {
            bearing_of(hook, point, centre_m, side)
        });
        // Not an `expect` and not a drop: a target behind the near plane is a normal thing to be
        // holding — half a swing looks like that — and `world_to_viewport` reports it as an
        // error. It keeps its **bearing** through [`edge_pixel`], and the clamp in `layout_for`
        // turns that into the screen edge on the right side (module header).
        let at = world.map(|p| match camera.world_to_viewport(camera_at, p) {
            Ok(px) => px,
            Err(_) => edge_pixel(camera_at.affine().inverse().transform_point3(p), viewport, side),
        });
        layout[side.index()] = layout_for(side, shapes[side.index()], at, viewport, bearing);
    }

    for (marker, _, mut node) in &mut markers {
        let arm = layout[marker.side.index()];
        put(
            &mut node,
            match marker.part {
                MarkerPart::Glyph => arm.glyph,
                MarkerPart::Tether => arm.tether,
            },
        );
    }
    for (label, mut node) in &mut labels {
        put(&mut node, layout[label.0.index()].label);
    }
}

/// Writes the two colours — and neither of them carries information the shape does not already
/// carry.
///
/// Cyan is `maps.ron`'s *"gas, Vector Gear, anchor points"*, read through [`signal`] and never
/// written as a literal. [`NEUTRAL`] is the crosshair's own "the ray hits nothing worth having"
/// grey, borrowed rather than spelled a second time — a second literal is how two elements that
/// mean the same thing start looking different.
pub fn paint_arm_aim(
    data: Res<GameData>,
    mut markers: Query<
        (&ArmAimState, &ArmMiss, &mut BackgroundColor, &mut BorderColor),
        With<ArmMarker>,
    >,
) {
    let cyan = signal(&data, "cyan");
    // `F-028`: the one thing colour is allowed to carry, and it carries it for [`MISS_HINT_S`]
    // and not a frame longer. It is an **event** colour, not a state colour — the shape still
    // says everything about the state, so `F-171`'s rule ("the colour carries nothing the shape
    // does not") holds for every state the marker can sit in.
    let crimson = signal(&data, "crimson");
    for (state, miss, mut fill, mut border) in &mut markers {
        // An outline glyph is transparent inside and coloured at the edge; a filled one is the
        // other way round. Which of the two it is comes out of the shape table, so the colour
        // cannot disagree with the geometry.
        let shape = shape_of(*state);
        let ink = match (miss.reason, state) {
            (Some(_), _) => crimson,
            (None, ArmAimState::Free) => NEUTRAL,
            (None, ArmAimState::Ready | ArmAimState::Busy | ArmAimState::Anchored) => cyan,
        };
        let (wanted_fill, wanted_border) = if shape.border_px > 0.0 {
            (Color::NONE, ink)
        } else {
            (ink, Color::NONE)
        };
        if fill.0 != wanted_fill {
            fill.0 = wanted_fill;
        }
        if border.top != wanted_border {
            border.set_all(wanted_border);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{AimPoint, BodyId, HookArm};

    const SCREEN: Vec2 = Vec2::new(1280.0, 720.0);

    /// `F-028` + `F-029` — the **five** things an arm can say are five different words, and
    /// each of them asks the player for a different move. Two rows that read the same are a
    /// table that explains nothing.
    ///
    /// The fifth is `F-029`'s: a rope torn off a dying titan. It rides on this channel and it
    /// may not read like any of the four misses — he did not aim badly, he had a good anchor
    /// and it died.
    #[test]
    fn f028_every_reason_gets_its_own_word_under_the_marker() {
        let all = [
            ArmHint::Miss(MissReason::NothingInRange),
            ArmHint::Miss(MissReason::OutOfReach),
            ArmHint::Miss(MissReason::SurfaceHoldsNothing),
            ArmHint::Miss(MissReason::NoCarrier),
            ArmHint::CarrierGone,
        ];
        for (i, a) in all.iter().enumerate() {
            assert!(!miss_label(*a).is_empty(), "{a:?} has no word");
            // Short enough to be read out of the corner of the eye mid-swing.
            assert!(miss_label(*a).len() <= 12, "{a:?} says {:?}, too long", miss_label(*a));
            for b in &all[i + 1..] {
                assert_ne!(miss_label(*a), miss_label(*b), "{a:?} and {b:?} say the same word");
            }
        }
    }

    /// `F-028` — the hint appears on the press, survives long enough to be read, and then
    /// **goes away by itself**.
    ///
    /// The last part is the one that matters: the marker is a statement about the world right
    /// now, and a caption that outlived its cause would make it lie.
    #[test]
    fn f028_the_miss_hint_starts_on_the_press_and_clears_itself() {
        let quiet = ArmMiss::default();
        assert_eq!(quiet.reason, None, "a fresh marker says nothing");

        // The press.
        let why = ArmHint::Miss(MissReason::SurfaceHoldsNothing);
        let hinting = step_miss(quiet, Some(why), 1.0 / 60.0);
        assert_eq!(hinting.reason, Some(why));
        assert_eq!(hinting.left_s, MISS_HINT_S);

        // It survives a full second of frames — long enough to read two words.
        let mut running = hinting;
        for _ in 0..60 {
            running = step_miss(running, None, 1.0 / 60.0);
        }
        assert_eq!(running.reason, Some(why), "it vanished too fast");

        // And it is gone before two seconds, back to the exact default so that `set_if_neq`
        // stops writing.
        for _ in 0..60 {
            running = step_miss(running, None, 1.0 / 60.0);
        }
        assert_eq!(running, ArmMiss::default(), "the hint outlived the situation it described");

        // A second press wins over a hint that is still running, even for the same reason.
        let far = ArmHint::Miss(MissReason::OutOfReach);
        let second = step_miss(hinting, Some(far), 1.0 / 60.0);
        assert_eq!(second, ArmMiss { reason: Some(far), left_s: MISS_HINT_S });

        // `F-029`: and a rope torn off a dying titan wins over a running miss hint too. The
        // rope going slack is the fact the player is standing in right now.
        let torn = step_miss(second, Some(ArmHint::CarrierGone), 1.0 / 60.0);
        assert_eq!(torn, ArmMiss { reason: Some(ArmHint::CarrierGone), left_s: MISS_HINT_S });
    }

    #[test]
    fn f171_a_busy_arm_is_not_a_ready_one() {
        // The rule the whole element hangs on. `vector::hook` only fires from `Idle`, so an arm
        // whose tip is out has to say so — a `Ready` ring over a retracting arm is a promise the
        // simulation does not keep.
        assert_eq!(state_for(&HookState::Idle, true), ArmAimState::Ready);
        assert_eq!(state_for(&HookState::Idle, false), ArmAimState::Free);
        assert_eq!(state_for(&HookState::Retracting, true), ArmAimState::Busy);
        assert_eq!(
            state_for(&HookState::Flying { target_m: Vec3::ZERO, body: BodyId(1) }, true),
            ArmAimState::Busy
        );
        assert_eq!(
            state_for(&HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO }, false),
            ArmAimState::Anchored,
            "an anchored arm is anchored whatever the aim ray happens to be looking at"
        );
    }

    #[test]
    fn f171_the_shape_table_is_the_only_place_the_arm_numbers_live() {
        for state in [
            ArmAimState::Free,
            ArmAimState::Ready,
            ArmAimState::Busy,
            ArmAimState::Anchored,
        ] {
            let shape = shape_of(state);
            let expected = if shape.tether_px.is_some() { 2 } else { 1 };
            assert_eq!(node_count(state), expected, "{state:?}: the two tables disagree");
        }
    }

    #[test]
    fn f171_an_arm_with_its_tip_out_previews_its_own_tip_and_not_the_shared_aim() {
        // The wiring that makes the two markers two markers. Each arm's tip sits somewhere of
        // its own, and the arm that is holding has to preview ITS point. Goes red the day
        // somebody wires all four states back to one shared value.
        let aim = two_sided_aim();
        let held = Vec3::new(-31.0, 17.0, -12.0);
        let flown = Vec3::new(44.0, 3.0, -20.0);
        let hook = Hook {
            arms: [
                HookArm {
                    state: HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO },
                    tip_m: held,
                },
                HookArm { state: HookState::Flying { target_m: flown, body: BodyId(2) }, tip_m: flown },
            ],
        };
        assert_eq!(target_of(&hook, &aim, Side::Left), Some(held));
        assert_eq!(target_of(&hook, &aim, Side::Right), Some(flown));
    }

    /// Two idle arms aimed at two genuinely different places — `F-023`'s hemispheres, as
    /// `vector::aim` writes them.
    fn two_sided_aim() -> ArmAim {
        ArmAim {
            arms: [
                AimPoint {
                    point_m: Some(Vec3::new(-24.0, 9.0, -40.0)),
                    body: Some(BodyId(7)),
                    anchorable: true,
                },
                AimPoint {
                    point_m: Some(Vec3::new(31.0, 5.0, -55.0)),
                    body: Some(BodyId(8)),
                    anchorable: true,
                },
            ],
        }
    }

    #[test]
    fn f026_an_idle_marker_stands_on_its_own_arms_point() {
        // ★ **The user's sentence as a rule, at the smallest level there is.** *"und da wo das
        // seil am ende auch landet soll die markierung hin"* — the idle preview reads
        // `ArmAim::target_of(side)`, which is the same field `vector::hook::fire` turns into
        // `Flying { target_m }`. Two arms, two points, and neither may be the other's.
        let aim = two_sided_aim();
        let idle = Hook::default();
        assert_eq!(target_of(&idle, &aim, Side::Left), aim.target_of(Side::Left));
        assert_eq!(target_of(&idle, &aim, Side::Right), aim.target_of(Side::Right));
        assert_ne!(
            target_of(&idle, &aim, Side::Left),
            target_of(&idle, &aim, Side::Right),
            "the two arms aim at two different places and the markers say the same thing — \
             then the pair is one marker drawn twice (FIND-047)"
        );
        assert_eq!(
            target_of(&idle, &ArmAim::default(), Side::Left),
            None,
            "nothing in range is not a point at the origin"
        );
    }

    #[test]
    fn f026_a_target_behind_the_camera_keeps_its_side() {
        // Behind the near plane there is no pixel, and `world_to_viewport` says so. There is
        // still a side, and mid-swing that side is the whole message. The pseudo-pixel has to
        // be far enough out that `layout_for`'s clamp puts the marker on that edge.
        let behind_right = Vec3::new(12.0, 1.0, 40.0); // camera space: +Z is BEHIND
        let behind_left = Vec3::new(-12.0, 1.0, 40.0);
        for side in Side::ALL {
            let r = edge_pixel(behind_right, SCREEN, side);
            let l = edge_pixel(behind_left, SCREEN, side);
            assert!(r.x > SCREEN.x, "a point behind and to the right went to {r:?}");
            assert!(l.x < 0.0, "a point behind and to the left went to {l:?}");

            // And through the layout: the marker sits ON the edge, not off it.
            let shape = shape_of(ArmAimState::Anchored);
            let laid = layout_for(side, shape, Some(r), SCREEN, Bearing::World);
            assert!(
                laid.glyph.x + shape.glyph_w_px <= SCREEN.x && laid.glyph.x > SCREEN.x * 0.5,
                "{side:?} behind-right was laid out at {:?}",
                laid.glyph
            );
        }

        // Dead behind: no bearing at all, so the arm's own side decides instead of (0, 0).
        let dead = Vec3::new(0.0, 0.0, 30.0);
        assert!(edge_pixel(dead, SCREEN, Side::Left).x < 0.0);
        assert!(edge_pixel(dead, SCREEN, Side::Right).x > SCREEN.x);
        assert!(
            edge_pixel(Vec3::new(f32::NAN, 1.0, 9.0), SCREEN, Side::Right).is_finite(),
            "a NaN target must not become a NaN pixel"
        );
    }

    #[test]
    fn f170_a_world_marker_keeps_its_pixel_and_a_badge_keeps_out_of_the_box() {
        // ★ **The two halves of the keep-out rule, swept over the whole screen.**
        //
        // Until FIND-129 this test asserted one thing — a world-tracked marker never reaches
        // `F-170`'s box — and that claim is what put the marker **150 px** from the rope on the
        // common case, because the place a player aims at is the middle of his screen by
        // construction. The box now applies to the half of the rule it was written for:
        //
        // - **a badge** (`at` is `None`: the arm found nothing at all) claims nothing about the
        //   world, parks in its side slot and stays clear of the whole box — unchanged;
        // - **a marker with a place in it** is drawn on that place. Its x is never touched, and
        //   the only pixels it may not sit on are [`SIGHT_CORE_PX`] — the ones being cut.
        let box_min_x = SCREEN.x * KEEP_OUT_LOW_PCT / 100.0;
        let box_max_x = SCREEN.x * KEEP_OUT_HIGH_PCT / 100.0;
        let box_min_y = SCREEN.y * KEEP_OUT_LOW_PCT / 100.0;
        let box_max_y = SCREEN.y * KEEP_OUT_HIGH_PCT / 100.0;

        let mut dodges = 0;
        for state in [
            ArmAimState::Free,
            ArmAimState::Ready,
            ArmAimState::Busy,
            ArmAimState::Anchored,
        ] {
            let shape = shape_of(state);
            let full_h = shape.glyph_h_px + shape.tether_px.map_or(0.0, |t| 4.0 + t);
            for side in Side::ALL {
                for step_x in 0..=64 {
                    for step_y in 0..=36 {
                        let at = Vec2::new(
                            SCREEN.x * step_x as f32 / 64.0,
                            SCREEN.y * step_y as f32 / 36.0,
                        );

                        // 1 · the badge — the box rule, exactly as before.
                        let badge = layout_for(side, shape, None, SCREEN, Bearing::World);
                        let (lo, hi) = if badge.label_right {
                            (badge.glyph.x, badge.label.x + LABEL_W_PX)
                        } else {
                            (badge.label.x, badge.glyph.x + shape.glyph_w_px)
                        };
                        assert!(
                            !(lo < box_max_x
                                && hi > box_min_x
                                && badge.glyph.y < box_max_y
                                && badge.glyph.y + full_h > box_min_y),
                            "{state:?} {side:?}: an arm with no target at all put its cluster \
                             at x {lo:.1}..{hi:.1}, y {:.1}..{:.1} — inside the keep-out box \
                             x {box_min_x:.1}..{box_max_x:.1}, y {box_min_y:.1}..{box_max_y:.1}",
                            badge.glyph.y,
                            badge.glyph.y + full_h
                        );

                        // 2 · the marker with a place — its x is the projection, full stop.
                        let l = layout_for(side, shape, Some(at), SCREEN, Bearing::World);
                        // The viewport clamp of step 2 is part of the honest answer: it is
                        // the same courtesy every marker gets, and it moves nothing that fits.
                        let label_out = LABEL_GAP_PX + LABEL_W_PX;
                        let (lo_extra, hi_extra) =
                            if l.label_right { (0.0, label_out) } else { (label_out, 0.0) };
                        let honest_x = (at.x - shape.glyph_w_px * 0.5).clamp(
                            lo_extra,
                            (SCREEN.x - shape.glyph_w_px - hi_extra).max(lo_extra),
                        );
                        assert!(
                            (l.glyph.x - honest_x).abs() < 0.01,
                            "{state:?} {side:?} aimed at {at:?}: the glyph's x is {:.1} and the \
                             projected point is at {:.1}. A world marker's x is where the rope \
                             goes and nothing is allowed to move it sideways",
                            l.glyph.x,
                            honest_x
                        );

                        // 3 · …and it never sits on the pixels being cut.
                        let core = SCREEN * 0.5;
                        assert!(
                            !(l.glyph.x < core.x + SIGHT_CORE_PX
                                && l.glyph.x + shape.glyph_w_px > core.x - SIGHT_CORE_PX
                                && l.glyph.y < core.y + SIGHT_CORE_PX
                                && l.glyph.y + full_h > core.y - SIGHT_CORE_PX),
                            "{state:?} {side:?} aimed at {at:?} put the glyph at {:?}, on the \
                             {SIGHT_CORE_PX} px square the player is cutting",
                            l.glyph
                        );
                        // 4 · the vertical step is the ONLY licence, and it is bounded.
                        let honest_y = (at.y - shape.glyph_h_px * 0.5)
                            .clamp(0.0, (SCREEN.y - full_h).max(0.0));
                        if (l.glyph.y - honest_y).abs() > 0.01 {
                            dodges += 1;
                            assert!(
                                (l.glyph.y - honest_y).abs() <= SIGHT_CORE_PX + full_h + 1.0,
                                "{state:?} {side:?} aimed at {at:?}: the glyph stepped from \
                                 y {honest_y:.1} to {:.1} to clear the sight core — that is \
                                 further than the core plus a glyph and it is no longer a dodge",
                                l.glyph.y
                            );
                        }
                    }
                }
            }
        }
        assert!(
            dodges > 0,
            "the sight-core step never fired over the whole screen sweep — it is dead code and \
             clause 3 above is proving nothing"
        );
    }

    #[test]
    fn f023_a_fan_marker_keeps_its_angle_and_clears_the_aim_pixel() {
        // ★ **The other half of the trap.** `f170_no_projected_point_can_push_a_marker_into_the_middle`
        // proves a WORLD-tracked marker never reaches the box; this proves a FAN marker never
        // reaches the aim pixel — which is the only part of the middle it was ever protecting.
        // Swept over the whole screen, both idle shapes, both arms.
        //
        // **Since FIND-099 the x is never touched at all.** The sight core is a little square
        // and the marker steps *down* out of it, because a fan marker's y is the crosshair's y
        // at every angle and its x is the entire message. The old rule clamped the x, which
        // made a fixed slot out of every fan under 16 px — i.e. out of the whole low end of the
        // band as soon as the player widened his FOV.
        let centre = SCREEN * 0.5;
        for state in [ArmAimState::Free, ArmAimState::Ready] {
            let shape = shape_of(state);
            let size = Vec2::new(shape.glyph_w_px, shape.glyph_h_px);
            let mut dodged = 0;
            for side in Side::ALL {
                for step_x in 0..=256 {
                    let at = Vec2::new(SCREEN.x * step_x as f32 / 256.0, SCREEN.y * 0.5);
                    let l = layout_for(side, shape, Some(at), SCREEN, Bearing::Fan);
                    let lo = l.glyph;
                    let hi = l.glyph + size;
                    let clear = hi.x <= centre.x - SIGHT_CORE_PX
                        || lo.x >= centre.x + SIGHT_CORE_PX
                        || hi.y <= centre.y - SIGHT_CORE_PX
                        || lo.y >= centre.y + SIGHT_CORE_PX;
                    assert!(
                        clear,
                        "{state:?} {side:?} aimed at {at:?} put the glyph at {lo:?}..{hi:?}, \
                         over the {SIGHT_CORE_PX} px square the player is actually cutting"
                    );
                    if l.glyph.y != at.y - shape.glyph_h_px * 0.5 {
                        dodged += 1;
                    }
                    // **The x is the projection and nothing else** — no slot, no clamp, no
                    // rounding, at any angle. This is the assertion the fixed slot could never
                    // satisfy, and since FIND-099 it holds over the sight core too. Skipped
                    // within 60 px of either edge, where step 2's viewport clamp is the thing
                    // moving the glyph — the reachable fan reaches 252 px of 1280, so that band
                    // is 130 px of screen the fan can never be on anyway.
                    if at.x > 60.0 && at.x < SCREEN.x - 60.0 {
                        assert_eq!(
                            l.glyph.x,
                            at.x - shape.glyph_w_px * 0.5,
                            "{state:?} {side:?} aimed at {at:?} was moved to {:.1} — a fan \
                             marker's x is the projection and nothing else",
                            l.glyph.x
                        );
                    }
                }
            }
            assert!(
                dodged > 0,
                "{state:?}: the sweep crossed the middle of the screen and the glyph never once \
                 stepped out of the sight core — the guard is dead code and the test proved \
                 nothing about it"
            );
        }

        // And the dodge is exactly out of the core and not a hand-wave: the glyph's top edge
        // lands on the bottom of the square, so it touches and never overlaps.
        let shape = shape_of(ArmAimState::Ready);
        let l = layout_for(Side::Right, shape, Some(centre), SCREEN, Bearing::Fan);
        assert_eq!(l.glyph.x, centre.x - shape.glyph_w_px * 0.5, "the x was moved");
        assert_eq!(l.glyph.y, centre.y + SIGHT_CORE_PX, "the glyph did not step out of the core");
    }

    #[test]
    fn f171_a_marker_never_claims_the_wrong_half_of_the_screen() {
        // A left arm holding something on the right has to be drawn on the right — a marker
        // shoved to "its" side would be a second FIND-047, one abstraction later.
        //
        // **Since FIND-129 the claim is stronger, not weaker.** It used to be met by pushing the
        // glyph past the keep-out box on the correct side, which is how a point 26 px right of
        // centre came to be drawn 150 px out. Now the x is not moved at all, so a point in the
        // right half is drawn in the right half by arithmetic — and the letter, which is the one
        // node that could still creep inward, is what is left to check.
        let shape = shape_of(ArmAimState::Anchored);
        let middle_y = SCREEN.y * 0.5;
        let centre_x = SCREEN.x * 0.5;
        for side in Side::ALL {
            let at_right = Vec2::new(SCREEN.x * 0.52, middle_y);
            let right = layout_for(side, shape, Some(at_right), SCREEN, Bearing::World);
            assert_eq!(
                right.glyph.x,
                at_right.x - shape.glyph_w_px * 0.5,
                "{side:?} holding a point right of centre was drawn at {:?} — a world marker's \
                 x is where the rope is",
                right.glyph
            );
            assert!(right.glyph.x + shape.glyph_w_px * 0.5 > centre_x);
            assert!(right.label_right, "{side:?}: the letter has to stay outboard");

            let at_left = Vec2::new(SCREEN.x * 0.48, middle_y);
            let left = layout_for(side, shape, Some(at_left), SCREEN, Bearing::World);
            assert_eq!(
                left.glyph.x,
                at_left.x - shape.glyph_w_px * 0.5,
                "{side:?} holding a point left of centre was drawn at {:?}",
                left.glyph
            );
            assert!(left.glyph.x + shape.glyph_w_px * 0.5 < centre_x);
            assert!(!left.label_right);
            assert!(
                left.label.x + LABEL_W_PX <= left.glyph.x,
                "{side:?}: the letter of a left-hand marker crept to the inboard side"
            );

            // A **badge** — an arm with no target at all — still parks in its own side slot,
            // and that is the half the arm belongs to and not the half a point is in.
            let badge = layout_for(side, shape, None, SCREEN, Bearing::World);
            assert_eq!(badge.label_right, matches!(side, Side::Right));
        }

        // **Two arms holding the SAME world point are drawn on the same pixel**, and that is
        // the honest picture: there is one place and two ropes going to it. Until FIND-129 they
        // were shoved to opposite slots, which drew two places that did not exist. What still
        // tells them apart is the letter, and dead on the axis the arm's own side decides which
        // way it hangs — `Q` outboard left, `E` outboard right (module header).
        let on_axis = Vec2::new(SCREEN.x * 0.5, middle_y);
        let l = layout_for(Side::Left, shape, Some(on_axis), SCREEN, Bearing::World);
        let r = layout_for(Side::Right, shape, Some(on_axis), SCREEN, Bearing::World);
        assert_eq!(l.glyph, r.glyph, "one point, two ropes, and the glyphs came apart");
        assert!(!l.label_right, "Q's letter has to hang to the left of a shared point");
        assert!(r.label_right, "E's letter has to hang to the right of a shared point");
        assert!(
            l.label.x < r.label.x,
            "the two letters landed on top of each other: {:?} {:?}",
            l.label,
            r.label
        );
    }

    #[test]
    fn f171_a_marker_stays_on_the_screen() {
        // A point far outside the frustum still projects to a finite pixel a long way off, and a
        // marker parked at x = -4000 is a marker that does not exist.
        let shape = shape_of(ArmAimState::Anchored);
        for at in [
            Vec2::new(-9000.0, -4000.0),
            Vec2::new(40_000.0, 22_000.0),
            Vec2::new(f32::NAN, 12.0),
        ] {
            for side in Side::ALL {
                let l = layout_for(side, shape, Some(at), SCREEN, Bearing::World);
                assert!(
                    l.glyph.x >= 0.0 && l.glyph.x + shape.glyph_w_px <= SCREEN.x,
                    "{side:?} aimed at {at:?} left the screen: {:?}",
                    l.glyph
                );
                assert!(l.glyph.y >= 0.0 && l.glyph.y <= SCREEN.y, "{:?}", l.glyph);
            }
        }
    }
}
