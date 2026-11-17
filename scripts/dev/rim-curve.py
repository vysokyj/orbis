#!/usr/bin/env python3
"""A dielectric lobe's curve per ring of a sphere: MDL's Schlick against Octane's Fresnel (#188).

  rim-curve.py model <alpha1> <n> [ratio2 alpha2 [grazing]]
      The curve's part alone, per ring of a sphere seen head-on (r / R
      bins below): the directional albedo of the dual lobe in a white
      furnace with Octane's curve (grazing x Fresnel at the IOR of n /
      grazing, how the export writes the layer) over the same lobes with
      the MDL's `custom_curve_layer` (n + (grazing - n)(1 - k.h)^5, the
      layer weighted at the microfacet as `libbsdf`'s curve_layer_sample).
      Both on Iray's distribution (GGX at alpha = roughness^2, lobe 2 at
      ratio2), so the ratio is the curve and nothing else. n = Iray's
      normal reflectivity (specularlevel x the occlusion's facing).
  rim-curve.py measure <octane.png> <iray.png> [--matte LABEL] [--frame F]
                       [LABEL=alpha1,n[,ratio2,alpha2[,grazing]]]...
      Per sphere and ring, the blue channel Octane / Iray (a red base
      leaves almost only the white specular there), minus the matte
      sphere's blue (--matte: its label prefix, the diffuse's share), and
      for each LABEL given the curve's ratio from `model` and what is left
      over it and over the frame (--frame, default the matte sphere's red
      Octane / Iray per ring, else 1). Discs from the scene's geometry
      (sphere_discs.py); Octane in the export's own pass (octane_pass.py).

Uniform light is an approximation of the scene's HDRI; where the ratio of
the measured to the model is flat over the rings, the curve is the shape.
"""
import sys
import os
if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import numpy as np

EDGES = [0, 0.3, 0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.93]


def fresnel(n, c):
    c = np.clip(c, 0, 1)
    ct = np.sqrt(np.maximum(1 - (1 - c * c) / (n * n), 0))
    rp = (ct - c * n) / (ct + c * n)
    ro = (c - ct * n) / (c + ct * n)
    return 0.5 * (rp * rp + ro * ro)


def ior_of(f0):
    return (1 + np.sqrt(f0)) / (1 - np.sqrt(f0))


def g1(a, c):
    c = np.clip(c, 1e-6, 1)
    return 2 / (1 + np.sqrt(1 + a * a * (1 - c * c) / (c * c)))


def albedo(a, thv, curve, rng, n=40000):
    """E[curve(v.h) G1(l)] over GGX's visible normals (Heitz 2018)."""
    if a < 1e-4:
        return curve(np.cos(thv))
    v = np.array([np.sin(thv), 0, np.cos(thv)])
    vh = np.array([a * v[0], a * v[1], v[2]])
    vh /= np.linalg.norm(vh)
    t1v = np.cross([0, 0, 1], vh)
    t1v /= np.linalg.norm(t1v)
    t2v = np.cross(vh, t1v)
    u1, u2 = rng.random(n), rng.random(n)
    r, phi = np.sqrt(u1), 2 * np.pi * u2
    t1, t2 = r * np.cos(phi), r * np.sin(phi)
    s = 0.5 * (1 + vh[2])
    t2 = (1 - s) * np.sqrt(1 - t1 * t1) + s * t2
    nh = t1[:, None] * t1v + t2[:, None] * t2v + np.sqrt(np.maximum(0, 1 - t1 * t1 - t2 * t2))[:, None] * vh
    h = np.stack([a * nh[:, 0], a * nh[:, 1], np.maximum(1e-6, nh[:, 2])], 1)
    h /= np.linalg.norm(h, axis=1)[:, None]
    vdh = h @ v
    l = 2 * vdh[:, None] * h - v
    return np.mean(np.where(l[:, 2] > 0, g1(a, l[:, 2]), 0) * curve(vdh))


def model(alpha1, n, ratio2=0.0, alpha2=0.0, grazing=1.0):
    rng = np.random.default_rng(1)
    octane = lambda c: grazing * fresnel(ior_of(min(n / grazing, 0.99)), c)
    iray = lambda c: n + (grazing - n) * (1 - np.clip(c, 0, 1)) ** 5
    out = []
    for lo, hi in zip(EDGES, EDGES[1:]):
        rs = lo + (np.arange(5) + 0.5) * (hi - lo) / 5
        eo = ei = 0
        for rr in rs:
            th = np.arcsin(rr)
            for a, w in ((alpha1, 1 - ratio2), (alpha2, ratio2)):
                if w > 0:
                    eo += rr * w * albedo(a, th, octane, rng)
                    ei += rr * w * albedo(a, th, iray, rng)
        out.append(eo / ei)
    return np.array(out)


def fmt(v):
    return ' '.join(f'{x:5.2f}' for x in v)


def parse_lobe(s):
    """LABEL=alpha1,n[,...] -> (label, values); None for anything else.

    A file that exists is a path, whatever it is named: orbx-variants
    renders carry `name=value` in their names (#192)."""
    if os.path.exists(s) or '=' not in s:
        return None
    label, vals = s.split('=', 1)
    if not label or os.sep in label:
        return None
    try:
        return label, [float(x) for x in vals.split(',')]
    except ValueError:
        return None


def measure(args):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import sphere_discs
    import octane_pass
    from PIL import Image
    load = lambda p: (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2
    matte, frame, lobes, paths = None, None, [], []
    it = iter(args)
    for a in it:
        if a == '--matte':
            matte = next(it)
        elif a == '--frame':
            frame = float(next(it))
        elif (lobe := parse_lobe(a)) is not None:
            lobes.append(lobe)
        else:
            paths.append(a)
    p, label = octane_pass.pick(paths[0])
    print(f'{os.path.basename(p)}: Octane pass {label}')
    O, I = load(p), load(paths[1])
    ds, src = sphere_discs.discs(paths[0], shape=O.shape)
    print(sphere_discs.describe(ds, src))
    yy, xx = np.mgrid[0:O.shape[0], 0:O.shape[1]]

    def rings(d):
        rr = np.sqrt(((xx - d.cx) / d.rx) ** 2 + ((yy - d.cy) / d.ry) ** 2)
        ms = [(rr >= lo) & (rr < hi) for lo, hi in zip(EDGES, EDGES[1:])]
        return np.array([[O[m].mean(0), I[m].mean(0)] for m in ms])  # ring, (O, I), rgb

    base = None
    fr = np.ones(len(EDGES) - 1) * (frame or 1.0)
    if matte:
        dm = next(d for d in ds if d.label.startswith(matte))
        base = rings(dm)
        if frame is None:
            fr = base[:, 0, 0] / base[:, 1, 0]
    print('rings r/R  ', ' '.join(f'{lo:.2f}' for lo in EDGES[:-1]), '(lower edges)')
    print(f'{"frame":24s}  {fmt(fr)}')
    for d in ds:
        if matte and d.label.startswith(matte):
            continue
        r = rings(d)
        o, i = r[:, 0, 2], r[:, 1, 2]
        if base is not None:
            o, i = o - base[:, 0, 2], i - base[:, 1, 2]
        meas = o / i
        print(f'{d.label[:24]:24s}B {fmt(meas)}')
        lobe = next((v for l, v in lobes if d.label.startswith(l)), None)
        if lobe:
            m = model(*lobe)
            print(f'{"  curve (model)":24s}  {fmt(m)}')
            print(f'{"  left over the curve":24s}  {fmt(meas / m)}')
            print(f'{"  ... and the frame":24s}  {fmt(meas / m / fr)}')


if sys.argv[1] == 'model':
    print('rings r/R  ', ' '.join(f'{lo:.2f}' for lo in EDGES[:-1]), '(lower edges)')
    print('Octane / Iray', fmt(model(*[float(x) for x in sys.argv[2:]])))
elif sys.argv[1] == 'measure':
    measure(sys.argv[2:])
else:
    print(__doc__)
    sys.exit(1)
