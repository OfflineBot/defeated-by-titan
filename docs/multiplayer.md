# multiplayer — the seam, and the first thing that went through it

Updated: 2026-08-19 · Stage: 🟨 for the socket transport, 🟧 for the ground rules that have a
number behind them (see the table at the bottom)

**This document changed its nature on 2026-08-19.** For ten days it was *"the plan that is not
built today"* and every line of it was a promise about the future. The promise was kept in the
sense that mattered — the architecture really did survive contact — and it is now no longer a
plan alone: **a second player can be in this world, and his input can come from another
process.**

**What is still not built is most of it.** Read [What this is NOT](#what-this-is-not) before
using the word multiplayer about anything here. That section is the important one.

---

## The eight rules, and where they stand in the code

| # | Rule | where it holds today |
|---|---|---|
| 1 | **Simulation and presentation are separate.** The simulation reads input + state and writes state; rendering, HUD and sound **only read**. | the simulation runs in `FixedUpdate`, `render`/`hud`/`sound` in `Update` |
| 2 | **Input is a piece of data, not a key press.** There is **one** `Intent`, and the simulation reads only that. | `shared::Intent`; filled by `net` — from the keyboard, a script, **or a UDP datagram** (`net::socket`) |
| 3 | **There is no "the player".** Never `.single()`. Every player is one of many. | `shared::PlayerId`; gas and blades are **components on the player**. `LocalPlayer` is the only place that knows who "I" am |
| 4 | **Fixed simulation step** (60 Hz), the image interpolates in between. | `Time<Fixed>` at 60 Hz in `main.rs` |
| 5 | **Determinism where it is cheap.** Randomness only out of a seeded generator whose seed is part of the state. | `shared::Rng` (`seed + tick`) — ⚠️ but see the socket's own limit below |
| 6 | **Authority is named.** The documentation of every domain says who writes a shared field. | the authority table in [`docs/architecture.md`](architecture.md) |
| 7 | **Stable ids instead of pointers.** | `PlayerId`, `TitanId` — and a **seat** now outlives both the connection and the body (`net::session::Roster`, F-158a) |
| 8 | **`serde` on everything that is state**, and messages designed to fit down a wire. | `net::wire` is the cash-in: `Intent` encodes to **exactly 37 bytes** with `to_le_bytes` and no field it cannot carry |

## The seam, and what now stands in it

```text
Keyboard ─┐
Script    ─┼─► net::Inbox ─► net::deliver_intents ─► Intent on the player ─► Simulation
UDP       ─┘   (PlayerId → Intent)                   FixedPreUpdate
```

**Three sources, one channel — and the third one is real since 2026-08-19.** The line that had
to change to make a player arrive over a network is `Transport::LocalOnly` becoming
`Transport::{LocalOnly, Socket}`. **Nothing behind `deliver_intents` was touched**, no domain
grew an edge, and no system learned that a network exists. That is the whole return on ten days
of not writing `.single()`.

| file | what it is |
|---|---|
| `src/net/wire.rs` | the **frame**: 37 bytes, little-endian, one version byte. No checksum, no sequence number, no authentication |
| `src/net/session.rs` | the **roster**: who is here, and the seat that is held for 120 s after a line drops (F-158a) |
| `src/net/socket.rs` | the **transport**: a UDP port, non-blocking, capped at 256 datagrams per tick |
| `src/net/local.rs` | the keyboard, unchanged |

### How a player joins, end to end

1. A datagram arrives from an address nobody has seen. `net::socket::receive_frames` allocates
   a `PlayerId` from `IdCounter` and opens a `Seat`.
2. It writes `shared::SeatPlayer`. **`net` does not spawn bodies** — the same seam
   `mission`→`titan` uses for `SpawnTitan`, and it costs no domain edge.
3. `player::seat_players` builds the body: capsule, gas, blades, two hooks, `PlayerId` — the
   same body the local player has, minus the `LocalPlayer` marker.
4. Every further frame from that address goes into the `Inbox` under **the seat's** id.

⚠️ **The `PlayerId` inside the datagram is thrown away.** The chair belongs to the address the
packet came from, so a peer cannot send an intent in somebody else's name. That is the only
security property in this transport and it is one line.

<a id="what-this-is-not"></a>
## What this is NOT

- **Nothing is sent back.** No snapshots, no state replication, no interpolation, no
  reconciliation. A peer drives a body in the host's world and **cannot see it**. Two copies of
  this game do not make a co-op session; a sender and a host make *one* world with two players
  in it.
- **There is therefore no client.** The thing on the other end today is a script that writes 37
  bytes into a socket (see below). Writing a real client means replicating the world, which is
  the part that is not built.
- **No reliability, no ordering, no handshake, no NAT traversal, no encryption.** UDP delivers
  what it delivers. That is survivable *because* an `Intent` is absolute and idempotent — a lost
  frame costs one tick and the next repairs it, which is exactly why the aim spread travels as
  an angle and not as a wheel notch.
- 🔴 **A run over the socket is not reproducible.** The simulation is in `FixedUpdate`, ids are
  stable and the wire carries `Intent` — every *ingredient* of determinism is present — but a
  datagram is read on whichever tick it happens to arrive on and nothing delays it to a fixed
  one. **`--lag 200` is deterministic; the socket is not.** Whoever builds lockstep or rollback
  on top of this starts by putting the frame's own `tick` back to work: it is on the wire, it is
  a `u64`, and today only the receiver's clock is used.
- **No pause over a network.** `menu::Screen::Paused` stops `Time<Virtual>`, i.e. the whole
  simulation. One machine may not do that to a session; the note is on the enum variant.

## The four ground rules (bible 3.6) — where each one stands

| Rule | Status | Where |
|---|---|---|
| **No collision between players** (F-163a) | 🟧 — measured: two bodies 0.1 m apart shoved each other **0.194 m each per second** before the fix, 0.000 m after. ⚠️ It cost the aim ray, see B-010 below | `shared::PLAYER_COLLIDES_WITH`, attached in `player::spawn_player`. `tests/multiplayer.rs::f163a_two_players_in_the_same_spot_do_not_push_each_other` |
| **No damage between players** (F-162a) | 🟧 — it held before anybody wrote it down, and now it is guarded: `blades::cut::sweep` casts against `LAYER_TITAN_CORTEX` and `LAYER_TITAN_BODY` only, and a player is a member of neither | `tests/multiplayer.rs::f162a_a_player_is_not_a_member_of_any_mask_a_blade_cuts` |
| **Separate loot per player** (F-160a) | ⬜ — **there is no loot.** `mission::run::KillTally` credits a titan to exactly one player and never twice, which is kill *credit* and not loot. Nothing global exists to make separate yet | `src/mission/run.rs` |
| **No kicking in public instances** (F-170a) | 🟧 by construction — the lobby has no kick row and `net` exposes no way to drop a named seat. The only thing that frees a chair is silence plus 120 s | `net::session::Roster`, `src/menu/lobby.rs` |

## The lobby

`Screen::Lobby` shows the squad — every seat, in id order, with `you` marked and a disconnected
peer greyed — and one row that opens or closes the port.

**There is no *Join* row and no address field**, deliberately. Joining means seeing the world
you joined, and the world is not replicated; a text field that took an address and then showed
nothing would be the exact thing this project has a rule against
(`src/menu/title.rs`: *do not add a row nothing can spawn*).

## How to see two players today

`tools/peer.py` is the other end. **It is not a client** — it sends input and receives nothing,
for the reason in the section above.

```bash
./target/debug/defeated_by_titan --host --headless --ticks 900 &
python3 tools/peer.py --forward 8                    # he holds W for eight seconds
python3 tools/peer.py --move-x 1 --seconds 8 &       # and a second one strafes
```

The host prints where everybody is, once per second, while the door is open:

```
net: tick 600 player 1 [you]              at  0.0 0.0   0.0
net: tick 600 player 2 [127.0.0.1:35480]  at  3.0 0.0 -57.7
net: tick 600 player 3 [127.0.0.1:52130]  at 42.8 0.0   0.0
```

The player id inside the datagram is a claim and is ignored — a peer gets the seat his address
owns. `game.ron: net.port` is the default port; `--port` moves it, for the flag and for the
lobby row alike.

⚠️ **Nobody can join while the lobby is open.** A screen that is not `Playing` stops
`Time<Virtual>`, so `FixedUpdate` — and with it the socket read — does not run
(`menu::apply_screen`). The port stays bound and the datagrams wait in the kernel buffer; they
are read the moment the game is running again.

## What is still open

- Dedicated or host: [`QUESTIONS.md`](QUESTIONS.md) Q-008. PvP: Q-003. The aim assist across
  machines: **Q-038, answered 2026-08-19** — see there.
- ~~A rope can still be aimed at another player's collider.~~ **Answered 2026-08-19, and it was
  indeed the next thing that was wrong** (`docs/BUGS.md` B-010): two players who no longer shove
  each other stand *inside* each other, so an aim ray started in a team mate's capsule and
  `solid: true` returned the caster's own eye at distance 0. **A player's body is now air to a
  hook ray** — `shared::AIM_RAY_SEES` (`ALL & !LAYER_PLAYER`) on `vector::aim::cast` and on
  `vector::hook::anchorable_beyond_reach`. Whether he blocks a blade, a camera or anything else
  is *not* decided by that.
- A peer who goes quiet keeps his last `Intent`, by design (`Inbox::last` — a player without a
  new message must not stop dead). A player who disconnects mid-sprint therefore runs on for
  120 s. Nobody has decided whether that is right.

## The guard

**`tests/multiplayer.rs`** spawns two players, runs the simulation, and falls over the second
somebody writes `.single()` on a player query or puts player state into a `Resource`. Since
2026-08-19 it also drives a second player **through a real UDP socket** — a test that reached
the second player through the same local function as the first would prove nothing about a wire
(`docs/FINDINGS.md` FIND-103).

Related: [`architecture.md`](architecture.md) · [`lessons/supervision.md`](lessons/supervision.md)
