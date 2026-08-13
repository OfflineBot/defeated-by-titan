//! Blade pips — how many pairs are left, and how sharp the pair in use still is.
//!
//! **Economy instead of cooldowns** (`prompts/init.md` §1): blades go blunt and break, and you
//! reload at a supply point. A cooldown can be felt; a stock of five cannot, so it has to be
//! on screen.
//!
//! Cyan, because the blades are part of the Vector Gear (`docs/conventions.md` §3). Crimson
//! for a broken pair — that is the "critical state" the same table reserves it for.
//!
//! **How many pips there are is a number and therefore lives in RON**
//! (`gear.ron: blades.start_pairs`), not as a `5` in this file (`CLAUDE.md` rule 2).
//!
//! **Everything on this cluster has a producer, and both halves of the economy run.**
//! `player::spawn_player` inserts [`Blades::fresh`](crate::shared::Blades::fresh);
//! `blades::cut` books `gear.ron: blades.wear_per_hit` (0.12) off `sharpness` on every reported
//! `TitanHit`, a torso graze costing `× wear_torso_factor`; a pair spent to zero draws a spare
//! through `swap_pair`, and with no spare left [`Blades::is_broken`](crate::shared::Blades) makes
//! `cut` cast nothing at all. `blades::resupply` is the way back up, at a rack of the hub.
//!
//! So the three things this file paints are the three things that move:
//! **how many pips** ← `pairs_left` · **how wide the fill** ← `sharpness` · **crimson plate** ←
//! `is_broken()`. `tests/hud.rs` is what says the bar follows the component, and
//! `scripts/f070-hub.txt` is where a whole sortie blunts it (0.760) and a rack hones it back.

use bevy::prelude::*;

use crate::data::GameData;
use crate::hud::{signal, HudElement, PLATE};
use crate::shared::{Blades, LocalPlayer};

/// Marker on the whole cluster — hidden while there is no [`Blades`] to show.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BladeCluster;

/// One pip. `index` counts from the left, 0-based.
#[derive(Component, Clone, Copy, Debug)]
pub struct BladePip {
    pub index: u8,
}

/// The track under the pips whose **width** is the sharpness of the pair in use.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SharpnessTrack;

/// The fill inside [`SharpnessTrack`].
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SharpnessBar;

const RIGHT_PCT: f32 = 3.0;
const BOTTOM_PCT: f32 = 6.0;
const PIP_W_PX: f32 = 14.0;
const PIP_H_PX: f32 = 18.0;
const GAP_PX: f32 = 4.0;
const SHARP_H_PX: f32 = 6.0;

pub fn spawn_blade_pips(mut commands: Commands, data: Res<GameData>) {
    let cyan = signal(&data, "cyan");
    let pairs = data.gear.blades.start_pairs;
    let row_width = f32::from(pairs) * PIP_W_PX + f32::from(pairs.saturating_sub(1)) * GAP_PX;

    let cluster = commands
        .spawn((
            Name::new("hud_blade_cluster"),
            BladeCluster,
            HudElement,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Percent(RIGHT_PCT),
                bottom: Val::Percent(BOTTOM_PCT),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(GAP_PX),
                display: Display::None,
                ..default()
            },
        ))
        .id();

    let row = commands
        .spawn((
            Name::new("hud_blade_row"),
            HudElement,
            Node { column_gap: Val::Px(GAP_PX), ..default() },
        ))
        .id();
    commands.entity(cluster).add_child(row);

    for index in 0..pairs {
        let pip = commands
            .spawn((
                Name::new(format!("hud_blade_pip_{index}")),
                BladePip { index },
                HudElement,
                BackgroundColor(PLATE),
                Node { width: Val::Px(PIP_W_PX), height: Val::Px(PIP_H_PX), ..default() },
            ))
            .id();
        commands.entity(row).add_child(pip);
    }

    let track = commands
        .spawn((
            Name::new("hud_sharpness_track"),
            SharpnessTrack,
            HudElement,
            BackgroundColor(PLATE),
            Node {
                width: Val::Px(row_width),
                height: Val::Px(SHARP_H_PX),
                ..default()
            },
        ))
        .with_child((
            Name::new("hud_sharpness_fill"),
            SharpnessBar,
            HudElement,
            BackgroundColor(cyan),
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        ))
        .id();
    commands.entity(cluster).add_child(track);
}

/// Lights `pairs_left` pips and pulls the sharpness bar onto `sharpness`.
pub fn update_blade_pips(
    data: Res<GameData>,
    players: Query<Option<&Blades>, With<LocalPlayer>>,
    mut cluster: Query<&mut Node, (With<BladeCluster>, Without<SharpnessBar>)>,
    mut pips: Query<(&BladePip, &mut BackgroundColor)>,
    mut track: Query<&mut BackgroundColor, (With<SharpnessTrack>, Without<BladePip>)>,
    mut bar: Query<&mut Node, With<SharpnessBar>>,
) {
    let blades = players.iter().next().flatten().copied();
    for mut node in &mut cluster {
        node.display = if blades.is_some() { Display::Flex } else { Display::None };
    }
    let Some(blades) = blades else {
        return;
    };

    let cyan = signal(&data, "cyan");
    let crimson = signal(&data, "crimson");
    for (pip, mut colour) in &mut pips {
        colour.0 = if pip.index < blades.pairs_left { cyan } else { PLATE };
    }
    // A broken pair is not "a very short bar" — at zero sharpness the fill has no width at
    // all, and an empty plate looks exactly like a fresh HUD that has not updated yet. So the
    // plate itself goes crimson: "critical state", `docs/conventions.md` §3.
    for mut colour in &mut track {
        colour.0 = if blades.is_broken() { crimson } else { PLATE };
    }
    for mut node in &mut bar {
        node.width = Val::Percent(blades.sharpness.clamp(0.0, 1.0) * 100.0);
    }
}
