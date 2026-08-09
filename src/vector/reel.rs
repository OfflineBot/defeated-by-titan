//! `F-005` Reel-in — **a change of length, not a pulling force.**
//!
//! Build reel-in as a force toward the anchor and you get the „lineares Ziehen" that `F-004`
//! explicitly rules out. As a change of length the acceleration falls out as a side effect of
//! the rope constraint — and `shared::rope::rope_reel_in` scales the **tangential** velocity
//! by `L_prev / L_new` while it does so. That is the difference between "the player gains
//! height" and "the player gains speed", and the speed is the feel the whole game hangs on.
//!
//! This module writes only the **desired value** ([`ReelSpeed`], in m/s per side); it is
//! `player::rope::shorten_ropes` that carries it out — **per substep**, on the
//! `DistanceJoint`'s `limits.max`, clamped at `vector.min_rope_m`. One field, one writer.
//!
//! ## Three conditions, and why none of them is redundant
//!
//! A side reels in when it is **anchored**, when the button is **held**, and when
//! [`GasGrant::reel_in`] says this tick's gas has been paid. `vector::gas` documents the
//! grant as "button held *and* paid", so the second condition looks doubled — it is not:
//! `gas_budget` is chained **before** `hook::update_hooks` and therefore reads a [`Hook`]
//! that is one tick old (`vector/gas.rs`, third decision). The grant of a tick in which the
//! hook has just let go is stale by exactly that one tick, and without the button check a
//! released hook would still pull for one tick. The `Hook` read here is **this** tick's.
//!
//! And the grant is **per player**, the reel speed **per side**: two ropes reeled in at once
//! cost one player's gas and shorten both. That is a game-value question
//! (`vector.gas_reel_per_s`), not a mechanism — it stands in the report of this job.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Buttons, GasGrant, Hook, Intent, ReelSpeed, Side};

/// Writes [`ReelSpeed`] per side: `vector.reel_speed_m_s` or 0.
///
/// **The only writer of [`ReelSpeed`].** The whole component is assembled and assigned, so
/// there is no clearing system and no side that keeps pulling because nobody reset it.
/// `set_if_neq` and not a bare assignment: a component that reports itself changed on all
/// sixty ticks makes every `Changed<T>` filter after it worthless (§11), and the value is the
/// same either way.
pub fn reel_in(data: Res<GameData>, mut players: Query<(&Intent, &Hook, &GasGrant, &mut ReelSpeed)>) {
    let rate_m_s = data.game.vector.reel_speed_m_s;

    for (intent, hook, grant, mut reel) in &mut players {
        let mut wanted = ReelSpeed::default();
        if grant.reel_in && intent.pressed(Buttons::REEL_IN) {
            for side in Side::ALL {
                if hook.arm(side).state.is_anchored() {
                    wanted.m_s[side.index()] = rate_m_s;
                }
            }
        }
        reel.set_if_neq(wanted);
    }
}
