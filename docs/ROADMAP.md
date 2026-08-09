# ROADMAP — what deliberately comes later

Updated: 2026-08-09

**This is what has been recorded, understood and deliberately not built.** The difference to
`docs/TODO.md`: that file holds work that is coming up. This one holds work that is **not**
coming up, and **why** — so that nobody starts it by accident and nobody believes it was
forgotten.

## The rule that sorts everything else

> **No meta system before the Vector Gear gate is passed.**
> Skill tree, economy, lineages, raids and cosmetics are **not started** as long as the
> movement does not feel convincing (bible 6.1, `prompts/init.md` §2).

The genre's graveyard is full of games with elaborate skill trees and movement that feels
wrong. **The P1 gate is a blind test against the reference with ten testers; our movement has
to be rated at least level with it. Not passing means iterating, not moving on.**

## After the gate, in this order (bible 6.2)

| Phase | Content | Gate |
|---|---|---|
| **P2 Combat core** | one titan with a full attack and death cycle, cortex hit zone, blade durability, resupply, hit stop | One minute of fighting a single titan is fun **without any reward at all** |
| **P3 First map** | Ashgate District as a graybox with tuned anchor density, then an art pass | Traversal times show a measurable difference between beginner and expert |
| **P4 Mission loop** | Skirmish and Breach, director system, debrief, rewards | A player voluntarily plays three missions back to back |
| **P5 Onboarding** | four tutorial stages, training grounds, adaptive hints | 80% completion rate on the first mission |
| **P6 Enemy variety** | all eight kinds, size classes, group dynamics | Test players can name every kind and explain how to counter it |
| **P7 Progression** | levels, gear budget, skill tree, traits, lineages with pity, compendium | Four different builds land within 10% of each other in effectiveness |
| **P8 Content build-out** | maps 2–5, mission modes, modifiers, Traversal Trial | Every map has a recognizably distinct traversal identity |
| **P9 Raids** | raid framework, two bosses, matchmaking, loot, environmental weapons | A group fails on the first attempt and wants to go again immediately |
| **P10 Vessel Forms** | bonding, two forms with their own moveset | The form does not feel like an enlarged player |
| **P11 Polish** | accessibility, latency and load tests, telemetry, season structure | Load test with 20 players and a full titan budget without a frame drop |

## Explicitly not today

| Item | why later | where it is recorded |
|---|---|---|
| **Bonding / Vessel Forms** (9 rows) | the single most expensive item: its own rigs, ~60 animations, its own balancing — and it **replaces** the core movement instead of extending it | `docs/QUESTIONS.md` Q-004 (v1.0 or v1.5) |
| **Lance Charge** | ranged weapon; presupposes the combat core | `docs/features.ron` |
| **Horses** | locomotion outside the Vector Gear; only once the gear stands | `prompts/init.md` §1 |
| **The actual netcode** | the *architecture* is built for it from day 1, the *code* is not built today | [`docs/multiplayer.md`](multiplayer.md) |
| **Raids and the four raid bosses** (The Bound One, The Dancer, The Bulwark, The Ashwalker) | meta system — falls under the gate rule | `docs/backlog/gameplay.ron` |
| **Store / season pass / monetization** (10 rows) | outside Roblox an open product question; **none of it gets built** | `docs/QUESTIONS.md` Q-001 |
| **Trading between players** | cheating, black markets, support load; the benefit drops because of pillar P3 | `docs/QUESTIONS.md` Q-005 |
| **Shadows** | the most expensive switch in the game. **At the end, and with a number.** | `docs/lessons/performance.md` |

## What already counts from the backlog, although it comes late

- **The Traversal Trial is not a side mode, it is the project's litmus test** (bible 2/P1): a
  player who flies elegantly through the city without killing a single titan has to have fun.
  It uses existing maps and costs almost nothing — but it can only come once there is a map.
- **At least half of all enemy kinds carry an anti-autopilot property** (bible 4). That is a
  requirement on P6, not a feature that gets added afterwards.

Related: [`docs/TODO.md`](TODO.md) · [`docs/QUESTIONS.md`](QUESTIONS.md) · [`docs/STATUS.md`](STATUS.md)
