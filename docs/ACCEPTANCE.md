# ACCEPTANCE — what the user should please look at once

Updated: 2026-08-09 · Stage: 🟨

**Claude never sets ✅ himself.** Not for green tests, not for a pretty screenshot, not
"because it obviously runs". **🟧 is the highest stage Claude may award himself**
(`prompts/init.md` §8). What Claude considers ripe stands here — with the evidence beside it,
so that looking takes two minutes and not twenty.

## What a line here looks like

| Item | ID | Stage now | where to look | how long |
|---|---|---|---|---|
| *(example)* Sinking a hook | F-001 | 🟧 | `cargo run --features wayland -- --sandbox`, then hold `docs/images/f001-hook.png` next to it | 2 min |

---

## The proven route to an image

**Since 2026-08-09 there is one.** `--screenshot <path>` takes a PNG after `--ticks <n>`
simulation steps and exits. The trigger is a **tick**, not a second and not a key: the same
command therefore delivers the same image tomorrow, and a task can satisfy its own image line
without a human pressing a key at exactly the right moment.

```bash
# The recommended route — a fixed 1280x720, independent of compositor and window size:
cargo run --features wayland,audio -- --offscreen \
    --script scripts/t006-shot-far.txt --ticks 110 --screenshot docs/images/t006-world-far.png

# With a window — you watch while it happens. The same SCENE, but not the same image:
# the aspect ratio comes from the compositor, not from the command.
cargo run --features wayland,audio -- \
    --script scripts/t006-shot-near.txt --ticks 110 --screenshot docs/images/t006-player-view-window.png
```

**Measured `[cachy]`, not claimed:**

| Question | Answer | Evidence |
|---|---|---|
| Does an image come out? | yes, in both modes, exit 0 | the four PNGs below |
| Is it reproducible? | **`--offscreen` yes, bit for bit** | two runs, `sha256 = eb212dfe…` both times |
| And with a window? | **no** | the same command delivered 1267x1390, and four minutes later 627x974 — **the compositor decides the image size, not the command** |
| Does it work without a graphics session? | yes | `env -u WAYLAND_DISPLAY -u DISPLAY … --offscreen` writes a full image |
| Does it work with `--headless`? | **no, and it says so now** | `--headless` sets `backends: None`; the combination ends in **exit 1** and a line that points at `--offscreen` |

The three modes and the passages in the Bevy source that prove them are in
[`src/debug/screenshot.rs`](../src/debug/screenshot.rs). **`--headless` stays image-less** —
that is not convenience, it is the consequence of `backends: None`.

## Ready to be looked at

| Item | ID | Stage now | where to look | how long |
|---|---|---|---|---|
| There is an image at all | T-006 | 🟧 | `docs/images/t006-world-far.png` and `docs/images/t006-player-view.png` — the first pixels this project has ever seen | 2 min |

The three pieces of evidence for this one 🟧, so that it can be re-checked and, if need be,
moved back down (`docs/STATUS.md` belongs to the main head, what stands here is only the
proposal):

- **Image:** the four PNGs in `docs/images/`, looked at and described below.
- **Number:** 1280x720 · `sha256 = eb212dfe…` the same across two runs · 696808 / 1138720
  bytes in window mode at 1267x1390.
- **A test that goes red:** the run itself. `--screenshot` ends in **exit 1** when no PNG
  appears — cross-checked with `--headless --screenshot`: exit 1, no file. There is **no**
  `cargo test` case that checks the pixels; what `cargo test` covers is only the flag and the
  trigger tick (`src/shared/cli.rs`, `src/debug/screenshot.rs`).

**What is on the images** — four images, two views times two routes:

| File | Size | what is on it |
|---|---|---|
| `t006-world-far.png` | 1280x720 | From 19–20 m up (the `warp` sets 20 m, and by the time the image is taken the player has fallen about a meter — the two `assert`s bracket 18 < h < 21), 45 m behind the origin: the olive ground slab out to its edge at 245 m, and on it the four placeholder blocks from `src/world/mod.rs` — a brick-red cube (8 m), a small sand-brown one (4 m), two stone-gray ones (12 m and 18 m). All four stand cleanly on the ground, none is stuck in it |
| `t006-player-view.png` | 1280x720 | The same scene from **1.6 m eye height**, 4 m in front of the origin. The scale holds: the 4 m cube at 16 m distance just clears the horizon line, the two large blocks fill the right edge of the frame. A human is small here — which is exactly what `scale.ron` asked for |
| `t006-world-far-window.png` | 1267x1390 | The same scene, but captured **through the window** — portrait, because niri laid the tile out that way. Vertically identical (the field of view is defined vertically), horizontally cropped. The evidence that the window route works **and** that its image size does not belong to the command |
| `t006-player-view-window.png` | 1267x1390 | same again, from eye height |

**And what is NOT on them, although you would expect it:**

- **No sky.** The upper half of the frame is a uniform dark gray — that is Bevy's
  `ClearColor`, not atmosphere. There is neither a sky nor the distance fog that bible 3.4
  asks for (`docs/architecture.md`, translation table).
- **No shadows.** Deliberately: `shadow_maps_enabled: false` in `src/render/mod.rs`, with a
  reason. You see surface shading only, no cast shadow.
- **No city.** `assets/data/maps.ron` describes one, but `world::map::build_map` is an empty
  stub — what stands in the image are the **four placeholder blocks** from `spawn_ground`.
- **No looking around.** `render::camera::rotate_camera` is empty as well; the camera always
  looks at −Z. `look` in a script changes nothing about the image, only `warp` does.

## Not ready, and why

| Item | Stage | what is missing for 🟧 |
|---|---|---|
| **everything visible except the screenshot route itself** | ⬜/🟨 | The screenshot route stands, but an image on its own does not make anything 🟧: that takes **an image, a number and a test that goes red**. Whoever raises something to 🟧 now fetches their image line with the command above — the excuse "there is no image on this machine" is gone |
| An image on **machine A** (`debian`) | open | `--offscreen` needs a wgpu adapter, not a window. That it works **without a graphics session** is measured on `[cachy]`; that the N100 under debian finds an adapter is **not** (`docs/QUESTIONS.md` Q-009) |

## What the user also wanted to see (`prompts/init.md` §16)

These six points are the acceptance of the **commission**, not of individual features. Where
they stand belongs in the closing report of every session:

| # | Asked for | Where it stands |
|---|---|---|
| 1 | `cargo test` — the output, summarized without cutting anything | **62 green, 0 red** `[debian]`: 42 unit tests in the crate, 10 `tests/data.rs`, 7 `tests/multiplayer.rs`, 3 `tests/domains.rs`. None skipped |
| 2 | At least two screenshots in `docs/images/` — **on machine B**. On A instead: the `--headless` script runs with their `assert` results and the note "pixels unseen" | **Four images, all taken on `[cachy]` and looked at** (table above), out of `scripts/t006-shot-far.txt` and `scripts/t006-shot-near.txt`, both with their `assert`s holding and exit 0. On top of that the two older runs: `scripts/t007-first-run.txt` (6 `assert`, 180 ticks) and `scripts/t019-latency.txt` at `--lag 200` (3 `assert`). Counter-check: a deliberately wrong `assert` ends in **exit 1** and prints the measured value |
| 3 | `docs/STATUS.md`, in which every item carries one of the four stages — and **not a single ✅** | stands, generated from `docs/features.ron`: 245 rows, **239 ⬜ · 6 🟨 · 0 🟧 · 0 ✅** |
| 4 | **This file**, filled in | stands — and the list "ready to be looked at" has had its **first line** since 2026-08-09 |
| 5 | The model table and at least one `.blend` with anchors set | **open.** There is no Blender on machine A ([`environment.md`](environment.md)); all that would come of it is `tools/blend/*.py` without a `.blend` and without a `.glb`. The chain is described in [`models.md`](models.md), but **not built** — ⬜, not 🟨 |
| 6 | An honest paragraph: **what is built but not seen** | **Almost everything — but not everything any more.** Seen now: the ground, the four placeholder blocks, the light, the camera height, the scale and the colors. Everything else stays unseen, and two things stood out on the images as **missing** rather than broken: the city from `maps.ron` (stub) and the camera rotation (stub). The window route has been seen, but is **not reproducible** — only `--offscreen` is |

> **The one rule above all others: measure first, then claim.** Almost every expensive
> mistake in a project like this one is a place where something reasonable was *explained*
> instead of being *measured* in a minute — and the explanation was the problem.

Related: [`docs/STATUS.md`](STATUS.md) · [`docs/environment.md`](environment.md) ·
[`docs/QUESTIONS.md`](QUESTIONS.md)
