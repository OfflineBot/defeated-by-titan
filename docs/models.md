# models — the asset chains (models, atlas, sound, VFX), and how YOU swap a model

Updated: 2026-08-12 · Stage: 🟨 for the **model registry and the swap** — `assets/data/art.ron`
decides primitive-or-`.glb`, `src/render/model.rs` executes it, the cuboid it replaces is
hidden, the game state plays the clip and the model's `cortex` empty moves the kill zone;
`tests/render.rs` and `tests/titan.rs` hold every direction (seen red first). ⬜ for **everything above it in the chain**: there is no
`tools/blend/`, no `tools/atlas/`, no `tools/sound/`, no `.blend` and no auto export, and
machine A has no Blender — see below and [`docs/environment.md`](environment.md). `assets/3d/`
exists since 2026-08-12 and is **empty on purpose**: the game runs with no `.glb` at all.

## The chain

```
tools/blend/<name>.py  ──►  assets/3d/blend/<name>.blend  ──►  assets/3d/glb/<name>.glb  ──►  assets/data/art.ron
  the agent writes it       YOU open it and fill it in          exported automatically          the switch
```

**Why a script and not a `.blend` directly:** a `.blend` is a binary lump — in git nobody sees
what changed, and you cannot write one without starting Blender. A script is a diff, is
reproducible, and is the place where *our* placeholder lives.

```bash
blender --background --factory-startup --python tools/blend/vanguard.py
```

---

## For the user: this is how I swap a model

**This section describes what is built, not what is planned** (2026-08-12). The registry, the
fallback and the animation seam run; the Blender half of the chain above them does not exist
yet — there is no `tools/blend/`, no auto export, and not a single `.blend`. So the route that
works today starts with a finished `.glb`, from wherever you got it.

### The three steps

1. **Put the file at `assets/3d/glb/<name>.glb`.** That folder is committed to git on purpose
   (§"The auto export" below) — a model that only exists on your machine is a game that only
   runs on your machine.
2. **Change one line in `assets/data/art.ron`:**

   ```ron
   "titan_husk": (source: Primitive, scale: 1.0, attribution: None, animations: {}),
   //            ^^^^^^^^^^^^^^^^^^ becomes
   "titan_husk": (source: Gltf("3d/glb/titan_husk.glb"), scale: 1.0, attribution: None,
                  animations: {}),
   ```

   The path is relative to `assets/`. Nothing else changes, and **no Rust is touched** — the
   code only ever asks for the logical name `titan_husk`.
3. **Start the game.** The cuboid rig is replaced by your model at the same place, in the same
   size class, on the same entity.

**To go back**, put `source: Primitive` back. The primitive path is not a legacy branch that
rots — it is what every row says today, and `tests/render.rs` holds both directions.

### Your own animations

`animations:` maps a **game state** to the name of the clip **inside your file**:

```ron
"titan_husk": (source: Gltf("3d/glb/titan_husk.glb"), scale: 1.0, attribution: None,
               animations: {"idle": "Idle", "walk": "Walk", "windup": "Windup",
                            "strike": "Strike"}),
```

The names on the left are the ones **the game asks for**; the values on the right are yours,
exactly as the clip is called in Blender. `{}` means "this model animates nothing", and that is
a legal answer.

**They are not invented for the art pipeline — they are the states the simulation is already
in** (`src/render/model.rs::clip_state_of_titan` / `clip_state_of_movement`). Nothing here is a
second state machine that can disagree with the first one:

| Model of a… | state name | comes from | plays |
|---|---|---|---|
| titan | `idle` | `TitanState::Idle` | looping |
| titan | `walk` | `TitanState::Pursue` | looping |
| titan | `windup` | `TitanState::Windup` | **once** |
| titan | `strike` | `TitanState::Strike` | **once** |
| titan | `recover` | `TitanState::Recover` | **once** |
| titan | `death` | `TitanState::Death` | **once** |
| player | `idle` | `MovementState::Grounded` | looping |
| player | `fall` | `MovementState::Airborne` | looping |
| player | `swing` | `MovementState::Tethered` | looping |
| player | `wall` | `MovementState::OnWall` | looping |
| player | `downed` | `MovementState::Downed` | **once** |

A state you leave out of the map is not an error — that state simply has no clip, and you are
told (next section). **There is no blending and no transition table:** the state changes, the
clip changes, full stop. A cross-fade is a decision nobody has made yet.

### What you will see if you get it wrong

Every one of these is a **line in the log**, never a crash and never a silent wrong result —
which is the whole point, because the three glTF traps at the bottom of this file all look
identical from the outside.

| What you did | What happens | What the log says |
|---|---|---|
| Named a file that is not there | the entity keeps its cuboid | `ERROR … points at "3d/glb/x.glb" and that file did not load` |
| Named a model that is not in `art.ron` (e.g. a typo in `titan.ron`) | the entity keeps its cuboid | `WARN model "titan_hsuk" is not in art.ron` |
| Named a clip that is not in the file | that **one** state has no animation, the rest work — and **the cuboid rig comes back for that state**, because the cuboid is the thing the game can still animate | `WARN … maps the state "windup" to the clip "Windup", and that clip is NOT in the file. The clips the file does carry are [...]` |
| Left out the `cortex` empty | the cortex stays where the rig computes it | `WARN model "titan_husk" carries no "cortex" empty` |
| Put the `cortex` empty in the wrong place | the model is used as it is, and you are told by how much | `WARN … its "cortex" empty sits at 6.20 m, and scale.ron puts the cortex of this size class at 8.90 m — 2.70 m out` |

That last row is the one that matters most and is the easiest to miss: **`F-030` says a titan
dies only from a cut into the cortex.** If your model's neck is somewhere else than the size
table says, the cut lands where the silhouette looks right and the kill zone is elsewhere. The
tolerance is one number, `cortex_tolerance_m` in `art.ron`, and it is 0.15 m.

### What a swap does and does not do (2026-08-12, second pass)

**Closed on 2026-08-12** (`docs/FINDINGS.md` FIND-054, `tests/render.rs` + `tests/titan.rs`):

- **The cuboid gets out of the way.** When your scene has *arrived*, every primitive mesh on
  that entity is hidden — and **only the picture**: the body collider, the cortex sensor and
  every length the rig computed stay where they were, so a hidden cortex still kills. A file
  that never loads hides nothing, which is why a typo leaves a titan standing rather than
  making him invisible.
- **The clips play.** One `AnimationPlayer` per spawned scene, one `AnimationGraph` per model,
  and the game state picks the node (table above).
- **The `cortex` anchor decides where the titan dies.** If your model brings the empty, the kill
  zone moves there; if it does not, the position `scale.ron` computes stands, exactly as before.

**Still open:**

- **The other seven anchors are read and used by nobody.** `hook.l/r`, `hand.l/r`, `eye`,
  `hit.min/max` land on the entity as `ModelAnchors` and no domain asks for them yet — a hook
  still bites where the collider is, not where the model says.
- **No transitions.** State in, clip out, no cross-fade, no blend tree.
- **No `.blend` route.** Everything above the `.glb` in the chain diagram is still a plan.
- **No `.glb` has ever been through any of this.** Every path above is proven on a **synthetic**
  `WorldAsset` built inside the test, because the repository has no file and must not have one.
  What a real export does with names, orientation and its own `AnimationPlayer` is ⬜.

Seen: `docs/images/f003-city-after.png` — the city out of `maps.ron` after the registry landed,
still every bit of it a primitive, because every row still says `Primitive`.

> ⚠️ **A `.blend` you have touched is sacred.** The generator **never** overwrites it. It
> checks: the file exists and is newer than its script → *"edited by the user, left alone"*
> into the log, done. Only what is missing is created anew; everything else only with an
> explicit `--force <name>`. **Blender has no history** — whoever breaks this rule deletes
> work nobody can restore.

---

## The conventions that make replacing cheap in the first place

They stand **here** and as a comment header in **every** `tools/blend/*.py`:

| Rule | why |
|---|---|
| **1 Blender unit = 1 meter** | Scale is done in the model, not through `scale` in the RON — that field is an emergency brake, not a working tool |
| **Origin between the feet** | otherwise every model stands half in the ground |
| **Facing −Z, upright** | Model Z-up in Blender, the exporter turns it with `export_yup=True`. **Do not rotate it yourself**, or it turns twice |
| **Color through vertex colors, not through a texture** | Lowpoly needs no UV map, and vertex colors survive any remodeling |
| **One object per meaningful part**, named (`head`, `arm.r`, …) | amputation and animation hang on that later |

### The anchors are empties with fixed names

With them the modeler decides **where**, the RON decides **how strong**:

| Empty | what for |
|---|---|
| `cortex` | **the kill zone.** A cortex hit kills, no matter how full the titan is |
| `hit.min` / `hit.max` | the hit zone, as a cuboid |
| `hook.l` / `hook.r` | where the hooks of the Vector Gear bite |
| `hand.l` / `hand.r` | grabbing, throwing |
| `eye` | look direction, blinding |
| `hook.<anything>` | **an OPEN family, since 2026-08-18** — every rope point the architecture kit carries: `hook.traufe` (eaves), `hook.first` (ridge), `hook.krone` (crown), `hook.gesims_15..105` (the wall's cornice ladder) |

The eight names above `hook.<anything>` are a **closed** list and stay one: a Blender typo in
`cortx` has to read as *missing*, not as *new*. The `hook.` prefix cannot be a list, and that
is measured rather than assumed — the pack's 565 hook empties carry **212 distinct names, 130
of them appearing in exactly one file**. `shared::anchors::is_anchor_name` is the one place
both halves live: `ANCHOR_NAMES.contains(name) || name.starts_with("hook.")`.

**How they are read (since 2026-08-12):** a glTF node arrives in Bevy as an entity with a
`Name`. When a model's scene instance is ready, `render::model::read_the_models_anchors` walks
it, picks up every empty the table above names, converts it into the model root's own
space and puts it on the entity as `ModelAnchors` — **in meters, relative to the origin between
the feet.** The list of names is in `src/shared/anchors.rs::ANCHOR_NAMES`, and it is this table.
(The type lives in `shared/` and not in `render/` because `titan` has to **read** it: `render`
writes, everybody else reads — `docs/architecture.md`.)

> **A missing empty does not become a point at `(0,0,0)`** — that would be a kill zone between
> the feet, and it would look like a physics bug rather than a modelling mistake. It becomes
> **absent**: `ModelAnchors::get` answers `None` and the reader keeps using the position the
> rig computes. A missing `cortex` is additionally a `WARN` at load, and a `cortex` that sits
> further than `art.ron: cortex_tolerance_m` from what `scale.ron` says for that size class is
> a second `WARN` **with both numbers in it**.
>
> **The `cortex` anchor is read since 2026-08-12** — `titan::rig::cortex_from_the_model` moves
> the sensor there, and the fallback is the **absence** of that write, so a model without the
> empty keeps the position `scale.ron` computes. Measured: `scale.ron` 8.90 m → model 9.30 m,
> cortex measured at 9.30 m (`tests/titan.rs::f030_a_models_cortex_anchor_beats_the_computed_position`).
> ⚠️ **The other seven anchors are still read by nobody**: a hook bites where the collider is,
> not where `hook.l` says.

---

## The size table — given by the user, 2026-08-09

**This is the truth about sizes.** It beats every derivation: wherever the backlog conversion
was used before (0.28 m per backlog unit, [`docs/QUESTIONS.md`](QUESTIONS.md) Q-002) and the
result contradicts this table, **this table** holds. The conversion stays only for everything
the user has said nothing about.

> The table stands **machine-readable** in `assets/data/scale.ron` — this here is the version
> for the modeler. Since 2026-08-09 that is no longer a promise but a guard:
> `tests/data.rs::t005_the_size_table_in_the_docs_shows_the_same_numbers` reads **this file**
> and falls over as soon as a number here deviates from the RON, or a new structure in the RON
> is missing here. That is why the numbers stand **exactly as in the RON** — without a
> trailing zero (`1.8 m`, not `1.80 m`); otherwise the guard checks nothing.

| Object | Height | Note |
|---|---|---|
| **Reference** | | |
| Human | 1.8 m | check the capsule **exactly** |
| Door | 2.1 m | |
| Street width | 6–8 m | keep them narrow |
| **Architecture (×1.0)** | | |
| Small house (1 story) | 4.5 m | eaves 3 m |
| Town house (2 stories) | 8 m | eaves 6 m |
| Large house (3 stories) | 11.5 m | upper limit of the residential stock |
| Tall house (5 stories) | 18 m | **rare** — `maps.ron: layout.tall_fraction`, and the answer to [Q-036](QUESTIONS.md) |
| Tree | 12 m | foreground layering |
| Watchtower on the wall | 12 m | |
| Church / bell tower | 35 m | landmark, not a grid house |
| **Titans (×1.4)** | | cortex at ~89 % |
| Small titan | 4.2 m | Cortex 3.7 m |
| Medium titan | 10 m | Cortex 8.9 m |
| Large titan | 14 m | Cortex 12.5 m |
| Huge titan | 21 m | Cortex 18.7 m |
| Abnormal / Boss | 28 m | Cortex 24.9 m — row title as the user has it, [Q-020](QUESTIONS.md) |
| Titan head size | 1/9 – 1/10 of the height | human = 1/7.5 |
| **Walls (×2.4)** | | |
| Wall height | 120 m | |
| Wall thickness at the top | 28 m | |
| Wall thickness at the base | 45 m | battered |
| Intermediate platform | 60 m | stopover on the climb |
| Stone course | 0.6 m | scale ladder, visible joints |
| Horizontal banding | 15 m | a band every 15 m |
| **Boss** | | |
| The Ashwalker | 150 m | 30 m above the wall |
| **Camera / Vector Gear** | | |
| Camera height | 1.6 m | |
| Ground-combat field of view | 55–65 degrees | biggest lever — vertical or horizontal? [Q-021](QUESTIONS.md) |
| Anchor range | 500 m | 90 m until 2026-08-10, 200 m until 2026-08-12, see below |
| Speed | ×1.5 | vs. standard — reference open, [Q-018](QUESTIONS.md) |

*The user labelled the last four rows with the reference work's own terms; here they stand
with the project terms from [`docs/conventions.md`](conventions.md) §2.*

> ⚠️ **One row is translated, not understood.** The user writes „Abnormaler / Boss — 28 m"
> (*Abnormal / Boss — 28 m*). In the project vocabulary "Abnormal" is a **titan kind** and is
> called **Errant** ([`docs/conventions.md`](conventions.md) §2) — so the row could mean "the
> Errant is 28 m tall" instead of "there is a size class called Boss". The second reading is
> the one that holds here, `assets/data/titan.ron` leaves the Errant at 10 m, and the class
> `boss` has no representative. **That is an assumption, not a translation** — it stands as
> [Q-020](QUESTIONS.md) and is withdrawn by one sentence from the user.

> ⚠️ **The anchor range was 90 m and is 200 m since 2026-08-10.** The user played the build and
> asked for it to be *much* longer. The 90 m were his own figure from the day before — a live
> instruction beats the number it replaces, the same precedence rule that had put the 90 m in
> place of the backlog's derived 112 m ([Q-002](QUESTIONS.md)). 200 m is half the 400 m graybox:
> the largest range at which *where you stand* is still a decision. The reasoning, and what
> would have to be rolled back, is [Q-035](QUESTIONS.md).
>
> **What that changes for the modeler:** nothing about heights, and everything about how often a
> landmark is worth building. The **anchor ceiling stays at 14.5 m** — it comes from roof height
> plus `min_rope_m` and has nothing to do with range. But the church (35 m) is now reachable from
> **more than half the city** instead of from its own block, so a tall silhouette earns its
> polygons over a far larger area. Two numbers moved with the range and both are in
> `assets/data/game.ron`: `hook_speed_m_s` 90 → **160** (a 200 m shot at 90 m/s would hang in the
> air for 2.22 s; at 160 m/s it arrives in 1.25 s, and
> `tests/data.rs::t005_a_hook_shot_at_full_range_arrives_before_the_target_has_moved` holds that
> under 1.5 s) and `world.half_extent_m` 300 → **400**, because the spatial grid has to carry
> half the map plus one full range.

> ⚠️ **And 200 m → 500 m on 2026-08-12 — the user named the number himself.** Verbatim
> ([`docs/NEXT.md`](NEXT.md) §1A): *„und das seil muss deutlich deutlich schneller gespannt
> werden. nicht frame perfekt aber mit ca 500m pro sekunde. **mit der range 500 meter!**"* Third
> time the same precedence rule decides it and third time in the same direction.
>
> **What made the old ceiling movable is that its argument had expired**: "half the 400 m
> graybox" was written against a map that has not shipped since 2026-08-12. Ashgate is 700 m
> across, so 500 m is 71 % of the district's edge — *where you stand* is still a decision, but a
> much looser one, and from here the lever that keeps it one is **the map, not the range**.
>
> **For the modeler this is the same change as last time, one size up:** heights do not move and
> the anchor ceiling stays 14.5 m, but the church (35 m) is now reachable from *anywhere* in
> ashgate, so a tall silhouette is worth its polygons across the whole district and a landmark
> that only reads from its own block is wasted. Three numbers moved with the range, all in
> `assets/data/game.ron`: `hook_speed_m_s` 160 → **500** (the user's own figure, and it puts a
> full-range shot back at 1.0 s), `hook_retract_speed_m_s` 120 → **500** (new guard: a miss is
> back inside 1.0 s) and `world.half_extent_m` 600 → **900**, because the grid still has to
> carry half the map plus one full range — 350 + 500 = 850, so 900 is that floor plus 50 m.

### The three scales — and why nobody "corrects" them

Architecture ×1.0, titans ×1.4, walls ×2.4. **The world is deliberately not scaled uniformly.**
A house is as big as a house is; a titan is exaggerated; a wall is monumental. The human is
small, the threat out of all proportion, the wall a horizon — that is the visual language, not
an arithmetic error.

Whoever levels the three factors later because some number strikes him as unrealistic makes the
game technically cleaner and artistically dead — and notices only once everything looks
arbitrary. That is why `tests/data.rs::t005_the_scale_factors_stay_unequal` goes red the moment
somebody tries.

**The city is flat, and that is intended.** The residential stock runs from 4.5 m to 11.5 m.
The vertical comes from the wall (120 m), the church (35 m), the watchtower (12 m) and the
trees (12 m). A sea of tiled roofs with single structures rising out of it — not a skyline.

**And that is exactly why the four landmarks are not decoration.** From 11.5 m of roof height
and a 3.0 m minimum rope (`assets/data/game.ron: vector.min_rope_m`) follows an **anchor ceiling
of 14.5 m** — above that no rope holds on a residential house. The cortex of `large` sits at
12.5 m, of `huge` at 18.7 m, of `boss` at 24.9 m. So a city without church and tower leaves
three of five size classes attackable only ballistically: jump at it,
hit, or fall. Whoever builds a model for the church, the watchtower or a tree is building
**game mechanics**, not scenery. The arithmetic stands as [Q-022](QUESTIONS.md), and
`tests/data.rs` pins down that the start map really carries an anchorable landmark.

### Head and cortex: the two rules that readability hangs on

- **The head is 1/9 to 1/10 of the body height** — on a human it is 1/7.5, that is, relatively
  **larger**. That is exactly what the eye reads "the thing is huge" off, instead of "the thing
  is near". Too large a head makes every titan look like a doll, no matter how many meters the
  data sheet says.
- **The cortex sits at about 89 % of the body height.** That is not decoration, that is the
  **only lethal weak point** (`F-030`) — on the 21 m titan, therefore, at 18.7 m. The `cortex`
  empty in the model belongs there, not "somewhere up top".
  **The figure in meters governs, not the percentage:** the five cortex heights stand
  individually in `assets/data/scale.ron` (`titan.classes[...].cortex_height_m`), because the
  user named them individually. The 89 % are the *rule* against which it is checked whether one
  of the five drifts away — computed from it, the small titan would be 4 cm off.
- **The cortex is smaller than the head.** Sounds obvious; it was not: until 2026-08-09 the
  small titan carried a hit zone 0.80 m across on a head of 0.42–0.47 m. `tests/data.rs` now
  pins down `2 × cortex_radius_m ≤ head height`.

The two together decide whether the bible's criterion holds: **the cortex has to be
recognizable at a distance.** At 100 m a head of 2.1 m (21 m titan, 1/10) is about 1.2 degrees
wide — visible. The hit zone itself is smaller; whether its radius should grow with the size is
open and stands as [Q-019](QUESTIONS.md).

> **Which distance actually applies?** `docs/features.ron` `F-030` demands, in its own words,
> „Cortex ist aus 100 **Backlog-Einheiten** Entfernung erkennbar" (*the cortex is recognizable
> from 100 **backlog units** away*) — that is 28 m (factor 0.28), not 100 m. The difference is a
> factor of 3.6 in pixels and helps decide Q-019. Measured at 1920 × 1080 with the new field of
> view: the Husk's cortex (1.10 m) is **36.7 px** wide at 28 m, **10.3 px** at 100 m. The change
> from 90 to 60 degrees **almost doubled** that number — for `F-030` the narrower image is the
> better number, not the worse one.

### The wall's scale ladder: 0.6 m and 15 m

A 120 m wall without structure is **a gray surface**. The eye has nothing to read size off, and
up close the same wall looks like a 12 m one. Those two numbers are there against exactly that:

- **A stone course of 0.6 m, with visible joints.** One course is a third of a human — whoever
  stands on the wall sees three courses beside him and knows at once how high he is. The joints
  have to be **visible**: a smooth wall with a stone texture does not do it.
- **Horizontal banding every 15 m.** The coarse ladder: eight bands up to the crown, the fourth
  at the height of the intermediate platform.

**These are not decorative details to be cut first when performance is optimized** — they are
the reason the wall *looks* big. `tests/data.rs::t005_the_walls_scale_ladder_stays_readable`
pins both numbers down.

### Where the game-effective numbers live

**In `assets/data/*.ron`, nowhere else** — so that nobody maintains the table twice:

| What | File |
|---|---|
| The table itself, as data | `assets/data/scale.ron` |
| Player capsule, camera height, field of view, anchor range | `assets/data/game.ron` |
| Size class per titan kind (**no height per kind**) | `assets/data/titan.ron` |
| Street width, height window, placed landmarks | `assets/data/maps.ron` |

The last three only **mirror** what stands in `scale.ron`; `tests/data.rs` falls over the moment
one of them deviates. This file here is the **explanation** for the modeler — whoever wants to
change a number changes it in the RON and writes the reason here. And because "change them
together" is a request and not a tool,
`t005_the_size_table_in_the_docs_shows_the_same_numbers` checks the table above cell by cell
against the RON.

## Three glTF traps that all look the same

*("my model is white / chrome / invisible")*

1. **Bevy reads only `COLOR_0`.** If a Blender mesh has **two** color attributes, the painted
   color lands in `COLOR_1` and the model arrives **white**. Make sure in the `.py` that there
   is only one.
2. **A missing `metallicFactor` means 1.0**, that is, *fully metallic* — a diffuse material
   without the value looks like chrome in the game. The export sets it to `0.0` where it is
   missing.
3. **Do not export cameras and lights along with it** (`export_cameras=False`,
   `export_lights=False`). Otherwise a second sun hangs in every model, and the scene gets
   brighter from model to model.

## The auto export

At game start (in `data/`, before everything else) one step checks for every `.blend`: **is the
`.glb` missing or older?** Then export, otherwise do nothing.

```bash
blender --background --factory-startup <file>.blend \
  --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='assets/3d/glb/<name>.glb', \
     export_format='GLB', export_yup=True, export_apply=True, \
     export_cameras=False, export_lights=False)"
```

- **No Blender installed?** → **warn once**, use the `.glb` that is there, **do not crash**.
  The game has to run on a machine without Blender. **That is exactly the case on machine A**
  ([`docs/environment.md`](environment.md)).
- Flags: `--reexport` (rebuild everything), `--no-export` (save startup time). Plus a
  standalone tool `src/bin/export_models.rs`, so that it runs without a game start.
- **`assets/3d/glb/` is committed** and is **not** in `.gitignore` — otherwise the game runs on
  no machine without Blender.

## The switch: `assets/data/art.ron`

**Built since 2026-08-12** (`src/render/model.rs`, `tests/render.rs`). The switch is an
**enum**, not a `bool` plus a path:

```ron
(
    models: {
        // the row that ships bound, verbatim
        "titan_husk":  (source: Gltf("3d/glb/a-042-koerpertyp-a-hager-mittel.glb"),
                        scale: 1.0, attribution: None, animations: {}),
        "titan_large": (source: Primitive, scale: 1.0, attribution: None, animations: {}),
    },
    cortex_tolerance_m: 0.15,
)
```

`Primitive` ⇒ the game leaves the **procedural placeholder** standing (the cuboid rig, the
blocks out of `maps.ron`). `Gltf(path)` ⇒ it loads that file, relative to `assets/`, and hangs
the scene on the entity as a child at the same transform.

**Why an enum and not `use_blend: bool` + `blend: String`**, which is what this section said
until 2026-08-12: rule 2 forbids `serde(default)`, so "no model configured" has to be
*expressible* rather than *absent*. With a bool and a path, a typo in the path is a silently
loaded nothing; with `Primitive` as a named variant, an unknown word is a **crash at load with
a line number**, and the shipped state of the repo — no `.glb` anywhere — is something the file
can actually say.

**Both paths have to work at all times**, and both use the same anchors, the same hit zone and
the same scaling — otherwise switching is not a switch but a rebuild. Four tests hold that,
and they were seen red first with the systems unregistered:
`f030_a_model_without_a_file_stays_the_primitive_it_is_today`,
`f030_a_configured_model_spawns_a_scene_instead_of_the_primitive`,
`f030_an_unknown_model_name_never_takes_the_geometry_away`,
`f030_every_configured_model_names_a_file_that_is_on_disk`.

**No file name in Rust code.** An `asset_server.load("titan.glb")` in the middle of a system is
a bug; there is **one** place that reads the registry (`data/`), everybody else asks for the
logical name. `tools/norms.py` checks it.

## The drop of 2026-08-18 — 278 models, and what it taught the loader

`assets/3d/glb/` holds **278 .glb (26 MB)** and `assets/texturen/` **17 atlases (16 MB)**, both
tracked. The registry's own header carries the inventory; this section carries what had to
change in the code and what is still open.

⚠️ **`assets/texturen/` must not be renamed.** Every model references its atlas from the inside
by the relative URI `../../texturen/TEX-*.png`. The German folder name is the pack's internal
contract, like the German node names inside the files, and is one of the places
`CLAUDE.md` rule 2 does not reach.

### What the pack is authored in, measured

The drop is in the game's exact metres. Confirmed on anchors nobody had used: the door leaf
`tuer_blatt` is 2.100 m against `scale.ron reference.door_height_m 2.1`, the three half-timbered
houses 4.500 / 8.000 / 11.500 against `heights_m`, seven wall pieces exactly 120.000 against
`wall.height_m`, `a-001-basis-rig-vanguard` 1.800 against `human_height_m`. **`scale: 1.0` is
therefore the measurement and not a default**, and
`tests/render.rs::f030_every_configured_row_is_drawn_at_the_scale_it_was_authored_in` holds it
down.

### Three things the first real `.glb` broke, and where they are fixed

1. **`scale:` moved the mesh and not the anchors.** `position_in` composes the chain up to but
   not including the scene child, and the scene child is what carries the model's transform —
   so a row at `scale: 2.0` rendered at double size with its kill zone at the single-size
   height, silently. The anchors are now transformed with the mesh
   (`read_the_models_anchors`), and `f030_the_fit_reaches_the_anchors_and_not_only_the_mesh`
   goes red if that is undone.
2. **The pack faces the other way.** `docs/conventions.md` and `titan::rig` put a body's
   forward at **-Z**; the drop authors its faces at **+Z**, and says so twice in every file
   with a front (`a-042-koerpertyp-a-hager-mittel`: `eye` at z = +0.92, nape `cortex` at
   z = -0.139; `a-136-npc-vanguard`: `eye` at z = +0.20). Unturned, an aggroed husk walking at
   the player renders its **back** to him. `render::model::MODEL_FACES` turns the mesh **and**
   the anchors into the game's frame; `f030_a_model_arrives_turned_into_the_games_own_frame`
   pins both halves.
3. **One logical name has to dress two size classes.** `titan.ron` gives `titan_husk` to three
   medium kinds *and* two small ones, the way the cuboid rig has always been built — one shape
   at the class height. `render::model::fit_to_class` brings a model to the entity's own size,
   preferring the **cortex** as the yardstick and falling back to the `hit.min`/`hit.max` pair.
   Measured, which of the two to give up: fitting by *height* instead moved the husk's kill
   zone from 8.90 m to 8.85 m, and **five** tests in `tests/titan.rs` went red on those 5 cm.
   Fitting by *cortex* moves the silhouette 0.6 % and moves no hit zone at all. The height then
   carries the check the cortex can no longer carry — a model whose cortex sits at the wrong
   fraction of itself now warns about its height instead.

⚠️ **`hit.min`/`hit.max` is a corner pair, not an ordered AABB.** On all 278 files
`hit.max.z < hit.min.z`, from Blender's +Y-forward to glTF's -Z-forward. Never `max - min`;
`authored_height_m` takes an absolute value and the next consumer takes a componentwise
min/max.

### ✅ Closed: the nape is 0.14 m deep in the pack and 0.55 m in the game

`titan::rig::cortex_in_head` builds the kill zone at `head_m * 0.5` — **0.55 m** behind the
head's axis on a medium body, 0.77 m on a large one — and Q-030's whole lesson, *the nape is cut
from behind, never from the front*, is that depth. The drop puts its `cortex` empty on the
**skin** of the neck instead: **0.139 m** off the neck axis on the medium body, **0.194 m** on
the large one, 0.07–0.30 m across all 26 full bodies. Taken literally that moved the kill zone
~0.4 m forward and made a husk cuttable from the FRONT (measured: blade **-0.066 m** past the
cortex on a front pass).

**The fix that landed on 2026-08-18 is a clamp, not a drop:**

```rust
// src/titan/rig.rs
pub fn cortex_in_head_from_model(&self, anchor: Vec3) -> Vec3 {
    let local = anchor - Vec3::new(0.0, self.head_centre_m(), 0.0);
    Vec3::new(local.x, local.y, local.z.max(self.head_m * 0.5))
}
```

**The model decides the Cortex's height and its side; the rig decides the minimum depth.** A
model that puts its nape *further back* is still believed — that direction can only sharpen the
approach angle, and it is the only direction that cannot do damage. The obvious patch (hard-drop
x and z, `Vec3::new(0.0, anchor.y - head_centre, head_m * 0.5)`) was rejected because it takes
the existing 🟧 `f030_a_models_cortex_anchor_beats_the_computed_position` red: *the nape is a
point, not a height.*

It also could not have been "obey the model", because **the drop does not agree with itself**:
`a-042-…-mittel` puts `cortex` at z = -0.139 (on its own `halswulst` mesh), while
`a-040-titan-basis-rig` and `a-046-cortex-mesh` both put the same anchor at **-0.450**, the
middle of the amber blob. There is no single "what the model says" to obey.

> 📌 **For the asset pipeline:** the `cortex` empty belongs at the **centre of the amber blob**,
> the way `a-046-cortex-mesh` already places it — not on the neck skin. If the 26 body files did
> that, the clamp would never fire and the model would genuinely own all three components.

### 🔴 The open one: 1.39 cm of x keeps `titan_large` on `Primitive`

`titan_husk` ships **bound** (`a-042-koerpertyp-a-hager-mittel.glb`) and dresses five of the
eight titan kinds — husk, errant, chorus, scuttler, weaver. `titan_large` does not, and the
reason is one component:

| component | computed by the rig | out of the model | equal? |
|---|---|---|---|
| height above the feet | 12.50 m | 12.50 m (`fit_to_class` fits **by the cortex**) | yes |
| depth behind the neck | 0.77 m | 0.19 m → clamped to 0.77 m | yes |
| **x, off the centre line** | **0.0** (`rig.rs:184`) | **-0.0139** (authored +0.0139, turned by `MODEL_FACES`) | **no** |

Measured on a spawned warden with the row bound: *"the model's `cortex` empty moves the kill
zone from 12.50 m to 12.50 m above the feet, and its depth of 0.19 m was held back to the rig's
0.77 m"*. And that centimetre flips a pinned 🟧 measurement —
`tests/titan.rs::q031_the_nape_survives_a_titan_who_tracks_you` goes red with
*"warden: the pass at 0.2 m of air lands again (blade -0.020 m)"*. Q-031 pinned that a `large`
titan who turns while you cross eats the 0.20 m pass and leaves 0.15 m; with the model bound the
warden gets **easier**, silently, out of an art file.

Two readings, and they point the same way. Q-031's margin at 0.20 m was about a centimetre wide,
which is worth knowing on its own — and the drop's x is authoring noise rather than anatomy
(+0.010 on the medium body, +0.0139 on the large, and a nape is on the centre line). So the
likely one-line fix is in `cortex_in_head_from_model`: take the model's height and its depth,
leave x at the rig's 0.0, exactly as the depth is already clamped. Until somebody makes that
call the row waits — **a binding that takes a 🟧 test red is a wrong binding, not a red test.**

The repro costs no rebuild, because `art.ron` is data:

```bash
sed -i 's|"titan_large":    (source: Primitive|"titan_large":    (source: Gltf("3d/glb/a-042-koerpertyp-a-hager-gross.glb")|' assets/data/art.ron
cargo test --test titan 2>&1 | grep -E '^test result'
```

### Still not consumed

- **439 `hook.*` empties across 144 files** — eaves (`hook.traufe`), ridges (`hook.first`),
  crowns (`hook.krone`), the wall's cornice ladder (`hook.gesims_15..105`). **The loader keeps
  them since 2026-08-18** (`hook.` is an open family, see the anchor table): a wall segment that
  logged *"2 anchor(s) read"* now logs *"11 anchor(s) read out of the file, 9 of them hook.*
  rope points"*. What is still missing is **two links, not one**, and neither is the loader's:
  1. **No world block carries a `ModelName`**, so **0 of the 439 reach the loader in the running
     game** — they all sit in the architecture kit. `ModelName` lives in `src/render/model.rs`
     and `world` has no allow-list edge to `render`, so it has to move to `shared` (where
     `ModelAnchors` already is) before `BlockPlan::spawn` can insert one.
  2. **Nothing consumes a per-model anchor.** `vector::aim::cast` raycasts avian colliders and
     `vector::hook::anchor_target` takes the ray's hit point; neither ever reads `ModelAnchors`.
     The only consumer of any anchor in the tree is `titan::rig::cortex_from_the_model`, and it
     reads `cortex` alone.
- **`hit.min`/`hit.max`, `hand.l`/`hand.r`, `eye`** — read onto every entity, consumed by
  nothing but `fit_to_class`. `eye` is a measured first-person camera height sitting unused
  (1.69 m on `a-136-npc-vanguard` against a 1.8 m capsule).
- **Zero animation clips in all 278 files.** `animations: {}` is the only honest value; a name
  that is not in the file brings the cuboid rig back on screen through `PrimitiveFallback`.
- **Nine of the ten logical names have nothing that spawns them.** Only
  `render::model::name_the_titans_model` inserts a `ModelName`, so houses, walls, streets,
  blades, gear and the player's own body cannot wear a model however good the match is. Two of
  the nine are nearly there and two are not:
  - `house_small` / `house_town` / `house_large` — `world::map` now **plans** a model name
    (`BlockPlan.model`, `world::map::dress_for`) and `dress_for` refuses any name whose `art.ron`
    row is not `Gltf(...)`, so these three rows are the switch for the whole district. With the
    switch on: **602 of 937 houses dressed, 5155 → 3393 blocks (−34 %)**. They stay `Primitive`
    because `BlockPlan::spawn` does not insert the `ModelName` yet — flipping them today would
    rewrite the footprints and drop every cuboid roof while nothing renders.
  - `vanguard` / `blade` — the match is measured (`a-136-npc-vanguard` is 1.814 m against
    scale.ron's 1.8), but the camera is a **child** of the player at eye height, so a body model
    on the local player is seen from inside. That is a first-person/third-person decision, not a
    row.
  - `tree_giant` — **a missing model, not a scale factor**: `heights_m.tree` is 12.0 and the
    drop's smallest giant tree is 34 m.
  - `crate_resupply` — `a-130-nachschubstation` (3.90 × 4.20 × 3.35) would replace a silhouette
    that was hand-tuned on purpose (FIND-080) and needs its own before/after picture.

## The same chain three more times: atlas, sound, VFX

**The rule behind the model chain is not about models.** It is always the same four links:

```
script (the source, in the repo)  →  generated asset  →  a RON switch  →  the game
```

It holds for a color atlas and a sound exactly as it holds for a mesh. **Whoever starts a new
*kind* of asset builds the chain with it** — not "by hand just this once", because "just this
once" is how an asset ends up with no source anybody can edit.

| Chain | Source | Result | Switch |
|---|---|---|---|
| Model | `tools/blend/<name>.py` → `.blend` (**the user's**) | `assets/3d/glb/<name>.glb` | `art.ron` |
| Atlas | `tools/atlas/<name>.py` | `assets/textures/atlas/` + the UV assignment as RON | the registry |
| Sound | `tools/sound/<name>.py` | `assets/audio/sfx/<name>.ogg` | the registry |
| VFX | — (data, not code) | `assets/vfx/<name>` as a definition | the registry |

### The registry: one line per asset, one line to swap it

The code **never** asks for a file, always for a **logical name**. `art.ron` above is the model
half of it; the same table carries sounds and effects:

```ron
"sfx_hook_hit": (sound: File("audio/sfx/hook_hit.ogg"), volume: 0.8, use_asset: true),
"sfx_gas":      (sound: Recipe("gas_hiss"), volume: 0.5, looping: true, use_asset: true),
"vfx_steam":    (vfx: Recipe("steam"), use_asset: true),
```

`use_asset: false` falls back to the **placeholder path** — a primitive, a silent sound, no
effect. **Both paths have to run at all times and have the same size and timing**, or the switch
is a rebuild rather than a switch. A missing asset **crashes at load** with its name, not
silently as a white cube in the middle of the game.

### Color: atlas or vertex colors — decide, and write it down

The bible gives the direction: **the environment runs off one single atlas** (color consistency,
few draw calls), **figures and titans may use vertex colors** (survives remodeling, no UV work).
Which applies to which asset stands in the registry's `color:` field and **not in the code**.

And the three signal colors — cyan, amber, crimson — are reserved for gameplay: **they may not
appear in the atlas as decoration** ([`docs/gameplay/world.md`](gameplay/world.md)). That is not a
preference, it is the reason the game is readable at speed.

### Sound: a recipe, and **measured instead of heard**

**There are no ears in this environment.** So a sound is finished exactly when it is
**measurable**: length, base frequency, envelope, peak level, and whether it loops (start ==
end). A recipe (`tools/sound/<name>.py`) is the source, the `.ogg` is the result, the registry is
the switch — the same three links as a model.

**Only original or licensed music**, ever. Third-party soundtracks are a legal dead end and the
bible files it under risks; references may be listened to and described, never taken
([`docs/QUESTIONS.md`](QUESTIONS.md) Q-006).

### Where the files live

```
assets/
  data/                 the RON files — including the registry
  3d/
    blend/              ⭐ THE SOURCE, hand-editable — this is where the user goes
    glb/                GENERATED — never touch by hand, committed anyway
  textures/
    atlas/              GENERATED: the ONE environment color atlas, out of tools/atlas/
    hand/               hand-made PNGs — the generator never touches these
  audio/
    sfx/                generated or hand-made
    music/              original or licensed only
  vfx/                  effect definitions (data, not code)
  extern/               ⭐ DOWNLOADED PLACEHOLDERS — strictly separated from our own
    3d/  audio/  textures/
    ATTRIBUTION.md      one line per file: URL · date · licence · what it replaces
```

**Three separations that do not get blurred:** `tools/` **builds** things, `scripts/` **plays**
the game, `assets/` holds the **result** — and `assets/extern/` alone holds anything
third-party.

## Own and third-party stay separated

The plan is: **in the end the user replaces everything himself, piece by piece.** That works
exactly when **one command** can answer at any time: *what is still third-party, where does it
live, what is it supposed to become?*

1. **Third-party files live exclusively under `assets/extern/`.** Never in `3d/blend`,
   `3d/glb`, `audio/sfx` — that is where our own work lives. That one separation is the whole
   trick.
2. **`assets/extern/ATTRIBUTION.md` lists every file**: file name · URL · date · license ·
   **which own asset it later becomes**. Without an entry no file may lie there.
3. **In the registry every third-party asset carries `attribution:`.** That makes the
   replacement list a `grep`, and `cargo test --test assets -- --ignored --nocapture` prints
   the report: *asset · own/third-party · source · replaced by*.
4. **The public repo does not get the third-party files.** `assets/extern/` is in
   `.gitignore`; what ships with it is `ATTRIBUTION.md` and `tools/fetch_external.sh`, which
   fetches everything back.
5. **A break in style is a bug, even in a placeholder.** A highly detailed third-party model
   next to low-poly work of our own falsifies the judgement about movement and readability.

## Where it stands

`cargo test --test models -- --ignored --nocapture` is meant to print the table: *model ·
`.blend` there? · `.glb` current? · painted? · anchors complete? · wired into the RON?* — and
**that test file does not exist.** What exists today is `cargo test --test render` (ten
`f030_*` tests) and `cargo test --test titan` (one), and between them they answer everything
that can be answered without a single file: a row without a model stays a primitive, a row with
one produces a scene, the scene hides the cuboid, the state plays the clip, a clip that is
missing says so and brings the cuboid back, and the `cortex` empty moves the kill zone.

**Since 2026-08-18 that is no longer only synthetic.** `cargo test --test render` is 38 tests,
and three of them read the shipped registry itself rather than a fixture:

| test | what a wrong `art.ron` costs, and what goes red |
|---|---|
| `f030_the_shipped_registry_binds_exactly_the_rows_that_have_a_home` | pins the bound set in **both** directions — unbinding is silent (every other model test then runs over an empty set and stays green), and binding too much is silent too |
| `f030_every_glb_art_ron_names_even_in_a_comment_is_a_file_that_exists` | the eight `.glb` paths the registry writes down, **including the ones only a blocker note names**, are real files |
| `f030_a_bound_row_names_no_clip_because_its_file_carries_none` | reads the `.glb`'s own JSON chunk: an invented clip name is not "an unanimated model", it is **no model at all in that one state** |

plus `f030_a_bound_glb_agrees_with_scale_ron_about_the_body_it_dresses`, whose loop was allowed
to be empty while everything was `Primitive` and no longer is. All four were seen red on data
alone — `art.ron` is data, so a Rule-5 counter-check on it costs no rebuild at all.

| 2026-08-18 | state |
|---|---|
| `assets/data/art.ron` as the one switch | 🟧 — one row bound in the SHIPPED file, `--headless --ticks 300` exit 0 with **zero** asset warnings, and the four headless lines are byte-identical to the all-`Primitive` baseline |
| primitive fallback, in both directions | 🟨 |
| the primitive hidden under an arrived model | 🟨 built, seen red first, **on a synthetic scene** |
| `.glb` really loading, painted, upright | 🟧 — `docs/images/t075-titan.png`: span 384 px vs 391.9 predicted (−1.5 %), implied height 9.853 m against class 10.0, feet +0.119 m off the ground plane. ⚠️ That picture was taken through a **mirror asset root**; what changed on 2026-08-18 is that the **shipped** `art.ron` names the same file, so the next window run repeats it with no fixture |
| anchors read out of the model | 🟧 — `model "titan_husk": 6 anchor(s) read out of the file`, real binary, shipped `art.ron` |
| the `cortex` anchor moving the kill zone | 🟧 — on the shipped binding: *8.90 m → 8.90 m, depth 0.14 m held back to the rig's 0.55 m* |
| the other anchors (`hand.*`, `eye`, `hook.*`) | 🟨 kept by the loader since 2026-08-18, **consumed by nobody**, and **0 of the 439 `hook.*` reach it in the running game** |
| animation clips resolved by name | 🟨 — and there is nothing to resolve: **0 clips in 278 files** |
| a game state playing its clip | 🟨 built, **on a hand-made `AnimationClip`** |
| `titan_large` bound | ⬜ — blocked on 1.39 cm of x, see the 🔴 section above |
| a house / wall / tree wearing a model | ⬜ — planned by `world::map`, and no entity carries a `ModelName` |
| the `.blend` half of the chain | ⬜ |

**The four stages apply to models just the same** (§8): a placeholder out of primitives is ⬜ or
🟨 — never more, no matter how well it fits in. Only a real model that somebody has **seen** is
🟧.

Related: [`docs/conventions.md`](conventions.md) · [`docs/environment.md`](environment.md) ·
[`docs/STATUS.md`](STATUS.md)
