# PLAN-GAME — the build order for a playable game

Updated: 2026-08-09 · Stage: 🟨 (a plan, not a measurement)

One document, executable round by round. Built out of the three surveys
(`plan-titan.md`, `plan-combat.md`, `plan-game.md`) plus my own verification of the tree as it
stands at `ca2a51e` with the vector round still uncommitted in the working copy.

Read-only survey; nothing in the project was touched. Everything I checked myself carries a
`file:line`; everything I took from a survey says which one.

---

## 0. How to read this

- **Round 0** is the main head alone. It is not optional and it is not parallel: it writes the
  seam every later agent depends on. Skipping it converts the fan-out into integration work.
- **Rounds 1–3** each run **four agents**, with a named file owner and a named acceptance
  criterion. **Round 4** is the main head again: integrate, measure, set stages, commit.
- Every round ends with a **gate**. The gate is a command with an exit code, not a feeling.
- `docs/features.ron`, `docs/STATUS.md`, `docs/TODO.md`, `Cargo.toml`, `src/main.rs`,
  `src/lib.rs`, `assets/data/*.ron` stay with the main head throughout (CLAUDE.md). Agents
  **report** the line they need; they do not write it.

### The measured constraint on parallelism — this changes the shape of the plan

```
$ df -h /home/offlinebot   →  928G total, 844G used, 80G free (92 %)
$ du -sh target/           →  75G
```

**A per-agent `CARGO_TARGET_DIR` is not possible on this machine.** Two of them would not fit
on the free disk. So all four agents share one `target/`, and cargo takes an exclusive lock on
it: a second `cargo` prints *"Blocking waiting for file lock on build directory"* and waits.

Consequence, and it is binding for every job brief:

> **Four agents may write in parallel. Only one may compile at a time.**
> An agent may run `cargo check` and `cargo test --test <its own file>`. It may **not** run a
> bare `cargo test` and it may **not** run a build in a loop. The full `cargo test` is the main
> head's, once, at the round gate.

`cargo clean` is forbidden this session: 75 G would have to be rebuilt against an 80 G margin.
A `--release` build does not fit at all.

---

## 1. What "playable" means here

Eight sentences. This is the bar we clear, not the bar we would like.

A person types **one command** and gets a window in which the mouse pointer is captured and
looking around works. They stand in the grey city, and with the two mouse buttons they can sink
a hook into a wall, swing, reel in and boost, and reach roof height under their own power. Left
of the screen a cyan bar tells them how much gas is left; in the middle a crosshair tells them
whether what they are looking at can be hooked. Somewhere in the city stands a titan of boxes,
ten metres tall, with an amber sphere at the back of its neck; it notices them, walks toward
them without walking through a house, and before every strike it visibly raises an arm for
six tenths of a second. If the player crosses that amber sphere with a blade at speed, **the
world stops for a tenth of a second**, the camera kicks, and the titan collapses and evaporates
— and a counter in the top of the screen goes from `0/3` to `1/3`. If they let the titan hit
them enough times they go down and the screen says **LOST**; if they kill all three before the
clock runs out it says **WON**. `Esc` releases the mouse and gets them out.

That is the whole game. **One mission, one enemy kind, one way to win, two ways to lose,
three minutes.**

What is deliberately *not* in that paragraph, and is not in the bar: any number that is saved,
any second player, any sound worth the name, any menu beyond pause, any model that is not a box.

---

## 2. Ground truth (verified, not assumed)

| | state | evidence |
|---|---|---|
| backlog | 238 ⬜ · 6 🟨 · 1 🟧 · 0 ✅ of 245 | `docs/STATUS.md:14` |
| vector round | **in flight, uncommitted**: `aim.rs` 156 L, `hook.rs` 326 L, `gas.rs` 231 L, `boost.rs` 88 L — **`reel.rs` is still a 24-line stub** (`src/vector/reel.rs:19-24`, "filled in by job E") | `git status --short`, `wc -l` |
| the rope | **an avian `DistanceJoint`, decided and measured** — the hand-written solver is retired | `src/player/integrator.rs:1-20` |
| `TitanPlugin` | `fn build(&self, _app: &mut App) {}` | `src/titan/mod.rs:23` |
| `CombatPlugin`, `BladesPlugin` | same, empty | `src/combat/mod.rs`, `src/blades/mod.rs` |
| `SpawnTitan` | registered (`src/lib.rs:78`), written by the script driver, **no reader** | `src/shared/message.rs:54` |
| `Metric::Titans` | counts `world.titans` — works the moment an entity carries `TitanId` | `src/debug/mod.rs:242` |
| `Health` | **does not exist anywhere in `src/`** | grep |
| UI | `spawn_overlay`/`update_overlay` compile at `src/debug/mod.rs:287-331` and are **registered nowhere** | grep for `spawn_overlay` |
| `IsDefaultUiCamera` | exists, and `DefaultUiCamera::get()` short-circuits on it at `bevy_ui-0.19.0/src/ui_node.rs:2991` **before** the `RenderTarget::Window` filter at `:2997-3003` | read myself |
| `DespawnOnExit<S>` | `bevy_state-0.19.0/src/state_scoped.rs:149` (`StateScoped` is gone) | read myself |
| `SpatialQuery::cast_shape` | `avian3d-0.7.0/src/spatial_query/system_param.rs:446-462`; the struct at `:60-64` carries **no `Without<Sensor>`** | read myself |
| `Collider::capsule_endpoints` / `::sphere` | `.../collision/collider/parry/mod.rs:800` / `:725` | read myself |
| `CustomPositionIntegration` | `.../dynamics/integrator/mod.rs:195`, filtered out of `integrate_positions` at `:504` | read myself |

---

## 3. Round 0 — the seam (main head alone, no fan-out)

Nothing here is a feature. Everything here is a thing four agents would otherwise each invent
differently. **This is the round that decides whether the fan-out produces progress or merge
conflicts.**

### 0.1 Land the vector round

Finish `reel.rs`, `cargo check`, `cargo test`, `python3 tools/normen.py`, commit.

**Gate — and this is the plan's first tripwire:** a `--script` run in which the player hooks,
swings and boosts must hold

```
assert speed > 25
```

at a named tick. `assert speed` already exists (`src/debug/script.rs:63`, `Metric::Speed`). If
that number does not come out, see Risk 1 in §7 — **the rest of this plan is designed for a
player who moves at 30 m/s and is worth much less below it.**

### 0.2 Write the shared seam (`src/shared/`)

Four things, in one commit, because four agents in Round 1 read them:

| what | why it is main-head work |
|---|---|
| **collision layer constants** — `LAYER_WORLD`, `LAYER_PLAYER`, `LAYER_TITAN_BODY`, `LAYER_TITAN_CORTEX` | R1-A puts the cortex on a layer, R1-B filters for it. If they invent the constant separately, the cut silently never lands (Survey B §3). |
| **`Health { current, max }`** as a `Component` in `shared/state.rs` | It has **no F-ID anywhere in the backlog** (Survey C §2). Two agents need it in Round 2. Next to `MovementState::Downed`, which already documents the intent (`src/shared/state.rs:125-127`). |
| **`HitStop { ticks_left: u32 }`** as a `Component` in `shared/` | R2-A gates on it, R2-C's camera reads it. It is a **tick counter, never a clock** — see F-034 below. |
| **`TitanState`** (the FSM enum) as a `Component` in `shared/` | R1-A writes it, R1-C's overlay reads it, R2-D extends it. A `titan`-private enum would force an allow-list edge just so the overlay can print a word. |

`shared/` is not on CLAUDE.md's main-head-only list, but the fan-out rule *is* — "the interface
must stand before the fan-out starts". These four go in Round 0.

### 0.3 Write the missing numbers into RON (main head only)

Surveys A §5 and B §6 between them list ~30 missing values. **Ten of them block Round 1.** All
of them land in **one commit**, every one of them marked `⚠️ UNTUNED` in the file the way
`game.ron` and `gear.ron` already do, so a reversal is a file edit and not a code edit.

**Blocking — Round 1 cannot start without these:**

| file | field | why it blocks |
|---|---|---|
| `scale.ron: titan` | `width_fraction` (one fraction, all classes) | **The biggest single hole.** Nothing in the repo says how wide a titan is. No collider, no rig, no answer to the alley question. Survey A used 0.25 × height as an *estimate*; an estimate in a spawner is two truths waiting to drift. |
| `scale.ron: titan` | `torso_fraction`, `arm_fraction`, `leg_fraction`, `shoulder_height_fraction` | the box rig's proportions. Head fractions are already there (`scale.ron:123-124`); these six are not. |
| `titan.ron` per kind | `turn_deg_per_s` | Without it the husk snaps to face the player and **his entire lesson — "fundamentals of the approach angle", bible §4 — ceases to exist.** The most important feel number in the file. |
| `titan.ron` per kind | `strike_s`, `recover_s` | `windup_s` exists; the other two thirds of an attack do not. `recover_s` **is** the punish window. `animations.ron` AN-085/086/087 give the totals (1.0–1.2 s). |
| `titan.ron` per kind | `attack_range_m`, `attack_cooldown_s`, `aggro_radius_m`, `accel_m_s2`, `death_s` | the FSM has no `Pursue → Windup` edge without a range and no `Idle → Pursue` edge without a radius |
| `titan.ron` per kind | `health` | **there is no titan health anywhere** (Survey B §6.1) — and `regen_per_s` already regenerates it. Not needed for the cortex kill (which is a rule), needed the moment anything else is. |
| `maps.ron: palette` or a new `signals` block | the three signal colours (amber / cyan / crimson) | They exist **only as prose** (bible 3.4, `docs/conventions.md:69-79`) and as a number **nowhere**. There is nothing to paint the cortex with. As data, the rule "they appear nowhere else" also becomes a test. |
| `titan.ron` or `scale.ron` | `windup_arm_deg`, `windup_lean_deg`, `strike_arm_deg` | pose angles are game values (rule 2). "Raise the arm 140°" in Rust never gets tuned. |
| `gear.ron: blades` | `reach_m`, `thickness_m`, `swing_s`, `active_from_s`, `active_to_s`, `cooldown_s` | `reach_m` **decides whether a cut lands at all** and does not exist. Without `active_*` the blade cuts during its own wind-up; without `cooldown_s` it is autofire. |
| `gear.ron: feel` (new block) | `hit_stop_cortex_s`, `hit_stop_normal_s`, `camera_kick_deg`, `camera_kick_s` | F-034 has no numbers at all today |
| `missions.ron` | `kill_target` on the template | F-071 counts to a number that must come from the file |

**Cap the spawnable classes at `large` (14 m)** for this session. At 14 m and a 0.25 width
fraction a titan is 3.5 m wide in a 7.0 m street — 1.75 m clearance per side, and nothing has
to be invented. `huge` (21 m) is tight, `boss` (28 m) is exactly 0.00 m and jams the alley
mouth as a silent wall (Survey A §3). That is a **user decision**, not a tuning call → §3.4.

### 0.4 Four lines into `docs/QUESTIONS.md`

These are not decidable by an agent and they are load-bearing this session:

1. **28 m boss vs. 7 m streets.** Two binding user figures from the same day that are not
   jointly satisfiable in a grid city. The plan's answer for now is "cap at 14 m"; it needs
   confirming.
2. **Q-019 is now on the critical path.** The cortex radius decides whether F-030's "100" is
   met, and it sizes the amber marker. It has sat as an assumption; the titan round is where it
   becomes load-bearing.
3. **Which way does the cortex face?** A 360° sphere makes the titan a floating bullseye and
   deletes the approach-angle skill F-030 exists to create. A rear hemisphere is the design.
   Decides F-060 later.
4. **Does a cortex hit kill by rule or by threshold?** It stands in a doc comment
   (`src/shared/message.rs:21`) and nowhere else. F-031 calibrates against the answer.

Also for `docs/FINDINGS.md`: **F-064 and F-052's acceptance criteria still say 3 m / 7 m / 15 m**
while the user gave five classes in metres. That fix is in `gameplay/features.xlsx` +
`python3 tools/features.py`, which regenerates `STATUS.md` and `TODO.md`. **Do it in Round 0 or
not at all this session** — regenerating those two files mid-session while stages are being set
is pure diff noise.

### 0.5 Extend the script vocabulary (main head, `src/debug/`)

Three new `Metric` variants, because Rounds 2 and 3 write their acceptance criteria in them:
`Kills`, `Phase` (as a number), `Health`. `measure()` is at `src/debug/mod.rs:240-254`,
the parser at `src/debug/script.rs:205-217`. Fifteen lines, and without them the mission
rounds have no instrument.

### Round 0 gate

```bash
cargo check 2>&1 | grep '^error'      # empty
cargo test                            # green, count recorded
python3 tools/normen.py               # clean
cargo run -- --offscreen --script scripts/<vector>.txt --ticks N   # exit 0, assert speed > 25 holds
```

---

## 4. Round 1 — something stands in the world, and the screen works

Four agents. **This is the round the whole plan hangs on**: at the end of it there is a titan,
a cut that reaches it, and a screen that can prove both.

| job | owns (writes) | must not touch | builds |
|---|---|---|---|
| **R1-A `titan`** | `src/titan/mod.rs` + new files under `src/titan/`, new `tests/titan.rs` | everything else | **F-050** (reduced), **F-056**, **F-064** |
| **R1-B `combat`** | `src/combat/mod.rs`, `src/blades/mod.rs` + new files, new `tests/combat.rs` | `src/titan/**` | **F-030** |
| **R1-C `screen`** | `src/render/mod.rs`, `src/debug/mod.rs` | `src/debug/screenshot.rs` | **P1**, **P2** (no F-ID) |
| **R1-D `input`** | `src/net/local.rs`, `src/net/mod.rs`, `src/menu/mod.rs`, `docs/BUGS.md` | `src/lib.rs` | **P3**, **P4** (no F-ID) |

### What each one does

**R1-A — the titan.** Consume `SpawnTitan`; build a **nine-box hierarchy** (pelvis · torso ·
head · **cortex** · 2 arms · 2 legs) with local `Transform`s, cortex parented under the head so
it follows the pose through `GlobalTransform` for free. `RigidBody::Kinematic` +
**`CustomPositionIntegration`** — without that marker the titan moves **twice per tick**,
because kinematic bodies do get a `SolverBody`
(`avian3d-0.7.0/src/dynamics/solver/islands/mod.rs:87`) and `integrate_positions` filters only
`Without<CustomPositionIntegration>` (`.../integrator/mod.rs:504`). The cortex is a **child
`Sensor` sphere on `LAYER_TITAN_CORTEX`** — physically intangible, still hittable, because
`SpatialQuery`'s collider query carries no `Without<Sensor>`
(`spatial_query/system_param.rs:60-64`), unlike `MoveAndSlide`
(`character_controller/move_and_slide.rs:82`). FSM reduced to
`Idle · Pursue · Windup · Strike · Recover · Death` — `Alerted` needs F-051, `Stagger` needs
F-032, both are extra arms of the same enum later. Movement this round is **straight-line
`MoveAndSlide`**; the path comes in Round 2. Consume `TitanHit{Cortex}` → `Death` → scale to
zero over `death_s`, **drop the collider on tick one**.
**Pose is a pure function of `(TitanState, ticks_in_state)` — never `Time`.** `AnimationPlayer`
is available (`bevy_internal-0.19.0/src/default_plugins.rs:85`) and must not be used: it
advances on `Time`, not `Time<Fixed>`, and that breaks the bit-identical `--offscreen`
screenshot the whole evidence route rests on (`docs/ACCEPTANCE.md`, `sha256 = eb212dfe…`).

**R1-B — the cut.** A swing state machine off `Buttons::SLASH_LEFT`/`SLASH_RIGHT`
(`src/shared/intent.rs:66-67`), gated by `active_from_s`/`active_to_s`. **One swept
`cast_shape` per active blade per tick**, capsule from `Collider::capsule_endpoints`, along the
player's displacement of that tick, in `SimulationSystems::PostStep` (avian's `Writeback` has
run, so `Transform`, `Position` and `shared::Velocity` agree — `src/lib.rs:120-131`). **Two
filtered casts, cortex layer first**, because `cast_shape` returns only the closest hit and the
cortex sits *inside* the body silhouette. Fill `TitanHit.speed_m_s` with the **closing** speed
`max(0, (v_player − v_titan) · d̂)`, not `|v|`.

> ⚠️ **R1-B must not copy `aim.rs`'s filter rule.** `src/vector/aim.rs:31-38` casts unfiltered
> and checks the mask *afterwards*, because a filtered ray travels through untagged geometry
> and a hook through a wall is a bug. **Combat is the opposite case**: the cortex is
> deliberately hidden inside the body and the layer filter is the entire point. Same crate,
> opposite rule — state it in the brief or an agent will "fix" it.

R1-B's tests build **their own minimal fixture** (one `Collider::sphere` on the cortex layer at
a known position) rather than waiting on R1-A. That is what makes the two jobs genuinely
parallel; the real titan meets the real blade at the round gate, not inside an agent.

**R1-C — the screen.** Two deliverables, both tiny, both unblocking:
`IsDefaultUiCamera` on the camera in `render::attach_camera` (`src/render/mod.rs:67-95`), and
`debug::spawn_overlay` + `debug::update_overlay` registered in `DebugPlugin` — they compile
today at `src/debug/mod.rs:287-331` and are dead code. Extend the overlay by **one line per
living titan**: `husk#1 Windup 21/36`, read from the `shared`-owned `TitanState`.

**R1-D — the mouse.** Three things, and the first is a bug:
`net::local::read_input` runs in `FixedPreUpdate` (`src/net/mod.rs:49`) and reads
`AccumulatedMouseMotion`, which is **assigned** each `PreUpdate`
(`bevy_input-0.19.0/src/mouse.rs:257-267`) while `FixedPreUpdate` runs **0..n times per frame**
(`bevy_app-0.19.0/src/main_schedule.rs:358-361`). At 144 fps most frames' mouse motion is
**thrown away**; on catch-up frames it is applied **twice**. `src/net/local.rs:22-24` claims the
opposite in prose. **Repro first, then the fix** (rule 5). Then: cursor grab
(`CursorOptions` default is `grab_mode: None`, `bevy_window-0.19.0/src/window.rs:782-789`;
`Locked` on Wayland, Bevy falls back on X11 at `:768-771`) and `Esc` to release it. A locked
pointer with no release key is a game you have to `pkill`.

### Round 1 gate — all four must hold

```bash
cargo check 2>&1 | grep '^error'                       # empty
cargo test                                             # green, no test lost
cargo run -- --offscreen --script scripts/f056-husk.txt --ticks 240 \
     --screenshot docs/images/f056-husk.png            # exit 0
sha256sum docs/images/f056-husk.png                    # run twice, identical
```

Plus, by hand: `spawn titan husk 0 0 -40` followed by `assert titans > 0` must now hold
where it has always measured `0`. And the F3 overlay must be **in** the offscreen PNG — if it
is not, the `IsDefaultUiCamera` reasoning was wrong and Round 2's HUD has no evidence route
(Risk 3).

---

## 5. Round 2 — the kill lands, the mission ends, the screen shows it

| job | owns | builds |
|---|---|---|
| **R2-A `combat/feel`** | `src/combat/` (+ new `hitstop.rs`), `src/render/camera.rs`, `tests/combat.rs` | **F-034**, **P5** (player `Health`, no F-ID) |
| **R2-B `mission`** | `src/mission/` + new files, new `tests/mission.rs` | **F-070**, **F-071** |
| **R2-C `hud`** | `src/hud/`, new `src/hud/crosshair.rs`, new `tests/hud.rs` | **F-170**, **F-171** |
| **R2-D `titan` II** | `src/titan/`, `tests/titan.rs` | **F-053**, **F-052** (reduced) |

**R2-A.** Hit stop as a **tick-counted component**, gating the drive systems of the two entities
involved. **Never `Time<Virtual>::set_relative_speed`** — `Time<Fixed>` accumulates its overstep
from `Time<Virtual>::delta()` (`bevy_time-0.19.0/src/fixed.rs:243-247`), so slowing virtual time
slows the **tick rate itself**, and `Tick` carries the RNG seed and every intent's stamp
(`src/shared/schedule.rs:14-19`). avian's `Time<Physics>::set_relative_speed` has the same
problem. Camera kick lives in `Update` and is purely visual. Second half: player `Health`,
subtracted on a titan `Strike` in range, `MovementState::Downed` at zero — **do not despawn the
player**, the type documents "a state with a timer, not a removed entity".

**R2-B.** `MissionPhase` as a `States` enum with `DespawnOnExit<S>` (not `StateScoped`, renamed
at `bevy_state-0.19.0/src/state_scoped.rs:149`). Three points that are easy to get wrong:
mission phase as a `Resource` is fine and player state is not — write that reasoning into the
file so nobody quotes rule 4 at it in three weeks; the kill counter is a **component on a
`Mission` entity with per-`PlayerId` counts**, not a `Resource<u32>`, because F-096/F-161a want
per-player credit later and it costs nothing today; and **the timer counts fixed ticks**
(330 s = 19 800 ticks), because a wall-clock timer makes every script run flaky.

**R2-C.** Five elements: gas bar (cyan, left), blade pips (right), health bar (crimson, bottom
centre), crosshair (centre, **a ring or four ticks with a hole**), objective line (amber, top
centre). Colours are not a free choice (`docs/conventions.md:69-75`). Two API changes that
will bite anyone writing from memory: `TextFont.font_size` is now `FontSize`
(`bevy_text-0.19.0/src/text.rs:392`, enum `:487-500`) and `TextFont.font` is now `FontSource`
(`:383`, enum `:282-307`). No font asset is needed (`default_font`, `lib.rs:140-146`); no
`Camera2d` is needed. The repo already has both right at `src/debug/mod.rs:288-298` — copy that.

**R2-D.** The telegraph pose (a pure function of tick, see R1-A) and **navigation**:
`MoveAndSlide` is the right *collision* tool and the wrong *navigation* tool. A titan steering
straight at a 28 m block face (`maps.ron:51 lot_m: 28.0`) slides along it to the corner and then
steers back into it — the classic corner oscillation, and **no physics setting fixes it**. The
map is a regular grid (`lot_m 28 + street_m 7` = 35 m pitch, 400 m across ⇒ ≤ 121 nodes); a
BFS/A\* recomputed at 1–2 Hz per titan is a rounding error, is deterministic, and reads block
AABBs from `shared::SpatialIndex` **without an allow-list edge**. Also: the default
`penetration_rejection_threshold: 0.5` is in **literal metres** (`PhysicsLengthUnit` default
1.0, `solver/plugin.rs:201-206`) and is player-sized — a titan whose radius is metres needs a
per-size-class `MoveAndSlideConfig`, and those numbers belong in RON.

### Round 2 gate

Full `cargo test`; the two `--offscreen` runs `scripts/f070-lost.txt` and
`scripts/f071-won.txt` exit 0; **the HUD is visible in both PNGs**; two runs of each give the
same `sha256`.

---

## 6. Round 3 — a door in, a door out, and someone trying to break it

| job | owns | builds |
|---|---|---|
| **R3-A `menu`** | `src/menu/mod.rs`, `src/mission/` (the `--mission` reader only) | pause screen + `Cli::mission` reader — **F-175 partial, not claimable** |
| **R3-B `sound`** | `src/sound/mod.rs` | the silence line + one gas-empty beep |
| **R3-C `evidence`** | `scripts/*.txt` **only** | writes and runs every acceptance script, takes every picture, reports which criteria actually hold |
| **R3-D `refutation`** | **nothing** — writes only `docs/FINDINGS.md` | tries to make Rounds 1–2's claims false |

**R3-A.** `Cli::mission` is parsed (`src/shared/cli.rs:25`, `:83`) and has **no reader anywhere**,
though `missions.ron` loads fine into `GameData.missions` (`src/data/mod.rs:71`). One reader, one
pause screen with Resume and Quit. F-175 says "every screen in at most two clicks" and this is
one screen — report it as 🟨 with the note that four of its five main areas do not exist.

**R3-B.** `bevy_audio::Pitch` synthesizes a sine and is registered out of the box
(`bevy_audio-0.19.0/src/pitch.rs:9-33`, `lib.rs:103`) — one beep at gas-empty costs four lines.
The `audio` feature needs `alsa.pc`, which is present on machine B and **absent on machine A**
(a 13 m 40 s build that then aborts). So: `#[cfg(not(feature = "audio"))]` must log **exactly one
line** at startup — *"audio feature off — this run is silent"*. A silent game that says it is
silent is a decision; a silent game that says nothing is a bug report waiting to be filed.
**No audio row can reach 🟧 by this project's own standard — a sound cannot be photographed.**
Ceiling: 🟨 with a measured frequency and duration, and the note that nobody has listened.

**R3-C** exists because the alternative is each building agent grading its own homework. It owns
no source file, so it never takes the cargo lock for a build of its own.

**R3-D** exists because CLAUDE.md requires it: *"every finding gets an independent stage that
tries to refute it. A claim nobody has attacked is 🟨."* Give it the three things this plan is
least sure of: **(a)** does `cast_shape` in `PostStep` see a titan that moved this tick, or are
the BVH AABBs one tick stale (`collider_tree/mod.rs:78-84` puts `UpdateAabbs` in `BroadPhase`,
i.e. at the *start* of the step)? **(b)** does `MoveAndSlide` actually jam where Survey A says
it jams — that section is reasoned from `project_velocity`'s contract
(`move_and_slide.rs:1127`), **not executed**; `examples/probe_avian.rs` is the existing pattern
for a 20-line check. **(c)** is the F3 overlay really in the `--offscreen` PNG, or did the
`IsDefaultUiCamera` reasoning only look right?

---

## 7. Round 4 — integrate (main head alone)

Full `cargo test`. `python3 tools/normen.py`. One end-to-end run:
`--offscreen --script scripts/game-full.txt` — spawn, fly, cut, win — exit 0, screenshot,
sha256 twice. Then set stages in `docs/features.ron`, `python3 tools/features.py`, write
`docs/ACCEPTANCE.md`, commit per the message norm, and **write the paragraph about what stayed
unseen**. Rows that only have two of the three pieces of evidence go to 🟨. Doubt moves the
stage down, not up.

---

## 8. Acceptance criteria — one per F-ID

Format: **claim · test · goes red when · number · picture**. Each one names the *broken
implementation it catches*. A criterion that cannot name one is worthless and is not in this
list.

---

### F-050 — Titan state machine (`titan`, prio 1, `depends_on: []`)

- **Claim.** A spawned husk runs `Idle → Pursue → Windup → Strike → Recover → Pursue`, each
  state lasts the number of ticks the RON says, and every state is visible in the overlay.
- **Test.** `tests/titan.rs::f050_the_husk_winds_up_for_as_long_as_the_file_says` — record the
  state each tick into a `Vec`; assert the sequence is exactly that, and that the `Windup` run
  length equals `round(data.titan("husk").windup_s * 60)` = **36**, read from `GameData`, not
  from a literal.
- **Goes red when** somebody adds a `Pursue → Strike` edge that skips `Windup`, or drives the
  state timer off `Time::delta_secs()` so the run length wobbles by ±2 ticks.
- **Catches:** *the FSM that is decoration* — an enum field that is set correctly while the
  titan walks and hits at the same time because nothing gates on it. A "the state changed"
  assertion passes that; a **tick-count on `Windup`** does not.
- **Number.** Ticks per state on a husk at 25 m: Pursue *n*, Windup **36**, Strike, Recover —
  all five recorded, with `[cachy]`.
- **Picture.** `docs/images/f050-states.png` — `--offscreen`, F3 overlay line reading
  `husk#1 Windup 21/36` **and** the titan's arm visibly raised in the same frame. Overlay text
  and pose must agree; that is the point of the picture.

---

### F-056 — Husk (`titan`, prio 1, `depends_on: F-050`)

- **Claim.** `spawn titan husk x y z` produces exactly one entity carrying `TitanId`, 10.0 m
  tall, with its cortex sphere centred at 8.9 m, and it dies on a cortex hit.
- **Test.** Two, and both are needed.
  `f056_the_cortex_sits_where_scale_ron_says` — assert the cortex child's
  `GlobalTransform.translation.y` equals `data.titan_cortex_height_m("husk")` ± 0.01, **and**
  that that function returns 8.9 ± 0.01 from the file.
  `f056_a_cortex_hit_removes_the_husk` — send `TitanHit{Cortex}`, step `death_s * 60` ticks,
  assert the `TitanId` count goes 1 → 0 and the collider is gone on tick one.
- **Goes red when** the cortex height is a Rust constant (change `medium` in `scale.ron` to
  12.0 and the first assertion follows the file while a literal does not), or when the cortex
  is parented to the pelvis so it does not follow the pose.
- **Catches:** *the titan whose hit zone is placed by a magic number* — the single most likely
  shortcut, because 8.9 is right there in the survey. The two-sided assertion (component
  follows `GameData`, `GameData` follows the file) is what makes it impossible to fake.
- **Number.** 1 entity; cortex centre **8.900 m**; body capsule radius from `width_fraction`;
  ms to spawn 1 and 8.
- **Picture.** `docs/images/f056-husk.png` — `--offscreen`, `PhysicsDebugPlugin` wireframe on
  (`Cargo.toml` already enables it *because* "🟧 demands a picture; without it there is no
  picture of a collider"): the box rig, the **amber** cortex sphere at neck height, and the
  1.8 m player capsule in the same frame for scale.

---

### F-064 — Size classes (`titan`, prio 1, `depends_on: F-050`)

- **Claim.** Every kind's height and cortex height come from its `size_class` in `scale.ron`,
  and nothing above `large` (14 m) can spawn this session.
- **Test.** `f064_no_kind_spawns_above_the_class_cap` — iterate all eight kinds, assert the
  spawn either produces a body of the file's height ± 0.01 or refuses with a named error for
  `huge`/`boss`. Plus the existing `tests/data.rs` guards.
- **Goes red when** somebody widens the cap silently, or when a new kind names a class that
  does not exist.
- **Catches:** *the spawner that scales one mesh by a hard-coded factor per kind* — which
  passes for the husk and silently gives the scuttler a 10 m body.
- **Number.** Five classes: 4.2 / 10.0 / 14.0 / (21.0 refused) / (28.0 refused).
- **Picture.** `docs/images/f064-classes.png` — small, medium and large side by side with the
  player, one frame, `--offscreen`.
- ⚠️ **The row's own acceptance ("all three classes require different approach angles") cannot
  be met** — it names three classes and the user gave five, and "approach angle" needs flight.
  This row caps at **🟨** with that note.

---

### F-030 — Nape hit zone / Cortex (`combat`, prio 1, `depends_on: []`)

- **Claim.** A blade swept along the player's displacement of one tick registers
  `TitanHit{zone: Cortex}` when it crosses the cortex sphere, and a non-cortex zone when it
  crosses the body.
- **Test.** Three, and the second is the one that matters.
  `f030_a_pass_at_30_m_s_hits_the_cortex`;
  **`f030_a_pass_at_75_m_s_still_hits_the_weaver`** — 75 m/s is 1.250 m per tick against a
  0.46 m cortex diameter, i.e. **0.37 ticks inside the target**;
  `f030_the_torso_does_not_count_as_the_cortex` — a pass that crosses the torso in front of the
  cortex must report a non-cortex zone, not `Cortex` and not `None`.
- **Goes red when** the swept `cast_shape` is replaced by a blade collider, a `Sensor` +
  `CollisionStart`, or an AABB overlap. All three sample *positions* once per tick, and avian's
  24 substeps do not help: `SubstepSchedule` re-runs only the solver
  (`dynamics/solver/schedule.rs:49-67`), broad and narrow phase run once per step
  (`collision/narrow_phase/mod.rs:131-147`).
- **Catches:** *the sensor-overlap implementation* — by name. It passes at 8 m/s, passes the
  husk at 30 m/s, and is **arithmetically incapable** of passing the weaver at 75 m/s. And the
  third test catches *the single unfiltered cast*, which returns the torso and never the cortex.
- **Number.** Hits at 8 / 30 / 75 m/s = 3 of 3. µs per cast over 1000 casts `[cachy]`, against
  the project's own recorded 0.21 µs for a 112 m ray over 4000 cuboids
  (`src/world/index.rs:24-25`).
- **Picture.** `docs/images/f030-cortex.png` — the amber cortex in wireframe with the swept
  blade capsule drawn as a gizmo along its segment, at the tick of contact.
- ⚠️ The row's acceptance says **"recognisable from 100 studs"** = 28.0 m at the project factor,
  while the bible says 100 **metres** (`Design-Bibel.md:45`) — a factor of 3.6 on the
  readability half of the criterion. **The hit half can be proven this session; the readability
  half cannot be written until Q-019 is answered.** At 100 m a 1.10 m cortex is **10.3 px**
  wide; at 28 m it is 36.7 px.

---

### F-034 — Hit stop and impact frames (`combat`, prio 1, `depends_on: F-030`)

- **Claim.** On `TitanHit{Cortex}` the player and that titan stop advancing for
  `round(hit_stop_cortex_s * 60)` ticks **while `Tick` keeps counting**, and the camera kicks.
- **Test.** `f034_the_hit_stop_freezes_the_bodies_and_not_the_clock` — assert `Tick` advances by
  N, the player's `Position` is bit-identical for exactly `round(hit_stop_cortex_s*60)` ticks,
  and differs on the next one.
- **Goes red when** it is implemented as `Time<Virtual>::set_relative_speed(0.05)` — because
  `run_fixed_main_schedule` accumulates the overstep from `Time<Virtual>::delta()`
  (`bevy_time-0.19.0/src/fixed.rs:243-247`), so the **tick stops advancing too** and the first
  assertion fails.
- **Catches:** *the one-line `Time<Virtual>` implementation* — by name. It is the obvious first
  instinct, it looks perfect on screen, and over a wire it is a per-client divergence nobody can
  reproduce.
- **Number.** Ticks from first cortex contact to despawn, **with and without**: without hit stop
  a husk cortex is crossed in `1.10 m ÷ 30 m/s = 36.7 ms` = **2.2 ticks**; with a 0.12 s stop,
  **~9.4**. Both measured, both in the commit message.
- **Picture.** `docs/images/f034-hitstop.png` — two crops from one `--offscreen` run at tick *t*
  and *t+6*: the titan visibly further into its dissolve, the player's position in the F3
  overlay **identical to the digit**.
- ⚠️ The row's own acceptance is a **blind test with human testers**. It is not satisfiable by
  an agent. Ceiling **🟧 on the substitute criteria above**, with the sentence "the blind test
  has not been run" in the note.

---

### F-053 — Telegraphed attacks (`titan`, prio 1, `depends_on: F-050`)

- **Claim.** Every attack has a wind-up ≥ 0.4 s during which the striking hand moves ≥ 150 px on
  a 1080-line screen at 40 m.
- **Test.** `f053_the_wind_up_moves_the_hand_far_enough_to_see` — take the hand's
  `GlobalTransform` at wind-up start and end, project both at f = 935.3 px
  (= (1080/2)/tan(30°), from `game.ron: camera.fov_deg: 60.0` read vertically per
  `bevy_camera-0.19.0/src/projection.rs:284-287`), assert Δ ≥ 150 px at 40 m. Plus the existing
  floor at `tests/data.rs:30`.
- **Goes red when** the wind-up is a colour flash, a 5° twitch, or when the pose angles in RON
  go to zero.
- **Catches:** *the telegraph that is a state and a timer with no pose* — the likeliest shortcut
  of all, because F-050 already provides the state. **A single screenshot cannot catch it**: a
  still frame of a titan with its arm down is indistinguishable from a titan with no telegraph.
  Only the two-sample pixel delta catches it.
- **Number.** Predicted hand travel: **412 px at 20 m · 206 px at 40 m · 82 px at 100 m**
  (10 m husk, shoulder at 0.82 × h, arm 0.44 × h, hand travelling ≈ 8.8 m). Measured must be
  within 10 % of those, or the rig proportions in RON are wrong.
- **Picture.** `docs/images/f053-windup.png` — **two panels**, tick *t* and *t+36*, same camera,
  arm demonstrably in two positions.

---

### F-052 — Pathfinding with size logic (`titan`, prio 1, `depends_on: F-050`) — **reduced**

- **Claim.** A husk spawned on the far side of a city block reaches the player without its body
  ever entering a block AABB.
- **Test.** `f052_the_husk_walks_around_the_house_and_not_through_it` — log the titan's position
  each tick; assert (a) it comes within `attack_range_m` inside *n* ticks, and (b) **no** logged
  position lies inside any block AABB inflated by the titan's radius, beyond `skin_width`.
- **Goes red when** the titan steers straight at the player and relies on `MoveAndSlide` — then
  (a) fails at the corner (oscillation, he never arrives) or (b) fails on penetration.
- **Catches:** *pure slide, no path* — the implementation that looks completely correct in an
  open field and deadlocks the first time a house is between the two.
- **Number.** Ticks to arrive over a 60 m detour; max penetration into a block AABB (must be
  ≤ `skin_width` = 0.01 m).
- **Picture.** `docs/images/f052-path.png` — top-down (`warp` high, `look 0 -90`), the titan's
  logged path as gizmo dots, curving around the block.
- ⚠️ **The row's own acceptance ("15 m titan destroys small props, 3 m titan avoids them")
  cannot be met**: there are **no props in `maps.ron` at all** — every block is `solid: true`
  and nothing is breakable — and the metres in the criterion are the stale pre-2026-08-09 pair.
  This row caps at **🟨** and the *size logic* half is explicitly not built.

---

### F-070 — Mission state machine (`mission`, prio 1, `depends_on: []`)

- **Claim.** The mission runs `Briefing → Deploying → Active → (Won | Lost)`, counted in fixed
  ticks, and a new mode is a row in `missions.ron`.
- **Test.** `f070_the_timeout_loses_the_mission_at_the_tick_the_file_says` — 0 kills; assert
  the phase becomes `Lost` at tick `target_duration_s * 60` = **19 800**, ±1.
- **Goes red when** the timeout accumulates `Time::delta_secs()` (then the tick varies with the
  frame rate and ±1 fails) or the duration is a Rust constant (set `target_duration_s: 10.0` in
  the file and a literal keeps waiting).
- **Catches:** *the wall-clock timer* — which passes a loose "eventually loses" test, is
  invisible in review, and makes every `--script` run flaky forever after.
- **Number.** Tick of the `Lost` transition: **19 800**. Tick of `Won` in the kill run.
- **Picture.** `docs/images/f070-lost.png` — the objective line reading `0/3` and the word
  `LOST`, with the tick from the F3 overlay in the same frame.

---

### F-071 — Skirmish (`mission`, prio 1, `depends_on: F-070`) — **reduced**

- **Claim.** Killing `kill_target` titans flips the mission to `Won`; the progress is on screen
  at every moment.
- **Test.** Two.
  `f071_the_last_kill_and_not_the_first_wins_the_mission` — 3 titans; assert `Active` after
  kills 1 and 2, `Won` on kill 3, counter reading 1, 2, 3.
  **`f071_an_empty_field_before_the_first_wave_is_not_a_win`** — assert the phase is still
  `Active` at tick 1, when zero titans exist because no wave has spawned yet.
- **Goes red when** the win check counts `TitanHit` messages (a torso hit then wins) or checks
  `titans == 0` (the mission is then won at tick 0, before anything has spawned).
- **Catches:** both of those, by name. The second is the more dangerous, because it *looks*
  right and produces an instant, silent win that reads as a bug in the spawner.
- **Number.** 3/3 kills; tick of `Won`; **and the count of non-cortex `TitanHit` messages in the
  same run, which must be > 0** — otherwise the first test proved nothing.
- **Picture.** `docs/images/f071-won.png` — objective line `3/3` in amber, phase `WON`.
- ⚠️ The description's civilian clause ("without too many NPC civilians dying") is **dropped**:
  there are no NPCs and no F-ID for them nearby. Ceiling **🟨** with that note, so the row can be
  moved back down later without an argument.

---

### F-170 — HUD base layout (`hud`, prio 1, `depends_on: []`)

- **Claim.** Gas, blades, health and objective are readable without opening a menu, and no HUD
  node covers the central 20 % × 20 % of the screen.
- **Test.** Two, and the second is the one with teeth.
  `f170_nothing_covers_the_middle_of_the_screen` — compute the screen rects of every
  `HudElement` from `ComputedNode`, assert none intersects the central box, with the crosshair
  modelled as four tick nodes around a hole rather than one node.
  **`f170_the_gas_bar_follows_the_gas_and_not_the_clock`** — set `Gas.current` to 30 of 100,
  assert the bar node's width is `Val::Percent(30.0)` ± 0.1; then 0 and 100.
- **Goes red when** somebody widens the objective banner to full width, or wires the bar to a
  constant.
- **Catches:** *the bar that is a picture of a bar* — the specific failure Survey C names, where
  every element of F-170's list is present and three of them are hard-coded because they have no
  producer yet.
- **Number.** The four displayed values at a named tick, matching the F3 overlay's digits
  exactly.
- **Picture.** `docs/images/f170-hud.png`, taken **with `--offscreen`**. If that image is empty,
  `IsDefaultUiCamera` is not on the camera: `DefaultUiCamera::get()` filters for
  `RenderTarget::Window` only (`bevy_ui-0.19.0/src/ui_node.rs:2997-3003`) and
  `screenshot.rs:218-222` swaps the target to `Image` — UI roots then get `Entity::PLACEHOLDER`
  and `physical_size: UVec2::ZERO`. **This criterion exists as much to catch that trap as to
  prove the HUD.**
- ⚠️ Health bar depends on P5 (`Health`). If P5 slips, the HUD ships with four elements and the
  row is 🟨.

---

### F-171 — Dynamic crosshair (`hud`, prio 1, `depends_on: F-002`)

- **Claim.** The crosshair has three distinguishable shapes for *no anchor* / *anchor in range* /
  *cortex in range*, and they differ in geometry, not only in colour.
- **Test.** `f171_the_three_states_differ_in_shape_not_only_in_colour` — assert the three states
  produce three different `(node_count, width, height)` tuples **while all three
  `BackgroundColor`s are forced equal in the test**.
- **Goes red when** the states are three colours on one node.
- **Catches:** exactly that — and forcing the colours equal is what makes it impossible to pass
  by accident. The row's acceptance is "the states are distinguishable under colour blindness",
  and there is no other way to make that falsifiable.
- **Number.** Node count and px size per state.
- **Picture.** `docs/images/f171-crosshair.png` — three crops. The picture proves they render;
  the **test** is what proves the colour-blindness clause.

---

### P1–P5 — the five prerequisites the backlog has no row for

Worth stating on its own: **there is no F-ID in 245 rows for "the game has a working mouse".**

> **2026-08-10 — the first windowed session.** Until today nobody had seen this game outside an
> offscreen buffer, and P3/P4 were unprovable in principle on machine A. On machine B they were
> attacked from **outside the process**, with `grim` reading the real compositor and `ydotool`
> injecting real evdev events. P4 and the pause screen now have a picture and a number
> (below), and since the tests landed they have the third leg too — **both are 🟧**. They were
> 🟨 for several hours on two of three pieces, which is the rule and not a formality; the stage
> moved only when the tests had been seen red.
> **What the in-process tests still cannot show**, and why the external run is not redundant:
> they spawn a bare `Window` entity with winit disabled, so they prove the game's *decision*
> about the pointer and never the compositor's execution of it. The X11 `Locked` → `Confined`
> fallback (`bevy_window-0.19.0/src/window.rs:768-771`) is invisible to them. Only the
> `ydotool`/`grim` run covers that half, and only on a machine with a screen.
> The first frame of the game ever seen in a real window is `docs/images/p4-first-light.png`
> (2560×1440): a titan that towers over the skyline, all five HUD elements correctly placed —
> and a flat, shadowless, textureless greybox with a one-colour sky. It is recognisably a city
> and a titan. It is not yet a place.
> Traps that cost this session real time are in [`FINDINGS.md`](FINDINGS.md) FIND-019 to FIND-021
> — read them **before** taking any window measurement, especially FIND-019, which produced a
> confident wrong bug report across twelve measurements.

| # | claim | test / goes red when | number | picture |
|---|---|---|---|---|
| **P1** `IsDefaultUiCamera` | UI renders into an `--offscreen` image | `tests/render.rs::the_camera_is_the_default_ui_camera`; red when the component is dropped. **The real proof is a PNG** | UI root `physical_size` = 1280×720, not 0×0 | any HUD picture above |
| **P2** F3 overlay registered | position, gas, tick and every titan's state are on screen behind F3 | `tests/debug.rs` asserts the systems are in the schedule; red when the `add_systems` line goes | — | `docs/images/f050-states.png` |
| **P3** mouse-look accumulation | applied yaw over a run equals raw device motion ± 1 % at any frame rate | red when `read_input` reads `AccumulatedMouseMotion` directly in `FixedPreUpdate`. **Repro before fix** (rule 5) | applied vs. raw yaw at 60 and at `--novsync`; today's loss is ~1 frame in 2.4 at 144 fps | — (a number, not a picture) |
| **P4** cursor grab + `Esc` **🟧** | the pointer is captured on start and released on `Esc` | `tests/menu.rs` — 10 tests, every one **seen red** under a targeted one-line mutation and green after restore: `want = CursorGrabMode::None` → 4 red · `Esc` made a no-op → 7 red · the self-healing guard dropped → 2 red · `Time<Virtual>` left running → 1 red · cursor never hidden → 4 red. `git diff src/menu/` empty afterwards, both files hash-matching HEAD. `p4_the_pointer_follows_the_screen_through_four_toggles` asserts `Screen`, `grab_mode` **and** `visible` after each of four presses — a toggle that dies on the second press survives the older one-flip tests | **seen in a real window, 2026-08-10 [cachy]**: a real 400 px `ydotool mousemove` rotates the view by 918 426 / 3 686 400 px (24.9 %) while captured and by **0 px** while paused; `grim -c` finds **0 px** of cursor while captured and a **165 px** blob at screen centre while paused; after 400 units of captured motion the cursor reappears at x 1280..1292 instead of the x≈1400 a free pointer would show. Four toggles, both directions. Idle noise 0 px | `docs/images/p4-cursor-captured.png` · `docs/images/p4-cursor-released.png` · `docs/images/p4-playing.png` |
| **P4b** the pause screen **🟧** | `Esc` brings `Paused` / `Resume (Esc)` / `Quit`, and the simulation really stops | `tests/menu.rs::f175_a_paused_game_does_not_simulate` (red at `left: 53 / right: 23` when the virtual clock is left running — the in-process twin of the external 215 px / 0 px tick measurement) · `f175_the_pause_screen_is_on_screen_once_and_stays_once` (red at `left: 42 / right: 12` when `spawn_pause_screen`'s guard is dropped). **The suspected duplicate-overlay bug does not exist**: 12 `PauseElement` entities after one pause, still 12 after five idle frames and a full Resume/pause cycle | **[cachy]**: before/after `Esc` 3 686 397 / 3 686 400 px changed, control run without `Esc` **0 px**. Verified by CONTENT, not by "the image changed": two bands of the exact `PLATE` colour, each **44 px tall × 240 px wide** at x 1160..1399, i.e. `Val::Px(240.0)` × `Val::Px(44.0)` with `row_gap` 14 (755−740−1). Real pause, not an overlay: F3 tick digits change **215 px** over 2 s playing, **0 px** paused | `docs/images/p4-paused.png` · `docs/images/p4-pause-plate.png` |
| **P5** player `Health` + `Downed` | a titan strike in range subtracts; at 0 the player is `Downed`, **not despawned** | `tests/combat.rs::p5_a_downed_player_is_a_state_and_not_a_removed_entity`; red when the entity is despawned | hits to down, ticks of invulnerability | `docs/images/f070-lost.png` |

---

## 9. Where this deviates from `docs/TODO.md`, and why

`docs/TODO.md` is generated from `depends_on`, so its *cross*-row order is real and I respect
it. Its *within-domain* order also puts Must before Should before Could, and **that is where I
deviate**. Nine places, all deliberate:

1. **`combat`: F-030 → F-034, skipping F-031, F-032, F-033.** All three are prio 1 and all three
   come before F-034 in the table. Reason: **F-031's acceptance is unmeasurable** — there is no
   titan health anywhere, so "60 % more damage" has nothing to be 60 % of, and
   `damage_per_m_s: 1.4` is calibrated against nothing (Survey B §6.1). F-032 makes the fight
   interesting, not possible. F-033 is an economy for a fight that does not exist yet. F-034 is
   ahead of them because of one number: **36.7 ms**. Without a hit stop the entire kill happens
   in 2.2 frames and the player never sees it, only a counter change — and the row's own
   description calls it *"the most important reward signal in the game"*.
2. **`titan`: F-050 → F-056 → F-064 → F-053 → F-052, skipping F-054, F-065, F-066** (all prio 1)
   **and F-051, F-055** (prio 2). F-054 is a *performance* criterion about 60 titans at 8 ms; we
   spawn three. ⚠️ But build the FSM on an explicit `next_tick_at` accumulator from line one —
   retrofitting a variable tick rate touches every state, making the accumulator constant today
   costs nothing. F-065/F-066 are a director and a pool for a wave of three. F-051 is replaced
   by one number, `aggro_radius_m`.
3. **`titan`: F-057–F-063 (seven of the eight kinds) are not built.** Each is one anti-autopilot
   mechanic on top of F-050, and none is reachable before F-050 is 🟧. The husk is chosen because
   the bible gives it the only role with **no** mechanic underneath it: *"fundamentals of the
   approach angle"*.
4. **`hud`: F-170 → F-171, skipping F-172, F-175, F-176, F-177.** F-172 and F-175 are prio 1 and
   come *before* F-171 in the table. F-172 ("no action is hard-wired") is **violated by
   construction today** — every binding is a literal at `src/net/local.rs:56-67`, and the file
   admits it in its own header. F-171 jumps the queue because it is the **only readout the player
   has of F-002**, which is a raycast with no other visible consequence.
5. **`world`: the entire level-2 anchor block (F-021–F-032a, twelve rows, ten of them prio 1) is
   not built.** Level-1 raycast aiming (F-002/F-003) already produces a hook. Hemisphere
   candidate search, Q/E snap, scoring and highlighting are a *quality* upgrade to aiming, and
   the gate is "does the movement convince", not "is aiming assisted".
6. **`mission`: F-070 → F-071 only**, then stop. That respects the order; it just stops early.
7. **The five P-rows have no F-ID at all** and are scheduled first. The backlog contains no row
   for the cursor, the pause key, the mouse-motion bug or the UI camera. That gap is itself worth
   writing into `docs/FINDINGS.md`.
8. **`net`, `save`, `progress`, `squad` are untouched** — 87 rows, many prio 1. Justified by the
   bible's gate, below.
9. **F-052 and F-064 are built against criteria I know are stale** (they still name 3 m / 7 m /
   15 m). Both cap at 🟨 for that reason, and the source fix is a Round 0 item in
   `gameplay/features.xlsx` — or nothing, but never mid-session.

---

## 10. What is explicitly NOT built

The bible's gate is not ours to relax: **no meta system before the Vector Gear convinces**
(`prompts/init.md` §13, bible phase plan). Naming the list is what stops the plan from growing.

**Forbidden by the gate — not deferred by us:**

- **All of `progress/` — 30 rows.** F-120 levels and XP curve, F-121/F-122 gear rank and budget,
  **F-123 skill tree**, F-124 respec, F-125 loadouts, F-126 perks, **F-127/F-128 lineages**,
  F-129 Ichor path, F-130 artefacts, F-131/F-132 ascension, F-133 achievements, F-134 compendium,
  **F-140/F-141/F-142 currencies, loot tables and pity**, F-143–F-148, **F-225 cosmetics**,
  F-226 season pass, F-227 private servers, F-228, F-229 shop.
- **All raids and bosses** — F-090 raid framework, F-091–F-094 the four bosses, F-095–F-099.
- **`squad/` — 33 rows.** No second player, no revive, no marking, no leaderboards, no guilds.
- **`save/` — 8 rows.** Nothing is persisted. **Closing the window deletes everything.**

**Deferred by us, with a reason, not by the gate:**

- `net/` — 16 rows. The seam exists (`Intent`, `--lag`, T-019 🟨). No wire, no prediction, no
  interpolation.
- `combat` F-031, F-032, F-033, F-035–F-044 — thirteen rows. No speed-damage curve, no limb
  zones, no blade wear, no lance, no injuries, no combos, no damage numbers, no finisher camera,
  no ground melee.
- `titan` F-051, F-054, F-055, F-057–F-063, F-065, F-066 — fourteen rows. One enemy kind, no
  perception model, no LOD, no group behaviour, no spawn budget, no pooling.
- `mission` F-072–F-084 and the whole tutorial ladder F-185–F-190. **One mode.**
- `hud` F-172, F-175 (beyond one pause screen), F-176, F-177, F-178.
- `world` F-021–F-032a — level-2 anchor points in full.
- **All art.** Machine A has no Blender (`assets/data/art.ron:8-9`), so: no rigs (`models.ron`
  A-040–A-045), no skinned deformation, no foot IK, no authored death sequence (AN-081's
  "collapse then vaporize" becomes a box scaled to zero), no cortex mesh (A-046's "slightly
  raised, steaming neck-piece" becomes a flat amber box). `art.ron`'s `use_blend: false` branch
  **is** the placeholder pipeline; a box titan is not a hack around it.
- **Sound — 118 rows in `docs/backlog/audio.ron`, one beep built.** And note: `tools/sound/`
  **does not exist**; it is an assumption recorded in Q-006, not something built.
- **No sky, no fog, no shadows.** All three already noted as absent in `docs/ACCEPTANCE.md:79-83`.

---

## 11. Three risks, and the measurement that shows each one early

### Risk 1 — the Vector Gear does not reach flight speed

Everything in this plan is designed for a player moving at 30 m/s. If `reel.rs` and the joint do
not produce that, the whole combat round is tested at **6 m/s run speed**
(`game.ron:36 run_speed_m_s: 6.0`) and proves nothing about the game — the bible's own sentence
is *"combat is the side effect of good movement"*.

- **Measurement, at the Round 0 gate:** one `--script` run — two hooks, a boost, a reel —
  holding `assert speed > 25`. That assert exists today.
- **If it fails:** do **not** proceed to Round 1 as written. Survey C's fallback (F-185, the
  movement course) is *not* free either — it depends on F-006 swerve, which does not exist as a
  file. The honest smaller fallback is: keep Round 1, cut from a jump instead of a flight, and
  write in `docs/ACCEPTANCE.md` that **combat has never been tested at the speed it is designed
  for**. Do not let that sentence go missing.

### Risk 2 — the swept cast misses moving targets, and it is found in Round 3

The cut runs in `PostStep`. The collider tree's AABBs are refreshed in
`ColliderTreeSystems::UpdateAabbs`, configured `.in_set(PhysicsStepSystems::BroadPhase)`
(`avian3d-0.7.0/src/collider_tree/mod.rs:78-84`) — i.e. at the **start** of the step. So in
`PostStep` the tree's AABBs are one tick old while `cast_shape`'s narrow phase reads current
`Position`/`Rotation`. avian enlarges those AABBs by `AABB_MARGIN` plus a velocity term
(`collider_tree/update.rs:707-780`), which *almost certainly* covers a titan at 2–11 m/s.
**"Almost certainly" is not a measurement**, and this is the mechanism the entire game is built
on. `src/vector/aim.rs:53-61` raises the mirror-image concern from the other side and says in so
many words that it is safe *only because every body in the world is static today* — and
**combat is the domain that brings the first moving collider into the world**.

- **Measurement, as R1-B's FIRST test, not its last:** a scuttler crossing at 11 m/s with the
  blade passing exactly at the tick boundary. If the static case is green and that one is red,
  the AABB enlargement is insufficient.
- **Cost if found late:** the fix is one line (order the cut after a manual tree refresh, or cast
  a slightly enlarged shape) — but every criterion in Rounds 1 and 2 would have been signed off
  against a mechanism that does not work on moving targets, and every one would have to be re-run.

### Risk 3 — the evidence route collapses and nothing can reach 🟧

`--offscreen` is the project's **only** bit-identical screenshot route (`sha256 = eb212dfe…`).
Two things in this plan can break it, and neither announces itself:

- **The HUD is invisible in exactly the screenshots meant to prove it.** Predicted from source
  (§8 F-170), fix is one component — but the fix is *reasoned, not measured*. An agent could
  report "HUD built, image attached" and the image would show no HUD.
- **A procedural pose written against `Time` instead of ticks** silently destroys bit-identity.
  Nothing errors; the sha256 just stops matching, and if nobody checks it, the reproducibility
  claim quietly becomes false.

- **Measurement, at the end of *every* round, not just the last:**
  `cargo run -- --offscreen --script … --screenshot X.png` twice, `sha256sum X.png` twice,
  identical — and open the PNG and look for the overlay text. Two commands, thirty seconds, and
  they protect the only thing that separates this project from "sollte jetzt gehen".

**A fourth, cheaper one, stated for completeness:** the fan-out itself. Four agents share one
75 G `target/` on 80 G of free disk (§0). If Round 1's wall clock is not clearly under 3× the
slowest single agent, the parallelism is buying queueing, and Round 2 should be run with two.
Measure it: note the round's start and end time. That is the whole instrument.

---

## 12. The honest paragraph — what this will NOT be

A player who starts this at the end of the session gets a **three-minute proof of concept, not a
game they will play twice.** There is exactly **one enemy**: a ten-metre stack of grey boxes with
an amber ball on its neck, which walks at you in a straight line, raises an arm, and swings.
There is no second kind, no perception model, no group behaviour, and nothing that reacts to
being flanked — the husk exists precisely because it is the one titan in the bible with **no**
trick to it. **Nothing is saved**: no XP, no gear, no unlocks, no settings; closing the window
deletes the entire session, and there is no skill tree, no currency, no loot and no cosmetics
because the design bible forbids building them before the movement convinces. There is **no
second player**, though every line of the architecture is shaped by the assumption that there
will be one. **It will be almost silent** — one beep at most, on one of the two machines, and
118 sound rows untouched, including the gas hiss the genre is named for. **Nothing is animated**
in the sense a player means it: the titan's arm swings because a number rotates a box, its feet
slide, it has no face, and when it dies it shrinks and fades because there is no Blender on this
project's build machine. The city is a seeded grid of identical blocks with **no landmark, no
wall, no bastion, and no sky** — the upper half of every screenshot is flat dark grey, and that
is `ClearColor`, not atmosphere. Damage is **binary**: the cortex kills, everything else does
literally nothing, so there is no reason to aim anywhere but one place and no blade wear to
manage. The player can be knocked down but not revived and not respawned; the mission simply says
`LOST`. And **every number in it is invented** — most of them written by the main head in Round 0
against no measurement at all, marked `⚠️ UNTUNED`, waiting for the first person who plays it to
say that the titan is too slow or the gas runs out too fast.

What it **will** be: one command, a captured mouse, a hook that catches, a swing that gains
speed, a target that has to be approached rather than clicked, a kill you can feel because the
world stops for a tenth of a second, and a verdict at the end. That is a game. It is a small one,
and calling it anything larger would be the exact failure this project's own CLAUDE.md warns
about: **"gebaut, ungetestet — 🟨" ist ein guter Satz, "sollte jetzt gehen" ist keiner.**

Expected stage tally at the end, if every round lands: **F-050, F-056, F-030, F-034, F-070,
F-170, F-171 at 🟧** · **F-052, F-053, F-064, F-071 at 🟨** with their reasons written down ·
**0 ✅**, because only the user sets that.
