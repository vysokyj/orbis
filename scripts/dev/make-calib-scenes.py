#!/usr/bin/env python3
"""ORBIS calibration scenes: variants of Calib-Materials.duf (eight spheres on
a plane under an HDRI) that each change one thing, written beside it.

  python3 scripts/dev/make-calib-scenes.py [<Calib-Materials.duf>]

- Calib-PBRSkin: the spheres in DAZ's PBRSkin shader — matte, dual lobe
  specular at reflectivity 0.25 / 0.5 / weight 0.25, top coat alone, and the
  Gladio outfit's cloth and armour (#14 follow-up: Gladiator's red cloth
  rendered pink, its armour too bright).
- Calib-PBRSkin-Occlusion: PBRSkin's Specular Occlusion (facing,
  grazing, weight, a map) and Detail (roughness mult, a tiled normal
  map), one change per sphere (#54).
- Calib-Tone-*: Iray tone mapping off the calibrated defaults — Burn
  Highlights, Crush Blacks, Gamma — to check the tone LUT's curve (#14),
  fitted so far on Gladiator alone.
- Calib-Disp: displacement (#24) by uniform maps, measured on the
  spheres' silhouettes.
- Calib-Overlay: Diffuse Overlay with weight / colour maps (#71).
- Calib-MapDepth: one grey as an 8-bit, 16-bit grey and 16-bit RGB map
  on displacement, roughness and opacity (#93): gamma "auto" by the
  file's pixel type; -2 the same on colour maps (8-bit, 16-bit grey and
  RGB, float TIFF, `map_gamma` 1) and the top coat's weight map.
- Calib-BumpRes: one height map at 256 / 1024 / 4096 px, bump factors,
  UV tiles and a half-size sphere (#75): Iray's bump vs the map's size.
- Calib-BumpGamma-Uber, -Uber2, -NVIDIA (#91): Calib-Leaf's cards lit in
  the plane of v, a synthetic sawtooth height map (and normal maps) at
  `map_gamma` 0 / 1 / 16 bit - the heights' decode gamma (bump-gamma.py);
  -NVIDIA built headless by daz-build-leaf-scene.dsa (Concrete), tuned
  here. Calib-NVIDIA-BumpG1: Calib-NVIDIA's bump maps at `map_gamma` 1.
  Calib-NVIDIA-CastFlat / -NoComp (#92); Calib-NVIDIA-Brushed (#98): Metal
  Brushed on all eight spheres, its maps flat or its bump off one at a time.
- Calib-Furnace-Metal / -Brushed (#98), -MetalBump / -CoatTint (#111): white
  Uber metals, Metal Brushed, flat-bumped metals, tinted Weighted coats
  under a uniform dome, the floor hidden - each lobe's directional albedo.
- Calib-Resin / Calib-Furnace-Resin (#113): NVIDIA's resin on Calib-NVIDIA2's
  spheres, tint / roughness / coat one at a time, under the HDRI and the dome.
- Calib-ArchGlass / Calib-Furnace-ArchGlass (#115): architectural glass on
  Calib-NVIDIA3's spheres, refr_gloss / reflection / Fresnel term one at a time.
- Calib-Furnace-UberGlass(-PL3), -ArchGlass2, -Resin3 (#116): the rough
  transmit lobe through the Uber against NVIDIA's modules - thin walls, IOR,
  tint, Firefly Filter off, Max Path Length 3.
- Calib-Mi-ImageGamma (#91, .mi only): maps the image_library doesn't
  list - which channel's default_image_gamma DAZ gives the image.
- Calib-Mi-ImageMod, -ImageMod2: per-map image modifications (#77) - tiles, tiling
  offsets, scale / offset / invert, grey channel - for DAZ's .mi export.
- Calib-Mi-SSSTint: SSS Reflectance Tint with Base Color Effect 0 / 1 / 2,
  translucency and the effect's channel shown or hidden (#52), for the .mi
  and an Iray render.
- Calib-Mi-Visible, -Visible-Shown: every Uber channel group off its
  defaults, with the base material's hidden channels kept hidden or all
  shown - which ones DAZ reads otherwise (#52 follow-up); -Visible2(-Shown)
  the gloss, backscatter and coat groups in every mixing mode; -Visible3
  what lets hidden gloss anisotropy through (Glossy Roughness > 0).
- Calib-Mi-FullRefl, -FullRefl-Shown (#120, .mi only): a refracting
  sphere with shared glossy inputs per mixing mode, Abbe, thin wall,
  layered weight - when DAZ sets full_refraction_reflection_enable.
- Calib-DualLobeGlossy (#106): the glossy layer over the dual lobe, each
  alone and both together (weight, roughness, reflectivity, Top Coat).
- Calib-DualLobeWeighted (#121): the dual lobe under weighted mixing
  (Base Mixing 2), each part alone and both (shares, weights below 1,
  Top Coat, reflectivity, dual lobe weight).
- Calib-DualLobeWeightedTL (#128): the same with translucency, which skips
  the weighted Mix (gloss / dual lobe alone and both, translucency 1, Top
  Coat, weights below 1, no translucency, Glossy Reflectivity 0).
- Calib-WeightedTL, -Thin (#131): weighted mixing with translucency on a
  diffuse map - Diffuse / Translucency / Glossy Weight combinations, plain
  colours and a dark diffuse map under lighter translucency / glossy maps.
- Thin walls (#134): Calib-Materials's sphere material is thin-walled, and
  so is every generated sphere that doesn't set `Thin Walled` false
  (Calib-WeightedTL is -Thin's twin in fact). The closed twins: Calib-SSS-Solid,
  -WeightedTL-Solid, -DualLobeWeightedTL-Solid, -MetalRefr-Solid,
  -MetalRefr-TL-Solid.
- Calib-DOF-*: depth of field, focus at 1.5 m with the spheres at ~4.6 m,
  f/4, f/8, f/16 — to check Iray's 10x aperture.

Renders are capped by samples, not time (scripts/dev/daz-render.sh): Iray is
then deterministic, so the tone variants share their noise pixel for pixel.
Existing scenes are kept (--force rewrites EVERY generated scene in the
scenes dir - to rebuild one, delete its .duf instead); dump and export the new ones
with scripts/dev/daz-dump.sh.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, glob, gzip, json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

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
    duf_camera.ensure(d)  # else Iray renders DAZ's Perspective View (#97)
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
geo_of = {n['id']: n['geometries'][0]['id'] for n in spheres}


def pbr_spheres(name, variants):
    d = copy.deepcopy(base)
    d_nodes = {n['id']: n for n in d['scene']['nodes']}
    labels = []
    for s0, (label, colour, over) in zip(spheres, variants):
        n = d_nodes[s0['id']]
        n['label'] = label
        labels.append(label)
        lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == n['url'])
        lib['label'] = label
        m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + geo_of[n['id']])
        channels = []
        for k, v in {**OFF, **over}.items():
            if isinstance(k, tuple):
                # A map on the channel (its value from `over`), listed in
                # image_library, as DAZ saves it.
                c = next(c['channel'] for c in channels if c['channel']['id'] == k[0])
                c['image_file'] = v
                images = d.setdefault('image_library', [])
                if not any(i['id'] == v for i in images):
                    images.append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
            else:
                set_channel(channels, k, v, TYPES[type(v)])
        m['url'] = PBRSKIN
        m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
        m['extra'] = [{'type': 'studio/material/daz_brick'}, {'type': 'studio_material_channels', 'channels': channels}]
    made.append((save(d, name), labels))


pbr_spheres('Calib-PBRSkin', VARIANTS)

# PBRSkin metal under translucency (Gladiator's armour: metal 0.75 by map,
# translucency 0.85): how much of the diffuse share the translucency takes
# in Iray, with and without a metallic map (our own synthetic greys).
ARM = {**DL, 'Metallicity Enable': True, 'Specular Lobe 1 Roughness': 0.4, 'Translucency Color': GREY}
MAPS = '/Runtime/Textures/ORBIS/'
pbr_spheres('Calib-PBRSkin-MetalTL', [
    ('M1 metal 0.75', GREY, {**ARM, 'Metallic Weight': 0.75}),
    ('M2 metal 0.75 tl 0.85', GREY, {**ARM, 'Metallic Weight': 0.75, 'Translucency Enable': True, 'Translucency Weight': 0.85}),
    ('M3 metal 0.75 tl 0.5', GREY, {**ARM, 'Metallic Weight': 0.75, 'Translucency Enable': True, 'Translucency Weight': 0.5}),
    ('M4 metal 0.3 tl 0.85', GREY, {**ARM, 'Metallic Weight': 0.3, 'Translucency Enable': True, 'Translucency Weight': 0.85}),
    ('M5 metal 0 tl 0.85', GREY, {**ARM, 'Metallic Weight': 0.0, 'Translucency Enable': True, 'Translucency Weight': 0.85}),
    ('M6 white map tl 0.85', GREY, {**ARM, 'Metallic Weight': 0.75, ('Metallic Weight', 'image'): MAPS + 'calib-disp-white.png',
                                    'Translucency Enable': True, 'Translucency Weight': 0.85}),
    ('M7 grey map tl 0.85', GREY, {**ARM, 'Metallic Weight': 0.75, ('Metallic Weight', 'image'): MAPS + 'calib-disp-grey.png',
                                   'Translucency Enable': True, 'Translucency Weight': 0.85}),
    ('M8 grey map', GREY, {**ARM, 'Metallic Weight': 0.75, ('Metallic Weight', 'image'): MAPS + 'calib-disp-grey.png'}),
])

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
# The base material is thin-walled (see uber_spheres), so Calib-SSS's
# spheres are thin-walled shells, S4 no different from the rest (#134);
# Calib-SSS-Solid is the same with closed solids (S4 alone thin-walled).
for sss_name, walls in [('Calib-SSS', {}), ('Calib-SSS-Solid', {'Thin Walled': False})]:
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
        for k, v in {'Base Mixing': 0, 'Glossy Layered Weight': 0.0, 'Translucency Color': SKINLIKE, **walls, **over}.items():
            set_channel(ch, k, v, TYPES.get(type(v), 'float'))
    made.append((save(d, sss_name), sss_labels))

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
IMAGE_MOD_DEFAULTS = {'grayscale_mode': 'average', 'scale': 1, 'offset': 0, 'invert': False, 'horizontal_tiles': 1,
                      'horizontal_tiling_offset': 0, 'vertical_tiles': 1, 'vertical_tiling_offset': 0}


def uber_spheres(name, colour, plain, variants):
    """Eight Uber spheres: Calib-Materials's sphere material reset to a plain
    base, then `plain`, then each sphere's `over`. The base material is
    **thin-walled** (`Thin Walled` true, not the Uber default) and the reset
    keeps it so - every scene generated so far relies on it, and Iray reads
    the channel only where light passes (translucency, refraction: the
    volume, the transmission's sides). A closed solid has to say
    `'Thin Walled': False` (the `-Solid` twins, #134)."""
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
                     'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Thin Walled': True, **plain, **over}.items():
            # A map given as (path, {field: value}) carries DAZ's per-map
            # texture-instance settings (`image_modification`, #77), the
            # fields it leaves out at DAZ's defaults.
            imod = None
            if isinstance(k, tuple) and isinstance(v, tuple):
                v, imod = v[0], {**IMAGE_MOD_DEFAULTS, **v[1]}
            if isinstance(k, tuple) and k[0] == 'diffuse':
                # The base colour's map: on the scene material's top-level
                # channel, as DAZ saves it (see Calib-Mixing).
                m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': over.get('diffuse', [1.0, 1.0, 1.0]), 'image_file': v}}
                if imod:
                    m['diffuse']['channel']['image_modification'] = imod
                lib_images = d.setdefault('image_library', [])
                if not any(i['id'] == v for i in lib_images):
                    lib_images.append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
            elif isinstance(k, tuple):
                # A map: on the scene material's channel, listed in
                # image_library, as DAZ saves them (see Calib-Mixing). A
                # third element names the channel's type (a colour).
                extra = m.setdefault('extra', [])
                sc = next((e for e in extra if e.get('type') == 'studio_material_channels'), None)
                if sc is None:
                    sc = {'type': 'studio_material_channels', 'channels': []}
                    extra.append(sc)
                typ = k[2] if len(k) > 2 else 'float'
                value = over.get(k[0], [1.0, 1.0, 1.0] if typ == 'float_color' else 1.0)
                sc['channels'].append({'channel': {'id': k[0], 'type': typ, 'current_value': value, 'image_file': v}})
                if imod:
                    sc['channels'][-1]['channel']['image_modification'] = imod
                lib_images = d.setdefault('image_library', [])
                if not any(i['id'] == v for i in lib_images):
                    lib_images.append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
            elif isinstance(v, dict):
                # A whole channel entry (a user property or a channel the
                # Uber definition lacks), as the scene that carries it saved it.
                ch[:] = [c for c in ch if c['channel']['id'] != k] + [copy.deepcopy(v)]
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
# Anisotropy, second round (#23): the rotation's sign (0.125 / 0.375 mirror
# each other; Calib-Aniso's 0.25 can't tell), the weighted and the rotated
# Top Coat, Backscattering Anisotropy with the glossy one on (DAZ leaves it
# out of the .mi when that is off), the curve between 0, 0.5 and 1.
uber_spheres('Calib-Aniso2', GREY_SPHERE, {**GLOSS, 'Glossy Roughness': 0.4}, [
    ('K1 aniso 1 rot 0.125', {'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.125}),
    ('K2 aniso 1 rot 0.375', {'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.375}),
    ('K3 metal rough 0.2 aniso 1', {'Metallic Weight': 1.0, 'Glossy Roughness': 0.2, 'Glossy Anisotropy': 1.0}),
    ('K4 weighted coat aniso 1', {'Glossy Layered Weight': 0.0, 'Top Coat Weight': 1.0, 'Top Coat Layering Mode': 1, 'Top Coat Roughness': 0.3, 'Top Coat Anisotropy': 1.0}),
    ('K5 top coat aniso 1 rot 0.25', {'Glossy Layered Weight': 0.0, 'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.3, 'Top Coat Anisotropy': 1.0, 'Top Coat Rotations': 0.25}),
    ('K6 backscatter aniso 0.8', {'Glossy Anisotropy': 0.5, 'Backscattering Weight': 1.0, 'Backscattering Roughness': 0.3, 'Backscattering Anisotropy': 0.8}),
    ('K7 aniso 0.25', {'Glossy Anisotropy': 0.25}),
    ('K8 aniso 0.75', {'Glossy Anisotropy': 0.75}),
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
# Dual lobe, second round (#26): is lobe 2's roughness squared as lobe 1's
# (E7 / E8 against Calib-DualLobe's D5), and Uber's defaults (ratio 0.85,
# lobe 2 roughness 0: a mirror lobe; 152 materials in My Library).
uber_spheres('Calib-DualLobe2', [0.45, 0.05, 0.04], DL, [
    ('E1 ratio 0.85 r2 0', {'Dual Lobe Specular Ratio': 0.85, 'Specular Lobe 2 Roughness': 0.0}),
    ('E2 ratio 0.85 r2 0.3', {'Dual Lobe Specular Ratio': 0.85, 'Specular Lobe 2 Roughness': 0.3}),
    ('E3 ratio 0.5 r2 0.3', {'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.3}),
    ('E4 ratio 0.5 r2 0.9', {'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.9}),
    ('E5 lobe 2 only r 0.3', {'Dual Lobe Specular Ratio': 0.0, 'Specular Lobe 2 Roughness': 0.3}),
    ('E6 lobe 2 only r 0.9', {'Dual Lobe Specular Ratio': 0.0, 'Specular Lobe 2 Roughness': 0.9}),
    ('E7 lobe 1 only r 0.6', {'Dual Lobe Specular Ratio': 1.0, 'Specular Lobe 1 Roughness': 0.6}),
    ('E8 lobe 1 r 0.6 unsquared', {'Dual Lobe Specular Ratio': 1.0, 'Specular Lobe 1 Roughness': 0.6, 'Roughness Squared': False}),
])
# The glossy layer over the dual lobe (#106): DAZ's MDL has both in the
# stack, the gloss over the dual lobe over the diffuse. G1 / G2 each layer
# alone (the decomposition), G3 - G8 both: the gloss's weight, a sharp
# gloss over a broad lobe, reflectivity 1 (the IOR), under a Top Coat (the
# Layered material), the dual lobe at half weight.
DLG = {**DL, 'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.1,
       'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.5, 'Glossy Reflectivity': 0.5}
DLG_SPHERES = [
    ('G1 gloss only', {'Dual Lobe Specular Weight': 0.0}),
    ('G2 dual lobe only', {'Glossy Layered Weight': 0.0}),
    ('G3 both', {}),
    ('G4 both gloss 0.4', {'Glossy Layered Weight': 0.4}),
    ('G5 sharp gloss broad lobe', {'Glossy Roughness': 0.15, 'Specular Lobe 1 Roughness': 0.6, 'Dual Lobe Specular Ratio': 1.0}),
    ('G6 both gloss refl 1', {'Glossy Reflectivity': 1.0}),
    ('G7 both top coat', {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.1}),
    ('G8 both dual lobe w0.5', {'Dual Lobe Specular Weight': 0.5, 'Dual Lobe Specular Ratio': 1.0}),
]
uber_spheres('Calib-DualLobeGlossy', [0.45, 0.05, 0.04], DLG, DLG_SPHERES)
# Its decomposition per sphere (#106): each sphere sees its own light, so
# G1 / G2 are no baseline for G3 - G8. The same eight spheres with the
# gloss off (-dl: the dual lobe alone, G7 keeps its Top Coat), the dual
# lobe off (-gloss) and everything off (-bare: the diffuse) split each
# sphere into its layers where it stands.
for suffix, off in [('dl', {'Glossy Layered Weight': 0.0}),
                    ('gloss', {'Dual Lobe Specular Weight': 0.0}),
                    ('bare', {'Glossy Layered Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0})]:
    uber_spheres(f'Calib-DualLobeGlossy-{suffix}', [0.45, 0.05, 0.04], DLG,
                 [(label, {**over, **off}) for label, over in DLG_SPHERES])
# The dual lobe under weighted mixing (#121): DAZ's normalized_base mixes
# the diffuse with its dual lobe at Diffuse Weight against the gloss at
# Glossy Weight. W1 / W2 each part alone (the decomposition: W2 has no
# gloss part, so it is Calib-DualLobe's lobe at Base Mixing 2), W3 - W8
# both: the glossy share, the weights summing below 1 (normalized_mix
# leaves the rest black), a Top Coat over both (one blended lobe),
# reflectivity 1 (the IOR), the dual lobe at half weight.
DLW = {**DL, 'Base Mixing': 2, 'Dual Lobe Specular Ratio': 0.5, 'Specular Lobe 2 Roughness': 0.1,
       'Glossy Layered Weight': 0.0, 'Diffuse Weight': 1.0, 'Glossy Weight': 0.5, 'Glossy Roughness': 0.6}
uber_spheres('Calib-DualLobeWeighted', [0.45, 0.05, 0.04], DLW, [
    ('W1 gloss only', {'Dual Lobe Specular Weight': 0.0}),
    ('W2 dual lobe only', {'Glossy Weight': 0.0}),
    ('W3 both', {}),
    ('W4 both gloss 0.8 diffuse 0.2', {'Diffuse Weight': 0.2, 'Glossy Weight': 0.8}),
    ('W5 both diffuse 0.5 gloss 0.3', {'Diffuse Weight': 0.5, 'Glossy Weight': 0.3}),
    ('W6 both top coat', {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.1}),
    ('W7 both lobe refl 1', {'Dual Lobe Specular Reflectivity': 1.0}),
    ('W8 both dual lobe w0.5', {'Dual Lobe Specular Weight': 0.5, 'Dual Lobe Specular Ratio': 1.0}),
])
# The same with translucency (#128): weighted mixing then skips the Mix
# (mesh hair's path), the dual lobe on `specular`, the gloss the coating.
# T7 without translucency pairs with Calib-DualLobeWeighted's W3; T8's
# Glossy Reflectivity 0 asks whether normalized_base's plain glossy reads it.
DLWT = {**DLW, 'Translucency Weight': 0.5, 'Translucency Color': [0.9, 0.6, 0.4]}
DLWT_SPHERES = [
    ('T1 gloss only', {'Dual Lobe Specular Weight': 0.0}),
    ('T2 dual lobe only', {'Glossy Weight': 0.0}),
    ('T3 both', {}),
    ('T4 both translucency 1', {'Translucency Weight': 1.0}),
    ('T5 both top coat', {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.1}),
    ('T6 both diffuse 0.5 gloss 0.3', {'Diffuse Weight': 0.5, 'Glossy Weight': 0.3}),
    ('T7 both no translucency', {'Translucency Weight': 0.0}),
    ('T8 both gloss refl 0', {'Glossy Reflectivity': 0.0}),
]
uber_spheres('Calib-DualLobeWeightedTL', [0.45, 0.05, 0.04], DLWT, DLWT_SPHERES)
# Calib-DualLobeWeightedTL's spheres are thin-walled (the base's wall); the
# same closed (#134).
uber_spheres('Calib-DualLobeWeightedTL-Solid', [0.45, 0.05, 0.04], {**DLWT, 'Thin Walled': False}, DLWT_SPHERES)
# Weighted mixing's shares with translucency (#131): the export brightens
# the diffuse map by `weighted_mix` (translucency as a third share), while
# DAZ's normalized_base keeps the translucency inside the diffuse part
# (diffuse_base: diffuse reflection at 1 - t over the transmission) and,
# with no gloss part, reads no Diffuse Weight. Only a diffuse map takes the
# brightening, so every sphere has one (uniform synthetic maps, ours): a
# red one (Y 0.13) under an orange translucency (Y 0.65), and the hair case
# - a dark diffuse map under 3.5x lighter translucency and glossy maps.
# W1 / W2 without translucency (Diffuse Weight unread: W2 = W1), W3 - W5 the
# translucency with no gloss (W4 = W3 if Diffuse Weight stays unread), W6 /
# W7 with the gloss (Dw + Gw past 1 and below), W8 the hair maps.
# -Thin: the same thin-walled (mesh hair's cards).
WTL_MAPS = {name: f'/Runtime/Textures/ORBIS/calib-wtl-{name}.png' for name in ('red', 'dark', 'light')}
for name, rgb in {'red': (180, 60, 50), 'dark': (100, 70, 45), 'light': (200, 150, 100)}.items():
    p = os.path.join(os.path.dirname(out_dir), WTL_MAPS[name].lstrip('/'))
    if not os.path.exists(p):
        from PIL import Image
        os.makedirs(os.path.dirname(p), exist_ok=True)
        Image.new('RGB', (64, 64), rgb).save(p)
WTL = {'Base Mixing': 2, ('diffuse', 'image'): WTL_MAPS['red'], 'Diffuse Weight': 1.0, 'Glossy Weight': 0.0,
       'Glossy Layered Weight': 0.0, 'Glossy Roughness': 0.4, 'Glossy Color': [1.0, 1.0, 1.0],
       'Translucency Weight': 0.0, 'Translucency Color': [0.9, 0.6, 0.4]}
WTL_SPHERES = [
    ('W1 diffuse only', {}),
    ('W2 diffuse 0.5', {'Diffuse Weight': 0.5}),
    ('W3 tl 0.5', {'Translucency Weight': 0.5}),
    ('W4 tl 0.5 diffuse 0.5', {'Translucency Weight': 0.5, 'Diffuse Weight': 0.5}),
    ('W5 tl 1', {'Translucency Weight': 1.0}),
    ('W6 tl 0.5 gloss 0.5', {'Translucency Weight': 0.5, 'Glossy Weight': 0.5}),
    ('W7 tl 0.5 diffuse 0.5 gloss 0.3', {'Translucency Weight': 0.5, 'Diffuse Weight': 0.5, 'Glossy Weight': 0.3}),
    ('W8 hair maps tl 0.5 gloss 0.5', {'Translucency Weight': 0.5, 'Glossy Weight': 0.5, ('diffuse', 'image'): WTL_MAPS['dark'],
                                       ('Translucency Color', 'image', 'float_color'): WTL_MAPS['light'],
                                       ('Glossy Color', 'image', 'float_color'): WTL_MAPS['light']}),
]
# Calib-WeightedTL was meant as the closed twin but inherits the base's thin
# wall: it is -Thin's scene under another name (both rendered alike, #134).
# -Solid is the closed one.
uber_spheres('Calib-WeightedTL', [1.0, 1.0, 1.0], WTL, WTL_SPHERES)
uber_spheres('Calib-WeightedTL-Thin', [1.0, 1.0, 1.0], {**WTL, 'Thin Walled': True}, WTL_SPHERES)
uber_spheres('Calib-WeightedTL-Solid', [1.0, 1.0, 1.0], {**WTL, 'Thin Walled': False}, WTL_SPHERES)
# A metal under a partial refraction (#129): DAZ's metallic_layered_base
# lays the refraction (weighted_layer at Refraction Weight) over the metal,
# so Iray's metal keeps (1 - r) of its colour and the refraction r whatever
# the metal; Universal's transmission sits in its dielectric, under the
# metallic blend. R1 / R2 refraction alone, M1 / M2 metal alone (the
# decomposition), X1 - X4 both. The red base is the metal's colour, the
# refraction white (Glossy Color, shared inputs): the channels split them.
# -g0 without the glossy layer (the refraction then reflects nothing of its
# own, `refraction_reflection_enable` off); -TL with translucency 0.5, which
# a refractive metal keeps out of the translucent-metal Mix and dimming.
MR = {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.2, 'Refraction Index': 1.5}
MR_SPHERES = [
    ('R1 refraction 0.3', {'Refraction Weight': 0.3}),
    ('R2 refraction 0.7', {'Refraction Weight': 0.7}),
    ('M1 metal 0.75', {'Metallic Weight': 0.75}),
    ('M2 metal 0.3', {'Metallic Weight': 0.3}),
    ('X1 metal 0.75 refraction 0.3', {'Metallic Weight': 0.75, 'Refraction Weight': 0.3}),
    ('X2 metal 0.75 refraction 0.7', {'Metallic Weight': 0.75, 'Refraction Weight': 0.7}),
    ('X3 metal 0.3 refraction 0.3', {'Metallic Weight': 0.3, 'Refraction Weight': 0.3}),
    ('X4 metal 0.3 refraction 0.7', {'Metallic Weight': 0.3, 'Refraction Weight': 0.7}),
]
uber_spheres('Calib-MetalRefr', [0.45, 0.05, 0.04], MR, MR_SPHERES)
uber_spheres('Calib-MetalRefr-g0', [0.45, 0.05, 0.04], {**MR, 'Glossy Layered Weight': 0.0}, MR_SPHERES)
uber_spheres('Calib-MetalRefr-TL', [0.45, 0.05, 0.04], {**MR, 'Translucency Weight': 0.5, 'Translucency Color': [0.9, 0.6, 0.4]}, MR_SPHERES)
# The three above are thin-walled (the base's wall, #134): refraction
# through a shell, not a solid. -Solid / -TL-Solid close them (#135, #136).
uber_spheres('Calib-MetalRefr-Solid', [0.45, 0.05, 0.04], {**MR, 'Thin Walled': False}, MR_SPHERES)
uber_spheres('Calib-MetalRefr-TL-Solid', [0.45, 0.05, 0.04], {**MR, 'Translucency Weight': 0.5, 'Translucency Color': [0.9, 0.6, 0.4],
                                                              'Thin Walled': False}, MR_SPHERES)
# Volume scattering (#28) on a skin's volume (Victoria 8's: translucency
# 0.9, Transmitted Color over 0.125, scattering over 0.015 cm): chromatic
# SSS scatters −ln(SSS Color) per distance, mono SSS Amount per distance
# (DAZ's MDL); SSS Reflectance Tint isn't in the MDL.
SKIN_VOLUME = {'Translucency Weight': 0.9, 'Translucency Color': [0.9, 0.55, 0.45], 'Thin Walled': False,
               'Transmitted Color': [0.98, 0.48, 0.35], 'Transmitted Measurement Distance': 0.125,
               'Scattering Measurement Distance': 0.015, 'SSS Mode': 1, 'SSS Color': [0.98, 0.69, 0.76],
               'SSS Amount': 2.0, 'SSS Direction': 0.0}
uber_spheres('Calib-SSSColor', [0.8, 0.55, 0.45], SKIN_VOLUME, [
    ('V1 chromatic', {}),
    ('V2 chromatic grey', {'SSS Color': [0.7, 0.7, 0.7]}),
    ('V3 chromatic blue', {'SSS Color': [0.6, 0.75, 0.95]}),
    ('V4 chromatic deep', {'SSS Color': [0.995, 0.95, 0.95]}),
    ('V5 mono amount 2', {'SSS Mode': 0}),
    ('V6 mono amount 0.5', {'SSS Mode': 0, 'SSS Amount': 0.5}),
    ('V7 tint blue', {'SSS Reflectance Tint': [0.7, 0.85, 1.0]}),
    ('V8 no translucency', {'Translucency Weight': 0.0}),
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
# Colour effects over an opaque matte grey (#27, #52): does a top coat's or a
# glossy layer's colour tint what lies under it? Morgan Hair's coat (0.975,
# beige, effect 2) darkened its hair in Iray.
COAT_RED = {'Top Coat Weight': 1.0, 'Top Coat Color': [0.9, 0.3, 0.2], 'Top Coat Roughness': 0.3}
COAT_MORGAN = {'Top Coat Weight': 0.975, 'Top Coat Color': [0.522, 0.494, 0.443], 'Top Coat Roughness': 0.3}
GLOSS_RED = {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3, 'Glossy Color': [0.9, 0.3, 0.2]}
uber_spheres('Calib-CoatEffect', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 0.0}, [
    ('E1 coat red fx0', {**COAT_RED, 'Top Coat Color Effect': 0}),
    ('E2 coat red fx1', {**COAT_RED, 'Top Coat Color Effect': 1}),
    ('E3 coat red fx2', {**COAT_RED, 'Top Coat Color Effect': 2}),
    ('E4 coat morgan fx0', {**COAT_MORGAN, 'Top Coat Color Effect': 0}),
    ('E5 coat morgan fx2', {**COAT_MORGAN, 'Top Coat Color Effect': 2}),
    ('E6 gloss red fx0', {**GLOSS_RED, 'Glossy Color Effect': 0}),
    ('E7 gloss red fx1', {**GLOSS_RED, 'Glossy Color Effect': 1}),
    ('E8 gloss red fx2', {**GLOSS_RED, 'Glossy Color Effect': 2}),
])
# Thin films with an IOR contrast (#31): Calib-ThinFilm's films sit at the
# IOR of the layer under them and hardly show in Iray; Morgan Hair's coat
# film (193 876 nm at 1.91 over a coat of 1.5) darkened its hair 0.5–0.7 ×.
COAT = {'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.3, 'Top Coat IOR': 1.5}
uber_spheres('Calib-FilmIOR', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 0.0}, [
    ('F1 coat no film', COAT),
    ('F2 coat 500 1.91', {**COAT, 'Top Coat Thin Film': 500.0, 'Top Coat Thin Film IOR': 1.914}),
    ('F3 coat 194um 1.91', {**COAT, 'Top Coat Thin Film': 193875.6, 'Top Coat Thin Film IOR': 1.914}),
    ('F4 coat 194um 1.5', {**COAT, 'Top Coat Thin Film': 193875.6, 'Top Coat Thin Film IOR': 1.5}),
    ('F5 coat 194um 1.3', {**COAT, 'Top Coat Thin Film': 193875.6, 'Top Coat Thin Film IOR': 1.3}),
    ('F6 coat 2000 1.91', {**COAT, 'Top Coat Thin Film': 2000.0, 'Top Coat Thin Film IOR': 1.914}),
    ('F7 gloss 500 1.91', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3,
                           'Base Thin Film': 500.0, 'Base Thin Film IOR': 1.914}),
    ('F8 gloss 194um 1.91', {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.3,
                             'Base Thin Film': 193875.6, 'Base Thin Film IOR': 1.914}),
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
# With dispersion Iray's transmission is smooth whatever the roughness (DAZ's
# MDL, `refraction_base`): pairs without / with Abbe at a shared glossy
# roughness and at the refraction's own (Share Glossy Inputs off).
GL_GLOSS = {**GL, 'Glossy Layered Weight': 1.0}
GL_OWN = {**GL, 'Share Glossy Inputs': False}
uber_spheres('Calib-Dispersion-Rough', [0.6, 0.6, 0.6], GL, [
    ('R1 gloss 0.3 abbe 0', {**GL_GLOSS, 'Glossy Roughness': 0.3}),
    ('R2 gloss 0.3 abbe 20', {**GL_GLOSS, 'Glossy Roughness': 0.3, 'Abbe': 20.0}),
    ('R3 own 0.3 abbe 0', {**GL_OWN, 'Refraction Roughness': 0.3}),
    ('R4 own 0.3 abbe 20', {**GL_OWN, 'Refraction Roughness': 0.3, 'Abbe': 20.0}),
    ('R5 gloss 0.6 abbe 0', {**GL_GLOSS, 'Glossy Roughness': 0.6}),
    ('R6 gloss 0.6 abbe 20', {**GL_GLOSS, 'Glossy Roughness': 0.6, 'Abbe': 20.0}),
    ('R7 own 0.6 abbe 0', {**GL_OWN, 'Refraction Roughness': 0.6}),
    ('R8 own 0.6 abbe 20', {**GL_OWN, 'Refraction Roughness': 0.6, 'Abbe': 20.0}),
])

# --- Channel -> MDL mapping (docs/daz/iray-uber-mdl.md): scenes exported to
# Iray's .mi only (scripts/dev/daz-export-mi.dsa, scripts/dev/mi-args.py),
# no Iray references — what DAZ Studio passes to the Uber MDL per channel.
RED = [0.6, 0.1, 0.05]
uber_spheres('Calib-Map-Metal', RED, GLOSS, [
    ('M1 metal 0.25', {'Metallic Weight': 0.25}),
    ('M2 metal 0.5', {'Metallic Weight': 0.5}),
    ('M3 metal 0.75', {'Metallic Weight': 0.75}),
    ('M4 metal 1', {'Metallic Weight': 1.0}),
    ('M5 metal 1 refl 1', {'Metallic Weight': 1.0, 'Glossy Reflectivity': 1.0}),
    ('M6 metal 1 blue gloss', {'Metallic Weight': 1.0, 'Glossy Color': [0.1, 0.3, 0.9]}),
    ('M7 metal 0.5 no gloss', {'Metallic Weight': 0.5, 'Glossy Layered Weight': 0.0}),
    ('M8 metal 0.5 spec mode', {'Metallic Weight': 0.5, 'Base Mixing': 1, 'Glossy Specular': [0.5, 0.5, 0.5], 'Glossiness': 0.7}),
])
COATED = {**GLOSS, 'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.2}
uber_spheres('Calib-Map-TopCoat', RED, COATED, [
    ('C1 refl 0.25', {'Top Coat Layering Mode': 0, 'Reflectivity': 0.25}),
    ('C2 refl 1 w0.5', {'Top Coat Layering Mode': 0, 'Reflectivity': 1.0, 'Top Coat Weight': 0.5}),
    ('C3 weighted red', {'Top Coat Layering Mode': 1, 'Top Coat Color': [0.9, 0.2, 0.1]}),
    ('C4 fresnel 1.8', {'Top Coat Layering Mode': 2, 'Top Coat IOR': 1.8}),
    ('C5 curve', {'Top Coat Layering Mode': 3, 'Top Coat Curve Normal': 0.1, 'Top Coat Curve Grazing': 0.8, 'Top Coat Curve Exponent': 3.0}),
    ('C6 refl specular', {'Top Coat Layering Mode': 0, 'Reflectivity': 0.5, 'Top Coat Specular': [0.2, 0.5, 0.9]}),
    ('C7 effect 1', {'Top Coat Layering Mode': 2, 'Top Coat Color': [0.9, 0.2, 0.1], 'Top Coat Color Effect': 1}),
    ('C8 effect 2', {'Top Coat Layering Mode': 2, 'Top Coat Color': [0.9, 0.2, 0.1], 'Top Coat Color Effect': 2}),
])
# Channels the audit found unread (#15): Top Coat Specular in the other
# layering modes (Calib-Map-TopCoat's C6 has mode 0), PBR Masked Threshold
# (a user property some tear materials carry, saved as below) and Multiply
# Specular Through Opacity (a DAZ Default / brick channel, here on Uber).
PBR_MASKED = {'user_property': True, 'group': '/Physically-Based', 'channel': {
    'id': 'PBR Masked Threshold', 'type': 'float', 'label': 'PBR Masked Threshold', 'value': 0, 'current_value': 0.999,
    'min': 0, 'max': 1, 'clamped': True, 'step_size': 0.01, 'default_image_gamma': 1, 'mappable': True}}
MSTO_OFF = {'group': '/Specular', 'channel': {'id': 'Multiply Specular Through Opacity', 'type': 'bool',
            'label': 'Multiply Specular Through Opacity', 'value': True, 'current_value': False}}
def pbr_masked(t):
    e = copy.deepcopy(PBR_MASKED)
    e['channel']['current_value'] = t
    return e
uber_spheres('Calib-Map-Audit', RED, COATED, [
    ('A1 weighted specular', {'Top Coat Layering Mode': 1, 'Top Coat Specular': [0.2, 0.5, 0.9]}),
    ('A2 fresnel specular', {'Top Coat Layering Mode': 2, 'Top Coat Specular': [0.2, 0.5, 0.9]}),
    ('A3 curve specular', {'Top Coat Layering Mode': 3, 'Top Coat Specular': [0.2, 0.5, 0.9]}),
    ('A4 refl 1 specular', {'Top Coat Layering Mode': 0, 'Reflectivity': 1.0, 'Top Coat Specular': [0.9, 0.5, 0.2]}),
    ('A5 masked 0.999 cutout 0.9989', {'Top Coat Weight': 0.0, 'Cutout Opacity': 0.9989, 'PBR Masked Threshold': pbr_masked(0.999)}),
    ('A6 masked 0.7 cutout 0.5', {'Top Coat Weight': 0.0, 'Cutout Opacity': 0.5, 'PBR Masked Threshold': pbr_masked(0.7)}),
    ('A7 masked 0.3 cutout 0.5', {'Top Coat Weight': 0.0, 'Cutout Opacity': 0.5, 'PBR Masked Threshold': pbr_masked(0.3)}),
    ('A8 msto off cutout 0.5', {'Top Coat Weight': 0.0, 'Cutout Opacity': 0.5, 'Multiply Specular Through Opacity': MSTO_OFF}),
])
TRANSL = {'Translucency Weight': 0.5, 'Translucency Color': [0.2, 0.6, 0.9]}
uber_spheres('Calib-Map-Effect', RED, GLOSS, [
    ('E1 base effect 1', {**TRANSL, 'Base Color Effect': 1}),
    ('E2 base effect 2', {**TRANSL, 'Base Color Effect': 2}),
    ('E3 gloss effect 1', {'Glossy Color': [0.9, 0.3, 0.2], 'Glossy Color Effect': 1}),
    ('E4 gloss effect 2', {'Glossy Color': [0.9, 0.3, 0.2], 'Glossy Color Effect': 2}),
    ('E5 diffuse strength 0.5', {'Diffuse Strength': 0.5}),
    ('E6 overlay effect 1', {'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Color': [0.1, 0.3, 0.9], 'Diffuse Overlay Color Effect': 1}),
    ('E7 overlay squared', {'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Weight Squared': True, 'Diffuse Overlay Color': [0.1, 0.3, 0.9]}),
    ('E8 transl 0.5', TRANSL),
])
SPECM = {'Base Mixing': 1, 'Glossy Layered Weight': 1.0, 'Glossiness': 0.7, 'Glossy Color': [1.0, 1.0, 1.0]}
uber_spheres('Calib-Map-SpecMode', RED, SPECM, [
    ('S1 spec 0', {'Glossy Specular': [0.0, 0.0, 0.0]}),
    ('S2 spec 0.2', {'Glossy Specular': [0.2, 0.2, 0.2]}),
    ('S3 spec 0.5', {'Glossy Specular': [0.5, 0.5, 0.5]}),
    ('S4 spec 1', {'Glossy Specular': [1.0, 1.0, 1.0]}),
    ('S5 spec red', {'Glossy Specular': [0.9, 0.3, 0.2]}),
    ('S6 spec 0.5 w0.5', {'Glossy Specular': [0.5, 0.5, 0.5], 'Glossy Layered Weight': 0.5}),
    ('S7 glossiness 0.3', {'Glossy Specular': [0.5, 0.5, 0.5], 'Glossiness': 0.3}),
    ('S8 weighted mode', {'Base Mixing': 2, 'Glossy Weight': 0.4, 'Diffuse Weight': 0.8}),
])
uber_spheres('Calib-Map-Misc', RED, GLOSS, [
    ('X1 backscatter blue', {'Backscattering Weight': 0.5, 'Backscattering Color': [0.1, 0.3, 0.9], 'Backscattering Roughness': 0.3}),
    ('X2 backscatter unshared', {'Backscattering Weight': 0.5, 'Backscattering Color': [0.1, 0.3, 0.9], 'Share Glossy Inputs': False}),
    ('X3 dual lobe', {'Dual Lobe Specular Weight': 0.8, 'Dual Lobe Specular Reflectivity': 0.6, 'Specular Lobe 1 Roughness': 0.4, 'Specular Lobe 2 Roughness': 0.2, 'Dual Lobe Specular Ratio': 0.7}),
    ('X4 abbe 30', {'Refraction Weight': 1.0, 'Refraction Index': 1.6, 'Abbe': 30.0}),
    ('X5 refraction rough', {'Refraction Weight': 0.7, 'Refraction Roughness': 0.3, 'Share Glossy Inputs': False}),
    ('X6 aniso', {'Glossy Anisotropy': 0.6, 'Glossy Anisotropy Rotations': 0.25}),
    ('X7 volume', {'Thin Walled': False, 'Refraction Weight': 1.0, 'Transmitted Color': [0.5, 0.7, 0.9], 'Transmitted Measurement Distance': 2.0, 'SSS Amount': 0.3, 'Scattering Measurement Distance': 0.5, 'SSS Direction': 0.4}),
    ('X8 flakes', {'Metallic Flakes Weight': 0.5, 'Metallic Flakes Color': [0.9, 0.8, 0.3], 'Metallic Flakes Size': 0.01, 'Metallic Flakes Roughness': 0.2}),
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
# Map depth (#93): gamma "auto" by the file's pixel type. Iray's loader
# reads an 8-bit map at 2.2, a 16-bit single channel at 1.0, 16-bit RGB at
# 2.2 (MDL SDK `get_pixel_type_and_gamma`). The same grey 128 (8 bits) /
# 32896 (16 bits, the same 0.502) as displacement over 0..10 cm (the
# silhouette: 2.2 cm at 2.2, 5 cm linear), roughness and opacity; a 16-bit
# grey at `map_gamma` 2.2 (Octane's gamma pin on a 16-bit PNG).
def png16(path, a):
    """A 16-bit PNG of a uint16 array (h, w) grey or (h, w, 3) RGB."""
    import struct, zlib
    h, w = a.shape[:2]
    rows = b''.join(b'\0' + a[y].astype('>u2').tobytes() for y in range(h))
    chunk = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    ihdr = struct.pack('>IIBBBBB', w, h, 16, 2 if a.ndim == 3 else 0, 0, 0, 0)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))
MD = {name: f'/Runtime/Textures/ORBIS/calib-mapdepth-{name}.png' for name in ('16', '16-rgb', '16-g22')}
for name, path in MD.items():
    p = os.path.join(os.path.dirname(out_dir), path.lstrip('/'))
    if not os.path.exists(p):
        import numpy as np
        png16(p, np.full((64, 64, 3) if name == '16-rgb' else (64, 64), 128 * 257, np.uint16))
MD_GLOSS = {'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 1.0}
uber_spheres('Calib-MapDepth', GREY_SPHERE, {}, [
    ('D1 disp 8-bit', {('Displacement Strength', 'image'): DISP['grey'], **R10}),
    ('D2 disp 16-bit', {('Displacement Strength', 'image'): MD['16'], **R10}),
    ('D3 disp 16-bit rgb', {('Displacement Strength', 'image'): MD['16-rgb'], **R10}),
    ('D4 disp 16-bit gamma 2.2', {('Displacement Strength', 'image'): MD['16-g22'], **R10}),
    ('R1 rough 8-bit', {('Glossy Roughness', 'image'): DISP['grey'], **MD_GLOSS}),
    ('R2 rough 16-bit', {('Glossy Roughness', 'image'): MD['16'], **MD_GLOSS}),
    ('O1 opacity 8-bit', {('Cutout Opacity', 'image'): DISP['grey'], 'Cutout Opacity': 1.0}),
    ('O2 opacity 16-bit', {('Cutout Opacity', 'image'): MD['16'], 'Cutout Opacity': 1.0}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] == MD['16-g22']:
            i['map_gamma'] = 2.2
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Calib-MapDepth2: the same rule on colour maps (diffuse: 8-bit, 16-bit grey
# and RGB, a float TIFF, the 8-bit map at `map_gamma` 1) and on the top
# coat's weight map in the weighted and the anisotropic (layered) coat.
MD['f32'] = '/Runtime/Textures/ORBIS/calib-mapdepth-f32.tif'
p = os.path.join(os.path.dirname(out_dir), MD['f32'].lstrip('/'))
if not os.path.exists(p):
    from PIL import Image
    import numpy as np
    Image.fromarray(np.full((64, 64), 128 / 255, np.float32), 'F').save(p)
MD_COAT = {'Glossy Layered Weight': 0.0, 'Top Coat Weight': 1.0, 'Top Coat Roughness': 0.3}
uber_spheres('Calib-MapDepth2', GREY_SPHERE, {}, [
    ('C1 colour 8-bit', {('diffuse', 'image'): DISP['grey']}),
    ('C2 colour 16-bit', {('diffuse', 'image'): MD['16']}),
    ('C3 colour 16-bit rgb', {('diffuse', 'image'): MD['16-rgb']}),
    ('C4 colour float', {('diffuse', 'image'): MD['f32']}),
    ('C5 colour gamma 1', {('diffuse', 'image'): DISP['grey-g1']}),
    ('T1 weighted coat map 8-bit', {('Top Coat Weight', 'image'): DISP['grey'], **MD_COAT, 'Top Coat Layering Mode': 1}),
    ('T2 weighted coat map 16-bit', {('Top Coat Weight', 'image'): MD['16'], **MD_COAT, 'Top Coat Layering Mode': 1}),
    ('T3 aniso coat map 8-bit', {('Top Coat Weight', 'image'): DISP['grey'], **MD_COAT, 'Top Coat Anisotropy': 0.5}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] == DISP['grey-g1']:
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

# .mi sweep (#62): what sets the headlamp DAZ hands Iray — its intensity,
# the camera's focus distance, the offsets (HeadlampMode On, no dome).
for name, over in (('i05', {'Headlamp Intensity': 0.5}), ('i2', {'Headlamp Intensity': 2.0}),
                   ('dof200', {'Depth of Field': 200.0}), ('offset0', {'XHeadlampOffset': 0.0, 'YHeadlamp': 0.0, 'ZHeadlampOffset': 0.0})):
    d = headlamp(1, 0, True)
    cam = next(l for l in d['node_library'] if l.get('type') == 'camera')
    chs = next(e for e in cam['extra'] if e.get('type') == 'studio_node_channels')['channels']
    for k, v in over.items():
        set_channel(chs, k, v, 'float')
    if 'Depth of Field' in over:
        cam['perspective']['focal_distance'] = over['Depth of Field']
    made.append((save(d, f'Calib-Mi-Headlamp-{name}'), []))

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

# --- .mi sweeps (#62): what DAZ hands Iray where no other scene shows it,
# read from DAZ's .mi export (scripts/dev/daz-export-mi.dsa), not rendered.
# Values chosen so each conversion is unambiguous (e.g. Glossy Roughness
# and 1 − Glossiness differ).
G = '/Runtime/Textures/ORBIS/calib-disp-grey.png'
MIX = {'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3, 'Glossiness': 0.2}
LOBES = {'Dual Lobe Specular Weight': 0.8, 'Specular Lobe 1 Roughness': 0.3, 'Specular Lobe 1 Glossiness': 0.2,
         'Specular Lobe 2 Roughness': 0.5, 'Specular Lobe 2 Glossiness': 0.6, 'Dual Lobe Specular Ratio': 0.6}
OWN = {'Share Glossy Inputs': False, 'Refraction Weight': 0.6, 'Refraction Roughness': 0.4, 'Refraction Glossiness': 0.3,
       'Refraction Color': [0.3, 0.6, 0.9]}
for mixing in (1, 2):
    uber_spheres(f'Calib-Mi-Mix{mixing}', [0.6, 0.3, 0.2], {'Base Mixing': mixing, **MIX, 'Glossy Weight': 0.5}, [
        ('M1 coat', {'Top Coat Weight': 0.8, 'Top Coat Roughness': 0.3, 'Top Coat Glossiness': 0.2}),
        ('M2 metal', {'Metallic Weight': 0.7}),
        ('M3 lobes', LOBES),
        ('M4 backscatter', {'Backscattering Weight': 0.5, 'Backscattering Roughness': 0.3, 'Backscattering Glossiness': 0.2,
                            'Share Glossy Inputs': False, 'Backscattering Color': [0.2, 0.4, 0.8]}),
        ('M5 flakes', {'Metallic Flakes Weight': 0.6, 'Metallic Flakes Roughness': 0.3, 'Metallic Flakes Glossiness': 0.2}),
        ('M6 own refraction', OWN),
        ('M7 metal full', {'Metallic Weight': 1.0}),
        ('M8 coat aniso', {'Top Coat Weight': 0.8, 'Top Coat Roughness': 0.3, 'Top Coat Glossiness': 0.2, 'Top Coat Anisotropy': 0.5}),
    ])
# Maps: how DAZ passes a channel's texture (gamma, the value as a factor,
# tiling), colour and float channels, Base Mixing 0 and 1.
uber_spheres('Calib-Mi-Maps', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3}, [
    ('P1 colour map', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4]}),
    ('P2 tiled', {('diffuse', 'image'): TILES, 'Horizontal Tiles': 3.0, 'Vertical Tiles': 2.0, 'Horizontal Offset': 0.25, 'Vertical Offset': 0.5}),
    ('P3 roughness map', {('Glossy Roughness', 'image'): G, 'Glossy Roughness': 0.7}),
    ('P4 glossiness map', {'Base Mixing': 1, ('Glossiness', 'image'): G, 'Glossiness': 0.6}),
    ('P5 metal map', {('Metallic Weight', 'image'): G, 'Metallic Weight': 0.8}),
    ('P6 gloss colour map', {('Glossy Color', 'image', 'float_color'): TILES, 'Glossy Color': [0.9, 0.5, 0.3]}),
    ('P7 translucency map', {'Translucency Weight': 0.5, ('Translucency Color', 'image', 'float_color'): TILES, 'Translucency Color': [0.7, 0.7, 0.7]}),
    ('P8 bump and coat map', {('Bump Strength', 'image'): G, 'Bump Strength': 0.6, ('Top Coat Weight', 'image'): G, 'Top Coat Weight': 0.7}),
])
# Map offsets (#65): Calib-Mi-Maps' 0.5 is a whole checker period, so it can't
# tell the sign or scale of an offset. 0.125 / 0.375 are not periodic; with
# tiles 1 (offset alone) and 2 x 2 (offset vs tile scale). Iray reference vs
# Octane decides how material.rs writes the transform's translation.
uber_spheres('Calib-Map-Offset', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3}, [
    ('O1 none', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4]}),
    ('O2 U 0.125', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4], 'Horizontal Offset': 0.125}),
    ('O3 V 0.125', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4], 'Vertical Offset': 0.125}),
    ('O4 V -0.375', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4], 'Vertical Offset': -0.375}),
    ('O5 tiles 2 U V 0.125', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4], 'Horizontal Tiles': 2.0, 'Vertical Tiles': 2.0, 'Horizontal Offset': 0.125, 'Vertical Offset': 0.125}),
    ('O6 tiles 3x2 V 0.125', {('diffuse', 'image'): TILES, 'diffuse': [0.8, 0.6, 0.4], 'Horizontal Tiles': 3.0, 'Vertical Tiles': 2.0, 'Vertical Offset': 0.125}),
])
# Per-map image modifications (#77): DAZ's texture-instance settings on one
# channel's map (`image_modification`) — tiles and tiling offsets against
# the material's tiles and offsets, scale / offset / invert, the grey
# channel. Only DAZ's .mi export decides (`file_texture`'s uvw transform,
# colour scale / offset, mono source). A ramp image (ours: red along U,
# green along V, a blue checker) makes value changes and the channel pick
# visible.
RAMP = '/Runtime/Textures/ORBIS/calib-imagemod-ramp.png'
ramp_path = os.path.join(os.path.dirname(out_dir), RAMP.lstrip('/'))
if not os.path.exists(ramp_path):
    from PIL import Image
    import numpy as np
    yy, xx = (np.mgrid[0:512, 0:512] + 0.5) / 512
    checker = ((np.floor(xx * 8) + np.floor(yy * 8)) % 2) * 0.8 + 0.1
    rgb = np.stack([0.1 + 0.8 * xx, 0.1 + 0.8 * (1 - yy), checker], -1)
    Image.fromarray((rgb * 255).round().astype(np.uint8)).save(ramp_path)
uber_spheres('Calib-Mi-ImageMod', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3}, [
    ('I1 map tiles 3x2', {('diffuse', 'image'): (TILES, {'horizontal_tiles': 3, 'vertical_tiles': 2}), 'diffuse': [0.8, 0.6, 0.4]}),
    ('I2 map and material tiles offsets', {('diffuse', 'image'): (TILES, {'horizontal_tiles': 2, 'vertical_tiles': 3, 'horizontal_tiling_offset': 0.125, 'vertical_tiling_offset': 0.25}),
                                           'Horizontal Tiles': 1.5, 'Vertical Tiles': 2.0, 'Horizontal Offset': 0.1, 'Vertical Offset': 0.2}),
    ('I3 colour scale offset', {('diffuse', 'image'): (RAMP, {'scale': 0.5, 'offset': 0.2}), 'diffuse': [0.8, 0.6, 0.4]}),
    ('I4 colour invert', {('diffuse', 'image'): (RAMP, {'invert': True}), 'diffuse': [0.8, 0.6, 0.4]}),
    ('I5 roughness invert scale offset', {('Glossy Roughness', 'image'): (RAMP, {'invert': True, 'scale': 1.5, 'offset': -0.1, 'horizontal_tiles': 2}), 'Glossy Roughness': 0.7}),
    ('I6 gloss weight red channel', {('Glossy Layered Weight', 'image'): (RAMP, {'grayscale_mode': 'redchannel', 'scale': 0.6, 'offset': 0.3}), 'Glossy Layered Weight': 0.9}),
    ('I7 bump tiles invert scale', {('Bump Strength', 'image'): (TILES, {'horizontal_tiles': 4, 'vertical_tiles': 4, 'invert': True, 'scale': 2}), 'Bump Strength': 0.6}),
    ('I8 per-map tiles differ', {('diffuse', 'image'): (TILES, {'horizontal_tiles': 0.5, 'vertical_tiling_offset': 0.375}), 'diffuse': [0.8, 0.6, 0.4],
                                 ('Glossy Roughness', 'image'): (RAMP, {'horizontal_tiles': 3, 'grayscale_mode': 'alpha'}), 'Glossy Roughness': 0.8, 'Vertical Tiles': 2.0}),
])
# The other grey channels, scale above 1 on colour maps, opacity and coat
# maps, and a normal map's tiles with the tiling offset 1 the Calib-Skin
# figures' normal maps carry.
uber_spheres('Calib-Mi-ImageMod2', [0.6, 0.6, 0.6], {'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3}, [
    ('J1 roughness green channel', {('Glossy Roughness', 'image'): (RAMP, {'grayscale_mode': 'greenchannel'}), 'Glossy Roughness': 0.8}),
    ('J2 roughness blue channel', {('Glossy Roughness', 'image'): (RAMP, {'grayscale_mode': 'bluechannel'}), 'Glossy Roughness': 0.8}),
    ('J3 roughness maximum', {('Glossy Roughness', 'image'): (RAMP, {'grayscale_mode': 'maximum'}), 'Glossy Roughness': 0.8}),
    ('J4 coat weight invert offset', {('Top Coat Weight', 'image'): (RAMP, {'invert': True, 'offset': 0.25}), 'Top Coat Weight': 0.8}),
    ('J5 opacity scale', {('Cutout Opacity', 'image'): (RAMP, {'scale': 0.8}), 'Cutout Opacity': 1.0}),
    ('J6 gloss colour scale 2', {('Glossy Color', 'image', 'float_color'): (RAMP, {'scale': 2}), 'Glossy Color': [0.9, 0.5, 0.3]}),
    ('J7 translucency colour scale', {'Translucency Weight': 0.5, ('Translucency Color', 'image', 'float_color'): (RAMP, {'scale': 1.5, 'offset': 0.1}), 'Translucency Color': [0.7, 0.7, 0.7]}),
    ('J8 normal map tiles', {('Normal Map', 'image'): (TILES, {'horizontal_tiles': 3, 'vertical_tiles': 3, 'horizontal_tiling_offset': 1, 'vertical_tiling_offset': 1}), 'Normal Map': 1.0}),
])
# Emission by Luminance Units: 0 cd/m², 1 kcd/m², 2 cd/ft², 3 kcd/ft², 4 lm, 5 W.
uber_spheres('Calib-Mi-Emit', [0.6, 0.6, 0.6], {'Emission Color': [1.0, 0.9, 0.8], 'Luminance': 300.0}, [
    (f'E{u} units {u}', {'Luminance Units': u}) for u in range(6)
] + [('E6 two sided', {'Luminance Units': 0, 'Two Sided Light': True}), ('E7 temperature', {'Emission Temperature': 3000.0})])

# SSS Reflectance Tint with Base Color Effect (#52): with translucency and
# an effect DAZ multiplies the diffuse by the linear tint — only while the
# Base Color Effect channel isn't saved hidden (the base scene's is; a
# strand hair's Uber had it shown). Each sphere one switch away from S4.
SSST = {'SSS Reflectance Tint': [0.9, 0.5, 0.3], 'Translucency Color': [0.8, 0.76, 0.69], 'Thin Walled': True}
STW = {'Translucency Weight': 0.06}


def shown_effect(v):
    """Base Color Effect saved shown (no "visible": false)."""
    return {'channel': {'id': 'Base Color Effect', 'type': 'enum', 'label': 'Base Color Effect', 'value': 0, 'current_value': v,
                        'enum_values': ['Scatter Only', 'Scatter & Transmit', 'Scatter & Transmit Intensity']}}


uber_spheres('Calib-Mi-SSSTint', [0.85, 0.81, 0.73], SSST, [
    ('S1 no translucency effect 2', {'Base Color Effect': shown_effect(2)}),
    ('S2 translucency effect 0', {**STW, 'Base Color Effect': shown_effect(0)}),
    ('S3 translucency effect 1', {**STW, 'Base Color Effect': shown_effect(1)}),
    ('S4 translucency effect 2', {**STW, 'Base Color Effect': shown_effect(2)}),
    ('S5 effect 2 hidden', {**STW, 'Base Color Effect': 2}),
    ('S6 effect 2 glossiness mode', {**STW, 'Base Color Effect': shown_effect(2), 'Base Mixing': 1}),
    ('S7 effect 2 closed translucency 0.5', {'Translucency Weight': 0.5, 'Base Color Effect': shown_effect(2), 'Thin Walled': False}),
    ('S8 effect 2 diffuse map', {**STW, 'Base Color Effect': shown_effect(2), ('diffuse', 'image'): RAMP, 'diffuse': [0.85, 0.81, 0.73]}),
])

# Channel visibility (#52 follow-up): the same eight spheres twice, every
# Uber channel group off its defaults — Calib-Mi-Visible keeps the base
# material's hidden channels (77 of 118 saved "visible": false),
# Calib-Mi-Visible-Shown saves them all shown. Their .mi tell which hidden
# channels DAZ treats otherwise (the SSS Reflectance Tint did).
VIS_SSS = {'Translucency Weight': 0.6, 'Translucency Color': [0.9, 0.6, 0.4], 'Base Color Effect': 2, 'SSS Reflectance Tint': [0.9, 0.7, 0.5],
           'Transmitted Color': [0.8, 0.5, 0.3], 'Transmitted Measurement Distance': 0.3, 'SSS Amount': 0.4, 'SSS Color': [0.7, 0.4, 0.3],
           'Scattering Measurement Distance': 0.2, 'SSS Direction': -0.4, 'Invert Transmission Normal': True, 'Diffuse Strength': 0.8, 'Diffuse Roughness': 0.3}
VIS_VARIANTS = [
    ('V1 mono sss', {**VIS_SSS, 'SSS Mode': 0}),
    ('V2 chromatic sss', {**VIS_SSS, 'SSS Mode': 1, 'Base Color Effect': 1}),
    ('V3 glossiness mode', {'Base Mixing': 1, 'Glossy Layered Weight': 0.8, 'Glossiness': 0.6, 'Glossy Specular': [0.5, 0.4, 0.3], 'Glossy Color': [0.9, 0.8, 0.6],
                            'Glossy Color Effect': 2, 'Glossy Anisotropy': 0.5, 'Glossy Anisotropy Rotations': 0.2, 'Glossiness Squared': True,
                            'Backscattering Weight': 0.4, 'Backscattering Color': [0.3, 0.6, 0.9], 'Backscattering Glossiness': 0.5, 'Backscattering Anisotropy': 0.3}),
    ('V4 dual lobe', {'Dual Lobe Specular Weight': 0.7, 'Dual Lobe Specular Reflectivity': 0.6, 'Specular Lobe 1 Roughness': 0.4, 'Specular Lobe 2 Roughness': 0.15,
                      'Dual Lobe Specular Ratio': 0.6, 'Roughness Squared': False}),
    ('V5 top coat', {'Top Coat Weight': 0.7, 'Top Coat Color': [0.8, 0.5, 0.3], 'Top Coat Color Effect': 2, 'Top Coat Roughness': 0.3, 'Top Coat Layering Mode': 3,
                     'Top Coat Curve Normal': 0.1, 'Top Coat Curve Grazing': 0.8, 'Top Coat Curve Exponent': 3.0, 'Top Coat Anisotropy': 0.4, 'Top Coat Rotations': 0.3,
                     'Top Coat Thin Film': 300.0, 'Top Coat Thin Film IOR': 1.7, 'Top Coat Bump Mode': 1, 'Top Coat Bump': 0.4}),
    ('V6 refraction', {'Refraction Weight': 0.7, 'Refraction Color': [0.7, 0.9, 0.6], 'Refraction Roughness': 0.2, 'Refraction Index': 1.6, 'Abbe': 30.0,
                       'Share Glossy Inputs': False, 'Base Thin Film': 250.0, 'Base Thin Film IOR': 1.8, 'Glossy Layered Weight': 0.5}),
    ('V7 flakes overlay', {'Metallic Flakes Weight': 0.6, 'Metallic Flakes Color': [0.9, 0.7, 0.3], 'Metallic Flakes Color Effect': 1, 'Metallic Flakes Roughness': 0.2,
                           'Metallic Flakes Size': 0.002, 'Metallic Flakes Strength': 0.7, 'Metallic Flakes Density': 0.6, 'Metallic Flakes Thin Film': 200.0,
                           'Metallic Flakes Thin Film IOR': 1.6, 'Diffuse Overlay Weight': 0.4, 'Diffuse Overlay Weight Squared': True,
                           'Diffuse Overlay Color': [0.3, 0.5, 0.8], 'Diffuse Overlay Color Effect': 2, 'Diffuse Overlay Roughness': 0.4}),
    ('V8 emission', {'Emission Color': [1.0, 0.8, 0.6], 'Emission Temperature': 4000.0, 'Luminance': 200.0, 'Luminance Units': 1, 'Luminous Efficacy': 20.0,
                     'Two Sided Light': True}),
]
uber_spheres('Calib-Mi-Visible', [0.7, 0.5, 0.4], {}, VIS_VARIANTS)
if made[-1][0]:
    d = load(made[-1][0])
    for m in d['material_library']:
        for e in m.get('extra', []):
            for c in e.get('channels', []) or []:
                c['channel'].pop('visible', None)
    p = os.path.join(out_dir, 'Calib-Mi-Visible-Shown.duf')
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)

# The same groups in the other mixing modes (gloss and backscatter
# anisotropy reached Iray hidden in mode 0 — Calib-Aniso — but not in
# mode 1; the coat curve not when hidden in mode 0).
VIS_GLOSS = {'Glossy Layered Weight': 0.8, 'Glossy Weight': 0.6, 'Glossy Roughness': 0.3, 'Glossiness': 0.6, 'Glossy Anisotropy': 0.5,
             'Glossy Anisotropy Rotations': 0.2, 'Backscattering Weight': 0.4, 'Backscattering Roughness': 0.4, 'Backscattering Glossiness': 0.5,
             'Backscattering Anisotropy': 0.3}
VIS_COAT = {'Top Coat Weight': 0.7, 'Top Coat Roughness': 0.3, 'Top Coat Glossiness': 0.6, 'Top Coat Layering Mode': 3, 'Top Coat Curve Normal': 0.1,
            'Top Coat Curve Grazing': 0.8, 'Top Coat Curve Exponent': 3.0, 'Top Coat Anisotropy': 0.4, 'Top Coat Rotations': 0.3}
uber_spheres('Calib-Mi-Visible2', [0.7, 0.5, 0.4], {}, [
    ('W1 gloss mode 0', {**VIS_GLOSS, 'Base Mixing': 0}),
    ('W2 gloss mode 1', {**VIS_GLOSS, 'Base Mixing': 1}),
    ('W3 gloss mode 2', {**VIS_GLOSS, 'Base Mixing': 2}),
    ('W4 coat mode 0', {**VIS_COAT, 'Base Mixing': 0}),
    ('W5 coat mode 1', {**VIS_COAT, 'Base Mixing': 1}),
    ('W6 coat mode 2', {**VIS_COAT, 'Base Mixing': 2, 'Glossy Weight': 0.5}),
    ('W7 tint mode 2', {**VIS_SSS, 'Base Mixing': 2, 'Base Color Effect': 1}),
    ('W8 rotation alone mode 1', {'Base Mixing': 1, 'Glossy Layered Weight': 0.8, 'Glossiness': 0.6, 'Glossy Anisotropy Rotations': 0.2}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for m in d['material_library']:
        for e in m.get('extra', []):
            for c in e.get('channels', []) or []:
                c['channel'].pop('visible', None)
    p = os.path.join(out_dir, 'Calib-Mi-Visible2-Shown.duf')
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)

# What opens hidden gloss anisotropy: the Glossy Roughness channel above 0
# (in glossiness mode too, where the roughness comes from Glossiness). V3's
# setting with one change per sphere.
VIS_V3 = dict(VIS_VARIANTS[2][1])
uber_spheres('Calib-Mi-Visible3', [0.7, 0.5, 0.4], {}, [
    ('Y1 mode 1 roughness 0', VIS_V3),
    ('Y2 mode 1 roughness 0.3', {**VIS_V3, 'Glossy Roughness': 0.3}),
    ('Y3 mode 1 glossy weight', {**VIS_V3, 'Glossy Weight': 0.6}),
    ('Y4 mode 1 backscatter roughness', {**VIS_V3, 'Backscattering Roughness': 0.4}),
    ('Y5 mode 1 roughness 0.01', {**VIS_V3, 'Glossy Roughness': 0.01}),
    ('Y6 mode 0 roughness 0.3', {**VIS_V3, 'Glossy Roughness': 0.3, 'Base Mixing': 0}),
    ('Y7 mode 0 roughness 0', {**VIS_V3, 'Base Mixing': 0}),
    ('Y8 mode 2 roughness 0', {**VIS_V3, 'Base Mixing': 2, 'Glossy Weight': 0.6}),
])

# Full refraction reflection (#120): Calib-Strand's Morgan nocoat (glossiness
# mode, shared inputs, Abbe 7.5, thin walled) got
# full_refraction_reflection_enable false from DAZ. A closed refracting
# sphere with shared glossy inputs in glossiness mode, one change per
# sphere; R8 is Morgan's channels in mode 0. -Shown: the same, every
# channel shown.
FR = {'Refraction Weight': 0.5, 'Refraction Index': 1.5, 'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.3, 'Glossiness': 0.6,
      'Share Glossy Inputs': True, 'Thin Walled': False, 'Abbe': 0.0}
FR_MORGAN = {'Base Mixing': 1, 'Glossy Layered Weight': 0.9795402, 'Share Glossy Inputs': True, 'Glossy Color': [0.9411765, 0.9137255, 0.8196079],
             'Glossy Color Effect': 2, 'Glossy Specular': [0.7215686, 0.7019608, 0.6156863], 'Glossiness': 0.8382353, 'Glossy Anisotropy': 0.7352941,
             'Glossy Anisotropy Rotations': 0.25, 'Translucency Weight': 0.0591133, 'Base Color Effect': 2,
             'Translucency Color': [0.8039216, 0.7647059, 0.6862745], 'Refraction Index': 1.794118, 'Refraction Weight': 0.1061947,
             'Abbe': 7.522124, 'Thin Walled': True}
uber_spheres('Calib-Mi-FullRefl', [0.7, 0.5, 0.4], FR, [
    ('R1 mode 0', {'Base Mixing': 0}),
    ('R2 mode 1', {'Base Mixing': 1}),
    ('R3 mode 2', {'Base Mixing': 2, 'Glossy Weight': 0.5}),
    ('R4 mode 1 abbe 7.5', {'Base Mixing': 1, 'Abbe': 7.5}),
    ('R5 mode 1 thin walled', {'Base Mixing': 1, 'Thin Walled': True}),
    ('R6 mode 1 layered weight 0.5', {'Base Mixing': 1, 'Glossy Layered Weight': 0.5}),
    ('R7 mode 1 Morgan', FR_MORGAN),
    ('R8 mode 0 Morgan', {**FR_MORGAN, 'Base Mixing': 0}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for m in d['material_library']:
        for e in m.get('extra', []):
            for c in e.get('channels', []) or []:
                c['channel'].pop('visible', None)
    p = os.path.join(out_dir, 'Calib-Mi-FullRefl-Shown.duf')
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)

# --- Top Coat Bump (#61): DAZ's MDL gives the coat its own normal
# (`calc_mix_layer_normal`): the Top Coat Bump (height or normal map, by Top
# Coat Bump Mode) blended with the base bump, else the base's bump / normal
# map. A sharp clear coat over a matte red base, so every highlight is the
# coat's. Synthetic maps (ours): dimples for the coat, ridges for the base,
# each as a height map and as the tangent-space normal map of that height.
BUMPS = {name: f'/Runtime/Textures/ORBIS/calib-coatbump-{name}.png' for name in ('dimples', 'dimples-n', 'ridges', 'ridges-n')}
if not all(os.path.exists(os.path.join(os.path.dirname(out_dir), p.lstrip('/'))) for p in BUMPS.values()):
    from PIL import Image
    import numpy as np
    n = 512
    yy, xx = np.mgrid[0:n, 0:n] / n
    heights = {'dimples': 0.5 + 0.5 * np.sin(2 * np.pi * 8 * xx) * np.sin(2 * np.pi * 8 * yy),
               'ridges': 0.5 + 0.5 * np.sin(2 * np.pi * 24 * yy)}
    for name, h in heights.items():
        Image.fromarray((h * 255).round().astype(np.uint8)).save(os.path.join(os.path.dirname(out_dir), BUMPS[name].lstrip('/')))
        # Slopes per UV unit, scaled so the steepest tilt is ~45 degrees;
        # OpenGL convention (green = +V, V up the image's rows reversed).
        dy, dx = np.gradient(h, 1.0 / n)
        s = 1.0 / max(np.abs(dx).max(), np.abs(dy).max())
        nx, ny, nz = -dx * s, dy * s, np.ones_like(h)
        ln = np.sqrt(nx * nx + ny * ny + nz * nz)
        rgb = np.stack([nx / ln, ny / ln, nz / ln], -1) * 0.5 + 0.5
        Image.fromarray((rgb * 255).round().astype(np.uint8)).save(os.path.join(os.path.dirname(out_dir), BUMPS[name + '-n'].lstrip('/')))
CB = {'Glossy Layered Weight': 0.0, 'Top Coat Weight': 1.0, 'Top Coat Layering Mode': 2, 'Top Coat IOR': 1.5, 'Top Coat Roughness': 0.05}
COAT_H = {('Top Coat Bump', 'image'): BUMPS['dimples'], 'Top Coat Bump': 1.0, 'Top Coat Bump Mode': 0}
uber_spheres('Calib-CoatBump', [0.45, 0.05, 0.04], CB, [
    ('B1 coat smooth', {}),
    ('B2 base bump', {('Bump Strength', 'image'): BUMPS['ridges'], 'Bump Strength': 1.0}),
    ('B3 coat bump', COAT_H),
    ('B4 coat bump 3', {**COAT_H, 'Top Coat Bump': 3.0}),
    ('B5 coat normal', {('Top Coat Bump', 'image'): BUMPS['dimples-n'], 'Top Coat Bump': 1.0, 'Top Coat Bump Mode': 1}),
    ('B6 base normal', {('Normal Map', 'image'): BUMPS['ridges-n'], 'Normal Map': 1.0}),
    ('B7 coat + base bump', {**COAT_H, ('Bump Strength', 'image'): BUMPS['ridges'], 'Bump Strength': 1.0}),
    ('B8 coat + base normal', {**COAT_H, ('Normal Map', 'image'): BUMPS['ridges-n'], 'Normal Map': 1.0}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] in BUMPS.values():
            i['map_gamma'] = 1
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))

# --- Diffuse Overlay with maps (#71): Iray Uber's overlay is a second
# diffuse at the overlay weight over the base. Synthetic maps (ours): a
# weight map of latitude belts at 1, 0.5, 0.25, 0 (twice), so squaring
# shows on the 0.5 / 0.25 belts, and a blue / yellow checker as the
# overlay colour. Weight map alone, colour map alone, both; Weight Squared
# on / off; the weight's value as the map's factor; overlay roughness.
OVL = {name: f'/Runtime/Textures/ORBIS/calib-overlay-{name}.png' for name in ('weight', 'colour')}
if not all(os.path.exists(os.path.join(os.path.dirname(out_dir), p.lstrip('/'))) for p in OVL.values()):
    from PIL import Image
    import numpy as np
    n = 256
    yy, xx = np.mgrid[0:n, 0:n]
    belts = np.array([1.0, 0.5, 0.25, 0.0])[(yy * 8 // n) % 4]
    Image.fromarray((belts * 255).round().astype(np.uint8)).save(os.path.join(os.path.dirname(out_dir), OVL['weight'].lstrip('/')))
    check = ((xx * 8 // n) + (yy * 8 // n)) % 2 == 0
    rgb = np.where(check[..., None], np.array([30, 80, 230]), np.array([235, 205, 40]))
    Image.fromarray(rgb.astype(np.uint8)).save(os.path.join(os.path.dirname(out_dir), OVL['colour'].lstrip('/')))
OW = {('Diffuse Overlay Weight', 'image'): OVL['weight']}
OC = {('Diffuse Overlay Color', 'image', 'float_color'): OVL['colour']}
uber_spheres('Calib-Overlay', [0.45, 0.05, 0.04], {}, [
    ('V1 weight map', {**OW, 'Diffuse Overlay Weight': 1.0, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V2 weight map squared', {**OW, 'Diffuse Overlay Weight': 1.0, 'Diffuse Overlay Weight Squared': True, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V3 weight map 0.5 squared', {**OW, 'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Weight Squared': True, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V4 colour map 0.5', {**OC, 'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V5 colour map 0.5 squared', {**OC, 'Diffuse Overlay Weight': 0.5, 'Diffuse Overlay Weight Squared': True, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V6 both maps', {**OW, **OC, 'Diffuse Overlay Weight': 1.0, 'Diffuse Overlay Color': [1.0, 1.0, 1.0]}),
    ('V7 both maps tint 0.6', {**OW, **OC, 'Diffuse Overlay Weight': 0.6, 'Diffuse Overlay Color': [0.5, 0.8, 1.0]}),
    ('V8 both maps rough 1', {**OW, **OC, 'Diffuse Overlay Weight': 1.0, 'Diffuse Overlay Color': [1.0, 1.0, 1.0], 'Diffuse Overlay Roughness': 1.0}),
])

# --- Bump vs the height map's resolution (#75). MDL's reference
# `base::file_bump_texture` (MDL SDK) takes the slope as the difference of
# the texels one either side, so a bump factor tilts the normal by
# factor × 2 × dh/du / width: the same pattern at 4x the pixels is 4x
# flatter, and UV tiling or the object's size don't change it. Octane's
# bump goes by the slope per UV instead. The Calib-CoatBump coat with the
# same 8 x 8 dimples as a 16-bit height map (8 bits would step at 4096 px)
# at 256 / 1024 / 4096 px; R4 / R5 / T2 should match R2 in Iray if the
# rule holds; S1 is R2 on a sphere at half size.
BRES = {n: f'/Runtime/Textures/ORBIS/calib-bumpres-dimples-{n}.png' for n in (256, 1024, 4096)}
if not all(os.path.exists(os.path.join(os.path.dirname(out_dir), p.lstrip('/'))) for p in BRES.values()):
    from PIL import Image
    import numpy as np
    for n, p in BRES.items():
        yy, xx = (np.mgrid[0:n, 0:n] + 0.5) / n
        h = 0.5 + 0.5 * np.sin(2 * np.pi * 8 * xx) * np.sin(2 * np.pi * 8 * yy)
        Image.fromarray((h * 65535).round().astype(np.uint16)).save(os.path.join(os.path.dirname(out_dir), p.lstrip('/')))
def bres(n, f, tiles=1.0):
    d = {('Top Coat Bump', 'image'): BRES[n], 'Top Coat Bump': f, 'Top Coat Bump Mode': 0}
    if tiles != 1.0:
        d.update({'Horizontal Tiles': tiles, 'Vertical Tiles': tiles})
    return d
uber_spheres('Calib-BumpRes', [0.45, 0.05, 0.04], CB, [
    ('R1 256 px f4', bres(256, 4.0)),
    ('R2 1024 px f4', bres(1024, 4.0)),
    ('R3 4096 px f4', bres(4096, 4.0)),
    ('R4 4096 px f16', bres(4096, 16.0)),
    ('R5 256 px f1', bres(256, 1.0)),
    ('T1 1024 px f4 tiles 2', bres(1024, 4.0, 2.0)),
    ('T2 256 px f1 tiles 4', bres(256, 1.0, 4.0)),
    ('S1 1024 px f4 half size', bres(1024, 4.0)),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] in BRES.values():
            i['map_gamma'] = 1
    s1 = next(n for n in d['scene']['nodes'] if n.get('label', '').startswith('S1'))
    lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == s1['url'])
    lib['general_scale']['current_value'] = 0.5
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))

# --- Calib-Mi-ImageGamma (#91, .mi only): maps the scene's image_library
# doesn't list. The Calib-Skin spheres' cropped maps, saved without
# entries, went to Iray at gamma 1 (their channels' default_image_gamma),
# while Calib-Mi-AoA's calib-tiles.png, also unlisted, went "auto" on its
# Normal Map (default 1) - the same file was on the diffuse (default 0)
# too. One file per channel alone, then one file on two channels in either
# order: which channel's default does DAZ give the image?
IG = {k: f'/Runtime/Textures/ORBIS/calib-imggamma-{k}.png' for k in 'abcdef'}
for p in IG.values():
    p = os.path.join(os.path.dirname(out_dir), p.lstrip('/'))
    if not os.path.exists(p):
        from PIL import Image
        Image.new('L', (64, 64), 128).save(p)
uber_spheres('Calib-Mi-ImageGamma', GREY_SPHERE, {}, [
    ('I1 diffuse map', {('diffuse', 'image'): IG['a']}),
    ('I2 bump map', {('Bump Strength', 'image'): IG['b'], 'Bump Strength': 1.0}),
    ('I3 normal map', {('Normal Map', 'image'): IG['c'], 'Normal Map': 1.0}),
    ('I4 roughness map', {('Glossy Roughness', 'image'): IG['d'], 'Glossy Layered Weight': 1.0, 'Glossy Roughness': 0.5}),
    ('I5 diffuse map shared', {('diffuse', 'image'): IG['e']}),
    ('I6 bump map shared', {('Bump Strength', 'image'): IG['e'], 'Bump Strength': 1.0}),
    ('I7 bump map shared first', {('Bump Strength', 'image'): IG['f'], 'Bump Strength': 1.0}),
    ('I8 diffuse map shared second', {('diffuse', 'image'): IG['f']}),
])
if made[-1][0]:
    d = load(made[-1][0])
    d['image_library'] = [i for i in d.get('image_library', []) if i['map'][0]['url'] not in IG.values()]
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))

# --- Height-map gamma (#91): Calib-Leaf's three vertical cards, lit from
# the front 45 degrees up (the distant light turned round), Scene Only, tone
# mapping linear (burn 1, crush 0). A synthetic 128 px height map whose v
# runs up the card, in the light's plane: four bands of 32 texels across u -
# a sawtooth rising 8 levels a texel along v, the same falling, a 0.5 grey
# (128) with a step to 160 every 32 texels, and a flat 128. The rising and
# the falling band tilt the normal by the same slope away from / towards
# the light, so (I_fall - I_rise) / (I_fall + I_rise) = f s tan 45 per row,
# with no fitted scale: a 1.0 decode makes the sawtooth's slope constant
# along v, a 2.2 decode grows it as h^1.2 (zero at the bottom, 2.2 x the
# linear slope at the top, equal near 0.5). The same map at `map_gamma` 1
# and as a 16-bit PNG (#90: Iray reads a 16-bit single channel at 1.0)
# beside it; normal maps of a known tilt (0.25 up / down) at gamma auto and
# 1 in Calib-BumpGamma-Uber2. Calib-BumpGamma-NVIDIA: the same cards in
# NVIDIA's Concrete (`file_bump_texture` on the whole surface), its colour
# a flat grey map, Overall Reflectivity 0 (a plain diffuse); built headless
# by daz-build-leaf-scene.dsa with the Concrete preset, tuned here once.
BG = {name: f'/Runtime/Textures/ORBIS/calib-bumpgamma{name}.png' for name in ('', '-g1', '-16', '-n', '-n-g1', '-grey')}
if not all(os.path.exists(os.path.join(os.path.dirname(out_dir), p.lstrip('/'))) for p in BG.values()):
    from PIL import Image
    import numpy as np
    n = 128
    j = (n - 1 - np.arange(n))[:, None] * np.ones((1, n), int)  # v texel index, 0 at the bottom row
    band = np.ones((n, 1), int) * (np.arange(n)[None, :] // 32)
    h = np.select([band == 0, band == 1, band == 2], [8 * (j % 32), 248 - 8 * (j % 32), np.where(j % 64 < 32, 128, 160)], 128)
    def put(name, img):
        img.save(os.path.join(os.path.dirname(out_dir), BG[name].lstrip('/')))
    for name in ('', '-g1'):
        put(name, Image.fromarray(h.astype(np.uint8)))
    put('-16', Image.fromarray((h * 257).astype(np.uint16)))
    t = np.select([band == 0, band == 1], [0.25, -0.25], 0.0)  # tilt n_y / n_z, +v up the card
    nv = np.stack([np.zeros_like(t), t, np.ones_like(t)], -1)
    nv /= np.linalg.norm(nv, axis=-1, keepdims=True)
    for name in ('-n', '-n-g1'):
        put(name, Image.fromarray(((nv + 1) / 2 * 255).round().astype(np.uint8)))
    put('-grey', Image.fromarray(np.full((n, n), 186, np.uint8)))  # 0.5 at 2.2
BG_F = 4.0


def bump_gamma_cards(d, variants, nvidia):
    """Label Calib-Leaf's cards, turn the light to the front, give each card
    its map (path, channel, factor) and set the images' `map_gamma`."""
    sun = next(l for l in d['node_library'] if l['id'].startswith('DistantLight'))
    next(r for r in sun['rotation'] if r['id'] == 'y')['current_value'] = 0.0
    # A stop darker than Calib-Leaf (8.1): lit from the front its cards came out
    # at 0.8 linear, the tilt towards the light would clip.
    # Calib-L-Distant-45's (the NVIDIA cards' base) exposure.
    set_channel(node_channels(d, 'Tonemapper Options'), 'Exposure Value', 9.1, 'float')
    set_channel(node_channels(d, 'Tonemapper Options'), 'Shutter Speed', 8.574187, 'float')
    # A black floor: its bounce lit the cards by 45 % of the light's share
    # (an unknown ambient that dilutes the tilt's contrast); the distant
    # light alone leaves the ratio exact. (Hidden, DAZ's .mi moved the dome
    # ground's 1 mm drop, untested; the floor has no gloss.)
    floor = next(n for n in d['scene']['nodes'] if n.get('label') == 'Plane')
    fm = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + floor['geometries'][0]['id'])
    fm['diffuse']['channel']['current_value'] = [0.0, 0.0, 0.0]
    cards = sorted((n for n in d['scene']['nodes'] if n.get('label', '').startswith('Leaf ')), key=lambda n: n['label'])
    mlib = {m['id']: m for m in d['material_library']}
    lib_images = d.setdefault('image_library', [])
    gammas = {}
    for card, (label, path, cid, f, gamma) in zip(cards, variants):
        card['label'] = label
        lib = next(l for l in d['node_library'] if '#' + l['id'].replace('(', '%28').replace(')', '%29') == card['url'])
        lib['label'] = label
        m = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + card['geometries'][0]['id'])
        extra = m.setdefault('extra', [])
        sc = next((e for e in extra if e.get('type') == 'studio_material_channels'), None)
        if sc is None:
            sc = {'type': 'studio_material_channels', 'channels': []}
            extra.append(sc)
        if nvidia:
            sc['channels'] = [c for c in sc['channels'] if c['channel']['id'] not in ('Diffuse Color', 'Bump Strength', 'glossy_weight')]
            sc['channels'] += [{'channel': {'id': 'Diffuse Color', 'type': 'image', 'image_file': BG['-grey']}},
                               {'channel': {'id': 'Bump Strength', 'type': 'float', 'current_value': f, 'image_file': path}},
                               {'channel': {'id': 'glossy_weight', 'type': 'float', 'current_value': 0.0}}]
            gammas[BG['-grey']] = 0
        else:
            ch = next(e for e in mlib[m['url'].lstrip('#')]['extra'] if e['type'] == 'studio_material_channels')['channels']
            grey = [0.7297, 0.7297, 0.7297]  # 0.5 linear, as the NVIDIA cards' grey map
            for x in (m, mlib[m['url'].lstrip('#')]):
                x['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': grey}}
            for k, v in {'Base Mixing': 0, 'Glossy Layered Weight': 0.0, 'Metallic Weight': 0.0, 'Translucency Weight': 0.0,
                         'Thin Walled': False, 'Diffuse Roughness': 0.0}.items():
                set_channel(ch, k, v, TYPES.get(type(v), 'float'))
            sc['channels'] = [c for c in sc['channels'] if c['channel']['id'] != cid]
            sc['channels'].append({'channel': {'id': cid, 'type': 'float', 'current_value': f, 'image_file': path}})
        gammas[path] = gamma
    for path, gamma in gammas.items():
        lib_images[:] = [i for i in lib_images if i['map'][0]['url'] != path]
        lib_images.append({'id': path, 'name': os.path.basename(path), 'map_gamma': gamma, 'map': [{'url': path, 'label': os.path.basename(path)}]})


if os.path.exists(leaf):
    for name, variants in {
        'Calib-BumpGamma-Uber': [('B1 height 8-bit auto', BG[''], 'Bump Strength', BG_F, 0),
                                 ('B2 height 8-bit gamma 1', BG['-g1'], 'Bump Strength', BG_F, 1),
                                 ('B3 height 16-bit auto', BG['-16'], 'Bump Strength', BG_F, 0)],
        'Calib-BumpGamma-Uber2': [('B4 normal 8-bit auto', BG['-n'], 'Normal Map', 1.0, 0),
                                  ('B5 normal 8-bit gamma 1', BG['-n-g1'], 'Normal Map', 1.0, 1),
                                  ('B6 height 8-bit auto half', BG[''], 'Bump Strength', BG_F / 2, 0)]}.items():
        d = load(leaf)
        bump_gamma_cards(d, variants, False)
        made.append((save(d, name), None))
bgn = os.path.join(out_dir, 'Calib-BumpGamma-NVIDIA.duf')
if os.path.exists(bgn):
    d = load(bgn)
    if any(n.get('label', '').startswith('Leaf ') for n in d['scene']['nodes']):
        bump_gamma_cards(d, [('N1 height 8-bit auto', BG[''], None, BG_F, 0),
                             ('N2 height 8-bit gamma 1', BG['-g1'], None, BG_F, 1),
                             ('N3 height 16-bit auto', BG['-16'], None, BG_F, 0)], True)
        duf_camera.ensure(d)  # DAZ saved it through its Perspective View (#97)
        open(bgn, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        print(bgn, '(cards tuned)')
# Calib-NVIDIA-BumpG1: Calib-NVIDIA with its bump maps at `map_gamma` 1, so
# Iray reads them linear - Cast Iron's band-passed detail (#89, linear
# 1.05 x, 2.2 1.13 x) with Iray's decode known: does the measure itself
# favour one decode? Cast Iron's MDL reads the same map as its roughness,
# and the presets' gloss tint too (#90), in Iray and the export alike.
nv = os.path.join(out_dir, 'Calib-NVIDIA.duf')
if os.path.exists(nv):
    d = load(nv)
    for i in d['image_library']:
        if i['map'][0]['url'] in ('/resources/brushed_a.jpg', '/resources/grunge_d.jpg', '/resources/concrete_plain_bump.png',
                                  '/resources/concrete_fine_polished_bump.png'):
            i['map_gamma'] = 1
    made.append((save(d, 'Calib-NVIDIA-BumpG1'), None))
# Cast Iron's detail (#92): Octane's band-passed detail falls as its bump
# grows (the sub-pixel normal spread blurs the reflection: bump off 1.66 x
# Iray, on 1.30 x, twice the slope 1.07 x), so Iray's spread looks about
# twice the bake's, though Iray's slope matches it on a magnified card
# (Calib-BumpGamma's step). Calib-NVIDIA-CastFlat: Cast Iron's bump off
# (Bump Strength 0, the map kept) - is Iray's gap in the bump or the base
# lobe? Calib-NVIDIA-NoComp: texture compression off (both thresholds
# above every map; DAZ hands 2048 px maps such as grunge_d at "high") -
# does Iray's lossy compression add the spread?
if os.path.exists(nv):
    d = load(nv)
    for m in d['scene']['materials']:
        if 'Cast%20Iron' in m.get('url', ''):
            for e in m.get('extra', []):
                for c in e.get('channels', []):
                    if c['channel']['id'] == 'Bump Strength':
                        c['channel']['current_value'] = 0
    made.append((save(d, 'Calib-NVIDIA-CastFlat'), None))
    d = load(nv)
    for e in d['scene'].get('extra', []):
        for el in e.get('render_options', {}).get('render_elements', []):
            for ch in el.get('children', []):
                for c in ch.get('channels', []):
                    if c['channel']['id'] in ('Texture Compression Medium Threshold', 'Texture Compression High Threshold'):
                        c['channel']['current_value'] = 16384
    made.append((save(d, 'Calib-NVIDIA-NoComp'), None))
# Metal Brushed's disk (#98): Octane 1.16 x Iray, and none of Octane's own
# switches (bump off, weight or lobe flat) moves it. So the Iray side, one
# part at a time: Calib-NVIDIA-Brushed puts Metal Brushed on all eight
# spheres (one MDL module: one DAZ session, #94), N<k> in Calib-NVIDIA's
# order, with flat maps from calib-disp-* (grey 128 = 0.218 linear: weight
# 0.609, lobe u 0.107 / v 0.421). The geometry is Calib-NVIDIA's: its dump
# serves this scene with the scene path changed.
if os.path.exists(nv):
    d = load(nv)
    mats = d['scene']['materials']
    brushed = next(m for m in mats if 'Metal%20Brushed' in m.get('url', ''))
    geo_label = {g['id']: n['label'] for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    WG, WW, WB = (MAPS + f'calib-disp-{c}.png' for c in ('grey', 'white', 'black'))
    BR = [('B1 brushed', {}),
          ('B2 bump 0', {'Bump Strength': 0.0}),
          ('B3 weight flat', {'brush_texture': WG}),
          ('B4 lobe flat', {'roughness_texture_u': WG, 'roughness_texture_v': WG}),
          ('B5 all flat bump 0', {'brush_texture': WG, 'roughness_texture_u': WG, 'roughness_texture_v': WG, 'Bump Strength': 0.0}),
          ('B6 near iso flat bump 0', {'brush_texture': WG, 'roughness_texture_u': WW, 'roughness_texture_v': WB, 'Bump Strength': 0.0}),
          ('B7 weight black', {'brush_texture': WB}),
          ('B8 weight white', {'brush_texture': WW})]
    for i, m in enumerate(mats):
        label = geo_label.get(m['geometry'].lstrip('#'))
        if not label or not label.startswith('N'):
            continue
        k = int(label[1]) - 1
        new = copy.deepcopy(brushed)
        new.update(id=m['id'], geometry=m['geometry'], uv_set=m.get('uv_set', brushed.get('uv_set')))
        ch = next(e for e in new['extra'] if e['type'] == 'studio_material_channels')['channels']
        for cid, v in BR[k][1].items():
            c = next(c for c in ch if c['channel']['id'] == cid)['channel']
            if isinstance(v, str):
                c['image_file'] = v
                if not any(im['id'] == v for im in d['image_library']):
                    d['image_library'].append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
            else:
                c['current_value'] = v
        mats[i] = new
        for n in d['scene']['nodes']:
            if n.get('label') == label:
                n['label'] = BR[k][0]
    made.append((save(d, 'Calib-NVIDIA-Brushed'), None))
# Iray's Phong lobe in a furnace (#98): white Uber metals (Metallic Weight 1,
# base white: the metal's curve is 1 head-on and grazing, so the reflection
# is `simple_glossy_bsdf` alone) at Iray roughness r (DAZ squares Glossy
# Roughness: 0.2236 -> 0.05), under a uniform dome (our own constant HDR,
# 0.5) with the floor hidden: each sphere over the background is the lobe's
# directional albedo at that view angle. Octane's is measured (#95).
FURNACE = '/Runtime/Textures/ORBIS/calib-furnace.hdr'
p = os.path.join(os.path.dirname(out_dir), FURNACE.lstrip('/'))
if not os.path.exists(p):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    # Radiance RGBE, flat scanlines: every texel (128, 128, 128, 128) = 0.5.
    open(p, 'wb').write(b'#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 32 +X 64\n' + bytes([128] * 4 * 64 * 32))
uber_spheres('Calib-Furnace-Metal', [1.0, 1.0, 1.0], {'Metallic Weight': 1.0}, [
    (f'F{k + 1} metal r {r}', {'Glossy Roughness': round(r ** 0.5, 4)}) for k, r in enumerate((0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.7, 1.0))])
if made[-1][0]:
    d = load(made[-1][0])
    for n in d['scene']['nodes']:
        if n.get('label') == 'Plane':
            n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}])
            ext = next(e for e in n['extra'] if e.get('type') == 'studio_node_channels')
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
        for e in n.get('extra', []):
            for c in e.get('channels') or []:
                if c['channel']['id'] == 'Environment Map':
                    c['channel']['image_file'] = FURNACE
                    c['channel']['current_value'] = 1
    d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
        {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Calib-Furnace-Brushed: Calib-NVIDIA-Brushed under the same dome, the floor
# hidden - NVIDIA's `metal_generic` against the Uber's lobe, ring by ring.
nvb = os.path.join(out_dir, 'Calib-NVIDIA-Brushed.duf')
if os.path.exists(nvb):
    d = load(nvb)
    for n in d['scene']['nodes']:
        if n.get('label') == 'Plane':
            ext = next(e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}]) if e.get('type') == 'studio_node_channels')
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
        for e in n.get('extra', []):
            for c in e.get('channels') or []:
                if c['channel']['id'] == 'Environment Map':
                    c['channel']['image_file'] = FURNACE
                    c['channel']['current_value'] = 1
    d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
        {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    made.append((save(d, 'Calib-Furnace-Brushed'), None))
# Calib-Furnace-MetalBump (#111): Calib-Furnace-Metal's white Uber metals,
# F5 - F8 with a flat grey bump map at Bump Strength 1 - a zero gradient, so
# the normal is unchanged, but the layers' normal becomes a
# `base::file_bump_texture` call, as in NVIDIA's `metal_generic` (which
# darkens towards the rim in Iray; the Uber metal, `state::normal()`, doesn't).
uber_spheres('Calib-Furnace-MetalBump', [1.0, 1.0, 1.0], {'Metallic Weight': 1.0}, [
    (f'F{k + 1} metal r {r}{" flat bump" if bump else ""}', {'Glossy Roughness': round(r ** 0.5, 4),
     **({('Bump Strength', 'image'): G, 'Bump Strength': 1.0} if bump else {})})
    for k, (bump, r) in enumerate([(b, r) for b in (False, True) for r in (0.05, 0.1, 0.2, 0.4)])])
if made[-1][0]:
    d = load(made[-1][0])
    for n in d['scene']['nodes']:
        if n.get('label') == 'Plane':
            n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}])
            ext = next(e for e in n['extra'] if e.get('type') == 'studio_node_channels')
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
        for e in n.get('extra', []):
            for c in e.get('channels') or []:
                if c['channel']['id'] == 'Environment Map':
                    c['channel']['image_file'] = FURNACE
                    c['channel']['current_value'] = 1
    d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
        {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Calib-Furnace-CoatTint (#111): the same lobe's tint against its weight -
# NVIDIA's metals tint `simple_glossy_bsdf` (0.65, 0.543), the Uber metals
# don't (white lobe, colour through the curve). The Uber's Weighted top coat
# is `weighted_layer(weight, simple_glossy_bsdf(tint: Top Coat Color))`,
# here over a black base (diffuse 0, no gloss): colour 1 / 0.65 / 0.4 at
# weight 1 and colour 1 at weight 0.65, at Iray r 0.1 and 0.4.
uber_spheres('Calib-Furnace-CoatTint', [0.0, 0.0, 0.0],
             {'Glossy Layered Weight': 0.0, 'Top Coat Layering Mode': 1, 'Top Coat Color Effect': 0}, [
    (f'C{k + 1} coat {c} w {w} r {r}', {'Top Coat Weight': w, 'Top Coat Color': [c, c, c], 'Top Coat Roughness': round(r ** 0.5, 4)})
    for k, (r, (c, w)) in enumerate([(r, cw) for r in (0.1, 0.4) for cw in ((1.0, 1.0), (0.65, 1.0), (1.0, 0.65), (0.4, 1.0))])])
if made[-1][0]:
    d = load(made[-1][0])
    for n in d['scene']['nodes']:
        if n.get('label') == 'Plane':
            n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}])
            ext = next(e for e in n['extra'] if e.get('type') == 'studio_node_channels')
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
        for e in n.get('extra', []):
            for c in e.get('channels') or []:
                if c['channel']['id'] == 'Environment Map':
                    c['channel']['image_file'] = FURNACE
                    c['channel']['current_value'] = 1
    d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
        {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Resin's level (#113): NVIDIA's Resin Polyurethane Matte on all eight of
# Calib-NVIDIA2's spheres (one MDL module: one DAZ session, #94), one part
# at a time - the base tint (grey 0.9 sRGB = A7, white), the roughness
# (0.02 / 0.1 / 0.25 / 0.5) and the coat (Glossy Weight 0). Calib-Resin
# under Calib-NVIDIA2's HDRI and floor, Calib-Furnace-Resin under the
# uniform dome, the floor hidden. The geometry is Calib-NVIDIA2's: its dump
# serves both with the scene path and the sphere labels (A<k> -> R<k> ...)
# changed.
RESIN = [('R1 grey r 0.25', 0.9, 0.25, None), ('R2 grey r 0.02', 0.9, 0.02, None),
         ('R3 white r 0.25', 1.0, 0.25, None), ('R4 white r 0.02', 1.0, 0.02, None),
         ('R5 grey r 0.25 coat 0', 0.9, 0.25, 0.0), ('R6 white r 0.25 coat 0', 1.0, 0.25, 0.0),
         ('R7 grey r 0.5', 0.9, 0.5, None), ('R8 grey r 0.1', 0.9, 0.1, None)]
# Calib-Furnace-Resin2 (#115): the uncoated resin's roughness alone (white,
# Glossy Weight 0, r 0.02 - 1), R6 and R3 again as anchors.
RESIN2 = [('U1 white r 0.02 coat 0', 1.0, 0.02, 0.0), ('U2 white r 0.1 coat 0', 1.0, 0.1, 0.0),
          ('U3 white r 0.25 coat 0', 1.0, 0.25, 0.0), ('U4 white r 0.5 coat 0', 1.0, 0.5, 0.0),
          ('U5 white r 0.75 coat 0', 1.0, 0.75, 0.0), ('U6 white r 1 coat 0', 1.0, 1.0, 0.0),
          ('U7 grey r 0.5 coat 0', 0.9, 0.5, 0.0), ('U8 white r 0.25', 1.0, 0.25, None)]
nv2 = os.path.join(out_dir, 'Calib-NVIDIA2.duf')
for name, furnace, RESIN in (('Calib-Resin', False, RESIN), ('Calib-Furnace-Resin', True, RESIN), ('Calib-Furnace-Resin2', True, RESIN2)):
    if not os.path.exists(nv2):
        break
    d = load(nv2)
    mats = d['scene']['materials']
    resin = next(m for m in mats if 'Resin%20Polyurethane' in m.get('url', ''))
    geo_label = {g['id']: n['label'] for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    for i, m in enumerate(mats):
        label = geo_label.get(m['geometry'].lstrip('#'))
        if not label or not label.startswith('A'):
            continue
        k = int(label[1]) - 1
        lab, grey, rough, weight = RESIN[k]
        new = copy.deepcopy(resin)
        new.update(id=m['id'], geometry=m['geometry'], uv_set=m.get('uv_set', resin.get('uv_set')))
        new['diffuse']['channel']['current_value'] = [grey] * 3
        ch = next(e for e in new['extra'] if e['type'] == 'studio_material_channels')['channels']
        ch[:] = [c for c in ch if c['channel']['id'] not in ('roughness', 'glossy_weight')]
        ch.append({'channel': {'id': 'roughness', 'type': 'float', 'current_value': rough}})
        if weight is not None:
            ch.append({'channel': {'id': 'glossy_weight', 'type': 'float', 'current_value': weight}})
        mats[i] = new
        for n in d['scene']['nodes']:
            if n.get('label') == label:
                n['label'] = lab
    if furnace:
        for n in d['scene']['nodes']:
            if n.get('label') == 'Plane':
                ext = next(e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}]) if e.get('type') == 'studio_node_channels')
                ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
            for e in n.get('extra', []):
                for c in e.get('channels') or []:
                    if c['channel']['id'] == 'Environment Map':
                        c['channel']['image_file'] = FURNACE
                        c['channel']['current_value'] = 1
        d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
            {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    made.append((save(d, name), None))

# Architectural glass (#115): Calib-NVIDIA3's spheres all transparency 1,
# one part at a time - the transmit lobe's refr_gloss alone (reflectivity
# 0: no layer weights; 1 / 0.6 / 0.4 / 0.2 / 0), the reflection on top
# (reflectivity 0.6, Schlick's curve) smooth and at 0.2, and at 0.2 with
# Use Fresnel Term (fresnel_layer at IOR 1.5). Calib-ArchGlass under
# Calib-NVIDIA3's HDRI and floor, Calib-Furnace-ArchGlass under the
# uniform dome, the floor hidden; one MDL module, one DAZ session each
# (#94). Geometry from Calib-NVIDIA3's dump (labels A<k> -> G<k> ...).
ARCH_GLASS = [('G1 refr 1', 1.0, 0.0, False), ('G2 refr 0.6', 0.6, 0.0, False),
              ('G3 refr 0.4', 0.4, 0.0, False), ('G4 refr 0.2', 0.2, 0.0, False),
              ('G5 refr 0', 0.0, 0.0, False), ('G6 refr 1 refl 0.6', 1.0, 0.6, False),
              ('G7 refr 0.2 refl 0.6', 0.2, 0.6, False), ('G8 refr 0.2 refl 0.6 fresnel', 0.2, 0.6, True)]
nv3 = os.path.join(out_dir, 'Calib-NVIDIA3.duf')
for name, furnace in (('Calib-ArchGlass', False), ('Calib-Furnace-ArchGlass', True)):
    if not os.path.exists(nv3):
        break
    d = load(nv3)
    mats = d['scene']['materials']
    geo_label = {g['id']: n['label'] for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    glass = next(m for m in mats if geo_label.get(m['geometry'].lstrip('#'), '').startswith('A8'))
    for i, m in enumerate(mats):
        label = geo_label.get(m['geometry'].lstrip('#'))
        if not label or not label.startswith('A'):
            continue
        lab, refr, refl, fres = ARCH_GLASS[int(label[1]) - 1]
        new = copy.deepcopy(glass)
        new.update(id=m['id'], geometry=m['geometry'], uv_set=m.get('uv_set', glass.get('uv_set')))
        ch = next(e for e in new['extra'] if e['type'] == 'studio_material_channels')['channels']
        ch[:] = [c for c in ch if c['channel']['id'] not in ('refr_gloss', 'reflectivity', 'brdf_fresnel')]
        ch.append({'channel': {'id': 'refr_gloss', 'type': 'float', 'current_value': refr}})
        ch.append({'channel': {'id': 'reflectivity', 'type': 'float', 'current_value': refl}})
        if fres:
            ch.append({'channel': {'id': 'brdf_fresnel', 'type': 'bool', 'current_value': True}})
        mats[i] = new
        for n in d['scene']['nodes']:
            if n.get('label') == label:
                n['label'] = lab
    if furnace:
        for n in d['scene']['nodes']:
            if n.get('label') == 'Plane':
                ext = next(e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}]) if e.get('type') == 'studio_node_channels')
                ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
            for e in n.get('extra', []):
                for c in e.get('channels') or []:
                    if c['channel']['id'] == 'Environment Map':
                        c['channel']['image_file'] = FURNACE
                        c['channel']['current_value'] = 1
        d['image_library'] = [i for i in d['image_library'] if 'RuinsB' not in i['map'][0]['url']] + [
            {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]
    made.append((save(d, name), None))


def furnace_dome(d):
    """The uniform dome (FURNACE), the floor hidden."""
    for n in d['scene']['nodes']:
        if n.get('label') == 'Plane':
            ext = next(e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}]) if e.get('type') == 'studio_node_channels')
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
        for e in n.get('extra', []):
            for c in e.get('channels') or []:
                if c['channel']['id'] == 'Environment Map':
                    c['channel']['image_file'] = FURNACE
                    c['channel']['current_value'] = 1
    d['image_library'] = [i for i in d.get('image_library', []) if 'RuinsB' not in i['map'][0]['url'] and i['id'] != 'calib-furnace.hdr'] + [
        {'id': 'calib-furnace.hdr', 'name': 'calib-furnace.hdr', 'map_gamma': 1, 'map_type': 'LatLong', 'map': [{'url': FURNACE, 'label': 'calib-furnace.hdr'}]}]


def render_option(d, cid, value):
    """Set one of the scene's Iray render settings channels."""
    def walk(o):
        if isinstance(o, dict):
            c = o.get('channel')
            if isinstance(c, dict) and c.get('id') == cid:
                yield c
            for v in o.values():
                yield from walk(v)
        elif isinstance(o, list):
            for v in o:
                yield from walk(v)
    found = list(walk(next(e for e in d['scene']['extra'] if e.get('type') == 'studio_render_settings')))
    assert len(found) == 1, (cid, len(found))
    found[0]['current_value'] = value


# Iray's rough transmission (#116). Calib-Furnace-UberGlass: the same df as
# architectural's transmit lobe (`simple_glossy_bsdf`, scatter_transmit:
# Refraction Weight 1, no gloss, own roughness, squared) through DAZ's Uber
# at MDL roughness 0.019 / 0.051 / 0.134 / 0.354 (architectural_gloss_to_rough
# at refr_gloss 0.6 / 0.4 / 0.2 / 0), 0.218, 0.6, and resin's
# scatter_reflect_transmit (shared gloss, full Fresnel reflection) at 0.25 /
# 0.5 - NVIDIA's material or Iray's lobe. -PL3: the same at Max Path Length
# 3 (camera, two surface events, dome: no light that bounced inside).
UBER_GLASS = [(f'T{k + 1} transmit r {r}', {'Refraction Weight': 1.0, 'Share Glossy Inputs': False, 'Refraction Roughness': round(r ** 0.5, 4)})
              for k, r in enumerate((0.019, 0.051, 0.134, 0.218, 0.354, 0.6))] + [
             (f'T{k + 7} reflect_transmit r {r}', {'Refraction Weight': 1.0, 'Glossy Layered Weight': 1.0, 'Glossy Roughness': round(r ** 0.5, 4)})
              for k, r in enumerate((0.25, 0.5))]
for name, path_len in (('Calib-Furnace-UberGlass', None), ('Calib-Furnace-UberGlass-PL3', 3)):
    uber_spheres(name, [1.0, 1.0, 1.0], {'Glossy Layered Weight': 0.0, 'Thin Walled': False, 'Refraction Index': 1.5, 'Share Glossy Inputs': True}, UBER_GLASS)
    if made[-1][0]:
        d = load(made[-1][0])
        furnace_dome(d)
        if path_len:
            render_option(d, 'Max Path Length', path_len)
        open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
# Calib-Furnace-ArchGlass2 (#116): the transmit lobe alone (reflectivity 0)
# with the Firefly Filter off - refr_gloss 0.6 / 0.2 / 0 again, thin-walled
# at 0.2 and 0 (no refraction), IOR 1.2 and 2.0 at 0.2, refr_color 0.5 at
# 0.2 (the number of tinted events).
ARCH_GLASS2 = [('H1 refr 0.6', 0.6, {}), ('H2 refr 0.2', 0.2, {}), ('H3 refr 0', 0.0, {}),
               ('H4 refr 0.2 thin', 0.2, {'thin_walled': True}), ('H5 refr 0 thin', 0.0, {'thin_walled': True}),
               ('H6 refr 0.2 ior 1.2', 0.2, {'Index of Refraction': 1.2}), ('H7 refr 0.2 ior 2', 0.2, {'Index of Refraction': 2.0}),
               ('H8 refr 0.2 color 0.5', 0.2, {'refr_color': [0.5, 0.5, 0.5]})]
if os.path.exists(nv3):
    d = load(nv3)
    mats = d['scene']['materials']
    geo_label = {g['id']: n['label'] for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    glass = next(m for m in mats if geo_label.get(m['geometry'].lstrip('#'), '').startswith('A8'))
    for i, m in enumerate(mats):
        label = geo_label.get(m['geometry'].lstrip('#'))
        if not label or not label.startswith('A'):
            continue
        lab, refr, over = ARCH_GLASS2[int(label[1]) - 1]
        new = copy.deepcopy(glass)
        new.update(id=m['id'], geometry=m['geometry'], uv_set=m.get('uv_set', glass.get('uv_set')))
        ch = next(e for e in new['extra'] if e['type'] == 'studio_material_channels')['channels']
        ch[:] = [c for c in ch if c['channel']['id'] not in ('refr_gloss', 'reflectivity', 'brdf_fresnel', *over)]
        ch.append({'channel': {'id': 'refr_gloss', 'type': 'float', 'current_value': refr}})
        ch.append({'channel': {'id': 'reflectivity', 'type': 'float', 'current_value': 0.0}})
        for k, v in over.items():
            ch.append({'channel': {'id': k, 'type': TYPES.get(type(v), 'float'), 'current_value': v}})
        mats[i] = new
        for n in d['scene']['nodes']:
            if n.get('label') == label:
                n['label'] = lab
    furnace_dome(d)
    render_option(d, 'Firefly Filter Enable', False)
    made.append((save(d, 'Calib-Furnace-ArchGlass2'), None))
# Calib-Furnace-Resin3 (#116): Calib-Furnace-Resin2 at Max Path Length 3.
p2 = os.path.join(out_dir, 'Calib-Furnace-Resin2.duf')
if os.path.exists(p2) and not os.path.exists(os.path.join(out_dir, 'Calib-Furnace-Resin3.duf')):
    d = load(p2)
    render_option(d, 'Max Path Length', 3)
    made.append((save(d, 'Calib-Furnace-Resin3'), None))

# --- PBRSkin Specular Occlusion and Detail (#54): `pbr_material` scales the
# dual lobe's custom curve by occlusion - head-on lerp(1, Facing, w) ×
# specularlevel, grazing lerp(1, Grazing, w), w = Ambient Occlusion Weight
# (a map on skins) - and its roughness by lerp(1, Detail Specular
# Roughness Mult, Detail Weight), the Detail Normal Map blended onto the
# base normal at Detail Weight (tiled by Detail Horizontal / Vertical
# Tiles). A red dual-lobe base (reflectivity 0.5, roughness 0.4) with
# one change per sphere; O1 is the reference without either.
# (DL and RED are redefined above for other scenes: spelled out here.)
OCC = {'Dual Lobe Specular Enable': True, 'Dual Lobe Specular Weight': 1.0, 'Dual Lobe Specular Reflectivity': 0.5,
       'Specular Lobe 1 Roughness': 0.4, 'Specular Lobe 2 Roughness Mult': 0.4, 'Dual Lobe Specular Ratio': 0.15}
SO = {'Specular Occlusion Enable': True}
DT = {'Detail Enable': True}
RED = [0.45, 0.05, 0.04]
pbr_spheres('Calib-PBRSkin-Occlusion', [
    ('O1 dual lobe', RED, OCC),
    ('O2 occlusion default', RED, {**OCC, **SO, 'Specular Occusion Facing': 0.6, 'Specular Occlusion Grazing': 0.1, 'Ambient Occlusion Weight': 1.0}),
    ('O3 occlusion grazing', RED, {**OCC, **SO, 'Specular Occusion Facing': 1.0, 'Specular Occlusion Grazing': 0.1, 'Ambient Occlusion Weight': 1.0}),
    ('O4 occlusion facing', RED, {**OCC, **SO, 'Specular Occusion Facing': 0.25, 'Specular Occlusion Grazing': 1.0, 'Ambient Occlusion Weight': 1.0}),
    ('O5 occlusion w0.5', RED, {**OCC, **SO, 'Specular Occusion Facing': 0.6, 'Specular Occlusion Grazing': 0.1, 'Ambient Occlusion Weight': 0.5}),
    ('O6 occlusion grey map', RED, {**OCC, **SO, 'Specular Occusion Facing': 0.6, 'Specular Occlusion Grazing': 0.1, 'Ambient Occlusion Weight': 1.0,
                                    ('Ambient Occlusion Weight', 'image'): MAPS + 'calib-disp-grey.png'}),
    ('O7 detail rough 0.5', RED, {**OCC, **DT, 'Detail Weight': 1.0, 'Detail Specular Roughness Mult': 0.5}),
    ('O8 detail normal w0.5', RED, {**OCC, **DT, 'Detail Weight': 0.5, 'Detail Specular Roughness Mult': 0.25, 'Detail Normal Map': 1.0,
                                    ('Detail Normal Map', 'image'): BUMPS['dimples-n'], 'Detail Horizontal Tiles': 4.0, 'Detail Vertical Tiles': 4.0}),
])
if made[-1][0]:
    d = load(made[-1][0])
    for i in d['image_library']:
        if i['id'] in BUMPS.values():
            i['map_gamma'] = 1
    open(made[-1][0], 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))

# Scene dumps: scripts/dev/daz-dump.sh (DAZ evaluates each scene headless).
