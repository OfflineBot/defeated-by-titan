//! `P5` — **a downed player is a state, not a removed entity.**
//!
//! [`MovementState::Downed`] documents itself as *"out of the fight instead of dead: a state
//! with a timer, not a removed entity"* — team mates revive you (bible 3.6, `squad/`). A
//! `despawn` at zero health looks identical for one frame and then deletes the `PlayerId`,
//! the `Gas`, the hooks and the seat that a dropped connection is supposed to hold for 120 s.
//! `tests/combat.rs::p5_a_downed_player_is_a_state_and_not_a_removed_entity` goes red on it.
//!
//! ## ⚠️ What is NOT here, and why it is not a decision of this file
//!
//! **Two numbers this feature needs do not exist in any RON file**, and rule 2 forbids
//! inventing them in Rust — a game value in Rust never gets tuned:
//!
//! | number | where it belongs | what is missing without it |
//! |---|---|---|
//! | `game.ron: player.health` | `PlayerTuning` in `src/data/mod.rs` | nothing can spawn a player [`Health`], so `assert health > 0` still measures *nothing* |
//! | `titan.ron: <kind>.damage` | `TitanKind` in `src/data/mod.rs` | a `Strike` in range has no amount to subtract |
//!
//! `docs/PLAN-GAME.md` §0.3 lists ten blocking RON values and **neither of these two is on the
//! list** — the hole was not seen when the plan was written. Both files and `src/data/mod.rs`
//! belong to the main head (`CLAUDE.md`), so this file carries the mechanism and the finding,
//! not a made-up 100.0.
//!
//! What *is* here is the half that needs no number: at zero the player goes to
//! [`MovementState::Downed`] and stays in the world. It works today for anybody who has a
//! [`Health`] — and it is the half that is easy to get wrong for good.

use bevy::prelude::*;

use crate::shared::{Health, MovementState, PlayerId, SimulationSystems};

/// At zero health the player is **downed**, never despawned.
///
/// One-way: leaving `Downed` is `squad`'s decision (revive), not this file's — the same split
/// [`Health::heal`] already documents.
pub fn down_at_zero(mut players: Query<(&Health, &mut MovementState), With<PlayerId>>) {
    for (health, mut state) in &mut players {
        if health.is_empty() {
            // `set_if_neq`, so a downed player does not report a changed `MovementState` on
            // all sixty ticks and make every `Changed<T>` filter behind him worthless.
            state.set_if_neq(MovementState::Downed);
        }
    }
}

/// Registered from [`super::CombatPlugin`].
///
/// `Drive` and not `PostStep`: `player::locomotion::ground_locomotion` reads
/// [`MovementState`] before every avian system in `Integrate`, so a player who goes down this
/// tick stops running in the same tick and not in the next one.
pub fn register(app: &mut App) {
    app.add_systems(FixedUpdate, down_at_zero.in_set(SimulationSystems::Drive));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_health_is_a_state_and_the_arithmetic_saturates() {
        // The component half of the claim; the ECS half is in `tests/combat.rs`, because only
        // there can an entity actually be despawned by mistake.
        let mut h = Health::full(100.0);
        h.damage(9999.0);
        assert!(h.is_empty());
        assert_eq!(h.current, 0.0, "health went negative — a second death condition");
        // And a downed player is still a player: `Downed` is a variant of `MovementState`,
        // not the absence of one.
        assert_ne!(MovementState::Downed, MovementState::default());
    }
}
