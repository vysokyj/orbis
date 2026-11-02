#!/usr/bin/env python3
"""ORBIS calibration: keep Iray renders of the generated scenes in git.

  python3 scripts/dev/keep-refs.py [--demo] [--renderer NAME] [--queue-log LOG] <render.png>...

--demo: renders of Demo- scenes (demo/scenes.toml), kept in demo/references/iray with the scene's sha256

Copies each render into calib/references/iray/ and records how it was made in
calib/references/iray/renders.tsv (samples, time cap, renderer, date, render
time). Only scenes that are in calib/scenes are taken (generated from scratch,
DAZ built-ins and primitives only); every other render is skipped, so a
user's own scene or DAZ Store content never gets into the repo. A render with
fewer samples than the one already kept is skipped too.

Samples and the time cap come from the capped copy of the scene the render
was made from (beside the png, or in the queue's done/ folder); the render
time from the queue log or the <png>.run.txt that daz-render.sh leaves.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import csv, datetime, filecmp, gzip, json, os, re, shutil, sys

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
scenes = os.path.join(repo, 'calib', 'scenes')
refs = os.path.join(repo, 'calib', 'references', 'iray')
calib = os.path.expanduser('~/Library/Caches/orbis/calib')
FIELDS = ['scene', 'samples', 'max_time', 'renderer', 'rendered', 'seconds']

args = sys.argv[1:]
demo = '--demo' in args
if demo:
    # Demo- scenes: store content stays in the library, only renders are kept.
    args.remove('--demo')
    refs = os.path.join(repo, 'demo', 'references', 'iray')
    FIELDS = FIELDS + ['scene_sha256']
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import demo as demo_lib
tsv = os.path.join(refs, 'renders.tsv')
renderer = 'DAZ Studio Iray (CPU)'
logs = []
pngs = []
while args:
    a = args.pop(0)
    if a == '--renderer': renderer = args.pop(0)
    elif a == '--queue-log': logs.append(args.pop(0))
    else: pngs.append(a)
if not pngs:
    sys.exit(__doc__)


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def caps(p):
    found = {}
    def walk(v):
        if isinstance(v, dict):
            if v.get('id') in ('Max Samples', 'Max Time'): found[v['id']] = v.get('current_value', v.get('value'))
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
    for e in load(p)['scene'].get('extra', []):
        if e.get('type') == 'studio_render_settings': walk(e)
    return found


def seconds(name, png):
    for log in logs:
        hits = re.findall(rf'rendered {re.escape(name)}\.duf in (\d+) s', open(log).read())
        if hits: return hits[-1]
    run = png + '.run.txt'
    if os.path.exists(run):
        hits = re.findall(r'rendered in (\d+) s', open(run).read())
        if hits: return hits[-1]
    return ''


rows = {}
if os.path.exists(tsv):
    rows = {r['scene']: r for r in csv.DictReader(open(tsv), delimiter='\t')}
changed = 0
for png in pngs:
    name = os.path.basename(png)[:-4]
    if demo:
        if name not in demo_lib.entries():
            print(f'skip {name}: not in demo/scenes.toml (python3 scripts/dev/demo.py add)')
            continue
        lib_scene = demo_lib.scene_path(name)
        if not os.path.exists(lib_scene) or os.path.getmtime(lib_scene) > os.path.getmtime(png):
            print(f'skip {name}: the scene was saved after this render, so the render is stale')
            continue
    elif not os.path.exists(os.path.join(scenes, name + '.duf')):
        print(f'skip {name}: not a generated scene in calib/scenes')
        continue
    copy = next((p for p in (png[:-4] + '.duf', os.path.join(calib, 'queue', 'done', name + '.duf'),
                             os.path.join(calib, 'queue-local', 'done', name + '.duf')) if os.path.exists(p)), None)
    if not copy:
        print(f'skip {name}: no capped scene copy to read the samples from')
        continue
    c = caps(copy)
    samples = int(c.get('Max Samples', 0))
    old = rows.get(name)
    dst = os.path.join(refs, name + '.png')
    if old and int(old['samples'] or 0) > samples:
        print(f'skip {name}: {samples} samples, the kept one has {old["samples"]}')
        continue
    if os.path.exists(dst) and filecmp.cmp(png, dst, shallow=False) and old:
        continue
    shutil.copyfile(png, dst)
    rows[name] = {'scene': name, 'samples': samples, 'max_time': c.get('Max Time', ''), 'renderer': renderer,
                  'rendered': datetime.datetime.fromtimestamp(os.path.getmtime(png)).strftime('%Y-%m-%d %H:%M'),
                  'seconds': seconds(name, png)}
    if demo:
        rows[name]['scene_sha256'] = demo_lib.sha256(demo_lib.copy_path(name))
    changed += 1
    print(f'kept {name}: {samples} samples, {renderer}')
with open(tsv, 'w', newline='') as f:
    w = csv.DictWriter(f, FIELDS, delimiter='\t', lineterminator='\n')
    w.writeheader()
    for k in sorted(rows):
        w.writerow(rows[k])
print(f'{changed} kept, {len(rows)} in renders.tsv')
