#!/usr/bin/env python3
"""ORBIS light calibration scenes (#6, #8): variants of the Calib-Point /
-Area / -Spot / -Distant / -Emissive scenes, cleaned up, written as
Calib-L-*.duf beside them.

  python3 scripts/dev/make-light-scenes.py [<scenes dir>]

The originals (2026-10-24) had faults: a Genesis 8 Female with
random-walk skin in every scene (its own error mixed into the light's),
partly off the floor; the sphere floating 1.2 m up; the cubes sitting
where the spot's beam peaks; one flux / size / angle per light; the point
light measured in the bottom 10–20 % of the range. Here:

- the figure is hidden, the sphere rests on the floor beside the cubes;
- the scene's lights only (Environment Mode "Scene Only", no dome);
- tone mapping at the calibrated defaults (Burn 1, Crush 0, Gamma 2.2);
  the exposure is set per scene by scripts/dev/calib-render.py from a test
  render, so nothing clips and nothing sits in the shadows;
- each light in several configurations (flux, height, size, two-sided,
  shape, angle, units); the spot's profile on an empty floor seen from
  straight above.

Existing scenes are kept (--force rewrites): calib-render.py tunes their
exposure in place. Dump and export them with scripts/dev/daz-dump.sh.
"""
import gzip, json, os, sys

scenes = next((a for a in sys.argv[1:] if not a.startswith('--')), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'


def load(name):
    raw = open(os.path.join(scenes, name + '.duf'), 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def lib(d, lid):
    return next(l for l in d['node_library'] if l['id'] == lid)


def channels(entry):
    ext = next((e for e in entry.setdefault('extra', []) if e.get('type') == 'studio_node_channels'), None)
    if ext is None:
        ext = {'type': 'studio_node_channels', 'version': 5, 'channels': []}
        entry['extra'].append(ext)
    return ext.setdefault('channels', [])


def set_channel(chs, cid, value):
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    typ = {bool: 'bool', float: 'float', int: 'int'}.get(type(value), 'float')
    chs.append({'channel': {'id': cid, 'type': typ, 'current_value': value}})


def set_xyz(entry, key, values):
    for c in entry[key]:
        if c['id'] in values:
            c['current_value'] = values[c['id']]


def hide(d, pred):
    """Hide scene nodes (Visible off on the scene node)."""
    for n in d['scene']['nodes']:
        if pred(n):
            set_channel(channels(n), 'Visible', False)


def clean(d, profile=False):
    hide(d, lambda n: 'Genesis' in n.get('url', ''))
    sphere = lib(d, 'pSphere(1120000)24_12')
    set_xyz(sphere, 'translation', {'x': 120.0, 'y': 0.0})
    env = channels(lib(d, 'Environment Options'))
    set_channel(env, 'Environment Mode', 3)
    set_channel(env, 'Draw Dome', False)
    set_channel(env, 'Draw Ground', False)
    tone = channels(lib(d, 'Tonemapper Options'))
    for k, v in (('Burn Highlights', 1.0), ('Crush Blacks', 0.0), ('Gamma', 2.2), ('Saturation', 1.0)):
        set_channel(tone, k, v)
    if profile:
        # The spot's footprint on the bare floor, seen from straight above.
        hide(d, lambda n: n.get('url', '').startswith('#pCube') or n.get('url', '').startswith('#pSphere'))
        cam = lib(d, 'Camera')
        set_xyz(cam, 'translation', {'x': 0.0, 'y': 520.0, 'z': 0.0})
        set_xyz(cam, 'rotation', {'x': -90.0, 'y': 0.0, 'z': 0.0})


def light(d, lid, ch=None, t=None, r=None):
    l = lib(d, lid)
    for k, v in (ch or {}).items():
        set_channel(channels(l), k, v)
    if t:
        set_xyz(l, 'translation', t)
    if r:
        set_xyz(l, 'rotation', r)


def emitter(d, ch):
    """The emissive plane's material (Calib-Emissive: the scene material on
    the second plane's geometry)."""
    m = next(m for m in d['material_library'] if m['id'] == 'Default-5')
    ext = next(e for e in m['extra'] if e['type'] == 'studio_material_channels')
    for k, v in ch.items():
        set_channel(ext['channels'], k, v)


DOWN = {'x': -90.0, 'y': 0.0, 'z': 0.0}
VARIANTS = [
    # name, source, profile view, edit
    ('Point-5k', 'Calib-Point', False, lambda d: None),
    ('Point-5k-h150', 'Calib-Point', False, lambda d: light(d, 'PointLight', t={'y': 150.0})),
    ('Point-20k', 'Calib-Point', False, lambda d: light(d, 'PointLight', {'Flux': 20000.0})),
    ('Point-legacy', 'Calib-Point', False, lambda d: light(d, 'PointLight', {'Photometric Mode': False, 'Intensity': 1.0})),
    ('Sphere-d20', 'Calib-Point', False, lambda d: light(d, 'PointLight', {'Light Geometry': 3, 'Width': 20.0, 'Height': 20.0})),
    ('Sphere-d50', 'Calib-Point', False, lambda d: light(d, 'PointLight', {'Light Geometry': 3, 'Width': 50.0, 'Height': 50.0})),
    ('Rect-25', 'Calib-Area', False, lambda d: light(d, 'PointLight', {'Width': 25.0, 'Height': 25.0}, r=DOWN)),
    ('Rect-50', 'Calib-Area', False, lambda d: light(d, 'PointLight', r=DOWN)),
    ('Rect-100', 'Calib-Area', False, lambda d: light(d, 'PointLight', {'Width': 100.0, 'Height': 100.0}, r=DOWN)),
    ('Rect-50-2sided', 'Calib-Area', False, lambda d: light(d, 'PointLight', {'Two Sided': True}, r=DOWN)),
    ('Disc-50', 'Calib-Area', False, lambda d: light(d, 'PointLight', {'Light Geometry': 2}, r=DOWN)),
    ('Spot-scene', 'Calib-Spot', False, lambda d: None),
    ('Spot-s30-e4', 'Calib-Spot', True, lambda d: light(d, 'SpotLight', {'Spread Angle': 30.0}, t={'x': 0.0, 'y': 250.0, 'z': 0.0}, r=DOWN)),
    ('Spot-s60-e4', 'Calib-Spot', True, lambda d: light(d, 'SpotLight', t={'x': 0.0, 'y': 250.0, 'z': 0.0}, r=DOWN)),
    ('Spot-s90-e4', 'Calib-Spot', True, lambda d: light(d, 'SpotLight', {'Spread Angle': 90.0}, t={'x': 0.0, 'y': 250.0, 'z': 0.0}, r=DOWN)),
    ('Spot-s60-e1', 'Calib-Spot', True, lambda d: light(d, 'SpotLight', {'Beam Exponent': 1.0}, t={'x': 0.0, 'y': 250.0, 'z': 0.0}, r=DOWN)),
    ('Spot-s60-e16', 'Calib-Spot', True, lambda d: light(d, 'SpotLight', {'Beam Exponent': 16.0}, t={'x': 0.0, 'y': 250.0, 'z': 0.0}, r=DOWN)),
    ('Distant-30', 'Calib-Distant', False, lambda d: light(d, 'DistantLight', r={'x': -30.0})),
    ('Distant-45', 'Calib-Distant', False, lambda d: None),
    ('Distant-60', 'Calib-Distant', False, lambda d: light(d, 'DistantLight', r={'x': -60.0})),
    ('Distant-45-5k', 'Calib-Distant', False, lambda d: light(d, 'DistantLight', {'Flux': 5000.0})),
    ('Emit-cd', 'Calib-Emissive', False, lambda d: None),
    ('Emit-cd-2sided', 'Calib-Emissive', False, lambda d: emitter(d, {'Two Sided Light': True})),
    ('Emit-lm', 'Calib-Emissive', False, lambda d: emitter(d, {'Luminance Units': 4, 'Luminance': 20000.0})),
    ('Emit-W', 'Calib-Emissive', False, lambda d: emitter(d, {'Luminance Units': 5, 'Luminance': 50.0})),
]

only = {a for a in sys.argv[2:] if not a.startswith('--')}
for name, src, profile, edit in VARIANTS:
    if only and name not in only:
        continue
    d = load(src)
    clean(d, profile)
    edit(d)
    path = os.path.join(scenes, f'Calib-L-{name}.duf')
    if os.path.exists(path) and '--force' not in sys.argv:
        continue   # it may carry the exposure calib-render.py tuned
    open(path, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(path)
