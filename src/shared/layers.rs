//! Collision layers — **one place, so that the collider and the cast agree.**
//!
//! This module exists for one failure mode. `titan` puts the cortex sphere on a layer;
//! `combat` casts its blade with a filter for that layer. If the two jobs each invent their
//! own constant, nothing breaks loudly: the collider is spawned, the cast runs, the cast
//! simply never returns anything, and the cut silently never lands. A seam with two spellings
//! is worse than no seam.
//!
//! **Bit 0 is not ours.** avian reserves the first bit for its default layer
//! (`avian3d-0.7.0/src/collision/collider/layers.rs:31-32` and `LayerMask::DEFAULT` at :121),
//! and a collider spawned **without** a `CollisionLayers` component is on exactly that bit
//! (`CollisionLayers::DEFAULT` at :373-376). A spatial query keeps an entity when
//! `memberships & filter.mask != NONE` (`spatial_query/query_filter.rs:97-101`). So if one of
//! our layers sat on bit 0, every untagged wall in the city would answer a cortex-filtered
//! cast — the exact bug this file prevents. The four constants therefore start at bit 1, and
//! a test says so.
//!
//! **How they are used.** The values are avian `LayerMask`s, so they drop straight into both
//! ends of the seam without a conversion:
//!
//! ```ignore
//! // the writer (titan): the cortex is a member of its layer, and collides with nothing
//! CollisionLayers::new(LAYER_TITAN_CORTEX, LayerMask::NONE)
//! // the reader (combat): a cast that sees the cortex and nothing else
//! SpatialQueryFilter::from_mask(LAYER_TITAN_CORTEX)
//! ```
//!
//! ⚠️ **Since 2026-08-19 exactly one thing wears a `CollisionLayers` component: the player**
//! ([`PLAYER_COLLIDES_WITH`], attached in `player::spawn_player`, so that two players cannot
//! shove each other — F-163a). Everything else still spawns untagged, i.e. on avian's default
//! bit: the city (`src/world/map.rs`) and every titan collider. A cortex-filtered cast is
//! correct anyway (untagged geometry does not match), but [`LAYER_WORLD`] is still a **label
//! without a wearer**. It stands here so that whoever does attach it does not invent a fifth
//! name.

use avian3d::prelude::{LayerMask, PhysicsLayer};

/// The bit assignment, in avian's own idiom.
///
/// The derive is what avian documents for this (`layers.rs:38-51`); it hands out `1 << index`
/// per variant and moves the `#[default]` variant to index 0
/// (`avian_derive-0.2.3/src/lib.rs:90-107`). This enum is the **authority over the bits** —
/// the `LAYER_*` constants below are the API everybody uses, and a test binds the two
/// together so they cannot drift.
///
/// `Default` is a variant of its own and deliberately unused by us: it is avian's reserved
/// first bit, not a layer of ours.
#[derive(PhysicsLayer, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GameLayer {
    /// avian's reserved default bit. **Not one of ours** — see the module doc.
    #[default]
    Default,
    /// Static geometry: ground, houses, roofs. What a hook may anchor to and a body stands on.
    World,
    /// A player body. Players do not collide with each other (`docs/multiplayer.md`, F-163a),
    /// which is a *filter* decision — the membership still has to exist to be filterable.
    Player,
    /// A titan's limbs and torso. Solid, and **not** a kill zone.
    TitanBody,
    /// The cortex. A `Sensor` sphere hidden **inside** the body silhouette, which is why the
    /// layer filter is the entire point here: an unfiltered cast returns the torso in front of
    /// it and never the cortex.
    TitanCortex,
}

/// Static world geometry. See [`GameLayer::World`].
pub const LAYER_WORLD: LayerMask = LayerMask(1 << 1);

/// Player bodies. See [`GameLayer::Player`].
pub const LAYER_PLAYER: LayerMask = LayerMask(1 << 2);

/// Titan limbs and torso. See [`GameLayer::TitanBody`].
pub const LAYER_TITAN_BODY: LayerMask = LayerMask(1 << 3);

/// The cortex — the only place a titan dies. See [`GameLayer::TitanCortex`].
pub const LAYER_TITAN_CORTEX: LayerMask = LayerMask(1 << 4);

/// **What a player's collider collides with: everything except another player.**
///
/// The bible's ground rule F-163a (`docs/multiplayer.md`, `src/squad/mod.rs`): *no collision
/// between players* — *"at this speed the single biggest source of frustration there is"*.
/// Two players have to be able to pass through each other at full speed.
///
/// Measured before it existed: two bodies standing 0.1 m apart shoved each other **0.194 m
/// each in one second** of simulation, with nobody pressing a key
/// (`tests/multiplayer.rs::f163a_two_players_in_the_same_spot_do_not_push_each_other`).
///
/// ⚠️ It changes the **contact** filter, and that alone changed what a ray sees — the
/// opposite of what this paragraph claimed until 2026-08-19. Two players who no longer push
/// each other **stay overlapping**, so a ray from one player's eye now starts *inside* the
/// other's capsule and `solid: true` answers it at distance 0. The fix is not to give the
/// shove back; it is [`AIM_RAY_SEES`] (`docs/BUGS.md` B-010).
pub const PLAYER_COLLIDES_WITH: LayerMask = LayerMask(LayerMask::ALL.0 & !LAYER_PLAYER.0);

/// **What a hook ray is allowed to find: everything except another player.**
///
/// A team mate is **not** a surface. He is not anchorable (a player carries no
/// `shared::Body`, so `vector::aim` resolves him to `anchorable: false`), which means an
/// unfiltered ray that lands on him does not produce an anchor — it produces a *dead* aim
/// point that hides the wall behind him. That is the shape of `B-007`, where a titan hid a
/// wall the same way, and the bible's ground rule F-163a is the reason it now happens
/// constantly: since players stopped colliding they stand *inside* each other
/// (`docs/multiplayer.md`, `PLAYER_COLLIDES_WITH`).
///
/// So the rope ray treats a player like air: it passes through him and anchors on what is
/// really out there. Bit 0 (avian's default, worn by every untagged wall and every titan
/// collider) stays in the mask, so nothing but a player answers differently.
///
/// Used by `vector::aim::cast` and by `vector::hook::anchorable_beyond_reach` — the two
/// unfiltered casts in the game. Measured: `tests/vector_aiming.rs::
/// f002_a_side_ray_that_finds_nothing_falls_back_to_the_centre_ray` reported the aim point at
/// `Vec3(0.0, 1.5999687, 0.0)` — the caster's own eye height, distance ~0 — before this mask.
pub const AIM_RAY_SEES: LayerMask = LayerMask(LayerMask::ALL.0 & !LAYER_PLAYER.0);

// ⚠️ **There is deliberately no `LAYER_TITAN_LIMB`, and that was measured** (`F-032`,
// 2026-08-19). A titan's arm and leg hit zones are `shared::HitZoneOf` boxes with **no collider
// at all**: `vector::aim` casts its hook ray **unfiltered** on purpose (`src/vector/aim.rs:31`,
// "hit first, then check anchorable"), and it resolves the carrier with
// `bodies.get(hit.entity)` on the *collider* entity — no walk up the hierarchy. An arm box
// carries no `Body`, and the arm sticks out of the root capsule (`w/2 .. 3w/4` against a radius
// of `w/2`), so the first version of this feature put a real `Sensor` collider there and
// **broke `F-029`**: the ray hit the arm before the capsule, the lookup missed, and
// `tests/titan.rs::f029_a_rope_bites_a_walking_titan_and_rides_him` reported *"the rope found no
// anchor on a titan 30 m away and dead in the crosshair"*. A layer cannot fix that — avian's
// default `SpatialQueryFilter` has `mask: LayerMask::ALL`, so **every** collider answers an
// unfiltered ray whatever its membership. See `blades::cut::limb_zone` for what was built
// instead.

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_FOUR: [(&str, LayerMask); 4] = [
        ("world", LAYER_WORLD),
        ("player", LAYER_PLAYER),
        ("titan_body", LAYER_TITAN_BODY),
        ("titan_cortex", LAYER_TITAN_CORTEX),
    ];

    #[test]
    fn a_hook_ray_sees_the_world_and_a_titan_but_never_a_player() {
        // The mask is what `vector::aim` hands avian; avian keeps an entity when
        // `memberships & mask != NONE` (`query_filter.rs:97-101`). So this is the whole
        // behaviour of the fix, spelled as four bit tests.
        assert_eq!(AIM_RAY_SEES.0 & LAYER_PLAYER.0, 0, "a team mate still blocks the rope");
        assert_ne!(
            AIM_RAY_SEES.0 & LayerMask::DEFAULT.0,
            0,
            "bit 0 is every untagged wall in the city and every titan collider — dropping it \
             would make the hook find nothing at all"
        );
        assert_ne!(AIM_RAY_SEES.0 & LAYER_TITAN_BODY.0, 0, "a titan is anchorable (F-029)");
        assert_ne!(AIM_RAY_SEES.0 & LAYER_WORLD.0, 0, "the label's wearer, when it gets one");
    }

    #[test]
    fn the_four_layers_are_four_distinct_non_zero_bits() {
        for (name, mask) in ALL_FOUR {
            assert_ne!(mask.0, 0, "{name} is the empty mask — it matches nothing at all");
            assert_eq!(
                mask.0.count_ones(),
                1,
                "{name} is not a single bit ({:#b}) — a layer is one bit, a mask is many",
                mask.0
            );
        }
        for (i, (a_name, a)) in ALL_FOUR.iter().enumerate() {
            for (b_name, b) in ALL_FOUR.iter().skip(i + 1) {
                assert_ne!(
                    a.0, b.0,
                    "{a_name} and {b_name} share a bit — a filter for one returns the other"
                );
            }
        }
    }

    #[test]
    fn no_layer_sits_on_avians_reserved_default_bit() {
        // A collider without a `CollisionLayers` component is a member of `LayerMask::DEFAULT`
        // (avian3d-0.7.0/src/collision/collider/layers.rs:373-376). Sharing that bit would
        // make every untagged wall in the city answer a cortex-filtered cast.
        for (name, mask) in ALL_FOUR {
            assert_eq!(
                (mask & LayerMask::DEFAULT).0,
                0,
                "{name} overlaps avian's default bit — untagged geometry would match it"
            );
        }
    }

    #[test]
    fn the_constants_agree_with_the_derived_enum() {
        // The whole point of the file: two spellings of the same layer must be the same bit.
        assert_eq!(LAYER_WORLD.0, GameLayer::World.to_bits());
        assert_eq!(LAYER_PLAYER.0, GameLayer::Player.to_bits());
        assert_eq!(LAYER_TITAN_BODY.0, GameLayer::TitanBody.to_bits());
        assert_eq!(LAYER_TITAN_CORTEX.0, GameLayer::TitanCortex.to_bits());
        assert_eq!(
            GameLayer::default().to_bits(),
            LayerMask::DEFAULT.0,
            "the `#[default]` variant must stay avian's reserved bit 0"
        );
    }
}
