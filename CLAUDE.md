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
   🔴 **And the fixture is the half nobody attacks.** A green test proves the fixture and the code
   agree; it says nothing about whether either is right. Six ways that has cost this project a
   round, each with its measurement, are in
   [`docs/lessons/fixtures.md`](docs/lessons/fixtures.md) — **read it before you trust a sweep.**
   The six-line version:
   - **delete the thing you are measuring** and check the number moves;
   - name every variable the code reads, name every variable the fixture varies — **the difference
     is the bug**, and both lists belong in the test's own comment;
   - **count what you skip**: a `continue` is invisible in the denominator, and `0 of N` about the
     wrong set is not a zero;
   - **write the `n = 2` case first** and make the elements disagree — this game has two hooks, so
     an aggregate over one arm is a test of a different function;
   - **sample the boundary itself**, at 0 and at ±1 ULP;
   - **ask what your instrument does with its own worst case** — the last one printed `none`
     exactly where the error was largest.
   ⚠️ **And do not re-derive another domain's decision — read it.** Two implementations of one
   question drift, and no amount of sweeping finds it, because both are yours. One writer decides,
   everyone else reads the answer.

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


### The five that carry almost all of it — each one measured, each one a rule

🔴 **1 · NEVER READ RAW `cargo` OUTPUT.** One `cargo test` here writes **629 952 bytes / 3 482
lines**, and the signal in it is **21 lines**. Reading that raw costs ~**150 000 tokens per test
run**. Filter every cargo and every game run, no exceptions:

```bash
cargo test --test world 2>&1 | grep -E '^test result|^error|panicked|FAILED' | head -30
cargo check                2>&1 | grep -E '^error' | head -20
./target/debug/defeated_by_titan --headless --script scripts/x.txt --ticks 900 2>&1 \
  | grep -E 'MARK|assert|script run finished|ERROR' | tail -25
# the exit code needs its OWN run — a pipeline's $? is the LAST command's:
./target/debug/defeated_by_titan --headless --script scripts/x.txt --ticks 900 >/dev/null 2>&1; echo $?
```

🔴 **2 · NEVER OPEN A BIG DOC TO ADD A LINE.** `cat >>` to append; `grep -n` for the anchor and
`sed -n 'a,bp'` for the slice. **And a queue file past its cap gets archived into `docs/archive/`
with a one-line index per entry** — nothing is ever deleted, and finding an old entry stays a
`grep`.
⚠️ **`tools/norms.py` now FAILS on this instead of asking you to remember it**
(`QUEUE_CAPS_KB`), because the sentence alone did not work: it said 150 kB and quoted
`FINDINGS.md` at 108 while the file had reached **812 kB** — seven and a half times its own rule,
for seventeen days. The tightest cap is on **`CLAUDE.md` itself, 45 kB**, because it is the one
file every agent reads WHOLE: at 40 kB that was ~10 300 tokens per agent per round, and splitting
the measurements out into `docs/lessons/` bought back about 2 800 of them.

🔴 **3 · A MEASUREMENT ROUND PINS ITS OWN BINARY *AND* ITS DATA.**
`cp target/debug/defeated_by_titan "$SCRATCH/dbt-pinned"` before any A/B, and interleave
A/B/A/B on a shared machine. *"Same binary"* is a claim, and it is false by default while other
agents are live — twice on 2026-08-13 one was rebuilt underneath a round mid-run.

🔴 **4 · ATTACK CLAIMS, NOT CHORES.** A round whose deliverable is a value in a `.ron`, or a fix
with a captured red-then-green message, carries its own proof and does **not** get an adversary.
A round that asserts the game now BEHAVES some way does.

🔴 **5 · AUDIT THE ROUND'S SPEND BEFORE STARTING THE NEXT.** Name the largest line item, say
whether it was necessary, and **fix the cause as a rule** so it cannot recur. Measure, do not
estimate — every entry in the lessons file below was one `wc` away the whole time.

**The full record, with the numbers and the seven cheap fixes of 2026-08-10:**
→ [`docs/lessons/token-cost.md`](docs/lessons/token-cost.md)

🔴 **AND ONE THAT IS NOT ABOUT TOKENS AND IS ABSOLUTE: NEVER `git add -A` IN THIS REPOSITORY.**
**Not with a clean status. Not ever.** Twice a sweep commit has swallowed work an agent was still
writing — most recently `b29c7dc`, whose message is about gates and water and which contains
**978 lines** of the marker fix and the titan rig. The history now lies about what happened, and
the branch is pushed, so by the no-rewrite rule it stays wrong.
The 2026-08-12 version said *"stage explicit paths **while agents are live**"* and that qualifier
is what failed: the supervisor twice believed no agent was live and twice was wrong. **You cannot
reliably know, so the condition is gone.**

```bash
git add docs/QUESTIONS.md src/vector/gas.rs   # yes: every path named
git add -A                                    # NO
git status --short                            # and read it before every commit
```
⚠️ **And never chain it** — `git add -A && norms.py && git commit` reads as one safe operation
and is three, of which the first is the dangerous one.

**The one that is not a saving:** do not cut the red test, the counter-check on a claim, or the
honest paragraph. Those are what made the output worth anything, and each caught something real.

## Speed — measured 2026-08-29, because he said it was going too slowly

He asked twice: *„überlege dir wie man die dev prozess beschleunigen kann"* and *„sind zu viele
checks drin die unnötig viel zeit fressen?"* — and then gave the observation that found the real
answer: *„der pc ist nicht wirklich ausgelastet. dennoch ist vim am ruckeln."*

### 🔴 1 · The machine was SWAPPING, not busy — and `nice` was the wrong knob all along

```
%Cpu(s): 10,3 us · 80,8 id · 5,7 wa      the CPU is IDLE
swpd 10 274 052 KB   free 923 352 KB     10 GB in swap, under 1 GB free
si 39 628 KB/s                           and actively paging back IN
```

Every `nice -n 15` in this project was ordering **CPU time**, which was never scarce. What stutters
an editor here is its pages being evicted to make room for a linker. **Fixed at the source:**
`[profile.dev]` asked for **full debug info over every dependency built at `opt-level = 3`** — the
`dev` default, never a decision — and `target/` had grown to **91 GB**.

| | before | after |
|---|---|---|
| `debug` | full (default) | `line-tables-only` + `split-debuginfo = "unpacked"` |
| linker | `rust-lld` | **`mold`**, which was installed on this machine and unwired |
| **incremental `cargo build --tests`** | **127 s** | **29 s** |
| free memory | 900 MB | 9 GB |

⚠️ **`opt-level` was NOT touched** — it is the difference between 20 and 200 fps and the reason the
profile exists. Only the debug data went. Nothing in this repo runs a step debugger; the evidence
is scripts, asserts and screenshots, and `line-tables-only` keeps the file and line a panic prints.
⚠️ **Add `ionice -c 3` next to `nice -n 15`** in anything long-running. `nice` alone answers a
question nobody asked.

### 2 · The gate does not have to be the whole suite

`cargo test` is **181 s at best and 530 s under contention**, and every round pays it at least
once. **At a round gate, run `--lib` plus the integration binaries whose files the round touched.**
The full suite runs **once**, before the push that closes the round — which is also where `git
status --short` and the `pgrep` precondition already live. A green binary you did not touch is not
evidence, it is a re-run.

### 3 · Attack claims, not chores — this was already the rule and it was ignored

`CLAUDE.md` has said *"attack claims; do not attack chores"* since 2026-08-10. Measured this week:
**eight of ten rounds were refuted**, and the ones that found something real were attacking a
**claim about behaviour** — a two-anchor guarantee, a sweep's coverage, a marker that names a key.
The ones that cost the most and found the least were attacking a **number** or a **red test with a
red-then-green control**, which is a chore and carries its own proof.
**So: a round whose whole deliverable is a value in a `.ron`, or a fix with a captured red-then-green
message, does not get an adversary. A round that asserts the game now BEHAVES some way does.**

### 4 · The findings are 42 % of the output, and that is mine

Measured over this session: **7 917 lines of code and tests against 5 737 lines of docs.** A finding
is worth what its *measurement* and its *rule* are worth — the essay around them is paid for twice,
once to write and once every time an agent greps past it. **A `FIND-` entry is the number, the
control that moves it, and the one sentence somebody will act on.** If it needs a table, it earns
one; if it needs three paragraphs of reasoning, that reasoning belongs in the code comment next to
the thing it explains.

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

# Every evidence script AT ITS OWN documented invocation, in parallel. Reads each header for
# its own --ticks and flags, and reports CUTOFF separately from RED — a run that was cut off
# has not failed. The supervisor hand-rolled this twice and called 43 scripts red where 26
# were (2026-08-29).
tools/corpus.sh                      # one line per script, then a GREEN/RED/CUTOFF/CRASH tally
tools/corpus.sh red                  # only the ones that really failed an assert
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
