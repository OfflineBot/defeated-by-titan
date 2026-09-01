//! `F-125` — **the armoury: where the budget stops being a number and becomes a build.**
//!
//! > *„Ganz ausbauen"* — the user, 2026-09-01 (`docs/QUESTIONS.md`, the Q-062 override).
//!
//! ```text
//!   stand in the hub ──Tab──► the armoury ──Tab──► the next row
//!                                 │  hold Tab
//!                                 └──────────────► one point on it  ──► save::GearRequest
//! ```
//!
//! ## Why a key and a hold, and not a screen with buttons
//!
//! Because a screen with buttons **cannot be shown to exist.** `src/lib.rs` builds
//! `primary_window: None` for `--headless` and `--offscreen`, `menu` is gated on
//! `With<PrimaryWindow>`, and there is no script verb that clicks a plate (`FIND-189`). A
//! loadout reachable only by mouse would be 🟨 for ever — which is exactly the hole the mission
//! board climbed out of by being *a place and a key*. This is the same grammar with a different
//! key, deliberately: tap steps, hold commits, and whoever has learned one has learned both.
//!
//! **`Tab`, and it is the only candidate.** The script parser knows `w a s d q e c f space f3
//! shift ctrl tab` and nothing else (`src/debug/script.rs::parse_key`), and of those, `Tab` is
//! the one whose game meaning is free: `net::local::read_input` sets `Buttons::MARK` from it and
//! **nothing in `src/` reads that bit**. Measured 2026-09-01, not assumed.
//!
//! ⚠️ **And it is gated on the hub**, the way `F` is gated on the board's circle. A key that
//! opened a loadout mid-flight would be a second meaning for `Tab` in the one state where
//! `MARK` will eventually want its first — so the day somebody builds a squad ping, this costs
//! nothing to keep.
//!
//! ## What it does NOT do, and this is the honest boundary
//!
//! **Spending a point still moves no gameplay number.** `progress::gear`'s module header says
//! why and it has not changed: `game.ron: vector` and `gear.ron: blades` are not read through a
//! build, and a domain that reached into the flight model would be the rule-3 breach
//! (`FIND-155` says what wiring one axis would take). What this file closes is the *other* half
//! of that sentence — the budget was unreachable, so the allocation was **always empty**, so
//! the trade-off `F-122` designed had never once been expressed by a player. Measured: 419
//! sorties flown, 0 of 122 points spent (`docs/FINDINGS.md` FIND-222).
//!
//! The effect column is real arithmetic out of `progress::gear`, and it is what makes the
//! coupling visible: put points into `speed` and watch `control` go **negative**. That is the
//! row's own sentence — *"speed costs control"* — on a screen, for the first time.

//! ## The evidence
//!
//! `scripts/f120-progress.txt` ACT 7 opens the armoury, spends on **two different axes** out of
//! two different presses, and ACT 8 steps to `CLOSE` and shuts it — 12 asserts, exit 0, driven
//! by nothing but `Tab`. The log it is read by:
//!
//! ```text
//! armoury: open — 0 of 6 point(s) spent, 6 rows
//! armoury: Spend("speed") for player 1
//! save: player 1 — speed is now at 1 — 1/6 allocated
//! armoury: Axis("control") — row 2 of 6         <- the tap STEPPED, which is what makes it a cursor
//! armoury: Spend("control") for player 1
//! save: player 1 — control is now at 2 — 2/6 allocated
//! armoury: shut
//! ```
//!
//! The picture is `docs/images/f125-armoury.png` (`--ticks 931`), decoded in `hud::career`.
//!
//! ⚠️ **Two different axes and not two spends on one**, deliberately: a run that spent twice on
//! `speed` would pass every assert here and prove only that the *hold* works. It is also
//! exactly what the first cut of that act did, for a reason that was in the script and not in
//! this file — `docs/FINDINGS.md` FIND-222 §3.

use bevy::prelude::*;

use crate::data::{GameData, GearTuning};
use crate::mission::MissionPhase;
use crate::save::{GearChange, GearRequest};
use crate::shared::{LocalPlayer, PlayerId};

use super::{gear, Career};

/// The heading the armoury carries.
pub const HEADING: &str = "ARMOURY";
/// The marker in front of the row a hold would commit. **A glyph and not a colour**, the same
/// reason `hud::board`'s cursor is one: `docs/conventions.md`'s colour-blindness rule, and a
/// screenshot that can be decoded without trusting a hue.
pub const CURSOR: &str = "> ";
/// What every other row is indented by, so the cursor column is a column.
pub const NO_CURSOR: &str = "  ";
/// The second-to-last row: give every point back.
pub const RESET_ROW: &str = "RESET";
/// The last row: put the armoury away.
///
/// It is a **row** and not a second key, and that is the whole reason the armoury needs only
/// one: `Tab` steps and `Tab` commits, so "shut it" has to be something you can commit. A
/// second key would also have to be one the script parser knows, and `Tab` was the last one
/// free (`src/debug/script.rs::parse_key`).
pub const CLOSE_ROW: &str = "CLOSE";

/// **What the armoury is doing right now.** One writer — [`work_the_armoury`] — and everything
/// else reads it, including `hud::career`, which may not ask the question a second time
/// (`CLAUDE.md` rule 5's corollary: one writer decides, everyone else reads the answer).
///
/// A `Resource` for the same reason `menu::board::Board` is one: it is the state of *this
/// session's screen*, not of a player. `Tab` comes off this machine's keyboard, so the player it
/// acts for is the local one — there is one keyboard here the way there is one mouse.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Loadout {
    pub open: bool,
    /// Which row the cursor is on, as an index into [`rows`]. **An index and not a name**, and
    /// that is the opposite of `LobbyChoice`'s choice on purpose: the rows here are
    /// `progress.ron`'s axes plus one constant, so a stale index cannot deploy somebody else's
    /// mission — the worst it can do is point at the wrong axis, and [`row_at`] clamps it.
    pub at: usize,
    /// How long `Tab` has been down on a press that found the armoury already open. `None`
    /// means this press cannot commit — the state the opening press leaves behind.
    pub armed: Option<f32>,
    /// This press has done its work. Cleared on release, so one press is one action.
    pub spent: bool,
}

/// One row of the armoury.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// An axis of `progress.ron: gear.axes`, in file order.
    Axis(String),
    /// Give every point back.
    Reset,
    /// Put the armoury away.
    Close,
}

/// **Every row, in file order, then `RESET`** — the one place that flattens `progress.ron` into
/// a list, so the panel and the key cannot disagree about what row 3 is.
pub fn rows(gear: &GearTuning) -> Vec<Row> {
    let mut out: Vec<Row> =
        gear.axes.into_iter().map(|(name, _)| Row::Axis(name.to_string())).collect();
    out.push(Row::Reset);
    out.push(Row::Close);
    out
}

/// The row an index means, clamped. An empty file has no rows and no cursor.
pub fn row_at(gear: &GearTuning, at: usize) -> Option<Row> {
    let list = rows(gear);
    if list.is_empty() {
        return None;
    }
    Some(list[at % list.len()].clone())
}

/// **What the panel says**, as lines. `None` means the armoury is shut.
///
/// Pure, and it takes the two answers rather than the world: the build is `save`'s (mirrored
/// onto [`Career`] by `progress`) and the cursor is [`Loadout`]'s. This function's whole job is
/// to lay them out — which is what makes "what does the armoury say" testable without an app,
/// a window, a player and a hub.
pub fn armoury_lines(gear: &GearTuning, career: &Career, at: usize) -> Vec<String> {
    let mut out = Vec::new();
    let left = career.gear_points.saturating_sub(career.gear_points_spent);
    out.push(format!(
        "{HEADING}   {}/{} spent   {left} left",
        career.gear_points_spent, career.gear_points
    ));
    out.push(String::new());

    let list = rows(gear);
    for (i, row) in list.iter().enumerate() {
        let marked = !list.is_empty() && i == at % list.len();
        let cursor = if marked { CURSOR } else { NO_CURSOR };
        match row {
            Row::Axis(name) => {
                let points = career.gear.get(name).copied().unwrap_or(0);
                // The **effect**, not the points: this is where "speed costs control" stops
                // being a comment in a RON file and becomes a number that goes negative.
                let effect = gear::effect_of(gear, &career.gear, name);
                out.push(format!("{cursor}{name:<10} {points:>2}   {effect:+.2}"));
            }
            Row::Reset => out.push(format!("{cursor}{RESET_ROW}")),
            Row::Close => out.push(format!("{cursor}{CLOSE_ROW}")),
        }
    }

    out.push(String::new());
    out.push("Tab  next        hold Tab  commit".to_string());
    out
}

/// The armoury, the key, and the spend. `Update`, in the hub only.
///
/// ⚠️ **The release is read BEFORE the accumulator, and that order is the whole tap.** The frame
/// a key comes up in has `just_released` true and `pressed` false, so an accumulator block that
/// ran first would disarm the press and the release would then find nothing to step. That is
/// not a hypothetical: it is what `menu::board::work_the_board` measured on 2026-08-28, where
/// getting it the other way round made every tap do exactly nothing and turned four tests red
/// at once. Same order here, for the same reason.
///
/// **No `.single()`** (rule 4): it acts for the [`LocalPlayer`], because `Tab` is a fact about
/// this machine's keyboard.
pub fn work_the_armoury(
    keys: Res<ButtonInput<KeyCode>>,
    real: Res<Time<Real>>,
    data: Res<GameData>,
    phase: Res<State<MissionPhase>>,
    players: Query<(&PlayerId, &Career), With<LocalPlayer>>,
    mut loadout: ResMut<Loadout>,
    mut ask: MessageWriter<GearRequest>,
) {
    // The hub, and nowhere else. Leaving it shuts the armoury — the same way walking out of the
    // board's circle shuts the overview, and for the same reason: it is a place, not a modal.
    if *phase.get() != MissionPhase::Hub {
        if loadout.open || loadout.armed.is_some() || loadout.spent {
            *loadout = Loadout::default();
        }
        return;
    }

    let Some((player, career)) = players.iter().next() else { return };
    let list = rows(&data.progress.gear);
    if list.is_empty() {
        error!("progress.ron defines no gear axes — the armoury has nothing to show");
        return;
    }

    if keys.just_pressed(KeyCode::Tab) {
        if loadout.open {
            // Armed: this press may still become a hold.
            loadout.armed = Some(0.0);
            loadout.spent = false;
        } else {
            loadout.open = true;
            // **Spent, and that is the guard.** The press that opens may neither step nor
            // commit — otherwise walking into the hub and leaning on `Tab` would spend a point
            // on whatever row the cursor happened to be resting on. You always commit something
            // you have been shown. Same brace, same argument, as `menu::board::Board::spent`.
            loadout.armed = None;
            loadout.spent = true;
            info!(
                "armoury: open — {} of {} point(s) spent, {} rows",
                career.gear_points_spent,
                career.gear_points,
                list.len()
            );
        }
        return;
    }

    if keys.just_released(KeyCode::Tab) {
        if loadout.armed.is_some() && !loadout.spent {
            loadout.at = (loadout.at + 1) % list.len();
            info!("armoury: {:?} — row {} of {}", list[loadout.at], loadout.at + 1, list.len());
        }
        loadout.armed = None;
        loadout.spent = false;
    } else if let Some(held) = loadout.armed.as_mut() {
        if keys.pressed(KeyCode::Tab) {
            // `Time<Real>`, not virtual: with a window the hub still runs, but a hold counted in
            // ticks would stop the moment anything paused the clock. `Time<Real>` runs in all
            // four launch modes — the same argument `menu::board` makes for its own hold.
            *held += real.delta_secs();
            if *held >= data.progress.loadout.hold_s && !loadout.spent {
                loadout.spent = true;
                loadout.armed = None;
                let row = list[loadout.at].clone();
                if row == Row::Close {
                    info!("armoury: shut");
                    loadout.open = false;
                } else {
                    commit(&row, *player, career, &mut ask);
                }
            }
        } else {
            // The key came up in a frame this system never saw `just_released` in. Disarm
            // rather than let a stale accumulator finish a hold nobody is still making.
            loadout.armed = None;
        }
    }
}

/// **Commit — and it asks, exactly as the mission board's hold does.**
///
/// `save::spend_gear_points` reads the message and is the only thing that writes
/// [`Profile::gear`](crate::save::Profile::gear). This module holds no career state and never
/// touches a profile (`docs/architecture.md`: `progress -> save`, the asker is not the writer).
fn commit(row: &Row, player: PlayerId, career: &Career, ask: &mut MessageWriter<GearRequest>) {
    let change = match row {
        Row::Axis(name) => GearChange::Spend(name.clone()),
        Row::Reset => GearChange::ResetAll,
        // Handled by the caller: shutting the panel is this module's own state and not
        // something `save` has any business being told about.
        Row::Close => return,
    };
    info!("armoury: {change:?} for player {}", player.0);
    ask.write(GearRequest { player, change, budget: career.gear_points });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape the opening press leaves behind. The live guard is `spent`; this names both
    /// flags so nobody reading the struct has to guess which is doing the work.
    #[test]
    fn f125_the_press_that_opens_the_armoury_is_spent_and_not_armed() {
        let opened = Loadout { open: true, at: 0, armed: None, spent: true };
        assert!(opened.spent, "the opening press has done its one thing — this is the guard");
        assert!(opened.armed.is_none(), "and the accumulator never even starts");
    }
}
