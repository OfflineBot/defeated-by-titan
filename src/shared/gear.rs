//! The Vector Gear as **state**: hooks, rope length, aim point, the gas booking, the drives.
//!
//! The types live in `shared/` although `vector` and `player` write them, because `hud`,
//! `sound`, `render` and `debug` have to **read** them (`F-001`: "states are visible in the
//! HUD"). Who writes stands in the authority table in `docs/architecture.md` — not in the
//! type.
//!
//! **No `Entity` and no pointer in any field.** A hook hangs on a
//! [`BodyId`](super::ids::BodyId); when the carrier disappears, the index reports it and the
//! hook releases. That way every field here survives a snapshot, a rollback and one day a
//! wire (`docs/multiplayer.md` rules 7 and 8).
//!
//! ## The split you expect while reading and do not find here
//!
//! The **rope length is not in the hook**. `Hook` belongs to `F-001` (the state machine),
//! the shortening belongs to `F-005`, and the constraint is enforced by the integrator. Had
//! the length lived in `Hook`, three jobs would have written the same field. This way it is
//! written by exactly the one who also enforces it: [`RopeLength`] belongs to
//! `player::integrator::step`.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::ids::BodyId;
use super::intent::Buttons;

/// Left or right — two **independently** steerable hooks (`F-001`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// Index into [`Hook::arms`] and [`RopeLength`]. `0 = left`, `1 = right` — the same
    /// order as in [`super::rope::rope_step`].
    pub fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    pub const ALL: [Side; 2] = [Side::Left, Side::Right];
}

/// The four states `F-001` names, in its own words: "idle, flying, anchored, retracting".
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum HookState {
    #[default]
    Idle,
    /// The tip flies towards `target_m` at `vector.hook_speed_m_s`.
    Flying {
        target_m: Vec3,
        /// Which body the aim ray hit. It can disappear before the impact — then the hook
        /// releases with `ReleaseReason::BodyGone`.
        body: BodyId,
    },
    /// Anchored. The anchor point stands **in the body's frame**, not in the world: if the
    /// carrier moves (from `F-029` on), the anchor rides along.
    Anchored {
        body: BodyId,
        local_m: Vec3,
    },
    /// The tip comes back at `vector.hook_retract_speed_m_s`.
    Retracting,
}

impl HookState {
    pub fn is_anchored(&self) -> bool {
        matches!(self, HookState::Anchored { .. })
    }
}

/// One hook arm: its state and where its tip currently is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HookArm {
    pub state: HookState,
    /// World position of the tip — for the rope on screen and the impact sound.
    /// Meaningless while `Idle`.
    pub tip_m: Vec3,
}

/// Both hooks of one player.
///
/// **One component with two slots, no child entities**: Bevy does not hang the same
/// component on an entity twice, and child entities would be `Entity` references inside
/// something that gets saved.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Hook {
    pub arms: [HookArm; 2],
}

impl Hook {
    pub fn arm(&self, side: Side) -> &HookArm {
        &self.arms[side.index()]
    }

    /// How many arms are anchored right now — the number behind `assert hooks` in a script.
    pub fn anchored_count(&self) -> u32 {
        self.arms.iter().filter(|a| a.state.is_anchored()).count() as u32
    }
}

/// The enforced rope length per side. `0.0` means **no constraint**.
///
/// Set at the moment of anchoring, shortened after that by `F-005` and — when the wall wins
/// — **paid out** to the actual distance (`docs/interface.md`, "rope versus wall, and who
/// referees"). Never fought against a wall.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RopeLength {
    pub lengths_m: [f32; 2],
    /// The rope had to be paid out beyond `vector.hook_range_m`.
    ///
    /// The integrator sets the flag, `vector::hook::update_hooks` reads it in the **next**
    /// tick and releases with `ReleaseReason::Overextended`. One tick of delay, and in
    /// exchange exactly one writer per field — instead of a second writer on `Hook`.
    pub overextended: [bool; 2],
}

impl RopeLength {
    pub fn length_m(&self, side: Side) -> f32 {
        self.lengths_m[side.index()]
    }
}

/// Where a hook would fly **right now** (`F-002`, free aiming) — the **centre** ray.
///
/// Valid for one tick, recomputed every tick. `F-002` in its own words: "this layer stays
/// ALWAYS active and is never replaceable by the snap system."
///
/// Since `F-023` this is the **crosshair's** source and no longer the hook's: what the two
/// arms fire at stands in [`ArmAim`], which `vector::aim` writes in the same tick out of two
/// further rays. The two are kept apart because they answer different questions — "what is
/// under the crosshair" is one point, "where would Q and E take me" is two.
///
/// `#[require(ArmAim)]`: an entity that aims carries both, always, without anybody having to
/// remember it at the spawn site. `player::spawn_player` is not this domain's file, and a
/// component that has to be inserted in a foreign file to be written here is a rule nobody
/// can see (`docs/architecture.md`, file ownership).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[require(ArmAim)]
pub struct AimPoint {
    /// First **solid** hit of the look ray, in world coordinates. `None` means: nothing in
    /// range.
    pub point_m: Option<Vec3>,
    /// Which body was hit.
    pub body: Option<BodyId>,
    /// Whether what was hit is anchorable (`F-003`).
    ///
    /// **Kept apart from `point_m`, not pre-filtered.** A ray that skips untagged bodies
    /// hooks through walls — `F-023` forbids that in so many words.
    pub anchorable: bool,
}

/// Where **each arm's** hook would fly right now — the hemisphere split of `F-023`.
///
/// The user, 2026-08-12: *„es muss mehr rechts und links spreaden!! … und da wo das seil am
/// ende auch landet soll die markierung hin vom seil"*. `F-023` had specified it long before
/// that: the candidate set is split relative to the camera forward axis, "Q bedient
/// ausschliesslich die linke Menge, E ausschliesslich die rechte".
///
/// **Same type per side as the centre ray**, not a third shape. The plan sketched an
/// `ArmAim { point_m, body, anchorable }`, which is [`AimPoint`] field for field; a second
/// struct with the same three fields would mean two spellings of one answer, and
/// `vector::hook::anchor_target` would need a copy. What is per-arm here is the *ray*, not
/// the *answer's shape* — so the arms hold `[AimPoint; 2]`, indexed like [`Hook::arms`] and
/// [`RopeLength::lengths_m`] by [`Side::index`].
///
/// ## The one rule that makes it worth a component
///
/// **This is what the hook fires at, and what the HUD draws — the same value, the same
/// tick.** `vector::aim` resolves everything here, including the fallback to the centre ray
/// when a side ray finds nothing anchorable; `vector::hook` re-casts nothing at fire time and
/// `hud::arm_aim` computes nothing of its own. That is why the user's *„und dann muss das
/// seil auch dahin!!"* holds by construction instead of by agreement between two files
/// (`docs/FINDINGS.md` FIND-047: the markers used to be fixed screen badges that never moved).
///
/// **One writer:** `vector::aim::aim`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArmAim {
    pub arms: [AimPoint; 2],
}

impl ArmAim {
    pub fn side(&self, side: Side) -> &AimPoint {
        &self.arms[side.index()]
    }

    /// Where this arm's rope would land, or `None` when the arm has nothing to fire at.
    ///
    /// The HUD marker and the hook read **this**; whoever draws a marker from anything else
    /// is drawing a promise the rope does not keep.
    pub fn target_of(&self, side: Side) -> Option<Vec3> {
        self.side(side).point_m
    }
}

/// Result of **this** tick's gas booking (`F-018`).
///
/// Whoever reads `false` here got no gas and writes zero into its drive. Without that detour
/// `F-005` and `F-007` would both call `Gas::try_spend` — two writers on one field, and with
/// a nearly empty tank the system order would decide who pays. The **priority** is a
/// balancing decision and stands in `assets/data/game.ron` (`vector.gas_priority`), not as
/// an `if` in the code.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GasGrant {
    pub boost: bool,
    pub reel_in: bool,
    /// `F-006` rope steering (`docs/NEXT.md` §1B). True while a hook is anchored, a movement key
    /// is down **and** this tick's `gas_steer_per_s` was paid — so
    /// `player::locomotion::air_control` needs no second condition of its own, exactly like
    /// [`boost`](Self::boost). **On `false` the rope term is zero, not halved**: the free-air
    /// look term keeps `air_accel_empty_fraction` on an empty tank, the rope term does not exist
    /// without gas.
    pub steer: bool,
    /// `F-008` dodge — **true on exactly one tick per double-tap**, never on two in a row.
    ///
    /// The other two are grants for a *held* button and stay true while it is held. This one
    /// is a grant for an *edge*: `net::local::read_input` presses `Buttons::DODGE` on the tick
    /// the second `Space` press lands and on no other, so a reader may treat `true` as "the
    /// impulse happens now" and needs no edge detection of its own.
    pub dodge: bool,
}

/// Contribution of ground run and air control, in m/s².
///
/// **Written every tick, even when it is zero** — then nobody has to clear it, there is no
/// clearing system and no state that lives one tick too long.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunAccel(pub Vec3);

/// Contribution of the gas boost along the look direction, in m/s² (`F-007`).
/// This is where `F-006` swerve and `F-008` dash dock on later — one system more, not one
/// type more.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoostAccel(pub Vec3);

/// Desired rope shortening per side, in m/s (`F-005`).
///
/// **No force.** Reel-in is a change of length; whoever turns it into a pull towards the
/// anchor gets the "linear pulling" that `F-004` explicitly rules out. The acceleration
/// falls out as a side effect of the rope constraint — exactly the centripetal motion
/// `F-004` asks for.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReelSpeed {
    pub m_s: [f32; 2],
}

/// The buttons of the **previous** tick, as a component on the player.
///
/// Edge detection (`Buttons::just_pressed`) needs a previous state. A `Local<Buttons>` would
/// be **wrong**: a `Local` belongs to the system, not to the entity — with two players both
/// share the same previous value, and in a snapshot it is invisible, so it survives no
/// rollback (`docs/multiplayer.md` rules 3 and 5).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PrevButtons(pub Buttons);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f001_both_sides_sit_at_fixed_indices() {
        // The order is part of the interface: `rope_step`, `RopeLength` and `Hook` all
        // index the same way.
        assert_eq!(Side::Left.index(), 0);
        assert_eq!(Side::Right.index(), 1);
    }

    #[test]
    fn f001_a_fresh_hook_is_anchored_to_nothing() {
        let h = Hook::default();
        assert_eq!(h.anchored_count(), 0);
        assert!(!h.arm(Side::Left).state.is_anchored());
    }

    #[test]
    fn f001_anchored_count_counts_only_anchored_arms() {
        let mut h = Hook::default();
        h.arms[Side::Left.index()].state =
            HookState::Anchored { body: BodyId(3), local_m: Vec3::Y };
        h.arms[Side::Right.index()].state =
            HookState::Flying { target_m: Vec3::ZERO, body: BodyId(4) };
        assert_eq!(h.anchored_count(), 1);
    }
}
