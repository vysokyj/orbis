#!/usr/bin/env python3
"""ORBIS calibration: Calib-SkinG9-<skin>-<light>[-<variant>] (#57) -
Genesis 9 PBRSkin skins on free content.

  python3 scripts/dev/make-skin-g9-scenes.py [--force] [--rebuild] [<skin>...]

Calib-Skin (#19) is Genesis 8 with store skins, all Iray Uber. These are
its Genesis 9 twins for the direct PBRSkin mapping: the Genesis 9 figure
(shape and pose untouched) wearing one of its own base skins from Genesis 9
Starter Essentials (free with DAZ Studio), G9 Base Shirt on, so only the
skin differs between the scenes. Beside the head a matte 18 % grey sphere
(the light and exposure, independent of the skin). Built headless from
calib/scenes/Calib-L-Distant-45 by daz-build-skin-scene.dsa (its cubes and
big sphere hidden; the builder's second sphere removed here), then varied:

- front: the distant light 30 degrees above the camera
- back:  the light behind the head, 20 degrees up (light through the ears)
- hdri:  Calib-HDRI's environment and tone mapping, no distant light
- front-w0:     Translucency Enable off on every skin surface (the diffuse
                layer and the lobes alone: what is not the volume)
- front-gloss0: Dual Lobe Specular Enable off (the diffuse layer and the
                volume alone: what is not the lobes)
- front-nosss:  Sub Surface Enable off (the volume absorbs, never
                scatters: what the volume's scattering gives back)
- back-w0:      the backlit twin of front-w0 (what reaches the ears
                through the skin)
- hdri-w0, hdri-gloss0: front-w0's and front-gloss0's twins under the
                dome (#229)
- front-g0, hdri-g0: SSS Direction 0 on every skin surface (the volume
                scatters evenly, not back toward the light; #229)
- Calib-Mi-SkinG9[-nosss|-w0]: F02's front (and two variants) with every
                mesh but the figure hidden, the figure raised 1 cm, for the
                .mi fixtures only
- Calib-Mi-SkinG9-Floor: Calib-Mi-SkinG9 with the figure where it stands
                (soles under 0): the auto ground under a SubD figure (#228)

Skins, light to dark by their head maps' mean (0.79 / 0.53 / 0.29 red):
F02 = G9 Feminine Skin 02, F04 = G9 Feminine Skin 04, M03 = G9 Masculine
Skin 03. The scenes only reference the content. Written to "My
Library/Scenes" and calib/scenes alike (byte-identical, so the export finds
the cached dump of either, #159); an existing scene is kept unless --force.
The built bases stay in the cache (--rebuild builds them again).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, os, subprocess, sys, urllib.parse
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import duf_camera, duf_io

repo = os.path.dirname(os.path.dirname(here))
CALIB = os.path.join(repo, 'calib', 'scenes')
LIB = os.environ.get('ORBIS_CONTENT', '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library')
MYLIB = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')
SCENES = os.path.join(MYLIB, 'Scenes')
BUILD = os.path.expanduser('~/Library/Caches/orbis/skin-g9')
G9 = LIB + '/People/Genesis 9/Genesis 9.duf'
UBER = LIB + '/Shader Presets/Iray/DAZ Uber/!Iray Uber Base.duf'
SHIRT = LIB + '/People/Genesis 9/Clothing/Daz Originals/Base Clothing/G9 Base Shirt.duf'
MATS = LIB + '/People/Genesis 9/Materials/Daz Originals/Base Materials/G9 {} MAT.duf'
SKINS = {'F02': 'Feminine Skin 02', 'F04': 'Feminine Skin 04', 'M03': 'Masculine Skin 03'}
LIGHTS = {'front': (-30.0, 0.0), 'back': (-20.0, 180.0), 'hdri': None}
# variant: (light, {channel: value} on every skin surface)
VARIANTS = {
    'front-w0': ('front', {'Translucency Enable': False}),
    'front-gloss0': ('front', {'Dual Lobe Specular Enable': False}),
    'front-nosss': ('front', {'Sub Surface Enable': False}),
    'back-w0': ('back', {'Translucency Enable': False}),
    # #229: the dome's split (F02 under it 1.16 - 1.20 x Iray, the lamp
    # right): diffuse + lobes, diffuse + volume, and the volume's
    # backward bias off under both lights (does Iray's g -0.7 throw the
    # lamp's light back toward the camera beside it?).
    'hdri-w0': ('hdri', {'Translucency Enable': False}),
    'hdri-gloss0': ('hdri', {'Dual Lobe Specular Enable': False}),
    'front-g0': ('front', {'SSS Direction': 0.0}),
    'hdri-g0': ('hdri', {'SSS Direction': 0.0}),
}
# Calib-Mi-SkinG9<suffix>: the .mi fixture scenes (src/iray.rs, #57).
MI = {'': {}, '-nosss': {'Sub Surface Enable': False}, '-w0': {'Translucency Enable': False}}
SKIN = ('Head', 'Body', 'Arms', 'Legs')
# sRGB 0.46 = linear 0.18 (DAZ decodes colour channels with gamma 2.2)
GREY = [0.46, 0.46, 0.46]
# Head and shoulders (57 cm high at 1.5 m), the grey sphere 26 cm to the
# head's left at eye height; Calib-Skin's lens. Genesis 9's eyes sit higher
# than Genesis 8 Female's (157 cm): 160 centres them in the frame.
EYE_Y = 160.0
CAMERA = {'translation': (0.0, EYE_Y, 150.0), 'rotation': (0.0, 0.0, 0.0), 'focal_length': 130.0, 'yfov': 0.380}
IMAGE = [960, 720]


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def channels(m):
    for e in m.setdefault('extra', []):
        if e.get('type') == 'studio_material_channels':
            return e['channels']
    e = {'type': 'studio_material_channels', 'channels': []}
    m['extra'].append(e)
    return e['channels']


def set_channel(chs, cid, value, typ=None):
    typ = typ or ('bool' if isinstance(value, bool) else 'float')
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            c['channel'].pop('image_file', None)
            return
    chs.append({'channel': {'id': cid, 'type': typ, 'current_value': value}})


def set_xyz(node, key, xyz):
    for c in node[key]:
        c['current_value'] = xyz['xyz'.index(c['id'])]


def lib_node(d, prefix):
    return next(l for l in d['node_library'] if l['id'].startswith(prefix))


def build(skin, out):
    os.makedirs(BUILD, exist_ok=True)
    done = out + '.done'
    if os.path.exists(done):
        os.remove(done)
    # The skin preset first, then the shirt: the builder applies each
    # "mat|" preset to the figure, and a wearable preset loads the shirt
    # fitted to it.
    subprocess.run([os.path.join(here, 'daz-script.sh'), os.path.join(here, 'daz-build-skin-scene.dsa'),
                    os.path.join(CALIB, 'Calib-L-Distant-45.duf'), G9, UBER, out,
                    'mat|' + MATS.format(SKINS[skin]), 'mat|' + SHIRT, '--'],
                   env=dict(os.environ, DONE_FILE=done), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    status = open(done).read() if os.path.exists(done) else 'no done file'
    print(f'{skin}: {status.strip()}')
    if not status.startswith('saved 0') or ': false' in status:
        sys.exit(f'{skin}: build failed')


def skin_materials(d):
    """The figure's own skin surfaces (Head, Body, Arms, Legs) in the scene
    and their library entries."""
    fig = next(n for n in d['scene']['nodes'] if n.get('label') == 'Genesis 9')
    geo = '#' + fig['geometries'][0]['id']
    mlib = {m['id']: m for m in d.get('material_library', [])}
    out = [m for m in d['scene']['materials'] if m.get('geometry') == geo and m['id'] in SKIN]
    assert len(out) == len(SKIN), f'skin surfaces: {[m["id"] for m in out]}'
    return [(m, mlib.get(urllib.parse.unquote(m['url'].lstrip('#')))) for m in out]


def prepare(base):
    """Camera, image, the grey sphere; the builder's skin sphere removed."""
    sc = base['scene']
    by_label = {n.get('label'): n for n in sc['nodes']}
    skin_ball = by_label['Skin Sphere']
    geo = '#' + skin_ball['geometries'][0]['id']
    sc['nodes'] = [n for n in sc['nodes'] if n is not skin_ball]
    sc['materials'] = [m for m in sc['materials'] if m.get('geometry') != geo]
    grey = by_label['Grey Sphere']
    nlib = {l['id']: l for l in base['node_library']}
    for c in nlib[urllib.parse.unquote(grey['url'].lstrip('#'))]['translation']:
        if c['id'] == 'y':
            c['current_value'] = EYE_Y - 8.0  # the primitive's origin is its bottom
    mlib = {m['id']: m for m in base.get('material_library', [])}
    gm = next(m for m in sc['materials'] if m.get('geometry') == '#' + grey['geometries'][0]['id'])
    lm = mlib[urllib.parse.unquote(gm['url'].lstrip('#'))]
    lm['diffuse']['channel']['current_value'] = GREY
    gm['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': GREY}}
    for k, v in {'Glossy Layered Weight': 0.0, 'Translucency Weight': 0.0, 'Top Coat Weight': 0.0,
                 'Metallic Weight': 0.0, 'Base Mixing': 0}.items():
        set_channel(channels(lm), k, v, 'int' if k == 'Base Mixing' else 'float')
    cam = lib_node(base, 'Camera')
    set_xyz(cam, 'translation', CAMERA['translation'])
    set_xyz(cam, 'rotation', CAMERA['rotation'])
    cam['perspective'].update({'focal_distance': CAMERA['translation'][2], 'focal_length': CAMERA['focal_length'], 'yfov': CAMERA['yfov']})
    for e in cam.get('extra', []):
        for c in e.get('channels', []):
            if c['channel']['id'] == 'Focal Length':
                c['channel']['current_value'] = CAMERA['focal_length']
    ro = next(e for e in sc['extra'] if e['type'] == 'studio_render_settings')['render_options']
    ro.update({'imageSize': IMAGE, 'aspect_ratio': [4, 3]})


def lit(base, light, hdri):
    d = copy.deepcopy(base)
    rot = LIGHTS[light]
    if rot is None:
        d['node_library'] = [l for l in d['node_library'] if not l['id'].startswith('DistantLight')]
        d['scene']['nodes'] = [n for n in d['scene']['nodes'] if not n['id'].startswith('DistantLight')]
        for lid in ('Environment Options', 'Tonemapper Options'):
            i = next(i for i, l in enumerate(d['node_library']) if l['id'] == lid)
            d['node_library'][i] = copy.deepcopy(next(l for l in hdri['node_library'] if l['id'] == lid))
    else:
        set_xyz(lib_node(d, 'DistantLight'), 'rotation', (rot[0], rot[1], 0.0))
    return d


def write(d, name):
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    duf_camera.ensure(d)
    for dirpath in (SCENES, CALIB):
        dst = os.path.join(dirpath, name + '.duf')
        if os.path.exists(dst) and '--force' not in sys.argv:
            print(f'  kept {dst}')
            continue
        duf_io.save(dst, d)
        print(f'  wrote {dst}')


def hide_all_but_figure(d):
    """`Visible` off on every mesh but Genesis 9 itself (eyes, tear, lashes,
    brows, shirt, the grey sphere): DAZ's .mi leaves a hidden node out
    (#224), so the fixture holds the figure's PBRSkin surfaces alone (its
    skin and mouth) - the eyes' Layered Image Editor maps and the Uber
    wearables are other conversions'."""
    for n in d['scene']['nodes']:
        if not n.get('geometries') or n.get('label') in ('Genesis 9', 'Plane'):
            continue
        ext = next((e for e in n.setdefault('extra', []) if e.get('type') == 'studio_node_channels'), None)
        if ext is None:
            ext = {'type': 'studio_node_channels', 'version': 5}
            n['extra'].append(ext)
        chans = ext.setdefault('channels', [])
        c = next((c['channel'] for c in chans if c['channel']['id'] == 'Visible'), None)
        if c is None:
            c = {'id': 'Visible', 'type': 'bool'}
            chans.append({'channel': c})
        c['current_value'] = False


def lift_figure(d, cm):
    """Genesis 9 raised off the floor: its control cage's soles sit 0.95 mm
    below 0 (the refined feet in the .mi 0.65 mm), and DAZ puts the auto
    ground under the cage - a gap of the ground's, not the materials'
    (#228; Calib-Mi-SkinG9-Floor holds it). Raised, the ground stays at 0."""
    fig = next(n for n in d['scene']['nodes'] if n.get('label') == 'Genesis 9')
    fig['translation'] = [{'id': 'x', 'current_value': 0.0}, {'id': 'y', 'current_value': cm}, {'id': 'z', 'current_value': 0.0}]


def variant(base, hdri, light, change):
    d = lit(base, light, hdri)
    for m, lm in skin_materials(d):
        for k, v in change.items():
            for x in (m, lm):
                if x is not None:
                    set_channel(channels(x), k, v)
    return d


def main():
    wanted = [a for a in sys.argv[1:] if not a.startswith('--')] or list(SKINS)
    hdri = load(os.path.join(SCENES, 'Calib-HDRI.duf'))
    for skin in wanted:
        raw = os.path.join(BUILD, f'Calib-SkinG9-{skin}-base.duf')
        if '--rebuild' in sys.argv or not os.path.exists(raw):
            build(skin, raw)
        base = load(raw)
        prepare(base)
        for light in LIGHTS:
            write(lit(base, light, hdri), f'Calib-SkinG9-{skin}-{light}')
        for name, (light, change) in VARIANTS.items():
            write(variant(base, hdri, light, change), f'Calib-SkinG9-{skin}-{name}')
        # .mi fixtures (not rendered): F02's front, -nosss and -w0 with the
        # figure alone.
        if skin == 'F02':
            for name, change in MI.items():
                d = variant(base, hdri, 'front', change)
                hide_all_but_figure(d)
                lift_figure(d, 1.0)
                write(d, f'Calib-Mi-SkinG9{name}')
            # The figure where it stands: the auto ground under its cage (#228).
            d = variant(base, hdri, 'front', {})
            hide_all_but_figure(d)
            write(d, 'Calib-Mi-SkinG9-Floor')


if __name__ == '__main__':
    main()
