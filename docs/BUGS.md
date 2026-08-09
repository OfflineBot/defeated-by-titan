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

*(none — there is no running game yet in which one could occur. The first entry is called
`B-001`.)*

## Closed bugs

*(none yet)*

Related: [`docs/FINDINGS.md`](FINDINGS.md) (foreign mistakes) · [`docs/STATUS.md`](STATUS.md) ·
[`docs/lessons/`](lessons/)
