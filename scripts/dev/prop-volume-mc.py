#!/usr/bin/env python3
"""A translucent Uber prop as Iray sees it (#87): a closed sphere, Monte Carlo.

  python3 scripts/dev/prop-volume-mc.py [--paths N] [--radius cm] [--limit n,...] <scene.duf | name=channels.json>...

DAZ's MDL, read from `irayubermaterial.mdl` (`diffuse_trans_mix`,
`add_uber_volume`): the surface is a Lambertian reflection of weight 1 - w
tinted by the base colour over a Lambertian transmission of weight w tinted
by Translucency Color (no Fresnel, no IOR: `diffuse_transmission_bsdf`);
a closed, not thin-walled surface holds a homogeneous volume with
sigma_a = -ln(Transmitted Color) / d_t and sigma_s = SSS Amount / d_s
(mono, clamped to 1 as DAZ passes it) or -ln(SSS Color) / d_s (chromatic,
`logarithmic_scatter_enable`), Henyey-Greenstein at SSS Direction; colours
linear, as in the .mi. Light that meets the surface from inside leaves
with w Tc or is reflected back in with (1 - w) b.

Prints per sphere and channel the sphere's albedo under uniform light (a
white furnace: (1 - w) b + w Tc E, E the share of the light that entered
and comes out again), what of it the volume gives (w Tc E), and E cut at
each --limit volume vertices on the path (Iray's iray_max_sss_path_length,
Octane's maxScatterDepth / diffuse depth). Radius 15 cm (Calib-Materials'
spheres) unless --radius. Only geometry-free inputs: the sphere's light,
its position in the scene, gloss are left out - a ratio between two
spheres of one scene at one position is what this predicts. A thin-walled
sphere is not modelled (its line shows the wall's reflection alone, "thin").

Checked on Calib-SSSColor (#87): Iray's spheres over this albedo, each at
its position's light (Octane's plain diffuse 0.5 on the same sphere),
0.93 - 0.98, as its plain diffuse sphere (0.95 - 0.96).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import gzip, json, math, sys
import numpy as np

rng = np.random.default_rng(87)


def srgb_lin(c):
    """DAZ hands Iray its colours linear: gamma 2.2 on the sRGB values (the .mi)."""
    return [max(0.0, x) ** 2.2 for x in c]


def hg(g, n):
    if abs(g) < 1e-6:
        return 2 * rng.random(n) - 1
    x = (1 - g * g) / (1 - g + 2 * g * rng.random(n))
    return (1 + g * g - x * x) / (2 * g)


def rot(d, mu):
    n = len(d)
    phi = 2 * np.pi * rng.random(n)
    s = np.sqrt(np.maximum(0, 1 - mu * mu))
    a = np.where(np.abs(d[:, 2:3]) < 0.9, np.array([[0, 0, 1.0]]), np.array([[1.0, 0, 0]]))
    u = np.cross(d, a)
    u /= np.linalg.norm(u, axis=1, keepdims=True)
    v = np.cross(d, u)
    return d * mu[:, None] + u * (s * np.cos(phi))[:, None] + v * (s * np.sin(phi))[:, None]


def lamb(nrm):
    return rot(nrm, np.sqrt(rng.random(len(nrm))))


def escape(sa, ss, g, refl, trans, R, N, limits):
    """Share of the light entering the sphere (Lambertian, at a pole: a
    furnace is the same at every point of a sphere) that leaves it, total
    and with at most `limits` volume vertices on the path."""
    st = sa + ss
    if st <= 0:
        # An empty ball: every crossing meets the wall again.
        e = trans / (1 - refl) if refl < 1 else 1.0
        return e, [e for _ in limits]
    alb = ss / st
    p = np.tile([0.0, 0.0, R], (N, 1))
    d = lamb(-p / R)
    w = np.ones(N)
    k = np.zeros(N, dtype=np.int64)
    alive = np.ones(N, bool)
    out = 0.0
    out_k = np.zeros(len(limits))
    for _ in range(200000):
        idx = np.flatnonzero(alive)
        if len(idx) == 0:
            break
        P, D = p[idx], d[idx]
        b = np.einsum('ij,ij->i', P, D)
        c = np.einsum('ij,ij->i', P, P) - R * R
        t_wall = -b + np.sqrt(np.maximum(b * b - c, 0))
        s = -np.log(1 - rng.random(len(idx))) / st
        scat = s < t_wall
        # Scattering inside.
        si = idx[scat]
        p[si] = P[scat] + D[scat] * s[scat, None]
        w[si] *= alb
        k[si] += 1
        d[si] = rot(D[scat], hg(g, len(si)))
        # The wall: out with `trans`, back in with `refl` (Lambertian).
        wi = idx[~scat]
        pw = P[~scat] + D[~scat] * t_wall[~scat, None]
        pw *= R / np.linalg.norm(pw, axis=1, keepdims=True)
        e = w[wi] * trans
        out += e.sum()
        for j, n in enumerate(limits):
            out_k[j] += e[k[wi] <= n].sum()
        p[wi] = pw
        w[wi] *= refl
        d[wi] = lamb(-pw / R)
        # Russian roulette.
        low = alive & (w < 1e-3)
        lr = np.flatnonzero(low)
        keep = rng.random(len(lr)) < 0.1
        w[lr[keep]] *= 10
        alive[lr[~keep]] = False
        alive &= w > 0
    return out / N, list(out_k / N)


def channels_of(path):
    """The spheres' Uber channels of a generated Calib-Materials variant."""
    raw = open(path, 'rb').read()
    try:
        raw = gzip.decompress(raw)
    except OSError:
        pass
    d = json.loads(raw)
    mats = {}
    geo_label = {}
    for n in d['scene']['nodes']:
        if 'geometries' in n:
            for g in n['geometries']:
                geo_label[g['id']] = n.get('label')
    for m in d['scene']['materials']:
        label = geo_label.get(m['geometry'].lstrip('#'), m['geometry'])
        lib = next(l for l in d['material_library'] if '#' + l['id'] == m['url'] or l['id'] == m['url'].lstrip('#'))
        ch = {c['channel']['id']: c['channel'].get('current_value') for e in lib.get('extra', []) if e.get('type') == 'studio_material_channels' for c in e['channels']}
        ch['diffuse'] = m.get('diffuse', {}).get('channel', {}).get('current_value') or lib.get('diffuse', {}).get('channel', {}).get('current_value')
        if ch.get('Translucency Weight') is not None:
            mats[label] = ch
    return mats


def volume(ch):
    """sigma_a, sigma_s (1/cm per channel, linear colours) and g, as DAZ's MDL."""
    d_t = ch.get('Transmitted Measurement Distance') or 0.0
    d_s = ch.get('Scattering Measurement Distance') or 0.0
    tc = srgb_lin(ch.get('Transmitted Color') or [1, 1, 1])
    sa = [(-math.log(min(max(c, 1e-4), 1.0)) / d_t) if d_t > 0 and any(x > 0 for x in tc) else 0.0 for c in tc]
    if (ch.get('SSS Amount') or 0.0) <= 0:
        # DAZ sets sub_surface_enable only with SSS Amount > 0, in both
        # modes (Calib-SSS-Solid's .mi: S7, chromatic at Amount 0).
        ss = [0.0] * 3
    elif int(ch.get('SSS Mode') or 0) == 1:
        sc = srgb_lin(ch.get('SSS Color') or [0, 0, 0])
        ss = [(-math.log(min(max(c, 1e-4), 1.0)) / d_s) if d_s > 0 else 0.0 for c in sc]
    else:
        a = min(max(ch.get('SSS Amount') or 0.0, 0.0), 1.0)
        ss = [a / d_s if d_s > 0 else 0.0] * 3
    return sa, ss, max(-0.95, min(0.95, ch.get('SSS Direction') or 0.0))


def main():
    args = sys.argv[1:]
    N, R, limits = 20000, 15.0, [16, 64, 128, 256, 1024]
    while args and args[0].startswith('--'):
        o = args.pop(0)
        if o == '--paths':
            N = int(args.pop(0))
        elif o == '--radius':
            R = float(args.pop(0))
        elif o == '--limit':
            limits = [int(x) for x in args.pop(0).split(',')]
    print(f'radius {R} cm, {N} paths; E cut at {limits} volume vertices')
    for path in args:
        print(f'## {path}')
        for label, ch in channels_of(path).items():
            w = min(max(ch.get('Translucency Weight') or 0.0, 0.0), 1.0)
            b = srgb_lin(ch['diffuse'] or [1, 1, 1])
            tcol = srgb_lin(ch.get('Translucency Color') or [0.5] * 3)
            thin = bool(ch.get('Thin Walled'))
            sa, ss, g = volume(ch)
            rows = []
            for k in range(3):
                if w <= 0 or thin:
                    rows.append(((1 - w) * b[k], 0.0, 0.0, []))
                    continue
                e, ek = escape(sa[k], ss[k], g, (1 - w) * b[k], w * tcol[k], R, N, limits)
                rows.append(((1 - w) * b[k] + w * tcol[k] * e, w * tcol[k] * e, e, ek))
            alpha = [ss[k] * (1 - g) / max(ss[k] * (1 - g) + sa[k], 1e-9) for k in range(3)]
            f = lambda xs: ' / '.join(f'{x:.4f}' for x in xs)
            print(f'{label:24s} albedo {f(r[0] for r in rows)}  volume {f(r[1] for r in rows)}  E {f(r[2] for r in rows)}'
                  f'  alpha {f(alpha)}  sigma_s {f(ss)} /cm' + ('  thin' if thin else ''))
            for j, n in enumerate(limits):
                if rows[0][3]:
                    print(f'{"":24s}   E at {n:5d} vertices {f(r[3][j] / max(r[2], 1e-12) for r in rows)} of E')


if __name__ == '__main__':
    main()
