# 2026-11-16 PBRSkin metal without its dual lobe (#178)

Issue #178 (found in #167): with Metallicity on and Dual Lobe Specular
Enable off, the export wrote one Universal at Metallic Weight w with a
metal at lobe 1's roughness (Calib-PBRSkin-Metal N7: metal 0.5, pin
0.496). Settled from the MDL and the `.mi`, without an Iray render.

## 1. What Iray does (`pbr_skin.mdl`)

- `dual_specular_bsdf` = `weighted_layer(lobe 2, lobe 1)` with both
  lobes enabled, `specular_bsdf` with lobe 2 off, and `bsdf()` (black)
  with `dual_lobe_specular_enable` false.
- The surface: `weighted_layer(metallic, directional_factor(base colour,
  base: dual_specular_bsdf), base: diffuse_specular_bsdf)`. Without the
  dual lobe the metal's layer is `directional_factor` over black: it
  reflects nothing, and the surface is (1 - w) x the diffuse stack
  (diffuse, translucency, makeup); the Top Coat (`pbr_add_clearcoat`)
  lies over the whole of it.
- Dual Lobe Specular Weight only weighs the dielectric `custom_curve_layer`;
  0 with the lobe enabled leaves the metal's lobes (not this case).

## 2. What DAZ hands Iray (`.mi`)

New fixture `calib/references/mi/Calib-PBRSkin-Metal.json` (headless
export, 1 scene): N7 `metallic` 0.5, `metallic_enable` absent (MDL
default true; N1, metal off, has it as a literal false),
`dual_lobe_specular_enable` false, `roughness` 0.4, `roughness2_weight`
0.15. N6's `dual_lobe_specular_weight` 0.5 reaches the MDL too (the
metal ignores it there). `src/iray.rs`'s fixture test passes unchanged
(the enables are not modelled in `iray::pbrskin`).

## 3. Shipped

`pbr_metal_unlobed` (src/material.rs): a PBRSkin metal with the dual lobe
disabled goes through `translucent_metal_material` (opaque too): a Mix
by the Metallic Weight (its map as the amount) of a black part
(`black_part`, the old Uber black node factored out: albedo and specular
0, the Top Coat, the opacity) and the base at metal 0. `metal_part` at
1 writes the black part for such a material, so the makeup's metal side
follows. No constant.

## 4. Octane side

Calib-PBRSkin-Metal, enhanced-final, 400 samples, Octane's denoised pass,
mean linear luminance per sphere (core < 0.5 R, rim 0.8 - 0.93 R):

| sphere | before core / rim | after core / rim |
|---|---|---|
| N7 metal 0.5, dual lobe off | 0.0122 / 0.0332 | 0.0072 / 0.0092 |
| N1 - N6, N8 | | equal to 0.0001 |

The rim's metal highlight is gone (Iray's metal there has no lobe), the
core keeps half the red diffuse. No Iray reference yet: N7 is in #167's
`## Iray batch` (5000, held back for a queue-side DAZ restart); the new
render replaced `~/Library/Caches/orbis/calib/c167/metal/` (raw and
denoised; the `__brdf=2` variant there still has the old N7). On the
reference, N7 should match within the row's usual metal ratio (N2's).
Render 30 s.

## Process

- The issue asked to wait for Iray's N7; the MDL plus the `.mi` already
  pin it (a layer over `bsdf()` is black by definition), so the Iray
  reference in #167's batch is a check, not the decision.
