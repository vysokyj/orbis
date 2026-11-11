# 2026-11-11 Gloss over dual lobe: per-sphere layers (#106)

Resume of #106 with the decomposition references (commit 0ff276c):
Calib-DualLobeGlossy-dl (gloss off), -gloss (dual lobe off), -bare
(gloss, dual lobe and Top Coat off), 5000 samples, beside the
Calib-DualLobeGlossy reference from the first compare
(`2026-11-11-gloss-over-dual-lobe-compare.md`). The three private
Calib-Skin-Base-hdri-coat0 / -dlgloss / -dlglosstc renders (400 samples)
came in the same server run; they stay in the cache.

## Octane side
Re-rendered on the current export (enhanced-final, 400 samples) - the skin
renders were from 12:19, before 3d57828. The sphere scenes moved by
< 0.001 per sphere, the skin by ≤ 0.005: nothing exported since touches
them.

## Per sphere (mean linear luminance over the upper disc)
Regions as in the first compare. Layer = scene − bare; loss = (dl + gloss
− both) / (dl + gloss), the share of the two layers that stacking takes.

| sphere | both | dl scene | gloss scene | bare | dual lobe alone | gloss alone | Iray loss | Octane loss |
|---|---|---|---|---|---|---|---|---|
| G1 gloss only | 1.12 | 1.06 | 1.12 | 1.06 | - | 1.21 | - | - |
| G2 dual lobe only | 1.12 | 1.12 | 1.08 | 1.08 | 1.16 | - | - | - |
| G3 both | 1.22 | 1.13 | 1.22 | 1.08 | 1.17 | 1.33 | 5.0 % | 2.5 % |
| G4 gloss 0.4 | 1.22 | 1.14 | 1.21 | 1.08 | 1.19 | 1.42 | 2.7 % | 1.0 % |
| G5 sharp gloss, broad lobe | 1.17 | 1.11 | 1.15 | 1.08 | 1.14 | 1.20 | 5.9 % | 2.5 % |
| G6 gloss refl 1 | 1.27 | 1.16 | 1.49 | 1.08 | 1.21 | 1.61 | 5.1 % | 15.7 % |
| G7 Top Coat | 1.39 | 1.22 | 1.45 | 1.08 | 1.24 | 1.51 | 26.4 % | 24.1 % |
| G8 dual lobe W 0.5 | 1.48 | 1.22 | 1.48 | 1.08 | 1.37 | 1.66 | 3.6 % | 1.4 % |

(Octane / Iray in the first six columns.) Centre, rim and the brightest
3 % tell the same; the rim's gloss alone goes to 1.82 (G6) / 1.84 (G8).

What it says:
- **The stacking holds.** Iray's custom-curve gloss takes 3 - 6 % of the
  layers under it, Octane's coating 1 - 2.5 %: at most 1.5 % of a sphere,
  not the 10 % the first compare guessed from G1 / G2. With the Top Coat
  (G7) both lose a quarter. No base darkening from `custom_curve_layer`
  needed.
- **The error is each layer alone where the sphere stands.** The diffuse
  is 1.08 everywhere; the same glossy layer (G1, G3, G8: weight 1, r 0.5,
  reflectivity 0.5) is 1.21 / 1.33 / 1.66 at spheres 1 / 3 / 8, the dual
  lobe alone 1.14 - 1.37. That is the light at that position or the
  lobes' response to it, not the mapping of #106 -> #137.
- G6 is the one stack that differs: at Glossy Reflectivity 1 Octane's
  coating takes 15.7 % of the dual lobe (Iray 5.1 %), and its gloss alone
  rises x 2.69 over G3 against Iray's x 2.21 (cf. #100). In #137.

## Skin (private Genesis 8 base skin, HDRI)
Mean luminance per region (skin-compare.py's boxes, plus the face box
380 - 580 × 110 - 330 and its brightest 3 %), ratios within one renderer:

| region | Iray dlgloss / coat0 | Octane | Iray dlglosstc / coat0 | Octane |
|---|---|---|---|---|
| forehead | 1.15 | 1.18 | 1.16 | 1.02 |
| cheek L | 1.29 | 1.32 | 1.30 | 1.02 |
| nose | 1.29 | 1.32 | 1.31 | 1.01 |
| ear R | 1.38 | 1.42 | 1.39 | 1.02 |
| neck | 1.14 | 1.17 | 1.15 | 1.01 |
| chest | 1.17 | 1.18 | 1.18 | 1.01 |
| face box | 1.34 | 1.37 | 1.35 | 1.02 |
| face top 3 % | 1.13 | 1.12 | 1.13 | 1.01 |

The OpenPBR coat over the dual lobe rises as Iray does (within 3 %):
the mapping holds. With the skin's Top Coat Iray keeps the dual lobe
(the coat adds ~1 %), while the export's fallback (one coat: the gloss
alone) drops it: the face 0.79 × Iray, cheeks / nose 0.82. Large ->
#138 (three layers on OpenPBR). Nothing fitted.

## Process
- The decomposition scenes settled in one compare what the first round
  could only guess: the G1 / G2 baseline would have led to a fitted base
  darkening for a stack that is right.
- The comparison script lived in the scratchpad (per-sphere discs from
  Calib-Glossy's red spheres, as `highlight-spread.py`).

## Outcome
#106 closed: the gloss on the coating over the dual lobe holds on spheres
and skin. Open: #137 (each layer alone off by position), #138 (OpenPBR
skin with a Top Coat).
