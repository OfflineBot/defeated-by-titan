# tools/fleet — the drawn-vs-collider instruments, in-repo

The FIND-225 lesson enforced: an instrument that lives only in a session scratchpad is
destroyed overnight and its numbers become unauditable. These are the 2026-09-02 B-042/
B-043 instruments, kept where the next fleet run can re-derive every number:

- `measure.py` — remnant/prop glb voxelization (0.20 m surface + flood-filled interior,
  6-band slices, 0.25 m occupancy grids) that derived the `art.ron` hulls rows.
- `validate.py` — the analytic fleet: compound-surface-to-mesh distance per class; its
  `HULLS` dict also PRESERVES the five validated placed-prop rows held back by the
  `tests/world.rs` cuboid guard (see B-043 attribution, F1).
- `glb_slice.py` — the B-042 attribution oracle (drawn titan surface vs capsules).
- `fit_caps.py` / `verify_caps.py` — the titan `drawn_poses` capsule fit and its check
  (23 780 surface samples; flesh-outside-capsule and registering-air tables).

They read the repo's glb/ron files read-only and print tables; none of them writes.
