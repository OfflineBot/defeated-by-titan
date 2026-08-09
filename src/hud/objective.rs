//! The objective line — amber, top centre.
//!
//! # This element is deliberately empty, and that is the finished state for today
//!
//! **There is no producer.** `mission` is a 20-line stub; the phase machine and the kill
//! counter are job R3-B's work, and the counter is to become a component on a `Mission`
//! entity with per-`PlayerId` counts (`docs/PLAN-GAME.md` §5) — not a `Resource<u32>`.
//!
//! And `hud` may not read it even once it exists: the allow list in `docs/architecture.md` is
//! **empty**, `hud → mission` is not on it, and `tests/domains.rs` falls over on the first
//! `use crate::mission`. The authority table already names `hud` as a *reader* of mission
//! state, so the edge is intended — it just has to be written down with its reason, or the
//! state has to reach `shared` the way [`Gas`](crate::shared::Gas) and
//! [`Blades`](crate::shared::Blades) did. That decision is not this job's to take.
//!
//! So what stands here is the **node and nothing else**: spawned, laid out where it belongs,
//! `Display::None`, with empty text. [`objective_text`] is the seam, one function, with the
//! signature the producer will fill.
//!
//! Why not just leave the element out? Because then "the HUD has five elements" would be
//! decided by whoever reads the list next. An empty, hidden, *named* node says what is
//! missing; a missing node says nothing. And `tests/hud.rs` holds it empty: the moment
//! somebody writes `"Kill 5 titans"` in here to make the screenshot look finished, the test
//! goes red.

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::{signal, HudElement};

/// Marker on the objective line.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ObjectiveLine;

const TOP_PCT: f32 = 3.0;
const LEFT_PCT: f32 = 30.0;
const WIDTH_PCT: f32 = 40.0;
const FONT_PX: f32 = 18.0;

/// No font asset and no `Camera2d` — `default_font` is on in `Cargo.toml`, so the default
/// `TextFont` resolves to the built-in `FiraMono-subset.ttf`.
///
/// Two field types changed in bevy 0.19 and bite anyone writing this from memory:
/// `font_size` is a [`FontSize`] (`bevy_text-0.19.0/src/text.rs:392`, enum `:487-500`) and
/// `font` is a `FontSource` (`:383`, enum `:282-307`).
pub fn spawn_objective(mut commands: Commands, data: Res<GameData>) {
    let amber = signal(&data, "amber");
    commands.spawn((
        Name::new("hud_objective"),
        ObjectiveLine,
        HudElement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(FONT_PX), ..default() },
        TextLayout::justify(Justify::Center),
        TextColor(amber),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(TOP_PCT),
            left: Val::Percent(LEFT_PCT),
            width: Val::Percent(WIDTH_PCT),
            // Hidden until something produces an objective. See the module doc.
            display: Display::None,
            ..default()
        },
    ));
}

/// The seam. `None` means: nobody is producing an objective — hide the line.
///
/// It takes what it needs as arguments and not as a `Res`, so that the day the producer
/// lands, exactly one call site changes and this function is testable without an app.
pub fn objective_text(objective: Option<&str>) -> Option<String> {
    objective.map(str::to_owned)
}

/// Writes the line — and today writes nothing, because [`objective_text`] gets `None`.
///
/// **The `None` is the point and it is not a stub.** A system that hard-codes a string here
/// would put an objective on screen that no mission produced, and the screenshot would look
/// complete. `docs/PLAN-GAME.md` §8 names that failure by name.
pub fn update_objective(mut lines: Query<(&mut Text, &mut Node), With<ObjectiveLine>>) {
    // The producer, when it exists: the mission's objective for the local player.
    let objective: Option<&str> = None;
    let text = objective_text(objective);
    for (mut line, mut node) in &mut lines {
        match &text {
            Some(t) => {
                if line.0 != *t {
                    line.0.clone_from(t);
                }
                node.display = Display::Flex;
            }
            None => {
                if !line.0.is_empty() {
                    line.0.clear();
                }
                node.display = Display::None;
            }
        }
    }
}
