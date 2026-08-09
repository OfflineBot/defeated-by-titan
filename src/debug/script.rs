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
//! hook left                   # hook out
//! wait 1.2                    # commands are deferred — otherwise you photograph an empty field
//! mark eingehakt              # a line in the log to line a screenshot up against
//! assert speed > 25           # ⭐ the script may judge for itself: if it falls over, it is a test
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
    /// `hook left|right <seconds>` — hold a real mouse button
    Hook { right: bool, duration_s: f32 },
    /// `wait <seconds>`
    Wait(f32),
    /// `mark <text>`
    Mark(String),
    /// `assert <metric> <comparison> <value>`
    Assert { metric: Metric, comparison: Comparison, value: f32 },
    /// `end` — stop early
    End,
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
        "hook" => {
            let side = *t.get(1).ok_or("`left` or `right` is missing")?;
            let right = match side {
                "left" => false,
                "right" => true,
                other => return Err(format!("side {other:?} — allowed: left, right")),
            };
            Ok(ScriptCommand::Hook {
                right,
                // Without a value: one tick. A hook is a tap, not autofire.
                duration_s: if t.len() > 2 { number(t.get(2), "duration")? } else { 0.05 },
            })
        }
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
                other => {
                    return Err(format!(
                        "metric {other:?} is not measurable — known: \
                         speed, height, gas, titans, tick, health, kills, phase"
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
        "end" => Ok(ScriptCommand::End),
        other => Err(format!("unknown command {other:?}")),
    }
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
        other => {
            return Err(format!(
                "key {other:?} is unknown — known: W A S D Q E C F F3 Space Shift Ctrl"
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
    fn an_unknown_metric_still_lists_what_is_known() {
        // The error message is the only place a script author finds the vocabulary.
        let f = parse("assert cloud == 1").expect_err("`cloud` measures nothing");
        assert!(f[0].reason.contains("not measurable"));
        for known in ["health", "kills", "phase"] {
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
    fn comparisons_evaluate_correctly() {
        assert!(Comparison::Greater.holds(26.0, 25.0));
        assert!(!Comparison::Greater.holds(25.0, 25.0));
        assert!(Comparison::GreaterEqual.holds(25.0, 25.0));
        assert!(Comparison::LessEqual.holds(25.0, 25.0));
        assert!(Comparison::Equal.holds(25.0005, 25.0), "floating point needs a tolerance");
        assert!(!Comparison::Equal.holds(25.1, 25.0));
    }
}
