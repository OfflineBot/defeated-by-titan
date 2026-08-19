#!/usr/bin/env python3
"""glb_merge.py — concatenates the primitives of a `.glb` that share a material.

    python3 tools/glb_merge.py [--check] [--quiet] [FILE ...]

Default FILE set: every `assets/3d/glb/*.glb`.

WHY THIS EXISTS (docs/FINDINGS.md FIND-105, FIND-107)
-----------------------------------------------------
`a-083-fachwerkhaus-gross.glb` is **115 separate meshes that all share ONE material**.
Bevy spawns a glTF scene as an entity hierarchy — one entity per node — so the 278 dressed
houses of Ashgate were ~33 000 entities whose transforms propagate every single tick. FIND-105
measured the district's headless tick at **29.6 ms** against a 16.7 ms budget and showed the
cost tracks glTF **node count**, not block count and not instance count.

Primitives that share a material and an attribute set can be concatenated into ONE primitive
with **no visual change whatsoever**. Nothing is decimated, nothing is re-authored: the same
triangles, the same vertices, the same texture, in fewer nodes. That is the whole tool.

WHY THE STANDARD LIBRARY (no pygltflib, no numpy)
--------------------------------------------------
Same reason as `tools/features.py`: machine A has neither pip nor passwordless sudo, and a GLB
is a 12-byte header plus `(u32 length, u32 type, payload)` chunks around a JSON document. That
is enough. `struct` and `array` do the rest.

WHY IT IS SAFE TO RUN
---------------------
The assets are the user's and they are git-tracked. So:

* **Nothing is deleted.** A file is either rewritten in place or left exactly as it was.
* **It is idempotent.** A file whose material groups are already single-primitive is not
  written at all — so a second run is a no-op and `git status` stays clean.
* **`--check` writes nothing** and reports what a real run would do, like
  `tools/features.py --check`.
* **Every invariant below is asserted per file, against the OUTPUT BYTES, before the write.**
  If one fails the file is skipped and the failure is reported — the tool never ships a file
  it could not prove identical.

THE INVARIANTS (docs/FINDINGS.md FIND-103: the check must not ask the merge code)
---------------------------------------------------------------------------------
`snapshot()` re-derives all of these from raw accessor bytes. It never calls the merge. It is
run once on the bytes that came off disk and once on the bytes that are about to go onto it,
and the two records have to agree:

1. **The named empties survive at the same world transform.** All 278 files carry
   `hit.min`/`hit.max`, 45 carry `cortex`, and the architecture carries 439 `hook.*` points
   across 144 files. The loader reads these (`src/shared/anchors.rs`) and the kill zone and
   every rope anchor depend on them. A merge that flattens one is a silent gameplay change.
2. **The world-space geometry is identical** — every triangle, as a sorted multiset of
   (position, normal, uv) corner triples in world space, plus the total bounding box. A node's
   translation is baked into its vertices when it merges upward; if that were skipped the house
   would come apart and this check is what would see it.
3. **Vertex count and triangle count are unchanged.** Re-grouping, not decimating.
4. **The materials and the image URIs are unchanged** — `../../texturen/TEX-*.png`, relative,
   folder name intact.
5. **The file stays a valid GLB**: magic `glTF`, version 2, chunk types JSON/BIN, and a
   declared total length equal to the on-disk size.

WHAT IT REFUSES TO TOUCH
------------------------
The pack is uniform today (all 278 files verified 2026-08-19): one scene, one untransformed
root, every mesh node a direct child of it carrying a translation only, one primitive per mesh,
mode 4, attributes exactly POSITION/NORMAL/TEXCOORD_0. `preconditions()` asserts every one of
those per file. A future export that breaks any of them is **skipped with a reason**, not
guessed at — a rotation or a scale on a mesh node would need baking into the normals too, and
that is a different tool than this one.
"""

from __future__ import annotations

import array
import json
import math
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GLB_DIR = ROOT / "assets" / "3d" / "glb"

GLB_MAGIC = 0x46546C67  # 'glTF'
CHUNK_JSON = 0x4E4F534A
CHUNK_BIN = 0x004E4942

FLOAT = 5126
USHORT = 5123
UINT = 5125
TRIANGLES = 4
ARRAY_BUFFER = 34962
ELEMENT_ARRAY_BUFFER = 34963

COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
FMT = {FLOAT: "f", USHORT: "H", UINT: "I"}
SIZE = {FLOAT: 4, USHORT: 2, UINT: 4}

# UNSIGNED_SHORT tops out here; past it a merged group needs UNSIGNED_INT indices. The pack's
# biggest single material group today is 24 704 vertices (a-042 titan bodies), so nothing hits
# it — the branch exists so that the day something does, it merges instead of corrupting.
USHORT_MAX = 65535

# How close two float32 values have to be to count as the same metre. Baking a translation is
# `pos + t` rounded to float32; the reference side does the same sum in float64. At the pack's
# largest coordinate (~120 m) a float32 ulp is ~1e-5 m, so 1e-3 m is two orders of margin — and
# still a thousand times tighter than any real geometric change, which is centimetres at least.
TOL_M = 1e-3


# ---------------------------------------------------------------------------
# The container
# ---------------------------------------------------------------------------


def read_glb(data: bytes) -> tuple[dict, bytes]:
    """`(glTF JSON, BIN chunk)` out of GLB bytes, with the header checked (glTF 2.0 §4.4)."""
    if len(data) < 12:
        raise ValueError("shorter than a GLB header")
    magic, version, total = struct.unpack_from("<III", data, 0)
    if magic != GLB_MAGIC:
        raise ValueError("no glTF magic")
    if version != 2:
        raise ValueError(f"glTF version {version}, not 2")
    if total != len(data):
        raise ValueError(f"header says {total} bytes, the file is {len(data)}")
    doc, binary, at = None, b"", 12
    while at + 8 <= len(data):
        length, kind = struct.unpack_from("<II", data, at)
        at += 8
        payload = data[at : at + length]
        if len(payload) != length:
            raise ValueError("a chunk runs past the end of the file")
        if kind == CHUNK_JSON:
            doc = json.loads(payload.decode("utf-8"))
        elif kind == CHUNK_BIN:
            binary = payload
        else:
            raise ValueError(f"unknown chunk type {kind:#x}")
        at += length
    if doc is None:
        raise ValueError("no JSON chunk")
    return doc, binary


def write_glb(doc: dict, binary: bytes) -> bytes:
    """GLB bytes. JSON padded with spaces, BIN with zeros, both to 4 (§4.4.3)."""
    text = json.dumps(doc, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    text += b" " * (-len(text) % 4)
    binary += b"\0" * (-len(binary) % 4)
    total = 12 + 8 + len(text) + (8 + len(binary) if binary else 0)
    out = struct.pack("<III", GLB_MAGIC, 2, total)
    out += struct.pack("<II", len(text), CHUNK_JSON) + text
    if binary:
        out += struct.pack("<II", len(binary), CHUNK_BIN) + binary
    return out


def read_accessor(doc: dict, binary: bytes, index: int) -> list:
    """One accessor as a flat list of Python numbers, straight out of the BIN chunk.

    Deliberately dumb: no strides, no sparse, no normalisation — `preconditions` has already
    refused any file that uses them, so this cannot silently mis-read one.
    """
    acc = doc["accessors"][index]
    view = doc["bufferViews"][acc["bufferView"]]
    comp = COMPONENTS[acc["type"]]
    fmt = FMT[acc["componentType"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    count = acc["count"] * comp
    values = array.array(fmt)
    values.frombytes(binary[start : start + count * SIZE[acc["componentType"]]])
    if len(values) != count:
        raise ValueError(f"accessor {index} runs past the buffer")
    if sys.byteorder != "little":
        values.byteswap()
    return list(values)


# ---------------------------------------------------------------------------
# The independent check (FIND-103) — never calls the merge
# ---------------------------------------------------------------------------


def snapshot(data: bytes) -> dict:
    """Everything that must not change, re-derived from raw GLB bytes.

    Run on the input file and on the output bytes. It walks the node tree itself, applies each
    node's own translation itself, and reads the vertex floats itself. It shares no code with
    `merge_document` — which is the point (FIND-103: a check that asks the same function the
    same question passes when both are wrong).
    """
    doc, binary = read_glb(data)
    nodes = doc.get("nodes", [])

    parent = {}
    for i, node in enumerate(nodes):
        for child in node.get("children", []):
            parent[child] = i

    def world(index: int) -> tuple[float, float, float]:
        x = y = z = 0.0
        at = index
        while at is not None:
            t = nodes[at].get("translation", [0.0, 0.0, 0.0])
            x, y, z = x + t[0], y + t[1], z + t[2]
            at = parent.get(at)
        return (x, y, z)

    empties = sorted(
        (node["name"], world(i))
        for i, node in enumerate(nodes)
        if "mesh" not in node and node.get("name")
    )

    triangles = []
    verts = 0
    lo = [math.inf] * 3
    hi = [-math.inf] * 3
    for i, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        offset = world(i)
        for prim in doc["meshes"][node["mesh"]]["primitives"]:
            pos = read_accessor(doc, binary, prim["attributes"]["POSITION"])
            nrm = read_accessor(doc, binary, prim["attributes"]["NORMAL"])
            uv = read_accessor(doc, binary, prim["attributes"]["TEXCOORD_0"])
            idx = read_accessor(doc, binary, prim["indices"])
            verts += len(pos) // 3
            # Rounded to float32 here as well, because that is what storing a baked vertex
            # does. Without it the two sides differ by up to half a float32 ulp on every
            # coordinate, ties in the sort break differently, and the comparison turns into a
            # tolerance argument. With it the two triangle lists are bit-identical or the merge
            # is wrong — which is the check worth having.
            world_pos = array.array(
                "f", [pos[v + a] + offset[a] for v in range(0, len(pos), 3) for a in range(3)]
            )
            for a in range(3):
                column = world_pos[a::3]
                lo[a] = min(lo[a], min(column, default=math.inf))
                hi[a] = max(hi[a], max(column, default=-math.inf))
            corners = [
                (
                    world_pos[3 * v],
                    world_pos[3 * v + 1],
                    world_pos[3 * v + 2],
                    nrm[3 * v],
                    nrm[3 * v + 1],
                    nrm[3 * v + 2],
                    uv[2 * v],
                    uv[2 * v + 1],
                )
                for v in idx
            ]
            for t in range(0, len(corners), 3):
                triangles.append(corners[t] + corners[t + 1] + corners[t + 2])

    return {
        "empties": empties,
        "vertices": verts,
        "triangles": sorted(triangles),
        "bbox": (tuple(lo), tuple(hi)),
        "materials": json.dumps(doc.get("materials", []), sort_keys=True),
        "images": json.dumps(doc.get("images", []), sort_keys=True),
        "bytes": len(data),
    }


def _close(a, b) -> bool:
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return (a == b) or (math.isfinite(a) and math.isfinite(b) and abs(a - b) <= TOL_M)
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        return len(a) == len(b) and all(_close(x, y) for x, y in zip(a, b))
    return a == b


def compare(before: dict, after: dict) -> list[str]:
    """The list of broken invariants, empty when the merge was lossless."""
    bad = []
    if before["empties"] != after["empties"]:
        was = dict(before["empties"])
        now = dict(after["empties"])
        lost = sorted(set(was) - set(now))
        gained = sorted(set(now) - set(was))
        moved = sorted(k for k in set(was) & set(now) if not _close(was[k], now[k]))
        bad.append(f"named empties changed: lost={lost} gained={gained} moved={moved}")
    if before["vertices"] != after["vertices"]:
        bad.append(f"vertex count {before['vertices']} -> {after['vertices']}")
    if len(before["triangles"]) != len(after["triangles"]):
        bad.append(f"triangle count {len(before['triangles'])} -> {len(after['triangles'])}")
    elif before["triangles"] != after["triangles"]:
        # Both sides are float32-rounded, so this is an exact comparison on purpose. The
        # numbers in the message say whether it is a real move or a rounding argument.
        pairs = list(zip(before["triangles"], after["triangles"]))
        n = sum(1 for a, b in pairs if a != b)
        worst = max(
            (abs(x - y) for a, b in pairs for x, y in zip(a, b)),
            default=0.0,
        )
        bad.append(
            f"{n} of {len(pairs)} triangles differ in world space, worst coordinate {worst:.6f}"
        )
    if not _close(before["bbox"], after["bbox"]):
        bad.append(f"bounding box {before['bbox']} -> {after['bbox']}")
    if before["materials"] != after["materials"]:
        bad.append("the material list changed")
    if before["images"] != after["images"]:
        bad.append(f"the image URIs changed: {before['images']} -> {after['images']}")
    return bad


# ---------------------------------------------------------------------------
# The merge
# ---------------------------------------------------------------------------


def preconditions(doc: dict) -> str | None:
    """`None` when this document is one the merge understands; else why it is not.

    Every clause is a thing the merge would get silently wrong, not a matter of taste. A
    rotation or a scale on a mesh node is the loudest of them: baking it would have to
    transform the normals by the inverse transpose, which this tool does not do.
    """
    if len(doc.get("scenes", [])) != 1 or len(doc["scenes"][0].get("nodes", [])) != 1:
        return "not exactly one scene with exactly one root node"
    root = doc["scenes"][0]["nodes"][0]
    if any(k in doc["nodes"][root] for k in ("translation", "rotation", "scale", "matrix")):
        return "the root node carries a transform"
    if "skins" in doc or "animations" in doc:
        return "carries skins or animations"
    children = set(doc["nodes"][root].get("children", []))
    for i, node in enumerate(doc["nodes"]):
        if i == root:
            continue
        if i not in children:
            return f"node {i} ({node.get('name')!r}) is not a direct child of the root"
        if "mesh" not in node:
            continue
        if node.get("children"):
            return f"mesh node {node.get('name')!r} has children"
        for k in ("rotation", "scale", "matrix"):
            if k in node:
                return f"mesh node {node.get('name')!r} carries a {k}"
    for mesh in doc.get("meshes", []):
        for prim in mesh["primitives"]:
            if prim.get("mode", TRIANGLES) != TRIANGLES:
                return f"mesh {mesh.get('name')!r} is not TRIANGLES"
            if set(prim["attributes"]) != {"POSITION", "NORMAL", "TEXCOORD_0"}:
                return f"mesh {mesh.get('name')!r} has attributes {sorted(prim['attributes'])}"
            if "indices" not in prim:
                return f"mesh {mesh.get('name')!r} is not indexed"
    for acc in doc.get("accessors", []):
        if "sparse" in acc or acc.get("normalized"):
            return "a sparse or normalized accessor"
        if "bufferView" not in acc:
            return "an accessor without a bufferView"
    for view in doc.get("bufferViews", []):
        if view.get("byteStride"):
            return "an interleaved bufferView"
    if len(doc.get("buffers", [])) > 1:
        return "more than one buffer"
    return None


class _Bin:
    """The output BIN chunk, one accessor per bufferView, every view 4-aligned."""

    def __init__(self) -> None:
        self.blob = bytearray()
        self.views: list[dict] = []
        self.accessors: list[dict] = []

    def add(self, values, component: int, kind: str, target: int, minmax=None) -> int:
        self.blob += b"\0" * (-len(self.blob) % 4)
        offset = len(self.blob)
        buf = array.array(FMT[component], values)
        if sys.byteorder != "little":
            buf.byteswap()
        self.blob += buf.tobytes()
        self.views.append(
            {
                "buffer": 0,
                "byteOffset": offset,
                "byteLength": len(self.blob) - offset,
                "target": target,
            }
        )
        acc = {
            "bufferView": len(self.views) - 1,
            "byteOffset": 0,
            "componentType": component,
            "count": len(values) // COMPONENTS[kind],
            "type": kind,
        }
        if minmax:
            acc["min"], acc["max"] = minmax
        self.accessors.append(acc)
        return len(self.accessors) - 1


def merge_document(doc: dict, binary: bytes) -> tuple[dict, bytes, dict]:
    """The merged document, its BIN chunk, and what was done.

    One node per (material, attribute set) group, in first-appearance order; every empty kept
    exactly as it was. The group's primitives are concatenated with each node's translation
    added into its POSITION values — normals and UVs are carried through untouched, which is
    correct precisely because `preconditions` has refused anything but a pure translation.
    """
    root_index = doc["scenes"][0]["nodes"][0]
    root = doc["nodes"][root_index]

    groups: dict[int | None, list[int]] = {}
    empties: list[int] = []
    for i in root.get("children", []):
        node = doc["nodes"][i]
        if "mesh" not in node:
            empties.append(i)
            continue
        prim = doc["meshes"][node["mesh"]]["primitives"][0]
        groups.setdefault(prim.get("material"), []).append(i)

    stats = {
        "primitives_before": sum(len(v) for v in groups.values()),
        "primitives_after": len(groups),
        "nodes_before": len(doc["nodes"]),
        "nodes_after": 1 + len(groups) + len(empties),
        "empties": len(empties),
    }
    if stats["primitives_before"] == stats["primitives_after"]:
        return doc, binary, stats  # already merged — do not rewrite

    out = _Bin()
    nodes: list[dict] = []
    meshes: list[dict] = []

    for material, members in groups.items():
        pos: list[float] = []
        nrm: list[float] = []
        uv: list[float] = []
        idx: list[int] = []
        lo = [math.inf] * 3
        hi = [-math.inf] * 3
        for i in members:
            node = doc["nodes"][i]
            t = node.get("translation", [0.0, 0.0, 0.0])
            prim = doc["meshes"][node["mesh"]]["primitives"][0]
            p = read_accessor(doc, binary, prim["attributes"]["POSITION"])
            base = len(pos) // 3
            for v in range(0, len(p), 3):
                for a in range(3):
                    value = p[v + a] + t[a]
                    pos.append(value)
                    lo[a] = min(lo[a], value)
                    hi[a] = max(hi[a], value)
            nrm += read_accessor(doc, binary, prim["attributes"]["NORMAL"])
            uv += read_accessor(doc, binary, prim["attributes"]["TEXCOORD_0"])
            idx += [base + v for v in read_accessor(doc, binary, prim["indices"])]

        vertices = len(pos) // 3
        component = USHORT if vertices - 1 <= USHORT_MAX else UINT
        # The float32 round-trip has to happen BEFORE min/max is written, or the accessor's
        # declared bounds can fall a ulp inside the vertices it bounds and a validator says so.
        packed = array.array("f", pos)
        pos = list(packed)
        lo = [min(pos[v + a] for v in range(0, len(pos), 3)) for a in range(3)]
        hi = [max(pos[v + a] for v in range(0, len(pos), 3)) for a in range(3)]

        name = "merged" if len(groups) == 1 else f"merged.{len(meshes)}"
        prim_out = {
            "attributes": {
                "POSITION": out.add(pos, FLOAT, "VEC3", ARRAY_BUFFER, (lo, hi)),
                "NORMAL": out.add(nrm, FLOAT, "VEC3", ARRAY_BUFFER),
                "TEXCOORD_0": out.add(uv, FLOAT, "VEC2", ARRAY_BUFFER),
            },
            "indices": out.add(idx, component, "SCALAR", ELEMENT_ARRAY_BUFFER),
            "mode": TRIANGLES,
        }
        if material is not None:
            prim_out["material"] = material
        meshes.append({"name": name, "primitives": [prim_out]})
        nodes.append({"name": name, "mesh": len(meshes) - 1})

    for i in empties:
        nodes.append(dict(doc["nodes"][i]))

    new_root = dict(root)
    new_root["children"] = list(range(len(nodes)))
    nodes.append(new_root)

    merged = dict(doc)
    merged["nodes"] = nodes
    merged["meshes"] = meshes
    merged["accessors"] = out.accessors
    merged["bufferViews"] = out.views
    merged["buffers"] = [{"byteLength": len(out.blob) + (-len(out.blob) % 4)}]
    merged["scenes"] = [dict(doc["scenes"][0], nodes=[len(nodes) - 1])]
    merged["scene"] = 0
    return merged, bytes(out.blob), stats


# ---------------------------------------------------------------------------


def process(path: Path, check: bool) -> tuple[str, dict | None, str]:
    """`(verdict, stats, detail)` for one file. Writes only when `check` is false."""
    data = path.read_bytes()
    try:
        doc, binary = read_glb(data)
    except ValueError as e:
        return "broken", None, str(e)
    why = preconditions(doc)
    if why:
        return "skipped", None, why
    merged, blob, stats = merge_document(doc, binary)
    if stats["primitives_before"] == stats["primitives_after"]:
        return "already", stats, "every material group is already one primitive"
    out = write_glb(merged, blob)
    bad = compare(snapshot(data), snapshot(out))
    if bad:
        return "REFUSED", stats, "; ".join(bad)
    if not check:
        path.write_bytes(out)
    return "merged", stats, f"{len(data)} -> {len(out)} bytes"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    quiet = "--quiet" in argv
    files = [Path(a) for a in argv if not a.startswith("--")]
    if not files:
        files = sorted(GLB_DIR.glob("*.glb"))
    if not files:
        print(f"no .glb under {GLB_DIR}", file=sys.stderr)
        return 1

    tally: dict[str, int] = {}
    worst = 0
    before = after = nodes_before = nodes_after = 0
    failed = []
    for path in files:
        verdict, stats, detail = process(path, check)
        tally[verdict] = tally.get(verdict, 0) + 1
        if stats:
            before += stats["primitives_before"]
            after += stats["primitives_after"]
            nodes_before += stats["nodes_before"]
            nodes_after += stats["nodes_after"]
            worst = max(worst, stats["primitives_after"])
        if verdict in ("merged", "REFUSED", "skipped", "broken") and not quiet:
            head = f"{verdict:8s} {path.name:44s}"
            if stats:
                head += f" prims {stats['primitives_before']:4d} -> {stats['primitives_after']:2d}"
                head += f"  nodes {stats['nodes_before']:4d} -> {stats['nodes_after']:2d}"
            print(f"{head}  {detail if verdict != 'merged' else ''}".rstrip())
        if verdict in ("REFUSED", "broken"):
            failed.append((path.name, detail))

    print(
        f"\n{'would merge' if check else 'merged'}: "
        + ", ".join(f"{v} {k}" for k, v in sorted(tally.items()))
    )
    if before:
        print(
            f"primitives {before} -> {after}  ({after / before:.1%});  "
            f"nodes {nodes_before} -> {nodes_after}  ({nodes_after / nodes_before:.1%});  "
            f"most primitives left in one file: {worst}"
        )
    for name, detail in failed:
        print(f"  !! {name}: {detail}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
