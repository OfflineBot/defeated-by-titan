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
