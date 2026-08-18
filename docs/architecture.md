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
menu -> mission    # the Escape menu offers *Abandon sortie* inside a sortie and *Mission
                   #  select* outside one, and that is the same argument the `hud` line below
                   #  makes: a screen has to be right in the frame it is drawn in, so it needs
                   #  the STATE (`State<MissionPhase>`) and not an event that fired three ticks
                   #  ago. **Read-only, one predicate** (`menu::in_a_sortie`) — the buttons that
                   #  act write `shared::DeployRequest` and `shared::AbandonSortie`, and
                   #  `mission::take_orders_from_the_menu` is what sets the phase. `mission`
                   #  stays the one writer of mission state, exactly as `vector` stayed the one
                   #  writer of `Gas` when a refuel station started asking instead of writing
                   #  (FIND-063). A message will not do for the READ half — "is a sortie
                   #  running" is state, and mirroring it into shared/ would give it a second
                   #  writer for the sake of one button's label.
hud -> mission     # the objective line draws the kill counter and the verdict, and
                   #  `docs/PLAN-GAME.md` §1 makes both part of "playable": `0/3 -> 1/3`
                   #  and the word WON/LOST. Same reason as the line above and no other:
                   #  a HUD has to be right in the frame it is drawn in, so it needs the
                   #  STATE (`KillTally`, `State<MissionPhase>`), not a TitanHit that
                   #  fired three ticks ago. Read-only — `mission` stays the one writer.
hud -> menu        # the HUD is the GAME's overlay and a menu is not the game: the crosshair
                   #  ran straight down the middle of the pause column, and the objective
                   #  counter, the gas bar, the blade pips and the Q/E markers drew over all
                   #  three screens (FIND-092 §4, `docs/images/f175-pause.png`). Same argument
                   #  as the two lines above and no other: what is on screen has to be right in
                   #  the frame it is drawn in, so `hud::hide_while_a_menu_is_up` needs the
                   #  STATE (`menu::Screen`) and not an event that fired three ticks ago.
                   #  **Read-only, one comparison** — `menu` stays the one writer of `Screen`,
                   #  and this system writes only the HUD's own `Visibility`, never a
                   #  `Node.display`, which is what keeps the pixel-exact F-170/F-171 claims
                   #  true while playing. A message will not do: "is a menu up" is state, and
                   #  mirroring it into shared/ would give it a second writer for the sake of
                   #  one overlay.
titan -> mission   # a titan's LIFETIME is his sortie: `titan::spawn_titan` hangs
                   #  `DespawnOnExit(MissionPhase::Active)` on the rig root, so the bodies of a
                   #  finished sortie stop existing in the same transition as its pending waves
                   #  (`mission::open_the_field` carries the identical marker). Before it, a
                   #  sortie that ended left every titan walking — through the debrief, into the
                   #  hub, and onto the ring of the NEXT sortie (FIND-068).
                   #  It is a component, not a read: no titan system queries mission state, the
                   #  despawning is `bevy_state`'s own `despawn_entities_on_exit_state`, and
                   #  `mission` stays the one writer of the phase while `titan` stays the one
                   #  writer of titan bodies (authority table below) — which is the whole point,
                   #  a despawn living in `mission` would be the rule-4 breach.
                   #  A message will not do: `SortieEnded` would be read a tick after the
                   #  verdict at best, so a wave released ON the deciding tick outlives the
                   #  mission forever, and it would put a second lifetime mechanism beside the
                   #  one this very transition already runs. It is also the first edge that
                   #  points BACKWARDS along the plugin order below — harmless, because a
                   #  component name creates no init-order requirement, and named here so that
                   #  nobody has to rediscover it.
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
| mission state | `mission` | `hud`, `squad`, and since 2026-08-13 `menu` — **read-only**: the Escape menu offers *Abandon sortie* only inside one. The two buttons that act write `shared::DeployRequest` / `shared::AbandonSortie` and `mission::take_orders_from_the_menu` is what sets the phase, so the lobby is a front door to `hub::deploy_on_contact`'s mechanism and not a second one |
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
