//! `F-030` — **the cut.** One swept `cast_shape` per active blade per tick.
//!
//! A titan dies only from a fast cut into the cortex. This file is the thing that sends the
//! message; what a cortex hit means for a body is `titan`'s business and nobody else's.
//!
//! ## Why a swept cast and not a blade collider
//!
//! Because everything else samples *positions* once per tick. A blade collider, a `Sensor`
//! plus `CollisionStart`, an AABB overlap — all three ask "do these two shapes overlap **now**"
//! and are only asked once per step: `SubstepSchedule` re-runs the solver alone
//! (`avian3d-0.7.0/src/dynamics/solver/schedule.rs:49-67`), broad and narrow phase run once
//! (`.../collision/narrow_phase/mod.rs:131-147`).
//!
//! The arithmetic that kills the idea: a weaver's cortex is `2 × 0.23 m = 0.46 m` across
//! (`titan.ron:75`), and 75 m/s is `75 / 60 = 1.250 m` per tick. The player is inside the
//! target for **0.37 of a tick**. An overlap test hits that 37 times in a hundred; it passes at
//! 8 m/s, it passes the husk at 30 m/s, and it is arithmetically incapable of passing the
//! weaver — which is exactly what `tests/combat.rs::f030_a_pass_at_75_m_s_still_hits_the_weaver`
//! is for.
//!
//! ## Two filtered casts, cortex layer first — and this is the OPPOSITE of `vector::aim`
//!
//! `src/vector/aim.rs:31-38` casts **unfiltered** and checks the mask afterwards, deliberately:
//! a filtered ray travels through untagged geometry, and a hook through a wall is a bug.
//!
//! **Combat is the exact opposite case.** The cortex is deliberately hidden *inside* the body
//! silhouette (`src/titan/rig.rs:434-440` puts it half a head's depth back, inside a body
//! capsule of radius `width_m / 2`), and `cast_shape` returns only the **closest** hit. One
//! unfiltered cast therefore returns the torso every single time and never the cortex. The
//! layer filter is not a shortcut here, it is the whole mechanism.
//!
//! Same crate, opposite rule. Whoever "fixes" the inconsistency breaks the game silently.
//!
//! ## Two casts, then a refinement — `F-032`, 2026-08-19
//!
//! Until 2026-08-19 a titan carried exactly **one** body collider, the root capsule, and this
//! file said so: a non-cortex hit could be reported as "not the cortex" and no more than that,
//! so `HitZone::ArmLeft`, `LegRight` and the rest had never been produced by anything in this
//! game (`docs/FINDINGS.md` FIND-109). `titan::rig` now publishes each arm and each leg as a
//! [`HitZoneOf`] box — **data, not a collider** — and the two casts stay exactly two:
//!
//! | step | what | zone |
//! |---|---|---|
//! | 1 | `cast_shape` on [`LAYER_TITAN_CORTEX`] | [`HitZone::Cortex`] — the only one that kills |
//! | 2 | `cast_shape` on [`LAYER_TITAN_BODY`] | [`HitZone::Torso`], the honest catch-all |
//! | 3 | [`limb_zone`] — the same swept blade against that titan's four limb boxes | the limb, if the blade really crossed one |
//!
//! **Step 3 is not a third collider, and that was measured.** A `Sensor` per limb on a layer of
//! its own is the obvious build, it was the first build, and it broke `F-029` inside the hour:
//! `vector::aim` casts the hook ray **unfiltered** on purpose and resolves the carrier off the
//! collider entity, an arm sticks out of the root capsule, and a rope aimed at a titan started
//! biting his arm — which carries no `Body` — instead of his body. A collision layer cannot fix
//! that; avian's default filter has `mask: LayerMask::ALL`. So a titan's physics after `F-032`
//! is bit for bit what it was before it, and the four zones cost the collision world nothing.
//!
//! **Why a refinement and not a replacement.** Every limb box lies inside or on the surface of
//! the capsule (FIND-116), so the silhouette is what a cast finds first; the limb is the *more
//! specific* answer to the same contact, and it is only asked for where the body already
//! answered. Head and eye are still not produced — see `titan::rig::hit_zone` for why the head
//! box was deliberately left out rather than renaming the graze of every measured cut in the
//! game.

use avian3d::parry::math::Pose3;
use avian3d::parry::query::{cast_shapes, ShapeCastOptions};
use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::data::{BladeTuning, GameData};
use crate::shared::{
    Blades, HitStop, HitZone, HitZoneOf, Intent, PlayerId, Side, Tick, TitanHit, TitanId, Velocity,
    LAYER_TITAN_BODY, LAYER_TITAN_CORTEX,
};

use super::swing::{BladeTiming, SweptFrom, Swings};

/// What one cast found. Separate from [`TitanHit`] because the message wants stable ids and
/// the cast returns an `Entity` — the translation happens in one place, [`cut`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladeHit {
    /// The **collider** entity that was hit, not the body. For the cortex those are two
    /// different entities (`src/titan/rig.rs:649-666`).
    pub collider: Entity,
    pub zone: HitZone,
    /// How far along the sweep the blade met it, in metres.
    pub distance_m: f32,
}

/// **Which zones one swing has already reported**, per side, one bit per [`HitZone`] — `F-032`.
///
/// ## Why the boolean it replaces was not enough any more
///
/// `super::swing::Swing::has_grazed` is one bit: *this swing has already booked a non-cortex
/// hit*. It exists because every titan is wider than his own neck, so a pass that ends in the
/// nape meets the body one or more ticks earlier, and a single "this swing has landed" flag
/// would end every swing on the shoulder.
///
/// With one bit for all seven non-cortex zones, **a limb could never be reported.** Measured
/// 2026-08-19 on the real husk: the blade meets the root capsule at `z = −119.19` and the arm
/// box at `z = −119.69`, 0.50 m apart — which at 30 m/s is exactly one tick, at 8 m/s is four
/// ticks, and at 75 m/s is none at all. The zone a cut reported would have depended on how fast
/// the player was flying, which is the sort of thing nobody finds for a month.
///
/// So the rule keeps its shape and gains a width: **each zone is reported at most once per
/// swing.** A pass along a flank still books one `Torso` however many ticks it spends inside
/// the silhouette; a pass that crosses the arm on the way books the arm as well, once.
/// [`HitZone::Cortex`] is not in here at all — it has its own flag (`Swing::has_cut`) and it
/// ends the swing.
///
/// It hangs on the **player**, not in a resource: twenty of them one day, each with his own
/// swing (`docs/multiplayer.md` rule 3). It is per-swing scratch state and deliberately not a
/// field of `Swing` — `src/blades/swing.rs` belonged to another hand the day this was written,
/// and a component that is cleared from the same loop that reads it has one writer either way.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrazedZones {
    left: u8,
    right: u8,
}

impl GrazedZones {
    fn side(&self, side: Side) -> u8 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut u8 {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// A swing that is not cutting carries no history into the next one.
    pub fn clear(&mut self, side: Side) {
        *self.side_mut(side) = 0;
    }

    pub fn contains(&self, side: Side, zone: HitZone) -> bool {
        self.side(side) & bit_of(zone) != 0
    }

    pub fn insert(&mut self, side: Side, zone: HitZone) {
        *self.side_mut(side) |= bit_of(zone);
    }
}

/// One bit per zone. Spelled out rather than derived from a discriminant, so that adding a
/// variant to [`HitZone`] is a compile error here and not a silent collision on bit 0.
fn bit_of(zone: HitZone) -> u8 {
    match zone {
        HitZone::Cortex => 1 << 0,
        HitZone::Head => 1 << 1,
        HitZone::Eye => 1 << 2,
        HitZone::ArmLeft => 1 << 3,
        HitZone::ArmRight => 1 << 4,
        HitZone::LegLeft => 1 << 5,
        HitZone::LegRight => 1 << 6,
        HitZone::Torso => 1 << 7,
    }
}

/// The blade as a segment in **world** coordinates, for one side of one player.
///
/// Endpoints, not a centre plus a rotation: [`Collider::capsule_endpoints`] takes the two ends
/// in the collider's own frame, so a capsule built from world points and cast from the world
/// origin with an identity rotation sits exactly where the arithmetic here says it does. No
/// second rotation to get wrong (the same reasoning as `titan::rig`'s use of it).
///
/// The blade hangs from the hand at eye height and reaches `reach_m` **outward**, left for the
/// left blade and right for the right one. Eye height and not the origin between the feet:
/// `player.eye_height_m` is the same number `render` hangs the camera on and `vector::aim`
/// starts its ray from — one offset, one place, or a crosshair and a cut point at different
/// things (`src/vector/aim.rs:eye`).
pub fn blade_segment(
    from_m: Vec3,
    look: Vec3,
    side: Side,
    eye_height_m: f32,
    reach_m: f32,
) -> (Vec3, Vec3) {
    let hand = from_m + Vec3::Y * eye_height_m;
    let out = match side {
        Side::Left => -blade_right(look),
        Side::Right => blade_right(look),
    };
    (hand, hand + out * reach_m)
}

/// **The one direction the blade lies on**, for a given look — the right blade's, and the
/// left one's negated.
///
/// Pulled out of [`blade_segment`] on 2026-08-19 so that [`super::hold`] can point the *drawn*
/// pair at exactly what the cast uses, out of the same expression. Two functions that both
/// "work out where the blade is" are two blades, and the picture would be the one that lies
/// (`FIND-113`: the camera is part of the shot).
pub fn blade_right(look: Vec3) -> Vec3 {
    // Right-hand rule on Bevy's axes: for `look = −Z` this gives `+X`, which is the right
    // hand of a body whose forward is −Z (`docs/conventions.md`, same convention as
    // `titan::rig::shoulder_in_torso`).
    let right = look.cross(Vec3::Y).normalize_or_zero();
    // Looking straight up or straight down leaves no horizontal "right". The blade then
    // stays on the world X axis instead of collapsing to a point — a zero-length capsule is
    // a sphere at the hand, and the cut would silently lose its whole reach.
    if right.length_squared() > 0.0 { right } else { Vec3::X }
}

/// What one **reported** hit costs the pair in the harness, in sharpness — `F-033`.
///
/// Straight out of `gear.ron: blades`, and the only place the two zones are told apart:
/// `wear_per_hit` for the cortex, `wear_per_hit * wear_torso_factor` for everything else. The
/// factor is below 1.0 and the argument for that is in `gear.ron` next to the number, because
/// it is an argument about **the file's value**, not about this function.
pub fn wear_of(blades: &BladeTuning, zone: HitZone) -> f32 {
    match zone {
        HitZone::Cortex => blades.wear_per_hit,
        // Everything else is a graze along hardened hide. Since 2026-08-19 that really does
        // include the four limbs (`F-032`, `super::cut`'s header); `Head` and `Eye` still have
        // no producer. A limb cut is a cut into a limb and not into the nape, and one rate for
        // all of them is the right default until somebody measures otherwise — the factor is
        // below 1.0 and `gear.ron` carries the argument for that next to the number.
        _ => blades.wear_per_hit * blades.wear_torso_factor,
    }
}

/// **Books one landed hit against the harness.** Returns `true` when the pair broke and a spare
/// was drawn out of the harness.
///
/// The order matters and is the whole "no soft lock" claim:
///
/// 1. the pair loses [`wear_of`] sharpness, floored at zero — a pair carries no debt into the
///    next one;
/// 2. at zero, [`Blades::swap_pair`] draws a spare and the player fights on. `pairs_left`
///    counts **spares**, so `start_pairs: 5` is six pairs' worth of fighting;
/// 3. with no spare left, `swap_pair` refuses and the harness stays at zero sharpness. That is
///    [`Blades::is_broken`], and [`cut`] then casts nothing at all.
///
/// A dry player is **not** stuck: he flies, he swings, he is still a target, he simply cannot
/// kill — and the way back is `blades::resupply` at a rack of the headquarters, which hones the
/// pair in his hands to fresh in half a second (`gear.ron: resupply.sharpen_per_s`). That is the
/// whole point of "economy instead of cooldowns": running out is a **place to go**, not a timer
/// to wait out.
pub fn spend(harness: &mut Blades, blades: &BladeTuning, zone: HitZone) -> bool {
    let cost = wear_of(blades, zone);
    // A file with a broken number must not be a file that makes blades immortal *or* one that
    // sets `sharpness` to `NaN` — a `NaN` sharpness is never `<= 0.0`, so `is_broken()` would be
    // false forever and the harness would silently stop wearing.
    if !cost.is_finite() || cost <= 0.0 {
        return false;
    }
    harness.sharpness = (harness.sharpness - cost).max(0.0);
    if harness.sharpness > 0.0 {
        return false;
    }
    harness.swap_pair()
}

/// **The cut.** Casts the blade of every active swing along the player's displacement of this
/// tick and writes [`TitanHit`].
///
/// Runs in `SimulationSystems::PostStep`: avian's `Writeback` has run, so `Transform`,
/// `Position` and `shared::Velocity` all describe the end of *this* step (`src/lib.rs:120-131`).
///
/// **Never `.single()`** on the player query — there are twenty of them one day
/// (`docs/multiplayer.md` rule 3).
#[allow(clippy::type_complexity)]
pub fn cut(
    mut commands: Commands,
    data: Res<GameData>,
    space: SpatialQuery,
    tick: Res<Tick>,
    mut messages: MessageWriter<TitanHit>,
    parents: Query<&ChildOf>,
    titans: Query<(&TitanId, &Velocity)>,
    // **The seam of `F-032`.** The boxes `titan::rig` published for each arm and each leg. This
    // domain never learns how a titan is assembled — it walks down from the body the cast
    // already found and asks each box what it is (`docs/FINDINGS.md` FIND-109). `Children` is
    // Bevy's own relation: reading it is not an edge into `titan/`, the same argument
    // [`owner`] makes about `ChildOf`.
    children: Query<&Children>,
    limbs: Query<(&HitZoneOf, &GlobalTransform)>,
    // `&mut Blades` and not `Option<&mut Blades>`: `player::spawn_player` gives every player a
    // harness, and a player without one has no blades and therefore never cuts — the same shape
    // `super::swing::equip` documents for `Swings`. `blades` is the **only** writer of `Blades`
    // (`docs/architecture.md`, authority table); this is its second writing system after
    // `super::resupply::apply_restock_requests`, and they never race: that one runs in
    // `SimulationSystems::Intent`, this one in `PostStep`, so within a tick a player standing at
    // a rack is restocked first and charged for what he cuts afterwards.
    mut players: Query<(
        Entity,
        &PlayerId,
        &Intent,
        &Transform,
        &Velocity,
        &BladeTiming,
        &mut Swings,
        &mut SweptFrom,
        &mut Blades,
        Option<&HitStop>,
        // `F-032`: which zones this player's two swings have already booked. `Option`, because
        // it is scratch state that no spawner has to remember — the first swing that reports
        // anything brings it into being, and from then on it is written in place.
        Option<&mut GrazedZones>,
    )>,
) {
    let blades = &data.gear.blades;
    let eye_height_m = data.game.player.eye_height_m;

    for (
        entity,
        id,
        intent,
        transform,
        velocity,
        timing,
        mut swings,
        mut from,
        mut harness,
        stop,
        grazed,
    ) in &mut players
    {
        // One local copy per player per tick: both sides are decided against it and it is
        // written back once, so two blades landing on the same tick cannot lose each other's
        // bit through two competing `insert`s.
        let mut zones_seen = grazed.as_deref().copied().unwrap_or_default();
        let now = transform.translation;
        // **The displacement of this tick**, and nothing derived from a velocity: the clamp
        // (`F-012`) and every contact of the step sit between `Velocity` and what the body
        // really did.
        let start = from.0;
        let delta = now - start;
        from.0 = now;

        // The hit stop freezes the blade with the body. Without it one slash lands again on
        // every tick of its own impact frame.
        if stop.is_some_and(HitStop::is_frozen) {
            continue;
        }

        // **A dry harness cuts nothing** — `F-033`. Not "cuts for less damage": `titan::brain::
        // receive_hits` kills on `Cortex` **by rule** and never consults the speed
        // (`src/shared/message.rs:21`), so a broken blade that still wrote a `TitanHit` would be
        // a free kill with no steel behind it.
        //
        // Read-only through `Deref`, so this does **not** wake `Changed<Blades>` sixty times a
        // second on a player who is not cutting. `Blades` is only ever written below, inside the
        // branch where a hit actually landed.
        //
        // **This is a state, not a soft lock.** He still flies, still swings, still bleeds; the
        // way back is a rack (`super::resupply`), and the HUD has already said so for a while —
        // `hud::blade_pips` paints the sharpness plate crimson on `is_broken()`.
        if harness.is_broken() {
            continue;
        }

        for side in Side::ALL {
            let swing = swings.side_mut(side);
            if !swing.is_active(timing) {
                // Outside the cutting window there is no swing to remember. Cleared here and
                // not on the swing's edge, so that this file owns the whole life of the bits it
                // reads (`super::swing::Swing::start` clears `has_grazed` at the same moment).
                zones_seen.clear(side);
            }
            if !swing.is_active(timing) || swing.has_cut {
                continue;
            }
            let (a, b) =
                blade_segment(start, intent.look_dir(), side, eye_height_m, blades.reach_m);
            let Some(hit) = sweep(&space, entity, blades.thickness_m, a, b, delta) else {
                continue;
            };
            // The hit entity is the collider. The `TitanId` hangs on the root of the rig, so
            // walk up — `combat` may not know how a titan is assembled, and it does not have
            // to: `ChildOf` is Bevy's, not `titan`'s.
            let Some((root, titan_id, titan_velocity)) = owner(&parents, &titans, hit.collider)
            else {
                continue;
            };

            // **`F-032`: the body answered, now ask which part of it.** Only here, only for this
            // titan, and only when the cast found the silhouette rather than the nape.
            let zone = match hit.zone {
                HitZone::Torso => {
                    let capsule = Collider::capsule_endpoints(blades.thickness_m, a, b);
                    limb_zone(&children, &limbs, root, &capsule, delta).unwrap_or(HitZone::Torso)
                }
                other => other,
            };
            // **Each zone is reported once, a cut ends the swing.** Every titan is wider than
            // his own neck, so the blade meets the body one or more ticks before the nape; one
            // single "this swing has landed" flag would end every swing on the shoulder and
            // make the cortex unreachable on every kind (`super::swing::Swing::has_grazed`).
            // Since `F-032` the flag is one bit per zone and not one bit in total — see
            // [`GrazedZones`] for the measurement that made the difference matter.
            if zone != HitZone::Cortex && zones_seen.contains(side, zone) {
                continue;
            }

            // **The CLOSING speed**, projected on the direction the blade travelled — not
            // `|v|`. A player flying past parallel to a running titan closes on nothing, and
            // `damage_per_m_s` on his 30 m/s would be a lie about what happened.
            let closing_m_s = closing_speed(velocity.0, titan_velocity.0, delta);
            if closing_m_s < blades.min_speed_m_s {
                // A scratch. Deliberately **no message at all** and not a message with a
                // small number: `titan::brain::receive_hits` kills on `Cortex` by rule and
                // does not consult the speed, so a slow touch would be a free kill
                // (`src/shared/message.rs:21`).
                continue;
            }

            if zone == HitZone::Cortex {
                swing.has_cut = true;
            } else {
                // Both, and they are not the same statement: `has_grazed` is `blades::swing`'s
                // own "this swing has touched a body" and stays exactly what it was, while the
                // mask is what decides whether THIS zone may still be reported.
                swing.has_grazed = true;
                zones_seen.insert(side, zone);
            }
            // Loud, once per swing per zone. A cut that lands is the single most important
            // event in this game, and a `--script` run has no other way to say when it
            // happened — the screenshot ticks of `scripts/f030-cortex.txt` are read off this
            // line.
            info!(
                "tick {}: cut titan {} {:?} at {:.2} m/s (player {})",
                tick.0, titan_id.0, zone, closing_m_s, id.0
            );
            messages.write(TitanHit {
                titan: titan_id,
                by: *id,
                zone,
                speed_m_s: closing_m_s,
            });

            // **And the blade pays for it.** Here and nowhere else: exactly the hits that were
            // reported are the hits that cost, so a touch under `min_speed_m_s` — which did
            // nothing to the titan and produced no message — costs nothing either. A cost whose
            // effect the player cannot see is a cost he cannot learn from.
            //
            // `&mut harness` is the first `DerefMut` on this player this tick, and it is inside
            // this branch on purpose: `Changed<Blades>` now means "the harness really moved",
            // which is what `hud::blade_pips` and every future listener get to rely on.
            if spend(&mut harness, blades, zone) {
                // Loud, like the cut above it: drawing a spare is one of five moments in a whole
                // mission, and a `--script` run has no other way to see it happen.
                info!(
                    "tick {}: player {} broke a pair, {} spare(s) left",
                    tick.0, id.0, harness.pairs_left
                );
            }
            if harness.is_broken() {
                info!("tick {}: player {} is out of blades — a rack is the way back", tick.0, id.0);
            }
        }

        match grazed {
            // In place: no `Commands`, no flush, and no `Changed` on a player who is not cutting
            // — the component is only written when the mask really moved.
            Some(mut on_body) => {
                if *on_body != zones_seen {
                    *on_body = zones_seen;
                }
            }
            // The first swing of this player that booked anything brings it into being.
            None => {
                if zones_seen != GrazedZones::default() {
                    commands.entity(entity).insert(zones_seen);
                }
            }
        }
    }
}

/// `max(0, (v_player − v_titan) · d̂)`, projected on the direction the blade swept.
///
/// Zero displacement leaves no direction to project on; the closing speed is then the length
/// of the relative velocity, which is the only honest answer when the blade did not travel.
pub fn closing_speed(player_m_s: Vec3, titan_m_s: Vec3, delta_m: Vec3) -> f32 {
    let relative = player_m_s - titan_m_s;
    match delta_m.try_normalize() {
        Some(direction) => relative.dot(direction).max(0.0),
        None => relative.length(),
    }
}

/// **Two casts, cortex first.** See the module header for why the order is the mechanism.
///
/// The limb zones are **not** in here: they are not colliders, and they are asked for by
/// [`limb_zone`] afterwards, only where this function already answered [`HitZone::Torso`].
///
/// `entity` is excluded so that the player's own capsule can never answer — it is on avian's
/// default bit and would not match either mask today, but the day somebody hangs
/// `LAYER_PLAYER` on him this line is what stops a player from cutting himself.
pub fn sweep(
    space: &SpatialQuery,
    player: Entity,
    thickness_m: f32,
    a_m: Vec3,
    b_m: Vec3,
    delta_m: Vec3,
) -> Option<BladeHit> {
    if !(a_m.is_finite() && b_m.is_finite() && delta_m.is_finite() && thickness_m > 0.0) {
        // A `NaN` out of a broken `Intent` would become a `NaN` cast and, one message later,
        // a titan that dies for no reason anybody can reproduce (`prompts/init.md` §9d).
        return None;
    }
    // World-space endpoints, so the cast origin is the world origin and the rotation is the
    // identity. See `blade_segment`.
    let capsule = Collider::capsule_endpoints(thickness_m, a_m, b_m);
    let (direction, max_distance) = match Dir3::new(delta_m) {
        Ok(direction) => (direction, delta_m.length()),
        // A standing cut. The cast still runs: `ignore_origin_penetration` is `false`, so an
        // overlap that is already there at distance 0 is reported
        // (`avian3d-0.7.0/src/spatial_query/shape_caster.rs:433-437`).
        Err(_) => (Dir3::NEG_Z, 0.0),
    };
    let config = ShapeCastConfig::from_max_distance(max_distance);

    for (mask, zone) in [
        (LAYER_TITAN_CORTEX, HitZone::Cortex),
        (LAYER_TITAN_BODY, HitZone::Torso),
    ] {
        let filter = SpatialQueryFilter::from_excluded_entities([player]).with_mask(mask);
        if let Some(hit) = space.cast_shape(&capsule, Vec3::ZERO, Quat::IDENTITY, direction, &config, &filter)
        {
            return Some(BladeHit { collider: hit.entity, zone, distance_m: hit.distance });
        }
    }
    None
}

/// **Which limb the blade really crossed** — `F-032`, and the whole of the secondary hit zones.
///
/// Asked **only** where [`sweep`] already answered [`HitZone::Torso`], and only about the
/// descendants of the one titan that answered. That is rule 6's half of this feature: no
/// system in this file ever walks all the limbs in the world to answer a question about the
/// body in front of the blade — with nothing hit, this function is not called at all, and with
/// something hit it is four box tests.
///
/// The test is the same swept capsule the cast used, run against each box with `parry`'s own
/// `cast_shapes` instead of the spatial tree. That is not an approximation of the cast: it is
/// the identical algorithm avian calls one level up (`SpatialQuery::cast_shape` walks the BVH
/// and then dispatches to exactly this). What it does not need is a collider in the world, and
/// that is the point — see the module header.
///
/// Ties break on the **earlier** impact, so a blade that crosses an arm and a leg in one tick
/// reports the one it reached first.
pub fn limb_zone(
    children: &Query<&Children>,
    limbs: &Query<(&HitZoneOf, &GlobalTransform)>,
    root: Entity,
    blade: &Collider,
    delta_m: Vec3,
) -> Option<HitZone> {
    let here = Pose3::from_parts(Vec3::ZERO, Quat::IDENTITY);
    let options = ShapeCastOptions { max_time_of_impact: 1.0, ..default() };
    let mut best: Option<(f32, HitZone)> = None;
    for entity in children.iter_descendants(root) {
        let Ok((zone, at)) = limbs.get(entity) else {
            continue;
        };
        let (_, rotation, translation) = at.to_scale_rotation_translation();
        let half = zone.half_extent_m;
        let box_shape = Collider::cuboid(half.x * 2.0, half.y * 2.0, half.z * 2.0);
        // `vel1 = delta_m` over a unit of time, so `max_time_of_impact: 1.0` is exactly the
        // displacement of this tick — the same window `ShapeCastConfig::from_max_distance` gives
        // the cast above.
        let Ok(Some(hit)) = cast_shapes(
            &here,
            delta_m,
            &**blade.shape_scaled(),
            &Pose3::from_parts(translation, rotation),
            Vec3::ZERO,
            &**box_shape.shape_scaled(),
            options,
        ) else {
            continue;
        };
        if best.is_none_or(|(t, _)| hit.time_of_impact < t) {
            best = Some((hit.time_of_impact, zone.zone));
        }
    }
    best.map(|(_, zone)| zone)
}

/// From a collider entity up to the body that owns it — the **root entity**, its id and its
/// velocity.
///
/// The cortex is a grandchild of the root (`src/titan/rig.rs`), the body capsule sits on the
/// root itself — so this walks up until it finds the [`TitanId`], and gives up rather than
/// guessing. `ChildOf` is Bevy's own relation: reading it is not an edge into `titan`.
fn owner(
    parents: &Query<&ChildOf>,
    titans: &Query<(&TitanId, &Velocity)>,
    collider: Entity,
) -> Option<(Entity, TitanId, Velocity)> {
    let mut at = collider;
    loop {
        if let Ok((id, velocity)) = titans.get(at) {
            return Some((at, *id, *velocity));
        }
        at = parents.get(at).ok()?.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gear.ron`'s numbers, spelled out — so a change to the file shows up as a red test here
    /// and not as a silently different rate of wear.
    fn tuning() -> BladeTuning {
        BladeTuning {
            start_pairs: 5,
            wear_per_hit: 0.12,
            wear_torso_factor: 0.5,
            damage_per_m_s: 1.4,
            min_speed_m_s: 8.0,
            reach_m: 1.6,
            thickness_m: 0.12,
            swing_s: 0.35,
            active_from_s: 0.08,
            active_to_s: 0.22,
            cooldown_s: 0.30,
        }
    }

    #[test]
    fn f033_the_graze_costs_less_than_the_cut_because_every_kill_pays_both() {
        // The measured fact this direction rests on: a pass that ends in a nape reports
        // `[Torso, Cortex]`, because every titan is wider than his own neck
        // (`tests/combat.rs::f030_the_cortex_wins_over_the_body_it_hides_in`). A torso factor
        // above 1.0 would charge every kill more for the titan's shoulders than for the nape.
        let t = tuning();
        let cut = wear_of(&t, HitZone::Cortex);
        let graze = wear_of(&t, HitZone::Torso);
        assert!(
            graze < cut,
            "a graze ({graze}) costs at least as much as the cut ({cut}) — every successful pass \
             pays both, so this taxes winning"
        );
        assert!((cut - 0.12).abs() < 1e-6, "the cut costs {cut}, gear.ron says wear_per_hit");
        assert!((graze - 0.06).abs() < 1e-6, "the graze costs {graze}, expected 0.12 × 0.5");

        // One clean kill is one graze plus one cut, and that is the number the whole budget is
        // read off: 0.18 a kill, 5.5 kills to a pair.
        let mut b = Blades::fresh(5);
        spend(&mut b, &t, HitZone::Torso);
        spend(&mut b, &t, HitZone::Cortex);
        assert!((b.sharpness - 0.82).abs() < 1e-5, "one kill left {} sharpness", b.sharpness);
        assert_eq!(b.pairs_left, 5, "a kill must not cost a whole spare");
    }

    #[test]
    fn f033_a_spent_pair_draws_a_spare_and_the_last_one_leaves_the_harness_dry() {
        // `pairs_left` counts SPARES and `sharpness` is the pair in his hands
        // (`super::resupply::restock` hones that one first). So `start_pairs: 2` is three pairs'
        // worth of fighting, and the harness is only dry when the third one is gone.
        let t = tuning();
        let mut b = Blades::fresh(2);

        // Eight cortex cuts is 0.96 — the pair is nearly through and no spare has been drawn.
        for _ in 0..8 {
            assert!(!spend(&mut b, &t, HitZone::Cortex), "a spare was drawn too early: {b:?}");
        }
        assert!((b.sharpness - 0.04).abs() < 1e-5, "sharpness is {}", b.sharpness);
        assert_eq!(b.pairs_left, 2);

        // The ninth finishes it. A pair carries no debt into the next one: the 0.08 it could not
        // pay is simply gone, and the fresh pair starts at 1.0.
        assert!(spend(&mut b, &t, HitZone::Cortex), "the ninth cut did not draw a spare: {b:?}");
        assert_eq!(b.pairs_left, 1);
        assert_eq!(b.sharpness, 1.0);
        assert!(!b.is_broken());

        // Run the last two pairs out. The harness ends dry and STAYS dry — no negative
        // sharpness, no wrapping `u8`, and `is_broken()` is what `cut` reads to cast nothing.
        for _ in 0..40 {
            spend(&mut b, &t, HitZone::Cortex);
        }
        assert_eq!(b.pairs_left, 0);
        assert_eq!(b.sharpness, 0.0, "a dry harness must sit at exactly zero, not below it");
        assert!(b.is_broken());
        assert!(!spend(&mut b, &t, HitZone::Cortex), "a dry harness handed out another spare");
    }

    #[test]
    fn f033_a_nonsense_wear_number_neither_freezes_the_blade_nor_makes_it_nan() {
        // The mirror of `super::resupply::restock`'s guard. A `NaN` sharpness is never `<= 0.0`,
        // so `is_broken()` would be false forever and the harness would silently stop wearing —
        // a bug that looks exactly like the one this whole feature was built to remove.
        for bad in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            let t = BladeTuning { wear_per_hit: bad, ..tuning() };
            let mut b = Blades::fresh(5);
            assert!(!spend(&mut b, &t, HitZone::Cortex), "wear_per_hit = {bad} drew a spare");
            assert_eq!(b.sharpness, 1.0, "wear_per_hit = {bad} moved the pair to {}", b.sharpness);
            assert!(!b.is_broken());
        }
    }

    #[test]
    fn the_blade_reaches_outward_and_not_forward() {
        // Looking at −Z, the right blade lies on +X and the left on −X — an arm's length, not
        // a lance (`gear.ron: reach_m`).
        let (a, b) = blade_segment(Vec3::ZERO, Vec3::NEG_Z, Side::Right, 1.6, 1.6);
        assert_eq!(a, Vec3::new(0.0, 1.6, 0.0), "the blade hangs at eye height");
        assert!((b - Vec3::new(1.6, 1.6, 0.0)).length() < 1e-5, "right blade at {b:?}");
        let (_, b) = blade_segment(Vec3::ZERO, Vec3::NEG_Z, Side::Left, 1.6, 1.6);
        assert!((b - Vec3::new(-1.6, 1.6, 0.0)).length() < 1e-5, "left blade at {b:?}");
        assert!((a - b).length() > 0.0);
    }

    #[test]
    fn looking_straight_down_still_leaves_a_blade() {
        // `look × Y` is zero on the vertical axis. Without the fallback the capsule collapses
        // to a sphere at the hand and the cut silently loses its whole reach.
        let (a, b) = blade_segment(Vec3::ZERO, Vec3::NEG_Y, Side::Right, 1.6, 1.6);
        assert!((b - a).length() > 1.5, "the blade collapsed: {a:?} .. {b:?}");
    }

    #[test]
    fn the_closing_speed_is_a_projection_and_not_a_length() {
        // The failure this catches: `|v|`. A player at 30 m/s flying PAST a titan closes on
        // nothing, and `damage_per_m_s * 30` would be a lie about what happened.
        let past = closing_speed(Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0));
        assert!((past - 30.0).abs() < 1e-4, "head-on gives the full speed, got {past}");

        // Flying at 30 along +X while the titan runs away at 11 along +X: 19 m/s of closing.
        let chasing = closing_speed(
            Vec3::new(30.0, 0.0, 0.0),
            Vec3::new(11.0, 0.0, 0.0),
            Vec3::new(30.0, 0.0, 0.0),
        );
        assert!((chasing - 19.0).abs() < 1e-4, "chasing gives {chasing}, expected 19");

        // And the titan running INTO the blade adds his speed.
        let head_on = closing_speed(
            Vec3::new(30.0, 0.0, 0.0),
            Vec3::new(-11.0, 0.0, 0.0),
            Vec3::new(30.0, 0.0, 0.0),
        );
        assert!((head_on - 41.0).abs() < 1e-4, "head-on gives {head_on}, expected 41");

        // Moving away is never negative damage.
        let away = closing_speed(Vec3::new(-30.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(-30.0, 0.0, 0.0));
        assert!(away >= 0.0);
        let sideways = closing_speed(Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0));
        assert!(sideways.abs() < 1e-4, "a perpendicular pass closes on nothing, got {sideways}");
    }
}
