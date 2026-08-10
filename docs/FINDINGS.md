# FINDINGS — mistakes I tripped over on the way past

Updated: 2026-08-09

**Whoever trips over something that is not part of their own task: write it down, with the
measurement beside it** — so that somebody else can check whether it really is wrong.

**Do not fix it quietly on the way past.** A foreign mistake fixed in passing is a fix nobody
reviewed, and it hides in the diff of a task where nobody is looking for it
(`prompts/init.md` §9c). Format: `FIND-00n <symptom>` + measurement.

---

## FIND-001 — The anchor density in the backlog is not a number

**Symptom:** `prompts/init.md` §2 calls the anchor density „die wichtigste Zahl" (*the most
important number*) in sheet `08_Maps`. In the table it is qualitative.

**Measurement:** all 12 map rows carry `Sehr hoch` (3×), `Hoch` (4×), `Mittel` (4×) or
`Niedrig` (2×) — `grep -o 'anchor_density: "[^"]*"' docs/backlog/maps.ron`. Not one numeric
value.

**Why it counts:** bible 6.2 makes the anchor density the gate for P3 („Traversal-Zeiten
zeigen messbaren Unterschied zwischen Anfaenger und Experte" — *traversal times show a
measurable difference between beginner and expert*). Four words cannot be tuned and cannot be
checked.

**Whose it is:** level design, not the setup. Recorded as a decision in
[`docs/QUESTIONS.md`](QUESTIONS.md) Q-010.

## FIND-002 — The backlog is written for Roblox in several places

**Symptom:** not only the bible — `01_Spielfunktionen` names Roblox building blocks directly.

**Measurement:** `F-003` demands „Oberflaechen mit CollectionService-Tag `AnchorSurface`",
`F-004` names `RopeConstraint`, sheet `05_VFX` names `ParticleEmitter + Beam`, sheet `08_Maps`
measures in `studs`, `T-001` is called „Rojo- und Git-Aufsetzung".

**Why it counts:** `prompts/init.md` §2 governs the passages in the **bible** and says: what
turns up on top of that while working gets **translated and added to
`docs/architecture.md`** — not followed, not ignored, not asked back. Which is exactly what
happened: the translation table in [`docs/architecture.md`](architecture.md) now has four
rows more.

**Whose it is:** done, nothing open. It stands here so the next person knows that the table
grows instead of being fixed.

## FIND-003 — Sheet 11 of the spreadsheet is an independent cross-check, not a data source

**Symptom:** `prompts/init.md` §2 warns that sheet 11 consists of formulas and, without
`data_only=True`, hands back `=COUNTIF(...)` instead of numbers — possibly without a cached
value at all.

**Measurement:** all 47 formula cells have a cached value (`<v>` next to `<f>`);
`tools/features.py` warns if that ever stops being true. The values agree with our own
extraction **exactly**: 194 / 100 / 100 / 28 / 39 / 118 / 45 / 12 / 51, total **687**, and so
does the priority split of sheet 01 (99 Must / 71 Should / 24 Could).

**Why it counts:** the extraction is thereby **confirmed by the spreadsheet itself**, not
only by our own count. That is worth more than the row-count guard alone — those numbers come
out of a different calculation (`COUNTA` in Excel) than ours.

**Whose it is:** done.

---

# The counter-check of the vector round (R1-D, 2026-08-09 `[debian]`)

The round was stopped before its counter-check ran. FIND-004 .. FIND-011 are that
counter-check. Every mutation named below was made in the working copy, measured, and taken
back out again; `git status --short` afterwards showed no file of `src/vector/` modified.

**Baseline before anything was touched**, so that every "red" below is a real one:
`cargo test --test vector_hooks --test vector_aiming --test vector_gas --test vector_boost
--test world` → 10 + 8 + 11 + 5 + 7 = **41 passed, 0 failed**.

## FIND-004 — No hook in the real game can anchor: no body in the world carries a `BodyId`

**Symptom:** `F-001` and `F-002` are each green on their own, and together they do nothing.
Every trigger pull in the running game ends as `ReleaseReason::NoAnchor`.

**Measurement:** `cargo run -- --headless --script scripts/f-001-hooks.txt --ticks 800`
(exit 0, "script run finished: 5 asserts held, 536 ticks") logs for two full seconds of
holding, one button each:

```
hook Left  of player 1 found nothing anchorable (t=112)
hook Right of player 1 found nothing anchorable (t=174)
```

Zero `hook … anchored on body …` lines. The cause is static and checkable without a run:
`world::index::maintain_index` (`src/world/index.rs:61-70`) has an **empty body**, and
`grep -rn BodyId src/` finds no other writer of it — so `vector::aim`'s
`Query<(&Body, Option<&BodyId>)>` (`src/vector/aim.rs:65`) always yields `None`, and
`anchor_target` (`src/vector/hook.rs:76-81`) returns `None` on `aim.body?` for **every**
block in the map, tagged or not.

**Why it counts:** no test in the tree can go red on this. `tests/vector_hooks.rs` injects
the carrier by hand into the index (`put_body`, lines 138-147) and forces the `AimPoint`
(`ForcedAim`, lines 52-62); and **no test in `tests/vector_aiming.rs` asserts `AimPoint.body`
at all** — `grep -n '\.body' tests/vector_aiming.rs` returns nothing. The seam between the
two features is the one place neither guard looks. It is honestly written down in both module
headers, which is why this is a finding and not a bug report — but a feature that cannot be
used in the game it is built into does not get past 🟨 on the strength of its unit tests.

**Whose it is:** `T-036a` (`world::index::maintain_index`). Not `vector`.

## FIND-005 — "A shot only leaves from `Idle`" is a decision no test defends

**Symptom:** `src/vector/hook.rs:24-27` spends a paragraph on decision 1 and on its price —
"a lockout after a release: `rope_length / vector.hook_retract_speed_m_s`, so 0.25 s on a
30 m rope". Nothing measures that the lockout exists.

**Measurement:** the most likely broken implementation is an arm that may fire again while
its tip is still coming home. Inserted before the `HookState::Retracting` arm:

```rust
HookState::Retracting if just_pressed && anchor_target(aim).is_some() => {
    let (target_m, body) = anchor_target(aim).expect("checked in the guard");
    next.state = HookState::Flying { target_m, body };
    next.tip_m = hand_m;
}
```

`cargo test --test vector_hooks` → **`test result: ok. 10 passed; 0 failed`**. Reverted with
`git checkout -- src/vector/hook.rs`.

**Why it counts:** `Retracting` is the state that makes the four-state machine a machine
rather than a decoration, and the sentence "that is what makes `Retracting` a state" is
currently 🟨 by the project's own rule. The missing test is one line long: hold, release,
press again inside the retract window, and assert the arm is still `Retracting`.

**Whose it is:** `F-001`, `tests/vector_hooks.rs`.

## FIND-006 — `gas_priority` is an attempt order, not a priority, and the degenerate tank is untested

**Symptom:** `src/vector/gas.rs:12-14` says the priority decides "who pays first" on a tight
tank. What `book()` actually does is *try each consumer in turn*: a consumer that cannot
afford its tick does not stop the ones behind it. At `gas_priority: [Boost, ReelIn]`,
`gas_boost_per_s = 18` and `gas_reel_per_s = 6` (`assets/data/game.ron`, 0.3 and 0.1 per tick
at 60 Hz), a tank holding **0.15** serves `ReelIn` and refuses `Boost` — the lower-priority
consumer wins because the higher one is too expensive.

**Measurement:** turned `book()` into a strict priority — `if !grant.boost { break; }` and
`if !grant.reel_in { break; }` after each `try_spend` — which is the opposite reading of the
same sentence. `cargo test --test vector_gas` → **`test result: ok. 11 passed; 0 failed`**.
Hand-checked against all six unit tests in `src/vector/gas.rs` as well (that binary does not
run them): every one of them uses a tank of 0.35, 0.1 or full, so none puts the tank
**between** the two costs with both consumers wanting. Reverted.

**Why it counts:** which of the two behaviours is wanted is a balancing decision
(`docs/QUESTIONS.md` Q-017) and today it is neither written down nor measured — it is an
accident of a `for` loop. `F-018`'s own words are "at 0 no more flying"; "at 0.15, reel but
no boost" is a rule nobody has agreed to.

**Whose it is:** `F-018` / Q-017.

## FIND-007 — `boost.rs`: `Option<Forces>` and `set_if_neq` are argued for and unguarded

**Symptom:** the doc comment on `gas_boost` (`src/vector/boost.rs:70-74, 88-90`) argues for
both: `Option<Forces>` because "with a plain `Forces` the whole row would drop out of the
query and `BoostAccel` would silently keep the value of the tick before", and `set_if_neq`
because a component reporting itself changed sixty times a second makes every
`Changed<BoostAccel>` filter worthless (§11).

**Measurement:** both undone at once — `Option<Forces>` → `Forces`, `drive.set_if_neq(...)` →
`*drive = BoostAccel(wanted)`. `cargo test --test vector_boost` →
**`test result: ok. 5 passed; 0 failed`**. Reverted.

**Why it counts:** two of the file's five documented decisions are unmeasured. They are not
wrong today; they are simply not *known* to be right, and the next person to touch the query
gets no warning.

**Whose it is:** `F-007`, `tests/vector_boost.rs`.

## FIND-008 — Three of the five acceptance criteria need a HUD, and the HUD is two empty functions

**Symptom:** `docs/features.ron` demands, verbatim: `F-001` "Zustaende sind im HUD sichtbar",
`F-007` "Gasleiste sinkt sichtbar", `F-018` "Gasverbrauch … im HUD ablesbar". None of the
three can be met.

**Measurement:** `src/hud/gas_bar.rs` — `spawn_gas_bar` and `update_gas_bar` both have empty
bodies; there is no hook-state widget anywhere in `src/hud/`; `debug::spawn_overlay` (the F3
overlay that would print the tank) has no caller. Decoded `docs/images/f-018-gas.png` and
`docs/images/f-007-boost.png` pixel by pixel: **0 cyan pixels** in either, of 921 600 — and
cyan is the reserved colour for gas (`docs/conventions.md` §3). There is no bar in either
picture.

**Why it counts:** F-001, F-007 and F-018 are capped at 🟨 by their own acceptance rows, no
matter how green their tests are — exactly as `src/hud/mod.rs` already states for F-018. It
should be stated for the other two as well. On the way past: `src/hud/gas_bar.rs` is still
written in German and links `docs/konventionen.md`, which was renamed to `conventions.md`.

**Whose it is:** `hud` (F-018 image duty), and the stage column of three rows.

## FIND-009 — `HANDOVER.md` §7's test count is wrong by 28

**Symptom:** §1 says "the count before the vector round was **151 green**", §7 says
`origin/session-2026-08-09` is at "**176 tests green**". Both describe this branch.

**Measurement:** counting `#[test]` over `src/**` and `tests/**` per revision:

```
7966f0a (before the vector commit) = 151
1549656 (the vector commit)        = 204
9835b2d (the handover itself)      = 204
c42ff6d (HEAD today)               = 204
```

`grep -rn '#\[ignore' src/ tests/` → 0. So **§1 is right and §7 is wrong**: no revision in
this history has 176, and the handover was itself written on a 204 tree. (`cargo test` also
compiles at most 4 doc-test fences, which can only push the number further above 176.)

**Why it counts:** §7 is the line the next session will quote as "the tree was green at N".
A wrong N turns the first real regression into an argument about arithmetic.

**Whose it is:** `docs/HANDOVER.md`.

## FIND-010 — `STATUS.md` and `features.ron` never learned that the round happened

**Symptom:** the round built four features and 53 tests; the file CLAUDE.md rule 1 calls "the
only truth about the progress" does not know.

**Measurement:** `docs/STATUS.md` — `F-001 ⬜ | —`, `F-002 ⬜ | —`, `F-007 ⬜ | —`,
`F-018 ⬜ | —`, tally "238 ⬜ · 6 🟨 · 1 🟧 · 0 ✅ of 245 rows". `docs/features.ron` — all four
still `stage: Unbuilt` with `evidence: ""`. Meanwhile the four files hold 812 lines
(`wc -l src/vector/{aim,boost,gas,hook}.rs`) and their four test files 1878, 34 of those
tests green (plus 7 in `tests/world.rs`, measured above).

**Why it counts:** this is the *honest* direction of the error — nothing is over-claimed —
but the document of record is 53 tests behind the tree, and `python3 tools/features.py`
regenerates `STATUS.md` from `features.ron`, so the gap does not close by editing STATUS.
`docs/HANDOVER.md` §2 already names it as the last step of the round; it is recorded here so
it is not lost with the handover.

**Whose it is:** the main head (`docs/features.ron` is not a subagent's file).

## FIND-011 — `F-003`'s criterion still cannot be demonstrated, now for a second reason

**Symptom:** `F-003`'s note in `docs/features.ron` says it stays 🟨 because "the acceptance
criterion is 'no hook on untagged parts', and there is no hook yet". There is a hook now, and
the criterion is **still** not demonstrable.

**Measurement:** two things are missing. (1) No test fires a hook at the map's **own**
untagged geometry. The nearest thing, `tests/vector_hooks.rs:602-617`, builds a synthetic
`AimPoint { anchorable: false }` over a hand-inserted index entry — it tests
`anchor_target()`, not the wall at `z = -33.5` from `assets/data/maps.ron`. (2) Because of
FIND-004, **nothing anchors at all**: a run that shows "no hook on the untagged wall" today
shows the same thing on the tagged roof, so it separates nothing. The other two halves of the
same acceptance row — "Tagging-Werkzeug im Studio vorhanden" and "Heatmap zeigt
Flaechenabdeckung" — exist nowhere and have no row in the translation table of
`docs/architecture.md` (only the `CollectionService` tag itself has one).

**Why it counts:** F-003 has been sitting at 🟨 waiting for the hook. The hook arriving did
not move it, and the reason has changed — that should be in the note, or the next session
will re-derive it.

**Whose it is:** `F-003` note in `docs/features.ron`; the missing test belongs to `world`.

## What survived the counter-check — measured, not assumed

Findings that only find fault are worth as little as findings that find none. These claims
were attacked and held:

- **`aim.rs`'s "hit first, then check anchorable" (`F-023`) is real and guarded.** Replaced
  the unfiltered `cast_ray` with a `cast_ray_predicate` that skips non-anchorable bodies —
  the exact "optimisation" the header warns about. **3 of 8 tests went red**, with the reading
  the header predicts: `the ray landed at z = -41 — the untagged wall stands at z = -33.5 …
  Anything past -34 means the ray went THROUGH the wall`, and
  `aim point Vec3(-30.0, 1.5999687, -41.0) instead of Vec3(-30.0, 1.5999687, -33.5)`.
- **`hook.rs`'s "a miss reports `NoAnchor` in the same tick and the arm stays `Idle`" holds in
  the real game**, not only in a test: see the two log lines in FIND-004, and the script's
  `assert speed < 0.5` held both times — a hook that found nothing moved the player by
  nothing.
- **The trigger fires on the edge, not on the hold, in the real game.** Two buttons held for
  60 ticks each produced exactly two log lines, not 120.
- **`boost.rs` does not boost for free.** Changed `if grant.boost` to
  `if grant.boost || intent.pressed(Buttons::BOOST)` — the plausible mistake — and
  `f007_a_held_button_without_a_grant_produces_exactly_zero` went red at
  `left: Vec3(-0.0, 0.0, -34.0), right: Vec3(0.0, 0.0, 0.0)`. The grant, and only the grant,
  decides.
- **`docs/images/f-007-boost.png` is a measurement and it re-measures.** Decoded and counted
  independently: the brick-red 8 m cube's top face runs x = 404 .. 521, that is **118 px wide,
  centred on 462.5** — the script predicted "(463, 118), 118 px" *before* the run. Ground
  **91.43 %** against the claimed 91.4 %, **0** `ClearColor` pixels, **0** cyan, **0**
  crimson. The sand-brown 4 m block's top face is 54 px against 54 predicted.
- **`docs/images/f-002-aiming.png` shows what its caption claims.** At the image centre
  (640, 360) the pixel lies in the bright top-face band y = 337 .. 381, bounded by cyan
  outline pixels `(0, 237, 255)` at y = 335 and y = 382 — the crosshair is on the **tagged
  roof**. The gray body filling the frame from y = 503 downwards has no cyan boundary — the
  untagged wall, in front of it, not shot through.

## FIND-012 — The titan's strike is a cylinder: facing is decorative, and the approach angle means nothing

**Symptom:** the user's tuning question was "does the husk turn slowly enough at
`turn_deg_per_s: 50` that the approach angle means something?". The question has no tuning
answer, because nothing reads a titan's facing when damage is booked.

**Measurement:** `src/combat/strike.rs:99-103`, `reaches()` is

```
ground_m <= reach_m && to.y <= top_m && to.y >= -reach_m
```

**No dot product, no forward vector, no half-angle.** A horizontal distance, a ceiling and a
floor — a cylinder around the titan's axis. Attacking from behind and attacking from the front
book the identical damage. Independently: `grep` over `src/titan/` finds exactly **one** writer
of a titan's root yaw (`brain.rs:428`); the rotations in `rig.rs` are `from_rotation_x` on child
arm and torso entities, and `SpawnTitan` carries no facing at all (`rig.rs:239`).

Second half of the same hole (this is FIND-012's original subject and it survived the attack):
the turn block in `brain.rs:411-429` runs only under `state == Pursue && distance_m >
attack_range_m` (`brain.rs:398-400`), and `distance_m` is horizontal (`brain.rs:333`). So a titan
does not turn inside 6.0 m, nor in `Idle`, `Windup`, `Strike` or `Recover`. Every cut in
`scripts/game-full.txt` lands at **1.882 m** from the axis — all three acts offset 1.80 m in −X
*and* 0.55 m in +Z — so the husk in those acts never turns once: `pursuing` is false from his
first tick onward.

**Why it counts:** two knobs the user was asked to judge (`turn_deg_per_s`, and
`attack_range_m` behind it) cannot be judged, because the mechanic they would govern is not
implemented. Tuning either one changes nothing a player can feel. This is a **design hole, not
a number** — and it should be answered before any of the 26 untuned values are touched, because
it decides whether "the approach angle" is a thing this game has.

**Whose it is:** combat design. Recorded as a decision in [`QUESTIONS.md`](QUESTIONS.md) Q-031.

**Confidence:** source reading plus arithmetic, produced by one agent and then attacked by a
second one that was told to refute it and could not. **Nothing here has been run** — 🟨.

## FIND-013 — The reel does not accelerate the player, it assigns his radial velocity

**Symptom:** `src/vector/reel.rs:5` describes F-005 as "a change of length, not a pulling
force". The implementation makes the length change set the velocity outright, in one tick.

**Measurement:** the chain is complete in avian's own source.
`avian3d-0.7.0/src/dynamics/solver/xpbd/plugin.rs:259-270` does
`body.linear_velocity += (delta_position - pre_solve_delta_pos) / delta_secs` **per substep**;
`DistanceJoint::solve` (`joints/mod.rs:326-344`) applies nothing while `distance <= max`, and
runs at compliance 0 against a static anchor, so the correction completes inside its own
substep; `player/rope.rs::attach_ropes` sets `limits.max` to the **current** distance, so the
player is exactly at the limit the moment `Ctrl` goes down. The algebra that follows: the
correction is `(v_r + r)·h`, so the projected radial velocity afterwards is **exactly `−r`** —
not "0 → 28 m/s" but **"anything → 28 m/s", independent of what it was before**.

**What is NOT claimed:** an earlier version of this finding put the figure at 40 320 m/s²
(4110 g) by dividing the step by one substep. That is a difference quotient across a
discontinuity and it is not a number anything observes. The honest observable is **+28 m/s
within one tick**, against `player.boost_m_s2 = 34`, which needs 49 ticks for the same change.

**Why it counts:** `vector.reel_speed_m_s` is the knob the user singled out and asked "jolt or
swing?". If this holds under measurement, the answer is *jolt, structurally* — and no value of
`reel_speed_m_s` can make it a swing, because the knob does not set an acceleration. What is
missing is a value that does not exist yet (an ease-in, `vector.reel_ramp_s`) or a change to
`min_rope_m`, which carries no `UNTUNED` marker at all because its stated reason is a camera
constraint.

**MEASURED, 2026-08-10 [cachy], and it is exact.** Sixteen runs against the frozen binary, with
`vector.reel_speed_m_s` varied in a scratch copy of `assets/` (the game reads `assets/data`
relative to the working directory — `src/data/mod.rs:46-62` — so a value can be changed without
recompiling; the scratch tree was proven to be the one being read before any number was taken).

| tick | speed | Δ | height |
|---|---|---|---|
| 149 (`key Ctrl`) | 0.000 | — | −0.000 |
| 150 | **28.000** | **+28.000** | 0.287 |
| 151 | 31.102 | +3.102 | 0.751 |
| 152 | 31.153 | +0.051 | 1.214 |

**+28.000 m/s in one 16.7 ms tick = 1680 m/s² = 171 g**, and then +0.05 m/s per tick. The onset
is ~500× the steady state. Across nine rungs the first tick reads 10.003 / 14.001 / 20.002 /
24.001 / 28.000 / 32.001 / 36.000 / 45.001 / 60.002 — **the law is `speed := reel_speed_m_s`, to
three decimals.** Checked against a moving start as well: reel pressed while airborne at
5.500 m/s gives **28.307 m/s** one tick later, against 28.34 predicted by "radial component
destroyed, tangential kept". Two runs of the same configuration are byte-identical over 153
samples.

**The number the project has been quoting is not the reel's output.** The reel completes at
**54.18 m/s**; `scripts/f-001-hooks.txt` samples 5 ticks later, after the rope at `min_rope_m`
has whipped the player around and taken **14.2 % of his speed in one tick** (54.010 → 46.327).
The rig reproduced f-001's known pair exactly (46.415 m/s at 13.064 m) before measuring anything
new, which is what makes the rest of the ladder trustworthy.

**Stage 🟧** for the step itself: measured, reproduced, and the rig validated against a
previously known number.

## FIND-014 — The punish window is fine for the husk and too short for two of the three titan kinds

**Symptom:** an agent claimed `recover_ticks (24) >= swing_ticks + cooldown_ticks (39)` is red
today. The claim is **refuted**, and the correct version of it is red somewhere else.

**Measurement:** the numbers are right (24, 21, 18 recover; 36 windup, 12 strike, 90 cooldown;
units are ticks, converted once through `round(s·hz)`; `swing.rs:113-120` starts the blade
cooldown at swing *end*, so 39 is the right cut-to-cut period). The comparison was wrong.
`brain.rs:254-255` sets `cooldown_left = 90` on entry to **Windup**, and `decide` (`:291`)
blocks the next Windup until it reaches 0. So the real no-attack window is
`attack_cooldown − windup − strike + recover = 90 − 36 − 12 + 24 = ` **42 ticks against 39
needed** — a second cut fits, with 3 ticks to spare.

The correct assert is `attack_cooldown_ticks − windup_ticks − strike_ticks >= swing_ticks +
blade_cooldown_ticks`. It is **green for the husk and red for the scuttler (18 < 39) and the
weaver (25 < 39)**.

**Why it counts:** neither of the two small kinds is in play yet, so this bites the day one
spawns — and it will look like a combat bug rather than a data one. It is also a clean example
of the rule: the first agent had every number right and the conclusion wrong.

**Confidence:** 🟨, arithmetic over the RON files and the state machine; not run.

## FIND-015 — `src/vector/reel.rs:5` cites a function nothing calls

**Symptom:** the module header points at `shared::rope::rope_reel_in`.

**Measurement:** `grep` finds no production caller of that function.

**Why it counts:** small, but it is the header of exactly the file whose documented intent and
implementation disagree (FIND-013). Whoever reads the header to understand the reel is sent to
retired code.

## FIND-016 — The rope eats 14 % of the player's speed in one tick, against a documented 4.26 % per second

**Symptom:** measured while running the reel ladder (FIND-013), not looked for.

**Measurement [cachy], 2026-08-10:** at `reel_speed_m_s = 28` the player reaches 54.010 m/s and
is at 46.327 m/s on the **next tick** — **−14.2 % in 16.7 ms**, at the moment the rope hits
`min_rope_m` and whips him around. The `substeps: 24` comment in `assets/data/game.ron` claims a
swing loss of 4.26 % **per second**, and `tests/vector_rope.rs` measures 0.43 %/s over 60 ticks.
Those two describe a hanging swing; nothing in the repository describes the whip.

**Why it counts:** it is the difference between the reel's real output (54.18 m/s) and the number
this project has been quoting for a day (46.414). It also means the single most violent event in
the movement system — a 7.7 m/s loss in one tick — is undocumented and untested. Whether it is
correct physics for a taut rope at the length floor or a solver artefact is **not decided here**.

**Confidence:** 🟧 as a measurement (reproduced byte-identical), ⬜ as an explanation.

## FIND-017 — `MaxLinearSpeed(75)` is load-bearing tuning, not a safety backstop

**Measurement [cachy]:** end-of-reel speed against `reel_speed_m_s` is monotone increasing —
23.2 (r=20) → 38.6 (r=24) → 54.2 (r=28) → 69.0 (r=32) → **75.00 pinned** (r=36, 45, 60). The
clamp is reached between r=32 and r=36.

**Why it counts:** above r ≈ 35 the reel's end speed *is* the clamp, so anyone raising the reel
rate past 32 is turning a knob that no longer does anything. A clamp that silently becomes the
tuning value is the kind of thing that costs a day. The unclamped peaks (75.02 / 75.62 / 75.77)
also show the clamp leaking by up to 1 %.

**Also refuted here:** the hypothesis that end speed is a **U** in `reel_speed_m_s` with a
minimum near 28-32. Two independent paper models predicted it (55.5 m/s at r=20, 75.35 at r=14);
the measurements are 23.2 and 23.4 — wrong in magnitude and in direction. The hypothesis is not
undecided, it is **dead**.

**Confidence:** 🟧, nine rungs, two of them repeated identically.

## FIND-018 — The two lowest ladder rungs are contaminated, and the rig they inherit is poor

**Symptom:** reported by the measuring agent as a result rather than dropped, which is why it is
here.

**Measurement:** at `reel_speed_m_s = 10` the player hits the watchtower at t=221, **17 ticks
before** the reel completes — the 10.05 m/s recorded at the nominal completion tick is a
post-impact number and says nothing about the reel. At r = 14 the impact is at t=214 and
completion at t=213: one tick of margin. Pre-impact peaks are 21.98 (r=10) and 23.41 (r=14).

**Why it counts:** the `scripts/f-001-hooks.txt` geometry ends in a collision on **every** rung,
so every number in the ladder inherits a rig that terminates in a wall. A clean reel measurement
needs an anchor with ~40 m of nothing beneath it, and no such fixture exists in the repository.

**Confidence:** 🟧 for the contamination itself; the numbers above r=20 are unaffected.

## FIND-019 — A pixel diff of 0 means "nothing changed" AND "the camera is clamped on a blank plane"

**Symptom:** the window campaign of 2026-08-10 produced a **false bug**, held it across three
runs and twelve measurements, and only then refuted it. It reported "after `Esc`/`Esc` the mouse
never rotates the view again" — a one-way-door bug in the cursor grab. There is no such bug.

**Measurement:** the agent's own large `ydotool mousemove` calls had driven the pitch into its
`±89°` clamp, so the camera was staring at flat ground or flat sky, **where a yaw change is
invisible**. The two cases are numerically distinguishable and nothing was looking:

| frame kind | distinct colours | std |
|---|---|---|
| camera clamped on a featureless plane | **19-23** | 13.09 |
| camera on the city, mouse demonstrably working | **1110-1360** | ~67 |

**The fix is a method, not a patch:** switch the **F3 overlay on for every window measurement**,
so the tick counter stands inside the picture. Then "nothing happened" and "the process is
frozen" stop being the same number, and a 0-pixel diff becomes readable. With the overlay on, a
playing frame changes 215 px over 2 s and a paused one changes 0 px — the pause is provable
precisely because the tick is in frame.

**Why it counts:** every window screenshot this project will ever take is a pixel diff over a
parked scene. This trap is available at every one of them, and it produces a confident wrong
answer rather than an error.

## FIND-020 — `--novsync` does nothing in a window on niri, and the frame rate is the display's

**Measurement [cachy], `wl_surface.commit` counted on the Wayland wire over the last 10 s of a
22 s run:** vsync **1801 commits = 180.1 fps**, median frame time 5.551 ms, p95 5.907 ms;
`--novsync` **1801 commits = 180.1 fps** — identical. DP-2 runs at 180.002 Hz.

**Cause:** `wp_tearing_control` never appears in the protocol log — the client never asks for it,
so `PresentMode::AutoNoVsync` cannot be honoured and the swapchain stays throttled to the
refresh rate.

**Why it counts:** `prompts/init.md` §11 introduces `--novsync` so that "what does this cost?"
is not measured against the 16.6 ms ceiling six times over. **In a window on this machine that
flag is a no-op**, so any performance measurement taken in a window is measuring the display.
Performance work has to go through `--offscreen` or `--headless` until the tearing-control
protocol is requested. Also note the ceiling here is 5.55 ms, not 16.6 — this display is 180 Hz,
not 60.

## FIND-021 — Four things the window taught us that no headless run could

**All measured [cachy] 2026-08-10, first windowed session in the project's life.**

1. **`wtype -k F3` never reaches the game; `ydotool key 61:1 61:0` does.** `wtype -k Escape`
   works. Anyone driving the window from outside must use `ydotool` for function keys.
   (Evidence: overlay absent after `wtype`, present after `ydotool`, absent again after the next
   `ydotool` toggle.)
2. **`niri msg windows` reports the game with `App ID: (unset)`.** An earlier recon in the same
   session reported `app_id defeated_by_titan` and was **wrong**. The only reliable handle is the
   window title. If an `app_id` is wanted, winit has to be told to set one.
3. **A `--script` run exits the moment the script's last line is consumed — `--ticks 0` does not
   hold it open.** `scripts/f170-hud.txt` died at "4 asserts held, 493 ticks" and took the window
   with it. A windowed script that an outside observer needs to look at must end in a long
   `wait`; that is why `scripts/p4-cursor.txt` ends in `wait 600`.
4. **The window opens tiled at 947x1030 on DP-1**, not fullscreen and not on the 2560x1440
   output. Every window measurement has to move and fullscreen it first
   (`niri msg action move-window-to-monitor-down`, `fullscreen-window`) or it is measuring a
   947 px viewport.
5. **`--mission tutorial` with nobody at the keyboard is a loss screen in ~16 s.** The husk
   spawns at (24, 0, 0), walks to the standing player and downs him: `mission LOST kills 0/3`,
   player `Downed`, by t≈941. Not a bug — but every unattended windowed mission run ends in
   `LOST` unless something moves.

## FIND-022 — 🔴 The rope is not rendered, and no picture in this repository has ever shown one

**Symptom:** found while decoding the evidence images for `scripts/f-flight-cut.txt` — not looked
for, and the single most consequential finding of 2026-08-10.

**Measurement, from the pixels and not from the source:** `docs/images/f030-flightcut.png`, taken at
the tick of a cortex cut landed under rope momentum, contains exactly **11 cyan connected
components**, and every one of them is an axis-aligned HUD rectangle — gas fill 274×12, pip
underline 86×6, five pips 14×18, four crosshair ticks 22×3 and 3×22. **Not one diagonal
segment.** Then, and only then, the source: `src/render/rope.rs` is

```rust
pub fn draw_ropes(_spieler: Query<(&Hook, &Transform)>, mut _gizmos: Gizmos) {}
```

registered at `src/render/mod.rs:33` and marked "to be filled by assignment S — F-001, screenshot
required". It never was.

**Why it counts, and it counts more than anything else found today:**

1. **Every screenshot in this repository captioned "hook" or "rope" is mis-captioned** —
   `docs/images/f-001-hooks.png` and `docs/images/f003-anchors.png` among them. They cannot be
   showing a rope. Any evidence line that leans on them has to be re-read.
2. **The core mechanic of this game is invisible.** The Vector Gear is two hooks; a player cannot
   see either of them. Whatever the movement feels like, it currently cannot be *read*.
3. It makes a picture-based 🟧 impossible for F-001, F-004 and F-005 by construction, which is
   probably why those three have sat at 🟨 through two sessions with good numbers behind them.

**Confidence:** 🟧 as a finding — measured in the image, then confirmed in the source, then
confirmed a third way (a control run with the blade swing removed produced **byte-identical**
files, so the blade is not drawn either).

## FIND-023 — `MovementState::Tethered` is never written, so the overlay says `Airborne` on a rope

**Measurement:** `src/player/integrator.rs:100` writes only `Grounded` and `Airborne`;
`src/combat/health.rs:63` writes `Downed`. Nothing anywhere writes `Tethered`. The F3 overlay at
t=148 and t=152 of `scripts/f-flight-cut.txt` reads **`Airborne`** while an arm is anchored and
the gas ledger is being debited for it.

**Why it counts:** together with FIND-022 it means **there is no pixel in this build — world or
HUD — that says a rope is attached.** The only proxy is the gas bar draining by 2.79 px per gas
unit, which is indirect and needs decoding to read at all.

## FIND-024 — `--offscreen` segfaults at teardown after writing the PNG, in about 1 run in 6

**Measurement [cachy]:** 3 of ~19 `--offscreen --screenshot` runs exited **139**
(`Segmentation fault (core dumped)`), preceded by `NVVM compilation failed: 3`. In every case
`image written: … (478081 bytes)` and `Screenshot saved to …` were logged **first**, and the
identical command exited 0 on retry.

**Why it counts:** any harness that reads a screenshot run's exit code as a verdict will flake
about one time in six, and it will look like a failing test. It is a second, independent reason
for the rule that was already in force for a different reason: **run every script twice — once
`--headless` for the exit code, once `--offscreen` for the image.** No repro line yet beyond
"run it about six times".

## FIND-025 — `--offscreen` IS bit-identical at speed; the "only at slow moments" rule is wrong

**Symptom:** the project has been operating on the rule *"`--offscreen` is only bit-identical at
slow moments; at 46 m/s two identical runs differ in 38 828 of 921 600 pixels — the simulation
does not drift, the shutter catches a different tick."* That rule is not right.

**Measurement [cachy] 2026-08-10:** two runs of `scripts/f-flight-cut.txt` at `--ticks 152`,
where the cut lands at **74.70 m/s** — considerably faster than 46 — are **bit-identical**:
`sha256 a490e4d9…` twice, **0 differing pixels of 921 600**. The t=148 pair is likewise identical
(`sha256 eaca46d9…`).

**Cause:** `src/render/camera.rs` deliberately does **no interpolation between simulation
steps**, so a rendered frame is a pure function of the tick number. Speed cannot move the
shutter, because there is no shutter to move.

**Why it counts:** the 38 828-pixel difference measured for `f-001-hooks` is real and now has no
explanation. It should be **re-opened** rather than carried forward as a law — the current rule
tells future sessions to expect nondeterminism where the renderer provides determinism, and it
would excuse a genuine nondeterminism bug as expected behaviour.

*(Append further findings here. A finding without a measurement is an opinion.)*

Related: [`docs/BUGS.md`](BUGS.md) (our own bugs) · [`docs/QUESTIONS.md`](QUESTIONS.md)
