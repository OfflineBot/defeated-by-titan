//! The simulation tick, and the order in which input comes into being.
//!
//! Both live in `shared/` because **two** domains need them and neither may own the other:
//! `net` delivers intents, `debug` produces them from a script. If the `SystemSet` sat in
//! `net`, `debug` would need an edge to it — and the domain rule would have been softened
//! just to express an ordering (`docs/architecture.md`).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The simulation tick. Counts up in `FixedPreUpdate`, **before** anyone reads it.
///
/// It is part of the state: the seeded rng computes from `(seed, tick)`, and an `Intent`
/// carries the tick it was meant for (§6 rules 2 and 5).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tick(pub u64);

/// The three stages in which one tick's input comes into being.
///
/// **The order is the whole point.** Without it, whether a key press from the script still
/// arrives in the same tick depends on the order the systems happen to run in — and that is
/// not a design, it is a coin flip at 60 Hz.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntentSystems {
    /// Who **produces** input: the `--script` driver presses real keys.
    Source,
    /// Who **collects** it: keyboard and mouse become one `Intent` in the inbox.
    Collect,
    /// Who **delivers** it: count the tick up, hand the due intents to the players.
    Deliver,
}

/// The six stages of **one simulation step** in `FixedUpdate`.
///
/// They are chained with `.chain()` and configured in **exactly one** place: `src/lib.rs`.
/// Not in a plugin, because four domains are members — a domain that fixes the order of
/// another one is a hidden edge past the allow list. The type lives here for the same reason
/// as [`IntentSystems`].
///
/// **The order is the answer to "who wins".** It stands here and not in `.before()`/
/// `.after()` lines spread over five files, because a coin flip at 60 Hz is, over a wire, a
/// divergence nobody reproduces (`docs/architecture.md`).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SimulationSystems {
    /// The spatial index is brought up to date **before** anyone asks it: take in new
    /// bodies, strike out the queued ones, report `BodyGone`.
    Spatial,
    /// Questions to the world before anything moves: the aim ray (`F-002`). Every system in
    /// this set sees the same index snapshot.
    World,
    /// `Intent` -> state changes and bookings: debit gas (`F-018`), fire and release hooks
    /// (`F-001`). `Velocity` is **never** touched here — and exactly that gives `F-014`
    /// momentum chaining without one line of extra work: a hook swap cannot lose speed at
    /// all.
    Intent,
    /// Every contributor writes **its own** drive component by assignment, never with `+=`.
    /// The sets are disjoint, so the order inside this set provably does not matter —
    /// **deliberately no `.chain()`**.
    Drive,
    /// **The only writer of a player's `Transform`, `Velocity`, `MovementState` and
    /// `RopeLength`.** Integration, the clamp (`F-012`), substeps, the rope constraint
    /// (`F-004`), collision (`F-013`).
    Integrate,
    /// Consequences of the step that nobody wants in the middle of the integration: store
    /// this tick's buttons (edge detection in the next tick).
    PostStep,
}
