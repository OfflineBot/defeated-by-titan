//! `F-051` the perception model, `F-055` the ring, `F-054` the level of detail.
//!
//! Three features and one system, because all three are answers to the same question: **what
//! does this titan know about the man in the street, and how often does he ask?**
//!
//! ## `F-051` — two channels, and they are not the same shape
//!
//! | channel | shape | speed | source |
//! |---|---|---|---|
//! | the eye | a **cone**: `aggro_radius_m` long, `sight_half_angle_deg` wide | **instant** | the titan's facing |
//! | the ear | a **circle**: `hearing_radius_m` | **accumulates** | the player's own noise |
//!
//! The asymmetry is the whole feature. `F-051`'s acceptance sentence is *"a player who acts
//! quietly is discovered later than one who boosts"*, and it can only ever be true for a titan
//! who is **not looking at you** — so the number that decides it has to be the ear's. A titan
//! staring at a man 30 m in front of him who "has not noticed yet" is not a stealth mechanic,
//! it is a bug report.
//!
//! **What the ear hears is a noise radius in meters**, and the player carries it with him:
//!
//! ```text
//! noise_m = min(max_noise_m, (quiet_m + speed_m_s * noise_per_speed_m) * f)
//! f       = rope_factor while MovementState::Tethered, else 1.0
//! ```
//!
//! That is the gas. Every m/s the vector gear buys is bought with a jet, and
//! [`MovementState::Tethered`] is the state in which the jet is what is carrying him
//! (`docs/architecture.md`: `vector` owns the `Transform` there). A player who cuts the gas and
//! falls is *fast* but not *under power*, and he is quieter by `rope_factor` for it.
//!
//! ⚠️ **The honest gap.** The design says *the sound of gas*; what this reads is
//! **speed and `MovementState`**, both out of `shared/`. It is a proxy, and it is the right
//! one available: a `Gas` delta per player would need `titan` to keep a per-player memory,
//! which is player state living outside `player` (`docs/multiplayer.md` rule 3). The patch
//! that would close it properly is a `shared::Noise` component written by `vector::gas` — it
//! is in this round's report, and `src/shared/` was another hand's file.
//!
//! ## `F-055` — the ring
//!
//! Six titans used to walk the same line to the same point and arrive as one silhouette. A
//! slot is a **rank**, not a negotiation: [`claim_slots`] sorts the titans that share a target
//! by [`TitanId`] and hands out `0..n`. Deterministic on every machine
//! (`docs/multiplayer.md` rule 4), no arbitration message, and no `rand`.
//!
//! ## `F-054` — the level of detail
//!
//! [`Lod`] carries a period in ticks and the number of ticks the brain owes when it next runs.
//! **Nothing is thrown away**: the tick accumulators of `brain::advance` are stepped by
//! [`Lod::steps`] rather than by 1, so a far titan's wind-up still lasts `windup_s` of
//! wall-clock — it is only *decided* on a coarser grid. `brain::walk` is untouched and runs
//! every tick for everybody, which is the feature row's "position interpolation only".

use bevy::prelude::*;

use crate::data::{GameData, TitanCrowd, TitanKind, TitanLod, TitanPerception};
use crate::shared::{MovementState, PlayerId, TitanId, TitanState, Velocity};

use super::brain::TitanTarget;
use super::rig::TitanBody;

/// **How far this titan sees and hears** — baked at spawn, never looked up by name.
///
/// The same argument as [`TitanTuning`](super::brain::TitanTuning)'s: a `BTreeMap<String, _>`
/// lookup per titan per tick costs nothing at three titans and shows up at sixty, which is the
/// feature two doors down (`F-054`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Senses {
    /// The length of the sight cone — `titan.ron: <kind>.aggro_radius_m`.
    pub sight_range_m: f32,
    /// Half its width, in **radians** (degrees in the file, converted at the boundary).
    pub sight_half_angle_rad: f32,
    /// The ceiling on the ear, in meters. What is actually heard is the smaller of this and
    /// the player's own noise radius.
    pub hearing_radius_m: f32,
}

impl Senses {
    pub fn of(kind: &TitanKind) -> Self {
        Senses {
            sight_range_m: kind.aggro_radius_m,
            sight_half_angle_rad: kind.sight_half_angle_deg.to_radians(),
            hearing_radius_m: kind.hearing_radius_m,
        }
    }
}

/// **What this titan currently knows.** One accumulator and one latch, never a `Timer`.
///
/// `level` runs `0.0 ..= 1.0`. Sight pins it to 1.0 in a single tick; the ear pushes it up at
/// `hearing_gain_per_s * strength`; nothing at all drains it at `forget_per_s`.
///
/// [`detected`](Self::detected) is a **latch with hysteresis** and not `level >= 1.0`: without
/// the floor a player who steps out of the cone for one tick resets the whole chase, and the
/// titan flickers between `Pursue` and `Idle` at 60 Hz. It goes true at 1.0 and false again at
/// `perception.lose_level`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Awareness {
    pub level: f32,
    pub detected: bool,
    /// Was the player inside the sight cone **this tick**? Held so a test can tell an eye from
    /// an ear without re-deriving the geometry, and so the F3 overlay can one day say which.
    pub saw: bool,
    /// How loudly he was heard this tick, `0.0 ..= 1.0`. 0 means silence, not "quiet".
    pub heard: f32,
    /// The noise radius the player carried this tick, in meters. **The number `F-051`'s
    /// acceptance is measured in.**
    pub noise_m: f32,
}

/// `F-055` — which of the ring's bearings this titan has been given, out of how many.
///
/// `of == 0` means "nobody else wants this player", and then there is no ring at all: a lone
/// titan walks straight at you, exactly as before.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct CrowdSlot {
    pub index: u32,
    pub of: u32,
}

impl CrowdSlot {
    /// The bearing of this slot, in radians around the target. Slot 0 is the straight line, so
    /// a lone titan and the first of a crowd walk the same path.
    ///
    /// The bearings alternate left/right off the line rather than running round the clock —
    /// `0, +1, -1, +2, -2, …` — so that with two titans the pair splits, instead of one
    /// walking at you and the other walking round to your back while you are still alone with
    /// the first.
    pub fn bearing_rad(&self, slots: u32) -> f32 {
        if self.of <= 1 || slots <= 1 {
            return 0.0;
        }
        let step = std::f32::consts::TAU / slots as f32;
        let half = (self.index as i32 + 1) / 2;
        let sign = if self.index % 2 == 0 { 1.0 } else { -1.0 };
        sign * half as f32 * step
    }
}

/// `F-054` — how often this titan's brain runs, and how much it owes when it does.
///
/// Written by [`perceive`] every tick and read by [`perceive`] itself and by
/// `brain::advance` — one writer, two readers, so the two can never disagree about whether
/// this was a thinking tick.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Lod {
    /// How many ticks apart two brain runs are. 1 = every tick.
    pub period_ticks: u32,
    /// Ticks since the last brain run. **Counted, not derived from `tick % period`**: the
    /// period changes as the titan walks, and a modulo would then skip or double a run at
    /// every tier boundary.
    pub since: u32,
    /// Is this a thinking tick?
    pub due: bool,
    /// How many ticks the brain owes its accumulators this run. Equal to `since` at the moment
    /// it was reset, so `windup_s` still lasts `windup_s` however coarse the grid is.
    pub steps: u32,
}

impl Default for Lod {
    fn default() -> Self {
        // Due on the first tick a titan exists: `Idle` is a state you can observe, but a titan
        // that spends his first three ticks not thinking would enter `Pursue` late and move
        // the tick counts `f050` photographs.
        Lod { period_ticks: 1, since: 1, due: true, steps: 1 }
    }
}

/// The tier a distance falls in, as a period in ticks. **Never zero.**
pub fn period_ticks(lod: &TitanLod, distance_m: f32, simulation_hz: f64) -> u32 {
    let hz = if distance_m <= lod.near_m {
        simulation_hz as f32
    } else if distance_m <= lod.mid_m {
        lod.mid_hz
    } else {
        lod.far_hz
    };
    if hz <= 0.0 {
        return 1;
    }
    ((simulation_hz as f32 / hz).round() as u32).max(1)
}

/// **How loud the player is, as a radius in meters.** Pure, so the test can ask it without an
/// app and without a titan.
pub fn loudness_m(feel: &TitanPerception, speed_m_s: f32, tethered: bool) -> f32 {
    let base = feel.quiet_m + speed_m_s.max(0.0) * feel.noise_per_speed_m;
    let under_power = if tethered { feel.rope_factor } else { 1.0 };
    (base * under_power).min(feel.max_noise_m)
}

/// Is the player inside the sight cone? **On the ground plane** — a titan does not lose you by
/// being above you, and every other cone in this game (`strike_half_angle_deg`,
/// `cortex_half_angle_deg`) is measured the same way.
pub fn sees(senses: &Senses, forward: Vec3, to_player: Vec3) -> bool {
    let flat = Vec3::new(to_player.x, 0.0, to_player.z);
    let distance_m = flat.length();
    if distance_m > senses.sight_range_m {
        return false;
    }
    let f = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let t = flat.normalize_or_zero();
    if f == Vec3::ZERO || t == Vec3::ZERO {
        // Standing on top of him, or a body with no facing: there is nothing left to be blind
        // to.
        return true;
    }
    f.dot(t) >= senses.sight_half_angle_rad.cos()
}

/// How strongly a noise of `noise_m` is heard at `distance_m`: `0.0` outside, rising to `1.0`
/// at the titan's own feet.
///
/// The reach is `min(noise_m, hearing_radius_m)` — **a ceiling on the kind's side and a source
/// on the player's**. That is what makes the bellower's 160 m ear worth anything and a quiet
/// player at 30 m inaudible to it anyway.
pub fn hears(senses: &Senses, noise_m: f32, distance_m: f32) -> f32 {
    let reach = noise_m.min(senses.hearing_radius_m);
    if reach <= 0.0 || distance_m >= reach {
        return 0.0;
    }
    ((reach - distance_m) / reach).clamp(0.0, 1.0)
}

/// One step of the accumulator. `dt` is **`steps / simulation_hz`**, not the frame's delta:
/// see [`Lod`].
pub fn step_awareness(a: &mut Awareness, feel: &TitanPerception, dt: f32) {
    if a.saw {
        a.level = 1.0;
    } else if a.heard > 0.0 {
        a.level = (a.level + feel.hearing_gain_per_s * a.heard * dt).min(1.0);
    } else {
        a.level = (a.level - feel.forget_per_s * dt).max(0.0);
    }
    if a.level >= 1.0 {
        a.detected = true;
    } else if a.level <= feel.lose_level {
        a.detected = false;
    }
}

/// **The one writer of [`TitanTarget`], [`Awareness`] and [`Lod`].**
///
/// It runs before `brain::advance` in `SimulationSystems::Drive` and it is what decides
/// whether `advance` runs at all this tick. Splitting it out of `advance` is what makes
/// `F-051` testable: the awareness of a titan can be read one tick after the player moved,
/// without a state edge having to happen first.
///
/// Rule 6: the loop is titans × players, and the expensive half of it — the geometry and the
/// accumulator — is behind [`Lod::due`]. The cheap half (the distance the tier is chosen from)
/// has to run every tick, or a titan could never leave the far tier.
#[allow(clippy::type_complexity)]
pub(super) fn perceive(
    data: Res<GameData>,
    players: Query<(&PlayerId, &Transform, &Velocity, Option<&MovementState>), Without<TitanBody>>,
    mut bodies: Query<
        (&Transform, &TitanState, &Senses, &mut TitanTarget, &mut Awareness, &mut Lod),
        With<TitanBody>,
    >,
) {
    let hz = data.game.simulation_hz;
    let feel = &data.titans.perception;
    let lod_table = &data.titans.lod;

    for (transform, state, senses, mut target, mut awareness, mut lod) in &mut bodies {
        // The nearest player, every tick and for everybody: it is the number the tier itself is
        // chosen from, and it is two subtractions per player.
        let nearest = nearest_player(&players, transform.translation, feel);

        lod.period_ticks = period_ticks(lod_table, nearest.0.distance_m, hz);
        lod.since = lod.since.saturating_add(1);
        if lod.since < lod.period_ticks {
            lod.due = false;
            lod.steps = 0;
            continue;
        }
        lod.due = true;
        lod.steps = lod.since;
        lod.since = 0;

        // A corpse perceives nothing, and its `TitanTarget` is not read by anything: `walk`
        // and `decide` both leave on `Death` before they reach it.
        if *state == TitanState::Death {
            awareness.saw = false;
            awareness.heard = 0.0;
            continue;
        }

        let (found, noise_m) = nearest;
        *target = found;

        awareness.noise_m = noise_m;
        awareness.saw = found.player.is_some()
            && sees(senses, *transform.forward(), found.pos - transform.translation);
        awareness.heard = if found.player.is_some() {
            hears(senses, noise_m, found.distance_m)
        } else {
            0.0
        };
        let dt = lod.steps as f32 / hz as f32;
        step_awareness(&mut awareness, feel, dt);
    }
}

/// The nearest player on the ground plane, **and how loud he is**.
///
/// Never `.single()` — there are twenty of them one day, and a titan that only ever sees
/// player 1 is a single-player game you notice too late (`docs/multiplayer.md` rule 3). Ties
/// break on the lower [`PlayerId`], so the answer does not depend on iteration order.
fn nearest_player(
    players: &Query<(&PlayerId, &Transform, &Velocity, Option<&MovementState>), Without<TitanBody>>,
    from: Vec3,
    feel: &TitanPerception,
) -> (TitanTarget, f32) {
    let mut best = TitanTarget::default();
    let mut speed_m_s = 0.0f32;
    let mut tethered = false;
    for (id, transform, velocity, movement) in players {
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
            speed_m_s = velocity.speed_m_s();
            tethered = movement == Some(&MovementState::Tethered);
        }
    }
    let loudness =
        if best.player.is_some() { loudness_m(feel, speed_m_s, tethered) } else { 0.0 };
    (best, loudness)
}

/// `F-055` — **hands out the ring's bearings.**
///
/// One pass over the titans that have a target and know it, grouped by the player they are
/// coming for, sorted by [`TitanId`]. The sort is what makes it deterministic: two machines
/// that reach tick *n* hand out the same slots, without a message and without an rng.
///
/// Rule 6: it touches only titans that are actually coming for somebody. A field of idle ones
/// costs one `is_some()` each.
pub(super) fn claim_slots(
    data: Res<GameData>,
    mut bodies: Query<(&TitanId, &TitanTarget, &Awareness, &TitanState, &mut CrowdSlot)>,
) {
    let slots = data.titans.crowd.slots.max(1);
    let mut coming: Vec<(PlayerId, TitanId)> = Vec::new();
    for (id, target, awareness, state, _) in &bodies {
        if *state == TitanState::Death {
            continue;
        }
        if let (Some(player), true) = (target.player, awareness.detected) {
            coming.push((player, *id));
        }
    }
    if coming.is_empty() {
        for (_, _, _, _, mut slot) in &mut bodies {
            *slot = CrowdSlot::default();
        }
        return;
    }
    coming.sort();
    for (id, target, awareness, state, mut slot) in &mut bodies {
        let claim = match (target.player, awareness.detected, *state != TitanState::Death) {
            (Some(player), true, true) => {
                let of = coming.iter().filter(|(p, _)| *p == player).count() as u32;
                let index = coming
                    .iter()
                    .filter(|(p, _)| *p == player)
                    .position(|(_, t)| t == id)
                    .unwrap_or(0) as u32;
                CrowdSlot { index: index % slots, of }
            }
            _ => CrowdSlot::default(),
        };
        if *slot != claim {
            *slot = claim;
        }
    }
}

/// Where the ring puts this titan's aim, as an offset added to the straight line.
///
/// The offset **fades exactly the way `behaviour.flank_offset_m` does** — full beyond twice a
/// kind's own `attack_range_m`, zero at the range itself. Without the fade a ring of titans
/// orbits at `ring_radius_m` forever and nobody ever reaches the man in the middle; the chorus
/// round of 2026-08-19 measured that and the comment on `brain::aim` carries the numbers.
pub fn ring_offset(
    crowd: &TitanCrowd,
    slot: &CrowdSlot,
    to_player: Vec3,
    distance_m: f32,
    attack_range_m: f32,
) -> Vec3 {
    if slot.of <= 1 || crowd.ring_radius_m <= 0.0 || attack_range_m <= 0.0 {
        return Vec3::ZERO;
    }
    let fade = ((distance_m - attack_range_m) / attack_range_m).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return Vec3::ZERO;
    }
    let bearing = slot.bearing_rad(crowd.slots.max(1));
    if bearing == 0.0 {
        return Vec3::ZERO;
    }
    let flat = Vec3::new(to_player.x, 0.0, to_player.z);
    if flat.length_squared() <= f32::EPSILON {
        return Vec3::ZERO;
    }
    // Rotate the LINE, then take the difference: the aim point is the player displaced along
    // the ring, which is a position and not a heading. A rotated heading would make the titan
    // spiral in instead of walking to a place.
    let along = flat.normalize();
    let displaced = Quat::from_rotation_y(bearing) * (-along) * crowd.ring_radius_m;
    displaced * fade
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feel() -> TitanPerception {
        TitanPerception {
            quiet_m: 8.0,
            noise_per_speed_m: 1.2,
            rope_factor: 1.6,
            max_noise_m: 120.0,
            hearing_gain_per_s: 1.5,
            forget_per_s: 0.25,
            lose_level: 0.3,
        }
    }

    fn senses() -> Senses {
        Senses {
            sight_range_m: 45.0,
            sight_half_angle_rad: 110.0f32.to_radians(),
            hearing_radius_m: 35.0,
        }
    }

    /// ★ **The acceptance sentence of `F-051`, as arithmetic.** A boosting player carries a
    /// bigger noise radius than a walking one at the same speed, and a standing one carries
    /// the floor.
    #[test]
    fn f051_gas_is_louder_than_the_same_speed_without_it() {
        let f = feel();
        assert_eq!(loudness_m(&f, 0.0, false), 8.0);
        let falling = loudness_m(&f, 30.0, false);
        let boosting = loudness_m(&f, 30.0, true);
        assert!(boosting > falling, "{boosting} vs {falling}");
        assert!((falling - 44.0).abs() < 1e-3, "{falling}");
        assert!((boosting - 70.4).abs() < 1e-3, "{boosting}");
        // The ceiling really is one.
        assert_eq!(loudness_m(&f, 500.0, true), 120.0);
    }

    /// **The blind spot is behind, which is where the nape is.**
    #[test]
    fn f051_the_cone_is_blind_behind() {
        let s = senses();
        // Bevy's forward is −Z.
        assert!(sees(&s, Vec3::NEG_Z, Vec3::new(0.0, 0.0, -20.0)), "straight ahead");
        assert!(sees(&s, Vec3::NEG_Z, Vec3::new(20.0, 0.0, 0.0)), "90 degrees off, inside 110");
        assert!(!sees(&s, Vec3::NEG_Z, Vec3::new(0.0, 0.0, 20.0)), "dead astern");
        assert!(!sees(&s, Vec3::NEG_Z, Vec3::new(0.0, 0.0, -60.0)), "in the cone but too far");
    }

    /// The ear is the smaller of the two radii, and it is a ramp and not a switch.
    #[test]
    fn f051_the_ear_is_the_smaller_of_the_two_radii() {
        let s = senses();
        assert_eq!(hears(&s, 8.0, 20.0), 0.0, "a quiet player at 20 m");
        assert!(hears(&s, 70.4, 20.0) > 0.0, "a boosting player at 20 m");
        // Capped by the kind's own ear: reach is 35, not 70.4.
        let near = hears(&s, 70.4, 7.0);
        assert!((near - 0.8).abs() < 1e-3, "{near}");
    }
}
