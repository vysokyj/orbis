#!/usr/bin/env python3
"""ORBIS Demo scenes (hand-made, store content, never in git): manifest and freshness.

  python3 scripts/dev/demo.py list
  python3 scripts/dev/demo.py add <Demo-Name> "<purpose>" [issue]
  python3 scripts/dev/demo.py check
  python3 scripts/dev/demo.py restore [<name>]
  python3 scripts/dev/demo.py update [<name>]

The git copy demo/scenes/<name>.duf is the master (contributor author / e-mail
/ website blanked - no personal data in the repo). The library file
<library>/Scenes/<name>.duf (ORBIS_LIBRARY, default "/Volumes/Data/DAZ 3D/
Studio/My Library") is only used by DAZ.
- restore: put the git copy back into the library (the user's rule: git wins
  unless they say "update"); a differing library file is backed up first to
  ~/Library/Caches/orbis/demo-backup/, its mtime is kept.
- update: only when the user asks - take the library file into git (sanitised). `check`
compares the sha256 of each scene with the one its Iray reference was
rendered from (demo/references/iray/renders.tsv): a re-saved scene makes
the reference stale, and nothing is overwritten silently.
"""
import csv, gzip, hashlib, json, os, re, shutil, sys, time, tomllib

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


def has_contributor(path):
    raw = open(path, 'rb').read()
    d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
    c = d.get('asset_info', {}).get('contributor') or {}
    return any(c.get(k) for k in ('author', 'email', 'website'))


def restore(name):
    cp, lp = copy_path(name), scene_path(name)
    if not os.path.exists(cp):
        sys.exit(f'{name}: no git copy (demo.py add)')
    if os.path.exists(lp):
        same = sanitized(name) == open(cp, 'rb').read()
        if same and not has_contributor(lp):
            return
        if same:  # only the contributor details differ: nothing to back up
            mtime = os.path.getmtime(lp)
            shutil.copyfile(cp, lp)
            os.utime(lp, (mtime, mtime))
            print(f'restored {name} from git (contributor details removed from the library file)')
            return
        backup = os.path.expanduser('~/Library/Caches/orbis/demo-backup')
        os.makedirs(backup, exist_ok=True)
        shutil.copyfile(lp, os.path.join(backup, f'{name}-{time.strftime("%Y%m%d-%H%M%S")}.duf'))
        mtime = os.path.getmtime(lp)
    else:
        mtime = None
    shutil.copyfile(cp, lp)
    if mtime:
        os.utime(lp, (mtime, mtime))
    print(f'restored {name} from git (the library file differed; old one in demo-backup/)' if mtime else f'restored {name} from git')


def status(name):
    cp = copy_path(name)
    if not os.path.exists(cp):
        return 'no git copy (demo.py add)'
    p = scene_path(name)
    if not os.path.exists(p):
        return 'library file missing (demo.py restore)'
    ref = refs().get(name)
    differs = sanitized(name) != open(cp, 'rb').read()
    if differs:
        return 'library differs from git: git wins (demo.py restore) unless the user says update'
    if not ref:
        return 'no reference yet'
    return 'fresh' if ref.get('scene_sha256') == sha256(cp) else 'STALE (the git copy changed since the reference)'


def main(argv):
    cmd = argv[0] if argv else 'list'
    if cmd == 'list':
        for name, e in entries().items():
            print(f'{name}\t{status(name)}\t{e.get("purpose", "")}')
    elif cmd in ('update', 'sync'):
        for name in (argv[1:2] or entries()):
            if os.path.exists(scene_path(name)):
                sync(name)
    elif cmd == 'restore':
        for name in (argv[1:2] or entries()):
            restore(name)
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
