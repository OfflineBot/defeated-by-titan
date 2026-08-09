//! The gas bar — **the one place `F-018` becomes visible.**
//!
//! Without it the gas level is nowhere in the picture, and `F-018` stays 🟨 ("logic tested,
//! pixels unseen") no matter how green its test is. A number in the terminal is not a picture.
//!
//! Cyan, because cyan is the reserved colour for gas and Vector Gear
//! (`docs/conventions.md` §3); `F-018` calls it "blue bar" in so many words. The colour comes
//! out of `maps.ron`'s `signals:` block through [`signal`], never as a literal.
//!
//! Reads the state **of the local player** through [`LocalPlayer`] — no `.single()`, because
//! every player is one of many.
//!
//! **Bottom left, horizontal, and the fill node's `width` is the reading.** That is not a
//! layout taste: `tests/hud.rs::f170_the_gas_bar_follows_the_gas_and_not_the_clock` asserts
//! `Node::width == Val::Percent(fraction * 100)`, so a bar whose length lived in a transform
//! or in a texture offset would not be checkable at all.

use bevy::prelude::*;

use crate::data::GameData;
use crate::hud::{signal, HudElement, PLATE};
use crate::shared::{Gas, LocalPlayer};

/// Marker on the track — the dark plate the fill runs inside.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GasTrack;

/// Marker on the node whose **width** shows the gas level.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GasBar;

/// Left edge of the track, in percent of the screen width.
const LEFT_PCT: f32 = 3.0;
/// Distance from the bottom edge, in percent of the screen height.
const BOTTOM_PCT: f32 = 6.0;
/// Length of the track, in percent of the screen width.
const WIDTH_PCT: f32 = 22.0;
/// Height of the track in logical pixels. A bar you read at 60 m/s is a shape, not a number.
const HEIGHT_PX: f32 = 16.0;
const PAD_PX: f32 = 2.0;

/// Builds the bar once at `Startup`.
pub fn spawn_gas_bar(mut commands: Commands, data: Res<GameData>) {
    let cyan = signal(&data, "cyan");
    commands
        .spawn((
            Name::new("hud_gas_track"),
            GasTrack,
            HudElement,
            BackgroundColor(PLATE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(LEFT_PCT),
                bottom: Val::Percent(BOTTOM_PCT),
                width: Val::Percent(WIDTH_PCT),
                height: Val::Px(HEIGHT_PX),
                padding: UiRect::all(Val::Px(PAD_PX)),
                ..default()
            },
        ))
        .with_child((
            Name::new("hud_gas_fill"),
            GasBar,
            HudElement,
            BackgroundColor(cyan),
            Node {
                // Percent **of the track's content box** — so the reading is the number the
                // test asserts, at any resolution, with no pixel arithmetic in between.
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ));
}

/// Pulls the width onto [`Gas::fraction`].
///
/// `set_if_neq` on the `Mut<Node>` is not worth it here: `Node` is compared field by field by
/// change detection anyway, and a standing player's gas does not change, so the assignment is
/// the cheap part. What matters is that this reads `Gas` **every frame** and stores nothing —
/// a cached percentage would be a second writer of the same fact.
pub fn update_gas_bar(
    players: Query<&Gas, With<LocalPlayer>>,
    mut bars: Query<&mut Node, With<GasBar>>,
) {
    let Some(gas) = players.iter().next() else {
        return;
    };
    let percent = gas.fraction() * 100.0;
    for mut node in &mut bars {
        node.width = Val::Percent(percent);
    }
}
