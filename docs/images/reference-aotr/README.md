# reference-aotr — screenshots of the REFERENCE game, not of ours

Updated: 2026-08-23 · Stage: 🟧 (captured from the running game, read and measured)

**What this is.** 20 captures of *Attack on Titan Revolution* (Roblox), map **Shiganshina**,
taken on the Windows machine on 2026-08-23 while the user played — the first time the reference
and this project ran side by side. `aotr-shiganshina-01..20.png`, **exactly 4 seconds apart**,
1600 px wide, cropped to the game window only.

**Why they are in the repository.** The user asked for it in so many words, and the reason is that
the Windows machine is not where the work happens: *"speichere alle screenshots im projekt! damit
man spqter auf linux weiter entwickeln kann!"*. Without them, everything measured here would have
to be re-measured on a machine that cannot run the reference at all.

**They are not evidence for any `F-ID`.** They are reference material — the city's scale and
density, the roof spacing, the HUD, the wall, the titan markers, and what the player is doing
while flying. Which is why the folder is registered as an asset pack in `tools/norms.py`
(`ASSET_PACKS`) instead of each file having to be linked from a doc: the mention rule would fire
20 times on 20 healthy files.

## What has already been read out of them

| finding | what the images gave |
|---|---|
| **`FIND-150`** | the **gas percentage in every frame**, 4 s apart: `61 61 60 59 58 55 54 53 52 51 50 48 47 46 42 41 41 39 39 37`. That is 0.316 %/s average, ~400 s per tank at ordinary flight, ~100 s pushed, and **exactly 0 while idle**. The reference research had filed tank and burn rate as *"unknown and not obtainable"*. |
| **`FIND-149`** | the HUD carries **no speed readout** — gas %, blades 3/3, five ability slots, GOLD/LUCK/EXP multipliers, objectives. The one candidate for a distance is the `N/A` field under the crosshair, still untested. |

## What is still in them and has not been read

- **A ruler.** No frame yet has the character flat against a building with both fully in view, so
  the character's 5 studs have not been turned into a metre scale. Until that exists, `Gear Shift`
  at "600 m/s" cannot be checked against our `max_speed_m_s: 75` — see `FIND-149`, the unit
  question.
- **Roof spacing and street width**, which would give a distance per swing.
- **The city silhouette** against ours: the wall dwarfs the houses here in a way `maps.ron` does
  not reproduce (`docs/NEXT.md`, the 58 m swing-gate note).

**How more were taken**, if the sequence needs extending: the capture scripts are
`shot.ps1` (one window) and `burst.ps1` (N shots every M seconds) — both written to the session
scratchpad on the Windows machine, both plain PowerShell with `System.Drawing`, no install.

Related: [`../../FINDINGS.md`](../../FINDINGS.md) FIND-149 · FIND-150 ·
[`../../windows.md`](../../windows.md) · [`../../gameplay/references.md`](../../gameplay/references.md)
