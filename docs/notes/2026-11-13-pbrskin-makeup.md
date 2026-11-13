# 2026-11-13 PBRSkin Makeup layer (#142)

Split from #54: PBRSkin's Makeup channels (Base Color, Weight, Roughness
Mult, Metallicity Enable) were read by nothing on the Octane side. The
library sets them in 9 scenes, Makeup Enable is on in none - so this is
done from the MDL alone, with an Iray reference parked for the check.

## What the MDL does (read, no fit)

`pbr_skin.mdl`'s `pbr_material`, under `makeup_enable` (and
`diffuse_enable`):

- `diffuse_makeup_bsdf` = `weighted_layer(makeup_weight,
  diffuse_reflection_bsdf(makeup_color, diffuse_roughness),
  diffuse_translucency_bsdf, state::normal())` - over the diffuse **and**
  its translucency (so the makeup covers the light that would enter the
  volume), under the dual lobe's `custom_curve_layer`, the metal's
  `weighted_layer` and the Top Coat (`pbr_add_clearcoat`).
- `roughness_comp = roughness × lerp(1, makeup_roughness_mult,
  makeup_weight) × lerp(1, roughness_detail, detail_weight)` (#54 took
  the Detail half).
- The metal: `directional_factor(normal_tint: metallic_makeup_enable &&
  makeup_enable ? blend_colors(basecolor, makeup_color,
  color_layer_blend, makeup_weight).tint : basecolor, dual_specular_bsdf)`
  - a plain lerp of the tints.

Every layer above the makeup is linear in its base (W·F·layer + (1 −
W·F)·base for the curve, w·layer + (1 − w)·base for the weighted
layers). So a Mix material at Makeup Weight of two whole materials - one
in the makeup's colour without translucency / subsurface, one without
makeup - is the MDL's stack over the blended base exactly, whatever the
base is (Universal, the OpenPBR skin, a Layered coat). The metal tints
mix the same way: makeup side tinted by the makeup gives lerp(base,
makeup, w); without Makeup Metallicity Enable the makeup side is itself a
Mix by the Metallic Weight of the metal in the base colour and the
makeup diffuse. The roughness factor is one value in Iray: exact on both
sides at a flat weight; with a weight map (a mask) the makeup side at
the full mult and the base at 1 - exact at mask 0 / 1, two roughnesses
mixed in between.

Not representable: the makeup diffuse uses the unbumped normal
(`state::normal()`), the dual lobe over it the bumped one; Universal has
one normal pin. Documented as a limit.

## Scenes and the `.mi`

Generated (`make-calib-scenes.py`, new block at the end with its own
`MK_DL` / `MK_RED` - the script redefines `DL` and `RED` above, #54's
trap): the red dual-lobe base of Calib-PBRSkin-Occlusion (reflectivity
0.5, roughness 0.4), makeup blue (0.1, 0.3, 0.8).

`Calib-PBRSkin-Makeup`: K1 tie (= O1), K2 weight 1, K3 0.5, K4 the
weight map (`calib-overlay-weight.png`, bands 0 / 0.25 / 0.5 / 1), K5
w 0.5 mult 0.25, K6 w 1 mult 2 (the channel's max), K7 metal 1 + Makeup
Metallicity at w 0.5, K8 metal 1 without it.

`Calib-PBRSkin-Makeup-TL`: L1 translucency 0.8 (grey), L2 + makeup 0.5,
L3 + the weight map, L4 a Top Coat (Top Coat Enable on, 0.5, r 0.2), L5 +
makeup 0.5, L6 the subsurface skin path (translucency 0.8, Sub Surface
Enable, a white diffuse map × red - the skin path needs a diffuse map),
L7 + makeup 0.5, L8 + the weight map.

`pbr_spheres` learned a `('diffuse', 'image')` key for L6 – L8; the
other scenes it writes are unchanged (only the two new files were
written).

DAZ's `.mi` (fixtures `calib/references/mi/Calib-PBRSkin-Makeup*.json`):

| | makeup_enable | makeup_weight | makeup_color | makeup_roughness_mult | metallic_makeup_enable |
|---|---|---|---|---|---|
| K1, L1, L4, L6 | false | 1 | (1, 1, 1) | 1 | - |
| K2 – K8, L2 / L5 / L7 | (default true) | as saved | (0.00658, 0.0718, 0.612) | as saved | K7 true, else - |
| K4, L3, L8 | (true) | the map, gamma "auto" | as above | 1 | - |

- **Makeup Base Color is 8-bit.** 0.1 / 0.3 / 0.8 reached Iray as 0.00658
  / 0.0718 / 0.612 = (26 / 77 / 204 ÷ 255)^2.2, not 0.1^2.2 = 0.00631:
  `PBRSkin.dsf` types it `color` (8 bits), `diffuse` / Translucency /
  Transmitted are `float_color`. Top Coat, SSS and Flakes Color are
  `color` too; unproven (the fixtures use 8-bit-exact values) - #152.
- A switch DAZ leaves at the MDL default is not written: `makeup_enable`
  only when false, `metallic_makeup_enable` only when true,
  `sub_surface_enable` only when false (L6 – L8 have none). The fixture
  test now takes a missing switch as `pbr_skin.mdl`'s default.
- `iray::pbrskin` carries `makeup_color`, `makeup_roughness_mult`, the
  weight as float or map and both switches; 1242 PBRSkin arguments agree.

Process: the first `.mi` run exported both scenes in one DAZ session; the
second's PBRSkin materials all came out as a default Uber
(`shader_decl.mi` without `pbr_skin.mdl`). Alone in its own session it
was right - #153.

## Octane side

Export (`makeup_mix_material`): K2 the makeup side alone (weight 1, no
map); K3 – K8, L2, L3, L5, L7, L8 a Mix (19) at the weight or its map.
Rendered enhanced-final, 400 samples, about 1 min a scene
(`~/Library/Caches/orbis/calib/octane/enhanced-final/`, denoised pass).
Sphere cores (r < 0.5 of the disc, discs from Calib-PBRSkin-Occlusion's
Octane render: same geometry, all red; #146), linear RGB:

| sphere | R | G | B | | sphere | R | G | B |
|---|---|---|---|---|---|---|---|---|
| K1 tie | 0.110 | 0.022 | 0.027 | | L1 translucent | 0.056 | 0.044 | 0.057 |
| K2 w 1 | 0.015 | 0.049 | 0.387 | | L2 + w 0.5 | 0.027 | 0.039 | 0.215 |
| K3 w 0.5 | 0.050 | 0.030 | 0.192 | | L3 + map | 0.035 | 0.034 | 0.114 |
| K4 map | 0.063 | 0.020 | 0.088 | | L4 top coat | 0.080 | 0.019 | 0.027 |
| K5 w 0.5 m 0.25 | 0.039 | 0.024 | 0.162 | | L5 + w 0.5 | 0.043 | 0.029 | 0.168 |
| K6 w 1 m 2 | 0.011 | 0.032 | 0.274 | | L6 skin | 0.039 | 0.034 | 0.048 |
| K7 metal 0.5 tint | 0.027 | 0.015 | 0.125 | | L7 + w 0.5 | 0.019 | 0.026 | 0.151 |
| K8 metal 0.5 | 0.034 | 0.010 | 0.066 | | L8 + map | 0.027 | 0.028 | 0.082 |

K1 = Calib-PBRSkin-Occlusion's O1 to the 4th digit (0.1095 / 0.0215 /
0.0270): the path without makeup is unchanged. Over translucency the
makeup side dominates at w 0.5 (L2, L7), as the MDL says: the base side
keeps only 0.2 of its red diffuse, the makeup covers the translucency
too. The first render had K7 / K8 at metal 1, which hides the diffuse
entirely (K8 was a plain red metal); regenerated at metal 0.5 before
anything was committed.

## Where it stands

Iray reference parked: both scenes at 5000 samples (PBRSkin built-in, no
NVIDIA preset). Resume: compare K2 – K8 / L2 – L8 against their ties (K1,
L1, L4, L6) and Iray; the Mix should hold the colour at the sphere
centres within the frame's offset, K5 / K6 the highlight's spread, K4 /
L3 / L8 the bands.
