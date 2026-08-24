//! Blade resupply — `F-033`, the half of "economy instead of cooldowns" that had no way back.
//!
//! **Gas has had [`Gas::refill`](crate::shared::Gas::refill) since the hub landed. Blades had
//! nothing.** `Blades::pairs_left` counted down and never up, `sharpness` fell and never rose,
//! and the whole design sentence — *"you reload at supply points, from the horse, or off fallen
//! comrades"* ([`super`]) — had no code behind it at all. This module is the counterpart, and it
//! exists because the user asked for the building the racks stand in: *„auch das main gebäude in
//! dem der gas und schwert nachschub ist muss da sein"* (2026-08-12).
//!
//! ## Why this is a free function in `blades` and not a method on `Blades`
//!
//! [`Blades`](crate::shared::Blades) lives in `shared/` so that `hud` and `sound` can **read**
//! it — but **`blades` is its only writer**, and that authority is the whole reason
//! `docs/architecture.md` has a table. `Gas` cost a repair on 2026-08-12 for having two
//! (`docs/FINDINGS.md` FIND-063); this does not get to be the second one. So the arithmetic
//! stands here, in the owning domain, and a rack that wants a player restocked **asks**.
//!
//! ## The caller, since 2026-08-12 evening
//!
//! The same shape gas ended up with, one message and one system — and it is wired now:
//!
//! ```text
//! shared::message   BladeRestockRequest { player: PlayerId, seconds: f32 }
//! mission::hub      restock_at_stations (PostStep) — sends it while a player stands inside
//!                   `gear.ron: resupply.range_m` of a rack
//! blades::resupply  apply_restock_requests (Intent, next tick) — the ONLY caller of `restock`
//! ```
//!
//! The message type lives in `shared/message.rs` and the edge `mission -> shared` is free, so
//! no domain edge was bought for it — exactly as with `RefuelRequest`. **`blades` reads the
//! request and this domain does the arithmetic**, because `Blades` has one writer and it is
//! this one (`docs/architecture.md`, authority table). The cost is one tick, for the same
//! reason gas pays it: applying in the same tick would mean ordering a `blades` system against
//! a `mission` system, which no domain may do. At 1.5 pairs/s a tick is 0.025 of a pair.
//!
//! **The falsifiable test is `tests/mission.rs::
//! f033_a_rack_asks_for_blades_and_never_writes_the_harness_itself`**, which runs the rack with
//! no `blades` in the app at all — the one shape a whole-app test cannot distinguish
//! (`FINDINGS.md` FIND-063, and FIND-066 for this half).
//!
//! ## The accumulator is not decoration
//!
//! `pairs_left` is a `u8` and `blade_pairs_per_s` is 1.5. At 60 Hz that is 0.025 pairs per
//! tick, and every honest way to add 0.025 to an integer needs somewhere to keep the remainder.
//! [`RestockCarry`] is that place, it hangs on the player, and it is written here and nowhere
//! else. Without it the rate silently rounds to zero and the rack looks broken.

use bevy::prelude::*;

use crate::data::{GameData, ResupplyTuning};
use crate::shared::{BladeRestockRequest, Blades, PlayerId};

/// The fraction of a blade pair a player has already been handed at a rack.
///
/// Hangs on the player, written **only** by [`restock`]. It is not in `shared/` because nothing
/// outside this domain has a reason to look at it: it is not a resource the HUD shows, it is the
/// remainder of an integer division.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RestockCarry(pub f32);

/// Hands `dt_s` seconds' worth of resupply to one player. **The only place `Blades` grows.**
///
/// Two things happen, in this order and for a reason: the pair **in the harness** is honed
/// first, then whole spare pairs are added to it. A player who runs in on a blunt blade wants
/// the thing in his hand to work before he wants a fifth spare.
///
/// * `capacity_pairs` is `gear.ron: blades.start_pairs` — what the harness holds. It plays
///   exactly the role [`Gas::max`](crate::shared::Gas::max) plays for the tank, and like the
///   tank it is a cap and not a target.
/// * Returns `true` if anything actually changed, so a caller can stay quiet when a player
///   stands at a full rack.
///
/// Nonsense in, nothing out: a non-finite or non-positive `dt_s` changes nothing, the same way
/// [`Gas::refill`](crate::shared::Gas::refill) refuses a non-finite amount. A negative second
/// would be a way to *steal* blades at a supply point.
pub fn restock(
    blades: &mut Blades,
    carry: &mut RestockCarry,
    tuning: &ResupplyTuning,
    capacity_pairs: u8,
    dt_s: f32,
) -> bool {
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return false;
    }

    let mut changed = false;

    // 1. The pair in the harness.
    if blades.sharpness < 1.0 && tuning.sharpen_per_s.is_finite() && tuning.sharpen_per_s > 0.0 {
        blades.sharpness = (blades.sharpness + tuning.sharpen_per_s * dt_s).min(1.0);
        changed = true;
    }

    // 2. The spares. `pairs_left` is whole pairs; the remainder waits in `carry`.
    if blades.pairs_left >= capacity_pairs {
        // A full harness does not accumulate a head start for the next fight.
        if carry.0 != 0.0 {
            carry.0 = 0.0;
            changed = true;
        }
        return changed;
    }
    if !tuning.blade_pairs_per_s.is_finite() || tuning.blade_pairs_per_s <= 0.0 {
        return changed;
    }

    carry.0 += tuning.blade_pairs_per_s * dt_s;
    let whole = carry.0.floor();
    if whole >= 1.0 {
        let room = u32::from(capacity_pairs - blades.pairs_left);
        let handed = (whole as u32).min(room);
        blades.pairs_left += handed as u8;
        carry.0 -= handed as f32;
        changed = true;
        if blades.pairs_left >= capacity_pairs {
            carry.0 = 0.0;
        }
    }
    changed
}

/// **The only caller of [`restock`] in the game**, and therefore the only thing that ever grows
/// a harness.
///
/// A rack of the main building sends [`BladeRestockRequest`] while a player stands at it
/// (`mission::hub::restock_at_stations`); this reads it one tick later and does the arithmetic
/// **here**, in the domain that owns [`Blades`].
///
/// Three things that look like details and are not:
///
/// - **The tuning is read here, not sent.** `blade_pairs_per_s`, `sharpen_per_s` and the
///   `blades.start_pairs` cap are this domain's numbers. A sender that multiplied them would be
///   the second writer of `Blades` in everything but the `&mut` (see [`BladeRestockRequest`]).
/// - **[`RestockCarry`] is inserted on first use.** It is the remainder of an integer division
///   and nothing outside this file reads it, so no other system has a reason to put it on a
///   player — and a player who has never stood at a rack does not need to carry one. The value
///   the first request produced is what gets inserted, so nothing is lost.
/// - **`set_if_neq` on the harness**, exactly as `vector::gas::apply_refuel_requests` does on
///   the tank: a full harness at a full rack must not wake `Changed<Blades>` sixty times a
///   second for a number that did not move (§6 rule 6).
pub fn apply_restock_requests(
    mut commands: Commands,
    data: Res<GameData>,
    mut requests: MessageReader<BladeRestockRequest>,
    mut players: Query<(Entity, &PlayerId, &mut Blades, Option<&mut RestockCarry>)>,
) {
    let tuning = &data.gear.resupply;
    let capacity = data.gear.blades.start_pairs;
    for request in requests.read() {
        for (entity, id, mut blades, carry) in &mut players {
            if *id != request.player {
                continue;
            }
            // On copies, so `Blades` is only marked changed when the harness really is
            // different — `restock` says whether anything happened.
            let mut harness = *blades;
            let mut remainder = carry.as_deref().copied().unwrap_or_default();
            let changed = restock(&mut harness, &mut remainder, tuning, capacity, request.seconds);
            if changed {
                blades.set_if_neq(harness);
            }
            match carry {
                Some(mut held) => {
                    held.set_if_neq(remainder);
                }
                // `insert` and not `Commands::spawn`: the component lands at the next sync
                // point carrying the fraction this very request produced.
                None => {
                    commands.entity(entity).insert(remainder);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> ResupplyTuning {
        // The file's numbers, spelled out so a change in `gear.ron` shows up as a red test and
        // not as a silently different rate.
        ResupplyTuning {
            gas_per_s: 40.0,
            range_m: 4.0,
            blade_pairs_per_s: 1.5,
            sharpen_per_s: 2.0,
            // `F-019`'s three, spelled out for the same reason as the four above them. They are
            // the FIELD stations' numbers and nothing in this file reads them — a rack in the
            // hub is infinite and has no duration (`src/world/supply.rs` has the table).
            station_uses: 3,
            station_refill_s: 1.5,
            station_radius_m: 6.0,
        }
    }

    #[test]
    fn f033_a_rack_hands_back_whole_pairs_and_never_more_than_the_harness_holds() {
        let mut b = Blades { pairs_left: 0, sharpness: 1.0 };
        let mut c = RestockCarry::default();
        let t = tuning();

        // 1.5 pairs/s at 60 Hz: nothing whole in the first tick, and that is the accumulator
        // doing its job rather than the rate rounding to zero.
        assert!(!restock(&mut b, &mut c, &t, 5, 1.0 / 60.0));
        assert_eq!(b.pairs_left, 0);
        assert!(c.0 > 0.0);

        // A second at the rack is one and a half pairs — one lands, half waits.
        assert!(restock(&mut b, &mut c, &t, 5, 1.0 - 1.0 / 60.0));
        assert_eq!(b.pairs_left, 1);
        assert!((c.0 - 0.5).abs() < 1e-4, "carry is {}", c.0);

        // Five pairs from empty is 3.33 s, and the cap holds afterwards.
        for _ in 0..600 {
            restock(&mut b, &mut c, &t, 5, 1.0 / 60.0);
        }
        assert_eq!(b.pairs_left, 5);
        assert_eq!(c.0, 0.0, "a full harness must not bank a head start");
        assert!(!restock(&mut b, &mut c, &t, 5, 1.0), "a full rack visit changes nothing");
    }

    #[test]
    fn f033_the_blade_in_the_hand_is_honed_before_a_spare_is_handed_over() {
        // A player runs in on a nearly broken pair. After a quarter second the thing in his
        // hand works again — before a single spare has arrived (1.5/s needs 0.67 s).
        let mut b = Blades { pairs_left: 2, sharpness: 0.1 };
        let mut c = RestockCarry::default();
        let t = tuning();

        restock(&mut b, &mut c, &t, 5, 0.25);
        assert!((b.sharpness - 0.6).abs() < 1e-4, "sharpness is {}", b.sharpness);
        assert_eq!(b.pairs_left, 2, "no spare yet, and that is the point");

        restock(&mut b, &mut c, &t, 5, 0.25);
        assert_eq!(b.sharpness, 1.0, "honing caps at fresh");
        assert!(!b.is_broken());
    }

    #[test]
    fn f033_a_nonsense_second_never_grows_the_harness() {
        // The mirror of `Gas::refill`'s guard. A negative second at a supply point would be a
        // way to take blades OUT of the harness, and it would arrive as a division by a frame
        // time somewhere far from here.
        let t = tuning();
        for dt in [0.0f32, -1.0, f32::NAN, f32::INFINITY] {
            let mut b = Blades { pairs_left: 1, sharpness: 0.5 };
            let mut c = RestockCarry(0.9);
            assert!(!restock(&mut b, &mut c, &t, 5, dt), "dt_s = {dt} did something");
            assert_eq!(b.pairs_left, 1);
            assert_eq!(b.sharpness, 0.5);
            assert_eq!(c.0, 0.9);
        }
    }
}
