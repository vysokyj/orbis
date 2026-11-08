#!/usr/bin/env python3
# Slab transmission Monte Carlo: a backlit slab of a homogeneous volume
# (normal incidence, eta 1.4 Fresnel at both faces), the Henyey-Greenstein
# phase at g against the similar isotropic medium (sigma_s (1 - g), g 0).
#   scripts/dev/sss-slab-mc.py <sigma_a /mm> <sigma_s /mm> <g>
# Prints the transmission at 1 / 2 / 3 / 5 / 9 mm for HG g, the similar
# isotropic medium, their ratio, and the isotropic medium at the unreduced
# sigma_s (at g 0 that is Iray's Calib-SSSThick volume: Victoria 8's red
# 0.043 0.356 gives 0.74 / 0.59 / 0.46 / 0.30 / 0.12, Iray 0.73 / 0.56 /
# 0.44 / 0.28 / 0.12). #88, docs/notes/2026-11-08-sss-direction.md.
import numpy as np, sys
rng=np.random.default_rng(1)
def fres(cosi,eta):  # from inside (n=eta) to outside (1)
    sint=eta*np.sqrt(np.maximum(0,1-cosi**2)); tir=sint>=1
    cost=np.sqrt(np.maximum(0,1-sint**2))
    rs=((eta*cosi-cost)/(eta*cosi+cost))**2; rp=((eta*cost-cosi)/(eta*cost+cosi))**2
    R=0.5*(rs+rp); R[tir]=1; return R
def hg(g,n):
    if abs(g)<1e-6: return 2*rng.random(n)-1
    x=(1-g*g)/(1-g+2*g*rng.random(n)); return (1+g*g-x*x)/(2*g)
def run(sa,ss,g,d,N=200000,eta=1.4):
    st=sa+ss; alb=ss/st
    z=np.zeros(N); mu=np.ones(N); w=np.ones(N)*(1-((eta-1)/(eta+1))**2); T=0.0
    alive=np.ones(N,bool)
    for it in range(3000):
        idx=np.nonzero(alive)[0]
        if len(idx)==0: break
        s=-np.log(rng.random(len(idx)))/st
        zn=z[idx]+mu[idx]*s
        out_t=zn>=d; out_r=zn<=0
        ex=out_t|out_r
        if ex.any():
            e=idx[ex]; R=fres(np.abs(mu[e]),eta); refl=rng.random(len(e))<R
            T+= w[e][out_t[ex]&~refl].sum()
            # reflected: mirror and continue from boundary
            keep=e[refl]; zb=np.where(out_t[ex][refl],d,0.0)
            z[keep]=zb; mu[keep]=-mu[keep]
            alive[e[~refl]]=False
        inn=idx[~ex]; z[inn]=zn[~ex]; w[inn]*=alb
        # scatter
        ct=hg(g,len(inn)); phi=2*np.pi*rng.random(len(inn))
        m=mu[inn]; sinp=np.sqrt(np.maximum(0,1-m*m)); st_=np.sqrt(np.maximum(0,1-ct*ct))
        mu[inn]=m*ct+sinp*st_*np.cos(phi)
        dead=w<1e-3; alive&=~dead
    return T/N
sa,ss=float(sys.argv[1]),float(sys.argv[2]); g=float(sys.argv[3])
for d in [1,2,3,5,9]:
    a=run(sa,ss,g,d); b=run(sa,ss*(1-g),0.0,d); c=run(sa,ss,0.0,d)
    print(f"d {d} mm  HG g {a:.4f}  iso reduced {b:.4f}  ratio {a/b:.3f}   iso unreduced {c:.4f}")
