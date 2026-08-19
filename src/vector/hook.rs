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
//! 1. **A shot leaves from `Idle` and from `Retracting`.** ⚠️ It used to leave only from
//!    `Idle`, and that was wrong: the user, 2026-08-12, *„wenn ich mit seilen festhake (was
//!    instant sein soll)"*. The old rule opened a lockout of
//!    `rope_length / vector.hook_retract_speed_m_s` after every release — measured **21.6
//!    ticks** on a 180 m rope at the file's 500 m/s — and it did something worse than making
//!    the player wait: it **swallowed the trigger**. A press during the retract found the
//!    wrong state and was dropped, and by the time the arm reached `Idle` the button was
//!    already held, so `just_pressed` never came back and the arm never fired at all. The
//!    player had to let go and press a *second* time.
//!    **A fresh trigger now cancels the retract** (`tests/vector_hooks.rs::
//!    f002_a_refire_during_retract_flies_again_within_one_tick`, ≤ 1 tick). `Retracting` is
//!    still a state — it is what brings a missed tip home and what the rope is drawn from —
//!    it is just no longer a lockout.
//! 2. **A shot at nothing anchorable never becomes a flight.**
//!    [`HookState::Flying`](crate::shared::HookState::Flying) carries a
//!    [`BodyId`](crate::shared::BodyId) and not an `Option<BodyId>` (`src/shared/gear.rs`), so
//!    there is no way to express "the tip is on its way to nothing" without inventing a
//!    sentinel id in someone else's type. The trigger therefore reports
//!    `ReleaseReason::NoAnchor` **in the same tick** and the arm stays `Idle`. What that costs
//!    is the flight time of a miss — a miss is free today. The fix is not ours to make: it is
//!    a field in `shared/gear.rs` and stands in the report as a finding.
//! 7. **A miss says why** (`F-028`, 2026-08-19). *„teilweise kann man gar nicht usen weil keine
//!    ahnung wieso"* — until today all four ways a pull can fail came out as the same silent
//!    `NoAnchor` and the arm just stayed `Idle`. [`ReleaseReason::NoAnchor`] now carries a
//!    [`MissReason`], decided by [`miss_reason`] out of exactly the fields
//!    [`anchor_target`] read, plus one probe ray ([`anchorable_beyond_reach`]) that is cast
//!    **only in the tick a pull failed** and tells "open sky" from "further than
//!    `hook_range_m`". Nothing about where a rope goes changed; what changed is that the
//!    failure has a name in the log and a word on the HUD (`hud::arm_aim`).
//! 3. **The tip starts and comes home at eye height** (`player.eye_height_m`), the same point
//!    `vector::aim` shoots its ray from. Not because the gear sits at the eye, but because the
//!    flight distance has to be the distance the aim ray measured — otherwise the flight time
//!    is off by the offset between the two, and nobody would ever find that number again. A
//!    real shoulder socket is a number the RON does not have yet (finding).
//! 4. **`tip_m` is not touched while `Idle`.** The type documents it as meaningless there, and
//!    dragging it along with the hand would mark [`Hook`] changed for every player in every
//!    tick — change detection for a value nobody may read (§11: nothing changes per frame).
//! 5. **A refire starts in the hand, not where the retracting tip happens to be.** The tip
//!    snaps home in the tick the new shot leaves. The price is one frame of visual jump when
//!    a very long rope is cancelled early; what it buys is decision 3's invariant — the
//!    flight time is `aim distance / hook_speed_m_s` and nothing else. A flight that started
//!    at an arbitrary leftover position would take a time nobody can compute from the file.
//! 6. **The two arms fire at [`ArmAim`], not at [`AimPoint`].** `vector::aim` casts three
//!    rays and publishes a resolved target per side (`F-023`'s hemisphere split); this file
//!    re-casts nothing at fire time, so the marker the HUD draws and the point the rope flies
//!    to are the same number by construction — *„und dann muss das seil auch dahin!!"*.
//!    [`AimPoint`] is still read by the crosshair, and nowhere here.
//!
//! The picture and the run that belong to this file: `scripts/f-001-hooks.txt` and
//! `docs/images/f-001-hooks.png`; the tests are in `tests/vector_hooks.rs`.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    AimPoint, ArmAim, Body, BodyGone, BodyId, BodyMask, Buttons, Hook, HookAnchored, HookArm,
    HookReleased, HookState, Intent, MissReason, PlayerId, PrevButtons, ReleaseReason, RopeLength,
    Side, SpatialIndex, Tick, AIM_RAY_SEES,
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

/// **Why that pull found nothing** (`F-028`) — the inverse of [`anchor_target`], and the only
/// place the three failures are told apart.
///
/// The user, 2026-08-18: *„teilweise kann man gar nicht usen weil keine ahnung wieso."* Until
/// today every one of these came out as the same silent `None`, the arm stayed [`HookState::
/// Idle`] and nothing on screen moved. `B-007` is the measurement: a titan carries no
/// [`Body`], so aiming at one is a *hit* that holds nothing — and because he is solid he also
/// blocks the wall behind him. Those are two different sentences to the player ("aim past
/// him") and one of them used to be indistinguishable from "you are pointing at the sky".
///
/// **A pure function on exactly what [`anchor_target`] read**, plus one boolean the caller
/// paid a ray for. The order of the arms is the order the world answers in — no hit at all
/// first, then a hit that does not hold, then the carrier — so that a case can never be
/// swallowed by an earlier one that is also true.
///
/// ⚠️ It is only ever called where `anchor_target` said `None`; for an aim that *does* hold it
/// still returns the last arm, and that is deliberate — a reason for a shot that left is
/// nonsense, and a `panic!` in the fire path would be worse than a nonsense word in a log.
pub fn miss_reason(aim: &AimPoint, anchorable_beyond_reach: bool) -> MissReason {
    match (aim.point_m, aim.anchorable) {
        (None, _) if anchorable_beyond_reach => MissReason::OutOfReach,
        (None, _) => MissReason::NothingInRange,
        (Some(_), false) => MissReason::SurfaceHoldsNothing,
        (Some(_), true) => MissReason::NoCarrier,
    }
}

/// One probe ray, **cast only in the tick a pull found nothing at all**, to tell "open sky"
/// from "too far".
///
/// ## Why this does not break decision 6, and cannot
///
/// Decision 6 says this file re-casts nothing at fire time, so that the marker and the rope
/// are one number. That holds: this ray is asked **only** where `anchor_target` already
/// returned `None`, i.e. where there is no target for anything to disagree about. Its answer
/// goes into a log line and a HUD word and touches no [`Hook`] field.
///
/// ## Why the look direction is the right line
///
/// `vector::aim` falls the arm back to the **centre** ray whenever the side ray found nothing
/// hookable (`if anchor_target(&found).is_some() { found } else { centre }`), so an
/// [`ArmAim`] that misses *is* the centre ray — and the centre ray is `intent.look_dir()`.
/// Recomputing the side direction here would be a second spelling of the fan for no gain.
///
/// ## What it costs
///
/// One BVH walk (measured 0.21 us against 4000 blocks, `vector::aim`'s header), on a trigger
/// edge that failed — never per frame and never per entity (§6 rule 6). `reach_m` is **twice
/// `world.half_extent_m`**: the whole world and not a new tuning number, because the question
/// is "is there anything at all out there", not "how far may an assist look".
fn anchorable_beyond_reach(
    space: &SpatialQuery,
    bodies: &Query<(&Body, Option<&BodyId>)>,
    player: Entity,
    from_m: Vec3,
    look: Vec3,
    reach_m: f32,
) -> bool {
    if !from_m.is_finite() || !(reach_m.is_finite() && reach_m > 0.0) {
        return false;
    }
    let Ok(direction) = Dir3::new(look) else {
        return false;
    };
    // Same mask as `vector::aim::cast`, and for the same reason: a team mate standing in the
    // line would answer this ray, carry no `Body`, and turn "there is something out there,
    // just too far" into "there is nothing out there" (`shared::AIM_RAY_SEES`).
    let filter = SpatialQueryFilter::from_excluded_entities([player]).with_mask(AIM_RAY_SEES);
    let Some(hit) = space.cast_ray(from_m, direction, reach_m, true, &filter) else {
        return false;
    };
    // Anchorable **and** carried: exactly what `anchor_target` would have accepted, so the
    // word "out of reach" is only ever said about a point that reach alone was in the way of.
    match bodies.get(hit.entity) {
        Ok((body, Some(_))) => body.mask.contains(BodyMask::ANCHORABLE),
        _ => false,
    }
}

/// Sends an arm out, if the trigger found something that holds. Reports whether it left.
///
/// The **one** place a shot starts, called from `Idle` and from `Retracting` — two call sites
/// with two copies of this is how a refire ends up obeying a different rule than a first shot.
/// The tip starts in the hand (decision 5) and [`HookState::Flying`] carries the carrier it is
/// flying at (`B-001`), so a miss can never become a flight.
fn fire(arm: &mut HookArm, aim: &AimPoint, hand_m: Vec3) -> bool {
    match anchor_target(aim) {
        Some((target_m, body)) => {
            arm.state = HookState::Flying { target_m, body };
            arm.tip_m = hand_m;
            true
        }
        None => false,
    }
}

/// Drives both arms through their states and reports every change.
pub fn update_hooks(
    tick: Res<Tick>,
    time: Res<Time<Fixed>>,
    data: Res<GameData>,
    index: Res<SpatialIndex>,
    space: SpatialQuery,
    bodies: Query<(&Body, Option<&BodyId>)>,
    mut gone_messages: MessageReader<BodyGone>,
    mut anchored: MessageWriter<HookAnchored>,
    mut released: MessageWriter<HookReleased>,
    mut players: Query<(
        Entity,
        &PlayerId,
        &Intent,
        &PrevButtons,
        &Transform,
        &ArmAim,
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
    // The whole world, so "too far" is answered without a second tuning number of its own.
    let probe_m = 2.0 * data.game.world.half_extent_m;

    for (entity, id, intent, prev, transform, arm_aim, rope, mut hook) in &mut players {
        // The hand is the eye — see decision 3 in the module header.
        let hand_m = transform.translation + Vec3::Y * data.game.player.eye_height_m;
        let fresh = intent.buttons.just_pressed(prev.0);

        for side in Side::ALL {
            let i = side.index();
            let held = intent.buttons.contains(button(side));
            let just_pressed = fresh.contains(button(side));

            let arm = hook.arms[i];
            let mut next = arm;

            // What THIS arm is aiming at — its own hemisphere's ray (`F-023`), already
            // resolved by `vector::aim` and never re-cast here (decision 6).
            let aim = arm_aim.side(side);

            match arm.state {
                HookState::Idle => {
                    if just_pressed && !fire(&mut next, aim, hand_m) {
                        // The trigger was pulled and nothing caught. The arm stays
                        // `Idle` (decision 2), but `hud` and `sound` still learn that
                        // a shot happened — **and since `F-028` they learn why**. One
                        // probe ray, on a failed edge only, tells open sky from too far.
                        let why = miss_reason(
                            aim,
                            aim.point_m.is_none()
                                && anchorable_beyond_reach(
                                    &space,
                                    &bodies,
                                    entity,
                                    hand_m,
                                    intent.look_dir(),
                                    probe_m,
                                ),
                        );
                        released.write(HookReleased {
                            player: *id,
                            side,
                            reason: ReleaseReason::NoAnchor(why),
                            tick: tick.0,
                        });
                        info!(
                            "hook {side:?} of player {} found no anchor: {why:?} — {} (t={})",
                            id.0,
                            why.explains(),
                            tick.0
                        );
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
                    // **The refire** (decision 1). The trigger is honoured in the tick it
                    // goes down, whatever the tip is doing — that is what "instant" means,
                    // and it is why this branch is checked before the retract step below.
                    if just_pressed {
                        if fire(&mut next, aim, hand_m) {
                            info!(
                                "hook {side:?} of player {} fired again during the retract \
                                 (t={})",
                                id.0, tick.0
                            );
                        } else {
                            // A refire at nothing anchorable. Reported like any other miss —
                            // with the same reason, through the same function, so a refire can
                            // never explain itself differently from a first shot.
                            let why = miss_reason(
                                aim,
                                aim.point_m.is_none()
                                    && anchorable_beyond_reach(
                                        &space,
                                        &bodies,
                                        entity,
                                        hand_m,
                                        intent.look_dir(),
                                        probe_m,
                                    ),
                            );
                            released.write(HookReleased {
                                player: *id,
                                side,
                                reason: ReleaseReason::NoAnchor(why),
                                tick: tick.0,
                            });
                            info!(
                                "hook {side:?} of player {} found no anchor on a refire: \
                                 {why:?} — {} (t={})",
                                id.0,
                                why.explains(),
                                tick.0
                            );
                        }
                    }
                    if next.state == HookState::Retracting {
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

    /// `F-028` — the four ways a pull fails are four different sentences to the player, and
    /// they were one silent `None` until 2026-08-19 (`B-007`, the user's *„keine ahnung
    /// wieso"*).
    ///
    /// The pairing with [`anchor_target`] is the point: **every** aim this function is asked
    /// about is one `anchor_target` rejected, so the two can never disagree about whether a
    /// shot left.
    #[test]
    fn f028_a_pull_that_finds_nothing_says_which_of_the_four_it_was() {
        let holds = AimPoint {
            point_m: Some(Vec3::new(0.0, 8.0, -20.0)),
            body: Some(BodyId(4)),
            anchorable: true,
        };
        // Nothing on that line at all, and nothing beyond reach either: open sky. Turn.
        let sky = AimPoint::default();
        assert_eq!(miss_reason(&sky, false), MissReason::NothingInRange);
        // The same empty aim, but the probe found an anchor further out: come closer. This is
        // the pair that used to be indistinguishable, and it is the whole reason the probe
        // exists.
        assert_eq!(miss_reason(&sky, true), MissReason::OutOfReach);
        // `B-007`: a titan is a solid hit that carries no `Body` — a surface that holds
        // nothing, and it hides the wall behind it. Aim past him.
        let titan = AimPoint { anchorable: false, body: None, ..holds };
        assert_eq!(miss_reason(&titan, false), MissReason::SurfaceHoldsNothing);
        // A hit surface stays a hit surface even when something anchorable stands behind it —
        // the probe may not talk over the near hit, or "aim past him" turns into "come
        // closer" and the player walks the wrong way.
        assert_eq!(miss_reason(&titan, true), MissReason::SurfaceHoldsNothing);
        // Anchorable, but no stable carrier — `B-001`'s failure, and a world fault rather
        // than a player error.
        assert_eq!(
            miss_reason(&AimPoint { body: None, ..holds }, false),
            MissReason::NoCarrier
        );
        // And every one of them is a reason `anchor_target` really did reject.
        for aim in [sky, titan, AimPoint { body: None, ..holds }] {
            assert_eq!(anchor_target(&aim), None);
        }
        // Four variants, four different sentences — a table where two rows read the same is a
        // table that explains nothing.
        let all = [
            MissReason::NothingInRange,
            MissReason::OutOfReach,
            MissReason::SurfaceHoldsNothing,
            MissReason::NoCarrier,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a.explains(), b.explains(), "{a:?} and {b:?} say the same thing");
            }
        }
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
