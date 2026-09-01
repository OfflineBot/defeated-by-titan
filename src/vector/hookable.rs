//! **What a hook may take hold of** — `Q-078`, and it is ONE predicate with ONE call per ray.
//!
//! The user, 2026-08-27:
//!
//! > *„es soll auf jeglicher oberflqche einhaken. nicht an hardcoded punkten etc!"*
//!
//! and, minutes later, the sentence that decides the shape of this file rather than its
//! answer:
//!
//! > *„spaeter soll man auch bestimmte sachen toggeln koennen. also an bestimmte sachen ran
//! > haken an andere nicht aber grundsetzlich erstmal ales!"*
//!
//! So `F-003 Getaggte Ankerflaechen` is cancelled **as a rule** and kept **as data**. Its
//! `maps.ron: anchorable` column is not dead and must not be deleted: it is the vocabulary a
//! future switch selects on, and deleting it would destroy the thing the switch needs
//! (`docs/QUESTIONS.md` Q-078).
//!
//! ## What this replaced, and why the replacement is not "one `if` removed"
//!
//! Until today `vector::aim::cast` wrote `anchorable: body.mask.contains(ANCHORABLE)` and
//! `vector::hook::anchorable_beyond_reach` asked the same bit a second time. Deleting the
//! condition would have made everything hookable in one line — and thrown the column away
//! with it, because nothing would read it any more and the next map would stop writing it.
//! What stands here instead answers *"is this KIND of surface hookable right now"*, with
//! every kind answering `true`. Restoring `F-003` is then one value
//! ([`HookableSurfaces::TAGGED_ONLY`]) and no code at all — which is exactly the rollback
//! point Q-078 asked for.
//!
//! ## Why this is not a settings screen
//!
//! He said *„später"*. The requirement today is that the toggle is **cheap**, not that it
//! exists — so there is a resource with a default, and nothing in `menu/` knows about it. The
//! day it becomes a knob it moves into a RON file like every other game value
//! (`CLAUDE.md` rule 2); until then it is a Rust default that is `true` for everything, which
//! is a statement no file has to carry.
//!
//! ## Why a `Resource` is admissible here (`docs/multiplayer.md` rule 3)
//!
//! It is **world configuration, not player state**: it decides what the map is made of, it is
//! the same for everybody in a session, and it has to be, or two clients would simulate
//! different worlds. That is the same category as `data::GameData`, and the opposite of
//! `shared::PlayerSettings` — which is a *local* preference and therefore may never decide
//! another player's simulation. The day the level designer's switch travels, it travels with
//! the map, not per player.

use bevy::prelude::*;

use crate::shared::{Body, BodyMask};

/// The category vocabulary a hook is filtered on. **Today it is the surviving `F-003` tag,
/// and nothing else.**
///
/// ⚠️ **Two variants is not the ambition, it is the reach.** The categories a future switch
/// will most likely want — *titan bodies*, *the ground* — cannot be told apart at the cast
/// site yet: a titan's root capsule and a house both arrive as a `shared::Body` with
/// `SOLID | ANCHORABLE` and nothing distinguishes them, and `vector` has no edge to `titan`
/// in the allow list of `docs/architecture.md` to go and ask. Adding one is a variant here, a
/// line in [`Self::of`], and nothing else — that is the whole point of the shape. Whoever
/// needs it either puts a marker into `shared` or buys the edge with a reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// `maps.ron: anchorable: true` — what `F-003` used to call an anchor surface, and what
    /// `shared::AnchorSurface` / `BodyMask::ANCHORABLE` still mark. 2901 of 2901 blocks on
    /// Ashgate, 14 of 36 placed rows on the graybox.
    Tagged,
    /// Solid geometry carrying no tag: the graybox's 22 `anchorable: false` rows — the ground
    /// slab, the wall in front of the brick-red house, the aqueduct's twenty columns.
    /// **Hookable since Q-078**, and the only reason this variant is not merged into the one
    /// above is that it is the switch's handle.
    Untagged,
    /// **Water** — `BodyMask::WATER`. The user, 2026-08-29, asked what a hook does to the
    /// river and answered: *„Nein — Wasser haelt keinen Haken."*
    ///
    /// ⭐ **The first category this switch has ever turned off**, and it is exactly the use
    /// Q-078 was left open for: *„spaeter soll man auch bestimmte sachen toggeln koennen. also
    /// an bestimmte sachen ran haken an andere nicht"*. It is one variant, one line in
    /// [`Self::of`] and one clause in [`HookableSurfaces::default`] — no second flag, no
    /// per-body `bool`, nothing in `maps.ron`.
    Water,
}

impl SurfaceKind {
    /// Every kind, so that a test cannot forget the one that was added last.
    pub const ALL: [SurfaceKind; 3] =
        [SurfaceKind::Tagged, SurfaceKind::Untagged, SurfaceKind::Water];

    /// Which kind a body is. **The one place the `ANCHORABLE` bit is still read**, so that
    /// the tag has exactly one meaning in the game and not two spellings of it.
    pub fn of(body: &Body) -> Self {
        // ⚠️ **Water is asked FIRST**, and the order is the rule and not a style: a volume that
        // was ever tagged anchorable as well would otherwise come back `Tagged` and hold a
        // hook. The one answer the user gave about water is that it holds none, so no other
        // bit may overrule it.
        if body.mask.contains(BodyMask::WATER) {
            Self::Water
        } else if body.mask.contains(BodyMask::ANCHORABLE) {
            Self::Tagged
        } else {
            Self::Untagged
        }
    }

    const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// For a log line and an assertion message. Not `Display`: it is a debug word, not text
    /// the player ever sees.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tagged => "tagged",
            Self::Untagged => "untagged",
            Self::Water => "water",
        }
    }
}

/// **The switch.** Which [`SurfaceKind`]s a hook may take hold of right now.
///
/// A bit set rather than one `bool` per kind, for the reason `shared::BodyMask` gives: adding
/// a purpose is a constant, not a field, and several purposes can be asked in one comparison.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookableSurfaces(u32);

impl Default for HookableSurfaces {
    /// *„grundsetzlich erstmal ales"* — everything **except water**.
    ///
    /// The exception is his too, and it is the newer of the two sentences (2026-08-29,
    /// against 2026-08-27): *„Nein — Wasser haelt keinen Haken."* His instruction beats my
    /// derivation and beats his own earlier number (`CLAUDE.md`, and `Q-002`), so the default
    /// is `EVERYTHING` minus one bit and not a new constant beside it — [`Self::EVERYTHING`]
    /// still means everything, and it is still what a *new* kind arrives inside of.
    fn default() -> Self {
        Self::EVERYTHING.with(SurfaceKind::Water, false)
    }
}

impl HookableSurfaces {
    /// Every kind, **including any that is added after this line was written.** That is
    /// deliberate and it is `u32::MAX` for exactly that reason: a new [`SurfaceKind`] must
    /// arrive hookable, or the day somebody splits *ground* out of [`SurfaceKind::Untagged`]
    /// the ground silently stops holding a rope and no test in the tree says so.
    pub const EVERYTHING: Self = Self(u32::MAX);

    /// **`F-003` as it stood until 2026-08-27** — the rollback point Q-078 names. Nothing in
    /// the game sets this today; it exists so that the old rule is one *value* away and not a
    /// code change, and so that a test can prove the map data still carries the distinction.
    pub const TAGGED_ONLY: Self = Self(SurfaceKind::Tagged.bit());

    /// Nothing at all. A hook finds a surface and refuses it, every time — the shape a level
    /// with no anchors would have.
    pub const NOTHING: Self = Self(0);

    pub const fn allows(self, kind: SurfaceKind) -> bool {
        self.0 & kind.bit() != 0
    }

    /// One kind switched on or off. The whole of *„an bestimmte sachen ran haken an andere
    /// nicht"* whenever it arrives.
    pub const fn with(self, kind: SurfaceKind, on: bool) -> Self {
        if on {
            Self(self.0 | kind.bit())
        } else {
            Self(self.0 & !kind.bit())
        }
    }
}

/// **The predicate.** May a hook take hold of what a ray landed on?
///
/// `body` is what `Query<(&Body, ..)>::get(hit.entity)` returned — `None` when the hit entity
/// carries no [`Body`] at all.
///
/// ⚠️ **`None` is `false`, and that is not a category decision.** An entity without a `Body`
/// is not a surface with a kind, it is a hit with no hull and no stable carrier: `vector::
/// hook::anchor_target` needs a `BodyId` to hang the anchor in the carrier's frame (`B-001`),
/// and a rope on an entity nothing indexes drifts the moment the entity moves. Returning
/// `true` here would produce a hook that fires and then releases as `NoCarrier` one tick
/// later, which is worse to play than a hook that does not fire. It is also *visibly* wrong
/// rather than silently hookable, which is what `vector::aim::cast` says about the same
/// lookup failing.
pub fn is_hookable(rules: HookableSurfaces, body: Option<&Body>) -> bool {
    match body {
        Some(b) => rules.allows(SurfaceKind::of(b)),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(anchorable: bool) -> Body {
        let mask = if anchorable {
            BodyMask::SOLID.with(BodyMask::ANCHORABLE)
        } else {
            BodyMask::SOLID
        };
        Body { half_size_m: Vec3::splat(1.0), mask }
    }

    #[test]
    fn f003_everything_solid_is_hookable_today_whatever_the_map_tagged() {
        // Q-078, and the reason this file exists. **Both** solid kinds, not one: a predicate
        // tested only against the tagged surface is a test of the rule that was just deleted.
        //
        // ⚠️ Water is deliberately not in this loop and has its own test below. It went out of
        // it on 2026-09-01 with the river, when the user answered *„Nein — Wasser haelt keinen
        // Haken."* — the first time this switch has ever been anything but wide open.
        let rules = HookableSurfaces::default();
        for kind in [SurfaceKind::Tagged, SurfaceKind::Untagged] {
            assert!(rules.allows(kind), "{} refuses a hook by default", kind.name());
        }
        assert!(is_hookable(rules, Some(&body(true))));
        assert!(is_hookable(rules, Some(&body(false))));
    }

    fn water_body() -> Body {
        // Exactly what a water body would carry: solid enough to be hit, marked as water.
        Body { half_size_m: Vec3::splat(1.0), mask: BodyMask::SOLID.with(BodyMask::WATER) }
    }

    #[test]
    fn f003_water_holds_no_hook_and_it_is_this_switch_that_refuses_it() {
        // The user, 2026-08-29: *„Nein — Wasser haelt keinen Haken."* — the FIRST category the
        // Q-078 switch turns off, and the whole of the mechanism he asked for in the same
        // breath as *„spaeter soll man auch bestimmte sachen toggeln koennen"*.
        let rules = HookableSurfaces::default();
        assert!(!rules.allows(SurfaceKind::Water), "the default lets a hook bite the river");
        assert!(!is_hookable(rules, Some(&water_body())), "a water body answered a hook");
        // And it is the SWITCH that refuses, not the absence of a category: turn the bit back
        // on and the very same body holds. Without this line the test above would still pass
        // if `SurfaceKind::of` had simply stopped recognising water.
        let permissive = rules.with(SurfaceKind::Water, true);
        assert!(is_hookable(permissive, Some(&water_body())), "the switch does not reach water");
    }

    #[test]
    fn f003_the_water_bit_outranks_the_anchorable_bit_on_one_body() {
        // A volume that was ALSO tagged anchorable — the shape a copy-pasted map row would
        // have. Water is asked first, so the tag cannot buy it back.
        let both = Body {
            half_size_m: Vec3::splat(1.0),
            mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE).with(BodyMask::WATER),
        };
        assert_eq!(SurfaceKind::of(&both), SurfaceKind::Water);
        assert!(!is_hookable(HookableSurfaces::default(), Some(&both)));
    }

    #[test]
    fn f003_a_kind_that_nobody_has_written_yet_arrives_hookable() {
        // `EVERYTHING` is `u32::MAX` and not a list of the two variants that exist today. The
        // day somebody splits `Ground` out of `Untagged`, the ground has to keep holding a
        // rope without anybody remembering to add it here — *„grundsätzlich erstmal alles"*.
        // Bit 7 stands in for that future variant; no `SurfaceKind` has it today.
        assert_ne!(HookableSurfaces::EVERYTHING.0 & (1 << 7), 0);
    }

    #[test]
    fn f003_the_tag_still_separates_the_two_kinds_so_the_switch_has_a_handle() {
        // The half of `F-003` that survives: the DATA still tells the two apart, so a switch
        // can act on it. If `SurfaceKind::of` ever stopped reading the mask, everything would
        // still be hookable — and the tag column in `maps.ron` would become unreachable
        // without a single test going red.
        assert_eq!(SurfaceKind::of(&body(true)), SurfaceKind::Tagged);
        assert_eq!(SurfaceKind::of(&body(false)), SurfaceKind::Untagged);
        assert!(is_hookable(HookableSurfaces::TAGGED_ONLY, Some(&body(true))));
        assert!(!is_hookable(HookableSurfaces::TAGGED_ONLY, Some(&body(false))));
        assert!(!is_hookable(HookableSurfaces::NOTHING, Some(&body(true))));
    }

    #[test]
    fn f003_a_hit_without_a_body_holds_nothing_under_every_setting() {
        // Not a category — a hit with no hull and no carrier (`B-001`). No value of the
        // switch may turn it into an anchor.
        for rules in [
            HookableSurfaces::EVERYTHING,
            HookableSurfaces::TAGGED_ONLY,
            HookableSurfaces::NOTHING,
        ] {
            assert!(!is_hookable(rules, None));
        }
    }

    #[test]
    fn f003_one_kind_switches_without_moving_the_others() {
        // `with` is what a settings screen will call. Off, then on again, and the neighbour
        // never moves — the failure mode of a hand-written bit mask.
        let off = HookableSurfaces::default().with(SurfaceKind::Untagged, false);
        assert!(!off.allows(SurfaceKind::Untagged));
        assert!(off.allows(SurfaceKind::Tagged), "switching one kind off moved the other");
        let back = off.with(SurfaceKind::Untagged, true);
        assert!(back.allows(SurfaceKind::Untagged) && back.allows(SurfaceKind::Tagged));
    }
}
