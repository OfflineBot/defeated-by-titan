#!/usr/bin/env python3
"""B-042 oracle: drawn glb surface vs the rig's colliders, per titan kind.

Parses the glb (JSON+BIN chunks, node hierarchy, POSITION accessors),
flattens all mesh vertices into model-root space, then reports the drawn
silhouette half-width at the heights the rig cares about, both in authored
units and scaled by the game's own fit (cortex fit, render/model.rs).

Worst case of the instrument: a height slice with NO vertices prints 'none'
explicitly with the vertex count, never 0.0 (docs/lessons/fixtures.md rule 6).
"""
import json, struct, sys, math

def parse_glb(path):
    with open(path, 'rb') as f:
        data = f.read()
    assert data[:4] == b'glTF', 'not a glb'
    length = struct.unpack('<I', data[8:12])[0]
    off = 12
    jsonchunk = binchunk = None
    while off < length:
        clen, ctype = struct.unpack('<I4s', data[off:off+8])
        chunk = data[off+8:off+8+clen]
        if ctype == b'JSON':
            jsonchunk = json.loads(chunk)
        elif ctype == b'BIN\x00':
            binchunk = chunk
        off += 8 + clen
    return jsonchunk, binchunk

def read_accessor(g, binchunk, idx):
    acc = g['accessors'][idx]
    bv = g['bufferViews'][acc['bufferView']]
    comp = acc['componentType']; count = acc['count']; typ = acc['type']
    ncomp = {'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[typ]
    fmt = {5126:'f',5123:'H',5125:'I',5121:'B',5122:'h',5120:'b'}[comp]
    size = struct.calcsize(fmt)
    start = bv.get('byteOffset',0) + acc.get('byteOffset',0)
    stride = bv.get('byteStride', ncomp*size)
    out = []
    for i in range(count):
        o = start + i*stride
        out.append(struct.unpack_from('<'+fmt*ncomp, binchunk, o))
    return out

def node_matrix(n):
    if 'matrix' in n:
        m = n['matrix']  # column major
        return [[m[0],m[4],m[8],m[12]],[m[1],m[5],m[9],m[13]],[m[2],m[6],m[10],m[14]],[m[3],m[7],m[11],m[15]]]
    t = n.get('translation',[0,0,0]); r = n.get('rotation',[0,0,0,1]); s = n.get('scale',[1,1,1])
    x,y,z,w = r
    R = [[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],
         [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],
         [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]]
    M = [[R[i][j]*s[j] for j in range(3)]+[t[i]] for i in range(3)]
    M.append([0,0,0,1])
    return M

def matmul(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]

def xform(m,v):
    return tuple(m[i][0]*v[0]+m[i][1]*v[1]+m[i][2]*v[2]+m[i][3] for i in range(3))

def collect(path):
    g, binchunk = parse_glb(path)
    verts = []
    anchors = {}
    scene = g['scenes'][g.get('scene',0)]
    ident = [[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
    def walk(idx, parent_m):
        n = g['nodes'][idx]
        m = matmul(parent_m, node_matrix(n))
        name = n.get('name','')
        if 'mesh' not in n and name:
            anchors[name] = (m[0][3], m[1][3], m[2][3])
        if 'mesh' in n:
            mesh = g['meshes'][n['mesh']]
            for prim in mesh['primitives']:
                if 'POSITION' in prim['attributes']:
                    for v in read_accessor(g, binchunk, prim['attributes']['POSITION']):
                        verts.append(xform(m, v))
        for c in n.get('children',[]):
            walk(c, m)
    for root in scene['nodes']:
        walk(root, ident)
    return verts, anchors

def slice_halfwidth(verts, y_lo, y_hi):
    """max horizontal distance from the y-axis of any vertex in [y_lo,y_hi]."""
    sel = [v for v in verts if y_lo <= v[1] <= y_hi]
    if not sel:
        return None, 0
    return max(math.hypot(v[0], v[2]) for v in sel), len(sel)

if __name__ == '__main__':
    path = sys.argv[1]
    verts, anchors = collect(path)
    ys = [v[1] for v in verts]
    print(f"file: {path}")
    print(f"verts: {len(verts)}  y range authored: {min(ys):.3f} .. {max(ys):.3f}")
    for k in sorted(anchors):
        if any(k.startswith(p) for p in ('cortex','hit.','eye')):
            a = anchors[k]
            print(f"anchor {k}: ({a[0]:.3f}, {a[1]:.3f}, {a[2]:.3f})")
    # authored cortex y decides the fit (render/model.rs fit_to_class)
    cortex = anchors.get('cortex')
    if cortex is None:
        print('NO cortex anchor — fit would fall back to hit box height')
        sys.exit(0)
    for kind, height_m, cortex_m in [('husk/errant/chorus (medium)', 20.0, 17.8),
                                     ('scuttler/weaver (small)', 8.4, 7.4)]:
        fit = cortex_m / cortex[1]
        print(f"\n== {kind}: fit = {cortex_m}/{cortex[1]:.4f} = {fit:.4f}, drawn height = {(max(ys)-min(ys))*fit:.2f} m (class {height_m}) ==")
        # rig numbers (src/titan/rig.rs, scale.ron)
        w = 0.20*height_m; torso_r = w/2
        leg = 0.48*height_m; torso = 0.41*height_m
        head = height_m - leg - torso; neck_r = head/2
        shoulder = 0.82*height_m
        rows = [
            ('Cortex height', cortex_m, neck_r),
            ('neck mid', (shoulder + height_m)/2, neck_r),
            ('shoulder', shoulder, torso_r),
            ('torso mid', (torso_r + shoulder - torso_r)/2 + 0.0, torso_r),
            ('hip', leg, torso_r),
            ('knee', leg*0.5, torso_r),
        ]
        print(f"{'slice':>18} {'y_m':>7} {'drawn r':>8} {'collider r':>10} {'collider - drawn':>16}  verts")
        for label, y_m, coll_r in rows:
            y_auth = y_m / fit
            band = 0.25 / fit  # +-0.25 m slice in game metres
            r, n = slice_halfwidth(verts, y_auth-band, y_auth+band)
            if r is None:
                print(f"{label:>18} {y_m:7.2f} {'none':>8} {coll_r:10.2f} {'(no verts in slice)':>16}  0")
            else:
                drawn = r*fit
                print(f"{label:>18} {y_m:7.2f} {drawn:8.2f} {coll_r:10.2f} {coll_r-drawn:16.2f}  {n}")
        # cortex sphere vs drawn nape skin, z axis (MODEL_FACES flips z: game z = -authored z)
        # rig clamp: kill zone z = max(model_z*fit, head/2)
        model_z_game = -cortex[2]*fit
        clamped = max(model_z_game, neck_r)
        print(f"cortex anchor z (game frame, after fit): {model_z_game:.3f} m; rig clamps kill-zone z to {clamped:.3f} m (= neck_r)")
