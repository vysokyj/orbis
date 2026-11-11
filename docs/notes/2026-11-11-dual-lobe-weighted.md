# 2026-11-11 — Dual lobe under weighted mixing (#121)

Goal: keep Iray Uber's dual lobe specular on a weighted material (Base
Mixing 2). The export's Mix material (`weighted_mix_material`) wrote its
diffuse part (`material2`) with `specular` 0, so the dual lobe vanished —
found while working #106, by code review.

## What Iray does (docs first)
DAZ's `irayubermaterial.mdl`, `normalized_base`: `dl_diffuse_bsdf` is the
diffuse with the dual lobe over it (`add_dual_lobe_specular`, the same
`custom_curve_layer` as in `metallic_layered_base`: the diffuse keeps
1 − r·W head-on, 1 − W at grazing). `normalized_mix` takes that at Diffuse
Weight against the gloss part (plain glossy, backscattering, refraction)
at Glossy Weight. So the dual lobe belongs to the diffuse part, not to the
gloss. Calib-Mi-Mix2's M3 already showed DAZ hands the lobes over under
Base Mixing 2 (lobe roughnesses squared, ratio as is).

## Mapping (no fitted constant)
`material2` takes the dual lobe exactly as a dual lobe without a glossy
layer (calibrated on Calib-DualLobe / -DualLobe2, #26, #95): lobe 1 on
`specular` at its share, lobe 2 the `coating` at its Phong pin, IOR above
r 0.5; under a Top Coat (written on both Mix parts) one lobe at the full
share and the ratio's blended roughness. The roughness is lobe 1's
(`roughness_source`), isotropic (Glossy Anisotropy is the gloss's);
`material1`, the gloss metal, keeps the glossy layer's own roughness
(#106). With a lobe the base is no longer Oren-Nayar on the roughness pin
(the Lambertian × F(σ) instead, as everywhere a gloss shares the pin).

## New scene: Calib-DualLobeWeighted
Calib-DualLobe's red, Base Mixing 2, Diffuse 1 / Glossy 0.5, gloss
roughness 0.6, dual lobe W 1, ratio 0.5, lobes 0.3 / 0.1:
W1 gloss only, W2 dual lobe only (Glossy Weight 0: no gloss part), W3
both, W4 diffuse 0.2 / gloss 0.8, W5 diffuse 0.5 / gloss 0.3 (sum < 1),
W6 + Top Coat (r 0.1), W7 lobe reflectivity 1, W8 dual lobe W 0.5.

`.mi` export → fixture `calib/references/mi/Calib-DualLobeWeighted.json`.
It found one `src/iray.rs` difference: on W2 (weighted, Glossy Weight 0)
DAZ passes no `glossy_roughness` (the MDL's 0) though it still sets
`glossy_roughness_enable`; we had 0.36. Fixed from the fixture, as the
backscattering weight and gloss colour already were for that case.

Octane side rendered (enhanced-final, 400 samples): all eight spheres
render, W2 the red dual lobe alone, the rest the weighted mix with the
lobes' highlight over the gloss metal.

## Side findings (issues)
- #126: `normalized_mix` leaves weights summing below 1 unscaled (the rest
  black); the export normalises by the sum, so W5 should come out 1.25 ×.
- #127: Glossy Layered Weight still counts as gloss under Base Mixing 2,
  where Iray ignores it.
- #128: translucent weighted materials skip the Mix and still drop the
  dual lobe.

## Pending
Iray reference of Calib-DualLobeWeighted (5000 samples). Resume: W1 and
W2 against Calib-Mixing's weighted sphere and Calib-DualLobe's ratios
(they test nothing new); W3, W4, W6 – W8 decide the mapping; W5 decides #126.
