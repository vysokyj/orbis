#!/usr/bin/env python3
"""ORBIS calibration: Calib-VDB-<variant> (#21) — VDB volumes.

  python3 scripts/dev/make-vdb-scenes.py [--force]

Calib-HDRI with the haze volume of the user's Gladiator scene added: a
node ("DTAS - Optimized Haze", DimensionTheory's Atmospheric Skies) whose
material is DAZ's "Simple OpenVDB MDL Volume" (Iray: the VDB's density
times the scattering / absorption multipliers, base::lookup_volume_
coefficients). Variants:

- base:   as in Gladiator (density 1000, relative to size, absorption 0.09)
- d100 / d10000: Density Multiplier 100 / 10000
- noabs:  Absorption Multiplier 0 (scattering only)
- abs:    Scattering Multiplier 0, Absorption Multiplier 1 (absorption only)
- norel:  Density Relative To Size off (the node's scale 0.1 no longer
          thins it)
- tint:   Scattering Multiplier (1, 0.6, 0.3)
- iso:    SSS Direction 0 (DTAS sets 0.85; the brick may leave Iray's
          anisotropy off regardless)

The haze is DAZ Store content: the scenes stay in My Library, never in
calib/scenes. Existing scenes are kept (--force rewrites).
"""
import copy, gzip, json, os, sys

SCENES = '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
VARIANTS = {
    'base': {},
    'd100': {'Density Multiplier': 100},
    'd10000': {'Density Multiplier': 10000},
    'noabs': {'Absorption Multiplier': [0, 0, 0]},
    'abs': {'Scattering Multiplier': [0, 0, 0], 'Absorption Multiplier': [1, 1, 1]},
    'norel': {'Density Relative To Size': False},
    'tint': {'Scattering Multiplier': [1, 0.6, 0.3]},
    'iso': {'SSS Direction': 0},
}


def load(path):
    with open(path, 'rb') as f:
        return json.loads(gzip.decompress(f.read()))


def haze(gladiator):
    """The haze node and its material, with ids no Calib-HDRI node uses."""
    mats = [m for m in gladiator['scene']['materials'] if 'Volume File' in json.dumps(m)]
    geo = mats[0]['geometry'].lstrip('#')
    node = next(n for n in gladiator['scene']['nodes'] if any(g['id'] == geo for g in n.get('geometries', [])))
    node = copy.deepcopy(node)
    for g in node['geometries']:
        if g['id'] == geo:
            g['id'] = 'vdb-geometry'
    mat = copy.deepcopy(mats[0])
    mat['id'] = 'vdb-Volume'
    mat['geometry'] = '#vdb-geometry'
    return node, mat


def main():
    force = '--force' in sys.argv
    base = load(f'{SCENES}/Calib-HDRI.duf')
    node, mat = haze(load(f'{SCENES}/Gladiator.duf'))
    for name, changes in VARIANTS.items():
        out = f'{SCENES}/Calib-VDB-{name}.duf'
        if os.path.exists(out) and not force:
            print(f'kept {out}')
            continue
        d = copy.deepcopy(base)
        m = copy.deepcopy(mat)
        for x in m.get('extra', []):
            if x.get('type') == 'studio_material_channels':
                for c in x['channels']:
                    c = c.get('channel', c)
                    if c.get('id') in changes:
                        c['current_value'] = changes[c['id']]
        d['scene']['nodes'].append(copy.deepcopy(node))
        d['scene']['materials'].append(m)
        with open(out, 'wb') as f:
            f.write(gzip.compress(json.dumps(d).encode()))
        print(f'wrote {out}')


main()
