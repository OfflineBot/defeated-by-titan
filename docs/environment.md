# environment — which machine can do what

Updated: 2026-08-18 · Stage: 🟧 (measured, not estimated — the command stands next to every row)

The project runs on two machines, and they cannot do the same things. Whoever mixes them up
takes a missing graphics session for a bug, or an N100 for a performance regression
(`prompts/init.md` §14). **First question of every session: `hostname`.**

## A — `debian` (measured 2026-08-09)

| Question | Answer | measured with |
|---|---|---|
| Hostname | `debian` | `hostname` |
| Kernel | 6.12.85+deb13-amd64 | `uname -r` |
| Cores | **4** | `nproc` |
| Graphics session | **none** — `WAYLAND_DISPLAY` and `DISPLAY` both empty | `echo "$WAYLAND_DISPLAY $DISPLAY"` |
| Compositor | **no `niri`** | `command -v niri` → empty |
| Disk `/` | 452 G, 406 G free (6 % used) | `df -h /home` |
| Rust | **1.97.1** (8bab26f4f 2026-07-14), cargo 1.97.1 | `rustc --version` |
| Blender | **missing** | `command -v blender` → empty |
| `gh` | present, logged in as `OfflineBot` | `gh auth status` |
| Python | 3.13.5, **no pip, no ensurepip, no openpyxl** | `python3 --version` |
| LibreOffice | **missing** | `command -v libreoffice` |
| Network | reachable (crates.io, static.rust-lang.org) | `curl -sI` |
| passwordless sudo | **no** | `sudo -n true` |

### What was installed here afterwards

- **Rust** did not exist on this machine. Installed with `rustup` into `~/.cargo`
  (`--no-modify-path --profile minimal`). **Which means: `export PATH="$HOME/.cargo/bin:$PATH"`
  belongs in front of every `cargo` call**, for as long as the shell does not do it itself.

### What follows from that — with nothing glossed over

| Item | Consequence |
|---|---|
| **No window** | **The ceiling on A is 🟨**, with the note *"logic tested, pixels unseen — machine A"*. No screenshot through the compositor, so no 🟧 out of a window (§8, §14). |
| **No Blender** | The model chain only builds `tools/blend/*.py`; `.blend` and `.glb` do not come into being here. The game has to run without Blender — **warn once, use the `.glb` that is there, do not crash** (§7). |
| **No pip** | `tools/features.py` uses the standard library and nothing else (`zipfile` + `xml`), no `openpyxl`. That is why it runs on any machine without an install. |
| **4 cores** | Parallelism 2–3 at a time, and **the compiler is a consumer too** (§17). Twenty agents on four cores are slower than three. |
| **N100 class** | **No performance statement comes from here.** Every number in `STATUS.md`/`BUGS.md` carries `[debian]` — a frame time without a machine is not a measurement. |

### The one open proof: offscreen rendering

`prompts/init.md` §14 allows one exception: an image out of a render target instead of out of
a window. **Until it is proven that this really delivers a PNG on this machine, it is worth
nothing** — and so far it is not proven. See `docs/QUESTIONS.md` (Q-009) and `docs/TODO.md`.

## B — `offlinebot` (measured 2026-08-09)

Until then the figures came from `prompts/init.md` §14 and were wrong in two places — see the
consequences table below. They are now re-measured with the same commands as on A.

| Question | Answer | measured with |
|---|---|---|
| Hostname | `offlinebot` | `hostname` |
| Kernel | 7.1.6-1-cachyos | `uname -r` |
| Cores / threads | 8 cores, **16 threads** (Ryzen 7 5800X) | `nproc` · Bevy's `SystemInfo` reports `core_count: 8` |
| Graphics session | **yes** — `WAYLAND_DISPLAY=wayland-1`, `DISPLAY=:0` | `echo "$WAYLAND_DISPLAY $DISPLAY"` |
| Compositor | **niri 26.04** (8ed0da4) | `niri --version` |
| GPU | NVIDIA GeForce RTX 3080 | `nvidia-smi --query-gpu=name --format=csv,noheader` |
| RAM | 31 GB, 24 GB of it free | `free -g` |
| Disk `/home` | 928 G, **128 G free (87 % used)** | `df -h /home` |
| Rust | **1.95.0** (59807616e 2026-04-14), cargo 1.95.0 | `rustc --version` |
| Where cargo lives | **`/usr/bin/cargo`** — *not* in `~/.cargo` | `which cargo` |
| Blender | **5.2.0 LTS** present | `blender --version` |
| `gh` | present, logged in as `OfflineBot` | `gh auth status` |
| Python | 3.14.6, `pip3` present, **no openpyxl** | `python3 --version` |
| LibreOffice | **missing** | `command -v libreoffice` |
| passwordless sudo | **no** | `sudo -n true` |

### What follows from that — the three differences to A that hurt

| Item | Consequence |
|---|---|
| **A window exists** | **On B, 🟧 is reachable.** Screenshot via `niri msg action screenshot-window`. On A it ends at 🟨 (§8, §14) — whatever is built there has to be *seen* here, or it stays 🟨. |
| **Rust is OLDER here than on A** | B: **1.95.0**, A: 1.97.1. The direction is the unexpected one: what compiles on A does not necessarily compile here. A language feature from 1.96/1.97 shows up only on B. **The project's floor is 1.95.0.** |
| **`cargo` lives in `/usr/bin`** | The `export PATH="$HOME/.cargo/bin:$PATH"` from `CLAUDE.md` is a **machine-A line**. Here it has no effect, but it does no harm. |
| **Blender is here** | The model chain can really be driven here — on A it cannot. Model work belongs on B. |
| **16 threads, but `cargo` locks `target/`** | The parallelism does **not** come from `nproc`: building agents wait on the same lock. **Four at a time** are usable, not sixteen (`docs/lessons/supervision.md`). |

⚠️ **Run `df -h /home` before every larger build.** `ld: signal 7 / Bus error` while linking
means a full disk, not broken code (§15). And the old figure in this file was far too low:
**this project's `target/` measures 17 G** (`du -sh target`), not 1 GB. With 128 G free that is
still headroom, but `cargo clean` costs a full Bevy rebuild afterwards — do not call it out of
habit.

## Measured build times

| What | Machine | Time | Command |
|---|---|---|---|
| first `cargo build` (Bevy 0.19.0 + ~460 crates, `opt-level = 3` for dependencies) | `[debian]` | see `docs/lessons/environment.md` | `cargo build` |
| `cargo check` over the whole tree, Bevy already built | `[cachy]` | **1 min 22 s** | `cargo check` |
| `cargo test` (95 tests, warm tree) | `[cachy]` | under 10 s | `cargo test` |

## Where a run finds `assets/` — and why that is not the working directory

**One answer for both halves of `assets/`.** The RON files and the models live in the same
folder, so they are found the same way: `data::assets_dir()` (`src/data/mod.rs`) is the only
place that resolves it, and `src/lib.rs` hands its result to Bevy as
`AssetPlugin::file_path`. The order is **the working directory first, then the crate the
binary was built from** — the second one is the compile-time `CARGO_MANIFEST_DIR`, baked into
the executable.

| How it is started | RON | models |
|---|---|---|
| `cargo run` | ✅ | ✅ |
| `./target/debug/defeated_by_titan` from anywhere | ✅ | ✅ **since 2026-08-18** |
| the binary copied somewhere else | ✅ | ✅ |
| from a **mirror asset root** (a copy of `assets/` as the working directory) | ✅ | ✅ |

**What it looked like before 2026-08-18.** Bevy resolves its asset folder against
`BEVY_ASSET_ROOT`, then the `CARGO_MANIFEST_DIR` **environment variable**, and only then
against the executable's own directory (`bevy_asset-0.19.0/src/io/file/mod.rs:19-29`).
`cargo run` and `cargo test` both set that variable — so the two places we look were the two
places that worked, while **every script run in this project starts the binary directly** and
looked in `target/debug/assets/`. Measured that day, same mirror asset root, two binaries:

```
before  ERROR bevy_asset::server: Path not found: <exe dir>/assets/3d/glb/a-042-….glb
after   INFO  art.ron: 1 model(s) come out of a file, the rest stay primitives   (no error)
```

Nothing had ever been loaded through the asset server — all eight `art.ron` rows said
`Primitive` — which is the only reason it stayed invisible for nine days.

**How to see for yourself, from any directory:**

```bash
cd / && env -u CARGO_MANIFEST_DIR -u BEVY_ASSET_ROOT RUST_LOG=bevy_asset=debug \
  /path/to/target/debug/defeated_by_titan --headless --ticks 1 2>&1 | grep 'base path'
```

Guarded by `tests/data.rs::the_bare_binary_finds_its_assets_from_a_foreign_working_directory`.

Related: [`docs/lessons/environment.md`](lessons/environment.md) (the traps), `prompts/init.md` §14/§15.
