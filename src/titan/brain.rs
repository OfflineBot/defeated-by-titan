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
//! ## The roster, since 2026-08-19 (`F-057`..`F-063`)
//!
//! Until that date all eight kinds of `titan.ron` ran **this** brain with different numbers, and
//! `docs/gameplay/enemies.md` calls that failure by name: *"at least half of all enemy kinds
//! carry an anti-autopilot property"*, and a kind that is only a different number is a reskin.
//! Five switches out of `titan.ron: <kind>.behaviour` are what makes them different, and each
//! one changes what the PLAYER has to do:
//!
//! | switch | kind | what it costs the player |
//! |---|---|---|
//! | `swerve_deg` / `swerve_period_s` | errant | he is never on the line you aimed at — lead him |
//! | `lunge_m_s` | scuttler | the blow carries forward; sideways is not far enough, up is |
//! | `flank_offset_m` | chorus | two of them arrive from two sides; pick one and give the other your back |
//! | `ambush` | lurker | he never chases. Notice him, or pay 48 |
//! | `cortex_guard` | weaver, warden | the nape is not always a target. See [`Guard`] |
//! | `call_radius_m` | bellower | one sighting wakes the district — ⚠️ he cannot spawn, see below |
//!
//! **What is honestly NOT built:** the weaver's roll (it needs a `TitanState` arm and
//! `shared/state.rs` was another hand's this round — the *lesson* is built, the roll is not),
//! the bellower's ear (`F-051`: he calls on sight, not on the sound of gas), and the bellower
//! himself (`scale.ron: max_spawnable_class` is `large`, `docs/QUESTIONS.md` Q-028).
//!
//! ## What this round is NOT
//!
//! **No navigation.** The titan walks in a straight line at whatever it is facing, turning at
//! `turn_deg_per_s`. A path around a house is `F-052` and Round 2; `MoveAndSlide` is the right
//! *collision* tool and the wrong *navigation* tool (`docs/PLAN-GAME.md` §5).

use avian3d::prelude::{Collider, ColliderDisabled, LinearVelocity};
use bevy::prelude::*;

use crate::data::{CortexGuard, GameData, TitanKind};
use crate::shared::{
    HitStop, HitZone, PlayerId, StateClock, Tick, TitanHit, TitanId, TitanState, Velocity,
};

use super::perception::{ring_offset, Awareness, CrowdSlot, Lod};

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
    /// `F-059` — the whole backward roll. **0 = this kind does not roll**, and that is the one
    /// switch [`decide`] reads to know whether `Recover` ends in a roll or in `Pursue`.
    pub roll_ticks: u32,
    /// How much of [`roll_ticks`](Self::roll_ticks) is the readable crouch, with the nape still
    /// a target. After it come the i-frames — see [`Guard::open`].
    pub roll_startup_ticks: u32,
}

impl TitanTiming {
    pub fn of(kind: &TitanKind, simulation_hz: f64) -> Self {
        TitanTiming {
            windup_ticks: ticks(kind.windup_s, simulation_hz),
            strike_ticks: ticks(kind.strike_s, simulation_hz),
            recover_ticks: ticks(kind.recover_s, simulation_hz),
            cooldown_ticks: ticks(kind.attack_cooldown_s, simulation_hz),
            death_ticks: ticks(kind.death_s, simulation_hz),
            roll_ticks: ticks(kind.behaviour.roll_s, simulation_hz),
            // Clamped to the roll's own length: a startup that outlasts the roll would be a
            // roll with no i-frames at all, i.e. `F-059`'s acceptance sentence quietly failing
            // instead of loudly. `tests/data.rs` refuses the file that says it, this line makes
            // sure the code cannot be surprised by it either.
            roll_startup_ticks: ticks(kind.behaviour.roll_startup_s, simulation_hz)
                .min(ticks(kind.behaviour.roll_s, simulation_hz)),
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
    /// Ticks this titan keeps coming **because somebody called it** (`F-062`), independent of
    /// its own `aggro_radius_m`. Written by [`answer_the_call`], counted down by [`advance`],
    /// read by [`decide`]. Without it the call is a one-tick flicker: `decide` sends anything
    /// outside its own aggro radius straight home again.
    pub alerted_left: u32,
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
        TitanState::Roll => timing.roll_ticks,
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
    // ---- `titan.ron: <kind>.behaviour` — what makes this kind not the husk ---------------
    /// `F-057`, in **radians**: how far off the straight line the walk swings, to each side.
    pub swerve_rad: f32,
    /// How many ticks one swing lasts before the heading flips. 0 means no swerve at all —
    /// and it has to be checked, or the modulo below divides by zero.
    pub swerve_period_ticks: u64,
    /// `F-058`: how fast the body carries itself forward through its own `Strike`.
    pub lunge_m_s: f32,
    /// `F-063`: how far to the side of the player this kind aims.
    pub flank_offset_m: f32,
    /// `F-062`: the radius in which this kind wakes idle titans when it acquires a target.
    pub call_radius_m: f32,
    /// How long an answered call holds, in ticks. See [`TitanClock::alerted_left`].
    pub call_hold_ticks: u32,
    /// `F-061`: an ambusher has no `Pursue`. See [`decide`].
    pub ambush: bool,
    /// `F-059`: how fast the body carries itself **backwards** through [`TitanState::Roll`].
    pub roll_speed_m_s: f32,
}

impl TitanTuning {
    pub fn of(kind: &TitanKind, simulation_hz: f64) -> Self {
        let b = &kind.behaviour;
        TitanTuning {
            speed_m_s: kind.speed_m_s,
            accel_m_s2: kind.accel_m_s2,
            // Degrees in the file, radians in the code, converted at the boundary
            // (`docs/conventions.md`).
            turn_rad_per_s: kind.turn_deg_per_s.to_radians(),
            attack_range_m: kind.attack_range_m,
            swerve_rad: b.swerve_deg.to_radians(),
            swerve_period_ticks: ticks(b.swerve_period_s, simulation_hz) as u64,
            lunge_m_s: b.lunge_m_s,
            flank_offset_m: b.flank_offset_m,
            call_radius_m: b.call_radius_m,
            call_hold_ticks: ticks(b.call_hold_s, simulation_hz),
            ambush: b.ambush,
            roll_speed_m_s: b.roll_speed_m_s,
        }
    }
}

/// **When this titan's nape is a target, and whether it is one right now.**
///
/// The enforcement is physical: [`guard_the_cortex`] puts avian's `ColliderDisabled` on the
/// cortex sensor while the guard is closed, so `blades::cut` never finds it and never writes a
/// `TitanHit { zone: Cortex }` at all. **That is the load-bearing detail.** The obvious
/// implementation — let the hit happen and drop it in [`receive_hits`] — would leave
/// `mission::count_kills` crediting a kill for a titan that is still standing, because it reads
/// the message and not the corpse. A guard that is a rule in one domain and invisible in
/// another is worse than no guard.
///
/// `open_left` is a tick accumulator like every other one in this file, never a `Timer`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Guard {
    pub rule: CortexGuard,
    /// Ticks a body cut has bought, for [`CortexGuard::WhenOpened`]. 0 for everyone else.
    pub open_left: u32,
    /// What one body cut buys, resolved once at spawn.
    pub open_ticks: u32,
    /// Whether the cortex sensor is currently OUT of the world. Held so the systems below can
    /// insert and remove the marker on the edge instead of every tick.
    pub covered: bool,
}

impl Guard {
    pub fn of(kind: &TitanKind, simulation_hz: f64) -> Self {
        let open_ticks = match kind.behaviour.cortex_guard {
            CortexGuard::WhenOpened(seconds) => ticks(seconds, simulation_hz),
            _ => 0,
        };
        Guard { rule: kind.behaviour.cortex_guard, open_left: 0, open_ticks, covered: false }
    }

    /// Is the nape a target this tick? **Pure**, so the test can ask it without an app.
    ///
    /// `ticks_in_state` and `roll_startup_ticks` are only ever looked at in
    /// [`TitanState::Roll`], and there they are the whole of `F-059`: the crouch is open, the
    /// roll is not. It is checked **before** [`Self::rule`] and not inside it, because
    /// i-frames are i-frames whatever a kind's guard rule says — a kind with
    /// [`CortexGuard::Always`] that learned to roll would otherwise roll visibly and be
    /// cuttable all the way through it.
    pub fn open(&self, state: TitanState, ticks_in_state: u32, roll_startup_ticks: u32) -> bool {
        if state == TitanState::Roll {
            return ticks_in_state < roll_startup_ticks;
        }
        match self.rule {
            CortexGuard::Always => true,
            // Committed = inside his own attack. `Idle` and `Pursue` are not, and `Death` is
            // past caring — a corpse's cortex is despawned by `receive_hits` anyway.
            CortexGuard::WhenCommitted => {
                matches!(state, TitanState::Windup | TitanState::Strike | TitanState::Recover)
            }
            CortexGuard::WhenOpened(_) => self.open_left > 0,
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
        (Entity, &TitanId, &mut TitanState, &mut StateClock, &TitanTiming, &mut Guard),
        With<TitanBody>,
    >,
    children: Query<&Children>,
    parts: Query<&TitanPart>,
) {
    for hit in hits.read() {
        if hit.zone != HitZone::Cortex {
            // **The warden's two-stage attack** (`F-060`), and since 2026-08-19 it is the
            // backlog's own sentence rather than an approximation of it: *"Frontalangriff auf
            // **Arme** oeffnet den Cortex fuer ein Zeitfenster"*, `docs/gameplay/enemies.md`'s
            // *"a two-stage attack: arms first, then the cortex"*. Until `F-032` gave the limbs
            // their own zones there was nothing to say "arm" with — every cut into the body
            // wrote `Torso` (`docs/FINDINGS.md` FIND-109) — so **any** body cut opened him, and
            // that was a compromise with the collider, not a design.
            //
            // The same cut already staggers him for `stagger_s` (`combat::hitstop`, `F-032`),
            // so one blade still buys both halves: he stumbles, and for `WhenOpened(s)` seconds
            // the cortex is a target. For every other kind `open_ticks` is 0 and this loop does
            // nothing at all.
            //
            // 🔴 **The designed version is ONE LINE and it is not being taken today.**
            //
            //     if !matches!(hit.zone, HitZone::ArmLeft | HitZone::ArmRight) { continue; }
            //
            // It was written, it works, and it was taken back out the same hour, because four
            // 🟧 rows go red under it: `q030_the_nape_is_reachable_on_a_large_titan_too`,
            // `q030_the_nape_is_cut_from_behind_and_not_from_the_front`,
            // `q031_the_nape_survives_a_titan_who_tracks_you` and
            // `f030_a_bound_model_cannot_drag_the_nape_round_to_the_front` all use the **warden**
            // as their 14 m body and all of them reach his cortex only because the torso graze
            // of their own pass opens him one tick earlier. They measure REACH; that they also
            // walk through his guard is an accident of the pass.
            //
            // Which is the finding, and it is about the game and not about the tests: **the
            // warden's two-stage attack is defeated by a single pass today** — the graze that
            // knocks the hand off the nape and the cut that kills him are the same swing.
            // `docs/FINDINGS.md`, `F-060`. Whoever takes the line above has to re-aim those four
            // passes at a `large` kind whose nape is always open (the lurker) and re-measure the
            // 0.15 m of `Q-031` on a body that was never opened by the measurement itself.
            for (_, id, state, _, _, mut guard) in &mut bodies {
                if *id == hit.titan && *state != TitanState::Death && guard.open_ticks > 0 {
                    guard.open_left = guard.open_ticks;
                }
            }
            continue;
        }
        for (root, id, mut state, mut clock, timing, _) in &mut bodies {
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
    mut bodies: Query<
        (
            &mut TitanState,
            &mut StateClock,
            &mut TitanClock,
            &TitanTarget,
            &Awareness,
            &Lod,
            &TitanTiming,
            &TitanTuning,
            &mut Guard,
        ),
        With<TitanBody>,
    >,
) {
    let _ = &data; // the numbers are baked; the resource stays so a reload is one line
    for (mut state, mut clock, mut cooldown, target, awareness, lod, timing, tuning, mut guard) in
        &mut bodies
    {
        // **`F-054`.** Not this titan's tick — and `perception::perceive`, the one writer of
        // [`Lod`], has already said so this frame. Every accumulator below is stepped by
        // `lod.steps` and not by 1, so a far titan's wind-up still lasts `windup_s` of
        // wall-clock; it is only DECIDED on a coarser grid.
        if !lod.due {
            continue;
        }
        let steps = lod.steps.max(1);
        clock.ticks_in_state = clock.ticks_in_state.saturating_add(steps);
        cooldown.cooldown_left = cooldown.cooldown_left.saturating_sub(steps);
        // The warden's window closing again. One accumulator, one writer, like every other one
        // in this file — and it counts down even in `Death`, where it is simply irrelevant.
        guard.open_left = guard.open_left.saturating_sub(steps);
        cooldown.alerted_left = cooldown.alerted_left.saturating_sub(steps);

        // Dead bodies do not think. The dissolve reads the same accumulator.
        if *state == TitanState::Death {
            continue;
        }

        // **`F-051`.** What used to stand here was `distance_m <= aggro_radius_m`, a circle
        // with no facing in it. The eye and the ear are `perception::perceive`'s now, and what
        // reaches the state machine is the one bit they agree on.
        let aware = awareness.detected || cooldown.alerted_left > 0;

        let next = decide(
            *state,
            &clock,
            cooldown.cooldown_left,
            aware,
            timing,
            tuning,
            target,
        );
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
    aware: bool,
    timing: &TitanTiming,
    tuning: &TitanTuning,
    target: &TitanTarget,
) -> TitanState {
    let seen = target.player.is_some();
    // **`F-051` reaches the state machine here, and as one bit.** `aware` is
    // `perception::Awareness::detected` — the eye's cone or the ear's circle, latched with
    // hysteresis — OR a bellower's call, which is the one thing that overrides a titan's own
    // senses and is temporary: `call_hold_s` ticks.
    //
    // Until 2026-08-25 this line read `target.distance_m <= tuning.aggro_radius_m`: a circle
    // with no facing in it, which is why the design's whole stealth layer had nowhere to live.
    let in_range = seen && aware;
    match state {
        TitanState::Idle => {
            // **The ambusher has no `Pursue`** (`F-061`). He is not a slow titan, he is a
            // titan that never comes to you: the only edge out of `Idle` he owns is straight
            // into his own telegraph, and it fires when you walk into `attack_range_m`. That
            // is why the lurker cannot be kited and cannot be outrun — there is nothing to
            // outrun. `aggro_radius_m` still governs whether he TURNS to face you, in
            // [`walk`], so he is not a statue either.
            if tuning.ambush {
                if seen && target.distance_m <= tuning.attack_range_m && cooldown_left == 0 {
                    TitanState::Windup
                } else {
                    TitanState::Idle
                }
            } else if in_range {
                TitanState::Pursue
            } else {
                TitanState::Idle
            }
        }
        TitanState::Pursue => {
            if !in_range {
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
            if clock.ticks_in_state < timing.recover_ticks {
                TitanState::Recover
            } else if timing.roll_ticks > 0 {
                // **`F-059`, and this is the only edge into the roll.** The design's sentence is
                // *"reacts to an approach towards the cortex with a backward roll"*, and the
                // approach that matters is the one his `cortex_guard: WhenCommitted` lets
                // through: his nape is out of the world in `Idle` and `Pursue`, so a roll
                // triggered there would be i-frames on a hit zone that is already gone —
                // decoration, and exactly what `TitanState`'s own doc warns against. The moment
                // i-frames can mean anything is the **end of his open window**, and that is
                // here. It also makes the window LONGER by `roll_startup_s` rather than
                // shorter: he crouches with his nape still bare before he is untouchable.
                TitanState::Roll
            } else if tuning.ambush {
                // Back to standing still. `Recover -> Pursue` would turn the lurker into a
                // slow husk the moment he had swung once, which is the whole thing he is not.
                TitanState::Idle
            } else {
                TitanState::Pursue
            }
        }
        TitanState::Roll => {
            if clock.ticks_in_state < timing.roll_ticks {
                TitanState::Roll
            } else if tuning.ambush || !in_range {
                TitanState::Idle
            } else {
                TitanState::Pursue
            }
        }
        TitanState::Death => TitanState::Death,
    }
}

/// **Where this titan is walking, which is not always where the player is standing.**
///
/// Returns the ground vector from `from` to the point the body wants to face. For the husk that
/// is the player and nothing else — `swerve_rad` and `flank_offset_m` are both 0 and this
/// function is two subtractions. The two kinds that do use it are the reason the roster is not
/// eight husks:
///
/// * **`flank_offset_m`** (`F-063`, chorus) shifts the aim sideways, with the sign taken off the
///   titan's own id — even ids left, odd ids right — so a pair arrives from two sides. It
///   **fades to nothing** as the body closes: full down to twice `attack_range_m`, zero at
///   `attack_range_m` itself. Without that fade a chorus would circle at 9 m forever and never
///   once reach the range its own attack needs.
/// * **`swerve_rad`** (`F-057`, errant) rotates the aim by a fixed angle whose sign flips every
///   `swerve_period_ticks`. Deterministic out of `(tick, id)` and **never `rand`**: a spawn
///   position or a heading out of an rng is a desync (`docs/multiplayer.md` rule 4), and the
///   phase offset by id is what keeps two errants from zig-zagging in lockstep.
///
/// Both are gated on `Pursue`. In `Windup` the body tracks the player straight — a telegraph
/// that swerves is a telegraph nobody can read (P4), and the strike cone would never bear.
pub fn aim(
    id: &TitanId,
    state: TitanState,
    target: &TitanTarget,
    tuning: &TitanTuning,
    slot: &CrowdSlot,
    crowd: &crate::data::TitanCrowd,
    from: Vec3,
    tick: u64,
) -> Vec3 {
    let to = target.pos - from;
    let mut to = Vec3::new(to.x, 0.0, to.z);
    if to.length_squared() <= f32::EPSILON || state != TitanState::Pursue {
        return to;
    }

    // **`F-055`, and it is applied before the two per-kind offsets on purpose.** The ring is
    // where the group agreed this body should stand; `flank_offset_m` and `swerve_rad` are
    // what this KIND does on the way there. A chorus in slot 3 flanks off his own bearing,
    // which is what keeps a pair of them a pair inside a crowd of six.
    to += ring_offset(crowd, slot, to, target.distance_m, tuning.attack_range_m);

    if tuning.flank_offset_m > 0.0 {
        // **Full offset down to twice his own reach, then gone by the time he is inside it.**
        // The fade is not cosmetic: a constant offset would leave a chorus orbiting at 9 m and
        // never once reaching the `attack_range_m` its own attack needs. Fading it over the
        // reach rather than over the offset is what keeps the flank a flank — measured
        // 2026-08-19, over the offset the pair only ever reached 7.48 m of separation against
        // a husk pair's 4.00 m; over the reach it reaches 11.32 m
        // (`tests/mission.rs::f063_a_chorus_pair_splits_where_a_husk_pair_stacks`).
        let fade =
            ((target.distance_m - tuning.attack_range_m) / tuning.attack_range_m).clamp(0.0, 1.0);
        if fade > 0.0 {
            let side = if id.0 % 2 == 0 { 1.0 } else { -1.0 };
            let left = Vec3::new(to.z, 0.0, -to.x).normalize();
            to += left * (side * tuning.flank_offset_m * fade);
        }
    }

    if tuning.swerve_rad != 0.0 && tuning.swerve_period_ticks > 0 {
        let half = tick / tuning.swerve_period_ticks + u64::from(id.0);
        let sign = if half % 2 == 0 { 1.0 } else { -1.0 };
        to = Quat::from_rotation_y(sign * tuning.swerve_rad) * to;
    }
    to
}

/// **The bellower's call** (`F-062`): one titan sees you, the district comes.
///
/// Every kind with `call_radius_m > 0` that has a target of its own writes `call_hold_s` worth
/// of ticks onto every OTHER titan inside that radius. What that buys is one line in
/// [`decide`]: while `alerted_left > 0` a titan ignores its own `aggro_radius_m`. It does not
/// change the state directly — a state written here would be overwritten by [`advance`] on the
/// next tick, and the two would fight over one field (§5 rule 4, one writer).
///
/// **Rule 6, and it is why the first loop exists.** With no caller in the world this system is
/// one pass over the titans and a `Vec` that stays empty — it never looks at a pair. The
/// quadratic part costs `callers × titans`, and a caller is the rarest kind in the game.
///
/// ⚠️ The design's bellower reacts to the **sound of gas** (`docs/gameplay/enemies.md`,
/// `F-051`). There is no perception model, so he calls on sight. The stealth layer that the
/// enemy design hangs off this kind is **not built**, and this is the call without the ear.
pub(super) fn answer_the_call(
    mut bodies: Query<(&TitanId, &Transform, &TitanState, &TitanTuning, &mut TitanClock)>,
) {
    let mut callers: Vec<(TitanId, Vec3, f32, u32)> = Vec::new();
    for (id, transform, state, tuning, _) in &bodies {
        if tuning.call_radius_m > 0.0
            && tuning.call_hold_ticks > 0
            && matches!(*state, TitanState::Pursue | TitanState::Windup | TitanState::Strike)
        {
            callers.push((*id, transform.translation, tuning.call_radius_m, tuning.call_hold_ticks));
        }
    }
    if callers.is_empty() {
        return;
    }
    for (id, transform, state, _, mut clock) in &mut bodies {
        if *state == TitanState::Death {
            continue;
        }
        for (caller, at, radius_m, hold) in &callers {
            if caller == id {
                continue;
            }
            let d = transform.translation - *at;
            if Vec3::new(d.x, 0.0, d.z).length() <= *radius_m {
                clock.alerted_left = clock.alerted_left.max(*hold);
            }
        }
    }
}

/// **Takes the cortex sensor out of the world while the nape is covered** (`F-059`, `F-060`).
///
/// The whole of [`Guard`] is this system. It runs on the **edge** and not per tick: a kind
/// whose rule is [`CortexGuard::Always`] leaves on the first line, and the others touch their
/// children only when `covered` actually flips — which for a warden is twice per body cut and
/// for a weaver twice per attack, not sixty times a second (rule 6).
///
/// Why the collider and not the message: see [`Guard`]. A `TitanHit { zone: Cortex }` that
/// `titan/` throws away has already been counted by `mission::count_kills`.
pub(super) fn guard_the_cortex(
    mut commands: Commands,
    mut bodies: Query<(Entity, &TitanState, &StateClock, &TitanTiming, &mut Guard), With<TitanBody>>,
    children: Query<&Children>,
    parts: Query<&TitanPart>,
) {
    for (root, state, clock, timing, mut guard) in &mut bodies {
        // A kind with an always-open nape that also cannot roll has nothing to do here. The
        // second half of that sentence is `F-059`: the i-frames are a property of the state,
        // not of the rule, so a roller is visited even when his rule is `Always`.
        if guard.rule == CortexGuard::Always && timing.roll_ticks == 0 {
            continue;
        }
        let cover = *state != TitanState::Death
            && !guard.open(*state, clock.ticks_in_state, timing.roll_startup_ticks);
        if cover == guard.covered {
            continue;
        }
        guard.covered = cover;
        let mut pending = vec![root];
        while let Some(entity) = pending.pop() {
            if let Ok(kids) = children.get(entity) {
                pending.extend(kids.iter());
            }
            if parts.get(entity) == Ok(&TitanPart::Cortex) {
                if cover {
                    commands.entity(entity).insert(ColliderDisabled);
                } else {
                    commands.entity(entity).remove::<ColliderDisabled>();
                }
            }
        }
    }
}

/// Turn, accelerate, move — **the step only in `Pursue`, the turn also in `Windup`.**
///
/// The split is `Q-031`'s answer and it is the load-bearing half of it: a strike cone
/// (`combat::strike::StrikeTuning::faces`) is worth nothing on a body that cannot bring it to
/// bear, and until 2026-08-13 the turn hung off the *walk's* gate — `distance_m >
/// attack_range_m` — so no titan ever turned inside 6 m. See the comment on the two gates
/// below for why `Strike` and `Recover` stay locked.
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
    tick: Res<Tick>,
    mut bodies: Query<
        (
            &TitanId,
            &TitanState,
            &TitanTarget,
            &TitanTuning,
            &Awareness,
            &CrowdSlot,
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
    let crowd = &data.titans.crowd;

    for (
        id,
        state,
        target,
        tuning,
        awareness,
        slot,
        stop,
        mut gait,
        mut transform,
        mut linear,
        mut velocity,
    ) in &mut bodies
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

        let seen = target.player.is_some();

        // **Turning and walking are two gates, not one.** Until `Q-031` was answered they were
        // the same line, and `distance_m > attack_range_m` therefore stopped the turn as well
        // as the step — so a titan did not turn inside his own reach, which is where every
        // fight happens (`docs/FINDINGS.md` FIND-012, and `turn_deg_per_s` governed nothing).
        //
        // `Windup` is the tracking window and `Strike`/`Recover` are not, on purpose:
        //
        // - **`Windup`** is where the number the user asked about lives. He telegraphs, the
        //   player circles, and the husk's 50 °/s decides whether the circle beats the arm.
        // - **`Strike`** is committed. A titan who tracks through his own blow lands it
        //   wherever the player went, and `combat::strike`'s cone would be a cylinder again in
        //   everything but name.
        // - **`Recover`** is the punish window (`recover_s`). A titan who spends it turning to
        //   face you has no punish window.
        // - **`Idle`** has no target to turn to, and **`Death`** never reaches this line.
        // **The ambusher tracks you with his eyes and nothing else** (`F-061`). He has no
        // `Pursue` at all, so without this arm the only ticks a lurker could turn in are the
        // 24 of his own `windup_s` — 18° at 45 °/s — and a body he never came about to face
        // would swing his 60° cone at empty street. He turns on the spot inside
        // `aggro_radius_m` (25 m) and takes no step: `gait.speed_m_s` stays 0 through all of
        // it, which is what `tests/mission.rs::f061_…` measures.
        // **`F-051`**: what used to stand here was `distance_m <= aggro_radius_m`, so a lurker
        // turned to watch anything inside a circle, including a man standing dead behind him
        // in silence. He watches what he has actually NOTICED now — which for an ambusher is
        // usually the ear, and that is the right instrument for the job: `hearing_radius_m` 75
        // against `aggro_radius_m` 25 is the whole of what makes him a lurker rather than a
        // slow husk.
        let crouched_and_watching =
            tuning.ambush && *state == TitanState::Idle && awareness.detected;
        let turning = seen
            && (matches!(*state, TitanState::Pursue | TitanState::Windup) || crouched_and_watching);
        let pursuing =
            *state == TitanState::Pursue && seen && target.distance_m > tuning.attack_range_m;
        // **The leap** (`F-058`). The one place in this file where a titan moves outside
        // `Pursue`, and it is deliberate: the scuttler's blow carries his body forward through
        // his own `Strike`, so a player who sidestepped at 2.5 m is still inside it when it
        // lands. He does NOT turn while he does it — `Strike` is committed, see above — which
        // is what leaves the answer open: go up, not sideways.
        let lunging = *state == TitanState::Strike && tuning.lunge_m_s > 0.0;

        if lunging {
            gait.speed_m_s = tuning.lunge_m_s;
            let step = *transform.forward() * tuning.lunge_m_s;
            transform.translation += step * dt;
            linear.0 = step;
            velocity.0 = step;
            continue;
        }

        // **The roll** (`F-059`). The second place in this file where a titan moves outside
        // `Pursue`, and the mirror of the leap above: the body carries itself **backwards**,
        // still facing the player, and does not turn while it does. That it keeps its facing is
        // what makes the roll a retreat and not a flight — he comes straight back at you.
        //
        // He rolls through his own startup as well, and that is deliberate: a crouch that
        // stands still and then teleports into a slide is a tell the player learns to ignore.
        if *state == TitanState::Roll && tuning.roll_speed_m_s > 0.0 {
            gait.speed_m_s = tuning.roll_speed_m_s;
            let step = *transform.back() * tuning.roll_speed_m_s;
            transform.translation += step * dt;
            linear.0 = step;
            velocity.0 = step;
            continue;
        }

        if !pursuing {
            // Planted. A titan that keeps sliding through its own wind-up is the "FSM as
            // decoration" failure, in one line. He may still turn on the spot — that is a
            // facing, not a gait, and `gait.speed_m_s` stays 0 through all of it.
            gait.speed_m_s = 0.0;
            linear.0 = Vec3::ZERO;
            velocity.0 = Vec3::ZERO;
        }

        if !turning {
            continue;
        }

        // ---- turn -------------------------------------------------------------------
        let to = aim(id, *state, target, tuning, slot, crowd, transform.translation, tick.0);
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

        if !pursuing {
            // He turned on the spot and that is all he is allowed this tick.
            continue;
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
        TitanTiming {
            windup_ticks: 36,
            strike_ticks: 12,
            recover_ticks: 24,
            cooldown_ticks: 90,
            death_ticks: 60,
            roll_ticks: 0,
            roll_startup_ticks: 0,
        }
    }

    /// The husk's numbers: no swerve, no leap, no flank, no call, no ambush.
    fn tuning() -> TitanTuning {
        TitanTuning {
            speed_m_s: 3.0,
            accel_m_s2: 3.0,
            turn_rad_per_s: 50f32.to_radians(),
            attack_range_m: 6.0,
            swerve_rad: 0.0,
            swerve_period_ticks: 0,
            lunge_m_s: 0.0,
            flank_offset_m: 0.0,
            call_radius_m: 0.0,
            call_hold_ticks: 0,
            ambush: false,
            roll_speed_m_s: 0.0,
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
            decide(TitanState::Pursue, &clock, 0, true, &timing(), &tuning(), &at(1.0)),
            TitanState::Windup
        );
    }

    #[test]
    fn a_cooldown_holds_the_titan_in_pursue_even_inside_reach() {
        let clock = StateClock::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, 7, true, &timing(), &tuning(), &at(1.0)),
            TitanState::Pursue
        );
    }

    #[test]
    fn losing_the_target_falls_back_to_idle() {
        let clock = StateClock::default();
        let nobody = TitanTarget::default();
        assert_eq!(
            decide(TitanState::Pursue, &clock, 0, false, &timing(), &tuning(), &nobody),
            TitanState::Idle
        );
        // The other half of the same rule, and since `F-051` it means something sharper than
        // it used to: not "he is outside a 45 m circle" but **"he has not noticed him"**. The
        // distance in the target is now irrelevant to this edge — see the test below.
        assert_eq!(
            decide(TitanState::Idle, &clock, 0, false, &timing(), &tuning(), &at(99.0)),
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
                decide(state, &clock, 0, true, &t, &u, &at(1.0)),
                wanted,
                "{state:?} @ {ticks_in_state}"
            );
        }
    }

    #[test]
    fn death_is_a_one_way_street() {
        let clock = StateClock { ticks_in_state: 9999, state_ticks: 60 };
        assert_eq!(
            decide(TitanState::Death, &clock, 0, true, &timing(), &tuning(), &at(1.0)),
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
                decide(state, &last_inside, 0, true, &t, &u, &at(1.0)),
                state,
                "{state:?} ended one tick before its own `state_ticks`"
            );
            assert_ne!(
                decide(state, &first_after, 0, true, &t, &u, &at(1.0)),
                state,
                "{state:?} ran past the `state_ticks` the overlay prints under it"
            );
        }
    }

    /// ★ **`F-051` at the state machine, in two lines.**
    ///
    /// The same titan, the same target, the same one metre of distance — and the only thing
    /// that decides whether he comes is whether he has noticed. Until 2026-08-25 this function
    /// read the distance itself (`distance_m <= aggro_radius_m`) and a titan with his back
    /// turned came anyway; delete the `aware` gate from [`decide`] and the first assert below
    /// goes red.
    #[test]
    fn f051_a_titan_that_has_not_noticed_you_does_not_come_however_close_you_are() {
        let clock = StateClock::default();
        assert_eq!(
            decide(TitanState::Idle, &clock, 0, false, &timing(), &tuning(), &at(1.0)),
            TitanState::Idle,
            "one metre away and unnoticed"
        );
        assert_eq!(
            decide(TitanState::Idle, &clock, 0, true, &timing(), &tuning(), &at(1.0)),
            TitanState::Windup,
            "one metre away and noticed"
        );
        // And the far end: noticed at 99 m is a walk, not a stand.
        assert_eq!(
            decide(TitanState::Idle, &clock, 0, true, &timing(), &tuning(), &at(99.0)),
            TitanState::Pursue
        );
    }
}
