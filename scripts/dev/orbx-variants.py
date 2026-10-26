#!/usr/bin/env python3
"""Variants of an exported .orbx with material pins changed in its
project.ocs, so one Octane batch tests several hypotheses:

  python3 scripts/dev/orbx-variants.py <in.orbx> <out dir> <hypothesis>...

Each hypothesis writes <out dir>/<scene>__<hypothesis>.orbx; render them with
scripts/dev/octane-render.sh and compare with scripts/dev/calib-compare-all.py
(the Iray reference of <scene> serves every variant).

Hypotheses (by the Iray channels the variants stand for):
- `gloss-2r`: Iray Uber Glossy Reflectivity r → Universal specular 2·r
  (r = 0.5 → the default 1).
- `gloss-ior`: specular 1 and the dielectric IOR from F0 = 0.08·r
  (r = 0.5 → 1.5, 0 → 1: no reflection), as Iray's reflectivity reads.
- `gloss-uncapped`: specular 2·r without the cap at 1 (r 1 → 2), to see
  whether Universal takes a specular above 1.
- `gloss-coat`: the reflectivity above 0.5 (specular capped at 1) as a
  clear coating of weight 2·r − 1 at the glossy roughness.
- `transl-diffuse`: Iray translucency w on props as Universal's diffuse
  transmission: albedo × (1 − w), `transmission` albedo × w (Translucency
  Color = the base colour in Calib-SSS / P6), `transmissionType` 1
  (Diffuse; 3, Thin Wall (Diffuse), when thin-walled).
- `transl-thin`: the same, `transmissionType` 3 for all.
- `transl-dark`: no transmission, albedo × (1 − 0.67 w) (the measured loss).
- `transm-x2`: the export's transmission of non-thin-walled translucency
  doubled (≤ 1): Tc · min(2w, 1).
- `ior=<n>`: Universal's Dielectric IOR (`ior`) n on every material but
  the floor (Plane).
- `dl-ior`: the same for PBRSkin's Dual Lobe Specular Reflectivity.
- `twosided-x1`: two-sided emitters at the one-sided power (the export
  doubles it, `TWO_SIDED_POWER`, assuming Octane splits it over both faces).
The channel values come from the scene materials' names (Calib-Glossy /
Calib-PBRSkin label their spheres with them) via HYPOTHESES below.
"""
import math, os, re, subprocess, sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../..')
tool = os.path.join(root, 'target/release/examples/orbx_ocs')

# Material (prefix of its name in project.ocs) → the Iray reflectivity it
# was built with (Calib-Glossy G*, Calib-PBRSkin P*; see make-calib-scenes.py).
REFLECTIVITY = {
    'G2_refl_0_5': 0.5, 'G3_refl_0_25': 0.25, 'G4_refl_1': 1.0, 'G5_refl_0': 0.0,
    'G6_rough_0_1': 0.5, 'G7_rough_0_6': 0.5, 'G8_dual_lobe': 0.5,
    'P2_dual_lobe_0_25': 0.25, 'P3_dual_lobe_0_5': 0.5, 'P4_dual_lobe_w0_25': 0.25,
    'P6_Gladio_cloth': 0.25, 'P7_metal': 0.25, 'P8_Gladio_armour': 0.25,
}
# Calib-Glossy's G4 roughness as the export writes it (its `roughness` pin).
COAT_ROUGHNESS = os.environ.get('COAT_ROUGHNESS', '0.09')
# Material → (Translucency Weight, thin-walled) (Calib-SSS S*, Calib-PBRSkin P6 / P8).
TRANSLUCENT = {
    'S2_transl_0_5': (0.5, False), 'S3_transl_1': (1.0, False), 'S4_thin_walled': (0.5, True),
    'S5_red_transmit': (0.5, False), 'S6_sss_amount': (0.5, False), 'S7_chromatic': (0.5, False),
    'S8_short_scatter': (0.5, False), 'P6_Gladio_cloth': (0.5, False), 'P8_Gladio_armour': (0.85, False),
    'Leaf_2': (0.5, False), 'Leaf_3': (0.5, True),
}
next_id = [990000]


def value_node(typ, attr_type, value):
    next_id[0] += 1
    return (f"<node id='{next_id[0]}' type='{typ}'>\n"
            f"         <attr name='value' type='{attr_type}'>{value}</attr>\n        </node>")


def set_pin(ocs, material, pin, node_xml):
    """Set `pin` of the Universal material whose input is named
    /Scene/Materials/<material>…: replace the pin's node, or add the pin."""
    m = re.search(rf"name='/Scene/Materials/{re.escape(material)}[^']*'", ocs)
    if not m:
        return ocs
    start = ocs.index("<node", ocs.index("<pin name='input'>", m.end()))
    # The material node ends at its matching </node>.
    depth, i = 0, start
    for t in re.finditer(r"<node\b|</node>", ocs[start:]):
        depth += 1 if t.group() == '<node' else -1
        if depth == 0:
            end = start + t.end()
            break
    block = ocs[start:end]
    pin_re = re.compile(rf"(<pin name='{pin}'>\s*)<node\b.*?</node>(\s*</pin>)", re.S)
    if pin_re.search(block):
        # Only the pin's own node (no nested nodes inside the value nodes we set).
        block = pin_re.sub(lambda mm: mm.group(1) + node_xml + mm.group(2), block, count=1)
    else:
        at = block.index('\n', block.index("compatibilityVersion")) + 1
        block = block[:at] + f"       <pin name='{pin}'>\n        {node_xml}\n       </pin>\n" + block[at:]
    return ocs[:start] + block + ocs[end:]


def albedo_of(ocs, material, pin='albedo'):
    m = re.search(rf"name='/Scene/Materials/{re.escape(material)}[^']*'", ocs)
    if not m:
        return None
    a = re.search(rf"<pin name='{pin}'>\s*<node[^>]*>\s*<attr name='value' type='8'>([^<]*)</attr>", ocs[m.end():])
    return [float(x) for x in a.group(1).split()] if a else None


def ior_from(r):
    f0 = 0.08 * r
    s = math.sqrt(f0)
    return (1 + s) / (1 - s) if f0 > 0 else 1.0


def halve_double_sided(ocs):
    """Undo TWO_SIDED_POWER: halve the power of every emission that has
    doubleSided on (its power pin comes before doubleSided in the node)."""
    pat = re.compile(r"(<pin name='power'>\s*<node[^>]*>\s*<attr name='value' type='8'>)([0-9.eE+-]+)( 0 0</attr>"
                     r"(?:(?!<pin name='power'>).)*?<pin name='doubleSided'>\s*<node[^>]*>\s*<attr name='value' type='1'>1)", re.S)
    return pat.sub(lambda m: m.group(1) + f'{float(m.group(2)) / 2:.6g}' + m.group(3), ocs)


def apply(ocs, hyp):
    if hyp == 'twosided-x1':
        return halve_double_sided(ocs)
    if hyp.startswith('ior='):
        for mat in sorted(set(re.findall(r"name='/Scene/Materials/([^']*)'", ocs))):
            if not mat.startswith('Plane'):
                ocs = set_pin(ocs, mat, 'ior', value_node(6, 8, f"{hyp[4:]} 0 0"))
        return ocs
    if hyp == 'transm-x2':
        for mat, (w, thin) in TRANSLUCENT.items():
            t = None if thin else albedo_of(ocs, mat, 'transmission')
            if t:
                ocs = set_pin(ocs, mat, 'transmission', value_node(33, 8, ' '.join(f'{min(2 * c, 1):.6f}' for c in t)))
        return ocs
    if hyp.startswith('transl-'):
        for mat, (w, thin) in TRANSLUCENT.items():
            a = albedo_of(ocs, mat)
            if a is None:
                continue
            rgb = lambda k: ' '.join(f'{c * k:.6f}' for c in a)
            if hyp == 'transl-dark':
                ocs = set_pin(ocs, mat, 'albedo', value_node(33, 8, rgb(1 - 0.67 * w)))
                continue
            ocs = set_pin(ocs, mat, 'albedo', value_node(33, 8, rgb(1 - w)))
            ocs = set_pin(ocs, mat, 'transmission', value_node(33, 8, rgb(w)))
            ocs = set_pin(ocs, mat, 'transmissionType', value_node(57, 2, '3' if thin or hyp == 'transl-thin' else '1'))
        return ocs
    for mat, r in REFLECTIVITY.items():
        dual = mat.startswith(('P', 'G8'))
        if hyp in ('gloss-2r', 'gloss-uncapped') and not dual:
            share = 2 * r if hyp == 'gloss-uncapped' else min(2 * r, 1)
            ocs = set_pin(ocs, mat, 'specular', value_node(33, 8, ' '.join([f'{share:.4f}'] * 3)))
        elif hyp == 'gloss-coat' and not dual and r > 0.5:
            ocs = set_pin(ocs, mat, 'coating', value_node(33, 8, ' '.join([f'{2 * r - 1:.4f}'] * 3)))
            ocs = set_pin(ocs, mat, 'coatingRoughness', value_node(6, 8, f'{COAT_ROUGHNESS} 0 0'))
        elif (hyp == 'gloss-ior' and not dual) or (hyp == 'dl-ior' and dual):
            ocs = set_pin(ocs, mat, 'specular', value_node(33, 8, '1 1 1'))
            ocs = set_pin(ocs, mat, 'index4', value_node(6, 8, f'{ior_from(r):.4f} 0 0'))
    return ocs


src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
scene = os.path.basename(src)[:-5]
tmp = os.path.join(out, scene + '.ocs')
subprocess.run([tool, 'get', src, tmp], check=True)
ocs = open(tmp).read()
for hyp in sys.argv[3:]:
    v = apply(ocs, hyp)
    p = os.path.join(out, f'{scene}__{hyp}.ocs')
    open(p, 'w').write(v)
    subprocess.run([tool, 'set', src, p, os.path.join(out, f'{scene}__{hyp}.orbx')], check=True)
    os.remove(p)
    print(f'{scene}__{hyp}: {sum(1 for _ in re.finditer("990[0-9]{3}", v))} pins set, {"changed" if v != ocs else "unchanged"}')
os.remove(tmp)
