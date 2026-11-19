#!/usr/bin/env python3
"""ORBIS calibration: posed Genesis 9 figures in free wearables (#203) -
generic stand-ins for the deleted teens scenes (Calib-Disp-Teens,
Calib-Feet-Teens, teens itself as the big export).

  python3 scripts/dev/make-figure-scenes.py [--force] [--rebuild] [<scene>...]

Built headless from calib/scenes/Calib-L-Distant-45 by
daz-build-figure-scene.dsa (cubes, sphere and sun removed: lit by DAZ's
default dome, DTHDR-RuinsB, as the hair scenes are; the floor stays), with
Genesis 9 Starter Essentials content only (free with DAZ Studio): Genesis 9,
G9 Base Shirt, G9 Base Shorts, G9 Base dForce Pixie Hair and the G9 Base
Poses. Then here: the camera at 65 mm aimed at a box, exposure EV 15.7
(Calib-Hair's), the dome drawn, no ground, 1280 x 720.

- Calib-Fit-G9 (wardrobe fit, docs/notes/2026-10-29-wardrobe-fit.md): one
  figure in Base Pose 13 Walking, shirt, shorts and hair, framed from the
  waist to the floor at the front left: the shorts on the stepping thighs,
  the feet on the floor. The fit bug (#51) was wearables left in a stale
  fit; any conforming wearable on a posed figure shows it. No free shoes
  for Genesis 9 exist, so the feet are bare (the export's skin against the
  floor is still a check).
- Calib-Disp-G9 / -off / -Skin (displacement on a figure,
  docs/notes/2026-10-29-displacement.md, Calib-Disp-Teens): the same
  figure, the camera close on the shorts (about 0.6 mm per pixel, Disp-Teens
  had 1 mm). A synthetic relief map (ours, calib-disp-ridges.png: 16
  horizontal sine ridges over the UV square, 8-bit, gamma automatic)
  as Displacement Strength on the shirt and the shorts, 0 - 0.5 cm (the
  5 mm teens' sweatshirt carried), SubD Displacement Level 2. -off: the
  same at strength 0 (what is not displacement). -Skin: the map on the
  figure's own PBRSkin surfaces instead (Body, Legs, Arms, Head) - skin
  displacement, unchecked so far.
- Calib-Big-G9 (export memory and timing, docs/notes/2026-11-05-*.md):
  eight figures in a 4 x 2 grid, each in its own base pose with shirt,
  shorts and hair, at DAZ's SubD render level (2). Too big for the Iray
  Server's 4 GB and for an Octane series (#50): an export measurement only,
  never queued.

The scenes are written to "My Library/Scenes" and calib/scenes; the map to
"My Library/Runtime/Textures/ORBIS". The built bases are kept in the cache
(--rebuild builds them again); an existing scene is kept unless --force.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, math, os, struct, subprocess, sys, zlib
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import duf_camera, duf_io

repo = os.path.dirname(os.path.dirname(here))
CALIB = os.path.join(repo, 'calib', 'scenes')
LIB = os.environ.get('ORBIS_CONTENT', '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library')
MYLIB = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')
SCENES = os.path.join(MYLIB, 'Scenes')
BUILD = os.path.expanduser('~/Library/Caches/orbis/figures')
G9 = 'People/Genesis 9/Genesis 9.duf'
POSES = 'People/Genesis 9/Poses/Daz Originals/Base Poses/Base/G9 Base Pose {} G9B.duf'
SHIRT = 'People/Genesis 9/Clothing/Daz Originals/Base Clothing/G9 Base Shirt.duf'
SHORTS = 'People/Genesis 9/Clothing/Daz Originals/Base Clothing/G9 Base Shorts.duf'
PIXIE = 'People/Genesis 9/Hair/Daz Originals/Base Hair/G9 Base dForce Pixie Hair.duf'
WEAR = (SHIRT, SHORTS, PIXIE)
# base name: [(x, z, yaw, pose)]
BASES = {
    'one': [(0, 0, 0, '13 Walking')],
    'big': [(x, z, 0, pose) for (x, z), pose in zip(
        [(x, z) for z in (0, -120) for x in (-150, -50, 50, 150)],
        ['12 Standing', '13 Walking', '15 Standing', '16 Walking', '17 Standing', '19 Standing', '01 Running', '02 Stretching'])],
}
FOCAL = 65.0
SIZE = (1280, 720)
EV = 15.7  # Calib-Hair's: a figure under the bare RuinsB dome, f/8, ISO 100
RIDGES = '/Runtime/Textures/ORBIS/calib-disp-ridges.png'
DISP = {'Displacement Strength': 1.0, 'Minimum Displacement': 0.0, 'Maximum Displacement': 0.5, 'SubD Displacement Level': 2}
CLOTHES = ('#G9TShirtBasics', '#EssentialShorts')
SKIN = ('Body', 'Legs', 'Arms', 'Head')
# scene: (base, camera target box lo / hi in cm, yaw, pitch, the change)
SCENES_SPEC = {
    'Calib-Fit-G9': ('one', (-30, 0, -40), (30, 110, 40), 30.0, 10.0, None),
    'Calib-Disp-G9': ('one', (-20, 70, -15), (20, 110, 15), 20.0, 5.0, 'clothes'),
    'Calib-Disp-G9-off': ('one', (-20, 70, -15), (20, 110, 15), 20.0, 5.0, 'clothes-off'),
    'Calib-Disp-G9-Skin': ('one', (-20, 70, -15), (20, 110, 15), 20.0, 5.0, 'skin'),
    'Calib-Big-G9': ('big', (-200, 0, -160), (200, 180, 40), 0.0, 10.0, None),
}


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, path):
    duf_io.save(path, d)


def write_ridges():
    """16 sine ridges along v, 8-bit grey, no gamma of its own (DAZ decodes
    it at 2.2, as it does every automatic map)."""
    path = os.path.join(MYLIB, RIDGES.lstrip('/'))
    if os.path.exists(path):
        return path
    os.makedirs(os.path.dirname(path), exist_ok=True)
    n = 1024
    rows = b''.join(b'\x00' + bytes([round(255 * (0.5 + 0.5 * math.sin(2 * math.pi * 16 * (y + 0.5) / n)))] * n) for y in range(n))
    chunk = lambda t, b: struct.pack('>I', len(b)) + t + b + struct.pack('>I', zlib.crc32(t + b) & 0xffffffff)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', n, n, 8, 0, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
    open(path, 'wb').write(png)
    print(f'wrote {path}')
    return path


def build(name):
    os.makedirs(BUILD, exist_ok=True)
    out = os.path.join(BUILD, f'{name}-base.duf')
    done = out + '.done'
    if os.path.exists(done):
        os.remove(done)
    items = []
    for x, z, yaw, pose in BASES[name]:
        items.append(f'figure:{x},{z},{yaw}:' + os.path.join(LIB, G9))
        items.append('pose:' + os.path.join(LIB, POSES.format(pose)))
        items += ['wear:' + os.path.join(LIB, w) for w in WEAR]
    subprocess.run([os.path.join(here, 'daz-script.sh'), os.path.join(here, 'daz-build-figure-scene.dsa'),
                    os.path.join(CALIB, 'Calib-L-Distant-45.duf'), out, *items, '--'],
                   env=dict(os.environ, DONE_FILE=done), check=False)
    log = open(done).read() if os.path.exists(done) else 'no .done'
    print(f'{name}: {log.strip()}')
    if not log.startswith('saved 0') or ' false ' in log:
        sys.exit(f'{name}: build failed')
    return out


def lib_node(d, kind=None, label=None):
    return next(n for n in d['node_library'] if (kind and n.get('type') == kind) or (label and n.get('label') == label))


def node_channel(n, cid, value):
    for e in n.get('extra', []):
        for c in e.get('channels', []):
            if c['channel']['id'] == cid:
                c['channel']['current_value'] = value
                return
    raise KeyError(cid)


def transform(n, pos, rot):
    for key, values in (('translation', pos), ('rotation', rot)):
        for c, v in zip(n[key], values):
            c['current_value'] = v


def aim(d, lo, hi, yaw, pitch):
    """The camera at FOCAL, YAW to the left and PITCH above the box's
    centre, at the distance where the box fills the frame (DAZ fits the
    camera's Frame Width to the image's height)."""
    c = [(a + b) / 2 for a, b in zip(lo, hi)]
    cam = lib_node(d, kind='camera')
    film = next(c['channel'] for e in cam.get('extra', []) for c in e.get('channels', []) if c['channel']['id'] == 'Frame Width')
    vfov = 2 * math.atan(film.get('current_value', film.get('value')) / 2 / FOCAL)
    height = max(hi[1] - lo[1], (hi[0] - lo[0]) * SIZE[1] / SIZE[0])  # the box's width, too
    dist = height / (2 * math.tan(vfov / 2)) + (hi[2] - lo[2]) / 2
    y, p = math.radians(yaw), math.radians(pitch)
    pos = (c[0] + dist * math.cos(p) * math.sin(y), c[1] + dist * math.sin(p), c[2] + dist * math.cos(p) * math.cos(y))
    transform(cam, pos, (-pitch, yaw, 0))
    node_channel(cam, 'Focal Length', FOCAL)
    node_channel(cam, 'DOF', False)
    return pos


def render_setup(d, subd):
    tm = lib_node(d, label='Tonemapper Options')
    node_channel(tm, 'Exposure Value', EV)
    node_channel(tm, 'Shutter Speed', 2 ** EV / 64)  # f/8, as DAZ links them
    env = lib_node(d, label='Environment Options')
    node_channel(env, 'Environment Mode', 0)  # Dome and Scene (the base: Scene Only)
    node_channel(env, 'Draw Dome', True)
    node_channel(env, 'Draw Ground', False)
    ro = next(x for x in d['scene']['extra'] if x.get('type') == 'studio_render_settings')['render_options']
    ro['imageSize'] = list(SIZE)
    ro['aspect_ratio'] = [16, 9]
    if subd is None:
        return
    # SubD level 1 at render on the one-figure scenes (both renderers read
    # the same level; Genesis 9 at DAZ's 2 is 1.6 M faces, Calib-Shell-G9).
    for n in d['scene']['nodes']:
        for g in n.get('geometries', []):
            for e in g.get('extra', []):
                for c in e.get('channels', []):
                    if c['channel']['id'] == 'SubDRenderLevel':
                        c['channel']['current_value'] = subd


def set_channels(m, values):
    ext = next((e for e in m.setdefault('extra', []) if e.get('type') == 'studio_material_channels'), None)
    if ext is None:
        ext = {'type': 'studio_material_channels', 'channels': []}
        m['extra'].append(ext)
    for cid, v in values.items():
        image = None
        if isinstance(v, tuple):
            v, image = v
        c = next((c['channel'] for c in ext['channels'] if c['channel']['id'] == cid), None)
        if c is None:
            c = {'id': cid, 'type': 'int' if cid.endswith('Level') else 'float'}
            ext['channels'].append({'channel': c})
        c['current_value'] = v
        if image:
            c['image_file'] = image
        else:
            c.pop('image_file', None)


def displace(d, which):
    if which.startswith('clothes'):
        mats = [m for m in d['scene']['materials'] if m.get('geometry') in CLOTHES]
    else:
        mats = [m for m in d['scene']['materials'] if m.get('geometry') == '#Genesis9-1'
                and m['url'].split('#')[1] in SKIN]
    assert mats, f'no materials for {which}'
    strength = 0.0 if which.endswith('-off') else DISP['Displacement Strength']
    for m in mats:
        set_channels(m, {**DISP, 'Displacement Strength': (strength, RIDGES)})
    return len(mats)


def main():
    force, rebuild = '--force' in sys.argv, '--rebuild' in sys.argv
    names = [a for a in sys.argv[1:] if not a.startswith('--')] or list(SCENES_SPEC)
    write_ridges()
    bases = {}
    for scene in names:
        base, lo, hi, yaw, pitch, change = SCENES_SPEC[scene]
        if base not in bases:
            path = os.path.join(BUILD, f'{base}-base.duf')
            if rebuild or not os.path.exists(path):
                path = build(base)
            bases[base] = load(path)
        d = copy.deepcopy(bases[base])
        pos = aim(d, lo, hi, yaw, pitch)
        render_setup(d, None if base == 'big' else 1)
        note = f', {displace(d, change)} materials displaced ({change})' if change else ''
        print(f'{scene}: camera {[round(v) for v in pos]}{note}')
        d['asset_info']['id'] = f'/Scenes/{scene}.duf'
        duf_camera.ensure(d)
        for dirpath in (SCENES, CALIB):
            dst = os.path.join(dirpath, scene + '.duf')
            if os.path.exists(dst) and not force:
                print(f'  kept {dst}')
                continue
            save(d, dst)
            print(f'  wrote {dst}')


if __name__ == '__main__':
    main()
