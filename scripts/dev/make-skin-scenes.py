#!/usr/bin/env python3
"""ORBIS calibration: Calib-Skin-<figure>-<light>[-w0] (#19) — skin tones.

  python3 scripts/dev/make-skin-scenes.py [--force] [<figure>...]

Each figure is the same Genesis 8 Basic Female (shape and pose untouched)
wearing one character's skin preset, so only the skin differs between the
scenes. Beside her head: a sphere in her Torso material, its maps cropped
to one even patch of skin (a prop: does the skin rule need a figure?) and a matte 18 % grey sphere (the light and
exposure, independent of the skin). Built headless from Calib-L-Distant-45
by daz-build-skin-scene.dsa, then varied here:

- front: the distant light 30 degrees above the camera (colour, brightness)
- back:  the light behind her, 20 degrees up (light through ears and nose)
- hdri:  Calib-HDRI's environment and tone mapping, no distant light
- -w0:   every skin surface at Translucency Weight 0 (the surface without
         its subsurface part)

The scenes use DAZ Store skins: they stay in My Library (never in
calib/scenes), like Calib-Hair. Existing scenes are kept (--force rewrites).
"""
import copy, gzip, json, os, re, subprocess, sys
from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
SCENES = '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
LIB = '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library'
CHARS = LIB + '/People/Genesis 8 Female/Characters'
# Light to dark, as their maps read; each one full-body material preset.
FIGURES = {
    'Angel': CHARS + '/Angel/Iray/Angel Full Mat.duf',
    'Victoria': CHARS + '/Victoria 8/Materials/Iray/Victoria 8 All Maps.duf',
    'Kayo': CHARS + '/Kayo 8/Materials/Iray/Kayo 8 All Maps.duf',
    'Gabriela': CHARS + '/Gabriela 8/Materials/Iray/Gabriela 8 All Maps.duf',
    'Nishi': CHARS + '/Jessaii/Nishi/Materials Iray/Nishi 00 All Maps.duf',
    'Twosret': CHARS + '/Twosret 8/Materials/Iray/Twosret 8 All Maps.duf',
    'Monique': CHARS + '/Monique 8/Materials/Iray/Monique 8 All Maps.duf',
    'Zurie': CHARS + '/Zurie/Materials/Zurie All Maps.duf',
}
LIGHTS = {'front': (-30.0, 0.0), 'back': (-20.0, 180.0), 'hdri': None}
# sRGB 0.46 = linear 0.18 (DAZ decodes colour channels with gamma 2.2)
GREY = [0.46, 0.46, 0.46]
# Head and shoulders (57 cm high at 1.5 m), the spheres at ±26 cm beside
# the eyes; twice the base scene's focal length, yfov to match.
CAMERA = {'translation': (0.0, 157.0, 150.0), 'rotation': (0.0, 0.0, 0.0), 'focal_length': 130.0, 'yfov': 0.380}
IMAGE = [960, 720]
# The skin sphere's maps: below the breast beside the navel, as fractions of
# the torso map (left, top, right, bottom); no nipple, navel or seam.
PATCH = (0.28, 0.52, 0.40, 0.64)
MY_LIB = '/Volumes/Data/DAZ 3D/Studio/My Library'
CONTENT = [LIB, MY_LIB]
FORCE = '--force' in sys.argv
wanted = [a for a in sys.argv[1:] if not a.startswith('--')] or list(FIGURES)


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, p):
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(p)


def channels(m):
    """A material's channel list (created when it has none)."""
    for e in m.setdefault('extra', []):
        if e.get('type') == 'studio_material_channels':
            return e['channels']
    e = {'type': 'studio_material_channels', 'channels': []}
    m['extra'].append(e)
    return e['channels']


def set_channel(chs, cid, value, typ='float'):
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            c['channel'].pop('image_file', None)
            return
    chs.append({'channel': {'id': cid, 'type': typ, 'current_value': value}})


def merged(lib_m, scene_m):
    """The library material's channels with the scene's overrides on top."""
    out = {c['channel']['id']: copy.deepcopy(c) for c in channels(copy.deepcopy(lib_m))}
    for c in channels(copy.deepcopy(scene_m)):
        out.setdefault(c['channel']['id'], {'channel': {}})['channel'].update(c['channel'])
    return list(out.values())


def lib_node(d, prefix):
    return next(l for l in d['node_library'] if l['id'].startswith(prefix))


def set_xyz(node, key, xyz):
    for c in node[key]:
        c['current_value'] = xyz['xyz'.index(c['id'])]


def crop(figure, rel):
    """A patch of a torso map (the same UV region on every Genesis 8
    figure), saved under My Library; returns its content-relative path."""
    src = next((r + rel for r in CONTENT if os.path.exists(r + rel)), None)
    if not src:
        sys.exit(f'{figure}: map not found: {rel}')
    im = Image.open(src)
    w, h = im.size
    x0, y0, x1, y1 = PATCH
    # Without the UDIM tile in the name (…_1002): Iray would put the map on
    # UV tile 1002, off the sphere's 0–1 UVs, and leave the sphere white.
    stem = re.sub(r'[_ ]?10\d\d$', '', os.path.splitext(os.path.basename(rel))[0])
    out_rel = f'/Runtime/Textures/ORBIS/Calib-Skin/{figure}/{stem}-patch.png'
    os.makedirs(os.path.dirname(MY_LIB + out_rel), exist_ok=True)
    im.crop((round(x0 * w), round(y0 * h), round(x1 * w), round(y1 * h))).save(MY_LIB + out_rel)
    return out_rel


def build(name, preset, out):
    done = out + '.done'
    env = dict(os.environ, DONE_FILE=done)
    subprocess.run([os.path.join(here, 'daz-script.sh'), os.path.join(here, 'daz-build-skin-scene.dsa'),
                    SCENES + '/Calib-L-Distant-45.duf', LIB + '/People/Genesis 8 Female/Genesis 8 Basic Female.duf',
                    LIB + '/Shader Presets/Iray/DAZ Uber/!Iray Uber Base.duf', out, 'mat|' + preset],
                   env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    status = open(done).read() if os.path.exists(done) else 'no done file'
    if not status.startswith('saved 0'):
        sys.exit(f'{name}: build failed: {status}')


hdri = load(SCENES + '/Calib-HDRI.duf')
for name in wanted:
    base_p = f'{SCENES}/Calib-Skin-{name}.duf'
    if FORCE or not os.path.exists(base_p):
        build(name, FIGURES[name], base_p)
    base = load(base_p)
    sc = base['scene']
    mlib = {m['id']: m for m in base['material_library']}
    by_label = {n.get('label'): n for n in sc['nodes']}
    figure_geo = '#' + by_label['Genesis 8 Female']['geometries'][0]['id']
    skin = [m for m in sc['materials'] if m['geometry'] == figure_geo]
    torso = next(m for m in skin if m['id'].startswith('Torso'))

    def sphere_material(label):
        geo = '#' + by_label[label]['geometries'][0]['id']
        m = next(m for m in sc['materials'] if m['geometry'] == geo)
        return m, mlib[m['url'].lstrip('#')]

    # Skin sphere: the Torso material, merged into the sphere's own, its
    # maps cropped to one even patch of skin.
    m, lm = sphere_material('Skin Sphere')
    lm['extra'] = [e for e in lm.get('extra', []) if e.get('type') != 'studio_material_channels']
    chs = merged(mlib[torso['url'].lstrip('#')], torso)
    diffuse = copy.deepcopy(torso['diffuse'])
    for c in chs + [diffuse]:
        if c['channel'].get('image_file'):
            c['channel']['image_file'] = crop(name, c['channel']['image_file'])
    lm['extra'].append({'type': 'studio_material_channels', 'channels': chs})
    m['extra'] = [{'type': 'studio/material/uber_iray', 'version': '1.1.0.0'}]
    # Keep the library's channel definition ("mappable": without it DAZ
    # drops the map and the sphere renders white); set its value and map.
    lm['diffuse']['channel'].update({k: v for k, v in diffuse['channel'].items() if k in ('current_value', 'image_file')})
    m['diffuse'] = diffuse
    # Grey sphere: matte, no translucency.
    m, lm = sphere_material('Grey Sphere')
    lm['diffuse']['channel']['current_value'] = GREY
    m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': GREY}}
    for k, v in {'Glossy Layered Weight': 0.0, 'Translucency Weight': 0.0, 'Top Coat Weight': 0.0,
                 'Metallic Weight': 0.0, 'Base Mixing': 0}.items():
        set_channel(channels(lm), k, v, 'int' if k == 'Base Mixing' else 'float')
    cam = lib_node(base, 'Camera')
    set_xyz(cam, 'translation', CAMERA['translation'])
    set_xyz(cam, 'rotation', CAMERA['rotation'])
    cam['perspective'].update({'focal_distance': CAMERA['translation'][2], 'focal_length': CAMERA['focal_length'], 'yfov': CAMERA['yfov']})
    # DAZ (and the export) read the Focal Length channel before the perspective block.
    for e in cam.get('extra', []):
        for c in e.get('channels', []):
            if c['channel']['id'] == 'Focal Length':
                c['channel']['current_value'] = CAMERA['focal_length']
    ro = next(e for e in sc['extra'] if e['type'] == 'studio_render_settings')['render_options']
    ro.update({'imageSize': IMAGE, 'aspect_ratio': [4, 3]})

    for light, rot in LIGHTS.items():
        for w0 in (False, True):
            d = copy.deepcopy(base)
            if rot is None:
                # Calib-HDRI's dome and exposure; the sun goes.
                d['node_library'] = [l for l in d['node_library'] if not l['id'].startswith('DistantLight')]
                d['scene']['nodes'] = [n for n in d['scene']['nodes'] if not n['id'].startswith('DistantLight')]
                for lid in ('Environment Options', 'Tonemapper Options'):
                    i = next(i for i, l in enumerate(d['node_library']) if l['id'] == lid)
                    d['node_library'][i] = copy.deepcopy(next(l for l in hdri['node_library'] if l['id'] == lid))
            else:
                sun = lib_node(d, 'DistantLight')
                set_xyz(sun, 'rotation', (rot[0], rot[1], 0.0))
            if w0:
                dl = {m['id']: m for m in d['material_library']}
                ids = {m['id'] for m in skin}
                for m in d['scene']['materials']:
                    if m['id'] in ids:
                        set_channel(channels(m), 'Translucency Weight', 0.0)
                        set_channel(channels(dl[m['url'].lstrip('#')]), 'Translucency Weight', 0.0)
                sm = next(m for m in d['scene']['materials'] if m['geometry'] == '#' + by_label['Skin Sphere']['geometries'][0]['id'])
                set_channel(channels(dl[sm['url'].lstrip('#')]), 'Translucency Weight', 0.0)
            p = f'{SCENES}/Calib-Skin-{name}-{light}' + ('-w0' if w0 else '') + '.duf'
            if FORCE or not os.path.exists(p):
                save(d, p)
