#!/usr/bin/env python3
"""Point-light highlight per sphere of the Calib-Point-* scenes (#114):
the peak, and width x height (px) of the run through the peak above 0.3 /
0.5 of it (luminance, gamma 2.2 decoded). Spheres left to right, projected
from the render's scene (scripts/dev/sphere_discs.py, #146; printed first).

  python3 scripts/dev/point-highlight.py <render.png>...

Each render is read in the export's own pass (scripts/dev/octane_pass.py:
the `denoised/` twin when the variant denoises, #139), named in the output.
"""
import sys
if any(a in ('-h', '--help') for a in sys.argv[1:]):
    print(__doc__)
    sys.exit(0)
import os
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import octane_pass
import sphere_discs

Y = np.array([0.2126, 0.7152, 0.0722])


def run(line, c):
    a = c
    while a > 0 and line[a - 1]:
        a -= 1
    b = c
    while b < len(line) - 1 and line[b + 1]:
        b += 1
    return b - a + 1


for p in sys.argv[1:]:
    src, label = octane_pass.pick(p)
    L = (np.asarray(Image.open(src).convert('RGB')).astype(float) / 255) ** 2.2 @ Y
    discs, dsrc = sphere_discs.discs(p, L.shape)
    out = []
    for k, d in enumerate(discs):
        win = L[max(int(d.cy - d.ry), 0):int(d.cy + d.ry) + 1, max(int(d.cx - d.rx), 0):int(d.cx + d.rx) + 1]
        iy, ix = np.unravel_index(np.argmax(win), win.shape)
        pk = win[iy, ix]
        cells = []
        for lv in (0.3, 0.5):
            m = win > lv * pk
            cells.append(f'{run(m[iy], ix)}x{run(m[:, ix], iy)}')
        out.append(f'S{k + 1} {pk:.3f} ' + ' '.join(cells))
    print(f"{p.split('/')[-1]} [{label}]")
    print('  ' + sphere_discs.describe(discs, dsrc))
    print('  ' + ' | '.join(out))
