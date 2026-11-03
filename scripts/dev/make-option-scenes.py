#!/usr/bin/env python3
"""ORBIS calibration: scenes for Iray options not covered yet, one option at a time.

  python3 scripts/dev/make-option-scenes.py [--force] [<My Library Scenes dir>]

Variants of existing generated scenes (DAZ built-ins and primitives only), no
DAZ Studio needed. Written into My Library/Scenes (ORBIS_LIBRARY or the
argument) and copied into calib/scenes; existing scenes are kept (--force).
- Calib-Tone-*  tonemapper: vignetting, per-component burn, saturation, white point (#38)
- Calib-Env-*   environment: tint, dome rotation / orientation, lighting blur / resolution,
                intensity, ground, matte fog, ground fog, dome modes (#37, #65)
- Calib-RS-*    render settings: caustics, path length, firefly, bloom, pixel filter, spectral (#43)
- Calib-L-Opt-* light options on Calib-L-Point-5k: decay, shadow intensity / softness / colour,
                illumination, render emitter, cast shadows (#35); Calib-L-Opt-Rect-* on
                Calib-L-Rect-50: render emitter (light facing the camera), light portal (with a dome);
                Calib-L-Opt-Spot-* on Calib-L-Mi-Spot-rect: render emitter of a rect spot facing the camera
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
        ('WhiteWarm', {'White Point': [1.0, 0.8, 0.6]}), ('WhiteScale2', {'White Point Scale': 2.0})]
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
      ('Spectral', {'Spectral Rendering Enable': True})]
variants('Calib-Glass', 'Calib-RS', None, RS, render=True)

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
