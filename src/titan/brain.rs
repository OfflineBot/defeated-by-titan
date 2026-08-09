//! The reduced state machine (`F-050`), the walk, and the death (`F-056`).
//!
//! ## The FSM is not decoration
//!
//! An enum field that is set correctly while the titan walks and hits at the same time is not
//! a state machine, it is a label. So **everything gates on it**: [`walk`] moves nothing that
//! is not in [`TitanState::Pursue`], and the attack cannot be reached except through
//! `Windup → Strike → Recover`. A "the state changed" assertion passes a label; a tick count
//! on `Windup` does not, which is why
//! `tests/titan.rs::f050_the_husk_winds_up_for_as_long_as_the_file_says` counts.
//!
//! ## Ticks, not seconds
//!
//! `titan.ron` speaks in seconds, the game counts ticks, and the conversion happens **once**,
//! at the boundary, into [`TitanTiming`] — the same rule as
//! [`HitStop`](crate::shared::HitStop)'s. Nothing in here ever reads `Time::delta_secs()`: the
//! step is `1 / game.simulation_hz`, a constant out of the file, so two machines that reach
//! tick *n* stand in the same place (`docs/multiplayer.md` rule 4).
//!
//! ## Two arms of the enum are missing on purpose
//!
//! `Alerted` belongs to `F-051` and `Stagger` to `F-032`. Neither is built, and a variant
//! nothing enters or leaves is exactly the decoration above.
//!
//! ## What this round is NOT
//!
//! **No navigation.** The titan walks in a straight line at whatever it is facing, turning at
//! `turn_deg_per_s`. A path around a house is `F-052` and Round 2; `MoveAndSlide` is the right
//! *collision* tool and the wrong *navigation* tool (`docs/PLAN-GAME.md` §5).

use avian3d::prelude::{Collider, LinearVelocity};
use bevy::prelude::*;

use crate::data::{GameData, TitanKind};
use crate::shared::{
    HitZone, PlayerId, TitanHit, TitanId, TitanState, Velocity,
};

use super::rig::{TitanBody, TitanPart};

/// How long each state lasts, **in ticks**, resolved once at spawn.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitanTiming {
    pub windup_ticks: u32,
    pub strike_ticks: u32,
    pub recover_ticks: u32,
    /// From the start of one `Windup` to the earliest next one.
    pub cooldown_ticks: u32,
    /// How long the body takes to dissolve. The collider goes on tick one regardless.
    pub death_ticks: u32,
}

impl TitanTiming {
    pub fn of(kind: &TitanKind, simulation_hz: f64) -> Self {
        TitanTiming {
            windup_ticks: ticks(kind.windup_s, simulation_hz),
            strike_ticks: ticks(kind.strike_s, simulation_hz),
            recover_ticks: ticks(kind.recover_s, simulation_hz),
            cooldown_ticks: ticks(kind.attack_cooldown_s, simulation_hz),
            death_ticks: ticks(kind.death_s, simulation_hz),
        }
    }
}

/// Seconds from the file into ticks. **Rounded, once**, so that 0.6 s at 60 Hz is 36 ticks and
/// not 35 or 37 depending on where the multiplication happened.
pub fn ticks(seconds: f32, simulation_hz: f64) -> u32 {
    let n = (seconds as f64 * simulation_hz).round();
    if n.is_finite() && n > 0.0 { n as u32 } else { 0 }
}

/// The explicit tick accumulator. **Not a clock, and not a `Timer`.**
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitanClock {
    /// Ticks already completed in the current state. The entry tick of a state is 0.
    pub ticks_in_state: u32,
    /// Ticks before the next `Pursue → Windup` is allowed.
    pub cooldown_left: u32,
}

/// The numbers of one kind that the FSM and the walk need each tick, resolved once at spawn.
///
/// Baked, not looked up by name: a `BTreeMap<String, _>` lookup per titan per tick is the kind
/// of thing that costs nothing at three titans and shows up at sixty (`F-054`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TitanTuning {
    pub speed_m_s: f32,
    pub accel_m_s2: f32,
    pub turn_rad_per_s: f32,
    pub attack_range_m: f32,
    pub aggro_radius_m: f32,
}

impl TitanTuning {
    pub fn of(kind: &TitanKind) -> Self {
        TitanTuning {
            speed_m_s: kind.speed_m_s,
            accel_m_s2: kind.accel_m_s2,
            // Degrees in the file, radians in the code, converted at the boundary
            // (`docs/conventions.md`).
            turn_rad_per_s: kind.turn_deg_per_s.to_radians(),
            attack_range_m: kind.attack_range_m,
            aggro_radius_m: kind.aggro_radius_m,
        }
    }
}

/// Who this titan is walking at, and how far away that is — **on the ground plane**.
///
/// A `PlayerId` and not an `Entity`, because this is the kind of state that goes down a wire
/// one day (`docs/multiplayer.md` rule 5). Written by [`advance`], read by [`walk`], so both
/// see the same target in the same tick.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TitanTarget {
    pub player: Option<PlayerId>,
    pub pos: Vec3,
    pub distance_m: f32,
}

/// Current ground speed. Its own scalar because the direction is the body's facing: a titan
/// walks where it looks, which is what makes `turn_deg_per_s` a feel number at all.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TitanGait {
    pub speed_m_s: f32,
}

/// `TitanHit { zone: Cortex }` → `Death`, and the collider goes **this tick**.
///
/// The cortex kills **by rule**, not by threshold — `shared::message.rs:21` says so, and
/// `Health` is not consulted. Every other zone is ignored here on purpose: the damage curve is
/// `F-031`, it has no calibration (`docs/PLAN-GAME.md` §9.1), and a made-up one would be a
/// number in Rust.
pub(super) fn receive_hits(
    mut commands: Commands,
    mut hits: MessageReader<TitanHit>,
    mut bodies: Query<(Entity, &TitanId, &mut TitanState, &mut TitanClock), With<TitanBody>>,
    children: Query<&Children>,
    parts: Query<&TitanPart>,
) {
    for hit in hits.read() {
        if hit.zone != HitZone::Cortex {
            continue;
        }
        for (root, id, mut state, mut clock) in &mut bodies {
            if *id != hit.titan || *state == TitanState::Death {
                continue;
            }
            *state = TitanState::Death;
            clock.ticks_in_state = 0;
            // **A corpse is never a wall.** The body collider goes now, not when the dissolve
            // is over — a player who cut this titan is flying at 30 m/s and is inside its
            // silhouette on the next tick.
            commands.entity(root).remove::<Collider>();
            // And the cortex goes with it, or a second blade could kill the same titan again.
            let mut pending = vec![root];
            while let Some(entity) = pending.pop() {
                if let Ok(kids) = children.get(entity) {
                    pending.extend(kids.iter());
                }
                if parts.get(entity) == Ok(&TitanPart::Cortex) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

/// One tick of the state machine: pick the target, count the accumulator up, decide the edge.
pub(super) fn advance(
    data: Res<GameData>,
    players: Query<(&PlayerId, &Transform), Without<TitanBody>>,
    mut bodies: Query<
        (
            &Transform,
            &mut TitanState,
            &mut TitanClock,
            &mut TitanTarget,
            &TitanTiming,
            &TitanTuning,
        ),
        With<TitanBody>,
    >,
) {
    let _ = &data; // the numbers are baked; the resource stays so a reload is one line
    for (transform, mut state, mut clock, mut target, timing, tuning) in &mut bodies {
        clock.ticks_in_state = clock.ticks_in_state.saturating_add(1);
        clock.cooldown_left = clock.cooldown_left.saturating_sub(1);

        // Dead bodies do not think. The dissolve reads the same accumulator.
        if *state == TitanState::Death {
            continue;
        }

        *target = nearest_player(&players, transform.translation);

        let next = decide(*state, &clock, timing, tuning, &target);
        if next != *state {
            // Every attack starts the cooldown, not every recovery: `attack_cooldown_s` is the
            // gap between two attacks, and it is shorter than one full attack for no kind.
            if next == TitanState::Windup {
                clock.cooldown_left = timing.cooldown_ticks;
            }
            *state = next;
            clock.ticks_in_state = 0;
        }
    }
}

/// The edges of `F-050`, and **nothing else is an edge.**
///
/// There is deliberately no `Pursue → Strike`: an attack is only ever reachable through its
/// own telegraph. That is what pillar P4 means by "readability before realism", and it is what
/// the tick-count test protects.
pub fn decide(
    state: TitanState,
    clock: &TitanClock,
    timing: &TitanTiming,
    tuning: &TitanTuning,
    target: &TitanTarget,
) -> TitanState {
    let seen = target.player.is_some();
    match state {
        TitanState::Idle => {
            if seen && target.distance_m <= tuning.aggro_radius_m {
                TitanState::Pursue
            } else {
                TitanState::Idle
            }
        }
        TitanState::Pursue => {
            if !seen || target.distance_m > tuning.aggro_radius_m {
                TitanState::Idle
            } else if target.distance_m <= tuning.attack_range_m && clock.cooldown_left == 0 {
                TitanState::Windup
            } else {
                TitanState::Pursue
            }
        }
        TitanState::Windup => {
            if clock.ticks_in_state >= timing.windup_ticks {
                TitanState::Strike
            } else {
                TitanState::Windup
            }
        }
        TitanState::Strike => {
            if clock.ticks_in_state >= timing.strike_ticks {
                TitanState::Recover
            } else {
                TitanState::Strike
            }
        }
        TitanState::Recover => {
            if clock.ticks_in_state >= timing.recover_ticks {
                TitanState::Pursue
            } else {
                TitanState::Recover
            }
        }
        TitanState::Death => TitanState::Death,
    }
}

/// The nearest player on the ground plane. **Never `.single()`** — there are twenty of them
/// one day, and a titan that only ever sees player 1 is a single-player game you notice too
/// late (`docs/multiplayer.md` rule 3). Ties break on the lower `PlayerId`, so the answer does
/// not depend on iteration order.
fn nearest_player(
    players: &Query<(&PlayerId, &Transform), Without<TitanBody>>,
    from: Vec3,
) -> TitanTarget {
    let mut best = TitanTarget::default();
    for (id, transform) in players {
        let to = transform.translation - from;
        let distance_m = Vec3::new(to.x, 0.0, to.z).length();
        let closer = match best.player {
            None => true,
            Some(current) => {
                distance_m < best.distance_m || (distance_m == best.distance_m && *id < current)
            }
        };
        if closer {
            best = TitanTarget { player: Some(*id), pos: transform.translation, distance_m };
        }
    }
    best
}

/// Turn, accelerate, move — **and only in `Pursue`.**
///
/// The one writer of a titan's `Transform`. avian is not the second one: the body is
/// `RigidBody::Kinematic` **and** carries `CustomPositionIntegration`, so `integrate_positions`
/// skips it (`avian3d-0.7.0/src/dynamics/integrator/mod.rs:503-504`). `LinearVelocity` is still
/// written, because the broad phase enlarges a moving body's AABB from it and because `combat`
/// computes the *closing* speed of a cut from it — it is information here, not a drive.
pub(super) fn walk(
    data: Res<GameData>,
    mut bodies: Query<
        (
            &TitanState,
            &TitanTarget,
            &TitanTuning,
            &mut TitanGait,
            &mut Transform,
            &mut LinearVelocity,
            &mut Velocity,
        ),
        With<TitanBody>,
    >,
) {
    // The step comes out of the file, never off a clock: `Time::delta_secs()` in here would
    // make the titan's path depend on the frame rate, and with it the `--offscreen` sha256.
    let dt = (1.0 / data.game.simulation_hz) as f32;

    for (state, target, tuning, mut gait, mut transform, mut linear, mut velocity) in &mut bodies {
        let pursuing = *state == TitanState::Pursue
            && target.player.is_some()
            && target.distance_m > tuning.attack_range_m;

        if !pursuing {
            // Planted. A titan that keeps sliding through its own wind-up is the "FSM as
            // decoration" failure, in one line.
            gait.speed_m_s = 0.0;
            linear.0 = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
            continue;
        }

        // ---- turn -------------------------------------------------------------------
        let to = target.pos - transform.translation;
        let to = Vec3::new(to.x, 0.0, to.z);
        if to.length_squared() > f32::EPSILON {
            // Bevy's forward is −Z, so the yaw that looks at `to` is `atan2(−x, −z)`.
            let wanted = f32::atan2(-to.x, -to.z);
            let current = transform.rotation.to_euler(EulerRot::YXZ).0;
            let mut delta = wanted - current;
            // Take the short way round, or a titan turns 350° to the left instead of 10° to
            // the right and the approach angle stops meaning anything.
            while delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            while delta < -std::f32::consts::PI {
                delta += std::f32::consts::TAU;
            }
            let step = (tuning.turn_rad_per_s * dt).min(delta.abs()) * delta.signum();
            transform.rotation = Quat::from_rotation_y(current + step);
        }

        // ---- accelerate and move ----------------------------------------------------
        gait.speed_m_s = (gait.speed_m_s + tuning.accel_m_s2 * dt).min(tuning.speed_m_s);
        let forward = *transform.forward();
        let step = forward * gait.speed_m_s;
        transform.translation += step * dt;
        linear.0 = step;
        velocity.0 = step;
    }
}

/// `Death` — the body shrinks to nothing over `death_s` and is then gone.
///
/// There is no authored collapse: machine A has no Blender, so `AN-081`'s "collapse, then
/// vaporize" is a box scaled to zero (`docs/PLAN-GAME.md` §10). Scaling the **root** is safe
/// because the collider left on tick one.
pub(super) fn dissolve(
    mut commands: Commands,
    mut bodies: Query<
        (Entity, &TitanState, &TitanClock, &TitanTiming, &mut Transform),
        With<TitanBody>,
    >,
) {
    for (entity, state, clock, timing, mut transform) in &mut bodies {
        if *state != TitanState::Death {
            continue;
        }
        if clock.ticks_in_state >= timing.death_ticks {
            commands.entity(entity).despawn();
            continue;
        }
        let left = if timing.death_ticks == 0 {
            0.0
        } else {
            1.0 - clock.ticks_in_state as f32 / timing.death_ticks as f32
        };
        transform.scale = Vec3::splat(left.max(0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> TitanTiming {
        TitanTiming { windup_ticks: 36, strike_ticks: 12, recover_ticks: 24, cooldown_ticks: 90, death_ticks: 60 }
    }

    fn tuning() -> TitanTuning {
        TitanTuning {
            speed_m_s: 3.0,
            accel_m_s2: 3.0,
            turn_rad_per_s: 50f32.to_radians(),
            attack_range_m: 6.0,
            aggro_radius_m: 45.0,
        }
    }

    fn at(distance_m: f32) -> TitanTarget {
        TitanTarget { player: Some(PlayerId(1)), pos: Vec3::ZERO, distance_m }
    }

    #[test]
    fn six_tenths_of_a_second_is_thirty_six_ticks() {
        assert_eq!(ticks(0.6, 60.0), 36);
        assert_eq!(ticks(0.2, 60.0), 12);
        assert_eq!(ticks(0.4, 60.0), 24);
        // No negative and no NaN duration ever becomes a huge u32.
        assert_eq!(ticks(-1.0, 60.0), 0);
        assert_eq!(ticks(f32::NAN, 60.0), 0);
    }

    #[test]
    fn there_is_no_edge_from_pursue_to_strike() {
        // The edge `F-050` exists to forbid. An attack is only ever reachable through its own
        // telegraph, or the wind-up is a decoration nobody has to respect.
        let clock = TitanClock::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, &timing(), &tuning(), &at(1.0)),
            TitanState::Windup
        );
    }

    #[test]
    fn a_cooldown_holds_the_titan_in_pursue_even_inside_reach() {
        let clock = TitanClock { ticks_in_state: 0, cooldown_left: 7 };
        assert_eq!(
            decide(TitanState::Pursue, &clock, &timing(), &tuning(), &at(1.0)),
            TitanState::Pursue
        );
    }

    #[test]
    fn losing_the_target_falls_back_to_idle() {
        let clock = TitanClock::default();
        let nobody = TitanTarget::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, &timing(), &tuning(), &nobody),
            TitanState::Idle
        );
        assert_eq!(
            decide(TitanState::Idle, &clock, &timing(), &tuning(), &at(99.0)),
            TitanState::Idle
        );
    }

    #[test]
    fn the_attack_runs_windup_strike_recover_and_back_to_pursue() {
        let t = timing();
        let u = tuning();
        for (state, ticks_in_state, wanted) in [
            (TitanState::Windup, 35, TitanState::Windup),
            (TitanState::Windup, 36, TitanState::Strike),
            (TitanState::Strike, 11, TitanState::Strike),
            (TitanState::Strike, 12, TitanState::Recover),
            (TitanState::Recover, 23, TitanState::Recover),
            (TitanState::Recover, 24, TitanState::Pursue),
        ] {
            let clock = TitanClock { ticks_in_state, cooldown_left: 0 };
            assert_eq!(decide(state, &clock, &t, &u, &at(1.0)), wanted, "{state:?} @ {ticks_in_state}");
        }
    }

    #[test]
    fn death_is_a_one_way_street() {
        let clock = TitanClock { ticks_in_state: 9999, cooldown_left: 0 };
        assert_eq!(
            decide(TitanState::Death, &clock, &timing(), &tuning(), &at(1.0)),
            TitanState::Death
        );
    }
}
