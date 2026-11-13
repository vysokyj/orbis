#!/usr/bin/env python3
"""ORBIS calibration: a scene's strand hair handed to Iray another way.

  python3 scripts/dev/strand-tess.py <in.duf> <out.duf> <sides> [--parent <text>]

Sets the Viewport and Render Line Tessellation Sides of every Line
Tessellation modifier in the scene (only those whose parent node id
contains <text>, with --parent) to <sides>: under 2 DAZ hands Iray the
polylines as `hair` fibres (round, as Octane's curves), 2 as flat ribbons,
3 as prisms (make-strand-scenes.py). The geometry and materials stay; the
scene's own content is copied as it is, so a scene with store content stays
out of git like its source (#99: Calib-Hair-Teens with Morgan Hair as
fibres, local). Writes gzip, keeps the camera (duf_camera.ensure).
"""
import sys
if len(sys.argv) < 4 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0 if len(sys.argv) > 1 and sys.argv[1] in ('-h', '--help') else 2)
import gzip, json, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

src, dst, sides = sys.argv[1], sys.argv[2], int(sys.argv[3])
parent = sys.argv[sys.argv.index('--parent') + 1] if '--parent' in sys.argv else ''
d = duf_camera.load(src)
n = 0
for m in d.get('scene', {}).get('modifiers', []):
    extra = m.setdefault('extra', [])
    if not any(e.get('type') == 'studio/modifier/line_tessellation' for e in extra):
        continue
    if parent not in str(m.get('parent', '')):
        continue
    chans = next((e for e in extra if e.get('type') == 'studio_modifier_channels'), None)
    if chans is None:
        chans = {'type': 'studio_modifier_channels', 'channels': []}
        extra.append(chans)
    for w in ('Viewport', 'Render'):
        cid = f'{w} Line Tessellation Sides'
        c = next((c['channel'] for c in chans['channels'] if c['channel']['id'] == cid), None)
        if c is None:
            chans['channels'].append({'channel': {'id': cid, 'type': 'int', 'current_value': sides}})
        else:
            c['current_value'] = sides
    n += 1
    print('set', m.get('parent'), sides)
if not n:
    sys.exit('no Line Tessellation modifier matched')
name = os.path.splitext(os.path.basename(dst))[0]
if 'asset_info' in d:
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
duf_camera.ensure(d)
open(dst, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
print(dst)
