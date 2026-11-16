# 2026-11-16 — PBRSkin's rim is Schlick's curve, not the lobes (#188)

#188 (from #167's resume, `2026-11-16-pbrskin-metal-lobes.md` §6): every
PBRSkin dual-lobe sphere on a red base came out 1.30 – 1.45 × Iray in the
blue channel at the rim (0.8 – 0.93 R), where the red base leaves almost
only the white specular, in every lobe model (Beckmann, GGX EP, GGX); the
matte sphere 1.09. The question: is that the dielectric's level at grazing
(`custom_curve_layer` against Universal's Fresnel), and how much of it is
left for #186 (GGX vs GGX EP) once it is split out. No new Iray: the
references in `calib/references/iray` (5000) and the #167 Octane renders
(`~/Library/Caches/orbis/calib/c167/r2`, enhanced-final 400, denoised
pass) plus two new Octane renders of Calib-PBRSkin-Occlusion.

## 1. Docs

- `pbr_skin.mdl`: `diffuse_specular_bsdf` = `custom_curve_layer(
  normal_reflectivity: facing_comp × specularlevel, grazing_reflectivity:
  grazing_comp, weight: Dual Lobe Specular Weight, layer:
  dual_specular_bsdf, base: the diffuse)`, exponent the default 5; both
  lobes `microfacet_ggx_smith_bsdf` at α = r² (lobe 2 at the ratio,
  `weighted_layer`). Without Specular Occlusion facing_comp = grazing_comp
  = 1. The `.mi` (Calib-PBRSkin-Metal N1): `specularlevel` 0.04,
  `specular_occlusion_enable` false, roughness 0.4, roughness2 0.4,
  roughness2_weight 0.15.
- MDL spec 1.11 §21.6 `custom_curve_layer`: a Schlick-style curve
  normal + (grazing − normal)(1 − cos)^exponent; the base weighted
  1 − weight · curve. `libbsdf`'s `curve_layer_sample` (as
  `src/mdl_layer.rs` documents it) weights the layer w · c(k · h), the
  curve at the microfacet, the base 1 − w · max(c(n · k1), c(n · k2)).
- The export (spec, Dual lobe specular; `DualLobe::of`): the layer's
  weight on the pins, the IOR of F0 (n / g under occlusion) on `ior` /
  `coatingIor`: Universal's physical Fresnel. #137 measured that curve at
  1.12 – 1.30 × Schlick's at 45 – 60° and kept it (spec, Glossy
  Reflectivity).

## 2. The curve alone, per ring (`scripts/dev/rim-curve.py model`)

The rim's view angles are asin(0.8 – 0.93) = 53 – 68°, exactly where
Fresnel / Schlick peaks. Model: the dual lobe's directional albedo in a
white furnace, per ring of a sphere seen head-on (area-weighted), on
Iray's own distribution (GGX at α = r², lobe 2 at the ratio, VNDF
sampling), with Octane's curve (g · Fresnel at the IOR of n / g) over the
MDL's (n + (g − n)(1 − k · h)^5). Same lobes on both sides, so the ratio
is the curve and nothing else. Nothing fitted.

| lobes (α1, n, ratio, α2, g) | 0 – 0.3 | 0.3 – 0.5 | 0.5 – 0.6 | 0.6 – 0.7 | 0.7 – 0.8 | 0.8 – 0.85 | 0.85 – 0.9 | 0.9 – 0.93 |
|---|---|---|---|---|---|---|---|---|
| P2 (0.36, 0.02, 0.15, 0) | 1.03 | 1.06 | 1.11 | 1.15 | 1.19 | 1.17 | 1.11 | 1.05 |
| P3 (0.36, 0.04, 0.15, 0) | 1.02 | 1.04 | 1.08 | 1.13 | 1.18 | 1.21 | 1.20 | 1.16 |
| N1 / K1 / O1 (0.16, 0.04, 0.15, 0.026) | 1.01 | 1.03 | 1.08 | 1.14 | 1.22 | 1.25 | 1.21 | 1.15 |
| O2 occlusion default (n 0.024, g 0.1) | 1.00 | 1.00 | 1.01 | 1.01 | 1.01 | 1.01 | 0.99 | 0.96 |
| O3 grazing 0.1 (n 0.04, g 0.1) | 1.00 | 1.00 | 1.00 | 0.99 | 0.98 | 0.96 | 0.93 | 0.89 |
| O4 facing 0.25 (n 0.01, g 1) | 1.01 | 1.05 | 1.10 | 1.14 | 1.10 | 0.99 | 0.88 | 0.80 |
| O5 occlusion w 0.5 (n 0.032, g 0.55) | 1.01 | 1.03 | 1.06 | 1.11 | 1.18 | 1.22 | 1.21 | 1.17 |

## 3. Measured over the model (`rim-curve.py measure`)

Blue channel Octane / Iray per ring; on Calib-PBRSkin minus the matte
P1's blue (Octane's red diffuse leaves 2.0 – 2.2 × Iray's blue, 0.0005
linear: 4 – 20 % of a lobe's blue), frame = P1's red per ring (1.07 –
1.09); the other scenes have no matte sphere, frame 1.09 (N7 / P1).
Rows: what is left over the curve and the frame.

| sphere, lobe model | 0 – 0.3 | 0.3 – 0.5 | 0.5 – 0.6 | 0.6 – 0.7 | 0.7 – 0.8 | 0.8 – 0.85 | 0.85 – 0.9 | 0.9 – 0.93 |
|---|---|---|---|---|---|---|---|---|
| N1 measured, GGX (`__ggx`) | 1.07 | 1.10 | 1.15 | 1.23 | 1.32 | 1.35 | 1.31 | 1.24 |
| N1 left, GGX | **0.98** | **0.98** | **0.98** | **0.99** | **1.00** | **0.99** | **0.99** | **0.99** |
| K1 left, GGX | 0.97 | 0.98 | 0.98 | 0.99 | 0.99 | 0.99 | 0.99 | 0.99 |
| N1 left, export (Beckmann, r 0.4) | 0.87 | 1.01 | 1.04 | 1.01 | 0.98 | 1.02 | 1.05 | 1.08 |
| P2 left, export (GGX EP, α 0.36) | 1.02 | 1.03 | 1.01 | 1.02 | 1.04 | 1.06 | 1.08 | 1.08 |
| P3 left, export (GGX EP) | 1.00 | 1.01 | 1.01 | 1.00 | 1.00 | 1.02 | 1.04 | 1.05 |
| P2 left, GGX (`brdf6=2`) | 1.10 | 1.13 | 1.12 | 1.18 | 1.19 | 1.19 | 1.18 | 1.15 |
| P3 left, GGX | 1.03 | 1.04 | 1.06 | 1.07 | 1.10 | 1.11 | 1.10 | 1.09 |
| P2 left, Beckmann (`brdf6=1`) | 1.10 | 1.17 | 1.19 | 1.38 | 1.55 | 1.61 | 1.56 | 1.49 |

On the r 0.4 lobes at GGX — Iray's distribution — the curve takes the
whole rim: what is left is flat at 0.97 – 1.00 from the core to 0.93 R
(measured 1.07 → 1.35 → 1.24, the model's shape exactly, peak at 0.8 –
0.85 R and down at the last ring as Fresnel / Schlick turns past 65°).
The uniform-light model is enough there; the HDRI's sun doesn't show.

### The curve's shape moved by Specular Occlusion (Calib-PBRSkin-Occlusion)

The decisive test: the same lobes under five different curves, the
export as is (Beckmann at r 0.4, so O1's leftover row is the
distribution's own shape), new Octane render
(`~/Library/Caches/orbis/calib/octane/enhanced-final`), Iray reference
of 2026-11-13:

| sphere | rim 0.85 – 0.9 measured | model | left over curve and frame, core → rim |
|---|---|---|---|
| O1 dual lobe | 1.40 | 1.21 | 0.87 1.02 1.03 1.01 0.98 1.02 1.05 1.09 |
| O2 occlusion default | 1.10 | 0.99 | 0.90 1.02 1.06 1.05 1.00 1.00 1.02 1.04 |
| O3 grazing 0.1 | 0.99 | 0.93 | 0.88 0.99 1.05 1.05 1.00 0.96 0.98 1.00 |
| O4 facing 0.25 | 1.08 | 0.88 | 0.99 1.01 1.07 1.12 1.18 1.12 1.13 1.16 |
| O5 occlusion w 0.5 | 1.37 | 1.21 | 0.91 0.96 1.03 1.06 1.08 1.05 1.04 1.08 |

The same with every lobe at GGX (`orbx-variants.py brdf=2`,
`~/Library/Caches/orbis/calib/c188`), left over curve and frame, core → rim:

| sphere | measured, core → rim | left |
|---|---|---|
| O1 | 1.08 1.10 1.15 1.23 1.32 1.34 1.31 1.24 | 0.98 0.98 0.98 0.99 1.00 0.99 0.99 0.99 |
| O2 | 1.08 1.07 1.07 1.08 1.10 1.09 1.08 1.05 | 0.99 0.98 0.97 0.98 0.99 1.00 1.00 1.01 |
| O3 | 1.06 1.05 1.04 1.04 1.04 1.02 0.99 0.95 | 0.97 0.96 0.96 0.96 0.97 0.97 0.98 0.98 |
| O4 | 1.21 1.20 1.28 1.38 1.39 1.21 1.05 0.93 | 1.10 1.05 1.06 1.10 1.16 1.13 1.09 1.07 |
| O5 | 1.08 1.09 1.13 1.20 1.30 1.37 1.35 1.30 | 0.99 0.98 0.98 0.99 1.01 1.03 1.02 1.02 |

Four curves — rising (O1, O5), flat (O2), falling (O3) — each measured
in its own shape and left flat at 0.96 – 1.03 once the model divides it
out.

The rim excess goes where the curve sends it: gone under a grazing of 0.1
(O2, O3: 1.10 / 0.99 at the rim against O1's 1.40), back at w 0.5 (O5).
O4 (n 0.01, a quarter of O1's specular) carries the red base's blue
(no matte sphere in this scene to take it off: ~+0.1 at a quarter of the
signal) on top of the same shape.

## 4. Reading

- The rim's 1.30 – 1.45 × is the frame (1.08 – 1.09, the diffuse's offset
  everywhere) times the curve (1.15 – 1.25 at 0.8 – 0.93 R): Universal's
  physical Fresnel against `custom_curve_layer`'s Schlick, at the
  microfacet. Derived from the MDL and the spec's own Fresnel, nothing
  fitted. It is #137's component, now on PBRSkin's spheres; the decision
  there stands (Octane's dielectric is the physical one, no IOR draws
  Schlick's shape) — no change to the export.
- For #186, with the curve and the frame divided out: at r 0.4 plain GGX
  is Iray (0.97 – 1.00 flat), Beckmann at the Phong pin is 0.87 at the
  core and 1.08 at the rim (its shape). At α 0.36 (P2 / P3) GGX EP is
  1.00 – 1.08, plain GGX 1.03 – 1.19, Beckmann 1.10 – 1.61. So GGX EP's
  "lower rim" at α 0.36 is not hiding the curve: after the curve, GGX EP
  is still the nearer at that roughness, and plain GGX is 3 – 10 % above
  Iray's GGX of the same α there from the core out — a level, not the
  rim. That is #186's to explain (the pin, or Octane's GGX vs MDL's at
  α 0.36), not a curve question.
- P4 (W 0.25) sits ~0.08 above P2 on every ring including the core —
  a quarter of P2's blue, where the 8-bit blue and the matte subtraction
  weigh 4×; not the curve (same shape), not read further.

## Process

- On a red base the blue channel is the specular alone only after the
  base's own blue is taken off: Octane's red diffuse has twice Iray's blue
  (0.001 vs 0.0005), which is 4 – 20 % of a lobe's blue and inflated the
  dim spheres' ratios (P4 1.40 → 1.20 at the core).
- Dividing by a model per ring, not per sphere, is what separates a curve
  (a shape over the rings) from a distribution (a level, or a core / rim
  tilt): the flat GGX row was the proof.

Timings: Calib-PBRSkin-Occlusion export + render, and its GGX variant,
~1 – 2 min each at 400 samples; the furnace model a few s per sphere.
