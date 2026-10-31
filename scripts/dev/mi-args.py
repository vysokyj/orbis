#!/usr/bin/env python3
"""The MDL arguments DAZ Studio gives each Iray Uber material, from a .mi
export (scripts/dev/daz-export-mi.dsa): the compiled channel -> parameter
mapping, observed (docs/daz/iray-uber-mdl.md).

  python3 scripts/dev/mi-args.py <scene.mi>...          # a table per scene
  python3 scripts/dev/mi-args.py --param metallic <scene.mi>...   # one column per matching parameter

Only arguments that differ from the MDL defaults are in the file; a
missing one is at its default. Textures show as the texture's name.
"""
import re, sys


def materials(path):
    text = open(path, encoding="latin-1").read()
    out = {}
    for m in re.finditer(r'^shader "DS_([^"]+)"\n(?:\t[^\n]*\n)*?\t"mdl::daz_3d::irayubermaterial::iray_uber[^"]*"\n\t\(\n(.*?)\n\t\)', text, re.M | re.S):
        name = re.sub(r"__Default_[0-9a-f]+_\d+$", "", m.group(1))
        args = {}
        for line in m.group(2).split("\n"):
            a = re.match(r'\t\t"([^"]+)"\s*(=?)\s*(.*?),?$', line.strip("\n"))
            if a:
                args[a.group(1)] = a.group(3).strip('"') if not a.group(2) else "-> " + a.group(3).strip('"')
        out[name] = args
    return out


argv = sys.argv[1:]
pattern = None
if argv[:1] == ["--param"]:
    pattern, argv = argv[1], argv[2:]
for path in argv:
    mats = materials(path)
    print(f"## {path}\n")
    if pattern:
        keys = sorted({k for a in mats.values() for k in a if re.search(pattern, k)})
        print("| material | " + " | ".join(keys) + " |")
        print("|---" * (len(keys) + 1) + "|")
        for name, a in mats.items():
            print(f"| {name} | " + " | ".join(a.get(k, "") for k in keys) + " |")
    else:
        for name, a in mats.items():
            print(f"### {name}")
            for k, v in a.items():
                print(f"- {k} = {v}")
    print()
