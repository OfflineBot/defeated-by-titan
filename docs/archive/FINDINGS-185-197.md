# FINDINGS — archive, `FIND-185` .. `FIND-197`

Updated: 2026-09-01 · Stage: 🟧 (measured, and moved to keep the live file readable)

> **Why this file exists.** The live `docs/FINDINGS.md` is capped at the newest 20 entries, on the
> rule that a queue file past ~150 kB gets archived. **Nothing here is retired or wrong.**
> Grep for the id; never open this whole.

## The index

- **FIND-185 — `scripts/f004-towers.txt` leg 3 does not lose a rope, it **never fires one**: `NothingInRange — open sky`**
- **FIND-186 — 🛑 §3D was attempted twice, refuted 2/2 both times, and **reverted**. The assumption is wrong, not the execution.**
- **FIND-187 — the hub had six working doors and said nothing about any of them**
- **FIND-188 — the hub line named one door and the walk opened another, on a 7° band between them**
- **FIND-189 — a `--script` run cannot assert on anything the HUD says, so HUD scripts are evidence and not gates**
- **FIND-190 — the hub line stopped predicting the walk, because the walk is 25 mm wide**
- **FIND-191 — the reel can ask for two lengths no position in space satisfies, and it is older than the joint on `Drive`**
- **FIND-192 — `assert speed` cannot tell an escape from a haul, and it cost `scripts/f176-pull.txt` its only red line**
- **FIND-193 — the hub line's fourth refutation: **a promise that could only be read while it was false, and a pointer that ranked by the wrong thing****
- **FIND-194 — a derived constant carried into a test BY HAND is a stale literal with extra steps, and gravity −32 found five of them**
- **FIND-195 — with the ratchet held, 64.4 % of ticks ask for two rope maxima no position satisfies, and a cross-model differential is NOT a control for it**
- **FIND-196 — holding `W` closes LESS distance than holding nothing, and it predates the joint**
- **FIND-197 — the anchor field is deleted: 2 528 lines, six RON keys and one allow-list edge for a system the hook never called**

---

## FIND-185 — `scripts/f004-towers.txt` leg 3 does not lose a rope, it **never fires one**: `NothingInRange — open sky`

**2026-08-26 [offlinebot].** `f004-towers` is red at HEAD and still red; the commission asked for its
failure **identity**, not a repair, and the asserts were not touched.

`line 266: assert Rope == 1 — measured 0.000` is leg 3 of the swing chain. The hook log says what
happened, on **both** binaries, at the same tick:

```
MARK t=720 f004-leg-2
hook Right of player 1 found no anchor: NothingInRange — nothing within hook range on that
     line — open sky (t=721)
hook Left  of player 1 let go: Released (t=723)
MARK t=776 f004-leg-3
```

Legs 1 and 2 anchor (body 109 at (0, 59.42, 231.50) at t=544; body 112 at (0, 56.96, 196.50) at
t=666). **Leg 3's shot finds nothing** — the script's own comment already says *„leg 3 — and this is
where the lane runs out"*. So `assert rope == 1` is not a rope that was lost mid-swing; it is a shot
that missed, and the two asserts after it (`Height < 19.1 — 40.581`, `Speed > 35.6 — 29.528`) are
**measuring a ballistic fall**, not a swing. The identical `NothingInRange` at t=721 appears on the
pre-round binary, so this round did not cause it.

**The causal chain is the flight height, and it is shared with `scripts/w5-lane.txt` line 165 and
`scripts/f025-chain.txt` line 111** (the same three-legged lane, the same message). Legs 1 and 2 now
arc **higher** than the brackets the script was cut for — `Height < 31.9 — 39.738`,
`Height < 26.9 — 45.641` — so by leg 3 the player is at 40 m instead of 19 m and a shot at pitch 39°
goes over the beams. **Re-aiming the script is a separate job and it needs the user**: the brackets
encode a flight shape that three rounds of rope work have changed, and rewriting them to fit is
exactly the move `docs/NEXT.md` §3 warns against (*"do not loosen that assert"*).

**Regression net, 20 scripts that fire a hook, before/after this round** (the four the last
commission named fire none — `grep -c '^hook '` is 0 in all four):
**no exit code went green → red**, and no assert changed from holding to failing. `f176-pull` moved
**1 → 0**. Every other red script keeps the identical set of failing lines; the numbers move by
0.3–1.7 m and several move *toward* their brackets (`f025-chain` line 114 `Height < 30 — 30.087`
now holds; `f004-towers` line 259 `Speed > 32.7` 25.712 → 25.960; line 269 29.005 → 29.528).

**⚠️ And a naming debt found on the way, not fixed:** the §3D tests and `scripts/f176-pull.txt` are
prefixed `f176`, but **`F-176` is "Barrierefreiheit"** and `F-177` is "Grafikeinstellungen"
(`docs/features.ron`). The prefix names a feature that has nothing to do with the rope. New tests in
this round kept the established `f176_` prefix rather than inventing a second wrong one; renaming
the family is a mechanical job for whoever owns the norms pass.

**Related:** `FIND-182` · `FIND-183` · `docs/NEXT.md` §3 · `F-004` `F-025`

---

## FIND-186 — 🛑 §3D was attempted twice, refuted 2/2 both times, and **reverted**. The assumption is wrong, not the execution.

**2026-08-26 · [debian] · the stop condition in `CLAUDE.md` fired for the first time**

> *"Stop when the DoD is met, when the limit is reached, or when **the same hypothesis has failed
> twice** — then it is not the execution that is wrong but the assumption."*

Two independent rounds built `docs/NEXT.md` §3D (the rope always pulls; `A`/`D` steer without
escaping). Four independent adversaries attacked them. **All four refuted, all at high
confidence.** Both attempts are reverted; `src/player/locomotion.rs`, `src/player/rope.rs` and
`tests/player.rs` are back at HEAD. The second attempt is saved at
`/tmp/claude-1000/-home-offlinebot-Documents-defeated-by-titan/e29ae3a7-3fb1-4023-b5fd-83655fd732f4/scratchpad/rope-3d-attempt.patch` (1705 lines).

### Attempt 1 — constrain the drive against the **aggregate** rope axis

`rope_axis = unit(Σ r̂ᵢ)`. Refuted: that bounds `d(Σdᵢ)/dt`, **not** `d(dᵢ)/dt`. With two
anchors 120° apart the player flies `40.00 → 69.33 m` **away** from an arm — the user's symptom,
unfixed. Also shipped a **194.9 m/s² vertical elevator on a strafe key** (19.9 g against gravity's
20) by subtracting along the 3-D axis, and a **250 m/s² taut-brake with no ground gate** that lifted
a walking hooked player to **50.6 m/s at 16.5 m**. (`FIND-182`, `FIND-183`.)

### Attempt 2 — constrain **per arm**, by iterative projection with an exact net scale

Refuted harder, and the new failures are worse than the old ones:

| what | measured |
|---|---|
| `held_off_every_rope` returns `Vec3::ZERO` | **25.8 %** of ticks with a lateral key held (13 931 / 54 000); a legitimate command up to the full **36.00 m/s** thrown away on 12.2 % |
| two hooks, on the ground | **0.000 m/s at all eight yaws** — the player cannot move at all |
| one hook, in the hub | **`A`, `D`, `S` all 0.000 m/s**; only `W` survives |
| the ground elevator | **not gone** — it fires at **+1.35 s** and the round's own new fixture samples at **+1.0 s** |
| "zero newly-failing assert lines" | **18** newly-failing lines across the 20 hook-firing scripts |
| `Q-057`'s payout | claimed 0.500 m/s sustained; the running game shows **0.002 m/s** |

The annihilation is a **`0.0 / negative = -0.0`** in the net scale: every floor is `min(0, ·)`,
which is *exactly* `0.0f32` whenever the player is closing, so `scale = floor/along` zeroes the
whole command whenever the sweeps fail to converge — and they fail because the residual correction
(~1e-6) is below the ULP of a 36–52 m/s vector.

### 🔴 The three lessons, and the third is the expensive one

**1 · A constraint that is satisfiable by standing still is satisfied by standing still.** The
round's own defence — *"mutually unsatisfiable floors cannot occur: every floor is `min(0,…)`, so
standing still is in the set for any geometry"* — **is the defect stated as a proof of safety.**
`Vec3::ZERO` being always feasible is exactly why a solver that scales toward feasibility lands on
it. **Never scale a command toward a constraint set that contains the origin; subtract the
violating component instead.**

**2 · The fixture moved to cover the bug and stopped one tick short of it.**
`f176_a_lateral_key_produces_no_vertical_acceleration_at_all` asserts **only** `a.y` — and
`a.y` of a zero vector is zero, so it passes on all **92** of its own geometries where the drive
returns `ZERO` while a 29.49 m/s strafe was legal. Its sibling asserts
`worst_lift <= accel_max_m_s2`, which `clamp_length_max(accel_max_m_s2)` guarantees on the
previous line — **an unfalsifiable assertion.** And the new ground script samples at +1.0 s while
the elevator starts at +1.35 s. **Three separate guards, each shaped exactly like the thing it was
written to catch, and each blind to it.** This is `FIND-103`'s family again: *a test that asks the
screen and the function the same question passes when both are wrong.*

**3 · Two rounds spent ~1.27 M subagent tokens re-deriving a solved problem.**
Multi-constraint velocity projection is what a physics solver does, and **this project already has
one**: `RopeForceModel::Pendulum` builds an avian `DistanceJoint`, and avian solves n
simultaneous constraints correctly, including the two-anchor case both attempts got wrong.
`Drive` deliberately builds **no joint** (`FIND-152`) — which is why every one of these
constraints had to be hand-rolled in a velocity target, twice, badly.

### The hypothesis for attempt 3 — **stop hand-rolling the constraint**

**Give `Drive` a real `DistanceJoint` with `limits.max = rest_m`** and let the solver hold the
length, so the drive only ever has to supply a *direction and a speed* and never has to prove a
geometric invariant. That is the one shape neither attempt tried, it is the shape `Pendulum`
already uses in this codebase, and it makes R4 (the ratchet) a one-line `limits.max` update
instead of a new mechanic.
⚠️ **It is not free:** `FIND-152` removed the joint from `Drive` for a reason, and that reason
has to be read and answered before this is attempted — **read it first.**

**READ, 2026-08-26. The answer is: the precondition does not block attempt 3.** `FIND-152`'s
sentence is

> *"`Drive` builds no `DistanceJoint` at all. **Not `JointDisabled`**: `combat::hitstop::advance`
> removes that marker from every joint of a body when a freeze lifts (`src/combat/hitstop.rs:295`),
> so a rope disabled for a **model** reason would come back to life after the first hit the player
> takes, mid-flight, and nothing would say why."*

That is an argument against a **disabled** joint — a joint switched off by a marker that another
domain silently strips. It says nothing against an **enabled** joint whose `limits.max` is the
ratchet's `rest_m`, which is precisely what `Pendulum` already runs in this codebase. The hazard
`FIND-152` names cannot fire on a joint that is never disabled.

🔴 **The one real interaction to handle instead**, and it is not the one the finding warns about:
`player::rope::shorten_ropes` queries `(&Rope, &mut DistanceJoint)`, and a jointless rope *simply
does not match* — which is why `B-005`'s ratchet is inert under `Drive` today. **Give `Drive` a
joint and `shorten_ropes` starts running on it**, so `Ctrl`'s reel and the new `rest_m` become two
writers on one `limits.max`. **One field, one writer** (rule 3) has to be settled *before* the
joint exists, not after — decide whether `rest_m` IS `limits.max` (one writer, the ratchet, and
`Ctrl` simply lowers it) or whether the two are separate quantities.
⚠️ And `f149_under_drive_a_hooked_player_who_presses_nothing_is_not_held_up_by_his_rope` asserts
**2.499 m of fall against `Pendulum`'s 0.000** with the anchor straight overhead. A `limits.max`
joint at the bite distance keeps that test green *only* while `rest_m` starts at the bite distance
and the joint is a **maximum** and not a fixed length. **That test is the gate on attempt 3** — it
is the one that tells a rope apart from a leash, and it already exists.

### What is NOT in doubt

The user's requirement stands, and `scripts/f176-pull.txt` is kept **red on purpose** to say so.
The diagnosis of the original bug is also confirmed twice over, in the running game: `A+W` at a
51.55 m anchor goes to **96.75 m** on HEAD. *„ich gehe in einer geraden linie weg von dem
ankerpunkt"* is real, it is still there, and it is still worth fixing.

**Related:** `FIND-181`–`FIND-185` · `FIND-152` · `FIND-103` · `Q-056` · `Q-057` ·
`docs/NEXT.md` §3D · `F-005` `F-006`

## FIND-187 — the hub had six working doors and said nothing about any of them

**The user, 2026-08-26:** *„von der lobby aus muss man auch neue missionen starten koennen!"* —
and with „lobby" the walkable hub, not a menu (*„mit lobby mein ich auch rumlaufen. also eher eine
art hub."*, 2026-08-24).

**Every door he asked for already existed and was green that day.** Six pads deploy their own
sortie (`mission::hub::deploy_on_contact`, `scripts/f175-loop.txt` — 19 asserts, exit 0);
`Esc → Mission select → Deploy` and `Debrief → To the lobby → Deploy` are held by
`tests/menu.rs::f175_the_lobby_deploys_the_sortie_it_shows` and
`::f175_redeploy_flies_the_same_sortie_again_and_through_the_hub`. **The symptom was real
anyway**, and the cause was measured before anything was changed (`docs/images/f177-hub-control.png`
is that state):

1. **The pads say nothing.** Six `Block`s, all `amber` out of `maps.ron`, all `radius_m: 3.0` —
   decoded identical to 1/255 and to the pixel between two of them. No label, no icon, no mesh
   above ground; and `grep -rn 'Text2d|Text3d|billboard' src/` is empty, so a sign on a pad is not
   a thing this engine can draw at all.
2. **The HUD said nothing, and its one candidate was switched off twice.** `hud::objective` is a
   real amber banner at top centre, spawned `Display::None`, `None` for `MissionPhase::Hub` — and
   `None` before that anyway at `let tally = tally?;`, because `open_hub` despawns the mission
   entity on the way in.
3. **The one walkable door was behind him.** At the cold start the three skirmish pads — including
   the one `missions.ron` calls *"the door you find without looking for it"* — stand at
   150.7° / 180° / 209.3°. The only pad in the frustum is one corner of the parcours slab,
   0.66 % of the frame.

So the whole screen carried two glyphs, `Q` and `E`, and he concluded exactly what it told him.
**Same family as FIND-178** — the gap between what the game does and what the screen says — with
the sign flipped: there the HUD promised a snap that did not exist, here it refused to mention a
door that did.

**The fix is one HUD element and no re-routing** (`src/hud/hub_prompt.rs`, `F-177`). Three amber
lines, top centre, only in `MissionPhase::Hub`, in the rectangle `objective` already owns and can
never share (that one is `None` in the hub, this one everywhere else):

```
Deploy: Ashgate Skirmish / Recruit
16 m ahead - walk onto the amber pad
Esc: Mission select
```

**It introduces no number.** Geometry and size are `objective`'s constants; the two names are
`template.name` / `level.name` out of `missions.ron` (the strings the lobby's own rows are built
from); the key's row is `menu::pause::MISSION_SELECT_ROW`, the button's own constant; the four
bearing words divide the circle by their own count, so ±45° and ±135° fall out of `BEARING_WORDS`
rather than being chosen. Nothing was added to `assets/data/`.

**Measured, and with the control that deletes the thing being measured:**

| frame | amber px in the banner band `x 384..896, y 15..95` | bbox | mean sRGB |
|---|---|---|---|
| `docs/images/f177-hub.png` — cold start | **2949** | x 410..869, y 24..80 | (239, 202, 91) |
| `docs/images/f177-hub-turned.png` — after `look 180 0` | **3123** | x 447..832, y 24..80 | (238, 201, 89) |
| `docs/images/f177-hub-control.png` — the same tick with `hub_prompt::update_hub_prompt` **unregistered** | **0** | — | — |

The frame and its control differ in **5536 pixels and in nothing else**: the whole difference is
inside `x[409..870] y[23..80]`. `y max 80` against a keep-out box that starts at 288 px.
With that one line taken out, all four whole-app tests go red with the original message —
*"nothing on the hub screen names a door … the whole HUD says: ["F3", "Q", "E"]"*.

**Two things this cost that are worth keeping.**

*The first rule for which door to name was wrong, and a test caught it.* "The nearest one you are
facing" named the **breach** pad 51.3° off at 12.8 m while the player was pointing straight at the
recruit pad at 16 m. A compass answering a question the player did not ask is worse than none, so
the rule is now **most nearly ahead**, then nearer, then the pad's own name. It also settles a real
tie: from the spawn point breach `(-10, 0, 8)` and parcours `(-10, 0, -8)` are the same distance to
the float, so a nearest-first rule would have been decided by archetype iteration order — the trap
`deploy_on_contact` documents for overlapping circles.

*A line that redraws per metre must not log per metre.* The first version logged whenever the text
changed, which is six lines a second while walking. The log key is now the sentence **with the
number sliced out** — door and way — so `scripts/f177-door.txt`'s log is eight lines that read as
*the screen said X → the player did what X said → the phase moved*.

**What is still true and was not fixed here:** the six pads remain six identical unmarked amber
squares — a player cannot tell recruit from elite from breach from across the square, and only the
one he is pointing at is ever named. That is pad-marking work in `mission::hub` plus `missions.ron`,
and it is a different job from making the door findable at all.

---

## FIND-188 — the hub line named one door and the walk opened another, on a 7° band between them

**2026-08-27 · `src/hud/hub_prompt.rs` · found by an independent adversary against a real HEAD
build, fixed the same day · the `FIND-178` family, one layer up**

`F-177` shipped a three-line amber prompt in the hub — *Deploy: X / Y · N m ahead - walk onto the
amber pad · Esc: Mission select* — and it was measured, photographed and cheap. It was also
**lying**, and none of its five tests could see it:

```
cold --hub start, no warp, look 140 0, key W 3.0
  the screen:  Deploy: Ashgate Skirmish / Veteran   18 m ahead - walk onto the amber pad
  the game:    deployment: "breach" at "recruit" — a player is 3.0 m from the pad
```

A different template **and** a different difficulty than the line named, doing exactly and only
what the line instructed.

### The cause: two rules answering different questions, with no test between them

| | question it answered | where |
|---|---|---|
| `hud::hub_prompt::pick_door` | *which door am I most nearly pointing at?* — smallest \|bearing\|, distance only as a tiebreak, no range limit, no idea what is on the path | `src/hud/hub_prompt.rs` |
| `mission::hub::deploy_on_contact` | *which pad am I standing in?* — nearest within `radius_m` | `src/mission/hub.rs:296` |

At yaw 140 from the spawn point the breach pad is 12.8 m away at +11.3° and the veteran pad is
18.5 m at −10.6°: **veteran wins the name by 0.7° of bearing, breach wins the sortie because it is
the one the walk crosses.** The bearing rule was itself a deliberate repair — it cured a 51° case
where the line named a pad the player was not looking at — and it is documented and enshrined in a
test. It cured one case and created this one.

**The band sits exactly between the two doors a player turns to compare.** Turning left from the
cold spawn sweeps 0° → 45° (Rookery ahead) → 116° (Breach ahead) → **136–142 (wrong)** → 145
(Veteran).

### The size of it, measured rather than argued

A ±20 m / 0.5 m / 1° grid of the hub floor around the cold spawn point — 81 × 81 × 360 =
**2 361 960 stances**, of which **918 719 start a sortie** if you hold `W`:

| | stances that start a sortie the line did not name | that change door mid-walk |
|---|---|---|
| bearing rule (HEAD) | **299 746 (32.6 %)** | 563 394 |
| walk rule (fixed) | **0** | **0** |

### The fix: the line names the pad the walk reaches, because the pad is the authority

`deploy_on_contact` decides which sortie starts, so the HUD is the thing that must agree with it —
never the other way round, and `src/mission/hub.rs` was not touched. A pad is **on the path** when
its centre is within its own `radius_m` of the ray the player faces (in three dimensions, the same
distance the pad measures) and the crossing is not behind him; the earliest crossing wins, then
the nearer, then the pad's own name. Only when **nothing** is on the path does it fall back to
smallest \|bearing\| — which is what keeps the 51° cure, and keeps it for a better reason: at
`look 180 0` the recruit pad is not merely more nearly ahead, it is the only one walking forward
can reach.

Two properties fell out of it that the bearing rule could not have:

- **No mid-walk flip is expressible.** A straight walk follows one line, so `perp` never changes
  and the pad entered first stays the pad entered first. 563 394 → 0.
- **A knife edge that has no answer now says so.** A pad the walk enters and leaves again inside a
  single simulation step (0.1 m = `run_speed_m_s / simulation_hz`) may or may not be sampled by
  `deploy_on_contact` — a coin flip decided by where the player's acceleration put the ticks.
  There the line names both doors and **no distance**: *Deploy: A / a  or  B / b · the walk clips
  an edge - turn to pick a door*. 33 of 918 719 stances (0.0036 %). A line that hedges is honest;
  a line that guesses is `FIND-178` again.

### Two more defects the same adversary found in the same element

- **The log was a false evidence artifact.** The dedupe key sliced the metre count out of the
  sentence but the whole sentence was logged, so when `open_hub` warped the player the displayed
  distance moved 14 m with no new log line — the log read `27 m to your left` while the screen was
  photographed reading `13 m`. Now the logged text **is** the key: the door and the way, cut from
  the sentence on screen, with no number in either.
- **One bad pad blanked the whole line.** `update_hub_prompt` picked the best pad and only then
  did `templates.get(…)?`, so a single pad naming a mission that is not in `missions.ron` emptied
  the element instead of falling through to the next door. The filter now runs **before** the
  choice.
- And `bearing_word(45.0)` returned `"to your right"` (`.round()` ties away from zero) while the
  test oracle's `rel.abs() <= 45.0` said `"ahead"` — two implementations disagreeing on a boundary
  that had no test. Both now say the outer word at exactly ±45° and ±135°, and it is asserted.

### 🔴 And the lesson, which is the third time in three rounds

**The fixture is where it hid.** `scripts/f177-door.txt` deployed twice and used `look 180 0` for
both — *the single most honest yaw in the hub*: recruit dead ahead at 0.0°, nothing else within
40°. Every rule anyone could write agrees there. It never crossed the band, and its "second
sortie" was the same mission a second time. **A fixture aimed at the one stance where every
candidate rule agrees measures nothing about the rule.** It now deploys from two stances inside
the band and into two different missions.

The gate is `tests/hud.rs::f177_no_stance_in_the_hub_names_a_door_and_opens_another` — a sweep,
not an example, with a marching oracle that applies `deploy_on_contact`'s own pad test step by
step rather than re-solving the equation the code under test solves (`FIND-103`).

---

## FIND-189 — a `--script` run cannot assert on anything the HUD says, so HUD scripts are evidence and not gates

**2026-08-27 · `src/debug/script.rs` · foreign territory, reported not fixed**

Measured while closing `FIND-188`: with the walk rule disabled in one line, `scripts/f177-door.txt`
put `Deploy: Ashgate Skirmish / Veteran` on the screen directly above `deployment: "breach" at
"recruit"` — the whole defect, in the log, twice — and **all 13 asserts held and the exit code was
0.** The same one-line break takes `tests/hud.rs::f177_no_stance_in_the_hub_names_a_door_and_opens_another`
from 0 to 299 746 lying stances.

`assert` knows `phase`, `kills`, `titans`, `height`, `gas`, `speed`, `rope`, `health`, `blades`
and `sharpness`. **Nothing it knows can see a string.** So every `scripts/*.txt` written to
demonstrate a HUD element is a picture a human reads, never a thing that can fail — and the two
are easy to confuse, because such a script *does* print the element's own log line right next to
the game's.

**What would fix it** (`src/debug/script.rs`, not this hand's): a metric that reads the HUD text,
e.g. `assert hud contains "Ashgate Breach"` — a string comparison verb rather than a float one. It
would make `f177-door.txt`, and every future HUD script, a gate. Until then, **a commission that
asks a script to prove a HUD claim has to say which Rust test is the actual gate.**

## FIND-190 — the hub line stopped predicting the walk, because the walk is 25 mm wide

**2026-08-27 · `src/hud/hub_prompt.rs` · closes `FIND-188`'s repair · measured, not argued**

`FIND-188` was *"the line named one door and the walk opened another"*, and it was repaired by
making the line **model the forward walk**: a pad is on the path when its centre lies within
`radius_m` of the ray the player faces. Two independent adversaries broke that in one round, on
**one root cause with four faces**.

**The root cause: `pick_door`'s `perp2` was a THREE-DIMENSIONAL miss distance**, so it was a
function of the player's `y` — and `y` moves constantly.

| face | measured |
|---|---|
| the hub landing | `mission::hub::open_hub` warps every player to `missions.ron: hub.spawn_m` = `(0, 2, 0)` on the cold `--hub` start **and after every sortie**. From `y = 2.0` at yaw 140 the breach pad is 2.518 m off the ray horizontally and **3.216 m in 3D**, over `radius_m` 3.0 → "not on the path" → the bearing fallback → the wrong door. Crossover height 1.631 m, the fall 11.5 ticks: `--ticks 2/4/6/8/10` → *Veteran*, `12/14/16/20/30` → *Breach*, in the running game, to the tick |
| the walk bob | 0.050 / 0.086 m every tick or two — **36 mm** — flips "on the path" on alternate ticks at a tangency. At `(18, 0.06, 2)` `look 150`: screen *Ashgate Skirmish / Elite*, game `escort`/`recruit`, 5 runs of 5 |
| the jump | `game.ron: jump_speed_m_s` 6.5 against gravity −20 → a 1.056 m apex under any `Space` |
| the props / the angled walk | the ray sees through all **73 solid props** in the muster yard (3.1 % of 903 stances, 40 % at the work corner) and models a straight walk the player does not take: **101 of 152 (66 %)** `W`+`A` / `W`+`D` stances started a different sortie than the line named |

**And the case that ends the question.** From the cold spawn, yaw **115** and yaw **116** render
**byte-identical screens**. At 116 holding `W` deploys breach; at 115 holding `W` for 60 m deploys
nothing — the ray misses by **2.5 cm**. *Which sortie will a forward walk start* is not answerable
from a sentence, so the line stopped answering it.

### 🔴 And the gate was green for three rounds, in the same shape each time

`f177_no_stance_in_the_hub_names_a_door_and_opens_another` swept **2 361 960** stances and took
**one** height from `stand(&mut app)`, varying only `x`/`z`/`yaw`. **The one axis the rule depended
on was the one axis the fixture held constant** — and it is the axis the game moves the player
along at the exact moment he arrives in the hub. `walk_forward`/`walk_forward_quantised` inherited
the same constant `y`, so the oracle could not disagree either. *Before trusting a sweep, ask what
it holds constant, and write the answer in the test's own comment.*

### The repair: a **promise** and a **pointer**, and no model of anything

* **Promise** — only when `hub::deploy_on_contact`'s own test says yes (*a pad within its own
  `radius_m`, 3D, nearest wins*), asked over the same pads, from the same `Transform`, on the same
  tick: `Deploy: Ashgate Skirmish / Recruit` / `on the pad - the sortie is starting`. True by
  construction; it lives the one frame between contact and `Deploying`, which is exactly as long
  as it is true (**3946 amber px in the banner band at tick 335, 0 at tick 336**).
* **Pointer** — everywhere else: `Ashgate Skirmish / Veteran` / `25 m in front of you - amber pad`.
  It names a door and promises nothing. No verb, no *"walk onto"*, and **no *"ahead"*** — the
  forward bearing word is now *"in front of you"*, the mirror of *"behind you"*, so all four words
  are positions rather than instructions.

That removes the whole class: **no ray, no `perp`, no `step_m`, no path model** — so the `y`
dependence, the 36 mm bob, the jump apex, the 73 props and the angled walk all stop being
questions. `src/mission/hub.rs` is untouched; `deploy_on_contact` remains the only authority.

**Numbers.** The sweep now varies `x`, `z`, `yaw` **and `y`** — 81 × 81 × 360 × 4 heights (2.0 the
hub landing, 1.056 the jump apex, 0.086 and 0.050 the two walk-bob heights) = **9 447 840
stances**: **813 960** stand on a pad, **813 960 promises / 8 633 880 pointers** (8.6 % / 91.4 %),
816 067 sentences read, and **0** contacts the line was silent about, **0** promises with no pad,
**0** promises of the wrong door. `promises == contacts` exactly.

**The red control.** One line — `bearing`'s `to.length()` → the horizontal length, i.e. a *second*
geometry for the same question — puts **135 000 lying promises** on the board, the first at
`y = 2.0`; an independent count says 92 160 of them exist only at the landing height and 33 480
only at the jump apex. **The old one-height fixture could not have seen 125 640 of them.**

Real-game evidence, all `--headless --hub`, one pinned binary: the landing at yaw 140 (pointer
*Veteran*, then `deployment: "breach"` + `Deploy: Ashgate Breach / Recruit` on the same tick), the
bob at `(18, 0.06, 2)` `look 150` (pointer *Elite*, deploys `escort`, promise *The Long Cart /
Recruit*), yaw 115 (60 m, nothing deploys, the pointer re-aims and never promises) against yaw 116
(deploys breach), and a `W`+`D` angled walk (pointer *Veteran*, deploys breach). `f177-door.txt`
13 asserts, `f175-loop.txt` 19, `f070-hub.txt` 42, all exit 0.

**Two smaller things fixed with it:** the module doc stated the hedge count three inconsistent ways
(33 / 97 of 918 719, denominator 918 714 elsewhere) — the hedge is **gone**, and with it the
counts; and the two-door sentence that wrapped onto a **fourth** line in an element built as three
cannot be produced any more. The widest sentence the element can say is now measured in the
laid-out node (`f177_the_line_stands_above_the_keep_out_box_and_never_beside_the_objective`).


## FIND-191 — the reel can ask for two lengths no position in space satisfies, and it is older than the joint on `Drive`

> ✅ **FIXED 2026-08-28 — `docs/BUGS.md` B-013, and the boundary it left behind is `FIND-198`.**
> `player::rope::hold_the_pair` stops the reel before `L_l + L_r` falls under the anchor
> separation. Over 84 cells with `Ctrl` held: worst arm **51.7104 m → 0.0092 m** past its own
> maximum, **7 920 → 0** infeasible ticks, **5 208 → 0** ticks pinned by a violation. The guard
> named below was **inverted rather than deleted** and is now
> `b013_the_two_rope_hold_reaches_both_force_models_because_the_reel_is_one_system`; the metres
> live in `f004_two_far_apart_anchors_hold_the_player_instead_of_dragging_him_past_a_maximum`
> and in `scripts/f012-tworopes.txt`. Everything below is the state as measured on 2026-08-27.

**2026-08-27 · [offlinebot] · found by the acceptance matrix of `Q-058`, not by a bug report**

Two anchors **170° apart**, both ropes 30 m, `Ctrl` held. `player::rope::shorten_ropes` takes
`vector.reel_speed_m_s` = 28 m/s off **each** `limits.max` per second, so within one second both
maxima are at `vector.min_rope_m` = 3.0 m — while the two anchors are **56 m apart**. No point
in space is 3 m from both. avian keeps one arm and abandons the other: the player is dragged
onto the right-hand anchor and sits there at 0.000 m/s with the left rope **50.167 m** past its
own maximum.

```
  t  0 |v| 75.049  Left max 30.118 dist 30.115  |  Right max 30.118 dist 30.118
  t 30 |v| 28.004  Left max 16.118 dist 40.049  |  Right max 16.118 dist 16.118
  t 60 |v|  0.000  Left max  3.000 dist 53.167  |  Right max  3.000 dist  3.000
```

🔴 **`Q-058` did not cause it. Measured against `Pendulum`, which has carried a `DistanceJoint`
since `F-004`: the two models agree to THREE DECIMALS on every sampled tick** — same 50.167 m,
same 0.000 m/s, same length per tick. The guard is
`tests/vector_rope.rs::find191_the_reel_can_make_two_maxima_impossible_and_that_is_older_than_the_drive_s_joint`,
and it asserts the *agreement*, not the number: if the two ever part company, giving `Drive` a
joint changed the reel rather than inheriting it. The per-tick probe is
`probe_two_opposite_anchors_with_the_reel_held` (`#[ignore]`).

**It matters more than it did**, and that is the honest part: `rope_force_model` ships as
`Drive`, so a defect that used to need a deliberate `game.ron` edit to reach is now on the path
a player walks.

### Why it is not fixed in this round

The rule would have to teach one arm's reel about the **other arm's geometry** — a per-arm floor
derived from where the second anchor is. That is exactly the hand-rolled multi-arm reasoning
`FIND-186` records as refuted 4/4, and the cheap-looking version of it (stop reeling an arm
whose rope is already violated) only moves the number from 50.2 m to 25.8 m, because the arm
that *wins* still drags the player 30 m across.

**What the premise of `Q-058` actually buys, stated precisely:** *"`limits = (0, L)` and the
solver holds all of them at once"* is true **for maxima that are simultaneously satisfiable**.
The ratchet alone never produces an unsatisfiable pair — the take-up only follows the distance
that really exists downward — which is why the 288-cell acceptance matrix without `Ctrl` reads a
worst excess of **0.0050 m** while this one reads 50.167 m. The reel is the one input that can
ask for the impossible.

**Related:** `Q-058` · `FIND-186` · `FIND-152` · `B-004` · `F-005`

## FIND-192 — `assert speed` cannot tell an escape from a haul, and it cost `scripts/f176-pull.txt` its only red line

**2026-08-27 · [offlinebot] · the same family as `FIND-103` and as `CLAUDE.md` §6 rule 5's chain test**

`scripts/f176-pull.txt` ACT 2 is *"on the ground, hooked, holding `S`, the player does not
move"*, written as `assert speed < 2.0`. Before `Q-058` it read **6.000 m/s** — exactly
`player.run_speed_m_s` — and that really was a player walking straight out of his own rope. With
the joint it reads **7.630 m/s**, and the assert calls that the same failure. It is not: the
player is being pulled **toward** his anchor at `vector.drive_idle_speed_m_s` = 12 m/s.

**The distance is the number the requirement is about, and the harness cannot ask for it** —
`src/debug/script.rs`'s `rope` metric is `hook.anchored_count()`, an arm count, not a length. So
the proxy was a speed, and a speed is symmetric: 6 m/s away and 12 m/s toward both read
"> 2".

Measured as the distance instead, one second of `S` from a taut 36.723 m rope
(`tests/vector_rope.rs::f176_under_drive_walking_backwards_on_a_taut_rope_still_closes_on_the_anchor`):

| | after one second |
|---|---|
| `Drive` | **35.637 m** — still closing |
| `Pendulum` | 36.715 m — stands; the constraint holds and nothing pulls |
| `Drive`, rope let go | 39.619 m — the control, a man walking away |

Two smaller things the same measurement caught, and both were *fixtures lying quietly*:

1. **`place()` does not move a player for more than one tick.** It writes `Position`, and the
   next tick's `Transform` sync puts the body back. A first version of this fixture placed once
   and then ran 30 ticks — it measured a player at the **origin** on a 63.5 m rope that was
   slack for every tick of the run, and every number it produced was about nothing at all. Use
   `warp` for anything that has to stay put across ticks; `place` only inside a per-tick loop.
2. **The stance's own script says +Z is blocked** (*"a walk into it reads 0.915 m/s instead of
   6.000"*), which is why `scripts/f176-pull.txt` sets `look 180 66.6`. A fixture that omitted
   the look had its control move **0.028 m** and would have reported a wall as a rope.

**The lesson, and it is the cheap half:** when the mechanism under an assert changes sign — a
push becoming a pull — **a magnitude assert survives the change and starts lying.** Before
trusting a proxy after a redesign, ask what the proxy would read for the *opposite* outcome. If
it reads the same, it is not a guard any more. → `docs/QUESTIONS.md` Q-061, which is the
decision this one is not allowed to make on its own.

**Related:** `Q-061` · `Q-058` · `FIND-172` · `FIND-103` · `FIND-186` · `docs/NEXT.md` §3D R1

---

## FIND-193 — the hub line's fourth refutation: **a promise that could only be read while it was false, and a pointer that ranked by the wrong thing**

**2026-08-27 · `src/hud/hub_prompt.rs` · two independent adversaries, two distinct defects · fixed**

`F-177` has now been refuted four times. The third attempt was a **promise/pointer split**: a
promise (`Deploy: X / Y` · `on the pad - the sortie is starting`) exactly where
`mission::hub::deploy_on_contact` fires, a pointer everywhere else. Both halves were broken.

### 1. The promise lied on **every** homecoming, and it was a schedule bug

`hud::hub_prompt::update_hub_prompt` ran in `Update` and re-asked `deploy_on_contact`'s question
of the player's `Transform`. `mission::hub::open_hub` sends the player home by **writing a
message** from `OnEnter(MissionPhase::Hub)` — i.e. in `StateTransition` — and
`player::apply_warps` applies it in `FixedUpdate` / `SimulationSystems::Integrate`. Between the
two there are `Update` frames in which the phase is already `Hub`, all six pads exist, and the
`Transform` is **still the position the finished mission left the player in**.
`deploy_on_contact` never judges that position, because the warp runs earlier in the same fixed
step.

Measured 5/5 deterministic, at all six pads, with five different mission names falsely promised;
the boundary is exactly `radius_m` (2.8 m promises, 3.4 m does not). The window is
**unconditional, not a race**. Reproduced here as a whole-app test against the shipped code:

```
one `Update` frame after `OnEnter(Hub)` ... the player is still at Vec3(0.0, 0.0, 16.0) — where
the finished sortie left him, on the "skirmish"/"recruit" pad. The screen reads
    Deploy: Ashgate Skirmish / Recruit
    on the pad - the sortie is starting
```

The module doc asserted the opposite in so many words — *"the `Transform` it reads is the one
`FixedUpdate` wrote, which is the same value `deploy_on_contact` judged in `PostStep` of that
step — that identity is what makes the promise impossible to falsify"*. **It was false, and
`FIND-190` and `Q-060b` rested on it.**

### 2. The pointer is 100 % of what a human reads, and it ranked by bearing

The promise was on screen for **3.4 ms** (two consecutive log lines, `00:26:42.608941 →
.612336`) before the phase transition — because the instant it is true is the instant the sortie
starts. So everything a player actually reads in the hub is the pointer, and the pointer ranked
by `|bearing|` first: over the 40 × 40 m yard, **1 521 370 of 2 129 040 pointer stances (71.5 %)**
named a pad that was **not** the nearest, median 11.7 m farther; of the 1 298 238 whose line read
*"… in front of you"*, **121 033 (9.3 %) open a different door** and **619 412 (47.7 %) open
nothing in 60 m**. And the distance is a straight line **through solid geometry**: photographed
inside the garrison hall naming a pad on the far side of a wall.

### 3. The fixture hid it for the fourth round running, in two new ways

- **Provenance.** `f177_no_stance_in_the_hub_names_a_door_and_opens_another` swept 9 447 840
  stances and said out loud *"what this fixture holds constant: nothing that the rule reads"*. It
  held one thing constant: **where the position came from.** Every stance was a `Vec3` the test
  invented, and the oracle was a hand-copy of `deploy_on_contact` fed **the same invented `Vec3`**
  — two functions asked about the same invented point cannot disagree about *which* point it is,
  so defect 1 was unreachable by construction. Nothing in `tests/hud.rs` ever entered
  `MissionPhase::Hub` out of a finished mission.
- **An assertion that only forbids words.** For all 8 633 880 pointer stances — **91.4 % of the
  sweep** — the only assertion was that the string did *not* contain three words. It made no
  claim about which door was named, how far, or in which direction. The headline *"0 wrong
  doors"* was entirely about the promise.

**This is the fourth shape of `CLAUDE.md` §6 rule 5 with a new axis: a sweep's coverage is also
bounded by the PROVENANCE of its inputs.** Add the row to the fixture's own table — *"where the
position came from: NO"* — and then write the one test whose positions are the game's.

### The repair: **delete the promise, rank by distance, and say nothing until the hub has landed him**

- **There is no contact test in `src/hud/` any more.** *"Is a pad under him"* has exactly one
  implementation, in `mission`. The promise is deleted outright: a sentence nobody can read while
  it is true and everybody can read while it is false is not an element. (This is the honest half
  of the *one writer* shape without the `mission` diff — see the diff **not** taken, below.)
- **`hub_prompt::nearest_known_pad` ranks by 3D distance**, tie-broken by the pad's own name —
  the same ranking `deploy_on_contact` uses, so the two can no longer disagree about which door is
  *the* door here. Yaw moves the direction word and nothing else.
- **`hub_prompt::HubLanded`** is `false` from `OnEnter(Hub)` until the first `FixedLast` of the
  visit, and the line renders nothing while it is false.
- The sentence is a **location**: `Ashgate Breach / Recruit` / `nearest amber pad: 13 m to your
  left` / `Esc: Mission select`, and `hub_prompt::INSTRUCTION_WORDS` is the testable form of
  *"contains no instruction"*.

**Evidence, and both halves were seen red first.**

| claim | test | red when |
|---|---|---|
| the homecoming is silent until the warp lands | `tests/hud.rs::f177_the_line_says_nothing_until_the_hub_has_landed_the_player` | against the shipped code, with the `Deploy:` sentence quoted above; and again on the one-line break `let landed_here = true` |
| every stance names the nearest door | `::f177_no_stance_in_the_hub_names_a_door_that_is_not_the_nearest` — 9 447 840 stances, **0 skipped** | one-line break back to bearing-first: **6 802 164 wrong doors, 6 654 536 wrong distances, 612 363 stances where the pad fires and the line names another**. The refuted sweep reported **0** for all of it |

`docs/images/f177-line-left.png` / `-front.png`: the same door line and key line at two yaws
(glyph masks differ by 207 and 104 px — anti-aliasing against a turned scene) against **1902 px**
on the direction line. The door does not move; the direction does.

### The `src/mission/hub.rs` diff that was **not** taken, and why

The textbook repair is *one writer*: `deploy_on_contact` publishes what it decided (a resource
naming the pad the player is standing on, written where it already runs) and `hud` reads it. That
needs a line in `src/mission/hub.rs`, which this round did not own. It was **not needed**: with
the promise deleted, `hud` asks the question no longer, so there is nothing to publish. If a
future round wants a lasting *"you are on the pad"* confirmation, it belongs to
`MissionPhase::Deploying` — `mission`'s own phase — and not to this line.

**Related:** `FIND-188` · `FIND-190` · `FIND-178` · `FIND-103` · `docs/QUESTIONS.md` Q-060/Q-060b

---

## FIND-194 — a derived constant carried into a test BY HAND is a stale literal with extra steps, and gravity −32 found five of them

**Measured 2026-08-27 [offlinebot]**, `tests/player.rs`, while landing `Q-058`. The gravity
change (`gravity_m_s2` −20 → −32, `jump_speed_m_s` 6.5 → 8.2, commit `1ca7d26`) left **five tests
red in a file nothing in that commit touched**:

| test | assert | measured | why |
|---|---|---|---|
| `t007_a_jump_reaches_exactly_the_height_the_file_allows` | `\|y − 0.90\| < 0.05` | 0.9977 | literal `6.5·0.2 − 10·0.04` |
| `f004_a_hook_in_the_wall_does_not_glue_the_player` | `\|risen − 0.90\| < 0.05` | 0.9978 | the same literal, copied |
| `f014_a_slide_comes_to_a_full_stop_without_input` | `speed > 19.0` after 2 ticks | 18.9333 | `20 − 20·2/60` written out |
| `f014_a_slide_…` (second assert) | `\|v − 10.0\| < 1.0` at 0.5 s | **0.0000** | see below — this one is not arithmetic |
| `f014_the_input_still_steers_the_carried_momentum` | `\|speed − 20.0\| < 1.5` | 14.4922 | literal `30 − 20·0.5` |
| `f006_w_flies_where_you_look_a_and_d_go_sideways_and_s_never_thrusts` | `\|v − a\| < 0.5`, `a = −g/2` | 10.0002 vs 16 | **the wrong key** |

**Three different failure modes, and only the first is boring.**

1. **Four are plain stale literals.** Every one of them had the derivation written in the comment
   right above it (*"v0 = 6.5 m/s at g = −20: 6.5·0.2 − 10·0.04 = 0.90 m"*) and the derivation
   evaluated by hand into the assert. All four are re-derived from `game.ron` now, so the next
   constant change moves them by itself. **The comment being correct is what made this invisible:
   nobody reading the test could tell the number was frozen.**

2. **`f006` was deriving from a key that stopped being the source of truth.** It read
   `let a = -d.game.gravity_m_s2 / 2.0` under the sentence *"the air control is half of gravity"*,
   and left a note: *"the day it becomes `game.ron: player.air_accel_m_s2` this line reads that
   key."* **That day had already come** — `air_accel_m_s2: 10.0` is read by
   `player::locomotion::air_control` and by nothing else — and the test kept deriving from
   gravity, so the moment the user moved gravity the test demanded 16 m/s² from a 10 m/s²
   accelerator. **A derived constant is only derived while somebody re-derives it; carried by
   hand it is a literal that looks like a derivation**, which is worse, because a literal is at
   least obviously frozen.

3. 🔴 **And one of the five was hiding a mechanism, which is the reason this entry exists.**
   `f014_a_slide_…` sampled at 0.5 s and asserted `20 − decel·0.5`. At −20 that read 10 m/s and
   was a fair mid-ramp reading. At −32 it reads **exactly 0.0000**, and the naive re-derivation
   (`20 − 32·0.5 = 4.0`) is red too. The reason is a floor nothing in the test knew about:
   `ground_step` returns the desired velocity *directly* below `run_speed_m_s`, so the linear
   brake runs only down to `run_speed_m_s + decel/hz` = **6.53 m/s** and the last 6.5 m/s go in
   **one tick**. The snap sat at 0.683 s under the old constant — *after* the sample — and at
   **0.421 s** under the new one, *before* it. **The literal `10.0` had been standing in front of
   a cliff for as long as the test existed and nobody could see it**, because at −20 the sample
   never reached it. The test now derives the sample time from the snap and asserts that the
   sample is still on the ramp; if a future constant moves the snap past it again, the test says
   so instead of measuring a standstill and calling it a slide.

**The rule this earns:** a test may hold a literal only if the literal is the *specification*
(`run_speed_m_s` is 6.0 because the file says 6.0). A number that is a **consequence** of two
other numbers must be computed in the test from those numbers — and if the consequence has a
**piecewise** shape, the test must assert which piece it is sampling. Point 3 is the expensive
half: arithmetic staleness goes red and gets noticed, a sample that has quietly walked off the
end of a ramp goes red **with a plausible-looking number** and gets "re-derived" into a second
wrong assert. It nearly was.

**Not fixed here, reported:** `scripts/f-*.txt` is aimed at −20 the same way, per assert line
(`docs/NEXT.md` §3G) — and the same three modes apply to it. Re-aiming the corpus is the main
head's call.

**Related:** `docs/NEXT.md` §3G · `FIND-051` · `Q-058` · `T-007` `F-004` `F-006` `F-014`

---

## FIND-195 — with the ratchet held, 64.4 % of ticks ask for two rope maxima no position satisfies, and a cross-model differential is NOT a control for it

> ✅ **FIXED 2026-08-28 — `docs/BUGS.md` B-013 · `FIND-198` · `Q-079`.** `player::rope::hold_the_pair`
> stops the reel before `L_l + L_r` falls under the anchor separation, so the 64.4 % becomes **0**.
> Re-measured on a wider sweep of its own (3 heights x 4 length pairs x 7 separations, `Ctrl` held,
> 10 080 ticks): **7 920 → 0** infeasible ticks, worst arm **51.7104 m → 0.0092 m** past its own
> maximum, **5 208 → 0** ticks pinned by a violation. This entry's second half — *a cross-model
> differential is not a control for it* — is untouched and still true; the absolute bound is now
> `tests/vector_rope.rs::f004_two_far_apart_anchors_hold_the_player_instead_of_dragging_him_past_a_maximum`.

**Measured 2026-08-27 [offlinebot]**, `tests/vector_rope.rs`, four 288-cell matrices (4 anchor
separations × 2 elevations × 4 yaws × 9 key combos × 90 ticks), built as `Q-058`'s acceptance
criterion: *two anchors, all nine key combos, the full composition, and no individual arm distance
may exceed its own `limits.max`.*

**Without `Ctrl`, the criterion holds as written**: worst excess **+0.0050 m** in the air and
**+0.0050 m** on the ground, over 288 cells each, 0 infeasible ticks, 0 ticks with a rope missing.
Against the rollback (`commands.spawn(rope);` back in the `Drive` arm of
`player::rope::attach_ropes`) the same matrices read **+51.1978 m** and **+50.0737 m** — four
orders of magnitude, which is what makes it a guard and not a tolerance.

**With `Ctrl` held the criterion is not satisfiable by any solver, and the number says how often.**
`player::rope::shorten_ropes` walks *both* `limits.max` toward `min_rope_m` while the anchors stay
where they are, and two joints with maxima `Lₗ`, `Lᵣ` on anchors `dₐ` apart have a solution iff
`Lₗ + Lᵣ ≥ dₐ`. Over the matrix that inequality is **false on 16 704 of 25 920 ticks (64.4 %)** on
the ground and **16 712 of 25 920 (64.5 %)** in the air. That is `FIND-191` at matrix scale, it is
`shorten_ropes`' defect, and it predates the joint on `Drive` by the whole life of `Pendulum`.

**So the assert is restricted to the ticks where the promise is a promise — and the exclusion is
counted, not silent.** Over those ticks the worst excess is **+0.2748 m** (air) / **+0.2750 m**
(ground), against a band of **one tick of reel**, `reel_speed_m_s / simulation_hz` = 0.4667 m plus
the 1 cm solver slop. That band is *derived*: `limits.max` can fall 0.4667 m underneath a body the
solver has already placed, so the excess is a **shrinking maximum**, not a lengthening rope — the
opposite of what §3F forbids. The run uses 59 % of it.

### 🔴 The part that was measured the hard way: a `Drive`-vs-`Pendulum` differential is not a control

The obvious control for *"`Q-058` INHERITED the reel, it did not change it"* is to run the same
matrix under both force models and assert the per-cell difference. **Written that way it goes red
at 0.1878 m** (120° separation, 20° elevation, yaw 0°, `D` held) — and that is not a defect, it is
the two force models doing their two different jobs: `D` under `Drive` is `rope_drive`'s lateral
velocity target, `D` under `Pendulum` is `rope_steer`'s force, and with **no key at all** `Drive`
still has `rope_winch`'s always-on pull that `Pendulum` has never had (`FIND-172`). **A model
differential over the ticks where the drive is acting is a test that the two models are the same
thing, and `f149_the_two_force_models_are_not_the_same_thing` exists to say they are not.**

The differential is a valid control **only in the saturated regime** — once the pair is infeasible
the end state is geometric (the player sits at `min_rope_m` from one anchor and the other rope is
over by the rest, whatever the keys were doing), and there the two models agree to **0.0000 m on
all 288 cells in both stances**. That is the claim `Q-058` actually made, and it is the one
asserted.

**The general shape, and it is the fourth face of `CLAUDE.md` §6 rule 5:** when a sweep's
criterion is unreachable for structural reasons, the honest move is neither to loosen the bound
nor to drop the cells — it is to **state the feasibility condition, restrict to it, and print the
size of the exclusion beside the result**. `64.4 %` is a fact about `shorten_ropes` that no
loosened tolerance would ever have produced.

**Still open, and it is not this round's:** the fix for `FIND-191` would have to teach one arm's
reel about the other arm's geometry — the hand-rolled multi-arm reasoning that was refuted 4/4 in
`FIND-186`. It wants a joint-side answer, not a `shorten_ropes` rule.

**Related:** `FIND-191` · `FIND-186` · `FIND-172` · `FIND-183` · `Q-058` · `docs/NEXT.md` §3F ·
`F-004` `F-005` `F-006`

---

## FIND-196 — holding `W` closes LESS distance than holding nothing, and it predates the joint

**2026-08-27 · [offlinebot] · measured on two binaries one control run apart**

Found while landing the rope joint, as a side effect of re-aiming `Q-061`'s `S` assert. It is not
the joint's doing and it is not a regression — **it is a pre-existing property of the drive that
nobody had a number for.**

`tests/input.rs::r7_s_tightens_the_rope_and_never_reels_it_in`'s fixture: from `(75, 2, -30)`,
`look 0 44`, an 82.2 m rope onto the wall gallery, 120 ticks.

| key held | HEAD (no joint) | with the joint |
|---|---|---|
| `S` alone, always-on pull **deleted** | **+8.911 m** — opens the rope | **−0.157 m** — moves nothing |
| `S`, pull on | +8.308 m | **−11.937 m** |
| `W`, pull on | **−11.270 m** | **−11.270 m — identical to three decimals** |

**Two things fall out of this table.**

**1 · The joint does exactly what the user asked for, and touches nothing else.** *„aber NICHT das
seil verlängern!!"* — with the joint, `S` cannot open the rope, so walking backwards moves the
player **0.157 m** instead of **8.911 m**. `W` is unchanged to three decimals, so the joint is not
paying for that with the drive.

**2 · 🔴 But `W` closes 11.270 m where the pure always-on pull closes 11.937 m.** Holding the key
that is supposed to fly you *at* your anchor is **0.667 m worse over two seconds than holding no key
at all**, in a geometry where the anchor is 44° up and 82 m away. The drive does not add to the
winch here — it **replaces it with something slower**, and it is the shipped composition
(`wanted = flight + winch`, where `flight`'s look gate `max(0, l̂ · r̂)` shrinks the target while
the winch's floor does not).

**Why nobody saw it:** the assert that spanned these two numbers said
`assert!(with_s >= with_w)` under the words *"S closes more distance than W — it still has a rope
power the other movement keys do not have"*. It was **a proxy for a claim about `S`** and it passed
for eighteen days because `S` used to walk you backwards (+8.308) and any closing number beat it.
The moment `S` stopped retreating, the proxy started reporting on `W` — and failed. **A proxy that
outlives its subject is how a green suite stops meaning anything**, which is the same family as
`FIND-103` and the four fixture failures of this week.

**Not fixed here, deliberately.** Whether `W` should beat the free pull is a tuning question that
belongs with `Q-064` (Shift) and `Q-063` (gravity) at the play test, and the drive numbers are all
provisional until he has flown them. **Filed with its number so the next round starts from a
measurement instead of from the proxy.**

**Related:** `Q-061` · `Q-063` · `Q-064` · `FIND-172` · `FIND-103` · `F-005` `F-006`

---

## FIND-197 — the anchor field is deleted: 2 528 lines, six RON keys and one allow-list edge for a system the hook never called

**Measured 2026-08-28 [offlinebot], on the instruction of the user, 2026-08-27:**

> *„es soll auf jeglicher oberflqche einhaken. nicht an hardcoded punkten etc!"*

### What went, and what it cost to keep

`world::AnchorField` generated a discrete point list per district — corners inset 0.5 m,
edge points every 12 m, wall courses every 15 m, capped at 48 per block, bucketed into 32 m
cells — and adopted every `hook.*` empty out of the loaded models on top. `hud::anchor_marks`
drew twelve rings on the best of them. Removed today, exactly:

| file | lines |
|---|---|
| `src/world/anchor.rs` | **787** |
| `src/hud/anchor_marks.rs` | **666** |
| `tests/world.rs` — the `F-021`/`F-022`/`F-023`/`F-031a` block, 7 tests | **407** |
| `tests/hud.rs` — the `F-026`/`F-027`/`F-030a` block, 9 tests + 3 fixtures | **589** |
| `scripts/f026-marks.txt` (+ 2 frames, 1.7 MB) | **84** |
| plugin registrations, `AnchorBlock`, `log_field`, headers | **~95** |
| | **2 641 deleted / 113 added / net −2 528** |

`--lib` did not move (271 → 271): **neither deleted source file carried a single `#[test]`.**
Every line of proof it had lived in the two integration binaries.

### 🔴 The finding that outlives the deletion: NOTHING WAS BROKEN, AND THAT WAS THE PROBLEM

`FIND-160` had already recorded that `grep AnchorField src/` outside `src/world/` came back
empty, and `FIND-178`/`B-011` recorded what happened when it was finally given a consumer: the
HUD lettered `Q` on a point 194.15 px away from the point `Q` actually flies at. The repair
that round chose was to take the **letters** off and keep the field. That was the wrong half.

**A system that can only be wired up by making the game lie about itself is not unfinished —
it is a second answer to a question that already had one.** The first answer was there the
whole time and it is three words: `vector::aim` casts a **ray**, and asks what it hits whether
it carries `shared::AnchorSurface`. The field was a parallel model of the same world, built
from the same `maps.ron` plan, and any consumer of it would have had to agree with the ray or
contradict it. That is the shape `CLAUDE.md` rule 5's corollary already names — *do not
re-derive another domain's decision, read it* — arriving as 1 453 lines of source instead of
as one stale `Transform`.

### The evidence that the shipped rule is the right one: `scripts/f001-surfaces.txt`

Five surfaces of five different kinds, one run, 14 asserts held, exit 0:

| leg | surface | body | the rope bit at |
|---|---|---|---|
| 1 | a wall's flat vertical face | 150 | `(51.00,  4.12,  −1.00)` |
| 2 | the same building's ROOF, top face | 150 | `(51.00, 11.50,  −8.32)` |
| 3 | a 120 m gate tower, high on its side | 70 | `(24.00, 20.57, −92.50)` |
| 4 | a house the SEED built, not a hand | 1526 | `(−150.00, 10.06, −6.28)` |
| 5 | the pavement, 30° down | 11 | `(51.00,  0.05,  10.23)` |
| 6 | **open sky — the control** | — | `found no anchor: NothingInRange` |

**Not one of those points is a corner, an edge, a 12 m spacing, a 15 m course or a named
`hook.*` empty** — i.e. not one is a place the deleted field would have offered. Leg 6 is what
makes the other five falsifiable: the same trigger, pointed at nothing, anchors nothing.

### And the number that says the request is already satisfied

`build_map` prints it on every single run and nobody had read it:

```
map "Ashgate": 2901 blocks built (245 placed, 2656 generated), 2901 of them anchorable
```

**All 2 901.** Every hand-placed row carries `anchorable: true` and the generated lots run at
`anchorable_fraction: 1.0`. There is no untagged block on the shipped map to aim at — so the
"hook on any surface" the user asked for is not a feature to build, it is a feature to **stop
covering up**. (The F-003 rule that an untagged body is a hit but not an anchor is still in
`src/vector/hook.rs`; `maps.ron: graybox` still has unanchorable rows if it ever needs a red.)

### 🔴 What survives, and cutting it would have been the failure

**The aim assist is not the anchor field and never touched it.** `vector::aim::probe_dirs` /
`pick_best` / `score_candidate` / `required_margin` sweep the **ray** sideways off the
crosshair's own screen row and score what the probes *hit*; `AimCandidate` is built from
`hook::anchor_target(&probe)`, a raycast result. `shared::PlayerSettings::assist_catch_pct` /
`assist_strength_pct` and `hud::catch_band` are its user-facing half — the user asked for the
band twice, including *„es soll in der ui angezeigt werden von wo bis wo gesearched wird"*.

Verified after the deletion, `scripts/f016-band.txt`, 9 asserts held, exit 0, and decoded
offscreen at one stand: the catch-0 and catch-100 frames differ in **632 px**, all of them in
`y 352..368`, **`x 412..867` on the crosshair's own row 360** — 228 px left and 227 px right of
centre, which is the same extent `src/hud/mod.rs` recorded before the field existed.

### What is now dead but NOT deleted, because it is not this job's to delete

**1 564 `hook.*` empties, across 831 model instances, are still parsed into
`shared::ModelAnchors` on every single run and nothing reads one of them.** Counted from the
loader's own log:

```
./target/debug/defeated_by_titan --headless --script scripts/f001-surfaces.txt --ticks 300 2>&1   | grep -oE '[0-9]+ of them hook\.\* rope' | awk '{s+=$1; n++} END{print n, s}'
  -> 831 1564
```

`AnchorField::adopt_named` was their only consumer. `shared::anchors::HOOK_PREFIX` and the
`is_anchor_name` branch that keeps them (`src/render/model.rs`) are `shared`/`render`
territory. ⚠️ **Do not delete them reflexively:** `ANCHOR_NAMES` is a closed list whose
whole argument (in `src/shared/anchors.rs`) is that a Blender typo must show as a *missing*
anchor, and dropping the open `hook.` family changes what the loader keeps for every future
consumer. It is a decision, not a cleanup.

**Six `game.ron: game.hud` keys are dead data with no reader** — `marker_max`,
`marker_cone_h_deg`, `marker_cone_v_deg`, `marker_refresh_hz`, `marker_far_opacity`,
`marker_min_gap_px`. 🔴 **They cannot be removed from Rust alone.** This project has no
`serde(default)` and `HudTuning` is `deny_unknown_fields`, so dropping the struct while the
RON block stands makes the game refuse to load — and dropping the RON block first does the
same. **The `assets/data/game.ron` edit and the `src/data/mod.rs` edit have to be in one
commit.** `HudTuning` is left in place and marked, not deleted, because `assets/data/*.ron`
belongs to the main head.

**And one survey claim that was wrong, checked before acting on it:** `assets/data/maps.ron`
does **not** carry authored `hook.gesims_*` ladders. `grep -rn 'hook\.' assets/data/*.ron`
outside comments returns **nothing**; the two hits in `maps.ron` are English sentences ending
in the word "hook", and `hook.gesims_15..105` appears once, in a **comment** in `art.ron`. The
real `hook.*` names live inside the `.glb` files. **No `maps.ron` key becomes dead, and
`scripts/f003-ashgate.txt`'s 40 asserts are untouched by this round.**

**Related:** `B-011` (closed WONT FIX) · `FIND-160` · `FIND-178` · `Q-067` ·
`docs/PLAN.md` §0 · `F-024` (not to be built) `F-026` `F-027` (subject removed)

---

