#!/usr/bin/env python3
"""Iray references for calibration scenes, one after another (DAZ takes the
whole CPU — never alongside an Octane probe):

  python3 scripts/dev/calib-render.py [--auto-ev] [--samples N] <scene.duf>...

Each scene renders through scripts/dev/daz-render.sh to
~/Library/Caches/orbis/calib/iray/<scene>.png, capped by samples (Iray is
then deterministic). With --auto-ev a 40-sample test render first sets the
scene's Exposure Value (written into the .duf, which the ORBIS export reads
too) so the 99th percentile sits at 85 % of white: nothing clips, nothing
lives in the bottom few levels.
"""
import gzip, json, math, os, subprocess, sys
import numpy as np
from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
out = os.path.expanduser('~/Library/Caches/orbis/calib/iray')
os.makedirs(out, exist_ok=True)
args = sys.argv[1:]
auto = '--auto-ev' in args
samples = 400
if '--samples' in args:
    samples = int(args[args.index('--samples') + 1])
scenes = [a for a in args if a.endswith('.duf')]


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def tone_channels(d):
    l = next(l for l in d['node_library'] if l['id'] == 'Tonemapper Options')
    return next(e for e in l['extra'] if e['type'] == 'studio_node_channels')['channels']


def get_ev(p):
    return next(c['channel'].get('current_value', c['channel'].get('value')) for c in tone_channels(load(p))
                if c['channel']['id'] == 'Exposure Value')


def set_ev(p, ev):
    """EV through the shutter, as DAZ does: Iray exposes by shutter, f-number
    and ISO; the EV channel is only derived from them."""
    d = load(p)
    ch = {c['channel']['id']: c['channel'] for c in tone_channels(d)}
    n = ch['Aperture'].get('current_value', 8.0)
    iso = ch['Film ISO'].get('current_value', 100.0)
    ch['Shutter Speed']['current_value'] = 2 ** ev / (n * n) * (iso / 100.0)
    ch['Exposure Value']['current_value'] = ev
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))


def render(scene, png, n):
    r = subprocess.run([os.path.join(here, 'daz-render.sh'), scene, png, '3600', str(n)],
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    open(png + '.run.txt', 'w').write(r.stdout)
    return r.returncode == 0


for scene in scenes:
    name = os.path.splitext(os.path.basename(scene))[0]
    png = os.path.join(out, name + '.png')
    if auto:
        test = os.path.join(out, name + '.test.png')
        for _ in range(4):
            ev = get_ev(scene)
            if not render(scene, test, 40):
                break
            a = np.asarray(Image.open(test).convert('RGB')).astype(float) / 255
            p99 = np.percentile(a.max(-1), 99)
            if p99 >= 0.995:          # clipped: no measure, go 3 stops darker
                set_ev(scene, ev + 3)
                continue
            lin, target = max(p99, 0.02) ** 2.2, 0.85 ** 2.2
            new = round(ev + math.log2(lin / target), 1)
            set_ev(scene, new)
            print(f'{name}: EV {ev} → {new} (p99 {p99:.2f})', flush=True)
            break
        for f in (test, test + '.run.txt', test + '.log'):
            if os.path.exists(f):
                os.remove(f)
    # DAZ Studio now and then crashes on its own mid-render (a segfault
    # closing a dialog, Calib-Skin-Monique-hdri): try again, twice at most.
    for attempt in range(3):
        ok = render(scene, png, samples)
        if ok:
            break
        print(f'{name}: DAZ Studio failed, attempt {attempt + 1}', flush=True)
    print(f'{name}: {"ok" if ok else "FAILED"} (EV {get_ev(scene)})', flush=True)
