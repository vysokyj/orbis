#!/usr/bin/env python3
"""ORBIS Demo scenes (hand-made, store content, never in git): manifest and freshness.

  python3 scripts/dev/demo.py list
  python3 scripts/dev/demo.py add <Demo-Name> "<purpose>" [issue]
  python3 scripts/dev/demo.py check
  python3 scripts/dev/demo.py sync

The scene is read from <library>/Scenes/<name>.duf (ORBIS_LIBRARY, default
"/Volumes/Data/DAZ 3D/Studio/My Library"); it is never written. `add` and
`sync` write a copy to demo/scenes/ (git holds the source too) with the
contributor's author / e-mail / website blanked - no personal data in the repo. `check`
compares the sha256 of each scene with the one its Iray reference was
rendered from (demo/references/iray/renders.tsv): a re-saved scene makes
the reference stale, and nothing is overwritten silently.
"""
import csv, gzip, hashlib, json, os, re, sys, tomllib

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
manifest = os.path.join(repo, 'demo', 'scenes.toml')
tsv = os.path.join(repo, 'demo', 'references', 'iray', 'renders.tsv')
library = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')


def scene_path(name):
    return os.path.join(library, 'Scenes', name + '.duf')


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def entries():
    return tomllib.load(open(manifest, 'rb')) if os.path.exists(manifest) else {}


def refs():
    return {r['scene']: r for r in csv.DictReader(open(tsv), delimiter='\t')} if os.path.exists(tsv) else {}


def copy_path(name):
    return os.path.join(repo, 'demo', 'scenes', name + '.duf')


def sanitized(name):
    """The library scene as gzip bytes, contributor details blanked, deterministic."""
    raw = open(scene_path(name), 'rb').read()
    d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
    c = d.get('asset_info', {}).get('contributor')
    if isinstance(c, dict):
        for k in ('author', 'email', 'website'):
            if k in c:
                c[k] = ''
    return gzip.compress(json.dumps(d, indent=1).encode(), mtime=0)


def sync(name):
    os.makedirs(os.path.dirname(copy_path(name)), exist_ok=True)
    data = sanitized(name)
    if not os.path.exists(copy_path(name)) or open(copy_path(name), 'rb').read() != data:
        open(copy_path(name), 'wb').write(data)
        print(f'synced {name} -> demo/scenes ({len(data) // 1024} KiB)')


def status(name):
    p = scene_path(name)
    if not os.path.exists(p):
        return 'scene missing in the library'
    cp = copy_path(name)
    if not os.path.exists(cp) or open(cp, 'rb').read() != sanitized(name):
        return 'repo copy outdated (demo.py sync)'
    ref = refs().get(name)
    if not ref:
        return 'no reference yet'
    return 'fresh' if ref.get('scene_sha256') == sha256(p) else 'STALE (scene changed since the reference)'


def main(argv):
    cmd = argv[0] if argv else 'list'
    if cmd == 'list':
        for name, e in entries().items():
            print(f'{name}\t{status(name)}\t{e.get("purpose", "")}')
    elif cmd == 'sync':
        for name in entries():
            if os.path.exists(scene_path(name)):
                sync(name)
    elif cmd == 'check':
        bad = 0
        for name in entries():
            s = status(name)
            print(f'{name}: {s}')
            bad += s != 'fresh'
        sys.exit(1 if bad else 0)
    elif cmd == 'add' and len(argv) >= 3:
        name, purpose = argv[1], argv[2]
        if not re.fullmatch(r'Demo-[A-Za-z0-9_.-]+', name):
            sys.exit('the name must start with Demo-')
        if not os.path.exists(scene_path(name)):
            sys.exit(f'no such scene: {scene_path(name)}')
        if name in entries():
            sys.exit(f'{name} is already in the manifest')
        issue = f'issue = {int(argv[3])}\n' if len(argv) > 3 else ''
        with open(manifest, 'a') as f:
            f.write(f'\n[{name}]\npurpose = "{purpose.replace(chr(34), chr(39))}"\n{issue}')
        sync(name)
        print(f'added {name}')
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])
