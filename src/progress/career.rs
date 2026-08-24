//! `F-120` and `F-121` — **the level, what it hands out, and the rank it is worth.**
//!
//! Everything in here is a **pure function of a [`Profile`] and `progress.ron`**. Nothing is
//! stored: the level, the skill points, the gear budget and the rank are all derived, every
//! time, from the one number the save file actually carries ([`Profile::xp`]). That is the same
//! rule `src/save/profile.rs` already states for the loss count — *two numbers that have to
//! agree are one number that is wrong by next week* — and here it buys something concrete: a
//! rebalance of `progress.ron` re-levels every career on the next load instead of leaving a
//! hundred save files quoting yesterday's curve.
//!
//! ## Why the whole curve is three numbers and not a hundred rows
//!
//! `F-120`'s acceptance is *"the curve is defined in config and adjustable without a code
//! change"*. The step from level `n` to `n+1` costs
//! `first_step_xp * step_growth^(n-1)`, rounded, and a level is the largest one whose steps fit
//! inside the career. A designer moves two numbers and the whole ladder moves with them; a
//! table of a hundred rows would be adjustable too, and nobody would ever adjust it.
//!
//! ## Rule 4 — [`Career`] is a component, keyed by the player
//!
//! Not a `Resource`, for the identical reason [`Profile`] is not one: twenty people fly one
//! sortie and each of them walks away with his own level. `tests/progress.rs` queries it per
//! player and never calls `.single()`.

use bevy::prelude::*;

use crate::data::{LevelTuning, Progress, RankTier};
use crate::save::Profile;

use super::gear;

/// **What a screen shows and what a gate asks**, derived from the profile every time the
/// profile changes.
///
/// It carries nothing the [`Profile`] does not already imply — it is a cache with a name, and
/// the name is what keeps `hud` and `menu` from each re-deriving the curve slightly differently.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Career {
    pub level: u32,
    /// Lifetime experience — the one stored number the rest of this file is a function of.
    pub xp: u64,
    /// How far into the current level, and how big that level is: together they are the bar the
    /// debrief draws. `None` above means the ceiling has been reached.
    pub xp_into_level: u64,
    pub xp_for_the_next_level: Option<u64>,
    /// `F-120`: "every level gives skill points". **Granted and unspent** — the tree they go
    /// into is `F-123` and it is not built.
    pub skill_points: u32,
    /// `F-122`'s budget, in full — what the career has *earned*, not what is left.
    pub gear_points: u32,
    pub gear_points_spent: u32,
    /// `F-121`, E..S.
    pub rank: String,
    /// What the sortie that just ended was worth. **Zero except in the frames right after a
    /// verdict** — this is the number the debrief exists to show.
    pub last_sortie_xp: u64,
    /// Set for the same few frames when that sortie crossed a level boundary.
    pub levelled_up_to: Option<u32>,
}

/// What the step from `from_level` to the next one costs. Zero at and above the ceiling.
///
/// Rounded per step and then summed — **not** summed and then rounded. The difference is a few
/// XP at level 100 and it matters for one reason only: [`xp_for_level`] and [`level_for_xp`]
/// have to agree exactly at every boundary, and they can only do that if there is one
/// definition of a step.
pub fn step_xp(levels: &LevelTuning, from_level: u32) -> u64 {
    if from_level == 0 || from_level >= levels.max_level {
        return 0;
    }
    let step = levels.first_step_xp as f64 * (levels.step_growth as f64).powi(from_level as i32 - 1);
    if !step.is_finite() || step <= 0.0 {
        // A `step_growth` of 0 or a negative `first_step_xp` is a broken file, not a game state.
        // One XP per level keeps the curve monotone so nothing downstream divides by zero.
        return 1;
    }
    step.round().max(1.0) as u64
}

/// The **total** experience it takes to be `level`. Level 1 costs nothing — that is where a
/// career starts.
pub fn xp_for_level(levels: &LevelTuning, level: u32) -> u64 {
    let mut total = 0u64;
    for from in 1..level.min(levels.max_level) {
        total = total.saturating_add(step_xp(levels, from));
    }
    total
}

/// The level a career with this much experience is at. Monotone, capped, and exact at the
/// boundary in both directions (`tests/progress.rs`).
pub fn level_for_xp(levels: &LevelTuning, xp: u64) -> u32 {
    let mut level = 1u32;
    let mut spent = 0u64;
    while level < levels.max_level {
        let step = step_xp(levels, level);
        match spent.checked_add(step) {
            Some(next) if next <= xp => {
                spent = next;
                level += 1;
            }
            _ => break,
        }
    }
    level
}

/// `F-120`: what a level is worth in skill points. Level 1 has not levelled up yet.
pub fn skill_points(levels: &LevelTuning, level: u32) -> u32 {
    levels.skill_points_per_level * level.saturating_sub(1)
}

/// `F-122`'s budget, as a function of the level and nothing else.
pub fn gear_points(levels: &LevelTuning, level: u32) -> u32 {
    levels.gear_points_at_level_one + levels.gear_points_per_level * level.saturating_sub(1)
}

/// `F-121` — the letter for a budget. The highest rung whose threshold is reached.
///
/// It does **not** assume the list is sorted: it takes the largest threshold at or below the
/// points. `tests/progress.rs::f121_the_rank_ladder_is_ascending_and_starts_at_zero` is what
/// keeps the file sorted for a human's sake; this function is what keeps the game right anyway.
pub fn rank_for(ranks: &[RankTier], gear_points: u32) -> &str {
    ranks
        .iter()
        .filter(|r| r.min_gear_points <= gear_points)
        .max_by_key(|r| r.min_gear_points)
        .or_else(|| ranks.iter().min_by_key(|r| r.min_gear_points))
        .map(|r| r.name.as_str())
        .unwrap_or("")
}

/// Where a rank stands on the ladder, in **file order** — which is why the ladder has to be
/// ascending, and why a test says so.
pub fn rank_index(ranks: &[RankTier], name: &str) -> Option<usize> {
    ranks.iter().position(|r| r.name == name)
}

/// `F-121` — **may a career of this rank fly this door?**
///
/// The key is the one [`crate::save::SortieOutcome::cleared_key`] produces:
/// `"skirmish/veteran"`, or a bare `"tutorial"` for a mission with no tier. A door that is not
/// in `progress.ron: gates` is open to everybody, and the shipped file gates nothing at all —
/// the reason stands in that file and in `docs/QUESTIONS.md` Q-051.
///
/// **An unknown rank name locks the door rather than opening it.** A typo in the gate must not
/// be the thing that lets everybody through; it has to be the thing somebody notices.
pub fn may_fly(progress: &Progress, career_rank: &str, key: &str) -> bool {
    let Some(required) = progress.gates.get(key) else { return true };
    let Some(needs) = rank_index(&progress.ranks, required) else {
        error!(
            "progress.ron: gate {key:?} asks for rank {required:?}, which is not on the ladder \
             — the door stays shut"
        );
        return false;
    };
    match rank_index(&progress.ranks, career_rank) {
        Some(has) => has >= needs,
        None => false,
    }
}

impl Career {
    /// The whole derivation, in one place. **The only place** — `hud` and `menu` read this and
    /// never the curve.
    pub fn of(profile: &Profile, p: &Progress) -> Career {
        let level = level_for_xp(&p.levels, profile.xp);
        let earned = gear_points(&p.levels, level);
        Career {
            level,
            xp: profile.xp,
            xp_into_level: profile.xp.saturating_sub(xp_for_level(&p.levels, level)),
            xp_for_the_next_level: (level < p.levels.max_level)
                .then(|| step_xp(&p.levels, level)),
            skill_points: skill_points(&p.levels, level),
            gear_points: earned,
            gear_points_spent: gear::spent_points(&profile.gear),
            rank: rank_for(&p.ranks, earned).to_string(),
            last_sortie_xp: 0,
            levelled_up_to: None,
        }
    }

    /// The same career, plus **what changed since `before`** — the two numbers a debrief screen
    /// is made of. Kept out of [`Career::of`] on purpose: a delta is a fact about two moments
    /// and not about a profile.
    pub fn after(&self, before: &Career) -> Career {
        Career {
            last_sortie_xp: self.xp.saturating_sub(before.xp),
            levelled_up_to: (self.level > before.level).then_some(self.level),
            ..self.clone()
        }
    }

    /// One line for the log, so a headless run says out loud what it carried in and out.
    pub fn one_line(&self) -> String {
        match self.xp_for_the_next_level {
            Some(step) => format!(
                "level {} ({}/{} xp), rank {}, {} gear points ({} spent), {} skill points",
                self.level,
                self.xp_into_level,
                step,
                self.rank,
                self.gear_points,
                self.gear_points_spent,
                self.skill_points
            ),
            None => format!(
                "level {} (max), rank {}, {} gear points ({} spent), {} skill points",
                self.level, self.rank, self.gear_points, self.gear_points_spent, self.skill_points
            ),
        }
    }
}
