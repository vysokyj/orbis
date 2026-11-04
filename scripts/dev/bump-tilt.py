#!/usr/bin/env python3
"""How far a bump tilts the normal, per sphere (#75): the angle between two
Info-kernel shading-normal renders (scripts/dev/octane-probe-bumptilt.lua),
without and with bump mapping, RMS over each sphere's inner disc (radius
0.7, to keep the grazing rim out).

  python3 scripts/dev/bump-tilt.py <flat.png> <bump.png> [label]

Spheres: the connected runs of columns whose normals differ from the
ground's (straight up), left to right. Prints one RMS tilt (degrees) each.
"""
import sys
if len(sys.argv) < 3 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import numpy as np
from PIL import Image


def normals(p):
    a = np.asarray(Image.open(p).convert('RGB'), float) / 255 * 2 - 1
    ln = np.linalg.norm(a, axis=-1, keepdims=True)
    return a / np.maximum(ln, 1e-6), ln[..., 0]


(n0, l0), (n1, _) = normals(sys.argv[1]), normals(sys.argv[2])
up = np.array([0.0, 1.0, 0.0])
# The ground's normal is straight up, the background has none (grey):
# spheres are where a normal leans away from up.
obj = (n0 @ up < 0.97) & (l0 > 0.5)
cols = obj.sum(0) > 6
spheres, x = [], 0
while x < len(cols):
    if cols[x]:
        x0 = x
        while x < len(cols) and cols[x]:
            x += 1
        if x - x0 > 10:
            ys = np.nonzero(obj[:, x0:x].sum(1) > 2)[0]
            spheres.append(((x0 + x - 1) / 2, (ys.min() + ys.max()) / 2, (x - x0) / 2))
    x += 1
yy, xx = np.mgrid[0:n0.shape[0], 0:n0.shape[1]]
ang = np.degrees(np.arccos(np.clip((n0 * n1).sum(-1), -1, 1)))
out = []
for cx, cy, r in spheres:
    m = (xx - cx) ** 2 + (yy - cy) ** 2 < (0.7 * r) ** 2
    out.append(np.sqrt((ang[m] ** 2).mean()))
print('spheres (x, y, r):', [(round(a), round(b), round(c)) for a, b, c in spheres], file=sys.stderr)
print(f'{sys.argv[3] if len(sys.argv) > 3 else "tilt":10}', ' '.join(f'{t:6.2f}' for t in out))
