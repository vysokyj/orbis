# 2026-11-12 — Genesis 9 eye: the dual lobe's weight map (#133)

Follows [2026-11-11-g9-eye-volume.md](2026-11-11-g9-eye-volume.md). With
the eye volume right (`-nogloss`), switching the dual lobe on (`-bare`)
added a grey sheen over the iris in Octane (+0.02 linear) that Iray
barely shows (+0.0014), with a much harder bump relief. No Iray render in
this session: the existing references (4e8780f) were enough.

## Decompose first

The issue's candidates, in order of what DAZ hands Iray:

- **The weight.** The G9 eye's `Dual Lobe Specular Weight` is 1 with a
  map, `G9_Eyes_SLW.jpg`: black on the sclera, ≤ 0.17 over the iris
  (mean ≈ 0.04). Calib-G9Eye's `.mi` (local, `eye66/mi`) passes the map's
  mono value itself as `pbr_material.dual_lobe_specular_weight`, gamma 1.
  The exported `project.ocs` had `specular` 0.85 and `coating` 0.15 as
  plain values: the map was dropped. The eye renders through Universal
  (`is_eye` keeps it off the OpenPBR skin path), and Universal's
  dual-lobe-alone branch wrote `[share × lobe1; 3]` with no map, while
  `skin_gloss` (OpenPBR) mapped it since #19. So the iris reflected
  ≈ 20 × Iray's gloss.
- **Specular occlusion** (`pbr_skin.mdl`: `lerp(1, facing, occlusion)`
  on the curve's head-on and grazing values) would also cut the gloss,
  but `specular_occlusion_enable` is false in DAZ's `.mi` (Calib-PBRSkin
  fixture) and the eye doesn't switch it on: not a component here.
- **Roughness / reflectivity**: 0.6, mult 0.4, ratio 0.15, reflectivity
  0.5 — PBRSkin defaults, mapped as on the skins.
- **Bump relief**: the normal map is the same on both; the relief was the
  full-strength gloss lighting it up. Nothing to change once the weight
  is right.

## Fix (no constant)

`dual_lobe_weight_pin`: the weight map × the pin's share, on Universal's
`specular` (lobe 1, or both lobes under a Top Coat / glossy coating) and
`coating` (lobe 2), the same helper `skin_gloss` now uses.

## Octane / Iray, enhanced-final, 400 samples

Medians of each channel, linear, regions as in the volume note (iris ring
r 70 – 115 px around (972, 607), sclera boxes left / right). The "before"
column is re-measured with this script (per-channel medians of the region
averaged, slightly different from the volume note's numbers).

| scene | iris before | iris after | sclera L after | sclera R after |
|---|---|---|---|---|
| -nogloss (unchanged) | 0.90 / 0.82 / 0.84 | — | 1.08 / 1.00 / 0.97 | 1.09 / 1.00 / 0.97 |
| -bare | 1.25 / 1.49 / 1.85 | 0.91 / 0.88 / 0.92 | 1.06 / 0.94 / 0.93 | 1.09 / 1.00 / 1.00 |
| Calib-G9Eye | 1.32 / 1.49 / 1.96 | 0.95 / 0.88 / 1.00 | 1.03 / 0.97 / 0.90 | 1.09 / 1.00 / 1.00 |

The gloss now adds +0.0012 over the iris (Iray +0.0008); the iris's
relief looks like Iray's side by side. ~3 min per scene.

## Concerns other issues

Any Iray Uber or PBRSkin material rendered on Universal with a mapped
Dual Lobe Specular Weight took the full gloss before this; on OpenPBR
skins it was already mapped. #137 (glossy + dual lobe 1.2 – 1.7 ×) and
#138 are on spheres without a weight map: not explained by this.
