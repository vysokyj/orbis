#!/usr/bin/env python3
"""ORBIS calibration: a scene with some material channels changed.

  python3 scripts/dev/scene-channels.py <in.duf> <out.duf> --mat <text>[,<text>...] <Channel>=<value>...

Sets each channel on every scene material whose id contains one of the
<text>s (one --mat, several channels). <value>: a number, `r,g,b`, `true` / `false`, or
`nomap` (drops the channel's image map, keeps its value) - added to the
scene material when it doesn't override that channel yet, as DAZ saves a
changed channel. The scene's own content is copied as it is, so a scene
with store content stays out of git like its source (#161: Calib-Hair-Teens
variants with Morgan Hair's parts off, local). Writes gzip, keeps the
camera (duf_camera.ensure).
"""
import sys
if len(sys.argv) < 5 or sys.argv[1] in ('-h', '--help') or '--mat' not in sys.argv:
    print(__doc__)
    sys.exit(0 if len(sys.argv) > 1 and sys.argv[1] in ('-h', '--help') else 2)
import gzip, json, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

src, dst = sys.argv[1], sys.argv[2]
at = sys.argv.index('--mat')
texts, sets = sys.argv[at + 1].split(','), sys.argv[at + 2:]


def value(v):
    if v in ('true', 'false'):
        return v == 'true'
    if ',' in v:
        return [float(x) for x in v.split(',')]
    return float(v)


d = duf_camera.load(src)
n = 0
for m in d.get('scene', {}).get('materials', []):
    if not any(t in m.get('id', '') for t in texts):
        continue
    extra = m.setdefault('extra', [])
    chans = next((e for e in extra if e.get('type') == 'studio_material_channels'), None)
    if chans is None:
        chans = {'type': 'studio_material_channels', 'channels': []}
        extra.append(chans)
    for s in sets:
        cid, v = s.split('=', 1)
        c = next((c['channel'] for c in chans['channels'] if c['channel']['id'] == cid), None)
        if v == 'nomap':
            if c is not None:
                c.pop('image_file', None)
                c.pop('image', None)
            continue
        if c is None:
            chans['channels'].append({'channel': {'id': cid, 'current_value': value(v)}})
        else:
            c['current_value'] = value(v)
    n += 1
    print('set', m['id'], ' '.join(sets))
if not n:
    sys.exit(f'no material id contains any of {texts!r}')
name = os.path.splitext(os.path.basename(dst))[0]
if 'asset_info' in d:
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
duf_camera.ensure(d)
open(dst, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
print(dst)
