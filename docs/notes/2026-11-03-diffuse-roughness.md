# 2026-11-03 Diffuse Roughness, overlay roughness (#32)

Goal: Iray Uber's Diffuse Roughness, Diffuse Overlay (Weight, Weight
Squared, Color, Roughness, Color Effect) and Diffuse Strength in Octane.
Scene: `Calib-Diffuse` (eight flat spheres, base red 0.45 / 0.05 / 0.04,
no gloss), Iray reference at 5000 samples (2026-10-30). Variant
enhanced-final, Octane as exported.

## What DAZ's MDL does

`diffuse_base` in `irayubermaterial.mdl`: the diffuse is a
`diffuse_reflection_bsdf` (Oren-Nayar) at `diffuse_roughness`; the
overlay a `weighted_layer` of a second one (overlay colour, its own
roughness) at the overlay weight over diffuse + translucency. The .mi
fixture (`calib/references/mi/Calib-Diffuse.json`) shows the values as
is: σ 0.5 / 1, overlay roughness 1, overlay weight 0.5 (0.25 squared).
Diffuse Strength is applied to the colour before the MDL.

## Measuring

Calib-compare-all's sphere boxes come from Calib-Glossy's nine-sphere
layout and don't fit this eight-sphere scene (nine numbers, the fifth
between two spheres), so the spheres were measured on their own: the
middle 60 % disc of each, at the eight centres, linear luminance (gamma
2.2), Octane / Iray.

Iray's spheres, linear luminance: O1 smooth 0.0128, O2 σ 0.5 0.0100,
O3 σ 1 0.0079 — 0.78 / 0.62 of the smooth one's. Oren-Nayar's A term
1 − 0.5 σ² / (σ² + 0.33) says 0.784 / 0.624.

## Baseline and variants

| sphere | before | `diffuseBrdf` 0 | 1 | **2** | 3 | 2, roughness 0 | final |
|---|---|---|---|---|---|---|---|
| O1 smooth | 1.08 | 1.08 | 1.08 | 1.08 | 1.08 | 1.08 | 1.08 |
| O2 σ 0.5 | 1.26 | 1.26 | 1.26 | 1.02 | 1.10 | 1.26 | 1.02 |
| O3 σ 1 | 1.49 | 1.48 | 1.48 | 0.97 | 1.12 | 1.49 | 0.97 |
| O4 strength 0.5 | 1.09 | | | | | | 1.09 |
| O5 overlay 0.5 white | 1.04 | | | | | | 1.04 |
| O6 overlay 1 blue | 0.97 | | | | | | 0.97 |
| O7 overlay squared | 1.04 | | | | | | 1.04 |
| O8 overlay σ₀ 1 | 1.40 | | | | | | 0.89 |

Variants by `orbx-variants.py set=…diffuseBrdf=e<n>+…roughness=f<σ>` on
O2 / O3. Octane's Oren-Nayar (2) at σ on the roughness pin darkens by
0.81 / 0.65 (Iray 0.78 / 0.62); 0 and 1 are Lambertian; 3 something
brighter (not used). With roughness 0 Oren-Nayar renders Lambert: it
reads Universal's one `roughness` pin, which the gloss, metal and
transmission also use. 28 – 29 s per render.

Diffuse Strength, the overlay's weight, Weight Squared and Color Effect
were already exported (#27); they match (1.04 – 1.09, like the smooth
sphere). The blue overlay's red channel is 0.53 × Iray, on a channel
near black (luminance 0.97).

## Rule

- No gloss / metal / refraction / transmission on the pin: Oren-Nayar
  (`diffuseBrdf` 2) at σ.
- Otherwise Lambertian × A(σ) — the light amount, not the shape (flatter
  terminator, back-scatter). On Calib-Diffuse it would give O2 / O3
  1.26 × 0.784 = 0.99 and 1.49 × 0.624 = 0.93.
- The overlay's share × A(σ₀) (÷ A(σ) under Oren-Nayar): O8 1.40 → 0.89,
  a bit dark (the Octane Oren-Nayar keeps ~4 % more than A).
- Not the skin path (`standard_surface`): its albedo is fitted on Iray
  renders that include the skin's Oren-Nayar.

132 of 420 user scenes set Diffuse Roughness on an Iray Uber material
(channel audit), the overlay roughness 12, an overlay weight map 83.

## Open

- Overlay with weight / colour maps: [#71](https://github.com/vysokyj/orbis/issues/71)
  (needs a .mi fixture and an Iray reference).
- Diffuse Roughness maps: the value only.
