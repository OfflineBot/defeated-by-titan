//! squad — fellow players and escort: going down, reviving, marking
//!
//! The Bible's four ground rules (3.6) are **not negotiable** and stand here in the code: no
//! damage between players, **no collision** between players (at this speed the single biggest
//! source of frustration there is), separate loot per player, no exclusion in public
//! instances.
//!
//! **Downed instead of dead**: "dead" is a state with a timer, not a removal of the entity.
//! That produces the most valuable moment in co-op design — somebody has to decide whether to
//! land in the middle of titan fire to pull another player back up.
//!
//! **Still empty, and two of the four rules are nonetheless kept — elsewhere, on purpose.**
//! A rule about what a *body* is belongs where bodies are made, not in a domain that would
//! have to reach across to enforce it (2026-08-19, `docs/multiplayer.md`):
//!
//! | rule | where it is kept | guard |
//! |---|---|---|
//! | no collision between players | `shared::PLAYER_COLLIDES_WITH`, attached in `player::spawn_player` | `tests/multiplayer.rs::f163a_two_players_in_the_same_spot_do_not_push_each_other` — measured at **0.194 m of shove per second** before it existed |
//! | no damage between players | `blades::cut::sweep` casts against the two titan masks and nothing else | `tests/multiplayer.rs::f162a_a_player_is_not_a_member_of_any_mask_a_blade_cuts` |
//! | separate loot | **nothing to keep yet** — there is no loot. `mission::run::KillTally` is kill credit, not loot | — |
//! | no kicking | by construction: the lobby has no kick row and `net` exposes no way to drop a named seat. Only silence plus 120 s frees a chair | `net::session::Roster` |
//!
//! What is left for this folder is the part that really is its own: **downed instead of dead**,
//! the revive, and the mark. The plugin stands in the tree so that the order in `lib.rs` is
//! right from the start and a fan-out across domains is possible without five agents creating
//! the same folder (`prompts/init.md` §17).

use bevy::prelude::*;

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, _app: &mut App) {}
}
