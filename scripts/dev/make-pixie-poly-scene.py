#!/usr/bin/env python3
"""ORBIS calibration (#221): Pixie's render hairs as a plain polyline node.

  python3 scripts/dev/make-pixie-poly-scene.py [--scene NAME] [--material COL] [--split] [--draw-dome]

Reads the cached dump of calib/scenes/<NAME>.duf (default
Calib-Hair-Pixie-T1-nofig-nogloss-pr0: Pixie's guides alone) and writes
"My Library/Scenes/<NAME>-poly.duf": the dump's strands as one polyline
node in world space (no figure, no dForce generator), on
Calib-Strand-DensePile-HDRI's material of column COL (default 4: Pixie
without its dual lobe, as -nogloss), under -DensePile's dome (DAZ's
default, not drawn) and through <NAME>'s camera and image size - so
Pixie's boxes (hair-set-compare.py) read it as they read <NAME>.

The geometry comes from Pixie Hair (free DAZ content): the scene stays in
My Library, never in git; this generator is the reproducible part.
Splits what DAZ hands Iray for a generated hair (figure, generator, Line
Tessellation) from the strands themselves: Octane / Iray on the polyline
copy against <NAME>'s 0.73. --split writes <NAME>-poly-split.duf: Pixie's
Hair02 strands on their own node with the refraction off (as DAZ hands
Iray Hair02), the rest on COL's material. --draw-dome draws the dome
(-drawn), as <NAME> does.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera, duf_io

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
args = sys.argv[1:]
name = args[args.index('--scene') + 1] if '--scene' in args else 'Calib-Hair-Pixie-T1-nofig-nogloss-pr0'
col = int(args[args.index('--material') + 1]) if '--material' in args else 4
split = '--split' in args
drawn = '--draw-dome' in args

src = os.path.join(REPO, 'calib', 'scenes', name + '.duf')
cache = os.path.expanduser('~/Library/Caches/orbis/dumps')
stem = name + '-'
dump = None
for f in sorted(os.listdir(cache)):
    if f.startswith(stem) and len(f) == len(stem) + 13 and f.endswith('.json'):
        d = json.load(open(os.path.join(cache, f)))
        if d.get('scene', '').endswith(name + '.duf'):
            dump = d
            break
if dump is None:
    sys.exit(f'no cached dump of {src}')
node = next(n for n in dump['nodes'] if n.get('polylines'))
v = node['vertices']
strands = [[[round(v[3 * i + c], 4) for c in range(3)] for i in q] for q in node['polylines']]
groups = [node['material_groups'][g] for g in node['polyline_materials']]

base = duf_io.load(os.path.join(REPO, 'calib', 'scenes', 'Calib-Strand-DensePile-HDRI.duf'))
pix = duf_io.load(src)
oid = f'H{col}'
gid = 'geom-' + oid
d = copy.deepcopy(base)
d['asset_info']['id'] = f'/Scenes/{name}-poly.duf'
d['asset_info']['contributor'] = {'author': '', 'email': '', 'website': ''}
keep = {'Camera', 'Environment Options', 'Tonemapper Options', oid}
d['node_library'] = [n for n in d['node_library'] if n['id'] in keep]
d['scene']['nodes'] = [n for n in d['scene']['nodes'] if n['id'].rsplit('-', 1)[0] in keep]
d['geometry_library'] = [g for g in d['geometry_library'] if g['id'] == gid]
d['uv_set_library'] = [u for u in d['uv_set_library'] if u['id'] == 'uv-' + gid]
d['scene']['materials'] = [m for m in d['scene']['materials'] if m['geometry'] == '#' + gid + '-i']
d['material_library'] = [m for m in d['material_library'] if m['id'] == 'Mat-' + oid]
# Pixie's camera (and image size) in place of -DensePile's
for lib in ('node_library',):
    d[lib] = [n for n in d[lib] if n['id'] != 'Camera'] + [copy.deepcopy(next(n for n in pix[lib] if n['id'] == 'Camera'))]
d['scene']['nodes'] = [n for n in d['scene']['nodes'] if n['id'] != 'Camera-1'] + [copy.deepcopy(next(n for n in pix['scene']['nodes'] if n['id'] == 'Camera-1'))]
d['scene']['current_camera'] = pix['scene'].get('current_camera', '#Camera-1')
for e in d['scene']['extra']:
    if e.get('type') == 'studio_render_settings':
        e['render_options']['imageSize'] = next(x for x in pix['scene']['extra'] if x.get('type') == 'studio_render_settings')['render_options']['imageSize']


def fill(oid, gid, strands, label):
    """The node `oid`'s polyline geometry = `strands` (world space)."""
    g = next(x for x in d['geometry_library'] if x['id'] == gid)
    verts, lines = [], []
    for s in strands:
        lines.append([0, 0] + list(range(len(verts), len(verts) + len(s))))
        verts.extend(s)
    g['vertices'] = {'count': len(verts), 'values': verts}
    g['polyline_list'] = {'count': len(lines), 'segment_count': sum(len(l) - 3 for l in lines), 'values': lines}
    uv = next(u for u in d['uv_set_library'] if u['id'] == 'uv-' + gid)
    uv.update({'vertex_count': len(verts), 'uvs': {'count': len(verts), 'values': [[0, 0]] * len(verts)}})
    nl = next(n for n in d['node_library'] if n['id'] == oid)
    for c in nl['translation']:
        c['current_value'] = 0.0
    nl['label'] = label


suffix = '-poly'
if split:
    # --split: Pixie's Hair02 strands (Hair01 without the refraction, the
    # .mi) on their own node, H<col>'s material with Refraction Weight 0;
    # the rest (Hair01, Hairline01 / 02: the same arguments) on H<col>'s.
    suffix = '-poly-split'
    o2, g2 = oid + 'r', gid + 'r'
    for lib, key, new in (('node_library', oid, o2), ('geometry_library', gid, g2), ('uv_set_library', 'uv-' + gid, 'uv-' + g2), ('material_library', 'Mat-' + oid, 'Mat-' + o2)):
        x = copy.deepcopy(next(e for e in d[lib] if e['id'] == key))
        x['id'] = new
        if 'name' in x and lib != 'uv_set_library':
            x['name'] = new
        if lib == 'geometry_library':
            x['default_uv_set'] = '#uv-' + g2
        d[lib].append(x)
    m2 = next(m for m in d['material_library'] if m['id'] == 'Mat-' + o2)
    for e in m2['extra']:
        for c in e.get('channels', []) or []:
            if c['channel']['id'] == 'Refraction Weight':
                c['channel']['current_value'] = 0.0
    sn = copy.deepcopy(next(n for n in d['scene']['nodes'] if n['id'] == oid + '-1'))
    sn.update({'id': o2 + '-1', 'url': '#' + o2, 'name': o2, 'label': o2})
    sn['geometries'] = [{**sn['geometries'][0], 'id': g2 + '-i', 'url': '#' + g2}]
    d['scene']['nodes'].append(sn)
    sm = copy.deepcopy(d['scene']['materials'][0])
    sm.update({'id': 'Mat-' + o2 + '-i', 'url': '#Mat-' + o2, 'geometry': '#' + g2 + '-i', 'uv_set': '#uv-' + g2})
    d['scene']['materials'].append(sm)
    fill(o2, g2, [s for s, gname in zip(strands, groups) if gname == 'Hair02'], f'{o2} {name} Hair02 strands')
    fill(oid, gid, [s for s, gname in zip(strands, groups) if gname != 'Hair02'], f'{oid} {name} strands but Hair02')
else:
    fill(oid, gid, strands, f'{oid} {name} strands')
if drawn:
    # --draw-dome: the dome drawn, as in <NAME> (the -HDRI scenes don't).
    suffix += '-drawn'
    for src_n in (next(n for n in d['node_library'] if n['id'] == 'Environment Options'), next(n for n in d['scene']['nodes'] if n['id'] == 'Environment Options-1')):
        for e in src_n.get('extra', []):
            for c in e.get('channels', []) or []:
                if c['channel']['id'] == 'Draw Dome':
                    c['channel']['current_value'] = True
d['asset_info']['id'] = f'/Scenes/{name}{suffix}.duf'
duf_camera.ensure(d)
p = os.path.join(duf_io.library, 'Scenes', name + suffix + '.duf')
duf_io.save(p, d)
print(p, len(strands), 'strands')
