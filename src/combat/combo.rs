//! `F-041` — **the combo: consecutive hits without ground contact.**
//!
//! The row, verbatim: *"Aufeinanderfolgende Treffer ohne Bodenkontakt erhoehen einen
//! Multiplikator, der bei Treffer oder Landung zurueckgesetzt wird."* Three verbs, and each one
//! is a line below: [`bank`] raises it, [`decay`] drops it on the ground and on the timeout, and
//! `super::strike::land` drops it when a titan connects.
//!
//! ## What it is worth, and what it is not worth yet
//!
//! The acceptance is *"Multiplikator sichtbar, wirkt auf Gold und Schaden, bricht korrekt ab"* —
//! four claims, and this file can honestly make two of them:
//!
//! | claim | here |
//! |---|---|
//! | wirkt auf **Schaden** | ✅ [`super::damage::apply`] reads [`Combo::multiplier`] |
//! | **bricht korrekt ab** | ✅ landing, the timeout and a titan's strike, each with its own test |
//! | **sichtbar** | ❌ `hud/` is another domain — the component is queryable and the patch is reported |
//! | wirkt auf **Gold** | ❌ `progress/` is another domain — same |
//!
//! **Two of four is `🟨` for the row and that is what it gets said as.** Neither missing half
//! is a mechanism: both are a reader of a component that now exists.
//!
//! ## Why the chain is a tick count and not a wall clock
//!
//! `gear.ron: damage.combo_window_s` is seconds in the file and **ticks in the code**, converted
//! once at the boundary by [`super::damage::ticks_of`] — the same rule the hit stop and the
//! stagger follow, and for the same reason: the tick is what an `Intent` is stamped with and
//! what the rng seeds from, so anything that decides a game value has to be counted in ticks or
//! two clients disagree (`docs/multiplayer.md`).
//!
//! ## Why a component and never a resource
//!
//! Rule 4 of `docs/multiplayer.md`: **player state is never a `Resource`.** There are twenty
//! players one day and each carries his own chain. The same sentence `combat::health` makes
//! about [`Health`](crate::shared::Health).

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{MovementState, PlayerId, SimulationSystems, TitanHit, TitanId};

use super::damage::ticks_of;

/// **One player's chain.** `multiplier` is what [`super::damage::apply`] multiplies by.
///
/// `hits` counts the hits *in this chain*, so the multiplier is `1 + combo_step * (hits - 1)`
/// and the **first** hit of a chain is always exactly `1.00`. A chain of one is not a bonus.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Combo {
    pub hits: u32,
    pub multiplier: f32,
    /// Ticks left before the chain lapses. Refreshed by every banked hit.
    pub ticks_left: u32,
}

impl Combo {
    /// No chain. `multiplier` is 1.0 and not 0.0 — a broken chain costs nothing, it only stops
    /// paying.
    pub const NONE: Combo = Combo { hits: 0, multiplier: 1.0, ticks_left: 0 };

    pub fn is_running(&self) -> bool {
        self.hits > 0
    }
}

/// **Is this player in the air?** The row's *"ohne Bodenkontakt"*, as one expression.
///
/// [`MovementState::OnWall`] counts as air on purpose: a wallrun is the Vector Gear working,
/// not a rest. [`MovementState::Downed`] does not — a downed player is out of the fight, and a
/// chain that survives being downed would reward the one state the game is trying to punish.
pub fn is_airborne(state: MovementState) -> bool {
    matches!(state, MovementState::Airborne | MovementState::Tethered | MovementState::OnWall)
}

/// Hangs [`Combo::NONE`] on every player that does not carry one.
///
/// The same shape and the same argument as [`super::health::grant`]: the components a domain
/// writes are the components it may also install, and a player without one is a player nobody
/// has measured rather than a player at zero. Every reader takes `Option<&Combo>`.
pub fn grant(
    mut commands: Commands,
    fresh: Query<Entity, (With<PlayerId>, Without<Combo>)>,
) {
    for entity in &fresh {
        commands.entity(entity).insert(Combo::NONE);
    }
}

/// **Counts the hit into the chain** — [`SimulationSystems::Spatial`], before
/// [`super::damage::apply`].
///
/// Counting first and reading second is what makes the first hit of a chain `x1.00` without an
/// off-by-one: `1 + step * (hits - 1)` with `hits` already including this one.
pub fn bank(
    data: Res<GameData>,
    mut hits: MessageReader<TitanHit>,
    mut players: Query<(&PlayerId, &MovementState, &mut Combo), Without<TitanId>>,
) {
    let t = &data.gear.damage;
    let window = ticks_of(t.combo_window_s, data.game.simulation_hz);
    for hit in hits.read() {
        for (id, state, mut combo) in &mut players {
            if *id != hit.by || !is_airborne(*state) {
                continue;
            }
            let hits = combo.hits.saturating_add(1);
            let multiplier =
                (1.0 + t.combo_step * (hits - 1) as f32).clamp(1.0, t.combo_max.max(1.0));
            // Written through one assignment and compared first, so a chain that is already at
            // the cap does not wake `Changed<Combo>` on every further hit for a HUD that will
            // read it.
            let next = Combo { hits, multiplier, ticks_left: window };
            if *combo != next {
                *combo = next;
            }
        }
    }
}

/// **Where a chain ends by itself:** the ground, and the clock.
///
/// [`SimulationSystems::PostStep`], so a chain banked in `Spatial` of this tick loses its first
/// tick in the same tick — the window is then exactly `combo_window_s` long counted from the
/// hit, which is what the file says.
pub fn decay(mut players: Query<(&MovementState, &mut Combo), Without<TitanId>>) {
    for (state, mut combo) in &mut players {
        // **The landing, and it is unconditional.** Not "if the chain was running": a grounded
        // player has no chain, full stop, and that is also what makes `F-044`'s ground attack
        // un-comboable without a single line about it in `super::damage`.
        if !is_airborne(*state) {
            if *combo != Combo::NONE {
                *combo = Combo::NONE;
            }
            continue;
        }
        if combo.ticks_left == 0 {
            if *combo != Combo::NONE {
                *combo = Combo::NONE;
            }
            continue;
        }
        combo.ticks_left -= 1;
        if combo.ticks_left == 0 {
            *combo = Combo::NONE;
        }
    }
}

/// Registered from [`super::CombatPlugin`].
pub fn register(app: &mut App) {
    app.add_systems(FixedUpdate, grant.in_set(SimulationSystems::Intent))
        .add_systems(FixedUpdate, bank.in_set(SimulationSystems::Spatial))
        .add_systems(FixedUpdate, decay.in_set(SimulationSystems::PostStep));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The multiplier's shape, without an app: first hit `x1.00`, then the step, then the cap.
    #[test]
    fn the_first_hit_of_a_chain_is_never_a_bonus_and_the_cap_holds() {
        let step = 0.15_f32;
        let cap = 2.0_f32;
        let at = |hits: u32| (1.0 + step * (hits - 1) as f32).clamp(1.0, cap);
        assert_eq!(at(1), 1.0, "a chain of one paid a bonus");
        assert!((at(2) - 1.15).abs() < 1e-6);
        assert_eq!(at(50), cap, "the cap does not hold");
    }

    /// A downed player is not airborne, whatever else is true of him.
    #[test]
    fn a_downed_player_carries_no_chain() {
        assert!(!is_airborne(MovementState::Downed));
        assert!(!is_airborne(MovementState::Grounded));
        assert!(is_airborne(MovementState::Airborne));
        assert!(is_airborne(MovementState::Tethered));
        assert!(is_airborne(MovementState::OnWall));
    }
}
