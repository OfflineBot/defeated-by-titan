#!/usr/bin/env python3
"""Measure glb meshes for B-043 hull authoring.

Parses the glb JSON+BIN chunks directly (no deps), extracts every mesh primitive's
triangles through the node transform chain, then:
  * reports the mesh AABB and the hit.min/hit.max pair (the envelope the plan uses),
  * voxelizes the surface (dense point sampling on triangles), flood-fills the outside,
    marks everything unreachable as solid,
  * greedily covers the solid voxels with axis-aligned boxes (fully-occupied only, so
    no phantom volume is ever re-introduced),
  * prints per-y-band x/z AABBs as a cross-check.
"""
import struct, json, math, sys
from collections import deque

GLB_DIR = "/home/offlinebot/Documents/defeated-by-titan/assets/3d/glb/"

COMP = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
NCOMP = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def load_glb(path):
    b = open(path, "rb").read()
    assert b[:4] == b"glTF"
    total = struct.unpack("<I", b[8:12])[0]
    at = 12
    doc = None
    bin_chunk = b""
    while at < total:
        ln, ty = struct.unpack("<I4s", b[at : at + 8])
        data = b[at + 8 : at + 8 + ln]
        if ty == b"JSON":
            doc = json.loads(data)
        elif ty == b"BIN\x00":
            bin_chunk = data
        at += 8 + ln
    return doc, bin_chunk


def accessor(doc, bin_chunk, idx):
    acc = doc["accessors"][idx]
    bv = doc["bufferViews"][acc["bufferView"]]
    fmt, sz = COMP[acc["componentType"]]
    n = NCOMP[acc["type"]]
    count = acc["count"]
    stride = bv.get("byteStride", sz * n)
    base = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
    out = []
    for i in range(count):
        o = base + i * stride
        out.append(struct.unpack_from("<" + fmt * n, bin_chunk, o))
    return out


def node_matrix(node):
    if "matrix" in node:
        m = node["matrix"]  # column-major
        return [[m[0], m[4], m[8], m[12]], [m[1], m[5], m[9], m[13]], [m[2], m[6], m[10], m[14]], [0, 0, 0, 1]]
    t = node.get("translation", [0, 0, 0])
    q = node.get("rotation", [0, 0, 0, 1])
    s = node.get("scale", [1, 1, 1])
    x, y, z, w = q
    r = [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]
    return [
        [r[0][0] * s[0], r[0][1] * s[1], r[0][2] * s[2], t[0]],
        [r[1][0] * s[0], r[1][1] * s[1], r[1][2] * s[2], t[1]],
        [r[2][0] * s[0], r[2][1] * s[1], r[2][2] * s[2], t[2]],
        [0, 0, 0, 1],
    ]


def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def xform(m, p):
    return (
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    )


IDENT = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]


def triangles(path):
    doc, bin_chunk = load_glb(path)
    tris = []
    hit = {}

    def walk(idx, m):
        node = doc["nodes"][idx]
        m2 = matmul(m, node_matrix(node))
        name = node.get("name", "")
        if name in ("hit.min", "hit.max"):
            hit[name] = xform(m2, (0, 0, 0))
        if "mesh" in node:
            mesh = doc["meshes"][node["mesh"]]
            for prim in mesh["primitives"]:
                pos = accessor(doc, bin_chunk, prim["attributes"]["POSITION"])
                pos = [xform(m2, p) for p in pos]
                if "indices" in prim:
                    ind = [i[0] for i in accessor(doc, bin_chunk, prim["indices"])]
                else:
                    ind = list(range(len(pos)))
                for i in range(0, len(ind), 3):
                    tris.append((pos[ind[i]], pos[ind[i + 1]], pos[ind[i + 2]]))
        for c in node.get("children", []):
            walk(c, m2)

    for scene_node in doc["scenes"][doc.get("scene", 0)]["nodes"]:
        walk(scene_node, IDENT)
    return tris, hit


def sample_tri(a, b, c, step):
    """Points on the triangle at <= step spacing (barycentric grid)."""
    eab = math.dist(a, b)
    eac = math.dist(a, c)
    ebc = math.dist(b, c)
    n = max(2, int(math.ceil(max(eab, eac, ebc) / step)) + 1)
    pts = []
    for i in range(n + 1):
        for j in range(n + 1 - i):
            u = i / n
            v = j / n
            w = 1 - u - v
            pts.append((a[0] * w + b[0] * u + c[0] * v, a[1] * w + b[1] * u + c[1] * v, a[2] * w + b[2] * u + c[2] * v))
    return pts


class Vox:
    def __init__(self, tris, cell):
        self.cell = cell
        xs = [p[0] for t in tris for p in t]
        ys = [p[1] for t in tris for p in t]
        zs = [p[2] for t in tris for p in t]
        self.mn = (min(xs), min(ys), min(zs))
        self.mx = (max(xs), max(ys), max(zs))
        self.nx = max(1, int(math.ceil((self.mx[0] - self.mn[0]) / cell)))
        self.ny = max(1, int(math.ceil((self.mx[1] - self.mn[1]) / cell)))
        self.nz = max(1, int(math.ceil((self.mx[2] - self.mn[2]) / cell)))
        self.occ = set()
        step = cell * 0.45
        for a, b, c in tris:
            for p in sample_tri(a, b, c, step):
                i = min(self.nx - 1, max(0, int((p[0] - self.mn[0]) / cell)))
                j = min(self.ny - 1, max(0, int((p[1] - self.mn[1]) / cell)))
                k = min(self.nz - 1, max(0, int((p[2] - self.mn[2]) / cell)))
                self.occ.add((i, j, k))

    def fill_interior(self):
        """Flood the OUTSIDE from a border shell; anything unreached and unoccupied is interior."""
        outside = set()
        dq = deque()
        for i in range(-1, self.nx + 1):
            for j in range(-1, self.ny + 1):
                for k in range(-1, self.nz + 1):
                    if i in (-1, self.nx) or j in (-1, self.ny) or k in (-1, self.nz):
                        dq.append((i, j, k))
                        outside.add((i, j, k))
        while dq:
            i, j, k = dq.popleft()
            for di, dj, dk in ((1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)):
                q = (i + di, j + dj, k + dk)
                if q in outside or q in self.occ:
                    continue
                if not (-1 <= q[0] <= self.nx and -1 <= q[1] <= self.ny and -1 <= q[2] <= self.nz):
                    continue
                outside.add(q)
                dq.append(q)
        for i in range(self.nx):
            for j in range(self.ny):
                for k in range(self.nz):
                    v = (i, j, k)
                    if v not in self.occ and v not in outside:
                        self.occ.add(v)

    def greedy_boxes(self):
        left = set(self.occ)
        boxes = []
        while left:
            seed = min(left)  # deterministic
            i0, j0, k0 = seed
            i1, j1, k1 = i0, j0, k0

            def full(a0, b0, c0, a1, b1, c1):
                for a in range(a0, a1 + 1):
                    for b in range(b0, b1 + 1):
                        for c in range(c0, c1 + 1):
                            if (a, b, c) not in left:
                                return False
                return True

            grown = True
            while grown:
                grown = False
                for axis in range(3):
                    lo = [i0, j0, k0]
                    hi = [i1, j1, k1]
                    hi2 = hi[:]
                    hi2[axis] += 1
                    if hi2[axis] < (self.nx, self.ny, self.nz)[axis] and full(
                        lo[0], lo[1], lo[2], hi2[0], hi2[1], hi2[2]
                    ):
                        i1, j1, k1 = hi2
                        grown = True
                    lo2 = lo[:]
                    lo2[axis] -= 1
                    if lo2[axis] >= 0 and full(lo2[0], lo2[1], lo2[2], hi[0], hi[1], hi[2]):
                        i0, j0, k0 = lo2
                        grown = True
            for a in range(i0, i1 + 1):
                for b in range(j0, j1 + 1):
                    for c in range(k0, k1 + 1):
                        left.discard((a, b, c))
            boxes.append((i0, j0, k0, i1, j1, k1))
        return boxes

    def box_m(self, b):
        i0, j0, k0, i1, j1, k1 = b
        c = self.cell
        return (
            (self.mn[0] + i0 * c, self.mn[1] + j0 * c, self.mn[2] + k0 * c),
            (self.mn[0] + (i1 + 1) * c, self.mn[1] + (j1 + 1) * c, self.mn[2] + (k1 + 1) * c),
        )


def report(name, path, cell):
    tris, hit = triangles(GLB_DIR + path)
    xs = [p[0] for t in tris for p in t]
    ys = [p[1] for t in tris for p in t]
    zs = [p[2] for t in tris for p in t]
    print(f"== {name} ({path}) — {len(tris)} tris")
    print(
        f"  mesh AABB  x [{min(xs):+.3f}, {max(xs):+.3f}]  y [{min(ys):+.3f}, {max(ys):+.3f}]  z [{min(zs):+.3f}, {max(zs):+.3f}]"
    )
    if hit:
        hmin, hmax = hit.get("hit.min"), hit.get("hit.max")
        print(f"  hit.min {tuple(round(v,3) for v in hmin)}  hit.max {tuple(round(v,3) for v in hmax)}")
        ext = tuple(abs(hmax[i] - hmin[i]) for i in range(3))
        mid = tuple((hmax[i] + hmin[i]) / 2 for i in range(3))
        print(f"  hit extent {tuple(round(v,2) for v in ext)}  hit centre {tuple(round(v,3) for v in mid)}")
        print(
            f"  envelope halfwidths ±({ext[0]/2:.3f}, {ext[2]/2:.3f}) around ORIGIN — mesh centre offset x {(min(xs)+max(xs))/2:+.3f} z {(min(zs)+max(zs))/2:+.3f}"
        )
    v = Vox(tris, cell)
    v.fill_interior()
    boxes = v.greedy_boxes()
    boxes.sort(key=lambda b: -((b[3] - b[0] + 1) * (b[4] - b[1] + 1) * (b[5] - b[2] + 1)))
    vol = sum((b[3] - b[0] + 1) * (b[4] - b[1] + 1) * (b[5] - b[2] + 1) for b in boxes)
    print(f"  voxels {len(v.occ)} (cell {cell}) -> {len(boxes)} greedy boxes")
    acc = 0
    for b in boxes:
        n = (b[3] - b[0] + 1) * (b[4] - b[1] + 1) * (b[5] - b[2] + 1)
        acc += n
        mn, mx = v.box_m(b)
        print(
            f"    box ({mn[0]:+.2f},{mn[1]:+.2f},{mn[2]:+.2f})..({mx[0]:+.2f},{mx[1]:+.2f},{mx[2]:+.2f})  vox {n}  cum {100*acc/vol:.0f}%"
        )
        if acc / vol > 0.97 and n < 8:
            print(f"    ... remaining {len(boxes)-boxes.index(b)-1} boxes under 8 voxels each, skipped")
            break
    # per-y-band AABBs as the cross-check / mound recipe
    bands = 6
    ymin, ymax = min(ys), max(ys)
    h = (ymax - ymin) / bands
    for bi in range(bands):
        lo, hi2 = ymin + bi * h, ymin + (bi + 1) * h
        bx = [
            p[0]
            for t in tris
            for p in sample_tri(*t, 0.3)
            if lo <= p[1] <= hi2
        ]
        bz = [
            p[2]
            for t in tris
            for p in sample_tri(*t, 0.3)
            if lo <= p[1] <= hi2
        ]
        if bx:
            print(
                f"  band y [{lo:.2f},{hi2:.2f}]  x [{min(bx):+.2f},{max(bx):+.2f}]  z [{min(bz):+.2f},{max(bz):+.2f}]"
            )
    print()


if __name__ == "__main__":
    jobs = [
        ("ruin_roof_collapsed", "a-089-ruine-dach-eingestuerzt.glb"),
        ("ruin_roof_half", "a-089-ruine-dach-haelfte.glb"),
        ("ruin_gable", "a-089-ruine-giebel.glb"),
        ("ruin_heap", "a-089-ruine-haufen.glb"),
        ("ruin_upper_floor", "a-089-ruine-obergeschoss.glb"),
        ("ruin_pillar", "a-089-ruine-pfeiler.glb"),
        ("ruin_wall_corner", "a-089-ruine-wand-ecke.glb"),
        ("ruin_wall_high", "a-089-ruine-wand-hoch.glb"),
        ("rubble_beams", "a-090-schutt-balken.glb"),
        ("rubble_cover", "a-090-schutt-deckung.glb"),
        ("rubble_flat", "a-090-schutt-flach.glb"),
        ("rubble_heap_large", "a-090-schutt-haufen-gross.glb"),
        ("rubble_high", "a-090-schutt-hoch.glb"),
        ("rubble_wall_piece", "a-090-schutt-wandstueck.glb"),
        ("market_stall", "a-087-marktstand-zeltdach.glb"),
        ("gas_drum", "a-132-fass-stehend.glb"),
        ("lamp_post", "a-088-laterne-strasse.glb"),
        ("signpost", "a-088-wegweiser.glb"),
        ("banner_long", "a-133-banner-lang.glb"),
        ("hand_cart", "a-131-karren-intakt.glb"),
        ("crate_small", "a-132-kiste-klein.glb"),
        ("sentry", "a-136-npc-vanguard.glb"),
    ]
    want = sys.argv[1:] or [j[0] for j in jobs]
    for name, path in jobs:
        if name in want:
            report(name, path, 0.20)
