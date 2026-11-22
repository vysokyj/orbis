#!/usr/bin/env python3
"""ORBIS: per-cell light of the strand scenes' 4 x 4 grid (rows G - J,
columns 1 - 4 of make-strand-scenes.py, 1920 x 1080), so arc / pile
compares are measured one way (#221).

  python3 scripts/dev/strand-cells.py [--shares a/b,c/d] <render.png>...

Each cell is the mean linear luminance (sRGB decoded, Rec. 709 weights)
x 1000 over the strands' silhouette inside the cell's box: the first
image with an alpha channel (an Iray reference) gives the silhouette
(alpha > 0.5), else every pixel of the first image brighter than 2 / 255.
The same mask is used for every image, so the ratios compare like with
like. --shares prints column ratios per row (e.g. 1/2,3/4: the dual
lobe's share with Pixie opaque and Pixie whole on -Pile).
"""
import sys
import numpy as np
from PIL import Image

# Boxes around the cells (x0, x1, y0, y1), the -Arc / -Pile layout.
COLS = [(170, 390), (630, 840), (1080, 1290), (1530, 1750)]
ROWS = [('G', 60, 175), ('H', 245, 365), ('I', 480, 610), ('J', 830, 960)]


def linear(a):
    c = a[..., :3].astype(np.float64) / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    return lin @ np.array([0.2126, 0.7152, 0.0722])


def main(argv):
    shares = []
    if '--shares' in argv:
        i = argv.index('--shares')
        shares = [tuple(int(c) for c in s.split('/')) for s in argv[i + 1].split(',')]
        argv = argv[:i] + argv[i + 2:]
    if not argv:
        print(__doc__)
        return 2
    imgs = [(p, np.array(Image.open(p))) for p in argv]
    ref = next((a for _, a in imgs if a.ndim == 3 and a.shape[2] == 4), None)
    mask = ref[..., 3] > 127 if ref is not None else linear(imgs[0][1]) > (2 / 255) ** 2.2
    print('mask:', 'alpha of the first RGBA image' if ref is not None else 'first image > 2/255')
    for p, a in imgs:
        lum = linear(a)
        print(p)
        for r, y0, y1 in ROWS:
            vals = []
            for x0, x1 in COLS:
                m = mask[y0:y1, x0:x1]
                vals.append(1000 * lum[y0:y1, x0:x1][m].mean() if m.any() else float('nan'))
            line = f'  {r}: ' + ' '.join(f'{v:7.3f}' for v in vals)
            for s in shares:
                line += f'   {s[0]}/{s[1]} {vals[s[0] - 1] / vals[s[1] - 1]:.3f}'
            print(line)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
