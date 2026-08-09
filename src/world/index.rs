//! The spatial index, maintained — `T-036a`.
//!
//! The **type** lives in [`shared::spatial`](crate::shared::spatial) so that `vector` and
//! `player` can ask it without needing an edge to `world`. All that stands here is **who
//! keeps it current**.
//!
//! ## Why maintenance does not run through `RemovedComponents`
//!
//! Evidence from the installed source: the buffers of `RemovedComponents` are swapped in
//! `World::clear_trackers` (`bevy_ecs-0.19.0/src/world/mod.rs:1735-1738`), and
//! `clear_trackers` runs **once per `App::update`** (`bevy_app-0.19.0/src/sub_app.rs:149`)
//! — not once per tick. `FixedMain`, by contrast, runs "0, 1 or more times during a single
//! update" (`bevy_time-0.19.0/src/fixed.rs:37-39`). `src/lib.rs` drives headless at 240 Hz
//! against 60 Hz fixed: **three out of four frames run without `FixedMain`**, and a message
//! from one of those frames is gone before the maintainer ever sees it. An obstacle would
//! stay in the grid forever, and a hook hangs on a house that no longer exists.
//!
//! Hence: an **observer** on `Remove` that pushes the id into the index's mailbox
//! (`SpatialIndex::queue_removal`). Observers run the moment something is removed and not at
//! the end of the frame — the information survives any number of frames.
//!
//! ## Why this in-house build is still standing although avian is faster
//!
//! Measured on 2026-08-09: avian's `SpatialQuery` answers a 112 m ray against 4000 cuboids
//! in **0.21 us**. Against that number a hand-written grid has no argument left, and the
//! honest direction is: **this index goes away.**
//!
//! It still does not go away today, for three reasons that have nothing to do with taste:
//!
//! 1. **The successor is not wired up.** `PhysicsPlugins` is not registered in `src/lib.rs`;
//!    `SpatialQuery` answers nothing at all today. Deleting the only query path there is
//!    before the replacement runs would mean building on 🟨 (§6 rule 1).
//! 2. **The type does not belong to this domain.** `SpatialIndex` lives in `shared::spatial`
//!    and is asked by `vector` and `player`. Whoever deletes here leaves dead code standing
//!    there — and that is a decision about somebody else's files.
//! 3. What happens to `T-036a` in `docs/features.ron` is the main head's call. As long as a
//!    line stands there demanding a grid, deleting is not tidying up but a silent
//!    contradiction of the target list.
//!
//! That fixes the order: first register `PhysicsPlugins` and move `vector` over to
//! `SpatialQuery`, then delete here and in `shared::spatial`, then bring `T-036a` up to
//! date. **Not the other way around.**

use bevy::prelude::*;

use crate::shared::{
    IndexEntry, IdCounter, Body, BodyId, BodyGone, BodyMask, SpatialIndex, Tick,
};

/// Takes in new bodies, strikes queued ones out and reports them as [`BodyGone`].
///
/// Runs first in the fixed step (`SimulationSystems::Spatial`): the index is current
/// **before** anyone asks it. New bodies get their [`BodyId`] from the [`IdCounter`] here —
/// consecutive, not random, so that two machines end up with the same order.
///
/// What is read is the `GlobalTransform`, not the `Transform`: for a child body (a titan
/// limb from `F-029` on) the `Transform` is **local**, and the world center is only in the
/// `GlobalTransform`. Today the two are identical — the line is still right now instead of
/// wrong later.
// to be filled by assignment R — T-036a
pub fn maintain_index(
    mut _commands: Commands,
    mut _index: ResMut<SpatialIndex>,
    mut _ids: ResMut<IdCounter>,
    _tick: Res<Tick>,
    mut _weg: MessageWriter<BodyGone>,
    _neu: Query<(Entity, &Body, &GlobalTransform), Without<BodyId>>,
    _bekannt: Query<(&BodyId, &Body, &GlobalTransform), Changed<GlobalTransform>>,
) {
}

/// Observer: a [`Body`] disappears.
///
/// Pushes its id into the index's mailbox. The maintainer collects it in the next fixed step
/// and sends [`BodyGone`] — that is what the hooks hanging on it release on.
///
/// `Option<ResMut<SpatialIndex>>`, because an observer can also fire before the resource is
/// inserted (test apps, `App::finish`). A missing index is then not a crash but a body that
/// never existed in the index anyway.
// to be filled by assignment R — T-036a
pub fn on_body_removed(
    _ereignis: On<Remove, Body>,
    _ids: Query<&BodyId>,
    _index: Option<ResMut<SpatialIndex>>,
) {
}

/// Build the mask out of an entity's marker state.
///
/// One place where `anchorable`/`solid` turn into bits — otherwise the translation stands in
/// `world::map` **and** here, and one of the two goes stale.
pub fn mask_from(solid: bool, anchorable: bool) -> BodyMask {
    let mut m = BodyMask::NONE;
    if solid {
        m = m.with(BodyMask::SOLID);
    }
    if anchorable {
        m = m.with(BodyMask::ANCHORABLE);
    }
    m
}

/// Build an entry out of a body and a world position. The only place that knows the entry's
/// center is the entity's world position.
pub fn entry_from(id: BodyId, body: &Body, world: &GlobalTransform) -> IndexEntry {
    IndexEntry {
        id,
        center_m: world.translation(),
        half_size_m: body.half_size_m,
        mask: body.mask,
    }
}
