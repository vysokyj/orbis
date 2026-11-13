#!/usr/bin/env python3
"""Where the spheres of a calibration scene are in its renders (#146).

  python3 scripts/dev/sphere_discs.py [--size WxH] <scene | scene.duf | render.png>...
      Prints, per scene, the discs (label, centre, radius in px) and their
      source.
  python3 scripts/dev/sphere_discs.py --check <matte render.png>...
      The projected discs against the ones detected in a matte render of
      the same scene (red spheres, gloss-split.py's old finder): the
      difference per sphere, px.

Imported by the compare scripts (calib-compare-all.py, highlight-spread.py,
gloss-split.py, furnace-rings.py, point-highlight.py, refr-volume-mc.py
--measure), as octane_pass.py is:

  ds, source = sphere_discs.discs(scene_or_png, shape=img.shape)
  m = sphere_discs.mask(d, shape, 0.9)            # pixels inside 0.9 R
  print(sphere_discs.describe(ds, source))         # the report's line

Source of truth is the scene's geometry, not image analysis: the spheres
are projected through the scene's camera.

- spheres: every node of the scene's dump (`~/Library/Caches/orbis/dumps`,
  the one written for this scene) whose world-space vertices lie on a
  sphere (50 vertices or more, distance to their centroid within 2 % of
  its mean, equal extents along x, y and z), in front of
  the camera and not around it. Without a dump: the .duf's own primitives
  (`geometry_library` vertices placed by the node definition's
  translation / scale; rotation and parents are left out, `source` says
  `duf`).
- camera: the .duf's `current_camera` (else its first camera), as
  src/usd/camera.rs reads it: translation, Euler rotation (`XYZ` = Rz Ry
  Rx), Focal Length, Frame Width fitted to the image height, the render
  size from the render settings. Lens shift / distortion are not modelled
  (a scene that uses them is flagged).
- each sphere's disc is the bounding box of its projected vertices: an
  off-axis sphere projects as an ellipse longer towards the frame's edge
  (rx > ry by ~8 % at Calib-Glossy's outer spheres); `r` is the smaller
  half-axis, and mask() uses both. Pixel-index convention: pixel (i, j)'s
  centre is at (i, j), as np.mgrid gives it.
- order: rows top to bottom, each left to right.

No scene geometry (no .duf, no dump, no sphere in it): the discs are
detected in a matte render given by `matte=` (red spheres, R > 2.5 G),
source `matte <file>`; never in a lit glossy render, whose brightest region
is the sun's highlight, not the sphere (#137, #146).

The .duf is looked up in calib/scenes, then in ORBIS_LIBRARY (default
"/Volumes/Data/DAZ 3D/Studio/My Library")/Scenes; the dump by the scene
path it records. A render is mapped to its scene by its name
(`<scene>[__<hypothesis>].png`).
"""
import glob
import gzip
import json
import math
import os
import re
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
SCENES = os.path.join(REPO, 'calib', 'scenes')
LIBRARY = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')
DUMPS = os.path.expanduser('~/Library/Caches/orbis/dumps')
SPHERE_TOL = 0.02
_cache = {}


class Disc:
    __slots__ = ('label', 'cx', 'cy', 'rx', 'ry')

    def __init__(self, label, cx, cy, rx, ry):
        self.label, self.cx, self.cy, self.rx, self.ry = label, cx, cy, rx, ry

    @property
    def r(self):
        return min(self.rx, self.ry)

    def scaled(self, k):
        return Disc(self.label, (self.cx + .5) * k - .5, (self.cy + .5) * k - .5, self.rx * k, self.ry * k)

    def __iter__(self):  # (cx, cy, r), the old finders' tuple
        return iter((self.cx, self.cy, self.r))

    def __repr__(self):
        return f'{self.label} ({self.cx:.1f}, {self.cy:.1f}) R {self.rx:.1f} x {self.ry:.1f}'


def scene_name(arg):
    b = os.path.basename(arg)
    for ext in ('.png', '.duf', '.exr', '.jpg'):
        if b.endswith(ext):
            b = b[:-len(ext)]
    return b.split('__')[0]


def load_duf(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def find_duf(name):
    for d in (SCENES, os.path.join(LIBRARY, 'Scenes')):
        p = os.path.join(d, name + '.duf')
        if os.path.exists(p):
            return p
    return None


def find_dump(name):
    """The newest dump written for <name>.duf (dumps are named <stem>-<hash>.json)."""
    best = None
    for p in glob.glob(os.path.join(DUMPS, glob.escape(name) + '-*.json')):
        if not re.fullmatch(r'[0-9a-f]{8}', p[len(DUMPS) + 1 + len(name) + 1:-5]):
            continue  # <name>-<suffix>-<hash>: another scene
        if best is None or os.path.getmtime(p) > os.path.getmtime(best):
            best = p
    return best


def _ch3(node, key, default, base=None):
    out = list(base) if base else [default] * 3
    for c in node.get(key) or []:
        i = 'xyz'.find(c.get('id', '?'))
        if i >= 0:
            out[i] = c.get('current_value', c.get('value', default))
    return out


def _channels(node):
    out = {}
    for e in node.get('extra') or []:
        for c in e.get('channels') or []:
            ch = c.get('channel') or {}
            if 'id' in ch:
                out[ch['id']] = ch.get('current_value', ch.get('value'))
    return out


def _euler(deg, order):
    m = np.eye(3)
    for axis in order:
        i = 'XYZ'.find(axis)
        if i < 0:
            continue
        s, c = math.sin(math.radians(deg[i])), math.cos(math.radians(deg[i]))
        r = [np.array([[1, 0, 0], [0, c, -s], [0, s, c]]), np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]]),
             np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])][i]
        m = r @ m
    return m


def render_size(duf):
    for e in duf.get('scene', {}).get('extra') or []:
        if e.get('type') == 'studio_render_settings':
            s = (e.get('render_options') or {}).get('imageSize')
            if s and s[0] > 0 and s[1] > 0:
                return int(s[0]), int(s[1])
    return 1920, 1080


def camera(duf):
    """(position cm, rotation 3x3, focal mm, frame height mm, (w, h), notes)
    of the scene's current camera, as src/usd/camera.rs reads it."""
    lib = {n['id']: n for n in duf.get('node_library', [])}
    nodes = duf.get('scene', {}).get('nodes', [])
    cur = (duf.get('scene', {}).get('current_camera') or '').lstrip('#')

    def defn(n):
        u = n.get('url', '')
        return lib.get(u.lstrip('#').split('?')[0].replace('%20', ' ').replace('%28', '(').replace('%29', ')')) \
            if u.startswith('#') else None
    cams = [n for n in nodes if (defn(n) or {}).get('type') == 'camera']
    node = next((n for n in cams if n['id'] == cur), cams[0] if cams else None)
    if node is None:
        return None
    d = defn(node) or {}
    t = _ch3(node, 'translation', 0.0, _ch3(d, 'translation', 0.0))
    r = _ch3(node, 'rotation', 0.0, _ch3(d, 'rotation', 0.0))
    rot = _euler(r, d.get('rotation_order') or node.get('rotation_order') or 'XYZ')
    ch = _channels(d)
    ch.update(_channels(node))
    persp = node.get('perspective') or d.get('perspective') or {}
    focal = ch.get('Focal Length', persp.get('focal_length', 65.0))
    frame = ch.get('Frame Width') or 36.0
    notes = []
    if node.get('parent') or d.get('parent'):
        notes.append('camera parented (parent ignored)')
    for k in ('Lens Shift X', 'Lens Shift Y', 'Lens Stereo Offset', 'Lens Distortion Type'):
        if ch.get(k):
            notes.append(f'{k} {ch[k]} not modelled')
    return np.array(t, float), rot, float(focal), float(frame), render_size(duf), notes


def _dump_spheres(path):
    d = json.load(open(path))
    out = []
    for n in d.get('nodes', []):
        v = np.asarray(n.get('vertices') or [], float).reshape(-1, 3)
        if len(v) < 50 or n.get('polylines') or n.get('polyline_counts'):
            continue
        out.append((n.get('label') or n.get('name'), v))
    return out


def _duf_spheres(duf):
    geo = {g['id']: np.asarray(g['vertices']['values'], float) for g in duf.get('geometry_library', [])
           if g.get('vertices', {}).get('values')}
    lib = {n['id']: n for n in duf.get('node_library', [])}
    out = []
    for n in duf.get('scene', {}).get('nodes', []):
        g = (n.get('geometries') or [{}])[0]
        gid = (g.get('url') or '').lstrip('#').replace('%28', '(').replace('%29', ')')
        if gid not in geo:
            continue
        d = lib.get(n.get('url', '').lstrip('#').replace('%28', '(').replace('%29', ')'), {})
        t = np.array(_ch3(d, 'translation', 0.0), float)
        s = np.array(_ch3(d, 'scale', 1.0), float) * (d.get('general_scale') or {}).get('current_value', 1.0)
        out.append((n.get('label') or d.get('label') or n['id'], geo[gid] * s + t))
    return out


def _project(meshes, cam):
    pos, rot, focal, frame, (w, h), _ = cam
    fy = focal / frame * h  # px per unit of tan, vertical frame fit
    discs = []
    for label, v in meshes:
        if len(v) < 50:
            continue
        ext = v.max(0) - v.min(0)
        if ext.min() < 0.9 * ext.max():
            continue  # flat or elongated: not a sphere
        c = v.mean(0)
        dist = np.linalg.norm(v - c, axis=1)
        if dist.mean() <= 0 or (dist.max() - dist.min()) / dist.mean() > SPHERE_TOL:
            continue
        if np.linalg.norm(pos - c) <= dist.mean():
            continue  # around the camera (a furnace dome)
        p = (v - pos) @ rot  # camera space: x right, y up, -z forward
        if (p[:, 2] >= 0).any():
            continue
        x = w / 2 + fy * p[:, 0] / -p[:, 2] - .5
        y = h / 2 - fy * p[:, 1] / -p[:, 2] - .5
        if x.max() < 0 or x.min() > w or y.max() < 0 or y.min() > h:
            continue
        discs.append(Disc(label, (x.min() + x.max()) / 2, (y.min() + y.max()) / 2,
                          (x.max() - x.min()) / 2, (y.max() - y.min()) / 2))
    return discs, (w, h)


def order(discs):
    """Rows top to bottom, each left to right."""
    rows = []
    for d in sorted(discs, key=lambda d: d.cy):
        if rows and abs(d.cy - rows[-1][-1].cy) < rows[-1][-1].r:
            rows[-1].append(d)
        else:
            rows.append([d])
    return [d for row in rows for d in sorted(row, key=lambda d: d.cx)]


def detect_matte(img):
    """Red spheres of a matte render (linear RGB float): Discs left to right
    (gloss-split.py's finder, #137). Only for a render whose spheres are all
    matte red."""
    red = (img[..., 0] > 2.5 * img[..., 1]) & (img[..., 0] > 0.02)
    cols = np.flatnonzero(red.sum(0) > 5)
    if not len(cols):
        return []
    groups, start = [], cols[0]
    for c0, c1 in zip(cols[:-1], cols[1:]):
        if c1 - c0 > 3:
            groups.append((start, c0))
            start = c1
    groups.append((start, cols[-1]))
    out = []
    for x0, x1 in groups:
        if x1 - x0 < 20:
            continue
        rows = np.flatnonzero(red[:, x0:x1 + 1].sum(1) > 2)
        out.append(Disc(f'S{len(out) + 1}', (x0 + x1) / 2, (rows[0] + rows[-1]) / 2,
                        (x1 - x0) / 2, (rows[-1] - rows[0]) / 2))
    return out


def projected(name):
    """(discs at the scene's render size, (w, h), source) from the scene's
    geometry, or None."""
    if name in _cache:
        return _cache[name]
    res = None
    dump = find_dump(name)
    duf_path = None
    if dump:
        try:
            sp = json.load(open(dump)).get('scene', '')
            duf_path = sp if os.path.exists(sp) else None
        except (OSError, ValueError):
            dump = None
    duf_path = find_duf(name) or duf_path
    if duf_path:
        duf = load_duf(duf_path)
        cam = camera(duf)
        if cam:
            meshes, src = (_dump_spheres(dump), 'dump ' + os.path.basename(dump)) if dump else (_duf_spheres(duf), 'duf')
            ds, size = _project(meshes, cam)
            if ds:
                note = ('; ' + ', '.join(cam[5])) if cam[5] else ''
                res = (order(ds), size, f'projected ({src}, camera {os.path.basename(duf_path)}){note}')
    _cache[name] = res
    return res


def discs(scene, shape=None, matte=None):
    """(Discs, source) for a scene (a name, its .duf or one of its renders),
    in pixels of an image of `shape` (h, w[, c]; default the scene's render
    size). Falls back to detection in `matte` (path or linear float array)
    when the scene has no geometry to project; ([], 'none') without either."""
    name = scene_name(scene)
    p = projected(name)
    if p:
        ds, (w, h), src = p
        if shape is not None and (shape[1], shape[0]) != (w, h):
            k = shape[1] / w
            ds = [d.scaled(k) for d in ds]
            src += f', scaled {w}x{h} -> {shape[1]}x{shape[0]}'
        return ds, src
    if matte is not None:
        img = matte
        if isinstance(matte, str):
            from PIL import Image
            img = (np.asarray(Image.open(matte).convert('RGB')).astype(float) / 255) ** 2.2
        ds = order(detect_matte(img))
        return ds, f'matte {os.path.basename(matte) if isinstance(matte, str) else "render"}'
    return [], 'none'


def mask(d, shape, frac=1.0, grid=None):
    """Pixels inside frac of disc d (elliptic: both half-axes)."""
    yy, xx = grid if grid is not None else np.mgrid[0:shape[0], 0:shape[1]]
    return np.hypot((xx - d.cx) / d.rx, (yy - d.cy) / d.ry) < frac


def dist(d, shape, grid=None):
    """Each pixel's distance from d's centre over its (elliptic) radius."""
    yy, xx = grid if grid is not None else np.mgrid[0:shape[0], 0:shape[1]]
    return np.hypot((xx - d.cx) / d.rx, (yy - d.cy) / d.ry)


def box(d, frac=0.6):
    """(y0, y1, x0, x1) of the square within frac of d's half-axes."""
    return (int(d.cy - d.ry * frac), int(d.cy + d.ry * frac), int(d.cx - d.rx * frac), int(d.cx + d.rx * frac))


def describe(ds, source):
    """One report line: the discs used and where they came from."""
    if not ds:
        return f'spheres: none found ({source})'
    return f'spheres ({source}): ' + '; '.join(
        f'{d.label} {d.cx:.0f},{d.cy:.0f} R {d.rx:.1f}x{d.ry:.1f}' for d in ds)


if __name__ == '__main__':
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        sys.exit(0)
    if args[0] == '--check':
        from PIL import Image
        worst = 0
        for p in args[1:]:
            img = (np.asarray(Image.open(p).convert('RGB')).astype(float) / 255) ** 2.2
            pr, src = discs(p, img.shape)
            det = detect_matte(img)
            print(f'{os.path.basename(p)}: {len(pr)} projected ({src}), {len(det)} detected')
            for a in pr:
                b = min(det, key=lambda b: math.hypot(a.cx - b.cx, a.cy - b.cy), default=None)
                if b is None:
                    continue
                e = (a.cx - b.cx, a.cy - b.cy, a.rx - b.rx, a.ry - b.ry)
                worst = max(worst, max(abs(x) for x in e))
                print(f'  {a!r} vs detected ({b.cx:.1f}, {b.cy:.1f}) R {b.rx:.1f} x {b.ry:.1f}: '
                      f'd {e[0]:+.1f} {e[1]:+.1f}, dR {e[2]:+.1f} {e[3]:+.1f}')
        print(f'worst {worst:.1f} px')
        sys.exit(0)
    size = None
    if args[0] == '--size':
        w, h = args[1].split('x')
        size, args = (int(h), int(w)), args[2:]
    for a in args:
        ds, src = discs(a, size)
        print(f'{scene_name(a)}: ' + describe(ds, src))
