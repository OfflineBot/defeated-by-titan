//! `F-031` — **damage comes out of speed**, and `F-044`'s ground attack is the exception that
//! proves it.
//!
//! ## The hole this file fills, and it was a double one
//!
//! `gear.ron: blades.damage_per_m_s = 1.4` had **no reader anywhere in `src/`** until
//! 2026-08-25 — `grep -rn damage_per_m_s src/` found the field, the doc comment and two test
//! literals, and not one line that multiplied by it. That is the exact shape
//! `docs/FINDINGS.md` FIND-075 records for `wear_per_hit`, and it had a second half nobody had
//! joined it to: `titan::rig` hangs `Health::full(titan.ron: <kind>.health)` on every body it
//! builds, and **nothing in the game had ever written that component**. A number with no
//! reader on one side, a pool with no writer on the other.
//!
//! `src/hud/hit_mark.rs` says so out loud in its own header — *"`F-031` (the damage formula)
//! is unbuilt, and `gear.ron: blades.damage_per_m_s` has no reader. Printing an invented `142`
//! would be the lie"* — which is why `F-043`'s hit mark prints a **speed**. It can print a
//! damage number now; that change is `hud`'s and is reported, not taken here.
//!
//! ## The formula
//!
//! ```text
//! damage = blades.damage_per_m_s x closing_m_s x zone factor x combo multiplier
//! ```
//!
//! **`closing_m_s` is where the row's "Schnittwinkel" already lives.** `blades::cut::closing_speed`
//! is `max(0, (v_player − v_titan) · d̂)` — the relative velocity *projected on the direction the
//! blade actually swept*. A player flying past parallel to a running titan closes on nothing and
//! books nothing; the same flight straight into him books all of it. So the angle term is not
//! missing, it is upstream, and it is upstream in the one place that can measure it: the message
//! carries a scalar because `shared::TitanHit` has no room for a geometry (see **Open** below).
//!
//! ## 🔴 Emptying the pool is **not** a kill, and it never becomes one
//!
//! `docs/gameplay/pillars.md` and the bible are not negotiable here: *a Titan dies only from a
//! fast cut into the Cortex.* [`zone_factor`] returns **0.0** for [`HitZone::Cortex`] for that
//! reason — the lethal zone books no wound damage at all, so no arithmetic in this file can
//! ever reach the decision `titan::brain::receive_hits` makes by rule. A regression here is a
//! `0.0` turning into a number, and
//! `tests/combat.rs::f031_the_cortex_books_no_wound_damage_at_all` is what goes red on it.
//!
//! What the pool buys instead is `docs/gameplay/enemies.md`'s own sentence made spendable —
//! *"every other hit zone is preparation, not damage"*: cut a titan enough and **he goes
//! down**, `gear.ron: damage.collapse_s` long, and his nape is then a target that is standing
//! still. Then the wounds steam shut, the pool is full again, and
//! `gear.ron: damage.collapse_refractory_s` says he cannot be put back on the floor for four
//! seconds — a window, not a stun lock, and lock-free *by construction* rather than by an
//! arithmetic argument that depends on six other numbers at once.
//!
//! ## Where in the tick this runs
//!
//! [`SimulationSystems::Spatial`], **after** [`super::hitstop::begin`] and after
//! [`super::combo::bank`]. `blades::cut` writes [`TitanHit`] in `PostStep`, so the earliest
//! deterministic place to read it is the first stage of the next tick — the same seam
//! `hitstop::begin` and `titan::brain::receive_hits` already sit on. The two `.after()`s are
//! not decoration: `begin` and this system both hold `&mut HitStop`, and `bank` has to have
//! raised the multiplier before it is read.
//!
//! ## The evidence
//!
//! | what | how |
//! |---|---|
//! | the formula, the cortex zero, the collapse, the lock guard | `tests/combat.rs`, `cargo test --test combat` |
//! | a husk floored by three chest cuts, and a ground attack that is worth 5 | `scripts/f031-damage.txt` |

use bevy::prelude::*;

use crate::data::{DamageTuning, GameData, Gear};
use crate::shared::{
    Health, HitStop, HitZone, PlayerId, SimulationSystems, Tick, TitanHit, TitanId, TitanState,
};

use super::combo::Combo;
use super::hitstop::Stagger;

/// **The zone's share of the hit** — and [`HitZone::Cortex`] is deliberately `0.0`.
///
/// The cortex is not "the zone with the biggest factor". It is the zone that is decided
/// somewhere else entirely, by rule, in `titan::brain::receive_hits`, and it must never appear
/// in a sum that a tuning pass could move. Giving it a factor is how a damage formula quietly
/// becomes the thing that kills titans.
pub fn zone_factor(t: &DamageTuning, zone: HitZone) -> f32 {
    match zone {
        HitZone::Cortex => 0.0,
        HitZone::Torso => t.zone_torso_factor,
        HitZone::Head => t.zone_head_factor,
        HitZone::Eye => t.zone_eye_factor,
        HitZone::ArmLeft | HitZone::ArmRight | HitZone::LegLeft | HitZone::LegRight => {
            t.zone_limb_factor
        }
    }
}

/// `F-044` — **was this hit made from the ground?**
///
/// It is read off the message and off nothing else. `blades::cut` refuses a hit under
/// `gear.ron: blades.min_speed_m_s` *unless* the player is standing on the ground, so a
/// reported hit below that floor is a ground attack and there is no other way to produce one.
///
/// Deliberately **not** a `MovementState` lookup on `hit.by`: this system runs one tick after
/// the cut, and a player who jumped in between would have his ground attack silently repriced
/// as an airborne one. A pure function of the message cannot drift.
pub fn is_ground_attack(gear: &Gear, speed_m_s: f32) -> bool {
    speed_m_s < gear.blades.min_speed_m_s
}

/// The whole formula, as one function, so a test can ask it the question the acceptance
/// criterion asks: *"ein Schnitt bei doppelter Geschwindigkeit erzeugt mindestens 60 Prozent
/// mehr Schaden"*.
///
/// `multiplier` is floored at 1.0: a combo can only ever add. A `NaN` or a negative anywhere
/// in the chain returns `0.0` rather than healing the titan — the same defensive shape
/// [`Health::damage`] carries for the same reason.
pub fn damage_of(gear: &Gear, zone: HitZone, speed_m_s: f32, multiplier: f32) -> f32 {
    let factor = zone_factor(&gear.damage, zone);
    if !factor.is_finite() || factor <= 0.0 {
        return 0.0;
    }
    let base = if is_ground_attack(gear, speed_m_s) {
        gear.damage.ground_damage
    } else {
        gear.blades.damage_per_m_s * speed_m_s
    };
    let multiplier = if multiplier.is_finite() { multiplier.max(1.0) } else { 1.0 };
    let damage = base * factor * multiplier;
    if damage.is_finite() && damage > 0.0 { damage } else { 0.0 }
}

/// Seconds into ticks. **Rounded, once, at the boundary** — the same arithmetic as
/// [`super::hitstop::stagger_ticks`] and `blades::swing::ticks`.
pub fn ticks_of(seconds: f32, simulation_hz: f64) -> u32 {
    let n = (seconds as f64 * simulation_hz).round();
    if n.is_finite() && n > 0.0 { n as u32 } else { 0 }
}

/// **A titan who has just been floored may not be floored again while this is on him.**
///
/// The whole no-stun-lock claim is this component and not an inequality between numbers. See
/// `gear.ron: damage.collapse_refractory_s` for why the arithmetic version was rejected: it is
/// already false for the scuttler at the shipped values.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollapseGuard {
    pub ticks_left: u32,
}

/// Reads [`TitanHit`], books the damage on the titan's wound pool and floors him at zero.
#[allow(clippy::type_complexity)]
pub fn apply(
    mut commands: Commands,
    data: Res<GameData>,
    tick: Res<Tick>,
    mut hits: MessageReader<TitanHit>,
    // `Without<TitanId>` on both halves, or Bevy refuses the system: two queries that both ask
    // for a component are only disjoint if the filters say so (`B0001`). Nothing is a player
    // and a titan at once, so the filter is a statement of fact and not a workaround.
    players: Query<(&PlayerId, Option<&Combo>), Without<TitanId>>,
    mut titans: Query<
        (
            Entity,
            &TitanId,
            &mut Health,
            Option<&TitanState>,
            Option<&Stagger>,
            Option<&mut HitStop>,
            Option<&CollapseGuard>,
        ),
        Without<PlayerId>,
    >,
) {
    let gear = &data.gear;
    let hz = data.game.simulation_hz;
    for hit in hits.read() {
        // The multiplier as it stands **after** `combo::bank` has counted this very hit, which
        // is why that system is ordered before this one: the first hit of a chain is `x1.00`
        // by `1 + step * (n - 1)`, so counting first and reading second gives the row's own
        // sentence without an off-by-one anywhere.
        let multiplier = players
            .iter()
            .find(|(id, _)| **id == hit.by)
            .and_then(|(_, combo)| combo)
            .map_or(1.0, |c| c.multiplier);
        let damage = damage_of(gear, hit.zone, hit.speed_m_s, multiplier);
        if damage <= 0.0 {
            // The cortex, every time. It is decided in `titan::brain::receive_hits` by rule and
            // nothing here may touch it.
            continue;
        }
        for (entity, id, mut health, state, stagger, stop, guard) in &mut titans {
            if *id != hit.titan {
                continue;
            }
            // A corpse has no wound pool. `titan::brain::receive_hits` already stripped his
            // colliders on the tick the cortex was cut, so this is belt and braces — but a
            // dissolving body is in the world for `death_s` seconds and a second blade can
            // still be swinging through where it was.
            if state == Some(&TitanState::Death) {
                continue;
            }
            let left = health.damage(damage);
            info!(
                "tick {}: titan {} took {:.1} in the {:?} at {:.1} m/s (x{:.2}) — pool {:.0}/{:.0}",
                tick.0, id.0, damage, hit.zone, hit.speed_m_s, multiplier, left, health.max
            );
            if left > 0.0 {
                continue;
            }
            if guard.is_some() {
                // Floored recently. The pool is drained and refilled and he stays on his feet —
                // that is the refractory period doing its one job.
                health.current = health.max;
                continue;
            }
            let floor_ticks = ticks_of(gear.damage.collapse_s, hz);
            let refractory = ticks_of(gear.damage.collapse_refractory_s, hz);
            // **The wounds steam shut.** Refilled here and not when he gets up, so that the
            // pool is never observably zero and no reader has to know what zero means.
            health.current = health.max;
            commands.entity(entity).insert(CollapseGuard { ticks_left: refractory.max(1) });
            if floor_ticks > 0 {
                match stop {
                    // **The longer of the two, never the sum**, exactly as `hitstop::begin`
                    // treats a stagger. A cut that floors a titan already staggered him one
                    // stage earlier in this same tick.
                    Some(mut existing) => {
                        existing.ticks_left = existing.ticks_left.max(floor_ticks)
                    }
                    None => {
                        commands.entity(entity).insert(HitStop::new(floor_ticks));
                    }
                }
            }
            info!(
                "tick {}: titan {} COLLAPSED — {} ticks on the floor, {} of refractory (his own \
                 stagger is {} ticks)",
                tick.0,
                id.0,
                floor_ticks,
                refractory,
                stagger.map_or(0, |s| s.ticks)
            );
        }
    }
}

/// One tick off every refractory window, and the component goes at zero.
///
/// [`SimulationSystems::PostStep`], the same stage and the same shape as
/// [`super::hitstop::advance`] — counting down before the step would give a window that is one
/// tick shorter than the file asks for.
pub fn advance_guards(mut commands: Commands, mut guarded: Query<(Entity, &mut CollapseGuard)>) {
    for (entity, mut guard) in &mut guarded {
        guard.ticks_left = guard.ticks_left.saturating_sub(1);
        if guard.ticks_left == 0 {
            commands.entity(entity).remove::<CollapseGuard>();
        }
    }
}

/// Registered from [`super::CombatPlugin`].
pub fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        apply
            .in_set(SimulationSystems::Spatial)
            // Both hold `&mut HitStop`. Two domains may not order each other; two systems of
            // the same domain must.
            .after(super::hitstop::begin)
            // The multiplier has to be banked before it is read.
            .after(super::combo::bank),
    )
    .add_systems(FixedUpdate, advance_guards.in_set(SimulationSystems::PostStep));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped file, read the way `data::GameData::load` reads it — not a hand-built
    /// fixture. A formula test against invented numbers proves the formula and says nothing
    /// about the game (`docs/FINDINGS.md` FIND-103).
    fn gear() -> Gear {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data");
        GameData::load(&dir).gear
    }

    /// The acceptance sentence of `F-031`, as arithmetic, with no ECS in the way:
    /// *"Ein Schnitt bei doppelter Geschwindigkeit erzeugt mindestens 60 Prozent mehr
    /// Schaden."* The whole-app version is in `tests/combat.rs`; this one is what still runs
    /// when the app will not build.
    #[test]
    fn double_the_speed_is_at_least_sixty_percent_more_damage() {
        let gear = gear();
        let slow = damage_of(&gear, HitZone::Torso, 12.0, 1.0);
        let fast = damage_of(&gear, HitZone::Torso, 24.0, 1.0);
        assert!(slow > 0.0, "a 12 m/s chest cut books nothing at all");
        assert!(
            fast >= slow * 1.6,
            "F-031: {slow:.1} at 12 m/s against {fast:.1} at 24 m/s is only {:.0} % more — the \
             row asks for 60",
            (fast / slow - 1.0) * 100.0
        );
    }

    /// The one line that must never become a number.
    #[test]
    fn the_cortex_never_books_wound_damage() {
        assert_eq!(
            damage_of(&gear(), HitZone::Cortex, 75.0, 4.0),
            0.0,
            "the cortex booked wound damage — a titan dies from the nape by rule, and a \
             formula that can reach that decision is a second way to kill"
        );
    }

    /// `F-044`'s acceptance, as arithmetic: *"niemals die effizientere Wahl"*. The cheapest
    /// airborne cut a player can produce is one at exactly `min_speed_m_s`.
    #[test]
    fn a_ground_attack_is_worth_less_than_the_cheapest_airborne_cut() {
        let gear = gear();
        let floor = gear.blades.min_speed_m_s;
        let ground = damage_of(&gear, HitZone::Torso, floor - 0.01, 1.0);
        let airborne = damage_of(&gear, HitZone::Torso, floor, 1.0);
        assert!(ground > 0.0, "F-044: the ground attack is worth nothing at all");
        assert!(
            ground < airborne,
            "F-044: a ground attack books {ground:.1} against {airborne:.1} for the slowest \
             cut a flying player can make — the row says it may never be the better choice"
        );
    }

    /// The no-stun-lock claim, at the file level: he is up longer than he is down.
    #[test]
    fn the_refractory_window_is_longer_than_the_collapse() {
        let d = gear().damage;
        assert!(
            d.collapse_refractory_s > d.collapse_s,
            "gear.ron: damage.collapse_s {} against collapse_refractory_s {} — a titan that \
             can be floored again before he has got up is a stun lock",
            d.collapse_s,
            d.collapse_refractory_s
        );
    }

    /// Every zone answers, and only one of them answers zero.
    #[test]
    fn every_zone_but_the_cortex_is_worth_something() {
        let t = gear().damage;
        for zone in [
            HitZone::Torso,
            HitZone::Head,
            HitZone::Eye,
            HitZone::ArmLeft,
            HitZone::ArmRight,
            HitZone::LegLeft,
            HitZone::LegRight,
        ] {
            assert!(
                zone_factor(&t, zone) > 0.0,
                "gear.ron: damage — {zone:?} is worth nothing, so a cut into it is a cut into \
                 nothing"
            );
        }
    }
}
