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
//! In the air this system writes nothing at all — no air control, exactly as before the
//! physics arrived. Whoever wants air control adds it as a **contribution** in
//! `SimulationSystems::Drive` (that is what [`RunAccel`](crate::shared::RunAccel) is there
//! for) and not as a second assignment here, or the two fight over the same field.
//!
//! Reads [`MovementState`] from the **end of the previous tick** —
//! [`super::integrator::readback`] derives it out of the real contacts after the physics step.
//! One tick of lag is deterministic and cheaper than an order dependency inside the step.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{Buttons, Intent, MovementState};

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

    for (intent, state, mut velocity) in &mut players {
        // On the rope the body belongs to `vector`; downed and on the wall are states of
        // their own. This system speaks only for a player standing on the ground.
        if *state != MovementState::Grounded {
            continue;
        }

        // Movement is player-local: rotate into world coordinates first, then apply.
        let (sin, cos) = intent.yaw.sin_cos();
        let forward = Vec3::new(-sin, 0.0, -cos);
        let right = Vec3::new(cos, 0.0, -sin);
        let desired = (forward * intent.move_y + right * intent.move_x).clamp_length_max(1.0)
            * s.run_speed_m_s;

        let next = ground_step(
            velocity.0.xz(),
            desired.xz(),
            s.run_speed_m_s,
            decel_m_s2,
            dt_s,
        );
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
