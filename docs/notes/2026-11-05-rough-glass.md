# 2026-11-05 — Rough glass: Beckmann at √r

Issue [#72](https://github.com/vysokyj/orbis/issues/72): rough refraction
without dispersion rendered 1.14 – 1.19 × Iray (Calib-Dispersion-Rough R5,
R7), R7 less frosted than Iray, R1 sharper. Scenes: Calib-Dispersion-Rough
(Iray, 5000 samples, 2026-11-04), checks on Calib-Glass, Calib-Glossy,
Calib-Dispersion, Calib-RS-PathLen4 (existing Iray references). Variant
enhanced-final, Octane 256 samples (`octane-render.sh`), no new Iray
renders.

Metric per sphere disc (radius 36 px, linear luminance, gamma 2.2): mean
luminance ratio Octane / Iray, and the detail metric of the dispersion
note (mean |blur 1.5 px − blur 4 px| / luminance × 100). The clear glass
spheres run 1.02 – 1.04 × Iray in every scene (the frame).

## What the docs say

- DAZ's Uber MDL (`refraction_base`, `choose_bsdf`): with the default
  `dfm_glossy` every refraction lobe — shared roughness (a `fresnel_layer`
  of the glossy reflection over a transmit-only lobe) or its own — is
  `df::simple_glossy_bsdf`, and `choose_bsdf` never passes
  `multiscatter_tint`.
- MDL spec 21.1.3: `simple_glossy_bsdf` is a Phong distribution, exponent
  2 / roughness², with v-cavities shadowing; rough microfacet lobes lose
  albedo for missing multiple scattering, which only `multiscatter_tint`
  (default 0) adds back. So Iray's rough glass is single-scatter and
  loses light.
- Octane manual (BRDF Models): GGX energy preserving adds multiple
  scattering ("no energy is lost"); "Octane" (`brdf` 0, Universal's
  default) is the pre-3.07 model, "not physically accurate".
- The export so far: shared roughness → the gloss rule, GGX energy
  preserving (6) at `ggx_roughness`; own roughness → `brdf` 0 at r × 0.3
  (a Calib-Glass R6 fit).
- Phong of exponent n ≈ Beckmann at α = √(2 / (n + 2)) (Walter et al.
  2007); with n = 2 / r², α = r / √(1 + r²) ≈ r. Octane's roughness pin is
  √α (the gloss curve already ends at √r). So the analogue of Iray's lobe
  is single-scatter Beckmann at roughness √r — no constant to fit.

## Attempts (Calib-Dispersion-Rough, ratio / detail; Iray detail in brackets)

| variant | R1 gloss 0.3 [8.3] | R3 own 0.3 [5.4] | R5 gloss 0.6 [1.2] | R7 own 0.6 [1.5] | Glass R6 own 0.3 [6.4] |
|---|---|---|---|---|---|
| before (GGX-EP `ggx_roughness` / Octane × 0.3) | 1.02 / 11.3 | 1.03 / 5.7 | **1.19** / 1.2 | **1.14** / 2.6 | 1.04 / 6.7 |
| v1 shared → plain GGX (2) | 1.02 / 11.2 | — | 0.86 / 1.2 | — | — |
| v2 both plain GGX, `ggx_roughness`, no × 0.3 | 1.02 / 11.2 | 1.03 / 7.5 | 0.86 / 1.2 | 0.85 / 1.4 | 1.03 / 8.8 |
| v3 own → GGX-EP, `ggx_roughness` | — | 1.03 / 7.5 | 1.19 / 1.2 | 1.19 / 1.3 | 1.03 / 8.8 |
| v4 plain GGX at √r | 1.02 / 6.4 | 1.01 / 4.2 | 0.86 / 1.2 | 0.85 / 1.4 | 1.02 / 5.0 |
| **v5 Beckmann (1) at √r** | 1.04 / 8.4 | 1.03 / 5.4 | **1.09** / 1.4 | **1.07** / 1.5 | 1.03 / 6.4 |
| v6 GGX-EP at √r | 1.04 / 6.3 | 1.03 / 4.2 | 1.19 / 1.2 | 1.19 / 1.3 | 1.04 / 5.0 |
| v7 STD (7) at √r | 1.04 / 8.0 | 1.03 / 5.2 | 1.07 / 1.3 | 1.07 / 1.5 | 1.03 / 6.2 |

(— : unchanged from before.) Reading:

- Brightness is the BSDF's multiple scattering: energy preserving GGX
  gives back what Iray's lobe loses (1.19 whatever the roughness curve);
  Octane's legacy model at × 0.3 sits there too (1.14). Plain GGX loses
  more than Iray (0.85 – 0.86) — its long tail sends more of the lobe to
  grazing microfacets that are masked / totally reflected.
- Blur is the roughness curve: GGX's long tail at √r blurs too much at
  r 0.09 (4.2 – 5.0 against 5.4 / 6.4), `ggx_roughness` (fitted to
  reflection highlights) too little (7.5 / 8.8, R1's 11.3).
- Beckmann at √r matches the blur at both roughnesses without a constant
  and is the closest single-scatter model in brightness. STD (default
  spread) lands within noise of it but has a free parameter and no MDL
  counterpart; Beckmann is the model Phong converts to.

What is left at r 0.36 — 1.09 / 1.07 against the frame's 1.03 — fits
Smith shadowing (Octane) vs v-cavities (Iray); Octane has no v-cavities
option, so it stays.

## Rule (spec, Refraction)

Refraction ≥ 0.5, not thin-walled (shared or own roughness, any glossy
layer on it too) → `brdf` 1 at √r; `REFRACTION_ROUGHNESS_SCALE` (0.3) is
gone. Thin walls, refraction < 0.5 and Top Coats keep their rules.

## Final check (new binary, 256 samples)

| scene | before | after |
|---|---|---|
| Calib-Dispersion-Rough R5 / R7 | 1.19 / 1.14 (detail 1.2 / 2.6) | 1.09 / 1.07 (1.4 / 1.6) |
| Calib-Dispersion-Rough R1 / R3 | 1.02 / 1.03 (11.3 / 5.7) | 1.04 / 1.03 (8.3 / 5.4) |
| Calib-Dispersion-Rough dispersive R2/R4/R6/R8 | 1.01 – 1.03 | 1.01 – 1.03 |
| Calib-Glass (8 spheres) | R6 1.04 / 6.7, others as below | R6 1.03 / 6.4; all other spheres within 0.003 |
| Calib-Dispersion (smooth, 8) | 1.03 | 1.03 |
| Calib-RS-PathLen4 | — | every sphere within 0.007 |
| Calib-Glossy (no refraction) | — | identical within 0.002 |

Calib-Glass R4 (absorbing, 0.55) and R7 (half refraction, 0.86) are as
they were — not rough glass, out of scope.

Timings: one render pass of 2 scenes × 4 variants ~4 min; each scene
20 – 36 s at 256 samples.

## Process lessons

- The two numbers in the issue (brightness and blur) had two different
  causes (BSDF model vs roughness curve); splitting them over a 2 × 4 grid
  of variants answered both in two render batches.
- An environment-variable switch in a scratch build kept the variants
  cheap; removed before the commit.
