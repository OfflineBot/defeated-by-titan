# enemies — why at least half of them have to break the autopilot, and what each one teaches

Updated: 2026-08-19 · Stage: 🟨 (the chapter is the design; what of it is in the code stands in
[What is built of this](#what-is-built-of-this-2026-08-19) below — seven of the eight kinds
fight differently, one of them cannot spawn, and none of the numbers has been played)

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
| **The cortex is the only lethal spot** | every other hit zone is preparation, not damage. Legs off = it falls; arms off = it cannot grab; eyes = it cannot see you. `combat` sends the hit, `titan` decides what it means for its body ([`../architecture.md`](../architecture.md)). **The zones exist since 2026-08-19** — arms and legs; the fall, the grab and the blinding do not (see below) |
| **Every attack has a wind-up of ≥ 0.4 s** | a readability guarantee (P4), pinned in the RON — an attack that undercuts it is a bug, not a difficulty setting |
| **Titans vaporize, they do not bleed** | steam, not blood ([`world.md`](world.md)) |
| **Size classes, not per-kind heights** | `assets/data/titan.ron` carries a size class per kind; the heights themselves live once, in `scale.ron` ([`../models.md`](../models.md)) |
| **The cortex sits at ~89 % of body height and is smaller than the head** | both are pinned by `tests/data.rs` — a cortex that is a point feels like a broken game |

## What is built of this (2026-08-19)

**Seven of the eight kinds now fight differently**, out of `assets/data/titan.ron: <kind>.behaviour`
(`F-057`..`F-063`): the errant swerves, the scuttler lunges through his own blow, the chorus pair
splits, the lurker never takes a step, the warden's hand covers his nape, the weaver is only
cuttable inside his own attack — and, since today, rolls out of it.

**Secondary hit zones exist** (`F-032`). A blade now reports `ArmLeft`, `ArmRight`, `LegLeft` or
`LegRight` where it used to report the catch-all `Torso` for the whole body — measured on the real
husk, `scripts/f032-swords.txt`: the pass that produced *"cut titan 3 Torso"* at every height in
`docs/FINDINGS.md` FIND-109 now produces `Torso` on the chest line and `LegLeft` a few ticks later
as the blade falls past the knee. What a limb hit **means** is still only the stagger every body
cut buys: *"Bein-Treffer laesst Titan stuerzen"* and *"Augen-Treffer erzeugt 3 s
Orientierungslosigkeit"* (`F-032`'s acceptance) are the next step and are **not built**, and there
is no eye zone at all — an eye is 20 cm on a 10 m body and the box rig has no feature that small.

**The weaver's roll is built** (`F-059`). His attack ends in `TitanState::Roll` instead of walking
back into `Pursue`: `roll_startup_s` of crouch with his nape **still a target** — a longer window
than he had, and the readable startup the design asks for — then the rest of `roll_s` with the
cortex sensor out of the world while he carries himself `roll_speed_m_s` backwards. Measured:
27 ticks, 9 of them open, **3.90 m of retreat**. He does *not* roll on approach, and that is a
decision rather than a shortcut: his nape is out of the world in `Idle` and `Pursue` anyway, so
i-frames there would be invulnerability on a hit zone that is already gone.

🔴 **The bellower still cannot spawn, and since 2026-08-20 there is only ONE reason left.** It is
the one `docs/QUESTIONS.md` Q-028 records: he is class `huge` and `scale.ron: max_spawnable_class`
is `large`.

There used to be a second, measured on 2026-08-19 by raising the cap and running the suite, and it
was worse than "half a kind": **his nape could not be reached at all.** A 21 m body at
`width_fraction` 0.25 is 2.625 m of radius, plus the player's 0.35 m is 2.975 m of clearance,
against `reach_m` 1.60 + `cortex_radius_m` 0.70 + `thickness_m` 0.12 = 2.42 m of blade —
**−0.555 m** (`docs/FINDINGS.md` FIND-124). **That is closed.** `cortex_radius_m` 0.70 → 1.16 (the
head rule's own ceiling for a 21 m body), `reach_m` 1.60 → 2.00 and `thickness_m` 0.12 → 0.20 give
**3.36 m of blade against 2.975 m of clearance = +0.385 m**, and the test below now asserts that
direction instead (FIND-147). And the thing he exists for, the **ear**, is
`F-051` and does not exist, so a spawnable bellower today calls on sight and pulls a 90 m radius
with no counterplay, because the counterplay this chapter specifies is *play quietly* and nothing
can hear gas. Both are pinned by `tests/titan.rs::f064_the_bellower_stays_blocked_until_the_ear_exists`,
which is the one place in the repository that names him against the cap: **lifting the cap is one
line in `scale.ron` plus deleting that test, and nothing else.**

## The open one

**Is "Abnormal / Boss — 28 m" a size class or a statement about the Errant?** The user's own
table can be read either way; the project reads it as a size class, `titan.ron` leaves the Errant
at 10 m, and the class `boss` has no representative yet. **That is an assumption, not a
translation** — [`../QUESTIONS.md`](../QUESTIONS.md) Q-020, withdrawn by one sentence from him.

Related: [`README.md`](README.md) · [`world.md`](world.md) · [`core-loop.md`](core-loop.md) ·
[`../models.md`](../models.md) · [`../conventions.md`](../conventions.md)
