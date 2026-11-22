#!/usr/bin/env python3
"""ORBIS strand-hair calibration scenes (#79): straight synthetic strands,
one light, the Uber gloss on them one part at a time.

  python3 scripts/dev/make-strand-scenes.py [<scenes dir>]

Built from calib/scenes/Calib-L-Distant-45.duf (camera at twice its focal
length, tone mapping, the 45 degree sun, Scene Only) with its floor, cubes
and sphere removed: the
objects hang in a black void, so each one's light is its own and the
strands' silhouettes are masks. Every geometry is written into the scene
(DSON `geometry_library`, no content needed):

- row A (y 105): one thick strand each, a polyline of 7 points along x,
  Line Start / End Width 20 - a tube big enough to see where across it the
  highlight sits, and (in the .mi) how wide and how many-sided DAZ
  tessellates a line for Iray;
- row B (y 75): the same tubes as polygon meshes (24 sides, smooth, capped,
  UV u around and v along as DAZ's line tessellation has it), radius 1 cm:
  in Octane a curve against a mesh tube is the curve primitive's shading;
- row C (y 30 - 42): patches of 3000 thin strands (Morgan Hair's widths,
  0.2 / 0.1), horizontal - several strands deep, so light bounces between
  them as in hair;
- row D (y -45 - -5): the same patches with the strands vertical (along
  the light's plane: the reflection cone misses the camera).

Materials (all channels shown, as product content saves them):
  1 diffuse grey 0.5; 2 gloss isotropic (white, reflectivity 0.5,
  roughness 0.16, no diffuse); 3 the same anisotropic (0.735, rotation
  0.25, as Morgan's); 4 Morgan's gloss (glossiness mode, Glossy Color /
  Specular, effect 2, no diffuse); 5 Morgan's Strands material without its
  top coat and maps (diffuse, translucency, overlay, refraction, effects).
Rows A / B take 1 2 3 4, rows C / D 1 3 4 5.

The -Sparse scenes hold only rows E / F: patches thin enough that the
background shows through, so the reference's alpha is the strands' coverage
and follows their width (rows C / D are several strands deep: alpha 0.93 -
0.99, saturated). Diffuse grey, one width along the strand:
- row E (at row C's place): 300 horizontal strands each, Line Width 0.1 /
  0.2 / 0.4 / 0.8 (sub-pixel; 0.25 / 0.5 / 1 / 2 strand widths per width of
  patch);
- row F (at row D's): 40 vertical strands of 0.75 / 1.5 / 3 (about a pixel
  and wider), and row E's 300 x 0.2 turned vertical.
Each width is twice the one before: the export's curves at k x the width
on one patch are what 1 x would draw on the next.

The -Hair scenes carry DAZ's Blended Dual Lobe Hair shader (the free dForce
Starter Essentials; the scene only references it) one part at a time. Its
MDL (`dual_lobe_hair.mdl`) is a highlight glossy (Highlight Weight) over a
hair-coloured glossy (Glossy Layer Weight) over a diffuse *transmission*
in the Transmission Color - no hair BSDF. Rows: G one thick strand (Line
Width 20), H 300 sparse strands of 0.2 (single scattering), I 3000 of 0.15
(many strands deep), J row H's vertical. Columns, set `blond` (Calib-Hair's
dForce Corporate Hair values): 1 the whole shader, 2 its hair gloss alone
(transmission black, no highlight), 3 its transmission alone, 4 the
highlight lobe alone (weight 1). Set `white` (-HairW): 1 white transmission
alone, 2 transmission 0.5 (linear) alone, 3 a white gloss alone (weight 1,
black transmission), 4 white gloss 0.5 over white transmission.
The -UberT scenes hold the same rows with Iray Uber standing in for the
shader's parts (materials 6 - 9: Translucency Weight 1 white / 0.5 / blond,
and a glossy layer 0.25 over the blond): the export already writes Uber on
strands as a Universal material, so they show what a Universal on curves
does with a diffuse transmission. The -UberG scenes do that for the
shader's glossy lobe (materials 10 - 13, weighted mixing: a white gloss
alone, anisotropy 1 along the hair shader's tangent / along Uber's own /
isotropic, and 0.5 of it over a white translucency): Uber renders reliably
where the hair shader's module fails to re-import (#94).
The -UberM scenes take Morgan's Strands material apart (materials 5, 14 -
16: whole, without its gloss, without its translucency, without its
refraction) on fibres and on ribbons (-T2, Morgan's shape), sun and dome.
-UberA-Dome / -UberA-T2-Dome (#161): plain diffuse at 0.2 / 0.5 / 0.85
and 0.85 with translucency 0.48 (materials 17, 1, 18, 19) on fibres and
ribbons under the dome: how the albedo moves Octane's curves against Iray.
-UberM-Edge-T2 (-Dome) put the same strands in the same places with each
ribbon turned 65 degrees about its strand (node and geometry turned against
each other), as Calib-Hair-Teens shows Morgan's ribbons (#99).
-Cutout-Dome / -Cutout2-Dome (#202): grey 0.5 fibres under the dome with a
Cutout Opacity map (ours, written into My Library's Runtime/Textures/ORBIS):
-Cutout none, opaque for u < 0.5, for v < 0.5, but for v 0.75 - 1;
-Cutout2 a plain 0.5, the u and v maps inverted, all black. The strands'
UV set is all 0, so the first maps all read opaque there and the second
transparent: which coordinate Iray's fibres read a cutout map at.
-Coat-Dome / -Coat-T2-Dome (#215): Calib-Hair-Pixie's white top coat
(weight 1, reflectivity mode 0.5, roughness 0.2) alone over black, over
grey 0.5 and dark 0.2 (the bases of -UberA-Dome), and the same curve as a
glossy layer over the grey (materials 27 - 30), fibres and ribbons.
-CoatArc-HDRI / -CoatArc-T2-HDRI (#215): the coat on bent strands (half
circles, 18 points each) under DAZ's default dome (DTHDR-RuinsB, not
drawn) at the hair-set scenes' exposure: rows G one thick arc, H 300, I
3000 over the top (toward the camera, up, away), J 3000 facing the
camera; dark 0.2 bare, dark + coat, coat over black, coat over grey.
-PixieArc-HDRI / -PixieArc-T2-HDRI (#215): the same arcs with Calib-Hair-
Pixie's Hair01 material (its values; Pixie's widths 0.05 / 0.02, 12000
strands in I / J): whole, + coat, without translucency and refraction,
that + coat (materials 31 - 34).
-GlossLobeArc-HDRI / -GlossLobeArc-T2-HDRI (#220): the -LobeArc arcs with
Pixie's material + a white glossy layer 0.3 (its partial refraction over
gloss over dual lobe), with / without the lobe, with / without its
translucency (materials 38 - 41).
-Pile-HDRI (#221): Pixie's pile taken apart on the -LobeArc arcs, every
row a small layer of arcs over the top (6 cm across, 1.5 cm deep, as
Pixie's hair over the head): G THIN widths at -LobeArc's optical density
(750 strands), H Pixie's widths 0.05 / 0.02 at Pixie's density (3000: the
same coverage per cm, strands 4 x thinner and 4 x as many), I = H over a
cap (a half cylinder just under the arcs, plain diffuse in Pixie's
colour, as the hair cap and head under Pixie's strands), J = G over the
cap. Columns: Pixie opaque with / without its dual lobe, Pixie whole
(translucency, refraction) with / without it (materials 33, 35, 31, 42).

How DAZ hands a polyline to Iray depends on the hair's Line Tessellation
modifier (its .mi export reads the Viewport sides, the render the Render
sides; both are set alike here): none -> Iray `hair` (cubic B-spline fibres,
radius = Line Width / 20 cm); 2 sides -> flat ribbons one Line Width (mm)
wide, the normal the object's +z made perpendicular to the strand (-y for a
strand along z); 3 sides -> flat-shaded triangular prisms. Morgan Hair (#52)
carries 2 sides, dForce Corporate (Calib-Hair) 3. The modifier only runs on
a figure, so tessellated strands are written as figure nodes.

Scenes: Calib-Strand (the template's sun, fibres), Calib-Strand-Side (the
sun turned 60 degrees about y: the strands' cone), Calib-Strand-Dome (no
sun, the HDRI dome of Calib-Aniso, not drawn), Calib-Strand-T2 / -T3 (the
sun, ribbons / prisms), Calib-Strand-T2-Dome (ribbons under the dome, as
Morgan's hair in Calib-Hair-Teens), Calib-Strand-Sparse / -Sparse-T2 /
-Sparse-T3 (rows E / F as fibres / ribbons / prisms), Calib-Strand-Hair /
-HairW (the hair shader's sets as fibres), -Hair-T3 / -HairW-T3 (as prisms,
dForce Corporate Hair's shape), -Hair-Back / -Hair-T3-Back (set `blond`,
the sun turned behind the strands: what comes through them), -UberT /
-UberT-T3 (Uber's translucency on the same rows), -UberG / -UberG-T3
(Uber's weighted gloss on them; -Dome / -Side / -Back: under the dome, the
sun turned 60 and 180 degrees). Existing scenes are kept (--force
rewrites them); written to "My Library/Scenes" and copied into
calib/scenes. --tess-probe <dir> writes row A at several sides into <dir>
(scratch, for the .mi). --mohawk <DAZ library> writes Calib-Mi-Mohawk-T2
(the free dForce Mohawk at sides 2, for its .mi; My Library only).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import copy, gzip, json, math, os, random, shutil, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera, duf_io

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CALIB = os.path.join(REPO, 'calib', 'scenes')
_opt_vals = {sys.argv[i + 1] for i, a in enumerate(sys.argv[:-1]) if a in ('--mohawk', '--tess-probe')}
out_dir = next((a for a in sys.argv[1:] if not a.startswith('--') and a not in _opt_vals), None) or '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
FORCE = '--force' in sys.argv


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def channels_of(entry, typ):
    return next(e for e in entry['extra'] if e.get('type') == typ)['channels']


def set_channel(chs, cid, value):
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    if cid == 'Top Coat Reflectivity':  # the template's material doesn't save it
        chs.append({'channel': {'id': cid, 'type': 'float', 'current_value': value}})
        return
    raise KeyError(cid)


base = load(os.path.join(CALIB, 'Calib-L-Distant-45.duf'))
dome_src = load(os.path.join(CALIB, 'Calib-Aniso.duf'))
# DAZ's default dome (DTHDR-RuinsB, its sun in it) and exposure, as the
# hair-set scenes carry them (make-hair-set-scenes.py, Calib-Hair-Pixie-T1).
hdri_src = load(os.path.join(CALIB, 'Calib-Hair-Pixie-T1.duf'))
# Pixie's Hair01 material (the free G9 Base dForce Pixie Hair, as the scene
# saves it): its values, not its maps (it has none), for -PixieArc.
PIXIE = {c['channel']['id']: c['channel']['current_value']
         for m in hdri_src['material_library'] if m['id'] == 'Hair01'
         for e in m['extra'] if e.get('type') == 'studio_material_channels' for c in e['channels']
         if 'current_value' in c['channel'] and not c['channel'].get('image_file')}
PIXIE_BASE = [0.3098039, 0.2352941, 0.2]   # its Diffuse Color
PIXIE_WIDTHS = (PIXIE['Line Start Width'], PIXIE['Line End Width'])

KEEP = {'Camera', 'Tonemapper Options', 'Environment Options', 'DistantLight'}
template_node = copy.deepcopy(next(l for l in base['node_library'] if l['id'] == 'pCube(1120000)1'))
template_geom = copy.deepcopy(next(g for g in base['geometry_library'] if g['id'] == 'geometry-3'))
template_mat = copy.deepcopy(next(m for m in base['material_library'] if m['id'] == 'Default'))
for c in channels_of(template_mat, 'studio_material_channels'):
    c['channel'].pop('visible', None)  # shown, as product content saves them (#52)

# --- materials -----------------------------------------------------------
BLACK = [0.0, 0.0, 0.0]
PLAIN = {'Base Mixing': 0, 'Metallic Weight': 0.0, 'Dual Lobe Specular Weight': 0.0, 'Top Coat Weight': 0.0,
         'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Glossy Layered Weight': 0.0}
GLOSS = {'Glossy Layered Weight': 1.0, 'Glossy Color': [1.0, 1.0, 1.0], 'Glossy Reflectivity': 0.5,
         'Glossy Roughness': 0.16}
ANISO = {'Glossy Anisotropy': 0.7352941, 'Glossy Anisotropy Rotations': 0.25}
# Morgan Hair's Strands (Calib-Hair-Teens, local): its glossy layer ...
MORGAN_GLOSS = {'Base Mixing': 1, 'Glossy Layered Weight': 0.9795402, 'Share Glossy Inputs': True,
                'Glossy Color': [0.9411765, 0.9137255, 0.8196079], 'Glossy Color Effect': 2,
                'Glossy Specular': [0.7215686, 0.7019608, 0.6156863], 'Glossiness': 0.8382353, **ANISO}
# ... and its base, without the top coat and the maps.
MORGAN_BASE = {'Diffuse Overlay Weight': 0.05, 'Diffuse Overlay Weight Squared': True,
               'Diffuse Overlay Color': [0.7843137, 0.7450981, 0.682353], 'Diffuse Overlay Color Effect': 2,
               'Translucency Weight': 0.0591133, 'Base Color Effect': 2,
               'Translucency Color': [0.8039216, 0.7647059, 0.6862745], 'Invert Transmission Normal': True,
               'SSS Reflectance Tint': [0.682353, 0.6666667, 0.6156863],
               'Refraction Index': 1.794118, 'Refraction Weight': 0.1061947, 'Abbe': 7.522124, 'Thin Walled': True}
# Weighted mixing, the gloss alone: white, dForce Corporate Hair's base_roughness.
# Calib-Hair-Pixie-*-coat's top coat (make-hair-set-scenes.py COAT, #215).
COAT = {'Top Coat Weight': 1.0, 'Top Coat Color': [1.0, 1.0, 1.0], 'Top Coat Layering Mode': 0,
        'Top Coat Reflectivity': 0.5, 'Top Coat Roughness': 0.2}
# A white glossy layer for Pixie's material (#220); Base Mixing 0 as Pixie.
PIXIE_GLOSS = {'Glossy Layered Weight': 0.3, 'Glossy Color': [1.0, 1.0, 1.0], 'Glossy Reflectivity': 0.5,
               'Glossy Roughness': 0.3}
WEIGHTED = {'Base Mixing': 2,'Diffuse Weight': 0.0, 'Glossy Weight': 1.0, 'Glossy Color': [1.0, 1.0, 1.0], 'Glossy Roughness': 0.298913}
MATERIALS = {
    1: ('diffuse', [0.5, 0.5, 0.5], {}),
    2: ('gloss iso', BLACK, GLOSS),
    3: ('gloss aniso', BLACK, {**GLOSS, **ANISO}),
    4: ('Morgan gloss', BLACK, MORGAN_GLOSS),
    5: ('Morgan nocoat', [0.854902, 0.8117647, 0.7294118], {**MORGAN_GLOSS, **MORGAN_BASE}),
    # Uber as the hair shader's parts (-UberT): a diffuse transmission alone
    # (Translucency Weight 1), and a glossy layer over it.
    6: ('translucent white', [1.0, 1.0, 1.0], {'Translucency Weight': 1.0, 'Translucency Color': [1.0, 1.0, 1.0]}),
    7: ('translucent half', [1.0, 1.0, 1.0], {'Translucency Weight': 1.0, 'Translucency Color': [0.7353570, 0.7353570, 0.7353570]}),
    8: ('translucent blond', [1.0, 1.0, 1.0], {'Translucency Weight': 1.0, 'Translucency Color': [0.8862745, 0.7960784, 0.7254902]}),
    9: ('gloss over translucent blond', [1.0, 1.0, 1.0],
        {'Translucency Weight': 1.0, 'Translucency Color': [0.8862745, 0.7960784, 0.7254902], 'Glossy Layered Weight': 0.25,
         'Glossy Color': [0.8235294, 0.7882353, 0.7176471], 'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.298913}),
    # Uber as the hair shader's glossy lobe (-UberG): weighted mixing is the
    # same `weighted_layer` of a `simple_glossy_bsdf` through
    # `anisotropy_conversion`; the hair shader hands it `texture_tangent_v`,
    # Uber `texture_tangent_u`, a quarter turn apart.
    10: ('weighted gloss aniso along v', BLACK, {**WEIGHTED, 'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.25}),
    11: ('weighted gloss aniso along u', BLACK, {**WEIGHTED, 'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.0}),
    12: ('weighted gloss iso', BLACK, {**WEIGHTED, 'Glossy Anisotropy': 0.0}),
    13: ('weighted gloss 0.5 over translucent white', [1.0, 1.0, 1.0],
         {**WEIGHTED, 'Diffuse Weight': 0.5, 'Glossy Weight': 0.5, 'Translucency Weight': 1.0, 'Translucency Color': [1.0, 1.0, 1.0],
          'Glossy Anisotropy': 1.0, 'Glossy Anisotropy Rotations': 0.25}),
    # Morgan's Strands (5) one part off at a time (-UberM).
    14: ('Morgan nocoat, no gloss', [0.854902, 0.8117647, 0.7294118], {**MORGAN_GLOSS, **MORGAN_BASE, 'Glossy Layered Weight': 0.0}),
    15: ('Morgan nocoat, no translucency', [0.854902, 0.8117647, 0.7294118], {**MORGAN_GLOSS, **MORGAN_BASE, 'Translucency Weight': 0.0}),
    16: ('Morgan nocoat, no refraction', [0.854902, 0.8117647, 0.7294118], {**MORGAN_GLOSS, **MORGAN_BASE, 'Refraction Weight': 0.0}),
    # Plain diffuse at a dark / a blond albedo, and the blond with Hudson
    # Hair's translucency (-UberA, #161: Octane's curves against Iray's
    # ribbons on darker and on blond store hairs; material 1 is the middle).
    17: ('diffuse dark', [0.2, 0.2, 0.2], {}),
    18: ('diffuse blond', [0.85, 0.85, 0.85], {}),
    19: ('diffuse blond translucent', [0.85, 0.85, 0.85], {'Translucency Weight': 0.48, 'Translucency Color': [0.85, 0.85, 0.85]}),
    # Grey diffuse under a Cutout Opacity map (-Cutout, #202): which of the
    # map's axes Iray's fibres read along the strand and which around it
    # (Iray programmer's manual 8.1.4: texture space 0 is x along the fibre,
    # y around it), and what a transparent band around the fibre costs.
    20: ('diffuse cutout u half', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'u-half'}),
    21: ('diffuse cutout v half', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'v-half'}),
    22: ('diffuse cutout v band', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'v-band'}),
    # -Cutout2: the first scene's maps all read opaque at uv (0, 0), the
    # corner every strand's UVs sit at (the generated UV set is all 0).
    # These read transparent there, and a plain 0.5 shows whether the
    # cutout reaches Iray's fibres at all.
    23: ('diffuse cutout 0.5', [0.5, 0.5, 0.5], {'Cutout Opacity': 0.5}),
    24: ('diffuse cutout u half inverted', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'u-half-inv'}),
    25: ('diffuse cutout v half inverted', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'v-half-inv'}),
    26: ('diffuse cutout black', [0.5, 0.5, 0.5], {('Cutout Opacity', 'image'): 'black'}),
    # Calib-Hair-Pixie's white top coat (make-hair-set-scenes.py COAT) on
    # plain strands (-Coat, #215): alone over black, over the grey 0.5 and
    # the dark 0.2 of -UberA-Dome (its columns 2 / 1 are these bases bare),
    # and the same curve as a glossy layer over the grey (Uber's Glossy is
    # `add_top_coat` in custom-curve mode reused: Iray should draw 30 as 28;
    # the export writes 30 on Universal's specular, the coat on its coating).
    27: ('top coat over black', BLACK, COAT),
    28: ('top coat over grey', [0.5, 0.5, 0.5], COAT),
    29: ('top coat over dark', [0.2, 0.2, 0.2], COAT),
    31: ('Pixie', PIXIE_BASE, {}),
    32: ('Pixie + top coat', PIXIE_BASE, COAT),
    33: ('Pixie opaque', PIXIE_BASE, {'Translucency Weight': 0.0, 'Refraction Weight': 0.0}),
    34: ('Pixie opaque + top coat', PIXIE_BASE, {'Translucency Weight': 0.0, 'Refraction Weight': 0.0, **COAT}),
    # Pixie's dual lobe on the arcs (#216): Pixie opaque (33) against the
    # same without its lobe, the lobe alone over black, and the lobe's curve
    # as a glossy layer over black. Uber's dual lobe is the *base* of a
    # custom_curve_layer under the diffuse (weighted by the curve at the
    # macro normal), the glossy layer its *layer* (at the half vector,
    # `libbsdf`): 35 against 38 shows which of the two Iray's fibres draw.
    35: ('Pixie opaque, no dual lobe', PIXIE_BASE, {'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Dual Lobe Specular Weight': 0.0}),
    36: ('Pixie dual lobe over black', BLACK, {'Translucency Weight': 0.0, 'Refraction Weight': 0.0}),
    37: ('Pixie dual lobe curve as a glossy layer over black', BLACK,
         {'Translucency Weight': 0.0, 'Refraction Weight': 0.0, 'Dual Lobe Specular Weight': 0.0,
          'Glossy Layered Weight': PIXIE['Dual Lobe Specular Weight'], 'Glossy Color': [1.0, 1.0, 1.0],
          'Glossy Reflectivity': PIXIE['Dual Lobe Specular Reflectivity'],
          'Glossy Roughness': 0.5 * (PIXIE['Specular Lobe 1 Roughness'] + PIXIE['Specular Lobe 2 Roughness'])}),
    # A glossy layer over Pixie's dual lobe under its partial refraction
    # (#220): the MDL lays the refraction (0.28) over the gloss over the
    # lobe, so the refraction Mix's base part keeps both. With and without
    # the lobe (its share under gloss + refraction), over Pixie's
    # translucency (the strand hairs, a Mix already) and without (one node
    # before #220, a G8M beard's case).
    38: ('Pixie + gloss', PIXIE_BASE, PIXIE_GLOSS),
    39: ('Pixie + gloss, no dual lobe', PIXIE_BASE, {**PIXIE_GLOSS, 'Dual Lobe Specular Weight': 0.0}),
    40: ('Pixie + gloss, no translucency', PIXIE_BASE, {**PIXIE_GLOSS, 'Translucency Weight': 0.0}),
    41: ('Pixie + gloss, no translucency, no dual lobe', PIXIE_BASE, {**PIXIE_GLOSS, 'Translucency Weight': 0.0, 'Dual Lobe Specular Weight': 0.0}),
    # Pixie's pile on the arcs (#221): the real hair's materials (31 and
    # its -nogloss twin) beside the opaque pair (33 / 35), and the cap the
    # strands lie on: plain diffuse in Pixie's colour (no Pixie channels).
    42: ('Pixie, no dual lobe', PIXIE_BASE, {'Dual Lobe Specular Weight': 0.0}),
    43: ('cap: diffuse in Pixie colour', PIXIE_BASE, {}),
    30: ('coat-like gloss over grey', [0.5, 0.5, 0.5], {'Glossy Layered Weight': 1.0, 'Glossy Color': [1.0, 1.0, 1.0],
                                                        'Glossy Reflectivity': 0.5, 'Glossy Roughness': 0.2}),
}
# The -Cutout maps (ours, 256 x 256, pure 0 / 1 so no gamma reads them
# differently), in My Library's Runtime/Textures/ORBIS: u-half opaque for
# u < 0.5 (the image's left half), v-half for v < 0.5 (its lower half),
# v-band opaque but for v 0.75 - 1 (its top quarter: a quarter of the
# fibre's circumference, Athena Hair's maps fade 15 - 35 % of v).
CUTOUT_MAPS = {name: f'/Runtime/Textures/ORBIS/calib-cutout-{name}.png' for name in ('u-half', 'v-half', 'v-band', 'u-half-inv', 'v-half-inv', 'black')}


def cutout_map(name):
    path = os.path.join(os.path.dirname(out_dir), CUTOUT_MAPS[name].lstrip('/'))
    if not os.path.exists(path):
        from PIL import Image
        import numpy as np
        yy, xx = (np.mgrid[0:256, 0:256] + 0.5) / 256
        u, v = xx, 1 - yy
        a = {'u-half': u < 0.5, 'v-half': v < 0.5, 'v-band': v < 0.75, 'u-half-inv': u >= 0.5, 'v-half-inv': v >= 0.5, 'black': u < 0}[name]
        os.makedirs(os.path.dirname(path), exist_ok=True)
        Image.fromarray(np.where(a, 255, 0).astype(np.uint8)).save(path)
    return CUTOUT_MAPS[name]
THICK = 20.0                     # Line Start / End Width of rows A
THIN = (0.2, 0.1)                # Morgan's
# -Pile rows (#221): strands, widths, over the cap. Pixie (dump of
# Calib-Hair-Pixie-T1): 174 040 strands of 4.4 cm, about 360 strands per
# cm2 across the hair layer; at its widths (0.035 mm mean) 1.3 strand
# widths per cm of path, as -LobeArc's 3000 THIN arcs (83 / cm2, 0.15 mm).
PILE_ACROSS, PILE_DEPTH, PILE_R = 6.0, 1.5, 8.0
PILE = {'G': (750, 'thin', False), 'H': (3000, 'pixie', False), 'I': (3000, 'pixie', True), 'J': (750, 'thin', True)}
TUBE_R = 1.0                     # cm, rows B


# --- geometry ------------------------------------------------------------
def line_strand(x0, x1, y, z, n=7):
    return [[x0 + (x1 - x0) * k / (n - 1), y, z] for k in range(n)]


def patch(seed, count, axis):
    """`count` straight strands, 50 cm (horizontal) / 40 cm (vertical)
    long, random over a 12 x 3 cm cross-section, around the origin."""
    rnd = random.Random(seed)
    out = []
    for _ in range(count):
        a, b = rnd.uniform(-6, 6), rnd.uniform(-1.5, 1.5)
        if axis == 'x':
            out.append(line_strand(-25, 25, a, b, 8))
        else:
            out.append([[a, -20 + 40 * k / 7, b] for k in range(8)])
    return out


def arc_patch(seed, count, plane, r=8.0, across=12.0, depth=3.0):
    """`count` strands bent into half circles of radius r +- 1.5 cm, 18
    points each (as Pixie's render hairs), spread 12 cm across: plane 'yz'
    arcs from the camera side over the top to the back (hair over a
    crown, seen from the front), 'xy' arcs facing the camera (the side of
    a head). count 1: one arc of radius r."""
    rnd = random.Random(seed)
    out = []
    for _ in range(count):
        a = rnd.uniform(-across / 2, across / 2) if count > 1 else 0.0
        rr = r + (rnd.uniform(-depth / 2, depth / 2) if count > 1 else 0.0)
        pts = []
        for k in range(18):
            t = math.pi * k / 17
            if plane == 'yz':
                pts.append([a, rr * math.sin(t) - r / 2, rr * math.cos(t)])
            else:
                pts.append([rr * math.cos(t), rr * math.sin(t) - r / 2 + a / 4, a / 4])
        out.append(pts)
    return out


def strands_geometry(gid, strands):
    g = copy.deepcopy(template_geom)
    verts, lines = [], []
    for s in strands:
        lines.append([0, 0] + list(range(len(verts), len(verts) + len(s))))
        verts.extend(s)
    g.update({'id': gid, 'name': gid,
              'vertices': {'count': len(verts), 'values': [[round(c, 4) for c in v] for v in verts]},
              'polylist': {'count': 0, 'values': []},
              'polyline_list': {'count': len(lines), 'segment_count': sum(len(l) - 3 for l in lines), 'values': lines},
              'default_uv_set': '#uv-' + gid})
    uv = {'id': 'uv-' + gid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(verts),
          'uvs': {'count': len(verts), 'values': [[0, 0]] * len(verts)}, 'polygon_vertex_indices': []}
    return g, uv


def half_tube_geometry(gid, length, r, cy, sides=24, rings=5):
    """The upper half of an open tube along x (y >= cy), its axis at
    (y cy, z 0): the cap under -Pile's arcs. Smooth, faces out."""
    g = copy.deepcopy(template_geom)
    verts, polys, uvs = [], [], []
    for i in range(rings):
        x = -length / 2 + length * i / (rings - 1)
        for j in range(sides + 1):
            t = -math.pi / 2 + math.pi * j / sides
            verts.append([x, cy + r * math.cos(t), r * math.sin(t)])
            uvs.append([j / sides, i / (rings - 1)])
    vid = lambda i, j: i * (sides + 1) + j
    for i in range(rings - 1):
        for j in range(sides):
            # around +x the ring runs y -> z (as tube_geometry): faces out
            polys.append([0, 0, vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j)])
    g.update({'id': gid, 'name': gid,
              'vertices': {'count': len(verts), 'values': [[round(c, 5) for c in v] for v in verts]},
              'polylist': {'count': len(polys), 'values': polys},
              'default_uv_set': '#uv-' + gid})
    g.pop('polyline_list', None)
    uv = {'id': 'uv-' + gid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(verts),
          'uvs': {'count': len(uvs), 'values': [[round(a, 5), round(b, 5)] for a, b in uvs]},
          'polygon_vertex_indices': []}
    return g, uv


def tube_geometry(gid, length=60.0, r=TUBE_R, sides=24, rings=7):
    """A capped tube along x, centred on the origin. UV u around (one seam
    column more), v along, as DAZ tessellates a line for Iray."""
    g = copy.deepcopy(template_geom)
    verts, polys, uvs, pvi = [], [], [], []
    for i in range(rings):
        x = -length / 2 + length * i / (rings - 1)
        for j in range(sides):
            t = 2 * math.pi * j / sides
            verts.append([x, r * math.cos(t), r * math.sin(t)])
    for i in range(rings):
        for j in range(sides + 1):
            uvs.append([j / sides, i / (rings - 1)])
    vid = lambda i, j: i * sides + j % sides
    uid = lambda i, j: i * (sides + 1) + j
    for i in range(rings - 1):
        for j in range(sides):
            f = len(polys)
            # outward: around +x the ring runs y -> z, so (i, j) (i+1, j) (i+1, j+1) (i, j+1) faces out
            polys.append([0, 0, vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j)])
            for v, u in ((vid(i, j), uid(i, j)), (vid(i, j + 1), uid(i, j + 1)),
                         (vid(i + 1, j + 1), uid(i + 1, j + 1)), (vid(i + 1, j), uid(i + 1, j))):
                if u != v:
                    pvi.append([f, v, u])
    # caps: a centre vertex and a triangle fan at each end
    for i, sign in ((0, -1), (rings - 1, 1)):
        c = len(verts)
        verts.append([-length / 2 if i == 0 else length / 2, 0.0, 0.0])
        uc = len(uvs)
        uvs.append([0.5, 0.5])
        for j in range(sides):
            a, b = vid(i, j), vid(i, j + 1)
            tri = [c, b, a] if sign < 0 else [c, a, b]
            f = len(polys)
            polys.append([0, 0] + tri)
            for v in tri:
                u = uc if v == c else uid(i, j if v == a else j + 1)
                if u != v:
                    pvi.append([f, v, u])
    g.update({'id': gid, 'name': gid,
              'vertices': {'count': len(verts), 'values': [[round(c, 5) for c in v] for v in verts]},
              'polylist': {'count': len(polys), 'values': polys},
              'default_uv_set': '#uv-' + gid})
    g.pop('polyline_list', None)
    uv = {'id': 'uv-' + gid, 'name': 'default', 'label': 'Default UVs', 'vertex_count': len(verts),
          'uvs': {'count': len(uvs), 'values': [[round(a, 5), round(b, 5)] for a, b in uvs]},
          'polygon_vertex_indices': pvi}
    return g, uv


def material(mid, k, widths):
    label, colour, over = MATERIALS[k]
    m = copy.deepcopy(template_mat)
    m['id'] = mid
    m['diffuse']['channel']['current_value'] = colour
    chs = channels_of(m, 'studio_material_channels')
    known = {c['channel']['id'] for c in chs}
    pixie = {c: v for c, v in PIXIE.items() if c in known and c not in ('Line Start Width', 'Line End Width')} if k in (31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42) else {}
    for cid, v in {**PLAIN, **pixie, **over, 'Line Start Width': widths[0], 'Line End Width': widths[1]}.items():
        if isinstance(cid, tuple):
            next(c['channel'] for c in chs if c['channel']['id'] == cid[0])['image_file'] = cutout_map(v)
        else:
            set_channel(chs, cid, v)
    return m, colour


# Objects: (id, label, row, x, material, kind)
X = [-112.5, -37.5, 37.5, 112.5]
OBJECTS = []
for col, k in enumerate((1, 2, 3, 4)):
    OBJECTS.append((f'A{k}', f'A{k} thick {MATERIALS[k][0]}', 105.0, X[col], k, 'thick'))
    OBJECTS.append((f'B{k}', f'B{k} tube {MATERIALS[k][0]}', 75.0, X[col], k, 'tube'))
for col, k in enumerate((1, 3, 4, 5)):
    OBJECTS.append((f'C{k}', f'C{k} strands {MATERIALS[k][0]}', 36.0, X[col], k, 'patch-x'))
    OBJECTS.append((f'D{k}', f'D{k} strands vertical {MATERIALS[k][0]}', -25.0, X[col], k, 'patch-y'))


# Sparse patches (rows E / F): id -> (strands, Line Width).
SPARSE = {}
for col, (w_e, (n_f, w_f)) in enumerate(zip((0.1, 0.2, 0.4, 0.8), ((40, 0.75), (40, 1.5), (40, 3.0), (300, 0.2)))):
    SPARSE[f'E{col + 1}'] = (300, w_e)
    SPARSE[f'F{col + 1}'] = (n_f, w_f)
    OBJECTS.append((f'E{col + 1}', f'E{col + 1} sparse 300 x {w_e}', 36.0, X[col], 1, 'patch-x'))
    OBJECTS.append((f'F{col + 1}', f'F{col + 1} sparse vertical {n_f} x {w_f}', -25.0, X[col], 1, 'patch-y'))


# Blended Dual Lobe Hair (rows G - J): Calib-Hair's dForce Corporate Hair.
HAIR_URL = '/data/Daz%203D/dForce%20Starter%20Essentials/Blended%20Dual%20Lobe%20Hair%20Shader/Blended%20Dual%20Lobe%20Hair.dsf#Blended%20Dual%20Lobe%20Hair'
BLOND = {'Hair Root Color': [0.8235294, 0.7882353, 0.7176471], 'Root Transmission Color': [0.8862745, 0.7960784, 0.7254902],
         'Highlight Root Color': [0.827451, 0.8156863, 0.6901961], 'Glossy Layer Weight': 0.25, 'base_roughness': 0.298913,
         'Highlight Weight': 0.04891304, 'highlight_roughness': 0.8206522, 'separation': 0.0,
         'Anisotropy': 1.0, 'Anisotropy Rotations': 0.0, 'Bump Strength': 0.0, 'Cutout Opacity': 1.0, 'strength': 1.0}
WHITE, HALF = [1.0, 1.0, 1.0], [0.7353570, 0.7353570, 0.7353570]   # 0.5 linear
HAIR_SETS = {
    'blond': [('whole', {}),
              ('gloss', {'Root Transmission Color': BLACK, 'Highlight Weight': 0.0}),
              ('transmission', {'Glossy Layer Weight': 0.0, 'Highlight Weight': 0.0}),
              ('highlight', {'Highlight Weight': 1.0})],
    'white': [('transmission white', {'Root Transmission Color': WHITE, 'Glossy Layer Weight': 0.0, 'Highlight Weight': 0.0}),
              ('transmission half', {'Root Transmission Color': HALF, 'Glossy Layer Weight': 0.0, 'Highlight Weight': 0.0}),
              ('gloss white', {'Hair Root Color': WHITE, 'Root Transmission Color': BLACK, 'Glossy Layer Weight': 1.0, 'Highlight Weight': 0.0}),
              ('gloss half over white', {'Hair Root Color': WHITE, 'Root Transmission Color': WHITE, 'Glossy Layer Weight': 0.5, 'Highlight Weight': 0.0})],
}
TIP = {'Hair Root Color': 'Hair Tip Color', 'Root Transmission Color': 'Tip Transmission Color', 'Highlight Root Color': 'Tip Highlight Color'}
for col in range(4):
    k = col + 1
    OBJECTS.append((f'G{k}', f'G{k} thick', 105.0, X[col], k, 'thick'))
    OBJECTS.append((f'H{k}', f'H{k} sparse', 75.0, X[col], k, 'patch-x'))
    OBJECTS.append((f'I{k}', f'I{k} dense', 36.0, X[col], k, 'patch-x'))
    OBJECTS.append((f'J{k}', f'J{k} sparse vertical', -25.0, X[col], k, 'patch-y'))
    SPARSE[f'H{k}'], SPARSE[f'I{k}'], SPARSE[f'J{k}'] = (300, 0.2), (3000, 0.15), (300, 0.2)


def hair_material(mid, gid, k, widths, hair):
    """A scene material on the Blended Dual Lobe Hair shader: root and tip
    alike, the set's column `k` over Calib-Hair's values."""
    values = {**BLOND, **HAIR_SETS[hair][k - 1][1]}
    values.update({TIP[c]: values[c] for c in TIP})
    values.update({'Line Start Width': widths[0], 'Line End Width': widths[1]})
    chans = [{'channel': {'id': c, 'type': 'float_color' if isinstance(v, list) else 'float', 'current_value': v, 'image': None}}
             for c, v in values.items()]
    return {'id': mid + '-i', 'url': HAIR_URL, 'geometry': '#' + gid + '-i', 'groups': ['Default'],
            'diffuse': {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': values['Hair Root Color']}},
            'uv_set': '#uv-' + gid,
            'extra': [{'type': 'studio/material/daz_brick'}, {'type': 'studio_material_channels', 'channels': chans}]}


def tessellation(d, oid, sides):
    """DAZ's Line Tessellation modifier on a strand node (as dForce hair and
    Morgan Hair carry it): without it DAZ hands Iray the polylines as Iray
    hair (`hair`, cubic B-spline fibres); with it as a `trilist` of
    `sides`-sided strips."""
    chans = lambda cur: [{'channel': {'id': f'{w} Line Tessellation Sides', 'type': 'int', 'label': f'{w} Line Tessellation Sides',
                                      'value': 1 if w == 'Render' else 0, 'current_value': cur,
                                      'min': -2147483648, 'max': 2147483647, 'step_size': 1},
                          'group': '/General/Line Tessellation'} for w in ('Viewport', 'Render')]
    mid = 'Line Tessellation-' + oid
    d.setdefault('modifier_library', []).append({
        'id': mid, 'name': 'Line Tessellation', 'parent': '#' + oid,
        'extra': [{'type': 'studio/modifier/line_tessellation'}, {'type': 'studio_modifier_channels', 'channels': chans(sides)}]})
    d['scene'].setdefault('modifiers', []).append({
        'id': mid + '-1', 'url': '#' + mid.replace(' ', '%20'), 'parent': '#' + oid + '-1',
        'extra': [{'type': 'studio/modifier/line_tessellation'},
                  {'type': 'studio_modifier_channels', 'channels': [{'channel': {'id': c['channel']['id'], 'type': 'int', 'current_value': c['channel']['current_value']}} for c in chans(sides)]}]})


def turned(g, axis, deg):
    """The geometry turned by -deg about its strands' axis (x or y), for a
    node turned by +deg: the strands stay where they were in the world, only
    the object's frame - and with it a ribbon's normal, the object's +z made
    perpendicular to the strand - turns (-Edge scenes)."""
    c, s = math.cos(math.radians(-deg)), math.sin(math.radians(-deg))
    def rot(v):
        x, y, z = v
        if axis == 'x':  # y -> z positive
            return [x, y * c - z * s, y * s + z * c]
        return [x * c + z * s, y, -x * s + z * c]  # about y: z -> x positive
    g['vertices']['values'] = [[round(q, 4) for q in rot(v)] for v in g['vertices']['values']]
    return g


def build(name, sun_y=None, dome=False, tess=None, rows='ABCD', save_dir=None, hair=None, mats=None, turn=None, arcs=False, hdri=False):
    d = copy.deepcopy(base)
    d['asset_info']['id'] = f'/Scenes/{name}.duf'
    d['asset_info']['contributor'] = {'author': '', 'email': '', 'website': ''}
    d['node_library'] = [l for l in d['node_library'] if l['id'] in KEEP]
    d['scene']['nodes'] = [n for n in d['scene']['nodes'] if n['url'].lstrip('#').replace('%20', ' ') in KEEP]
    d['geometry_library'], d['uv_set_library'], d['material_library'], d['scene']['materials'] = [], [], [], []
    # Twice the template's focal length (65 mm): the rows fill the frame.
    cam = next(l for l in d['node_library'] if l['id'] == 'Camera')
    set_channel(channels_of(cam, 'studio_node_channels'), 'Focal Length', 130.0)
    if sun_y is not None:
        light = next(l for l in d['node_library'] if l['id'] == 'DistantLight')
        next(c for c in light['rotation'] if c['id'] == 'y')['current_value'] = sun_y
    if dome:
        d['node_library'] = [l for l in d['node_library'] if l['id'] not in ('DistantLight', 'Environment Options', 'Tonemapper Options')]
        d['scene']['nodes'] = [n for n in d['scene']['nodes'] if not n['id'].startswith(('DistantLight', 'Environment', 'Tonemapper'))]
        for lid in ('Environment Options', 'Tonemapper Options'):
            d['node_library'].append(copy.deepcopy(next(l for l in dome_src['node_library'] if l['id'] == lid)))
            d['scene']['nodes'].append(copy.deepcopy(next(n for n in dome_src['scene']['nodes'] if n['id'] == lid + '-1')))
        env = next(l for l in d['node_library'] if l['id'] == 'Environment Options')
        set_channel(channels_of(env, 'studio_node_channels'), 'Draw Dome', False)
        d['image_library'] = copy.deepcopy(dome_src['image_library'])
    if hdri:  # DAZ's default dome, not drawn (the strands' silhouettes stay masks)
        d['node_library'] = [l for l in d['node_library'] if l['id'] not in ('DistantLight', 'Environment Options', 'Tonemapper Options')]
        d['scene']['nodes'] = [n for n in d['scene']['nodes'] if not n['id'].startswith(('DistantLight', 'Environment', 'Tonemapper'))]
        for lid in ('Environment Options', 'Tonemapper Options'):
            d['node_library'].append(copy.deepcopy(next(l for l in hdri_src['node_library'] if l['id'] == lid)))
            d['scene']['nodes'].append(copy.deepcopy(next(n for n in hdri_src['scene']['nodes'] if n['id'] == lid + '-1')))
        env = next(l for l in d['node_library'] if l['id'] == 'Environment Options')
        set_channel(channels_of(env, 'studio_node_channels'), 'Draw Dome', False)
        d['image_library'] = [copy.deepcopy(i) for i in hdri_src['image_library'] if 'RuinsB' in i['id']]
    for oid, label, y, x, k, kind in OBJECTS:
        if oid[0] not in rows:
            continue
        gid = 'geom-' + oid
        if mats:
            k = mats[k - 1]
        backing = False
        if arcs == 'pile':
            count, wkind, backing = PILE[oid[0]]
            g, uv = strands_geometry(gid, arc_patch(ord(oid[0]) * 10 + k, count, 'yz', r=PILE_R, across=PILE_ACROSS, depth=PILE_DEPTH))
            widths = PIXIE_WIDTHS if wkind == 'pixie' else THIN
            label = f'{oid} pile {count} x {wkind}' + (' over cap' if backing else '')
        elif arcs and kind != 'tube':
            # rows G / H / I: one thick arc, 300 sparse, 3000 dense over the
            # top; J: 3000 dense facing the camera
            count = {'G': 1, 'H': 300, 'I': 3000, 'J': 3000}[oid[0]]
            if arcs == 'pixie' and oid[0] in 'IJ':
                count = 12000  # Pixie's strands are a quarter as wide as THIN
            g, uv = strands_geometry(gid, arc_patch(ord(oid[0]) * 10 + k, count, 'xy' if oid[0] == 'J' else 'yz'))
            widths = (THICK, THICK) if oid[0] == 'G' else PIXIE_WIDTHS if arcs == 'pixie' else THIN
        elif kind == 'thick':
            g, uv = strands_geometry(gid, [line_strand(-30, 30, 0, 0)])
            widths = (THICK, THICK)
        elif kind == 'tube':
            g, uv = tube_geometry(gid)
            widths = THIN
        elif oid in SPARSE:
            count, w = SPARSE[oid]
            g, uv = strands_geometry(gid, patch(ord(oid[0]) * 10 + int(oid[1]), count, kind[-1]))
            widths = (w, w)
        else:
            g, uv = strands_geometry(gid, patch(ord(kind[-1]) * 10 + k, 3000, kind[-1]))
            widths = THIN
        axis = 'y' if kind == 'patch-y' else 'x'
        if turn and kind != 'tube':
            g = turned(g, axis, turn)
        d['geometry_library'].append(g)
        d['uv_set_library'].append(uv)
        nl = copy.deepcopy(template_node)
        nl.update({'id': oid, 'name': oid, 'label': label})
        if turn and kind != 'tube':
            next(c for c in nl['rotation'] if c['id'] == axis)['current_value'] = turn
        if tess is not None and kind != 'tube':
            nl['type'] = 'figure'  # DAZ runs a modifier stack on figures only
        for c in nl['translation']:
            c['current_value'] = {'x': x, 'y': y, 'z': 0.0}[c['id']]
        d['node_library'].append(nl)
        d['scene']['nodes'].append({
            'id': oid + '-1', 'url': '#' + oid, 'name': oid, 'label': label,
            'geometries': [{'id': gid + '-i', 'url': '#' + gid, 'name': 'shape', 'label': 'shape', 'type': 'polygon_mesh'}],
            'extra': [{'type': 'studio_node_channels', 'version': 5}]})
        if tess is not None and kind != 'tube':
            tessellation(d, oid, tess)
        if hair:
            d['scene']['materials'].append(hair_material('Mat-' + oid, gid, k, widths, hair))
            continue
        m, colour = material('Mat-' + oid, k, widths)
        for c in channels_of(m, 'studio_material_channels'):
            v = c['channel'].get('image_file')
            images = d.setdefault('image_library', [])
            if v and not any(i['id'] == v for i in images):
                images.append({'id': v, 'name': os.path.basename(v), 'map_gamma': 0, 'map': [{'url': v, 'label': os.path.basename(v)}]})
        d['material_library'].append(m)
        d['scene']['materials'].append({
            'id': 'Mat-' + oid + '-i', 'url': '#Mat-' + oid, 'geometry': '#' + gid + '-i', 'groups': ['Default'],
            'diffuse': {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}},
            'uv_set': '#uv-' + gid, 'extra': [{'type': 'studio/material/uber_iray', 'version': '1.1.0.0'}]})
        if backing:  # the cap just under the arcs' inner radius
            bid = oid + 'cap'
            bg, buv = half_tube_geometry('geom-' + bid, PILE_ACROSS + 2.0, PILE_R - PILE_DEPTH / 2 - 0.05, -PILE_R / 2)
            d['geometry_library'].append(bg)
            d['uv_set_library'].append(buv)
            nb = copy.deepcopy(template_node)
            nb.update({'id': bid, 'name': bid, 'label': oid + ' cap'})
            for c in nb['translation']:
                c['current_value'] = {'x': x, 'y': y, 'z': 0.0}[c['id']]
            d['node_library'].append(nb)
            d['scene']['nodes'].append({
                'id': bid + '-1', 'url': '#' + bid, 'name': bid, 'label': oid + ' cap',
                'geometries': [{'id': 'geom-' + bid + '-i', 'url': '#geom-' + bid, 'name': 'shape', 'label': 'shape', 'type': 'polygon_mesh'}],
                'extra': [{'type': 'studio_node_channels', 'version': 5}]})
            m, colour = material('Mat-' + bid, 43, THIN)
            d['material_library'].append(m)
            d['scene']['materials'].append({
                'id': 'Mat-' + bid + '-i', 'url': '#Mat-' + bid, 'geometry': '#geom-' + bid + '-i', 'groups': ['Default'],
                'diffuse': {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}},
                'uv_set': '#uv-geom-' + bid, 'extra': [{'type': 'studio/material/uber_iray', 'version': '1.1.0.0'}]})
    duf_camera.ensure(d)
    if save_dir:  # a scratch scene
        p = os.path.join(save_dir, name + '.duf')
        duf_io.save(p, d)
        return p
    p = os.path.join(out_dir, name + '.duf')
    if os.path.exists(p) and not FORCE:
        print('kept', p)
    else:
        duf_io.save(p, d)
        print(p)
    shutil.copy2(p, os.path.join(CALIB, name + '.duf'))


MOHAWK = 'People/Genesis 8 Female/Hair/Strand-Based Hair Samples/G8F dForce Hair Mohawk.duf'


def mohawk_scene(lib):
    """Calib-Mi-Mohawk-T2: the free dForce Mohawk preset (dForce Starter
    Essentials, on Genesis 8 Female) as a scene at Line Tessellation 2, with
    this file's camera: its .mi shows the ribbon frames of generated render
    hairs (#79). It carries the preset's own data, so it stays in My Library
    only (not calib/scenes); a headless DAZ crashed saving it itself."""
    d = load(os.path.join(lib, MOHAWK))
    d['asset_info']['type'] = 'scene'
    d['asset_info']['id'] = '/Scenes/Calib-Mi-Mohawk-T2.duf'
    d['node_library'] = [copy.deepcopy(next(l for l in base['node_library'] if l['id'] == 'Camera'))]
    d['scene']['nodes'].append(copy.deepcopy(next(n for n in base['scene']['nodes'] if n['url'].lstrip('#') == 'Camera')))
    for m in d['scene']['modifiers']:
        if m['id'] == 'Line Tessellation':
            m['extra'].append({'type': 'studio_modifier_channels', 'channels': [
                {'channel': {'id': f'{w} Line Tessellation Sides', 'type': 'int', 'current_value': 2}} for w in ('Viewport', 'Render')]})
    duf_camera.ensure(d)
    p = os.path.join(out_dir, 'Calib-Mi-Mohawk-T2.duf')
    duf_io.save(p, d)
    print(p)


if '--mohawk' in sys.argv:
    mohawk_scene(sys.argv[sys.argv.index('--mohawk') + 1])
    sys.exit(0)
if '--tess-probe' in sys.argv:
    # scratch: row A at each Render Line Tessellation Sides, for the .mi
    probe = sys.argv[sys.argv.index('--tess-probe') + 1]
    os.makedirs(probe, exist_ok=True)
    for t in (None, 0, 1, 2, 3, 6):
        print(build(f'Strand-Tess-{t}', tess=t, rows='A', save_dir=probe))
    sys.exit(0)
build('Calib-Strand')
build('Calib-Strand-Side', sun_y=60.0)
build('Calib-Strand-Dome', dome=True)
build('Calib-Strand-T2', tess=2)
build('Calib-Strand-T3', tess=3)
build('Calib-Strand-T2-Dome', dome=True, tess=2)
build('Calib-Strand-Sparse', rows='EF')
build('Calib-Strand-Sparse-T2', tess=2, rows='EF')
build('Calib-Strand-Sparse-T3', tess=3, rows='EF')
for _name, _kw in (('', {}), ('-T3', {'tess': 3})):
    build('Calib-Strand-Hair' + _name, rows='GHIJ', hair='blond', **_kw)
    build('Calib-Strand-HairW' + _name, rows='GHIJ', hair='white', **_kw)
    build('Calib-Strand-Hair' + _name + '-Back', rows='GHIJ', hair='blond', sun_y=180.0, **_kw)
    build('Calib-Strand-HairW' + _name + '-Dome', rows='GHIJ', hair='white', dome=True, **_kw)
    build('Calib-Strand-UberT' + _name, rows='GHIJ', mats=(6, 7, 8, 9), **_kw)
    build('Calib-Strand-UberG' + _name, rows='GHIJ', mats=(10, 11, 12, 13), **_kw)
    # The glossy lobe under other lights: DAZ's prisms all face one way.
    build('Calib-Strand-UberG' + _name + '-Dome', rows='GHIJ', mats=(10, 11, 12, 13), dome=True, **_kw)
    build('Calib-Strand-UberG' + _name + '-Side', rows='GHIJ', mats=(10, 11, 12, 13), sun_y=60.0, **_kw)
    build('Calib-Strand-UberG' + _name + '-Back', rows='GHIJ', mats=(10, 11, 12, 13), sun_y=180.0, **_kw)
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-UberM' + _name, rows='GHIJ', mats=(5, 14, 15, 16), **_kw)
    build('Calib-Strand-UberM' + _name + '-Dome', rows='GHIJ', mats=(5, 14, 15, 16), dome=True, **_kw)
# Morgan's ribbons as Calib-Hair-Teens shows them, about 65 degrees from
# face-on (#99): the same strands in the world as -UberM-T2, only the
# ribbons' normal turned about each strand.
build('Calib-Strand-UberM-Edge-T2', rows='GHIJ', mats=(5, 14, 15, 16), tess=2, turn=65.0)
build('Calib-Strand-UberM-Edge-T2-Dome', rows='GHIJ', mats=(5, 14, 15, 16), tess=2, turn=65.0, dome=True)
# Plain diffuse by albedo under the dome (#161): fibres (Octane's curves'
# own shape in Iray) and ribbons (what DAZ hands Iray for Line
# Tessellation 2, Faithful's reference for ribbon hair).
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-UberA' + _name + '-Dome', rows='GHIJ', mats=(17, 1, 18, 19), dome=True, **_kw)
# Cutout Opacity maps on fibres under the dome (#202): grey 0.5 without a
# map (Calib-Strand-UberA-Dome's column 2 material, here in column 1), then
# the u-half, v-half and v-band maps.
build('Calib-Strand-Cutout-Dome', rows='GHIJ', mats=(1, 20, 21, 22), dome=True)
build('Calib-Strand-Cutout2-Dome', rows='GHIJ', mats=(23, 24, 25, 26), dome=True)
# Pixie's top coat on fibres and ribbons under the dome (#215): Iray's coat
# adds 1.2 - 1.4 x on Pixie as fibres, 1.6 - 2.1 x as ribbons. The coat
# alone, over the two -UberA-Dome bases, and as a glossy layer.
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-Coat' + _name + '-Dome', rows='GHIJ', mats=(27, 28, 29, 30), dome=True, **_kw)
# The same coat on bent strands under DAZ's default dome and its sun, as
# Calib-Hair-Pixie is lit (#215): on Pixie Iray's coat gathers into sun
# glints, Octane's spreads into a sky sheen; on straight strands under
# Calib-Aniso's dome the two match. Columns: the dark base bare, with the
# coat, the coat alone, the coat over grey.
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-CoatArc' + _name + '-HDRI', rows='GHIJ', mats=(17, 29, 27, 28), arcs=True, hdri=True, **_kw)
# Pixie's own hair material on the bent strands at Pixie's widths (#215):
# whole and without its translucency / refraction (on fibres Iray drops
# every transmission), each bare and with the coat.
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-PixieArc' + _name + '-HDRI', rows='GHIJ', mats=(31, 32, 33, 34), arcs='pixie', hdri=True, **_kw)
# Pixie's dual lobe on the arcs under the same dome and sun (#216): Iray's
# lobe adds 1.22 - 1.55 x on Pixie, Octane's 1.04 - 1.14 x, every
# tessellation. Columns: Pixie opaque with / without its lobe, the lobe
# alone, its curve as a glossy layer (MDL layer vs base, above). THIN
# widths as -CoatArc (-PixieArc's 12000 strands at Pixie's widths crashed
# the server's GPU, #217).
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-LobeArc' + _name + '-HDRI', rows='GHIJ', mats=(33, 35, 36, 37), arcs=True, hdri=True, **_kw)
# A glossy layer over Pixie's dual lobe under its partial refraction
# (#220): Pixie + gloss with / without its lobe, over its translucency and
# without; THIN widths as -LobeArc (#217).
for _name, _kw in (('', {}), ('-T2', {'tess': 2})):
    build('Calib-Strand-GlossLobeArc' + _name + '-HDRI', rows='GHIJ', mats=(38, 39, 40, 41), arcs=True, hdri=True, **_kw)
# Pixie's pile taken apart (#221): on Calib-Hair-Pixie-T1 the side patch's
# lobe share is 1.36 x Iray's over a base 0.69 - 0.75 x, while -LobeArc's
# arcs match (1.02 - 1.04 x). Width alone (G / H: the same coverage per cm,
# Pixie's strands 4 x thinner), the cap under the strands (H / I, G / J),
# the real material beside the opaque one (columns 3 / 4 against 1 / 2).
build('Calib-Strand-Pile-HDRI', rows='GHIJ', mats=(33, 35, 31, 42), arcs='pile', hdri=True)
