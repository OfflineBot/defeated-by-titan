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
which is the honest guard. `gas_tank: 300.0` stays.
**What remains is the feature itself: refuel stations as world objects**, plus a mission loop where
going back to the main building is a decision. `Gas::refill` exists, is called by nobody, and is
reserved for them.
⚠️ **Until they exist, 300 gas is the entire supply of a run.** Whether that is playable is a feel
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
here: `Speed > 25 → 0.000`, `Height > 12 → 0.050`, `Gas < 300 → 300.000`. No anchor → no reel → the
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
