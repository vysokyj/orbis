#!/usr/bin/env python3
"""ORBIS calibration: Calib-OmniHair-<variant> (#21) — DAZ's OmniHair.

  python3 scripts/dev/make-omnihair-scenes.py [--force]

Calib-Hair with its strand hair ("dForceCorporate Hairs") switched from
Blended Dual Lobe Hair to the built-in "OmniHair Root-Tip Blend" brick
shader (OmniSurface::OmniHairBase: Chiang's hair BSDF, its absorption from
melanin and a base colour). The material is the scene's own one with the
URL and channels swapped, its strand widths kept. Variants:

- blonde / brown / black: melanin 0.175 / 0.65 / 1 (OmniHair's presets),
  redness 0.5, white root and tip at Diffuse Strength 1 (melanin only)
- red:    melanin 0.4, redness 1
- dyed:   melanin 0.3, root and tip (0.8, 0.3, 0.2) — the base colour's
          absorption blended with the melanin's by √melanin
- wendy:  as the user's Wendy scene: melanin 1, Diffuse Strength 0.8
          (white × 0.8 ≠ 1: the blend applies), roughness 0.05
- highlight: brown with Highlight Weight 0.5, Highlight colour (0.9, 0.7, 0.5)
          (OmniHair's diffuse lobe)

The scenes wear a DAZ Store hair: they stay in My Library, never in
calib/scenes. Existing scenes are kept (--force rewrites).
"""
import copy, gzip, json, os, sys

SCENES = '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
URL = '/data/DAZ%203D/Built-in%20Content/OmniSurface/OmniHair%20Root-Tip%20Blend.dsf#OmniHair%20Root-Tip%20Blend'
HAIR = 'dForceCorporate Hairs'
BASE = {
    'Melanin': 0.65, 'Melanin Redness': 0.5, 'Diffuse Strength': 1.0,
    'Hair Root Color': [1, 1, 1], 'Hair Tip Color': [1, 1, 1],
    'Highlight Weight': 0.0, 'Highlight Root Color': [1, 1, 1], 'Tip Highlight Color': [1, 1, 1],
    'Roughness': 0.2, 'Roughness Tip': 0.2, 'Specular Reflection Shift': 3.0,
    'Azimuthal Roughness': 0.2, 'Azimuthal Roughness Tip': 0.2, 'IOR': 1.55,
}
VARIANTS = {
    'blonde': {'Melanin': 0.175},
    'brown': {},
    'black': {'Melanin': 1.0},
    'red': {'Melanin': 0.4, 'Melanin Redness': 1.0},
    'dyed': {'Melanin': 0.3, 'Hair Root Color': [0.8, 0.3, 0.2], 'Hair Tip Color': [0.8, 0.3, 0.2]},
    'wendy': {'Melanin': 1.0, 'Diffuse Strength': 0.8, 'Roughness': 0.05, 'Roughness Tip': 0.05},
    'highlight': {'Highlight Weight': 0.5, 'Highlight Root Color': [0.9, 0.7, 0.5], 'Tip Highlight Color': [0.9, 0.7, 0.5]},
}


def load(path):
    with open(path, 'rb') as f:
        return json.loads(gzip.decompress(f.read()))


def main():
    force = '--force' in sys.argv
    scene = load(f'{SCENES}/Calib-Hair.duf')
    for name, changes in VARIANTS.items():
        out = f'{SCENES}/Calib-OmniHair-{name}.duf'
        if os.path.exists(out) and not force:
            print(f'kept {out}')
            continue
        d = copy.deepcopy(scene)
        geos = {'#' + g['id'] for n in d['scene']['nodes'] if n.get('label') == HAIR for g in n.get('geometries', [])}
        values = dict(BASE, **changes)
        hit = 0
        for m in d['scene']['materials']:
            if m.get('geometry') not in geos:
                continue
            m['url'] = URL
            m.pop('diffuse', None)
            for x in m.get('extra', []):
                if x.get('type') == 'studio_material_channels' and isinstance(x.get('channels'), list):
                    # Keep the strand widths, replace the shader's channels.
                    kept = [c for c in x['channels'] if c.get('channel', c).get('id', '').startswith('Line ')]
                    x['channels'] = kept + [{'channel': {'id': k, 'type': 'float_color' if isinstance(v, list) else 'float', 'current_value': v, 'image': None}} for k, v in values.items()]
                    hit += 1
        if not hit:
            sys.exit(f'{name}: no brick channels on {HAIR}')
        with open(out, 'wb') as f:
            f.write(gzip.compress(json.dumps(d).encode()))
        print(f'wrote {out}')


main()
