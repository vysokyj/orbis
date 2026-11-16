#!/usr/bin/env python3
"""Octane's glossy lobes against the analytic microfacet lobe under the
scene's own HDRI or a white dome (#186; #95's lost `hdrint3.py`).

  hdri-lobe.py variants <Calib-Glossy.orbx|Calib-Point-UberGloss.orbx> <out dir> <label>:<brdf>:<d|m|l|p>:<h|w>[:<r,g,b>]...
      Variants of a Calib-Glossy export with every sphere replaced: G1 a
      Lambert of albedo 0.2 (the normaliser), G2 - G8 black dielectrics
      (specular 1, IOR 1.5; `d`) or white metals (metallic 1; `m`) at the
      pins in PINS, Universal `brdf` <brdf>; or the dielectric lobe as a
      Specular Layer (139) over a black base, in a Layered material (143,
      `l`) or on OpenPBR's `layer` pin (505, `p`), or two of specular 0.5
      in a Material Layer Group there (`g`; #190); <r,g,b> a
      constant normal-map colour on the lobe; floor hidden; the HDRI kept
      (`h`) or the dome a uniform white (`w`). Writes
      <out dir>/Calib-Glossy__<label>.orbx; render with octane-render.sh
      (the raw pass serves: disc and ring means). From a
      Calib-Point-UberGloss export: its eight spheres at POINT_PINS under
      its one small light (no Lambert, no floor; compare with `ratio`).
  hdri-lobe.py ratio <scene> <a.png> <b.png>
      Per sphere disc (< 0.93 R, linear), render a over render b.
  hdri-lobe.py measure <render.png> <hdr> ggx|beckmann diel|metal [--white] [--corr]
      Per sphere and ring of r / R: (Octane / Octane's Lambert G1) over
      (analytic / analytic Lambert). The analytic lobe at alpha = pin^2:
      each pixel's camera ray hits its sphere (dump + .duf camera, as
      sphere_discs.py), the half vector sampled from GGX's visible normals
      (Heitz 2018) or Beckmann's D cos, weight F(v.h) G (separable Smith,
      --corr height-correlated), the radiance read from <hdr> (lat-long;
      the export's own file: `orbx_ocs file <orbx> assets/tex/<name>.hdr
      <out>`), its orientation fitted on G2's mirror (correlation in log
      printed). A metal's rough pins (>= 0.6) are the integration's
      baseline; its sharp ones are the sun glint's position and say little.

Gamma 2.2 decode (the export's imager response); 8-bit, so keep the
Lambert sphere's brightest pixel under 1 (printed).
"""
import math
import os
import re
import subprocess
import sys
if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
    print(__doc__)
    sys.exit(0)
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
PINS = {'G2_refl_0_5': 0.05, 'G3_refl_0_25': 0.4, 'G4_refl_1': 0.5, 'G5_refl_0': 0.6,
        'G6_rough_0_1': 0.7, 'G7_rough_0_6': 0.8, 'G8_dual_lobe': 1.0}
PIN_ORDER = [None, 0.05, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]  # discs left to right
# Calib-Point-UberGloss (one small light, no floor): every sphere replaced,
# U1 - U8 left to right; 0.448 is Calib-Skin-Nishi's dual lobe (#190).
POINT_PINS = {'U1_dielectric_r_0_25': 0.3, 'U2_dielectric_r_0_354': 0.4, 'U3_dielectric_r_0_5': 0.448,
              'U4_dielectric_r_0_7': 0.5, 'U5_dielectric_r_1_0': 0.6, 'U6_metal_r_0_354': 0.7,
              'U7_metal_r_0_5': 0.8, 'U8_metal_r_0_7': 1.0}
EDGES = [0, 0.3, 0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.93]


# --- variants -------------------------------------------------------------

def variants(src, out, specs):
    tool = os.path.join(subprocess.check_output([os.path.join(HERE, 'target-dir.sh')], text=True).strip(), 'release/examples/orbx_ocs')
    os.makedirs(out, exist_ok=True)
    tmp = os.path.join(out, 'project.ocs')
    subprocess.run([tool, 'get', src, tmp], check=True)
    ocs0 = open(tmp).read()
    nid = [990000]

    def n():
        nid[0] += 1
        return nid[0]

    def val(t, at, v):
        return f"<node id='{n()}' type='{t}'><attr name='value' type='{at}'>{v}</attr></node>"

    def uni(name, pins):
        body = ''.join(f"<pin name='{p}'>{x}</pin>" for p, x in pins.items())
        return f"<node id='{n()}' type='130' name='{name}'><attr name='compatibilityVersion' type='2'>15050000</attr>{body}</node>"

    def block_end(s, start):
        depth = 0
        for t in re.finditer(r"<node\b|</node>", s[start:]):
            depth += 1 if t.group() == '<node' else -1
            if depth == 0:
                return start + t.end()

    def replace_mat(ocs, mat, xml):
        m = re.search(rf"name='/Scene/Materials/{mat}[^']*'", ocs)
        start = ocs.index("<node", ocs.index("<pin name='input'>", m.end()))
        return ocs[:start] + xml + ocs[block_end(ocs, start):]

    scene = os.path.basename(src).split('.')[0]
    point = 'Point' in scene
    for spec in specs:
        label, brdf, kind, env, *nrm = spec.split(':')
        # Upper case: the base a grey diffuse of albedo 0.5 instead of black
        # (what the layer's transmission leaves of it; #190), `B` that base
        # alone (OpenPBR, specular 0, no layer).
        grey, kind = ('0.5 0.5 0.5', kind.lower()) if kind.isupper() else ('0 0 0', kind)
        # An optional constant normal-map colour on the lobe (#190).
        npin = lambda: f"<pin name='normal'>{val(33, 8, nrm[0].replace(',', ' '))}</pin>" if nrm else ''
        if point:
            ocs = ocs0
        else:
            ocs = replace_mat(ocs0, 'Plane', uni('Plane_Default', {'opacity': val(31, 6, 0)}))
            ocs = replace_mat(ocs, 'G1_matte', uni('G1_matte_Default', {'albedo': val(33, 8, '0.2 0.2 0.2'), 'specular': val(33, 8, '0 0 0')}))
        for mat, pin in (POINT_PINS if point else PINS).items():
            if kind == 'b':
                ocs = replace_mat(ocs, mat, f"<node id='{n()}' type='505' name='{mat}_Default'><attr name='compatibilityVersion' type='2'>15050000</attr>"
                                  f"<pin name='base'>{val(31, 6, 1)}</pin><pin name='baseColor'>{val(33, 8, grey)}</pin><pin name='specular'>{val(31, 6, 0)}</pin></node>")
                continue
            if kind in ('l', 'p', 'g'):
                # The same dielectric lobe as a Specular Layer (139) over a
                # black base: a Layered material (143, `layered_coat_node`)
                # or OpenPBR's `layer` pin (505, `skin_dual_lobe_layers`);
                # `g` two layers of specular 0.5 in a Material Layer Group
                # (144) there, the normal on Layer 1 only (the skin's lobe 1).
                def layer(spec, normal):
                    return (f"<node id='{n()}' type='139' name='{mat} layer'><attr name='compatibilityVersion' type='2'>15050000</attr>"
                            f"<pin name='specular'>{val(33, 8, spec)}</pin><pin name='brdf'>{val(57, 2, brdf)}</pin>"
                            f"<pin name='roughness'>{val(31, 6, pin)}</pin><pin name='ior'>{val(31, 6, 1.5)}</pin>{npin() if normal else ''}</node>")
                if kind == 'l':
                    base = uni(mat + '_base', {'albedo': val(33, 8, grey), 'specular': val(33, 8, '0 0 0')})
                    xml = (f"<node id='{n()}' type='143' name='{mat}_Default'><attr name='pin_count' type='2'>1</attr>"
                           f"<attr name='compatibilityVersion' type='2'>15050000</attr><pin name='material1'>{base}</pin>"
                           f"<pin name='Layer 1' dynamicType='33'>{layer('1 1 1', True)}</pin></node>")
                else:
                    top = layer('1 1 1', True) if kind == 'p' else (
                        f"<node id='{n()}' type='144' name='{mat} group'><attr name='pin_count' type='2'>2</attr>"
                        f"<attr name='compatibilityVersion' type='2'>15050000</attr>"
                        f"<pin name='Layer 1' dynamicType='33'>{layer('0.5 0.5 0.5', True)}</pin>"
                        f"<pin name='Layer 2' dynamicType='33'>{layer('0.5 0.5 0.5', False)}</pin></node>")
                    xml = (f"<node id='{n()}' type='505' name='{mat}_Default'><attr name='compatibilityVersion' type='2'>15050000</attr>"
                           f"<pin name='base'>{val(31, 6, 1)}</pin><pin name='baseColor'>{val(33, 8, grey)}</pin>"
                           f"<pin name='specular'>{val(31, 6, 0)}</pin><pin name='layer'>{top}</pin></node>")
                ocs = replace_mat(ocs, mat, xml)
                continue
            p = {'albedo': val(33, 8, '1 1 1' if kind == 'm' else grey), 'specular': val(33, 8, '1 1 1'),
                 'brdf': val(57, 2, brdf), 'roughness': val(31, 6, pin)}
            p['metallic' if kind == 'm' else 'ior'] = val(31, 6, 1 if kind == 'm' else 1.5)
            if nrm:
                p['normal'] = val(33, 8, nrm[0].replace(',', ' '))
            ocs = replace_mat(ocs, mat, uni(mat + '_Default', p))
        if env == 'w' and not point:
            m = re.search(r"<node id='(\d+)' type='34'>\s*<attr name='filename' type='11'>[^<]*\.hdr</attr>", ocs)
            ocs = ocs[:m.start()] + f"<node id='{m.group(1)}' type='33'><attr name='value' type='8'>1 1 1</attr></node>" + ocs[block_end(ocs, m.start()):]
        p = os.path.join(out, f'v_{label}.ocs')
        open(p, 'w').write(ocs)
        subprocess.run([tool, 'set', src, p, os.path.join(out, f'{scene}__{label}.orbx')], check=True)


# --- measure --------------------------------------------------------------

def read_hdr(p):
    """Radiance RGBE, new-style RLE scanlines."""
    with open(p, 'rb') as f:
        while f.readline().strip():
            pass
        dims = f.readline().split()
        h, w = int(dims[1]), int(dims[3])
        data = f.read()
    img = np.zeros((h, w, 4), np.uint8)
    i = 0
    for y in range(h):
        assert data[i] == 2 and data[i + 1] == 2, 'only RLE scanlines'
        i += 4
        for c in range(4):
            x = 0
            while x < w:
                cnt = data[i]
                i += 1
                if cnt > 128:
                    cnt -= 128
                    img[y, x:x + cnt, c] = data[i]
                    i += 1
                else:
                    img[y, x:x + cnt, c] = np.frombuffer(data[i:i + cnt], np.uint8)
                    i += cnt
                x += cnt
    e = img[..., 3].astype(np.int32)
    s = np.where(e > 0, np.ldexp(1.0, e - 136), 0.0)
    return (img[..., :3] * s[..., None]) @ np.array([0.2126, 0.7152, 0.0722])


class Env:
    def __init__(self, path, white):
        self.white = white
        self.Y = None if white else read_hdr(path)
        self.sgn, self.off = 1, 0.0

    def __call__(self, d):
        if self.white:
            return 1.0
        h, w = self.Y.shape
        u = (self.sgn * np.arctan2(d[..., 0], d[..., 2]) / (2 * np.pi) + self.off) % 1.0
        v = np.arccos(np.clip(d[..., 1], -1, 1)) / np.pi
        return self.Y[np.minimum((v * h).astype(int), h - 1), np.minimum((u * w).astype(int), w - 1)]


def fresnel(n, c):
    c = np.clip(c, 0, 1)
    ct = np.sqrt(np.maximum(1 - (1 - c * c) / (n * n), 0))
    rp = (ct - c * n) / (ct + c * n)
    ro = (c - ct * n) / (c + ct * n)
    return 0.5 * (rp * rp + ro * ro)


def smith_lambda(a, c, model):
    c = np.clip(c, 1e-6, 1)
    t2 = (1 - c * c) / (c * c)
    if model == 'ggx':
        return (-1 + np.sqrt(1 + a * a * t2)) / 2
    m = 1 / (a * np.sqrt(t2) + 1e-9)  # Beckmann, Walter's rational fit
    return np.where(m < 1.6, (1 - 1.259 * m + 0.396 * m * m) / (3.535 * m + 2.181 * m * m), 0)


def frame(nrm):
    t = np.where(np.abs(nrm[:, 1:2]) < 0.99, np.array([[0, 1, 0]]), np.array([[1, 0, 0]]))
    t1 = np.cross(t, nrm)
    t1 /= np.linalg.norm(t1, axis=1)[:, None]
    return t1, np.cross(nrm, t1)


def lobe(a, nrm, v, env, diel, model, corr, rng, spp=192):
    t1, t2 = frame(nrm)
    vl = np.repeat(np.stack([(v * t1).sum(1), (v * t2).sum(1), (v * nrm).sum(1)], 1), spp, 0)
    T1, T2, NN = (np.repeat(x, spp, 0) for x in (t1, t2, nrm))
    u1, u2 = rng.random(len(vl)), rng.random(len(vl))
    if model == 'ggx':  # visible normals
        vh = np.stack([a * vl[:, 0], a * vl[:, 1], vl[:, 2]], 1)
        vh /= np.linalg.norm(vh, axis=1)[:, None]
        lensq = vh[:, 0] ** 2 + vh[:, 1] ** 2
        B1 = np.where(lensq[:, None] > 1e-12, np.stack([-vh[:, 1], vh[:, 0], 0 * lensq], 1) / np.sqrt(np.maximum(lensq, 1e-12))[:, None], np.array([[1, 0, 0]]))
        B2 = np.cross(vh, B1)
        r, phi = np.sqrt(u1), 2 * np.pi * u2
        p1, p2 = r * np.cos(phi), r * np.sin(phi)
        s = 0.5 * (1 + vh[:, 2])
        p2 = (1 - s) * np.sqrt(1 - p1 * p1) + s * p2
        nh = p1[:, None] * B1 + p2[:, None] * B2 + np.sqrt(np.maximum(0, 1 - p1 * p1 - p2 * p2))[:, None] * vh
        h = np.stack([a * nh[:, 0], a * nh[:, 1], np.maximum(1e-6, nh[:, 2])], 1)
    else:  # D(h) cos
        c = 1 / np.sqrt(1 - a * a * np.log(1 - u1))
        sn, phi = np.sqrt(1 - c * c), 2 * np.pi * u2
        h = np.stack([sn * np.cos(phi), sn * np.sin(phi), c], 1)
    h /= np.linalg.norm(h, axis=1)[:, None]
    vdh = (vl * h).sum(1)
    ll = 2 * vdh[:, None] * h - vl
    Lv, Ll = smith_lambda(a, vl[:, 2], model), smith_lambda(a, ll[:, 2], model)
    G = 1 / (1 + Lv + Ll) if corr else 1 / ((1 + Lv) * (1 + Ll))
    w = G * (1 + Lv) if model == 'ggx' else G * vdh / (vl[:, 2] * h[:, 2])
    L = env(ll[:, 0:1] * T1 + ll[:, 1:2] * T2 + ll[:, 2:3] * NN)
    F = fresnel(1.5, vdh) if diel else 1.0
    return np.where((ll[:, 2] > 0) & (vdh > 0), w * F * L, 0).reshape(-1, spp).mean(1)


def lambert(nrm, env, rng, spp=256):
    t1, t2 = frame(nrm)
    u1, u2 = rng.random(len(nrm) * spp), rng.random(len(nrm) * spp)
    r, phi = np.sqrt(u1), 2 * np.pi * u2
    T1, T2, NN = (np.repeat(x, spp, 0) for x in (t1, t2, nrm))
    d = (r * np.cos(phi))[:, None] * T1 + (r * np.sin(phi))[:, None] * T2 + np.sqrt(1 - u1)[:, None] * NN
    return (np.ones(len(d)) * env(d)).reshape(-1, spp).mean(1)


def scene_geometry(scene, shape):
    import sphere_discs as sd
    duf = sd.load_duf(sd.find_duf(scene))
    pos, rot, focal, fr, wh, _ = sd.camera(duf)
    fy = focal / fr * wh[1]
    spheres = []
    for _, vv in sd._dump_spheres(sd.find_dump(scene)):
        ext = vv.max(0) - vv.min(0)
        c = vv.mean(0)
        if ext.min() > 0.9 * ext.max() and ((c - pos) @ rot)[2] < 0:
            spheres.append((c, np.linalg.norm(vv - c, axis=1).mean()))
    ds, _ = sd.discs(scene, shape=shape)

    def px(c):
        p = (c - pos) @ rot
        return wh[0] / 2 + fy * p[0] / -p[2] - .5
    return pos, rot, fy, wh, [(d, *min(spheres, key=lambda s: abs(px(s[0]) - d.cx))) for d in ds]


def hits(d, c, R, pos, rot, fy, wh):
    ys, xs = np.mgrid[int(d.cy - d.ry - 2):int(d.cy + d.ry + 3), int(d.cx - d.rx - 2):int(d.cx + d.rx + 3)]
    xs, ys = xs.ravel(), ys.ravel()
    dw = np.stack([(xs + .5 - wh[0] / 2) / fy, -(ys + .5 - wh[1] / 2) / fy, -np.ones(len(xs))], 1) @ rot.T
    dw /= np.linalg.norm(dw, axis=1)[:, None]
    oc = pos - c
    b = dw @ oc
    disc = b * b - (oc @ oc - R * R)
    hit = disc > 0
    nrm = (pos + (-b - np.sqrt(np.maximum(disc, 0)))[:, None] * dw - c) / R
    rr = np.sqrt(np.maximum(0, 1 - ((-dw) * nrm).sum(1) ** 2))
    return xs[hit], ys[hit], nrm[hit], -dw[hit], rr[hit]


def measure(png, hdr, model, kind, white, corr):
    from PIL import Image
    import octane_pass
    path, label = octane_pass.pick(png)
    img = np.asarray(Image.open(path).convert('RGB'), float) / 255.0
    Y = (img ** 2.2) @ np.array([0.2126, 0.7152, 0.0722])
    pos, rot, fy, wh, sph = scene_geometry('Calib-Glossy', img.shape)
    env = Env(hdr, white)
    rng = np.random.default_rng(1)
    diel = kind == 'diel'
    print(f'{os.path.basename(path)}: Octane pass {label}; {model} {kind}, {"white dome" if white else os.path.basename(hdr)}')
    if not white:
        d, c, R = sph[1]
        xs, ys, nrm, v, rr = hits(d, c, R, pos, rot, fy, wh)
        m = rr < 0.9
        cv = (nrm[m] * v[m]).sum(1)
        refl = 2 * cv[:, None] * nrm[m] - v[m]
        obs = np.log(Y[ys[m], xs[m]] / (fresnel(1.5, cv) if diel else 1) + 1e-5)
        best = (-2, 1, 0.0)
        for env.sgn in (1, -1):
            for env.off in np.arange(0, 1, 1 / 512):
                cc = np.corrcoef(obs, np.log(env(refl) + 1e-5))[0, 1]
                best = max(best, (cc, env.sgn, env.off))
        _, env.sgn, env.off = best
        print(f'HDRI orientation from G2: sign {env.sgn} offset {env.off:.4f}, corr(log) {best[0]:.3f}')
    d, c, R = sph[0]
    xs, ys, nrm, v, rr = hits(d, c, R, pos, rot, fy, wh)
    m = rr < 0.93
    oL, aL = Y[ys[m], xs[m]].mean(), 0.2 * lambert(nrm[m][::3], env, rng).mean()
    print(f'Lambert G1: Octane {oL:.4f}, analytic {aL:.4f}; its brightest pixel {img[ys[m], xs[m]].max():.3f}')
    print('pin   ' + ' '.join(f'{lo:.2f}-{hi:.2f}' for lo, hi in zip(EDGES, EDGES[1:])) + '   disc < 0.93')
    for k in range(1, len(sph)):
        d, c, R = sph[k]
        xs, ys, nrm, v, rr = hits(d, c, R, pos, rot, fy, wh)
        m = rr < 0.93
        o = Y[ys[m], xs[m]] / oL
        an = lobe(PIN_ORDER[k] ** 2, nrm[m], v[m], env, diel, model, corr, rng) / aL
        rr = rr[m]
        row = [o[b].mean() / an[b].mean() for b in ((rr >= lo) & (rr < hi) for lo, hi in zip(EDGES, EDGES[1:]))]
        print(f'{PIN_ORDER[k]:<5} ' + ' '.join(f'{x:9.3f}' for x in row) + f'   {o.mean() / an.mean():.3f}')


def ratio(scene, a, b):
    """Disc means of render a over render b (same scene, linear)."""
    from PIL import Image
    import octane_pass
    import sphere_discs as sd
    ims = []
    for png in (a, b):
        path, _ = octane_pass.pick(png)
        ims.append((np.asarray(Image.open(path).convert('RGB'), float) / 255.0) ** 2.2 @ np.array([0.2126, 0.7152, 0.0722]))
    ds, _ = sd.discs(scene, shape=ims[0].shape)
    out = []
    for d in ds:
        ys, xs = np.mgrid[0:ims[0].shape[0], 0:ims[0].shape[1]]
        m = ((xs - d.cx) / d.rx) ** 2 + ((ys - d.cy) / d.ry) ** 2 < 0.93 ** 2
        out.append(ims[0][m].sum() / max(ims[1][m].sum(), 1e-9))
    print(f'{os.path.basename(a)} / {os.path.basename(b)}: ' + ' '.join(f'{x:.3f}' for x in out))


if sys.argv[1] == 'ratio':
    ratio(sys.argv[2], sys.argv[3], sys.argv[4])
elif sys.argv[1] == 'variants':
    variants(sys.argv[2], sys.argv[3], sys.argv[4:])
elif sys.argv[1] == 'measure':
    a = [x for x in sys.argv[2:] if not x.startswith('--')]
    measure(a[0], a[1], 'ggx' if a[2] == 'ggx' else 'beckmann', a[3], '--white' in sys.argv, '--corr' in sys.argv)
else:
    sys.exit(__doc__)
