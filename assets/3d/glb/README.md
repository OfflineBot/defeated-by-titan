# assets/3d/glb — the models the game actually loads

**This folder is tracked in git, and that is not an accident.** `docs/models.md` and
`prompts/init.md` §18 both say the same thing: the game has to run on a machine **without
Blender**. A `.glb` that only exists after somebody exports it is a game that only runs for
whoever has Blender — so the exported file is committed, not the recipe alone.

`assets/3d/blend/` next to it holds the `.blend` sources; `.blend1`/`.blend2` backups are
ignored.

## Nothing in here yet — and the game runs anyway

Every row in `assets/data/art.ron` says `source: Primitive` today, so **nothing here is asked
for**. `tests/render.rs::f030_the_repository_runs_with_not_a_single_glb` keeps that honest: it
goes red the moment a row names a file that is not committed beside it.

## What goes in here

Our own work only. Third-party files live under `assets/extern/`, which **is** in `.gitignore`,
and every one of them has a row in `assets/extern/ATTRIBUTION.md` (`docs/models.md`).

How to put a model in: [`docs/models.md`](../../../docs/models.md), section *"For the user:
this is how I swap a model"*.
