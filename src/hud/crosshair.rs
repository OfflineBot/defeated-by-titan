//! `F-171` — the crosshair: **an X of four strokes around an empty middle**, in three shapes.
//!
//! # The shape is his sentence (2026-09-01)
//!
//! > *„mach zudem die verbindung zu einem einfachen crosshair und nicht so gkreise mit seiten
//! > strichen etc. sollen 4 striche wo in der mitte nichts und 45deg rotiert und gröse eher
//! > mittel bis klein. aktuell ist mittel bis groß. größe einstellbar und farbe auch!"*
//!
//! Four strokes on the diagonals ([`CrosshairPart::direction`]), rotated 45° so the element
//! is an X and never a `+`; the exact middle stays empty ([`GAP_FLOOR_PX`] — the whole sight
//! core, not just the centre pixel); the base sizes live in `game.ron: hud.crosshair` and the
//! player scales them 50–200 % (`PlayerSettings::crosshair_size_pct`) and picks the colour
//! (`PlayerSettings::crosshair_colour`, `shared::settings::CROSSHAIR_COLOURS`).
//!
//! ## The keep-out box no longer applies to this element — its replacement claim is stronger
//!
//! Until 2026-09-01 the four ticks stood OUTSIDE `F-170`'s 20 % keep-out box (128 px from the
//! centre at 1280 × 720 — measured 151 px to the far corner), because the box was defined as
//! "the crosshair's own reach". A mittel-klein X lives INSIDE that box by definition, so the
//! crosshair joins the arm markers (FIND-098/FIND-129) as a named exemption in
//! `tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen` — and what replaces the box
//! for it is `the_x_crosshair_hugs_the_centre_and_keeps_the_aim_pixel_free`: every pixel
//! within 60 px of the centre, the whole sight core empty, in every state, at 1 px sampling.
//!
//! # Why the three states differ in **geometry**
//!
//! `F-171`'s acceptance is "the states are distinguishable under colour blindness". Three
//! colours on one node satisfy every screenshot and no colour-blind player. So:
//!
//! | state | strokes | outer marks | visible nodes |
//! |---|---|---|---|
//! | [`CrosshairState::Free`] | short, thin | — | 4 |
//! | [`CrosshairState::Anchor`] | long, thick | — | 4 |
//! | [`CrosshairState::Cortex`] | long, thick | four, further out | 8 |
//!
//! Colour rides on top and carries **no information of its own** — which is also why the
//! player may recolour the Free state at will: [`paint_crosshair`] draws his choice there,
//! and keeps cyan/amber for `Anchor`/`Cortex`, because those two ARE the signals
//! (`docs/conventions.md` §3).
//! `tests/hud.rs::f171_the_three_states_differ_in_shape_not_only_in_colour` forces all three
//! `BackgroundColor`s equal and still has to be able to tell them apart.
//!
//! # Three systems, because three fields have three writers
//!
//! [`sense_crosshair`] writes [`CrosshairState`], [`shape_crosshair`] writes `Node`,
//! [`paint_crosshair`] writes `BackgroundColor`. Splitting shape from paint is what lets the
//! test neutralise the colour and still drive the real geometry code — a single system that
//! wrote both could only be tested against itself.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::data::{CrosshairTuning, GameData};
use crate::hud::{signal, HudElement, ShowWhileTuning, TUNING_Z};
use crate::shared::{AimPoint, Intent, LocalPlayer, PlayerSettings, LAYER_TITAN_CORTEX};

/// What the crosshair is looking at.
///
/// Lives on **every tick node**, not on a resource and not on the player: it is view state of
/// the local client, it is written by exactly one system, and a `Resource` would be the first
/// step towards player state in a resource (`docs/multiplayer.md` rule 2). Eight copies of one
/// enum cost nothing and keep the test able to set a state without inventing an entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrosshairState {
    /// Nothing hookable, no cortex — the ray ends in the sky or in a wall you cannot take.
    #[default]
    Free,
    /// What you are looking at is anchorable (`F-003`).
    Anchor,
    /// A cortex is in range. The only place a titan dies.
    Cortex,
}

/// Which of the eight nodes this is — four strokes, four outer marks, one per diagonal.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairPart {
    StrokeUpLeft,
    StrokeUpRight,
    StrokeDownLeft,
    StrokeDownRight,
    MarkUpLeft,
    MarkUpRight,
    MarkDownLeft,
    MarkDownRight,
}

impl CrosshairPart {
    pub const ALL: [CrosshairPart; 8] = [
        CrosshairPart::StrokeUpLeft,
        CrosshairPart::StrokeUpRight,
        CrosshairPart::StrokeDownLeft,
        CrosshairPart::StrokeDownRight,
        CrosshairPart::MarkUpLeft,
        CrosshairPart::MarkUpRight,
        CrosshairPart::MarkDownLeft,
        CrosshairPart::MarkDownRight,
    ];

    pub const fn is_mark(self) -> bool {
        matches!(
            self,
            CrosshairPart::MarkUpLeft
                | CrosshairPart::MarkUpRight
                | CrosshairPart::MarkDownLeft
                | CrosshairPart::MarkDownRight
        )
    }

    /// The unit vector from the screen centre along this part's diagonal, screen convention
    /// (`+y` is down).
    pub fn direction(self) -> Vec2 {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
        match self {
            CrosshairPart::StrokeUpLeft | CrosshairPart::MarkUpLeft => Vec2::new(-D, -D),
            CrosshairPart::StrokeUpRight | CrosshairPart::MarkUpRight => Vec2::new(D, -D),
            CrosshairPart::StrokeDownLeft | CrosshairPart::MarkDownLeft => Vec2::new(-D, D),
            CrosshairPart::StrokeDownRight | CrosshairPart::MarkDownRight => Vec2::new(D, D),
        }
    }

    /// The node's rotation: a horizontal bar turned clockwise onto its own diagonal — that is
    /// the „45deg rotiert" of the request, literally. `UL↔DR` lie on the `+45°` diagonal
    /// (screen `y` is down), `UR↔DL` on `-45°`.
    pub fn angle_deg(self) -> f32 {
        match self {
            CrosshairPart::StrokeUpLeft
            | CrosshairPart::MarkUpLeft
            | CrosshairPart::StrokeDownRight
            | CrosshairPart::MarkDownRight => 45.0,
            _ => -45.0,
        }
    }
}

/// The geometry of one state, in logical pixels, already scaled by the player's size slider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrosshairShape {
    /// Length of a stroke along its diagonal.
    pub stroke_len_px: f32,
    /// Thickness of a stroke.
    pub stroke_thick_px: f32,
    /// Distance from the exact centre to a stroke's inner end, along the diagonal — the
    /// „in der mitte nichts", never under [`GAP_FLOOR_PX`].
    pub gap_px: f32,
    /// The outer marks: their length and the gap past the stroke's outer end, or `None`.
    pub mark: Option<(f32, f32)>,
}

/// The smallest the centre gap may become, at any size setting.
///
/// The whole sight core has to stay empty — [`super::arm_aim::SIGHT_CORE_PX`] (6) on each side
/// of the aim pixel — and the nearest thing a diagonal stroke can offer the core is the
/// projection of the core's own corner onto that diagonal: `6·√2 ≈ 8.49 px`. 9.0 clears it
/// with margin, so `50 %` of a small base cannot drag a stroke over the pixel the player is
/// aiming with. Guarded by `tests/hud.rs::the_x_crosshair_hugs_the_centre_and_keeps_the_aim_
/// pixel_free`, which samples the core at 1 px pitch in every state.
pub const GAP_FLOOR_PX: f32 = 9.0;

/// The one table. `game.ron: hud.crosshair` holds the base numbers, the player's slider
/// scales them, and this function is the only place the two meet — change either and the
/// picture and the test move together.
pub fn shape_of(state: CrosshairState, x: &CrosshairTuning, size_pct: f32) -> CrosshairShape {
    // The slider's own window, repeated defensively: a NaN percentage must not become a NaN
    // node (the same rule as `render::speed_fov_deg`'s pct guard).
    let scale = if size_pct.is_finite() { (size_pct / 100.0).clamp(0.5, 2.0) } else { 1.0 };
    let gap_px = (x.gap_px * scale).max(GAP_FLOOR_PX);
    match state {
        CrosshairState::Free => CrosshairShape {
            stroke_len_px: x.stroke_len_px * scale,
            stroke_thick_px: (x.stroke_thick_px * scale).max(1.5),
            gap_px,
            mark: None,
        },
        CrosshairState::Anchor => CrosshairShape {
            stroke_len_px: x.anchor_len_px * scale,
            stroke_thick_px: (x.anchor_thick_px * scale).max(1.5),
            gap_px,
            mark: None,
        },
        CrosshairState::Cortex => CrosshairShape {
            stroke_len_px: x.anchor_len_px * scale,
            stroke_thick_px: (x.anchor_thick_px * scale).max(1.5),
            gap_px,
            mark: Some((x.mark_len_px * scale, x.mark_gap_px * scale)),
        },
    }
}

/// How many nodes a state shows. The first element of the tuple `F-171` compares.
pub const fn node_count(state: CrosshairState) -> usize {
    match state {
        CrosshairState::Cortex => 8,
        _ => 4,
    }
}

/// Nothing hooked, nothing lethal: a neutral tick.
///
/// **Deliberately not a signal colour.** Cyan, amber and crimson mean gas, cortex and danger
/// (`docs/conventions.md` §3); "the ray hits nothing worth having" means none of the three, so
/// painting it cyan would be the first leak in that rule.
/// `pub`, because [`arm_aim`](crate::hud::arm_aim) means exactly the same thing with it — "this
/// arm's ray hits nothing worth having" — and a second literal in a second file is how two
/// elements that say the same thing start looking different.
pub const NEUTRAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.75);

pub fn spawn_crosshair(mut commands: Commands) {
    for part in CrosshairPart::ALL {
        commands.spawn((
            Name::new(format!("hud_crosshair_{part:?}")),
            part,
            CrosshairState::default(),
            HudElement,
            // The origin of the search band's ruler — it stays up on the settings screen
            // (`hud::ShowWhileTuning`), and there it has to be over `plate::BACKDROP`.
            ShowWhileTuning,
            GlobalZIndex(TUNING_Z),
            BackgroundColor(NEUTRAL),
            // The rotation is per PART and never per state, so it is set once at spawn —
            // [`shape_crosshair`] writes `Node` only. Layout rotates a node about its own
            // centre (`bevy_ui-0.19.0/src/layout/mod.rs:269,299` — `local_center` is added
            // after the affine), which is what lets `node_for` place centres and this
            // component tilt them in place.
            UiTransform::from_rotation(Rot2::degrees(part.angle_deg())),
            // The real geometry follows in the first `shape_crosshair` run — it needs
            // `GameData` and the player's size, and a spawn that read both would draw the
            // same node a second way.
            Node { position_type: PositionType::Absolute, display: Display::None, ..default() },
        ));
    }
}

/// Where one part stands, for one shape — the node BEFORE its rotation.
///
/// Each part is a bar whose **centre** sits `d` pixels from the screen centre along the
/// part's own diagonal; the spawn-time `UiTransform` then tilts it about that centre. The
/// centre of the screen comes in as `left/top: 50 %` plus a pixel margin, so the element
/// rides the resolution while the bar itself stays the same weight — same idea as the old
/// percent insets, anchored on the middle instead of on a box.
fn node_for(part: CrosshairPart, shape: CrosshairShape) -> Node {
    let mut node = Node { position_type: PositionType::Absolute, ..default() };
    let (len, thick, d) = if part.is_mark() {
        match shape.mark {
            Some((mark_len_px, mark_gap_px)) => (
                mark_len_px,
                shape.stroke_thick_px,
                shape.gap_px + shape.stroke_len_px + mark_gap_px + mark_len_px * 0.5,
            ),
            // No marks in this state: the four nodes stay, with no size at all.
            // `Display::None` and not a despawn — an entity that comes and goes changes the
            // archetype 60 times a second and would make the node count depend on when you
            // look.
            None => {
                node.display = Display::None;
                return node;
            }
        }
    } else {
        (shape.stroke_len_px, shape.stroke_thick_px, shape.gap_px + shape.stroke_len_px * 0.5)
    };
    let centre = part.direction() * d;
    node.width = Val::Px(len);
    node.height = Val::Px(thick);
    node.left = Val::Percent(50.0);
    node.top = Val::Percent(50.0);
    node.margin = UiRect {
        left: Val::Px(centre.x - len * 0.5),
        top: Val::Px(centre.y - thick * 0.5),
        ..default()
    };
    node
}

/// The pure rule. Cortex beats anchor: the cortex is the only place a titan dies, so when
/// both are true the crosshair says the lethal one.
pub const fn state_for(anchorable: bool, cortex_in_range: bool) -> CrosshairState {
    if cortex_in_range {
        CrosshairState::Cortex
    } else if anchorable {
        CrosshairState::Anchor
    } else {
        CrosshairState::Free
    }
}

/// The eye point — origin between the feet, eye `eye_height_m` above it.
///
/// **This is the same formula as `vector::aim::eye`, written a second time**, because `hud`
/// may not reach into `vector` (the allow list in `docs/architecture.md` is empty). Two
/// spellings of one offset are exactly how a crosshair and a hook end up pointing at
/// different things — which is why `tests/hud.rs::f171_the_crosshair_eye_is_the_aim_eye`
/// pins the two together and goes red the day one of them moves.
pub fn eye(translation_m: Vec3, eye_height_m: f32) -> Vec3 {
    translation_m + Vec3::Y * eye_height_m
}

/// Reads the world, writes [`CrosshairState`] and nothing else.
///
/// The anchor half is free: `vector::aim` has already written [`AimPoint`] this tick, and
/// `anchorable` is exactly the question "may a hook take this".
///
/// The cortex half needs its own ray, because [`AimPoint`] does not carry the answer — and it
/// must not: `point_m`, `body` and `anchorable` are kept as separate fields precisely so that
/// nothing pre-filters the cast (`F-023`, "no hooking through walls").
///
/// # What this ray sees, and what it does not
///
/// The filter is `LAYER_TITAN_CORTEX`, so the cortex sphere answers **through the titan's own
/// head** — that is the entire purpose of the layer (`shared::layers`), an unfiltered cast
/// returns the torso in front of it and never the cortex. What the filter also does is see the
/// cortex **through a wall**, and there is no cheap guard against it today: the city's blocks
/// carry no `CollisionLayers` at all, so a second ray filtered on `LAYER_WORLD` would find
/// nothing. That is a limitation of the marker, not of this system, and the crosshair is a
/// hint — whether a cut lands is `combat`'s swept cast to decide.
pub fn sense_crosshair(
    data: Res<GameData>,
    space: SpatialQuery,
    players: Query<(Entity, &Transform, &Intent, &AimPoint), With<LocalPlayer>>,
    mut parts: Query<&mut CrosshairState>,
) {
    let Some((player, transform, intent, aim)) = players.iter().next() else {
        return;
    };
    // The same range the hook uses, out of `game.ron` — a crosshair that promises an anchor
    // the hook cannot reach is worse than none.
    let range_m = data.game.vector.hook_range_m;
    let origin = eye(transform.translation, data.game.player.eye_height_m);
    let cortex = cortex_in_range(&space, player, origin, intent.look_dir(), range_m);
    let next = state_for(aim.anchorable, cortex);
    for mut state in &mut parts {
        state.set_if_neq(next);
    }
}

/// One cortex-filtered ray. Separated from the system so it takes no `Res` and can be measured.
pub fn cortex_in_range(
    space: &SpatialQuery,
    player: Entity,
    eye_m: Vec3,
    look: Vec3,
    range_m: f32,
) -> bool {
    if !eye_m.is_finite() || !(range_m.is_finite() && range_m > 0.0) {
        return false;
    }
    let Ok(direction) = Dir3::new(look) else {
        return false;
    };
    let filter = SpatialQueryFilter::from_mask(LAYER_TITAN_CORTEX)
        .with_excluded_entities([player]);
    space.cast_ray(eye_m, direction, range_m, true, &filter).is_some()
}

/// Writes `Node` — **the shape and nothing else.** The rotation was set at spawn and never
/// moves; the size slider flows in here, so a settings click reshapes the X on the next frame
/// with no restart (`F-024`'s rule, applied to a picture).
pub fn shape_crosshair(
    data: Res<GameData>,
    settings: Res<PlayerSettings>,
    mut parts: Query<(&CrosshairPart, &CrosshairState, &mut Node)>,
) {
    let x = &data.game.hud.crosshair;
    for (part, state, mut node) in &mut parts {
        let wanted = node_for(*part, shape_of(*state, x, settings.crosshair_size_pct));
        if *node != wanted {
            *node = wanted;
        }
    }
}

/// Writes `BackgroundColor` — **the colour and nothing else**, and it carries no information
/// the shape does not already carry. The `Free` state is the player's own pick
/// (*„farbe auch!"*); `Anchor`/`Cortex` stay cyan/amber over any pick, because those two
/// states ARE the signals and a signal is not a preference (`docs/conventions.md` §3).
pub fn paint_crosshair(
    data: Res<GameData>,
    settings: Res<PlayerSettings>,
    mut parts: Query<(&CrosshairState, &mut BackgroundColor), With<CrosshairPart>>,
) {
    let cyan = signal(&data, "cyan");
    let amber = signal(&data, "amber");
    for (state, mut colour) in &mut parts {
        let wanted = match state {
            CrosshairState::Free => settings.crosshair_colour(),
            CrosshairState::Anchor => cyan,
            CrosshairState::Cortex => amber,
        };
        if colour.0 != wanted {
            colour.0 = wanted;
        }
    }
}
