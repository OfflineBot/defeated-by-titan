# multiplayer — the plan that is not built today

Updated: 2026-08-09 · Stage: ⬜ (none of it is built — what is built is the **seam** alone, `src/net/`)

**The netcode is not part of this commission.** No server, no prediction, no lag
compensation. **But every decision that would make multiplayer impossible or expensive later
is avoided today** — making a finished single-player game network-capable normally means
rewriting the simulation (`prompts/init.md` §6).

## The eight rules, and where they stand in the code

| # | Rule | where it already holds today |
|---|---|---|
| 1 | **Simulation and presentation are separate.** The simulation reads input + state and writes state; rendering, HUD and sound **only read**. | the simulation runs in `FixedUpdate`, `render`/`hud`/`sound` in `Update` |
| 2 | **Input is a piece of data, not a key press.** There is **one** `Intent` (movement, look, buttons, tick), and the simulation reads only that. | `shared::Intent`; filled by `net` — from the keyboard **or** a script **or**, later, the network |
| 3 | **There is no "the player".** Never `.single()`. Every player is one of many. | `shared::PlayerId`; gas and blades are **components on the player**, never a `Resource`. `LocalPlayer` is the only place that knows who "I" am |
| 4 | **Fixed simulation step** (60 Hz), the image interpolates in between. | `Time<Fixed>` at 60 Hz in `main.rs` |
| 5 | **Determinism where it is cheap.** Randomness only out of a seeded generator whose seed is part of the state. | `shared::Rng` (`seed + tick`), never `rand::random()` in the middle of a system |
| 6 | **Authority is named.** The documentation of every domain says who writes a shared field. | the authority table in [`docs/architecture.md`](architecture.md) — **later that sentence reads "the server"** |
| 7 | **Stable ids instead of pointers.** Everything that is saved or sent uses ids of our own. | `PlayerId`, `TitanId` — **never** Bevy's `Entity` (a local index with a generation; on another machine it is something else). Saves the save game while it is at it |
| 8 | **`serde` on everything that is state**, and messages designed to fit down a wire — data, no handles, no `Entity`. | `#[derive(Serialize, Deserialize)]` on all `shared/` types |

## What the bible has already decided — no open questions left

| Spec (bible 3.6) | Consequence for the code |
|---|---|
| **Own movement on the client**, everything else on the server (titans, targets, damage, loot) | The separation from rule 1 is thereby **prescribed**, not chosen: movement may react locally at once, a cortex hit never |
| **20 players per mission, 10 per raid, 40 in the hub** | Nothing scales with "one player". Twenty players with **two ropes** each plus sixty titans are the real load test — not the graphics |
| **No damage, no collision between players** (F-162a, F-163a) | Two players have to be able to pass through each other at full speed; knockback stays as a tactical element |
| **Separate loot per player** (F-160a) | Loot is never global state. Everyone rolls for himself — no race |
| **Downed instead of instant death** (F-159a), revived by teammates | "dead" is a **state with a timer**, not a removal of the entity → belongs to `squad/`. Solo players get a limited self-revive |
| **No kicking in public instances** (F-170a) | reporting and local muting, nothing else |
| **A dropped connection holds the slot for 120 s** (F-158a) | The session outlives the player; his state hangs on a `PlayerId`, not on a connection (rule 7) |
| **T-019: every movement feature is tested at 200 ms of simulated latency** | **The lag switch belongs in the tooling** (`--lag 200`), not in a later ticket: "feels good locally" is not an acceptance |

## The seam: `src/net/`

`NetPlugin` does exactly one thing today — provide the **`LocalOnly`** transport, which pushes
the local player's intents into the simulation. That way the place where client and server will
later stand is **there and empty**, instead of being cut through five domains afterwards.

```
Keyboard  ─┐
Script    ─┼─► net::Inbox ─► net::deliver_intents ─► Intent on the player ─► Simulation
(network) ─┘   (PlayerId → raw Intent)                FixedPreUpdate
```

**Three sources, one channel.** The script driver is not a second, wrong way to play — it
writes into the same inbox as the keyboard, and every system behind it is the real one. That
channel is exactly the one multiplayer needs: **one effort, two problems solved.**

## What is still open

Dedicated or host, and whether the bible's numbers stand: [`docs/QUESTIONS.md`](QUESTIONS.md)
Q-008. PvP: Q-003. None of it blocks the work — `net/` is transport-agnostic.

## The guard

**`tests/multiplayer.rs`** spawns **two** player entities and lets the simulation run a few
ticks. It falls over the second somebody writes `.single()` on a player query or puts player
state into a `Resource`. **Without it this document rots quietly** — and you notice only when
multiplayer is due, that is, after months of work you then have to touch.

Related: [`docs/architecture.md`](architecture.md) · [`docs/lessons/supervision.md`](lessons/supervision.md)
