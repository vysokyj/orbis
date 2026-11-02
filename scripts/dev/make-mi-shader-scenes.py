#!/usr/bin/env python3
"""ORBIS .mi sweeps for DAZ's other shaders (#62): channel variants of a
scene daz-build-shader-scene.dsa built with the presets on its spheres.
Read through DAZ's .mi export, not rendered.

  python3 scripts/dev/make-mi-shader-scenes.py [<scenes dir>]

- Calib-Mi-Hair (from Calib-Mi-Hair-base): Blended Dual Lobe Hair on
  spheres 1-4, OmniHair Root-Tip Blend on 5-8, each varied.
- Calib-Mi-DzDefault2 (from Calib-Mi-DzDefault, DS Defaults presets): the
  DAZ Default shader's channels, which DAZ turns into Iray Uber.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import gzip, json, os, sys

scenes = sys.argv[1] if len(sys.argv) > 1 else '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'

VARIANTS = {
    'Calib-Mi-Hair': ('Calib-Mi-Hair-base', {
        'S2': {'Hair Root Color': [0.3, 0.2, 0.1], 'Hair Tip Color': [0.8, 0.7, 0.5], 'Glossy Layer Weight': 0.8, 'base_roughness': 0.4},
        'S3': {'Highlight Weight': 0.5, 'highlight_roughness': 0.3, 'separation': 0.2, 'Highlight Root Color': [0.9, 0.6, 0.3], 'Tip Highlight Color': [0.4, 0.5, 0.9]},
        'S4': {'Anisotropy': 0.5, 'Anisotropy Rotations': 0.25, 'Cutout Opacity': 0.7, 'Root Transmission Color': [0.2, 0.4, 0.8], 'Tip Transmission Color': [0.9, 0.2, 0.1]},
        'S6': {'Melanin': 0.2, 'Melanin Redness': 0.8, 'Roughness': 0.4, 'Roughness Tip': 0.6},
        'S7': {'Highlight Weight': 0.5, 'Highlight Root Color': [0.9, 0.6, 0.3], 'Tip Highlight Color': [0.4, 0.5, 0.9], 'Specular Reflection Shift': 5.0,
               'Azimuthal Roughness': 0.4, 'Azimuthal Roughness Tip': 0.6},
        'S8': {'Diffuse Strength': 0.5, 'Hair Root Color': [0.3, 0.2, 0.1], 'Hair Tip Color': [0.8, 0.7, 0.5], 'Melanin': 0.9},
    }),
    'Calib-Mi-DzDefault2': ('Calib-Mi-DzDefault', {
        'S1': {'glossiness': 0.3, 'specular_strength': 0.5, 'specular': [0.8, 0.6, 0.4]},
        'S2': {'diffuse_strength': 0.5, 'glossiness': 0.7},
        'S3': {'reflection_strength': 0.6, 'reflection': [0.9, 0.8, 0.7], 'glossiness': 0.9},
        'S4': {'refraction_strength': 0.8, 'refraction': [0.6, 0.8, 1.0], 'ior': 1.4},
        'S5': {'transparency': 0.4, 'glossiness': 0.5},
        'S6': {'ambient': [0.5, 0.4, 0.3], 'ambient_strength': 2.0},
        'S7': {'specular_strength': 0.0, 'glossiness': 0.5},
        'S8': {'specular': [1.0, 1.0, 1.0], 'specular_strength': 1.0, 'glossiness': 0.95, 'Lighting Model': 1},
    }),
}

TYPES = {list: 'float_color', bool: 'bool', int: 'int', float: 'float'}

for name, (src, variants) in VARIANTS.items():
    d = json.load(gzip.open(os.path.join(scenes, src + '.duf')))
    geo = {g['id']: n.get('label', '') for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    for m in d['scene']['materials']:
        label = geo.get(m['geometry'].rsplit('#', 1)[-1], '')
        over = variants.get(label.split(' ')[0], {})
        chs = next((e for e in m.get('extra', []) if e.get('channels') is not None), None)
        lib = next((l for l in d.get('material_library', []) if '#' + l['id'] == m.get('url')), {})
        for k, v in over.items():
            # The DAZ Default shader keeps its channels at the top level
            # (`"glossiness": {"channel": …}`), the others in `extra`.
            if k in lib or k in m:
                c = m.setdefault(k, {'channel': {'id': k, 'type': TYPES[type(v)]}})['channel']
                c['current_value'] = v
                continue
            if chs is None:
                chs = {'type': 'studio_material_channels', 'channels': []}
                m.setdefault('extra', []).append(chs)
            c = next((c['channel'] for c in chs['channels'] if c['channel']['id'] == k), None)
            if c is None:
                c = {'id': k, 'type': TYPES[type(v)]}
                chs['channels'].append({'channel': c})
            c['current_value'] = v
    out = os.path.join(scenes, name + '.duf')
    open(out, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(out)
