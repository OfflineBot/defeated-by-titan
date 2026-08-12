//! The six phases of a session — `F-070`, and the hub loop of 2026-08-12.
//!
//! ```text
//! Hub ──(a player stands on a deployment pad)──► Deploying ──► Active ──┬─► Won ──┐
//!  ▲                                                                    └─► Lost ─┤
//!  └────────────────── after `missions.ron: hub.debrief_s` ─────────────────────── ┘
//! ```
//!
//! `Briefing` is still there and still the default: it is the phase of a run that has **no**
//! session at all (`--sandbox`, a plain `cargo run`, every script that names no mission). There
//! is no `Paused` (that is `menu`), no `Extraction` (that is `F-073`, not built) and no
//! `Restart` — a variant nothing enters and nothing leaves is decoration, and `titan/brain.rs`
//! says the same thing about `Alerted` and `Stagger`.
//!
//! ## Why the hub is a phase here and not a `Screen` in `menu`
//!
//! Because it is **a place, not a screen**. In the hub the pointer stays locked, time keeps
//! running and the player walks — that is `Screen::Playing` in every respect, and widening
//! `menu::Screen` would have made "the game is paused" and "the player is in the hub" two
//! answers to the same question that can disagree. One enum, one truth about where the session
//! is; `tests/menu.rs::f072_the_hub_is_a_place_and_not_a_screen` is the guard.
//!
//! Three things fall out of that for free: every existing reader (`hud`, the F3 overlay,
//! `assert phase` in a script) sees the hub without changing a line, the hub's props get their
//! lifetime from `DespawnOnExit(MissionPhase::Hub)`, and a script can say `assert phase == 5`.
//!
//! ## Why a `Resource` is the right shape here, and why that is not a breach of rule 4
//!
//! `docs/multiplayer.md` rule 4 says: **player state is never a `Resource`.** The reason is
//! arithmetic, not taste — there are *many* players, and a resource can only hold one of
//! anything, so the first co-op session finds every one of them sharing a single gas tank.
//!
//! **There is exactly one mission, and every player in it is in the same one.** A player who
//! were in another phase than his squad mates would not be a feature, he would be the bug.
//! So the phase is the one piece of state that a `Resource` describes *correctly*:
//! [`State<MissionPhase>`](bevy::prelude::State) is written by `mission` and read by `hud`,
//! `debug` and `sound`, and there is nothing per player about it.
//!
//! **What is per player sits per player**, right next to it and on purpose: the kill counter
//! is a [`KillTally`](super::run::KillTally) with one number per
//! [`PlayerId`](crate::shared::PlayerId), not a `Resource<u32>` — `F-096` and `F-161a` want
//! per-player credit later, and it costs nothing today (`docs/PLAN-GAME.md` §5).
//!
//! Whoever reads this in three weeks and reaches for rule 4: the rule is about *player* state.
//! This is not player state. Do not "fix" it into a component on the player — one mission
//! whose phase lives in five places is five phases by next week.

use bevy::prelude::*;

/// Where the sortie stands. **The one truth about the phase**, written only by `mission`.
///
/// `Briefing` is the initial state and it is also the state of a game that has **no** mission:
/// `cargo run` and `--sandbox` stay here forever, and every mission system is gated on
/// `Active`, so nothing in this domain runs at all without `--mission`.
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum MissionPhase {
    /// The template is picked, nothing is in the world yet.
    #[default]
    Briefing,
    /// The mission entity, its clock and its counter come into being.
    Deploying,
    /// The clock runs, the waves come, kills count. **The only phase in which anything of
    /// this domain ticks.**
    Active,
    /// `kill_target` reached.
    Won,
    /// The clock ran out, or every player is out of the fight.
    Lost,
    /// **The main building.** You walk, the Vector Gear is idle, gas comes back at the
    /// stations, and a deployment pad starts the next sortie (user, 2026-08-12).
    ///
    /// Not the default: a run that says nothing about a session must not silently be standing
    /// in a hub with live trigger volumes in it — that is `--hub`'s job, and `Briefing`
    /// remains the honest reading of "nobody asked for anything".
    Hub,
}

impl MissionPhase {
    /// The number a script compares against — `assert phase == 4` is `Lost`.
    ///
    /// A script's `assert` measures an `f32` and nothing else (`debug::script::Metric`), so the
    /// enum needs a number. It is written down **here**, next to the variants, and not in the
    /// parser: a script that means `Lost` and gets `Won` because somebody inserted a variant in
    /// the middle is a green run that measured the opposite of what it says.
    ///
    /// **Append only.** New phases get the next free number, they never take one.
    pub fn code(self) -> u8 {
        match self {
            MissionPhase::Briefing => 0,
            MissionPhase::Deploying => 1,
            MissionPhase::Active => 2,
            MissionPhase::Won => 3,
            MissionPhase::Lost => 4,
            // Appended, not inserted: `scripts/f070-lost.txt` says `assert phase == 4` and
            // `scripts/f071-won.txt` says `== 2`. A variant that took a number would turn
            // both of them into green runs measuring the opposite of what they claim.
            MissionPhase::Hub => 5,
        }
    }

    /// The word on the screen. `hud` draws it, the F3 overlay carries it until then.
    pub fn label(self) -> &'static str {
        match self {
            MissionPhase::Briefing => "BRIEFING",
            MissionPhase::Deploying => "DEPLOYING",
            MissionPhase::Active => "ACTIVE",
            MissionPhase::Won => "WON",
            MissionPhase::Lost => "LOST",
            MissionPhase::Hub => "HUB",
        }
    }

    /// Whether a verdict has been spoken.
    ///
    /// ⚠️ **This is no longer terminal.** Until 2026-08-12 `Won` and `Lost` were the end of the
    /// run; a sortie deployed out of the hub now leaves them again after
    /// `missions.ron: hub.debrief_s` (`mission::hub::return_to_hub`). A sortie started with
    /// `--mission <name>` still stays where it lands — it came from nowhere, so there is
    /// nowhere to go back to, and three scripts and two tests measure the verdict long after
    /// it fell.
    pub fn is_decided(self) -> bool {
        matches!(self, MissionPhase::Won | MissionPhase::Lost)
    }

    /// Whether this is a phase in which a sortie is **running** — the mission entity exists and
    /// its clock is ticking.
    pub fn is_running(self) -> bool {
        matches!(self, MissionPhase::Deploying | MissionPhase::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phase_has_its_own_number() {
        // The numbers are what a script writes down. Two phases sharing one would make
        // `assert phase == 3` pass on a lost mission.
        let all = [
            MissionPhase::Briefing,
            MissionPhase::Deploying,
            MissionPhase::Active,
            MissionPhase::Won,
            MissionPhase::Lost,
            MissionPhase::Hub,
        ];
        let mut codes: Vec<u8> = all.iter().map(|p| p.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "two phases share a number: {codes:?}");
        assert_eq!(MissionPhase::Lost.code(), 4, "scripts written today say 4 for Lost");
        assert_eq!(MissionPhase::Won.code(), 3);
        // The hub was appended on 2026-08-12 and took nobody's number. Held explicitly,
        // because `scripts/f070-lost.txt` and `scripts/f071-won.txt` are files this test
        // cannot see.
        assert_eq!(MissionPhase::Hub.code(), 5, "the hub takes the next free number, not a used one");
    }

    #[test]
    fn only_won_and_lost_are_a_verdict() {
        assert!(MissionPhase::Won.is_decided());
        assert!(MissionPhase::Lost.is_decided());
        assert!(!MissionPhase::Active.is_decided());
        assert!(!MissionPhase::Briefing.is_decided());
        assert!(!MissionPhase::Deploying.is_decided());
        // The hub is not a verdict — it is where you are when there is none.
        assert!(!MissionPhase::Hub.is_decided());
    }

    #[test]
    fn only_deploying_and_active_are_a_running_sortie() {
        // What `hub::return_to_hub` and the despawn of the mission entity hang on: in the hub
        // there is no clock, and after the verdict the clock has stopped.
        assert!(MissionPhase::Deploying.is_running());
        assert!(MissionPhase::Active.is_running());
        assert!(!MissionPhase::Hub.is_running());
        assert!(!MissionPhase::Briefing.is_running());
        assert!(!MissionPhase::Won.is_running());
        assert!(!MissionPhase::Lost.is_running());
    }

    #[test]
    fn a_game_without_a_mission_starts_in_briefing() {
        // `--sandbox` and a plain `cargo run` must not silently be in a mission.
        assert_eq!(MissionPhase::default(), MissionPhase::Briefing);
    }
}
