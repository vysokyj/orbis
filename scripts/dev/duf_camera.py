#!/usr/bin/env python3
"""ORBIS calibration: the camera a .duf scene renders through.

  python3 scripts/dev/duf_camera.py [--fix] <scene.duf>...

DAZ renders through the active viewport's camera. A scene saved while the
viewport looked through the Perspective View has no `scene.current_camera`,
and a DAZ that opens it keeps showing its own Perspective View, wherever
that was left: the Iray Server queue rendered such scenes from there, while
`orbis` exports the scene's camera (#97: Calib-NVIDIA2, -BumpGamma-NVIDIA,
-CastShadows-Off and -Shell-G9 got references from another view).

Prints every scene with a camera node but no current_camera and exits 1
(iray-queue.sh refuses such a batch). --fix sets current_camera to the
scene's only camera node in place (a scene with several is reported, not
guessed). The make-*.py generators call ensure() before they save.
"""
import gzip, json, sys


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def cameras(d):
    """Scene node ids that instance a camera from the node library."""
    lib = {n['id'] for n in d.get('node_library', []) if n.get('type') == 'camera'}
    out = []
    for n in d.get('scene', {}).get('nodes', []):
        url = n.get('url', '')
        if url.startswith('#') and url[1:].split('?')[0] in lib:
            out.append(n['id'])
    return out


def missing(d):
    """True when the scene has a camera but none is current."""
    return bool(cameras(d)) and not d.get('scene', {}).get('current_camera')


def ensure(d):
    """Make the scene's only camera current when none is; returns the
    current camera's id (None without a camera). Raises with several
    cameras and none current: which one is meant isn't for a script."""
    s = d.setdefault('scene', {})
    if s.get('current_camera') or not cameras(d):
        return s.get('current_camera')
    cams = cameras(d)
    if len(cams) > 1:
        raise ValueError(f'{len(cams)} cameras ({", ".join(cams)}), none current')
    s['current_camera'] = '#' + cams[0]
    return s['current_camera']


if __name__ == '__main__':
    args = sys.argv[1:]
    if not args or args[0] in ('-h', '--help'):
        print(__doc__)
        sys.exit(0)
    fix = '--fix' in args
    bad = 0
    for p in (a for a in args if a != '--fix'):
        d = load(p)
        if not missing(d):
            continue
        if not fix:
            print(f'{p}: no current_camera (DAZ would render its Perspective View, not {", ".join(cameras(d))})')
            bad += 1
            continue
        try:
            cam = ensure(d)
        except ValueError as e:
            print(f'{p}: {e}')
            bad += 1
            continue
        open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        print(f'{p}: current_camera {cam}')
    sys.exit(1 if bad else 0)
