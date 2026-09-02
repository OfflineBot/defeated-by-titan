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

use crate::data::{GameData, RopeForceModel, VectorTuning};
use crate::shared::{
    Buttons, Gas, GasGrant, Hook, Intent, Invulnerable, MovementState, RunAccel, Slide, Tick,
    Velocity,
};

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
    tick: Res<Tick>,
    mut players: Query<(
        &Intent,
        &MovementState,
        &mut LinearVelocity,
        Option<&Slide>,
        &Hook,
        &Transform,
    )>,
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
    // §5E-b: the same three values `air_control`'s winch runs on — ONE constructor, so the
    // two systems cannot disagree about what a live pull is.
    let model = data.game.vector.rope_force_model;
    let idle_winch = WinchTuning::idle(&data.game.vector);
    let floor_fraction = data.game.vector.drive_steer_pull_fraction;

    for (intent, state, mut velocity, slide, hook, transform) in &mut players {
        // On the rope the body belongs to `vector`; downed and on the wall are states of
        // their own. This system speaks only for a player standing on the ground.
        if *state != MovementState::Grounded {
            continue;
        }

        // 🔴 **§5E-b (2026-09-01): a live pull owns the body, and the legs let go — the tick
        // the hook bites, not the tick the feet leave.** Third ruling on this gate; see
        // [`ground_pull_live`] for the trail. Without this `continue`, `ground_step` assigns
        // the XZ velocity every tick a slow player is `Grounded` and deletes the drag the
        // winch just built — the pull would be a buzz that grinds in place instead of a haul
        // (below ~69° of elevation the winch cannot out-lift gravity and closing the distance
        // IS the horizontal axis; the threshold is derived at the winch's call site).
        //
        // The checks before the slide branch, deliberately: a bite during a slide hands the
        // body to the rope mid-slide — the yank is the point of §5E-b, and the slide's
        // i-frames keep running on their own clock (`Invulnerable` is not touched here).
        // `S` (pull_scale = 0) or a release is one tap and gives the legs back, jump and all.
        let hand_m = transform.translation + Vec3::Y * s.eye_height_m;
        let mut to_anchors_m = [Vec3::ZERO; 2];
        let mut anchored = 0;
        for arm in &hook.arms {
            if arm.state.is_anchored() {
                to_anchors_m[anchored] = arm.tip_m - hand_m;
                anchored += 1;
            }
        }
        if ground_pull_live(
            model,
            &to_anchors_m[..anchored],
            intent.move_x,
            intent.move_y,
            idle_winch,
            floor_fraction,
        ) {
            continue;
        }

        // **`F-010` — a slide is not a run, and this is where the two are told apart.**
        //
        // *„Gleit-Ausweichmanoever am Boden mit I-Frames und Momentum-Erhalt ... geht fliessend
        // in Sprint ueber."* The slide holds ONE direction — the one it started in — and one
        // speed: the larger of `player.slide_speed_m_s` and what the player already had. That
        // `max` is the *„Momentum-Erhalt"* half of the row in one line: landing at 30 m/s and
        // sliding keeps 30, and a slide can therefore never be a brake.
        //
        // **It bypasses `ground_step` entirely, and that is the decision.** `ground_step` is
        // the run: it decelerates anything above `run_speed_m_s` at μg and it steers with the
        // stick. Both are wrong for a slide — a slide you can steer is a run, and a slide that
        // decays is not a dodge. When the deadline passes, this branch simply stops running and
        // `ground_step` picks the velocity up **where the slide left it**: nothing is reset,
        // nothing is zeroed, which is exactly *„geht fliessend in Sprint ueber"*.
        //
        // `velocity.y` is untouched, so gravity and the ground contact keep doing their work
        // and a slide off a ledge becomes a fall instead of a hover.
        if let Some(slide) = slide.filter(|s| s.active(tick.0)) {
            let kept = velocity.0.xz().length().max(s.slide_speed_m_s);
            let v = slide.dir_m * kept;
            velocity.x = v.x;
            velocity.z = v.z;
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
        // the apex came out at 1.1588 m instead of 1.0562 m — measured on 2026-08-09, when
        // `jump_speed_m_s` was 6.5 against a `gravity_m_s2` of −20. **Both numbers moved on
        // 2026-08-27** (8.2 against −32) and the apex they allow is `v²/2g` = **1.0506 m** —
        // the pair was chosen to hold the height and change the *time*, so the 10 % the guard
        // is worth is the same 10 %. You push OFF the ground; you cannot push off it while
        // already leaving it.
        // Measured against a false positive: at rest `velocity.y` is exactly 0.0 — over 300
        // ticks it was never once greater.
        if intent.pressed(Buttons::JUMP) && velocity.y <= 0.0 {
            // Speed, not an impulse: `jump_speed_m_s` is a height you can compute in your
            // head (v²/2g), and it stays that number no matter what the capsule weighs.
            velocity.y = s.jump_speed_m_s;
        }
    }
}

/// `F-010` **Slide-Dodge am Boden** — starts a slide, and grants the i-frames that are the
/// point of it.
///
/// > *„Gleit-Ausweichmanoever am Boden mit I-Frames und Momentum-Erhalt."* — `F-010`.
/// > Acceptance: *„Slide vermeidet Stomp-Angriff; geht fliessend in Sprint ueber."*
///
/// ## One button, two verbs, and the state decides which
///
/// `Buttons::DODGE` is the dash in the air (`F-008`, `vector::boost`) and the slide on the
/// ground. That is not a saving of keys, it is what the reference does and what a player
/// expects: the evasive button evades with whatever you are standing on. `vector::gas` reads
/// the same `MovementState` from the other side and refuses to bill a **flip** on the ground
/// for the identical reason — one evasion per state, never two at two prices.
///
/// ## Why it costs no gas
///
/// It is the only verb in this round that is free, and that is a decision: gas is what the
/// **gear** burns, and a slide is legs. Billing it would make the one move a player has left on
/// an empty tank cost the thing he has run out of, which is a death spiral wearing a mechanic's
/// hat. What bounds it instead is `player.slide_cooldown_s` — measured from the tick a slide
/// *starts*, so it includes the slide itself — and the fact that
/// `player.slide_min_speed_m_s` refuses one from standing. A held `C` is then one slide per 54
/// ticks and never a stance.
///
/// ## The i-frames are shorter than the movement, on purpose
///
/// `slide_iframes_s` (0.30 s) against `slide_duration_s` (0.55 s): the tail of the slide is the
/// part that carries you out from under the foot, and invulnerability that lasts as long as the
/// whole movement is a dodge nobody has to time. The acceptance sentence says *avoids* a stomp,
/// not *ignores* one.
///
/// **Sole writer of [`Slide`].** Contributor to [`Invulnerable`] through
/// [`Invulnerable::extend_to`], which is a `max` and therefore cannot shorten a window
/// `vector::dodge::flip` granted.
///
/// Runs in `SimulationSystems::Integrate` **before** [`ground_locomotion`] (`.chain()` in
/// `player::PlayerPlugin`), so a slide that starts this tick is already driving this tick's
/// velocity — a one-tick delay on an evasive move is a blow that lands.
pub fn start_slides(
    data: Res<GameData>,
    tick: Res<Tick>,
    mut players: Query<(
        &Intent,
        &MovementState,
        &LinearVelocity,
        &mut Slide,
        &mut Invulnerable,
    )>,
) {
    let s = &data.game.player;
    let hz = data.game.simulation_hz;
    // Seconds in the file, ticks in the code (`docs/conventions.md`). `round` and not `as`:
    // 0.55 s at 60 Hz is 33.000002 in f32 and truncating would silently cost a tick.
    let duration_ticks = (s.slide_duration_s as f64 * hz).round().max(0.0) as u64;
    let iframe_ticks = (s.slide_iframes_s as f64 * hz).round().max(0.0) as u64;
    let cooldown_ticks = (s.slide_cooldown_s as f64 * hz).round().max(0.0) as u64;

    for (intent, state, velocity, mut slide, mut iframes) in &mut players {
        if *state != MovementState::Grounded || !intent.pressed(Buttons::DODGE) {
            continue;
        }
        // **`F-028`'s rule, on the one button that now has no fallback.** Until 2026-08-25 a
        // grounded `C` that this system refused still bought a *dash* — `vector::gas` had no
        // ground test, so the press always did *something*. It does not any more, and a press
        // that answers with nothing is the failure `F-028` exists to remove. The hint belongs
        // on the HUD (`src/hud/arm_aim.rs`, somebody else's file); this is the log line that
        // says which of the two refusals it was. It cannot spam: `Buttons::DODGE` is an **edge**
        // — `net::local::DodgeTap` presses it on at most one tick per gesture.
        if !slide.ready(tick.0, cooldown_ticks) {
            // The clock is `started_at_tick` and not `until_tick`, because that is the one
            // `Slide::ready` counts from — a message that measures a different clock than the
            // refusal is a message that will one day say "0.00 s to go" while refusing.
            let ready_at = slide.started_at_tick.map_or(slide.until_tick, |t| t + cooldown_ticks);
            info!(
                "F-010: no slide at tick {} — still cooling down, {:.2} s to go",
                tick.0,
                ready_at.saturating_sub(tick.0) as f64 / hz,
            );
            continue;
        }
        // **The direction is the one he is already going**, not the one he is looking or
        // steering. A slide is momentum redirected, and there is nothing to redirect if there
        // is no momentum: below `slide_min_speed_m_s` there is no slide at all, which is also
        // what keeps `normalize_or_zero` from ever being asked for the zero vector.
        let flat = velocity.0.xz();
        if flat.length() < s.slide_min_speed_m_s {
            info!(
                "F-010: no slide at tick {} — {:.1} m/s is under slide_min_speed_m_s {:.1}",
                tick.0,
                flat.length(),
                s.slide_min_speed_m_s,
            );
            continue;
        }
        let dir = flat.normalize_or_zero();
        *slide = Slide {
            until_tick: tick.0 + duration_ticks,
            dir_m: Vec3::new(dir.x, 0.0, dir.y),
            started_at_tick: Some(tick.0),
        };
        iframes.extend_to(tick.0 + iframe_ticks);
        info!(
            "F-010: slide at tick {} — {:.1} m/s carried, until tick {}, i-frames to {}",
            tick.0,
            flat.length(),
            slide.until_tick,
            iframes.until_tick,
        );
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
    /// [`crate::data::VectorTuning::drive_accel_max_m_s2`] — the ceiling on the chase, and
    /// **the player's weight**. `<= 0` means no drive at all, the same safe direction as the
    /// ramp.
    pub accel_max_m_s2: f32,
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

/// `docs/NEXT.md` §5D (2026-09-01) — **the drive is the player's own thrust: `W` along the
/// LOOK, `A`/`D` across it. The radial pull is NOT here — it is [`rope_winch`]'s, always.**
///
/// The user, at the controller, refining `FIND-149`'s model mid-round:
///
/// > *„es sollte doch eigendlich so sein. dass es immer rangezogen wird standardmäßig. und es
/// > geht danach wie der character schaut. aber w geht in die richtung etc. aber wenn
/// > verbunden wird immer rangezogen so stark wie man zur seite geht desto weniger. aber
/// > dennoch AUßER man drückt S dann nur zur seite"*
///
/// ```text
/// l̂ = unit(look)   ê_right = (cos yaw, 0, −sin yaw)   w⁺ = clamp(move_y, 0, 1)
/// a_W   = l̂ · max(0, speed·w⁺ − v·l̂) / ramp                    (only while w⁺ > 0)
/// a_lat = ê_right · (lateral·clamp(move_x, −1, 1) − v·ê_right) / ramp   (only while move_x ≠ 0)
/// a     = clamp_len(a_W + a_lat, accel_max)
/// ```
///
/// ## What §5D changed here, and what survived it
///
/// 1. **The look GATE is gone, and `FIND-196` is what it cost.** The old drive chased a
///    velocity along the ROPE, scaled by `max(0, l̂·r̂ᵢ)` — so holding `W` closed **11.270 m**
///    where holding nothing closed **11.937 m**: the gate shrank the drive's target while the
///    winch's floor did not, and the composition `flight + winch` let the chase fight the
///    pull. Now the two terms own disjoint jobs: the winch owes the rope axis (rule 1), the
///    drive owes the look axis (rule 2), and neither can starve the other because
///    **`a_W` never brakes** — the same `max(0, …)` as [`rope_winch`]'s property 1. Re-measured
///    in `tests/input.rs::r7_*`: every forward key ≥ the free pull.
/// 2. **`W` goes where the character looks — not at the anchor.** *„aber w geht in die
///    richtung"*: the thrust direction is `l̂` itself. A rope 90° off the look used to zero
///    `W`; now it is simply not `W`'s business. The arms are still the guard (no rope, no
///    drive, and `vector::gas` still bills it through `grant.steer`), but their geometry
///    steers nothing here any more — which also retires the old model's direction blend: two anchors are
///    bit-identical to one (`tests/player.rs::f153_a_second_rope_does_not_halve_the_drive`).
/// 3. **It is still a chase, still with the `FIND-172` weight.** `(wanted − v·axis)/τ` per
///    axis, one `clamp_length_max(accel_max)` over the sum: the onset is the exponential
///    (*„etwas smoother übergang, aber recht schnell"*), a small correction is under the
///    ceiling and immediate (`FIND-153`), a large one costs time in proportion to its size.
/// 4. **`A`/`D` REPLACE the crossing momentum on their own axis and touch nothing else** —
///    `Q-050`'s two lessons, kept: with `W` released nothing chases the flight down (no term
///    without its key), and the lateral target is a bound, not an addition (`D` on a fast
///    flight cannot ride it up to `vector.max_speed_m_s`; the ê_right component is chased to
///    `lateral·mx` and the other axes are never read).
/// 5. **What `W` no longer does: eat the crossing momentum.** `FIND-153`'s *„ziemlich
///    gerade"* was bought by chasing the WHOLE velocity onto the rope line; §5D supersedes it
///    — the pull is the winch's and the straightness now comes from thrust + pull sharing a
///    direction whenever the player looks where he flies. His newer word wins (`Q-002`).
/// 6. **What `W` also no longer does: brake a flight above `drive_speed_m_s`.** A thrust on
///    top of the pull (§5D rule 2) has no business slowing anything; the old full-velocity
///    chase braked everything above its target. `vector.max_speed_m_s` stays the outer bound.
/// 7. **`S` is not here at all** — `w⁺` clamps it away, as ever. Its §5D rule 4 job (cancel
///    the pull) lives in [`pull_scale`], at the winch's call site.
///
/// ## Where the old §5C sentences went
///
/// - *„nur wenn ich a oder d drücke dass es stärker zur seite geht als rangezogen"*
///   (`FIND-172`): the lateral (24 m/s target) beats the scaled pull
///   (`drive_idle_speed_m_s · pull_scale ≤ 12`) by construction now — measured in
///   `tests/player.rs::f172_a_or_d_turns_the_drive_further_sideways_than_the_rope_pulls_it_in`.
/// - `drive_steer_pull_fraction` left this struct: it is [`pull_scale`]'s endpoint — the
///   fraction of the PULL that survives full lateral — not a direction weight in the drive.
/// - The fade paragraph: the drive has no radial component left to run into `min_rope_m`, and
///   the winch has its own floor (property 3). `FIND-191`'s joint cliff is unchanged.
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
    // Both guards are the safe direction for a number out of a file, same as `rope_winch`'s:
    // a ramp of zero divides, and no rope means no drive — the guard is the arms even though
    // their geometry steers nothing here (§5D point 2 above).
    if to_anchors_m.is_empty() || !(t.ramp_s > 0.0) {
        return Vec3::ZERO;
    }
    let forward = move_y.clamp(0.0, 1.0);
    let mut accel = Vec3::ZERO;
    if forward > 0.0 {
        // Same guard as everywhere in this file: a zero look has no direction, and a NaN in a
        // velocity is unrecoverable.
        if let Some(along_look) = look_dir.try_normalize() {
            let wanted_m_s = t.speed_m_s * forward;
            let has_m_s = velocity_m_s.dot(along_look);
            // `max(0, …)` by branch: the thrust never brakes (§5D point 6 — `rope_winch`
            // property 1's rule, on the look axis).
            if has_m_s < wanted_m_s {
                accel += along_look * ((wanted_m_s - has_m_s) / t.ramp_s);
            }
        }
    }
    if move_x != 0.0 {
        let (sin, cos) = yaw.sin_cos();
        let right = Vec3::new(cos, 0.0, -sin);
        let wanted_m_s = t.lateral_m_s * move_x.clamp(-1.0, 1.0);
        // No `max(0, …)` here and that is the difference between an axis you steer and an
        // axis you ride: the lateral REPLACES the crossing momentum (§5D point 4), so it may
        // slow the ê_right component — that is what turning is — and only that component.
        accel += right * ((wanted_m_s - velocity_m_s.dot(right)) / t.ramp_s);
    }
    // The `FIND-172` weight: one ceiling over the sum, so `W`+`D` is one player's strength
    // and not two.
    accel.clamp_length_max(t.accel_max_m_s2)
}

/// The three numbers [`rope_winch`] is made of — and **not one of them is new**: they are
/// `vector.drive_idle_speed_m_s`, `vector.min_rope_m` and `vector.drive_idle_ramp_s`, already
/// in `game.ron` and already tuned.
///
/// A struct and not three `f32` in a row, for [`DriveTuning`]'s reason: swap the speed and the
/// floor and the compiler says nothing while the winch silently becomes a 3 m/s crawl that
/// stops 28 m short of the anchor.
///
/// ⚠️ **Until `Q-058` the first two of these were `reel_speed_m_s` and `drive_ramp_s`, because
/// `Ctrl` came through here too.** It does not any more — `Ctrl` shortens the joint, see
/// [`rope_winch`]'s header — so the struct now carries the idle pull's numbers and nothing
/// else. `tests/player.rs` still builds it by hand at other values, which is what a pure
/// function is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WinchTuning {
    /// [`crate::data::VectorTuning::drive_idle_speed_m_s`] — the closing speed the always-on
    /// pull winds in at.
    pub speed_m_s: f32,
    /// [`crate::data::VectorTuning::min_rope_m`] — the floor. The joint clamps its
    /// `limits.max` here; the winch simply stops here.
    pub min_rope_m: f32,
    /// [`crate::data::VectorTuning::drive_idle_ramp_s`] — the time constant of the onset.
    /// `<= 0` means **no winch at all**, the safe direction for a number that divides.
    pub ramp_s: f32,
    /// **The ceiling on the haul, in m/s² — and it exists because of what a winch does to a
    /// player flying AWAY from his anchor** (`FIND-172`).
    ///
    /// Property 1 below says the winch can never brake, and that is true of the *closing*
    /// speed. It is not a bound on the *acceleration*: at 30 m/s outbound the coefficient is
    /// `(speed + 30)/ramp`, which at the idle pull's own numbers is **120 m/s²** against the
    /// 34 m/s² it produces from rest. Measured 2026-08-26 on
    /// `tests/player.rs::f004_the_ground_does_not_write_the_velocity_of_a_player_the_rope_drags`:
    /// with the always-on pull unbounded, a player handed over to the rope at 29.67 m/s was at
    /// **0.00 m/s** half a second later. That is "zu aggressiv" in a new key.
    ///
    /// `air_control` derives it as `drive_idle_speed_m_s / drive_idle_ramp_s` rather than
    /// reading a fifth key: the always-on pull may never haul harder than it does off a
    /// standing start.
    pub accel_max_m_s2: f32,
}

/// How much of the always-on pull survives the player's own movement keys, `0..=1` —
/// `docs/NEXT.md` §5D rules 3 and 4 as one function.
///
/// ```text
/// scale = move_y < 0 ? 0 : 1 − (1 − floor_fraction) · clamp(|move_x|, 0, 1)
/// ```
///
/// **Rule 3, the ramp:** *„wenn verbunden wird immer rangezogen so stark wie man zur seite
/// geht desto weniger"* — proportional, so the curve is a LINE in `|move_x|` from `1.0` to
/// `floor_fraction` (= `game.ron: vector.drive_steer_pull_fraction`). The `1.0` endpoint is
/// deliberately not a file key: it is rule 1 itself (anchored ⇒ always pulled by default),
/// and a tunable there would let the file contradict the rule. The fraction endpoint is
/// **never zero** — „desto weniger", not „bis nichts" — which `tests/player.rs::
/// f149_the_drive_numbers_are_the_ones_the_file_can_defend` holds against the file.
/// Until 2026-09-01 this was a binary switch (`mx == 0 ? 1 : fraction`) buried in
/// `rope_drive`'s direction blend; §5D names it a ramp on the PULL and it lives here, at the
/// winch's call site ([`WinchTuning::scaled`]).
///
/// **Rule 4, the cancel:** *„aber dennoch AUßER man drückt S dann nur zur seite"* — with `S`
/// held the pull is zero and only the lateral remains. `move_y < 0` and not "S": the axis is
/// what the `Intent` carries, so `W`+`S` (0) is the no-key ramp, not a cancel — one keyboard,
/// one axis, no second channel to disagree with it. The THIRD meaning `S` has had; the trail
/// is `docs/QUESTIONS.md` Q-061 → Q-091.
#[must_use]
pub fn pull_scale(move_x: f32, move_y: f32, floor_fraction: f32) -> f32 {
    if move_y < 0.0 {
        return 0.0;
    }
    1.0 - (1.0 - floor_fraction.clamp(0.0, 1.0)) * move_x.abs().clamp(0.0, 1.0)
}

impl WinchTuning {
    /// The always-on pull's numbers out of the file, in ONE place — `air_control` and
    /// `ground_locomotion` both need them since §5E-b, and two constructions of the same
    /// struct are two chances for the derived ceiling to disagree.
    ///
    /// **The ceiling is derived, not a fifth key**: the always-on pull may never haul harder
    /// than it does off a standing start. Without it a player flying *away* from his anchor
    /// is yanked back at `(idle + v)/ramp` (`FIND-172`, and it took a `F-004` test to
    /// 0.00 m/s).
    #[must_use]
    pub fn idle(v: &VectorTuning) -> Self {
        Self {
            speed_m_s: v.drive_idle_speed_m_s,
            min_rope_m: v.min_rope_m,
            ramp_s: v.drive_idle_ramp_s,
            accel_max_m_s2: v.drive_idle_speed_m_s / v.drive_idle_ramp_s,
        }
    }

    /// The same winch at a fraction of its closing speed — §5D rule 3's ramp is applied HERE,
    /// at the call site, so [`rope_winch`] itself stays input-blind.
    ///
    /// **The ceiling scales with the speed**: the scaled pull may never haul harder than IT
    /// does off a standing start — the same derivation `air_control` makes for the unscaled
    /// one ([`WinchTuning::accel_max_m_s2`]).
    #[must_use]
    pub fn scaled(self, factor: f32) -> Self {
        Self {
            speed_m_s: self.speed_m_s * factor,
            accel_max_m_s2: self.accel_max_m_s2 * factor,
            ..self
        }
    }
}

/// **Does the always-on pull own this body?** — `docs/NEXT.md` §5E-b (2026-09-01), the THIRD
/// ruling on the ground gate, and the newest word wins (`Q-002`'s precedence rule):
///
/// > *„und aktuell wenn cih mich hooke werde ich nicht autmoatisch rangezogen! das fehlt
/// > noch! aktuell muss ich noch in die richtung schauen bewegen! fixe das noch!"*
///
/// 1. `FIND-172` built the pull; 2. `Q-055`/`Q-056` gated it `in_the_air` so a hooked player
/// in the hub kept his legs; 3. §5E-b overturns the gate: **a bite pulls immediately, ground
/// included** — the hub worry is defused by the release being one tap.
///
/// Two readers, one decision (CLAUDE.md rule 5's drift warning): `air_control` uses this to
/// run the winch and the flight controls for a grounded, pulled body; `ground_locomotion`
/// uses it to let go of that same body — the legs and the rope must agree on whose tick it
/// is, or the ground writes the XZ velocity the winch just built (measured under `FIND-182`:
/// a ground brake plus a rope is an elevator).
///
/// The conditions, and why each one is here:
/// - **`Drive` only** — `Pendulum` has no winch, and that model must stay bit-identical.
/// - **`speed > 0` and `ramp > 0`** — [`rope_winch`]'s own off-switches. A deleted pull
///   (`drive_idle_speed_m_s: 0`, the test fixture's control) must hand the legs back too,
///   or the fixture measures a limp body instead of the ground.
/// - **[`pull_scale`]` > 0`** — §5D rule 4 on the ground: `S` cancels the pull, so the
///   `S`-holder keeps his legs and stands planted. The lateral ramp (rule 3) never reaches
///   zero (`drive_steer_pull_fraction`, guarded by `f149_*`), so `A`/`D` ride the pull
///   instead of cancelling it, exactly as in the air.
/// - **An arm beyond `min_rope_m`** — property 3's floor. A bite inside the floor pulls
///   nothing, so it must not take the legs either.
///
/// **Deliberately NOT a condition: the closing speed.** [`rope_winch`] goes quiet while the
/// body already closes at `drive_idle_speed_m_s`, and a predicate that read the velocity
/// would flip legs/rope every few ticks around that line — the body would stutter between
/// the winch and `ground_step`'s assignment. Owned is owned until a key or the geometry says
/// otherwise.
///
/// ⚠️ **The one shape this leaves standing, measured and not hidden:** two arms whose
/// directions cancel (opposed anchors) are a live pull with no axis — [`rope_winch`] returns
/// `ZERO`, the legs are let go, and the body stands still under gravity until a key moves it
/// (the flight controls run for it) or `S`/release hands it back. Rare on purpose: it needs
/// two bites at opposite azimuths at leg height.
#[must_use]
pub fn ground_pull_live(
    model: RopeForceModel,
    to_anchors_m: &[Vec3],
    move_x: f32,
    move_y: f32,
    t: WinchTuning,
    floor_fraction: f32,
) -> bool {
    model == RopeForceModel::Drive
        && t.speed_m_s > 0.0
        && t.ramp_s > 0.0
        && pull_scale(move_x, move_y, floor_fraction) > 0.0
        && to_anchors_m.iter().any(|a| a.length() > t.min_rope_m)
}

/// **The always-on pull under [`crate::data::RopeForceModel::Drive`]** — closing speed along
/// the rope, and nothing else.
///
/// ```text
/// r̂ = unit( Σᵢ unit(tipᵢ − h) )   over the arms further away than `min_rope_m`
/// c  = v · r̂                                          (the closing speed the player already has)
/// a  = r̂ · max(0, idle_speed − c) / ramp
/// ```
///
/// ## Why this exists at all, and why it is not `W`
///
/// The user, 2026-08-26: *„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"*, and
/// since §5D rule 1 this function is THE pull — the drive has no radial term left at all. A
/// hooked player is pulled toward his anchor **with no key held**, at the low point of a swing
/// with the anchor behind and above him (`F-005`: *„Spieler kann aus dem Tiefpunkt Hoehe
/// gewinnen"*), whatever he looks at — no look enters this function, and removing the look's
/// last grip on the pull is what closed `FIND-196`.
///
/// So the two terms are one trade and the player can feel which is which:
///
/// | | `W` — the drive | the free pull |
/// |---|---|---|
/// | speed | `drive_speed_m_s` 52 | `drive_idle_speed_m_s` 12 |
/// | key | `W`, and it costs gas | none, and it costs nothing |
/// | direction | **the look's, exactly** (§5D rule 2) | the rope axis, straight up the rope |
/// | scaled by | nothing but the empty tank | [`pull_scale`]: the lateral ramp, `S` = 0 |
/// | ends | at `drive_speed_m_s` along the look | at `min_rope_m` |
///
/// 🔴 **`Ctrl` was here until `Q-058` and is not any more (2026-08-27).** This function used to
/// carry the reel as well, at `reel_speed_m_s`, and the reason was one sentence: *„`Drive`
/// builds no `DistanceJoint`, so there is no length for `player::rope::shorten_ropes` to
/// shorten and `Ctrl` was a dead key that `vector::gas` still billed"* (`Q-050`, `FIND-152`).
/// **That sentence is false since `player::rope::attach_ropes` gives a `Drive` rope a joint**,
/// so the reel went back to being what it is under `Pendulum` — a change of `limits.max`, with
/// the `L_prev/L_new` amplification of the tangential velocity that an acceleration on the body
/// cannot reproduce (`player::rope`'s header: 58.23 m/s out of `v0 = 20`, against exactly
/// 20.000 for the hand-written version that was retired). Keeping both would have paid the reel
/// once and delivered it twice.
///
/// ## The four properties, and each one is a test
///
/// 1. **It can never brake.** The coefficient is `max(0, idle_speed − c)`, so the winch only
///    ever *raises* the closing speed toward `drive_idle_speed_m_s`. Already closing at 40 m/s? The
///    term is `Vec3::ZERO`. That is not a nicety: `Q-050`'s other half was a released `W` whose
///    chase read a 52.9 m/s flight as an error and killed it, and a winch that *sets* the
///    closing speed would be the same bug in a new key.
/// 2. **It touches the rope axis and nothing else.** Your crossing momentum is not part of the
///    dot product and never appears in the output, so winching in the middle of a swing keeps
///    the swing. That is the one thing the pendulum's reel is famous for
///    (`shared::rope::rope_reel_in` scales the tangential velocity instead of eating it), and
///    it is the half of it that survives without a constraint. **What does NOT survive is the
///    amplification** — `L_prev/L_new` is the constraint's doing and there is no constraint.
/// 3. **It stops at `min_rope_m`,** per arm, the same floor `shorten_ropes` clamps its
///    `limits.max` to. An arm inside the floor contributes no direction at all, so a player who
///    has arrived is not pushed into the wall he is hanging on.
/// 4. **No look gate, and that is the whole point** (see the table). It is also why the winch
///    is **not** billed at all: nothing is held, so there is nothing `vector::gas` could charge
///    for — see `tests/vector_rope.rs::f172_*`, whose empty-tank control is exactly this claim.
/// 5. **It has a ceiling, and property 1 is why it needs one** (`FIND-172`). "Can never brake"
///    is a statement about the closing speed and **not** about the acceleration: a player
///    travelling *outbound* at 30 m/s is 42 m/s of gap, i.e. 120 m/s² at the idle pull's
///    numbers, against the 34 m/s² the same pull produces from rest. See
///    [`WinchTuning::accel_max_m_s2`] for the measurement.
///
/// ⚠️ **The one shape that is still ugly, measured and not explained away:** fly through an
/// anchor in **open air** — a hook that bit a lamp post rather than a wall — and you shoot
/// past it, `r̂` flips, and the winch hauls you back. It is bounded by `drive_idle_speed_m_s`
/// and it
/// is exactly what the pendulum does at `min_rope_m` (`FIND-035`: 17 m/s out of the player in
/// one tick), so it is not a regression — but it is not designed either. Anchors sit on
/// surfaces, which is why it is hard to reach: the wall arrives before the flip does.
/// → `docs/QUESTIONS.md` Q-050.
#[must_use]
pub fn rope_winch(to_anchors_m: &[Vec3], velocity_m_s: Vec3, t: WinchTuning) -> Vec3 {
    // Both guards are the safe direction for a number out of a file: a ramp of zero divides,
    // and a winch speed of zero is a winch that is switched off.
    if to_anchors_m.is_empty() || !(t.ramp_s > 0.0) || !(t.speed_m_s > 0.0) {
        return Vec3::ZERO;
    }
    let mut blend = Vec3::ZERO;
    for to_anchor in to_anchors_m {
        // Property 3. `length` and not `length_squared` against a squared floor, because the
        // floor is read from the file in metres and squaring it here would be a second place to
        // get `min_rope_m` wrong.
        if to_anchor.length() <= t.min_rope_m {
            continue;
        }
        // Same guard as [`rope_drive`]: a tip exactly on the hand has no direction, and a NaN
        // in a velocity is unrecoverable (§9d).
        let Some(direction) = to_anchor.try_normalize() else {
            continue;
        };
        blend += direction;
    }
    // Two arms 180° apart cancel, and then there is no rope axis to wind along — `ZERO`, not a
    // NaN and not an arbitrary pick between them.
    let Some(along) = blend.try_normalize() else {
        return Vec3::ZERO;
    };
    // Property 1, and it is one `max`.
    let closing_m_s = velocity_m_s.dot(along);
    if closing_m_s >= t.speed_m_s {
        return Vec3::ZERO;
    }
    // Property 2: the whole output lies on `along`, so no other axis of the velocity is read
    // and none is written. Property 5, the ceiling, is the only thing that may shorten it.
    (along * ((t.speed_m_s - closing_m_s) / t.ramp_s)).clamp_length_max(t.accel_max_m_s2)
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
        accel_max_m_s2: data.game.vector.drive_accel_max_m_s2,
    };
    // `FIND-172`'s always-on pull, and since `Q-058` that is **all** the winch is: *„ich will
    // dass es immer ranzieht. nicht nur wenn ich w drücke!"* `Ctrl` is not here any more — see
    // the `winch` match below. One constructor, shared with `ground_locomotion` (§5E-b).
    let idle_winch = WinchTuning::idle(&data.game.vector);

    for (intent, state, velocity, gas, hook, grant, transform, mut drive, forces) in &mut players {
        // The hand, not the feet — the same point `vector::hook` fires from and
        // `hud::crosshair` measures from, so the rope this steers along is the rope that is
        // drawn (`player.eye_height_m`, one key, three readers).
        //
        // ⚠️ **Hoisted out of the flight branch on 2026-08-25, and that is a decision, not
        // tidying.** The winch below has to work on the ground: `scripts/game-full.txt` ACT 1
        // starts with a player *standing still* — `MovementState::Grounded`, so
        // [`in_flight`] is false — and `F-005`'s whole acceptance sentence is that he gets off
        // it. The pendulum's reel never had this problem, because it works on the rope's
        // `limits.max` and not on the body. Two subtractions per tick; nothing else moved.
        let hand_m = transform.translation + Vec3::Y * s.eye_height_m;
        let mut to_anchors_m = [Vec3::ZERO; 2];
        let mut anchored = 0;
        for arm in &hook.arms {
            if arm.state.is_anchored() {
                to_anchors_m[anchored] = arm.tip_m - hand_m;
                anchored += 1;
            }
        }

        let in_the_air = in_flight(*state, velocity.0.xz().length(), top_m_s);
        // §5E-b (2026-09-01): a live pull owns the body, standing included — the third
        // ruling on the ground gate, see [`ground_pull_live`]. `ground_locomotion` reads the
        // SAME predicate and lets go of exactly these bodies, so the flight controls here
        // are not fighting `ground_step`'s assignment. `OnWall` and `Downed` stay excluded:
        // §5E-b overturned the GROUND gate, not the wall run and not the death floor.
        let pulled = !matches!(*state, MovementState::OnWall | MovementState::Downed)
            && ground_pull_live(
                model,
                &to_anchors_m[..anchored],
                intent.move_x,
                intent.move_y,
                idle_winch,
                data.game.vector.drive_steer_pull_fraction,
            );
        let flight = if in_the_air || pulled {
            let accel_m_s2 =
                if gas.is_empty() { full_m_s2 * s.air_accel_empty_fraction } else { full_m_s2 };
            let look =
                air_thrust(intent.look_dir(), intent.yaw, intent.move_x, intent.move_y, accel_m_s2);

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

        // **The always-on pull** (`FIND-172`, [`rope_winch`]): *„ich will dass es immer
        // ranzieht. nicht nur wenn ich w drücke!"* (the user, 2026-08-26). It is free — no key,
        // no grant, nothing bills it — and it is bounded by a closing **speed**, so a hooked
        // player who presses nothing is carried and never hauled.
        //
        // 🔴 **`Ctrl` IS NOT HERE ANY MORE — `Q-058` folded it back into the joint, 2026-08-27,
        // and that is one key with one mechanism instead of two.** It stood here for exactly
        // one reason, written in [`rope_winch`]'s own header: *„`Drive` builds no
        // `DistanceJoint`, so there is no length for `player::rope::shorten_ropes` to shorten
        // and `Ctrl` was a dead key that `vector::gas` still billed"* (`Q-050`). **A `Drive`
        // rope has a joint now** (`player::rope::attach_ropes`), so `shorten_ropes` runs on it
        // and `Ctrl` shortens `limits.max` exactly as it always has under `Pendulum` — with the
        // angular-momentum amplification that is the whole feel of the Vector Gear (58.23 m/s
        // out of `v0 = 20`, `player::rope`'s header) and that an acceleration on the body
        // cannot produce. Leaving both in would pay the reel once and deliver it twice, which
        // is the sentence
        // `tests/player.rs::f005_ctrl_never_adds_an_acceleration_to_the_body_under_either_model`
        // already made about `Pendulum` and now makes about both.
        //
        // **The gas ledger did not move**: `vector::gas` still grants `reel_in`, `vector::reel`
        // still writes `ReelSpeed` on that grant, and `shorten_ropes` still consumes it. What
        // changed is only which of the two mechanisms spends it.
        //
        // - 🔴 **Ground included, since §5E-b (2026-09-01).** Until then this term was gated
        //   `in_the_air` — the deliberate `Q-055`/`Q-056` decision that a hooked player
        //   standing on the ground kept his legs. The user overturned it, third ruling,
        //   newest word wins: *„wenn cih mich hooke werde ich nicht autmoatisch rangezogen!
        //   das fehlt noch!"* The bite pulls immediately; [`ground_pull_live`] is the gate
        //   now, and `ground_locomotion` reads the same predicate to let go of the body.
        //   **The contact-break threshold is derived, not tuned**: the ceiling is
        //   `drive_idle_speed_m_s / drive_idle_ramp_s` = 34.29 m/s² against gravity 32, so a
        //   standing bite lifts cleanly only above `asin(32/34.29)` ≈ 69° of elevation; below
        //   that the pull DRAGS along the ground (friction is 0.0) until the geometry
        //   steepens — measured across elevations in
        //   `tests/player.rs::f176_the_contact_break_threshold_is_the_derived_69_degrees`.
        // - **No empty-tank exception, and the drive has one.** Nothing bills this term, so
        //   there is no tank for it to be empty — `drive_idle_speed_m_s` pulls whatever the gas
        //   says.
        // - 🔴 **Since §5D (2026-09-01) the movement keys scale it** — [`pull_scale`]: the
        //   lateral ramps it down toward `drive_steer_pull_fraction` („so stark wie man zur
        //   seite geht desto weniger"), `S` cancels it outright („AUßER man drückt S dann nur
        //   zur seite"). The scale rides [`WinchTuning::scaled`], so the ceiling shrinks with
        //   the speed and a scaled pull can never haul harder than ITS standing start. This is
        //   the ONE place the player's input reaches the pull; the winch itself stays
        //   input-blind and look-blind (rule 1 — removing the look's grip on the pull is what
        //   closed `FIND-196`).
        let winch = match model {
            RopeForceModel::Drive if anchored > 0 && (in_the_air || pulled) => {
                let scale = pull_scale(
                    intent.move_x,
                    intent.move_y,
                    data.game.vector.drive_steer_pull_fraction,
                );
                rope_winch(&to_anchors_m[..anchored], velocity.0, idle_winch.scaled(scale))
            }
            _ => Vec3::ZERO,
        };

        // Two terms, added, never chosen — `vector::boost::gas_boost`'s rule for the boost and
        // the dash, for its reason: holding `Ctrl` through a drive is a thing a player does in
        // his first minute, and either winning over the other would be a rule nobody can see.
        // Under `Pendulum` the second term is `Vec3::ZERO` by construction, so that model comes
        // out of this function bit for bit as it did before the winch existed.
        let wanted = flight + winch;

        drive.set_if_neq(RunAccel(wanted));

        if let Some(mut forces) = forces {
            // avian skips a zero vector itself (`query_data.rs:483`), so the `ZERO` case costs
            // nothing and needs no branch of its own here.
            forces.apply_linear_acceleration(wanted);
        }
    }
}
