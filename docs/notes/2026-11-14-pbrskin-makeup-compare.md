# 2026-11-14 PBRSkin Makeup: Iray compare (#142)

Resume of #142 after its Iray batch: does the Makeup Mix of 334acaa (a Mix
material at Makeup Weight of the material in the makeup's colour and the
material without it, from `pbr_skin.mdl`, no fit) render as Iray's
`weighted_layer`?

## Which references are real

- `Calib-PBRSkin-Makeup` (91b1388): rendered 00:00 in the queue's session
  of 23:30, before its `pbr_skin` clash - no MDL import error in DAZ's log
  for it.
- `Calib-PBRSkin-Makeup-TL`: the 00:00 render hit `MDL Import Error (4004)`
  for `::daz_3d::pbr_skin` (#94) and was a fallback; `keep-refs.py` skipped
  it. The reference in d226a45 is the re-render of 01:01, the first scene
  of a fresh queue session (started 00:58:29); DAZ's log between its open
  and its saved image has no import error. Both are real PBRSkin renders.

## Octane side

Re-rendered with the current export (after #87, #114, #140's detail normal
and the 8-bit colours; enhanced-final, 400 samples, denoised pass), the
`.duf` paths absolute (#159). About a minute a scene; the numbers equal the
2026-11-13 render's to the second decimal.

## Numbers

Octane / Iray at the same sphere position (linear RGB, discs from the
dump, `sphere_discs.py`); bins r / R: core < 0.5, mid 0.5 - 0.8, rim
0.8 - 0.93, luminance:

| sphere | core R / G / B | Y core | Y mid | Y rim |
|---|---|---|---|---|
| K1 tie (= O1) | 1.13 / 1.07 / 1.09 | 1.10 | 1.20 | 1.27 |
| K2 w 1 | 0.96 / 1.05 / 1.08 | 1.06 | 1.11 | 1.11 |
| K3 w 0.5 | 1.11 / 1.03 / 1.09 | 1.07 | 1.16 | 1.16 |
| K4 weight map | 1.14 / 1.01 / 1.09 | 1.07 | 1.21 | 1.20 |
| K5 w 0.5 mult 0.25 | 1.13 / 1.03 / 1.10 | 1.07 | 1.12 | 1.13 |
| K6 w 1 mult 2 | 1.24 / 1.12 / 1.09 | 1.12 | 1.14 | 1.18 |
| K7 metal 0.5 tinted | 1.12 / 1.02 / 1.06 | 1.05 | 1.21 | 1.30 |
| K8 metal 0.5 | 1.13 / 1.07 / 1.11 | 1.10 | 1.26 | 1.44 |
| L1 tie translucent | 1.07 / 1.07 / 1.07 | 1.07 | 1.14 | 1.18 |
| L2 + w 0.5 | 1.04 / 1.05 / 1.08 | 1.06 | 1.13 | 1.15 |
| L3 + map | 1.05 / 1.05 / 1.09 | 1.06 | 1.17 | 1.17 |
| L4 tie top coat | 1.18 / 1.00 / 1.04 | 1.09 | 1.25 | 1.28 |
| L5 + w 0.5 | 1.13 / 1.03 / 1.12 | 1.08 | 1.21 | 1.22 |
| L6 tie skin | 1.20 / 1.29 / 1.29 | 1.27 | 1.28 | 1.27 |
| L7 + w 0.5 | 1.12 / 1.12 / 1.12 | 1.12 | 1.22 | 1.29 |
| L8 + map | 1.18 / 1.18 / 1.15 | 1.17 | 1.22 | 1.35 |

(The red channel of K6 is 0.011 vs 0.009: the blue makeup leaves only the
highlight in red.)

Reading:

- **The Mix holds.** Every makeup sphere's core sits at 1.05 - 1.12, the
  frame the ties and Calib-PBRSkin-Occlusion's row show at these positions
  (O2 - O8 1.05 - 1.14). K2 (the makeup side alone) 1.06; the base side is
  K1 / O1. Over translucency (L2 / L3) at L1's 1.06 - 1.07, under the Top
  Coat (L5) at L4's 1.08 - 1.09. Over the subsurface skin L7 / L8 land
  between the makeup side (~1.06) and the skin tie (1.27, #57's skin
  baseline), as a Mix of the two must; L7's makeup dominates at w 0.5
  because the makeup also covers the translucency.
- **Weight map** (K4, L3, L8; bands 0 / 64 / 128 / 255 of 8 bits, gamma
  auto): a row profile through K4's centre column shows the four bands at
  the same rows in both renderers (no phase or mirror) and each band at
  the frame's ratio (B 1.04 - 1.15 per band; the zero band's blue,
  ~0.01, 0.85 - 0.98). The effective weights read from the profile agree
  (band 64 ~0.12, band 128 ~0.42 in both, against the white and zero
  bands at their latitudes), so the export's gamma 2.2 for an 8-bit
  `L` map is Iray's "auto". Coarse quarter-disc bins had shown 1.2 - 1.3
  in the low bands: those mixed the zero band's tiny blue with the rim.
- **Roughness mult** (`highlight-spread.py`, p99 / median): K5 / K3
  Octane 9.8 / 6.0 = 1.63, Iray 8.1 / 5.0 = 1.62; K6 / K2 Octane
  3.3 / 4.5 = 0.73, Iray 3.0 / 3.9 = 0.77. The mult sharpens / spreads
  the highlight as in Iray.
- **Metal tint**: K7 / K8 per channel Octane R 0.77, B 1.90; Iray R 0.77,
  B 2.02 - the `blend_colors` lerp is right. Both metal spheres are too
  bright at the rim (1.30 / 1.44 against K1's 1.27) and sharper (spread
  1.23 / 1.32 × Iray against 1.09 - 1.19): the PBRSkin metal's own, not
  the makeup's (the Mix is linear, the split matches) - #167.

Nothing fitted, no rule changed: the spec gains the check.

## Process

- Coarse bins on a weight-map sphere mix bands of very different level;
  a row profile through the centre column (the bands are latitudes) is
  what tells a gamma or phase error from the rim.
- The ties carry the frame at their own position only; with no tie at
  every position, the neighbouring scene's row (Occlusion) gives the
  per-position frame for the same base.

## Where it stands

#142 closed. Open: the PBRSkin metal's rim (#167), the skin baseline
(#57), the Top Coat normal (#166).
