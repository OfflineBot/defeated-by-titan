//! `P5` — **the player has health, and at zero he is downed, not removed.**
//!
//! [`MovementState::Downed`] documents itself as *"out of the fight instead of dead: a state
//! with a timer, not a removed entity"* — team mates revive you (bible 3.6, `squad/`). A
//! `despawn` at zero health looks identical for one frame and then deletes the `PlayerId`,
//! the `Gas`, the hooks and the seat that a dropped connection is supposed to hold for 120 s.
//! `tests/combat.rs::p5_a_downed_player_is_a_state_and_not_a_removed_entity` goes red on it.
//!
//! ## The number comes out of `game.ron`, and it did not exist until 2026-08-09
//!
//! `game.ron: player.health = 100.0`, read through [`GameData`] and never a literal. It is
//! calibrated against `titan.ron: husk.damage = 34.0`: **three strikes and you are down** —
//! see [`super::strike`], which is the half that subtracts.
//!
//! Until that number was in the file this whole feature was inert: nothing produced a
//! [`Health`], so `assert health > 0` measured *nothing*, the HUD's crimson bar hid itself, and
//! `mission::decide`'s "every player down ⇒ `Lost`" branch queried an empty set. All three come
//! alive with [`grant`] and not one of them needed a line changed.
//!
//! ## Why `combat` installs the component and not `player::spawn_player`
//!
//! Because `src/player/` belongs to another domain, and **the components a domain writes are
//! the components it may also install** — `blades::swing::equip` says the same sentence about
//! `Swings` and settles the precedent. A player without a [`Health`] is then not a player at
//! zero health but a player nobody has measured, and every reader in the repository already
//! makes exactly that distinction with `Option<&Health>` (`hud::health_bar`, `debug::measure`).

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Health, MovementState, PlayerId, SimulationSystems};

/// Hangs `Health::full(game.ron: player.health)` on every player that does not have one.
///
/// A system and not an observer, so that a player who arrives over the wire one day
/// (`net::LocalOnly` is a seam, `docs/multiplayer.md`) is served by the same line as the local
/// one, and so that the mechanism is visible in the schedule rather than in a callback. The
/// query is archetype-filtered and empty on all but one tick of the run.
///
/// `SimulationSystems::Intent`: the sets are `.chain()`ed in `src/lib.rs`, so the insert is
/// flushed before `Drive` ([`down_at_zero`]) and before `PostStep`
/// ([`super::strike::land`]) — a player cannot be hit in the same tick he was equipped and
/// have the hit fall on a component that is not there yet.
pub fn grant(
    mut commands: Commands,
    data: Res<GameData>,
    fresh: Query<Entity, (With<PlayerId>, Without<Health>)>,
) {
    for entity in &fresh {
        commands.entity(entity).insert(Health::full(data.game.player.health));
    }
}

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
    app.add_systems(FixedUpdate, grant.in_set(SimulationSystems::Intent))
        .add_systems(FixedUpdate, down_at_zero.in_set(SimulationSystems::Drive));
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
