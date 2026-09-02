//! The `LocalOnly` transport: keyboard and mouse become an [`Intent`].
//!
//! **This is the only place in the game that knows what a key is.** Everything behind it
//! reads nothing but the `Intent` — and therefore does not know whether a human, a script
//! or one day the network is playing (`prompts/init.md` §6 rule 2).
//!
//! ⚠️ Since 2026-09-01 the bindings are **`PlayerSettings::binds`** (`F-172`): the scheme
//! below is `shared::settings::KeyBinds::DEFAULT`, the keybinds page in `menu::settings`
//! rebind them, and `saves/settings.ron` keeps them. They are a UI setting and not a
//! balancing number, which is why no RON tuning file ever got a key for them. The movement
//! axes (`WASD`) and the mouse buttons are still fixed — see `KeyBinds`' own doc for why that
//! is said out loud.
//!
//! ## The DEFAULT scheme (user, 2026-08-10, after the first human play session)
//!
//! `Q` `HOOK_LEFT` · `E` `HOOK_RIGHT` · `LMB` `SLASH_LEFT` · `RMB` `SLASH_RIGHT` ·
//! `F` `SLASH_LEFT` (second binding) · `Shift` `BOOST` · `Ctrl` `REEL_IN` · `Space` `JUMP` ·
//! `C` `DODGE` · `Tab` `MARK` · `WASD` movement · **the wheel sets the aim spread**.
//!
//! ⚠️ **`S` is movement and nothing else.** It was briefly a second binding for `REEL_IN`;
//! that was a misreading of *„mit s spannt man nur das seil"* and the user reported it as a
//! defect the first time he played it (`docs/NEXT.md` §1A req 7). See the `REEL_IN` line
//! below.
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
use crate::shared::settings::HookFire;
use crate::shared::{
    Hook, HookState, LookOverride, Intent, LocalPlayer, PlayerId, PlayerSettings, Buttons, Tick,
};

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
    settings: Res<PlayerSettings>,
    mut look: Local<Look>,
    mut space: Local<DodgeTap>,
    // `F-009` — `[A, D]`, one arming state per side. See the comment at the call site for why
    // it cannot be one shared state.
    mut flips: Local<[DodgeTap; 2]>,
    // The toggle state of the two rope triggers — `[left, right]`, like `flips`. A `Local`
    // for the same reason as every other state in this file: it belongs to this machine's
    // keyboard, not to a player.
    mut hook_latches: Local<[HookLatch; 2]>,
    // Read-only, to un-latch when an arm lets go on its own (miss, body gone, overextended) —
    // `Hook` is a `shared` component and `vector::hook` stays its only writer. Without this a
    // missed tap would leave the latch up, and the next honest tap would toggle a rope that
    // no longer exists instead of firing.
    hooks: Query<&Hook, With<LocalPlayer>>,
    local: Query<&PlayerId, With<LocalPlayer>>,
) {
    // There is no such thing as "the player" — but there is exactly one that is ME. If he
    // does not exist (yet), that is not an error: the world is only just being built.
    let Some(me) = local.iter().next().copied() else {
        return;
    };

    // ⚠️ **The mouse reads `PlayerSettings`, not `game.ron` — since 2026-08-13.** The two
    // start out as the same number (`shared::settings` seeds the resource out of
    // `game.ron: camera`); what changed is that the person at the keyboard may move it
    // afterwards, which is the whole of *„zudem fehlen settings"*. `game.ron` stays the source
    // of the **starting** value and of everything the simulation is balanced with; a
    // sensitivity is neither.
    let degrees_per_px = settings.mouse_deg_per_px.to_radians();
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
        look.yaw -= d.x * degrees_per_px;
        // **Invert Y is a factor, not a branch.** `pitch_sign()` is `+1` or `-1`, so the line
        // below is the same line it was before the setting existed — and there is no second
        // path through it that only inverted players ever take.
        let limit = settings.pitch_limit_deg.to_radians();
        look.pitch = (look.pitch - settings.pitch_sign() * d.y * degrees_per_px)
            .clamp(-limit, limit);
    }

    // **Every key below comes out of `settings.binds`** (`F-172`, 2026-09-01) — the defaults
    // are the 2026-08-10 scheme, verbatim, and `KeyBinds::DEFAULT` is where it now lives.
    // The movement axes (`WASD`) and the mouse buttons are still fixed; see [`crate::shared::
    // settings::KeyBinds`] for why that is said out loud instead of implied.
    let b = settings.binds;
    let jump = keys.pressed(b.jump);
    let dodge = space.feed(
        jump,
        keys.pressed(b.dodge),
        tick.0,
        data.game.vector.dodge_double_tap_window_ticks,
    );

    // `F-009` — **two independent double-tap detectors, one per side**, and they are literally
    // [`DodgeTap`]s: the gesture is the same one `Space` performs, so it gets the same code
    // rather than a second implementation with the same three off-by-ones in it. The `false`
    // is the type's second route (`C` for the dash) — a flip has no single-key binding today,
    // and the day it gets one it goes exactly there.
    //
    // Two states and not one, because `A`-then-`D` must **not** be a flip: that is a player
    // changing direction, not asking for anything, and one shared arming tick would fire on it
    // constantly. Left is checked first only because a tie is impossible — one keyboard cannot
    // produce two second-taps on the same tick without the player holding both keys, in which
    // case `move_x` is 0 and `vector::gas` refuses the flip anyway.
    let flip_window = data.game.vector.flip_double_tap_window_ticks;
    let a_down = keys.pressed(KeyCode::KeyA);
    let d_down = keys.pressed(KeyCode::KeyD);
    let flip_left = flips[0].feed(a_down, false, tick.0, flip_window);
    let flip_right = flips[1].feed(d_down, false, tick.0, flip_window);
    // 🔴 **A press on one side cancels a pending arm on the other**, and a test found this
    // missing rather than a design meeting. `tests/input.rs::f009_a_left_then_a_right_is_not_a_
    // flip` played `A · D · A · D` at two-tick spacing and got TWO flips: the third key is an
    // `A`, the first key was an `A`, and four ticks is well inside the 18-tick window — so two
    // independent detectors happily called it a double tap. That is a player **changing
    // direction**, which is what `A`/`D` are for in a swing (`docs/NEXT.md` §1a), and it would
    // have billed `gas_flip` for ordinary steering.
    //
    // Disarmed **after** the feed, never before: doing it first would eat the second tap of an
    // honest `A · nothing · A` on any tick the other key happened to be held.
    if d_down {
        flips[0].disarm();
    }
    if a_down {
        flips[1].disarm();
    }

    let mut t = Buttons::NONE;
    // The first tap of a dodge is **still a jump**, and so is the second. Nothing here consumes
    // `Space` — a dodge on the ground is a jump that then throws you, and swallowing the jump
    // would make the move feel like it ate an input. `F-008` adds a button, it does not steal
    // one.
    t.set(Buttons::JUMP, jump);
    t.set(Buttons::BOOST, keys.pressed(b.boost));
    // 🔴 **`Ctrl` and NOTHING else.** `S` was a second binding for `REEL_IN` here between
    // 2026-08-12 and 2026-08-13, and it was a **misreading**: *„mit s »spannt« man nur das
    // seil!"* was taken to mean "reel in", but **„spannen" is keeping a rope taut, not hauling
    // it in**. The user played it and said so in one sentence (`docs/NEXT.md` §1A, req 7):
    // *„aktuell wenn ich seil spanne und s drücke werde ich stark zum seil gezogen! das soll
    // nicht sein!"* — measured, a held `S` closed **56 m** in 120 ticks
    // (`tests/input.rs::r7_s_held_on_a_taut_rope_does_not_close_the_distance`), because
    // `reel_speed_m_s` is 28 and 120 ticks are 2 s.
    //
    // So `S` is nothing but the backwards axis again (the `forward` line below), and the rope
    // stays taut because the rope constraint keeps it taut — not because a key asks for it.
    t.set(Buttons::REEL_IN, keys.pressed(b.reel_in));
    // **`C` stays, and it is not a leftover.** Two routes to one bit, exactly like `F` next to
    // the left mouse button above:
    //   - a double-tap is a **gesture**, and the bible asks for accessibility (3.5) — a player
    //     who cannot tap twice inside 0.3 s must still be able to dodge, and a gesture is not
    //     rebindable in an options menu the way a key is;
    //   - `C` is what survives when the bindings become a RON file; the double-tap is what
    //     survives as the *feel* the user asked for. Neither replaces the other.
    // Both arrive as **one tick** of `DODGE` — see [`DodgeTap`] for why a held `C` may not.
    t.set(Buttons::DODGE, dodge);
    // `F-009`. **One bit and no side**, because the side is `move_x` on this very tick: the key
    // that produced the gesture is down, so `move_x` is -1 or +1 and `vector::dodge` reads it.
    // Two bits saying what one already says is two bits that can disagree.
    //
    // Nothing here consumes `A`/`D` — the strafe axis below is written from the same keys, the
    // way the first tap of a dodge is still a jump. A flip adds a verb, it does not steal one.
    t.set(Buttons::FLIP, flip_left || flip_right);
    // The ropes are on the keyboard and the blades are on the mouse (user, 2026-08-10, after
    // the first time a human played this: the ropes have to be **steerable**). A hand can hold
    // `Q` and `E` and still aim; it cannot hold both mouse buttons and still aim. `MARK` had to
    // move off `Q` for that and went to `Tab`, which nothing else uses.
    t.set(Buttons::MARK, keys.pressed(b.mark));
    // **The rope triggers go through [`HookLatch`]** (user, 2026-09-01: *„mach dass q und e
    // toggle sind und nicht hold"*). Under `HookFire::Toggle` a tap latches the held bit on;
    // a later tap on the latched arm is a **re-fire** (user, 2026-09-01, second note: *„und
    // später nochmal e drücke soll das seil weg "neu" raus gehen und toggeln!"*): its press
    // edge drops the bit — the old rope releases in that very tick — and its key-up, if it
    // comes inside [`HOOK_TAP_MAX_TICKS`], raises the bit again, which is the fresh fire.
    // A press held LONGER than the boundary adds no new rope: that is the pure release, one
    // gesture away on the same key. The first press of a rope still releases on a long
    // key-up as before, which is what keeps every `hook <side> <seconds>` line in `scripts/`
    // meaning what it always meant — a sub-boundary tap anchors and stays, a long hold is
    // released on key-up. Under `Hold` the latch is a wire: bit = key, bit for bit the
    // pre-2026-09-01 behaviour.
    //
    // The `arm_is_out` argument is what un-latches after a **self**-release: a missed tap, a
    // vanished carrier, an overextended rope. The `anchored` argument is §5E-c's re-fire
    // gate: only a press on an ANCHORED arm arms the key-up re-fire. Both read last tick's
    // state — see [`HookLatch::feed`] for why that is exactly right and not a race.
    let toggle = settings.hook_fire == HookFire::Toggle;
    let arm = hooks.iter().next().copied().unwrap_or_default().arms;
    let out = |s: HookState| matches!(s, HookState::Flying { .. } | HookState::Anchored { .. });
    let anchored = |s: HookState| matches!(s, HookState::Anchored { .. });
    t.set(
        Buttons::HOOK_LEFT,
        hook_latches[0].feed(
            keys.pressed(b.hook_left),
            out(arm[0].state),
            anchored(arm[0].state),
            tick.0,
            toggle,
        ),
    );
    t.set(
        Buttons::HOOK_RIGHT,
        hook_latches[1].feed(
            keys.pressed(b.hook_right),
            out(arm[1].state),
            anchored(arm[1].state),
            tick.0,
            toggle,
        ),
    );
    // The keyboard slash stays next to the left mouse button — a second binding, not a
    // leftover. It is the only route to `SLASH_LEFT` that `debug::script::parse_key` can
    // reach, because a script has no way to press a mouse button except `slash left|right`.
    t.set(
        Buttons::SLASH_LEFT,
        mouse_buttons.pressed(MouseButton::Left) || keys.pressed(b.slash_left),
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

    /// **Forget the tap that is waiting for a partner.**
    ///
    /// `F-009` needs it and `F-008` does not: the dash has one detector and nothing can
    /// interrupt it, while the flip has **two** — one for `A`, one for `D` — and a press on
    /// either side has to cancel the other's pending arm, or `A · D · A` reads as a double tap
    /// of `A`. See the call site in [`read_input`] for the test that found it.
    ///
    /// It does **not** touch `space_down`/`dodge_key_down`: those are the previous tick's key
    /// state, i.e. a fact about the keyboard, and clearing them would manufacture an edge on
    /// the next tick out of a key that never went up.
    pub fn disarm(&mut self) {
        self.armed_at = None;
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
/// ## It gathers the **wheel** too, and for the identical reason
///
/// `AccumulatedMouseScroll` is assigned in `PreUpdate` exactly like the motion above
/// (`bevy_input-0.19.0/src/mouse.rs:272-286` — `= delta`, not `+=`), so a wheel read straight
/// out of `FixedPreUpdate` would drop notches on frames without a fixed step and count them
/// twice on catch-up frames. The name of this system stays as it is because
/// `src/net/mod.rs` registers it and that file is not this job's; what it does is in this
/// paragraph and in `tests/input.rs::f023_the_wheel_notches_survive_any_frame_rate`.
pub fn gather_mouse_motion(
    motion: Res<AccumulatedMouseMotion>,
    mut pending: ResMut<MouseSinceTick>,
) {
    pending.delta += motion.delta;
}

/// The longest press that still counts as a **tap** under [`HookFire::Toggle`], in ticks.
///
/// Since 2026-09-01 (§5E-c) it is also the boundary between a re-tap's two verbs on a
/// latched arm: key-up inside it re-fires, key-up past it is the pure release.
///
/// 18 ticks = 0.3 s at 60 Hz. A UI constant like `PIXELS_PER_NOTCH`, not a `game.ron` value:
/// it is a property of the control, and a tuning file that carries it invites somebody to
/// balance the game with it. The number sits between the two populations that exist:
/// a human tap is 3–9 ticks, and the shortest **hold** any evidence script performs on an
/// anchor it must let go of again is 0.52 s = 31 ticks (`scripts/f025-chain.txt`). The 0.2 s
/// taps in `scripts/f028-why.txt` are under the boundary on purpose — three are misses (the
/// latch clears itself off the idle arm) and the fourth staying anchored is what a tap now
/// means.
pub const HOOK_TAP_MAX_TICKS: u64 = 18;

/// One rope trigger's toggle state (`F-172`'s neighbour — user, 2026-09-01: *„mach dass q und
/// e toggle sind und nicht hold (oder in einstellungen einstellbar)"*).
///
/// **This is the whole toggle.** It lives on the keyboard side of the `Intent`, exactly like
/// [`DodgeTap`]: the simulation keeps reading a plain held bit (`vector::hook`'s `held` /
/// `just_pressed` pair), never learns which mode produced it, and a network client or a
/// script driver is unaffected by construction. The rules, each one a test below:
///
/// - **Hold mode is a wire.** `bit = key`, bit for bit the old behaviour.
/// - **A tap latches.** Press edge with the latch down: latch up (the rising bit is
///   `vector::hook`'s fire edge).
/// - **A tap on a latched arm is a RE-FIRE** (user, 2026-09-01: *„und später nochmal e drücke
///   soll das seil weg "neu" raus gehen und toggeln!"*). Its press edge drops the bit — the
///   falling edge is the release of the old rope, in the tick of the press, exactly his
///   *„das seil weg"* — and its key-up **inside** [`HOOK_TAP_MAX_TICKS`] raises the bit
///   again: a fresh fire edge at the current aim, his *„neu raus"*. The rising edge lands
///   while the arm is `Retracting` (or already `Idle` on a short rope), and both fire on the
///   edge since `F-002` — the retract is not a lockout, so no tick is swallowed. The re-fire
///   arms only over an arm that is really **anchored**; a press over a rope still in flight
///   is the plain toggle-off it always was.
/// - **A re-tap held past the boundary is the PURE release.** The rope has been gone since
///   the press; a key-up after more than [`HOOK_TAP_MAX_TICKS`] adds nothing. One key, two
///   verbs, and the tap is the common one. (⚠️ 2026-09-01, second half of §5E-c: until then
///   the second tap was a plain toggle-off. `docs/QUESTIONS.md` Q-095 records the one
///   reading this implementation chose: the release half happens at the press edge, not at
///   key-up, because the press cannot yet know whether it is a tap or a hold and holding the
///   OLD rope through the decision would re-fire it on every long press.)
/// - **A long press on a FRESH rope is still a hold.** First press: key-up after more than
///   [`HOOK_TAP_MAX_TICKS`] drops the latch — so a Hold-trained hand, and every
///   `hook <side> <seconds>` script line, gets release-on-key-up unchanged.
/// - **A self-release un-latches — but only once the key is up.** The latch is up, the key is
///   not down, the arm is neither flying nor anchored, and the tap is at least one tick old:
///   the arm let go on its own (a miss, `BodyGone`, `Overextended`), and a latch left up
///   would swallow the next tap as a toggle-off of a rope that no longer exists. While the
///   key IS down the bit stays up whatever the arm does — a held trigger means a held
///   trigger, which is what the `bindings_*` seam tests pin. `arm_is_out` is **last tick's**
///   state, because `read_input` runs in `FixedPreUpdate` and `vector::hook` in
///   `FixedUpdate` — which is exactly why the `tick > down_at` guard exists: on the tap's own
///   tick the stale state may not clear the latch that tap just set.
#[derive(Default)]
pub struct HookLatch {
    /// The key at the previous tick — the edge memory, like [`DodgeTap::space_down`].
    key_down: bool,
    /// Whether the held bit is currently latched on.
    latched: bool,
    /// The tick of the press that set the latch. Meaningless while un-latched.
    down_at: u64,
    /// The press tick of a **re-tap** — a press that found the latch already up. `Some`
    /// while that press is still held: its release half has already gone out (the falling
    /// edge, at the press), and the key-up decides only whether a fresh fire follows (a
    /// tap, inside [`HOOK_TAP_MAX_TICKS`]) or nothing does (a hold — the pure release).
    retap_at: Option<u64>,
}

impl HookLatch {
    /// The `HOOK_*` bit for this tick.
    pub fn feed(
        &mut self,
        key: bool,
        arm_is_out: bool,
        arm_is_anchored: bool,
        tick: u64,
        toggle: bool,
    ) -> bool {
        let pressed = key && !self.key_down;
        let released = !key && self.key_down;
        self.key_down = key;
        if !toggle {
            // Hold: the latch mirrors the key, so switching modes mid-game starts clean.
            self.latched = key;
            self.retap_at = None;
            return key;
        }
        // Only once the key is UP: while it is held, the bit staying up is exactly what a
        // held trigger means (and what every `bindings_*` seam test pins) — the stale case
        // this exists for is a latch left up over an arm that let go while nobody was
        // touching the key.
        if self.latched && !key && !arm_is_out && tick > self.down_at {
            self.latched = false;
        }
        if pressed {
            if self.latched {
                // The re-tap (2026-09-01): *„das seil weg"* — the falling edge NOW sends
                // the anchored arm to `Retracting` in this very tick. Whether *„neu raus"*
                // follows is the key-up's decision below, because only the key-up can tell
                // a tap from a hold — and it is armed only over an arm that is really
                // ANCHORED: a press over a rope still in flight is the plain toggle-off it
                // always was, so trigger-spam cannot chain rope after rope out of one aim.
                self.latched = false;
                self.retap_at = arm_is_anchored.then_some(tick);
            } else {
                self.latched = true;
                self.down_at = tick;
            }
        }
        if released {
            if let Some(t0) = self.retap_at.take() {
                if tick.saturating_sub(t0) <= HOOK_TAP_MAX_TICKS {
                    // *„neu raus"*: the rising edge, on the key-up tick. The arm is
                    // `Retracting` (or `Idle` again on a short rope) and both fire on the
                    // edge — `vector::hook` decision 1, `F-002`: the retract is not a
                    // lockout, so the fresh flight starts in this same tick.
                    self.latched = true;
                    self.down_at = t0;
                }
                // Past the boundary: the pure release. The rope has been gone since the
                // press edge; a long hold adds nothing new.
            } else if tick.saturating_sub(self.down_at) > HOOK_TAP_MAX_TICKS {
                self.latched = false;
            }
        }
        self.latched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives one latch over a keyboard trace. The second element says whether the arm is
    /// out (flying or anchored) at tick `t`, the third whether it is ANCHORED — the test's
    /// stand-in for what `vector::hook` did last tick. In these traces an arm that is out is
    /// anchored unless a test says otherwise; the flying case has its own test below.
    fn run(latch: &mut HookLatch, trace: &[(bool, bool)], toggle: bool) -> Vec<bool> {
        trace
            .iter()
            .enumerate()
            .map(|(t, (key, arm_out))| latch.feed(*key, *arm_out, *arm_out, t as u64, toggle))
            .collect()
    }

    #[test]
    fn in_hold_mode_the_bit_is_the_key_bit_for_bit() {
        let mut latch = HookLatch::default();
        let trace = [(true, false), (true, true), (false, true), (true, true), (false, false)];
        let bits = run(&mut latch, &trace, false);
        assert_eq!(bits, vec![true, true, false, true, false]);
    }

    /// The 2026-09-01 re-fire (`docs/NEXT.md` §5E-c: *„das seil weg, neu raus, und toggeln"*).
    /// Until that day the second tap was a plain toggle-off; now its press releases and its
    /// short key-up fires fresh.
    ///
    /// What the code reads: the key edges, the tick distance to `HOOK_TAP_MAX_TICKS`, last
    /// tick's arm state. What this fixture varies: all three. The arm bit is held `true`
    /// through the re-tap on purpose — the stale-clear must NOT eat the latch the key-up
    /// just set.
    #[test]
    fn a_tap_on_a_latched_arm_releases_at_the_press_and_refires_on_the_short_key_up() {
        let mut latch = HookLatch::default();
        // Tap: 3 ticks down, then up; the arm flies from tick 1 and anchors.
        let mut trace = vec![(true, false), (true, true), (true, true)];
        trace.extend(std::iter::repeat_n((false, true), 30));
        // The re-tap: two ticks down, then up, then two quiet ticks.
        trace.push((true, true)); // t33 — the press: release of the old rope
        trace.push((true, true)); // t34 — still held, still down
        trace.push((false, true)); // t35 — the key-up: the fresh fire
        trace.push((false, true)); // t36 — the new rope holds
        let bits = run(&mut latch, &trace, true);
        assert!(bits[..33].iter().all(|b| *b), "the latch has to hold through the whole gap");
        assert!(!bits[33], "the re-tap's press has to drop the bit — that is the release");
        assert!(!bits[34], "and it stays down while the key is held");
        assert!(bits[35], "the key-up inside HOOK_TAP_MAX_TICKS has to re-raise the bit — the fresh fire");
        assert!(bits[36], "and the new rope is latched exactly like a first tap's");
    }

    /// The other half of the same gesture: held past `HOOK_TAP_MAX_TICKS`, the re-tap is the
    /// PURE release — the rope went at the press and the key-up adds nothing.
    #[test]
    fn a_retap_held_past_the_boundary_is_a_pure_release_and_nothing_refires() {
        let mut latch = HookLatch::default();
        let mut trace = vec![(true, false), (false, true), (false, true)];
        // The re-tap press at t3, held for HOOK_TAP_MAX_TICKS + 3 ticks.
        trace.push((true, true));
        trace.extend(std::iter::repeat_n((true, false), HOOK_TAP_MAX_TICKS as usize + 2));
        trace.push((false, false)); // the late key-up
        trace.push((false, false));
        let bits = run(&mut latch, &trace, true);
        assert!(bits[2], "the first tap latched");
        assert!(!bits[3], "the re-tap's press releases the rope immediately");
        let up = trace.len() - 2;
        assert!(!bits[up], "a key-up past HOOK_TAP_MAX_TICKS must NOT fire fresh");
        assert!(!bits[up + 1], "and nothing comes back afterwards");
    }

    /// The re-fire gate: a second press over a rope that is OUT but not yet ANCHORED is the
    /// plain toggle-off it always was — the quick key-up fires nothing.
    #[test]
    fn a_retap_over_a_still_flying_rope_is_a_plain_toggle_off_not_a_refire() {
        let mut latch = HookLatch::default();
        // (key, out, anchored): the rope flies from t1 and never anchors before the re-tap.
        let trace = [
            (true, false, false),
            (false, true, false),
            (false, true, false),
            (true, true, false),  // the second press, mid-flight
            (false, true, false), // the quick key-up
            (false, true, false),
        ];
        let bits: Vec<bool> = trace
            .iter()
            .enumerate()
            .map(|(t, (k, o, a))| latch.feed(*k, *o, *a, t as u64, true))
            .collect();
        assert!(bits[2], "the first tap latched");
        assert!(!bits[3], "the press drops the bit — the toggle-off");
        assert!(!bits[4], "the quick key-up must NOT re-fire: this arm never anchored");
        assert!(!bits[5]);
    }

    /// A re-fire that finds nothing must not leave the latch up over an idle arm — the
    /// stale-clear rule catches it one tick later and the next tap is a fresh fire.
    #[test]
    fn a_refire_that_misses_clears_itself_off_the_idle_arm() {
        let mut latch = HookLatch::default();
        let trace = vec![
            (true, false),  // t0 tap: fire
            (false, true),  // t1 anchored
            (true, true),   // t2 re-tap press: release
            (false, true),  // t3 key-up: the re-fire edge (arm bit is last tick's, still out)
            (false, false), // t4 the re-fire missed — the arm never came out again
            (false, false), // t5
            (true, false),  // t6 the next tap
        ];
        let bits = run(&mut latch, &trace, true);
        assert!(bits[3], "the short key-up re-fires");
        assert!(!bits[4], "an arm that stayed idle clears the latch the re-fire left up");
        assert!(bits[6], "and the next tap is a fresh fire, not a toggle of nothing");
    }

    #[test]
    fn a_long_press_still_releases_on_key_up() {
        let mut latch = HookLatch::default();
        let mut trace = vec![(true, false)];
        trace.extend(std::iter::repeat_n((true, true), HOOK_TAP_MAX_TICKS as usize + 5));
        trace.push((false, true));
        trace.push((false, true));
        let bits = run(&mut latch, &trace, true);
        assert!(bits[HOOK_TAP_MAX_TICKS as usize], "held: the bit stays while the key is down");
        assert!(
            !bits[trace.len() - 2],
            "key-up after more than HOOK_TAP_MAX_TICKS has to release, or every \
             `hook <side> <seconds>` script line changes its meaning"
        );
    }

    #[test]
    fn a_missed_tap_unlatches_so_the_next_tap_fires_instead_of_toggling_nothing() {
        let mut latch = HookLatch::default();
        // Tap at nothing: the arm never leaves Idle.
        let mut trace = vec![(true, false), (true, false), (false, false), (false, false)];
        // The next tap must be a FIRE (rising edge), not a toggle-off.
        trace.push((true, false));
        let bits = run(&mut latch, &trace, true);
        assert!(bits[0], "the tap itself raises the bit — that edge is the fire");
        assert!(!bits[3], "an arm that stayed idle clears the latch");
        assert!(bits[4], "the next tap is a fresh fire");
    }

    #[test]
    fn the_arms_own_release_unlatches_too() {
        let mut latch = HookLatch::default();
        // Tap, anchored for a while, then the rope overextends and the arm retracts.
        let mut trace = vec![(true, false), (false, true), (false, true), (false, true)];
        trace.push((false, false)); // vector::hook released the arm on its own
        trace.push((false, false));
        trace.push((true, false)); // the next tap
        let bits = run(&mut latch, &trace, true);
        assert!(bits[3]);
        assert!(!bits[5], "the latch has to follow the arm's own release");
        assert!(bits[6], "and the next tap fires");
    }
}
