# references — numbers taken from research, each with its source

Updated: 2026-08-12 · Stage: 🟨 (researched once, not attacked by a second head)

**Why this file exists.** On 2026-08-12 a research round produced the geometry the district is
built to, and it was written into a scratch directory that a later agent could no longer find —
because the session directory had changed underneath it. The numbers below are load-bearing for
`assets/data/maps.ron` and `assets/data/scale.ron`, so they live in the repository now.

**How to read it.** Every figure carries a confidence: `canonical` (stated by the source work),
`real-measured` (surveyed from the real town the district is modelled on) or `inferred`. **Where a
figure is inferred, it is a decision, not a fact** — and it is the user's to overrule.

⚠️ **We build our own game.** These are proportions and measurements used to inform geometry;
nothing here reproduces artwork or text, and every name in this project is our own
(`docs/architecture.md`'s translation table).

## The three figures that are canonical

| what | value | note |
|---|---|---|
| the wall | **50 m** | our `scale.ron: wall.height_m` is **120 m** — that is 50 × `wall_factor: 2.4`, and the project's own scaling turned out to agree with the source without anyone having checked |
| the largest titan | **60 m** | above our current `titan.ron` ceiling; see `Q-028` |
| the gate opening | **8 m** | the only size anchor for a gate anywhere in the source material |

**There is no canonical plan of the district.** The wiki material is plot, not geometry. Everything
below therefore comes from the real town the district is modelled on.

## The street-to-height ratio — the number that decides traversal

Surveyed from **Nördlingen** (Bavaria) via OpenStreetMap: **1 344 buildings, 463 ways, 4 100 street
centreline samples measured against building outlines.**

| | measured | confidence |
|---|---|---|
| street width, facade to facade | **8.1 m** (residential 7.7 m) | real-measured |
| building ridge | **11–13 m** — 167 of 246 tagged buildings are 2-storey, 65 are 3 | real-measured (storeys), inferred (absolute ridge) |
| **street : height** | **0.62 : 1** | real-measured |
| building height | ≈ **1.6 × street width** | derived |

**Why this is the most important number in the file:** a rope swings only while the horizontal gap
to the anchor is smaller than the anchor's height (`d < H`, `FINDINGS.md` FIND-041). At 0.62 : 1 the
houses **are** the traversal route — `d < H` holds from mid-street to the ridge opposite. **Dense
medieval streets and good traversal are the same design**, which is why the fix for "it does not
look right" and the fix for "the rope does nothing" were the same change.

## The build numbers derived from it

| | value | confidence |
|---|---|---|
| frontage per house | 12–14 m | real-measured |
| party walls | **zero gaps**, closed block perimeters | real-measured |
| street width | 7.5–9 m | real-measured |
| block pitch | 40–45 m | real-measured 60, tightened so courtyards stay swingable |
| ground coverage | 39 % | real-measured |
| density | ~24 buildings / ha | real-measured |
| district footprint | **900 × 470 m** | **inferred** — source estimates span 5 × 5 km down to 0.86 km and cannot be reconciled |

## What could not be established, and is therefore invented deliberately

- **The waterway.** No source gives it a course or a width, and the model town has **no river inside
  its wall at all** — only a moat. **Our canal is a gameplay decision, not fidelity**: it is a
  lowered channel with bridges, because a district needs a barrier that is crossed rather than
  flown over (`FIND-056`).
- **The garrison headquarters.** Only its *function* is canonical — gas and blades are kept there
  and it is fought over. Its size, plan and position in the district are ours (`F-019`, `FIND-064`).
- **The district's true size**, see above.
- **Anything between 12 m and 35 m.** Our own size table has a hole there, and it is why the swing
  lanes are still scaffolding rather than architecture — `docs/QUESTIONS.md` **Q-036**, open, the
  user's to answer.

## The lesson this file is also here to record

**A scratch directory is not a deliverable.** The research above cost a full round and was written
only to `/tmp`; a later agent looked for it, could not find it, and rebuilt its conclusions from
two `FINDINGS` entries instead. **Research that anything gets built on belongs in `docs/`.**
