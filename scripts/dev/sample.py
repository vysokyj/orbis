#!/usr/bin/env python3
"""ORBIS Sample scenes (hand-made, store content, never in git): manifest and freshness.

  python3 scripts/dev/sample.py list
  python3 scripts/dev/sample.py add <Sample-Name> "<purpose>" [issue]
  python3 scripts/dev/sample.py check

The scene is read from <library>/Scenes/<name>.duf (ORBIS_LIBRARY, default
"/Volumes/Data/DAZ 3D/Studio/My Library"); it is never written. `check`
compares the sha256 of each scene with the one its Iray reference was
rendered from (sample/references/iray/renders.tsv): a re-saved scene makes
the reference stale, and nothing is overwritten silently.
"""
import csv, hashlib, os, re, sys, tomllib

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
manifest = os.path.join(repo, 'sample', 'scenes.toml')
tsv = os.path.join(repo, 'sample', 'references', 'iray', 'renders.tsv')
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


def status(name):
    p = scene_path(name)
    if not os.path.exists(p):
        return 'scene missing in the library'
    ref = refs().get(name)
    if not ref:
        return 'no reference yet'
    return 'fresh' if ref.get('scene_sha256') == sha256(p) else 'STALE (scene changed since the reference)'


def main(argv):
    cmd = argv[0] if argv else 'list'
    if cmd == 'list':
        for name, e in entries().items():
            print(f'{name}\t{status(name)}\t{e.get("purpose", "")}')
    elif cmd == 'check':
        bad = 0
        for name in entries():
            s = status(name)
            print(f'{name}: {s}')
            bad += s != 'fresh'
        sys.exit(1 if bad else 0)
    elif cmd == 'add' and len(argv) >= 3:
        name, purpose = argv[1], argv[2]
        if not re.fullmatch(r'Sample-[A-Za-z0-9_.-]+', name):
            sys.exit('the name must start with Sample-')
        if not os.path.exists(scene_path(name)):
            sys.exit(f'no such scene: {scene_path(name)}')
        if name in entries():
            sys.exit(f'{name} is already in the manifest')
        issue = f'issue = {int(argv[3])}\n' if len(argv) > 3 else ''
        with open(manifest, 'a') as f:
            f.write(f'\n[{name}]\npurpose = "{purpose.replace(chr(34), chr(39))}"\n{issue}')
        print(f'added {name}')
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])
