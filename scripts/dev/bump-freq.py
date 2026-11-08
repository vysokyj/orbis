#!/usr/bin/env python3
"""How Octane turns a height map (or a normal map) into shading normals,
texel by texel (#85). The probe scripts/dev/octane-probe-bumpfreq.lua
renders a flat ground plane's shading normal (Info kernel, one point
sample a pixel) without and with bump mapping, and its UVs.

  python3 scripts/dev/bump-freq.py make <out dir>
      writes stripes.png (16-bit height: sine stripes along u, periods
      2 3 4 6 8 12 24 48 texels in bands of 48, the same slope amplitude)
      and stripes-n.png (the same as MDL's file_bump_texture would tilt it
      at factor 4, as a tangent-space normal map)
  python3 scripts/dev/bump-freq.py <probe dir> [height|normal] [tiles]
      <probe dir> holds orbis_bf_{flat,bump,uv}.png of the plane with the
      map on its `bump` (height) or `normal` pin, the pattern repeated
      `tiles` times along u

Put the map in an export's package (examples/orbx_ocs `put`) and on the
ground material's pin. Rows of the image are lines of constant v on the
plane, so u is linear along a row: fitted per row from the 8-bit UV pass.
The slope along u is -n_x / n_y (exact at any tilt). Prints, per band, the
RMS slope and the share of samples with no tilt at all, and how well a
bilinear (smooth) or a per-texel (stepped) height explains the samples.
"""
import sys
if len(sys.argv) < 3 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import os
import numpy as np
from PIL import Image

PERIODS = [2, 3, 4, 6, 8, 12, 24, 48]
BAND = 48


def stripes():
    h = np.zeros(BAND * len(PERIODS))
    x = np.arange(BAND)
    for k, p in enumerate(PERIODS):
        # Phase pi/4: period 2 doesn't sit on its zero crossings.
        h[k * BAND:(k + 1) * BAND] = 0.5 + 0.45 * p / 48 * np.sin(2 * np.pi * x / p + np.pi / 4)
    return h


if sys.argv[1] == 'make':
    out = sys.argv[2]
    h = stripes()
    w = len(h)
    Image.fromarray(np.round(np.tile(h, (w, 1)) * 65535).astype(np.uint16)).save(os.path.join(out, 'stripes.png'))
    d = np.roll(h, -1) - np.roll(h, 1)
    n = np.stack([-4 * d, np.zeros_like(d), np.ones_like(d)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    Image.fromarray(np.tile(((n + 1) / 2 * 255).round().astype(np.uint8)[None], (w, 1, 1))).save(os.path.join(out, 'stripes-n.png'))
    sys.exit(0)

d = sys.argv[1]
kind = sys.argv[2] if len(sys.argv) > 2 else 'height'
tiles = int(sys.argv[3]) if len(sys.argv) > 3 else 1
h = np.tile(stripes(), tiles)
W = len(h)


def load(name):
    return np.asarray(Image.open(os.path.join(d, f'orbis_bf_{name}.png')).convert('RGB')).astype(float) / 255


flat, bump, uv = load('flat') * 2 - 1, load('bump') * 2 - 1, load('uv')
T, S, P = [], [], []
for y in range(flat.shape[0]):
    xs = np.nonzero(flat[y, :, 1] > 0.98)[0]
    if len(xs) < 200:
        continue
    xs = xs[(xs > xs.min() + 5) & (xs < xs.max() - 5)]
    a = np.polyfit(xs, uv[y, xs, 0], 1)
    n = bump[y, xs]
    T.append(np.polyval(a, xs) * W)
    S.append(-n[:, 0] / n[:, 1])
    P.append(np.full(len(xs), 1 / (a[0] * W)))
t, s, px = (np.concatenate(v) for v in (T, S, P))
ok = (t >= 0) & (t < W)
t, s, px = t[ok], s[ok], px[ok]
print(f'{len(t)} samples, {px.min():.1f} - {px.max():.1f} px per texel')
band = (np.mod(np.floor(t), BAND * len(PERIODS)) // BAND).astype(int)
H = lambda i: h[np.mod(i, W)]
# Texel centres at (i + 0.5) / W: a bilinear height's slope within
# [i + 0.5, i + 1.5) is h[i+1] - h[i].
c = t - 0.5
i = np.floor(c).astype(int)
smooth = H(i + 1) - H(i)
if kind == 'normal':
    # The map's own n_x, bilinear; slope = -n_x / n_z.
    dd = np.roll(h, -1) - np.roll(h, 1)
    nx = -4 * dd / np.sqrt(1 + 16 * dd ** 2)
    f = c - i
    nxi = (1 - f) * nx[np.mod(i, W)] + f * nx[np.mod(i + 1, W)]
    smooth = -nxi / np.sqrt(1 - nxi ** 2)
zero = np.abs(s) < 1.5 / 255
print('band (period)   ' + ' '.join(f'{p:>6}' for p in PERIODS))
print('RMS slope       ' + ' '.join(f'{s[band == k].std():6.3f}' for k in range(len(PERIODS))))
print('share untilted  ' + ' '.join(f'{zero[band == k].mean():6.2f}' for k in range(len(PERIODS))))
r = [np.corrcoef(smooth[band == k], s[band == k])[0, 1] for k in range(len(PERIODS))]
print('corr. smooth    ' + ' '.join(f'{x:6.2f}' for x in r))
