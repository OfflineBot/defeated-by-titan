# enemies — why at least half of them have to break the autopilot, and what each one teaches

Updated: 2026-08-12 · Stage: 🟨 (carried over out of the design bible; only the Husk exists in
the code, and only as a body with a cortex — no AI, no attack cycle)

## The finding this whole chapter comes out of

The most important result of analysing the reference: **of its enemy types, exactly one demands
real timing** — the one that rolls back when you approach the nape and is invulnerable
afterwards. Everything else is mobile feed.

That is not a small complaint. It is the difference between a combat system and a target range,
and it is invisible for the first ten hours of play — which is why it has to be a rule up front
rather than a fix later.

> ## The rule: **at least half of all enemy kinds carry an anti-autopilot property.**

**This is a requirement on P6, not a feature to be added afterwards** ([`../ROADMAP.md`](../ROADMAP.md)).
An enemy roster built without it produces a game where the optimal approach is the same approach
every time, and no amount of later tuning recovers that.

## The eight kinds

The names are binding — they come from sheet `10_Namensschema` and no reference term survives in
the code, the assets, the UI or the docs ([`../conventions.md`](../conventions.md) §2).

| Kind | The autopilot break | What the player has to learn |
|---|---|---|
| **Husk** | — | the basics of approach angle |
| **Errant** | unpredictable changes of direction | leading your shots, anticipation |
| **Scuttler** | very high speed, a leaping attack | vertical evasion |
| **Weaver** | a dodge roll with i-frames after the startup | timing instead of spam |
| **Warden** | actively shields the cortex with its hand | a two-stage attack: arms first, then the cortex |
| **Lurker** | motionless ambush, a grab out of the air | changes of altitude, attention |
| **Bellower** | reacts to the **sound of gas**, calls reinforcements | resource discipline, playing quietly |
| **Chorus** | pairs cover each other | target prioritization, separation |

**Six of eight break the autopilot.** The Husk is the teaching piece and the Errant is the first
step up; every kind after that costs a specific habit.

### The Bellower and the Lurker together add a stealth layer

The reference has nothing like it: **spending gas becomes loud.** That couples the resource to
risk instead of leaving it a pure timer — which is the same move as "economy instead of
cooldowns" in [`core-loop.md`](core-loop.md), one floor up.

It also means the gas number is doing two jobs at once, and the second one only appears in the
game once the Bellower does. Whoever tunes gas before then is tuning half a system.

## Four raid bosses

**The Bound One · The Dancer · The Bulwark · The Ashwalker** — renamed from the reference in
sheet `10_Namensschema`. They belong to P9 and are explicitly behind the Vector Gear gate
([`../ROADMAP.md`](../ROADMAP.md)). The Ashwalker is the one with a scale consequence today: at
**150 m** it stands 30 m above the wall, and that number is already in `assets/data/scale.ron`.

## What the enemy design already forces on the code

None of this is AI work, and all of it is decided:

| Decision | Consequence |
|---|---|
| **The cortex is the only lethal spot** | every other hit zone is preparation, not damage. Legs off = it falls; arms off = it cannot grab; eyes = it cannot see you. `combat` sends the hit, `titan` decides what it means for its body ([`../architecture.md`](../architecture.md)) |
| **Every attack has a wind-up of ≥ 0.4 s** | a readability guarantee (P4), pinned in the RON — an attack that undercuts it is a bug, not a difficulty setting |
| **Titans vaporize, they do not bleed** | steam, not blood ([`world.md`](world.md)) |
| **Size classes, not per-kind heights** | `assets/data/titan.ron` carries a size class per kind; the heights themselves live once, in `scale.ron` ([`../models.md`](../models.md)) |
| **The cortex sits at ~89 % of body height and is smaller than the head** | both are pinned by `tests/data.rs` — a cortex that is a point feels like a broken game |

## The open one

**Is "Abnormal / Boss — 28 m" a size class or a statement about the Errant?** The user's own
table can be read either way; the project reads it as a size class, `titan.ron` leaves the Errant
at 10 m, and the class `boss` has no representative yet. **That is an assumption, not a
translation** — [`../QUESTIONS.md`](../QUESTIONS.md) Q-020, withdrawn by one sentence from him.

Related: [`README.md`](README.md) · [`world.md`](world.md) · [`core-loop.md`](core-loop.md) ·
[`../models.md`](../models.md) · [`../conventions.md`](../conventions.md)
