# bevy — what the installed version 0.19.0 is really called, and what it costs

Updated: 2026-08-09 · Stage: 🟨 (every line here was looked up in the installed source; none of
it has been run)

> ⚠️ **Bevy's API turns hard between minor versions.** Look it up in the docs of the
> **installed** version (`cargo doc --open -p bevy`, or straight in
> `~/.cargo/registry/src/*/bevy_*-0.19.0/src/`), not from memory and not from blog posts two
> versions old. **Check, do not assume** (`prompts/init.md` §3).

Everything here is checked against `~/.cargo/registry/src/index.crates.io-*/bevy_*-0.19.0/`,
with file and line. What carries no evidence is not in here.

## Names that have moved recently

| Item | in 0.19.0 | Evidence |
|---|---|---|
| Buffered messages | **`Message`**, not `Event` | `bevy_ecs/src/message/mod.rs:100` (`pub trait Message`) |
| Registering | **`App::add_message::<M>()`** | `bevy_app/src/app.rs:435` |
| Writing / reading | **`MessageWriter<M>` / `MessageReader<M>`** | `bevy_ecs/src/message/message_writer.rs:62`, `message_reader.rs:34` |
| Delta in seconds | **`time.delta_secs()`** | `bevy_time/src/time.rs:283` |
| Fixed step size | **`Time::<Fixed>::from_hz(f64)`** | `bevy_time/src/fixed.rs:105` |
| Fixed schedules | `FixedPreUpdate`, `FixedUpdate`, `FixedPostUpdate` | `bevy_app/src/main_schedule.rs:118/133/141` |
| Anything visible | **`Mesh3d(pub Handle<Mesh>)`** and **`MeshMaterial3d<M>(pub Handle<M>)`** — single components, no bundles | `bevy_mesh/src/components.rs:102`, `bevy_pbr/src/mesh_material.rs:41` |
| Camera | **`Camera3d`** with `#[require(Camera, Projection)]` | `bevy_camera/src/components.rs:25` |
| Text | **`Text(pub String)`**, plus `TextFont`, `TextColor(pub Color)` | `bevy_ui/src/widget/text.rs:111`, `bevy_text/src/text.rs:376/1066` |
| Return value of `App::run()` | **`AppExit`** (`Success` \| `Error(NonZero<u8>)`) | `bevy_app/src/app.rs:192`, `1568` |

## The trap that would really have caught us: `AmbientLight` is a **component**

In 0.19 `AmbientLight` is **no longer a `Resource`** but a component with `#[require(Camera)]` —
it belongs on the **camera** and overrides the default `GlobalAmbientLight` there
(`bevy_light/src/ambient_light.rs:9-12`). Write `insert_resource` and you get no compiler error
where you expect one, and a scene that is simply darker than intended.

`DirectionalLight` for its part demands `Transform` and `Visibility` via `#[require(..)]`
(`bevy_light/src/directional_light.rs:67-73`) — they no longer have to be handed in by hand.

## Starting without a graphics session

Both parts together, otherwise wgpu goes looking for an adapter anyway:

| Part | exactly |
|---|---|
| no window | `WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, .. }` — `bevy_window/src/lib.rs:74`, `158-175` |
| no renderer | `RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })), .. }` — `bevy_render/src/settings.rs:41`, `223-228` |
| the loop | `ScheduleRunnerPlugin::run_loop(Duration)` or `::run_once()` — `bevy_app/src/schedule_runner.rs:57`, `64` |

⚠️ **`RenderCreation::Automatic` takes a `Box<WgpuSettings>`**, not `WgpuSettings`
(`settings.rs:227`). That is exactly the kind of detail you write down wrong from memory.

`ExitCondition::DontExit` is needed **because without a window the app otherwise shuts down
immediately**: the default `OnAllClosed` sees zero windows and goes down
(`bevy_window/src/lib.rs:72-74`).

### Three traps at exactly this spot

1. **`primary_window: None` alone is not enough.** `WinitPlugin::build` builds the event loop
   **unconditionally** and with `.expect("Failed to build event loop")`
   (`bevy_winit/src/lib.rs:90-128`); on Linux, with no `WAYLAND_DISPLAY` and no `DISPLAY`, winit
   returns an error (`winit-0.30.13/src/platform_impl/linux/mod.rs:754-766`). So it panics
   **before a single system runs**. The only way out: `disable::<WinitPlugin>()`.
2. **`ScheduleRunnerPlugin` is NOT in `DefaultPlugins`** as long as the feature `bevy_window` is
   on (`bevy_internal/src/default_plugins.rs:19-20`). After `disable::<WinitPlugin>()`
   **nobody** drives the app otherwise: `App::run()` falls back to `run_once` and does exactly
   **one** update (`bevy_app/src/app.rs:159`, `1539-1550`). So `.add(...)` — and **not**
   `.set(...)`, because `set` and `disable` **panic** when the plugin is not in the group at all
   (`bevy_app/src/plugin_group.rs:312-319`, `496-508`).
3. **`disable::<WinitPlugin>()` itself needs a `cfg` guard.** Build with
   `--no-default-features` and there is neither the module path `bevy::winit` nor the entry in
   the group — the call would be a compiler error or a panic. That is why it sits in
   `src/lib.rs` behind `#[cfg(any(feature = "x11", feature = "wayland"))]`.

### And a field that only looks like a switch

`DirectionalLight::contact_shadows_enabled` **on its own does nothing**. Contact shadows are a
screen-space technique and additionally need a `ContactShadows` component on the camera. The
switch that really costs compute time is called `shadow_maps_enabled`.

## The script driver writes into **real** input

`ButtonInput::press` and `::release` are **public** (`bevy_input/src/button_input.rs:149`,
`172`). That lets a `--script` run write into the same input a human triggers — **no second,
false way to play** (`prompts/init.md` §12b). For the mouse there is
`AccumulatedMouseMotion { delta: Vec2 }` (`bevy_input/src/mouse.rs:218-221`), which sums up the
motion between two frames.

## What the build cost on this machine

`bevy = "0.19.0"` with the `default` features does **not** build on machine A. The umbrella
features `2d`, `3d` and `ui` all pull in `default_platform`, and `x11`, `wayland` and
`bevy_gilrs` are hard-wired inside it (`bevy-0.19.0/Cargo.toml:2736`, `2756-2768`).

| Attempt | Result | Time `[debian]` |
|---|---|---|
| `bevy = "0.19.0"` (default) | aborts in `wayland-sys`: `wayland-client.pc` missing | 9m22s |
| feature list by hand, but with `audio` | aborts in `alsa-sys`: `alsa.pc` missing | 13m40s |

On this machine there are exactly three `.pc` files (`openssl`, `libuv`, `libcrypt`) and no
passwordless `sudo`. The solution stands in `Cargo.toml`: `default-features = false`, a base
list **without anything that needs a system library at build time**, and our own features on top
of it — `x11` (the default), `wayland`, `audio`. `cargo build --no-default-features` needs
nothing at all.

**Remember:** with Bevy the question is never "which feature do I want?" but "what does this
feature pull in?". `cargo tree -e features` answers it, a blog post does not.

## The traps from `prompts/init.md` §3, still valid unchanged

- **`add_plugins((..))` takes at most ~15 elements per tuple**, a system at most ~16 parameters.
  Both hit you as an unreadable trait error. Solution: nest them (`((A, B), C, …)`), or bundle
  the parameters into a `SystemParam` struct.
- **Commands are delayed.** What you spawn this frame exists only at the end of the frame. A
  test or a script that spawns and checks in the same breath checks into the void — which is why
  in the driver a `wait` follows every `spawn`.
- **`cargo run`, NEVER `./target/debug/<name>`.** The bare binary looks for `assets/` relative to
  the working directory and finds nothing: empty world, no error message, looks exactly like a
  render bug. (`src/data/mod.rs` catches precisely this and says so out loud.)
- **Audio:** Bevy's default decoder is Vorbis and nothing else. Use `.wav` and you need the
  feature `wav` — otherwise every sound loads without an error and plays **silence**.
- **RON has no `include`.** When a data file gets too large you split it **in Rust** (read
  several files and join them), not in RON.
- **Without `[profile.dev.package."*"] opt-level = 3` a debug build is unplayable.** Bevy itself
  does batching, transform propagation and rendering; our own crate stays at `opt-level = 1` and
  cheap to compile.

Related: [`environment.md`](environment.md) · [`performance.md`](performance.md) ·
[`workflow.md`](workflow.md) · [`../architecture.md`](../architecture.md) ·
[`../conventions.md`](../conventions.md)
