//! The save game on disk: where it is, how it is written, and how it survives a version change.
//!
//! ## Rule 2 says "no `serde(default)`". A save file is the one place where that is wrong.
//!
//! `CLAUDE.md` §2 is about **tuning data**: a missing `gas_per_s` in `gear.ron` has to crash at
//! startup, loudly, with a file name — because a human typed that file, the value has no honest
//! stand-in, and running the game on a silent `0.0` is worse than not running it at all.
//!
//! **A save file is not tuning data.** Nobody typed it; *this program* wrote it, possibly a
//! version ago, and the player's evening is inside it. A format that crashes on a file
//! yesterday's build produced is not rigour, it is data loss with a stack trace — and `F-200`
//! is explicit that no data may be lost.
//!
//! So the rule is split, and the split is the whole of this module:
//!
//! | | `assets/data/*.ron` (tuning) | the save file |
//! |---|---|---|
//! | missing field | **crash at load** — rule 2, unchanged | filled from [`Profile::default`] **and named in a `WARN`** |
//! | unknown field | crash | ignored (a newer build wrote it) |
//! | unreadable file | crash | kept as `.broken`, career starts empty, `ERROR` in the log |
//! | who wrote it | a human | this program |
//!
//! **`serde(default)` is allowed in `src/save/` and nowhere else.** It is not silent here: the
//! loader parses every field as an `Option` first, so it can say *which* fields were absent
//! before it fills them ([`Loaded::missing`]). A default you can read in the log is a decision;
//! a default you cannot is the thing rule 2 was written against.
//!
//! ## `F-201` — the schema number, and what it actually buys
//!
//! Every file carries `schema:` as its **first** field, and it is read on its own before
//! anything else is parsed. Two directions, two different answers:
//!
//! - **older than this build** → migrate. Today the migration is the field-filling above,
//!   because [`SCHEMA`] 1 is the first shipped version and there is no earlier shape to
//!   translate. The `match` in [`migrate`] is where the second arm goes, and
//!   `tests/save.rs::f201_the_field_list_is_frozen_until_the_schema_number_moves` is what makes
//!   somebody write it: it fails the moment a field is added or renamed.
//! - **newer than this build** → **refuse, and do not touch the file.** An old build that
//!   parsed a new file leniently and then wrote it back would delete every field it did not
//!   understand. That is the duplication-and-loss case `F-200` names, and it is the only
//!   loading error in here that is not recoverable.
//!
//! ## Where the file lives
//!
//! `saves/player-<id>.ron`, one file per player — `saves/` was already in `.gitignore`.
//! One file per [`PlayerId`](crate::shared::PlayerId) and not one table for everybody: two
//! players' careers must not be able to corrupt each other, and a rename is atomic per file.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::XpTuning;
use crate::shared::PlayerId;

use super::profile::{xp_of_a_bare_career, Profile};

/// The version of the shape in [`Profile`]. **Bumped whenever a field is added, removed or
/// renamed**, together with a new arm in [`migrate`].
pub const SCHEMA: u32 = 2;

/// The fields of [`Profile`], written down where a human can see them.
///
/// This is not documentation, it is the tripwire: `tests/save.rs` serialises a default profile
/// and compares the keys in the file against this list. Adding a field without deciding what an
/// old file's missing value becomes is exactly how a save format eats an evening, and this list
/// is what forces the decision.
pub const PROFILE_FIELDS: [&str; 8] = [
    "sorties_flown",
    "sorties_won",
    "titans_felled",
    "best_kills_in_a_sortie",
    "seconds_in_the_field",
    "cleared",
    "xp",
    "gear",
];

/// Where save games go — or [`None`], which means **this process does not touch the disk**.
///
/// A path and not player state, so a `Resource` is the correct shape here (the same argument
/// `mission::phase` makes for `MissionPhase`): there is one filesystem, and a player who saved
/// into a different directory than his squad mate would be the bug, not the feature.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct SaveDir(pub Option<PathBuf>);

impl SaveDir {
    /// Decides, once, whether this process saves and where.
    ///
    /// 1. **`DBT_SAVE_DIR`** — that directory. Empty or `off` switches saving off entirely.
    ///    This is the door a test and a headless evidence run come through.
    /// 2. **A test binary** (the executable sits in `target/*/deps/`) — off. Twenty integration
    ///    tests build the real app through `defeated_by_titan::app`, and a test suite that
    ///    quietly writes into the developer's `saves/` is a test suite that can read its own
    ///    leftovers next time. *"It must not write during tests unless a test asks it to."*
    /// 3. Otherwise **`<crate root>/saves/`**, and the game saves.
    pub fn discover() -> SaveDir {
        if let Ok(from_env) = std::env::var("DBT_SAVE_DIR") {
            let trimmed = from_env.trim();
            if trimmed.is_empty() || trimmed == "off" {
                return SaveDir(None);
            }
            return SaveDir(Some(PathBuf::from(trimmed)));
        }
        if in_a_test_binary() {
            return SaveDir(None);
        }
        SaveDir(Some(root().join("saves")))
    }

    pub fn path(&self) -> Option<&Path> {
        self.0.as_deref()
    }
}

/// Whether the running executable is a `cargo test` binary.
///
/// Not `cfg!(test)`: that is false inside `src/` when an **integration** test in `tests/` links
/// the library, which is precisely the case this has to catch. What is always true of a test
/// binary and never of the game is where it lives — `target/<profile>/deps/`.
fn in_a_test_binary() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.ends_with("deps")))
        .unwrap_or(false)
}

/// The crate's root, resolved the same way `data::assets_dir` resolves `assets/`: the directory
/// the binary was built from if it still exists, otherwise the directory the binary sits in.
/// A shipped executable has no `CARGO_MANIFEST_DIR` to go back to.
fn root() -> PathBuf {
    let built_from = Path::new(env!("CARGO_MANIFEST_DIR"));
    if built_from.is_dir() {
        return built_from.to_path_buf();
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `saves/player-3.ron`. The id is in the **file name**, so a directory listing already answers
/// "whose careers are on this machine" without opening anything.
pub fn profile_path(dir: &Path, player: PlayerId) -> PathBuf {
    dir.join(format!("player-{}.ron", player.0))
}

/// What came back off the disk, and what had to be decided on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub profile: Profile,
    /// How this profile came to be. Goes into the log verbatim.
    pub note: LoadNote,
    /// The fields the file did not have, filled from [`Profile::default`]. Empty for a file
    /// this build wrote itself. **Never silent** — [`super::load_profiles`] warns with the
    /// names in it.
    pub missing: Vec<&'static str>,
}

/// The five ways a load can go. Four of them produce a usable profile; none of them deletes
/// anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadNote {
    /// No file yet — a first sortie. Not an error and not a warning.
    Fresh,
    /// A file of this build's own schema, read whole.
    Current,
    /// An older schema, brought forward by [`migrate`].
    Migrated { from: u32 },
    /// The file could not be parsed. It has been **kept** under `keep`, and the career starts
    /// empty rather than the run refusing to start.
    Broken { keep: PathBuf, why: String },
    /// The file was written by a **newer** build. Nothing has been touched, and this process
    /// will not save over it.
    FromTheFuture { schema: u32 },
}

impl LoadNote {
    /// Whether this process may write this player's file back.
    ///
    /// `false` for exactly one case, and it is the one that matters: a file from a newer build.
    /// Writing over it would silently drop every field this build does not know about.
    pub fn may_write(&self) -> bool {
        !matches!(self, LoadNote::FromTheFuture { .. })
    }
}

/// The envelope, and it is deliberately its own type: [`SCHEMA`] is read **before** the profile,
/// so a file from the future is refused without its body ever being parsed against a shape it
/// does not have.
#[derive(Serialize)]
struct FileOut<'a> {
    schema: u32,
    profile: &'a Profile,
}

#[derive(Deserialize)]
struct EnvelopeIn {
    schema: u32,
}

/// The profile as it may appear **on disk** — every field optional, so the loader can name what
/// was absent instead of silently defaulting it.
///
/// This is the `serde(default)` that `src/save/` is allowed and the rest of the tree is not; the
/// module header says why.
#[derive(Deserialize, Default)]
#[serde(default)]
struct ProfileOnDisk {
    sorties_flown: Option<u32>,
    sorties_won: Option<u32>,
    titans_felled: Option<u32>,
    best_kills_in_a_sortie: Option<u32>,
    seconds_in_the_field: Option<f32>,
    cleared: Option<BTreeSet<String>>,
    xp: Option<u64>,
    gear: Option<BTreeMap<String, u32>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FileIn {
    profile: ProfileOnDisk,
}

/// The reader for the profile body, and the one line of RON configuration in the tree.
///
/// `IMPLICIT_SOME` is what lets a field the file simply does not have read back as `None`
/// instead of a parse error. Without it every optional field in [`ProfileOnDisk`] would demand
/// the literal `Some(2)` in the file — so a save written by hand, or by a build that had one
/// field fewer, would come back as `Expected option` and the whole career would be declared
/// broken. Measured the day this module was written: `(schema: 1, profile: (sorties_flown: 2))`
/// failed at column 37 for exactly that reason.
///
/// It applies to **this parse and nothing else**. `assets/data/*.ron` keeps `data`'s strict
/// reader, where a missing value still has to crash (rule 2).
fn lenient() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

/// Brings a parsed file forward to [`SCHEMA`] and reports what had to be invented.
///
/// **Where the next migration goes.** Today there is one shipped schema, so the single arm is
/// the field-filling itself; when `F-120` adds `xp` the arm for 1 is the place that says what
/// an XP-less profile is worth, in one function, next to the number it changes.
fn migrate(schema: u32, disk: ProfileOnDisk, xp: &XpTuning) -> (Profile, Vec<&'static str>) {
    let fallback = Profile::default();
    let mut missing = Vec::new();
    let mut take = |present: bool, name: &'static str| {
        if !present {
            missing.push(name);
        }
    };
    take(disk.sorties_flown.is_some(), "sorties_flown");
    take(disk.sorties_won.is_some(), "sorties_won");
    take(disk.titans_felled.is_some(), "titans_felled");
    take(disk.best_kills_in_a_sortie.is_some(), "best_kills_in_a_sortie");
    take(disk.seconds_in_the_field.is_some(), "seconds_in_the_field");
    take(disk.cleared.is_some(), "cleared");
    take(disk.xp.is_some(), "xp");
    take(disk.gear.is_some(), "gear");

    // Schema 1 — the first shipped shape. A field the file does not carry is the field's own
    // zero, and that is a real answer: a career with no record of a sortie has not flown one.
    // **The second schema's arm goes here**, as `if schema < 2 { .. }` before this line, so that
    // an old file walks forward one version at a time instead of through a growing `match` that
    // has to know every combination.
    debug_assert!(schema <= SCHEMA, "a future schema is refused before it ever reaches migrate");
    let profile = Profile {
        sorties_flown: disk.sorties_flown.unwrap_or(fallback.sorties_flown),
        sorties_won: disk.sorties_won.unwrap_or(fallback.sorties_won),
        titans_felled: disk.titans_felled.unwrap_or(fallback.titans_felled),
        best_kills_in_a_sortie: disk
            .best_kills_in_a_sortie
            .unwrap_or(fallback.best_kills_in_a_sortie),
        seconds_in_the_field: disk
            .seconds_in_the_field
            .unwrap_or(fallback.seconds_in_the_field),
        cleared: disk.cleared.unwrap_or(fallback.cleared),
        xp: disk.xp.unwrap_or(fallback.xp),
        gear: disk.gear.unwrap_or(fallback.gear),
    };
    let profile = if schema < 2 { schema_1_to_2(profile, disk.xp.is_some(), xp) } else { profile };
    (profile, missing)
}

/// **Schema 1 -> 2: the career gets the experience it already earned.**
///
/// Schema 1 is every file written before `F-120`, and it has no `xp:` at all. Filling it with
/// zero would be *correct* and would still be data loss in the way that matters: a player who
/// flew forty sorties yesterday would open the game at level 1 today. So the old record is
/// re-paid through [`xp_of_a_bare_career`] — the four numbers a schema-1 file carries are exactly
/// the four facts a sortie is paid for.
///
/// `had_xp` is checked rather than `profile.xp == 0`: a genuine schema-1 file that somebody
/// hand-edited an `xp:` into keeps its number, and a career that really is worth zero is not
/// re-invented on every load.
fn schema_1_to_2(mut profile: Profile, had_xp: bool, xp: &XpTuning) -> Profile {
    if !had_xp {
        profile.xp = xp_of_a_bare_career(
            profile.sorties_flown,
            profile.sorties_won,
            profile.titans_felled,
            profile.seconds_in_the_field,
            xp,
        );
    }
    profile
}

/// Renders a profile exactly as it goes on disk. **Public because the test asserts against the
/// bytes** — a round trip through this module's own `from_str` would pass with the format
/// broken in both directions (`FINDINGS.md` FIND-103).
pub fn render(profile: &Profile) -> String {
    let out = FileOut { schema: SCHEMA, profile };
    let pretty = ron::ser::PrettyConfig::new().struct_names(false);
    // The only failure a serializer of plain numbers and strings has is a non-string map key,
    // and there is none in `Profile`. If that ever changes the test above goes red first.
    ron::ser::to_string_pretty(&out, pretty).expect("a Profile has no unserialisable field")
        + "\n"
}

/// Reads one player's profile. **Never fails** — every way the file can be wrong has an answer
/// that keeps the file and lets the game start.
pub fn read_profile(dir: &Path, player: PlayerId, xp: &XpTuning) -> Loaded {
    let path = profile_path(dir, player);
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Loaded { profile: Profile::default(), note: LoadNote::Fresh, missing: Vec::new() }
        }
        Err(e) => {
            return Loaded {
                profile: Profile::default(),
                note: LoadNote::Broken { keep: path, why: e.to_string() },
                missing: Vec::new(),
            }
        }
    };

    let envelope: EnvelopeIn = match ron::de::from_str(&text) {
        Ok(v) => v,
        Err(e) => return keep_the_broken_one(&path, &format!("no readable `schema:` — {e}")),
    };
    if envelope.schema > SCHEMA {
        // Nothing is parsed, nothing is renamed, nothing will be written. The player's file
        // belongs to a build that understands it.
        return Loaded {
            profile: Profile::default(),
            note: LoadNote::FromTheFuture { schema: envelope.schema },
            missing: Vec::new(),
        };
    }

    let body: FileIn = match lenient().from_str(&text) {
        Ok(v) => v,
        Err(e) => return keep_the_broken_one(&path, &e.to_string()),
    };
    let (profile, missing) = migrate(envelope.schema, body.profile, xp);
    let note = if envelope.schema == SCHEMA {
        LoadNote::Current
    } else {
        LoadNote::Migrated { from: envelope.schema }
    };
    Loaded { profile, note, missing }
}

/// Moves an unreadable file aside instead of writing over it. **`F-200`: no data loss.**
///
/// The bytes stay on disk under `player-1.ron.broken`, so a player who lost a career can be
/// given it back by hand and a bug in this module leaves its own evidence behind. The next
/// write creates a fresh `player-1.ron` beside it.
fn keep_the_broken_one(path: &Path, why: &str) -> Loaded {
    let keep = path.with_extension("ron.broken");
    // A failed rename is not fatal: the career still starts, and `may_write` is true, so the
    // next sortie writes a fresh file — which is the same outcome, minus the copy.
    let _ = fs::rename(path, &keep);
    Loaded {
        profile: Profile::default(),
        note: LoadNote::Broken { keep, why: why.to_string() },
        missing: Vec::new(),
    }
}

/// Writes one player's profile. **Atomic:** a full temporary file, flushed, then a rename over
/// the target.
///
/// `rename` within one directory is the platform's atomic replace, so the file a reader sees is
/// either the whole old one or the whole new one — never a half-written one, whatever the
/// process does in between. Writing in place is how a save game becomes a truncated save game
/// when the game is closed at the wrong moment, and `F-200` does not allow that.
pub fn write_profile(dir: &Path, player: PlayerId, profile: &Profile) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = profile_path(dir, player);
    let scratch = path.with_extension("ron.writing");
    {
        use std::io::Write;
        let mut f = fs::File::create(&scratch)?;
        f.write_all(render(profile).as_bytes())?;
        // Without this the rename can land before the bytes do, and a power cut leaves an
        // atomically renamed empty file — which is worse than a half-written one, because it
        // parses.
        f.sync_all()?;
    }
    fs::rename(&scratch, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A literal, not `progress.ron`: these four tests are about the FILE — where it goes,
    /// what happens to a broken one — and none of them is about what a sortie earns.
    fn xp_tuning() -> XpTuning {
        XpTuning {
            per_sortie_flown: 1.0,
            per_sortie_won: 1.0,
            per_titan_felled: 1.0,
            per_minute_in_the_field: 1.0,
            difficulty_multipliers: BTreeMap::new(),
            without_a_difficulty: 1.0,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbt-save-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a temp dir");
        dir
    }

    #[test]
    fn a_test_binary_does_not_get_a_save_directory() {
        // This unit test IS a test binary, which is what makes the assertion meaningful:
        // `SaveDir::discover()` must decide "off" here, or `cargo test` starts writing into
        // the developer's `saves/`.
        assert!(in_a_test_binary(), "a cargo test binary lives in target/*/deps/");
        // The env var is the one thing that overrides it — and it is how tests and the
        // headless evidence run point somewhere harmless.
        assert_eq!(SaveDir::discover(), SaveDir(None));
    }

    #[test]
    fn the_file_name_carries_the_player_id() {
        let p = profile_path(Path::new("/tmp/x"), PlayerId(7));
        assert_eq!(p.file_name().unwrap(), "player-7.ron");
    }

    #[test]
    fn a_missing_file_is_a_first_sortie_and_not_an_error() {
        let dir = scratch("fresh");
        let got = read_profile(&dir, PlayerId(1), &xp_tuning());
        assert_eq!(got.note, LoadNote::Fresh);
        assert_eq!(got.profile, Profile::default());
        assert!(got.note.may_write());
    }

    #[test]
    fn a_file_from_a_newer_build_is_refused_and_left_alone() {
        let dir = scratch("future");
        let path = profile_path(&dir, PlayerId(1));
        let original = "(schema: 99, profile: (sorties_flown: 7, moons_visited: 3))";
        fs::write(&path, original).unwrap();

        let got = read_profile(&dir, PlayerId(1), &xp_tuning());
        assert_eq!(got.note, LoadNote::FromTheFuture { schema: 99 });
        assert!(!got.note.may_write(), "an old build must not write over a newer file");
        // ⭐ The bytes, not the struct: the file is still exactly what it was.
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn an_unreadable_file_is_kept_and_the_career_starts_empty() {
        let dir = scratch("broken");
        let path = profile_path(&dir, PlayerId(1));
        fs::write(&path, "this is not RON at all {{{").unwrap();

        let got = read_profile(&dir, PlayerId(1), &xp_tuning());
        let LoadNote::Broken { keep, .. } = &got.note else {
            panic!("expected Broken, got {:?}", got.note);
        };
        assert_eq!(fs::read_to_string(keep).unwrap(), "this is not RON at all {{{");
        assert!(!path.exists(), "the unreadable file is moved aside, not left in the way");
        assert_eq!(got.profile, Profile::default());
    }
}
