//! The five phases of a sortie — `F-070`.
//!
//! `Briefing → Deploying → Active → (Won | Lost)`. Nothing else. There is no `Paused` (that is
//! `menu`), no `Extraction` (that is `F-073`, not built) and no `Restart` — a variant nothing
//! enters and nothing leaves is decoration, and `titan/brain.rs` says the same thing about
//! `Alerted` and `Stagger`.
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
        }
    }

    /// Whether a verdict has been spoken. **Once true, it never goes back** in this build:
    /// there is no restart (`F-074`, not built).
    pub fn is_decided(self) -> bool {
        matches!(self, MissionPhase::Won | MissionPhase::Lost)
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
        ];
        let mut codes: Vec<u8> = all.iter().map(|p| p.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "two phases share a number: {codes:?}");
        assert_eq!(MissionPhase::Lost.code(), 4, "scripts written today say 4 for Lost");
        assert_eq!(MissionPhase::Won.code(), 3);
    }

    #[test]
    fn only_won_and_lost_are_a_verdict() {
        assert!(MissionPhase::Won.is_decided());
        assert!(MissionPhase::Lost.is_decided());
        assert!(!MissionPhase::Active.is_decided());
        assert!(!MissionPhase::Briefing.is_decided());
        assert!(!MissionPhase::Deploying.is_decided());
    }

    #[test]
    fn a_game_without_a_mission_starts_in_briefing() {
        // `--sandbox` and a plain `cargo run` must not silently be in a mission.
        assert_eq!(MissionPhase::default(), MissionPhase::Briefing);
    }
}
