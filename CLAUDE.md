# CLAUDE.md — how this project works

**Defeated by Titan** — a 3D low-poly action game about the fight against Titans, in **Bevy
(Rust)**. The core is the **Vector Gear**: two hooks, two gas tanks, two blades. A Titan dies
**only** from a fast cut into the **Cortex**.

> 🔒 **The engine is Bevy (Rust). NOT Roblox.** Confirmed by the user on 2026-08-09. The
> design bible is written for Roblox in six places — those places are **translated, not
> obeyed**. The translation table lives in [`docs/architecture.md`](docs/architecture.md) and
> it **grows**: whoever runs into a new Roblox instruction adds the line there.

**This project is still being built up out of `prompts/` and `gameplay/`.** That is bootstrap
scaffolding and it disappears once it has been carried over (`prompts/init.md` §18). Until
then: `gameplay/` decides the **content**, `prompts/` the **craft**.

---

## Start of a session — always, in full, in this order

```bash
hostname                                    # 'debian' = no window, and that is fine
ls -lt prompts/ && ls -R gameplay/          # new commission? The user adds work at any time
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
6. **Nothing changes per frame, everything per second**, and nothing runs over all entities to
   answer a question about the ten meters in front of your nose.
   → [`docs/lessons/performance.md`](docs/lessons/performance.md)

## How the work is done: **a supervisor in the `/loop` who writes nothing himself**

This project is **not worked through serially by one head**, but with workflows and subagents
— **broadly parallel and scientifically**. That is binding, not recommended
(`prompts/init.md` §17, spelled out in
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

The build-up plan (`prompts/init.md` §13, `Stufenplan`) is at **setup**. From its step 3 on,
the bible's phase plan takes over — and **its hard rule: no meta system before the Vector Gear
gate is passed.** Trait tree, economy, lineages, raids, cosmetics do **not get started** while
the movement does not yet feel convincing.

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
look at the user-messages.md. 
its what the player feels about the game. stuff that has to change.
the next time you see this EXTRA tab. rewrite it so it makes more sense for you! its for the players notes. when play testing the game. 
