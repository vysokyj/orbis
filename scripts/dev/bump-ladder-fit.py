#!/usr/bin/env python3
"""Which bump power fits Iray, per sphere (#75): Octane renders of one
scene at every bump power × k (a ladder) against the Iray reference. Both
band-passed (Gaussian blur lo minus blur hi, in px, RGB), the residual
over each sphere's inner disc (radius 0.7) relative to Iray's band-passed
signal; the k with the smallest residual fits. Same sphere, same
reflection, so the environment cancels; a phase-free measure (band energy
alone) doesn't work, each sphere reflects a different part of the sky.

  python3 scripts/dev/bump-ladder-fit.py [--band lo,hi] <iray.png> <k>=<render.png>...

Spheres: Calib-BumpRes's eight (R1 R2 R3 R4 R5 T1 T2 S1) at 1920 x 1080.
Ladder: scale BUMP_POWER in src/material.rs, rebuild, calib-variants.sh.
"""
import sys
if len(sys.argv) < 3 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import numpy as np
from PIL import Image, ImageFilter

SPHERES = [(424, 560, 47), (577, 560, 46), (730, 560, 46), (883, 560, 46),
           (1036, 560, 46), (1189, 560, 46), (1343, 560, 46), (1493, 583, 23)]
NAMES = ['R1', 'R2', 'R3', 'R4', 'R5', 'T1', 'T2', 'S1']
args = sys.argv[1:]
lo, hi = 1.5, 6.0
if args[0] == '--band':
    lo, hi = map(float, args[1].split(','))
    args = args[2:]
yy, xx = np.mgrid[:1080, :1920]
discs = [(xx - x) ** 2 + (yy - y) ** 2 < (0.7 * r) ** 2 for x, y, r in SPHERES]


def band(p):
    im = Image.open(p).convert('RGB').resize((1920, 1080))
    return (np.asarray(im.filter(ImageFilter.GaussianBlur(lo)), float)
            - np.asarray(im.filter(ImageFilter.GaussianBlur(hi)), float))


ref = band(args[0])
ladder = sorted((float(a.split('=', 1)[0]), a.split('=', 1)[1]) for a in args[1:])
rows = []
for k, p in ladder:
    o = band(p)
    rows.append([np.sqrt(((ref[d] - o[d]) ** 2).mean() / (ref[d] ** 2).mean()) for d in discs])
rows = np.array(rows)
print('k      ' + ' '.join(f'{n:>6}' for n in NAMES))
for (k, _), r in zip(ladder, rows):
    print(f'{k:5.3f}  ' + ' '.join(f'{v:6.2f}' for v in r))
print('best   ' + ' '.join(f'{ladder[i][0]:6.2f}' for i in rows.argmin(0)))
