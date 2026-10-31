#!/usr/bin/env python3
"""Highlight spread per sphere: p99 / median luminance over each of the
eight spheres (the disc found from Calib-Glossy's red Iray spheres), lower
= broader highlight (#56). Both renders decoded at gamma 2.2.

  python3 scripts/dev/highlight-spread.py [--vs <iray.png>] <render.png>...
--vs adds each sphere's mean luminance over the reference's (render / Iray).
"""
import os, sys
import numpy as np
from PIL import Image

iray = os.path.expanduser('~/Library/Caches/orbis/calib/iray')
ref = os.path.join(os.path.dirname(__file__), '..', '..', 'calib', 'references', 'iray', 'Calib-Glossy.png')
Y = np.array([0.2126, 0.7152, 0.0722])


def load(p):
    return (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2


g = load(ref)
red = (g[..., 0] > 1.6 * g[..., 1]) & (g[..., 0] > 0.12)
cols = np.flatnonzero(red.any(0))
groups, start = [], cols[0]
for c0, c1 in zip(cols[:-1], cols[1:]):
    if c1 - c0 > 5:
        groups.append((start, c0)); start = c1
groups.append((start, cols[-1]))
discs = []
for x0, x1 in (g for g in groups if g[1] - g[0] > 20):
    rows = np.flatnonzero(red[:, x0:x1 + 1].any(1))
    discs.append(((x0 + x1) / 2, (rows[0] + rows[-1]) / 2, (x1 - x0) / 2))
yy, xx = np.mgrid[0:g.shape[0], 0:g.shape[1]]
args = sys.argv[1:]
vs = None
if args[:1] == ['--vs']:
    vs, args = load(args[1]) @ Y, args[2:]
for p in args:
    L = load(p) @ Y
    spread, ratio = [], []
    for cx, cy, r in discs:
        m = (np.hypot(xx - cx, yy - cy) < 0.9 * r) & (yy < cy + 0.3 * r)
        spread.append(np.percentile(L[m], 99) / np.median(L[m]))
        if vs is not None:
            ratio.append(L[m].mean() / vs[m].mean())
    print(f"{os.path.basename(p)}: spread " + ' '.join(f'{x:.1f}' for x in spread)
          + ('' if vs is None else '; / Iray ' + ' '.join(f'{x:.2f}' for x in ratio)))
