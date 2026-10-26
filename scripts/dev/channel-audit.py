#!/usr/bin/env python3
"""Which DAZ channels do scenes set, and which does the export read?

  python3 scripts/dev/channel-audit.py [<scenes dir>...] > audit.md

Walks every .duf: cameras, lights, the Tonemapper / Environment / render
settings, and materials (scene materials over their library definitions).
A channel counts as *set* in a scene when its current value differs from
its default (DSON `value`). Handled = its id appears as a string literal in
src/ (what the export can read). The report lists, per kind of owner, the
channels set in any scene: in how many scenes, an example value, handled
or not — unhandled ones first.
"""
import collections, glob, gzip, json, os, re, sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../..')
dirs = sys.argv[1:] or ['/Volumes/Data/DAZ 3D/Studio/My Library/Scenes']
handled = set()
for f in glob.glob(os.path.join(root, 'src/**/*.rs'), recursive=True):
    handled |= set(re.findall(r'"([A-Za-z][A-Za-z0-9 /_\-()]+)"', open(f).read()))

# owner kind → channel → [scenes, example value]
seen = collections.defaultdict(lambda: collections.defaultdict(lambda: [set(), None]))
scenes = 0
render_values = collections.defaultdict(lambda: collections.defaultdict(set))


def kind_of(entry):
    t = entry.get('type')
    if t == 'camera':
        return 'camera'
    if t == 'light':
        return 'light'
    i = entry.get('id', '')
    for k in ('Tonemapper Options', 'Environment Options'):
        if i.startswith(k):
            return k
    return None


def channels(entry):
    for c in entry.get('channels', []) or []:
        if isinstance(c.get('channel'), dict):
            yield c['channel']
    for e in entry.get('extra', []) or []:
        for c in e.get('channels', []) or []:
            ch = c.get('channel')
            if ch:
                yield ch
    for k, v in entry.items():
        if isinstance(v, dict) and isinstance(v.get('channel'), dict):
            yield v['channel']


def is_set(ch):
    cur = ch.get('current_value')
    if cur is None and not ch.get('image_file'):
        return False
    return cur != ch.get('value') or bool(ch.get('image_file'))


for d in dirs:
    for p in sorted(glob.glob(os.path.join(d, '**/*.duf'), recursive=True)):
        try:
            raw = open(p, 'rb').read()
            doc = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
        except Exception:
            continue
        if 'scene' not in doc:
            continue
        scenes += 1
        name = os.path.basename(p)
        libs = {l.get('id'): l for l in doc.get('node_library', [])}
        for n in doc['scene'].get('nodes', []):
            lid = n.get('url', '').split('#')[-1].replace('%20', ' ').replace('%28', '(').replace('%29', ')')
            entry = libs.get(lid)
            if not entry:
                continue
            k = kind_of(entry)
            if not k:
                continue
            merged = {c['id']: c for c in channels(entry)}
            merged.update({c['id']: c for c in channels(n)})
            for ch in merged.values():
                if is_set(ch):
                    s = seen[k][ch['id']]
                    s[0].add(name)
                    s[1] = s[1] if s[1] is not None else (ch.get('image_file') or ch.get('current_value'))
        for e in doc['scene'].get('extra', []):
            if e.get('type') == 'studio_render_settings':
                # No defaults saved here: record every value, report the
                # channels whose value differs between scenes.
                for e2 in e.get('render_options', {}).get('render_elements', []) or []:
                    for ch in channels(e2):
                        render_values[ch['id']][json.dumps(ch.get('current_value'))].add(name)
        mlib = {m.get('id'): m for m in doc.get('material_library', [])}
        for m in doc['scene'].get('materials', []):
            base = mlib.get(m.get('url', '').split('#')[-1])
            shader = 'material: ' + (m.get('url', '').split('#')[-1] if not m.get('url', '').startswith('#') else
                                     next((e['type'].split('/')[-1] for e in (base or {}).get('extra', []) if e.get('type', '').startswith('studio/material')), '?'))
            merged = {c['id']: c for c in channels(base or {})}
            merged.update({c['id']: c for c in channels(m)})
            for ch in merged.values():
                if is_set(ch):
                    s = seen[shader][ch['id']]
                    s[0].add(name)
                    s[1] = s[1] if s[1] is not None else (ch.get('image_file') or ch.get('current_value'))

for cid, vals in render_values.items():
    if len(vals) > 1:
        common = max(vals.values(), key=len)
        for v, sc in vals.items():
            if sc is not common:
                s = seen['render settings'][cid]
                s[0] |= sc
                s[1] = s[1] if s[1] is not None else json.loads(v)
print(f'# DAZ channels set in {scenes} scenes vs. read by the export\n')
for k in sorted(seen, key=lambda k: -sum(len(v[0]) for v in seen[k].values())):
    rows = sorted(seen[k].items(), key=lambda kv: (kv[0] in handled, -len(kv[1][0]), kv[0]))
    print(f'## {k}\n\n| channel | scenes | example | read |\n|---|---|---|---|')
    for cid, (sc, ex) in rows:
        ex = str(ex)
        if len(ex) > 50:
            ex = '…' + ex[-47:]
        print(f'| {cid} | {len(sc)} | {ex} | {"yes" if cid in handled else "**no**"} |')
    print()
