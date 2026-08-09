//! The health bar — crimson, bottom centre.
//!
//! **Its producer does not exist yet.** Nothing in the running game puts a
//! [`Health`] on the local player; job R3-A builds that. So this file does the one thing that
//! is honest in the meantime: it queries `Option<&Health>` and **hides the whole element**
//! when the component is absent.
//!
//! That is deliberate and it is tested. The alternative — spawning the bar full and letting
//! it sit there at 100 % — is the failure `docs/PLAN-GAME.md` §8 names in advance as *"the
//! bar that is a picture of a bar"*: it photographs perfectly, it survives a round, and the
//! day somebody wires the real health up, nobody can tell whether it ever worked.
//!
//! The arithmetic, on the other hand, is finished and checked: [`Health`] lives in `shared`,
//! so `tests/hud.rs` inserts the **real** component and asserts the width follows it. When
//! R3-A lands, this file needs no change — only the test's "and it is hidden while absent"
//! half goes away.

use bevy::prelude::*;

use crate::data::GameData;
use crate::hud::{signal, HudElement, PLATE};
use crate::shared::{Health, LocalPlayer};

/// Marker on the track — hidden while there is no [`Health`] to show.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HealthTrack;

/// Marker on the node whose **width** shows the health.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HealthBar;

/// Centred: `left` + `width` add up to symmetric margins, without a flex parent that would
/// have to span the screen — and a parent spanning the screen would cover the middle.
const LEFT_PCT: f32 = 40.0;
const WIDTH_PCT: f32 = 20.0;
const BOTTOM_PCT: f32 = 3.0;
const HEIGHT_PX: f32 = 12.0;
const PAD_PX: f32 = 2.0;

pub fn spawn_health_bar(mut commands: Commands, data: Res<GameData>) {
    let crimson = signal(&data, "crimson");
    commands
        .spawn((
            Name::new("hud_health_track"),
            HealthTrack,
            HudElement,
            BackgroundColor(PLATE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(LEFT_PCT),
                bottom: Val::Percent(BOTTOM_PCT),
                width: Val::Percent(WIDTH_PCT),
                height: Val::Px(HEIGHT_PX),
                padding: UiRect::all(Val::Px(PAD_PX)),
                // Starts hidden. Whoever removes this line has to explain what the bar is
                // showing before R3-A exists.
                display: Display::None,
                ..default()
            },
        ))
        .with_child((
            Name::new("hud_health_fill"),
            HealthBar,
            HudElement,
            BackgroundColor(crimson),
            Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() },
        ));
}

/// Follows [`Health::fraction`] — or hides, when nobody produces health.
///
/// `Option<&Health>` and not a second query: a player without the component is **not** a
/// player at zero health, it is a player nobody has measured (the same distinction
/// `debug::metric` makes for `assert health`). Zero would be a readable, plausible, wrong
/// picture.
pub fn update_health_bar(
    players: Query<Option<&Health>, With<LocalPlayer>>,
    mut tracks: Query<&mut Node, (With<HealthTrack>, Without<HealthBar>)>,
    mut bars: Query<&mut Node, With<HealthBar>>,
) {
    let health = players.iter().next().flatten().copied();
    for mut track in &mut tracks {
        track.display = if health.is_some() { Display::Flex } else { Display::None };
    }
    let percent = health.map_or(0.0, |h| h.fraction() * 100.0);
    for mut bar in &mut bars {
        bar.width = Val::Percent(percent);
    }
}
