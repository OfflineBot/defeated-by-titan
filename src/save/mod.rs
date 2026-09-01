//! save — the save game: profile, and the room the gear budget, traits and lineage grow into.
//!
//! **What survives quitting, as of 2026-08-19:** one [`Profile`] per player — sorties flown and
//! won, titans felled, the best single sortie, time in the field, and which difficulties have
//! been cleared. It lives in `saves/player-<id>.ron` and it is written the moment a sortie
//! reaches a verdict.
//!
//! ## The two domains, and why they are two
//!
//! - **`save` is storage.** The file format, where it lives, the atomic write, the schema and
//!   its migration. It knows what a career *is*; it does not know what a sortie *is*.
//! - **`progress` is meaning.** It is the domain that notices a verdict has fallen and who was
//!   in it, and it says so with one [`SortieOutcome`] per player.
//!
//! The seam between them is a message, and it is the same seam — and the same reason — as
//! `shared::RefuelRequest` between a hub station and the gas tank (`FINDINGS.md` FIND-063):
//! **[`Profile`] has exactly one writer.** `progress` asks, `save` writes. Two domains moving
//! the same number is not a design, it is a coin toss at 60 Hz, and over a wire two machines
//! disagreeing about a player's career.
//!
//! ## Rule 4 — there is no such thing as *the* profile
//!
//! [`Profile`] is a **component on the player**, keyed by [`PlayerId`]. Not a `Resource`: a
//! resource holds one of a thing, and the first co-op sortie would give twenty people one
//! shared career — the identical arithmetic that keeps `Gas` a component, and the shape
//! `Q-038` currently has open against `PlayerSettings`. Nothing in this module calls
//! `.single()`, and `tests/save.rs::f200_two_players_get_two_careers_and_two_files` is the
//! guard that keeps it that way.
//!
//! ## `F-200`, honestly scoped
//!
//! *No data loss, no duplication* is the requirement, and what is built of it here is: an
//! **atomic** write (temp file, `sync_all`, rename), **one writer**, an unreadable file **kept**
//! rather than overwritten, and a file from a newer build **refused** rather than truncated.
//! The bible's session lock across servers is not built — there is no server to lock against
//! (`docs/architecture.md`, the Roblox translation table). The shape it needs is here: the
//! record hangs on a [`PlayerId`], never on an `Entity` and never on a connection.
//!
//! ## What is not saved yet, and it is the obvious next step
//!
//! **`PlayerSettings`** — mouse sensitivity, FOV, the aim assist. They are per *machine*, not
//! per career, and they are a `Resource` written by `menu`; saving them from here would make
//! `save` a second writer of a field somebody else owns. It wants its own file
//! (`saves/settings.ron`), loaded before `menu` touches anything, and it wants `Q-038` decided
//! first.

pub mod file;
pub mod profile;

use bevy::prelude::*;

pub use file::{profile_path, render, LoadNote, Loaded, SaveDir, PROFILE_FIELDS, SCHEMA};
pub use profile::{
    xp_earned, xp_of_a_bare_career, GearChange, GearRequest, Profile, SortieOutcome,
};

use crate::data::GameData;
use crate::shared::PlayerId;

/// Where a player's profile came from, and whether this build may write it back.
///
/// It sits next to the [`Profile`] on the same entity instead of inside it, because it is not
/// part of the career — it is a fact about *this run's* file, and it must never be serialised
/// into the save.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ProfileFile {
    pub may_write: bool,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveDir::discover());
        app.add_message::<SortieOutcome>();
        app.add_message::<GearRequest>();
        // `Update` and not `FixedUpdate`: file I/O is not simulation. It must not sit in the
        // fixed step, where a slow disk would stretch a tick and change how the game moves —
        // and nothing in the simulation reads a `Profile`.
        app.add_systems(Update, (load_profiles, record_outcomes, spend_gear_points).chain());
    }
}

/// Gives every player without one a [`Profile`], read off the disk.
///
/// The query is `Without<Profile>` and not `Added<PlayerId>`: a player who arrives in a frame
/// this system has already run in would otherwise never get a career. It costs nothing to run
/// every frame — after the first, the archetype it matches is empty (`CLAUDE.md` rule 6).
///
/// **No `.single()`.** Every player is one of many, and each of them gets his own file.
fn load_profiles(
    mut commands: Commands,
    dir: Res<SaveDir>,
    data: Res<GameData>,
    fresh: Query<(Entity, &PlayerId), Without<Profile>>,
) {
    for (entity, player) in &fresh {
        let Some(dir) = dir.path() else {
            // No save directory: this process does not touch the disk (a test binary, or
            // `DBT_SAVE_DIR=off`). The player still gets a career — it just starts empty every
            // time, and nothing is written.
            commands.entity(entity).insert((Profile::default(), ProfileFile { may_write: false }));
            continue;
        };
        let loaded = file::read_profile(dir, *player, &data.progress.xp);
        match &loaded.note {
            LoadNote::Fresh => info!(
                "save: player {} has no profile yet — a first sortie ({})",
                player.0,
                file::profile_path(dir, *player).display()
            ),
            LoadNote::Current => info!(
                "save: profile for player {} loaded — {}",
                player.0,
                loaded.profile.one_line()
            ),
            LoadNote::Migrated { from } => info!(
                "save: profile for player {} migrated from schema {} to {} — {}",
                player.0, from, SCHEMA, loaded.profile.one_line()
            ),
            LoadNote::Broken { keep, why } => error!(
                "save: profile for player {} is unreadable ({why}) — kept at {}, the career \
                 starts empty",
                player.0,
                keep.display()
            ),
            LoadNote::FromTheFuture { schema } => error!(
                "save: profile for player {} was written by a newer build (schema {schema}, this \
                 one reads {SCHEMA}) — it will NOT be written over, and nothing from this \
                 session is saved",
                player.0
            ),
        }
        if !loaded.missing.is_empty() {
            // Loud, on purpose: a default you can read in the log is a decision, a default you
            // cannot is the thing `CLAUDE.md` rule 2 was written against.
            warn!(
                "save: profile for player {} had no {:?} — filled from the empty career",
                player.0, loaded.missing
            );
        }
        let may_write = loaded.note.may_write();
        commands.entity(entity).insert((loaded.profile, ProfileFile { may_write }));
    }
}

/// Books what `progress` reports and puts the result on disk. **The only writer of
/// [`Profile`], and the only thing in the tree that writes a save file.**
fn record_outcomes(
    mut outcomes: MessageReader<SortieOutcome>,
    dir: Res<SaveDir>,
    data: Res<GameData>,
    mut players: Query<(&PlayerId, &mut Profile, &ProfileFile)>,
) {
    for outcome in outcomes.read() {
        let mut anybody = false;
        for (player, mut profile, on_disk) in &mut players {
            if *player != outcome.player {
                continue;
            }
            anybody = true;
            let earned = profile.record(outcome, &data.progress.xp);
            let Some(dir) = dir.path() else { continue };
            if !on_disk.may_write {
                warn!(
                    "save: player {} flew a sortie, but his file belongs to a newer build — \
                     nothing written",
                    player.0
                );
                continue;
            }
            match file::write_profile(dir, *player, &profile) {
                Ok(path) => info!(
                    "save: player {} — +{} xp, {} → {}",
                    player.0,
                    earned,
                    profile.one_line(),
                    path.display()
                ),
                // Not a panic: losing a sortie's record is bad, losing the running game on top
                // of it is worse, and the message says exactly what happened.
                Err(e) => error!(
                    "save: could not write the profile of player {} to {}: {e}",
                    player.0,
                    dir.display()
                ),
            }
        }
        if !anybody {
            // A sortie was recorded for somebody who is not in the world — a disconnect
            // between the verdict and the write, and exactly the case `F-200` calls data loss.
            warn!(
                "save: a sortie outcome arrived for player {} but no such player is here — \
                 nothing recorded",
                outcome.player.0
            );
        }
    }
}

/// `F-125` — **books what the armoury asked for**, and writes the file it changed.
///
/// The twin of [`record_outcomes`] and the same shape exactly: `progress` asks with a
/// [`GearRequest`], this is the only thing that moves [`Profile::gear`], and the save is written
/// the moment it moves. A loadout that survived only until the next launch would be a loadout
/// nobody bothers to set.
///
/// **Refusals are loud and change nothing.** An overspend, an axis `progress.ron` no longer
/// defines, a reset of an empty build: each is a `warn!` with the reason in it and no write at
/// all — not a panic, because losing a keypress is bad and losing the running game on top of it
/// is worse, and not silence, because a screen that does nothing when you press a key is the
/// bug this whole round exists to end.
fn spend_gear_points(
    mut asks: MessageReader<GearRequest>,
    dir: Res<SaveDir>,
    data: Res<GameData>,
    mut players: Query<(&PlayerId, &mut Profile, &ProfileFile)>,
) {
    for ask in asks.read() {
        let mut anybody = false;
        for (player, mut profile, on_disk) in &mut players {
            if *player != ask.player {
                continue;
            }
            anybody = true;
            match profile.spend_gear(&ask.change, ask.budget, &data.progress.gear) {
                Err(why) => {
                    warn!("save: player {} — {why}", player.0);
                    continue;
                }
                Ok(what) => info!("save: player {} — {what}", player.0),
            }
            let Some(dir) = dir.path() else { continue };
            if !on_disk.may_write {
                warn!(
                    "save: player {} changed his build, but his file belongs to a newer build \
                     — nothing written",
                    player.0
                );
                continue;
            }
            if let Err(e) = file::write_profile(dir, *player, &profile) {
                error!(
                    "save: could not write the profile of player {} to {}: {e}",
                    player.0,
                    dir.display()
                );
            }
        }
        if !anybody {
            warn!(
                "save: a gear request arrived for player {} but no such player is here — \
                 nothing changed",
                ask.player.0
            );
        }
    }
}
