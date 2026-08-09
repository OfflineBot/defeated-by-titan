//! What a running sortie is made of: a clock in ticks, a counter per player, a wave list.
//!
//! ## Ticks, never a wall clock
//!
//! `missions.ron` speaks in seconds, the game counts ticks, and the conversion happens **once**,
//! at the boundary, in [`to_ticks`] — the same rule `titan::brain::ticks` and
//! `shared::HitStop` follow. Nothing in this domain ever reads `Time::delta_secs()`.
//!
//! That is not tidiness. A timeout that accumulates `delta_secs()` fires at a tick that depends
//! on the frame rate — so `assert phase == 4` after `wait 332` in a script passes on the
//! developer's machine, fails in a busy CI, and passes again when you look at it. Every
//! `--script` run in the repository becomes flaky, forever, and the cause is invisible in
//! review because the *code* looks right (`docs/PLAN-GAME.md` §8, `F-070`).
//!
//! ## The counter is a component with per-player numbers
//!
//! Not a `Resource<u32>`. There is one mission but many players, `F-096` (score) and `F-161a`
//! (contribution) want to know who cut which titan, and a `BTreeMap<PlayerId, u32>` costs
//! nothing today. See [`KillTally`].

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use crate::data::MissionTemplate;
use crate::shared::{PlayerId, TitanId};

/// Seconds out of the file into ticks. **Rounded, once**, so that 330 s at 60 Hz is 19 800 and
/// not 19 799 or 19 801 depending on where the multiplication happened.
///
/// The same function as `titan::brain::ticks`, deliberately copied and not borrowed: a domain
/// that reaches into another one for four lines of arithmetic is an edge past the allow list
/// (`docs/architecture.md`).
pub fn to_ticks(seconds: f32, simulation_hz: f64) -> u64 {
    let n = (seconds as f64 * simulation_hz).round();
    if n.is_finite() && n > 0.0 { n as u64 } else { 0 }
}

/// The running sortie. **Exactly one entity carries this**, and it comes into being in
/// `OnEnter(MissionPhase::Deploying)`.
///
/// An entity and not a resource, because everything hanging off it — the counter, the clock —
/// is per mission and one day per squad, and because a component can be queried by the three
/// domains that read it without any of them knowing this module's shape.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Mission {
    /// The key in `missions.ron` — what `--mission <name>` said.
    pub template: String,
    /// The display name out of the file (`"First Ride"`).
    pub name: String,
}

/// The mission clock. **In ticks, and only in ticks.**
///
/// `started_at_tick` is the tick the mission was deployed at (0 for a normal launch) and not a
/// timestamp: two machines that reach tick *n* have the same amount of mission left
/// (`docs/multiplayer.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionClock {
    pub started_at_tick: u64,
    /// `round(target_duration_s * simulation_hz)`, taken from `missions.ron`. **Never a
    /// literal in Rust** — change the file, and the deadline moves without a rebuild.
    pub duration_ticks: u64,
    /// The tick the verdict was decided on, for the log and for the report. `None` while the
    /// mission runs.
    ///
    /// It is written one tick **before** `State<MissionPhase>` reads the new phase: a
    /// `NextState` set inside `FixedUpdate` is applied by the `StateTransition` schedule of the
    /// next frame. That offset is real, it is exactly one tick at one fixed step per frame, and
    /// it is why the acceptance criterion of `F-070` says ±1.
    pub decided_at_tick: Option<u64>,
}

impl MissionClock {
    pub fn new(started_at_tick: u64, template: &MissionTemplate, simulation_hz: f64) -> Self {
        MissionClock {
            started_at_tick,
            duration_ticks: to_ticks(template.target_duration_s, simulation_hz),
            decided_at_tick: None,
        }
    }

    /// The tick the mission is lost on if nothing else has happened by then.
    pub fn deadline_tick(&self) -> u64 {
        self.started_at_tick.saturating_add(self.duration_ticks)
    }

    /// Ticks the mission has been running.
    pub fn elapsed(&self, tick: u64) -> u64 {
        tick.saturating_sub(self.started_at_tick)
    }

    /// Whether the clock has run out **at this tick**.
    pub fn expired(&self, tick: u64) -> bool {
        self.duration_ticks > 0 && tick >= self.deadline_tick()
    }
}

/// Who killed how many. **A component on the mission, with one number per player.**
///
/// `credited` is not bookkeeping for its own sake. A dissolving titan **keeps its `TitanId` for
/// `death_s`** (1.0 s for a husk, `titan::brain::dissolve`), so a second `TitanHit { Cortex }`
/// on a body that is already dying is a thing that can arrive — and a counter that took it
/// would report 4/3 kills off three titans. The set answers "has this titan already been paid
/// for", which is the only question that makes a kill a kill.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct KillTally {
    /// `kill_target` out of `missions.ron`. **The number the mission counts to lives in the
    /// file**, not here (§4).
    pub target: u32,
    by_player: BTreeMap<PlayerId, u32>,
    credited: BTreeSet<TitanId>,
}

impl KillTally {
    pub fn with_target(target: u32) -> Self {
        KillTally { target, ..default() }
    }

    /// Books one kill. Returns `false` when this titan was already paid for — **the second
    /// cortex hit on a dissolving body counts nothing.**
    pub fn credit(&mut self, player: PlayerId, titan: TitanId) -> bool {
        if !self.credited.insert(titan) {
            return false;
        }
        *self.by_player.entry(player).or_insert(0) += 1;
        true
    }

    /// What this player is credited with. `0` for a player who has not cut anything — that is
    /// a real answer, not a missing one.
    pub fn of(&self, player: PlayerId) -> u32 {
        self.by_player.get(&player).copied().unwrap_or(0)
    }

    /// Every kill of the squad. **This is what the objective counts** — the mission is won by
    /// the team, `F-096` is what splits the credit up.
    pub fn total(&self) -> u32 {
        self.by_player.values().sum()
    }

    /// Whether `kill_target` is reached.
    pub fn reached(&self) -> bool {
        self.target > 0 && self.total() >= self.target
    }

    /// Who is on the board, in a stable order. `BTreeMap`, so two machines list them the same.
    pub fn players(&self) -> impl Iterator<Item = (&PlayerId, &u32)> {
        self.by_player.iter()
    }
}

/// One wave of the template, resolved to a tick and a position.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingTitan {
    pub at_tick: u64,
    pub kind: String,
    pub pos: Vec3,
}

/// Everything still to come. **Carries `DespawnOnExit(MissionPhase::Active)`.**
///
/// That is the whole reason it is an entity of its own instead of a field on [`Mission`]: the
/// moment the mission is decided, the pending waves have to stop existing. Without it the
/// 210 s wave of the tutorial still walks into a mission that was won at 100 s, and a titan
/// spawning after `WON` looks like a bug in the spawner rather than what it is — a schedule
/// nobody switched off.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct WaveSchedule {
    /// Sorted by `at_tick`. Released from the front.
    pub pending: Vec<PendingTitan>,
    /// How many have been released so far — for the log and for the overlay.
    pub released: u32,
}

impl WaveSchedule {
    /// Turns the template's waves into single titans with a tick and a place.
    ///
    /// **The positions are derived, not read** — `missions.ron: waves` has no position field
    /// and this round does not get to invent one (`assets/data/` belongs to the main head).
    /// The ring radius is `maps.ron: layout.clear_radius_m`, i.e. exactly the circle the city
    /// generator keeps free, so no titan is spawned inside a house. It is deterministic and
    /// carries no rng: titan *i* of *n* stands at `i/n` of the circle.
    ///
    /// ⚠️ This is a stand-in for a real spawn rule (`F-072`, not built). It is written down
    /// here and reported, so that nobody mistakes it for a design decision.
    pub fn of(template: &MissionTemplate, started_at_tick: u64, simulation_hz: f64, ring_m: f32) -> Self {
        let total: u32 = template.waves.iter().map(|w| w.count).sum();
        let mut pending = Vec::new();
        for wave in &template.waves {
            let at_tick = started_at_tick.saturating_add(to_ticks(wave.at_s, simulation_hz));
            for _ in 0..wave.count {
                let index = pending.len() as u32;
                pending.push(PendingTitan {
                    at_tick,
                    kind: wave.kind.clone(),
                    pos: ring_position(index, total, ring_m),
                });
            }
        }
        // Stable by tick, and within a tick in file order — `sort_by_key` keeps the order of
        // equal keys, so the same file gives the same list on every machine.
        pending.sort_by_key(|p| p.at_tick);
        WaveSchedule { pending, released: 0 }
    }

    /// Takes everything due at this tick off the front.
    pub fn take_due(&mut self, tick: u64) -> Vec<PendingTitan> {
        let cut = self.pending.iter().take_while(|p| p.at_tick <= tick).count();
        let due: Vec<PendingTitan> = self.pending.drain(..cut).collect();
        self.released += due.len() as u32;
        due
    }
}

/// Titan *i* of *n* on a circle of radius `ring_m` around the origin, on the ground.
///
/// Pure and without rng on purpose: a spawn position out of `rand::random()` is a desync, and
/// one out of `(seed, tick)` is a number nobody can reproduce from the file alone.
pub fn ring_position(index: u32, total: u32, ring_m: f32) -> Vec3 {
    let n = total.max(1) as f32;
    let angle = std::f32::consts::TAU * (index as f32) / n;
    Vec3::new(ring_m * angle.cos(), 0.0, ring_m * angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_become_ticks_exactly_once() {
        // 330 s at 60 Hz is the number the whole F-070 criterion hangs on.
        assert_eq!(to_ticks(330.0, 60.0), 19_800);
        assert_eq!(to_ticks(10.0, 60.0), 600);
        // Rounded, not truncated: 0.605 s would be 36.3 ticks.
        assert_eq!(to_ticks(0.605, 60.0), 36);
        // Nonsense does not become a silent zero-length mission that ends at tick 0.
        assert_eq!(to_ticks(-1.0, 60.0), 0);
        assert_eq!(to_ticks(f32::NAN, 60.0), 0);
    }

    #[test]
    fn the_deadline_is_the_start_plus_the_duration() {
        let clock = MissionClock { started_at_tick: 0, duration_ticks: 19_800, decided_at_tick: None };
        assert_eq!(clock.deadline_tick(), 19_800);
        assert!(!clock.expired(19_799));
        assert!(clock.expired(19_800), "the deadline tick is the one it fires on");
        assert_eq!(clock.elapsed(1_234), 1_234);
    }

    #[test]
    fn a_zero_length_mission_never_expires() {
        // A template with a missing duration would otherwise lose at tick 0, which reads as a
        // bug in the state machine rather than as a hole in the file.
        let clock = MissionClock { started_at_tick: 0, duration_ticks: 0, decided_at_tick: None };
        assert!(!clock.expired(10_000));
    }

    #[test]
    fn the_second_cortex_hit_on_the_same_titan_counts_nothing() {
        // A dissolving titan keeps its TitanId for death_s — measured this session.
        let mut tally = KillTally::with_target(3);
        assert!(tally.credit(PlayerId(1), TitanId(7)));
        assert!(!tally.credit(PlayerId(1), TitanId(7)), "the same titan twice is one kill");
        assert!(!tally.credit(PlayerId(2), TitanId(7)), "and not one per player either");
        assert_eq!(tally.total(), 1);
        assert_eq!(tally.of(PlayerId(1)), 1);
        assert_eq!(tally.of(PlayerId(2)), 0);
    }

    #[test]
    fn the_counter_keeps_the_credit_apart_and_the_total_together() {
        let mut tally = KillTally::with_target(3);
        tally.credit(PlayerId(1), TitanId(1));
        tally.credit(PlayerId(2), TitanId(2));
        tally.credit(PlayerId(1), TitanId(3));
        assert_eq!(tally.of(PlayerId(1)), 2);
        assert_eq!(tally.of(PlayerId(2)), 1);
        assert_eq!(tally.total(), 3);
        assert!(tally.reached(), "the squad reaches the target together");
    }

    #[test]
    fn a_target_of_zero_is_not_won_at_once() {
        // Otherwise a template without `kill_target` would be won before it started.
        let tally = KillTally::with_target(0);
        assert!(!tally.reached());
    }

    #[test]
    fn waves_are_released_at_their_tick_and_never_twice() {
        let mut schedule = WaveSchedule {
            pending: vec![
                PendingTitan { at_tick: 0, kind: "husk".into(), pos: Vec3::ZERO },
                PendingTitan { at_tick: 5_400, kind: "husk".into(), pos: Vec3::ZERO },
                PendingTitan { at_tick: 5_400, kind: "husk".into(), pos: Vec3::ZERO },
            ],
            released: 0,
        };
        assert_eq!(schedule.take_due(0).len(), 1);
        assert!(schedule.take_due(1).is_empty(), "nothing is released a second time");
        assert_eq!(schedule.take_due(9_000).len(), 2, "everything overdue comes at once");
        assert_eq!(schedule.released, 3);
        assert!(schedule.pending.is_empty());
    }

    #[test]
    fn the_spawn_ring_is_deterministic_and_keeps_its_radius() {
        // Same index, same place — on every machine, in every rollback.
        let a = ring_position(1, 4, 24.0);
        let b = ring_position(1, 4, 24.0);
        assert_eq!(a, b);
        for i in 0..4 {
            let p = ring_position(i, 4, 24.0);
            assert!((p.length() - 24.0).abs() < 1e-3, "titan {i} stands at {p:?}");
            assert_eq!(p.y, 0.0, "titans spawn on the ground, not in the air");
        }
        assert_ne!(ring_position(0, 4, 24.0), ring_position(1, 4, 24.0));
    }
}
