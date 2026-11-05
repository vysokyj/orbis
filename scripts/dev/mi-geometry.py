#!/usr/bin/env python3
"""Our export's geometry against DAZ's .mi export (#62): per object (by node
label), the vertex counts and how far each .mi vertex is from our mesh's
surface, in world space — pose, morphs, SubD, as Iray gets them.

  python3 scripts/dev/mi-geometry.py <scene.mi> <export.usda>

(`orbis export <scene.duf> -o <export.usda>`: the scene layer as text; an
.orbx or .usdz holds it binary.)

.mi objects are `trilist`s (vertices as big-endian float32 after
"scalar `": position, normal, then each UV set), placed by their instance
transform, which is world -> object (row vectors): world = (p - t) R^T.
Our USD is in metres; the .mi in DAZ's cm.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import math, re, struct, sys, zipfile


def point_triangle(p, a, b, c):
    """Squared distance from p to triangle abc (Ericson, Real-Time Collision Detection 5.1.5)."""
    sub = lambda u, v: (u[0] - v[0], u[1] - v[1], u[2] - v[2])
    dot = lambda u, v: u[0] * v[0] + u[1] * v[1] + u[2] * v[2]
    ab, ac, ap = sub(b, a), sub(c, a), sub(p, a)
    d1, d2 = dot(ab, ap), dot(ac, ap)
    if d1 <= 0 and d2 <= 0:
        q = a
    else:
        bp = sub(p, b); d3, d4 = dot(ab, bp), dot(ac, bp)
        if d3 >= 0 and d4 <= d3:
            q = b
        else:
            vc = d1 * d4 - d3 * d2
            if vc <= 0 and d1 >= 0 and d3 <= 0:
                v = d1 / (d1 - d3); q = (a[0] + v * ab[0], a[1] + v * ab[1], a[2] + v * ab[2])
            else:
                cp = sub(p, c); d5, d6 = dot(ab, cp), dot(ac, cp)
                if d6 >= 0 and d5 <= d6:
                    q = c
                else:
                    vb = d5 * d2 - d1 * d6
                    if vb <= 0 and d2 >= 0 and d6 <= 0:
                        w = d2 / (d2 - d6); q = (a[0] + w * ac[0], a[1] + w * ac[1], a[2] + w * ac[2])
                    else:
                        va = d3 * d6 - d5 * d4
                        if va <= 0 and (d4 - d3) >= 0 and (d5 - d6) >= 0:
                            w = (d4 - d3) / ((d4 - d3) + (d5 - d6)); q = (b[0] + w * (c[0] - b[0]), b[1] + w * (c[1] - b[1]), b[2] + w * (c[2] - b[2]))
                        else:
                            den = 1 / (va + vb + vc); v, w = vb * den, vc * den
                            q = (a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w)
    d = sub(p, q)
    return dot(d, d)


def surface_distance(points, tris, pts, cell=2.0):
    """Distance from each of `pts` to the nearest triangle (a grid of triangle boxes)."""
    grid = {}
    for t in tris:
        a, b, c = points[t[0]], points[t[1]], points[t[2]]
        lo = [int(math.floor(min(a[k], b[k], c[k]) / cell)) for k in range(3)]
        hi = [int(math.floor(max(a[k], b[k], c[k]) / cell)) for k in range(3)]
        for x in range(lo[0], hi[0] + 1):
            for y in range(lo[1], hi[1] + 1):
                for z in range(lo[2], hi[2] + 1):
                    grid.setdefault((x, y, z), []).append(t)
    out = []
    for q in pts:
        c = [int(math.floor(x / cell)) for x in q]
        best = math.inf
        for r in range(0, 8):
            for dx in range(-r, r + 1):
                for dy in range(-r, r + 1):
                    for dz in range(-r, r + 1):
                        if max(abs(dx), abs(dy), abs(dz)) != r:
                            continue
                        for t in grid.get((c[0] + dx, c[1] + dy, c[2] + dz), ()):
                            a, b, cc = points[t[0]], points[t[1]], points[t[2]]
                            try:
                                d = point_triangle(q, a, b, cc)
                            except ZeroDivisionError:   # a degenerate triangle: its corners
                                d = min(sum((q[k] - v[k]) ** 2 for k in range(3)) for v in (a, b, cc))
                            best = min(best, d)
            if best <= (r * cell) ** 2:
                break
        out.append(math.sqrt(best))
    return out


def mi_objects(path):
    b = open(path, 'rb').read()
    objects, i = {}, 0
    while True:
        i = b.find(b'\nobject "', i)
        if i < 0:
            break
        name = b[i + 9:b.find(b'"', i + 9)].decode('latin-1')
        hdr = re.search(rb'vertex (\d+) p( n)?((?: t \d+)*)', b[i:i + 400])
        n = int(hdr.group(1))
        per = 3 + (3 if hdr.group(2) else 0) + sum(int(x) for x in re.findall(rb't (\d+)', hdr.group(3)))
        s = b.find(b'scalar `', i)
        e = b.find(b'\nend object', i)
        if s < 0 or s > e:
            # A small object in text: [ x y z  nx ny nz  u v ... ]
            flat = [float(x) for x in b[b.find(b'[', i) + 1:b.find(b']', i)].split()]
        else:
            s += len(b'scalar `')
            flat = struct.unpack(f'>{n * per}f', b[s:s + n * per * 4])
        objects[name] = [flat[k * per:k * per + 3] for k in range(n)]
        i = e
    inst = {}
    for m in re.finditer(rb'\ninstance "([^"]+)" "([^"]+)"\n(.*?)\nend instance', b, re.S):
        body = m.group(3).decode('latin-1')
        h = re.search(r'"handle" "([^"]*)"', body)
        t = re.search(r'transform\n((?:\s+[-\d.e+ ]+\n){4})', body + '\n')
        if not h or not t:
            continue
        rows = [[float(x) for x in r.split()] for r in t.group(1).strip().split('\n')]
        inst[m.group(2).decode('latin-1')] = (h.group(1), rows)
    out = {}
    for obj, pts in objects.items():
        if obj not in inst:
            continue
        handle, T = inst[obj]
        R, t = [r[:3] for r in T[:3]], T[3][:3]
        out[handle] = [[sum((p[j] - t[j]) * R[i][j] for j in range(3)) for i in range(3)] for p in pts]
    return out


def usd_meshes(path):
    """Mesh name -> (points in cm, triangles)."""
    if path.endswith('.usda'):
        text = open(path, encoding='utf-8').read()
    else:  # a .usdz from before the binary layer
        text = zipfile.ZipFile(path).read('scene.usda').decode()
    out = {}
    for m in re.finditer(r'def Mesh "([^"]+)"(.*?)\n    \}', text, re.S):
        body = m.group(2)
        counts = [int(x) for x in re.search(r'faceVertexCounts = \[([^\]]*)\]', body).group(1).split(',')]
        idx = [int(x) for x in re.search(r'faceVertexIndices = \[([^\]]*)\]', body).group(1).split(',')]
        pts = [[float(x) * 100.0 for x in p.split(',')] for p in re.findall(r'\(([^)]*)\)', re.search(r'point3f\[\] points = \[([^\]]*)\]', body).group(1))]
        tris, k = [], 0
        for n in counts:
            f = idx[k:k + n]
            tris += [(f[0], f[j], f[j + 1]) for j in range(1, n - 1)]
            k += n
        out[m.group(1)] = (pts, tris)
    return out


def key(s):
    return re.sub(r'[^0-9A-Za-z]+', '_', s).strip('_').lower()


mi, usd = mi_objects(sys.argv[1]), usd_meshes(sys.argv[2])
ours_by_key = {key(k): v for k, v in usd.items()}
print(f"{'object':40} {'.mi verts':>10} {'ours':>8} {'max cm':>8} {'mean cm':>8}   (to our surface)")
for handle, pts in sorted(mi.items()):
    if len(pts) > 1_000_000:
        print(f"{handle:40} {len(pts):10}   (skipped: strand hair as triangles)")
        continue
    k = key(handle)
    cand = [v for kk, v in ours_by_key.items() if kk == k or kk.startswith(k + '_')]
    if not cand:
        print(f'{handle:40} {len(pts):10} {"-":>8}')
        continue
    points, tris = [], []
    for p, t in cand:
        tris += [(a + len(points), b + len(points), c + len(points)) for a, b, c in t]
        points += p
    d = surface_distance(points, tris, pts)
    print(f'{handle:40} {len(pts):10} {len(points):8} {max(d):8.4f} {sum(d) / len(d):8.4f}')
