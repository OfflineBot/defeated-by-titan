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
//! ## The one thing in here that is not the titan's own
//!
//! [`walk`] and [`dissolve`] read [`HitStop`](crate::shared::HitStop), which `combat` writes.
//! That is not an edge into `combat` — the component lives in `shared/` precisely so that the
//! two ends of an impact frame do not have to know each other — but it *is* the only place
//! where something outside this domain stops a titan, and it is here because nothing else can:
//! `RigidBodyDisabled` does nothing to a body avian never integrates. See [`walk`].
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
    HitStop, HitZone, PlayerId, StateClock, TitanHit, TitanId, TitanState, Velocity,
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

/// The part of the accumulator that is **this domain's own business**: the attack cooldown.
///
/// "How far into the current state" is *not* in here — it is
/// [`StateClock`](crate::shared::StateClock) in `shared/`, because `debug` has to print it and
/// `combat` may one day gate on it, and neither may reach into `titan/`. It was moved rather
/// than mirrored: two fields holding the same number are two fields that disagree the first
/// time somebody adds an edge and updates one of them (§5 rule 4, one writer per field).
///
/// `cooldown_left` stays because nothing outside `titan/` has any business with it — it is the
/// gap between two attacks, not a readable state of the body.
///
/// Still the explicit tick accumulator, **not a clock and not a `Timer`.**
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitanClock {
    /// Ticks before the next `Pursue → Windup` is allowed.
    pub cooldown_left: u32,
}

/// How long a state lasts, out of the timings resolved from `titan.ron` at spawn.
///
/// **The single source of `StateClock::state_ticks`.** It stands here, next to
/// [`decide`], because the number a state is compared against and the number that is printed
/// under it have to be the same number — a total computed a second time next to the overlay is
/// how `n/36` survives somebody changing `windup_s`.
///
/// `Idle` and `Pursue` have no length: they end when the world ends them, and 0 is what
/// [`StateClock`](crate::shared::StateClock) reads as "open-ended".
pub fn duration_ticks(state: TitanState, timing: &TitanTiming) -> u32 {
    match state {
        TitanState::Idle | TitanState::Pursue => 0,
        TitanState::Windup => timing.windup_ticks,
        TitanState::Strike => timing.strike_ticks,
        TitanState::Recover => timing.recover_ticks,
        TitanState::Death => timing.death_ticks,
    }
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
    mut bodies: Query<
        (Entity, &TitanId, &mut TitanState, &mut StateClock, &TitanTiming),
        With<TitanBody>,
    >,
    children: Query<&Children>,
    parts: Query<&TitanPart>,
) {
    for hit in hits.read() {
        if hit.zone != HitZone::Cortex {
            continue;
        }
        for (root, id, mut state, mut clock, timing) in &mut bodies {
            if *id != hit.titan || *state == TitanState::Death {
                continue;
            }
            *state = TitanState::Death;
            // The same pair as every other edge, from the same place: the dissolve reads
            // `ticks_in_state` and the overlay reads both, so `Death 0/60` is readable on the
            // very tick the cortex was cut.
            *clock = StateClock::entering(duration_ticks(TitanState::Death, timing));
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
///
/// **The one writer of [`StateClock`](crate::shared::StateClock)**, and it writes both of its
/// fields on the same line as the state they belong to. That is what lets the F3 overlay print
/// `husk#1 Windup 21/36` and have the fraction mean the pose in the same frame: `pose::apply_pose`
/// runs right after this system in `SimulationSystems::Drive`, off the same component, in the
/// same tick.
///
/// It runs in `FixedUpdate` and nowhere else. In `Update` the count would follow the frame rate
/// instead of the tick, the pose would go with it, and
/// `tests/titan.rs::f050_the_pose_does_not_depend_on_the_clock` is what falls over when it does.
#[allow(clippy::type_complexity)]
pub(super) fn advance(
    data: Res<GameData>,
    players: Query<(&PlayerId, &Transform), Without<TitanBody>>,
    mut bodies: Query<
        (
            &Transform,
            &mut TitanState,
            &mut StateClock,
            &mut TitanClock,
            &mut TitanTarget,
            &TitanTiming,
            &TitanTuning,
        ),
        With<TitanBody>,
    >,
) {
    let _ = &data; // the numbers are baked; the resource stays so a reload is one line
    for (transform, mut state, mut clock, mut cooldown, mut target, timing, tuning) in &mut bodies {
        clock.ticks_in_state = clock.ticks_in_state.saturating_add(1);
        cooldown.cooldown_left = cooldown.cooldown_left.saturating_sub(1);

        // Dead bodies do not think. The dissolve reads the same accumulator.
        if *state == TitanState::Death {
            continue;
        }

        *target = nearest_player(&players, transform.translation);

        let next = decide(*state, &clock, cooldown.cooldown_left, timing, tuning, &target);
        if next != *state {
            // Every attack starts the cooldown, not every recovery: `attack_cooldown_s` is the
            // gap between two attacks, and it is shorter than one full attack for no kind.
            if next == TitanState::Windup {
                cooldown.cooldown_left = timing.cooldown_ticks;
            }
            *state = next;
            // Counter and total together, out of the same timings the edge above was decided
            // on. Setting only the counter is how an overlay ends up printing `0/36` under a
            // `Strike` that lasts twelve ticks.
            *clock = StateClock::entering(duration_ticks(next, timing));
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
    clock: &StateClock,
    cooldown_left: u32,
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
            } else if target.distance_m <= tuning.attack_range_m && cooldown_left == 0 {
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
///
/// ## Why this system reads [`HitStop`]
///
/// Because nothing else can stop this titan. `combat::hitstop::begin` freezes the two bodies of
/// a hit by putting `HitStop` on them and `RigidBodyDisabled` on the player — but disabling a
/// rigid body does nothing to a titan, whose position avian never integrates
/// (`RigidBody::Kinematic` + `CustomPositionIntegration`, see [`super::rig::build_rig`]). Its
/// own comment says so and names this line. Without it a graze freezes the player and the titan
/// walks on through his own impact frame, which is the one thing an impact frame must not do.
#[allow(clippy::type_complexity)]
pub(super) fn walk(
    data: Res<GameData>,
    mut bodies: Query<
        (
            &TitanState,
            &TitanTarget,
            &TitanTuning,
            Option<&HitStop>,
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

    for (state, target, tuning, stop, mut gait, mut transform, mut linear, mut velocity) in
        &mut bodies
    {
        if stop.is_some_and(HitStop::is_frozen) {
            // **The impact frame, on the body that was hit.** `gait.speed_m_s` is deliberately
            // NOT reset: a hit stop is a frozen frame, not a stumble, and the titan carries on
            // at the speed he had. The two velocities do go to zero, because they describe what
            // this body does *this* tick and this tick it does nothing — `blades::cut` reads
            // `Velocity` for the closing speed of the next cast.
            linear.0 = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
            continue;
        }

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
///
/// It reads [`HitStop`] for the same reason [`walk`] does, and the case is the loudest one there
/// is: `hit_stop_cortex_s` is 0.12 s and the freeze begins on the very tick the titan dies. A
/// corpse that keeps shrinking through the impact frame of its own kill is the one hit stop in
/// the game the player is guaranteed to be looking at. **The death clock is not frozen** — that
/// is [`advance`]'s accumulator — so the body still vanishes after `death_s`; only the shrink
/// pauses.
pub(super) fn dissolve(
    mut commands: Commands,
    mut bodies: Query<
        (Entity, &TitanState, &StateClock, &TitanTiming, Option<&HitStop>, &mut Transform),
        With<TitanBody>,
    >,
) {
    for (entity, state, clock, timing, stop, mut transform) in &mut bodies {
        if *state != TitanState::Death || stop.is_some_and(HitStop::is_frozen) {
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
        let clock = StateClock::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, 0, &timing(), &tuning(), &at(1.0)),
            TitanState::Windup
        );
    }

    #[test]
    fn a_cooldown_holds_the_titan_in_pursue_even_inside_reach() {
        let clock = StateClock::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, 7, &timing(), &tuning(), &at(1.0)),
            TitanState::Pursue
        );
    }

    #[test]
    fn losing_the_target_falls_back_to_idle() {
        let clock = StateClock::default();
        let nobody = TitanTarget::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, 0, &timing(), &tuning(), &nobody),
            TitanState::Idle
        );
        assert_eq!(
            decide(TitanState::Idle, &clock, 0, &timing(), &tuning(), &at(99.0)),
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
            let clock = StateClock { ticks_in_state, state_ticks: duration_ticks(state, &t) };
            assert_eq!(
                decide(state, &clock, 0, &t, &u, &at(1.0)),
                wanted,
                "{state:?} @ {ticks_in_state}"
            );
        }
    }

    #[test]
    fn death_is_a_one_way_street() {
        let clock = StateClock { ticks_in_state: 9999, state_ticks: 60 };
        assert_eq!(
            decide(TitanState::Death, &clock, 0, &timing(), &tuning(), &at(1.0)),
            TitanState::Death
        );
    }

    /// **The total the overlay prints is the total the FSM compares against.**
    ///
    /// `duration_ticks` is the one place `StateClock::state_ticks` comes from, and every state
    /// with a length in `titan.ron` is exactly the number [`decide`] ends that state on. Goes
    /// red the moment somebody adds a state with a duration and forgets it here — which would
    /// show up in a picture as `Strike 4/0`, a fraction the overlay then quietly leaves off.
    #[test]
    fn every_timed_state_reports_the_length_it_is_ended_on() {
        let t = timing();
        for (state, wanted) in [
            (TitanState::Idle, 0),
            (TitanState::Pursue, 0),
            (TitanState::Windup, t.windup_ticks),
            (TitanState::Strike, t.strike_ticks),
            (TitanState::Recover, t.recover_ticks),
            (TitanState::Death, t.death_ticks),
        ] {
            assert_eq!(duration_ticks(state, &t), wanted, "{state:?}");
        }

        // The two-sided half: for every state that HAS a length, that length is where `decide`
        // hands over. A constant typed into `duration_ticks` would pass the loop above.
        let u = tuning();
        for state in [TitanState::Windup, TitanState::Strike, TitanState::Recover] {
            let total = duration_ticks(state, &t);
            let last_inside = StateClock { ticks_in_state: total - 1, state_ticks: total };
            let first_after = StateClock { ticks_in_state: total, state_ticks: total };
            assert_eq!(
                decide(state, &last_inside, 0, &t, &u, &at(1.0)),
                state,
                "{state:?} ended one tick before its own `state_ticks`"
            );
            assert_ne!(
                decide(state, &first_after, 0, &t, &u, &at(1.0)),
                state,
                "{state:?} ran past the `state_ticks` the overlay prints under it"
            );
        }
    }
}
