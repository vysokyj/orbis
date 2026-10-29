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
- Calib-Disp: displacement (#24) by uniform maps, measured on the
  spheres' silhouettes.
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

# --- Iray Uber translucency / subsurface on props (#15: SSS Color / Amount
# set in 74-113 scenes and not read; props skip the random-walk skin). A
# skin-like base, one material for all.
SKINLIKE = [0.8, 0.55, 0.45]
SSS = [
    ('S1 opaque', {'Translucency Weight': 0.0}),
    ('S2 transl 0.5', {'Translucency Weight': 0.5}),
    ('S3 transl 1', {'Translucency Weight': 1.0}),
    ('S4 thin walled', {'Translucency Weight': 0.5, 'Thin Walled': True}),
    ('S5 red transmit', {'Translucency Weight': 0.5, 'Transmitted Color': [0.9, 0.3, 0.2], 'Transmitted Measurement Distance': 1.0}),
    ('S6 sss amount', {'Translucency Weight': 0.5, 'SSS Amount': 0.5, 'SSS Color': [0.9, 0.6, 0.5]}),
    ('S7 chromatic', {'Translucency Weight': 0.5, 'SSS Mode': 1, 'Transmitted Color': [0.9, 0.3, 0.2]}),
    ('S8 short scatter', {'Translucency Weight': 0.5, 'Scattering Measurement Distance': 0.1}),
]
d = copy.deepcopy(base)
d_nodes = {n['id']: n for n in d['scene']['nodes']}
mlib = {m['id']: m for m in d['material_library']}
first = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[spheres[0]['id']])
base_extra = copy.deepcopy(mlib[first['url'].lstrip('#')]['extra'])
sss_labels = []
for s0, (label, over) in zip(spheres, SSS):
    n = d_nodes[s0['id']]
    n['label'] = label
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    sss_labels.append(label)
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': SKINLIKE}}
    lm = mlib[m['url'].lstrip('#')]
    lm['extra'] = copy.deepcopy(base_extra)
    lm['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': SKINLIKE}}
    ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
    for k, v in {'Base Mixing': 0, 'Glossy Layered Weight': 0.0, 'Translucency Color': SKINLIKE, **over}.items():
        set_channel(ch, k, v, TYPES.get(type(v), 'float'))
made.append((save(d, 'Calib-SSS'), sss_labels))

# --- Iray Uber refraction (#9: glass and water — colour, absorption, thin
# walls, roughness): the spheres' Uber material, grey, refraction variants.
GREY_GLASS = [0.6, 0.6, 0.6]
GLASS = [
    ('R1 glass', {'Refraction Weight': 1.0}),
    ('R2 water', {'Refraction Weight': 1.0, 'Refraction Index': 1.33}),
    ('R3 green', {'Refraction Weight': 1.0, 'Share Glossy Inputs': False, 'Refraction Color': [0.3, 0.9, 0.4]}),
    ('R4 absorb', {'Refraction Weight': 1.0, 'Transmitted Color': [0.2, 0.5, 0.9], 'Transmitted Measurement Distance': 5.0}),
    ('R5 thin', {'Refraction Weight': 1.0, 'Thin Walled': True}),
    ('R6 rough', {'Refraction Weight': 1.0, 'Share Glossy Inputs': False, 'Refraction Roughness': 0.3}),
    ('R7 half', {'Refraction Weight': 0.5}),
    ('R8 glossy', {'Refraction Weight': 1.0, 'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.05}),
]
d = copy.deepcopy(base)
d_nodes = {n['id']: n for n in d['scene']['nodes']}
mlib = {m['id']: m for m in d['material_library']}
first = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[spheres[0]['id']])
base_extra = copy.deepcopy(mlib[first['url'].lstrip('#')]['extra'])
glass_labels = []
for s0, (label, over) in zip(spheres, GLASS):
    n = d_nodes[s0['id']]
    n['label'] = label
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    glass_labels.append(label)
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': GREY_GLASS}}
    lm = mlib[m['url'].lstrip('#')]
    lm['extra'] = copy.deepcopy(base_extra)
    lm['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': GREY_GLASS}}
    ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
    for k, v in {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
                 'Translucency Weight': 0.0, 'Glossy Layered Weight': 0.0, 'Thin Walled': False, 'Refraction Index': 1.5,
                 'Share Glossy Inputs': True, **over}.items():
        set_channel(ch, k, v, TYPES.get(type(v), 'float'))
made.append((save(d, 'Calib-Glass'), glass_labels))

# --- Iray Base Mixing 1 / 2 and Top Coat as Fetroom's room surfaces use
# them (#10: walls 2x, weighted surfaces 0.81 in Octane). A synthetic tile
# texture (written beside the scenes' library, ours) stands in for the
# walls' map, as their Diffuse and Glossy Color.
TILES = '/Runtime/Textures/ORBIS/calib-tiles.png'
tiles_path = os.path.join(os.path.dirname(out_dir), TILES.lstrip('/'))
if not os.path.exists(tiles_path):
    from PIL import Image
    import numpy as np
    os.makedirs(os.path.dirname(tiles_path), exist_ok=True)
    yy, xx = np.mgrid[0:512, 0:512]
    grout = ((xx % 128) < 6) | ((yy % 128) < 6)
    tone = 0.55 + 0.25 * (((xx // 128) + (yy // 128)) % 2) - 0.1 * np.sin(xx / 9.0) * np.cos(yy / 13.0)
    rgb = np.stack([tone, tone * 0.93, tone * 0.82], -1)
    rgb[grout] = 0.25
    Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8)).save(tiles_path)
WALL = {'Base Mixing': 1, 'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.7419355, 'Glossiness': 0.08888889}
MIXING = [
    ('X1 wall flat', [0.6, 0.6, 0.6], None, {**WALL}),
    ('X2 wall tex', [1.0, 1.0, 1.0], TILES, {**WALL, ('Glossy Color', 'image'): TILES}),
    ('X3 wall tex diffuse', [1.0, 1.0, 1.0], TILES, {**WALL}),
    ('X4 inner topcoat', [0.835, 0.776, 0.71], None, {'Base Mixing': 1, 'Glossy Layered Weight': 1.0, 'Glossy Color': [0.204, 0.204, 0.204],
                                                     'Glossiness': 0.5, 'Top Coat Weight': 1.0, 'Top Coat Glossiness': 0.546875}),
    ('X5 ceiling', [0.545, 0.576, 0.627], None, {'Base Mixing': 2, 'Diffuse Weight': 0.5, 'Glossy Weight': 0.6, 'Glossy Color': [0.67, 0.70, 0.75],
                                                 'Glossiness': 1.0, 'Top Coat Weight': 0.8, 'Top Coat Glossiness': 1.0}),
    ('X6 jamb', [0.0, 0.0, 0.0], None, {'Base Mixing': 2, 'Diffuse Weight': 0.5, 'Glossy Weight': 0.6, 'Glossy Color': [0.094, 0.094, 0.094],
                                        'Glossiness': 1.0, 'Top Coat Weight': 0.8, 'Top Coat Glossiness': 1.0}),
    ('X7 weighted', [0.6, 0.6, 0.6], None, {'Base Mixing': 2, 'Diffuse Weight': 1.0, 'Glossy Weight': 0.5, 'Glossiness': 0.7}),
    ('X8 weighted diffuse', [0.6, 0.6, 0.6], None, {'Base Mixing': 2, 'Diffuse Weight': 1.0, 'Glossy Weight': 0.0}),
]
d = copy.deepcopy(base)
d_nodes = {n['id']: n for n in d['scene']['nodes']}
mlib = {m['id']: m for m in d['material_library']}
first = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[spheres[0]['id']])
base_extra = copy.deepcopy(mlib[first['url'].lstrip('#')]['extra'])
mixing_labels = []
for s0, (label, colour, diffuse_map, over) in zip(spheres, MIXING):
    n = d_nodes[s0['id']]
    n['label'] = label
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
    lib['label'] = label
    mixing_labels.append(label)
    m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
    lm = mlib[m['url'].lstrip('#')]
    diffuse = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
    if diffuse_map:
        diffuse['channel']['image_file'] = diffuse_map
    m['diffuse'] = copy.deepcopy(diffuse)
    # Keep the library's channel definition (mappable, …): a bare channel
    # there made DAZ drop the diffuse map.
    lm.setdefault('diffuse', {'channel': {'id': 'diffuse', 'type': 'float_color'}})['channel']['current_value'] = colour
    lm['extra'] = copy.deepcopy(base_extra)
    ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
    plain = {'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0, 'Translucency Weight': 0.0,
             'Refraction Weight': 0.0, 'Glossy Color': [1.0, 1.0, 1.0], 'Glossy Reflectivity': 0.5, 'Diffuse Weight': 1.0, 'Glossy Weight': 0.0}
    for k, v in {**plain, **over}.items():
        if isinstance(k, tuple):
            # DAZ reads a map from the scene material's channel (and lists
            # the image in image_library), as it saves them.
            scene_ch = next(e for e in m.setdefault('extra', []) if e.get('type') == 'studio_material_channels') \
                if any(e.get('type') == 'studio_material_channels' for e in m.get('extra', [])) else None
            if scene_ch is None:
                scene_ch = {'type': 'studio_material_channels', 'channels': []}
                m['extra'].append(scene_ch)
            scene_ch['channels'].append({'channel': {'id': k[0], 'type': 'float_color', 'current_value': [1.0, 1.0, 1.0], 'image_file': v}})
        else:
            set_channel(ch, k, v, TYPES.get(type(v), 'float'))
d.setdefault('image_library', []).append({'id': 'calib-tiles.png', 'name': 'calib-tiles.png', 'map_gamma': 0,
                                          'map': [{'url': TILES, 'label': 'calib-tiles.png'}]})
made.append((save(d, 'Calib-Mixing'), mixing_labels))

# --- Feature spheres for the coverage issues (#22–#43): eight Uber spheres
# each, one channel family varied, everything else plain (no translucency,
# refraction, top coat, dual lobe); the base colour and gloss per scene.
def uber_spheres(name, colour, plain, variants):
    d = copy.deepcopy(base)
    d_nodes = {n['id']: n for n in d['scene']['nodes']}
    mlib = {m['id']: m for m in d['material_library']}
    first = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[spheres[0]['id']])
    base_extra = copy.deepcopy(mlib[first['url'].lstrip('#')]['extra'])
    labels = []
    for s0, (label, over) in zip(spheres, variants):
        n = d_nodes[s0['id']]
        n['label'] = label
        lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
        lib['label'] = label
        labels.append(label)
        m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
        m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
        lm = mlib[m['url'].lstrip('#')]
        lm['extra'] = copy.deepcopy(base_extra)
        lm.setdefault('diffuse', {'channel': {'id': 'diffuse', 'type': 'float_color'}})['channel']['current_value'] = colour
        ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
        for k, v in {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
                     'Translucency Weight': 0.0, 'Refraction Weight': 0.0, **plain, **over}.items():
            if isinstance(k, tuple):
                # A map: on the scene material's channel, listed in
                # image_library, as DAZ saves them (see Calib-Mixing).
                extra = m.setdefault('extra', [])
                sc = next((e for e in extra if e.get('type') == 'studio_material_channels'), None)
                if sc is None:
                    sc = {'type': 'studio_material_channels', 'channels': []}
                    extra.append(sc)
                sc['channels'].append({'channel': {'id': k[0], 'type': 'float', 'current_value': over.get(k[0], 1.0), 'image_file': v}})
                lib_images = d.setdefault('image_library', [])
                if not any(i['id'] == v for i in lib_images):
                    lib_images.append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
            else:
                set_channel(ch, k, v, TYPES.get(type(v), 'float'))
    made.append((save(d, name), labels))


GREY_SPHERE = [0.5, 0.5, 0.5]
GLOSS = {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3}
# Glossy anisotropy (#23)
uber_spheres('Calib-Aniso', GREY_SPHERE, {**GLOSS, 'Glossy Roughness': 0.4}, [
    ('A1 iso', {}),
    ('A2 aniso 0.5', {'Glossy Anisotropy': 0.5}),
    ('A3 aniso 1', {'Glossy Anisotropy': 1.0}),
    ('A4 aniso 0.5 rot 0.25', {'Glossy Anisotropy': 0.5, 'Glossy Anisotropy Rotations': 0.25}),
    ('A5 aniso 1 rot 0.25', {'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.25}),
    ('A6 metal aniso 0.8', {'Metallic Weight': 1.0, 'Glossy Anisotropy': 0.8}),
    ('A7 top coat aniso 1', {'Glossy Layered Weight': 0.0, 'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.3, 'Top Coat Anisotropy': 1.0}),
    ('A8 rough 0.1 aniso 1', {'Glossy Roughness': 0.1, 'Glossy Anisotropy': 1.0}),
])
# Top Coat layering modes (#25): 0 Reflectivity, 1 Weighted, 2 Fresnel, 3 Custom Curve
TC = {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.1, 'Top Coat Glossiness': 0.9}
uber_spheres('Calib-TopCoat', [0.45, 0.05, 0.04], TC, [
    ('T1 refl 0.5', {'Top Coat Layering Mode': 0, 'Reflectivity': 0.5}),
    ('T2 refl 1', {'Top Coat Layering Mode': 0, 'Reflectivity': 1.0}),
    ('T3 weighted', {'Top Coat Layering Mode': 1}),
    ('T4 weighted w0.4', {'Top Coat Layering Mode': 1, 'Top Coat Weight': 0.4}),
    ('T5 fresnel 1.5', {'Top Coat Layering Mode': 2, 'Top Coat IOR': 1.5}),
    ('T6 fresnel 2', {'Top Coat Layering Mode': 2, 'Top Coat IOR': 2.0}),
    ('T7 curve', {'Top Coat Layering Mode': 3, 'Top Coat Curve Normal': 0.04, 'Top Coat Curve Grazing': 1.0, 'Top Coat Curve Exponent': 5.0}),
    ('T8 curve flat', {'Top Coat Layering Mode': 3, 'Top Coat Curve Normal': 0.3, 'Top Coat Curve Grazing': 0.3, 'Top Coat Curve Exponent': 5.0}),
])
# Dual lobe details (#26)
DL = {'Dual Lobe Specular Weight': 1.0, 'Dual Lobe Specular Reflectivity': 0.5, 'Specular Lobe 1 Roughness': 0.3}
uber_spheres('Calib-DualLobe', [0.45, 0.05, 0.04], DL, [
    ('D1 lobe 1 only', {'Dual Lobe Specular Ratio': 1.0}),
    ('D2 ratio 0.5 r2 0.6', {'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.6}),
    ('D3 ratio 0.15 r2 0.6', {'Dual Lobe Specular Ratio': 0.15, 'Specular Lobe 2 Roughness': 0.6}),
    ('D4 ratio 0.5 r2 0.1', {'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.1}),
    ('D5 lobe 2 only r 0.6', {'Dual Lobe Specular Ratio': 0.0, 'Specular Lobe 2 Roughness': 0.6}),
    ('D6 refl 0.25', {'Dual Lobe Specular Reflectivity': 0.25, 'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.6}),
    ('D7 refl 1', {'Dual Lobe Specular Reflectivity': 1.0, 'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.6}),
    ('D8 weight 0.5', {'Dual Lobe Specular Weight': 0.5, 'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.6}),
])
# Colour effects (#27): 0 Scatter Only, 1 Scatter & Transmit, 2 Scatter & Transmit Intensity
TR = {'Translucency Weight': 0.5, 'Translucency Color': [0.9, 0.6, 0.4], 'Thin Walled': True}
uber_spheres('Calib-ColorEffect', [0.3, 0.5, 0.8], TR, [
    ('C1 base 0', {'Base Color Effect': 0}),
    ('C2 base 1', {'Base Color Effect': 1}),
    ('C3 base 2', {'Base Color Effect': 2}),
    ('C4 glass glossy 0', {'Translucency Weight': 0.0, 'Refraction Weight': 1.0, 'Glossy Layered Weight': 1.0, 'Glossy Color': [0.9, 0.3, 0.2], 'Glossy Color Effect': 0}),
    ('C5 glass glossy 1', {'Translucency Weight': 0.0, 'Refraction Weight': 1.0, 'Glossy Layered Weight': 1.0, 'Glossy Color': [0.9, 0.3, 0.2], 'Glossy Color Effect': 1}),
    ('C6 coat 0', {'Top Coat Weight': 1.0, 'Top Coat Color': [0.9, 0.3, 0.2], 'Top Coat Color Effect': 0}),
    ('C7 coat 1', {'Top Coat Weight': 1.0, 'Top Coat Color': [0.9, 0.3, 0.2], 'Top Coat Color Effect': 1}),
    ('C8 mode1 specular', {'Translucency Weight': 0.0, 'Base Mixing': 1, 'Glossy Layered Weight': 1.0, 'Glossy Specular': [0.9, 0.3, 0.2], 'Glossiness': 0.8}),
])
# Base Mixing 1, specular / glossiness (#52): the Glossy Specular colour,
# the layer's weight, and Morgan Hair's glossy and top coat (a card hair in
# this mode rendered white in Octane, honey blond in Iray).
SPEC = {'Base Mixing': 1, 'Glossy Layered Weight': 1.0, 'Glossiness': 0.7, 'Glossy Color': [1.0, 1.0, 1.0]}
MORGAN = {'Glossy Color': [0.941, 0.914, 0.820], 'Glossy Specular': [0.722, 0.702, 0.616], 'Glossiness': 0.808,
          'Glossy Layered Weight': 0.98, 'Glossy Color Effect': 2}
uber_spheres('Calib-SpecMode', [0.5, 0.5, 0.5], SPEC, [
    ('S1 mode0 refl 0.5', {'Base Mixing': 0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3}),
    ('S2 spec 0', {'Glossy Specular': [0.0, 0.0, 0.0]}),
    ('S3 spec 0.2', {'Glossy Specular': [0.2, 0.2, 0.2]}),
    ('S4 spec 0.5', {'Glossy Specular': [0.5, 0.5, 0.5]}),
    ('S5 spec 1', {'Glossy Specular': [1.0, 1.0, 1.0]}),
    ('S6 spec 0.5 w0.5', {'Glossy Specular': [0.5, 0.5, 0.5], 'Glossy Layered Weight': 0.5}),
    ('S7 morgan', MORGAN),
    ('S8 morgan coat', {**MORGAN, 'Top Coat Weight': 0.975, 'Top Coat Color': [0.522, 0.494, 0.443], 'Top Coat Color Effect': 2,
                        'Top Coat Glossiness': 0.532}),
])
# Backscattering (#29)
uber_spheres('Calib-Backscatter', [0.45, 0.05, 0.04], {}, [
    ('B1 none', {}),
    ('B2 w0.25', {'Backscattering Weight': 0.25, 'Backscattering Roughness': 0.3}),
    ('B3 w0.5', {'Backscattering Weight': 0.5, 'Backscattering Roughness': 0.3}),
    ('B4 w1', {'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.3}),
    ('B5 w1 rough 0.7', {'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.7}),
    ('B6 w1 white', {'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.3, 'Backscattering Color': [1.0, 1.0, 1.0]}),
    ('B7 w1 blue', {'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.3, 'Backscattering Color': [0.2, 0.4, 1.0]}),
    ('B8 w1 aniso', {'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.3, 'Backscattering Anisotropy': 0.8}),
])
# Metallic flakes (#30)
FL = {'Metallic Flakes Weight': 1.0, 'Metallic Flakes Color': [1.0, 1.0, 1.0], 'Metallic Flakes Size': 0.001, 'Metallic Flakes Density': 1.0,
      'Metallic Flakes Roughness': 0.2, 'Metallic Flakes Strength': 1.0, 'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.05}
uber_spheres('Calib-Flakes', [0.05, 0.1, 0.4], FL, [
    ('F1 none', {'Metallic Flakes Weight': 0.0}),
    ('F2 default', {}),
    ('F3 size 0.005', {'Metallic Flakes Size': 0.005}),
    ('F4 density 0.3', {'Metallic Flakes Density': 0.3}),
    ('F5 rough 0.6', {'Metallic Flakes Roughness': 0.6}),
    ('F6 gold', {'Metallic Flakes Color': [1.0, 0.75, 0.3]}),
    ('F7 weight 0.5', {'Metallic Flakes Weight': 0.5}),
    ('F8 strength 0.3', {'Metallic Flakes Strength': 0.3}),
])
# Thin film (#31)
uber_spheres('Calib-ThinFilm', [0.05, 0.05, 0.05], {**GLOSS, 'Glossy Roughness': 0.1}, [
    ('H1 none', {}),
    ('H2 base 300', {'Base Thin Film': 300.0, 'Base Thin Film IOR': 1.5}),
    ('H3 base 500', {'Base Thin Film': 500.0, 'Base Thin Film IOR': 1.5}),
    ('H4 base 800', {'Base Thin Film': 800.0, 'Base Thin Film IOR': 1.5}),
    ('H5 base 500 ior 1.3', {'Base Thin Film': 500.0, 'Base Thin Film IOR': 1.3}),
    ('H6 coat 500', {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.05, 'Top Coat Thin Film': 500.0, 'Top Coat Thin Film IOR': 1.5}),
    ('H7 metal 500', {'Metallic Weight': 1.0, 'Base Thin Film': 500.0, 'Base Thin Film IOR': 1.5}),
    ('H8 base 1200', {'Base Thin Film': 1200.0, 'Base Thin Film IOR': 1.5}),
])
# Diffuse overlay, roughness, strength (#32)
uber_spheres('Calib-Diffuse', [0.45, 0.05, 0.04], {}, [
    ('O1 plain', {}),
    ('O2 roughness 0.5', {'Diffuse Roughness': 0.5}),
    ('O3 roughness 1', {'Diffuse Roughness': 1.0}),
    ('O4 strength 0.5', {'Diffuse Strength': 0.5}),
    ('O5 overlay 0.5 white', {'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('O6 overlay 1 blue', {'Diffuse Overlay Weight': 1.0, 'Diffuse Overlay Color': [0.1, 0.3, 0.9]}),
    ('O7 overlay squared', {'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Weight Squared': True, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('O8 overlay rough 1', {'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Color': [1.0, 1.0, 1.0], 'Diffuse Overlay Roughness': 1.0}),
])
# Dispersion (#33)
GL = {'Refraction Weight': 1.0, 'Refraction Index': 1.5, 'Thin Walled': False}
uber_spheres('Calib-Dispersion', [0.6, 0.6, 0.6], GL, [
    ('X1 abbe 0', {}),
    ('X2 abbe 60', {'Abbe': 60.0}),
    ('X3 abbe 40', {'Abbe': 40.0}),
    ('X4 abbe 20', {'Abbe': 20.0}),
    ('X5 abbe 10', {'Abbe': 10.0}),
    ('X6 ior 1.8 abbe 20', {'Refraction Index': 1.8, 'Abbe': 20.0}),
    ('X7 ior 2.4 abbe 30', {'Refraction Index': 2.4, 'Abbe': 30.0}),
    ('X8 ior 1.33 abbe 55', {'Refraction Index': 1.33, 'Abbe': 55.0}),
])

# --- Displacement (#24): uniform maps (synthetic, ours) push the whole
# sphere along its normals, so the silhouette measures the amount: Strength
# × the map's value between Minimum and Maximum Displacement (cm), and
# whether the map is read linearly (grey 0.5 = 128).
DISP = {name: f'/Runtime/Textures/ORBIS/calib-disp-{name}.png' for name in ('white', 'grey', 'black')}
for name, path in DISP.items():
    p = os.path.join(os.path.dirname(out_dir), path.lstrip('/'))
    if not os.path.exists(p):
        from PIL import Image
        os.makedirs(os.path.dirname(p), exist_ok=True)
        Image.new('L', (64, 64), {'white': 255, 'grey': 128, 'black': 0}[name]).save(p)
# How the map's value is read: grey levels, an RGB PNG, a JPEG, and the
# image's own gamma 1 (`map_gamma`, set below) — all over 0..10 cm.
for name, (mode, level, ext) in {'grey-rgb': ('RGB', 128, 'png'), 'grey64': ('L', 64, 'png'), 'grey192': ('L', 192, 'png'),
                                  'grey-jpg': ('L', 128, 'jpg'), 'grey-g1': ('L', 128, 'png')}.items():
    DISP[name] = f'/Runtime/Textures/ORBIS/calib-disp-{name}.{ext}'
    p = os.path.join(os.path.dirname(out_dir), DISP[name].lstrip('/'))
    if not os.path.exists(p):
        from PIL import Image
        Image.new(mode, (64, 64), (level,) * 3 if mode == 'RGB' else level).save(p, quality=100) if ext == 'jpg' else \
            Image.new(mode, (64, 64), (level,) * 3 if mode == 'RGB' else level).save(p)
R10 = {'Displacement Strength': 1.0, 'Minimum Displacement': 0.0, 'Maximum Displacement': 10.0}
uber_spheres('Calib-Disp-Map', GREY_SPHERE, {}, [
    ('M1 none', {}),
    ('M2 grey', {('Displacement Strength', 'image'): DISP['grey'], **R10}),
    ('M3 grey rgb', {('Displacement Strength', 'image'): DISP['grey-rgb'], **R10}),
    ('M4 grey 64', {('Displacement Strength', 'image'): DISP['grey64'], **R10}),
    ('M5 grey 192', {('Displacement Strength', 'image'): DISP['grey192'], **R10}),
    ('M6 grey jpg', {('Displacement Strength', 'image'): DISP['grey-jpg'], **R10}),
    ('M7 grey gamma 1', {('Displacement Strength', 'image'): DISP['grey-g1'], **R10}),
    ('M8 white', {('Displacement Strength', 'image'): DISP['white'], **R10}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] == DISP['grey-g1']:
            i['map_gamma'] = 1
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Data maps (#49): does DAZ decode a grey 128 map at 2.2 on other float
# channels too (as on displacement)? Each map with no gamma of its own
# (automatic) beside the same grey at `map_gamma` 1 and a plain 0.5.
GREY, GREY1 = DISP['grey'], DISP['grey-g1']
DG_GLOSS = {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.05}
uber_spheres('Calib-DataGamma', [0.6, 0.6, 0.6], {}, [
    ('G1 opacity map', {('Cutout Opacity', 'image'): GREY, 'Cutout Opacity': 1.0}),
    ('G2 opacity map g1', {('Cutout Opacity', 'image'): GREY1, 'Cutout Opacity': 1.0}),
    ('G3 opacity 0.5', {'Cutout Opacity': 0.5}),
    ('G4 metal map', {('Metallic Weight', 'image'): GREY, 'Metallic Weight': 1.0, **DG_GLOSS}),
    ('G5 metal map g1', {('Metallic Weight', 'image'): GREY1, 'Metallic Weight': 1.0, **DG_GLOSS}),
    ('G6 metal 0.5', {'Metallic Weight': 0.5, **DG_GLOSS}),
    ('G7 rough map', {('Glossy Roughness', 'image'): GREY, 'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 1.0}),
    ('G8 rough map g1', {('Glossy Roughness', 'image'): GREY1, 'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 1.0}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] == GREY1:
            i['map_gamma'] = 1
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
uber_spheres('Calib-Disp', GREY_SPHERE, {}, [
    ('P1 none', {}),
    ('P2 white 0..5', {('Displacement Strength', 'image'): DISP['white'], 'Displacement Strength': 1.0, 'Minimum Displacement': 0.0, 'Maximum Displacement': 5.0}),
    ('P3 white 0..5 s0.5', {('Displacement Strength', 'image'): DISP['white'], 'Displacement Strength': 0.5, 'Minimum Displacement': 0.0, 'Maximum Displacement': 5.0}),
    ('P4 black -5..5', {('Displacement Strength', 'image'): DISP['black'], 'Displacement Strength': 1.0, 'Minimum Displacement': -5.0, 'Maximum Displacement': 5.0}),
    ('P5 grey 0..10', {('Displacement Strength', 'image'): DISP['grey'], 'Displacement Strength': 1.0, 'Minimum Displacement': 0.0, 'Maximum Displacement': 10.0}),
    ('P6 white 0..5 s2', {('Displacement Strength', 'image'): DISP['white'], 'Displacement Strength': 2.0, 'Minimum Displacement': 0.0, 'Maximum Displacement': 5.0}),
    ('P7 white -5..5', {('Displacement Strength', 'image'): DISP['white'], 'Displacement Strength': 1.0, 'Minimum Displacement': -5.0, 'Maximum Displacement': 5.0}),
    ('P8 grey -5..5', {('Displacement Strength', 'image'): DISP['grey'], 'Displacement Strength': 1.0, 'Minimum Displacement': -5.0, 'Maximum Displacement': 5.0}),
])

# --- Camera headlamp (#22): Calib-Glossy has no light node, only its dome.
# HeadlampMode (camera: 0 Auto, 1 On, 2 Off) × the render setting Auto
# Headlamp (0, 1, 2 — labels unknown), with the dome and without (Scene
# Only, the headlamp alone). Same exposure as Calib-Glossy throughout.
glossy = load(os.path.join(out_dir, 'Calib-Glossy.duf'))
def headlamp(mode, auto, scene_only):
    d = copy.deepcopy(glossy)
    cam = next(l for l in d['node_library'] if l.get('type') == 'camera')
    for e in cam.get('extra', []):
        for c in e.get('channels', []):
            if c['channel']['id'] == 'HeadlampMode':
                c['channel']['current_value'] = mode
    def walk(v):
        if isinstance(v, dict):
            if v.get('id') == 'Auto Headlamp':
                v['current_value'] = auto
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)
    walk(d['scene'].get('extra', []))
    if scene_only:
        env = next(l for l in d['node_library'] if l['id'].startswith('Environment Options'))
        set_channel(next(e for e in env['extra'] if e.get('channels') is not None)['channels'], 'Environment Mode', 3, 'enum')
    return d
for mode, mname in ((0, 'auto'), (1, 'on'), (2, 'off')):
    for auto in (0, 1, 2):
        for scene_only in (False, True):
            name = f"Calib-Headlamp-{mname}-a{auto}{'-noenv' if scene_only else ''}"
            made.append((save(headlamp(mode, auto, scene_only), name), []))

# The headlamp in a scene with lights (Calib-L-Point-5k, from
# make-light-scenes.py): Auto Headlamp 1 / 2 with HeadlampMode Auto (neither
# lit in Iray), and HeadlampMode On (lit).
point = os.path.join(out_dir, 'Calib-L-Point-5k.duf')
if os.path.exists(point):
    for mode, auto, name in ((0, 1, 'a1'), (0, 2, 'a2'), (1, 2, 'on')):
        glossy_was, glossy = glossy, load(point)
        made.append((save(headlamp(mode, auto, False), f'Calib-Headlamp-light-{name}'), []))
        glossy = glossy_was

# --- Leaf cards (Calib-Leaf, built headless by daz-build-leaf-scene.dsa):
# its cards come with DAZ's default shader; give them an Iray Uber material
# (a copy of the sphere's) and the variants. Done once, on the built scene.
leaf = os.path.join(out_dir, 'Calib-Leaf.duf')
if os.path.exists(leaf):
    d = load(leaf)
    mlib = {m['id']: m for m in d['material_library']}
    cards = sorted((n for n in d['scene']['nodes'] if n.get('label', '').startswith('Leaf ')), key=lambda n: n['label'])
    uber = next(m for m in d['material_library'] if any(e.get('type') == 'studio/material/uber_iray' for e in m.get('extra', []))
                and len(next(e for e in m['extra'] if e['type'] == 'studio_material_channels')['channels']) > 100)
    GREEN = [0.25, 0.55, 0.1]
    LEAF = [('Leaf 1', {'Translucency Weight': 0.0}),
            ('Leaf 2', {'Translucency Weight': 0.5, 'Translucency Color': GREEN, 'Thin Walled': False}),
            ('Leaf 3', {'Translucency Weight': 0.5, 'Translucency Color': GREEN, 'Thin Walled': True})]
    tuned = False
    for card, (label, over) in zip(cards, LEAF):
        geo = '#' + card['geometries'][0]['id']
        m = next(m for m in d['scene']['materials'] if m['geometry'] == geo)
        lm = mlib[m['url'].lstrip('#')]
        if any(e.get('type') == 'studio/material/uber_iray' for e in lm.get('extra', [])):
            continue
        lm['extra'] = copy.deepcopy(uber['extra'])
        m['extra'] = [{'type': 'studio/material/uber_iray', 'version': '1.1.0.0'}]
        for x in (m, lm):
            x['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': GREEN}}
        ch = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
        for k, v in {'Base Mixing': 0, 'Glossy Layered Weight': 0.0, 'Metallic Weight': 0.0, **over}.items():
            set_channel(ch, k, v, TYPES.get(type(v), 'float'))
        tuned = True
    if tuned:
        open(leaf, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        print(leaf, '(cards tuned)')

# --- Thin-walled translucency (#45): Calib-Leaf's three cards, all thin
# walled with the green diffuse, varying the Translucency Weight w and
# Color Tc; lit from behind (Calib-Leaf's sun, towards the camera) or from
# the front (turned round), the camera side then dark behind the cards; and
# each with Base Color Effect 1 (Scatter & Transmit, -bce: the Genesis
# EyeSocket's). Calib-Leaf's dump serves them all: only materials and the
# light change.
if os.path.exists(leaf):
    DARK = [0.05, 0.02, 0.02]
    THIN = {'A': [{'Translucency Weight': 0.5, 'Translucency Color': [1.0, 1.0, 1.0]},
                  {'Translucency Weight': 0.5, 'Translucency Color': GREEN},
                  {'Translucency Weight': 0.5, 'Translucency Color': DARK}],
            'B': [{'Translucency Weight': 0.0},
                  {'Translucency Weight': 0.3, 'Translucency Color': DARK},
                  {'Translucency Weight': 0.65, 'Translucency Color': DARK}]}
    for light, rot_y, bce in ((l, r, b) for l, r in (('back', 180.0), ('front', 0.0)) for b in (0, 1)):
        for name, overs in THIN.items():
            d = load(leaf)
            mlib = {m['id']: m for m in d['material_library']}
            cards = sorted((n for n in d['scene']['nodes'] if n.get('label', '').startswith('Leaf ')), key=lambda n: n['label'])
            for card, over in zip(cards, overs):
                geo = '#' + card['geometries'][0]['id']
                m = next(m for m in d['scene']['materials'] if m['geometry'] == geo)
                ch = next(e for e in mlib[m['url'].lstrip('#')]['extra'] if e['type'] == 'studio_material_channels')['channels']
                for k, v in {'Thin Walled': True, 'Translucency Color': [1.0, 1.0, 1.0], 'Base Color Effect': bce, **over}.items():
                    set_channel(ch, k, v, TYPES.get(type(v), 'float'))
            sun = next(l for l in d['node_library'] if l['id'].startswith('DistantLight'))
            next(r for r in sun['rotation'] if r['id'] == 'y')['current_value'] = rot_y
            made.append((save(d, f'Calib-Thin-{light}-{name}' + ('-bce' if bce else '')), None))

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

# --- Draw Dome off: the HDRI lights, the camera sees black behind
d = copy.deepcopy(base)
env = next(l for l in d['node_library'] if l['id'] == 'Environment Options')
set_channel(next(e for e in env['extra'] if e['type'] == 'studio_node_channels')['channels'], 'Draw Dome', False, 'bool')
made.append((save(d, 'Calib-Dome-Off'), None))

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
