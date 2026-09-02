# PLAN — the complete state of the project, and what to do about it

Updated: 2026-08-27 · Stage: 🟨 (a plan, not a result)

> **Why this file exists.** The user, 2026-08-27: *„ok aktuell glaube ich drehen wir uns im
> kreis! analysiere das komplette projekt und plane es durch! KOMPLETT!"* He was right. Seven
> rounds produced four refutations on one HUD line and three on the rope, while 224 of 245
> features stood unbuilt. Six parallel readers surveyed the whole repository; this is what they
> found.


---

## 0 · ⭐ THE ANSWERS — the run-through of 2026-08-27, and what it changed

**All 23 questions were put to him and answered in one sitting** (`docs/QUESTIONS.md`, batches
1–6). **Three answers deleted work rather than creating it, and one of them was the thing this
project had been failing at for four rounds.**

**The three that made the project SMALLER:**

1. **The anchor field is gone.** *„es soll auf jeglicher oberflqche einhaken. nicht an hardcoded
   punkten etc!"* — `world::AnchorField` (787 lines, 1564 authored + 8108 generated points,
   rebuilt every load) was **the wrong idea, not an unfinished one**. `F-024` is not built,
   `F-026`/`F-027` lose their subject, and the raycast the game already uses is correct.
   ⚠️ The **aim assist survives** — it sweeps the *ray* sideways for a surface and never used the
   point list.
2. **The hub prompt is retired, replaced by a board.** *„wenn man in der hub auf ein board drückt
   (F) dann kommt man in eine mission übersciht"*. **Four refutation rounds were spent attacking a
   predictive text line he never asked for.** A board you walk to and press `F` on needs no bearing
   rule, no walk model, no ray — and the blank signpost is already standing in the hub.
3. **Ashgate stays intact.** So `docs/gameplay/world.md` is the file that is wrong, not the map.

**The one that made it bigger, knowingly:** progression gets **built out fully**, which overrides
`docs/PLAN-GAME.md` §10. He is the gate and he chose.

**And the gate itself is now real instead of impossible:** two gates, **him first**, then a softer
one — and *„ich sag wenn es passt"*, so **no agent may ever declare it passed**.

⚠️ **The build order in §4 below was written BEFORE these answers.** Ranks 6 and 7 are void (the
hub-spawn turn and the anchor snap), rank 11 moves behind rank 8, and rank 14 is unlocked. Read §0
first and §4 as history.

---

## 1 · What this game IS today

Today, if you launch it yourself with no script running, this is what you get. A title screen, Play, and you land in a hub yard with your back turned to every one of the six doors that start a mission — the nearest pad is 150 degrees behind you and 0.66 % of the screen. If you know to turn around, you walk to a pad and deploy into Ashgate: a real 700x700 m walled town, 2901 blocks, 831 placed models, half-timbered houses, rubble, market stalls, a tuned sun and haze out to 470 m, running at 237 fps. You fire two hooks with Q and E and they bite essentially instantly at up to 500 m. You swing and drive on the ropes, steer with A and D, dodge, flip, refuel and restock at four supply stations. That part is genuinely good and it is the most finished thing in the repository. Then it stops being a game. Shift — the thing that makes it a flying game — stopped lifting you this morning: gravity was changed from -20 to -32 and the boost number was not, so Shift now nets +2 m/s2 against your own weight instead of +14, and a boost that used to carry you 28 m carries you 4.17 m. Holding Ctrl with both hooks planted on anchors far apart asks for two rope lengths no point in space can satisfy, and on about two thirds of geometries the physics gives up and pins you at exactly 0.000 m/s. The titans are there, seven kinds, they walk, they telegraph, they swing, and they can kill you — that is proven. You killing them is proven in exactly one place: a 284-line script. Every other cut in the repository lands on a leg today. The whole thing happens in total silence: there is no audio code and no audio files anywhere. If you do win, the debrief tells you the mission name, a verdict word, a kill count and a clock, and puts you back in the same hub in front of the same six pads. The save file says 321 sorties have been flown and only the Recruit tier of anything has ever been beaten. Nothing between your first sortie and your three hundred and twenty-first is different.



## 2 · The gap

The gap is much smaller than the ledger makes it look, and it is not 224 features. The loop already closes end to end without a human in it — two scripts walked hub to pad to Active to Won to Debrief to Hub today with 25 asserts between them. What stands between that and a person doing it unassisted is five concrete things and one play session. (1) Shift has to lift you again: one number in game.ron, and the arithmetic behind it. (2) Ctrl must stop pinning you at zero when both hooks are planted: one geometric rule in the single writer of rope length. (3) The hub spawn has to face a door: one facing value. (4) A titan has to be killable by hand, not only by a script that computes the fall time — which today means retiming twelve scripts against the new gravity so the project can tell a real regression from a stale calibration, and then photographing at least one unscripted nape cut. (5) Sound — any sound at all — because a movement game whose entire feedback channel is gas, hook and blade currently gives you none of them. That is roughly two to three focused rounds of work, most of it arithmetic. After that a person can play it start to finish. What is then still missing is a REASON to fly the second sortie — XP goes to a file you cannot see, a gear budget nothing can spend, no locked door, one map for all five missions, no mode that asks the movement to prove itself. That second gap is real but it is behind the design's own gate, and it should stay there.



## 3 · Why we circled — six mechanisms, all measured

Six mechanisms, all measurable, none of them a lack of effort. FIRST AND LARGEST: the queue is 18 days behind the tree, in BOTH directions. docs/STATUS.md was generated 2026-08-12, docs/features.ron last written 2026-08-20, gameplay/features.xlsx untouched since 2026-08-09 — and tools/features.py has not been run in fifteen days. 54 rows marked Unbuilt carry a named test function; 20 carry an evidence script. F-003 still records "79 blocks" against a real 2901. Every round therefore starts from a picture of a project that does not exist, and FIND-039's failure (re-deriving a feature the backlog already specified, worse) is not an anecdote, it is the steady state. SECOND, AND THIS IS THE SHARP ONE: two of the three most recent work streams have no F-ID at all. features.ron:167 says F-176 is "Barrierefreiheit"; scripts/f176-pull.txt is a rope test. features.ron:168 says F-177 is "Grafikeinstellungen"; scripts/f177-door.txt and src/hud/hub_prompt.rs are a hub prompt line. Maps are worse — M-001..M-012 in docs/backlog/maps.ron have no F-ID whatsoever, so 30 person-days of Must work on Ashgate has never appeared in TODO.md or STATUS.md. Work with no row on the ledger has no acceptance criterion and therefore no stopping condition; the only way such a round ends is when someone gets tired of attacking it. That is the mechanical explanation for four refutation rounds on one HUD text line. THIRD: the refutation discipline was pointed at the wrong targets. CLAUDE.md item 6 already says "attack claims; do not attack chores" — a HUD line and a rope tuning value are chores with a red test, and they absorbed seven rounds, while the genuinely unattacked claims (that the cut scripts prove a titan dies, that the anchor field does anything, that the corpus can detect a regression) went unexamined until today. FOURTH: the corpus re-aim was deferred behind the rope round on purpose (NEXT.md §3G) and that dependency runs the wrong way. The amber corpus is what makes every round expensive to triage; deferring it behind a round that then ran four attempts and is STILL uncommitted meant every one of those attempts paid the triage tax. ~40 red asserts across ~12 scripts now have three different causes (gravity, the new joint, real defects) with nothing separating them. FIFTH: the most important open defect in the movement system — Ctrl with two ropes pins the player at 0.000 m/s on 64.4 % of geometries — exists only inside docs/FINDINGS.md, the 108 kB file this project's own rules forbid opening. grep for FIND-191 in QUESTIONS.md, BUGS.md and NEXT.md returns nothing. A finding that is not promoted into a queue file has not been found. SIXTH: process. gravity_m_s2 was changed -20 to -32 while a round was live, without re-deriving boost_m_s2 or air_accel_m_s2 (whose own comment in game.ron is now factually false) and without re-aiming the twelve scripts that encode a fall time. One data line turned the entire evidence base amber in a single commit. And 13 files, +3044/-316, spanning two unrelated features, are sitting uncommitted — so the next round cannot build on them and instead re-enters them, which is exactly what happened three times.



---

## 4 · The build order


### 1. Land the tree. Two separate commits, not one: (a) the DistanceJoint rope (src/player/rope.rs, locomotion.rs, integrator.rs, tests/vector_rope.rs, tests/player.rs, scripts/f176-pull.txt) with FIND-191/192/194/195 and Q-058/060/061; (b) the hub prompt line (src/hud/hub_prompt.rs, src/hud/mod.rs, src/hud/objective.rs, src/menu/pause.rs, tests/menu.rs, tests/hud.rs). Do NOT extend either. Stage them honestly at built-but-unseen except where a picture already exists. Then push.

**F-IDs:** `F-004, F-005, F-006 (rope); the hub line has NO F-ID — it borrowed F-177, which is Grafikeinstellungen`  ·  **Blocked by:** nothing

Nothing in git says either feature exists, and an unlanded round is a round the next session re-enters — which is measurably what happened three times on the rope and four times on the hub line. Both halves build: the 15:00 binary already contains them and their tests run green (vector_rope 28/0, player 58/0, lib 280/0, hud control pair measured). Landing costs nothing and removes the single largest cause of repeated work. It also unblocks every git operation for the rest of the plan.


### 2. Freeze the gravity number and re-derive its dependents in one pass: boost_m_s2, air_accel_m_s2, air_lateral_m_s2, and every comment in assets/data/game.ron that states a relationship to -gravity_m_s2 (lines 99-146 are now factually wrong). One RON edit, one arithmetic note per changed value, plus a data test that pins the ratios so the next gravity change cannot silently break them again.

**F-IDs:** `F-005, F-006, F-007`  ·  **Blocked by:** Q-063 (is -32 final), Q-064 (what Shift should be)

Shift is the input that makes this a flying game and it stopped working this morning: boost 34 against gravity 32 nets +2 m/s2 where it netted +14. Measured in the running game, scripts/f-007-boost.txt reads Height 4.170 against an expected 28.5 and Speed 3.679 against 27.5. The user's own acceptance sentence for the whole movement system — stay in the air until the gas runs out — is arithmetically unreachable today. This is the one item a player would notice in the first ten seconds of flight.


### 3. Re-aim the ENTIRE script corpus against the settled gravity, in one round, by file. ~12 scripts: f-001-hooks, f-007-boost, f003-ashgate, f004-towers, f005-feel, f006-drive, f025-chain, f029-grapple, f030-cortex, f030-hitbox (seven per-kind recomputes), f031-damage, f032-swords, f034-hitstop, q030-reach, game-full, f070-hub, f175-loop, f177-door, f073-escort. Every changed number gets its arithmetic written into the script header (fall_time = sqrt(2*(warp_y - target_y)/g)). Two scripts get triaged separately, not retimed: f025-chain lines 101/197 (a hook that does not anchor, a rope that does not release) and f029-grapple (Rope==0 reads 1, Titans==1 reads 2) are real defects, not calibration. w5-lane.txt gets route 2 from NEXT.md §2D (aim with settings assist_strength 100, not by hand-compensated angle) — it has been red since 2026-08-19 for a reason unrelated to towers.

**F-IDs:** `F-003, F-007, F-025, F-029, F-030, F-031, F-032, F-034, F-070, F-072, F-073, F-175`  ·  **Blocked by:** rank 2 (the numbers must be settled first, or the re-aim is paid twice)

This is the project's only evidence base and it is currently amber everywhere at once for three different reasons, with nothing separating them. Until it is re-aimed every future round pays a triage tax on ~40 red asserts and cannot distinguish a regression from a stale number. Reader 2 proved the cost is arithmetic, not code: changing ONE number in f030-cortex (wait 0.90 -> 0.71) turned 'cut LegLeft at 33.07 m/s, 1 of 2 failed' into 'cut Cortex at 26.67 m/s, exit 0'. Deferring this behind the rope round was the wrong direction of dependency and it compounded across four attempts.


### 4. Rebuild the ledger and give the unnamed work rows. Audit all 245 features.ron rows against the tree (54 marked Unbuilt carry a named test; 20 carry an evidence script), correct in BOTH directions, add M-001..M-012 as rows so maps enter the queue, add a row for the hub (M-001 The Rookery is what f070/f177 actually build), correct F-003's evidence string, then run tools/features.py and regenerate STATUS.md and TODO.md. In the same round, sweep the dead code the survey found: RopeForceModel::Pendulum + locomotion::rope_steer (unreachable since the joint landed), SpatialIndex::cast_ray/aabb_overlaps stub bodies, src/vector/reel.rs's pass-through, and fix src/net/mod.rs's '37 bytes' against wire.rs's 33.

**F-IDs:** `all 245 rows; M-001..M-012 have no F-ID today`  ·  **Blocked by:** Q-071 for the map rows (it is his spreadsheet); nothing for the rest

This is the instrument the supervisor steers by, and it is 18 days stale in both directions. It is the root cause named by four of six readers independently. It is an hour or two of mechanical work with no gameplay in it, and without it every plan written from here starts from a false map — which is the documented mechanism of FIND-039. Runs in parallel with ranks 2-3: different files, mechanical criterion.


### 5. Clamp the two-rope reel so it cannot ask for a geometrically impossible pair. player::rope::shorten_ropes is already the single writer of limits.max; add the rule that L_left + L_right may never fall below the anchor separation. Then file FIND-191 into docs/BUGS.md with its repro so it stops living only in the 108 kB file.

**F-IDs:** `F-005, F-012`  ·  **Blocked by:** Q-065 (clamp, auto-release, or accept)

Holding Ctrl with both hooks planted drags the player onto one anchor and pins him at exactly 0.000 m/s with the other rope 50.167 m past its own maximum — infeasible on 16704 of 25920 ticks (64.4 %) on the ground and 64.5 % in the air. This is a movement-killer in a game that is entirely movement, it is reachable with one key, and it is tracked in no queue file at all. Two hooks is this game's premise, not an edge case.


### 6. Turn the hub spawn to face a door, and settle the hub line's stopping condition. One facing value (or move the three skirmish pads); assets/data/missions.ron:57 already asserts the outcome. Add the test nobody has written: compare every pad bearing against the spawn facing. Give the hub prompt its fade/first-run rule so the F-177 line has an acceptance criterion and stops being re-openable.

**F-IDs:** `F-070; the hub itself is M-001, which needs a row (rank 4)`  ·  **Blocked by:** Q-066 (turn the spawn or move the pads), Q-059 (how long the line stays up)

This is literally the first ten seconds of the game and the player is looking at a wall. The pads sit at 150.7 / 180.0 / 209.3 degrees against a 45.7 degree half-FOV. The fix is one number and it has been deliberately withheld for a hub-line round that has now been refuted four times — the cheap fix is blocked by the expensive one, which is the circling in miniature.


### 7. Make the anchor field decide where the hook goes — build F-024, or delete the field. If build: wire vector::hook to world::AnchorField so Q and E snap to the best left/right candidate, with the three aim modes (FREI / ASSISTIERT / SNAP) the row specifies, and F-028's no-candidate fallback so no keypress is ever silent. F-016's regulator and F-023's scoring already exist and are already user-facing settings.

**F-IDs:** `F-024 (prio1, 8 pd); unlocks F-016, F-025, F-028; validates F-021, F-022, F-023, F-031a`  ·  **Blocked by:** Q-067 (build or delete); then Q-010 (anchor density as a number)

8108 authored, validated, spatially indexed anchor points are generated every load and decide NOTHING — vector::hook has never heard of AnchorField, and the HUD draws rings for a search the hook does not perform. B-011 was closed by removing the Q/E letters from the markers rather than by wiring the snap. This is prio1, it is the dependency of F-016 and F-028, it is the single biggest built-but-inert asset in the repository, and it is what makes level design of anchor density mean anything. Doing neither — which is today — is the worst of the three states.


### 8. Restore combat evidence: prove in the running game that a titan dies, unscripted where possible. Retimed f030-cortex and f030-hitbox from rank 3, then one photographed nape kill per kind (seven kinds have data, a brain and a rig; only the husk has ever been photographed). Fix F-031a's validation gate while here — src/world/anchor.rs:222 is_clean() does not check `holes`, and the shipped district reports holes 7 and passes as clean. Fix F-043's hit marker to print the quantity the damage formula now actually computes.

**F-IDs:** `F-030, F-031, F-031a, F-032, F-043, F-051, F-057..F-063`  ·  **Blocked by:** rank 3; Q-047 (rear-only nape); Q-019/Q-026 (cortex readable at 100 m or 28 m)

Right now the only place a titan visibly dies is one 284-line rope script, and a full sortie (game-full) ends with 0 kills where it expects 3. The code is fine — that was proven with a one-number change — but the project's own evidence says the core loop is broken, and every combat round will keep spending itself refuting that ghost until the pictures exist. Six of seven kinds cannot rise above built-but-unseen without them.


### 9. F-014 Momentum-Chaining — an experienced player can INCREASE speed across five hook changes instead of losing it. Build it, and build the instrument that measures it: speed at each hook release and each new bite, across a fixed route, with the n=2 case written first and the two anchors made to DISAGREE.

**F-IDs:** `F-014 (prio1, 10 pd, depends_on F-004)`  ·  **Blocked by:** ranks 2, 3, 5, 7

This is the Vector Gear gate criterion written as a feature. pillars.md's whole sentence — a movement game with a high mastery ceiling, in which fighting is the side effect of good movement — is unfalsifiable until chaining exists and is measurable. It is prio1, 10 pd, and it is the dependency of the only mode that tests the gate. It cannot be measured while boost is dead, while Ctrl pins the player, or while the corpus is amber, which is exactly why it sits here and not earlier.


### 10. Sound, first pass: gas hiss tied to actual consumption, hook fire and hook bite, blade cut, titan footstep, and the cortex kill. src/sound/mod.rs is 24 lines with an empty Plugin::build and there are zero audio files in the repository. Only the user can judge it; agents can only assert that the events fire.

**F-IDs:** `no vector-domain audio row exists in features.ron — assign one in rank 4`  ·  **Blocked by:** Q-069 (build now or stay silent)

This is not a meta system and it does not sit behind the gate — it IS the movement's feedback channel. Every verb the Vector Gear has (gas, hook, blade) is currently silent, so the gate itself cannot be judged fairly: nobody can rate this against Attack on Titan Revolution with the audio off. It is placed after chaining because chaining decides what the gas hiss has to track, and before the gate because the gate is a felt judgement.


### 11. F-077 Traversal Trial — checkpoint time trial on the existing map, with a persisted best time and a ghost replay. Reuses Ashgate; no combat, no new content.

**F-IDs:** `F-077 (9 pd, depends_on F-014); enables F-160`  ·  **Blocked by:** rank 9; Q-068 (promote it to prio1 or leave it Should)

pillars.md calls this the litmus test of the whole project; gameplay/features.xlsx ranks it Should/prio2 behind F-014. That contradiction is one of the reasons the gate has never been approachable — the design's own measuring instrument is not scheduled. It is the cheapest possible way to turn 'does the movement feel good' from an opinion into a number the user can beat.


### 12. RUN THE GATE. The user plays a Traversal Trial run and a sortie, with sound, and says yes or no in one sentence into docs/QUESTIONS.md. Whatever he says gets written down as the gate result with a date, and the 224 features behind it either unlock or stay locked with a reason.

**F-IDs:** `the exit condition for all 224 unbuilt rows`  ·  **Blocked by:** Q-062 (who is the gate), ranks 9 and 11

pillars.md:26-30 defines the gate as a blind test against Attack on Titan Revolution with ten testers and then states outright that an agent cannot satisfy it. So the gate defaults to NEVER, and nobody chose that. Every meta system in this project is parked behind a door with no handle, which is why rounds keep drifting to whatever is nearest to hand — and what was nearest to hand turned out to be HUD and settings work that PLAN-GAME.md §10 explicitly defers. Nothing below this line should start until this is answered.


### 13. The Ashgate ruin pass. docs/gameplay/world.md: the war is already lost, Ashgate has long since fallen, the Vanguard runs salvage into its own ruins. What ships is an intact, tidy, inhabited walled town. Zero hits for ruin/rubble/collapse in maps.ron's hand-placed blocks; 14 ruin and rubble models ship in the pack. Also decide the 203 untextured hand-placed blocks — the wall, both gatehouses, the gantries, the bell towers, the garrison hall — which are the entire silhouette of the district.

**F-IDs:** `F-003, M-002 (30 pd, Must, no F-ID today)`  ·  **Blocked by:** Q-070 (ruins, keep the town, or a ruin pass on top); rank 12

The user said this himself on 2026-08-18: 'weil aktuell ist das nicht die echte map!' and it has not been actioned since. It is placed here, not earlier, because it is expensive (the wall is one 700 m band against an 11.20 m tile module, so dressing it means re-cutting maps.ron and re-aiming f003-ashgate's 40 asserts) and because it changes nothing about whether the movement convinces — which is the only question the gate asks.


### 14. The reason to fly a second sortie — ONLY after the gate says yes. Fill DebriefLedger (it is a named, documented, empty Node) with the XP, level and rank the sortie earned; put something in progress.ron gates so may_fly is called by the game and not only by tests; build the loadout screen so the ~200 earned gear points can be spent. Decide the difficulty ladder: in 321 recorded sorties only Recruit has ever been cleared.

**F-IDs:** `F-120, F-121, F-122, F-125, F-080`  ·  **Blocked by:** rank 12 (the gate); Q-051 (keep or park progression); Q-072 (what the debrief shows)

The loop closes and nothing on the other side of it changes — that is the honest blocker for retention, and every piece of it already has code and tests. But PLAN-GAME.md §10 lists all of progress/ as forbidden by the gate and names F-120 explicitly, and progression was already started behind that gate once. It goes here, after the gate, or the rule that has kept this project's scope honest stops meaning anything.


---


## 5 · The questions and the raw reader maps — 📦 archived

All 23 questions were put to him and answered on 2026-08-27 and 2026-08-29; the live record is
[`QUESTIONS.md`](QUESTIONS.md) and [`archive/QUESTIONS-answered.md`](archive/QUESTIONS-answered.md).
The six readers' raw maps and the questions as they stood are in
[`archive/PLAN-2026-08-27.md`](archive/PLAN-2026-08-27.md).

⚠️ **Those maps describe a tree that has changed under them** — the anchor field is deleted, the
ground is a continuous height field, the titans are twice the size, the wall is cut into modules,
the river has water. Read them as a record of 2026-08-27, not as a description of the game.


## §5E smooth terrain — the round plan (scouted 2026-09-02, runs after the 6-commit push)

His words: "ok und die welt hat jetzt harte höhen. aber es soll smooth sein und mehr
elevation! also richtiges terrain!" (§5E) · grass, "nicht verschiedene hardcoded stufen
sondern wirklich terrain" (§5A).

**Design (scout report, full version in the session scratchpad):** `shared::TerrainField`
stays the ONE writer but stores f32 corner heights (quantised `step`/`rise_m` deleted). THE
surface is the triangle mesh over that grid with the fixed diagonal (i,j)->(i+1,j+1);
`height_at_m` evaluates the same triangulation — collider (one static trimesh per map, hole
cells cut for the canal), render mesh and oracle are one surface from one
`corner_heights()` slice. `data::Terrain` gets `elevation_m` (THE amplitude knob) and
`max_grade` (~0.35 m/m ≈ 19°, under the 50° grounded limit); `amplitude_m` renamed to
unitless `amplitude` so every stale RON crashes loud; `rise_m` deleted. Footings keep
`lowest_over` (plinth-by-burial, ≤ ~1.9 m worst case) — terrace-cut rejected as circular
and cascading. B-018 class dissolves under the max-grade guard.

**Fan-out A → B → C, strict sequence, one writer per file:**
- A (field): src/shared/terrain.rs · data/mod.rs Terrain block · maps.ron terrain blocks ·
  tests/data.rs. Interface frozen before B: `corner_m`, `corner_heights`, `height_at_m`
  (triangle-exact), `lowest_over_m`. Acceptance: --lib grade/pin/rim/seed tests, relief
  grows with elevation_m (1x vs 2x).
- B (world): world/map.rs · new shared/ground.rs (TerrainSheet component) · shared/mod.rs ·
  tests/world.rs. Pads + greedy merge deleted, ONE terrain entity (trimesh + Body +
  AnchorSurface + TerrainSheet). Acceptance: --test world grade invariant on Ashgate,
  relief ≥ 30 m, pin sweep exact-zero, 50-house never-floats sweep; --lib; vector_aiming +
  player untouched-green.
- C (render+evidence): render/mod.rs (TerrainSheet->Mesh, same diagonal) ·
  scripts/w2-terrain-walk.txt re-pins · two offscreen screenshot pairs at two elevation_m
  values (street: no step edges visible; aerial: relief visibly doubles). Q-086 (how much
  elevation) is answered by the two photographs, values 24/48 are proposals.

**Risk lines the agents must carry:** trimesh internal-edge jitter (w2 walk is the
detector, avian edge flags the fallback) · canal holes must be cut exactly or ground pokes
through the water box · block-count tests move deliberately when ~6300 pads vanish ·
titan-ring spawn at y=0 depends on the flat disc (assert map.rs:1285 guards it).

## B-041 fix — the design (scouted 2026-09-02, builds after the B-042/B-043 round lands)

**Candidate D won: a second instance of the aim resolution in the EMPTY `World` set.**
`SimulationSystems::World` has been empty since aim moved to PostStep, and its doc never
stopped saying "the aim ray". Add `pub fn pre_fire_aim(<identical params>)` in
`src/vector/aim.rs` forwarding to `aim` (distinct fn type = distinct SystemTypeSet),
registered `in_set(SimulationSystems::World)`. Per tick: World runs it with THIS tick's
delivered look on the end-of-last-tick transform (the same Vec3 the drawn frame used —
nothing writes Transform between PostStep and Intent); `update_hooks` in Intent then
consumes a direction-fresh ArmAim; PostStep's `aim` overwrites for the HUD exactly as
today. When the look did not change, inputs are bit-identical through the same code and
`set_if_neq` elides the write — the behavioural delta is confined to ticks where the look
moved. Decision 6 in hook.rs stays literally true (hook re-casts nothing); the assist
resolves once, in aim.rs. Rejected: A (hook into PostStep — breaks the one-tick-lag pins,
gas contract, forced-aim harness), B (fire-time re-cast — harness cannot intercept a real
cast, both vector test files die), C (latch to next tick — adds the latency the user
fought twice).

**Build agent owns:** src/vector/aim.rs · src/vector/mod.rs (register + rewrite the
:72-84 comment block, now the documented-bug prose) · src/vector/hook.rs (decision-6
header amendment ONLY) · tests/vector_hooks.rs:124 + tests/vector_rope.rs:113
(`force_aim.in_set(World).after(pre_fire_aim)` — NEVER `.after(aim)`, that is the exact
B-030 shape) · docs closes B-041 + new FIND. Also: src/debug/mod.rs:363-365 stale claim
becomes true again — touch up.

**Acceptance in order:** scripts/b041-stale-look.txt 7/7 green exit 0 (A/C/E/F flip,
B/D/W stay) · tools/test.sh --test vector_aiming with the printed worst-px at FIND-219
magnitude (med 0.000 / max 0.01) captured · full tools/test.sh (schedules guard) · re-run
whole binaries vector_hooks, vector_rope, vector_aiming, hud, input, latch, player (f176
timing must NOT move — that is candidate A's failure mode, absent in D) · regression
scripts f-001-hooks, f172-hook-toggle, f176-pull, f026-turn, f002-look(+turned) ·
b043-flick-gap re-measured (the 1.02 m @ 14 m one-tick term collapses to ≈0; read its
asserts before re-pinning) · THEN f025-chain re-run, never re-pinned first (FIND-228).
Scout's open point: whether b043-flick-gap pins red numbers or is a pure harness — read
before re-running.
