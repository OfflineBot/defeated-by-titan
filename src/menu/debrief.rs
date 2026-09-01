//! The debrief — **the end of the loop**, and the screen that was missing from it.
//!
//! > *„erstelle die ganze gameloop mit lobby und main menu etc."* — the user, 2026-08-23.
//!
//! Until 2026-08-24 a sortie ended like this: `mission::announce` wrote one line into the log,
//! the word `WON` stood over the field for three seconds, and the hub took the player back. He
//! was never told anything. **A loop that cannot report is not a loop** — it is the same run
//! twice with a pause in the middle.
//!
//! ```text
//!   Active ──┬─► Won ──┐                              ┌─ Redeploy ─► the same sortie again
//!            └─► Lost ─┴─ hub.verdict_s ─► DEBRIEF ───┤
//!                                          (here)     └─ To the lobby / Esc ─► Screen::Lobby
//! ```
//!
//! ## Why this is a `Screen` **and** a `MissionPhase`, and neither one alone
//!
//! [`MissionPhase::Debrief`](crate::mission::MissionPhase::Debrief) is the **state of the
//! session**: the sortie is over, its entity is still standing with the clock and the counter
//! on it, and no new one may start except through the hub. A `--headless` run is in it too, and
//! that is the whole reason it is a phase — it has a number (`6`), so
//! `scripts/f175-loop.txt` can say `assert phase == 6` and the last station of the loop is
//! evidence instead of a claim. A run with no window has no menu at all (`menu`, module docs),
//! so a debrief that were only a screen could never be seen by anything on this machine.
//!
//! [`Screen::Debrief`](super::Screen::Debrief) is what the player **reads**. It is a plate over
//! a stopped world, exactly like the title, and stopping the world is the point: every screen
//! that is not `Playing` pauses `Time<Virtual>` (`menu::apply_screen`), a paused clock runs no
//! `FixedUpdate`, and `mission::hub::walk_the_way_home` — which would otherwise take the player
//! to the hub after `missions.ron: hub.debrief_s` — is a `FixedUpdate` system. **So the number
//! in the file is only ever spent by a run that has nobody looking at it**, and a player reads
//! the report for as long as he likes. One mechanism, no `has a window` branch anywhere in
//! `mission`.
//!
//! ## What is on it, and where every number comes from
//!
//! The mission entity, which is still alive: `hub::open_hub` despawns it on the way into the
//! hub and not a tick earlier, precisely so this plate and `mission::report` have something to
//! read. [`Verdict`] carries the word — `WON`/`LOST` out of `MissionPhase::label`, because the
//! phase itself now says `DEBRIEF` and a second spelling of the verdict is how a game ends up
//! telling the player two different things about the same sortie.
//!
//! ## The ledger: what the sortie earned
//!
//! Levels, XP and gear rank are `progress`' and this screen computes none of them. [`Career`] is
//! read, [`progress::ledger`](crate::progress::ledger) turns it into lines, and
//! [`fill_the_ledger`] hangs them under [`DebriefLedger`] between the sortie's numbers and the
//! two buttons. **The same lines are drawn by `hud::career`** in the runs that have no window
//! and therefore no plate at all — one list, two surfaces, exactly as `hud::board` draws
//! `menu::lobby::entries`.

use bevy::prelude::*;

use super::{plate, PauseElement, Screen};
use crate::data::GameData;
use crate::mission::{KillTally, Mission, MissionClock, Verdict};
use crate::progress::{ledger, Career};
use crate::shared::{AbandonSortie, DeployRequest, LocalPlayer};

/// What a button on the debrief does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebriefAction {
    /// **The same sortie again.** The template and the difficulty of the one that just ended —
    /// a `DeployRequest`, the identical message the lobby's *Deploy* row writes, so `mission`
    /// routes it through the hub and no second mechanism is built for "one more time".
    Redeploy,
    /// Back to the mission list, which is where a loop starts over.
    Lobby,
}

/// **The place `progress` draws into** — filled since 2026-09-01, and this is the record of
/// what it cost.
///
/// It was spawned as a named, documented, **empty** node on 2026-08-24 with three preconditions
/// written on it. All three are now met, in exactly the shape it asked for:
///
/// 1. **a read that is not a `&mut` of anybody's field** — [`Career`], a component `progress`
///    already recomputes on `Changed<Profile>`. Read-only here, and nothing in `menu` derives a
///    level, a threshold or a rank. `save` stays the one writer of `Profile` and `progress`
///    stays the one writer of `Career`;
/// 2. **one line on the allow list** — `menu -> progress` in `docs/architecture.md`, carrying
///    the same argument `menu -> mission` does: a screen has to be right in the frame it is
///    drawn in, so it needs the STATE and not a message that fired three ticks ago;
/// 3. **`plate::note`s as children of this node**, between the sortie's numbers and the two
///    buttons — [`fill_the_ledger`], and no other line of `spawn_debrief_screen` moved.
///
/// ⚠️ **The words are `progress::ledger`'s and not this file's.** `hud::career` draws the same
/// list where a plate cannot exist (`--headless`/`--offscreen` build no window, `FIND-189`), and
/// two screens formatting one career is the drift rule 5's corollary is about. This file chooses
/// where the rows go; it does not choose what they say.
///
/// **What it was worth, measured** (`docs/FINDINGS.md` FIND-222): the shipped save had flown
/// **419 sorties** and spent **0 of 122** gear points. The numbers had been right the whole
/// time — `Career::last_sortie_xp` was correct in the very run that proved the screen was
/// silent — and there had never been a surface that said them.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DebriefLedger;

/// **Puts the career under the marker**, once per plate.
///
/// The filter is `Without<Children>` and not `Added<DebriefLedger>`, and that is not a
/// preference: the plate is built the frame [`Screen`] becomes `Debrief`, and there is no rule
/// that says `progress::refresh_careers` has run its `Changed<Profile>` pass by then. `Added`
/// fires exactly once and would lose the race silently — an empty column under a heading, which
/// is the one thing this node's own doc says it must never be. This retries until there is
/// something to draw and then stops, because a filled node has children.
///
/// It costs nothing to run every frame: outside a debrief the archetype it matches is empty
/// (`CLAUDE.md` rule 6), and the query is two components deep.
///
/// **No `.single()`** (rule 4). The career drawn is the **local** player's — a plate is this
/// machine's surface, the same reason `menu::board::work_the_board` filters `LocalPlayer` for a
/// key off this machine's keyboard. Twenty people fly one sortie and each reads his own report.
pub fn fill_the_ledger(
    mut commands: Commands,
    careers: Query<&Career, With<LocalPlayer>>,
    ledgers: Query<Entity, (With<DebriefLedger>, Without<Children>)>,
) {
    if ledgers.is_empty() {
        return;
    }
    // No career means no player in the world — a plate over a run that has nobody in it. The
    // column stays empty rather than inventing a level, and the node is still there, so the
    // moment a career exists the next frame fills it.
    let Some(career) = careers.iter().next() else { return };
    let lines = ledger::debrief_lines(career);
    for node in &ledgers {
        commands.entity(node).with_children(|column| {
            for line in &lines {
                column.spawn(plate::note(line.clone()));
            }
        });
    }
}

/// Builds the plate out of the mission that just ended.
///
/// `mission` is `Option` for the one case that is not a bug: the entity is despawned the moment
/// the hub opens, and a frame can land between the phase leaving `Debrief` and this plate being
/// taken down. Loud text rather than an empty plate — an empty screen that cannot be clicked
/// out of is the failure this whole domain was built against (`P4`).
pub fn spawn_debrief_screen(
    commands: &mut Commands,
    data: &GameData,
    sortie: Option<(&Mission, &MissionClock, &KillTally, Option<&Verdict>)>,
) {
    let hz = data.game.simulation_hz;
    commands.spawn(plate::root(Screen::Debrief, "debrief")).with_children(|screen| {
        let Some((mission, clock, tally, verdict)) = sortie else {
            screen.spawn(plate::title("Debrief"));
            screen.spawn(plate::note("the sortie is already gone — there is nothing to report"));
            row(screen, DebriefAction::Lobby, "To the lobby  (Esc)", true);
            return;
        };

        // The word is the phase enum's, handed over by `Verdict::label`. `mission::announce` is
        // the one writer of it; a plate that spelled `WON` itself would be a second answer to
        // "how did this end", and the HUD's big line already spells it the first way.
        screen.spawn(plate::title(verdict.map_or("DEBRIEF", |v| v.label())));
        screen.spawn(plate::note(mission.name.clone()));

        screen.spawn(plate::note(format!(
            "{} / {} cortex kills",
            tally.total(),
            tally.target
        )));

        // Both times out of the clock and both in ticks until the last moment — the mission
        // never counted a wall clock and neither does its report (`mission::run`, `to_ticks`).
        let flown = clock.decided_at_tick.unwrap_or(clock.started_at_tick);
        let flown = flown.saturating_sub(clock.started_at_tick);
        screen.spawn(plate::note(format!(
            "{} of {} on the clock",
            clock_face(flown as f64 / hz),
            clock_face(clock.duration_ticks as f64 / hz)
        )));

        // The ledger. Named, and filled by [`fill_the_ledger`] rather than here — see
        // `DebriefLedger`. A column with a gap, so the rows read as a block and not as more
        // sortie numbers; empty until the career is read, and empty forever in the one case
        // that is not a bug (no `Career` on the local player, which is a run with no player).
        screen.spawn((
            Name::new("debrief_ledger"),
            PauseElement,
            DebriefLedger,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                margin: UiRect::vertical(Val::Px(10.0)),
                ..default()
            },
        ));

        row(screen, DebriefAction::Redeploy, "Redeploy", false);
        row(screen, DebriefAction::Lobby, "To the lobby  (Esc)", true);
    });
}

/// One button, named the way every other screen in this domain names its buttons.
fn row(screen: &mut ChildSpawnerCommands, action: DebriefAction, label: &str, chosen: bool) {
    screen
        .spawn((
            Name::new(format!("debrief_{action:?}")),
            action,
            plate::button(plate::BUTTON_W, chosen),
        ))
        .with_child(plate::label(label));
}

/// `m:ss`. Seconds in, one reading out — the lobby writes the same face for the same reason,
/// and a player who has just flown `3:12` should not have to read `192 s` afterwards.
fn clock_face(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    format!("{:.0}:{:02}", (seconds / 60.0).floor(), (seconds % 60.0).round() as u32)
}

/// What the buttons do.
///
/// **They move the screen and nothing else.** Ending the sortie is
/// [`close_the_debrief`]'s job, by whichever door the plate was left through — see there for
/// why that is one writer instead of three.
///
/// ⚠️ *Redeploy* reads `mission::Sortie` for the order it is repeating. That is the finished
/// sortie's own order, not a fresh choice: "the same one again" has to mean the same difficulty
/// too, and `LobbyChoice` may say something else entirely because the player last touched it
/// three sorties ago. It **asks** with a `DeployRequest`, the identical message the lobby's
/// *Deploy* row writes, so `mission::take_orders_from_the_menu` is still the one thing that
/// sets a phase.
pub fn debrief_buttons(
    buttons: Query<(&Interaction, &DebriefAction)>,
    sortie: Res<crate::mission::Sortie>,
    mut screen: ResMut<Screen>,
    mut deploy: MessageWriter<DeployRequest>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            DebriefAction::Redeploy => match sortie.0.as_ref() {
                Some(order) => {
                    info!("debrief: redeploying {:?} at {:?}", order.template, order.difficulty);
                    deploy.write(DeployRequest {
                        template: order.template.clone(),
                        difficulty: order.difficulty.clone(),
                    });
                    *screen = Screen::Playing;
                }
                None => {
                    // Nothing to repeat. Loud, and the screen stays where it is rather than
                    // handing the player back to a game with no sortie in it.
                    error!("Redeploy was pressed and no sortie order is on record");
                }
            },
            DebriefAction::Lobby => *screen = Screen::Lobby,
        }
    }
}

/// **The debrief comes up with the phase** — `OnEnter`, once, and only over a game that is
/// being played.
///
/// ⚠️ **It was a per-frame `if phase == Debrief && screen == Playing` for about an hour, and
/// that version deadlocks the game.** Leaving the report puts the screen back to `Playing`
/// while the phase is *still* `Debrief` — the sortie ends through `AbandonSortie`, which
/// `mission::take_orders_from_the_menu` may only act on once the clock runs. A rule that says
/// "any `Playing` frame during a debrief opens the debrief" therefore re-opens the plate on the
/// very next frame, `apply_screen` stops the clock again, and neither the pending order nor the
/// timer that would end the phase can ever run. Measured, not feared:
/// `tests/menu.rs::f175_the_debrief_is_a_screen_and_it_waits_for_the_player` sat on
/// `the phase is Hub` with the plate still up.
///
/// So it is an **entry**, not a condition. `OnEnter` is the schedule that means "once, on the
/// way in", and it is the same one `mission::report` writes its line from.
///
/// It writes no phase — `menu` may read `mission` and never write it (`docs/architecture.md`,
/// allow list) — and it is registered behind `run_if(there_is_a_window)`, so a `--headless` or
/// `--script` run never opens it. That is the whole reason `missions.ron: hub.debrief_s` still
/// means something: with no screen to hold the phase open, those ticks are what a run with
/// nobody watching waits instead.
pub fn open_the_debrief(mut screen: ResMut<Screen>) {
    if *screen == Screen::Playing {
        *screen = Screen::Debrief;
    }
}

/// And it goes away with the phase — the belt to [`open_the_debrief`]'s braces.
///
/// Every door out of the report moves the screen itself, so this finds nothing to do on any
/// path that exists today. It is here for the one that does not: a plate that outlived its
/// phase would be a full-screen overlay over a running game with the pointer free and no button
/// on it that means anything, which is the `P4` failure with a different picture.
pub fn close_the_debrief_screen(mut screen: ResMut<Screen>) {
    if *screen == Screen::Debrief {
        *screen = Screen::Playing;
    }
}

/// **Leaving the debrief ends the sortie** — through the two buttons, through `Esc`, and
/// through any third door somebody adds later without reading this file.
///
/// One writer of [`AbandonSortie`] for the whole screen, and that is deliberate: *Redeploy*,
/// *To the lobby* and `Esc` all mean "I have read it", and three buttons each writing the
/// message themselves is three places for the fourth one to forget. It is the shape
/// `menu::remember_the_way_into_settings` already uses for the same class of mistake.
///
/// `Screen` is change-detected, so this fires on the one frame the plate was left and never
/// again. `mission::take_orders_from_the_menu` holds the message until the clock runs — which
/// for *To the lobby* is not until the player leaves the lobby as well, and that is correct:
/// the sortie is over either way, and what he does next decides whether the hub or the next
/// deployment comes first.
pub fn close_the_debrief(
    phase: Res<State<crate::mission::MissionPhase>>,
    screen: Res<Screen>,
    mut abandon: MessageWriter<AbandonSortie>,
) {
    if screen.is_changed()
        && *screen != Screen::Debrief
        && *phase.get() == crate::mission::MissionPhase::Debrief
    {
        abandon.write(AbandonSortie);
    }
}
