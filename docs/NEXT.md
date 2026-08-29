# NEXT — what to do first, in order, when the session restarts

Updated: 2026-08-10 · Stage: 🟨 (a queue, not a result — nothing here has been done)

Written at the end of the first session that had a **window** and a **human who played the game**. This file is the queue; [`HANDOVER.md`](HANDOVER.md) is the state. Read the handover
first, then do the session ritual from [`CLAUDE.md`](../CLAUDE.md), then start at 1 below.

**Branch: `session-2026-08-09`.** `main` is still diverged — see `HANDOVER.md` §7.

> ⚠️ **Read the `## ✅ DONE` section at the BOTTOM of this file first** — several items in the
> numbered list below have since been closed, and the section at the end says which. The `Gas`
> two-writer violation that used to stand here is **fixed** (`FIND-063`).
> *(New entries are appended at the end because `>>` costs nothing to read while opening this file
> to insert at the top costs ~4 000 tokens. **Order by the number, not by the position.**)*

---

## 0. Before anything: the three files the user answers in

`git log --oneline -5 -- docs/QUESTIONS.md docs/BUGS.md docs/FINDINGS.md` and read what moved.
**Open and his:** `Q-031` (does a titan's facing matter at all), `Q-032`/`Q-033` (do the reel and
gas numbers feel right), `Q-035` (200 m hook range, and whether a 1.25 s worst-case hook flight is
too slow), plus the older `Q-019`, `Q-025`–`Q-028`.

Also his, and named in this session but never answered:
- **the 58 m swing gates are taller than anything he has ever named** (the church is 35 m). They
  work — 208 m of chained swinging — but they change the city's silhouette. `maps.ron`, the
  `swing lane` block, one rollback point.
- **`boost_rope_fraction`**: 0.5 today. The measurement says 1.0 gives the most speed and 30 % more
  energy; the reason to stay below 1.0 is control, not physics (FIND-045).

---

## 1. ⚠️ THE AIR-CONTROL SPEC — the user wrote it down, it is coherent, and it supersedes several of my decisions

From `user-messages.md`, 2026-08-12, migrated here because that file gets emptied for new notes.
**This is one design, not six requests.** Read it whole before building any part.

> *"wenn man w drückt und verbunden ist bekommt man schon movement! bei a und d movement zur seite.
> mit s »spannt« man nur das seil! … das a d sorgt dafür dass man nicht immer direkt zum seil
> gezogen wird!"*

**1a. WASD is the air control while roped.** `W` = thrust forward, `A`/`D` = lateral, `S` = tension
the rope only (no thrust). **The stated purpose of `A`/`D` is that you are not always dragged
straight at your anchor** — that is the steering the rope has never had.
⚠️ **This supersedes the boost/rope blend** (`boost_rope_fraction`, `FIND-045`/`FIND-046`). Direction
comes from WASD relative to the rope, not from a look/rope lerp. **Do NOT finish the old blend
before reading this** — item 2 below is now subordinate to this spec, and the anti-parallel bug
(90° off-look) may simply disappear with the blend itself.

**1b. Flight mode is a state with hysteresis, and touching the ground does not leave it.**
> *"nur weil man den boden berührt ist man nicht direkt aus flugmodus raus, erst wenn man langsam
> genug ist läuft man wieder"*

A speed threshold, not a contact test. **We already have exactly this machinery**: `MovementState::Tethered`
went in on 2026-08-10 with the rule *"the legs cannot produce more than the ground's top speed"*
(`FIND-037`), threshold `run_speed_m_s + (-gravity_m_s2)/simulation_hz` = 6.3333. **Flight mode is
the same idea generalised**, and `ground_locomotion`'s momentum carry (`F-014`) is already the other
half. This is a small change on top of what exists, not a new system.

**1c. Two boosts, not one.**
> *"mit doppel leertaste boostet man stark in die lauf richtung (ein weiter dodge) der viel gas
> aufbraucht. das andere boosten verbraucht sehr wenig!"*

`Shift` while in flight = the cheap, continuous boost. **Double-tap `Space` = a strong dodge in the
run direction, expensive.** Note `Buttons::DODGE` already exists on `C` and is written by nobody —
that is the variant's home. Double-tap needs an edge timer; the tick counter is the honest one.
⚠️ `gas_boost_per_s: 18.0` is currently the *only* boost cost. It becomes the **cheap** one, and
the dodge needs its own key. Both `⚠️ UNTUNED`.

**1d. Gas refuels only at stations. ✅ half done 2026-08-12.** `Q-033` is answered and the
regeneration is **removed outright** — mechanism, both RON keys, both struct fields, six tests
(`FIND-049`). Not "set to 0.0": `deny_unknown_fields` means a re-added key now **crashes on load**,
which is the honest guard. ⚠️ **`gas_tank` is 15000.0 since 2026-08-20, not 300** — the user, for the third time and with a number: *"immernoch viel zu wenig gas. man kann nicht testen! mach das 50 fache!"* (`Q-046`). The guard's reasoning is unchanged; only the figure moved.
**What remains is the feature itself: refuel stations as world objects**, plus a mission loop where
going back to the main building is a decision. `Gas::refill` exists, is called by nobody, and is
reserved for them.
⚠️ **Until they exist, the tank is the entire supply of a run** — 15000 since 2026-08-20, which outlasts a 330 s sortie by 2.5x and therefore hides the problem rather than solving it (`Q-044`: no refuel station exists anywhere in the world, and base-only refill is the user's own `Q-033`). Whether that is playable is a feel
question no test answers, and it is sharper now than it was yesterday.
⚠️ **`scripts/f-018-gas.txt` now exits 1** — its ACT 4 brackets (`gas > 5.4` / `< 6.2`) existed only
because the refill put 10/s back. It needs re-cutting and `docs/images/f-018-gas.png` re-taking.
Small, mechanical, and nobody has done it.

**1e. Without gas you keep about half.**
> *"ohne gas kann man immernoch w a d nutzen um etwas movement aufzubauen (aber hälfte ca)"*

So WASD air thrust is **not** gated on gas — it is halved without it. That is what stops an empty
tank from being the dead end it is today, and it is the honest answer to *"seile ohne boost bringen
gar nichts"*.

**1f. The acceptance criterion, in his words:**
> *"es soll möglich sein wenn man gut ist die ganze zeit in der luft zu bleiben bis das gas
> ausgeht."*

**A skilled player stays airborne until the gas runs out.** That is measurable and it is the gate
for this whole item: a script that never touches the ground for the length of a full tank.

**1g. Ropes much longer, RON-configurable.**
> *"seile sollen deutlich länger gehen! deutlich (über ron configurierbar!)"*

`hook_range_m` went 90 → **200** on 2026-08-10 (`Q-035`) — **that may already be this request, or he
may want more.** It is one RON value and `tests/data.rs` guards `range/speed ≤ 1.5 s`, so raising it
again means raising `hook_speed_m_s` with it. **Ask before spending a round on it.**

**Build order I would defend:** 1b (flight state, smallest, unblocks the rest) → 1a (WASD air
control, the core) → 1e (half without gas) → 1c (two boosts) → 1d (stations) → measure 1f.

## 1z. `prompts/DefeatedByTitan_Design-Bibel.md` — the last scaffolding file, deliberately still here

`prompts/init.md` was audited section by section and **deleted** on 2026-08-12 (`FIND-048`,
transfer record in [`RELEASE.md`](RELEASE.md)). The bible was walked in the same pass and its
content is in [`gameplay/`](gameplay/README.md) — pillars, world, enemies, core loop.

**It was NOT deleted, and that is a judgement, not an oversight.** The audit is 🟨 by its own
verdict: *"no independent agent has attacked it"*, and this file is the **design authority** —
`init.md` itself put it "inhaltlich über dieser Datei". Retiring the source of every WHY in the
project on one head's reading is the kind of step this project's own rules say to attack first.

**To close it:** one cheap counter-check — an agent that did not do the carry-over reads the bible
and `docs/gameplay/` side by side and names anything the latter loses. If nothing, delete it. It is
recoverable from history either way, so the cost of being wrong is small — but so is the cost of
checking.

## 2. The boost blend — a fork the USER decides, do not settle it by taste

**Since WASD air control landed (`F-006`, FIND-051), the blend is no longer needed as a steering
mechanism.** Two honest options and they contradict each other:

- **`boost_rope_fraction: 0.0`** — one RON value, no code change, and **FIND-046's 90°-off-look
  band disappears** (the early return at 0.0 is bit-exact to pure look direction). But it deletes
  the thing the user literally asked for: *"wenn man boostet soll man in richtung seil und
  mauszeiger fliegen"*.
- **Keep the blend and fix it** — the interrupted commission: rewrite the refuted physics rationale
  (FIND-045), make `w` an actual dial (`nlerp` gives 3 % of the angle at 170° separation), and cap
  the deviation from the look direction so "rope behind you, boost forward" stops throwing you
  sideways.

**What still argues FOR the blend even now:** flying *at* your anchor shortens the rope through
`B-005`'s ratchet, and that is what lifts an arc bottom from 18.3 m underground to 14.8 m above
(`FIND-041`). Looking at the anchor while thrusting reaches the same loop — so the blend is a
convenience, not the only route.

**Ask him. Do not pick.** If he wants it kept, the fix is the commission above; if not, `0.0` first
and delete `boost_direction`/`rope_dir` in a later cleanup once he has felt it.

## 2b. Finish or DELETE the boost blend — the commission, if he keeps it

An agent was rebuilding it when the session ended and **had written nothing** (verified: it was
still reading). The commission is fully specified in `FINDINGS.md` FIND-045 and FIND-046. What has
to happen:

1. **Delete the refuted physics claim** from `src/vector/boost.rs`'s header and from
   `assets/data/game.ron`'s `boost_rope_fraction` comment. Both currently say a taut rope absorbs
   radial thrust and that this is why the value is 0.5. **It is false** — a rope is a one-sided
   constraint and inward thrust is never absorbed. Replace the rationale with **control**, and put
   the measured table (FIND-045) where somebody changing the number will see it.
2. **Fix the anti-parallel band (FIND-046).** Rope behind you + boost forward — *"happens in every
   swing"* — currently sends the player ~90° off-look at `w = 0.5`. At 170° separation the boost
   goes 85° off-look. **This is a feel bug the user will hit in his first minute.**
3. **Make `w` an actual dial.** `nlerp` is not angularly linear: at 170° separation `0.25` moves the
   boost 3 % of the way and `0.75` moves it 97 %. `slerp` fixes the linearity but **not** item 2 —
   an angle cap on the deviation from look direction probably does. Pick one, argue it, measure it.
4. **Do not change 0.5 on your own authority** — it is the user's. But if the blend's shape changes
   what 0.5 *means*, say so loudly.
5. Preserve what the refutation round CONFIRMED and must not regress: magnitude exactly
   `boost_m_s2` at every angle and every `w` (worst case measured 7.63e−6 over 3.6 M samples);
   `w = 0.0` **bit-exact** to pure look direction (the early return is load-bearing — 1 ULP per
   component without it); no NaN over 200 k adversarial near-cancellations.
6. **Add the missing range guard** in `tests/data.rs`: `boost_rope_fraction: 2.0` loads and is
   silently clamped to 1.0, `-1.0` clamps to 0.0, `NaN` falls through to look direction. Guard
   `0.0 ..= 1.0` and finiteness. Red first.

## 2. `B-004` — cutting a titan while roped panics the process

**This is the game's core loop and it aborts.** `combat::hitstop::begin` puts `RigidBodyDisabled`
on the player while the `DistanceJoint` is live; the joint's later removal trips
`island.joint_count > 0` inside avian (`islands/mod.rs:786`). Repro is one character in
`scripts/f-flight-cut.txt`: `hook right 0.74` → `0.80`, exit 101.
Second face, same bug: `player::rope::shorten_ropes` keeps shortening through the frozen ticks and
pays 0.93 m back in one tick as a clamp-limited 74.700 m/s.
**F-034 is 🟧 → 🟨 because of this and does not go back up until it is fixed.**

## 3. `min_rope_m` is a cliff — 17 m/s in one tick, now on every fast approach

FIND-035. At the length floor the constraint annihilates the whole radial component at once:
**38.684 → 21.480 m/s between two ticks**, and after it the player is on pure ballistics.
Since `B-005`'s slack take-up the floor is reached **without a button being held**, so this is on
the common path now. It is what both `game-full` and `f-001-hooks` end up measuring, and it is the
reason `assert speed > 25` is red at 20.147 while the flight peaks at **38.684 at t=230**.
**Do not loosen that assert.** The repairs are (a) ease the cliff in `src/player/rope.rs`, and
(b) add a second sample at the peak so both facts are recorded.

## 4. The rope is drawn but does not look like a rope — `player.hand_offset_m`

FIND-022 / FIND-040. The simulation's hand is **bit-identical to the camera position**, so every
rope projects to a vertical line through the crosshair. One RON key (`player.hand_offset_m`,
proposed 0.30 m lateral) fixes it.
⚠️ It does **not** fix the aim markers — measured, a 0.30 m offset is 6.2 px at 30 m and 1.9 px at
100 m. Two symptoms, one missing number, only one of them cured.

## 5. The two aim points are not aim points yet

FIND-039 / FIND-047. What exists are **per-arm state badges** pinned at fixed screen coordinates
(measured at the same pixels across four runs with four different aims). Useful; not what the user
asked for.
**The real version is already in the user's own backlog** — `docs/backlog/gameplay.ron` **F-023**:
the candidates are split into left and right hemispheres relative to camera forward, **Q serves the
left set, E the right**. It needs `F-021` (discrete anchor points) first. Both ⬜, both in `vector`.
A per-arm carrier in `shared` (`ArmAim { point_m, body, anchorable }`) crosses **no domain edge**
and changes `hud` on one line.
Two smaller items from the counter-round: `Ready` vs `Busy` differ by only 8 px of ring diameter
(the weak pair against F-026's *"say without thinking"*), and the shape test over-neutralises by
filling the ring.

## 6. Widen the swing lane, or decide it stays one lane

Nine gates on one row at z = 70 give 208 m of chaining at 4.8× running on zero gas. **The other
three quadrants still have FIND-026's problem.** The design rule is now arithmetic and needs no
playtest (FIND-041): a usable arc needs `d < H` — **the horizontal gap must be smaller than the
anchor height** — and speed comes from spacing, `v = sqrt(2g·descent)`. Anchors must stand over
**open ground**, so the shape is a gate and not a tower (FIND-042).

## 7. Housekeeping that is cheap and keeps biting

- **`src/shared/gear.rs:99`** documents `Hook::anchored_count` as *"the number behind `assert
  hooks`"*. The metric is called **`rope`**. One word.
- **`vector.rope_iterations` and `player.max_substep_m` are dead values** read by nothing, each
  defended by a passing test (FIND-027, FIND-030). Two guards that cannot fail for the reason they
  name.
- **`t005_the_wall_is_reachable_in_two_moves`** is a test whose name outlives its meaning: at 200 m
  of range the 120 m crown is one move from the ground.
- **`--screenshot` + `--script`**: you get the image **or** the verdict, never both (FIND-032
  finding 1). One line in `src/debug/screenshot.rs` — return `AppExit::error()` when
  `ScriptRun::failures` is non-empty, *after* the file is on disk.
- **`world.half_extent_m` clears its bound with exactly zero margin** (`400/2 + 200 = 400`, assert
  is `>=`). The next map size change or range change is a **coupled** change.

---

## The three process rules this session paid for

1. **Every claim gets a round that tries to refute it, by an agent that did not build it.** Three
   refutation rounds ran; **all three overturned something already reported as fact** — including a
   physics argument the supervisor had reasoned out and written into two files (FIND-045). Roughly
   **one in three** unattacked claims was wrong.
2. **Grep `docs/backlog/` for the F-ID before commissioning a feature.** F-026 specified the aim
   markers — acceptance sentence, colour-blindness clause and all — a year before the user asked
   for them, and the re-derived version was worse (FIND-039).
3. **A change to `src/net/local.rs` is never a local change.** One line of rebinding broke 9 scripts
   and 25 tests across 3 suites, and every failure presented as a physics or state-machine bug
   (FIND-038). Whoever touches it runs the whole suite **and** the script corpus.

And the one that outranks all three: **the user played the game for a few minutes and found more
real problems than a day of instrumented measurement did.** Gas that never refilled, a rope that
went slack on every fast approach, an overshoot invisible to every test we own because no test ever
flew at an anchor without holding reel. **Ask him to play. Then measure what he says.**

## 0. ⚠️ FIRST, and it is small: give `Gas` back its single writer

`mission::hub::refuel_at_stations` writes `Gas` directly (2026-08-12). `vector::gas` owns every
debit. **Two writers on one field is the violation rule 4 exists to prevent** — it works today only
because the two are disjoint *by phase*, and "disjoint by phase" is precisely the argument that
stops being true over the network, where it becomes two authorities on one value.

**The fix is ~30 lines:** the hub sends a `RefuelRequest` (or reuses the `GasGrant` seam), and
`vector::gas` is the only thing that ever touches the tank. `docs/architecture.md`'s authority table
carries the entry, `FINDINGS.md` FIND-057 §2 the two alternatives and the rollback.

Do this before anything else builds on the hub — the longer two writers stand, the more code
assumes it is allowed.

**While you are there:** `src/lib.rs`'s flag help omits `--hub`, and the hub is opt-in rather than
the default start (`FIND-057` §5 carries the ASSUMPTION and the rollback). Whether the game should
*boot* into the hub is the user's call.

## 3. Ashgate: make the houses the swing route, and take the scaffolding out

`ashgate` is built and 🟧 for geometry — wall, two gates on one axis, canal with bridges, towers,
25 hand-placed houses, 471 blocks, a measured swing at 33.328 m arc bottom / 31.375 m/s on zero gas
(`FIND-056`). **It looks like a district.** Two things are wrong with it and they are one thing:

**The gantries read as scaffolding through a medieval quarter**, and they only exist because the lot
grid is suburban. `FIND-058`: a rope swings while `d < H`, the pitch is `lot_m 28 + street_m 7` =
**35 m**, the houses are **11.5 m** — so nothing swings and a gantry had to be invented. **At ~10 m
of pitch the houses themselves are the lane** (10 < 11.5), the arc bottom sits over the pavement
where the reference puts it, and the scaffolding can go.
**"Make it look right" and "make the rope worth using" have the same answer: shrink the grid.**
⚠️ **Measure the block cost first** — ~12x the blocks at a 10 m pitch over 700 m; `half_extent_m` is
already 600 and the index 22 500 cells. It is the body count, not the grid, that will bite.

**Two numbers that are the user's, both cheap:**
- `scale.ron: architecture.eaves_m` has **no `house_large`**, so every 11.5 m house stays a flat box
  while 4.5 / 8.0 get pitched caps. One number turns a third of the street front into roofs.
- The 56/60 m gantries stand above the church (35 m) — the same open question as the graybox's
  (Q-022/Q-023), and it goes away entirely if the gantries do.

**And the mission still plays in the graybox.** `scripts/game-full.txt` warps to graybox
coordinates; moving the shipped mission into ashgate is a separate, deliberate step.

## 3b. The main building — you must be able to walk INTO it, and the resupply is inside

The user, 2026-08-12: *"auch das main gebäude in dem der gas und schwert nachschub ist muss da sein
(in das gebäude muss man rein laufen können. drinnen sind die nachschübe)"*.

**Today the hub is three amber circles on open ground.** `mission::hub` has `DeploymentPoint` and
`RefuelStation` and they work (`f070-hub.txt`, 20 asserts, exit 0) — but there is no building, no
door, no interior. That is the gap between "the loop runs" and "there is a place".

What it needs:
1. **An enterable building in `ashgate`** — walls with a real doorway, a floor, a roof, and an
   interior big enough to walk around in. ⚠️ **This world has no subtraction**: every block is a
   cuboid, so a doorway is a *gap between blocks*, not a hole cut in one. `FIND-056` already
   records the trap that bit the wall gates ("the plinth ate the gate" — decorative courses were
   emitted straight across the openings).
2. **Gas AND blade resupply inside it.** Gas exists (`RefuelStation`, `Gas::refill`, `F-019`).
   **Blades do not** — `Blades` is written by `blades`, and there is no resupply for them at all;
   `F-033 Klingenhaltbarkeit` is ⬜. Adding blade resupply means either a second station type or one
   station that restores both. **`blades` owns `Blades`** — do not add a second writer, we already
   have that problem with `Gas` (item 0).
3. **The interior must not break the aiming invariant**: `f002_every_tagged_surface_in_the_map_is_reachable_by_free_aiming`
   walks every anchorable block. An interior wall nothing can hook from outside is a tagged surface
   nothing can reach — either mark the interior `anchorable: false` or accept it and say why.
4. Roof reachable by rope, so arriving by air is the natural way in.

The layout research for the district is at
`…/scratchpad/shiganshina-spec.md` — build the HQ where the reference puts the garrison, not
wherever there is room.

## 3c. Two defects the map flip exposed — fix them in the rebuild, not separately

**1. 28 row houses cannot be hooked where the player aims.** They are built as a body plus a
narrower ridge cap, and the cap is `anchorable: false` sitting exactly over the roof centre. So the
highest point — the thing a player swinging the main street looks at and shoots — answers
`NoAnchor`, while a 2 m tagged ledge survives on either side. `FIND-059` measured it: 28 of 228
anchorable blocks report unreachable, and **none of them is genuinely unreachable** — it is the
caps. Either the caps become `anchorable: true` or the roof stops being a lie. **This is level
design, not a test bug**, which is why `f002_every_tagged_surface_...` stayed pinned to the graybox
rather than being loosened.

**2. `scripts/game-full.txt` breaks in ashgate and was deliberately not fixed.** 5 of 23 asserts,
**all in ACT 1**, which does `warp 24 0 -20` and hooks a graybox watchtower that does not exist
here: `Speed > 25 → 0.000`, `Height > 12 → 0.050`, `Gas < 15000 → 15000.000`. No anchor → no reel → the
tank is untouched → he never leaves the pavement. **The other 18 hold and the mission still wins:**
`MISSION WON at tick 898 — 3/3 kills`, because ACTs 2–4 are falls, not swings.
⚠️ **~30 other scripts also warp into graybox coordinates and were not checked.** Moving the shipped
mission into ashgate is a deliberate step, not a repair: it means re-cutting ACT 1 against real
district geometry.

## 3d. Unexplained: something over the ashgate origin stops a body at y = 200

Found while pinning the physics tests (`FIND-061`) and **not chased**, because pinning the test to
the graybox removed the question from that test rather than answering it:

`tests/vector_boost.rs::f007_the_boost_does_not_outrun_the_top_speed` spawns a flier at **y = 200**
and boosts it. Against ashgate it measured **exactly 0.0000 m/s** — *"below the clamp, so this test
is measuring something other than the clamp"*. The wall is 120 m; **nothing in the map should exist
at 200 m**, and a body in open air 80 m above the tallest structure should accelerate.

Either something is up there that should not be, or a body over ashgate's origin is being stopped
by something else entirely — and a mechanism that can silently zero a flier's velocity is worth
knowing about before it does it to a player. **One script, a per-tick trace of position and
velocity at y = 200 over ashgate, should settle it in minutes.**

Related and probably unconnected, from the same round: ashgate's **aprons are 0.3 m thick on a
ground slab whose top is exactly y = 0**, so every apron edge — street, square, boulevard, quay — is
a **5 cm lip**, and the player spawns on one (he rests at 0.04996878 m instead of 0). The overlap is
deliberate: it is what makes `world/map.rs` drop a generated lot. Whether a player at 40 m/s trips
on it is unmeasured.

---

## ✅ DONE 2026-08-12 — items closed since this file was written

- **§0 `Gas` single writer** — fixed. `mission::hub` writes `shared::RefuelRequest`; `vector::gas`
  applies it in `apply_refuel_requests` and is the only thing that ever raises a tank. No domain
  edge (the message lives in `shared`). One tick of latency by design = 0.67 gas at 40 gas/s.
  Behaviour unchanged: `scripts/f070-hub.txt` still 20 asserts, exit 0, same marks at the same
  ticks. `FIND-063`.
  ⚠️ **One deliberate deviation:** `RefuelRequest` is registered in `VectorPlugin`, not in
  `src/lib.rs` where the other eight messages live. The argument for it — a write path into `Gas`
  should not exist without its applier — is good enough to keep, but it is inconsistent and the
  next reader will wonder. Moving it is a safe one-liner in `src/lib.rs`.

## 4. 🔴 THE SCRIPT CORPUS IS AIMED AT A MAP THAT NO LONGER SHIPS

Flipping `current` to `ashgate` silently invalidated the evidence base. Measured, unmodified, today:

| script | result in ashgate | what it proved |
|---|---|---|
| `scripts/f-flight-cut.txt` | `hook Right … found nothing anchorable (t=112)` → **9 of 21 asserts failed, exit 1** | **the core loop**: a cortex cut landed out of rope flight |
| `scripts/game-full.txt` | 5 of 23 failed, all ACT 1 | the end-to-end mission run |

Both are written against `map "Graybox": 79 blocks` and a church that is **not built here**. The
mission still wins (ACTs 2–4 are falls, not swings), but **the flight half of the game currently
has no passing evidence at all.**

⚠️ **~30 other scripts also `warp` to graybox coordinates and have not been checked.**

**This is not a repair, it is a re-aim**, and it needs the district's real geometry:
- from spawn `(0, 2, 0)` a hook at pitch 70–80 finds nothing in any of eight yaws — **the ray flies
  over the district**; `look 0 30` anchors at `(0.00, 58.00, −97.60)` (measured while fixing B-004).
- `f-flight-cut`'s `hook right 0.74` dodge **is no longer needed** — B-004 is fixed, so the rope may
  stay attached through the cut. Re-aiming it is also the chance to measure what the cut speed
  becomes when it is not pinned to the 75.0 m/s clamp.

**Do this before trusting any 🟧 that rests on a script**, and re-run the whole corpus once when it
is done — a script that anchors nothing still exits 0 if its asserts never fire.

---

## 5. ⚠️ PLAYER NOTES 2026-08-12b — he played the district. Read every line; two of them say we went backwards.

Migrated verbatim from `user-messages.md` (then emptied, per the ritual in `CLAUDE.md`).

### 5a. The hook is too slow to fire
> *"seile gehen viel zu lang zum schießen! das sollte schneller gehen."*

`vector.hook_speed_m_s` is **160**, raised from 90 on 2026-08-10 *because* the range went to 200 m
(`Q-035`). Worst-case flight is **1.25 s** and `tests/data.rs` guards `range/speed ≤ 1.5 s`. **He has
now judged that and it is too slow.** Raising the speed is one RON value; the guard's ceiling comes
down with it. Ask what "schneller" means in a number, or measure a few and let him pick.

### 5b. 🔴 W on a rope walks instead of pulling — and TWO ropes should launch you
> *"wenn ich connected und w drücke lauf ich nur. aber will dass man dann rangezogen wird!"*
> *"wenn ich 2 seile platziere soll ich erstmal stehen bleiben auf dem boden. wenn ich aber dann w
> drücke (oder in deren richtung laufe) sollen diese seile »aktivieren« und mich beschleunigen und
> in flug modus gehen. nicht nur laufen!"*

**This is the single most important entry in the file.** F-006 air control landed on 2026-08-12 and
it only fires **above** `ground_top_speed_m_s` (6.3333) — so a player standing on the ground with a
rope attached presses W and `ground_locomotion` walks him at 6 m/s. The rope does nothing.
What he is describing: **the ropes are a launcher.** Two anchors + directional input = the ropes
*activate*, accelerate him along/between them, and hand him to flight mode. That is not tuning —
`in_flight`'s threshold is the wrong gate for a roped player, and `FIND-050`'s reasoning (the legs
cannot produce more than the ground's top speed) does not cover "the ROPE is producing it".

### 5c. 🔴 Hook everything, and show where it lands
> *"man soll überall seinen haken inmachen können! auch an den boden oder dächer, alles!"*
> *"und es soll previewd werden wo der aktuelle haken landen würde! also sollte richtig angezeigt
> werden. nicht nur am fadenkreuz. weil das stimmt auch nicht."*
> *"zudem sollen diese weiter auseinander sein. also weiter rechts und links!"*

Three things, all in the aiming layer:
1. **Everything anchorable** — ground and roofs included. Today `anchorable` is per block and the
   ground is deliberately `false` (`maps.ron`: *"otherwise you hook into the ground slabs"*). That
   decision is now overruled by the user.
2. **A real landing preview in the WORLD**, not a crosshair state. `FIND-047` measured that the two
   arm badges are pinned at fixed screen coordinates and never move — he has noticed exactly that,
   and he is right that *"das stimmt auch nicht"*.
3. **The two markers further apart, left and right.** `FIND-039`/`F-023` already say the real answer:
   the candidate set splits into a **left and a right hemisphere**, Q serves one and E the other. His
   "weiter rechts und links" is that feature, described from the outside.

### 5d. 🔴 The district got WORSE, and it was my change
> *"häuser sind alle ineinander! keine unterschiedliche höhen! es sieht überhaupt nicht aus wie eine
> attack on titan map! viel zu kompakt!"*

The 2026-08-12 density rebuild closed the streets to a surveyed 0.62:1 ratio (`FIND-058`,
`gameplay/references.md`) — **and it produced merged blocks with a flat skyline.** He is judging the
result and the result is wrong: party walls made the houses read as one mass, and
`min_height_m 4.5 → 8.0` plus a missing `house_large` roof killed the height variation.
**Do not defend the survey number against his eye.** What he wants is visible height variation and
separated buildings; the swing rule (`d < H`) has to be satisfied *within* that, not instead of it.
`Q-036` (a 24–28 m class) is still the lever that would let the skyline breathe.

### 5e. Everything is flat — colour and light
> *"aktuell sieht man nicht so viel unterschiede. alles sehr flat (auch farben, licht etc)"*

No shadows, one ambient level, a flat sky. Untouched since the first window session, where the same
thing was written down and nothing was done about it.

### 5f. The physics IS the product — ropes accelerate, Shift only adds, and Shift sprints on the ground
> *"es ist wichtig dass es schönes physics movement ist. die seile geben gute beschleunigung! shift
> soll nur mehr beschleunigung geben. auf dem boden soll ich damit rennen können!"*

Three statements, and together with 5b they define the whole movement model:

1. **The ropes are the engine.** They give the acceleration — not gas, not WASD. That reverses the
   current arrangement, where the rope is a passive constraint (`limits = (0, L)`, it pulls only when
   you exceed the length and never adds energy) and every acceleration comes from `boost_m_s2: 34`
   or `air_accel_m_s2: 10`. **A `DistanceJoint` cannot pull you toward an anchor — it can only stop
   you leaving.** Making the ropes an engine is a mechanic that does not exist yet: a reel-in force
   along the rope while the arm is anchored and the player asks for it.
2. **Shift is only MORE acceleration** — a multiplier on what the ropes already give, not the primary
   source. Today it is the primary source.
3. **Shift sprints on the ground.** It currently does nothing at all while `Grounded` — `gas_boost`
   applies `boost_m_s2` along the look direction regardless of state, and `ground_locomotion` then
   overwrites the horizontal component every tick, so on foot Shift is silently a no-op.

⚠️ **This is the design spine, not a feature request.** It touches `vector::reel`, `vector::boost`,
`player::locomotion` and `player::rope` at once, and it supersedes the reel's current shape
(`FIND-013`: the reel *assigns* radial velocity in one tick — an engine has to *add force*, not
overwrite velocity). Design it whole before building any piece.

---

## ✅ ROUND 2026-08-12b — what landed and what is still missing

Integration pass over four streams, each verdict written by an agent that did **not** build the
result. Every number below was re-measured by the integrator against the working tree at
`6b748a2` + 25 modified files (binary confirmed current: `cargo build` → no-op).

### What is now TRUE — one line per stream

- **scripts** — CONFIRMED. `scripts/f-flight-cut.txt` reproduces `25 asserts held, 363 ticks`,
  exit 0, with a real **28.09 m/s cortex cut** (not the 75.0 clamp) off an anchor at
  `0.00 56.00 227.24`; `scripts/game-full.txt` runs the whole tutorial to `MISSION WON`,
  **23 asserts held, exit 0**. No assert was loosened — verified by `diff` of executable lines
  only, and both deliverables were red-checked by reverting them in `/tmp`.
- **stations** — CONFIRMED. The in-hall supply rack is real and reachable: each rack has
  **57 standable 0.25 m cells = 3.56 m² of floor** inside its 4 m reach, recomputed from
  `maps.ron` from scratch; with the four-line script repair applied the hub loop is
  **20/20 asserts, exit 0**, so gas genuinely comes back at the rack. Rule 4 holds — `mission`
  asks, `blades` writes, and a planted `&mut Blades` in `mission::hub` makes
  `f033_a_rack_asks_for_blades_and_never_writes_the_harness_itself` go red.
- **dodge** — mechanism CONFIRMED, economics REFUTED. Through real keypresses in the full app:
  **24 m/s impulse** (dodge 29.667 vs. matched control 5.0–6.5 at the identical tick) for
  **exactly 45 gas**, on both `C` and double-tap `Space`, air and ground, no stub in the chain
  (`net/local.rs:132` → `vector/gas.rs:147` → `vector/boost.rs:288/302` → `Forces`).
- **sortie** — CONFIRMED. `DespawnOnExit(MissionPhase::Active)` clears the field on both arms,
  proven in the running game with a counterfactual: **`titans == 2` at t=548 → `titans == 0` at
  t=634 (Won) → 0 in the hub at t=834**; the lost arm reaches `phase == 5 && titans == 0` at
  t=1096. Against the broken build the same script fails `assert Titans == 0 — measured 1.000`.

### 🔴 REFUTED — the most valuable lines of the round, in full

**1. The dodge is billed at the speed clamp and buys nothing.** `assets/data/game.ron:263
max_speed_m_s: 75.0` is applied as avian `MaxLinearSpeed` (`src/player/mod.rs:157`). Measured,
8 asserts held, exit 0: after five mashed `C` dodges speed = 75.0 and gas = 75.0; after the
**sixth** dodge speed is **still 75.0** (74.9 < s < 75.1) and gas = **30.0**. *45 gas bought
0.000 m/s.* A six-dodge burst is 270 gas → 75 m/s = **3.6 gas per m/s, 1.92× worse than the
reported 1.875**, with the last two dodges buying literally nothing (`assert speed > 100`
failed, measured 75.000). `gas_budget`'s own stated rule — *"the cost follows the effect, not
the button"* — is enforced for the no-direction case and left open for the no-headroom case,
which is the case a Vector Gear flight actually lives in: a pure swing is 17–21 m/s, so three
dodges reach the clamp. The author knew the clamp existed (`game.ron:228` names it) and applied
that knowledge to choosing 24, not to the billing. **No caveat shipped.**

**2. The dodge economics test is a tautology.** `tests/vector_boost.rs:808
f008_the_dodge_is_the_expensive_boost_and_shift_is_the_cheap_one` computes
`gas_dodge/dodge_impulse_m_s ÷ gas_boost_per_s/boost_m_s2` out of `GameData` and asserts ≥ 3.0.
**It runs no system.** It stayed green under all three independent breaks of the dodge
implementation. "A test holds ≥ 3.0" is a fact about the RON file, not about the game — and
refutation 1 is precisely what it cannot see.

**3. A dead test reference ships in game data.** `assets/data/game.ron:175` cites
`tests/vector_boost.rs::f008_the_dodge_is_the_expensive_boost_per_metre_per_second`. No such
test exists. `tools/norms.py` reports clean, so **it does not check test names cited from
`.ron`** — and norms.py was offered as evidence in the same report.

**4. 🟧 REFUTED on the scripts stream.** The report claims "Stage 🟧 for both deliverables" and
two lines later admits "No screenshot taken". Rule 1 needs image **and** number **and**
red-checked test. `docs/images/f071-won.png` is dated 10 Aug and shows a framing that no longer
exists. Honest stage for both: **🟨**.

**5. FIND-065 is a re-discovery, not a finding.** `git show HEAD:docs/FINDINGS.md` already
carries the `--offscreen --screenshot` exit-0-with-red-asserts defect *with its exact one-line
fix* (`src/debug/screenshot.rs:274 exit_when_written` never reads `run.failures`), and
`scripts/f-001-hooks.txt:13` and `scripts/game-full.txt:34` both document it in committed files.
FIND-065 cites none of them. Its "20 of 35" is also not reproducible: 28 scripts name
`--screenshot`, 12 of those also document a headless verdict, so the real count of
screenshot-only scripts is **16**, of which four are red or truncating.

**6. The sortie stream understated its own seam — the hub loop is a one-way door.**
`combat::health::grant` is `Without<Health>`; `Health::heal` has **zero production callers**
(only its own unit tests in `src/shared/state.rs`); `combat::health::down_at_zero`
(`health.rs:63`) is the sole writer of `MovementState::Downed` and nothing writes it back, while
`player/integrator.rs:183-188` `continue`s past anything that is not Grounded/Airborne/Tethered.
**Downed is terminal.** Measured: after a lost sortie the player stands in the hub at
`health < 1`, walks the exact 2.9 s route that reaches the pad in every other run, and at t=1376
is still `phase == 5`. That is not the "limp" the report describes; it is a **loop-breaker**.

**7. B-004 was inverted, not eliminated — and it is still live at HEAD.** One `sed` on the file
the scripts stream shipped kills the process:
`sed 's/^hook right 4.0/hook right 0.74/' scripts/f-flight-cut.txt` → `let go: Released (t=157)`
→ `panicked at avian3d-0.7.0/.../islands/mod.rs:820: Neither body … is in an island`,
**exit 101**, re-measured by the integrator *after* commit `6b748a2` ("a cut on a rope no longer
kills the process"). Bracketed: release at t=157 panics; t=161, 167, 173, 185, 353 are all clean.
The cortex cut is t=153 and `gear.ron: hit_stop_cortex_s 0.12` = 7.2 ticks — **the panic window
IS the impact frame.** B-004 used to panic *after* the frame and be clean *inside* it; it now
panics *inside* and is clean *after*. `docs/BUGS.md:426` marks B-004 FIXED, and its evidence is
a release 293 ticks after the frame — the safe side. Third face, uncovered, player-reachable:
**cut a cortex while roped and let go within 0.12 s.**

### What a player still CANNOT do — ordered by how much it blocks "a finished game"

1. **Cut a cortex on a rope and let go** — releasing inside the 0.12 s impact frame kills the
   process (exit 101). The shipped script's 4.0 s hold hides it by releasing 200 ticks late.
2. **Play a second sortie after losing one** — Downed is terminal, health never resets, no
   revive exists in any domain. The first defeat permanently closes the hub loop.
3. **Fly anywhere but one hand-computed anchor** — the only near-vertical rope in Ashgate is a
   single gantry beam at y=56. Generated housing is 8–11.5 m, and a shallow reel gives ~19 m/s,
   not the 28 the flagship script shows.
4. **Wear a blade down, therefore need the restock at all** — `Blades::break_pair` has **no
   caller anywhere** in `src/` or `tests/`, and `gear.ron: blades.wear_per_hit` has **no reader**
   (only a comment at `src/blades/mod.rs:89`, the field, and the value). The whole station
   repairs a state the player cannot reach. *A system exists; a player cannot use it.*
5. **Deploy at full health after a WON sortie** — health carries over unchanged; nothing heals.
6. **Be safe in the hub** — a titan spawned outside a sortie is cleared by nothing and keeps
   striking: 6 × `strike: player 1 takes 34.0` logged while `phase == 5`.
7. **See where the supply is** — `open_hub` spawns the cyan marker as `Block` 8 × 0.2 × 8 at
   (-42, 0.15, ±6.5) and the opaque 5 × 1.2 × 9 rack sits on top of it. Two 1.5 m ankle-high
   strips stay visible, one of them a dead-end slot behind the rack, and the pad's lower half is
   inside the depot slab. `Block` gets no collider — pure signal, and no image of it exists.
8. **Refuel by walking the aisle** — a 6.5 m sideways step is required and nothing signposts it.
9. **See a dodge happen** — no VFX, no sound, no HUD tell; `grep` finds no reader of
   `GasGrant.dodge` outside `vector/boost.rs`. A 24 m/s displacement with zero feedback.
10. **Tell when a dodge was thrown away** — at 75 m/s he pays 15 % of his tank for nothing: no
    refusal, no click, no flash. Held boost has the same hole at 0.3 gas/tick; the dodge is
    **150× worse per event**.
11. **Dodge away from something behind him** — `S` alone yields `None` by design, so the one
    direction a dodge is for in a Titan fight is the one it refuses.
12. **Spend his tank safely** — no cooldown; `C` is edge-triggered but re-pressable every other
    tick, so 270 gas can go in 0.45 s. `F-008`'s backlog asks for a cooldown ("Anzahl der Dashes
    ist ein Stat"), the user's spec makes gas the only limiter. **That conflict is the user's.**
13. **Trust any picture-class run** — 16 scripts have no headless verdict command at all, and
    four of them (`f-007-boost` 2/13, `p1-overlay` 1/3, `p1-no-overlay` 1/3, `f070-lost`) are red
    or truncating behind an exit 0.
14. **Run the hub script green out of the box** — `scripts/f070-hub.txt` is **exit 1** in the
    working tree (`line 86: assert Gas > 299 — measured 263.701`). The four-line repair is
    proven to turn it 20/20 green; nobody has applied it.
15. **Run the graybox corpus** — `f004-towers` is 17/39 red (17 asserts on gate towers Ashgate
    has no equivalent of); 8 scripts are un-re-aimed and only 3 were fixed. `f070-lost`'s own
    documented `--ticks 19950` truncates (`instruction 7 of 7 is still running`, exit 1).
16. **Measure blades from a script at all** — `debug::script::Metric` has Speed/Height/Gas/
    Titans/Tick/Health/Kills/Phase/Rope and **no Blades**. The blade half's only path to 🟧 is a
    HUD-pips screenshot.
17. **See any of this** — not one deliverable in the round has an image, so **nothing in it is
    🟧**. The verdict-instant pop (bodies vanish on the frame that says WON) is a feel decision
    still made entirely blind.

### Gate tallies — integrator's own runs, 2026-08-12

```
cargo test --test mission --test titan --test vector_boost --test input
  → 27 passed · 20 passed · 22 passed · 20 passed — 89 total, 0 failed
python3 tools/norms.py                       → NORMS: clean (539 checks)
cargo build                                  → Finished in 0.30s (no-op; binary matches tree)

scripts/f070-hub.txt   --hub --ticks 2000            → 1 of 20 asserts failed · EXIT 1
   line 86: assert Gas > 299 — measured 263.701
   MARKs t=92 in-the-hub · 232 gas-burnt · 330 refuelled · 452 deployed · 594 first-kill
         · 753 won · 955 home
scripts/game-full.txt  --mission tutorial --ticks 1600 → 23 asserts held, 1200 ticks · EXIT 0
   MARKs t=684 cut-1 · 805 cut-2 · 926 cut-3 · 1018 won
   ⚠️ shifted ~120 ticks later than the scripts stream's run (cuts were 653/774/895, won 898)
      — same asserts, same verdict, different timing, after `6b748a2`. Nobody has explained it.
sed 's/^hook right 4.0/hook right 0.74/' scripts/f-flight-cut.txt → PANIC, EXIT 101 (B-004 face 3)
```

**Honest paragraph — what went unseen.** Not one image was produced in this entire round, so
every stage in it is 🟨 and the ceiling was never approached. Three of the four streams verified
their own mechanism in the running game and none of them looked at it. The `game-full` timing
shift of ~120 ticks is unexplained and I did not chase it — the asserts are wide enough to
absorb it, which is exactly the property that would let a real regression through unnoticed. And
the round closed with a **known process-killing panic reachable by an ordinary player action**
(cut, release) sitting behind a `BUGS.md` entry that says FIXED.

### 5g. What the reference actually does — read this BEFORE building 5b/5f

Full document: `docs/gameplay/references.md` § "The reference's movement model". Researched from the
developers' own patch record (three years of it, reproduced on the wiki), not from folklore.
**The design round that chose our movement model did not have this — check its build order against
the six points below before building anything.**

1. **The hook is a VELOCITY DRIVE toward the anchor, not a constraint.** Magnitude is an "ODM Speed"
   stat (190 → 210 across patches, max 257.5), the turn rate is a separate "ODM Control" percentage,
   and it blends with a **momentum vector that survives the unhook**. The rope is a visual beam.
   ⚠️ **Ours is an avian `DistanceJoint` that can only stop you leaving.** This is the architectural
   answer to *"die seile geben gute beschleunigung"* and it is not a tuning change.
2. **Boost is an independent camera-directed thrust**, not a rope multiplier — two verbs, hold and
   a double-tap impulse. ⚠️ **The user asked for the opposite** (*"shift soll nur mehr beschleunigung
   geben"* = a multiplier on what the ropes give). **His word wins**; note the divergence and build his.
3. **Its boost is on Space, its sprint on Shift.** The user put boost on Shift. Again his word wins —
   but our double-tap dodge should know the reference's mega-boost goes in the **camera** direction
   while he asked for the **run** direction (`NEXT.md` §1c). Do not silently split the difference.
4. **Hookability is permissive by default** — a collision raycast, with exclusions subtracted one at a
   time (players, invisible tree hitboxes). That is exactly §5c's *"man soll überall seinen haken
   inmachen können"*, and it means our per-block `anchorable` flag is the wrong default.
5. **One reticle serves both hooks**, and the HUD prints a **distance number** — no number means out
   of range. ⚠️ The user wants **two separated previews** (§5c). That is a deliberate improvement over
   the reference, not an oversight; build his, and keep the distance readout idea, it is free.
6. **Momentum is the whole game.** Three years of patches: the recurring complaint is never top speed,
   it is anything that **eats momentum** — rolling, skills, hooks breaking on hit, physics stalling.
   ⚠️ **We have exactly that bug and it is measured**: `FIND-035`, the `min_rope_m` cliff, 38.684 →
   21.480 m/s in ONE tick, and since `B-005` it fires on every fast approach. Fix it as part of the
   movement work, not as a separate chore.

**One more, and it reframes a ⬜ row:** in the reference **damage scales with speed** (a perk needs
you above 70 % of max speed for full damage; another converts 1 % speed into 0.3–0.6 % crit damage).
**Movement and combat are one system there.** Our `F-031` *Geschwindigkeitsabhaengige Schadensformel*
is ⬜ and is far more central than its row suggests — and it is the other half of `Q-031` (whether a
titan's facing matters at all).

## 6. Three evidence images are void — they photograph a city that no longer exists

Found while re-aiming the corpus (`FIND-073`). These were taken in the graybox and the shipped map is
now `ashgate`, so **any 🟧 that rests on them rests on a photograph of a place that is gone:**

| image | why it is void |
|---|---|
| `docs/images/f170-hud.png` | its script's shutter tick moved 200 → 320 |
| `docs/images/f-001-hooks.png` | sha `231e7d86…` is the graybox's; also FIND-022 — it never showed a rope, because `draw_ropes` was an empty stub when it was taken |
| `docs/images/f-007-boost.png` | its whole projected-pixel evidence block is graybox geometry (the 8 m cube's top face at x 404..521) |

They are marked ⚠️ in place rather than deleted — a deleted image loses the record of what was once
claimed. **Retake all three against ashgate**, and re-decode them: the evidence in those rows is
pixel arithmetic (px counts, bounding boxes, sha256), and every number in it has to be re-measured,
not just the picture re-shot.

⚠️ Do this **after** the district rebuild settles (`NEXT.md` §5d — heights and separation are being
changed right now). Retaking them against a map that is about to change again is wasted work.

## 1A. ⚠️ THE HOOK/BOOST SPEC, 2026-08-12 — verbatim, and it overrules several of my decisions

> *"wenn ich mit seilen festhake (was instant sein soll) und w in die richtung drücke will ich dass
> man deutlich mehr geboosted wird. also dass man dort richtig hingezogen wird. wenn man aber a oder
> d drückt wird nach links/rechts geboostet! wenn man zur seite schaut soll die steuerung mitdrehen.
> also wenn ich 45 grad nach links und w drücke dann etwas eingezogen aber auch boost zur seite.
> aktuell wenn ich seil spanne und s drücke werde ich stark zum seil gezogen! das soll nicht sein!
> und das seil muss deutlich deutlich schneller gespannt werden. nicht frame perfekt aber mit ca
> 500m pro sekunde. mit der range 500 meter! aber man soll sehen wie es aufspannt! und es muss mehr
> rechts und links spreaden!! (mit mausrad soll man einstellen können wie weit auseinander es gehen
> darf!) und da wo das seil am ende auch landet soll die markierung hin vom seil, dass man direkt
> sieht wo man sich connected! das ist wichtig. und dann muss das seil auch dahin!!"*

**Nine requirements, and three of them are corrections of mine:**

| # | requirement | today | note |
|---|---|---|---|
| 1 | hooking is **instant** | `hook_speed_m_s: 160`, up to 1.25 s of flight | but see 3 |
| 2 | **range 500 m** | `hook_range_m: 200` (`Q-035`) | ⚠️ forces `world.half_extent_m` ≥ 400 + 500 = **900** |
| 3 | rope deploys at **~500 m/s**, *"nicht frame perfekt … aber man soll sehen wie es aufspannt"* | 160 m/s | **1 s at full range — visible, not instant.** 1 and 3 together mean *fast enough to feel instant, slow enough to see* |
| 4 | **`W` pulls you hard toward the anchor** | `air_accel_m_s2: 10.0`, look-relative | *"dass man dort richtig hingezogen wird"* — much stronger |
| 5 | **`A`/`D` boost left/right** | lateral air control exists at 10 m/s² | needs to be a *boost*, not a nudge |
| 6 | **the controls rotate with the look**: 45° left + `W` = partly reel, partly side-boost | W is look-relative already; the *reel* component is not | this is the mixing rule |
| 7 | 🔴 **`S` must NOT pull you to the rope** | **`S` is a second binding for `REEL_IN` — I added it** | my error: I read *"mit s spannt man nur das seil"* as reel-in. **"Spannen" = keep taut, not haul in.** |
| 8 | the two ropes **spread further left/right**, **mouse wheel** sets how far | both arms share ONE `AimPoint` (`FIND-039`) | this IS backlog **F-023**'s hemisphere split, confirmed by the user |
| 9 | 🔴 **the marker shows where the rope will actually land, and the rope goes there** | markers are **fixed screen badges that never move** (`FIND-047`) | *"das ist wichtig"* |

**8 and 9 are one feature**: per-arm aim (`F-021` discrete anchors + `F-023` hemispheres + `F-026`
the two markers), all ⬜, all in `vector`. The `ArmAim` carrier in `shared` was already designed for
it (`FIND-039`).

⚠️ **Cost of requirement 2, measure before committing:** `half_extent_m` 600 → 900 makes the index
grid 1800 × 1800 m at `cell_m: 8.0` = **50 625 cells against 22 500 — 2.25×**. And
`tests/data.rs`'s flight-time guard is `range / speed ≤ 1.5 s`; 500/500 = 1.0 s passes.

## 1B. THE EXECUTION PLAN for §1A — designed 3 ways, judged 9 ways, 2026-08-13

Produced by a design workflow (4 recon readers → 3 independent designs → 9 adversarial judgements
→ one plan). **W1 lands first and ALONE** — it freezes the key names; W2–W5 then run in parallel
with exclusive file ownership.

| # | item | files owned (exclusive) | acceptance number |
|---|---|---|---|
| **W1** | data, schema, guards | `game.ron` · `scale.ron` · `src/data/mod.rs` · `tests/data.rs` | `cargo test --test data` 0 failed · headless ashgate 900 ticks, mean frame time regression **≤ +10 %** |
| **W2** | R7 (`S` never reels) + wheel→spread | `src/net/local.rs` · `src/shared/intent.rs` · `tests/input.rs` · `tests/multiplayer.rs` | `S` held 120 ticks on a taut rope: Δdistance **≥ −0.05 m** (today ≈ **−56 m**) · 17 notches span 10°→44° |
| **W3** | instant refire + three rays + per-side `AimPoint` | `src/vector/aim.rs` · `src/vector/hook.rs` · `src/shared/gear.rs` · `tests/vector_aiming.rs` · `tests/vector_hooks.rs` | blocked ticks after release **≤ 1** (today up to 100) · at 28°/100 m the two side points are **≥ 45 m** apart · a side ray that finds nothing **falls back to the centre ray** |
| **W4** | the mixing rule + its gas bill | `src/player/locomotion.rs` · `src/vector/gas.rs` · `tests/player.rs` · `tests/vector_gas.rs` | 0°: **40.0 ± 0.5 m/s²** · 45°: radial **28.3**, tangential **7.1** · 90°: radial **0 ± 0.05** · unhooked: **bit-identical** to today |
| **W5** | marker = firing point | `src/hud/arm_aim.rs` · `src/hud/crosshair.rs` · `tests/hud.rs` | `|marker target − fired target| == 0` exactly · both glyphs **≥ 145 px** from centre x (keep-out edge 128) |

**New RON keys** (all `⚠️ UNTUNED`, all guarded, none with `serde(default)`):
`hook_range_m 200→500` · `anchor_range_m 200→500` · `hook_speed_m_s 160→500` ·
`hook_retract_speed_m_s 120→500` (**new guard** `range/retract ≤ 1.0 s`) · `world.half_extent_m 600→900` ·
`aim_spread_deg 28.0` (min 10, max 44, step 2) · `player.air_pull_m_s2 30.0` ·
`player.air_lateral_m_s2 18.0` · `player.air_pull_fade_m 12.0` · `vector.gas_steer_per_s 16.0` ·
`gas_priority` gains `Steer`. **`cell_m` stays 8.0** — `game.ron` says that lever is measured before
it is touched, and we have no measurement.

### The mixing rule, explicit

```
h = translation + Y·eye_height_m ;  per anchored arm i: r̂ᵢ = unit(tipᵢ − h), Lᵢ = |tipᵢ − h|
w⁺ = max(0, move_y)   mx = move_x   l̂ = look_dir()   ê_right = (cos yaw, 0, −sin yaw)   n = anchored
cᵢ = max(0, l̂ · r̂ᵢ)                                    // cosine projection, NOT nlerp (FIND-046)
fᵢ = clamp((Lᵢ − min_rope_m) / air_pull_fade_m, 0, 1)   // W lets go before FIND-035's cliff

look = clamp_len₁( l̂·w⁺ + ê_right·mx ) · air_accel_m_s2 · (gas empty ? air_accel_empty_fraction : 1)
rope = [ (1/n)·Σᵢ r̂ᵢ·air_pull_m_s2·w⁺·cᵢ·fᵢ + ê_right·air_lateral_m_s2·mx ]  if n>0 && grant.steer
a    = look + rope                                      // additive; the pull is OUTSIDE clamp_len₁
```

**Three judge-forced corrections, each a trap this project has already paid for once:**
1. **cosine projection, not `nlerp`** — `nlerp` is `FIND-046`'s 90°-off-look bug;
2. **per-arm `r̂ᵢ`, never the mean** — two opposed ropes average to zero and degenerate;
3. **`A`/`D` on the horizontal `ê_right`, not the rope tangent** — a tangent **flips sign** when the
   anchor passes beside you, inverting the strafe mid-swing.

**All nine judges named the same biggest flaw: the new thrust was free.** It is now a fourth
`GasConsumer::Steer` at 16/s, priced so 16/30 ≈ boost's 18/34 — the same gas per m/s the player
already pays. On an empty tank the look term halves as today and **the rope term is zero**.

⚠️ **The wheel carries an ABSOLUTE angle, not a delta** — a delta desyncs over the network and never
re-converges (rule 4).

## 1C. Three loose ends from W1–W4 (2026-08-13) — small, and one is a real bill

1. 🔴 **A grounded player is billed for thrust he does not get.** `wants_steer` is
   `n>0 && (w⁺>0 || mx≠0)` with **no flight-state term**, so standing on the ground with a hook in a
   wall and holding `W` costs **16 gas/s** while `air_control` produces nothing (it only runs above
   the ground top speed). Implemented as the spec was written, flagged rather than silently fixed
   (`FIND-085` §3). **The repair needs `vector → player`** (to read `locomotion::in_flight`) — an
   allow-list edge, i.e. a decision. The alternative is to move the want into `player`, which
   already knows the flight state.
2. **`GasGrant` crossed three ownership lines.** `Steer` needed five files, not the three
   `FIND-082` predicted: `GasGrant` lives in `src/shared/gear.rs` (W3's, and live at the time),
   `tests/data.rs` asserted `len() == 3`, `tests/vector_boost.rs` builds `Wants`/`Costs` literally.
   ⚠️ **Verify at the gate that `pub steer: bool` survived W3's concurrent edits to `gear.rs`.**
3. **My commission contradicted its own formula and W4 caught it.** The acceptance bullet said
   "pull == 0 while `L ≤ min_rope_m + air_pull_fade_m`"; the formula
   `f = clamp((L−min_rope)/fade,0,1)` is 0 **at** `min_rope_m` and **1** at `min_rope_m + fade`.
   The literal bullet would have killed the pull across the whole 5–15 m arc a swing lives in.
   The formula won, both ends are pinned by a test, and `game.ron`'s own comment ("full strength at
   15 m, zero at 3 m") independently agrees. **No action — recorded so nobody "fixes" it back.**

## 1D. ⚠️ THE CITY + SHELL SPEC, 2026-08-13 — verbatim, and item 10 reverses a founding decision

> *"dann implementiere sinnvollere 3d modelle und adde verschiedene höhen vom boden her! lass es wie
> die echte stadt aussehen! aktuell kann man es noch nicht erkennen! es sind random türme da die
> nicht so sein sollte. also nicht wie im anime! bitte geh nochmal komplett drüber! zudem fehlen
> settings. menu (also bei escape) und eine main lobby in der man die mission starten kann und auch
> ein attack system mit gegnern! und es ist extrem wichtig dass man wirklich überall sein seil
> festmachen kann. also überall! das ist wichtig! ohne ausnahmen!"*

| # | requirement | today |
|---|---|---|
| 1 | **sensible 3D models** | every visible thing is an untextured cuboid; `art.ron` can load glTF but **no `.glb` exists** |
| 2 | **varied ground heights** | the ground is **one flat slab**, `(0,−0.1,0) 400×0.2×400`. There is no terrain at all |
| 3 | **"look like the real city — you can't recognise it yet"** | dense blocks, flat roofs, orthogonal grid |
| 4 | 🔴 **"random towers that should not be there — not like the anime"** | the **58 m swing gantries**. He has now rejected them outright — `Q-036` is answered by deletion, not by a height number |
| 5 | **go over the whole thing again** | — |
| 6 | **settings** | do not exist |
| 7 | **menu on Escape** | Escape pauses with Resume/Quit only |
| 8 | **main lobby to start the mission** | the hub exists as a place; there is no menu/lobby UI |
| 9 | **attack system with enemies** | one husk kind, one telegraphed strike, no variety |
| 10 | 🔴🔴 **"you must be able to hook EVERYWHERE. without exception."** | **the opposite is a founding rule** |

### Item 10 is the one that changes the most, and it must not be done carelessly

`F-003` is *"Getaggte Ankerflaechen"* — **tagged** anchor surfaces — and `anchorable: false` is load
bearing in at least five places: the ground slab (*"otherwise you hook into the ground slabs"*), the
canal, the gate columns (`FIND-042`: an anchor over solid ground ends a swing inside the thing you
hang from), interior faces, and the untagged wall in `tests/vector_aiming.rs` that proves the aim ray
does not shoot through geometry.
**The user has now overruled that.** Everything is hookable, no exceptions.
**What has to be re-decided, not just flipped:**
- `F-003`'s whole acceptance sentence ("no hook on untagged parts") is void — the row needs rewriting,
  not a stage change;
- `tests/vector_aiming.rs::f002_an_untagged_wall_in_front_of_a_roof_is_not_hookable_and_not_transparent`
  asserts the old rule. **The transparency half must survive** (a ray must still not pass *through*
  a wall) even though the hookability half dies;
- the **ground** becomes hookable — measure what that does to a player who fires at the pavement;
- `maps.ron`'s `anchorable_fraction` and the whole tagging vocabulary stop meaning anything.

**Item 4 answers `Q-036` by deletion.** The gantries came from `FIND-058`/`FIND-041` (`d < H`, an
anchor must stand over open ground). Removing them without replacing the traversal they carry puts
the district back to `FIND-026`'s dead rope — **unless item 10 changes the arithmetic**, which is
exactly what has to be measured before they come out.

### ⚠️ Item 10 CLARIFIED by the user, 2026-08-13, minutes later — the check stays

> *"es soll später auch stark vereinzelt dinge geben die man nicht anchorn kann. aber sehr wenig!
> also kann der check drin bleiben"*

**So it is a DEFAULT FLIP, not a mechanism removal.** `anchorable` stays as a flag, the aim ray's
filtered cast stays, `F-003`'s machinery stays — what changes is that **anchorable becomes the
default and `false` becomes a rare, deliberate exception.**

That is strictly safer than the first reading and it removes three of the four risks §1D listed:
- the **transparency guarantee** needs no special handling — the filtered cast is untouched, so a ray
  still cannot pass *through* a wall;
- `tests/vector_aiming.rs::f002_an_untagged_wall_in_front_of_a_roof_is_not_hookable_and_not_transparent`
  keeps **both** halves; it just needs a surface that is still deliberately untagged to point at;
- `F-003`'s acceptance sentence survives — "no hook on untagged parts" is still true, there are simply
  far fewer untagged parts.

**What actually has to happen:** invert the default in `maps.ron` (and in the generator, which sets
`anchorable_fraction` — that key's meaning changes from "how much is hookable" to nearly 1.0), then
go through the current `anchorable: false` blocks one at a time and justify each survivor. The ones
with a real reason are the **ground slab** (hooking the pavement under your feet) and the **canal
bed**; the gate columns' reason (`FIND-042`) dies with the gantries if those are deleted.
**Rare exceptions are a design tool, not an oversight — each survivor gets a comment saying why.**

---

## §1E — the 2026-08-18 session, and the two things the user said

Both came in chat rather than through `user-messages.md`, so they are recorded here verbatim —
his phrasing carries information a paraphrase loses (CLAUDE.md, "quote him verbatim").

### 1 · The rope spread — ✅ DONE, and it exposed a units bug

> *"der spread für seile ist zu weit auseinander und sollte mehr dynamisch sein!"*

**He was right twice over, and the second half was a bug nobody had grounds to settle.**
`aim_spread_deg: 28.0` was read as a **half**-angle, so the two ropes opened **56°** and landed
**93.9 m apart at 100 m** — a fixed angle makes the metric separation grow without bound
(278 m apart at 200 m on the widest notch). `FIND-083` had recorded that the RON comment and
`docs/NEXT.md` §1B disagreed about half-vs-full by a factor of two, and it sat open because
nothing could break the tie. **His complaint is the tiebreaker**, and `FIND-086` is now closed:
the wheel is the angle **between** the rays, stated in exactly one place (`wheel_half_rad`).

What shipped, verified by adversaries who reproduced the arithmetic independently:

| at | before | after (grounded) | |
|---|---|---|---|
| 10 m | 9.39 m | **3.33 m** | −64.5 % |
| 25 m | 23.47 m | **8.33 m** | −64.5 % |
| 100 m | 93.89 m | 33.33 m | −64.5 % |

- **Near field is governed, not defaulted.** `aim_sep_full_reach_m: 108.0` (= 3 · `lot_m`) makes
  the metre budget scale with distance below that range, so `d` cancels and the near field is a
  **constant angle per state** (9.6° standing, 11.2° searching). **The wheel cannot undo it** —
  at the widest notch 10 m still gives 5.24 m, 35 % under the *old default* and 56 % under the
  old max. Far field stays constant-metres, which is what killed the runaway.
- **Dynamic** — at a fixed 100 m the fan spans 3.44°..28° with state and speed: tight swinging
  and boosting, open standing and searching, widest at open sky.
- Two bugs found on the way: the wheel clamp ran **before** the slew and was never re-applied
  (a 44→10 notch drop left the fan wider than allowed for ~0.19 s), and the HUD's keep-out box
  pinned both markers to a **fixed slot** so the narrower fan was invisible (`FIND-098`), which
  in turn uncovered a **608 px marker teleport on every shot** — the flying marker was given the
  rope's tip, which starts *in the hand*, on the near plane (`FIND-099`).

**Still open here:** the fallback flip (side ray misses, centre ray anchorable) moves the glyph
85–105 px with **no change of shape or colour**, so near a roof edge it can strobe. Fixing it
needs a fifth `ArmAimState` or a colour, and both move `F-171`'s photographed table — **it wants
his eyes in the running game before anyone picks.**

### 2 · The art pack — models exist, the town is not dressed yet

> *"in downloads sind defeated-by-titan assets. in zip. nutze diese!"*

**278 `.glb` + 17 atlases, installed and git-tracked** (341 files). The pack is authored **in the
game's exact metres** — `fachwerkhaus` klein/stadthaus/gross measure 4.5 / 8.0 / 11.5 m against
`scale.ron`'s `house_small` / `house_town` / `house_large` to the decimal — and it carries
machine-readable metadata the loader was already written for: `hit.min`/`hit.max` on all 278,
`cortex` on 45, and **439 named `hook.*` points across 144 files** (`traufe`, `first`, `krone`,
`spitze`, `gesims_15..105`).

⚠️ **The pack's own `assets/data/*.ron` were NOT copied** — they are the 2026-08-09 German
scaffolding (`skala.ron`), long superseded. ⚠️ **`assets/texturen/` must never be renamed**: every
`.glb` references `../../texturen/TEX-*.png` by relative URI. ⚠️ **Zero animation clips** — the
rigs are there, nothing is authored, so `animations: {}` stays everywhere and a titan will stand
rather than walk.

**A titan renders and is measured** (`docs/images/t075-titan.png`): span 384 px against 391.9
predicted (−1.5 %), implied height 9.853 m against a class 10.0, feet +0.119 m off the ground
plane. That is 🟧 and it is the only model that has reached it.

**Why the shipped game is still grey**, all five found and being cleared:
1. 🔴 `src/lib.rs` sets no asset root — Bevy resolves `assets/` against the **exe** dir, so
   `cargo run` works and `./target/debug/defeated_by_titan` does not. Every script run uses the
   direct binary, so every model-bearing run would render nothing **and exit 0**.
2. 🔴 The pack's `cortex` empty sits 0.139 m off the neck axis; binding it naively moves the kill
   zone ~0.4 m forward and **a husk becomes cuttable from the front** (`q030`/`q031` red).
   The design rule wins: model decides the height, rig keeps the nape depth.
3. `ANCHOR_NAMES` discards all 439 `hook.*` points — the whole anchorable surface of the
   architecture kit, in a grappling-hook game.
4. `world::map` spawns every building as a bare `Block` (size + colour, **no kind**), so there is
   nothing to hang a `ModelName` on — which is why 8 registry rows have models and no spawner.
5. `tools/norms.py` reports 317 orphans; the rule cannot survive a 278-file pack as-is.

**Not started, and deliberately:** the towers he rejected (*"es sind random türme da die nicht so
sein sollte"*) still stand — deleting them blind puts the rope back to contributing nothing
(`FIND-026`), so the replacement has to be measured first. And **animation**, which the pack
cannot supply.

### 3 · Two convention debts the art round left, both found by `tools/norms.py`

Neither is behaviour, both are the kind of thing that quietly rots `docs/STATUS.md`.

**a · The model tests are filed under the wrong feature.** `tests/render.rs` names ten tests
`f030_*` — but in `docs/features.ron` **`F-030` is "Nape-Trefferzone (Cortex)"**, the nape hit
zone, and the tests are about the model registry. Nothing in the backlog covers the registry at
all: it is **infrastructure**, so it wants a `T-` id (the existing ones run to `T-074`, and
`tests/` already uses `t003_`/`t005_`/`t036a_` for exactly this). Left as-is for now only because
a workflow was live in `tests/render.rs` — **whoever touches that file next renames them**, or
`F-030`'s evidence column will keep claiming model screenshots as proof of the cortex cut.

**b · The new screenshots break the naming rule.** `docs/images/t075-titan.png`, `-town.png`,
`-street.png`, `-player.png` do not match `<f-id>-<short>[-before|-after].png` (§10). They should
carry whatever id (a) settles on. `docs/images/t075-titan.png` is currently the **only** model
picture with a measurement behind it (384 px vs 391.9 predicted, feet +0.119 m), so it is worth
renaming rather than retaking.

### 4 · The tower measurement is already written and has never been run

Two orphans that `tools/norms.py` caught, both real work from 2026-08-13, neither of them scrap:

- **[`scripts/w5-lane.txt`](../scripts/w5-lane.txt)** — a complete, documented run that answers
  §1D item 4 (*"es sind random türme da die nicht so sein sollte"*) **with numbers instead of
  taste**. Rope only: no boost, no reel, no `W`, and `assert gas == 15000` at the end proves every
  metre of height and every m/s came out of the rope and gravity. It measures whether the town
  now carries the swing lane by itself, which became a live question when the district went
  **100 % anchorable** and grew terrain and gable roofs — the arithmetic in `FIND-041`/`FIND-058`
  predates all three.
- **[`docs/images/f003-skyline-before.png`](images/f003-skyline-before.png)** — the *before* half
  of the roofscape work. There is no `-after` yet; whoever takes the skyline round shoots one.

**This is the cheapest open item in the file:** the script exists, the criterion is in its header,
and the answer decides whether the gantries can be deleted or must be replaced by architecture.

---

## §1F — 🔴 THE MAP IS THE WRONG PLACE. The user, 2026-08-18:

> *"hast du die map schon überarbeitet? weil aktuell ist das nicht die echte map!"*

**He is right, and nobody had noticed.** This is not a look complaint — it is a setting
complaint, and it is checkable in one line.

[`docs/gameplay/world.md`](gameplay/world.md) states the premise:

> Humanity lives in **three concentric bastion rings** — **Ashgate** outside, **Ironrose** in the
> middle, **Highspire** inside. […] **The central difference to the source material: the war is
> already lost.** […] **Ashgate has long since fallen**; the Vanguard runs **salvage missions into
> its own ruins**, not campaigns of reconquest.

**What is actually built is an intact, inhabited, tidy walled town.**

```
grep -ci 'ruin|rubble|collapse|fallen|debris' assets/data/maps.ron   ->  0
maps in maps.ron                                                     ->  2 (graybox fixture, ashgate)
ruin/rubble models in the pack, all unused                           ->  14
```

The art drop ships the whole kit and the map uses none of it: `a-089-ruine-{dach-eingestuerzt,
dach-haelfte, giebel, haufen, obergeschoss, pfeiler, wand-ecke, wand-hoch}` and
`a-090-schutt-{balken, deckung, flach, haufen-gross, hoch, wandstueck}`.

### ⚠️ And the 2026-08-18 session made it worse, not better

Everything landed on the map that day pushed it **further from the design**: the district got
denser (926 houses, closed blocks, party walls), got **terrain** (6 levels, 3.00 m relief), got
**gabled roofs** and an 18 m `house_tall` class, and got an **enterable, furnished HQ**. Every one
of those answers *"lass es wie die echte stadt aussehen"* — and every one of them makes Ashgate
read as a **living town**, which is exactly what the design says it stopped being a century ago.

**The two complaints are not the same complaint**, and conflating them is how a day of good work
went in a direction the bible forbids:
- *"lass es wie die echte stadt aussehen"* → proportion, density, roofs, materials. **Done.**
- *"das ist nicht die echte map"* → **the place is wrong.** A fallen outer ring, not a market town.

### What that means concretely, in build order

1. **Ashgate has to fall.** Collapsed roofs, half-standing gables, rubble in the streets, blocked
   lanes, no market stalls and no lit lanterns. The kit exists; the generator has no notion of
   damage. ⚠️ **This is a traversal change, not a decoration change** — rubble alters what a rope
   can reach, and Ashgate is 100 % anchorable on purpose (§1D item 10), so a collapsed facade must
   still be hookable or the two rules fight.
2. **The salvage premise has to be visible.** Carts, crates, the objects the missions are *about*
   (`missions.ron` already knows about rescue and hold objectives). The pack has `a-087` stalls
   and prop atlases for exactly this.
3. **Ironrose and Highspire do not exist.** Two of three rings are unbuilt. Whether they are
   needed before the P1 movement gate is a real question — the gate is about *movement*, and one
   district can carry it — but the hub currently pretends to be inside a world that has one place.
4. **Sky and fog are still missing**, and `world.md` says so itself: *"Neither exists yet […]
   uniform dark gray upper half: that is Bevy's ClearColor, not a sky."* An elegiac ruin reads
   through **fog and light** more than through geometry, so this is closer to the theme than to
   the renderer.

**ASSUMPTION until he says otherwise:** Ashgate stays the one playable district and gets **ruined
in place**, rather than a second map being built. Rollback point: `assets/data/maps.ron`'s
`ashgate` block and whatever generator flag carries the damage.

---

# §2 — TOMORROW'S QUEUE (2026-08-19), from the user after playing

He played the build and wrote, 2026-08-18 late:

> *"ok bis jetzt deutlich besser. attack fehlt aber noch (mit schwertern..) die accuracy von
> anzeige zu wo seil landet ist nicht immer korrekt (was nicht gut ist) und teilweise kann man gar
> nicht usen weil keine ahnung wieso. es sollte best match sein. und seinstellen können wie weit
> ca es sein sollte und wie aggressive (damit ich testen kann was am besten wäre mach debug
> einstellungen dafür)"*

and, a minute later:

> *"zudem fehlen noch die häuser. mach das nicht jetzt. schreieb das auf für morgen"*

**⚠️ Read this section before designing anything. Four of his five complaints are already
specified features that were never built** — `FIND-039`'s lesson exactly (a feature re-derived
without reading the backlog came out worse than the spec). **Two of them match his words almost
literally.**

## §2A — 🔴 THE ANCHOR CANDIDATE SYSTEM. Five unbuilt features, one round.

Today the hook is a **pure raycast** (`F-002`, built): where the ray hits is where the rope goes.
The backlog specifies a whole **candidate-and-scoring** layer on top of it, and **none of it
exists**. His four aiming complaints are its four acceptance criteria.

| his words | feature | stage | the spec's own acceptance |
|---|---|---|---|
| *"teilweise kann man gar nicht usen weil keine ahnung wieso"* | **F-028** Fallback ohne Kandidat | ⬜ | *"Kein Tastendruck bleibt ohne Rückmeldung; der Spieler versteht immer, warum kein Haken gesetzt wurde."* |
| *"accuracy von anzeige zu wo seil landet ist nicht immer korrekt"* | **F-026** Highlighting der Ankerpunkte | ⬜ | *"Ein Testspieler kann jederzeit ohne Nachdenken sagen, wohin Q und E ihn bringen würden."* |
| *"es sollte best match sein"* | **F-024** Snap auf Q und E · **F-025** Bewertungsfunktion | ⬜ | angle deviation **45 %**, momentum preservation **25 %**, height advantage…; *"wählt nie einen Punkt hinter dem Spieler, wenn ein brauchbarer vor ihm liegt"* |
| *"einstellen wie weit ca und wie aggressive"* | **F-016** Ziel-Assist-Regler | ⬜ | a **0–100 % stepless snap catch angle**; 0 % = today's pure free aim |
| (density, so the view does not silt up) | **F-027** Marker-Dichtebegrenzung | ⬜ | max 12 markers, opacity and count in the settings |

**`F-024` also specifies the three aim modes he is asking to test between:**
**FREI** (no snap, pure raycast — today's behaviour), **ASSISTIERT** (snap only inside `F-016`'s
catch angle — *the default*), and a third full-snap mode. *"Moduswechsel ist ohne Neustart
wirksam."*

**So his "mach debug einstellungen dafür" is `F-016` + `F-024`'s mode switch, and the settings
screen built on 2026-08-18 is where they go** — it already carries live sliders (sensitivity, FOV,
aim spread) that take effect within a tick, seeded from `game.ron` with no new RON keys.

⚠️ **The accuracy complaint may ALSO be a real bug, not only a missing feature — check both.**
`F-023`'s whole promise is that the marker and the rope are one number, and `src/vector/aim.rs`
has exactly one `side_dirs` caller so they cannot diverge *in principle*. But `FIND-099` found the
flying marker was drawn from the rope's **tip** (which starts in the hand, on the near plane) and
`FIND-098` found the keep-out box pinning markers to a fixed slot — **two marker lies in one day,
both real, both invisible to their own tests.** A third is likelier than not. Start with a repro,
not with a design: `docs/BUGS.md`, rule 5.

**Suggested order:** F-028 first (it is the smallest and it answers *"keine Ahnung wieso"*, which
is the most frustrating of the four), then F-025 + F-024, then F-026, then F-016 + F-027.

## §2B — ⚔️ THE ATTACK. *"attack fehlt aber noch (mit schwertern..)"*

**The mechanism exists; what is missing is everything that makes it READ as an attack.**
`Buttons::SLASH_LEFT`/`SLASH_RIGHT` are declared and bound to the mouse in `src/net/local.rs`,
`src/blades/swing.rs` (338 lines) and `cut.rs` are built, and `F-030` (cortex) and `F-034`
(hit-stop) are 🟧 Proven. So a cut lands — but:

- **No blade is visible.** `art.ron: "blade"` is still `source: Primitive`. The pack has
  `a-023-klingengriffe`, `a-024-klingen-paar-neu`, `a-024-klingen-paar-gebrochen` and **ten**
  `a-025-klinge-kosmetik-*` finishes.
- **No swing animation.** The pack has **zero animation clips** — a swing has no motion to play.
- **Only the nape counts.** `F-032` Sekundäre Trefferzonen ⬜ — a blade in a titan's arm or leg
  does nothing at all.
- **No hit feedback.** `F-043` Schadenszahlen und Trefferfeedback ⬜.
- **No combo, no ground melee.** `F-041` ⬜, `F-044` ⬜.

**Hypothesis to verify first, not a claim:** he swings, the cut works mechanically, and nothing on
screen tells him so. **Cheapest honest test:** bind a debug readout of `Swings`/`BladeTiming` and
watch one sortie — before building any of the five features above.

## §2C — 🏚️ THE HOUSES. *"zudem fehlen noch die häuser"* — deferred by him to tomorrow

The plumbing is done and one thing blocks it: **`BlockPlan::spawn` never inserts `ModelName`,
because `ModelName` lives in `src/render/model.rs`** and `world` inserting it would need a
`world -> render` edge that `docs/architecture.md`'s allow list does not have.
**The project's own rule answers it:** shared component types live in `shared/` so a receiver
never needs an edge to its sender. Move `ModelName` there and the rest already exists —
`BlockPlan.model: Option<&'static str>`, the `DRESSING` table (`house_small` 6.56×4.50×8.32,
`house_town` 9.10×8.00×7.90, `house_large` 8.30×11.50×9.90, all measured against the real models)
and `dress_for`, which refuses to dress a name still on `Primitive`.

⚠️ **Do this together with §1F (ruining Ashgate), not before it.** Dressing 926 houses as intact
`fachwerkhaus` and then ruining the district throws the work away twice — the ruin kit
(14 unused models) and the house kit go through the same `BlockPlan.model` mechanism, so the
distribution is **one design decision asked once**.

## §2D — still open from today, unchanged

- **Commit.** 375 files, including the whole asset pack, are uncommitted. `target/` was deleted to
  free disk, so this costs a full rebuild (~20 min) plus the gate before anything is staged.
- **`scripts/w5-lane.txt`** has never been run — it answers the tower question with numbers.
- **Two label repairs** owed: `src/shared/settings.rs:59` still says *"Half-angle"*,
  `src/menu/settings.rs:99` prints *"deg max"* where it now means *"deg apart max"*.
- **Sky and fog do not exist** (`docs/gameplay/world.md` says so itself).

### §2A footnote — a third, narrower candidate for *"teilweise"*, unverified

`net::local::read_input` runs in `FixedPreUpdate` and samples `keys.pressed(..)` — a **level**,
not an edge (the edge is computed inside the sim against `PrevButtons`, which is correct). Bevy
runs the fixed loop **zero or more times per frame**, so at a high frame rate a tap shorter than
one 16.7 ms sim step can fall entirely between two fixed steps and never be sampled.

**Weaker than B-006 and B-007 and listed below them on purpose** — a normal tap is 30–80 ms and
spans two to five sim steps. Worth one measurement, not a round: log every `Buttons` edge the sim
sees against every physical key event for a minute of play, and compare the counts.

### §2D update — 🔴 the tower measurement cannot run: its scripts aim where they no longer point

Found 2026-08-19 while measuring `F-025` (`docs/FINDINGS.md` FIND-108).

`scripts/w5-lane.txt` and `scripts/f004-towers.txt` were written on 2026-08-13, when
`aim_spread_deg` was a **fixed** offset and `side_angle_rad` read it as a half-angle. **Every
`look` line in both files is hand-compensated by ±28°** for that offset — the header of
`w5-lane.txt` says so in as many words and explains the pitch correction
`asin(sin(pitch)·cos 28°)`.

**That offset stopped being fixed on 2026-08-18** (`FIND-096`: the fan is now a resolved
separation in metres, state- and distance-dependent, 3.4°..28° at 100 m). So the compensation
now points the shots into nothing:

```
w5-lane.txt ACT A leg 3:  its own table says 39.250 m/s  ·  measured today 1.202 m/s
```

**Consequence: the tower question is still unanswered and is now MORE expensive than §2D said.**
The script is not merely unrun — it is wrong, and re-aiming it is the job. Two honest routes:

1. **Re-aim both files against the resolved fan.** Tedious, and it bakes today's numbers in
   again — the next tuning of the spread breaks them a second time.
2. 🟢 **Aim them with the assist instead.** `settings assist_strength 100` now exists as a script
   verb (2026-08-19), and at full snap **the pitch of a shot barely matters** — 20° to 50° picked
   the same anchor, because a snap chain is aimed by **release time**, not by angle (FIND-108).
   That makes a lane script robust against every future spread change, which is exactly the
   property both files failed to have.

**Route 2 is the recommendation** and it is cheap: `scripts/f025-chain.txt` already proves the
shape works (5 swaps, 16.9 → 45.2 m/s, `rope == 1` on every leg).

⚠️ **And `f004-towers.txt`'s 16-of-39 red is not a map bug.** It has been misread as one at least
once (`FIND-096`); it is this same stale compensation. Nothing is wrong with the gantries that
this explains — which means the user's *"random türme die nicht so sein sollten"* still has to be
answered by measurement, not by the fact that the old script fails.

### §2E — the roster fights, and one kind still cannot spawn

Landed 2026-08-19. `docs/gameplay/enemies.md` names **8 kinds**; **7 now fight differently**, each
by a `titan.ron: <kind>.behaviour` switch read once at spawn and enforced in `brain.rs`. The proof
of each is a **kind-vs-husk pair in one app**, and each went red on a one-line RON edit —
[`scripts/f051-kinds.txt`](../scripts/f051-kinds.txt) holds the played version (7/7 asserts, exit 0):
10 s at 20 m costs **0** damage against the lurker and **34** against the husk, while 0.42 s at
**5 m** costs **48** against that same lurker.

⚠️ **The `bellower` still cannot spawn** — `scale.ron: max_spawnable_class` caps at `large`, and
raising it is no longer the one-line change `art.ron:183` has claimed since it was written: it
reddens three assertions in `tests/titan.rs::f064_*`. The exact diff is in `FIND-118`. And even
raised he is half a kind, because `F-051` (gas noise, the thing he is supposed to call *about*)
does not exist — he would call on sight.

⚠️ **Every number in the roster is UNTUNED.** Seven kinds' worth of swerve, lunge, guard window,
flank offset and facing cone were chosen to be *distinguishable*, not to be *good*. That is the
same state the aim-assist knobs are in, and it wants the same answer: the user plays and says which
ones feel wrong.

**`FIND-119` is the round's real lesson:** an ambusher that cannot turn swings at empty street.
The test that proved "the lurker does not chase" was green and blind to it; **writing the script
found it.** A behaviour test that only measures the thing it switched off cannot see what the
switch broke next to it.

### §2F — 🔴 a proven claim is contaminated, and the fix is being deliberately withheld

`FIND-123`, 2026-08-19. **One swing kills a warden.** The graze that knocks his hand off the nape
and the cut that goes into the nape are **the same swing** — `blades::cut::sweep` reports both
zones from one pass, and `receive_hits` opens the guard on the first and honours the second.

`F-060`'s designed version says the guard opens on a **frontal attack on the arms**, and that is
**one `matches!` in `receive_hits`**. It was built, it works, and the round **took it back out** —
because four 🟧 rows (`q030` ×2, `q031`, the `f030` model test) reach the warden's cortex *only*
because their own torso graze opens him. Landing the correct rule would redden all four, and this
project does not trade a proven claim for a new feature.

⚠️ **The consequence is worse than the bug: `Q-031`'s 0.15 m nape margin is measured through the
contaminated path.** That number is what the warden's whole "the nape survives a titan who tracks
you" claim rests on, and it was taken while a single swing could open and cut in one motion. **It
has to be re-measured once `F-060` lands properly**, and until then it should be read as an upper
bound, not a fact.

**What has to happen, in order:** re-aim `q030`/`q031` so they open the guard **explicitly**
(a separate arm pass) instead of relying on their own torso graze → land `F-060`'s one-line rule →
re-measure the 0.15 m → then `q031` means what it says. `f060_…` already asserts that an arm cut
opens him, so the day the rule lands that half is held.

**This is the second time today a proven row turned out to rest on something that moved**
(`FIND-113` was the first: `F-030` and `F-034`'s scripts had been red for days). Both were found by
work that had no reason to look. **The pattern is worth a habit: when a round touches a mechanism,
it should re-run the 🟧 evidence that depends on it, not only its own tests.**

### §2G — ✅ the tower question is answered, and the answer is "they stay"

Measured 2026-08-19 (`FIND-126`), after `scripts/w5-lane.txt` was re-aimed with the snap assist so
all three acts are compared by one aiming method:

| lane | top speed of a rope-only chain |
|---|---|
| the **gantries** | **32.6 m/s** |
| the wall gallery | 26.6 m/s |
| the **town roofscape** | **1.35 m/s** — a chain dies standing still after three legs |

**The user's *"es sind random türme da die nicht so sein sollte"* cannot be honoured by deletion.**
The town does not carry the lane, even now that it is 100 % anchorable with terrain and 18 m
ridges — the roofscape number is not "worse", it is *nothing*. Removing the gantries returns the
district to `FIND-026`'s dead rope, which is the state the whole feature exists to prevent.

**So the question changes shape** and it is his to answer:
1. **Replace them with architecture that holds a 35 m pitch** — the design's own vocabulary has
   candidates the pack already ships: a **bell tower** (`church` is 35 m in `scale.ron` and is a
   LANDMARK, not a grid house), a **gatehouse**, a **granary**, `a-095`/`a-096` wall works. A 58 m
   scaffold reads as game furniture; a 35 m spire reads as a town. **`FIND-041`'s arithmetic is the
   constraint: a usable arc needs the horizontal gap smaller than the anchor height**, so a 35 m
   anchor buys roughly a 30 m pitch, not 35.
2. **Or keep them and make them belong** — dress them as cranes, hoists, wall lifts (`a-099-mauerlift`
   exists), and they stop being "random towers".
3. **Or accept a slower district.** The gantry lane at 32.6 m/s is what the movement gate will be
   judged against; the gallery at 26.6 is the honest fallback.

⚠️ **The lane also lost the bottom 4 m of its swing room to the terrain**, and the gantry chain is
**3 legs now, not 5**: at 44° leg 4 takes the beam two stations ahead and its arc bottom goes under
the raised floor; at 48–52° it snaps to the mast of the gantry it is swinging on. So even the
baseline degraded when Ashgate gained relief — nobody noticed until the file was re-aimed.

### §2H — 🔴 the swing leaves the screen exactly when it cuts

`FIND-127`, 2026-08-19, and it is the strongest surviving candidate for the user's
*"attack fehlt aber noch (mit schwertern..)"* — stronger than the missing model was, because the
model landed and the complaint would survive it.

**The cut is cast at 90° from the view direction.** `fov_deg` 60 on 16:9 gives a horizontal
half-frustum of **45.7°**. So on the **eight ticks out of twenty-one** where `Swing::is_active` is
true — i.e. exactly the ticks that can land a hit — **the steel is outside the frustum by 44°.**
The player swings, the blade leaves the frame, a titan dies. Nothing he can see connects the two.

**This is the cut's geometry, not the drawing's.** `blades::cut::blade_segment` has always cast
sideways; the blade round only made it visible that it does. A **forward arc** would fix the
picture and the feel together — and it changes what `F-030` casts, so it is its own round:
`scripts/f030-cortex.txt` and `q030-reach.txt` have to be re-measured against it, and both are the
evidence behind 🟧 rows (`FIND-113` is the warning that they go stale silently).

**Two smaller things from the same finding:**
- **The drawn blade is 0.93 m where `reach_m` casts 1.60 m** — the picture under-promises the reach
  by 0.67 m, so a player learns a shorter weapon than he has. `gear.ron`'s `reach_m` is 🟧-adjacent
  and was not traded.
- **The pair is one merged mesh** (`a-024-klingen-paar-neu`) with no single-blade file in the drop,
  so both sides swing together and only one reads in frame. That is an **asset** question for the
  user, not a code one.

**And a latent mine the round removed on the way**, worth recording because it would have been
maddening: `render::attach_camera` selected on `Without<Children>`, and the blade is the **second**
thing ever hung on a player. Whichever landed first would have taken the other's place — **a sword
and no camera: black screen, exit 0, no warning.**

---

# §3 — the user, 2026-08-19, after playing with the assist

> *"ok von snapping. die seile sollen immer auf der horzontalen fest sein. also wenn das
> fadenkreuz 0, 0 ist sollen die seile nur auf der x achse snappen (objekte finden) also seitlich!
> dann ist es auch besser einzuschätzen. zudem sind die gebäude nicht auf dem boden sondern in der
> luft! die map passt aber immernoch nicht."*

## §3A — 🟢 the snap searches SIDEWAYS ONLY

**The spec, and it is a constraint that makes the feature legible:** the candidate search is
**locked to the horizontal**. At a crosshair of `(0, 0)` the two ropes may only find anchors along
the **x axis** — laterally, left and right. The snap must never move a rope **up or down** relative
to where the player is looking.

**His reason is the important part: *"dann ist es auch besser einzuschätzen."*** A snap that can
move in two axes is unpredictable; one that moves in a single, named axis can be learned. That is
worth more than a marginally better anchor.

**What that changes.** Today `vector::aim` casts a **probe fan** — `assist_probe_rings ×
assist_probes_per_ring` (2×4) rays per hemisphere, a 2D cone around the look direction
(`FIND-104`). It has to collapse to a **1D horizontal sweep** in the camera's own right axis — and
note `side_dirs` already yaws around the **camera's** up axis on purpose, so "horizontal" here means
**screen-horizontal at every pitch**, not world-horizontal. Looking 60° down, "sideways" is still
left and right of the crosshair.

⚠️ **`F-025`'s scoring survives** — angle deviation, momentum, height advantage — but the candidate
SET is now a line, not a cone, so the height-advantage term will rarely differ between candidates.
Say what that does to the weights.
⚠️ **`assist_catch_pct` keeps its meaning** (0–100 % → 0–20° off the crosshair) but now describes a
half-width, not a radius.
⚠️ This also bears on **`B-008`** (a downward shot resolving onto a roof aside) and on **`Q-039`**,
which asked whether the fan should collapse as pitch steepens. **His instruction answers a related
question and may answer that one too — check before assuming it does.**

## §3B — ✅ the buildings float, cause found

**Measured by the main head, do not re-derive:** every model in the pack has its origin at its
**feet** (`hit.min.y = 0.000`; `a-083-fachwerkhaus-gross` spans y 0.000..11.500), while
`BlockPlan::spawn` positions by the block's **centre** (`src/world/map.rs:283`,
`Transform::from_translation(self.center_m)`, with `ModelName` on the same entity at `:300`).
**So every dressed building floats by half its own height** — 5.75 m for a large house. A round is
fixing it.

## §3C — ⬜ *"die map passt aber immernoch nicht"* — NOT YET DIAGNOSED

He has said it twice now (§1F was the first) and has not said what. §3B is very likely part of it —
a district of buildings hovering half their height over a terraced ground reads as broken however
good the models are — but **that must not be assumed to be the whole answer.**

**Do not redesign the map on a guess.** The cheap, honest next step is the one that has worked all
session: **take a street-level and an aerial frame, read them, and list what is wrong** — then ask
him which of those he means. The §3B round has been told to do exactly that.

### §3C answered — *"die map passt aber immernoch nicht"*, measured

`FIND-134`, 2026-08-19. Three causes were proposed, all three were measured, and **two of them were
not what they looked like.**

**1 · Street level now reads as a town; from the air it does not.** That split is the finding. The
roofscape and the dressed houses are good; **what is bare is everything the model vocabulary cannot
dress**, and that is the large hand-placed geometry: the wall, the gates, the HQ, the gantries.

**2 · 🔴 THE WALL CANNOT BE DRESSED BY THIS MECHANISM AT ALL.** The pack's wall vocabulary
(`a-095`, `a-096`, `a-101`) is a **tile set authored at one module — 11.20 m wide, 120 m tall.**
Ashgate's wall is **monolithic 700 / 336 / 285 m bands**, and `fit_to_class` scales **uniformly**:
it can fit a tile to a box, it **cannot repeat one along it** (700 / 11.2 = 62.5 — the runs do not
even divide). **Dressing the wall means re-cutting it in `maps.ron`** — every collider in the
silhouette, the 40 asserts of `f003-ashgate.txt`, and the whole `hook.gesims_*` anchor ladder.
**That is a level-design round and it is the biggest single remaining visual item.**

⚠️ **And the proof that a shortcut would have been a disaster:** matching all **80** placed size
classes against all **279** files, the bell tower's best fit in the drop is a **gas canister (4 %)**
and the gatehouse's is a **severed arm (8 %)**. A fit-only dressing table would have filled the
district with nonsense that no test could have caught.

**3 · The ground is flat because it IS flat.** `terrain.step_m` **1.50 m** per `cell_m` **42 m** is
a **3.6 % grade** under 11.50 m houses. A retaining-wall/cap split was built to make the terraces
read, measured at **5 of 921 600 pixels** for **+255 blocks (+8.9 %)**, and reverted. The relief is
not hidden by the renderer — it is below the threshold of visibility. **Both numbers are the
user's** and both are constrained by `plan_terrain`'s stair asserts, so changing them is his call:
a steeper step reads from the air and costs walkability, which is exactly the trade
`FIND-091` measured once already (*a 0.36 m tread is a wall with a texture*).

**So the honest queue for the map, in order of what it would buy:**
1. **Re-cut the wall into modules** so the tile set can dress it — biggest aerial win, and it
   unlocks `hook.gesims_*` anchors along the wall as a side effect.
2. **A `model:` field on `maps.ron: blocks`** so a placed block can NAME its dress instead of being
   matched by size and palette. 12 of 215 are dressed today because only stalls and barrels had an
   honest match; the HQ, the gates and the towers need to be told, not guessed.
3. **Ask him about `step_m`** — 1.5 m over 42 m is his number and it is why the slope is invisible.

## §3D — 🔴 THE ROPE, 2026-08-26, verbatim. Four requirements, and one of them is a bug.

> *"ok wichtig: wenn ich von seil weg gehe. also seil ist vorne und ich laufe zurück werde cih
> nicht ran gezogen. sonst werde ich ranzeogen! wenn ich a oder d drücke zur seite soll ich auch
> noch rangezogen werden! AUCH. aber weniger! aktuell kann ich a drücekn und w und ich gehe in
> einer geraden linie weg von dem ankerpunkt (nicht gut). wenn das seil shcon eingezogen wurde soll
> es erstmal nicht länger werden!"*

**This continues 2026-08-24's *„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"*** —
the always-pull landed, and he is now saying the steering still has a hole in it.

**R1 · `S` must not cancel the pull.** Rope ahead, walking backwards → *"werde ich nicht ran
gezogen"*. The pull is unconditional; `S` is the one input that is allowed to fight it, and even
then it may only **slow** the approach, not reverse it into a retreat.

**R2 · `A`/`D` keep pulling. „AUCH. aber weniger!"** Lateral is a *component added to* the pull,
never a *replacement for* it. `drive_steer_pull_fraction` (0.35 today) is exactly this knob — the
question is whether it is applied at all on the lateral path, or only on the forward one.

**R3 · 🔴 THE BUG, and it is stated as a repro:** *"aktuell kann ich a drücken und w und ich gehe
in einer geraden linie weg von dem ankerpunkt"*. `A`+`W` together produce a straight line **away
from the anchor**. Two inputs that each individually pull, in combination push. That is a sign
error or a normalise-after-sum, not a tuning value.
⚠️ **Repro it before touching anything** — hook something, hold `A`+`W`, log rope length per tick.
Rising monotonically is the failure.

**R4 · The rope is a RATCHET.** *"wenn das seil shcon eingezogen wurde soll es erstmal nicht länger
werden"* — once reeled in, the rest length must not grow back on its own. `Ctrl` shortens it;
nothing silently lengthens it. („erstmal" = until released/re-fired, not forever.)
⚠️ This is a **`DistanceJoint` rest-length** change, and it interacts with `rope_winch`. It is also
the one of the four that is a **new mechanic** rather than a fix.

**Acceptance is one number, and it is the same for R1–R3:** with an anchor ahead and ANY of
`W`, `A`, `D`, `A+W`, `D+W`, `S`, or no key at all, **the anchor distance must not increase** while
the rope is taut. `S` may hold it flat; nothing may make it rise.

## §3E — 🔴 YOU SPAWN IN THE HUB WITH YOUR BACK TO THE DOOR, and the file says otherwise

**Measured 2026-08-27**, from the untouched cold-start frame (`--hub --offscreen`, no `look`, no
key). This is the root of the user's *„von der lobby aus muss man auch neue missionen starten
können!"* — not a missing feature, a missing **direction**.

`assets/data/missions.ron:57` states the design intent:

> *"`recruit` stands **straight ahead** of the spawn point — it is the door you find without
> looking for it; the two harder ones stand off to [either side]"*

**It is not ahead. It is behind.** Bearings from the spawn point at the default facing:

| pad | position | bearing from the spawn look |
|---|---|---|
| `skirmish` / **recruit** | `(0, 0, 16)` | **180.0°** — dead behind |
| `skirmish` / veteran | `(-9, 0, 16)` | 150.7° |
| `skirmish` / elite | `(9, 0, 16)` | 209.3° |
| `parcours` / recruit | `(-10, 0, -8)` | 51.3° — the only one on screen, and only its **corner** |

The camera faces **−Z** by default; every skirmish pad sits at **+Z**. `parcours` is 51.3° off-axis
against a half-FOV of 45.7° (`game.ron: camera.fov_deg` 60 vertical, 16:9), so **6120 px of one
corner clipping the bottom-left edge is the entire visible evidence that this place has doors** —
0.66 % of the frame.

**So the file's own sentence has never been true**, and nothing has ever checked it: no test
compares a pad's bearing against the spawn facing, and every script sets its own `look` before
walking (`f175-loop.txt` uses `look 180 0`, which is exactly the 180° turn a player does not know
to make).

### The fix is one of two lines, and it is the main head's

1. **Turn the spawn to face `+Z`** — one facing value, and `recruit` becomes what the file says it
   is. ⚠️ Check what reads the default facing first: the scripts all set `look` explicitly, so the
   blast radius should be small, but `f070-hub.txt` and `f177-door.txt` must be re-run.
2. **Or move the skirmish pads to `−Z`** and keep the facing. More disruptive: `f175-loop`,
   `f070-hub`, `f185-parcours` and `f072-breach` all walk to fixed coordinates.

**(1) is the cheap one and it matches the sentence already in the file.** ⚠️ Do it **after**
`F-177`'s round closes — it moves the geometry that round is measuring against, and a data change
under a live measurement has already cost this project two matrices.

⚠️ **And it does not replace `F-177`.** A visible door still needs to say what it is; turning the
player around only means he can see the paint. Both, in this order.

**Related:** `FIND-187` · `FIND-178` · `FIND-173` · `docs/QUESTIONS.md` Q-058 · `F-177` `F-175`

## §3F — 🔴 THE FRAME OF REFERENCE IS THE ANCHOR. The user, 2026-08-27, verbatim:

> *"ok folgendes: wenn man a oder d drückt (relativ zum anker (DAS IST WICHTIG), immer alles
> relativ zum anker gesehen) dann soll man zur seite gehen können. aber NICHT das seil
> verlängern!!"*

**This is the specification §3D was missing, and it answers `Q-058`.** Two statements, and the
first one is a *coordinate system*, not a feature:

**1 · EVERYTHING IS MEASURED FROM THE ANCHOR.** *„immer alles relativ zum anker gesehen"*, and he
capitalised the parenthesis himself. `A`/`D` are **tangential** — a movement on the sphere around
the anchor. They are **not** camera-right. That is precisely the defect `FIND-183` measured:
`sideways = right * (lateral_m_s * move_x)` with `right = Vec3::new(cos, 0.0, -sin)`, the camera's
yaw vector, **never made perpendicular to the rope**, so `A`/`D` could point straight away from the
anchor.

**2 · THE ROPE MAY NOT LENGTHEN.** *„aber NICHT das seil verlängern!!"*, two exclamation marks.
Not "less", not "slower" — **not at all**. Combined with §3D R4 (*„wenn das seil shcon eingezogen
wurde soll es erstmal nicht länger werden"*) this is a **hard maximum length**, and it is exactly
what an avian `DistanceJoint` with `limits = (0, L)` does: it corrects **only** when the distance
exceeds `L`, and the solver holds **all** of them at once.

### What this settles

**`Q-058` is ANSWERED — `Drive` gets a real rope.** The question was whether a hard maximum length
is acceptable given `FIND-149` (*the reference drives and does not swing*). *„NICHT das seil
verlängern"* **is** a hard maximum length. He has chosen the constraint, and being turned at `L`
is the price he has accepted by asking for it.
⚠️ **The rollback point stands** (`player::rope::attach_ropes`, the one branch that gives a `Drive`
rope its joint) — if the arc turns out to feel wrong when he plays it, that is where it comes out.

### And it kills both failed attempts at the root

Both were refuted for the *same* reason: they tried to prove a geometric invariant **by hand, in a
velocity target**, and could not do it for two arms. Attempt 1 bounded `unit(Σ r̂ᵢ)` — the **sum**
of the distances instead of each. Attempt 2 tried to satisfy both per-arm floors by scaling, and
**scaled to zero** on 25.8 % of ticks because the origin is in every constraint set (`FIND-186`).
**Against a joint, neither failure is expressible**: the solver enforces per-arm maximum length
simultaneously and by construction, and `shorten_ropes` — already the *single* writer of
`limits.max` — already contains the ratchet (`B-004`'s take-up: *"never upward"*).

**So the drive's whole job becomes a direction and a speed**, and it never has to prove anything
about distance. `A`/`D` become tangential *because the joint makes the radial component
impossible*, not because a projection removed it.

**Related:** `Q-058` · `FIND-186` · `FIND-183` · `FIND-152` · `FIND-149` · `B-004` · `B-005` ·
`docs/NEXT.md` §3D · `F-004` `F-005` `F-006`

## §3G — 🔴 THE SCRIPT CORPUS IS AIMED AT `gravity_m_s2: -20` AND THE FILE NOW SAYS `-32`

**Measured 2026-08-27 by the main head**, and independently by an adversary against **one pinned
binary** with only `game.ron` differing — so this is the gravity change and nothing else:

| script | before `1ca7d26` | now |
|---|---|---|
| `scripts/f175-loop.txt` | 19 asserts held, **15 of 15 runs** | **10 of 19 failed** |
| `scripts/f070-hub.txt` | 42 asserts, exit 0 | **16 of 42 failed** |
| `scripts/f177-door.txt` | 13 asserts, exit 0 | **5 of 13 failed** |

**This was predicted in the commit's own comment and it is not a regression** — the world got
heavier because the user asked for it twice, and a 30.8 m drop now takes **1.39 s instead of
1.76 s**. Everything timed against a fall is aimed at the old constant.

### The re-aim waits for the rope round, on purpose

`F-006`'s attempt 3 (the `DistanceJoint` under `Drive`, `Q-058`) is **live right now** and will move
flight geometry again. Re-aiming the corpus against gravity today and against the joint tomorrow is
the same work twice, and the second pass would be done against numbers the first pass invented.
**One re-aim, after the joint lands.**

### The rules for that re-aim, and they are not negotiable

1. **Re-derive, do not widen.** A bracket comes out of the new constant (`v = sqrt(2gh)`,
   `t = sqrt(2h/g)`), not out of whatever the run happened to print. **Loosening an assert until it
   passes is how a corpus stops being evidence.**
2. **Separate the two causes.** Both gravity and the joint landed in the same window. Every
   re-aimed line must say which of the two moved it — a report that conflates them is useless.
3. **A line that cannot be re-derived is marked red on purpose**, with a header saying why, the way
   `scripts/f176-pull.txt` already is.
4. **Pin the binary and the data** before any before/after (`cp target/debug/defeated_by_titan
   $SCRATCH/dbt-pinned`), and pin `assets/` too — this entry exists because that was not done.

### And the process failure, recorded because it is mine

The gravity change was made **while a round was live**, in the exact file (`assets/data/game.ron`)
that the same round's commission forbade its agents from touching, an hour after that commission
was written. It corrupted three of that round's evidence scripts and briefly looked like the
unexplained flake in `B-012`. **`CLAUDE.md` already carries this rule twice** — a measurement round
pins its own binary, and a data change under a live measurement has destroyed two matrices in this
project. It has now destroyed a third, and the supervisor wrote the warning himself.

**Related:** `docs/BUGS.md` B-012 · `Q-058` · `docs/NEXT.md` §3F · `F-006` `F-175` `F-177`

## §4A — 🔴 THE MAP IS FLAT AND YOU CAN FALL OFF THE SIDE. The user, 2026-08-27:

> *"es soll auch noch verschiedene höhen geben. aktuell sit alles flach von der map. und man kann
> an der seite einfach runterfallen!"*

**Two things, and the first one is already measured and was explicitly left to him.**

**1 · The relief is below the threshold of visibility, and it always was.** `FIND-134` point 3
measured it: `terrain.step_m` **1.50 m** over `cell_m` **42 m** is a **3.6 % grade** under 11.50 m
houses. A retaining-wall/cap split was built to make the terraces read, measured at **5 of 921 600
pixels** for **+255 blocks (+8.9 %)**, and reverted — *"the relief is not hidden by the renderer,
it is below the threshold of visibility."* The finding closed with *"Ask him about `step_m` — 1.5 m
over 42 m is his number and it is why the slope is invisible."*
**He has now answered without being asked: make it vary.**
⚠️ **The trade is known and was measured once already** (`FIND-091`): a steeper step reads from the
air and costs walkability — *"a 0.36 m tread is a wall with a texture"*. So this is not one number,
it is a number **plus** the stair asserts in `plan_terrain` that constrain it.
⚠️ **And "verschiedene höhen" may not mean terrain at all.** A flat district with tall buildings
and a flat district on a hillside are different fixes; so is *vertical* variety — roofs at
different heights to swing between. **Ask which he means before touching `step_m`** — this is
exactly the §3C mistake (redesigning the map on a guess) and it cost a round once.

**2 · 🔴 THE MAP HAS NO EDGE. You can walk or fly off the side.** This is a plain defect and it is
nobody's design decision. Ashgate is 700×700 m with a wall, and outside the wall there is
apparently nothing to stop a player leaving. **No repro is on file yet** — write one before fixing
(rule 5), because the honest answer differs by cause: no collider beyond the plate, a plate smaller
than the playable area, or a kill-plane that does not exist.
**Candidate answers, cheapest first:** an invisible boundary at the wall; a death/respawn plane far
below; the world simply continuing (generated ground) so there is no edge to find.

**Related:** `FIND-134` · `FIND-091` · `Q-070` (the town stays intact) · `docs/NEXT.md` §3C ·
`F-003` `M-002`

## §5A — 🔴 HE PLAYED IT, 2026-08-29. Seven things, verbatim, and two of them contradict a 🟧.

> *"ok problem: ist ist immernohc nicht am cursor. es bewegt sich immernohc also die target seile.
> das passt nicht. zudem steht no target selbst wenn ich ein seil dran hab.. das ist gar nicht gut.
> die hitboxen passen auch nicht. ich komme nicht an ein titan ran zum hitten! das passt auch
> nicht! zu not erstelle nochmal einen eigenen titan mit passender größe weil die aktuellen sind
> eher kleiner! gerne doppelt so groß oder so. und leichter hittable am nacken.. aber aktuell nicht
> gut. auch die verschiedenen höhen passen nicht! das soll grass sein und nicht so wie jetzt! und
> nicht verschiedene hardcoded stufen sondern wirklich terrain! und deutlich höher und niedriger
> als jetzt! und aktuell sind immernoch die großen türme /gates beim eingang. das passt GAR nicht
> zu attack on titan und existiert so nicht! adde andere türme beim wasser (das wasser ist auch
> VIEL zu klein)"*

**A round building stepped terraces was STOPPED mid-flight and reverted for point 5** — 373 lines
of `src/world/map.rs` and 64 of `maps.ron`, saved at
`$SCRATCH/relief-and-edges.patch`. It was building the thing he just rejected.

### 1 · 🔴 THE MARKER IS STILL NOT AT THE CURSOR, AND IT STILL MOVES — against a 🟧 claim

*„ist immernoch nicht am cursor. es bewegt sich immernoch"*. `F-026`/`FIND-129` claim the drawn
pixel **is** the projection of the point the rope flies to, measured at **0.0 px on fire and
anchor**, and an adversary re-derived it independently (`(640.00, 357.77)` computed against
`(639.5, 375.5)` measured, the 17.7 px in `y` being `SIGHT_CORE_PX`'s documented stand-down).
**His eye says otherwise and his eye wins** — `CLAUDE.md`: *a symptom he reports is real even when
the code looks right; every single one so far has been.*
**So the stage is wrong, not him.** `F-026` goes back to 🟨 until this is understood.
⚠️ **The likely gap: what was measured is not what he is looking at.** The 🟧 evidence is a
*projection at one tick*, decoded from a screenshot. He is describing **motion** — a marker that
lags, jitters or slides while he turns. Nothing in the evidence measures the marker across
consecutive frames, and `place_arm_aim` runs in `Update` against a `Transform` that `FixedUpdate`
writes (the same schedule split that produced `FIND-190`'s stale hub prompt). **Measure it in
motion, tick by tick, before changing anything.**

### 2 · 🔴 „NO TARGET" WHILE A ROPE IS ATTACHED

*„zudem steht no target selbst wenn ich ein seil dran hab"*. An anchored arm is `ArmAimState`'s
strongest state; reading *no target* there means the state machine and the rope disagree about
whether an arm is anchored. **This is a two-writers-on-one-question shape** (`FIND-190` again) and
it should be found by asking who writes `ArmAimState` and who writes `HookState`.

### 3 · 🔴 HE CANNOT REACH A TITAN TO HIT IT — and the titans are too small

*„die hitboxen passen auch nicht. ich komme nicht an ein titan ran zum hitten!"* and
*„die aktuellen sind eher kleiner! gerne doppelt so groß oder so. und leichter hittable am nacken"*.
⚠️ **This is the third time hitboxes have come back** (2026-08-24, 2026-08-26). The pass width was
widened 0.20 → 0.80 m once already and the lurker was *literally unkillable* before that.
**He is asking for two separate things and they must not be conflated:**
- **titan SIZE** — roughly double. `assets/data/titan.ron` and `scale.ron`; ⚠️ `max_spawnable_class`
  gates the bellower and the whole scale ladder is `Q-002`'s.
- **nape REACH** — easier to hit. That is `cortex_radius_m` and the pass geometry, and
  `FIND-206` already measured the current window as **one simulation step wide at 21 m/s**.
  **One tick is not a hitbox, it is a coincidence.**

### 4 · 🔴 REAL TERRAIN, NOT STEPS — and it is GRASS

*„das soll grass sein und nicht so wie jetzt! und nicht verschiedene hardcoded stufen sondern
wirklich terrain! und deutlich höher und niedriger als jetzt!"*
**Three separate demands:** a continuous height field (not `levels × step_m` terraces), a **grass**
surface, and a **much larger vertical range** than today's 3.6 % grade.
⚠️ `plan_terrain`'s stair asserts, `stair_rise_m` and the `step_m`-must-be-a-multiple rule all
exist to make *terraces* walkable. **A continuous field does not need them and they should not be
carried over** — but `FIND-091`'s real finding survives in a new form: *whatever the slope, the
player has to be able to walk up it.*

### 5 · 🔴 THE GATES AND TOWERS AT THE ENTRANCE ARE WRONG FOR THE GENRE

*„die großen türme /gates beim eingang. das passt GAR nicht zu attack on titan und existiert so
nicht!"* They are hand-placed in `maps.ron` and are part of the 203 untextured blocks that make up
the district's whole silhouette. **Removing them changes `f003-ashgate.txt`'s geometry and the
`hook.gesims_*` ladders.**

### 6 · 🔴 THE WATER IS MUCH TOO SMALL, AND THE TOWERS BELONG THERE INSTEAD

*„adde andere türme beim wasser (das wasser ist auch VIEL zu klein)"*. So the water becomes a real
feature of the map and the towers move to it.

**Related:** `FIND-129` · `FIND-190` · `FIND-206` · `FIND-134` · `FIND-091` · `Q-002` ·
`docs/NEXT.md` §4A · `F-026` `F-030` `F-003`
