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

use crate::data::{MissionTemplate, Objective, Wave};
use crate::shared::{PlayerId, TitanId};

use super::phase::MissionPhase;

/// The three numbers **one** sortie flies, after template and difficulty have been put
/// together — and the one place that decides which of the two wins.
///
/// `difficulty: None` is the direct drop-in (`--mission <name>`, what `F-070` and every script
/// in the repository uses) and takes the template's own numbers. `Some(key)` is a sortie
/// deployed out of the hub and takes the level's. **The fork lives here and nowhere else**: a
/// second `if difficulty.is_some()` in `deploy` and a third in `open_the_field` is how a
/// mission ends up counting to the recruit target against the elite clock.
#[derive(Clone, Debug, PartialEq)]
pub struct SortieNumbers<'a> {
    /// What the log and the HUD call it — `"Ashgate Skirmish · Recruit"`.
    pub name: String,
    pub target_duration_s: f32,
    pub kill_target: u32,
    pub waves: &'a [Wave],
    /// ⭐ **The mode, and it is the template's in both branches.**
    ///
    /// A difficulty owns three numbers — deadline, kill target, waves — and deliberately not
    /// this one (`data::MissionTemplate::objective`). A level that could turn a breach into a
    /// cull would make "which mission am I flying" a question with two answers, and the lobby
    /// shows the mission row and the difficulty row side by side.
    pub objective: &'a Objective,
}

/// Template ⊕ difficulty. `None` means the level is not in the file — and the caller says so
/// loudly instead of flying a mission with somebody else's numbers.
pub fn resolve<'a>(
    template: &'a MissionTemplate,
    difficulty: Option<&str>,
) -> Option<SortieNumbers<'a>> {
    let Some(key) = difficulty else {
        return Some(SortieNumbers {
            name: template.name.clone(),
            target_duration_s: template.target_duration_s,
            kill_target: template.kill_target,
            waves: &template.waves,
            objective: &template.objective,
        });
    };
    let level = template.difficulties.get(key)?;
    Some(SortieNumbers {
        name: format!("{} · {}", template.name, level.name),
        target_duration_s: level.target_duration_s,
        kill_target: level.kill_target,
        waves: &level.waves,
        // The template's, never the level's — see `SortieNumbers::objective`.
        objective: &template.objective,
    })
}

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

/// **What the sortie was decided as**, on the mission entity itself.
///
/// The verdict is a phase — `Won` or `Lost` — and that was enough for as long as the verdict
/// *was* the end of the run. It is not any more: [`MissionPhase::Debrief`] comes after both of
/// them, and a screen that is up during the debrief can read the phase all it likes and will
/// only ever be told `DEBRIEF`. So the answer is written down where the rest of the sortie's
/// numbers already live, next to the clock and the counter, by the one system that speaks it
/// (`mission::announce`).
///
/// **The word is not this component's.** [`Verdict::label`] hands back
/// [`MissionPhase::label`]'s, so the debrief plate and the HUD's big line cannot come to say
/// two different things — the same rule `hud::objective` states for itself.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub won: bool,
}

impl Verdict {
    /// `WON` or `LOST`, out of the phase enum and never out of a string here.
    pub fn label(self) -> &'static str {
        if self.won { MissionPhase::Won.label() } else { MissionPhase::Lost.label() }
    }
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
    /// The deadline comes out of the **resolved** numbers, not out of the template: a recruit
    /// sortie has seven minutes and an elite one five, and both are the same template.
    pub fn new(started_at_tick: u64, target_duration_s: f32, simulation_hz: f64) -> Self {
        MissionClock {
            started_at_tick,
            duration_ticks: to_ticks(target_duration_s, simulation_hz),
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
    /// ⚠️ This is a stand-in for a real spawn rule. It is written down here and reported, so
    /// that nobody mistakes it for a design decision.
    ///
    /// The waves come out of the **resolved** numbers ([`resolve`]) and not out of the
    /// template: which kinds come, when and how many is the third thing a difficulty changes.
    pub fn of(waves: &[Wave], started_at_tick: u64, simulation_hz: f64, ring_m: f32) -> Self {
        let total: u32 = waves.iter().map(|w| w.count).sum();
        let mut pending = Vec::new();
        for wave in waves {
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
    use crate::data::Difficulty;

    /// A template with one level in it, built here and not read from the file: what is under
    /// test is the **fork**, and a test that reads `missions.ron` would go red the day somebody
    /// balances it.
    fn two_shapes() -> MissionTemplate {
        MissionTemplate {
            name: "Skirmish".into(),
            map: "ashgate".into(),
            target_duration_s: 330.0,
            kill_target: 3,
            waves: vec![Wave { at_s: 0.0, kind: "husk".into(), count: 2 }],
            objective: Objective::Cull,
            difficulties: [(
                "elite".to_string(),
                Difficulty {
                    name: "Elite".into(),
                    target_duration_s: 300.0,
                    kill_target: 5,
                    waves: vec![
                        Wave { at_s: 0.0, kind: "husk".into(), count: 3 },
                        Wave { at_s: 60.0, kind: "scuttler".into(), count: 2 },
                    ],
                },
            )]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn a_sortie_without_a_difficulty_flies_the_templates_own_numbers() {
        // The direct drop-in `--mission <name>`, i.e. every script and every test written
        // before the hub existed. It must not silently pick a level.
        let t = two_shapes();
        let n = resolve(&t, None).expect("no difficulty is a legal sortie");
        assert_eq!(n.target_duration_s, 330.0);
        assert_eq!(n.kill_target, 3);
        assert_eq!(n.waves.len(), 1);
        assert_eq!(n.name, "Skirmish", "with no level the name carries no level");
    }

    #[test]
    fn a_difficulty_replaces_all_three_numbers_and_not_one_of_them() {
        // ⭐ The test that catches the cheap version of this feature: a difficulty that only
        // moves `kill_target` and lets the deadline and the waves stay behind. Five kills
        // against the 330 s clock and two husks would pass a looser test and be a different
        // mission from the one the file describes.
        let t = two_shapes();
        let n = resolve(&t, Some("elite")).expect("`elite` stands in the file");
        assert_eq!(n.kill_target, 5, "the level's kill target");
        assert_eq!(n.target_duration_s, 300.0, "the level's deadline, not the template's 330");
        assert_eq!(n.waves.len(), 2, "the level's waves, not the template's one");
        assert_eq!(n.waves[1].kind, "scuttler", "and its kinds — an elite sortie is worse, not longer");
        assert_eq!(n.name, "Skirmish · Elite");
        assert_eq!(n.objective, &Objective::Cull, "and the MODE is still the template's");
    }

    #[test]
    fn a_difficulty_cannot_change_the_mode() {
        // ⭐ The fourth number a level does **not** own. `data::Difficulty` has no `objective`
        // field at all, so this is a guard against somebody adding one and wiring it here: a
        // breach whose elite level is a cull is two missions under one name.
        let mut t = two_shapes();
        t.objective = Objective::Breach { gate_m: (0.0, 0.0, -14.0), reach_m: 7.0, breaches_allowed: 3 };
        let level = resolve(&t, Some("elite")).expect("`elite` stands in the file");
        let direct = resolve(&t, None).expect("no difficulty is a legal sortie");
        assert_eq!(level.objective, direct.objective, "the level flew a different mode");
        assert!(matches!(level.objective, Objective::Breach { .. }));
    }

    #[test]
    fn a_difficulty_the_file_does_not_know_resolves_to_nothing() {
        // No fallback to the template. A pad that names a level nobody wrote must refuse to
        // deploy, loudly — falling back would fly the wrong mission and look like it worked.
        assert!(resolve(&two_shapes(), Some("impossible")).is_none());
    }

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
