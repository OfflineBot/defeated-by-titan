//! **The career, on the one surface a run without a window still has.**
//!
//! > *„Ganz ausbauen"* — the user, 2026-09-01 (`docs/QUESTIONS.md`, the Q-062 override).
//!
//! ## Why this element exists at all, and it is not a preference
//!
//! `src/lib.rs` builds `primary_window: None` for **both** `--headless` and `--offscreen`, so
//! `menu`'s whole `Update` chain is gated off and `menu::debrief`'s plate — the thing that
//! actually reports a sortie to a player — **cannot exist in any run this machine can make**
//! (`FIND-189`). A debrief that were only a plate would therefore be a screen no script could
//! ever assert on and no screenshot could ever contain: 🟨 forever, by construction.
//!
//! So the report has two surfaces and one set of words, exactly as the mission list does:
//!
//! | run | what draws the ledger |
//! |---|---|
//! | windowed | `menu::debrief`'s plate — `Screen::Debrief`, and `hud::hide_while_a_menu_is_up` hides this element with the rest of the HUD |
//! | `--headless` / `--offscreen` | **this**, because `Screen` never moves in a run with no window (`menu::board`) |
//!
//! Exactly one of the two is ever on screen, and no rule needed an exception written for it —
//! the same mechanism `hud::board` already relies on.
//!
//! 🔴 **This element formats nothing.** The lines are
//! [`progress::ledger`](crate::progress::ledger)'s, the numbers are
//! [`Career`]'s, and the level, the rank and the budget are all decided by `Career::of` out of
//! `progress.ron`. That is rule 5's corollary applied to a screen: two surfaces formatting one
//! career drift, and no sweep finds it because both are ours. There is no curve, no threshold
//! and no `progress.ron` read in this file.
//!
//! ## When it is up
//!
//! | phase | this panel |
//! |---|---|
//! | `Won` · `Lost` · `Debrief` | the **ledger** — what the sortie earned, five lines |
//! | `Hub` | the **standing** — one line, level and rank and what is unspent |
//! | `Briefing` · `Deploying` · `Active` | nothing. `display: None`, an empty corner |
//!
//! It comes up on the **verdict** and not on the debrief, which is 3.0 s earlier
//! (`missions.ron: hub.verdict_s`): the earnings belong to the moment the sortie was decided,
//! and it also gives a script a window 252 ticks wide to aim a screenshot at instead of the
//! 72 the debrief phase alone would leave.
//!
//! ## Where it stands
//!
//! The **right** margin, against `hud::board`'s left one — in the hub both are up, and they are
//! the two halves of "what do I do next": the board is the door out, this is what walking
//! through it is worth. `LEFT_PCT + WIDTH_PCT` for the board is 36 % against a keep-out box
//! that starts at 40; this panel starts at `100 - 4 - 32 = 64 %`, against a box that ends at 60.
//! `tests/hud.rs` measures the real rect and does not take that arithmetic's word for it.

//! ## The evidence
//!
//! | what | run → picture |
//! |---|---|
//! | the ledger at the debrief, five lines under `CAREER` | `scripts/f120-progress.txt --ticks 701` → `docs/images/f120-debrief-ledger.png` |
//! | the armoury open, two axes bought, the coupling visible | `… --ticks 931` → `docs/images/f125-armoury.png` |
//! | the same run mid-sortie, panel down — **the control** | `… --ticks 431` |
//!
//! Decoded against that control rather than assumed, and every run under `DBT_SAVE_DIR=off` so
//! the career is the same on every run. In the panel's own rect (`x 819..1228, y 187..720` of a
//! 1280 × 720 frame) the control holds **0** pixels of the amber out of `maps.ron`
//! (sRGB 255, 215, 89) while the debrief frame holds **1 382**, resolving into exactly **6**
//! text bands — the heading and `progress::ledger`'s five lines — whose widths track the six
//! strings at a consistent 8.8–9.5 px per character at `LINE_PX` 16. The `F-170` keep-out box
//! holds **0** of it. The shot is bit-identical over two runs (`sha256 52779beb…`).
//!
//! The armoury frame is decoded against its own arithmetic rather than a control, which is the
//! stronger check here: with one point on `speed` and one on `control`, the panel reads
//! `speed 1 +1.00` and `control 1 +0.45`, and `0.45 = 1.00 − 0.55 × 1.00` is exactly
//! `progress.ron`'s `speed -> control` drag. **That is `F-122`'s "speed costs control" on a
//! screen for the first time.**

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::{GameData, GearTuning};
use crate::hud::{signal, HudElement};
use crate::mission::MissionPhase;
use crate::progress::{ledger, loadout, Career, Loadout};
use crate::shared::LocalPlayer;

/// Marker on the career panel.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CareerPanel;

/// Right edge, in percent of the screen — the mirror of [`board::LEFT_PCT`](super::board::LEFT_PCT).
pub const RIGHT_PCT: f32 = 4.0;
/// Top edge. The same band the board panel uses on the other side.
pub const TOP_PCT: f32 = 26.0;
/// How wide. **`100 - RIGHT_PCT - WIDTH_PCT` has to stay above
/// [`KEEP_OUT_HIGH_PCT`](super::KEEP_OUT_HIGH_PCT)** — 64 against a box that ends at 60, i.e.
/// 4 % of the width in hand, the same margin the board keeps on the left.
pub const WIDTH_PCT: f32 = 32.0;
/// One line, in logical pixels. The board panel's size — information you read when you look
/// for it, not a verdict.
pub const LINE_PX: f32 = 16.0;

/// **What the panel says.** `None` means draw nothing at all.
///
/// Pure, and it takes the two answers rather than the world: the phase is `mission`'s and the
/// career is `progress`', both already decided by one writer each. This function's whole job is
/// to pick which of `ledger`'s two forms belongs on screen — which is what makes "what does the
/// panel say" testable without an app, a window, a player and a sortie.
pub fn career_text(
    phase: MissionPhase,
    career: Option<&Career>,
    gear: &GearTuning,
    armoury: &Loadout,
) -> Option<String> {
    // Nothing during a fight. A career is not something you act on at 40 m/s, and the middle of
    // a sortie is the one place this panel would be pure clutter.
    let reporting = match phase {
        MissionPhase::Won | MissionPhase::Lost | MissionPhase::Debrief => true,
        MissionPhase::Hub => false,
        _ => return None,
    };
    // No career is not an error and not a zero: it is a run with nobody in it. An empty corner
    // says that honestly, where `LEVEL 0` would be a number nothing produced.
    let career = career?;

    // ⭐ **The armoury replaces the standing rather than sitting beside it.** One panel, one
    // corner, one thing to read — and `progress::loadout` already guarantees the armoury can
    // only be open in the hub, which is exactly where the standing would otherwise be. Two
    // stacked panels would need a second keep-out argument for a second rect; this needs none.
    if armoury.open && !reporting {
        let mut out = String::new();
        for (i, line) in loadout::armoury_lines(gear, career, armoury.at).iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        return Some(out);
    }

    let mut out = String::from(ledger::HEADING);
    out.push('\n');
    if reporting {
        for line in ledger::debrief_lines(career) {
            out.push('\n');
            out.push_str(&line);
        }
    } else {
        out.push('\n');
        out.push_str(&ledger::standing_line(career));
        // The one line that turns a number into an action. Without it the panel says "6 GP
        // UNSPENT" and never says HOW — which is the shape the whole finding is about.
        out.push_str("\n\nTab   armoury");
    }
    Some(out)
}

/// One `Text`, hidden until a sortie has been decided or the player is home.
pub fn spawn_career_panel(mut commands: Commands, data: Res<GameData>) {
    // Amber: `docs/conventions.md` §3 reserves it for "cortex, weak points, **objectives**", and
    // the deployment pads and the board panel are painted with the same key for the same reason.
    // What a sortie was worth is what the objective was FOR, and in the hub "there are points to
    // spend" is the hub's objective. Read from `maps.ron`, never a literal here.
    let amber = signal(&data, "amber");
    commands.spawn((
        Name::new("hud_career_panel"),
        CareerPanel,
        HudElement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(LINE_PX), ..default() },
        TextLayout::justify(Justify::Left),
        TextColor(amber),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(TOP_PCT),
            right: Val::Percent(RIGHT_PCT),
            width: Val::Percent(WIDTH_PCT),
            display: Display::None,
            ..default()
        },
    ));
}

/// Writes the panel out of the two answers somebody else already gave.
///
/// Every write is compared first. This runs in `Update` and the panel says the same thing for
/// hundreds of frames at a time; an unconditional write would mark the `Node` and the `Text`
/// changed every frame and put the UI layout to work for nothing (`CLAUDE.md` rule 6).
///
/// **No `.single()`** (rule 4): the career drawn is the **local** player's, because a HUD is
/// this machine's overlay. Twenty people fly one sortie and each reads his own.
pub fn update_career_panel(
    phase: Res<State<MissionPhase>>,
    data: Res<GameData>,
    armoury: Res<Loadout>,
    careers: Query<&Career, With<LocalPlayer>>,
    mut panels: Query<(&mut Text, &mut Node), With<CareerPanel>>,
) {
    let text =
        career_text(*phase.get(), careers.iter().next(), &data.progress.gear, &armoury);

    for (mut panel, mut node) in &mut panels {
        match &text {
            Some(t) => {
                if panel.0 != *t {
                    panel.0.clone_from(t);
                }
                if node.display != Display::Flex {
                    node.display = Display::Flex;
                }
            }
            None => {
                if !panel.0.is_empty() {
                    panel.0.clear();
                }
                if node.display != Display::None {
                    node.display = Display::None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::data::{GearAxis, GearCoupling};

    /// A gear block built by hand and not out of `progress.ron`: these tests are about **word
    /// order and the cursor**, and they must not go red when somebody rebalances the file. The
    /// balance itself is `tests/progress.rs`'s, against the real file.
    ///
    /// It keeps the shipped file's SHAPE — four axes, `speed -> control` — because the coupling
    /// is what the effect column exists to show.
    fn a_gear_block() -> GearTuning {
        GearTuning {
            diminishing_exponent: 0.62,
            axes: [
                ("speed".to_string(), GearAxis { strength_weight: 1.42 }),
                ("control".to_string(), GearAxis { strength_weight: 0.90 }),
                ("power".to_string(), GearAxis { strength_weight: 1.30 }),
                ("endurance".to_string(), GearAxis { strength_weight: 0.86 }),
            ]
            .into_iter()
            .collect(),
            couplings: vec![
                GearCoupling { spends: "speed".into(), costs: "control".into(), drag: 0.55 },
                GearCoupling { spends: "power".into(), costs: "endurance".into(), drag: 0.50 },
            ],
        }
    }

    fn shut() -> Loadout {
        Loadout::default()
    }

    fn a_career() -> Career {
        Career {
            level: 59,
            xp: 80_618,
            xp_into_level: 2_480,
            xp_for_the_next_level: Some(3_640),
            skill_points: 58,
            gear_points: 122,
            gear_points_spent: 0,
            rank: "A".to_string(),
            last_sortie_xp: 340,
            levelled_up_to: None,
            cleared: BTreeSet::new(),
            gear: BTreeMap::new(),
        }
    }

    /// **An empty corner during the fight**, and that is the half a panel usually gets wrong.
    /// ⚠️ `n = 2`: both a career and no career, at every playing phase — a panel that hid only
    /// when there was nothing to say would still draw over a sortie. And the armoury is tried
    /// OPEN as well, because a loadout that could be left up into a sortie would be the same
    /// bug wearing a different hat.
    #[test]
    fn f120_nothing_is_drawn_while_a_sortie_is_being_flown() {
        let g = a_gear_block();
        let career = a_career();
        let open = Loadout { open: true, ..default() };
        for phase in [MissionPhase::Briefing, MissionPhase::Deploying, MissionPhase::Active] {
            assert_eq!(career_text(phase, Some(&career), &g, &shut()), None, "with a career at {phase:?}");
            assert_eq!(career_text(phase, None, &g, &shut()), None, "without one at {phase:?}");
            assert_eq!(career_text(phase, Some(&career), &g, &open), None, "armoury open at {phase:?}");
        }
    }

    /// The verdict is where the earnings appear — 3.0 s before the debrief phase, because
    /// `missions.ron: hub.verdict_s` holds the word over the field first.
    #[test]
    fn f120_the_earnings_are_up_from_the_verdict_and_not_only_at_the_debrief() {
        let g = a_gear_block();
        let career = a_career();
        for phase in [MissionPhase::Won, MissionPhase::Lost, MissionPhase::Debrief] {
            let said = career_text(phase, Some(&career), &g, &shut()).expect("a decided sortie reports");
            assert!(said.contains("+340 XP"), "{phase:?}: {said:?}");
            assert!(said.contains("RANK A"), "{phase:?}: {said:?}");
        }
    }

    /// The hub gets the standing and **not** the delta: nothing has just been earned there, and
    /// a `+340 XP` that stayed up through the next walk to the board would be a lie with a
    /// number in it. It also has to name the key, or the budget is a number with no verb.
    #[test]
    fn f121_the_hub_shows_the_standing_and_never_the_last_sortie() {
        let said = career_text(MissionPhase::Hub, Some(&a_career()), &a_gear_block(), &shut())
            .expect("the hub reports");
        assert!(said.contains("LEVEL 59"), "{said:?}");
        assert!(said.contains("RANK A"), "{said:?}");
        assert!(!said.contains("+340"), "the hub is not a debrief: {said:?}");
        assert!(said.contains("Tab"), "a budget with no way to spend it is the finding: {said:?}");
    }

    /// No player, no panel — never an invented zero.
    #[test]
    fn f120_a_run_with_nobody_in_it_draws_no_career() {
        let g = a_gear_block();
        for phase in [MissionPhase::Hub, MissionPhase::Debrief, MissionPhase::Won] {
            assert_eq!(career_text(phase, None, &g, &shut()), None, "at {phase:?}");
        }
    }

    /// 🔴 **The words are `progress::ledger`'s.** If this file ever grows its own copy of a
    /// line, this test is what notices: it compares the panel against the ledger rather than
    /// against a literal, so a second spelling cannot pass by agreeing with itself.
    #[test]
    fn f120_the_panel_quotes_the_ledger_and_does_not_respell_it() {
        let g = a_gear_block();
        let career = a_career();
        let said = career_text(MissionPhase::Debrief, Some(&career), &g, &shut()).expect("reports");
        for line in ledger::debrief_lines(&career) {
            assert!(said.contains(&line), "the panel dropped the ledger line {line:?}: {said:?}");
        }
        let hub = career_text(MissionPhase::Hub, Some(&career), &g, &shut()).expect("reports");
        assert!(hub.contains(&ledger::standing_line(&career)), "{hub:?}");
    }

    /// ★ **The armoury takes the panel over in the hub, and it lists every axis the file has.**
    #[test]
    fn f125_the_open_armoury_replaces_the_standing_and_lists_every_axis() {
        let g = a_gear_block();
        let career = a_career();
        let open = Loadout { open: true, ..default() };
        let said = career_text(MissionPhase::Hub, Some(&career), &g, &open).expect("the hub reports");
        assert!(said.contains(loadout::HEADING), "{said:?}");
        assert!(!said.contains(ledger::HEADING), "one panel, one thing to read: {said:?}");
        for (axis, _) in &g.axes {
            assert!(said.contains(axis.as_str()), "the armoury dropped {axis:?}: {said:?}");
        }
        assert!(said.contains(loadout::RESET_ROW), "no way to undo a spend: {said:?}");
        assert!(said.contains(loadout::CLOSE_ROW), "no way out of the armoury: {said:?}");
    }

    /// ★ **The cursor stands on exactly one row, and stepping moves it.**
    /// ⚠️ `n = 2` and the elements disagree: every row is tried, and the assert is that the row
    /// the cursor is on is the row the index names — not merely that *a* cursor exists.
    #[test]
    fn f125_the_cursor_marks_exactly_one_row_and_the_index_says_which() {
        let g = a_gear_block();
        let career = a_career();
        let list = loadout::rows(&g);
        for (i, row) in list.iter().enumerate() {
            let open = Loadout { open: true, at: i, ..default() };
            let said = career_text(MissionPhase::Hub, Some(&career), &g, &open).expect("reports");
            let marked: Vec<&str> =
                said.lines().filter(|l| l.starts_with(loadout::CURSOR)).collect();
            assert_eq!(marked.len(), 1, "row {i} — one cursor or the panel names two answers: {said:?}");
            let wanted = match row {
                loadout::Row::Axis(name) => name.clone(),
                loadout::Row::Reset => loadout::RESET_ROW.to_string(),
                loadout::Row::Close => loadout::CLOSE_ROW.to_string(),
            };
            assert!(
                marked[0].contains(&wanted),
                "the cursor stands on the wrong row for {row:?}: {marked:?}"
            );
        }
    }

    /// 🔴 **`F-122`'s whole sentence, on a screen: "speed costs control".**
    ///
    /// Six points into speed and the *control* row has to read **negative** — that is the
    /// coupling, and until this panel existed no player could ever see it. The control is the
    /// empty build in the same call: same career, same file, one field different.
    #[test]
    fn f122_the_coupling_is_visible_and_control_goes_negative_when_speed_is_bought() {
        let g = a_gear_block();
        let open = Loadout { open: true, ..default() };

        let mut empty = a_career();
        empty.gear = BTreeMap::new();
        let before = career_text(MissionPhase::Hub, Some(&empty), &g, &open).expect("reports");
        let control_before = before
            .lines()
            .find(|l| l.contains("control"))
            .expect("a control row")
            .to_string();
        assert!(
            control_before.contains("+0.00"),
            "an empty build drags nothing: {control_before:?}"
        );

        let mut fast = a_career();
        fast.gear = BTreeMap::from([("speed".to_string(), 6)]);
        fast.gear_points_spent = 6;
        let after = career_text(MissionPhase::Hub, Some(&fast), &g, &open).expect("reports");
        let control_after =
            after.lines().find(|l| l.contains("control")).expect("a control row");
        assert!(
            control_after.contains('-'),
            "six points of speed have to take control BELOW zero, or the trade-off the whole \
             budget is built on is invisible: {control_after:?}"
        );
        assert!(
            after.lines().any(|l| l.contains("speed") && l.contains('+')),
            "and speed itself has to have gone up: {after:?}"
        );
    }

    /// The budget line says what is left, which is the number the player is deciding with.
    #[test]
    fn f125_the_armoury_says_what_is_spent_and_what_is_left() {
        let g = a_gear_block();
        let mut career = a_career();
        career.gear = BTreeMap::from([("power".to_string(), 4)]);
        career.gear_points_spent = 4;
        let open = Loadout { open: true, ..default() };
        let said = career_text(MissionPhase::Hub, Some(&career), &g, &open).expect("reports");
        assert!(said.contains("4/122 spent"), "{said:?}");
        assert!(said.contains("118 left"), "{said:?}");
    }
}
