# 2026-11-14 Backscattering under Base Mixing 1 and 2, compare (#155)

Resume of #155 with Calib-WeightedBack's Iray reference in (91b1388, 5000
samples, DAZ Studio Iray, Post Denoiser on@8): does the backscattering
lobe DAZ passes in glossiness (1) and weighted (2) mixing, mapped from
the MDL in 359451d (no constant), hold? Calib-MetalTL-Back's K7 was
compared in #145 (1.10 x Iray, see that note).

Octane: enhanced-final, 400 samples, denoised pass, re-exported from the
cached dump with HEAD (a251755, after #87, #114, #149). Numbers: linear
RGB means over 0 - 0.95 R of each projected disc (`sphere_discs.py`),
luminance Y from them; centre r < 0.4, rim 0.75 - 0.95. Floor 1.04.

## Octane / Iray

| sphere | Iray disc RGB | Y | R G B | centre Y | rim Y |
|---|---|---|---|---|---|
| B1 b0 g0.5 (control) | 0.154 0.125 0.164 | 1.04 | 1.05 1.03 1.03 | 1.04 | 1.04 |
| B2 b0.5 g0.5 | 0.106 0.083 0.091 | **1.05** | 1.06 1.04 1.04 | 1.05 | 1.06 |
| B3 b1 g0.5 (lobe alone in the gloss part) | 0.049 0.039 0.005 | 1.14 | 1.15 1.13 1.11 | 1.13 | 1.13 |
| B4 b0.5 g0.5 d0.5 (weights sum to 1) | 0.119 0.135 0.133 | **1.06** | 1.08 1.06 1.03 | 1.09 | 1.07 |
| B5 b0.5 g0 (no gloss part, no lobe) | 0.061 0.001 0.000 | 1.09 | 1.09 - - | 1.09 | 1.09 |
| B6 b0.5 rough 0.7 | 0.098 0.087 0.093 | **1.08** | 1.05 1.09 1.05 | 1.12 | 1.07 |
| B7 Base Mixing 1 b0.5 | 0.038 0.076 0.016 | 1.10 | 1.32 1.07 1.07 | 1.04 | 1.15 |
| B8 Base Mixing 0 b0.5 | 0.038 0.083 0.016 | 1.10 | 1.36 1.07 1.06 | 1.04 | 1.15 |

(B5's G / B are ~0.001: no ratio.)

- **The weighted construction holds.** The lobe as a layer of the gloss
  part (`weighted_layer(b, backscattering, glossy)`, Mix at b of the
  retro-reflector and the metal at the gloss part's scale): B2 / B4 / B6
  1.05 / 1.06 / 1.08, per channel within 1.03 - 1.09, as the control B1
  (1.04). B4 (Diffuse 0.5, weights summing to 1) holds as B2 (Diffuse 1,
  normalised to 2/3 : 1/3), so the normalisation is right with the lobe.
- **B3** (b 1: DAZ switches the glossy off, the lobe alone) 1.14: the
  lobe's own level under #73's rule (Calib-Backscatter w0.5 1.15, w1
  1.18), not this issue's.
- **B5** (Glossy Weight 0): no green anywhere in either renderer, as the
  `.mi` said (no `backscattering_enable`); 1.09 = Calib-Backscatter's
  B1 / Calib-Glossy's matte level.
- **B7 = B8** channel by channel (1.10 / 1.10, R 1.32 / 1.36): Base
  Mixing 1 is the mode-0 construction, the roughness from 1 -
  Backscattering Glossiness (0.7 -> 0.09) right. Their luminance 1.10
  ties the scene to Calib-Backscatter's mode-0 rule and to K7 (1.10).
- **Red on B7 / B8** (1.32 - 1.40 centre, R 0.038 -> 0.051 absolute):
  not the mixing (B7 = B8, B1 / B2 / B4 per channel fine). Split with
  disc means: B3's red minus 2/3 of B5's (the diffuse share at
  Diffuse 1 : Glossy 0.5) leaves the lobe's red per unit weight ~0.024 in
  Iray, ~0.036 in Octane (1.5 x; green 1.13 x), hidden in B3 by the red
  diffuse, visible where a gloss layer halves the diffuse (B7 / B8).
  Calib-RefrBack R2 (r 0, b 0.5, mode 0: B8's channels) shows the same,
  R 1.19. Calib-Backscatter used a shared white-ish lobe colour, so #73
  never tested a saturated tint. Filed #169; nothing fitted here.

## Process

- The first pass of the red ran through Calib-Glossy G2 as the "surface
  without lobe": its roughness and layered weight differ, and the split
  came out inconsistent (blue below the white gloss). The scene's own
  B3 / B5 (same channels, one part switched off each) gave a consistent
  split - the variants that switch one part off were already in the batch.
- Weighted mixing normalises Diffuse 1 / Glossy 0.5 to 2/3 : 1/3; forget
  that and the lobe's share looks 1.5 x off.

## Where it stands

#155 closed: Base Mixing 1 (K7 1.10, B7 1.10) and Base Mixing 2 (B2 /
B4 / B6 1.05 - 1.08, B5 no lobe) against Iray, no constant. The lobe's
hue under a saturated unshared colour: #169. Partial refraction with the
lobe: #157 (its Calib-RefrBack R2 is B8's twin).
