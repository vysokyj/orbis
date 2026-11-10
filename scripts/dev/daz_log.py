#!/usr/bin/env python3
"""ORBIS: MDL import failures in DAZ Studio's logs, per scene load.

  python3 scripts/dev/daz_log.py [<scene name>...]

DAZ Studio imports an MDL module into Iray's database once per session. A
later scene that uses the same NVIDIA preset (concrete, porcelain, ...)
in the same session logs "MDL Import Error (4004): The module ... was not
imported because at least one of the required DB elements is already in
use", and Iray renders that material as a plain fallback (#94: the
queue's Calib-NVIDIA-BumpG1 after Calib-BumpGamma-NVIDIA, concrete
untextured, 1.9 x brighter). Such a render is no reference.

Reads every instance's log (Studio6, Studio6 [<instance>]); prints each
scene load that logged an MDL import error, or only those of the given
scenes. Used by keep-refs.py (refuses the render) and status.py.
"""
import datetime, glob, os, re, sys

LOGS = os.path.expanduser('~/Library/Application Support/DAZ 3D/Studio6*/log.txt')
LOAD = re.compile(r'Begin asset load \(open\): (.*\.duf)\s*$')
ERROR = re.compile(r'MDL Import Error \(\d+\) : The module "([^"]+)"')
STAMP = re.compile(r'^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)')


def failed_loads(logs=None):
    """[(time, log, scene name, [modules])] for every scene load that
    logged an MDL import error, oldest first per log."""
    out = []
    for log in logs or sorted(glob.glob(LOGS)):
        cur = None
        for line in open(log, errors='replace'):
            m = LOAD.search(line)
            if m:
                if cur and cur[3]:
                    out.append(cur)
                t = STAMP.match(line)
                when = datetime.datetime.strptime(t.group(1), '%Y-%m-%d %H:%M:%S') if t else None
                cur = (when, log, os.path.basename(m.group(1))[:-4], [])
                continue
            m = ERROR.search(line)
            if m and cur and m.group(1) not in cur[3]:
                cur[3].append(m.group(1))
        if cur and cur[3]:
            out.append(cur)
    return out


def loads(name, logs=None):
    """Times of every load of <name>.duf, with its MDL import errors:
    [(time, [modules])]."""
    out = []
    for log in logs or sorted(glob.glob(LOGS)):
        cur = None
        for line in open(log, errors='replace'):
            m = LOAD.search(line)
            if m:
                if cur:
                    out.append(cur)
                cur = None
                if os.path.basename(m.group(1)) == name + '.duf':
                    t = STAMP.match(line)
                    cur = (datetime.datetime.strptime(t.group(1), '%Y-%m-%d %H:%M:%S') if t else None, [])
                continue
            m = ERROR.search(line)
            if m and cur is not None and m.group(1) not in cur[1]:
                cur[1].append(m.group(1))
        if cur:
            out.append(cur)
    return out


def errors_before(name, when):
    """MDL modules that failed in the last load of <name>.duf before
    `when` (a datetime) in any DAZ log; [] if none or no load found."""
    hits = [l for l in loads(name) if l[0] and l[0] <= when]
    return max(hits, key=lambda l: l[0])[1] if hits else []


if __name__ == '__main__':
    if any(a in ('-h', '--help') for a in sys.argv[1:]):
        print(__doc__)
        sys.exit(0)
    want = set(sys.argv[1:])
    for when, log, scene, mods in failed_loads():
        if not want or scene in want:
            inst = os.path.basename(os.path.dirname(log))
            print(f'{when}  {inst}  {scene}: {", ".join(mods)}')
