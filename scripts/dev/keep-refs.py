#!/usr/bin/env python3
"""ORBIS calibration: keep Iray renders of the generated scenes in git.

  python3 scripts/dev/keep-refs.py [--demo] [--renderer NAME] [--queue-log LOG] <render.png>...

--demo: renders of Demo- scenes (demo/scenes.toml), kept in demo/references/iray with the scene's sha256

Copies each render into calib/references/iray/ and records how it was made in
calib/references/iray/renders.tsv (samples, time cap, renderer, date, render
time, and `denoiser`: DAZ's Post Denoiser as the image had it, `on@<start
iteration>` or `off` - the compare scripts read Octane's matching pass, #139).
The renderer is told by the copy the render was made from: one in the
queue's done/ folder went through the Iray Server (iray-queue.sh), one
whose <png>.run.txt names the Iray Server through its own job queue
(iray-server.py submit, #195), any other through the local CPU Iray
(daz-render.sh); --renderer overrides it. The Iray
Server applies the Post Denoiser; the local CPU Iray never does (NVIDIA's
deep-learning denoiser runs on GPUs only), whatever the scene says, so a CPU
render is recorded `off` (#171). So is a headless one whose <png>.run.txt
says `device: cpu`: the server's GPU failed (CUDA error 700 on strand hair
or a G9 figure, 4 GB card) and its worker rendered on the CPU (#217;
iray-server.py reads the device from the server's log), and a Bridge queue
render whose <png>.run.txt (daz-render-queue.dsa, #219) says so. Only scenes that are in calib/scenes (generated from scratch,
DAZ built-ins and primitives only) or calib/scenes-store (calibration scenes
with the user's store content, put in git by the user's decision, 2026-11-18)
are taken; every other render is skipped, so a scene the user did not put in
git never gets its render into the repo. A render with
fewer samples than the one already kept is skipped too.

Samples and the time cap come from the capped copy of the scene the render
was made from (beside the png, or in the queue's done/ folder); the render
time from the queue log or the <png>.run.txt that daz-render.sh leaves.
A render whose scene load logged an MDL import error in DAZ's log (a preset
module already imported in that DAZ session, #94; daz_log.py) is skipped, and
so is one made from a copy without current_camera (DAZ rendered its
Perspective View, not the scene's camera, #97; duf_camera.py).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import csv, datetime, filecmp, gzip, json, os, re, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import daz_log, duf_camera

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
scenes = os.path.join(repo, 'calib', 'scenes')
refs = os.path.join(repo, 'calib', 'references', 'iray')
calib = os.path.expanduser('~/Library/Caches/orbis/calib')
FIELDS = ['scene', 'samples', 'max_time', 'renderer', 'rendered', 'seconds', 'denoiser']

args = sys.argv[1:]
store_scenes = os.path.join(repo, 'calib', 'scenes-store')
demo = '--demo' in args
if demo:
    # Demo- scenes: store content stays in the library, only renders are kept.
    args.remove('--demo')
    refs = os.path.join(repo, 'demo', 'references', 'iray')
    FIELDS = FIELDS + ['scene_sha256']
    import demo as demo_lib
tsv = os.path.join(refs, 'renders.tsv')
SERVER = 'Iray Server (Boost for Daz)'
HEADLESS = 'Iray Server (headless submit)'
CPU = 'DAZ Studio Iray (CPU)'
renderer = None
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
            if v.get('id') in ('Max Samples', 'Max Time', 'Post Denoiser Enable', 'Post Denoiser Start Iteration'):
                found[v['id']] = v.get('current_value', v.get('value'))
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
    for e in load(p)['scene'].get('extra', []):
        if e.get('type') == 'studio_render_settings': walk(e)
    return found


def device(png):
    # iray-server.py submit writes `device: gpu|cpu|?` from the server's log (#217).
    run = png + '.run.txt'
    if os.path.exists(run):
        m = re.search(r'^device: (\w+|\?)', open(run).read(), re.M)
        if m: return m[1]
    return None


def denoiser(c, how, dev=None):
    # Iray's Post Denoiser, as the render's copy had it (#139). The CPU Iray
    # can't run it (deep-learning denoiser, GPU only; #171 measured a CPU
    # 400-sample render's white grain at 3 - 5 x the server's): `off`. Also
    # a server render its worker made on the CPU after a GPU error (#217).
    if how == CPU or dev == 'cpu':
        return 'off'
    if 'Post Denoiser Enable' not in c:
        return '?'
    return f"on@{c.get('Post Denoiser Start Iteration', '?')}" if c['Post Denoiser Enable'] else 'off'


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
    elif not any(os.path.exists(os.path.join(d, name + '.duf')) for d in (scenes, store_scenes)):
        print(f'skip {name}: not a scene in calib/scenes or calib/scenes-store')
        continue
    failed = daz_log.errors_before(name, datetime.datetime.fromtimestamp(os.path.getmtime(png)))
    if failed:
        print(f'skip {name}: DAZ failed to import {", ".join(failed)} for this render (MDL Import Error in the DAZ log): '
              'a material rendered as a fallback. Re-render it in a fresh DAZ session.')
        continue
    # The copy the render was made from: the latest one saved before the png
    # (a CPU run's copy beside the png outlives a later queue render).
    copies = [p for p in (png[:-4] + '.duf', os.path.join(calib, 'queue', 'done', name + '.duf'),
                          os.path.join(calib, 'queue-local', 'done', name + '.duf')) if os.path.exists(p)]
    before = [p for p in copies if os.path.getmtime(p) <= os.path.getmtime(png)]
    copy = max(before or copies, key=os.path.getmtime, default=None)
    if not copy:
        print(f'skip {name}: no capped scene copy to read the samples from')
        continue
    if duf_camera.missing(load(copy)):
        print(f'skip {name}: rendered from a copy without current_camera, so through DAZ\'s Perspective View, '
              'not the scene\'s camera (#97). Fix the scene (duf_camera.py --fix) and re-render it.')
        continue
    c = caps(copy)
    # iray-server.py submit (#195) leaves <png>.run.txt naming the server.
    run = png + '.run.txt'
    headless = os.path.exists(run) and 'Iray Server' in open(run).read()
    how = renderer or (SERVER if os.path.dirname(copy) == os.path.join(calib, 'queue', 'done') else HEADLESS if headless else CPU)
    samples = int(c.get('Max Samples', 0))
    old = rows.get(name)
    dst = os.path.join(refs, name + '.png')
    if old and int(old['samples'] or 0) > samples:
        print(f'skip {name}: {samples} samples, the kept one has {old["samples"]}')
        continue
    if os.path.exists(dst) and filecmp.cmp(png, dst, shallow=False) and old:
        continue
    shutil.copyfile(png, dst)
    rows[name] = {'scene': name, 'samples': samples, 'max_time': c.get('Max Time', ''), 'renderer': how,
                  'rendered': datetime.datetime.fromtimestamp(os.path.getmtime(png)).strftime('%Y-%m-%d %H:%M'),
                  'seconds': seconds(name, png), 'denoiser': denoiser(c, how, device(png))}
    if demo:
        rows[name]['scene_sha256'] = demo_lib.sha256(demo_lib.copy_path(name))
    changed += 1
    dev = device(png) if how in (HEADLESS, SERVER) else None
    print(f"kept {name}: {samples} samples, {how}" + (f', device {dev}' if dev else ''))
    if how in (HEADLESS, SERVER) and dev in (None, '?'):
        print(f'  {name}: no device recorded (#217, #219): check its render time against GPU siblings '
              '(a Bridge queue render: iray-server.py bridge-devices <queue.log>); '
              'a CPU fallback needs its denoiser set to off by hand')
with open(tsv, 'w', newline='') as f:
    w = csv.DictWriter(f, FIELDS, delimiter='\t', lineterminator='\n')
    w.writeheader()
    for k in sorted(rows):
        w.writerow(rows[k])
print(f'{changed} kept, {len(rows)} in renders.tsv')
