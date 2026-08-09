# performance — the five rules from §11, the target numbers from the bible, and what is missing from both sources

Updated: 2026-08-09 · Stage: 🟨 (written down from the sources; none of it measured in this project yet)

§11 is titled *„die Regel, die man von Anfang an einhalten muss"* — the rule you have to keep
from the very start. Performance is therefore not a final polish here. Sources:
`prompts/init.md` §11 (lines 1151-1172) and `prompts/DefeatedByTitan_Design-Bibel.md` 3.5
(line 99) / 6.4 (line 235). What stands here stands there; where something is derived, it is
marked as derived.

## The target numbers from the bible

| Requirement | Number | Source |
|---|---|---|
| Frame rate | **the minimum profile and the full profile both aim at 60 FPS** | bible 3.5 |
| Minimum profile | entry-level laptop, **integrated graphics** | bible 3.5 |
| Quality profiles in total | **two instead of five** | bible 3.5 |
| The real stress test | **20 players with two ropes each + 60 titans** — "not the graphics" | bible 6.4 |
| Countermeasure for it | plan interpolation buffers and replication throttling **in from P1 on** | bible 6.4 |

60 FPS means 16.6 ms per frame — for **everything** together: simulation, network, rendering.
Neither source names a split of that budget (see Gaps).

## Rule 1 — the spatial index belongs in `world/`, from day one

A city has thousands of houses, a mission dozens of titans, every titan six limbs (§11). The
source puts it hard:

> **Nichts darf alle Entities durchlaufen, um eine Frage über die zehn Meter vor der Nase zu
> beantworten.**

— nothing may walk every entity to answer a question about the ten meters in front of your nose.

| | |
|---|---|
| **What** | grid cells → entities, in `world/` (see `docs/architecture.md`: `world/` keeps the spatial index) |
| **How it stays current** | through Bevy's `Added` / `RemovedComponents` — so the index **cannot go stale** |
| **Who uses it** | hook impact, blade hits, collision, titan target search — **all four**, no exception |
| **How you spot the violation** | a query without a location filter in a system that asks a local question |

## Rule 2 — nothing changes per frame, everything per second

`* time.delta_secs()` alone is **not** enough. Three sub-cases that all look as if they were
already frame-rate independent, and are not:

| Case | Wrong (per frame) | Right (per second) |
|---|---|---|
| **(a) Integers** | `(damage * dt).ceil()` — this "macht die Framerate zur Schadenszahl" (§11): the frame rate becomes the damage number | carry the fractions along, **never round** |
| **(b) Exponential smoothing** | `x += (target - x) * 0.1` | factor `1 - e^(-k*dt)` |
| **(c) Noise** | `noise * dt` | noise scales with **`sqrt(dt)`** |

**The instruction for this is organizational, not mathematical:** write one single helper in
`shared/` and use **only that one** (§11).

It is already in the repo: `src/shared/math.rs` — `clamped_dt_s`, `smooth`, `smooth_vec3`,
`noise_scale`. New callers take these functions, not a second calculation next to them.

## Rule 3 — measure first, then claim. And: debug is slow

```bash
cargo run              # debug build — our own crate sits at opt-level = 1
cargo run --release    # the only thing you may make a perf claim about
```

| Observation | Correct reaction |
|---|---|
| "it has been stuttering since today" — measured with `cargo run` | Do nothing. **Debug slowness is not a regression** (§11). Measure against `--release` first |
| "that system is too expensive" without a number | Do not start optimizing. Measure first, then claim |

## Rule 4 — under vsync every frame time is 16.6 ms

With vsync, the question "what does this cost?" measures **the same ceiling six times** (§11).

Both read together (§11 + bible 3.5, phrased this way in neither source): the target is 60 FPS,
and the vsync ceiling sits at exactly those 16.6 ms. "Runs at 60" is therefore **no** evidence
that there is room in the budget. Only measured without the ceiling does the number say
anything.

| Tool | Status | What it gives you |
|---|---|---|
| `--novsync` launch flag | **has to be built**, "early" (§11) — does not exist yet | ceiling gone, real frame time |
| Bevy's `RenderDiagnosticsPlugin` | named by §11 as an alternative or an addition, not built in here yet | **real GPU timestamps per render pass** |

## Rule 5 — shadows are the most expensive switch in the game

Point lights are almost free, shadows are not (§11). Therefore: **at the end, and with a
number.**

## The real stress test is not the graphics

The bible files this under **risks** (6.4), not under graphics:

> Zwanzig Spieler mit je zwei Seilen und sechzig Titanen sind die eigentliche Belastungsprobe,
> nicht die Grafik.

— twenty players with two ropes each and sixty titans are the real stress test, not the
graphics.

| What | Number | Where from |
|---|---|---|
| Players per instance | 20 | bible 6.4 |
| Ropes | 2 per player → **40** at once | the 2 per player stands in bible 6.4; the 40 is arithmetic |
| Titans | 60 | bible 6.4 |
| Limbs | 6 per titan → **360** (§11 × bible 6.4, named this way in neither source) | derived |

Practical consequence for the order of work: a scene that simulates 60 titans and 40 ropes at
the same time is the yardstick. And the countermeasure the bible names for it is **network**,
not rendering: interpolation buffers and replication throttling, planned in from P1 on (see
`docs/multiplayer.md`).

## Gaps — what is NOT in the sources

| Gap | Why it hurts |
|---|---|
| No split of the 16.6 ms across simulation / network / rendering | "too expensive" cannot be decided without a sub-budget |
| No hardware definition of the minimum profile beyond "entry-level laptop, integrated graphics" | without a concrete device, "60 FPS on the minimum profile" cannot be checked |
| No defined measurement scene | 60 FPS in what? The 20/40/60 scene from 6.4 is the obvious candidate, but nowhere written down as the check scene |
| No cell size for the spatial index | §11 says "grid cells", not how big. Has to be measured |
| No number for what shadows cost | "at the end, and with a number" — the number does not exist yet |
| No value for `k` in `1 - e^(-k*dt)` | §11 prescribes the formula, not the constant; `math.rs` takes it as a half-life parameter, to be determined per use case |

These six points belong in `docs/QUESTIONS.md` as questions, not in the code as assumptions.

Related: [architecture.md](../architecture.md) · [multiplayer.md](../multiplayer.md) · [conventions.md](../conventions.md) · [environment.md](../environment.md) · [QUESTIONS.md](../QUESTIONS.md) · [ROADMAP.md](../ROADMAP.md) · [environment.md (lessons)](environment.md) · [workflow.md](workflow.md) · [supervision.md](supervision.md)
