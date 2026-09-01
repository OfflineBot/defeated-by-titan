# BUGS — every bug with repro, evidence, cause, fix and test

Updated: 2026-08-10

> **A bug without evidence is a rumour — and uncertainty is a defect.**
> No "should work now", no "should be fine", no "probably fixed". Either you have it
> **evidenced**, or you write down that you do not (`prompts/init.md` §9).


> 📦 **Closed, superseded and won't-fix bugs live in**
> **[`archive/BUGS-closed.md`](archive/BUGS-closed.md)**, with a one-line index and every repro
> kept. They moved on 2026-08-29 so this file answers *what is broken right now*. **Nothing was
> deleted.**

---

## A bug report needs four fields — otherwise it is not one

| Field | what has to go in |
|---|---|
| **Repro** | the exact command: `cargo run -- --headless --script scripts/hook-edge.txt`, plus seed / coordinate / view direction from the F3 overlay and the **machine** (`[debian]`/`[cachy]`). Whoever cannot reproduce it cannot check it. |
| **Evidence** | screenshot in `docs/images/`, a log excerpt **or** a number (measured 34 m/s, expected ≤ 12). Not "looks wrong". |
| **Expectation** | what should happen instead — and **where you know that from** (RON line, doc paragraph, design decision). |
| **Cause** | `file:line`, as soon as it is known. As long as it is missing: **"cause unknown"**, not guessed. |

**No repro, no fix.** A bug without a repro is recorded as *unevidenced* and **not
repaired** — a fix for something you have never seen is a change without a reason, and you
cannot refute it afterwards either.

## A fix without a red test is a guess

The order is **not negotiable**:

1. **Write the test that shows the bug** — and run it until it is **red**. A test that was
   never red only proves that it compiles.
2. **Fix**, until it is green.
3. **Take the fix out again** and watch the test fall over once more. Only then do you know
   that the test checks *this* fix and not something next to it.
4. **Record it here:** cause, fix, test name. If it was a trap somebody can learn from: a
   file in `docs/lessons/`.

For a bug that only the eye sees (movement feel, camera stutter, a hook pointing into
nothing), the evidence is a **`--script` run with `assert`** plus a screenshot before/after.
That is exactly what the script driver in stage 1 is built for.

## Safety in the code: nothing may go wrong quietly

- **No `unsafe`.** Whoever believes they need it writes it into `docs/QUESTIONS.md`.
- **`unwrap()`/`expect()` only with a reason in the comment** — and **never** on data from a
  file or from input. While **loading** the RON, an immediate, loud abort with the file name
  is the *right* behavior (fail fast at startup); in the middle of the game it never is.
- **Physics needs guards.** Rope forces, normalizations and divisions produce NaN/∞ the
  moment a vector has length 0 or a frame lasts 0.5 s. NaN in a `Transform` is the bug that
  looks like "the player has disappeared": check the length before normalizing, clamp `dt`,
  and put a system in `debug/` that **warns once** when a position is not finite.
- **A `panic!` in the game is a bug**, even if it "never" happens. A `Result` swallowed with
  `let _ =` is an error nobody can see any more.

---

## B-014 — `Ctrl` is billed for rope the pair rule refuses to give: **59.766 gas for 0.4985 m**

**2026-08-27 · [offlinebot] · found by an adversarial verifier, self-controlled in one table**

`B-013`'s fix (`hold_the_pair`) stops the reel taking a step that would make two rope maxima
geometrically impossible. **It does not tell the gas ledger.**

`src/vector/gas.rs` gates the reel's cost on

```rust
intent.pressed(Buttons::REEL_IN) && to_anchors_m[..anchored].iter().any(|a| a.length() > min_rope_m)
```

— it asks about the **distance to the anchors**, which stays true forever in a stand-off, and it
has no idea `hold_the_pair` refused the step. And since `Q-058`, `Ctrl` acts **only** through
`limits.max` (`rope_winch`'s own header: *"Ctrl does not come through here any more"*), so a
refused step is **literally no effect at all**.

**Measured**, 600 ticks = 10 s of held `Ctrl`, `gas_reel_per_s: 6.0`:

| | anchors apart | ropes | rope taken | gas | rate |
|---|---|---|---|---|---|
| **the defect** | 57.709 m | 12 / 48 m | **0.4985 m** | **59.766** | **119.9 gas/m** |
| control, same table | 2.459 m | 30 / 30 m | 55.1684 m | 40.939 | 0.74 gas/m |

**The 0.4985 m was already taken at tick 180** — so **seven further seconds of billing moved
nothing at all.** The control shows the existing `Q-050` guard works when both arms reach
`min_rope_m`: billing stops. It simply does not fire for the *pair* case, because the pair case is
new.

⚠️ **This is a movement-killer in a movement game with a gas economy**: a player who holds `Ctrl`
in a geometry the rule refuses drains his tank for nothing and cannot see why — `RopeLength` is
**not drawn, not logged and not a script metric** (`grep -rn RopeLength src/hud/ src/debug/`
returns nothing), so the screen says the same thing whether the rope is moving or not.

**The fix is one predicate, and it must ask the question the reel actually answers:** bill only
when `shorten_ropes` *took* rope. `Q-050`'s guard already has the right shape; it is asking about
distance where it should be asking about progress. ⚠️ **One writer** — `shorten_ropes` decides,
`gas` reads the answer, never re-derives it (`CLAUDE.md` rule 5's corollary, and `FIND-190` is what
happens when two places answer one question).

**No repro script yet.** Write one before fixing: the metric the driver would need is *rope taken
per second*, which does not exist — so this bug is currently invisible to the whole script corpus.

**Related:** `B-013` · `Q-050` · `Q-058` · `FIND-191` · `F-005` `F-018`

---

## B-019 — once the mission has been running, a proven fall-cut hits `Torso`, `ArmLeft`, `LegLeft` — and **never the `Cortex`**

**Found 2026-08-29.** `scripts/f170-objective.txt` line 92 (`assert kills == 1`) reads **0.000**,
and it is **not** the gravity change: it is red at `gravity_m_s2: -20` too, with the file's own
committed numbers.

### The control is one line long, and it is the whole finding

Two scripts, identical but for a single leading `wait`, both at `gravity_m_s2: -20`, both
`--mission tutorial`:

```
              wait 1.5            <- present in one, absent in the other
              warp -1.80 30.8 18.55
              look 0 0
              wait 0.45
              spawn titan husk 0 0 18
              wait 0.90
              slash right 0.40
```

| leading wait | what the blade hits |
|---|---|
| *(none)* | `tick 90: cut titan 2 Torso at 30.00 m/s` · `tick 93: cut titan 2 Cortex at 30.33 m/s` · **kill 1/3** |
| `wait 0.0` | nothing |
| `wait 0.05` / `0.2` / `1.5` | nothing |

The no-preamble row reproduces the header's recorded `30.00 / 30.33 m/s` **to the centimetre per
second**, so the calibration was right and the geometry still is. What changed is that the cut
only lands when the husk is spawned in the mission's **first tick**.

### It is not a timing window — the window is nine ticks wide

At the shipped `-32`, sweeping the pre-slash wait with **no** preamble: every value in
`0.500 .. 0.650` (a 9-tick window) gives `cut Cortex` and a kill. With `wait 1.5` in front,
**no** value of that wait and **no** drop height in 42.0 .. 48.0 m produces a `Cortex` — the
blade lands on `Torso` (38.40 m/s), `ArmLeft` (39.47) and `LegLeft` (42.67) instead. So the
blade is reaching the body and missing **the nape specifically**.

### Hypothesis, explicitly UNPROVEN

`src/titan/brain.rs:897-910` turns a titan toward the player a step per tick, and
`src/titan/rig.rs` keeps *"the Cortex behind the neck"* (F-030, depth 0.55 m). A husk that has
come about presents its nape away from a blade that sweeps `+X` at yaw 0. Why the mission's
warmup should change a husk **spawned fresh afterwards** is exactly what is not established.

**The discriminating experiment nobody has run:** log the husk's yaw at the tick of the cut in
both runs. If they differ, it is the turn; if they do not, it is the pose clock and the culprit
is a global animation phase rather than a per-titan age.

**`scripts/f170-objective.txt` stays RED on purpose.** ⚠️ Its header says the cut is
`scripts/game-full.txt` ACT 2 *verbatim*, so **`game-full.txt` is very likely to carry the same
defect** — it is not in this round's group and was not checked.

---

## B-021 — a taut rope dumps 23.7 m/s in a quarter second with 6.6 m of air underneath

**2026-08-29 · [offlinebot] · measured by the movement group of the corpus re-aim; filed here
because its own filing collided with `B-018` and was silently lost**

`scripts/f005-feel.txt`: the moment the rope goes taut the player falls from **31.745 m/s to
8.092 m/s in 0.25 s**, with `rope` reading `1.000` on every sample and **6.6 m of air still under
him.**

**The two controls that rule out the obvious answers.** A collision would explain it, and gravity
would explain a slower version of it — neither survives:

| stand height | speed at 3.0 s |
|---|---|
| y = 16 | 2.212 m/s |
| y = 20 | 2.212 m/s |
| y = 24 | 2.629 m/s |

**Raising the stand removes every possible collision from the arc and does not give the swing
back.** And it is not a sampling artefact: the post-taut peak *anywhere* in the swing is
**8.535 m/s against 13.334 m/s before**.

⚠️ **This is the swing itself losing its energy**, which is `F-004`'s whole subject — *"Spieler
beschreibt sichtbar eine Bogenbahn; Geschwindigkeit steigt beim Ausschwingen"*. It appeared with
the `DistanceJoint` under `Drive` (`Q-058`) and nobody has separated the joint from the two-rope
feasibility rule (`B-013`) as its cause.

**No red unit test yet, and it wants the `n = 2` shape**: this game has two hooks, and a fixture
that passes one is a fixture for a different function (`CLAUDE.md` rule 5). Write the two-anchor
case first and make the anchors disagree.

⚠️ **Not to be re-aimed away.** `f005-feel.txt`'s bracket stays where it is and the script stays
red on purpose; that file measures the feel of the rope and a bracket moved to fit a 60 % energy
loss would delete the only instrument that can see it.

**Related:** `B-013` · `B-020` · `Q-058` · `FIND-191` · `F-004` `F-005` `F-006`

## B-023 — the world fence is correct, invisible and completely silent: 75 m/s into nothing, and the game says nothing at all

*(claimed 2026-08-29, stream B — being written)*

---

