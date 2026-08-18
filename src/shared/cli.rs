//! The launch flags — **the doors that go around the main menu.**
//!
//! For someone who cannot click, a main menu is a wall without a door. That is why the
//! testing infrastructure comes **before** the features, not "when there is time"
//! (`prompts/init.md` §12).
//!
//! Parsed by hand instead of with `clap`: there are nine flags, and a dependency you do not
//! need is a dependency that one day does not build — on this machine a very concrete worry
//! (`docs/environment.md`).

use bevy::prelude::*;
use std::path::PathBuf;

/// What the game was started with. Sits there as a `Resource` so that every domain can read
/// it without knowing `main.rs`.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Cli {
    /// No window, fixed tick, runs [`Cli::ticks`] steps and exits with an exit code that
    /// says whether every `assert` held. **The only way on a machine without a graphics
    /// session** (§14).
    pub headless: bool,
    /// Empty field, one titan, infinite gas — for looking around.
    pub sandbox: bool,
    /// Straight into a mission, no menu.
    pub mission: Option<String>,
    /// **Start in the hub** — the main building you walk around in and start missions from
    /// (user, 2026-08-12). Wins over [`Cli::mission`]: the hub is where a sortie is *chosen*.
    ///
    /// ⚠️ **Since 2026-08-13 this is what a plain `cargo run` does**, and the flag is only the
    /// explicit way to say it. It was opt-in for one day, recorded as an assumption with a
    /// rollback point (`docs/FINDINGS.md` FIND-057 §5), and the user closed it: *„eine main
    /// lobby in der man die mission starten kann"* — a front door you have to ask for is not a
    /// front door. [`hub_by_default`] is the whole rule and [`Cli::no_hub`] is the way back.
    pub hub: bool,
    /// `--no-hub`: **the old behaviour** — start in `Briefing`, an empty session with no hub
    /// furniture and no live trigger volumes. What every `--script` run gets anyway.
    pub no_hub: bool,
    /// One run from a text file (§12b). With `assert` it becomes a test.
    pub script: Option<PathBuf>,
    /// For measuring. Under vsync every frame time is 16.6 ms — so "what does this cost?"
    /// measures the same ceiling six times over (§11).
    pub novsync: bool,
    /// Simulated latency in milliseconds. **Every movement feature is also checked at
    /// 200 ms** (bible T-019) — "feels good locally" is not an acceptance.
    pub lag_ms: u32,
    /// Upper bound for `--headless`. 0 means: until the script ends.
    pub ticks: u64,
    /// Where a PNG gets written. **The only road to evidence for 🟧**
    /// (`docs/ACCEPTANCE.md`: "no image, no 🟧, no exceptions"). Together with
    /// [`Cli::ticks`] a screenshot is thereby reproducible and scriptable instead of timed
    /// by hand — see [`crate::debug::screenshot`].
    pub image: Option<PathBuf>,
    /// **GPU on, but no window.** The third mode next to a window and `--headless`:
    /// `--headless` switches `backends: None` (`crate::base_plugins`), and without an
    /// adapter there is no image. `--offscreen` lets wgpu work and renders into an `Image`.
    pub offscreen: bool,
    /// Export every model again (§7).
    pub reexport: bool,
    /// Skip the export, save startup time.
    pub no_export: bool,
    /// What was not understood. Reported **loudly** at startup — a mistyped flag that is
    /// quietly ignored costs an hour of debugging at the wrong end.
    pub unknown: Vec<String>,
}

impl Cli {
    pub fn from_argv() -> Self {
        Self::from_args(std::env::args().skip(1))
    }

    pub fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut s = Cli::default();
        let mut it = args.into_iter().peekable();
        while let Some(a) = it.next() {
            let mut value = |s: &mut Cli, name: &str| -> Option<String> {
                match it.next() {
                    Some(v) if !v.starts_with("--") => Some(v),
                    other => {
                        s.unknown.push(format!("{name} without a value"));
                        if let Some(v) = other {
                            // Do not swallow it: the next flag is a flag.
                            s.unknown.push(v);
                        }
                        None
                    }
                }
            };
            match a.as_str() {
                "--headless" => s.headless = true,
                "--offscreen" => s.offscreen = true,
                "--sandbox" => s.sandbox = true,
                "--hub" => s.hub = true,
                "--no-hub" => s.no_hub = true,
                "--novsync" => s.novsync = true,
                "--reexport" => s.reexport = true,
                "--no-export" => s.no_export = true,
                "--mission" => s.mission = value(&mut s, "--mission"),
                "--script" => s.script = value(&mut s, "--script").map(PathBuf::from),
                "--screenshot" => s.image = value(&mut s, "--screenshot").map(PathBuf::from),
                "--lag" => {
                    if let Some(v) = value(&mut s, "--lag") {
                        match v.parse() {
                            Ok(n) => s.lag_ms = n,
                            Err(_) => s.unknown.push(format!("--lag {v} is not a number")),
                        }
                    }
                }
                "--ticks" => {
                    if let Some(v) = value(&mut s, "--ticks") {
                        match v.parse() {
                            Ok(n) => s.ticks = n,
                            Err(_) => s.unknown.push(format!("--ticks {v} is not a number")),
                        }
                    }
                }
                other => s.unknown.push(other.to_string()),
            }
        }
        // A run without a window needs no more flags than necessary: whoever drives a script
        // and has no graphics session means --headless.
        //
        // **Unless he said `--offscreen`.** `--headless` switches the wgpu adapter off; this
        // line would therefore choke exactly the mode you picked in order to get an image
        // without a window — and do it quietly.
        if script_forces_headless(s.script.is_some(), s.offscreen, has_display()) {
            s.headless = true;
        }
        // **The front door.** A run that names no other door starts in the hub.
        if hub_by_default(s.no_hub, s.mission.is_some(), s.sandbox, s.script.is_some()) {
            s.hub = true;
        }
        s
    }

    /// Whether a window should be opened at all.
    pub fn wants_window(&self) -> bool {
        !self.headless && !self.offscreen
    }

    /// Whether there is a GPU, i.e. whether anything gets rendered at all.
    ///
    /// Only `--headless` switches it off (`backends: None`). `--offscreen` has no window
    /// **surface**, but it very much has an adapter — that is the whole difference between
    /// the two (`docs/QUESTIONS.md` Q-009).
    pub fn has_gpu(&self) -> bool {
        !self.headless
    }
}

/// Whether a run with no door named on the command line starts **in the hub**.
///
/// > *„und eine main lobby in der man die mission starten kann"* — the user, 2026-08-13.
///
/// A function of its own so the rule is one readable line and so it is checkable without
/// building an app. Three doors turn it off, and each for its own reason:
///
/// - `--mission <name>` names a sortie, so it is not asking for a place to choose one;
/// - `--sandbox` is the empty field for looking around, which a hub full of trigger volumes is
///   not;
/// - 🔴 **`--script <file>`, and this is the load-bearing one.** Twenty-eight of the thirty-five
///   scripts in `scripts/` name no mission, two of them assert `phase == 0` (`p1-overlay.txt`,
///   `p1-no-overlay.txt`), and every one of them would suddenly run inside a hub with live
///   deployment pads and supply stations in it — `f-018-gas.txt` measures a tank in a world that
///   would now have somewhere to refill it. None of them could be re-run in the session that
///   made this change. **A script run therefore keeps the old behaviour**, and a script that
///   wants the hub says `--hub`, which `scripts/f070-hub.txt` already does.
///
/// [`Cli::no_hub`] turns it off explicitly for the case none of the three covers.
fn hub_by_default(no_hub: bool, has_mission: bool, sandbox: bool, has_script: bool) -> bool {
    !no_hub && !has_mission && !sandbox && !has_script
}

/// Whether a script run is quietly switched over to `--headless`.
///
/// A function of its own, so that the rule is checkable **without environment variables**: a
/// test that sets `WAYLAND_DISPLAY` checks the machine and not the rule — and disturbs every
/// test running in parallel while it is at it.
fn script_forces_headless(hat_skript: bool, offscreen: bool, has_session: bool) -> bool {
    hat_skript && !offscreen && !has_session
}

/// Whether there is a graphics session.
///
/// Without one, a windowed start panics immediately and deep inside winit — a message that
/// looks like a bug in the game. **Better to check first and say one sentence people
/// understand** (§12d, `docs/environment.md`).
pub fn has_display() -> bool {
    let placed = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    placed("WAYLAND_DISPLAY") || placed("DISPLAY")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::from_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn flags_are_parsed() {
        let s = parse(&["--sandbox", "--novsync", "--lag", "200", "--ticks", "600"]);
        assert!(s.sandbox);
        assert!(s.novsync);
        assert_eq!(s.lag_ms, 200);
        assert_eq!(s.ticks, 600);
        assert!(s.unknown.is_empty(), "unexpected: {:?}", s.unknown);
    }

    #[test]
    fn the_hub_is_a_door_of_its_own_and_needs_no_mission_name() {
        // The hub picks the mission at a pad, so `--hub` carries no value. Without this arm
        // the flag lands in `unknown`, the run prints "unknown launch arguments" and starts in
        // `Briefing` — a game that looks like it ignored you.
        let s = parse(&["--headless", "--hub"]);
        assert!(s.hub);
        assert!(s.mission.is_none(), "--hub names no mission");
        assert!(s.unknown.is_empty(), "unexpected: {:?}", s.unknown);
    }

    /// ★ **The front door, 2026-08-13.** A plain `cargo run` starts where the missions are
    /// chosen — *„eine main lobby in der man die mission starten kann"*.
    #[test]
    fn a_run_that_names_no_door_starts_in_the_hub() {
        assert!(parse(&[]).hub, "a plain `cargo run` has to land in the hub");
        assert!(parse(&["--headless", "--ticks", "240"]).hub, "so does a plain headless run");
        assert!(!parse(&["--no-hub"]).hub, "--no-hub is the way back to the old behaviour");
        assert!(parse(&["--no-hub"]).unknown.is_empty(), "and it is a known flag");
        assert!(!parse(&["--mission", "tutorial"]).hub, "a named mission is its own door");
        assert!(!parse(&["--sandbox"]).hub, "the sandbox is not a hub");
    }

    /// 🔴 **A script run keeps the old behaviour, and this test is the reason it does.**
    ///
    /// Twenty-eight of the thirty-five scripts name no mission and two of them assert
    /// `phase == 0`. Starting them in a hub would put live deployment pads and refuel stations
    /// into runs that measure gas and count phases — and not one of them could be re-run on
    /// the day the default changed.
    #[test]
    fn a_script_run_does_not_get_the_hub_by_surprise() {
        let s = parse(&["--headless", "--script", "scripts/p1-overlay.txt", "--ticks", "400"]);
        assert!(!s.hub, "a script run must stay in Briefing unless it asked for the hub");
        let asked = parse(&["--headless", "--hub", "--script", "scripts/f070-hub.txt"]);
        assert!(asked.hub, "…and a script that asks for it still gets it");

        // Order: (--no-hub, has mission, sandbox, has script)
        assert!(hub_by_default(false, false, false, false), "nothing named: the hub");
        assert!(!hub_by_default(true, false, false, false), "--no-hub wins");
        assert!(!hub_by_default(false, true, false, false), "--mission wins");
        assert!(!hub_by_default(false, false, true, false), "--sandbox wins");
        assert!(!hub_by_default(false, false, false, true), "--script wins");
    }

    #[test]
    fn a_typo_flag_is_reported_and_not_swallowed() {
        // The edge case, not the normal case: flags that are quietly ignored cost an hour
        // of debugging at the wrong end.
        let s = parse(&["--sanbox"]);
        assert_eq!(s.unknown, vec!["--sanbox".to_string()]);
        assert!(!s.sandbox);
    }

    #[test]
    fn a_missing_value_does_not_eat_the_next_flag() {
        let s = parse(&["--mission", "--sandbox"]);
        assert!(s.mission.is_none());
        assert!(!s.unknown.is_empty());
        // --sandbox must not have vanished as a mission name
        assert!(s.unknown.iter().any(|u| u == "--sandbox"));
    }

    #[test]
    fn lag_with_nonsense_does_not_silently_become_zero() {
        let s = parse(&["--lag", "lots"]);
        assert_eq!(s.lag_ms, 0);
        assert!(s.unknown.iter().any(|u| u.contains("not a number")));
    }

    /// ⚠️ **`Cli::default()` and "no arguments" stopped being the same thing on 2026-08-13.**
    ///
    /// `hub` is now derived from what was *not* said, so a flagless command line differs from
    /// the struct default in exactly one field — and that difference is the front door. The
    /// struct default stays as it was on purpose: every test in this repository builds its
    /// `Cli` with `..default()`, and a default that started a hub would have moved several
    /// hundred of them into a world with live trigger volumes in it.
    #[test]
    fn empty_args_yield_the_defaults_except_the_front_door() {
        assert_eq!(parse(&[]), Cli { hub: true, ..Cli::default() });
        assert!(!Cli::default().hub, "the struct default stays Briefing — the tests live on it");
    }

    #[test]
    fn image_and_ticks_yield_a_reproducible_screenshot_job() {
        // Without a file extension, because `tools/norms.py` forbids asset paths in code
        // (§7). What is checked is the flag anyway, not the file name.
        let path = "docs/images/t006-world-far";
        let s = parse(&["--screenshot", path, "--ticks", "110"]);
        assert_eq!(s.image, Some(PathBuf::from(path)));
        assert_eq!(s.ticks, 110);
        assert!(s.unknown.is_empty(), "unexpected: {:?}", s.unknown);
    }

    #[test]
    fn offscreen_is_no_window_but_a_gpu() {
        // The whole point of the third mode: --headless takes the GPU away and with it
        // every chance of an image; --offscreen takes away only the window.
        let o = parse(&["--offscreen"]);
        assert!(!o.wants_window());
        assert!(o.has_gpu());

        let h = parse(&["--headless"]);
        assert!(!h.wants_window());
        assert!(!h.has_gpu());

        let f = parse(&[]);
        assert!(f.wants_window());
        assert!(f.has_gpu());
    }

    #[test]
    fn a_script_run_does_not_silently_turn_offscreen_into_headless() {
        // Without this exception, a machine with no graphics session would switch off
        // exactly the mode you picked in order to get an image without a window.
        // Order: (has script, offscreen, has session)
        assert!(script_forces_headless(true, false, false), "a script without a session means headless");
        assert!(!script_forces_headless(true, true, false), "--offscreen stays --offscreen");
        assert!(!script_forces_headless(true, false, true), "with a session the window stays");
        assert!(!script_forces_headless(false, false, false), "without a script nothing changes");
    }
}
