//! The objective line — amber, top centre. The counter and the verdict.
//!
//! # What this line is for
//!
//! `docs/PLAN-GAME.md` §1 spends two of its eight sentences on this element: *"a counter in the
//! top of the screen goes from `0/3` to `1/3`"* and *"the screen says **LOST**" / "it says
//! **WON**"*. Until 2026-08-10 neither existed — the verdict was reachable only through the F3
//! debug overlay, which is a tool and not a screen. A player could win the mission and never be
//! told.
//!
//! So the line reads the mission's own state and nothing else:
//!
//! | phase | the line |
//! |---|---|
//! | `Briefing`, `Deploying`, or no mission at all | **hidden**, empty |
//! | `Active` | `total/target`, e.g. `0/3` → `1/3` → `3/3` |
//! | `Won`, `Lost` | [`MissionPhase::label`], big |
//!
//! # Three things here are deliberate, and each has a test
//!
//! **The target is not a `3`.** It is `kill_target` out of `missions.ron`, carried in
//! [`KillTally::target`], and this file never writes a number of its own next to it
//! (`CLAUDE.md` rule 2). `tests/hud.rs::f170_the_objective_counts_the_real_kills` reads the
//! expectation out of `GameData`, so a literal here goes red the moment the file changes.
//!
//! **The words are not this file's.** `WON` and `LOST` belong to [`MissionPhase::label`], which
//! the F3 overlay already uses — a second spelling in the HUD is how a renamed phase ends up
//! saying two different things in two places. The same test reads this source file back and
//! falls over if any phase's word appears in it as a literal.
//!
//! **The count is the squad's, not mine.** [`KillTally`] holds one number per
//! [`PlayerId`](crate::shared::PlayerId) and the line sums them ([`KillTally::total`], whose own
//! doc says "this is what the objective counts"): the mission is won by the team, and a line
//! showing `KillTally::of(local)` would read `2/3` next to a `WON` in the first co-op session.
//! Splitting the credit up is `F-096`'s job, and it will read the same per-player map.
//!
//! # The failure this element was built against
//!
//! *"The bar that is a picture of a bar"* (`docs/PLAN-GAME.md` §8, `F-170`) — present, correct
//! looking, wired to a constant. This line spent a session hidden rather than showing an
//! invented objective, and the test that held it empty was not deleted when the producer
//! landed: it grew a second half, and now a line that hides in every case fails it too.

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::{signal, HudElement};
use crate::mission::{KillTally, MissionPhase};

/// Marker on the objective line.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ObjectiveLine;

const TOP_PCT: f32 = 3.0;
const LEFT_PCT: f32 = 30.0;
const WIDTH_PCT: f32 = 40.0;

/// The counter, `1/3`. Information, read when you look for it.
pub const COUNT_PX: f32 = 18.0;
/// The verdict. **Deliberately more than twice the counter**: `docs/PLAN-GAME.md` §1 asks that
/// "the screen says LOST", and a mission's whole outcome in 18 px is a thing you can miss while
/// watching a titan fall over. Still 3 % from the top edge and 40 % wide, so it stays out of the
/// central keep-out box — `tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen` laid out
/// forty characters at this size to check exactly that.
pub const VERDICT_PX: f32 = 44.0;

/// No font asset and no `Camera2d` — `default_font` is on in `Cargo.toml`, so the default
/// `TextFont` resolves to the built-in `FiraMono-subset.ttf`.
///
/// Two field types changed in bevy 0.19 and bite anyone writing this from memory:
/// `font_size` is a [`FontSize`] (`bevy_text-0.19.0/src/text.rs:392`, enum `:487-500`) and
/// `font` is a `FontSource` (`:383`, enum `:282-307`).
pub fn spawn_objective(mut commands: Commands, data: Res<GameData>) {
    let amber = signal(&data, "amber");
    commands.spawn((
        Name::new("hud_objective"),
        ObjectiveLine,
        HudElement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(COUNT_PX), ..default() },
        TextLayout::justify(Justify::Center),
        TextColor(amber),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(TOP_PCT),
            left: Val::Percent(LEFT_PCT),
            width: Val::Percent(WIDTH_PCT),
            // Hidden until a mission runs — `update_objective` decides that every frame.
            display: Display::None,
            ..default()
        },
    ));
}

/// The seam: mission state in, one line of text out. `None` means **hide the line**.
///
/// It takes what it needs as arguments and not as a `Res`, so it is testable without an app and
/// so that "what does the screen say" is one pure function and not a system.
///
/// `tally: None` is "no mission is running" — a plain `cargo run` or `--sandbox` stays in
/// `Briefing` forever and there is no mission entity, and then there is nothing to say. That is
/// the case that keeps this element honest: an empty screen corner, not an invented objective.
pub fn objective_text(phase: MissionPhase, tally: Option<&KillTally>) -> Option<String> {
    let tally = tally?;
    if phase.is_decided() {
        // The word is the phase's. Not a copy of it — see the module doc.
        return Some(phase.label().to_string());
    }
    match phase {
        // The only phase in which anything is being counted.
        MissionPhase::Active => Some(format!("{}/{}", tally.total(), tally.target)),
        // The mission exists but has not opened the field: `0/3` before the first tick would be
        // a count of something that cannot have happened yet.
        // `Hub` is here for the same reason (2026-08-12): standing in the main building is not
        // an objective, and the mission entity is despawned on the way in anyway — so this arm
        // is the belt to that braces.
        MissionPhase::Briefing | MissionPhase::Deploying | MissionPhase::Hub => None,
        MissionPhase::Won | MissionPhase::Lost => unreachable!("handled by is_decided above"),
    }
}

/// How big the line is drawn, in logical pixels. Pure, for the same reason as above.
pub fn objective_font_px(phase: MissionPhase) -> f32 {
    if phase.is_decided() { VERDICT_PX } else { COUNT_PX }
}

/// Writes the line out of the **real** mission state.
///
/// `KillTally` is a `Query` and not a resource because that is the shape it has: a component on
/// the mission entity with one number per player (`docs/PLAN-GAME.md` §5). The phase is a
/// `Res<State<_>>` because there is exactly one mission and every player is in it
/// (`mission::phase`, which explains at length why that is not a breach of multiplayer rule 4).
///
/// Every write is guarded by a comparison. This runs in `Update`, and an unconditional
/// `node.display = …` marks the `Node` changed on every frame, which puts the UI layout to work
/// on a line that says the same thing it said 16 ms ago (`CLAUDE.md` rule 6).
pub fn update_objective(
    phase: Res<State<MissionPhase>>,
    tallies: Query<&KillTally>,
    mut lines: Query<(&mut Text, &mut Node, &mut TextFont), With<ObjectiveLine>>,
) {
    let phase = *phase.get();
    let text = objective_text(phase, tallies.iter().next());
    let px = objective_font_px(phase);

    for (mut line, mut node, mut font) in &mut lines {
        match &text {
            Some(t) => {
                if line.0 != *t {
                    line.0.clone_from(t);
                }
                if !matches!(font.font_size, FontSize::Px(current) if current == px) {
                    font.font_size = FontSize::Px(px);
                }
                if node.display != Display::Flex {
                    node.display = Display::Flex;
                }
            }
            None => {
                if !line.0.is_empty() {
                    line.0.clear();
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
    use super::*;
    use crate::shared::{PlayerId, TitanId};

    #[test]
    fn f170_a_line_without_a_mission_is_no_line() {
        // `--sandbox` and a plain `cargo run` have no mission entity. Every phase, so that no
        // future variant quietly starts drawing an empty `/`.
        for phase in [
            MissionPhase::Briefing,
            MissionPhase::Deploying,
            MissionPhase::Active,
            MissionPhase::Won,
            MissionPhase::Lost,
        ] {
            assert_eq!(objective_text(phase, None), None, "{phase:?} without a mission");
        }
    }

    #[test]
    fn f170_the_counter_is_the_files_number_and_not_a_three() {
        // The target comes in with the tally, out of `missions.ron`. A `3` written into this
        // module would pass a test that only ever used the tutorial.
        let mut tally = KillTally::with_target(5);
        assert_eq!(objective_text(MissionPhase::Active, Some(&tally)).unwrap(), "0/5");
        tally.credit(PlayerId(1), TitanId(1));
        assert_eq!(objective_text(MissionPhase::Active, Some(&tally)).unwrap(), "1/5");
        tally.credit(PlayerId(2), TitanId(2));
        assert_eq!(
            objective_text(MissionPhase::Active, Some(&tally)).unwrap(),
            "2/5",
            "the second player's kill counts too — the mission is won by the squad"
        );
        // The same two kills against a different target read differently. That is the whole
        // point of the number living in the file.
        let mut two = KillTally::with_target(2);
        two.credit(PlayerId(1), TitanId(1));
        two.credit(PlayerId(1), TitanId(2));
        assert_eq!(objective_text(MissionPhase::Active, Some(&two)).unwrap(), "2/2");
    }

    #[test]
    fn f170_the_verdict_is_the_phases_own_word() {
        let tally = KillTally::with_target(3);
        assert_eq!(
            objective_text(MissionPhase::Won, Some(&tally)).unwrap(),
            MissionPhase::Won.label()
        );
        assert_eq!(
            objective_text(MissionPhase::Lost, Some(&tally)).unwrap(),
            MissionPhase::Lost.label()
        );
        // A verdict is not a counter: nobody wants to read `0/3` under the word LOST.
        assert!(!objective_text(MissionPhase::Lost, Some(&tally)).unwrap().contains('/'));
    }

    #[test]
    fn f170_the_verdict_is_drawn_bigger_than_the_counter() {
        assert!(objective_font_px(MissionPhase::Won) > objective_font_px(MissionPhase::Active));
        assert!(objective_font_px(MissionPhase::Lost) > objective_font_px(MissionPhase::Active));
        assert_eq!(objective_font_px(MissionPhase::Briefing), COUNT_PX);
    }
}
