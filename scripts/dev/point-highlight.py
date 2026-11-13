#!/usr/bin/env python3
"""Point-light highlight per sphere of the Calib-Point-* scenes (#114):
the peak, and width x height (px) of the run through the peak above 0.3 /
0.5 of it (luminance, gamma 2.2 decoded). Spheres S1 - S8 left to right
at Calib-Furnace-Metal's framing.

  python3 scripts/dev/point-highlight.py <render.png>...
"""
import sys
if any(a in ('-h', '--help') for a in sys.argv[1:]):
    print(__doc__)
    sys.exit(0)
import numpy as np
from PIL import Image

Y = np.array([0.2126, 0.7152, 0.0722])
XS = [424, 577, 730, 883, 1035, 1188, 1342, 1495]


def run(line, c):
    a = c
    while a > 0 and line[a - 1]:
        a -= 1
    b = c
    while b < len(line) - 1 and line[b + 1]:
        b += 1
    return b - a + 1


for p in sys.argv[1:]:
    L = (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2 @ Y
    out = []
    for k, cx in enumerate(XS):
        win = L[515:607, cx - 46:cx + 46]
        iy, ix = np.unravel_index(np.argmax(win), win.shape)
        pk = win[iy, ix]
        cells = []
        for lv in (0.3, 0.5):
            m = win > lv * pk
            cells.append(f'{run(m[iy], ix)}x{run(m[:, ix], iy)}')
        out.append(f'S{k + 1} {pk:.3f} ' + ' '.join(cells))
    print(p.split('/')[-1])
    print('  ' + ' | '.join(out))
