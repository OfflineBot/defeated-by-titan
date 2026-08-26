//! `F-026` + `F-027` — **the anchor field, on screen**: *there is an anchor point here.*
//!
//! > `F-026`, verbatim: *„Alle gueltigen Kandidaten werden als schwache Marker im Sichtfeld
//! > eingeblendet. Die beiden aktuell besten Punkte (links und rechts) werden hervorgehoben
//! > und tragen das jeweilige Tastensymbol. Vier Zustaende, jeweils durch FORM UND Farbe
//! > unterschieden (Farbenblindheit)."*
//!
//! **The second sentence of that spec is not implemented, on purpose, and this page is mostly
//! about why.** What is implemented is the first: the field of weak markers, `F-027`'s cap,
//! thinning and fade, `F-030a`'s cadence, and the fourth state — the point a rope is actually
//! on.
//!
//! ## Why this file exists at all: 1520 authored points that nobody could feel
//!
//! `docs/FINDINGS.md` FIND-160 measured it: `grep` for a reader of
//! [`AnchorField`] outside `src/world/` came back **empty**.
//! `F-023`'s candidate search, `F-026`'s markers and `F-027`'s density cap were tested
//! functions **with no caller** — a whole authored anchor system in the repository that no
//! player could see. This module is the consumer. It draws, it does not decide: every point it
//! puts a ring on comes out of [`AnchorField::candidates`], which is `world`'s function and
//! stays `world`'s.
//!
//! ## 🔴 THE LETTERS ARE GONE, AND THAT IS THE FEATURE
//!
//! Until 2026-08-26 this element drew the best candidate of each hemisphere as a big ring
//! carrying `Q` or `E`. **It was a lie, it was known to be a lie when it shipped, and the
//! module header said so and shipped it anyway.** `F-024` — *Snap auf Q und E*, the feature
//! that makes the hook fire at [`AnchorField`]'s best candidate — is `Unbuilt`.
//! [`fire`](crate::vector::hook) takes [`aim`](crate::vector::aim)'s **raycast** target, a
//! probe sweep over colliders that has never heard of this field. So `Q` did not go to the
//! ring that said `Q`.
//!
//! Measured end to end at the element's own stand, before the letters came off:
//!
//! ```text
//! the game's log:  hook Left anchored on body 980 at (51.00, 1.65, -1.00), 14 m dead ahead
//! that point projects to                              (640.00, 357.77)
//! this element's `Q` ring was drawn at                (445.93, 352.07)  -> 194.15 px, 17.30 deg
//! arm_aim's `Q` — the one the key OBEYS — at          (639.50, 375.50)  -> within 0.5 px in x
//! ```
//!
//! and in the test harness, `f026_exactly_one_q_and_one_e_are_on_the_screen_and_they_are_the_arms`
//! read **two `Q` glyphs 156.9 px apart and two `E` glyphs 279.7 px apart** on one screen at
//! one stand. `F-026`'s own acceptance — *„Ein Testspieler kann jederzeit ohne Nachdenken
//! sagen, wohin Q und E ihn bringen wuerden"* — was not merely unmet by that picture, it was
//! **actively contradicted**: the player was shown two answers to a one-answer question.
//!
//! **The rule, and it is the fifth instance of one family** (`FIND-098`, `FIND-099`,
//! `FIND-127`, `FIND-129`, `FIND-178`), the first of which was *known and shipped*:
//!
//! > **Never draw a promise the game does not keep. A disclosed lie is still a lie on screen** —
//! > the player does not read the module header. If a claim cannot be kept, the claim comes
//! > off; the drawing may be short, it may not be wrong.
//!
//! ⚠️ **And the fix is NOT to move the ring onto the ray's target.** That would be
//! [`arm_aim`](crate::hud::arm_aim) drawn a second time, and it would make the anchor field
//! decoration again — the exact state FIND-160 found it in. The ring stays on the authored
//! point; only the sentence it was captioned with is withdrawn.
//!
//! **What earns the letters back:** `F-024`, wiring `vector::hook` to [`AnchorField`]. The day
//! `Q` really fires at `field.best_of(.., Hemisphere::Left, 1)`, the letters belong on this
//! element again and the hemisphere rings come back with them. It is written up as a job with
//! its repro, not as a wish — `docs/BUGS.md` B-011.
//!
//! ## What this element claims today, and every word of it is true
//!
//! | claim | true today? |
//! |---|---|
//! | *"there is an anchor point here"* | **yes** — every ring is a real [`AnchorPoint`](crate::world::anchor::AnchorPoint), on its own pixel to 0.0 px |
//! | *"your rope is on this one"* | **yes** — [`MarkState::Anchored`] reads `Hook::arm(side).tip_m`, the rope's own anchor |
//! | *"pressing Q will take you to this one"* | **not said any more.** It was said, and it was false |
//!
//! ## The one promise, and it is absolute here
//!
//! **A mark is drawn on the pixel its point projects to, or it is not drawn.** No slot, no
//! clamp, no step-out, no interpolation of a screen position. That defect — a drawn thing that
//! is not where the real thing is — has been found four separate times in this HUD
//! (`FIND-098`, `FIND-099`, `FIND-127`, `FIND-129`), and the user has asked for the opposite
//! twice (*„wichtig wäre nur dass diese auch genau da sind visuell wo das seil auch landen
//! würde!"*, 2026-08-19). So this element gives itself **no** licence to move a mark:
//! [`place_anchor_marks`] writes `Camera::world_to_viewport`'s answer and nothing else, and
//! `tests/hud.rs::f026_every_anchor_mark_stands_on_its_own_projected_pixel` measures the
//! offset at **0.0 px** over a sweep of stands and looks. Where `arm_aim` allows itself 20 px
//! of vertical dodge, this allows 0.
//!
//! ## `F-170`'s keep-out box — what this element claims, and what it gives up instead
//!
//! It claims **no new exemption**. The exemption that already stands in [`crate::hud`] is
//! *"the two arm markers, whenever they carry a **place**"* — a marker with a point of its own
//! is held out of [`SIGHT_CORE_PX`] rather than out of the 20 % box, because applying the box
//! to it draws it where the rope does not go. **Every mark here carries a place**, so it is
//! the same argument applied to more markers of the same kind and not a third one.
//!
//! And it gives up more than `arm_aim` does, on both halves:
//!
//! * `arm_aim` **steps** a marker out of the sight core. This one **does not draw** it. A mark
//!   whose rectangle would touch [`SIGHT_CORE_PX`] is absent for that frame — the picture can
//!   be short, it cannot be wrong.
//! * Nothing is lost by that. A point sitting on the crosshair is the point
//!   [`crosshair`](crate::hud::crosshair) is already reporting and the point `arm_aim`'s own
//!   preview marker already stands on; this element is the *field around* it.
//!
//! `tests/hud.rs::f026_no_anchor_mark_ever_touches_the_sight_core` is the guard.
//!
//! ## Two drawn states, told apart by FORM first
//!
//! | state | form | tether | ink |
//! |---|---|---|---|
//! | [`MarkState::Candidate`] | small hollow ring, [`CANDIDATE_PX`] | — | [`NEUTRAL`], faded with distance |
//! | [`MarkState::Anchored`] | **filled** disc, [`ANCHORED_PX`] | yes | cyan |
//!
//! ⚠️ **`F-026` asks for four and gets two, and that is a deviation, not a simplification.**
//! The two that are gone are `BestLeft` and `BestRight`, and they are gone for the reason the
//! section above gives and for no other. The two that are left differ in **three** ways —
//! size, filled-vs-hollow, tether — so `F-026`'s parenthesis *(Farbenblindheit)* is still
//! satisfied with room to spare: a player who sees no colour at all can still tell the ring he
//! could hook from the disc he is hanging on. The table is read by [`form_of`] and by nothing
//! else, so a shape and a colour cannot disagree (`F-171`'s rule, and the reason
//! [`shape_anchor_marks`] and [`paint_anchor_marks`] are two systems).
//!
//! ## `F-027` — twelve, thinned, faded, and switchable off
//!
//! > *„Maximal 12 gleichzeitig sichtbare Marker, ausgewaehlt nach Bewertungsfunktion. Marker
//! > verblassen mit Distanz und werden bei hoher Punktdichte ausgeduennt … Deckkraft und
//! > Maximalanzahl sind in den Einstellungen regelbar, inklusive vollstaendiger Abschaltung."*
//!
//! Two mechanisms over the score order `world` already delivers ([`pick`]):
//!
//! 1. **screen-space thinning** at [`HudTuning::marker_min_gap_px`]: a candidate whose pixel
//!    lands within that many pixels of a mark already accepted is dropped. Thinning in *screen*
//!    space and not in world space is the point — two points 3 m apart at 150 m are one blob of
//!    ink;
//! 2. **the cap**, [`HudTuning::marker_max`], applied to what is left, best score first.
//!
//! `candidates` arrives already sorted by [`Candidate::score`] out of `world`, and this file
//! **re-derives no score of its own** — a second scoring rule in the HUD is how a marker and a
//! snap start disagreeing about which point is best, and the day `F-024` lands that would be
//! the next version of the bug this file just had.
//!
//! Fading is [`fade`], and it is applied to the weak candidates only. An anchored point keeps
//! full opacity at every distance: it is a fact about the rope, and a rope does not get less
//! true at 150 m.
//!
//! **Off is off.** `marker_max: 0` in `game.ron` draws nothing at all and — this is the half
//! that is easy to get wrong — **searches nothing at all**: [`sense_anchor_marks`] returns
//! before it touches the field, so switching the element off gives the frame time back instead
//! of hiding a cost. `tests/hud.rs::f027_switching_the_markers_off_draws_nothing_and_searches_nothing`.
//!
//! ## `F-030a` — 10 Hz for the SET, every frame for the PIXEL
//!
//! > *„… mit 10 Hz Aktualisierungsrate und Interpolation der Markerpositionen dazwischen."*
//!
//! Read literally that asks for an interpolated screen position, which is exactly the lie the
//! section above forbids. It is implemented on the reading that costs nothing and lies about
//! nothing: **the set of chosen points is refreshed at [`HudTuning::marker_refresh_hz`]; the
//! projection of each chosen point is recomputed every frame.** So a mark glides with the
//! camera perfectly smoothly — that is the interpolation the spec wants — while the pixel it
//! glides to is a real projection of a real point at every single frame, never a lerp between
//! two stale ones.
//!
//! The expensive half is the search, and the search is the half that runs at 10 Hz. Its budget
//! is `F-030a`'s *„unter 0,8 ms pro Frame"* and it is measured, not argued, in
//! `tests/hud.rs::f030a_the_candidate_search_stays_inside_its_frame_budget`.

use bevy::prelude::*;

use crate::data::{GameData, HudTuning};
use crate::hud::arm_aim::SIGHT_CORE_PX;
use crate::hud::crosshair::NEUTRAL;
use crate::hud::{signal, HudElement};
use crate::shared::{Hook, HookState, LocalPlayer, Side};
use crate::world::anchor::{AnchorField, Candidate};

/// Diameter of a weak candidate's ring, in logical pixels.
///
/// **Shape constants, not balancing values** — the same argument
/// [`ArmShape`](crate::hud::arm_aim::ArmShape) makes: `CLAUDE.md` rule 2 names *"a titan kind, a
/// blade tier, a gas cost"*, and the fact that a candidate is a small ring is none of the three.
/// The numbers in this file that **are** game values — how many marks, how far, how faded, how
/// often — all live in `game.ron: game.hud` and are read through [`HudTuning`].
pub const CANDIDATE_PX: f32 = 9.0;
/// Border width of a weak candidate's ring.
pub const CANDIDATE_BORDER_PX: f32 = 1.5;
/// Diameter of the filled disc that says *this rope is on this point*.
pub const ANCHORED_PX: f32 = 13.0;
/// Width of the anchored mark's tether stub — `F-026`'s *„mit Seilverbindung"*.
pub const TETHER_W_PX: f32 = 2.0;
/// Length of the anchored mark's tether stub.
pub const TETHER_H_PX: f32 = 10.0;

/// What one mark is saying — the two drawn states, plus the one that means *nothing to say*.
///
/// [`MarkState::Off`] is a state and not an absence because the marks are **pre-spawned and
/// hidden**, never spawned and despawned: an entity that comes and goes changes the archetype
/// every frame the player turns his head, and it would make the HUD's node count depend on when
/// you look. Same choice as [`catch_band`](crate::hud::catch_band) and the crosshair's corner
/// marks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkState {
    /// This slot has no point this frame.
    #[default]
    Off,
    /// *„Alle gueltigen Kandidaten … als schwache Marker"*.
    Candidate,
    /// *„Bereits verankert (gefuelltes Symbol mit Seilverbindung)"*.
    Anchored,
}

impl MarkState {
    /// The drawn states, in the order `F-026` lists them. `Off` is deliberately not in it.
    ///
    /// **Two, where `F-026` names four** — the module header carries the whole reason, and the
    /// short form is that the missing two carried a key symbol for a key that does not obey it.
    pub const DRAWN: [MarkState; 2] = [MarkState::Candidate, MarkState::Anchored];
}

/// One mark slot. `slot` is its index and nothing else — which point it carries is
/// [`MarkAt`], and that changes ten times a second.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorMark {
    pub slot: usize,
}

/// The tether stub of an anchored mark. One per slot; hidden unless that slot is anchored.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorMarkTether(pub usize);

/// **The world point this slot is marking**, and the distance it stands at.
///
/// `None` means the slot says nothing this frame. The `Vec3` is a *world* position, never a
/// screen position: the screen position is recomputed from it every single frame
/// ([`place_anchor_marks`]), which is what makes the promise in the module header keepable.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct MarkAt {
    pub point_m: Option<Vec3>,
    pub distance_m: f32,
}

/// The 10 Hz clock of [`sense_anchor_marks`] — `F-030a`'s *Aktualisierungsrate*.
///
/// A `Resource` and not a `Local`, for one reason: a `Local` cannot be read or set from a test,
/// and a refresh cadence nobody can force is a cadence nobody can measure. It is **not** player
/// state (`CLAUDE.md` rule 4) — it is this client's drawing cadence, the same kind of thing
/// `menu::Screen` is, and it carries no per-player field.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct MarkClock {
    /// Seconds since the last refresh of the *set*.
    pub since_s: f32,
    /// How many refreshes have happened. Only a test reads it, and it is what tells a stale set
    /// apart from a set that happens to be the same twice.
    pub refreshes: u64,
}

impl MarkClock {
    /// Is a refresh due, and if so consume the time.
    ///
    /// Pure, so `tests/hud.rs` can pin the cadence without a `Time` in the loop.
    pub fn due(&mut self, dt_s: f32, hz: f32) -> bool {
        self.since_s += dt_s.max(0.0);
        let period_s = if hz > 0.0 { 1.0 / hz } else { 0.0 };
        // **The first frame always searches.** A HUD that is blank for the first 100 ms is not
        // a bug a player would ever see, but a HUD that is blank until the clock happens to
        // roll over is a HUD no test can look at without waiting on a wall clock — and a test
        // that has to wait on a wall clock is a test that measures the machine.
        if self.refreshes == 0 || self.since_s >= period_s {
            self.since_s = 0.0;
            self.refreshes += 1;
            true
        } else {
            false
        }
    }
}

/// The geometry of one state, in logical pixels — the whole table, in one place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkForm {
    pub size_px: f32,
    /// `0.0` means the symbol is **filled**, anything else means it is a ring.
    pub border_px: f32,
    /// `F-026`'s *Seilverbindung* — only the anchored state has one.
    pub tether: bool,
}

/// The drawn states as different rectangles.
///
/// The one place the numbers live. `tests/hud.rs::f026_the_drawn_states_differ_in_form_not_only_in_colour`
/// asserts that no two rows are equal — a table with two rows that read the same is a table
/// that explains nothing, and it is exactly what a colour-blind player would be left with.
/// **No row carries a key symbol**, and `tests/hud.rs::f026_the_field_marks_name_no_key`
/// keeps it that way: the letters belong to the marker the key obeys.
pub const fn form_of(state: MarkState) -> MarkForm {
    match state {
        MarkState::Off => MarkForm { size_px: 0.0, border_px: 0.0, tether: false },
        MarkState::Candidate => {
            MarkForm { size_px: CANDIDATE_PX, border_px: CANDIDATE_BORDER_PX, tether: false }
        }
        MarkState::Anchored => MarkForm { size_px: ANCHORED_PX, border_px: 0.0, tether: true },
    }
}

/// `F-027`'s *„Marker verblassen mit Distanz"* — opacity from distance, `1.0` at the hand and
/// [`HudTuning::marker_far_opacity`] at the end of the reach.
///
/// Linear, clamped at both ends, and it never reaches zero: a mark that fades to invisible is a
/// mark whose *presence* carries information the player cannot read back, and `F-027`'s cap
/// already does the job of not drawing what should not be drawn.
pub fn fade(distance_m: f32, range_m: f32, far_opacity: f32) -> f32 {
    if range_m <= 0.0 {
        return far_opacity;
    }
    let t = (distance_m / range_m).clamp(0.0, 1.0);
    1.0 + (far_opacity - 1.0) * t
}

/// One chosen mark: a point, its distance, and what it is saying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub point_m: Vec3,
    pub distance_m: f32,
    pub state: MarkState,
}

/// **`F-027`'s two mechanisms** — the screen-space thinning, then the cap, over the score
/// order `world` delivered.
///
/// `on_screen` answers *"where does this world point land, in logical pixels"* and returns
/// `None` for a point the camera cannot show. It is a closure and not a `Camera` so that this —
/// the whole selection rule — is testable without a render target, a window or a viewport.
///
/// The order is the design's: *„ausgewaehlt nach Bewertungsfunktion"*, and `candidates` arrives
/// already sorted by [`Candidate::score`] out of `world`. This function
/// re-derives no score of its own — a second scoring rule in the HUD is how a marker and a snap
/// start disagreeing about which point is best.
///
/// `anchored` is **not** a selection: it is the two ropes, read off `Hook`, and it goes in
/// first because a point you are hanging on is worth drawing whether or not it is still a
/// candidate — after half a swing it usually is not.
pub fn pick(
    field: &AnchorField,
    candidates: &[Candidate],
    anchored: [Option<Vec3>; 2],
    eye_m: Vec3,
    tuning: &HudTuning,
    mut on_screen: impl FnMut(Vec3) -> Option<Vec2>,
) -> Vec<Pick> {
    let mut out: Vec<Pick> = Vec::new();
    if tuning.marker_max == 0 {
        return out;
    }
    let mut taken: Vec<Vec2> = Vec::new();

    // 1. The ropes. Not a claim about a key — a statement about where a rope of yours ends,
    //    read off `Hook` and nowhere else.
    for side in Side::ALL {
        if let Some(point_m) = anchored[side.index()] {
            let pick = Pick {
                point_m,
                distance_m: point_m.distance(eye_m),
                state: MarkState::Anchored,
            };
            if let Some(px) = on_screen(pick.point_m) {
                taken.push(px);
            }
            out.push(pick);
        }
    }

    // 2. + 3. The field behind them: thinned in screen space, then capped.
    let gap2 = tuning.marker_min_gap_px * tuning.marker_min_gap_px;
    for candidate in candidates {
        if out.len() >= tuning.marker_max as usize {
            break;
        }
        let Some(point) = field.get(candidate.index) else {
            continue;
        };
        // The ropes are already in — and they are in by POINT, not by index, so an anchored
        // arm's point cannot come back a second time as a weak ring underneath its own disc.
        if out.iter().any(|p| p.point_m == point.position_m) {
            continue;
        }
        let Some(px) = on_screen(point.position_m) else {
            continue;
        };
        if taken.iter().any(|t| t.distance_squared(px) < gap2) {
            continue;
        }
        taken.push(px);
        out.push(Pick {
            point_m: point.position_m,
            distance_m: candidate.distance_m,
            state: MarkState::Candidate,
        });
    }
    out
}

/// `marker_max` slots, every one of them hidden, plus a tether stub each.
pub fn spawn_anchor_marks(mut commands: Commands, data: Res<GameData>) {
    let cyan = signal(&data, "cyan");
    for slot in 0..data.game.hud.marker_max as usize {
        commands.spawn((
            Name::new(format!("hud_anchor_mark_{slot}")),
            AnchorMark { slot },
            MarkState::default(),
            MarkAt::default(),
            HudElement,
            BackgroundColor(Color::NONE),
            BorderColor::all(NEUTRAL),
            hidden(Node {
                position_type: PositionType::Absolute,
                border_radius: BorderRadius::MAX,
                ..default()
            }),
        ));
        commands.spawn((
            Name::new(format!("hud_anchor_tether_{slot}")),
            AnchorMarkTether(slot),
            HudElement,
            BackgroundColor(cyan),
            hidden(Node {
                position_type: PositionType::Absolute,
                width: Val::Px(TETHER_W_PX),
                height: Val::Px(TETHER_H_PX),
                ..default()
            }),
        ));
    }
}

fn hidden(mut node: Node) -> Node {
    node.display = Display::None;
    node.left = Val::Px(0.0);
    node.top = Val::Px(0.0);
    node
}

/// **The search** — `F-023`'s candidate list, `F-027`'s selection, at `marker_refresh_hz`.
///
/// It writes [`MarkState`] and [`MarkAt`] on the slots and nothing else; every pixel is
/// [`place_anchor_marks`]' business, and it runs every frame over what this left behind.
///
/// In `PostUpdate` after `CameraUpdateSystems` for the reason
/// [`place_arm_aim`](crate::hud::arm_aim::place_arm_aim) is: it needs the camera's
/// `GlobalTransform`, and the thinning in [`pick`] needs the camera's projection — thinning in
/// world metres would drop a mark that is nowhere near another mark on screen and keep two that
/// are one blob of ink.
pub fn sense_anchor_marks(
    time: Res<Time>,
    data: Res<GameData>,
    mut clock: ResMut<MarkClock>,
    field: Option<Res<AnchorField>>,
    players: Query<&Hook, With<LocalPlayer>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut marks: Query<(&AnchorMark, &mut MarkState, &mut MarkAt)>,
) {
    let tuning = &data.game.hud;
    // **Off is off, and it is off before the search and not after it** — the whole cost of the
    // element is the candidate query, so a switch that only hid the rings would be a switch
    // that gave the player nothing back.
    if tuning.marker_max == 0 {
        clear(&mut marks);
        return;
    }
    if !clock.due(time.delta_secs(), tuning.marker_refresh_hz) {
        return;
    }
    let (Some(field), Some((camera, camera_at))) = (field, cameras.iter().next()) else {
        clear(&mut marks);
        return;
    };
    let eye_m = camera_at.translation();
    let forward = camera_at.forward().as_vec3();
    let up = camera_at.up().as_vec3();
    let range_m = data.game.vector.hook_range_m;

    let candidates = field.candidates(
        eye_m,
        forward,
        up,
        range_m,
        tuning.marker_cone_h_deg,
        tuning.marker_cone_v_deg,
    );

    // `F-026`'s fourth state comes from the rope and not from the field: an anchor you are
    // hanging on does not have to still be a candidate to be worth drawing, and often is not —
    // you swing past it and it leaves the cone.
    let hook = players.iter().next();
    let anchored = Side::ALL.map(|side| {
        hook.and_then(|h| {
            let arm = h.arm(side);
            matches!(arm.state, HookState::Anchored { .. }).then_some(arm.tip_m)
        })
    });

    let picks = pick(&field, &candidates, anchored, eye_m, tuning, |p| {
        camera.world_to_viewport(camera_at, p).ok()
    });

    for (mark, mut state, mut at) in &mut marks {
        let want = picks.get(mark.slot);
        let want_state = want.map_or(MarkState::Off, |p| p.state);
        let want_at = MarkAt {
            point_m: want.map(|p| p.point_m),
            distance_m: want.map_or(0.0, |p| p.distance_m),
        };
        if *state != want_state {
            *state = want_state;
        }
        if *at != want_at {
            *at = want_at;
        }
    }
}

fn clear(marks: &mut Query<(&AnchorMark, &mut MarkState, &mut MarkAt)>) {
    for (_, mut state, mut at) in marks.iter_mut() {
        if *state != MarkState::Off {
            *state = MarkState::Off;
        }
        if at.point_m.is_some() {
            *at = MarkAt::default();
        }
    }
}

/// The **form** half of the table — size, and ring-or-filled.
pub fn shape_anchor_marks(mut marks: Query<(&MarkState, &mut Node), With<AnchorMark>>) {
    for (state, mut node) in &mut marks {
        let form = form_of(*state);
        let (w, h) = (Val::Px(form.size_px), Val::Px(form.size_px));
        if node.width != w {
            node.width = w;
        }
        if node.height != h {
            node.height = h;
        }
        let border = UiRect::all(Val::Px(form.border_px));
        if node.border != border {
            node.border = border;
        }
    }
}

/// The **ink** half — and it carries nothing the form does not already carry.
///
/// Cyan is `maps.ron`'s *"gas, Vector Gear, anchor points"*, read through
/// [`signal`](crate::hud::signal) and never written as a literal here. A weak candidate is
/// [`NEUTRAL`] — the crosshair's own *"nothing worth having yet"* grey, borrowed rather than
/// spelled a second time — at [`fade`]'s opacity.
pub fn paint_anchor_marks(
    data: Res<GameData>,
    mut marks: Query<
        (&MarkState, &MarkAt, &mut BackgroundColor, &mut BorderColor),
        With<AnchorMark>,
    >,
) {
    let cyan = signal(&data, "cyan");
    let tuning = &data.game.hud;
    let range_m = data.game.vector.hook_range_m;
    for (state, at, mut fill, mut border) in &mut marks {
        let ink = match state {
            MarkState::Off => Color::NONE,
            // A rope does not get less true at 150 m — module header.
            MarkState::Anchored => cyan,
            MarkState::Candidate => NEUTRAL.with_alpha(
                NEUTRAL.alpha() * fade(at.distance_m, range_m, tuning.marker_far_opacity),
            ),
        };
        // A ring is transparent inside and coloured at the edge; a filled symbol is the other
        // way round. Which of the two it is comes out of [`form_of`], so the colour cannot
        // disagree with the geometry.
        let (want_fill, want_border) =
            if form_of(*state).border_px > 0.0 { (Color::NONE, ink) } else { (ink, Color::NONE) };
        if fill.0 != want_fill {
            fill.0 = want_fill;
        }
        if border.top != want_border {
            border.set_all(want_border);
        }
    }
}

/// **The pixel** — every frame, for every slot, and it is the projection or it is nothing.
///
/// Three ways a mark is not drawn, and none of them moves it:
/// 1. the slot has no point ([`MarkState::Off`]);
/// 2. `Camera::world_to_viewport` refuses the point — behind the near plane, or off the
///    viewport. `arm_aim` keeps such a point's *bearing* and clamps it to the screen edge,
///    because an arm marker has to exist always; a field mark has no such duty and a ring on
///    the edge of the screen would claim a point that is not there;
/// 3. the mark's own rectangle would touch [`SIGHT_CORE_PX`] — the pixels the player is
///    cutting. See the module header: this element does not step out of the core, it stands
///    down.
pub fn place_anchor_marks(
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut marks: Query<(&AnchorMark, &MarkState, &MarkAt, &mut Node), Without<AnchorMarkTether>>,
    mut tethers: Query<(&AnchorMarkTether, &mut Node), Without<AnchorMark>>,
) {
    let camera = cameras.iter().next();
    let mut at_px: Vec<Option<(Vec2, MarkState)>> = Vec::new();
    for (mark, state, at, mut node) in &mut marks {
        if at_px.len() <= mark.slot {
            at_px.resize(mark.slot + 1, None);
        }
        let placed = camera.zip(at.point_m).and_then(|((camera, camera_at), point_m)| {
            camera.world_to_viewport(camera_at, point_m).ok()
        });
        let form = form_of(*state);
        let drawn = placed.filter(|px| *state != MarkState::Off && !over_core(*px, form, camera));
        at_px[mark.slot] = drawn.map(|px| (px, *state));
        match drawn {
            Some(px) => put(&mut node, px - Vec2::splat(form.size_px * 0.5)),
            None => hide(&mut node),
        }
    }
    for (tether, mut node) in &mut tethers {
        match at_px
            .get(tether.0)
            .copied()
            .flatten()
            .filter(|(_, s)| *s == MarkState::Anchored)
        {
            // Hanging straight down off the disc: `F-026`'s *Seilverbindung* as a stub, not as a
            // second rope. `render::rope` draws the rope itself in 3D and this may not compete
            // with it — two drawings of one rope is how they start disagreeing.
            Some((px, _)) => {
                put(&mut node, px + Vec2::new(-TETHER_W_PX * 0.5, ANCHORED_PX * 0.5))
            }
            None => hide(&mut node),
        }
    }
}

/// Would this mark's rectangle sit on the pixels the player is cutting?
///
/// The core is the centre of the **viewport**, which is where the crosshair is.
fn over_core(px: Vec2, form: MarkForm, camera: Option<(&Camera, &GlobalTransform)>) -> bool {
    let Some(viewport) = camera.and_then(|(c, _)| c.logical_viewport_size()) else {
        return false;
    };
    let core = viewport * 0.5;
    let half = form.size_px * 0.5;
    (px.x - core.x).abs() < half + SIGHT_CORE_PX && (px.y - core.y).abs() < half + SIGHT_CORE_PX
}

fn put(node: &mut Node, top_left: Vec2) {
    let (left, top) = (Val::Px(top_left.x), Val::Px(top_left.y));
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
}

fn hide(node: &mut Node) {
    if node.display != Display::None {
        node.display = Display::None;
    }
}
