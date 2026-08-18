//! **One plate, three screens.** The bundles every menu in this domain is built out of.
//!
//! The pause screen invented this look on 2026-08-12 — a dark backdrop, a column of 240 × 44
//! buttons, one title — and the settings screen and the lobby use *these* functions rather
//! than a second version of the same twenty lines. That is the whole point of the file: a
//! second UI idiom in a game with three screens is how a game ends up with three fonts.
//!
//! **No color of its own.** `docs/conventions.md` §3 reserves amber and cyan for things in the
//! *world* — objectives, gas, anchor points — and a menu that borrows them teaches the eye a
//! second meaning for the one signal it has to read at 40 m/s. So the selected row is a
//! lighter plate, never a colored one.

use bevy::prelude::*;
use bevy::text::FontSize;

use super::{MenuRoot, PauseElement, Screen};

/// Backdrop. Dark, not black: the game behind it stays readable, so a screenshot of a paused
/// frame still shows what was paused.
///
/// **Deepened from 0.72 to 0.90 on 2026-08-13.** At 0.72 the world behind decided how legible
/// the menu was: a grey of 166 came through at 92 and the plate measured 2.44:1 against it,
/// 1.10:1 against a night sortie (FIND-092 §4) — and a brighter frame than either had never
/// been photographed. What a menu is drawn over is not a thing this file can choose, so the
/// backdrop's job is to make it not matter. At 0.90 the same 166 comes through at 56 and the
/// whole range a frame can occupy is pinned into a band the [`PLATE_EDGE`] clears by 3:1 at
/// both ends. It is still not black: the paused world reads.
pub const BACKDROP: Color = Color::srgba(0.02, 0.03, 0.05, 0.90);
/// A button, at rest.
pub const PLATE: Color = Color::srgb(0.10, 0.12, 0.15);
/// **The line around a button, and the thing that makes it a button.**
///
/// WCAG 1.4.11 asks for 3:1 on the visual information that identifies a control, and the plate
/// on its own cannot carry that: against a dark frame it would have to be lighter, against a
/// bright one darker, and the near-white [`INK`] on it needs 4.5:1 of its own, which caps the
/// plate at a luminance of 0.148. The three together have no solution short of an opaque
/// backdrop — which is the one thing [`BACKDROP`] exists not to be. An edge has no such
/// conflict: it is read against its two neighbours only, and one colour clears both.
/// `tests/menu.rs::f175_the_menu_plate_is_legible_on_any_frame` holds the arithmetic.
pub const PLATE_EDGE: Color = Color::srgb(0.70, 0.73, 0.78);
/// How thick that line is. Two, because one pixel disappears on a scaled display.
pub const EDGE_PX: f32 = 2.0;
/// A button that is the current choice — lighter, and that is the only difference.
pub const PLATE_CHOSEN: Color = Color::srgb(0.22, 0.26, 0.32);
/// Text.
pub const INK: Color = Color::srgb(0.90, 0.93, 0.96);
/// Text that explains rather than acts. Still WCAG-AA against [`BACKDROP`] and [`PLATE`].
pub const INK_DIM: Color = Color::srgb(0.66, 0.71, 0.78);

/// The full-screen column every screen hangs under. Carries [`MenuRoot`], so the pair in
/// `menu::mod` can tell whose plate is on screen.
pub fn root(screen: Screen, name: &str) -> impl Bundle {
    (
        Name::new(format!("menu_{name}")),
        PauseElement,
        MenuRoot(screen),
        BackgroundColor(BACKDROP),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(14.0),
            ..default()
        },
    )
}

/// The one big line at the top.
pub fn title(text: impl Into<String>) -> impl Bundle {
    (
        PauseElement,
        Text::new(text.into()),
        TextFont { font_size: FontSize::Px(34.0), ..default() },
        TextColor(INK),
    )
}

/// A line that says something and does nothing.
pub fn note(text: impl Into<String>) -> impl Bundle {
    (
        PauseElement,
        Text::new(text.into()),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(INK_DIM),
    )
}

/// The gap between two things standing next to each other in a [`row`].
pub const ROW_GAP: f32 = 8.0;
/// The left column of the settings grid: what a setting is called.
pub const LABEL_W: f32 = 190.0;
/// One `-` or `+`.
pub const ARROW_W: f32 = 44.0;
/// The box between the two arrows, holding the value.
pub const VALUE_W: f32 = 150.0;
/// **The whole `- value +` block.** A control that is not a triple — the `Invert Y` toggle —
/// spans exactly this, so its two edges land on the same columns every arrow does. It was 208
/// px wide and lined up with neither of them (FIND-092 §4).
pub const SPAN_W: f32 = ARROW_W * 2.0 + VALUE_W + ROW_GAP * 2.0;
/// **The width of every settings row**, stated once. Each row is centred on its own, so a row
/// that is narrower than the others does not merely end early — it moves its label inwards and
/// takes the whole grid apart.
pub const ROW_W: f32 = LABEL_W + ROW_GAP + SPAN_W;

/// A row of things side by side — a setting and its two arrows, a line of difficulties.
pub fn row() -> impl Bundle {
    (
        PauseElement,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(ROW_GAP),
            ..default()
        },
    )
}

/// A button plate `width_px` wide. Spawn the action component **with** it and the label under
/// it — see any of the three screens for the shape.
pub fn button(width_px: f32, chosen: bool) -> impl Bundle {
    (
        PauseElement,
        Button,
        BackgroundColor(if chosen { PLATE_CHOSEN } else { PLATE }),
        BorderColor::all(PLATE_EDGE),
        Node {
            width: Val::Px(width_px),
            height: Val::Px(44.0),
            // Bevy's `box_sizing` default is `BorderBox` (`bevy_ui-0.19.0/src/ui_node.rs:505`),
            // so the line is drawn **inside** the width above and the geometry FIND-092 §3
            // measured to the pixel — 280 x 44, rows 452 px wide — does not move.
            border: UiRect::all(Val::Px(EDGE_PX)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    )
}

/// The text inside a button.
pub fn label(text: impl Into<String>) -> impl Bundle {
    (
        PauseElement,
        Text::new(text.into()),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(INK),
    )
}

/// The width every full-width button in this domain has.
pub const BUTTON_W: f32 = 280.0;
