# FINDINGS — mistakes I tripped over on the way past

Updated: 2026-08-12

**Whoever trips over something that is not part of their own task: write it down, with the
measurement beside it** — so that somebody else can check whether it really is wrong.

**Do not fix it quietly on the way past.** A foreign mistake fixed in passing is a fix nobody
reviewed, and it hides in the diff of a task where nobody is looking for it
(`prompts/init.md` §9c). Format: `FIND-00n <symptom>` + measurement.

---


> 📦 **The entries up to `FIND-184` live in
> [`archive/FINDINGS-001-184.md`](archive/FINDINGS-001-184.md)**, with a one-line index of every
> one of them. They moved on 2026-08-29 because this file had reached **812 kB** while the rule
> telling agents how to read it still said 108 kB. **Nothing was deleted.**

---


> 📦 **`FIND-185`..`FIND-197` live in [`archive/FINDINGS-185-197.md`](archive/FINDINGS-185-197.md)**
> with a one-line index. The live file keeps the newest 20. **Nothing was deleted.**

---

## FIND-198 — the two-rope stand-off has a boundary, and the boundary is a tightrope: the exact rule leaves 30x more sag than the same rule with one `min_rope_m` in hand

**2026-08-28 · [offlinebot] · `player::rope::hold_the_pair` · the fix for `FIND-191`, and the one
number in it that is not derivable**

`FIND-191`/`B-013` is fixed and the rule needed no taste: two `DistanceJoint`s with maxima
`L_l`, `L_r` on anchors `d_a` apart have a common solution **iff `L_l + L_r >= d_a`**. That is the
triangle inequality, it is necessary *and* sufficient, and the player's position is not in it.
**It is explicitly NOT the `FIND-186` mistake** — attempt 1 there bounded the *sum* of two
distances as a stand-in for bounding each of them, which is a heuristic and false at `n = 2`.

**What is not derivable is what happens AT the boundary.** At exactly `L_l + L_r = d_a` the two
spheres are **tangent**: the feasible set is a single point, the player stands on the straight
line between his two anchors, both constraint gradients lie along that line, and gravity pulls at
a right angle to both. Nothing in the pair carries his weight, and 24 XPBD substeps leave the
difference as sag. Measured over the same 84 cells (3 heights x 4 length pairs x 7 separations,
`Ctrl` held, 10 080 ticks,
`tests/vector_rope.rs::f004_two_far_apart_anchors_hold_the_player_instead_of_dragging_him_past_a_maximum`):

| the reel is stopped at | worst arm past its own maximum | as % of that arm | infeasible ticks | pinned ticks |
|---|---|---|---|---|
| nothing at all — the shipped build until today | **51.7104 m** | 1 724 % | 7 920 (78.6 %) | 5 208, **all by a violation** |
| `L_l + L_r = d_a` — the exact rule | 0.2810 m | 1.04 % | 0 | 9 |
| `L_l + L_r = d_a + vector.min_rope_m` — as shipped | **0.0092 m** | 0.11 % | 0 | 3 |
| for scale: the `Ctrl`-free 288-cell matrix, a plain swing | 0.0050 m | 0.017 % | — | — |

**184x from the rule, and another 30x from one term on top of it.** The margin is not a fudge:
two real ropes spanning a gap hang in a **V**; they do not stand as a horizontal line, because a
horizontal line needs infinite tension. The margin is exactly the slack the V is made of. Which
number it should be is `Q-079` — the honest home is `vector.two_rope_slack_m`, and `min_rope_m`
is the stand-in this round shipped under, with the rollback point named.

### Three things the round measured that are not the headline

1. **The residual scales with the span, not with the tick.** At the tangent boundary the sag was
   0.281 m on a 54.5 m span, 0.257 on 56.4, 0.245 on 48.8, 0.163 on 39.9 — **0.41 % to 0.52 % of
   `d_a` in every cell.** That is a compliance signature, not a solver hiccup, which is what says
   the tangency and not the reel is the cause.
2. **The fix could have been "turn the reel off", and only a control catches that.**
   `scripts/f012-tworopes.txt` ACT 3 is the same three seconds of `Ctrl` on **one** rope from the
   same tile: **y 49.511 -> 54.022, +4.5 m**, `F-005`'s own acceptance sentence. ACT 2, two ropes,
   reads y 48.291 -> 47.734 and 6.696 m/s. **Both acts hold in the shipped build; against a
   binary with the fix's one line deleted, ACT 2 reads `assert Speed > 2 — measured 0.001` and
   ACT 3 still passes** — so the control is doing its job in both directions.
3. **The old tripwire had to be INVERTED, and it said so itself.**
   `find191_the_reel_can_make_two_maxima_impossible_and_that_is_older_than_the_drive_s_joint`
   asserted `drive > 10.0` with the message *"`FIND-191` is either fixed or no longer reproduced
   by this fixture, and it must not stay written as if it were still true"*. It is now
   `b013_the_two_rope_hold_reaches_both_force_models_because_the_reel_is_one_system`, it keeps the
   cross-model differential unchanged (`Drive` **0.0093 m** · `Pendulum` **0.0093 m**, identical),
   and it asserts the repair instead of the defect. **A test that names a defect is a liability
   the day the defect dies; the one that names the CONTROL survives the fix.**

### What the sweep holds constant, said out loud

Every fixture in this project that hid a defect hid it in an axis it held constant (`CLAUDE.md`
§6 rule 5, four instances). This one varies separation, both rope lengths **against each other**,
and the player's height along **one column of the map** so nothing but `y` changes; it holds
**yaw** (0°, one value), the key combo (`Ctrl` only), the force model (`Drive`; the differential
above is what covers `Pendulum`), the anchors' elevation (20°), pitch (0) and the gas (full).
**There is no `continue` in the per-tick loop** — a cell that loses a rope is counted in
`ticks_with_one_rope` and asserted on (it read 0 of 10 080 before and after, so no cell was
excluded from any number above).

**Related:** `FIND-191` · `FIND-195` · `FIND-186` · `B-013` · `Q-079` · `Q-058` · `F-004` `F-005`

## FIND-199 — `Q-078` is built: the `F-003` tag is now a **switch**, and the guards that replaced it, measured

**2026-08-28.** The user, 2026-08-27: *„es soll auf jeglicher oberflqche einhaken. nicht an
hardcoded punkten etc!"* and, minutes later, *„spaeter soll man auch bestimmte sachen toggeln
koennen. also an bestimmte sachen ran haken an andere nicht aber grundsetzlich erstmal ales!"*

### 1 · What changed, in one line of behaviour and two files

`vector::aim::cast` used to write `anchorable: body.mask.contains(BodyMask::ANCHORABLE)` — i.e.
`F-003`, *"Kein Haken auf ungetaggten Parts moeglich"*. It now writes
`anchorable: is_hookable(hookable, body)`, and the same predicate replaced the **second** reading
of the same bit in `vector::hook::anchorable_beyond_reach`. New file: `src/vector/hookable.rs`
(`SurfaceKind::{Tagged, Untagged}`, `HookableSurfaces`, `is_hookable`), registered as
`app.init_resource::<HookableSurfaces>()` in `VectorPlugin`.

**The data is not dead, it is the handle.** `maps.ron: anchorable` still decides
`SurfaceKind::of`, and `HookableSurfaces::TAGGED_ONLY` restores `F-003` exactly — **one value, no
code**. That is the rollback point Q-078 asked for, and it is tested rather than claimed
(`tests/vector_aiming.rs::f003_the_tag_survives_as_a_switch_that_can_take_the_untagged_surfaces_back_out`
drives the real cast on the real map, flips the resource, and flips it back).

**Two implementations of one question became one.** `anchorable_beyond_reach` asked the
`ANCHORABLE` bit itself. With a switch that would have drifted on the first flip: the miss word
would have said *"out of reach"* about a surface the switch had turned off. Same shape as the
`CLAUDE.md` rule-5 corollary — *one writer decides, everyone else reads the answer*.

### 2 · The measurement that made this smaller than it looked

`Ashgate` logs **`2901 blocks built (245 placed, 2656 generated), 2901 of them anchorable`**.
There is **no untagged body on the shipped map**, so on Ashgate this change is provably a no-op:
`is_hookable(EVERYTHING, tagged) == is_hookable(TAGGED_ONLY, tagged) == true`. Every script in the
corpus behaves bit-identically before and after. The only map that can falsify anything is
`graybox`, with **22** `anchorable: false` rows — the 400 m ground slab, the wall at `z = -33.5`
and the aqueduct's twenty columns. The red test therefore lives there:
**145 stances, 145 hits, 0 skipped, 81 of them on untagged rows, 79 refused a hook before the fix
and 0 after.**

### 3 · `F-003` existed to *"verhindert Physik-Exploits"*. What actually holds now — measured

`scripts/q078-fling.txt`, **30 asserts, exit 0**, on Ashgate against `gravity_m_s2: -32`:

| attempt | reading |
|---|---|
| hook the pavement **under your own feet**, at rest | anchored at `(168.19, 1.50, -50.32)`; 0.37 s later **21.3 m/s**, about **5.8 m/s more than free fall** — the pull points down because the anchor does. `Ctrl` for 3 s ends at **0.000 m/s standing on the street**: the thing you are pulled into is the floor |
| hook the ground **while falling at 45.9 m/s** | 53.3 → **60.8 m/s**, rope slack the whole way (he falls *towards* his anchor). Arrives at rest |
| free fall from 90 m | **exactly `75.000`** and stays there — `vector.max_speed_m_s` is a hard avian clip, not a soft target. **Nothing in any run exceeded it** |
| drive into an anchor 8.3 m away at speed | **4.9 m/s** at the anchor; `min_rope_m` (3.0) plus the fade band ate it |
| **a rope on a walking titan and a rope on a roof 46 m below** | `rope == 2` for **5.5 s**, speed **12.5 → 3.6 → 1.7 → 4.1 m/s**, height **50.278 → 50.347 → 50.308** — **7 cm of drift while one anchor walked.** No resonance, no fling, no broken joint |

**So the existing guards hold for everything that was tried, and `F-012 Velocity-Clamp gegen Fling`
is not urgent.** The one thing that behaves *badly enough to feel* is not a speed at all: a hook
into the ground under your feet **adds to gravity**, which is a downward yank nobody has played
yet. That is a feel question for the user, not a physics bug.

### 4 · `F-029 Dynamische Ankerpunkte` arrived as a side effect, and it survives the joint

A titan's root capsule already carried `SOLID | ANCHORABLE`, so nothing had to be built. What was
**not** free is the physics: a rope anchored to a walking titan is a moving constraint against the
`DistanceJoint` that landed the same day (`F-005`, `7bb61cd`). `scripts/f029-grapple.txt` ACT 1
still holds (`rope == 1` across 2 s of walking), and the two-carrier row above is the harder case
and it is stable. **ACT 2 of that script fails (`Rope == 0`, `Titans == 1`) and it is not this
work:** the hook anchors correctly on the titan (`body 2903`), the *cortex cut* does not land —
its own header warns the pass closes at 9.75 m/s against `blades.min_speed_m_s` 8.0, and that is a
fall time, i.e. `docs/NEXT.md` §3G's gravity bucket.

**Related:** `Q-078` · `FIND-200` · `F-003` `F-012` `F-029` `F-023` · `docs/NEXT.md` §3G

## FIND-200 — the world fence is the ONE solid surface a hook cannot take, and it is invisible

**2026-08-28**, found while sweeping the graybox for `Q-078`. `world::bounds::build_bounds`
spawns four thick static `Collider::cuboid` panels around the map (`plan_fence`), and they carry
**no `shared::Body`** — so `vector::aim::cast` resolves them to `body: None`, and
`vector::hookable::is_hookable` refuses them by the rule that a hit with no hull and no `BodyId`
cannot carry an anchor (`B-001`).

**Measured:** a stance at `(212, 2, 0)` on the graybox — 12 m outside the 400 m ground slab —
looking back at the map centre reports a hit at **exactly `(210.0, 1.976, 0.0)`, `body: None`,
`hookable: false`**. That is the *outer* face of a fence panel whose inner face the log calls
`+-(200.0, 200.0)`. Four of 145 sweep stances landed there before they were clamped inside.

**Why it is not fixed here.** Outside the fence is a place the fence exists to prevent, so the
finding is about the **inside** face: a player flying at the map edge meets an invisible wall that
also refuses a rope, and *"everything is hookable"* now makes that a promise the world edge breaks.
`src/world/` is not this round's territory. Two options for whoever takes it: give the panels a
`Body` (then the edge is a climbable wall, which may be desirable), or leave them un-hookable and
say so in the HUD — `MissReason::SurfaceHoldsNothing` already exists and would be the honest word.

**ASSUMPTION the work continued under:** the fence stays un-hookable, because a rope on an
invisible wall is worse than a rope that refuses.
**Rollback point:** one line in `world::bounds::build_bounds` — add `Body { .. }` to the panel
bundle. Nothing in `vector` changes either way.

**Related:** `FIND-199` · `Q-078` · `B-001` · `F-003`

---

## FIND-201 — the mission board is the first hub door a SCRIPT can walk through, and that is why it survived where four rounds died

**2026-08-28, `F-177`, `--headless` + `--offscreen` on offlinebot.**

> *„wenn man in der hub auf ein board drueckt (F) dann kommt man in eine mission uebersciht in der
> man auswaehlen kann was man machen will!"* — the user, 2026-08-27.

### What was actually wrong with the four refuted rounds, and it was never the design

`hud::hub_prompt` was refuted four times over `bearing`, `walk model`, `ray` and a sweep that held
**height** constant. Underneath all four sits one structural fact, `FIND-189`: **`--headless` and
`--offscreen` build no window, `menu` is gated on `With<PrimaryWindow>`, and no script verb can
press `Esc` or click a plate.** Every one of those rounds put its feature behind something a run on
this machine cannot reach, so its only evidence was a fixture arguing with itself — and a fixture
arguing with itself is exactly what the four refutations found.

**The board is not a screen.** It is a place in the world and one key on the keyboard, and both of
those exist in all four launch modes. `scripts/f177-board.txt` therefore drives the whole loop with
`look`, `W` and `F` and nothing else: **17 asserts held, 1623 ticks, exit 0.**

### The three decisions that make it exercisable, and each one is load-bearing

1. **`menu::board::work_the_board` is registered OUTSIDE `menu`'s window gate** — the only system
   in the domain that is. It writes `Screen` **only where there is a window**: in a windowless run
   `hud::hide_while_a_menu_is_up` would hide the whole HUD on a `Screen::Lobby` nobody can draw, so
   moving the screen there would blank the one surface such a run has.
2. **The hold is on `Time<Real>`.** With a window the overview is `Screen::Lobby`,
   `menu::apply_screen` stops `Time<Virtual>`, and every `FixedUpdate` tick with it — a hold counted
   in ticks would never finish in the run a player is actually looking at.
3. **The board adds no prop.** `maps.ron: ashgate` has stood a signpost at (3.6, 1.8, −4.2) since
   2026-08-26 and the survey photographed it; `missions.ron: hub.board` is a trigger volume on it.
   `tests/mission.rs::f177_the_board_stands_on_the_signpost_that_is_already_in_the_yard` fails if
   either file moves without the other.

### The measurements

| what | number |
|---|---|
| `scripts/f177-board.txt --headless --hub --ticks 2000` | 17 asserts held, 1623 ticks, **exit 0** |
| amber (sRGB 255, 215, 89) in the panel rect, control frame at the spawn point (`--ticks 92`) | **0** — and 0 in the whole frame |
| the same rect, standing at the board, board shut (`--ticks 129`) | **372** px, `x 52..220, y 191..239` |
| the same rect, board open (`--ticks 154`) | **2 457** px, `x 52..346, y 191..511`, **13 sortie rows** = the 13 entries of `missions.ron` |
| the cursor column `x 45..70`, open vs one more `F` (`--ticks 179`) | the one marked row moves `y 251..258` → `y 270..277`, **one line pitch**, nothing else in the column moves |
| the 3D range sweep, `tests/menu.rs` | 2 × 15³ = **6 750** samples, **0** skipped, both answers present |
| the panel's right edge against the `F-170` keep-out box | `x 346` against `x 512` |
| the shot over two runs | bit-identical, `sha256 55132df4…` |

### The one bug this round produced, and it is a shape worth keeping

**The release has to be read BEFORE the hold accumulator.** The frame a key comes up in has
`just_released` true and `pressed` false, so an accumulator block that ran first disarmed the press
and the release then found nothing to step. Symptom: **every tap did exactly nothing** — twelve
presses landed on one sortie — while the *hold* worked perfectly, so the feature looked half-built
rather than mis-ordered. Four tests went red together and named it in one run.

### And one honest correction to my own code, found by breaking it

The comment on the opening press said `armed = None` was what stopped it deploying. **It is not.**
Flipping only that line leaves every `f177` test green; the live guard is `spent = true`, and
flipping *that* turns `f177_the_press_that_opens_chooses_nothing_and_the_next_tap_steps_one_on` red
with `left: veteran, right: recruit`. The comment now says which of the two flags is the brace.
**A guard you have not watched fail is a guess about your own code**, and this one was mine.

### What a refuter should attack first

1. **The panel is a second surface for one list.** It is `menu::lobby::entries` for the rows,
   `menu::lobby::chosen` for the highlight and `shared::DeployRequest` for the deploy — one
   mechanism, and with a window the panel hides itself because `Screen` leaves `Playing`. But
   **nobody has ever seen the plate and the panel in one session**, because nobody has run this
   game in a window on either machine. That claim is 🟨 and is written down as such.
2. **`hold_s: 0.35` is untuned.** It was picked, not measured against a hand.
3. **The keyboard route is the only one a script proves.** The mouse route through the plate is
   held by `tests/menu.rs` with a hand-spawned window entity and by nothing else.

**Related:** `FIND-189` · `FIND-178` · `FIND-181` · `FIND-190` · `Q-062` · `F-177`

---

## FIND-199 — the invisible wall is a collider, so it is visible to every ray: `fence_top_m` is a **test-driven** number, and two scripts already fly under the recovery plane

**Round:** `F-012`, the map's edge, 2026-08-28. Three things were measured that a "put a box at
the border" implementation would have got wrong, and each of them cost nothing to avoid **once
measured** and would have cost a day to find afterwards.

### 1. An invisible wall is invisible to the eye and not to a raycast

The fence is a `Collider`. `vector::hook`'s probe is `space.cast_ray(..)` against **avian**, and
`AIM_RAY_SEES` is `LayerMask::ALL & !LAYER_PLAYER` — a mask over *collision layers*, which every
untagged collider is a member of. So a fence panel answers an aim ray whether or not anything
draws it.

`tests/vector_hooks.rs::f028_a_failed_pull_says_which_of_the_four_it_was` puts the player at
**`(0, 400, 0)`** with a level aim and requires `MissReason::NothingInRange`, then spawns a real
anchorable wall **900 m out on the same line** and requires `MissReason::OutOfReach`. A fence that
went to the sky would have turned **both** into `SurfaceHoldsNothing`: the same shape as `B-010`,
where a team mate in the line turned a rope into a miss.

**So `bounds.fence_top_m` is 200.0 and not `f32::MAX`, and the reason is a test and not taste.**
80 m above the 120 m wall, 200 m under the one ray in the repository that is fired from above it.
⚠️ **Anything that raises the ceiling later has to move that test or accept the collision.**
Written into `assets/data/maps.ron` beside the number.

### 2. Hiding the fence from the ray is not free either — so it carries no `Body` instead

The other half of the same problem: a rope that *reaches* the fence must not hold on to it. That
could not be done with layers (membership `LAYER_PLAYER` would make the player pass through;
anything else is in `AIM_RAY_SEES`), so it is done with the **absence of `shared::Body`**:
`vector::hook` asks `bodies.get(hit.entity)` and answers `SurfaceHoldsNothing` when the query does
not match. The same absence keeps the fence out of the `SpatialIndex` — and out of
`tests/world.rs::t036a_every_body_gets_exactly_one_id`, which asserts
`bodies_in_the_world == plan_blocks().len()` and would have gone red on four extra bodies.
**A world collider that is not a body of the world is a supported shape; it just has to be
chosen on purpose.**

### 3. Two shipped scripts already fly **200 m under the world**, and one of them is 147 m from the plane

`bounds.recovery_plane_y_m` cannot simply be "somewhere below". Measured over the whole script
corpus, two scripts use the void outside the map as a test stage:

| script | warps to | falls to, worst act |
|---|---|---|
| `scripts/f030-hitbox.txt` | `z = 600.231 … 1200.77`, `y = 13.3 … 22.1` | ≈ **-153 m** (lurker act, 3.31 s of fall) |
| `scripts/f028-why.txt` | `(400, 0.05, 406)` — 50 m outside Ashgate, on purpose | ≈ **-23 m** (1.2 s) |

A plane at −150 m would have **recovered the player in the middle of a hitbox measurement** and
the failure would have looked like a titan bug. At −300 m, `f030-hitbox` clears it by 147 m; both
scripts were run with the feature in and **neither produced a single `under the world` line**.
Both were also run with `bounds::build_bounds` and `recover_the_fallen` unregistered and reported
the identical verdicts (`8 of 8` and `2 of 6` — pre-existing reds from the provisional
`gravity_m_s2 −32` / `boost_m_s2 46`, `docs/NEXT.md` 3G), so the feature is measured to change
nothing about them.

⚠️ **The general shape, and it is the one worth keeping:** a depth-triggered rule inherits every
place the existing corpus already goes. Before choosing the depth, `grep '^warp' scripts/` and
work out where each one *lands*, not where it starts — a warp is a position, and the number that
matters is the one gravity turns it into.

### 4. And the walk-off measurement itself: the cause was the boring one of four

`docs/BUGS.md` B-015 lists four candidates (no collider past the plate · a plate smaller than the
playable area · tunnelling at speed · nothing below). Measured: Ashgate's two ground slabs cover
`x ∈ [-350, 350]`, `z ∈ [-350, 350]` — its declared `size_m` **exactly**. **One metre past the
edge is already outside the world**, at every height, and 12 of 12 probe stances fell to `y = -44.0`
in 2 s, which is free fall at `gravity_m_s2 = -32` with nothing hit. No seam, no tunnel, no
undersized plate: there was simply never anything there.

**Related:** `B-015` · `F-012` · `B-010` · `docs/QUESTIONS.md` Q-002 · `FIND-197`

---

## FIND-202 — the two ends of one rope use two different anchor positions, and only one of them is tested

**2026-08-28 · [offlinebot] · found by an adversarial verifier, re-read and confirmed by the main head**

Since `Q-078` every surface is hookable, **including titan bodies** — which was described as
`F-029 Dynamische Ankerpunkte` *"arriving as a side effect"*. It did not arrive.

**A rope's physics anchor cannot move.** `player::rope::attach_ropes` spawns a
**`RigidBody::Static`** marker at the hit point (`src/player/rope.rs:~294`) and **nothing ever
writes it again** — every other occurrence of `rope.anchor` in the repository is a read
(`anchors.get`, `positions.get` at `:429`, `:434`, `:521`, `:767`, `:812`) or a `despawn_rope`.
The file's own header says so in as many words (`:38`, `:143`): *"the rope's other end is
`RigidBody::Static` and never had one"* and *"the anchor marker does not follow a moving carrier."*

### 🔴 So one rope has two ends that disagree about where it is attached

| | rides the carrier? | who reads it |
|---|---|---|
| `Hook::tip_m` | **yes** — `entry.center_m + local_m` (`src/vector/hook.rs:461`) | `player::locomotion::air_control` → `rope_drive`, `rope_winch` |
| `DistanceJoint.limits.max` | **no** — enforced against the stale marker | the avian solver |

**The arm follows the titan; the constraint holds the ground he was standing on.** And
`hold_the_pair` (`B-013`) takes its anchor separation `d_a` from exactly those two static markers,
**so the two-rope feasibility rule's inputs are constant no matter what a carrier does.**

### And the test that claims otherwise measures the half that works

`tests/titan.rs::f029_a_rope_bites_a_walking_titan_and_rides_him` (line ~2477) compares
`tip_before` against `tip_after` **and never looks at the joint or at `Rope.anchor`.** So *"a rope
rides a walking titan"* is **proven for the arm and unproven for the physics** — and the round that
reported *"7 cm of drift while ONE ANCHOR WALKED"* was measuring a static pair. **The fixture held
constant the one variable its own sentence named.** Sixth instance of that shape this week
(`CLAUDE.md` rule 5).

⚠️ **The ledger is right and the test name is wrong.** `F-029` is marked `Unbuilt`, correctly — but
a test named `f029_..._rides_him` reads like evidence that it is built. **A test named after a
feature is not a claim that the feature exists**, and this is the reverse of the 52 rows the tree
names while the ledger says unbuilt (`docs/PLAN.md` §3).

### What it costs, and why it is not fixed here

Making the anchor follow a carrier is not a rename: a moving `RigidBody::Static` is a contradiction,
so the marker has to become kinematic and be written every tick by whoever owns the carrier — which
is a **second writer** on a `Transform` that `apply_warps` and avian already share (rule 3), and a
moving constraint against the joint the rope only got yesterday (`Q-058`). **That is `F-029`'s whole
difficulty and it is a round of its own.**

**Until then, say it plainly:** you can hook a titan, the arm tracks him, and the rope pulls you
toward where he *was*.

**Related:** `Q-078` · `Q-058` · `B-013` · `FIND-191` · `F-029` `F-004` `F-005`

---

## FIND-203 — the gear has **no ceiling**: the climb is bounded by the tank, not by the sky, so no `fence_top_m` was ever going to be the mechanism

**Measured 2026-08-28**, `W` + `ShiftLeft` held, look pitch 89, no hook and no warp, on the shipped
`ashgate` and on `graybox`:

| from a standing start | 1 s | 2 s | 3 s | 4 s | 5 s | 6 s | 7 s | 8 s |
|---|---|---|---|---|---|---|---|---|
| height | 12.2 | 49.9 | 54.6 | 71.7 | 114.5 | 182.5 | **259.3** | 336.3 m |

Gas over the first six seconds: `15000.000 -> 14891.771` — **108.229 of a 15 000 tank, 0.72 %**.

🔴 **And the 657.5 m "apex" the refutation round reported is not an apex.** It is where that
script ran out of ticks. Flown to steady state the body simply sits at `vector.max_speed_m_s`
going up: measured over a ten-second window after five seconds of spin-up,

- **748.9 m climbed for 179.88 gas = 4.163 m per unit of gas** (74.89 m/s, i.e. the clamp),
- one full tank of 15 000 therefore lifts the gear **62 508 m** on `graybox` and **62 580 m** on
  `ashgate` — the difference is only the launch pad.

So the sentence that justified `fence_top_m: 200.0` — *"far above anything the gear reaches on gas
alone"* — was not merely 3.3x optimistic, it was the wrong **kind** of claim: **there is no height
at which a fence stops being flyable-over**, and every candidate height brings its own standable
ring on its top face (`B-016`, symptom 3).

**The consequence, and it is the design decision:** the fence is a *horizontal* mechanism and is
sized for normal play (80 m over Ashgate's 120 m wall, so walking and swinging along the coping
meet a wall and not a teleport). What closes the world is
`player::recovery::out_of_the_world` — **outside the map's own footprint is out of the world at any
height.**

**The measurement is now pinned in the data** rather than in a comment:
`assets/data/maps.ron: bounds.gear_ceiling_m` (62 508 / 62 580) and
`tests/player.rs::f012_the_gear_climbs_higher_than_the_fence_and_the_number_is_pinned`, which
re-flies it on both maps from both the ground and the map's tallest standable block and holds it to
5 %. Lower `vector.gas_tank` or raise `vector.boost_m_s2` and that test says so, with the new
number in the failure message.

**The habit this is an instance of:** the old comment argued a bound instead of measuring one, and
the argument was cheap enough to check in ninety seconds of simulated flight. **A number in RON
whose justification is a sentence about physics is a number nobody measured.**

**Related:** `B-016` · `B-015` · `FIND-199` · `F-012` · `F-007`

---

## FIND-204 — foreign territory: three scripts use "outside the map" as an empty stage, and the footprint rule now recovers them on the next tick — ✅ CLOSED 2026-08-29 (all three re-aimed onto the boulevard; 0 warps each; see `FIND-207`)

**Not fixed here** — `scripts/` outside `f012-edge.txt` was not this round's to touch. Recorded so
it is not discovered as a mystery.

Since `B-016`, `player::recovery::out_of_the_world` returns `PastTheEdge` for any body outside
`map.size_m`, at any height, and `recover_the_fallen` warps him back on the next tick. **Three
scripts deliberately `warp` out there** to get a titan alone against an empty sky (grep of all 71
scripts, 9 warps in total):

| script | line(s) | stance |
|---|---|---|
| `scripts/f028-why.txt` | 22 | `(400, 0.05, 406)` — 50 m outside Ashgate, on purpose |
| `scripts/f030-hitbox.txt` | 94, 105, 117, 128, 153, 162 | `z = 400.55 .. 1200.77` |
| `scripts/f032-swords.txt` | 125, 138 | `z = 450` and `z = 600` |

None of the three carries a positional `assert` at those stances, so **none of them fails** — which
is worse: they keep exiting 0 while measuring a player who has been teleported home. `f030-hitbox`
is the expensive one; `FIND-199` already noted it falls to about -153 m during its longest act.

**The repair is one coordinate per warp, and Ashgate has the room for it**: the open field inside
the fence reaches `|x|, |z| <= 350`, and the district itself stops well short of that — `z = 300`
with `x = 0` is as empty as `z = 600` was, at any of the heights those scripts use (13 .. 22 m).
The alternative, `--sandbox` (*"empty field, one titan, infinite gas"*), is the stage those acts
were reaching for in the first place.

⚠️ **Do not "fix" this by weakening the footprint rule.** A carve-out for `warp` is a carve-out for
the exact command the user's bug arrives through, and the grace it would need is a standable ring on
top of the fence (`B-016`).

**Related:** `B-016` · `FIND-199` · `F-012` · `F-028` `F-030` `F-032`

---

## FIND-205 — "move the fence off the boundary" does **not** close the ring: a capsule rests on its bottom sphere, which reaches `radius_m` over the lip

**Measured 2026-08-29**, and it refuted the obvious fix inside twenty minutes of running it.

`B-017`'s defect is that the fence's inner lip stood exactly on `hx` while
`out_of_the_world` tests `|x| > hx` strictly. The natural fix is *"stand the fence strictly
outside the footprint, then the strict `>` already recovers from every point of it"*. **It is
wrong, and the size of the move is not free.**

A capsule does not rest on the point under its origin. With `fence_margin_m: 0.0`:

| put at | came to rest at |
|---|---|
| `(349.999, 201, 0)` — a **millimetre inside** the map | `(349.938, 199.994, 0)`, on the fence's top face |
| `(350.000, 201, 0)` | `(350.0, 199.99994, 0)` |

**48 of 48** stances in `f012_the_ground_he_is_sent_back_to_is_never_ground_he_can_be_sent_back_from`
left `SafeGround` pointing at the face. Moving the fence out by one ULP (3.05e-5 m at 350 m)
would have changed **nothing**: the number to clear is not the float grid.

### The bracket, and both ends are measurements

```
   fence_rest_reach_m  <  fence_margin_m  <  player.radius_m
        0.10 m               0.18 m              0.35 m
```

- **Below**, how far back over the lip a body can **park** — come to rest and still be there ten
  seconds later. Measured **0.0000 m** over 44 stances: past the lip the contact normal tilts too
  far for friction and every body slides off into the map. Held at 0.10 as budget, because
  friction is avian's number and not ours.
- **Above**, `player.radius_m`: the solver holds a body pressed against the fence's inner face
  exactly a radius inside it — **-0.3500 m** relative to the map edge at `vector.max_speed_m_s`,
  to four decimals, so its penetration at the clamp is zero. At or over 0.35 a legitimately
  fenced-in player is outside the footprint and gets teleported for playing.

0.18 is the geometric middle: either bound has to move by ~1.9x before it bites.

### ⚠️ The corollary that cost the most time: **geometry alone is not the fix**

The window is only 3.5x wide, and it does **not** cover being `Grounded`: a body sliding off the
lip stays `Grounded` the whole way down, because its bottom sphere keeps touching the box's edge.
So `record_safe_ground` took stances **0.2590 m** back over the lip, at y = 199.886 — 200 m up
with nothing under them. The margin cannot close that; the recorder has to. It now judges
`recovery_destination(p) + one body radius outward` — **the whole body at the place he would be
put down** — because the origin alone cannot tell a stance on the map's own ground from a stance
on the fence's lip: the capsule that could be resting on either is the same size.

⚠️ **And the control run deleted the other half of that fix.** A velocity gate — *"refuse a
stance he is falling past"*, `|vy| > gravity_m_s2 / simulation_hz` — was written against the same
measurement and then could not be made to go red in **any** legal configuration, because the body
condition already covers every stance it would have refused. It was removed rather than shipped
(`CLAUDE.md` rule 5: a fix without a red test is a guess). The body condition itself is paid for
by `f012_at_the_smallest_legal_fence_margin_the_recorder_still_keeps_nothing_on_the_fence`, which
goes red at **4 of 20** with one line reverted.

## FIND-206 — the fourth `continue`: **64 of 648 skipped samples WERE the defect**, and the fixture asserted that it had reached them

`CLAUDE.md` rule 5's fourth shape, now measured for the second time on the same feature.

`tests/world.rs::f012_the_whole_top_face_of_the_fence_lies_outside_the_map_and_is_recovered_from`
swept every fence panel's top face and `continue`d past every sample landing on the inner line
`|coord| == size_m / 2`, counting them as `on_the_line`. It justified the skip with an unmeasured
sentence — *"A body cannot rest on a line"* — and then **asserted `on_the_line > 0`**, as if
reaching the defect and stepping over it were proof of coverage.

Deleting the two `continue`s: **64 of 648 (9.9 %) fail**, and they are exactly `B-017`. The
sentence was false: a body put on that line rested there for ten seconds and could be walked on.

Its sibling `tests/player.rs::f012_the_top_of_the_fence_is_not_a_ring_you_can_stand_on_outside_the_map`
dropped bodies at `across ∈ {0.5, 3.33, 6.67}` m **outward only, never at 0**, under a comment
that said *"What it skips: nothing."* It now samples `{-1 mm, -1 ULP, 0, +1 ULP, +1 mm, 0.5, t/3,
2t/3}`, 24 -> 64 stances.

**The habit:** when a rule is an inequality, the sweep's first three samples are the boundary
itself and one ULP either side. A `continue` that names a sample class is a claim about that
class, and a claim needs a measurement.

### And a fifth shape, cheap and embarrassing: **`f32::signum(0.0)` is `1.0`**

A bearing vector written `Vec3::new(lip.x.signum(), 0.0, lip.z.signum()).normalize()` comes out
**diagonal** for the four flat bearings, so a sweep that says "+x" measures a corner. It reported
a 0.3721 m overhang that was 0.7071 of a corner distance, and an earlier 0.0892 m "friction
reach" that was the same artefact. The form that works, and the one the rest of the F-012
fixtures already used, is `x.signum() * x.abs().min(1.0)`.

## FIND-207 — foreign territory: `scripts/f028-why.txt` measures a verdict the game no longer gives — a titan **anchors** a hook now

Re-aiming `f028-why.txt` (see `FIND-204`, now closed) put its act 2 back where it was written for
— husk 6 m in front, on the boulevard — and the run reads

```
hook Left of player 1 left the hand: 6.20 m, 1 ticks to the anchor
hook Left of player 1 anchored on body 2902 at 0.00 5.64 -88.50
```

i.e. **anchored on the titan**. The script's whole premise is the opposite:
*"a titan carries no `shared::Body`, so the ray ends on a surface that holds nothing —
SurfaceHoldsNothing"* (`B-007`). `F-024` retired the anchor field in favour of surfaces, and the
titan appears to have gained a `Body` with it. Not touched here: `src/vector/**` and
`src/titan/**` are another stream's. Either the script's expectation is stale or `B-007` has
silently reopened in the other direction, and one of the two has to be written down.


## FIND-211 — the map edge's loose ends: a fixture that was being teleported mid-measurement, and a script whose premise expired ten days before it was re-aimed

*(claimed 2026-08-29, stream B — being written)*

---

---

## FIND-210 — the district was flat because a **flight of stairs was pointed the wrong way**, and that capped `step_m` at 1.80 m

**Measured 2026-08-29.** The user, 2026-08-27 and again today: *„es soll auch noch verschiedene
hoehen geben. aktuell sit alles flach von der map."* — and asked which he meant, *„Das Gelaende
selbst — Huegel, Terrassen"*.

### The cause is one bound, and it was never written down anywhere

A falling terrace edge used to be a full-length stepped **bank** laid across the street. A bank
is bounded by the street it crosses: a house stands `street_m / 2 - cell_jitter_m` = 1.50 m back
on **each** side, so the run is at most 3.00 m, and at the 0.60 m tread a 0.30 m riser needs,

    step_m <= (3.00 / 0.60 + 1) * 0.30 = 1.80 m

**1.80 m per level was a hard ceiling, and no amount of tuning `maps.ron` could pass it.**
`FIND-134` had already ruled out the renderer (the retaining-wall/cap split moved **5 of
921 600 pixels**) and correctly named the 3.6 % grade as the cause — but read it as a number
somebody had chosen, when it was a number the geometry had chosen.

**Turn the flight ninety degrees and the bound changes.** A flight that runs *along* the wall is
bounded by the cell's **length** (42 m) instead of the street's width (3 m) — fourteen times the
room. The edge becomes a **retaining wall** with one stair cut into it, which is what a terraced
town actually has.

| | before | after |
|---|---|---|
| `terrain.step_m` | 1.50 m | **3.00 m** |
| peak relief | 7.50 m | **12.00 m** (= `scale.ron: house_large` 11.50) |
| ground under the houses, p10→p90 | 0.00 → 4.50 m | **0.00 → 9.00 m** |
| mean cell height | 1.69 m | **3.26 m** |
| terrace blocks | 1236 | **2337** (+89 %) |
| **aerial pixels changed, delta > 3** | *(FIND-134: 5)* | **249 979 of 921 600 (27 %)** |

Same 1280x720 vantage FIND-134 used (`f003-map-aerial`, `warp 150 45 60`, `look 48 -17`).

### ⚠️ The trap: raising `levels` makes the district FLATTER

`sky_m = step_m * (levels - 1) + door_height_m` is the line above which a hand-placed block stops
pinning its cell flat. Raise it and blocks that were **cut around as pillars** become **pins**
instead. Measured over the shipped map at `step_m` 1.5: `levels` 6 → 8 pins two more cells and
drops the peak **7.5 → 6.0 m**. `levels` stays 6.

**And the ceiling was never the constraint anyway:** the field is a distance transform from the
86 pinned cells and the grid rim, and it is already **saturated** — the shipped levels are
identical, cell for cell, to the pure transform computed with the ceiling set to 25. The only
lever on this map is `step_m`.

### The control that says the walk did not get harder

`stair_rise_m` did not move (0.30). Same route, same binary, only `gravity_m_s2` differing:
**-20 → 9.000 climbs to 11.837 m** (nine risers, on foot); **-32 → 9.000, not one riser**. That
is `B-018` unchanged — the new flight climbs at exactly the gravity the old bank climbed at and
fails at exactly the one it failed at.

### What it cost to find out, and the rule

A first pass concluded "the new stairs are a walkability regression" from a route that walked
**east** — and this harness cannot move a body along X at all while reporting `speed 6.000`
(**`B-022`**, found here). The conclusion was confident, reproducible and wrong.
**Before believing a body did not climb, prove it moved**: read a coordinate that must change,
not the speed, which reads full running speed into a wall.

## FIND-212 — the marker was 16.00 px below the cursor, always, and every test that "proved" it was 0.0 px measured the projection instead of the pixel

**Round:** 2026-08-29 · **Owner:** agent A · **Stage:** 🟧 (measured before and after, image + numbers + a test that goes red on one line)

The user, twice — 2026-08-19 *„wichtig wäre nur dass diese auch genau da sind visuell wo das seil
auch landen würde!"* and 2026-08-29 *„ist immernoch nicht am cursor. es bewegt sich immernoch."*
`F-026` stood at 🟧 on `FIND-129`, which measured **0.0 px on fire and on anchor**, and an
independent adversary re-derived it. **Both were right about different things.**

### 1 · What the old evidence measured, and what it could not see

`tests/hud.rs::f171_a_free_aim_point_projects_onto_the_crosshair` computes `eye + look_dir * d`,
projects it with `world_to_viewport`, and asserts the result is the centre pixel. It **never reads
a `Node`.** It proves the *projection* is exact — which it is, to 0.00 px, even through a 360 °/s
flick. It cannot see where the **glyph** was drawn, and the glyph is the element.
`CLAUDE.md` rule 5, provenance shape: *two computations agreeing about a point the test invented.*

### 2 · What the drawn pixel actually was — a per-frame trace through a real turn

`scripts/f026-turn.txt` (new) turns one `look` per tick; `hud::arm_aim::trace_arm_aim`
(`DBT_AIMTRACE=1`) prints, per frame and per arm, the world point, its projection, and the glyph
centre read back off the `Node`. **3 268 samples, Left arm, 1280 × 720, ~3.9 frames per fixed
tick.** `dproj` = point vs crosshair · `dglyph` = drawn glyph vs crosshair · `dgp` = glyph vs its
own point.

| phase | n | dproj med / max | **dglyph med / max** | dgp max |
|---|---|---|---|---|
| still, 30 ticks | 119 | 0.00 / 0.00 | **16.00 / 16.00** | 16.00 |
| slow pan, 120 °/s | 237 | 0.00 / 0.00 | **16.00 / 16.00** | 16.00 |
| flick, 360 °/s | 158 | 0.00 / 0.00 | **16.00 / 16.00** | 16.00 |
| boosting, 120 °/s | 473 | 7.02 / **46.53** | 16.88 / 46.53 | 16.00 |

**Within one fixed tick the drawn marker does not move at all (spread 0.0000 px over 120 ticks).**
There is no schedule race and no sub-frame jitter: `place_arm_aim` is already in `PostUpdate`.

`16.00` is `SIGHT_CORE_PX` (6) + half a 20 px glyph, to the pixel — `layout_for` step 3, the
"stand-down" FIND-129's adversary had already computed as 17.7 px and filed as *documented*.
**A skip you are proud of is still a skip.** After removing step 3's `Some(p)` arm: `dgp` **16.00
→ 0.00** in every phase, `dglyph` **16.00 → 0.00** standing, turning and flicking.
Image: `docs/images/f026-marker-at-cursor.png` (t = 140, same script, pinned binaries; 632 changed
pixels of 921 600, bbox 612,350..665,386). The ring is a 3 px annulus with a hollow ~14 px
interior, so the crosshair's own 6 px hole shows straight through it — measured off the frame.

### 3 · The remaining drift is NOT the layout, and it is not fixed: `vector::aim` is one fixed step stale

The `moving` row above is untouched by the fix (the frames at t = 349 are **bit-identical** before
and after — at 46 px out the marker was never inside the sight core, so step 3 never fired there).
`vector::aim::aim` runs in `FixedUpdate` / `SimulationSystems::World`, i.e. **before** `Integrate`;
the HUD projects its answer from the camera at the **end** of that step. One whole step of eye
travel separates the ray's origin from the projection's origin, and the error is `v/60/d`:

| eye speed | target distance | dproj |
|---|---|---|
| 18.3 m/s | 40.2 m | 0.73 px |
| 27.2 m/s | 10.4 m | 16.36 px |
| 29.4 m/s | 5.7 m | **39.29 px** |

**The one-line candidate:** move `aim` from `SimulationSystems::World` to `PostStep` in
`src/vector/mod.rs:64`. `vector::hook` reads `ArmAim` in `Drive`, before `Integrate`, so it would
then read a point cast from the position it is still standing at — **exactly as fresh for the
rope, and exact for the picture.** Not done here: `src/vector/mod.rs` was not this round's, and a
schedule move deserves its own red test.

### 4 · And a hole in the evidence route itself

`--offscreen` **without** `--screenshot` never swaps the camera's `RenderTarget`
(`debug::screenshot::swap_target` only fires for a screenshot job), so
`Camera::logical_viewport_size()` is `None`, `place_arm_aim` returns on its first guard, and the
markers sit on `spawn_arm_aim`'s placeholder percentages (34 %/64 %, top 50 %) for the whole run —
silently, exit code 0. Any offscreen run that is not also taking a picture measures nothing about
where a marker is.

### What the fix costs, said out loud rather than discovered later

`F-170`'s sight-core clause is **retired for a marker that carries a place** — that is a proven
🟧 claim being pulled back on his instruction, and the rollback point is one arm of one `match`
(`hud::arm_aim::layout_for`, step 3) plus five assertions that now say the opposite
(`tests/hud.rs::f170_nothing_covers_the_middle_of_the_screen`,
`f170_the_arm_markers_stay_out_of_the_middle_in_every_state`,
`f170_an_anchor_dead_ahead_stands_on_the_anchor`, and the two `src/hud/arm_aim.rs` unit tests).
**Measured, not argued, for `Ready`/`Anchored`:** the glyph is a 3 px annulus and its interior is
empty across x 633..647 in the shipped frame, so the crosshair's own 6 px hole shows straight
through it.
⚠️ **`Free` is the case that really does cover the sight now**: its glyph is a 20 × 4 flat dash,
so an idle marker with nothing hookable under it lies across the middle ±10 px × ±2 px. Nobody
has seen that in play yet. If he says the dash is in the way, the answer is a **hollow** `Free`
glyph, not the 16 px back.

### The rule

**A test that never reads the drawn `Node` is not a test of a drawn element.** Project and glyph
are two different numbers; the player reads the second one. And when a fixture holds a rule
constant by *skipping* the case it fires on — here: never measuring the pixel — that skip is the
first place to look.

---

## FIND-213 — the kill zone was **inside** the body he collides with, on all 8 kinds — and the obstacle scaled with the titan while the blade did not

*(2026-08-29, stream B. His words: „die hitboxen passen auch nicht. ich komme nicht an ein titan
ran zum hitten!" and „die aktuellen sind eher kleiner! gerne doppelt so gross oder so. und
leichter hittable am nacken.")*

### The measurement

A titan's physical collider was **one capsule of SHOULDER half-width** (`width_fraction 0.25 ×
height / 2`) running from the ankles to the crown. At nape height the head is `head_m / 2` =
`0.055 × height`, so the solid was **2.27× wider than the head it wrapped**. The amber sphere
sits `head_m / 2` behind the neck axis, so on every kind its rearmost point was **inside** that
capsule:

| kind | class | h | cortex back | body there | exposure |
|---|---|---|---|---|---|
| weaver | small | 4.2 | 0.461 | 0.524 | **−0.063** |
| scuttler | small | 4.2 | 0.431 | 0.524 | **−0.093** |
| husk | medium | 10.0 | 1.100 | 1.241 | **−0.141** |
| errant · chorus | medium | 10.0 | 1.050 | 1.241 | **−0.191** |
| warden · lurker | large | 14.0 | 1.540 | 1.732 | **−0.192** |
| bellower | huge | 21.0 | 2.315 | 2.605 | **−0.290** |

**1202 of 1221** measured (kind × class × a 2..120 m continuum) were buried. A cut only ever
landed because `blades::cut::sweep` casts `LAYER_TITAN_CORTEX` and `LAYER_TITAN_BODY`
**separately** — the blade reaches the nape by passing *through* solid titan, which works exactly
as long as `reach_m` 2.00 is longer than the titan is wide. That is the scaling trap: the
obstacle is `0.125 × height`, the blade is a constant. **Doubling the titans as he asked made the
`large` class hittable on 0 ticks — unkillable at every offset.** (Control below.)

### The second half, and it is the one the previous two rounds missed

`q030_the_nape_is_reachable_on_a_large_titan_too` reported **+0.87 m** of reach margin for the
same kind `scripts/f030-hitbox.txt` measured as **one simulation step**. Both were honest: the
margin is a bound **along** the blade, the window is the chord **across** it, and at the edge of
the first the second is zero. Swept tick by tick at 21 m/s, the shipped windows were

```
before   scuttler 1 · weaver 1 · chorus 2 · errant 2 · husk 3 · lurker  3 · warden  3   ticks
after    scuttler 3 · weaver 3 · chorus 7 · errant 7 · husk 8 · lurker 10 · warden 10
```

**Seven of seven kinds inside three ticks, and a single green pass per kind cannot tell 1 from
11.** Two 🟧 rounds of hitbox work (2026-08-24, 2026-08-26) both grew the *weapon* — pass width
0.20 → 0.80 m, `reach_m` 1.6 → 2.0, `thickness_m` 0.12 → 0.20 — and neither ever measured a
window.

### The fix: **a titan has a neck** (and it is the same change as the size he asked for)

`titan::rig::body_segments_m` is now the one writer of the body's width at a height, and it
returns two capsules: a torso ending at `shoulder_m`, and a neck+head at `head_m / 2`. The nape's
set-back **is** the neck's radius, so **half the sphere protrudes at every size, on every kind, by
construction and not by tuning**. And because the shoulder is now `0.07 × height` below the nape,
a 1.8 m player fits into the pocket beside the neck the moment `0.07 × height > 1.25 m`, i.e.
above ~17.9 m. **Doubling the classes is what opens that pocket** — his two sentences are one
change:

| | before | after |
|---|---|---|
| class heights | 4.2 / 10 / 14 / 21 / 28 | **8.4 / 20 / 28 / 42 / 56** |
| `cortex_radius_m` | 0.20 … 1.16 | **doubled, 0.40 … 2.32** |
| `width_fraction` | 0.25 | **0.20** — see below |
| clearance at the nape, `large` | 2.10 m (shoulder) | **1.89 m (neck), at twice the height** |
| cortex exposure, all 1221 geometries | **−0.06 … −1.74 m** | **exactly `+cortex_radius_m`, every kind, every class** |
| reach margin, tightest kind | +0.30 m (a `large`) | **+1.41 m (a `small`)** — the ordering inverted |
| window, ticks at 21 m/s | 1·1·2·2·3·3·3 | **3·3·7·7·8·10·10** |

The exposure column is the structural part and it is not tuning: the nape's set-back **is** the
neck's radius, both `head_m / 2`, so the sphere's centre lands on the skin and exactly
`cortex_radius_m` of it protrudes — at 8.4 m and at 56 m, on all eight kinds, in one arithmetic.

**`width_fraction` 0.25 → 0.20 came out of the doubling, not out of taste.** `0.25 × 28 m` is a
7.00 m body in a 6.0 m street and `tests/data.rs::t005_the_class_cap_names_a_class_that_exists…`
said so — the guard whose own comment had warned *"at `boss` (28 m) it is exactly 7.0 m and the
alley is blocked by a silent wall"*, one class early. Of the levers that could answer it, the
class heights are HIS (given today), `min_street_m` 6.0 is HIS (2026-08-09), and dropping
`max_spawnable_class` to `medium` would delete the warden and the lurker to fix a width. **This
number is the only one on the table the file itself marks UNTUNED**, and it does not touch the
nape: the `large` clearance is 1.890 m at either value, because it is the neck.
**ASSUMPTION:** he wants the size more than he wants the silhouette. **Rollback:** put 0.25 back
and `max_spawnable_class` down to `"medium"`.

Evidence: `scripts/f030-hitbox.txt` **8 asserts held**, all seven kinds killed at the nape at
double size · `docs/images/f030-nape-kill-after.png` (KILL 20.8 m/s on a 28 m lurker) ·
`docs/images/f030-size-after.png` · `tests/titan.rs::f030_the_nape_sticks_out_of_the_body_on_every_kind_and_every_class`
and `::f030_the_nape_is_hittable_for_more_than_one_tick_on_every_spawnable_kind`.

**Control (rule 5, second half):** one line of `body_segments_m` back to a single capsule →
exposure red on **1173 of 1221**, and lurker and warden read **0 ticks, a 0.00 m band, blade tip
+2.01 m against a 1.74 m sphere** — i.e. the class he most wanted to see would have been
unkillable at every offset and every timing.

### Foreign territory, not touched here

1. 🔴 **`gear.ron: blades.thickness_m` is 0.20 and its own comment argues 0.30 — four times.**
   `git log -S` says the value went `0.12 → 0.20` in `54fd93b` while that commit's own text and
   the shipped comment both say *"0.12 -> 0.30"* and compute windows (`±0.50 m`, `±0.85 m`) for a
   build that never existed. The shipped windows are `±0.40` and `±0.75`. It is exactly the
   missing tick on the `small` class: at 0.30 they measure **4** ticks instead of 3.
2. **`titan.ron: attack_range_m` did not scale** (6.0 / 8.0 / 2.5). A 20 m husk has an 8.8 m arm
   and stops 6.0 m from the player's centre — 3.5 m of that is his own body. Deliberately left,
   because it moves `B-020`'s premise and `scripts/f032-swords.txt` is red on his account.
3. **`scale.ron: max_spawnable_class` stays `large`.** Doubling did not touch the bellower gate
   (`Q-028`); `huge` and `boss` are covered by the shape test and by nothing that runs.

### The rule

**A hit zone that does not protrude from the collider is not a hitbox — it is a hole the cast
happens to reach through.** And when one number bounds a reach and another bounds a window, a
green pass measures neither: **sweep the axis the fixture takes from a single call.**

---

## FIND-214 — the ground: 42 m terraces became a 5 m height field, and the one number that bounds all of it was measured in eleven runs

**2026-08-29.** The user, after playing: *„auch die verschiedenen hoehen passen nicht! das soll
grass sein und nicht so wie jetzt! und nicht verschiedene hardcoded stufen sondern wirklich
terrain! und deutlich hoeher und niedriger als jetzt!"*

### 1 · The measurement everything else hangs from: what riser can he actually walk up?

`docs/BUGS.md` B-018 had one data point — a 0.30 m terrace riser is a wall at `gravity_m_s2:
-32`. That is not enough to design a field with. Eleven risers, ten steps each, `key W` from
flat, shipped binary, tread 3.0 m:

| riser m | 0.10 | 0.15 | 0.20 | 0.25 | 0.26 | 0.27 | 0.28 | 0.29 | 0.30 | 0.40 | 0.80 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| climbed m | 1.00 | 1.50 | 2.00 | 2.50 | 2.60 | 2.70 | **0.28** | 0.00 | 0.00 | 0.00 | 0.00 |

**0.27 m climbs, 0.28 m is a wall.** Treads of 1.0, 2.0 and 3.0 m at riser 0.25 all give the
full 2.50 m, so the *tread is not a factor* in that range and the riser is the whole
discriminator. ⚠️ The first version of this sweep read **0.000 for every riser including
0.10** and would have concluded that no step is climbable at all: the staircases were 33 m long
and the walk was 63 m, so he crested each one and fell off the far end onto the flat. The fix
was a plateau at the top — **a sweep that reports the same number for every input is measuring
the fixture, not the function.**

### 2 · What that one number buys, and why it decides the cell size

The steepest ground a cuboid world can have is `rise_m / cell_m`, because two neighbouring
cells always meet in a riser. With `rise_m` capped at 0.27 the cell is the only free variable,
and it costs blocks quadratically. Shipped: **`cell_m 5.0`, `rise_m 0.25` = a 5.0 % grade.**

| | terraces (before) | field (after) |
|---|---|---|
| cell | 42.0 m | **5.0 m** |
| quantum | `step_m` 1.50 m | `rise_m` **0.25 m** |
| grade | 3.6 % | **5.0 %** |
| relief | 0.00 .. 7.50 m | **-12.00 .. +8.25 m = 20.25 m** |
| distinct heights | 6 | **82** |
| ground blocks | 1 236 | **6 614** (19 600 cells, greedily merged) |
| map total | 2 901 | **8 278** |
| seed moves | 1 cell of 256 (FIND-101) | 27.6 .. 63.6 % of cells |

The shape comes from a two-sided envelope, not from the noise: `hi = rise * L1 distance to the
nearest pin`, `lo = -rise * L1 distance to the nearest pin or paving`, the noise clamped between
them and then lowered until no two neighbours differ by more than one rise. Pins land at exactly
0 under every seed, which is what keeps 243 hand-placed blocks standing where the file puts
them without one of them moving a centimetre.

### 3 · The pixels, because FIND-134 called 5 of 921 600 invisible

Same vantage, same script (`scripts/f003-map.txt`), pinned binary vs new:

| | changed > 2 | changed > 32 | green pixels |
|---|---|---|---|
| aerial (t=140) | **397 026 (43.1 %)** | 240 059 (26.0 %) | 5 -> **121 093** |
| street (t=129) | 60 030 (6.5 %) | 44 833 (4.9 %) | 0 -> **6 133** |

### 4 · Two things this round learned the hard way and neither is about terrain

**a · A colour slab can be a load-bearing apron.** The three `olive_green` field patches outside
the wall look like paint. Deleted together with the sand floors, the district grew by **146
houses** (898 -> 1044 facades) out there, because `plan_blocks` vetoes every generated house that
touches a placed block. They were put back. `grep` for a block's colour tells you nothing about
what it does.

**b · B-022's blanket sentence does not hold on the route this round needed.** It says *no script
in this repo can walk a body along X*. Control on the shipped binary, `warp -340 40 210`:
`look -90 0` + `key W` reads **-8.000 -> -5.100 -> -2.250**; the same run with `key W` deleted
reads **-8.000 -> -8.000 -> -8.000**; `look 0 0`, `look 90 0` and `look 180 0` each give a
different trace. Four yaws, four answers, and the control moves the number — so +X is reachable
and the yaw is obeyed. The likely cause of B-022 is a sign: it labels `look 90 0` as +X while
`scripts/f003-map.txt` and every other comment label `look -90 0` as +X, so its "+X" leg walked
into the neighbour it thought it was walking away from. Foreign territory (`src/debug`,
`src/input`), so it is recorded and not fixed.

### The rule

**When a cuboid world has to look continuous, the quantum is not a taste — it is whatever a body
can walk over, and that is one afternoon of measurement.** Everything else (cell size, relief,
block count, whether the round is affordable at all) is arithmetic on it. And before believing
any sweep of that quantum: check that the fixture is long enough for the biggest input, or every
row reports the same number for the same wrong reason.

### 5 · What the ground moved under, and it is three brackets in three files nobody owns here

Corpus, HEAD binary + HEAD assets against this round's, both through `tools/corpus.sh`:
**56 GREEN / 12 RED -> 46 GREEN / 22 RED.** Eleven scripts went red, and eight of them come
back GREEN the moment `assets/data/scale.ron` and `assets/data/titan.ron` are reverted to HEAD —
they belong to the titan-size round running beside this one, not to the ground
(`f029-grapple`, `f034-hitstop`, `f070-hub`, `f175-loop`, `f177-board`, `f-flight-cut`,
`game-full`, `q030-reach`). `w2-terrain-walk` went RED -> GREEN.

**Three are the ground's, and all three are the same one-line re-derivation:** a bracket that
pinned `1 * step_m = 1.500 m`, the height of a terrace that no longer exists. The claim in each
is *"he is standing on the ground and not through it"* and it is untouched; only the number
moved. Not edited here — none of the three files belongs to this round:

| file | line | reads | was |
|---|---|---|---|
| `scripts/f003-ground.txt` | 30 | `assert height > 1.0` | **-1.500** |
| `scripts/f171-crosshair.txt` | 120 | `assert height > 1.40` | **-0.058** |
| `scripts/q078-fling.txt` | 62 | `assert height > 1.0` | **-1.000** |

`f171-crosshair`'s own comment is the clearest statement of the pattern and of why this is a
re-derivation and not a loosening: *"The claim is unchanged: the stand is bare ground, not a
lot. Only the ground moved, so only the number did."*

### What the doubling cost the corpus, measured with a control

Every script that aims at a titan by absolute metres had to be re-derived, and **the whole
correction is one substitution** — the drop each file chose is untouched, only the husk moved:

```
warp y  += cortex_height_m 17.80 − 8.90 = 8.90
x offset = clearance 1.55 + the air that file already chose     (was 1.60 + air)
z offset = set-back 1.10 + whatever that file already added     (was 0.55)
```

Controlled by reverting **only** this stream's ten files and rebuilding, so the terrain stream's
own reds do not land in this column:

| | control (my files reverted) | after |
|---|---|---|
| corpus | 54 GREEN / 14 RED | **54 GREEN / 14 RED** |
| broken by the doubling and repaired | — | `q030-reach` · `f029-grapple` · `f034-hitstop` · `f175-loop` · `f177-board` · `f070-hub` · `game-full` |
| broken by the doubling and **NOT** repaired | — | 🔴 `f-flight-cut` |
| red on purpose that went green | `f032-swords` | see `docs/BUGS.md` B-020 |

🔴 **`scripts/f-flight-cut.txt` is handed on red, deliberately.** Its pass is not a fall but a
**reel up a rope past the nape**, so three things are coupled that the substitution above treats as
independent: the anchor beam's position, the height asserts along the climb, and the gas ledger the
file pins. Shifting the slash by the extra `8.90 / 28 = 0.318 s` of climb makes the cut land — on
`ArmLeft` and `Torso`, because at the lower part of the climb the player's fixed `x = −4` is now
**0.25 m inside** a torso capsule of radius 2.00 m (it was 1.25 m). The fix is to move the stand
out to `titan_x − 2.60` and re-measure the gas bounds, and that is the rope stream's file and the
rope stream's ledger. Diagnosed, not guessed: `1 of 25`, `assert titans == 0` at line 277.

---

## FIND-217 — the marker moved because the ray was cast from **last tick's eye**, and the instrument printed `none` for its own worst case

*(measured 2026-09-01 · `src/vector/mod.rs:88` · `src/hud/arm_aim.rs::trace_arm_aim` ·
`scripts/f026-turn.txt` · pinned binaries, one script, one machine)*

He said it twice: *„es bewegt sich immernoch also die target seile"*. FIND-212 removed a constant
16 px stand-down and left the moving half open. This is the moving half.

### 1 · The instrument first, because without it the number is over the wrong set

`trace_arm_aim` took its projection with `.ok()`, so **every point `Camera::world_to_viewport`
refuses — behind the near plane — printed the literal `none`** for `proj`, `dproj` and `dgp`. That
is exactly the sample where the marker is clamped to a screen edge and the error is largest.
Measured on `scripts/f-001-hooks.txt --ticks 400`: **347 of 1650 samples**, the worst of them
standing **350.00 px** from the crosshair with nothing printed. It now takes the same
[`edge_pixel`] fallback `place_arm_aim` takes and says which of the two it was in `clamp=`.
**A sample that carries no number leaves the denominator, and `0 of N` over the survivors is
arithmetic about the wrong set.**

### 2 · The defect: `aim` ran one whole fixed step ahead of the camera that draws its answer

`vector::aim` casts from `translation + Y·eye_height_m` in `SimulationSystems::World`, i.e.
**before** `Integrate`. `render::attach_camera` hangs the camera on the player at exactly that
offset, and the HUD projects the answer in `PostUpdate` — from the position **after** `Integrate`.
The error is one step of eye travel expressed as an **angle**, `v·dt/d`, so it diverges as the
player closes on the surface he is aiming at. At t = 432 the aim point is a wall **0.35 m** ahead
of an eye moving at 29.4 m/s: `tgt=45.113,0.850,32.128` from the previous eye, `proj=220.02` —
**419.98 px** from a crosshair at 640.

`scripts/f026-turn.txt`, Left arm, whole boost `t = 252..435`, 723 / 679 samples, pinned binaries:

| `dglyph` | median | p95 | max | worst single-frame jump |
|---|---|---|---|---|
| `World` (before) | 14.00 | 48.74 | **419.98** | **392.92** (t 431 → 432) |
| `PostStep` (after) | **0.00** | **0.00** | **0.01** | 0.01 |

The still, slow-pan and flick phases read 0.00 both ways — **the fixture that could see this had
to move the player**, and every earlier measurement of this element was taken standing still.

**The control that makes the table attributable:** over the **384 ticks both runs share**, `eye`
and `fwd` are printed identically to every decimal — 0 ticks differ on either — while `tgt`
differs on **182**. The schedule move changed the aim and provably nothing about the simulation,
so the pixels above are the aim's and not a physics divergence between two binaries.

The picture: `docs/images/f026-marker-in-flight-before.png` and `f026-marker-in-flight.png`
(`--ticks 432`, same script, both pinned binaries). **626 pixels differ**, all in rows 350..369:
the letter and the 20 px ring leave `x 192..229` and arrive at `x 612..663`, and the band
`x 600..680, y 350..370` holds **no cyan at all** before and `maps.ron`'s exact (63, 237, 249)
after.

### 3 · The fix is one line, and it costs the rope nothing

`app.add_systems(FixedUpdate, aim::aim.in_set(SimulationSystems::PostStep))`.
`hook::update_hooks` reads `ArmAim` in `Intent`, which is after `World` **and** after the previous
tick's `PostStep`; `Integrate` is the only writer of a player's `Transform` and it does not sit
between those two points, so **the eye the ray starts from is the same `Vec3` either way**. What
changes is that the rope now flies at the aim of the tick whose picture the player was looking at
when he pressed, instead of one no frame ever showed him.

### 4 · Three things it moved that were not the target

* `tests/hud.rs::f026_the_rope_flies_at_the_point_the_marker_stood_on` went red on an 11 mm drift:
  the `ArmAim` the shot used is now the one standing **before** the step, not after it. The test
  reads the other snapshot; the claim is unchanged and stronger.
* `tests/hud.rs::f024_a_snap_moves_the_marker_sideways_on_the_screen_and_never_up_or_down` carried
  a **compensation for this very defect** — a second `stand_and_look` after the step, put there to
  warp the eye back to where `aim` had cast from (1.4 px residual with the velocity zeroed, 8.6 px
  without). With `aim` in `PostStep` the compensation re-opens the gap from the other side and the
  test read 2.26 px. Deleted: worst vertical movement **0.001 px** over 144 pairs.
* `hud::crosshair` and `hud::catch_band` need no change — both read the answer rather than
  re-deriving it, which is the corollary rule, and both are now fed a point that agrees with the
  frame they are drawn into.

### 5 · And a measurement trap that cost a round of confusion

`Vec3::angle_between` is `acos` of a dot product, and near zero that formulation throws away half
the mantissa: two vectors that differ by nothing measurable come out at `sqrt(2·f32::EPSILON)` =
**4.88e-4 rad = 0.30 px**, which reads exactly like a residual defect. `|a − b·(a·b)| / (a·b)` is
the same angle's tangent and it is exact in the small. **Do not measure a small angle with
`angle_between`.**

**Evidence:** `tests/hud.rs::f026_the_marker_stays_on_the_cursor_while_he_is_flying` (240 samples,
`50.00 px → 0.00 px`) · `tests/vector_aiming.rs::f002_the_ray_starts_at_the_eye_the_frame_is_drawn_from`
(120 samples, `54.48 px → 0.00 px`). Both go red on the one-line revert to `World`, measured.

---

## FIND-216 — the game had no water at all, and the box that fixes it has to reach INTO the riverbed

**2026-09-01 · `[offlinebot]` · `assets/data/water.ron`, `src/world/water.rs`,
`src/player/swim.rs`, `src/shared/water.rs`**

`maps.ron:577` carried its own heading for eighteen days — **"The river, in a game that has no
water"**. The Ashgate canal was a dry lowered lane: 10 m between the quays, floor 4 m down,
`anchorable: false`, bridges as the only crossings. No surface, no colour, no rule for a body
that falls in. The user, 2026-08-29, answered the three questions that opens:

| | his words | where it lives now |
|---|---|---|
| on contact | *„Man schwimmt / wird langsam."* | `player::swim`, `water.ron: swim` |
| hookable | *„Nein — Wasser haelt keinen Haken."* | `vector::hookable::SurfaceKind::Water` |
| size | *„das wasser ist auch VIEL zu klein"* | **half open** — see §3 |

### 1 · The measurement: a water box that stops 0.05 m above the riverbed is a trap

The first version put the bed at **-3.95**, 0.05 m above the channel floor (-4.00), by the same
z-fight argument the x axis uses. `tests/player.rs::f003_a_body_dropped_into_the_canal…` went red
with **`the body sits 0.000 m under the surface`** after four seconds in the river.

The cause is arithmetic, and it is the whole finding: **exponential drag needs `v / drag_per_s`
metres to stop a body**, and a 20 m drop enters at `sqrt(2·32·20.6) = 36.3 m/s`, so it needs
**6.05 m** — against a channel that is **3.4 m** deep. So the body punches through to the floor,
lands at `y = -4.00`, and that is **below `min.y` of the water box**: `depth_m` answers *dry*, the
buoyancy never fires, and he lies on the bed for ever with no way out but the gear.

**The rule: a water volume must reach at or below the solid floor under it, never above.**
Fixed by putting the bed at **-4.15**, 0.15 m *inside* the 0.2 m floor slab — invisible, because
those faces are inside stone. `tests/world.rs::f003_the_canal_water_lies_inside_the_channel…`
now asserts the direction (`bed <= floor top` **and** `bed >= floor underside`), so the next map
cannot repeat it.

### 2 · The evidence, one run, `scripts/f-water.txt`, 11 asserts, exit 0

| t | moment | y | speed | gas |
|---|---|---|---|---|
| 19 | over the channel, dry | 18.392 | — | 15000.000 |
| 66 | the last tick before the water | 0.628 | **35.200** | — |
| 218 | in the river | -1.292 | **0.195** | — |
| 237 | rope on, **fired from inside the water** | -1.291 | rope 1 | — |
| 385 | out, on gas | **9.814** | — | **14973.018** |

**35.200 → 0.195 m/s is 180x in 2.5 s**, and he floats at `-1.292` — 0.692 m under the surface
against the 0.727 m the two files predict (`-gravity_m_s2 · surface_band_m / buoyancy_m_s2`).
Picture: `docs/images/f003-water.png`.

### 3 · What is NOT done: *„VIEL zu klein"* is half answered, and the diff is one file away

The water can be no wider than the hole in the ground it lies in, and that hole is
`maps.ron: blocks` — **another stream's file this round**. What is needed, exactly:

```
-  (center_m: (-70.0, -4.1, 0.0), size_m: (10.0, 0.2, 700.0), color: "stone_gray", …)   channel floor
-  (center_m: (-80.0, -1.8, 0.0), size_m: (10.0, 4.4, 700.0), …)                        quay west
-  (center_m: (-60.0, -1.8, 0.0), size_m: (10.0, 4.4, 700.0), …)                        quay east
+  (center_m: (-70.0, -4.1, 0.0), size_m: (40.0, 0.2, 700.0), color: "stone_gray", …)   channel floor
+  (center_m: (-95.0, -1.8, 0.0), size_m: (10.0, 4.4, 700.0), …)                        quay west
+  (center_m: (-45.0, -1.8, 0.0), size_m: (10.0, 4.4, 700.0), …)                        quay east
```

and then **two numbers in `water.ron`**: `size_m: (39.9, 3.55, 700.0)`. ⚠️ `maps.ron` argues at
:996 that the quays are 10 m apart *because* a 12 x 11 m row house must not fit into the gap —
at 40 m that argument is gone and the aprons have to carry it instead, or the layout will roll
houses over the water. The towers he asked for beside it (*„adde andere tuerme beim wasser"*) are
`maps.ron` rows as well and are not in this round.

### 4 · Two absences that are decisions, not omissions

* **Water carries no `Collider`.** A collider you can swim through is a `Sensor`, and a sensor
  answers `SpatialQuery::cast_ray` like anything else — avian clamps `tmin` to 0 for an origin
  **inside** a shape (`bevy_math-0.19.0/src/bounding/raycast3d.rs:64`), so the one shot the player
  needs to get out would be the one shot that answers at distance 0.
  `tests/player.rs::f003_a_hook_fired_from_inside_the_water_still_finds_the_quay_above_it` is
  the guard.
* **Water carries no `Body`, so it is not in the `SpatialIndex`** — and it would buy nothing if it
  were: **`SpatialIndex::cast_ray` and `::aabb_overlaps` are both still stubs**
  (`src/shared/spatial.rs`, "filled in by job R — T-036a") that answer `default()` and clear the
  output buffer. `player::swim` therefore reads a `Query<&WaterVolume>` (**one entity** on the
  shipped map), and the day the index answers, that is the one line that changes.

## FIND-218 — the 25 GB was not the map: it was bevy formatting 2 290 028 cycles

**2026-09-01 `[offlinebot]` · `B-030` · refutes the `n²`-over-blocks hypothesis outright**

The standing hypothesis was quadratic-in-block-count: the map grew 2901 → **8073 blocks**
yesterday, and `8073² · 76 B = 4.61 GB` against the 4.63 GB that failed — a 0.5 % match. It is a
coincidence, and here is the control that kills it.

| map | blocks | peak RSS, test green | peak RSS, cycle present | cycles enumerated |
|---|---|---|---|---|
| graybox | **101** | 166 / 168 MB | **529 624 kB** | **2 290 028** |
| ashgate | **8073** | 244 / 251 MB | **529 128 kB** | **2 290 028** |

A/B/A/B, one pinned binary, `ulimit -v 6291456`. **80× the blocks moves peak RSS by 1.5×**, and
the explosion is *bit-for-bit the same size on a map with 101 blocks* — it fires before the map
is built at all. The failing allocation is **4 966 055 936 = 37 · 2²⁷**, a `Vec` doubling, not
`n² · 76`; 4966055936 / 8073² = 76.2, not an integer. The arithmetic matched because two large
numbers were multiplied until they did.

**What it actually is:** a dependency cycle in `FixedUpdate` (`B-030`), and bevy's report about
it. `dependency_cycle_to_string` (`bevy_ecs-0.19.0/src/schedule/error.rs:174-206`) formats
**every simple cycle in the strongly connected component** into one `String`. The count of simple
cycles is combinatorial in the SCC, not linear in anything visible: 10 nodes here, 2.29 million
cycles, and the `String` doubles 155 MB → 310 → 620 → 1.24 G → 2.48 G → **4.96 G**.

**Complexity, before and after.** Before: `O(number of simple cycles in the SCC)` × the length of
each — unbounded in practice, and *no* map size makes it safe. After: the cycle is gone, so the
enumeration never runs (`O(1)`); and where a future cycle appears, the printed message is bounded
by `MAX_CYCLES_SHOWN · MAX_NODES_PER_CYCLE` = 3 × 12 node names, a constant. The residual
529 MB is bevy's own `Vec<Vec<SystemKey>>` of cycles, which we cannot bound without patching
bevy — but 0.53 GB does not kill a machine, and `tools/test.sh`'s cap covers it.

**Three rules this earned:**

1. 🔴 **`ulimit -v` and `mold` do not compose.** `( ulimit -v 6291456; cargo test ... )` — the
   invocation prescribed as the *safe* one — fails at the **linker** with
   `mold: cannot reserve 8589934592 bytes of virtual memory` whenever anything needs rebuilding.
   The guard reports a link error where you are looking for a memory error. **Compile uncapped,
   run capped** (`tools/test.sh`).
2. 🔴 **A test-only schedule edge is production code for the machine.** Neither
   `.after(aim)` line is shipped, the game runs fine, and `grep`ping `src/world/` for a nested
   loop found nothing because there was nothing to find. The suspect list was drawn from the
   *changed data* and the bug was in the *unchanged test*.
3. **Instrument, and instrument the allocator, not the loop.** A 30-line `#[global_allocator]`
   in the test binary that aborts with `Backtrace::force_capture()` above 200 MB named the site
   — `bevy_ecs::schedule::error::dependency_cycle_to_string` — on the first run. Bisecting
   Startup systems or reading `src/world/` would never have reached it: the allocation is not in
   our code at all.

---

## FIND-219 — the refute pass on the wall, the water and the marker: two hold, and the evidence for the third was a duplicate of the wrong picture

*(2026-09-01 · adversary round on the uncommitted three-stream build · `tools/corpus.sh` before
and after)*

**Which binary measured what, because two of these rounds need different answers.** Every
`maps.ron` and every `water.ron` control ran against **one pinned binary** (`dbt-round2`, copied
before the first edit) — the data moved and the code did not. The marker's A/B is the opposite by
construction: its control **is** a one-line source change, so the two runs are two binaries, and
what pins them is that nothing else in the tree moved between the builds and both traces come
from the same script and the same map.

### 1 · The marker — **HOLDS**, and it holds on a fixture FIND-217 never used

`f026-turn` reproduces FIND-217 exactly. The new fixture is a **rope-driven flight into the
gate hood** (`f003-wall.txt` ACT 2 at yaw 0 instead of 60): a target **0.354 m** in front of the
eye at 33 m/s, which is the `v·dt/d` geometry the finding said would diverge.

| fixture, Left arm, target under 5 m | `PostStep` (as built) | one-line revert to `World` |
|---|---|---|
| `f026-turn` t 0..620 | med 0.000 · **max 0.01 px** | med 27.06 · **max 419.98 px** · 392.92 jump |
| flight into the gate hood | med 0.010 · **max 0.010 px** | med 0.010 · **max 169.01 px** · 152.93 jump |

Both arms measured, not one (`ALL` and `d<5m` split, `n = 1800` per arm). Split by state, every
`Ready` / `Free` / `Busy` sample on both arms is **0.00 px**; the only non-zero rows are
`Anchored`, where the marker is supposed to sit on the anchor and not on the crosshair.

The two shipped pictures reproduce: **all** differing pixels lie in rows **350..369**, the cyan
`(63, 237, 249)` count in `x 600..680` goes **0 → 147** and in `x 192..229` goes **142 → 0**,
exactly as FIND-217 says. Only its total is off — **641** by a plain per-pixel comparison against
its **626**, and no threshold reproduces 626 (624 at `>16`, 630 at `>8`). A stated count should be
reproducible by the obvious instrument or say which one it used.

🔴 **But `clamp=0` still does not mean "a sane pixel".** `Camera::world_to_viewport` returns
`Ok` with coordinates thousands of pixels off-screen for a point just inside the near plane, and
`place_arm_aim` clamps the glyph to the rim anyway. On `f-001-hooks` the flag counts **532**
clamped Left-arm samples, while **352** more carry `clamp=0` with `dgp` up to **4672.87 px** —
i.e. the layout clamped them and the instrument says it did not. FIND-217 closed the `none` hole
and left this one; it does not move any number above, because those samples are all `Anchored`.

### 2 · The water — **HOLDS**, and the drowning trap it was built against does not reproduce

Four drop heights, one channel, `z = 60` (FIND-216 predicted a body needs `v / drag_per_s`
metres to stop against 3.55 m of water):

| drop | entry speed | after 2.5 s | height after 2.5 s | still there 6 s later |
|---|---|---|---|---|
| 4.6 m | — | 0.259 | **−1.295** | — |
| 20 m | 34.667 | 0.188 | **−1.293** | — |
| 60 m | 60.265 | 0.209 | **−1.291** | −1.294 |
| 200 m | **75.000** (terminal) | 0.154 | **−1.312** | — |

He floats at −1.29 every time; **he never lies on the bed**, which is the failure the bed at
−4.15 was moved to prevent. Warped into the corner of bed and quay at (−65.4, −3.9, 60) he rises
out of it. `player::swim` works: same key, two media — **6.000 m/s dry against 2.679 m/s wet**,
45 % of walking, which is *„Man schwimmt / wird langsam"* in one line.
⚠️ **My own first swim measurement said 0.256 m/s and was wrong**: the mark stood on the tick
`key w 4.0` expired, so it measured the release and not the hold. *Sample the middle of a hold,
never its last tick.*

**Each of the three water claims has its own knob and each knob moves its own number**, with the
dry control unmoved at 6.000 m/s in all three runs:

| knob | shipped | control | floating, no key | swimming |
|---|---|---|---|---|
| `drag_per_s` | 6.0 | 0.5 | 0.252 → **3.572** | 2.679 → 4.323 |
| `swim_speed_m_s` | 2.5 | 0.0 | 0.252 (unmoved) | 2.679 → **0.433** |
| `gas_cost_factor` | 2.0 | 1.0 | — | gas 14973.018 → **14977.814** |

🔴 **The gas surcharge is the one that does not survive being looked at.** Doubling the price of
working under water costs **4.796 units of a 15 000 tank — 0.032 %** — because only the part of
the climb that is still submerged is billed, and that is a fraction of a second. `f-water.txt`'s
`assert gas < 14990` holds at *both* settings, so the act does not measure the factor it says it
measures. Either the number is much larger than 2.0 or the assert should stop claiming it
(`docs/QUESTIONS.md` Q-084 owns the value).

### 3 · The wall — the mechanic holds, **the street picture did not exist**

`docs/images/f003-wall-street.png` was a **second copy of the aerial**, one tick off: against my
own render of its own documented tick it differed by mean 39.96 (862 569 px), against the aerial
by mean **1.60**. The aerial is genuine — my render of tick 114 is **bit-identical** to it. The
missing picture is the one the user's sentence is about (*„die grossen tuerme /gates beim
eingang… passt GAR nicht"*), and it is now rendered from ACT 0's mark tick, t = 103.

The swing is real and attributable. Deleting **one line** of `maps.ron` moves every number of
ACT 2, which is the control the round did not have:

| ACT 2 mark | shipped | gate cornice deleted | gate hood deleted | 30 m rung deleted |
|---|---|---|---|---|
| arc bottom | **15.318 m @ 42.282** | 4.371 @ 50.006 | unchanged | unchanged |
| far apex | **26.110 m @ 29.149** | 0.398 @ 42.417 (the ground) | unchanged | unchanged |

And against the **old map at the same stand**, which is the comparison that says whether the
wall gave the property back: yaw 60 was `30.481 m @ 8.558`, dying to 2.441 m/s; it is now
`15.318 m @ 42.282` and it comes back up. **The wall is a better swing than the gantry lane
was.** ⚠️ Only in the plane *along* it — see `B-031`, which is not a regression: the old map
read 38.407 → 0.346 m/s at the same stand and yaw. Sampled one tick apart, the strike is
**37.456 → 0.026 m/s in a single step**, then 29 ticks pinned at `y = 20.533` until the rope
lets go.

**And the headline "2901 → 8073 blocks" is not this round's number.** Measured: `8278 blocks
(243 placed, 8035 generated)` at `HEAD` against **`8073` (229 placed, 7844 generated)** in the
working tree. The map got *smaller*; 2901 is from some earlier map entirely.

### 4 · `f003_every_cornice_on_the_wall_hangs_over_open_ground` was measuring the wrong set — twice

Two defects, both found by **counting what the inner loop skips** rather than by reading it:

* **`o.hi.y > b.lo.y → continue`** treated an obstruction as "below" a ledge only when it lay
  *entirely* below it. A mass that **passes** the ledge is under it over its whole lower part —
  which is exactly the shape of a tower. Control: widen the inner-gate pier to reach 27.00 m off
  the centre line (`center_m: (-17, 60, -115), size_m: (6, 120, 44)` — deliberately 44 m deep, so
  that the `WALL_THICK_M` exclusion cannot answer for it instead). Old predicate: **2 of 51**
  cornices flagged. Fixed (`o.lo.y > b.lo.y - 1e-3`): **11 of 51**, on both faces. The eight it
  used to miss were every rung under 120 m.
  ⚠️ **The first control I tried did not isolate this** — putting a whole 20 x 120 x 55 m
  gatehouse tower back left the test green for a *different* reason (55 m deep > `WALL_THICK_M`
  45). A control that moves the number for the wrong reason proves nothing; it has to be the one
  variable.
* **the side test read a centre for the mass as well as for the ledge**, so every block centred
  *on* the wall line — the gate piers, every course of the wall itself — landed on the `>=` side
  and the two inward-face gate hoods were compared against **nothing at all**, reporting
  `far − half_plinth` as though it were a measurement. Now the ledge is placed by its centre and
  the mass by its **reach**. The `considered > 0` assert added beside it is what surfaced both.

**The rule: a `continue` in an inner loop needs a counter, and the counter needs its own assert.**
`0 of N` was never printed here — the test printed a *margin*, computed over an empty set.

### 5 · Four things the gate turned up in the tree — two of them this round's, two not

* `tests/data.rs::t005_every_script_that_asserts_gas_is_on_the_tank_checklist` was **red**:
  `scripts/f003-wall.txt` asserts `gas == 15000` twice and was on no group of the list. Fixed
  (`TANK_SCRIPTS_EXACT`).
* `scripts/f-water.txt` documented its own picture as `--image docs/images/f003-water.png`
  **twice**, and `--image` is not a flag — `unknown launch arguments: --image`. `corpus.sh` only
  ever reads the `--headless` line, so it never ran. Fixed to `--screenshot`, and the tick
  corrected from 260/400 to **399**, which is where the `VIEW` mark actually lands.
* `src/world/map.rs` warns `dead_code` on `Rect::grown`, `Rect::real`, `without` and `cut` — left
  behind by the committed terrain change, not by this round. Not touched.
* 🔴 **`tests/hud.rs::f177_the_board_panel_lists_exactly_what_missions_ron_offers` is
  load-flaky**, and it is not this round's: it failed at `the prompt does not name the board`
  inside a seven-binary gate at `-j 3` under `ulimit -v 6291456`, then passed **alone in 1.23 s**
  and **49 of 49 in its own binary in 7.59 s**, same tree, same binary, no rebuild between. Its
  warm-up is `for _ in 0..4 { app.update() }` with the default `TimeUpdateStrategy`, so how many
  fixed steps it gets is decided by how busy the machine is — which is the exact hazard
  `tests/hud.rs`'s own module doc states the rule against
  (`TimeUpdateStrategy::FixedTimesteps(1)`), applied to one test in the file and not to this one.
  **A gate failure you cannot reproduce twice trains you to skim the next real one.** Foreign
  territory (the hub board), so it is written here and not fixed.
* **`--test vector_rope` is 498.63 s of the gate's ~11 minutes**, at 460–490 % CPU and a flat
  2.93 GB RSS — one binary is three quarters of the wall clock. It also reports **6 ignored**,
  all deliberate `#[ignore = "measurement, not a criterion"]` probes; `FIND-218` reported this
  binary as "29" without them, and an ignored test is a skip like any other.


---

## FIND-222 — the progression was computed correctly and **read by nothing**

*(2026-09-01 · `F-120`/`F-121`/`F-122`/`F-125` · the "Ganz ausbauen" round)*

**The measurement.** `grep -rn "Career" src/ | grep -v "^src/progress/"` returned **0 lines**.
`Profile::cleared` had exactly two mentions outside its own declaration, both in `save/file.rs`
serde plumbing. `progress::career::may_fly` — the rank gate — was called from `tests/progress.rs`
and **from nowhere in `src/`**. The shipped save says why that mattered:

| | shipped `saves/player-1.ron` |
|---|---|
| sorties flown / won | **419 / 352** |
| xp | 80 618 → level 59, rank A |
| gear points earned / **spent** | 122 / **0** |
| `cleared` entries written / **read** | 5 (419 writes) / **0** |

Nothing was broken. Every number was right. `Career::last_sortie_xp` was correct in the very run
that proved the screen was silent — which is why the red test asserts the two halves separately
(`tests/menu.rs::f120_the_debrief_says_what_the_sortie_earned`), and the captured red is:

```
career after the sortie: level 1 (210/300 xp), rank E, 6 gear points (0 spent)
debrief plate: ... | 2 / 2 cortex kills | Ashgate Skirmish · Recruit | WON | DEBRIEF |
the debrief does not say what the sortie earned. The number exists (210 xp) and the
screen that exists to report it is silent
```

**The rule.** *A derived value with no reader is not a feature, it is a liability that tests
green.* Nine unit tests and four integration tests covered this curve, and not one of them could
fail for the reason that mattered. **The cheap check is one `grep` for the type name outside its
own folder**, and it takes a second.

**What closed it.** `progress::ledger` (the words, one place), `menu::debrief::fill_the_ledger`
(the plate), `hud::career` (the same words where no plate can exist), `progress::loadout` +
`save::GearRequest` (spending), and `hud::board`'s ladder markers (`cleared`'s first reader).
Evidence: `scripts/f120-progress.txt` (12 asserts, exit 0), `docs/images/f120-debrief-ledger.png`,
`docs/images/f125-armoury.png`, `docs/images/f121-ladder-cleared.png`.

### §2 — a plate cannot be evidence on this machine, and that is structural

`src/lib.rs` builds `primary_window: None` for **both** `--headless` and `--offscreen`, so
`menu`'s whole `Update` chain is gated off. The commission asked for *"an `--offscreen` frame of
the debrief"* and **that frame cannot exist**: the debrief is a `Screen`. Same wall `FIND-189`
records. So the report got the mission board's own treatment — one word list
(`progress::ledger`), two renderers — and the screenshot is `hud::career`'s panel.
Decoded against a mid-sortie control from the same run: **1 382 amber px** in the panel rect
(`x 819..1228, y 187..720`) against **0** in the control, resolving into **6 text bands** whose
widths track the six strings at 8.8–9.5 px/char; keep-out box holds 0; bit-identical over two
runs (`sha256 52779beb…`).

**The rule:** *if a surface only exists behind a window, it is 🟨 on this machine for ever —
give it a second renderer or do not claim it.*

### §3 🔴 A script whose `wait` is shorter than its own `key` hold silently merges two presses

Measured while writing `scripts/f120-progress.txt` ACT 7. `key tab 0.60` followed by `wait 0.40`
advances the script **while Tab is still down**, and Bevy's `ButtonInput::press` sets
`just_pressed` only for a key that was not already pressed:

```rust
pub fn press(&mut self, input: T) { if self.pressed.insert(input) { self.just_pressed.insert(input); } }
```

So the next `key tab` produces **no press edge at all**. The two presses become one long hold:
the tap that was supposed to step the cursor never happens, and the run spent **twice on `speed`**
while every line in the log looked healthy — `save: player 1 — speed is now at 2 — 2/6 allocated`
is a perfectly plausible sentence for a run that has just done the wrong thing. It cost a probe
build to find, and the code was never at fault.

`scripts/f177-board.txt` survives only because every one of its waits already exceeds its hold —
**by luck of authorship, not by a rule.** There is no guard for this anywhere.

**The rule:** *in any script, `wait` after a `key`/`hook`/`slash` must be longer than that
command's own duration.* Cheap check before trusting a key-driven run:
`awk '/^(key|hook|slash) /{h=$NF} /^wait /{if (h!="" && $2+0 <= h+0) print NR": wait "$2" after a "h" hold"; h=""}' scripts/x.txt`

**Related:** `FIND-189` · `FIND-063` · `FIND-155` · `Q-051` · `Q-062` · `Q-090`

---

## FIND-220 — 34 of 456 houses stood in mid-air over the river, and the veto that was supposed to stop them had never run

**2026-09-01 · `[offlinebot]` · `assets/data/maps.ron`, `assets/data/water.ron`,
`src/world/map.rs`, `tests/world.rs`, `scripts/f-towers.txt`**

The user asked for two things in one sentence — *„adde andere tuerme beim wasser (das wasser ist
auch VIEL zu klein)"* — and the second decides the first, so both landed together. Three
measurements came out of it.

### 1 · The widening is capped at 25 m by two buildings, and the second one only showed up in a script

`FIND-216 §3` specified 40 m centred on x = -70. Neither number survives contact:

* **west** — the pocket's flank wall stands at x = -120 with a 45.0 m plinth, so its face is at
  **x = -97.5**. A 40 m channel puts the west quay's outer edge at -100: 2.5 m *inside* that
  wall, for the 225 m the flank runs.
* **east** — and this one was found by a **regression, not by inspection**. At 30 m the corridor
  reached x = -45, and `scripts/f019-hq.txt` went from GREEN to RED with one line:
  `assert height < 0.3 — measured 0.400`. 0.400 is a quay top. The garrison headquarters' hall
  runs to **x = -47.0** (`block` at `(-46.25, 5.0, 0.0) 1.5 x 10 x 26` — the back wall the whole
  script is about), and the new quay had swallowed it, so the walk that is supposed to stop at
  the back wall climbed the embankment instead.

**With 2.5 m and 3.0 m of margin the corridor is x -95..-50 — 45 m, less two 10 m quays, so
25 m of water**, centred on x = -72.5 and not -70. Bridges 36 → 47 m. ⚠️ **A map change that
touches a 700 m run has to be A/B'd against the corpus, not reasoned about**: the flank wall was
found by reading, the HQ was not, and only the script found it.

### 2 · ⭐ The veto against houses over the channel had never fired, and nobody could tell

`plan_blocks` takes its veto over a house box that runs **from y = 0 upward**
(`ridge_center.y = veto_m * 0.5`). The channel floor's top is at **y = -4.00**. So the floor slab
has never vetoed anything: what kept houses off the water was the two **quays**, 0.4 m proud of
the paving, and `maps.ron`'s argument that *"a 12 x 11 m row house does not fit into a 10 m gap"*.
Widen the gap and the argument dies with it — **34 of 456 generated houses** appeared hanging
over the river, and no test in the repository said so.

**The fix is the rule `CellRole::Hole` already states for the ground: a placed block whose TOP is
below y = 0 vetoes by its FOOTPRINT, at any height** — a house may not stand where there is no
ground. Six lines in `plan_blocks`. Red-then-green control: `< 0.0` → `< -99.0` puts all 34 back.
Guard: `tests/world.rs::f003_no_grid_house_is_generated_over_the_open_channel`.

### 3 · The towers, and what one 17 × 2 × 10 block is worth

Twelve towers, six pairs, one per gap between the crossings the river already had that a single
crossing swing (2d = 22 m) cannot span, skipping the gaps a wall bounds — the two walls' cornice
ladders already cross the channel at 29 / 59 / 89 m. 35.0 m
(`scale.ron: church`), shaft 10 × 10 = the quay it stands on, three rungs corbelled **7.0 m inward
over the water** with tops at 12 / 23 / 35 m — one rung per 11.5 m, which is
`scale.ron: house_large`, the roofscape's own ridge.

`scripts/f-towers.txt`, 36 asserts, exit 0, `docs/images/f003-towers.png`:

| the same stand, the same tick | lowest point | ends |
|---|---|---|
| with the hook | **14.572 m** at 39.459 m/s | **12.000 m, speed 0.000** — the far tower's rung 1 |
| **no hook** | — | **-1.294 m, drifting at 0.255** — floating in the river |
| hook, and the far rung 1 **deleted** | 14.572 m | **-1.306 m, drifting at 0.509** — floating |

A body at rest reads its surface **exactly** (12.000 on a 12.00 deck, 23.000 on a 23.00 deck,
both measured); a floating one sits at -1.29 and never stops moving. So the third row is the
finding: **one deck is the difference between the far bank and a swim.** Gas 15000 → 15000 both
ways — the rope is free and the **climb** is the price: the two facing decks are 11.0 m apart, so
`bottom = H - d` gives 24.0 m at rung 3 and **1.0 m at rung 1** — 1.6 m over the water, inside the
1.8 m a body is long. You climb, or you swim.

### 4 · And one wrong word that cost two runs

`scripts/f003-wall.txt` said *"yaw +60 = 60 degrees off -Z toward +X"*. `shared::Intent::look_dir`
is `(-sin yaw, sin pitch, -cos yaw)`, so **+yaw is -X**. Aiming `f-towers.txt` from that comment
hooked the wrong bank of the river twice, and the swing looked plausible both times. Comment
fixed; every number measured under it stands.

**Related:** `FIND-216` · `FIND-041` · `FIND-042` · `Q-036` · `Q-088`

---

## FIND-221 — `B-021` is not an energy loss in the rope. **The player flies into the wall.**

**2026-09-01 · `[offlinebot]` · refutes `B-021`, merges it into `B-031`**

`B-021` says *"a taut rope dumps 23.7 m/s in a quarter second with 6.6 m of air underneath"* and
names two suspects: the `DistanceJoint` under `Drive` (`Q-058`) and the two-rope feasibility rule
(`B-013`). **Neither is in the causal path, and it is not a quarter second — it is one tick.**
One instrumented run of `scripts/f005-feel.txt` ACT 4 at the y = 20 stand:

| tick | p = (x, y, z) | \|v\| | v·r̂ | \|v_t\| | contact |
|---|---|---|---|---|---|
| 474 | (68.638, 6.520, **−97.202**) | 33.074 | 0.0219 | 33.074 | — |
| 475 | (68.503, 6.512, **−97.680**) | **7.898** | −0.0721 | **7.898** | **n = 1** |

1. **The loss is TANGENTIAL** — 33.074 → 7.898 across the rope while the radial part is 0.0219.
   A `DistanceJoint` applies its impulse **along** the rope. It cannot do this at all.
2. avian reports a contact on the player on **exactly that tick** and on every tick after it.
3. `z` is **pinned at −97.680** for the next 70 samples. `maps.ron`: *"z = +350 .. −97.5 the
   district"* — the main wall.
4. `rope_drive`'s look gate is **0.0000** for the whole act, so the drive contributes nothing.
5. **One** arm is anchored, so `hold_the_pair` never runs.

**The rule this earns.** `B-021`'s controls raised the stand to y = 16 / 20 / 24 and concluded
*"removing every possible collision does not give the swing back"*. They varied **height** against
a **vertical** face and could not move it — `docs/lessons/fixtures.md` §4 exactly, one bug later.
**Name the axis the obstacle lies on before you sweep the one you can reach.** The 6.6 m of air is
real; it is under the wrong direction. Two rounds were spent on an `F-004` energy bug that does not
exist, and the "fix" `B-021` asked for — separating the joint from the pair rule — would have
changed a mechanism that was never involved.

**Related:** `B-021` · `B-031` · `Q-058` · `B-013` · `F-004` `F-005`

---

## FIND-221b — `S` on a taut rope cannot brake, and the latch is `ground_top_speed_m_s`

**Measured 2026-09-01, `scripts/f176-pull.txt` line 133: `assert speed < 2 — measured 7.630`.**

`ground_top_speed_m_s` = `run_speed_m_s − gravity/hz` = 6.0 + 0.5333 = **6.533 m/s**, and
`integrator::movement_state` returns `Tethered` for an **anchored** player the moment his ground
speed passes it. `Tethered` takes him out of `ground_locomotion` (which is what `S` acts through)
and puts him **into** `locomotion::air_control`'s `in_the_air`, where `rope_winch`'s always-on pull
runs — free, keyless, up to `drive_idle_speed_m_s` = 12 m/s of closing speed. So the pull holds him
over the latch, and over the latch the legs are not allowed to brake him. 7.630 sits between 6.533
and 12.0, which is where the loop settles. **🟨 — the mechanism is derived from the two numbers and
the two predicates; `MovementState` itself was not sampled at that tick.**

Two of the user's own instructions meet here and disagree: *„ich will dass es immer ranzieht. nicht
nur wenn ich w drücke!"* (`FIND-172`) against §3F R1 *„seil ist vorne und ich laufe zurueck"*.
→ `docs/BUGS.md` B-034, `docs/QUESTIONS.md` Q-089.
