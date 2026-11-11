#!/usr/bin/env python3
"""ORBIS strand-hair calibration scenes (#79): straight synthetic strands,
one light, the Uber gloss on them one part at a time.

  python3 scripts/dev/make-strand-scenes.py [<scenes dir>]

Built from calib/scenes/Calib-L-Distant-45.duf (camera at twice its focal
length, tone mapping, the 45 degree sun, Scene Only) with its floor, cubes
and sphere removed: the
objects hang in a black void, so each one's light is its own and the
strands' silhouettes are masks. Every geometry is written into the scene
(DSON `geometry_library`, no content needed):

- row A (y 105): one thick strand each, a polyline of 7 points along x,
  Line Start / End Width 20 - a tube big enough to see where across it the
  highlight sits, and (in the .mi) how wide and how many-sided DAZ
  tessellates a line for Iray;
- row B (y 75): the same tubes as polygon meshes (24 sides, smooth, capped,
  UV u around and v along as DAZ's line tessellation has it), radius 1 cm:
  in Octane a curve against a mesh tube is the curve primitive's shading;
- row C (y 30 - 42): patches of 3000 thin strands (Morgan Hair's widths,
  0.2 / 0.1), horizontal - several strands deep, so light bounces between
  them as in hair;
- row D (y -45 - -5): the same patches with the strands vertical (along
  the light's plane: the reflection cone misses the camera).

Materials (all channels shown, as product content saves them):
  1 diffuse grey 0.5; 2 gloss isotropic (white, reflectivity 0.5,
  roughness 0.16, no diffuse); 3 the same anisotropic (0.735, rotation
  0.25, as Morgan's); 4 Morgan's gloss (glossiness mode, Glossy Color /
  Specular, effect 2, no diffuse); 5 Morgan's Strands material without its
  top coat and maps (diffuse, translucency, overlay, refraction, effects).
Rows A / B take 1 2 3 4, rows C / D 1 3 4 5.

How DAZ hands a polyline to Iray depends on the hair's Line Tessellation
modifier (its .mi export reads the Viewport sides, the render the Render
sides; both are set alike here): none -> Iray `hair` (cubic B-spline fibres,
radius = Line Width / 20 cm); 2 sides -> flat ribbons one Line Width (mm)
wide, the normal the object's +z made perpendicular to the strand (-y for a
strand along z); 3 sides -> flat-shaded triangular prisms. Morgan Hair (#52)
carries 2 sides, dForce Corporate (Calib-Hair) 3. The modifier only runs on
a figure, so tessellated strands are written as figure nodes.

Scenes: Calib-Strand (the template's sun, fibres), Calib-Strand-Side (the
sun turned 60 degrees about y: the strands' cone), Calib-Strand-Dome (no
sun, the HDRI dome of Calib-Aniso, not drawn), Calib-Strand-T2 / -T3 (the
sun, ribbons / prisms), Calib-Strand-T2-Dome (ribbons under the dome, as
Morgan's hair in Calib-Hair-Teens). Existing scenes are kept (--force
rewrites them); written to "My Library/Scenes" and copied into
calib/scenes. --tess-probe <dir> writes row A at several sides into <dir>
(scratch, for the .mi). --mohawk <DAZ library> writes Calib-Mi-Mohawk-T2
(the free dForce Mohawk at sides 2, for its .mi; My Library only).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, math, os, random, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CALIB = os.path.join(REPO, 'calib', 'scenes')
_opt_vals = {sys.argv[i + 1] for i, a in enumerate(sys.argv[:-1]) if a in ('--mohawk', '--tess-probe')}
out_dir = next((a for a in sys.argv[1:] if not a.startswith('--') and a not in _opt_vals), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
FORCE = '--force' in sys.argv


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def channels_of(entry, typ):
    return next(e for e in entry['extra'] if e.get('type') == typ)['channels']


def set_channel(chs, cid, value):
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    raise KeyError(cid)


base = load(os.path.join(CALIB, 'Calib-L-Distant-45.duf'))
dome_src = load(os.path.join(CALIB, 'Calib-Aniso.duf'))

KEEP = {'Camera', 'Tonemapper Options', 'Environment Options', 'DistantLight'}
template_node = copy.deepcopy(next(l for l in base['node_library'] if l['id'] == 'pCube(1120000)1'))
template_geom = copy.deepcopy(next(g for g in base['geometry_library'] if g['id'] == 'geometry-3'))
template_mat = copy.deepcopy(next(m for m in base['material_library'] if m['id'] == 'Default'))
for c in channels_of(template_mat, 'studio_material_channels'):
    c['channel'].pop('visible', None)  # shown, as product content saves them (#52)

# --- materials -----------------------------------------------------------
BLACK = [0.0, 0.0, 0.0]
PLAIN = {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
         'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Glossy Layered Weight': 0.0}
GLOSS = {'Glossy Layered Weight': 1.0, 'Glossy Color': [1.0, 1.0, 1.0], 'Glossy Reflectivity': 0.5,
         'Glossy Roughness': 0.16}
ANISO = {'Glossy Anisotropy': 0.7352941, 'Glossy Anisotropy Rotations': 0.25}
# Morgan Hair's Strands (Calib-Hair-Teens, local): its glossy layer ...
MORGAN_GLOSS = {'Base Mixing': 1, 'Glossy Layered Weight': 0.9795402, 'Share Glossy Inputs': True,
                'Glossy Color': [0.9411765, 0.9137255, 0.8196079], 'Glossy Color Effect': 2,
                'Glossy Specular': [0.7215686, 0.7019608, 0.6156863], 'Glossiness': 0.8382353, **ANISO}
# ... and its base, without the top coat and the maps.
MORGAN_BASE = {'Diffuse Overlay Weight': 0.05, 'Diffuse Overlay Weight Squared': True,
               'Diffuse Overlay Color': [0.7843137, 0.7450981, 0.682353], 'Diffuse Overlay Color Effect': 2,
               'Translucency Weight': 0.0591133, 'Base Color Effect': 2,
               'Translucency Color': [0.8039216, 0.7647059, 0.6862745], 'Invert Transmission Normal': True,
               'SSS Reflectance Tint': [0.682353, 0.6666667, 0.6156863],
               'Refraction Index': 1.794118, 'Refraction Weight': 0.1061947, 'Abbe': 7.522124, 'Thin Walled': True}
MATERIALS = {
    1: ('diffuse', [0.5, 0.5, 0.5], {}),
    2: ('gloss iso', BLACK, GLOSS),
    3: ('gloss aniso', BLACK, {**GLOSS, **ANISO}),
    4: ('Morgan gloss', BLACK, MORGAN_GLOSS),
    5: ('Morgan nocoat', [0.854902, 0.8117647, 0.7294118], {**MORGAN_GLOSS, **MORGAN_BASE}),
}
THICK = 20.0                     # Line Start / End Width of rows A
THIN = (0.2, 0.1)                # Morgan's
TUBE_R = 1.0                     # cm, rows B


# --- geometry ------------------------------------------------------------
def line_strand(x0, x1, y, z, n=7):
    return [[x0 + (x1 - x0) * k / (n - 1), y, z] for k in range(n)]


def patch(seed, count, axis):
    """`count` straight strands, 50 cm (horizontal) / 40 cm (vertical)
    long, random over a 12 x 3 cm cross-section, around the origin."""
    rnd = random.Random(seed)
    out = []
    for _ in range(count):
        a, b = rnd.uniform(-6, 6), rnd.uniform(-1.5, 1.5)
        if axis == 'x':
            out.append(line_strand(-25, 25, a, b, 8))
        else:
            out.append([[a, -20 + 40 * k / 7, b] for k in range(8)])
    return out


def strands_geometry(gid, strands):
    g = copy.deepcopy(template_geom)
    verts, lines = [], []
    for s in strands:
        lines.append([0, 0] + list(range(len(verts), len(verts) + len(s))))
        verts.extend(s)
    g.update({'id': gid, 'name': gid,
              'vertices': {'count': len(verts), 'values': [[round(c, 4) for c in v] for v in verts]},
              'polylist': {'count': 0, 'values': []},
              'polyline_list': {'count': len(lines), 'segment_count': sum(len(l) - 3 for l in lines), 'values': lines},
              'default_uv_set': '#uv-' + gid})
    uv = {'id': 'uv-' + gid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(verts),
          'uvs': {'count': len(verts), 'values': [[0, 0]] * len(verts)}, 'polygon_vertex_indices': []}
    return g, uv


def tube_geometry(gid, length=60.0, r=TUBE_R, sides=24, rings=7):
    """A capped tube along x, centred on the origin. UV u around (one seam
    column more), v along, as DAZ tessellates a line for Iray."""
    g = copy.deepcopy(template_geom)
    verts, polys, uvs, pvi = [], [], [], []
    for i in range(rings):
        x = -length / 2 + length * i / (rings - 1)
        for j in range(sides):
            t = 2 * math.pi * j / sides
            verts.append([x, r * math.cos(t), r * math.sin(t)])
    for i in range(rings):
        for j in range(sides + 1):
            uvs.append([j / sides, i / (rings - 1)])
    vid = lambda i, j: i * sides + j % sides
    uid = lambda i, j: i * (sides + 1) + j
    for i in range(rings - 1):
        for j in range(sides):
            f = len(polys)
            # outward: around +x the ring runs y -> z, so (i, j) (i+1, j) (i+1, j+1) (i, j+1) faces out
            polys.append([0, 0, vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j)])
            for v, u in ((vid(i, j), uid(i, j)), (vid(i, j + 1), uid(i, j + 1)),
                         (vid(i + 1, j + 1), uid(i + 1, j + 1)), (vid(i + 1, j), uid(i + 1, j))):
                if u != v:
                    pvi.append([f, v, u])
    # caps: a centre vertex and a triangle fan at each end
    for i, sign in ((0, -1), (rings - 1, 1)):
        c = len(verts)
        verts.append([-length / 2 if i == 0 else length / 2, 0.0, 0.0])
        uc = len(uvs)
        uvs.append([0.5, 0.5])
        for j in range(sides):
            a, b = vid(i, j), vid(i, j + 1)
            tri = [c, b, a] if sign < 0 else [c, a, b]
            f = len(polys)
            polys.append([0, 0] + tri)
            for v in tri:
                u = uc if v == c else uid(i, j if v == a else j + 1)
                if u != v:
                    pvi.append([f, v, u])
    g.update({'id': gid, 'name': gid,
              'vertices': {'count': len(verts), 'values': [[round(c, 5) for c in v] for v in verts]},
              'polylist': {'count': len(polys), 'values': polys},
              'default_uv_set': '#uv-' + gid})
    g.pop('polyline_list', None)
    uv = {'id': 'uv-' + gid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(verts),
          'uvs': {'count': len(uvs), 'values': [[round(a, 5), round(b, 5)] for a, b in uvs]},
          'polygon_vertex_indices': pvi}
    return g, uv


def material(mid, k, widths):
    label, colour, over = MATERIALS[k]
    m = copy.deepcopy(template_mat)
    m['id'] = mid
    m['diffuse']['channel']['current_value'] = colour
    chs = channels_of(m, 'studio_material_channels')
    for cid, v in {**PLAIN, **over, 'Line Start Width': widths[0], 'Line End Width': widths[1]}.items():
        set_channel(chs, cid, v)
    return m, colour


# Objects: (id, label, row, x, material, kind)
X = [-112.5, -37.5, 37.5, 112.5]
OBJECTS = []
for col, k in enumerate((1, 2, 3, 4)):
    OBJECTS.append((f'A{k}', f'A{k} thick {MATERIALS[k][0]}', 105.0, X[col], k, 'thick'))
    OBJECTS.append((f'B{k}', f'B{k} tube {MATERIALS[k][0]}', 75.0, X[col], k, 'tube'))
for col, k in enumerate((1, 3, 4, 5)):
    OBJECTS.append((f'C{k}', f'C{k} strands {MATERIALS[k][0]}', 36.0, X[col], k, 'patch-x'))
    OBJECTS.append((f'D{k}', f'D{k} strands vertical {MATERIALS[k][0]}', -25.0, X[col], k, 'patch-y'))


def tessellation(d, oid, sides):
    """DAZ's Line Tessellation modifier on a strand node (as dForce hair and
    Morgan Hair carry it): without it DAZ hands Iray the polylines as Iray
    hair (`hair`, cubic B-spline fibres); with it as a `trilist` of
    `sides`-sided strips."""
    chans = lambda cur: [{'channel': {'id': f'{w} Line Tessellation Sides', 'type': 'int', 'label': f'{w} Line Tessellation Sides',
                                      'value': 1 if w == 'Render' else 0, 'current_value': cur,
                                      'min': -2147483648, 'max': 2147483647, 'step_size': 1},
                          'group': '/General/Line Tessellation'} for w in ('Viewport', 'Render')]
    mid = 'Line Tessellation-' + oid
    d.setdefault('modifier_library', []).append({
        'id': mid, 'name': 'Line Tessellation', 'parent': '#' + oid,
        'extra': [{'type': 'studio/modifier/line_tessellation'}, {'type': 'studio_modifier_channels', 'channels': chans(sides)}]})
    d['scene'].setdefault('modifiers', []).append({
        'id': mid + '-1', 'url': '#' + mid.replace(' ', '%20'), 'parent': '#' + oid + '-1',
        'extra': [{'type': 'studio/modifier/line_tessellation'},
                  {'type': 'studio_modifier_channels', 'channels': [{'channel': {'id': c['channel']['id'], 'type': 'int', 'current_value': c['channel']['current_value']}} for c in chans(sides)]}]})


def build(name, sun_y=None, dome=False, tess=None, rows='ABCD', save_dir=None):
    d = copy.deepcopy(base)
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    d['asset_info']['contributor'] = {'author': '', 'email': '', 'website': ''}
    d['node_library'] = [l for l in d['node_library'] if l['id'] in KEEP]
    d['scene']['nodes'] = [n for n in d['scene']['nodes'] if n['url'].lstrip('#').replace('%20', ' ') in KEEP]
    d['geometry_library'], d['uv_set_library'], d['material_library'], d['scene']['materials'] = [], [], [], []
    # Twice the template's focal length (65 mm): the rows fill the frame.
    cam = next(l for l in d['node_library'] if l['id'] == 'Camera')
    set_channel(channels_of(cam, 'studio_node_channels'), 'Focal Length', 130.0)
    if sun_y is not None:
        light = next(l for l in d['node_library'] if l['id'] == 'DistantLight')
        next(c for c in light['rotation'] if c['id'] == 'y')['current_value'] = sun_y
    if dome:
        d['node_library'] = [l for l in d['node_library'] if l['id'] not in ('DistantLight', 'Environment Options', 'Tonemapper Options')]
        d['scene']['nodes'] = [n for n in d['scene']['nodes'] if not n['id'].startswith(('DistantLight', 'Environment', 'Tonemapper'))]
        for lid in ('Environment Options', 'Tonemapper Options'):
            d['node_library'].append(copy.deepcopy(next(l for l in dome_src['node_library'] if l['id'] == lid)))
            d['scene']['nodes'].append(copy.deepcopy(next(n for n in dome_src['scene']['nodes'] if n['id'] == lid + '-1')))
        env = next(l for l in d['node_library'] if l['id'] == 'Environment Options')
        set_channel(channels_of(env, 'studio_node_channels'), 'Draw Dome', False)
        d['image_library'] = copy.deepcopy(dome_src['image_library'])
    for oid, label, y, x, k, kind in OBJECTS:
        if oid[0] not in rows:
            continue
        gid = 'geom-' + oid
        if kind == 'thick':
            g, uv = strands_geometry(gid, [line_strand(-30, 30, 0, 0)])
            widths = (THICK, THICK)
        elif kind == 'tube':
            g, uv = tube_geometry(gid)
            widths = THIN
        else:
            g, uv = strands_geometry(gid, patch(ord(kind[-1]) * 10 + k, 3000, kind[-1]))
            widths = THIN
        d['geometry_library'].append(g)
        d['uv_set_library'].append(uv)
        nl = copy.deepcopy(template_node)
        nl.update({'id': oid, 'name': oid, 'label': label})
        if tess is not None and kind != 'tube':
            nl['type'] = 'figure'  # DAZ runs a modifier stack on figures only
        for c in nl['translation']:
            c['current_value'] = {'x': x, 'y': y, 'z': 0.0}[c['id']]
        d['node_library'].append(nl)
        d['scene']['nodes'].append({
            'id': oid + '-1', 'url': '#' + oid, 'name': oid, 'label': label,
            'geometries': [{'id': gid + '-i', 'url': '#' + gid, 'name': 'shape', 'label': 'shape', 'type': 'polygon_mesh'}],
            'extra': [{'type': 'studio_node_channels', 'version': 5}]})
        if tess is not None and kind != 'tube':
            tessellation(d, oid, tess)
        m, colour = material('Mat-' + oid, k, widths)
        d['material_library'].append(m)
        d['scene']['materials'].append({
            'id': 'Mat-' + oid + '-i', 'url': '#Mat-' + oid, 'geometry': '#' + gid + '-i', 'groups': ['Default'],
            'diffuse': {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}},
            'uv_set': '#uv-' + gid, 'extra': [{'type': 'studio/material/uber_iray', 'version': '1.1.0.0'}]})
    duf_camera.ensure(d)
    if save_dir:  # a scratch scene
        p = os.path.join(save_dir, name + '.duf')
        open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        return p
    p = os.path.join(out_dir, name + '.duf')
    if os.path.exists(p) and not FORCE:
        print('kept', p)
    else:
        open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        print(p)
    shutil.copy2(p, os.path.join(CALIB, name + '.duf'))


MOHAWK = 'People/Genesis 8 Female/Hair/Strand-Based Hair Samples/G8F dForce Hair Mohawk.duf'


def mohawk_scene(lib):
    """Calib-Mi-Mohawk-T2: the free dForce Mohawk preset (dForce Starter
    Essentials, on Genesis 8 Female) as a scene at Line Tessellation 2, with
    this file's camera: its .mi shows the ribbon frames of generated render
    hairs (#79). It carries the preset's own data, so it stays in My Library
    only (not calib/scenes); a headless DAZ crashed saving it itself."""
    d = load(os.path.join(lib, MOHAWK))
    d['asset_info']['type'] = 'scene'
    d['asset_info']['id'] = '/Scenes/Calib-Mi-Mohawk-T2.duf'
    d['node_library'] = [copy.deepcopy(next(l for l in base['node_library'] if l['id'] == 'Camera'))]
    d['scene']['nodes'].append(copy.deepcopy(next(n for n in base['scene']['nodes'] if n['url'].lstrip('#') == 'Camera')))
    for m in d['scene']['modifiers']:
        if m['id'] == 'Line Tessellation':
            m['extra'].append({'type': 'studio_modifier_channels', 'channels': [
                {'channel': {'id': f'{w} Line Tessellation Sides', 'type': 'int', 'current_value': 2}} for w in ('Viewport', 'Render')]})
    duf_camera.ensure(d)
    p = os.path.join(out_dir, 'Calib-Mi-Mohawk-T2.duf')
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)


if '--mohawk' in sys.argv:
    mohawk_scene(sys.argv[sys.argv.index('--mohawk') + 1])
    sys.exit(0)
if '--tess-probe' in sys.argv:
    # scratch: row A at each Render Line Tessellation Sides, for the .mi
    probe = sys.argv[sys.argv.index('--tess-probe') + 1]
    os.makedirs(probe, exist_ok=True)
    for t in (None, 0, 1, 2, 3, 6):
        print(build(f'Strand-Tess-{t}', tess=t, rows='A', save_dir=probe))
    sys.exit(0)
build('Calib-Strand')
build('Calib-Strand-Side', sun_y=60.0)
build('Calib-Strand-Dome', dome=True)
build('Calib-Strand-T2', tess=2)
build('Calib-Strand-T3', tess=3)
build('Calib-Strand-T2-Dome', dome=True, tess=2)
