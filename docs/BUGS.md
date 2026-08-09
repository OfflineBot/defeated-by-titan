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

*(none)*

## Closed bugs

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
