# Fixtures — six ways a test measured the wrong thing, each with its number

Updated: 2026-08-29 · Stage: 🟧 (every one of these was measured, and every one shipped green)

> **Why this file exists.** `CLAUDE.md` rule 5 grew to 6.2 kB of war stories inside a constitution
> that every agent reads whole — about 1 600 tokens per agent per round, paid to re-read evidence
> nobody was disputing. **The rules stayed there; the measurements came here.** Grep this file when
> you are about to trust a fixture; do not carry it.

**The one sentence that generates all six:** a green test proves the fixture and the code agree.
It says nothing about whether either is right, and **the fixture is the half nobody attacks**.

---

## 1 · A number that moves is not a number that means something (2026-08-19)

A chain test read `32 → 36 → 42 m/s` and looked like proof that the rope accelerates — until a
control run with the third `hook` line **deleted** reproduced the same numbers to three decimals.
Legs 2 and 3 had anchored **nothing**; it was measuring gravity. `assert speed` cannot tell a swing
from a fall. The fix was one more assert (`rope == 1` on every leg).

**The habit:** before believing a measurement, **delete the thing it is supposed to be measuring
and check the number moves.**

---

## 2 · A test that asks the screen and the function the same question passes when both are wrong

`FIND-103`. The oracle and the code shared a derivation, so they could not disagree.

---

## 3 · A fixture that passes ONE element cannot see a TWO-element bug (2026-08-26)

`rope_drive(to_anchors_m: &[Vec3], …)` was given a per-arm guarantee — *"the outbound half is
projected out, so `target · r̂ >= 0` on every tick and for every key"* — and implemented against
`unit(Σ r̂ᵢ)`, which bounds the **sum** of the distances and not **each** of them. With two anchors
120° apart the player flies **40.00 → 69.33 m away** from an anchor: the exact symptom the round
existed to fix.

Of **21** `rope_drive(&[` call sites in `tests/player.rs`, **exactly one passed two anchors — and
it set `move_x = 0.0` and asserted only magnitude.** `rope_taut_brake` was *never* called with more
than one. The suite was 271 + 65 green and every number in the report was honest.

**This game has two hooks. Two is not an edge case here, it is the premise** — and the same trap
sits in every signature taking a slice, a `Vec` or an iterator. An aggregate (`Σ`, `mean`, `max`)
is exactly where a per-element promise goes to die, and it is invisible at `n = 1` because there
the aggregate **is** the element.

---

## 4 · A sweep's size is not its coverage — ask what it HOLDS CONSTANT (2026-08-27)

`f177_no_stance_in_the_hub_names_a_door_and_opens_another` swept **2 361 960 stances** and reported
**0** lies while the shipped game lied deterministically at `(0, 2, 0)` yaw 140, photographed. It
varied `x`, `z` and `yaw`, and took its **height** from one `stand()` call — and height is the
single axis the rule depended on, because the miss-distance was three-dimensional. It is also the
axis the game moves the player along at the exact moment he enters the hub (`open_hub` warps to
`y = 2.0`, an 11.5-tick fall). **A million samples of the wrong slice measure the slice.**

### 4a · A `continue` is a silent exclusion, invisible in the denominator

The same round reported `0` for a defect its sweep skipped by construction:
`let Some(walked) = … else { continue; }` dropped every stance where nothing starts — **55.8 %** of
the stances whose line promised something.

### 4b · And the sharpest version: the sweep ASSERTED its own blind spot was non-empty (2026-08-29)

A fence test `continue`d past every sample lying exactly on the boundary, named them `on_the_line`,
justified the skip with the unmeasured sentence *"a body cannot rest on a line"* — and then
asserted `on_the_line > 0`, treating the exclusion as a feature. **Those 64 of 648 samples were the
defect**, and a body put there rests for ten seconds. **A skip you are proud of is still a skip.**

---

## 5 · The PROVENANCE of the input is an axis too (2026-08-27)

A sweep of **9 447 840** stances stated in its own comment *"what this fixture holds constant:
nothing that the rule reads"* — and it was wrong, because every stance was a `Vec3` **the test
invented** and handed to the pure function, while its oracle was a hand-copy of the shipping code
**fed the same invented `Vec3`**. Two functions asked about the same invented point cannot disagree
about which point it is, so the real defect — the HUD reading a *stale* position, one schedule ahead
of the code it agreed with — was unreachable by construction.

**When two pieces of code are supposed to agree, make them agree about something the GAME
produced.** If nothing in your test file ever enters the state under test *the way the game enters
it*, your sweep is a unit test of arithmetic wearing an integration test's numbers.

---

## 6 · The instrument can have the hole (2026-08-29)

`hud::arm_aim::trace_arm_aim` printed the literal string `none` whenever `Camera::world_to_viewport`
refused the point — **which is precisely when the marker is clamped to a screen edge and the error
is at its maximum**. 192 of 519 anchored samples carried no value, and for 161 consecutive samples
it reported nothing while the drawn glyph sat **705 px** from the crosshair. A table built from that
trace reads `0.00` in a regime where the true value is five figures.

**Rule 4a, built into the measuring device itself.** Check what your instrument drops before you
trust what it prints.

---

## The checklist, if you read nothing else

1. **Delete the thing you are measuring** and check the number moves.
2. **Name every variable the code reads. Name every variable the fixture varies. The difference is
   the bug** — write both lists into the test's own comment.
3. **Count what you skip.** `0 of N` is arithmetic about the wrong set unless you also report the
   samples you never reached.
4. **Write the `n = 2` case first** and make the elements disagree.
5. **Sample the boundary itself** — at 0, at ±1 ULP, at a millimetre either side.
6. **Ask what your instrument does with its own worst case.**
