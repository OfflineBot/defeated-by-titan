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
    /// `F-009` flip — **true on exactly one tick per double-tap of `A` or `D`**, like
    /// [`dodge`](Self::dodge) and for the same reason: `net::local::read_input` presses
    /// `Buttons::FLIP` on the tick the second press lands and on no other.
    ///
    /// The flip is billed like the dodge — flat, once, `vector.gas_flip` — and it is a
    /// **fifth** consumer rather than a second meaning for `dodge`, because the two differ in
    /// price, in direction and in what they buy: a dodge is 24 m/s where WASD points for 45
    /// gas, a flip is 18 m/s strictly sideways plus i-frames for 20. One field with two prices
    /// is the shape a ledger cannot audit.
    pub flip: bool,
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

/// `F-019` — **a refuel point out in the field**, and the thing `Q-044` said was missing.
///
/// ## Why it lives in `shared/` and not in `world/`
///
/// Two domains have to see it and neither may see the other: `world` spawns it out of
/// `maps.ron` and runs the pump, and `render` has to paint an empty one differently
/// (*„leere Station wird visuell markiert"*). `docs/architecture.md` already names exactly
/// this case — *"`world` spawns entities with components out of `shared/`, and `render`
/// queries those components without knowing this domain"* — so the component is the seam and
/// no allow-list line is bought.
///
/// ## The one rule that keeps it from being an exploit
///
/// **A use is spent when the refill STARTS, not when it finishes.** The obvious alternative —
/// pay out gradually, charge the use at the end — lets a player tap in and out of the circle
/// forever and drink an unbounded amount for nothing. The alternative to *that* — pay out
/// nothing until 1.5 s have elapsed — makes a station that is interrupted worth exactly zero,
/// which is a rule a player cannot see happening to him.
///
/// So: entering an idle station with `uses_left > 0` costs the use immediately and starts the
/// pump. The pump then runs its `refill_s` **whether he stays or not** — a running pump keeps
/// pumping — and everyone standing in the circle drinks from it, which is what makes one
/// station a squad's decision rather than a queue (`docs/multiplayer.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SupplyStation {
    /// How far the circle reaches, in meters. Measured in **3D** from the centre: a player
    /// 40 m above a station is not standing at it.
    pub radius_m: f32,
    /// Reloads still in it. `0` is an empty station, and an empty station is still there —
    /// it is marked, not despawned, or a player learns nothing from having drained it.
    pub uses_left: u32,
    /// Seconds left on the refill that is running. `0.0` means idle.
    pub charge_s: f32,
    /// What one reload takes, out of `gear.ron: resupply.station_refill_s`. Copied onto the
    /// component at spawn, like `mission::hub::RefuelStation` does with its rate, so the pump
    /// is a function of what stands in the world and needs no `GameData` per player per tick.
    pub refill_s: f32,
    /// Gas per second while the pump runs. One reload is a **whole tank**, so this is
    /// `vector.gas_tank / station_refill_s` — 10000/s at today's numbers, which sounds absurd
    /// and is exactly right: it runs for 1.5 s and `Gas::refill` caps at the tank.
    pub gas_per_s: f32,
    /// **One reload per visit.** True from the tick a pump starts until the circle is empty
    /// again.
    ///
    /// Without it a player who merely stands on a station drains every reload in it back to
    /// back — measured, `3 -> 0` in 4.5 s with nobody pressing anything, which is not *„begrenzte
    /// Nachladungen"* but a station that empties itself. With it, three uses are three
    /// **visits**: you take your tank, you leave, and coming back is a decision you make again.
    pub served_this_visit: bool,
}

impl SupplyStation {
    pub fn running(&self) -> bool {
        self.charge_s > 0.0
    }

    pub fn empty(&self) -> bool {
        self.uses_left == 0 && !self.running()
    }
}

/// `F-008` — **what actually bounds a dash**, and it exists because the gas price stopped
/// bounding it.
///
/// Measured on 2026-08-20 (`docs/QUESTIONS.md` Q-046, `docs/FINDINGS.md` FIND-152): the
/// testability tank went `300 -> 15000`, so `vector.gas_dodge: 45` went from **6.7 dashes per
/// sortie** to **333**. The backlog row says *"mit eigenem Cooldown ... Anzahl der Dashes ist
/// ein Stat"* and neither half existed — a dash was a traversal move you could hold down.
///
/// Two limits and they answer two different questions:
///
/// - [`left`](Self::left) is *how many in a row* — the stat. It refills one charge every
///   `vector.dodge_recharge_s`, and only up to [`max`](Self::max).
/// - [`spent_at_tick`](Self::spent_at_tick) is *how fast* — `vector.dodge_cooldown_s` between
///   two dashes, so a full magazine cannot be emptied in three ticks.
///
/// **The charge is what the gas grant asks, and `vector::dodge` is the one writer.**
/// `vector::gas::gas_budget` reads it and refuses to bill a dash it will not get
/// (`FIND-152`'s shape in reverse: the gate has to be in front of the money, not behind it).
///
/// `left` is a float and not a `u8` on purpose: the recharge accumulator would otherwise need
/// a second field, and a second field is a second thing that can disagree with the first.
/// Only `left >= 1.0` is a dash.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DodgeCharges {
    /// Whole and fractional charges. A dash costs exactly `1.0`.
    pub left: f32,
    /// The ceiling, out of `game.ron: vector.dodge_charges`. Never a number in Rust.
    pub max: f32,
    /// The tick the last dash was granted on. `None` means "never dashed", which is the only
    /// state in which the cooldown cannot refuse.
    pub spent_at_tick: Option<u64>,
}

impl DodgeCharges {
    pub fn new(max: f32) -> Self {
        Self { left: max, max, spent_at_tick: None }
    }

    /// Whether a dash may fire **this** tick: a whole charge in the magazine and the cooldown
    /// elapsed since the last one.
    ///
    /// `saturating_sub` for the same reason `net::local::DodgeTap::feed` uses it — a tick
    /// counter that ever ran backwards yields `0`, i.e. "not yet", instead of an underflow
    /// panic in the middle of the simulation.
    pub fn ready(&self, tick: u64, cooldown_ticks: u64) -> bool {
        self.left >= 1.0
            && self.spent_at_tick.is_none_or(|t| tick.saturating_sub(t) >= cooldown_ticks)
    }
}

/// `F-009` / `F-010` — **the only invulnerability a player has**, and it is a deadline, not a
/// flag.
///
/// A tick and not a countdown: a countdown needs somebody to tick it down every frame and it
/// is wrong the moment two systems do. A deadline is written once by the move that grants it
/// and read by `combat::strike::land`, which already knows the tick.
///
/// ⚠️ **It is never removed.** An expired deadline is simply in the past, so there is no
/// despawn system, no `Option`, and no window in which the component exists but means nothing.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Invulnerable {
    /// The first tick on which blows land again.
    pub until_tick: u64,
}

impl Invulnerable {
    /// Extends, never shortens. Two moves whose windows overlap give the longer of the two —
    /// a flip out of a slide must not cut the slide's i-frames short.
    pub fn extend_to(&mut self, tick: u64) {
        self.until_tick = self.until_tick.max(tick);
    }

    pub fn active(&self, tick: u64) -> bool {
        tick < self.until_tick
    }
}

/// `F-010` — the ground slide: **a deadline and a direction**, and nothing else.
///
/// The same shape as [`Invulnerable`] and for the same reason. `dir_m` is a horizontal unit
/// vector in world space, fixed at the tick the slide starts: a slide you can steer is a run,
/// and *„geht fliessend in Sprint ueber"* is about what happens when it **ends**, not during.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    /// The first tick on which the player runs again. `0` means "not sliding" and is the
    /// value a fresh player carries.
    pub until_tick: u64,
    /// World-space horizontal unit vector. Meaningless while `until_tick` is in the past.
    pub dir_m: Vec3,
    /// The tick the last slide **started** — the cooldown clock, so that a slide cannot be
    /// re-entered on the tick it ends.
    pub started_at_tick: Option<u64>,
}

impl Slide {
    pub fn active(&self, tick: u64) -> bool {
        tick < self.until_tick
    }

    /// Whether a new slide may start: not already sliding, and `slide_cooldown_ticks` since
    /// the last one began.
    pub fn ready(&self, tick: u64, cooldown_ticks: u64) -> bool {
        !self.active(tick)
            && self.started_at_tick.is_none_or(|t| tick.saturating_sub(t) >= cooldown_ticks)
    }
}

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
