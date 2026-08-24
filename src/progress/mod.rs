//! progress — what a finished sortie **means**, and it is the other half of the save game.
//!
//! ⚠️ **The meta systems still come only after the Vector Gear gate.** The trait tree, the two
//! currencies, the lineages and the ascension (`F-120`…`F-148`) are ⬜ and none of them is
//! started here (bible 6.1): the graveyard of this genre is made of games that built the
//! economy before the movement convinced. What *is* built is the seam they all hang off —
//! **the one place that notices a sortie has ended and says who was in it.**
//!
//! ## The one thing this domain does today
//!
//! On `OnEnter(Won)` and `OnEnter(Lost)` it reads the mission's own numbers and writes one
//! [`SortieOutcome`](crate::save::SortieOutcome) **per player**. That is all. It does not touch
//! a [`Profile`](crate::save::Profile), it does not open a file, and it does not decide what a
//! kill is worth.
//!
//! ### Why `save` and not a `&mut Profile` here
//!
//! Because `Profile` has exactly one writer (`docs/architecture.md`, authority table), and the
//! reason is the one `Gas` cost a repair for on 2026-08-12 (`FINDINGS.md` FIND-063): a second
//! domain moving the same field is a coin toss at 60 Hz and, over a wire, two machines
//! disagreeing about a player's career. So this domain **asks** — and it asks with *facts*
//! (kills, seconds, a verdict), never with rewards. "12 XP for a husk" is `F-120`'s question
//! and its answer belongs in a RON file, not in this one (rule 2).
//!
//! ### Why it reads `mission` instead of waiting for a message
//!
//! `mission` sends nothing when a sortie ends, and it may not be made to: a `SortieEnded`
//! message would be a **second** end-of-sortie mechanism beside the state transition that
//! already despawns the field, and `docs/architecture.md` argues exactly that under
//! `titan -> mission`. The verdict, the tally and the clock are **state**, and the one frame
//! they are all still true in is the transition itself — `mission::announce` reads them the
//! same way, in the same schedule, for the same reason. Read-only, and `mission` stays the one
//! writer of every one of them.
//!
//! ### What is deliberately not recorded
//!
//! An **abandoned** sortie. `shared::AbandonSortie` takes the phase straight back to `Hub`
//! without a verdict, so nothing was decided and nothing is booked — walking out of a fight you
//! are losing must not be able to pad a career. When that turns out to be the wrong call it is
//! one `OnEnter` away, and the profile field it needs already exists.

pub mod career;
pub mod gear;

pub use career::Career;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::data::GameData;
use crate::mission::{KillTally, Mission, MissionClock, MissionPhase, Sortie};
use crate::save::{Profile, SortieOutcome};
use crate::shared::{PlayerId, Tick};

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        // Two thin systems and one function: a Bevy system cannot take the verdict as an
        // argument, and duplicating the body is how the two halves drift apart.
        app.add_systems(OnEnter(MissionPhase::Won), record_a_win)
            .add_systems(OnEnter(MissionPhase::Lost), record_a_loss)
            // `Update` and not `FixedUpdate`: a career is not simulation, nothing in the fixed
            // step reads it, and it only ever recomputes when the profile behind it moved.
            .add_systems(Update, refresh_careers);
    }
}

/// Everything the recorder needs, in one place so the two entry points cannot disagree.
#[derive(SystemParam)]
pub struct Field<'w, 's> {
    tick: Res<'w, Tick>,
    data: Res<'w, GameData>,
    sortie: Res<'w, Sortie>,
    missions: Query<'w, 's, (&'static Mission, &'static MissionClock, &'static KillTally)>,
    players: Query<'w, 's, &'static PlayerId>,
}

fn record_a_win(field: Field, out: MessageWriter<SortieOutcome>) {
    record(true, field, out);
}

fn record_a_loss(field: Field, out: MessageWriter<SortieOutcome>) {
    record(false, field, out);
}

/// One [`SortieOutcome`] per player, and none at all when there was no sortie.
///
/// **Per player and not per sortie** — twenty people fly the same mission and each of them
/// carries his own kills out of it (`F-096`, `F-161a`). `KillTally::of` already answers per
/// player, so the shape costs nothing today and is the one that is still right on the day
/// `net` is real. No `.single()` on the player query: there is no such thing as *the* player.
fn record(won: bool, field: Field, mut out: MessageWriter<SortieOutcome>) {
    let mut flown = field.missions.iter();
    let Some((mission, clock, tally)) = flown.next() else {
        // A verdict without a mission entity. `mission::deploy` refuses to build a half-mission,
        // so this is the honest "nothing was flown" and not an error worth shouting about.
        return;
    };
    if flown.next().is_some() {
        // Two tallies in one frame would book every sortie twice. `mission::hub::open_hub`
        // despawns the old mission before the new one is furnished precisely so this cannot
        // happen — if it ever does, the career is what pays for it.
        error!("progress: two missions ended in the same frame — only the first is recorded");
    }

    let hz = field.data.game.simulation_hz;
    let ticks_flown = field.tick.0.saturating_sub(clock.started_at_tick);
    let seconds = (ticks_flown as f64 / hz) as f32;
    // The tier comes from the order, not from the template: `skirmish` is one mission with
    // three of them, and a Recruit clear is not an Elite clear (`missions.ron`).
    let difficulty = field.sortie.0.as_ref().and_then(|order| order.difficulty.clone());

    let mut recorded = 0u32;
    for player in &field.players {
        out.write(SortieOutcome {
            player: *player,
            template: mission.template.clone(),
            difficulty: difficulty.clone(),
            won,
            kills: tally.of(*player),
            seconds,
            tick: field.tick.0,
        });
        recorded += 1;
    }
    info!(
        "progress: sortie {:?} {} at tick {} after {:.1} s — {} {} recorded, {}/{} kills",
        mission.template,
        if won { "WON" } else { "LOST" },
        field.tick.0,
        seconds,
        recorded,
        if recorded == 1 { "career" } else { "careers" },
        tally.total(),
        tally.target
    );
}

/// **`F-120`/`F-121`/`F-122` — what the profile MEANS**, recomputed onto the player whenever
/// `save` has moved it.
///
/// `Changed<Profile>` and not every frame (rule 6): after the first frame the archetype this
/// matches is empty until a sortie is booked. Change detection is what makes the ordering
/// against `save::record_outcomes` a non-question — whichever of the two runs first, the career
/// is right on the next run of this system at the latest, and a debrief screen stands for
/// hundreds of frames.
///
/// **No `.single()`, and one [`Career`] per player** (rule 4). `save` stays the only writer of
/// [`Profile`]; this domain writes only the derived thing, which nobody else touches.
fn refresh_careers(
    mut commands: Commands,
    data: Res<GameData>,
    mut players: Query<(Entity, &PlayerId, &Profile, Option<&mut Career>), Changed<Profile>>,
) {
    for (entity, player, profile, career) in &mut players {
        let fresh = Career::of(profile, &data.progress);
        match career {
            Some(mut standing) => {
                let updated = fresh.after(&standing);
                if updated.last_sortie_xp > 0 {
                    info!(
                        "progress: player {} earned {} xp — {}{}",
                        player.0,
                        updated.last_sortie_xp,
                        updated.one_line(),
                        match updated.levelled_up_to {
                            Some(level) => format!(" — LEVEL {level}"),
                            None => String::new(),
                        }
                    );
                }
                *standing = updated;
            }
            None => {
                info!("progress: player {} carries in {}", player.0, fresh.one_line());
                commands.entity(entity).insert(fresh);
            }
        }
    }
}
