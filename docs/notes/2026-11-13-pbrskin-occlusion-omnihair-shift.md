# 2026-11-13 PBRSkin Specular Occlusion / Detail roughness, OmniHair shift (#54)

Issue #54 bundled five unread channel groups from the coverage audit (#15).
Worked in the order the sources settle them; the rest went to their own
issues: the PBRSkin Detail Normal Map (#140), the OOT Hairblending 2 Cap
shader (#141, store content: no committable fixture), PBRSkin Makeup
(#142, never enabled in the library).

## PBRSkin Specular Occlusion and Detail roughness (from `pbr_skin.mdl`)

Read, no render fitted:

- `pbr_material`'s dual lobe sits in a `custom_curve_layer` whose head-on
  reflectivity is `lerp(1, specular_occlusion_facing, occlusion_weight) ×
  specularlevel` and grazing `lerp(1, specular_occlusion_grazing,
  occlusion_weight)` under `specular_occlusion_enable` (else F0 and 1).
- `roughness_comp = roughness × lerp(1, makeup_roughness_mult, makeup_weight)
  × lerp(1, roughness_detail, detail_weight)` before squaring; lobe 2's is
  `roughness_comp × roughness2`.
- The Detail Normal Map is blended onto the base normal by
  `base::blend_normals` at `detail_weight` (`pbr_detail_normal`).

The curve W (n + (g − n)(1 − cos)^5) equals a Schlick curve at weight W · g
and F0 n / g, so the existing dual-lobe mapping (weight + IOR from F0) takes
occlusion exactly: weight × g, F0 × facing / g. No constant.

**Fixture.** New generated scene `Calib-PBRSkin-Occlusion` (eight PBRSkin
spheres, red dual lobe at reflectivity 0.5, roughness 0.4): O1 reference;
O2 occlusion at DAZ's defaults (facing 0.6, grazing 0.1, weight 1); O3
grazing only; O4 facing only (0.25); O5 weight 0.5; O6 a grey map on the
occlusion weight; O7 Detail roughness mult 0.5; O8 Detail weight 0.5, mult
0.25, a tiled synthetic normal map. Its `.mi`:

| sphere | specular_occlusion_enable | occlusion_weight | facing / grazing | detail_enable | detail_weight / roughness_detail |
|---|---|---|---|---|---|
| O1 | false | 1 | 0.6 / 0.1 | false | 1 / 1 |
| O2 – O5 | (default: true) | 1 / 1 / 1 / 0.5 | as set | false | 1 / 1 |
| O6 | (true) | the map | 0.6 / 0.1 | false | 1 / 1 |
| O7 | false | 1 | 0.6 / 0.1 | (true) | 1 / 0.5 |
| O8 | false | 1 | 0.6 / 0.1 | (true) | 0.5 / 0.25 |

So DAZ passes the channels as saved; an enabled switch is left at the MDL
default rather than written. `src/iray.rs` gained `roughness_detail` and the
occlusion weight as float-or-map; the fixture test passes on all 696
PBRSkin arguments.

First generator run wrote the scene with every dual lobe **off**: the
generator redefines `DL` and `RED` further down for other scenes, and the
new block (appended at the end) picked those up. Spelled out locally.

Library survey: occlusion on in 4 scenes. The mapped G9 skins (Kojnice,
Yoga, industrial) sit at facing = grazing = 1 — a no-op whatever the map;
prob.duf uses facing 0.3, grazing default 0.1, weight 1, no map. Hence an
occlusion map counts by its value only (documented limit). Detail is on in
every Genesis 9 base skin (weight 1, mult 1: roughness unchanged, only the
normal map — #140).

Octane side rendered (enhanced-final, 400 samples,
`~/Library/Caches/orbis/calib/octane/enhanced-final/`); the Iray reference
waits in #54's batch. Expected on compare: O2 – O6 rims dimmer than O1 in
both, O7 / O8 sharper highlights; O8's normal map missing in Octane (#140).

## OmniHair Specular Reflection Shift → Octane `offset`

OmniHairBase hands `chiang_hair_bsdf` `cuticle_angle = shift × −0.0174533`
(radians; MDL: positive tilts the scales toward the root). Octane's Hair
(147) `offset` pin (probe: default 0, slider 0 – 1, no unit in its
description or the manual).

Unit: Calib-OmniHair-brown (private scene, local Iray reference, shift 3)
exported once, `orbx-variants.py hair-offset=<v>` (new hypothesis), 400
samples, denoised crown crop:

| offset | look | corr(Octane(v) − Octane(0), Iray − Octane(0)) | mean abs error vs Iray |
|---|---|---|---|
| −0.15 | | 0.13 | 0.0241 |
| −0.0524 | | 0.21 | 0.0228 |
| 0 | | – | 0.0228 |
| 0.0524 | next to 0 | 0.52 | 0.0199 |
| 0.15 | | 0.58 | 0.0167 |
| 0.5 | crown turned yellow-bright | | |
| 3 | crown grey (wrapped) | | |

Radians (0.5 rad = 29° changes everything, 3 is nonsense; in degrees both
would be invisible). Sign: Iray's difference follows the positive offset,
so Octane's positive offset is DAZ's positive shift (the opposite of MDL's
sign). Rule: `offset = shift.to_radians()`. The larger correlation at 0.15
is not a reason to scale it: the crown is also brighter overall in Octane
here (mean Y 0.073 vs 0.054 in this crop), which any darkening offset
partly "explains".

Timings: four hair variants 2 – 3 min each in Octane, half the crop.

## Process

- Check a generator's shared names before appending a block at the end.
- A dump is keyed by the scene's path: dump and export the same copy
  (`calib/scenes/…`), the library copy has none.
