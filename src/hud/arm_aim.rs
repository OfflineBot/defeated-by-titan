//! **The permanent aim preview: one marker per arm, `Q` left and `E` right, always on screen.**
//!
//! The user asked for it in so many words after playing on 2026-08-10: *"und es muss auch
//! visuell immer 2 punkte angezeigt werden so der e und q haken hingehen würden!"* — two points,
//! always, showing where the two hooks would go.
//!
//! # The hard question, and why the two markers do **not** stand on two world points
//!
//! A hook is aimed along the camera ray. The eye of [`vector::aim`](crate::vector) is
//! `translation + Y * eye_height_m`, `render`'s camera hangs on the same number, and
//! `vector::hook::update_hooks` puts **both** arms on the one [`AimPoint`] that ray produced.
//! So today the two arms genuinely go to the *same* place — and every point on a ray out of the
//! camera projects onto the *same* pixel. Two markers on two "different" world points would
//! therefore have to be invented, and an invented one is the failure this whole module is built
//! against: *"the bar that is a picture of a bar"* (`docs/PLAN-GAME.md` §8).
//!
//! The design bible does say what is supposed to make them different, and it is neither a
//! shoulder socket nor a left/right probe ray. `docs/backlog/gameplay.ron` `F-023`
//! (*Kandidatensuche mit Hemisphaeren-Aufteilung*): the candidate set is split **relative to the
//! camera forward axis into a left and a right hemisphere; `Q` serves only the left set, `E`
//! only the right one**. And `F-026` (*Highlighting der Ankerpunkte*) is this element, spelled
//! out: the two best points carry the key symbol, four states, *"jeweils durch FORM UND Farbe
//! unterschieden (Farbenblindheit)"*, acceptance *"a test player can at any time say without
//! thinking where Q and E would take him"*. That machinery hangs on discrete anchor points
//! (`F-021`), and `F-021`, `F-023`, `F-024` are all ⬜ and all live in `vector`, not here.
//!
//! So this file draws **what is true today**: each arm's own state, on its own side of the
//! screen, in four shapes. The pair stands symmetric around the crosshair for exactly as long as
//! the two arms really do share one target, and it comes apart the moment they stop sharing it —
//! which happens today whenever one arm is anchored and the other is not.
//!
//! # Why the markers sit beside the crosshair and not on it
//!
//! `F-170` keeps the central 20 % × 20 % of the screen free ([`KEEP_OUT_HIGH_PCT`]), and that is
//! the same reason the crosshair is four ticks around a hole. The aim point of a free arm
//! projects onto the exact centre of that hole, so a marker drawn *on* it would be a marker
//! drawn where nothing may be drawn. Left arm goes left of the hole, right arm right of it,
//! both below it — which is also the only arrangement in which "the left one is `Q`" needs no
//! explaining.
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
//! # What it costs per frame
//!
//! **No ray, no spatial query, no iteration over anything.** [`sense_arm_aim`] reads [`Hook`]
//! and [`AimPoint`] off the local player — both already written this tick by `vector` — and
//! compares four small components. [`shape_arm_aim`] writes a `Node` only when the shape really
//! changed, [`paint_arm_aim`] a colour only when the colour really changed, and the two `Q`/`E`
//! labels are written once at startup and never again. A standing player produces zero writes
//! per frame (`CLAUDE.md` rule 6).

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::crosshair::NEUTRAL;
use crate::hud::{signal, HudElement, KEEP_OUT_HIGH_PCT};
use crate::shared::{AimPoint, Hook, HookState, LocalPlayer, Side};

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
    /// The arm is holding. This is the one case in which the two markers say different things
    /// today.
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

/// Below the keep-out box, so the pair can never creep into the middle of the screen — whatever
/// state it is in and at whatever resolution, because the inset is a percentage and the box is
/// a percentage.
const TOP_PCT: f32 = KEEP_OUT_HIGH_PCT + 5.0;
/// Half the gap between the two markers, in percent of the width, measured from the centre line.
const SIDE_PCT: f32 = 2.0;
/// Gap between the glyph and its tether.
const TETHER_GAP_PX: f32 = 4.0;
/// Width of the tether stem.
const TETHER_W_PX: f32 = 4.0;
/// The widest glyph any state draws. The `Q`/`E` label is placed against **this** and not
/// against the current shape, so the letter does not jump sideways when the ring grows.
const GLYPH_MAX_PX: f32 = 28.0;
/// Gap between the glyph column and its letter.
const LABEL_GAP_PX: f32 = 6.0;
/// The letter. Small: it names the key, it is not the readout.
pub const LABEL_PX: f32 = 15.0;

/// **The tether points away from the centre, not towards it.** Downwards the bounding box can
/// only grow further from the keep-out box; upwards a 16 px stem would eat into it on a short
/// enough window, and `f170_nothing_covers_the_middle_of_the_screen` runs at exactly one
/// resolution.
pub fn spawn_arm_aim(mut commands: Commands, data: Res<GameData>) {
    let neutral = NEUTRAL;
    for side in Side::ALL {
        let shape = shape_of(ArmAimState::default());
        commands.spawn((
            Name::new(format!("hud_arm_marker_{side:?}")),
            ArmMarker { side, part: MarkerPart::Glyph },
            ArmAimState::default(),
            HudElement,
            BackgroundColor(neutral),
            BorderColor::all(Color::NONE),
            node_for(side, MarkerPart::Glyph, shape),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_tether_{side:?}")),
            ArmMarker { side, part: MarkerPart::Tether },
            ArmAimState::default(),
            HudElement,
            BackgroundColor(neutral),
            BorderColor::all(Color::NONE),
            node_for(side, MarkerPart::Tether, shape),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_label_{side:?}")),
            ArmMarkerLabel(side),
            HudElement,
            Text::new(key_label(side)),
            TextFont { font_size: FontSize::Px(LABEL_PX), ..default() },
            TextColor(signal(&data, "cyan")),
            label_node(side),
        ));
    }
}

/// The `Q` / `E` letter. Written once at startup and never touched again — it names a key
/// binding, and a key binding does not change 60 times a second.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmMarkerLabel(pub Side);

/// Where one node of one side stands, for one shape.
///
/// The insets are **percent of the screen** (so the pair keeps its distance from the keep-out
/// box at any window size) and the sizes are pixels (so the marker keeps its weight).
fn node_for(side: Side, part: MarkerPart, shape: ArmShape) -> Node {
    let mut node = Node { position_type: PositionType::Absolute, ..default() };
    let inset = Val::Percent(50.0 + SIDE_PCT);
    match part {
        MarkerPart::Glyph => {
            node.width = Val::Px(shape.glyph_w_px);
            node.height = Val::Px(shape.glyph_h_px);
            node.border = UiRect::all(Val::Px(shape.border_px));
            // `border_radius` is a field of `Node` in bevy 0.19 (`ui_node.rs:738`), not a
            // component of its own — so the ring and the dash are one write, not two.
            node.border_radius =
                if shape.round { BorderRadius::MAX } else { BorderRadius::ZERO };
            node.top = Val::Percent(TOP_PCT);
            match side {
                Side::Left => node.right = inset,
                Side::Right => node.left = inset,
            }
        }
        MarkerPart::Tether => match shape.tether_px {
            Some(length_px) => {
                node.width = Val::Px(TETHER_W_PX);
                node.height = Val::Px(length_px);
                node.top = Val::Percent(TOP_PCT);
                // Centred under a glyph whose width depends on the state, without a flex parent
                // — the parent would have to span both nodes and would then be one more
                // rectangle the keep-out test has to reason about.
                let centre = Val::Px((shape.glyph_w_px - TETHER_W_PX) * 0.5);
                let down = Val::Px(shape.glyph_h_px + TETHER_GAP_PX);
                match side {
                    Side::Left => {
                        node.right = inset;
                        node.margin = UiRect::top(down).with_right(centre);
                    }
                    Side::Right => {
                        node.left = inset;
                        node.margin = UiRect::top(down).with_left(centre);
                    }
                }
            }
            // `Display::None` and not a despawn — an entity that comes and goes changes the
            // archetype every time an arm catches, and would make the node count depend on when
            // you look. The crosshair's corner marks are switched off the same way.
            None => node.display = Display::None,
        },
    }
    node
}

/// The letter's node. Outward of the glyph column, never inward — inward is the keep-out box.
fn label_node(side: Side) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        top: Val::Percent(TOP_PCT),
        ..default()
    };
    let inset = Val::Percent(50.0 + SIDE_PCT);
    let out = Val::Px(GLYPH_MAX_PX + LABEL_GAP_PX);
    match side {
        Side::Left => {
            node.right = inset;
            node.margin = UiRect::right(out);
        }
        Side::Right => {
            node.left = inset;
            node.margin = UiRect::left(out);
        }
    }
    node
}

/// Reads the local player's two arms, writes [`ArmAimState`] and nothing else.
///
/// `anchorable` is the **shared** aim answer — one ray, two arms, which is what
/// `vector::hook::update_hooks` does with it. The day `F-023`'s hemispheres land, this is the
/// one line that changes: the two arms then read two different candidates, and every shape,
/// colour and test below stays as it is.
pub fn sense_arm_aim(
    players: Query<(&Hook, &AimPoint), With<LocalPlayer>>,
    mut markers: Query<(&ArmMarker, &mut ArmAimState)>,
) {
    let Some((hook, aim)) = players.iter().next() else {
        return;
    };
    for (marker, mut state) in &mut markers {
        state.set_if_neq(state_for(&hook.arm(marker.side).state, aim.anchorable));
    }
}

/// Writes `Node` — **the shape and nothing else.**
pub fn shape_arm_aim(mut markers: Query<(&ArmMarker, &ArmAimState, &mut Node)>) {
    for (marker, state, mut node) in &mut markers {
        let wanted = node_for(marker.side, marker.part, shape_of(*state));
        if *node != wanted {
            *node = wanted;
        }
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
        (&ArmAimState, &mut BackgroundColor, &mut BorderColor),
        With<ArmMarker>,
    >,
) {
    let cyan = signal(&data, "cyan");
    for (state, mut fill, mut border) in &mut markers {
        // An outline glyph is transparent inside and coloured at the edge; a filled one is the
        // other way round. Which of the two it is comes out of the shape table, so the colour
        // cannot disagree with the geometry.
        let shape = shape_of(*state);
        let ink = match state {
            ArmAimState::Free => NEUTRAL,
            ArmAimState::Ready | ArmAimState::Busy | ArmAimState::Anchored => cyan,
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
    use crate::shared::BodyId;

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
}
