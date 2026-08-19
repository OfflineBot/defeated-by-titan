//! The guard over "nothing survives quitting" — `F-200`, `F-201`.
//!
//! ⚠️ **These tests assert against the BYTES on disk, not against a round trip.**
//! `docs/FINDINGS.md` FIND-103: a test that serialises and deserialises with the same code
//! passes with the format broken in both directions. So the file is written by the game's own
//! writer and then compared, character for character, against a literal in this file. When that
//! literal has to change, the schema number has to change with it — which is the point.
//!
//! The other half of the evidence is not in here and cannot be: a real process has to end and a
//! second one has to see the first. That is the `--headless` round trip in the commit message.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use defeated_by_titan::data::GameData;
use defeated_by_titan::player::spawn_player;
use defeated_by_titan::save::{
    profile_path, render, Profile, ProfileFile, SaveDir, SortieOutcome, PROFILE_FIELDS, SCHEMA,
};
use defeated_by_titan::shared::{Cli, IdCounter, PlayerId};

/// A directory of this test's own, emptied first. No `tempfile` crate in the tree, and one
/// dependency for four `create_dir_all`s is not a trade worth making.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dbt-save-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The **real** app, headless, with its save directory pointed at a scratch dir — the app the
/// game actually runs, not a second similar one.
fn app(dir: &PathBuf) -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(SaveDir(Some(dir.clone())));
    app
}

/// What a career of exactly this shape looks like on disk. **The frozen bytes.**
const A_CAREER_ON_DISK: &str = "\
(
    schema: 1,
    profile: (
        sorties_flown: 4,
        sorties_won: 1,
        titans_felled: 9,
        best_kills_in_a_sortie: 5,
        seconds_in_the_field: 612.5,
        cleared: [
            \"skirmish/veteran\",
        ],
    ),
)
";

fn a_career() -> Profile {
    Profile {
        sorties_flown: 4,
        sorties_won: 1,
        titans_felled: 9,
        best_kills_in_a_sortie: 5,
        seconds_in_the_field: 612.5,
        cleared: BTreeSet::from(["skirmish/veteran".to_string()]),
    }
}

/// ⭐ **The format itself, in bytes a human can read.**
///
/// Not a round trip: this is the literal file, and it goes red when a field is added, renamed,
/// reordered or formatted differently — every one of which changes what yesterday's build can
/// read (FIND-103).
#[test]
fn f200_a_profile_is_a_file_a_human_can_read() {
    assert_eq!(render(&a_career()), A_CAREER_ON_DISK);
}

/// The tripwire on `F-201`. A new field without a new schema number is a save format that eats
/// an evening — this is what forces the decision to be made rather than discovered.
#[test]
fn f201_the_field_list_is_frozen_until_the_schema_number_moves() {
    let text = render(&Profile::default());
    let keys: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.contains("profile:"))
        .skip(1)
        .filter_map(|l| l.trim().split_once(':').map(|(k, _)| k))
        .filter(|k| !k.starts_with('(') && !k.is_empty())
        .collect();
    assert_eq!(
        keys, PROFILE_FIELDS,
        "the profile's shape changed. Bump save::file::SCHEMA, add the migration arm, and \
         update PROFILE_FIELDS and A_CAREER_ON_DISK — an old file has to keep loading"
    );
    assert_eq!(SCHEMA, 1, "SCHEMA moved: this test's literals move with it");
}

/// ⭐ The write is **atomic** (`F-200`: no data loss). What a reader can ever see is the whole
/// old file or the whole new one — never a half-written one, and never a stray `.writing`.
#[test]
fn f200_a_written_profile_leaves_no_half_file_behind() {
    let dir = scratch("atomic");
    defeated_by_titan::save::file::write_profile(&dir, PlayerId(3), &a_career()).unwrap();

    let path = profile_path(&dir, PlayerId(3));
    assert_eq!(fs::read_to_string(&path).unwrap(), A_CAREER_ON_DISK);
    let leftovers: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != "player-3.ron")
        .collect();
    assert!(leftovers.is_empty(), "the temporary file survived the rename: {leftovers:?}");
}

/// ⭐ `F-201`: a file yesterday's build wrote — one that has never heard of half the fields —
/// **loads**, and the loader says which fields it had to fill in. A save that crashed on it
/// would be data loss with a stack trace.
#[test]
fn f201_a_profile_from_an_older_shape_still_loads_and_names_what_was_missing() {
    let dir = scratch("older");
    fs::write(
        profile_path(&dir, PlayerId(1)),
        "(schema: 1, profile: (sorties_flown: 2, titans_felled: 6))",
    )
    .unwrap();

    let got = defeated_by_titan::save::file::read_profile(&dir, PlayerId(1));
    assert_eq!(got.profile.sorties_flown, 2, "what the file HAD is what came back");
    assert_eq!(got.profile.titans_felled, 6);
    assert_eq!(got.profile.sorties_won, 0, "what it had not is the empty career's value");
    assert_eq!(
        got.missing,
        vec!["sorties_won", "best_kills_in_a_sortie", "seconds_in_the_field", "cleared"],
        "a default nobody can read in the log is the thing rule 2 was written against"
    );
}

/// ⭐ `F-200`, the other direction: a file from a **newer** build is refused, not truncated.
/// An old build that read it leniently and wrote it back would delete every field it does not
/// know — the exact data loss the bible's ProfileStore requirement is about.
#[test]
fn f200_a_newer_build_s_file_is_never_written_over() {
    let dir = scratch("future");
    let path = profile_path(&dir, PlayerId(1));
    let untouched = "(schema: 12, profile: (sorties_flown: 40, lineage: \"reiss\"))";
    fs::write(&path, untouched).unwrap();

    let mut app = app(&dir);
    app.update();
    app.update();

    let world = app.world_mut();
    let mut q = world.query::<(&PlayerId, &Profile, &ProfileFile)>();
    let (_, profile, on_disk) = q.iter(world).next().expect("the local player has a career");
    assert_eq!(*profile, Profile::default(), "nothing from a future file is guessed at");
    assert!(!on_disk.may_write, "this run must not save over it");

    // And the bytes are still the bytes.
    assert_eq!(fs::read_to_string(&path).unwrap(), untouched);
}

/// ⭐ **Rule 4: two players, two careers, two files.** The one test that would go red if a
/// `Resource<Profile>` or a `.single()` ever crept in — with one profile for the world, the
/// second player's two kills would land on the first player's record.
#[test]
fn f200_two_players_get_two_careers_and_two_files() {
    let dir = scratch("two");
    let mut app = app(&dir);
    app.update(); // Startup: the world and the local player

    let second = {
        let world = app.world_mut();
        let data = world.resource::<GameData>().clone();
        let mut ids = world.resource::<IdCounter>().to_owned();
        let mut commands = world.commands();
        let e = spawn_player(&mut commands, &mut ids, &data, Vec3::new(20.0, 2.0, 0.0), false);
        let counted = ids;
        world.flush();
        *world.resource_mut::<IdCounter>() = counted;
        e
    };
    app.update(); // both players now have a profile

    let their_id = *app.world().entity(second).get::<PlayerId>().expect("a stable id");
    app.world_mut().write_message(SortieOutcome {
        player: their_id,
        template: "skirmish".into(),
        difficulty: Some("veteran".into()),
        won: true,
        kills: 2,
        seconds: 30.0,
        tick: 1800,
    });
    app.update();

    let world = app.world_mut();
    let mut q = world.query::<(&PlayerId, &Profile)>();
    let mut careers: Vec<(u32, u32, u32)> =
        q.iter(world).map(|(id, p)| (id.0, p.sorties_flown, p.titans_felled)).collect();
    careers.sort_unstable();
    assert_eq!(
        careers,
        vec![(1, 0, 0), (their_id.0, 1, 2)],
        "the sortie landed on exactly one career"
    );

    // ⭐ And on disk: one file, his, and none for the player who flew nothing.
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(files, vec![format!("player-{}.ron", their_id.0)]);
}

/// A run with no save directory touches no disk. `SaveDir::discover` decides this for every
/// `cargo test` binary, which is why the other twenty integration tests do not write anything.
#[test]
fn f200_without_a_save_directory_nothing_is_written() {
    let dir = scratch("off");
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(SaveDir(None));
    app.update();
    app.update();

    let player = {
        let world = app.world_mut();
        let mut q = world.query::<&PlayerId>();
        *q.iter(world).next().expect("a player")
    };
    app.world_mut().write_message(SortieOutcome {
        player,
        template: "tutorial".into(),
        difficulty: None,
        won: false,
        kills: 0,
        seconds: 10.0,
        tick: 600,
    });
    app.update();

    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0, "a disabled save wrote a file anyway");
    let world = app.world_mut();
    let mut q = world.query::<&Profile>();
    assert_eq!(
        q.iter(world).next().unwrap().sorties_flown,
        1,
        "the career still counts inside the session — only the disk is off"
    );
}
