# HANDOVER — where this session stopped and what comes next

Updated: 2026-08-09 · Stage: 🟨 (written down, not verified by a second head)

**Read this first if you are picking the project up.** Then do the session ritual from
[`CLAUDE.md`](../CLAUDE.md) — it is not optional, and it will tell you things this file does
not.

---

## 1. Where we actually stand

| | |
|---|---|
| Engine | Bevy 0.19 + **avian3d 0.7.0** (the user chose it; it requires exactly bevy 0.19.0) |
| Language | **English everywhere.** Only `prompts/`, `gameplay/`, quotes from them and the git history stay German |
| Tests | see `cargo test` — the count before the vector round was **151 green** |
| Stages | `docs/STATUS.md` — **1 🟧** (T-006), 6 🟨, the rest ⬜ of 245 rows |
| Disk | ⚠️ **92 % full, `target/` alone is 75 G.** No `cargo clean` (75 G rebuild against an 80 G margin), no `--release` build, and **only one compiler at a time** |

**What runs today:** the city builds itself out of `assets/data/maps.ron` from a seed (79
blocks, 63 of them taggable), the camera turns with the intent, gizmos show which surface is
hookable, and `--offscreen` writes a real PNG without a window — bit-identical across runs.
That last one is why anything can ever leave 🟨.

**What does not run yet:** the game. There is no hook, no rope, no gas, no titan, no combat,
no HUD, no mission. `src/vector/` was empty until the round that just got interrupted.

---

## 2. The interrupted round — read this before you touch `src/`

The **Vector Gear round was stopped mid-flight** at the user's request. State in the working
copy, uncommitted:

- **Done and in the tree:** the seam (avian registered, player is a physics body), plus
  F-001 hooks, F-002 aiming, F-018 gas, F-007 boost. `cargo check --all-targets` is clean.
  Their pictures are in `docs/images/f-001-hooks.png` and the three next to it.
- **Not started:** F-004 pendulum (`src/vector/rope.rs`) and F-005 reel-in
  (`src/vector/reel.rs`) — both still stubs.
- **Not run:** the counter-check that would have attacked all of it, taken the flight
  pictures and judged the stages. **Nothing from this round is verified by a second head, so
  none of it is above 🟨**, whatever the individual reports claim.

The full commissions are in
[`measurements/stage3-commissions.js`](measurements/stage3-commissions.js)
— verbatim and re-runnable.

---

## 3. The architecture is measured. Do not re-litigate it.

Three measurement rounds produced this. Every line carries the number that decided it, and
the long form is in [`measurements/`](measurements/README.md) plus `examples/probe_avian.rs` (3192 lines of
runnable probes — that file is the memory of those rounds).

```
ROPE:      avian DistanceJoint, limits = (0, L)
           58.23 m/s from v0 20 when reeling (angular momentum preserved)
           against exactly 20.000 for a hand-written clamp — the clamp eats the feeling
REFEREE:   none needed. Worst wall penetration -0.0043 m of -0.01 allowed, 18 cases
           BUT every world collider MUST carry a RigidBody (Static is enough), or a
           referee added later is blind to it
SUBSTEPS:  24. Holds swing loss (4.26 %/s vs 8.97 at 6) AND the wall.
           0.72 ms/tick with 20 players in a 401-body city, budget 4 ms
REEL-IN:   shorten limits.max PER SUBSTEP, never per tick (per tick injects
           rate x SubstepCount = 677 m/s), plus MaxLinearSpeed(max_speed_m_s)
```

**Three blockers with their fixes** — all measured, do not rediscover them:

1. `App::update()` does not call `Plugin::finish()`. avian creates `SolverDiagnostics` there
   and takes it as a non-optional `ResMut`, so **every test panics** once avian is
   registered. Fix: `app.finish(); app.cleanup();` at the end of `pub fn app()`.
2. A ray hits the player's own collider and `with_excluded_entities([player])` does not help
   — the filter matches the *collider* entity. Fix: `Collider::capsule_endpoints(...)` on the
   **same** entity, no child collider.
3. `CollisionEventsEnabled` on the body does nothing. For impact data use `Collisions` +
   `ContactPoint::normal_speed` instead — no events needed.

Plus: players need `SleepingDisabled`, or one hanging still on a rope falls asleep.

---

## 4. What comes next, in order

The full plan is [`PLAN-GAME.md`](PLAN-GAME.md); its definition of "playable" is one
mission, one enemy kind, one way to win, two ways to lose, three minutes.

1. **Finish the vector round.** F-004 pendulum and F-005 reel-in on top of the hooks, then
   the counter-check that was cut: attack every criterion, look at every picture, write a
   script that actually flies (aim, hook, swing, reel, boost, release), and judge each F-ID.
   Only then set stages in `docs/features.ron` and regenerate with `python3 tools/features.py`.
2. **The titan.** Minimum is F-050 (reduced state machine), F-056 (husk), F-053 (telegraphed
   attack) and F-030 (the cortex hit). Kinematic body **plus `CustomPositionIntegration`** —
   without that marker a kinematic titan moves twice per tick. Navigation is A\* over the
   11×11 street grid, not `MoveAndSlide` (that is a collision tool, not a navigation tool).
3. **Combat.** The cut must be a **swept `cast_shape`**, not a collider or a sensor: at
   30 m/s the player is inside the cortex for 0.8 of a tick, and avian's 24 substeps do not
   help because broad and narrow phase run once per step. Damage comes from the **relative**
   speed projected onto the cast direction. F-034 hit stop is not optional — a husk cortex
   kill lasts 36.7 ms, which is 2.2 frames, and without the stop the player never sees it.
   Hit stop must be tick-counted; `Time<Virtual>::set_relative_speed` would slow the tick rate
   itself and break the seeded rng.
4. **The frame around it.** Mission state machine (kill 3, or the clock), HUD (gas bar,
   crosshair, counter), pause with `Esc`, and a window that captures the mouse — today the
   game mostly runs headless with scripts.

---

## 5. Open questions the user owns

`docs/QUESTIONS.md` has 27, each with an `ASSUMPTION:` the work runs under and a rollback
point. The four that will bite soonest:

- **Q-025** — a 28 m titan is about 7.0 m wide and the alley is 7.0 m. Both numbers are the
  user's; together they are unsatisfiable. Does not bite until a big titan spawns in the city.
- **Q-026** — the cortex must be readable from 100 studs (features.ron) or 100 metres
  (bible). Factor 3.6, and it decides the whole approach design.
- **Q-027** — a titan has no health value anywhere in the repository, no body width and no
  turn rate.
- **Q-002** — the stud→metre factor lost one of its three cross-checks when the user set the
  anchor range directly to 90 m. Either 0.28 still holds for everything else, or every map
  number shrinks by 20 %.

---

## 6. The honest paragraph — what went unseen

**Nobody has played this.** Every claim in this project rests on tests, measurements and
offscreen screenshots; not one frame has been seen in a window by a human being. The city
looks like a city in a picture — whether flying through it feels like anything is unknown, and
the user's own numbers make the houses unusually flat (4.5–11.5 m on a 28 m block, so roughly
1:3 to 1:6). The vector round was stopped before its counter-check, so its four finished
features are self-reported and unattacked. `examples/probe_avian.rs` has no incoming reference
and escapes the zombie rule only because the norms tool does not glob `examples/`. And the
disk is at 92 %: the next session should deal with that before it starts a big build, because
`ld: signal 7` reads like a compiler bug and is a full disk.

---

## 7. Git state — read before you push

**The work is on the branch `session-2026-08-09`, not on `main`.**

`main` and `origin/main` have **diverged**: the seventeen setup commits exist on GitHub under
different hashes than the ones in the local history (`6a4e87b` there against `86e6b35` here
for the same initial commit — someone rewrote the history at some point). `main` is 32 ahead
and 17 behind, and neither is an ancestor of the other.

Pushing this session's work onto `main` would therefore have needed a force push, and that
would have thrown away seventeen commits on the remote. **That decision is the user's, not
mine**, so the work went to its own branch instead. Nothing is lost, and everything is on
GitHub:

```
origin/session-2026-08-09   ← this session, 176 tests green
origin/main                 ← the old setup commits, untouched
```

Content-wise the branch contains everything `origin/main` has; the difference is 160 files,
+21572/-6760. Whoever continues either merges the branch, or force-pushes it onto `main`
after checking that nothing on the remote is worth keeping — **check first, the seventeen
commits are real work.**
