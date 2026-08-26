# CLAUDE.md — how this project works

**Defeated by Titan** — a 3D low-poly action game about the fight against Titans, in **Bevy
(Rust)**. The core is the **Vector Gear**: two hooks, two gas tanks, two blades. A Titan dies
**only** from a fast cut into the **Cortex**.

> 🔒 **The engine is Bevy (Rust). NOT Roblox.** Confirmed by the user on 2026-08-09. The
> design bible is written for Roblox in six places — those places are **translated, not
> obeyed**. The translation table lives in [`docs/architecture.md`](docs/architecture.md) and
> it **grows**: whoever runs into a new Roblox instruction adds the line there.

**The bootstrap prompt is gone.** `prompts/init.md` was carried over in full on 2026-08-12 and
deleted — every rule in it now has a permanent home in `docs/` or in this file, audited section by
section (`docs/FINDINGS.md` FIND-048, the transfer record in [`docs/RELEASE.md`](docs/RELEASE.md)).
It is readable in the history: `git show <sha>:prompts/init.md`.
**`gameplay/features.xlsx` stays** — it is not scaffolding, it is the generator source for
`docs/features.ron` and the whole `docs/backlog/`. **`docs/gameplay/` is the design**: what the
game is and why.

---

## Start of a session — always, in full, in this order

```bash
hostname                                    # 'debian' = no window, and that is fine
cat user-messages.md                        # the player's notes — READ FIRST, then migrate + empty
git status --short && git log --oneline -5  # what did another session do?
cat docs/STATUS.md docs/TODO.md             # where do we actually stand?
export PATH="$HOME/.cargo/bin:$PATH"        # Rust lives in ~/.cargo (machine A)
cargo check 2>&1 | grep '^error'            # is the tree green BEFORE I touch it?
```

**End of a session:** bring `docs/STATUS.md` + `docs/TODO.md` up to date · screenshots into
`docs/images/` with a normed name · a new insight into `docs/lessons/` · an open question into
`docs/QUESTIONS.md` · commit with a normed message · **and one honest paragraph about what
went unseen.**

## The six rules that always hold

1. **The four stages are the only truth about progress.**
   ⬜ not implemented · 🟨 built, **untested and unseen** · 🟧 tested **and** seen in the
   running game (image + number + a test that goes red) · ✅ **only the user sets this**.
   **Going backwards is allowed and wanted. Never build on 🟨. Doubt moves the stage down,
   not up.** → [`docs/STATUS.md`](docs/STATUS.md)
2. **Numbers belong in RON, not in Rust.** A titan kind, a blade tier, a gas cost: file work.
   The code holds units and mechanics, nothing else. **No `serde(default)` for game values** —
   a missing value has to crash on load.
   **One language: English, everywhere** (user, 2026-08-09: *„es sootlle im bestfall nirgendwo
   deutsch sein! alles auf englisch!"*). File names, identifiers, comments, test names, RON
   keys, docs, log and HUD output, commit messages. German stays only in what is **not ours**:
   `prompts/`, `gameplay/`, everything quoted out of them, and the git history.
   → [`docs/conventions.md`](docs/conventions.md) §4
3. **One domain = one folder = one plugin = standalone.** Only `shared`, `data` and Bevy are
   free; every other edge needs a line with a reason in the allow list of
   [`docs/architecture.md`](docs/architecture.md), and `tests/domains.rs` falls over
   otherwise. Communication runs over components and messages. **One field, one writer.**
4. **Multiplayer decides the architecture from day 1** — the netcode does not get built today,
   but nothing gets built that makes it expensive later. No `.single()` on players, player
   state never as a `Resource`, input is an `Intent`, simulation in `FixedUpdate`, stable ids
   instead of `Entity`. → [`docs/multiplayer.md`](docs/multiplayer.md)
5. **A bug without evidence is a rumor; a fix without a red test is a guess.**
   First the test that falls over; then the fix; then take the fix back out and watch the test
   go red again. **No repro, no fix.** → [`docs/BUGS.md`](docs/BUGS.md)
   🔴 **And the assertion has to be able to tell the two apart.** Measured 2026-08-19: a chain
   test read `32 → 36 → 42 m/s` and looked like proof that the rope accelerates — until a control
   run with the third `hook` line **deleted** reproduced the same numbers to three decimals. Legs
   2 and 3 had anchored **nothing**; it was measuring gravity. **`assert speed` cannot tell a
   swing from a fall.** The fix was one more assert (`rope == 1` on every leg), and the habit is
   the cheap part: **before believing a measurement, delete the thing it is supposed to be
   measuring and check the number moves.** Same shape as `FIND-103` — *a test that asks the
   screen and the function the same question passes when both are wrong.*
   🔴 **And the third shape, measured 2026-08-26: a fixture that passes ONE element cannot see a
   TWO-element bug.** `rope_drive(to_anchors_m: &[Vec3], ...)` was given a per-arm guarantee —
   *"the outbound half is projected out, so `target · r̂ >= 0` on every tick and for every key"* —
   and implemented against `unit(Sum r̂_i)`, which bounds the **sum** of the distances and not
   **each** of them. With two anchors 120° apart the player flies `40.00 -> 69.33 m` **away** from
   an anchor: the exact symptom the round existed to fix. Of **21** `rope_drive(&[` call sites in
   `tests/player.rs`, **exactly one passed two anchors — and it set `move_x = 0.0` and asserted
   only magnitude.** `rope_taut_brake` was *never* called with more than one. The suite was
   271 + 65 green and every number in the report was honest; **what it measured was not the whole
   function.**
   **This game has two hooks. Two is not an edge case here, it is the premise** — and the same trap
   sits in every signature taking a slice, a `Vec` or an iterator. **When a function takes a
   collection, a test that passes one element is a test of a different function.** Write the
   `n = 2` case *first* and make the elements **disagree** — two anchors at 120°, two players
   pulling opposite ways, two writers on one field. An aggregate (`Sum`, `mean`, `max`) is exactly
   where a per-element promise goes to die, and it is invisible at `n = 1` because there the
   aggregate **is** the element.
6. **Nothing changes per frame, everything per second**, and nothing runs over all entities to
   answer a question about the ten meters in front of your nose.
   → [`docs/lessons/performance.md`](docs/lessons/performance.md)

## How the work is done: **a supervisor in the `/loop` who writes nothing himself**

This project is **not worked through serially by one head**, but with workflows and subagents
— **broadly parallel and scientifically**. That is binding, not recommended
(carried over into
[`docs/lessons/supervision.md`](docs/lessons/supervision.md)).

- **The supervisor runs permanently in the `/loop`** and triggers workflows and subagents. He
  **plans, delegates, checks and integrates — he writes nothing himself.** Whoever starts
  writing code has stopped being a supervisor.
- **One iteration:** current state → hypothesis + acceptance criterion **up front** → parallel
  delegation → collect the results → check them against the criteria → integrate and decide
  about the next round.
- **Stop** when the DoD is met, when the limit is reached, or when **the same hypothesis has
  failed twice** — then it is not the execution that is wrong but the assumption
  → `docs/QUESTIONS.md`.
- **Parallel is the default.** The question is never "can this be parallel?" but "why not?".
  The cut runs along **domain**, along **`F-ID`** (`depends_on` says what does *not* go at the
  same time), along **check dimension**, and for bulk work along **file**.
- **This has to stand before the fan-out:** the interface (components, messages, signatures),
  **file ownership** (one writer per file) and the **acceptance criterion**. Otherwise the
  fan-out produces integration work instead of progress.
- **Width comes from `nproc`, not from wishful thinking** — and the compiler is a consumer
  too. On machine A that means **2–3 at a time**.
- **After every fan-out:** `cargo check 2>&1 | grep '^error'` and `cargo test`. Five agents
  green on their own are not automatically green together.
- **Every round of findings gets an independent round that tries to refute it.** A claim
  nobody has attacked is 🟨.
- **Only the main head touches these files:** `Cargo.toml`, `src/main.rs`, `src/lib.rs`,
  `assets/data/*.ron`, `docs/STATUS.md`, `docs/TODO.md`. Subagents **report** only.
- **Every subagent commission names:** which files belong to it, which sections it should read
  (not "read everything"), the evidence obligation, and that foreign territory goes to
  `docs/FINDINGS.md` instead of being quietly fixed along the way. Its report has the fixed
  format: **`Task · Done · Evidence · Stage · Open · Findings`** — a free-text report cannot
  be integrated.

## Token cost is a constraint too (user, 2026-08-12)

> *„es soll etwas tokeneffizienter gearbeitet werden. es gilt zwar immernoch: Qualität wichtiger
> als tokenverbrauch/effizienz. aber bei jeder iteration wäre es vermutlich sinnvoll wenn man die
> conversation compacted! damit agents nicht mit zu viel context erstellt werden! aktuell ist der
> tokenverbauch zu hoch für zu wenige ergebnisse!"*

**Quality still outranks cost — that does not change.** What changes is that a round which produces
one finding for 400 000 tokens is a *bad round*, and the supervisor is the one spending it.

- **Compact between iterations.** A fresh agent inherits the commission, not the session. Long
  context makes agents slower, not better.
- **A commission is a briefing, not a transcript.** Name the files, the acceptance criterion and
  the two or three measured facts it needs. Do not paste findings it can read itself.
- **Fewer, tighter agents.** Three well-scoped beat six overlapping. Every agent that has to
  re-derive shared context is the supervisor's waste, not the agent's.
- **Measure before you delegate.** One `grep` by the main head often replaces a whole commission —
  and several rounds on 2026-08-10 were spent re-deriving something one file already said.
- **Do not order the full `cargo test` in a commission.** It belongs to the main head, once, at the
  round gate. Ordering it three times in parallel cost load average 205 and a 22-minute build.
- 🔴 **But a restricted test list MUST include `--lib` whenever the agent writes code under
  `src/`.** Measured 2026-08-18: a commission said *"only `--test world`, `--test data`,
  `--test render`"*, and the agent's own five unit tests lived inside the new
  `src/shared/terrain.rs` — which **only `--lib` runs**. So its tests were never executed once,
  it reported green in good faith, and
  `f003_the_same_seed_yields_exactly_the_same_ground` sat red until the gate found it days later.
  **The restriction excluded the one binary its work was in.** The agent did nothing wrong; the
  supervisor wrote a list that could not see the agent's own file.
  **So: name `--lib` in every commission that touches `src/`, and name the integration binaries
  that cover the file being changed — the cut is by WHAT THE AGENT WRITES, not by what the
  feature is called.** A cheap habit that would have caught it: `grep -c '#\[test\]' <the file>`
  before writing the commission.
- **Read `docs/backlog/` for the F-ID before designing anything** — `FIND-039`: a feature was
  re-derived over a day that the backlog had already specified, and the re-derived version was
  worse.

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

Measured the same day: `docs/FINDINGS.md` is **108 kB (~27 000 tokens)**, `docs/QUESTIONS.md`
**68 kB**. Appending an entry with an edit tool means reading the whole file first — so a
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

### And one that is not about tokens: **never `git add -A` while an agent is still working**

On 2026-08-12 a sweep commit swallowed a whole feature that was still being written — the model
registry landed inside a commit whose message describes only the `init.md` carry-over, so **no
commit in the history says that feature exists.** Nothing was lost, and the history now lies about
what happened, which is worse than a messy diff.
**Stage explicit paths while agents are live** (`git add docs/ src/vector/gas.rs`), or wait for the
round to close. `git status --short` before every commit, and if a path you did not touch is
modified, find out whose it is first.

**The one that is not a saving: do not cut the red test, the counter-check on a claim, or the
honest paragraph.** Those are the things that made the session's output worth anything, and every
one of them caught something real.

## Autonomous operation — when nobody is standing next to you

The normal case in this project is that the user is **not** there. Then this holds on top:

- **Blocking is forbidden.** An open question does not stop the work. Whoever waits burns the
  session and delivers nothing.
- **A decision that belongs to the user gets made anyway** — but visibly, into
  [`docs/QUESTIONS.md`](docs/QUESTIONS.md), and there stand **the `ASSUMPTION:` the work
  continued under, and the place that would have to be rolled back** if he decides otherwise.
  A question without an assumption and without a rollback point is useless: it costs him time
  and gives him nothing back.
- **A problem that really does stop the work** goes to `docs/BUGS.md` (with a repro) or
  `docs/FINDINGS.md` (foreign territory) — and the work continues **somewhere else**.
- **✅ stays the user's, 🟧 is the ceiling** — without exception. With nobody there to
  disagree, the temptation to set a stage one too high is bigger.
- **The independent counter-check stands in for the user.** What nobody has attacked is 🟨 —
  and whoever attacks it has to be an agent that did not build the result itself.
- **No progress is claimed that is not evidenced.** "Should work now" is worth less here than
  elsewhere, because nobody notices it right away.
- **At the end of every autonomous stretch stands one honest paragraph about what went
  unseen.**

### The division of labour (user, 2026-08-09)

> *„deine aufgabe ist es die agents so zu managen dass diese ein komplettes spiel fertig
> bekommen in einer sitzung. auch wenn diese sitzung sehr lange geht!"*

- **The supervisor does not stop while anything is left.** One round ends, the next one starts
  in the same breath. Reporting is not a resting point — a report that is not followed by the
  next commission has wasted the round.
- **The user answers in the files, not in the chat.** He plays, and what he notices he writes
  into `docs/QUESTIONS.md`, `docs/BUGS.md` or `docs/FINDINGS.md` — sometimes through another
  agent. **Therefore every round begins by looking whether those three files changed**
  (`git status`, `git log`, and read them). An answer that lies there unread is worse than no
  answer, because the assumption keeps running on top of it.
- **Ask for as little as possible.** Deciding under a documented `ASSUMPTION:` is the norm;
  asking is the exception, and it never stops the work.
- **The order is not free.** The design bible gates everything behind the Vector Gear: no
  skill tree, economy, lineages, raids or cosmetics before the movement convinces. The build
  order lives in `docs/TODO.md` and it is derived, not invented.

## Where things are

| Question | File |
|---|---|
| **What am I building, and why?** | [`docs/gameplay/`](docs/gameplay/README.md) — ⭐ the design: pillars, world, enemies, core loop |
| **What does the player say is wrong?** | [`user-messages.md`](user-messages.md) — his notes. **Read first, migrate, then empty** |
| **What do I do first, right now?** | [`docs/NEXT.md`](docs/NEXT.md) — the queue the last session left, in order |
| What is open, in which order? | [`docs/TODO.md`](docs/TODO.md) *(generated)* |
| How far may I trust a thing? | [`docs/STATUS.md`](docs/STATUS.md) *(generated)* |
| Which machine, what works here? | [`docs/environment.md`](docs/environment.md) |
| Domains, allow list, who writes what | [`docs/architecture.md`](docs/architecture.md) |
| Axes, units, terms, naming norms | [`docs/conventions.md`](docs/conventions.md) |
| Swapping models (written for the user) | [`docs/models.md`](docs/models.md) |
| What is not mine to decide | [`docs/QUESTIONS.md`](docs/QUESTIONS.md) |
| What deliberately comes later | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| How the parallel work is organized | [`docs/lessons/supervision.md`](docs/lessons/supervision.md) |
| All the docs at a glance | [`docs/README.md`](docs/README.md) |

## The tools

```bash
python3 tools/features.py            # gameplay/features.xlsx -> features.ron + TODO + STATUS
python3 tools/features.py --check    # only the row-count guard per sheet
python3 tools/norms.py               # terms, dead links, orphan files, test names
python3 tools/norms.py --commit-msg .git/COMMIT_EDITMSG
```

**Starting the game** — which window system gets linked is decided by the machine:

```bash
cargo build                          # machine A (debian): x11, builds everywhere
cargo play                           # machine B (offlinebot/niri): --features wayland
cargo run -- --headless --script scripts/<f-id>-<short>.txt   # one run without a window
```

## What does not apply yet

The bootstrap build-up plan is **used up** — its disposition is recorded in
[`docs/ROADMAP.md`](docs/ROADMAP.md), including the one step that was **skipped rather than
finished** (the model chain; it is being built now). From here the design's own phase plan governs
— and **its hard rule: no meta system before the Vector Gear gate is passed.** Trait tree, economy,
lineages, raids, cosmetics do **not get started** while the movement does not yet feel convincing.
The gate is a blind test against **Attack on Titan Revolution** with ten testers, in which our
movement has to score at least level ([`docs/gameplay/pillars.md`](docs/gameplay/pillars.md)).

## Push — always, and it changes what may be rewritten

**The user, 2026-08-19: *„pushe immer. immer aktuell halten."*** So a closed round ends
`gate → commit → push`, not `gate → commit`. `origin` is
`git@github.com:OfflineBot/defeated-by-titan.git`; the working branch is pushed as itself.

🔴 **And therefore: no history rewrite after a push.** On 2026-08-19, while nothing was pushed
yet, two commit messages had to be repaired and the repair `git reset --hard`'d six commits — a
chained command whose destructive half ran and whose restorative half did not, because
`cherry-pick` has no `-q` flag. The work survived only because the SHAs were still in the reflog.
**Now that the branch is public, that move is off the table**: a rewrite would break the remote,
and a rewrite *plus* a mistake would break it irrecoverably.
**So the message has to be right the FIRST time** — run
`python3 tools/norms.py --commit-msg .git/COMMIT_EDITMSG` **before** committing, not after

🔴 **And never pipe that check into anything.** Measured 2026-08-20, after it had already let a
bad subject through: `norms.py --commit-msg X | tail -1 && git commit` **always commits**, because
a pipeline's exit status is the LAST command's and `tail` always succeeds. The check printed its
violation and the guard never fired — the commit is public and, by the rule above, has to stay.
**Run it bare and let it gate:**

```bash
python3 tools/norms.py --commit-msg /tmp/cm.txt || echo "FIX THE MESSAGE, DO NOT COMMIT"
python3 tools/norms.py --commit-msg /tmp/cm.txt && git commit -q -F /tmp/cm.txt   # bare, no pipe
```

Same shape as the `$?`-after-a-pipeline trap already recorded for the game's exit code. **A guard
you cannot see failing is not a guard**
(the 72-character subject limit is the one that bit twice), and never chain a destructive git
command with anything.

## Commit messages

```
<F-ID|T-ID|docs|test|tool|fix|chore>: <one line, what is different now>   ← max 72, English

Stage: 🟨 → 🟧
Evidence: tests/vector.rs::f014_boost_consumes_gas · docs/images/f014-boost.png · 12.4 → 3.1 ms [debian]
```

⚠️ **No tool or author traces.** No `Co-Authored-By`, no signature, no "generated with", no
model name — in no commit message, no PR description, no tag. A commit message describes **the
change**, not its author. `tools/norms.py --commit-msg` checks that.

## Finally

**Measure first, then claim.** Almost every expensive mistake in a project like this one is a
place where something reasonable was *explained* instead of being *measured* in a minute.
Write what you know: "built, untested — 🟨" is a good sentence, "should work now" is not.

# EXTRA
## `user-messages.md` — the player's notes. Read it FIRST, every session.

**It is the most valuable file in this repository**, and that is measured, not flattery: on
2026-08-10 four sentences the user typed after a few minutes of play found gas that never refilled,
a rope that went slack on every fast approach, and an overshoot **no test here could have found** —
because no test had ever flown at an anchor without holding reel. A full day of instrumented
measurement found none of the three.

**It is his to write and mine to empty.** He plays, he writes what felt wrong, in German, in
whatever form he likes. Nothing in it is a specification; it is a symptom report, and my job is to
find the cause.

### The ritual for it — do this before any other work

1. **Read it.** If it is empty, say so and carry on with [`docs/NEXT.md`](docs/NEXT.md).
2. **Migrate every line out of it** into the place that will actually be read again:
   - a **decision that is his** → [`docs/QUESTIONS.md`](docs/QUESTIONS.md), and if it answers an
     open `Q-`, mark that one ✅ ANSWERED and write down **what has to be rolled back**;
   - a **thing to build** → [`docs/NEXT.md`](docs/NEXT.md), in build order, quoting him verbatim
     so the intent survives my paraphrase;
   - a **symptom with a cause** → [`docs/BUGS.md`](docs/BUGS.md) with a repro, or
     [`docs/FINDINGS.md`](docs/FINDINGS.md) with a measurement;
   - a note about **how I work** → into this file, as a rule, not as a memo.
3. **Then empty it** back to the template below and commit it with the migration, so he always
   opens a blank page and never has to wonder whether I read the last one.
4. **Quote him verbatim in the migrated entry.** His phrasing carries information my summary loses
   — *"das a d sorgt dafür dass man nicht immer direkt zum seil gezogen wird"* is a whole control
   scheme in one clause, and I would have flattened it to "add lateral control".

### The two rules his notes have already earned

- **A symptom he reports is real even when the code looks right.** Every single one so far has
  been. Find the cause; do not explain the symptom away.
- **His instruction beats my derivation, and beats his own earlier number.** That precedence rule
  is already in `scale.ron` and `Q-002`; his notes are where it gets exercised.

### The template `user-messages.md` gets reset to

```markdown
# Movement

# Combat

# Feel / Look

# Technisches zu Claude in dem Projekt
```

### The gate has a precondition, and the supervisor has now broken it three times

`cargo test` at the round gate is only meaningful against a **settled** tree. Started while an agent
is still writing, it measures a half-finished file and reports failures that mean nothing — on
2026-08-12 that produced "276 failed" once and `unresolved import shot_verdict` twice, none of them
real.

**Before starting the gate, check that nothing is live:**

```bash
pgrep -f 'cargo (test|check|build)' | wc -l     # must be 0
git status --short                              # and know whose every dirty path is
```

If either says otherwise, the round is not closed and the gate is premature. **A gate you have to
explain is worse than one you did not run** — and worse still, the noise trains you to skim the next
real failure.

Same precondition for `git add`: stage explicit paths while agents are live, never `-A`.
