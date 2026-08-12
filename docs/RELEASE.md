# RELEASE — the finish line: carry over, clean up, publish, and dissolve the scaffolding

Updated: 2026-08-12 · Stage: 🟨 (step 1 is audited and closed — see
[`FINDINGS.md`](FINDINGS.md) FIND-048; steps 2–4 have not been executed)

`prompts/` and `gameplay/` are **bootstrap scaffolding**, not part of the finished project. They
come down in four steps, **in this order**, and this file is where the order lives now that the
file which used to hold it is going away.

> ⚠️ **Never delete a file you have not finished carrying over**, and never one the user has just
> put in (`ls -lt prompts/ gameplay/` before dismantling anything). In doubt: leave it and ask.
> **The history is a safety net, but only for a commit that was pushed.**

---

## Step 1 — carry over: the scaffolding may end up holding nothing unique

**Carrying over means rewriting, not pasting.** Whatever the code or the docs already say gets
cut; whatever a future agent needs gets spelled out. Two sources for the same rule means one of
them will be lying soon, and you will not know which.

This is the map the commission itself laid down, with the permanent home each part went to. It is
kept as a record, so that a reader who finds the deleted file in the history can see where each
section landed:

| From `prompts/` | Permanent home |
|---|---|
| Rules that always hold (RON, domain standalone, the four stages, the bug doctrine, "per second") | [`../CLAUDE.md`](../CLAUDE.md) — as an **index**, short, with pointers |
| Bevy setup and the engine traps | [`lessons/bevy.md`](lessons/bevy.md) |
| Axes, units, looking direction | [`conventions.md`](conventions.md) §1 |
| Domains, plugin order, the allow list | [`architecture.md`](architecture.md) |
| The model chain, and the other asset chains | [`models.md`](models.md) (+ a comment header in every `tools/blend/*.py`) |
| The four stages | the head of [`STATUS.md`](STATUS.md), short form in `CLAUDE.md` |
| The bug and safety doctrine | the head of [`BUGS.md`](BUGS.md), short form in `CLAUDE.md` |
| Performance rules | [`lessons/performance.md`](lessons/performance.md) |
| Flags, `--script`, screenshots | [`lessons/workflow.md`](lessons/workflow.md) + the key bindings in the root `README.md` |
| Two machines + environment traps | [`environment.md`](environment.md) (measured) and [`lessons/environment.md`](lessons/environment.md) (what they cost) |
| Working method: parallelism + scientific method | [`lessons/supervision.md`](lessons/supervision.md) |
| Game content and the reference | [`gameplay/`](gameplay/README.md) |
| What deliberately comes later | [`ROADMAP.md`](ROADMAP.md) |
| The acceptance of the commission itself | [`ACCEPTANCE.md`](ACCEPTANCE.md) |
| The finish line itself | **this file** |

*(The table above named German file names — `konventionen.md`, `architektur.md`,
`arbeitsweise.md`, `umgebung.md`, `FRAGEN.md`, `modelle.md`. This repository is English
throughout since 2026-08-09; the names above are the translated ones. The mapping is here so that
a reader of the old file is not left hunting for a file that never existed here.)*

### The check, and it is a grep, not a feeling

```bash
grep -rn "prompts/init.md" . --exclude-dir=target --exclude-dir=.git
```

**Every remaining hit has to be a pointer to history, not load-bearing content.** A citation like
"(`prompts/init.md` §9)" next to a rule that now lives in `docs/BUGS.md` is an *attribution* — it
is safe to delete the file, but the citation is then pointing at nothing, and the honest fix is to
repoint it at the permanent home. **Rewrite the references before the `git rm`, not after.**

And read [`../CLAUDE.md`](../CLAUDE.md) once as a stranger: *does it tell me in thirty seconds how
this project ticks and where the traps are?* If not, the step is not finished — no matter how
clean the grep is.

---

## Step 2 — before publishing: public means public

What is pushed once is indexable, including after a deletion. **The check comes before the
push:**

- **`.gitignore`** covers `target/`, `saves/`, `*.blend1`, `*.blend2`, `*.log`, temporary scripts
  and **`assets/extern/`**. **`assets/3d/glb/` is NOT ignored** — the game has to run on a machine
  without Blender.
- **No credentials, tokens, keys or paths with real names** — in the working tree *and* in the
  history:
  ```bash
  git log -p | grep -niE "token|secret|api[_-]?key|password"
  ```
- **`assets/extern/` is ignored**, `ATTRIBUTION.md` is complete (one line per file) and
  `tools/fetch_external.sh` fetches everything back ([`models.md`](models.md)). The replacement
  report goes to the user before he starts replacing anything.
- `cargo test` green, `cargo build --release` green, `grep -rn "TEMP" src/` empty.
- The root **`README.md`** reads for a stranger: what this is, how to start it, what is finished
  (with the stage legend), which keys. Plus a **`LICENSE`** — which one is the user's decision and
  is filed as [`QUESTIONS.md`](QUESTIONS.md) Q-007, which runs under the assumption *no `LICENSE`
  file* (= all rights reserved). **Until he answers, no licence file gets invented.**

---

## Step 3 — the public GitHub repository

The user authorized this in advance; it is part of the commission and needs no further question.
But **only after steps 1 and 2**.

**Look first whether the repo already exists** — the scaffolding was pushed once when the
commission was set up, so this is usually a push, not a creation:

```bash
gh auth status                      # not logged in? ask the user to type: ! gh auth login
git remote -v                       # is there an 'origin'?
git push -u origin main             # yes -> just push

gh repo create defeated-by-titan --public --source=. --remote=origin --push \
   --description "3D titan-fighting game in Bevy (Rust) — Vector Gear, cortex hits, co-op sorties"
                                    # no -> create it like this
```

> **As of 2026-08-12 `origin` already exists** (`git@github.com:OfflineBot/defeated-by-titan.git`).
> So this step is a push and a **visibility check**, not a creation. Whether the remote is public
> has not been verified here; `gh repo view --json visibility` answers it in one command.

Afterwards: name the URL in the chat and mention `gh repo view --web` once. **If `gh` is missing
or not logged in, do not improvise with `git remote add` and a guessed URL** — ask the user for
the login and finish the rest.

---

## Step 4 — dismantle the scaffolding

**One line per commit**, so the history shows where each thing went:

```bash
# 1. the inbox: design -> docs/gameplay/, work -> docs/TODO.md, numbers -> the RON files,
#    images -> docs/gameplay/images/ — ONLY THEN:
git rm -r gameplay/
git commit -m "gameplay dissolved: design -> docs/gameplay, work -> docs/TODO.md"

# 2. the prompts, one at a time, each only once it is worked through AND carried over:
git rm prompts/init.md
git commit -m "docs: initial prompt carried over and removed (readable in the history)"
git rm -r prompts/ 2>/dev/null   # only when it is empty
git rm init.md                    # the starter in the root directory, last
git commit -m "chore: bootstrap scaffolding removed — README, CLAUDE.md and docs lead from here"
git push
```

**`gameplay/features.xlsx` is never deleted.** It moves to `docs/gameplay/features.xlsx` and stays
the source; `tools/features.py` gets repointed at the new path in the same commit.

**Rewrite the references before every `git rm`, not after.** A move is finished only when

```bash
grep -rn -e 'prompts/' -e 'gameplay/' . --exclude-dir=target --exclude-dir=.git
```

shows nothing but hits that speak of the history **on purpose**. Every link in `README.md`,
`CLAUDE.md`, `docs/**`, in the RON files and in the `tools/` scripts points at the new place
afterwards. `python3 tools/norms.py` then has to stay clean: **no dead markdown link, no
unreferenced file.** That is part of this step, not "tidying up later".

### The two sentences that have to survive the deletion

1. **In `CLAUDE.md`, one line stays:** *"The initial prompt has been worked through and deleted;
   it is readable in the git history (`git show <sha>:prompts/init.md`)."* The history is the
   safety net — which is why the file is **deleted and not merely renamed**.
2. **In the root `README.md`, tell the user where his wishes go now:** design into
   [`gameplay/`](gameplay/README.md), work into [`TODO.md`](TODO.md), play notes into
   `user-messages.md`. Without that line he drops a file tomorrow into a folder that no longer
   exists.

### And the other files in `prompts/`

**Each is deleted as soon as it is worked through and carried over** — individually, with a commit
message that says where its content went. `DefeatedByTitan_Design-Bibel.md` is carried into
[`gameplay/`](gameplay/README.md); whatever the user adds later follows the same route. What is
still open stays, and so does the folder. Once `prompts/` is empty, it goes.

Related: [`README.md`](README.md) · [`gameplay/README.md`](gameplay/README.md) ·
[`FINDINGS.md`](FINDINGS.md) · [`QUESTIONS.md`](QUESTIONS.md) · [`conventions.md`](conventions.md)
