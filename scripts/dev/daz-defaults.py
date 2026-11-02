#!/usr/bin/env python3
"""A DAZ material definition's channel default values (id -> value), as
JSON: the facts src/iray.rs's tests need from DAZ's own definitions
(e.g. PBRSkin.dsf) without the file itself, which isn't ours to ship.

  python3 scripts/dev/daz-defaults.py <definition.dsf> > calib/references/daz-defaults/<Name>.json
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import gzip, json, sys

raw = open(sys.argv[1], "rb").read()
doc = json.loads(gzip.decompress(raw) if raw[:2] == b"\x1f\x8b" else raw)
mat = doc["material_library"][0]
out = {}
for v in mat.values():
    if isinstance(v, dict) and "channel" in v:
        c = v["channel"]
        out[c["id"]] = c.get("current_value", c.get("value"))
for e in mat.get("extra", []):
    for c in e.get("channels", []):
        c = c["channel"]
        out[c["id"]] = c.get("current_value", c.get("value"))
json.dump(out, sys.stdout, indent=1, sort_keys=True)
sys.stdout.write("\n")
