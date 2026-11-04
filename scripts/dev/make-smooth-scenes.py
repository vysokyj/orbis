#!/usr/bin/env python3
"""ORBIS calibration scenes for geometry smoothing and Round Corners (#40):
copies of Calib-Materials.duf (eight spheres on a plane under an HDRI) whose
spheres become low-poly shapes, each with one setting of DAZ's
/Geometry/Smoothing channels, written beside it.

  python3 scripts/dev/make-smooth-scenes.py [--force] [<Calib-Materials.duf>]

- Calib-Smooth: Smooth On / Smooth Angle on an icosahedron (all its edges
  41.8 degrees), cubes (90), a 12-sided cylinder (30 round the side, 90 at
  the caps) and an icosahedron whose two halves carry different materials.
- Calib-RoundCorners: Round Corners Radius 0 / 0.005 / 0.01 / 0.02 / 1,
  Roundness 0, Across Materials on / off on a cube whose top is another
  material.

The shapes are our own geometry (no content); a glossy grey Iray Uber so the
HDRI's highlights trace the normals. Dump with scripts/dev/daz-dump.sh, .mi
with scripts/dev/daz-export-mi.dsa.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, math, os, sys

src = next((a for a in sys.argv[1:] if a.endswith('.duf')), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes/Calib-Materials.duf'
out_dir = os.path.dirname(src)
FORCE = '--force' in sys.argv


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def set_channel(channels, cid, value, typ):
    for c in channels:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    channels.append({'channel': {'id': cid, 'type': typ, 'current_value': value}})


# --- Shapes: (vertices in cm, faces, material index per face). Like the
# base's spheres: 30 cm across, standing on y = 0 of their own frame.
S = 15.0


def cube(top_material=False):
    v = [[x, y, z] for y in (0, 2 * S) for z in (S, -S) for x in (-S, S)]
    # 0..3 bottom (y 0), 4..7 top; counter-clockwise seen from outside.
    faces = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [3, 2, 6, 7], [2, 0, 4, 6], [1, 3, 7, 5]]
    mats = [0, 1 if top_material else 0, 0, 0, 0, 0]
    return v, faces, mats


def icosahedron(split=False):
    t = (1 + 5 ** 0.5) / 2
    raw = [[-1, t, 0], [1, t, 0], [-1, -t, 0], [1, -t, 0], [0, -1, t], [0, 1, t],
           [0, -1, -t], [0, 1, -t], [t, 0, -1], [t, 0, 1], [-t, 0, -1], [-t, 0, 1]]
    r = (1 + t * t) ** 0.5
    v = [[p[0] / r * S, p[1] / r * S + S, p[2] / r * S] for p in raw]
    faces = [[0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
             [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1]]
    # Two materials split by x (no face straddles x = 0 with its centroid).
    mats = [1 if split and sum(v[i][0] for i in f) > 0 else 0 for f in faces]
    return v, faces, mats


def cylinder(n=12):
    v = []
    for y in (0, 2 * S):
        for k in range(n):
            a = 2 * math.pi * k / n
            v.append([S * math.sin(a), y, S * math.cos(a)])
    v += [[0, 0, 0], [0, 2 * S, 0]]
    faces = [[k, (k + 1) % n, n + (k + 1) % n, n + k] for k in range(n)]
    faces += [[2 * n, (k + 1) % n, k] for k in range(n)]
    faces += [[2 * n + 1, n + k, n + (k + 1) % n] for k in range(n)]
    return v, faces, [0] * len(faces)


def outward(v, faces):
    """Flip faces whose winding points into the shape (centroid test)."""
    c = [sum(p[k] for p in v) / len(v) for k in range(3)]
    out = []
    for f in faces:
        a, b, d = v[f[0]], v[f[1]], v[f[2]]
        u = [b[k] - a[k] for k in range(3)]
        w = [d[k] - a[k] for k in range(3)]
        n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]]
        m = [sum(v[i][k] for i in f) / len(f) - c[k] for k in range(3)]
        out.append(f if sum(n[k] * m[k] for k in range(3)) >= 0 else f[::-1])
    return out


GLOSSY_GREY = {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
               'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Glossy Layered Weight': 1.0,
               'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.2}
GREY = [0.5, 0.5, 0.5]
BLUE = [0.1, 0.2, 0.5]
TYPES = {bool: 'bool', float: 'float', int: 'int'}

base = load(src)
spheres = [n for n in base['scene']['nodes'] if n['id'].startswith('pSphere')]
xs = {l['id']: next(c['current_value'] for c in l['translation'] if c['id'] == 'x')
      for l in base['node_library'] if l['id'].startswith('pSphere')}
spheres.sort(key=lambda n: xs[n['url'].lstrip('#').replace('%28', '(').replace('%29', ')')])


def scene(name, variants):
    p = os.path.join(out_dir, name + '.duf')
    if os.path.exists(p) and not FORCE:
        return
    d = copy.deepcopy(base)
    nodes = {n['id']: n for n in d['scene']['nodes']}
    mlib = {m['id']: m for m in d['material_library']}
    for k, (s0, (label, shape, rot_y, over)) in enumerate(zip(spheres, variants)):
        n = nodes[s0['id']]
        n['label'] = label
        lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
        lib['label'] = label
        for c in lib.get('rotation', []):
            if c['id'] == 'y':
                c['current_value'] = rot_y
        v, faces, mats = shape
        faces = outward(v, faces)
        gid, uid = f'orbis-shape-{k}', f'orbis-uv-{k}'
        groups = ['A', 'B'] if 1 in mats else ['Default']
        # Planar UVs per vertex (the shapes need no texture; DAZ wants a set).
        lo = [min(p[i] for p in v) for i in range(3)]
        hi = [max(p[i] for p in v) for i in range(3)]
        d['uv_set_library'].append({'id': uid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(v),
                                    'uvs': {'count': len(v), 'values': [[(p[0] - lo[0]) / (hi[0] - lo[0]), (p[1] - lo[1]) / (hi[1] - lo[1])] for p in v]}})
        d['geometry_library'].append({
            'id': gid, 'name': 'geometry', 'type': 'polygon_mesh',
            'vertices': {'count': len(v), 'values': v},
            'polygon_groups': {'count': 1, 'values': ['Default']},
            'polygon_material_groups': {'count': len(groups), 'values': groups},
            'polylist': {'count': len(faces), 'values': [[0, m] + f for f, m in zip(faces, mats)]},
            'default_uv_set': '#' + uid,
        })
        inst = n['geometries'][0]
        inst['url'] = '#' + gid
        m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + inst['id'])
        m['groups'] = [groups[0]]
        m['uv_set'] = '#' + uid
        scene_mats = [m]
        if len(groups) > 1:
            m2 = copy.deepcopy(m)
            m2['id'] = m['id'] + '-B'
            m2['groups'] = ['B']
            lm2 = copy.deepcopy(mlib[m['url'].lstrip('#')])
            lm2['id'] = lm2['id'] + '-B'
            d['material_library'].append(lm2)
            mlib[lm2['id']] = lm2
            m2['url'] = '#' + lm2['id']
            d['scene']['materials'].append(m2)
            scene_mats.append(m2)
        for i, sm in enumerate(scene_mats):
            colour = BLUE if i else GREY
            sm['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
            lm = mlib[sm['url'].lstrip('#')]
            lm.setdefault('diffuse', {'channel': {'id': 'diffuse', 'type': 'float_color'}})['channel']['current_value'] = colour
            ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
            for key, val in {**GLOSSY_GREY, **over}.items():
                set_channel(ch, key, val, TYPES[type(val)])
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)


scene('Calib-Smooth', [
    ('S1 ico smooth off', icosahedron(), 0, {'Smooth On': False}),
    ('S2 ico angle 30', icosahedron(), 0, {'Smooth Angle': 30.0}),
    ('S3 ico angle 45', icosahedron(), 0, {'Smooth Angle': 45.0}),
    ('S4 ico default', icosahedron(), 0, {}),
    ('S5 ico two materials', icosahedron(split=True), 0, {}),
    ('S6 cube default', cube(), 30, {}),
    ('S7 cube angle 91', cube(), 30, {'Smooth Angle': 91.0}),
    ('S8 cylinder default', cylinder(), 0, {}),
])
# MDL takes the radius in meters (spec 19.3); DAZ hands the channel over as
# it is. 0.005 - 0.02 (0.5 - 2 cm) are the issue's sizes; R5's 1 tells the
# unit apart (1 m smooths the whole 30 cm cube, 1 cm would look like R3).
RC = {'Round Corners Radius': 0.01}
scene('Calib-RoundCorners', [
    ('R1 radius 0', cube(), 30, {}),
    ('R2 radius 0.005', cube(), 30, {'Round Corners Radius': 0.005}),
    ('R3 radius 0.01', cube(), 30, RC),
    ('R4 radius 0.02', cube(), 30, {'Round Corners Radius': 0.02}),
    ('R5 radius 1', cube(), 30, {'Round Corners Radius': 1.0}),
    ('R6 radius 0.01 roundness 0', cube(), 30, {**RC, 'Round Corners Roundness': 0.0}),
    ('R7 two materials across', cube(top_material=True), 30, RC),
    ('R8 two materials not across', cube(top_material=True), 30, {**RC, 'Round Corners Across Materials': False}),
])
