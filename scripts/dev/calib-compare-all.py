#!/usr/bin/env python3
"""Compare Octane renders of the calibration scenes with their Iray
references, scene by scene:

  python3 scripts/dev/calib-compare-all.py [<octane dir>] > report.md

Iray: ~/Library/Caches/orbis/calib/iray/<scene>.png (scripts/dev/calib-render.py);
Octane: <octane dir>/<scene>[__<hypothesis>].png (scripts/dev/octane-render.sh,
default ~/Library/Caches/orbis/calib/octane). Both are decoded to linear with gamma
2.2 (Iray's Gamma 2.2 tone mapping, the export's Gamma 2.2 response) and
compared as Octane / Iray luminance per region:

- sphere scenes (Calib-PBRSkin, -Glossy, -SSS, -Tone-*, -DOF-*): each sphere,
  found in Calib-Glossy's Iray render (the spheres are red there), and the
  floor in front;
- light scenes (Calib-L-*): floor front / left / right, the cubes, the
  sphere; spot profiles (Calib-L-Spot-s*): the radial profile, normalised;
- anything else: the whole frame.

Writes <octane dir>/compare/<scene>.png (Iray | Octane) beside the report.
"""
import glob, math, os, sys
import numpy as np
from PIL import Image, ImageDraw

home = os.path.expanduser('~/Library/Caches/orbis/calib')
iray_dir = os.path.join(home, 'iray')
oct_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(home, 'octane')
out_dir = os.path.join(oct_dir, 'compare')
os.makedirs(out_dir, exist_ok=True)
Y = np.array([0.2126, 0.7152, 0.0722])


def load(p):
    return np.asarray(Image.open(p).convert('RGB')).astype(float) / 255


def lin(a):
    return a ** 2.2


def sphere_boxes():
    """Bounding boxes of the eight spheres, left to right, from the red
    spheres of Calib-Glossy's Iray render."""
    a = load(os.path.join(iray_dir, 'Calib-Glossy.png'))
    red = (a[..., 0] > 1.6 * a[..., 1]) & (a[..., 0] > 0.12)
    cols = np.flatnonzero(red.any(0))
    groups, start = [], cols[0]
    for c0, c1 in zip(cols[:-1], cols[1:]):
        if c1 - c0 > 5:
            groups.append((start, c0)); start = c1
    groups.append((start, cols[-1]))
    boxes = []
    for x0, x1 in groups:
        rows = np.flatnonzero(red[:, x0:x1 + 1].any(1))
        # The middle of the sphere (avoid the rim and the floor contact).
        cx, cy, rx, ry = (x0 + x1) / 2, (rows[0] + rows[-1]) / 2, (x1 - x0) / 2, (rows[-1] - rows[0]) / 2
        boxes.append((int(cy - ry * .6), int(cy + ry * .6), int(cx - rx * .6), int(cx + rx * .6)))
    return boxes


LIGHT_REGIONS = {'floor front': (880, 980, 860, 1060), 'floor left': (880, 980, 300, 500),
                 'floor right': (880, 980, 1420, 1620)}


def ratio(o, i, box):
    y0, y1, x0, x1 = box
    a, b = lin(o[y0:y1, x0:x1]) @ Y, lin(i[y0:y1, x0:x1]) @ Y
    ok = (i[y0:y1, x0:x1].max(-1) < 0.99) & (o[y0:y1, x0:x1].max(-1) < 0.99)
    if ok.sum() < 20 or b[ok].mean() <= 1e-6:
        return None
    return a[ok].mean() / b[ok].mean()


def spot_profile(a):
    E = lin(a) @ Y
    h, w = E.shape
    cmpp = 2 * 520 * math.tan(math.atan(25 / 65)) / h
    yy, xx = np.mgrid[0:h, 0:w]
    r = np.hypot(xx - w / 2 + .5, yy - h / 2 + .5) * cmpp
    th = np.degrees(np.arctan(r / 250))
    I = E * (250 ** 2 + r ** 2) ** 1.5 / 250
    out = []
    for t in range(0, 40, 2):
        m = (th >= t) & (th < t + 2) & (r < 195)
        out.append(I[m].mean() if m.sum() > 20 else np.nan)
    out = np.array(out)
    return out / np.nanmax(out[:2])


boxes = sphere_boxes() if os.path.exists(os.path.join(iray_dir, 'Calib-Glossy.png')) else []
rows = []
# Octane renders, variants (`<scene>__<hypothesis>`, scripts/dev/orbx-variants.py)
# against their scene's Iray reference.
for opath in sorted(glob.glob(os.path.join(oct_dir, 'Calib-*.png'))):
    name = os.path.basename(opath)[:-4]
    ipath = os.path.join(iray_dir, name.split('__')[0] + '.png')
    if not os.path.exists(ipath):
        continue
    i, o = load(ipath), load(opath)
    if o.shape != i.shape:
        o = np.asarray(Image.fromarray((o * 255).astype(np.uint8)).resize(i.shape[1::-1])).astype(float) / 255
    cells = []
    if name.startswith('Calib-L-Spot-s'):
        pi, po = spot_profile(i), spot_profile(o)
        cells.append('profile Octane / Iray by 2° from the axis: ' + ' '.join(
            f'{a / b:.2f}' if b > 0.02 and not math.isnan(a) else '·' for a, b in zip(po, pi)))
    elif name.startswith('Calib-L-'):
        for k, box in LIGHT_REGIONS.items():
            r = ratio(o, i, box)
            cells.append(f'{k} {r:.2f}' if r else f'{k} –')
    elif boxes and any(name.startswith(p) for p in ('Calib-PBRSkin', 'Calib-Glossy', 'Calib-Tone', 'Calib-DOF', 'Calib-SSS', 'Calib-Glass', 'Calib-Mixing', 'Calib-NVIDIA')):
        rs = [ratio(o, i, b) for b in boxes]
        cells.append('spheres ' + ' '.join(f'{r:.2f}' if r else '–' for r in rs))
        fr = ratio(o, i, (boxes[0][1] + 60, boxes[0][1] + 140, 700, 1220))
        cells.append(f'floor {fr:.2f}' if fr else 'floor –')
    else:
        r = ratio(o, i, (0, i.shape[0], 0, i.shape[1]))
        cells.append(f'frame {r:.2f}' if r else 'frame –')
    rows.append((name, cells))
    w = 960
    a = Image.fromarray((i * 255).astype(np.uint8)).resize((w, w * i.shape[0] // i.shape[1]))
    b = Image.fromarray((o * 255).astype(np.uint8)).resize(a.size)
    sheet = Image.new('RGB', (2 * w, a.size[1] + 18), 'white')
    sheet.paste(a, (0, 18)); sheet.paste(b, (w, 18))
    d = ImageDraw.Draw(sheet)
    d.text((6, 3), f'{name}: Iray', fill='black'); d.text((w + 6, 3), 'Octane (ORBIS)', fill='black')
    sheet.save(os.path.join(out_dir, name + '.png'))

print('# Calibration: Octane / Iray (linear luminance)\n')
print(f'{len(rows)} scenes with both renders; 1.00 = equal. Side by side: `{out_dir}`.\n')
print('| scene | Octane / Iray |\n|---|---|')
for name, cells in rows:
    print(f'| {name} | {"; ".join(cells)} |')
