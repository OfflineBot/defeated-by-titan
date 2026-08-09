# conventions — axes, units, terms, norms

Updated: 2026-08-09 · Stage: 🟨 (settled and written down; `tools/norms.py` checks the
mechanically checkable part)

> **Two forms for the same thing means no form.** A norm is not about pretty, it is about
> **greppable**: `git log --oneline | grep F-014` has to answer the history of a feature
> (`prompts/init.md` §10). Whoever starts something that will recur norms it **here**, before
> using it a second time.

## 1. Axes, units, looking direction

| Decision | Value | why |
|---|---|---|
| **Length unit** | **1 Bevy unit = 1 meter** | A human is 1.8; a titan 3–15; a hook flies 60–120. Numbers you can check in your head. |
| **Up** | **+Y** | Bevy default, not negotiable |
| **Looking direction** | **−Z**, `yaw = 0` means looking along −Z | Bevy camera default. A model is turned **in the `.blend`**, never by an offset in the config: one offset field per model is the beginning of thirty offset fields. |
| **Blender** | model Z-up, export with `export_yup=True` | The exporter turns it. **Do not rotate it yourself**, or it turns twice. |
| **Origin of a body model** | **between the feet** | otherwise every model stands half in the ground |
| **Angles** | **radians** in code, **degrees** in RON and scripts | Degrees can be read and typed, radians can be computed with. Conversion exactly at the boundary. |
| **Time** | seconds. Simulation in `FixedUpdate` at **60 Hz** | §6 rule 4: over the network a frame-dependent result is not a comfort problem, it is desync. |

### 1 stud = 0.28 m

Backlog and bible are written in **studs** (the Roblox unit), this project computes in meters.
The conversion factor is the Roblox value **0.28 m/stud**, and it is cross-checked against the
backlog's own numbers:

| Backlog number | × 0.28 | plausibility |
|---|---|---|
| hook range 400 studs (`F-002`) | **112 m** | `prompts/init.md` §1 says „ein Haken fliegt 60–120" — *a hook flies 60–120* ✓ |
| Ashgate District 2000 × 2000 studs | **560 × 560 m** | a city you cross in 5–7 min ✓ |
| Titanwood 3000 × 3000 studs | **840 × 840 m** | the largest map ✓ |

> `ASSUMPTION:` The factor is inferred, not confirmed by the user — the three cross-checks
> above are the only evidence. Recorded in `docs/QUESTIONS.md` as **Q-002**. **The conversion
> happens once, when a number is taken into an `assets/data/*.ron`** — there are no studs in
> the code, and `tools/norms.py` goes red if "stud" turns up in `src/`.

## 2. Terms — sheet `10_Namensschema` is binding

**No reference term in the code, in assets, in the UI or in the docs.** A `nape` field or an
`odm` module is a mistake, and `tools/norms.py` finds it.

Machine-readable: [`docs/backlog/naming.ron`](backlog/naming.ron) (40 rows, generated). The
important ones:

| instead of | **here** | in the code |
|---|---|---|
| ODM Gear / 3DMG | **Vector Gear** (VG) | domain `src/vector/` |
| Nape | **Cortex** | the empty in the model is called `cortex` |
| Survey Corps / Scouts | **The Vanguard** / a Vanguard | `src/player/` |
| Thunder Spear | **Lance Charge** | (roadmap) |
| Titan Shifting / Shifter form | **Bonding** / **Vessel Form** | (roadmap) |
| Titan Serum | **Ichor Vial** | |
| Family / Clan · Perk · Artifact · Memory · Prestige | **Lineage** · **Trait** · **Relic** · **Echo** · **Ascension** | `src/progress/` |
| Gold · Gems | **Mark** · **Sigil** | |
| Pure · Abnormal · Crawler · Ducker | **Husk** · **Errant** · **Scuttler** · **Weaver** | `assets/data/titan.ron` |
| *(new)* | **Warden** · **Lurker** · **Bellower** · **Chorus** | |
| Attack · Female · Armored · Colossal Titan | **The Bound One** · **The Dancer** · **The Bulwark** · **The Ashwalker** | raid bosses |
| Town Central | **The Rookery** (hub) | |
| Shiganshina · Trost · Outskirts · Forest of Giant Trees · Utgard · Docks · Stohess | **Ashgate District** · **Brackwall** · **The Fallow** · **Titanwood** · **Hollowkeep** · **Saltpier** · **Highspire** | `assets/data/maps.ron` |
| Walls Maria/Rose/Sina | **Ashgate / Ironrose / Highspire Ring** | |

**Three spellings of the project name, each in exactly one place:** crate
`defeated_by_titan` (Rust does not like hyphens) · window title **"Defeated by Titan"** ·
GitHub repo `defeated-by-titan`. **No plural s.**

## 3. The three signal colors — not negotiable

| Color | meaning | may appear nowhere else |
|---|---|---|
| **Cyan** | gas, Vector Gear, anchor points | no cyan set dressing |
| **Amber** | cortex, weak points, objectives | no amber lanterns |
| **Crimson** | danger, damage, critical state | no red roofs |

The base palette stays muted against them: stone gray, brick red, olive, sand. This rule is
the reason a player at full speed, in a fight with twenty team mates, still recognizes what is
relevant to him (bible 3.4). **It holds for placeholders just as much.**

## 4. Names in the repo

| What | Norm | Example |
|---|---|---|
| **Commit subject** | `<F-ID\|scope>: <one line, what is different now>`, max 72 characters, English, active, no period, no emoji | `F-014 vector: gas drain while boosting` |
| **Commit scopes** | exactly five, for when there is no F-ID | `docs:` `test:` `tool:` `fix:` `chore:` |
| **Branch** | `<f-id>-<short>` or `<scope>/<short>` | `f014-gas-boost`, `fix/hook-edge` |
| **Test file** | `tests/<domain>.rs` | `tests/vector.rs` |
| **Screenshot** | `docs/images/<f-id>-<short>[-before\|-after].png` | `docs/images/f014-boost-after.png` |
| **Script** | `scripts/<f-id>-<short>.txt`, containing `mark <f-id>-<keyword>` | `scripts/f014-boost.txt` |
| **STATUS row** | `\| Item \| ID \| Stage \| Evidence \| Note \|`, date in **ISO** with the machine | `… \| 🟧 \| tests/vector.rs … \| 2026-08-09 [cachy] \|` |
| **Bug** | `B-007 <title>` + the four fields from §9 | `B-007 hook does not hold on an edge` |
| **Question** | `Q-003 <question>` + context + `ASSUMPTION:` | |
| **Foreign find** | `FIND-005 <symptom>` + measurement | |
| **Doc header** | `# <name> — <one sentence>`, below it `Updated: <ISO> · Stage: <mark>` | |
| **Test name** | `<f_id>_<the claim that holds>` — not `test_gas` | `f014_boost_consumes_gas` |
| **RON key** | `snake_case` (bound to the field names) | `hook_range_m` |
| **Rust** | `snake_case` functions, `CamelCase` types, **domain folders always singular** | file `src/vector/hook.rs`, in it `fn fire_hook()` |
| **Subagent report** | fixed: `Task · Done · Evidence · Stage · Open · Findings` | a free-text report cannot be integrated |

⚠️ **No tool or author traces in commit messages, PR descriptions or tags.** No
`Co-Authored-By`, no signature, no "generated with", no model name. A commit message describes
**the change**, not its author — the author is in git's author field and nowhere else.
`tools/norms.py` fails a message containing `Co-Authored-By`, `Generated`, `Claude`, `AI` or
`🤖`.

### One language: English, everywhere (user, 2026-08-09)

The user said it three times in one day, and the third sentence overrides the first two:

> „programmiere alles in deutsch. filebenennung! code aber auch comments alle auf englisch!"
>
> „wenn nicht sicher dann eher englisch!"
>
> „es sootlle im bestfall nirgendwo deutsch sein! alles auf englisch!"

*(Write everything in German — file naming! — but code and comments all in English · if in
doubt, rather English · ideally there should be German nowhere, everything in English.)*

**Everything is English.** File and folder names, module names, types, fields, functions,
variables, comments, test names, RON keys, the documentation, HUD and log output, the script
driver's metrics, tool output, commit messages. There is no second category, and therefore no
boundary left to argue about — which is the point: the two previous rules ("one language:
German, throughout", then "two languages, and the boundary runs at the file edge") both cost
more time in border disputes than they ever saved in reading.

German stays in exactly three places, and each time for the same reason — **it is not ours**:

- `prompts/init.md`, `prompts/DefeatedByTitan_Design-Bibel.md` and `gameplay/` — the user's own
  documents. They are translated **into** work, not translated as text.
- Everything quoted **out of** them: the sheet tab names (`01_Spielfunktionen` …
  `11_Zusammenfassung`) that `tools/features.py` looks up, the column headers it maps, and the
  German prose that flows from `features.xlsx` through `docs/features.ron` into
  `docs/STATUS.md` and `docs/TODO.md`. A quotation that gets translated stops being a
  quotation, and a translated dict key silently stops matching.
- The **git history**. Old commit messages are the record of what happened, not documentation.
  The new commit norm applies from the migration commit onward, not backwards.

Umlauts are transcribed (`ae oe ue ss`) wherever German still legitimately appears in source,
RON keys or file names; in running text (Markdown, UI text) they are written properly.

The migration was done in one pass, not mixed in a little at a time — a half-migrated tree has
two words for every thing, which is the state this section exists to prevent. Its history and
its rollback point are in [`docs/QUESTIONS.md`](QUESTIONS.md) Q-024.

## 5. Units in the name

Whoever declares a constant that measures meters writes the **unit into the name** — or the
comment at the place it is used, and the number into the RON (§4): `hook_range_m`, `gas_per_s`,
`windup_s`. A field called `range` is a question, not a value.

## 6. No orphan files

**Every file in the repo is either linked and used — or deleted. There is nothing in between.**
Every `docs/*.md` stands in [`docs/README.md`](README.md), every asset in the registry, every
`tools/` script in some document. Whoever creates a file links it **in the same commit**.
`tools/norms.py` checks both: no dead Markdown link, no unreferenced file. **Nothing is kept
"just in case"** — no `*_old.rs`, no `titan_v2.blend`. Git is the backup.

Related: [`docs/architecture.md`](architecture.md) · [`docs/models.md`](models.md) ·
[`docs/backlog/naming.ron`](backlog/naming.ron)
