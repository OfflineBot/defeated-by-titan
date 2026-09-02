#!/usr/bin/env python3
"""Validate candidate hulls against the glb meshes.

Per class:
  * containment: every box inside [-ax/2, ax/2] x [0, ay] x [-az/2, az/2] (envelope frame);
  * phantom: sample the compound's EXPOSED surface (points not inside another box),
    distance to the nearest mesh triangle -> median / p95 / max;
  * coverage: fraction of mesh surface samples within 0.30 m of the compound;
  * envelope baseline: the same phantom numbers for the single envelope cuboid.
"""
import math, sys
from measure import triangles, sample_tri, GLB_DIR

# name -> (glb, envelope [ax, ay, az], [(min),(max)] boxes in authored metres)
HULLS = {
    "ruin_roof_collapsed": ("a-089-ruine-dach-eingestuerzt.glb", [7.04, 2.62, 5.16], [
        ((-3.50, 0.00, -2.56), (3.32, 0.85, 2.17)),
        ((-2.70, 0.85, -2.51), (2.70, 1.75, 1.50)),
        ((-2.70, 1.75, -1.98), (2.70, 2.19, 1.50)),
        ((-1.74, 2.19, -2.09), (0.97, 2.60, -0.93)),
    ]),
    "ruin_roof_half": ("a-089-ruine-dach-haelfte.glb", [6.72, 4.74, 4.93], [
        ((-3.34, 0.00, 0.90), (2.45, 0.79, 1.90)),
        ((-2.45, 0.00, -2.25), (2.45, 0.79, 0.90)),
        ((-2.45, 0.79, -1.89), (2.35, 3.40, -1.49)),
        ((1.95, 0.79, -1.49), (2.35, 4.40, 1.31)),
        ((-2.25, 0.79, 1.31), (2.35, 2.40, 1.91)),
    ]),
    "ruin_gable": ("a-089-ruine-giebel.glb", [6.47, 8.49, 4.01], [
        ((-2.75, 0.00, -0.60), (2.80, 0.80, 0.45)),
        ((-3.00, 0.00, 0.45), (-1.55, 0.80, 1.95)),
        ((1.85, 0.00, -1.95), (3.20, 0.80, -0.70)),
        ((-2.60, 0.80, -0.16), (2.59, 5.40, 0.24)),
        ((-2.20, 5.40, -0.16), (2.59, 6.60, 0.24)),
        ((0.99, 4.80, -0.54), (1.99, 8.40, 0.42)),
    ]),
    "ruin_heap": ("a-089-ruine-haufen.glb", [7.49, 2.40, 5.81], [
        ((-3.68, 0.00, -2.88), (3.61, 0.80, 2.71)),
        ((-3.15, 0.80, -2.30), (3.30, 1.20, 2.03)),
        ((-2.62, 1.20, -1.84), (2.71, 2.00, 2.07)),
        ((0.13, 2.00, -1.65), (2.83, 2.40, 0.00)),
    ]),
    "ruin_upper_floor": ("a-089-ruine-obergeschoss.glb", [6.95, 5.55, 4.93], [
        ((-3.46, 0.00, -1.77), (3.00, 0.80, 1.80)),
        ((1.75, 0.00, 1.80), (3.18, 0.80, 2.42)),
        ((-2.57, 0.80, -1.77), (-2.17, 3.60, 0.43)),
        ((2.03, 0.80, -1.77), (2.63, 3.00, 1.23)),
        ((-2.57, 0.80, 1.23), (0.63, 5.00, 1.63)),
        ((0.63, 0.80, 1.23), (2.63, 3.60, 1.63)),
        ((-2.57, 3.00, -1.77), (2.63, 3.60, 1.23)),
        ((2.03, 3.60, -0.37), (2.63, 5.00, 1.63)),
    ]),
    "ruin_pillar": ("a-089-ruine-pfeiler.glb", [5.86, 9.00, 4.87], [
        ((-1.25, 0.00, -1.05), (1.25, 0.80, 0.80)),
        ((-2.91, 0.00, -0.35), (-1.25, 0.80, 0.12)),
        ((-2.90, 0.00, 0.90), (-1.35, 0.80, 2.40)),
        ((1.05, 0.00, -2.10), (2.60, 0.80, -0.90)),
        ((-2.91, 0.80, -0.33), (-0.25, 2.60, 0.27)),
        ((-1.05, 0.80, -0.93), (0.95, 3.00, 0.87)),
        ((-0.85, 3.00, -0.73), (0.95, 8.40, 0.67)),
        ((-0.66, 8.40, -0.61), (0.76, 8.95, 0.61)),
    ]),
    "ruin_wall_corner": ("a-089-ruine-wand-ecke.glb", [6.47, 5.60, 6.80], [
        ((-0.25, 0.00, -3.38), (0.35, 3.20, 0.18)),
        ((-0.25, 3.20, -0.42), (0.55, 5.60, 0.18)),
        ((0.35, 0.00, -0.42), (3.22, 3.60, 0.18)),
        ((1.35, 3.60, -0.22), (3.22, 4.40, 0.18)),
        ((-1.80, 0.00, -3.36), (-0.25, 0.80, -2.60)),
        ((2.00, 0.00, 0.40), (3.22, 1.00, 1.95)),
    ]),
    "ruin_wall_high": ("a-090-schutt-balken.glb", [6.22, 6.94, 3.96], [  # placeholder path fixed below
        ((-3.05, 0.00, -0.55), (2.75, 0.80, 1.55)),
        ((-0.60, 0.00, -1.75), (1.20, 0.80, -0.55)),
        ((-2.60, 0.80, -0.17), (2.60, 6.20, 0.23)),
    ]),
    "rubble_beams": ("a-090-schutt-balken.glb", [4.10, 2.10, 3.70], [
        ((-2.03, 0.00, -1.80), (2.03, 0.70, 1.83)),
        ((-1.51, 0.70, -1.35), (1.37, 1.05, 0.92)),
        ((-0.65, 1.05, -1.05), (1.10, 2.10, 1.10)),
    ]),
    "rubble_cover": ("a-090-schutt-deckung.glb", [3.70, 1.20, 3.31], [
        ((-1.79, 0.00, -1.50), (1.74, 0.60, 1.64)),
        ((-1.79, 0.60, -1.22), (1.09, 1.20, 0.92)),
    ]),
    "rubble_flat": ("a-090-schutt-flach.glb", [3.94, 0.90, 2.95], [
        ((-1.84, 0.00, -1.43), (1.95, 0.45, 1.46)),
        ((-1.39, 0.45, -1.00), (1.48, 0.90, 1.36)),
    ]),
    "rubble_heap_large": ("a-090-schutt-haufen-gross.glb", [6.20, 3.00, 4.80], [
        ((-3.08, 0.00, -2.38), (3.08, 0.50, 2.38)),
        ((-2.71, 0.50, -2.09), (3.02, 1.00, 2.32)),
        ((-2.32, 1.00, -1.77), (1.99, 1.50, 2.07)),
        ((-2.14, 1.50, -1.51), (2.01, 2.50, 0.98)),
        ((-0.78, 2.50, -0.99), (0.94, 3.00, 0.65)),
    ]),
    "rubble_high": ("a-090-schutt-hoch.glb", [4.20, 1.80, 3.50], [
        ((-1.90, 0.00, -1.60), (2.06, 0.60, 1.71)),
        ((-1.57, 0.60, -1.36), (1.41, 1.20, 1.53)),
        ((-1.34, 1.20, -1.32), (1.27, 1.80, 1.07)),
    ]),
    "rubble_wall_piece": ("a-090-schutt-wandstueck.glb", [4.33, 2.40, 2.80], [
        ((-2.15, 0.00, -1.38), (1.80, 0.80, 1.38)),
        ((-1.08, 0.80, -0.25), (1.18, 2.40, 0.33)),
    ]),
    "market_stall": ("a-087-marktstand-zeltdach.glb", [4.20, 2.91, 3.64], [
        ((-1.57, 0.00, -1.48), (1.57, 1.94, 1.47)),
        ((-2.00, 1.94, -1.70), (2.00, 2.90, 1.72)),
    ]),
    "lamp_post": ("a-088-laterne-strasse.glb", [0.64, 4.20, 0.64], [
        ((-0.26, 0.00, -0.26), (0.26, 0.70, 0.26)),
        ((-0.11, 0.70, -0.11), (0.11, 2.80, 0.11)),
        ((-0.28, 2.80, -0.28), (0.28, 4.20, 0.28)),
    ]),
    "signpost": ("a-088-wegweiser.glb", [2.26, 3.60, 1.63], [
        ((-0.22, 0.00, -0.22), (0.22, 0.60, 0.22)),
        ((-0.08, 0.60, -0.08), (0.08, 3.60, 0.08)),
        ((-0.06, 1.80, -0.80), (0.07, 2.40, 0.06)),
        ((-1.06, 2.40, -0.06), (1.12, 3.00, 0.06)),
        ((-0.08, 3.00, -0.08), (1.12, 3.60, 0.08)),
    ]),
    "banner_long": ("a-133-banner-lang.glb", [1.24, 4.20, 0.68], [
        ((-0.07, 0.00, -0.08), (0.09, 4.20, 0.09)),
        ((-0.49, 0.70, -0.15), (0.49, 1.40, 0.08)),
        ((-0.58, 3.50, -0.08), (0.58, 4.20, 0.08)),
    ]),
    "hand_cart": ("a-131-karren-intakt.glb", [2.40, 1.80, 4.80], [
        ((-0.97, 0.00, -1.60), (0.97, 1.20, 1.45)),
        ((-0.74, 1.20, -1.35), (0.74, 1.80, 2.39)),
    ]),
}
HULLS["ruin_wall_high"] = ("a-089-ruine-wand-hoch.glb", HULLS["ruin_wall_high"][1], HULLS["ruin_wall_high"][2])


def point_tri_dist(p, a, b, c):
    # standard closest-point-on-triangle
    def sub(u, v):
        return (u[0] - v[0], u[1] - v[1], u[2] - v[2])

    def dot(u, v):
        return u[0] * v[0] + u[1] * v[1] + u[2] * v[2]

    ab, ac, ap = sub(b, a), sub(c, a), sub(p, a)
    d1, d2 = dot(ab, ap), dot(ac, ap)
    if d1 <= 0 and d2 <= 0:
        return math.dist(p, a)
    bp = sub(p, b)
    d3, d4 = dot(ab, bp), dot(ac, bp)
    if d3 >= 0 and d4 <= d3:
        return math.dist(p, b)
    vc = d1 * d4 - d3 * d2
    if vc <= 0 and d1 >= 0 and d3 <= 0:
        t = d1 / (d1 - d3)
        return math.dist(p, (a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t))
    cp = sub(p, c)
    d5, d6 = dot(ab, cp), dot(ac, cp)
    if d6 >= 0 and d5 <= d6:
        return math.dist(p, c)
    vb = d5 * d2 - d1 * d6
    if vb <= 0 and d2 >= 0 and d6 <= 0:
        t = d2 / (d2 - d6)
        return math.dist(p, (a[0] + ac[0] * t, a[1] + ac[1] * t, a[2] + ac[2] * t))
    va = d3 * d6 - d5 * d4
    if va <= 0 and (d4 - d3) >= 0 and (d5 - d6) >= 0:
        t = (d4 - d3) / ((d4 - d3) + (d5 - d6))
        return math.dist(p, (b[0] + (c[0] - b[0]) * t, b[1] + (c[1] - b[1]) * t, b[2] + (c[2] - b[2]) * t))
    denom = 1.0 / (va + vb + vc)
    v = vb * denom
    w = vc * denom
    return math.dist(p, (a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w))


class TriGrid:
    """coarse spatial hash so distance queries are not O(all tris)."""

    def __init__(self, tris, cell=1.0):
        self.cell = cell
        self.tris = tris
        self.grid = {}
        for i, (a, b, c) in enumerate(tris):
            mnx, mxx = min(a[0], b[0], c[0]), max(a[0], b[0], c[0])
            mny, mxy = min(a[1], b[1], c[1]), max(a[1], b[1], c[1])
            mnz, mxz = min(a[2], b[2], c[2]), max(a[2], b[2], c[2])
            for gx in range(int(mnx // cell), int(mxx // cell) + 1):
                for gy in range(int(mny // cell), int(mxy // cell) + 1):
                    for gz in range(int(mnz // cell), int(mxz // cell) + 1):
                        self.grid.setdefault((gx, gy, gz), []).append(i)

    def dist(self, p, cap=8.0):
        best = cap
        c = self.cell
        r = 0
        seen = set()
        px, py, pz = int(p[0] // c), int(p[1] // c), int(p[2] // c)
        while r * c - c <= best and r < 12:
            for gx in range(px - r, px + r + 1):
                for gy in range(py - r, py + r + 1):
                    for gz in range(pz - r, pz + r + 1):
                        if max(abs(gx - px), abs(gy - py), abs(gz - pz)) != r:
                            continue
                        for i in self.grid.get((gx, gy, gz), []):
                            if i in seen:
                                continue
                            seen.add(i)
                            d = point_tri_dist(p, *self.tris[i])
                            if d < best:
                                best = d
            r += 1
        return best


def inside_any(p, boxes, eps=1e-6):
    for mn, mx in boxes:
        if mn[0] - eps < p[0] < mx[0] + eps and mn[1] - eps < p[1] < mx[1] + eps and mn[2] - eps < p[2] < mx[2] + eps:
            return True
    return False


def box_surface_samples(mn, mx, step=0.25):
    pts = []
    axes = [(0, 1, 2), (1, 0, 2), (2, 0, 1)]
    for fixed, u, v in axes:
        for val in (mn[fixed], mx[fixed]):
            nu = max(2, int((mx[u] - mn[u]) / step) + 1)
            nv = max(2, int((mx[v] - mn[v]) / step) + 1)
            for i in range(nu + 1):
                for j in range(nv + 1):
                    p = [0.0, 0.0, 0.0]
                    p[fixed] = val
                    p[u] = mn[u] + (mx[u] - mn[u]) * i / nu
                    p[v] = mn[v] + (mx[v] - mn[v]) * j / nv
                    pts.append(tuple(p))
    return pts


def dist_to_boxes(p, boxes):
    best = 1e9
    for mn, mx in boxes:
        dx = max(mn[0] - p[0], 0, p[0] - mx[0])
        dy = max(mn[1] - p[1], 0, p[1] - mx[1])
        dz = max(mn[2] - p[2], 0, p[2] - mx[2])
        best = min(best, math.sqrt(dx * dx + dy * dy + dz * dz))
    return best


def run(names):
    bad = 0
    for name in names:
        glb, env, boxes = HULLS[name]
        ax, ay, az = env
        for mn, mx in boxes:
            ok = (
                -ax / 2 - 1e-6 <= mn[0] and mx[0] <= ax / 2 + 1e-6
                and 0 - 1e-6 <= mn[1] and mx[1] <= ay + 1e-6
                and -az / 2 - 1e-6 <= mn[2] and mx[2] <= az / 2 + 1e-6
                and mn[0] < mx[0] and mn[1] < mx[1] and mn[2] < mx[2]
            )
            if not ok:
                print(f"{name}: BOX OUT OF ENVELOPE {mn}..{mx} (env ±{ax/2:.3f} / 0..{ay} / ±{az/2:.3f})")
                bad += 1
        tris, _ = triangles(GLB_DIR + glb)
        grid = TriGrid(tris)
        # phantom: exposed compound surface (skip the floor face and buried faces)
        ds = []
        for mn, mx in boxes:
            others = [b for b in boxes if b != (mn, mx)]
            for p in box_surface_samples(mn, mx):
                if p[1] < 0.02:  # the ground plane face is not hookable air
                    continue
                if inside_any(p, others):
                    continue
                ds.append(grid.dist(p))
        ds.sort()
        med = ds[len(ds) // 2]
        p95 = ds[int(len(ds) * 0.95)]
        # envelope baseline
        eb = [((-ax / 2, 0.0, -az / 2), (ax / 2, ay, az / 2))]
        es = sorted(
            grid.dist(p) for p in box_surface_samples(*eb[0]) if p[1] >= 0.02
        )
        # coverage of the drawn surface
        cov_n = cov_hit = 0
        for t in tris:
            for p in sample_tri(*t, 0.35):
                cov_n += 1
                if dist_to_boxes(p, boxes) <= 0.30:
                    cov_hit += 1
        print(
            f"{name:>20}: hull phantom med {med:.2f} p95 {p95:.2f} max {ds[-1]:.2f}  "
            f"(envelope was med {es[len(es)//2]:.2f} p95 {es[int(len(es)*0.95)]:.2f} max {es[-1]:.2f})  "
            f"coverage {100*cov_hit/cov_n:.0f}%  boxes {len(boxes)}"
        )
    if bad:
        print(f"{bad} boxes out of envelope")
        sys.exit(1)


if __name__ == "__main__":
    run(sys.argv[1:] or list(HULLS))
