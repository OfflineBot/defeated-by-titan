# gameplay — the design: what the game is, why it is that, and where the user's wishes land

Updated: 2026-08-12 · Stage: 🟨 (carried over out of the design bible and the commission; the
content is settled, but almost none of it has been built yet — the stages live in
[`../STATUS.md`](../STATUS.md))

**This folder is the WHY.** `docs/STATUS.md` says how far a thing is, `docs/TODO.md` says what
comes next, `assets/data/*.ron` holds the numbers — and this folder says **what the game is
supposed to be and why a number is the number it is**. A design decision that lives only in a
commit message is a decision the next session will overturn by accident.

| File | What is in it |
|---|---|
| [`pillars.md`](pillars.md) | the game in one sentence, the five design pillars, the ten things done better than the reference, the success metrics |
| [`world.md`](world.md) | setting, tone, what titans are, the visual style, the platform rules |
| [`enemies.md`](enemies.md) | the enemy philosophy, the eight titan kinds, the anti-autopilot rule |
| [`core-loop.md`](core-loop.md) | the Vector Gear loop: hook, swing, gas, cut — and the four rules the reference is copied under |

## Where the user puts his wishes

**Design goes here, one file per topic. Work goes to [`../TODO.md`](../TODO.md). Numbers go to
`assets/data/*.ron`. What felt wrong while playing goes to `user-messages.md` in the repository
root.** That is the whole routing table, and it exists because the original inbox — the
`gameplay/` folder next to `prompts/` — is bootstrap scaffolding that gets dissolved
([`../RELEASE.md`](../RELEASE.md)). Whoever dissolves it writes this sentence into the root
`README.md` too, or the user drops a file tomorrow into a folder that no longer exists.

**Nothing the user wrote gets deleted, overwritten or ticked off in place.** It gets *migrated*
— into this folder, into `docs/TODO.md`, into a RON, into [`../QUESTIONS.md`](../QUESTIONS.md) —
and it gets **quoted verbatim** on the way, because his phrasing carries information a paraphrase
loses. Ticking off happens in `docs/STATUS.md` and nowhere else.

## The order of precedence, now that there is no `prompts/` any more

It used to read: `gameplay/` decides the content, `prompts/` decides the craft, and the design
bible beats the commission wherever the two disagree about content. With the scaffolding gone,
the same rule survives in three lines:

1. **A live instruction from the user beats everything**, including his own earlier number. That
   precedence is not a courtesy; it is written into `assets/data/scale.ron` and into
   [`../QUESTIONS.md`](../QUESTIONS.md) Q-002, and the hook range moving 90 m → 200 m on
   2026-08-10 is what it looks like when it fires.
2. **This folder decides the content**, `docs/conventions.md`, `docs/architecture.md` and
   `CLAUDE.md` decide the craft. A conflict between content and craft is not a conflict: the
   content says *what*, the craft says *how it is proven*.
3. **A conflict that does not resolve that way is not ours to settle** → `docs/QUESTIONS.md`,
   with the `ASSUMPTION:` the work continues under and the place that would have to be rolled
   back.

## The spreadsheet is the work list — and it is read by script, never typed

`gameplay/features.xlsx` is the production backlog: twelve sheets, 245 rows that became `F-` and
`T-` ids. It is **extracted**, never transcribed — with hundreds of rows, typing loses lines and
nobody finds out which. `tools/features.py` does the extraction with the standard library alone
(an `.xlsx` is a ZIP of XML), which is why it runs on a machine with no `pip`
([`../lessons/environment.md`](../lessons/environment.md) trap 3).

**The proof that nothing was lost is a number:** rows in the sheet == records in the extract.
That count is the guard in `tools/features.py --check` and it stands in
[`../backlog/README.md`](../backlog/README.md). If it does not match, the extraction is not
finished — and you know exactly how many rows are missing instead of suspecting that some are.

### The six ways a spreadsheet looks completely read and is not

Every one of these produces a clean-looking export with content missing from it:

| Trap | What happens |
|---|---|
| **Several sheets** | the second sheet is often half the game. Walk all worksheets, not the active one |
| **Formulas instead of values** | the cell holds `=COUNTIF(...)` instead of a number. Sheet `11_Zusammenfassung` is nothing but formulas — it is deliberately **not** transferred and serves as an independent counter-check instead ([`../FINDINGS.md`](../FINDINGS.md) FIND-003) |
| **Meaning in the COLOR** | a priority or status coded as a fill color is **invisible** in every text export. Read the fill, or ask in `docs/QUESTIONS.md`. Do not ignore it |
| **Merged cells** | one heading across five columns delivers four empty values |
| **Hidden rows and columns** | still content |
| **Trailing blank rows, cell comments** | the row count lies high, and a comment sometimes holds the actual requirement |

### When the user drops in a new version of the spreadsheet

**Extract again and diff the extraction** — never overwrite and move on:

- **new rows** come in as ⬜;
- **changed rows** keep their stage and get a note saying what changed;
- **rows that vanished are not silently deleted.** They go into `docs/QUESTIONS.md` as a list,
  because a row disappearing is either a decision of his or an accident of his, and those two
  need opposite treatments.

The work state — stage, evidence, note — lives in `docs/features.ron` and is **carried over** on
a re-run. That is the only reason a re-extraction is cheap.

### The backlog's own status column maps onto the four stages, one to one

Two status systems side by side would be two truths about progress. There is one:

| Backlog | Stage | who sets it |
|---|---|---|
| `Offen` | ⬜ unbuilt | — |
| `In Arbeit` | 🟨 built, untested, unseen | Claude |
| `Review` | 🟧 proven (test + picture + number) | Claude |
| `Fertig` | ✅ accepted | **only the user** |
| `Zurueckgestellt` / `Gestrichen` | stays ⬜, with a note | the user |

**And the MoSCoW column is the build order:** `Must` before `Should` before `Could`. On a
schedule conflict every `Could` falls first, then Vessel Forms entirely — that is the bible's
own instruction (6.4), not a suggestion.

Related: [`../ROADMAP.md`](../ROADMAP.md) · [`../conventions.md`](../conventions.md) ·
[`../QUESTIONS.md`](../QUESTIONS.md) · [`../backlog/README.md`](../backlog/README.md)
