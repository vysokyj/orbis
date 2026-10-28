#!/usr/bin/env python3
"""Variants of an exported .orbx with material pins changed in its
project.ocs, so one Octane batch tests several hypotheses:

  python3 scripts/dev/orbx-variants.py <in.orbx> <out dir> <hypothesis>...

Each hypothesis writes <out dir>/<scene>__<hypothesis>.orbx (or
__<label>.orbx for `<label>~<hypothesis>`); render them with
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
- `daylight-model=<n>`: the Daylight environment's model (0 Preetham,
  1 Octane, 2 Nishita, 3 Hosek-Wilkie — the export's).
- `daylight=<model>,<power>,<sun>,<ground start>`: the Daylight
  environment's model, power, sunIntensity and groundStartAngle.
- `turbidity=<t>`: the Daylight environment's turbidity.
- `absorb-x<k>`: every absorption medium's scale × k.
- `thin-ior1`: thin-walled glass (transmissionType 2) at IOR 1.
- `dl-ior`: the same for PBRSkin's Dual Lobe Specular Reflectivity.
- `sunset=<g>`: the Daylight environment's sunset colour grey g (the
  Octane model's; Faithful sets it).
- `set=<material>.<pin>=<f|c|p><value>[+…]`: any Universal pin of the
  material named /Scene/Materials/<material>… — `f` a float, `c` a grey
  colour, `p` the texture on that pin at power × value, `x` the pin's
  colour, or its texture's power, × value (e.g.
  `set=Genesis_8_Female_Cornea.coating=c0+Genesis_8_Female_Cornea.roughness=f0.07`).
- `hair=<k>,<s>`: every Hair material (147) at albedo × k and specular s.
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
    pm = re.search(rf"<pin name='{pin}'>\s*", block)
    if pm:
        # The pin's node up to its matching </node> (a texture nests nodes).
        a, depth = pm.end(), 0
        for t in re.finditer(r"<node\b|</node>", block[a:]):
            depth += 1 if t.group() == '<node' else -1
            if depth == 0:
                b = a + t.end()
                break
        block = block[:a] + node_xml + block[b:]
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


def hair_pin_re(material, pin):
    """A colour pin of the Hair material node named `material` (the node,
    not a /Scene/Materials path: strands carry it inline)."""
    return re.compile(rf"(<node id='\d+' type='147' name='{re.escape(material)}'>(?:(?!</node>\s*</pin>\s*</node>).)*?"
                      rf"<pin name='{pin}'>\s*<node[^>]*>\s*<attr name='value' type='8'>)([^<]*)", re.S)


def albedo_of_node(ocs, material):
    return [float(x) for x in hair_pin_re(material, 'albedo').search(ocs).group(2).split()]


def set_hair_pin(ocs, material, pin, value):
    return hair_pin_re(material, pin).sub(lambda m: m.group(1) + value, ocs, count=1)


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
    if hyp.startswith('set='):
        for item in hyp[4:].split('+'):
            target, value = item.split('=', 1)
            mat, pin = target.rsplit('.', 1)
            kind, v = value[0], value[1:]
            if kind == 'f':
                ocs = set_pin(ocs, mat, pin, value_node(31, 6, v))
            elif kind == 'c':
                ocs = set_pin(ocs, mat, pin, value_node(33, 8, f'{v} {v} {v}'))
            elif kind == 'x':
                c = albedo_of(ocs, mat, pin)
                if c is not None:
                    ocs = set_pin(ocs, mat, pin, value_node(33, 8, ' '.join(f'{x * float(v):.6f}' for x in c)))
                    continue
                kind = 'p'
            if kind == 'p':
                m = re.search(rf"name='/Scene/Materials/{re.escape(mat)}[^']*'", ocs)
                at = ocs.index(f"<pin name='{pin}'>", m.end())
                pw = re.compile(r"(<pin name='power'>\s*<node[^>]*>\s*<attr name='value' type='6'>)([0-9.eE+-]+)")
                pm = pw.search(ocs, at)
                ocs = ocs[:pm.start()] + pm.group(1) + f'{float(pm.group(2)) * float(v):.6g}' + ocs[pm.end():]
        return ocs
    if hyp.startswith('sunset='):
        g = hyp.split('=')[1]
        return re.sub(r"(<node id='100219' type='33'>\s*<attr name='value' type='8'>)[^<]*", lambda m: m.group(1) + f'{g} {g} {g}', ocs)
    if hyp.startswith('hair='):
        k, s = (float(x) for x in hyp[5:].split(','))
        for mat in re.findall(r"<node id='\d+' type='147' name='([^']*)'", ocs):
            a = albedo_of_node(ocs, mat)
            ocs = set_hair_pin(ocs, mat, 'albedo', ' '.join(f'{c * k:.6f}' for c in a))
            ocs = set_hair_pin(ocs, mat, 'specular', ' '.join([f'{s:.6f}'] * 3))
        return ocs
    if hyp.startswith('absorb-x'):
        k = float(hyp[len('absorb-x'):])
        pat = r"(<node id='\d+' type='58'>\s*<pin name='scale'>\s*<node[^>]*>\s*<attr name='value' type='9'>)([0-9.eE+-]+)"
        return re.sub(pat, lambda m: m.group(1) + f'{float(m.group(2)) * k:.6g}', ocs)
    if hyp == 'thin-ior1':
        pat = r"(<pin name='ior'>\s*<node[^>]*>\s*<attr name='value' type='6'>)[^<]*(</attr>\s*</node>\s*</pin>\s*<pin name='transmissionType'>\s*<node[^>]*>\s*<attr name='value' type='2'>2)"
        return re.sub(pat, lambda m: m.group(1) + '1' + m.group(2), ocs)
    if hyp.startswith('turbidity='):
        return re.sub(r"(<node id='100212' type='6'>\s*<attr name='value' type='9'>)[^<]*", lambda m: m.group(1) + hyp.split('=')[1] + ' 0 0 0', ocs)
    if hyp.startswith('daylight='):
        model, power, sun, start = hyp.split('=')[1].split(',')
        val = lambda node, v: (rf"(<node id='{node}' type='\d+'>\s*<attr name='value' type='\d+'>)[^<]*", lambda m: m.group(1) + v)
        for node, v in ((100215, model), (100213, f'{power} 0 0 0'), (100214, f'{sun} 0 0 0')):
            ocs = re.sub(*val(node, v), ocs)
        pin = (f"       <pin name='groundStartAngle'>\n        <node id='100218' type='6'>\n"
               f"         <attr name='value' type='9'>{start} 0 0 0</attr>\n        </node>\n       </pin>\n")
        at = ocs.index("<pin name='sun_size'>")
        at = ocs.rindex('\n', 0, at) + 1
        return ocs[:at] + pin + ocs[at:]
    if hyp.startswith('daylight-model='):
        return re.sub(r"(<node id='100215' type='57'>\s*<attr name='value' type='2'>)\d+", lambda m: m.group(1) + hyp.split('=')[1], ocs)
    if hyp.startswith('ior='):
        for mat in sorted(set(re.findall(r"name='/Scene/Materials/([^']*)'", ocs))):
            if not mat.startswith('Plane'):
                ocs = set_pin(ocs, mat, 'ior', value_node(31, 6, hyp[4:]))
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
for arg in sys.argv[3:]:
    # `<label>~<hypothesis>` names the variant <scene>__<label>.
    label, _, hyp = arg.rpartition('~') if '~' in arg else (arg, '', arg)
    v = apply(ocs, hyp)
    p = os.path.join(out, f'{scene}__{label}.ocs')
    open(p, 'w').write(v)
    subprocess.run([tool, 'set', src, p, os.path.join(out, f'{scene}__{label}.orbx')], check=True)
    os.remove(p)
    print(f'{scene}__{label}: {sum(1 for _ in re.finditer("990[0-9]{3}", v))} pins set, {"changed" if v != ocs else "unchanged"}')
os.remove(tmp)
