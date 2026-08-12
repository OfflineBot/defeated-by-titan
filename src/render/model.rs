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

use bevy::animation::graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex};
use bevy::animation::{AnimationClip, AnimationPlayer, RepeatAnimation};
use bevy::world_serialization::WorldInstanceReady;
use bevy::asset::LoadState;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::data::{GameData, ModelSource};
use crate::shared::{MovementState, TitanKindName, TitanState};

// The anchor contract lives in `shared/` and not here, because `titan` has to READ it: the
// `cortex` empty out of the file is where the titan dies. See `shared::anchors`.
pub use crate::shared::anchors::{ModelAnchors, ANCHOR_NAMES, CORTEX_ANCHOR};

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
    /// Logical name -> the graph its [`AnimationPlayer`] plays out of, and one node per state.
    ///
    /// Built lazily by [`drive_animations`] out of [`clips`](Self::clips), **one graph asset
    /// per model and not per entity**: sixty husks share one graph, and a node index means the
    /// same thing on all of them.
    pub graphs: BTreeMap<String, ModelGraph>,
    /// Logical names still waiting for their file. Emptied by [`resolve_animation_clips`].
    pending: Vec<String>,
    /// `(model, state)` pairs already shouted about. **A warning that repeats every frame is a
    /// warning nobody reads** (rule 6) — and a titan flipping `Idle`/`Pursue` would produce
    /// exactly that.
    warned: BTreeSet<(String, String)>,
}

/// One model's animation graph: the asset the player reads, and where each state sits in it.
#[derive(Debug, Clone)]
pub struct ModelGraph {
    pub handle: Handle<AnimationGraph>,
    /// Game state -> its node. **A state that is absent here has no clip in the file** — that
    /// is the missing-clip case, and it is never filled with a neighbour.
    pub nodes: BTreeMap<String, AnimationNodeIndex>,
}

/// Which clip name a titan's state asks for. **The names are the ones in `docs/models.md`.**
///
/// [`TitanState`] is the honest source: it is what `titan::brain` already decides, what
/// `combat` gates on and what the F3 overlay prints. An animation state machine of its own
/// would be a second FSM that disagrees with the first one the day somebody adds an edge.
pub fn clip_state_of_titan(state: TitanState) -> &'static str {
    match state {
        TitanState::Idle => "idle",
        TitanState::Pursue => "walk",
        TitanState::Windup => "windup",
        TitanState::Strike => "strike",
        TitanState::Recover => "recover",
        TitanState::Death => "death",
    }
}

/// The same for the player's body ([`MovementState`]).
pub fn clip_state_of_movement(state: MovementState) -> &'static str {
    match state {
        MovementState::Grounded => "idle",
        MovementState::Airborne => "fall",
        MovementState::Tethered => "swing",
        MovementState::OnWall => "wall",
        MovementState::Downed => "downed",
    }
}

/// Does this state loop, or does it play once and stop?
///
/// **Looping is the exception, not the rule.** A state a body can stand in for ever loops; a
/// state that is a beat of an attack plays once, because a looping wind-up is a titan that
/// telegraphs for ever (`F-053`).
pub fn clip_repeats(state: &str) -> bool {
    matches!(state, "idle" | "walk" | "fall" | "swing" | "wall")
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

/// What is playing on this entity's model right now, and whether there was anything to play.
///
/// The memo that keeps [`drive_animations`] from doing its work every frame (rule 6): a state
/// is switched **on the edge**, not per tick. `has_clip: false` is not a resting state — it is
/// re-tried, because the file may still have been loading when the state was first asked for.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PlayingClip {
    /// The **game state** (`idle`, `walk`, `windup`, …), not the clip name inside the file.
    pub state: String,
    /// Did that state resolve to a clip? `false` means the model stands still for it.
    pub has_clip: bool,
}

/// **The visible half of the missing-clip warning.**
///
/// `docs/FINDINGS.md` FIND-053: a clip name that is not in the file has *no visual symptom* —
/// the model spawns, renders, is the right size and stands perfectly still. So when a model
/// that **declares** animations cannot show the state the game is in, the cuboid rig comes
/// back: it is driven by `titan::pose` and it does show the wind-up. You see a placeholder
/// instead of a lie, and the log says why.
///
/// A model that declares **no** animations at all (`animations: {}`) is a legal answer and
/// gets `false` — a static model is not a broken one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveFallback(pub bool);

/// The memo of what [`hide_the_primitive_under_a_model`] last did. Never set by hand.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimitiveHidden(pub bool);

/// **State in, clip out.** The seam the user asked for, finished.
///
/// Deliberately *not* a blend tree and *not* a transition table: [`TitanState`] already decides
/// what the body is doing, and a second state machine over the top of it disagrees with the
/// first one the day somebody adds an edge. `idle`/`walk` loop, `windup`/`strike` play once
/// (`clip_repeats`).
///
/// The [`AnimationPlayer`] is taken from the model where the glTF loader put it
/// (`bevy_gltf-0.19.0/src/loader/mod.rs:1088-1093` inserts one on an animated hierarchy's
/// root); only when the instance brings none does this put one on the scene child itself.
pub fn drive_animations(
    mut commands: Commands,
    data: Res<GameData>,
    mut assets: ResMut<ModelAssets>,
    mut graph_assets: ResMut<Assets<AnimationGraph>>,
    owners: Query<(
        Entity,
        &ModelName,
        &ModelBody,
        Option<&TitanState>,
        Option<&MovementState>,
        Option<&PlayingClip>,
    )>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for (entity, wanted, body, titan_state, movement, playing) in &owners {
        let ModelBody::Scene(scene) = body else {
            continue;
        };
        // The state the *game* is in. A titan says so, the player's body says so, and an
        // entity that says neither has nothing to drive.
        let state = match (titan_state, movement) {
            (Some(s), _) => clip_state_of_titan(*s),
            (None, Some(m)) => clip_state_of_movement(*m),
            (None, None) => continue,
        };
        if playing.is_some_and(|p| p.state == state && p.has_clip) {
            continue;
        }
        // Nothing has been resolved for this model yet — the file may still be in flight. Do
        // not decide "no clip" on a question that has not been answered.
        if !assets.clips.contains_key(&wanted.name) {
            continue;
        }

        build_graph_once(&mut assets, &mut graph_assets, &wanted.name);
        let node = assets
            .graphs
            .get(&wanted.name)
            .and_then(|g| g.nodes.get(state))
            .copied();

        let Some(node) = node else {
            // The fourth glTF trap (FIND-053): loud, once, and with something to look at.
            let declares = data.model(&wanted.name).is_some_and(|m| !m.animations.is_empty());
            if declares && assets.warned.insert((wanted.name.clone(), state.to_string())) {
                warn!(
                    "model {:?}: the game is in state {state:?} and this model has no clip for \
                     it. The cuboid rig stays visible for that state — a model that stands \
                     still would look exactly like a model that has no animation at all \
                     (docs/models.md, docs/FINDINGS.md FIND-053).",
                    wanted.name
                );
            }
            commands.entity(entity).insert((
                PlayingClip { state: state.to_string(), has_clip: false },
                PrimitiveFallback(declares),
            ));
            continue;
        };

        let handle = assets.graphs[&wanted.name].handle.clone();
        let Some(player_entity) = animation_player_of(*scene, &children, &players) else {
            // The instance brought none — put one on the scene child and play next frame, when
            // the component actually exists. Not an error: a hand-built world asset (and every
            // test fixture in this repository) has no `AnimationPlayer` of its own.
            commands
                .entity(*scene)
                .insert((AnimationPlayer::default(), AnimationGraphHandle(handle)));
            continue;
        };

        commands.entity(player_entity).insert(AnimationGraphHandle(handle));
        if let Ok(mut player) = players.get_mut(player_entity) {
            // One state at a time. No cross-fade, no transition table — that is a separate
            // decision and it is not this seam's to make.
            player.stop_all();
            let active = player.play(node);
            if clip_repeats(state) {
                active.repeat();
            } else {
                active.set_repeat(RepeatAnimation::Never);
            }
        }
        commands.entity(entity).insert((
            PlayingClip { state: state.to_string(), has_clip: true },
            PrimitiveFallback(false),
        ));
    }
}

/// One [`AnimationGraph`] per model, built the first time a state asks for it.
///
/// Not per entity: sixty husks share one graph asset, and a node index means the same thing on
/// every one of them.
fn build_graph_once(
    assets: &mut ModelAssets,
    graph_assets: &mut Assets<AnimationGraph>,
    name: &str,
) {
    if assets.graphs.contains_key(name) {
        return;
    }
    let Some(clips) = assets.clips.get(name) else {
        return;
    };
    if clips.is_empty() {
        return;
    }
    let mut graph = AnimationGraph::new();
    let root = graph.root;
    let mut nodes = BTreeMap::new();
    for (state, clip) in clips {
        nodes.insert(state.clone(), graph.add_clip(clip.clone(), 1.0, root));
    }
    let handle = graph_assets.add(graph);
    info!("model {name:?}: animation graph built with {} state(s)", nodes.len());
    assets.graphs.insert(name.to_string(), ModelGraph { handle, nodes });
}

/// The scene's own [`AnimationPlayer`], or the first one under it. `None` = the instance
/// brought none.
fn animation_player_of(
    scene: Entity,
    children: &Query<&Children>,
    players: &Query<&mut AnimationPlayer>,
) -> Option<Entity> {
    if players.contains(scene) {
        return Some(scene);
    }
    children.iter_descendants(scene).find(|e| players.contains(*e))
}

/// **The primitive gets out of the way.**
///
/// Until 2026-08-12 a configured model spawned its scene *beside* the cuboid rig and both were
/// visible — for a titan that is two bodies in one place. This hides the rig, and it hides
/// **only the picture**: `Visibility` is not read by avian, by `GlobalTransform` propagation or
/// by `SpatialQuery`, so the body collider, the cortex sensor and every length the rig computes
/// stay exactly where they were. That is the whole reason it is `Visibility` and not a despawn
/// or a removed `Mesh3d`: a hidden cortex still kills, and it comes back with one line of RON.
///
/// **The trigger is the arrived scene, not the configured row** — a file that never loads
/// leaves the cuboids standing, which is what `docs/models.md` promises for a wrong path.
pub fn hide_the_primitive_under_a_model(
    mut commands: Commands,
    owners: Query<(Entity, &ModelBody, Option<&PrimitiveFallback>, Option<&PrimitiveHidden>)>,
    arrived: Query<(), With<WorldAssetRoot>>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
) {
    for (entity, body, fallback, hidden) in &owners {
        let want_hidden = match body {
            ModelBody::Primitive => false,
            ModelBody::Scene(scene) => {
                arrived.contains(*scene) && !fallback.is_some_and(|f| f.0)
            }
        };
        if hidden.is_some_and(|h| h.0 == want_hidden) {
            continue;
        }
        let skip = match body {
            ModelBody::Scene(scene) => Some(*scene),
            ModelBody::Primitive => None,
        };
        let visibility =
            if want_hidden { Visibility::Hidden } else { Visibility::Inherited };
        for part in primitive_parts(entity, skip, &children, &meshes) {
            commands.entity(part).insert(visibility);
        }
        // A block carries its cuboid on the entity ITSELF, not on a child (`build_block_meshes`).
        // Hiding it would take the scene child with it — unless the child says
        // `Visibility::Visible`, which ignores a hidden parent. That is the one case where the
        // model has to be louder than its owner.
        if meshes.contains(entity) {
            commands.entity(entity).insert(visibility);
            if let Some(scene) = skip {
                commands.entity(scene).insert(if want_hidden {
                    Visibility::Visible
                } else {
                    Visibility::Inherited
                });
            }
        }
        commands.entity(entity).insert(PrimitiveHidden(want_hidden));
    }
}

/// Every mesh under `owner` that is **not** part of the model's own scene.
///
/// Walked by hand instead of `iter_descendants` so that the scene's subtree can be cut out
/// whole: hiding the model to make the model visible would be a funny bug to debug.
fn primitive_parts(
    owner: Entity,
    skip: Option<Entity>,
    children: &Query<&Children>,
    meshes: &Query<(), With<Mesh3d>>,
) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut pending = vec![owner];
    while let Some(entity) = pending.pop() {
        if Some(entity) == skip {
            continue;
        }
        if entity != owner && meshes.contains(entity) {
            found.push(entity);
        }
        if let Ok(kids) = children.get(entity) {
            pending.extend(kids.iter());
        }
    }
    found
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
