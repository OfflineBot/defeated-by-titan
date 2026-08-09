# measurements — the raw evidence behind the architecture decisions

Updated: 2026-08-09 · Stage: 🟧 (every file here is a command output, not an opinion)

**Why this folder exists:** the rope decision cost three measurement rounds and about eight
hours of agent time. The summary lives in [`../HANDOVER.md`](../HANDOVER.md) §3, but a summary
cannot be re-checked. These are the reports the summary was built from — with the file:line
citations into avian's source and the raw numbers.

Do not re-run these measurements. If you disagree with a conclusion, read the report first:
several of them contain a counter-check that already refuted an earlier, plausible answer.

| File | What it settles |
|---|---|
| [`avian-first-probe.md`](avian-first-probe.md) | Does avian work at all here? Rope, two ropes, reel-in, raycast, tunnelling, substeps — the first nine probes |
| [`joint-vs-wall.md`](joint-vs-wall.md) | Does reeling in pull the player through a wall? (Yes, in 23 ticks — and which of four repairs actually helps) |
| [`joint-vs-hybrid.md`](joint-vs-hybrid.md) | `DistanceJoint` against a hand-written rope solver, same scenarios side by side |
| [`move-and-slide.md`](move-and-slide.md) | avian's sweep solver as a referee between rope and geometry — and why none is needed |
| [`rope-decision.md`](rope-decision.md) | The four-line recommendation with the number carrying each line, and what a rollback would cost |
| [`avian-blockers.md`](avian-blockers.md) | Three blockers that each cost a day if rediscovered: `finish()`, the self-hitting ray, `CollisionEventsEnabled` |
| [`stage3-commissions.js`](stage3-commissions.js) | The commissions of the interrupted Vector Gear round, verbatim — re-runnable |

**The one thing to read if you read nothing else:** `rope-decision.md`. Its author refuted his
own intermediate assumption inside the same report and left the refutation standing.
