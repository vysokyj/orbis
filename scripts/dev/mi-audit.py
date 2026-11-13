#!/usr/bin/env python3
"""ORBIS: does a .mi fixture hold the shaders its scene asks for? (#153)

  python3 scripts/dev/mi-audit.py [<fixture.json | scene.mi>...]

DAZ's .mi exporter can silently hand Iray a default Iray Uber instead of
a scene's brick shader (PBRSkin, an NVIDIA preset, our own MDL modules):
daz-export-mi.dsa exported every second scene of a session that way while
the previous scene's Iray data was still being torn down (#153). Such a
fixture verifies src/iray.rs against the wrong material.

Per surface, the scene's .duf says which shader it uses (`extra` type
`studio/material/uber_iray` = Iray Uber, `studio/material/daz_brick` = a
shader brick: an MDL module); the fixture's material says what DAZ handed
Iray (`daz_3d::irayubermaterial::iray_uber_unified` for Uber, else the
brick's own call, wrapped in `daz_3d::basic::pass_material`). A brick
surface whose fixture shows Uber, or the reverse, is a mismatch - except
the bricks DAZ itself hands Iray as Uber (AoA Subsurface, a 3Delight
brick graph: `aoa_subsurface_as_uber` in src/iray.rs). Other shader types
(DzDefault, legacy) aren't checked.

Without arguments: every fixture in calib/references/mi. A .mi is read
through mi-oracle.py. The scene is calib/scenes/<name>.duf, else
<library>/Scenes/<name>.duf (ORBIS_LIBRARY, default "/Volumes/Data/DAZ
3D/Studio/My Library"); a fixture without a scene is reported as such.
Exit status 1 when any fixture disagrees with its scene. Used by
mi-oracle.py, which refuses to write such a fixture.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import glob, gzip, importlib.util, json, os, sys, urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
LIBRARY = os.environ.get('ORBIS_LIBRARY', '/Volumes/Data/DAZ 3D/Studio/My Library')
UBER = 'daz_3d::irayubermaterial::'  # iray_uber_unified, iray_uber
# Bricks without an MDL module of their own: DAZ hands Iray an Uber for them
# (the spec, "AoA Subsurface"; Calib-Mi-AoA ... -AoA5, -Legacy).
AS_UBER = ('#AoA_Subsurface',)


def scene_path(name):
    for p in (os.path.join(REPO, 'calib', 'scenes', name + '.duf'),
              os.path.join(LIBRARY, 'Scenes', name + '.duf')):
        if os.path.exists(p):
            return p
    return None


def load_duf(path):
    b = open(path, 'rb').read()
    return json.loads(gzip.decompress(b) if b[:2] == b'\x1f\x8b' else b)


def scene_shaders(duf):
    """{"<node label>/<surface>": ("uber" | "brick" | other type, url)}."""
    d = load_duf(duf)
    sc = d.get('scene', {})
    label = {}
    for n in sc.get('nodes', []):
        lab = n.get('label') or n.get('id')
        for g in n.get('geometries', []):
            label['#' + g.get('id', '')] = lab
    types = {}
    for m in d.get('material_library', []):
        for e in m.get('extra', []):
            if e.get('type', '').startswith('studio/material/'):
                types['#' + m['id']] = e['type']
    out = {}
    for m in sc.get('materials', []):
        t = None
        for e in m.get('extra', []):
            if e.get('type', '').startswith('studio/material/'):
                t = e['type']
        url = m.get('url', '')
        t = t or types.get(url)
        kind = {'studio/material/uber_iray': 'uber', 'studio/material/daz_brick': 'brick'}.get(t, t)
        if kind == 'brick' and url.endswith(AS_UBER):
            kind = 'uber'
        lab = label.get(m.get('geometry', ''))
        for g in m.get('groups', []):
            out['%s/%s' % (lab, g)] = (kind, urllib.parse.unquote(url))
    return out


def fixture_kind(mat):
    """'uber', 'brick' (a call into the brick's module), or 'empty': a
    pass_material without a call (#153's broken export: its `v` argument
    an inline literal, no module)."""
    mdl = mat.get('mdl', '')
    if mdl.startswith(UBER):
        return 'uber'
    calls = [v for v in mat.get('args', {}).values() if isinstance(v, dict) and v.get('mdl')]
    return 'brick' if calls or mdl != 'daz_3d::basic::pass_material' else 'empty'


def audit(name, fixture):
    """[problem strings] of one fixture (a dict as mi-oracle writes it)."""
    duf = scene_path(name)
    if not duf:
        return None
    want = scene_shaders(duf)
    bad = []
    for key, mat in sorted(fixture.get('materials', {}).items()):
        if key not in want:
            continue
        kind, url = want[key]
        if kind not in ('uber', 'brick'):
            continue
        got = fixture_kind(mat)
        if got != kind:
            bad.append('%s: scene %s (%s), fixture %s' % (key, kind, url, mat.get('mdl')))
    return bad


def oracle():
    spec = importlib.util.spec_from_file_location('mi_oracle', os.path.join(HERE, 'mi-oracle.py'))
    mod = importlib.util.module_from_spec(spec)
    sys.argv, argv = [sys.argv[0]], sys.argv
    try:
        spec.loader.exec_module(mod)
    finally:
        sys.argv = argv
    return mod


def main(args):
    paths = args or sorted(glob.glob(os.path.join(REPO, 'calib', 'references', 'mi', '*.json')))
    failed = noscene = checked = 0
    mo = None
    for p in paths:
        name = os.path.basename(p).rsplit('.', 1)[0]
        if p.endswith('.mi'):
            mo = mo or oracle()
            fixture = mo.parse(p) if hasattr(mo, 'parse') else None
            if fixture is None:
                sys.exit('mi-audit: mi-oracle.py has no parse()')
        else:
            fixture = json.load(open(p))
        bad = audit(name, fixture)
        if bad is None:
            noscene += 1
            print('%s: no scene' % name)
            continue
        checked += 1
        if bad:
            failed += 1
            print('%s: %d surface(s) disagree with the scene' % (name, len(bad)))
            for b in bad:
                print('  ' + b)
    print('%d checked, %d disagree, %d without a scene' % (checked, failed, noscene))
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
