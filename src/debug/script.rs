//! The `--script` driver: playing the game without typing.
//!
//! **This is the point where projects like this fail:** everything is built, nothing is
//! seen, because every feature sits behind mouse and keyboard and nobody is at the
//! keyboard. So the test infrastructure comes **before** the features
//! (`prompts/init.md` §12).
//!
//! One text file, one instruction per line:
//!
//! ```text
//! spawn titan husk 20 0 -40   # kind and position in meters
//! look 0 -10                  # look direction in degrees (yaw, pitch)
//! key Space 0.3               # hold the key for 0.3 s
//! hook left                   # hook out — the rope, on `Q`/`E`
//! slash right 0.2             # the blade, on the right mouse button
//! wait 1.2                    # commands are deferred — otherwise you photograph an empty field
//! mark eingehakt              # a line in the log to line a screenshot up against
//! assert speed > 25           # ⭐ the script may judge for itself: if it falls over, it is a test
//! settings assist_catch 100    # this MACHINE's aim assist, 0..100 % — not a player's
//! ```
//!
//! `assert` is the reason this is more than a demo: it turns a **run** into a test — and
//! movement feel is exactly the kind of thing no unit test gets hold of.
//!
//! The driver writes into **the same inputs a human triggers**
//! (`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`) — **no second, wrong way to
//! play.** The one exception is the look: for that there is a "pretend" vector, because a
//! mouse knows no absolute angle.

use bevy::prelude::*;

/// One instruction from the script file, with the line number for the error message.
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub line: usize,
    pub command: ScriptCommand,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScriptCommand {
    /// `spawn titan <kind> <x> <y> <z>`
    SpawnTitan { kind: String, pos: Vec3 },
    /// `warp <x> <y> <z>` — the player stands exactly there afterwards (§12c)
    Warp(Vec3),
    /// `look <yaw_deg> <pitch_deg>`
    Look { yaw_deg: f32, pitch_deg: f32 },
    /// `key <name> <seconds>` — hold a real key
    Key { code: KeyCode, duration_s: f32 },
    /// `hook left|right <seconds>` — hold the real rope key, `Q` or `E`.
    ///
    /// ⚠️ It pressed `MouseButton::Left`/`Right` until 2026-08-10. The user moved the ropes
    /// onto the keyboard so they can be **steered** while aiming (`src/net/local.rs`), and the
    /// mouse buttons became the blades. The verb kept its name because a dozen scripts say it
    /// and it still says what it means; only the button underneath moved. A script that was
    /// not repointed on that day swung a sword where it meant to fire a rope, silently.
    Hook { right: bool, duration_s: f32 },
    /// `slash left|right <seconds>` — hold a real mouse button.
    ///
    /// The counterpart to [`ScriptCommand::Hook`], and it exists because `parse_key` cannot
    /// reach a mouse button at all: after the rebinding the blades live on `LMB`/`RMB`, and
    /// without this verb no script could cut with the right blade at all (`KeyF` is a second
    /// binding for the LEFT one only).
    Slash { right: bool, duration_s: f32 },
    /// `wait <seconds>`
    Wait(f32),
    /// `mark <text>`
    Mark(String),
    /// `assert <metric> <comparison> <value>`
    Assert { metric: Metric, comparison: Comparison, value: f32 },
    /// `settings <key> <value>` — move one row of this **machine's** [`PlayerSettings`].
    ///
    /// ⚠️ **A machine setting and not a player's.** `shared::PlayerSettings` is a `Resource`:
    /// it is what the person at *this* keyboard has set, and `vector::aim` applies the aim
    /// assist to `With<LocalPlayer>` only (`docs/FINDINGS.md` FIND-104, `Q-038`). A script line
    /// therefore changes the local machine's preference — it is **not** addressed at a player
    /// and there is no way to give two players different knobs from a script. The day the
    /// knobs travel they move into `Intent`, and this verb grows a
    /// player argument with them.
    Settings { key: Setting, value: f32 },
    /// `end` — stop early
    End,
}

/// Which row of [`PlayerSettings`](crate::shared::PlayerSettings) a `settings` line moves.
///
/// **Deliberately only the two aim-assist knobs**, and the reason is the same one
/// [`Metric`] gives for its own shortness: a key that changes nothing a script can measure is
/// not a script verb, it is a settings screen. These two are the only fields of
/// `PlayerSettings` that (a) change what the *simulation* does and (b) have no other route out
/// of a script — the mouse sensitivity and the FOV are a device and a picture, and `look`
/// bypasses the first one anyway.
///
/// Both are percentages, `0..100`, and `0` is defined as the absence of the feature
/// (`F-016`): `settings assist_strength 0` is bit-for-bit the free aim that shipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// `assist_catch` — how far off the crosshair the hook may still catch, 0..100 %.
    /// 100 % is `shared::settings::ASSIST_CATCH_MAX_DEG` (20°) off the look direction.
    AssistCatch,
    /// `assist_strength` — how much better a candidate has to be than the point you are
    /// really aiming at, 0..100 %. 0 % never snaps (FREI), 100 % needs no margin (SNAP).
    AssistStrength,
}

impl Setting {
    /// The word a script writes.
    pub fn key(self) -> &'static str {
        match self {
            Setting::AssistCatch => "assist_catch",
            Setting::AssistStrength => "assist_strength",
        }
    }

    /// Writes the value into the machine's settings. **One field, one writer** — the driver is
    /// the only caller, and it runs in `FixedPreUpdate` where the settings screen does not.
    pub fn apply(self, s: &mut crate::shared::PlayerSettings, value: f32) {
        match self {
            Setting::AssistCatch => s.assist_catch_pct = value,
            Setting::AssistStrength => s.assist_strength_pct = value,
        }
    }
}

/// What an `assert` can measure. Deliberately few and **all of them measurable** — a
/// metric nobody can recompute is not a test criterion (§17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    /// Speed of the local player in m/s
    Speed,
    /// Height of the local player in meters
    Height,
    /// Gas of the local player, absolute
    Gas,
    /// Number of living Titans
    Titans,
    /// The simulation tick
    Tick,
    /// Health of the local player, absolute (`shared::Health.current`).
    ///
    /// Nothing spawns player health yet (that is `P5`). Until then the metric measures
    /// **nothing**, and "nothing" counts as failed — see [`crate::debug`]`::measure`. That is
    /// the safe direction: an `assert health > 0` that passed on a missing component would be
    /// the exact silent lie this whole file exists to prevent.
    Health,
    /// Titans killed in the running mission, **for the local player**.
    ///
    /// The vocabulary came first on purpose: Round 2 wrote its acceptance criteria in it
    /// (`docs/PLAN-GAME.md` §8, `F-071`), and a criterion you cannot write down is a criterion
    /// nobody checks. Since `F-071` it reads the `KillTally` of the running mission; **without
    /// a mission it measures nothing**, and nothing counts as failed.
    Kills,
    /// Which phase the mission is in, as a number:
    /// `0` Briefing · `1` Deploying · `2` Active · `3` Won · `4` Lost.
    ///
    /// The mapping lives with the enum (`mission::MissionPhase::code`), not here: a script that
    /// means `Lost` and gets `Won` because somebody inserted a variant in the middle is a green
    /// run that measured the opposite of what it says.
    Phase,
    /// How many of the local player's hook arms are **anchored** right now — `0`, `1` or `2`
    /// ([`Hook::anchored_count`](crate::shared::Hook::anchored_count)).
    ///
    /// **The first metric that observes the Vector Gear itself.** Until it existed, a script
    /// that wanted to say "the rope was still on him when the blade went through the nape"
    /// had to argue from the GAS LEDGER: reel gas is debited only while `REEL_IN` is held
    /// **and** an arm is anchored (`src/vector/gas.rs`), so a falling tank implies a rope.
    /// That is a proxy with about five ticks of resolution — `scripts/f-flight-cut.txt` names
    /// the gap in its own header — and it cannot say *on which tick*. This one can.
    ///
    /// A count and not a yes/no, because `F-001`'s two hooks are independent: `== 2` is a
    /// sentence a script has to be able to write.
    Rope,
    /// Spare blade pairs in the local player's harness
    /// ([`Blades::pairs_left`](crate::shared::Blades)).
    ///
    /// ⚠️ **Today this number can only ever go up.** The resupply half of `F-033` is built and
    /// wired (`blades::resupply`, `mission::hub::restock_at_stations`) **and so is wear**, since
    /// 2026-08-13: `blades::cut` books `gear.ron: blades.wear_per_hit` on every `TitanHit`, a
    /// non-cortex zone costs `× wear_torso_factor`, `swap_pair` draws a spare at zero and
    /// `is_broken()` makes `cut` cast nothing at all (`docs/FINDINGS.md` FIND-079).
    ///
    /// ⚠️ **`blades` is still the wrong metric for a resupply claim, and that has been measured.**
    /// It counts *pairs left*, and a whole sortie spends about **0.24** sharpness — a fought kill
    /// costs 0.12 for the cortex plus half that for the shoulder the blade meets on the way in,
    /// an arranged fall-cut only the 0.12 — so `pairs_left` never leaves 5 and
    /// `assert blades == 5` stays true whatever happens. **Use [`Metric::Sharpness`]**: that is
    /// the number that moves. `scripts/f070-hub.txt` was rewritten around it and now goes red two
    /// ways (skip the rack: `Sharpness > 0.99 — measured 0.760`; remove the cuts:
    /// `Sharpness < 0.8 — measured 1.000`).
    Blades,
    /// Condition of the pair in the local player's hands, `0.0`..`1.0`
    /// ([`Blades::sharpness`](crate::shared::Blades)).
    ///
    /// **This is the metric a resupply claim should use** — see the note on [`Metric::Blades`]
    /// for why `blades` cannot go red and this can.
    Sharpness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
}

impl Comparison {
    pub fn holds(self, actual: f32, expected: f32) -> bool {
        match self {
            Comparison::Greater => actual > expected,
            Comparison::GreaterEqual => actual >= expected,
            Comparison::Less => actual < expected,
            Comparison::LessEqual => actual <= expected,
            // Floating point is not compared for equality; for meters, m/s and gas a
            // tolerance of 1e-3 is the order of magnitude called "does not matter".
            Comparison::Equal => (actual - expected).abs() <= 1e-3,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Comparison::Greater => ">",
            Comparison::GreaterEqual => ">=",
            Comparison::Less => "<",
            Comparison::LessEqual => "<=",
            Comparison::Equal => "==",
        }
    }
}

/// An error while reading the file — **with a line number**. A script that silently skips
/// a line is worse than one that does not run at all: the run then looks green and has
/// left half of it undone.
#[derive(Debug, PartialEq)]
pub struct ScriptError {
    pub line: usize,
    pub text: String,
    pub reason: String,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {} — {:?}", self.line, self.reason, self.text)
    }
}

pub fn parse(content: &str) -> Result<Vec<Instruction>, Vec<ScriptError>> {
    let mut done = Vec::new();
    let mut errors = Vec::new();

    for (i, raw) in content.lines().enumerate() {
        let line = i + 1;
        let without_comment = raw.split('#').next().unwrap_or("").trim();
        if without_comment.is_empty() {
            continue;
        }
        match parse_line(without_comment) {
            Ok(command) => done.push(Instruction { line, command }),
            Err(reason) => errors.push(ScriptError {
                line,
                text: raw.trim().to_string(),
                reason,
            }),
        }
    }

    if errors.is_empty() { Ok(done) } else { Err(errors) }
}

fn number(t: Option<&&str>, command: &str) -> Result<f32, String> {
    t.ok_or_else(|| format!("{command} is missing"))?
        .parse()
        .map_err(|_| format!("{command} is not a number: {:?}", t.unwrap()))
}

fn parse_line(line: &str) -> Result<ScriptCommand, String> {
    let t: Vec<&str> = line.split_whitespace().collect();
    match t.first().copied().unwrap_or("") {
        "spawn" => {
            if t.get(1).copied() != Some("titan") {
                return Err("only `spawn titan <kind> <x> <y> <z>` is known".into());
            }
            let kind = t.get(2).ok_or("titan kind is missing")?.to_string();
            Ok(ScriptCommand::SpawnTitan {
                kind,
                pos: Vec3::new(
                    number(t.get(3), "x")?,
                    number(t.get(4), "y")?,
                    number(t.get(5), "z")?,
                ),
            })
        }
        "warp" => Ok(ScriptCommand::Warp(Vec3::new(
            number(t.get(1), "x")?,
            number(t.get(2), "y")?,
            number(t.get(3), "z")?,
        ))),
        "look" => Ok(ScriptCommand::Look {
            yaw_deg: number(t.get(1), "yaw")?,
            pitch_deg: number(t.get(2), "pitch")?,
        }),
        "key" => Ok(ScriptCommand::Key {
            code: parse_key(t.get(1).ok_or("key name is missing")?)?,
            duration_s: number(t.get(2), "duration")?,
        }),
        // The two triggers of the Vector Gear. Same shape on purpose: they are the same
        // gesture on two different devices since the rebinding, and a script author who knows
        // one knows the other.
        "hook" => Ok(ScriptCommand::Hook {
            right: side(&t)?,
            duration_s: hold(&t)?,
        }),
        "slash" => Ok(ScriptCommand::Slash {
            right: side(&t)?,
            duration_s: hold(&t)?,
        }),
        "wait" => Ok(ScriptCommand::Wait(number(t.get(1), "duration")?)),
        "mark" => {
            let text = t[1..].join(" ");
            if text.is_empty() {
                return Err("`mark` without text — a mark without a name is not a mark".into());
            }
            Ok(ScriptCommand::Mark(text))
        }
        "assert" => {
            let metric = match *t.get(1).ok_or("metric is missing")? {
                "speed" => Metric::Speed,
                "height" => Metric::Height,
                "gas" => Metric::Gas,
                "titans" => Metric::Titans,
                "tick" => Metric::Tick,
                "health" => Metric::Health,
                "kills" => Metric::Kills,
                // `phase` became measurable with `F-070`: the mission state machine it reads
                // exists since 2026-08-09. Until then it was refused on purpose — a parser
                // that accepted it would have handed the mission round a green run that
                // measured nothing.
                "phase" => Metric::Phase,
                // `rope` reads `shared::Hook`, which has been on every player since `F-001` —
                // so unlike `phase` this one never had to be refused. What it replaces is a
                // proxy, not a hole: see [`Metric::Rope`].
                "rope" => Metric::Rope,
                // The harness. Both read `shared::Blades`, which `hud` already draws, so no
                // domain edge was bought for them — `debug -> blades` is not on the allow list
                // of `docs/architecture.md` and none was needed.
                "blades" => Metric::Blades,
                "sharpness" => Metric::Sharpness,
                other => {
                    return Err(format!(
                        "metric {other:?} is not measurable — known: \
                         speed, height, gas, titans, tick, health, kills, phase, rope, \
                         blades, sharpness"
                    ));
                }
            };
            let comparison = match *t.get(2).ok_or("comparison is missing")? {
                ">" => Comparison::Greater,
                ">=" => Comparison::GreaterEqual,
                "<" => Comparison::Less,
                "<=" => Comparison::LessEqual,
                "==" => Comparison::Equal,
                other => return Err(format!("comparison {other:?} — allowed: > >= < <= ==")),
            };
            Ok(ScriptCommand::Assert {
                metric,
                comparison,
                value: number(t.get(3), "comparison value")?,
            })
        }
        // The one verb that reaches `shared::PlayerSettings`. Without it the whole aim assist
        // (`F-016`/`F-024`/`F-025`) was unreachable from every script in the repository, and
        // `F-025`'s own acceptance — a chain that accelerates over five swaps **with the snap
        // on** — could not be run at all.
        "settings" => {
            let key = match *t.get(1).ok_or("setting key is missing")? {
                "assist_catch" => Setting::AssistCatch,
                "assist_strength" => Setting::AssistStrength,
                other => {
                    return Err(format!(
                        "{other:?} is not a setting a script may move — known: \
                         assist_catch, assist_strength"
                    ));
                }
            };
            let value = number(t.get(2), "setting value")?;
            // ⚠️ **Refused, not clamped.** A line that asked for 150 % and silently got 100 %
            // would leave a run measuring something other than what it says — the same failure
            // the whole error list above exists to prevent.
            if !(0.0..=100.0).contains(&value) {
                return Err(format!(
                    "{} is a percentage and {value} is outside 0..100",
                    key.key()
                ));
            }
            Ok(ScriptCommand::Settings { key, value })
        }
        "end" => Ok(ScriptCommand::End),
        // The list is spelled out and not left to a reader of this file: the error message is
        // the only place a script author finds the vocabulary, and `slash` was invisible for
        // exactly as long as it was missing from here.
        other => Err(format!(
            "unknown command {other:?} — known: \
             spawn, warp, look, key, hook, slash, wait, mark, assert, settings, end"
        )),
    }
}

/// `left`/`right` after a `hook` or a `slash` — `true` means right.
fn side(t: &[&str]) -> Result<bool, String> {
    match *t.get(1).ok_or("`left` or `right` is missing")? {
        "left" => Ok(false),
        "right" => Ok(true),
        other => Err(format!("side {other:?} — allowed: left, right")),
    }
}

/// The optional hold time of a `hook`/`slash`. Without a value: one tick — a trigger is a
/// tap, not autofire.
fn hold(t: &[&str]) -> Result<f32, String> {
    if t.len() > 2 { number(t.get(2), "duration") } else { Ok(0.05) }
}

/// Only the keys the game really uses. A complete table would be three hundred lines
/// nobody maintains — whoever binds a new key adds it here.
fn parse_key(name: &str) -> Result<KeyCode, String> {
    Ok(match name {
        "W" | "w" => KeyCode::KeyW,
        "A" | "a" => KeyCode::KeyA,
        "S" | "s" => KeyCode::KeyS,
        "D" | "d" => KeyCode::KeyD,
        "Q" | "q" => KeyCode::KeyQ,
        "E" | "e" => KeyCode::KeyE,
        "C" | "c" => KeyCode::KeyC,
        "F" | "f" => KeyCode::KeyF,
        "Space" | "space" => KeyCode::Space,
        // The overlay toggle. Without it a script cannot switch the numbers into its own
        // screenshot — and a HUD nobody can photograph cannot reach 🟧.
        "F3" | "f3" => KeyCode::F3,
        "Shift" | "shift" => KeyCode::ShiftLeft,
        "Ctrl" | "ctrl" => KeyCode::ControlLeft,
        // `MARK` since the rebinding of 2026-08-10 (`src/net/local.rs`). Without it a script
        // cannot press MARK at all — the button moved off `Q` and `Q` is a rope now.
        "Tab" | "tab" => KeyCode::Tab,
        other => {
            return Err(format!(
                "key {other:?} is unknown — known: W A S D Q E C F F3 Space Shift Ctrl Tab"
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_from_the_docs_parses() {
        let s = "\
spawn titan husk 20 0 -40   # kind and position in meters
look 0 -10
key Space 0.3
hook left
wait 1.2
mark eingehakt
assert speed > 25
";
        let a = parse(s).expect("the example must be parseable");
        assert_eq!(a.len(), 7);
        assert_eq!(
            a[0].command,
            ScriptCommand::SpawnTitan { kind: "husk".into(), pos: Vec3::new(20.0, 0.0, -40.0) }
        );
        assert_eq!(a[5].command, ScriptCommand::Mark("eingehakt".into()));
        assert_eq!(
            a[6].command,
            ScriptCommand::Assert { metric: Metric::Speed, comparison: Comparison::Greater, value: 25.0 }
        );
    }

    #[test]
    fn comments_and_blank_lines_vanish_without_error() {
        let a = parse("# just a comment\n\n   \nmark here\n").expect("valid");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].line, 4, "the line number must be the REAL one");
    }

    #[test]
    fn a_typo_is_reported_and_not_skipped() {
        // The real purpose of the error list: a script that silently skips a line looks
        // green and has left half of it undone.
        let f = parse("mark one\nspwan titan husk 0 0 0\nmark two\n")
            .expect_err("must fail");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, 2);
        assert!(f[0].reason.contains("unknown command"));
    }

    #[test]
    fn all_errors_come_at_once() {
        let f = parse("assert cloud > 1\nkey Umlaut 1\nhook high\n").expect_err("three errors");
        assert_eq!(f.len(), 3, "not just the first error, all of them");
    }

    #[test]
    fn missing_numbers_are_an_error_not_a_zero() {
        let f = parse("wait\n").expect_err("duration is missing");
        assert!(f[0].reason.contains("is missing"));
        let f = parse("look 0\n").expect_err("pitch is missing");
        assert!(f[0].reason.contains("pitch"));
        let f = parse("warp 1 two 3\n").expect_err("not a number");
        assert!(f[0].reason.contains("not a number"));
    }

    #[test]
    fn hook_without_a_duration_is_a_tap_not_autofire() {
        let a = parse("hook right").expect("valid");
        assert_eq!(a[0].command, ScriptCommand::Hook { right: true, duration_s: 0.05 });
    }

    #[test]
    fn slash_is_a_verb_and_it_mirrors_hook_exactly() {
        // The blades moved onto the mouse on 2026-08-10 and `parse_key` cannot reach a mouse
        // button, so without this verb `SLASH_RIGHT` had no route out of a script at all.
        let a = parse("slash left\nslash right 0.2\n").expect("`slash` is a verb");
        assert_eq!(a[0].command, ScriptCommand::Slash { right: false, duration_s: 0.05 });
        assert_eq!(a[1].command, ScriptCommand::Slash { right: true, duration_s: 0.2 });
        // Same error shape as `hook`, out of the same helper.
        let f = parse("slash high").expect_err("`high` is not a side");
        assert!(f[0].reason.contains("allowed: left, right"), "{:?}", f[0].reason);
    }

    #[test]
    fn an_unknown_command_lists_the_vocabulary_it_does_have() {
        // An error message that names no vocabulary is the reason a script author guesses —
        // and `slash` is exactly the verb that would stay invisible.
        let f = parse("swing left\n").expect_err("`swing` is not a command");
        assert!(f[0].reason.contains("unknown command"), "{:?}", f[0].reason);
        for known in ["hook", "slash", "key", "assert"] {
            assert!(
                f[0].reason.contains(known),
                "the error has to list {known:?}: {:?}",
                f[0].reason
            );
        }
    }

    #[test]
    fn tab_is_a_key_a_script_can_press() {
        // `MARK` sits on `Tab` since the rebinding — `Q` is a rope now. Without this arm no
        // script can press MARK at all.
        assert_eq!(parse_key("Tab"), Ok(KeyCode::Tab));
        assert_eq!(parse_key("tab"), Ok(KeyCode::Tab));
    }

    #[test]
    fn mark_without_text_is_not_a_mark() {
        assert!(parse("mark").is_err());
        assert!(parse("mark   # just a comment").is_err());
    }

    #[test]
    fn the_metrics_the_mission_rounds_need_are_already_in_the_vocabulary() {
        // `health` and `kills` parse today, long before anything writes them — otherwise
        // Round 2 would have to change the parser and its own feature in the same breath, and
        // nobody would be able to tell afterwards which of the two the run was measuring.
        for (line, expected) in [
            ("assert health > 0", Metric::Health),
            ("assert kills >= 3", Metric::Kills),
        ] {
            let a = parse(line).unwrap_or_else(|e| panic!("{line:?}: {e:?}"));
            let ScriptCommand::Assert { metric, .. } = a[0].command else {
                panic!("{line:?} did not become an assert");
            };
            assert_eq!(metric, expected);
        }
    }

    #[test]
    fn phase_became_a_metric_with_the_mission_state_machine() {
        // It was refused until `F-070` existed. Now it parses — and the number it compares
        // against is the one `mission::MissionPhase::code` writes down, not a fresh one.
        let a = parse("assert phase == 4").expect("`phase` is measurable since F-070");
        let ScriptCommand::Assert { metric, comparison, value } = a[0].command else {
            panic!("`assert phase == 4` did not become an assert");
        };
        assert_eq!(metric, Metric::Phase);
        assert_eq!(comparison, Comparison::Equal);
        assert_eq!(value, 4.0);
    }

    #[test]
    fn rope_is_a_metric_and_counts_up_to_two() {
        // `== 2` has to be writable: `F-001`'s hooks are independent, so a yes/no would lose
        // the one distinction the two arms exist for. That the number really follows the
        // component is `tests/debug.rs::assert_rope_counts_the_anchored_arms_and_is_not_a_constant`.
        let a = parse("assert rope >= 1\nassert rope == 2\n").expect("`rope` is measurable");
        let ScriptCommand::Assert { metric, comparison, value } = a[0].command else {
            panic!("`assert rope >= 1` did not become an assert");
        };
        assert_eq!(metric, Metric::Rope);
        assert_eq!(comparison, Comparison::GreaterEqual);
        assert_eq!(value, 1.0);
        let ScriptCommand::Assert { value, .. } = a[1].command else { panic!("not an assert") };
        assert_eq!(value, 2.0);
    }

    #[test]
    fn an_unknown_metric_still_lists_what_is_known() {
        // The error message is the only place a script author finds the vocabulary.
        let f = parse("assert cloud == 1").expect_err("`cloud` measures nothing");
        assert!(f[0].reason.contains("not measurable"));
        for known in ["health", "kills", "phase", "rope"] {
            assert!(
                f[0].reason.contains(known),
                "the error has to list {known:?}: {:?}",
                f[0].reason
            );
        }
    }

    #[test]
    fn f3_is_a_key_a_script_can_press() {
        // Without it a script cannot switch the overlay into its own screenshot.
        assert_eq!(parse_key("F3"), Ok(KeyCode::F3));
        assert_eq!(parse_key("f3"), Ok(KeyCode::F3));
    }

    #[test]
    fn the_settings_verb_reaches_the_two_assist_knobs() {
        // **The gap `F-025` could not be tested through.** The driver had 46 verbs and none of
        // them touched `shared::PlayerSettings`, so the whole aim assist was unreachable from
        // every script in the repository — and its own acceptance ("a chain over 5 swaps with
        // the snap on") could not be run at all.
        let a = parse("settings assist_catch 100\nsettings assist_strength 55\n")
            .expect("`settings` is a verb");
        assert_eq!(
            a[0].command,
            ScriptCommand::Settings { key: Setting::AssistCatch, value: 100.0 }
        );
        assert_eq!(
            a[1].command,
            ScriptCommand::Settings { key: Setting::AssistStrength, value: 55.0 }
        );
    }

    #[test]
    fn an_unknown_settings_key_is_an_error_and_not_a_line_that_does_nothing() {
        // The failure mode this whole file exists to prevent: a script line that is quietly
        // ignored leaves the run green and half of it undone (§6 rule 2's spirit — a missing
        // value crashes, it does not default).
        let f = parse("settings assist_agression 100\n").expect_err("that key does not exist");
        assert!(f[0].reason.contains("not a setting"), "{:?}", f[0].reason);
        for known in ["assist_catch", "assist_strength"] {
            assert!(
                f[0].reason.contains(known),
                "the error has to list {known:?}: {:?}",
                f[0].reason
            );
        }
        // And a value outside the slider's own window is refused at PARSE time, not clamped
        // silently: a script that asked for 150 % and got 100 % measured something other than
        // what it says.
        let f = parse("settings assist_catch 150\n").expect_err("150 % is not a percentage");
        assert!(f[0].reason.contains("0..100"), "{:?}", f[0].reason);
        let f = parse("settings assist_strength\n").expect_err("the value is missing");
        assert!(f[0].reason.contains("is missing"), "{:?}", f[0].reason);
    }

    #[test]
    fn the_vocabulary_in_the_error_message_lists_settings_too() {
        // `slash` was invisible for exactly as long as it was missing from that list.
        let f = parse("setting assist_catch 100\n").expect_err("`setting` is not a command");
        assert!(f[0].reason.contains("settings"), "{:?}", f[0].reason);
    }

    #[test]
    fn a_settings_line_moves_the_field_it_names_and_no_other() {
        let mut s = knobs_at_zero();
        Setting::AssistCatch.apply(&mut s, 100.0);
        assert_eq!(s.assist_catch_pct, 100.0);
        assert_eq!(s.assist_strength_pct, 0.0, "the other knob may not move");
        Setting::AssistStrength.apply(&mut s, 100.0);
        assert_eq!(s.assist_strength_pct, 100.0);
        // Independent of the code under test: 100 % catch is the 20 deg end stop, and
        // `assist_is_on` needs BOTH knobs off zero.
        assert!(s.assist_is_on());
        assert!((s.assist_catch_deg() - 20.0).abs() < 1e-4);
    }

    /// A `PlayerSettings` with both knobs at zero, built without `GameData`.
    fn knobs_at_zero() -> crate::shared::PlayerSettings {
        crate::shared::PlayerSettings {
            mouse_deg_per_px: 0.08,
            invert_y: false,
            fov_deg: 60.0,
            pitch_limit_deg: 89.0,
            assist_catch_pct: 0.0,
            assist_strength_pct: 0.0,
        }
    }

    #[test]
    fn comparisons_evaluate_correctly() {
        assert!(Comparison::Greater.holds(26.0, 25.0));
        assert!(!Comparison::Greater.holds(25.0, 25.0));
        assert!(Comparison::GreaterEqual.holds(25.0, 25.0));
        assert!(Comparison::LessEqual.holds(25.0, 25.0));
        assert!(Comparison::Equal.holds(25.0005, 25.0), "floating point needs a tolerance");
        assert!(!Comparison::Equal.holds(25.1, 25.0));
    }
}
