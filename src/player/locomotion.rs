//! Ground locomotion — **the one system that writes the player's horizontal velocity.**
//!
//! ## Below the run speed: a target. Above it: a floor. (`F-014`)
//!
//! Until 2026-08-10 this system **assigned** `velocity.x/z` on every grounded tick, and that
//! assignment was the single largest reason the user's verdict after his first session was
//! *"seile ohne boost bringen gar nichts"*. Measured `[cachy]`, 27 headless runs: released at
//! the bottom of a pendulum arc the player touches down at **39.717 m/s**, and two ticks later
//! he is at **0.000 m/s** with no key held and at **6.000 m/s** — `run_speed_m_s` — with W
//! held. A swing that covers 48.02 m in 2.83 s, 2.83× running speed, is thrown away by the
//! first blade of grass it touches. There is nothing left for `F-014` Momentum-Chaining to
//! chain.
//!
//! So the rule is split at `run_speed_m_s`:
//!
//! - **At or below it the ground is exactly what it always was:** an assignment. Starting to
//!   walk is instant, stopping is instant, turning is instant. Ground combat is supposed to
//!   feel crisp, and every existing promise about walking (`tests/player.rs`,
//!   `scripts/p3-mouse.txt`) is a promise about this branch. It is untouched.
//! - **Above it the run speed is a floor, not a target.** A player who arrives at 30 m/s
//!   keeps 30 m/s and bleeds down toward the floor; a player who arrives at 30 m/s and holds
//!   W does not get pulled back to 6.
//!
//! That is one rule and it needs exactly one rate: how fast the ground eats speed.
//!
//! ## Where the deceleration rate comes from — and where it will come from
//!
//! `decel = -gravity_m_s2`. **That is not an invented number, it is an identity:** dry
//! Coulomb friction brakes at `μ·g`, and this is that expression at `μ = 1.0`. The same move
//! `player::integrator::readback` makes when it takes its ground-contact threshold from
//! `world.collision_margin_m` instead of inventing a second one. At the file's `-20 m/s²` it
//! means: 30 m/s carries **1.20 s and 21.6 m** before the floor, a little under the 35 m
//! block pitch of the city (`maps.ron`: `lot_m 28` + `street_m 7`) — one block of chain per
//! swing.
//!
//! **The day somebody wants to tune that independently of gravity, the knob is a new RON key
//! `game.ron: player.ground_decel_m_s2`, and this line reads it instead.** It is not one
//! today because `μ = 1.0` is a statement anybody can check, and a second untuned number that
//! nobody has measured is worth less than a derivation that says out loud what it assumes.
//!
//! ## Why a direct velocity write and not a force
//!
//! Everything else on this player is an acceleration: gravity comes from avian, the gas boost
//! from [`BoostAccel`](crate::shared::BoostAccel) through `Forces`
//! (`apply_linear_acceleration`, "ignoring mass"), so mass never becomes a tuning number.
//! Running on the ground is the exception, and deliberately so: a run speed is a **target**,
//! not a push. `run_speed_m_s = 6.0` has to mean 6 m/s on the first tick and after ten
//! seconds, uphill and against a wall — an acceleration plus a drag term would turn one honest
//! number in `game.ron` into two dishonest ones, and the value you actually reach would depend
//! on the surface. So: assignment, on exactly one component, in exactly one place. What
//! `F-014` adds above the run speed does not change that — it only stops the assignment from
//! reaching **down** into speed the player earned somewhere else.
//!
//! **That makes this system a writer of `LinearVelocity`** — avian is the other one, and the
//! two never run at the same time: this one is ordered `.before(PhysicsSystems::First)`, so
//! the solver reads what was written here and nobody reads a half-written velocity. The row
//! is in the authority table of `docs/architecture.md`.
//!
//! ## Only on the ground — and "on the ground" stopped being the same as "touching it"
//!
//! Until 2026-08-10 this header claimed *"on the rope the body belongs to `vector`"*, and that
//! sentence was **false for a player whose feet were still on the floor**:
//! [`MovementState::Tethered`] was declared in `src/shared/state.rs` and written by nobody, so
//! a roped player read `Grounded` and this system wrote his horizontal velocity while the rope
//! was carrying him. It cost `scripts/f-001-hooks.txt` its whole headline number on exactly
//! **one** tick — the arithmetic and the measurement are in
//! [`super::integrator::movement_state`], which is where the sentence is now made true.
//!
//! Nothing in this file changed for it. `*state != MovementState::Grounded` was already the
//! right line; what changed is that `Grounded` now means what it says.
//!
//! Reads [`MovementState`] from the **end of the previous tick** —
//! [`super::integrator::readback`] derives it out of the real contacts after the physics step.
//! One tick of lag is deterministic and cheaper than an order dependency inside the step.
//!
//! ## `F-006` Swerve — and the sentence that used to stand here
//!
//! Until 2026-08-12 this header said *"in the air this system writes nothing at all — no air
//! control"*, and that was the whole of the user's complaint after he played it:
//!
//! > *„wenn man w drückt und verbunden ist bekommt man schon movement! bei a und d movement zur
//! > seite. mit s »spannt« man nur das seil! … das a d sorgt dafür dass man nicht immer direkt
//! > zum seil gezogen wird!"* (2026-08-12, `docs/NEXT.md` §1a)
//!
//! That is `F-006` out of `docs/backlog/gameplay.ron`, word for word: *"Richtungseingabe
//! waehrend des Einzugs moduliert die Flugbahn seitlich, nach oben und unten. **Kein binaeres
//! Ziel-Anfliegen**."* It arrives as [`air_control`], a **contribution** in
//! `SimulationSystems::Drive` — that is what [`RunAccel`](crate::shared::RunAccel) has been
//! declared for since day one — and **not** as a second assignment in this system, or the two
//! would fight over one field.
//!
//! ## Flight mode is not a state, it is the same threshold read the other way round
//!
//! > *„nur weil man den boden berührt ist man nicht direkt aus flugmodus raus, erst wenn man
//! > langsam genug ist läuft man wieder"* (2026-08-12, `docs/NEXT.md` §1b)
//!
//! **A speed decides it, not a contact** — and the speed is one this file has had since
//! `F-014`: [`super::integrator::ground_top_speed_m_s`], `run_speed_m_s` plus one tick of the
//! ground's own step, 6.3333 m/s at the file's numbers. That is the same number
//! `movement_state` already splits `Grounded` from `Tethered` on (`FIND-037`), and it is the
//! same sentence: **the legs cannot produce more than the ground's top speed.** Below it your
//! legs drive you; above it — or with your feet off the floor — they cannot reach, and the same
//! WASD becomes thrust.
//!
//! So no new [`MovementState`] variant was added and none was widened. `Tethered` was already
//! flight mode for the roped half of this; what was missing is that flight mode had no
//! controls. Two things follow, and both are deliberate:
//!
//! - **Above the threshold this system stops steering.** `desired` becomes `Vec2::ZERO`, so
//!   `ground_step` degenerates to what it always was at that end — a pure brake along the
//!   direction of travel. The steering that `F-014` used to do with the legs at 22.44° per half
//!   second is now [`air_control`]'s, at 11.5° with a tank and 5.8° without one, and that pair
//!   is guarded by `f014_the_input_still_steers_the_carried_momentum` and
//!   `f006_above_the_threshold_the_legs_stop_steering_and_the_air_takes_over`.
//! - **The ground keeps braking a skidding player**, because he is still `Grounded` and this
//!   system still runs for him. Without that brake there would be no way back below the
//!   threshold at all, and *„erst wenn man langsam genug ist läuft man wieder"* would never
//!   come true — you would slide at 30 m/s forever on `player.friction: 0.0`.
//!
//! **The jump stays.** Nobody asked to lose it, and a jump is vertical: it produces no
//! horizontal speed the threshold could be about.

use avian3d::prelude::{Forces, LinearVelocity, WriteRigidBodyForces};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Buttons, Gas, Intent, MovementState, RunAccel, Velocity};

/// One tick of horizontal ground movement, as a function of nothing but its arguments.
///
/// `current` and `desired` are **XZ velocities in world space**, in m/s; `desired` is the
/// input direction already multiplied by `run_speed_m_s`, so a half-pressed stick is half the
/// run speed exactly as before. The return value is the new XZ velocity.
///
/// Three properties, and every one of them has a test in `tests/player.rs`:
///
/// 1. `|current| <= run_speed_m_s` ⇒ the result **is** `desired`. Bit for bit the behaviour
///    of every version before `F-014`.
/// 2. Above it the magnitude only ever falls, by `decel_m_s2 * dt_s` per tick, and never
///    below `|desired|` — that is the floor. **It never rises**, so a landing cannot conjure
///    speed out of nothing.
/// 3. The input still steers: the desired direction is added at the same rate `decel_m_s2`
///    and the result is renormalised, so the velocity turns at `decel/|v|` rad/s — 38 °/s at
///    30 m/s, 114 °/s at 10 m/s. Fast is hard to turn, which is the trade a chain is made of.
pub fn ground_step(
    current: Vec2,
    desired: Vec2,
    run_speed_m_s: f32,
    decel_m_s2: f32,
    dt_s: f32,
) -> Vec2 {
    let speed = current.length();

    // Walking. Unchanged since the first version of this file, and deliberately so: below the
    // run speed the ground is crisp, not skatey — start, stop and turn are all instant.
    if speed <= run_speed_m_s {
        return desired;
    }

    // Carried momentum. The run speed has stopped being a target and become a floor.
    let step = decel_m_s2 * dt_s;
    // `.max(desired.length())` and not `.max(run_speed_m_s)`: a half-pressed stick asks for
    // less, and asking for nothing at all (no key) asks for zero — which is what brings a
    // slide to a real stop instead of leaving the player gliding at walking pace forever.
    let magnitude = (speed - step).max(desired.length()).max(0.0);
    // `normalize_or_zero` cannot bite here: `|current| > run_speed_m_s >= step` in every case
    // that reaches this line, so the sum can never be the zero vector.
    let direction = (current + desired.normalize_or_zero() * step).normalize_or_zero();
    direction * magnitude
}

/// Running and jumping — the only direct writer of a player's [`LinearVelocity`].
///
/// Runs in `SimulationSystems::Integrate`, before **every** avian system.
pub fn ground_locomotion(
    data: Res<GameData>,
    mut players: Query<(&Intent, &MovementState, &mut LinearVelocity)>,
) {
    let s = &data.game.player;
    // The tick length out of the file, not out of `Time::delta_secs()`: the simulation runs at
    // `simulation_hz` and a movement that accumulates wall-clock deltas desyncs (§6 rule 4).
    let dt_s = 1.0 / data.game.simulation_hz as f32;
    // See the module header: `μ·g` at `μ = 1.0`, not a number somebody typed in.
    let decel_m_s2 = -data.game.gravity_m_s2;
    // Above this the legs cannot reach the velocity any more and the input is [`air_control`]'s
    // — the module header, and `super::integrator::ground_top_speed_m_s` for the number.
    let top_m_s = super::integrator::ground_top_speed_m_s(&data);

    for (intent, state, mut velocity) in &mut players {
        // On the rope the body belongs to `vector`; downed and on the wall are states of
        // their own. This system speaks only for a player standing on the ground.
        if *state != MovementState::Grounded {
            continue;
        }

        // **Flight mode, and it is one comparison** (`docs/NEXT.md` §1b): above the ground's
        // top speed the legs have nothing to push against that they could push harder than,
        // so the input is not running any more — it is thrust, and [`air_control`] takes it.
        // What is left here is the brake, which is what eventually brings him back below the
        // line and puts him back on his legs.
        let desired = if velocity.0.xz().length() > top_m_s {
            Vec2::ZERO
        } else {
            // Movement is player-local: rotate into world coordinates first, then apply.
            let (sin, cos) = intent.yaw.sin_cos();
            let forward = Vec3::new(-sin, 0.0, -cos);
            let right = Vec3::new(cos, 0.0, -sin);
            ((forward * intent.move_y + right * intent.move_x).clamp_length_max(1.0)
                * s.run_speed_m_s)
                .xz()
        };

        let next = ground_step(velocity.0.xz(), desired, s.run_speed_m_s, decel_m_s2, dt_s);
        velocity.x = next.x;
        velocity.z = next.y;

        // **`velocity.y <= 0.0` is not decoration — it is worth 10 % of jump height.**
        // Contact data is one tick old (the narrow phase runs before the solver), so a player
        // who has already taken off still counts as `Grounded` for one more tick. Without
        // this guard, holding the button set the jump speed a second time on that tick and
        // the apex came out at 1.1588 m instead of 1.0562 m — measured on 2026-08-09. You
        // push OFF the ground; you cannot push off it while already leaving it.
        // Measured against a false positive: at rest `velocity.y` is exactly 0.0 — over 300
        // ticks it was never once greater.
        if intent.pressed(Buttons::JUMP) && velocity.y <= 0.0 {
            // Speed, not an impulse: `jump_speed_m_s` is a height you can compute in your
            // head (v²/2g), and it stays that number no matter what the capsule weighs.
            velocity.y = s.jump_speed_m_s;
        }
    }
}

/// **Is this body flying?** — as a function of nothing but its arguments.
///
/// One comparison and two states that are never flying. `ground_speed_m_s` is `|v.xz|`, the
/// same horizontal speed [`super::integrator::movement_state`] judges on, and
/// `ground_top_speed_m_s` is [`super::integrator::ground_top_speed_m_s`].
///
/// `Grounded` is the only state that has to ask, and that **is** the user's sentence: touching
/// the ground does not end flight, being slow does. `Tethered` and `Airborne` are flight by
/// definition — the rope or nothing at all is carrying him, and in both cases his legs are not.
///
/// The two that are never flight are not an oversight. `Downed` is *"out of the fight instead
/// of dead"* (`shared::MovementState`), and a body that may not walk may certainly not fly;
/// `OnWall` belongs to `F-006`'s neighbour `F-013` and its input means something else there.
/// Both are written by other domains, so this function stays exhaustive on purpose — a new
/// variant has to be decided here and cannot fall through into "flying".
pub fn in_flight(state: MovementState, ground_speed_m_s: f32, ground_top_speed_m_s: f32) -> bool {
    match state {
        MovementState::Grounded => ground_speed_m_s > ground_top_speed_m_s,
        MovementState::Airborne | MovementState::Tethered => true,
        MovementState::OnWall | MovementState::Downed => false,
    }
}

/// Where WASD pushes in the air, in m/s² — **the whole of `F-006`'s direction rule**, testable
/// without an `App`.
///
/// Three keys and three different answers, and each one is the user's:
///
/// - **`W` flies where you look.** `look_dir` is the full 3-D look vector, so the pitch is what
///   gains and loses height — that is `F-006`'s *"seitlich, nach oben und unten"* without a
///   second pair of keys for it.
/// - **`A`/`D` are lateral and stay HORIZONTAL**, off the yaw and never off the pitch.
///   *„das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird"* — a strafe that
///   tilted with the pitch would drive you into the street in exactly the situation it is for,
///   because in a fast swing you are looking down.
/// - **`S` is not a thrust at all.** *„mit s »spannt« man nur das seil!"* — the rope tension is
///   `vector`'s to build, and until it is, `S` costs the body nothing. Hence `.max(0.0)` and
///   not a signed forward axis.
///
/// `clamp_length_max(1.0)` and not a normalize: it is the same rule `ground_locomotion` applies
/// to the same two axes, so `W`+`D` is one thrust at 45° and not 1.41 of them, while a
/// half-pressed axis stays half.
pub fn air_thrust(look_dir: Vec3, yaw: f32, move_x: f32, move_y: f32, accel_m_s2: f32) -> Vec3 {
    let (sin, cos) = yaw.sin_cos();
    let right = Vec3::new(cos, 0.0, -sin);
    (look_dir * move_y.max(0.0) + right * move_x).clamp_length_max(1.0) * accel_m_s2
}

/// `F-006` Swerve — WASD in the air, as an acceleration.
///
/// **Sole writer of [`RunAccel`].** Contributor — never sole writer — of
/// `VelocityIntegrationData::linear_increment`, which belongs to avian. The whole arrangement
/// is `vector::boost::gas_boost`'s, deliberately: an acceleration and not a force, so mass
/// never becomes a tuning number; `Option<Forces>` so a player in his first tick, before
/// avian's prepare step, does not silently keep last tick's drive; `set_if_neq` so a body that
/// is not thrusting does not invalidate every `Changed<RunAccel>` filter behind it.
///
/// ## Where the two numbers come from — `game.ron`, since 2026-08-12
///
/// Both were **expressions in this function** until FIND-051 was read back against rule 2:
/// `-gravity_m_s2 / 2` for the thrust and a literal `0.5` for the empty tank. A derivation is
/// still a game value, and a game value belongs in the file — so they are
/// [`air_accel_m_s2`](crate::data::PlayerTuning::air_accel_m_s2) and
/// [`air_accel_empty_fraction`](crate::data::PlayerTuning::air_accel_empty_fraction) now, with
/// no `serde(default)`: a `game.ron` without them crashes on load.
///
/// **What moving them must not lose is the reasoning, so it stands in the file too.** `10.0` is
/// half of `-gravity_m_s2` on purpose — **WASD alone can never hold you up**, so whatever keeps
/// a player in the air stays the rope or the gas, which is the acceptance criterion the user
/// wrote for this whole block (`docs/NEXT.md` §1f: *„es soll möglich sein wenn man gut ist die
/// ganze zeit in der luft zu bleiben bis das gas ausgeht"*). The bounds that follow are now
/// checked instead of assumed: strictly **below `-gravity_m_s2`** (`tests/data.rs`), and well
/// below `vector.boost_m_s2 = 34` — 29 % of it today, so `Shift` stays the strong option.
///
/// **The fraction is not a derivation at all, it is the spec:** *„ohne gas kann man immernoch w
/// a d nutzen um etwas movement aufzubauen (aber hälfte ca)"* (`docs/NEXT.md` §1e). So the air
/// control is **not gated** on gas — it only gets weaker, and an empty tank stops being the dead
/// end behind *„seile ohne boost bringen gar nichts"*.
///
/// ⚠️ **The two are one decision with `f014_the_input_still_steers_the_carried_momentum`**,
/// which asserts more than 10° of turn and measures 11.22°: an air control below ≈ 0.53·g takes
/// that test red. Both keys are ⚠️ UNTUNED.
///
/// ## What is read and what is not
///
/// [`Gas`] is **read**, never written — `vector::gas::gas_budget` is its one writer, and this
/// system books nothing, because it costs nothing. That is the difference to
/// `vector::boost::gas_boost`, which may not even look at the button without the grant: there,
/// thrust and debit have to be one decision. Here there is no debit to get out of step with.
///
/// [`Velocity`] and [`MovementState`] both come from the **end of the previous tick**
/// (`super::integrator::readback`), so the state and the speed this decides on are one
/// consistent snapshot instead of two ticks mixed. It also keeps `LinearVelocity` out of the
/// query, which [`Forces`] declares `Write` access to.
pub fn air_control(
    data: Res<GameData>,
    mut players: Query<(&Intent, &MovementState, &Velocity, &Gas, &mut RunAccel, Option<Forces>)>,
) {
    let s = &data.game.player;
    let top_m_s = super::integrator::ground_top_speed_m_s(&data);
    // Out of the file, not out of gravity — see the header. `tests/data.rs` guards the bound
    // the old derivation used to guarantee on its own.
    let full_m_s2 = s.air_accel_m_s2;

    for (intent, state, velocity, gas, mut drive, forces) in &mut players {
        let wanted = if in_flight(*state, velocity.0.xz().length(), top_m_s) {
            let accel_m_s2 =
                if gas.is_empty() { full_m_s2 * s.air_accel_empty_fraction } else { full_m_s2 };
            air_thrust(intent.look_dir(), intent.yaw, intent.move_x, intent.move_y, accel_m_s2)
        } else {
            Vec3::ZERO
        };

        drive.set_if_neq(RunAccel(wanted));

        if let Some(mut forces) = forces {
            // avian skips a zero vector itself (`query_data.rs:483`), so the `ZERO` case costs
            // nothing and needs no branch of its own here.
            forces.apply_linear_acceleration(wanted);
        }
    }
}
