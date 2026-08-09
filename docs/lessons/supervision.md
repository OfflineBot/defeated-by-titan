# supervision — a supervisor who writes nothing himself, delegates broadly in parallel, and ties every claim to a criterion before the work starts

Updated: 2026-08-09 · Stage: 🟨 (written down from `prompts/init.md` §17 — the method is
described, but in this repo it has not once been measured over a full iteration)

## The problem

One head works through the commission file from top to bottom. That feels tidy and it is the
most expensive mode this project has: the remaining cores are bored, every claim gets explained
instead of measured, and the same wrong assumption is implemented three times over because
nobody ever wrote it down.

The source lays down two rules for this, both **binding**, not recommended: work happens
**broadly in parallel** and by **scientific method**. `prompts/init.md` is the **commission**,
not the work order of a single head.

## a) The supervisor writes nothing

**A supervisor runs permanently in the `/loop`** and triggers workflows and subagents. He
**plans, delegates, checks and integrates** — nothing more. The moment he starts writing a file
himself, he has stopped being a supervisor and is the sixth agent, waiting on himself.

**The iteration — six steps, in this order:**

| # | Step | how you can tell it was skipped |
|---|---|---|
| 1 | current state | nobody can say what held before the change |
| 2 | hypothesis **+ acceptance criteria** | the criterion appears after the result and always fits |
| 3 | parallel delegation | one agent works, the remaining cores stand still |
| 4 | collect results | partial results vanish into transcripts |
| 5 | check **against the criteria** | "looks good" instead of a test name and a measured value |
| 6 | integrate and decide on the next round | five green branches, no green tree |

**Stopping conditions** — the loop ends when the DoD is met, when the limit is reached, or when
**the same hypothesis has failed twice**.

The third case is the important one and the one most easily walked past. The same hypothesis
failing twice does **not** mean the execution was sloppy — it means the **assumption is wrong**.
The third attempt is lost time; the question belongs in `docs/QUESTIONS.md`.

## b) Domain experts who name deviations instead of working around them

**A senior expert is set up for every specialist area the project implies.** The source names as
examples: Vector Gear physics, titan behavior, rendering / 3D pipeline, data / RON, tooling &
test, docs & status.

Every expert

- **decides on his own authority inside his domain**,
- keeps **all** project rules,
- and **names deviations explicitly instead of working around them**.

That last one is the sentence to remember from this whole section: **a named deviation is a
decision, a silent one is a bug with a running start.**

**Binding for everyone — supervisor and experts alike:**

> falsifiable hypothesis → fix the check criterion **in advance** → measure reproducibly →
> evaluate.

Every statement with evidence, assumptions marked as `ASSUMPTION:`, uncertainty declared,
nothing invented; when something is unclear, measure or escalate, do not guess. From the
delegating side that reads: **a result without a check criterion fixed in advance is not a
result, it is an opinion.**

## c) Parallel is the default — the question is "why not?"

The question is never "can this be parallel?". Serial is right only where a file needs **a
single writer** (the table in section f) or where step N really needs the **result** of N−1.
Everything else runs at the same time.

**The four ways to cut, in this order of preference:**

| # | Cut | Example | why it holds |
|---|---|---|---|
| 1 | by **domain** | `vector/`, `titan/`, `world/`, `hud/` at the same time | file ownership and domain are the same thing (§5) |
| 2 | by **`F-ID`** | independent features from §2 | `depends_on` in `features.ron` says what **cannot** run at the same time |
| 3 | by **check dimension** | correctness · edges · performance · "what happens over the network" (§6) | four viewpoints find four things; four identical checkers find the same thing three times |
| 4 | by **file** for bulk work | one agent per `tools/blend/*.py`, per doc file, per spreadsheet sheet | no seam, no merge |

Cut 1 presumes the skeleton stands — **serial first, then wide**: stage 0–1 (skeleton,
`Cargo.toml`, `main.rs`, domain folders with empty plugins, the `docs/` skeleton) is done by
**one** head alone. A fan-out onto an empty folder produces five incompatible drafts of the same
file.

## d) Width comes from `nproc`, not from wishful thinking

```bash
nproc
```

The ceiling is the machine (§14) — and **the compiler is a consumer too**.

| Machine | cores/threads | agents at once |
|---|---|---|
| A | 4 cores | **2–3** |
| B | 16 threads | 8 and more |

**Twenty agents on four cores are slower than three — they only wait together.**

**Pipeline, not barrier.** A feature that has been checked through does not wait for five others
to finish. A join point is right only where a step needs **all** the preceding results together:
deduplicating, a total count, "are there any findings at all?".

**What every parallelization needs BEFOREHAND** — otherwise it produces integration work instead
of progress:

1. **The interface stands.** Components, messages and signatures are written down **and
   committed**. Parallelize before the seam stands and you spend the time afterwards integrating
   five drafts of the same file.
2. **File ownership is handed out.** Every file has exactly **one** agent allowed to write it.
3. **The acceptance criterion is noted** — beforehand, not afterwards.

And after that, after **every** join:

```bash
cargo check 2>&1 | grep '^error'
cargo test
```

**Five agents green on their own are not automatically green together** — each of them has seen
only his own half.

## e) Scientific means measurable, reproducible, refutable

Not an attitude, a way of working — for every claim, from "the hook does not hold" to "this is
faster":

| # | Rule | what it costs when it is missing |
|---|---|---|
| 1 | **Measure the current state first, then change it** — the baseline **before** the change is the most valuable number there is | you "fix" things that were never broken, and you never learn whether your change did anything |
| 2 | **Write the hypothesis down before you measure** — falsifiable ("if X, then Y drops below Z"), together with its check criterion | an explanation that appears after the measurement always fits and is worthless for exactly that reason |
| 3 | **One variable per experiment** | in the end you only know that *something* helped |
| 4 | **Reproducible means the command stands next to it** — full command line, seed, coordinate, look direction, **machine** (`[cachy]`/`[debian]`, §14) | what nobody can re-stage is not a measurement, it is an anecdote |
| 5 | **Measure times more than once**: N runs, **median and percentile**, not the mean; never compare across machines | one run is noise |
| 6 | **Try to refute it before you believe it** — every claim gets an independent attempt to knock it over ("find the case where this is wrong") | what has survived no attack is 🟨 (§8) |
| 7 | **A negative result is a result** and gets written down (`docs/lessons/`, `docs/BUGS.md`) | in three weeks somebody tries exactly the same thing again |
| 8 | **Mark assumptions as `ASSUMPTION:`, declare uncertainty, invent nothing**; when it is unclear, measure or escalate (§9) | a guess gets read as knowledge |

Point 6 is not a politeness loop: **checking belongs in the workflow, not in hope.** After every
discovery step and every build step comes an **independent** step with the order "find the case
where this breaks" — a different agent, not the same one.

## f) These files are touched by the MAIN HEAD only

They are written as a **whole** file; two agents do not merge them (one writer per file, §5).

| File | why |
|---|---|
| `Cargo.toml` | two agents, two dependency lists, one survives |
| `src/main.rs` | **the plugin list** is the seam of the whole project |
| `src/lib.rs` | the same for the module list |
| `assets/data/*.ron` | RON is written as a whole file — lost lines are seen by nobody |
| `docs/STATUS.md`, `docs/TODO.md` | **the main head enters them**, subagents only *report* |

**What does parallelize well:** one agent per **domain** (`vector/`, `titan/`, `world/`,
`hud/` …) as soon as the skeleton stands · one agent per `tools/blend/*.py` · one agent per doc
file · one agent who looks up in the **installed** Bevy docs what an API is really called in
this version (§3).

## g) What EVERY subagent order has to name

Otherwise it delivers the plausible instead of the correct — four points, none of them optional:

| # | Point | Reason |
|---|---|---|
| 1 | **Which files are his** and which he may *only read* | one writer per file |
| 2 | **Which sections he should read** — e.g. "`prompts/init.md` §5 + §8 + §9, `docs/architecture.md`" | "read everything" means a subagent with an 800-line order rebuilds half the prompt |
| 3 | **The evidence obligation (§9)**: what he claims, he measures — the return value carries **a test name, a measured value and the stage (§8)**, not "implemented it" | otherwise the report is an opinion |
| 4 | **No foreign ground**: what he notices but does not own goes to `docs/FINDINGS.md` | fixing it along quietly is an invisible merge conflict |

**The fixed report format of the return value:**

```
Task · Done · Evidence · Stage · Open · Findings
```

The six fields cover point 3 (**Evidence**, **Stage**) and point 4 (**Findings**); **Open** is
the honest remainder. The format stands word for word in the norm table of the commission
(`prompts/init.md` §10, line 1097, marked there explicitly with "(§17)") and in
`docs/conventions.md`: "a free-text report cannot be integrated".

**And across all of it:** `docs/STATUS.md` is the only truth about progress. A workflow that has
built something is **not done while the line is missing**.

## h) Autonomous operation — the normal case, not the exception

The user is usually not there. That changes nothing about the method, but it removes the
authority that would otherwise object — and the round has to be built against exactly that.

**A round without the user** is the same as one with him: measure the current state → write down
the hypothesis and the **acceptance criterion in advance** → delegate in parallel → check the
results **against the criteria noted beforehand** → integrate → decide on the next round. The
only difference is at the end: there is no "let me just go and ask".

| Situation | what to do | what is forbidden |
|---|---|---|
| An open decision that belongs to the user | `docs/QUESTIONS.md` with `ASSUMPTION:` **and** the place that would have to be taken back — then keep building | waiting; or deciding quietly without writing it down |
| Something is broken and reproducible | `docs/BUGS.md` with the repro, then the fix per rule 5 | a "fix on suspicion" without a red test |
| Something is broken but there is **no repro** | mark it in `docs/BUGS.md` as a rumour, work on somewhere else | guessing until it happens to go away |
| Foreign ground turns up | `docs/FINDINGS.md` | fixing it along quietly |
| The same hypothesis has failed twice | keep the count on the entry in `docs/BUGS.md`, then on to `docs/QUESTIONS.md` | a third attempt with the same assumption |

**In autonomous operation the counter-check is not a luxury, it is the stand-in for the user.**
Were he here he would object — now that has to be done by an agent who did not build the result
himself. Rule 6 from e) thereby becomes an obligation after *every* build step, not only after
discovery steps. This session proved it: two carefully built interface drafts were knocked over
by two independent attackers with 30 evidenced findings, among them a pendulum that on a short
rope would have lost 99.2 % of its speed per second, and an image criterion that could not be
evidenced because the camera in this project does not rotate at all. Nobody would have noticed
either of them until it showed up in the game.

**And the ceiling:** only the user sets ✅. Autonomously, **🟧 is the maximum**, even when
everything is green. Doubt moves the stage down, not up.

## Gaps

| Gap | what is missing |
|---|---|
| ~~"when the **limit** is reached"~~ | **closed 2026-08-09:** at most **two** build attempts and one counter-check per `F-ID`; after that it is not the execution that is wrong but the assumption → `docs/QUESTIONS.md`. At most **four building** orders per round (see d). |
| ~~"the same hypothesis has failed twice"~~ | **closed 2026-08-09:** the count stands on the entry in `docs/BUGS.md`. Without an entry there is no "second time", and therefore no second attempt either. |
| Specialist areas | the list of senior experts trails off in "…" — which domains actually get set up is a decision nobody has taken yet. |
| Cross-reference | for serial work §17 points at "the list above"; the table of main-head files sits further **down** in the source. The same list is meant. |
| Nothing is measured | **partly closed 2026-08-09:** on machine B (`offlinebot`) `nproc` = 16, but the width does not hang on that — **`cargo` takes a lock on `target/`**, so building agents wait for each other. Four at a time is the usable limit, not sixteen. For machine A the number is still 🟨. |

Related: [workflow](workflow.md) · [performance](performance.md) · [STATUS](../STATUS.md) · [TODO](../TODO.md) · [QUESTIONS](../QUESTIONS.md) · [FINDINGS](../FINDINGS.md) · [BUGS](../BUGS.md) · [ACCEPTANCE](../ACCEPTANCE.md) · [ROADMAP](../ROADMAP.md) · [architecture](../architecture.md) · [conventions](../conventions.md) · [environment](../environment.md) · [README](../README.md)
