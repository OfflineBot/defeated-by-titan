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
//! ## The ledger: the named place progression goes
//!
//! Levels, XP and gear rank are `progress`' and they are being built in a parallel round; this
//! screen does not compute a single one of them. What it leaves is [`DebriefLedger`] — a named,
//! empty column between the sortie's numbers and the buttons — and the contract is written on
//! it. **It draws nothing today**: a heading with nothing under it is the row-that-does-nothing
//! this domain refuses to spawn anywhere else.

use bevy::prelude::*;

use super::{plate, PauseElement, Screen};
use crate::data::GameData;
use crate::mission::{KillTally, Mission, MissionClock, Verdict};
use crate::shared::{AbandonSortie, DeployRequest};

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

/// **The place `progress` draws into**, and it is empty on purpose.
///
/// The debrief is where a career becomes visible: what the sortie earned, which level it moved,
/// which gear rank it unlocked. None of that is computed here and none of it is computed by
/// `menu` at all — `progress` already reads the finished mission and writes one
/// `save::SortieOutcome` per player (`src/progress/mod.rs`), and `save::Profile` already holds
/// `sorties_flown`, `sorties_won`, `titans_felled`, `best_kills_in_a_sortie`,
/// `seconds_in_the_field` and `cleared`.
///
/// **What has to exist before rows appear under this marker:**
///
/// 1. a **read** of what the sortie changed that is not a `&mut` of anybody's field — the
///    obvious shape is the one `mission` uses for its own numbers: a component on the finished
///    mission entity, or a `Resource` written once by `progress` in the same `OnEnter` it
///    already runs in;
/// 2. one line on the allow list of `docs/architecture.md` (`menu -> progress`, read-only,
///    with the same reason `menu -> mission` carries);
/// 3. nothing else. The rows are `plate::note`s spawned as children of this node, between the
///    sortie's own numbers and the two buttons, and no other line of this file moves.
///
/// It carries no colour, no height and no child. It is a hole with a name, and it is here so
/// that the shape is decided **before** two rounds decide it differently.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DebriefLedger;

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

        // The ledger. Named, empty, and documented — see `DebriefLedger`.
        screen.spawn((Name::new("debrief_ledger"), PauseElement, DebriefLedger, Node::default()));

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
