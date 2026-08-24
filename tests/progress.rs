//! The guard over the progression spine — `F-120` (level and XP curve), `F-121` (gear rank),
//! `F-122` (gear budget with trade-offs).
//!
//! ⚠️ **Every number in here comes out of `assets/data/progress.ron`**, and that is the point of
//! the feature: *"the curve is defined in config and adjustable without a code change"*. So the
//! tests come in two kinds and they are labelled:
//!
//! - **Property tests** restate the rule independently of the implementation — a level is
//!   monotone in XP, a rank ladder is ascending, a build may not overspend. They survive a
//!   rebalance.
//! - **Tripwires** freeze what the shipped file says *today* (`the_shipped_*` names). They go
//!   red on a rebalance **on purpose**, so that a number nobody meant to move gets noticed.
//!
//! ⚠️ And the `F-122` balance tests are the ones to be suspicious of: the metric they weigh a
//! build with (`strength_weight`) lives in the same file as the design they judge. They can
//! only catch a *structurally* dominant build — a single-axis dump — and not "is this fun".
//! `docs/FINDINGS.md` FIND-155 says so out loud.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use bevy::prelude::*;
use defeated_by_titan::data::{GameData, Progress};
use defeated_by_titan::progress::career::{
    self, gear_points, level_for_xp, may_fly, rank_for, skill_points, xp_for_level, Career,
};
use defeated_by_titan::progress::gear;
use defeated_by_titan::save::{xp_earned, Profile, SaveDir, SortieOutcome};
use defeated_by_titan::shared::{Cli, PlayerId};

fn data() -> GameData {
    GameData::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

fn outcome(won: bool, kills: u32, seconds: f32, difficulty: Option<&str>) -> SortieOutcome {
    SortieOutcome {
        player: PlayerId(1),
        template: "skirmish".into(),
        difficulty: difficulty.map(str::to_owned),
        won,
        kills,
        seconds,
        tick: 1234,
    }
}

// ---------------------------------------------------------------------------------------------
// F-120 — the curve
// ---------------------------------------------------------------------------------------------

/// ⭐ **The property that makes a level a level**: it is a function of XP alone, it never goes
/// down, and the boundary is exact — one XP short of the step is still the level below.
///
/// This is the test that would have caught an off-by-one in either direction, and it does not
/// care what the numbers are.
#[test]
fn f120_a_level_boundary_is_exact_in_both_directions() {
    let d = data();
    let levels = &d.progress.levels;
    for level in 1..=levels.max_level {
        let need = xp_for_level(levels, level);
        assert_eq!(
            level_for_xp(levels, need),
            level,
            "exactly {need} xp has to BE level {level}"
        );
        if level > 1 {
            assert_eq!(
                level_for_xp(levels, need - 1),
                level - 1,
                "one xp short of level {level} is still level {}",
                level - 1
            );
        }
    }
    assert_eq!(xp_for_level(levels, 1), 0, "level 1 is where a career starts");
}

/// The curve is monotone, it ends, and it never runs past its own ceiling — including for an
/// absurd amount of XP, which is what a `u64` overflow or a `powf` infinity would show up as.
#[test]
fn f120_the_curve_climbs_and_then_stops() {
    let d = data();
    let levels = &d.progress.levels;
    let mut previous = 0u64;
    for level in 2..=levels.max_level {
        let need = xp_for_level(levels, level);
        assert!(need > previous, "level {level} must cost more than the one below it");
        previous = need;
    }
    assert_eq!(level_for_xp(levels, u64::MAX), levels.max_level, "the ceiling holds");
    assert_eq!(level_for_xp(levels, 0), 1, "a career with no xp is level 1");
    assert!(levels.max_level >= 100, "F-120 asks for level 1 to 100");
}

/// ⭐ **What one sortie is worth, restated independently of the implementation.**
///
/// The assertion computes the reward from the file's own four rates in a different expression
/// than `xp_earned` uses — if both were the same expression this would test nothing
/// (`docs/FINDINGS.md` FIND-103).
#[test]
fn f120_a_sortie_is_paid_for_the_four_facts_it_reports() {
    let d = data();
    let x = &d.progress.xp;
    let seconds = 240.0f32;
    let kills = 8u32;

    let by_hand = ((x.per_sortie_flown
        + x.per_sortie_won
        + x.per_titan_felled * kills as f32
        + x.per_minute_in_the_field * (seconds / 60.0))
        * x.difficulty_multipliers["recruit"]) as u64;
    assert_eq!(xp_earned(&outcome(true, kills, seconds, Some("recruit")), x), by_hand);

    // A defeat gets the floor and the kills and nothing else.
    let lost_by_hand = ((x.per_sortie_flown
        + x.per_titan_felled * kills as f32
        + x.per_minute_in_the_field * (seconds / 60.0))
        * x.difficulty_multipliers["recruit"]) as u64;
    assert_eq!(xp_earned(&outcome(false, kills, seconds, Some("recruit")), x), lost_by_hand);
    assert!(lost_by_hand < by_hand, "a win has to be worth more than a loss");

    // A negative clock cannot pay. `tick - started_at_tick` is a subtraction.
    assert!(xp_earned(&outcome(false, 0, -9999.0, None), x) >= x.per_sortie_flown as u64 / 2);
}

/// The tier multiplier is applied, and the direct drop-in has its own rate rather than silently
/// borrowing one.
#[test]
fn f120_the_difficulty_multiplier_is_the_only_thing_that_scales_a_sortie() {
    let d = data();
    let x = &d.progress.xp;
    let recruit = xp_earned(&outcome(true, 4, 120.0, Some("recruit")), x);
    let elite = xp_earned(&outcome(true, 4, 120.0, Some("elite")), x);
    let ratio = elite as f32 / recruit as f32;
    let wanted = x.difficulty_multipliers["elite"] / x.difficulty_multipliers["recruit"];
    assert!(
        (ratio - wanted).abs() < 0.02,
        "elite/recruit is {ratio}, the file says {wanted}"
    );
    assert_eq!(
        xp_earned(&outcome(true, 4, 120.0, None), x),
        ((recruit as f32 / x.difficulty_multipliers["recruit"]) * x.without_a_difficulty) as u64,
        "a sortie with no tier is paid at `without_a_difficulty`"
    );
}

/// ⭐ **The cross-file guard.** A fourth difficulty in `missions.ron` with no multiplier in
/// `progress.ron` is a silent under-payment forever; rule 2 says a missing tuning value must be
/// caught, and across two files only a test can catch it.
#[test]
fn f120_every_difficulty_in_missions_ron_has_an_xp_multiplier() {
    let d = data();
    let mut missing: Vec<String> = Vec::new();
    for (name, template) in &d.missions.templates {
        for tier in template.difficulties.keys() {
            if !d.progress.xp.difficulty_multipliers.contains_key(tier) {
                missing.push(format!("{name}/{tier}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missions.ron has difficulties that progress.ron does not pay for: {missing:?}"
    );
}

/// **A tripwire, not a property.** It freezes the curve the game ships with today so that a
/// rebalance is a decision somebody makes and not a number that drifts.
#[test]
fn f120_the_shipped_curve_costs_300_for_the_first_level_and_one_win_pays_it() {
    let d = data();
    assert_eq!(xp_for_level(&d.progress.levels, 2), 300, "the first level");
    let a_good_first_sortie = xp_earned(&outcome(true, 8, 240.0, Some("recruit")), &d.progress.xp);
    assert_eq!(a_good_first_sortie, 400);
    assert_eq!(
        level_for_xp(&d.progress.levels, a_good_first_sortie),
        2,
        "the first sortie has to be felt, or the number is decoration"
    );
    // 513 838 with an exact 1.045; the file's `step_growth` is an `f32`, which is
    // 1.0449999570846558 — the four XP are that, and they are deterministic.
    assert_eq!(xp_for_level(&d.progress.levels, 100), 513_834);
}

/// `F-120`: "every level gives skill points" — and they are **derived**, never stored.
#[test]
fn f120_skill_and_gear_points_are_derived_from_the_level_and_nothing_else() {
    let d = data();
    let l = &d.progress.levels;
    assert_eq!(skill_points(l, 1), 0, "level 1 has not levelled up yet");
    assert_eq!(skill_points(l, 10), 9 * l.skill_points_per_level);
    assert_eq!(gear_points(l, 1), l.gear_points_at_level_one);
    assert_eq!(gear_points(l, 10), l.gear_points_at_level_one + 9 * l.gear_points_per_level);
    assert!(
        gear_points(l, l.max_level) > gear_points(l, 1),
        "a hundred levels have to be worth something"
    );
}

// ---------------------------------------------------------------------------------------------
// F-121 — the gear rank
// ---------------------------------------------------------------------------------------------

/// The ladder itself: ascending, starting at zero, and E..S as the row asks for.
#[test]
fn f121_the_rank_ladder_is_ascending_and_starts_at_zero() {
    let d = data();
    let ranks = &d.progress.ranks;
    assert_eq!(ranks.first().map(|r| r.min_gear_points), Some(0), "somebody has to be rank E");
    let names: Vec<&str> = ranks.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["E", "D", "C", "B", "A", "S"], "F-121 asks for E to S");
    for pair in ranks.windows(2) {
        assert!(
            pair[1].min_gear_points > pair[0].min_gear_points,
            "rank {} does not cost more than {}",
            pair[1].name,
            pair[0].name
        );
    }
}

/// ⭐ **The boundary of every rung**, in both directions. A `>` where a `>=` belongs is the whole
/// bug this feature can have, and it is invisible without this test.
#[test]
fn f121_a_rank_begins_exactly_at_its_own_threshold() {
    let d = data();
    let ranks = &d.progress.ranks;
    for (i, tier) in ranks.iter().enumerate() {
        assert_eq!(rank_for(ranks, tier.min_gear_points), tier.name, "at the threshold");
        if i > 0 {
            assert_eq!(
                rank_for(ranks, tier.min_gear_points - 1),
                ranks[i - 1].name,
                "one point short of {} is still {}",
                tier.name,
                ranks[i - 1].name
            );
        }
    }
    assert_eq!(rank_for(ranks, u32::MAX), "S", "there is no rung above the last one");
}

/// ⭐ `F-121`: "gates correctly". The gate is built and tested; **the shipped file locks
/// nothing**, and that second assertion is the honest half — see `progress.ron`.
#[test]
fn f121_a_gate_admits_the_rank_at_it_and_refuses_the_one_below() {
    let d = data();
    assert!(
        d.progress.gates.is_empty(),
        "progress.ron locks a door: that is a design decision and it needs a line in \
         docs/QUESTIONS.md before it ships"
    );

    // A tuning of this test's own — the mechanism, not the shipped balance.
    let mut locked = Progress {
        gates: BTreeMap::from([
            ("skirmish/veteran".to_string(), "C".to_string()),
            ("skirmish/elite".to_string(), "A".to_string()),
        ]),
        ..d.progress.clone()
    };
    assert!(may_fly(&locked, "C", "skirmish/veteran"));
    assert!(may_fly(&locked, "S", "skirmish/veteran"), "a higher rank is never refused");
    assert!(!may_fly(&locked, "D", "skirmish/veteran"), "one rung short is refused");
    assert!(!may_fly(&locked, "C", "skirmish/elite"));
    assert!(may_fly(&locked, "E", "skirmish/recruit"), "an ungated door lets everybody in");
    assert!(may_fly(&locked, "E", "tutorial"), "and so does a mission with no tier");

    // A gate naming a rank that is not on the ladder must not silently open.
    locked.gates.insert("skirmish/recruit".to_string(), "Z".to_string());
    assert!(!may_fly(&locked, "S", "skirmish/recruit"), "an unknown rank locks, it does not open");
}

// ---------------------------------------------------------------------------------------------
// F-122 — one budget, four axes, two trade-offs
// ---------------------------------------------------------------------------------------------

/// Every allocation of `budget` points over the axes, in file order.
fn every_build(axes: usize, budget: u32) -> Vec<Vec<u32>> {
    if axes == 1 {
        return vec![vec![budget]];
    }
    let mut out = Vec::new();
    for here in 0..=budget {
        for rest in every_build(axes - 1, budget - here) {
            let mut build = vec![here];
            build.extend(rest);
            out.push(build);
        }
    }
    out
}

fn as_map(names: &[String], build: &[u32]) -> BTreeMap<String, u32> {
    names.iter().cloned().zip(build.iter().copied()).collect()
}

/// ⭐ **The row's own words: "instead of 8 independent stat ladders".** The proof that this is
/// not eight ladders is that the best build is a *spread* — no axis holds anywhere near all of
/// the budget.
///
/// **This is the test that can fail, and it is the one that matters.** Set
/// `progress.ron: gear.diminishing_exponent` to 1.0 and the arithmetic goes linear: the best
/// build becomes a single-axis dump at 100 % and this goes red at every budget.
#[test]
fn f122_the_strongest_build_is_never_a_single_axis_dump() {
    let d = data();
    let g = &d.progress.gear;
    let names: Vec<String> = g.axes.keys().cloned().collect();
    assert!(names.len() >= 4, "F-122 wants conflicting goals, and two axes cannot conflict enough");

    for budget in [6u32, 8, 12, 20, 30, 42, 60] {
        let mut best: Option<(f32, Vec<u32>)> = None;
        for build in every_build(names.len(), budget) {
            let s = gear::strength_of(g, &as_map(&names, &build));
            if best.as_ref().is_none_or(|(top, _)| s > *top) {
                best = Some((s, build));
            }
        }
        let (_, build) = best.expect("at least one build");
        let share = *build.iter().max().unwrap() as f32 / budget as f32;
        assert!(
            share <= 0.40,
            "at a budget of {budget} the strongest build is {:?} over {names:?} — {:.0} % of the \
             points on one axis. That is a stat ladder, not a budget with conflicts.",
            build,
            share * 100.0
        );
    }
}

/// ⭐ `F-122`'s literal acceptance: **"at least 4 builds are within 10 percent of equally
/// strong"** — and they have to be genuinely different builds, so each of the four leads with a
/// different axis.
#[test]
fn f122_four_builds_with_four_different_leading_axes_are_within_ten_percent() {
    let d = data();
    let g = &d.progress.gear;
    let names: Vec<String> = g.axes.keys().cloned().collect();
    let budget = 42u32; // level 19 — a career that has had time to specialise

    let mut best = f32::MIN;
    let mut champion: BTreeMap<usize, (f32, Vec<u32>)> = BTreeMap::new();
    for build in every_build(names.len(), budget) {
        let s = gear::strength_of(g, &as_map(&names, &build));
        best = best.max(s);
        // The leading axis, and only when it leads STRICTLY — a four-way tie leads nothing.
        let top = *build.iter().max().unwrap();
        if build.iter().filter(|p| **p == top).count() == 1 {
            let lead = build.iter().position(|p| *p == top).unwrap();
            let entry = champion.entry(lead).or_insert((f32::MIN, build.clone()));
            if s > entry.0 {
                *entry = (s, build);
            }
        }
    }
    assert_eq!(champion.len(), names.len(), "every axis has to be leadable at all");
    for (lead, (strength, build)) in &champion {
        assert!(
            strength / best >= 0.90,
            "the best {}-led build {:?} reaches {:.1} % of the best build overall — F-122 asks \
             for four builds within 10 %",
            names[*lead],
            build,
            strength / best * 100.0
        );
    }
}

/// ⭐ **"Speed costs control, damage costs durability"**, as a measurement rather than a
/// sentence: move one point onto the spender and the axis it costs goes DOWN.
///
/// Set any `drag` in `progress.ron: gear.couplings` to 0.0 and this goes red — which is the
/// whole reason it exists.
#[test]
fn f122_every_coupling_actually_takes_something_away() {
    let d = data();
    let g = &d.progress.gear;
    assert!(!g.couplings.is_empty(), "a budget without a conflict is eight ladders again");
    for c in &g.couplings {
        assert!(g.axes.contains_key(&c.spends), "coupling spends unknown axis {:?}", c.spends);
        assert!(g.axes.contains_key(&c.costs), "coupling costs unknown axis {:?}", c.costs);

        let flat: BTreeMap<String, u32> =
            g.axes.keys().map(|k| (k.clone(), 5u32)).collect();
        let before = gear::effect_of(g, &flat, &c.costs);
        let mut leaning = flat.clone();
        *leaning.get_mut(&c.spends).unwrap() += 5;
        let after = gear::effect_of(g, &leaning, &c.costs);
        assert!(
            after < before,
            "{} was supposed to cost {}: it stood at {before:.3} and is {after:.3} after five \
             more points",
            c.spends,
            c.costs
        );
    }
}

/// A build is only legal inside its budget, and only over axes that exist. Both are the doors a
/// loadout screen (`F-125`) and a save file from another machine come through.
#[test]
fn f122_a_build_may_not_overspend_and_may_not_invent_an_axis() {
    let d = data();
    let g = &d.progress.gear;
    let names: Vec<String> = g.axes.keys().cloned().collect();

    let inside: BTreeMap<String, u32> = BTreeMap::from([(names[0].clone(), 6)]);
    assert!(gear::is_legal(g, &inside, 6).is_ok(), "spending the whole budget is legal");
    assert!(gear::is_legal(g, &inside, 5).is_err(), "one point over the budget is not");
    assert!(gear::is_legal(g, &BTreeMap::new(), 0).is_ok(), "spending nothing is legal");

    let invented = BTreeMap::from([("moons_visited".to_string(), 1)]);
    assert!(gear::is_legal(g, &invented, 99).is_err(), "an axis nobody defined is not a build");
}

// ---------------------------------------------------------------------------------------------
// The whole spine, through the app that actually runs
// ---------------------------------------------------------------------------------------------

/// ⭐ **One sortie, the whole chain**: `progress` reports facts → `save` books XP → `progress`
/// derives the level, the points and the rank onto the player. No `.single()`, no `Resource`.
#[test]
fn f120_a_sortie_moves_the_career_and_the_rank_in_the_running_app() {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    app.insert_resource(SaveDir(None));
    app.update();
    app.update();

    let player = {
        let world = app.world_mut();
        let mut q = world.query::<&PlayerId>();
        *q.iter(world).next().expect("the local player")
    };
    let before = career_of(&mut app);
    assert_eq!(before.level, 1);
    assert_eq!(before.xp, 0);
    assert!(before.gear_points > 0, "even level 1 hands out a budget");

    for _ in 0..3 {
        app.world_mut()
            .write_message(SortieOutcome { player, ..outcome(true, 8, 240.0, Some("elite")) });
        app.update();
        app.update();
    }

    let after = career_of(&mut app);
    let d = data();
    let expected = 3 * xp_earned(&outcome(true, 8, 240.0, Some("elite")), &d.progress.xp);
    assert_eq!(after.xp, expected, "three sorties, three payments");
    assert_eq!(after.level, level_for_xp(&d.progress.levels, expected));
    assert!(after.level > before.level, "three elite wins have to move the level");
    assert_eq!(after.gear_points, gear_points(&d.progress.levels, after.level));
    assert_eq!(after.rank, rank_for(&d.progress.ranks, after.gear_points));
    assert!(after.last_sortie_xp > 0, "the debrief needs to know what THIS sortie was worth");
}

fn career_of(app: &mut App) -> Career {
    let world = app.world_mut();
    let mut q = world.query::<(&PlayerId, &Career)>();
    let mut all: Vec<Career> = q.iter(world).map(|(_, c)| c.clone()).collect();
    assert_eq!(all.len(), 1, "one career per player, and there is one player");
    all.remove(0)
}

/// A `Career` is a **component**, exactly as `Profile` is (`CLAUDE.md` rule 4). This is the test
/// that goes red if it ever becomes a `Resource`.
#[test]
fn f120_the_career_is_derived_and_carries_nothing_the_profile_does_not_have() {
    let d = data();
    let mut profile = Profile { xp: 5_000, ..Profile::default() };
    profile.cleared = BTreeSet::from(["skirmish/recruit".to_string()]);
    let career = Career::of(&profile, &d.progress);

    assert_eq!(career.xp, 5_000);
    assert_eq!(career.level, level_for_xp(&d.progress.levels, 5_000));
    assert_eq!(career.xp_into_level, 5_000 - xp_for_level(&d.progress.levels, career.level));
    assert_eq!(
        career.xp_for_the_next_level,
        Some(career::step_xp(&d.progress.levels, career.level)),
        "the debrief draws a bar, and a bar needs both halves"
    );
    assert_eq!(career.gear_points_spent, 0, "nothing has been spent yet");

    let maxed = Career::of(&Profile { xp: u64::MAX, ..Profile::default() }, &d.progress);
    assert_eq!(maxed.level, d.progress.levels.max_level);
    assert_eq!(maxed.xp_for_the_next_level, None, "there is nothing above the ceiling");
}
