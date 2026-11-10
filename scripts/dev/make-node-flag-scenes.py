#!/usr/bin/env python3
"""ORBIS calibration: node render flag scenes (#36), the second step after
scripts/dev/daz-build-node-flags.dsa.

  python3 scripts/dev/make-node-flag-scenes.py <DAZ out dir> [<My Library Scenes dir>]

Takes the scenes the DAZ script saved, aims Calib-Shell-G9's camera at the
figure (whole body, straight on: DAZ's setWSPos didn't stick in a saved
scene), renders the figure at SubD level 1 (2.3 M faces with the shell at
DAZ's 2: too big for a calibration render) and copies them into calib/scenes and My Library/Scenes
(ORBIS_LIBRARY or the argument). Only DAZ built-ins and primitives: the
base scene's, and Genesis 9 (free with DAZ Studio), referenced only.
"""
import gzip, json, os, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
calib = os.path.join(repo, 'calib', 'scenes')
src = sys.argv[1]
lib = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library'), 'Scenes')
SCENES = ['Calib-Mi-Priority', 'Calib-CastShadows-Off', 'Calib-Shell-G9']


def aim(d, pos, rot):
    cam = next(n for n in d['node_library'] if n['id'] == 'Camera')
    for key, values in (('translation', pos), ('rotation', rot)):
        for c, v in zip(cam[key], values):
            c['current_value'] = v


for name in SCENES:
    raw = open(os.path.join(src, name + '.duf'), 'rb').read()
    d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
    if name == 'Calib-Shell-G9':
        aim(d, [0, 92, 440], [0, 0, 0])
        for n in d['scene']['nodes']:
            for g in n.get('geometries', []):
                for e in g.get('extra', []):
                    for c in e.get('channels', []):
                        if c['channel']['id'] == 'SubDRenderLevel':
                            c['channel']['current_value'] = 1
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    duf_camera.ensure(d)  # DAZ saved them through its Perspective View (#97)
    dst = os.path.join(calib, name + '.duf')
    open(dst, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    if os.path.isdir(lib):
        shutil.copyfile(dst, os.path.join(lib, name + '.duf'))
    print(name)
