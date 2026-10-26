#!/usr/bin/env python3
"""ORBIS calibration scenes: variants of Calib-Materials.duf (eight spheres on
a plane under an HDRI) that each change one thing, written beside it.

  python3 scripts/dev/make-calib-scenes.py [<Calib-Materials.duf>]

- Calib-PBRSkin: the spheres in DAZ's PBRSkin shader — matte, dual lobe
  specular at reflectivity 0.25 / 0.5 / weight 0.25, top coat alone, and the
  Gladio outfit's cloth and armour (#14 follow-up: Gladiator's red cloth
  rendered pink, its armour too bright).
- Calib-Tone-*: Iray tone mapping off the calibrated defaults — Burn
  Highlights, Crush Blacks, Gamma — to check the tone LUT's curve (#14),
  fitted so far on Gladiator alone.
- Calib-DOF-*: depth of field, focus at 1.5 m with the spheres at ~4.6 m,
  f/4, f/8, f/16 — to check Iray's 10x aperture.

Renders are capped by samples, not time (scripts/dev/daz-render.sh): Iray is
then deterministic, so the tone variants share their noise pixel for pixel.
Existing scenes are kept (--force rewrites); dump and export the new ones
with scripts/dev/daz-dump.sh.
"""
import copy, glob, gzip, json, os, subprocess, sys

src = next((a for a in sys.argv[1:] if a.endswith('.duf')), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes/Calib-Materials.duf'
out_dir = os.path.dirname(src)


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


FORCE = '--force' in sys.argv


def save(d, name):
    """Write a scene unless it exists (it may carry a tuned exposure);
    --force rewrites. Returns its path, None when kept."""
    p = os.path.join(out_dir, name + '.duf')
    if os.path.exists(p) and not FORCE:
        return None
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)
    return p


def set_channel(channels, cid, value, typ=None):
    for c in channels:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    ch = {'id': cid, 'current_value': value}
    if typ:
        ch['type'] = typ
    channels.append({'channel': ch})


def node_channels(d, lib_id):
    lib = next(l for l in d['node_library'] if l['id'] == lib_id)
    ext = next(e for e in lib['extra'] if e['type'] == 'studio_node_channels')
    return ext['channels']


base = load(src)
spheres = [n for n in base['scene']['nodes'] if n['id'].startswith('pSphere')]
# Scene order is left to right only by x; sort so the variants read in a row.
xs = {l['id']: next(c['current_value'] for c in l['translation'] if c['id'] == 'x')
      for l in base['node_library'] if l['id'].startswith('pSphere')}
spheres.sort(key=lambda n: xs[n['url'].lstrip('#').replace('%28', '(').replace('%29', ')')])

PBRSKIN = '/data/Daz%203D/Built-in%20Content/Daz%20Iray%20PBRSkin/PBRSkin.dsf#PBRSkin'
RED = [0.45, 0.05, 0.04]
GREY = [0.6, 0.6, 0.6]
# label, base colour, channel overrides (PBRSkin ids, as in Gladiator.duf)
OFF = {'Dual Lobe Specular Enable': False, 'Top Coat Weight': 0.0, 'Metallicity Enable': False,
       'Translucency Enable': False, 'Translucency Weight': 0.0}
DL = {'Dual Lobe Specular Enable': True, 'Dual Lobe Specular Weight': 1.0, 'Dual Lobe Specular Reflectivity': 0.25,
      'Specular Lobe 1 Roughness': 0.6, 'Specular Lobe 2 Roughness Mult': 0.0, 'Dual Lobe Specular Ratio': 0.15}
TC = {'Top Coat Weight': 0.25, 'Top Coat Reflectivity': 0.5, 'Top Coat Roughness': 0.6721311}
VARIANTS = [
    ('P1 matte', RED, {}),
    ('P2 dual lobe 0.25', RED, DL),
    ('P3 dual lobe 0.5', RED, {**DL, 'Dual Lobe Specular Reflectivity': 0.5}),
    ('P4 dual lobe w0.25', RED, {**DL, 'Dual Lobe Specular Weight': 0.25}),
    ('P5 top coat', RED, TC),
    # Gladio maps its translucency colour with the base colour map; PBRSkin's
    # default (skin-pale) turned the sphere pink in Iray too.
    ('P6 Gladio cloth', RED, {**DL, **TC, 'Translucency Enable': True, 'Translucency Weight': 0.5,
                              'Translucency Color': RED}),
    ('P7 metal', GREY, {**DL, 'Metallicity Enable': True, 'Metallic Weight': 0.9, 'Specular Lobe 1 Roughness': 0.4}),
    ('P8 Gladio armour', GREY, {**DL, **TC, 'Metallicity Enable': True, 'Metallic Weight': 0.75,
                               'Specular Lobe 1 Roughness': 0.4, 'Translucency Enable': True, 'Translucency Weight': 0.85,
                               'Translucency Color': GREY}),
]
TYPES = {bool: 'bool', float: 'float', int: 'int', list: 'float_color'}

made = []

# --- PBRSkin spheres (renamed in the copy only: the other variants keep the
# base's labels, which the shared dump matches by)
d = copy.deepcopy(base)
geo_of = {n['id']: n['geometries'][0]['id'] for n in spheres}
d_nodes = {n['id']: n for n in d['scene']['nodes']}
pbr_labels = []
for s0, (label, colour, over) in zip(spheres, VARIANTS):
    n = d_nodes[s0['id']]
    n['label'] = label
    pbr_labels.append(label)
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    channels = []
    for k, v in {**OFF, **over}.items():
        set_channel(channels, k, v, TYPES[type(v)])
    m['url'] = PBRSKIN
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
    m['extra'] = [{'type': 'studio/material/daz_brick'}, {'type': 'studio_material_channels', 'channels': channels}]
made.append((save(d, 'Calib-PBRSkin'), pbr_labels))

# --- Iray Uber gloss (#15: Glossy Reflectivity is set in 128 scenes and not
# read): the spheres' own Uber materials, red, PBR metallicity / roughness.
UBER = [
    ('G1 matte', {'Glossy Layered Weight': 0.0}),
    ('G2 refl 0.5', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3}),
    ('G3 refl 0.25', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.25, 'Glossy Roughness': 0.3}),
    ('G4 refl 1', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 1.0, 'Glossy Roughness': 0.3}),
    ('G5 refl 0', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.0, 'Glossy Roughness': 0.3}),
    ('G6 rough 0.1', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.1}),
    ('G7 rough 0.6', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.6}),
    ('G8 dual lobe', {'Glossy Layered Weight': 0.0, 'Dual Lobe Specular Weight': 1.0, 'Dual Lobe Specular Reflectivity': 0.5,
                      'Specular Lobe 1 Roughness': 0.3}),
]
d = copy.deepcopy(base)
d_nodes = {n['id']: n for n in d['scene']['nodes']}
mlib = {m['id']: m for m in d['material_library']}
uber_labels = []
# One base material for all (the leftmost sphere's, M0 matte): the original
# spheres differ in more than their gloss (a tinted glossy colour, other
# mixing modes), which would ride along with each variant.
first = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[spheres[0]['id']])
base_extra = copy.deepcopy(mlib[first['url'].lstrip('#')]['extra'])
for s0, (label, over) in zip(spheres, UBER):
    n = d_nodes[s0['id']]
    n['label'] = label
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    uber_labels.append(label)
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': RED}}
    lm = mlib[m['url'].lstrip('#')]
    lm['extra'] = copy.deepcopy(base_extra)
    ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
    for k, v in {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
                 'Translucency Weight': 0.0, 'Refraction Weight': 0.0, **over}.items():
        set_channel(ch, k, v, TYPES.get(type(v), 'float'))
    lm['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': RED}}
made.append((save(d, 'Calib-Glossy'), uber_labels))

# --- Tone mapping
TONES = [('Burn05', {'Burn Highlights': 0.5}), ('Burn0', {'Burn Highlights': 0.0}),
         ('Crush01', {'Crush Blacks': 0.1}), ('Crush03', {'Crush Blacks': 0.3}),
         ('Gamma18', {'Gamma': 1.8}), ('Gamma26', {'Gamma': 2.6}),
         ('Gladiator', {'Burn Highlights': 0.0, 'Crush Blacks': 0.06985294, 'Gamma': 1.856618}),
         # ISO 200 at the same shutter and f-number: Iray (exposure ∝ ISO /
         # (N² S)) one stop brighter; DAZ's derived EV reads 14 there.
         ('ISO200', {'Film ISO': 200.0, 'Exposure Value': 14.0}),
         ('Default', {})]
for name, over in TONES:
    d = copy.deepcopy(base)
    ch = node_channels(d, 'Tonemapper Options')
    for k, v in over.items():
        set_channel(ch, k, v, 'float')
    made.append((save(d, 'Calib-Tone-' + name), None))

# --- Depth of field
for fstop in (4, 8, 16):
    d = copy.deepcopy(base)
    cam = next(l for l in d['node_library'] if l['id'] == 'Camera')
    cam['perspective'].update({'depth_of_field': True, 'focal_distance': 150.0, 'fstop': fstop})
    # DAZ renders by the camera's channels ("Aperature" is DAZ's spelling).
    ch = next(e for e in cam['extra'] if e['type'] == 'studio_node_channels')['channels']
    set_channel(ch, 'DOF', True, 'bool')
    set_channel(ch, 'Depth of Field', 150.0, 'float')
    set_channel(ch, 'Aperature', float(fstop), 'float')
    made.append((save(d, f'Calib-DOF-f{fstop}'), None))

# Scene dumps: scripts/dev/daz-dump.sh (DAZ evaluates each scene headless).
