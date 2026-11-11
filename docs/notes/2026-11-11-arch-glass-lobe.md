# 2026-11-11 — Architectural glass: Iray's rough transmit lobe, measured (#115)

Goal: replace the fitted `ARCH_GLASS_LOSS` (1 − 0.36 r_t, Calib-NVIDIA3's
HDRI at two refr_gloss values) by a factor derived as #113 derived resin's
(closed sphere in a white furnace, `libbsdf` vs Octane), check
`ARCH_GLASS_ROUGH` (3.7), and the uncoated resin's residual #113 left
(1.10 – 1.15 ×). Variant enhanced-final, Octane 1024 samples, Iray Server
5000.

## The MDL

`nvidia/architectural.mdl`: `custom_curve_layer(refl_weight, r0
brdf_0_degree_refl, r90, brdf_curve, coat, base)` — `fresnel_layer
(refr_ior)` with Use Fresnel Term — over `weighted_layer(transparency,
weighted_layer(translucency?, diffuse_transmission, simple_glossy_bsdf
(refr_color, architectural_gloss_to_rough(refr_gloss), scatter_transmit)),
diffuse)`; the coat a reflect-only `simple_glossy_bsdf` at
`architectural_gloss_to_rough(refl_gloss)`; closed solid of refr_ior.
`refl_weight` = reflectivity × the reflection colour's peak.

`libbsdf` (read, not copied):
- `scatter_transmit`: f_refl = 0 — every sample refracts, weight 1 (no
  Fresnel); a totally reflected one reflects (weight 1, min(G1, G2)).
- `curve_layer_sample`: the base's weight 1 − w · max(c(n·k1), c(nk2)),
  nk2 the reflection of k1 about the microfacet only when the curve has an
  IOR (`fresnel_layer`) and the event refracts; `custom_curve_layer`
  returns IOR −1 → nk2 = |n·k2|. So Schlick's layer has no critical angle.
- `base.mdl`'s `architectural_gloss_to_rough` is √(2 · 2^(−4 − 14 g)) in
  MDL SDK 2018.1, 2021, 2024.1 and master alike.

## Code

`src/resin.rs` → `src/mdl_layer.rs`: a `Stack` (coat weight, curve —
Fresnel or Schlick —, coat roughness, base scatter mode and roughness,
IOR); the coat samples its own microfacet when its roughness differs (the
same one otherwise, so resin's paths are bit-identical: all non-NVIDIA
Calib scenes export byte-identical, resin unchanged before the lobe term).

## 1. The model's prediction (dead end: libbsdf doesn't lose it)

White transmit lobe alone (reflectivity 0), sphere in the furnace, disk
r < 0.75: `libbsdf` and Smith at the same roughness are within 1 – 3 %
(r 0.5: 0.842 v 0.87; r 1 0.563 v 0.73 at α 0.71). At the formula's
roughness `libbsdf` keeps 1.00 / 1.00 / 0.99 / 0.87 at refr_gloss 0.6 /
0.4 / 0.2 / 0; at 3.7 r 1.00 / 0.995 / 0.85 / 0.56. Neither explains a
1.53 × at refr_gloss 0.2.

## 2. Scenes

`make-calib-scenes.py`: **Calib-ArchGlass** (HDRI + floor) and
**Calib-Furnace-ArchGlass** (dome 0.5, floor hidden) — Calib-NVIDIA3's
spheres all transparency 1: G1 – G5 refr_gloss 1 / 0.6 / 0.4 / 0.2 / 0 at
reflectivity 0 (the lobe alone), G6 / G7 reflectivity 0.6 at refr_gloss
1 / 0.2 (Schlick), G8 0.2 with Use Fresnel Term. **Calib-Furnace-Resin2**:
uncoated white resin r 0.02 / 0.1 / 0.25 / 0.5 / 0.75 / 1, grey r 0.5,
coated white r 0.25 (anchors R6 / R3). Dumps cloned from Calib-NVIDIA3's /
Calib-Furnace-Resin's (labels renamed). Iray: 81 s, 143 s, 72 s; the queue
DAZ restarted between the two architectural scenes (#94).

## 3. Furnace, disk r < 0.75 (sphere / dome)

| sphere | Iray | Octane, k = 1, α 3.7 r | Octane, k = 1, α ≈ r | `libbsdf` at r / 3.7 r |
|---|---|---|---|---|
| G1 refr_gloss 1 | 1.000 | 1.000 | 1.000 | 1.000 / 1.000 |
| G2 0.6 | 0.964 | 1.003 | 1.000 | 1.000 / 1.000 |
| G3 0.4 | 0.938 | 1.005 | 1.002 | 1.000 / 0.995 |
| G4 0.2 | 0.614 | 0.886 | 1.005 | 0.99 / 0.847 |
| G5 0 | 0.512 | 0.605 | 0.972 | 0.87 / 0.563 |
| G6 1, refl 0.6 | 0.997 | 0.999 | 1.000 | 0.981 |
| G7 0.2, refl 0.6 | 0.655 | 0.895 | 1.005 | — |
| G8 0.2, refl 0.6, Fresnel | 0.416 | 0.895 | 1.006 | — |

- Smooth glass keeps all in Iray (no scene constant here, unlike
  Calib-Furnace-Resin's 0.967 on a near-smooth resin).
- Octane keeps what the Smith model does (lobe at α 0.5: 0.886 v 0.87;
  its coating, whatever its IOR, loses nothing: G7 = G8).
- Iray's lobe loses far more than `libbsdf`'s. Roughness hypotheses for
  Iray's lobe, `libbsdf` disk: 3.7 r 1.00 / 0.995 / 0.847 / 0.563; 5 r
  1.00 / 0.98 / 0.74 / 0.56; 7 r 0.999 / 0.933 / 0.593 / 0.563 (G2 / G3 /
  G4 / G5). 7 r fits G3 / G4, not G2 (a flat 4 % over the disk, which no
  roughness draws: rings 0.99 0.97 0.96 0.96 0.96 0.96) nor G5. Ring
  profile G4 0.71 → 0.46 from centre to rim, steeper than `libbsdf` at any
  roughness. Iray's release notes mention legacy mia parameter matching
  for glossy transmission (exponent × 7 for thin-walled, removed 2021.0);
  Iray Photoreal's rough transmission isn't `libbsdf`'s. Can't be derived
  from what we can read: #116.

Uncoated resin (Calib-Furnace-Resin2, white): Iray 0.971 / 0.963 / 0.920 /
0.678 / 0.462 / 0.324 at r 0.02 / 0.1 / 0.25 / 0.5 / 0.75 / 1; `libbsdf`
1.000 / 0.999 / 0.973 / 0.824 / 0.655 / 0.531; Octane (k 1) 1.003 / 1.004 /
0.995 / 0.894 / 0.811 / 0.740 (Smith model 1.000 / 0.999 / 0.983 / 0.882 /
0.778 / 0.708). Same story: Iray's rough transmission loses more, ~3 % even
at r 0.02; no single roughness scale fits (1.5 r: 0.908 at 0.25 v 0.920,
0.655 at 0.5 v 0.678, 0.388 at 1 v 0.324).

## 4. Decomposed: lobe measured, layers derived

The lobe alone (decomposed: no coat, no lighting structure) is the part
`libbsdf` doesn't explain; measure it, keep the layering derived. Rule:
Iray = `libbsdf`'s stack with an extra tint τ on every base event, τ such
that the lobe alone keeps what Iray's lobe-alone furnace sphere does
(disk); k such that Octane's sphere keeps that (`tint_factor_measured`).
The check — only the layer model left, no number fitted:

| sphere | prediction | Iray |
|---|---|---|
| glass 0.2, refl 0.6 Schlick (τ 0.861 from G4) | 0.657 | 0.655 |
| glass 0.2, refl 0.6 Fresnel term | 0.421 | 0.416 |
| resin grey r 0.25 coated (τ 0.974 from U3) | 0.473 (0.496 without τ) | 0.473 |
| resin white r 0.25 coated | 0.722 (0.759) | 0.722 / 0.737 |
| resin grey r 0.1 coated (τ 0.982) | 0.605 (0.626) | 0.605 |
| resin grey r 0.02 coated (τ 0.985) | 0.627 (0.645) | 0.623 |
| resin grey r 0.5 coated (τ 0.917) | 0.240 (0.277) | 0.257 |

#113's "Iray 3 – 6 % below the model on every sphere" was the lobe.
Tables: `ARCH_GLASS_LOBE` (by refr_gloss, G5 – G1), `RESIN_LOBE` (by MDL
roughness, U1 – U6), linear. k (white): glass 0.98 / 0.97 / 0.85 / 0.93 at
refr_gloss 0.6 / 0.4 / 0.2 / 0, 0.85 under reflectivity 0.6, 0.73 with the
Fresnel term; resin grey coated 0.98 / 0.92 / 0.81 / 0.59 at r 0.02 / 0.1
/ 0.25 / 0.5, uncoated white 0.99 / 0.98 / 0.97 / 0.89.

`ARCH_GLASS_ROUGH` (3.7): not `architectural_gloss_to_rough` (unchanged)
nor masking (libbsdf's lobe at the formula's roughness loses almost
nothing); Iray's lobe is rougher-looking and lossier at once — kept as the
look's fit, part of #116.

## Results (Octane / Iray, disk r < 0.75; before → after)

| scene / sphere | before | after |
|---|---|---|
| Calib-ArchGlass G1 – G3 | 1.03 / 1.01 / 0.95 | 1.03 / 1.02 / 1.03 |
| G4 refr_gloss 0.2 | 0.99 | 1.07 |
| G5 refr_gloss 0 | **0.51** | 1.08 |
| G6 / G7 (refl 0.6) | 1.04 / 1.01 | 1.02 / 1.06 |
| G8 Fresnel term | **1.75** | 1.40 |
| Calib-Furnace-ArchGlass G4 / G5 / G7 / G8 (floor 1.02) | — | 1.07 / 1.04 / 1.05 / 1.19 |
| Calib-NVIDIA3 A6 / A7 / A8 | 1.01 / 1.02 / 1.05 (2026-11-05) | 1.02 / 1.10 / 1.10 |
| Calib-NVIDIA2 A6 glass / A7 resin / A8 | 1.04 / 1.05 / 0.95 | 1.05 / 1.00 / 0.93 |
| Calib-NVIDIA red resin | 0.99 | 0.95 (centre 0.83) |
| Calib-Resin R1 / R2 / R3 / R4 | 1.05 / 1.07 / 1.03 / 1.05 | 1.00 / 1.04 / 0.97 / 1.02 |
| R5 / R6 uncoated r 0.25 | 1.15 / 1.10 | 1.09 / 1.04 |
| R7 r 0.5 / R8 r 0.1 | 1.21 / 1.00 | 1.07 / 0.96 |
| Calib-Furnace-Resin2 U1 – U3 / U4 / U5 / U6 | — | 1.03 – 1.04 / 1.08 / 1.15 / 1.17 |

Floors 1.04 (HDRI), 1.02 (furnace). Hue unchanged (b/r, g/r within 0.03;
the red resin's g/r 0.24 v 0.19 as before). Calib-Glossy, -PBRSkin, -SSS,
-Glass, -Mixing, -TopCoat, -Leaf, -Thin-*, -ThinFilm export byte-identical
to the installed binary.

G8: the model itself gives disk 1.08 at a whole-sphere match — Iray's
critical-angle weights darken the centre more than Octane's level does;
with the HDRI 1.40. Like resin's rim (#113) and the uncoated r ≥ 0.75
(1.15), the spread of the loss isn't a constant tint: #117.

## Process lessons

- The lobe-alone furnace sphere (reflectivity 0 / glossy weight 0) is the
  decomposition that settled both questions: it showed the loss is the
  lobe (not layering) and gave the number the layer model then predicted
  without a fit. Put a "base alone" sphere in every layered furnace scene.
- A fit on a whole render (1 − 0.36 r_t) can hold at the two points it was
  fitted on and be 2 × off one step beyond (refr_gloss 0): add the ends
  of the range to the scene.
- `keep-refs.py` labelled the three queue renders "DAZ Studio Iray (CPU)"
  with no time (fixed by hand in renders.tsv from the queue log).
