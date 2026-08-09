# HANDOVER — where this session stopped and what comes next

Updated: 2026-08-09 · Stage: 🟨 (written down, not verified by a second head)

**Read this first if you are picking the project up.** Then do the session ritual from
[`CLAUDE.md`](../CLAUDE.md) — it is not optional, and it will tell you things this file does
not.

---

## 1. Where we actually stand

**Updated 2026-08-09, late. The four sections below replace what stood here before; almost
every line of the previous version had become false.**

| | |
|---|---|
| Engine | Bevy 0.19 + **avian3d 0.7.0** |
| Branch | **`session-2026-08-09`**, pushed. `main` is still the old, diverged history — see §7 |
| Tests | **317 green, 0 red** (204 at the start of this session) |
| Stages | 🟧 T-006, B-001/T-036a, P1, P2, F-030, F-034, F-070, F-171 · 🟨 F-001, F-002, F-003, F-004, F-005, F-007, F-018, F-050, F-056, F-064, F-071, F-170 |
| Disk | ⚠️ **The old warning was wrong.** 372 G free, 14 % used, `target/` 34 G. `--release` and a rebuild are affordable. The one-compiler rule still holds — it is cargo's lock on `target/`, not a space problem |
| Pictures | `--offscreen` **works on machine A** (Intel ADL-N, Vulkan, Mesa 25.0.7). Q-009 is answered. This is what let anything reach 🟧 here |

**What runs today:** the city builds from a seed; a hook fires, **anchors on a real building**,
and hangs an avian `DistanceJoint`; reeling in gains speed (62.73 m/s from v0 20); a husk stands
in the street with an amber cortex on his nape, walks, winds up, strikes and dies to a cortex
cut; the swept blade hits at 8, 30 and 75 m/s; the world stops for 7 ticks on a kill; a mission
runs 19 800 ticks and says `LOST`, or `WON` on the third kill; and a HUD draws gas, blades and a
three-shape crosshair.

**What does not run yet:** the mouse is not captured and there is no window anybody has seen; a
player flying past a *solid* husk **cannot reach the nape** (Q-030); and nothing is saved.

---

## 2. The three things that cost this session the most, so you do not repeat them

1. **The whole vector round was unreachable in the real game and 41 tests were green.**
   `world::index::maintain_index` was an empty stub, so no entity ever carried a `BodyId`, so
   every hook reported `found nothing anchorable`. Every test injected the carrier by hand.
   Written up as `B-001`. **The lesson is not "write more tests" — it is that a test which
   builds its own world proves nothing about the world the player is in.** Every round since
   ends with a `--script` run in the real game.
2. **A script asserted the broken behaviour and locked it in.** `scripts/f-001-hooks.txt`
   carried `assert speed < 0.5` with the comment *"a hook that finds nothing must not move the
   player"*. It reported "5 asserts held", exit 0, for a completely dead feature. It now holds
   14 asserts, and the old version goes red 4 of 14.
3. **The handover you are reading was wrong about the disk, the test count and the state of the
   vector round.** Check `git log` and run the ritual before believing any document here,
   including this one.

---

## 3. The architecture is measured. Do not re-litigate it.

```
ROPE:      avian DistanceJoint, limits = (0, L)     — built, F-004 🟨
REFEREE:   none needed                              — every world collider carries a RigidBody
SUBSTEPS:  24                                       — game.ron
REEL-IN:   shorten limits.max PER SUBSTEP, never per tick, plus MaxLinearSpeed
```

Long form in [`measurements/`](measurements/README.md). Three avian traps, each a day if
rediscovered, are in `measurements/avian-blockers.md` — and two more were measured this session:
**`CustomPositionIntegration`** on a kinematic body (without it a titan moves 6.000 m where the
file says 3.000), and **avian reserves collision-layer bit 0** for its default layer, so a layer
placed there makes every untagged wall answer a cortex-filtered cast.

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
