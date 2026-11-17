#!/usr/bin/env python3
"""ORBIS calibration scenes that use our own MDL modules (#112).

  python3 scripts/dev/make-mdl-scenes.py [--force] [--root <dir>] [<library>]

Installs `calib/mdl/*.mdl` (our own modules, nothing of NVIDIA's or DAZ's)
into the content library under `<root>/orbis/` (default root
`shaders/iray`, the layout of DAZ's own `Contents/shaders/iray`), writes a
material file with one Shader Mixer brick material per module material
(`DzCustomMDLBrick` "MDL Callable" `orbis/<module>.mdl#<material>` into
`DzMDLSurfaceBrick`, the graph DAZ's NVIDIA MDL Examples use) to
`<library>/data/ORBIS/Calib/MDL Rim/MDL Rim.dsf`, and the scenes into
`<library>/Scenes`, copied to `calib/scenes`:

- Calib-Mi-MdlPath (.mi only): one sphere per candidate MDL search root
  (`<library>/shaders/iray`, `<library>`, `<library>/data`), which one DAZ
  resolves a custom module from.
- Calib-Furnace-MdlVersion: Calib-Furnace-Metal's spheres under the uniform
  dome, floor hidden, the same material at "mdl 1.0" and "mdl 1.3" side by
  side - the bare white lobe at r 0.1 and 0.4, the lobe under
  metal_generic's two weighted layers, a closed rough refractor (IOR 1.5,
  r 0.2). Iray reads simple_glossy_bsdf with a legacy behaviour in modules
  older than 1.3 (MDL SDK `set_simple_glossy_bsdf_legacy_enabled`); NVIDIA's
  examples are 1.0, DAZ's Uber 1.7.
- Calib-Furnace-MdlLobe: the "mdl 1.0" lobe at Calib-Furnace-Metal's eight
  roughnesses (its Uber spheres are the same lobe at 1.7).
- Calib-Furnace-MdlTransmit: the closed refractor at r 0.05 / 0.1 / 0.2 /
  0.4 in "mdl 1.0" and "mdl 1.3" (#116).
- Calib-Point-MdlAniso (#114): the same spheres without the dome, lit by a
  point light at the camera - the highlight's shape of the lobe at 1.0 /
  1.3: isotropic, anisotropic, and architectural's base:: roughness pair.
- Calib-Furnace-MdlTir (#117): the closed transmit lobe at r 0.2, IOR 1.2 /
  2.0, and thin-walled at r 0.2 / 0.4, each in "mdl 1.0" / "mdl 1.3" - the
  rings say whether the legacy lobe's loss falls towards the rim with the
  critical angle (total internal reflection) or is flat (a thin wall).
- Calib-Furnace-MdlResin (#117): resin's base lobe (reflect_transmit, IOR
  1.49) at r 0.25 / 0.5 / 0.75 / 1 in 1.0 / 1.3 - Calib-Furnace-Resin2
  U3 - U6's twins.
- Calib-Furnace-MdlArch (#117, second round): why NVIDIA's architectural
  transmit lobe loses more than the plain 1.0 lobe at its formula's
  roughness - the lobe with base::architectural_gloss_to_rough(refr_gloss
  0.4 / 0.2 / 0) (1.0, and 0.2 at 1.3), architectural's layer structure
  around it at that roughness and at the same value as a constant, thin
  walls (Calib-Furnace-ArchGlass G3 - G5 / -ArchGlass2 H4's twins).
- Calib-Furnace-MdlLegacy (#117, second round): the 1.0 transmit lobe at
  r 0.3 / 0.5 / 0.7 / 1 closed and 0.1 / 0.6 / 1 thin-walled, one 1.3 twin
  (r 0.5) - the per-event loss g(r, cos) above r 0.4.
- Calib-Furnace-MdlLegacy2 (#117, third round): the 1.0 transmit lobe at
  r 0.6 / 1 over the IOR - closed at 1.05 (two barely bent events, as a
  thin wall's), 1.2 / 2 at r 0.6, 2 at r 1 - and the 1.3 lobe's thin walls
  at r 0.6 / 1 and closed 1.05 r 1: whether the per-event loss depends on
  the bending (thin walls lose more per event than closed spheres at 1.5)
  or `libbsdf`'s thin model is off at high r.
- Calib-Furnace-MdlGeneric (#112, second round): what is left of
  metal_generic one piece at a time, each at 1.0 / 1.3 - the lobe's tint
  (0.65, r 0.4), anisotropy (B6's u 0.42 / v 0.35, default tangent_u), a
  varying roughness expression (value 0.4), and metal_generic's whole
  structure (weighted layers, file_bump_texture normal) at
  Calib-Furnace-Brushed B6's constants.
- Calib-Furnace-MdlTextured (#112, third round): G7 keeps M3's rim where
  B6 falls (B6 / G7 ~ 0.94 sqrt(cos theta_v)); what B6 has on top, one
  piece at a time on G7 (all "mdl 1.0"): a valid flat bump texture at
  factor 0 and at 0.4 (Iray's builtin bump path), the weight and the
  roughness u / v through base::file_texture on the flat maps, all of
  them, NVIDIA's metal_brushed itself at B6's inputs through our brick,
  the bump at factor 0 at "mdl 1.3", and the bare white r 0.4 lobe under
  that bump. The flat maps (calib-disp-grey / -white / -black from
  `<library>/Runtime/Textures/ORBIS`, make-calib-scenes.py's) are copied
  next to the modules.
- Calib-Furnace-MdlRoughTex (#112, fourth round): MdlTextured's H4 - G7
  with its roughness through base::file_texture - is B6's rim (0.93
  sqrt(cos theta_v) x G7). On the bare white lobe (MdlVersion M3 / M4):
  the roughness 0.4 through base::file_texture at 1.0 / 1.3, through a
  plain tex::lookup_float, a roughness varying by 1e-4 without a texture
  at 1.0 / 1.3, the tint through base::file_texture at a constant
  roughness, and the file_texture roughness at r 0.1 / 0.7.
- Calib-Point-MdlReflTrans (#191): Calib-Point-MdlAniso's light on resin's
  base lobe (scatter_reflect_transmit, IOR 1.49, closed, an opaque interior
  so only the front reflection is seen) at 1.0 / 1.3, r 0.354 / 0.5 / 0.7,
  with 1.3 twins at r' = r sqrt(2 / (2 + r^2)) (cos^(e' - 1) = cos^e):
  whether the transmitting legacy lobe is cos^e as the reflect-only one.
- Calib-Back-MdlTransmit (#191): the same spheres in architectural's
  transmit lobe (closed, IOR 1.5), the camera level with them and a spot
  light behind each on its line of sight (`back_lit`): the lobe through
  two rough refractions head-on.
- Calib-Point-ArchAniso (#114): that light on NVIDIA's Architectural
  preset (its reflection only) at glossiness 0 / 0.15 / 0.3, isotropic
  and anisotropic - the scene the export maps (Octane side).
- Calib-Point-UberGloss (#162): that light on plain Uber spheres (no MDL
  module of ours): a dielectric gloss over a black base at Iray r 0.25 /
  0.354 / 0.5 / 0.7 / 1 and a dark grey metal at r 0.354 / 0.5 / 0.7 - the
  Phong lobe's highlight head-on where the export's `phong_brdf` leaves
  Beckmann (dielectric alpha > 0.25), the metals the Beckmann control.
- Calib-Point-DualLobeBase-Grey / -Black / -Off, Calib-Point-DualLobeCurve-
  Grey / -Black and their Calib-Furnace-* twins (#193): Uber's dual lobe
  (no gloss) over a grey diffuse 0.5, over black, and the grey alone -
  what the lobe leaves of the diffuse per angle; P1 - P8 at Phong pins 0.3
  - 0.9 (W 1, F0 0.04), C1 - C8 at pin 0.448 over the curve's W / R and the
  two lobes' roughnesses / ratio. The point twins at EV 6.1 (the grey
  unclipped), the furnace ones at Calib-Furnace-Metal's exposure.

Calib-Mi-MdlPath's .mi (2026-11-13): only P1 resolves, and only once the
script maps `<library>/shaders/iray` as an MDL directory - DAZ searches its
own `shaders/iray` and its MDL directories (none set), not the content
directories. daz-export-mi.dsa and daz-render-queue.dsa map
`<content dir>/shaders/iray` for their session (addMDLDirectory).

Each module may be imported once per DAZ session (#94): the scenes use
different modules, but a second render of any needs a DAZ restart.
Scenes are kept when they exist (--force rewrites); the modules and the
material file are always rewritten.
"""
import copy, gzip, json, os, shutil, sys

if '-h' in sys.argv or '--help' in sys.argv:
    print(__doc__)
    sys.exit(0)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
args = [a for a in sys.argv[1:] if not a.startswith('--')]
FORCE = '--force' in sys.argv
ROOT = sys.argv[sys.argv.index('--root') + 1] if '--root' in sys.argv else 'shaders/iray'
if ROOT in args:
    args.remove(ROOT)
LIB = args[0] if args else '/Volumes/Data/DAZ 3D/Studio/My Library'
SCENES = os.path.join(LIB, 'Scenes')
DSF_URL = '/data/ORBIS/Calib/MDL%20Rim/MDL%20Rim.dsf'
DSF = os.path.join(LIB, 'data/ORBIS/Calib/MDL Rim/MDL Rim.dsf')
FURNACE_METAL = os.path.join(SCENES, 'Calib-Furnace-Metal.duf')


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def s(value):
    """A Shader Mixer settings node."""
    return {'type': 'settings', 'value': value}


def brick_material(mid, callable_):
    """A material_library entry: a custom MDL material straight into the
    MDL surface brick, no user parameters."""
    custom = s({'DAZStudioClassName': 'DzCustomMDLBrick',
                'Attributes': s({'BrickSettings': s({'BrickTitle': 'Custom MDL (1)', 'HideParams': False, 'PosX': -143, 'PosY': -312,
                                                     'BrickIsRightToLeft': True, 'Width': ['i', 200], 'Height': ['i', 120]}),
                                 'BrickVersion': 1,
                                 'BrickProperties': s({'MDL Callable': s({'Label': 'MDL Callable', 'Hidden': False, 'Path': '/General',
                                                                          'UserProperty': False, 'CanAnimate': False, 'Locked': False,
                                                                          'Manipulator': True, 'Favorite': False, 'GlobalFavorite': False,
                                                                          'Selected': False, 'AutoFollow': False, 'Value': callable_})})}),
                'Parameters': s({'Result': s({'Tokens': 'elr_Result_b1_t0', 'Label': 'Result'})})})
    surface = s({'DAZStudioClassName': 'DzMDLSurfaceBrick',
                 'Attributes': s({'BrickSettings': s({'BrickSet': s({'Name': 'shader_MDL_Surface_1', 'LastInvalidate': '0'}),
                                                      'BrickTitle': 'MDL Surface (2)', 'HideParams': False, 'PosX': -409, 'PosY': -276,
                                                      'BrickIsRightToLeft': True, 'Width': ['i', 200], 'Height': ['i', 135]}),
                                  'BrickVersion': 1, 'BrickProperties': s({})}),
                 'Parameters': s({'material': s({'Label': 'Material'})})})
    setup = s({'Bricks': s({'Brick_1': custom, 'Brick_2': surface}),
               'Connections': s({'Connection_0': s({'SourceBrick': 'Brick_2', 'TargetBrick': 'Brick_1',
                                                    'SourceParam': 'material', 'TargetParam': 'Result'})}),
               'BrickUserName': mid})
    return {'id': mid, 'extra': [{'type': 'studio/material/daz_brick', 'brick_settings': {'BrickSetup': setup}},
                                 {'type': 'studio_material_channels', 'channels': []}]}


# (scene, [(sphere label, material id, MDL callable)]); the module file of
# each callable comes from calib/mdl, installed under ROOT unless the
# callable's material id says otherwise (the path probe).
PATH_PROBE = [('P1 root shaders-iray', 'path_shaders_iray', 'orbis/orbis_path_a.mdl#probe', 'shaders/iray'),
              ('P2 root library', 'path_library', 'orbis/orbis_path_b.mdl#probe', ''),
              ('P3 root data', 'path_data', 'orbis/orbis_path_c.mdl#probe', 'data')]
VERSION = [('M1 v1.0 reflect r 0.1', 'v10_reflect_r01', 'orbis/orbis_rim_v10.mdl#reflect_r01'),
           ('M2 v1.3 reflect r 0.1', 'v13_reflect_r01', 'orbis/orbis_rim_v13.mdl#reflect_r01'),
           ('M3 v1.0 reflect r 0.4', 'v10_reflect_r04', 'orbis/orbis_rim_v10.mdl#reflect_r04'),
           ('M4 v1.3 reflect r 0.4', 'v13_reflect_r04', 'orbis/orbis_rim_v13.mdl#reflect_r04'),
           ('M5 v1.0 layered r 0.1', 'v10_layered_r01', 'orbis/orbis_rim_v10.mdl#layered_r01'),
           ('M6 v1.3 layered r 0.1', 'v13_layered_r01', 'orbis/orbis_rim_v13.mdl#layered_r01'),
           ('M7 v1.0 transmit r 0.2', 'v10_transmit_r02', 'orbis/orbis_rim_v10.mdl#transmit_r02'),
           ('M8 v1.3 transmit r 0.2', 'v13_transmit_r02', 'orbis/orbis_rim_v13.mdl#transmit_r02')]
# For the next step if "mdl 1.0" is the rim: the 1.0 lobe at
# Calib-Furnace-Metal's roughnesses (its Uber spheres are the 1.7 twin),
# and the closed refractor at four roughnesses in 1.0 and 1.3 (#116).
R_NAME = lambda r: ('%g' % r).replace('.', '')
LOBE = [(f'L{k + 1} v1.0 reflect r {r}', f'v10_lobe_r{R_NAME(r)}', f'orbis/orbis_lobe_v10.mdl#reflect_r{R_NAME(r)}')
        for k, r in enumerate((0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.7, 1.0))]
TRANSMIT = [(f'T{k + 1} v{v} transmit r {r}', f'v{v.replace(".", "")}_sweep_transmit_r{R_NAME(r)}',
             f'orbis/orbis_transmit_v{v.replace(".", "")}.mdl#transmit_r{R_NAME(r)}')
            for k, (v, r) in enumerate([(v, r) for v in ('1.0', '1.3') for r in (0.05, 0.1, 0.2, 0.4)])]
# #114: the highlight's shape under a point light at the camera - the same
# lobe at 1.0 / 1.3: isotropic r 0.2, anisotropic u 0.4 / v 0.1 (the same
# geometric mean), and the pair base::anisotropy_conversion makes from
# base::architectural_gloss_to_rough(0) with the mia semantic (iso and
# anisotropy 4: architectural's own reflection, as Calib-NVIDIA2's A4 / A5).
ANISO = [(f'S{k + 1} v{v} {what}', f'v{v.replace(".", "")}_pt_{mat}', f'orbis/orbis_aniso_v{v.replace(".", "")}.mdl#{mat}')
         for k, (what, mat, v) in enumerate([(w, m, v) for w, m in (('iso r 0.2', 'iso_r02'), ('aniso u 0.4 v 0.1', 'aniso_u04_v01'),
                                                                    ('arch g 0 iso', 'arch_g0_iso'), ('arch g 0 aniso 4', 'arch_g0_a4'))
                                             for v in ('1.0', '1.3')])]
# #117: is the angular loss of NVIDIA's 1.0 rough transmission (Iray /
# libbsdf falls from the centre to the rim on closed spheres, flat on thin
# walls, more with IOR and roughness) the legacy lobe's, and is it total
# internal reflection? The closed transmit lobe at r 0.2 (#112's M7 / T3
# are IOR 1.5) at IOR 1.2 / 2.0, and thin-walled at r 0.2 / 0.4 (no
# refraction, no TIR), each at 1.0 / 1.3; resin's scatter_reflect_transmit
# base at Calib-Furnace-Resin2's U3 - U6 roughnesses (IOR 1.49).
TIR = [(f'X{k + 1} v{v} {what}', f'v{v.replace(".", "")}_tir_{mat}', f'orbis/orbis_tir_v{v.replace(".", "")}.mdl#{mat}')
       for k, (what, mat, v) in enumerate([(w, m, v) for w, m in (('transmit r 0.2 ior 1.2', 'transmit_r02_ior12'),
                                                                  ('transmit r 0.2 ior 2', 'transmit_r02_ior2'),
                                                                  ('thin transmit r 0.2', 'thin_transmit_r02'),
                                                                  ('thin transmit r 0.4', 'thin_transmit_r04'))
                                           for v in ('1.0', '1.3')])]
RESIN = [(f'R{k + 1} v{v} resin r {r}', f'v{v.replace(".", "")}_resin_r{R_NAME(r)}', f'orbis/orbis_resin_v{v.replace(".", "")}.mdl#resin_r{R_NAME(r)}')
         for k, (v, r) in enumerate([(v, r) for v in ('1.0', '1.3') for r in (0.25, 0.5, 0.75, 1.0)])]
# #117 second round: the legacy lobe is `libbsdf`'s x a per-event loss
# g(r, cos), IOR- and wall-independent (MdlTir / MdlTransmit), NVIDIA's
# resin base = the plain 1.0 lobe ring for ring (MdlResin v Resin2), but
# architectural's G3 matches the 1.0 lobe only at ~3.7 x its formula's
# roughness: the base:: roughness call, the layer structure, or a constant.
ARCH_MDL = [(f'A{k + 1} v{v} {what}', f'v{v.replace(".", "")}_arch_{mat}', f'orbis/orbis_arch_v{v.replace(".", "")}.mdl#{mat}')
            for k, (what, mat, v) in enumerate([('gtr(0.4)', 'gtr04', '1.0'), ('gtr(0.2)', 'gtr02', '1.0'), ('gtr(0)', 'gtr0', '1.0'),
                                                ('gtr(0.2)', 'gtr02', '1.3'), ('struct gtr(0.2)', 'struct_gtr02', '1.0'),
                                                ('struct r 0.134', 'struct_r0134', '1.0'), ('thin gtr(0.2)', 'thin_gtr02', '1.0'),
                                                ('thin struct gtr(0.2)', 'thin_struct_gtr02', '1.0')])]
LEGACY = [(f'K{k + 1} v{v} {what}', f'v{v.replace(".", "")}_legacy_{mat}', f'orbis/orbis_legacy_v{v.replace(".", "")}.mdl#{mat}')
          for k, (what, mat, v) in enumerate([('transmit r 0.3', 'transmit_r03', '1.0'), ('transmit r 0.5', 'transmit_r05', '1.0'),
                                              ('transmit r 0.7', 'transmit_r07', '1.0'), ('transmit r 1', 'transmit_r1', '1.0'),
                                              ('thin transmit r 0.1', 'thin_transmit_r01', '1.0'), ('thin transmit r 0.6', 'thin_transmit_r06', '1.0'),
                                              ('thin transmit r 1', 'thin_transmit_r1', '1.0'), ('transmit r 0.5', 'transmit_r05', '1.3')])]
# #117 third round: the thin walls' per-event g over-predicts the closed
# spheres' loss above r 0.3 (r 1: 0.24 v Iray 0.34) - the IOR / bending.
LEGACY2 = [(f'L{k + 1} v{v} {what}', f'v{v.replace(".", "")}_legacy2_{mat}', f'orbis/orbis_legacy2_v{v.replace(".", "")}.mdl#{mat}')
           for k, (what, mat, v) in enumerate([('ior 1.05 r 0.6', 'ior105_r06', '1.0'), ('ior 1.05 r 1', 'ior105_r1', '1.0'),
                                               ('ior 1.2 r 0.6', 'ior12_r06', '1.0'), ('ior 2 r 0.6', 'ior2_r06', '1.0'),
                                               ('thin r 0.6', 'thin_r06', '1.3'), ('thin r 1', 'thin_r1', '1.3'),
                                               ('ior 2 r 1', 'ior2_r1', '1.0'), ('ior 1.05 r 1', 'ior105_r1', '1.3')])]
# #112 second round: MdlVersion's white isotropic 1.0 lobe is only 2 - 7 %
# below 1.3, not Metal Brushed's ~30 % rim; split the rest of
# metal_generic (orbis_generic_v10 / _v13, see the module).
GENERIC = [(f'G{k + 1} v{v} {what}', f'v{v.replace(".", "")}_generic_{mat}', f'orbis/orbis_generic_v{v.replace(".", "")}.mdl#{mat}')
           for k, (what, mat, v) in enumerate([(w, m, v) for w, m in (('tint 0.65 r 0.4', 'tint_r04'), ('aniso u 0.42 v 0.35', 'aniso_b6'),
                                                                      ('varying r 0.4', 'varying_r04'), ('generic B6', 'generic_b6'))
                                               for v in ('1.0', '1.3')])]
# #112 third round: the pieces between G7 and B6 (orbis_textured_v10 /
# _v13, see the module); H6 imports nvidia::metal_examples (Calib-NVIDIA*'s
# Metal Brushed / Cast Iron module: one import per DAZ session, #94).
TEXTURED = [(f'H{k + 1} v{v} {what}', f'v{v.replace(".", "")}_textured_{mat}', f'orbis/orbis_textured_v{v.replace(".", "")}.mdl#{mat}')
            for k, (what, mat, v) in enumerate([('bump 0', 'bump0', '1.0'), ('bump 0.4', 'bump04', '1.0'),
                                                ('weight map', 'weight_map', '1.0'), ('rough map', 'rough_map', '1.0'),
                                                ('all maps', 'all_maps', '1.0'), ('nvidia B6', 'nvidia_b6', '1.0'),
                                                ('bump 0', 'bump0', '1.3'), ('white r 0.4 bump 0', 'white_bump0', '1.0')])]
# #112 fourth round: H4 (G7's roughness through base::file_texture) is
# B6's rim; which property of that roughness, at which version, on the
# bare white lobe (orbis_roughtex_v10 / _v13, see the module).
ROUGHTEX = [(f'J{k + 1} v{v} {what}', f'v{v.replace(".", "")}_roughtex_{mat}', f'orbis/orbis_roughtex_v{v.replace(".", "")}.mdl#{mat}')
            for k, (what, mat, v) in enumerate([('file r 0.4', 'file_r04', '1.0'), ('file r 0.4', 'file_r04', '1.3'),
                                                ('lookup r 0.4', 'lookup_r04', '1.0'), ('varying r 0.4', 'varying_r04', '1.0'),
                                                ('varying r 0.4', 'varying_r04', '1.3'), ('file tint r 0.4', 'file_tint', '1.0'),
                                                ('file r 0.1', 'file_r01', '1.0'), ('file r 0.7', 'file_r07', '1.0')])]
# #191: is Iray's legacy (1.0) lobe cos^e in its transmitting modes too,
# as #184 found the reflect-only one? Per roughness the 1.0 lobe, the 1.3
# one (libbsdf, cos^(e - 1)) and a 1.3 twin at r' = r sqrt(2 / (2 + r^2))
# (cos^(e' - 1) = cos^e: the 1.0 highlight to a pixel if the legacy lobe
# is cos^e); r 0.7 without its plain 1.3 sphere (8 slots).
TWIN = lambda r: round(r * (2 / (2 + r * r)) ** 0.5, 4)
LOBE_TWINS = [(what, mat, v) for r in (0.354, 0.5) for what, mat, v in
              ((f'v1.0 r {r}', f'r{R_NAME(r)}', '1.0'), (f'v1.3 r {r}', f'r{R_NAME(r)}', '1.3'), (f'v1.3 twin r {TWIN(r)}', f'twin_r{R_NAME(r)}', '1.3'))] + \
             [('v1.0 r 0.7', 'r07', '1.0'), (f'v1.3 twin r {TWIN(0.7)}', 'twin_r07', '1.3')]
REFL_TRANS = [(f'S{k + 1} {what}', f'v{v.replace(".", "")}_rtlobe_{mat}', f'orbis/orbis_rtlobe_v{v.replace(".", "")}.mdl#{mat}')
              for k, (what, mat, v) in enumerate(LOBE_TWINS)]
BACK_TRANS = [(f'T{k + 1} {what}', f'v{v.replace(".", "")}_tlobe_{mat}', f'orbis/orbis_tlobe_v{v.replace(".", "")}.mdl#{mat}')
              for k, (what, mat, v) in enumerate(LOBE_TWINS)]
# NVIDIA's architectural preset itself under the same light, its lobe alone
# (diffuse black, reflectivity 1, Schlick 1 / 1: a white constant lobe): the
# scene the export maps, so it has an Octane side. (label, channels).
ARCH_POINT = [('P1 arch g 0 iso', {'refl_gloss': 0.0, 'anisotropy': 1.0}),
              ('P2 arch g 0 aniso 4', {'refl_gloss': 0.0, 'anisotropy': 4.0}),
              ('P3 arch g 0.15 iso', {'refl_gloss': 0.15, 'anisotropy': 1.0}),
              ('P4 arch g 0.15 aniso 4', {'refl_gloss': 0.15, 'anisotropy': 4.0}),
              ('P5 arch g 0.3 iso', {'refl_gloss': 0.3, 'anisotropy': 1.0}),
              ('P6 arch g 0.3 aniso 4', {'refl_gloss': 0.3, 'anisotropy': 4.0}),
              ('P7 arch g 0 aniso 0.25', {'refl_gloss': 0.0, 'anisotropy': 0.25}),
              ('P8 arch g 0 aniso 4 rot 0.25', {'refl_gloss': 0.0, 'anisotropy': 4.0, 'anisotropy_rotation': 0.25})]

# Modules: ours from calib/mdl under ROOT; the path probe's three copies of
# one tiny module under each candidate root.
src = os.path.join(REPO, 'calib/mdl')
dst = os.path.join(LIB, ROOT, 'orbis')
os.makedirs(dst, exist_ok=True)
for f in sorted(os.listdir(src)):
    if f.endswith('.mdl') and not f.startswith('orbis_path'):
        shutil.copyfile(os.path.join(src, f), os.path.join(dst, f))
# The flat maps orbis_textured_* / orbis_roughtex_* read as /orbis/calib-disp-*.png: B6's own.
for c in ('grey', 'white', 'black'):
    shutil.copyfile(os.path.join(LIB, 'Runtime/Textures/ORBIS', f'calib-disp-{c}.png'), os.path.join(dst, f'calib-disp-{c}.png'))
probe = open(os.path.join(src, 'orbis_path.mdl')).read()
for _, _, c, root in PATH_PROBE:
    d = os.path.join(LIB, root, 'orbis')
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, c.split('/')[1].split('#')[0]), 'w').write(probe)

os.makedirs(os.path.dirname(DSF), exist_ok=True)
lib_mats = [brick_material(mid, c) for _, mid, c, *_ in PATH_PROBE + VERSION + LOBE + TRANSMIT + ANISO + TIR + RESIN + GENERIC + ARCH_MDL + LEGACY + TEXTURED + LEGACY2 + ROUGHTEX + REFL_TRANS + BACK_TRANS]
dsf = {'file_version': '0.6.0.0',
       'asset_info': {'id': DSF_URL, 'type': 'material', 'contributor': {'author': 'ORBIS', 'email': '', 'website': ''}, 'revision': '1.0'},
       'material_library': lib_mats}
open(DSF, 'wb').write(gzip.compress(json.dumps(dsf, indent=1).encode()))
print(DSF)


def node_channels(entry):
    ext = next((e for e in entry.setdefault('extra', []) if e.get('type') == 'studio_node_channels'), None)
    if ext is None:
        ext = {'type': 'studio_node_channels', 'version': 5}
        entry['extra'].append(ext)
    return ext.setdefault('channels', [])


def set_channel(chs, cid, value):
    for c in chs:
        if c['channel']['id'] == cid:
            c['channel']['current_value'] = value
            return
    chs.append({'channel': {'id': cid, 'type': 'bool' if isinstance(value, bool) else 'float', 'current_value': value}})


def point_lit(d, ev):
    """No dome (Scene Only), one point light of Calib-L-Point-5k's (5000 lm,
    point geometry) 5 cm above the camera, so each sphere's highlight is its
    lobe seen head-on; exposure value `ev` (shutter 2^ev / 64 at f/8, ISO
    100, as DAZ keeps them)."""
    src = load(os.path.join(SCENES, 'Calib-L-Point-5k.duf'))
    light = copy.deepcopy(next(l for l in src['node_library'] if l['id'] == 'PointLight'))
    cam = next(l for l in d['node_library'] if l['id'] == 'Camera')
    pos = {c['id']: c.get('current_value', c.get('value', 0)) for c in cam['translation']}
    for c in light['translation']:
        c['current_value'] = pos[c['id']] + (5.0 if c['id'] == 'y' else 0.0)
    d['node_library'].append(light)
    d['scene']['nodes'].append(copy.deepcopy(next(n for n in src['scene']['nodes'] if n['id'] == 'PointLight-1')))
    env = node_channels(next(l for l in d['node_library'] if l['id'] == 'Environment Options'))
    set_channel(env, 'Environment Mode', 3)
    set_channel(env, 'Draw Dome', False)
    tone = node_channels(next(l for l in d['node_library'] if l['id'] == 'Tonemapper Options'))
    set_channel(tone, 'Exposure Value', ev)
    set_channel(tone, 'Shutter Speed', 2.0 ** ev / 64.0)


def arch_material(m, channels):
    """Calib-NVIDIA2's A5 material entry (NVIDIA's Architectural preset,
    which ships with DAZ Studio) on this sphere, reduced to its reflection:
    diffuse black, reflectivity 1, Schlick 0 / 90 degrees 1 / 1, white."""
    a5 = next(x for x in load(os.path.join(SCENES, 'Calib-NVIDIA2.duf'))['scene']['materials'] if x['id'] == 'Default-8')
    e = copy.deepcopy(a5)
    e['id'], e['geometry'], e['groups'] = m['id'], m['geometry'], m.get('groups', ['Default'])
    if m.get('uv_set'):
        e['uv_set'] = m['uv_set']
    else:
        e.pop('uv_set', None)
    e['diffuse']['channel']['current_value'] = [0.0, 0.0, 0.0]
    chs = next(x for x in e['extra'] if x.get('type') == 'studio_material_channels')['channels']
    vals = {'refl_color': [1.0, 1.0, 1.0], 'reflectivity': 1.0, 'brdf_0_degree_refl2': 1.0, 'brdf_90_degree_refl2': 1.0,
            'anisotropy_rotation': 0.0, **channels}
    for c in chs:
        if c['channel']['id'] in vals:
            c['channel']['current_value'] = vals.pop(c['channel']['id'])
    assert not vals, vals
    return e


def back_lit(d, ev, spheres, behind=60.0):
    """No dome, the camera lowered to the spheres' centres and level (0, 30,
    450; Calib-Furnace-Metal's spheres are R 15 about y 15 in their
    geometry, translated 15 up), and behind each of `spheres` (scene nodes) a spot light on the
    line from the camera through its centre, `behind` cm past the centre,
    aimed back at the camera (a Y turn only): flat beam (Beam Exponent 0)
    of 40 degrees full spread, which covers its sphere (R 15) and no other
    (50 cm apart), 5000 lm as Calib-L-Point-5k's point. Seen through a
    closed transmitting sphere, each lamp is a blob whose radius maps the
    deviation the two rough events must make (the ball lens's, ~ the
    entry angle): the transmit lobe head-on. Exposure value `ev`."""
    import math
    import urllib.parse
    lib = {l['id']: l for l in d['node_library']}
    geo = {g['id']: g for g in d.get('geometry_library', [])}

    def centre(n):
        t = [next(c.get('current_value', c.get('value', 0)) for c in lib[urllib.parse.unquote(n['url'].lstrip('#'))]['translation'] if c['id'] == a) for a in 'xyz']
        v = geo[n['geometries'][0]['url'].lstrip('#')]['vertices']['values']
        return [t[i] + sum(p[i] for p in v) / len(v) for i in range(3)]

    cam = lib['Camera']
    C = (0.0, centre(spheres[0])[1], 450.0)
    for c in cam['translation']:
        c['current_value'] = C['xyz'.index(c['id'])]
    for c in cam['rotation']:
        c['current_value'] = 0.0
    env = node_channels(next(l for l in d['node_library'] if l['id'] == 'Environment Options'))
    set_channel(env, 'Environment Mode', 3)
    set_channel(env, 'Draw Dome', False)
    tone = node_channels(next(l for l in d['node_library'] if l['id'] == 'Tonemapper Options'))
    set_channel(tone, 'Exposure Value', ev)
    set_channel(tone, 'Shutter Speed', 2.0 ** ev / 64.0)
    src = load(os.path.join(SCENES, 'Calib-L-Spot-s30-e4.duf'))
    spot = next(l for l in src['node_library'] if l['id'] == 'SpotLight')
    for k, n in enumerate(spheres):
        S = centre(n)
        u = [S[i] - C[i] for i in range(3)]
        m = math.sqrt(sum(x * x for x in u))
        L = [S[i] + u[i] / m * behind for i in range(3)]
        lid = f'SpotLight-{k + 1}'
        e = copy.deepcopy(spot)
        e['id'] = e['name'] = lid
        e['label'] = f'Spot {n["label"].split()[0]}'
        e['spot']['falloff_angle'] = 40
        for c in e['translation']:
            c['current_value'] = L['xyz'.index(c['id'])]
        # -Z turned by Y phi is (-sin phi, 0, -cos phi): towards the camera.
        phi = math.degrees(math.atan2(u[0], u[2]))
        for c in e['rotation']:
            c['current_value'] = phi if c['id'] == 'y' else 0.0
        chs = node_channels(e)
        for cid, val in (('Flux', 5000.0), ('Spread Angle', 40.0), ('Beam Exponent', 0.0), ('Shadow Softness', 0.0)):
            set_channel(chs, cid, val)
        d['node_library'].append(e)
        d['scene']['nodes'].append({'id': lid + '-1', 'url': '#' + lid, 'name': lid, 'label': e['label'],
                                    'extra': [{'type': 'studio_node_channels', 'version': 5}]})


def furnace_scene(name, spheres, ev=None, arch=False, back=None):
    """Calib-Furnace-Metal (uniform dome, floor hidden) with its first
    len(spheres) spheres, in label order F1 ..., on our brick materials
    (`arch`: on NVIDIA's architectural preset with the sphere's channels);
    the rest hidden. With `ev`, lit by a point light instead of the dome
    (`point_lit`); with `back`, by a spot behind each sphere at that
    exposure (`back_lit`)."""
    p = os.path.join(SCENES, name + '.duf')
    if os.path.exists(p) and not FORCE:
        return p
    d = load(FURNACE_METAL)
    d['asset_info']['id'] = '/Scenes/' + name + '.duf'
    if ev is not None:
        point_lit(d, ev)
    geo_node = {g['id']: n for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    lit = []
    for i, m in enumerate(d['scene']['materials']):
        n = geo_node.get(m['geometry'].lstrip('#'))
        if not n or not n.get('label', '').startswith('F'):
            continue
        k = int(n['label'][1]) - 1
        if k >= len(spheres):
            ext = next((e for e in n.setdefault('extra', [{'type': 'studio_node_channels', 'version': 5}])
                        if e.get('type') == 'studio_node_channels'), None)
            if ext is None:
                ext = {'type': 'studio_node_channels', 'version': 5}
                n['extra'].append(ext)
            ext.setdefault('channels', []).append({'channel': {'id': 'Visible', 'type': 'bool', 'current_value': False}})
            n['label'] = 'hidden ' + n['label']
            continue
        label, mid = spheres[k][0], spheres[k][1]
        n['label'] = label
        if arch:
            d['scene']['materials'][i] = arch_material(m, mid)
            continue
        d['scene']['materials'][i] = {'id': m['id'], 'url': DSF_URL + '#' + mid, 'geometry': m['geometry'],
                                      'groups': m.get('groups', ['Default']), 'uv_set': m.get('uv_set'),
                                      'extra': [{'type': 'studio/material/daz_brick'}]}
        if d['scene']['materials'][i]['uv_set'] is None:
            del d['scene']['materials'][i]['uv_set']
        n['label'] = label
        lit.append(n)
    if back is not None:
        back_lit(d, back, lit)
    duf_camera.ensure(d)  # else Iray renders DAZ's Perspective View (#97)
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    shutil.copyfile(p, os.path.join(REPO, 'calib/scenes', name + '.duf'))
    print(p)
    return p


furnace_scene('Calib-Mi-MdlPath', PATH_PROBE)
furnace_scene('Calib-Furnace-MdlVersion', VERSION)
furnace_scene('Calib-Furnace-MdlLobe', LOBE)
furnace_scene('Calib-Furnace-MdlTransmit', TRANSMIT)
# Peaks: a Phong lobe's D(0) ~ 1 / (pi r_u r_v): 8 at r 0.2 or 0.4 / 0.1,
# 2.5 at architectural's glossiness 0 (r 0.354); 5000 lm at 4.6 m. EV 9:
# the r 0.2 group ~0.4 of white, glossiness 0 ~0.1 (Octane, Calib-Point-
# ArchAniso's first render at EV 8.3: glossiness 0 peaked at 0.19).
furnace_scene('Calib-Point-MdlAniso', ANISO, ev=9.0)
# #191: the white 1.0 lobe at r 0.354 peaked 0.018 at EV 11.5 (Calib-Point-
# ArchAniso P1); reflect_transmit's front reflection is F 0.039 of it: EV 3
# puts r 0.354 near 0.25, r 0.7 near 0.07.
furnace_scene('Calib-Point-MdlReflTrans', REFL_TRANS, ev=3.0)
# Back-lit: EV 16 (spots aimed 15 cm low) put the blobs' edges near 0.7, EV
# 16.5 aimed clipped r 0.354's peak ~6x (the profile through the model):
# EV 20 puts it near 0.5, r 0.7 near 0.15.
furnace_scene('Calib-Back-MdlTransmit', BACK_TRANS, back=20.0)
# Architectural's glossiness 0.15 / 0.3 peak ~8x / ~50x above 0 (at EV 8.3
# both clipped, glossiness 0 0.19): at EV 11.5 glossiness 0 ~0.02, 0.3 ~1.
furnace_scene('Calib-Point-ArchAniso', ARCH_POINT, ev=11.5, arch=True)


def uber_point_scene(name, spheres, ev):
    """Calib-Furnace-Metal's Uber spheres (floor hidden) under `point_lit`
    (`ev` None: under its own uniform dome, the furnace), sphere k on the
    channels of spheres[k] (label, {channel: value}, base colour); its
    material_library entry carries the Uber channels."""
    p = os.path.join(SCENES, name + '.duf')
    if os.path.exists(p) and not FORCE:
        return p
    d = load(FURNACE_METAL)
    d['asset_info']['id'] = '/Scenes/' + name + '.duf'
    if ev is not None:
        point_lit(d, ev)
    geo_node = {g['id']: n for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    mlib = {m['id']: m for m in d['material_library']}
    for m in d['scene']['materials']:
        n = geo_node.get(m['geometry'].lstrip('#'))
        if not n or not n.get('label', '').startswith('F'):
            continue
        label, chans, colour = spheres[int(n['label'][1]) - 1]
        n['label'] = label
        m['diffuse'] = {'channel': {'id': 'diffuse', 'type': 'float_color', 'current_value': colour}}
        lm = mlib[m['url'].lstrip('#')]
        lm['diffuse']['channel']['current_value'] = colour
        chs = next(e for e in lm['extra'] if e['type'] == 'studio_material_channels')['channels']
        vals = dict(chans)
        for c in chs:
            if c['channel']['id'] in vals:
                c['channel']['current_value'] = vals.pop(c['channel']['id'])
        assert not vals, vals
    duf_camera.ensure(d)
    open(p, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    shutil.copyfile(p, os.path.join(REPO, 'calib/scenes', name + '.duf'))
    print(p)
    return p


# #162: the dielectric gloss (Glossy Layered Weight 1, Reflectivity 0.5:
# F0 0.04) peaks ~0.04 x Calib-Point-ArchAniso's white lobe (0.018 at r
# 0.354, EV 11.5): EV 3.4 puts r 0.354 near 0.2, r 0.25 ~0.4, r 1 ~0.05;
# the metals at base 0.1 stay below 1 at r 0.354.
DIEL = lambda r: {'Metallic Weight': 0.0, 'Glossy Layered Weight': 1.0, 'Glossy Reflectivity': 0.5, 'Glossy Roughness': round(r ** 0.5, 4)}
METAL = lambda r: {'Metallic Weight': 1.0, 'Glossy Roughness': round(r ** 0.5, 4)}
UBER_POINT = [(f'U{k + 1} dielectric r {r}', DIEL(r), [0.0, 0.0, 0.0]) for k, r in enumerate((0.25, 0.354, 0.5, 0.7, 1.0))] + \
             [(f'U{k + 6} metal r {r}', METAL(r), [0.1, 0.1, 0.1]) for k, r in enumerate((0.354, 0.5, 0.7))]
uber_point_scene('Calib-Point-UberGloss', UBER_POINT, ev=3.4)


# #193: what Iray's dual lobe leaves of the diffuse under it, per angle.
# `add_dual_lobe_specular` makes the diffuse the *layer* of a
# custom_curve_layer (1 - R W head-on, 1 - W at grazing) over the lobes;
# libbsdf weighs that layer w c(k1 . h) (flat 1 - R W under a light at the
# camera) and the lobes 1 - w max(c(n . k1), c(n . k2)). Each scene in
# three twins - a grey diffuse 0.5 under the dual lobe, the lobe over
# black (the lobe alone), the grey without the lobe - so (grey - black) /
# off is the diffuse's share per ring; under the point light at the camera
# and in the uniform furnace. P1 - P8: W 1, Reflectivity 0.5 (F0 0.04),
# both lobes at the MDL roughness whose `phong_pin` is the label's pin
# (0.448 Calib-Skin-Nishi's); C1 - C8 at pin 0.448 vary the curve (W, R)
# and the lobes. #190's grey base at Calib-Point-UberGloss's EV 3.4
# clipped 38 - 56 % of each disc; EV 6.1 puts the grey's centre near 0.2
# and pin 0.3's lobe on it under 1.
def dl_rough(pin):
    """The MDL roughness r whose Phong pin (`phong_pin`: alpha =
    r sqrt(2 / (2 + r^2))) is `pin`, as the Uber channel (sqrt r: Roughness
    Squared)."""
    lo, hi = 0.0, 1.5
    for _ in range(60):
        m = (lo + hi) / 2
        lo, hi = (m, hi) if m * (2 / (2 + m * m)) ** 0.5 < pin * pin else (lo, m)
    return round(lo ** 0.5, 4)


def DL(w, refl, pin1, pin2=None, ratio=0.85):
    # Calib-Furnace-Metal's spheres are metals: Metallic Weight 0 here.
    return {'Metallic Weight': 0.0, 'Glossy Layered Weight': 0.0, 'Dual Lobe Specular Weight': w, 'Dual Lobe Specular Reflectivity': refl, 'Specular Lobe 1 Roughness': dl_rough(pin1),
            'Specular Lobe 2 Roughness': dl_rough(pin2 if pin2 is not None else pin1), 'Dual Lobe Specular Ratio': ratio}


DL_PINS = (0.3, 0.4, 0.448, 0.5, 0.6, 0.7, 0.8, 0.9)
DL_BASE = [(f'P{k + 1} pin {p}', DL(1.0, 0.5, p)) for k, p in enumerate(DL_PINS)]
DL_CURVE = [('C1 w1 r0.5', DL(1.0, 0.5, 0.448)), ('C2 w1 r1', DL(1.0, 1.0, 0.448)), ('C3 w1 r0.25', DL(1.0, 0.25, 0.448)),
            ('C4 w1 r0', DL(1.0, 0.0, 0.448)), ('C5 w0.5 r0.5', DL(0.5, 0.5, 0.448)), ('C6 w0.25 r0.5', DL(0.25, 0.5, 0.448)),
            ('C7 lobe2 pin 0.3', DL(1.0, 0.5, 0.448, 0.3)), ('C8 lobe2 pin 0.8 ratio 0.5', DL(1.0, 0.5, 0.448, 0.8, 0.5))]
for light, ev in (('Point', 6.1), ('Furnace', None)):
    for set_, spheres in (('Base', DL_BASE), ('Curve', DL_CURVE)):
        uber_point_scene(f'Calib-{light}-DualLobe{set_}-Grey', [(l, c, [0.5] * 3) for l, c in spheres], ev)
        uber_point_scene(f'Calib-{light}-DualLobe{set_}-Black', [(l, c, [0.0] * 3) for l, c in spheres], ev)
    uber_point_scene(f'Calib-{light}-DualLobeBase-Off', [(l, {**c, 'Dual Lobe Specular Weight': 0.0}, [0.5] * 3) for l, c in DL_BASE], ev)
furnace_scene('Calib-Furnace-MdlTir', TIR)
furnace_scene('Calib-Furnace-MdlResin', RESIN)
furnace_scene('Calib-Furnace-MdlGeneric', GENERIC)
furnace_scene('Calib-Furnace-MdlArch', ARCH_MDL)
furnace_scene('Calib-Furnace-MdlLegacy', LEGACY)
furnace_scene('Calib-Furnace-MdlTextured', TEXTURED)
furnace_scene('Calib-Furnace-MdlLegacy2', LEGACY2)
furnace_scene('Calib-Furnace-MdlRoughTex', ROUGHTEX)
