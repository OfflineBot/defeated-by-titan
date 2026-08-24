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
//! | [`health`] | `P5`: the player carries `game.ron: player.health`; at zero he is **downed**, never despawned |
//! | [`strike`] | `P5`: a titan's `Strike` in reach takes `titan.ron: <kind>.damage` off — **once per strike, not once per tick** |
//! | [`damage`] | `F-031` + `F-044`: what a landed hit is **worth**, out of the closing speed — and the collapse that emptying a titan's wound pool buys |
//! | [`combo`] | `F-041`: consecutive hits without ground contact raise a multiplier that feeds [`damage`] |
//!
//! ## What landed on 2026-08-25 — `F-031`, `F-041`, `F-044`
//!
//! Until that day this domain read [`TitanHit`](crate::shared::TitanHit) and did exactly two
//! things with it: it froze the bodies and it staggered the titan. **The `speed_m_s` the
//! message exists to carry was read by nobody** — `gear.ron: blades.damage_per_m_s` had no
//! reader anywhere in `src/`, and `titan::rig`'s `Health::full(titan.ron: <kind>.health)` had
//! no writer. [`damage`] is both halves at once; the argument is in its header.
//!
//! **That is the second way to lose.** `mission::decide` already carried the "every player down
//! ⇒ `Lost`" branch and it was inert, because nothing in the running game produced a
//! [`Health`](crate::shared::Health). It is not duplicated here; it is fed.
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
//! | the health bar draining and the mission lost | `scripts/p5-downed.txt` → `docs/images/p5-downed.png` |
//!
//! `P5`, measured on debian: strikes at ticks 449 / 539 / 629, health `100 → 66 → 32 → 0`,
//! `Downed` at 630, `MISSION LOST` at 629 against a deadline of 19 800.
//!
//! ⚠️ `F-034`'s own acceptance in the backlog is a **blind test with human testers**. That is
//! not satisfiable by an agent, and **the blind test has not been run.**

/// `F-041` — the combo multiplier: consecutive hits without ground contact.
pub mod combo;
/// `F-031` + `F-044` — **the damage formula**: what a landed hit is worth, and what emptying a
/// titan's wound pool buys. The reader `gear.ron: blades.damage_per_m_s` never had.
pub mod damage;
pub mod health;
pub mod hitstop;
pub mod strike;

use bevy::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        // Each half registers its own systems in the stage its own header argues for, instead
        // of one list here that has to repeat the reasoning in a comment.
        hitstop::register(app);
        health::register(app);
        // Before `damage`, so that the `.after(combo::bank)` edge in `damage::register` has a
        // system to point at. Bevy resolves ordering by `SystemId` and not by insertion, so
        // this is documentation rather than a requirement — but the file reads in the order the
        // tick runs in, which is worth more than the alphabet.
        combo::register(app);
        damage::register(app);
        strike::register(app);
    }
}
