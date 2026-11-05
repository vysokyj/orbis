#!/usr/bin/env python3
"""Which DAZ channels do scenes set, and which does the export read?

  python3 scripts/dev/channel-audit.py [<scenes dir>...] > audit.md

Walks every .duf: cameras, lights, the Tonemapper / Environment / render
settings, and materials (scene materials over their library definitions).
A channel counts as *set* in a scene when its current value differs from
its default (DSON `value`). Read = its id appears as a string literal in
src/, or matches a `format!` template there ("Lens Distortion K{i}"). An
unread channel listed in scripts/dev/channel-audit-known.tsv shows its
reason (spec section / issue) instead of **no**. The report lists, per kind
of owner, the channels set in any scene: in how many scenes, an example
value, read or why not — unexplained ones first — and ends with the
unexplained ones.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import collections, glob, gzip, json, os, re, sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../..')
dirs = sys.argv[1:] or ['/Volumes/Data/DAZ 3D/Studio/My Library/Scenes']
handled = set()
templates = []
for f in glob.glob(os.path.join(root, 'src/**/*.rs'), recursive=True):
    text = open(f).read()
    handled |= set(re.findall(r'"([A-Za-z][A-Za-z0-9 /_\-()]+)"', text))
    # Ids built with format! ("Lens Distortion K{i}", "Dome Orientation
    # {a}"): a template that starts with a capitalised word, each
    # placeholder standing for one word.
    for t in re.findall(r'format!\(\s*"([A-Z][A-Za-z ]{3,}\{[^"]*)"', text):
        if re.fullmatch(r'[A-Za-z0-9 ]*(\{[^{}]*\}[A-Za-z0-9 ]*)+', t):
            templates.append(re.compile(''.join(re.escape(p) if not p.startswith('{') else '[A-Za-z0-9]+'
                                                for p in re.split(r'(\{[^{}]*\})', t))))


def is_read(cid):
    return cid in handled or any(t.fullmatch(cid) for t in templates)


# Known unread channels, each with its reason (the spec section or issue):
# owner-kind regex, channel regex, reason (scripts/dev/channel-audit-known.tsv).
known = []
for line in open(os.path.join(root, 'scripts/dev/channel-audit-known.tsv')):
    if line.strip() and not line.startswith('#'):
        o, c, why = line.rstrip('\n').split('\t')
        known.append((re.compile(o), re.compile(c), why))


def why_unread(kind, cid):
    return next((why for o, c, why in known if o.fullmatch(kind) and c.fullmatch(cid)), None)

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


def merge(merged, entry):
    """Overlay an entry's channels field by field: a scene keeps only what
    it changed (`current_value`), the default (`value`) stays the
    definition's."""
    for c in channels(entry):
        merged.setdefault(c['id'], {}).update(c)


CONTENT = [os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library'), '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library']
_definitions = {}


def definition(url):
    """The material a library .dsf defines (`/data/…/X.dsf#X`), or None."""
    path = url.split('#')[0].replace('%20', ' ').replace('%28', '(').replace('%29', ')')
    if not path.endswith('.dsf'):
        return None
    if path not in _definitions:
        _definitions[path] = None
        for c in CONTENT:
            p = os.path.join(c, path.lstrip('/'))
            if os.path.exists(p):
                try:
                    raw = open(p, 'rb').read()
                    lib = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw).get('material_library') or [None]
                    _definitions[path] = lib[0]
                except Exception:
                    pass
                break
    return _definitions[path]


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
            merged = {c['id']: dict(c) for c in channels(entry)}
            merge(merged, n)
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
            url = m.get('url', '')
            base = mlib.get(url.split('#')[-1])
            shader = 'material: ' + (url.split('#')[-1] if not url.startswith('#') else
                                     next((e['type'].split('/')[-1] for e in (base or {}).get('extra', []) if e.get('type', '').startswith('studio/material')), '?'))
            # A shader from a library file: its definition holds the
            # defaults the scene leaves out.
            merged = {c['id']: dict(c) for c in channels(definition(url) or {})}
            merge(merged, base or {})
            merge(merged, m)
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
unexplained = []
for k in sorted(seen, key=lambda k: -sum(len(v[0]) for v in seen[k].values())):
    state = {cid: 'yes' if is_read(cid) else why_unread(k, cid) or '**no**' for cid in seen[k]}
    rows = sorted(seen[k].items(), key=lambda kv: (state[kv[0]] != '**no**', state[kv[0]] == 'yes', -len(kv[1][0]), kv[0]))
    print(f'## {k}\n\n| channel | scenes | example | read |\n|---|---|---|---|')
    for cid, (sc, ex) in rows:
        ex = str(ex)
        if len(ex) > 50:
            ex = '…' + ex[-47:]
        print(f'| {cid} | {len(sc)} | {ex} | {state[cid]} |')
        if state[cid] == '**no**':
            unexplained.append((k, cid, len(sc), sorted(sc)[:3]))
    print()
print(f'## Unexplained: {len(unexplained)}\n')
for k, cid, n, sc in unexplained:
    print(f'- {k}: {cid} ({n} scenes: {", ".join(sc)})')
