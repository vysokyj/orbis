#!/usr/bin/env python3
"""A refracting Uber solid with Iray's volume in a white furnace (#104), Monte Carlo.

  python3 scripts/dev/refr-volume-mc.py [--paths N] [--radius cm] [--scale s] [--iso] [--inner ior] <scene.duf>...
  python3 scripts/dev/refr-volume-mc.py --measure <render.png>...
      (each render in the export's own pass, scripts/dev/octane_pass.py, #139)

DAZ's MDL (`irayubermaterial.mdl`): Refraction Weight 1 with the glossy
layer shared is a smooth dielectric at Refraction Index (Fresnel both ways,
total internal reflection inside); a closed, not thin-walled sphere holds
`add_uber_volume`'s homogeneous medium, sigma_a = -ln(Transmitted Color) /
d_t, sigma_s = SSS Amount / d_s (mono, clamped to 1) or -ln(SSS Color) /
d_s (chromatic), Henyey-Greenstein at SSS Direction (`anisotropic_vdf`'s
directional_bias); colours linear, as in the .mi (scripts/dev/
prop-volume-mc.py reads them).

Under a uniform white dome a pixel of the sphere shows, by reciprocity,
the share of a beam entering there along the view that leaves again (any
direction). Prints per sphere and channel that share averaged over the
middle square calib-compare-all.py measures (±0.6 of the radius, the view
taken parallel), with --scale s multiplying sigma_s and --iso dropping the
phase (what an Octane variant renders). Radius 15 cm (Calib-Materials'
spheres) unless --radius. Gloss roughness, the camera's perspective and
the spheres' view of each other are left out.

--inner: the wall seen from inside at another IOR (1: no internal
reflection), a hypothesis knob.

--measure: per sphere of a render of a Calib-Furnace-* scene (found as
what differs from the dome, left to right), the middle square's mean
linear RGB (gamma 2.2) over the dome's (the top rows): the same share,
to set beside the Monte Carlo. Checked (#104): the exact semi-infinite
albedo (Chandrasekhar's H, R 150 cm) within 1 %; Octane's scattering
medium without fake shadows within 1 - 2 % on grey volumes.
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import importlib.util, os, sys
import numpy as np

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('prop_volume_mc', os.path.join(here, 'prop-volume-mc.py'))
pv = importlib.util.module_from_spec(spec)
sys.argv, _argv = [sys.argv[0]], sys.argv
spec.loader.exec_module(pv)
sys.argv = _argv
rng = np.random.default_rng(104)
pv.rng = rng


def fresnel(cosi, eta):
    """Unpolarised Fresnel reflectance and cos of the refracted angle;
    eta = n_incident / n_transmitted; total internal reflection -> 1."""
    sin2t = eta * eta * np.maximum(0, 1 - cosi * cosi)
    tir = sin2t >= 1
    cost = np.sqrt(np.maximum(0, 1 - sin2t))
    rs = (eta * cosi - cost) / np.maximum(eta * cosi + cost, 1e-12)
    rp = (cosi - eta * cost) / np.maximum(cosi + eta * cost, 1e-12)
    F = np.where(tir, 1.0, 0.5 * (rs * rs + rp * rp))
    return F, cost


def refract(d, nrm, cosi, cost, eta):
    """d refracted through the surface whose normal `nrm` faces against d."""
    return eta * d + (eta * cosi - cost)[:, None] * nrm


def reflect(d, nrm):
    return d - 2 * np.einsum('ij,ij->i', d, nrm)[:, None] * nrm


def escape(sa, ss, g, ior, R, N, box=0.6, inner=None):
    """Share of a beam along -z, entering uniformly over the middle square
    (±box R), that leaves the sphere again."""
    xy = (2 * rng.random((N, 2)) - 1) * box * R
    p = np.column_stack([xy, np.sqrt(R * R - (xy * xy).sum(1))])
    d = np.tile([0.0, 0.0, -1.0], (N, 1))
    nrm = p / R
    cosi = -np.einsum('ij,ij->i', d, nrm)
    F, cost = fresnel(cosi, 1 / ior)
    out = F.copy()
    w = 1 - F
    d = refract(d, nrm, cosi, cost, 1 / ior)
    st = sa + ss
    alb = ss / st if st > 0 else 0.0
    alive = w > 0
    for _ in range(100000):
        idx = np.flatnonzero(alive)
        if len(idx) == 0:
            break
        P, D = p[idx], d[idx]
        b = np.einsum('ij,ij->i', P, D)
        c = np.einsum('ij,ij->i', P, P) - R * R
        t_wall = -b + np.sqrt(np.maximum(b * b - c, 0))
        s = -np.log(1 - rng.random(len(idx))) / st if st > 0 else np.full(len(idx), np.inf)
        scat = s < t_wall
        si = idx[scat]
        p[si] = P[scat] + D[scat] * s[scat, None]
        w[si] *= alb
        d[si] = pv.rot(D[scat], pv.hg(g, len(si)))
        # The wall from inside: Fresnel at n -> 1, out or back in.
        wi = idx[~scat]
        pw = P[~scat] + D[~scat] * t_wall[~scat, None]
        pw *= R / np.linalg.norm(pw, axis=1, keepdims=True)
        n_in = -pw / R  # faces against the outgoing ray
        Dw = D[~scat]
        ci = -np.einsum('ij,ij->i', Dw, n_in)
        Fw, _ = fresnel(ci, ior if inner is None else inner)
        leave = rng.random(len(wi)) >= Fw
        out[wi[leave]] += w[wi[leave]]
        w[wi[leave]] = 0
        stay = wi[~leave]
        p[wi] = pw
        d[stay] = reflect(Dw[~leave], n_in[~leave])
        # Russian roulette.
        low = alive & (w < 1e-3)
        lr = np.flatnonzero(low)
        keep = rng.random(len(lr)) < 0.1
        w[lr[keep]] *= 10
        w[lr[~keep]] = 0
        alive = w > 0
    return out.mean()


def measure(paths):
    from PIL import Image
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import octane_pass
    for path in paths:
        src, label = octane_pass.pick(path)
        a = (np.asarray(Image.open(src).convert('RGB')).astype(float) / 255) ** 2.2
        bg = np.median(a[: a.shape[0] // 10].reshape(-1, 3), axis=0)
        # The spheres: whatever differs from the dome, grouped by columns.
        mask = np.abs(a - bg).max(-1) > 0.1 * max(bg.max(), 1e-3)
        cols = np.flatnonzero(mask.sum(0) > 10)
        groups, start = [], cols[0]
        for c0, c1 in zip(cols[:-1], cols[1:]):
            if c1 - c0 > 5:
                groups.append((start, c0))
                start = c1
        groups.append((start, cols[-1]))
        boxes = []
        for x0, x1 in groups:
            rows = np.flatnonzero(mask[:, x0:x1 + 1].sum(1) > 10)
            cx, cy, r = (x0 + x1) / 2, (rows[0] + rows[-1]) / 2, (x1 - x0) / 2
            boxes.append((int(cy - r * .6), int(cy + r * .6), int(cx - r * .6), int(cx + r * .6)))
        cells = []
        for y0, y1, x0, x1 in boxes:
            m = a[y0:y1, x0:x1].reshape(-1, 3).mean(0) / np.maximum(bg, 1e-6)
            cells.append(' / '.join(f'{x:.3f}' for x in m))
        print(f'{os.path.basename(path)} [{label}] (dome {bg[0]:.3f})')
        for k, c in enumerate(cells):
            print(f'  {k + 1}: {c}')


def main():
    args = sys.argv[1:]
    if args and args[0] == '--measure':
        return measure(args[1:])
    N, R, scale, iso, inner = 40000, 15.0, 1.0, False, None
    while args and args[0].startswith('--'):
        o = args.pop(0)
        if o == '--paths':
            N = int(args.pop(0))
        elif o == '--radius':
            R = float(args.pop(0))
        elif o == '--scale':
            scale = float(args.pop(0))
        elif o == '--iso':
            iso = True
        elif o == '--inner':
            inner = float(args.pop(0))
    print(f'radius {R} cm, {N} paths, sigma_s x {scale}' + (', isotropic' if iso else '') + (f', inside the wall at IOR {inner}' if inner else ''))
    for path in args:
        print(f'## {os.path.basename(path)}')
        for label, ch in pv.channels_of(path).items():
            sa, ss, g = pv.volume(ch)
            ior = ch.get('Refraction Index') or 1.5
            g = 0.0 if iso else g
            e = [escape(sa[k], ss[k] * scale, g, ior, R, N, inner=inner) for k in range(3)]
            f = lambda xs: ' / '.join(f'{x:.3f}' for x in xs)
            print(f'{label:26s} out {f(e)}  sigma_a {f(sa)}  sigma_s {f(s * scale for s in ss)} /cm  g {g}')


main()
