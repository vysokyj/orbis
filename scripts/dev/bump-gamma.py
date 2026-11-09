#!/usr/bin/env python3
"""Height-map decode gamma from a render of a Calib-BumpGamma-* scene (#91).

  python3 scripts/dev/bump-gamma.py [--gamma G] <render.png>...

Each card carries the synthetic map make-calib-scenes.py writes
(calib-bumpgamma*.png, 128 texels, bump factor 4; Uber2's third card 2): four
bands of 32 texels across u - a sawtooth rising 8 levels a texel along v,
the same falling, a 128 grey with a step to 160, a flat 128. The distant
light comes from the front 45 degrees up, in the plane of v, and nothing
else lights the cards (a black floor, Scene Only). Each sawtooth band
against the flat band of the same row:

    R = I_band / I_flat = (1 -+ x) / sqrt(1 + x^2),   x = f s tan 45 = f s

(Lambert; - for the rising band, tilted away from the light), solved for
x per texel row, whatever the albedo or exposure. MDL's
`file_bump_texture` takes s = h[j+1] - h[j-1] of the decoded heights:
a 1.0 decode gives f * 16 / 255 on every row of both bands, a 2.2 decode
f * ((8 (j+1) / 255)^2.2 - (8 (j-1) / 255)^2.2) at each band's own height
(rising h = 8 j, falling 8 (31 - j)): it grows as h^1.2. The two bands
differ in height row by row, so their ratio alone isn't f s at 2.2 (the
first version of this script compared it so and read 2.2 as "in
between"). Printed: x per row for both bands against both predictions,
and the fit-free verdict, their RMS. Normal-map cards (a tilt of +-0.25 in
the first two bands, flat in the others) get the tilted bands' contrast
and their flat band against the third card's against what a 1.0 and a
2.2 decode of the 8-bit texels predict (Lambert).

PNG renders are decoded at --gamma (default 2.2: DAZ's tone mapping with
burn 1 / crush 0, and the export's, are a pure 2.2 curve).
"""
import sys
if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import numpy as np
from PIL import Image

args = sys.argv[1:]
gamma = 2.2
if args[0] == '--gamma':
    gamma, args = float(args[1]), args[2:]

N, PERIOD, STEP = 128, 32, 8


# rows away from the sawtooth's wrap: its 248-level drop tilts j 0 / 31
# hard, and Iray's texture filtering spreads it over a few texels more than
# Octane's (Iray's falling band at j 3 - 4 read 30 % steep)
ROWS = np.arange(5, PERIOD - 5)


def slope(f, g, h):
    """f s at texel height index h (heights 8 h / 255) decoded at gamma g."""
    d = lambda k: (STEP * k / 255) ** g
    return f * (d(h + 1) - d(h - 1))


def tilt(R, sign):
    """x from R = (1 + sign x) / sqrt(1 + x^2), x in 0 .. 1.5."""
    xs = np.linspace(0, 1.5, 15001)
    Rs = (1 + sign * xs) / np.sqrt(1 + xs ** 2)
    o = np.argsort(Rs)
    return np.interp(R, Rs[o], xs[o])


LIGHT = np.array([0.0, np.sqrt(0.5), np.sqrt(0.5)])  # tangent space (u, v, n): from v up, 45 degrees


def normal_pred(g):
    """Lambert shading of the normal-map card's rising-tilt, falling-tilt and
    flat band, its texels decoded at gamma g as MDL's
    `tangent_space_normal_texture` reads them ((c - 0.5) 2, |z|)."""
    def shade(n):
        c = np.round((np.asarray(n) / np.linalg.norm(n) + 1) / 2 * 255) / 255
        d = 2 * c ** g - 1
        d[2] = abs(d[2])
        return max(0.0, d @ LIGHT / np.linalg.norm(d))
    up, down, flat = shade([0, 0.25, 1]), shade([0, -0.25, 1]), shade([0, 0, 1])
    return (up - down) / (up + down), flat / LIGHT[2]


def cards(lum):
    """Each card as the homography from its UV square to pixels. Cards are
    the grey runs (neither the black floor / background nor a white dome)
    on the row through the middle of the grey area, split where the run
    breaks; corners are the extremes of x + y and x - y of each card's
    grey pixels (u 0 left, v 0 at the bottom), which holds for the mild
    perspective of either view (the Iray Server rendered
    Calib-BumpGamma-NVIDIA from DAZ's Perspective View, not Camera-1)."""
    grey = (lum > 0.03) & (lum < 0.8)
    cols = grey.sum(0) > 20
    edges = np.nonzero(np.diff(np.concatenate([[0], cols.astype(int), [0]])))[0]
    out = []
    for x0, x1 in zip(edges[0::2], edges[1::2]):
        if x1 - x0 < 40:
            continue
        ys, xs = np.nonzero(grey[:, x0:x1])
        xs = xs + x0
        pick = lambda v: (xs[np.argmax(v)], ys[np.argmax(v)])
        tl, br, tr, bl = pick(-(xs + ys)), pick(xs + ys), pick(xs - ys), pick(ys - xs)
        src = [(0, 1), (1, 0), (1, 1), (0, 0)]
        A = []
        for (u, v), (x, y) in zip(src, (tl, br, tr, bl)):
            A += [[u, v, 1, 0, 0, 0, -x * u, -x * v, -x], [0, 0, 0, u, v, 1, -y * u, -y * v, -y]]
        H = np.linalg.svd(np.array(A, float))[2][-1].reshape(3, 3)
        out.append((H, x1 - x0))
    return out


def sample(lum, H, u, v):
    """lum at UV points (bilinear) through the card's homography."""
    p = H @ np.stack([u, v, np.ones_like(u)])
    x, y = p[0] / p[2], p[1] / p[2]
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = x - x0, y - y0
    g = lambda yy, xx: lum[np.clip(yy, 0, lum.shape[0] - 1), np.clip(xx, 0, lum.shape[1] - 1)]
    return (g(y0, x0) * (1 - fx) + g(y0, x0 + 1) * fx) * (1 - fy) + (g(y0 + 1, x0) * (1 - fx) + g(y0 + 1, x0 + 1) * fx) * fy


def band_rows(lum, H, b, inset=0.0):
    """Mean lum per texel row j (0 .. N-1, v up) over texels 32 b + 8 .. 32 b
    + 24 across u, a few points per texel along v (inset trims the row's
    edges)."""
    us = (32 * b + 8 + np.arange(16 * 4) / 4 + 0.125) / N
    out = np.empty(N)
    for j in range(N):
        vs = (j + np.linspace(0.2 + inset, 0.8 - inset, 4)) / N
        uu, vv = np.meshgrid(us, vs)
        out[j] = sample(lum, H, uu.ravel(), vv.ravel()).mean()
    return out


for path in args:
    a = np.asarray(Image.open(path).convert('RGB'), float) / 255
    lum = (a ** gamma) @ np.array([0.2126, 0.7152, 0.0722])
    name = path.rsplit('/', 1)[-1]
    is_uber2 = 'Uber2' in name
    print(f'== {name}')
    found = cards(lum)
    for k, (H, w) in enumerate(found):
        rise, fall, step, flat = (band_rows(lum, H, b) for b in range(4))
        js = np.arange(N) % PERIOD
        inner = (js > 2) & (js < 29)
        if is_uber2 and k < 2:
            # bands 0 / 1 tilt towards / away from the light; the flat band
            # against card 3's flat height band (no tilt either way)
            m = ((rise - fall) / (rise + fall))[inner].mean()
            ref = band_rows(lum, found[2][0], 3)[inner].mean()
            (r1, f1), (r2, f2) = normal_pred(1.0), normal_pred(2.2)
            print(f'card {k + 1} (normal map): r {m:.3f}, flat band / height card flat {flat[inner].mean() / ref:.3f}; '
                  f'1.0 decode {r1:.3f} / {f1:.3f}, 2.2 decode {r2:.3f} / {f2:.3f}')
            continue
        f = 2.0 if (is_uber2 and k == 2) else 4.0
        row = lambda v: np.array([v[js == j].mean() for j in ROWS])
        xr, xf = tilt(row(rise / flat), -1), tilt(row(fall / flat), 1)
        pr = {g: slope(f, g, ROWS) for g in (1.0, 2.2)}
        pf = {g: slope(f, g, PERIOD - 1 - ROWS) for g in (1.0, 2.2)}
        rms = lambda g: np.sqrt((np.concatenate([xr - pr[g], xf - pf[g]]) ** 2).mean())
        print(f'card {k + 1} (f {f:g}, {w} px wide): RMS of f s vs 1.0 decode {rms(1.0):.4f}, vs 2.2 decode {rms(2.2):.4f}')
        print('   j  rise   1.0   2.2   fall   1.0   2.2')
        for n, jj in enumerate(ROWS):
            if jj % 4 == 1 or jj == ROWS[-1]:
                print(f'  {jj:2d} {xr[n]:.3f} {pr[1.0][n]:.3f} {pr[2.2][n]:.3f}  {xf[n]:.3f} {pf[1.0][n]:.3f} {pf[2.2][n]:.3f}')
        print(f'   flat band {flat[inner].mean():.4f}, step band off the steps {step[inner].mean():.4f}')
