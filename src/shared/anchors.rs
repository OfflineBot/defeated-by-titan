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
    fn the_cortex_is_one_of_the_names_a_model_may_carry() {
        assert!(ANCHOR_NAMES.contains(&CORTEX_ANCHOR));
        assert_eq!(ANCHOR_NAMES.len(), 8, "the table in docs/models.md has eight rows");
    }
}
