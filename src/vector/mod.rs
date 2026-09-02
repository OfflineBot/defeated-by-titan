//! vector — THE CORE: hooks, rope, momentum, gas, boost, wallrun
//!
//! **The game lives and dies by this feel — not by the Titan AI.**
//! A player who flies elegantly through the city without killing a single Titan has to be
//! having fun. If that does not work, nothing works (bible 2, pillar P1).
//!
//! Hence the hard gate: **no meta system before the movement convinces.** And hence every
//! number here lives in `assets/data/game.ron` and none in the code.
//!
//! Two **independently** steerable hooks (`F-001`), pendulum physics with two hooks set
//! (`F-004`), reel-in (`F-005`), swerve (`F-006`). Rope forces need guards: normalizing a
//! zero-length vector produces NaN, and NaN in the `Transform` looks like
//! "the player has vanished" (§9d).
//!
//! **Status:** the seam is in place, the math is missing. Five modules, five files, five
//! jobs — **the registration here is already complete**, so that no agent has to touch this
//! file later and no two jobs end up fighting over it
//! (`docs/interface.md`, file ownership).
//!
//! | File | F-ID | writes |
//! |---|---|---|
//! | `aim.rs` | `F-002`, `F-003` | `AimPoint` |
//! | `hookable.rs` | `F-003` (`Q-078`) | nothing — one predicate, and the switch behind it |
//! | `gas.rs` | `F-018` | `Gas`, `GasGrant` |
//! | `hook.rs` | `F-001` | `Hook`, `PrevButtons` |
//! | `boost.rs` | `F-007`, `F-008` (the impulse) | `BoostAccel` |
//! | `dodge.rs` | `F-008` (the magazine), `F-009` | `DodgeCharges`, `Invulnerable` |
//! | `reel.rs` | `F-005` | `ReelSpeed` |
//!
//! What is **not** here: `Velocity`, `Transform`, `RopeLength`. Those are written by the
//! integrator in `player::integrator` — one writer, not two.

pub mod reel;
pub mod gas;
pub mod hook;
pub mod boost;
pub mod dodge;
pub mod aim;
pub mod hookable;

use bevy::prelude::*;

use crate::shared::{RefuelRequest, SimulationSystems};

pub struct VectorPlugin;

impl Plugin for VectorPlugin {
    fn build(&self, app: &mut App) {
        // The channel a refuel station asks over. Registered **here** and not in `src/lib.rs`
        // with the other messages, for the reason that makes this message different from all
        // of them: it is a write path into `Gas`, this domain is the only writer of `Gas`
        // (`docs/architecture.md`, authority table), and a channel into a field must not be
        // able to exist without the one system that applies it. Registering a message twice is
        // a no-op in Bevy, so moving the line to `lib.rs` later costs nothing.
        app.add_message::<RefuelRequest>();

        // **`Q-078`: what a hook may take hold of.** Everything, today — the resource exists
        // so that *„spaeter soll man auch bestimmte sachen toggeln koennen"* costs one value
        // and no code (`vector::hookable`). It is registered here rather than in `src/lib.rs`
        // for the same reason `RefuelRequest` is: it decides a rule this domain is the only
        // reader of, and a switch that can be missing is a switch whose default is invisible.
        app.init_resource::<hookable::HookableSurfaces>();

        // 🔴 **The aim resolution runs TWICE per tick — one function, two instances, one per
        // reader** (`docs/BUGS.md` B-029 + B-041, `docs/FINDINGS.md` FIND-217).
        //
        // **`PostStep`: `aim`, for the PICTURE.** The aim ray is cast from
        // `translation + Y·eye_height_m`; `render::attach_camera` hangs the camera on the player
        // at exactly that offset, and the HUD projects the ray's answer through the camera at the
        // **end** of the step. Resolved before `Integrate`, the ray started at LAST tick's eye
        // and the marker was drawn through THIS tick's — an **angle** error, `v·dt/d`, that
        // diverges as he closes on the surface: over the boost of `scripts/f026-turn.txt`,
        // median 14.00 px, p95 48.74 px, **max 419.98 px** and a 392.92 px jump in one frame
        // (B-029; *„es bewegt sich immernoch also die target seile"* — the user, twice).
        //
        // **`World`: `aim::pre_fire_aim`, for the ROPE.** `hook::update_hooks` consumes
        // `ArmAim` in `Intent` and re-casts nothing (its decision 6); with `PostStep` as the
        // only writer it read the PREVIOUS tick's resolution, so a `hook` in the same tick as
        // a `look` flew along the OLD look — the gun and the eye disagreed for exactly one
        // tick (B-041, `scripts/b041-stale-look.txt`). This instance re-resolves with THIS
        // tick's delivered look on the end-of-last-tick `Transform`, which is the very `Vec3`
        // the drawn frame used: nothing writes a player's `Transform` between `PostStep` and
        // `Intent` (`Integrate` is the only writer and sits between them the other way round).
        //
        // When the look did not move between the two instances, the inputs are bit-identical
        // through the same code and `set_if_neq` elides the second write — the behavioural
        // delta is confined to exactly the ticks B-041 was about
        // (`tests/vector_aiming.rs::f002_the_ray_starts_at_the_eye_the_frame_is_drawn_from`,
        // `tests/hud.rs::f026_the_marker_stays_on_the_cursor_while_he_is_flying`).
        //
        // `PostStep` also means the picture's ray sees avian's dynamic tree as `Integrate`
        // left it rather than as the previous tick left it — a strict improvement for the day
        // a titan limb becomes an anchor (`F-029`), and the one-tick lag `aim`'s own header
        // warns about.
        app.add_systems(FixedUpdate, aim::pre_fire_aim.in_set(SimulationSystems::World))
            .add_systems(FixedUpdate, aim::aim.in_set(SimulationSystems::PostStep))
            // `.chain()`: the tank is topped up BEFORE this tick's spending is booked, and the
            // gas budget is booked BEFORE the hook switches — otherwise it hangs on system
            // order whether a freshly set hook already costs gas in the same tick.
            //
            // `apply_refuel_requests` reads requests that `mission::hub` wrote in the PREVIOUS
            // tick's `PostStep` — one tick of latency, bought on purpose. Applying in the same
            // tick would mean ordering a `vector` system against a `mission` system, which is a
            // hidden edge past the allow list; and inside one set, without an order, whether
            // the request arrives this tick or the next would be a coin flip at 60 Hz.
            .add_systems(
                FixedUpdate,
                (gas::apply_refuel_requests, gas::gas_budget, hook::update_hooks)
                    .chain()
                    .in_set(SimulationSystems::Intent),
            )
            // **Deliberately without `.chain()`**: both write their own component by
            // assignment, the `&mut` sets are disjoint, so the order is provably
            // irrelevant — and Bevy really does run them in parallel.
            // **Deliberately without `.chain()`**: each writes its own component by
            // assignment and the `&mut` sets are disjoint — `BoostAccel`, `ReelSpeed`,
            // `DodgeCharges`, `Invulnerable`, one writer each — so the order is provably
            // irrelevant, and Bevy really does run them in parallel.
            //
            // All four read this tick's `GasGrant`, which `gas::gas_budget` wrote back in
            // `Intent`. `dodge::spend_and_recharge` therefore charges for the very dash
            // `boost::gas_boost` is throwing on the same tick — not for last tick's.
            .add_systems(
                FixedUpdate,
                (
                    boost::gas_boost,
                    reel::reel_in,
                    dodge::spend_and_recharge,
                    dodge::flip,
                )
                    .in_set(SimulationSystems::Drive),
            )
            .add_systems(
                FixedUpdate,
                hook::store_prev_buttons.in_set(SimulationSystems::PostStep),
            );
    }
}
