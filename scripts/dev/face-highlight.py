#!/usr/bin/env python3
"""The Calib-Skin faces' highlight: p99.5 of the face box, Octane / Iray (#151).

  python3 scripts/dev/face-highlight.py <octane dir> [<scene>...]
  python3 scripts/dev/face-highlight.py --sub <minus dir> <octane dir> [<scene>...]
  python3 scripts/dev/face-highlight.py --blur [--sub <minus dir>] <octane dir> ...

Per Calib-Skin-<figure>-<light>[-variant] render in <octane dir>: linear
luminance (gamma 2.2 decode, skin-compare.py's), the 99.5th percentile over
the face box x 380 - 580, y 90 - 340 of the 960 x 720 render (the box of
the spec's numbers, 2026-11-09 note), Octane over its Iray reference
(~/Library/Caches/orbis/calib/iray/<scene>.png). Octane is read on both
passes, so one render says what the denoiser moves: `raw` (the beauty) and
`denoised` (the `denoised/` twin, when there is one); `pass` is what
octane_pass.py's rule reads for this directory (#139).

--sub <minus dir>: the dual lobe's own share as the spec measures it,
p99.5 of (render - its gloss0 twin), e.g. dlflat - gloss0: each scene
<X>-dlflat in <octane dir> minus <X>-gloss0 in <minus dir> (the same
directory is fine), the Iray side alike; the twins without bump or normal
map pair the same way (<X>-dlflatnb - <X>-gloss0nb, -dlflatnn - -gloss0nn).

--blur: the p99.5 after a 2-pass 5-tap binomial blur (sigma ~1.4 px) of
the image (or the difference): the grain-robust number - raw and denoised,
400 and 2000 samples agree within ~3 % on it, where a pixel p99.5 over a
raw 400-sample face reads the random walk's grain (#151).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import glob, os, sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import octane_pass

IRAY = os.path.expanduser('~/Library/Caches/orbis/calib/iray')
Y = np.array([0.2126, 0.7152, 0.0722])
BOX = (380, 90, 580, 340)  # x0, y0, x1, y1
BLUR = False


def lum(p):
    return ((np.asarray(Image.open(p).convert('RGB'), float) / 255) ** 2.2) @ Y


def blur(x):
    k = np.array([1, 4, 6, 4, 1], float) / 16
    for _ in range(2):
        x = np.apply_along_axis(lambda r: np.convolve(r, k, 'same'), 1, x)
        x = np.apply_along_axis(lambda c: np.convolve(c, k, 'same'), 0, x)
    return x


def p995(a):
    """p99.5 over the face box of a full-frame luminance image."""
    if BLUR:
        a = blur(a)
    x0, y0, x1, y1 = BOX
    return float(np.percentile(a[y0:y1, x0:x1], 99.5))


def passes(png):
    """{'raw': path, 'denoised': path or None}."""
    den = octane_pass.twin(png)
    return {'raw': png, 'denoised': den if os.path.exists(den) else None}


def main():
    global BLUR
    args = sys.argv[1:]
    if args[:1] == ['--blur']:
        BLUR, args = True, args[1:]
    minus = None
    if args[:1] == ['--sub']:
        minus, args = args[1], args[2:]
    odir, want = args[0], args[1:]
    rows = []
    for p in sorted(glob.glob(os.path.join(odir, 'Calib-Skin-*.png'))):
        scene = os.path.basename(p)[:-4]
        if want and not any(w in scene for w in want):
            continue
        if minus is not None:
            head, _, var = scene.rpartition('-')
            if not var.startswith('dlflat'):
                continue
            mscene = head + '-gloss0' + var[len('dlflat'):]
            mp = os.path.join(minus, mscene + '.png')
            ir, mir = os.path.join(IRAY, scene + '.png'), os.path.join(IRAY, mscene + '.png')
            if not (os.path.exists(mp) and os.path.exists(ir) and os.path.exists(mir)):
                continue
            iv = p995(lum(ir) - lum(mir))
            pa, pm = passes(p), passes(mp)
            raw = p995(lum(pa['raw']) - lum(pm['raw'])) / iv
            den = (p995(lum(pa['denoised']) - lum(pm['denoised'])) / iv
                   if pa['denoised'] and pm['denoised'] else None)
        else:
            ir = os.path.join(IRAY, scene + '.png')
            if not os.path.exists(ir):
                continue
            iv = p995(lum(ir))
            pa = passes(p)
            raw = p995(lum(pa['raw'])) / iv
            den = p995(lum(pa['denoised'])) / iv if pa['denoised'] else None
        rows.append((scene, iv, raw, den, octane_pass.short(octane_pass.pick(p)[1])))
    what = 'dlflat - gloss0' if minus is not None else 'face'
    what += ', blurred' if BLUR else ''
    print(f'p99.5 of the {what} box {BOX}, Octane / Iray (Iray: {IRAY})\n')
    print('| scene | Iray p99.5 | raw | denoised | denoised / raw | pass |')
    print('|---|---|---|---|---|---|')
    for scene, iv, raw, den, lb in rows:
        d = f'{den:.3f}' if den is not None else '-'
        r = f'{den / raw:.3f}' if den is not None else '-'
        print(f'| {scene} | {iv:.4f} | {raw:.3f} | {d} | {r} | {lb} |')


if __name__ == '__main__':
    main()
