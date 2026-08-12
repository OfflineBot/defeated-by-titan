# core-loop — hook, swing, gas, cut: the four things copied from the reference and the one thing that kills

Updated: 2026-08-12 · Stage: 🟨 (the loop runs end to end in a script — `scripts/game-full.txt`
reaches `MISSION WON` — but nothing in it has been played by a human, and P1's gate is a human
gate)

## The loop in one paragraph

You are a Vanguard salvage hand with the **Vector Gear**: two grappling hooks, two gas tanks, two
blades. You hook in, you swing, you accelerate with gas, and you kill a titan **only** by a fast
cut into the **Cortex**. Everything else costs it a leg and costs you time.

**The reference is [Attack on Titan Revolution](https://www.roblox.com/games/13379208636/Attack-on-Titan-Revolution)**
(Roblox). What gets copied from it is four building blocks and a warning, not a feature list.

## The four building blocks

### 1. The Vector Gear is the core

Firing a hook, reeling the rope, swing energy, a gas boost, a boost dash, wall running. **The
game stands or falls with this feeling — not with the titan AI.** That sentence is the reason for
the whole build order: `vector/` is the one domain nothing else may be built on top of while it
is still 🟨.

### 2. The Cortex is the only truth

**A cortex hit kills, no matter how full the titan is.** Everything else is preparation:

| Hit | What it buys you |
|---|---|
| legs off | it falls |
| arms off | it cannot grab |
| eyes | it cannot see you |
| cortex | **it dies** |

There is no health bar to grind down. That is what makes a fight a positioning problem instead of
a damage problem, and it is what pillar P2 (skill beats numbers) actually rests on.

### 3. Damage comes out of speed

**A cut from standing scratches. The same cut at 30 m/s kills.** The formula belongs in the RON,
never in the code — a balancing change that needs a rebuild is a balancing change that does not
happen ([`../conventions.md`](../conventions.md), `CLAUDE.md` rule 2).

This is also the sentence that makes the movement and the combat one system rather than two: you
cannot make the fight easier by fighting better, only by *flying* better.

> **Measured, and it is a caveat:** the flight cut in `F-030` currently lands at **74.70 m/s**,
> which is `vector.max_speed_m_s` — the clamp, not a chosen speed. Recorded in
> [`../STATUS.md`](../STATUS.md) against `B-004`. A damage curve tuned against a clamped input is
> tuned against an artefact.

### 4. Economy instead of cooldowns

**Gas is finite. Blades go blunt and break.** You resupply at supply points, from a horse, or off
a fallen comrade.

A cooldown asks you to wait. An economy asks you to decide — and it is the hook the Bellower
hangs on later, because spending gas is loud ([`enemies.md`](enemies.md)). Every reflex to
replace one of these with a timer is a reflex to delete a decision.

## What else comes across, and in what shape

| Block | What is meant |
|---|---|
| **Titan kinds** | eight, with the project's own names and an anti-autopilot property on six of them → [`enemies.md`](enemies.md) |
| **Missions / raids** | a sortie has objectives and phases: clear titans, escort a squad, hold a gate, a boss with phase changes. The arc is **5–7 minutes** ([`pillars.md`](pillars.md), improvement 10) |
| **Progression in data** | XP, **Mark**/**Sigil**, gear tiers, **Traits**, **Lineage** passives — **every number in a RON file**, no balancing in Rust |
| **Multiplayer** | co-op sorties. The netcode is not built today; **the architecture is built for it from day 1** → [`../multiplayer.md`](../multiplayer.md) |

## What is deliberately not copied

**Bonding / Vessel Forms** (becoming a titan yourself), **horses**, and the **Lance Charge** are
recorded and not built — they are in [`../ROADMAP.md`](../ROADMAP.md) with the reason beside
each. Vessel Forms in particular *replace* the core movement instead of extending it, which is
exactly the wrong shape for a project whose first gate is the core movement.

## Placeholders: the freedom, and its one limit

**Downloaded assets are explicitly allowed** — this is a prototype, and the user replaces every
model, texture and sound himself at the end. Until then, what counts is that the prototype gets
good, not that every polygon is ours. A titan is a human with wrong proportions, and that is three
primitives.

**The limit is style, not ownership.** The bible's style rules apply to placeholders exactly as
they apply to finished art: low poly, flat colors, the three signal colors for gameplay only. A
placeholder that falls stylistically out of frame **falsifies the very judgement the prototype
exists to produce** — you cannot tell whether the movement reads well in a scene that does not
read well.

Where third-party files live, how they stay swappable in one line, and why they never reach the
public repository: [`../models.md`](../models.md).

Related: [`README.md`](README.md) · [`pillars.md`](pillars.md) · [`enemies.md`](enemies.md) ·
[`world.md`](world.md) · [`../multiplayer.md`](../multiplayer.md) · [`../STATUS.md`](../STATUS.md)
