//! State on a body — **components, never a `Resource`.**
//!
//! Gas, blades and movement state hang on *one* player. As a `Resource` they would be
//! global, and with that the game would be a single-player game you only notice as one when
//! multiplayer comes around (`prompts/init.md` §6 rule 3). The same sentence holds for
//! [`Health`], [`HitStop`] and [`TitanState`]: twenty players and sixty titans, each with
//! their own number, and not one of them global.
//!
//! They live in `shared/` although `vector` and `blades` write them, because `hud` and
//! `sound` have to **read** them — and opening an edge between domains for that would be the
//! beginning of the end of the domain rule. **Who writes stands in the authority table in
//! `docs/architecture.md`, not in the type.**

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Gas is finite, and **spending gas is loud** — the Bellower reacts to the noise (bible 4).
/// That couples the resource to the risk instead of making it a plain timer.
///
/// ## It does not come back on its own (`docs/QUESTIONS.md` Q-033)
///
/// The user decided it on 2026-08-12: *"gas refillt nur im main gebäude an bestimmten
/// stationen/objekten"*. Refuelling is **a place you go to**, not a rate. So the simulation
/// has exactly one writer, `vector::gas`: it *lowers* this number in `gas_budget` and *raises*
/// it in `apply_refuel_requests`, and nothing else touches it. A refuel station belongs to
/// `mission` and therefore **asks** — [`RefuelRequest`](super::message::RefuelRequest), applied
/// one tick later (`docs/FINDINGS.md` FIND-063). A field like `regen_delay_left_s` used to hang here
/// for a timer-shaped regeneration; it is gone on purpose, and
/// `tests/vector_gas.rs::f018_an_idle_tank_never_refills_on_its_own` keeps it gone.
///
/// The numbers (tank size, drain per second, boost cost) stand in `assets/data/gear.ron`,
/// **not here** (§4).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gas {
    pub current: f32,
    pub max: f32,
    /// `--sandbox` sets this: infinite gas, for looking around (§12a).
    pub unlimited: bool,
}

impl Gas {
    pub fn full(max: f32) -> Self {
        Gas { current: max, max, unlimited: false }
    }

    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 { (self.current / self.max).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Tries to spend `amount`. `false` means: there was not enough, **and nothing was
    /// deducted**.
    ///
    /// No partial spend: "gas exactly zero at the moment of the boost" is one of the edge
    /// cases that belong in a test (§8) — half a boost would be harder to explain than none.
    pub fn try_spend(&mut self, amount: f32) -> bool {
        if self.unlimited {
            return true;
        }
        if !amount.is_finite() || amount < 0.0 {
            return false;
        }
        if self.current + 1e-6 < amount {
            return false;
        }
        self.current = (self.current - amount).max(0.0);
        true
    }

    /// Puts `amount` back, **capped at [`max`](Self::max)**.
    ///
    /// ⚠️ **Exactly one system in the simulation calls this** (Q-033):
    /// `vector::gas::apply_refuel_requests`, on a `RefuelRequest` that a refuel station of the
    /// main building sent. Whoever wires it to a timer, an idle branch or a tick is undoing the
    /// user's decision — and whoever calls it from another domain has made `Gas` a field with
    /// two writers, which is what 2026-08-12 cost a repair (see the type's doc).
    pub fn refill(&mut self, amount: f32) {
        if amount.is_finite() && amount > 0.0 {
            self.current = (self.current + amount).min(self.max);
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.unlimited && self.current <= 0.0
    }
}

/// Blades go blunt and break. **Economy instead of cooldowns** (`prompts/init.md` §1):
/// you reload at supply points, from the horse, or off fallen comrades.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blades {
    /// How many pairs are left in the harness.
    pub pairs_left: u8,
    /// Condition of the pair in use, 1.0 = fresh, 0.0 = broken.
    pub sharpness: f32,
}

impl Blades {
    pub fn fresh(pairs: u8) -> Self {
        Blades { pairs_left: pairs, sharpness: 1.0 }
    }

    pub fn is_broken(&self) -> bool {
        self.sharpness <= 0.0
    }

    /// Put in a fresh pair. `false` means: none left.
    pub fn swap_pair(&mut self) -> bool {
        if self.pairs_left == 0 {
            return false;
        }
        self.pairs_left -= 1;
        self.sharpness = 1.0;
        true
    }
}

/// Velocity in m/s.
///
/// A component of its own and not a value derived from the `Transform`: **damage comes out
/// of speed** (`prompts/init.md` §1), and a quantity that damage comes out of must not
/// depend on how much time passed between two frames. It is also exactly the number
/// `assert speed > 25` measures (§12b).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Velocity(pub Vec3);

impl Velocity {
    pub fn speed_m_s(&self) -> f32 {
        self.0.length()
    }
}

/// What the player's body currently hangs on.
///
/// **It decides who is allowed to write the `Transform`.** `player` writes it on the ground
/// and in free fall, `vector` on the rope — never both at once. Two writers on the same
/// field are not a design, they are a coin flip at 60 Hz (§5 rule 4).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementState {
    #[default]
    Grounded,
    Airborne,
    /// At least one hook holds — from here on the `Transform` belongs to `vector`.
    Tethered,
    OnWall,
    /// Out of the fight instead of dead: a state with a timer, not a removed entity.
    /// Revived by team mates (bible 3.6, `squad/`).
    Downed,
}

/// What a body can take before it is out of the fight. Players **and** titans.
///
/// ⚠️ **This type has no `F-ID` anywhere in the backlog.** It is not a forgotten feature row,
/// it is a hole: `MovementState::Downed` above already describes what happens when this
/// number reaches zero, `titan.ron` is getting a `health` field per kind, and the damage
/// curve is supposed to come out of speed — and none of those three has a component to write
/// to. It is a **seam type**, so do not go looking for the row that owns it, and do not set a
/// stage against it.
///
/// **The numbers stand in RON** (`titan.ron: health`, the player's in `game.ron`), never
/// here (§4). What stands here is the arithmetic: it saturates at 0 and it never exceeds
/// `max`. A negative health is not a smaller number, it is a second death condition nobody
/// tests for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn full(max: f32) -> Self {
        Health { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 { (self.current / self.max).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Takes `amount` off. **Saturates at 0** and returns what is left.
    ///
    /// A nonsensical amount (negative, `NaN`) changes nothing — a negative hit would be a
    /// heal, and that is the kind of thing a damage formula produces at 3 a.m.
    pub fn damage(&mut self, amount: f32) -> f32 {
        if amount.is_finite() && amount > 0.0 {
            self.current = (self.current - amount).max(0.0);
        }
        self.current
    }

    /// Heals by `amount`, **capped at [`max`](Self::max)**. Does not revive on its own: who is
    /// allowed to leave `MovementState::Downed` is `squad`'s decision, not this type's.
    pub fn heal(&mut self, amount: f32) {
        if amount.is_finite() && amount > 0.0 {
            self.current = (self.current + amount).min(self.max);
        }
    }

    /// Zero — "out of the fight", not "delete the entity" (see [`MovementState::Downed`]).
    pub fn is_empty(&self) -> bool {
        self.current <= 0.0
    }
}

/// The impact freeze after a hit, counted in **simulation ticks** (`F-034`).
///
/// **A tick counter, never a clock — and that is the whole type.** The obvious
/// implementation is one line, `Time<Virtual>::set_relative_speed(0.05)`, it looks perfect on
/// screen, and it is wrong for a reason that no screenshot shows:
/// `run_fixed_main_schedule` accumulates `Time<Fixed>`'s overstep out of
/// `Time<Virtual>::delta()` (`bevy_time-0.19.0/src/fixed.rs:243-247`), so slowing virtual
/// time slows **the tick rate itself**. And the tick is not a display quantity here:
/// [`Tick`](super::schedule::Tick) is what [`Rng`](super::rng::Rng) seeds from and what every
/// [`Intent`](super::intent::Intent) is stamped with (`docs/multiplayer.md` rules 2 and 5).
/// Freezing it means the random numbers stall and the input stamps drift — per client, which
/// over a wire is a divergence nobody can reproduce.
///
/// So: the clock keeps running, the tick keeps counting, and a body carrying a `HitStop` with
/// `ticks_left > 0` simply does not advance this tick. The number of ticks comes from RON
/// (`gear.ron: feel.hit_stop_cortex_s` × 60), not from here.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitStop {
    pub ticks_left: u32,
}

impl HitStop {
    pub fn new(ticks: u32) -> Self {
        HitStop { ticks_left: ticks }
    }

    /// Is this body frozen **this** tick?
    pub fn is_frozen(&self) -> bool {
        self.ticks_left > 0
    }

    /// One tick down. **Stops at 0** instead of wrapping — a `u32` that wraps once is a body
    /// frozen for 4.29 billion ticks, and it happens the first time two hits land on the same
    /// entity in the same tick.
    pub fn tick(&mut self) -> u32 {
        self.ticks_left = self.ticks_left.saturating_sub(1);
        self.ticks_left
    }
}

/// What a titan is doing. The FSM of `F-050`, as a component.
///
/// **It lives in `shared/` and not in `titan/` on purpose.** `titan` writes it, `combat`
/// gates on it, and the F3 overlay in `debug` prints it (`husk#1 Windup 21/36`). A
/// `titan`-private enum would force an entry in the allow list of `docs/architecture.md`
/// purely so that a debug overlay can print one word — and an allow list that grows for
/// reasons like that stops being a rule.
///
/// ⚠️ **Two arms are missing on purpose, do not "complete" the enum.** `Alerted` belongs to
/// `F-051` and `Stagger` to `F-032`, neither of which is being built this session. Adding a
/// variant with nothing that enters or leaves it produces an FSM that is decoration — a state
/// that is set correctly while nothing gates on it, which is exactly what `F-050`'s
/// tick-count test exists to catch.
///
/// **How long each state lasts stands in `titan.ron`** (`windup_s`, `strike_s`, `recover_s`,
/// `death_s`), in ticks, never in `Time::delta_secs()` — the pose is a pure function of
/// `(TitanState, ticks_in_state)` so that an `--offscreen` run is bit-identical.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TitanState {
    /// Standing around. Nobody in range, nothing to do.
    #[default]
    Idle,
    /// Walking towards a target.
    Pursue,
    /// The telegraph (`F-053`): the attack is committed and readable, and has not landed yet.
    Windup,
    /// The blow itself.
    Strike,
    /// **The punish window.** The reason an attack is worth baiting.
    Recover,
    /// Cortex cut. Dissolving over `death_s`, collider already gone.
    Death,
}

/// **How far through its current [`TitanState`] a body is** — in simulation ticks, as
/// `ticks_in_state` out of `state_ticks`.
///
/// ## Why the number lives here and not next to the state machine
///
/// The same argument as [`TitanState`]'s, one step further. `husk#1 Windup` in the F3 overlay
/// is equally true on tick 1 and on tick 35, so it cannot show whether the wind-up really
/// lasts as long as `titan.ron` says — and that is the *only* thing `F-050`'s picture
/// criterion asks a screenshot to prove (`docs/PLAN-GAME.md` §8: the overlay reads
/// `husk#1 Windup 21/36` while the arm is visibly up in the same frame). With the counter
/// private to `titan/`, `debug` would need a line in the allow list of `docs/architecture.md`
/// purely so that an overlay can print a number, and an allow list that grows for reasons like
/// that stops being a rule.
///
/// ## Both numbers, or the picture proves nothing
///
/// `ticks_in_state` alone is a number without a scale, and a `state_ticks` typed next to the
/// place that prints it is a constant that keeps reading `n/36` after somebody has changed
/// `windup_s`. So both are written **together, by the one system that owns the state edge**
/// (`titan::brain::advance` — the authority table in `docs/architecture.md`, not this type),
/// out of the timings resolved from `titan.ron` at spawn.
///
/// **`state_ticks == 0` means open-ended.** `Idle` and `Pursue` last as long as the world makes
/// them last; a fraction over a total of zero is not a reading, so whoever prints this leaves
/// it off rather than showing `17/0`.
///
/// It is a tick counter and never a clock, for the reason [`HitStop`] spells out: the pose is a
/// pure function of `(TitanState, ticks_in_state)`, which is what makes an `--offscreen` run
/// bit-identical.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateClock {
    /// Ticks already completed in the current state. **The entry tick of a state is 0.**
    pub ticks_in_state: u32,
    /// How long the current state lasts in total. **0 = open-ended**, see the type's doc.
    pub state_ticks: u32,
}

impl StateClock {
    /// A state that begins **now** and runs for `state_ticks` (0 = open-ended).
    pub fn entering(state_ticks: u32) -> Self {
        StateClock { ticks_in_state: 0, state_ticks }
    }

    /// Does this state have a length that can be printed as a fraction at all?
    pub fn is_timed(&self) -> bool {
        self.state_ticks > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_only_spends_what_is_there() {
        let mut g = Gas::full(100.0);
        assert!(g.try_spend(30.0));
        assert!((g.current - 70.0).abs() < 1e-6);
        assert!(!g.try_spend(80.0), "80 out of 70 must not succeed");
        assert!((g.current - 70.0).abs() < 1e-6, "a failed boost costs nothing");
    }

    #[test]
    fn gas_exactly_zero_at_the_moment_of_the_boost() {
        // Exactly the edge case from prompts/init.md §8 — the normal case works almost by
        // itself, the bugs sit at the edges.
        let mut g = Gas::full(10.0);
        assert!(g.try_spend(10.0));
        assert!(g.is_empty());
        assert!(!g.try_spend(0.001));
        assert_eq!(g.fraction(), 0.0);
    }

    #[test]
    fn gas_rejects_nonsensical_amounts() {
        let mut g = Gas::full(10.0);
        assert!(!g.try_spend(-5.0), "a negative spend would be a refill");
        assert!(!g.try_spend(f32::NAN));
        assert!((g.current - 10.0).abs() < 1e-6);
    }

    #[test]
    fn sandbox_gas_never_runs_out() {
        let mut g = Gas { unlimited: true, ..Gas::full(1.0) };
        assert!(g.try_spend(1000.0));
        assert!(!g.is_empty());
    }

    #[test]
    fn gas_does_not_overflow_on_refill() {
        let mut g = Gas::full(50.0);
        g.try_spend(10.0);
        g.refill(999.0);
        assert!((g.current - 50.0).abs() < 1e-6);
    }

    #[test]
    fn blades_swap_until_the_belt_is_empty() {
        let mut k = Blades::fresh(2);
        k.sharpness = 0.0;
        assert!(k.is_broken());
        assert!(k.swap_pair());
        assert!(!k.is_broken());
        assert!(k.swap_pair());
        assert!(!k.swap_pair(), "an empty harness yields no more pairs");
        assert_eq!(k.pairs_left, 0);
    }

    #[test]
    fn health_saturates_at_zero_and_never_goes_negative() {
        let mut h = Health::full(100.0);
        assert!((h.damage(30.0) - 70.0).abs() < 1e-6);
        // Overkill is the normal case, not the edge case: damage comes out of speed.
        assert_eq!(h.damage(9999.0), 0.0);
        assert!(h.current >= 0.0, "health went negative — that is a second death condition");
        assert!(h.is_empty());
        assert_eq!(h.damage(1.0), 0.0, "a corpse does not get more negative");
        assert_eq!(h.fraction(), 0.0);
    }

    #[test]
    fn health_heals_no_further_than_max() {
        let mut h = Health::full(50.0);
        h.damage(40.0);
        h.heal(5.0);
        assert!((h.current - 15.0).abs() < 1e-6);
        h.heal(999.0);
        assert!((h.current - 50.0).abs() < 1e-6, "healing past max invents hit points");
        assert!((h.fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn health_rejects_nonsensical_amounts() {
        let mut h = Health::full(10.0);
        h.damage(-5.0);
        assert!((h.current - 10.0).abs() < 1e-6, "negative damage would be a heal");
        h.damage(f32::NAN);
        assert!((h.current - 10.0).abs() < 1e-6, "NaN damage must not poison the number");
        h.damage(4.0);
        h.heal(f32::NAN);
        assert!((h.current - 6.0).abs() < 1e-6);
    }

    #[test]
    fn hit_stop_counts_down_to_zero_and_stops_there() {
        let mut s = HitStop::new(3);
        assert!(s.is_frozen());
        assert_eq!(s.tick(), 2);
        assert_eq!(s.tick(), 1);
        assert_eq!(s.tick(), 0);
        assert!(!s.is_frozen(), "0 ticks left means the body moves again");
        // The one that matters: a wrapping u32 freezes the body for 4.29 billion ticks.
        assert_eq!(s.tick(), 0);
        assert_eq!(s.ticks_left, 0);
        assert!(!s.is_frozen());
    }

    #[test]
    fn hit_stop_survives_a_snapshot() {
        // It hangs on a body, so it has to go into a save and one day down a wire
        // (docs/multiplayer.md rule 8). `Copy` so that reading it costs nothing.
        let s = HitStop::new(7);
        let copied = s;
        assert_eq!(copied.ticks_left, 7);
        assert_eq!(s.ticks_left, 7, "HitStop must be Copy, not moved");

        let text = ron::to_string(&s).expect("HitStop must serialize");
        let back: HitStop = ron::de::from_str(&text).expect("HitStop must deserialize");
        assert_eq!(back, s);
    }

    #[test]
    fn titan_state_starts_idle() {
        // `Default` is what a freshly spawned titan gets before its FSM has run once.
        assert_eq!(TitanState::default(), TitanState::Idle);
    }

    #[test]
    fn a_state_clock_starts_at_zero_and_knows_when_it_has_no_length() {
        let fresh = StateClock::default();
        assert_eq!(fresh.ticks_in_state, 0, "the entry tick of a state is 0");
        assert!(!fresh.is_timed(), "a state with no length must not be printed as `n/0`");
        // `Idle` and `Pursue` end when the world ends them, not when a counter runs out.
        assert!(!StateClock::entering(0).is_timed());
        assert!(StateClock::entering(36).is_timed());
        assert_eq!(StateClock::entering(36).ticks_in_state, 0);
    }

    #[test]
    fn a_state_clock_survives_a_snapshot() {
        // It hangs on a body, so it goes into a save and one day down a wire
        // (`docs/multiplayer.md` rule 8) — and the pair has to arrive together, or the
        // receiving side prints a fraction out of two different ticks.
        let c = StateClock { ticks_in_state: 21, state_ticks: 36 };
        let copied = c;
        assert_eq!(copied, c, "StateClock must be Copy, not moved");

        let text = ron::to_string(&c).expect("StateClock must serialize");
        let back: StateClock = ron::de::from_str(&text).expect("StateClock must deserialize");
        assert_eq!(back, c);
    }
}
