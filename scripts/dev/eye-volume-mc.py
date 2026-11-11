#!/usr/bin/env python3
"""Genesis 9's eye as Iray sees it (#66): a PBRSkin ball, Monte Carlo.

  python3 scripts/dev/eye-volume-mc.py [<SSS Direction> [<Translucency Weight>]]

Light enters through `pbr_skin.mdl`'s diffuse transmission lobe (weight t,
tint 1, Lambertian), scatters in the volume (`pbr_add_volume`: absorption
from Transmitted Color at 0.1 cm, scattering from PBRSkin's default SSS
Color at 0.015 cm, both linear as in the .mi, Henyey-Greenstein at SSS Direction), and each time it
meets the surface from inside leaves with t (Lambertian) or is reflected
back in with 1 - t (the diffuse layer, white). Prints, per channel, the
share of the entering light that comes out (all of the ball / its front
half): the transmitted path's albedo is t times that, the eye's
(1 - t) + t times that. Radius 1.3 cm (Genesis 9's l_eye bone). No
specular layers, no Fresnel (the MDL's lobes have none).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import numpy as np, sys
rng=np.random.default_rng(2)
def hg(g,n):
    if abs(g)<1e-6: return 2*rng.random(n)-1
    x=(1-g*g)/(1-g+2*g*rng.random(n)); return (1+g*g-x*x)/(2*g)
def rot(d,mu):
    # new direction at cos mu from d, random azimuth
    n=len(d); phi=2*np.pi*rng.random(n); s=np.sqrt(np.maximum(0,1-mu*mu))
    a=np.where(np.abs(d[:,2:3])<0.9,np.array([[0,0,1.]]),np.array([[1.,0,0]]))
    u=np.cross(d,a); u/=np.linalg.norm(u,axis=1,keepdims=True); v=np.cross(d,u)
    return d*mu[:,None]+u*(s*np.cos(phi))[:,None]+v*(s*np.sin(phi))[:,None]
def lamb(nrm):
    n=len(nrm); r=rng.random(n); mu=np.sqrt(r); return rot(nrm,mu)
def run(sa,ss,g,t,T,B,R=13.0,N=200000,front=True):
    st=sa+ss; alb=ss/st
    # entry point: the front pole (the camera sees the front); inward normal
    p=np.tile([0,0,R],(N,1)).astype(float); nin=-p/R
    d=lamb(nin); w=np.ones(N); out=np.zeros(N); outfront=0.0
    alive=np.ones(N,bool)
    for it in range(5000):
        i=np.nonzero(alive)[0]
        if len(i)==0: break
        s=-np.log(rng.random(len(i)))/st
        # distance to sphere exit
        pp=p[i]; dd=d[i]; b=(pp*dd).sum(1); c=(pp*pp).sum(1)-R*R
        tex=-b+np.sqrt(np.maximum(0,b*b-c))
        hit=s>=tex
        # scatter inside
        k=i[~hit]; p[k]=pp[~hit]+dd[~hit]*s[~hit][:,None]; w[k]*=alb
        d[k]=rot(d[k],hg(g,len(k)))
        # at the surface
        h=i[hit]; ph=pp[hit]+dd[hit]*tex[hit][:,None]; p[h]=ph
        r=rng.random(len(h))
        esc=r<t*T; refl=(r>=t*T)&(r<t*T+(1-t)*B)
        e=h[esc]; out[e]+=w[e]; alive[e]=False
        outfront+=w[e][ph[esc][:,2]>0].sum()
        dead=h[~esc&~refl]; alive[dead]=False
        rr=h[refl]; d[rr]=lamb(-p[rr]/R)
        # Russian roulette
        lw=np.nonzero(alive&(w<1e-3))[0]
        if len(lw): kill=rng.random(len(lw))<0.5; alive[lw[kill]]=False; w[lw[~kill]]*=2
    return out.sum()/N, outfront/N
# per channel /mm
# The colours as DAZ hands them to Iray (Calib-G9Eye's .mi, linear: the
# channels' sRGB values decoded): Transmitted Color 0.976 / 0.482 / 0.353,
# SSS Color 0.976 / 0.694 / 0.761.
sa=-np.log(np.array([0.948965,0.201096,0.101145]))/1.0   # 0.1 cm
sss=np.array([0.948965,0.447871,0.547994])
ss=-np.log(sss)/0.15                                          # 0.015 cm
g=float(sys.argv[1]) if len(sys.argv)>1 else -0.7977528
t=float(sys.argv[2]) if len(sys.argv)>2 else 0.8
print('sigma_a /mm',sa,'sigma_s /mm',ss,'albedo',ss/(ss+sa))
for ch in range(3):
    tot,fr=run(sa[ch],ss[ch],g,t,1.0,1.0)
    print('ch',ch,'escape per unit entering', round(tot,4),'front hemisphere',round(fr,4))
