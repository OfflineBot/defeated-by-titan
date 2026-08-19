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
pub use crate::shared::anchors::{
    is_anchor_name, ModelAnchors, ModelName, ANCHOR_NAMES, CORTEX_ANCHOR, HOOK_PREFIX,
};

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
            height_m: data.titan_height_m(kind),
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
                        model_transform(model.scale),
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

/// **The drop faces the other way, and this is where that is undone.**
///
/// `docs/conventions.md` and the titan rig agree that a body's forward is **-Z**
/// (`TitanRig::shoulder_in_torso`: "a body whose forward is -Z", `cortex_in_head` puts the nape
/// at +Z). The asset drop of 2026-08-18 is authored the other way round, and it says so twice
/// in every file that has a front: on `a-042-koerpertyp-a-hager-mittel` the `eye` empty sits at
/// z = **+0.92** and the `cortex` empty — the nape — at z = **-0.139**. Same on the human kit:
/// `a-136-npc-vanguard` puts its eye at z = +0.20.
///
/// Measured, not argued: with no rotation a husk that has aggroed and is walking at the player
/// renders its **back** to him, and the `cortex` anchor lands on the wrong side of the neck —
/// which took `tests/titan.rs::q030_the_nape_is_cut_from_behind_and_not_from_the_front` red,
/// i.e. the titan became cuttable from the front.
///
/// **It belongs here and not in the files.** A model's authored axis convention is the seam
/// between a file and the engine, which is exactly what this module is; and it is one property
/// of one coherent export, not eight rows of RON that can drift apart.
pub const MODEL_FACES: f32 = std::f32::consts::PI;

/// The model's own transform on the scene child: the drop's frame turned into the game's, at
/// the size [`fit_to_class`] worked out.
fn model_transform(scale: f32) -> Transform {
    Transform {
        translation: Vec3::ZERO,
        rotation: Quat::from_rotation_y(MODEL_FACES),
        scale: Vec3::splat(scale),
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
    // How many of what came back are rope points rather than rig landmarks — the number the
    // log line used to report as *ignored*. Until 2026-08-18 the filter here was
    // `ANCHOR_NAMES.contains(&name)` and nothing else, so a wall segment carrying nine hand
    // placed cornices arrived with **two** anchors, and the 439 `hook.*` empties the drop
    // spreads over 144 files were dropped at load. `shared::anchors::is_anchor_name` now lets
    // the whole `hook.` family through; the closed list still governs the rig landmarks.
    let mut hooks = 0usize;
    for descendant in children.iter_descendants(instance_root) {
        let Ok(name) = names.get(descendant) else {
            continue;
        };
        let name = name.as_str();
        if !is_anchor_name(name) {
            continue;
        }
        if name.starts_with(HOOK_PREFIX) {
            hooks += 1;
        }
        found.insert(name.to_string(), position_in(descendant, instance_root, &parents, &transforms));
    }

    // **The model is brought to the size the simulation already believes in.**
    //
    // `titan.ron` deliberately gives several size classes the same logical model, and the
    // primitive rig has always been built at the class height out of `scale.ron`. A `.glb` is
    // authored at exactly one height, so without this the small kinds would render at the
    // medium one's — a 4.2 m collider inside a 10 m picture, which is exactly what `art.ron`'s
    // own header forbids ("the same size, hit zone and scale").
    let fit = fit_to_class(&found, wanted.height_m, wanted.cortex_height_m);
    let scale = data.model(&wanted.name).map_or(1.0, |m| m.scale) * fit;

    // ⚠️ **And the anchors go with it — both halves of it.** `position_in` composes the chain
    // up to but NOT including the scene child, and the scene child is the entity that carries
    // the model's own transform. So what it returns is in the FILE's frame at the FILE's size,
    // while the mesh is drawn turned ([`MODEL_FACES`]) and scaled. Two silent bugs lived in
    // that gap: an `art.ron: scale` of 2.0 put the cortex sensor at half the visible head
    // height, and the drop's nape anchor landed on the front of the neck.
    // Taken here, while `found` is still in model units — after the loop below it would be
    // multiplied by `scale` a second time.
    let stands = fitted_height_m(&found, scale);
    let into_the_game = Quat::from_rotation_y(MODEL_FACES);
    for anchor in found.values_mut() {
        *anchor = into_the_game * (*anchor * scale);
    }
    commands.entity(instance_root).insert(model_transform(scale));

    match found.get(CORTEX_ANCHOR) {
        // ⚠️ **Only for something that is supposed to have one.** `cortex_height_m` is `Some`
        // exactly for a titan (`name_the_titans_model`); a house, a ruin or a wall segment is
        // `None`, and since 2026-08-19 there are ~790 of those in the district. Warning per
        // instance turned one honest sentence about a titan into 790 lines of log per run —
        // and a log nobody reads is a log that hides the next real one.
        None if wanted.cortex_height_m.is_none() => {}
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

    // **The other end of the same check.** [`fit_to_class`] matches the cortex exactly when
    // the model brings one — so a body whose cortex sits at the wrong fraction of itself no
    // longer shows up as a displaced cortex, it shows up as a body of the wrong height. If
    // only the cortex were checked, the fit would silence the very warning that catches a
    // wrongly authored model (an `a-045` head part, authored in its parent rig's space, is
    // 1.32 m tall and carries the rig's cortex at 8.92 m — it would pass a cortex check and
    // render as a 10 m titan made of one head).
    if let (Some(height), Some(expected)) = (stands, wanted.height_m) {
        let off = (height - expected).abs();
        if off > data.art.cortex_tolerance_m {
            warn!(
                "model {:?}: brought to the cortex scale.ron names, this body stands \
                 {height:.2} m tall and its size class is {expected:.2} m — {off:.2} m out, \
                 past art.ron's cortex_tolerance_m of {:.2}. Either the model is authored for \
                 another class or its cortex sits at the wrong fraction of it (docs/models.md).",
                wanted.name, data.art.cortex_tolerance_m
            );
        }
    }

    let count = found.len();
    commands.entity(owner).insert(ModelAnchors(found));
    info!(
        "model {:?}: {count} anchor(s) read out of the file, {hooks} of them {HOOK_PREFIX}* rope \
         points, drawn at scale {scale:.4}",
        wanted.name
    );
}

/// How much a model has to be scaled so that it agrees with `scale.ron` about **this entity**.
///
/// One logical name dresses two size classes on purpose — `titan.ron` gives `titan_husk` to
/// three medium kinds (10.0 m) *and* to two small ones (4.2 m), the way the primitive rig has
/// always been built: one shape at the class height. A `.glb` is authored at exactly one size,
/// so it is brought to the entity's, against one of two yardsticks:
///
/// 1. **The `cortex` empty, whenever the model brings one and `scale.ron` names one.** That is
///    the number this game is about — a titan dies at its cortex and nowhere else — and
///    hitting it exactly is what keeps a swapped model and the cuboid rig killable in the same
///    place. `art.ron`'s header asks for "the same size, hit zone and scale", and the two
///    cannot both be exact: the drop authors its cortex at 0.8854 of its own height while
///    `scale.ron`'s classes use 0.8929. **Measured, on 2026-08-18, which of the two to give
///    up:** fitting by height instead moved the husk's kill zone from 8.90 m to 8.85 m, and
///    five tests in `tests/titan.rs` went red on that 5 cm — a warden nape pass missed by
///    0.020 m and a husk was suddenly cuttable from the front. Fitting by cortex moves the
///    silhouette by 0.6 % (5.7 cm on a 10 m body) and moves no hit zone at all.
/// 2. **The `hit.min`/`hit.max` pair**, for everything that carries no cortex — 278 of 278
///    models in the drop carry it, and it is the only size claim a `.glb` makes about itself
///    that a machine can read. ⚠️ **A corner pair, not an ordered AABB:** on all 278 files
///    `hit.max.z < hit.min.z`, from Blender's +Y-forward to glTF's -Z-forward flip. Hence the
///    absolute value; whoever consumes those two anchors next takes a componentwise min/max.
///
/// `1.0` when neither yardstick has both its numbers — a model that states no size, or an
/// entity whose size nobody wrote down, is drawn exactly as it was authored. **Never a guess.**
pub fn fit_to_class(
    anchors: &BTreeMap<String, Vec3>,
    wanted_height_m: Option<f32>,
    wanted_cortex_m: Option<f32>,
) -> f32 {
    if let (Some(cortex), Some(wanted)) = (anchors.get(CORTEX_ANCHOR), wanted_cortex_m) {
        if cortex.y > f32::EPSILON && wanted > 0.0 {
            return wanted / cortex.y;
        }
    }
    let Some(wanted) = wanted_height_m else {
        return 1.0;
    };
    let Some(authored) = authored_height_m(anchors) else {
        return 1.0;
    };
    if authored <= f32::EPSILON || wanted <= 0.0 {
        return 1.0;
    }
    wanted / authored
}

/// What the model says it is tall, out of its own `hit.min`/`hit.max` pair. `None` when it
/// states no size at all — which is a legal answer and not an error (see [`fit_to_class`]).
pub fn authored_height_m(anchors: &BTreeMap<String, Vec3>) -> Option<f32> {
    let lo = anchors.get("hit.min")?;
    let hi = anchors.get("hit.max")?;
    Some((hi.y - lo.y).abs())
}

/// How tall the model actually stands once it is drawn at `scale`.
///
/// **The counterweight to a cortex fit.** Matching the cortex exactly means the cortex can no
/// longer report a badly authored model, so the height has to — see the check in
/// [`read_the_models_anchors`].
pub fn fitted_height_m(anchors: &BTreeMap<String, Vec3>, scale: f32) -> Option<f32> {
    authored_height_m(anchors).map(|h| h * scale)
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
