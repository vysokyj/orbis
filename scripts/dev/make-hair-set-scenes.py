#!/usr/bin/env python3
"""ORBIS calibration: Calib-Hair-<name>[-nogloss] (#161) - more strand hairs
than Morgan Hair, framed and lit like its scenes, for the Look split.

  python3 scripts/dev/make-hair-set-scenes.py [--force] [--rebuild] [<name>...]

#161 (2026-11-17): Faithful follows Iray's ribbons, Enhanced keeps Octane's
round curves, to be tuned on more hairs than Morgan alone. Each scene is
one figure wearing one strand hair (polylines, `BasisCurves` in the
export), built headless from calib/scenes/Calib-L-Distant-45 by
daz-build-hair-scene.dsa (cubes, sphere and sun removed: lit by DAZ's
default dome, DTHDR-RuinsB, as the Morgan scenes are), then here:

- the camera at 65 mm, 1280 x 720 (as Calib-Hair-Teens), from the front
  left and above, aimed at the hair's bounding box (read from its .dsf),
  the hair about half the frame's height;
- exposure EV 15.7 (Calib-Hair's: a figure under the bare dome), the dome
  drawn, no ground (the scene's floor plane stays, out of frame);
- the figure at SubD render level 1 (both renderers read the same level);
- -nogloss: Glossy Layered Weight, Glossy Weight and Dual Lobe Specular
  Weight 0 on the hair's materials (Calib-Hair-Teens-nogloss: the glossy layer, Morgan's
  only gloss): the base under the gloss.
- Pixie only (#203, the deleted Calib-Hair-Teens-* on free content):
  -T2 / -T1 the hair's Line Tessellation sides 2 (ribbons) / 1 (fibres),
  each also -nogloss and -coat (a white top coat, COAT); -T1-nofig(-nogloss)
  the strands alone (every other mesh hidden), -T1-nocap-nogloss without
  the hair cap (#221).

HAIRS: what each covers (DAZ's Render Line Tessellation Sides: 2 = flat
ribbons in Iray, 3 = prisms, under 2 = round `hair` fibres; all Iray Uber):
- Pixie: Genesis 9 + G9 Base dForce Pixie Hair (Genesis 9 Starter
  Essentials, free with DAZ Studio), sides 3, 21 755 strands, Uber PBR
  metallicity mode with a dual lobe specular and no glossy layer,
  translucency 0.33, refraction 0.28. Free content: written to calib/scenes too,
  its references go to git.
- Kevin, Hudson: store ribbon hairs by Morgan's vendor (sides 2) on
  Genesis 8 Basic Male - Kevin in Uber's PBR metallicity mode (Base
  Mixing 0), Hudson in specular / glossiness mode with a top coat as
  Morgan. Private: My Library only.
- Athena: a store hair by another vendor, Uber on fibres (sides 1) on
  Genesis 8 Basic Female. Private: My Library only.

Private scenes stay out of git (no dumps, renders or numbers beyond the
lab note's ratios). The built bases are kept in the cache (--rebuild
builds them again); an existing scene is kept unless --force.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, math, os, shutil, subprocess, sys, urllib.parse
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import duf_camera, duf_io

repo = os.path.dirname(os.path.dirname(here))
CALIB = os.path.join(repo, 'calib', 'scenes')
LIB = os.environ.get('ORBIS_CONTENT', '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library')
SCENES = os.path.join(os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library'), 'Scenes')
BUILD = os.path.expanduser('~/Library/Caches/orbis/hairset')
G9 = 'People/Genesis 9/Genesis 9.duf'
G8M = 'People/Genesis 8 Male/Genesis 8 Basic Male.duf'
G8F = 'People/Genesis 8 Female/Genesis 8 Basic Female.duf'
# name: (figure, hair wearable, generic = may go to git)
HAIRS = {
    'Pixie': (G9, 'People/Genesis 9/Hair/Daz Originals/Base Hair/G9 Base dForce Pixie Hair.duf', True),
    'Kevin': (G8M, 'People/Genesis 8 Male/Hair/Kevin Hair/Kevin Hair.duf', False),
    'Hudson': (G8M, 'People/Genesis 8 Male/Hair/Hudson Hair/Hudson Hair Wearable.duf', False),
    'Athena': (G8F, 'People/Genesis 8 Female/Hair/HM/Athena Hair/Athena Hair.duf', False),
}
FOCAL = 65.0
SIZE = (1280, 720)
# Calib-Hair's exposure (a figure under the bare RuinsB dome); f/8, ISO 100.
EV = 15.7
# The camera: 25 degrees to the figure's left, 15 above, at the distance
# where the hair's bounding-box height would fill FILL of the frame (DAZ
# fits the camera's Frame Width to the image's height). The rendered hair
# takes about 0.6 x FILL (the box's depth, the head's own shape): 0.9 puts
# it at about half the frame, Morgan's share in Calib-Hair-Teens.
YAW, PITCH, FILL = 25.0, 15.0, 0.9


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def save(d, path):
    duf_io.save(path, d)


def build(name):
    figure, hair, _ = HAIRS[name]
    os.makedirs(BUILD, exist_ok=True)
    out = os.path.join(BUILD, f'{name}-base.duf')
    done = out + '.done'
    if os.path.exists(done):
        os.remove(done)
    env = dict(os.environ, DONE_FILE=done)
    # A headless DAZ crashed (segfault) as each Genesis 8 hair started to
    # follow its figure (Kevin, Hudson, Athena, 2026-11-17); with a window
    # it builds. Genesis 9 + the Pixie hair builds headless.
    if figure != G9:
        env['HEADLESS'] = '0'
    subprocess.run([os.path.join(here, 'daz-script.sh'), os.path.join(here, 'daz-build-hair-scene.dsa'),
                    os.path.join(CALIB, 'Calib-L-Distant-45.duf'), os.path.join(LIB, figure), os.path.join(LIB, hair), out, '--'],
                   env=env, check=False)
    log = open(done).read() if os.path.exists(done) else 'no .done'
    print(f'{name}: {log.strip()}')
    if not log.startswith('saved 0'):
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


def hair_nodes(d):
    """Scene nodes whose geometry is strands, with that geometry's .dsf data."""
    out = []
    for n in d['scene']['nodes']:
        for g in n.get('geometries', []):
            path = LIB + urllib.parse.unquote(g.get('url', '').split('#')[0])
            if not os.path.isfile(path):
                continue
            gid = urllib.parse.unquote(g['url'].split('#')[1])
            geo = next((x for x in load(path).get('geometry_library', []) if x['id'] == gid), None)
            if geo and geo.get('polyline_list'):
                out.append((n, g, geo))
    return out


def aim(d):
    pts = [p for _, _, geo in hair_nodes(d) for p in geo['vertices']['values']]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    c = [(a + b) / 2 for a, b in zip(lo, hi)]
    height = hi[1] - lo[1]
    cam = lib_node(d, kind='camera')
    film = next(c['channel'] for e in cam.get('extra', []) for c in e.get('channels', []) if c['channel']['id'] == 'Frame Width')
    vfov = 2 * math.atan(film.get('current_value', film.get('value')) / 2 / FOCAL)
    dist = height / FILL / (2 * math.tan(vfov / 2)) + (hi[2] - lo[2]) / 2
    y, p = math.radians(YAW), math.radians(PITCH)
    pos = (c[0] + dist * math.cos(p) * math.sin(y), c[1] + dist * math.sin(p), c[2] + dist * math.cos(p) * math.cos(y))
    transform(cam, pos, (-PITCH, YAW, 0))
    node_channel(cam, 'Focal Length', FOCAL)
    node_channel(cam, 'DOF', False)
    return lo, hi, pos


def render_setup(d):
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
    # SubD level 1 at render (Genesis 9 at DAZ's 2: 1.6 M faces, as Calib-Shell-G9).
    for n in d['scene']['nodes']:
        for g in n.get('geometries', []):
            for e in g.get('extra', []):
                for c in e.get('channels', []):
                    if c['channel']['id'] == 'SubDRenderLevel':
                        c['channel']['current_value'] = 1


def hair_materials(d):
    geos = {'#' + urllib.parse.quote(g['id']) for _, g, _ in hair_nodes(d)} | {'#' + g['id'] for _, g, _ in hair_nodes(d)}
    return [m for m in d['scene']['materials'] if m.get('geometry') in geos]


# The Uber's specular parts over the base: the glossy layer (Morgan's,
# Calib-Hair-Teens-nogloss; Base Mixing 0 / 1), its weighted-mode twin
# (Base Mixing 2: Glossy Weight - Athena's 0.4, where the layered weight is
# not read) and the dual lobe specular (the Pixie hair's only gloss: Dual
# Lobe Specular Weight 0.53, Glossy Layered Weight 0).
GLOSS = ('Glossy Layered Weight', 'Glossy Weight', 'Dual Lobe Specular Weight')


def nogloss(d):
    mats = hair_materials(d)
    assert mats, 'no hair materials'
    for m in mats:
        ext = next((e for e in m.setdefault('extra', []) if e.get('type') == 'studio_material_channels'), None)
        if ext is None:
            ext = {'type': 'studio_material_channels', 'channels': []}
            m['extra'].append(ext)
        for cid in GLOSS:
            c = next((c['channel'] for c in ext['channels'] if c['channel']['id'] == cid), None)
            if c is None:
                c = {'id': cid, 'type': 'float'}
                ext['channels'].append({'channel': c})
            c['current_value'] = 0.0
    return len(mats)


def sides(d, n):
    """The hair's Line Tessellation sides (Render and Viewport alike: DAZ's
    .mi export reads the Viewport sides, its render the Render sides)."""
    mods = [m for m in d['scene'].get('modifiers', []) if any(e.get('type') == 'studio/modifier/line_tessellation' for e in m.get('extra', []))]
    assert mods, 'no line tessellation modifier'
    for m in mods:
        ext = next(e for e in m['extra'] if e.get('type') == 'studio_modifier_channels')
        for w in ('Render', 'Viewport'):
            cid = f'{w} Line Tessellation Sides'
            c = next((c['channel'] for c in ext['channels'] if c['channel']['id'] == cid), None)
            if c is None:
                c = {'id': cid, 'type': 'int'}
                ext['channels'].append({'channel': c})
            c['current_value'] = n
    return len(mods)


# A top coat on the hair (#203, replacing Calib-Hair-Teens' coat variants):
# Morgan's coat on fibres was never measured in Iray (2026-11-14 note). A
# plain white coat, reflectivity mode, over the hair's own Uber.
COAT = {'Top Coat Weight': 1.0, 'Top Coat Color': [1.0, 1.0, 1.0], 'Top Coat Layering Mode': 0,
        'Top Coat Reflectivity': 0.5, 'Top Coat Roughness': 0.2}


def coat(d):
    mats = hair_materials(d)
    assert mats, 'no hair materials'
    for m in mats:
        ext = next((e for e in m.setdefault('extra', []) if e.get('type') == 'studio_material_channels'), None)
        if ext is None:
            ext = {'type': 'studio_material_channels', 'channels': []}
            m['extra'].append(ext)
        for cid, v in COAT.items():
            c = next((c['channel'] for c in ext['channels'] if c['channel']['id'] == cid), None)
            if c is None:
                c = {'id': cid, 'type': 'float_color' if isinstance(v, list) else 'int' if cid.endswith('Mode') else 'float'}
                ext['channels'].append({'channel': c})
            c['current_value'] = v
    return len(mats)


# What lies under the strands (#221): Pixie's fibre base is 0.7 x Iray's
# and its side patch's lobe share 1.36 x, yet arcs and piles of the same
# material at Pixie's widths and density, over a cap, match (2026-11-22
# pile note). -nofig hides every mesh but the strands (figure, its eyes,
# brows, hair cap, the floor plane: the hair over the bare dome, as the
# arcs), -nocap the hair cap alone. `Visible` off on the node: a hidden
# figure or cap does not hide the hair under it (src/usd/mesh.rs).
def hide(d, pick):
    hair = {id(n) for n, _, _ in hair_nodes(d)}
    hidden = []
    for n in d['scene']['nodes']:
        if not n.get('geometries') or id(n) in hair or not pick(n):
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
        hidden.append(n.get('label'))
    assert hidden, 'nothing hidden'
    return hidden


def nofig(d):
    return hide(d, lambda n: True)


def nocap(d):
    return hide(d, lambda n: 'Hair Cap' in n.get('label', ''))


# The strands' own shape (#221 / #215): with figure and cap out (Iray's
# -nofig-nogloss / -nogloss 1.00 / 1.04 / 0.94, Octane's base still 0.73 x)
# what is left of Pixie against the arcs is how its render hairs lie:
# DAZ's pre-render generator puts `PR Hairs (Per Guide)` 3 around each
# guide and pulls them into clumps (`PR Clumpiness 1` 0.85, 2 clump curves
# per cm2) - neighbours ~4 strand widths apart, 3 - 4 % touching, where the
# piles' random arcs never come within 2. -noclump: clumpiness 0;
# -pr0: no generated hairs per guide. Set on each hair surface's
# simulation modifier (DZ__SPS_<surface>), by label.
def pr(values):
    def run(d):
        hits = 0
        for m in d.get('modifier_library', []):
            if not m.get('id', '').startswith('DZ__SPS_'):
                continue
            for e in m.get('extra', []):
                for c in e.get('channels', []) or []:
                    ch = c.get('channel', {})
                    if ch.get('label') in values:
                        ch['current_value'] = values[ch['label']]
                        hits += 1
        assert hits, f'no {list(values)} channel'
        return hits
    return run


def chain(*fs):
    def run(d):
        return [f(d) for f in fs]
    return run


# The Pixie hair (sides 3) as ribbons (-T2) and fibres (-T1, under 2 = Iray
# `hair`), each whole, without its gloss and with a top coat (#203: the
# ribbon / fibre split, gloss and coat variants Calib-Hair-Teens-* held,
# on free content).
PIXIE_VARIANTS = [(f'-T{n}{s}', chain(lambda d, n=n: sides(d, n), *([f] if f else [])))
                  for n in (2, 1) for s, f in (('', None), ('-nogloss', nogloss), ('-coat', coat))]
# #221: the fibres without what lies under them (hide() above).
PIXIE_VARIANTS += [('-T1-nofig', chain(lambda d: sides(d, 1), nofig)),
                   ('-T1-nofig-nogloss', chain(lambda d: sides(d, 1), nofig, nogloss)),
                   ('-T1-nocap-nogloss', chain(lambda d: sides(d, 1), nocap, nogloss))]
# #221: the render hairs' shape (pr() above), strands alone, no gloss.
PIXIE_VARIANTS += [('-T1-nofig-nogloss-noclump', chain(lambda d: sides(d, 1), nofig, nogloss, pr({'PR Clumpiness 1': 0.0}))),
                   ('-T1-nofig-nogloss-pr0', chain(lambda d: sides(d, 1), nofig, nogloss, pr({'PR Hairs (Per Guide)': 0}))),
                   ('-T1-nofig-nogloss-tipsep', chain(lambda d: sides(d, 1), nofig, nogloss, pr({'PR Hair Tip Separation': 0.2})))]
# -noclump's dump came out identical to -nofig-nogloss (clumpiness does not
# move Pixie's render hairs); what bundles them is the generated hairs
# meeting their guide at the tip (`PR Hair Tip Separation` 0.01 - 0.06 cm
# against `PR Hair Distribution Radius` 0.2 at the root). -tipsep: the tips
# as far apart as the roots, the same strands.


def main():
    force, rebuild = '--force' in sys.argv, '--rebuild' in sys.argv
    names = [a for a in sys.argv[1:] if not a.startswith('--')] or list(HAIRS)
    for name in names:
        _, _, generic = HAIRS[name]
        base = os.path.join(BUILD, f'{name}-base.duf')
        if rebuild or not os.path.exists(base):
            base = build(name)
        d0 = load(base)
        lo, hi, pos = aim(d0)
        render_setup(d0)
        print(f'{name}: hair {[round(v) for v in lo]} - {[round(v) for v in hi]} cm, camera {[round(v) for v in pos]}')
        variants = [('', None), ('-nogloss', nogloss)]
        if name == 'Pixie':
            variants += PIXIE_VARIANTS
        for suffix, change in variants:
            scene = f'Calib-Hair-{name}{suffix}'
            d = copy.deepcopy(d0)
            if change:
                print(f'  {scene}: {change(d)} hair materials')
            d['asset_info']['id'] = f'/Scenes/{scene}.duf'
            duf_camera.ensure(d)
            targets = [SCENES] + ([CALIB] if generic else [])
            for dirpath in targets:
                dst = os.path.join(dirpath, scene + '.duf')
                if os.path.exists(dst) and not force:
                    print(f'  kept {dst}')
                    continue
                save(d, dst)
                print(f'  wrote {dst}')


if __name__ == '__main__':
    main()
