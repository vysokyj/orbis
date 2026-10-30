#!/usr/bin/env python3
"""ORBIS calibration: channel variants of a Calib-Skin scene (#26, #28).

  python3 scripts/dev/make-skin-variants.py <figure> <light> [<variant>...]

Writes Calib-Skin-<figure>-<light>-<variant>.duf beside
Calib-Skin-<figure>-<light>.duf (make-skin-scenes.py) with the variant's
channels set on every material of the figure and on the skin sphere — so
Iray shows what one channel does to a real skin. The scenes use DAZ Store
skins: they stay in My Library, like the scenes they come from. Without
variants: all of them.
"""
import copy, gzip, json, os, sys

SCENES = '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
VARIANTS = {
    # Volume scattering (#28). Chromatic SSS scatters −ln(SSS Color) per
    # Scattering Measurement Distance; mono SSS Amount per distance (DAZ's
    # MDL, add_uber_volume).
    'ssscgrey': {'SSS Color': [0.7, 0.7, 0.7]},
    'ssscblue': {'SSS Color': [0.6, 0.75, 0.95]},
    'ssscdeep': {'SSS Color': [0.995, 0.95, 0.95]},
    'mono': {'SSS Mode': 0},
    'monoa05': {'SSS Mode': 0, 'SSS Amount': 0.5},
    'tintblue': {'SSS Reflectance Tint': [0.7, 0.85, 1.0]},
    'tintred': {'SSS Reflectance Tint': [1.0, 0.7, 0.6]},
    # What keeps the Genesis 8 base skin (mono, Base Color Effect 1, SSS
    # Direction −0.65, w 0.5) off the baked rule (#28): one at a time.
    'bce0': {'Base Color Effect': 0},
    'sssdir0': {'SSS Direction': 0.0},
    'w09': {'Translucency Weight': 0.9},
    # Dual lobe specular (#26): Ratio weighs lobe 1 against lobe 2.
    'dlratio1': {'Dual Lobe Specular Ratio': 1.0},
    'dlratio0': {'Dual Lobe Specular Ratio': 0.0},
    'dlsharp2': {'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.1},
}


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, p):
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)


def channels(m):
    for e in m.setdefault('extra', []):
        if e.get('type') == 'studio_material_channels':
            return e['channels']
    e = {'type': 'studio_material_channels', 'channels': []}
    m['extra'].append(e)
    return e['channels']


def set_channel(chs, cid, value):
    typ = 'float_color' if isinstance(value, list) else 'float'
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            c['channel'].pop('image_file', None)
            return
    chs.append({'channel': {'id': cid, 'type': typ, 'current_value': value}})


if len(sys.argv) < 3:
    sys.exit(__doc__)
figure, light = sys.argv[1:3]
wanted = sys.argv[3:] or list(VARIANTS)
src = f'{SCENES}/Calib-Skin-{figure}-{light}.duf'
base = load(src)
by_label = {n.get('label'): n for n in base['scene']['nodes']}
geos = {'#' + by_label[k]['geometries'][0]['id'] for k in ('Genesis 8 Female', 'Skin Sphere')}
for v in wanted:
    d = copy.deepcopy(base)
    lib = {m['id']: m for m in d['material_library']}
    for m in d['scene']['materials']:
        if m['geometry'] in geos:
            for k, x in VARIANTS[v].items():
                set_channel(channels(m), k, x)
                set_channel(channels(lib[m['url'].lstrip('#')]), k, x)
    save(d, f'{SCENES}/Calib-Skin-{figure}-{light}-{v}.duf')
