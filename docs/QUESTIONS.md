# QUESTIONS — decisions that are not mine to make

Updated: 2026-08-09

**Nothing here interrupts anybody.** Every question gets an `ASSUMPTION:` that the work
carries on under until the answer arrives — and the work runs past it instead of waiting
(`prompts/init.md` §2, §10). Answered questions move to the bottom, they are not deleted.

---

## Q-001 — Is there a store at all outside Roblox?

**Context:** the bible's fifth design pillar is „Der Store verkauft nur Aussehen" (*the store
sells looks only*: cosmetics, private servers, season pass), and `01_Spielfunktionen` contains
**5 rows of `Monetarisierung`** and **5 rows of `Live Ops`**. `prompts/init.md` §2 says about
it: Robux, season pass and the Roblox store **fall away in that form**, the *principle*
stands, and whether any of it happens outside Roblox at all is a product question.

**ASSUMPTION:** none of it gets built. The ten rows stay ⬜ in `docs/STATUS.md` and do **not**
disappear from `docs/features.ron` — they are recorded, not struck.

## Q-002 — Is 1 stud = 0.28 m the right conversion factor?

**Context:** backlog and bible count in studs (the Roblox unit), this project in meters
(`prompts/init.md` §3: 1 Bevy unit = 1 meter). The factor determines every distance in the
game.

**Evidence for 0.28** (the Roblox value), three independent cross-checks:

| Backlog | × 0.28 | matches |
|---|---|---|
| hook range 400 studs (`F-002`) | 112 m | `prompts/init.md` §1: „ein Haken fliegt 60–120" (*a hook flies 60–120*) |
| Ashgate District 2000 × 2000 studs | 560 × 560 m | mission arc 5–7 min |
| Titanwood 3000 × 3000 studs | 840 × 840 m | largest map |

**ASSUMPTION:** 0.28 m/stud. The conversion happens **once**, when a number is taken over into
an `assets/data/*.ron`; there are no studs in the code. If the factor is wrong, only RON
numbers change, no code.

### Addendum 2026-08-09 — the first cross-check has fallen away

The user delivered a size table and gave the anchor range in it **directly as 90 m**
(`assets/data/scale.ron: vector.anchor_range_m`). That makes the precedence rule apply: **a
direct figure in meters from the user beats any derivation.** `game.ron` now stands at 90 m,
no longer at 112 m.

That hits the justification of the factor at the root: the 112 m **were** the first of the
three cross-checks above. Calculated backwards, 90 m / 400 studs would give a factor of
**0.225 m/stud** — 20% below 0.28. Exactly one of two possibilities follows, and **which one
is the user's call, not mine**:

1. **0.28 keeps holding for everything else**, and the hook range is simply a game-value
   decision that has nothing to do with the backlog number. Then Ashgate (560 × 560 m) and
   Titanwood (840 × 840 m) stay as they are.
2. **The factor is too high altogether.** Then every number converted so far shrinks by 20%:
   Ashgate to 450 × 450 m, Titanwood to 675 × 675 m — and every map would have to be
   recalculated.

**ASSUMPTION (until answered):** possibility 1. The factor 0.28 stays for everything the user
has said **nothing** about; where he gives a figure in meters, his figure holds and the
conversion is not consulted at all. The two remaining cross-checks (Ashgate, Titanwood) carry
the factor alone — **that is thinner than before**, and the stage in `docs/conventions.md` §1
is to be read accordingly.

**To roll back:** the line `1 stud = 0.28 m` in `docs/conventions.md` §1 and every `size_m` in
`assets/data/maps.ron` that came out of a stud number. Not affected: everything from
`assets/data/scale.ron` — that comes straight from the user and was never converted.

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

## Q-006 — Music: own composer or a licensing library?

**Context:** bible 8.4 and bible 6.4 (risk: audio rights): **exclusively original or licensed
music**. Affects budget and schedule from P4 onwards.

**ASSUMPTION:** until answered there are only **sound recipes** (`tools/sound/*.py`,
reproducible) and CC0 placeholders under `assets/extern/` with a line in `ATTRIBUTION.md`. No
third-party music in the repo, not even as a placeholder.

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

## Q-009 — Does offscreen rendering really deliver an image on machine A? *(half answered, see the addendum)*

**Context:** `prompts/init.md` §14 allows as an exception that an image comes out of a render
target instead of a window — **but only once it is proven that it really does deliver a PNG on
the N100.** Claimed, it is worth nothing. Without that proof the ceiling on machine A is
**🟨**, and this project has **not one image** so far.

**ASSUMPTION:** it does not work until it is measured. Everything built on A stays 🟨 with the
note *"logic tested, pixels unseen — machine A"*. Recorded as a task in `docs/TODO.md`.

### Addendum 2026-08-09 — half of it is measured, and it is the half that decides

Measured on `[cachy]` with the new `--offscreen` (`src/debug/screenshot.rs`), not explained:

| Question | Result |
|---|---|
| Does a render-target image **without a window** deliver a PNG? | **Yes.** 1280x720, full scene, exit 0 |
| Also **without a graphics session**? | **Yes.** `env -u WAYLAND_DISPLAY -u DISPLAY` changes nothing about the result — winit is switched off, `ScheduleRunnerPlugin` drives the app |
| Is it reproducible? | **Yes, bit for bit.** Two runs, `sha256 = eb212dfe…` both times |
| Why did it not work before? | Because `--headless` set **`backends: None`** and therefore never even looked for an adapter. "No window" and "no GPU" were the same decision — that was the real reason, not a limit of Bevy. `bevy_render-0.19.0/src/lib.rs:501-506` shows it: the window is an `Option` when the renderer is built |

**What is NOT proven by this:** that the **N100 under debian** finds a wgpu adapter.
`[cachy]` has an RTX 3080 with a Vulkan driver; the measurement shows that **no window and no
compositor** are needed, not that every GPU plays along.

**The question therefore shrinks** from "does offscreen rendering work at all?" to **"does
machine A find a wgpu adapter?"** — and that is no longer a design question but a single
measurement on A: `cargo run -- --offscreen --script scripts/t006-shot-near.txt --ticks 110
--screenshot docs/images/t006-player-view.png`. If it comes out well, the ceiling on A rises
from 🟨 to 🟧. Until then the assumption stands for **A**; for **B** it is settled.

### ANSWERED 2026-08-09 — by measurement on A, not by a decision

**Machine A finds an adapter.** Measured on `[debian]` by two independent jobs in the same
session, each reporting it without knowing the other was looking:

```
AdapterInfo { name: "Intel(R) Graphics (ADL-N)", device_type: IntegratedGpu,
              backend: Vulkan, driver: "Intel open-source Mesa driver",
              driver_info: "Mesa 25.0.7-2" }
```

Four PNGs were produced on A in that session, every one of them **bit-identical over repeated
runs**: `p1-overlay.png` (`sha256 054aaeff…`, three runs, 625 728 B), `b001-anchor.png`
(`aaf52739…`), `f056-husk.png` (`ade7a6b7…`), `f050-states.png` (`8c20c551…`).

**This is the question that gated everything.** Without an adapter on A, nothing built on A
could ever have carried a picture, and 🟧 needs a picture — so the whole project would have been
capped at 🟨 on its own main machine. **The ceiling on A is now 🟧.** The assumption above is
withdrawn; the `T-006` note in `docs/features.ron` no longer applies and has been corrected.

**What is still not proven:** that a **window** run works on A. Only `--offscreen` was measured.
Nobody has seen this game in a window on this machine, and that is a different question
(`docs/umgebung.md`).

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

## Q-024 — German or English in the source? The instruction contradicts itself

**Context:** on 2026-08-09 the user wrote, **verbatim**:

> „programmiere alles in deutsch. filebenennung! code aber auch comments alle auf englisch!"

The sentence contains both: *„alles in deutsch"* (*everything in German*) and *„alle auf
englisch"* (*all in English*). Two readings are possible, and they lead to completely
different work on **7815 lines in 53 files**:

1. **Everything stays German.** Then "code aber auch comments alle auf englisch" is a slip of
   the pen and there is nothing to do — `docs/conventions.md` said exactly that until today
   ("one language: German, throughout").
2. **File names German, identifiers and comments English.** Then *„filebenennung!"* qualifies
   the preceding *„alles in deutsch"*, and the *„aber"* marks the contrast for everything
   else.

**Addendum 2026-08-09, shortly afterwards — the user sharpened it:**

> „wenn nicht sicher dann eher englisch!"

*(if in doubt, rather English)*

That **confirms** reading 2 and answers the question at its core. It stays here anyway,
because it set a tiebreaker that goes beyond the wording: **German holds only where it is
named explicitly** (file names, commit messages, `docs/`); every doubtful case goes to
English. Two cases moved because of it that stood in German in the first version of this
question: log and HUD output, and the metrics of the script driver (`speed|tempo`,
`height|hoehe`, `titans|titanen` — the German second names are dropped).

**ASSUMPTION: reading 2.** Three reasons, each weak on its own, together carrying:
*„filebenennung!"* stands with an exclamation mark **directly behind** "alles in deutsch" and
reads as its qualification; the *„aber"* before "comments" only makes sense at a language
switch; and reading 1 would be an instruction that changes nothing — the user does not write
an instruction so that everything stays as it is.

The border was written out in [`docs/conventions.md`](conventions.md) §4. In short: files,
folders and module names German · types, fields, functions, comments, test names and RON keys
English · commit messages, `docs/` and everything a **player** reads (HUD, log lines) German.

**To roll back:** exactly one commit. The migration runs as **one** conversion, not
step by step — half-German code would be worse than either of the two answers. A `git revert`
of that one commit restores the German state completely; after that only
`docs/conventions.md` §4 and this question have to be turned back. Everything that arrives
**new** between migration and answer follows reading 2 and would have to be brought along
under reading 1 — which is why this question stands here near the top, not at the bottom.

**Addendum 2026-08-09, later the same day — the user removed the last limit:**

> „es sootlle im bestfall nirgendwo deutsch sein! alles auf englisch!"

*(ideally there should be no German anywhere — everything in English)*

That sentence overrides both earlier ones and, with them, the whole border above:
**everything becomes English** — file names, identifiers, comments, RON keys, documentation,
tool output, log lines, the commit norm. German remains only in the user's own documents
(`prompts/`, `gameplay/`), in the quotations taken from them, and in the git history, which is
a record and not documentation. The section about the two languages in
[`docs/conventions.md`](conventions.md) §4 is therefore **deleted rather than translated**;
the one rule that replaces it is: *everything is English.*

The question is thereby **answered by the user himself** and stands here only as the record of
how the answer came about. What the migration leaves open is a matter of wording, and it
belongs to the user rather than to me:

- **the four stage names** `Unbuilt · Built · Proven · Accepted` — ✅ is the user's marker
  (rule 1), and the migration renames it;
- **whether a German-relapse checker ships at all**: after the migration, German in `src/`,
  `assets/data/`, `tools/` and `scripts/` is itself a violation, and nothing checks that
  today.

**To roll back:** still exactly one commit — the migration commit itself.

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

---

## Answered

*(nothing yet — the user's first answer comes here, with a date)*
