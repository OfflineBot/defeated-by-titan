# FINDINGS — mistakes I tripped over on the way past

Updated: 2026-08-09

**Whoever trips over something that is not part of their own task: write it down, with the
measurement beside it** — so that somebody else can check whether it really is wrong.

**Do not fix it quietly on the way past.** A foreign mistake fixed in passing is a fix nobody
reviewed, and it hides in the diff of a task where nobody is looking for it
(`prompts/init.md` §9c). Format: `FIND-00n <symptom>` + measurement.

---

## FIND-001 — The anchor density in the backlog is not a number

**Symptom:** `prompts/init.md` §2 calls the anchor density „die wichtigste Zahl" (*the most
important number*) in sheet `08_Maps`. In the table it is qualitative.

**Measurement:** all 12 map rows carry `Sehr hoch` (3×), `Hoch` (4×), `Mittel` (4×) or
`Niedrig` (2×) — `grep -o 'anchor_density: "[^"]*"' docs/backlog/maps.ron`. Not one numeric
value.

**Why it counts:** bible 6.2 makes the anchor density the gate for P3 („Traversal-Zeiten
zeigen messbaren Unterschied zwischen Anfaenger und Experte" — *traversal times show a
measurable difference between beginner and expert*). Four words cannot be tuned and cannot be
checked.

**Whose it is:** level design, not the setup. Recorded as a decision in
[`docs/QUESTIONS.md`](QUESTIONS.md) Q-010.

## FIND-002 — The backlog is written for Roblox in several places

**Symptom:** not only the bible — `01_Spielfunktionen` names Roblox building blocks directly.

**Measurement:** `F-003` demands „Oberflaechen mit CollectionService-Tag `AnchorSurface`",
`F-004` names `RopeConstraint`, sheet `05_VFX` names `ParticleEmitter + Beam`, sheet `08_Maps`
measures in `studs`, `T-001` is called „Rojo- und Git-Aufsetzung".

**Why it counts:** `prompts/init.md` §2 governs the passages in the **bible** and says: what
turns up on top of that while working gets **translated and added to
`docs/architecture.md`** — not followed, not ignored, not asked back. Which is exactly what
happened: the translation table in [`docs/architecture.md`](architecture.md) now has four
rows more.

**Whose it is:** done, nothing open. It stands here so the next person knows that the table
grows instead of being fixed.

## FIND-003 — Sheet 11 of the spreadsheet is an independent cross-check, not a data source

**Symptom:** `prompts/init.md` §2 warns that sheet 11 consists of formulas and, without
`data_only=True`, hands back `=COUNTIF(...)` instead of numbers — possibly without a cached
value at all.

**Measurement:** all 47 formula cells have a cached value (`<v>` next to `<f>`);
`tools/features.py` warns if that ever stops being true. The values agree with our own
extraction **exactly**: 194 / 100 / 100 / 28 / 39 / 118 / 45 / 12 / 51, total **687**, and so
does the priority split of sheet 01 (99 Must / 71 Should / 24 Could).

**Why it counts:** the extraction is thereby **confirmed by the spreadsheet itself**, not
only by our own count. That is worth more than the row-count guard alone — those numbers come
out of a different calculation (`COUNTA` in Excel) than ours.

**Whose it is:** done.

---

*(Append further findings here. A finding without a measurement is an opinion.)*

Related: [`docs/BUGS.md`](BUGS.md) (our own bugs) · [`docs/QUESTIONS.md`](QUESTIONS.md)
