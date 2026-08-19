//! The box rig — **every length in here comes out of `assets/data/scale.ron`.**
//!
//! Nine entities, and each of them exists for a reason a test can name:
//!
//! ```text
//! root      RigidBody::Kinematic + CustomPositionIntegration + body collider   y = 0 (feet)
//! └ pelvis  the hip node, at y = leg_fraction * h
//!   ├ leg_left / leg_right          two boxes, y = 0 .. leg_fraction * h
//!   └ torso   leans about the hip   y = leg .. leg + torso  (= cortex_fraction * h)
//!     ├ arm_left / arm_right        hinged at shoulder_height_fraction * h
//!     └ head    y = cortex_fraction * h .. h
//!       └ cortex   a Sensor sphere on LAYER_TITAN_CORTEX
//! ```
//!
//! ## Why the cortex hangs under the **head** and not under the root
//!
//! Because then it follows the pose for free, through `GlobalTransform`, and there is no
//! second place that has to remember to move it. Parented to the pelvis it would sit still
//! while the titan leans — and a hit zone that does not follow the body is a hit zone that is
//! right in the screenshot and wrong in the game. `tests/titan.rs::f056_the_cortex_sits_where_scale_ron_says`
//! exists for exactly that mistake.
//!
//! ## Why the cortex height is **not** `height_m * cortex_fraction`
//!
//! `cortex_fraction` is a check rule, not a source (`scale.ron:99-113`). The five cortex
//! heights are the user's figures in metres and come through
//! [`GameData::titan_cortex_height_m`]. Computed from the fraction the `small` class lands
//! 4 cm off. The cortex's local offset under the head is therefore *derived from the metre
//! figure*, not from the fraction.
//!
//! ## What is invented here, and it is not a length
//!
//! `scale.ron` gives the rig its four vertical fractions, its width fraction, the shoulder
//! height and the arm length. It does **not** say how the body width is split between two
//! legs, two arms and a torso, and it does not say how deep a titan is. Those are integer
//! subdivisions of `width_fraction * h` (`w/2`, `w/4`, `w/8`) and the depth is the width — no
//! new number enters the code, only arithmetic on one that is already in the file. A titan
//! that is 2.5 m wide is 2.5 m deep, and each leg is half the body wide.
//!
//! ## The sign convention of the three pose angles
//!
//! **Positive degrees are a right-hand rotation about +X**, i.e. `Quat::from_rotation_x`.
//! Under it `windup_arm_deg: 140` carries the hand up over the head on the **forward** side
//! (Bevy's forward is −Z), `strike_arm_deg: -30` carries it down and 30° past the hanging
//! rest pose, and `windup_lean_deg: 12` tips the torso **back**. The three numbers are
//! ⚠️ UNTUNED in the file and their sign is fixed nowhere else — so it is fixed here, once,
//! and `docs/FINDINGS.md` gets the line.

use avian3d::prelude::{Collider, CollisionLayers, LayerMask, RigidBody, Sensor};
use avian3d::prelude::CustomPositionIntegration;
use bevy::prelude::*;

use crate::data::{GameData, TitanKind};
use crate::shared::{
    Body, BodyMask, Health, HitZone, HitZoneOf, ModelAnchors, StateClock, TitanId, TitanKindName,
    TitanState, Velocity, CORTEX_ANCHOR, LAYER_TITAN_BODY, LAYER_TITAN_CORTEX,
};

/// Which box of the rig this is. Its own type instead of eight marker components, so that a
/// test can walk the whole rig with one query.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitanPart {
    Pelvis,
    Torso,
    Head,
    Cortex,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
}

impl TitanPart {
    /// The eight parts, in the order the rig is built. `tests/titan.rs` iterates this.
    pub const ALL: [TitanPart; 8] = [
        TitanPart::Pelvis,
        TitanPart::Torso,
        TitanPart::Head,
        TitanPart::Cortex,
        TitanPart::ArmLeft,
        TitanPart::ArmRight,
        TitanPart::LegLeft,
        TitanPart::LegRight,
    ];
}

/// Half the extent of one box, in metres, in its own local frame.
///
/// It stands on the entity so that a test can measure the **assembled** rig against
/// `scale.ron` — world AABB out of `GlobalTransform` × half extent — instead of measuring the
/// spawner against itself. For the cortex all three components are the sphere radius.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PartExtent(pub Vec3);

/// Marks the root of a titan: the one entity that carries [`TitanId`], the body collider and
/// the physics body. Every box of the rig is a descendant of it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TitanBody;

/// Every length of one titan, in metres, resolved **once at spawn** out of `scale.ron` and
/// `titan.ron`.
///
/// Resolved once and not per tick, for the same reason `HitStop` counts ticks and not
/// seconds: the conversion from the file's units to the game's happens at the boundary, and
/// after that there is exactly one number in play (`docs/conventions.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TitanRig {
    /// Total body height (`scale.ron: titan.classes[..].height_m`).
    pub height_m: f32,
    /// `width_fraction * height_m`. Also the depth.
    pub width_m: f32,
    /// `leg_fraction * height_m` — ground to hip.
    pub leg_m: f32,
    /// `torso_fraction * height_m` — hip to nape.
    pub torso_m: f32,
    /// What is left over above the nape: `height_m − leg_m − torso_m`.
    pub head_m: f32,
    /// `shoulder_height_fraction * height_m`, above the ground.
    pub shoulder_m: f32,
    /// `arm_fraction * height_m`, from the shoulder to the hand.
    pub arm_m: f32,
    /// **The user's figure in metres**, not `height_m * cortex_fraction`.
    pub cortex_height_m: f32,
    /// `titan.ron: cortex_radius_m`.
    pub cortex_radius_m: f32,
}

impl TitanRig {
    /// Resolves the rig of one kind. `None` means the kind's size class is not in
    /// `scale.ron` — the caller says so loudly instead of building a titan of height 0.
    pub fn of(data: &GameData, kind: &TitanKind) -> Option<TitanRig> {
        let s = &data.scale.titan;
        let height_m = data.titan_height_m(kind)?;
        let cortex_height_m = data.titan_cortex_height_m(kind)?;
        let leg_m = s.leg_fraction * height_m;
        let torso_m = s.torso_fraction * height_m;
        Some(TitanRig {
            height_m,
            width_m: s.width_fraction * height_m,
            leg_m,
            torso_m,
            head_m: height_m - leg_m - torso_m,
            shoulder_m: s.shoulder_height_fraction * height_m,
            arm_m: s.arm_fraction * height_m,
            cortex_height_m,
            cortex_radius_m: kind.cortex_radius_m,
        })
    }

    /// Height of the head's centre above the ground, with the torso upright.
    pub fn head_centre_m(&self) -> f32 {
        self.leg_m + self.torso_m + self.head_m * 0.5
    }

    /// Height of the torso's centre above the ground, with the torso upright.
    pub fn torso_centre_m(&self) -> f32 {
        self.leg_m + self.torso_m * 0.5
    }

    /// Where the cortex sits **inside the head's local frame**.
    ///
    /// **Y is negative**: the nape is the underside of the head, which is what a nape is.
    /// Derived from the metre figure and not from `cortex_fraction`, so that a class whose
    /// cortex is not exactly `height_m * 0.89` still lands where the file says.
    ///
    /// **Z is positive, i.e. backwards** (Bevy's forward is −Z), by half the head's depth.
    /// Two reasons, and neither is a free choice:
    ///
    /// - It is where the design puts it — *"an amber sphere at the back of its neck"*
    ///   (`docs/PLAN-GAME.md` §1), and the husk's whole lesson is the **approach angle**.
    /// - Without it the sphere is **invisible**. The cortex radius did not follow the size
    ///   table (`titan.ron:20-26`, `docs/QUESTIONS.md` Q-019), so a husk's cortex is 1.10 m
    ///   across on a head that is 1.10 m tall: centred on the neck axis, the lower half sits
    ///   inside the torso and the upper half inside the head, and there is nothing to
    ///   photograph. Measured, not guessed — the first `--offscreen` run showed a titan with
    ///   no amber on it anywhere.
    ///
    /// ⚠️ The **hit** is deliberately still a full sphere, reachable from every side. Whether
    /// only a rear hemisphere may be cut is the user's question (`docs/PLAN-GAME.md` §3.4
    /// point 3, `F-060`); moving the marker to where the design says it is does not answer it.
    pub fn cortex_in_head(&self) -> Vec3 {
        Vec3::new(
            0.0,
            self.cortex_height_m - self.head_centre_m(),
            self.head_m * 0.5,
        )
    }

    /// The same point, but taken **out of the model** — with one of its three components
    /// overruled.
    ///
    /// A `cortex` empty in a `.glb` is given in the model root's own space — metres above the
    /// origin between the feet, +Z backwards once `render::model::MODEL_FACES` has turned the
    /// drop's frame into the game's (`docs/models.md`). The rig's cortex hangs under the head,
    /// so the anchor has to be expressed in the head's frame; in the rest pose the head's origin
    /// sits at `(0, head_centre_m(), 0)` above the root with no rotation, which makes that
    /// conversion a subtraction. [`cortex_in_head`](Self::cortex_in_head) is the same formula
    /// with the rig's own numbers put in.
    ///
    /// **And then the depth is clamped to `head_m * 0.5`.** Measured against the drop of
    /// 2026-08-18, and it is not a matter of taste:
    ///
    /// - all 26 full bodies put their `cortex` empty **0.06–0.38 m** behind the neck axis
    ///   (`a-042-…-mittel.glb`: 0.139 m) — on the skin of a neck about 0.36 m deep, right where
    ///   their `halswulst` mesh is. The pack's own base rig and its dedicated cortex part
    ///   (`a-040`, `a-046`) say **0.450 m** instead, the middle of the amber blob. The drop does
    ///   not agree with itself, so there is no single "what the model says" to obey here.
    /// - The body a blade has to reach past is **not that neck**. It is this rig's box, and the
    ///   box does not change when a model is bound — a model is a picture plus anchors, the
    ///   collider stays `width_fraction * height_m` deep, 2.5 m for a husk. A kill sphere of
    ///   `cortex_radius_m: 0.55` centred 0.139 m behind that box's axis reaches forward to
    ///   z −0.41, and `tests/titan.rs::f030_a_bound_model_cannot_drag_the_nape_round_to_the_front`
    ///   measured what follows: the husk is then cut **from the front**, blade 0.066 m *inside*
    ///   the cortex. `F-030` is a 🟧 row with red-checked evidence and "the nape is on the back
    ///   of the neck" is the design's central rule — so it is not the rule that gives way to a
    ///   modelling detail.
    ///
    /// **Why a clamp and not a flat `head_m * 0.5`.** Only one direction can do damage. A model
    /// that puts its nape *further back* than the rig does — the lurker's body, 1.74 m once it
    /// is fitted to `large` — only makes the approach angle sharper, and dropping that would be
    /// the exact defect `F-030` exists to close, moved into another axis. So: **the model
    /// decides the height and the side, the rig decides the minimum depth.**
    ///
    /// The x is the model's untouched: no rule in this game is about left and right, and the
    /// drop's own x is 0.010–0.028 m, i.e. authoring noise nothing should be re-centred for.
    ///
    /// **Why the rest pose and not the current one:** the cortex is a child of the head and
    /// therefore follows the lean for free through `GlobalTransform`. A conversion through the
    /// *current* pose would apply that lean twice, and the kill zone would drift a little
    /// further out of the head with every degree of wind-up.
    pub fn cortex_in_head_from_model(&self, anchor: Vec3) -> Vec3 {
        let local = anchor - Vec3::new(0.0, self.head_centre_m(), 0.0);
        Vec3::new(local.x, local.y, local.z.max(self.head_m * 0.5))
    }

    /// Where a shoulder sits **inside the torso's local frame**. `right` is +X, which is the
    /// right hand of a body whose forward is −Z.
    pub fn shoulder_in_torso(&self, right: bool) -> Vec3 {
        let x = self.width_m * 0.5 + self.width_m * 0.125;
        Vec3::new(
            if right { x } else { -x },
            self.shoulder_m - self.torso_centre_m(),
            0.0,
        )
    }
}

/// The local `Transform` of one arm for a hinge angle, **hinged at the shoulder**.
///
/// The arm box is one entity, not a pivot plus a box: its rotation is the hinge and its
/// translation is the shoulder plus the rotated half-arm. Two entities per arm would be the
/// obvious way and would put two more transforms between the shoulder and the hand for
/// `F-053` to have to unwind.
pub fn arm_transform(rig: &TitanRig, right: bool, angle_deg: f32) -> Transform {
    let rotation = Quat::from_rotation_x(angle_deg.to_radians());
    let shoulder = rig.shoulder_in_torso(right);
    Transform {
        translation: shoulder + rotation * Vec3::new(0.0, -rig.arm_m * 0.5, 0.0),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Where the hand of one arm sits **inside the torso's local frame**, for a hinge angle.
///
/// `F-053` measures the telegraph on this point; it is here so that the measurement and the
/// pose cannot drift apart.
pub fn hand_in_torso(rig: &TitanRig, right: bool, angle_deg: f32) -> Vec3 {
    let rotation = Quat::from_rotation_x(angle_deg.to_radians());
    rig.shoulder_in_torso(right) + rotation * Vec3::new(0.0, -rig.arm_m, 0.0)
}

/// The local `Transform` of the torso for a lean angle, **hinged at the hip**.
pub fn torso_transform(rig: &TitanRig, lean_deg: f32) -> Transform {
    let rotation = Quat::from_rotation_x(lean_deg.to_radians());
    Transform {
        translation: rotation * Vec3::new(0.0, rig.torso_m * 0.5, 0.0),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Builds the nine entities and returns the root.
///
/// The root sits at `pos` with its **origin between the feet** (`docs/conventions.md`) and
/// faces −Z, Bevy's forward. `SpawnTitan` carries no facing, so a fresh titan looks the way
/// every un-rotated thing in this project looks, and the FSM turns it from there at
/// `turn_deg_per_s`.
#[allow(clippy::too_many_arguments)]
pub fn build_rig(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    data: &GameData,
    kind_name: &str,
    kind: &TitanKind,
    rig: &TitanRig,
    id: TitanId,
    pos: Vec3,
) -> Entity {
    let w = rig.width_m;

    // The body colour comes out of the one palette; the cortex out of the **signals** block,
    // which is separate exactly so that amber cannot leak into set dressing
    // (`docs/conventions.md` §3). A missing key is loud, not a grey stand-in.
    let body_rgb = data
        .color("stone_gray")
        .unwrap_or_else(|| panic!("maps.ron palette has no key \"stone_gray\" — the titan rig has no colour"));
    let amber = data
        .maps
        .signals
        .get("amber")
        .map(|(r, g, b)| [*r, *g, *b])
        .unwrap_or_else(|| panic!("maps.ron signals has no key \"amber\" — there is nothing to paint the cortex with"));

    let body_material = materials.add(matte(body_rgb));
    let cortex_material = materials.add(StandardMaterial {
        // The cortex is the one thing on the body that has to be found at speed, so it is the
        // one thing that emits. Everything else in this game is matte.
        emissive: LinearRgba::rgb(amber[0], amber[1], amber[2]),
        ..matte(amber)
    });

    let root = commands
        .spawn((
            // The `Name` is for the inspector and the log. **The contract is the component
            // below it**: until `TitanKindName` existed, `combat` and `debug` had to parse
            // `titan_<kind>_<id>` back out of this string, which made a debugging convenience
            // into an unwritten cross-domain interface (`src/combat/strike.rs:50-58` reported
            // it as a defect). The name stays because a titan you cannot recognise in a dump is
            // its own kind of cost — but nothing reads a fact out of it any more.
            Name::new(format!("titan_{kind_name}_{}", id.0)),
            TitanKindName::new(kind_name),
            TitanBody,
            id,
            *rig,
            TitanState::default(),
            // Idle has no length, so the pair starts open-ended. `brain::advance` owns it from
            // the next tick on — this is only what the body is born with.
            StateClock::default(),
            Health::full(kind.health),
            Velocity::default(),
            Transform::from_translation(pos),
            (
                // **Kinematic, and therefore `CustomPositionIntegration`.** Without the
                // marker the titan moves twice per tick: `SolverBodyPlugin` creates a
                // `SolverBody` for every dynamic *and kinematic* body
                // (`avian3d-0.7.0/src/dynamics/solver/solver_body/plugin.rs:25-30`, inserted
                // at `:147-150`), and `integrate_positions` filters only
                // `Without<CustomPositionIntegration>`
                // (`.../dynamics/integrator/mod.rs:503-504`). `titan::brain::walk` is the one
                // writer of this body's position; avian must not be the second.
                RigidBody::Kinematic,
                CustomPositionIntegration,
                // Endpoints, not `Collider::capsule`: that one centres the capsule on the
                // origin and would sink the titan half his height into the ground, because a
                // body's origin lies between its feet (the same trap as `player/mod.rs`).
                Collider::capsule_endpoints(
                    w * 0.5,
                    Vec3::new(0.0, w * 0.5, 0.0),
                    Vec3::new(0.0, rig.height_m - w * 0.5, 0.0),
                ),
                // Member of the body layer, colliding with **everything**. Not with
                // `LAYER_WORLD | LAYER_PLAYER`: today nothing in the repo wears a
                // `CollisionLayers` component (`src/shared/layers.rs:28-32`), so a mask of our
                // own four layers would make the titan collide with nothing at all and walk
                // through the city as a ghost. The membership is what `combat` filters on and
                // it is right from today.
                CollisionLayers::new(LAYER_TITAN_BODY, LayerMask::ALL),
            ),
            // **`F-029`: this is the whole of "a titan holds a rope".**
            //
            // `vector::aim` has hit this capsule from the day it existed — it casts with
            // avian's default filter on purpose, so that a rope can never travel *through* an
            // untagged wall. What it could not do was hand the hit on: `hook::anchor_target`
            // asks for a [`BodyId`], no titan entity carried a [`Body`], and the arm therefore
            // stayed `Idle` without a word. That was **`B-007`**, both halves of it — because a
            // solid body that holds nothing also *hides* the good wall behind it.
            //
            // `world::index::maintain_index` does the rest and needed no line: it hands out the
            // [`BodyId`], and its third block (`Changed<GlobalTransform>`) re-inserts the hull
            // of a body that moved. `vector::hook` stores the anchor as
            // `local_m = point − entry.center_m` and reads it back as `entry.center_m + local_m`
            // every tick, so the rope rides a walking titan without anybody writing a rope
            // system for it (`tests/titan.rs::f029_a_rope_bites_a_walking_titan_and_rides_him`).
            //
            // **Where the hull is measured from.** `world::index::entry_from` puts the entry's
            // centre on the entity's world position, and a titan's origin lies **between his
            // feet** (`docs/conventions.md`) — so the vertical half below is measured about the
            // feet and not about the waist. That is deliberate and it costs nothing: the index
            // grids **XZ only** (`shared::spatial`, decision 1), the anchor arithmetic is a pure
            // difference of two positions, and `cast_ray`/`aabb_overlaps` are stubs with no
            // callers. The XZ half is the capsule's own radius, which is what decides how many
            // cells a walking titan touches.
            //
            // ⚠️ **The whole silhouette holds, the nape included — and that does not hand the
            // player a kill.** `F-030`'s rule is about where a *cut* lands, not where a rope may
            // bite, and it is guarded by a **speed**: `blades::cut` drops every pass under
            // `gear.ron: blades.min_speed_m_s` (8.0 m/s) before it ever looks at the zone
            // (`src/blades/cut.rs:248`). A player parked on the nape closes at ~0 m/s and cuts
            // nothing; he has bought position, which is the genre's core move, not a free kill.
            // The alternative — an un-hookable head — cannot be built here anyway: the rig has
            // exactly one collider, the root capsule, and every limb box sits *inside* it
            // (`docs/FINDINGS.md` FIND-109), so a ray never reaches a limb to be refused by it.
            Body {
                half_size_m: Vec3::new(w * 0.5, rig.height_m * 0.5, w * 0.5),
                mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
            },
        ))
        .id();

    // ---- pelvis: the hip node ---------------------------------------------------------
    //
    // Its box straddles the leg/torso seam and is half the body width tall. It overlaps both
    // neighbours; that is what a joint looks like in a box rig, and it invents no new length.
    let pelvis_half = Vec3::new(w * 0.5, w * 0.25, w * 0.5);
    let pelvis = commands
        .spawn((
            Name::new("pelvis"),
            TitanPart::Pelvis,
            PartExtent(pelvis_half),
            Transform::from_xyz(0.0, rig.leg_m, 0.0),
            Mesh3d(meshes.add(Cuboid::new(w, w * 0.5, w))),
            MeshMaterial3d(body_material.clone()),
        ))
        .id();
    commands.entity(root).add_child(pelvis);

    // ---- legs: children of the hip, hanging down to the ground -------------------------
    let leg_half = Vec3::new(w * 0.25, rig.leg_m * 0.5, w * 0.25);
    let leg_mesh = meshes.add(Cuboid::new(w * 0.5, rig.leg_m, w * 0.5));
    for (part, zone, sign) in [
        (TitanPart::LegLeft, HitZone::LegLeft, -1.0f32),
        (TitanPart::LegRight, HitZone::LegRight, 1.0),
    ] {
        let leg = commands
            .spawn((
                Name::new(if sign < 0.0 { "leg_left" } else { "leg_right" }),
                part,
                PartExtent(leg_half),
                Transform::from_xyz(sign * w * 0.25, -rig.leg_m * 0.5, 0.0),
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
                hit_zone(leg_half, zone),
            ))
            .id();
        commands.entity(pelvis).add_child(leg);
    }

    // ---- torso: leans about the hip ----------------------------------------------------
    let torso_half = Vec3::new(w * 0.5, rig.torso_m * 0.5, w * 0.5);
    let torso = commands
        .spawn((
            Name::new("torso"),
            TitanPart::Torso,
            PartExtent(torso_half),
            torso_transform(rig, 0.0),
            Mesh3d(meshes.add(Cuboid::new(w, rig.torso_m, w))),
            MeshMaterial3d(body_material.clone()),
        ))
        .id();
    commands.entity(pelvis).add_child(torso);

    // ---- arms: hinged at the shoulder, inside the torso's frame ------------------------
    let arm_half = Vec3::new(w * 0.125, rig.arm_m * 0.5, w * 0.125);
    let arm_mesh = meshes.add(Cuboid::new(w * 0.25, rig.arm_m, w * 0.25));
    for (part, zone, right) in [
        (TitanPart::ArmLeft, HitZone::ArmLeft, false),
        (TitanPart::ArmRight, HitZone::ArmRight, true),
    ] {
        let arm = commands
            .spawn((
                Name::new(if right { "arm_right" } else { "arm_left" }),
                part,
                PartExtent(arm_half),
                arm_transform(rig, right, 0.0),
                Mesh3d(arm_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
                hit_zone(arm_half, zone),
            ))
            .id();
        commands.entity(torso).add_child(arm);
    }

    // ---- head, and the cortex under it -------------------------------------------------
    let head_half = Vec3::splat(rig.head_m * 0.5);
    let head = commands
        .spawn((
            Name::new("head"),
            TitanPart::Head,
            PartExtent(head_half),
            Transform::from_xyz(0.0, rig.torso_m * 0.5 + rig.head_m * 0.5, 0.0),
            Mesh3d(meshes.add(Cuboid::new(rig.head_m, rig.head_m, rig.head_m))),
            MeshMaterial3d(body_material),
        ))
        .id();
    commands.entity(torso).add_child(head);

    let cortex = commands
        .spawn((
            Name::new("cortex"),
            TitanPart::Cortex,
            PartExtent(Vec3::splat(rig.cortex_radius_m)),
            Transform::from_translation(rig.cortex_in_head()),
            Mesh3d(meshes.add(Sphere::new(rig.cortex_radius_m))),
            MeshMaterial3d(cortex_material),
            // **Physically intangible, still hittable.** `SpatialQuery`'s collider query
            // carries no `Without<Sensor>` (`avian3d-0.7.0/src/spatial_query/system_param.rs:59-64`),
            // unlike `MoveAndSlide`'s (`.../character_controller/move_and_slide.rs:82`) — so a
            // blade cast finds it while nothing ever bumps into it.
            Collider::sphere(rig.cortex_radius_m),
            Sensor,
            CollisionLayers::new(LAYER_TITAN_CORTEX, LayerMask::NONE),
        ))
        .id();
    commands.entity(head).add_child(cortex);

    root
}

/// **`F-029`: the ropes let go of a corpse, in the tick he dies.**
///
/// `brain::receive_hits` takes the body collider off the root the moment the cortex is cut —
/// *"a corpse is never a wall"* — but a collider is not what a rope hangs on. The rope hangs on
/// the [`Body`]/[`BodyId`] pair, and without this line it would stay taut through the whole
/// `death_s` dissolve and only come free when the entity is despawned. A second of hanging on a
/// dead titan is the acceptance sentence of `F-029` failing: *"löst sich beim Tod des Titanen
/// mit Feedback."*
///
/// **No new channel** (the commission's rule and the right one): taking the component off makes
/// `world::index`'s `on_body_removed` observer fire, the id lands in the index's mailbox, and
/// `maintain_index` writes [`BodyGone`](crate::shared::BodyGone) in the next tick's
/// `SimulationSystems::Spatial` — which is the set `vector::hook` already reads it in, with
/// `ReleaseReason::BodyGone` already in the enum. One tick, deterministic: `Death` is decided in
/// `Drive`, `Spatial` is the first set of the *next* tick.
///
/// `vector::hook`'s second guard (`index.body(id).is_none()`) fires on the same tick from the
/// same removal, so the message and the lookup can never disagree about a corpse.
///
/// **Rule 6:** `Changed<TitanState>` is empty on every tick in which no titan changed state —
/// which is all but a handful per titan per sortie. It never walks the living.
///
/// `With<Body>` and not a `Death`-only query, so a titan already released is not visited again
/// for the rest of his dissolve.
pub fn the_ropes_let_go_of_a_corpse(
    mut commands: Commands,
    dying: Query<(Entity, &TitanState), (With<TitanBody>, With<Body>, Changed<TitanState>)>,
) {
    for (entity, state) in &dying {
        if *state != TitanState::Death {
            continue;
        }
        // One line per death, not per tick — and it is the line that explains a rope going
        // slack under a player who was mid-swing.
        info!("the titan is dead: his Body goes, and every rope on him releases (F-029)");
        commands.entity(entity).remove::<Body>();
    }
}

/// **Where a swapped model dies.**
///
/// Until 2026-08-12 the cortex was computed from `scale.ron` and nothing else — so a model the
/// user dropped in *rendered* in the right place and *died* in the computed one. This is the
/// one line that closes it: when the `.glb` brings a `cortex` empty,
/// [`ModelAnchors`](crate::shared::ModelAnchors) carries it and the sensor moves there. When it
/// does not, nothing happens at all and the computed position stands — the fallback is the
/// **absence** of a write, which is why a missing empty can never become a kill zone at the
/// origin.
///
/// `scale.ron` stays the yardstick, not the loser: `render::model` compares the anchor against
/// it and shouts past `art.ron: cortex_tolerance_m`. The model decides *where*, the file
/// decides *whether that is plausible*.
///
/// **`Changed<ModelAnchors>` and nothing per tick** (rule 6): the component is written once,
/// when the scene instance is ready.
pub fn cortex_from_the_model(
    anchors: Query<(Entity, &TitanRig, &ModelAnchors), Changed<ModelAnchors>>,
    children: Query<&Children>,
    parts: Query<&TitanPart>,
    mut transforms: Query<&mut Transform>,
) {
    for (root, rig, model) in &anchors {
        let Some(anchor) = model.get(CORTEX_ANCHOR) else {
            continue;
        };
        let Some(cortex) = children
            .iter_descendants(root)
            .find(|e| parts.get(*e) == Ok(&TitanPart::Cortex))
        else {
            continue;
        };
        let Ok(mut transform) = transforms.get_mut(cortex) else {
            continue;
        };
        let local = rig.cortex_in_head_from_model(anchor);
        if transform.translation == local {
            continue;
        }
        if local.z > anchor.z + 1e-4 {
            // One line per titan, not per tick, and it is the line that explains a kill zone
            // sitting somewhere other than where the modeller put it.
            info!(
                "the model's {CORTEX_ANCHOR:?} empty moves the kill zone from {:.2} m to {:.2} m \
                 above the feet, and its depth of {:.2} m was held back to the rig's {:.2} m — \
                 the Cortex stays behind the neck (F-030)",
                rig.cortex_height_m, anchor.y, anchor.z, local.z
            );
        } else {
            info!(
                "the model's {CORTEX_ANCHOR:?} empty moves the kill zone from {:.2} m to {:.2} m \
                 above the feet, {:.2} m behind the neck as the model asks (F-030)",
                rig.cortex_height_m, anchor.y, local.z
            );
        }
        transform.translation = local;
    }
}

/// **One limb box, as a hit zone** — `F-032`, and the end of "a titan has exactly one collider".
///
/// It is one component and **no collider at all**: [`HitZoneOf`] publishes the box's half
/// extent as data, `blades::cut::limb_zone` tests the swept blade against it, and the physics
/// world never learns that the box exists. Why it is not a `Sensor` on a layer of its own —
/// which is what this was for the first two hours of its life — is written on [`HitZoneOf`]:
/// `vector::aim` casts the hook ray **unfiltered**, an arm sticks out of the root capsule, and
/// a collider there takes the rope off the titan. The measurement is in `docs/FINDINGS.md`.
///
/// ## ⚠️ Every limb box lies INSIDE the root capsule, and that is what the tiering is for
///
/// FIND-116 named this as the thing that would make limb hit zones change nothing: the capsule
/// has radius `w/2`, the legs span `0 .. w/2` of it and the arms `w/2 .. 3w/4`, so a cast that
/// asked one question for both would answer with the silhouette nearly every time. **The
/// overlap is therefore resolved by precedence, not by distance:** `blades::cut` asks the
/// cortex layer, then the body layer, and a body hit is then **refined** against these boxes.
/// A blade inside the capsule and inside an arm box is an arm hit; a blade inside the capsule
/// and inside nothing else is the honest catch-all `Torso`.
///
/// Measured 2026-08-19 on the real husk: the chest pass of
/// `tests/combat.rs::f032_a_body_cut_staggers_the_titan_and_never_kills_him` still reports
/// `[Torso]`, the same pass moved 1.75 m to his right reports `[Torso, ArmRight]`, and at knee
/// height `[Torso, LegRight]`.
///
/// ## What deliberately does NOT get one
///
/// **The head, the torso and the pelvis.** Not an oversight:
///
/// * the head box spans `cortex_height_m .. height_m` and a nape pass crosses it, so a head
///   zone would rename the graze of every cut in the game from `Torso` to `Head` — and
///   `F-030`, `q030`, `q031` and `F-034` are 🟧 rows whose evidence is those passes. A zone
///   nothing reads yet is not worth moving measured evidence for.
/// * [`HitZone::Eye`] is `F-032`'s third half (*"Augentreffer erzeugt 3 s Orientierungslosigkeit"*)
///   and it needs an **eye anchor on the model**, not a box on the rig: an eye is 20 cm on a
///   10 m body and this rig has no feature that small. The pack's `a-064-zone-*` empties name
///   `cortex`, `cortex-gross`, `gelenk`, `huefte`, `riss` and `schulter` — no eye either.
/// * the torso and the pelvis **are** the body. `Torso` is what the root capsule already
///   answers, and a second box for the same zone is two things to keep in agreement for no gain.
///
/// **Rule 6:** one component per limb, written once at spawn, never touched again. It follows
/// the pose through `GlobalTransform` for free, exactly like the cortex.
fn hit_zone(half_extent_m: Vec3, zone: HitZone) -> HitZoneOf {
    HitZoneOf { zone, half_extent_m }
}

/// A matte material. A missing `metallic` means 1.0 in glTF, and a diffuse material without
/// the value looks like chrome (`docs/models.md`, glTF trap 2) — the same holds for a
/// hand-built one.
fn matte(rgb: [f32; 3]) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::linear_rgb(rgb[0], rgb[1], rgb[2]),
        metallic: 0.0,
        perceptual_roughness: 0.95,
        ..default()
    }
}
