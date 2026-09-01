# Token cost — where it actually went, with the numbers

Updated: 2026-08-29 · Stage: 🟧 (every figure here was measured, most of them the expensive way)

> **Why this file exists.** These war stories lived inside `CLAUDE.md`, which every agent reads
> whole — 12.8 kB of evidence, about 3 200 tokens per agent per round, paid to re-read arguments
> nobody disputes. **The rules stayed in the constitution; the measurements came here.**
> Grep this when a number is challenged; do not carry it.

---

### 🔴 THE BIGGEST ONE, and it dwarfs the rest: **never read raw `cargo` output**

Measured 2026-08-12: **one `cargo test` in this repo writes 629 952 bytes / 3 482 lines. The signal
in it is 21 lines.** An agent that reads that raw pays roughly **150 000 tokens for a single test
run** — and a round has several. This one line item was most of the session's spend.

**Every `cargo` and every game run gets filtered. No exceptions.**

```bash
cargo test --test world 2>&1 | grep -E '^test result|^error|panicked|FAILED' | head -30
cargo check                2>&1 | grep -E '^error' | head -20
# a failure? ask for that ONE test's detail, not the suite's:
cargo test --test world <one_test> 2>&1 | grep -A6 'panicked' | head -20
# the game prints a full Bevy startup log on every run:
./target/debug/defeated_by_titan --headless --script scripts/x.txt --ticks 900 2>&1 \
  | grep -E 'MARK|assert|script run finished|ERROR' | tail -25
# and the exit code needs its OWN run — a pipeline's $? is the LAST command's:
./target/debug/defeated_by_titan --headless --script scripts/x.txt --ticks 900 >/dev/null 2>&1; echo $?
```

**Put this in every commission.** It is the difference between a 200 k round and a 20 k round, and
it costs nothing in quality — the 21 lines are the whole verdict.

### 🔴 THE SECOND ONE: **never open a big doc to add a line to it**

Measured 2026-08-12: `docs/FINDINGS.md` was **108 kB (~27 000 tokens)**, `docs/QUESTIONS.md`
**68 kB**.
🔴 **Re-measured 2026-08-29 and the rule had rotted by 7.5x: `FINDINGS.md` had reached 812 kB and
209 entries** — roughly 200 000 tokens, i.e. larger than most context windows, in the one file this
rule tells you to grep. **The rule survived; the number in it did not, and nobody re-measured it
for seventeen days.** `FIND-001`..`FIND-184` are now in `docs/archive/FINDINGS-001-184.md` with a
one-line index (812 kB → **100 kB** live).
**So the rule has a second half now: a queue file that passes ~150 kB gets archived, and whoever
notices it is over is the one who does it.** Live sizes as of 2026-08-29: `FINDINGS.md` 100 kB ·
`QUESTIONS.md` 216 kB · `NEXT.md` 112 kB · `PLAN.md` 104 kB · `BUGS.md` 100 kB — **three of those
are already over and are the next to go.** Appending an entry with an edit tool means reading the whole file first — so a
five-agent round paid ~135 000 tokens to add fifty lines.

```bash
cat >> docs/FINDINGS.md <<'EOF'      # append: costs nothing to read
...
EOF
grep -n '^## FIND-041' docs/FINDINGS.md && sed -n '820,860p' docs/FINDINGS.md   # read ONE entry
```

Both files carry an explicit append marker at the bottom for this. Same rule for any file over
~20 kB: **`grep -n` for the anchor, `sed -n 'a,bp'` for the slice.** Never `Read` a 100 kB file to
change three lines.

### A measurement round pins its own binary — twice on 2026-08-13 one was rebuilt mid-run

Two agents measuring before/after had `target/debug/defeated_by_titan` **rebuilt underneath them**
by another agent's landing commit: one saw a new `PlayerTuning` field appear between two runs and
the old binary stop loading, another saw the map grow **2048 → 2059 blocks** between its A and its B.
A serial before/after would have reported both as a regression.

**So, for any A/B measurement:**

```bash
cp target/debug/defeated_by_titan "$SCRATCH/dbt-pinned"   # both halves run against THIS
```

and where the thing being measured is noisy or the machine is shared, **interleave A/B/A/B** rather
than running all the As then all the Bs — that is what let W1 report `+0.09 %` honestly while a
render job was running beside it.

**"Same binary" is a claim, and it is false by default while other agents are live.**

### The standing rule: **audit the round's spend before starting the next one**

The user asked for this on 2026-08-12 and it is now part of closing a round, next to the gate:

1. **Where did this round's tokens go?** Name the largest line item, not a feeling.
2. **Was it necessary?** A bug hunt, a refutation round, a measurement sweep — expensive and worth
   it. A chore that cost six figures — not.
3. **Fix the cause, in this file, as a rule** — so it cannot recur. Both entries above came out of
   exactly this audit, and each was a ~300x and a ~30x saving that had been invisible for a day.
4. **Measure, do not estimate.** `wc -c` on what agents read, `wc -l` on what they run. Both of the
   findings above were one `wc` away the whole time.

### Where the tokens actually went on 2026-08-10, and the seven cheap fixes

Measured against that session, the waste was **not** in the agents doing the work. It was in the
supervisor. In descending order of cost:

1. **The double round-trip on every finding.** Agent measures → writes a long report → the
   supervisor rewrites it into `docs/FINDINGS.md`. That pays for the same prose twice.
   **Fix: the agent writes its own `FIND-` entry directly**, with an id the supervisor assigns in
   the commission so two agents cannot collide. The supervisor then verifies and integrates
   instead of transcribing. Roughly halves the cost of a finding.
2. **Commissions that paste what the agent could read.** A briefing does not need forty lines of
   measurements quoted into it. **Name the file and the id** — `docs/FINDINGS.md` FIND-035 — plus
   the two or three numbers the agent needs to start. It reads faster than I can paste.
3. **Narrating whole agent reports back to the user.** He needs the decision, the number and what
   is still unknown — not the transcript. **One screen, not five.**
4. **Re-running the whole gate.** Four full `cargo test` runs in one session at ~17 min of linking
   each. **One per round, at the round gate, and only after the agents have landed.**
5. **Reading whole files to change three lines.** `grep -n` and `sed -n 'a,bp'` first; `Read` the
   range, not the file.
6. **Counter-checking mechanical work.** The refutation rounds paid for themselves three times out
   of three — but each time on a **claim or a measurement**. A rebinding or a doc repair with a red
   test does not need one. **Attack claims; do not attack chores.**
7. **Screenshots taken out of habit.** A picture is required for 🟧 and for nothing else. Two runs
   per script (verdict + image) is the rule *when a stage needs the image*, not per run.

8. **Four big agents at once is the expensive mistake, not any one of them.** Each re-derives the
   same context and each writes a full report. **Prefer one stream at a time, finished and
   verified, then the next.** Parallel is still the default *for cheap, well-cut work* (one file
   each, one criterion each) — but a round with two 100 k-token jobs running beside each other buys
   nothing that running them in sequence would not, and it makes integration worse.
   **Rule of thumb: parallel when the cut is by file and the criterion is mechanical; serial when
   the agent has to think.**
9. **Cap the report.** "Under 40 lines, do not summarise the docs you wrote — I can read them."
   A report that restates its own deliverable is paid for twice.

🔴 **An id is claimed by WRITING the entry, not by planning to.** Measured 2026-08-29: three
agents in one round each read the same `NEXT FREE ID` / last `## B-` line, each wrote a finding
against it, and **two of the three writes were lost** — `B-018` and `B-019` ended up describing
one group's bugs while two other groups had already committed scripts pointing at those ids for
*their* bugs. The result was evidence scripts marked red on purpose citing an explanation of
something else, which is exactly the triage cost that round existed to remove.
**So: the supervisor assigns the id in the commission** — `CLAUDE.md` already says that for
`FIND-` and it holds for `B-` and `Q-` too — or the agent claims it by appending the heading
FIRST and filling it in after. **Never read a free id and hold it in your head while you do the
work**: in a parallel round somebody else has read the same number.

### And one that is not about tokens: **never `git add -A` while an agent is still working**

On 2026-08-12 a sweep commit swallowed a whole feature that was still being written — the model
registry landed inside a commit whose message describes only the `init.md` carry-over, so **no
commit in the history says that feature exists.** Nothing was lost, and the history now lies about
what happened, which is worse than a messy diff.
🔴 **AND IT HAPPENED AGAIN ON 2026-08-29, so the rule is now absolute: NEVER `git add -A` IN THIS
REPOSITORY. NOT EVER.** Commit `b29c7dc` carries the message *"docs: the gates carry a mechanic and
there is no water at all"* and contains **978 lines across seven files** — the arm-aim marker fix,
the titan rig's neck, `scripts/f026-turn.txt` and their tests — swallowed out of two agents that
were still writing. The history now says that work is about gates and water. **The branch is
pushed, so by the no-rewrite rule it stays wrong.**

The 2026-08-12 version of this rule said *"stage explicit paths **while agents are live**"*, and
that qualifier is what failed: the supervisor twice believed no agent was live and twice was wrong,
once because a workflow was between phases and once because `pgrep` counts builds and not thinking.
**You cannot reliably know, so the condition has to go.**

```bash
git add docs/QUESTIONS.md docs/NEXT.md        # yes: every path named
git add -A                                    # NO. not with a clean status, not ever
git status --short --                          # and read it before every commit
```
⚠️ **And never chain it**: `git add -A && norms.py && git commit` reads as one safe operation and
is three, of which the first is the dangerous one.

**Stage explicit paths while agents are live** (`git add docs/ src/vector/gas.rs`), or wait for the
round to close. `git status --short` before every commit, and if a path you did not touch is
modified, find out whose it is first.

**The one that is not a saving: do not cut the red test, the counter-check on a claim, or the
honest paragraph.** Those are the things that made the session's output worth anything, and every
one of them caught something real.
