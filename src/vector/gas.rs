//! `F-018` The gas budget — **the only place that debits `Gas`.**
//!
//! Without this detour `F-007` (boost) and `F-005` (reel-in) would both call
//! `Gas::try_spend`. That method is deliberately atomic and without partial spending
//! (`shared::state`), so on a tight tank the **system order** would decide who pays — the
//! coin toss at 60 Hz that `docs/architecture.md` forbids, and on the network a desync
//! nobody reproduces.
//!
//! Here it is booked **once per tick** and the result published as [`GasGrant`]. Whoever
//! reads `false` there writes zero into his drive.
//!
//! The **priority** on a tight tank lives in `assets/data/game.ron`
//! (`vector.gas_priority`), not as an `if` here: "what runs out first?" is a balancing
//! decision (`docs/QUESTIONS.md` Q-017).
//!
//! ## The contract the drive systems may rely on
//!
//! `GasGrant.boost == true` means **both** at once: the button is held *and* this tick's gas
//! has been paid. `vector::boost` therefore needs no second condition of its own — one
//! `if grant.boost` is the whole check. The same for `GasGrant.reel_in`. Were the want
//! condition to live in two places, one of them would be wrong by next week, and it would
//! be wrong as a **free** boost.
//!
//! `GasGrant.steer` is the same contract for the rope half of the mixing rule
//! (`docs/NEXT.md` §1B, `player::locomotion::rope_steer`) — and it is the one all nine judges of
//! that plan asked for independently, because without it the strongest thrust in the game would
//! have been the only free one.
//!
//! Written **every tick, for every player**, and by assignment — there is no clearing
//! system and no grant that lives one tick too long.
//!
//! ## Three decisions that are not obvious from the code
//!
//! - **Reel-in only wants gas when a hook holds.** Pressing the button in free fall pulls
//!   on nothing, so it costs nothing. The cost follows the effect, not the button.
//! - **The grant is per player, not per side.** [`GasGrant`] carries one `reel_in`, and
//!   `game.ron` carries one `gas_reel_per_s` — so two taut ropes reeled in at once cost
//!   exactly as much as one. That is a game-value question, not a mechanism, and it is
//!   listed in the report of this job rather than answered here.
//! - **The `Hook` read here is one tick old.** `vector/mod.rs` chains `gas_budget` **before**
//!   `hook::update_hooks` on purpose. A hook that anchors in this tick therefore starts
//!   costing gas in the next one — one tick of delay in exchange for an order that does not
//!   depend on which system Bevy happens to run first.
//!
//! ## The budget only ever subtracts; gas comes back only when a station asks (Q-033)
//!
//! **[`gas_budget`] has no refill in it, and its absence is a decision.** The user answered it
//! on 2026-08-12: *„gas refillt nur im main gebäude an bestimmten stationen/objekten"* — gas
//! comes back **at a place you go to**, never on a timer.
//!
//! That place is `mission::hub`'s refuel station, and since the same day it **asks**:
//! [`apply_refuel_requests`] takes a `RefuelRequest` and is the only thing in the game that
//! ever raises a tank. It sits in this file because `Gas` has **one** writer and it is this
//! file — a station that called `Gas::refill` itself was the rule-4 violation this repair took
//! back out (`docs/FINDINGS.md` FIND-063). Nothing about the answer to Q-033 changes with it:
//! no rate, no timer, no idle branch — a tank rises only while somebody stands in a station.
//!
//! Between 2026-08-10 and then this file did regenerate the tank while nobody was spending,
//! after a `gas_regen_delay_s` pause. That was an assumption made under the autonomous rule
//! while the question was open, and the user picked none of the three shapes it offered. The
//! whole mechanism came back out: `refill_tank`, `arm_pause`, `Gas::regen_delay_left_s` and
//! both RON keys. **Do not put a rate back in when the tank feels tight** — the answer is the
//! stations (queued in `docs/NEXT.md` §1d), and the reason is in the bible: burning gas is
//! loud and a Bellower answers it, so the resource is coupled to *risk*, which a tank that
//! quietly fills itself while you hang around is not.
//!
//! How long one held boost lasts is therefore `gas_tank / gas_boost_per_s` and nothing else —
//! **833.3 s at the numbers of 2026-08-20** (it was 16.67 s while `gas_tank` was 300). The tank
//! is what got bigger for that, three times now: 100 -> 300 -> 15000, each time because the user
//! said so after playing. The rule is the division; the seconds are whatever the file currently
//! says. See `assets/data/game.ron: vector.gas_tank` and `docs/QUESTIONS.md` Q-046.
//!
//! `Changed<Gas>` stays a signal because of it. A tick in which nobody wants gas writes
//! nothing at all to the tank, so the HUD is not woken sixty times a second by a number that
//! did not move (`tests/vector_gas.rs::f018_an_idle_tank_never_refills_on_its_own`).
//!
//! Cost, evidence and image of this file: `tests/vector_gas.rs`, `scripts/f-018-gas.txt`,
//! `docs/images/f-018-gas.png`.

use bevy::prelude::*;

use crate::data::{GameData, GasConsumer};
use crate::shared::{
    Buttons, DodgeCharges, Gas, GasGrant, Hook, Intent, MovementState, PlayerId, RefuelRequest,
    Submerged, Tick,
};

/// Puts gas back — **the only thing in the game that ever raises a tank** (Q-033).
///
/// The gas comes back at a place you walk to, and the place is `mission::hub`'s refuel station.
/// But `Gas` has one writer and it is this file (`docs/architecture.md`, authority table), so
/// the station **asks** with a [`RefuelRequest`] and this applies it. Until 2026-08-12 the
/// station called `Gas::refill` itself; that was a second writer on one field, disjoint from
/// this one only *by phase*, and "disjoint by phase" is the argument that stops being true over
/// a wire (`docs/FINDINGS.md` FIND-063).
///
/// Three properties this has to keep, and each one is a test:
///
/// - **A request names a player and fills that player's tank.** Not `LocalPlayer`, not
///   `.single()` — `docs/multiplayer.md` rule 2. Two players in one station are two requests.
/// - **It never raises a tank above `Gas::max`**: `Gas::refill` caps, and a station that keeps
///   asking after the tank is full changes nothing.
/// - **`set_if_neq`, exactly like [`gas_budget`]**: a full tank in a station must not report
///   `Changed<Gas>` sixty times a second for a number that did not move (§6 rule 6).
///
/// **One tick late, deliberately.** The station sees the player in `PostStep` and this runs in
/// the next tick's `Intent`. Ordering it into the same tick would mean a `vector` system
/// ordered against a `mission` system — a hidden edge past the allow list — and the whole point
/// of the repair was not to buy an edge. It is the same trade the `Hook` read above makes, and
/// at `gear.ron: resupply.gas_per_s` of 40 one tick is 0.67 gas.
pub fn apply_refuel_requests(
    mut requests: MessageReader<RefuelRequest>,
    mut players: Query<(&PlayerId, &mut Gas)>,
) {
    for request in requests.read() {
        for (id, mut gas) in &mut players {
            if *id != request.player {
                continue;
            }
            let mut tank = *gas;
            tank.refill(request.amount);
            gas.set_if_neq(tank);
        }
    }
}

/// **Does the rope steer this tick move the player at all?** The `Steer` half of the budget's
/// want condition, as a pure function so that a test can hold it against the thrust it is
/// supposed to be paying for.
///
/// The arguments are `player::locomotion::rope_steer`'s own, minus the two magnitudes and the
/// yaw — because the question is whether that function returns `Vec3::ZERO`, and neither
/// `pull_m_s2` nor the yaw can decide that.
///
/// `to_anchors_m` is one `tip − hand` per **anchored** arm, unnormalised, exactly as
/// `rope_steer` takes it.
pub fn steer_has_effect(
    to_anchors_m: &[Vec3],
    look_dir: Vec3,
    move_x: f32,
    move_y: f32,
    min_rope_m: f32,
    fade_m: f32,
) -> bool {
    if to_anchors_m.is_empty() {
        return false;
    }
    // `A`/`D` are the player's own thrust across the rope and nothing scales them down, so a
    // lateral key on an anchored rope always moves him and always pays. This is the half of the
    // steer that was never in doubt.
    if move_x != 0.0 {
        return true;
    }
    // `S` is not a haul (`docs/NEXT.md` §1A requirement 7) — the same `.max(0.0)` as the thrust.
    if move_y.max(0.0) <= 0.0 {
        return false;
    }
    // And the half that was: **the pull is `max(0, l̂ · r̂)` times `clamp((L − min)/fade)`, and
    // both of those are zero over most of a swing.** A player hangs *under* his anchor and looks
    // where he is going, so `l̂ · r̂` is negative and the pull is exactly `Vec3::ZERO` — measured
    // over `scripts/f018-budget.txt`: mean delivered pull 0.0012 of `air_pull_m_s2` across 99
    // sampled steer ticks, while 16/s was charged for every one of them. 48.3 % of the tank.
    to_anchors_m.iter().any(|to_anchor| {
        let Some(direction) = to_anchor.try_normalize() else {
            return false;
        };
        look_dir.dot(direction) > 0.0 && to_anchor.length() > min_rope_m && fade_m > 0.0
    })
}

/// **Diagnostics only, and off unless `DBT_GAS_LEDGER=1` stands in the environment.**
///
/// The tank is one number, so a player who says *„gas ist VIEL zu schnell weg"* cannot be
/// answered from it: 300 gone tells you nothing about WHICH verb spent it. This splits the
/// same debit four ways as it happens — the amount, and how many ticks each consumer *wanted*
/// versus how many it was *granted* — so a sortie can be read as a ledger instead of a slope.
///
/// It is not a game value and it is not state anybody may read: nothing in the simulation
/// looks at it, it lives in a `Local` of [`gas_budget`], and with the variable unset the whole
/// thing is four adds and no output. Kept because the next tuning round needs the same number
/// (`docs/FINDINGS.md` FIND-139).
#[derive(Default)]
pub struct Ledger {
    tick: u64,
    /// Gas actually debited, per consumer, in the order
    /// `[boost, steer, reel_in, dodge, flip]`.
    spent: [f32; 5],
    /// Ticks in which the consumer *wanted* gas.
    wanted: [u32; 5],
    /// Ticks in which it was *granted* — below `wanted` only on a tank that ran short.
    granted: [u32; 5],
    /// What really left the tank, summed straight off `Gas::current`. The four `spent` entries
    /// have to add up to this; if they do not, something bills gas that this file does not see.
    debited: f32,
}

/// Is the ledger switched on? Read once, not sixty times a second.
fn ledger_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("DBT_GAS_LEDGER").is_ok_and(|v| v != "0"))
}

/// Debits this tick's gas and writes [`GasGrant`].
///
/// The **only** writer of `Gas` in the simulation (`docs/architecture.md`, authority table).
pub fn gas_budget(
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    mut ledger: Local<Ledger>,
    tick: Res<Tick>,
    // `Option` on both of the new ones, and that is not laziness — it is what keeps this
    // system's contract the same for a fixture as for a real player. A `&DodgeCharges` in the
    // filter would drop every test player that does not carry one **out of the query
    // entirely**, and a player who is not billed is a player who never runs out of gas: the
    // exact shape of `FIND-152`, where a whole-app test never reached the code it was testing.
    // `None` therefore means "no magazine to ask" and behaves as it did before `F-008` — and
    // `player::spawn_player_with_id` gives every real body both components from tick 1.
    mut players: Query<(
        &Intent,
        &Hook,
        &mut Gas,
        &mut GasGrant,
        &Transform,
        Option<&DodgeCharges>,
        Option<&MovementState>,
        // `Option` for the same reason as the two above it: a fixture player that carries no
        // `Submerged` must still be billed, and `None` means dry. Every real body gets one at
        // spawn (`player::spawn_player_with_id`).
        Option<&Submerged>,
    )>,
) {
    let vector = &data.game.vector;
    // `Time<Fixed>` and not `1.0 / simulation_hz`: the timestep is set from the very same
    // number (`src/lib.rs`), and a value derived twice is a value that drifts once.
    let dt = time.delta_secs();
    // Per **tick**, not per second. `gas_boost_per_s` is 18/s; at 60 Hz that is 0.3 per tick,
    // and 60 ticks of it are exactly the 18 from the file. Multiplying by `dt` a second time
    // somewhere further along would cost 0.005/s — nothing anybody notices while playing, and
    // `tests/vector_gas.rs` goes red on it.
    let boost_cost = vector.gas_boost_per_s * dt;
    let reel_cost = vector.gas_reel_per_s * dt;
    // A rate like the two above, and billed per tick like them: 16/s is 0.2667 per tick at
    // 60 Hz. It buys `player.air_pull_m_s2` / `air_lateral_m_s2` of thrust in
    // `player::locomotion::air_control`, and the price is the boost's own price per m/s of
    // speed bought — 16/30 against 18/34 (`assets/data/game.ron`, `tests/data.rs` holds it).
    let steer_cost = vector.gas_steer_per_s * dt;
    // **Not multiplied by `dt`, and that is the point of `F-008`.** The other two are rates and
    // are billed per tick; a dodge is one impulse and is billed once, on the tick the double-tap
    // lands. `gas_dodge` therefore has no `_per_s` in its name and must not grow one.
    let dodge_cost = vector.gas_dodge;
    // Seconds in the file, ticks in the code (`docs/conventions.md`). `round`, not `as u64`:
    // 0.6 s at 60 Hz is 36.000004 in f32 and truncating would make the cooldown 36 ticks on one
    // machine and 35 on the next.
    let cooldown_ticks = (vector.dodge_cooldown_s as f64 * data.game.simulation_hz).round() as u64;
    // `F-009`, flat like the dodge and for the identical reason: it is an impulse, not a rate,
    // so it must never be multiplied by `dt` and must never grow a `_per_s` in its name.
    let flip_cost = vector.gas_flip;

    for (intent, hook, mut gas, mut grant, transform, charges, state, submerged) in &mut players {
        // **The hand and the two rope directions, once**, for the three verbs below that all
        // need them. Hoisted above `wants_reel_in` on 2026-08-25 — it used to sit further down,
        // next to `wants_steer`, and the reel could not see it.
        let hand_m = transform.translation + Vec3::Y * data.game.player.eye_height_m;
        let mut to_anchors_m = [Vec3::ZERO; 2];
        let mut anchored = 0;
        for arm in &hook.arms {
            if arm.state.is_anchored() {
                to_anchors_m[anchored] = arm.tip_m - hand_m;
                anchored += 1;
            }
        }

        let wants_boost = intent.pressed(Buttons::BOOST);
        // **The cost follows the effect, and `anchored_count() > 0` was not enough.** A reel
        // whose rope is already at the floor moves nobody: under `Pendulum`
        // `player::rope::shorten_ropes` clamps `limits.max` at `vector.min_rope_m` and stops,
        // under `Drive` `player::locomotion::rope_winch` drops that arm out of its blend at the
        // very same number. Both did it silently while this line kept billing
        // `gas_reel_per_s` — the same shape as `FIND-139`'s steer, one verb further along
        // (`Q-050`). The distance is the arm's own, so **one arm at the floor and one out at
        // 40 m still pays**, which is right: the far one really is winding in.
        let wants_reel_in = intent.pressed(Buttons::REEL_IN)
            && to_anchors_m[..anchored].iter().any(|to_anchor| {
                to_anchor.length() > vector.min_rope_m
            });
        // The same shape as the line above it, and for the same reason: **the cost follows the
        // effect, not the button.** A double-tap with no movement key held has no direction to
        // throw the player in (`boost::dodge_direction` answers `None`), so `vector::boost`
        // would write zero — and billing 15 % of a tank for zero thrust is the invisible leak
        // that whole detour exists to prevent. One rule, one function, two callers.
        // 🔴 **`F-008`'s magazine, and the gate sits IN FRONT OF THE MONEY.** Q-046 measured
        // that the price stopped bounding the dash the day the tank went 300 -> 15000 (333
        // dashes per sortie), so what bounds it now is `DodgeCharges` — and asking it *here*,
        // in the same expression as the direction, is what makes a refused dash cost nothing
        // at all. Booking first and refusing later would debit 45 gas for an impulse that never
        // happens, which is precisely the invisible leak the `dodge_direction` clause above was
        // added to prevent. One rule, one place.
        // 🔴 **ONE predicate, and since 2026-08-25 BOTH air verbs ask it** (`FIND-`, below).
        // *„auf dem Boden ist die gleiche Ausweichbewegung `F-010`s Slide"* is the rule the
        // flip already obeyed and the dash did not, and the hole was not theoretical: measured
        // on `scripts/game-full.txt`'s map, **one press of `C` while running fired the slide
        // AND the dash** — gas 15000 -> 14955, a charge gone, and the slide's promised
        // `max(current, 12)` delivered **38.166 m/s**, because `vector::boost::gas_boost` adds
        // `dodge_impulse_m_s` on the same tick, on top of the velocity
        // `player::locomotion::ground_locomotion` had just written. One button, two moves, two
        // prices — and `game.ron` says in writing that the ground one is free.
        //
        // **Exhaustive, and no longer `!= Grounded`.** `OnWall` is `F-013`'s state and `Downed`
        // is *„out of the fight instead of dead"* — neither is flying, and a body that may not
        // walk may certainly not flip. That is `player::locomotion::in_flight`'s own list of
        // what is never flight, arrived at from the other side; it cannot be *called* from here
        // because `vector -> player` is not on the allow list of `docs/architecture.md`, so a
        // `match` that a new `MovementState` variant makes the compiler complain about is the
        // next best thing.
        //
        // `None` still counts as air, unchanged: a fixture with no `MovementState` is not
        // standing anywhere, and refusing both verbs would make them untestable without a floor.
        let in_the_air = state
            .is_none_or(|s| matches!(s, MovementState::Airborne | MovementState::Tethered));
        let can_dash =
            charges.is_none_or(|c| c.ready(tick.0, cooldown_ticks));
        let wants_dodge = intent.pressed(Buttons::DODGE)
            && in_the_air
            && can_dash
            && super::boost::dodge_direction(intent).is_some();
        // **`F-009`, and it is an AIR move.** *„Doppeltipp A/D erzeugt seitlichen
        // Ausweichsprung **in der Luft**"* — on the ground the same evasion is `F-010`'s
        // slide, which is a different move with a different cost (none). Without this clause
        // a player standing in the street would pay `gas_flip` for a sideways hop, and the
        // ground would have two evasive verbs that do the same thing at two prices.
        //
        // The same `in_the_air` the dash asks, and the reason it is one binding and not two
        // copies: two copies drift, and the drift is what let the dash fire on the ground for a
        // day while this line refused.
        let wants_flip = intent.pressed(Buttons::FLIP) && in_the_air && intent.move_x != 0.0;
        // **`docs/NEXT.md` §1B, and the same rule a third time: the cost follows the effect.**
        // The rope term of the mixing rule is `n > 0 && (w⁺ > 0 || mx ≠ 0)` — no anchored hook
        // means there is no rope direction to push along and no lateral boost to add, and no
        // movement key means both halves of it are multiplied by zero. `S` alone is `w⁺ = 0`
        // (*„mit s »spannt« man nur das seil"*) and buys nothing, so it pays nothing.
        // **The one that was a bill and not a price** (`docs/FINDINGS.md` FIND-139). Until
        // 2026-08-20 this line read `anchored_count() > 0 && (move_y.max(0.0) > 0.0 ||
        // move_x != 0.0)` — the BUTTON — and `player::locomotion::rope_steer` then delivered
        // `Vec3::ZERO` for most of every swing, because its pull carries a `max(0, l̂ · r̂)` and
        // a player hangs *under* his anchor while looking where he is going. Measured over
        // `scripts/f018-budget.txt`: 144.8 of 300 gas — **48.3 % of the tank, the largest line
        // item in the game** — bought a mean thrust of 0.0012 of `air_pull_m_s2`.
        //
        // [`steer_has_effect`] is that condition read off the geometry instead, and
        // `tests/vector_gas.rs::f006_the_steer_is_billed_exactly_when_the_rope_really_thrusts`
        // holds the two against each other over 750 geometries, so the copy here cannot drift
        // away from the formula it is paying for.
        let wants_steer = steer_has_effect(
            &to_anchors_m[..anchored],
            intent.look_dir(),
            intent.move_x,
            intent.move_y,
            vector.min_rope_m,
            data.game.player.air_pull_fade_m,
        );

        if !wants_boost && !wants_reel_in && !wants_dodge && !wants_steer && !wants_flip {
            // Nobody wants anything, so **the tank is not touched at all** — not even to
            // write the same number back. `Changed<Gas>` is a signal the HUD and one day the
            // wire read, and a tank that reports a change every tick without changing is a
            // lie in that signal. Refilling belongs to the stations, not here (Q-033).
            grant.set_if_neq(GasGrant::default());
            continue;
        }

        // On a copy, so that `Gas` is only marked changed when something about the tank really
        // is different — and so that [`book`] stays a plain function a test can drive without
        // an app.
        let mut tank = *gas;
        let booked = book(
            &vector.gas_priority,
            Wants {
                boost: wants_boost,
                reel_in: wants_reel_in,
                steer: wants_steer,
                dodge: wants_dodge,
                flip: wants_flip,
            },
            // The land prices, and then the water surcharge — one call, all five verbs.
            // `Submerged` is written by `player::swim` and read here; `vector` never decides a
            // second time whether the player is wet, which is the FIND-, `CLAUDE.md` rule 5
            // corollary: one writer decides, everyone else reads the answer.
            if submerged.is_some_and(|s| s.wet()) {
                Costs {
                    boost: boost_cost,
                    reel_in: reel_cost,
                    steer: steer_cost,
                    dodge: dodge_cost,
                    flip: flip_cost,
                }
                .under_water(data.water.swim.gas_cost_factor)
            } else {
                Costs {
                    boost: boost_cost,
                    reel_in: reel_cost,
                    steer: steer_cost,
                    dodge: dodge_cost,
                    flip: flip_cost,
                }
            },
            &mut tank,
        );
        if ledger_enabled() {
            let wants = [wants_boost, wants_steer, wants_reel_in, wants_dodge, wants_flip];
            let grants =
                [booked.boost, booked.steer, booked.reel_in, booked.dodge, booked.flip];
            let costs = [boost_cost, steer_cost, reel_cost, dodge_cost, flip_cost];
            for i in 0..5 {
                ledger.wanted[i] += u32::from(wants[i]);
                ledger.granted[i] += u32::from(grants[i]);
                if grants[i] {
                    ledger.spent[i] += costs[i];
                }
            }
            // The control on the four adds above, and the reason the ledger is worth reading:
            // what it claims left the tank is compared against what really left it. A consumer
            // billed twice, or billed somewhere outside this file, shows up here and nowhere
            // else.
            ledger.debited += gas.current - tank.current;
        }
        gas.set_if_neq(tank);
        grant.set_if_neq(booked);
    }

    if ledger_enabled() {
        ledger.tick += 1;
        if ledger.tick % 60 == 0 {
            let total: f32 = ledger.spent.iter().sum();
            let tank = data.game.vector.gas_tank;
            info!(
                "gas ledger t={t} spent={total:.1} of {debited:.1} debited ({pct:.2}% of tank) | boost={b:.1} steer={s:.1} reel={r:.1} dodge={d:.1} flip={f:.1} | wanted_ticks boost={wb} steer={ws} reel={wr} dodge={wd} flip={wf} | granted_ticks boost={gb} steer={gs} reel={gr} dodge={gd} flip={gf}",
                t = ledger.tick,
                debited = ledger.debited,
                // ⚠️ `.2`, not `.0`. At `gas_tank: 15000` a whole ordinary sortie spends
                // ~223 gas = 1.49 % of the tank, so `{:.0}` printed the total as "1%" and
                // rounded all four line items to 0 % or 1 % — erasing exactly the split this
                // ledger exists to show (it is the instrument FIND-139 was found with, and
                // the one `scripts/f018-budget.txt` is written around). The absolute gas
                // figures beside it are the primary reading; the percentage is the scale.
                pct = 100.0 * total / tank,
                b = ledger.spent[0],
                s = ledger.spent[1],
                r = ledger.spent[2],
                d = ledger.spent[3],
                f = ledger.spent[4],
                wb = ledger.wanted[0],
                ws = ledger.wanted[1],
                wr = ledger.wanted[2],
                wd = ledger.wanted[3],
                wf = ledger.wanted[4],
                gb = ledger.granted[0],
                gs = ledger.granted[1],
                gr = ledger.granted[2],
                gd = ledger.granted[3],
                gf = ledger.granted[4],
            );
        }
    }
}

/// Books one tick for **one** tank, in the order the file names.
///
/// Pure on purpose: the order is a game value, and a game value has to be testable in both
/// directions without editing `assets/data/game.ron` (which belongs to the main head).
///
/// Every consumer is served **at most once** per tick, whatever the list says. A duplicate
/// entry in `gas_priority` is a data error — `tests/vector_gas.rs` names it — but it must not
/// turn into a double debit in the meantime.
pub fn book(priority: &[GasConsumer], wants: Wants, costs: Costs, gas: &mut Gas) -> GasGrant {
    let mut grant = GasGrant::default();
    // **Exhaustive `match`, no `_` arm.** The day `GasConsumer` gets a third variant
    // (`F-008` dash is already written into `docs/features.ron`), this file has to stop
    // compiling. A catch-all would instead silently hand the new consumer nothing.
    for consumer in priority {
        match consumer {
            GasConsumer::Boost => {
                if wants.boost && !grant.boost {
                    grant.boost = gas.try_spend(costs.boost);
                }
            }
            GasConsumer::ReelIn => {
                if wants.reel_in && !grant.reel_in {
                    grant.reel_in = gas.try_spend(costs.reel_in);
                }
            }
            GasConsumer::Steer => {
                if wants.steer && !grant.steer {
                    grant.steer = gas.try_spend(costs.steer);
                }
            }
            GasConsumer::Dodge => {
                if wants.dodge && !grant.dodge {
                    grant.dodge = gas.try_spend(costs.dodge);
                }
            }
            GasConsumer::Flip => {
                if wants.flip && !grant.flip {
                    grant.flip = gas.try_spend(costs.flip);
                }
            }
        }
    }
    grant
}

/// Who wants gas this tick. **A struct and not three `bool` arguments** — [`book`] would
/// otherwise take three `bool`s and three `f32`s in a row, and the day somebody swaps two of
/// them the compiler says nothing and a dodge is billed a reel-in's price.
///
/// Every field is the *want*, already filtered by whether it can have an effect:
/// `reel_in` is false without an anchored hook, `dodge` is false without a movement direction
/// (`vector::boost::dodge_direction`). **The cost follows the effect, not the button.**
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Wants {
    pub boost: bool,
    pub reel_in: bool,
    /// `F-006` rope steering: `anchored_count() > 0` **and** a movement key that is not only
    /// `S`. Both halves are the effect, not the button.
    pub steer: bool,
    pub dodge: bool,
    /// `F-009` flip: the double-tap landed, the player is **not** `Grounded`, and `move_x` is
    /// not zero. All three halves are the effect, not the button — a flip with no sideways
    /// input has no direction to throw anybody in.
    pub flip: bool,
}

/// What one tick of each consumer costs, in gas.
///
/// ⚠️ **`boost`, `reel_in` and `steer` are per-tick amounts** — the rate out of the file already
/// multiplied by `dt` — **and `dodge` is not.** A dodge is one impulse and is billed whole, on
/// the one tick its grant is true. Multiplying it by `dt` as well would make it 60 times
/// cheaper and nobody would see why.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Costs {
    pub boost: f32,
    pub reel_in: f32,
    pub steer: f32,
    pub dodge: f32,
    /// `F-009`, and flat like `dodge`: `vector.gas_flip`, never multiplied by `dt`.
    pub flip: f32,
}

impl Costs {
    /// **What the gear costs while it is working under water**, `water.ron:
    /// swim.gas_cost_factor` on every one of the five.
    ///
    /// The user, 2026-08-29, on what the river does to a body: *„Man schwimmt / wird
    /// langsam."* Losing speed is `player::swim`; this is the other half, and it is the half
    /// the player pays to **leave** — getting out of the channel is a hook and a reel, and
    /// both cost double while he is in it.
    ///
    /// ⚠️ **All five, rates and impulses alike, and one factor for all of them.** Two
    /// arguments, both of which were nearly got wrong here:
    ///
    /// * A factor and not a second set of prices, because five water prices beside five land
    ///   prices is five pairs waiting to disagree the next time one of them moves.
    /// * It multiplies `dodge` and `flip` too, which are **impulses** and are never multiplied
    ///   by `dt` (see the field docs above). That is exactly why this is a method on `Costs`
    ///   and not a factor slipped into the three `_per_s` lines in [`gas_budget`]: applied
    ///   there it would have silently exempted the two most expensive verbs in the file.
    pub fn under_water(self, factor: f32) -> Self {
        Self {
            boost: self.boost * factor,
            reel_in: self.reel_in * factor,
            steer: self.steer * factor,
            dodge: self.dodge * factor,
            flip: self.flip * factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 60 Hz, the numbers from `game.ron` as of 2026-08-09: 18/s and 6/s.
    const BOOST: f32 = 18.0 / 60.0; // 0.3
    const REEL: f32 = 6.0 / 60.0; //  0.1
    /// `F-006` rope steering, 16/s from `game.ron` as of 2026-08-13.
    const STEER: f32 = 16.0 / 60.0; // 0.26667
    /// `F-008`, and **flat** — not divided by 60. The dodge is billed once, not per second.
    const DODGE: f32 = 45.0;
    /// `F-009`, flat as well, 20.0 from `game.ron` as of 2026-08-24.
    const FLIP: f32 = 20.0;

    /// The two oldest continuous consumers as every test below spells them; `F-006` and `F-008`
    /// are off unless a test says otherwise. A helper and not four literals per call, so that
    /// adding a fifth consumer one day touches one line rather than nine — which is exactly what
    /// `Steer` did on 2026-08-13.
    fn wants(boost: bool, reel_in: bool) -> Wants {
        Wants { boost, reel_in, steer: false, dodge: false, flip: false }
    }

    fn costs() -> Costs {
        Costs { boost: BOOST, reel_in: REEL, steer: STEER, dodge: DODGE, flip: FLIP }
    }

    #[test]
    fn f018_the_file_decides_who_gets_the_last_drop() {
        // A tank that covers each of them alone but not both: 0.35 out of 0.3 + 0.1. This is
        // the case in which, without the booking, the system order would decide — and the
        // system order is not a design.
        let mut first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, true), costs(), &mut first);
        assert!(g.boost, "the file names Boost first, so Boost gets the drop");
        assert!(!g.reel_in, "and there is nothing left over for the second one");
        assert!((first.current - 0.05).abs() < 1e-6, "0.35 - 0.3 = 0.05, got {}", first.current);

        // The same tank, the other order — and the other one wins. If this holds, the order
        // really is a value from the file and not an `if` in the code.
        let mut second = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::ReelIn, GasConsumer::Boost], wants(true, true), costs(), &mut second);
        assert!(g.reel_in, "ReelIn stands first here");
        assert!(!g.boost, "0.35 - 0.1 = 0.25 is not enough for a boost costing 0.3");
        assert!((second.current - 0.25).abs() < 1e-6, "got {}", second.current);
    }

    #[test]
    fn f018_exactly_one_of_the_two_is_served_when_the_tank_is_short() {
        // The claim as a claim, independent of which one it is: on a tight tank one and only
        // one gets fuel. Half of each would be the answer nobody can explain.
        for order in [
            [GasConsumer::Boost, GasConsumer::ReelIn],
            [GasConsumer::ReelIn, GasConsumer::Boost],
        ] {
            let mut gas = Gas { current: 0.35, ..Gas::full(100.0) };
            let g = book(&order, wants(true, true), costs(), &mut gas);
            assert_eq!(
                u8::from(g.boost) + u8::from(g.reel_in),
                1,
                "order {order:?} served {g:?}"
            );
        }
    }

    #[test]
    fn f018_an_empty_tank_pays_for_no_half_boost() {
        // `F-018` in its own words: "at 0 no more flying, only ground movement".
        let mut gas = Gas { current: 0.1, ..Gas::full(100.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, false), costs(), &mut gas);
        assert!(!g.boost, "0.1 does not cover a boost costing 0.3");
        assert!(
            (gas.current - 0.1).abs() < 1e-6,
            "a refused boost costs nothing — the tank holds {} instead of 0.1",
            gas.current
        );
    }

    #[test]
    fn f018_whoever_does_not_want_gas_does_not_pay() {
        let mut gas = Gas::full(100.0);
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(false, false), costs(), &mut gas);
        assert_eq!(g, GasGrant::default());
        assert!((gas.current - 100.0).abs() < 1e-6, "tank at {}", gas.current);
    }

    #[test]
    fn f018_the_sandbox_tank_grants_everything_and_stays_full() {
        // `--sandbox`: infinite gas, for looking around (§12a).
        let mut gas = Gas { unlimited: true, ..Gas::full(1.0) };
        let g = book(&[GasConsumer::Boost, GasConsumer::ReelIn], wants(true, true), costs(), &mut gas);
        assert!(g.boost && g.reel_in, "in the sandbox both get fuel: {g:?}");
        assert!((gas.current - 1.0).abs() < 1e-6, "and nothing leaves the tank");
        assert!(!gas.is_empty());
    }

    #[test]
    fn f018_booking_never_puts_a_drop_back_in() {
        // The station rule (Q-033) from the booking's side: `book` is the only thing this file
        // does to a tank, and it is monotone downwards. Nine hundred ticks of every
        // combination of the two buttons, and the number may not rise once — a refill smuggled
        // in here would look exactly like the regeneration that was taken out on 2026-08-12.
        for (wants_boost, wants_reel) in [(true, true), (true, false), (false, true), (false, false)]
        {
            let mut gas = Gas { current: 5.0, ..Gas::full(300.0) };
            let mut previous = gas.current;
            for tick in 0..900 {
                book(
                    &[GasConsumer::Boost, GasConsumer::ReelIn],
                    wants(wants_boost, wants_reel),
                    costs(),
                    &mut gas,
                );
                assert!(
                    gas.current <= previous + 1e-9,
                    "tick {tick} of ({wants_boost}, {wants_reel}) took the tank from {previous} \
                     up to {} — gas comes back only at a station (docs/QUESTIONS.md Q-033)",
                    gas.current
                );
                assert!(gas.current <= gas.max, "and never above max: {} of {}", gas.current, gas.max);
                previous = gas.current;
            }
        }
    }

    /// The fourth arm exists and it debits. Without this the `Steer` entry in `gas_priority`
    /// would be a consumer that is named and never served — which is exactly the state
    /// `FIND-082` describes and this file was left un-compiling to prevent.
    #[test]
    fn f006_the_rope_steer_is_billed_and_is_not_free() {
        let mut gas = Gas::full(100.0);
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Steer, GasConsumer::ReelIn],
            Wants { steer: true, ..wants(false, false) },
            costs(),
            &mut gas,
        );
        assert!(g.steer, "an anchored rope with W held got no grant: {g:?}");
        assert!(
            (gas.current - (100.0 - STEER)).abs() < 1e-6,
            "one tick of rope steering took {} instead of {STEER} — all nine judges of \
             docs/NEXT.md §1B named a free rope thrust as its biggest flaw",
            100.0 - gas.current
        );
        assert!(!g.boost && !g.reel_in, "and it paid for nobody else: {g:?}");
    }

    /// `Steer` stands **second** in `game.ron`, so the deliberate press wins the last drop over
    /// the one that is held all flight long (`docs/QUESTIONS.md` Q-037). Both directions, so the
    /// claim is about the file and not about an `if`.
    #[test]
    fn f006_the_file_decides_whether_the_last_drop_boosts_or_steers() {
        let mut boost_first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Steer],
            Wants { steer: true, ..wants(true, false) },
            costs(),
            &mut boost_first,
        );
        assert!(g.boost && !g.steer, "Boost stands first: {g:?}");

        let mut steer_first = Gas { current: 0.35, ..Gas::full(100.0) };
        let g = book(
            &[GasConsumer::Steer, GasConsumer::Boost],
            Wants { steer: true, ..wants(true, false) },
            costs(),
            &mut steer_first,
        );
        assert!(g.steer, "Steer stands first here: {g:?}");
        assert!(!g.boost, "0.35 - 0.2667 = 0.0833 does not cover a boost costing 0.3");
    }

    #[test]
    fn f018_a_doubled_entry_in_the_file_does_not_debit_twice() {
        // A broken `gas_priority` is a data error and has its own test in
        // `tests/vector_gas.rs`. Until somebody sees it, it must not quietly cost double.
        let mut gas = Gas::full(100.0);
        let g = book(
            &[GasConsumer::Boost, GasConsumer::Boost, GasConsumer::ReelIn],
            wants(true, false),
            costs(),
            &mut gas,
        );
        assert!(g.boost);
        assert!(
            (gas.current - (100.0 - BOOST)).abs() < 1e-6,
            "one boost costs one boost — the tank holds {}",
            gas.current
        );
    }

    // -----------------------------------------------------------------------------------
    // Water — the other half of *„Man schwimmt / wird langsam"* (the user, 2026-08-29).
    // Losing speed is `player::swim`; this is what the gear costs while it is under water,
    // and it is what the player pays to LEAVE the river.
    // -----------------------------------------------------------------------------------

    #[test]
    fn f018_the_gear_costs_more_under_water_and_the_two_impulses_are_not_exempt() {
        // 🔴 **All five, and the two flat ones are the point of this test.** The obvious place
        // to put a water factor is next to the three `_per_s` lines in `gas_budget` — and that
        // version would have silently exempted `dodge` (45 gas) and `flip` (20 gas), the two
        // most expensive verbs in the file, because they are impulses and never touch `dt`.
        let dry = costs();
        let wet = dry.under_water(2.0);
        assert!((wet.boost - dry.boost * 2.0).abs() < 1e-6, "boost {} -> {}", dry.boost, wet.boost);
        assert!((wet.reel_in - dry.reel_in * 2.0).abs() < 1e-6);
        assert!((wet.steer - dry.steer * 2.0).abs() < 1e-6);
        assert!(
            (wet.dodge - dry.dodge * 2.0).abs() < 1e-6,
            "the dodge is an impulse and was left dry at {} against {}",
            wet.dodge,
            dry.dodge
        );
        assert!(
            (wet.flip - dry.flip * 2.0).abs() < 1e-6,
            "the flip is an impulse and was left dry at {} against {}",
            wet.flip,
            dry.flip
        );
        // A factor of 1 is water that is free — a legal value, and the control that says the
        // five lines above are about the factor and not about the constant 2.
        let free = dry.under_water(1.0);
        assert_eq!(free, dry, "gas_cost_factor 1.0 changed a price");
    }

    #[test]
    fn f018_a_tank_that_covers_one_reel_on_the_quay_covers_none_in_the_river() {
        // What the surcharge actually *does*, through `book` and not through arithmetic: a
        // tank holding exactly one dry reel-in serves it on land and refuses it in the water.
        // That is the whole of "the way out costs you something".
        let priority = [GasConsumer::Boost, GasConsumer::ReelIn];
        let mut on_the_quay = Gas::full(REEL);
        let dry = book(&priority, wants(false, true), costs(), &mut on_the_quay);
        assert!(dry.reel_in, "one reel's worth of gas did not buy one reel on land");

        let mut in_the_river = Gas::full(REEL);
        let wet = book(&priority, wants(false, true), costs().under_water(2.0), &mut in_the_river);
        assert!(!wet.reel_in, "the same tank bought the same reel under water — water is free");
        assert!(
            (in_the_river.current - REEL).abs() < 1e-6,
            "a refused reel still took {} out of the tank",
            REEL - in_the_river.current
        );
    }

    #[test]
    fn f018_the_factor_the_game_ships_with_really_makes_water_cost_more() {
        // The number itself, out of `water.ron` and not out of a literal here: a factor of 1.0
        // or 0.0 would leave every test above green and the feature switched off.
        let data = crate::data::GameData::load(&crate::data::assets_dir().join("data"));
        let factor = data.water.swim.gas_cost_factor;
        assert!(
            factor > 1.0,
            "water.ron: swim.gas_cost_factor is {factor} — the gear costs no more in the river \
             than on the quay, and half of what he asked for is missing"
        );
    }
}
