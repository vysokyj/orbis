#!/usr/bin/env python3
"""Compare Octane renders of the Calib-Skin scenes (#19) with their Iray
references, region by region: luminance and hue.

  python3 scripts/dev/skin-compare.py <octane dir> [<scene>...] > report.md

Iray: ~/Library/Caches/orbis/calib/iray/<scene>.png; Octane:
<octane dir>/<scene>[__<variant>].png (orbx-variants.py names). Per region
the median pixel, linearised (gamma 2.2): Y = Octane / Iray luminance,
r/g and b/g = Octane's channel ratio over Iray's (1.00 = same hue; b/g
above 1 = bluer than Iray). The grey sphere checks the light, the skin
sphere is the Torso material on a prop.
"""
import glob, os, sys
import numpy as np
from PIL import Image

IRAY = os.path.expanduser('~/Library/Caches/orbis/calib/iray')
Y = np.array([0.2126, 0.7152, 0.0722])
# x0, y0, x1, y1 in the 960 x 720 render (make-skin-scenes.py's camera)
REGIONS = {
    'forehead': (455, 120, 505, 150),
    'cheek L': (405, 270, 435, 300),
    'cheek R': (525, 270, 555, 300),
    'nose': (472, 235, 488, 255),
    'ear L': (372, 200, 382, 235),
    'ear R': (578, 200, 588, 235),
    'neck': (455, 390, 505, 430),
    'chest': (420, 520, 540, 580),
    'shoulder': (250, 480, 310, 520),
    'skin sphere': (780, 335, 830, 385),
    'grey sphere': (130, 335, 180, 385),
}


def lin(p):
    return (np.asarray(Image.open(p).convert('RGB'), float) / 255) ** 2.2


def med(a, box):
    x0, y0, x1, y1 = box
    return np.median(a[y0:y1, x0:x1].reshape(-1, 3), 0)


def compare(octane_png, iray_png):
    o, i = lin(octane_png), lin(iray_png)
    out = {}
    for k, box in REGIONS.items():
        mo, mi = med(o, box), med(i, box)
        if mi @ Y < 1e-4:
            continue
        g = lambda m, c: m[c] / max(m[1], 1e-6)
        out[k] = (float(mo @ Y / (mi @ Y)), g(mo, 0) / max(g(mi, 0), 1e-6), g(mo, 2) / max(g(mi, 2), 1e-6), mi)
    return out


def main():
    odir = sys.argv[1]
    want = sys.argv[2:]
    pngs = sorted(glob.glob(os.path.join(odir, 'Calib-Skin-*.png')))
    print('| scene | region | Y | r/g | b/g | Iray linear rgb |')
    print('|---|---|---|---|---|---|')
    for p in pngs:
        name = os.path.basename(p)[:-4]
        scene = name.split('__')[0]
        if want and not any(w in name for w in want):
            continue
        ref = os.path.join(IRAY, scene + '.png')
        if not os.path.exists(ref):
            continue
        for k, (y, rg, bg, mi) in compare(p, ref).items():
            print(f'| {name} | {k} | {y:.2f} | {rg:.2f} | {bg:.2f} | {mi[0]:.3f} {mi[1]:.3f} {mi[2]:.3f} |')


if __name__ == '__main__':
    main()
