#!/usr/bin/env python3
"""Which displacement does Octane use in a Mix (19) / Layered (143)
material (#144)? Rewrites Calib-Disp's P2 (white map 0..5 cm: +5 cm,
amount 0.05) in project.ocs and writes one .orbx per variant:

  python3 scripts/dev/orbx-disp-variants.py <Calib-Disp.orbx> <out dir> [wrap]

`wrap`: the Layered material wrapped in a Mix at amount 1 / 0 instead of
the first set. Needs `cargo build --release --example orbx_ocs`. Render
with scripts/dev/octane-render.sh and measure P2's width at row 575
(lab note 2026-11-13-displacement-in-mixes).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import os, re, subprocess, sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../..')
tool = os.path.join(subprocess.check_output([os.path.join(root, 'scripts/dev/target-dir.sh')], text=True).strip(), 'release/examples/orbx_ocs')
src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
tmp = os.path.join(out, 'src.ocs')
subprocess.run([tool, 'get', src, tmp], check=True)
ocs = open(tmp).read()

# P2's material input node and the Universal inside it.
m = re.search(r"(   <node id='\d+' type='20007' name='/Scene/Materials/P2_white_0__5_Default'[^\n]*\n.*?\n    <pin name='input'>\n)(.*?\n)(    </pin>\n   </node>\n)", ocs, re.S)
head, uni, tail = m.group(1), m.group(2), m.group(3)
dm = re.search(r"\n *<pin name='displacement'>\n.*?\n      </pin>", uni, re.S)
disp_pin = dm.group(0)
uni_nodisp = uni.replace(disp_pin, '')
disp_node = re.search(r"<pin name='displacement'>\n(.*)\n *</pin>$", disp_pin.strip('\n'), re.S).group(1)

nid = [900000]
def reid(xml):
    def f(_):
        nid[0] += 1
        return f"id='{nid[0]}'"
    return re.sub(r"id='\d+'", f, xml)

def U(amount=None):
    """The Universal, with its displacement at `amount` (m) or none."""
    if amount is None:
        return reid(uni_nodisp)
    return reid(uni.replace("<attr name='value' type='9'>0.05 0 0 0</attr>", f"<attr name='value' type='9'>{amount} 0 0 0</attr>"))

def D(amount):
    return reid(disp_node.replace("<attr name='value' type='9'>0.05 0 0 0</attr>", f"<attr name='value' type='9'>{amount} 0 0 0</attr>"))

def mix(m1, m2, disp=None, amount=0.5):
    nid[0] += 1
    s = f"<node id='{nid[0]}' type='19' name='mix'>\n<pin name='amount'>\n"
    nid[0] += 1
    s += f"<node id='{nid[0]}' type='31'><attr name='value' type='6'>{amount}</attr></node>\n</pin>\n"
    s += f"<pin name='material1'>\n{m1}</pin>\n"
    if m2 is not None:
        s += f"<pin name='material2'>\n{m2}</pin>\n"
    if disp is not None:
        s += f"<pin name='displacement'>\n{D(disp)}\n</pin>\n"
    return s + "</node>\n"

def layered(base):
    nid[0] += 1
    return (f"<node id='{nid[0]}' type='143' name='layered'>\n<attr name='pin_count' type='2'>0</attr>\n"
            f"<attr name='compatibilityVersion' type='2'>15050000</attr>\n<pin name='material1'>\n{base}</pin>\n</node>\n")

V = {
    'wrap1': lambda: mix(layered(U()), None, 0.05, 1.0),
    'wrap0': lambda: mix(layered(U()), None, 0.05, 0.0),
    'wrap1-both': lambda: mix(layered(U()), layered(U()), 0.05, 1.0),
} if sys.argv[3:] == ['wrap'] else {
    'base': lambda: U(0.05),
    # Displacement on the sub-materials only.
    'mix-sub': lambda: mix(U(0.05), U(0.05)),
    # Only on the first sub-material.
    'mix-m1': lambda: mix(U(0.05), U()),
    # Only on the Mix.
    'mix-top': lambda: mix(U(), U(), 0.05),
    # Both, the same: doubled?
    'mix-both': lambda: mix(U(0.05), U(0.05), 0.05),
    # Both, different: the Mix 5 cm, the subs 10 cm.
    'mix-conflict': lambda: mix(U(0.10), U(0.10), 0.05),
    # A Mix in a Mix: outer 5 cm, inner 10 cm.
    'mix-nested': lambda: mix(mix(U(), U(), 0.10), U(), 0.05),
    # Layered: base Universal with it (no own pin).
    'lay-base': lambda: layered(U(0.05)),
    # Layered over a Mix that carries it.
    'lay-mix': lambda: layered(mix(U(), U(), 0.05)),
}
for name, make in V.items():
    s = ocs.replace(head + uni + tail, head + make() + tail)
    p = os.path.join(out, f'{name}.ocs')
    open(p, 'w').write(s)
    subprocess.run([tool, 'set', src, p, os.path.join(out, f'Calib-Disp__{name}.orbx')], check=True)
    os.remove(p)
os.remove(tmp)
print('\n'.join(sorted(os.listdir(out))))
