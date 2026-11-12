#!/usr/bin/env python3
"""ORBIS .mi sweeps for DAZ's other shaders (#62): channel variants of a
scene daz-build-shader-scene.dsa built with the presets on its spheres.
Read through DAZ's .mi export, not rendered.

  python3 scripts/dev/make-mi-shader-scenes.py [<scenes dir>]

- Calib-Mi-Hair (from Calib-Mi-Hair-base): Blended Dual Lobe Hair on
  spheres 1-4, OmniHair Root-Tip Blend on 5-8, each varied.
- Calib-Mi-Hair2 (from Calib-Mi-Hair-base): Blended's Root To Tip Bias /
  Gain, Highlight Weight 0 / 1 and `strength` (#42).
- Calib-Mi-DzDefault2 (from Calib-Mi-DzDefault, DS Defaults presets): the
  DAZ Default shader's channels, which DAZ turns into Iray Uber.
- Calib-Mi-Legacy (from Calib-Mi-Legacy-base: UberSurface, dzGlossy,
  dzGlossyMetal, dzMatte, dzMetal, dzPlastic, dzSkin, AoA Subsurface on
  S1-S8, as DAZ Studio 6 applies them - no 3Delight, so the scripted
  shaders keep only Diffuse Color and Opacity Strength; #64).
- Calib-Mi-AoA (from Calib-Mi-AoA-base, AoA Subsurface on every sphere): its
  channels varied - DAZ renders it as Iray Uber too (#64); Calib-Mi-AoA2 the
  second sweep, Calib-Mi-AoA3 maps, Calib-Mi-AoA4 Specular 2 colours.
- Calib-Mi-Legacy2 (from Calib-Mi-Legacy-base): the UberSurface channels a
  DAZ Studio 4 save carries (USURFACE below), varied, on every sphere but
  S8 - what DAZ Studio 6 makes of an old scene (#64).
- Calib-Mi-DzDefault3 / Calib-Mi-AoA5: Multiply Specular Through Opacity
  off on transparent spheres (#15); Calib-Mi-Arch (from Calib-NVIDIA3):
  Architectural's Enable Absorption.
- Calib-Mi-DzDefault4 ... -DzDefault8 (from Calib-Mi-DzDefault): DAZ
  Default's maps, bump / normal / displacement / tiling, lighting models,
  the hidden Subsurface group (DzDefault6-Shown: every channel shown) and
  when refraction shares the glossy inputs (#41).
"""
import sys as _sys
if any(a in ('-h', '--help') for a in _sys.argv[1:]):
    print(__doc__)
    _sys.exit(0)
import gzip, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import duf_camera

scenes = sys.argv[1] if len(sys.argv) > 1 else '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'

VARIANTS = {
    'Calib-Mi-Hair': ('Calib-Mi-Hair-base', {
        'S2': {'Hair Root Color': [0.3, 0.2, 0.1], 'Hair Tip Color': [0.8, 0.7, 0.5], 'Glossy Layer Weight': 0.8, 'base_roughness': 0.4},
        'S3': {'Highlight Weight': 0.5, 'highlight_roughness': 0.3, 'separation': 0.2, 'Highlight Root Color': [0.9, 0.6, 0.3], 'Tip Highlight Color': [0.4, 0.5, 0.9]},
        'S4': {'Anisotropy': 0.5, 'Anisotropy Rotations': 0.25, 'Cutout Opacity': 0.7, 'Root Transmission Color': [0.2, 0.4, 0.8], 'Tip Transmission Color': [0.9, 0.2, 0.1]},
        'S6': {'Melanin': 0.2, 'Melanin Redness': 0.8, 'Roughness': 0.4, 'Roughness Tip': 0.6},
        'S7': {'Highlight Weight': 0.5, 'Highlight Root Color': [0.9, 0.6, 0.3], 'Tip Highlight Color': [0.4, 0.5, 0.9], 'Specular Reflection Shift': 5.0,
               'Azimuthal Roughness': 0.4, 'Azimuthal Roughness Tip': 0.6},
        'S8': {'Diffuse Strength': 0.5, 'Hair Root Color': [0.3, 0.2, 0.1], 'Hair Tip Color': [0.8, 0.7, 0.5], 'Melanin': 0.9},
    }),
    # Root-to-tip blend shape, highlight extremes, `strength` (#42).
    'Calib-Mi-Hair2': ('Calib-Mi-Hair-base', {
        'S1': {'Root To Tip Bias': 0.2, 'Root to Tip Gain': 0.7, 'Hair Root Color': [0.3, 0.2, 0.1], 'Hair Tip Color': [0.8, 0.7, 0.5]},
        'S2': {'Highlight Weight': 0.0, 'strength': 0.5},
        'S3': {'Highlight Weight': 1.0, 'highlight_roughness': 0.5, 'separation': 0.0},
        'S4': {'Root To Tip Bias': 0.8, 'Root to Tip Gain': 0.3, 'Glossy Layer Weight': 0.0, 'Root Transmission Color': [0.2, 0.4, 0.8], 'Tip Transmission Color': [0.9, 0.2, 0.1]},
    }),
    'Calib-Mi-DzDefault2': ('Calib-Mi-DzDefault', {
        'S1': {'glossiness': 0.3, 'specular_strength': 0.5, 'specular': [0.8, 0.6, 0.4]},
        'S2': {'diffuse_strength': 0.5, 'glossiness': 0.7},
        'S3': {'reflection_strength': 0.6, 'reflection': [0.9, 0.8, 0.7], 'glossiness': 0.9},
        'S4': {'refraction_strength': 0.8, 'refraction': [0.6, 0.8, 1.0], 'ior': 1.4},
        'S5': {'transparency': 0.4, 'glossiness': 0.5},
        'S6': {'ambient': [0.5, 0.4, 0.3], 'ambient_strength': 2.0},
        'S7': {'specular_strength': 0.0, 'glossiness': 0.5},
        'S8': {'specular': [1.0, 1.0, 1.0], 'specular_strength': 1.0, 'glossiness': 0.95, 'Lighting Model': 1},
    }),
}

# omUberSurface's channels as DAZ Studio 4 saved them: id -> (group, type,
# default[, enum values]).
USURFACE = {
    'Bump Active': ('/Bump', 'bool', False), 'Bump Strength': ('/Bump', 'float', 1.0),
    'Bump Minimum': ('/Bump', 'float', -0.1), 'Bump Maximum': ('/Bump', 'float', 0.1),
    'Displacement Active': ('/Displacement', 'bool', False), 'Displacement Strength': ('/Displacement', 'float', 1.0),
    'Displacement Minimum': ('/Displacement', 'float', -0.1), 'Displacement Maximum': ('/Displacement', 'float', 0.1),
    'Diffuse Active': ('/Diffuse', 'bool', True), 'Diffuse Strength': ('/Diffuse', 'float', 1.0), 'Diffuse Roughness': ('/Diffuse', 'float', 1.0),
    'Opacity Active': ('/Opacity/Basic', 'bool', True), 'Opacity Color': ('/Opacity/Basic', 'color', [1, 1, 1]),
    'Map Tiling U': ('/UV Maps/Tiling', 'float', 1.0), 'Map Tiling V': ('/UV Maps/Tiling', 'float', 1.0),
    'Specular Active': ('/Specular/Primary/Basic', 'bool', True), 'Specular Color': ('/Specular/Primary/Basic', 'color', [0, 0, 0]),
    'Specular Strength': ('/Specular/Primary/Basic', 'float', 1.0), 'Glossiness': ('/Specular/Primary/Basic', 'float', 0.1),
    'Specular Sharpness': ('/Specular/Primary/Basic', 'float', 0.0),
    'Specular2 Active': ('/Specular/Secondary/Basic', 'bool', False), 'Specular2 Color': ('/Specular/Secondary/Basic', 'color', [0, 0, 0]),
    'Specular2 Strength': ('/Specular/Secondary/Basic', 'float', 1.0), 'Specular2 Roughness': ('/Specular/Secondary/Basic', 'float', 0.1),
    'Ambient Active': ('/Ambient', 'bool', False), 'Ambient Color': ('/Ambient', 'color', [0, 0, 0]), 'Ambient Strength': ('/Ambient', 'float', 0.0),
    'Refraction Active': ('/Opacity/Refraction', 'bool', False), 'Index of Refraction': ('/Opacity/Refraction', 'float', 1.3),
    'Reflection Active': ('/Reflection/Basic', 'bool', False), 'Reflection Mode': ('/Reflection/Basic', 'enum', 0, ['Environment Map', 'Raytrace']),
    'Reflection Color': ('/Reflection/Basic', 'color', [0, 0, 0]), 'Reflection Strength': ('/Reflection/Basic', 'float', 1.0),
    'Reflection Blur': ('/Reflection/Basic', 'float', 0.0),
    'Fresnel Active': ('/Reflection/Fresnel', 'bool', False), 'Fresnel Strength': ('/Reflection/Fresnel', 'float', 1.0),
    'Fresnel Falloff': ('/Reflection/Fresnel', 'float', 0.4),
    'Velvet Active': ('/Velvet', 'bool', False), 'Velvet Color': ('/Velvet', 'color', [0, 0, 0]), 'Velvet Strength': ('/Velvet', 'float', 1.0),
    'Velvet Falloff': ('/Velvet', 'float', 20.0),
    'Subsurface Active': ('/Subsurface Scattering', 'bool', False), 'Subsurface Color': ('/Subsurface Scattering', 'color', [0, 0, 0]),
    'Subsurface Strength': ('/Subsurface Scattering', 'float', 1.0), 'Subsurface Refraction': ('/Subsurface Scattering', 'float', 1.5),
    'Subsurface Scale': ('/Subsurface Scattering', 'float', 4.0),
    'Translucency Active': ('/Translucency', 'bool', False), 'Translucency Color': ('/Translucency', 'color', [0, 0, 0]),
    'Translucency Strength': ('/Translucency', 'float', 1.0),
}
# Channels written with their whole definition when the material lacks them.
DEFS = {**USURFACE, 'Multiply Specular Through Opacity': ('/Specular', 'bool', True)}
TILES = '/Runtime/Textures/ORBIS/calib-tiles.png'

VARIANTS.update({
    # DAZ Studio 6 as it applies the presets: Diffuse Color, Opacity Strength
    # (S8 AoA Subsurface is a brick graph with its own channels).
    'Calib-Mi-Legacy': ('Calib-Mi-Legacy-base', {
        'S1': {'diffuse': [0.8, 0.4, 0.2], 'transparency': 0.6},
        'S2': {'diffuse': [0.2, 0.6, 0.8]},
        'S3': {'diffuse': (TILES, [1.0, 0.8, 0.6])},
        'S5': {'diffuse': [1.0, 0.8, 0.2], 'transparency': 0.3},
        'S6': {'transparency': (TILES, 1.0)},
        'S7': {'diffuse': [0.6, 0.4, 0.2]},
        'S8': {'diffuse': [0.2, 0.6, 0.2], 'Specular Strength': 0.5, 'Glossiness': 0.6, 'Specular Color': [1.0, 0.6, 0.4],
               'Diffuse Strength': 0.7, 'Reflection Strength': 0.4, 'strength22': 0.5, 'Bump Strength': 0.5},
    }),
    # AoA Subsurface (a 3Delight brick graph) on every sphere, its channels
    # varied (ids from AoA_Subsurface.dsf; labels in the comments).
    'Calib-Mi-AoA': ('Calib-Mi-AoA-base', {
        'S1': {'Glossiness': 0.3, 'Specular Strength': 1.0, 'Specular Color': [1.0, 1.0, 1.0]},
        'S2': {'Glossiness': 0.9, 'Diffuse Strength': 0.5},
        'S3': {'Glossiness': 0.95, 'Specular Color': [0.8, 0.6, 0.4], 'Specular Strength': 0.8, 'transparency': 0.6},
        # Reflection Strength / Color.
        'S4': {'Glossiness': 0.0, 'Reflection Strength': 0.8, 'Reflection Color': [0.6, 0.8, 1.0]},
        # Specular 2 Strength / Color / Glossiness.
        'S5': {'Glossiness': 0.5, 'value222': 0.6, 'color2': [0.4, 0.6, 1.0], 'strength22': 0.5},
        # Subsurface Off - On, Subsurface Strength / Color, Ambient.
        'S6': {'value23': 1.0, 'Ambient Strength': 0.6, 'Ambient Color': [1.0, 0.8, 0.6], 'Ambient Strength2': 0.5, 'color4': [0.6, 0.4, 0.2]},
        # Velvet Strength / Color; bump and tiling.
        'S7': {'strength3': 0.7, 'color3': [0.2, 0.4, 1.0], 'Bump Strength': (TILES, 0.6), 'Positive Bump': 0.02, 'Negative Bump': -0.02,
               'Horizontal Tiles': 2.0, 'Vertical Tiles': 3.0, 'diffuse': (TILES, [1.0, 1.0, 1.0])},
        # Normal Map Strength; displacement.
        'S8': {'Normal Map': (TILES, None), 'alpha': 0.5, 'Displacement Strength': (TILES, 0.5), 'Glossiness': 0.75, 'Specular Strength': 0.0},
    }),
    # The second sweep: Specular Strength 0, Specular 2, subsurface.
    'Calib-Mi-AoA2': ('Calib-Mi-AoA-base', {
        'S1': {'Specular Strength': 0.0, 'Glossiness': 0.5},
        'S2': {'Specular Strength': 0.0, 'Glossiness': 0.2, 'transparency': (TILES, 1.0)},
        'S3': {'Specular Strength': 0.0, 'Glossiness': 1.0, 'Normal Map': (TILES, None), 'alpha': 1.0},
        'S4': {'Specular Strength': 1.0, 'Glossiness': 0.75, 'Displacement Strength': (TILES, 0.5)},
        # Subsurface: Custom material, Shading Scale, IOR, scatter / absorb.
        'S5': {'value23': 1.0, 'Ambient Strength': 1.0, 'Ambient Color': [0.6, 0.4, 0.2], 'sssmaterial': 0, 'Scale': 0.2, 'ior': 1.5,
               'cScatter_x': 0.5, 'cAbsorb_y': 0.2},
        'S6': {'value23': 1.0, 'sssmaterial': 9, 'Glossiness': 0.5, 'Ambient Strength': 0.3},
        'S7': {'value222': 1.0, 'strength22': 0.2, 'color2': [1.0, 1.0, 1.0]},
        'S8': {'value222': 0.5, 'strength22': 0.9, 'color2': [1.0, 0.8, 0.6], 'Specular Strength': 0.0, 'Glossiness': 0.75},
    }),
    # The third: maps on the channels the conversion reads.
    'Calib-Mi-AoA3': ('Calib-Mi-AoA-base', {
        'S1': {'Specular Strength': (TILES, 0.8), 'Glossiness': 0.6},
        'S2': {'Specular Color': (TILES, [1.0, 0.8, 0.6]), 'Glossiness': 0.6},
        'S3': {'Glossiness': (TILES, 0.7)},
        'S4': {'Diffuse Strength': (TILES, 0.8), 'Glossiness': 0.6},
        'S5': {'Specular Strength': 0.0, 'value222': (TILES, 0.6), 'color2': [1.0, 1.0, 1.0]},
        'S6': {'value222': 0.5, 'color2': (TILES, [1.0, 0.8, 0.6]), 'Glossiness': 0.6},
        'S7': {'value222': 0.5, 'color2': [1.0, 1.0, 1.0], 'strength22': (TILES, 0.6), 'Glossiness': 0.6},
        'S8': {'Bump Strength': (TILES, 0.5), 'Normal Map': (TILES, None), 'Glossiness': 0.6},
    }),
    # The fourth: Specular 2 colours below 1 (hue or colour?), as the
    # glossy layer (Specular Strength 0) and as the top coat.
    'Calib-Mi-AoA4': ('Calib-Mi-AoA-base', {
        'S1': {'Specular Strength': 0.0, 'value222': 0.5, 'color2': [0.6, 0.4, 0.2], 'strength22': 0.8},
        'S2': {'value222': 0.5, 'color2': [0.6, 0.4, 0.2], 'strength22': 0.8, 'Glossiness': 0.6},
        'S3': {'Specular Strength': 0.0, 'value222': 0.0, 'color2': [0.6, 0.4, 0.2], 'strength22': 0.8},
        'S4': {'Specular Strength': 0.5, 'Specular Color': [0.4, 0.2, 0.2], 'value222': 1.0, 'color2': [0.4, 0.4, 0.4], 'strength22': 0.95},
    }),
    # An old (DAZ Studio 4) save: the whole USURFACE set on S1-S7, varied.
    'Calib-Mi-Legacy2': ('Calib-Mi-Legacy-base', {
        'S1': {'USURFACE': 1, 'diffuse': [0.8, 0.4, 0.2], 'Diffuse Strength': 0.6, 'Specular Color': [1, 1, 1], 'Specular Strength': 0.5, 'Glossiness': 0.7},
        'S2': {'USURFACE': 1, 'Specular Color': [0.9, 0.6, 0.3], 'Glossiness': 0.3, 'Specular2 Active': True, 'Specular2 Color': [1, 1, 1],
               'Specular2 Strength': 0.4, 'Specular2 Roughness': 0.2},
        'S3': {'USURFACE': 1, 'Reflection Active': True, 'Reflection Mode': 1, 'Reflection Color': [1, 1, 1], 'Reflection Strength': 0.6,
               'Fresnel Active': True},
        'S4': {'USURFACE': 1, 'Refraction Active': True, 'Index of Refraction': 1.45, 'transparency': 0.3, 'Opacity Color': [0.5, 0.8, 1.0]},
        'S5': {'USURFACE': 1, 'Subsurface Active': True, 'Subsurface Color': [1, 0.75, 0.68], 'Subsurface Strength': 0.6,
               'Translucency Active': True, 'Translucency Color': [1, 0.5, 0.4], 'Translucency Strength': 0.3},
        'S6': {'USURFACE': 1, 'Bump Active': True, 'Bump Strength': (TILES, 0.8), 'Map Tiling U': 2.0, 'Map Tiling V': 3.0,
               'diffuse': (TILES, [1, 1, 1])},
        'S7': {'USURFACE': 1, 'Velvet Active': True, 'Velvet Color': [0.4, 0.4, 1.0], 'Ambient Active': True, 'Ambient Color': [0.5, 0.4, 0.3],
               'Ambient Strength': 0.5, 'Diffuse Active': False},
    }),
    # Multiply Specular Through Opacity off (#15; on by default, in DAZ
    # Default's definition): transparent spheres against their twin S1.
    'Calib-Mi-DzDefault3': ('Calib-Mi-DzDefault', {
        'S1': {'transparency': 0.4, 'specular_strength': 1.0, 'glossiness': 0.6},
        'S2': {'transparency': 0.4, 'specular_strength': 1.0, 'glossiness': 0.6, 'Multiply Specular Through Opacity': False},
        'S3': {'transparency': 0.8, 'glossiness': 0.8, 'Multiply Specular Through Opacity': False},
        'S4': {'Multiply Specular Through Opacity': False},
        'S5': {'transparency': (TILES, 1.0), 'glossiness': 0.6, 'Multiply Specular Through Opacity': False},
    }),
    # Architectural's Enable Absorption (refr_falloff_on, #15): on glass
    # (A5, A6 with a distance) and on an opaque sphere (A1), as a scene's
    # metal doors carry it.
    'Calib-Mi-Arch': ('Calib-NVIDIA3', {
        'A1': {'refr_falloff_on': True},
        'A5': {'refr_falloff_on': True},
        'A6': {'refr_falloff_on': True, 'refr_falloff_dist': 2.0},
        'A7': {'refr_falloff_on': True, 'refr_falloff_color_on': True, 'refr_falloff_color': [0.8, 0.6, 0.4], 'refr_falloff_dist': 3.0},
    }),
    # The same on AoA Subsurface (a brick graph whose definition lacks it:
    # added as the brick materials that carry it save it).
    'Calib-Mi-AoA5': ('Calib-Mi-AoA-base', {
        'S1': {'transparency': 0.6, 'Glossiness': 0.6},
        'S2': {'transparency': 0.6, 'Glossiness': 0.6, 'Multiply Specular Through Opacity': False},
        'S3': {'Multiply Specular Through Opacity': False},
    }),
    # DAZ Default's maps and its remaining channels (#41): maps one per
    # sphere (DzDefault4), the rest of the maps and the bump / displacement
    # values without a map (DzDefault5), the lighting models and the hidden
    # Subsurface group (DzDefault6, with a -Shown twin).
    'Calib-Mi-DzDefault4': ('Calib-Mi-DzDefault', {
        'S1': {'diffuse': (TILES, [1.0, 1.0, 1.0]), 'u_scale': 2.0, 'v_scale': 3.0, 'u_offset': 0.2, 'v_offset': 0.4},
        'S2': {'diffuse_strength': (TILES, 0.8)},
        'S3': {'specular': (TILES, [1.0, 0.8, 0.6]), 'glossiness': 0.6},
        'S4': {'specular_strength': (TILES, 0.8), 'glossiness': 0.6},
        'S5': {'glossiness': (TILES, 0.6)},
        'S6': {'transparency': (TILES, 0.8), 'glossiness': 0.6},
        'S7': {'bump': (TILES, 0.5), 'bump_min': -0.02, 'bump_max': 0.02},
        'S8': {'normal': (TILES, None), 'glossiness': 0.6},
    }),
    'Calib-Mi-DzDefault5': ('Calib-Mi-DzDefault', {
        'S1': {'displacement': (TILES, 0.5), 'displacement_min': -0.05, 'displacement_max': 0.05},
        'S2': {'refraction_strength': (TILES, 0.8), 'ior': 1.4},
        'S3': {'refraction': (TILES, [1.0, 1.0, 1.0]), 'refraction_strength': 0.8, 'ior': 1.4},
        'S4': {'ambient': (TILES, [1.0, 1.0, 1.0]), 'ambient_strength': 1.0},
        'S5': {'reflection': (TILES, [1.0, 1.0, 1.0]), 'reflection_strength': (TILES, 0.6)},
        'S6': {'bump': 0.5, 'displacement': 0.5, 'glossiness': 0.6},
        'S7': {'diffuse': (TILES, [1.0, 0.8, 0.6]), 'specular_strength': (TILES, 0.8), 'glossiness': (TILES, 0.6), 'u_scale': 2.0, 'v_scale': 3.0},
        'S8': {'transparency': (TILES, 1.0), 'bump': (TILES, 0.5), 'normal': (TILES, None), 'glossiness': 0.6},
    }),
    'Calib-Mi-DzDefault6': ('Calib-Mi-DzDefault', {
        'S1': {'Lighting Model': 1, 'diffuse': [0.8, 0.4, 0.2], 'specular': [1.0, 1.0, 1.0], 'glossiness': 0.6},
        'S2': {'Lighting Model': 2, 'glossiness': 0.6},
        'S3': {'Lighting Model': 2, 'Subsurface Scatter Color': [0.8, 0.4, 0.2], 'Subsurface Sheen Color': [0.6, 0.6, 0.4], 'Subsurface Thickness': 0.6},
        'S4': {'Lighting Model': 3, 'glossiness': 0.6},
        'S5': {'Lighting Model': 4, 'specular_strength': 1.0, 'glossiness': 0.6},
        'S6': {'Lighting Model': 5, 'diffuse': [0.2, 0.4, 0.8], 'glossiness': 0.6},
        'S7': {'Subsurface Scatter Color': [0.8, 0.4, 0.2], 'Subsurface Sheen Color': [0.6, 0.6, 0.4], 'Subsurface Thickness': 0.6},
        'S8': {'specular': [0.0, 0.0, 0.0], 'glossiness': 0.0},
    }),
    # What the first three left open: the weight under a specular colour
    # map, refraction white without a map / coloured with one, what Matte
    # keeps, a glossiness map with a coloured specular or at strength 0,
    # tiling on bump and normal maps.
    'Calib-Mi-DzDefault7': ('Calib-Mi-DzDefault', {
        'S1': {'specular': (TILES, [0.6, 0.4, 0.2]), 'specular_strength': 0.5, 'glossiness': 0.6},
        'S2': {'refraction_strength': 0.8, 'refraction': [1.0, 1.0, 1.0], 'ior': 1.4},
        'S3': {'refraction_strength': (TILES, 0.8), 'refraction': [0.6, 0.8, 1.0], 'ior': 1.4},
        'S4': {'Lighting Model': 4, 'specular': [0.8, 0.6, 0.4], 'specular_strength': 0.5, 'glossiness': 0.3, 'diffuse_strength': 0.5},
        'S5': {'Lighting Model': 4, 'specular_strength': (TILES, 0.8), 'glossiness': (TILES, 0.6)},
        'S6': {'glossiness': (TILES, 0.6), 'specular': [0.8, 0.6, 0.4], 'specular_strength': 0.5},
        'S7': {'specular_strength': 0.0, 'glossiness': (TILES, 0.6)},
        'S8': {'normal': (TILES, None), 'bump': (TILES, 0.5), 'u_scale': 2.0, 'v_offset': 0.4},
    }),
    # When refraction shares the glossy inputs: a white refraction under a
    # coloured specular, a refraction colour equal to the specular's,
    # refraction with a glossiness map, under Matte.
    'Calib-Mi-DzDefault8': ('Calib-Mi-DzDefault', {
        'S1': {'refraction_strength': 0.8, 'refraction': [1.0, 1.0, 1.0], 'specular': [0.8, 0.6, 0.4], 'ior': 1.4},
        'S2': {'refraction_strength': 0.8, 'refraction': [1.0, 0.6, 0.4], 'specular': [1.0, 0.6, 0.4], 'ior': 1.4},
        'S3': {'refraction_strength': 0.8, 'refraction': [0.6, 0.8, 1.0], 'glossiness': (TILES, 0.6), 'ior': 1.4},
        'S4': {'Lighting Model': 4, 'refraction_strength': 0.8, 'refraction': [0.6, 0.8, 1.0], 'ior': 1.4, 'transparency': 0.6},
        'S5': {'Lighting Model': 4, 'transparency': 0.6},
        'S6': {'refraction_strength': 0.8, 'refraction': [0.6, 0.8, 1.0], 'ior': 1.4, 'transparency': 0.6},
    }),
})
# Scenes also saved with every channel shown (skill mi-fixture: a hidden
# channel and a shown one can reach Iray differently).
SHOWN = {'Calib-Mi-DzDefault6'}

# DAZ keeps a `color` channel (DAZ Default's, the scripted shaders') in 8
# bits: 0.3 reaches Iray as 77/255 - give those multiples of 0.2.
TYPES = {list: 'float_color', bool: 'bool', int: 'int', float: 'float', type(None): 'image'}

for name, (src, variants) in VARIANTS.items():
    d = json.load(gzip.open(os.path.join(scenes, src + '.duf')))
    geo = {g['id']: n.get('label', '') for n in d['scene']['nodes'] for g in n.get('geometries', [])}
    for m in d['scene']['materials']:
        label = geo.get(m['geometry'].rsplit('#', 1)[-1], '')
        over = variants.get(label.split(' ')[0], {})
        chs = next((e for e in m.get('extra', []) if e.get('channels') is not None), None)
        lib = next((l for l in d.get('material_library', []) if '#' + l['id'] == m.get('url')), {})
        if over.pop('USURFACE', None):
            over = {**{k: s[2] for k, s in USURFACE.items()}, **over}
        for k, v in over.items():
            image = None
            if isinstance(v, tuple):
                image, v = v
            # The DAZ Default shader keeps its channels at the top level
            # (`"glossiness": {"channel": …}`), the others in `extra`.
            if k in lib or k in m:
                c = m.setdefault(k, {'channel': {'id': k, 'type': TYPES[type(v)]}})['channel']
                c['current_value'] = v
                if image:
                    c['image_file'] = image
                continue
            if chs is None:
                chs = {'type': 'studio_material_channels', 'channels': []}
                m.setdefault('extra', []).append(chs)
            c = next((c['channel'] for c in chs['channels'] if c['channel']['id'] == k), None)
            if c is None:
                c = {'id': k, 'type': TYPES[type(v)]}
                entry = {'channel': c}
                if k in DEFS:
                    # A whole definition, as DAZ Studio 4 wrote it.
                    group, kind, default, *enum = DEFS[k]
                    c.update({'type': kind, 'name': k, 'label': k, 'value': default})
                    if enum:
                        c['enum_values'] = enum[0]
                    entry['group'] = group
                chs['channels'].append(entry)
            c['current_value'] = v
            if image:
                c['image_file'] = image
    duf_camera.ensure(d)  # else Iray renders DAZ's Perspective View (#97)
    out = os.path.join(scenes, name + '.duf')
    open(out, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
    print(out)
    if name in SHOWN:
        for m in d.get('material_library', []) + d['scene']['materials']:
            for c in [v['channel'] for v in m.values() if isinstance(v, dict) and 'channel' in v] + \
                    [c['channel'] for e in m.get('extra', []) for c in e.get('channels', []) or []]:
                c.pop('visible', None)
        out = os.path.join(scenes, name + '-Shown.duf')
        open(out, 'wb').write(gzip.compress(json.dumps(d, indent=1).encode()))
        print(out)
