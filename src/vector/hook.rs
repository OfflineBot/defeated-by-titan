//! `F-001` The double hook — the state machine of both arms.
//!
//! `Idle -> Flying -> Anchored -> Retracting -> Idle`, per side independently (`F-001`
//! verbatim: „Zwei unabhaengig steuerbare Enterhaken (links/rechts), einzeln abfeuerbar und
//! loesbar").
//!
//! **It fires on the edge, not on the hold**: `Buttons::just_pressed` against
//! [`PrevButtons`] — holding is not firing. The previous state is a **component on the
//! player**, not a `Local<Buttons>`: a `Local` belongs to the system and is shared by all
//! players (player 2 fires when player 1 lets go), and it is invisible in the snapshot, so it
//! survives no rollback.
//!
//! This module is the **only writer of [`Hook`]** and the only sender of
//! `HookAnchored`/`HookReleased`. Two reasons to release come from outside and are merely
//! carried out here:
//! - `BodyGone` (the carrier is gone) — in the same tick, because
//!   `SimulationSystems::Spatial` runs before `SimulationSystems::Intent`.
//! - `RopeLength::overextended` (the wall has won) — **one tick later**, because the
//!   integrator only sets it in `SimulationSystems::Integrate`. One tick of lag is the price
//!   for `Hook` having exactly one writer.
//!
//! ## Four decisions this file makes, and what each of them costs
//!
//! 1. **A shot only leaves from `Idle`.** The arm has to have its tip back in the hand before
//!    it goes out again — that is what makes `Retracting` a state and not a decoration. The
//!    price is a lockout after a release: `rope_length / vector.hook_retract_speed_m_s`, so
//!    0.25 s on a 30 m rope at the file's 120 m/s. Both ends of that number are in the RON.
//! 2. **A shot at nothing anchorable never becomes a flight.**
//!    [`HookState::Flying`](crate::shared::HookState::Flying) carries a
//!    [`BodyId`](crate::shared::BodyId) and not an `Option<BodyId>` (`src/shared/gear.rs`), so
//!    there is no way to express "the tip is on its way to nothing" without inventing a
//!    sentinel id in someone else's type. The trigger therefore reports
//!    `ReleaseReason::NoAnchor` **in the same tick** and the arm stays `Idle`. What that costs
//!    is the flight time of a miss — a miss is free today. The fix is not ours to make: it is
//!    a field in `shared/gear.rs` and stands in the report as a finding.
//! 3. **The tip starts and comes home at eye height** (`player.eye_height_m`), the same point
//!    `vector::aim` shoots its ray from. Not because the gear sits at the eye, but because the
//!    flight distance has to be the distance the aim ray measured — otherwise the flight time
//!    is off by the offset between the two, and nobody would ever find that number again. A
//!    real shoulder socket is a number the RON does not have yet (finding).
//! 4. **`tip_m` is not touched while `Idle`.** The type documents it as meaningless there, and
//!    dragging it along with the hand would mark [`Hook`] changed for every player in every
//!    tick — change detection for a value nobody may read (§11: nothing changes per frame).
//!
//! The picture and the run that belong to this file: `scripts/f-001-hooks.txt` and
//! `docs/images/f-001-hooks.png`; the tests are in `tests/vector_hooks.rs`.

use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    AimPoint, BodyGone, BodyId, Buttons, Hook, HookAnchored, HookReleased, HookState, Intent,
    PlayerId, PrevButtons, ReleaseReason, RopeLength, Side, SpatialIndex, Tick,
};

/// Which button belongs to which arm. One place, so that left and right cannot drift apart.
pub fn button(side: Side) -> Buttons {
    match side {
        Side::Left => Buttons::HOOK_LEFT,
        Side::Right => Buttons::HOOK_RIGHT,
    }
}

/// What a trigger pull finds — or `None`, which means `ReleaseReason::NoAnchor`.
///
/// **Hit first, then check anchorable** (`F-023`): `vector::aim` delivers the nearest *solid*
/// hit together with its mask and does not pre-filter, so a ray that ends on an untagged wall
/// arrives here as `anchorable: false` — and a hook through the wall is exactly what must not
/// happen.
///
/// The **range** is not re-checked here. `vector.hook_range_m` is `aim`'s rule and belongs to
/// exactly one place; a second check with a second reading of the same number is how the two
/// drift apart.
///
/// A free function so that the rule can be tested without an app running.
pub fn anchor_target(aim: &AimPoint) -> Option<(Vec3, BodyId)> {
    if !aim.anchorable {
        return None;
    }
    Some((aim.point_m?, aim.body?))
}

/// Drives both arms through their states and reports every change.
pub fn update_hooks(
    tick: Res<Tick>,
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    index: Res<SpatialIndex>,
    mut gone_messages: MessageReader<BodyGone>,
    mut anchored: MessageWriter<HookAnchored>,
    mut released: MessageWriter<HookReleased>,
    mut players: Query<(
        &PlayerId,
        &Intent,
        &PrevButtons,
        &Transform,
        &AimPoint,
        &RopeLength,
        &mut Hook,
    )>,
) {
    // Collected once instead of read per player: a `MessageReader` has one cursor, and the
    // second player would find it empty. The allocation happens only in a tick in which a
    // body really disappeared — `is_empty()` is the guard, and `Vec::new()` does not allocate.
    let gone: Vec<BodyId> = if gone_messages.is_empty() {
        Vec::new()
    } else {
        gone_messages.read().map(|m| m.body).collect()
    };

    let dt = time.delta_secs();
    let v = &data.game.vector;
    let flight_per_tick_m = v.hook_speed_m_s * dt;
    let retract_per_tick_m = v.hook_retract_speed_m_s * dt;

    for (id, intent, prev, transform, aim, rope, mut hook) in &mut players {
        // The hand is the eye — see decision 3 in the module header.
        let hand_m = transform.translation + Vec3::Y * data.game.player.eye_height_m;
        let fresh = intent.buttons.just_pressed(prev.0);

        for side in Side::ALL {
            let i = side.index();
            let held = intent.buttons.contains(button(side));
            let just_pressed = fresh.contains(button(side));

            let arm = hook.arms[i];
            let mut next = arm;

            match arm.state {
                HookState::Idle => {
                    if just_pressed {
                        match anchor_target(aim) {
                            Some((target_m, body)) => {
                                next.state = HookState::Flying { target_m, body };
                                next.tip_m = hand_m;
                            }
                            None => {
                                // The trigger was pulled and nothing caught. The arm stays
                                // `Idle` (decision 2), but `hud` and `sound` still learn that
                                // a shot happened — that is what the reason is for.
                                released.write(HookReleased {
                                    player: *id,
                                    side,
                                    reason: ReleaseReason::NoAnchor,
                                    tick: tick.0,
                                });
                                info!(
                                    "hook {side:?} of player {} found nothing anchorable (t={})",
                                    id.0, tick.0
                                );
                            }
                        }
                    }
                }

                HookState::Flying { target_m, body } => {
                    if !held {
                        next.state = HookState::Retracting;
                        released.write(HookReleased {
                            player: *id,
                            side,
                            reason: ReleaseReason::Released,
                            tick: tick.0,
                        });
                    } else if gone.contains(&body) {
                        next.state = HookState::Retracting;
                        released.write(HookReleased {
                            player: *id,
                            side,
                            reason: ReleaseReason::BodyGone,
                            tick: tick.0,
                        });
                    } else {
                        // The target stands still in the world for the length of the flight.
                        // A tip that chased a moving carrier would arrive at a time nobody
                        // can compute — and moving carriers are `F-029`, not today.
                        let to_target = target_m - arm.tip_m;
                        let distance_m = to_target.length();
                        if distance_m <= flight_per_tick_m {
                            match index.body(body) {
                                Some(entry) => {
                                    next.tip_m = target_m;
                                    // In the CARRIER's frame, not in the world's: from
                                    // `F-029` on the anchor rides along when the body moves.
                                    next.state = HookState::Anchored {
                                        body,
                                        local_m: target_m - entry.center_m,
                                    };
                                    anchored.write(HookAnchored {
                                        player: *id,
                                        side,
                                        body,
                                        point_x: target_m.x,
                                        point_y: target_m.y,
                                        point_z: target_m.z,
                                        tick: tick.0,
                                    });
                                    info!(
                                        "hook {side:?} of player {} anchored on body {} at \
                                         {:.2} {:.2} {:.2} (t={})",
                                        id.0, body.0, target_m.x, target_m.y, target_m.z, tick.0
                                    );
                                }
                                None => {
                                    // The carrier vanished while the tip was in the air and
                                    // nobody sent a message about it. The index is the truth
                                    // about existence; the message is only the notification.
                                    next.state = HookState::Retracting;
                                    released.write(HookReleased {
                                        player: *id,
                                        side,
                                        reason: ReleaseReason::BodyGone,
                                        tick: tick.0,
                                    });
                                }
                            }
                        } else {
                            // `distance_m > flight_per_tick_m >= 0`, so the division is safe —
                            // normalizing a zero vector would be the NaN that looks like
                            // "the player has vanished" (§9d).
                            next.tip_m = arm.tip_m + to_target / distance_m * flight_per_tick_m;
                        }
                    }
                }

                HookState::Anchored { body, local_m } => {
                    let carrier = index.body(body);
                    let reason = if !held {
                        Some(ReleaseReason::Released)
                    } else if gone.contains(&body) || carrier.is_none() {
                        Some(ReleaseReason::BodyGone)
                    } else if rope.overextended[i] {
                        Some(ReleaseReason::Overextended)
                    } else {
                        None
                    };
                    match reason {
                        Some(reason) => {
                            next.state = HookState::Retracting;
                            released.write(HookReleased {
                                player: *id,
                                side,
                                reason,
                                tick: tick.0,
                            });
                            info!(
                                "hook {side:?} of player {} let go: {reason:?} (t={})",
                                id.0, tick.0
                            );
                        }
                        None => {
                            if let Some(entry) = carrier {
                                next.tip_m = entry.center_m + local_m;
                            }
                        }
                    }
                }

                HookState::Retracting => {
                    let to_hand = hand_m - arm.tip_m;
                    let distance_m = to_hand.length();
                    if distance_m <= retract_per_tick_m {
                        next.tip_m = hand_m;
                        next.state = HookState::Idle;
                    } else {
                        next.tip_m = arm.tip_m + to_hand / distance_m * retract_per_tick_m;
                    }
                }
            }

            // Write only on a real change: a `Mut` that is touched every tick marks `Hook`
            // changed every tick, and every reader (`hud`, `render`, `sound`) then works for
            // a value that did not move (§11).
            if next != arm {
                hook.arms[i] = next;
            }
        }
    }
}

/// Stores this tick's buttons for the edge detection in the next one.
///
/// Runs at the end of the step (`SimulationSystems::PostStep`) — until then every reader has
/// seen the edge. **The only writer of [`PrevButtons`]**; anyone else who needs an edge reads
/// this component instead of keeping a second previous state of his own.
pub fn store_prev_buttons(mut players: Query<(&Intent, &mut PrevButtons)>) {
    for (intent, mut prev) in &mut players {
        // Only on change, for the same reason as above: a player who presses nothing must not
        // produce a write in every one of the 60 ticks of a second.
        if prev.0 != intent.buttons {
            prev.0 = intent.buttons;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f001_the_two_arms_hang_on_the_two_mouse_buttons() {
        // Left and right must not be swapped anywhere. The mapping onto the real mouse is in
        // `src/net/local.rs`; what is fixed here is the mapping onto the arm.
        assert_eq!(button(Side::Left), Buttons::HOOK_LEFT);
        assert_eq!(button(Side::Right), Buttons::HOOK_RIGHT);
        assert_ne!(button(Side::Left), button(Side::Right));
    }

    #[test]
    fn f001_a_shot_needs_a_point_a_body_and_a_surface_that_holds() {
        let full = AimPoint {
            point_m: Some(Vec3::new(0.0, 8.0, -20.0)),
            body: Some(BodyId(4)),
            anchorable: true,
        };
        assert_eq!(anchor_target(&full), Some((Vec3::new(0.0, 8.0, -20.0), BodyId(4))));

        // A ray that ends on an untagged wall is a hit, but not an anchor (`F-023`).
        assert_eq!(anchor_target(&AimPoint { anchorable: false, ..full }), None);
        // Nothing in range at all.
        assert_eq!(anchor_target(&AimPoint { point_m: None, ..full }), None);
        // A point without a carrier would be an anchor in mid-air.
        assert_eq!(anchor_target(&AimPoint { body: None, ..full }), None);
        assert_eq!(anchor_target(&AimPoint::default()), None);
    }
}
