//! What a swapped model brought with it — **the empties, in metres, on the entity.**
//!
//! ## Why this type lives in `shared/` and not in `render/`
//!
//! `render` **writes** it (`render::model::read_the_models_anchors` walks the glTF instance),
//! and `titan` **reads** it: the `cortex` empty out of the file is where the titan dies
//! (`F-030`), and until somebody reads it a swapped model *renders* in the right place and
//! *dies* in the computed one. A `render`-private component would force
//! `titan -> render` into the allow list of `docs/architecture.md` purely so that a rig can ask
//! where its own kill zone is — and an allow list that grows for reasons like that stops being
//! a rule. The same argument put [`TitanState`](super::TitanState) and
//! [`StateClock`](super::StateClock) here.
//!
//! **Who writes stands in the authority table in `docs/architecture.md`, not in the type**:
//! `render::model` is the one writer, everybody else reads.

use bevy::prelude::*;
use std::collections::BTreeMap;

/// The empties a model may carry, exactly as `docs/models.md` names them.
///
/// **Names, not numbers** — this is a contract with Blender, so it belongs in code the way an
/// axis convention does. A typo in Blender therefore does not raise an error, it produces a
/// *missing* anchor; that is why `render::model::read_the_models_anchors` shouts about a
/// missing `cortex` instead of silently doing nothing.
pub const ANCHOR_NAMES: [&str; 8] =
    ["cortex", "hit.min", "hit.max", "hook.l", "hook.r", "hand.l", "hand.r", "eye"];

/// The one anchor whose absence is a gameplay bug and not a missing detail (`F-030`).
pub const CORTEX_ANCHOR: &str = "cortex";

/// **The open family.** Everything named `hook.<anything>` is an anchorable point, whatever
/// comes after the dot.
///
/// [`ANCHOR_NAMES`] is a **closed** list on purpose — `cortex`, `eye`, `hand.l` are rig
/// landmarks, one per body, and a typo in Blender must show up as a missing one rather than as
/// a new one. Hook points are the opposite kind of thing and the 2026-08-18 drop proves it:
/// **565 `hook.*` empties across 207 of 278 files, under 212 distinct names, 130 of which occur
/// in exactly one file** — `hook.gesims_15..105` (a cornice every 15 m up a 120 m wall),
/// `hook.traufe` (eaves), `hook.first` (ridge), `hook.krone`, `hook.spitze` (spire),
/// `hook.wurzelbogen_quer` on one single tree. Only `hook.l`/`hook.r` were on the whitelist, so
/// **439 across 144 files were dropped at load** — the entire anchorable surface of the
/// architecture kit, in a game whose one verb is a grappling hook.
///
/// A closed list cannot hold that: it would have to be re-edited for every eaves a modeller
/// names, and the failure mode of forgetting is silent. So the rule is the prefix, and the
/// **name** carries the meaning for whoever consumes it later.
pub const HOOK_PREFIX: &str = "hook.";

/// Whether an empty out of a `.glb` is an anchor the loader keeps.
///
/// One place, so that `render::model` (which walks the instance) and any future check over the
/// pack cannot disagree about what an anchor is.
pub fn is_anchor_name(name: &str) -> bool {
    ANCHOR_NAMES.contains(&name) || name.starts_with(HOOK_PREFIX)
}

/// The anchors that came **out of the model**, in the model root's own space, in metres.
///
/// Inserted on **every** entity with a `ModelName`, empty for a primitive. That is deliberate:
/// a reader asks [`ModelAnchors::get`], gets `None`, and uses the computed rig position — one
/// code path for both worlds, which is the difference between a switch and a rebuild
/// (`docs/models.md`).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ModelAnchors(pub BTreeMap<String, Vec3>);

impl ModelAnchors {
    /// The anchor the model brought, or `None` — *never* a substitute. A cortex that quietly
    /// becomes `Vec3::ZERO` is a kill zone between the feet.
    pub fn get(&self, anchor: &str) -> Option<Vec3> {
        self.0.get(anchor).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// "This entity is the logical model `name`." — **and since 2026-08-19 that includes a house.**
///
/// ## Why this type moved out of `render/` on 2026-08-19
///
/// `render` is still its only **reader** — [`ModelName`] is what `render::model::spawn_models`
/// answers with a glTF scene. What changed is who **writes** it: `world::map` plans a model per
/// generated house (`BlockPlan::model`), and a `render`-private component would have forced a
/// `world -> render` edge into the allow list of `docs/architecture.md` for nothing but a
/// string. That is exactly the trade this folder exists to refuse — the same argument that put
/// [`ModelAnchors`] and every message type here (`shared`, module header). The district went
/// undressed for a day because the type was on the wrong side of that line
/// (`docs/NEXT.md` §2C).
/// The component is written by `render::model::name_the_titans_model` out of `titan.ron`, and it may be put
/// on any entity by hand. **It carries no file name** — the registry decides that.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ModelName {
    /// The key into `art.ron: models`.
    pub name: String,
    /// Where `scale.ron` says the cortex sits for this entity, in meters above its origin.
    /// `None` for everything that is not a titan.
    ///
    /// Only used to **check** a model that brings its own `cortex` empty. `scale.ron` stays
    /// the one truth; this is the yardstick a swapped model is held against.
    pub cortex_height_m: Option<f32>,
    /// How tall `scale.ron` says this entity is, in meters. `None` for everything that is not
    /// a titan.
    ///
    /// **This is what lets one logical name serve two size classes.** `titan.ron` gives
    /// `titan_husk` to three medium kinds (10.0 m) *and* to two small ones (4.2 m), the way
    /// the primitive rig has always been built — one shape at the class height. A `.glb` is
    /// authored at exactly one height, so `render::model::fit_to_class` brings it to this one.
    pub height_m: Option<f32>,
    /// **Where, in this entity's own frame, the model's floor belongs** — in metres on y.
    ///
    /// The whole drop is authored **on its feet**: `hit.min.y = 0` on 204 of 278 files, and on
    /// every one the world puts down (`a-042` bodies, `a-083` houses, `a-089` ruins, `a-090`
    /// rubble). The 74 that differ are *parts* authored in their parent rig's space — a head at
    /// y = 8.9, a cloak at y = 0.15 — and moving those would be the bug, not the fix.
    ///
    /// So the field is an `Option` and the two answers are different sentences:
    ///
    /// * `None` — **the entity's origin is the model's origin.** Draw the file where it was
    ///   authored. That is a titan (its origin is the ground under it, which is the same frame
    ///   `cortex_height_m` is measured in), the blades in a hand, and every hand-placed model.
    /// * `Some(y)` — bring the model's authored floor to `y`. A **dressed block** passes
    ///   `-size_m.y * 0.5`, because a block's origin is the box's **centre** while the model's
    ///   is its feet — without it the building floats by half its own height (5.75 m for
    ///   `a-083-fachwerkhaus-gross`), which is what the user saw on 2026-08-19:
    ///   *„zudem sind die gebäude nicht auf dem boden sondern in der luft!"*
    ///
    /// ⚠️ **It moves the anchors with the mesh.** `render::model::read_the_models_anchors`
    /// applies one offset to the scene child's transform *and* to every entry of
    /// [`ModelAnchors`], so what the rope bites and what the eye sees stay one thing. A version
    /// of this fix that moved only the drawing would be `FIND-098`/`-099`/`-127`/`-129` again.
    pub feet_y_m: Option<f32>,
}

impl ModelName {
    pub fn new(name: impl Into<String>) -> Self {
        ModelName { name: name.into(), cortex_height_m: None, height_m: None, feet_y_m: None }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_anchor_is_absent_and_never_the_origin() {
        // The whole reason the type answers `Option` instead of `Vec3`: `(0,0,0)` on a titan is
        // a kill zone between the feet, and it looks like a physics bug rather than a
        // modelling mistake.
        let empty = ModelAnchors::default();
        assert!(empty.is_empty());
        assert_eq!(empty.get(CORTEX_ANCHOR), None);

        let mut some = ModelAnchors::default();
        some.0.insert(CORTEX_ANCHOR.to_string(), Vec3::new(0.0, 8.9, 0.4));
        assert_eq!(some.get(CORTEX_ANCHOR), Some(Vec3::new(0.0, 8.9, 0.4)));
        assert_eq!(some.get("hook.l"), None, "one anchor present must not imply the others");
    }

    #[test]
    fn every_hook_point_is_an_anchor_and_the_rig_landmarks_stay_a_closed_list() {
        // The pack's own names, measured 2026-08-18 — 212 distinct ones, most of them in a
        // single file. Whitelisting these was never going to happen.
        for name in ["hook.l", "hook.traufe", "hook.gesims_105", "hook.wurzelbogen_quer"] {
            assert!(is_anchor_name(name), "{name:?} is a point a rope can bite");
        }
        for name in ANCHOR_NAMES {
            assert!(is_anchor_name(name));
        }
        // …and the family does not swallow the rest of the file. A wall segment carries 44
        // nodes and 9 of them are hooks; the other 35 are meshes and joints.
        for name in ["hook", "hooks.l", "Cube.003", "cortex.old", "Armature", ""] {
            assert!(!is_anchor_name(name), "{name:?} is not an anchor");
        }
    }

    #[test]
    fn the_cortex_is_one_of_the_names_a_model_may_carry() {
        assert!(ANCHOR_NAMES.contains(&CORTEX_ANCHOR));
        assert_eq!(ANCHOR_NAMES.len(), 8, "the table in docs/models.md has eight rows");
    }
}
