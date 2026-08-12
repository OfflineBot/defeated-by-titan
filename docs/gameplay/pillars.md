# pillars — the game in one sentence, the five rules it is measured against, and the numbers that say whether it worked

Updated: 2026-08-12 · Stage: 🟨 (carried over out of the design bible; none of these numbers has
been measured in this project — there are no players yet)

## The game in one sentence

> **A movement game with a high mastery ceiling, in which fighting is the side effect of good
> movement.**

Everything else is decoration. **A feature that contradicts that sentence gets cut**, no matter
how much work already went into it. That clause is the reason this file exists at all: without a
written sentence to cut against, nothing ever gets cut.

## The five pillars

### P1 — Movement is the product

The Vector Gear is not a way of getting between fights. It **is** the fight. A player who flies
elegantly through a city without killing a single titan has to have fun. **If that does not work,
nothing works.**

**Consequence:** the Traversal Trial — a pure movement mode, no combat — is not a side mode, it is
the litmus test of the whole project. It reuses existing maps and costs almost nothing.

**The gate this pillar owns:** a blind test against the reference with ten testers, and our
movement has to be rated **at least level with it**. Not passing means iterating, not moving on.
Everything meta-shaped waits behind that gate ([`../ROADMAP.md`](../ROADMAP.md)).

> ⚠️ **An agent cannot satisfy this gate.** Ten human testers are ten human testers. Every row
> whose acceptance criterion is "feels good" is capped at 🟧 by evidence and at 🟨 by honesty
> until somebody plays it — which is what makes `user-messages.md` the most valuable file in the
> repository.

### P2 — Skill beats numbers

A level-20 player with clean technique has to finish a mission faster than a level-90 player with
bad technique. Stat growth opens content; it does not replace ability.

**The measure:** variance in time-to-kill between beginners and experts on an identical build.
**Target: at least 2.5×.**

### P3 — No progress without a guarantee

Randomness may accelerate, never gate. **Every goal in the game is reachable on a deterministic
path**, and every probability and counter is visible in the game.

**Consequence:** no empty lineages, no 0.05 % dead ends, no item a diligent player can never
reach. Pity is on everything.

### P4 — Readability before realism

Every titan attack has a **wind-up of at least 0.4 s**. The cortex is recognizable **from 100 m**.
Every kind of hit has its own sound. **The player must never have to ask why he died.**

These are hard numbers, not sentiment — they are pinned in `assets/data/*.ron` and checked by
`tests/data.rs`. The three signal colors ([`world.md`](world.md)) belong to this pillar too: they
are the reason a player at full speed, in a fight with twenty team mates, still sees what matters
to him.

> **The 100 m are contested and it matters.** `docs/features.ron` `F-030` states the criterion in
> backlog units, which converts to **28 m**, not 100 m — a factor of 3.6 in pixels. Measured at
> 1920×1080: the Husk's cortex is 36.7 px wide at 28 m and 10.3 px at 100 m
> ([`../models.md`](../models.md)). The decision is [`../QUESTIONS.md`](../QUESTIONS.md) Q-019 /
> Q-026.

### P5 — The store sells appearance only

Cosmetics, private servers, a season pass. **No inventory slots, no drop rates, no re-rolls, no
loadout slots.** A deliberate revenue decision in favour of reputation and longevity.

> **Outside Roblox this pillar has no mechanism.** There is no platform economy here. The
> *principle* stands — whatever is ever sold is appearance — but whether any of it happens at all
> is a product question for the user, filed as [`../QUESTIONS.md`](../QUESTIONS.md) Q-001.
> **None of it gets built** ([`../architecture.md`](../architecture.md), translation table).

## The ten things done better than the reference

This is the competitive argument, and it is not "more content" — it is content that was thought
through. Each one carries the measure that says whether it worked.

| # | Change | Why | Measure |
|---|---|---|---|
| 1 | **Onboarding in four stages** | the reference has no tutorial at all; players learn the controls mid-fight. The cheapest retention win in the project | first-mission completion above **80 %** |
| 2 | **Pity on everything, no empty lineages** | an 80 % roll on "nothing" at game start is a statistically guaranteed bad first impression | **100 %** of players with an active ability after 3 h |
| 3 | **An upgrade budget instead of stat ladders** | eight independent ladders produce 120 purchases and not one decision. A shared budget with trade-offs produces real builds | most-played build under **25 %** share |
| 4 | **Directed grab escape instead of key mashing** | QTE mashing tests keyboard hardware and finger stamina, not skill. A hook in the opposite direction is a real test of reaction and orientation | no measurable advantage from a higher click rate |
| 5 | **Ascension instead of a prestige reset** | mechanical skill and gear ranks survive; only build decisions reset, at a larger budget. The player gets more flexible, not weaker | ascension rate above **60 %** of eligible players |
| 6 | **An in-game compendium** | the reference pushes players onto external wikis for basic knowledge. Cheap to build, strongly differentiating | **no** value exists that is only findable outside the game |
| 7 | **A store with no progression for sale** | the reference sells inventory slots and drop rates — the solution to problems it created itself | store audit: **zero** stat-effective items |
| 8 | **The Traversal Trial as its own mode** | the best thing about the genre is the movement, and no title in the genre has a mode made only of it | weekly active users of the mode above **25 %** |
| 9 | **Anti-autopilot enemies** | without it the combat degenerates into clicking on targets ([`enemies.md`](enemies.md)) | TTK variance beginner→expert above **2.5×** |
| 10 | **A 5–7 minute mission arc** | the reference averages ~21 min per session, that is 2–4 missions. Each has to be a complete arc with guaranteed, felt progress | mean mission length **5–7 min**, abandon rate under **8 %** |

## The success metrics

Collected continuously from P4 onward. They are here so that "it feels better now" has something
to fall against.

| Metric | Target | Why this number |
|---|---|---|
| First-mission completion | > 80 % | the direct test of the onboarding |
| D1 retention | > 35 % | good-industry-normal for the genre |
| D7 retention | > 15 % | shows whether the loop carries |
| Mean session length | 20–30 min | the reference sits at about 21 min |
| Missions per session | 3–4 | confirms the mission length |
| TTK variance beginner→expert | > 2.5× | proves skill counts (P2) |
| Share of the most-played build | < 25 % | proves real build variety (improvement 3) |
| Players with an active lineage after 3 h | 100 % | proves pity works (P3) |
| Weekly Traversal Trial usage | > 25 % | proves movement carries on its own (P1) |
| Crash rate | < 0.5 % | basic technical hygiene |
| Mean FPS (minimum / full profile) | 60 / 60 | the reference delivers a stable ~60; that is the bar |
| Share of sessions with team mates | > 70 % | proves the multiplayer loop carries |
| Reconnect rate after a drop | > 60 % | shows whether the 120 s slot reservation really works |

## The decisions the bible left open

All five are recorded, none of them blocks the work, and each one runs under a written
`ASSUMPTION:` in [`../QUESTIONS.md`](../QUESTIONS.md):

| Open question | Where |
|---|---|
| PvP, yes or no? (specified as pure co-op; PvP would be a second project) | Q-003 |
| Vessel Forms in v1.0 or v1.5? (the single most expensive item; prepare technically, build later) | Q-004 |
| Trading between players, yes or no? (its value drops sharply under P3) | Q-005 |
| Music: own composer or a licence library? | affects P4 onward; only original or licensed music, ever ([`../models.md`](../models.md)) |
| **Who owns config authority?** | **answered, and it is the one this project settled first:** the numbers live in `assets/data/*.ron`, never in Rust ([`../conventions.md`](../conventions.md), `CLAUDE.md` rule 2). The bible's own warning — "if balancing values sit in the code, the project is unsteerable after six months" — is the reason that rule outranks convenience |

Related: [`README.md`](README.md) · [`world.md`](world.md) · [`enemies.md`](enemies.md) ·
[`core-loop.md`](core-loop.md) · [`../ROADMAP.md`](../ROADMAP.md)
