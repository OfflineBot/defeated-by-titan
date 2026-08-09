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

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::Inbox;
use crate::data::GameData;
use crate::shared::{LookOverride, Intent, LocalPlayer, PlayerId, Buttons, Tick};

/// Reads the real input and posts an [`Intent`] built from it into the inbox.
///
/// Runs in `FixedPreUpdate`, in the set `IntentSystems::Collect` — so **once per simulation
/// tick** and guaranteed **after** the script driver (`IntentSystems::Source`). Mouse
/// motion gathered between two ticks is not lost in the process: `AccumulatedMouseMotion`
/// sums it up across the frames.
pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
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
    if let Some((yaw, pitch)) = look_override.0.take() {
        // The script driver's "pretend" look vector (§12b). A mouse knows no absolute
        // angle — `look 0 -10` does, and that is exactly what a reproducible run needs.
        look.yaw = yaw;
        look.pitch = pitch;
    } else {
        let d = mouse_motion.delta;
        look.yaw -= d.x * k.mouse_deg_per_px.to_radians();
        look.pitch = (look.pitch - d.y * k.mouse_deg_per_px.to_radians())
            .clamp(-k.pitch_limit_deg.to_radians(), k.pitch_limit_deg.to_radians());
    }

    let mut t = Buttons::NONE;
    t.set(Buttons::JUMP, keys.pressed(KeyCode::Space));
    t.set(Buttons::BOOST, keys.pressed(KeyCode::ShiftLeft));
    t.set(Buttons::REEL_IN, keys.pressed(KeyCode::ControlLeft));
    t.set(Buttons::DODGE, keys.pressed(KeyCode::KeyC));
    t.set(Buttons::MARK, keys.pressed(KeyCode::KeyQ));
    t.set(Buttons::HOOK_LEFT, mouse_buttons.pressed(MouseButton::Left));
    t.set(Buttons::HOOK_RIGHT, mouse_buttons.pressed(MouseButton::Right));
    t.set(Buttons::SLASH_LEFT, keys.pressed(KeyCode::KeyF));
    t.set(Buttons::SLASH_RIGHT, keys.pressed(KeyCode::KeyE));

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
