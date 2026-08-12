# architecture — domains, plugin order, the allow list

Updated: 2026-08-09 · Stage: 🟨 (the structure stands and is checked by `tests/domains.rs`;
most domains are still empty plugins)

## The rule in one sentence

**One domain = one folder = one plugin = standalone.** A folder under `src/` exports exactly
one `pub struct XPlugin` with `impl Plugin`. Whatever has no plugin is `shared/` or a mistake
(`prompts/init.md` §5).

## What a domain may use

| allowed without being listed here | not allowed |
|---|---|
| `bevy::*` | calling a function of another domain |
| `crate::shared::*` — types that belong to nobody | `use crate::<other domain>` without a line in the allow list |
| `crate::data::*` — the loaded RON data | |

**Communication runs over components and messages, not over calls.** `combat` sends
`TitanHit { titan, by, zone, speed_m_s }`; `titan` reads it and decides what that means for its
body. `combat` does not know how a titan is built. **That is why every message type lives in
`shared/`** — otherwise every receiver would need an edge to its sender, and the rule would be
empty within a week.

### The allow list

`tests/domains.rs` reads **exactly this block** and goes red the moment an edge shows up in the
code that is not listed here. Format: `from -> to   # reason`. An edge without a reason is not
a decision, it is an oversight with a colon in it.

```allowed
debug -> mission   # the F3 overlay and `assert phase|kills` read the mission phase and the
                   #  kill counter. A message will not do: both are STATE, not an event —
                   #  a tool that has to show what the game is doing right now cannot be
                   #  served by a TitanHit that fired three ticks ago, and mirroring the
                   #  state into shared/ would give one field two writers.
debug -> player    # the F3 overlay prints whether the player is FLYING, and flight is not a
                   #  component: it is `player::locomotion::in_flight` over
                   #  `player::integrator::ground_top_speed_m_s` (FIND-050 — a skidding
                   #  `Grounded` player IS in flight, and the overlay used to print the
                   #  variant and lie). A message will not do — it is a predicate over the
                   #  current state, not an event — and mirroring the answer into shared/
                   #  would give the player's state a second writer for the sake of one text
                   #  line. Read-only, and the derivation stays `player`'s alone.
hud -> mission     # the objective line draws the kill counter and the verdict, and
                   #  `docs/PLAN-GAME.md` §1 makes both part of "playable": `0/3 -> 1/3`
                   #  and the word WON/LOST. Same reason as the line above and no other:
                   #  a HUD has to be right in the frame it is drawn in, so it needs the
                   #  STATE (`KillTally`, `State<MissionPhase>`), not a TitanHit that
                   #  fired three ticks ago. Read-only — `mission` stays the one writer.
```

**One line as of 2026-08-09**, and it was empty until then. `prompts/init.md` §5 names `render`
reads `world` as an example of a possible exception — that one is still not needed: `world`
spawns entities with components out of `shared/`, and `render` queries those components without
knowing `world`. Whoever enters the next edge writes down, as above, why a message would not do.

## The order in `main.rs` is the dependency order

```
data → save → net → world → render → player → vector → blades → titan →
combat → mission → progress → squad → hud → sound → menu → debug
```

`data` runs before everything else: it loads the RON and puts `GameData` down as a resource. A
missing value crashes **at startup** there, loudly and with a file name — not silently as a
zero in the middle of the game (§4).

| Folder | Plugin | what for |
|---|---|---|
| `shared/` | *(none)* | types that belong to nobody: `Intent`, `PlayerId`, `TitanId`, messages, meters, axis helpers |
| `data/` | `DataPlugin` | load RON → `GameData` + handles. Before everything else. |
| `save/` | `SavePlugin` | the save game: profile, gear budget, traits, lineage, progress |
| `net/` | `NetPlugin` | ⭐ the seam for multiplayer: today the `LocalOnly` transport, which pushes the local player's intents into the simulation (§6) |
| `world/` | `WorldPlugin` | the maps, bastion rings, houses; anchor points, collision, **spatial index** |
| `render/` | `RenderPlugin` | camera, light, sky, building meshes, loading models |
| `player/` | `PlayerPlugin` | the body: running, jumping, ground collision, state machine |
| `vector/` | `VectorPlugin` | ⭐ **the core** (Vector Gear): hooks, rope, momentum, gas, boost, wallrun |
| `blades/` | `BladesPlugin` | blades: swing, wear, breakage, swapping, resupply |
| `titan/` | `TitanPlugin` | titans: rig, limbs, cortex, AI |
| `combat/` | `CombatPlugin` | hits, damage out of speed, amputation, steam, death |
| `mission/` | `MissionPlugin` | the sortie: objectives, phases, spawn waves, win/loss |
| `progress/` | `ProgressPlugin` | XP, Mark/Sigil, gear budget, traits, lineage, ascension |
| `squad/` | `SquadPlugin` | team mates and escorts: downed, revive, marking |
| `hud/` | `HudPlugin` | gas, blade condition, target markers, crosshair |
| `sound/` | `SoundPlugin` | gas hiss, hook impact, blade cut, titan footstep |
| `menu/` | `MenuPlugin` | main menu, pause, options |
| `debug/` | `DebugPlugin` | F3 overlay, gizmos, the `--script` driver |

## Who writes what — the authority table

**One field, one writer.** Two systems on the same field are not a design, they are a coin
toss at 60 Hz — and over the network a divergence nobody can reproduce (§5 rule 4, §6 rule 6).
**Later, "the writer" simply means "the server".**

| Field / component | Writer | Readers |
|---|---|---|
| `Intent` on the player | **`net`** (`LocalOnly` transport: keyboard, script driver, later the network) | `player`, `vector`, `blades` |
| the player's `Transform` | `player` (ground, gravity) and `vector` (rope forces) — **split by state**, never at the same time; the state stands in `shared::MovementState` | everyone, read-only |
| `Gas`, `Blades` | `vector` and `blades` respectively | `hud`, `sound` |
| `Gas` — **one writer again since 2026-08-12** (the second writer stood for one day) | **`vector::gas` and nothing else.** It debits in `gas_budget` (`Intent`) and it is the only thing that ever raises a tank, in `apply_refuel_requests` (`Intent`, one tick after the request). A refuel station is `mission` furniture and **asks**: `mission::hub::refuel_at_stations` writes `shared::RefuelRequest` in `PostStep` and holds no `&mut Gas` anywhere | `hud`, `sound`. The message lives in `shared`, so no domain edge was bought for it. Red test: `tests/mission.rs::f072_a_station_asks_for_gas_and_never_writes_the_tank_itself` runs the station with **no `vector` in the app** — the shape a whole-app test cannot see. History and the one-tick cost: `FINDINGS.md` FIND-063, the violation it replaced FIND-057 §2 |
| titan body (limbs, cortex) | `titan` | `combat` reads, sends messages |
| `shared::ModelAnchors` (the empties a swapped glTF brings: `cortex`, `hook.l/r`, `hand.l/r`, `eye`, `hit.min/max`) | **`render::model`** — it is the only thing that reads a loaded scene's node tree | `titan::rig` (the `cortex` anchor overrides the position computed from `scale.ron`; **no anchor ⇒ no write at all**, so the computed value stands unchanged) |
| mission state | `mission` | `hud`, `squad` |
| save game | `save` | `progress` |

## The three separations that do not get blurred

`tools/` **builds** things (Blender, atlas, sound, checkers) — `scripts/` **plays** the game
(`--script` runs) — `assets/` holds the **result**, and `assets/extern/` alone holds anything
third-party.

## Translations out of the design bible (Roblox → Bevy)

The bible is written for Roblox in six places, because the reference is a Roblox title.
**Those places are translated, not obeyed** (`prompts/init.md` §2). Whatever else turns up
while working is added **here** — not asked back about, not ignored.

| Bible / backlog (Roblox) | here (Bevy/Rust) | found in |
|---|---|---|
| `CollectionService` tag `AnchorSurface` | a component `AnchorSurface` on the entity, tracked through the spatial index in `world/` | `F-003` |
| `RopeConstraint` | our own rope solver in `vector/` against `Time<Fixed>`, no engine constraint | `F-004` |
| `ParticleEmitter` / `Beam` | effect definitions as data under `assets/vfx/`, evaluated in `render/` | sheet `05_VFX` |
| `studs` as a length unit | **1 stud = 0.28 m**, see `docs/conventions.md` | sheets `08_Maps`, `F-002` |
| `Places` / instances | Bevy `States` + scenes; "instance" here means a server session | bible 3.6 |
| ProfileStore / session lock | `save/` with the same **requirement** (no data loss, no duplication), our own implementation | bible 6.4 |
| Future Lighting / color atlas / distance fog | Bevy PBR + `DirectionalLight` + fog; **the style stays exactly as it is** | bible 3.4 |
| Roblox store / Robux / season pass | drops out in this form — **none of it gets built**, see `docs/QUESTIONS.md` Q-001 | bible 2 P5 |
| platform moderation (no gore) | stays as a **style rule**: titans vaporize, steam instead of blood | bible 3.3 |

Related: [`docs/conventions.md`](conventions.md) · [`docs/multiplayer.md`](multiplayer.md) ·
[`docs/lessons/bevy.md`](lessons/bevy.md)
