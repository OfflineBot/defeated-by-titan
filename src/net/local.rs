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
//!
//! ## `F-008`: the double-tap is detected **here**, and that is the seam working
//!
//! *„mit doppel leertaste boostet man stark in die lauf richtung"* (user, 2026-08-12,
//! `docs/NEXT.md` §1c). A double-tap is a property of a **key**, not of the simulation, so it
//! is resolved on this side of the `Intent` and the simulation never learns that `Space` exists
//! — it reads `Buttons::DODGE`, which was declared on day one and written by nobody until now.
//! A script therefore reaches the dodge with `press dodge`, and one day a network client sends
//! the same bit. Both of those are the reason the seam is here at all.
//!
//! **The clock is [`Tick`], and it has to be.** `Time<Virtual>` is what `--ticks` and the seeded
//! rng are held steady against; reading it here would make the window a different length under
//! `--headless` than on a machine with a window. The tick counter is the one clock that ticks
//! once per simulation step by definition.
//!
//! **The edge is taken per tick, not with `just_pressed`, and that is `B-002` again.**
//! `ButtonInput::just_pressed` is a *per-frame* flag, cleared in `PreUpdate`. This system runs
//! in `FixedPreUpdate`, i.e. **0..n times per frame** — so on a catch-up frame with two fixed
//! steps `just_pressed` would be true in **both** of them and a single press would be a
//! double-tap in two ticks; on a frame with no fixed step the press would be dropped entirely.
//! That is exactly the failure [`gather_mouse_motion`] exists for, in another dress. Hence
//! [`SpaceEdge`]: the state of `Space` **at the previous tick**, kept by this system, compared
//! per tick. Every other button in this file is sampled with `pressed()` per tick as well, so
//! `Space` now behaves like all of them and no differently.

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
    mut space: Local<DodgeTap>,
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

    let jump = keys.pressed(KeyCode::Space);
    let dodge = space.feed(
        jump,
        keys.pressed(KeyCode::KeyC),
        tick.0,
        data.game.vector.dodge_double_tap_window_ticks,
    );

    let mut t = Buttons::NONE;
    // The first tap of a dodge is **still a jump**, and so is the second. Nothing here consumes
    // `Space` — a dodge on the ground is a jump that then throws you, and swallowing the jump
    // would make the move feel like it ate an input. `F-008` adds a button, it does not steal
    // one.
    t.set(Buttons::JUMP, jump);
    t.set(Buttons::BOOST, keys.pressed(KeyCode::ShiftLeft));
    // `Ctrl` **and** `S`. The user, 2026-08-12 (`docs/NEXT.md` §1a): *„mit s »spannt« man nur
    // das seil!"* — in the air `S` is the one WASD key that produces no thrust
    // (`player::locomotion::air_thrust`), and what it does instead is tension the rope. A
    // SECOND binding and not a move: `Ctrl` has to stay reachable for a hand that is holding
    // `W`, and `scripts/f-flight-cut.txt` presses it. On the ground `S` keeps walking him
    // backwards — the axis below is untouched.
    t.set(
        Buttons::REEL_IN,
        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::KeyS),
    );
    // **`C` stays, and it is not a leftover.** Two routes to one bit, exactly like `F` next to
    // the left mouse button above:
    //   - a double-tap is a **gesture**, and the bible asks for accessibility (3.5) — a player
    //     who cannot tap twice inside 0.3 s must still be able to dodge, and a gesture is not
    //     rebindable in an options menu the way a key is;
    //   - `C` is what survives when the bindings become a RON file; the double-tap is what
    //     survives as the *feel* the user asked for. Neither replaces the other.
    // Both arrive as **one tick** of `DODGE` — see [`DodgeTap`] for why a held `C` may not.
    t.set(Buttons::DODGE, dodge);
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

/// `F-008`'s edge memory: what `Space` and `C` did **last tick**, and when `Space` was last
/// tapped.
///
/// A `Local` and not a `Resource`, like [`Look`] and for the same reason — this belongs to
/// [`read_input`] and to nobody else. It is per **machine**, not per player: there is one
/// keyboard here the same way there is one mouse, and everything past the `Intent` is per
/// player again.
#[derive(Default)]
pub struct DodgeTap {
    /// `Space` at the end of the previous tick. Not `just_pressed` — see the file header,
    /// `B-002`.
    space_down: bool,
    /// `C` at the end of the previous tick.
    dodge_key_down: bool,
    /// The tick of the last **first** tap that is still waiting for its partner. `None` once it
    /// has been spent or has expired.
    armed_at: Option<u64>,
}

impl DodgeTap {
    /// Whether `Buttons::DODGE` is pressed **this** tick. True on at most one tick per gesture.
    ///
    /// Three properties, and each one is a test in `tests/input.rs`:
    ///
    /// - **Both routes are edges.** A held `C` fires once, not sixty times a second. It has to
    ///   be that way: a dodge costs a flat `vector.gas_dodge` (45), so a `DODGE` bit that stayed
    ///   true while a key is held would empty a 300-gas tank in **seven ticks** — 0.11 s — and
    ///   the player would only ever see that his gas was gone. The rate limit is the edge, not a
    ///   cooldown; `F-008`'s own cooldown is a separate thing and is not built.
    /// - **The first tap is consumed.** Three taps inside the window are **one** dodge, not two.
    ///   Without `armed_at.take()` the second tap would arm the third, and a player drumming on
    ///   `Space` would pay 45 gas per tap.
    /// - **A late second tap re-arms rather than failing.** Tap, wait a second, tap, tap: the
    ///   third is the partner of the second. Anything else would make a mistimed pair swallow
    ///   the next honest attempt.
    ///
    /// `saturating_sub` and `<=`: the window is inclusive, and a tick counter that ever went
    /// backwards (a rewind, one day) yields `0` and a dodge rather than an underflow panic in
    /// the input path.
    pub fn feed(&mut self, space: bool, dodge_key: bool, tick: u64, window_ticks: u64) -> bool {
        let space_edge = space && !self.space_down;
        let key_edge = dodge_key && !self.dodge_key_down;
        self.space_down = space;
        self.dodge_key_down = dodge_key;

        let mut fired = key_edge;
        if space_edge {
            match self.armed_at {
                Some(first) if tick.saturating_sub(first) <= window_ticks => {
                    self.armed_at = None;
                    fired = true;
                }
                _ => self.armed_at = Some(tick),
            }
        }
        fired
    }
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
