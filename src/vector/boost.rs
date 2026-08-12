//! `F-007` Gas boost — an acceleration along **look direction blended with rope direction**,
//! for as long as the tank pays.
//!
//! ## Why the rope steers the boost at all (user, 2026-08-10)
//!
//! After playing it he asked for this in so many words: *„wenn man boostet soll man in richtung
//! seil und mauszeiger fliegen! also dahin. dass wenn man zur hook schaut und gehookt ist und
//! boostet man stark in die richtung fliegt!"* Up to that day the boost was **pure look
//! direction** and the rope had no influence on it whatever.
//!
//! Taken literally — thrust straight at the anchor — it would be wrong, and this is the one
//! thing worth understanding before touching [`boost_direction`]: a thrust along the rope is
//! **radial**, a taut rope absorbs exactly the radial component, and radial thrust therefore
//! adds **no tangential speed**. It winches you in and kills the swing. So neither end is right,
//! and the answer is a blend whose weight is a number in the file
//! (`game.ron: vector.boost_rope_fraction`, ⚠️ UNTUNED).
//!
//! **What the rope half buys, and it is the argument for the whole feature:** since `B-005` the
//! enforced rope length ratchets down to the true distance every substep
//! (`player::rope::shorten_ropes`), so flying *at* your anchor permanently **shortens** the
//! rope. And a shorter rope is what lifts the bottom of the arc off the ground — measured in
//! `docs/BUGS.md` `B-005`: 53.3 m of rope shortened to 20.2 m moved the bottom of the arc from
//! 18.3 m **underground** to 14.8 m **above** it. Boost, shorten, swing higher, boost again:
//! that loop is what turns a dead leash into a usable swing, and a look-only boost cannot reach
//! it. Whoever sets `boost_rope_fraction` back to `0.0` is switching that loop off, not just
//! changing a feel.
//!
//! ## It is an acceleration, not a force — and that is a decision about the RON file
//!
//! avian offers both. A force would drag the **mass** into every game value: the player's
//! capsule (`radius_m` 0.35, `height_m` 1.8) has a `ComputedMass` of **0.6029 kg** at avian's
//! default density (measured `[offlinebot]`), so a "boost force" of 20 would mean 33 m/s²
//! today and something else the day somebody gives the player a density. `boost_m_s2` in
//! `game.ron` is a number you can check against gravity in your head — 34 against 20 means
//! "the boost beats gravity by 70 %" on every machine, at every mass. So:
//! [`apply_linear_acceleration`], documented in so many words as *"ignoring mass"*
//! (`avian3d-0.7.0/src/dynamics/rigid_body/forces/query_data.rs:475-487`). Measured against
//! the alternative: with `apply_force` a 10 kg player reaches −7.68 m/s where a 0.6 kg one
//! reaches −112.79; with the acceleration both reach **−68.002785, bit for bit**
//! (`tests/vector_boost.rs`).
//!
//! ## Who owns the field this writes into
//!
//! ⚠️ One thing to know before reading the authority table: the [`Forces`] **query data**
//! declares `Write<LinearVelocity>` (`query_data.rs:105-121`), so on paper this system holds
//! mutable access to it. It never writes it — an acceleration goes nowhere near the velocity —
//! but the scheduler counts that access, and `player::locomotion::ground_locomotion` is the
//! one that really does assign it. The two never overlap: `Drive` runs before `Integrate`.
//!
//! [`Forces`] does **not** touch `LinearVelocity` for an acceleration. It adds into
//! `VelocityIntegrationData::linear_increment` (`.../integrator/mod.rs:235-239`), which is an
//! **accumulator**: gravity lands in the same field (`.../integrator/mod.rs:297-298`), it is
//! multiplied by the substep delta once per step, and avian clears it itself after the substep
//! loop (`clear_velocity_increments`, `.../integrator/mod.rs:316-327`). **Nothing is reset by
//! hand here** — a hand-written reset would fight the engine for the one field it already owns.
//!
//! That is also why this system may sit in `SimulationSystems::Drive` next to
//! `reel::reel_in` without a `.chain()`: two contributors adding into an accumulator commute,
//! and the one field each of them *assigns* ([`BoostAccel`] here) is its own.
//!
//! ## Why the gas decides, and the button does not
//!
//! Read here is [`GasGrant`], and **only** that. `Gas::try_spend` is never called from this
//! file — `vector::gas` (`F-018`) books once per tick and publishes the result, so that a
//! nearly empty tank does not let the system order decide who pays (`shared::gear`,
//! `docs/QUESTIONS.md` Q-017).
//!
//! It would be tempting to also ask `Intent::pressed(Buttons::BOOST)` here. That is exactly
//! what this file does not do: **the debit and the thrust have to be one decision.** If the
//! grant said yes and this system said no, the player would have paid gas for nothing — and a
//! leak you cannot see is worse than one you can. [`Intent`] is read for the **direction**
//! only. `vector::gas` states the other half of the same contract in its own header:
//! `GasGrant.boost == true` means the button is held **and** this tick's gas is paid.
//!
//! Without a grant the drive holds `Vec3::ZERO` and the acceleration is exactly zero, not a
//! fraction (`F-018`: at 0 there is no more flying).
//!
//! `F-006` Swerve landed in `player::locomotion` as its own component ([`RunAccel`]) — the
//! note that once stood here is stale for it. **`F-008` Dodge did dock on**, and it is one
//! system less than that note promised: it is a second term inside [`gas_boost`], not a system
//! of its own, because [`BoostAccel`] has **one** writer and two systems assigning one
//! component is the rule-4 violation §4 is about.
//!
//! ## The second boost — `F-008`, the user's words, 2026-08-12 (`docs/NEXT.md` §1c)
//!
//! > *„mit doppel leertaste boostet man stark in die lauf richtung (ein weiter dodge) der viel
//! > gas aufbraucht. das andere boosten verbraucht sehr wenig!"*
//!
//! Two boosts that differ in **three** ways at once, and every one of them is deliberate:
//!
//! | | `Shift` — the cheap one | double-tap `Space` — the dodge |
//! |---|---|---|
//! | billed | a rate, per second, while held | a flat amount, once per double-tap |
//! | direction | look, blended toward the rope | **the movement input** (`W`/`A`/`D`) |
//! | shape | an acceleration you steer for seconds | one tick's velocity change |
//!
//! **The direction is the whole point of the feature.** *„in die lauf richtung"* — not where
//! the camera points. In a fast swing you are looking down at the street you are about to
//! cross, and a dodge along the look vector would put you into it. `A`/`D` are how you leave
//! the plane of the rope (`docs/NEXT.md` §1a: *„das a d sorgt dafür dass man nicht immer direkt
//! zum seil gezogen wird"*), and the dodge is that same steering spent in one tick.
//! [`dodge_direction`] is the rule and it is a free function, so it can be checked without an
//! app — and so that `vector::gas` can ask the **same** question this file answers.
//!
//! **Why the direction rule lives here twice over.** It is deliberately the same shape as
//! `player::locomotion::air_thrust`, and it is deliberately **not** a call into it: `vector` may
//! not `use crate::player` (`docs/architecture.md`, allow list), and the edge is not worth
//! buying for four lines of trigonometry. The duplication is knowing, and it is a finding
//! (`docs/FINDINGS.md` FIND-067) rather than a thing pretended not to exist — the fix, when
//! somebody wants it, is one helper in `shared::math`, which no domain has to ask permission
//! for.
//!
//! ## Why it is an *impulse* and what the file therefore holds
//!
//! `vector.dodge_impulse_m_s` is **m/s** — a velocity change — and this file divides it by the
//! fixed timestep to get the acceleration avian is handed. That is not a detour, it is the only
//! way the number in the file means the same thing at another tick rate: avian multiplies
//! `linear_increment` by the substep delta once (`integrator/mod.rs:308-309`) and then adds it
//! in **every** substep (`IntegrationSystems::Velocity` runs in the `SubstepSchedule`), so an
//! acceleration written once per tick is worth exactly `accel * fixed_dt` of velocity. Dividing
//! by `fixed_dt` here makes that product the number out of the file, bit for bit.
//!
//! ⚠️ **The consequence is a genuinely large acceleration for one tick** — 24 m/s at 60 Hz is
//! 1440 m/s², forty-two times [`boost_m_s2`]-sized. That is what "impulse" means and it is not
//! a bug; what it does mean is that anything which ever comes to read [`BoostAccel`] as "how
//! hard is he boosting" must not be surprised by one tick in seven hundred that is two orders
//! of magnitude larger. Nothing reads it today except the tests.
//!
//! ## Where the dodge's gas is booked, and why not here
//!
//! In `vector::gas`, with the other two, through `GasConsumer::Dodge` — same contract, same
//! reason: **the debit and the thrust have to be one decision.** That has one sharp consequence
//! for [`dodge_direction`]: the "no movement input, no dodge" rule cannot live only in this
//! file. If it did, `gas_budget` would bill the full flat cost for a double-tap with no `W`,
//! and this system would write `ZERO` — a player paying 15 % of his tank for nothing, which is
//! precisely the invisible leak the header above forbids. So `gas_budget` asks
//! [`dodge_direction`] the same question before it spends, exactly as it asks
//! `hook.anchored_count()` before it bills a reel-in. **One rule, one function, two callers.**
//!
//! Seen: `scripts/f-007-boost.txt` · `docs/images/f-007-boost.png` ·
//! measured in `tests/vector_boost.rs`.
//!
//! [`RunAccel`]: crate::shared::RunAccel
//! [`boost_m_s2`]: crate::data::VectorTuning::boost_m_s2

use avian3d::prelude::{Forces, WriteRigidBodyForces};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::math::direction;
use crate::shared::{BoostAccel, GasGrant, Hook, Intent, Side};

/// The direction the ropes pull the boost toward: the **mean of the unit directions** from the
/// hand to every anchored tip. `None` means "no rope has a say" — nothing anchored, the player
/// standing exactly on his anchor, or two ropes pointing exactly opposite each other.
///
/// **The mean of the directions, not the mean of the anchor points.** A 42 m rope and a 4 m rope
/// steer equally; taking the midpoint of the two anchors instead would let the far one outvote
/// the near one by its distance, which is a property of where you happened to hook and not of
/// how you are hanging.
///
/// A free function, like `hook::anchor_target`, so the rule can be tested without an app.
pub fn rope_dir(hand_m: Vec3, hook: &Hook) -> Option<Vec3> {
    let mut sum = Vec3::ZERO;
    for side in Side::ALL {
        let arm = hook.arm(side);
        if arm.state.is_anchored() {
            // `direction` is `None` for the zero vector and for anything non-finite, so an
            // anchor the player is standing exactly on contributes nothing instead of a NaN.
            sum += direction(arm.tip_m - hand_m).unwrap_or(Vec3::ZERO);
        }
    }
    // Two exactly opposed ropes sum to zero, and so does an idle gear: both are `None`, and
    // both mean the same thing — the rope cannot say where "toward the rope" is.
    direction(sum)
}

/// Where a boost pushes: `look_dir` blended `rope_fraction` of the way toward `rope_dir`, then
/// **renormalized**.
///
/// The renormalize is the whole arithmetic of this function. `lerp` between two unit vectors is
/// not a unit vector — at 90° apart and `0.5` it is 0.707 long — so without it
/// `vector.boost_m_s2` would quietly become a number that depends on where the player is
/// looking. **The blend decides the direction and never the strength**
/// (`tests/vector_boost.rs`).
///
/// Two ways out, and both of them are the look direction:
/// - **no rope** (`rope_dir` is `None`) — unhooked, the gear steers nothing;
/// - **the blend cancels** — `look_dir` exactly opposite `rope_dir` at `0.5` gives the zero
///   vector, and `normalize(ZERO)` is NaN. That is not a theoretical case: "hooked behind you,
///   boosting forward" happens in every swing. A NaN here becomes a NaN `Transform`, and a NaN
///   `Transform` is how a player vanishes from the world (§9d).
///
/// `rope_fraction == 0.0` returns before touching the arithmetic at all, and that is deliberate:
/// `look_dir` is a unit vector only to about `1e-7`, so `direction(lerp(look, rope, 0.0))` would
/// come back a few bits away from `look_dir`. `0.0` in the file has to mean **the behaviour we
/// had**, bit for bit, or the knob is a rewrite instead of a knob.
pub fn boost_direction(look_dir: Vec3, rope_dir: Option<Vec3>, rope_fraction: f32) -> Vec3 {
    // Clamped, because outside `0..=1` this stops being a blend: at `2.0` and 90° apart the
    // extrapolation points *away* from where the player is looking, which is no reading of what
    // was asked for. The RON value is not range-checked anywhere yet (finding).
    let w = rope_fraction.clamp(0.0, 1.0);
    let Some(rope) = rope_dir.filter(|_| w > 0.0) else {
        return look_dir;
    };
    direction(look_dir.lerp(rope, w)).unwrap_or(look_dir)
}

/// Where a dodge throws you (`F-008`): the **movement input**, as a unit vector, or `None`
/// when there is no movement input at all.
///
/// `None` is not an error and not a fallback — it is the answer *"then there is no dodge"*, and
/// `vector::gas` reads it as *"then it costs nothing"*. The alternatives were both worse: a
/// dodge along the look direction is the one thing the user's sentence rules out, and a dodge
/// straight forward off the yaw would fire an expensive, uncontrolled lunge out of a button a
/// player pressed while standing still.
///
/// The three keys, and each of the three is `docs/NEXT.md` §1a rather than a choice made here:
///
/// - **`W` goes where you look**, with the pitch in it — the dodge climbs and dives with the
///   camera, which is the only way a dodge can leave a street it is flying down.
/// - **`A`/`D` are horizontal**, off the yaw and never off the pitch. A lateral dodge that
///   tilted with the pitch would drive into the ground in exactly the situation it is for.
/// - **`S` is not a direction.** *„mit s »spannt« man nur das seil"* — hence `.max(0.0)`, and
///   hence `S` alone yields `None` and no dodge at all, rather than a backwards one.
///
/// `direction` (and not `normalize`) is what makes the zero case a `None` instead of a NaN, and
/// the whole rest of this file exists to keep NaN out of a `Transform` (§9d).
///
/// **Deliberately the same shape as `player::locomotion::air_thrust`** — the dodge has to go
/// where WASD was already pushing, or it is a second control scheme. It is copied and not
/// called for the reason in the header: `vector -> player` is not on the allow list.
/// The one difference is the last step: `air_thrust` uses `clamp_length_max(1.0)`, so a
/// half-pressed axis thrusts half; here the length is normalised away, because the **strength**
/// of a dodge is `vector.dodge_impulse_m_s` and nothing else. `W`+`D` is one dodge at 45°, not
/// 1.41 of them and not 0.71 of one.
pub fn dodge_direction(intent: &Intent) -> Option<Vec3> {
    let (sin, cos) = intent.yaw.sin_cos();
    let right = Vec3::new(cos, 0.0, -sin);
    direction(intent.look_dir() * intent.move_y.max(0.0) + right * intent.move_x)
}

/// Writes [`BoostAccel`] = [`boost_direction`] * `vector.boost_m_s2`, or `ZERO`, and hands the
/// same vector to avian as a linear acceleration.
///
/// **Sole writer of [`BoostAccel`].** Contributor — never sole writer — of
/// `VelocityIntegrationData::linear_increment`, which belongs to avian.
///
/// [`Hook`] and [`Transform`] are read, never written: the hook state belongs to
/// `vector::hook`, the transform to the integrator. `hook::update_hooks` runs in
/// `SimulationSystems::Intent` and this system in `Drive`, so the `tip_m` read here is **this**
/// tick's anchor position and not last tick's.
///
/// `Option<Forces>` and not a plain `Forces`: the physics components arrive with avian's own
/// prepare step, and a player in his very first tick has none of them yet. With a plain
/// `Forces` the whole row would drop out of the query and [`BoostAccel`] would silently keep
/// the value of the tick before — the one thing its "written every tick, even when it is zero"
/// contract exists to prevent (`shared::gear`).
pub fn gas_boost(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    mut players: Query<(&Intent, &GasGrant, &Hook, &Transform, &mut BoostAccel, Option<Forces>)>,
) {
    let strength_m_s2 = data.game.vector.boost_m_s2;
    let rope_fraction = data.game.vector.boost_rope_fraction;
    let eye_height_m = data.game.player.eye_height_m;
    // `Time<Fixed>` and not `1.0 / simulation_hz`, for `vector::gas::gas_budget`'s reason: the
    // timestep is set from that very number in `src/lib.rs`, and a value derived twice is a
    // value that drifts once. Dividing by it is what turns the m/s in the file into the
    // acceleration avian integrates back into exactly that m/s (see the header).
    let dt = time.delta_secs();
    let dodge_m_s2 = if dt > 0.0 { data.game.vector.dodge_impulse_m_s / dt } else { 0.0 };

    for (intent, grant, hook, transform, mut drive, forces) in &mut players {
        // The hand is the eye — the same point `vector::hook` flies the tip from and to, and
        // the same one `render::rope` draws the rope from. Anything else would make the
        // direction the player is given differ from the rope he can see.
        let hand_m = transform.translation + Vec3::Y * eye_height_m;
        // `look_dir()` is a unit vector by construction (`shared::intent`, checked there over
        // the whole yaw/pitch range); `boost_direction` keeps it one and never returns a NaN,
        // which is what the two degenerate cases in its doc comment are about (§9d).
        let dir = boost_direction(intent.look_dir(), rope_dir(hand_m, hook), rope_fraction);
        let held = if grant.boost { dir * strength_m_s2 } else { Vec3::ZERO };
        // `grant.dodge` is already "the double-tap landed **and** the flat cost is paid" — the
        // same contract `grant.boost` carries, so there is no second condition here either.
        // `dodge_direction` is asked again rather than remembered: `gas_budget` only spends
        // when it answers `Some`, so on a granted tick this branch cannot be `None` — and
        // `unwrap_or(ZERO)` is the honest way to say that without a panic in the simulation.
        let dodge = if grant.dodge {
            dodge_direction(intent).unwrap_or(Vec3::ZERO) * dodge_m_s2
        } else {
            Vec3::ZERO
        };
        // **Summed, not chosen.** Holding `Shift` through a dodge is a thing a player will do
        // in his first minute, and either of the two winning over the other would be a rule
        // nobody can see. Two accelerations add; that is all that happens.
        let wanted = held + dodge;

        // `set_if_neq`: a component that reports itself changed on all sixty ticks makes every
        // `Changed<BoostAccel>` filter behind it worthless — and a player who is not boosting
        // really does not change.
        drive.set_if_neq(BoostAccel(wanted));

        if let Some(mut forces) = forces {
            // avian itself skips a zero vector (`query_data.rs:483`), so the `ZERO` case costs
            // nothing and needs no branch of its own here.
            forces.apply_linear_acceleration(wanted);
        }
    }
}
