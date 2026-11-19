#!/usr/bin/env python3
"""ORBIS: read and write DAZ scenes (.duf) the one way every copy in git takes (#208).

  python3 scripts/dev/duf_io.py check [<scene.duf | dir>...]
  python3 scripts/dev/duf_io.py fix [--library] <scene.duf | dir>...
  python3 scripts/dev/duf_io.py copy [--to <dir>] <library scene.duf>...

DAZ Studio writes the contributor details of its user (asset_info.contributor:
author, e-mail, website) into every scene it saves. No personal data goes into
the repo, so each scene a generator writes, and each copy into calib/scenes,
calib/scenes-store or demo/scenes, has them blanked; `save()` / `encode()` do
it, and write deterministic bytes (gzip mtime 0, JSON indent 1, as demo.py).
A scene DAZ saved outside the library (daz-build-instance-scenes.dsa saves
into the cache) keeps that absolute save path, under the user's home, in
asset_info.id; they set it library-relative, `/Scenes/<name>.duf`, as every
other generator does (#209).

- check: list scenes whose contributor is not blank or that hold a home path
  anywhere (names only, never the values); exit 1 if any. Default: calib/scenes, calib/scenes-store,
  demo/scenes. `cargo test` runs the same guard
  (committed_scenes_carry_no_contributor).
- fix: blank them (and set a home-path asset_info.id library-relative) in place. --library also rewrites the My Library/Scenes
  file of the same name when it is byte-identical to the scene before the fix
  (its mtime kept): `orbis export` finds the cached dump of a calib/scenes copy
  only through a byte-identical library file (dump.rs, find_dump, #159).
- copy: after a scene DAZ saved itself (daz-build-*.dsa): blank the library
  scene in place (mtime kept) and copy it into --to (default calib/scenes).
"""
import gzip, json, os, shutil, sys

repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
library = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')
KEYS = ('author', 'email', 'website')
DIRS = ('calib/scenes', 'calib/scenes-store', 'demo/scenes')


def load(path):
    raw = open(path, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def blank(d):
    """Blank the contributor details in place; True if any was set."""
    c = d.get('asset_info', {}).get('contributor') if isinstance(d, dict) else None
    if not isinstance(c, dict):
        return False
    had = any(c.get(k) for k in KEYS)
    for k in KEYS:
        if k in c:
            c[k] = ''
    return had


HOME = '/Users/'


def home_id(d):
    """asset_info.id when it is an absolute path under a home directory."""
    i = d.get('asset_info', {}).get('id') if isinstance(d, dict) else None
    return i if isinstance(i, str) and i.startswith(HOME) else None


def normalise_id(d):
    """A home-path asset_info.id -> `/Scenes/<name>.duf`; True if changed."""
    i = home_id(d)
    if i is None:
        return False
    d['asset_info']['id'] = '/Scenes/' + i.rsplit('/', 1)[-1]
    return True


def home_paths(d):
    """Number of strings anywhere in the scene that hold a home path."""
    if isinstance(d, str):
        return int(HOME in d)
    if isinstance(d, dict):
        return sum(home_paths(k) + home_paths(v) for k, v in d.items())
    if isinstance(d, list):
        return sum(home_paths(v) for v in d)
    return 0


def dirty(d):
    """What a committed scene may not carry: contributor details, home paths."""
    return has_contributor(d) or home_paths(d) > 0


def has_contributor(d):
    c = d.get('asset_info', {}).get('contributor') if isinstance(d, dict) else None
    return isinstance(c, dict) and any(c.get(k) for k in KEYS)


def encode(d, indent=1):
    """The scene as the gzip bytes that go into git: contributor blanked, id library-relative, deterministic."""
    blank(d)
    normalise_id(d)
    return gzip.compress(json.dumps(d, indent=indent).encode(), mtime=0)


def save(path, d, indent=1):
    data = encode(d, indent)
    with open(path, 'wb') as f:
        f.write(data)


def write_keep_mtime(path, data):
    st = os.stat(path)
    with open(path, 'wb') as f:
        f.write(data)
    os.utime(path, (st.st_atime, st.st_mtime))


def scenes(args):
    for a in args:
        if os.path.isdir(a):
            yield from sorted(os.path.join(a, f) for f in os.listdir(a) if f.endswith('.duf'))
        else:
            yield a


def main(argv):
    cmd = argv[0] if argv else ''
    if cmd == 'check':
        bad = 0
        for p in scenes(argv[1:] or [os.path.join(repo, d) for d in DIRS]):
            d = load(p)
            if has_contributor(d):
                print(f'{os.path.relpath(p, repo)}: contributor details not blank (duf_io.py fix)')
            n = home_paths(d)
            if n:
                fixable = ' (duf_io.py fix)' if n == 1 and home_id(d) else ' (asset_info.id is fixed by duf_io.py fix; the rest by hand)' if home_id(d) else ''
                print(f'{os.path.relpath(p, repo)}: {n} home path(s){fixable}')
            bad += dirty(d)
        print(f'{bad} scene(s) with contributor details or home paths')
        return 1 if bad else 0
    if cmd == 'fix':
        args = argv[1:]
        lib = '--library' in args
        args = [a for a in args if a != '--library']
        n = nlib = 0
        for p in scenes(args):
            old = open(p, 'rb').read()
            d = load(p)
            if not (has_contributor(d) or home_id(d)):
                continue
            data = encode(d)
            write_keep_mtime(p, data)
            n += 1
            lp = os.path.join(library, 'Scenes', os.path.basename(p))
            if lib and os.path.isfile(lp) and open(lp, 'rb').read() == old:
                write_keep_mtime(lp, data)
                nlib += 1
        print(f'fixed {n} scene(s)' + (f', {nlib} library copies with them' if lib else ''))
        return 0
    if cmd == 'copy':
        args = argv[1:]
        to = os.path.join(repo, 'calib', 'scenes')
        if '--to' in args:
            i = args.index('--to')
            to = args[i + 1]
            del args[i:i + 2]
        for p in args:
            data = encode(load(p))
            if open(p, 'rb').read() != data:
                write_keep_mtime(p, data)
            dst = os.path.join(to, os.path.basename(p))
            shutil.copyfile(p, dst)
            print(f'{p} -> {os.path.relpath(dst, repo)} (contributor blanked, id library-relative)')
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
