# QUESTIONS — decisions that are not mine to make

Updated: 2026-08-09

**Nothing here interrupts anybody.** Every question gets an `ASSUMPTION:` that the work
carries on under until the answer arrives — and the work runs past it instead of waiting
(`prompts/init.md` §2, §10). Answered questions move to the bottom, they are not deleted.

---


> 📦 **Answered, resolved and superseded questions live in**
> **[`archive/QUESTIONS-answered.md`](archive/QUESTIONS-answered.md)**, with a one-line index.
> They moved on 2026-08-29 because this file had reached **222 kB** and could no longer answer
> the question it exists for: *what is still his to decide?* **Nothing was deleted.**

---

## Q-001 — Is there a store at all outside Roblox?

**Context:** the bible's fifth design pillar is „Der Store verkauft nur Aussehen" (*the store
sells looks only*: cosmetics, private servers, season pass), and `01_Spielfunktionen` contains
**5 rows of `Monetarisierung`** and **5 rows of `Live Ops`**. `prompts/init.md` §2 says about
it: Robux, season pass and the Roblox store **fall away in that form**, the *principle*
stands, and whether any of it happens outside Roblox at all is a product question.

**ASSUMPTION:** none of it gets built. The ten rows stay ⬜ in `docs/STATUS.md` and do **not**
disappear from `docs/features.ron` — they are recorded, not struck.

## Q-003 — PvP yes or no?

**Context:** bible 8.1. The game is specified as pure co-op. PvP would be a separate balancing
line, server-authoritative hit checking and permanent maintenance — „kein Feature, sondern ein
zweites Projekt" (*not a feature, a second project*). The bible says it itself: **decide now,
not in month 12.**

**ASSUMPTION:** pure co-op. `squad/` builds on "no damage, no collision between players".

## Q-004 — Vessel Forms in v1.0 or v1.5?

**Context:** bible 8.2 — the single most expensive item (its own rigs, ~60 animations, its own
balancing), replaces the core movement instead of extending it. The bible's own proposal: plan
for v1.5, prepare technically.

**ASSUMPTION:** v1.5. The 9 `Vessel Form` rows stand in `docs/ROADMAP.md`, not in the build
plan.

## Q-005 — Trading between players yes or no?

**Context:** bible 8.3. Binding against cheating, black markets, support load. With pillar P3
(no progress without a guarantee) the benefit drops considerably.

**ASSUMPTION:** no. Nothing in the code presupposes trading.

## Q-007 — Which license does the public repo get?

**Context:** `prompts/init.md` §18 step 2 demands a `LICENSE` before publication — and
explicitly: **do not invent a license file** if the user has said nothing.

**ASSUMPTION:** do not create a `LICENSE`. The repo goes online without a license file
(= all rights reserved) until the user chooses.

## Q-008 — Dedicated server or host, and exactly how many players?

**Context:** `prompts/init.md` §6 asks the question explicitly; bible 3.6 already fixes the
**numbers** (20 per deployment, 10 per raid, 40 in the hub) and the **authority model** (own
movement on the client, everything else on the server), but not the way it is operated.

**ASSUMPTION:** the bible's numbers hold. `src/net/` stays transport-agnostic: `LocalOnly`
today, and nothing in the code decides whether the later server is dedicated or a host.
Details in [`docs/multiplayer.md`](multiplayer.md).

## Q-010 — The anchor density needs a number

**Context:** `prompts/init.md` §2 calls the anchor density „die wichtigste Zahl" (*the most
important number*) in `08_Maps` — but what stands there in the backlog is **`Hoch` / `Mittel`
/ `Niedrig` / `Sehr hoch`**, so no number at all. Bible 6.2 makes the anchor density the gate
for P3. Qualitatively it can neither be tuned nor checked.

**ASSUMPTION:** anchor density is defined as **anchorable surfaces per 1000 m²**, the four
steps get calibrated against measured traversal times while Ashgate District is being built,
and the number lands in `assets/data/maps.ron`. The link to the qualitative column stays in
`docs/backlog/maps.ron`.

## Q-011 — What happens to the 24 `Could` rows if the schedule gets tight?

**Context:** bible 6.4 and `prompts/init.md` §2: „Bei Terminkonflikt fallen zuerst alle
`Could`" (*if the schedule conflicts, all `Could`s go first*) — not a recommendation. But
there is no deadline in this commission.

**ASSUMPTION:** no deadline, so nothing falls. The MoSCoW order only determines the **order**
in `docs/TODO.md`: 139 Must before 81 Should before 25 Could.

## Q-012 — What does `avian3d` mean for later rollback in multiplayer?

**Context:** the user decided on 2026-08-09: **`avian3d` is used.** Before that, a home-built
solution out of axis-aligned boxes was planned. The decision has been made and is not up for
debate — what is open is its **consequence**: `docs/multiplayer.md` demands an architecture
that does not make later rollback expensive. A third-party physics engine, however, holds
state we do not write ourselves (contact caches, warm-start impulses, sleep states) — and how
much of that has to be captured for a snapshot decides whether rollback later costs a week or
a month.

`avian3d 0.7.0` requires exactly `bevy 0.19.0` (checked source:
`~/.cargo/registry/src/*/avian3d-0.7.0/Cargo.toml`) and brings a feature
`enhanced-determinism` with it that switches on `libm`.

**ASSUMPTION:** `avian3d 0.7.0` with `enhanced-determinism`. The physics state is treated as
**restorable** until the opposite is measured; the simulation keeps running in `FixedUpdate`,
and input stays an `Intent` (§6 rule 2).

**To roll back:** the line in `Cargo.toml`, the authority table in `docs/architecture.md`
(avian writes `Transform`/`Position` and `LinearVelocity` itself), and every place that uses
an avian type instead of one of our own. The domain structure, the `Intent` channel and the
RON values stay untouched by it — that was the purpose of the cut.

## Q-013 — How long may a rope be at most?

**Context:** evidenced in this session by an exhaustive search: **no source names a maximum
rope length.** Not the design bible, not `docs/features.ron` (F-001, F-004, F-005), not
`assets/data/game.ron`. There is only `vector.min_rope_m` (3.0 m) and `vector.hook_range_m`
(**90 m** — the range of the *hook*, not of the *rope*). Without an upper bound F-004 is not
fully specified: it decides whether you can still swing 200 m while hanging from a tower.

**ASSUMPTION:** the rope length is the **distance at the moment of anchoring**, capped at
`vector.hook_range_m` (90 m). After that it is only ever **shortened** (F-005), never
lengthened — unless the collision pushes the player out, in which case it is pulled after him
and the hook releases on overextension.

**To roll back:** a single new RON value (`vector.max_rope_m`) and the place that sets the
length when anchoring. No structural break.

*Addendum 2026-08-09:* **112 m** stood here twice. Since the user's size table the number is
90 m (Q-002); Q-018 and Q-019 were brought along, this question was not. Corrected. In
practice it means: the cap is 20% lower, and in the flat city it does not matter anyway —
what is usable there are ropes of 3 to 7 m (Q-022).

## Q-014 — How big is a grid cell, and is the grid three-dimensional?

**Context:** `assets/data/game.ron: world.cell_m` stands at 8.0 m and is **unmeasured** —
`docs/lessons/performance.md` says it literally: "§11 says 'grid cells', not how big. Has to
be measured." Until 2026-08-09 `game.ron` pointed at **Q-013** for this number, and Q-013 is
the question about the maximum rope length. So the question had a reference, but no question.

What changed with the size table:

| Quantity | Value | at `cell_m` 8.0 |
|---|---|---|
| street width | 7.0 m | **no** cell contains street only |
| `house_large` | 11.5 m | occupies 32 cells — below `large_body_cells` (64), so it goes into the grid ✓ |
| wall | 120 × 45 m | 4500 cells ⇒ large-body list ✓ |
| Ashwalker | 150 m | 475 cells ⇒ large-body list ✓ |
| 90 m ray | | crosses 11.2 cells horizontally, 15 vertically at the wall |

The second half of the question, never asked before: **`world.half_extent_m` (300 m) was only
calculated against the plane** (400/2 + 90 = 290). In height, wall (120 m) and Ashwalker
(150 m) stand on top of each other — 270 m, so 30 m of margin. Whether the grid has a Y axis
at all is open: `src/world/index.rs` is an empty shell.

**ASSUMPTION:** `cell_m` stays at 8.0 m and `half_extent_m` at 300 m until `world/index`
stands and can be measured. `tests/data.rs::t005_the_grid_carries_the_worlds_height_too`
records that the extent covers the 270 m — independently of whether the grid uses it today. A
grid that is smaller than its world is wrong either way.

**To roll back:** two numbers in `assets/data/game.ron: world`. No code.

## Q-015 — How thin may a wall be?

**Context:** `assets/data/game.ron: world.min_wall_m` (0.5 m) calibrates
`player.max_substep_m` and with it the whole tunneling guard (F-012): the integrator may
never travel further per substep than the thinnest wall. The number stands in **no source** —
`docs/backlog/models.ron` A-080 names a 4-unit grid, but that is the module width of a
building part, not its thickness. `game.ron:109` has pointed at this question since the number
was entered; **written it never was.**

Recalculated with today's values: `max_speed_m_s` 75 ⇒ 1.25 m per 60 Hz tick ⇒ at
`max_substep_m` 0.25 that is five substeps of 0.25 m each, and `min_wall_m` is exactly twice
that. Cleanly calibrated — but on an invented number.

**ASSUMPTION:** 0.5 m. Any wall thinner than that is a level error and not a physics case; the
guard `t005_a_substep_is_smaller_than_the_thinnest_wall` records the relation, whatever number
the user names.

**To roll back:** `world.min_wall_m` and, if the wall gets thinner, `player.max_substep_m` in
the same ratio. No code — that is the purpose of the calibration.

## Q-016 — How fast does a missed shot come back?

**Context:** `F-001` names "retracting" as the fourth hook state; `assets/data/game.ron` knew
only the outbound flight until then. `vector.hook_retract_speed_m_s` stands at 120.0 m/s —
faster than the outbound flight (90.0 m/s), so that a miss is not punished twice. **A pure
starting value, no source**; `game.ron:51` has pointed at this question since the number was
entered, written it never was.

**ASSUMPTION:** 120.0 m/s, i.e. outbound flight × 1.33. The reason is game feel, not physics:
the punishment for a miss is the time lost *up to* the miss, not after it.

**To roll back:** one number in `assets/data/game.ron: vector`. No code.

## Q-017 — Who gets paid first when the gas tank is not enough for both?

**Context:** `assets/data/game.ron: vector.gas_priority` stands at `[Boost, ReelIn]` — so on a
tight tank the boost gets its gas first and the reel-in falls away. That is a **game-value
decision** and therefore a RON value and not an `if` in `src/vector/gas.rs` (`gas.rs:14` and
`game.ron:64` both point here; written the question never was).

The reverse is seriously defensible: whoever can still reel in at 3% tank reaches a wall and
survives; whoever can only boost flies faster into nothing. `[ReelIn, Boost]` would be the
forgiving order, `[Boost, ReelIn]` the more expressive one.

**ASSUMPTION:** `[Boost, ReelIn]`. Reason: the boost is the action the player triggers
explicitly at that moment, the reel-in often runs alongside — and an input that is silently
not carried out is the worse experience.

**To roll back:** the order in one line of `assets/data/game.ron`. The code reads the list, it
knows no order.

## Q-018 — „Geschwindigkeit x1,5 vs. Standard": which standard?

**Context:** the user's size table (2026-08-09) names, under *camera / Vector Gear*, a
**„Geschwindigkeit x1,5 vs. Standard"** (*speed ×1.5 vs. standard*). The factor is given, but
the **reference quantity is missing** — and without it a factor is not a number. What stands
in `assets/data/game.ron`:

| Candidate | Value | ×1.5 would be | plausible? |
|---|---|---|---|
| `player.run_speed_m_s` | 6.0 m/s | 9.0 m/s | no — a Vector Gear that is barely faster than running is not a Vector Gear |
| `vector.max_speed_m_s` | 75.0 m/s | 112.5 m/s | possible, but 112.5 m/s is over 400 km/h |
| `vector.hook_speed_m_s` | 90.0 m/s | 135.0 m/s | rather not — that is the flight time of the hook, not a player speed |
| a *reference* speed that stands nowhere | ? | ? | most likely — and then we are missing the reference number |

The factor is recorded and lives in `assets/data/scale.ron: vector.speed_factor`. It is
**used in no calculation anywhere**.

**ASSUMPTION:** `vector.max_speed_m_s` stays **unchanged at 75.0 m/s**. The factor 1.5 is
stored but not applied — putting a number with an unknown reference into a formula produces a
speed nobody has justified, and that only shows up in the blind test, once everything else has
been tuned on top of it.

**To roll back:** exactly one line — `vector.max_speed_m_s` in `assets/data/game.ron`.
`scale.ron: vector.speed_factor` stays either way, because it holds the user's own figure. No
code is affected.

### Addendum 2026-08-09 — the city says something other than the number

Calculated against the sizes of the table (the rope solver from `src/shared/rope.rs` rebuilt
and checked against four of its own assertions):

| Source of the speed | Result |
|---|---|
| pure swinging from an 11.5 m roof | **17–21 m/s** (energy check: `sqrt(2·20·11.5)` = 21.45 m/s) |
| roof to roof across a 7 m street, 6 m/s run-up | 17.09 m/s — and **12.64 m/s if you reel in** |
| full reel-in along a clear 30 m line of sight | **75 m/s after 0.97 s** for 5.8 of 100 gas |
| what the titans' windups leave readable | **6–20 m/s** |

What 75 m/s means in this city: the 400 m graybox in **5.33 s**, a 28 m block in 373 ms, and a
7 m street as a curve at **164 g** and 1228 degrees/s of turn rate — with a camera half-life
of 0.05 s (three ticks). The tick itself holds up cleanly: 1.25 m per tick, five substeps, no
tunneling.

From that follows a statement that **stands in no file**: the table has quietly made
**swinging the combat speed and reeling in the travel speed**. That is a coherent design — but
`max_speed_m_s: 75.0` reads like the opposite. Two further observations on the factor:
75 / 1.5 = **50.0 m/s**, and **no RON file contains a 50**. There is no reference in the data
from which 75 emerges as "×1.5".

**The ASSUMPTION stays unchanged** — 75.0 m/s is a clamp against fling exploits (bible 6.4),
not a target speed, and a number with an unknown reference does not get touched. The addendum
stands here so that when the calibration happens, it is on the table **what** is being
calibrated against.

## Q-019 — Does the cortex grow with the titan?

**Context:** the size table made the titans considerably larger (`assets/data/titan.ron`,
2026-08-09): the Bellower went from 10 m to 21 m, the Warden from 8 m to 14 m, the Husk from
5 m to 10 m. **`cortex_radius_m` stayed unchanged in the process** — 0.40 to 0.70 m, because
the user said nothing about it and I do not invent game values.

That tips a ratio that used to hold: the Bellower's cortex was 0.70 m on a 10 m body (7% of
the height), now it is 0.70 m on 21 m (3.3%). Bible and `F-030` demand **„Cortex aus 100 m
erkennbar"** (*cortex recognizable from 100 m*) — at 100 m a 0.7 m sphere is about 0.4 degrees
wide, roughly a fingernail at arm's length. The user's head-size rule (1/9 to 1/10 of the
height) suggests that the hit zone **grows with it**: a 21 m titan has a head of a good 2 m.

**ASSUMPTION:** `cortex_radius_m` stays **as it is** for now, absolute and per kind. Reason: it
is a pure balancing value (how easy is it to hit?), not a scale value (how big is the thing?),
and it will be calibrated in the P1 blind test anyway. It stands here so that the question is
**on the table** during that calibration instead of being a surprise.

**To roll back:** eight numbers in `assets/data/titan.ron`. Alternatively — and that would be
the consistent route if the user says "grow with it" — a `cortex_radius_fraction` in
`assets/data/scale.ron: titan`, out of which `src/data/mod.rs` computes the radius. Then
`cortex_radius_m` per kind falls away entirely.

### Addendum 2026-08-09 — two values were not too small, they were impossible

The user's head rule (1/9 to 1/10 of the height) now stands as a number in `scale.ron`, and
with it something became measurable that nobody could see before: **on two kinds the cortex
was bigger than the whole head.**

| Kind | Class | Head height (1/9) | Cortex diameter | Ratio |
|---|---|---|---|---|
| `scuttler` | small, 4.2 m | 0.47 m | **0.80 m** | 171% |
| `weaver` | small, 4.2 m | 0.47 m | **0.90 m** | 193% |

That is not a balancing question but a geometry question, and it is **older than the size
table** (a 3.5 m body with a 0.80 m cortex). Both values are corrected to a radius of 0.20 and
0.23 m respectively, `tests/data.rs::t005_the_cortex_fits_under_the_titans_head` holds the
upper bound. **The actual question stays open:** the ratio of cortex to body has halved right
across all kinds.

Two numbers that defuse the question and therefore belong here:

1. **The narrower field of view has nearly doubled the cortex.** At 1920 × 1080 and 60 degrees
   instead of 90, the Husk cortex (1.10 m) is **10.3 px** wide at 100 m instead of 5.9 px, at
   50 m 20.6 px, at 28 m 36.7 px. Visible yes — aimable only from about 4 px, which holds out
   to 257 m.
2. **`F-030` does not demand 100 m at all.** The acceptance sentence reads literally „Cortex
   ist aus 100 Backlog-Einheiten Entfernung erkennbar" (*cortex is recognizable from 100
   backlog units away*) — that is **28.0 m** (factor 0.28) or 22.5 m (the factor implied by
   90/400). At 28 m the requirement is overfulfilled by a factor of 3.5. Which of the two
   readings holds is decided along with this question.

## Q-020 — „Abnormaler / Boss, 28 m": size class or the Errant?

**Context:** the user's size table names as its largest titan row **„Abnormaler / Boss | 28 m
| Nacken 24,9 m"**. In the project vocabulary, however, "Abnormal" is not a size word but a
**kind**: `docs/conventions.md` §2, `docs/backlog/naming.ron:24` and `tools/norms.py` (the
FORBIDDEN list) all three translate it bindingly to **Errant**.

The row therefore has two readings:

1. **Size class.** The user is describing a size and uses "abnormal/boss" as a label for "the
   biggest thing there is". Then 28 m is a class without a kind.
2. **A statement about the kind.** The user is saying the Errant is 28 m tall. Then
   `assets/data/titan.ron` is wrong: it has `errant` at `medium` (10 m).

The difference is no formality — it is a factor of 2.8 on an enemy the player meets in the
second minute of play.

**ASSUMPTION:** reading 1, the size class. Reasons: the row stands in a **size** table between
four other size rows; the user writes „Abnormaler / **Boss**", and "boss" is not a kind in any
source; and the four other rows („Kleiner Titan", „Mittlerer Titan") are sizes too, not kinds.
`scale.ron` carries the class `boss` (28 m), no kind occupies it, and `errant` stays at 10 m.

**To roll back:** one line in `assets/data/titan.ron` (`errant` from `medium` to `boss`) — no
more than that, because no height is maintained per kind. That is exactly what the classes are
for.

## Q-021 — Are the 55–65 degrees of field of view meant horizontally or vertically?

**Context:** the user names „FOV Bodenkampf 55-65 Grad" (*FOV for ground combat 55–65
degrees*) and calls it **„groesster Hebel"** (*the biggest lever*) — it is the number he
himself credits with the largest effect. What happens to it, though, nobody decided; it
follows from Bevy: `src/render/mod.rs:86` puts `fov_deg` into `PerspectiveProjection.fov`, and
Bevy documents that field literally as *"The vertical field of view (FOV) in radians"*
(`bevy_camera-0.19.0/src/projection.rs:284-287`).

| 16:9 | vertical | horizontal |
|---|---|---|
| old value | 90 degrees | 121.3 degrees |
| **today** | **60 degrees** | **91.5 degrees** |
| the user's window | 55 / 65 | 85.6 / 97.1 |

If the figure were meant **horizontally** — the usual reading when somebody says "FOV" —
`game.ron` would have to hold **32.6 to 39.4**, not 55–65. The two readings differ by a factor
of **1.7** in effective image width.

What the switch from 90 to 60 degrees vertically has already brought is measured and goes in
the direction the user wanted: the focal length rises from 540 to 935 px, everything moves
**73% faster across the screen** at the same real speed, a large titan at 60 m fills 30% of
the image height instead of 18%, and the cortex at 100 m is nearly twice as wide (Q-019).

**ASSUMPTION:** vertical, the way Bevy reads the field. Reason: it is the reading that works
**without a conversion**, and it fulfills the intention ("biggest lever", a tighter image)
audibly better than 32.6 degrees, which would be a telescope. `docs/conventions.md` §1 norms
axes, units and angles — but **no FOV convention**; it belongs there and is missing.

**To roll back:** two numbers in `assets/data/scale.ron: camera` and one in
`assets/data/game.ron: camera.fov_deg`. No code — `src/render/` only reads.

## Q-022 — The flat city carries no rope above 14.5 m. What holds?

**Context:** two of the user's specifications meet, and nobody had done the arithmetic.
Residential buildings 4.5–11.5 m, anchor range 90 m, plus `vector.min_rope_m` 3.0 m. From that
follows an **anchor ceiling**: 11.5 + 3.0 = **14.50 m** is the highest point a rope can hold
on a grid house. Above it lies:

| Class | Cortex | above the ceiling |
|---|---|---|
| `large` (14 m) | 12.5 m | −2.0 m (just fits) |
| `huge` (21 m) | 18.7 m | **+4.2 m** |
| `boss` (28 m) | 24.9 m | **+10.4 m** |

**Every approach to a titan of 14 m or more would therefore be ballistic:** let go, fly, one
pass without a correction, and whoever misses falls with nothing to hook. It shows up as "the
Vector Gear feels wrong against big titans", and the blame falls on the rope solver, the boost
and the camera — because the cause is a house height and a rope minimum in two other files.

Two further consequences of the same geometry, calculated:

- **87% of the 90 m hook range produce no swing at all on a grid house.** A clean arc needs a
  rope length ≤ the anchor height; above 11.5 m there is none. What is usable are ropes of
  **3 to 7 m** — one street width.
- **The swing is slower than the fall.** Free fall from 11.5 m takes 1.07 s, a quarter period
  of an 11.5 m rope 1.19–1.41 s. You are down before you have swung out.

**The user's numbers are not touched.** What changes is the **composition of the city** — and
that is the route `assets/data/maps.ron` had prescribed for itself anyway ("church, watchtower
and wall are placed as `blocks`") without taking it: until 2026-08-09 none of those buildings
stood in any map.

**ASSUMPTION:** the vertical gets built, the specification does not get lowered. The graybox
now carries a church (35 m), a watchtower (12 m) and a tree (12 m) as `blocks` with
`landmark: true`; the anchor ceiling rises with them from 14.5 m to **38 m** and covers all
five size classes. `tests/data.rs` checks both: that the scale knows a structure above the
cortex of every class, and that the start map really has placed an anchorable landmark.

**The design question stays open regardless:** should a large titan be attackable out of the
residential buildings at all, or is "go find yourself a tower" the intended answer? That is
the user's call, not the arithmetic's.

**To roll back:** the three `landmark` blocks in `assets/data/maps.ron` and the two guards. No
number of the user's, no code.

## Q-023 — Is the wall flank anchorable?

**Context:** `scale.ron: wall.platform_height_m` (60 m) is justified by the crown (120 m) not
being reachable in one move with 90 m of range. Recalculated with the batter ((45 − 28)/2 =
8.5 m of setback, 4.05 degrees) that holds:

| Move | Distance | within 90 m? |
|---|---|---|
| ground → platform | 60.15 m | ✓ |
| platform → crown | 60.15 m | ✓ |
| ground → crown directly | 120.30 m | ✗ |

**But that is not the reason for the platform.** The highest point of the wall flank that lies
within 90 m of the wall foot is **y = 89.78 m** — with two free moves (90 m + 30 m) the crown
would be reachable without a platform too. The platform only carries weight once the **flank
itself is not an anchor surface** and only platform and crown take a hook. That decision
stands nowhere.

It is not a small one: an anchorable flank turns the wall into a 120 m climbing wall with a
free choice of route, an untagged flank turns it into a structure with **two** entrances — and
only then is the ascent a question of skill. The ascent itself is measured as comfortable:
60 m of reeling in at 28 m/s takes 2.14 s and costs 12.9 of 100 gas, the whole wall via the
platform 25.7 gas.

**ASSUMPTION:** the flank is **not** anchorable, platform and crown are. Reason: only that way
is the number 60 m, which the user gave, effective at all — otherwise it would be decoration,
and treating a specification as decoration is the worse of the two assumptions.

**To roll back:** an `anchorable` field on the wall blocks, as soon as the wall exists as a
`blocks` entry. Today it stands in no map, so the assumption costs nothing.

## Q-025 — A 28 m titan does not fit through a 7 m alley. Both numbers are yours.

**Context:** Two of your specifications meet and the result had not been computed. The size
table gives `boss` a height of 28 m; `maps.ron` gives the city an alley width of 7.0 m (you
set it from "Strassenbreite 6-8 m"). Nothing in the whole repository says how **wide** a titan
is — there is no body width in `titan.ron`, no collider, no footprint. On the common estimate
of 0.25 x height, a 28 m titan is exactly **7.0 m wide**: he fills the alley from wall to wall.
What happens then is not "it looks tight", it is a physics failure — two opposing contact
normals cancel each other, the titan stops dead at the alley mouth, and if he starts out
overlapping at all, avian's `penetration_rejection_threshold` (default 0.5, and that is in
**metres**) discards the contacts and he stands inside the house.

It never happens today: `boss` has no representative and the largest spawnable class is
`large` at 21 m — still 5.25 m wide on the same estimate, which fits an alley but not a
doorway. The question becomes load-bearing the moment a big titan spawns in the city.

**ASSUMPTION:** Big titans do not spawn inside the block grid. The `husk` (10 m, ~2.5 m wide)
is the only kind the first mission uses, and it fits everywhere. The alley stays at 7.0 m
because you set it; the size classes stay because you set them.

**What would have to be rolled back:** nothing yet — no code depends on it. The moment a
titan wider than an alley exists, one of three has to give, and **which one is yours to
choose**: the alley gets wider, the big classes stay out of the city (they fight on the wall
or in the open), or the city grows a main street that the grid keeps clear.

## Q-026 — Is the cortex readable from 100 metres or from 100 studs?

**Context:** Two sources disagree by a factor of 3.6, and this decides how big the only lethal
hit zone in the game has to be. `docs/features.ron` F-030 says the cortex must be recognisable
from **100 studs** — at the project factor of 0.28 m/stud that is 28.0 m. The design bible
(`Design-Bibel.md:45`) says **100 metres**.

The number is not cosmetic. At 60 degrees vertical field of view on 1080 px, a 1.10 m cortex
is **10.3 px** across at 100 m — a smudge. At 28 m it is 37 px, which is a target you can aim
at. Everything about the combat design hangs on which one is meant: the cortex radius, whether
the approach has to be a diving pass or can be a considered swing, and whether the hit zone
needs a colour marker to be findable at all.

**ASSUMPTION:** 100 **metres**, from the bible — it is the design document and the more
demanding of the two. That does **not** mean the cortex has to be 10 px of grey: the marker
carries the readability, not the geometry. `vfx.ron` VFX-011 already specifies an amber,
unlit, emissive marker as a Must — that is the thing you see from 100 m, and the geometry
underneath stays honest at its real size.

**What would have to be rolled back:** `cortex_radius_m` in `titan.ron` (already disputed as
Q-019) and the marker size. No structure, no code — the numbers live in RON.

## Q-027 — A titan has no health, and no width, and cannot turn

**Context:** Not a decision so much as three holes found while planning the titan round, all
in the same place: `assets/data/titan.ron` describes eight kinds with height, speed, cortex
radius, regeneration and wind-up — and nothing else. Concretely missing, all **blocking** for
a titan that can actually fight:

- **health** — there is no such value anywhere in the repository. `regen_per_s` regenerates a
  quantity that does not exist. A cortex hit kills outright (that is the design), but every
  other hit zone (F-032) needs something to reduce.
- **body width** — see Q-025. Without it there is no collider and no answer to the alley.
- **`turn_deg_per_s`** — without a turn rate the husk's entire purpose ("learning the approach
  angle") does not exist, because he can always face you instantly.
- **`strike_s` and `recover_s`** — the wind-up is guarded (`>= 0.4 s`, `tests/data.rs`), but
  the recovery **is** the punish window, and it is unspecified.

**ASSUMPTION:** I invent starting values, mark them `UNTUNED` in the file the way `game.ron`
does, and guard each one in `tests/data.rs`. They are placeholders for your judgement, not
design decisions — the first playtest will move all of them.

**What would have to be rolled back:** only numbers in `titan.ron`. Nothing structural.

## Q-028 — May a titan bigger than 14 m spawn this session? I capped it at 14 m without you.

**Context:** Q-025 records that your two numbers do not fit together — a 28 m titan in a 7.0 m
street. Until 2026-08-09 that was theoretical, because nothing said how wide a titan is. Now
`scale.ron: titan.width_fraction` exists (0.25, invented by me), so the arithmetic can be done,
and it comes out like this:

| class | height | width at 0.25 | clearance per side in a 7.0 m street |
|---|---|---|---|
| `small` | 4.2 m | 1.05 m | 2.98 m |
| `medium` | 10.0 m | 2.50 m | 2.25 m |
| `large` | 14.0 m | 3.50 m | **1.75 m** |
| `huge` | 21.0 m | 5.25 m | 0.88 m |
| `boss` | 28.0 m | **7.00 m** | **0.00 m** |

A `boss` does not squeeze through the alley; he **is** the alley. He would stand in the mouth of
it as a silent wall, and the failure would read as a pathfinding bug, not as a size decision —
which is the expensive kind of wrong.

**The decision is yours** (it is your 28 m and your 7 m), and I made it anyway rather than stop:

**ASSUMPTION:** `scale.ron: titan.max_spawnable_class: "large"`. Nothing above 14 m spawns this
session. `huge` and `boss` stay in `classes` — they are recorded, not struck — and the bellower
(`huge`) therefore cannot be spawned, which costs nothing today because only the husk is built.

**What would have to be rolled back:** **one line** in `assets/data/scale.ron`
(`max_spawnable_class`), plus whatever `F-052` navigation then has to learn about a body wider
than the street. No code, no structure. This is deliberately the cheapest reversal in the file.

**The alternative you might actually want** is the other direction: widen the streets. That is
`maps.ron: layout.street_m`, also one line — but it is a number you gave (6–8 m, "keep them
tight"), and widening it changes every hook distance in the city. That is why I did not.

## Q-029 — Twenty-six numbers in `titan.ron`, `gear.ron` and `scale.ron` are now mine, not yours

**Context:** to build the titan and the cut at all, the values listed in Q-027 had to exist.
On 2026-08-09 I wrote them: nine per-kind combat numbers in `titan.ron` (turn rate, acceleration,
strike, recovery, attack range, cooldown, aggro radius, death time, health), six blade numbers
and four hit-stop numbers in `gear.ron`, six rig fractions and three pose angles in `scale.ron`,
one `kill_target` in `missions.ron`, three signal colours in `maps.ron`.

**Every one is marked `⚠️ UNTUNED` in the file**, next to the reasoning that produced it. They
are not measurements and they are not design — they are the smallest values that let the thing
run, so that the first person who plays it has something to say "too slow" about.

Two of them are worth your eye specifically, because they are not arbitrary:

- **`turn_deg_per_s: 50.0` for the husk.** This is the number that decides whether the husk
  teaches anything. Too high and he always faces you and the approach angle stops mattering;
  too low and he is a statue. 50°/s means a half turn takes 3.6 s.
- **`hit_stop_cortex_s: 0.12`.** Forced by arithmetic, not taste: a husk cortex is crossed in
  36.7 ms at 30 m/s — **2.2 frames** — so without a stop the kill is invisible and the player
  sees only a counter change.

**ASSUMPTION:** these numbers stand until somebody plays it. **What would have to be rolled
back:** only file values. No code depends on any particular one of them; the code holds units
and mechanics, which is rule 2.

## Q-030 — The blade reaches exactly as far as the titan is wide, and not one centimetre further

**Context:** measured on 2026-08-09 while building `F-030`, and this is a measurement, not a
worry. `gear.ron: blades.reach_m` is **1.60 m**. A husk is 10 m tall at
`scale.ron: titan.width_fraction 0.25`, so his body radius is **1.25 m**, and the player's
capsule radius is **0.35 m**. `1.25 + 0.35 = 1.60`.

**There is zero margin.** For the blade tip to touch the cortex the player's capsule has to be
*exactly* on the titan's axis, and one centimetre closer is a collision instead of a pass. In
the measurement a test player at 30 m/s was thrown off at `(−28.4, 0, −13.0)` m/s while still
**1.7 m** from the cortex, and no pass ever landed against a solid body.

It gets worse with size, not better, and it is not fixable by shrinking the titan: a `large`
titan (14 m) has a body radius of 1.75 m, so `1.75 + 0.35 = 2.10 m` against a 1.60 m blade —
**0.50 m short**. Lowering `width_fraction` to 0.18 fixes the husk and still fails at `large`.

**Three ways out, and the third is the one I believe in:**

1. **Raise `reach_m`.** One line. But a 2.4 m blade on a 1.8 m person is not a blade any more,
   and it would have to keep growing with the largest titan you ever want to fight.
2. **Lower `width_fraction`.** One line. Does not reach; fails at `large` and above, and makes
   every titan skinnier for a reason that has nothing to do with how a titan should look.
3. **The body collider should follow the rig instead of being one fat column.** The cortex sits
   on the **nape**, at head height, and a head is `0.11 × height` — far narrower than the torso.
   A player cutting the nape never has to clear the torso's radius at all. The reach only looks
   impossible because the titan is currently one collider as wide at the neck as at the hips.

### REFUTED 2026-08-09, by measurement — and the mistake was mine

**The question above is wrong, and it was wrong when I wrote it.** An independent job was sent
to build option 3 and came back having measured that there is nothing to fix. Nothing in the
tree was changed to achieve that; `reach_m` is still 1.60 and `width_fraction` is still 0.25.

**My arithmetic left out two lengths that stand in the same two files.** The blade never has to
touch the titan's axis — it has to touch a **sphere** with a **swept capsule**:

| | husk (`medium`) | warden (`large`) |
|---|---|---|
| clearance the two capsules need | `1.25 + 0.35` = 1.60 m | `1.75 + 0.35` = 2.10 m |
| `reach_m` + `cortex_radius_m` + `thickness_m` | `1.60 + 0.55 + 0.12` = **2.27 m** | `1.60 + 0.60 + 0.12` = **2.32 m** |
| **margin** | **+0.67 m** | **+0.22 m** |

**Measured against a real solid husk**, real player, real `Intent`, real blade, 30 m/s at nape
height: cortex cut on tick 10, **0.484 m of air** at closest approach, **0.00 m/s** across the
flight line, 30.00 m/s throughout — never thrown off. Then hardened against my own wishful
reading with a 4 × 4 × 8 × 6 sweep (class × approach × offset × timing phase at 20/30/45/60 m/s):
the husk lands **6 of 6 phases at every speed** from three of four cardinal approaches. The
frontal approach never lands, which is correct — the cortex is on the back of the neck.

**Where my `(−28.4, 0, −13.0)` came from:** `tests/combat.rs::fly_past` places the player at
`cortex.x − 0.80` while the fixture's body capsule has radius 1.25 **centred on the cortex
axis**. The player was started 0.80 m *inside* a 1.25 m body. **The ejection was the test
fixture, not the titan** — I took a number out of a harness and reasoned about the game with it.

And option 3 would have made it **worse**, also measured: at nape height the player's capsule
spans `nape−1.6 … nape+0.2`, so it sits alongside the torso, whose box half-width is the same
1.25 m — no gain — while giving the hanging arms colliders would raise the required clearance to
2.225 m and close the windows above.

**Nothing to roll back. Nothing was changed.** What this cost: one job that measured instead of
building, which is the cheap outcome. What it would have cost unmeasured: per-part colliders
built to fix a defect that does not exist, and a real regression shipped as a fix.

**Still open, and it is a real one:** the **lurker**'s margin is **+0.120 m**, and measured, the
widest gap between the two capsules that still lands a cut is **0.10 m** at 30 m/s. Reachable in
arithmetic, not flyable in practice. The lever is `titan.ron: cortex_radius_m` — which
**Q-019 already flags** as the one value that did not follow the size table — and not
`width_fraction` or `reach_m`. That one is yours.

---

**The original assumption, kept for the record, and now withdrawn:**

**ASSUMPTION:** option 3, and **I have changed no number**. `reach_m` stays 1.60 m and
`width_fraction` stays 0.25, because both are honest values and the defect is in the collider
shape, not in them. Changing a number here would hide a geometry bug behind a tuning value —
and it would still be wrong at `large`.

**What would have to be rolled back:** nothing yet. The work is a follow-up job in `titan/`
(per-part colliders, or a narrower capsule at head height), not a file edit. If you would rather
have the cheap version now, it is one line in `gear.ron` and it says so above.

**Why it matters today:** the cut works and is proven at 8, 30 and 75 m/s against the cortex —
but those tests place the blade themselves. **A player flying past a solid husk cannot currently
reach the nape**, which means the kill is proven as a mechanism and not yet as a thing a player
can do.

## Q-031 — Is the approach angle a thing this game has? Today it is not, and no number can make it one.

**Context:** on 2026-08-10 you asked me to judge two of the 26 untuned numbers, and named the
husk's `turn_deg_per_s: 50` as one that "decides everything — does the husk turn so slowly that
the approach angle means something?". I went to tune it and found there is nothing to tune.

**Measurement** (the full record is [`FINDINGS.md`](FINDINGS.md) FIND-012, produced by one agent
and then attacked by an independent one that was asked to refute it and could not):

- `src/combat/strike.rs:99-103` — the titan's strike test is
  `ground_m <= reach_m && to.y <= top_m && to.y >= -reach_m`. **A cylinder.** No dot product, no
  forward vector. Front and back book the same damage.
- `src/titan/brain.rs:398-400` — the turn block runs only while `Pursue && distance_m >
  attack_range_m`, and `distance_m` is horizontal. **A titan does not turn inside 6.0 m**, nor in
  `Idle`, `Windup`, `Strike` or `Recover`.
- Every cut in `scripts/game-full.txt` lands at **1.882 m** from the axis. The husk in all three
  acts never turns once.

So `turn_deg_per_s` governs an approach that has no consequence, in a zone the titan has already
left by the time the fight starts. **This is not a tuning question and I will not pretend it is
one by picking a number.** Answering "50 or 80?" would produce a value, a commit, and no
difference whatsoever to anybody playing.

**The question that is actually yours:** should a titan's facing matter?

1. **No — the strike stays omnidirectional.** Then `turn_deg_per_s` is decoration, the nape is
   reachable from any side, and the skill in the fight is entirely about height, speed and
   timing. This is a legitimate design; it is roughly what a "swarm of hazards" game does. Then
   the honest move is to delete the knob, not tune it.
2. **Yes — the strike gets a facing cone, and the titan turns inside its attack range.** Then the
   approach angle becomes real, `turn_deg_per_s` becomes the number you said it was, and the
   cortex-on-the-nape design finally means something mechanically: coming from behind is *safer*,
   not merely equally good. This is what the design bible implies everywhere it talks about the
   nape.

**ASSUMPTION: (2), and I have implemented none of it.** I am assuming the bible means what it
says about the nape, so the hole gets recorded rather than papered over with a value. **I have
changed no number and no line of combat code on the strength of this assumption.**

**What would have to be rolled back:** nothing. That is the point of stopping here. If you pick
(1), Q-031 closes and `turn_deg_per_s` plus `attack_range_m` should be deleted from `titan.ron`
with a line saying why. If you pick (2), it is a job in `combat/strike.rs` (a half-angle against
the titan's forward vector) and one in `titan/brain.rs` (drop the `distance_m > attack_range_m`
guard on the turn), each with a red test first.

**Why it matters today:** it gates the tuning round you asked for. Seven of the 26 untuned
numbers are per-kind combat values in `titan.ron`, and at least two of them are meaningless until
this is decided.

## Q-032 — The reel is a jolt, and no value of `reel_speed_m_s` can make it a swing

**Context:** on 2026-08-10 you asked: *"vector.reel_speed_m_s (ein Reel gibt 46,4 m/s — liest
sich das als Ruck oder als Schwung?)"* and said it was file work, not code work. **It is a jolt,
it is measured, and it is the one part of your instruction the measurement contradicts: this
cannot be fixed in a file.**

**Measured [cachy], 2026-08-10** (full record in [`FINDINGS.md`](FINDINGS.md) FIND-013, 16 runs,
reproduced byte-identical):

- The player's speed goes `0.000 → 28.000` m/s in **one tick**: **1680 m/s², 171 g**. The reel's
  own next tick adds +0.05 m/s. The onset is ~500× the steady state.
- Across nine rungs the first tick equals the configured rate to three decimals. **The law is
  `speed := reel_speed_m_s`** — an assignment, not an acceleration. Confirmed against a moving
  start (airborne at 5.5 m/s → 28.307 one tick later).
- End speed is **monotone** in `reel_speed_m_s` and pins at `MaxLinearSpeed(75)` between r=32 and
  r=36. There is no sweet spot to find by trying values.

**Your premise number is also wrong, and it was mine, not yours.** 46.414 m/s is not what a reel
gives. The reel ends at **54.18 m/s**; `scripts/f-001-hooks.txt` samples five ticks later, after
the rope has taken 14.2 % of his speed in one tick (FIND-016). Every earlier statement in this
repository about "a reel gives 46.4 m/s" describes a reel plus a loss.

> ⚠️ **And 46.414 was not a rope number at all.** Isolated later the same day (FIND-033): with the
> rope untouched and only `src/player/locomotion.rs` reverted, the same script gives **46.414**;
> with today's locomotion and **four different rope behaviours**, it gives **19.344** in every
> case. The old figure came from `ground_locomotion` *deleting* the player's horizontal velocity
> on every tick while he was still `Grounded` on the rope, so only the joint's vertical work
> accumulated and it threw him past his own anchor. **19.344 m/s is the honest number the rope
> reaches today**, and `docs/PLAN-GAME.md` §3.1 Risk 1 (a player at 30 m/s) is currently **not met
> by rope work** — every 30 m/s in this repository is a fall.

**So the honest answer to "which value?" is: none of them.** Raising it makes the jolt bigger;
lowering it makes the jolt smaller *and* the reel weaker, and below ~20 it stops being useful.
The knob that would decide feel does not exist yet — an ease-in on the length rate. What it would
buy, **computed** from the measured law (not measured; nothing ramped has ever run):

| `reel_ramp_s` | per-tick Δv | accel | ramp | reel duration (was 0.5297 s) | extra gas |
|---|---|---|---|---|---|
| — (today) | **28.000 m/s** | 1680 m/s² · 171 g | 1 tick | 0.5297 s | — |
| 0.05 | 9.333 | 560 m/s² · 57 g | 3 tk | 0.5547 s (+4.7 %) | 0.15 |
| **0.15** | **3.111** | 187 m/s² · 19 g | 9 tk | 0.6047 s (+14.2 %) | 0.45 |
| 0.25 | 1.867 | 112 m/s² · 11 g | 15 tk | 0.6547 s (+23.6 %) | 0.75 |

0.15 s is the value at which the onset becomes indistinguishable from what the reel already does
one tick later (+3.102 m/s), for 4.5 ticks and 0.45 gas of 100.

**ASSUMPTION: `reel_speed_m_s` stays at 28.0 and I have changed nothing.** Tuning it would be
tuning the size of a discontinuity, and it would bury a mechanical defect under a number — the
same mistake Q-030 refused to make. 28 also sits with headroom below the clamp, which values
above 32 do not.

**What would have to be rolled back:** nothing. No file value was changed and no code was
touched. The follow-up is `vector.reel_ramp_s` in `game.ron` plus an ease-in in
`src/vector/reel.rs`, behind a red test that asserts the first tick's Δv is *below* the rate —
which is exactly the assert that would have caught this a day ago.

**Second lever, and it carries no `⚠️ UNTUNED` marker at all:** `min_rope_m`. Measured at stock
rate, 3.0 → **54.18 m/s** and 5.0 → **39.62 m/s**. Two metres removes 27 % of the top speed. Its
stated reason in the file is a *camera* constraint, so it is doing two unrelated jobs at once and
should probably split into a separate `vector.min_reel_m`.

## Q-033 (original entry, kept for the record) — The gas never refilled. I gave it a regeneration; the shape of it is yours.

**Context:** on 2026-08-10 the user played the game — **the first time a human ever has** — and
said: *"der boost hält nicht lang genug"* and *"seile ohne boost bringen gar nichts"* and
*"also gas tank sollte sehr viel mehr haben"*.

**Measured, and it is worse than the complaint:** `Gas` was written in exactly two places —
`Gas::full()` at spawn (`src/player/mod.rs:118,171`) and `gas_budget` in `src/vector/gas.rs`,
which only ever subtracts. **There was no refill of any kind.** At `gas_boost_per_s: 18.0` on a
`gas_tank: 100.0` that is **5.6 s of boost for a 330 s mission**, after which the Vector Gear
was dead for the rest of the run. Titans have carried a `regen_per_s` since 2026-08-09; the
player had nothing.

**I asked him which shape the refill should take (regenerate while idle / refill on the ground /
never but a much bigger tank) and he did not answer that half.** So, under the autonomous rule:

**ASSUMPTION: it regenerates while neither boosting nor reeling, after a short delay**, with
these values — all four in `assets/data/game.ron`, all marked `⚠️ UNTUNED`:

| key | was | now | what it buys |
|---|---|---|---|
| `gas_tank` | 100.0 | **300.0** | the user asked for "sehr viel mehr" |
| `gas_boost_per_s` | 18.0 | 18.0 | unchanged, boost still costs |
| `gas_regen_per_s` | — | **10.0** | refills **only while nothing is being spent** |
| `gas_regen_delay_s` | — | **0.5** | tapping boost does not become free |

**Consequence: a full tank is `300 / 18` = 16.67 s of continuous boost, against 5.6 s before**,
and an empty tank refills in `300 / 10` = 30 s of not using it. The delay is 30 ticks.

⚠️ **Correction, 2026-08-10.** This entry first claimed "holding boost drains at a net 8/s, so a
full tank is 37.5 s". **That was wrong and it was the supervisor's arithmetic, not the
implementation's.** A net drain requires the refill to keep running *during* the boost, which
contradicts the decision one line above it, empties `gas_regen_delay_s` of meaning, and
contradicts the very tests this entry asked for ("does not refill while boost is held"). The
agent implemented the decision and reported the error rather than quietly matching the number.
**The tripled tank is the whole answer to the user's complaint; the regeneration lengthens no
single held boost at all** — what it buys is that the gear is never permanently dead.
*If 37.5 s is what is actually wanted*, the change is one line — move `refill_tank` above the
spend branch in `src/vector/gas.rs` — and then `gas_regen_delay_s` must be deleted, because it
would no longer mean anything, and three tests go red on purpose.

**Why regeneration and not one of the other two:** the bible coples the resource to risk rather
than to a timer — burning gas is *loud*, and a Bellower answers it (bible line 159). A resource
that is simply gone after five seconds cannot carry that; a resource you keep choosing to spend
can. Refilling only on the ground was the tempting alternative and it loses the same way: it
turns the Vector Gear into a thing you use between rests, and pillar P1 says it **is** the
combat, not the transport between fights (bible line 29).

**What would have to be rolled back:** four values in one file and one system. No feature depends
on the shape of the refill; `gas_regen_per_s: 0.0` restores the old behaviour exactly, and
`gas_regen_delay_s` becomes dead but harmless. If you would rather have "ground only", that is a
condition on one `if` in `src/vector/gas.rs`, not a redesign.

**Still open and NOT decided by me:** whether 300 / 18 / 10 / 0.5 *feel* right. No test can
answer that. It is the same class of question as Q-029, and only you can close it.

## Q-034 — A rope that has been shortened already stays shortened. You asked; here is the answer.

**Context:** the user, 2026-08-10: *"und wenn mit seilen verbunden und wurde kürzer soll erstmal
nicht länger werden. aber da bin ich nicht sicher."*

**It already behaves that way, and this is not a question but a confirmation.**
`src/player/rope.rs:287` is the only write to the length after anchoring:

```rust
let next_m = (joint.limits.max - rate_m_s * dt).max(min_rope_m);
```

It only ever subtracts, floored at `min_rope_m: 3.0`. `attach_ropes` sets the initial length once,
to the distance that really existed at the moment of anchoring, "so anchoring never yanks the
player" (`rope.rs:105`). **Nothing anywhere increases `limits.max`.** A rope that got shorter
stays shorter until the hook is released.

**The neighbouring fact, which is probably the one that matters:** `limits.min = 0.0`, so the
joint corrects **only** when the distance exceeds the maximum. The rope is a **leash, never a
rod** — it pulls and can never push. Together with a reel that *assigns* velocity instead of
adding to it (`FINDINGS.md` FIND-013), the working hypothesis for *"seile ohne boost bringen gar
nichts"* is that **a rope-only player has no mechanism at all for turning a swing into height or
into speed he did not already have.** That is being measured; it is not yet established.

## Q-035 — The hook range is 200 m now. That contradicts `prompts/init.md` §1 and your own 90 m.

**Context:** on 2026-08-10, after playing it, the user wrote *"zudem muss die hook range sehr viel
länger sein!"*. He had given the 90 m himself on 2026-08-09
(`scale.ron: vector.anchor_range_m`, and Q-002 records that it displaced the backlog's 112 m).

**Two documents now disagree with the instruction, and I followed the instruction:**

- `prompts/init.md` **§3** (line 411, item 5 „Eine Einheit": *„ein Haken fliegt 60–120"*) names a
  design window of **60–120 m**. 200 is well outside it, and
  `tests/data.rs::t005_hook_range_stays_in_the_design_window` fired on exactly that.
  *(That sentence has been cited as §1 in this file and in the test's comment since 2026-08-09 —
  wrong section, verified 2026-08-10. Corrected here and in the guard; Q-002 carries the same
  error and is corrected there too.)*
- The user's own earlier figure was 90 m.

The precedence rule this project already runs on (Q-002, and stated in `scale.ron` itself) is that
**a direct figure in metres from the user beats any derivation** — and a live instruction beats the
number it replaces, including one of his own. So the window was widened rather than the value
trimmed, and the guard's message now names the conflict and the date instead of quietly passing.

**ASSUMPTION: 200 m, and here is why not 150 and not 400.** The graybox is 400 m across. At 400 m
of range every anchor in the world is always in reach and *where you stand stops being a decision*
— the map would no longer exist as a design surface. 200 m is the largest range that keeps position
meaningful, and it reaches the church (35 m, the only structure tall enough to give a real arc,
`FINDINGS.md` FIND-026) from more than half the city, which is precisely what the measurements say
is missing.

**Two numbers moved with it, and neither is taste:**

| key | was | now | why |
|---|---|---|---|
| `vector.hook_speed_m_s` | 90.0 | **160.0** | a hook is a projectile: at 90 m/s a 200 m shot takes **2.22 s** to arrive — longer than most of a swing, and long enough that the anchor you aimed at is not where you are any more. 160 holds the worst case at **1.25 s**, against the 1.0 s the old 90 m/90 m/s pair cost. |
| `world.half_extent_m` | 300.0 | **400.0** | forced: the spatial index must carry half the map plus one full range, `200 + 200 = 400` exactly. **Cost: the grid covers 800×800 m instead of 600×600 at `cell_m: 8.0` — 10 000 cells against 5 625, i.e. 1.78× the index memory** for the same city. It buys nothing by itself; it is the price of the longer rope. |

**What would have to be rolled back:** three numbers in two RON files, one widened window in
`tests/data.rs`, and one table row in `docs/models.md`. No code depends on any of them. Setting
`anchor_range_m` back to 90 and `half_extent_m` back to 300 restores the previous behaviour exactly;
`hook_speed_m_s` would go back to 90 with it, or stay at 160 as a separate improvement.

**Still open and not decided by me:** whether 200 m actually feels right, and whether a 1.25 s
worst-case hook flight is acceptable or already too slow to read. Both are yours.

---

## Answered

*(nothing yet — the user's first answer comes here, with a date)*

---

## ⬇️ APPEND NEW QUESTIONS BELOW THIS LINE — with `>>`, never with an edit tool

This file is **68 kB**. Opening it to add one entry costs ~17 000 tokens.

```bash
cat >> docs/QUESTIONS.md <<'END'

## Q-0nn — <the question, in one line>
...
END
grep -n '^## Q-033' docs/QUESTIONS.md && sed -n '900,940p' docs/QUESTIONS.md   # read ONE entry
```

Every entry carries the same four parts, and a question missing them is useless to the user:
the **context**, the **ASSUMPTION:** the work continued under, **what would have to be rolled
back**, and **why it matters today**.

## Q-036 — `scale.ron` has nothing between 12 m and 35 m, and that hole is why the scaffolding cannot go

**Context:** you asked for the district to look like the reference *"möglichst akkurat"*, and for the
scaffolding-looking gantries to be architecture instead. Two rebuilds later the density is right
(7.00 m facade to facade, street:ridge **0.82 : 1** against a surveyed real ratio of 0.62 : 1, 596
facing facades against 311 before) — **and the gantries still cannot be removed.** The reason is a
gap in your own size table, not in the map.

**Measured:**
- A rope between two houses now satisfies `d < H` — but the anchor stands over **solid house**, so
  the pendulum's low point is *inside the building*. Measured: a 7 m street gives an **arc bottom of
  5.92 m at 14.278 m/s, and the swing ends against the facade it hooked**. It is a hop, not a lane.
- Any rope long enough to be a lane (**≥ 18 m**) gives `11.5 − 18 = −6.5 m` of arc bottom — i.e.
  underground — because housing is capped at **11.5 m**.
- What would replace the gantries honestly: **a corbelled tower top at 24–28 m.** From a roof at
  11.5 m, 20 m away, that gives an arc bottom of **+2.1 m** — a real lane over the street.

**`scale.ron: architecture` has 4.5 / 8.0 / 11.5 / 12 (watchtower) / 35 (church).** There is
**nothing between 12 and 35**, so every structure that could carry a swing lane is either a house
(too low) or the church (a single landmark). The surveyed reference ridge is **13 m**, so even our
tallest house is slightly under the real thing.

**ASSUMPTION: the gantries stay, and I have invented no new height class.** Making one up would put
a building type in the world that you never named, in a project where the size table is explicitly
yours (`scale.ron` header: *"NOT untuned — it is given by the user"*).

**What I need from you — one or two numbers:**
- a **tower / gatehouse / granary class around 24–28 m** (this is the one that removes the
  scaffolding), and
- optionally `architecture.eaves_m` for **`house_large`**, which is missing entirely — that is why
  every 11.5 m house in the screenshot is a flat box while the 4.5 and 8.0 classes had pitched caps.

**What would have to be rolled back if you say no:** nothing. The map works and is 🟧 for geometry.
The gantries stay, the skyline stays flat, and the district keeps a piece of game furniture down its
main axis.

## Q-040 — "horizontal" is the SCREEN's horizontal, not the world's. Is that what he meant?

**Raised 2026-08-19 by the round that built the sideways-only snap (`docs/FINDINGS.md` FIND-133).**
His sentence names the case where the two readings **coincide** and says nothing about the rest:

> *„wenn das fadenkreuz 0, 0 ist sollen die seile nur auf der x achse snappen … also seitlich!"*

At a level crosshair, screen-horizontal **is** world-horizontal. They part company the moment he
pitches:

- **screen-horizontal (built):** the sweep spans `look·cos θ ± right·sin θ` with `right` the
  camera's own horizontal. Looking 60° down, "sideways" is still left and right *of the crosshair*;
  the candidates' world elevation then spans `asin(sin(−60°)·cos α)`, i.e. −60° to −54.5° at the
  20° end stop, and at −89° it spans a full 20° of world elevation. **On the screen the deviation
  is exactly zero at every pitch** — measured 0.000006° over 313 published points, 0.041 px over
  136 drawn pairs.
- **world-horizontal (not built):** the sweep stays in the world's XZ plane whatever the pitch.
  Then the *world* claim is exact — but looking 60° down the two markers walk **diagonally** away
  from the crosshair on screen, and at −90° "sideways" has no screen meaning at all.

**ASSUMPTION the work continues under: screen-horizontal.** Three reasons, in order of weight:
1. **His own criterion is a reading criterion** — *„dann ist es auch besser einzuschätzen"* — and
   what he reads is the screen. A marker that leaves the crosshair's row cannot be judged against
   the crosshair however tidy its world coordinates are.
2. His literal constraint as the brief states it is *"the snap must never move a rope up or down
   **relative to where the player is looking**"*, and "relative to where he is looking" is the
   camera's frame by definition.
3. `F-023`'s two side rays have yawed around the **camera's** up axis since 2026-08-18, for exactly
   this reason (`side_dirs`, FIND-096). One axis for the fan and another for the search would put
   the two markers and the two candidates in different frames.

**Rollback point if he meant the world's horizontal:** `src/vector/aim.rs::probe_dirs`, one line —
replace `basis[1]` and `basis[0]` with their XZ projections (`Vec3::new(look.x, 0, look.z)`
normalised, and `right` unchanged, which is already horizontal), and re-normalise. Then
`tests/vector_hooks.rs::f024_every_probe_sits_on_the_crosshairs_own_row_and_never_above_or_below_it`
and `::f024_a_published_snap_point_never_sits_above_or_below_the_crosshair_in_the_running_game`
**both have to be rewritten** — they assert the camera frame — and
`tests/hud.rs::f024_a_snap_moves_the_marker_sideways_on_the_screen_and_never_up_or_down` would go
red at every non-zero pitch by design. Nothing else reads the sweep's frame.

### What would settle it

Him flying, looking 45–60° down at a street with the assist at 50/50, and saying whether the two
markers sliding **along the crosshair's row** reads right — or whether he expected them to stay
level with the *horizon* instead.

---

## Q-041 — the marker now dodges the crosshair on 78 % of samples. Does that read as the snap moving up?

**Raised 2026-08-19, same round.** A cost of the feature and it is measured, not feared.

⚠️ **Updated 2026-08-23 and the number roughly doubled again, for a second and unrelated reason**
(`FIND-154`): `F-023`'s fan is retired, so **both** arms now carry the same point instead of two
different ones, and the pair dodges together where one of them used to be off at its own side ray.
Same sweep: **588 of 752** samples step out of the core, against 302 of 708 yesterday and 101 of
768 the day before. **The worst step did not move** — 20.5 px against 21.5 and 20.0 — and the
horizontal error is 0.45 px over the whole sweep, so this is still a question about *how often*,
never about *how far*. The three answers below stand unchanged.

Every candidate now projects onto the crosshair's own screen row, so a marker near the middle of
the screen collides with `SIGHT_CORE_PX` — the 6 px square the player is cutting — far more often
than it did when candidates were scattered over a disc. `tests/hud.rs::f023_the_drawn_pixel_...`,
same sweep, across the three rounds: **101 of 768 → 302 of 708 → 588 of 752**, and the worst step
went 20.0 px → 21.5 px → 20.5 px. The step is bounded (`full_h/2 + SIGHT_CORE_PX`) and it is the
only vertical movement a marker has left — but it is now the common case rather than the corner one.

**The three answers:**
1. **Leave it.** The step is small, bounded, and `F-170` ("nothing covers the middle of the
   screen") is a photographed claim the user made himself.
2. **Shrink `SIGHT_CORE_PX`** from 6. Every pixel off it is a pixel the marker keeps. FIND-098
   measured that the *keep-out box* cannot shrink below 3.4 % without collapsing the crosshair —
   this is the other, much smaller square and it has never been measured.
3. **Let the marker sit on the core** when it holds a place, and thin the glyph instead. Maximal
   honesty, and it trades against `F-170` a second time in one day.

**ASSUMPTION the work continues under: (1).** The dodge is the rule FIND-129 landed hours earlier
and it is bounded; changing it twice in one day on nobody's complaint is how a verified picture
gets un-verified. **Rollback point for (2):** `src/hud/arm_aim.rs::SIGHT_CORE_PX`, one constant —
`f023_the_drawn_pixel_...` and `f170_a_world_marker_keeps_its_pixel_and_a_badge_keeps_out_of_the_box`
both read it and move with it. For (3) it is the two `over_the_core` arms of `layout_for`.

### What would settle it

**A screenshot, and this round did not take one.** Nobody has looked at a marker sitting 12–24 px
under the crosshair in a window. That is 🟨 on the look and 🟧 on the number.

## Q-043 — `W` on a rope does nothing unless you are looking at your own hook. Is that the verb you asked for? (2026-08-20, F-018 budget round)

**Measured, `docs/FINDINGS.md` FIND-139:** over one ordinary sortie across Ashgate
(`scripts/f018-budget.txt`), holding `W` on a taut rope delivered a mean thrust of **0.0012** of
`player.air_pull_m_s2` — median **0.0000**, across 99 sampled ticks. Three full seconds of `W` in
the pose a swing actually spends its time in move the player by **nothing at all**, and that is now
pinned in the script as `assert gas == 300` after LEG B.

**Why:** `player::locomotion::rope_steer` multiplies the pull by `cᵢ = max(0, l̂ · r̂ᵢ)`. You hang
*under* your anchor and you look where you are going, so `l̂ · r̂ᵢ` is negative for most of a swing
and the clamp takes the whole pull to zero. `W` hauls you along the rope **only while you are
looking at your own hook.**

**And you asked for that verb yourself,** which is why this is a question and not a bug:

> *„wenn ich mit seilen festhake und w in die richtung drücke will ich dass man deutlich mehr
> geboosted wird"*

and `F-023` landed under the line *"W hauls you where you look"*. Both readings are defensible and
they are different games:

1. **`W` hauls you along the rope, and aiming at your hook is the skill.** What is built today. The
   clamp is then correct and the verb is a deliberate move, not an ambient one — but a player who
   never looks up at his hook will never find it, and nothing on screen tells him.
2. **`W` hauls you where you LOOK, and the rope only bends it.** The clamp becomes a blend
   (`0.5 + 0.5·(l̂ · r̂)`, or the look direction with a rope-fraction like `boost_rope_fraction`
   already does for the boost), so `W` always thrusts and the rope decides how much of it is along
   the leash. Closer to the commit line, and it makes `W` the traversal verb it reads as.

**ASSUMPTION the work continued under:** (1) — the mechanic is left exactly as it is. This round
changed only the **price**: gas is no longer charged for the ticks the thrust is zero
(`vector::gas::steer_has_effect`). Nothing about the feel of `W` moved, and no game value moved.

**Rollback point if you want (2):** it is one expression in
`player::locomotion::rope_steer` — `let projection = look_dir.dot(direction).max(0.0);` — plus a
new blend key in `game.ron`'s player block. Everything this round built survives it unchanged:
`steer_has_effect` reads the same formula, and
`tests/vector_gas.rs::f006_the_steer_is_billed_exactly_when_the_rope_really_thrusts` would go red
on the day the formula changes and be re-cut against the new one. **That test is the thing that
makes (2) cheap** — the price cannot silently fall out of step with the thrust again.

---

## Q-044 — the tank is 17 seconds and the mission is 330. The knob has now been turned twice for the same sentence. (2026-08-20)

**Your sentence, 2026-08-20:** *„eine sache die mir noch auffällt. gas ist VIEL zu schnell weg!"*
**Your sentence, 2026-08-10:** *„also gas tank sollte sehr viel mehr haben"* — which is why
`gas_tank` went 100 → 300.

**What this round did NOT do is turn it a third time,** and the reason is the measurement: the
tank was not the cause. 48.3 % of it was being charged for a thrust that was never delivered
(FIND-139), and fixing the bill gave the same sortie **77 of 300 gas back** without touching a
single game value. That is the honest part of the answer.

**Here is the part that is not solved, and it is yours.** With the bill repaired, 300 gas buys:

| what you are doing | gas/s | seconds of it in one tank |
|---|---|---|
| swinging, nothing held | 0 | ∞ — **a swing is free, and that is the design** |
| `W` on a rope, looking forward | 0 | ∞ — see Q-043 |
| holding `Shift` | 18 | **16.7 s** |
| `Shift` + `W` aimed at your hook | 34 | 8.8 s |
| one dodge (`C`) | 45 flat | **6.7 dodges, and nothing else** |

**A sortie is 330 s.** There is no refill anywhere in the world — that is your own decision,
`Q-033`, *„gas refillt nur im main gebäude an bestimmten stationen/objekten"* — and **the refuel
stations do not exist yet outside the hub** (`docs/NEXT.md` §1d). So the whole mission's supply of
thrust is seventeen seconds, and no burn rate in `game.ron` closes a 20x gap.

**This round did not touch the refill rule and will not:**
`tests/vector_gas.rs::f018_an_idle_tank_never_refills_on_its_own` goes red the moment anyone puts
a drop back on a timer, and the bible's argument for that is good — burning gas is loud, a Bellower
answers it, and a tank that fills itself on a roof is exactly the clock the design refuses.

**ASSUMPTION the work continued under:** the gap is a **missing world feature, not a wrong number**
— the stations of `NEXT.md` §1d are the answer, and until they are built a sortie is meant to feel
supply-limited. **Rollback point if you disagree:** it is one number,
`assets/data/game.ron: vector.gas_tank`, and the derivations that hang off it are the "gas per m/s
bought" table in the same block — the table stays true at any tank size, so raising it breaks
nothing. `f018-budget.txt`'s brackets and
`tests/vector_gas.rs::f018_a_tank_is_a_whole_mission_of_flying_because_nothing_refills_it` are the
two places that would have to be re-cut.

**What it would cost, said plainly (JOB 3 of this round):** every gas you give back makes the
Vector Gear less of a resource and the bible's *"gas is the clock"* weaker. **The repair in
FIND-139 does not cost that** — it only stopped charging for nothing, and every gas the player
now keeps is gas he was never getting thrust for. **A bigger tank would cost it**, and that is
exactly why this round left `gas_tank` alone and wrote this entry instead.

---

## Q-045 — the rope now takes 70 % of your weight when you look at your own hook. Is that the trade you meant — and does it make `Q-043`'s split too sharp? (2026-08-20, F-005 feel round)

**Your sentence, verbatim:**

> *„und man muss wenn man sich hookt und in die richtung gehen stärker in die richtung gehen! also
> wenn man da hin schaut dass nicht alle physics also gravitiy so stark sind. dass man gerader
> hingezogen wird. damit es sich gut anfühlt. damit man gut steuern kann damit. aber wenn man nicht
> hinschaut man auch gut kreise schwingen kann"*

**What was built, and the numbers** (`docs/FINDINGS.md` FIND-141). The obvious knob,
`vector.boost_rope_fraction`, provably could not do it: it blends a *direction*, and at full
alignment the look **is** the rope, so the blend is a no-op exactly where you want the change.
What was wrong instead was arithmetic — looking straight down a rope with `W` held, the thrust is
40 m/s² along it and gravity is 20 across it, so the game hauled you **26.57° below the line you
were aiming at**, and in the real game 1.5 s of `W` at an anchor 21.7 m above you produced
**zero metres of climb**.

So `player.air_pull_lift_fraction: 0.7` takes 70 % of `gravity_m_s2` off — **gated on the same
`max(0, look · rope)` the pull itself is gated on**, which is your two clauses in one term:

| | before | now |
|---|---|---|
| droop when you look at the anchor | 26.57° | **8.53°** |
| height after 1.5 s of `W` at an anchor 21.7 m up | 11.997 m (from 12.0) | **26.828 m** |
| swing speed with the look 90° off the rope | 13.334 m/s | 13.431 m/s (**+0.7 %**) |

**Two things for you to judge, and the work did not wait for either:**

1. **0.7 is a guess with bounds, not a measurement.** Above 1.0 you are weightless while looking at
   your hook and there is no arc left to fall into; below ~0.5 the approach still visibly droops.
   0.7 leaves 6.0 m/s² of weight — a running-speed's worth of downward pull every second.
2. 🔴 **It makes `Q-043` sharper, and that may be the wrong direction.** `Q-043` already asks
   whether `W` doing *nothing* while you look away from your hook is the verb you wanted. The gap
   between the two poses is now **wider**, not narrower: looking at the anchor buys 40 m/s² *and*
   70 % of your weight; looking away still buys the 10 m/s² of free air control and nothing else.
   If your answer to `Q-043` is "no, `W` should haul me even when I look forward", then this key's
   gate is the wrong gate and both have to be re-cut together.

**ASSUMPTION the work continued under:** your two clauses describe **one trade along the angle**,
and the alignment cosine is that angle — so the relief belongs on the same gate as the pull, and
the swing is meant to keep the full −20 it is built on.
**Rollback point:** two lines and nothing else. `assets/data/game.ron: player.air_pull_lift_fraction`
back to `0.0` restores the game you played, bit for bit (the term is `direction * pull + Y * lift`
under one shared gate — at `0.0` the `Y` half is gone and the rest is untouched). The bound test
`tests/player.rs::f005_the_gravity_relief_is_a_fraction_between_the_droop_and_weightlessness` and
the `> 0.5` half of its claim would have to go with it.

**Not decided here and left for you:** `vector.boost_rope_fraction` is still the constant `0.5`,
so a boost fired while you look 90° off your rope is still dragged half-way toward the anchor —
radial, which a taut rope eats, which the file's own comment calls "killing the swing". That is the
same complaint from the Shift side and it belongs to `src/vector/boost.rs`, which another agent
held this round. The proposed patch is in FIND-141 and it is **unmeasured**.

---

## Q-046 — the tank is 15000 because you asked for it a third time. Is that the balance, or only the testability? (2026-08-20, gas round)

> **You, 2026-08-20:** *„immernoch viel zu wenig gas. man kann nicht testen! mach das 50 fache!"*

**Done, exactly as asked:** `assets/data/game.ron: vector.gas_tank` **300.0 → 15000.0**. No
compromise at 10x, no derivation argued against it. Your instruction beats a derivation and beats
your own earlier number — that precedence is already written down in `CLAUDE.md`, in `scale.ron`
and in `Q-002`, and this is the third time you have said this same sentence (100 → 300 on
2026-08-10 for „also gas tank sollte sehr viel mehr haben", then 300 → 15000 today).

**What it buys, in the only unit that matters here:** at `gas_boost_per_s: 18` a full tank is
**833.3 seconds of continuously held boost**, against a sortie of 330 s. It was 16.7 s. The tank
now outlasts the whole mission by 2.5x, so a run cannot end for lack of gas while you are trying
to feel the movement. That is what „man kann nicht testen" asked for and it is met.

**ASSUMPTION the work continued under:** this is a **testability** value and not a balance
decision. Nothing has been tuned around it, nothing else was moved to compensate, and no test now
requires the tank to be this big — the floors that mention it (`> 10 s` of boost, `> 20` bursts)
were deliberately left as loose downward guards rather than tightened around 15000, precisely so
that putting it back is a one-line edit and not a red suite.

**Rollback point: `assets/data/game.ron: vector.gas_tank` back to `300.0`.** Nothing in `src/`
depends on the magnitude. What must be worked through with it is listed **in the build**, which is
new: `tests/data.rs::t005_the_gas_tank_is_the_value_the_user_asked_for_and_names_its_dependents`
pins the number and its failure message names every script, test and doc that quotes it. Two things
would want deliberate restoring rather than a number swap:
- the dodges-per-tank **ceiling** in `tests/vector_boost.rs::f008_the_dodge_is_the_expensive_boost
  _and_shift_is_the_cheap_one` (`per_tank <= 15`), removed today because 15000/45 = 333 makes its
  own sentence — "over 15 it is not expensive at all" — **true**;
- `scripts/f-018-gas.txt` ACT 3 and `scripts/f170-hud.txt`'s 82 % ruler, both re-cut below.

### 🔴 What the 50x does NOT answer, and it is the question you actually have to decide

**`Q-044` stays open and this makes it louder, not quieter.** With the steering bill made honest
(FIND-139, this morning) a 300 tank bought ~16.7 s of held boost against a 330 s sortie, and
**there is still no refuel station anywhere in the world.** The design's refill rule is yours and
it is base-only (`Q-033`: „gas refillt nur im main gebäude"). No burn rate closes a 20x gap — only
the stations do, and they are queued in `docs/NEXT.md` §1d and not built.

So today the gap is closed from the other end, by a tank fifty times the size of the one the
economy was drawn for. **That has a cost, and it is not the number — it is that gas stopped being
a decision.** Concretely: a dodge was 6.67 per tank and is now 333, and `F-008`'s own cooldown is
still not built (FIND-067), so **nothing at all now limits the dodge as a traversal move.** If you
fly it and it feels like the gas is gone from the game rather than merely generous, that is this,
and the fix is the stations plus a rollback — not a burn rate.

**The three questions that are yours, in the order they matter:**
1. Is 15000 what you want to keep, or is it "enough to test with" until the stations exist?
2. Should the refuel stations be built next (they are the only thing that makes a smaller tank
   playable), or does the sortie simply not have a gas economy at all for now?
3. Does the dodge need its cooldown now that its gas price no longer limits it?

### What was re-cut rather than rebased, and why

Two claims could not survive a 50x tank as numbers, and both were **suspended visibly instead of
being weakened into always-green asserts**:
- **`scripts/f-018-gas.txt` ACT 3, "the tank empties and a held button buys no half-boost".**
  Emptying 15000 at 18/s is 833 s = 50 000 ticks, and a headless run is **wall-clock locked at
  60 Hz** (measured today: 3094 ticks take 52 s whether the player flies or stands still), so that
  is a 14-minute run — and the exit code needs its own second run. The claim moved into
  `tests/vector_gas.rs::f018_the_tank_runs_dry_and_stays_at_zero_instead_of_going_negative`, which
  now sets a two-second tank on the player and burns that. ACT 3 measures the rate's linearity over
  twelve seconds instead, which is a real claim it can still make.
- **`scripts/f170-hud.txt`'s 82 % gas bar**, the ruler F-170 exists for. At 15000 the same three
  seconds of boost draw a **99.64 %** bar — full to any ruler. Getting back to 82 % needs 2700 gas
  = 150 s of boost = 9000 ticks per run. It is suspended, `docs/images/f170-hud.png` is marked
  stale in the script's own header, and the mechanism half still goes red in `tests/hud.rs`.
  **`docs/images/f-018-gas.png` is stale the same way** — it shows an empty bar for a run that now
  ends 97.96 % full.

Both come back in one line each if `src/debug/script.rs` gains a **`settings gas <n>`** verb so a
script can ask for a small tank for one act. That file was foreign territory to this change, so it
is a proposal in `docs/FINDINGS.md` (FIND-142) and not something taken quietly.

---

## Q-047 — the blade got longer, thicker and a rear-only rule. Three knobs, and I want you to feel them. (2026-08-20, F-030 hitbox round)

**Your words:** *„hitboxen passen noch nich!"* — and you asked whether we had looked at how
Attack on Titan: Revolution does it. We had not; that round covered movement only. Now we have
(`docs/gameplay/references.md`, the whole last section), and the answer changed what I did.

### What was actually wrong, measured before anything was touched

`scripts/f030-hitbox.txt` flies one nape pass at every kind that can spawn, each with **0.60 m of
air between your capsule and his**. Before today: **2 of 7 kinds died.** The **lurker could not be
killed from any offset at all** — 8 passes from 0.00 m to 0.70 m of air, 8 torso hits, not one
nape. The reason is not the nape's size. It is that a titan's body capsule is
`width_fraction × height`, so the clearance you must cross grows with the class **three times
faster than the nape may grow** (your own head rule caps it at `height / 18`). The kind with the
smallest nape in the game has the widest tolerance; the big ones had none.

After: **7 of 7, exit 0.** `docs/FINDINGS.md` FIND-147 has the whole table.

### 🔴 The three numbers I most want you to feel, in this order

| knob | was | is | what it feels like if it is wrong |
|---|---|---|---|
| `gear.ron: blades.reach_m` | 1.60 | **2.00** | too long: you kill things you did not aim at, and a fly-by feels like a lawn mower. Too short: the 14 m kinds are unkillable again |
| `gear.ron: blades.thickness_m` | 0.12 | **0.20** | this is **vertical** forgiveness in a flying pass. At 0.12 your **eye** had to be within ±0.32 m of a scuttler's nape at 21 m/s. Too high and the cut stops feeling aimed |
| `titan.ron: cortex_half_angle_deg` | did not exist | **110–130 per kind** | the nape may only be cut from inside that arc off his **back**. Too tight: a correct pass silently books a graze. Too wide: the titan is a floating bullseye and the approach angle is decoration |

**Roll back:** one line each in `assets/data/gear.ron`; the gate is one line per kind in
`assets/data/titan.ron` and **180.0 deletes it** without touching a line of Rust. The three raised
napes (`warden` 0.60→0.77, `lurker` 0.50→0.77, `bellower` 0.70→1.16) are one number each in the
same file.

### ASSUMPTION I worked under, and the thing it cost

**ASSUMPTION:** *the nape being cuttable only from behind is a RULE, not a coincidence* —
`docs/PLAN-GAME.md` §3.4 point 3 says so (*"a 360° sphere makes the titan a floating bullseye"*)
and it has been an open question since day one. I built it, because without it a longer blade cuts
a husk's nape **from straight in front of him** by 0.32 m, and I was not willing to trade that.

**⚠️ And it cost something I want you to know about rather than find.** Before today, a titan
turning to face you slowly ate your margin — 5 cm on a warden across one pass. It no longer does:
tracking on and tracking off both leave the same 0.95 m of usable air. The approach angle is now a
**cliff** (the nape shuts after he has come about **45–60°** round towards you) where it used to be
a **gradient**. A cliff is more learnable and less subtle. If it reads as sudden — as "it worked a
second ago and now it does not" — the fix is not the gate, it is that nothing on screen tells you
his nape has shut. Which brings me to:

### The three things I did NOT fix, and any of them could be what you actually felt

Your sentence is three words and it fits four different defects. This round measured one.

1. **You cannot see the thing that hits.** `FIND-127`: the blade is cast at 90° to where you are
   looking, and the camera sees ±45.7° — so on the eight of twenty-one ticks that can actually
   cut, the steel is **44° off screen**. The drawn pair is 0.93 m of model against what is now a
   2.00 m cast, and I made that gap 0.40 m worse today. The reference's answer to "where do I hit"
   is **feedback** — its crosshair turns red on the weak point. We have none.
2. **A slow swing does nothing, silently.** `gear.ron: min_speed_m_s` 8.0. Stand still, swing at a
   walking titan: the cast **lands**, the blade is inside the nape, and no message is written at
   all. That reads exactly like a broken hitbox.
3. **His hitbox, not yours.** `combat::strike::reaches` is a flat cone from the titan's origin with
   no arm geometry in it — a blow can land with the arm nowhere near you.

**If „hitboxen passen noch nich" was about any of those three, say which and the next round goes
straight at it.** If it was about the nape, this round is the answer and the three knobs above are
yours to move.

---

## Q-051 — the progression spine is built (F-120/F-121/F-122). Two decisions inside it are yours, and one of them is whether it should exist yet (2026-08-24)

**You asked for features — *„es fehlen SEHR viele features!"* — and twelve of the unbuilt prio-1
rows are `progress`.** So a sortie now earns experience, experience is a level, a level is a gear
budget, and a gear budget is a rank from E to S. `assets/data/progress.ron` holds every number;
`src/progress/career.rs` and `src/progress/gear.rs` hold the mechanics and no numbers at all.

### 1. This stands on the line the design draws, and you are the one who may move it

`docs/gameplay/pillars.md` and `docs/PLAN-GAME.md` §10 both say it plainly: **no meta system
before the Vector Gear gate is passed**, and `F-120`, `F-121` and `F-122` are named in that
forbidden list by their ids. The gate is a blind test against Attack on Titan Revolution with ten
human testers, and it has not been run.

`ASSUMPTION:` **your instruction outranks the plan**, so it is built — but built as the *smallest
honest version*: a number that grows, a budget that is earned, and a rank derived from it.
**Nothing that was forbidden downstream of it was started**: no skill tree (`F-123`), no perks
(`F-126`), no currencies (`F-140`), no loot tables, no pity counters (`F-128`/`F-142`), no
lineages, no shop. And **no gear point changes a gameplay number** — `game.ron` and `gear.ron`
are untouched, so the movement you are judging is exactly the movement you judged yesterday.

**Rollback point:** delete `assets/data/progress.ron`, the `progress:` field in
`src/data/mod.rs::GameData`, `src/progress/career.rs`, `src/progress/gear.rs`,
`tests/progress.rs` and the `xp` / `gear` fields of `save::Profile` (schema back to 1). The
career counters `F-200` already shipped are untouched by all of it.

### 2. Does the rank come from what you have EARNED or from what you have SPENT?

`F-121` says the rank *"rises through gear improvement"*. That is ambiguous the moment the points
exist but the screen to spend them does not (`F-125` loadouts is ⬜).

`ASSUMPTION:` **earned**. `rank_for` reads the budget the level hands out, not the points
currently allocated. Two reasons: a player who has not yet redistributed his points is not less
experienced than one who has, and a rank derived from spending would gate content behind a screen
that does not exist — which is the shape of bug that ships as "the game is locked and I cannot
find out why".

**Rollback point:** one line — `src/progress/career.rs::Career::of` passes `earned` to
`rank_for`; passing `gear::spent_points(&profile.gear)` instead is the other reading, and
`tests/progress.rs::f121_a_rank_begins_exactly_at_its_own_threshold` does not care which.

### 3. What the numbers currently say, so you can tell me they are wrong

- A won recruit skirmish with eight kills in four minutes is **400 xp**. A loss with the same
  kills is 280. Elite pays 2.2x, veteran 1.5x.
- **The first level costs 300**, so your first good sortie is a level. The whole ladder to 100 is
  513 838 xp — about 1 280 sorties at that rate. **⚠️ Untuned, and that last number is the one I
  would expect you to hate first.**
- A level is **1 skill point** (unspendable until `F-123`) and **2 gear points**. Level 1 starts
  with 6, level 100 has 204.
- Rank **D** at 12 gear points (level 4), **C** at 26 (11), **B** at 48 (22), **A** at 90 (43),
  **S** at 150 (73).
- **Nothing is gated.** `progress.ron: gates` is empty on purpose — locking a difficulty you can
  fly today, to demonstrate a rank, is the wrong trade while the movement is still the open
  question. One line in that file locks one door.

### 4. The four gear axes, and the one thing I could not test

`speed`, `control`, `power`, `endurance`, with your row's own two sentences as numbers: **speed
costs control, power costs endurance**. Points have diminishing returns, so the strongest build is
always a spread — measured, at every budget from 6 to 60 the best build puts at most 29 % of its
points on one axis, and the four builds that lead with the four different axes are within 0.2 % of
each other.

⚠️ **That is measured against a stand-in and I want you to know it.** "Strength" is a weighted sum
whose weights live in the same file as the design they judge (`docs/FINDINGS.md` FIND-155). It can
prove no build structurally dominates. It cannot prove a build is fun, and it cannot prove
anything at all until a gear point actually moves `game.ron` — which is the next step and is
**not** taken.

Related: `F-120` · `F-121` · `F-122` · `F-200` · `docs/FINDINGS.md` FIND-155 ·
`assets/data/progress.ron`

**The evidence run for all of it is `scripts/f120-career.txt`** — three separate processes against
one `DBT_SAVE_DIR`: one that migrates a save written before the curve existed, one that flies a
sortie and books what it earned, one that starts cold and reads the level back off the disk. The
numbers are in the commit message, not here.

---

## Q-053 — the drive has a weight now, and how much of it is yours. Also: mass is decoration in this game, and that is a decision (2026-08-26)

**You said, one minute after asking for the always-on pull:**

> *„zudem fühlt sich die gravitation nicht richtig an. oder die masse von dem character. es fühlt
> sich zu leicht an."*

**It is neither gravity nor the mass, and both were left alone.** `gravity_m_s2` is −20.0, twice
the real number. Your body's avian mass is real but **nothing in this game reads it**: every force
on the player goes through `Forces::apply_linear_acceleration`, which ignores mass on purpose so
that a tuning number never has to be divided by a weight.

What actually made you weightless is the shape of the rope drive. It chases a **velocity**:
`a = (v* − v)/ramp`. Unbounded, that replaces your whole velocity in the same ~0.2 s **however
fast you were going** — measured 167 ms to start a flight from nothing and **217 ms to turn a
70 m/s flight around**. Nothing resists you, because nothing was ever asked to.

**ASSUMPTION (the work continued under this):** the answer is a ceiling on the drive's
acceleration, `game.ron: vector.drive_accel_max_m_s2 = 250.0`, and **not** a gravity or mass
number. It is still 12.5 g — *„nicht so physics accurate aber mehr haptisch"* — but it is finite,
so a big change of velocity now costs time in proportion to its size: **233 ms** to start a
flight, **433 ms** to reverse one.

**What it costs, in your numbers, and this is the part to feel:**

| | before | now |
|---|---|---|
| ms to 90 % of the drive speed, from rest | 167 | **233** |
| angle off the rope after a quarter second of `W`, from 40 m/s across it | 2.2° | **8.0°** |
| ms to turn a full flight around | 217 | **433** |

So: **it is a little slower to start and it bends a little more before it straightens, and in
exchange it has weight.** If you want the old snap back, that is one line —
`drive_accel_max_m_s2: 875.0` is the game you played on 2026-08-25 to the digit. If it is still
too light, **lower** it (150 is noticeably heavy).

**Roll back:** `assets/data/game.ron: vector.drive_accel_max_m_s2`. Nothing else has to move —
the two test bands that were widened for it name the old number in their comment
(`tests/player.rs::f153_the_drive_is_felt_inside_a_quarter_of_a_second`,
`tests/vector_rope.rs::f153_under_drive_w_pulls_the_flight_onto_the_rope_line`).

### The three other numbers of this round, in the same place

- **`drive_speed_m_s: 70.0 → 52.0`** — *„man wird zu sehr rangezogen"*. This is how fast `W`
  hauls you along the rope. `f006-drive` measures 52.940 m/s after 1.5 s of `W` against 70.90
  before. **Bounded below by ~21 m/s** (a pendulum swing) and above by `max_speed_m_s: 75`.
- **`drive_idle_speed_m_s: 12.0` / `drive_idle_ramp_s: 0.35`** — the always-on pull. 12 m/s of
  closing speed, twice your run speed. ⚠️ **This is the one with a side effect worth knowing
  about:** on a rope that points straight up, holding **nothing** now climbs at 5 m/s, free and
  for ever. `Ctrl` is still 2.3× faster and it is what `F-005` is for, but *vertical is no longer
  something you have to pay for*. If that is wrong, the fix is one number
  (`drive_idle_ramp_s: 0.6` makes a vertical rope hold you exactly still instead of lifting you).
- **`drive_steer_pull_fraction: 0.35`** — how much of the inward pull survives while `A`/`D` is
  held. Lower = the sideways move wins harder. At 1.0 you get the 2026-08-25 game back, where the
  rope won.

**Related:** `docs/FINDINGS.md` FIND-172 · `Q-050` · `FIND-149` (amended: the *„wenn ich nichts
drücke wird nicht rangezogen"* observation was deliberately not followed) · `FIND-153`

---

## Q-054 — on the ground, `S` now lifts you into the free pull. „Spannen" and „immer ranziehen" meet there, and the meeting is yours (2026-08-26)

**Two of your own instructions, both still in force:**

> *„mit s »spannt« man nur das seil!"* (2026-08-12, `docs/NEXT.md` §1A req 7 — and, after
> playing it: *„aktuell wenn ich seil spanne und s drücke werde ich stark zum seil gezogen! das
> soll nicht sein!"*)

> *„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"* (2026-08-26, `FIND-172`)

**They do not collide in the air.** They collide in exactly one place: **standing on the ground
with a rope already taut.** Measured, tick by tick, on the market square with 81.2 m of rope up to
the wall gallery (`docs/FINDINGS.md` FIND-175):

```text
  t0..t7   S walks you backwards at 6 m/s, away from the anchor. Grounded. That is „spannen".
  t8       the taut rope answers, and it points 44° up: v.y 0.00 -> +1.86 m/s. You leave the ground.
  t10      MovementState::Tethered — and the always-on pull now has you, at 12 m/s, for free.
  t120     9.427 m closer to the anchor than you started.
```

**`S` itself is innocent** and that is measured, not argued: with the always-on pull deleted in one
key, the same held `S` **opens** the rope by **+8.911 m** — he walks his 12 m backwards and stays
`Grounded` for all 120 ticks. `A`, which never tautens the rope, reads `+0.882 m` with the pull and
without it. So `S` never touches the rope's length. What drags you is your own *„immer ranziehen"*,
one tick after the rope has picked you up off the street.

**The question is only this: should tautening the rope on the ground be allowed to launch you?**

1. **Leave it.** It is the shortest path from standing to flying that does not cost gas, and it is
   arguably a *feature*: hook a roof, walk back, get lifted. `F-005` says `Ctrl` is how a standing
   player leaves the ground; this makes `S` a second, free way.
2. **The free pull only starts in the air you did not get there by rope tension** — i.e. gate the
   idle winch on `MovementState::Airborne` from a jump or a fall, not from `Tethered`. More rule
   than the game has anywhere else, and hard to see while playing.
3. **The rope must not lift a grounded player at all** — the constraint clamps the body but never
   adds upward velocity while `Grounded`. That deletes the 2026-08-12 symptom at the root and
   costs the launch in option 1.

**ASSUMPTION: option 1 — it stays as it is, and the test was rewritten to say so honestly.**
`tests/input.rs::r7_s_tightens_the_rope_and_never_reels_it_in` (renamed from
`…_does_not_close_the_distance`, because under the shipped game it *does* close it) now asserts
that `S` is not a reel **on the axis `S` owns** — the run with the pull deleted — and keeps
`with_s >= with_w` on the shipped one. Nothing in the game changed for this.

**ROLLBACK if you want 2 or 3:** the only production line involved is the `in_the_air` arm of the
winch match in `src/player/locomotion.rs` (`RopeForceModel::Drive if anchored > 0 && (grant.reel_in
|| in_the_air)`). Narrow that predicate and the test's `with_s` number goes to roughly the
`s_alone` it already measures (`+8.911 m`); the two asserts that would then need re-aiming are
`with_s < s_alone - 5.0` and `with_s >= with_w`, both named in the test with the reason.

**Related:** `docs/FINDINGS.md` FIND-175 · FIND-172 · `Q-053` · `docs/NEXT.md` §1A req 7 ·
`F-005` `F-006`

---

## Q-056 — on the ground the rope **forbids** instead of **hauls**, and that supersedes Q-055's assumption (2026-08-26)

**🔴 This replaces the `ASSUMPTION:` written in `Q-055`.** That one said the pull would reach the
ground and *„become a lean, not a drag"* — applying while the player produces no ground movement of
his own. **That is not what shipped, and Q-055's rollback point no longer describes the code.**
Read this entry instead; `Q-055`'s question and its four *"why it is not obviously safe"* paragraphs
still stand and are still worth answering.

**What actually shipped, and why it is a third answer rather than either of the two Q-055 weighed:**

The always-on pull keeps its `in_the_air` gate, untouched. What changed is one line in
`ground_locomotion`: the walk the legs ask for has its **outbound half along the horizontal rope
direction removed** before `ground_step` ever sees it
(`player::locomotion::ground_desired_on_a_rope`).

**ASSUMPTION the work continues under:** on the ground a rope is a **constraint and not a motor**.
Walking *at* the anchor is the full `run_speed_m_s`, walking *around* it is the full
`run_speed_m_s`, walking *away* is the one component that disappears. `S` with the rope ahead is a
**stand**, not a slide-in.

**Why this and not the pull:** `ground_locomotion` **assigns** `LinearVelocity.xz` every tick below
the run speed (`ground_step` property 1: *"the result **is** `desired`"*). An acceleration handed to
avian is therefore worth `a·Δt` = **0.57 m/s** at the idle pull's own ceiling and is overwritten
before it can travel a metre — so *"let the pull reach the ground"* would have changed nothing
horizontally and would only have **lifted a hooked player off the floor** wherever his anchor is
above him (34.3 m/s² up against the file's 20 m/s² of gravity). That is the hub broken for a
mechanic nobody would have felt.

**What it costs, plainly:** a hooked player cannot back off. There is no *„das Seil spannen"* by
walking backwards any more — the rope is already taut by construction (`Rope::rest_m` is a `min`
over the distance ever reached). Releasing the hook is `Q`/`E` and one key.

**Rollback point:** `src/player/locomotion.rs`, `ground_locomotion` — the
`let desired = if rope_holds_the_legs && let Some(anchors) = …` block, five lines, and the
`rope_holds_the_legs` line above the loop. Deleting both takes the ground back to 2026-08-26
exactly. The gate is `vector.rope_force_model == Drive && vector.drive_idle_speed_m_s > 0.0`, so
`sed -i 's/drive_idle_speed_m_s: 12.0/drive_idle_speed_m_s: 0.0/'` switches it off without a
rebuild — the same key `tests/player.rs::without_the_always_on_pull` uses.

**What would settle it in ten seconds of play:** hook a hub wall and try to walk away. If being
unable to feels like a bug, the answer is the revert above. If it feels like a rope, keep it — and
then the open question is whether the ground should also *drag*, which is the half this deliberately
did not do.

**Evidence:** `tests/player.rs::f176_walking_backwards_on_a_rope_does_not_walk_away_in_the_real_app`
(30.00 → 30.00 m hooked, → 36.00 m with the hook released) ·
`scripts/f176-pull.txt` (0.000 m/s hooked, 6.000 m/s released, 6 asserts, exit 0).

### 🔴 AMENDED 2026-08-26 — this entry was right, and the round that wrote it did the thing it forbade

The paragraph *„Why this and not the pull"* above argues the always-on winch **out of the ground**
because an acceleration handed to avian there *"would only have **lifted a hooked player off the
floor** wherever his anchor is above him (34.3 m/s² up against the file's 20 m/s² of gravity)"*.

**A 250 m/s² brake was then put on the ground in the same round** — `rope_taut_brake`, in the `taut`
arm of `air_control`, with no `in_flight` gate. Seven times the term this entry refused, and it did
exactly what this entry said it would: hooked, standing, anchor above, one second of `A` took the
player from **5.858 m/s at 0.313 m to 50.597 m/s at 15.035 m** (`FIND-182`, reproduced on the
user's own binary). A tangential walk leaves its circle by 0.0075 m/s of forward-Euler residual and
the brake read that as an escape.

**So the entry's ASSUMPTION is unchanged and now actually holds:** on the ground the rope is a
**constraint and not a motor**, and `ground_desired_on_a_rope` — which never leaves the XZ plane and
therefore cannot lift anybody — is the **only** rope force a grounded player feels. The brake is
gated on `in_flight`, which costs nothing: an anchored player who is not standing is `Tethered`, and
`player::locomotion::in_flight` calls that flight by definition.

**Rollback point, unchanged and now two lines wider:** the `let desired = if rope_holds_the_legs …`
block in `ground_locomotion`, plus the `in_the_air &&` in the `taut` arm of `air_control`.

**Evidence for the amendment:** `scripts/f176-pull.txt` ACT 2b — exit **1** before, **0** after,
failing on `assert Height < 1 — measured 15.035` and `assert Speed < 7 — measured 50.597`.

**⚠️ And two comments in `src/player/locomotion.rs` used to name `Q-052` as this entry's rollback
point.** They name `Q-056` now (`FIND-184`). `src/data/mod.rs:665`'s `Q-052` is a different and
correct reference.

**Related:** `Q-055` · `Q-050` · `FIND-182` · `FIND-183` · `FIND-184` · `FIND-181` · `FIND-172` ·
`FIND-037` · `docs/NEXT.md` §3D · `F-004` `F-005` `F-006`

---

## Q-059 — the hub line stands there for ever; should it step aside once you know it?

**Context:** `F-177`, 2026-08-26. You wrote *„von der lobby aus muss man auch neue missionen
starten koennen!"*, and the cause was not a missing door but a silent screen — six working pads,
two working menu routes, and not one word about any of them (`docs/FINDINGS.md` FIND-187). The fix
is three amber lines at the top of the hub, `docs/images/f177-hub.png`:

```
Deploy: Ashgate Skirmish / Recruit
16 m ahead - walk onto the amber pad
Esc: Mission select
```

**The open question is how long it should stay.** As built it is up the whole time you are in the
hub — a permanent compass. The two alternatives are a **first-run hint** (up until the first sortie
has been flown, then never again) and a **fade** (up for a few seconds after every entry into the
hub). A permanent line is the most useful to somebody who has never played and the most
patronising to somebody who has flown fifty sorties, and it is the one piece of chrome that stands
over the place you walk in.

**ASSUMPTION:** permanent, because the failure being fixed is *"there is no way to know"* and a
hint that has already faded cannot fail safe. It costs nothing but three lines of screen in the one
phase where nothing else is happening, and the objective counter owns that same rectangle the
moment a sortie starts, so it is never in the way of a fight.

**What has to be rolled back if you want it otherwise:** only
`hud::hub_prompt::hub_prompt_text` and its caller. A fade needs one clock and one number
(`missions.ron: hub.prompt_s`), a first-run hint needs `save::Profile` to say whether a sortie was
ever flown — the element already returns `Option<String>` and hides itself on `None`, so both are
an extra arm in one pure function, not a rework. Nothing else in the game reads it.

**Why it matters today:** it is the first line of text this game has ever shown a player who is not
in a fight, and whatever it does will read as the house style for every hint after it.

---

## Q-061 — `scripts/f176-pull.txt` ACT 2 asserts `Speed < 2`, and §3D R1 says the opposite

**Asked 2026-08-27**, by the agent that built `Q-058` (the `DistanceJoint` on a `Drive` rope).
The run is **1 of 9 red** and this is the one red line. It is left red rather than repaired,
because repairing it is a decision about what the requirement *is*.

**The two sentences disagree.** `scripts/f176-pull.txt` ACT 2 encodes:

> **R1 · on the ground, hooked, holding `S`, the player does not move.** — `assert speed < 2.0`

`docs/NEXT.md` §3D R1 encodes the same user message as:

> **`S` must not cancel the pull.** The pull is unconditional; `S` is the one input that is
> allowed to fight it, and **even then it may only slow the approach, not reverse it into a
> retreat.**

The user's own words are *„wenn ich von seil weg gehe. also seil ist vorne und ich laufe zurück
werde cih nicht ran gezogen. **sonst werde ich ranzeogen!**"* — a bug report saying the pull
**should** survive walking backwards, not a request to be nailed to the floor.

**What the build does now**, measured over one second of `S` from a taut 36.723 m rope
(`tests/vector_rope.rs::f176_under_drive_walking_backwards_on_a_taut_rope_still_closes_on_the_anchor`):

| | after one second |
|---|---|
| `Drive` (shipped) | **35.637 m** — slowed to a crawl and still closing |
| `Pendulum` | 36.715 m — he stops dead; nothing pulls at all |
| `Drive`, rope let go | 39.619 m — a man walking away, the control |

So §3D R1 as `NEXT.md` states it is **met**, and the assert reads **7.630 m/s** and fails.
🔴 **The assert cannot tell the two apart**: an escape at `run_speed_m_s` = 6 and a haul at
`drive_idle_speed_m_s` = 12 are both "faster than 2". That is `CLAUDE.md` §6 rule 5's second
half in a new key, and the script harness cannot be fixed into telling them apart — `rope` is
the anchored **arm count**, not a length (`src/debug/script.rs`), so there is no distance metric
to ask with.

**ASSUMPTION the work continued under:** §3D R1 means what `docs/NEXT.md` says it means — the
pull is unconditional and `S` only slows it — so the build is right and the assert is the thing
that is out of date. **The line was NOT touched**, so nothing has to be rolled back if the
answer is the other way; the run simply goes green the moment somebody re-aims it.

**What has to change if he wants the other reading** (`S` = stand still on the ground): the
always-on pull would need a ground gate, i.e. `player::locomotion::air_control`'s winch branch
would have to stop firing for `MovementState::Grounded` — and that contradicts `FIND-172`
(*„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"*), which is his instruction
from 2026-08-26.

**What settles it in ten seconds of play:** hook something in front of you, stand still, hold
`S`. **If being drawn slowly toward it feels right, the assert goes; if being planted feels
right, the pull needs a ground gate.**

**Related:** `Q-058` · `FIND-172` · `FIND-192` · `docs/NEXT.md` §3D R1 · `F-005` `F-006`

---

## Q-079 — how much play must two ropes keep over the straight line between their anchors?

**Asked 2026-08-28 · `player::rope::hold_the_pair` · decided under an `ASSUMPTION`, work continued**

`B-013`/`FIND-191` is fixed: the reel may no longer ask for a pair of maxima no position in space
satisfies. The rule is exact and needs no taste — `L_left + L_right >= d_anchors` is the necessary
and sufficient condition for two spheres to meet. **The open part is one term on top of it.**

**At exactly `L_l + L_r = d_a` the two spheres are tangent and the feasible set is a single
point.** The player stands on the straight line between his two anchors, both constraint gradients
point along that line, and gravity pulls at a right angle to both — so nothing in the pair carries
his weight and the solver leaves the difference as sag. Measured over 84 cells, `Ctrl` held,
10 080 ticks (`tests/vector_rope.rs::f004_two_far_apart_anchors_…`):

| the reel is stopped at | worst arm past its own maximum | pinned ticks |
|---|---|---|
| `L_l + L_r = d_a` — the exact rule | **0.2810 m** (1.04 % of that arm) | 9 |
| `L_l + L_r = d_a + vector.min_rope_m` — as shipped | **0.0092 m** (0.11 %) | 3 |
| for scale: the `Ctrl`-free 288-cell matrix, a plain swing | 0.0050 m | — |

**ASSUMPTION the work continues under:** the margin is **`vector.min_rope_m` = 3.0 m**. It is a
stand-in. The honest home for the number is a key of its own — **`vector.two_rope_slack_m`** — and
`src/player/` does not own `assets/data/game.ron`. `min_rope_m` was taken because it is the
closest thing the file already says: *the shortest rope this game has*, so the rule reads *"two
ropes together always keep one short rope's worth of play over the straight-line span"*, which is
a sentence in units the file already speaks. Physically it is also the right shape — two real
ropes spanning a gap hang in a **V**, they do not stand as a horizontal line, and the margin is
exactly the slack that lets the V exist.

**What the user actually gets to decide**, and it is a feel question no test can settle: with a
3 m margin over a 56 m span the player sags roughly 9 m below the line between his anchors and can
swing in that bowl (3 pinned ticks in 10 080 — he keeps moving). With no margin he is frozen on
the line. He chose *„Beide Seile bleiben, du haengst fest — zwei Seile die sich widersprechen
halten dich."* **Both readings are „festhängen"; only one of them is „eingefroren".**

**Rollback point — ONE line**, `src/player/rope.rs::hold_the_pair`:

```rust
let allowed_m = (wanted[i].cur_m + wanted[j].cur_m - separation_m - min_rope_m).max(0.0);
//                                                                ^^^^^^^^^^^^ delete for the
//                                                                exact rule, restore for a key
```

**The diff `assets/data/game.ron` would need** (a main-head file; not made here) — in the
`vector` block, next to `min_rope_m`:

```ron
        // The play two ropes keep over the straight line between their anchors. At 0.0 the pair
        // can be pulled tangent, the feasible set collapses to one point, and the solver leaves
        // 0.2810 m of sag instead of 0.0092 m (`docs/QUESTIONS.md` Q-079, `docs/BUGS.md` B-013).
        two_rope_slack_m: 3.0,
```

and `hold_the_pair` then reads `data.game.vector.two_rope_slack_m` instead of `min_rope_m`. No
allow-list line is needed: `player -> data` already exists.

**Related:** `B-013` · `FIND-191` · `FIND-195` · `Q-058` · `F-004` · `F-005`

---

## Q-080 — you chose „beide Seile halten dich", and the shipped model does not deliver it

**2026-08-27**, raised by the adversary who verified `B-013`, and it is a **design-fidelity**
question, not a defect.

You were asked what should happen when both hooks are planted on far-apart anchors and you hold
`Ctrl`, and you chose *„Beide Seile bleiben, du hängst fest — zwei Seile die sich widersprechen
halten dich."* The fix delivers the first half exactly: **no arm ever exceeds its own maximum**
(384 cells / 98 640 ticks, worst excess **0.0244 m**, zero violations, zero lengthening).

**But the second half — actually hanging — is delivered by the model that is not shipped.**
Measured, two anchors 56.4–57.7 m apart, `Ctrl` held, 300 ticks:

| model | ticks pinned at 0.000 m/s |
|---|---|
| `Drive` — **what ships** | **0 – 3** |
| `Pendulum` — not shipped | 166 – 300 |

Under `Drive` the free always-on idle winch and gravity keep moving you whatever `Ctrl` does, so in
practice **you are essentially never stuck**. Both ropes stay and nothing tears — the ropes *hold* —
but the sensation you picked is a `Pendulum` sensation.

**ASSUMPTION the work continues under:** this is **fine and probably better**. Being pinned at
0.000 m/s in a movement game is the thing `FIND-191` was filed about; what you asked for was that
the ropes not break, and they do not. **No further work is done to make you stick.**

**What settles it in ten seconds of play:** plant both hooks on two far-apart anchors — two gantry
beams, or opposite roof edges — and hold `Ctrl`. If you expected to hang and instead keep drifting,
say so and the idle winch gets a stand-down in that geometry. If drifting feels right, this closes.

⚠️ **You cannot currently see which of the two states you are in.** Rope length is not drawn, not
logged, and not a script metric, so a stand-off and a violation look identical from the outside —
which is exactly why `FIND-191` lived undetected. **A rope-length readout would pay for itself**
and is not built.

**Rollback point:** the `in_the_air` arm of the idle winch in `player::locomotion` — one condition.

**Related:** `B-013` · `B-014` · `FIND-191` · `Q-058` · `Q-050` · `F-005` `F-006`

## Q-081 — being put back is a **teleport**, and I chose it over a wall you cannot fly over (2026-08-28)

You asked for both: *„unsichtbare wand + wenn man runterfaellt wegen bug teleport man zurueck!"*.
The wall is built and it holds sideways. What `B-016` measured is that it does **nothing** upward:
you clear its 200 m in under seven seconds on `W` + `Shift` for 0.72 % of a tank, and the climb has
no apex at all — one tank is worth 62 km (`FIND-203`). Raising the number does not help, because
every fence height has a solid invisible ring on its top face that you can stand on outside the map.

**ASSUMPTION:** the world is closed by the *recovery*, not by the wall — **outside the map's
footprint is out of the world at any height, and you are put straight back on the last ground you
stood on.** So if you fly up over the edge and drift outward, you get **teleported**, one tick after
you cross, with a `warn!` in the log. Normal play never touches it: the fence still stops walking,
swinging and a 75 m/s arrival, and the eleven legitimate falls and swings measured in
`tests/player.rs::f012_a_legitimate_fall_inside_the_map_is_never_recovered_at_any_height` (a 121 m
tower dive, a courtyard fall, three 60 m hook drops, five swings along the outside of the wall and
the far edge) send **zero** warps.

**What you might not like:** the teleport is instant and silent apart from the log. Flying straight
up 600 m over the middle of the district is still allowed and always will be — you are over the
world there, and you come down. It is only *outward* that ends the flight.

**The two alternatives I did not take, and why:**

1. **A taller fence.** 62 km of climb per tank says no height is enough, and each one adds its own
   standable ring at the top.
2. **A lid over the district.** It would end high flight, which is the movement this whole game is
   about, and the aim ray in `tests/vector_hooks.rs` fires level from 400 m up and needs open sky.

**If you want it different, the rollback points are small and named:**

- *"do not teleport me, just stop me"* → the fence needs a **top face that slopes**, or the panels
  need to become a closed dome; `src/world/bounds.rs::plan_fence` is the only file, and
  `player::recovery::out_of_the_world` keeps its `PastTheEdge` arm as the backstop.
- *"give me a moment before the teleport"* → a grace in **time**, not in space, in
  `recover_the_fallen`: N ticks outside before the warp fires. It must not become a grace in
  **metres** — that is a standable ring exactly that wide (`B-016`).
- *"put me somewhere else"* → `SafeGround` is the last ground you stood on; a fixed respawn point
  would be one line in `recovery::recover_the_fallen`.

**Related:** `B-016` · `B-015` · `FIND-203` · `FIND-204` · `F-012`

---

## Q-082 — a recovery that fails twice **gives up and leaves you standing outside the world**, and you see one red line and nothing else (2026-08-29)

**What happened.** The recovery of `F-012` teleported the player **1501 times in 25 s**, one per
tick, always to the same place, with sixty `warn!` lines a second behind it (`B-017`). The
destination was the fence's top face, so it never held; warping to it again was not a fix.

**ASSUMPTION (the work continued under it):** the recovery gets **two** attempts per episode and
then stops.

1. the ground he last stood on;
2. still out of the world one tick later, in the same place? Then that ground is not to be
   trusted, whatever recorded it — **his spawn point**, which nothing in play can poison;
3. still out after that? **Stop.** One `error!` line naming both attempts, then silence until he
   is back in the world.

**So what the player sees when it happens:** at worst two teleports inside two ticks (33 ms) and
then, if both failed, **he stands still, where he is, outside the world, with no HUD message at
all.** He can still move, look and hook; nothing else happens to him.

**Why I chose it over the alternatives.** A player standing still outside the world is a bug he
can see and report; a player teleported sixty times a second is a bug hiding behind its own
noise, and that is exactly what shipped. Step 3 should also be unreachable — the spawn point is
inside the world by construction — so the visible behaviour is "two ticks and you are home".

**What you might want instead, and it is yours to say:**

- a HUD line — *"recovered"*, or *"the world let go of you, tell someone"*;
- a hard respawn at the spawn point with the run reset, rather than leaving him standing;
- a kick to the hub.

**Rollback point if you disagree:** `src/player/recovery.rs::recover_the_fallen`, the
`if attempt > 2` arm, and `recovery::Recoveries`. The bound itself (two per episode) is what
`tests/player.rs::f012_a_recovery_whose_destination_does_not_hold_is_not_repeated_every_tick`
asserts; changing what happens *after* the bound does not touch the bound.

---

## Q-086 — how tall should the ground be? `terrain.step_m` is now 3.00 m and that is a taste call

**Asked 2026-08-29** (`FIND-210`). He said the map is flat and that he means the ground itself —
*„Das Gelaende selbst — Huegel, Terrassen"*. What was in the way was geometry, and that is fixed:
a flight now runs along the retaining wall instead of banking across the street, so `step_m` is
no longer capped at 1.80 m. **How far past it to go is not a geometry question any more.**

`ASSUMPTION:` **3.00 m per level**, which puts the district's peak at 12.00 m — one
`house_large` (11.50 m) — and the grade at 7.1 %. Chosen so the relief reads as *one storey of
ground* rather than as a cliff, and so the existing 10 % grade guard in
`tests/world.rs::f003_the_terrain_grade_stays_inside_what_the_flight_geometry_can_carry` still
holds without being touched.

Measured alternatives, same binary, same vantage — all of them legal, none of them free:

| `step_m` | peak | grade | note |
|---|---|---|---|
| 1.50 | 7.50 m | 3.6 % | what he called flat |
| 3.00 | **12.00 m** | 7.1 % | shipped |
| 3.60 | 14.40 m | 8.6 % | still under the grade guard |
| 4.50 | 18.00 m | 10.7 % | **trips the guard**; the guard would have to be re-derived, not raised |
| 6.00 | 24.00 m | 14.3 % | a 6 m wall in a 6 m street |

`ROLLBACK:` one number — `assets/data/maps.ron`, `ashgate.terrain.step_m`. It must stay a whole
multiple of `stair_rise_m` (0.30). Nothing else in the change depends on its value: the flight
geometry, the tests and the two scripts all derive from it. Screenshots of 3.6 and 4.5 were
taken and are in the round's scratch if he wants to look before deciding.

⚠️ Costs that come with it and do not depend on taste: **+89 % terrace blocks** (1236 → 2337),
and the district is now climbed by **finding the stairs** rather than by walking straight up any
edge — a 5.40 m flight at one corner of each 42 m cell instead of a bank along the whole of it.

---

