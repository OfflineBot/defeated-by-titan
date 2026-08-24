//! What a player carries out of a sortie and back into the next one.
//!
//! **A profile is per player and it is a component, not a resource** (`docs/multiplayer.md`
//! rule 3). There is no such thing as *the* profile, exactly as there is no such thing as *the*
//! player: twenty people fly one sortie and each of them walks away with his own record. A
//! `Resource<Profile>` would give the whole squad one shared career the first time a second
//! player exists — the same arithmetic that keeps `Gas` a component, and the very shape
//! `Q-038` has open against `PlayerSettings`.
//!
//! It hangs on a [`PlayerId`](crate::shared::PlayerId) and never on an `Entity`: an `Entity` is
//! a local index with a generation, so it means something else after a restart — which is the
//! one thing a save game may not tolerate (`shared::ids`, rule 7).
//!
//! ## What is in it today, and what deliberately is not
//!
//! Only what the game can actually produce right now: a sortie was flown, it was won or lost,
//! titans were felled, the clock ran for a while, and a difficulty was cleared. There is **no**
//! currency, **no** XP curve, **no** trait tree and **no** lineage in here — `F-120`, `F-122`,
//! `F-123`, `F-127` and `F-140` are ⬜ and inventing their fields today would mean writing a
//! save format around numbers nobody has designed. What is here instead is the **shape** they
//! grow into: a versioned record on a stable id, with a migration path that is already
//! exercised ([`super::file`]).
//!
//! A career nobody can spend is still worth having: it is what makes a sortie *count*.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::XpTuning;
use crate::shared::PlayerId;

/// One player's career. **Written by `save` and by nothing else** (`docs/architecture.md`,
/// authority table) — `progress` decides what a sortie *means* and says so with a
/// [`SortieOutcome`]; this domain is the only thing that ever moves a number in here or puts
/// it on disk.
///
/// Every collection in it is **ordered** (`BTreeSet`, never `HashSet`): the file is compared
/// byte for byte by `tests/save.rs`, and a set whose iteration order depends on a hash seed
/// would write a different file on every run and make the whole format untestable.
#[derive(Component, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Every sortie that reached a verdict. An **abandoned** sortie is not one of them: it
    /// never entered `Won` or `Lost`, so nothing was decided and nothing is recorded.
    pub sorties_flown: u32,
    /// How many of those ended in `Won`. `sorties_flown - sorties_won` is the loss count; it
    /// is not stored, because two numbers that have to agree are one number that is wrong by
    /// next week.
    pub sorties_won: u32,
    /// Lifetime cortex kills credited to this player — `mission::KillTally::of`, summed over
    /// every sortie. Squad mates' kills are **not** in here: the tally is per player already.
    pub titans_felled: u32,
    /// The best single sortie. A record you can beat is the cheapest goal a game owns.
    pub best_kills_in_a_sortie: u32,
    /// Time inside a running sortie, in seconds. Hub time and menu time are not in it.
    pub seconds_in_the_field: f32,
    /// Which sorties have been **won**, as `"<template>"` or `"<template>/<difficulty>"`.
    ///
    /// This is the one field that is already a gate: three difficulty tiers exist today
    /// (`missions.ron: skirmish.difficulties`), and "has this player ever cleared Veteran"
    /// is a question the hub will ask the moment `F-121` lands. A `BTreeSet<String>` and not
    /// an enum, for the same reason `TitanKindName` is a `String`: the tiers are RON keys and
    /// the file may grow a fourth one without a rebuild (rule 2).
    pub cleared: BTreeSet<String>,
    /// `F-120` — lifetime experience. **The one number the whole progression spine hangs off**:
    /// the level, the skill points, the gear budget and the rank are all derived from it and
    /// none of them is stored, because two numbers that have to agree are one number that is
    /// wrong by next week.
    pub xp: u64,
    /// `F-122` — how the gear budget has been spent, axis name -> points. A `BTreeMap` for the
    /// same reason `cleared` is a `BTreeSet`: the file is compared byte for byte.
    ///
    /// It is **the allocation and not the result**: what those points do is
    /// `progress::gear`, out of `progress.ron`, and it is recomputed on load. Storing a
    /// derived stat would freeze yesterday's balance into every save file in existence.
    pub gear: BTreeMap<String, u32>,
}

impl Profile {
    /// Books one finished sortie. **The only thing that changes a profile.**
    ///
    /// It is deliberately total and boring — no branching on difficulty, no multiplier, no
    /// reward. The moment a number here depends on tuning, that number belongs in a RON file
    /// and this function takes it as an argument (rule 2).
    pub fn record(&mut self, outcome: &SortieOutcome, xp: &XpTuning) -> u64 {
        let earned = xp_earned(outcome, xp);
        self.xp = self.xp.saturating_add(earned);
        self.sorties_flown += 1;
        if outcome.won {
            self.sorties_won += 1;
            self.cleared.insert(outcome.cleared_key());
        }
        self.titans_felled += outcome.kills;
        self.best_kills_in_a_sortie = self.best_kills_in_a_sortie.max(outcome.kills);
        self.seconds_in_the_field += outcome.seconds.max(0.0);
        earned
    }

    /// One line for the log, so that a headless run says out loud what it carried in.
    pub fn one_line(&self) -> String {
        format!(
            "{} sorties ({} won), {} titans felled, best {}, {:.1} s in the field",
            self.sorties_flown,
            self.sorties_won,
            self.titans_felled,
            self.best_kills_in_a_sortie,
            self.seconds_in_the_field
        )
    }
}

/// `F-120` — **what one finished sortie is worth**, out of `progress.ron` and nowhere else.
///
/// Four facts, four rates, one tier multiplier. The shape is deliberately flat: no streak bonus,
/// no first-win-of-the-day, no catch-up curve. Each of those is a design decision somebody has
/// to make, and inventing one here would put it in Rust where nobody can tune it (rule 2).
///
/// **A win is a bonus on top of the floor, not a replacement for it.** A defeat still earns
/// `per_sortie_flown` plus its kills plus its time — a bad night has to be worth something or
/// the only rational play after the first death is to quit to the hub.
pub fn xp_earned(outcome: &SortieOutcome, xp: &XpTuning) -> u64 {
    let minutes = outcome.seconds.max(0.0) / 60.0;
    let base = xp.per_sortie_flown
        + if outcome.won { xp.per_sortie_won } else { 0.0 }
        + xp.per_titan_felled * outcome.kills as f32
        + xp.per_minute_in_the_field * minutes;
    (base.max(0.0) * xp.multiplier_for(outcome.difficulty.as_deref())).max(0.0) as u64
}

/// `F-201` — **what a career written before the XP curve existed is worth.**
///
/// The migration arm for schema 1 (`src/save/file.rs`). A career that has flown four sorties must
/// not come back as level 1 because this build learned to count: the sorties happened. The four
/// numbers a schema-1 file carries are exactly the four facts [`xp_earned`] is paid for, so the
/// old record is re-paid at the no-tier rate — the file does not remember which tiers they were,
/// and inventing a tier for them would hand out experience nobody earned.
pub fn xp_of_a_bare_career(
    flown: u32,
    won: u32,
    felled: u32,
    seconds: f32,
    xp: &XpTuning,
) -> u64 {
    let base = flown as f32 * xp.per_sortie_flown
        + won as f32 * xp.per_sortie_won
        + felled as f32 * xp.per_titan_felled
        + xp.per_minute_in_the_field * (seconds.max(0.0) / 60.0);
    (base.max(0.0) * xp.without_a_difficulty).max(0.0) as u64
}

/// **This sortie is over, and this is what this player did in it** — `progress` asking, `save`
/// writing.
///
/// The same seam as `shared::RefuelRequest` and for the same reason (`FINDINGS.md` FIND-063):
/// [`Profile`] has exactly one writer. `progress` owns the *meaning* of a finished sortie — it
/// is the domain that knows a verdict has fallen and who was in it — and `save` owns the record
/// and the file. A `progress` that reached into a `&mut Profile` itself would be a second
/// authority on the one piece of state the whole career hangs on, and over a wire two machines
/// disagreeing about how many sorties somebody has flown.
///
/// It carries **facts, not rewards**: kills and seconds and a verdict, never "12 XP". What a
/// sortie is worth is `F-120`'s question and it is not answered here.
///
/// One message **per player**, not one per sortie: twenty people fly the same sortie and each
/// of them gets his own line, which is what keeps this correct on the day `net` is real.
#[derive(Message, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortieOutcome {
    pub player: PlayerId,
    /// The key in `missions.ron: templates` — `"skirmish"`.
    pub template: String,
    /// The key in that template's `difficulties`, or `None` for the direct drop-in
    /// `--mission <name>` (`mission::SortieOrder`).
    pub difficulty: Option<String>,
    pub won: bool,
    /// This player's own credited kills, out of `mission::KillTally::of`.
    pub kills: u32,
    /// How long the sortie ran, derived from the tick count and `game.ron: simulation_hz` —
    /// never from a wall clock, so two machines that reach tick *n* agree.
    pub seconds: f32,
    /// The tick the verdict fell on. In the message because a save is a thing you have to be
    /// able to argue about afterwards.
    pub tick: u64,
}

impl SortieOutcome {
    /// What goes into [`Profile::cleared`]: `"skirmish/veteran"`, or `"tutorial"` when the
    /// sortie was entered directly and has no tier.
    pub fn cleared_key(&self) -> String {
        match &self.difficulty {
            Some(d) => format!("{}/{}", self.template, d),
            None => self.template.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tuning literal and not `progress.ron`: these five tests are about the BOOKKEEPING —
    /// that a loss still counts, that the best sortie is a maximum — and they must not go red
    /// when somebody rebalances the curve. The curve's own tests live in `tests/progress.rs`
    /// and read the real file.
    fn xp_tuning() -> XpTuning {
        XpTuning {
            per_sortie_flown: 10.0,
            per_sortie_won: 0.0,
            per_titan_felled: 0.0,
            per_minute_in_the_field: 0.0,
            difficulty_multipliers: BTreeMap::from([("veteran".to_string(), 1.0)]),
            without_a_difficulty: 1.0,
        }
    }

    fn outcome(won: bool, kills: u32, seconds: f32) -> SortieOutcome {
        SortieOutcome {
            player: PlayerId(1),
            template: "skirmish".into(),
            difficulty: Some("veteran".into()),
            won,
            kills,
            seconds,
            tick: 100,
        }
    }

    #[test]
    fn a_lost_sortie_still_counts_as_flown() {
        // The one thing that makes a defeat worth anything: it happened, and the record says
        // so. A profile that only counted wins would make a bad night invisible.
        let mut p = Profile::default();
        p.record(&outcome(false, 2, 30.0), &xp_tuning());
        assert_eq!(p.sorties_flown, 1);
        assert_eq!(p.sorties_won, 0);
        assert_eq!(p.titans_felled, 2);
        assert!(p.cleared.is_empty(), "a lost sortie clears nothing");
    }

    #[test]
    fn only_a_win_clears_a_difficulty() {
        let mut p = Profile::default();
        p.record(&outcome(true, 3, 60.0), &xp_tuning());
        assert_eq!(p.cleared.iter().collect::<Vec<_>>(), vec!["skirmish/veteran"]);
    }

    #[test]
    fn the_direct_drop_in_has_no_tier_in_its_key() {
        // `--mission tutorial` flies the template's own numbers and belongs to no difficulty.
        let mut o = outcome(true, 1, 5.0);
        o.template = "tutorial".into();
        o.difficulty = None;
        assert_eq!(o.cleared_key(), "tutorial");
    }

    #[test]
    fn the_best_sortie_is_a_maximum_and_never_the_last_one() {
        let mut p = Profile::default();
        p.record(&outcome(true, 5, 10.0), &xp_tuning());
        p.record(&outcome(false, 1, 10.0), &xp_tuning());
        assert_eq!(p.best_kills_in_a_sortie, 5, "a bad night does not erase the record");
        assert_eq!(p.titans_felled, 6);
        assert_eq!(p.seconds_in_the_field, 20.0);
    }

    #[test]
    fn a_negative_clock_cannot_shrink_a_career() {
        // `tick - started_at_tick` is a subtraction, and a subtraction that ever goes the
        // wrong way must not take time back off the record.
        let mut p = Profile::default();
        p.record(&outcome(false, 0, -50.0), &xp_tuning());
        assert_eq!(p.seconds_in_the_field, 0.0);
    }
}
