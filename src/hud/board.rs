//! **The mission board's own line** — what `F` does when you are standing at the signpost, and
//! the overview itself in the one kind of run that cannot draw a plate.
//!
//! > *„wenn man in der hub auf ein board drueckt (F) dann kommt man in eine mission uebersciht
//! > in der man auswaehlen kann was man machen will!"* — the user, 2026-08-27 (`Q-059`).
//!
//! ## Two states, and each of them is a **fact**, never a prediction
//!
//! | [`Board`] says | this element draws |
//! |---|---|
//! | out of range | nothing. `display: None`, an empty corner |
//! | in range, shut | two lines: the board's name and what `F` does — quoting `menu::pause::MISSION_SELECT_ROW`, never re-spelling it |
//! | open | the sortie list of `menu::lobby::entries`, `>` on the one that would fly |
//!
//! 🔴 **This element measures nothing itself.** Whether the player is at the board is
//! `menu::board::work_the_board`'s answer and is read out of [`Board`]; which sortie is chosen
//! is `menu::lobby::chosen`'s answer and is read out of [`LobbyChoice`]. That is the corollary
//! that cost this project a round on 2026-08-27 — *the HUD asked `deploy_on_contact`'s question
//! a second time, in a different schedule, from a different `Transform`, and drifted.* One
//! writer decides, everyone else reads the answer. There is no `Transform`, no distance and no
//! `missions.ron` list in this file that is not `entries()`'s.
//!
//! 🔴 **And it is not the retired `hud::hub_prompt`.** That element tried to name the door you
//! were *walking towards* — a bearing rule, a walk model and a ray — and was refuted four
//! times, most recently by a sweep of 2 361 960 stances that held the player's **height**
//! constant while the rule it tested was three-dimensional. Nothing here predicts anything: it
//! says what one key does while you stand in one circle, and the circle is a number in
//! `missions.ron`.
//!
//! ## Why it also draws the list, and why that is not a second mission list
//!
//! With a window the overview is `menu::Screen::Lobby` — the plate that already lists
//! `missions.ron` and already deploys. The moment the board opens it, [`Screen`] leaves
//! `Playing` and [`hide_while_a_menu_is_up`](super::hide_while_a_menu_is_up) hides **this whole
//! element** along with the rest of the HUD, so exactly one surface is ever on screen and no
//! rule needed an exception written for it.
//!
//! What is left is the run with no window at all, where `menu`'s entire `Update` chain is gated
//! off and `Screen` never moves (`menu::board`): `--headless` and `--offscreen`, which is every
//! run anybody has ever made on this machine. There the plate cannot exist, and this is the
//! same list — from the same [`entries`], highlighted by the same [`chosen`], deployed by the
//! same `DeployRequest` — drawn where the plate cannot be. That is one list with two surfaces,
//! not two lists.
//!
//! ## The evidence
//!
//! | what | run → picture |
//! |---|---|
//! | the prompt, standing at the signpost with the board shut | `scripts/f177-board.txt --ticks 120` → `docs/images/f177-board-prompt.png` |
//! | the overview open, `>` on the sortie that would fly | `scripts/f177-board.txt --ticks 154` → `docs/images/f177-board-open.png` |
//! | **one more `F`, and the cursor has moved one row** | `… --ticks 179` → `docs/images/f177-board-chosen.png` |
//! | the same run 37 ticks earlier, at the spawn point — **the control** | `… --ticks 92` → `docs/images/f177-board-control.png` |
//!
//! Decoded against that control rather than assumed. In the panel's own rect
//! (`x 51..460, y 187..648` of a 1280 × 720 frame) the control holds **0** pixels of the amber
//! out of `maps.ron` (sRGB 255, 215, 89) — and 0 in the whole frame — while the prompt holds
//! **372** in `x 52..220, y 191..239` and the open board **2 457** in `x 52..346, y 191..511`.
//! The open frame resolves into **23 text bands**, of which 13 are sortie rows — exactly the 13
//! entries `missions.ron` offers — and in the cursor column (`x 45..70`) there are three bands
//! and no more: the heading, the footer, and **one** row. Between `--ticks 154` and `--ticks
//! 179` — one press of `F` — that one row moves from `y 251..258` to `y 270..277`, one line
//! pitch down, and nothing else in the column moves. The right edge of the whole panel is
//! `x 346` against a keep-out box that starts at `x 512`. The shot is bit-identical over two
//! runs (`sha256 55132df4…`).
//!
//! ## The ladder, since 2026-09-01
//!
//! Each row can now carry a marker: [`CLEARED`] for a rung this career has won, `LOCKED <rank>`
//! for one `progress.ron: gates` refuses it. **[`Career::cleared`] is `Profile::cleared`'s first
//! reader** — it had been written on every won sortie since 2026-08-19 and read by nothing
//! (`docs/FINDINGS.md` FIND-222). Picture: `docs/images/f121-ladder-cleared.png`
//! (`scripts/f120-progress.txt --ticks 1155`), one `*` on `skirmish recruit`, which is the
//! rung that run had just won, and no `LOCKED` anywhere because the shipped `gates` is empty
//! (`docs/QUESTIONS.md` Q-090).
//!
//! ⚠️ **The decode above predates the markers** and was measured with an empty career. The
//! panel is wider now — `tests/hud.rs::a_career_that_has_cleared_everything` is what keeps the
//! keep-out claim honest against the widest state rather than the narrowest.

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::{GameData, Progress};
use crate::hud::{signal, HudElement};
use crate::menu::board::Board;
use crate::menu::lobby::{chosen, entries, LobbyChoice};
use crate::menu::pause::MISSION_SELECT_ROW;
use crate::progress::{career, Career};
use crate::shared::LocalPlayer;

/// Marker on the board panel.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BoardPanel;

/// Left edge, in percent of the screen.
pub const LEFT_PCT: f32 = 4.0;
/// Top edge. Below the gas bar's own band and well clear of the objective line.
pub const TOP_PCT: f32 = 26.0;
/// How wide. **`LEFT_PCT + WIDTH_PCT` has to stay under
/// [`KEEP_OUT_LOW_PCT`](super::KEEP_OUT_LOW_PCT)** — 4 + 32 = 36 against a box that starts at
/// 40, i.e. 4 % of the width in hand. `tests/hud.rs` measures the real rect and does not take
/// this arithmetic's word for it.
pub const WIDTH_PCT: f32 = 32.0;
/// One line, in logical pixels. The same size as the objective counter: information you read
/// when you look for it, not a verdict.
pub const LINE_PX: f32 = 16.0;

/// The marker put in front of the sortie that would fly. **A glyph and not a colour**, so the
/// element survives `docs/conventions.md`'s colour-blindness rule the way the crosshair does —
/// and so a screenshot can be decoded without trusting a hue.
pub const CURSOR: &str = "> ";
/// What every other row is indented by, so the cursor column is a column.
pub const NO_CURSOR: &str = "  ";

/// The board's own name on the panel's first line.
pub const HEADING: &str = "MISSION BOARD";

/// What marks a rung the player has **won** at least once.
///
/// A glyph and not a colour, the same rule the cursor follows — and a glyph rather than the
/// word "cleared" because it has to sit at the end of thirteen rows without making the widest
/// of them wider than the panel.
pub const CLEARED: &str = "  *";

/// What marks a rung `progress.ron: gates` will not let this rank fly.
pub const LOCKED: &str = "  LOCKED";

/// **What the panel says.** `None` means draw nothing at all.
///
/// Pure, and it takes the two answers rather than the world: everything it needs has already
/// been decided by one writer each (`menu::board::Board`, `menu::lobby::chosen`), and this
/// function's whole job is to turn them into lines. That is what makes "what does the screen
/// say" testable without an app, a window, a player and a hub.
///
/// ⚠️ `list` is `menu::lobby::entries`'s output and nothing else. A list built here would be
/// the second implementation of "what sorties are there", and two of those drift.
pub fn board_text(
    in_range: bool,
    open: bool,
    list: &[(String, Option<String>)],
    picked: Option<&(String, Option<String>)>,
    progress: &Progress,
    standing: Option<&Career>,
) -> Option<String> {
    if !in_range {
        return None;
    }
    if !open {
        // What the key does, in the words the plate uses for the same door. `F` is a fact
        // about the circle he is standing in, not a guess about where he is going.
        return Some(format!("{HEADING}\n\nF   {MISSION_SELECT_ROW}"));
    }
    if list.is_empty() {
        // Loud rather than an empty panel that looks like the board is broken.
        return Some(format!("{HEADING}\n\nassets/data/missions.ron has no templates"));
    }
    let mut out = String::from(HEADING);
    out.push('\n');
    for entry in list {
        out.push('\n');
        out.push_str(if Some(entry) == picked { CURSOR } else { NO_CURSOR });
        out.push_str(&row(entry));
        out.push_str(&standing_on(entry, progress, standing));
    }
    out.push_str("\n\nF   next        hold F   deploy");
    Some(out)
}

/// **Where this career stands on this rung** — the difficulty ladder, said out loud.
///
/// Three answers and they are exclusive: a door this rank may not open is `LOCKED` and says
/// which rank it wants; a door already won carries [`CLEARED`]; everything else says nothing,
/// because a row that reads "not yet cleared" on eleven of thirteen lines is noise.
///
/// 🔴 **The key is `save::SortieOutcome::cleared_key`'s and it is not rebuilt here.** That
/// function decides what `("skirmish", Some("veteran"))` is called in the save file
/// (`"skirmish/veteran"`, and a bare `"tutorial"` for a template with no tier), and a second
/// spelling of it in this file would be two implementations of one question — the drift rule 5's
/// corollary is about. The one thing this file does is put the same two strings together the
/// same way, which is why `tests/hud.rs` compares it against a real `Profile`'s own set rather
/// than against a literal.
fn standing_on(
    entry: &(String, Option<String>),
    progress: &Progress,
    standing: Option<&Career>,
) -> String {
    let Some(career) = standing else { return String::new() };
    let key = match &entry.1 {
        Some(level) => format!("{}/{level}", entry.0),
        None => entry.0.clone(),
    };
    // Locked beats cleared: a rank that has fallen (it cannot today — the rank is derived from
    // XP, which never goes down) would otherwise show a door as both won and shut.
    if !career::may_fly(progress, &career.rank, &key) {
        return match progress.gates.get(&key) {
            Some(rank) => format!("{LOCKED} {rank}"),
            None => LOCKED.to_string(),
        };
    }
    if career.cleared.contains(&key) {
        return CLEARED.to_string();
    }
    String::new()
}

/// One sortie, as one line: the template key and the difficulty key out of `missions.ron`.
///
/// **The keys and not the display names**, deliberately: the keys are what
/// `menu::lobby::LobbyChoice` holds, what the log line prints and what a script's reader has to
/// match up — and `tests/hud.rs` reads them straight out of `GameData`, so an invented string
/// here has nowhere to hide.
fn row(entry: &(String, Option<String>)) -> String {
    match &entry.1 {
        Some(level) => format!("{}  {level}", entry.0),
        // The tutorial: no levels, the template's own numbers (`mission::run::resolve`).
        None => entry.0.clone(),
    }
}

/// One `Text`, hidden until the player is at the board.
pub fn spawn_board_panel(mut commands: Commands, data: Res<GameData>) {
    // Amber: `docs/conventions.md` §3 reserves it for "cortex, weak points, **objectives**",
    // and the deployment pads are painted with the same key for the same reason — a door out
    // of the hub is the hub's objective. Read from `maps.ron`, never a literal here.
    let amber = signal(&data, "amber");
    commands.spawn((
        Name::new("hud_board_panel"),
        BoardPanel,
        HudElement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(LINE_PX), ..default() },
        TextLayout::justify(Justify::Left),
        TextColor(amber),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(TOP_PCT),
            left: Val::Percent(LEFT_PCT),
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
pub fn update_board_panel(
    board: Res<Board>,
    data: Res<GameData>,
    choice: Res<LobbyChoice>,
    careers: Query<&Career, With<LocalPlayer>>,
    mut panels: Query<(&mut Text, &mut Node), With<BoardPanel>>,
) {
    // Out of range is the overwhelmingly common case — the player is in a sortie, or 200 m up
    // over the district — and it costs nothing to answer before touching `missions.ron`.
    let text = if board.in_range {
        let list = entries(&data);
        let picked = chosen(&data, &choice);
        board_text(
            board.in_range,
            board.open,
            &list,
            picked.as_ref(),
            &data.progress,
            careers.iter().next(),
        )
    } else {
        None
    };

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

    /// **The shipped `progress.ron`**, read the way `data::GameData::load` reads it — not a
    /// hand-built stand-in. The ladder's whole point is that it reflects the file: a gate added
    /// to `gates: {}` has to appear on the board without a line of Rust moving, and a fixture
    /// with its own ranks would prove the opposite of that.
    fn progress() -> Progress {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data");
        crate::data::GameData::load(&dir).progress
    }

    fn list() -> Vec<(String, Option<String>)> {
        vec![
            ("tutorial".into(), None),
            ("skirmish".into(), Some("recruit".into())),
            ("skirmish".into(), Some("veteran".into())),
        ]
    }

    /// A career at a named rank with a named set of clears. Everything else is a placeholder —
    /// this file draws two of `Career`'s fields and no others.
    fn career_at(rank: &str, cleared: &[&str]) -> Career {
        Career {
            level: 1,
            xp: 0,
            xp_into_level: 0,
            xp_for_the_next_level: Some(300),
            skill_points: 0,
            gear_points: 6,
            gear_points_spent: 0,
            cleared: cleared.iter().map(|s| s.to_string()).collect::<BTreeSet<String>>(),
            gear: BTreeMap::new(),
            rank: rank.to_string(),
            last_sortie_xp: 0,
            levelled_up_to: None,
        }
    }

    /// The case that keeps the element honest: **an empty corner, not an invented prompt.**
    #[test]
    fn f177_a_player_who_is_not_at_the_board_is_told_nothing() {
        let (l, p) = (list(), progress());
        for open in [false, true] {
            assert_eq!(
                board_text(false, open, &l, Some(&l[1]), &p, None),
                None,
                "out of range with open = {open}"
            );
        }
    }

    /// The prompt quotes the plate's own row and never re-spells it — `FIND-178` is what
    /// happens when a HUD carries its own copy of a label.
    #[test]
    fn f177_the_prompt_names_the_key_and_quotes_the_row_it_opens() {
        let (l, p) = (list(), progress());
        let said = board_text(true, false, &l, Some(&l[1]), &p, None).expect("in range, shut");
        assert!(said.contains(MISSION_SELECT_ROW), "the prompt has to name the row: {said:?}");
        assert!(said.contains('F'), "the prompt has to name the key: {said:?}");
        // Shut means shut: no sortie is offered before the board has been opened.
        assert!(!said.contains("veteran"), "a shut board may not list sorties: {said:?}");
    }

    /// The cursor stands on the entry that would fly, and on exactly one of them.
    #[test]
    fn f177_the_open_board_marks_exactly_the_sortie_that_would_fly() {
        let (l, p) = (list(), progress());
        for (i, entry) in l.iter().enumerate() {
            let said =
                board_text(true, true, &l, Some(entry), &p, None).expect("in range, open");
            assert_eq!(
                said.matches(CURSOR).count(),
                1,
                "entry {i} — one cursor, or the panel names two answers: {said:?}"
            );
            let marked: Vec<&str> =
                said.lines().filter(|line| line.starts_with(CURSOR)).collect();
            assert_eq!(marked.len(), 1);
            assert!(
                marked[0].contains(&entry.0),
                "the cursor stands on the wrong row for {entry:?}: {marked:?}"
            );
        }
    }

    /// A panel with nothing to show says so. An empty list drawn as an empty panel is the
    /// "bar that is a picture of a bar" with no pixels.
    #[test]
    fn f177_a_file_with_no_templates_is_said_out_loud() {
        let said =
            board_text(true, true, &[], None, &progress(), None).expect("in range, open");
        assert!(said.contains("missions.ron"), "{said:?}");
    }

    /// ★★ 🔴 **THE DIFFICULTY LADDER, and the control is the same call with no career.**
    ///
    /// `Profile::cleared` had been written on every won sortie since 2026-08-19 and read by
    /// **nothing** — 419 sorties into a set nobody opened (`docs/FINDINGS.md` FIND-222). This
    /// is the reader.
    ///
    /// ⚠️ The `n = 2` case with the elements DISAGREEING: two skirmish rungs, one cleared and
    /// one not, in the same panel. A marker that simply appeared on every row, or on none,
    /// passes a one-row test and fails this one.
    #[test]
    fn f121_a_cleared_rung_is_marked_and_an_unflown_one_beside_it_is_not() {
        let (l, p) = (list(), progress());
        let career = career_at("E", &["skirmish/recruit"]);
        let said = board_text(true, true, &l, Some(&l[1]), &p, Some(&career)).expect("open");

        let recruit = said.lines().find(|x| x.contains("recruit")).expect("a recruit row");
        let veteran = said.lines().find(|x| x.contains("veteran")).expect("a veteran row");
        assert!(recruit.contains(CLEARED), "the won rung is not marked: {recruit:?}");
        assert!(!veteran.contains(CLEARED), "an unflown rung is marked: {veteran:?}");
        assert_eq!(
            said.matches(CLEARED).count(),
            1,
            "one clear, one marker — anything else and the ladder is decoration: {said:?}"
        );

        // The control: the identical call with no career shows no ladder at all, which is what
        // every run made before this change looked like.
        let blind = board_text(true, true, &l, Some(&l[1]), &p, None).expect("open");
        assert!(!blind.contains(CLEARED), "a run with no career invents a clear: {blind:?}");
    }

    /// ★ **A gate that is set is a gate the board shows** — and it names the rank it wants,
    /// because "LOCKED" with no reason is a wall a player cannot plan against.
    ///
    /// The gate is injected into the **loaded** file rather than into a stand-in: this is the
    /// exact call `update_board_panel` makes, so the day somebody fills `gates: {}` the panel
    /// already says so.
    #[test]
    fn f121_a_gated_rung_names_the_rank_it_wants_and_the_open_ones_stay_quiet() {
        let mut p = progress();
        p.gates.insert("skirmish/veteran".to_string(), "C".to_string());
        let l = list();
        let career = career_at("E", &["skirmish/recruit"]);
        let said = board_text(true, true, &l, Some(&l[1]), &p, Some(&career)).expect("open");

        let veteran = said.lines().find(|x| x.contains("veteran")).expect("a veteran row");
        assert!(veteran.contains(LOCKED), "a gated rung is not marked shut: {veteran:?}");
        assert!(veteran.contains('C'), "and it has to name the rank it wants: {veteran:?}");
        assert_eq!(said.matches(LOCKED).count(), 1, "only the gated rung is shut: {said:?}");

        // And a career that HAS the rank walks through the same door.
        let ranked = career_at("C", &[]);
        let open = board_text(true, true, &l, l.get(1), &p, Some(&ranked)).expect("open");
        assert!(!open.contains(LOCKED), "rank C is refused its own gate: {open:?}");
    }

    /// 🔴 **The shipped file locks nothing, and that is a decision worth a guard.**
    /// `progress.ron: gates` is empty on purpose (`docs/QUESTIONS.md` Q-051, Q-090). If a round
    /// ever fills it, this test goes red and whoever filled it has to say so out loud.
    #[test]
    fn f121_the_shipped_ladder_takes_no_playable_content_away() {
        let p = progress();
        let l = list();
        let beginner = career_at("E", &[]);
        let said = board_text(true, true, &l, Some(&l[0]), &p, Some(&beginner)).expect("open");
        assert!(
            !said.contains(LOCKED),
            "the shipped progress.ron has started locking doors — that is a design change and \
             it needs a line in docs/QUESTIONS.md, not a green test: {said:?}"
        );
    }
}
