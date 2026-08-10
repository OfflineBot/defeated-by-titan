//! The `LocalOnly` transport: keyboard and mouse become an [`Intent`].
//!
//! **This is the only place in the game that knows what a key is.** Everything behind it
//! reads nothing but the `Intent` — and therefore does not know whether a human, a script
//! or one day the network is playing (`prompts/init.md` §6 rule 2).
//!
//! ⚠️ The bindings live **here in the code** for now, not in a RON file. They are not a
//! balancing number but a UI setting, and **rebindable keys are a requirement of the
//! bible** (3.5, accessibility) — they move into the options once `menu/` gets them. Until
//! then they are a default, not a design.
//!
//! ## The scheme (user, 2026-08-10, after the first human play session)
//!
//! `Q` `HOOK_LEFT` · `E` `HOOK_RIGHT` · `LMB` `SLASH_LEFT` · `RMB` `SLASH_RIGHT` ·
//! `F` `SLASH_LEFT` (second binding) · `Shift` `BOOST` · `Ctrl` `REEL_IN` · `Space` `JUMP` ·
//! `C` `DODGE` · `Tab` `MARK` · `WASD` movement.
//!
//! The ropes moved from the mouse to the keyboard because the user asked for them to be
//! **steerable**: a hand holds `Q` and `E` and still aims, and it cannot hold two mouse
//! buttons and still aim. `MARK` had to leave `Q` for that, and `Tab` was free.
//!
//! ⚠️ **`Q` and `E` no longer mean what a `--script` written before 2026-08-10 thinks they
//! mean**, and `hook left|right` presses a *mouse* button (`src/debug/mod.rs:220-224`) — which
//! is now a blade. Which scripts that touches is a finding, not a thing this file fixes.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::Inbox;
use crate::data::GameData;
use crate::shared::{LookOverride, Intent, LocalPlayer, PlayerId, Buttons, Tick};

/// Reads the real input and posts an [`Intent`] built from it into the inbox.
///
/// Runs in `FixedPreUpdate`, in the set `IntentSystems::Collect` — so **once per simulation
/// tick** and guaranteed **after** the script driver (`IntentSystems::Source`).
///
/// The mouse comes out of [`MouseSinceTick`] and **not** out of `AccumulatedMouseMotion`.
/// Why, and what it cost to find out, is in the header of [`gather_mouse_motion`] and in
/// `docs/BUGS.md` `B-002`.
pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: ResMut<MouseSinceTick>,
    tick: Res<Tick>,
    data: Res<GameData>,
    mut inbox: ResMut<Inbox>,
    mut look_override: ResMut<LookOverride>,
    mut look: Local<Look>,
    local: Query<&PlayerId, With<LocalPlayer>>,
) {
    // There is no such thing as "the player" — but there is exactly one that is ME. If he
    // does not exist (yet), that is not an error: the world is only just being built.
    let Some(me) = local.iter().next().copied() else {
        return;
    };

    let k = &data.game.camera;
    // Taken **unconditionally**, before the branch: an absolute `look` must not be dragged
    // off its angle one tick later by motion that was buffered before it. `tests/input.rs::
    // p3_a_script_look_still_overrides_the_mouse` is that sentence as a test.
    let d = std::mem::take(&mut mouse_motion.delta);
    if let Some((yaw, pitch)) = look_override.0.take() {
        // The script driver's "pretend" look vector (§12b). A mouse knows no absolute
        // angle — `look 0 -10` does, and that is exactly what a reproducible run needs.
        look.yaw = yaw;
        look.pitch = pitch;
    } else {
        look.yaw -= d.x * k.mouse_deg_per_px.to_radians();
        look.pitch = (look.pitch - d.y * k.mouse_deg_per_px.to_radians())
            .clamp(-k.pitch_limit_deg.to_radians(), k.pitch_limit_deg.to_radians());
    }

    let mut t = Buttons::NONE;
    t.set(Buttons::JUMP, keys.pressed(KeyCode::Space));
    t.set(Buttons::BOOST, keys.pressed(KeyCode::ShiftLeft));
    t.set(Buttons::REEL_IN, keys.pressed(KeyCode::ControlLeft));
    t.set(Buttons::DODGE, keys.pressed(KeyCode::KeyC));
    // The ropes are on the keyboard and the blades are on the mouse (user, 2026-08-10, after
    // the first time a human played this: the ropes have to be **steerable**). A hand can hold
    // `Q` and `E` and still aim; it cannot hold both mouse buttons and still aim. `MARK` had to
    // move off `Q` for that and went to `Tab`, which nothing else uses.
    t.set(Buttons::MARK, keys.pressed(KeyCode::Tab));
    t.set(Buttons::HOOK_LEFT, keys.pressed(KeyCode::KeyQ));
    t.set(Buttons::HOOK_RIGHT, keys.pressed(KeyCode::KeyE));
    // `F` stays on the left blade next to the left mouse button — a second binding, not a
    // leftover. It is the only route to `SLASH_LEFT` that `debug::script::parse_key` can
    // reach, because a script has no way to press a mouse button except `hook left|right`.
    t.set(
        Buttons::SLASH_LEFT,
        mouse_buttons.pressed(MouseButton::Left) || keys.pressed(KeyCode::KeyF),
    );
    t.set(Buttons::SLASH_RIGHT, mouse_buttons.pressed(MouseButton::Right));

    let forward = f32::from(keys.pressed(KeyCode::KeyW)) - f32::from(keys.pressed(KeyCode::KeyS));
    let strafe = f32::from(keys.pressed(KeyCode::KeyD)) - f32::from(keys.pressed(KeyCode::KeyA));

    inbox.push(
        me,
        Intent {
            move_x: strafe,
            move_y: forward,
            yaw: look.yaw,
            pitch: look.pitch,
            buttons: t,
            tick: tick.0,
        },
        tick.0,
    );
}

/// The look lives between the frames — it is the accumulated mouse motion, not its delta.
/// A `Local` and not a `Resource`, so it stays clear: this belongs to **this** system and
/// to nobody else.
#[derive(Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
}

/// The mouse motion of every **frame** since the last simulation **tick**.
///
/// A `Resource` and not a component: this is the state of a *device*, not of a player. There
/// is exactly one mouse on this machine the same way there is exactly one
/// `ButtonInput<KeyCode>` — and it stops being read the moment [`read_input`] has turned it
/// into an `Intent`. Nothing behind that seam ever sees it, so §6 rule 3 ("no player state in
/// a `Resource`") is untouched: an `Intent` is still what the simulation reads, and it still
/// hangs on a player.
#[derive(Resource, Debug, Default)]
pub struct MouseSinceTick {
    pub delta: Vec2,
}

/// Sums the frame's mouse motion into [`MouseSinceTick`] — **once per frame**, before the
/// fixed loop.
///
/// ## The bug this exists for (`B-002`, `docs/PLAN-GAME.md` §8 `P3`)
///
/// `AccumulatedMouseMotion` is **assigned** in `PreUpdate`, once per frame
/// (`bevy_input-0.19.0/src/mouse.rs:257-267` — the last line is `= delta`, not `+= delta`).
/// The fixed loop runs the entire `FixedMain` schedule **0..n times per frame**
/// (`bevy_time-0.19.0/src/fixed.rs:249-255`, `while ...expend()`). Reading a per-frame
/// resource from `FixedPreUpdate` therefore
///
/// - **throws motion away** on every frame in which no fixed step is due — measured
///   **58.7 %** at 144 fps and **88.4 %** at 500 fps, and
/// - **applies it again** on every catch-up frame — measured **+198.3 %** at 20 fps.
///
/// At exactly 60 fps, and only there, the two rates cancel and the ratio is 1.000. That is why
/// nobody saw it: the one rate this game was ever run at is the one rate at which it is right.
///
/// `RunFixedMainLoopSystems::BeforeFixedMainLoop` is the set that runs **exactly once per
/// frame regardless of the number of fixed updates**
/// (`bevy_app-0.19.0/src/main_schedule.rs:401-403`), and it sits after `PreUpdate` in the main
/// schedule order (`:224-232`) — so Bevy's own accumulation has already happened when this
/// runs.
pub fn gather_mouse_motion(
    motion: Res<AccumulatedMouseMotion>,
    mut pending: ResMut<MouseSinceTick>,
) {
    pending.delta += motion.delta;
}
