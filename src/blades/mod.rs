//! blades — the blades: **the swing and the cut** (`F-030`), wear, breakage, swapping.
//!
//! **Economy instead of cooldowns.** Blades go blunt and break; you reload at supply points,
//! from the horse, or on fallen comrades.
//!
//! Writes [`Blades`](crate::shared::Blades) and sends
//! [`TitanHit`](crate::shared::TitanHit).
//!
//! ## Why the cut lives HERE and not in `combat/`
//!
//! `docs/PLAN-GAME.md` §4 hands `F-030` to one job and names *both* folders. The domain rule
//! decides which of the two actually gets the code: the swing state machine and the cast that
//! reads it have to run in the same domain, because a `Swing` component defined in `blades/`
//! and read every tick by `combat/` is an edge with no line in the allow list of
//! `docs/architecture.md` — and `tests/domains.rs` falls over on it. Putting the state into
//! `shared/` would be the other way out, and `shared/` is not this job's to write.
//!
//! So the split runs along the message instead, which is exactly what the message is for:
//!
//! | domain | job |
//! |---|---|
//! | `blades` | the swing ([`swing`]) and the swept cast ([`cut`]) → **writes `TitanHit`** |
//! | `combat` | reads `TitanHit`: the hit stop (`F-034`), health and `Downed` (`P5`) |
//!
//! Neither knows the other, both know `shared`. That is the rule, not a workaround for it.
//!
//! ## The two traps that are paid for here
//!
//! 1. **The cut is a swept `cast_shape`, not a collider and not a `Sensor`.** All three of
//!    the obvious alternatives sample *positions* once per tick, and avian's 24 substeps do
//!    not help: `SubstepSchedule` re-runs only the solver
//!    (`avian3d-0.7.0/src/dynamics/solver/schedule.rs:49-67`), broad and narrow phase run once
//!    per step. At 75 m/s the player is inside a weaver's 0.46 m cortex for **0.37 of a tick**.
//! 2. **Two filtered casts, cortex layer first** — and that is the exact opposite of the rule
//!    `vector::aim` follows. See [`cut`].
//!
//! ## The evidence
//!
//! | what | how |
//! |---|---|
//! | the numbers and the red tests | `tests/combat.rs`, `cargo test --test combat` |
//! | the cut at the tick of contact | `scripts/f030-cortex.txt` → `docs/images/f030-cortex.png` |

pub mod cut;
/// **The way back.** `F-033`: blades go blunt and break, and until 2026-08-12 nothing in the
/// game ever gave one back. The arithmetic stands in this domain because `blades` is the only
/// writer of `Blades`; the rack that asks for it is `mission`'s and sends
/// [`BladeRestockRequest`](crate::shared::BladeRestockRequest).
pub mod resupply;
pub mod swing;

use bevy::prelude::*;

use crate::shared::{BladeRestockRequest, SimulationSystems};

pub struct BladesPlugin;

impl Plugin for BladesPlugin {
    fn build(&self, app: &mut App) {
        // The channel a supply rack asks over. Registered **here** and not in `src/lib.rs` with
        // the other messages, for the same reason `VectorPlugin` registers `RefuelRequest`: it
        // is a write path into `Blades`, this domain is the only writer of `Blades`
        // (`docs/architecture.md`, authority table), and a channel into a field must not be
        // able to exist without the one system that applies it. Registering a message twice is
        // a no-op in Bevy, so moving the line to `lib.rs` later costs nothing.
        app.add_message::<BladeRestockRequest>();

        app.add_systems(
            FixedUpdate,
            // `Intent`: the swing is a booking off a button, never a force. `.chain()`,
            // because the swing of tick *n* has to be advanced on a player who already
            // carries the components — `equip` inserts them, and Bevy's automatic sync point
            // between two chained systems is what makes that true in the same tick.
            (swing::equip, swing::advance)
                .chain()
                .in_set(SimulationSystems::Intent),
        )
        .add_systems(
            FixedUpdate,
            // `Intent`, one tick after `mission::hub::restock_at_stations` wrote the request in
            // `PostStep` — the same seam and the same one tick of latency
            // `vector::gas::apply_refuel_requests` pays, and for the same reason: applying in
            // the same tick would mean ordering a `blades` system against a `mission` system,
            // which is a hidden edge past the allow list of `docs/architecture.md`.
            //
            // **Deliberately not chained against the swing**, and it does not need to be even
            // now that a second system writes `Blades`. Since 2026-08-13 `cut::cut` books
            // `gear.ron: blades.wear_per_hit` — but it does it in `PostStep`, a different set,
            // so the order inside a tick is fixed by the schedule and not by a `.chain()` here:
            // **restock first, then pay for what you cut.** That is also the order a player
            // standing at a rack while fighting would expect.
            resupply::apply_restock_requests.in_set(SimulationSystems::Intent),
        )
        .add_systems(
            FixedUpdate,
            // `PostStep`: avian's `Writeback` has run, so `Transform`, `Position` and
            // `shared::Velocity` agree (`src/lib.rs:120-131`). Asking one stage earlier would
            // sweep along a displacement the player has not made yet.
            cut::cut.in_set(SimulationSystems::PostStep),
        );
    }
}
