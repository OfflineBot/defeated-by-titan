//! model — the seam between "a cuboid we drew" and "a model the user dropped in".
//!
//! The user, 2026-08-12: *„mach zudem, dass ich später einfach die 3d modelle austauschen
//! kann + eigene animationen adden kann!"* — make it so the 3D models can simply be swapped
//! later and own animations added. This file is that sentence, made runnable.
//!
//! ## The one rule the whole file exists for
//!
//! **The game runs with not a single `.glb` in the repository.** That is not a nicety, it is
//! how this repo works today (`assets/` holds `data/` and nothing else) and how CI stays
//! cheap. So `assets/data/art.ron` names a [`ModelSource`] per logical name, and
//! [`ModelSource::Primitive`] — an explicit variant, not a missing field — means *keep the
//! cuboid*. Nothing here spawns, loads or asks for a file until a line in that RON says so.
//!
//! ## The four seams, in the order they run
//!
//! | System | what it does |
//! |---|---|
//! | [`load_configured_models`] | one `Startup` pass over `art.ron`; only `Gltf(..)` entries ask the asset server for anything |
//! | [`resolve_animation_clips`] | once a glTF is in memory: state name -> clip name -> [`AnimationClip`] handle, **loudly** when a clip name is not in the file |
//! | [`name_the_titans_model`] | a titan carries [`TitanKindName`]; `titan.ron` names its model; that becomes a [`ModelName`] |
//! | [`spawn_models`] | [`ModelName`] -> either [`ModelBody::Primitive`] (do nothing, the rig stands) or a child carrying the scene |
//!
//! and one observer, [`read_the_models_anchors`], which is the part that makes a swap *work*
//! instead of merely *render*: the `cortex` empty out of the file beats the computed rig
//! position, and its absence falls back to the computed one instead of putting the kill zone
//! at the origin.
//!
//! ## ⚠️ Stage 🟨 — built, untested by a human eye
//!
//! The fallback direction is nailed down by `tests/render.rs`; the *loaded* direction has
//! never had a real `.glb` under it, because there is none. What is proven is that a
//! configured entry produces a scene root and an unconfigured one does not. What is **not**
//! proven is that a real exported model arrives upright, painted and the right size — that
//! needs a file and a screen (`docs/models.md`).

use bevy::animation::AnimationClip;
use bevy::world_serialization::WorldInstanceReady;
use bevy::asset::LoadState;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use std::collections::BTreeMap;

use crate::data::{GameData, ModelSource};
use crate::shared::TitanKindName;

/// The empties a model may carry, exactly as `docs/models.md` names them.
///
/// **Names, not numbers** — this is a contract with Blender, so it belongs in the code the way
/// an axis convention does. A typo in Blender therefore does not raise an error, it produces a
/// *missing* anchor; that is why [`read_the_models_anchors`] shouts about a missing `cortex`
/// instead of silently doing nothing.
pub const ANCHOR_NAMES: [&str; 8] =
    ["cortex", "hit.min", "hit.max", "hook.l", "hook.r", "hand.l", "hand.r", "eye"];

/// The one anchor whose absence is a gameplay bug and not a missing detail (`F-030`).
pub const CORTEX_ANCHOR: &str = "cortex";

/// "This entity is the logical model `name`."
///
/// The component is written by [`name_the_titans_model`] out of `titan.ron`, and it may be put
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
}

impl ModelName {
    pub fn new(name: impl Into<String>) -> Self {
        ModelName { name: name.into(), cortex_height_m: None }
    }
}

/// What [`spawn_models`] decided — and the whole point of the file: **both answers are
/// normal.**
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBody {
    /// `art.ron` says [`ModelSource::Primitive`], or the name is not in `art.ron` at all.
    /// Whatever built the cuboids keeps standing; nothing is spawned, nothing is hidden.
    Primitive,
    /// A child entity carries the glTF scene. It is a child and not the entity itself so that
    /// the model's own scale never fights the simulation's transform.
    Scene(Entity),
}

/// The anchors that came **out of the model**, in the model root's own space, in meters.
///
/// Inserted on **every** entity with a [`ModelName`], empty for a primitive. That is
/// deliberate: a reader asks [`ModelAnchors::get`], gets `None`, and uses the computed rig
/// position — one code path for both worlds, which is the difference between a switch and a
/// rebuild (`docs/models.md`).
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

/// Everything the registry asked the asset server for. Empty on a repo with no `.glb`.
#[derive(Resource, Debug, Default)]
pub struct ModelAssets {
    /// Logical name -> the glTF file, while and after it loads.
    pub gltf: BTreeMap<String, Handle<Gltf>>,
    /// Logical name -> scene root handle, once the file is in memory.
    pub scenes: BTreeMap<String, Handle<WorldAsset>>,
    /// Logical name -> game state -> the clip that state plays.
    ///
    /// **This is the animation seam.** A state that is absent here has no clip, and a caller
    /// falls back to the static pose — which is exactly what happens when the user names a
    /// clip that is not in his file.
    pub clips: BTreeMap<String, BTreeMap<String, Handle<AnimationClip>>>,
    /// Logical names still waiting for their file. Emptied by [`resolve_animation_clips`].
    pending: Vec<String>,
}

/// One `Startup` pass over `art.ron`.
///
/// **On a repo without a single `.glb` this system loads nothing and touches no path.** Only a
/// `Gltf(..)` line makes it ask for a file, and the path comes out of the RON — no file name
/// stands in Rust (`docs/models.md`, `tools/norms.py`).
pub fn load_configured_models(
    mut assets: ResMut<ModelAssets>,
    data: Res<GameData>,
    server: Res<AssetServer>,
) {
    for (name, model) in &data.art.models {
        let ModelSource::Gltf(path) = &model.source else {
            continue;
        };
        assets.gltf.insert(name.clone(), server.load(path.clone()));
        assets.pending.push(name.clone());
    }
    if !assets.pending.is_empty() {
        info!("art.ron: {} model(s) come out of a file, the rest stay primitives",
              assets.pending.len());
    }
}

/// Turns clip **names** into clip **handles**, once per model, and shouts when a name is wrong.
///
/// The three glTF traps in `docs/models.md` all look the same from the outside — white, chrome
/// or invisible — and a missing animation is the fourth of that family: nothing crashes, the
/// model simply stands still. So the failure is spelled out with the file, the state, the name
/// that was asked for **and the names that are actually in the file**; without that last list
/// the user has no way to find out what he should have typed.
pub fn resolve_animation_clips(
    mut assets: ResMut<ModelAssets>,
    data: Res<GameData>,
    gltfs: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
) {
    if assets.pending.is_empty() {
        return;
    }
    let mut still_pending = Vec::new();
    let names: Vec<String> = assets.pending.clone();

    for name in names {
        let Some(handle) = assets.gltf.get(&name).cloned() else {
            continue;
        };
        match server.get_load_state(&handle) {
            Some(LoadState::Failed(e)) => {
                let path = match data.art.models.get(&name).map(|m| &m.source) {
                    Some(ModelSource::Gltf(p)) => p.clone(),
                    _ => String::new(),
                };
                error!(
                    "art.ron: model {name:?} points at {path:?} and that file did not load \
                     ({e}). The entity keeps its primitive. Put the file under assets/ or set \
                     `source: Primitive` (docs/models.md)."
                );
                continue;
            }
            Some(LoadState::Loaded) => {}
            _ => {
                still_pending.push(name);
                continue;
            }
        }

        let Some(gltf) = gltfs.get(&handle) else {
            still_pending.push(name);
            continue;
        };

        if let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned()) {
            assets.scenes.insert(name.clone(), scene);
        } else {
            error!("model {name:?}: the file carries no scene at all — nothing to spawn");
        }

        let wanted = data.art.models.get(&name).map(|m| m.animations.clone()).unwrap_or_default();
        let mut resolved: BTreeMap<String, Handle<AnimationClip>> = BTreeMap::new();
        for (state, clip_name) in &wanted {
            match gltf.named_animations.get(clip_name.as_str()) {
                Some(clip) => {
                    resolved.insert(state.clone(), clip.clone());
                }
                None => {
                    let present: Vec<&str> =
                        gltf.named_animations.keys().map(|k| k.as_ref()).collect();
                    warn!(
                        "model {name:?}: art.ron maps the state {state:?} to the clip \
                         {clip_name:?}, and that clip is NOT in the file. The clips the file \
                         does carry are {present:?}. That state falls back to no animation — \
                         the model will stand still and nothing else will go wrong, which is \
                         exactly why this line exists (docs/models.md)."
                    );
                }
            }
        }
        if !wanted.is_empty() {
            info!("model {name:?}: {}/{} animation states resolved", resolved.len(), wanted.len());
        }
        assets.clips.insert(name.clone(), resolved);
    }

    assets.pending = still_pending;
}

/// A titan says which **kind** it is; `titan.ron` says which model that kind wears.
///
/// This is the whole wiring, and it costs no domain edge: [`TitanKindName`] lives in `shared`
/// and `titan.ron` in `data` — both free for every domain (`docs/architecture.md`). `render`
/// never learns that a domain `titan` exists.
pub fn name_the_titans_model(
    mut commands: Commands,
    data: Res<GameData>,
    fresh: Query<(Entity, &TitanKindName), Without<ModelName>>,
) {
    for (entity, kind_name) in &fresh {
        let Some(kind) = data.titan(kind_name.as_str()) else {
            continue;
        };
        commands.entity(entity).insert(ModelName {
            name: kind.model.clone(),
            cortex_height_m: data.titan_cortex_height_m(kind),
        });
    }
}

/// The switch itself: primitive or scene, decided by one line in `art.ron`.
///
/// A name that is **not** in `art.ron` is treated as a primitive and said out loud once. That
/// is the safe direction: an unknown model must never take the geometry away that is already
/// standing there.
pub fn spawn_models(
    mut commands: Commands,
    data: Res<GameData>,
    assets: Res<ModelAssets>,
    fresh: Query<(Entity, &ModelName), Without<ModelBody>>,
) {
    for (entity, wanted) in &fresh {
        let Some(model) = data.model(&wanted.name) else {
            warn!(
                "model {:?} is not in art.ron — the entity keeps its primitive. \
                 Every logical name needs a line there (docs/models.md).",
                wanted.name
            );
            commands
                .entity(entity)
                .insert((ModelBody::Primitive, ModelAnchors::default()));
            continue;
        };

        match &model.source {
            ModelSource::Primitive => {
                commands
                    .entity(entity)
                    .insert((ModelBody::Primitive, ModelAnchors::default()));
            }
            ModelSource::Gltf(path) => {
                let Some(handle) = assets.gltf.get(&wanted.name) else {
                    // Cannot happen while `load_configured_models` runs first; if the order is
                    // ever broken this is the sentence that says so instead of an empty world.
                    error!("model {:?} ({path:?}) was never asked for — order broken", wanted.name);
                    continue;
                };
                // The scene sits on a CHILD. The simulation owns the entity's own transform
                // (`docs/architecture.md`, one field one writer); the model owns only its own
                // scale, and `scale` in the RON is the emergency brake — 1 Blender unit is
                // 1 meter, so the honest value is 1.0 (`docs/models.md`).
                let scene = commands
                    .spawn((
                        Name::new(format!("model:{}", wanted.name)),
                        Transform::from_scale(Vec3::splat(model.scale)),
                    ))
                    .id();
                if let Some(world) = assets.scenes.get(&wanted.name) {
                    commands
                        .entity(scene)
                        .insert(WorldAssetRoot(world.clone()));
                } else {
                    // The file is still loading. `resolve_animation_clips` fills `scenes`, and
                    // the handle lands here on a later frame — see `attach_late_scenes`.
                    commands.entity(scene).insert(PendingScene(handle.clone()));
                }
                commands.entity(entity).add_child(scene);
                commands
                    .entity(entity)
                    .insert((ModelBody::Scene(scene), ModelAnchors::default()));
            }
        }
    }
}

/// A scene child whose file had not finished loading when its entity was spawned.
#[derive(Component, Debug, Clone)]
pub struct PendingScene(pub Handle<Gltf>);

/// Hangs the scene on the children that were spawned before their file was in memory.
///
/// Without this a titan that appears in the first frame after startup would keep an empty
/// child for ever — the failure would be *an entity with no model and no error message*, which
/// is the exact class of bug `docs/models.md` calls "all three traps look the same".
pub fn attach_late_scenes(
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
    waiting: Query<(Entity, &PendingScene)>,
) {
    for (entity, pending) in &waiting {
        let Some(gltf) = gltfs.get(&pending.0) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
        else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<PendingScene>()
            .insert(WorldAssetRoot(scene));
    }
}

/// **The part that makes a swap work instead of merely render.**
///
/// A glTF node called `cortex` arrives as an entity with a [`Name`]. This walks the freshly
/// spawned instance, collects every empty out of [`ANCHOR_NAMES`], converts it into the model
/// root's own space and writes it onto the entity that carries the [`ModelName`].
///
/// **What it does not do:** invent one. A model without a `cortex` gets a warning and an empty
/// map, and every reader keeps using the computed rig position. `docs/models.md` says a
/// missing empty makes the zone "a point" — an empty map is the honest way to say "ask the
/// rig", and a `Vec3::ZERO` would be a kill zone between the feet.
pub fn read_the_models_anchors(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    data: Res<GameData>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform>,
    names: Query<&Name>,
    owners: Query<(Entity, &ModelName, &ModelBody)>,
) {
    let instance_root = ready.entity;
    // Who does this instance belong to? The scene sits on a child, the ModelName on its owner.
    let Some((owner, wanted, _)) = owners
        .iter()
        .find(|(_, _, body)| matches!(body, ModelBody::Scene(e) if *e == instance_root))
    else {
        return;
    };

    let mut found: BTreeMap<String, Vec3> = BTreeMap::new();
    for descendant in children.iter_descendants(instance_root) {
        let Ok(name) = names.get(descendant) else {
            continue;
        };
        let name = name.as_str();
        if !ANCHOR_NAMES.contains(&name) {
            continue;
        }
        found.insert(name.to_string(), position_in(descendant, instance_root, &parents, &transforms));
    }

    match found.get(CORTEX_ANCHOR) {
        None => warn!(
            "model {:?} carries no {CORTEX_ANCHOR:?} empty. The cortex therefore stays where \
             the rig computes it — the model does not decide where it dies (F-030). Name an \
             empty {CORTEX_ANCHOR:?} in Blender to change that (docs/models.md).",
            wanted.name
        ),
        Some(anchor) => {
            if let Some(expected) = wanted.cortex_height_m {
                let off = (anchor.y - expected).abs();
                if off > data.art.cortex_tolerance_m {
                    warn!(
                        "model {:?}: its {CORTEX_ANCHOR:?} empty sits at {:.2} m, and \
                         scale.ron puts the cortex of this size class at {expected:.2} m — \
                         {off:.2} m out, past art.ron's cortex_tolerance_m of {:.2}. The cut \
                         will land where the silhouette says and the kill zone is somewhere \
                         else (F-030).",
                        wanted.name, anchor.y, data.art.cortex_tolerance_m
                    );
                }
            }
        }
    }

    let count = found.len();
    commands.entity(owner).insert(ModelAnchors(found));
    info!("model {:?}: {count} anchor(s) read out of the file", wanted.name);
}

/// An entity's translation in `root`'s space, by walking the chain up.
///
/// **Not through `GlobalTransform`** on purpose: that is filled in `PostUpdate`, and this runs
/// the moment the instance is ready. Reading a stale global here would put the anchors one
/// frame — and on the first frame, a whole world — off, and nothing would say so.
fn position_in(
    entity: Entity,
    root: Entity,
    parents: &Query<&ChildOf>,
    transforms: &Query<&Transform>,
) -> Vec3 {
    let mut local = transforms.get(entity).copied().unwrap_or_default();
    let mut at = entity;
    while at != root {
        let Ok(parent) = parents.get(at) else {
            break;
        };
        at = parent.parent();
        if at == root {
            break;
        }
        let Ok(above) = transforms.get(at) else {
            continue;
        };
        local = above.mul_transform(local);
    }
    local.translation
}
