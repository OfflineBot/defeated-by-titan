# models — the asset chains (models, atlas, sound, VFX), and how YOU swap a model

Updated: 2026-08-12 · Stage: ⬜ (the chains are described, none of them is built: there is no
`tools/blend/`, no `tools/atlas/`, no `tools/sound/` and no `assets/3d/` in this repo yet, and
machine A has no Blender — see below and [`docs/environment.md`](environment.md))

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

1. Open `assets/3d/blend/<name>.blend` in Blender.
2. **Swap the geometry.** The anchors (the empties, see below) are already in place — leave
   them where they are, or push them where they belong on *your* model.
3. Save. **Nothing else.** At the next game start the game sees that the `.blend` is newer
   than the `.glb`, exports it again and uses it.
4. If the model is still on `use_blend: false`: set **one line** in `assets/data/art.ron` to
   `use_blend: true`.

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

> **If an empty is missing, the zone is a point** — and a cortex that is a point feels like a
> broken game. That is why `tests/models.rs` falls over when a model with `use_blend: true` is
> missing a required anchor.

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
| Anchor range | 200 m | 90 m until 2026-08-10, see below |
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

```ron
models: {
    "vanguard":     (blend: "vanguard",     use_blend: true,  scale: 1.0),
    "titan_husk":   (blend: "titan_husk",   use_blend: false, scale: 1.0),  // still a placeholder
}
```

`use_blend: false` ⇒ the game builds the **procedural placeholder** out of Bevy primitives
(capsule/box/cylinder, tinted). `use_blend: true` ⇒ it loads the `.glb`. **Both paths have to
work at all times**, and both use the same anchors, the same hit zone and the same scaling —
otherwise switching is not a switch but a rebuild.

**No file name in Rust code.** An `asset_server.load("titan.glb")` in the middle of a system is
a bug; there is **one** place that reads the registry (`data/`), everybody else asks for the
logical name. `tools/norms.py` checks it.

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

`cargo test --test models -- --ignored --nocapture` prints the table: *model · `.blend` there? ·
`.glb` current? · painted? · anchors complete? · wired into the RON?* — exactly what another
agent wants to read in ten seconds.

**The four stages apply to models just the same** (§8): a placeholder out of primitives is ⬜ or
🟨 — never more, no matter how well it fits in. Only a real model that somebody has **seen** is
🟧.

Related: [`docs/conventions.md`](conventions.md) · [`docs/environment.md`](environment.md) ·
[`docs/STATUS.md`](STATUS.md)
