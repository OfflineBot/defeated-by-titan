//! **What the sortie earned, in words** — the one place that turns a [`Career`] into lines.
//!
//! `menu::debrief` draws these as plate rows and `hud::career` draws them as one text, and
//! **neither of them formats a number itself.** That is `CLAUDE.md` rule 5's corollary applied
//! to a screen: two implementations of "what does the debrief say" drift, and no sweep finds it
//! because both are ours. One writer decides, everyone else reads the answer.
//!
//! ## Why there are two surfaces at all, and why it is not a choice
//!
//! `src/lib.rs` builds `primary_window: None` for **both** `--headless` and `--offscreen`, so
//! `menu`'s whole `Update` chain is gated off and the debrief *plate* cannot exist in any run
//! this machine can make (`FIND-189`, and `menu::board` carries the long form). A plate is what
//! the player sees; a HUD text is the only thing a screenshot can ever contain. So the ledger
//! is one string list with two renderers — exactly the shape `hud::board` already uses for the
//! mission list, and for the identical reason.
//!
//! ## What is deliberately NOT in here
//!
//! No curve, no threshold, no multiplier. Every number these lines quote is already decided by
//! [`Career::of`](super::career::Career::of) out of `progress.ron`; this file chooses word
//! order and nothing else. A `progress.ron` rebalance changes what these lines say without one
//! character of this file moving — which is the test that a formatter has stayed a formatter.

use super::Career;

/// The heading the debrief's ledger column carries on both surfaces.
pub const HEADING: &str = "CAREER";

/// What one point of an unspent gear budget is called on screen. Named because
/// `tests/progress.rs` matches on it and a screenshot is decoded against it.
pub const UNSPENT: &str = "UNSPENT";

/// The word a level-up prints. **A word and not a colour** — `docs/conventions.md`'s
/// colour-blindness rule, the same reason `hud::board`'s cursor is `>` and not a hue, and the
/// same reason a screenshot can be decoded without trusting a channel.
pub const PROMOTED: &str = "PROMOTED";

/// **The ledger, top to bottom.** Line 0 is always what this sortie was worth.
///
/// The order is the order of the questions a player actually asks: *what did that get me*,
/// *where am I now*, *how far to the next one*, *what am I ranked*, *what can I spend*.
///
/// ⚠️ The last line only appears when there is something to spend, and that is the whole point
/// of it: a career that is fully allocated must not nag, and a career with 122 points sitting
/// in it has to say so **every single sortie** until they are gone. Measured 2026-09-01: the
/// shipped save had flown 419 sorties and spent 0 of 122 points, because nothing anywhere had
/// ever mentioned that they existed (`docs/FINDINGS.md` FIND-222).
pub fn debrief_lines(career: &Career) -> Vec<String> {
    let mut out = Vec::new();

    // 1. The earnings. The reason this screen exists at all.
    out.push(format!("+{} XP", career.last_sortie_xp));

    // 2. Where that leaves him, and whether it moved.
    match career.levelled_up_to {
        Some(level) => out.push(format!("LEVEL {level}  {PROMOTED}")),
        None => out.push(format!("LEVEL {}", career.level)),
    }

    // 3. How far to the next one. `None` is the ceiling and it says so rather than dividing.
    match career.xp_for_the_next_level {
        Some(step) => out.push(format!(
            "{} / {} XP to level {}",
            career.xp_into_level,
            step,
            career.level + 1
        )),
        None => out.push("MAX LEVEL".to_string()),
    }

    // 4. The rank, out of `progress.ron: ranks`.
    out.push(format!("RANK {}", career.rank));

    // 5. And the budget — only when there is one to spend.
    let unspent = career.gear_points.saturating_sub(career.gear_points_spent);
    if unspent > 0 {
        out.push(format!("{unspent} GEAR POINTS {UNSPENT}"));
    }

    out
}

/// **The standing, in one line** — for the corner of a screen rather than a column on a plate.
///
/// It carries no delta: it is what a player IS, not what a sortie DID, and it is the line the
/// hub draws while nothing has just been earned.
pub fn standing_line(career: &Career) -> String {
    let unspent = career.gear_points.saturating_sub(career.gear_points_spent);
    let mut line = format!("LEVEL {}   RANK {}", career.level, career.rank);
    if unspent > 0 {
        line.push_str(&format!("   {unspent} GP {UNSPENT}"));
    }
    line
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    /// A career built by hand and not out of `progress.ron`: these tests are about **word
    /// order**, and they must not go red when somebody rebalances the curve. The curve's own
    /// tests read the real file (`tests/progress.rs`).
    fn career(last_xp: u64, levelled: Option<u32>, earned: u32, spent: u32) -> Career {
        Career {
            level: 59,
            xp: 80_618,
            xp_into_level: 2_480,
            xp_for_the_next_level: Some(3_640),
            skill_points: 58,
            gear_points: earned,
            gear_points_spent: spent,
            rank: "A".to_string(),
            last_sortie_xp: last_xp,
            levelled_up_to: levelled,
            cleared: BTreeSet::new(),
            gear: BTreeMap::new(),
        }
    }

    /// 🔴 **The line the whole round is for.** The debrief's first line is what the sortie was
    /// worth, and before 2026-09-01 there was no line at all.
    #[test]
    fn f120_the_first_line_of_the_ledger_is_what_the_sortie_earned() {
        let lines = debrief_lines(&career(340, None, 122, 0));
        assert_eq!(lines[0], "+340 XP", "the earnings lead, or the screen buries them");
    }

    /// A level-up is a **word**, and it is on the level line rather than a line of its own —
    /// so a career that did not level up has no blank row where the promotion would be.
    #[test]
    fn f120_a_level_up_is_said_out_loud_and_a_flat_sortie_is_not() {
        let up = debrief_lines(&career(340, Some(60), 122, 0));
        assert!(up.iter().any(|l| l.contains(PROMOTED)), "{up:?}");
        assert!(up.iter().any(|l| l.contains("LEVEL 60")), "{up:?}");

        let flat = debrief_lines(&career(340, None, 122, 0));
        assert!(!flat.iter().any(|l| l.contains(PROMOTED)), "{flat:?}");
        assert!(flat.iter().any(|l| l.contains("LEVEL 59")), "{flat:?}");
    }

    /// 🔴 **The nudge, and its off switch.** A budget with something in it says so; a spent
    /// budget says nothing at all. Both halves, because a nag that cannot stop is a nag
    /// everybody learns to skip.
    #[test]
    fn f122_an_unspent_budget_is_named_and_a_spent_one_is_silent() {
        let idle = debrief_lines(&career(340, None, 122, 0));
        assert!(
            idle.iter().any(|l| l.contains("122 GEAR POINTS") && l.contains(UNSPENT)),
            "122 points sat unspent for 419 sorties because of exactly this line: {idle:?}"
        );

        let spent = debrief_lines(&career(340, None, 122, 122));
        assert!(
            !spent.iter().any(|l| l.contains(UNSPENT)),
            "a fully allocated career must not nag: {spent:?}"
        );

        // And the boundary itself: one point left is still a point.
        let almost = debrief_lines(&career(340, None, 122, 121));
        assert!(almost.iter().any(|l| l.contains("1 GEAR POINTS")), "{almost:?}");
    }

    /// The ceiling does not divide. `xp_for_the_next_level: None` is `progress.ron`'s
    /// `max_level` reached, and a screen that printed `2480 / 0` would be the honest bug.
    #[test]
    fn f120_a_maxed_career_says_so_instead_of_quoting_a_next_level() {
        let mut maxed = career(340, None, 204, 0);
        maxed.level = 100;
        maxed.xp_for_the_next_level = None;
        let lines = debrief_lines(&maxed);
        assert!(lines.iter().any(|l| l == "MAX LEVEL"), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("to level 101")), "{lines:?}");
    }

    /// The one-line form carries the standing and never a delta — it is drawn in the hub,
    /// where nothing has just been earned.
    #[test]
    fn f121_the_standing_line_carries_the_level_and_the_rank_and_no_delta() {
        let line = standing_line(&career(340, None, 122, 0));
        assert!(line.contains("LEVEL 59"), "{line}");
        assert!(line.contains("RANK A"), "{line}");
        assert!(!line.contains("340"), "the standing is not the last sortie: {line}");
    }
}
