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
use crate::shared::{Health, TitanId, TitanState, Velocity, LAYER_TITAN_BODY, LAYER_TITAN_CORTEX};

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
            Name::new(format!("titan_{kind_name}_{}", id.0)),
            TitanBody,
            id,
            *rig,
            TitanState::default(),
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
    for (part, sign) in [(TitanPart::LegLeft, -1.0f32), (TitanPart::LegRight, 1.0)] {
        let leg = commands
            .spawn((
                Name::new(if sign < 0.0 { "leg_left" } else { "leg_right" }),
                part,
                PartExtent(leg_half),
                Transform::from_xyz(sign * w * 0.25, -rig.leg_m * 0.5, 0.0),
                Mesh3d(leg_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
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
    for (part, right) in [(TitanPart::ArmLeft, false), (TitanPart::ArmRight, true)] {
        let arm = commands
            .spawn((
                Name::new(if right { "arm_right" } else { "arm_left" }),
                part,
                PartExtent(arm_half),
                arm_transform(rig, right, 0.0),
                Mesh3d(arm_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
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
