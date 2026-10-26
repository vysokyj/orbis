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
The scene dump of Calib-Materials (the geometry doesn't change) is stored for
every variant, at format 2, for `orbis export`.
"""
import copy, glob, gzip, json, os, subprocess, sys

src = sys.argv[1] if len(sys.argv) > 1 else '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes/Calib-Materials.duf'
out_dir = os.path.dirname(src)


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, name):
    p = os.path.join(out_dir, name + '.duf')
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
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
    ('P6 Gladio cloth', RED, {**DL, **TC, 'Translucency Enable': True, 'Translucency Weight': 0.5}),
    ('P7 metal', GREY, {**DL, 'Metallicity Enable': True, 'Metallic Weight': 0.9, 'Specular Lobe 1 Roughness': 0.4}),
    ('P8 Gladio armour', GREY, {**DL, **TC, 'Metallicity Enable': True, 'Metallic Weight': 0.75,
                               'Specular Lobe 1 Roughness': 0.4, 'Translucency Enable': True, 'Translucency Weight': 0.85}),
]
TYPES = {bool: 'bool', float: 'float'}

made = []

# --- PBRSkin spheres
d = copy.deepcopy(base)
geo_of = {n['id']: n['geometries'][0]['id'] for n in spheres}
for n, (label, colour, over) in zip(spheres, VARIANTS):
    n['label'] = label
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    channels = []
    for k, v in {**OFF, **over}.items():
        set_channel(channels, k, v, TYPES[type(v)])
    m['url'] = PBRSKIN
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
    m['extra'] = [{'type': 'studio/material/daz_brick'}, {'type': 'studio_material_channels', 'channels': channels}]
# Scene node labels too (the dump matches by label).
for n in d['scene']['nodes']:
    for s in spheres:
        if n['id'] == s['id']:
            n['label'] = s['label']
made.append((save(d, 'Calib-PBRSkin'), [s['label'] for s in spheres]))

# --- Tone mapping
TONES = [('Burn05', {'Burn Highlights': 0.5}), ('Burn0', {'Burn Highlights': 0.0}),
         ('Crush01', {'Crush Blacks': 0.1}), ('Crush03', {'Crush Blacks': 0.3}),
         ('Gamma18', {'Gamma': 1.8}), ('Gamma26', {'Gamma': 2.6}),
         ('Gladiator', {'Burn Highlights': 0.0, 'Crush Blacks': 0.06985294, 'Gamma': 1.856618}),
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

# --- Scene dumps (format 2) for `orbis export`
dumps = sorted(glob.glob(os.path.expanduser('~/Library/Caches/orbis/dumps/Calib-Materials-*.json')))
dump = json.load(open(dumps[-1]))
dump['format_version'] = 2
dump.setdefault('instances', [])
orbis = os.path.join(os.path.dirname(__file__), '../../target/release/orbis')
for path, labels in made:
    dd = copy.deepcopy(dump)
    dd['scene'] = path
    if labels:
        geo = [n for n in dd['nodes'] if n['label'] != 'Plane']
        old = ['M0 matte', 'M0 gloss03', 'M0 gloss1', 'M1 rough', 'M1 smooth', 'M1 tinted', 'M2 weighted', 'TopCoat']
        by_label = {n['label']: n for n in geo}
        # Same left-to-right order as the scene's spheres.
        for o, new in zip(old, labels):
            by_label[o]['label'] = new
    tmp = path + '.dump.json'
    json.dump(dd, open(tmp, 'w'))
    subprocess.run([orbis, 'export', path, '--dump', tmp, '-o', '/dev/null'], check=False,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if os.path.exists(tmp):
        os.remove(tmp)
    print(path)
