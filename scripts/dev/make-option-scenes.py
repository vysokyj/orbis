#!/usr/bin/env python3
"""ORBIS calibration: scenes for Iray options not covered yet, one option at a time.

  python3 scripts/dev/make-option-scenes.py [--force] [<My Library Scenes dir>]

Variants of existing generated scenes (DAZ built-ins and primitives only), no
DAZ Studio needed. Written into My Library/Scenes (ORBIS_LIBRARY or the
argument) and copied into calib/scenes; existing scenes are kept (--force).
- Calib-Tone-*  tonemapper: vignetting, per-component burn, saturation, white point (#38, #70)
- Calib-Env-*   environment: tint, dome rotation / orientation, lighting blur / resolution,
                intensity, ground, matte fog, ground fog, dome modes (#37, #65); Calib-Env-Mi-*
                for DAZ's .mi export only (every fog / dome channel off its default, #37)
- Calib-RS-*    render settings: caustics, path length, firefly, bloom, pixel filter, spectral (#43);
                Calib-RS-Mi-* for DAZ's .mi export only (triangle filter, SSS path length, bloom settings)
- Calib-L-Opt-* light options on Calib-L-Point-5k: decay, shadow intensity / softness / colour,
                illumination, render emitter, cast shadows (#35); Calib-L-Opt-Rect-* on
                Calib-L-Rect-50: render emitter (light facing the camera), light portal (with a dome);
                Calib-L-Opt-Spot-* on Calib-L-Mi-Spot-rect: render emitter of a rect spot facing the camera
- Calib-L-IES-* IES light profiles (#34): DAZ's own test profile (light_profile_test.ies in the
                app bundle) and calib/ies/Orbis-Asym.ies (synthetic, copied to My Library/Runtime/ORBIS)
                on a point light level or turned down, a rect (DAZ's spot has no Light Profile), and an emissive plane's Emission
                Profile; the bare floor seen from straight above (the emissive plane: the usual camera)
- Calib-Lens-*  camera lens options (#39): lens shift, stereo offset, distortion types on
                Calib-Tone-Default; aperture blades, radial bias, lens thickness on Calib-DOF-f4
"""
import copy, gzip, json, os, shutil, sys

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
calib = os.path.join(repo, 'calib', 'scenes')
args = [a for a in sys.argv[1:] if not a.startswith('--')]
lib = args[0] if args else os.path.join(os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library'), 'Scenes')
FORCE = '--force' in sys.argv


def load(name):
    raw = open(os.path.join(calib, name + '.duf'), 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, name):
    dst = os.path.join(calib, name + '.duf')
    if os.path.exists(dst) and not FORCE:
        return
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    open(dst, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    if os.path.isdir(lib):
        shutil.copyfile(dst, os.path.join(lib, name + '.duf'))
    print(name)


def node_channels(d, lib_id):
    n = next(l for l in d['node_library'] if l['id'] == lib_id)
    return next(e for e in n['extra'] if e['type'] == 'studio_node_channels')['channels']


def set_channel(channels, cid, value):
    for c in channels:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    raise KeyError(cid)


def set_render(d, cid, value):
    hit = []
    def walk(v):
        if isinstance(v, dict):
            if v.get('id') == cid and 'current_value' in v:
                v['current_value'] = value
                hit.append(1)
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
    for e in d['scene'].get('extra', []):
        if e.get('type') == 'studio_render_settings': walk(e)
    if not hit:
        raise KeyError(cid)


def variants(base, prefix, node, table, render=False):
    for name, over in table:
        d = copy.deepcopy(load(base))
        for k, v in over.items():
            if render: set_render(d, k, v)
            else: set_channel(node_channels(d, node), k, v)
        save(d, f'{prefix}-{name}')


TONE = [('Vignette05', {'Vignetting': 0.5}), ('Vignette1', {'Vignetting': 1.0}),
        ('BurnComp-off', {'Burn Highlights Per Component': False, 'Burn Highlights': 0.5}),
        ('BurnComp-on', {'Burn Highlights Per Component': True, 'Burn Highlights': 0.5}),
        ('Sat0', {'Saturation': 0.0}), ('Sat15', {'Saturation': 1.5}),
        ('WhiteWarm', {'White Point': [1.0, 0.8, 0.6]}), ('WhiteScale2', {'White Point Scale': 2.0}),
        # Other hues settle the white point's normalisation k (#70): the mean
        # of wp^2.2 and its Rec. 709 luminance differ by 11 % / 22 % here.
        ('WhiteCool', {'White Point': [0.6, 0.8, 1.0]}), ('WhiteGreen', {'White Point': [0.9, 1.0, 0.7]})]
variants('Calib-Tone-Default', 'Calib-Tone', 'Tonemapper Options', TONE)

ENV = [('Tint', {'Environment Tint': [1.0, 0.5, 0.3]}), ('Rot90', {'Dome Rotation': 90.0}),
       ('Rot180', {'Dome Rotation': 180.0}), ('OrientX30', {'Dome Orientation X': 30.0}),
       ('Blur', {'Environment Lighting Blur': True}), ('Res128', {'Environment Lighting Resolution': 128}),
       ('Intensity2', {'Environment Intensity': 2.0}),
       ('Ground', {'Draw Ground': True, 'Ground Reflectivity': [0.5, 0.5, 0.5], 'Ground Glossiness': 0.6}),
       ('GroundMatte', {'Draw Ground': True, 'Ground Reflectivity': [0.0, 0.0, 0.0], 'Ground Shadow Intensity': 1.0}),
       ('MatteFog', {'Matte Fog': True, 'Matte Fog Visibility': 100.0}),
       ('GroundFog', {'Ground Fog': True}),
       ('DomeMode1', {'Dome Mode': 1}), ('DomeMode2', {'Dome Mode': 2})]
variants('Calib-Tone-Default', 'Calib-Env', 'Environment Options', ENV)

RS = [('Caustics', {'Caustic Sampler': True}), ('PathLen4', {'Max Path Length': 4}),
      ('PathLen2', {'Max Path Length': 2}), ('FireflyOff', {'Firefly Filter Enable': False}),
      ('Bloom', {'Bloom Filter Enable': True, 'Bloom Filter Threshold': 1.0}),
      ('PixelBox', {'Pixel Filter': 0}), ('PixelRadius3', {'Pixel Filter Radius': 3.0}),
      ('Spectral', {'Spectral Rendering Enable': True}),
      # Bloom at DAZ's default threshold and above: what Iray does with bright spots only (#43).
      ('Bloom2000', {'Bloom Filter Enable': True}),
      ('Bloom10000', {'Bloom Filter Enable': True, 'Bloom Filter Threshold': 10000.0})]
variants('Calib-Glass', 'Calib-RS', None, RS, render=True)
# For DAZ's .mi export only: every render setting off its default (#43).
RS_MI = [('Triangle', {'Pixel Filter': 1, 'Pixel Filter Radius': 1.0}),
         ('Lengths', {'Max Path Length': 7, 'Max SSS Path Length': 12}),
         ('Bloom', {'Bloom Filter Enable': True, 'Bloom Filter Radius': 0.02, 'Bloom Filter Threshold': 3.0,
                    'Bloom Filter Brightness Scale': 2.0})]
variants('Calib-Glass', 'Calib-RS-Mi', None, RS_MI, render=True)

LIGHT = [('Decay0', {'Decay': 0}), ('Decay1', {'Decay': 1}), ('ShadowInt05', {'Shadow Intensity': 0.5}),
         ('ShadowInt0', {'Shadow Intensity': 0.0}), ('ShadowSoft05', {'Shadow Softness': 0.5}),
         ('ShadowColor', {'Shadow Color': [0.3, 0.0, 0.0]}), ('IllumDiffuse', {'Illumination': 1}),
         ('IllumSpecular', {'Illumination': 2}), ('EmitterOff', {'Render Emitter': False}),
         ('CastShadowsOff', {'Cast Shadows': False}), ('Temp3000', {'Temperature': 3000.0})]
variants('Calib-L-Point-5k', 'Calib-L-Opt', 'PointLight', LIGHT)

# An area light (#35): Facing turns it towards the camera (in the frame, its emitting face seen);
# a portal only passes the dome.
FACING = {'PointLight': {'translation': {'y': 100.0}, 'rotation': {'x': -150.0}}}
DOME = {'Environment Options': {'Environment Mode': 0}}
RECT = [('Facing', FACING), ('Facing-EmitterOff', {'PointLight': {**FACING['PointLight'], 'Render Emitter': False}}),
        ('Dome', DOME), ('Portal', {**DOME, 'PointLight': {'Light Portal': True}})]
for name, over in RECT:
    d = copy.deepcopy(load('Calib-L-Rect-50'))
    for node, chans in over.items():
        n = next(l for l in d['node_library'] if l['id'] == node)
        for k, v in chans.items():
            if isinstance(v, dict):  # translation / rotation axes
                for c in n[k]:
                    c['current_value'] = v.get(c['id'], c['current_value'])
            else:
                set_channel(node_channels(d, node), k, v)
    save(d, f'Calib-L-Opt-Rect-{name}')

# A rect spot (an Octane analytic quad) turned up to the camera above it: does the camera see it (#35)?
SPOT_UP = {'rotation': {'x': 90.0}}
for name, over in [('Facing', SPOT_UP), ('Facing-EmitterOff', {**SPOT_UP, 'Render Emitter': False})]:
    d = copy.deepcopy(load('Calib-L-Mi-Spot-rect'))
    n = next(l for l in d['node_library'] if l['id'] == 'SpotLight')
    for k, v in over.items():
        if isinstance(v, dict):
            for c in n[k]:
                c['current_value'] = v.get(c['id'], c['current_value'])
        else:
            set_channel(node_channels(d, 'SpotLight'), k, v)
    save(d, f'Calib-L-Opt-Spot-{name}')

# .mi-only scenes (#37): every ground fog, matte fog and finite dome channel off its default, so
# DAZ's .mi export shows how each reaches Iray (scales, heights, units). No Iray render needed.
ENV_MI = [('Fog', {'Ground Fog': True, 'Ground Fog Decay Start': 50.0, 'Ground Fog Decay Height': 120.0,
                   'Ground Fog Density Start': [0.5, 0.6, 0.7], 'Ground Fog Density Start Scale': 2.0,
                   'Ground Fog Density End': [0.1, 0.2, 0.3], 'Ground Fog Density End Scale': 0.5,
                   'Ground Fog Albedo': [0.9, 0.8, 0.7], 'Ground Fog Scattering Type': 1,
                   'Ground Fog Particle Size': 8.0, 'Ground Fog Anisotropy': 0.3,
                   'Ground Position Mode': 1, 'Ground Origin Y': 30.0}),
          ('MatteFog', {'Matte Fog': True, 'Matte Fog Visibility': 50.0, 'Matte Fog Visibility Tint': [1.0, 0.5, 0.25],
                        'Matte Fog Brightness': 0.5, 'Matte Fog Brightness Relative to Environment': False,
                        'Matte Fog Brightness Tint': [0.8, 0.6, 0.4]}),
          ('Box', {'Dome Mode': 5, 'Dome Width': 300.0, 'Dome Height': 250.0, 'Dome Depth': 400.0,
                   'Dome Scale Multiplier': 50.0, 'Dome Origin X': 10.0, 'Dome Origin Y': 20.0, 'Dome Origin Z': 30.0,
                   'Ground Texture Scale': 150.0, 'Ground Visible from Below': False, 'Visualize Finite Dome': True}),
          ('SphereGround', {'Dome Mode': 3, 'Dome Radius': 70.0, 'Dome Scale Multiplier': 30.0}),
          ('Box-NoGround', {'Dome Mode': 4}),
          ('FogHG', {'Ground Fog': True, 'Ground Fog Anisotropy': 0.3}),
          ('GroundBelow', {'Draw Ground': True, 'Ground Visible from Below': False})]
variants('Calib-Tone-Default', 'Calib-Env-Mi', 'Environment Options', ENV_MI)

# Camera lens options (#39), one group per scene. Lens Distortion Type is an enum: 0 none,
# 1 spherical, 2 cylindrical, 3 poly3, 4 inv_poly3, 5 poly5, 6 inv_poly5, 7 ptlens, 8 inv_ptlens,
# 9 brown, 10 inv_brown, 11 equidistant. The camera sees ~69 deg horizontally (65 mm, film 88.9 mm).
LENS = [('Shift', {'Lens Shift X': 5.0, 'Lens Shift Y': -3.0}),
        ('Stereo', {'Lens Stereo Offset': 100.0}),
        ('Spherical', {'Lens Distortion Type': 1}),
        ('Cylindrical', {'Lens Distortion Type': 2}),
        ('Poly3', {'Lens Distortion Type': 3, 'Lens Distortion K1': 0.3}),
        ('InvPoly5', {'Lens Distortion Type': 6, 'Lens Distortion K1': 0.2, 'Lens Distortion K2': 0.1}),
        ('Brown', {'Lens Distortion Type': 9, 'Lens Distortion K1': 1.0, 'Lens Distortion K2': 0.05, 'Lens Distortion K3': -0.3,
                   'Lens Distortion K4': 0.1, 'Lens Distortion K5': 0.2}),
        # An ideal equidistant fisheye, 120 deg across (k2 = theta / scale, Iray manual 13.3).
        ('Equidistant', {'Lens Distortion Type': 11, 'Lens Distortion K2': 2.0943951}),
        # For DAZ's .mi export only: every remaining distortion channel off its default.
        ('Mi-PTLens', {'Lens Distortion Type': 8, 'Lens Distortion K1': 0.1, 'Lens Distortion K2': -0.1, 'Lens Distortion K3': 0.2,
                       'Lens Distortion Scale': 0.8, 'Lens Maximum FOV': 3.0})]
variants('Calib-Tone-Default', 'Calib-Lens', 'Camera', LENS)
LENS_DOF = [('Blades5', {'Aperture Blades': 5, 'Aperture Blade Rotation': 20.0}),
            ('Bias02', {'Lens Radial Bias': 0.2}),
            ('Bias08', {'Lens Radial Bias': 0.8}),
            ('Thick', {'Lens Thickness': 50.0})]
variants('Calib-DOF-f4', 'Calib-Lens', 'Camera', LENS_DOF)

# IES light profiles (#34). DAZ's test profile is dark within 35 deg of its axis and lit beyond
# (axially symmetric); Orbis-Asym a downlight to 60 deg, 1 / 0.6 / 0.1 / 0.3 at horizontal
# angles 0 / 90 / 180 / 270: which way each profile points in Iray shows on the bare floor.
DAZ_IES = '/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/shaders/iray/resources/light_profile_test.ies'
ASYM_IES = '/Runtime/ORBIS/Orbis-Asym.ies'
if os.path.isdir(lib):
    rt = os.path.join(os.path.dirname(lib), 'Runtime', 'ORBIS')
    os.makedirs(rt, exist_ok=True)
    shutil.copyfile(os.path.join(repo, 'calib', 'ies', 'Orbis-Asym.ies'), os.path.join(rt, 'Orbis-Asym.ies'))


def set_xyz(d, node, key, values):
    n = next(l for l in d['node_library'] if l['id'] == node)
    for c in n[key]:
        c['current_value'] = values.get(c['id'], c['current_value'])


def bare_floor_from_above(d):
    """Hide the cubes and the sphere; the camera 5.2 m above the floor looking down."""
    for n in d['scene']['nodes']:
        if n.get('url', '').startswith(('#pCube', '#pSphere')):
            ext = next((e for e in n.setdefault('extra', []) if e.get('type') == 'studio_node_channels'), None)
            if ext is None:
                ext = {'type': 'studio_node_channels', 'version': 5, 'channels': []}
                n['extra'].append(ext)
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
    set_xyz(d, 'Camera', 'translation', {'x': 0.0, 'y': 520.0, 'z': 0.0})
    set_xyz(d, 'Camera', 'rotation', {'x': -90.0, 'y': 0.0, 'z': 0.0})


DOWN = {'x': -90.0, 'y': 0.0, 'z': 0.0}
IES = [('Point', 'Calib-L-Point-5k', 'PointLight', DAZ_IES, {'x': 0.0, 'y': 0.0, 'z': 0.0}),
       ('Point-Down', 'Calib-L-Point-5k', 'PointLight', DAZ_IES, DOWN),
       ('Asym-Down', 'Calib-L-Point-5k', 'PointLight', ASYM_IES, DOWN),
       ('Rect', 'Calib-L-Rect-50', 'PointLight', DAZ_IES, DOWN)]
for name, base, node, ies, rot in IES:
    d = copy.deepcopy(load(base))
    set_channel(node_channels(d, node), 'Light Profile', ies)
    set_xyz(d, node, 'translation', {'x': 0.0, 'y': 250.0, 'z': 0.0})
    set_xyz(d, node, 'rotation', rot)
    bare_floor_from_above(d)
    save(d, f'Calib-L-IES-{name}')
# The emissive plane (2.5 m up, facing down) with an Emission Profile, from the usual camera.
for name, ies in (('Emit', DAZ_IES), ('Emit-Asym', ASYM_IES)):
    d = copy.deepcopy(load('Calib-L-Emit-cd'))
    m = next(m for m in d['material_library'] if m['id'] == 'Default-5')
    set_channel(next(e for e in m['extra'] if e['type'] == 'studio_material_channels')['channels'], 'Emission Profile', ies)
    save(d, f'Calib-L-IES-{name}')
