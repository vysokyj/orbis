#!/usr/bin/env python3
"""A white gloss over a red diffuse, per sphere, split by colour (#137).

  gloss-split.py measure <octane dir> <iray dir> <scene> <bare scene> [disc|core|rim]
      Per sphere of a red uber_spheres row (Calib-Glossy, -DualLobe,
      -DualLobeGlossy...): each pixel is a * bare + g * white, so with the
      bare (matte) scene's own pixels a = (R - G) / (Rb - Gb), g = R - a Rb
      (means over the region). Prints the diffuse kept (a) and the gloss
      (g) in both renderers, g's Octane / Iray, the bare scene's Octane /
      Iray (the frame's offset), and where the gloss peaks on the sphere
      (offset / radius, theta_d = the half angle between view and light at
      a peak, where n = h). Discs: projected from <scene>'s geometry
      (scripts/dev/sphere_discs.py, printed first), else found in the Iray
      render of <bare> (all spheres matte), never in a glossy render (#146). Gamma 2.2
      decode; Octane in the export's own pass (scripts/dev/octane_pass.py:
      the `denoised/` twin when the variant denoises, #139), named first.
  gloss-split.py fresnel
      Universal's physical Fresnel at the IOR of F0 = 0.08 r against the
      MDL's Schlick curve F0 + (1 - F0)(1 - cos)^5, per angle and over the
      hemisphere (cosine-weighted).
  gloss-split.py sun <dx> <dy> [--rough R] [--f0 F] [--dual R1 R2 RATIO]
      One distant light whose highlight peaks at (dx, dy) (offset / radius,
      y up) on a sphere seen head-on: the gloss under it, single scatter,
      `libbsdf`'s model (Phong half-vector D, v-cavities, the curve:
      Schlick at k.h for the glossy layer; W * Schlick at max(n.k1, n.k2)
      for the dual lobe, the custom_curve_layer's base) against Octane's
      (Beckmann at alpha = r / sqrt(1 + r^2), Smith, Fresnel at the IOR of
      F0) - Octane / Iray over the disc, the core (r < 0.7) and the peak.
"""
import sys
if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import os
import numpy as np

Y = np.array([0.2126, 0.7152, 0.0722])


def load(p, octane=False):
    from PIL import Image
    if octane:
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import octane_pass
        p, label = octane_pass.pick(p)
        print(f'{os.path.basename(p)}: Octane pass {label}')
    return (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2


def fresnel(n, c):
    c = np.clip(c, 0, 1)
    ct = np.sqrt(np.maximum(1 - (1 - c * c) / (n * n), 0))
    rp = (ct - c * n) / (ct + c * n)
    ro = (c - ct * n) / (c + ct * n)
    return 0.5 * (rp * rp + ro * ro)


def ior_of(f0):
    return (1 + np.sqrt(f0)) / (1 - np.sqrt(f0))


def measure(od, idir, scene, bare, region):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import sphere_discs
    bI = load(os.path.join(idir, bare + '.png'))
    discs, dsrc = sphere_discs.discs(scene, bI.shape, matte=bI)
    print(sphere_discs.describe(discs, dsrc))
    yy, xx = np.mgrid[0:bI.shape[0], 0:bI.shape[1]]
    out = {}
    for name, d in (('O', od), ('I', idir)):
        S, B = load(os.path.join(d, scene + '.png'), name == 'O'), load(os.path.join(d, bare + '.png'), name == 'O')
        den = B[..., 0] - B[..., 1]
        ok = np.abs(den) > 0.01
        gmap = S[..., 0] - np.where(ok, (S[..., 0] - S[..., 1]) / np.where(ok, den, 1), 1) * B[..., 0]
        k = np.ones(5) / 5
        gmap = np.apply_along_axis(lambda v: np.convolve(v, k, 'same'), 0, gmap)
        gmap = np.apply_along_axis(lambda v: np.convolve(v, k, 'same'), 1, gmap)
        rows = []
        for disc in discs:
            cx, cy, r = disc.cx, disc.cy, disc.ry
            dist = sphere_discs.dist(disc, bI.shape, (yy, xx))
            m = {'disc': dist < 0.9, 'core': dist < 0.7, 'rim': (dist >= 0.7) & (dist < 0.95)}[region]
            s, b = S[m].mean(0), B[m].mean(0)
            a = (s[0] - s[1]) / (b[0] - b[1])
            pk = np.where(dist < 0.93, gmap, -1)
            j = np.argmax(pk)
            py, px = divmod(j, pk.shape[1])
            rows.append((a, s[0] - a * b[0], b @ Y, pk.flat[j], (px - cx) / disc.rx, (cy - py) / r))
        out[name] = rows
    print(f'{scene} over {bare} [{region}], {len(discs)} spheres')
    print('sph  bare O/I | diffuse kept O I | gloss O I  O/I | peak O/I at (dx, dy) theta_d')
    for k, (o, i) in enumerate(zip(out['O'], out['I'])):
        ratio = o[1] / i[1] if abs(i[1]) > 1e-4 else float('nan')
        dd = min(np.hypot(i[4], i[5]), 1.0)
        print(f'{k + 1}    {o[2] / i[2]:.3f} | {o[0]:.3f} {i[0]:.3f} | {o[1]:.4f} {i[1]:.4f} {ratio:.2f}'
              f' | {o[3] / max(i[3], 1e-4):.2f} at ({i[4]:+.2f}, {i[5]:+.2f}) {np.degrees(np.arcsin(dd)):.0f}')


def sun(dx, dy, rough, f0, dual, n_px=301):
    w_ior = ior_of(f0)
    npk = np.array([dx, dy, np.sqrt(max(1 - dx * dx - dy * dy, 0))])
    v = np.array([0, 0, 1.0])
    l = 2 * (npk @ v) * npk - v
    xs = np.linspace(-1, 1, n_px)
    X, Yy = np.meshgrid(xs, xs)
    Z = np.sqrt(np.clip(1 - X * X - Yy * Yy, 0, 1))
    n = np.stack([X, Yy, Z], -1)
    nv, nl = Z, n @ l
    h = (v + l) / np.linalg.norm(v + l)
    nh, vh = n @ h, v @ h
    ok = (X * X + Yy * Yy < 1) & (nl > 0) & (nv > 0)
    schlick = lambda c: f0 + (1 - f0) * (1 - np.clip(c, 0, 1)) ** 5
    lobes = [(rough, 1.0)] if dual is None else [(dual[0], dual[2]), (dual[1], 1 - dual[2])]
    fi = fo = 0
    with np.errstate(all='ignore'):
        for r, wt in lobes:
            if wt <= 0:
                continue
            e = 2 / r ** 2
            d_phong = (e + 1) / (2 * np.pi) * np.clip(nh, 0, 1) ** (e - 1)
            a = r / np.sqrt(1 + r * r)
            c2 = np.clip(nh, 1e-6, 1) ** 2
            d_beck = np.exp(-(1 - c2) / (c2 * a * a)) / (np.pi * a * a * c2 * c2)
            vcav = np.minimum(np.minimum(2 * nh * nv / vh, 1), np.minimum(2 * nh * nl / vh, 1))

            def g1(c):
                c = np.clip(c, 1e-9, 1)
                x = c / (a * np.sqrt(1 - c * c) + 1e-12)
                return np.where(x < 1.6, (3.535 * x + 2.181 * x * x) / (1 + 2.276 * x + 2.577 * x * x), 1.0)
            curve = schlick(vh) if dual is None else schlick(np.maximum(nv, nl))
            fi = fi + wt * curve * d_phong * vcav / (4 * nv)
            fo = fo + wt * fresnel(w_ior, vh) * d_beck * g1(nv) * g1(nl) / (4 * nv)
    Li, Lo = np.where(ok, fi, 0), np.where(ok, fo, 0)
    core = ok & (X * X + Yy * Yy < 0.49)
    j = np.unravel_index(np.argmax(Li), Li.shape)
    print(f'theta_d {np.degrees(np.arccos(vh)):.0f}: Octane / Iray disc {Lo[ok].sum() / Li[ok].sum():.3f}'
          f' core {Lo[core].sum() / max(Li[core].sum(), 1e-12):.3f} peak {Lo[j] / Li[j]:.3f}'
          f' (Fresnel / Schlick at the peak {fresnel(w_ior, vh) / schlick(vh):.3f})')


cmd, args = sys.argv[1], sys.argv[2:]
if cmd == 'measure':
    measure(*args[:4], args[4] if len(args) > 4 else 'disc')
elif cmd == 'fresnel':
    c = np.linspace(0, 1, 200001)[1:]
    for r in (0.1, 0.25, 0.5, 1.0):
        f0 = 0.08 * r
        n = ior_of(f0)
        s = lambda x: f0 + (1 - f0) * (1 - x) ** 5
        per = ' '.join(f'{d}° {fresnel(n, np.cos(np.radians(d))) / s(np.cos(np.radians(d))):.2f}' for d in (0, 30, 45, 55, 60, 70, 80, 85))
        hemi = np.trapezoid(fresnel(n, c) * c, c) / np.trapezoid(s(c) * c, c)
        print(f'r {r} (F0 {f0:.3f}, IOR {n:.3f}): {per}; hemispherical {hemi:.3f}')
elif cmd == 'sun':
    rough, f0, dual, rest = 0.25, 0.04, None, args[2:]
    while rest:
        if rest[0] == '--rough':
            rough, rest = float(rest[1]), rest[2:]
        elif rest[0] == '--f0':
            f0, rest = float(rest[1]), rest[2:]
        elif rest[0] == '--dual':
            dual, rest = tuple(float(x) for x in rest[1:4]), rest[4:]
        else:
            sys.exit(f'unknown option {rest[0]}')
    sun(float(args[0]), float(args[1]), rough, f0, dual)
else:
    sys.exit(__doc__)
