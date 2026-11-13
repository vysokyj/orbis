#!/usr/bin/env python3
"""Radial profiles of the white-furnace spheres (#117): Iray / Octane
renders against the `libbsdf` and Octane (Smith) sphere models.

  furnace-rings.py measure <render.png> [<render.png> ...]
      Per sphere of a Calib-Furnace-* scene (eight spheres in a row, the
      Calib-NVIDIA2 / -NVIDIA3 / uber_spheres layout): radiance over the
      dome's per ring of the entry radius (0-0.1 ... 0.8-0.9 of the
      silhouette) and the disk r < 0.75. Gamma 2.2 decode (DAZ's tone
      mapping, the export's imager response). Octane is read in the
      export's own pass (scripts/dev/octane_pass.py: the `denoised/` twin
      when the variant denoises, #139), named in the output.
  furnace-rings.py model [--model libbsdf|smith] [--mode transmit|rt|reflect]
                         [--rough R] [--ior N] [--thin] [--weight W]
                         [--curve fresnel|schlick] [--coat-rough R]
                         [--coat-ior N] [--tau T] [--loss EXPR] [--paths N]
      The same rings for a closed (or thin-walled) unit sphere traced as
      src/mdl_layer.rs traces it (a numpy port: Phong half-vector
      sampling + v-cavities for `libbsdf`, Beckmann + Smith for Octane;
      `fresnel_layer` / `custom_curve_layer`'s macro-normal weights for
      `libbsdf`, physical layering for Octane). --tau multiplies every base
      event; --loss is a per-base-event factor in `ci` (|n.k1|, the macro
      cosine the event is entered from), `co` (|n.k2|, the one it leaves
      to), `ce` (the camera ray's cosine at the entry point) and `inside`
      (1 when the event is met from inside), e.g.
      "0.8 + 0.2 * ci" - a hypothesis to hold against Iray's rings;
      --tir-dies: a transmit-only lobe kills a totally reflected sample
      instead of reflecting it.

Rings are by the entry point (a pixel's ray), so a ring mixes the entry
event at cos = sqrt(1 - r^2) with whatever happens inside. In a
thin-walled sphere both walls are met at that cosine: its rings are the
per-event loss as a function of the angle.

Spheres: centres / half-axes projected from the render's scene
(scripts/dev/sphere_discs.py, #146; printed first); the off-axis spheres
are wider under the perspective camera, so rings are elliptic.
"""
import argparse
import os
import sys

import numpy as np

RINGS = [(i / 10, (i + 1) / 10) for i in range(9)]


def lum(png):
    from PIL import Image
    a = np.asarray(Image.open(png).convert('RGB')).astype(float) / 255.0
    return (a ** 2.2) @ np.array([0.2126, 0.7152, 0.0722])


def measure(png):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import octane_pass
    import sphere_discs
    src, label = octane_pass.pick(png)
    L = lum(src)
    discs, dsrc = sphere_discs.discs(png, L.shape)
    print(sphere_discs.describe(discs, dsrc))
    bg = np.median(L[100:300, 100:300])
    yy, xx = np.mgrid[0:L.shape[0], 0:L.shape[1]]
    print(f'{src} [{label}]  dome {bg:.4f}')
    print('sphere  disk<0.75  ' + ' '.join(f'{a:.1f}-{b:.1f}' for a, b in RINGS))
    for k, d in enumerate(discs):
        r = sphere_discs.dist(d, L.shape, (yy, xx))
        disk = L[r < 0.75].mean() / bg
        rings = [L[(r >= a) & (r < b)].mean() / bg for a, b in RINGS]
        print(f'{k + 1:>6}  {disk:9.3f}  ' + ' '.join(f'{v:7.3f}' for v in rings))


# --- the sphere models (port of src/mdl_layer.rs) -------------------------

def fresnel(eta, c):
    c = np.clip(c, 0.0, 1.0)
    ct2 = 1.0 - (1.0 - c * c) / (eta * eta)
    ct = np.sqrt(np.maximum(ct2, 0.0))
    rp = (ct - c * eta) / (ct + c * eta)
    ro = (c - ct * eta) / (c + ct * eta)
    return np.where(ct2 < 0.0, 1.0, np.clip(0.5 * (rp * rp + ro * ro), 0.0, 1.0))


def dot(a, b):
    return np.sum(a * b, axis=-1)


def normalize(a):
    return a / np.maximum(np.linalg.norm(a, axis=-1, keepdims=True), 1e-300)


def sample_h(model, rough, n, rng):
    u1, u2 = rng.random(len(n)), rng.random(len(n))
    if model == 'libbsdf':
        e = 2.0 / max(rough, 1e-7) ** 2
        c = (1.0 - u1) ** (1.0 / (e + 1.0))
    else:
        c = 1.0 / np.sqrt(1.0 - rough * rough * np.log(1.0 - u1))
    s = np.sqrt(np.maximum(1.0 - c * c, 0.0))
    phi = 2 * np.pi * u2
    a = np.where(np.abs(n[:, :1]) > 0.9, [[0.0, 1.0, 0.0]], [[1.0, 0.0, 0.0]])
    t = normalize(np.cross(a, n))
    b = np.cross(n, t)
    return t * (s * np.cos(phi))[:, None] + b * (s * np.sin(phi))[:, None] + n * c[:, None]


def smith_g1(c, alpha):
    c = np.clip(np.abs(c), 1e-9, 1.0)
    x = c / (alpha * np.sqrt(1.0 - c * c) + 1e-12)
    return np.where(x < 1.6, (3.535 * x + 2.181 * x * x) / (1.0 + 2.276 * x + 2.577 * x * x), 1.0)


def vcav(nh, kh, nk):
    return np.minimum(2.0 * nh * nk / np.maximum(kh, 1e-12), 1.0)


def lobe(model, rough, n, v, h, eta, mode, thin, rng, tir_dies=False):
    """Direction, weight (0 = dead), refracted?"""
    vh, nv, nh = dot(v, h), dot(n, v), dot(n, h)
    ok = (vh > 0) & (nh > 0) & (nv > 0)
    e = np.ones(len(n)) if thin else eta
    ct2 = 1.0 - (1.0 - vh * vh) / (e * e)
    if mode == 'reflect':
        refr = np.zeros(len(n), bool)
    elif mode == 'rt':
        refr = rng.random(len(n)) >= fresnel(eta, vh)
    else:
        refr = ct2 >= 0.0
        if tir_dies:
            ok &= refr
    refl_dir = h * (2 * vh)[:, None] - v
    if thin:
        # thin-walled: the transmission is the reflection mirrored through the surface
        tr_dir = refl_dir - n * (2 * dot(n, refl_dir))[:, None]
    else:
        tr_dir = -v / e[:, None] + h * (vh / e - np.sqrt(np.maximum(ct2, 0.0)))[:, None]
    l = np.where(refr[:, None], tr_dir, refl_dir)
    nl = dot(n, l)
    ok &= np.where(refr, nl < 0, nl > 0)
    if model == 'libbsdf':
        g1, g2 = vcav(nh, vh, nv), vcav(nh, np.abs(dot(l, h)), np.abs(nl))
        g = np.where(refr & (not thin), np.maximum(g1 + g2 - 1.0, 0.0), np.minimum(g1, g2))
    else:
        g = smith_g1(nv, rough) * smith_g1(nl, rough)
    w = np.where(ok, vh * g / np.maximum(nv * nh, 1e-12), 0.0)
    return l, w, refr


def curve(kind, ior, r0, inside, c):
    if kind == 'fresnel':
        return fresnel(np.where(inside, 1.0 / ior, ior), c)
    return r0 + (1.0 - r0) * (1.0 - np.clip(c, 0, 1)) ** 5


def trace(a):
    rng = np.random.default_rng(117)
    N = a.paths
    rr = np.sqrt(rng.random(N))
    phi = 2 * np.pi * rng.random(N)
    p = np.stack([rr * np.cos(phi), rr * np.sin(phi), np.sqrt(np.maximum(1 - rr * rr, 0))], -1)
    d = np.tile([0.0, 0.0, -1.0], (N, 1))
    thr = np.ones(N)
    inside = np.zeros(N, bool)
    alive = np.ones(N, bool)
    out = np.zeros(N)
    coat_ior = a.coat_ior or a.ior
    r0 = ((a.ior - 1) / (a.ior + 1)) ** 2
    shared = a.coat_rough is None or a.coat_rough == a.rough
    coat_rough = a.rough if a.coat_rough is None else a.coat_rough
    for _ in range(32):
        idx = np.nonzero(alive)[0]
        if len(idx) == 0:
            break
        P, D, I = p[idx], d[idx], inside[idx]
        n = np.where(I[:, None], -P, P)
        v = -D
        eta = np.where(I, 1.0 / a.ior, a.ior)
        nk1 = np.clip(dot(n, v), 0, 1)
        h = sample_h(a.model, a.rough, n, rng)
        hc = h if shared else sample_h(a.model, coat_rough, n, rng)
        if a.model == 'libbsdf':
            p_layer = a.weight * curve(a.curve, a.ior, r0, I, nk1)
        else:
            p_layer = a.weight * curve(a.curve, coat_ior, r0, I, dot(v, hc))
        pick_coat = rng.random(len(idx)) < p_layer
        lc, wc, _ = lobe(a.model, coat_rough, n, v, hc, eta, 'reflect', a.thin, rng)
        lb, wb, rb = lobe(a.model, a.rough, n, v, h, eta, a.mode, a.thin, rng, a.tir_dies)
        if a.model == 'libbsdf':
            wc = wc * a.weight * curve(a.curve, a.ior, r0, I, dot(v, hc)) / np.maximum(p_layer, 1e-12)
            if a.curve == 'fresnel':
                nk2 = np.where(rb, 2 * dot(v, h) * dot(n, h) - nk1, np.abs(dot(n, lb)))
            else:
                nk2 = np.abs(dot(n, lb))
            base = 1 - a.weight * np.maximum(curve(a.curve, a.ior, r0, I, nk1), curve(a.curve, a.ior, r0, I, nk2))
            wb = wb * base / np.maximum(1 - p_layer, 1e-12)
        ci, co = nk1, np.abs(dot(n, lb))
        ce = np.sqrt(np.maximum(1 - rr[idx] ** 2, 0))
        wb = wb * a.tau * eval(a.loss, {'np': np}, {'ci': ci, 'co': co, 'ce': ce, 'inside': I.astype(float)})
        l = np.where(pick_coat[:, None], lc, lb)
        w = np.where(pick_coat, wc, wb)
        refr = ~pick_coat & rb
        thr[idx] *= w
        dead = w <= 0
        inside[idx] = I ^ refr
        esc = ~inside[idx] & ~dead
        out[idx[esc]] = thr[idx[esc]]
        alive[idx[esc | dead]] = False
        keep = idx[~esc & ~dead]
        L = normalize(l[~esc & ~dead])
        P2 = p[keep]
        p[keep] = normalize(P2 - L * (2 * dot(P2, L))[:, None])
        d[keep] = L
    return rr, out


def model(a):
    rr, out = trace(a)
    disk = out[rr < 0.75].mean()
    rings = [out[(rr >= x) & (rr < y)].mean() for x, y in RINGS]
    print(f'model {a.model} {a.mode} r {a.rough} ior {a.ior}{" thin" if a.thin else ""} w {a.weight} {a.curve} tau {a.tau} loss {a.loss}')
    print(f'  all {out.mean():.3f}  disk {disk:.3f}  ' + ' '.join(f'{v:7.3f}' for v in rings))


def main():
    np.seterr(all='ignore')
    if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
        print(__doc__)
        return
    if sys.argv[1] == 'measure':
        for png in sys.argv[2:]:
            measure(png)
        return
    ap = argparse.ArgumentParser()
    ap.add_argument('--model', default='libbsdf', choices=['libbsdf', 'smith'])
    ap.add_argument('--mode', default='transmit', choices=['transmit', 'rt', 'reflect'])
    ap.add_argument('--rough', type=float, default=0.1)
    ap.add_argument('--ior', type=float, default=1.5)
    ap.add_argument('--thin', action='store_true')
    ap.add_argument('--weight', type=float, default=0.0)
    ap.add_argument('--curve', default='fresnel', choices=['fresnel', 'schlick'])
    ap.add_argument('--coat-rough', type=float, default=None)
    ap.add_argument('--coat-ior', type=float, default=None)
    ap.add_argument('--tau', type=float, default=1.0)
    ap.add_argument('--tir-dies', action='store_true', help='a transmit lobe kills a totally reflected sample (hypothesis)')
    ap.add_argument('--loss', default='1.0')
    ap.add_argument('--paths', type=int, default=1 << 18)
    model(ap.parse_args(sys.argv[2:]))


if __name__ == '__main__':
    main()
