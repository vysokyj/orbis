# 2026-11-11 Gloss over dual lobe: first compare (#106)

Resume of #106 with the Calib-DualLobeGlossy Iray reference (5000
samples, commit 4e8780f). The private Calib-Skin-Base-hdri-coat0 /
-dlgloss / -dlglosstc scenes are in My Library (made 12:07 by
`make-skin-variants.py`) but were never rendered in Iray: their batch
rows didn't reach the queue, so the skin half of the compare waits.

## Octane side re-rendered
The 12:17 render predated the export changes since (3d57828 and the
weighted / metal fixes); the scene's dump was missing, so `daz-dump.sh`
again, then enhanced-final at 400 samples. Per sphere it moved by ≤ 0.01
against the old render. Calib-Glossy / Calib-DualLobe re-rendered too
(their renders were from Oct 30 / Nov 5): Calib-DualLobe now 1.05 – 1.18
× Iray (was 1.14 – 1.51), Calib-Glossy 0.71 – 1.27.

## Compare (mean linear luminance over the upper disc, Octane / Iray)
Regions: the sphere disc (r < 0.9, above 0.3 r below the centre), its
centre (r < 0.45), its rim (0.65 – 0.9) and the brightest 3 % of Iray's
pixels ("hi").

| sphere | mean | centre | rim | hi | Octane spread / Iray |
|---|---|---|---|---|---|
| G1 gloss only | 1.12 | 1.19 | 1.07 | 1.03 | 2.3 / 2.7 |
| G2 dual lobe only | 1.12 | 1.12 | 1.10 | 0.97 | 2.2 / 2.7 |
| G3 both | 1.22 | 1.23 | 1.19 | 1.20 | 1.8 / 1.9 |
| G4 gloss 0.4 | 1.21 | 1.22 | 1.19 | 1.15 | 1.6 / 1.6 |
| G5 sharp gloss, broad lobe | 1.17 | 1.15 | 1.20 | 1.12 | 1.7 / 1.8 |
| G6 gloss refl 1 | 1.27 | 1.23 | 1.29 | 1.37 | 2.8 / 2.4 |
| G7 Top Coat | 1.39 | 1.33 | 1.40 | 1.25 | 3.7 / 4.0 |
| G8 dual lobe W 0.5 | 1.48 | 1.38 | 1.50 | 1.70 | 4.1 / 3.0 |

By the Resume rule G3 / G4 / G6 are bright (8 – 13 % over the single
layers' 1.12) and G7 / G8 far off; highlight shapes hold except G8
(peakier) and G6.

## Decomposition (Octane)
Orbx variants with pins zeroed (`orbx-variants.py set=…`: specular 0,
coating 0, both) and then the same as scenes (below), per sphere where it
stands:

| sphere | diffuse | + dual lobe | + gloss | sum | both | loss |
|---|---|---|---|---|---|---|
| G3 | 0.0295 | +0.0359 | +0.0449 | 0.1103 | 0.1084 | 2 % of the layers |
| G8 | 0.0247 | +0.0301 | +0.0874 | 0.1422 | 0.1403 | 2 % |

Octane's coating hardly takes anything from the specular under it (2 %
of the layers' sum; centre 2 %, rim 7 %), as its own Fresnel at F0 0.04
would say. Iray's G3 is 0.0886: even if each layer were Octane's / 1.12
(G1, G2), the additive sum would be 0.097, so Iray loses ~10 % more in the
stack. G8: Octane's gloss alone at that sphere (0.112) is already above
Iray's whole G8 (0.095) — position 8 has a strong highlight (Calib-Glossy
and -DualLobe are 1.23 / 1.18 there too).

The lesson: **G1 / G2 are no baseline for G3 - G8**. Each sphere sits
under its own light (diffuse alone 0.025 – 0.030 on spheres 2 – 8, 0.048
on sphere 1; Octane's gloss alone 0.074 at sphere 3, 0.112 at sphere 8),
so the single-layer ratios at spheres 1 and 2 say nothing of the layers
at 3 – 8. No coefficient fitted: the Iray side needs the same split.

## Scenes
Calib-DualLobeGlossy-dl (gloss off on every sphere, G7 keeps its Top
Coat), -gloss (dual lobe off), -bare (everything off), from
`make-calib-scenes.py`; dumped, Octane side rendered (they reproduce the
orbx variants within 0.001 – 0.005).

## Open
Iray batch in #106: the three decomposition scenes and the three private
skin scenes. Then per sphere: Iray's both vs dl + gloss − bare says how
much Iray's custom-curve gloss takes from the dual lobe (Octane: 2 %); if
that loss explains G3 / G4 / G6, the coating needs the base darkened (an
MDL-derived factor, not a fit); G7 / G8 checked the same way.
