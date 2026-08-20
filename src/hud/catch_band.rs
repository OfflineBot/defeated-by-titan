//! `F-016` — **the search band**: where, from where to where, the aim assist is looking.
//!
//! > *„es soll in der ui angezeigt werden von wo bis wo gesearched wird damit man das besser
//! > einstellen kann!"* — the user, 2026-08-19
//!
//! The last clause is the acceptance criterion and not a nicety: this element exists **to be
//! read while the number is being changed**. `PlayerSettings::assist_catch_pct` is a
//! percentage of a constant nobody can see, and until today the only way to find out what
//! 40 % buys was to fire a hook and guess. The band turns the knob into a distance on the
//! screen.
//!
//! # What is drawn, and why it is a LINE
//!
//! Since `docs/FINDINGS.md` FIND-133 the candidate search is a **1D screen-horizontal sweep**:
//! `game.ron: vector.assist_probe_steps` rays per side at `theta = catch * (i + 1) / steps`
//! about the camera's own right axis, and the worst camera-vertical deviation of a published
//! point is **0.000006 deg**. The search really is a line, so this draws a line and not a box —
//! a box would claim a vertical reach the sweep does not have, which is the same class of lie
//! FIND-098, FIND-099 and FIND-129 were each a case of.
//!
//! | node | how many | what it says |
//! |---|---|---|
//! | [`CatchTick`] | `assist_probe_steps` per side | **one ray**, at its own angle |
//! | the outermost tick of each side (`step == steps - 1`) | 2 | the **end mark**: exactly `±catch`, drawn [`END_H_PX`] tall instead of [`TICK_H_PX`] |
//! | [`CatchRule`] | 2 | the **span** between the crosshair and each end mark |
//!
//! **The steps are drawn, and that is a decision** (the alternative was two end marks and a
//! bare span): eight rays a side is the resolution the player is buying, an anchor between two
//! probes is not found at all, and the ticks visibly crowd together as the catch narrows — so
//! the picture answers *"how far"* and *"how finely"* with the same eighteen nodes, at no extra
//! cost, because the tick index **is** the probe index.
//!
//! # Where the angle comes from — it may not be recomputed here
//!
//! Three numbers make the band, and **two of them are read out of the same expression
//! `vector::aim` reads**:
//!
//! - the half-width: [`PlayerSettings::assist_catch_deg`], the identical accessor
//!   `vector::aim::aim` calls to fill `ScoreContext::catch_rad`. There is one mapping from
//!   percent to degrees in this repository and both sides call it, so `ASSIST_CATCH_MAX_DEG`
//!   can move without this file knowing;
//! - the step count: `game.ron: vector.assist_probe_steps`, the identical field
//!   `vector::aim::aim` iterates;
//! - the distribution of the steps, `theta = catch * (i + 1) / steps`, which **is** written a
//!   second time in [`probe_theta_rad`] — because `hud` may not reach into `vector` (the allow
//!   list in `docs/architecture.md` has no `hud -> vector` line) and mirroring a `Vec3`
//!   generator through `shared` would give the sweep a second writer.
//!
//! That third one is the drift risk, and it is handled exactly the way the same problem was
//! handled for the eye in [`crosshair::eye`](crate::hud::crosshair::eye):
//! `tests/hud.rs::f016_the_band_stands_on_the_probe_rays_the_search_really_casts` projects the
//! directions **`vector::aim::probe_dirs` itself returns** and asserts every drawn tick stands
//! on one of them. The day the sweep changes shape, that test goes red before the picture lies.
//!
//! And the fourth number — where an angle lands in pixels — is never computed here at all:
//! [`place_catch_band`] projects a point one metre down each probe direction through
//! `Camera::world_to_viewport`, the same call [`place_arm_aim`](crate::hud::arm_aim::place_arm_aim)
//! makes. So the band follows `fov_deg` and the viewport by construction.
//!
//! ⚠️ **The camera basis and the aim basis are the same basis**, and that is why projecting
//! from the camera is legitimate: `render::camera::rotate_camera` builds
//! `Ry(yaw) * Rx(pitch)` with no roll, whose local X is `(cos yaw, 0, -sin yaw)` — the exact
//! vector `vector::aim::look_basis` uses as `right` — and whose local `-Z` is
//! `Intent::look_dir()`. The one-tick parallax of FIND-133's last section does not touch this
//! element: a **direction** projected from the camera's own position carries no distance, so
//! there is nothing for a translation to move.
//!
//! # The crosshair's keep-out box, and how this element resolves it
//!
//! The band is level with the crosshair and runs through the middle of the screen, which is
//! the one region `F-170` protects. It is the **second** documented exemption from
//! [`KEEP_OUT_PCT`](crate::hud::KEEP_OUT_PCT), on FIND-098's own argument and no new one: that
//! argument was *"the one element whose position is an angle rather than a place is measured
//! against [`SIGHT_CORE_PX`] instead, because the whole angular range it can occupy fits inside
//! this rectangle"*. The band **is** that angular range — at 1280 x 720 and `fov_deg: 60` its
//! end marks stand 227 px from centre at 100 %, 88 px at 40 % and 11 px at the 5 % the slider
//! steps in, and the box's own edge is at 128 px. Pushing the band out of the box would put the
//! 40 % band **outside its own extent**: it would show a wider search than the one running, for
//! every setting below about 55 %. That is not a smaller lie than FIND-129's, it is the same
//! one.
//!
//! What the band gives up instead is [`SIGHT_CORE_PX`] — the six pixels the player is actually
//! cutting. The two span rules stop at the core's edge and any tick that would stand on the core
//! is not drawn. **Nothing is moved**: a tick is drawn on its ray or it is not drawn at all, so
//! the picture can be short but it cannot be wrong.
//! `tests/hud.rs::f016_the_band_keeps_the_sight_core_clear` measures it over the whole slider,
//! and no *end mark* has ever been dropped by it — the narrowest catch the slider can dial
//! (5 % = 1 deg) still projects 10.9 px out, past the 6 px core.
//!
//! # When there is no REACH there is no band — and the colour says whether anything is searching
//!
//! **Two predicates, deliberately, and `Q-042` is the reason.** The band was gated on
//! [`PlayerSettings::assist_is_on`] — `catch > 0 && strength > 0` — until 2026-08-20, and
//! **both knobs ship at 0** (`shared::settings::PlayerSettings::from_world`). So the element
//! whose whole purpose is *„damit man das besser einstellen kann"* was absent in the one moment
//! it exists for: a player opens `Settings`, turns *Aim assist reach* up to any value at all,
//! and **nothing happens** — because a second, differently-named row is the master switch, and
//! the row he touched is the one that looks like it should draw the picture. Even the band's
//! own menu test had to press the strength button in secret before it could see one
//! (`tests/menu.rs::nudge_reach`), which is the trap written down as code.
//!
//! So the band is drawn from [`PlayerSettings::assist_has_reach`] — the reach alone. What that
//! costs is that **drawing and searching are now two decisions**, and this file says which is
//! which out loud rather than letting a picture imply it:
//!
//! | reach | strength | drawn | colour | what is true |
//! |---|---|---|---|---|
//! | 0 % | anything | **nothing** | — | free aim: there is no extent to draw |
//! | > 0 % | 0 % | the band, geometry unchanged | [`IDLE`] | *this is how far the search would look.* **No probe ray is cast** |
//! | > 0 % | > 0 % | the band, geometry unchanged | [`NEUTRAL`] | the sweep is running, over exactly these rays |
//!
//! ⚠️ **The PROBE is still [`PlayerSettings::assist_is_on`]'s and only its**, and
//! [`PlayerSettings::assist_has_reach`] may never be passed to `vector::aim`: `F-002`'s
//! guarantee is that 0 % is bit-for-bit the aim the game had before this feature existed, and
//! `tests/vector_hooks.rs::f016_at_zero_percent_the_aim_is_bit_for_bit_the_one_the_game_had_before`
//! is what holds it.
//!
//! **And this is not FIND-098 / FIND-099 / FIND-127 / FIND-129 with a new shape** — this
//! session has found four elements that drew something the game did not really have, so the
//! question gets answered and not waved past. In every one of those the *geometry* lied: a
//! marker stood where the thing was not. Here the geometry is bit-identical in both states, on
//! the probe rays `vector::aim::probe_dirs` itself returns, because the geometry answers *"how
//! far does my reach go"* — which is exactly as true when nothing is in flight as when
//! something is. The only claim that differs between the states is *"a ray is being cast right
//! now"*, and that claim is carried rather than implied:
//! `tests/hud.rs::f016_the_reach_alone_draws_the_band_and_the_colour_says_whether_it_searches`
//! asserts the two colours differ, that both clear 3:1 over the settings backdrop, and that
//! every tick stands in the same pixel in both.
//!
//! With the reach at 0 nothing is drawn at all, so a player who has never touched a slider
//! still sees exactly the HUD `F-170` and `F-171` were photographed with, node for node.

use bevy::prelude::*;

use crate::data::GameData;
use crate::hud::arm_aim::SIGHT_CORE_PX;
use crate::hud::crosshair::NEUTRAL;
use crate::hud::{HudElement, ShowWhileTuning, TUNING_Z};
use crate::shared::{PlayerSettings, Side};

/// One probe ray of the sweep, drawn where that ray goes.
///
/// `step` is `vector::aim::probe_dirs`' own `i`: `0` is the innermost ray and `steps - 1` is
/// the outermost, which sits exactly on `±catch` and is the end mark.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatchTick {
    pub side: Side,
    pub step: u32,
}

/// The span between the crosshair and one end mark. One per side, because the sight core is
/// between them.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatchRule(pub Side);

/// Thickness of a probe tick.
///
/// **Shape constants, not balancing values** — the same argument
/// [`crosshair::shape_of`](crate::hud::crosshair::shape_of) makes: a tick's width belongs in
/// RON no more than the fact that a bar is a rectangle does (`CLAUDE.md` rule 2 names *"a titan
/// kind, a blade tier, a gas cost"*). The one number in this element that **is** a game value —
/// the catch angle — is read out of the settings and out of `game.ron`, and nothing else here
/// decides anything.
pub const TICK_W_PX: f32 = 2.0;
/// Height of an inner probe tick.
pub const TICK_H_PX: f32 = 7.0;
/// Height of the outermost tick of each side — the end mark, and the number he is tuning.
pub const END_H_PX: f32 = 17.0;
/// Thickness of the span rule.
pub const RULE_H_PX: f32 = 1.0;

/// The band while **no probe ray is being cast** — the reach is dialled up but
/// [`PlayerSettings::assist_is_on`] is false, which today is every fresh run (`F-025` is not
/// built, so `assist_strength_pct` does nothing at all except open that gate).
///
/// It is [`NEUTRAL`]'s own white at a lower alpha and **not** a second hue: the band is the
/// same measurement in both states, and a change of colour would say the ruler had changed
/// rather than the search. Composited over `menu::plate::BACKDROP`'s 0.90 in linear light
/// (FIND-136 §2), 0.40 reads at **3.36:1 over the worst frame the game can put behind the menu
/// and 8.7:1 over black** — clear of WCAG 1.4.11's 3:1 in both, which it has to be, because
/// with `F-025` unbuilt **this is the only state a player can be in today** and the ruler has
/// to be readable in it. The live band is 5.43:1 / 15.4:1 on the same arithmetic.
///
/// ⚠️ The two states read only ~1.6:1 against **each other**, and that is a knowing trade and
/// not an oversight: pushing them 3:1 apart would need the idle one down at alpha 0.22, where
/// it reads 1.8:1 against its own background — illegible in the state the player is actually
/// in. Legible-in-both beat distinguishable-at-a-glance.
pub const IDLE: Color = Color::srgba(1.0, 1.0, 1.0, 0.40);

/// The angle of one probe off the crosshair, radians — **the second spelling of
/// `vector::aim::probe_dirs`' own `theta`**.
///
/// It is written twice for the reason the module header gives (there is no `hud -> vector`
/// edge), and it is pinned by
/// `tests/hud.rs::f016_the_band_stands_on_the_probe_rays_the_search_really_casts`, which
/// projects what `probe_dirs` actually returns and compares it to what this drew. Two spellings
/// of one angle are exactly how a marker and a rope end up in two places — the same trap
/// [`crosshair::eye`](crate::hud::crosshair::eye) sits in, and it is guarded the same way.
pub fn probe_theta_rad(catch_rad: f32, steps: u32, step: u32) -> f32 {
    let steps = steps.max(1);
    catch_rad * (step + 1) as f32 / steps as f32
}

/// `2 * assist_probe_steps` ticks and two span rules, all of them hidden.
///
/// Hidden and not absent: an entity that comes and goes changes the archetype every time the
/// player touches the slider, and it would make the HUD's node count depend on when you look —
/// the same reason the crosshair's corner marks and the arm tether are switched off with
/// `Display::None` instead of being despawned. The step count comes out of `game.ron` here and
/// nowhere else in this file, so the picture cannot have a different number of rays in it than
/// the sweep casts.
pub fn spawn_catch_band(mut commands: Commands, data: Res<GameData>) {
    let steps = data.game.vector.assist_probe_steps.max(1);
    for side in Side::ALL {
        commands.spawn((
            Name::new(format!("hud_catch_rule_{side:?}")),
            CatchRule(side),
            HudElement,
            ShowWhileTuning,
            GlobalZIndex(TUNING_Z),
            // Repainted every frame it is drawn — [`place_catch_band`] decides between
            // [`NEUTRAL`] and [`IDLE`]. This is only what an undrawn node happens to hold.
            BackgroundColor(IDLE),
            Node {
                position_type: PositionType::Absolute,
                height: Val::Px(RULE_H_PX),
                display: Display::None,
                ..default()
            },
        ));
        for step in 0..steps {
            commands.spawn((
                Name::new(format!("hud_catch_tick_{side:?}_{step}")),
                CatchTick { side, step },
                HudElement,
                // ⚠️ The whole point of the element: it is the picture of the number the
                // `Aim assist reach` row writes, so it stays up while that row is being used
                // (`hud::ShowWhileTuning`) and it draws over the backdrop (`hud::TUNING_Z`).
                ShowWhileTuning,
                GlobalZIndex(TUNING_Z),
                BackgroundColor(IDLE),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(TICK_W_PX),
                    height: Val::Px(tick_h_px(step, steps)),
                    display: Display::None,
                    ..default()
                },
            ));
        }
    }
}

/// The end mark is the outermost probe, drawn taller. It is not a node of its own: the ray at
/// `±catch` **is** the edge of the search, and a separate node for it could stand somewhere the
/// outermost ray does not.
pub fn tick_h_px(step: u32, steps: u32) -> f32 {
    if step + 1 >= steps.max(1) {
        END_H_PX
    } else {
        TICK_H_PX
    }
}

/// **The band itself:** projects each probe direction and puts its tick there.
///
/// In `PostUpdate` after `TransformSystems::Propagate` and `CameraUpdateSystems` and before
/// `UiSystems::Layout`, for the reason
/// [`place_arm_aim`](crate::hud::arm_aim::place_arm_aim) is: the camera's `GlobalTransform` and
/// its viewport size are both written in `PostUpdate`. It is also what makes the element answer
/// the knob **in the same tick it moves** — `menu`'s slider and `debug`'s `settings
/// assist_catch <n>` both write `PlayerSettings` in `Update`, and this reads it one stage later
/// in the same frame. No restart, no respawn, no cached angle.
///
/// Every write is compared first, so a player who is not touching the slider produces **zero**
/// writes however hard he swings the camera: the band's screen position is a function of the
/// projection and the catch angle only, and a rotation does not move it (a perspective camera
/// maps a direction at `theta` off its own forward axis to the same pixel column at every yaw
/// and pitch — which is also why the band is a thing that can be *learned*).
pub fn place_catch_band(
    data: Res<GameData>,
    settings: Option<Res<PlayerSettings>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut ticks: Query<(&CatchTick, &mut Node, &mut BackgroundColor), Without<CatchRule>>,
    mut rules: Query<(&CatchRule, &mut Node, &mut BackgroundColor), Without<CatchTick>>,
) {
    let steps = data.game.vector.assist_probe_steps.max(1);
    let settings = settings.as_deref();
    // **The drawing predicate is the REACH** (`Q-042`, module header): this is the picture of
    // the `Aim assist reach` row, and it may not need a second row's permission to appear.
    let live = settings
        .filter(|s| s.assist_has_reach())
        .map(|s| s.assist_catch_deg().to_radians());
    // **The searching predicate is `vector::aim`'s own**, and it decides exactly one thing
    // here: the colour. With the strength at 0 no probe ray is cast at all, so the band is
    // drawn in [`IDLE`] — the same reach, said as *would look* instead of *is looking*.
    let paint = if settings.is_some_and(|s| s.assist_is_on()) { NEUTRAL } else { IDLE };
    let camera = cameras.iter().next();

    let Some((catch_rad, (camera, camera_at))) = live.zip(camera) else {
        hide_all(&mut ticks, &mut rules);
        return;
    };
    let eye_m = camera_at.translation();
    let forward = *camera_at.forward();
    let right = *camera_at.right();
    let project = |dir: Vec3| camera.world_to_viewport(camera_at, eye_m + dir).ok();

    // The crosshair's own pixel, out of the same projection — not `viewport / 2`. A viewport
    // whose origin is not the screen's would put the whole band half a screen off, and the
    // crosshair is what the band is measured from.
    let Some(centre) = project(forward) else {
        hide_all(&mut ticks, &mut rules);
        return;
    };

    // `step == steps - 1` is the end mark; its pixel is also where each span rule stops.
    let mut end_px = [None, None];
    for (tick, mut node, mut colour) in &mut ticks {
        let sign = match tick.side {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let (sin_t, cos_t) = probe_theta_rad(catch_rad, steps, tick.step).sin_cos();
        // The identical expression `probe_dirs` builds: `look * cos + right * (sign * sin)`.
        let at = project(forward * cos_t + right * (sign * sin_t));
        // A tick standing on the pixels the player is cutting is **dropped, never moved** —
        // the band may be short, it may not be wrong (module header).
        let want = at.filter(|px| (px.x - centre.x).abs() >= SIGHT_CORE_PX);
        if tick.step + 1 >= steps {
            end_px[tick.side.index()] = want;
        }
        match want {
            Some(px) => {
                put(
                    &mut node,
                    px.x - TICK_W_PX * 0.5,
                    px.y - tick_h_px(tick.step, steps) * 0.5,
                    None,
                );
                repaint(&mut colour, paint);
            }
            None => hide(&mut node),
        }
    }

    for (rule, mut node, mut colour) in &mut rules {
        // From the edge of the sight core out to this side's end mark.
        let inner = match rule.0 {
            Side::Left => centre.x - SIGHT_CORE_PX,
            Side::Right => centre.x + SIGHT_CORE_PX,
        };
        let span = end_px[rule.0.index()].map(|px| match rule.0 {
            Side::Left => (px.x, inner - px.x),
            Side::Right => (inner, px.x - inner),
        });
        match span {
            Some((left, width)) if width > 0.0 => {
                put(&mut node, left, centre.y - RULE_H_PX * 0.5, Some(width));
                repaint(&mut colour, paint);
            }
            _ => hide(&mut node),
        }
    }
}

/// Writes `left`, `top`, `display` and — for a rule — `width`. Compared before written, so a
/// standing band costs nothing (`CLAUDE.md` rule 6).
fn put(node: &mut Node, left: f32, top: f32, width: Option<f32>) {
    let left = Val::Px(left);
    if node.left != left {
        node.left = left;
    }
    let top = Val::Px(top);
    if node.top != top {
        node.top = top;
    }
    if let Some(width) = width {
        let width = Val::Px(width);
        if node.width != width {
            node.width = width;
        }
    }
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
}

/// The one place the two states become visible. Compared before written like everything else
/// in this file — a band standing at a settled setting writes nothing at all (rule 6).
fn repaint(colour: &mut BackgroundColor, want: Color) {
    if colour.0 != want {
        colour.0 = want;
    }
}

fn hide(node: &mut Node) {
    if node.display != Display::None {
        node.display = Display::None;
    }
}

fn hide_all(
    ticks: &mut Query<(&CatchTick, &mut Node, &mut BackgroundColor), Without<CatchRule>>,
    rules: &mut Query<(&CatchRule, &mut Node, &mut BackgroundColor), Without<CatchTick>>,
) {
    for (_, mut node, _) in ticks.iter_mut() {
        hide(&mut node);
    }
    for (_, mut node, _) in rules.iter_mut() {
        hide(&mut node);
    }
}
