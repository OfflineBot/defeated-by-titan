//! `F-004` The rope — **an avian [`DistanceJoint`] with `limits = (0, L)`, and nothing else.**
//!
//! Three rounds of measurement retired the hand-written solver in `shared::rope`
//! (`docs/measurements/rope-decision.md`). The number that decided it: reeling in **through
//! the joint** reaches 58.23 m/s out of `v0 = 20` because the joint preserves angular
//! momentum, while the hand-written clamp gives exactly **20.000** — it eats the reel-in, and
//! the reel-in *is* the feel of the Vector Gear. This file does not re-litigate that; it
//! carries it out.
//!
//! `limits.min = 0.0` means **there is no lower limit**: `DistanceLimit::compute_correction`
//! corrects only when the distance exceeds the maximum
//! (`avian3d-0.7.0/src/dynamics/joints/mod.rs:329-343`). That *is* the definition of "a rope
//! pulls, it does not push" — no `if` here says so, the limit does.
//!
//! ## The four decisions in this file
//!
//! 1. **Reeling in happens per SUBSTEP, never per tick.** Measured with 24 substeps, `v0 50`:
//!    per tick the shortening injects `rate x SubstepCount` and the player reaches
//!    **677.66 m/s** and goes **2.53 m** through a wall; per substep it is 130.22 m/s, and
//!    together with the [`MaxLinearSpeed`](avian3d::prelude::MaxLinearSpeed) already on the
//!    body (`super::spawn_player`) 75.44 m/s with a worst wall penetration of −0.0043 m.
//!    Both screws are needed and they act on different things: per substep holds the *rope*,
//!    the clamp holds the *speed*. [`shorten_ropes`] therefore hangs in avian's
//!    [`SubstepSchedule`], and `tests/vector_rope.rs::f005_shortening_happens_per_substep_not_per_tick`
//!    is the guard that goes red the day somebody "simplifies" it into `FixedUpdate`.
//! 2. **The anchor end gets an entity of its own.** A joint needs a `RigidBody` on both ends,
//!    and the hook anchors on a [`BodyId`] out of [`SpatialIndex`](crate::shared::SpatialIndex)
//!    — an index entry, which by design carries **no** `Entity` (`shared/spatial.rs`: "an
//!    `Entity` belongs in nothing that gets saved or sent"). So there is nothing to hang the
//!    other end on, and [`attach_ropes`] spawns a `RigidBody::Static` marker at the anchor
//!    point, exactly as `examples/probe_avian.rs::anker` did in the measurement. It lives and
//!    dies with the joint.
//! 3. **The joint sits on an entity of its own, not on the player and not on the anchor.**
//!    Two ropes per player, and a component exists at most once per entity.
//! 4. **`RopeLength` is written after the physics step, by exactly one system.**
//!    [`sync_rope_length`] is the only writer of that component (`docs/architecture.md`,
//!    authority table). `vector::hook` reads `overextended` in the **next** tick — one tick of
//!    lag is the price for `Hook` having exactly one writer.
//!
//! ## What is deliberately not here
//!
//! The anchor marker **does not follow a moving carrier**. `vector::hook` keeps `tip_m` in
//! the carrier's frame and therefore already does, but moving carriers are `F-029` and today
//! every anchorable body in the world is a static block. Wiring the marker to the tip is
//! three lines; it is in this job's report as an open item rather than as untested code.

use avian3d::prelude::{DistanceJoint, Position, RigidBody, SubstepSchedule, Substeps};
use bevy::prelude::*;

use crate::data::GameData;
use crate::shared::{
    BodyGone, BodyId, HookAnchored, HookReleased, PlayerId, ReelSpeed, RopeLength, Side,
};

/// One rope, as a component on the joint entity.
///
/// Carries [`PlayerId`] and [`BodyId`] — stable ids, not `Entity` (`docs/multiplayer.md`
/// rule 5). The two `Entity` fields are the joint's own ends and never leave this file.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Rope {
    pub player: PlayerId,
    pub side: Side,
    /// The carrier the hook hangs on — the key `BodyGone` speaks in.
    pub body: BodyId,
    /// The `RigidBody::Static` marker at the anchor point (decision 2).
    pub anchor: Entity,
    /// The player's body. The joint's other end.
    pub body_entity: Entity,
}

/// `F-004` — creates the joint the moment a hook bites.
///
/// `L` is the **current** player-to-anchor distance, floored at `vector.min_rope_m`: the rope
/// starts at the length it really has, so anchoring never yanks the player.
pub fn attach_ropes(
    mut commands: Commands,
    data: Res<GameData>,
    mut messages: MessageReader<HookAnchored>,
    players: Query<(Entity, &PlayerId, &Position)>,
    ropes: Query<(Entity, &Rope)>,
) {
    let min_rope_m = data.game.vector.min_rope_m;

    for anchored in messages.read() {
        let Some((body_entity, _, position)) =
            players.iter().find(|(_, id, _)| **id == anchored.player)
        else {
            // A hook whose player vanished between `Intent` and `Drive`. Nothing to hang.
            continue;
        };

        // Defensive: `vector::hook` only anchors out of `Idle`, so a second rope on the same
        // side cannot happen — but two joints on one side would fight each other silently,
        // and that is not a bug anybody would find from the outside.
        for (entity, rope) in &ropes {
            if rope.player == anchored.player && rope.side == anchored.side {
                commands.entity(rope.anchor).despawn();
                commands.entity(entity).despawn();
            }
        }

        let point_m = anchored.point();
        let length_m = (point_m - position.0).length();
        if !length_m.is_finite() {
            // NaN in a `Transform` reads as "the player has vanished" (§9d). A rope that
            // cannot be measured is not built.
            warn!(
                "rope {:?} of player {} got a non-finite length — no joint",
                anchored.side, anchored.player.0
            );
            continue;
        }
        // The floor from the file, from the first tick on: a hook fired at a wall two meters
        // away must not put the camera inside it.
        let length_m = length_m.max(min_rope_m);

        let anchor = commands
            .spawn((
                Name::new(format!("rope_anchor_p{}_{:?}", anchored.player.0, anchored.side)),
                RigidBody::Static,
                Transform::from_translation(point_m),
            ))
            .id();

        commands.spawn((
            Name::new(format!("rope_p{}_{:?}", anchored.player.0, anchored.side)),
            Rope {
                player: anchored.player,
                side: anchored.side,
                body: anchored.body,
                anchor,
                body_entity,
            },
            // `(0, L)`: pulls, never pushes. See the module header.
            DistanceJoint::new(anchor, body_entity)
                .with_local_anchor1(Vec3::ZERO)
                .with_local_anchor2(Vec3::ZERO)
                .with_limits(0.0, length_m),
        ));

        info!(
            "rope {:?} of player {} attached at {:.2} m (t={})",
            anchored.side, anchored.player.0, length_m, anchored.tick
        );
    }
}

/// Removes the joint again — on **every** release reason, and on a carrier that is gone.
///
/// `BodyGone` is read here as well as in `vector::hook`, and on purpose: `hook` turns it into
/// a `HookReleased`, which is what this system really acts on, but a carrier that disappears
/// without anybody having a hook on it must not leave an anchor marker standing in the world.
/// Both paths despawn at most once — the query is the truth about what exists.
pub fn detach_ropes(
    mut commands: Commands,
    mut released: MessageReader<HookReleased>,
    mut gone: MessageReader<BodyGone>,
    ropes: Query<(Entity, &Rope)>,
) {
    // Collected once instead of read per rope: a `MessageReader` has one cursor, and the
    // second rope would find it empty. `Vec::new()` does not allocate.
    let released: Vec<(PlayerId, Side)> = if released.is_empty() {
        Vec::new()
    } else {
        released.read().map(|m| (m.player, m.side)).collect()
    };
    let gone: Vec<BodyId> = if gone.is_empty() {
        Vec::new()
    } else {
        gone.read().map(|m| m.body).collect()
    };
    if released.is_empty() && gone.is_empty() {
        return;
    }

    for (entity, rope) in &ropes {
        if released.contains(&(rope.player, rope.side)) || gone.contains(&rope.body) {
            commands.entity(rope.anchor).despawn();
            commands.entity(entity).despawn();
        }
    }
}

/// `F-005` — shortens `limits.max` by `ReelSpeed * substep_dt`, **once per substep.**
///
/// That this works at all is in the source: `solve_xpbd_joint::<DistanceJoint>` runs inside
/// the [`SubstepSchedule`] and re-reads `&mut DistanceJoint` in **every** substep
/// (`avian3d-0.7.0/src/dynamics/solver/xpbd/plugin.rs:160-203`), using `self.limits` directly
/// (`.../xpbd/joints/distance.rs:80`). A change made here therefore lands in the very next
/// substep.
///
/// `Time<Substeps>` and not `reel_speed / (simulation_hz * substeps)`: avian advances that
/// clock by `timestep / SubstepCount` once per physics tick
/// (`avian3d-0.7.0/src/schedule/mod.rs:246-254`), so the step size is derived from **one**
/// number instead of from two that drift apart the day `substeps` moves in `game.ron`.
pub fn shorten_ropes(
    time: Res<Time<Substeps>>,
    data: Res<GameData>,
    players: Query<(&PlayerId, &ReelSpeed)>,
    mut ropes: Query<(&Rope, &mut DistanceJoint)>,
) {
    let min_rope_m = data.game.vector.min_rope_m;
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (rope, mut joint) in &mut ropes {
        let Some((_, reel)) = players.iter().find(|(id, _)| **id == rope.player) else {
            continue;
        };
        let rate_m_s = reel.m_s[rope.side.index()];
        if rate_m_s <= 0.0 {
            continue;
        }
        let next_m = (joint.limits.max - rate_m_s * dt).max(min_rope_m);
        // Only on a real change: at the floor the value stops moving, and a joint marked
        // changed in every one of the 24 substeps is 24 lies per tick in every `Changed`
        // filter that comes after it (§11).
        if next_m != joint.limits.max {
            joint.limits.max = next_m;
        }
    }
}

/// Publishes the joint state as [`RopeLength`] — **the only writer of that component.**
///
/// Runs after `PhysicsSystems::Writeback`, so what is published is *this* step's result.
/// `lengths_m` is the **enforced** length (`limits.max`), not the distance actually reached:
/// the type documents it as "the enforced rope length per side, `0.0` means no constraint",
/// and the joint has 2-5 mm of its own error under load
/// (`docs/measurements/rope-decision.md`), which is not a length anybody set.
///
/// `overextended` is the wall winning: the player ended up further from his anchor than
/// `vector.hook_range_m` — the same number `vector::aim` measures the shot against, read from
/// the file and not invented a second time here. `vector::hook` reads the flag in the next
/// tick and releases with `ReleaseReason::Overextended`.
pub fn sync_rope_length(
    data: Res<GameData>,
    ropes: Query<(&Rope, &DistanceJoint)>,
    anchors: Query<&Transform>,
    mut players: Query<(&PlayerId, &Position, &mut RopeLength)>,
) {
    let hook_range_m = data.game.vector.hook_range_m;

    for (id, position, mut length) in &mut players {
        // Assembled whole and assigned whole: a side without a rope is `0.0` because the
        // default is, not because somebody remembered to clear it. At most two ropes per
        // player, so the inner scan is bounded by 2 x players and not by "all entities" (§11).
        let mut next = RopeLength::default();
        for (rope, joint) in &ropes {
            if rope.player != *id {
                continue;
            }
            let i = rope.side.index();
            next.lengths_m[i] = joint.limits.max;
            if let Ok(anchor) = anchors.get(rope.anchor) {
                next.overextended[i] =
                    (anchor.translation - position.0).length() > hook_range_m;
            }
        }
        length.set_if_neq(next);
    }
}

/// Registers everything this file owns. Called by [`super::PlayerPlugin`].
pub fn register(app: &mut App) {
    app.add_systems(
        SubstepSchedule,
        shorten_ropes
            // Before the solver reads `limits` in this substep, not after — otherwise the
            // shortening is always one substep late.
            .before(avian3d::dynamics::solver::schedule::SubstepSolverSystems::WarmStart)
            // The `SubstepSchedule` runs 24 times per tick; an ambiguity report over a system
            // that shares nothing mutable with avian's own is noise, and noise is what makes
            // a real report unread.
            .ambiguous_with_all(),
    );
}
