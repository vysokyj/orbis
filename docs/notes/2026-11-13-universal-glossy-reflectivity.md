# 2026-11-13 Universal glossy reflectivity as an IOR (#100)

Goal: the glossy half of #100. #81 moved Universal's dual lobe to the MDL
curve (weight W, the reflectivity as the IOR of F0); the glossy layer
still took Glossy Reflectivity r as a share of the specular, `specular ×
min(2r, 1)` at Universal's default IOR 1.5, plus the excess `2r − 1` as a
clear coating for r > 0.5 (`reflectivity_share`, `glossy_excess`), and
ignored a Glossy Reflectivity map. Variant enhanced-final, 400 samples,
against the existing Iray references (no new Iray).

## What Iray does (docs first)

`irayubermaterial.mdl`, `metallic_layered_base`: the glossy layer is
`add_top_coat` in `tcw_custom_curve` mode, normal reflectivity
`glossy_reflectivity`, grazing 1, exponent 5, weight Glossy Layered
Weight - Schlick's curve with F0 = `glossy_reflectivity`. DAZ hands Iray
0.08 r in Base Mixing 0 (`src/iray.rs`, Calib-Glossy's `.mi`: r 0 / 0.25
/ 1 → 0 / 0.02 / 0.08) and max(Glossy Specular) in Base Mixing 1 (the
same layer after DAZ's conversion - already mapped as the IOR of that F0,
`specular_ior`, #52). Weighted mixing (2) has no curve (#132).

A Glossy Reflectivity **map**: no generated scene has one; a store
outfit's `.mi` (a private scene, local only) shows DAZ passing it as a
`base::file_texture` with `color_scale` 0.08 × r, gamma 1, mono average -
the same F0 × map the skin path (`skin_glossy_layer`) already bakes.

## Analytic check: Schlick against Universal's Fresnel at the IOR of F0

Universal's dielectric Fresnel is the physical one at its `ior`. Cosine-
weighted hemispherical reflectance (Octane / Schlick), and the old share
form:

| r | F0 | n | Schlick | IOR form | share form (min(2r,1) at 1.5, + excess) |
|---|---|---|---|---|---|
| 0 | 0 | 1.00 | 0.048 | 0 (0.00) | 0 (0.00) |
| 0.05 | 0.004 | 1.14 | 0.051 | 0.032 (0.63) | 0.009 (0.18) |
| 0.1 | 0.008 | 1.20 | 0.055 | 0.044 (0.79) | 0.018 (0.33) |
| 0.25 | 0.02 | 1.33 | 0.067 | 0.066 (0.99) | 0.046 (0.69) |
| 0.5 | 0.04 | 1.50 | 0.086 | 0.092 (1.07) | 0.092 (1.07) |
| 1 | 0.08 | 1.79 | 0.124 | 0.133 (1.07) | coating stacked: ~1.5 |

Per angle the IOR form follows Schlick within a few % from r 0.25 up
(r 1 at 80°: 0.418 v 0.434; the share form 0.775). Below r ~0.2 the
physical Fresnel rises later than Schlick's (1 − cos)^5, and at r 0 (n 1)
it reflects nothing - Universal has no pin that gives a Schlick curve, so
that residual stays.

## Calib-Glossy, decomposed

The spheres are red (0.45, 0.05, 0.04) under a white gloss: per pixel the
gloss is (mean(G, B) − k R) / (1 − k), k the matte G1's G/B over R; the
diffuse is R − gloss. Core = inside 0.7 of the radius, rim 0.7 – 0.95.
Denoised pass (raw within 0.01). Sphere order G1 matte, G2 r 0.5, G3 r
0.25, G4 r 1, G5 r 0, G6 r 0.5 rough 0.1, G7 rough 0.6, G8 dual lobe.

| sphere | diffuse O/I before → after | gloss O/I before → after | gloss / diffuse before → after |
|---|---|---|---|
| G1 matte | 1.07 → 1.07 | (no gloss) | |
| G2 r 0.5 | 1.08 → 1.08 | 1.13 → 1.13 | 1.05 → 1.05 |
| G3 r 0.25 | 1.15 → 1.10 | 0.99 → 1.13 | 0.86 → 1.03 |
| G4 r 1 | 0.91 → 1.05 | 1.23 → 1.12 | 1.35 → 1.07 |
| G5 r 0 | 1.26 → 1.25 | 0.16 → 0.16 | 0.13 → 0.13 |
| G6 rough 0.1 | 1.09 | 1.15 → 1.16 | 1.06 |
| G7 rough 0.6 | 1.14 | 1.03 | 0.90 |
| G8 dual lobe | 0.92 | 1.31 | 1.42 |

Whole spheres (`calib-compare-all.py`): 1.08 1.11 1.13 1.25 1.11 1.16
1.13 1.08 → 1.08 1.11 1.14 1.17 1.10 1.16 1.13 1.08.

- r 0.25 and r 1 now sit with r 0.5 (1.03 / 1.07 v 1.05), as the
  analytic table predicts (relative to r 0.5: 0.92 / 1.00 predicted, 1.00
  / 0.99 measured). The r 1 sphere's diffuse came back from 0.91 to 1.05:
  the excess coating had darkened the base under it as well.
- r 0: unchanged - no gloss in Octane either way (share 0 before, IOR 1
  now). Iray's r 0 gloss is real: 27 % of r 0.5's on the core, more than
  Schlick's (1 − cos)^5 gives there; the sphere as a whole stays at 1.10
  because Octane's diffuse keeps what Iray's curve takes. Left as the
  residual (no floor fitted; Base Mixing 1 has a measured floor, IOR 1.1,
  `SPECULAR_MIN_IOR`, for its black specular - the same curve at F0 0).
- #137 (the glossy / dual lobe layers alone 1.2 – 1.7 × Iray, by
  position): on this scene the diffuse ratio alone swings 1.07 – 1.15
  across the Uber glossy spheres' places (#87's light-at-position effect),
  and their gloss sits at 1.03 – 1.16; the dual lobe sphere G8 is the
  outlier (gloss 1.31, over its diffuse 1.42) - its diffuse is the lowest
  (0.92), so position does not explain it here.

## Regression scenes

Before (HEAD's binary) and after, same dumps, 400 samples, denoised:
Calib-Map-Metal (r 1 metal), Calib-L-Distant-45 (the Calib-Materials
spheres, r 0 – 1), Calib-G9Eye (r 0.247), Calib-DualLobeGlossy (gloss over
a dual lobe, r 1). Each frame 1.000 after / before; 0.07 – 0.11 % of the
pixels move by more than 0.01 (noise), the moved pixels' after / Iray
1.06 / 1.00 / 1.05 / 1.22 v 1.06 / 1.00 / 1.03 / 1.22 before. No rule
recorded for them moves.

## Where it stands

Universal's glossy layer = weight on `specular` (or the `coating` over a
dual lobe), F0 = 0.08 r × map as the IOR; `reflectivity_share` and
`glossy_excess` are gone; the map is baked per texel (`ior_pin`). Open:
the low-r residual (r < 0.2) has no exact Universal form.

Process: the decomposition by colour (white gloss over a red diffuse)
needs no extra Iray variant; it reads 0.03 of "gloss" on the matte rim
(coloured sky light), so judge the rim against G1's.
