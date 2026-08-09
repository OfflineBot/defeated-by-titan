//! combat — **what a hit means**: the impact frame, health, being downed.
//!
//! **Damage comes out of speed.** A slash from standing scratches, the same slash at 30 m/s
//! kills — and the formula belongs in the RON, not in the code (`prompts/init.md` §1, §4).
//!
//! **The cortex is the only truth:** a cortex hit kills, no matter how full the titan is.
//! Everything else is preparation.
//!
//! No splatter: titans evaporate, wounds vent steam (Bible 3.3). That had two independent
//! reasons anyway, and it stays as a style rule even without platform moderation.
//!
//! ## What stands here since 2026-08-09 — `F-034`, `P5`
//!
//! | file | what |
//! |---|---|
//! | [`hitstop`] | `F-034`: the bodies stop for `round(hit_stop_cortex_s × 60)` ticks, the tick does not |
//! | [`health`] | `P5`: at zero the player is **downed**, never despawned |
//!
//! **The cut itself is not here.** `F-030` lives in `blades/` — the swing state machine and
//! the cast that reads it have to be in one domain, and a `Swing` in `blades/` read every tick
//! by `combat/` would be an edge with no line in the allow list of `docs/architecture.md`. The
//! two halves talk through [`TitanHit`](crate::shared::TitanHit), which is exactly what the
//! message is for; the reasoning is written out in `src/blades/mod.rs`.
//!
//! So this domain reads `TitanHit` and knows nothing about how a blade is swung, in the same
//! way `titan` reads it and this domain knows nothing about how a titan is built.
//!
//! ## The evidence
//!
//! | what | how |
//! |---|---|
//! | the numbers and the red tests | `tests/combat.rs`, `cargo test --test combat` |
//! | the frozen player and the dissolving titan | `scripts/f034-hitstop.txt` → `docs/images/f034-hitstop.png` |
//!
//! ⚠️ `F-034`'s own acceptance in the backlog is a **blind test with human testers**. That is
//! not satisfiable by an agent, and **the blind test has not been run.**

pub mod health;
pub mod hitstop;

use bevy::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        // Each half registers its own systems in the stage its own header argues for, instead
        // of one list here that has to repeat the reasoning in a comment.
        hitstop::register(app);
        health::register(app);
    }
}
