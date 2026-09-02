import struct, json, math
p="/home/offlinebot/Documents/defeated-by-titan/assets/3d/glb/a-042-koerpertyp-a-hager-mittel.glb"
d=open(p,'rb').read(); off=12; chunks={}
while off<len(d):
    ln,ty=struct.unpack_from('<II',d,off); off+=8; chunks[ty]=d[off:off+ln]; off+=ln
g=json.loads(chunks[0x4E4F534A]); binb=chunks[0x004E4942]
def acc(i):
    a=g['accessors'][i]; bv=g['bufferViews'][a['bufferView']]
    o=bv.get('byteOffset',0)+a.get('byteOffset',0)
    n=a['count']; t=a['type']; ct=a['componentType']
    ncomp={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[t]
    fmt={5126:'f',5123:'H',5125:'I'}[ct]
    sz=struct.calcsize(fmt); stride=bv.get('byteStride', ncomp*sz)
    return [struct.unpack_from('<'+fmt*ncomp, binb, o+k*stride) for k in range(n)]
verts=[]; tris=[]; base=0
for m in g['meshes']:
    for pr in m['primitives']:
        vs=acc(pr['attributes']['POSITION'])
        idx=[i[0] for i in acc(pr['indices'])]
        verts+=[(-v[0],v[1],-v[2]) for v in vs]   # game frame
        tris+=[(idx[k]+base,idx[k+1]+base,idx[k+2]+base) for k in range(0,len(idx),3)]
        base+=len(vs)
def sub(a,b): return (a[0]-b[0],a[1]-b[1],a[2]-b[2])
def add(a,b): return (a[0]+b[0],a[1]+b[1],a[2]+b[2])
def mul(a,s): return (a[0]*s,a[1]*s,a[2]*s)
def dot(a,b): return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
def norm(a): return math.sqrt(dot(a,a))
def seg_dist(p,a,b):
    ab=sub(b,a); t=dot(sub(p,a),ab)/max(dot(ab,ab),1e-12)
    t=max(0.0,min(1.0,t))
    return norm(sub(p,add(a,mul(ab,t)))), t
# surface samples: barycentric samples on each triangle so long thin boxes are covered
samples=[]
for (i,j,k) in tris:
    A,B,C=verts[i],verts[j],verts[k]
    for (u,v) in [(1,0),(0,1),(0,0),(1/3,1/3),(0.5,0.5),(0.5,0),(0,0.5),(0.25,0.25),(0.25,0.5),(0.5,0.25)]:
        w=1-u-v
        samples.append((A[0]*u+B[0]*v+C[0]*w, A[1]*u+B[1]*v+C[1]*w, A[2]*u+B[2]*v+C[2]*w))
print("samples",len(samples))
skel={
 "torso":  ((0.0,5.1,0.15),(0.0,8.25,-0.35),0.65),
 "head":   ((-0.08,9.15,-0.20),(-0.10,9.55,-0.75),0.50),
 "arm_l":  ((-0.90,8.05,-0.35),(-1.02,3.90,-1.45),0.28),
 "arm_r":  ((0.90,7.90,-0.40),(1.10,3.80,0.80),0.28),
 "hand_l": ((-1.02,3.60,-1.55),(-1.02,2.90,-1.70),0.28),
 "hand_r": ((1.10,3.50,0.80),(1.10,2.85,0.85),0.28),
 "leg_l":  ((-0.42,5.40,0.00),(-0.45,0.70,-0.85),0.30),
 "leg_r":  ((0.45,5.45,0.20),(0.50,0.70,0.70),0.30),
 "foot_l": ((-0.45,0.28,-0.85),(-0.45,0.20,-1.55),0.24),
 "foot_r": ((0.65,0.30,0.35),(0.65,0.25,1.10),0.24),
}
names=list(skel)
# assignment + report residuals per part
import collections
def assign():
    groups=collections.defaultdict(list)
    for s in samples:
        best=None;bn=None
        for n in names:
            a,b,r=skel[n]
            dist,_=seg_dist(s,a,b)
            dd=dist-r
            if best is None or dd<best: best=dd;bn=n
        groups[bn].append(s)
    return groups
groups=assign()
for n in names:
    G=groups[n]
    a,b,r=skel[n]
    ds=[seg_dist(s,a,b)[0] for s in G]
    ds.sort()
    print(f"{n:7s} n{len(G):6d} r_fit p95={ds[int(len(ds)*0.95)]:.3f} max={ds[-1]:.3f}")

# ---- iterate: PCA refit each group's segment, then reassign ----
def pca_refit(G, old):
    a,b,r = old
    cx=sum(s[0] for s in G)/len(G); cy=sum(s[1] for s in G)/len(G); cz=sum(s[2] for s in G)/len(G)
    c=(cx,cy,cz)
    # power iteration on covariance
    v=sub(b,a); L=norm(v); v=mul(v,1/max(L,1e-9))
    M=[[0.0]*3 for _ in range(3)]
    for s in G:
        d=sub(s,c)
        for i in range(3):
            for j in range(3):
                M[i][j]+=d[i]*d[j]
    for _ in range(50):
        nv=(M[0][0]*v[0]+M[0][1]*v[1]+M[0][2]*v[2],
            M[1][0]*v[0]+M[1][1]*v[1]+M[1][2]*v[2],
            M[2][0]*v[0]+M[2][1]*v[1]+M[2][2]*v[2])
        n=norm(nv)
        if n<1e-12: break
        v=mul(nv,1/n)
    ts=sorted(dot(sub(s,c),v) for s in G)
    t0=ts[int(len(ts)*0.02)]; t1=ts[int(len(ts)*0.98)-1]
    na=add(c,mul(v,t0)); nb=add(c,mul(v,t1))
    ds=sorted(seg_dist(s,na,nb)[0] for s in G)
    nr=ds[int(len(ds)*0.95)]
    # shrink segment by radius so the caps don't overreach the ends
    return (na,nb,nr)
for it in range(3):
    groups=assign()
    for n in names:
        if len(groups[n])>30:
            skel[n]=pca_refit(groups[n], skel[n])
groups=assign()
print("---- refit ----")
for n in names:
    G=groups[n]; a,b,r=skel[n]
    ds=sorted(seg_dist(s,a,b)[0] for s in G)
    print(f"{n:7s} n{len(G):6d} a=({a[0]:6.2f},{a[1]:6.2f},{a[2]:6.2f}) b=({b[0]:6.2f},{b[1]:6.2f},{b[2]:6.2f}) r95={ds[int(len(ds)*0.95)]:.3f} max={ds[-1]:.3f}")
# coverage vs the CAPSULE SET: how far outside the nearest capsule does flesh sit
def out_dist(s):
    return min(seg_dist(s,skel[n][0],skel[n][1])[0]-skel[n][2] for n in names)
outs=sorted(out_dist(s) for s in samples)
print("flesh outside nearest capsule: p50 %.3f p95 %.3f p99 %.3f max %.3f"%(outs[len(outs)//2],outs[int(len(outs)*0.95)],outs[int(len(outs)*0.99)],outs[-1]))
# where is the max?
worst=max(samples,key=out_dist)
print("worst uncovered at",["%.2f"%c for c in worst])
