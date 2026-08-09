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
//! It still does not go away today — but **reason 1 below has expired**, and the shape of
//! what is left is different from what this header used to claim. Checked on 2026-08-09,
//! `B-001`:
//!
//! 1. ~~**The successor is not wired up.**~~ **Stale.** `PhysicsPlugins::new(FixedUpdate)` is
//!    registered at `src/lib.rs:117`, `vector::aim` casts through avian's `SpatialQuery`
//!    (`src/vector/aim.rs:121`) and `tests/vector_aiming.rs` measures its hits against
//!    `maps.ron` to the centimetre. So for **rays** the successor is not merely wired up, it
//!    has already won the job: `grep -rn 'cast_ray\|aabb_overlaps' src/ tests/` finds **no
//!    caller** of [`SpatialIndex::cast_ray`] or [`SpatialIndex::aabb_overlaps`] outside
//!    `shared::spatial` itself. That half of the index is dead code with a stub body.
//! 2. **The other half is load-bearing, and `SpatialQuery` cannot do it.** `vector::hook`
//!    holds `Res<SpatialIndex>` and asks `index.body(id)` twice (`src/vector/hook.rs:180`
//!    and `:227`) — "where does carrier 42 stand *now*", to compute the anchor in the
//!    carrier's frame, and "does carrier 42 still exist". avian answers rays and shapes; it
//!    has no lookup by a stable id, because it does not know our ids. And **[`BodyId`] itself
//!    is handed out here and nowhere else** — no directory, no ids, no hooks.
//! 3. **The type does not belong to this domain.** `SpatialIndex` lives in `shared::spatial`
//!    and is asked by `vector` and `player`. Whoever deletes here leaves dead code standing
//!    there — and that is a decision about somebody else's files.
//! 4. What happens to `T-036a` in `docs/features.ron` is the main head's call. As long as a
//!    line stands there demanding a grid, deleting is not tidying up but a silent
//!    contradiction of the target list.
//!
//! **The decision that follows, and it is the reason this file is filled in rather than
//! deleted:** the short road the old header pointed at leads past the defect instead of
//! through it. Deleting the grid removes the two stubbed ray functions nobody calls; it does
//! **not** put a `BodyId` on a single house, and that — not the ray — is what `B-001` was.
//! Filling the maintainer is the smallest change that makes a hook hold, and it stays correct
//! whichever way the grid goes later.
//!
//! What is left of the old order, for whoever comes after: `cast_ray` and `aabb_overlaps` in
//! `shared::spatial` are now provably callerless and can go, together with the cells and the
//! large-body list. The **directory** stays, or moves somewhere that can answer
//! `BodyId -> position`. That is a change to somebody else's files and to `T-036a` — it is
//! written into the report, not done on the way past.

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
///
/// ⚠️ **And that costs one tick for a body spawned at runtime.** Transform propagation runs in
/// `PostStartup` and in `PostUpdate` (`bevy_transform-0.19.0/src/plugins.rs:27-37`), and
/// `RunFixedMainLoop` sits **before** `PostUpdate` in the frame. The city is spawned in
/// `Startup`, so it is propagated before the first fixed step and stands right from tick one.
/// A body that comes into being later is taken in here at the **origin** and moves to its real
/// place one tick later, when `Changed<GlobalTransform>` catches it. Measured in
/// `tests/world.rs::t036a_a_body_spawned_late_is_taken_in_and_stands_right_one_tick_later`.
/// Harmless for houses, **not** harmless for `F-029` — whoever hangs an anchor on a limb
/// spawned this frame has it at (0,0,0) for one tick.
///
/// ⚠️ **Which number a given body gets is not the order in `maps.ron`.** The `neu` query is
/// iterated by archetype, and an anchorable block (`AnchorSurface`) sits in a different
/// archetype from an untagged one — so `maps.ron: blocks[3]` came out as `BodyId(19)`,
/// measured in the run behind `scripts/b001-anchor.txt`. That is **deterministic** (the same
/// build and the same spawn order yield the same archetypes, so two machines agree — which is
/// all `docs/multiplayer.md` rule 5 asks for) but it is **not stable across map edits**: one
/// more anchorable block in the file renumbers others. `BodyId` is `Serialize`; whoever puts
/// one into a save game (`T-020`) is writing down a number that the next map edit invalidates.
///
/// **The order of the three blocks is the design, not a formatting choice.** Strike out
/// first, then take in, then update: an id is never handed out twice, so a body taken in
/// here can never be one that the mailbox is still holding, and a report for a body that has
/// just been replaced cannot slip out. The reverse order would let a `BodyGone` for id 42
/// leave in the same tick in which 42 was re-inserted.
///
/// Nothing here runs over all entities per tick: `Without<BodyId>` is empty from the second
/// tick on, and `Changed<GlobalTransform>` is empty for a city that stands still (§6 rule 6).
pub fn maintain_index(
    mut commands: Commands,
    mut index: ResMut<SpatialIndex>,
    mut ids: ResMut<IdCounter>,
    tick: Res<Tick>,
    mut weg: MessageWriter<BodyGone>,
    neu: Query<(Entity, &Body, &GlobalTransform), Without<BodyId>>,
    bekannt: Query<(&BodyId, &Body, &GlobalTransform), Changed<GlobalTransform>>,
) {
    // 1. What the observer collected since the last fixed step — however many frames ago that
    //    was.
    //
    //    The message goes out **whether or not** `remove` found anything. The two errors are
    //    not the same size: a duplicate `BodyGone` makes a hook that has already let go let go
    //    again (nothing happens), a missing one leaves a rope taut on a house that no longer
    //    exists. `vector::hook` reads it as `gone.contains(&body)`.
    for id in index.take_removals() {
        index.remove(id);
        weg.write(BodyGone { body: id, tick: tick.0 });
    }

    // 2. New bodies. **This is the only place in the game that hands out a `BodyId`** —
    //    consecutive out of `IdCounter`, so that two machines number the same city the same
    //    way (`docs/multiplayer.md` rule 5). Without this loop no entity in the world carries
    //    an id, `vector::aim` reports `body: None` on every hit and every hook shot ends as
    //    `ReleaseReason::NoAnchor`. That was `B-001`.
    //
    //    `commands.entity(..).insert(..)` takes effect at the end of this system, so the same
    //    body does not turn up in `neu` a second time next tick — and the index is written
    //    **now**, not deferred: `SimulationSystems::Spatial` runs before `World`, and the aim
    //    ray of this very tick must already find the body.
    for (entity, body, world) in &neu {
        let id = ids.next_body();
        index.insert(entry_from(id, body, world));
        commands.entity(entity).insert(id);
    }

    // 3. Bodies that moved. `insert` replaces instead of duplicating (`shared::spatial`), so
    //    the same call serves both cases. Nothing does this today — every house is static —
    //    but `F-029` (anchors on titan limbs) is exactly this line, and a stale hull is a hook
    //    that catches where the titan stood a second ago.
    for (id, body, world) in &bekannt {
        index.insert(entry_from(*id, body, world));
    }
}

/// Observer: a [`Body`] disappears.
///
/// Pushes its id into the index's mailbox. The maintainer collects it in the next fixed step
/// and sends [`BodyGone`] — that is what the hooks hanging on it release on.
///
/// `Option<ResMut<SpatialIndex>>`, because an observer can also fire before the resource is
/// inserted (test apps, `App::finish`). A missing index is then not a crash but a body that
/// never existed in the index anyway.
/// `Remove` and not `Despawn`: it fires for a `Body` taken off an entity that lives on **and**
/// for one that goes down with a despawn, and it runs **before** the component is really gone
/// (`bevy_ecs-0.19.0/src/lifecycle.rs:372-382`) — so the entity is still there and still
/// carries its [`BodyId`] when this reads it.
///
/// A body without an id was never in the index (the id and the entry are written in the same
/// breath by [`maintain_index`]), so there is nothing to report and nothing to strike out.
pub fn on_body_removed(
    ereignis: On<Remove, Body>,
    ids: Query<&BodyId>,
    index: Option<ResMut<SpatialIndex>>,
) {
    let Some(mut index) = index else {
        return;
    };
    if let Ok(id) = ids.get(ereignis.entity) {
        index.queue_removal(*id);
    }
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
