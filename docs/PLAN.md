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

## 5 · Every question that needs the user


**23 questions.** Ordered by how much they block.


### Q-062 — Two hundred and twenty-four features are locked behind a gate that says 'ten human testers rate our movement at least level with Attack on Titan Revolution' — and the design document itself says an agent can never satisfy that, so today the answer is 'never' and nobody chose it; will you be the gate?

**Why it matters:** This is the single largest blocker in the project and it is invisible because it never fails loudly. Every meta system — progression, gear, modes, the skill tree, the economy — is behind it. Because it can never open, rounds drift to whatever work is nearest to hand, and what was nearest to hand for the last week was a HUD line and a settings screen that the project's own plan explicitly defers. This question is why the last seven rounds looked random.


**Options:**

- You are the gate: you play one Traversal Trial run and one sortie, with sound, and say yes or no in one sentence. Costs you about twenty minutes and unblocks everything behind it.
- Keep the ten testers: costs nothing today, but every meta feature stays frozen indefinitely and this project can only ever build movement — which is a legitimate choice, it just has to be a chosen one.
- Drop the gate entirely: costs the one rule that has kept the scope honest for eighteen days, and meta systems start getting built on movement nobody has judged.

**Recommendation:** Option A. You are the only human who has ever played this. One sentence from you is worth more than ten strangers you do not have, and it turns a permanently closed door into a date in a file. Keep the wording of the gate as it stands for a future real playtest, but let your verdict be the operative one for now.


**Blocks:** Build order ranks 12, 13 and 14 entirely; and it is the reason ranks 1-11 keep getting bypassed


### Q-063 — Is gravity staying at -32, or are you still tuning the fall?

**Why it matters:** Twelve evidence scripts encode a fall time computed for -20. Retiming them is roughly half a day of arithmetic and it must not be paid twice. Until it is paid, the project cannot tell a real bug from a stale number in any of them, and every round starts by re-triaging about forty red asserts.


**Options:**

- Freeze -32 now: I retime the corpus today and combat, movement and the hub all have working evidence again by the end of the round.
- Still tuning: everything stays red, the scripts stay stale, and there is no proof a titan can be killed until you settle it — say when.
- Go further, to -55 (the Roblox-equivalent number the file's own comment names): costs a full re-derivation of every movement constant, plus the same corpus re-aim, plus a re-tune of jump, boost and air control together.

**Recommendation:** Freeze -32. It is a number you chose today; make it real, let me re-aim once against it, and if it still feels wrong after you play, changing it again then costs the same half day it costs now — but at least the intervening rounds will have been measurable.


**Blocks:** Build order ranks 2, 3 and 8 — nothing in combat or movement can be proven until this is fixed


### Q-064 — Shift barely lifts you any more — a boost that used to carry you 28 metres now carries you 4.17 — so what should Shift be?

**Why it matters:** boost_m_s2 is 34 and gravity is now 32, so Shift nets +2 m/s2 against your own weight instead of +14. You wrote the acceptance sentence for the whole movement system yourself: it should be possible, if you are good, to stay in the air the whole time until the gas runs out. On gas alone you now sink. Nobody re-derived this pair when gravity moved.


**Options:**

- Raise boost to about 54 so Shift keeps the same 1.7x punch over gravity it had before: one RON value plus a re-aim of the boost scripts, and gasless flight is possible again.
- Leave 34 so the ropes are the only real engine and Shift is only 'a bit more' — this matches your own note that the ropes give good acceleration and Shift should just add more, but it makes staying up on gas alone impossible.
- Make Shift horizontal-only thrust — it never fights gravity, it only makes you faster along your current line: costs a rewrite of the boost blend and changes how every existing boost script reads.

**Recommendation:** Option A, raise it to about 54. Your own acceptance sentence is the tiebreaker against your own later note, and it is the one that describes what the game is supposed to feel like. Option B is defensible but then the acceptance sentence has to be withdrawn in writing, not just quietly missed.


**Blocks:** Build order rank 2, which blocks rank 3, which blocks rank 8


### Q-065 — When you have both hooks planted on anchors far apart and you hold Ctrl to reel in, the game currently drags you onto one anchor and pins you at exactly zero — what should happen instead?

**Why it matters:** Both ropes shrink toward three metres while the anchors stay fifty-six metres apart, which no position in space can satisfy, so the physics abandons one arm. Measured: infeasible on 64.4 % of ground geometries and 64.5 % in the air, with one rope ending fifty metres past its own maximum. This is reachable with one key in a game that is entirely movement, and it is currently tracked in no bug list at all.


**Options:**

- Stop reeling an arm the moment the pair becomes impossible — you keep both hooks, the reel just stalls. Costs one geometric rule in the one function that already owns rope length.
- Auto-release the losing arm — you keep reeling and lose a hook. Costs a new release reason and a HUD message telling you why your hook went.
- Leave it and I mark it a known trap: costs nothing and leaves two thirds of two-anchor geometries undefined.

**Recommendation:** Option A. Losing a hook you did not release is the kind of thing that reads as the game cheating; a reel that just stops reading is legible. Option B is a good second choice IF the HUD says why, which is a whole extra piece of work.


**Blocks:** Build order rank 5; and it silently corrupts any measurement of chaining (rank 9)


### Q-066 — You spawn in the hub with your back to every door that starts a mission — should the spawn turn around, or should the doors move?

**Why it matters:** The three skirmish pads sit at 150.7, 180.0 and 209.3 degrees behind you against a 45.7 degree half field of view. The only pad on screen is one corner of another mission, 0.66 % of the frame. The mission file already claims the recruit pad 'stands straight ahead of the spawn point' — it is simply not true. No test has ever compared a pad bearing to the spawn facing.


**Options:**

- Turn the spawn to face the doors: one facing value, re-run two hub scripts, done in minutes.
- Move the three pads to the other side: changes coordinates that four different scripts walk to by hand, so it costs a re-aim of all four.
- Leave it and put an arrow or a marker on screen instead: costs a HUD element, and it solves the symptom rather than the fact that the room is laid out backwards.

**Recommendation:** Option A. It is one number, the mission file already asserts the outcome, and it is the first ten seconds of your game. This fix has been withheld for a week waiting on a hub-line round that has been refuted four times — that is the wrong way round and it should just land.


**Blocks:** Build order rank 6; and it makes every unscripted play session start with confusion


### Q-067 — The game draws a field of anchor markers all over the city and the hook completely ignores them — should the hook actually use them, or should the markers come off the screen?

**Why it matters:** 8108 anchor points are generated, validated and spatially indexed on every load, and drawn as rings on your HUD. The hook fires a raycast at your crosshair and has never read that field. So what you see and what you get are two different searches. The Q and E letters were already removed from the markers last week because they were a lie; the markers themselves stayed.


**Options:**

- Build the snap (F-024): Q and E anchor to the best candidate on that side without you having to aim exactly. Three modes — free aim, assisted (the default), full snap. Costs about one round, and it changes how every shot in the game lands.
- Delete the field and keep pure raycast aiming: cheap, makes the game honest immediately, and throws away five features' worth of anchor work including the density design.
- Leave it as it is: the field stays decoration and the markers keep promising something the hook does not do.

**Recommendation:** Option A, build it. This is prio1 in your own backlog, the aim-assist regulator and scoring it depends on are already built and already have sliders in your settings menu, and it is the piece that makes anchor density on a map mean anything. Option C is what we have now and it is strictly the worst of the three.


**Blocks:** Build order rank 7, which blocks rank 9 (chaining) and rank 11 (the trial)


### Q-061 — You have a hook in something ahead of you, you are standing on the ground, and you hold S to back away — should the rope draw you slowly toward the anchor anyway, or should you stay planted?

**Why it matters:** You said on 2026-08-26 that the rope should always pull, not just when you press W. The build now pulls you in slowly even on the ground while you walk backwards (36.72 m to 35.64 m in one second). One evidence script asserts the opposite and is deliberately red waiting for your answer.


**Options:**

- Drawn in slowly: matches what you already said, costs one line in a script and the run goes green.
- Planted on the ground, always pulls in the air: costs a ground gate on the pull, and it contradicts your own 'it should always pull'.
- Drawn in, but S is allowed to hold you flat — you never retreat, you never advance: costs a small clamp and is the most explainable of the three.

**Recommendation:** Option C. It honours 'always pulls' while making S mean something, and it matches the acceptance you already wrote for the rope: with any key held, the anchor distance must not increase; S may hold it flat, nothing may make it rise.


**Blocks:** One red script; and it settles what the rope's ground behaviour is before chaining is measured


### Q-047 — Should a nape cut only count from behind the titan, or from any angle?

**Why it matters:** You wrote 'hitboxen passen noch nich!' on 2026-08-20 and it has been open since. The nape is currently rear-only, roughly 110-115 degrees behind the titan, which makes the approach angle a skill. This decides whether the fight is a positioning problem or a clicking problem, and it is the reason several cuts you think should land do not.


**Options:**

- Stay rear-only: costs you kills when a titan turns mid-swing, and keeps the approach as the skill the whole cortex feature exists to create.
- Any angle: makes the titan a floating bullseye, much more forgiving, and deletes the one skill expression in the cut.
- Rear-only but wider (say 160 degrees): a middle position — you still cannot cut a titan head-on, but a partial turn no longer robs you.

**Recommendation:** Option C. Rear-only is the right principle and 110 degrees is probably too tight given the titans turn while you are committed to an approach. Widening is one number and you can feel the difference immediately.


**Blocks:** Build order rank 8 — the per-kind combat evidence should not be photographed against a hitbox you are about to change


### Q-019 — Should you be able to see and read a titan's nape from a hundred metres out, or only from about thirty?

**Why it matters:** Two documents disagree by a factor of 3.6. The pillars document says the cortex is recognisable from 100 m; the feature's own acceptance converts to 28 m. At 1920x1080 that is 10 pixels versus 37. The nape size also never grew when the titans doubled in height, so on the biggest kind it is now 6.7 % of the body where it used to be 14 %. This decides whether the fight is about approach lines or about precision at low speed.


**Options:**

- Readable at 100 m: the nape must get considerably bigger, which changes every titan silhouette and makes the kill notably easier.
- Readable at 28 m: approaches stay tight and demanding, and the pillars document gets amended to say so.
- Scale the nape with the titan's height rather than fixing an absolute size: keeps the big kinds killable at speed but makes the small ones much easier.

**Recommendation:** Option C combined with option B's distance. Scaling with height is what your eye expects from a bigger creature, and 28 m keeps the approach as the skill — but this one really needs you to look at a bellower and a husk side by side before deciding.


**Blocks:** Build order rank 8; and it is the readability half of the cortex feature's acceptance, which has never been satisfiable


### Q-069 — The game is completely silent — no gas hiss, no hook bite, no blade, no footsteps, no audio files at all — do we build a first pass now?

**Why it matters:** Every verb the Vector Gear has produces no sound. That is not polish: gas, hook and blade are the three things you are constantly judging by feel, and right now the only feedback for all three is a number on the HUD. It also means the movement gate cannot be judged fairly, because nobody rates a movement game against a reference with the audio off.


**Options:**

- Build a first pass now — gas hiss tied to actual consumption, hook fire, hook bite, blade cut, titan footstep, cortex kill. Roughly one round, and only you can ever judge it; an agent can only assert that the events fire.
- Stay silent until the movement is settled: costs nothing today, but every play session you do until then is judging an incomplete thing.
- Minimum viable — gas and hook bite only, two sounds: about a quarter of the work and it covers the two things you are constantly listening for in the reference game.

**Recommendation:** Option C first, then A. Two sounds is a fraction of the cost and it closes most of the feedback gap for the movement specifically. Do it before the gate, not after — you cannot fairly judge the flying with the flying making no noise.


**Blocks:** Build order rank 10; and it degrades the quality of the gate verdict in rank 12


### Q-068 — Your design document calls the Traversal Trial 'the litmus test of the whole project'; your feature spreadsheet ranks it as a nice-to-have behind other work — which is right?

**Why it matters:** The Traversal Trial is a pure movement time trial with checkpoints and a ghost replay, on the map that already exists. It is the only thing in the whole design that would turn 'does the movement feel good' into a number you can beat. It depends on momentum chaining, which is itself unbuilt and is literally the gate criterion written as a feature.


**Options:**

- Promote both to the front: about 19 person-days, delays combat depth, and gives you a repeatable way to feel whether a change made the movement better or worse.
- Leave it as a nice-to-have: costs nothing now, and the movement gate stays an opinion nobody can measure — which is how we got here.
- Build a stripped version — checkpoints and a timer, no ghost, no leaderboard: perhaps a third of the cost and it still gives you a number to beat.

**Recommendation:** Option C, then A later. A timer and five checkpoints on Ashgate is a small piece of work and it immediately makes every subsequent movement change falsifiable by you in sixty seconds. The ghost replay is the expensive half and it can wait.


**Blocks:** Build order ranks 9 and 11, and therefore the gate in rank 12


### Q-071 — Maps have no feature ID, so the twelve maps in the backlog — including Ashgate itself, thirty person-days of must-have work — have never once appeared in the project's own queue; should they get rows?

**Why it matters:** The hub you spawn in is 'M-001 The Rookery' in the backlog, with no ID, no stage and no acceptance criterion — which is exactly why work on it has been running under a borrowed ID that actually belongs to the graphics settings feature, and why that work has been refuted four times without ever being able to close. Work with no row cannot be finished; it can only be abandoned.


**Options:**

- Add M-001..M-012 to the spreadsheet as rows: costs you one edit, and thirty person-days of must-have work becomes visible to every future session.
- Leave maps in their own file only: costs nothing now, and the single largest item on the critical path stays invisible to the queue forever.

**Recommendation:** Option A, and I would extend it: any work stream that runs for more than one round needs a row with an acceptance criterion, or it has no stopping condition. That is the mechanical cause of the circling you noticed.


**Blocks:** Build order rank 4; and it is the structural fix for the failure mode this whole survey exists to stop


### Q-070 — Your world document says Ashgate has already fallen and the Vanguard runs salvage missions into its own ruins — what is built is a tidy, intact, inhabited walled town; which one is the game?

**Why it matters:** You said this yourself on 2026-08-18: 'weil aktuell ist das nicht die echte map!' Fourteen ruin and rubble models ship in the art pack and none of them is placed by hand. It also decides whether the 203 untextured grey boxes that form the whole silhouette of the city — the wall, both gatehouses, the gantries, the bell towers, the garrison hall — get dressed or get demolished.


**Options:**

- Rebuild the district as ruins: about thirty person-days, throws away the current town's hand placement, matches the setting you designed.
- Keep the town and rewrite the setting: costs the premise, saves the thirty days, and the game becomes a different story.
- A ruin pass on top of what exists — collapse some blocks, add debris, break the wall in two places: much cheaper, and it may read as neither one thing nor the other.

**Recommendation:** Option C first as an experiment you can look at, then decide between A and B with a picture in front of you. Deciding this from a document is how it stalled for nine days; deciding it from a screenshot takes you thirty seconds.


**Blocks:** Build order rank 13; nothing before it


### Q-051 — You have earned levels, XP and a gear budget across 321 recorded sorties, and the design forbids every progression system until the movement convinces — does the progression stay or get parked?

**Why it matters:** Levels, XP curves, ranks and gear budgets were built and tested behind a gate that the plan explicitly says forbids them. Nothing about it is broken; it just should not have been started, and every day it stays it pulls twelve more rows toward a skill tree and currencies.


**Options:**

- Keep it, stop adding: costs the work sitting idle, and the rule that protected the scope has been bent once visibly rather than quietly.
- Park and delete: the rollback is already written down file by file, and it costs the work already done.
- Keep it and lift the gate for progression only, in writing: costs the rule, but at least the exception is a decision instead of an accident.

**Recommendation:** Option A. Do not delete working, tested code to make a point, but write down that it is frozen at 'earn only' until the gate opens, and do not build the loadout screen or the skill tree before then. That preserves both the work and the rule.


**Blocks:** Build order rank 14; and it is the credibility of the whole stage ledger


### Q-072 — When you finish a sortie the debrief tells you the mission name, whether you won, how many titans you killed and how long it took — should it also show what the sortie earned you?

**Why it matters:** The XP, level and rank all exist and are all written to a save file you never see. The screen that would show them is a named, documented, completely empty box. It is the one moment in the game where your career changes and nothing tells you it did.


**Options:**

- Show XP, level and rank: about half a round, the numbers already exist.
- Keep it to facts — verdict, kills, clock — until the movement gate is passed: costs nothing, and stays honest to the rule that no meta system comes first.
- Show XP only, no level or rank: a middle position, one number, and it does not yet imply a progression system you have not committed to.

**Recommendation:** Option B until the gate opens, then A. It is genuinely half a round of work but it is the thin end of the meta wedge, and you already have one exception on the books. If the gate opens, do the full version.


**Blocks:** Build order rank 14


### Q-046 — Is a gas tank of 15000 the balance you actually want, or was it just made large so a test run would not run dry?

**Why it matters:** The gas budget system, its priority order and one whole evidence script were designed around a tank of 300. At 15000 the ledger describes a game that no longer exists — gas is effectively infinite in every test, which means no test has ever exercised running out.


**Options:**

- It is the balance: then the budget script and the gas priority order need re-deriving against the real number.
- It was testability: rollback is one number back to 300 and no code moves.
- Somewhere in between — pick a number where a good flight lasts roughly a minute: costs one measurement round to find it.

**Recommendation:** Option C. Your own acceptance sentence is 'stay in the air until the gas runs out' — that sentence only means something if the gas actually runs out, so the tank size should be derived from how long you want a flight to last, not picked.


**Blocks:** Nothing right now, but it will come up the moment boost is restored and flights get measured


### Q-073 — Should the always-anchored rope keep turning you around an anchor when you fly straight past it, or should it let you fly on?

**Why it matters:** You asked for a hard maximum rope length — 'NICHT das seil verlängern!!' — and that is now what the game does under both physics models. The consequence you have not felt yet is that flying past an anchor with W held swings you around it rather than letting you carry on. That is either exactly the reference feel or it is a leash, and only playing thirty seconds of it will tell you.


**Options:**

- It is right, keep it: nothing changes, and this is the arc the reference game has.
- It is a leash: the rollback is one branch in the rope attachment, and the rope design needs a fourth attempt.
- Keep the maximum but make it forgiving near the limit — the rope stretches a little before it turns you: costs a soft-limit tuning pass and it is the hardest of the three to get right.

**Recommendation:** Play it first and answer from the seat, not the document. This is the one question in this list that no amount of measurement can answer and the one where your thirty seconds beats a whole round of my work.


**Blocks:** Nothing formally, but three rope rounds have already been spent on this and a fourth would be the tiebreaker


### Q-074 — When you cut a titan, the hit marker prints your closing speed — it can now print the actual damage instead; which do you want to see?

**Why it matters:** The marker was written when the damage formula had no reader at all, so speed was the only number available. It has had a reader since 2026-08-25 and nobody went back. Speed tells you how well you flew; damage tells you how much good it did.


**Options:**

- Speed: keeps the emphasis on the movement, which is what the game is about.
- Damage: tells you whether that cut mattered, which is what you actually need to know mid-fight.
- Both, small: costs one extra text element and slightly more clutter on an already busy crosshair.

**Recommendation:** Option C, with damage large and speed small underneath. The damage answers 'did that work', the speed answers 'why', and you want both during the two seconds you have to decide whether to come round again.


**Blocks:** Build order rank 8, minor


### Q-028 — The largest titan, the bellower at 21 metres, is blocked from ever spawning and appears in no mission — has anyone ever been supposed to fight one?

**Why it matters:** He is the only one of the eight kinds nobody has ever met. He is blocked by a size cap because the streets are seven metres wide and he would clip through buildings, and his whole design — he hunts by the sound of your gas, and the counterplay is to fly quietly — depends on a hearing system that does not exist. The stealth layer your design calls the reference game's biggest gap is entirely inside this one kind.


**Options:**

- Fight one this session: means either widening the streets or accepting he clips through buildings, and he will have no hearing so the fight will be nothing like his design.
- Shelve huge titans until the city is rebuilt: costs nothing, and the most distinctive enemy in your roster stays theoretical.
- Build the hearing system first and fight him in open ground outside the walls: a middle path that tests the actual design without needing the city rebuilt.

**Recommendation:** Option C. The bellower's whole point is the hearing, and fighting him without it would prove nothing except that a large model clips through houses. Outside the wall costs no level design at all.


**Blocks:** Nothing in the build order; it will come up during rank 8


### Q-075 — The lobby offers to host a multiplayer game, and a second player who joins can move a body in your world but cannot see anything — what should that row do?

**Why it matters:** The networking is input-only: a peer's keystrokes arrive and drive a character in your process, and nothing is ever sent back. So the Host row advertises something that does not work. Making it work means sending world state, which touches every domain in the project.


**Options:**

- Build state replication now: several rounds, expensive, and it touches every system including the ones still being tuned.
- Remove the Host row until there is a real client: costs nothing and stops the menu promising something false.
- Leave the row and label it clearly as not implemented: costs one word and keeps the seam visible as a reminder.

**Recommendation:** Option B. The architecture rules have kept multiplayer cheap to add later, and that was the right call — but a button that does nothing is exactly the kind of small lie that makes the rest of the game feel unfinished. Take it out until there is something behind it.


**Blocks:** Nothing; it will come up whenever the menu is next touched


### Q-029 — Twenty-six numbers in the titan, gear and scale data files were invented by me rather than chosen by you, and have stood unreviewed for sixteen days — do you want to see them once?

**Why it matters:** Every measurement in this repository ultimately rests on those values: titan health pools, blade damage per metre per second, the metres-per-stud conversion under every other number, the maximum rope length, the hook range. If any of them is wrong, every tuning conclusion drawn on top of it is wrong too, and nobody would notice.


**Options:**

- Read them out once for a yes or no: costs one screen of numbers and about ten minutes of your time.
- Treat them as settled until something feels wrong in play: costs nothing now, and any of them could be quietly distorting what you feel.
- Review only the five that touch movement — conversion, rope length, hook range, max speed, gas cost — and leave the combat numbers: about two minutes and it covers the ones that matter before the gate.

**Recommendation:** Option C. Before you sit down to judge the movement, the five numbers underneath the movement should be yours and not mine. The combat twenty-one can wait until after the gate.


**Blocks:** Nothing hard, but it quietly underwrites the gate verdict in rank 12


### Q-059 — The line of text that tells you what to do in the hub — should it stay up the whole time you are there, fade after a few seconds, or only appear on your first ever sortie?

**Why it matters:** This one line has now consumed four rounds of work without ever closing, because nothing anywhere says what 'done' looks like for it. All three behaviours are the same small amount of code; what is missing is a decision.


**Options:**

- Always visible while you are in the hub: simplest, and it never stops being useful to a new player.
- Fades after a few seconds: costs one number, and it keeps the hub clean once you know the place.
- First sortie only, then never again: costs one flag in the save file, and it treats the hub as a tutorial rather than a room.

**Recommendation:** Option B, fade after about eight seconds, and it reappears if you stand still. It is the only one of the three that is right both for your first hour and your hundredth. Whatever you pick, this is the acceptance criterion that lets that work finally close.


**Blocks:** Build order rank 6 — and, more importantly, it is what stops that round reopening a fifth time


### Q-004 — The Vessel Forms — the nine-feature transformation system that replaces the core movement rather than extending it — are they in the first version or a later one?

**Why it matters:** This is the single most expensive item in the whole backlog: roughly 85 person-days plus character rigs and about sixty animations, and the core loop document says outright that they replace the movement rather than adding to it. It has been open since day one and it is the largest unpriced thing in the plan.


**Options:**

- Later version: costs nothing now, keeps the scope of the first game honest and finishable.
- First version: costs about 85 person-days plus art, behind a gate that has never been passed, for a system that changes what the game is.

**Recommendation:** Later. Nothing about the game's core sentence — a movement game where fighting is a side effect of good movement — needs them, and committing 85 person-days behind an unopened gate is exactly the shape of decision that has cost this project time already.


**Blocks:** Nothing today; it needs answering before any long-range plan is written


---

## 6 · The six readers' raw maps


### The Vector Gear — src/vector/ (hook, aim, gas, boost, dodge, reel) and src/player/ (rope, locomotion, integrator), their tests and scripts

**The one thing most blocking the player:** "Shift no longer boosts. `boost_m_s2: 34` against `gravity_m_s2: -32` leaves +2 m/s² net upward where it was +14, and nobody re-derived the pair when gravity moved today. Measured in the running game: `scripts/f-007-boost.txt` expects `Height > 28.5` after the boost leg and gets **4.170 m**, `Speed > 27.5` gets **3.679 m/s**. That single number kills the acceptance sentence the user himself wrote for the whole movement system (*„es soll möglich sein wenn man gut ist die ganze zeit in der luft zu bleiben bis das gas ausgeht\"*): on gas alone you now sink. The rope was fixed this week; the thing the player presses to stay up was broken the same day, silently, in a one-line data change."


**Works, with evidence:**

- THE HOOK FIRES AND BITES, AND IT IS EFFECTIVELY INSTANT. `hook_range_m: 500`, `hook_speed_m_s: 500`, and `hook_flight_max_s: 0.10` in `vector::hook::flight_per_tick_m` caps ANY shot at 6 ticks — the user's *„was instant sein soll"* (NEXT §1A) is met and visible. Evidence: `scripts/f-002-aiming.txt` 3 asserts held / 397 ticks, `scripts/f002-look.txt` + `f002-look-turned.txt` 3 each, `scripts/q048-one-point.txt` 12 asserts held / 624 ticks, all run today [offlinebot] against the working-tree binary.
- THE AIM ASSIST IS COMPLETE AND SIDEWAYS-ONLY (§3A is BUILT, not just decided). `vector::aim::probe_dirs(basis, catch_rad, steps, side)` is a 1-D sweep in the CAMERA right axis (`look*cos + right*(sign*sin)`), split Left/Right per arm; `pick_best` + `score_candidate` + `required_margin(margin_full, strength_pct)` implement F-023/F-024's scoring, F-025 and F-016's regulator. Both knobs are real user-facing settings (`src/menu/settings.rs`, `assist_catch_pct` / `assist_strength_pct`) with a script verb (`settings assist_strength N`). Evidence: `scripts/f016-band.txt` 9 asserts held / 368 ticks today.
- THE LANDING PREVIEW STANDS IN THE WORLD, not on the crosshair — the user's *„nicht nur am fadenkreuz. weil das stimmt auch nicht"* is answered. `hud::arm_aim::place_arm_aim` projects THAT arm's own `ArmAim` point through the real camera; `tests/hud.rs::f026_the_marker_stands_exactly_where_that_arm_fires` uses `assert_eq!` with no tolerance and `f026_the_rope_flies_at_the_point_the_marker_stood_on` fires the hook and compares `HookState::Flying{target_m}` against the same `Vec3`.
- GAS IS A WORKING LEDGER. `vector::gas::gas_budget` + `book(priority, wants, costs, gas)`, priority `[Boost, Steer, ReelIn, Dodge, Flip]`, tank 15000. Evidence: `scripts/f-018-gas.txt` 11 asserts held / 1362 ticks today; `tests/vector_gas.rs` binary run directly: 26 passed, 0 failed. The gasless-half rule (§1e) is real: `air_control` calls `rope_drive` with `drive_tuning.scaled(air_accel_empty_fraction)` when `gas.is_empty()`, and `air_accel_empty_fraction: 0.5`.
- DODGE AND FLIP ARE BUILT — and the backlog says Unbuilt. `src/vector/dodge.rs` (249 lines) has charges/recharge/cooldown (`spend_and_recharge`) and the sideways flip (`flip_velocity_m_s`, `flip`), both registered in `VectorPlugin`. Double-tap edge detection lives in `tests/input.rs` (`f008_two_space_taps_inside_the_window_are_one_dodge`, `f009_two_a_taps_...`, 12 tests). `game.ron` carries all nine keys (`dodge_charges: 3`, `dodge_recharge_s: 4`, `dodge_cooldown_s: 0.6`, `flip_impulse_m_s: 18`, `flip_iframes_s: 0.35`, `gas_flip: 20`). Evidence: `scripts/f008-dash.txt` 3 asserts held / 391 ticks today. `docs/features.ron` still reads `F-008|Unbuilt` and `F-009|Unbuilt`.
- A/D LATERAL STEERING WITH THE HOOK ANCHORED (F-006). `scripts/f024-sideways.txt` 12 asserts held / 972 ticks today.
- ATTEMPT 3 OF THE ROPE (Q-058, the `DistanceJoint` on a `Drive` rope) IS GREEN WHERE IT WAS MEASURED — it is not broken debris. I ran the pre-built test binaries directly, no rebuild: `tests/vector_rope.rs` **28 passed, 0 failed, 6 ignored** (322.7 s); `tests/player.rs` **58 passed, 0 failed**; `--lib` **280 passed, 0 failed**. `scripts/f176-pull.txt` is **1 of 9 red** and that one line is red ON PURPOSE (Q-061). FIND-195's 288-cell acceptance matrix: worst per-arm excess **+0.0050 m** with the joint against **+51.1978 m** against the rollback — four orders of magnitude, so it is a guard and not a tolerance.

**Stubbed — exists, does nothing:**

- F-024 — THE SNAP. The anchor field exists and is drawn (`src/world/anchor.rs` 787 lines; the running game logs `anchors: 1564 named hook.* points adopted, 9672 in the field`; `src/hud/anchor_marks.rs` paints twelve rings with F-027's density cap). **`vector::hook` never reads `AnchorField`.** Q and E fire the raycast; the field is decoration. B-011 was 'fixed' by *withdrawing the Q/E letters from the markers*, not by wiring the snap. `src/hud/anchor_marks.rs` header says this in its own words.
- SHIFT ON THE GROUND. `player::locomotion::ground_locomotion` reads no `Buttons::BOOST` at all (grepped: zero hits), and `vector::boost::gas_boost` has **no `MovementState` gate** — so on foot Shift charges `gas_boost_per_s: 18` per second and `ground_locomotion` overwrites the horizontal component the same tick. The user's *„auf dem boden soll ich damit rennen können"* (§5f.3) is unbuilt, and it is not free — it costs gas.
- `src/vector/reel.rs` (53 lines) is a pass-through: `reel_in` only writes `ReelSpeed` from `grant.reel_in`. The actual reel is `player::rope::shorten_ropes`. That is fine as a design, but the file's name promises a mechanic it does not hold.
- `RopeForceModel::Pendulum` and `locomotion::rope_steer` are now a second, unreachable movement model. Since Q-058 both models build the SAME joint with the same birth length; they differ only in one `match` arm in `air_control` (`rope_steer` force vs `rope_drive` + `rope_winch` velocity target) and in which gas consumer pays. `game.ron` ships `rope_force_model: Drive`, so `rope_steer` runs only if somebody hand-edits the file.

**Broken:**

- THE GRAVITY CHANGE MOVED ONE DERIVED CONSTANT AND LEFT THE OTHERS. `gravity_m_s2` went −20 → −32 today; `jump_speed_m_s` was deliberately moved 6.5 → 8.2 to hold the apex. **`boost_m_s2: 34` and `air_accel_m_s2: 10.0` were not touched.** Shift straight up now nets **+2 m/s²** where it netted +14. `air_accel_m_s2`'s own comment in `assets/data/game.ron:100-108` still reads *"10.0 is half of -gravity_m_s2"* and *"strictly below 20 (= -gravity_m_s2)"* — both are now false (10 is 31 %). MEASURED TODAY: `scripts/f-007-boost.txt` asserts `Height > 28.5` and reads **4.170**; `Speed > 27.5` reads **3.679**; the run also cut off at tick 300 with 7 of 24 instructions never run. §1f's acceptance (*„die ganze zeit in der luft zu bleiben bis das gas ausgeht"*) is now arithmetically out of reach on gas alone.
- THE REEL CAN ASK FOR A GEOMETRICALLY IMPOSSIBLE PAIR, AND IT IS ON THE SHIPPED PATH. `player::rope::shorten_ropes` walks BOTH `limits.max` toward `min_rope_m: 3.0` while the anchors stay where they are; two maxima are satisfiable only if `L_left + L_right >= anchor_separation`. FIND-195 measured that false on **16 704 of 25 920 ticks (64.4 %)** on the ground and 64.5 % in the air. FIND-191's point case: two anchors 170° apart, both ropes 30 m, `Ctrl` held → the player is dragged onto the right anchor and sits at **0.000 m/s** with the left rope **50.167 m** past its own maximum. It predates the joint (`Pendulum` agrees to three decimals) but `rope_force_model: Drive` means a player now walks into it with one key.
- TWO SCRIPTS ARE RED IN A WAY GRAVITY DOES NOT EXPLAIN. `scripts/f025-chain.txt`: 8 of 36 failed, and two are not tuning — `line 101: assert Rope == 1 — measured 0.000` (a hook that should have anchored did not) and `line 197: assert Rope == 0 — measured 1.000` (a rope that should have been released survived). `scripts/f029-grapple.txt`: 2 of 6 failed — `Rope == 0` measured 1.000 and `Titans == 1` measured 2.000, i.e. the dynamic-anchor grapple leg no longer finishes its kill. Both run today against the working-tree binary.
- THE SCRIPT CORPUS CANNOT CURRENTLY TELL A REGRESSION FROM AN EXPECTED NUMBER. Measured today (partly by the stopped round's own `corpus.txt`, partly by me): `f-001-hooks` 3 of 14 red, `f003-ashgate` 11 of 40, `f004-towers` 11 of 31, `f005-feel` 1 of 5, `f006-drive` 1 of 10, `f025-chain` 8 of 36, `f029-grapple` 2 of 6, `f-007-boost` 2 of 9 + cut off, plus §3G's known `f175-loop` / `f070-hub` / `f177-door`. That is the evidence base for the whole Vector Gear, and it is amber everywhere at once for three DIFFERENT reasons (gravity, the new joint, real defects) with nothing separating them.
- THE WHOLE OF ATTEMPT 3 IS UNCOMMITTED. `src/player/rope.rs`, `src/player/locomotion.rs`, `src/player/integrator.rs`, `tests/vector_rope.rs`, `tests/player.rs`, `scripts/f176-pull.txt` and the FIND-191/192/194/195 + Q-058/Q-060/Q-061 entries exist only in the working tree. Nothing in git says the feature exists. The code change itself is small and clean: one `match` in `attach_ropes` (both models now spawn the joint) and one guard in `air_control` (`Drive if anchored > 0 && in_the_air`, with `grant.reel_in` removed from the winch and handed to `shorten_ropes`). CLAUDE.md's *„never `git add -A` while an agent is still working"* applies — the tree also carries an unrelated unfinished `src/hud/hub_prompt.rs`.
- B-012 is still open: `f175-loop` reported `11 of 19 asserts failed` twice and then went green nine times, unexplained.


### Combat, titans and blades — src/combat/, src/titan/, src/blades/, assets/data/titan.ron, tests/combat.rs, tests/titan.rs, 12 scripts

**The one thing most blocking the player:** ["Today the titans can kill you and you cannot kill them. The only way to see a titan die in the running game is one 284-line rope script (f-flight-cut); every other cut script lands on a leg, and a full sortie (game-full) now ends with 0 kills and a mission that never reaches Won. The blocker is NOT the cortex code — I proved that by changing a single number in a scratchpad copy of f030-cortex (wait 0.90 -> 0.71), which turned '1 of 2 asserts failed / cut LegLeft at 33.07 m/s' into 'exit 0 / cut titan 1 Cortex at 26.67 m/s'. The blocker is that all seven cut scripts encode a fall time computed for gravity -20, the tests switch gravity off so they cannot see it, and the stage ledger has been stale for 15 days. Until the scripts are retimed, the project's own evidence will keep saying the core loop is broken while the code is fine, and every combat round will spend itself refuting a ghost. Retiming six of the seven is one arithmetic line each; f030-hitbox needs seven."]


**Works, with evidence:**

- THE TITANS CAN KILL YOU. scripts/p5-downed.txt, run today against target/debug/defeated_by_titan (mtime 27. Aug 15:00, newer than every src/ and assets/ file): 5 asserts held, 934 ticks, exit 0, 'MISSION LOST'. The strike cone, the health pool and the second losing condition are real end to end.
- THE CORTEX KILL IS REAL, out of a roped flight. scripts/f-flight-cut.txt: 25 asserts held, 363 ticks, exit 0, log line 'cut titan 1 Cortex at 28.08 m/s'. This is the ONLY script in the repo that shows a titan dying today.
- THE CORTEX KILL IS ALSO REAL out of a free fall — once the fall time is recomputed for the new gravity. Red-check I ran: copied scripts/f030-cortex.txt to the scratchpad and changed ONE number, `wait 0.90` -> `wait 0.71`. Before: '1 of 2 asserts failed', 'cut titan 1 LegLeft at 33.07 m/s'. After: '2 asserts held', exit 0, 'cut titan 1 Cortex at 26.67 m/s'. The mechanism is intact; the calibration is dead.
- tests/combat.rs 54 passed / 0 failed (10.4 s) and tests/titan.rs 38 passed / 0 failed (13.0 s), run from the existing binaries target/debug/deps/combat-b9976576c096dc41 and titan-962524e2bb2e133c (built 26. Aug 20:28, i.e. newer than every file in src/combat, src/titan and src/blades) against the CURRENT assets/data/game.ron with gravity -32. No rebuild needed.
- TITAN AI, TELEGRAPH, SENSES AND RIG ARE GREEN IN THE RUNNING GAME. scripts/f050-states.txt exit 0, f053-windup.txt exit 0, f051-kinds.txt 7 asserts held / 1206 ticks / exit 0, f056-husk.txt exit 0. None of these depends on the player's fall, which is exactly why they survived today's gravity change.
- GROUND MELEE AND BLADE RESUPPLY WORK. scripts/f044-ground.txt 3 asserts held / exit 0 ('cut titan 1 LegLeft at 0.00 m/s' — a standing cut that books wound damage), scripts/f019-supply.txt 3 asserts held / exit 0.
- WAVES SPAWN AND THE MISSION PHASE IS REAL. scripts/f071-won.txt WITH `--mission tutorial`: 5 asserts held, 184 ticks, exit 0, one husk out of the 0 s wave of assets/data/missions.ron. (Without --mission it reports 5 of 5 failed — that is the run's fault, not the game's.)
- SEVEN OF EIGHT KINDS SPAWN AND WALK: husk, errant, scuttler, weaver, warden, lurker, chorus all built rigs in scripts/f030-hitbox.txt today (titan 1..7 in the log). bellower is refused by name, deliberately, by titan::spawnable against scale.ron: titan.max_spawnable_class — tests/titan.rs::f064_the_bellower_stays_blocked_until_the_ear_exists.
- THE DAMAGE FORMULA HAS A READER. src/combat/damage.rs is live in the tick: 'tick 155: titan 1 took 41.7 in the LegLeft at 33.1 m/s (x1.00) — pool 58/100' in today's f030-cortex log. zone_factor returns 0.0 for HitZone::Cortex by rule, so no tuning pass can make the wound pool lethal.

**Stubbed — exists, does nothing:**

- Nothing in src/combat/, src/titan/ or src/blades/ is an empty file or a `todo!()` — grep for todo!/unimplemented!/STUB over all three folders returns zero hits. The stubbing in my area is at the LEDGER level, not the code level (see what_is_broken #3). What follows is genuinely absent, not stubbed.
- F-035 / F-036 Lance (ranged weapon, ammo, resupply): does not exist. `grep -rni lance src/` finds exactly one hit, a comment in src/blades/cut.rs:898. No component, no message, no data block.
- F-040 directed grip escape: cannot exist — shared::TitanState has exactly seven variants (Idle, Pursue, Windup, Strike, Recover, Roll, Death) and none of them holds a player. A titan has no way to grab you, so there is nothing to escape from.
- F-038 injuries / F-039 field medic / F-042 finisher camera / F-037 no-friendly-fire: no code anywhere in the three folders.
- F-052 pathfinding with size logic: brain::walk is a straight line plus a crowd flank offset. There is no path, so a titan does not know a street from a wall.
- F-041 combo is HALF built and its own header says so: src/combat/combo.rs lines 13-21 — 'wirkt auf Schaden' yes, 'bricht korrekt ab' yes, 'sichtbar' NO (no HUD reader), 'wirkt auf Gold' NO (no progress reader). Two of four.
- F-043 hit feedback still prints the WRONG QUANTITY. src/hud/hit_mark.rs prints a closing SPEED, and its header explains why: it was written while gear.ron: blades.damage_per_m_s had no reader. It has had one since 2026-08-25 and nobody went back.

**Broken:**

- EVERY CUT-EVIDENCE SCRIPT IN THE PROJECT IS RED, AND 0 OF 7 KINDS DIE. Measured today, one run each: f030-hitbox 8 of 8 asserts failed — all seven kinds survive their nape pass, where docs/QUESTIONS.md Q-047 records '7 of 7, exit 0' on 2026-08-20. Also red: f030-cortex 1/2, f031-damage 1/6, f032-swords 10/11, f034-hitstop 1/2, q030-reach 1/2, game-full 6/24. In game-full the player finishes a whole sortie with 0 kills where the script expects 3, and the mission never leaves Phase 2.
- THE CAUSE IS THE GRAVITY CHANGE, AND IT REACHES MUCH FURTHER THAN THE THREE SCRIPTS NAMED IN docs/NEXT.md 3G. Every cut script warps the player to a fixed height and waits a fixed time before slashing. At gravity -20 a 0.90 s fall from 18.5 m put the blade at the husk's 8.90 m cortex; at -32 it puts him at roughly 5.5 m, which is leg height. The logs say it in one line: every cut in every red script reads 'cut titan N LegLeft' or 'Torso'. Not one Cortex. So this is not seven bugs, it is one number and seven stale calibrations — and it is NOT a code regression.
- THE TEST SUITE STRUCTURALLY CANNOT SEE IT. tests/combat.rs:171 — 'Puts the player at `at_m` with `velocity_m_s` and **no gravity**'. The fixtures place() and hover() set the velocity by hand and switch gravity off, so 54 of 54 combat tests and 38 of 38 titan tests stayed green while the shipped game lost the ability to kill anything. This is CLAUDE.md rule 5's fourth shape exactly: the sweep holds constant the one axis the behaviour depends on.
- THE STAGE LEDGER IS 15 DAYS STALE AND WRONG FOR MOST OF THIS AREA. docs/features.ron (last touched 2026-08-20) says `stage: Unbuilt` for F-031, F-032, F-033, F-041, F-043, F-044, F-051, F-054, F-055, F-057, F-058 and F-059 — every one of which has shipped code and between 3 and 15 named tests (f031_ 6, f032_ 10, f033_ 15, f041_ 3, f044_ 3, f051_ 8, f054_ 1, f055_ 2). docs/STATUS.md was last regenerated 2026-08-12 and its own tally line still reads '222 unbuilt, 15 built, 8 proven'. Nobody can plan a round off this file, and the survey brief's own '224 unbuilt' comes from it.
- NO PICTURE EXISTS FOR MOST OF WHAT IS BUILT. docs/images/ has nothing for F-031, F-032, F-041, F-044, F-051, F-054, F-055, F-057, F-058 or F-059. Whatever the code does, none of those rows can stand above built-but-unseen, and the shape of the last seven rounds suggests several have been treated as done.
- THE HUGE CLASS HAS NEVER BEEN FOUGHT AND CANNOT BE. bellower is refused by the class cap AND is the only one of the eight kinds that appears in no wave anywhere in assets/data/missions.ron (husk 20, errant 8, scuttler 8, chorus 5, lurker 3, weaver 3, warden 2, bellower 0).
- A NAIVE RETIME DOES NOT FIX f030-hitbox. I tried the same one-number trick on it (wait 0.85 -> 0.67): still 8 of 8 failed, only the warden produced a Cortex. The seven kinds sit at seven different cortex heights and two different warp altitudes, so that script needs a per-kind recompute — fall_time = sqrt(2*(warp_y - cortex_y)/32) — not a sed.


### THE WORLD — src/world/, src/render/, assets/data/maps.ron, the model pack, the anchor field

**The one thing most blocking the player:** "The world is not what blocks the player — it is the most finished thing in this repository, and it is finished ahead of everything that would let anyone feel it. 2901 blocks, 831 dressed models, 8108 validated anchor points, tuned sun/sky/fog and 4.2 ms/frame, and the single mechanism the whole district exists to serve does not read any of it: the hook fires at a raycast, so the 8108-point anchor field changes nothing about where a rope goes (F-024 Unbuilt, src/hud/anchor_marks.rs header). The closest thing to a world-side blocker is second-order and cheap: STATUS.md/features.ron record this domain as almost entirely unbuilt, so every planning round starts from a picture of the world that is eighteen days and 2822 blocks out of date."


**Works, with evidence:**

- Ashgate builds and is big. Live log, today, current binary: `map "Ashgate": 2901 blocks built (245 placed, 2656 generated), 2901 of them anchorable`. 700x700 m, seed 3405691582. Only two maps exist in maps.ron: `ashgate` and `graybox` (400x400, a test fixture; every mission in missions.ron names `ashgate`).
- The district is dressed where a model existed. Live model-spawn log, 831 instances of 25 kinds: 278 houses (221 house_town, 57 house_large), 376 ruin pieces (8 kinds), 134 rubble pieces (6 kinds), 42 hub props (11 market_stall, 7 lamp_post, 7 gas_drum, 5 sentry, 5 crate_small, 3 signpost, 2 hand_cart, 2 banner_long), 1 blade. Picture: docs/images/f003-ashgate.png reads as a red-roofed half-timbered town with rubble, not a box field.
- The anchor field is real and validated. Live log: `anchors Ashgate: 8108 points over 2901 blocks in 3466 us (477 filled cells, buried 0 clustered 0 bad-normal 0 holes 7)`. Column grid at FIELD_CELL_M, not a cubic grid (rule 6). src/world/anchor.rs, 787 lines.
- Lighting, sky and fog are built and tuned with measurements behind every number. art.ron: sun 52000 lux at az 108 / el 36, shadows on, 4 cascades, 2048 texels, 400 m; ambient 2400 sky-blue; sky dome 3 stops, radius 820 m; fog 60..470 m linear. Seen: docs/images/f003-sky-fog-after.png — the gantry lane fades into haze over ~250 m and the shadows are directional and strong. Two interior lamps light the HQ hall (`map "Ashgate": 2 interior lamps lit`).
- Four supply stations run: `F-019: 4 supply stations on Ashgate — 3 reloads each, 1.5 s per reload, 10000 gas/s, 6.0 m reach`. src/world/supply.rs writes only RefuelRequest/BladeRestockRequest.
- Tests green, measured today: tests/world.rs 51/51, tests/render.rs 53/53, tests/data.rs 56/56. Run against the 2026-08-26 20:27 test binaries reading TODAY's assets, so the gravity -20 -> -32 change is included and none of these 160 tests notice it. The f156 hub-yard test that was red on 2026-08-26 is green now.
- Performance is nowhere near a problem yet. DBT_FRAMETIME=1, --offscreen, debug build, f003-ashgate, four windows: 4.667 / 4.225 / 4.225 / 4.224 ms/frame = 237 fps against a 16.7 ms budget.
- The muster yard exists as data and is visible: docs/images/f177-hub.png (taken today by the F-177 reader) shows two handcarts, a lantern, a signpost and a stall canopy on the yard. maps.ron carries it with four written placement rules and tests/world.rs::f156_every_deployment_pad_can_be_walked_to_from_the_hub_spawn_in_a_straight_line holds them down.

**Stubbed — exists, does nothing:**

- SpatialIndex::cast_ray and SpatialIndex::aabb_overlaps (src/shared/spatial.rs:230 and :237) have stub bodies — `RayResult::default()` and `out.clear()`. Nothing calls either; avian's SpatialQuery took the ray job. src/world/index.rs says so in its own header. The maintained half (body/insert/remove via an observer) is real and load-bearing.
- The entire anchor system is decoration. AnchorField's only consumer outside src/world/ is src/hud/anchor_marks.rs, which draws rings. vector::hook fires at vector::aim's raycast and has never heard of the field. F-024 (Snap on Q/E) is Unbuilt — the module header says it, and the Q/E letters were removed on 2026-08-26 because they were a lie. 8108 authored, validated, spatially indexed points decide nothing about where a rope goes.
- T-020 streaming, T-021 object pooling, T-022 LOD pipeline, T-023 material consolidation, T-024 profiling tools: all ⬜ and all genuinely absent. src/render/mod.rs::build_block_meshes makes one Cuboid mesh AND one StandardMaterial per block — 2901 of each, for 8 palette colours and ~80 distinct size classes. The design bible asks for one atlas and minimal draw calls (docs/gameplay/world.md); the atlases in assets/texturen/ reach the game only through .glb files, never through a placed block.
- house_small is registered in art.ron and spawns 0 times. The generator only ever asks for house_town and house_large.

**Broken:**

- docs/STATUS.md and docs/features.ron are STALE for this whole domain, and that is why the project feels like it is going in circles. features.ron:8 still records F-003 as `Built` with the evidence string `79 blocks from maps.ron (9 placed, 70 seeded, 63 taggable)` — dated 2026-08-09. The real map is 2901 blocks, 245 placed. Recorded `Unbuilt` while actually built AND tested green today: F-019 (4 stations running, 4 f019_ tests), F-021 / F-022 / F-023 / F-031a (8108 points, 6 tests), T-036a (the column grid, 477 cells). F-156 is recorded `Unbuilt` in the squad domain while the muster yard landed under that ID on 2026-08-26. A supervisor reading STATUS.md sees an empty world domain and re-plans work that exists.
- F-031a's validation gate excludes the one defect class it is named for. src/world/anchor.rs:222 — `is_clean()` checks points/buried/clustered/bad_normal and NOT `holes`, while holes is documented three lines above as *F-031a's Loecher in der Abdeckung*. The shipped district reports `holes 7` and passes tests/world.rs::f031a_the_shipped_district_passes_its_own_validation_report as clean. Seven columns hold a block and offer no anchor point, and the acceptance criterion is 'no release of a map without an error-free report'.
- 203 of the 245 hand-placed blocks are bare untextured cuboids, and they are the entire silhouette of Ashgate: the 120 m wall (bands 700 / 336 / 285 m wide, 15 m tall), both gatehouses (20x120x55), the gantry beams and columns, the bridges, the bell towers (8x35x8), the church, the trees, and the garrison headquarters you walk into. FIND-134 §2 measured why and it is structural, not laziness: the pack's wall vocabulary (a-095/a-096/a-101) is a tile set authored at one 11.20 m module, render::model::fit_to_class scales uniformly and cannot repeat a tile along a band, and 700/11.2 = 62.5 does not even divide. Visible in docs/images/f177-hub.png — the HQ is three flat grey slabs — and in f003-ashgate.png, where the wall is featureless bands.
- The ground is honestly flat. maps.ron terrain: cell_m 42, step_m 1.5, levels 6 = 7.5 m of relief over 42 m cells = a 3.6 % grade, against houses scale.ron allows 11.5 m of. FIND-134 §3 pinned it in a test rather than fixing it, because step_m/levels are the user's numbers. The largest readable ground feature is 13 % of one roof; from the air the district is houses on a pale sand plane.
- scripts/f003-ashgate.txt is 7 of 31 asserts red at --ticks 1400 and 21 of 101 instructions never run — it needs more ticks AND its bands were never re-aimed after FIND-172's drive change. Every failure is a Speed or Height band (line 88 Speed 4.267 vs <3, line 102 Height 44.363 vs <33.3, line 161 Height 105.000 vs >110), i.e. movement, not map. The map's own evidence script cannot currently be used to prove anything about the map.
- scripts/f070-hub.txt is 15 of 32 red at --ticks 2600 and its FIRST assert fails: `line 110: assert phase == 5 — measured 0.000`. The headless run does not boot into the hub at all any more; it was 42 asserts exit 0 on 2026-08-26. ⚠️ Measured against the 15:00 binary, which was built from the dirty tree (src/menu/pause.rs, src/hud/*, src/player/* are uncommitted debris), so this may be debris and not a landed regression — but it means the hub is not enterable in a script right now, which is how the hub is normally checked.


### The game loop and meta — src/mission/, src/menu/, src/progress/, src/save/, src/squad/, src/net/, src/sound/, assets/data/missions.ron

**The one thing most blocking the player:** The ring closes and **nothing on the other side of it changes**. A player wins a sortie, gets a verdict word, a kill count and a clock, and is put back in the same hub in front of the same six pads — in total silence (`sound` is an empty plugin, zero audio assets), with the XP he earned written only to a file he cannot see (`DebriefLedger` is an empty `Node`), with a gear budget nothing can spend (`Profile.gear` is `{}` and no code writes it), and with no door locked or opened (`progress.ron: gates` is `{}`, `may_fly` is called only by tests). The evidence is in the save file: 321 sorties, and `cleared` has never contained anything above `recruit`. Nothing in this area is crashing — what is missing is any reason to fly the second sortie.


**Works, with evidence:**

- THE RING CLOSES, measured today against the 15:00 binary, no rebuild: `scripts/f072-breach.txt` = **14 asserts held** and walks hub(phase 5) -> pad -> Active(2) -> gate falls -> Lost(4) -> Debrief(6) -> Hub(5); `scripts/f185-parcours.txt` = **11 asserts held** and walks hub(5) -> Active(2) -> five rings in order -> Won(3) -> Debrief(6) -> Hub(5). Both a win and a loss make it all the way round. The ring is proven by the two NEWEST modes, not by `f175-loop`.
- The hub as furniture: `scripts/f070-hub.txt` held its first 19 asserts today; all 7 of its failures start at line 229 and are downstream of one scripted fall-cut. Pads deploy, refuel stations refill, blade racks restock. Guards: `tests/mission.rs::f072_gas_comes_back_at_a_station_and_nowhere_else`, `::f033_a_player_at_a_rack_of_the_hub_walks_away_restocked`.
- Seven mission phases with append-only codes (Briefing 0 … Debrief 6), 4 unit tests in `src/mission/phase.rs` including one that pins Lost=4 / Hub=5 / Debrief=6 because scripts compare against the numbers.
- The menu ring in-app: title -> Play -> hub, pause -> Abandon / Quit-to-lobby / Settings, lobby built out of `missions.ron` and deploying via the same `DeployRequest` the pads use, debrief with Redeploy + To-the-lobby. 43 tests in `tests/menu.rs` (`f175_the_lobby_deploys_the_sortie_it_shows`, `f175_the_debrief_is_a_screen_and_it_waits_for_the_player`, `f175_redeploy_flies_the_same_sortie_again_and_through_the_hub`). Images: `docs/images/f175-lobby.png`, `f175-pause.png`, `f175-settings.png`.
- Save: one profile per player, atomic write, schema 2 with a 1->2 migration, an unreadable file kept rather than overwritten, a future-schema file refused. 8 tests in `tests/save.rs`. Live artefact right now: `saves/player-1.ron` reads `schema: 2, sorties_flown: 321, sorties_won: 295, titans_felled: 634, xp: 64741`.
- Progression math: XP curve, level, skill/gear points, E–S ranks, gear budget with diminishing returns and couplings. 16 tests in `tests/progress.rs`, including `f120_a_sortie_moves_the_career_and_the_rank_in_the_running_app` and `f122_four_builds_with_four_different_leading_axes_are_within_ten_percent`.
- Net as an input transport: 33-byte frame, one seat per source address, the sender's claimed `PlayerId` thrown away. `tests/multiplayer.rs::net_a_peer_on_a_real_socket_drives_his_own_body` and `::net_a_hostile_datagram_does_not_take_the_game_down`.
- Five mission templates with four distinct objective kinds — Cull (tutorial, skirmish), Breach, Parcours, Escort — and three difficulty tiers on four of them, all as RON, `mission::run::resolve` the one fork in code. `tests/mission.rs::f065_every_wave_of_every_difficulty_asks_for_a_kind_that_may_spawn` keeps the file honest.

**Stubbed — exists, does nothing:**

- `src/sound/mod.rs` — 24 lines, `Plugin::build` is empty. `find assets -name '*.wav' -o -name '*.ogg' -o -name '*.mp3'` returns NOTHING. **The game has no sound at all**, and the `audio` feature only links on machine B.
- `src/squad/mod.rs` — 34 lines, `build` empty. No downed state, no revive, no mark. Two of its four bible rules are kept in other domains; the two that are its own are unbuilt.
- `src/net/` — input only, nothing is sent back. A peer drives a body in the host's process and cannot see it. Two copies of this game are not a co-op session, and the lobby's Host row opens a port nothing can meaningfully join.
- `menu::debrief::DebriefLedger` — a named, documented, **empty `Node`**. The debrief shows verdict, mission name, kills and clock; it shows no XP, no level, no rank. The career is invisible to the player at the one moment it changes.
- `progress.ron: gates: {}` is empty and `progress::career::may_fly` is called only from `tests/progress.rs`. F-121's gating exists as a function nothing calls.
- `Profile.gear` is `{}` and **nothing in `src/` ever writes it**; `gear::is_legal` / `strength_of` / `effect_of` are called only from tests. 321 sorties have earned a budget with no screen to spend it in.
- A mission's `map:` field is decorative. `mission::deploy` only *warns* if the template names a map that is not `maps.ron: current`; the world always builds `current` = "ashgate". Two maps exist (graybox, ashgate) and all five templates say "ashgate" — a mission cannot change the map.
- `PlayerSettings` (mouse, FOV, aim assist) are not persisted at all — documented as deliberate in `src/save/mod.rs`, blocked on Q-038.

**Broken:**

- 🔴 **THE PROGRESS MAP IS 7+ DAYS STALE AND THAT IS WHY THE PROJECT FEELS CIRCULAR.** `docs/features.ron` — the generator source for STATUS.md and TODO.md — was last committed 2026-08-20 (`54fd93b`) and still carries `stage: Unbuilt` for F-072, F-073, F-120, F-121, F-122, F-185, F-200 and F-201. Every one of those has code, tests, and for F-072/F-185 a script that is **green today**. `docs/STATUS.md`'s own tally reads `222 ⬜ · 15 🟨 · 8 🟧 · 0 ✅` and is wrong for at least 8 rows in this area alone. Rounds are being planned against a map that says the last two weeks did not happen.
- 3 of the 5 ring scripts are RED today (my runs, 15:00 binary, unmodified tree): `f175-loop` **10 of 19 failed**; `f070-hub` **7 failed of 26 checked** and additionally cut off — "35 of 101 instructions never ran" at `--ticks 2200`, so its header's tick count is wrong; `f073-escort` **5 of 12 failed**.
- But the diagnosis is narrower than §3G reads: in `f175-loop` and `f070-hub` **every** failure is downstream of one scripted fall-cut that no longer lands. Phase reads `2` (Active) at every station because nothing ever won the sortie. The hub, the pad deploy and the second deploy (`MARK t=1153 f175-round-two`) all still happen. Nothing about the loop is failing there — one `warp`+`slash` pass is.
- `f073-escort` is NOT in §3G's or B-012's gravity table and is un-triaged: `line 322: assert Phase == 2 — measured 5.000`. Phase 5 is Hub, so the escort sortie **ends early and goes home** before a third of the script has run.
- Every headless evidence run books a sortie into the repo's shared `saves/player-1.ron` unless `DBT_SAVE_DIR` is set — and only `scripts/f120-career.txt` sets it. B-012 lists this as an unexcluded flake cause; my three runs today moved the counter to 321.
- That same file is the honest record of what has ever been beaten: `cleared: ["escort/recruit", "parcours/recruit", "skirmish/recruit", "tutorial"]`. **In 321 sorties nobody has ever won a Veteran or an Elite, and no breach has ever been survived.** The difficulty ladder is written, tuned and never exercised.
- No screenshot exists for the debrief, the title screen, breach, escort, parcours or the career — `docs/images/` has only `f070-*`, `f071-*`, `f175-lobby/pause/settings` and `f177-hub*`. By the project's own three-piece 🟧 rule, all six of those are 🟨 at most, whatever a script says.
- F-080 asks for five difficulty levels; there are three, and `tests/data.rs::t005` pins the keys to exactly `["recruit","veteran","elite"]`. `data::DifficultyLevel` has **no HP field at all**, so "scaling HP" is a `src/data/mod.rs` change, not file work.
- Doc mismatch: `src/net/mod.rs` says the wire frame is "**37 bytes**"; `wire::FRAME_BYTES` is `1+4+8+4*4+4` = **33**, and `wire.rs`'s own diagram says 33.
- Uncommitted debris touching this area: `src/hud/hub_prompt.rs` is **untracked but already wired** into `src/hud/mod.rs` (module, `HubLanded` resource, 4 systems) — a live feature no commit in the history says exists. `src/menu/pause.rs` (+13) and `tests/menu.rs` (+48) are modified alongside it.


### WHAT THE GAME IS SUPPOSED TO BE — design bible (docs/gameplay/), docs/backlog/, docs/features.ron, docs/ROADMAP.md, docs/PLAN-GAME.md

**The one thing most blocking the player:** "The movement has nowhere to prove itself and no mode that asks it to — and neither of those two things is in the queue, because neither has an F-ID. The P1 gate is 'a blind test against Attack on Titan Revolution with ten testers, our movement rated at least level with it'. The instrument the design specifies for that is F-077 Traversal Trial, which depends_on F-014 Momentum-Chaining ('an experienced player can INCREASE speed over 5 hook changes instead of losing it') — F-014 is Unbuilt and is literally the gate criterion written as a feature. The place it would run is M-002 Ashgate District, 30 pd, Must, status Offen — and maps have no F-ID at all, so M-002 has never once appeared in docs/TODO.md or docs/STATUS.md. So the supervisor reads a queue in which the two items that actually unblock the game's only gate are invisible, and the visible prio1 rows nearest the top are HUD and settings work that PLAN-GAME.md §10 already deferred. That is the mechanism of the circling, not a lack of discipline."


**Works, with evidence:**

- THE DESIGN ITSELF IS SETTLED AND COHERENT. /home/offlinebot/Documents/defeated-by-titan/docs/gameplay/ is 5 files, 82 kB, all cross-referenced, no contradictions found between pillars.md, core-loop.md, world.md and enemies.md. The game in one sentence (pillars.md): 'A movement game with a high mastery ceiling, in which fighting is the side effect of good movement.' with the cut rule attached: 'A feature that contradicts that sentence gets cut, no matter how much work already went into it.' This is not the problem area.
- F-030 cortex kill — ledger stage Proven. Evidence in docs/STATUS.md: hits at 8/30/75 m/s = 3 of 3, 9.0 us per cast over 1000 casts, red-checked three ways, docs/images/f030-cortex.png sha256 951aff7b twice [debian]; cut landed out of flight in scripts/f-flight-cut.txt, 25 asserts, exit 0.
- F-034 hit-stop — Proven. Position bit-identical for exactly 7 ticks = round(0.12*60) with a taut rope, 0 of 21 fresh apps dead after the fix, 7 of 21 dead when the line is removed. docs/images/f034-hitstop.png.
- F-050 + F-053 titan FSM and telegraphed attacks, F-070 mission FSM, F-170 HUD layout, F-171 dynamic crosshair — all Proven in docs/features.ron. docs/gameplay/core-loop.md records that scripts/game-full.txt reaches MISSION WON end to end.
- Seven of the eight titan kinds fight differently — docs/gameplay/enemies.md 2026-08-19, measured: the weaver's roll is 27 ticks, 9 of them open, 3.90 m of retreat; secondary hit zones report ArmLeft/LegLeft on the real husk via scripts/f032-swords.txt. NOTE: docs/features.ron still marks F-057..F-063 and F-032 as Unbuilt.
- Five movement verbs landed 2026-08-24 — docs/QUESTIONS.md Q-052 names F-008, F-009, F-010, F-017, F-019 as built (dash magazine: dodge_charges 3.0, dodge_recharge_s 4.0, dodge_cooldown_s 0.6). NOTE: docs/features.ron still marks all five Unbuilt.

**Stubbed — exists, does nothing:**

- ALL 12 MAPS. docs/backlog/maps.ron: M-001 The Rookery (Hub, Must, 25 pd) .. M-012 Trade Hall — every single one status 'Offen'. And there is NO F-ID for any map anywhere in docs/features.ron: the `world` domain holds 12 rows and all 12 are Ankersystem. M-002 Ashgate District is 30 person-days of Must work that never appears in docs/STATUS.md or docs/TODO.md, so it is invisible to the queue the supervisor reads.
- THE HUB IS BEING BUILT WITH NO BACKLOG ROW. scripts/f070-hub.txt, scripts/f177-door.txt and the uncommitted src/hud/hub_prompt.rs implement M-001 The Rookery, which is a maps.ron entry with no F-ID, no stage and no acceptance criterion. F-177 in features.ron is 'Grafikeinstellungen' (graphics settings) — the f177-* work is not that feature.
- F-077 Modus: Traversal Trial. pillars.md P1: 'the Traversal Trial is not a side mode, it is the litmus test of the whole project ... it reuses existing maps and costs almost nothing.' The spreadsheet gives it prio2 (Should), 9 pd, depends_on F-014 Momentum-Chaining, which is Unbuilt. The design's own gate instrument is not scheduled.
- docs/PLAN-GAME.md — 'the build order for a playable game', 60 kB, dated 2026-08-09 and never revised. Its §2 ground-truth table is 18 days stale (claims 238 unbuilt, TitanPlugin empty, Health does not exist anywhere). Its §1 bar for 'playable' — one command, hook, swing, box titan, amber cortex, hit stop, 0/3 counter, WON/LOST — has largely been reached, and NO successor plan document exists. This is the plan vacuum the circling happens in.
- The whole of docs/gameplay/ carries Stage 🟨 in its own headers. pillars.md: 'none of these numbers has been measured in this project — there are no players yet.' All thirteen success metrics (D1 retention >35 %, TTK variance >2.5x, first-mission completion >80 %) have no instrument behind them.
- P5 (the store sells appearance only) has no mechanism off Roblox — pillars.md says the principle stands but 'None of it gets built' (Q-001). F-225..F-229 monetization, 5 rows, correctly untouched.

**Broken:**

- 🔴 THE STAGE LEDGER IS WRONG IN BOTH DIRECTIONS AND EVERY BUILD DECISION IS BEING MADE FROM IT. docs/features.ron marks 222 of 245 rows Unbuilt. Measured: 54 of those Unbuilt-marked F-IDs carry a named test function (fNNN_...) in tests/ or src/, and 20 of them carry an evidence script in scripts/ — F-008, F-016, F-019, F-023, F-024, F-025, F-026, F-028, F-029, F-031, F-032, F-044, F-051, F-072, F-073, F-120, F-175, F-176, F-177, F-185. (A test alone is not proof of implementation — tests/titan.rs::f064_the_bellower_stays_blocked_until_the_ear_exists asserts a feature is ABSENT. That is the point: nobody can tell built from asserted-absent without opening each one.)
- 🔴 docs/STATUS.md and docs/TODO.md were last generated 2026-08-12 18:42; docs/features.ron was last written 2026-08-20 17:37. tools/features.py has not been run in 15 days. The generated queue is behind the ledger, and the ledger is behind the tree.
- ZERO features are user-accepted. docs/STATUS.md tally line reads '222 ⬜ · 15 🟨 · 8 🟧 · 0 ✅ of 245'. The survey brief's '2 user-confirmed' is a raw glyph count of the legend, not of rows. Nothing in this game has ever been signed off.
- 🔴 GATE VIOLATION, MEASURED. docs/PLAN-GAME.md §10 lists all of progress/ under 'Forbidden by the gate — not deferred by us', naming F-120 levels and XP explicitly. tests carry f120_, f121_, f122_ and scripts/f120-* exists. Progression was started behind the Vector Gear gate that ROADMAP.md calls 'not ours to relax'.
- 🔴 THE LAST ROUNDS WENT INTO THE ROWS THE PROJECT'S OWN PLAN DEFERS. F-175 Menuestruktur, F-176 Barrierefreiheit (prio2), F-177 Grafikeinstellungen — docs/PLAN-GAME.md §10 lists exactly 'hud F-172, F-175 (beyond one pause screen), F-176, F-177, F-178' as deferred. Meanwhile the P1 critical path is 8 prio1 Vector Gear rows and 10 prio1 Ankersystem rows, untouched or half-touched. That is the circle the user named.
- 🔴 THE MAP CONTRADICTS THE SETTING. world.md: 'The central difference to the source material: the war is already lost. Ashgate has long since fallen; the Vanguard runs salvage missions into its own ruins.' What is built is an intact, inhabited, tidy walled town — docs/NEXT.md §1F, the user 2026-08-18: 'weil aktuell ist das nicht die echte map!' Zero hits for ruin/rubble/collapse in assets/data/maps.ron; 14 ruin and rubble models ship in the pack and none is used.
- ANCHOR DENSITY HAS NO NUMBER. prompts/init.md called it 'die wichtigste Zahl' and ROADMAP P3's gate is 'traversal times show a measurable difference between beginner and expert'. docs/backlog/maps.ron carries only Hoch/Mittel/Niedrig/Sehr hoch. Q-010, open since the start, still running under an ASSUMPTION.
- P4 READABILITY IS TWO DIFFERENT NUMBERS. pillars.md: 'The cortex is recognizable from 100 m.' F-030's own acceptance in backlog units converts to 28 m. Factor 3.6 in pixels — 36.7 px at 28 m vs 10.3 px at 100 m at 1920x1080. Q-019/Q-026, unanswered, and it decides cortex size, which decides whether the fight is a positioning problem or a clicking problem.
- THE DAMAGE CURVE IS TUNED AGAINST AN ARTEFACT. core-loop.md rule 3 is 'damage comes out of speed', but F-031 is Unbuilt because there is no titan health to be a percentage of (PLAN-GAME §9.1), and F-030's kill lands at 74.70 m/s = vector.max_speed_m_s, i.e. the clamp, not a chosen speed. core-loop.md flags this itself: 'A damage curve tuned against a clamped input is tuned against an artefact.'
- THE BELLOWER'S ENTIRE DESIGN HAS NO COUNTERPLAY. enemies.md: he reacts to the sound of gas, and the counterplay is 'play quietly'. F-051 Wahrnehmungsmodell (the ear) does not exist, and he is class huge against scale.ron max_spawnable_class large (Q-028). The stealth layer that enemies.md calls the reference's biggest gap is one of eight kinds and it is blocked on one prio2 row.


### Debt and blockers — docs/QUESTIONS.md (61 unique Q-ids), docs/BUGS.md (7 B-entries), the 68-script corpus, docs/NEXT.md §3A–§3G

**The one thing most blocking the player:** "The hub. He launches the game and stands facing away from every door that starts a mission — 150.7°, 180.0°, 209.3° against a 45.7° half-FOV, with 0.66% of the frame as the only hint the place has doors at all. The fix is ONE facing value (§3E option 1, and `missions.ron:57` already asserts the outcome), and it has been deliberately withheld for a hub-line round that has now been refuted four times and is STILL uncommitted. So the player cannot enter his own game without knowing to turn 180°, and the cheap fix is being blocked by the expensive one. Runner-up, and it is close: holding Ctrl with two ropes drags you to 0.000 m/s on 64.4% of geometries (FIND-191) — a movement-killer in a game that is entirely movement — and it is tracked in no queue file."


**Works, with evidence:**

- The uncommitted tree BUILDS and the built binary already contains it. `target/debug/defeated_by_titan` mtime 08-27 15:00 is NEWER than every modified source (`/home/offlinebot/Documents/defeated-by-titan/src/player/rope.rs` 14:59, `src/hud/hub_prompt.rs` 14:37, `assets/data/game.ron` 02:40). The briefing calls this "unfinished debris" — it is not. It is a landed, evidenced, uncommitted round: 13 modified files, +3044/-316, of which 944 lines are new tests in tests/vector_rope.rs and 883 in tests/hud.rs.
- F-177's hub line is WIRED, not orphaned. `src/hud/mod.rs:166` has `pub mod hub_prompt;` plus five system registrations at :188–216. Evidence is a control pair: with `hub_prompt::update_hub_prompt` unregistered, 2949 amber px in the banner band become 0, the two frames differ in 5536 px and in nothing outside x[409..870] y[23..80] (`docs/images/f177-hub-control.png`, `f177-line-front.png`, `f177-line-left.png`).
- Q-058's DistanceJoint (rope attempt 3) passes its own acceptance matrix WITHOUT Ctrl: four 288-cell matrices (4 anchor separations x 2 elevations x 4 yaws x 9 key combos x 90 ticks), worst per-arm excess +0.0050 m air and ground, 0 infeasible ticks. Against the rollback (`commands.spawn(rope)` back in the Drive arm of `player::rope::attach_ropes`) the same matrices read +51.1978 m / +50.0737 m — four orders of magnitude, so it is a guard and not a tolerance (docs/FINDINGS.md FIND-195).
- Six scripts carry a self-claimed verdict with a tick count and exit 0: f004-towers (31 asserts, 838 ticks), f-flight-cut (25 asserts, 363 ticks), game-full (24 asserts, 1200 ticks), f170-objective (4 asserts, 457 ticks), p5-downed (5 asserts, 934 ticks), q030-reach (2 asserts).
- Two of seven bugs are FIXED with a date, a mechanism and a named cause: B-010 (2026-08-19, `vector::aim::cast` was casting on mask ALL from inside a team mate's capsule) and B-011 (2026-08-26, the Q/E letters withdrawn from `src/hud/anchor_marks.rs`).

**Stubbed — exists, does nothing:**

- F-024 — the feature that makes the hook actually FIRE at the anchor-field candidate. `src/hud/anchor_marks.rs` draws the field; `grep AnchorField src/` outside `src/world/` finds only that HUD file. B-011 withdrew the Q/E letters but records "F-024 still owes the snap". The field is decoration.
- The Vector Gear gate — the exit condition for all 224 unbuilt features. `docs/gameplay/pillars.md:26-30` defines it as a blind test against Attack on Titan Revolution with ten testers and then states outright: "An agent cannot satisfy this gate." It has never been run and cannot be run by anything in this repository. Every meta feature is parked behind a door that has no handle.
- Four of seven bugs are stage-yellow: B-006, B-007, B-008, B-009 are all marked "found by reading the code, NOT yet reproduced in the running game". Each carries a written repro recipe that nobody has executed. B-009's repro is four script lines sitting in the entry; `scripts/b008-down.txt` exists as B-008's repro and B-008 is still unreproduced.
- docs/QUESTIONS.md:1124 `## Q-0nn — <the question, in one line>` is a blank template sitting inside the live entry list, so it is counted as an entry by every grep of the file.
- Q-024 (German or English in the source) and Q-009 (does offscreen rendering work on machine A) are open on paper but dead in fact — CLAUDE.md rule 2 settled the first, and `docs/images/f177-line-front.png` settled the second. They inflate the open count.

**Broken:**

- 🔴 UNTRACKED DEFECT — the reel is geometrically impossible with two ropes and it is filed NOWHERE. `player::rope::shorten_ropes` takes `reel_speed_m_s`=28 off EACH arm's `limits.max`, so with two anchors 56 m apart both maxima reach `min_rope_m`=3.0 within a second. avian abandons one arm: player at 0.000 m/s, left rope 50.167 m past its own maximum. Infeasible on 16704/25920 ticks (64.4%) ground, 64.5% air. It predates the joint — Pendulum agrees to three decimals. This is FIND-191/FIND-195 and `grep -n 'FIND-191\|FIND-195' docs/QUESTIONS.md docs/BUGS.md docs/NEXT.md` returns NOTHING. It lives only in the 12000-line file nobody opens whole.
- Gravity −20 → −32 (commit 1ca7d26, made while a round was live): f175-loop 10 of 19 red, f070-hub 16 of 42 red, f177-door 5 of 13 red. Known, deliberate, §3G defers the re-aim until the joint lands. Not a finding.
- scripts/w5-lane.txt has been 19 of 51 RED since 2026-08-19 and unrepaired, and it is NOT marked red-on-purpose. Every `look` line is hand-compensated ±28° for an `aim_spread_deg` that stopped being a fixed offset on 2026-08-18 (FIND-096). ACT A leg 3 measures 1.202 m/s where its own table says 39.250. §2D names the cheap fix (route 2: aim with `settings assist_strength 100` instead of by angle) and it was never executed — so the user's "random türme die nicht so sein sollten" is still unanswered, for a reason that has nothing to do with towers.
- §3E — you spawn with your back to every door. Camera faces −Z; all three skirmish pads sit at +Z at bearings 150.7° / 180.0° / 209.3°. `assets/data/missions.ron:57` claims recruit "stands straight ahead of the spawn point". The only pad on screen is one corner of `parcours/recruit` at 51.3° against a 45.7° half-FOV — 6120 px, 0.66% of the frame. No test has ever compared a pad bearing to the spawn facing. The one-line fix is deliberately withheld until F-177 closes.
- B-012's ORIGINAL observation is still unexplained and must not be retired. f175-loop reported 11 of 19 failed twice at 23:15 UTC with MARK ticks identical to the green runs; `game.ron` was written at 02:40, i.e. afterwards. Two causes of one symptom, one known and one open. The entry says in bold: "Do not close this entry when the corpus is re-aimed."
- Q-061 — scripts/f176-pull.txt ACT 2 `assert speed < 2.0` contradicts docs/NEXT.md §3D R1. Left red ON PURPOSE with a header saying so. It is the only script in the corpus with an explicit red-on-purpose marker.
- Six evidence images are stale and still cited: `docs/images/f170-hud.png` (shows the 82% bar of the 300-gas era), plus the image halves of f-018-gas, f-007-boost, f070-lost, q030-reach, and f-001-hooks (graybox hash, explicitly "void"). Two more — f177-hub.png and f177-hub-turned.png — photograph the promise/pointer element that FIND-193 deleted; src/hud/mod.rs already annotates them as "not evidence for anything shipped".
- 51 of 61 unique Q-ids are genuinely OPEN. Q-001..Q-035 were opened 2026-08-09..08-13 and have therefore run under an unconfirmed ASSUMPTION for 14–18 days — including Q-002 (0.28 m/stud, the conversion under every number in the game), Q-029 (26 invented numbers in titan.ron/gear.ron/scale.ron), Q-035 (200 m hook range against his own 90 m), Q-013 (max rope length), Q-017/Q-037 (gas priority order).
