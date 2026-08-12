# NEXT — what to do first, in order, when the session restarts

Updated: 2026-08-10 · Stage: 🟨 (a queue, not a result — nothing here has been done)

Written at the end of the first session that had a **window** and a **human who played the game**. This file is the queue; [`HANDOVER.md`](HANDOVER.md) is the state. Read the handover
first, then do the session ritual from [`CLAUDE.md`](../CLAUDE.md), then start at 1 below.

**Branch: `session-2026-08-09`.** `main` is still diverged — see `HANDOVER.md` §7.

> ⚠️ **The literal first item is at the BOTTOM of this file, under `## 0.`** — `Gas` has two
> writers since the hub landed, and that is the violation rule 4 exists to prevent. It is ~30 lines
> and it should be done before anything else builds on the hub.
> *(It sits at the bottom because appending with `>>` costs nothing to read and opening this file to
> insert at the top costs ~4 000 tokens. Order by the number, not by the position.)*

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
