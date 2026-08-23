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
//!
//! ## `F-006` again, 2026-08-13: the rope is now half of the air control
//!
//! > *„wenn ich mit seilen festhake (was instant sein soll) und w in die richtung drücke will ich
//! > dass man deutlich mehr geboosted wird. also dass man dort richtig hingezogen wird. wenn man
//! > aber a oder d drückt wird nach links/rechts geboostet! wenn man zur seite schaut soll die
//! > steuerung mitdrehen. also wenn ich 45 grad nach links und w drücke dann etwas eingezogen aber
//! > auch boost zur seite."* (2026-08-12, `docs/NEXT.md` §1A)
//!
//! What [`air_thrust`] does is unchanged and stays the whole answer for a player with no rope.
//! What an **anchored** rope adds is [`rope_steer`], and the two are **added**, not blended:
//!
//! ```text
//! a = clamp_len₁(l̂·w⁺ + ê_right·mx)·air_accel_m_s2·(empty ? fraction : 1)      // the look term
//!   + (1/n)·Σᵢ r̂ᵢ·air_pull_m_s2·w⁺·max(0, l̂·r̂ᵢ)·fadeᵢ + ê_right·air_lateral_m_s2·mx
//! ```
//!
//! **The pull sits outside the `clamp_len₁`**, and that is the point of the whole plan: the clamp
//! is what makes `W`+`D` one thrust instead of 1.41 of them, and putting the rope pull inside it
//! would cap *„deutlich mehr geboosted"* at the free-air number it is meant to be three times.
//! At 0° the two agree in direction and the player closes at 10 + 30 = **40 m/s²**; at 90° the
//! projection is zero and there is no haul left at all, only the swing — which is the user's
//! *„das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird"* made arithmetic.
//!
//! It is not free: the rope half is gated on [`GasGrant::steer`], `vector.gas_steer_per_s: 16.0`,
//! priced at the boost's own gas-per-m/s (16/30 against 18/34).

use avian3d::prelude::{Forces, LinearVelocity, WriteRigidBodyForces};
use bevy::prelude::*;

use crate::data::{GameData, RopeForceModel};
use crate::shared::{Buttons, Gas, GasGrant, Hook, Intent, MovementState, RunAccel, Velocity};

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

/// The three numbers the rope half of the mixing rule needs, plus the rope floor it fades
/// against — **a struct and not four `f32` in a row**, the same argument
/// `vector::gas::Costs` makes: swap two of them and the compiler says nothing while the
/// strafe silently becomes the haul.
///
/// All four come out of RON: three from `game.ron: player`, `min_rope_m` from
/// `game.ron: vector` — deliberately the *same* key `vector::rope` enforces, not a second copy
/// of it, because the whole point of the fade is to end before that constraint begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteerTuning {
    /// [`crate::data::PlayerTuning::air_pull_m_s2`] — what `W` adds **along the rope**.
    pub pull_m_s2: f32,
    /// [`crate::data::PlayerTuning::air_lateral_m_s2`] — what `A`/`D` add across it.
    pub lateral_m_s2: f32,
    /// [`crate::data::PlayerTuning::air_pull_fade_m`] — the band above `min_rope_m` over which
    /// the pull comes up from nothing to full.
    pub fade_m: f32,
    /// [`crate::data::VectorTuning::min_rope_m`] — where the pull is **exactly zero**.
    pub min_rope_m: f32,
    /// **How much weight the aligned pull takes off the player, in m/s² of upward
    /// acceleration** — `-gravity_m_s2 * player.air_pull_lift_fraction`, already multiplied
    /// out by the caller so that this struct stays in one unit.
    ///
    /// Rides the same `cᵢ` and the same `fᵢ` as [`pull_m_s2`](Self::pull_m_s2), so at 90° off
    /// the rope it is exactly zero and the swing is the swing it always was. `0.0` is the
    /// behaviour before 2026-08-20, bit for bit.
    pub lift_m_s2: f32,
}

/// What an **anchored** rope adds to the air control, in m/s² — the rope half of the mixing
/// rule (`docs/NEXT.md` §1B), and the whole of the user's *„wenn ich mit seilen festhake und w
/// in die richtung drücke will ich dass man deutlich mehr geboosted wird"*.
///
/// `to_anchors_m` is one `tipᵢ − hand` per **anchored** arm, unnormalised, in world space; the
/// length and the direction both come out of it, so a caller cannot pass a direction that
/// disagrees with its own distance. An empty slice is the whole of the "no rope" case and
/// returns [`Vec3::ZERO`].
///
/// ```text
/// r̂ᵢ = unit(tipᵢ − h)   Lᵢ = |tipᵢ − h|   w⁺ = max(0, move_y)   ê_right = (cos yaw, 0, −sin yaw)
/// cᵢ = max(0, l̂ · r̂ᵢ)                     fᵢ = clamp((Lᵢ − min_rope_m) / fade_m, 0, 1)
/// rope = (1/n)·Σᵢ (r̂ᵢ·pull_m_s2 + ŷ·lift_m_s2)·w⁺·cᵢ·fᵢ  +  ê_right·lateral_m_s2·mx
/// ```
///
/// ## Three details that are the whole design, and each one is a trap already paid for once
///
/// 1. **`cᵢ` is a cosine projection, not an `nlerp` between look and rope.** `nlerp` is
///    `FIND-046`'s bug: at 170° of separation it moves the result 3 % of the way and sends the
///    player ~90° off his look direction — a dead band exactly where the player is most sure
///    what he asked for. `max(0, dot)` has no such band: it falls smoothly to zero at 90° and
///    stays there behind it, so looking away from the anchor *never* hauls you at it. That is
///    the answer to *„aktuell wenn ich seil spanne und s drücke werde ich stark zum seil
///    gezogen! das soll nicht sein!"* from the other side — `W` while looking away is the same
///    complaint with a different key.
/// 2. **Per-arm `r̂ᵢ`, never a mean direction.** Two ropes 180° apart average to the **zero
///    vector**, so a mean would make the strongest configuration in the game — hanging between
///    two anchors — the one where `W` does nothing. Each arm projects on its own and the *forces*
///    are what get averaged.
/// 3. **`A`/`D` ride `ê_right`, the horizontal look-right, never the rope tangent.** A tangent
///    **flips sign** the moment the anchor passes beside you, which inverts the strafe in the
///    middle of a swing — the one place a player is committed and cannot correct.
///
/// 4. **`lift_m_s2` is the same `cᵢ` again, on `ŷ`** — and it is the whole of the user's
///    *„wenn man da hin schaut dass nicht alle physics also gravitiy so stark sind. dass man
///    gerader hingezogen wird … aber wenn man nicht hinschaut man auch gut kreise schwingen
///    kann"* (2026-08-20). Measured before it existed (`docs/FINDINGS.md` FIND-131): looking
///    straight down a **horizontal** rope with `W` held, the thrust is 40 m/s² along the rope
///    and gravity is −20 across it, so the player is hauled **26.57° below the line he is
///    aiming at**. The pull was never weak; gravity was eating the whole of the straightness.
///    Because it rides `cᵢ`, the term dies with the alignment: at 90° off the rope it is
///    identically zero and the arc is untouched, which is the second half of his sentence.
///    ⚠️ It is a WEIGHT relief and not a thrust — nothing here reads `gravity_m_s2`; the
///    caller multiplies it out (`air_control`), so this function keeps one unit and one job.
///
/// **The `1/n` covers the pull only.** The lateral term is the player's own thrust, not the
/// rope's, so it does not get halved for owning two ropes; the pull is one budget shared out,
/// so two ropes at 60° apart pull as hard as one, just in a direction between them.
///
/// **The fade ends where `FIND-035`'s cliff begins.** At `min_rope_m` the length constraint
/// takes 17 m/s out of the player in a single tick, and thrusting straight at an anchor you are
/// 3 m from feeds exactly that. So `fᵢ` is 0 at `min_rope_m` and 1 at `min_rope_m + fade_m` —
/// 0 at 3 m and full at 15 m with the numbers of 2026-08-13.
pub fn rope_steer(
    to_anchors_m: &[Vec3],
    look_dir: Vec3,
    yaw: f32,
    move_x: f32,
    move_y: f32,
    t: SteerTuning,
) -> Vec3 {
    if to_anchors_m.is_empty() {
        return Vec3::ZERO;
    }
    // `S` is not a thrust and it is not a haul either — the same `.max(0.0)` as [`air_thrust`],
    // and for the user's same sentence about what `S` means (*„mit s »spannt« man nur das
    // seil"*). Requirement 7 of `docs/NEXT.md` §1A is that `S` must **never** pull you at the
    // rope, and this is where that would otherwise have crept back in.
    let forward = move_y.max(0.0);

    let mut pull = Vec3::ZERO;
    for to_anchor in to_anchors_m {
        let length_m = to_anchor.length();
        // A tip that sits exactly on the hand has no direction. It cannot happen for an
        // anchored arm at `min_rope_m: 3.0`, but `normalize()` would answer `NaN` and NaN in a
        // velocity is unrecoverable — so it is skipped, and skipped **without** shrinking `n`:
        // the budget stays shared between the arms the player actually holds.
        let Some(direction) = to_anchor.try_normalize() else {
            continue;
        };
        let projection = look_dir.dot(direction).max(0.0);
        let fade = ((length_m - t.min_rope_m) / t.fade_m).clamp(0.0, 1.0);
        let gate = forward * projection * fade;
        // `direction * pull` and `Y * lift` share the one gate, so there is no state of the
        // world in which the weight comes off without the haul being on.
        pull += (direction * t.pull_m_s2 + Vec3::Y * t.lift_m_s2) * gate;
    }

    let (sin, cos) = yaw.sin_cos();
    let right = Vec3::new(cos, 0.0, -sin);
    pull / to_anchors_m.len() as f32 + right * (t.lateral_m_s2 * move_x)
}

/// The three numbers [`rope_drive`] is made of — `game.ron: vector.drive_*`, and **all three
/// are the user's to judge** (`FIND-149`).
///
/// A struct and not three `f32` in a row, for [`SteerTuning`]'s reason: swap the speed and the
/// lateral and the compiler says nothing while the drive silently becomes a strafe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriveTuning {
    /// [`crate::data::VectorTuning::drive_speed_m_s`] — the speed `W` chases **along the rope**.
    pub speed_m_s: f32,
    /// [`crate::data::VectorTuning::drive_lateral_m_s`] — what `A`/`D` chase across it.
    pub lateral_m_s: f32,
    /// [`crate::data::VectorTuning::drive_ramp_s`] — the time constant of the ramp. `<= 0`
    /// means **no drive at all**, which is the safe direction for a number that divides.
    pub ramp_s: f32,
}

impl DriveTuning {
    /// The same drive at a fraction of its speed — *„ohne gas kann man immernoch w a d nutzen um
    /// etwas movement aufzubauen (aber hälfte ca)"* (`docs/NEXT.md` §1e).
    ///
    /// **The ramp is not scaled.** An empty tank makes the drive *weaker*, not *sluggish*: the
    /// time constant is the feel of the onset and the user described it separately from the
    /// strength.
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self { speed_m_s: self.speed_m_s * factor, lateral_m_s: self.lateral_m_s * factor, ..self }
    }
}

/// `FIND-149` — **the reference's rope: a velocity drive, in m/s², and `Vec3::ZERO` the moment
/// no key is held.**
///
/// The user, playing *Attack on Titan Revolution* beside this game on 2026-08-23:
///
/// > *„wenn ich mich hooke: dann werde ich direkt rangezogen wenn ich ran gehe. mit a und d kann
/// > man zur seite gehen. aber sonst wird man direkt hingezogen! **wenn ich nichts drucke dann
/// > wird auch nicht rangezogen!**"* … *„aber es ist ein etwas smoother übergang! aber recht
/// > schnell!"*
///
/// ```text
/// r̂ᵢ = unit(tipᵢ − h)   cᵢ = max(0, l̂ · r̂ᵢ)   w⁺ = clamp(move_y, 0, 1)   ê_right = (cos yaw, 0, −sin yaw)
/// v⁰ = v − ê_right·(v·ê_right) + ê_right·lateral·mx        (the flight, KEPT, steered sideways)
/// v* = clamp_len( lerp( v⁰ , unit(Σᵢ r̂ᵢ·cᵢ)·speed·w⁺·maxᵢ cᵢ + ê_right·lateral·mx , w⁺ ) , speed )
/// a  = (v* − v) / ramp
/// ```
///
/// ## The six things this function is, and why each one is that way
///
/// 1. **It is a chase toward a velocity, not a push.** `(v* − v)/τ` is an acceleration whose
///    magnitude falls to zero as the velocity arrives, so the speed is **capped by
///    construction** and the onset is an exponential with time constant `τ` — 63 % of the gap
///    closed in `τ`, 95 % in `3τ`. *„etwas smoother übergang, aber recht schnell"* is a time
///    constant and nothing else. [`rope_steer`], the pendulum's term, is the opposite: a
///    constant acceleration that builds speed for as long as it is held.
/// 2. **No target, no drive — and that is the load-bearing sentence.** `w⁺` is zero unless a
///    forward key is down, `mx` is zero unless a lateral one is, and if the whole target comes
///    out `Vec3::ZERO` the function returns `Vec3::ZERO` *before* the subtraction. Without that
///    early return a held-but-useless key would read as "chase 0 m/s" and **brake** a falling
///    player — an air brake nobody asked for, and the exact opposite of what the sentence says.
///    ⚠️ **Gravity is untouched either way.** He did not say you float; he said you are not
///    *pulled*. A hooked player who presses nothing falls exactly as if he had no hook.
/// 3. **`S` is not a haul.** The same `.max(0.0)` as [`air_thrust`] and [`rope_steer`], for the
///    same sentence (*„mit s »spannt« man nur das seil!"*) and the same requirement
///    (`docs/NEXT.md` §1A requirement 7).
/// 4. **`cᵢ`, the look gate, is [`rope_steer`]'s and is kept deliberately.** It is what makes
///    two anchors 180° apart usable — a mean *direction* would be the zero vector there — and it
///    is the same predicate `vector::gas::steer_has_effect` already bills on, so the drive costs
///    gas exactly when it moves the player. Looking away from an anchor never hauls you at it.
///
/// 5. **`maxᵢ cᵢ` for the strength, `unit(Σ r̂ᵢcᵢ)` for the direction — and NOT `1/n` for both.**
///    [`rope_steer`] divides by `n` because a *force* is one budget shared between the arms.
///    A target **velocity** is not a budget, and dividing it made the second hook a penalty:
///    measured on the pure function, an anchor straight ahead plus a second one 60° off it came
///    out at **0.661** of the single-rope target, i.e. hooking a second roof drove the player
///    **34 % slower**. Direction still blends — two ropes take you between them — but the speed
///    is the best-aligned arm's, and for one rope the whole expression is unchanged.
///    (`tests/player.rs::f153_a_second_rope_does_not_halve_the_drive`.)
/// 6. **`W` chases the whole velocity; `A`/`D` alone chase only their own axis.** The user,
///    2026-08-23, after playing this model: *"wenn ich mich hooke und w drücke … dann soll ich
///    erstmal ziemlich direkt daran gezogen werden. also ziemlich gerade. außer ich move nach
///    links (a oder rechts d). **es darf ‚strenger‘ sein. also nicht so physics accurate aber
///    mehr haptisch. also man macht was und man merkt es auch direkt!**"* — a design
///    instruction that outranks physical plausibility, so `W` eats the crossing momentum
///    instead of carrying it: that, and the ramp, is the whole of *"gerade"*.
///    ⚠️ **But the full chase is exactly what made `A`/`D` a brake** (`Q-050`): with `W`
///    released the target is `lateral` m/s sideways **and nothing else**, so the chase read the
///    flight's own 52.9 m/s as an error and killed it — measured 52.9 → 20.9 m/s in one second
///    in `scripts/f006-drive.txt`, **and nobody chose that.** So the released-`W` target keeps
///    the player's own velocity on every axis it does not command and replaces only `ê_right`;
///    `w⁺` lerps between the two targets, so a half-pressed stick gets half of each.
///    ⚠️ **And the cap is outside the lerp on purpose.** A lateral that merely *adds* to a
///    flight is the same mistake read the other way round: measured, `D` alone on a 70 m/s
///    flight arrived at **75.000 m/s**, which is `vector.max_speed_m_s` — the avian clamp, i.e.
///    the one number in this game nobody chose as a speed. Under the shared cap `A`/`D` is a
///    **redirect**: same speed, 23° of it pointing somewhere else.
///
/// **What is deliberately NOT here: the fade.** `air_pull_fade_m` exists because at
/// `min_rope_m` the *constraint* takes 17 m/s out of the player in one tick (`FIND-035`) — and
/// under [`crate::data::RopeForceModel::Drive`] there is no constraint to run into. The drive
/// is its own brake: arriving at the anchor, `r̂` swings past 90°, `cᵢ` goes to zero and the
/// target with it.
#[must_use]
pub fn rope_drive(
    to_anchors_m: &[Vec3],
    look_dir: Vec3,
    yaw: f32,
    move_x: f32,
    move_y: f32,
    velocity_m_s: Vec3,
    t: DriveTuning,
) -> Vec3 {
    if to_anchors_m.is_empty() || !(t.ramp_s > 0.0) {
        return Vec3::ZERO;
    }
    let forward = move_y.clamp(0.0, 1.0);

    // **Direction and strength are taken apart, and that is point 5.** `unit(Σ r̂ᵢ·cᵢ)` is the
    // direction — a blend between the arms, and bit for bit the old answer for one rope; `max cᵢ`
    // is the strength. The `/ n` that used to do both was [`rope_steer`]'s **force budget**
    // carried into a place that has none.
    let mut blend = Vec3::ZERO;
    let mut gate = 0.0;
    for to_anchor in to_anchors_m {
        // Same guard as `rope_steer`: a tip exactly on the hand has no direction, and a `NaN`
        // in a velocity is unrecoverable.
        let Some(direction) = to_anchor.try_normalize() else {
            continue;
        };
        let aligned = look_dir.dot(direction).max(0.0);
        blend += direction * aligned;
        gate = f32::max(gate, aligned);
    }
    let along = blend.try_normalize().unwrap_or(Vec3::ZERO) * (t.speed_m_s * forward * gate);

    let (sin, cos) = yaw.sin_cos();
    let right = Vec3::new(cos, 0.0, -sin);
    let sideways = right * (t.lateral_m_s * move_x);
    let driven = along + sideways;
    // Point 2 above. Exact equality and not an epsilon: the three ways to get here are all
    // exact zeros (no key, `S` alone, or every rope behind the look), and an epsilon would put
    // a dead band where a player who *is* holding a key gets nothing.
    if driven == Vec3::ZERO {
        return Vec3::ZERO;
    }
    // **Point 6, and it is a second TARGET, not a second chase.** With `W` released the drive
    // commands the sideways axis and **keeps the flight on every other one** — the player's own
    // velocity is the target there, so there is nothing to brake.
    let kept = velocity_m_s - right * velocity_m_s.dot(right) + sideways;
    // One cap for the whole thing, so `W`+`D` is a DIRECTION between them at the drive's own top
    // speed and not 1.06 of it — the same rule `air_thrust`'s `clamp_length_max` applies to the
    // same two axes, and the reason it sits outside the `lerp` is that it has to hold for the
    // kept flight too: without it `D` alone adds its 30 m/s to a 70 m/s flight and the player
    // arrives at `vector.max_speed_m_s`, i.e. at the avian clamp — measured 75.000 exactly.
    let target = kept.lerp(driven, forward).clamp_length_max(t.speed_m_s);
    (target - velocity_m_s) / t.ramp_s
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
/// [`Gas`] is **read**, never written — `vector::gas::gas_budget` is its one writer, and that did
/// not change when the rope term started costing something. This system reads the *verdict*
/// ([`GasGrant::steer`]) instead of the tank, exactly like `vector::boost::gas_boost` reads
/// `grant.boost`: thrust and debit are one decision, made once, in the file that owns the tank.
/// The free-air look term still reads `Gas` directly, because it is not gated on gas at all —
/// it only halves.
///
/// [`Hook`] and [`Transform`] joined the query with the mixing rule and are read-only here;
/// `vector::hook::update_hooks` keeps `tip_m` on an anchored arm in world space, and it runs in
/// `SimulationSystems::Intent`, one set ahead of this one.
///
/// [`Velocity`] and [`MovementState`] both come from the **end of the previous tick**
/// (`super::integrator::readback`), so the state and the speed this decides on are one
/// consistent snapshot instead of two ticks mixed. It also keeps `LinearVelocity` out of the
/// query, which [`Forces`] declares `Write` access to.
pub fn air_control(
    data: Res<GameData>,
    mut players: Query<(
        &Intent,
        &MovementState,
        &Velocity,
        &Gas,
        &Hook,
        &GasGrant,
        &Transform,
        &mut RunAccel,
        Option<Forces>,
    )>,
) {
    let s = &data.game.player;
    let top_m_s = super::integrator::ground_top_speed_m_s(&data);
    // Out of the file, not out of gravity — see the header. `tests/data.rs` guards the bound
    // the old derivation used to guarantee on its own.
    let full_m_s2 = s.air_accel_m_s2;
    let steer = SteerTuning {
        pull_m_s2: s.air_pull_m_s2,
        lateral_m_s2: s.air_lateral_m_s2,
        fade_m: s.air_pull_fade_m,
        min_rope_m: data.game.vector.min_rope_m,
        // The one place `gravity_m_s2` is read for this: a fraction in the file, m/s² in the
        // struct. `-` because the RON carries gravity as the negative number it is.
        lift_m_s2: -data.game.gravity_m_s2 * s.air_pull_lift_fraction,
    };
    // `FIND-149`. Read once, outside the loop, like every other tuning value here — and read
    // even under `Pendulum`, because a `game.ron` that is missing a `drive_*` key has to crash
    // on load whichever model it selects (§4: no `serde(default)` for a game value).
    let model = data.game.vector.rope_force_model;
    let drive_tuning = DriveTuning {
        speed_m_s: data.game.vector.drive_speed_m_s,
        lateral_m_s: data.game.vector.drive_lateral_m_s,
        ramp_s: data.game.vector.drive_ramp_s,
    };

    for (intent, state, velocity, gas, hook, grant, transform, mut drive, forces) in &mut players {
        let wanted = if in_flight(*state, velocity.0.xz().length(), top_m_s) {
            let accel_m_s2 =
                if gas.is_empty() { full_m_s2 * s.air_accel_empty_fraction } else { full_m_s2 };
            let look =
                air_thrust(intent.look_dir(), intent.yaw, intent.move_x, intent.move_y, accel_m_s2);

            // The hand, not the feet — the same point `vector::hook` fires from and
            // `hud::crosshair` measures from, so the rope this steers along is the rope that is
            // drawn (`player.eye_height_m`, one key, three readers).
            let hand_m = transform.translation + Vec3::Y * s.eye_height_m;
            let mut to_anchors_m = [Vec3::ZERO; 2];
            let mut anchored = 0;
            for arm in &hook.arms {
                if arm.state.is_anchored() {
                    to_anchors_m[anchored] = arm.tip_m - hand_m;
                    anchored += 1;
                }
            }

            // **`grant.steer` is the whole check** — it already means "a rope holds, a key is
            // down and this tick's gas was paid" (`vector::gas`). And the branch is a branch and
            // not a `+ Vec3::ZERO`: an unhooked player has to come out of this function
            // **bit-identical** to the version before the mixing rule existed, which
            // `tests/player.rs::f006_without_a_rope_the_air_control_is_bit_identical_to_before`
            // asserts with `assert_eq!` and not with an epsilon.
            //
            // On an empty tank the two halves part company on purpose: `look` keeps
            // `air_accel_empty_fraction` (*„ohne gas kann man immernoch w a d nutzen … aber
            // hälfte ca"*), the rope term is **zero**. Half a rope pull for no gas would be the
            // free thrust §1B was rewritten to remove.
            // `FIND-149` — **the fork, and it is one line in `game.ron`.** `Pendulum` is the
            // branch that stood here before 2026-08-23, untouched; `Drive` is the reference's
            // model, and the two must not be allowed to look alike
            // (`tests/player.rs::f149_the_two_force_models_are_not_the_same_thing`).
            //
            // `look` is added in **both** branches and is the same term in both, so an unhooked
            // player comes out of this function bit-identical under either model — which is
            // what `f006_without_a_rope_the_air_control_is_bit_identical_to_before` asserts
            // with `assert_eq!` and no epsilon.
            let rope_term = match model {
                RopeForceModel::Pendulum if anchored > 0 && grant.steer => rope_steer(
                    &to_anchors_m[..anchored],
                    intent.look_dir(),
                    intent.yaw,
                    intent.move_x,
                    intent.move_y,
                    steer,
                ),
                // **`grant.steer` OR an empty tank**, and the two are not the same gate.
                // `grant.steer` means "a rope holds, a key is down and this tick's gas was
                // paid" (`vector::gas`) — the drive is billed exactly like the pendulum's pull
                // and through the same predicate, so `FIND-150`'s *idle costs nothing* survives
                // unchanged. An **empty** tank is the user's own exception and not a hole in
                // the ledger: *„ohne gas kann man immernoch w a d nutzen um etwas movement
                // aufzubauen (aber hälfte ca)"* (`docs/NEXT.md` §1e) — there is nothing left to
                // debit, so the drive runs at [`crate::data::PlayerTuning::air_accel_empty_fraction`]
                // of its speed instead of stopping the player dead in the air.
                RopeForceModel::Drive if anchored > 0 && (grant.steer || gas.is_empty()) => {
                    let t = if gas.is_empty() {
                        drive_tuning.scaled(s.air_accel_empty_fraction)
                    } else {
                        drive_tuning
                    };
                    rope_drive(
                        &to_anchors_m[..anchored],
                        intent.look_dir(),
                        intent.yaw,
                        intent.move_x,
                        intent.move_y,
                        velocity.0,
                        t,
                    )
                }
                _ => Vec3::ZERO,
            };
            look + rope_term
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
