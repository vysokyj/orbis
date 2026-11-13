#!/usr/bin/env python3
"""Highlight spread per sphere: p99 / median luminance over each sphere of
the render's scene (0.9 of the disc, without its part more than 0.3 R below the centre; the discs
projected from the scene, scripts/dev/sphere_discs.py, printed first, #146),
lower = broader highlight (#56). Both renders decoded at gamma 2.2.

  python3 scripts/dev/highlight-spread.py [--vs <iray.png>] <render.png>...
--vs adds each sphere's mean luminance over the reference's (render / Iray).
Each render is read in the export's own pass (scripts/dev/octane_pass.py:
the `denoised/` twin when the variant denoises, #139), named in the output.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import os, sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import octane_pass
import sphere_discs

Y = np.array([0.2126, 0.7152, 0.0722])


def load(p):
    return (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2


args = sys.argv[1:]
vs = None
if args[:1] == ['--vs']:
    vs, args = load(args[1]) @ Y, args[2:]
for p in args:
    src, label = octane_pass.pick(p)
    L = load(src) @ Y
    discs, dsrc = sphere_discs.discs(p, L.shape)
    yy, xx = np.mgrid[0:L.shape[0], 0:L.shape[1]]
    print(sphere_discs.describe(discs, dsrc))
    spread, ratio = [], []
    for d in discs:
        m = sphere_discs.mask(d, L.shape, 0.9, (yy, xx)) & (yy < d.cy + 0.3 * d.ry)
        spread.append(np.percentile(L[m], 99) / np.median(L[m]))
        if vs is not None:
            ratio.append(L[m].mean() / vs[m].mean())
    print(f"{os.path.basename(p)} [{label}]: spread " + ' '.join(f'{x:.1f}' for x in spread)
          + ('' if vs is None else '; / Iray ' + ' '.join(f'{x:.2f}' for x in ratio)))
