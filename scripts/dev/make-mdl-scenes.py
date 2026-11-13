#!/usr/bin/env python3
"""ORBIS calibration scenes that use our own MDL modules (#112).

  python3 scripts/dev/make-mdl-scenes.py [--force] [--root <dir>] [<library>]

Installs `calib/mdl/*.mdl` (our own modules, nothing of NVIDIA's or DAZ's)
into the content library under `<root>/orbis/` (default root
`shaders/iray`, the layout of DAZ's own `Contents/shaders/iray`), writes a
material file with one Shader Mixer brick material per module material
(`DzCustomMDLBrick` "MDL Callable" `orbis/<module>.mdl#<material>` into
`DzMDLSurfaceBrick`, the graph DAZ's NVIDIA MDL Examples use) to
`<library>/data/ORBIS/Calib/MDL Rim/MDL Rim.dsf`, and the scenes into
`<library>/Scenes`, copied to `calib/scenes`:

- Calib-Mi-MdlPath (.mi only): one sphere per candidate MDL search root
  (`<library>/shaders/iray`, `<library>`, `<library>/data`), which one DAZ
  resolves a custom module from.
- Calib-Furnace-MdlVersion: Calib-Furnace-Metal's spheres under the uniform
  dome, floor hidden, the same material at "mdl 1.0" and "mdl 1.3" side by
  side - the bare white lobe at r 0.1 and 0.4, the lobe under
  metal_generic's two weighted layers, a closed rough refractor (IOR 1.5,
  r 0.2). Iray reads simple_glossy_bsdf with a legacy behaviour in modules
  older than 1.3 (MDL SDK `set_simple_glossy_bsdf_legacy_enabled`); NVIDIA's
  examples are 1.0, DAZ's Uber 1.7.
- Calib-Furnace-MdlLobe: the "mdl 1.0" lobe at Calib-Furnace-Metal's eight
  roughnesses (its Uber spheres are the same lobe at 1.7).
- Calib-Furnace-MdlTransmit: the closed refractor at r 0.05 / 0.1 / 0.2 /
  0.4 in "mdl 1.0" and "mdl 1.3" (#116).

Calib-Mi-MdlPath's .mi (2026-11-13): only P1 resolves, and only once the
script maps `<library>/shaders/iray` as an MDL directory - DAZ searches its
own `shaders/iray` and its MDL directories (none set), not the content
directories. daz-export-mi.dsa and daz-render-queue.dsa map
`<content dir>/shaders/iray` for their session (addMDLDirectory).

Each module may be imported once per DAZ session (#94): the scenes use
different modules, but a second render of any needs a DAZ restart.
Scenes are kept when they exist (--force rewrites); the modules and the
material file are always rewritten.
"""
import copy, gzip, json, os, shutil, sys

if '-h' in sys.argv or '--help' in sys.argv:
    print(__doc__)
    sys.exit(0)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
args = [a for a in sys.argv[1:] if not a.startswith('--')]
FORCE = '--force' in sys.argv
ROOT = sys.argv[sys.argv.index('--root') + 1] if '--root' in sys.argv else 'shaders/iray'
if ROOT in args:
    args.remove(ROOT)
LIB = args[0] if args else '/Volumes/Data/DAZ 3D/Studio/My Library'
SCENES = os.path.join(LIB, 'Scenes')
DSF_URL = '/data/ORBIS/Calib/MDL%20Rim/MDL%20Rim.dsf'
DSF = os.path.join(LIB, 'data/ORBIS/Calib/MDL Rim/MDL Rim.dsf')
FURNACE_METAL = os.path.join(SCENES, 'Calib-Furnace-Metal.duf')


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def s(value):
    """A Shader Mixer settings node."""
    return {'type': 'settings', 'value': value}


def brick_material(mid, callable_):
    """A material_library entry: a custom MDL material straight into the
    MDL surface brick, no user parameters."""
    custom = s({'DAZStudioClassName': 'DzCustomMDLBrick',
                'Attributes': s({'BrickSettings': s({'BrickTitle': 'Custom MDL (1)', 'HideParams': False, 'PosX': -143, 'PosY': -312,
                                                     'BrickIsRightToLeft': True, 'Width': ['i', 200], 'Height': ['i', 120]}),
                                 'BrickVersion': 1,
                                 'BrickProperties': s({'MDL Callable': s({'Label': 'MDL Callable', 'Hidden': False, 'Path': '/General',
                                                                          'UserProperty': False, 'CanAnimate': False, 'Locked': False,
                                                                          'Manipulator': True, 'Favorite': False, 'GlobalFavorite': False,
                                                                          'Selected': False, 'AutoFollow': False, 'Value': callable_})})}),
                'Parameters': s({'Result': s({'Tokens': 'elr_Result_b1_t0', 'Label': 'Result'})})})
    surface = s({'DAZStudioClassName': 'DzMDLSurfaceBrick',
                 'Attributes': s({'BrickSettings': s({'BrickSet': s({'Name': 'shader_MDL_Surface_1', 'LastInvalidate': '0'}),
                                                      'BrickTitle': 'MDL Surface (2)', 'HideParams': False, 'PosX': -409, 'PosY': -276,
                                                      'BrickIsRightToLeft': True, 'Width': ['i', 200], 'Height': ['i', 135]}),
                                  'BrickVersion': 1, 'BrickProperties': s({})}),
                 'Parameters': s({'material': s({'Label': 'Material'})})})
    setup = s({'Bricks': s({'Brick_1': custom, 'Brick_2': surface}),
               'Connections': s({'Connection_0': s({'SourceBrick': 'Brick_2', 'TargetBrick': 'Brick_1',
                                                    'SourceParam': 'material', 'TargetParam': 'Result'})}),
               'BrickUserName': mid})
    return {'id': mid, 'extra': [{'type': 'studio/material/daz_brick', 'brick_settings': {'BrickSetup': setup}},
                                 {'type': 'studio_material_channels', 'channels': []}]}


# (scene, [(sphere label, material id, MDL callable)]); the module file of
# each callable comes from calib/mdl, installed under ROOT unless the
# callable's material id says otherwise (the path probe).
PATH_PROBE = [('P1 root shaders-iray', 'path_shaders_iray', 'orbis/orbis_path_a.mdl#probe', 'shaders/iray'),
              ('P2 root library', 'path_library', 'orbis/orbis_path_b.mdl#probe', ''),
              ('P3 root data', 'path_data', 'orbis/orbis_path_c.mdl#probe', 'data')]
VERSION = [('M1 v1.0 reflect r 0.1', 'v10_reflect_r01', 'orbis/orbis_rim_v10.mdl#reflect_r01'),
           ('M2 v1.3 reflect r 0.1', 'v13_reflect_r01', 'orbis/orbis_rim_v13.mdl#reflect_r01'),
           ('M3 v1.0 reflect r 0.4', 'v10_reflect_r04', 'orbis/orbis_rim_v10.mdl#reflect_r04'),
           ('M4 v1.3 reflect r 0.4', 'v13_reflect_r04', 'orbis/orbis_rim_v13.mdl#reflect_r04'),
           ('M5 v1.0 layered r 0.1', 'v10_layered_r01', 'orbis/orbis_rim_v10.mdl#layered_r01'),
           ('M6 v1.3 layered r 0.1', 'v13_layered_r01', 'orbis/orbis_rim_v13.mdl#layered_r01'),
           ('M7 v1.0 transmit r 0.2', 'v10_transmit_r02', 'orbis/orbis_rim_v10.mdl#transmit_r02'),
           ('M8 v1.3 transmit r 0.2', 'v13_transmit_r02', 'orbis/orbis_rim_v13.mdl#transmit_r02')]
# For the next step if "mdl 1.0" is the rim: the 1.0 lobe at
# Calib-Furnace-Metal's roughnesses (its Uber spheres are the 1.7 twin),
# and the closed refractor at four roughnesses in 1.0 and 1.3 (#116).
R_NAME = lambda r: ('%g' % r).replace('.', '')
LOBE = [(f'L{k + 1} v1.0 reflect r {r}', f'v10_lobe_r{R_NAME(r)}', f'orbis/orbis_lobe_v10.mdl#reflect_r{R_NAME(r)}')
        for k, r in enumerate((0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.7, 1.0))]
TRANSMIT = [(f'T{k + 1} v{v} transmit r {r}', f'v{v.replace(".", "")}_sweep_transmit_r{R_NAME(r)}',
             f'orbis/orbis_transmit_v{v.replace(".", "")}.mdl#transmit_r{R_NAME(r)}')
            for k, (v, r) in enumerate([(v, r) for v in ('1.0', '1.3') for r in (0.05, 0.1, 0.2, 0.4)])]

# Modules: ours from calib/mdl under ROOT; the path probe's three copies of
# one tiny module under each candidate root.
src = os.path.join(REPO, 'calib/mdl')
dst = os.path.join(LIB, ROOT, 'orbis')
os.makedirs(dst, exist_ok=True)
for f in sorted(os.listdir(src)):
    if f.endswith('.mdl') and not f.startswith('orbis_path'):
        shutil.copyfile(os.path.join(src, f), os.path.join(dst, f))
probe = open(os.path.join(src, 'orbis_path.mdl')).read()
for _, _, c, root in PATH_PROBE:
    d = os.path.join(LIB, root, 'orbis')
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, c.split('/')[1].split('#')[0]), 'w').write(probe)

os.makedirs(os.path.dirname(DSF), exist_ok=True)
lib_mats = [brick_material(mid, c) for _, mid, c, *_ in PATH_PROBE + VERSION + LOBE + TRANSMIT]
dsf = {'file_version': '0.6.0.0',
       'asset_info': {'id': DSF_URL, 'type': 'material', 'contributor': {'author': 'ORBIS', 'email': '', 'website': ''}, 'revision': '1.0'},
       'material_library': lib_mats}
open(DSF, 'wb').write(gzip.compress(json.dumps(dsf, indent=1).encode()))
print(DSF)


def furnace_scene(name, spheres):
    """Calib-Furnace-Metal (uniform dome, floor hidden) with its first
    len(spheres) spheres, in label order F1 ..., on our brick materials;
    the rest hidden."""
    p = os.path.join(SCENES, name + '.duf')
    if os.path.exists(p) and not FORCE:
        return p
    d = load(FURNACE_METAL)
    d['asset_info']['id'] = '/Scenes/' + name + '.duf'
    geo_node = {g['id']: n for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    for i, m in enumerate(d['scene']['materials']):
        n = geo_node.get(m['geometry'].lstrip('#'))
        if not n or not n.get('label', '').startswith('F'):
            continue
        k = int(n['label'][1]) - 1
        if k >= len(spheres):
            ext = next((e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}])
                        if e.get('type') == 'studio_node_channels'), None)
            if ext is None:
                ext = {'type': 'studio_node_channels', 'version': 5}
                n['extra'].append(ext)
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
            n['label'] = 'hidden ' + n['label']
            continue
        label, mid = spheres[k][0], spheres[k][1]
        d['scene']['materials'][i] = {'id': m['id'], 'url': DSF_URL + '#' + mid, 'geometry': m['geometry'],
                                      'groups': m.get('groups', ['Default']), 'uv_set': m.get('uv_set'),
                                      'extra': [{'type': 'studio/material/daz_brick'}]}
        if d['scene']['materials'][i]['uv_set'] is None:
            del d['scene']['materials'][i]['uv_set']
        n['label'] = label
    duf_camera.ensure(d)  # else Iray renders DAZ's Perspective View (#97)
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    shutil.copyfile(p, os.path.join(REPO, 'calib/scenes', name + '.duf'))
    print(p)
    return p


furnace_scene('Calib-Mi-MdlPath', PATH_PROBE)
furnace_scene('Calib-Furnace-MdlVersion', VERSION)
furnace_scene('Calib-Furnace-MdlLobe', LOBE)
furnace_scene('Calib-Furnace-MdlTransmit', TRANSMIT)
