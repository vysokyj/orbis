#!/usr/bin/env python3
"""Skin texture contrast of Octane renders against Iray, Calib-Skin fronts
(#47, #179; docs/notes/2026-11-12-skin-texture-contrast.md).

  python3 scripts/dev/skin-texture.py <octane dir> [<octane dir>...] -- <scene>...

Log luminance (gamma 2.2 decode), band-passed with Gaussians: fine = L -
G(1.2 px) (pores), mid = G(1.2) - G(4) (fine wrinkles, pore clusters). The
standard deviation of each band in forehead, cheeks, chest and shoulder
(skin-compare.py's regions of the 960 x 720 framing; the neck holds the
jaw's shadow edge and is left out), Octane over Iray; per scene the median
of the five regions, and the face (forehead + cheeks) median apart. 1.00 =
Iray's texture. Octane is read in its own pass (octane_pass.py: the
`denoised/` twin when there is one). Iray: ~/Library/Caches/orbis/calib/iray/.
"""
import sys, os
if any(a in ('-h', '--help') for a in sys.argv[1:]) or '--' not in sys.argv:
    print(__doc__)
    sys.exit(0)
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import octane_pass

IRAY = os.path.expanduser('~/Library/Caches/orbis/calib/iray')
FACE = {'forehead': (455, 120, 505, 150), 'cheek L': (405, 270, 435, 300), 'cheek R': (525, 270, 555, 300)}
BODY = {'chest': (420, 520, 540, 580), 'shoulder': (250, 480, 310, 520)}


def gaussian_filter(y, sigma):
    r = int(3 * sigma + 0.5)
    k = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
    k /= k.sum()
    p = np.pad(y, r, mode='reflect')
    p = np.apply_along_axis(lambda v: np.convolve(v, k, 'valid'), 1, p)
    return np.apply_along_axis(lambda v: np.convolve(v, k, 'valid'), 0, p)


def bands(png):
    a = np.asarray(Image.open(png).convert('RGB'), float) / 255
    y = np.log(np.maximum((a ** 2.2) @ [0.2126, 0.7152, 0.0722], 1e-4))
    g1, g4 = gaussian_filter(y, 1.2), gaussian_filter(y, 4.0)
    return y - g1, g1 - g4


def sd(b, box):
    x0, y0, x1, y1 = box
    return b[y0:y1, x0:x1].std()


i = sys.argv.index('--')
dirs, scenes = sys.argv[1:i], sys.argv[i + 1:]
print('| scene | ' + ' | '.join(f'{os.path.basename(d.rstrip("/"))} fine / mid (face fine / mid)' for d in dirs) + ' |')
print('|---|' + '---|' * len(dirs))
for s in scenes:
    ref = bands(os.path.join(IRAY, s + '.png'))
    row = []
    for d in dirs:
        png, kind = octane_pass.pick(os.path.join(d, s + '.png'))
        o = bands(png)
        ratio = lambda k, regs: [sd(o[k], b) / sd(ref[k], b) for b in regs.values()]
        allr = {**FACE, **BODY}
        row.append(f'{np.median(ratio(0, allr)):.2f} / {np.median(ratio(1, allr)):.2f} ({np.median(ratio(0, FACE)):.2f} / {np.median(ratio(1, FACE)):.2f}){"" if kind.startswith("denoised") else " " + kind}')
    print(f'| {s} | ' + ' | '.join(row) + ' |')
