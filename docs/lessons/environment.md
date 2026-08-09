# environment — two machines, four traps that really snapped shut here, and the error messages that mean something other than what they say

Updated: 2026-08-09 · Stage: 🟨 (the four traps under "What really happened here" did occur in
this repo and carry a file or a command as evidence — only trap 2 has a measured **time**; the
generic traps further down are taken from `prompts/init.md` §15 and have not been triggered in
this repo)

## The problem

The environment does not cost time because it is broken, but because its symptoms look like
program errors. A missing window reads like a bug. An N100 reads like a performance regression.
A full disk announces itself as a linker crash. Suspect the code at this point and you search in
the wrong place for hours.

**The first question of every session, before anything else:**

```bash
hostname          # 'debian' → headless (A) · 'offlinebot' → full graphics (B)
uname -r; nproc; echo "WAYLAND=$WAYLAND_DISPLAY DISPLAY=$DISPLAY"
df -h /home       # on B, before the first build
```

The measured values are in [`docs/environment.md`](../environment.md) — A measured, B so far
only taken over from `prompts/init.md` §14. What stands here is only what the differences cost.

| | **A — `debian`** | **B — `offlinebot`** |
|---|---|---|
| Desktop | **none** — no monitor, no Wayland/X | niri (Wayland), kitty, fish |
| CPU / GPU | Intel N100, 4 cores · UHD Graphics | Ryzen 7 5800X, 16 threads · RTX 3080 |
| A window | **not possible — and that is fine** | possible |
| Screenshot | only through offscreen rendering, if that works | `niri msg action screenshot-window` |
| Highest reachable stage | **🟨** | 🟧 |

---

## On A you work, you only check differently

No window is no reason to stop building. Fully possible:

| What | With what | Why it works without a screen |
|---|---|---|
| Logic tests | `cargo test` | Vector Gear math, hit zones, damage curves, RON validation, world generation as numbers, the domain test, `tests/multiplayer.rs` — all numbers |
| Script runs with `assert` | `cargo drive <script>` (alias for `cargo run -- --headless --script`) | **only if the `--headless` mode exists**: `primary_window: None`, a fixed tick, N ticks, and an exit code that says whether every `assert` held |
| Model chain | `blender --background` | `.py` → `.blend` → `.glb` and the structure test (empties, vertex colors, `metallicFactor`) need no display |
| Spreadsheet extraction | `python3 tools/features.py` | pure file work |
| Docs, cleanup, refactoring | editor | pure file work |

That a script run is checkable on **every** machine is the real reason for the `--headless` mode
in stage 1 — not convenience. It is the only bridge between A, B and a CI one day.

## What does NOT work on A — no excuses

| Prohibition | Why | What you do instead |
|---|---|---|
| **No image ⇒ no 🟧** | the ceiling is 🟨 with the note *"logic tested, pixels unseen — machine A"* | Do not round up, no "it surely looks right". Catch it up on B. |
| An offscreen PNG as evidence | Bevy can draw into a render target and Vulkan needs no display for that — but **claimed, it is worth nothing** | First prove that it really delivers an image on the N100, then use it as evidence |
| **No performance claim** | an N100 with integrated graphics and a 5800X with a 3080 are not a measurement series | **Every number in `STATUS.md`/`BUGS.md` carries `[debian]` or `[cachy]`.** A frame time without a machine is not a measurement |
| No `niri msg` | it does not exist on A | the whole screenshot section applies to B only |
| Reporting slow builds | 4 cores | That is **not a regression**, that is the N100 |

## On B, additionally

Full graphics means the full burden of proof: this is where the screenshots are taken, where the
measuring happens, where a 🟨 becomes a 🟧. And here the disk is the enemy — **`df -h /home`
before the first build**.

---

## What really happened here — four traps on machine A

### 1. Rust was not installed at all

No `rustc`, no `cargo`. Installed afterwards with `rustup` into `~/.cargo`.

**The cost for the next person:** the shell does not find `cargo`, and the message
(`command not found`) looks like a broken machine.

```bash
export PATH="$HOME/.cargo/bin:$PATH"     # belongs in front of every cargo call on A
```

### 2. Bevy's umbrella features drag `wayland` along — and A cannot build it

The umbrella features `3d`, `ui` and `2d` pull in `default_platform`, and `wayland` sits
hard-wired inside it. On A `wayland-client.pc` is missing, and without passwordless sudo it is
not going to turn up either.

| | |
|---|---|
| Symptom | `cargo build` aborts in `wayland-sys` |
| Measured cost | **9m22s** up to the abort `[debian]` — the time is gone before the first line of our own code is compiled |
| Cause | not the code, not the Bevy version: one missing `.pc` file of the system |
| Fix | `default-features = false` and the feature list **by hand** in `Cargo.toml`, plus `[features] default = ["x11"]` with `x11 = ["bevy/bevy_winit", "bevy/x11"]` and `wayland = ["bevy/bevy_winit", "bevy/wayland"]` |

Why `x11` is the default: winit uses `x11rb`/`x11-dl` for it, and those load at runtime via
`dlopen` — so **no** system library is needed at build time. `wayland` needs `wayland-client.pc`
and is optional for exactly that reason.

```bash
cargo build                            # A (debian): the default, builds anywhere
cargo run --features wayland,audio     # B (offlinebot) — short: cargo play
```

`audio` has the same problem one floor down: it needs `alsa.pc`, and that is missing on A just
as much. With the hand-written list **plus `audio`** the build aborts after **13m40s** in
`alsa-sys` (measured `[debian]`, stands in `Cargo.toml`). That is why `audio` is optional too
and sits only in `cargo play`.

The reasoning stands as a comment in `Cargo.toml`, the shorthands in `.cargo/config.toml`.
**Switch `default` back on there and you give away nine minutes on A and notice it too late.**

### 3. No pip, no ensurepip, no openpyxl, no libreoffice

The spreadsheet still has to be read. So `tools/features.py` reads the `.xlsx` with the
**standard library**: an `.xlsx` is a ZIP of XML, `zipfile` + `xml` are enough.

**Not** "then just do not extract on A" and **not** "install openpyxl" — the installation
attempt fails on the missing pip and the missing sudo and costs nothing but search time. The
side effect is the actual win: the extraction now runs on **every** machine without an
installation.

### 4. `target/` is large before a binary even exists

Measured `[debian]`: `du -sh target/` → **5.1 G**, almost all of it `target/debug/deps`, and at
that point `target/debug/defeated_by_titan` does not exist yet. That is not a leak, that is the
order of magnitude of Bevy with `opt-level = 3` for dependencies. Whoever does not plan for it
lands in the first generic trap below (`ld: signal 7`).

---

## The generic traps: when the message means something other than what it says

| Message | What it **really** means | What you do |
|---|---|---|
| `ld: signal 7` / `Bus error` while linking | **The disk is full.** `target/debug/deps` piles up Bevy binaries in the three-digit GB range | **`df -h /home` first**, do not suspect the code. Then `cargo clean`, or `rm -rf target/debug/incremental` specifically |
| `undefined hidden symbol: anon.….llvm.…` | a broken incremental cache after a build that was killed | `rm -rf target/debug/incremental` |
| A red build that is green | you filtered on `.rs` and caught the warnings with it | `cargo check 2>&1 \| grep '^error'` — do **not** filter on `.rs` |
| "but the teardown did happen" | `pkill` stood at the front of the chain, found no process, returned exit 1 — and swallowed the rest | **NEVER put `pkill` at the front of a command chain.** `pkill -f target/debug/defeated_by_titan` also returns exit 144 now and then: that is normal |

The disk is the enemy on **B**, not on A: on B it has been full once already (on A, according to
[`docs/environment.md`](../environment.md), 406 G of 452 G are free). That is why `df -h /home`
is mandatory there, not mere caution.

---

## Several agents in the same repo

Files change underneath you, and the build is red in between **without any doing of yours**.
That is not a regression, that is another agent in the middle of an edit.

| Rule | Why |
|---|---|
| **Read the file fresh before every edit** | your picture of the file can be stale without you having noticed anything |
| **NEVER `git stash`, `git checkout --`, `git clean -fdx`** | that throws away the work of somebody typing right next to you. There is no case in which it is right here |
| A RON file is written as a **WHOLE** file | two sessions do not merge. **Whoever saves last wins everything** |
| After every write into a shared file, check with `grep` that your value is in there | otherwise you never notice the loss |
| Enter in [`docs/STATUS.md`](../STATUS.md) which domain you are working on | so that another agent takes a different one — avoiding conflicts is cheaper than resolving them |

## `// TEMP`

**Always** mark temporary hacks for taking a screenshot with `// TEMP`, and afterwards:

```bash
grep -rn TEMP src/
```

A forgotten test hack is a ghost the next person hunts — and he hunts it in the game code, not
in the tool.

---

## Gaps

- **Offscreen rendering on A is unproven.** Whether the N100 really delivers a PNG through a
  render target has been tested by nobody here. Until then the ceiling on A stays 🟨.
- **`blender --background` has not run on A** — Blender is missing on this machine
  ([`docs/environment.md`](../environment.md)). That the model chain works headless is the
  claim of the source, not a measurement of this project.
- **The generic traps** (bus error, `anon.llvm`, exit 144) have not been triggered here. They
  stand as a warning, not as a record.
- **The `--headless` mode does not exist yet.** `src/shared/cli.rs` does already read the flag,
  but `src/main.rs` starts only `MinimalPlugins` — there is neither a window nor a run. As long
  as that is so, A can do `cargo test` and nothing else; that is the reason the mode belongs in
  stage 1 and not later.
- **The table "On A you work" says what would be possible, not what is there.** There is no
  `tests/` in this repo yet (so no `tests/multiplayer.rs` either), and no `tools/blend/` either.
  Those lines come from `prompts/init.md` §14 and are a requirement, not a record.

Related: [docs/environment.md](../environment.md) · [STATUS.md](../STATUS.md) · [BUGS.md](../BUGS.md) · [conventions.md](../conventions.md) · [FINDINGS.md](../FINDINGS.md) · [lessons/workflow.md](workflow.md) · [lessons/performance.md](performance.md)
