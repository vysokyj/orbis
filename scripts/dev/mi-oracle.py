#!/usr/bin/env python3
"""What DAZ Studio hands Iray, from its .mi export (scripts/dev/daz-export-mi.dsa),
as JSON: render options, camera, environment, lights and every material's MDL
call with its arguments, textures and light profiles inlined (path, gamma;
IES path, flags, hermite degree, resolution). No geometry. These
are the oracle the DAZ -> Iray conversions in src/iray.rs are tested against
(#62): `calib/references/mi/<scene>.json`.

  python3 scripts/dev/mi-oracle.py <scene.mi>...                 # JSON on stdout
  python3 scripts/dev/mi-oracle.py --out calib/references/mi <scene.mi>...   # one file per scene

Names lose DAZ's per-export ids: materials are "<node label>/<surface>",
lights and the camera their node label. Texture paths are content-relative
("/Runtime/Textures/...") or "<daz>/..." under the DAZ Studio app.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import json, os, re, sys

ID = re.compile(r"(_[0-9a-f]+)?_\d+$")


def strip_id(name):
    name = name[3:] if name.startswith("DS_") else name
    return ID.sub("", name)


def value(tok):
    """A .mi argument / attribute value: numbers, a string, a bool."""
    tok = tok.strip().rstrip(",").strip()
    if tok in ("true", "on"):
        return True
    if tok in ("false", "off"):
        return False
    if tok.startswith('"') and tok.endswith('"') and tok.count('"') == 2:
        return tok[1:-1]
    parts = tok.split()
    try:
        nums = [float(p) for p in parts]
    except ValueError:
        return tok
    nums = [int(n) if n.is_integer() and "." not in p and "e" not in p.lower() else n for n, p in zip(nums, parts)]
    return nums[0] if len(nums) == 1 else nums


def content_path(path):
    """Content-relative: drop the content directory (anything up to
    /Runtime/ or /data/), or the DAZ Studio app bundle."""
    m = re.search(r"/(Runtime|data|Textures)/", path)
    app = re.search(r"DAZStudio\.app/Contents/(.*)$", path)
    if app:
        return "<daz>/" + app.group(1)
    return path[m.start():] if m else path


def blocks(text):
    """Top-level statements as (keyword, header line, body lines); geometry
    (`object`) and inline texture data are skipped."""
    lines = text.split("\n")
    i, out = 0, []
    while i < len(lines):
        line = lines[i]
        if not line or line.startswith(("#", "$", "'", "\t", " [", "[")) and not line.lstrip().startswith(("gamma", "compression", "color texture", "scalar texture")):
            i += 1
            continue
        head = line.split()[0] if line.split() else ""
        if head == "object":
            while i < len(lines) and lines[i] != "end object":
                i += 1
            i += 1
            continue
        if head in ("shader", "options", "camera", "instance", "light", "instgroup", "lightprofile"):
            body = []
            i += 1
            end = "end " + head
            while i < len(lines):
                l = lines[i]
                if l == end or (head == "shader" and not l.startswith("\t")):
                    break
                body.append(l)
                i += 1
            if head != "shader":
                i += 1
            out.append((head, line, body))
            continue
        if " texture " in " " + line.strip() + " ":
            out.append(("texture", line.strip(), []))
        elif head == "render":
            out.append(("render", line, []))
        i += 1
    return out


def parse(path):
    text = open(path, encoding="latin-1", newline="").read()
    textures, shaders, instances, lights, profiles = {}, {}, [], {}, {}
    options = camera = None
    for kind, head, body in blocks(text):
        if kind == "texture":
            m = re.match(r'(.*?)\b(color|scalar|vector) texture "([^"]+)"\s*"?([^"]*)"?', head)
            if m:
                flags = m.group(1).split()
                t = {"path": content_path(m.group(4)), "type": m.group(2)}
                for k in ("gamma", "compression"):
                    if k in flags:
                        t[k] = value(flags[flags.index(k) + 1])
                # Without a gamma flag DAZ leaves it to Iray: "g_Auto" in the name.
                g = re.search(r"\.g_([^.]+(?:\.\d+)?)\.c_", m.group(3))
                if "gamma" not in t and g:
                    t["gamma"] = value(g.group(1)) if g.group(1) != "Auto" else "auto"
                textures[m.group(3)] = t
        elif kind == "lightprofile":
            p = {}
            for l in body:
                k, _, v = l.strip().partition(" ")
                p[k] = content_path(value(v)) if k == "file" else value(v)
            profiles[re.match(r'lightprofile "([^"]+)"', head).group(1)] = p
        elif kind == "shader":
            name = re.match(r'shader "([^"]+)"', head).group(1)
            call, args, attrs = None, {}, {}
            for l in body:
                s = l.strip()
                a = re.match(r'attribute \S+ "([^"]+)" (.*)$', s)
                if a and call is None:
                    attrs[a.group(1)] = value(a.group(2))
                elif s.startswith('"mdl::'):
                    call = s.strip('"')
                elif s.startswith('="') or s.startswith('= "'):
                    call = "= " + s.split('"')[1]
                else:
                    a = re.match(r'"([^"]+)"\s*(=?)\s*(.*?),?$', s)
                    if a:
                        args[a.group(1)] = {"ref": a.group(3).strip('"')} if a.group(2) else value(a.group(3))
            shaders[name] = {"call": call, "args": args, "attrs": attrs}
        elif kind == "instance":
            m = re.match(r'instance "([^"]+)" "([^"]+)"', head)
            inst = {"name": m.group(1), "item": m.group(2), "material": [], "attrs": {}, "transform": None}
            for j, l in enumerate(body):
                s = l.strip()
                if s.startswith("material "):
                    inst["material"] = re.findall(r'"([^"]+)"', s)
                elif s == "transform":
                    inst["transform"] = [value(body[j + k]) for k in range(1, 5)]
                a = re.match(r'attribute \S+ "([^"]+)" (.*)$', s)
                if a:
                    inst["attrs"][a.group(1)] = value(a.group(2))
            instances.append(inst)
        elif kind == "light":
            name = re.match(r'light "([^"]+)"', head).group(1)
            l = {"shader": None, "fields": {}}
            for s in (x.strip() for x in body):
                if s.startswith("= "):
                    l["shader"] = s.split('"')[1]
                elif s.startswith("attribute"):
                    a = re.match(r'attribute \S+ "([^"]+)" (.*)$', s)
                    l["fields"][a.group(1)] = value(a.group(2))
                elif s:
                    k, _, v = s.partition(" ")
                    l["fields"][k] = value(v) if v else True
            lights[name] = l
        elif kind in ("options", "camera"):
            d = {}
            for s in (x.strip() for x in body):
                a = re.match(r'attribute \S+ "([^"]+)" (.*)$', s)
                if a:
                    d[a.group(1)] = value(a.group(2))
                elif s:
                    k, _, v = s.partition(" ")
                    d[k] = value(v) if v else True
            if kind == "options":
                options = d
            else:
                camera = d

    def tree(name, seen=()):
        """A shader as {"mdl", "args"}, references resolved, textures inlined."""
        if name in textures:
            return {"texture": textures[name]}
        s = shaders.get(name)
        if s is None or name in seen:
            return {"ref": strip_id(name)}
        call = s["call"] or ""
        if call.startswith("= "):
            return tree(call[2:], seen + (name,))
        out = {"mdl": re.sub(r"^mdl::", "", call)}
        args = {}
        for k, v in s["args"].items():
            if isinstance(v, dict):
                args[k] = tree(v["ref"], seen + (name,))
            elif isinstance(v, str) and v in textures:
                args[k] = {"texture": textures[v]}
            elif isinstance(v, str) and v in profiles:
                args[k] = {"light_profile": profiles[v]}
            else:
                args[k] = v
        out["args"] = args
        return out

    scene = {"options": {}, "camera": camera, "environment": None, "lights": {}, "materials": {}}
    if options:
        env = options.pop("environment_function", None)
        scene["options"] = options
        if env:
            scene["environment"] = tree(env)
    for inst in instances:
        handle = inst["attrs"].get("handle", strip_id(inst["name"]))
        if inst["item"] in lights:
            l = lights[inst["item"]]
            scene["lights"][handle] = {
                "fields": l["fields"],
                "material": tree(l["shader"]) if l["shader"] else None,
                "transform": inst["transform"],
            }
        for mat in inst["material"]:
            surface = strip_id(mat).split("__", 1)[-1]
            t = tree(mat)
            t["attrs"] = {k: v for k, v in shaders.get(mat, {}).get("attrs", {}).items() if k != "material_id"}
            scene["materials"][f"{handle}/{surface}"] = t
    return scene


def main(argv):
    out_dir = None
    if argv[:1] == ["--out"]:
        out_dir, argv = argv[1], argv[2:]
    for path in argv:
        scene = parse(path)
        text = json.dumps(scene, indent=1, sort_keys=True) + "\n"
        if out_dir:
            name = os.path.basename(path)[: -len(".mi")]
            with open(os.path.join(out_dir, name + ".json"), "w") as f:
                f.write(text)
        else:
            sys.stdout.write(text)


if __name__ == "__main__":
    main(sys.argv[1:])
