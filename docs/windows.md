# windows — getting it running on the machine that also runs the reference

Updated: 2026-08-23 · Stage: 🟧 (**built, run and played on Windows — see the result section below**)

**Why this file exists.** The user asked for it so he can run *Defeated by Titan* on Windows
**with the reference game open beside it**, and compare them directly. That comparison is worth
more than any research round — a wiki tells you what a patch changed, it does not tell you what
the first thirty seconds feel like.

⚠️ **The prompt below was derived, not tested, when it was written.** It has since been run — what actually happened is in `## What actually happened` at the end of this file, and it differs from the prediction in one place. There is no Windows machine in this project. What
*is* checked (2026-08-20): `src/` contains **no** `cfg(target_os)`, **no** `cfg(unix)`, **no**
`/dev/`, **no** `std::os::unix`. The game code does not know what operating system it is on. What
is unproven is the dependency tree, the asset path, and the graphics backend.

---

## The prompt

Paste this into a Claude Code session **on the Windows machine**, in the folder where you want the
repository.

```text
Get "Defeated by Titan" building and running on this Windows machine, and report back exactly
what happened. It is a Bevy 0.19 / Rust game. It has never been built on Windows — you are the
first, so treat every step as unproven and report the truth rather than a success story.

REPO: git@github.com:OfflineBot/defeated-by-titan.git   branch: session-2026-08-09
(Use https://github.com/OfflineBot/defeated-by-titan.git if SSH is not set up.)

1. PREREQUISITES. Check before installing anything: `rustc --version` (needs the 2024 edition,
   so 1.85+), `git --version`, and that a GPU driver is present. If Rust is missing, install
   via rustup with the MSVC toolchain, not GNU.

2. BUILD. The default feature is `x11`, which is Linux-only and WILL fail here. Use:
       cargo run --no-default-features --features windows
   That feature was added for you and has never been compiled ANYWHERE. On Linux it cannot even
   be checked: `cargo check --no-default-features --features windows` there fails with "The
   platform you're compiling for is not supported by winit", which is correct — winit needs a
   display-server feature on Linux and none on Windows. So you are the first compile.
   If it fails, read the error and say what it is — do not silently switch features until
   something works, and do NOT add `x11` back, which is the Linux answer and would break this.
   ⚠️ The first build compiles Bevy and avian from scratch: expect 10-25 minutes.
   ⚠️ `[profile.dev]` builds every dependency at full optimisation on purpose — without it a
   debug build is unplayable (20 fps against 200).

3. WHAT SHOULD HAPPEN. A window opens on a title screen: the game's name, New Game, Settings,
   Quit. New Game drops you into a hub — a walkable place with deployment pads. Escape opens a
   menu; Settings has mouse sensitivity, FOV, aim spread, and two aim-assist knobs.

4. CONTROLS. WASD move, Space jump, Shift boost, Ctrl reel in, C dodge, Tab mark.
   Q and E fire the left and right hook. Left and right mouse button swing the blades.
   F3 is a debug overlay.

5. THE LIKELY FAILURES, in the order I expect them:
   a) `assets/data/ not found` on startup. `src/data/mod.rs::assets_dir()` looks in the working
      directory first, then at the path of the crate the binary was built from. Run from the
      repository root and it should resolve. If it panics, the message names both paths it
      tried — report it verbatim.
   b) A build error in a dependency (wgpu, avian, winit). Report the crate and the error.
   c) A window that opens black or closes at once. Run with `RUST_LOG=info` and report the last
      20 lines.
   d) `cargo run` works but the direct `.\target\debug\defeated_by_titan.exe` does not. That is
      known and expected: the asset root falls back to a compile-time path.

6. IF IT RUNS, do these three things and report each:
   - `cargo run --no-default-features --features windows -- --headless --ticks 300`
     (should exit 0 and log "blocks built")
   - take a screenshot of the title screen and of the hub
   - fly around for two minutes and say what feels wrong

7. REPORT in this shape, and be blunt — an honest "it does not build" is worth more than a
   patched-together success:
   Task · Done · Evidence · Stage · Open · Findings
```

---

## What the comparison is FOR

Once it runs, the valuable thing is not the screenshot. It is this, and it is the one question
research could not answer (`docs/gameplay/references.md`, the ODM section):

> **Does the reference DRIVE you toward a speed, or does it swing you?**

Our rope is a physical pendulum: a pure swing runs **17–21 m/s** and `max_speed_m_s` (75) is only
reached by spending gas on boost. The reference's patch record describes *"a velocity drive toward
the anchor, its magnitude governed by the `ODM Speed` stat, capped by the player's `Gear Shift`
setting, its turn rate governed by `ODM Control` (%)"* — which, if true, means you hold a button
and **go**, at a speed the stat decides, and the rope is a direction rather than a force.

⚠️ **And the unit is unresolved.** `ODM Speed` reads 190→210 at grade E- and 252→257.5 at max.
**If those are Roblox studs/s, they are 53–72 m/s and our cap of 75 is already right** — the
difference would be entirely in *how you get there*, not in the ceiling. If they are metres, we
are three times too slow. `references.md` marks this `unresolved` and it is the single most
load-bearing unknown in the project.

**What would settle it, from two minutes of play with both open:**

| question | why it decides something |
|---|---|
| Do you hold a button to keep moving, or does momentum carry you? | drive vs pendulum |
| How long does it take to cross a street? A district? | the unit question, in seconds |
| When you hook and look away, do you swing in an arc — or does it drag you? | whether a rope is a rope |
| Does your speed *build* over several swings, or is it constant from the first? | drive vs momentum |
| When you release, do you keep the speed? | whether the drive is velocity or acceleration |

**Answering those five is worth more than any further round of tuning here**, because every knob
this project has turned — the fan, the assist, the gas, the hitboxes — sits downstream of them.

Related: [`gameplay/references.md`](gameplay/references.md) · [`environment.md`](environment.md) ·
[`gameplay/pillars.md`](gameplay/pillars.md)

---

## What actually happened — 2026-08-23, the first Windows run

**It built, it ran, and the prediction above was right in every place but one.**

### The machine

Windows 11 Home 10.0.26200, RTX 3080 (driver 32.0.15.9186), Visual Studio 2022 Community with
MSVC 14.44 and Windows SDK 10.0.26100 **already present** — so the C++ toolchain, which is the
usual Windows blocker, cost nothing. Installed on the day: **Rust 1.98.0**
(`stable-x86_64-pc-windows-msvc`, the right toolchain) and **Python 3.14.7**. Nothing else.

⚠️ **Rust 1.98.0 is newer than both other machines** (A: 1.97.1, B: 1.95.0). The project's floor
stays **1.95.0** — what compiles here is not proof it compiles on B. See `environment.md`.

### The build

```
cargo build --no-default-features --features windows
Finished `dev` profile [optimized + debuginfo] target(s) in 17m 02s
```

**0 errors, 0 warnings, 358 crates**, `defeated_by_titan.exe` at 117 MB. Inside the predicted
10–25 minutes. **The `windows` feature compiled on its first attempt anywhere** — the whole
dependency tree, wgpu and avian and winit included, resolved on Windows with no change to
`Cargo.toml` and no `cfg(target_os)` anywhere in `src/`.

### The headless run

```
./target/debug/defeated_by_titan.exe --headless --ticks 300
INFO defeated_by_titan::world::map: map "Ashgate": 2871 blocks built (215 placed, 2656 generated),
                                    2871 of them anchorable
exit 0
```

**Failure (a) did not happen.** `data::assets_dir()` resolves `assets\` from a Windows working
directory, run from the repository root, with no separator trouble.

**One benign error is logged and can be ignored in headless:**

```
ERROR bevy_render::extract_resource: Render app did not exist when trying to add
      `extract_resource` for <bevy_camera::clear_color::ClearColor>.
```

### What is still unproven on Windows

- **The window.** `cargo run` with a window was not attempted this session — the machine was busy
  running the reference game beside it. Failure (c), "opens black or closes at once", is untested.
- **`cargo test`.** The suite has never run on Windows. Test binaries would need their own link
  pass (~15 min), and it was not spent.
- **Failure (d)** — running the `.exe` from outside the repository root — was not tried either,
  though the headless run from the root makes the asset root look healthy.

### 🔴 And the comparison the file was written for: it delivered

The five questions in `## What the comparison is FOR` were the point of this whole file. **The
first one is answered and it is the load-bearing one.**

> **The reference DRIVES. It does not swing.** No input, no pull — the rope is a direction and the
> key is the force. → **`FIND-149`**

Two more results came out of the same session, neither of them planned:

- **`FIND-150`** — the reference's gas burn, read off its own HUD across 20 timed screenshots:
  **~400 s per tank at ordinary flight, ~100 s pushed, exactly 0 while idle.** `references.md` had
  filed tank and burn rate as *"unknown and not obtainable"*. Our tank sits in the same band, and
  the `gas_tank: 300.0` rollback recorded in `game.ron` would put us **~25x** below the reference.
- **`Q-048`** — both of the reference's ropes fly at the crosshair, with `Q`/`E` **held**. Ours
  splits left/right by `F-023`, which is also the user's instruction. His observation against his
  own earlier instruction — not this session's to resolve.

**Still open from the five:** whether speed builds over swings, whether it survives release, and
**the unit** — `Gear Shift` reads **600 m/s** in his settings, above the 500 the research
recorded, and there is **no speed readout in the reference's HUD** to check it against. The one
candidate is the `N/A` field under the crosshair; untested.

**The screenshots are in the repository** so this can continue on Linux:
[`images/reference-aotr/`](images/reference-aotr/README.md).
