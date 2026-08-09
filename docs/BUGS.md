# BUGS — every bug with repro, evidence, cause, fix and test

Updated: 2026-08-09

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
