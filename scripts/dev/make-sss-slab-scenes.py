#!/usr/bin/env python3
"""ORBIS calibration: Calib-SSSThick-* (#86) - light through thin slabs of
Iray's skin volume, against their thickness.

  python3 scripts/dev/make-sss-slab-scenes.py [--force] [<Calib-L-Distant-45.duf>]

Variants of Calib-L-Distant-45 (DAZ primitives only): its three 40 cm cubes
flattened to slabs a few millimetres thick, the distant light behind them
(5 degrees up, so the floor in front stays nearly dark), the camera as it
is: what the camera sees of a slab is the light the volume passes through
it. The slabs are Uber with no gloss, Translucency Weight 1 and a white
Translucency Color - the volume alone (Octane: OpenPBR subsurface 1, the
walk's colour the volume's A(alpha)). The sphere stays the scene's matte
grey. Why: a backlit ear in Octane passes 0.3 - 0.6 of Iray's red with the
same volume, and the deficit differs by figure (ear shape), not by w or Tc;
a slab of known thickness splits the medium from the geometry.

- Calib-SSSThick-A: 1, 3, 9 mm, the Genesis 8 skin volume (Victoria 8's:
  Transmitted Color over 0.125 cm, SSS Color over 0.015 cm, chromatic);
- Calib-SSSThick-B: 2, 5, 15 mm, the same volume;
- Calib-SSSThick-Grey: 1, 3, 9 mm, SSS Color grey 0.7 (another albedo);
- Calib-SSSThick-B-ev10: B two stops brighter (EV 10.1, the rest 12.1).

The slabs are props: the export gives them Universal, not the random walk;
the Octane side is a probe that puts the walk on them.

Existing scenes are kept (--force rewrites). Dump and export them with
scripts/dev/daz-dump.sh; copy them into calib/scenes with their references.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, os, sys

src = next((a for a in sys.argv[1:] if a.endswith('.duf')), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes/Calib-L-Distant-45.duf'
out_dir = os.path.dirname(src)
FORCE = '--force' in sys.argv
CUBE = 40.0  # cm, the primitive's depth (z -20 .. 20)
# Victoria 8's skin volume (sRGB colours, DAZ cm), as in make-calib-scenes.py.
VOLUME = {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
          'Glossy Layered Weight': 0.0, 'Refraction Weight': 0.0, 'Base Color Effect': 0,
          'Translucency Weight': 1.0, 'Translucency Color': [1.0, 1.0, 1.0], 'Thin Walled': False,
          'Transmitted Color': [0.9764706, 0.4823529, 0.3529412], 'Transmitted Measurement Distance': 0.125,
          'Scattering Measurement Distance': 0.015, 'SSS Mode': 1, 'SSS Color': [0.9764706, 0.6941177, 0.7607843],
          'SSS Direction': 0.0}
SCENES = {
    'Calib-SSSThick-A': ((1, 3, 9), {}),
    'Calib-SSSThick-B': ((2, 5, 15), {}),
    'Calib-SSSThick-Grey': ((1, 3, 9), {'SSS Color': [0.7, 0.7, 0.7]}),
    # Two stops brighter, in case the thick slabs sink into the shadows.
    'Calib-SSSThick-B-ev10': ((2, 5, 15), {}, 10.1),
}
LIGHT_ROTATION = {'x': -5.0, 'y': 180.0}
# Through 1 mm the slab passed almost all of the light: three stops darker
# than the base scene's EV 9.1, so the thinnest slab doesn't clip.
EXPOSURE = 12.1
TYPES = {bool: 'bool', float: 'float', int: 'int', list: 'float_color'}


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def set_channel(channels, cid, value):
    for c in channels:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            c['channel'].pop('image_file', None)
            return
    channels.append({'channel': {'id': cid, 'type': TYPES[type(value)], 'current_value': value}})


def set_axis(lib, key, axis, value):
    for c in lib.setdefault(key, []):
        if c['id'] == axis:
            c['current_value'] = value
            return
    lib[key].append({'id': axis, 'type': 'float', 'current_value': value})


base = load(src)
for name, (thick, over, *ev) in SCENES.items():
    p = os.path.join(out_dir, name + '.duf')
    if os.path.exists(p) and not FORCE:
        continue
    d = copy.deepcopy(base)
    nlib = {l['id']: l for l in d['node_library']}
    mlib = {m['id']: m for m in d['material_library']}
    cubes = sorted((n for n in d['scene']['nodes'] if n.get('label', '').startswith('Cube')), key=lambda n: n['label'])
    for n, mm in zip(cubes, thick):
        lib = nlib[n['url'].lstrip('#').replace('%28', '(').replace('%29', ')')]
        set_axis(lib, 'scale', 'z', mm / 10.0 / CUBE)
        n['label'] = lib['label'] = f'Slab {mm} mm'
        geo = '#' + n['geometries'][0]['id']
        for m in d['scene']['materials']:
            if m['geometry'] == geo:
                lm = mlib[m['url'].lstrip('#')]
                ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
                for k, v in {**VOLUME, **over}.items():
                    set_channel(ch, k, v)
                m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': [0.5, 0.5, 0.5]}}
    tone = next(e for e in nlib['Tonemapper Options']['extra'] if e.get('channels'))['channels']
    set_channel(tone, 'Exposure Value', ev[0] if ev else EXPOSURE)
    light = nlib['DistantLight']
    for axis, v in LIGHT_ROTATION.items():
        set_axis(light, 'rotation', axis, v)
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)
