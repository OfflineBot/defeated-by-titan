//! `F-171` — the dynamic crosshair: **four ticks around a hole**, in three shapes.
//!
//! # Why four nodes and not one
//!
//! The claim of `F-170` is that no HUD node covers the central 20 % × 20 % of the screen —
//! and the crosshair sits in the middle. One node with a dot in it would cover exactly the
//! pixels the player is aiming at. So the crosshair is four ticks standing **outside** the
//! keep-out box ([`KEEP_OUT_LOW_PCT`]..[`KEEP_OUT_HIGH_PCT`]), and the box itself is the hole.
//!
//! That sentence is the reason the box is 20 % and not 3 %: the number is **the crosshair's own
//! reach**, and it is why FIND-098 exempted the two arm-aim glyphs from the box instead of
//! shrinking it. Shrinking it to a width the resolved fan clears would have collapsed this
//! element to a 44 px cross and moved every pixel of `F-171`'s photographed geometry.
//!
//! That makes the crosshair wide — at 1280 × 720 the ticks stand 128 px left and right of
//! centre and 72 px above and below, because 20 % of the width is not 20 % of the height. It
//! is a deliberate consequence of the acceptance criterion, not an accident of layout.
//!
//! # Why the three states differ in **geometry**
//!
//! `F-171`'s acceptance is "the states are distinguishable under colour blindness". Three
//! colours on one node satisfy every screenshot and no colour-blind player. So:
//!
//! | state | ticks | corner marks | visible nodes |
//! |---|---|---|---|
//! | [`CrosshairState::Free`] | short, thin | — | 4 |
//! | [`CrosshairState::Anchor`] | long, thick | — | 4 |
//! | [`CrosshairState::Cortex`] | long, thick | four, further out | 8 |
//!
//! Colour rides on top (neutral / cyan / amber) and carries **no information of its own**.
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

use crate::data::GameData;
use crate::hud::{signal, HudElement, KEEP_OUT_HIGH_PCT};
use crate::shared::{AimPoint, Intent, LocalPlayer, LAYER_TITAN_CORTEX};

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

/// Which of the eight nodes this is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairPart {
    Left,
    Right,
    Up,
    Down,
    CornerUpLeft,
    CornerUpRight,
    CornerDownLeft,
    CornerDownRight,
}

impl CrosshairPart {
    pub const ALL: [CrosshairPart; 8] = [
        CrosshairPart::Left,
        CrosshairPart::Right,
        CrosshairPart::Up,
        CrosshairPart::Down,
        CrosshairPart::CornerUpLeft,
        CrosshairPart::CornerUpRight,
        CrosshairPart::CornerDownLeft,
        CrosshairPart::CornerDownRight,
    ];

    pub const fn is_corner(self) -> bool {
        matches!(
            self,
            CrosshairPart::CornerUpLeft
                | CrosshairPart::CornerUpRight
                | CrosshairPart::CornerDownLeft
                | CrosshairPart::CornerDownRight
        )
    }
}

/// The geometry of one state. Every number in logical pixels or percent of the screen.
///
/// These are **shape constants, not balancing values** — they do not belong in RON any more
/// than the fact that a bar is a rectangle does (`CLAUDE.md` rule 2 names "a titan type, a
/// blade level, a gas cost"). What would belong in RON is a UI scale factor, and there is
/// none yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrosshairShape {
    /// Length of a tick along its long axis.
    pub tick_len_px: f32,
    /// Thickness of a tick.
    pub tick_thick_px: f32,
    /// Distance of the ticks from the keep-out box, in percent of the screen.
    pub tick_gap_pct: f32,
    /// Corner marks: edge length and distance from the box, or `None` when there are none.
    pub corner: Option<(f32, f32)>,
}

/// The one table. Change a number here and both the picture and the test move together.
pub const fn shape_of(state: CrosshairState) -> CrosshairShape {
    match state {
        CrosshairState::Free => CrosshairShape {
            tick_len_px: 10.0,
            tick_thick_px: 2.0,
            tick_gap_pct: 1.0,
            corner: None,
        },
        CrosshairState::Anchor => CrosshairShape {
            tick_len_px: 22.0,
            tick_thick_px: 3.0,
            tick_gap_pct: 1.0,
            corner: None,
        },
        CrosshairState::Cortex => CrosshairShape {
            tick_len_px: 22.0,
            tick_thick_px: 3.0,
            tick_gap_pct: 1.0,
            corner: Some((12.0, 3.0)),
        },
    }
}

/// How many nodes a state shows. The first element of the tuple `F-171` compares.
pub const fn node_count(state: CrosshairState) -> usize {
    match shape_of(state).corner {
        Some(_) => 8,
        None => 4,
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
            BackgroundColor(NEUTRAL),
            node_for(part, shape_of(CrosshairState::default())),
        ));
    }
}

/// Where one part stands, for one shape.
///
/// The insets are **percent of the screen** and the sizes are pixels: the hole then scales
/// with the resolution while the ticks stay the same weight, which is what keeps the keep-out
/// test true at any window size instead of only at 1280 × 720.
fn node_for(part: CrosshairPart, shape: CrosshairShape) -> Node {
    let mut node = Node { position_type: PositionType::Absolute, ..default() };
    match shape.corner {
        Some((size, gap)) if part.is_corner() => {
            let inset = Val::Percent(KEEP_OUT_HIGH_PCT + gap);
            node.width = Val::Px(size);
            node.height = Val::Px(size);
            match part {
                CrosshairPart::CornerUpLeft => {
                    node.right = inset;
                    node.bottom = inset;
                }
                CrosshairPart::CornerUpRight => {
                    node.left = inset;
                    node.bottom = inset;
                }
                CrosshairPart::CornerDownLeft => {
                    node.right = inset;
                    node.top = inset;
                }
                _ => {
                    node.left = inset;
                    node.top = inset;
                }
            }
        }
        // No corner marks in this state: the four nodes stay, with no size at all.
        // `Display::None` and not a despawn — an entity that comes and goes changes the
        // archetype 60 times a second and would make the node count depend on when you look.
        _ if part.is_corner() => {
            node.display = Display::None;
        }
        _ => {
            let inset = Val::Percent(KEEP_OUT_HIGH_PCT + shape.tick_gap_pct);
            // Half the thickness back, so the tick is centred on the axis and not hanging
            // off it — a 1.5 px offset nobody sees until the crops are laid on top of
            // each other.
            let half = Val::Px(-shape.tick_thick_px * 0.5);
            match part {
                CrosshairPart::Left | CrosshairPart::Right => {
                    node.width = Val::Px(shape.tick_len_px);
                    node.height = Val::Px(shape.tick_thick_px);
                    node.top = Val::Percent(50.0);
                    node.margin = UiRect::top(half);
                    if part == CrosshairPart::Left {
                        node.right = inset;
                    } else {
                        node.left = inset;
                    }
                }
                _ => {
                    node.width = Val::Px(shape.tick_thick_px);
                    node.height = Val::Px(shape.tick_len_px);
                    node.left = Val::Percent(50.0);
                    node.margin = UiRect::left(half);
                    if part == CrosshairPart::Up {
                        node.bottom = inset;
                    } else {
                        node.top = inset;
                    }
                }
            }
        }
    }
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

/// Writes `Node` — **the shape and nothing else.**
pub fn shape_crosshair(mut parts: Query<(&CrosshairPart, &CrosshairState, &mut Node)>) {
    for (part, state, mut node) in &mut parts {
        let wanted = node_for(*part, shape_of(*state));
        if *node != wanted {
            *node = wanted;
        }
    }
}

/// Writes `BackgroundColor` — **the colour and nothing else**, and it carries no information
/// the shape does not already carry.
pub fn paint_crosshair(
    data: Res<GameData>,
    mut parts: Query<(&CrosshairState, &mut BackgroundColor), With<CrosshairPart>>,
) {
    let cyan = signal(&data, "cyan");
    let amber = signal(&data, "amber");
    for (state, mut colour) in &mut parts {
        let wanted = match state {
            CrosshairState::Free => NEUTRAL,
            CrosshairState::Anchor => cyan,
            CrosshairState::Cortex => amber,
        };
        if colour.0 != wanted {
            colour.0 = wanted;
        }
    }
}
