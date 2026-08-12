//! **The landing preview: one marker per arm, `Q` left and `E` right, standing on the point in
//! the world that arm is aimed at.**
//!
//! The user asked for the element on 2026-08-10 (*"und es muss auch visuell immer 2 punkte
//! angezeigt werden so der e und q haken hingehen würden!"*) and then rejected the first
//! answer on 2026-08-12, correctly:
//!
//! > *"es soll previewd werden wo der aktuelle haken landen würde! also sollte richtig angezeigt
//! > werden. nicht nur am fadenkreuz. weil das stimmt auch nicht."*
//! > *"zudem sollen diese weiter auseinander sein. also weiter rechts und links!"*
//!
//! He was describing `FINDINGS.md` FIND-047 from the outside: the two markers were pinned at
//! `top 65 %` / `left|right 52 %` and were photographed **at the same pixels across four runs
//! with four different aims**. They were state badges wearing a location's clothes.
//!
//! # What moved, and what still cannot
//!
//! [`place_arm_aim`] projects each arm's own world target through the real camera
//! (`Camera::world_to_viewport`) and puts the marker there. For the three states in which an arm
//! has a world point of its own that is **not** on the camera axis — `Anchored`, and `Flying` /
//! `Retracting` while the tip is out — the marker now travels across the screen with that point,
//! and the two arms genuinely stand on two different places.
//!
//! For an **idle** arm it cannot, and that is measured rather than argued.
//! `tests/hud.rs::f171_a_free_aim_point_projects_onto_the_crosshair` casts the same ray
//! `vector::aim` casts and projects the result: at three look angles and two distances it lands
//! **0.000 px from the centre of the screen**, every time. The reason is a chain of equalities the
//! repo already relies on — `vector::aim` starts at `translation + Y·eye_height_m`,
//! `render::attach_camera` hangs the camera on the player at exactly
//! `Transform::from_xyz(0, eye_height_m, 0)`, and `tests/render.rs` nails
//! `Transform::forward() == Intent::look_dir()`. The aim ray **is** the view ray, so every point on
//! it is the crosshair pixel. This is the same fact `render::rope` ran into when it could not draw
//! a rope from the hand.
//!
//! **So a free arm's honest preview is the crosshair, and two idle arms cannot stand on two
//! points until the two arms are aimed at two points.** That is `docs/backlog/gameplay.ron`
//! `F-023` (*Kandidatensuche mit Hemisphaeren-Aufteilung*: the candidate set is split relative to
//! the camera forward axis, `Q` serves the left set and `E` the right), it hangs on `F-021`
//! (discrete anchor points, ⬜), and its spread is a **tuning number** — which under `CLAUDE.md`
//! rule 2 has to live in `assets/data/game.ron` under `vector:`. The gap is written up in
//! `docs/FINDINGS.md` FIND-070 with exactly what is missing.
//!
//! What the idle pair *can* honestly say is **which side each arm serves**, and it now says it
//! loudly: with nothing to project onto, the two markers part around `F-170`'s keep-out box
//! instead of huddling ~55 px under the crosshair. That is the second sentence — *"weiter rechts
//! und links"* — and the number is not invented: it is the one rectangle this HUD already may not
//! cover.
//!
//! # Why the middle of the screen survives a world-tracked marker
//!
//! `F-170` keeps the central 20 % × 20 % free ([`KEEP_OUT_LOW_PCT`]..[`KEEP_OUT_HIGH_PCT`]) and
//! `tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen` is a proven 🟧 claim. A marker
//! that follows a world point **will** eventually be aimed straight at, so the guard cannot be
//! left to luck. [`layout_for`] therefore pushes the whole cluster — glyph, tether and letter —
//! horizontally out of the box, **towards the side the point is already on**, so the marker never
//! claims the wrong half of the screen. Ties (a point dead on the axis) go to the arm's own side.
//! The push is applied **last**, after the screen clamp: on a viewport small enough for the two to
//! disagree, the proven claim wins and a few pixels of the marker leave the screen instead.
//!
//! # Four shapes, and the colour carries nothing
//!
//! | [`ArmAimState`] | what it means | glyph | tether | nodes |
//! |---|---|---|---|---|
//! | [`Free`](ArmAimState::Free) | idle, and the ray finds nothing hookable | flat dash | — | 1 |
//! | [`Ready`](ArmAimState::Ready) | idle, and this arm would catch | ring | — | 1 |
//! | [`Busy`](ArmAimState::Busy) | the tip is out: flying or coming home | wide ring | — | 1 |
//! | [`Anchored`](ArmAimState::Anchored) | this arm is holding | filled disc | yes | 2 |
//!
//! The four bounding boxes are four different `(node count, width, height)` tuples, so
//! `tests/hud.rs::f171_the_two_arm_markers_differ_in_shape_not_only_in_colour` can force every
//! `BackgroundColor` **and** every `BorderColor` to one value and still tell them apart. That is
//! `F-171`'s rule and `F-026`'s acceptance, and it is the only way either is falsifiable.
//!
//! # One field, one writer — inside a single component
//!
//! `Node` is now written by two systems, and they are split **by field**, not by entity:
//! [`shape_arm_aim`] owns `width`, `height`, `border`, `border_radius` and `display`;
//! [`place_arm_aim`] owns `left`, `top`, `right`, `bottom` and `margin`. Neither ever reads back
//! what the other wrote. Written as two whole-`Node` assignments they would overwrite each other
//! every frame and change detection would fire 60 times a second on a standing player — the exact
//! failure `docs/lessons/performance.md` rule 1 is about.
//!
//! # What it costs per frame
//!
//! **No ray and no spatial query.** [`sense_arm_aim`] reads [`Hook`] and [`AimPoint`] off the
//! local player — both already written this tick by `vector` — and [`place_arm_aim`] adds two
//! matrix multiplications, one per arm. Every write is guarded by a comparison, so a standing
//! player with a still camera produces zero writes (`CLAUDE.md` rule 6).

use bevy::prelude::*;
use bevy::text::FontSize;

use crate::data::GameData;
use crate::hud::crosshair::NEUTRAL;
use crate::hud::{signal, HudElement, KEEP_OUT_HIGH_PCT, KEEP_OUT_LOW_PCT};
use crate::shared::{AimPoint, Hook, HookState, LocalPlayer, Side};

/// Which arm this node belongs to, and which of its two nodes it is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmMarker {
    pub side: Side,
    pub part: MarkerPart,
}

/// The glyph is the marker; the tether is the short stem that only the anchored state shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerPart {
    Glyph,
    Tether,
}

/// What one arm would do if you pressed its key right now.
///
/// Lives on the marker nodes and not on the player and not in a `Resource`: it is view state of
/// the local client, it has exactly one writer, and player state in a resource is the first step
/// this project forbids (`docs/multiplayer.md` rule 2). Same choice as
/// [`CrosshairState`](crate::hud::crosshair::CrosshairState), for the same reason.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArmAimState {
    /// The arm is idle and the aim ray ends somewhere no hook can take.
    #[default]
    Free,
    /// The arm is idle and would catch. Press the key and it flies.
    Ready,
    /// The tip is not in the hand — flying out or being reeled back. **The key does nothing**
    /// until it is home again (`vector::hook`, decision 1: a shot only leaves from `Idle`).
    ///
    /// Flying and retracting are one state on purpose: what the player has to know here is that
    /// this arm is not available, and the rope in the world says which of the two it is.
    Busy,
    /// The arm is holding.
    Anchored,
}

/// The pure rule: one arm's hook state plus "is the shared aim point hookable" → one shape.
///
/// A free function, so the rule is testable without a physics world, a camera and a look
/// direction having to be arranged first.
pub fn state_for(hook: &HookState, anchorable: bool) -> ArmAimState {
    match hook {
        HookState::Idle => {
            if anchorable {
                ArmAimState::Ready
            } else {
                ArmAimState::Free
            }
        }
        HookState::Flying { .. } | HookState::Retracting => ArmAimState::Busy,
        HookState::Anchored { .. } => ArmAimState::Anchored,
    }
}

/// **The world point this arm's marker stands on**, or `None` when it has none.
///
/// The three states with a point of their own take [`HookArm::tip_m`](crate::shared::HookArm),
/// which `vector::hook` walks along on every tick and `render::rope` already draws to — so the
/// marker and the rope can never disagree about where the arm is holding. An idle arm has no
/// point of its own and falls back to the shared [`AimPoint`], which is measured to be the
/// crosshair (module header); `None` there means the ray found nothing at all and the marker goes
/// to its side slot.
pub fn target_of(hook: &Hook, aim: &AimPoint, side: Side) -> Option<Vec3> {
    let arm = hook.arm(side);
    match arm.state {
        HookState::Idle => aim.point_m,
        HookState::Flying { .. } | HookState::Retracting | HookState::Anchored { .. } => {
            Some(arm.tip_m)
        }
    }
}

/// The geometry of one state, in logical pixels.
///
/// **Shape constants, not balancing values** — the same argument
/// [`CrosshairShape`](crate::hud::crosshair::CrosshairShape) makes for the crosshair: `CLAUDE.md`
/// rule 2 names "a titan kind, a blade tier, a gas cost", and the fact that a marker is a ring is
/// none of the three. What would belong in RON is a UI scale factor, and there is none yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArmShape {
    pub glyph_w_px: f32,
    pub glyph_h_px: f32,
    /// Border width. `0` means the glyph is filled, anything else means it is an outline — and
    /// outline against filled is a shape difference, not a colour one.
    pub border_px: f32,
    /// Whether the corners are rounded all the way (a ring or a disc) or not at all (a dash).
    pub round: bool,
    /// Length of the stem under the glyph, or `None` when this state has no second node.
    pub tether_px: Option<f32>,
}

/// The one table. Change a number here and the picture and the test move together.
pub const fn shape_of(state: ArmAimState) -> ArmShape {
    match state {
        ArmAimState::Free => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 4.0,
            border_px: 0.0,
            round: false,
            tether_px: None,
        },
        ArmAimState::Ready => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 20.0,
            border_px: 3.0,
            round: true,
            tether_px: None,
        },
        ArmAimState::Busy => ArmShape {
            glyph_w_px: 28.0,
            glyph_h_px: 28.0,
            border_px: 3.0,
            round: true,
            tether_px: None,
        },
        ArmAimState::Anchored => ArmShape {
            glyph_w_px: 20.0,
            glyph_h_px: 20.0,
            border_px: 0.0,
            round: true,
            tether_px: Some(16.0),
        },
    }
}

/// How many nodes a state shows — the first element of the tuple the test compares.
pub const fn node_count(state: ArmAimState) -> usize {
    match shape_of(state).tether_px {
        Some(_) => 2,
        None => 1,
    }
}

/// The key that fires this arm.
///
/// **A second spelling of `src/net/local.rs:84-85`**, where `Buttons::HOOK_LEFT` is bound to
/// `KeyCode::KeyQ` and `HOOK_RIGHT` to `KeyCode::KeyE`. `hud` may not reach into `net` (the
/// allow list in `docs/architecture.md` has no such line), so the letter is written twice — and
/// `tests/hud.rs::f171_the_marker_letters_are_the_keys_that_fire_the_arms` reads that source file
/// back and goes red the day somebody rebinds a hook without coming here. A label that names the
/// wrong key is worse than no label.
pub const fn key_label(side: Side) -> &'static str {
    match side {
        Side::Left => "Q",
        Side::Right => "E",
    }
}

/// Gap between the glyph and its tether.
const TETHER_GAP_PX: f32 = 4.0;
/// Width of the tether stem.
const TETHER_W_PX: f32 = 4.0;
/// The widest glyph any state draws. Used for the one-frame placeholder at spawn, where there is
/// no viewport to measure against yet.
pub const GLYPH_MAX_PX: f32 = 28.0;
/// Gap between the glyph and its letter.
const LABEL_GAP_PX: f32 = 6.0;
/// The letter. Small: it names the key, it is not the readout.
pub const LABEL_PX: f32 = 15.0;
/// How wide one letter is taken to be when the cluster is kept out of the middle.
///
/// A **deliberate over-estimate** and not a measurement: the real width comes out of text layout
/// one stage later, and the keep-out arithmetic has to run before it. `Q` at 15 px measures about
/// 10 px, so 12 leaves room and errs outward — outward is the safe direction, because the box is
/// inward. `tests/hud.rs` measures the real rects afterwards and falls over if this is ever too
/// small.
const LABEL_W_PX: f32 = 12.0;

/// Where one arm's three nodes stand this frame, in logical UI pixels (top-left corners).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmLayout {
    pub glyph: Vec2,
    pub tether: Vec2,
    pub label: Vec2,
    /// Whether the letter sits to the **right** of the glyph. It always sits on the side away
    /// from the middle of the screen, so it can never be the thing that creeps into the box.
    pub label_right: bool,
}

/// **The whole placement rule, as one pure function** — no camera, no `World`, no `Node`.
///
/// `at` is the arm's world target already projected into logical viewport pixels, or `None` when
/// there is nothing to project (no camera hit, nothing in range, or the point is behind the
/// player and `Camera::world_to_viewport` refused it — it returns `Err` for anything outside the
/// frustum, `bevy_camera-0.19.0/src/camera.rs:551-557`).
///
/// Three steps, in this order and the order is the design:
/// 1. put the glyph's centre on the projected point, or in the arm's side slot if there is none;
/// 2. clamp the cluster into the viewport, so a marker never leaves the screen entirely;
/// 3. push the cluster out of `F-170`'s keep-out box, towards the side the point is already on.
///
/// Step 3 is last on purpose: it is the proven 🟧 claim
/// (`tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen`) and step 2 is a courtesy, so on
/// a viewport small enough for the two to fight, the claim wins.
pub fn layout_for(side: Side, shape: ArmShape, at: Option<Vec2>, viewport: Vec2) -> ArmLayout {
    let vw = viewport.x.max(1.0);
    let vh = viewport.y.max(1.0);
    let box_min_x = vw * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_x = vw * KEEP_OUT_HIGH_PCT / 100.0;
    let box_min_y = vh * KEEP_OUT_LOW_PCT / 100.0;
    let box_max_y = vh * KEEP_OUT_HIGH_PCT / 100.0;

    let full_h = shape.glyph_h_px + shape.tether_px.map_or(0.0, |t| TETHER_GAP_PX + t);
    let label_out = LABEL_GAP_PX + LABEL_W_PX;

    // The slot an arm falls back to: hard against its own side of the keep-out box, at eye
    // level. That is the "weiter rechts und links" the player asked for, and the number is the
    // box rather than a taste.
    let slot_x = |right: bool| {
        if right {
            box_max_x
        } else {
            box_min_x - shape.glyph_w_px
        }
    };

    let (mut x, mut y, label_right) = match at {
        Some(p) if p.is_finite() => {
            // Which half is this point in? The letter and the keep-out push both follow that,
            // so a marker never points at the wrong side of the screen. Dead on the axis is the
            // idle case, and there the arm's own side decides — `Q` left, `E` right.
            let lean_right = if (p.x - vw * 0.5).abs() < 0.5 {
                matches!(side, Side::Right)
            } else {
                p.x > vw * 0.5
            };
            (p.x - shape.glyph_w_px * 0.5, p.y - shape.glyph_h_px * 0.5, lean_right)
        }
        _ => {
            let right = matches!(side, Side::Right);
            (slot_x(right), vh * 0.5 - shape.glyph_h_px * 0.5, right)
        }
    };

    // 2. into the viewport.
    let (lo_extra, hi_extra) = if label_right { (0.0, label_out) } else { (label_out, 0.0) };
    let min_x = lo_extra;
    let max_x = (vw - shape.glyph_w_px - hi_extra).max(min_x);
    x = x.clamp(min_x, max_x);
    y = y.clamp(0.0, (vh - full_h).max(0.0));

    // 3. out of the middle. The cluster is the glyph plus its letter; the tether hangs inside
    // the glyph's own width, so it adds nothing horizontally and its length is already in
    // `full_h`.
    let hits_box = x - lo_extra < box_max_x
        && x + shape.glyph_w_px + hi_extra > box_min_x
        && y < box_max_y
        && y + full_h > box_min_y;
    if hits_box {
        x = slot_x(label_right);
    }

    ArmLayout {
        glyph: Vec2::new(x, y),
        tether: Vec2::new(
            x + (shape.glyph_w_px - TETHER_W_PX) * 0.5,
            y + shape.glyph_h_px + TETHER_GAP_PX,
        ),
        label: Vec2::new(
            if label_right {
                x + shape.glyph_w_px + LABEL_GAP_PX
            } else {
                x - LABEL_GAP_PX - LABEL_W_PX
            },
            y + (shape.glyph_h_px - LABEL_PX) * 0.5,
        ),
        label_right,
    }
}

/// Six nodes: a glyph, a tether and a letter per arm.
///
/// The positions written here are a **one-frame placeholder** — there is no viewport at
/// `Startup`, so they are percentages that stand clear of the keep-out box, and
/// [`place_arm_aim`] overwrites them in pixels as soon as a camera reports a size.
pub fn spawn_arm_aim(mut commands: Commands, data: Res<GameData>) {
    for side in Side::ALL {
        let shape = shape_of(ArmAimState::default());
        let right = matches!(side, Side::Right);
        // Outside the box on the arm's own side: 34 % / 64 % of the width against a box that
        // runs 40 %..60 %.
        let x = if right {
            Val::Percent(KEEP_OUT_HIGH_PCT + 4.0)
        } else {
            Val::Percent(KEEP_OUT_LOW_PCT - 6.0)
        };
        commands.spawn((
            Name::new(format!("hud_arm_marker_{side:?}")),
            ArmMarker { side, part: MarkerPart::Glyph },
            ArmAimState::default(),
            HudElement,
            BackgroundColor(NEUTRAL),
            BorderColor::all(Color::NONE),
            placeholder(shape_node(MarkerPart::Glyph, shape), x),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_tether_{side:?}")),
            ArmMarker { side, part: MarkerPart::Tether },
            ArmAimState::default(),
            HudElement,
            BackgroundColor(NEUTRAL),
            BorderColor::all(Color::NONE),
            placeholder(shape_node(MarkerPart::Tether, shape), x),
        ));
        commands.spawn((
            Name::new(format!("hud_arm_label_{side:?}")),
            ArmMarkerLabel(side),
            HudElement,
            Text::new(key_label(side)),
            TextFont { font_size: FontSize::Px(LABEL_PX), ..default() },
            TextColor(signal(&data, "cyan")),
            placeholder(
                Node { position_type: PositionType::Absolute, ..default() },
                if right {
                    Val::Percent(KEEP_OUT_HIGH_PCT + 4.0 + GLYPH_MAX_PX / 10.0)
                } else {
                    Val::Percent(KEEP_OUT_LOW_PCT - 9.0)
                },
            ),
        ));
    }
}

fn placeholder(mut node: Node, x: Val) -> Node {
    node.left = x;
    node.top = Val::Percent(50.0);
    node
}

/// The `Q` / `E` letter.
///
/// It names a key binding, so its **text** is written once at startup and never again; only its
/// position follows the glyph.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmMarkerLabel(pub Side);

/// The size half of a node: everything [`shape_arm_aim`] owns, and nothing else.
fn shape_node(part: MarkerPart, shape: ArmShape) -> Node {
    let mut node = Node { position_type: PositionType::Absolute, ..default() };
    match part {
        MarkerPart::Glyph => {
            node.width = Val::Px(shape.glyph_w_px);
            node.height = Val::Px(shape.glyph_h_px);
            node.border = UiRect::all(Val::Px(shape.border_px));
            // `border_radius` is a field of `Node` in bevy 0.19 (`ui_node.rs:738`), not a
            // component of its own — so the ring and the dash are one write, not two.
            node.border_radius =
                if shape.round { BorderRadius::MAX } else { BorderRadius::ZERO };
        }
        MarkerPart::Tether => match shape.tether_px {
            Some(length_px) => {
                node.width = Val::Px(TETHER_W_PX);
                node.height = Val::Px(length_px);
            }
            // `Display::None` and not a despawn — an entity that comes and goes changes the
            // archetype every time an arm catches, and would make the node count depend on when
            // you look. The crosshair's corner marks are switched off the same way.
            None => node.display = Display::None,
        },
    }
    node
}

/// Reads the local player's two arms, writes [`ArmAimState`] and nothing else.
///
/// `anchorable` is the **shared** aim answer — one ray, two arms, which is what
/// `vector::hook::update_hooks` does with it. The day `F-023`'s hemispheres land, this is the
/// one line that changes: the two arms then read two different candidates, and every shape,
/// colour and test below stays as it is.
pub fn sense_arm_aim(
    players: Query<(&Hook, &AimPoint), With<LocalPlayer>>,
    mut markers: Query<(&ArmMarker, &mut ArmAimState)>,
) {
    let Some((hook, aim)) = players.iter().next() else {
        return;
    };
    for (marker, mut state) in &mut markers {
        state.set_if_neq(state_for(&hook.arm(marker.side).state, aim.anchorable));
    }
}

/// Writes the **size** fields of `Node` and no others — see the module header on why the split
/// is by field and not by entity.
pub fn shape_arm_aim(mut markers: Query<(&ArmMarker, &ArmAimState, &mut Node)>) {
    for (marker, state, mut node) in &mut markers {
        let wanted = shape_node(marker.part, shape_of(*state));
        if node.width != wanted.width
            || node.height != wanted.height
            || node.border != wanted.border
            || node.border_radius != wanted.border_radius
            || node.display != wanted.display
        {
            node.width = wanted.width;
            node.height = wanted.height;
            node.border = wanted.border;
            node.border_radius = wanted.border_radius;
            node.display = wanted.display;
        }
    }
}

/// Writes the **position** fields of `Node` and no others.
fn put(node: &mut Node, at: Vec2) {
    let left = Val::Px(at.x);
    let top = Val::Px(at.y);
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    // The placeholder at spawn only ever sets `left`/`top`, but a `right` left over from an
    // older layout would fight this one silently, so both are pinned to `Auto` here.
    if node.right != Val::Auto {
        node.right = Val::Auto;
    }
    if node.bottom != Val::Auto {
        node.bottom = Val::Auto;
    }
}

/// **The preview itself:** projects each arm's world target and puts its three nodes there.
///
/// Runs in `PostUpdate`, after `TransformSystems::Propagate` and `CameraUpdateSystems` and
/// before `UiSystems::Layout`. Not in `Update`: the camera's `GlobalTransform` and its viewport
/// size are both computed in `PostUpdate`, so a marker placed in `Update` would be one frame
/// behind the image — which is invisible standing still and very visible mid-swing, exactly when
/// the element has to be trusted.
///
/// `Camera3d` without a further filter is enough because there is at most one 3D camera:
/// `render::attach_camera` bails out as soon as one exists.
pub fn place_arm_aim(
    players: Query<(&Hook, &AimPoint), With<LocalPlayer>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut markers: Query<(&ArmMarker, &ArmAimState, &mut Node), Without<ArmMarkerLabel>>,
    mut labels: Query<(&ArmMarkerLabel, &mut Node), Without<ArmMarker>>,
) {
    let Some((camera, camera_at)) = cameras.iter().next() else {
        return;
    };
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };

    // Read the two shapes first, so both `&mut Node` loops below can be write-only.
    let mut shapes = [shape_of(ArmAimState::default()); 2];
    for (marker, state, _) in markers.iter() {
        shapes[marker.side.index()] = shape_of(*state);
    }

    let aim = players.iter().next();
    let mut layout = [ArmLayout::default(); 2];
    for side in Side::ALL {
        let world = aim.and_then(|(hook, point)| target_of(hook, point, side));
        // `.ok()` and not an `expect`: a target behind the player or past the far plane is a
        // normal thing to be holding, and `world_to_viewport` reports it as an error. It becomes
        // the side slot, which is the truthful answer — "that arm is not in view".
        let at = world.and_then(|p| camera.world_to_viewport(camera_at, p).ok());
        layout[side.index()] = layout_for(side, shapes[side.index()], at, viewport);
    }

    for (marker, _, mut node) in &mut markers {
        let arm = layout[marker.side.index()];
        put(
            &mut node,
            match marker.part {
                MarkerPart::Glyph => arm.glyph,
                MarkerPart::Tether => arm.tether,
            },
        );
    }
    for (label, mut node) in &mut labels {
        put(&mut node, layout[label.0.index()].label);
    }
}

/// Writes the two colours — and neither of them carries information the shape does not already
/// carry.
///
/// Cyan is `maps.ron`'s *"gas, Vector Gear, anchor points"*, read through [`signal`] and never
/// written as a literal. [`NEUTRAL`] is the crosshair's own "the ray hits nothing worth having"
/// grey, borrowed rather than spelled a second time — a second literal is how two elements that
/// mean the same thing start looking different.
pub fn paint_arm_aim(
    data: Res<GameData>,
    mut markers: Query<
        (&ArmAimState, &mut BackgroundColor, &mut BorderColor),
        With<ArmMarker>,
    >,
) {
    let cyan = signal(&data, "cyan");
    for (state, mut fill, mut border) in &mut markers {
        // An outline glyph is transparent inside and coloured at the edge; a filled one is the
        // other way round. Which of the two it is comes out of the shape table, so the colour
        // cannot disagree with the geometry.
        let shape = shape_of(*state);
        let ink = match state {
            ArmAimState::Free => NEUTRAL,
            ArmAimState::Ready | ArmAimState::Busy | ArmAimState::Anchored => cyan,
        };
        let (wanted_fill, wanted_border) = if shape.border_px > 0.0 {
            (Color::NONE, ink)
        } else {
            (ink, Color::NONE)
        };
        if fill.0 != wanted_fill {
            fill.0 = wanted_fill;
        }
        if border.top != wanted_border {
            border.set_all(wanted_border);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{BodyId, HookArm};

    const SCREEN: Vec2 = Vec2::new(1280.0, 720.0);

    #[test]
    fn f171_a_busy_arm_is_not_a_ready_one() {
        // The rule the whole element hangs on. `vector::hook` only fires from `Idle`, so an arm
        // whose tip is out has to say so — a `Ready` ring over a retracting arm is a promise the
        // simulation does not keep.
        assert_eq!(state_for(&HookState::Idle, true), ArmAimState::Ready);
        assert_eq!(state_for(&HookState::Idle, false), ArmAimState::Free);
        assert_eq!(state_for(&HookState::Retracting, true), ArmAimState::Busy);
        assert_eq!(
            state_for(&HookState::Flying { target_m: Vec3::ZERO, body: BodyId(1) }, true),
            ArmAimState::Busy
        );
        assert_eq!(
            state_for(&HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO }, false),
            ArmAimState::Anchored,
            "an anchored arm is anchored whatever the aim ray happens to be looking at"
        );
    }

    #[test]
    fn f171_the_shape_table_is_the_only_place_the_arm_numbers_live() {
        for state in [
            ArmAimState::Free,
            ArmAimState::Ready,
            ArmAimState::Busy,
            ArmAimState::Anchored,
        ] {
            let shape = shape_of(state);
            let expected = if shape.tether_px.is_some() { 2 } else { 1 };
            assert_eq!(node_count(state), expected, "{state:?}: the two tables disagree");
        }
    }

    #[test]
    fn f171_an_arm_with_its_tip_out_previews_its_own_tip_and_not_the_shared_aim() {
        // The wiring that makes the two markers two markers. A shared `AimPoint` sits at one
        // place; each arm's tip sits somewhere else, and the arm that is holding has to preview
        // ITS point. Goes red the day somebody wires all four states to `aim.point_m`.
        let aim = AimPoint {
            point_m: Some(Vec3::new(0.0, 2.0, -50.0)),
            body: Some(BodyId(1)),
            anchorable: true,
        };
        let held = Vec3::new(-31.0, 17.0, -12.0);
        let flown = Vec3::new(44.0, 3.0, -20.0);
        let hook = Hook {
            arms: [
                HookArm {
                    state: HookState::Anchored { body: BodyId(1), local_m: Vec3::ZERO },
                    tip_m: held,
                },
                HookArm { state: HookState::Flying { target_m: flown, body: BodyId(2) }, tip_m: flown },
            ],
        };
        assert_eq!(target_of(&hook, &aim, Side::Left), Some(held));
        assert_eq!(target_of(&hook, &aim, Side::Right), Some(flown));

        // ...and an idle arm has no point of its own, so it falls back to the shared answer.
        let idle = Hook::default();
        assert_eq!(target_of(&idle, &aim, Side::Left), aim.point_m);
        assert_eq!(
            target_of(&idle, &AimPoint::default(), Side::Left),
            None,
            "nothing in range is not a point at the origin"
        );
    }

    #[test]
    fn f170_no_projected_point_can_push_a_marker_into_the_middle() {
        // ★ **The deliberate answer to the trap.** A world-tracked marker is eventually aimed
        // straight at, and `f170_nothing_covers_the_middle_of_the_screen` is a proven claim. So
        // the placement rule is swept over the whole screen, in every state, and the cluster
        // has to stay out of the box every time.
        let box_min_x = SCREEN.x * KEEP_OUT_LOW_PCT / 100.0;
        let box_max_x = SCREEN.x * KEEP_OUT_HIGH_PCT / 100.0;
        let box_min_y = SCREEN.y * KEEP_OUT_LOW_PCT / 100.0;
        let box_max_y = SCREEN.y * KEEP_OUT_HIGH_PCT / 100.0;

        for state in [
            ArmAimState::Free,
            ArmAimState::Ready,
            ArmAimState::Busy,
            ArmAimState::Anchored,
        ] {
            let shape = shape_of(state);
            let full_h = shape.glyph_h_px + shape.tether_px.map_or(0.0, |t| 4.0 + t);
            for side in Side::ALL {
                for step_x in 0..=64 {
                    for step_y in 0..=36 {
                        let at = Vec2::new(
                            SCREEN.x * step_x as f32 / 64.0,
                            SCREEN.y * step_y as f32 / 36.0,
                        );
                        for target in [Some(at), None] {
                            let l = layout_for(side, shape, target, SCREEN);
                            // The cluster: the letter's outer edge to the glyph's other edge.
                            let (lo, hi) = if l.label_right {
                                (l.glyph.x, l.label.x + LABEL_W_PX)
                            } else {
                                (l.label.x, l.glyph.x + shape.glyph_w_px)
                            };
                            let overlaps = lo < box_max_x
                                && hi > box_min_x
                                && l.glyph.y < box_max_y
                                && l.glyph.y + full_h > box_min_y;
                            assert!(
                                !overlaps,
                                "{state:?} {side:?} aimed at {at:?} put the cluster at \
                                 x {lo:.1}..{hi:.1}, y {:.1}..{:.1} — inside the keep-out box \
                                 x {box_min_x:.1}..{box_max_x:.1}, y {box_min_y:.1}..\
                                 {box_max_y:.1}",
                                l.glyph.y,
                                l.glyph.y + full_h
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn f171_a_marker_never_claims_the_wrong_half_of_the_screen() {
        // The push has a direction, and the direction is the point's own. A left arm holding
        // something on the right has to be drawn on the right — a marker shoved to "its" side
        // would be a second FIND-047, one abstraction later.
        let shape = shape_of(ArmAimState::Anchored);
        let middle_y = SCREEN.y * 0.5;
        for side in Side::ALL {
            let right = layout_for(side, shape, Some(Vec2::new(SCREEN.x * 0.52, middle_y)), SCREEN);
            assert!(
                right.glyph.x >= SCREEN.x * KEEP_OUT_HIGH_PCT / 100.0,
                "{side:?} holding a point right of centre was drawn at {:?}",
                right.glyph
            );
            assert!(right.label_right, "{side:?}: the letter has to stay outboard");

            let left = layout_for(side, shape, Some(Vec2::new(SCREEN.x * 0.48, middle_y)), SCREEN);
            assert!(
                left.glyph.x + shape.glyph_w_px <= SCREEN.x * KEEP_OUT_LOW_PCT / 100.0,
                "{side:?} holding a point left of centre was drawn at {:?}",
                left.glyph
            );
            assert!(!left.label_right);
        }

        // Dead on the axis there is no lean, and then the arm's own side decides — that is the
        // only thing an idle pair can honestly say (module header).
        let on_axis = Vec2::new(SCREEN.x * 0.5, middle_y);
        let l = layout_for(Side::Left, shape, Some(on_axis), SCREEN);
        let r = layout_for(Side::Right, shape, Some(on_axis), SCREEN);
        assert!(l.glyph.x < r.glyph.x, "Q ended up right of E: {:?} {:?}", l.glyph, r.glyph);
    }

    #[test]
    fn f171_a_marker_stays_on_the_screen() {
        // A point far outside the frustum still projects to a finite pixel a long way off, and a
        // marker parked at x = -4000 is a marker that does not exist.
        let shape = shape_of(ArmAimState::Anchored);
        for at in [
            Vec2::new(-9000.0, -4000.0),
            Vec2::new(40_000.0, 22_000.0),
            Vec2::new(f32::NAN, 12.0),
        ] {
            for side in Side::ALL {
                let l = layout_for(side, shape, Some(at), SCREEN);
                assert!(
                    l.glyph.x >= 0.0 && l.glyph.x + shape.glyph_w_px <= SCREEN.x,
                    "{side:?} aimed at {at:?} left the screen: {:?}",
                    l.glyph
                );
                assert!(l.glyph.y >= 0.0 && l.glyph.y <= SCREEN.y, "{:?}", l.glyph);
            }
        }
    }
}
