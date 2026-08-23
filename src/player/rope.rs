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
//! ## The five decisions in this file
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
//! 5. **The length is a RATCHET: it follows the real distance down and never up** (`B-004`).
//!    `limits = (0, L)` corrects only when the distance *exceeds* `L`
//!    (`avian3d-0.7.0/src/dynamics/joints/mod.rs:326-344`), so a player who closes on his
//!    anchor faster than the rope is taken in leaves the rope **slack**, and a slack rope
//!    constrains nothing. Measured before the fix, on a 50.000 m rope with the reel **not**
//!    held: the enforced length stayed at 50.000 m and the player flew **50.000 m past the
//!    anchor** at 20, 25, 28, 30, 40, 55 and 75 m/s — the whole rope length, at every speed.
//!    With the reel held the overshoot starts exactly at `vector.reel_speed_m_s` = 28 m/s and
//!    grows: 8.667 m at 40, 16.000 m at 55, 22.500 m at 75. [`shorten_ropes`] therefore also
//!    takes up slack — without a rate cap and without the button — and the user's sentence
//!    („wenn ich mich festhake und ganz schnell ran fliege kann ich overshooten") is answered
//!    by the *length*, not by a faster reel.
//!
//!    ⚠️ **It was accused of costing this project its headline speed, and it was measured
//!    innocent** (2026-08-10). `scripts/game-full.txt` ACT 1 fell from 46.414 m/s to
//!    19.344 m/s on the day the take-up landed. Isolated behind a temporary switch, ACT 1
//!    reads **19.344 m/s and 9.881 m to the last digit with the take-up on, off, only while
//!    the reel is held, and with a 2 m slack margin** — four ropes, one number. The whole
//!    delta belongs to `src/player/locomotion.rs`; with that file at its previous revision
//!    ACT 1 reads 46.414 m/s and 13.064 m, again with the take-up both on and off.
//!    `tests/vector_rope.rs::measure_the_overshoot_past_the_anchor` says the same thing in
//!    the small: over 16 approaches the peak speed equals `v0` in **every** row with the
//!    take-up on and with it off — there is no whip for the ratchet to eat, while the
//!    overshoot goes from 50.000 m to 3.000 m (`vector.min_rope_m`).
//!
//! ## `B-003` — a teleport lets go of every rope, and says so
//!
//! A `WarpPlayer` puts the player somewhere else in one tick. A `DistanceJoint` that survives
//! that does not follow him — it **pulls him back**, and it does so without a single line
//! anywhere. Measured: one tick after a 55.73 m warp off a 9 m rope the player was **47.93 m**
//! back toward his old anchor, `script run finished` reported success and two of three kills in
//! `scripts/game-full.txt` silently did not happen.
//!
//! The release runs on **two** rails, and it needs both:
//!
//! 1. **The joint goes in `SimulationSystems::Drive`** ([`detach_ropes`], the same system that
//!    already carries out `HookReleased` and `BodyGone`) — one stage **before** the warp is
//!    written in `Integrate`, so avian never sees a joint and a teleported body in the same
//!    step. Doing it inside `apply_warps` would be too late: its `Commands` are applied at the
//!    next sync point, and that is behind `PhysicsSystems::StepSimulation`.
//! 2. **The arm learns about it through [`RopeLength::overextended`]**, set by
//!    [`sync_rope_length`] — the only writer of that component. `vector::hook` is the **only
//!    writer of `Hook`** and reads that flag one tick later (`src/vector/hook.rs`, header);
//!    asking through it is what keeps `Hook` at one writer instead of two. Without this rail
//!    the joint would be gone and the arm would stay `Anchored` on nothing — a hook that can
//!    never fire again.
//!
//! ⚠️ **The reason the player is told is `ReleaseReason::Overextended`, and that is a
//! stand-in.** The honest reason would be `ReleaseReason::Warped`, and that enum is
//! `src/shared/message.rs:113-123`, which this domain does not own. `Overextended` is the
//! closest of the four: it is the one that already means "the rope's length cannot be honoured
//! any more", and it is the one `hud` and `sound` already treat as "the rope tore" rather than
//! "the player let go". A warp under `vector.hook_range_m` — 55 m of the file's 90 — would
//! never trigger it on distance alone, which is exactly why the flag is set here by hand.
//!
//! ## What is deliberately not here
//!
//! The anchor marker **does not follow a moving carrier**. `vector::hook` keeps `tip_m` in
//! the carrier's frame and therefore already does, but moving carriers are `F-029` and today
//! every anchorable body in the world is a static block. Wiring the marker to the tip is
//! three lines; it is in this job's report as an open item rather than as untested code.

use avian3d::prelude::{
    DistanceJoint, JointDisabled, Position, RigidBody, RigidBodyDisabled, SubstepSchedule, Substeps,
};
use bevy::prelude::*;

use crate::data::{GameData, RopeForceModel};
use crate::shared::{
    BodyGone, BodyId, HitStop, Hook, HookAnchored, HookReleased, PlayerId, ReelSpeed, RopeLength,
    Side, WarpPlayer,
};

/// `B-004` — **the only way a rope is allowed to stop existing.**
///
/// A rope entity is never despawned directly. The [`DistanceJoint`] component comes off
/// **first**, in its own command, and only then does the entity die. That single ordering is
/// what makes the release survivable on *every* tick, and the argument is avian's, not ours.
///
/// avian has exactly four events that move a joint in or out of a
/// [`PhysicsIsland`](avian3d::prelude::PhysicsIslands) — three of them abort the process if the
/// joint's dynamic end is carrying [`RigidBodyDisabled`], because a disabled body has no
/// `BodyIslandNode` and the rope's other end is `RigidBody::Static` and never had one:
///
/// | event | avian entry point | needs the body enabled |
/// |---|---|---|
/// | the joint component is **added** without [`JointDisabled`] | `add_joint_to_graph::<T, Add, T, Without<JointDisabled>>` → `merge_islands` | **yes** — else `islands/mod.rs:820` |
/// | [`JointDisabled`] is **added** | `remove_joint_from_graph::<Add, (Disabled, JointDisabled)>` | **yes** — else `islands/mod.rs:786` |
/// | [`JointDisabled`] is **removed** | `add_joint_to_graph::<T, Remove, JointDisabled, …>` → `merge_islands` | **yes** — else `islands/mod.rs:820` |
/// | the joint component is **removed** | `remove_joint_from_graph::<Remove, T>` | only while the joint is still *in* the graph |
///
/// `combat::hitstop` keeps the first two safe by ordering its commands (`JointDisabled` before
/// `RigidBodyDisabled` on the way in, the other way round on the way out). **The third one has
/// no ordering that helps, because a despawn *is* the removal of `JointDisabled`** — and that
/// is the abort a player hit by letting go of the hook inside the 0.12 s impact frame
/// (`docs/BUGS.md` `B-004`, `docs/FINDINGS.md` FIND-072).
///
/// The fourth row is the one with a way out: `remove_joint_from_graph` starts with
/// `joint_graph.get(entity)` and **returns** when the joint is not in the graph — which is
/// exactly the state a `JointDisabled` joint is in. And once the [`DistanceJoint`] component is
/// gone, the despawn's `Remove, JointDisabled` trigger finds nothing: that observer's query is
/// `Query<(&T, …), With<JointComponentId>>` and no longer matches
/// (`avian3d-0.7.0/src/dynamics/solver/joint_graph/plugin.rs:116-131`).
///
/// So one unconditional order covers **both** columns, and there is no `if frozen` branch that
/// a later reader can get the wrong way round:
///
/// - **body enabled, joint live** → row 4 with the joint in the graph: the island's
///   `joint_count` goes 1 → 0 with the island still there. Then the despawn triggers nothing.
/// - **body disabled, joint `JointDisabled`** → row 4 with the joint *not* in the graph: an
///   early return. Then the despawn triggers nothing.
///
/// ⚠️ **Two commands, not one `EntityCommands` chain that ends in `despawn`.** `Commands` are
/// applied in the order they are queued and each one's hooks and observers run inside its own
/// application, so the `remove` is fully processed — graph, island, `JointComponentId` — before
/// the `despawn` is looked at. Putting the `despawn` first, or leaving the `remove` out, is the
/// bug back.
fn despawn_rope(commands: &mut Commands, joint: Entity, anchor: Entity) {
    commands.entity(joint).remove::<DistanceJoint>();
    commands.entity(joint).despawn();
    commands.entity(anchor).despawn();
}

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

/// **`B-003` — does this teleport leave the rope something it can still be?**
///
/// The 2026-08-10 fix released **every** rope on **every** `warp`, whatever the distance. It is
/// right for the case it was written for — 55.73 m off a 9.00 m rope dragged the player 47.93 m
/// back in a single tick — and wrong for the 35 scripts that use `warp` as the way to put a
/// player somewhere. It cost the `F-029` round two runs and it was written down nowhere a
/// script author looks (`docs/FINDINGS.md` FIND-116).
///
/// **The rule is not a taste, it is what the joint does.** `limits = (0, L)` corrects only when
/// the distance *exceeds* `L` (`avian3d-0.7.0/src/dynamics/joints/mod.rs:329-343`), so a
/// teleport that lands the player inside his own rope length cannot move him at all: the rope
/// goes slack, and [`shorten_ropes`]'s take-up spools the slack in like any other. Only the
/// excess is a drag, and it is a drag of roughly its own size — 46.73 m of excess measured
/// 47.93 m of drag. So the only question left is how much excess may survive, and that number
/// is `game.ron: vector.warp_rope_slack_m` with both of its bounds derived there.
///
/// A warp *toward* the anchor keeps the rope by the same rule and on purpose: nothing is pulled,
/// the player stands exactly where he was put (§12c), and the length ratchets down to the
/// distance that now really exists (`B-004`).
///
/// Pure — four numbers in, a `bool` out. Non-finite input answers `false`, i.e. cuts, which is
/// the safe direction.
pub fn warp_keeps_the_rope(dest_m: Vec3, anchor_m: Vec3, length_m: f32, slack_m: f32) -> bool {
    (dest_m - anchor_m).length() <= length_m + slack_m
}

/// `F-004` — creates the joint the moment a hook bites.
///
/// `L` is the **current** player-to-anchor distance, floored at `vector.min_rope_m`: the rope
/// starts at the length it really has, so anchoring never yanks the player.
///
/// A player who is warped in this same tick has his rope measured **from where the warp puts
/// him** (`B-003`): the `Position` read here is the one from before the teleport, and
/// `apply_warps` runs one stage later, in `Integrate`. Until 2026-08-19 such a hook got no rope
/// at all, which threw the shot away even when the teleport was five centimetres — see
/// [`warp_keeps_the_rope`].
pub fn attach_ropes(
    mut commands: Commands,
    data: Res<GameData>,
    mut messages: MessageReader<HookAnchored>,
    mut warped: MessageReader<WarpPlayer>,
    players: Query<(Entity, &PlayerId, &Position, Has<RigidBodyDisabled>)>,
    ropes: Query<(Entity, &Rope)>,
) {
    let min_rope_m = data.game.vector.min_rope_m;
    let model = data.game.vector.rope_force_model;
    let warped: Vec<(PlayerId, Vec3)> = if warped.is_empty() {
        Vec::new()
    } else {
        warped.read().map(|m| (m.player, Vec3::new(m.pos_x, m.pos_y, m.pos_z))).collect()
    };

    for anchored in messages.read() {
        let Some((body_entity, _, position, frozen)) =
            players.iter().find(|(_, id, _, _)| **id == anchored.player)
        else {
            // A hook whose player vanished between `Intent` and `Drive`. Nothing to hang.
            continue;
        };
        // The spot the length is measured from: where the player will stand at the end of this
        // tick, not where he stands while the message is read.
        let from_m = warped
            .iter()
            .find(|(id, _)| *id == anchored.player)
            .map_or(position.0, |(_, dest)| *dest);

        // Defensive: `vector::hook` only anchors out of `Idle`, so a second rope on the same
        // side cannot happen — but two joints on one side would fight each other silently,
        // and that is not a bug anybody would find from the outside.
        for (entity, rope) in &ropes {
            if rope.player == anchored.player && rope.side == anchored.side {
                // `B-004`: through the choke point, because this branch can fire while the
                // player is frozen — a second hook on the same side inside an impact frame.
                despawn_rope(&mut commands, entity, rope.anchor);
            }
        }

        let point_m = anchored.point();
        let length_m = (point_m - from_m).length();
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

        let rope = (
            Name::new(format!("rope_p{}_{:?}", anchored.player.0, anchored.side)),
            Rope {
                player: anchored.player,
                side: anchored.side,
                body: anchored.body,
                anchor,
                body_entity,
            },
        );
        // `(0, L)`: pulls, never pushes. See the module header.
        let joint = DistanceJoint::new(anchor, body_entity)
            .with_local_anchor1(Vec3::ZERO)
            .with_local_anchor2(Vec3::ZERO)
            .with_limits(0.0, length_m);
        // `B-004`, third face — a hook that bites **inside** an impact frame. A disabled body
        // has no island (`combat::hitstop`, header), and `add_joint` merges the islands of the
        // two ends, so a live joint born here aborts the process in `merge_islands` with
        // "Neither body … is in an island" instead of hanging on the player. Born disabled it
        // is never registered in the first place (`avian3d-0.7.0/src/dynamics/solver/
        // joint_graph/plugin.rs:79`, the `Without<JointDisabled>` filter), and
        // `combat::hitstop::advance` takes the marker off with every other one when the freeze
        // lifts. The rope exists from this tick on either way — its length and its
        // `RopeLength` are the ones measured here.
        //
        // ⚠️ **In the bundle, not as a second `insert`.** `Commands::spawn` is applied on its
        // own and triggers avian's `On<Add, DistanceJoint>` observer right there; a marker
        // queued behind it arrives one command too late and the joint is already registered.
        //
        // `FIND-149`, and it is the whole of [`RopeForceModel::Drive`]: **the joint is simply
        // not built.** Not `JointDisabled` — `combat::hitstop::advance` takes that marker off
        // every joint of a body when the freeze lifts (`src/combat/hitstop.rs:295`), so a rope
        // disabled for a *model* reason would come alive again after the first hit the player
        // takes, in the middle of a flight, and nothing would say why. A rope with no
        // `DistanceJoint` is invisible to `hitstop::joints_of`, to [`shorten_ropes`] and to
        // avian's island bookkeeping — and that is the correct meaning of "the rope applies no
        // force of its own".
        match model {
            RopeForceModel::Drive => {
                commands.spawn(rope);
            }
            RopeForceModel::Pendulum if frozen => {
                commands.spawn((rope, joint, JointDisabled));
            }
            RopeForceModel::Pendulum => {
                commands.spawn((rope, joint));
            }
        }

        info!(
            "rope {:?} of player {} attached at {:.2} m (t={})",
            anchored.side, anchored.player.0, length_m, anchored.tick
        );
    }
}

/// Removes the joint again — on **every** release reason, on a carrier that is gone, and on a
/// player who was teleported away (`B-003`).
///
/// `BodyGone` is read here as well as in `vector::hook`, and on purpose: `hook` turns it into
/// a `HookReleased`, which is what this system really acts on, but a carrier that disappears
/// without anybody having a hook on it must not leave an anchor marker standing in the world.
/// Both paths despawn at most once — the query is the truth about what exists.
///
/// `WarpPlayer` is the third, and it is the one that has to be handled **here** rather than
/// where the teleport happens: this system runs in `SimulationSystems::Drive`, one stage before
/// `player::apply_warps` moves the body in `Integrate`, so the joint is already gone by the
/// time avian's first system of that same tick looks for one. See the module header, `B-003`.
///
/// ⚠️ **Not every warp**, since 2026-08-19: only the ones that leave the player further from
/// his anchor than his rope is long, plus `vector.warp_rope_slack_m`. [`warp_keeps_the_rope`]
/// is the whole rule and the reason it is that one.
pub fn detach_ropes(
    mut commands: Commands,
    data: Res<GameData>,
    mut released: MessageReader<HookReleased>,
    mut gone: MessageReader<BodyGone>,
    mut warped: MessageReader<WarpPlayer>,
    ropes: Query<(Entity, &Rope, Option<&DistanceJoint>)>,
    anchors: Query<&Transform>,
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
    let warped: Vec<(PlayerId, Vec3)> = if warped.is_empty() {
        Vec::new()
    } else {
        warped.read().map(|m| (m.player, Vec3::new(m.pos_x, m.pos_y, m.pos_z))).collect()
    };
    if released.is_empty() && gone.is_empty() && warped.is_empty() {
        return;
    }
    let slack_m = data.game.vector.warp_rope_slack_m;

    for (entity, rope, joint) in &ropes {
        // **A rope with no joint cannot be dragged by a teleport, so a teleport cannot cut it**
        // (`FIND-149`, [`RopeForceModel::Drive`]). `B-003` exists because the constraint
        // corrects the excess inside one substep and throws the player 14 m/s per centimetre of
        // it; with no constraint there is no excess and no kick. What still ends such a rope is
        // [`sync_rope_length`]'s `overextended` — the wall winning at `hook_range_m` — one tick
        // later, which is the same path an ordinary flight out of range takes.
        let enforced_m = joint.map_or(f32::INFINITY, |j| j.limits.max);
        // The rule, and the only place it is decided: a teleport that lands inside the rope's
        // own length has nothing for the joint to correct, so the rope survives it. A missing
        // anchor transform counts as "cut" — the safe direction.
        let by_warp = warped.iter().find(|(id, _)| *id == rope.player).is_some_and(|(_, dest)| {
            let keep = anchors.get(rope.anchor).is_ok_and(|a| {
                warp_keeps_the_rope(*dest, a.translation, enforced_m, slack_m)
            });
            if !keep {
                let reach_m =
                    anchors.get(rope.anchor).map_or(f32::NAN, |a| (*dest - a.translation).length());
                // The line `B-003` did not have. A rope that is cut by something the player
                // did not ask for has to leave a trace — `scripts/game-full.txt` lost two of
                // three kills to exactly this happening in silence. Since 2026-08-19 it also
                // says by how much, so a script author can see whether he was 5 cm or 55 m out.
                info!(
                    "rope {:?} of player {} cut: the warp left him {reach_m:.2} m from his                      anchor on a {:.2} m rope (B-003)",
                    rope.side, rope.player.0, enforced_m
                );
            }
            !keep
        });
        if released.contains(&(rope.player, rope.side)) || gone.contains(&rope.body) || by_warp {
            // `B-004`, the third face: this is the release the player actually makes, and on
            // seven of every sixty ticks its player is frozen. `despawn_rope` is the only
            // ordering that survives both states — see its own doc comment.
            despawn_rope(&mut commands, entity, rope.anchor);
        }
    }
}

/// `F-005` — the two things that shorten `limits.max`, **both once per substep.**
///
/// 1. **The reel** takes `ReelSpeed * substep_dt` off the length while the button is held. It
///    costs gas, it is capped at `vector.reel_speed_m_s`, and it is the only one of the two
///    that can pull a player *toward* an anchor he is not already approaching.
/// 2. **The take-up** (`B-004`) follows the length down to the distance that really exists —
///    **no rate cap, no button, and never upward.** A rope that is given slack spools it in;
///    a rope that is taut is not touched. This is what makes the length a **ratchet**.
///
/// That the substep placement works at all is in the source: `solve_xpbd_joint::<DistanceJoint>`
/// runs inside the [`SubstepSchedule`] and re-reads `&mut DistanceJoint` in **every** substep
/// (`avian3d-0.7.0/src/dynamics/solver/xpbd/plugin.rs:160-203`), using `self.limits` directly
/// (`.../xpbd/joints/distance.rs:80`). A change made here therefore lands in the very next
/// substep.
///
/// `Time<Substeps>` and not `reel_speed / (simulation_hz * substeps)`: avian advances that
/// clock by `timestep / SubstepCount` once per physics tick
/// (`avian3d-0.7.0/src/schedule/mod.rs:246-254`), so the step size is derived from **one**
/// number instead of from two that drift apart the day `substeps` moves in `game.ron`.
///
/// # `B-004` — why the take-up is here and not in a system of its own
///
/// `limits.max` has exactly one writer, and that is a rule, not a preference
/// (`docs/architecture.md`, authority table; §6 rule 3). A second system that also lowered the
/// length would be a second writer of the same field, and the two would race inside a schedule
/// that runs 24 times per tick.
///
/// **Per substep and not per tick**, for the same reason the reel is: at
/// `vector.max_speed_m_s` = 75 the body moves 1.25 m per tick and 0.052 m per substep. A
/// take-up that only ran once per tick would leave up to a tick's worth of travel as slack —
/// exactly the hole this fixes, one order of magnitude smaller.
///
/// **`min_rope_m` still wins.** The floor is applied last, so a player who is closer to his
/// anchor than `vector.min_rope_m` does not get a rope shorter than the file allows.
///
/// **It cannot turn anchoring into a yank.** [`attach_ropes`] is born at exactly the distance
/// that exists, so on the first substep `min(limits.max, distance)` is `limits.max` and this
/// changes nothing at all.
///
/// **And it does not eat the swing.** Measured before the take-up existed
/// (`tests/vector_rope.rs::measure_the_dip_of_a_swing_below_its_own_length`): over 4 s on an
/// 8.000 m rope at `v0` 8/12/16/30 m/s, with gravity on and off, the distance to the anchor
/// dips below the enforced length by **0.0000 m** — the solver contributes no measurable
/// slack, so there is nothing for the ratchet to bite on. The one case that does dip is
/// `v0 = 20` with gravity on (0.7093 m), and there the rope is **really** slack: the player
/// goes over the top of the anchor and a real rope would hang loose too.
pub fn shorten_ropes(
    time: Res<Time<Substeps>>,
    data: Res<GameData>,
    players: Query<(&PlayerId, &ReelSpeed, Has<HitStop>)>,
    positions: Query<&Position>,
    mut ropes: Query<(&Rope, &mut DistanceJoint)>,
) {
    let min_rope_m = data.game.vector.min_rope_m;
    let dt = time.delta_secs();

    for (rope, mut joint) in &mut ropes {
        let mut next_m = joint.limits.max;
        let player = players.iter().find(|(id, _, _)| **id == rope.player);

        // 0. `B-004`, second face — **a frozen player does not spool rope.**
        //
        // `combat::hitstop` takes the body out of the solver for the impact frame; the length
        // is the one thing left that would keep moving. Measured before this line existed, on
        // `scripts/f-flight-cut.txt`: two frozen ticks of the torso hit stop stored **0.93 m**
        // of rope, which the very next unfrozen tick paid back as 74.700 m/s — `game.ron`'s
        // `vector.max_speed_m_s`, i.e. the clamp and not a speed anybody chose. Both halves of
        // the shortening are skipped, the reel included: the reel is not paid for either
        // (`vector::gas` debits per tick, and a frozen tick reels nothing).
        if player.is_some_and(|(_, _, frozen)| frozen) {
            continue;
        }

        // 1. The reel — a rate, and only while the button is held.
        if dt > 0.0
            && let Some((_, reel, _)) = player
        {
            let rate_m_s = reel.m_s[rope.side.index()];
            if rate_m_s > 0.0 {
                next_m -= rate_m_s * dt;
            }
        }

        // 2. The take-up — the slack the player has flown in, all of it, at once.
        //
        // The joint's own ends, so this is the very distance `DistanceLimit` measures: both
        // local anchors are `Vec3::ZERO` (see `attach_ropes`). A missing `Position` means the
        // anchor marker has not been through avian's prepare stage yet — then there is nothing
        // to measure and the length stays where it is, which is the safe direction.
        if let (Ok(anchor), Ok(body)) =
            (positions.get(rope.anchor), positions.get(rope.body_entity))
        {
            let distance_m = (anchor.0 - body.0).length();
            // A non-finite distance reads as "the player has vanished" (§9d) and must not be
            // allowed to set a length — `min` with a NaN would take the NaN.
            if distance_m.is_finite() {
                next_m = next_m.min(distance_m);
            }
        }

        let next_m = next_m.max(min_rope_m);
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
///
/// **A warp raises the same flag** (`B-003`), for every arm that is holding on — see the module
/// header for why the flag and not a write to `Hook`, and why the reason the player is told is
/// a stand-in. `Hook` is only **read** here; its one writer stays `vector::hook::update_hooks`.
pub fn sync_rope_length(
    data: Res<GameData>,
    mut warped: MessageReader<WarpPlayer>,
    ropes: Query<(&Rope, Option<&DistanceJoint>)>,
    anchors: Query<&Transform>,
    mut players: Query<(&PlayerId, &Position, &Hook, &mut RopeLength)>,
) {
    let hook_range_m = data.game.vector.hook_range_m;
    let slack_m = data.game.vector.warp_rope_slack_m;
    let min_rope_m = data.game.vector.min_rope_m;
    // Collected once instead of read per player — a `MessageReader` has one cursor, and the
    // second player would find it empty. `Vec::new()` does not allocate.
    let warped: Vec<(PlayerId, Vec3)> = if warped.is_empty() {
        Vec::new()
    } else {
        warped.read().map(|m| (m.player, Vec3::new(m.pos_x, m.pos_y, m.pos_z))).collect()
    };

    for (id, position, hook, mut length) in &mut players {
        // Assembled whole and assigned whole: a side without a rope is `0.0` because the
        // default is, not because somebody remembered to clear it. At most two ropes per
        // player, so the inner scan is bounded by 2 x players and not by "all entities" (§11).
        let mut next = RopeLength::default();
        for (rope, joint) in &ropes {
            if rope.player != *id {
                continue;
            }
            let i = rope.side.index();
            let reach_m: Option<f32> =
                anchors.get(rope.anchor).ok().map(|a| (a.translation - position.0).length());
            // **With no joint there is no enforced length, so what is published is the length
            // the rope really has** (`FIND-149`, [`RopeForceModel::Drive`]). The type's contract
            // is "the enforced rope length per side, `0.0` means no constraint", and `0.0` here
            // would be a lie of a different kind: `vector::hook` would keep an arm anchored on a
            // rope the HUD draws as absent. The floor is `min_rope_m` for the same reason
            // [`attach_ropes`] applies it — a length under it is not a length this game has.
            next.lengths_m[i] = match joint {
                Some(joint) => joint.limits.max,
                None => reach_m.unwrap_or(min_rope_m).max(min_rope_m),
            };
            if let Some(reach_m) = reach_m {
                next.overextended[i] = reach_m > hook_range_m;
            }
        }
        if let Some((_, dest_m)) = warped.iter().find(|(w, _)| w == id) {
            // `B-003`. Only the arms that really hold on: an `Idle` arm would carry a flag
            // nobody reads, and `RopeLength` would be marked changed for every player every
            // `warp` (§11).
            //
            // ⚠️ **And only the arms whose rope the warp really cut.** Since 2026-08-19 a
            // teleport inside the rope's own length keeps it ([`warp_keeps_the_rope`]), and an
            // arm flagged here would be let go of by `vector::hook` in the next tick with the
            // joint still standing. The predicate is asked a second time rather than the
            // absence of the rope being read off the query: `detach_ropes`' despawn is a
            // `Command` and whether it has been applied by the time this system runs is not a
            // thing this file gets to depend on.
            for side in Side::ALL {
                if !hook.arm(side).state.is_anchored() {
                    continue;
                }
                let survives = ropes
                    .iter()
                    .find(|(rope, _)| rope.player == *id && rope.side == side)
                    .and_then(|(rope, joint)| {
                        // A jointless rope (`Drive`) has nothing that could drag the player, so
                        // it survives every teleport — the same rule [`detach_ropes`] applies,
                        // and it has to be the same one or the flag and the joint would
                        // disagree about whether the rope still exists.
                        let enforced_m = joint.map_or(f32::INFINITY, |j| j.limits.max);
                        anchors.get(rope.anchor).ok().map(|a| (a.translation, enforced_m))
                    })
                    .is_some_and(|(anchor_m, length_m)| {
                        warp_keeps_the_rope(*dest_m, anchor_m, length_m, slack_m)
                    });
                if !survives {
                    next.overextended[side.index()] = true;
                }
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
