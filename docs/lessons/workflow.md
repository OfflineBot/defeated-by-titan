# workflow — you cannot click, so you build launch flags, a script driver and an overlay before you build a feature

Updated: 2026-08-09 · Stage: 🟨 (written down from `prompts/init.md` §12 — not one of the tools
described here has been built in this repo so far, so none of it has run)

## The problem

Everything is built, nothing is seen. Every feature sits behind mouse and keyboard, and nobody
is sitting at the keyboard. To an agent a main menu is a wall without a door: the build runs,
the window stands there, and nothing ever happens.

The source calls this the point where projects like this one fail. The conclusion is
uncomfortable: **the checking infrastructure is part of stage 1**, not a "when there is time".
For stage 1, `prompts/init.md` §13 explicitly lists `--sandbox`, `--script`, the F3 overlay, a
screenshot and `--headless` right next to the camera and gravity — in the same box, not behind
it.

## a) Launch flags that walk past the menu

| Flag | what for | what it costs when it is missing |
|---|---|---|
| `--mission tutorial` | straight into a mission, no menu | every run ends in the main menu |
| `--sandbox` | empty field, one titan, unlimited gas — for looking at things | nowhere to see anything in isolation |
| `--novsync` | for measuring | under vsync you measure the 16.6 ms ceiling six times (§11) |
| `--lag 200` | 200 ms of simulated latency (bible T-019) | every movement feature only ever gets checked locally |
| `--script <file>` | playing the game without typing | see above: nobody types |
| `--headless` | no window, fixed tick, N ticks, exit code | on machine A nothing at all is checkable |

```bash
cargo run -- --mission tutorial
cargo run -- --sandbox
cargo run -- --novsync
cargo run -- --lag 200
cargo run -- --script <file>
```

`--headless` is not in §12 but in §13/§14: `primary_window: None`, a fixed tick, runs N ticks
and **ends with an exit code** that says whether every `assert` held. Without this flag
`--script` is worthless on a machine with no graphics session.

## b) `--script` — the driver

A text file, **one instruction per line**. It writes into **the same input a human triggers**
(`ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`, plus a "pretend" look vector). **No second,
false way to play** — every system behind it is the real one. Build a side entrance and you test
the side entrance.

```text
spawn titan normal 20 0 -40   # kind and position in meters
look 0 -10                    # look direction in degrees (yaw, pitch)
key Space 0.3                 # hold the key for 0.3 s
hook left                     # hook out
wait 1.2                      # commands are delayed — otherwise you photograph an empty field
mark anchored                 # a line in the log to line a screenshot up with
assert speed > 25             # the script is allowed to judge for itself
```

| Instruction | Unit / meaning | the trap behind it |
|---|---|---|
| `spawn` | position in **meters** | — |
| `look` | yaw, pitch in **degrees** | — |
| `key` | hold time in **seconds** (`0.3`) | — |
| `wait` | seconds | **commands are delayed** — without `wait` you photograph an empty field |
| `mark` | a log line as an anchor | without an anchor nobody knows when the screenshot was taken |
| `assert` | a condition that has to hold | without `assert` it is a demo, not a test |
| `warp` | jump to a coordinate (§12c) | — |

**Why `assert` is the whole trick:** it turns a run into a test. The feel of movement is exactly
the kind of thing no unit test gets hold of — "anchors, and is faster than 25 afterwards" cannot
be checked as a pure function, but it can as a script run. If the `assert` falls over, the exit
code falls over, and that holds on every machine and one day in a CI.

## c) F3 overlay — every report reproducible

According to the source these belong on screen: position, look direction, speed, gas, hook
state, frame time. Together with `warp x y z` + `look` in the script. With that the user sends a
coordinate and you stand exactly there. The source rates this as **worth more than any bug
report form**.

## d) Screenshots — **machine B only**, not here

⚠️ **On this machine (`debian`, machine A) there is no graphics session and no `niri`.**
Measured in [`docs/environment.md`](../environment.md): `WAYLAND_DISPLAY` and `DISPLAY` both
empty, `command -v niri` empty. The section that follows is **not** executable here; it applies
to `offlinebot` (machine B, niri/Wayland).

```bash
setsid nohup cargo run -- --sandbox > /tmp/dbt.log 2>&1 < /dev/null & disown
sleep 20   # the first build takes a while
ID=$(niri msg --json windows | python3 -c "import sys,json;print([w['id'] for w in json.load(sys.stdin) if (w.get('title') or '')=='Defeated by Titan'][0])")
niri msg action focus-window --id $ID   # OTHERWISE the compositor throttles to ~5 fps
sleep 2
niri msg action screenshot-window --id $ID
```

The images land in `~/Pictures/Screenshots/`. **Copy them to `docs/images/` and link them in
`STATUS.md`** — a screenshot nobody can find again is not evidence.

| Trap | how you recognize it | what you do instead |
|---|---|---|
| an unfocused window | ~5 fps, looks **exactly** like a regression | `focus-window` before **every** fps measurement, then measure |
| several instances | — | check that only **one** instance is running — otherwise you photograph old code |
| shot too early | an empty field although `spawn` is in the script | `sleep 20` after the start, `wait` in the script, `mark` as the anchor |

## The case of "no graphics session at all"

No `WAYLAND_DISPLAY`, no `DISPLAY` → `cargo run` **panics immediately**. Then there is **no
image**. Then:

- The item stays **🟨**, with the note *"logic tested, pixels unseen — machine A"* (§14).
- You ask the user to take a look.
- **Do not round up.** No "it surely looks right". Only somebody who has seen something sets
  🟧; only the user sets ✅.

That is not a blockade: `cargo test`, `--headless` script runs with `assert`,
`blender --background` and the docs all run here in full (§14). Only the proof in pixels is
missing — and then it is declared as missing instead of claimed.

**Gap:** §14 allows offscreen rendering into a PNG as an exception, but **only once it is
proven** that it really delivers an image on this machine. Not proven so far (see
[`docs/QUESTIONS.md`](../QUESTIONS.md) Q-009).

## e) Research and assets — allowed, on three conditions

Explicitly allowed and wanted: YouTube (watching movement and level design — anchor density,
roof heights, street widths), Google / image search for references, technical articles on rope
physics, netcode and audio synthesis, and the **docs of the installed Bevy version** (per §3 the
most important source of all). Scripts are permitted: `yt-dlp` for subtitles and descriptions,
`curl`, a small parsing script.

**Downloading assets is allowed — this is a prototype.** Models, sounds, placeholder music. The
user replaces all of it himself later anyway (§7); until then a good prototype is worth more
than polygons of our own. As starting points the source names (not as a prescription): Kenney,
Poly Pizza, OpenGameArt, Quaternius, Sketchfab with the CC filter, Freesound (CC0), Pixabay.

| Rule | Why, and what happens otherwise |
|---|---|
| Everything third-party into `assets/extern/` + a line in `ATTRIBUTION.md` + `attribution:` in the registry (§7) | without those three it is a **zombie** (§10) — the user will not find it later to replace it |
| `assets/extern/` does **not** go into the public repo | it is ignored; `tools/fetch_extern.sh` obtains it again (§7) |
| Numbers and findings with their source into `docs/gameplay/references.md` | *"streets are 8–12 m wide so that one hook reaches both sides"* — **a number without an attribution is a claim** (§9) |
| Reference images into `gameplay/bilder/` or `docs/gameplay/references/`, with URL and date | otherwise the image is an anonymous JPG in a week |
| Where they contradict each other, reality wins | a blog post about Bevy versions is not a source, the installed docs are one |

The most valuable thing about a piece of research is rarely the file, it is the **number** — and
that is worth exactly as much as the attribution line next to it.

## What that means for the order of work

| first | then |
|---|---|
| `--sandbox`, `--script`, `--headless`, the F3 overlay | the first feature you want to look at with them |
| `mark` + `assert` in every script | a screenshot that matches a log line |
| `hostname` (§14) | the decision whether an image is possible today at all |

**The gap is closed (2026-08-10).** Every tool in this file exists and has run: eleven flags in
`src/shared/cli.rs`, a nine-verb script driver in `src/debug/script.rs`, the F3 overlay,
`--screenshot`, `--offscreen`, `--headless`. `scripts/game-full.txt` plays the whole game to
`MISSION WON at tick 898` with 23 asserts and exit 0. *(The paragraph that stood here said none
of it was built and that `src/main.rs` was a `MinimalPlugins` placeholder — true when written,
false for a day before anybody noticed. A stale gap note is worse than no note: it tells the next
session to build something that already exists.)*

## e) A failing assert is a probe, and `wait 0` is a per-tick clock

**The trick that turned "undecidable without running" into nine measured curves, without
touching `src/`.** The script vocabulary has no verb that *samples* anything — but
`src/debug/mod.rs:231-244` prints the measured value of an assert **when it fails**. So an
assert that can never hold is a read-out:

```
assert speed < -1        # can never be true -> always prints the real speed
```

Two mechanics turn that into a trace:

- **`wait 0` costs exactly one tick** (`run.wait_s = 0.0; return`, and `0.0 > 0.0` is false on
  the next tick). `wait 0.001` would cost two — the difference matters when you are counting.
- **Asserts with no `wait` between them run in the same tick**, so `tick` / `speed` / `height`
  next to each other form one coherent sample.

150 such blocks in a row gave a per-tick trace of the reel and produced the measurement that the
reel assigns velocity rather than accelerating it (`docs/FINDINGS.md` FIND-013): `0.000 →
28.000 m/s` in one tick, exact across nine configurations.

**And the values it reads are file values, so no rebuild is needed either.** `src/data/mod.rs:46-62`
resolves `assets/data` **relative to the working directory** before falling back to
`CARGO_MANIFEST_DIR`. Copy `assets/` into a scratch directory, edit a number there, run the
**already-built** binary from that directory, and you have swept a tuning value without paying
for a single compile — which matters, because cargo's lock on `target/` means only one agent can
compile at a time.

⚠️ **Prove the scratch tree is the one being read, before you take a number from it.** Put
something unparseable in your copy and check that the run dies on it. Otherwise a whole sweep can
silently measure the repository's unchanged value nine times.

Related: [`docs/environment.md`](../environment.md) · [`docs/lessons/environment.md`](environment.md) ·
[`docs/lessons/supervision.md`](supervision.md) ·
[`docs/lessons/performance.md`](performance.md) · [`docs/STATUS.md`](../STATUS.md) ·
[`docs/TODO.md`](../TODO.md) · [`docs/BUGS.md`](../BUGS.md) ·
[`docs/QUESTIONS.md`](../QUESTIONS.md) · [`docs/ROADMAP.md`](../ROADMAP.md) ·
[`docs/conventions.md`](../conventions.md) — source: `prompts/init.md` §12 (lines 1176–1264),
`--headless` from §13/§14.
