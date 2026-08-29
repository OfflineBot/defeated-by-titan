# BUGS — every bug with repro, evidence, cause, fix and test

Updated: 2026-08-10

> **A bug without evidence is a rumour — and uncertainty is a defect.**
> No "should work now", no "should be fine", no "probably fixed". Either you have it
> **evidenced**, or you write down that you do not (`prompts/init.md` §9).

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

## Wording

| do not write | but |
|---|---|
| "fixed" (without a red test before it) | "fixed, test `x` was red, is green" |
| "should work now" | "built, **untested** — 🟨" |
| "runs" | "seen in the game, screenshot `docs/images/…`" |
| "is faster" | "16.6 → 9.4 ms, `--release --novsync`, median of 5 runs [cachy]" |
| "probably works" | a line in `docs/QUESTIONS.md` or here |

**Doubt moves the stage down, not up** (§8, §9). If you are not sure, it is **🟨** — even if
it works. That costs nothing. A stage that is too high costs the next person half a day.

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

## Open bugs

### `B-005` — the rope goes slack on a fast approach, so you fly past your own anchor

> **Fixed the same day, and deliberately still filed under Open.** The fix has a red-then-green
> test and a measured before/after (50.000 m → 3.000 m), but **no agent other than the one that
> built it has attacked it**, and nobody has seen it in a window. By this project's own rule that
> is 🟨, and a 🟨 fix is not a closed bug. It moves to *Closed* when a second head re-measures it.

**Reported by the user on 2026-08-10**, from playing it: *"das seil hat eine maximal einhol
dauer. das heißt wenn ich mich festhake und ganz schnell ran fliege kann ich overshooten! weil
das seil nicht schnell genug 'eingefahren' wird!"*

| Field | |
|---|---|
| **Repro** | `cargo test --test vector_rope -- --ignored measure_the_overshoot_past_the_anchor`, `[cachy]`. A 50.000 m rope, gravity off, flying straight at the anchor. |
| **Evidence** | **Metres flown past the anchor, before the fix:** with the reel held — 3.000 (20 m/s) · 8.667 (40) · 16.000 (55) · 22.500 (75); **without the reel held — 50.000 at every single speed from 20 m/s up**, i.e. the whole rope. The 28 m/s threshold is exact only while the button is down (`enf_at_0` 15.002 measured at 40 m/s against `50·(1−28/v)` = 15.000; 31.334 against 31.333 at 75). **Without the button the enforced length never shrinks at all, so there is no threshold and a faster reel could never have fixed it.** After the fix: **3.000 m at every speed, with and without the reel** — exactly `vector.min_rope_m`, the geometric minimum. |
| **Expectation** | The gear is a winch. A cable that has been closed on does not pay itself back out, and it does not hang slack while you dive at the thing you are attached to. The user asked for the same property from the other side earlier the same day — *"wenn mit seilen verbunden und wurde kürzer soll erstmal nicht länger werden"* (Q-034). |
| **Cause** | `src/player/rope.rs::shorten_ropes` contains `if rate_m_s <= 0.0 { continue; }` — **the enforced length `limits.max` changes only while the reel button is held**, and then by at most `vector.reel_speed_m_s` = 28.0 m/s. `vector.max_speed_m_s` is 75.0. A `DistanceJoint` with `limits = (0, L)` corrects **only** when the distance exceeds `L` (`avian3d-0.7.0/src/dynamics/solver/xpbd/joints/mod.rs:326-344`). So closing on an anchor faster than 28 m/s outruns the limit, the rope goes slack, a slack rope constrains nothing, and the player sails past. **Without the reel held the limit never shrinks at all**, so slack develops on any approach at any speed. |

**The fix under test: a slack take-up ratchet** — `limits.max` follows the true distance downward,
with no rate cap and independently of the reel button, floored at `min_rope_m`. That is what a
winch does, and it answers Q-034 and this bug with one mechanic.

**FIXED 2026-08-10 [cachy].** Take-up added inside `shorten_ropes` — *not* a new system, because
`limits.max` has exactly one writer (rule 3) — in the existing `SubstepSchedule` slot before
`SubstepSolverSystems::WarmStart`:

```rust
limits.max = (limits.max - reel * dt).min(distance).max(min_rope_m);
```

Per **substep**, because at 75 m/s the body covers 1.25 m per tick against 0.052 m per substep.
*(That choice is reasoned, not measured — a per-tick variant was not built, because it would need
a second writer of `limits.max`.)*

**The risk was real and it was measured both ways: the swing survives.** A pendulum needs a
constant length through the arc, so if the distance dipped below the enforced length the ratchet
would shorten the rope every pass and winch the player into his own anchor. Over 4 s on an 8.000 m
rope at v0 = 8 / 12 / 16 / 30 m/s, gravity on and off, the dip was **0.0000 m in 9 of 10 cases** —
the solver produces no measurable slack, so there is nothing to bite on. After the fix the same
arcs end at **7.9997–7.9999 m**, i.e. 0.0003 m lost in 4 s. **No tolerance was needed and no new
`game.ron` key is required.** The tenth case (v0 = 20 with gravity on) does ratchet, and there the
rope is genuinely slack — the player goes over the top of the anchor, and that case dipped
0.7093 m *before* the fix too.

**Tests:** `tests/vector_rope.rs::b004_a_player_flying_at_his_anchor_does_not_pass_it` and
`::b004_the_enforced_length_never_grows`, both seen red first. Two further guards
(`b004_a_swing_keeps_its_length_across_the_arc`, `b004_anchoring_still_does_not_yank`) **were green
before the fix and could not be made red** — stated plainly rather than dressed up, because before
the fix nothing could shorten a swing at all. Four `#[ignore]`d measurement tests carry the raw
tables. `cargo test --test vector_rope`: 12 passed, 0 failed.

**And it partly answers `FINDINGS.md` FIND-026 — a split verdict, measured on real church
geometry with a rope born at 53.269 m:**

| closing flight first | rope after | arc bottom | reaches the ground |
|---|---|---|---|
| none | 53.269 m | **−18.32 m** | yes |
| 20 m/s for 1 s | 38.848 m | −3.90 m | yes |
| **40 m/s for 1 s** | **20.165 m** | **+14.79 m** | **no — taut 300/300 ticks** |
| 75 m/s for 1 s | 7.379 m | +27.57 m | no (hangs under the anchor) |

**Take-up alone does not fix FIND-026** — running off a roof moves you *away* from the anchor, so
nothing is taken up and the 0 m/s row reproduces the old result exactly. But one second of closing
at 40 m/s turns a 53.3 m leash into a 20.2 m rope and lifts the arc bottom from 18.3 m below
ground to 14.8 m above it. **An arc that could not exist in the graybox now exists — it has to be
paid for with gas first.** That is the mechanic the user described, and it turns his other
complaint (*"seile ohne boost bringen gar nichts"*) from a geometric impossibility into a tuning
question.

**Stage: 🟨.** Headless numbers only. Not seen in a window, no screenshot, and **not yet attacked
by an agent that did not build it**. The graybox rows inject the closing velocity and then zero
it, so their peak speeds are not what a player would actually carry into the arc.

**Still open:** `min_rope_m: 3.0` is now the entire overshoot budget, and FIND-026 §5 showed that
reaching that floor parks the player dead under his anchor at 0.002 m/s — take-up now reaches it
without a button being held. Whether 3.0 is still the right number is a `game.ron` question.

### `B-004` — cutting a titan while a rope is attached panics the game

**Found 2026-08-10 [cachy]** by `scripts/f-flight-cut.txt`, the first file in this repository
that lands a cut while roped. **This is the core loop of the game: hook, fly, cut.**

| Field | |
|---|---|
| **Repro** | `./target/debug/defeated_by_titan --headless --script scripts/f-flight-cut.txt --ticks 400`, `[cachy]`. In the file, change ACT 3's `hook right 0.74` to `hook right 0.80` — one character. 0.74 releases the rope at t=157, **inside** the 7-tick impact frame (153-159) and the run is clean; 0.80 releases at t=161 and it panics. Bracketed at t=161 / 173 / 203 / 263 / 353 — **every** release outside the impact frame panics, exit **101**. |
| **Evidence** | `assertion failed: island.joint_count > 0`, `avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:786`. Both halves are required: delete `key Ctrl` (rope, no hit) or delete `key E` (hit, no rope) and the identical t=353 release exits 0. Logs: `…/scratchpad/run-iso-{noreel,notitan,noslash}` against `run-rel2-{0.74,0.80,1.0}`. |
| **Expectation** | A cut landed under rope momentum is the game's central action; the design bible gates every meta system behind exactly this move. It must not depend on letting go of the rope first, and it must never abort the process. |
| **Cause** | `combat::hitstop::begin` puts `RigidBodyDisabled` on the player for the impact frame while the `DistanceJoint` is still attached. avian removes the body from its island; the joint's later removal then decrements a count that is already zero. **Second face, different file:** `player::rope::shorten_ropes` does not check `HitStop` either — it keeps shortening `limits.max` through an impact frame the player cannot follow, storing **0.93 m of rope over two frozen ticks**, which is paid back in a single tick as a clamp-limited **74.700 m/s**. |

**Why nothing caught it for a day.** `scripts/game-full.txt` lands all three of its cuts **out of
a fall, with no rope attached** — its own header says so. The 356-test suite never attaches a
joint and a hit stop to the same body in the same tick. Five pieces green separately.

**Consequence for `f-flight-cut.txt` today:** the file releases the rope 5 ticks after the cut to
dodge this, and says so in its header. Its cut therefore lands at **74.70 m/s**, which is
`vector.max_speed_m_s` — the clamp, not a speed anybody chose, and an artefact of the stored rope
above. The dodge goes away, and ACT 3 goes back to describing a rope, the day this is fixed. It
is also **not** established that the cut still lands once it is fixed: the 74.7 m/s snap is part
of why the sweep crosses the nape at all, and the timing will need re-measuring.

**No fix yet, and deliberately no fix today** — rule 5 wants the red test first, and the red test
here is a one-character script change that is already written down above.

## Closed bugs

### `B-003` — a teleport dragged the rope along, and nothing said so

**Fixed on 2026-08-10**, test `b003_a_warp_lets_go_of_every_rope` was red, is green.

| Field | |
|---|---|
| **Repro** | `cargo test --test vector_rope -- b003` at commit `c3c2ca4`, `[debian]` — two tests, both red before the fix. The same thing in the running game, with the script below at `--headless --ticks 700`: anchor on the watchtower, then `warp` 79.5 m away **while the left trigger is still held**. |
| **Evidence** | **In the test:** one tick after a warp of 55.73 m off two 9.00 m ropes, *both* `DistanceJoint`s were still holding and the player had been pulled **47.93 m** back toward his old anchor — 86 % of the way home, in a single tick. Both hooks stayed `Anchored` for ever, and **zero** `HookReleased` messages arrived in the three ticks after the warp. **In the game:** `assert Height > 35 — measured 10.722` and `assert Speed < 10 — measured 75.000` (that is `vector.max_speed_m_s`: the yank saturated the clamp), `script run finished: 2 of 4 asserts failed`, exit 1. After the fix the same run reads `4 asserts held, 471 ticks`, exit 0. **In anger:** `scripts/game-full.txt` lost **two of its three kills** to this, and the run still reported success — no assert fired, no message, not one line of the log. |
| **Expectation** | `prompts/init.md` §12c: a `warp` means *"the player stands exactly there afterwards"* — that is the whole worth of the verb, and a coordinate you get dragged off is not one. `F-004`: the rope is a constraint between the player and an anchor he chose; a body that teleports has not chosen anything. `src/vector/hook.rs` (header) names the reasons to let go that come from outside and are merely carried out there — a teleport is the third of them and was missing. |
| **Cause** | `src/player/mod.rs:185-200` (before the fix) — `apply_warps` wrote `Transform` and zeroed `LinearVelocity` and **nothing else**. `src/player/rope.rs:154-182` — `detach_ropes` released on `HookReleased` and on `BodyGone`, and a warp is neither, so the joint survived the teleport and avian solved it in the very same tick. Nothing in the chain was individually wrong: the warp is right, the joint is right, the hook state machine is right, `detach_ropes` is right, and the script driver is right. **Five pieces green separately and red together.** |

**Why it was silent, which is the expensive half.** Every channel that could have spoken was
closed by construction. `RopeLength::overextended` — the one back-channel `vector::hook` listens
to — compares the anchor distance against `vector.hook_range_m` (90 m) *after* the physics step,
and the joint's whole job is to make sure that distance never exceeds `limits.max` (9 m). **The
constraint erases the evidence that it acted.** The warps in `scripts/game-full.txt` are 55 m,
under the 90 m threshold anyway. And the script's asserts measured `speed` and `height`, both of
which a yanked player has in abundance. Exit code 0, twenty-three asserts held, two kills gone.

**Fix**, in two rails, both in `player` — the domain that owns the joint:

1. **`src/player/rope.rs::detach_ropes` also reads `WarpPlayer`** and despawns the joint and its
   anchor marker. It runs in `SimulationSystems::Drive`, **one stage before** `apply_warps` in
   `Integrate`, so avian never sees a teleported body still tied to an anchor. Doing it inside
   `apply_warps` would not have worked: its `Commands` land at the next sync point, and that is
   behind `PhysicsSystems::StepSimulation`. It logs `rope Left of player 1 cut: the player was
   warped away (B-003)`.
2. **`src/player/rope.rs::sync_rope_length` raises `RopeLength::overextended`** for every arm
   that is `is_anchored()` when its player is warped. That component has exactly one writer and
   still does; `vector::hook` — **the only writer of `Hook`** — reads the flag one tick later and
   lets the arm go itself, with a real `HookReleased` and its own log line. Asking through the
   message the hook module already listens to is the only way to release an arm from `player`
   without becoming a second writer of `Hook` (§6 rule 3).

`attach_ropes` was given the same guard: a hook that bites in the tick its player is warped gets
**no** joint at all, because the `Position` it would measure the length from is the one the
player is about to leave.

⚠️ **The reason the player is told is `ReleaseReason::Overextended`, and it is a stand-in.** The
honest one is `ReleaseReason::Warped`; that enum is `src/shared/message.rs:113-123`, which
`player` does not own. `Overextended` is the closest of the four — it is the one that already
means *"the rope's length cannot be honoured any more"*, and the one `hud` and `sound` read as
"the rope tore" rather than "the player let go". → `docs/FINDINGS.md`.

**Tests** — both seen red before the fix and red again with the fix taken back out (rule 5,
step 3, with identical numbers both times: `47.93 m`, `0 message(s)`):

| Test | what it pins |
|---|---|
| ★ `tests/vector_rope.rs::b003_a_warp_lets_go_of_every_rope` | **both** arms anchored on one anchor, then a 55 m warp: no `DistanceJoint` left, `RopeLength` `0.0` on both sides, both arms back to `HookState::Idle` four seconds later, and the player within 5 cm of the coordinate he was warped to. Gravity off, so the rope is the only thing left that could move him. Red before: *"2 joint(s) are still holding the player … dragged 47.93 m"*. |
| ★ `tests/vector_rope.rs::b003_a_warp_that_lets_go_says_so_out_loud` | **the silence.** A `HookReleased` for that player and that side really arrives within three ticks of the warp. A fix that drops the rope and says nothing has only fixed the yank. Red before: *"0 message(s) in three ticks: []"*. |
| `tests/vector_rope.rs::f004_the_rope_pulls_but_does_not_push` | was **green before and had to stay green** — and it moved the player with a `warp`, so after this fix it would have gone on passing with no joint in the world at all. It now uses a `place` helper that writes `Position` directly, plus an explicit `assert_eq!(joint_count, 1)`. Same for the `hang` harness, which pinned a flying hook's player with a warp in every tick. **A fix that makes a green test vacuous has broken it.** |

**Evidence that it is fixed, in the running game and not in a test** — the script (it lives in
the scratch directory; `scripts/` belongs to another job, and adding `scripts/b003-warp.txt`
is in `docs/FINDINGS.md`):

```
wait 1.5
look 0 34
warp 24 0 -20
wait 0.3
assert height < 0.5
hook left 6.0     # held for six seconds — the rope stays alive across everything below
wait 1.0
assert speed < 0.5
warp 24 40 40     # and away, with the rope still on: 79.5 m from the anchor, under the 90 m
wait 0.2          # of vector.hook_range_m, so `overextended` never fires on distance alone
assert height > 35
assert speed < 10
```

```
rope Left of player 1 attached at 17.83 m (t=124)
rope Left of player 1 cut: the player was warped away (B-003)
hook Left of player 1 let go: Overextended (t=175)
script run finished: 4 asserts held, 471 ticks        # exit 0
```

**Regression:** `cargo run -q -- --headless --mission tutorial --script scripts/game-full.txt
--ticks 1600` — `cut titan 2 Cortex` at t=656, `titan 3` at t=777, `titan 4` at t=898,
`MISSION WON at tick 898`, `23 asserts held, 1200 ticks`, exit 0 `[debian]`. Tick for tick the
run this bug was found in. The workaround in that file (`wait 4.2 # past t=533 … the joint is
gone`) is now unnecessary; taking it out is that file's job, not this one's.

**Two things to learn from it:**

1. **A constraint destroys the evidence that it acted.** `overextended` asks "is the player
   further from his anchor than a rope may be?" — *after* the solver has spent the tick making
   sure he is not. Every "did the limit get hit?" flag read downstream of the thing that enforces
   the limit is this same bug. The cheap check: for every guard condition, ask which system runs
   between the cause and the reading, and what that system does to the quantity being read.
2. **When two systems change the same body in one tick, the second one has to be told about the
   first.** The teleport and the joint are both right, and they are both right about a body only
   one of them may own for that tick. The seam is the stage order, and it is the only place the
   handover can be written down — which is why the release sits in `Drive` and not in the system
   that does the teleporting.

#### `B-003` — AMENDED 2026-08-19: not *every* warp, only the ones that lengthen the rope

**The 2026-08-10 fix released every rope on every `warp`, whatever the distance.** Right for the
case it was written for, wrong for the 35 scripts that use `warp` to put a player somewhere: it
cost the `F-029` round two runs on a warp that had *shortened* a rope from 62 m to 23 m, and it
was written down nowhere a script author looks (`docs/FINDINGS.md` FIND-116).

**The rule that replaced it is the joint's own.** `limits = (0, L)` corrects only when the
distance *exceeds* `L`, so a teleport that leaves the rope no longer than it already is has
nothing to correct: a warp **toward** the anchor, along it, or onto the same spot **keeps the
rope**, and the length ratchets down to the distance that now really exists (`B-004`). Only a
warp that lengthens the rope cuts it. `src/player/rope.rs::warp_keeps_the_rope` is the whole rule
and all three readers of `WarpPlayer` in that file ask it.

⚠️ **And the other direction has no usable budget — measured, and the first version of the key
was wrong because of it.** `vector.warp_rope_slack_m` was set to 0.25 m, derived from
`player.max_substep_m`. Then it was measured on a 9.00 m rope with gravity off:

| excess | drag in one tick | the speed the player leaves at |
|---|---|---|
| 0.0000 m | 0.0000 m | 0.000 m/s |
| 0.0001 m | 0.0024 m | 0.143 m/s |
| 0.0100 m | 0.2401 m | **14.403 m/s** |
| 0.0500 m | 1.2001 m | **72.004 m/s** |
| 0.2500 m | 1.4479 m | **75.000 m/s** = `vector.max_speed_m_s` |

The solver corrects the whole excess inside **one substep**, so it comes back out as a velocity of
`excess * simulation_hz * substeps` = excess × 1440 /s. **One centimetre is already twice running
speed.** So the key is a float tolerance and not a distance a player may be moved:
`0.004 m`, bounded by `<= player.run_speed_m_s / (simulation_hz * substeps)`.
`max_substep_m` bounds a *position* step and says nothing about a solver impulse — that is what
the guess was worth, and it is why the table is in `game.ron`.

**Also amended: `attach_ropes` no longer throws the shot away.** A hook that bites in the tick its
player is warped used to get **no rope at all**; it now measures its length **from the warp
destination**, which is the spot the player will really be standing on.

- **Tests:** `tests/vector_rope.rs::b003_a_warp_inside_the_rope_length_keeps_the_rope` (★ red
  first: *"a warp of 0.05 m cut a rope of 9.00 m. The joint had 0.0001 m to correct"* — 0 joints
  against 1) · `b003_a_warp_past_the_rope_length_still_lets_go` · `b003_the_warp_slack_is_bounded_by_the_files_own_numbers`.
  The two original `b003_` tests were green before and are green after: their warp is 55 m off a
  9 m rope.
- **In the running game:** all eight scripts that `warp` after a `hook`, A/B against one pinned
  binary with only the two keys different — `f003-ashgate`, `f004-towers`, `f019-hq`, `f025-chain`,
  `f028-why`, `f029-grapple`, `game-full` **byte-identical**, `w5-lane` different in one anchor
  (that one is `B-008`'s doing, not this rule's). **No warp in the repository changes its verdict**
  — every one of them is a teleport that lengthens the rope. The value is for the next script and
  for the `F-029` case.

### `B-002` — the mouse only turned the view correctly at exactly 60 fps

**Fixed on 2026-08-10**, test `p3_the_applied_yaw_equals_the_device_motion` was red, is green.

| Field | |
|---|---|
| **Repro** | `cargo test --test input -- --nocapture` at commit `7cf7f4b`, `[debian]`. The frame rate is set by `TimeUpdateStrategy::ManualDuration` (`bevy_time-0.19.0/src/lib.rs:118-119`), not by the machine, so the numbers below are the same on every box. **There is no `--script` repro and there cannot be one:** `src/debug/script.rs::parse_line` has no `mouse` verb, so a script can dictate an absolute `look` but never a device *delta* — and the delta is the whole subject. |
| **Evidence** | A run of *n* frames with a known mouse delta in every one of them, against the yaw that arrived on the player's `Intent`. Measured before the fix: <br>`60 fps: 300 frames, 300 ticks — raw -1.2566 rad, applied -1.2566 rad, ratio 1.000` <br>`144 fps: 300 frames, 124 ticks — raw -1.2566 rad, applied -0.5194 rad, ratio 0.413 (-58.7 %)` <br>`500 fps: 250 frames, 29 ticks — raw -1.0472 rad, applied -0.1215 rad, ratio 0.116 (-88.4 %)` <br>`20 fps: 60 frames, 179 ticks — raw -0.2513 rad, applied -0.7498 rad, ratio 2.983 (+198.3 %)` <br>`mixed 250/25 fps: 60 frames, 40 ticks — ratio 0.667 (-33.3 %)` <br>**58.7 % of the mouse motion was thrown away at 144 fps** — 124 of 300 frames reached a tick, i.e. one frame in 2.42, which is what `docs/PLAN-GAME.md` §8 predicted before anything was measured. At 20 fps every frame's motion was applied **three** times. After the fix all five runs read `ratio 1.000`. |
| **Expectation** | `docs/PLAN-GAME.md` §8, `P3`: *"applied yaw over a run equals raw device motion ± 1 % at any frame rate."* A mouse is a relative device — the sum of what it reported is the angle you turned. Nothing else in the game is allowed to depend on the frame rate either (§6 rule 6: nothing per frame, everything per second). |
| **Cause** | `src/net/local.rs:28` (before the fix) — `read_input` took `Res<AccumulatedMouseMotion>` while being registered in `FixedPreUpdate` (`src/net/mod.rs:49`). `AccumulatedMouseMotion.delta` is **assigned** once per frame in `PreUpdate` (`bevy_input-0.19.0/src/mouse.rs:257-267`; the last line of `accumulate_mouse_motion_system` is `accumulated_mouse_motion.delta = delta`, an `=`, not a `+=`). The fixed loop runs the whole `FixedMain` schedule **0..n times per frame** (`bevy_time-0.19.0/src/fixed.rs:249-255`: `while world.resource_mut::<Time<Fixed>>().expend() { schedule.run(world) }`). A per-frame value read from a schedule that runs a different number of times is dropped when the count is 0 and re-read when it is 2. |

**Why nobody saw it.** At exactly 60 fps, and only there, the two rates cancel: one frame, one
tick, `ratio 1.000`. The simulation is 60 Hz (`assets/data/game.ron:20`) and every run this
project has ever done is `--headless` or `--offscreen`, where there is no mouse at all. **The
one rate the game was ever run at is the one rate at which this code is right.**

**Why the prose said the opposite.** `src/net/local.rs:22-24` claimed *"mouse motion gathered
between two ticks is not lost in the process: `AccumulatedMouseMotion` sums it up across the
frames."* That sentence describes `MouseMotion` **messages**, which do accumulate between
readers; the resource named after them does not. The comment was not a slip of the pen — it was
the reason nobody looked.

**Fix.** `src/net/local.rs`: new resource `MouseSinceTick` and a system `gather_mouse_motion`
that adds `AccumulatedMouseMotion.delta` into it **once per frame**, registered in
`RunFixedMainLoop` in `RunFixedMainLoopSystems::BeforeFixedMainLoop` — the set that runs
"exactly once per frame, regardless of the number of fixed updates"
(`bevy_app-0.19.0/src/main_schedule.rs:401-403`) and sits after `PreUpdate` in the main schedule
order (`:224-232`). `read_input` now **takes** that buffer (`std::mem::take`) instead of reading
the per-frame resource: a frame with no tick keeps its motion for the next one, and a tick that
follows another in the same frame finds the buffer empty. The take happens **before** the
`LookOverride` branch, so a scripted absolute `look` is not dragged off its angle one tick later
by motion buffered before it. Two files changed, `src/net/local.rs` and `src/net/mod.rs`; no
other domain was touched.

*The road not taken:* moving `read_input` out of `FixedPreUpdate` into `Update`. That would have
fixed the mouse and broken the tick stamp on every `Intent` — `Intent.tick` is what the server
will discard stale input by (`src/shared/intent.rs:32-33`), and `docs/multiplayer.md` requires
the simulation to be driven from `FixedUpdate`. The buffer is the smaller change and the one that
keeps rule 4.

**Tests** — all four seen red before the fix and red again with the fix taken back out (rule 5,
step 3, with identical numbers both times):

| Test | what it pins |
|---|---|
| ★ `tests/input.rs::p3_the_applied_yaw_equals_the_device_motion` | three runs in one test — a mixed 250/25 fps pattern with frames carrying **0** and frames carrying **3** fixed steps, plus 144 fps and 60 fps. Applied yaw within 1 % of the device motion in all three. Red before: `ratio 0.667`, `0.413`, `1.000`. |
| `tests/input.rs::p3_a_frame_without_a_fixed_step_keeps_its_motion` | 250 frames of 2 ms, 31 ticks. Red before: *"88.4 % of the mouse motion was thrown away"*. |
| `tests/input.rs::p3_a_catch_up_frame_does_not_apply_its_motion_twice` | 60 frames of 50 ms, 181 ticks. Red before: *"198.3 % more yaw was applied than the mouse moved"*. |
| `tests/input.rs::p3_a_script_look_still_overrides_the_mouse` | an absolute `look` in a frame that also carries 500 px of motion wins, and the mouse has the wheel back the frame after. The guard on the fix's own failure mode — this one was **green before and had to stay green**. |

Each run ends with **one settling frame with the mouse still**, so that motion which is merely
buffered and whose tick is not yet due is not counted as lost. It hides nothing: with the defect
in place the 144 fps run is still 58.7 % short *after* the settling frame, because one extra tick
cannot give back 176 frames.

**Regression run:** `cargo run -q -- --headless --script scripts/p3-mouse.txt --ticks 600` —
`script run finished: 9 asserts held, 293 ticks`, exit 0 `[debian]`. It proves the *other* half
(the `look` override still steers the walk), not P3. No picture: **P3 is a number, and the plan
says so** (`docs/PLAN-GAME.md` §8, P3: *"— (a number, not a picture)"*).

**Two things to learn from it**, neither of which is "read the Bevy docs more carefully":

1. **A resource's name says what it accumulates, never over which schedule.**
   `AccumulatedMouseMotion` accumulates *within a frame*. Every per-frame resource read from a
   fixed schedule is this same bug — the cheapest check for the whole class is one `grep` for
   the schedule a system is registered in, next to the schedule its inputs are written in.
   `ButtonInput<KeyCode>` in the same function is the *reverse* case and is fine: a held key is
   a level, not a delta, so reading it twice or skipping it costs nothing.
2. **A test at one frame rate is a test at no frame rate.** The 60 fps case passed before and
   after the fix and would have "protected" this code forever. The rate has to be a *parameter*
   of the test, and `TimeUpdateStrategy::ManualDuration` makes it one for the price of two lines.

### `B-001` — no body in the world had an id, so no hook could hold

**Fixed on 2026-08-09**, test `f002_the_aim_names_the_body_it_hit` was red, is green.

| Field | |
|---|---|
| **Repro** | `cargo run -q -- --headless --script scripts/f-001-hooks.txt --ticks 800` at commit `b0360a2`, map `Graybox` out of `assets/data/maps.ron`, player warped to `(0, 0, 4)`, `[debian]`. Second, cleaner repro after the fact: `cargo run -q -- --headless --script scripts/b001-anchor.txt --ticks 800` — same thing with the aim horizontal instead of 8° down (see the finding below). |
| **Evidence** | `map "Graybox": 79 blocks built (9 placed, 70 generated), 63 of them anchorable` and then `hook Left of player 1 found nothing anchorable (t=112)` / `hook Right … (t=174)`. **63 anchorable blocks, and the hook caught on none of them — and `script run finished: 5 asserts held` with exit code 0.** The failure was silent. Measured a second way: `0` of `79` bodies carried a `BodyId`, and `SpatialIndex::len()` was `0`. |
| **Expectation** | `F-001`/`F-002`: a shot at a surface carrying `anchorable: true` in `maps.ron` anchors. `vector::hook::anchor_target` returns `Some((point, body))` only when `AimPoint` carries all three of `anchorable`, `point_m` **and** `body`; `docs/multiplayer.md` rule 5 says a hook holds a stable `BodyId` and never an `Entity`. So: `hook Left of player 1 anchored on body <n> at <x> <y> <z>`. |
| **Cause** | `src/world/index.rs:60-71` — `maintain_index` had an empty body ("to be filled by assignment R"). It is the **only** place in the game that hands out a `BodyId` from `IdCounter`, and the only writer of `SpatialIndex`. `src/world/index.rs:80-86` — `on_body_removed` was empty the same way. Therefore no entity carried a `BodyId`; `src/vector/aim.rs:65` queries `Option<&BodyId>` and always got `None`; `AimPoint.body` was `None` on every hit; `anchor_target` returned `None`; every shot became `ReleaseReason::NoAnchor`. |

**Why no test caught it.** `tests/vector_hooks.rs` puts its carrier into the index by hand
(`put_body`, with a comment saying the maintainer is a stub), so the hook suite never depended
on the maintainer. And `grep -n '\.body' tests/vector_aiming.rs` returned **nothing**: six
tests measured `AimPoint.point_m` to the centimetre and `AimPoint.anchorable` block by block,
and not one of them looked at the third field — the only one its single consumer needs. That
is the shape of the gap: a suite that measures what a system *computes* instead of what its
*consumer* reads.

**Fix.** `src/world/index.rs`: filled `maintain_index` (strike out the queued removals and
report them as `BodyGone`, hand every `Body` `Without<BodyId>` a consecutive id out of
`IdCounter` and insert it, re-insert everything with a `Changed<GlobalTransform>`) and
`on_body_removed` (push the id into the index mailbox). No other file changed —
`vector`, `player` and `shared` were already right and were not touched.

*The road not taken, since the file's own header pointed at it:* deleting this index in favour
of avian's `SpatialQuery`. Reason 1 of that header ("`PhysicsPlugins` is not registered") is
**stale** — it is registered at `src/lib.rs:117` and `vector::aim` already casts through
`SpatialQuery`. But the short road leads past the defect: `SpatialQuery` answers rays, not
"where does carrier 42 stand", which `vector::hook` asks twice (`hook.rs:180`, `:227`) — and it
hands out no ids at all. Deleting the grid would have removed two stubbed functions nobody
calls and left every house without a `BodyId`. Reasoning and the leftover cleanup are written
into the header of `src/world/index.rs`.

**Tests** — all five were seen red before the fix and red again with the fix taken back out
(rule 5, step 3):

| Test | what it pins |
|---|---|
| ★ `tests/vector_aiming.rs::f002_the_aim_names_the_body_it_hit` | all 63 tagged surfaces, aimed at from above: `AimPoint.body` is the `BodyId` **that very entity** carries — not just `is_some()`. Plus an untagged wall, which must be named too. Red before: *"63 of 63 tagged blocks carry no `BodyId`"*. |
| `tests/world.rs::t036a_every_body_gets_exactly_one_id` | 79 of 79 bodies carry an id, the ids are exactly the consecutive `1..=79` out of `IdCounter`, none twice, `SpatialIndex::len() == 79`, and none of it happens a second time on the next tick. Red before: *"79 of 79 bodies carry no `BodyId`"*. |
| `tests/world.rs::t036a_a_removed_body_is_struck_out_and_reported` | despawn a body: `BodyGone` arrives in the next fixed step **once**, and the index no longer holds it. `vector::hook` releases on that message. |
| `tests/world.rs::t036a_the_index_carries_the_anchorable_bit_from_the_file` | the mask in the index comes from `mask_from` and from the same `anchorable:` in `maps.ron` as `AnchorSurface` — 63 anchorable of 79, both numbers read from the plan, neither written in the test. Centre and half size too. |
| `tests/world.rs::t036a_a_body_spawned_late_is_taken_in_and_stands_right_one_tick_later` | the one-tick lag of `GlobalTransform` propagation, measured instead of argued (see the lesson below). |

**Evidence that it is fixed, in the running game and not in a test:**

```
$ cargo run -q -- --headless --script scripts/b001-anchor.txt --ticks 800     # [debian]
map "Graybox": 79 blocks built (9 placed, 70 generated), 63 of them anchorable
hook Left  of player 1 anchored on body 19 at 0.00 1.60 -10.00 (t=122)
rope Left  of player 1 attached at 14.09 m (t=122)
hook Right of player 1 anchored on body 19 at 0.00 1.60 -10.00 (t=184)
rope Right of player 1 attached at 14.09 m (t=184)
script run finished: 3 asserts held, 357 ticks
```

`0.00 1.60 -10.00` is the near face of the sand-brown 4 m block at `(0, 2, -12)`
(`maps.ron: blocks[3]`) at eye height, and `14.09 m` is the distance from the hand at
`(0, 1.6, 4)`. Before the fix the same script printed `found nothing anchorable` twice.

Picture: `docs/images/b001-anchor.png`, `--offscreen --ticks 150` (between anchoring at t=122
and release at t=173), taken **twice**, `sha256 aaf52739cebc1e62…` both times. ⚠️ **It shows
the block that was hooked, not the rope** — `src/render/rope.rs::draw_ropes` is registered and
empty, and it belongs to another job. The picture is evidence for the scene, the log line is
the evidence for the hook.

**Two things to learn from it**, neither of which is "read the code more carefully":

1. **A test suite that never asserts the field its consumer reads is not a guard.** The
   cheapest check for the whole class: for every component written by one system and read by
   another, grep the reader for the field names and then grep the tests for the same names.
   Here it was one `grep` away for a whole round.
2. **Exit code 0 is not evidence.** The broken run reported `5 asserts held` and returned 0,
   because `scripts/f-001-hooks.txt` asserted `speed < 0.5` — *the player must not move* — which
   is exactly what a hook that never catches produces. An assert that only passes while the
   feature is broken is worse than no assert.

Related: [`docs/FINDINGS.md`](FINDINGS.md) (foreign mistakes) · [`docs/STATUS.md`](STATUS.md) ·
[`docs/lessons/`](lessons/)

---

### `B-004` — **FIXED** on 2026-08-12 (the entry above is left standing as it was found)

**Fixed in two files, `src/combat/hitstop.rs` and `src/player/rope.rs`.** Four tests were red
first, and red again with each half of the fix taken back out (rule 5, step 3).

**The cause, one level deeper than the entry above had it.** It is not that avian dislikes a
joint on a disabled body — it is that disabling the body **destroys the island the joint is
counted in**: `On<Insert, (Disabled, RigidBodyDisabled)>` strips `BodyIslandNode`
(`avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:126-136`), whose `on_remove` hook removes the
now-empty island *with `joint_count` still at 1* (`islands/mod.rs:1338-1385`), and the fresh node
the thaw creates **recycles that same slot** with the count back at 0. The despawn then
decrements a zero. Full derivation: `docs/FINDINGS.md` FIND-062.

**The fix — avian's own `JointDisabled`, and the ORDER is the whole thing.**

1. `combat::hitstop::freeze` queues `JointDisabled` on every joint the body is an end of
   **before** it queues `RigidBodyDisabled` on the body. Commands are applied in queue order, so
   the joint leaves the island (`joint_graph/plugin.rs:87`) while the count is still right, and
   the island is then empty and clean when it is thrown away.
2. `combat::hitstop::advance` does it the other way round: `RigidBodyDisabled` off first, so the
   body has an island again by the time `JointDisabled` comes off and `add_joint` merges the two
   ends. The reverse order is the **second face** of the same bug and panics with
   `Neither body 1439v0 nor 441v0 is in an island` (`islands/mod.rs:820`).
3. `player::rope::attach_ropes` spawns a rope that is born **inside** an impact frame with
   `JointDisabled` already **in the bundle** — a hook can bite during the freeze, and a
   `spawn(...)` followed by a separate `.insert(JointDisabled)` is one command too late: the
   spawn triggers avian's `On<Add, DistanceJoint>` observer by itself.
4. `player::rope::shorten_ropes` skips a player carrying `HitStop` — the second face named in
   the entry above.

**What was deliberately not done:** the freeze is still avian's `RigidBodyDisabled`. `F-034`'s
claim is untouched and is now also proven **with a rope on the player**: `Position` bit-identical
for exactly `round(0.12 × 60)` = 7 ticks and moving on the 8th
(`tests/combat.rs::b004_the_freeze_is_still_bit_identical_with_a_rope_attached`) — a criterion
the old code could not have met, because the joint went on solving through the impact frame.

**Evidence.**

| | |
|---|---|
| **Red first** | `island.joint_count > 0`, `islands/mod.rs:786` in two of the three combat tests; `Neither body … is in an island`, `islands/mod.rs:820` in the third. |
| **Red again** | fix out of `hitstop::freeze`: the same two panics, the third stays green (its fix is in `rope.rs`). Fix out of `shorten_ropes`: `the rope was taken in by 2.3332 m over five frozen ticks … left: 24.400269 right: 26.73349` — and 2.3332 m is `reel_speed_m_s` 28 / 60 × 5 to four digits, which is the same arithmetic as the 0.93 m over two ticks in the entry above. |
| **Green** | `cargo test --test combat` 23 passed · `--test vector_rope` 13 passed, 4 ignored · `--test player` 25 passed. |
| **In the running game** | `cut titan 1 Torso at 28.05 m/s (t=160)` with the rope on him, `hook Right of player 1 let go: Released (t=453)` — 293 ticks after the impact frame, the release that used to abort the process — `script run finished: 1 asserts held, 531 ticks`, **exit 0**. |

⚠️ **The documented repro in the entry above cannot be run any more, and not because of this
fix.** `scripts/f-flight-cut.txt` has anchored nothing since the map became Ashgate
(`6e88eae`): `hook Right of player 1 found nothing anchorable (t=112)`, `9 of 21 asserts
failed`, exit 1 — with and without this fix. The in-game evidence above therefore comes from a
script rebuilt for the current map. **The `hook right 0.74` dodge in that file's ACT 1 is no
longer needed** — the bug it dodges is gone — but the file needs re-aiming for Ashgate before
any of its numbers, including the 74.70 m/s cut, can be measured again. `scripts/` is not this
job's to edit. → `docs/FINDINGS.md` FIND-062.

---

### ⚠️ `B-004` REOPENED 2026-08-12 — the fix INVERTED the panic, it did not remove it

**The `FIXED` block above is wrong and must not be trusted.** An independent refutation round
(it did not write the fix) found a third face that is **player-reachable**.

| | before the fix | after the fix |
|---|---|---|
| release **inside** the 0.12 s impact frame | clean | **PANIC, exit 101** |
| release **after** the impact frame | PANIC, exit 101 | clean |

**Repro, one line, against the shipped script:**

```bash
sed 's/^hook right 4.0/hook right 0.74/' scripts/f-flight-cut.txt > /tmp/b004.txt
./target/debug/defeated_by_titan --headless --script /tmp/b004.txt --ticks 400 >/dev/null 2>&1; echo $?
```

`let go: Released (t=157)` → `panicked at avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:820:
Neither body 1461v0 nor 439v0 is in an island`, **exit 101**.
Bracketed: **t=157 panics**; t=161, 167, 173, 185, 353 are all clean. The cortex cut is at t=153 and
`gear.ron: hit_stop_cortex_s` is 0.12 s = 7.2 ticks — **so the panic window is exactly the impact
frame.** Note the assertion is at `:820` (`add_joint` merging an island-less body), not the original
`:786` (`joint_count > 0`): a different assertion, the same root.

**Why the `FIXED` evidence missed it:** its "in the running game" run released **293 ticks after**
the impact frame — the safe side of the new bracket. And the shipped `scripts/f-flight-cut.txt` now
holds the hook for `4.0 s`, which releases ~200 ticks late and hides it in every run of the corpus.

**In one sentence, the thing a player does:** cut a titan's cortex while roped and let go of the
hook within an eighth of a second. That is not an edge case — it is the natural follow-through.

**Do not close this again without a bracket that sweeps the release tick ACROSS the impact frame**,
from before it to after it, in one script. The previous two fixes each moved the failure and were
each verified on the side they had moved it to.

---

### ✅ `B-004` FIXED 2026-08-12 (third attempt) — the despawn, not the freeze

**Both `FIXED` blocks above are superseded.** They are kept because the history of this bug is
the interesting part: it was closed twice and each close **moved** the abort instead of removing
it, because each was verified on the side of the bracket it had just made safe.

| release | original | after fix 1 (`JointDisabled`) | after fix 2 (order) | **now** |
|---|---|---|---|---|
| **inside** the 0.12 s impact frame | clean | clean | **abort `:820`** | clean |
| **after** the impact frame | **abort `:786`** | clean | clean | clean |

**The cause, in one sentence.** While the player carries `RigidBodyDisabled` he has no
`BodyIslandNode` and the rope's anchor is `RigidBody::Static` and never had one, so **any** avian
island transition on that joint aborts the process — and **despawning the rope entity is such a
transition that no ordering can avoid, because a despawn removes `JointDisabled` and removing
`JointDisabled` *is* the transition** (`joint_graph/plugin.rs:106`). Full matrix of body ×
joint-marker × transition, all four avian triggers, and why the freeze itself is not the problem:
`docs/FINDINGS.md` **FIND-072**.

**The fix — `src/player/rope.rs::despawn_rope`, one choke point, no `if frozen` branch.**

```rust
commands.entity(joint).remove::<DistanceJoint>();   // ← the fix, and it is this one line
commands.entity(joint).despawn();
commands.entity(anchor).despawn();
```

Removing the joint component first is a **no-op** when the joint is disabled
(`remove_joint_from_graph` early-returns on `joint_graph.get(entity)`) and the **ordinary**
removal when it is live (`joint_count` 1 → 0 with the island still standing). Either way the
despawn that follows finds no joint to re-register: avian's `Remove, JointDisabled` observer
queries `(&DistanceJoint, …)` and no longer matches. **Both columns of the table above are the
same code path now**, which is the property the last two fixes did not have. Both despawn sites
go through it — `detach_ropes` and `attach_ropes`'s defensive "a second rope on the same side".

**`F-034` was not traded away.** The freeze is still avian's `RigidBodyDisabled`; `Position` is
still bit-identical for exactly `round(0.12 × 60)` = 7 ticks **with a taut rope**
(`tests/combat.rs::b004_the_freeze_is_still_bit_identical_with_a_rope_attached`, unchanged and
green). The three alternatives that would have stopped disabling the body — `LockedAxes`,
`RigidBody::Kinematic`, and stomping `Position` back after the step — all make `combat` a writer
of `LinearVelocity` or `Position` and are argued down in FIND-072.

**The guard, and it is the deliverable: `tests/combat.rs` now sweeps the bracket.**

- `b004_the_rope_may_be_let_go_on_any_tick_across_the_impact_frame` — a **fresh app per tick**,
  release at `t+0 … t+20` after the cortex hit, all 21 must survive.
- `b004_a_rope_born_inside_the_impact_frame_may_also_be_let_go_of_at_once` — the same for a rope
  that is hung *during* the freeze and let go of before the thaw, `t+3 … t+20`.

**Do not close this a fourth time on a point test.** On today's code, taking the one line out of
`despawn_rope` kills `t+0 … t+6` — precisely the impact frame — and leaves `t+7 … t+20` green.
Any test that releases at a single tick has a 2-in-3 chance of sitting in the green part.

**Evidence.**

| | |
|---|---|
| **Red** | `7 of 21 ticks died: t+0 … t+6 Neither body 1453v0 nor … is in an island`; sweep B `4 died: t+3 … t+6`. |
| **Green** | `cargo test --test combat b004` → **5 passed** (the two sweeps plus the three older point tests). |
| **Red again** | `remove::<DistanceJoint>()` taken out of `despawn_rope`: `7 of 21 ticks died: t+0 … t+6`, sweep B `4 died: t+3 … t+6`. Same bracket, to the tick. |
| **In the running game** | the documented repro — `sed 's/^hook right 4.0/hook right 0.74/' scripts/f-flight-cut.txt` — **exit 101 → exit 0**. |

---

## B-006 — the crosshair lies for 0.18 s after every cortex kill

**Stage: 🟨 — found by reading, NOT yet reproduced in the running game.** Rule 5 says a bug
without evidence is a rumor; this entry is the hypothesis and the repro recipe, not a fix. Whoever
picks it up runs the repro FIRST and writes the measured numbers in here.

**Reported by the user, 2026-08-18, after playing:**

> *"die accuracy von anzeige zu wo seil landet ist nicht immer korrekt (was nicht gut ist)"*

The **"nicht immer"** is the clue that made this findable: the divergence is transient and it is
tied to killing something.

### The mechanism

| what | reads | schedule |
|---|---|---|
| `vector::aim::aim` — casts the hook rays | `intent.look_dir()`, **un-kicked** | `FixedUpdate`, `SimulationSystems::World` |
| `render::camera::rotate_camera` — points the image | `intent.pitch` **+ `kick.pitch_rad(tick)`** | `Update` |
| `hud::crosshair` | screen centre, i.e. wherever the camera points | `Update` |

`src/render/camera.rs:132` — `let pitch = (pitch + kick.pitch_rad(tick.0)).clamp(-limit, limit);`
`assets/data/gear.ron:70-71` — `camera_kick_deg: 3.5`, `camera_kick_s: 0.18`.

`F-034`'s kick fires on a **Cortex** hit only (`camera.rs:100`, *"Only the kill kicks"*). For those
0.18 s the image is pitched **3.5 ° down** while the aim ray is not, so **the crosshair is not the
aim point**. At `camera.fov_deg` 60 vertical that is ~5.8 % of screen height: **≈ 42 px at 720p,
≈ 84 px at 1440p** — an order of magnitude too big to be a rounding artefact.

⚠️ **The arm MARKERS are probably innocent.** They are world points pushed through
`Camera::world_to_viewport`, so they follow the kicked camera and keep landing on the true target;
`FIND-098`'s verifier measured the kick's effect on them as a `1/cos(kick)` factor, sub-pixel.
**It is the crosshair — the thing at screen centre that the player reads as "where I am
pointing" — that goes wrong.** Do not "fix" the markers.

### The repro to run first

```bash
# a script that cuts a cortex and then fires a hook inside the 0.18 s window (11 ticks at 60 Hz)
./target/debug/defeated_by_titan --headless --script scripts/<new>.txt --ticks 900 2>&1 \
  | grep -E 'MARK|assert|cut |Cortex' | tail -20
```
Cut, then `hook right` on the next tick, and compare the **fired direction** against the
**camera's forward** on that tick. Expected: they differ by up to 3.5 °, decaying to 0 over 11
ticks. `scripts/f-flight-cut.txt` already lands a cortex cut and is the cheapest starting point.

### The decision this needs, and it is a design question, not a repair

Both readings are defensible and they give opposite fixes:

1. **The kick is only an image effect and must not move the aim** — then the aim is right and the
   **crosshair must be drawn at the true aim point**, i.e. offset from screen centre while the
   kick runs. The crosshair stops being "the middle of the screen", which `src/hud/crosshair.rs`
   currently assumes throughout.
2. **What you see is what you get** — then the kick belongs in the `Intent` (or the aim must read
   the camera), and a kill genuinely nudges your aim down. That is a *feel* decision: it makes a
   kill cost you a beat, which may be exactly right for a game about committing to a cut.

**Reading (1) is the safer default** — it keeps `F-034`'s proven 🟧 evidence and touches only the
HUD — but **(2) is the one a player might actually prefer**, and only he can say.
→ `docs/QUESTIONS.md`, and it is cheap to try both because `camera_kick_deg` is one number in
`gear.ron`: set it to `0.0` and the symptom must vanish entirely. **That is also the fastest way
to confirm this is the cause at all**, and it needs no code change.

### Related

`FIND-098` and `FIND-099` each found a *different* marker lie on 2026-08-18 — the keep-out box
pinning both markers to a fixed slot, and the flying marker being drawn from the rope's tip (which
starts in the hand, on the near plane). **Both were real and both were invisible to their own
tests.** This is the third candidate in the same family, and the family's lesson is that
`F-023`'s "the marker and the rope are one number" was only ever proven for the *marker*, never
for the *crosshair*.

---

## B-007 — a titan eats your hook, and the game never says so

**Stage: 🟨 — found by reading the code, NOT yet reproduced in the running game.** The repro is
below and it is cheap. Whoever fixes this runs it first (rule 5).

**Reported by the user, 2026-08-18, after playing:**

> *"teilweise kann man gar nicht usen weil keine ahnung wieso"*

### The mechanism, and it is two failures in one

`vector::aim` casts with avian's **default `SpatialQueryFilter` — no layer mask, no predicate**
(`src/vector/aim.rs:32`, deliberately: *"hit first, then check anchorable"*, so a rope can never
go **through** an untagged wall). It therefore hits **every** collider, and a titan is a stack of
capsule colliders plus a cortex sensor (`src/titan/rig.rs:355-370`).

**But no titan entity carries a `shared::Body`.** `grep -rln 'shared::Body\|Body {' src/titan/`
returns nothing; the rig carries a marker struct `TitanBody` (`rig.rs:98`), which is a different
thing entirely. So:

```rust
// src/vector/hook.rs:95
pub fn anchor_target(aim: &AimPoint) -> Option<(Vec3, BodyId)> {
    if !aim.anchorable { return None; }     // ← titan carries no ANCHORABLE mask
    Some((aim.point_m?, aim.body?))         // ← and no BodyId at all
}
```
→ `None` → `ReleaseReason::NoAnchor` → **the arm stays `Idle` and nothing happens.**

1. **You cannot hook a titan.** That is `F-029` *Dynamische Ankerpunkte* (⬜ Unbuilt), whose
   description names exactly this: *"Ankerpunkte an bewegten Objekten: Titanenkörper (Schulter,
   Arm, Nacken), fahrende Eskortkarren, Hafenkräne, Mauerlifte."*
2. 🔴 **Worse, and nobody has written this down: a titan BLOCKS the hook.** The ray stops at the
   nearest solid hit, so a perfectly good wall **behind** a titan is unreachable. Ashgate is
   100 % anchorable, so the player's whole mental model is "everything holds" — and then the one
   moment it silently does not is the moment a 10 m body is between him and the city.

**Both failures happen precisely when a titan is in the line of fire, i.e. whenever he is
fighting.** That matches *"teilweise"* exactly, and it explains why it feels random: it depends on
what is in front of the crosshair, not on what he pressed.

### The repro

```bash
# aim at a spawned husk from ~15 m and pull both triggers; then aim at a wall BEHIND it
./target/debug/defeated_by_titan --headless --script scripts/<new>.txt --ticks 600 2>&1 \
  | grep -E 'MARK|assert|NoAnchor|hook|Idle' | tail -20
```
`scripts/f056-husk.txt` already spawns one and is the cheapest starting point. Expected:
`ReleaseReason::NoAnchor` on both pulls, with the arm never leaving `Idle`.

### The fix is two decisions, not one

- **Feedback** is `F-028` *Fallback ohne Kandidat* (⬜), whose acceptance is the user's sentence
  turned around: *"Kein Tastendruck bleibt ohne Rückmeldung; der Spieler versteht immer, warum
  kein Haken gesetzt wurde."* **This half is not optional and it is the cheap half** — a sound and
  a HUD state would have told him in one sortie what took a code read to find.
- **Whether a titan should hold a rope at all** is `F-029` and it is a *design* question with real
  consequences: hooking a titan is a central move in the genre, but it needs anchors that ride a
  moving body and detach cleanly on death (`F-029`'s own acceptance), and the pack ships
  `a-064-zone-{cortex,gelenk,huefte,riss}` hit-zone empties that look built for exactly that.
- **The blocking half needs an answer either way**, and it is independent of `F-029`: if a titan
  is not hookable, should the ray *pass through* him to the wall behind? "Hit first, then check
  anchorable" exists so a rope cannot go through a **wall**; a titan is not a wall, and treating
  him as one costs the player the city. → `docs/QUESTIONS.md`.

### B-007 — ✅ FIXED 2026-08-19 (`F-029` landed)

Both halves are closed, and the fix was **one component**: `shared::Body { mask: SOLID |
ANCHORABLE }` on the titan rig root (`src/titan/rig.rs`). Everything else this needed had been
standing unused — `vector::aim` always hit the capsule, `world::index::maintain_index`'s third
block (`Changed<GlobalTransform>`) was written for `F-029` and had nothing to update, and
`HookState::Anchored { body, local_m }` already resolves as `index.body(id).center_m + local_m`.

- **You can hook a titan.** `tests/titan.rs::f029_a_rope_bites_a_walking_titan_and_rides_him`: a
  husk walks **2.910 m** and the anchor follows to within **0.0389 m** (tolerance 0.0485 m = one
  tick of his own travel). A world-nailed anchor misses by the full 2.910 m.
- **A titan no longer hides the wall behind him**, because he *is* an anchor now.
- **Release on death** goes through the existing channel — the `Body` comes off at
  `TitanState::Death`, `world::index`'s `on_remove` observer fires, `ReleaseReason::BodyGone`.
  `scripts/f029-grapple.txt`: `cut titan 2 Cortex at 9.75 m/s (t=373)` → `let go: BodyGone (t=375)`.
- **Cost:** 159 ns per walking titan per tick, O(1) in world size. 100 titans = 0.095 % of a tick.

⚠️ **The "mit Feedback" half of `F-029`'s acceptance is only a log line.**
`hud::arm_aim::sense_arm_miss` matches only `ReleaseReason::NoAnchor(_)`, so a rope torn away by a
death says nothing on screen. One arm on an existing `match` in `src/hud/`.

---

## B-008 — a downward hook is unaimable and lands somewhere else in silence

**Stage: 🟨 — measured by the `F-029` round (`FIND-116`), not yet reproduced on its own.**

Aiming **down** does not do what the crosshair says. The arm rays sit on a fan around the look
direction (`F-023`), and at the shipped setting that is ~12.9° off centre per arm — so a shot aimed
at the ground under you resolves to whatever the *side* ray finds, which near a roof edge is not
what you were looking at. Measured:

```
left arm bit a roof 8.5 m aside      — body 150 at (38.00,  0.39,  -8.53)
from 520 m up, a tower top 429 m off — body  77 at (29.81, 123.00, -96.31)
```

🔴 **And `F-028`'s fallback cannot save it.** "No candidate ⇒ fall back to the centre ray" never
triggers in Ashgate, because the district is **100 % anchorable** (the user: *"überall! ohne
ausnahmen!"*) and the **ground is always under the cone** — so there is always *a* hit, just not
the one the player asked for. The two design decisions are individually right and fight each other
here.

This is `F-024`/`F-025` territory (the snap now picks a best candidate) and it wants the aim-assist
knobs pointed at it: does the scoring function prefer *what the crosshair is on* strongly enough
when the player looks straight down? → `docs/QUESTIONS.md`, and it is worth asking the user, since
he is the one who will feel it.

**Related:** `B-003` releases **every** rope on any `warp`, whatever the distance — that cost the
`F-029` round two runs and is documented nowhere a script author looks.


### B-008 — ✅ FIXED 2026-08-19 · the fallback now asks the right question

**`F-028`'s fallback asked *"did this side ray find anything anchorable?"*.** In Ashgate the
answer is never no — the district is 100 % anchorable and the ground is always under the cone — so
a side ray that had left the surface the crosshair stands on carried on and bit whatever it met.
**The question is generalised, not replaced:** *"did this side ray find the thing the crosshair is
on?"* (`src/vector/aim.rs::side_hit_is_coherent`), answered by the body id first and, for a
different body, by whether the hit is within `vector.aim_side_coherence_k` × the separation the
fan itself asked for (`d · sin(half)`).

**Where a downward shot lands now.** 30 m over the street at `(168.19, ., -50.12)`, looking
straight down, `scripts/b008-down.txt`:

```
before   hook Left  anchored on body 2154 at 164.53 12.34 -50.44     the roof cap, 11.50 m off
         hook Right anchored on body 2156 at 172.01 11.50 -50.46     the other one, 10.77 m off
         assert Height < 6 — measured 11.839          1 of 4 asserts failed, exit 1

after    hook Left  anchored on body 573 at 168.19 1.50 -50.63       the pavement, 0.00 m off
         hook Right anchored on body 573 at 168.19 1.50 -50.63
         script run finished: 4 asserts held, 290 ticks              exit 0
```

The fan is **untouched** — `FIND-096`'s narrowing stands, `side_dirs` still has exactly one
production caller, and `ArmAim` is still written once, so what the rope flies at and what the
marker shows is still one number (`F-023`).

- **Tests:** `tests/vector_aiming.rs::b008_a_shot_aimed_straight_down_lands_on_what_the_crosshair_stands_on`
  (★ red first: *"the Left arm aims at … body 2154 — 11.50 m off a crosshair that stands on body
  573 and asked for 5.85 m"*; red again with the tolerance set to `f32::INFINITY`) ·
  `b008_a_side_hit_that_left_the_crosshairs_surface_is_not_a_target` (the predicate against
  typed-in points).
- **Measured, and it is why the rule is this one** — the ratio of the real hit to what the fan
  asked for, from the same spot: level aiming (`-70°`, `-28°`) reads **1.02×** and is kept;
  straight down from 30 m reads **1.84×** and **1.96×** and is refused; straight down from 15 m
  reads **1.30×** and **2.34×**.

⚠️ **What it does NOT fix, and it is a feel question → `docs/QUESTIONS.md` Q-039.** Straight down
from **60 m** the two roofs beside the street sit **1.21×** and **1.25×** off the crosshair — which
is genuinely inside what the fan asked for at that range — so they are still what the arms take.
Whether a steep pitch should collapse the fan toward the centre outright is the user's call.

---

## B-009 — a body put inside the terrain is frozen, and nothing in the game says so

**Found 2026-08-19 while repairing `scripts/f004-towers.txt` (`docs/FINDINGS.md` FIND-126).
Not fixed — this is the write-up, and the repro is four lines.**

Ashgate got terrain on 2026-08-18 and the floor of the gantry lane settles at **2.400**. A
player placed **below** that surface does not get pushed out, does not fall, does not walk, and
gets **no log line, no message and no HUD state**. He is simply gone as an actor.

```
warp 0 0.5 257.5      # the lane floor here is 2.400 — this is 1.9 m INSIDE it
look 0 0
key W 1.5
wait 1.0
assert speed > 5.9    # measured 0.000  (and `height` reads -0.000, not 0.5)
```

**Measured, same run, same binary:** `speed 0.000` at three stands (`0/0.5/257.5`,
`0/0.5/257.5` at yaw 90, `6/0.5/250`) — and **6.000 m/s** at the church (`45, 1, -43`) and on
the boulevard (`0, 2, 100`). Dropped in from `y = 20` at the same spot he lands at 4.780 and
runs normally. **The runner is fine; the placement is not, and the silence is the bug.**

**Why it matters beyond a debug verb:** `warp` is a script verb, but a spawn point, a mission
deploy position or a respawn is the same operation with the same failure mode, and the symptom
a player would report is *"I could not move"* with nothing in the log to find. `B-007`'s lesson
applies exactly: **the failure is not the geometry, it is that nothing says so.**

**What a fix would have to decide** (not mine — `src/**` belongs to another agent this round):
whether a body found under the surface is lifted to it, refused with an error, or logged once —
and the script verb `warp` is the cheapest place to make it loud.

**Evidence:** `docs/FINDINGS.md` FIND-126 §2 · `scripts/f004-towers.txt` ACT 0, which now warps
in from `y = 6` and asserts the floor at `2.0 .. 2.9` as a terrain fact of its own.

---

## B-010 — a team mate blocks your rope, and it reads as a miss — 🟧 FIXED 2026-08-19

**Found by the round gate on 2026-08-19, two red tests in `tests/vector_aiming.rs`:**

```
f002_a_side_ray_that_finds_nothing_falls_back_to_the_centre_ray
f002_q_and_e_can_target_two_different_points
   "the centre ray landed at Vec3(0.0, 1.5999687, 0.0)"   ← the other player's eye, distance ~0
```

**The cause is a ground rule doing its job.** `docs/multiplayer.md` F-163a says *no collision
between players*, and since `player::spawn_player` wears
`CollisionLayers::new(LAYER_PLAYER, PLAYER_COLLIDES_WITH)` two players **stay overlapping**
instead of shoving each other apart. `vector::aim::cast` excluded only **its own** player and
cast on mask `ALL`, so the ray now started *inside* the team mate's capsule and `solid: true`
answered it at distance 0 — the aim point was the caster's own eye height, and the rope had
nowhere to go.

**Controlled, both directions, same binary otherwise:**

| change | `--test vector_aiming` |
|---|---|
| working tree | **2 failed** — aim point `(0.0, 1.5999687, 0.0)` |
| `CollisionLayers` line commented out, rebuilt | 22 passed — the players shove apart again |
| line restored, `AIM_RAY_SEES` added | 22 passed |
| `AIM_RAY_SEES` removed again, line kept | **2 failed**, identical message |

⚠️ **This was never only a test's problem, and the shove was never the fix.** Two players who
stand apart still block each other's rope the moment one crosses the other's line — and a
player carries no `shared::Body`, so `vector::aim` resolves him to `anchorable: false`: the
rope does not anchor on him, it *dies* on him and hides the wall behind. That is `B-007`'s
shape exactly, where a titan ate the hook. `docs/multiplayer.md` had it written down as the
next thing that would be wrong, untested, on the same day.

**The decision: a player's body is AIR to a hook ray.** Not a surface, not an obstacle, not a
line-of-sight blocker. `shared::AIM_RAY_SEES` is `ALL & !LAYER_PLAYER`, and both unfiltered
casts in the game now carry it — `vector::aim::cast` (the crosshair and both side rays) and
`vector::hook::anchorable_beyond_reach` (the "out of reach" probe behind `F-028`'s miss
reason). Untagged geometry is avian's bit 0 and stays in the mask, so **nothing but a player
answers differently**; `blades::cut` and `hud::crosshair` already cast on titan masks and were
never affected, and F-162a (no damage between players) is untouched.

**What it does NOT decide:** whether a player should block a *blade*, a *bullet* or the
camera. Only the hook ray was asked and only the hook ray was answered.

**Evidence:** `tests/vector_aiming.rs` 22/22 · `src/shared/layers.rs::
a_hook_ray_sees_the_world_and_a_titan_but_never_a_player` · `docs/FINDINGS.md` FIND-131

## B-011 — the anchor field lettered `Q` and `E` for a key that goes somewhere else — ✅ CLOSED 2026-08-28, **WONT FIX**: the field itself is deleted

> 🔴 **CLOSED BY DELETING THE SUBJECT, 2026-08-28.** The open half below — *"what earns the
> letters back: `F-024`, wire `vector::hook::fire` to `AnchorField::best_of(..)"* — **is not
> going to be built, and it was the wrong idea rather than an unfinished one.** The user,
> 2026-08-27:
>
> > *„es soll auf jeglicher oberflqche einhaken. nicht an hardcoded punkten etc!"*
>
> Hooking is a property of a **surface** (`shared::AnchorSurface` / `BodyMask::ANCHORABLE`),
> and where the rope bites is `vector::hook`'s raycast — which is what the game already did
> and what the letters `Q`/`E` already obeyed. So the disagreement this entry recorded is
> resolved in favour of the ray: `src/world/anchor.rs` (787 lines) and
> `src/hud/anchor_marks.rs` (666 lines) are **deleted**, together with `scripts/f026-marks.txt`
> and its two frames. `F-024` is not to be built; `F-026`/`F-027` lose their subject.
> Recorded in `docs/QUESTIONS.md` Q-067 and `docs/PLAN.md` §0; the deletion is
> `docs/FINDINGS.md` FIND-197.
>
> **What survives, and it is not the same thing:** the **aim assist** — `vector::aim`'s
> sideways sweep of the ray (`probe_dirs`, `pick_best`, `score_candidate`, `required_margin`)
> and the two settings `assist_catch_pct` / `assist_strength_pct`, drawn as the search band by
> `hud::catch_band`. That layer never read the point list; it searches **surfaces** off the
> crosshair's own row, which is exactly what the user asked for twice.
>
> **The guard that survives too**, rewritten to stand without the field:
> `tests/hud.rs::f026_exactly_one_q_and_one_e_are_on_the_screen_and_they_are_the_arms` — five
> looks, ten key labels, every one of them owned by an arm. The rule it exists for outlives
> both features: **never draw a promise the game does not keep.**


**Reported by** the refutation round of 2026-08-26, against `F-026`'s own acceptance sentence:
*„Ein Testspieler kann jederzeit ohne Nachdenken sagen, wohin Q und E ihn bringen wuerden."*

`src/hud/anchor_marks.rs` drew the best candidate of each hemisphere — straight out of
`world::AnchorField::best_of` — as a big cyan ring carrying the letter `Q` or `E`. **`F-024`,
the feature that makes the hook fire at that candidate, is unbuilt.** `vector::hook::fire`
takes `vector::aim`'s raycast target, a probe sweep over colliders that has never heard of
`AnchorField` (`grep AnchorField src/` outside `src/world/` finds only the HUD file). So the
letter and the rope named two different points, and both were on screen at once.

**Repro** (pinned binary, the element's own stand, `scripts/f026-marks.txt` leg 1 — the
script is deleted with the feature; the stand is `warp 51 0 13` / `look 0 0`):

```
the game's log:  hook Left anchored on body 980 at (51.00, 1.65, -1.00), 14 m dead ahead
that point projects to                              (640.00, 357.77)
anchor_marks' `Q` ring was drawn at                 (445.93, 352.07)  -> 194.15 px = 17.30 deg
arm_aim's `Q` — the one the key OBEYS — at          (639.50, 375.50)  -> within 0.5 px in x
```

and in the harness, one stand, one frame:

```
cargo test --test hud f026_exactly_one_q_and_one_e 2>&1 | grep -E 'glyphs at|test result'
  the screen carries 2 `Q` glyphs at [Vec2(470.5, 362.0), Vec2(616.0, 425.0)]   (156.9 px apart)
  and 2 `E` glyphs at                [Vec2(806.5, 362.0), Vec2(755.0, 638.0)]   (279.7 px apart)
```

**What was fixed, and what was NOT.** The *claim* came off, not the field. `MarkState::BestLeft`
and `MarkState::BestRight`, the `Q`/`E` labels and the `BEST_PX` rings are gone; the honest half
— the candidate field, `F-027`'s density cap and thinning, the distance fade, the off-switch,
the anchored rope's filled disc, the 0.0 px placement — is untouched and still measured.
**The snap itself is still missing**, and that is this entry's open half.

⚠️ **The fix is deliberately NOT "move the ring onto the ray's target".** That would be
`hud::arm_aim` drawn a second time and would put the anchor field back in the state FIND-160
found it in — 1520 authored points with no consumer that means anything.

**What earns the letters back:** `F-024` — wire `vector::hook::fire` to
`AnchorField::best_of(.., Hemisphere::Left|Right, 1)`. The day `Q` really flies at the ring,
`BestLeft`/`BestRight` and their labels come back with it and `F-026`'s acceptance sentence
becomes answerable for the first time. Until then the element says *"there is an anchor here"*
and nothing about a key.

**Guards, both of which go red on one line** (`Text::new("Q")` added to the mark spawn):
`tests/hud.rs::f026_exactly_one_q_and_one_e_are_on_the_screen_and_they_are_the_arms` (13 `Q`s,
12 of them owned by nothing) and `tests/hud.rs::f026_the_field_marks_name_no_key`.

**Evidence:** `tests/hud.rs` 53/53 · `--lib` 271/271 · before/after `--offscreen` frames at the
same stand, same data, pinned old binary vs new: **exactly two clusters differ, each 20x32 px**
(a `BEST_PX` ring plus its `LABEL_PX` letter), 0.058 % of the frame, and the removed `Q`
cluster's centre `(446.5, 345.5)` lands within 1 px of the ring measured above.
`docs/images/f026-marks-square.png` (deleted with the feature, 2026-08-28) ·
`docs/FINDINGS.md` FIND-178

---

## B-012 — `f175-loop` reported `11 of 19 asserts failed` twice, then went green nine times

> ⚠️ **PARTLY RESOLVED, 2026-08-27, and the resolution is the supervisor's own mistake.**
> A SECOND cause of this exact symptom was found and it is **fully explained and deterministic**:
> commit **`1ca7d26`** ("docs: the frame of reference is the anchor, and gravity is his number")
> raised `gravity_m_s2` **−20 → −32** and `jump_speed_m_s` **6.5 → 8.2**, **while a round was
> live**. Measured by an adversary against **one pinned binary**, before → after:
>
> | script | before | after |
> |---|---|---|
> | `f175-loop.txt` | 19 asserts held, **15 of 15 runs** | **10 of 19 failed, 9 of 9 runs** |
> | `f177-door.txt` | 13 asserts, exit 0 | **5 of 13 failed**, exit 1 |
> | `f070-hub.txt` | 42 asserts, exit 0 | **16 of 42 failed** |
>
> This is **not** a flake: same binary, same data, deterministic on both sides. It is the
> gravity change doing exactly what its own comment said it would — *"it moves every fall in the
> script corpus"* — and it is the trap `CLAUDE.md` records twice and that the supervisor had
> written into that very round's commission (*"DO NOT EDIT assets/data/*.ron"*) **an hour before
> editing it himself.**
>
> 🔴 **The original observation below is NOT explained by this.** It was logged at **23:15 UTC**,
> and `game.ron` was written at **02:40 local**, i.e. afterwards. So there are **two causes of one
> symptom**, one of them known and one still open, and the known one must not be allowed to
> retire the unknown one. **Do not close this entry when the corpus is re-aimed.**
>
> **What the re-aim must not do:** loosen an assert. A 30.8 m drop now takes 1.39 s instead of
> 1.76 s; the numbers move because the world got heavier, and the brackets get re-derived from
> the new constant, not widened until green.

### the original observation, still unexplained

**Observed 2026-08-27 [offlinebot]** by an adversarial verifier who was measuring something else,
and reported **because it touches evidence quality**, not because it can be attributed to any round.

**What happened.** In one batch at 23:15 UTC, `scripts/f175-loop.txt` reported
`11 of 19 asserts failed` and `scripts/f070-hub.txt` reported `15 of 42 asserts failed` — **each
twice in a row, exit 1** — with the **MARK ticks identical to the green runs**. Nine subsequent
`f175-loop` runs and a `f070-hub` run were green, including at load average 10.8 and immediately
after a forced relink.

**Why this matters more than a flaky test usually does.** `f175-loop` is the script the whole
hub→sortie→debrief→hub ring rests on, and it is quoted as evidence in several `FIND-` entries and
commit messages. **"19 asserts, exit 0" now has at least one observed counter-run on this machine.**
Every claim resting on a single green run of it is weaker than it reads.

⚠️ **The MARK ticks being identical is the interesting part.** The script reached the same places at
the same ticks and then disagreed about what it found there — so this is not a timing drift in the
script driver. Candidates, none checked:

- **load-dependent physics** — the machine was shared with an `elderfell_3d` session at `nice -4`
  (`docs/lessons/environment.md`), and `FixedUpdate` under starvation can run a different number of
  steps per frame;
- **`saves/player-1.ron`** — every run writes it and the path follows `CARGO_MANIFEST_DIR`, so runs
  share a career profile that advances (159 → 164 sorties in one session). An adversary checked
  this incidentally and found every assert matched to three decimals across profiles 0→4 and
  159→164, so it is **unlikely but not excluded**;
- **Bevy's nondeterministic startup ordering**, which the same adversary saw as the only difference
  between otherwise byte-identical logs.

### No repro yet, and that is the honest state

**`CLAUDE.md` rule 5 says a bug without evidence is a rumor — this one has evidence but no repro**,
so it is filed rather than fixed. **Do not "fix" it by re-running until it is green.**

**What would settle it, cheapest first:**

```bash
# 1. does it survive repetition at all?
for i in $(seq 1 40); do
  ./target/debug/defeated_by_titan --headless --hub --script scripts/f175-loop.txt --ticks 1800 \
    >/dev/null 2>&1; echo "$i:$?"
done | grep -v ':0$'          # anything printed is a reproduction

# 2. if it reproduces, does it stop reproducing with a fresh profile each run?
# 3. and does it reproduce under artificial load (the elderfell case, nice -4)?
```

⚠️ **Run it against a PINNED binary** (`cp target/debug/defeated_by_titan $SCRATCH/dbt-pinned`) —
a rebuild underneath a 40-run loop has destroyed two measurement series in this project already.

**Related:** `docs/lessons/environment.md` (the shared machine) · `F-175` `F-177` · `FIND-188`

---

## B-013 — holding `Ctrl` with two hooks drags you onto one anchor and leaves the other rope 50 m past its own maximum — 🟧 FIXED 2026-08-28

**Filed 2026-08-28. It was found on 2026-08-27 and it has lived only inside `docs/FINDINGS.md`
(`FIND-191`) since — a 108 kB file this project's own rules forbid opening whole, so nobody could
act on it. `grep -n 'FIND-191' docs/QUESTIONS.md docs/BUGS.md docs/NEXT.md` returned nothing for a
day. A finding that is not promoted into a queue file has not been found** (`docs/PLAN.md` §6,
cause 5). This entry is that promotion, and the fix landed with it.

### The symptom

Two hooks on far-apart anchors, `Ctrl` held: the player is dragged onto ONE anchor, sits there at
**0.000 m/s**, and the OTHER rope is **50.167 m past its own maximum**. Both ropes are still
attached and neither is released, so nothing in the game says a word.

### The cause, and it is one sentence

`player::rope::shorten_ropes` walked **both** `limits.max` toward `vector.min_rope_m` = 3 m at
`vector.reel_speed_m_s` = 28 m/s per arm, while the two anchors stayed where they were. **Two
maxima have a common solution only where `L_left + L_right >= d_anchors`** — the triangle
inequality. Below it no point in space honours both, so avian keeps one arm and abandons the
other. With two anchors 56 m apart both maxima reach 3 m within one second.

**It predates the joint on `Drive`** — `Pendulum` has carried a `DistanceJoint` since `F-004` and
agrees to three decimals — but since `Q-058` shipped `rope_force_model: Drive` with a real joint,
a player walks into it with **one key**.

### The repro

```bash
# the running game, and it is one line that goes red (ACT 2, `assert speed > 2`):
./target/debug/defeated_by_titan --headless --script scripts/f012-tworopes.txt --ticks 1400 \
  2>&1 | grep -E 'MARK|measured|script run finished' | tail -8
./target/debug/defeated_by_titan --headless --script scripts/f012-tworopes.txt --ticks 1400 \
  >/dev/null 2>&1; echo $?
# and the 84-cell measurement, which is where the metres are (the harness has no length metric):
nice -n 15 cargo test --test vector_rope -j 3 f004_two_far_apart_anchors -- --nocapture \
  --test-threads=1 2>&1 | grep -E '^test result|panicked|totals|^ *an arm'
```

Against a binary with the fix's one line deleted, ACT 2 reads **`assert Speed > 2 — measured
0.001`** and the run exits **1**; ACT 3, the one-rope control, still holds.

### The fix — `player::rope::hold_the_pair`, inside the one writer of `limits.max`

The reel may not take a step that puts `L_left + L_right` below the anchor separation. The
give-back is proportional to what each arm asked for, and `vector.min_rope_m` is re-applied after
it, so the per-arm floor still wins where it applies. `shorten_ropes` stays the **single writer**
of `limits.max` (`docs/architecture.md`, authority table; §6 rule 3) — no second system was added.

| over 84 cells, `Ctrl` held, 10 080 ticks | before | after |
|---|---|---|
| worst arm past its own maximum | **51.7104 m** | **0.0092 m** |
| ticks where no position satisfies both maxima | 7 920 (78.6 %) | **0** |
| ticks pinned under 0.05 m/s **by a violation** | 5 208 | **0** |
| ticks pinned at all (what the user chose) | 5 208 | 3 |

⚠️ **The rule keeps one `vector.min_rope_m` of play in hand on top of the separation, and that
term is an `ASSUMPTION`** — see `docs/QUESTIONS.md` Q-079. At exactly `L_l + L_r = d_a` the two
spheres are tangent, the feasible set is a **single point**, gravity pulls at a right angle to
both constraint gradients, and the solver leaves 0.2810 m of sag. The margin turns the point into
a small lens — a hanging V instead of a tightrope — and takes the sag to 0.0092 m.

**Guards:** `tests/vector_rope.rs::f004_two_far_apart_anchors_hold_the_player_instead_of_dragging_him_past_a_maximum`
· `scripts/f012-tworopes.txt` (ACT 2 the claim, ACT 3 the control).

**Related:** `FIND-191` · `FIND-195` · `FIND-186` · `Q-058` · `Q-079` · `F-004` · `F-005`

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

## B-015 — the map has no edge: past the plate there is **nothing**, and you fall forever — 🟧 FIXED 2026-08-28

**Reported by the user, 2026-08-27:** *„und man kann an der seite einfach runterfallen!"* — and,
asked what should be there, he named **both** mechanisms:
*„unsichtbare wand + wenn man runterfaellt wegen bug teleport man zurueck!"*

### The cause, and it is the first of the four candidates and not the interesting one

Ashgate's ground is two slabs, `x ∈ [-350, -85]` and `x ∈ [-55, 350]`, `z ∈ [-350, 350]`
(`assets/data/maps.ron`). They cover the declared `size_m: (700.0, 700.0)` **exactly** — the plate
is not too small, it is not a seam, and nothing tunnels. Past `|x| = 350` / `|z| = 350` there is
simply **no collider of any kind**, at any height, and nothing under it either. The deepest block
in the whole map is the channel floor with its underside at **-4.20 m**; below that the world is
empty down to `-inf`.

### The repro — `tests/player.rs::f012_the_ground_ends_at_the_map_edge_and_nothing_holds_you_past_it`

Built on `maps.ron: current`, the district that ships. Twelve stances: four directions
(`+x -x +z -z`) x three distances past the edge (`1, 5, 25 m`), each dropped from `y = 20` and
given 2 s. **12 of 12 fell clean through**, every one of them to `y = -44.0` — i.e. free fall at
`gravity_m_s2 = -32`, nothing hit:

```
+x +1 m -> y = -44.0    -x +1 m -> y = -44.0    +z +1 m -> y = -44.0    -z +1 m -> y = -44.0
+x +5 m -> y = -44.0    -x +5 m -> y = -44.0    +z +5 m -> y = -44.0    -z +5 m -> y = -44.0
+x +25 m -> y = -44.0   -x +25 m -> y = -44.0   +z +25 m -> y = -44.0   -z +25 m -> y = -44.0
```

**One metre past the edge is already outside the world.** There is no gradual boundary and no
warning.

### And the two speeds, because they fail differently

* **Walking**, `run_speed_m_s = 6.0`, held `D` for 4 s from 12 m inside the `+x` edge:
  `y = -62.96`. `tests/player.rs::f012_walking_at_the_edge_of_the_map_does_not_walk_you_off_it`.
* **`vector.max_speed_m_s = 75`**, launched level from 60 m inside the edge:
  **4 of 8 launches ended 362.82 m out** — 12.82 m past the half-extent, still at `y = 24.45`,
  i.e. already flying over open nothing.
  `tests/player.rs::f012_flying_at_the_edge_at_top_speed_does_not_tunnel_through_it`.

### And the half that gets forgotten

`tests/player.rs::f012_a_player_below_the_world_comes_back_to_where_he_last_stood` puts the body
2 m and 90 m below the recovery depth **by hand** — the way a bad warp or a seam does, with the
fence already defeated. Before the fix: *"dropped 2 m under the recovery plane (-300 m) he came
back to `Vec3(0.0, -302.28595, 0.0)`, not to the ground he last stood on
(`Vec3(0.0, 0.049950004, 0.0)`)"*. **Nothing in the game brought him back**, and no test in the
repository had ever asked.

### The fix

Both, and they are two mechanisms:

1. `world::bounds::build_bounds` — four invisible static boxes at the map's own edge, out of
   `maps.ron: bounds`. No `Block`, so `render` draws nothing; no `AnchorSurface`, so a hook that
   reaches one finds nothing to hold.
2. `player::recovery` — `SafeGround` on the player records the last tick he was `Grounded`;
   below `bounds.recovery_plane_y_m` he is sent back with a `WarpPlayer`, through the one existing
   writer of a player's `Transform`.

**What tells a fall out of the world from a legitimate 120 m dive is the DEPTH**, not the fall
distance and not the speed — the plane lies 295.8 m below the deepest block in the map.
The control is `tests/player.rs::f012_a_dive_from_the_top_of_the_wall_is_not_a_fall_out_of_the_world`,
and it was **green before the fix as well** — which is what makes it a control and not a second
copy of the feature.

**Evidence:** `scripts/f012-edge.txt` · `tests/player.rs::f012_*` (5) · `tests/world.rs::f012_*` (3)
**Related:** `F-012` · `docs/FINDINGS.md` FIND-199

---

## B-016 — the map edge was closed sideways and left open **upward**: you fly over the fence in seven seconds, and its top face is a standable ring outside the world — 🟧 FIXED 2026-08-28

`B-015` was closed with two mechanisms, a fence and a recovery plane, and both of them work. The
feature was still wrong, because **the recovery asked only about depth, and leaving is not only
downward.** Three symptoms, one hole.

**1 · You simply fly over it.** `fence_top_m` is 200 m. Height ladder from a standing start at
`(0, 2, 0)`, look straight up, holding only `W` + `Shift`:

| 1 s | 2 s | 3 s | 4 s | 5 s | 6 s | 7 s | 8 s |
|---|---|---|---|---|---|---|---|
| 12.2 | 49.9 | 54.6 | 71.7 | 114.5 | 182.5 | **259.3** | 336.3 m |

Gas for the first six seconds: `15000.000 -> 14891.771`, **0.72 % of one tank**. And there is no
apex to quote: held long enough the body sits at `vector.max_speed_m_s` going up — measured
**748.9 m per 10 s for 179.88 gas**, i.e. 4.163 m per unit, **62 580 m on one tank**. The sentence
written into `assets/data/maps.ron` for the 200 — *"far above anything the gear reaches on gas
alone"* — was wrong by 3.3x in seven seconds and by 313x on a tank.

**2 · The keyboard-only escape.** Two held keys from the spawn point, no warp except onto the
spawn: 7 s of `W`+`Shift` at pitch 89 -> **243.143 m**; `look -90 20` and 9 s -> **284.175 m and
outside the map**; 8 s later -> **-233.201 m, still falling through nothing**; the plane finally
fired at -300.6 m. That is **roughly ten seconds of falling out of the world**, which is the user's
original complaint (*„und man kann an der seite einfach runterfallen!"*) intact, upward.

**3 · The fence's top face is a solid, invisible, standable ring outside the map.** A body at
`(355, 210, 0)` comes to rest at **exactly y = 200.000** and is still there 14 s later; the same at
the corner `(355, 210, 355)`; 8 s of `W` along it leaves him at 200.000. He is outside the
footprint, so `record_safe_ground` correctly refuses to record him, and he is 500 m above
`recovery_plane_y_m`, so `recover_the_fallen` never fires. **Parked outside the world indefinitely,
on a 10 m wide invisible ledge running the whole way round the district at 200 m.**

**4 · And one latent, one RON number away.** `record_safe_ground` gated the footprint on
`map.size_m * 0.5 + bounds.fence_margin_m` — the **fence's** footprint, not the map's — inside a
file whose own header says it knows nothing about `world::bounds`. With `fence_margin_m = 0` they
coincide; with `500` the recovery returned the player to `(350.10, 120.50, -120.0)`, 0.1 m past the
end of the coping with nothing under it, and he fell out again and had to be recovered twice.

### The cause, in one sentence

**A player outside the map's footprint is out of the world whatever his height**, and the recovery
only knew how to ask "how far down".

### The fix

`player::recovery::out_of_the_world(map, p)` — **one pure function, called by both systems**, so the
question is asked once and cannot drift (`CLAUDE.md` rule 5's corollary). It answers `UnderThePlane`
below `recovery_plane_y_m` and `PastTheEdge` outside `map.size_m` **at any height**, and it reads
`size_m` **alone**, which closes 4 as well.

**The grace at the edge is zero, and the zero is derived, not chosen:** any grace of `g` metres is a
standable ring `g` metres wide on the fence's top face, so the width of the grace is the width of
the bug. It is affordable because the tolerance is on the inside — the fence's inner face stands
`fence_margin_m` outside the map edge and the solver keeps a capsule's origin `player.radius_m`
away from any solid face. Measured, not argued: a body flown into the fence at 75.000 m/s at six
heights got no closer than **0.30 m inside** the boundary
(`tests/player.rs::f012_a_body_driven_into_the_fence_at_top_speed_is_never_recovered`), and
`tests/data.rs::f012_the_fence_stands_within_one_body_radius_of_the_map_edge` keeps the inequality
true when somebody edits `maps.ron`.

**`fence_top_m` stays 200**, and that is the answer and not an omission: a taller fence still has a
top face, and the gear reaches 62 km, so every number is a number somebody beats. The fence's job is
horizontal — it is what normal play runs into — and the footprint rule catches everything over it.
The measurement now lives in `maps.ron: bounds.gear_ceiling_m` and
`tests/player.rs::f012_the_gear_climbs_higher_than_the_fence_and_the_number_is_pinned` re-flies it,
so the next person who lowers gas or raises boost finds out in a test rather than in the sky.

### Where the fixtures hid it — the height was the axis every one of them held constant

| fixture | what it pinned |
|---|---|
| `f012_flying_at_the_edge_at_top_speed_does_not_tunnel_through_it` | "the launch height (40 m)" |
| `f012_walking_at_the_edge_of_the_map_does_not_walk_you_off_it` | `y = 2.0` for all four edges |
| `f012_a_body_dropped_past_the_edge...` | drop height held at 20 m |
| `scripts/f012-edge.txt` | three fence acts at y = 3, 122 and 190 — **all below 200** |

Nothing anywhere compared `fence_top_m` against a measured ceiling of the gear. The new fixtures
sweep **11 heights** from `fence_bottom_m + 5` to 902 m, and both new script acts stand above 200.

**Repro (red before, green after):**
`./target/debug/defeated_by_titan --headless --offscreen --script scripts/f012-edge.txt --ticks 5100`
— with the footprint clause of `out_of_the_world` replaced by `false`, act 6 reads
`assert Height < 122 — measured 200.000` (the ring) and act 7 reads `measured 556.326` (still out
and climbing), exit **1**. With it, **20 asserts held**, exit **0**.

**Evidence:** `scripts/f012-edge.txt` acts 6-7 · `tests/player.rs::f012_*` (13) ·
`tests/world.rs::f012_*` (5) · `tests/data.rs::f012_*` (2) · `--lib` 281 · `--test world` 48 ·
`--test data` 58 · `--test player` 70, all exit 0
**Related:** `B-015` · `F-012` · `docs/FINDINGS.md` FIND-203, FIND-204 · `docs/QUESTIONS.md` Q-081

---

## B-017 — the fence's inner lip stands **exactly on the map's edge**, `|x| > hx` is strict, so the line |x| = 350.000 is a solid standable floor the world calls *in the world* — 🟧 FIXED 2026-08-29

**The user, 2026-08-27:** *„unsichtbare wand + wenn man runterfaellt wegen bug teleport man
zurueck!"* — this is the third defect in the second half of that sentence, and it was one
character wide.

### The repro, on the shipped binary and shipped Ashgate

```bash
./target/debug/defeated_by_titan --headless --script scripts/f012-edge.txt --ticks 6000
# ACT 8 is the repro; it did not exist before 2026-08-29 and every fixture the feature had
# held its one axis — the distance from the boundary — away from zero.
```

| stance | measured before |
|---|---|
| `warp 350 201 0` | rests at **200.000** after 3 s, **200.000** after 10 s |
| all four sides and a corner | **200.000** |
| `warp 349.999 201 0` | 199.965 — a millimetre inside |
| 6 s of held `W` from (350, 201, 0) | still **200.000**. He can walk on it. |

`src/player/recovery.rs::out_of_the_world` tests `p_m.x.abs() > hx` — **strictly** — and
`assets/data/maps.ron` shipped `fence_margin_m: 0.0`, so the fence's inner face stood exactly on
`hx`. The inner lip of its top face, the widest floor in the district, was therefore a
coordinate the rule called *in the world*.

### And it poisons the home, for the rest of the session

`record_safe_ground` used the same predicate, so it **recorded the ring**. Park at (350, 201, 0)
for 4 s, then fall from anywhere, and the game logs

```
back to Vec3(350.0, 200.49994, 0.0), the ground he stood on at tick 426
```

and every later recovery from any cause delivers the player onto the ledge. That falsifies
`recovery.rs`'s own header — *"the place you get sent back to can never itself be a place you get
sent back from"*. Reproduced under `--hub` as well.

### And it loops, non-terminating

From the ring, any lateral input nudges `x` from 350.000 to 350.1, `PastTheEdge` fires, the warp
puts him at `safe + lift` = (350.0, 200.49994, 0) — **back on the lip** — he drops the half metre
onto it and drifts out again. Counted in one run: **1501 warp lines, one per tick, 25.0 s of wall
clock**, every one with the identical destination, plus 60 `warn!` lines per second.

### The fix is three things, because the geometry alone is not enough

1. **`fence_margin_m: 0.0 -> 0.18`** in both maps, bracketed by two measurements and not chosen:
   `fence_rest_reach_m 0.10 < fence_margin_m 0.18 < player.radius_m 0.35`. ⚠️ **"Stand the fence
   outside the footprint and the strict `>` covers all of it" is false** — a capsule rests on its
   bottom sphere, which reaches `radius_m` over the lip, so a body a millimetre inside the map
   still rested on the face at 349.938. What has to be cleared is how far back over the lip a
   body can **park**, measured at **0.0000 m** over 44 stances (past the lip friction cannot hold
   the tilted normal and every body slides off into the map), budgeted at 0.10.
2. **`record_safe_ground` judges the destination and the whole body.** It asks
   `out_of_the_world(recovery_destination(map, p) + body_reach)` — the point the recovery will
   actually warp to, at the far corner of the capsule's own footprint. Without the body half a
   body grounded on the lip was recorded at `(349.920, 199.950, 0)` — 200 m up with nothing
   under it — in **4 of 20** stances at the smallest margin `tests/data.rs` allows.
   ⚠️ A third condition, *"and he was not falling past it"* (`|vy| > gravity_m_s2 /
   simulation_hz`), was written and then **removed**: the control run could not make it go red
   in any legal configuration, and a fix without a failing test is a guess.
3. **`recovery::Recoveries` bounds the loop at two warps.** Recorded ground, then the spawn
   point, then one `error!` and silence until he is in the world again. What moves him down the
   ladder is *whether the last recovery held* — out of the world again, within a second, still
   within one body height of where the warp put him — and never a clock, because a fall 400 m
   away eight ticks later is a different fall.

**Evidence:** `tests/world.rs::f012_the_whole_top_face_of_the_fence_lies_outside_the_map_and_is_recovered_from`
(**64 of 648** samples red, exactly the class the old fixture `continue`d past) ·
`tests/player.rs::f012_the_top_of_the_fence_is_not_a_ring_you_can_stand_on_outside_the_map`
(24 -> 64 samples, now including 0, ±1 ULP and ±1 mm) ·
`f012_the_ground_he_is_sent_back_to_is_never_ground_he_can_be_sent_back_from` (**48 of 48** red) ·
`f012_a_recovery_whose_destination_does_not_hold_is_not_repeated_every_tick` (300 warps -> 2) ·
`f012_nothing_that_can_rest_on_the_fence_rests_inside_the_map` (the parking measurement) ·
`tests/data.rs::f012_the_fence_stands_within_one_body_radius_of_the_map_edge` (the bracket).

---

### `B-018` — a rope going taut dumps 23.7 m/s in a quarter second, with nothing to hit

Found 2026-08-29 while re-deriving `scripts/f005-feel.txt` ACT 4 after `gravity_m_s2` -20 -> -32.
The act swings on a rope with the camera 90° off it and asserts `speed > 8.0`; it reads **1.691**.
The gravity change is only what made it visible — the loss itself is not a gravity number.

**Repro** (pinned debug binary, no window):

```
wait 1.5
warp 76 20 -70          # 20, NOT the script's 12 — this is the point: it removes the ground
look 0 36
wait 0.1
hook left 4.0
wait 0.15
look 90 0
key W 3.5
# then sample `speed` and `height` every 0.25 s
```

**Measured**, and `rope` reads `1.000` on every single sample — nothing lets go:

| t (s) | 0.25 | 0.50 | 0.75 | **1.00** | **1.25** | 1.50 | 1.75 | 2.00 | 2.50 | 3.00 |
|---|---|---|---|---|---|---|---|---|---|---|
| speed | 15.6 | 22.4 | 28.0 | **31.745** | **8.092** | 8.535 | 8.344 | 7.530 | 4.333 | 0.455 |
| height | 16.6 | 13.5 | 9.9 | **7.262** | **6.622** | 7.203 | 7.939 | 8.764 | 10.272 | 11.021 |

**31.745 -> 8.092 m/s in 0.25 s, while the player is 6.6 m above anything.** 74 % of the speed and
93 % of the kinetic energy leave on the tick the rope reaches full extension. A taut rope should
REDIRECT velocity — remove the component along the rope and keep the tangential part — and the
tangential part of a swing at that phase is most of it.

**The control that rules out a collision and rules out gravity:**
- at the script's staged `y = 12` the arc does reach the ground (`height` 0.050 at t=1.00) and
  loses 20 m/s there — that is a real ground contact and it is a *second*, separate effect;
- at `y = 16, 20, 24` the arc never touches anything (lowest `height` 4.010 / 6.622 / 11.809) and
  the 3.0 s speed is **2.212 / 2.212 / 2.629** — the same collapse, three times, with no contact.
  Raising the stand removes the collision and does **not** give the swing back.

**Why no re-timing hides it:** what survives the dump is a genuine pendulum, `T ≈ 3.2 s`, so
`wait 3.0` samples it near the top of its arc. Re-timing to the same phase
(`3.0 · sqrt(20/32) = 2.372 s`) reads ~5.5 m/s and still fails, because the amplitude after the
taut event is smaller and not merely shifted: **the post-taut peak anywhere in the swing is
8.535 m/s**, against 13.334 m/s measured before the constant moved.

**Not yet done:** no red unit test. The script assert stays red on purpose (its header carries
this table). The test this wants is in `tests/vector_rope.rs` — drive a rope to full extension
with a known tangential velocity and assert the tangential component survives — and it has to be
written the `n = 2` way, because this game has two hooks and one anchor cannot see a two-rope
dump. Until that test exists this entry is a measurement, not a diagnosis: the dump is confirmed,
the line of code that does it is not.

### `B-019` — a LARGE titan's nape is out of reach from the cushion every other kind is killed from: **one tick of kill window out of seven, and it closes if you give back 0.20 m**

Found 2026-08-29 by the corpus round that re-derived `scripts/f030-hitbox.txt` from
`game.ron: gravity_m_s2`. It is **not** the gravity change and **not** the map edge: it is the
only one of that file's eight asserts that a correct pass still cannot satisfy.

### The repro, on the shipped binary and shipped Ashgate

```bash
./target/debug/defeated_by_titan --headless --script scripts/f030-hitbox.txt --ticks 1600
# line 217: assert Titans == 0 — measured 1.000     <- the lurker act. THIS one.
# line 245 / line 252 are the same survivor counted again in the warden's two asserts.
```

The lurker act flies the same pass as the six kinds that die: 0.60 m of air between the two
capsules, `look 0 0`, a 21 m/s fall onto the nape, the titan spawned 0.21 s before the cut. It
reports `tick 762: cut titan 6 Torso at 20.80 m/s` and then `ArmLeft` — **never `Cortex`**.

### Two controls, and they disagree with each other about nothing

Both are one-variable sweeps of that act alone, everything else byte-identical.

| slash tick, at the shipped 0.60 m cushion | −3 | −2 | −1 | 0 | **+1** | +2 | +3 |
|---|---|---|---|---|---|---|---|
| zone the blade books | Torso | Torso | Torso | Torso | **Cortex** | Torso | ArmLeft |

| cushion, at the shipped tick | 0.60 | 0.40 | 0.20 | 0.00 |
|---|---|---|---|---|
| zone the blade books | Torso | **Cortex** | **Cortex** | **Cortex** |

So the nape is reachable on **one tick in seven** at 0.60 m, and on the failing tick as soon as
0.20 m of that air is given back. It is a reach defect, not an aim defect, and not a facing
defect — the facing is identical in all eleven runs above.

### The arithmetic, and why `FIND-147`'s band formula did not see it

For a large kind the blade tip reaches

    titan_x − (body_radius 1.75 + player_radius 0.35 + cushion 0.60) + reach_m 2.00 = titan_x − 0.70

and `titan.ron: lurker.cortex_radius_m` is **0.77**. The tip is therefore 0.07 m inside a 0.77 m
sphere, and the chord it can reach is `sqrt(0.77² − 0.70²)` = **0.32 m** tall — which at 21 m/s
(0.35 m per tick) is **one tick**, exactly what the sweep measures.

`FIND-147`'s band, `reach + thickness + cortex_radius − body_radius − player_radius`, evaluates
to **0.87 m** for a large kind and so calls 0.60 m comfortable. **The band is a LATERAL bound and
it is right; what it cannot say is that the vertical window collapses to zero at its own edge.**
A sphere is thin near its rim. Same for a medium kind: the tip reaches `titan_x − 0.20` into a
0.55 m sphere, chord `sqrt(0.55² − 0.20²)` = 0.51 m ≈ 3 ticks — which is why every medium kind in
that file dies and only the large one that never turns does not.

### ⚠️ The warden is not a counter-example — his act measures his rotation

He is large, has the same `cortex_radius_m` 0.77 and the same cushion, and he dies. His act
opens his guard with a first pass that turns him about 50°, and 0.77 m of cortex set-back swung
through 50° carries the sphere `0.77 × sin 50°` = **0.59 m** toward the blade. That is more than
the 0.20 m the second control shows is enough. **`f030-hitbox`'s warden result is therefore not
a hitbox measurement**, and the lurker — the one kind whose `ambush` behaviour provably never
takes a step — is the only clean large sample in the file.

### Not fixed, and the fix is not mine to pick

Three candidates, all in RON, all the user's or the main head's call:
`reach_m` (2.00, and `f030-cortex.txt`'s stand-off sweep says the last raise from 1.60 widened
that file's pass from 0.20 m to 0.80 m), `cortex_radius_m` per large kind (0.77, already raised
once on 2026-08-20 for exactly this class of failure), or `width_fraction` (0.25, which is what
makes `body_radius` grow three times faster than the cortex does — the term `FIND-147` names as
the real driver). **The script assert stays RED on purpose** and its header carries this table.

**What is missing:** a red unit test in `tests/combat.rs` that casts a blade at a large kind's
cortex from `body_radius + player_radius + 0.60` and asserts `Cortex`. It does not exist, and
until it does this entry is a measurement with two controls rather than a diagnosis of a line of
code. ⚠️ And when it is written it must sweep the **height** — the sweep that found this varied
the tick, and the tick is the height; a fixture that takes its height from one call would report
`Cortex` or `Torso` depending only on which single tick it happened to pick.

---

## B-018 — at `gravity_m_s2: -32` a 0.30 m step is a **wall**: the district's terraces cannot be walked up at all

**Found 2026-08-29** by re-running the corpus after the -20 -> -32 tuning change.
`scripts/w2-terrain-walk.txt` line 40 (`assert height > 5.2`) reads **3.000**. It is the one
script in the corpus whose whole job is this question, and its header already says why it
matters: *"Terraces you cannot walk up are a wall, not terrain."*

### The measurement, and the control that moves it

Same binary, same script, same route (`warp 168 4.2 292`, `look 0 0`, `key W`, no gear), the
**only** difference being `gravity_m_s2` in `assets/data/game.ron`:

| walking -Z along x = 168, height in m | t = 0 s | 10 s | 20 s | 40 s | 60 s |
|---|---|---|---|---|---|
| `gravity_m_s2: -20` | 3.000 | **4.500** | **6.000** | 6.000 | 6.000 |
| `gravity_m_s2: -32` (shipped) | 3.000 | 3.074 | 3.000 | 3.087 | 3.013 |

At -20 he climbs both terraces in 20 s, exactly as the script documents (3.0 -> 4.5 -> 6.0 m).
At -32 he never gets up **one** 0.30 m riser, and sixty seconds of holding `W` does not change
it — the 0.05-0.09 m wobble is him riding the step and sliding back off it.

### Repro

```bash
cargo run -- --headless --script scripts/w2-terrain-walk.txt --ticks 2000   # exit 1, line 40 reads 3.000
```

Control: copy `assets/` somewhere, set `gravity_m_s2: -20.0` in the copy, run the binary from
that directory (`src/data/mod.rs::assets_dir` takes `assets/` from the **cwd** first) — exit 0.

### The cause is named in the script's own header, and it is a ratio that stopped holding

*"a 0.30 m riser is climbable only because the player capsule has a 0.35 m radius and rides over
a box step of less than that (`src/player/mod.rs`)"*. Riding over a step is a contest between the
horizontal push and the downward force pinning the capsule against the riser; `gravity_m_s2` went
up 60 % and `run_speed_m_s` did not move. **Nothing in the code encodes the relationship**, which
is why a tuning change to one number silently deleted a traversal rule.

### Not fixed, and the fix is not mine to pick

Three candidates, all in RON: raise `player.run_speed_m_s` (6.0 — but it is a feel number the
user owns), give the player an explicit **step-up height** that is derived from the capsule
radius rather than emergent from the collision resolver, or lower the terrace riser below what
-32 can still ride. The middle one is the only one that cannot break again the next time gravity
moves. ⚠️ **`gravity_m_s2` is PROVISIONAL** (the user changes it at his play test), so a fix that
re-tunes a second number to match -32 buys one week.

**`scripts/w2-terrain-walk.txt` stays RED on purpose** and its header carries this table.

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

## B-020 — at `gravity_m_s2: -32` three husks kill the player; at `-20` he lives

**2026-08-29 · [offlinebot] · found by the adversary of the corpus re-aim round, controlled**

`scripts/f032-swords.txt` ends with three husks alive and standing next to the player. Its
`assert health > 0` had held since 2026-08-19. After the user's gravity change it reads **0**.

**The control that makes this a regression and not a calibration:** the same pinned binary, the
same unmodified script, with only `gravity_m_s2`/`boost_m_s2` reverted to `-20`/`34` through a
working-directory `assets/` copy → **11 asserts held, exit 0, including `health > 0`.** At `-32`
the player is dead. Probed act by act: **100 · 100 · 66 · 32 · 0**.

🔴 **It was very nearly written into the assert instead of filed.** The re-aim round changed the
line to `assert health == 0` under the derivation *"3 surviving husks × `husk.damage` 34 = 102 ≥
`player.health` 100"*. **That arithmetic contains no gravity, so it predicts 0 at both tunings** —
it explains the measurement either way and therefore explains nothing. `CLAUDE.md`'s rule is that
a bracket comes out of the physics, never out of what the run printed; a derivation that cannot
fail is the same defect wearing a formula. The line is back at `> 0` and red on purpose.

### Why a heavier world makes a husk deadlier — unproven, and worth one measurement

Candidates, none checked: the player falls into reach faster and eats an extra swing; the dodge and
the flip cover less ground against `-32` so an evade that used to clear a swing no longer does
(`flip_up_m_s` 6.0 now buys **0.5625 m** of rise where it bought 1.8 m — already flagged
`⚠️ UNTUNED` in `game.ron`); or the recovery from a knockdown is slower relative to the swing
cadence.

**What it is NOT to be fixed with:** raising `player.health`, lowering `husk.damage`, or removing a
husk from the script. All three are the assert change one file over.

⚠️ **This is one of the three numbers waiting on his play test** (`Q-063` gravity, `Q-064` Shift,
`Q-046` gas). If he settles gravity lower, this may close by itself — so **measure before
designing a fix.**

**Related:** `Q-063` · `Q-064` · `B-018` · `docs/NEXT.md` §3G · `F-032` `F-044` `F-009`

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

## B-022 — **no script in this repo can walk a body along X**: `key W` obeys the yaw on Z and ignores it on X, at a steady `speed 6.000`

**Found 2026-08-29** while re-deriving `scripts/w2-terrain-walk.txt` for `FIND-210`. Foreign
territory (`src/debug/script.rs` / `src/input`), so it is filed and not fixed here.

### The measurement, and it is one warp and four yaws

Ashgate, shipped `gravity_m_s2: -32`, standing in the middle of terrain cell (12,12) — level 4,
`x 168..210`, `z 168..210`, top **12.0 m**. Its neighbours are level 4 to the west and north,
level 3 (**9.0 m**) to the east and south, so each direction has a different, known answer.

```text
warp 189.0 13.0 189.0
look <yaw> 0
wait 1.5
key W 6.0          # 36 m at run_speed_m_s 6.0 — well past the 21 m to any edge
wait 6.5
assert height > 900
```

| leg | should read | reads |
|---|---|---|
| `key W` · `look 0 0` (−Z) | 12.0 | **12.000** ✓ |
| `key W` · `look 180 0` (+Z) | 9.0 | **9.000** ✓ |
| `key W` · `look 90 0` (+X) | 9.0 | **12.000** ✗ |
| `key W` · `look -90 0` (−X) | 12.0 | 12.000 (cannot tell) |
| `key D` · `look 0 0` (strafe +X) | 9.0 | **12.000** ✗ |
| `key A` · `look 0 0` (strafe −X) | 12.0 | 12.000 (cannot tell) |

**The Z axis obeys the yaw. The X axis is not reachable at all** — not by turning, not by
strafing.

### Why it is worse than it looks: `assert speed` reads 6.000 the whole time

A leg that goes nowhere reports **`speed 6.000`**, which is `player.run_speed_m_s` exactly. So a
route that walks east looks like a route that walks east in every metric a script can ask for,
and only a *height* that never changes gives it away. This cost the `FIND-210` round several
runs and produced a confident, wrong conclusion — "the new stairs are a walkability regression"
— from a body that was never moving. **`assert speed` cannot tell walking from standing**, the
same shape as the rope test that could not tell a swing from a fall (`CLAUDE.md` rule 5).

### What it invalidates

Every evidence script whose route walks in **x**. `scripts/w2-terrain-walk.txt` was rewritten
onto a −Z route because of this and says so; nobody has audited the rest.

### Repro

```bash
cargo run -- --headless --script scripts/<the file above> --ticks 900
```

**Not fixed, and not mine:** the terrain round owns `src/world/**`, not `src/debug` or
`src/input`. What a fix needs first is the answer to *which* of the two is wrong — the yaw
convention every script comment states (`# yaw +90 = +X`, `scripts/f003-ashgate.txt`) or the
movement basis the input builds from it.
