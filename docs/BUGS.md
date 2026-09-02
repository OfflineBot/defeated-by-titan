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
**⚠️ HALF-FIXED 2026-09-01, and the ✅ that stood here for an hour was over-broad.** An adversary
confirmed the PAIR half independently in the running game — a real two-anchor stand-off costs
**0.398 gas and billing stops after 4 ticks** — and then reproduced the headline symptom **to the
exact table number on ONE rope**: hooked to the wall, resting on the stone, `Ctrl` held 10 s costs
**59.766 gas** while height and speed sit at 37.000 / 0.000 for 9.05 s unchanged. `DBT_GAS_LEDGER`
attributes every tick of it to `reel`. Cause: `src/vector/gas.rs:219` rule 1 still asks
`to_anchor.length() > min_rope_m` — the DISTANCE — while `limits.max` is already pinned at
`min_rope_m` and cannot shrink; the stone holds the player 3 m off, so the test stays true forever.
**The fix's own diagnosis ("asking about distance where it should be asking about progress")
applies to rule 1 verbatim and rule 1 was not touched.** Open for the one-rope case.

The pair half, as shipped: `vector::gas::reel_has_effect` asks [`shared::rope::pair_budget_m`], the
pair rule's **own** function, instead of asking about the distance. Red-then-green, captured:
with the predicate broken back to the old shape a second of held `Ctrl` in the 57.709 m / 12 m /
48 m stand-off cost **5.9999 gas for rope nobody got**
(`tests/vector_gas.rs::f018_a_reel_the_pair_rule_refuses_costs_nothing_and_one_it_allows_costs_the_file`);
with it restored, **0.0000**, while the control at 2.459 m separation still pays the file's
`gas_reel_per_s`. Four unit tests in `src/vector/gas.rs` (**`--lib`**) carry the `n = 2` case, the
boundary at ±1 mm, and the `n = 1` non-regression.
⚠️ **One writer, and no second copy of the rule**: the expression moved out of
`player::rope::hold_the_pair` into `shared/rope.rs` unchanged, because `vector -> player` is not
on the allow list and two implementations of one question drift. The `Q-079` rollback point is
still the one line in `hold_the_pair`.
⚠️ **It does NOT ask whether the reel *took* rope**, which is B-014's own wording below and is a
**deadlock**: `vector::reel` writes `ReelSpeed` off this very grant, so an unbilled reel takes no
rope and would never be billed again. The question has to be geometric.

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

## B-021 — ⚠️ HALF-REFUTED 2026-09-01: ACT 4 was the wall — but the energy dump is REAL elsewhere

> 🔴 **The refutation below is right about ACT 4 and wrong as a closure.** An adversary replicated
> `scripts/f003-wall.txt` ACT 2 — the project's own showcase swing, arc bottom matching the
> script's recorded numbers — and measured **29.9 % of specific energy gone in ≤ 3 ticks**
> (t=218 `v=29.614` → t=221 `v=9.506`, 1261.2 → 883.6 J/kg) **while GAINING 0.488 m of height**,
> rope on, gas flat at 15000, and the swing recovering to 26.7 m/s afterwards — **not** `B-031`'s
> dead-stop signature (37.456 → 0.026 in one tick, then pinned). So the ACT-4 instance is the
> wall; a distinct tangential energy sink exists on a clean swing and is unexplained. **Open,**
> with `scratchpad/adv-b/b21-tick.txt` as the repro.

> 🔴 **The diagnosis below is wrong and the measurement under it is right.** One instrumented run
> of `scripts/f005-feel.txt` ACT 4 at the y = 20 stand (`docs/FINDINGS.md` FIND-221):
> the loss is **one tick, not a quarter second**, it is **tangential** (33.074 → 7.898 across the
> rope, radial part 0.0219 — a `DistanceJoint` applies its impulse *along* the rope and cannot do
> this), avian reports a **contact on exactly that tick**, and `z` is **pinned at −97.680** for
> the next 70 samples: `maps.ron`'s *"z = +350 .. −97.5 the district"*, i.e. the main wall.
> `rope_drive`'s look gate is 0.0000 throughout and **one** arm is anchored, so the two suspects
> named below — the `Drive` joint (`Q-058`) and the pair rule (`B-013`) — are not in the path at
> all. **This is `B-031`**, the silent wall collision, and `B-031`'s resolution is the one that
> closes it.
>
> ⚠️ **Why the controls below could not see it:** they varied the stand **height** (y = 16/20/24)
> against a **vertical** face. `docs/lessons/fixtures.md` §4 — a sweep's size is not its coverage.
> The 6.6 m of air is real and it is under the wrong direction.

## B-021 (as filed) — a taut rope dumps 23.7 m/s in a quarter second with 6.6 m of air underneath

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


## B-029 — the aim marker drifts off the crosshair while you fly, up to 420 px, and it is a schedule order

**FIXED 2026-09-01** · `F-026` · `docs/FINDINGS.md` FIND-217

**Symptom (the user, 2026-08-29, second time):** *„es bewegt sich immernoch also die target
seile"*. The two arm markers sit on the crosshair standing still and slide off it while moving —
worse the closer he is to what he is aiming at.

**Repro, and it returns a number rather than an impression:**

```bash
DBT_AIMTRACE=1 ./target/debug/defeated_by_titan --offscreen --screenshot /tmp/x.png \
    --script scripts/f026-turn.txt --ticks 620 2>&1 | grep AIMTRACE
```

Phase D (`t = 252..435`, boosting), Left arm, `dglyph`: **median 14.00 px, p95 48.74, max 419.98**,
with a 392.92 px jump between two consecutive frames. Standing, panning and flicking: 0.00 px in
every phase — **the still fixtures could not see it.**

**Cause:** `vector::aim` ran in `SimulationSystems::World`, before `Integrate`, so the ray started
at the eye from the *previous* step while the HUD projected its answer through the camera at the
*end* of the step. One step of eye travel, expressed as an **angle** (`v·dt/d`), so it diverges as
the distance to the target shrinks: at t = 432 the target was a wall 0.35 m ahead at 29.4 m/s.

**Fix:** `src/vector/mod.rs:88` — `aim` moved to `SimulationSystems::PostStep`. After: median
0.00, p95 0.00, max **0.01 px**. `vector::hook` reads `ArmAim` from a `Transform` no system has
touched in between, so the rope is unchanged; what it gains is that it now fires at the point the
frame in front of the player had actually drawn.

**Red tests, both seen red and both red again on the one-line revert:**
`tests/hud.rs::f026_the_marker_stays_on_the_cursor_while_he_is_flying` ·
`tests/vector_aiming.rs::f002_the_ray_starts_at_the_eye_the_frame_is_drawn_from`
**Pictures:** `docs/images/f026-marker-in-flight-before.png` → `docs/images/f026-marker-in-flight.png`

**Related:** `FIND-212` (the 16 px stand-down, the other half of the same complaint) · `FIND-217`

---

## B-028 — a rope fired at the quay from the river lifts you exactly to the surface and then drops you back

**2026-09-01 · `[offlinebot]` · open · found while building `F-water`, and it is NOT a water bug**

### Repro

```bash
cargo build
cat > /tmp/b028.txt <<'TXT'
warp -70 20 60
wait 3.5
look -90 45
hook left 8.0
wait 0.4
key ctrl 6.0
wait 1.0
mark B028 t+1.0
assert height > 99
wait 0.2
mark B028 t+1.2
assert height > 99
wait 0.2
mark B028 t+1.4
assert height > 99
wait 0.6
mark B028 t+2.0
assert height > 99
wait 2.0
mark B028 t+4.0
assert height > 99
end
TXT
./target/debug/defeated_by_titan --headless --script /tmp/b028.txt --ticks 700 2>&1 \
  | grep -E 'MARK|measured' | head -12
```

### Measured, `[offlinebot]`, 2026-09-01

```
t+1.0   y = -0.841     climbing
t+1.2   y = -0.600     <- exactly the water surface, and the peak
t+1.4   y = -1.190     falling back
t+2.0   y = -1.269
t+4.0   y = -1.294     settled at the float equilibrium again
```

The rope is anchored the whole time (`assert rope == 1` holds), `Ctrl` is held for six seconds,
and `Velocity` reads **28.079 m/s** throughout — `game.ron: vector.reel_speed_m_s` to the digit.
So the winch is running at full speed and the body advances **0.7 m in 1.2 s** and then loses it
again.

### What it is, as far as it has been measured

The anchor is on the east quay's inner face or on the city behind it, i.e. **+X of the player**,
and the quay wall stands between the two. `locomotion::rope_winch` re-writes the full
`reel_speed_m_s` along the rope on every tick; the wall eats the horizontal component, the body
grinds up the face, and at the lip it stops and falls back. The 28 m/s in the readback is the
velocity the winch **asked for**, not the one the body got — the same shape as `FIND-103`: the
instrument agrees with the code and both are wrong about the world.

### What it is NOT

Not the swim rule. `water.ron: swim.drag_per_s` slows the climb but does not stop it, and the
same reel out of the water works: the boost route in `scripts/f-water.txt` ACT 3b takes the body
from `-1.291` to `+9.814` in 1.2 s on gas. The bug is a rope-against-a-wall behaviour that the
channel merely makes easy to hit — an anchor 5 m away with a 4.4 m wall in front of it is
exactly what the river hands you.

### Why it is filed and not fixed

Foreign territory: it is `player::locomotion::rope_winch` / `vector::rope`, which this round does
not own. **The evidence script does not go through it** — `scripts/f-water.txt` proves the hook
fires from the water (ACT 3a) and leaves on gas (ACT 3b), so the acceptance is met without
standing on the broken path. `docs/FINDINGS.md` FIND-216 §2 has the numbers.

---

## B-030 — a schedule cycle in a test allocates 4.63 GB and OOM-kills the machine

**FIXED 2026-09-01** · `docs/FINDINGS.md` FIND-218 · fallout of `B-029`/`FIND-217`

**Symptom (the user's machine, 2026-09-01 09:18):** a test binary is OOM-killed and takes his
tmux session with it.

```
Out of memory: Killed process 274850 (vector_hooks-eb)
  total-vm: 81 085 156 kB   anon-rss: 25 289 420 kB
  task_memcg=/user.slice/.../tmux-spawn-45512ce8-....scope
```

**Repro (30 s, and it dies alone under the cap):**

```bash
nice -n 15 ionice -c 3 cargo build --test vector_hooks -j 3        # uncapped: mold needs 8 GB VM
( ulimit -v 6291456; ./target/debug/deps/vector_hooks-* \
    f001_a_hook_whose_carrier_disappears_releases --exact ) 2>&1 | grep 'memory allocation'
# memory allocation of 4966055936 bytes failed
```

**Cause — one line, and it is not in `src/`.** `FIND-217` moved `vector::aim::aim` from
`SimulationSystems::World` to `SimulationSystems::PostStep` (working tree, uncommitted).
`tests/vector_hooks.rs:100` and `tests/vector_rope.rs:97` still registered their `AimPoint`
injector as `force_aim.in_set(SimulationSystems::World).after(aim)`. The six stages are
`.chain()`ed in `src/lib.rs`, so `World → Intent → Drive → Integrate → PostStep → World`
**closes a dependency cycle in `FixedUpdate`**.

**Why one line costs 25 GB:** `Schedule::run` answers a build error by calling
`ScheduleBuildError::to_string`, and for a cycle that is `dependency_cycle_to_string`
(`bevy_ecs-0.19.0/src/schedule/error.rs:174-206`), which writes **one block per simple cycle**
into a single `String`. Measured here: **2 290 028 cycles**, a `String` doubling its way to a
**4 966 055 936-byte** `realloc` (= 37 · 2²⁷ — a `Vec` doubling, not a product of anything),
× 8 test threads in one binary = the 25 GB. The failing test was not special: **every** test in
`tests/vector_hooks.rs` builds the same app.

**Fix:** drop the `.after(aim)` in both files. With `aim` last in the tick, `World` at the start
of the next tick already **is** the last writer before `Intent` — the edge bought nothing.

**Guard (two, because the shape will come back):**
- `tests/vector_hooks.rs::schedules_build_or_explain` builds every schedule before the first
  `update()` and prints at most 3 cycles of at most 12 nodes. Bounded by a constant.
- `tools/test.sh` runs the suite under `ulimit -v 6291456` per process — compile first
  **uncapped** (mold reserves 8 GB of VM and dies under the cap with a link error that reads
  like a memory error), then run capped.

**Red-then-green control:** re-adding `.after(aim)` on the fixed tree fails in 11.4 s with
`schedule FixedUpdate does not build — 2290028 flat before/after cycle(s)`, peak RSS **529 MB**,
exit 101. Removing it again: `31 passed`, 7.2 s.

---

## B-031 — a swing whose plane is normal to the wall dumps 37.5 m/s in ONE tick against the gate hood

**2026-09-01 · `[offlinebot]` · not a regression — the old map read the same shape · open**

`assets/data/maps.ron` argues the wall's cornices as physics: *"the bottom of the pendulum lies
vertically under the ANCHOR… two columns and a crossbeam put the anchor over open ground."*
`scripts/f003-wall.txt` ACT 2 flies that at **yaw 60**, along the wall, and it is excellent
(15.318 m arc bottom at 42.282 m/s, back up to 26.110 m, nothing struck). At **yaw 0** — the
obvious thing a player does, straight at the gate he is flying through — the far half of the arc
ends in the hood over the passage.

### Repro

```
warp 0 40 -80        # ACT 2's stand, 24 m short of the inner gate, 40 m up
look 0 -11           # yaw 0 = straight at the wall, instead of ACT 2's yaw 60
wait 0.1
hook right 2.0
wait 0.95            # then `mark` + `assert speed > 99999` + `wait 0.0166` x 40, one per tick
```

Tick by tick, and it really is one tick — the marks above are `1/60 s` apart:

```
h=21.910  v=36.368      still falling into the arc
h=21.316  v=36.946
h=20.788  v=37.456      <- the last tick before the hood
h=20.532  v= 0.026      <- 37.456 -> 0.026 m/s, 99.93 % of the speed, in ONE step
h=20.533  v= 0.011 ... 0.001 for the next 29 ticks
h=20.515  v= 1.066      the 2 s hold ends, the rope lets go, and he falls to the street
```

No sound, no message, no HUD state change — the run reads exactly like a rope that went slack.

### Why it is on the list even though it is old

Measured on `HEAD`'s map at the same stand and yaw: **38.407 → 0.346 m/s** at `y = 19.403`. So
the wall did not cause it and the deleted gantries did not prevent it. What changed is that the
wall's own header now *claims* the property in prose, and the claim is only true in one plane.
`tests/world.rs::f003_every_cornice_on_the_wall_hangs_over_open_ground` measures the strip
**perpendicular** to the wall (3.50 m under the ladder, 5.50 m under the gate cornice) — a
necessary condition for the pendulum's bottom, and no condition at all on the far half of the
arc, which curves back into the face whenever the swing plane is normal to it.

### What would close it — his call, not mine (`docs/QUESTIONS.md`)

A body that loses 37.4 m/s into stone should say so: the collision needs a **verdict** the way a
cut does, and the same instant is where a wall-run or a kick-off would go. Until then the honest
statement is that the wall rewards the sideways swing and punishes the straight one silently,
and `f003-wall.txt`'s header says so in words with no assert behind it.

---

## B-032 — `net_a_peer_on_a_real_socket_drives_his_own_body` is flaky under parallel load

**2026-09-01 · [offlinebot] · seen once in a full-suite run, then green 3 of 3 alone**

`tools/test.sh` reported `tests/multiplayer.rs:424` failed inside the full suite. Re-run on its own
with the same binary and the same 6 GB cap: **ok, three times in a row.**

The name says why: it **binds a real socket**. A test that takes a real port cannot be run beside
an unknown number of other test binaries — `cargo test` runs binaries in parallel and each one
spawns its own threads, so two runs can collide on a port or on the scheduler.

**No repro under load yet**, so this is filed rather than fixed, and it is **not** a blocker: the
thing it guards (a peer on a real socket moving his own body) is proven every time it runs alone.

⚠️ **Do not "fix" it by deleting the socket.** The whole point of that test is that it is not a
loopback fake — `docs/multiplayer.md` rule 4 is the reason the socket is real. The honest fixes are
a port chosen from the OS (`:0`) rather than a fixed one, or `#[serial]`-style exclusion.

**Related:** `B-012` (the other unexplained flake, also under load) · `F-175` · `docs/multiplayer.md`

## B-034 — `S` on a taut rope does not brake: 7.630 m/s where the act promises under 2

**2026-09-01 · `[offlinebot]` · repro is an evidence script that has been red since it was
written · open, and the decision is the user's (`docs/QUESTIONS.md` Q-089)**

### Repro

`scripts/f176-pull.txt`, at its documented invocation (`--ticks 900`), ACT 2:

```
line 133: assert Speed < 2 — measured 7.630
```

The act is §3F R1 in the user's own words — *„seil ist vorne und ich laufe zurueck"* — and its
comment states the band: `run_speed_m_s` is 6.0, *"anything at or over it is a player who walked
straight out of his own rope"*. It reads **7.630**.

### The mechanism, and it is a latch (`docs/FINDINGS.md` FIND-221b)

`integrator::ground_top_speed_m_s` = `run_speed_m_s − gravity/hz` = 6.0 + 0.5333 = **6.533 m/s**.
`integrator::movement_state` makes an **anchored** player `Tethered` the moment his ground speed
passes it — and `Tethered` takes him out of `locomotion::ground_locomotion`, which is the only
thing `S` acts through, while putting him **into** `air_control`'s `in_the_air`, where
`rope_winch`'s always-on pull runs: free, keyless, up to `drive_idle_speed_m_s` = 12 m/s of
closing speed. Over the latch the pull holds him; over the latch the legs may not brake him.
7.630 sits between 6.533 and 12.0.

🟨 **The two numbers and the two predicates are measured and read; `MovementState` itself was not
sampled at that tick.** Whoever closes this samples it first — one `assert` is not a mechanism.

### Why it is not fixed here

Two of the user's own instructions meet and disagree, and neither is mine to overrule:
*„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"* (`FIND-172`, which is why the
winch has no key) against §3F R1 (which asks for a brake). → `Q-089`.

**Related:** `FIND-172` · `FIND-221b` · `Q-089` · `F-005` `F-006`

---

## B-012, the unexplained half — 40 of 40 green against a pinned binary, so it is LOAD or nothing

**2026-09-01 · [offlinebot] · the protocol step the entry prescribed and nobody had run**

⚠️ The entry itself sits in `docs/archive/BUGS-closed.md` because its *second* observation was
explained (the mid-round gravity commit) and the archiver read "PARTLY RESOLVED" as resolved. **The
first observation — `11 of 19 asserts failed` twice at 23:15 UTC with identical MARK ticks — was
never explained**, and this is its step 1:

`scripts/f175-loop.txt`, 40 consecutive runs, one pinned binary (`80ab69ed`), quiet machine:
**40 × exit 0.** So the flake does not live in the script, the binary or repetition. What remains
is the condition both observed failures shared: **heavy parallel load** (a foreign `nice -4` build
then; a full-suite run for `B-032`'s sibling). Step 2 — reproducing under artificial load — is
**deliberately not run while he is at the machine**; it means degrading his desktop on purpose.

**Related:** `B-032` (same shape: green alone, red once under the full suite) ·
`docs/archive/BUGS-closed.md` B-012

---

## B-036 — the save file is last-writer-wins, and any two game processes race on it

**2026-09-01 · [offlinebot] · found by the progression adversary, controlled, and it had already
bitten the real career the same day**

`save::record_outcomes` and `save::spend_gear_points` write the **whole file** from the in-memory
profile. Two processes on one save dir: the fast one spends a gear point and the file reads
`gear:{"speed":1}`; the slow one wins its sortie 6 s later and writes **its own** in-memory
profile — final file `gear:{}`. **The point is gone, both logs clean.** Control: the identical two
scripts run *sequentially* keep both. Concurrency is the only variable.

**It bit the same day:** `tools/corpus.sh` ran six games in parallel against `saves/player-1.ron`,
and the sweep sent the career **backwards** — 474 → 470 sorties, 798 xp, **two allocated gear
points erased**. A tally re-earns itself; a gear allocation is the player's decision.

**Mitigated at the tools** (2026-09-01): `corpus.sh` gives every run its own `DBT_SAVE_DIR` in
`mktemp -d`; `tools/test.sh` runs the whole suite against one scratch dir. The game's own race —
two real instances, or a future second player — **is not fixed** and needs read-merge-write or a
lock in `src/save/`.

⚠️ **Do not fix it with write-temp-then-rename alone.** That makes the clobber *atomic*; it does
not make it *right* — the slow writer still erases the fast one's spend, just cleanly.

**Related:** `docs/multiplayer.md` (a second player makes this the common case) · `F-120` `F-125`

---

## B-038 — rebind the left hook and the HUD still says `Q`: the arm-marker letters ignore `PlayerSettings::binds`

**2026-09-01, stream B.** `F-172` made the hook keys rebindable (`PlayerSettings::binds`,
keybinds page, `saves/settings.ron`), and `net::local::read_input` fires off the bind —
measured: `tests/input.rs::f172_a_rebound_key_fires_the_arm_and_the_old_key_is_dead` is green.
**`hud::arm_aim` still hardcodes the letters** (`arm_aim::key_label(side)` returns `"Q"`/`"E"`
as literals), so after a rebind the marker names a key that fires nothing — exactly the failure
the old grep-test called *"worse than no label"*.

**Repro** (unit-level, no window needed):
1. `PlayerSettings::binds.set(BindAction::HookLeft, KeyCode::KeyM)` — the settings screen does
   this on any rebind, and the file brings it back at every start.
2. `arm_aim::key_label(Side::Left)` still answers `"Q"`; the letter drawn beside the left
   marker is that answer.
3. Meanwhile `M` fires the arm and `Q` is dead (`tests/input.rs`, above).

**The fix is one read**: `arm_aim`'s label systems take `Res<PlayerSettings>` (`shared`, free
for every domain) and answer `shared::settings::key_label(binds.hook_left)`. Not done by
stream B because `src/hud/arm_aim.rs` is another stream's open file today. Until then
`tests/hud.rs::f171_the_marker_letters_are_the_keys_that_fire_the_arms` pins the letters to
`KeyBinds::DEFAULT` and carries this bug's number in its comment — whoever fixes it should
point that test at the LIVE settings and watch it hold.

**Related:** `F-172` · `FIND-224` · `docs/FINDINGS.md` FIND-129 (the marker's own contract)


---

## B-037 — under §5D, `W` while looking away from every anchor is a dead grant: no thrust, no bill

**2026-09-01 · [offlinebot] · found reading `vector::gas` for the §5D rebuild · NOT fixed — foreign file**

**Symptom.** Hooked, in flight, `W` held, every anchored arm behind the look (`l̂·r̂ ≤ 0` for
all arms): the player gets `air_accel_m_s2` = 10 m/s² of free-air thrust and **none** of the
drive's 52 m/s chase — and pays nothing. Under the old model that was correct (the drive's own
look gate returned exactly `Vec3::ZERO` there, and the cost followed the effect). Under §5D
rule 2 (*„aber w geht in die richtung"* — the LOOK's) the pure function
`player::locomotion::rope_drive` thrusts along the look at ANY angle to the rope, so the grant
is now refusing a key that would have an effect.

**Cause, one line.** `src/vector/gas.rs::steer_has_effect` — the `move_y > 0` half requires
`look_dir.dot(direction) > 0.0` for some anchored arm (the closing condition of the function,
line ~168). That predicate was `rope_drive`'s old zero-set, copied so the bill matches the
effect; `rope_drive`'s zero-set changed on 2026-09-01 and the predicate did not.

**Repro.** Pure-function pair, no app needed:
`rope_drive(&[anchor 180° behind the look], look, 0, 0.0, 1.0, v, t)` is **non-zero** since
§5D (`tests/player.rs::f149_a_hooked_player_who_holds_nothing_is_not_driven_at_all`, the
`behind` half measures it), while `steer_has_effect(same args…)` is **false** — the two
answers about "does W do something" disagree. In the app: `air_control`'s Drive arm runs on
`grant.steer || gas.is_empty()`, so the mismatch is visible as a `W` that works on an empty
tank (the exception path) and dies the moment the tank refills.

**The fix belongs to `vector`** (rule 3: one domain, one writer; and the header of
`steer_has_effect` itself says the question is "whether `rope_drive` returns `Vec3::ZERO`").
The §5D-true predicate for the `W` half: an anchored arm exists and the player is not already
at `drive_speed_m_s` along the look — the angle test goes. ⚠️ Billing gets BROADER (W on a
rope always costs), which is a gameplay change to name to him, not to sneak.

**Related:** `FIND-223` · `FIND-150` (idle costs nothing — untouched: the winch is not billed)
· `docs/NEXT.md` §5D rule 2

---

## B-039 — anchor points on dressed houses hang in the air (2026-09-01, FIXED)

**2026-09-01, dressing stream — FIXED same day.** The user, at the controller: *„zudem sind
die anchor points bei häusern in der luft! das passt nicht."* Measured before the fix (the
anchors-air round, 1584 logged bites over the 15 dressed houses of the shipped map): offset
from the bite to the nearest DRAWN surface **median 1.07 m, p90 2.18 m, worst 2.84 m**. Three
causes, three fixes, one round:

1. **Orientation transpose** — `plan_blocks` swaps frontage/depth for a house fronting along
   z, `render::model` drew every model at one fixed yaw. On 5 of 15 houses two visible walls
   stood 1.57–1.82 m INSIDE the collider, the mesh poked 0.31 m OUT through the other two.
   Fixed by the quarter turn: `BlockPlan::yaw_rad` → `shared::ModelYaw` → scene, anchors AND
   collider turn by one quat.
2. **Envelope slack** — the authored `hit` pair sits 0.23–0.30 m outside the visible mesh on
   every side of every a-083 file; the collider was that envelope.
3. **Roof shape** — one cuboid at full width to the ridge under a roof that slopes in above
   ~70 % of the height; roofline bites (the ones a player AIMS at) hung 1.3–2.8 m in the air.
   2+3 fixed by mesh-derived compound colliders (`art.ron: hulls`: wall boxes at the dominant
   wall planes + a convex ridge wedge), rounded INWARD on every disagreement.

**Repro/red:** `tests/dressing.rs` — captured red 2026-09-01: *"house_39_6 (house_town, yaw
0.00): the drawn mesh covers 10.10 x 8.77 m but the collider is 8.77 x 10.10 m"* and *"the +x
wall bites at x = 3.95 m — the drawn plane is 3.21"*; fix removed for one run (yaw forced 0.0)
→ same red; restored → 6/6 green. In-game: `scripts/f-dressing.txt`, 8 asserts held — the wall
bite lands at x 51.90 (= the drawn plane to the centimetre), the old collider face was 52.67.
**After-fleet (same probes, same instrument): median 0.03 m, p90 0.07 m, worst 1.21 m (1145 bites)** — see
`docs/FINDINGS.md` FIND-225 for the per-face table. Pictures:
`docs/images/b039-rope-before.png` (the anchor dot in open sky over the roof) vs
`docs/images/b039-rope-after.png` (same stance, same aim, the dot ON the drawn roofline).

**Still open in this bug's shadow:** remnants (ruin/rubble) and the 12 dressed props keep
their envelope cuboids (no `hulls` rows yet — smaller, lower, no roofline lane); dormers and
chimneys sit OUTSIDE the compound (conservative direction, a bite behind them lands a hand's
breadth under the drawn surface); `debug::gizmo` still outlines the envelope `Body`, not the
compound.

**Related:** `FIND-225` · `Q-093` (which way the facade faces is still undecided) · Q-067/Q-078
(the rejected candidate: NO return to hardcoded anchor lists)

## B-041 — reserved by the hook-toggle round 2026-09-01 (unused unless something bug-shaped appears)

### B-039 · Amendment (2026-09-02, adversarial round) — the red trail, stated precisely

The yaw break control re-reddens **2 of 6** dressing tests (the two plan-level ones); the
four collider-fixture tests hardcode their own yaw and are immune. And the quoted *"+x wall
bites at x = 3.95"* red belongs to the **pre-hulls** state — the yaw-forced run cannot
produce it. Both reds are real and both were reproduced by the adversary (the yaw red with
byte-identical restore); they are two different controls for two different halves of the
fix, and this entry originally let them read as one. Full amendment: `FIND-225` amendment
in `docs/FINDINGS.md`.

## B-040 — a hook bitten while standing pulled nobody: the always-on pull was gated `in_the_air` (2026-09-01, fixed the same day)

**His report** (`docs/NEXT.md` §5E-b, verbatim): *„und aktuell wenn cih mich hooke werde ich
nicht autmoatisch rangezogen! das fehlt noch! aktuell muss ich noch in die richtung schauen
bewegen! fixe das noch!"* — hooked, standing, looking at the anchor, and nothing happens until
he produces air time himself.

**Cause, and it was a decision, not an accident:** `FIND-172`'s free pull was deliberately
gated `in_the_air` (`Q-055`/`Q-056` — a hooked player in the hub keeps his legs), and
`ground_locomotion` assigned the XZ velocity of every grounded player each tick, so even an
ungated winch would have been erased (`FIND-182`'s elevator was that fight, won the wrong way
around). §5E-b overturns the decision; the hub worry is defused by release being one tap of Q/E.

**Repro/red — captured 2026-09-01, standing player, anchor 40 m straight overhead, no key:**
`tests/player.rs::f176_a_hook_bitten_while_standing_pulls_the_player_off_the_ground_at_once` —
*"90 ticks after a hook bit 40 m straight overhead the standing player has not left the ground
(y still ≈ -0.000) — the always-on pull is still gated `in_the_air` (FIND-172/Q-056)"*.

**Fix:** one predicate, two readers — `player::locomotion::ground_pull_live`; `air_control`
runs the winch and flight controls for a grounded pulled body (`in_the_air || pulled`),
`ground_locomotion` `continue`s past the same bodies so `ground_step` stops deleting what the
winch builds. Green: airborne after **31 ticks**, `MovementState::Tethered`, and the deletion
control (`drive_idle_speed_m_s: 0`) stands still.

**One-line break control:** `ground_pull_live`'s `t.speed_m_s > 0.0` flipped to `< 0.0`
(= the old gate's observable behaviour) re-reddens the headline test with the message above;
restored, 5/5 `f176_*` green. `S` through the bite stays `Grounded` all 120 ticks and walks at
6.0 m/s (§5D rule 4 survives).

**Related:** `FIND-226` (the numbers, the 68.96° contact-break line) · `Q-094` (drag-not-lift
below the line is a decision on his desk) · `B-020` (the 0.52 s standing escape is measured
there, not claimed) · `scripts/f176-pull.txt` ACT 5.

## B-018 — the step-walls entry was lost in a filing collision; the class dissolves under §5E (2026-09-02)

The terrain-design scout of 2026-09-02 found that B-018 (terrain risers landing as
unclimbable step-walls) has NO standalone entry left in this file: its number went under in
the B-021 filing collision (see the note inside B-021, lines ~214-246), and the content now
lives only in FIND-214 riser table and the NEXT.md §5E references. Disposition instead of
restoration: the §5E smooth-terrain round replaces quantised risers with one continuous
triangle surface under a `max_grade` guard, so the step-wall CLASS loses its carrier. When
§5E lands, B-018 is retired with it; if §5E is ever rolled back, this paragraph is the
pointer to rebuild the entry from FIND-214.

## B-041 — a hook pressed in the same tick as a `look` fires along the PREVIOUS look (2026-09-02, OPEN)

Found while attributing `f025-chain`'s 12/36 (FIND-228). For exactly ONE tick the gun and
the eye disagree: `vector::aim::aim` runs in FixedUpdate `PostStep` (moved there by the
2026-09-01 marker fix, FIND-222), and `vector::hook` consumes the previous tick's `ArmAim`
and never re-casts (its own decision 6). A `hook` in the same tick as a `look` therefore
flies along the OLD look; 3 ticks of separation already behaves fresh.

**Repro:** `scripts/b041-stale-look.txt` — 4 of 7 asserts red today, exit 1, captured
2026-09-02: legs A/C red in Toggle (`assert Rope == 0 — measured 1.000` at the wall the
fresh 45° ray must clear; `assert Rope == 1 — measured 0.000` where the fresh level ray
must bite), legs E/F the same under `hook_fire 0` — the staleness is not the toggle's.
Waited controls (0.3 s) and the 3-tick window leg are green. Double dissociation, so it is
the look's AGE and nothing else. The script pins the CORRECT behaviour and goes green only
when this is fixed.

**Blast radius:** every corpus script that puts `hook` on the line after `look` with no
wait — the f025-chain legs 2..5 shape — and, in play, a hook clicked mid-flick lands where
the camera was 16 ms ago while the marker (PostStep-exact since FIND-222) shows the NEW
aim: what you see is not what you fire, one tick wide.

**Fix warning:** the naive fix is re-ordering aim before hook inside one tick — schedule
surgery in the exact class that produced the B-030 cycle OOM. `tools/test.sh` runs capped
and `schedules_build_or_explain` guards it, but the fix round must treat the ordering as a
claim and re-run `scripts/b041-stale-look.txt` (green = fixed) plus the f025 legs. Do NOT
re-pin f025-chain first (FIND-228: half its `look` lines are dead letters under this bug —
a pin would aim at the bug, the FIND-096 mistake).

## B-042 — titan hit zones: "unmöglich diese überhaupt zu treffen" (2026-09-02, OPEN, cause unknown)

His words, verbatim: *„ah und die hittboxen der titanen waren sehr schlecht. unmöglich
diese überhautp zu treffen."* A symptom he reports is real even when the code looks right —
every one so far has been.

**Context that makes it plausible:** the titans were DOUBLED (scale.ron, his §5A order
"gerne doppelt so groß") and the rig got real body segments with the neck window. Nobody
has ever measured whether hit zones, blade sweep reach and the DRAWN mesh scaled
consistently — this is the house disease (collider ≠ drawing, B-039) on the titan rig.
Candidate mechanisms to measure, not argue: hit-zone capsules offset/undersized vs the
doubled mesh; blade sweep length not scaled with the target; cortex window ticks too tight
at the new geometry; `width_fraction` 0.20 cut after the doubling.

**Repro:** none yet — that is the first deliverable. The instrument exists in shape:
f030-hitbox scripts + the B-039 fleet method (ray/sweep distance between the drawn glb
surface and the collider that actually registers). Binary state unknown for his session —
attribute against the CURRENT tree first.

## B-043 — the rope anchors in open air, again — this time NOT on a dressed house (2026-09-02, OPEN, cause unknown)

His words, verbatim: *„auch das seil war einfach in der luft verankert!"* — after the
B-039 house fix landed in the tree. Three candidate causes, each with an existing measured
foothold, none confirmed for his sighting:
1. **The documented B-039 debt**: remnants, the 12 props, dormers and chimneys still carry
   fat envelope colliders — a hook there bites an invisible box. Measured oversize exists
   in the B-039/FIND-225 record.
2. **B-041**: a hook in the same tick as a `look` fires along the PREVIOUS look — the
   anchor lands where the camera was, not where it is, which reads exactly as "in der
   Luft". Repro: scripts/b041-stale-look.txt (4/7 red).
3. Titan colliders vs drawn titan (see B-042 — if he hooked a titan, same disease).
**First deliverable is attribution**: an in-repo fleet-style sweep measuring
anchor-to-nearest-drawn-surface distance across remnants/props/titans, plus which of his
plausible targets have envelope-only colliders.
