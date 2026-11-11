#!/usr/bin/env python3
"""ORBIS calibration: Genesis 9 eye close-ups (#66), the second step after
scripts/dev/daz-build-eye-scene.dsa.

  python3 scripts/dev/make-eye-scenes.py <base.duf> [<My Library Scenes dir>] [--force]

<base.duf> is what the DAZ script saved (Calib-L-Distant-45 with Genesis 9
in place of the cubes and the sphere). Aims the camera at the figure's left
eye (straight on, 100 mm, 9 cm off the cornea), turns the distant light to
come from the front-left above (the brow shadowed the eye at 45 degrees),
renders the figures at SubD level 1, and writes the variants into calib/scenes and My Library/Scenes (ORBIS_LIBRARY
or the argument). Genesis 9 is free with DAZ Studio: the scenes reference it
only. A scene that exists is kept (its exposure may be tuned) unless --force.

Genesis 9's eye is one PBRSkin surface per eye (a closed ball): diffuse
(weight 1 - Translucency Weight) over a diffuse transmission into the
volume (absorption: Transmitted Color at 0.1 cm; scattering: PBRSkin's
default SSS Color at 0.015 cm, SSS Direction -0.8). The variants switch
those parts off one at a time (pbr_skin.mdl, `diffuse_translucency_bsdf`):

- Calib-G9Eye: as Genesis 9 loads.
- -bare: the eye moisture (cornea) and the tear cut out (Cutout Opacity 0).
- -nogloss: bare, the dual lobe specular off. The rest build on it:
- -t0: Translucency Weight 0 - the diffuse layer alone (the eye map).
- -t1: Translucency Weight 1 - the transmission into the volume alone.
- -t1-white: t1 without the translucency map (white): the volume's own
  return, one colour over the ball.
- -t08-white: Translucency Weight 0.8 (as loaded), both maps white: the
  diffuse layer and the volume together, the inner reflection included.
- -t1-white-nosss: t1-white with Sub Surface Enable off: absorption only,
  light through the ball.
"""
import sys as _sys
if len(_sys.argv) < 2 or _sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
calib = os.path.join(repo, 'calib', 'scenes')
args = [a for a in sys.argv[1:] if a != '--force']
FORCE = '--force' in sys.argv
src = args[0]
lib = args[1] if len(args) > 1 else os.path.join(os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library'), 'Scenes')

# Genesis 9's l_eye bone (Genesis9.dsf): centre (2.975, 159.09, 7.375) cm,
# its end - the cornea's apex - at z 8.68.
EYE = (2.975, 159.09, 8.68)


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def lib_node(d, lid):
    return next(n for n in d['node_library'] if n['id'] == lid)


def transform(n, pos, rot):
    for key, values in (('translation', pos), ('rotation', rot)):
        for c, v in zip(n[key], values):
            c['current_value'] = v


def node_channel(n, cid, value):
    for e in n.get('extra', []):
        for c in e.get('channels', []):
            if c['channel']['id'] == cid:
                c['channel']['current_value'] = value
                return
    raise KeyError(cid)


def channels(m):
    ext = next((e for e in m.get('extra', []) if e.get('type') == 'studio_material_channels'), None)
    if ext is None:
        ext = {'type': 'studio_material_channels', 'channels': []}
        m.setdefault('extra', []).append(ext)
    return ext['channels']


def set_channel(m, cid, value, typ, image=False):
    """A channel's value; image=None drops its map (the colour alone)."""
    chs = channels(m)
    c = next((c['channel'] for c in chs if c['channel']['id'] == cid), None)
    if c is None:
        c = {'id': cid, 'type': typ}
        chs.append({'channel': c})
    c['current_value'] = value
    if image is None:
        c.pop('image_file', None)
        c['image'] = None


def materials(d, *ids):
    out = [m for m in d['scene']['materials'] if m['id'] in ids]
    assert len(out) == len(ids), (ids, [m['id'] for m in out])
    return out


base = load(src)
transform(lib_node(base, 'Camera'), (EYE[0], EYE[1] + 0.3, EYE[2] + 9.0), (0, 0, 0))
cam = lib_node(base, 'Camera')
node_channel(cam, 'Focal Length', 100)
node_channel(cam, 'DOF', False)
# From the front, 20 degrees down and 25 to the side: a highlight beside the
# iris, the eye out of the brow's shadow.
transform(lib_node(base, 'DistantLight'), (0, 0, 0), (-20, 25, 0))
base['asset_info']['id'] = '/Scenes/Calib-G9Eye.duf'
# SubD level 1 at render (1.6 M faces at DAZ's 2: too big for a calibration
# render, as Calib-Shell-G9); both renderers read the same level.
for n in base['scene']['nodes']:
    for g in n.get('geometries', []):
        for e in g.get('extra', []):
            for c in e.get('channels', []):
                if c['channel']['id'] == 'SubDRenderLevel':
                    c['channel']['current_value'] = 1

EYES = ('Eye Left', 'Eye Right')


def bare(d):
    for m in materials(d, 'EyeMoisture Left-1', 'EyeMoisture Right-1', 'Tear-1'):
        set_channel(m, 'Cutout Opacity', 0.0, 'float', image=None)


def nogloss(d):
    bare(d)
    for m in materials(d, *EYES):
        set_channel(m, 'Dual Lobe Specular Enable', False, 'bool')


def translucency(w, white=False, sss=True):
    def f(d):
        nogloss(d)
        for m in materials(d, *EYES):
            set_channel(m, 'Translucency Weight', w, 'float')
            if white:
                set_channel(m, 'Translucency Color', [1, 1, 1], 'float_color', image=None)
                m['diffuse']['channel']['current_value'] = [1, 1, 1]
                m['diffuse']['channel']['image'] = None
                m['diffuse']['channel'].pop('image_file', None)
            if not sss:
                set_channel(m, 'Sub Surface Enable', False, 'bool')
    return f


VARIANTS = [
    ('Calib-G9Eye', lambda d: None),
    ('Calib-G9Eye-bare', bare),
    ('Calib-G9Eye-nogloss', nogloss),
    ('Calib-G9Eye-t0', translucency(0.0)),
    ('Calib-G9Eye-t1', translucency(1.0)),
    ('Calib-G9Eye-t1-white', translucency(1.0, white=True)),
    ('Calib-G9Eye-t08-white', translucency(0.8, white=True)),
    ('Calib-G9Eye-t1-white-nosss', translucency(1.0, white=True, sss=False)),
]

for name, change in VARIANTS:
    dst = os.path.join(calib, name + '.duf')
    if os.path.exists(dst) and not FORCE:
        print(name, 'kept')
        continue
    d = copy.deepcopy(base)
    change(d)
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    duf_camera.ensure(d)  # DAZ saved it headless, without current_camera (#97)
    open(dst, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    if os.path.isdir(lib):
        shutil.copyfile(dst, os.path.join(lib, name + '.duf'))
    print(name)
