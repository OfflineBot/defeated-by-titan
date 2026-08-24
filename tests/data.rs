//! The guard over the RON files.
//!
//! **Numbers belong in RON, not in Rust** (`prompts/init.md` §4) — and that is exactly why
//! the RON needs a test. A number in code is caught by the compiler; a number in a file is
//! caught by nobody, except here.
//!
//! What is checked is not "is it pretty", but **what the bible lays down as binding** and
//! **what has to be consistent with itself** (every reference points at something that
//! exists).

use defeated_by_titan::data::{GameData, GasConsumer, TitanScale};
use std::path::PathBuf;

fn data() -> GameData {
    GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

#[test]
fn t005_all_ron_files_load() {
    // No `serde(default)` for game values: a missing value is meant to crash. This test is
    // the place where that crash is not an aborted game but a red test.
    let d = data();
    assert!(!d.titans.kinds.is_empty(), "titan.ron without a single kind");
    assert!(!d.art.models.is_empty(), "art.ron without a single model");
    assert!(!d.maps.maps.is_empty(), "maps.ron without a single map");
    assert!(!d.maps.palette.is_empty(), "maps.ron without a single color");
}

#[test]
fn t005_every_titan_winds_up_at_least_four_tenths_of_a_second() {
    // Bible, pillar P4 (readability before realism): "every titan attack has a windup of at
    // least 0.4 seconds". That is not a recommendation — the player must never have to ask
    // why he died.
    for (name, kind) in &data().titans.kinds {
        assert!(
            kind.windup_s >= 0.4,
            "{name}: windup_s = {} — the bible demands at least 0.4 s",
            kind.windup_s
        );
    }
}

#[test]
fn t005_every_titan_points_at_a_model_that_exists() {
    // A reference into nothing is a bug of the same class as a dead link in the docs (§10).
    // Otherwise it only surfaces when exactly this titan gets spawned.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        assert!(
            d.art.models.contains_key(&kind.model),
            "{name} points at model {:?}, which is not in art.ron",
            kind.model
        );
    }
}

#[test]
fn t005_every_wave_names_a_titan_kind_that_exists() {
    let d = data();
    for (mission, template) in &d.missions.templates {
        for wave in &template.waves {
            assert!(
                d.titans.kinds.contains_key(&wave.kind),
                "mission {mission:?}: wave at {}s wants {:?}, which is not in titan.ron",
                wave.at_s,
                wave.kind
            );
            assert!(wave.count > 0, "mission {mission:?}: a wave with zero titans");
        }
    }
}

#[test]
fn t005_a_mission_arc_lasts_five_to_seven_minutes() {
    // Bible 5, change 10: average mission length 5–7 min. Every mission has to be a complete
    // arc with progress you can feel.
    for (name, v) in &data().missions.templates {
        assert!(
            (300.0..=420.0).contains(&v.target_duration_s),
            "{name}: target_duration_s = {} — the bible wants 5–7 min (300–420 s)",
            v.target_duration_s
        );
        assert!(
            v.waves.iter().all(|w| w.at_s <= v.target_duration_s),
            "{name}: a wave spawns after the end of the mission"
        );
    }
}

#[test]
fn t005_the_simulation_runs_at_sixty_hertz() {
    // §6 rule 4: a fixed simulation step. Over the network a frame-dependent result is not a
    // comfort problem but a desync — and by then it is too late to change it.
    let hz = data().game.simulation_hz;
    assert!((hz - 60.0).abs() < 1e-9, "simulation_hz = {hz}, expected 60");
}

#[test]
fn t005_hook_range_stays_in_the_design_window() {
    // The window WAS 60..=120, straight out of `prompts/init.md` §3 („Eine Einheit"): *„Ein
    // Mensch ist 1,8, ein Titan 3–15, ein Haken fliegt 60–120."* (Q-002 cites that sentence as
    // §1; it stands in §3, item 5. The sentence is what matters, not the section number.)
    //
    // ⚠️ **Widened to 60..=200 on 2026-08-10 — deliberately, and against the project's own
    // source document.** The user played the build and said *"zudem muss die hook range sehr
    // viel länger sein!"*. This project has a precedence rule for exactly this collision, and
    // it is written down twice (docs/QUESTIONS.md Q-002, assets/data/scale.ron:vector): **a
    // direct figure in metres from the user beats any derivation** — and a live instruction
    // beats the number it replaces, including the user's own earlier 90 m. So the window moved
    // and the value did not get trimmed back into it. The whole decision, with the rollback
    // point, is docs/QUESTIONS.md Q-035.
    //
    // The ceiling is 200 and not "no ceiling": the graybox is 400 m across, so 200 m is
    // exactly half the map. Beyond that every anchor in the world is always in reach and
    // *where you stand stops being a decision* — the map would no longer be a design surface.
    // A guard with no upper bound is not a guard, it is a comment.
    //
    // ⚠️ **Widened again to 60..=500 on 2026-08-12, and the ceiling this test wrote down as
    // "the user's call, not a tuning step" is exactly what he called.** Verbatim
    // (`docs/NEXT.md` §1A): *„und das seil muss deutlich deutlich schneller gespannt werden.
    // nicht frame perfekt aber mit ca 500m pro sekunde. **mit der range 500 meter!**"*. Third
    // time the same precedence rule decides it and third time in the same direction — a direct
    // figure in metres from the user beats every derivation, including his own 90 and his own
    // 200.
    //
    // **What made the old ceiling movable is that its argument had already expired.** "Half
    // the 400 m graybox" was written against a map that has not shipped since 2026-08-12;
    // ashgate is 700 m across, and 500 m is 71 % of that edge, not 125 % of it. So the
    // sentence the ceiling defended — *where you stand is a decision* — survives the widening,
    // but only just, and the lever that keeps it true from here is **the map, not this
    // number**: a district under ~1000 m across cannot make a 500 m rope positional again.
    //
    // The new ceiling is 500 and still not "no ceiling", and it is bounded from the outside as
    // well: `world.half_extent_m` has to carry half the longest map plus one full range
    // (`t005_the_grid_covers_map_and_hook_range`), so every metre added here is paid for in
    // index memory. Moving it a fourth time is the user's call, not a tuning step — and it is
    // now also a `half_extent_m` change, which that test will say out loud.
    let r = data().game.vector.hook_range_m;
    assert!(
        (60.0..=500.0).contains(&r),
        "hook_range_m = {r} — the window is 60..=500 m. Its floor is init.md §3 („ein Haken \
         fliegt 60–120\"); its ceiling was half the 400 m graybox (200) until 2026-08-12, when \
         the user named the range himself: \"mit der range 500 meter!\" (docs/NEXT.md §1A). \
         Against ashgate's 700 m edge that is 71 % of the district, so position still decides \
         something — above 500 it stops deciding anything, and the map, not this key, is what \
         has to grow first. Moving this ceiling again is the user's call, not a tuning step"
    );
}

#[test]
fn t005_a_hook_shot_at_full_range_arrives_before_the_target_has_moved() {
    // **The guard that was missing on 2026-08-10.** `hook_range_m` went 90 -> 200 because the
    // user asked for it; `hook_speed_m_s` had to go 90 -> 160 with it, and nothing in this
    // file would have noticed if it had not. That is the dangerous shape: the range is the
    // number somebody tunes, the speed is the number nobody thinks of, and the symptom is not
    // a crash but "the hook feels sluggish now".
    //
    // The fact nobody had pinned: **a hook is a projectile, so range / speed is how long the
    // worst-case shot takes to arrive.** 90 m at 90 m/s cost 1.0 s. 200 m at 160 m/s costs
    // 1.25 s. At the old 90 m/s the new range would have cost **2.22 s** — and this assert is
    // what turns that into a red test instead of a feel complaint three sessions later.
    //
    // Why the ceiling is 1.5 s: at `max_speed_m_s` (75) the player himself covers 112 m in
    // 1.5 s. Past that the anchor you aimed at is more than half a hook range behind you by
    // the time the hook gets there, so aiming stops being aiming and becomes leading a target
    // — a different game, and not one anybody decided to build. 1.5 s leaves the current
    // 1.25 s some room without leaving room for a doubling.
    let v = &data().game.vector;
    let flight_s = v.hook_range_m / v.hook_speed_m_s;
    assert!(
        flight_s <= 1.5,
        "a shot to maximum range takes {flight_s} s ({} m at {} m/s) — the ceiling is 1.5 s. \
         A hook is a projectile: range / speed is its worst-case flight time. If the range \
         grew, the speed has to grow with it (2026-08-10: 90 m/90 m/s = 1.0 s became \
         200 m/160 m/s = 1.25 s; leaving the speed at 90 would have meant 2.22 s). \
         docs/QUESTIONS.md Q-035",
        v.hook_range_m, v.hook_speed_m_s
    );
    // And the same fact from the other side, with no literal in it: while the hook flies, the
    // player is still moving. If he can outrun a full hook range during his own shot, the
    // anchor point he picked is meaningless by the time the hook lands.
    let drift_m = v.max_speed_m_s * flight_s;
    assert!(
        drift_m < v.hook_range_m,
        "at max_speed_m_s = {} the player travels {drift_m} m during the {flight_s} s flight \
         of a maximum-range shot — that is further than the {} m range itself. The hook would \
         be out of range of its own anchor before it arrives",
        v.max_speed_m_s, v.hook_range_m
    );
}

#[test]
fn t005_a_missed_hook_is_back_inside_one_second() {
    // **New on 2026-08-13, and the guard `hook_retract_speed_m_s` never had.** Until today the
    // only rule on it was a sentence in `game.ron` — "faster than the outward flight, so a
    // missed shot is not punished" — and that rule died the moment both numbers became 500.
    // "Faster than instant" is not a bound, and a key whose bound has expired is a key that
    // drifts.
    //
    // What replaces it is the requirement out of `docs/NEXT.md` §1A itself: hooking is
    // *„instant"*, and the first thing that makes hooking feel non-instant is not the flight
    // out — it is not being allowed to fire again because the last shot is still crawling
    // home. `hook_range_m / hook_retract_speed_m_s` is exactly how long that block can last in
    // the worst case, and one second is where a miss stops being a miss and becomes a
    // punishment.
    //
    // 1.0 s and not 1.5 s (the outward ceiling) on purpose: the outward flight is time the
    // player CHOSE to spend and can watch — „man soll sehen wie es aufspannt". The retract is
    // time he did not choose and cannot use. The number that buys him nothing gets the
    // tighter bound.
    let v = &data().game.vector;
    let retract_s = v.hook_range_m / v.hook_retract_speed_m_s;
    assert!(
        retract_s <= 1.0,
        "a miss at maximum range takes {retract_s} s to come back ({} m at {} m/s) — the \
         ceiling is 1.0 s. That is dead time the player did not choose, and §1A's first \
         requirement is that firing again is never blocked. If the range grew, the retract \
         speed has to grow with it (2026-08-12: 200 m/120 m/s = 1.67 s became \
         500 m/500 m/s = 1.0 s; leaving the retract at 120 would have meant 4.17 s)",
        v.hook_range_m, v.hook_retract_speed_m_s
    );
}

#[test]
fn t005_the_rope_pull_is_the_second_strongest_thrust_in_the_game() {
    // `docs/NEXT.md` §1A, requirement 4: *„dass man dort richtig hingezogen wird"* — and the
    // whole point of the key is that it is **more** than the air control a rope-less player
    // already has. Below `air_accel_m_s2` the feature is a rename.
    //
    // The upper bound is the one all nine judges of the §1B plan hit independently, from the
    // other side: a free thrust that beats `boost_m_s2` makes the gas boost pointless, and
    // "flight costs gas" is the game.
    let d = data();
    let p = &d.game.player;
    let boost = d.game.vector.boost_m_s2;
    assert!(
        p.air_pull_m_s2 > p.air_accel_m_s2,
        "air_pull_m_s2 = {} is not above air_accel_m_s2 = {} — then holding W on a rope is \
         what holding W in free air already does, and „deutlich mehr geboosted\" is untrue",
        p.air_pull_m_s2, p.air_accel_m_s2
    );
    assert!(
        p.air_pull_m_s2 <= boost,
        "air_pull_m_s2 = {} is above boost_m_s2 = {boost} — the thrust that costs nothing \
         would beat the thrust that costs gas",
        p.air_pull_m_s2
    );
    // The lateral is a SUM bound, and that is the difference between the two: W and D can be
    // held at the same time, so what must not beat one boost is `look + strafe`, not the
    // strafe alone.
    assert!(
        p.air_lateral_m_s2 >= p.air_accel_m_s2,
        "air_lateral_m_s2 = {} is below air_accel_m_s2 = {} — A/D on a rope would be weaker \
         than A/D without one, and §1A calls it a „boost\"",
        p.air_lateral_m_s2, p.air_accel_m_s2
    );
    let together = p.air_accel_m_s2 + p.air_lateral_m_s2;
    assert!(
        together <= boost,
        "air_accel_m_s2 + air_lateral_m_s2 = {together} exceeds boost_m_s2 = {boost} — \
         holding one strafe key would out-thrust a boost you paid for"
    );
}

#[test]
fn t005_the_rope_pull_lets_go_before_the_length_cliff() {
    // **The one bound in this group that is measured rather than argued.**
    // `docs/FINDINGS.md` FIND-035: at `min_rope_m` the length constraint takes 17 m/s out of
    // the player in a single tick. `air_pull_m_s2` thrusts straight at the anchor, i.e.
    // straight into that cliff, so the fade has to be over before the cliff starts — and
    // "before" has to be measured in the same unit the cliff is: rope length.
    let d = data();
    let fade = d.game.player.air_pull_fade_m;
    let min_rope = d.game.vector.min_rope_m;
    assert!(
        fade >= 2.0 * min_rope,
        "air_pull_fade_m = {fade} is under 2 * min_rope_m = {} — the fade would be shorter \
         than the cliff it exists to avoid (FIND-035: 17 m/s lost in one tick at min_rope_m)",
        2.0 * min_rope
    );
    assert!(
        fade <= 0.1 * d.game.vector.hook_range_m,
        "air_pull_fade_m = {fade} is over a tenth of hook_range_m = {} — a „near the anchor\" \
         special case that runs over a tenth of every rope in the game is not a special case, \
         and the pull would stop being what steers a swing",
        d.game.vector.hook_range_m
    );
}

#[test]
fn t005_rope_steering_costs_what_the_boost_costs_per_metre_per_second() {
    // The flaw all nine judges of the §1B plan named, made into a number: the new thrust must
    // not be free, and it must not be a third price point either. `gas_dodge` already
    // established the only honest way to compare two thrusts in this game — **gas per m/s of
    // speed bought**, not gas per second:
    //
    //     held boost   gas_boost_per_s / boost_m_s2    = 18 / 34 = 0.529
    //     rope steer   gas_steer_per_s / air_pull_m_s2 = 16 / 30 = 0.533
    //
    // 0.15 of tolerance is wide on purpose: this pins the *shape* of the decision (which
    // thrust the situation wants), not the tuning (how much either costs). At 0.15 the steer
    // could cost 28 % more or less per m/s than the boost and still pass — beyond that a
    // player stops choosing on situation and starts choosing on price, and the mixing rule
    // becomes a gas-saving trick.
    let d = data();
    let v = &d.game.vector;
    let boost_ratio = v.gas_boost_per_s / v.boost_m_s2;
    let steer_ratio = v.gas_steer_per_s / d.game.player.air_pull_m_s2;
    let delta = (steer_ratio - boost_ratio).abs();
    assert!(
        delta <= 0.15,
        "rope steering costs {steer_ratio} gas per m/s ({} / {}), the held boost {boost_ratio} \
         ({} / {}) — a difference of {delta} against a tolerance of 0.15. Then the cheap \
         thrust wins on price instead of on situation",
        v.gas_steer_per_s, d.game.player.air_pull_m_s2, v.gas_boost_per_s, v.boost_m_s2
    );
    // And the trivial half nobody would think to write down: a rate of 0 is a free thrust,
    // which is the whole thing this key exists to prevent.
    assert!(
        v.gas_steer_per_s > 0.0,
        "gas_steer_per_s = {} — the rope thrust would be free, and „flight costs gas\" is the \
         game",
        v.gas_steer_per_s
    );
}

#[test]
fn t005_the_gas_tank_is_the_value_the_user_asked_for_and_names_its_dependents() {
    // **This test exists because of `docs/FINDINGS.md` FIND-073**, which was written when
    // `gas_tank` went 100 -> 300 and three scripts were still asserting 100 two days later:
    //
    //   "a tuning value that triples silently invalidates every script that quoted it, and
    //    nothing in the build says which those are."
    //
    // Nothing was added then to make the build say which. On 2026-08-20 the same value moved
    // again — 300 -> 15000, a 50x, on the user's „mach das 50 fache!" (docs/QUESTIONS.md
    // Q-046) — and it invalidated 54 `assert gas` lines across 13 scripts, two tests and six
    // comment blocks. **So the build says which now, and it says it here.**
    //
    // This is a PIN, not a balance guard. It does not claim 15000 is right — it claims that
    // whoever changes it has been shown the list. To roll back to `gas_tank: 300.0`, change
    // the literal below and work the list.
    let v = &data().game.vector;
    const PINNED_GAS_TANK: f32 = 15000.0;
    assert_eq!(
        v.gas_tank,
        PINNED_GAS_TANK,
        "{}",
        tank_rollback_checklist(v.gas_tank, PINNED_GAS_TANK)
    );
}

// ── the checklist, as data rather than as prose ────────────────────────────────────────────
//
// **Grouped by HOW a line fails when the tank moves, because that is the only grouping that
// helps the person doing the move.** The dangerous group is not "the ones that break" — those
// announce themselves. It is the ones that stop failing:
//
//   * RAISE the tank and a LOWER bound goes quiet. `scripts/f070-hub.txt:156` was
//     `assert gas > 299` — F-070's ⭐ "the tank is full again". At 15000 the player walks in
//     holding 14964 and that line passes with `refuel_at_stations` deleted from the build
//     (FIND-142, measured).
//   * LOWER it and an UPPER bound goes quiet. `assert gas < 15000` is a literal copy of the
//     tank size; at `gas_tank: 300.0` it holds no matter what, and it was measured on
//     2026-08-20 in a sandboxed `assets/` copy: 300 tank, both burn rates zeroed, gas never
//     spent at all — `scripts/game-full.txt` reported **24 asserts held, fully green**.
//
// The four lists below are checked against `scripts/` itself by
// `t005_every_script_that_asserts_gas_is_on_the_tank_checklist`, so this cannot fall behind
// the directory again — which is the whole of what FIND-073 complained about.
const TANK_SCRIPTS_EXACT: &[&str] = &[
    "f-007-boost.txt",
    "f-018-gas.txt",
    "f003-ashgate.txt",
    "f003-ruins.txt",
    "f004-towers.txt",
    "f018-budget.txt",
    "f025-chain.txt",
    "w5-lane.txt",
];
const TANK_SCRIPTS_DELTA: &[&str] = &[
    "f-007-boost.txt",
    "f-018-gas.txt",
    "f-flight-cut.txt",
    "f003-ashgate.txt",
    "f008-dash.txt",
    "f018-budget.txt",
    "f019-supply.txt",
    "f070-hub.txt",
    "f170-hud.txt",
];
const TANK_SCRIPTS_QUIET_ON_RAISE: &[&str] = &[
    "f-flight-cut.txt",
    "f070-hub.txt",
    // ⚠️ Both of the 2026-08-24 scripts are in TWO groups, and they belong in both.
    //
    // `f008-dash.txt` brackets `15000 - 3 x gas_dodge = 14865` to ±10 — a DELTA, the width has
    // to survive and the centre has to move. Its opening `assert gas > 14999` is the other
    // kind: raise the tank and that bound stops failing without ever going red.
    //
    // `f019-supply.txt` is the same shape twice over: `assert gas < 14900` is what says the
    // tank was DRAINED (a DELTA off 15000 minus 2.3 s of boost and three dashes), and
    // `assert gas > 14999` is what says the station filled it again — and THAT one is the
    // dangerous kind squared, because a refill that overshoots and a tank that was raised look
    // identical to it.
    "f008-dash.txt",
    "f019-supply.txt",
];
const TANK_SCRIPTS_QUIET_ON_LOWER: &[&str] = &["f-001-hooks.txt", "game-full.txt"];

fn as_paths(names: &[&str]) -> String {
    names.iter().map(|n| format!("scripts/{n}")).collect::<Vec<_>>().join(" · ")
}

fn tank_rollback_checklist(now: f32, pinned: f32) -> String {
    format!(
        "`game.ron: vector.gas_tank` moved from {pinned} to {now}. That is allowed — it is the \
         user's number (Q-046) — but it is never a one-line change. Every one of these quotes it \
         or is derived from it:\n\
         \n\
           EXACT-TANK ASSERTS (`assert gas == <tank>`), which go RED:\n\
             {exact}\n\
           DELTA BRACKETS (shift by the difference, keep the WIDTH):\n\
             {delta}\n\
           BOUNDS THAT GO QUIET INSTEAD OF RED — the dangerous ones:\n\
             raise the tank and a LOWER bound stops failing:\n\
               {quiet_up}\n\
             lower it and an UPPER bound stops failing (both are `assert gas < 15000`):\n\
               {quiet_down}\n\
           TESTS:\n\
             tests/vector_boost.rs::f008_the_dodge_is_the_expensive_boost_and_shift_is_the_cheap_one\n\
             tests/mission.rs::f072_gas_comes_back_at_a_station_and_nowhere_else\n\
             tests/vector_gas.rs (the downward-only floors: boost seconds, bursts, reel share)\n\
           PROSE THAT STATES IT AS FACT:\n\
             assets/data/game.ron (the gas_tank, gas_reel_per_s, gas_steer_per_s, gas_dodge\n\
               comment blocks) · src/vector/gas.rs (module doc) · docs/NEXT.md\n\
             docs/HANDOVER.md · docs/QUESTIONS.md Q-033/Q-044/Q-046\n\
             docs/gameplay/references.md (the AoT:R comparison line)\n\
         \n\
         A run whose gas asserts merely stopped failing has stopped measuring.",
        exact = as_paths(TANK_SCRIPTS_EXACT),
        delta = as_paths(TANK_SCRIPTS_DELTA),
        quiet_up = as_paths(TANK_SCRIPTS_QUIET_ON_RAISE),
        quiet_down = as_paths(TANK_SCRIPTS_QUIET_ON_LOWER),
    )
}

#[test]
fn t005_every_script_that_asserts_gas_is_on_the_tank_checklist() {
    // **The list above is only worth something if it is COMPLETE**, and on 2026-08-20 it was
    // not: it named 11 scripts while 13 carried an `assert gas` line. The two it missed —
    // `scripts/f-001-hooks.txt` and `scripts/game-full.txt` — were both `assert gas < 15000`,
    // i.e. both in its own "goes quiet instead of red" category. A hand-kept list of files is
    // exactly the thing FIND-073 said the build should stop relying on, so it is kept by the
    // directory instead: every `scripts/*.txt` with a line beginning `assert gas` has to be
    // named in one of the four groups, and every name has to still exist.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let mut on_disk: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("scripts/ is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a script is readable");
        if text.lines().any(|line| line.starts_with("assert gas")) {
            on_disk.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    on_disk.sort();

    let mut named: Vec<String> = TANK_SCRIPTS_EXACT
        .iter()
        .chain(TANK_SCRIPTS_DELTA)
        .chain(TANK_SCRIPTS_QUIET_ON_RAISE)
        .chain(TANK_SCRIPTS_QUIET_ON_LOWER)
        .map(|n| (*n).to_owned())
        .collect();
    named.sort();
    named.dedup();

    let missing: Vec<&String> = on_disk.iter().filter(|n| !named.contains(n)).collect();
    let stale: Vec<&String> = named.iter().filter(|n| !on_disk.contains(n)).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "the gas-tank rollback checklist in this file no longer matches scripts/.\n\
         \n\
         NOT ON THE LIST but they assert gas — add each to the group that says how it fails when\n\
         the tank moves (RED / DELTA / QUIET-on-raise / QUIET-on-lower):\n\
           {missing:?}\n\
         ON THE LIST but no longer asserting gas — drop them:\n\
           {stale:?}\n\
         \n\
         The list is what `t005_the_gas_tank_is_the_value_the_user_asked_for_and_names_its_dependents`\n\
         prints when someone moves `game.ron: vector.gas_tank`, and a checklist that is short by two\n\
         files is how FIND-073 happened. Reproduce the directory side with:\n\
           grep -rlc '^assert gas' scripts/"
    );
}

#[test]
fn t005_no_value_is_zero_negative_or_nan() {
    // The edge case, not the normal one: a zero in a tank size is a division by zero three
    // systems later (§9d).
    let d = data();
    let v = &d.game.vector;
    let positive = [
        ("gas_tank", v.gas_tank),
        ("hook_speed_m_s", v.hook_speed_m_s),
        ("hook_retract_speed_m_s", v.hook_retract_speed_m_s),
        ("reel_speed_m_s", v.reel_speed_m_s),
        ("min_rope_m", v.min_rope_m),
        ("boost_m_s2", v.boost_m_s2),
        ("max_speed_m_s", v.max_speed_m_s),
        ("gas_steer_per_s", v.gas_steer_per_s),
        ("player.air_pull_m_s2", d.game.player.air_pull_m_s2),
        ("player.air_lateral_m_s2", d.game.player.air_lateral_m_s2),
        ("player.air_pull_fade_m", d.game.player.air_pull_fade_m),
        ("player.height_m", d.game.player.height_m),
        ("player.run_speed_m_s", d.game.player.run_speed_m_s),
        ("player.max_substep_m", d.game.player.max_substep_m),
        ("camera.fov_deg", d.game.camera.fov_deg),
        ("world.cell_m", d.game.world.cell_m),
        ("world.half_extent_m", d.game.world.half_extent_m),
        ("world.min_wall_m", d.game.world.min_wall_m),
        ("world.collision_margin_m", d.game.world.collision_margin_m),
    ];
    for (name, value) in positive {
        assert!(value.is_finite() && value > 0.0, "{name} = {value} — must be finite and > 0");
    }
    assert!(
        d.game.gravity_m_s2 < 0.0,
        "gravity_m_s2 = {} — downwards means negative, +Y is up",
        d.game.gravity_m_s2
    );
    for (name, kind) in &d.titans.kinds {
        let height = d
            .titan_height_m(kind)
            .unwrap_or_else(|| panic!("{name}: size class {:?} does not exist", kind.size_class));
        assert!(height > 0.0, "{name}: height_m = {height}");
        assert!(
            kind.cortex_radius_m > 0.0,
            "{name}: cortex_radius_m = 0 — a cortex that is a point feels like \
             a broken game (docs/models.md)",

        );
    }
}

#[test]
fn t005_eye_height_sits_inside_the_body() {
    let s = &data().game.player;
    assert!(
        s.eye_height_m > 0.0 && s.eye_height_m < s.height_m,
        "eye_height_m = {} does not fit height_m = {}",
        s.eye_height_m,
        s.height_m
    );
}

#[test]
fn t005_the_air_control_can_never_hold_a_player_up() {
    // `F-006`, and the bound the number used to carry in its own shape: until 2026-08-12 the
    // air control WAS `-gravity_m_s2 / 2` in `player::locomotion::air_control`, so "weaker than
    // gravity" was true by construction. Now that it is a key (§4, FIND-051), the guarantee has
    // to be a test — at or above `-gravity_m_s2` WASD alone lifts you, and gasless hovering
    // becomes free in a game whose whole gate is that flight costs gas.
    let d = data();
    let accel = d.game.player.air_accel_m_s2;
    let gravity = -d.game.gravity_m_s2;
    assert!(
        accel.is_finite() && accel > 0.0 && accel < gravity,
        "air_accel_m_s2 = {accel} — must be finite and in 0 < x < {gravity} (= -gravity_m_s2), \
         or WASD alone holds a player in the air"
    );
}

#[test]
fn t005_an_empty_tank_weakens_the_air_control_without_removing_it() {
    // Not a derivation, the user's spec: *„ohne gas kann man immernoch w a d nutzen um etwas
    // movement aufzubauen (aber hälfte ca)"* (docs/NEXT.md §1e). Above 1.0 an empty tank would
    // be STRONGER than a full one; below 0.0 it would thrust backwards. Both ends are the
    // whole check — the 0.5 in between is a tuning value and nobody's business here.
    let f = data().game.player.air_accel_empty_fraction;
    assert!(
        f.is_finite() && (0.0..=1.0).contains(&f),
        "air_accel_empty_fraction = {f} — a fraction lives in 0..=1; above 1 an empty tank \
         would be the strong one"
    );
}

#[test]
fn t005_a_substep_is_smaller_than_the_thinnest_wall() {
    // The core of the tunneling safeguard. At max_speed_m_s = 75 a 60 Hz tick covers 1.25 m;
    // without substeps the player drives through every wall — and only sometimes, which is
    // the worst kind of bug there is (F-012).
    let d = data();
    let substep = d.game.player.max_substep_m;
    let wall = d.game.world.min_wall_m;
    assert!(
        substep < wall,
        "max_substep_m = {substep} >= min_wall_m = {wall} — every substep can skip a wall"
    );
    assert!(
        d.game.world.collision_margin_m < substep,
        "collision_margin_m = {} must be smaller than one substep",
        d.game.world.collision_margin_m
    );
    // And the number of substeps at top speed has to stay finite and small.
    let substeps = (d.game.vector.max_speed_m_s / 60.0 / substep).ceil();
    assert!(
        (1.0..=32.0).contains(&substeps),
        "{substeps} substeps per tick at top speed — too many or none at all"
    );
}

#[test]
fn t005_the_grid_covers_map_and_hook_range() {
    // An anchor outside the grid lands in the border cell and is therefore in the wrong
    // place. The grid has to carry half the map PLUS one full hook range (T-036a).
    let d = data();
    let w = &d.game.world;
    assert!(w.cell_m > 0.0, "world.cell_m = {} — division by zero in the DDA", w.cell_m);
    assert!(w.large_body_cells >= 1, "large_body_cells = 0 puts every body into the linear list — that is exactly \
         the iteration §11 forbids");
    let mut tightest: f32 = 0.0;
    for (name, map) in &d.maps.maps {
        let needed = map.size_m.0.max(map.size_m.1) * 0.5
            + d.game.vector.hook_range_m;
        tightest = tightest.max(needed);
        assert!(
            w.half_extent_m >= needed,
            "{name}: half_extent_m = {} does not cover {needed} m ({} m map edge / 2 + \
             {} m hook_range_m). Raising the range raises this number with it — on \
             2026-08-12 the range went 200 -> 500 and this floor went 600 -> 850",
            w.half_extent_m,
            map.size_m.0.max(map.size_m.1),
            d.game.vector.hook_range_m
        );
    }
    // ⚠️ **And the ceiling, new on 2026-08-13.** Until today this guard had only a floor, and
    // a bound in one direction is not a guard: nothing here would have noticed a
    // `half_extent_m` of 5000, which is 1.56 million cells and 37 MB of empty `Vec` headers
    // for a district that needs 850 m.
    //
    // The ceiling is **25 % over the tightest map's requirement** and it is a memory statement,
    // because that is the only thing this number costs: `SpatialIndex::new` allocates
    // `columns²` cells once at startup and never per tick (`src/shared/spatial.rs`), and
    // `columns` is `2 * half_extent_m / cell_m`. So 25 % of slack is **1.56x the cells**, and
    // the measurement behind allowing even that much stands in `game.ron` next to the key: the
    // 600 -> 900 step (2.25x the cells) cost +0.5 % of user CPU over 900 headless ticks, inside
    // a 3 % run-to-run spread. Slack is cheap; unbounded slack is a leak nobody sees.
    let ceiling = tightest * 1.25;
    assert!(
        w.half_extent_m <= ceiling,
        "half_extent_m = {} is more than 25 % over the {tightest} m the widest map + one hook \
         range actually needs (ceiling {ceiling} m). The index is `columns²` cells with \
         `columns = 2 * half_extent_m / cell_m`, so this is quadratic: {} cells against the \
         {} the maps ask for. Grow the map or the range, not the padding",
        w.half_extent_m,
        (2.0 * w.half_extent_m / w.cell_m).ceil().powi(2),
        (2.0 * tightest / w.cell_m).ceil().powi(2)
    );
    // A cell bigger than the map would be a grid with one cell in it.
    assert!(w.cell_m < w.half_extent_m, "world.cell_m = {} is not a grid", w.cell_m);
}

#[test]
fn t005_gas_priority_names_every_consumer_exactly_once() {
    // Who pays when the tank runs low is a game-value decision (docs/QUESTIONS.md Q-017).
    // If a consumer is missing it never gets gas, and nobody goes looking for it in the RON.
    let r = &data().game.vector.gas_priority;
    for who in [
        GasConsumer::Boost,
        GasConsumer::Steer,
        GasConsumer::ReelIn,
        GasConsumer::Dodge,
        GasConsumer::Flip,
    ] {
        assert_eq!(
            r.iter().filter(|x| **x == who).count(),
            1,
            "gas_priority = {r:?} — {who:?} must appear exactly once"
        );
    }
    // Four since 2026-08-13: `Steer` (docs/NEXT.md §1B, FIND-082) is the rope half of the air
    // control, and it is a rate like the first two.
    // **Five since 2026-08-24:** `Flip` (`F-009`) is the second consumer that is not a rate —
    // it bills `vector.gas_flip` once, on the tick a double-tap of `A` or `D` lands in the air.
    // It is a fifth consumer and not a second meaning for `Dodge` because the two differ in
    // price, in direction and in what they buy, and one field with two prices is a ledger
    // nobody can audit (`docs/QUESTIONS.md` Q-052 §4 for why it is LAST).
    assert_eq!(r.len(), 5, "gas_priority = {r:?} — exactly five consumers, no more");
}

#[test]
fn t005_the_rope_solver_runs_at_least_twice() {
    // Zero passes would be a rope shut down; with one pass and two anchors the second
    // constraint violates the first one again.
    let n = data().game.vector.rope_iterations;
    assert!((1..=16).contains(&n), "rope_iterations = {n} — expected 1..16");
    assert!(n >= 2, "rope_iterations = {n} — with one pass the two-hook case (F-004) is violated \
         after a single tick");
}

#[test]
fn t005_the_current_map_is_in_maps_ron() {
    let d = data();
    assert!(
        d.current_map().is_some(),
        "maps.ron: current = {:?}, but that map is not listed under `maps`",
        d.maps.current
    );
}

#[test]
fn t005_every_block_names_a_color_from_the_palette() {
    // A reference into nothing is a bug of the same class as a dead link in the docs.
    // Without this test it only surfaces when exactly this block gets built.
    let d = data();
    for (id, map) in &d.maps.maps {
        for (i, k) in map.blocks.iter().enumerate() {
            assert!(
                d.color(&k.color).is_some(),
                "{id}: block {i} names color {:?}, which is not in the palette",
                k.color
            );
        }
        for color in &map.layout.colors {
            assert!(
                d.color(color).is_some(),
                "{id}: layout.colors names {color:?}, which is not in the palette"
            );
        }
        assert!(!map.layout.colors.is_empty(), "{id}: layout without a single color");
    }
}

#[test]
fn t005_every_map_can_be_built() {
    // The numbers `world` generates the city from: a zero or a swapped height range would be
    // an empty or an infinite city.
    let d = data();
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        assert!(map.size_m.0 > 0.0 && map.size_m.1 > 0.0, "{id}: size_m = {:?}",
                map.size_m);
        assert!(r.lot_m > 0.0 && r.street_m > 0.0, "{id}: lot_m/street_m must be > 0");
        assert!(
            r.min_height_m > 0.0 && r.min_height_m < r.max_height_m,
            "{id}: min_height_m = {} / max_height_m = {}", r.min_height_m, r.max_height_m
        );
        assert!((0.0..=1.0).contains(&r.density), "{id}: density = {} is not in 0..1", r.density);
        assert!(
            (0.0..=1.0).contains(&r.anchorable_fraction),
            "{id}: anchorable_fraction = {} is not in 0..1", r.anchorable_fraction
        );
        assert!(r.clear_radius_m > 0.0, "{id}: clear_radius_m = 0 builds a house on top of the player");
        // A street wider than a lot is not a city, it is a field with blocks on it.
        assert!(r.street_m < r.lot_m, "{id}: street_m {} >= lot_m {}", r.street_m, r.lot_m);

        for (i, k) in map.blocks.iter().enumerate() {
            let g = k.size_m;
            assert!(
                g.0 > 0.0 && g.1 > 0.0 && g.2 > 0.0,
                "{id}: block {i} has size_m = {g:?} — a cuboid without extent is not an \
                 obstacle but a division by zero"
            );
            assert!(
                k.solid || k.anchorable,
                "{id}: block {i} is neither solid nor anchorable — it would be invisible to \
                 every system that asks the index"
            );
        }
    }
}

#[test]
fn t005_the_graybox_carries_anchorable_and_untagged_surfaces() {
    // Only this makes "no hook possible on untagged parts" (F-003) runnable at all: a map on
    // which everything is anchorable cannot falsify the criterion.
    // ⚠️ **It names `graybox` and no longer follows `current`, since 2026-08-13.** The user:
    // *"es ist extrem wichtig dass man wirklich überall sein seil festmachen kann. also überall!
    // ohne ausnahmen!"* — so the **played** map (`ashgate`) is deliberately 100 % anchorable:
    // 2067 of 2067 blocks. Reading `current_map()` here asked the shipped district to keep a
    // property the user removed on purpose.
    //
    // What this guard is FOR survives untouched: the **fixture** must keep untagged geometry, or
    // the untagged path stops being falsifiable anywhere in the project. `graybox` keeps 22
    // untagged blocks and is what `tests/vector_aiming.rs` is pinned to — the same repair as
    // `FIND-061` (*a test that follows a mutable global has a level designer as a co-author*),
    // one file later.
    let d = data();
    let map = d
        .maps
        .maps
        .get("graybox")
        .expect("the graybox fixture must exist — it keeps the untagged path falsifiable");
    let anchorable = map.blocks.iter().filter(|k| k.anchorable).count();
    let untagged = map.blocks.iter().filter(|k| !k.anchorable).count();
    assert!(anchorable > 0, "not a single anchorable surface on the graybox fixture");
    assert!(
        untagged > 0,
        "every graybox surface is anchorable — then F-003 checks nothing anywhere, because the \
         played map is 100 % anchorable on purpose (docs/NEXT.md §1D item 10)"
    );
}

#[test]
fn t005_every_third_party_asset_carries_its_attribution() {
    // §7: without an `attribution` a third-party asset is a zombie — the user cannot find it
    // later in order to replace it. As long as everything is a placeholder the list is
    // empty; that is exactly when this test still has to exist, so the first foreign model
    // gets noticed.
    for (name, model) in &data().art.models {
        if let Some(h) = &model.attribution {
            assert!(
                h.contains("http") && h.contains("20"),
                "{name}: attribution {h:?} — expected URL · date · license · what it replaces"
            );
        }
        assert!(model.scale > 0.0, "{name}: scale = {}", model.scale);
    }
}

// ===========================================================================
// The scale — assets/data/scale.ron is the ONE truth about sizes
// ===========================================================================
//
// The numbers below were laid down by the **user** (2026-08-09). They are not untuned, they
// are decided. A value in a file that nobody checks drifts away — and sizes drift especially
// quietly, because a house that is too tall does not crash, it merely looks wrong. So behind
// every size here stands a test that goes RED.
//
// The precedence rule these tests enforce: **a direct figure in meters from the user beats
// every derivation** — including the conversion out of the backlog (docs/QUESTIONS.md Q-002).

#[test]
fn t005_the_player_capsule_is_exactly_the_human_reference() {
    // Catches: a player capsule that deviates from the 1.80 m reference — the user writes
    // "check the capsule exactly!" in so many words.
    // Without this test: every size comparison in the image would be off by that same factor.
    // A 1.9 m capsule makes a 21 m titan look like 20 m and the wall like 114 m — and you
    // would go tune the titan while the error sits on the player. Exactly the kind of bug
    // you only find once somebody writes it down.
    let d = data();
    assert_eq!(
        d.game.player.height_m, d.scale.reference.human_height_m,
        "player.height_m = {} deviates from the human reference = {} (scale.ron)",
        d.game.player.height_m, d.scale.reference.human_height_m
    );
}

#[test]
fn t005_eye_height_is_the_scales_camera_height() {
    // Catches: an eye height that is not the prescribed camera height (1.60 m), and an eye
    // height above the crown of the head.
    // Without this test: the old 1.65 m would simply have stayed. They were estimated from
    // body height, and an estimate next to a prescription does not stand out — it is
    // "roughly right", after all. At a 55–65 degree field of view, five centimeters of camera
    // height are exactly the difference between "I am standing in front of it" and "I am
    // floating in front of it".
    let d = data();
    let s = &d.game.player;
    assert_eq!(
        s.eye_height_m, d.scale.camera.height_m,
        "player.eye_height_m = {} != scale.ron camera.height_m = {}",
        s.eye_height_m, d.scale.camera.height_m
    );
    assert!(
        s.eye_height_m < s.height_m,
        "eye_height_m = {} is not below height_m = {} — the camera floats above the head",
        s.eye_height_m, s.height_m
    );
}

#[test]
fn t005_hook_range_is_the_users_anchor_range() {
    // Catches: every relapse to the derived 112 m (400 studs × 0.28, Q-002). The user states
    // 90 m directly, and a direct figure in meters beats every derivation.
    // Without this test: the 112 would come back with the next "let me recompute that from
    // the backlog" — with a rationale and a source, and therefore especially convincing. The
    // range decides whether the wall is reachable in two moves and how large the spatial grid
    // has to be; changing it quietly by 24 % moves both.
    let d = data();
    assert_eq!(
        d.game.vector.hook_range_m, d.scale.vector.anchor_range_m,
        "vector.hook_range_m = {} != scale.ron vector.anchor_range_m = {}",
        d.game.vector.hook_range_m, d.scale.vector.anchor_range_m
    );
}

#[test]
fn t005_ground_fov_stays_in_the_users_window() {
    // Catches: a field of view outside 55–65 degrees. The user calls it the "biggest lever",
    // and he explicitly names GROUND COMBAT — 60 degrees is the baseline, not the ceiling.
    // Without this test: the old 90 degrees would come back the moment somebody feels "too
    // cramped". 90 degrees make every titan small and every meter short; that is exactly the
    // perception this game must not have. The second assert pins down that the speed FOV
    // (F-017) goes UP — a smaller value would be a camera that zooms in at top speed instead
    // of opening up, that is, the opposite of a sense of speed.
    let d = data();
    let k = &d.game.camera;
    let m = &d.scale.camera;
    assert!(
        (m.min_ground_fov_deg..=m.max_ground_fov_deg).contains(&k.fov_deg),
        "camera.fov_deg = {} is not in {}..={} (scale.ron, ground-combat FOV)",
        k.fov_deg, m.min_ground_fov_deg, m.max_ground_fov_deg
    );
    assert!(
        k.fov_max_speed_deg >= k.fov_deg,
        "fov_max_speed_deg = {} < fov_deg = {} — F-017 opens the image up with speed, \
         it does not close it",
        k.fov_max_speed_deg, k.fov_deg
    );
    assert!(
        k.fov_max_speed_deg < 180.0,
        "fov_max_speed_deg = {} — from 180 degrees on the projection matrix degenerates",
        k.fov_max_speed_deg
    );
}

#[test]
fn t005_every_titan_kind_carries_exactly_one_size_class() {
    // Catches: a kind that points at a size class which does not exist in scale.ron (typo,
    // deleted class, invented class) — F-064.
    // Without this test: `titan_height_m()` returns `None`, and the first caller who writes
    // `unwrap_or(0.0)` spawns a titan of height zero. That one stands in the ground, its
    // cortex sits at 0 m, and you go looking for the bug in the collision code instead of in
    // one letter in one file.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).unwrap_or_else(|| {
            panic!(
                "{name}: size_class = {:?} is not in scale.ron titan.classes — \
                 known are {:?}",
                kind.size_class,
                d.scale.titan.classes.keys().collect::<Vec<_>>()
            )
        });
        // And the class really has to be a size: taller than a human, shorter than the
        // Ashwalker. Anything else is not a titan, it is a typo with a decimal point in it.
        assert!(
            height > d.scale.reference.human_height_m
                && height <= d.scale.titan.ashwalker_height_m,
            "{name}: class {:?} is {height} m — that is not a titan",
            kind.size_class
        );
    }
}

#[test]
fn t005_the_cortex_sits_at_eighty_nine_percent_of_the_height() {
    // Catches: a cortex height that no longer sits at ~89 % (F-030). The user gives five
    // pairs, all of which come out at 0.881–0.893; "~89 %" is his rounding, 88–90 % is the
    // window.
    //
    // ⚠️ Since 2026-08-09 this test checks a **ratio between two of the user's meter figures**
    // instead of a number against its own formula. Before that, `cortex_height_m` was
    // computed from `height_m * cortex_fraction` — and then this assert can never go red,
    // because it recomputes exactly the calculation that produced the value. A guard over a
    // derivation is decoration. Now both numbers stand in the RON, and the fraction is what
    // it is for the user: the rule by which you notice that one of the two has drifted.
    //
    // ALL classes are checked, not just the ones in use: `boss` (28 m) has no representative
    // in titan.ron today, and that is exactly why a mistake there would go unnoticed.
    let d = data();
    let m = &d.scale.titan;
    for (name, k) in &m.classes {
        let fraction = k.cortex_height_m / k.height_m;
        assert!(
            (0.88..=0.90).contains(&fraction),
            "class {name}: cortex {} m of {} m = {fraction} — the user prescribes ~89 % \
             (3.7/4.2 · 8.9/10 · 12.5/14 · 18.7/21 · 24.9/28)",
            k.cortex_height_m, k.height_m
        );
        // The cortex sits on the body, not above it: otherwise the marker points at thin air.
        assert!(
            k.cortex_height_m < k.height_m,
            "class {name}: cortex at {} m above the crown {} m", k.cortex_height_m, k.height_m
        );
        // And the number has to be a STATED figure, not a calculation. The user names his
        // cortex heights to the decimeter (3.7 · 8.9 · 12.5 · 18.7 · 24.9); whatever falls
        // out of `height_m * 0.89` has three decimal places (4.2 × 0.89 = 3.738). Without
        // this assert the relapse into the derivation would be invisible — 3.738 sits right
        // in the middle of the window above, and that is exactly why the window cannot
        // catch it.
        let decimeter = (k.cortex_height_m * 10.0).round() / 10.0;
        assert!(
            (k.cortex_height_m - decimeter).abs() < 1e-4,
            "class {name}: cortex_height_m = {} is not stated to a decimeter — it looks like \
             `height_m * cortex_fraction`. The five cortex heights are the users meter \
             values, not a derivation (docs/models.md)",
            k.cortex_height_m
        );
    }
    // And the fraction itself has to be the center of that window — otherwise the rule checks
    // something other than what it claims.
    assert!(
        (m.cortex_fraction - 0.89).abs() < 0.005,
        "cortex_fraction = {} — the user writes \"cortex at ~89 %\"", m.cortex_fraction
    );
    // Every kind reaches its cortex height through its class, with no detour via a formula.
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).expect("size class");
        let cortex = d.titan_cortex_height_m(kind).expect("cortex height");
        assert!(cortex > 0.0 && cortex < height, "{name}: cortex {cortex} m / height {height} m");
    }
}

#[test]
fn t005_the_cortex_fits_under_the_titans_head() {
    // Catches: a hit zone larger than the titan's entire head.
    // Without this test: that was exactly the state of things, and nobody could see it,
    // because until 2026-08-09 the user's head rule stood nowhere as a number. `scuttler` had
    // cortex_radius_m 0.40 — so 0.80 m of diameter — on a 4.2 m body whose head, by the
    // 1/9..1/10 rule, measures only 0.42..0.47 m. The cortex was almost twice the size of the
    // head; `weaver` (0.90 m) even more so. Geometrically impossible, and in the image the
    // small titan wears a bullseye where its neck should be.
    //
    // The test is deliberately only an UPPER BOUND. Whether the radius should grow with body
    // size is an open question for the user (docs/QUESTIONS.md Q-019) — that it must not
    // exceed the head is not.
    let d = data();
    for (name, kind) in &d.titans.kinds {
        let height = d.titan_height_m(kind).expect("size class");
        let head = d.titan_max_head_height_m(kind).expect("head height");
        let diameter = 2.0 * kind.cortex_radius_m;
        assert!(
            diameter <= head,
            "{name}: cortex_radius_m = {} ⇒ {diameter} m diameter on a {height} m \
             body whose head is at most {head} m tall (scale.ron \
             titan.max_head_fraction). A hit zone larger than the head cannot be a \
             neck joint",
            kind.cortex_radius_m
        );
    }
}

#[test]
fn t005_the_streets_stay_as_narrow_as_the_user_wants() {
    // Catches: a street width outside 6–8 m. The user writes "keep them narrow".
    // Without this test: the old 9 m would come back the moment somebody turns a knob on the
    // layout, and wider never stands out — it looks tidier, after all. But narrow streets are
    // the reason speed feels like speed: the wall flying past has to be close.
    let d = data();
    let r = &d.scale.reference;
    for (id, map) in &d.maps.maps {
        assert!(
            (r.min_street_m..=r.max_street_m).contains(&map.layout.street_m),
            "{id}: layout.street_m = {} is not in {}..={} (scale.ron)",
            map.layout.street_m, r.min_street_m, r.max_street_m
        );
    }
}

#[test]
fn t005_the_generated_city_stays_residential() {
    // Catches: generated houses outside the residential band (4.5 m small house to 11.5 m
    // large house). This used to say 8–34 m; 34 m is the size of a church.
    // Without this test: the city creeps back up, because "more anchor points up high" sounds
    // reasonable in every single case. **The city is MEANT to be flat.** The vertical comes
    // from the wall (120 m), the church (35 m), the watchtower and the trees — and those only
    // work as long as the residential band does not catch up with them. A skyline would not
    // be a balancing mistake, it would be a different game.
    let d = data();
    let h = &d.scale.architecture.heights_m;
    let low = h["house_small"];
    let high = h["house_large"];
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        assert!(
            r.min_height_m >= low,
            "{id}: min_height_m = {} is below the small house ({low} m)", r.min_height_m
        );
        assert!(
            r.max_height_m <= high,
            "{id}: max_height_m = {} is above the large house ({high} m) — that is landmark \
             height (church {} m) and belongs in `blocks` as a placed entry, \
             not rolled",
            r.max_height_m, h["church"]
        );
    }
}

#[test]
fn t005_placed_blocks_stay_residential_too() {
    // Catches: an explicitly placed cuboid above 11.5 m that does not declare itself a
    // landmark.
    // Without this test: half the city stands outside the rule that is supposed to keep it
    // flat. That was exactly the state of things — the guard above read only `layout`, while
    // `blocks` carried ridge heights of 12.0 / 14.0 / 18.0 m. 18 m is precisely the height
    // that the same test forbids the layout as "landmark height", and those three were not
    // landmarks, they were gray cubes.
    //
    // Only an UPPER BOUND, no lower one: a parapet, a wall panel and the ground slab may be
    // flatter than a small house. Upwards, `landmark` alone decides, and that is the point —
    // the exception has to name itself.
    let d = data();
    let high = d.scale.architecture.heights_m["house_large"];
    for (id, map) in &d.maps.maps {
        for (i, k) in map.blocks.iter().enumerate() {
            if k.landmark {
                continue;
            }
            let ridge = k.center_m.1 + k.size_m.1 * 0.5;
            assert!(
                ridge <= high,
                "{id}: block {i} has ridge height {ridge} m and thus stands above the \
                 residential band ({high} m). Either it is a structure from \
                 scale.ron:architecture.heights_m — then `landmark: true` — or it gets \
                 shortened. A gray cube is no reason to raise the city"
            );
        }
    }
}

#[test]
fn t005_every_size_class_has_a_structure_above_its_cortex() {
    // Catches: a size class whose cortex sits above every structure the scale knows about —
    // that is, a titan you cannot approach from above in this world.
    // Without this test: the size table created three classes (14 / 21 / 28 m) whose cortex
    // sits at 12.5 / 18.7 / 24.9 m, while the tallest residential house measures 11.5 m. Roof
    // height + min_rope_m gives an **anchor ceiling**; above it no rope holds. Every approach
    // would be ballistic — let go, fly, one pass with no correction — and the blame would go
    // to the rope release, the boost and the camera, because the cause is two numbers in two
    // other files that nobody holds up against each other.
    // The user's numbers stay; what changes is the composition of the city (church 35 m,
    // watchtower 12 m, tree 12 m). docs/QUESTIONS.md Q-022.
    let d = data();
    let m = &d.scale;
    let (tallest, height) = m
        .architecture
        .heights_m
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("scale.ron without a single structure");
    for (name, k) in &m.titan.classes {
        assert!(
            *height >= k.cortex_height_m,
            "class {name}: cortex at {} m, but the tallest structure is {tallest} with \
             {height} m — there is no anchor above that cortex",
            k.cortex_height_m
        );
    }
}

#[test]
fn t005_the_start_map_really_carries_the_vertical() {
    // Catches: a map that places nothing but residential buildings while the docs claim the
    // vertical comes from landmarks.
    // Without this test: exactly that gap. `maps.ron` promised itself "church, watchtower and
    // wall are placed as `blocks`" — and not one map contained any of them. The statement was
    // an intention, not a state, and the arithmetic behind it (highest anchor 11.5 m +
    // min_rope_m 3.0 m = a 14.5 m anchor ceiling) hit exactly the three size classes the size
    // table had just created.
    //
    // What is checked is the map that actually gets built — a number in `scale.ron` is not an
    // anchor, a block in the map is one.
    let d = data();
    let map = d.current_map().expect("current map");
    let high = d.scale.architecture.heights_m["house_large"];
    let highest_anchor = map
        .blocks
        .iter()
        .filter(|k| k.anchorable)
        .map(|k| k.center_m.1 + k.size_m.1 * 0.5)
        .fold(0.0f32, f32::max);
    let ceiling = highest_anchor + d.game.vector.min_rope_m;
    assert!(
        highest_anchor > high,
        "the highest ANCHORABLE point of the start map is at {highest_anchor} m — not \
         above the residential band ({high} m). Without an anchorable landmark the \
         vertical is a claim"
    );
    // And it has to carry the largest class titan.ron actually uses.
    let highest_cortex = d
        .titans
        .kinds
        .values()
        .filter_map(|a| d.titan_cortex_height_m(a))
        .fold(0.0f32, f32::max);
    assert!(
        ceiling >= highest_cortex,
        "anchor ceiling {ceiling} m, highest cortex of a used titan kind {highest_cortex} m \
         — every approach to that target would be ballistic (docs/FRAGEN.md Q-022)"
    );
}

#[test]
fn t005_every_map_carries_its_own_layout() {
    // Catches: a map smaller than the layout that is supposed to stand on it — and a clear
    // area around the origin that eats half the map.
    // Without this test: the city would have one or zero complete blocks, the layout would
    // generate almost nothing, and you would look for the bug in the generator instead of in
    // two numbers.
    let d = data();
    for (id, map) in &d.maps.maps {
        let r = &map.layout;
        let period = r.lot_m + r.street_m;
        let shortest_edge = map.size_m.0.min(map.size_m.1);
        assert!(
            shortest_edge >= 4.0 * period,
            "{id}: a map of {shortest_edge} m does not carry four grid periods of {period} m",
        );
        assert!(
            2.0 * r.clear_radius_m < shortest_edge,
            "{id}: clear_radius_m = {} eats the map ({shortest_edge} m)",
            r.clear_radius_m
        );
    }
}

#[test]
fn t005_the_scale_factors_stay_unequal() {
    // Catches: the "tidier-upper" who levels the three factors to 1.0 because unequal scales
    // look like a mistake.
    // Without this test: exactly that happens, with a clear conscience and in a one-liner.
    // Titans exaggerated by 1.4 and walls by 2.4 are the VISUAL LANGUAGE of the reference
    // work: the human small, the threat out of all proportion, the wall a horizon. A uniform
    // scale would be technically clean and artistically dead.
    let m = &data().scale;
    for (name, f) in [
        ("architecture_factor", m.architecture_factor),
        ("titan_factor", m.titan_factor),
        ("wall_factor", m.wall_factor),
    ] {
        assert!(f.is_finite() && f > 0.0, "{name} = {f} — must be finite and > 0");
    }
    assert!(
        m.titan_factor > m.architecture_factor,
        "titan_factor = {} is not greater than architecture_factor = {} — then titans are \
         no longer exaggerated",
        m.titan_factor, m.architecture_factor
    );
    assert!(
        m.wall_factor > m.titan_factor,
        "wall_factor = {} is not greater than titan_factor = {} — then the wall is no \
         horizon any more, just a big wall",
        m.wall_factor, m.titan_factor
    );
}

#[test]
fn t005_the_ashwalker_rises_thirty_meters_above_the_wall() {
    // Catches: every change to wall height or Ashwalker height that breaks the user's own
    // check — "150 m, 30 m above the wall".
    // Without this test: somebody lowers the wall to 100 m because it feels "unreachable",
    // and the boss's entrance loses exactly the image it is 150 m tall for. The relation
    // between the two numbers is the claim, not their magnitude.
    let m = &data().scale;
    let overhang = m.titan.ashwalker_height_m - m.wall.height_m;
    assert!(
        (overhang - 30.0).abs() < 0.01,
        "Ashwalker {} m − wall {} m = {overhang} m, the user prescribes 30 m",
        m.titan.ashwalker_height_m, m.wall.height_m
    );
}

#[test]
fn t005_the_wall_is_reachable_in_two_moves() {
    // Catches: a wall whose crown (120 m) or whose intermediate platform (60 m) is no longer
    // reachable within the anchor range.
    // Without this test: the three numbers wander independently — range down for balancing,
    // wall up for effect — and at some point the wall can no longer be climbed from below.
    // That only surfaces when somebody tries it in game, and then the guess is "the controls
    // are broken", not "three RON numbers do not fit together".
    //
    // ⚠️ **Since the range went 90 -> 200 m (2026-08-10, Q-035) this test has slack it did not
    // use to have, and the name is now optimistic:** at 200 m the crown (120 m) is reachable
    // from the ground in ONE move, so the intermediate platform is no longer load-bearing for
    // the climb. The asserts below are still the right asserts — they are what goes red if the
    // range comes back down — but nobody should read a green run here as evidence that the
    // two-move climb is still the design. Whether the platform still has a job is a question
    // for the user, not for this file.
    let d = data();
    let m = &d.scale.wall;
    let range = d.scale.vector.anchor_range_m;
    assert!(
        m.platform_height_m > 0.0 && m.platform_height_m < m.height_m,
        "platform_height_m = {} is not between the ground and the crown ({} m)",
        m.platform_height_m, m.height_m
    );
    assert!(
        m.platform_height_m <= range,
        "ground -> platform is {} m, the anchor range is {range} m",
        m.platform_height_m
    );
    assert!(
        m.height_m - m.platform_height_m <= range,
        "platform -> crown is {} m, the anchor range is {range} m",
        m.height_m - m.platform_height_m
    );
    // Battered means: thicker at the base than at the top. The other way round would be an
    // overhang no hook holds on to and under which every titan would find cover.
    assert!(
        m.base_thickness_m > m.top_thickness_m,
        "base_thickness_m = {} <= top_thickness_m = {} — the wall overhangs",
        m.base_thickness_m, m.top_thickness_m
    );
}

#[test]
fn t005_the_walls_scale_ladder_stays_readable() {
    // Catches: a stone course or a banding too coarse to read size off.
    // Without this test: both numbers look like decoration and get cut in the first
    // performance conversation ("less geometry on the wall"). Then the 120 m wall is a gray
    // surface, the eye has no ladder, and up close the wall reads like a 12 m wall. The stone
    // course has to be clearly smaller than a human — otherwise it is not a scale, just a
    // pattern.
    let m = &data().scale;
    assert!(
        m.wall.stone_course_m > 0.0 && m.wall.stone_course_m < m.reference.human_height_m * 0.5,
        "stone_course_m = {} — as a scale ladder a course must sit well below half a \
         human height ({} m)",
        m.wall.stone_course_m, m.reference.human_height_m * 0.5
    );
    assert!(
        m.wall.band_spacing_m > m.wall.stone_course_m && m.wall.band_spacing_m < m.wall.height_m,
        "band_spacing_m = {} must be coarser than a stone course and finer than the wall",
        m.wall.band_spacing_m
    );
    // Enough bands for the ladder to be a ladder at all.
    let bands = (m.wall.height_m / m.wall.band_spacing_m).floor();
    assert!(bands >= 4.0, "only {bands} bands on {} m of wall", m.wall.height_m);
}

#[test]
fn t005_every_eaves_belongs_to_a_structure_that_exists() {
    // Catches: an eaves height without a total height (typo in the key) and eaves that sit
    // above the ridge.
    // Without this test: the modeler builds a house with 6 m eaves and 4.5 m total height —
    // a roof that points downwards. A reference into nothing is the same bug as a dead link
    // in the docs, except that nobody clicks this one.
    let a = &data().scale.architecture;
    for (name, eaves) in &a.eaves_m {
        let height = a.heights_m.get(name).unwrap_or_else(|| {
            panic!("eaves_m names {name:?}, which is not in heights_m")
        });
        assert!(
            *eaves > 0.0 && eaves < height,
            "{name}: eaves {eaves} m do not fit under the total height {height} m"
        );
    }
    for (name, height) in &a.heights_m {
        assert!(*height > 0.0, "architecture.heights_m[{name}] = {height}");
    }
    assert!(!a.heights_m.is_empty(), "scale.ron without a single structure");
}

#[test]
fn t005_the_titan_head_stays_smaller_than_the_human_head() {
    // Catches: a head-size rule that makes the titan head relatively as large as the human
    // head (1/7.5).
    // Without this test: the first person to make a titan model "more proportional" raises
    // exactly this number — and from then on the model looks like a human standing too close
    // instead of a 21 m body. The relatively small head IS the impression of size; together
    // with cortex_fraction it decides whether the cortex is readable at 100 m (F-030).
    let m = &data().scale;
    let t = &m.titan;
    assert!(
        t.min_head_fraction > 0.0 && t.min_head_fraction < t.max_head_fraction,
        "min_head_fraction = {} / max_head_fraction = {} — min must be strictly smaller",
        t.min_head_fraction, t.max_head_fraction
    );
    assert!(
        t.max_head_fraction < m.reference.human_head_fraction,
        "max_head_fraction = {} is not smaller than the human fraction {} (1/7.5) — then \
         the titan reads as a nearby human, not as a big body",
        t.max_head_fraction, m.reference.human_head_fraction
    );
    // The user writes "1/9 - 1/10". The window has to really CONTAIN both fractions: at
    // 0.1111 a model built exactly on 1/9 = 0.111111… falls out of its own prescription by
    // one ten-thousandth. That is not a rounding detail, it is an upper bound that later gets
    // checked against.
    assert!(
        t.min_head_fraction <= 1.0 / 10.0 && t.max_head_fraction >= 1.0 / 9.0,
        "head fraction {}..{} does not include 1/10 = {} and 1/9 = {}",
        t.min_head_fraction, t.max_head_fraction, 1.0 / 10.0, 1.0 / 9.0_f32
    );
}

#[test]
fn t005_the_grid_carries_the_worlds_height_too() {
    // Catches: a world that grows taller than the grid indexing it.
    // Without this test: `half_extent_m` was computed against the map in the PLANE
    // (400/2 + 90 = 290) and never against height. In Y the wall (120 m) and the Ashwalker
    // (150 m) stand on top of each other — 270 m, so 30 m of margin. Whether the grid is
    // three-dimensional at all is open (docs/QUESTIONS.md Q-014); that a grid must not be
    // smaller than the world it holds is not. A body outside it lands in the border cell and
    // is therefore in the wrong place — and that looks like a bug in the ray cast.
    let d = data();
    let m = &d.scale;
    let highest_point = m.wall.height_m + m.titan.ashwalker_height_m;
    assert!(
        d.game.world.half_extent_m >= highest_point,
        "world.half_extent_m = {} does not cover wall ({} m) + Ashwalker ({} m) = \
         {highest_point} m",
        d.game.world.half_extent_m, m.wall.height_m, m.titan.ashwalker_height_m
    );
}

/// A number the way `docs/models.md` writes it: decimal point, no trailing zero.
fn doc_number(value: f32) -> String {
    format!("{value}")
}

#[test]
fn t005_the_size_table_in_the_docs_shows_the_same_numbers() {
    // Catches: every number in `scale.ron` that no longer has its row in `docs/models.md` —
    // and every new structure missing from the doc.
    // Without this test: `docs/models.md` is a second, complete and entirely unguarded copy
    // of the same ~30 numbers. Today the file secures that with the sentence "both are
    // changed together or not at all" — which is a request, not a guard. And it is the
    // version the **modeler** reads: a doc that quietly deviates from the data is worse than
    // no doc, because people build to it.
    //
    // What is checked is the cell form "| <number> m", not the bare number — otherwise `8 m`
    // would find itself inside `18 m` and the test would be green while knowing nothing.
    let d = data();
    let m = &d.scale;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/models.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));

    let mut expected: Vec<(String, String)> = vec![
        ("reference.human_height_m".into(), format!("| {} m", doc_number(m.reference.human_height_m))),
        ("reference.door_height_m".into(), format!("| {} m", doc_number(m.reference.door_height_m))),
        ("wall.height_m".into(), format!("| {} m", doc_number(m.wall.height_m))),
        ("wall.top_thickness_m".into(), format!("| {} m", doc_number(m.wall.top_thickness_m))),
        ("wall.base_thickness_m".into(), format!("| {} m", doc_number(m.wall.base_thickness_m))),
        ("wall.platform_height_m".into(), format!("| {} m", doc_number(m.wall.platform_height_m))),
        ("wall.stone_course_m".into(), format!("| {} m", doc_number(m.wall.stone_course_m))),
        ("wall.band_spacing_m".into(), format!("| {} m", doc_number(m.wall.band_spacing_m))),
        ("titan.ashwalker_height_m".into(), format!("| {} m", doc_number(m.titan.ashwalker_height_m))),
        ("camera.height_m".into(), format!("| {} m", doc_number(m.camera.height_m))),
        ("vector.anchor_range_m".into(), format!("| {} m", doc_number(m.vector.anchor_range_m))),
    ];
    for (name, height) in &m.architecture.heights_m {
        expected.push((format!("architecture.heights_m[{name}]"), format!("| {} m", doc_number(*height))));
    }
    for (name, class) in &m.titan.classes {
        expected.push((format!("titan.classes[{name}].height_m"),
                   format!("| {} m", doc_number(class.height_m))));
        expected.push((format!("titan.classes[{name}].cortex_height_m"),
                   format!("Cortex {} m", doc_number(class.cortex_height_m))));
    }

    for (source, wanted) in expected {
        assert!(
            text.contains(&wanted),
            "docs/models.md does not contain {wanted:?} — the doc deviates at {source} from \
             assets/data/scale.ron. Both change together or not at all"
        );
    }
}

// ===========================================================================
// The box rig — six fractions that only mean anything against each other
// ===========================================================================
//
// `width_fraction`, `leg_fraction`, `torso_fraction`, `shoulder_height_fraction` and
// `arm_fraction` came into scale.ron on 2026-08-09 as ⚠️ UNTUNED, and unlike everything above
// them they were **invented, not laid down**. They were chosen to add up against numbers that
// are laid down — and until now nothing said so. That is the dangerous kind of arithmetic:
// it is right today, it costs nothing to move one of the six, and when the cortex ends up in
// the belly the screenshot still shows a titan.

/// Where the head starts, as a fraction of the body height, out of the rig's own stack.
fn rig_seam(t: &TitanScale) -> f32 {
    t.leg_fraction + t.torso_fraction
}

/// What is left over above the seam — the head.
fn rig_head_fraction(t: &TitanScale) -> f32 {
    1.0 - rig_seam(t)
}

#[test]
fn t005_the_box_rig_seams_exactly_at_the_cortex() {
    // Catches: a leg or torso fraction that has been moved without moving the other one.
    // Without this test: the cortex is a sphere at 89 % of the height while the seam between
    // torso and head is somewhere else, and the amber ball sits in the middle of a box
    // instead of on the nape. In a screenshot that is a titan; in the game it is a weak point
    // that is no longer a neck.
    let d = data();
    let t = &d.scale.titan;
    let seam = rig_seam(t);
    assert!(
        (seam - t.cortex_fraction).abs() <= 0.001,
        "leg_fraction {} + torso_fraction {} = {seam}, but cortex_fraction = {} — the cortex \
         is the seam between torso and head, or it is not a nape",
        t.leg_fraction, t.torso_fraction, t.cortex_fraction
    );
}

#[test]
fn t005_the_box_rigs_head_stays_inside_the_users_head_rule() {
    // Catches: a rig whose leftover head falls outside the 1/10..1/9 window the user laid
    // down — the rule that makes a titan read as huge instead of as a doll.
    // Without this test: the head fraction is a SUBTRACTION of two invented numbers and is
    // therefore checked by nobody, while the window right next to it in the same file is
    // checked by t005_the_titan_head_stays_smaller_than_the_human_head.
    let d = data();
    let t = &d.scale.titan;
    let head = rig_head_fraction(t);
    assert!(
        (t.min_head_fraction..=t.max_head_fraction).contains(&head),
        "the rig leaves {head} of the body height for the head (1 − {} − {}), outside the \
         user's window {}..{}",
        t.leg_fraction, t.torso_fraction, t.min_head_fraction, t.max_head_fraction
    );
}

#[test]
fn t005_the_shoulders_sit_below_the_cortex_and_the_hands_above_the_ground() {
    // Catches: an arm that hinges at the nape (it can then not swing in front of the body,
    // and F-053's telegraph has nowhere to go), and an arm so long that the hand hangs
    // through the floor.
    // Without this test: both are one digit away and neither crashes.
    let d = data();
    let t = &d.scale.titan;
    assert!(
        t.shoulder_height_fraction < t.cortex_fraction,
        "shoulder_height_fraction = {} is not below cortex_fraction = {} — the arm hinges at \
         the nape and cannot swing in front of the body",
        t.shoulder_height_fraction, t.cortex_fraction
    );
    assert!(
        t.arm_fraction < t.shoulder_height_fraction,
        "arm_fraction = {} is not shorter than the shoulder height {} — the hand hangs \
         through the ground",
        t.arm_fraction, t.shoulder_height_fraction
    );
    assert!(
        t.width_fraction > 0.0 && t.width_fraction < 1.0,
        "width_fraction = {} — a titan is neither flat nor square", t.width_fraction
    );
}

#[test]
fn t005_the_rig_guard_really_notices_a_moved_fraction() {
    // **The guard over the two guards above.** They are green today, and a green assertion
    // that has never been seen red is a claim, not a check (CLAUDE.md rule 5). scale.ron is
    // the user's file and does not get edited to prove a point, so the mutation happens on a
    // copy: move one fraction and both guards have to fall over.
    let d = data();
    let mut broken = d.scale.titan.clone();

    broken.leg_fraction += 0.02;
    assert!(
        (rig_seam(&broken) - broken.cortex_fraction).abs() > 0.001,
        "a leg 2 % longer does not move the seam — the seam guard checks nothing"
    );
    assert!(
        !(broken.min_head_fraction..=broken.max_head_fraction).contains(&rig_head_fraction(&broken)),
        "a leg 2 % longer leaves the head inside the window — the head guard checks nothing"
    );

    let mut shrunk = d.scale.titan.clone();
    shrunk.torso_fraction -= 0.02;
    assert!((rig_seam(&shrunk) - shrunk.cortex_fraction).abs() > 0.001);
    assert!(
        !(shrunk.min_head_fraction..=shrunk.max_head_fraction).contains(&rig_head_fraction(&shrunk))
    );
}

#[test]
fn t005_the_class_cap_names_a_class_that_exists_and_leaves_something_out() {
    // Catches: a `max_spawnable_class` with a typo in it (nothing would then be spawnable at
    // all), and a cap raised to `boss`, which would put a 7.00 m body into a 7.00 m street.
    // The cap is a USER DECISION taken in his absence (docs/QUESTIONS.md Q-028) — this test
    // is what makes taking it back a visible change rather than a quiet one.
    let d = data();
    let name = &d.scale.titan.max_spawnable_class;
    let cap = d.scale.titan.classes.get(name).unwrap_or_else(|| {
        panic!(
            "titan.max_spawnable_class = {name:?} is not one of {:?}",
            d.scale.titan.classes.keys().collect::<Vec<_>>()
        )
    });
    let above = d
        .scale
        .titan
        .classes
        .values()
        .filter(|c| c.height_m > cap.height_m)
        .count();
    assert!(
        above > 0,
        "the cap {name:?} is the tallest class — then F-064's refusal path is never taken and \
         the test that covers it proves nothing"
    );
    // And the cap has to leave the street usable: width_fraction × height against
    // maps.ron layout.street_m.
    let width = cap.height_m * d.scale.titan.width_fraction;
    for (id, map) in &d.maps.maps {
        assert!(
            width < map.layout.street_m,
            "{id}: a {name} titan is {width} m wide in a {} m street — that is not a tight \
             alley, it is a wall",
            map.layout.street_m
        );
    }
}

// ---------------------------------------------------------------------------
// The file's order is the game's order (FIND-092 §4)
// ---------------------------------------------------------------------------

/// ★ **A list a player reads is the list the file wrote.**
///
/// `missions.ron` puts the difficulties in `recruit → veteran → elite` — easiest first, and
/// that ordering is a design decision, not an accident of spelling. A `BTreeMap` sorts by key
/// and hands the lobby `elite | recruit | veteran`: the hardest level first and the easiest one
/// in the middle (FIND-092 §4, measured on `docs/images/f175-lobby.png`). Same for the
/// templates, where the tutorial `First Ride` came second behind `Ashgate Skirmish`.
///
/// This test is the reason [`OrderedMap`](defeated_by_titan::data::OrderedMap) exists. It goes
/// red against any container that decides the order itself.
#[test]
fn t005_the_missions_keep_the_order_the_file_wrote_them_in() {
    let d = data();

    let templates: Vec<&str> = d.missions.templates.keys().map(String::as_str).collect();
    assert_eq!(
        templates,
        // The three modes were appended on 2026-08-25 (`F-072`, `F-073`, `F-185`) and the two
        // that were here kept their places: the lobby's mission row **is** this list, left to
        // right, and the tutorial has to stay the first thing a new player sees.
        ["tutorial", "skirmish", "breach", "parcours", "escort"],
        "missions.ron lists the tutorial first and the lobby has to offer it first"
    );

    let levels: Vec<&str> =
        d.missions.templates["skirmish"].difficulties.keys().map(String::as_str).collect();
    assert_eq!(
        levels,
        ["recruit", "veteran", "elite"],
        "the difficulty row runs easiest → hardest, because that is how the file runs"
    );
}

/// ★ **The bare binary finds `assets/` — from any working directory and any exe location.**
///
/// Bevy resolves [`AssetPlugin::file_path`] against `BEVY_ASSET_ROOT`, then against the
/// `CARGO_MANIFEST_DIR` **environment variable**, and only then against the executable's own
/// directory (`bevy_asset-0.19.0/src/io/file/mod.rs:19-29`). `cargo run` and `cargo test` both
/// set that variable — so the defect hid in exactly the two places we look, while **every
/// script run in this project starts `./target/debug/defeated_by_titan` directly** and would
/// have got `Path not found: <exe dir>/assets/3d/glb/…`.
///
/// It stayed invisible until 2026-08-18 because nothing had ever gone through the asset
/// server: all eight `art.ron` rows said `Primitive`. The first model row would have rendered
/// nothing and logged one line.
///
/// **The observation is Bevy's, not ours.** `FileAssetReader::new` logs the root *after* it has
/// joined base path and `file_path`, so this reads what the asset server actually uses and not
/// what we handed it. Measured the same day against one mirror asset root, two binaries:
///
/// | binary | what the log said |
/// |---|---|
/// | before | `Path not found: <exe dir>/assets/3d/glb/a-042-…glb` — the entity kept its primitive |
/// | after  | the model loads, no error |
#[test]
fn the_bare_binary_finds_its_assets_from_a_foreign_working_directory() {
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_defeated_by_titan"))
        // **Not the repository.** A run that stands in the repository finds `assets/` by
        // accident through the working directory and proves nothing about the binary.
        .current_dir("/")
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("BEVY_ASSET_ROOT")
        .env("RUST_LOG", "bevy_asset=debug")
        .args(["--headless", "--ticks", "1"])
        .output()
        .expect("the game binary did not start");

    let log = String::from_utf8_lossy(&run.stderr).into_owned()
        + &String::from_utf8_lossy(&run.stdout);
    let want = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");

    let roots: Vec<&str> = log.lines().filter(|l| l.contains("as its base path")).collect();
    assert!(
        !roots.is_empty(),
        "bevy_asset never said which root it uses — `RUST_LOG=bevy_asset=debug` no longer \
         reaches the log, or the message was renamed. Without that line this test proves \
         nothing, so it fails instead of passing quietly.\n{log}"
    );
    for line in &roots {
        assert!(
            line.contains(want),
            "the asset server resolved a root outside the repository.\nwanted: {want}\ngot:    {line}"
        );
    }
    assert!(
        !log.contains("Path not found"),
        "something under assets/ did not resolve in a plain 1-tick run:\n{log}"
    );
    assert!(run.status.success(), "a 1-tick headless run has to end at 0, ended {:?}", run.status);
}

/// ★ **The PNG decoder is named, not inherited.**
///
/// All 311 texture references in the 278 models of the pack are PNG. Until 2026-08-18 the
/// `png` feature came in transitively through `bevy_image`'s own default — so any feature trim
/// anywhere in the tree could have taken the decoder away, and Bevy does not fall over on a
/// format it cannot decode: it logs once and renders the material untextured. A dependency 278
/// files rest on is written down.
#[test]
fn the_png_decoder_is_an_explicit_feature_and_not_a_transitive_one() {
    let manifest = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml is unreadable");

    let list = manifest
        .split("bevy = {")
        .nth(1)
        .and_then(|rest| rest.split("] }").next())
        .expect("Cargo.toml no longer has a `bevy = { … }` dependency block");

    let named = list
        .lines()
        .filter_map(|l| l.split('#').next())
        .any(|code| code.contains("\"png\""));

    assert!(
        named,
        "`png` is not in bevy's feature list — every texture in assets/texturen/ is a PNG, \
         and a missing decoder is silent (docs/models.md)"
    );
}
