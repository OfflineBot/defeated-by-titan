import json, math, struct
exec(open('fit_caps.py').read().split('skel={')[0])  # reuse loader + helpers, stop before skel
# authored capsules, glb units, game frame (x right, y up, -z forward)
CAPS={
 "torso":  ((0.00,5.10, 0.10),( 0.00,8.20,-0.42),0.64),
 "head":   ((0.00,9.30,-0.05),(-0.10,9.55,-0.72),0.50),
 "arm_l":  ((-0.85,8.25,-0.32),(-1.05,4.00,-1.45),0.30),
 "arm_r":  (( 0.85,8.15,-0.45),( 1.12,3.90, 0.72),0.30),
 "hand_l": ((-1.04,3.80,-1.50),(-0.98,2.85,-1.75),0.28),
 "hand_r": (( 1.14,3.70, 0.70),( 1.10,2.80, 0.90),0.27),
 "leg_l":  ((-0.42,5.30,-0.10),(-0.46,0.50,-0.78),0.34),
 "leg_r":  (( 0.48,5.30, 0.12),( 0.62,0.50, 0.72),0.32),
 "foot_l": ((-0.47,0.22,-0.35),(-0.46,0.20,-1.60),0.28),
 "foot_r": (( 0.66,0.22,-0.05),( 0.66,0.28, 1.15),0.29),
}
# the rig's neck segment stays: husk h=20 -> glb units: neck_r = head_m/2 /2 ... compute from scale.ron fractions
# scale.ron: need leg_fraction, torso_fraction, shoulder_height_fraction. Read them:
import re
sc=open('/home/offlinebot/Documents/defeated-by-titan/assets/data/scale.ron').read()
def frac(name):
    m=re.search(name+r'\s*:\s*([0-9.]+)',sc); return float(m.group(1))
leg_f=frac('leg_fraction'); torso_f=frac('torso_fraction'); sh_f=frac('shoulder_height_fraction'); w_f=frac('width_fraction')
m=re.search(r'"medium"\s*:\s*\(\s*height_m\s*:\s*([0-9.]+)\s*,\s*cortex_height_m\s*:\s*([0-9.]+)',sc)
H=float(m.group(1)); CY=float(m.group(2))
print("medium height",H,"cortex",CY, "fracs",leg_f,torso_f,sh_f,w_f)
fit=CY/8.899999044124561
head_m=H-leg_f*H-torso_f*H
neck_r=head_m/2/fit  # in glb units
sh=sh_f*H/fit
hgt=H/fit
CAPS["neck"]=((0,sh-neck_r,0),(0,hgt-neck_r,0),neck_r)
print("neck seg glb units: r %.3f y %.2f..%.2f"%(neck_r,sh-neck_r,hgt-neck_r))
names=list(CAPS)
def out_dist(s):
    return min(seg_dist(s,CAPS[n][0],CAPS[n][1])[0]-CAPS[n][2] for n in names)
outs=sorted((out_dist(s),s) for s in samples)
print("flesh outside nearest capsule (glb units): p95 %.3f p99 %.3f max %.3f at %s"%(
  outs[int(len(outs)*0.95)][0],outs[int(len(outs)*0.99)][0],outs[-1][0],["%.2f"%c for c in outs[-1][1]]))
# air: sample each authored capsule surface, nearest mesh sample distance
import random
random.seed(1)
grid={}
CELL=0.4
for s in samples:
    grid.setdefault((int(s[0]//CELL),int(s[1]//CELL),int(s[2]//CELL)),[]).append(s)
def near_mesh(p):
    best=9e9
    ci,cj,ck=int(p[0]//CELL),int(p[1]//CELL),int(p[2]//CELL)
    for rad in range(1,8):
        for i in range(ci-rad,ci+rad+1):
            for j in range(cj-rad,cj+rad+1):
                for k in range(ck-rad,ck+rad+1):
                    for s in grid.get((i,j,k),[]):
                        d=norm(sub(p,s))
                        if d<best: best=d
        if best< (rad-1)*CELL: break
    return best
for n in names:
    a,b,r=CAPS[n]
    ax=sub(b,a); L=norm(ax); ax=mul(ax,1/max(L,1e-9))
    # perpendicular basis
    up=(0,1,0) if abs(ax[1])<0.9 else (1,0,0)
    u=(ax[1]*up[2]-ax[2]*up[1], ax[2]*up[0]-ax[0]*up[2], ax[0]*up[1]-ax[1]*up[0])
    u=mul(u,1/norm(u))
    v=(ax[1]*u[2]-ax[2]*u[1], ax[2]*u[0]-ax[0]*u[2], ax[0]*u[1]-ax[1]*u[0])
    worst=0; wp=None
    for ti in range(12):
        t=ti/11
        c=add(a,mul(sub(b,a),t))
        for gi in range(16):
            ang=gi/16*2*math.pi
            p=add(c,add(mul(u,r*math.cos(ang)),mul(v,r*math.sin(ang))))
            d=near_mesh(p)
            if d>worst: worst=d; wp=p
    print(f"air {n:7s}: capsule surface up to {worst:.3f} glb ({worst*fit:.2f} m at husk) from drawn flesh, at {['%.2f'%c for c in wp]}")
