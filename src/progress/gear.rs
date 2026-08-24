//! `F-122` — **one budget, four axes, two trade-offs.**
//!
//! The row is explicit about what this must not be: *"instead of 8 independent stat ladders, one
//! shared point budget with conflicting goals: speed costs control, damage costs durability"*.
//! Two mechanics carry that, and both live in `progress.ron`:
//!
//! 1. **Diminishing returns.** A point's effect is `points ^ diminishing_exponent`. Below 1.0 the
//!    tenth point on an axis is worth less than the first, so the strongest build is a spread and
//!    never a dump. At exactly 1.0 the arithmetic is linear and a single-axis dump wins every
//!    time — which is what `tests/progress.rs::f122_the_strongest_build_is_never_a_single_axis_dump`
//!    is there to catch.
//! 2. **Couplings.** Points on the spender are subtracted, at a `drag`, from the axis it costs.
//!    That is the sentence in the row, as a number.
//!
//! ## What this does NOT do yet, and it is the honest boundary of `F-122`
//!
//! **Nothing here moves a gameplay number.** `game.ron: vector` and `gear.ron: blades` are not
//! read through a build, and no system asks this module anything during a sortie. What is built
//! is the budget, the conflict, the legality check and the persistence — the allocation survives
//! quitting (`src/save/profile.rs`). Wiring an axis to a real number is a `vector`/`blades`
//! change and it is deliberately not made from here: a domain that reached into the flight model
//! would be the rule-3 breach, and `docs/FINDINGS.md` FIND-155 says what it would take.
//!
//! Which is also why [`strength_of`] is honest about being a **stand-in**. It weighs a build with
//! `strength_weight` out of the same file the build is tuned in. It can prove that no build
//! structurally dominates; it cannot prove one is fun.

use std::collections::BTreeMap;

use crate::data::GearTuning;

/// What `points` on one axis are worth before anything is taken off them.
fn gain(points: u32, exponent: f32) -> f32 {
    if points == 0 {
        return 0.0;
    }
    (points as f32).powf(exponent)
}

/// The effect of an axis **after** every coupling that costs it has taken its share.
///
/// It can go negative, and that is the point: a build that pours everything into speed does not
/// merely fail to buy control, it *loses* control. An effect clamped at zero would make the
/// trade-off free above a certain investment.
pub fn effect_of(gear: &GearTuning, spent: &BTreeMap<String, u32>, axis: &str) -> f32 {
    let points = |name: &str| spent.get(name).copied().unwrap_or(0);
    let own = gain(points(axis), gear.diminishing_exponent);
    let drag: f32 = gear
        .couplings
        .iter()
        .filter(|c| c.costs == axis)
        .map(|c| c.drag * gain(points(&c.spends), gear.diminishing_exponent))
        .sum();
    own - drag
}

/// **A stand-in for flying the build**, and the only reason it exists is that the acceptance
/// criterion of `F-122` is comparative: *"no dominant build exists; at least 4 builds are within
/// 10 percent"*. See the module header before trusting it for anything else.
pub fn strength_of(gear: &GearTuning, spent: &BTreeMap<String, u32>) -> f32 {
    gear.axes
        .into_iter()
        .map(|(name, axis)| axis.strength_weight * effect_of(gear, spent, name))
        .sum()
}

/// How much of the budget a build uses.
pub fn spent_points(spent: &BTreeMap<String, u32>) -> u32 {
    spent.values().copied().fold(0u32, u32::saturating_add)
}

/// Whether a build may exist: every axis is one the file defines, and the total fits the budget.
///
/// **Both halves are doors from outside.** A save file is written by this program, but it is a
/// file on a disk a player can edit, and an axis that was renamed in `progress.ron` leaves every
/// existing save quoting a name that no longer means anything. The error is a sentence rather
/// than an enum because its only consumers are a log line and a test.
pub fn is_legal(
    gear: &GearTuning,
    spent: &BTreeMap<String, u32>,
    budget: u32,
) -> Result<(), String> {
    let unknown: Vec<&str> = spent
        .keys()
        .filter(|name| !gear.axes.contains_key(name.as_str()))
        .map(String::as_str)
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "the build spends points on {unknown:?}, which progress.ron does not define"
        ));
    }
    let total = spent_points(spent);
    if total > budget {
        return Err(format!("the build spends {total} points of a budget of {budget}"));
    }
    Ok(())
}
