#!/usr/bin/env python3
"""What DAZ Studio hands Iray, from its .mi export (scripts/dev/daz-export-mi.dsa),
as JSON: render options, camera, environment, lights, mesh instances' render
flags (visible, shadow, label, matte...) and every material's MDL
call with its arguments, textures and light profiles inlined (path, gamma;
IES path, flags, hermite degree, resolution). Of the geometry only
`geometry_min_y`: the lowest world y (cm) of every exported mesh and hair
(control points) - DAZ's auto Iray ground sits there (#119). These
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


def geometry_min_y(path):
    """The lowest world y (cm) over the exported objects: `trilist` vertices
    (big-endian float32 after "scalar `", or a text list) and `hair` strands
    (cubic B-spline, x y z [radius] per control point), each placed by its instance's transform
    (world -> object, row vectors: world = (p - t) R^T). None without any."""
    import struct
    b = open(path, "rb").read()
    objects, i = {}, 0
    while True:
        i = b.find(b"\nobject \"", i)
        if i < 0:
            break
        name = b[i + 9:b.find(b'"', i + 9)].decode("latin-1")
        e = b.find(b"\nend object", i)
        hair = re.search(rb"\n\s*hair\n.*?vertex( radius)?.*?\n\s*scalar \[ (\d+) \]\n\s*", b[i:i + 400], re.S)
        if hair:
            # Cubic B-spline fibres: DAZ's strand vertices are the curve's
            # points at the knots, (P[k-1] + 4 P[k] + P[k+1]) / 6 (its end
            # control points lie beyond the strand). Strands start at the
            # float offsets of the `hair [ n ]` list, the last one its end.
            per, n = 4 if hair.group(1) else 3, int(hair.group(2))
            s = i + hair.end()
            def nums(s, n, f):
                if b[s:s + 8] == b"binary `":
                    return struct.unpack(f">{n}{f}", b[s + 8:s + 8 + n * 4])
                return [float(x) for x in b[s:b.find(b"\n", s)].split()]
            flat = nums(s, n, "f")
            h = re.compile(rb"hair \[ (\d+) \]\n\s*").search(b, s, e)
            offs = [int(x) for x in nums(h.end(), int(h.group(1)), "i")] if h else [0, n]
            pts = []
            for a, z in zip(offs, offs[1:]):
                cp = [flat[k:k + 3] for k in range(a, z, per)]
                pts += [[(cp[k - 1][j] + 4 * cp[k][j] + cp[k + 1][j]) / 6 for j in range(3)] for k in range(1, len(cp) - 1)] or cp
            objects[name] = pts
        else:
            hdr = re.search(rb"vertex (\d+) p( n)?((?: t \d+)*)", b[i:i + 400])
            if not hdr:
                i = e
                continue
            n = int(hdr.group(1))
            per = 3 + (3 if hdr.group(2) else 0) + sum(int(x) for x in re.findall(rb"t (\d+)", hdr.group(3)))
            s = b.find(b"scalar `", i)
            if s < 0 or s > e:
                flat = [float(x) for x in b[b.find(b"[", i) + 1:b.find(b"]", i)].split()]
            else:
                s += len(b"scalar `")
                flat = struct.unpack(f">{n * per}f", b[s:s + n * per * 4])
            objects[name] = [flat[k * per:k * per + 3] for k in range(n)]
        i = e
    low = None
    for m in re.finditer(rb'\ninstance "[^"]+" "([^"]+)"\n(.*?)\nend instance', b, re.S):
        pts = objects.get(m.group(1).decode("latin-1"))
        t = re.search(r"transform\n((?:\s+[-\d.e+ ]+\n){4})", m.group(2).decode("latin-1") + "\n")
        if not pts or not t:
            continue
        T = [[float(x) for x in r.split()] for r in t.group(1).strip().split("\n")]
        R, o = [r[:3] for r in T[:3]], T[3][:3]
        y = min(sum((p[j] - o[j]) * R[1][j] for j in range(3)) for p in pts)
        low = y if low is None else min(low, y)
    return None if low is None else round(low, 4)


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
                    if a and not a.group(2) and a.group(3).startswith("[") and '= "' in a.group(3):
                        # An array of references (`df::clamped_mix`'s components).
                        args[a.group(1)] = {"refs": re.findall(r'= "([^"]+)"', a.group(3))}
                    elif a:
                        args[a.group(1)] = {"ref": a.group(3).strip('"')} if a.group(2) else value(a.group(3))
            shaders[name] = {"call": call, "args": args, "attrs": attrs}
        elif kind == "instance":
            m = re.match(r'instance "([^"]+)" "([^"]+)"', head)
            inst = {"name": m.group(1), "item": m.group(2), "material": [], "attrs": {}, "flags": {}, "transform": None}
            for j, l in enumerate(body):
                s = l.strip()
                if s.startswith("material "):
                    inst["material"] = re.findall(r'"([^"]+)"', s)
                elif s == "transform":
                    inst["transform"] = [value(body[j + k]) for k in range(1, 5)]
                a = re.match(r'attribute \S+ "([^"]+)" (.*)$', s)
                if a:
                    inst["attrs"][a.group(1)] = value(a.group(2))
                f = re.match(r'(hide|visible|shadow|transparency|finalgather|globillum|caustic|reflection|refraction|trace) (\S+)$', s)
                if f:
                    # "finalgather on" and "finalgather 3" both occur: keep each.
                    inst["flags"][f.group(1) + ("_mode" if f.group(2).isdigit() else "")] = value(f.group(2))
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
            if isinstance(v, dict) and "refs" in v:
                args[k] = [tree(r, seen + (name,)) for r in v["refs"]]
            elif isinstance(v, dict):
                args[k] = tree(v["ref"], seen + (name,))
            elif isinstance(v, str) and v in textures:
                args[k] = {"texture": textures[v]}
            elif isinstance(v, str) and v in profiles:
                args[k] = {"light_profile": profiles[v]}
            else:
                args[k] = v
        out["args"] = args
        return out

    scene = {"options": {}, "camera": camera, "environment": None, "lights": {}, "materials": {}, "objects": {}}
    if options:
        env = options.pop("environment_function", None)
        scene["options"] = options
        if env:
            scene["environment"] = tree(env)
    seen = set()
    handles = {inst["attrs"].get("handle", strip_id(inst["name"])) for inst in instances}
    for inst in instances:
        handle = inst["attrs"].get("handle", strip_id(inst["name"]))
        if inst["item"] in lights:
            l = lights[inst["item"]]
            scene["lights"][handle] = {
                "fields": l["fields"],
                "material": tree(l["shader"]) if l["shader"] else None,
                "transform": inst["transform"],
            }
        if inst["material"]:
            # A mesh instance's render flags and attributes (#36).
            scene["objects"][handle] = {"flags": inst["flags"], "attrs": {k: v for k, v in inst["attrs"].items() if k != "handle"}}
        for mat in inst["material"]:
            # An instance node's copy uses its target's materials: they go
            # under the target (named after it), once.
            owner = strip_id(mat).split("__", 1)[0]
            if mat in seen or owner != handle and owner in handles:
                continue
            seen.add(mat)
            surface = strip_id(mat).split("__", 1)[-1]
            t = tree(mat)
            t["attrs"] = {k: v for k, v in shaders.get(mat, {}).get("attrs", {}).items() if k != "material_id"}
            scene["materials"][f"{handle}/{surface}"] = t
    scene["geometry_min_y"] = geometry_min_y(path)
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
