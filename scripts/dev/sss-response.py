#!/usr/bin/env python3
"""Octane's random-walk subsurface response (#19): Calib-SSS's eight spheres
with their materials replaced, the same colours as a matte Universal and as
OpenPBR Surface with subsurface — what colour Octane renders for the one it
is given.

  python3 scripts/dev/sss-response.py make <Calib-SSS.orbx> <out dir>
  python3 scripts/dev/sss-response.py measure <octane dir>

`make` writes <out dir>/SSSR-<variant>.orbx; render them with
scripts/dev/octane-render.sh, then `measure` prints each sphere's colour
over its matte Universal twin (variant `uni`), per channel.
"""
import os, re, subprocess, sys
import numpy as np

here = os.path.dirname(os.path.abspath(__file__))
tool = os.path.join(here, '../../target/release/examples/orbx_ocs')
SPHERES = ['S1_opaque', 'S2_transl_0_5', 'S3_transl_1', 'S4_thin_walled', 'S7_chromatic', 'S8_short_scatter', 'S5_red_transmit', 'S6_sss_amount']
# Linear colours, one per sphere: greys, then two skin tones (light, dark).
COLOURS = [(0.05,) * 3, (0.1,) * 3, (0.2,) * 3, (0.4,) * 3, (0.6,) * 3, (0.8,) * 3, (0.6, 0.35, 0.25), (0.15, 0.07, 0.04)]
SKIN_RADIUS = (0.001092, (1.0, 0.425, 0.389))   # m, per-channel scale (Monique's export)
VARIANTS = {
    'uni': None,                                             # matte Universal
    'pbr': dict(subsurface=0.0, radius=SKIN_RADIUS),         # OpenPBR, no subsurface
    'sss': dict(subsurface=1.0, radius=SKIN_RADIUS),         # skin radius
    'sss-white': dict(subsurface=1.0, radius=(0.001092, (1.0, 1.0, 1.0))),
    'sss-x5': dict(subsurface=1.0, radius=(0.00546, (1.0, 0.425, 0.389))),
    'sss-w05': dict(subsurface=0.5, radius=SKIN_RADIUS),
}
nid = [900000]


def node(t, value, vt=8):
    nid[0] += 1
    return f"<node id='{nid[0]}' type='{t}'>\n<attr name='value' type='{vt}'>{value}</attr>\n</node>"


def rgb(c):
    return node(33, ' '.join(f'{x:g}' for x in c))


def flt(x):
    return node(31, f'{x:g}', 6)


def material(name, colour, v):
    nid[0] += 1
    if v is None:
        pins = {'albedo': rgb(colour), 'specular': rgb((0, 0, 0)), 'roughness': flt(0)}
        t = 130
    else:
        r, scale = v['radius']
        pins = {'base': flt(1), 'baseColor': rgb(colour), 'specular': flt(0), 'roughness': flt(0.3),
                'subsurface': flt(v['subsurface']), 'subsurfaceColor': rgb(colour),
                'radius': node(6, f'{r:g} 0 0 0', 9), 'scale': rgb(scale), 'subsurfaceAnisotropy': node(6, '0 0 0 0', 9)}
        t = 505
    body = '\n'.join(f"<pin name='{k}'>\n{x}\n</pin>" for k, x in pins.items())
    return f"<node id='{nid[0]}' type='{t}' name='{name}'>\n<attr name='compatibilityVersion' type='2'>15050000</attr>\n{body}\n</node>"


def make(src, out):
    os.makedirs(out, exist_ok=True)
    ocs = os.path.join(out, 'in.ocs')
    subprocess.run([tool, 'get', src, ocs], check=True)
    base = open(ocs).read()
    for vname, v in VARIANTS.items():
        s = base
        for sph, col in zip(SPHERES, COLOURS):
            name = sph + '_Default'
            # The material's input pin: replace the node inside it.
            m = re.search(r"(<node id='\d+' type='20007' name='/Scene/Materials/" + re.escape(name) + r"'[^>]*>.*?<pin name='input'>\s*)(<node .*?</node>)(\s*</pin>\s*</node>)", s, re.S)
            if not m:
                sys.exit(f'no material {name}')
            # The input node ends where its pin closes: find the matching </node>.
            start, depth, i = m.start(2), 0, m.start(2)
            for t in re.finditer(r'<node\b|</node>', s[start:]):
                depth += 1 if t.group() == '<node' else -1
                if depth == 0:
                    end = start + t.end()
                    break
            s = s[:start] + material(name, col, v) + s[end:]
        path = os.path.join(out, f'SSSR-{vname}.ocs')
        open(path, 'w').write(s)
        subprocess.run([tool, 'set', src, path, os.path.join(out, f'SSSR-{vname}.orbx')], check=True)
        print(os.path.join(out, f'SSSR-{vname}.orbx'))


def measure(odir):
    sys.argv = [sys.argv[0], odir]
    exec(open(os.path.join(here, 'calib-compare-all.py')).read().split('boxes = sphere_boxes()')[0], globals())
    boxes = sphere_boxes()
    from PIL import Image
    lin = lambda p: (np.asarray(Image.open(p).convert('RGB'), float) / 255) ** 2.2
    med = lambda a, b: np.median(a[b[0]:b[1], b[2]:b[3]].reshape(-1, 3), 0)
    uni = lin(os.path.join(odir, 'SSSR-uni.png'))
    print('sphere (left to right): Universal linear rgb')
    for k, b in enumerate(boxes):
        print(f'  {k}: {med(uni, b).round(3)}')
    for vname in VARIANTS:
        p = os.path.join(odir, f'SSSR-{vname}.png')
        if vname == 'uni' or not os.path.exists(p):
            continue
        a = lin(p)
        print(f'{vname}: per sphere, rgb / Universal')
        for k, b in enumerate(boxes):
            print(f'  {k}: {(med(a, b) / np.maximum(med(uni, b), 1e-5)).round(3)}')


if __name__ == '__main__':
    if sys.argv[1] == 'make':
        make(sys.argv[2], sys.argv[3])
    else:
        measure(sys.argv[2])
