# 2026-11-16 Backscattering lobe under a saturated tint (#169)

Question: Calib-WeightedBack B7 / B8 (Base Mixing 1 / 0, Backscattering
Weight 0.5, unshared Backscattering Color 0.3 / 0.9 / 0.4 over a red
diffuse) render R 1.32 / 1.36 x Iray while G / B and Y hold (1.07, 1.10).
Is it the lobe's mapping (the metal's albedo / edgeTint, a non-linear
tint in Iray) or Octane's colour handling? Decompose before fitting; no
Iray render needed in the end.

Octane: enhanced-final export of the cached dump at HEAD (6621d9b), 400
samples, denoised pass, ~25 - 40 s a render. Numbers: linear RGB means
over 0 - 0.95 R of each projected disc (`sphere_discs.py`). Variants were
scratch edits of the export's `project.ocs` (`orbx_ocs get` / `set`):
every sphere's material replaced by one node of the export, so each
variant shows that node alone at all eight places of the row.

## Iray side: the tint is linear

- MDL spec: `backscattering_glossy_reflection_bsdf`'s `tint` is a scaling
  factor multiplied by the distribution; no Fresnel. DAZ's Uber passes
  `glossy_color` or `backscattering_color` straight in
  (`irayubermaterial.mdl`).
- `.mi` (calib/references/mi/Calib-WeightedBack.json, B3):
  `backscattering_color` 0.0707 / 0.793 / 0.133 = the sRGB channel
  decoded, the export's albedo (0.0705 / 0.790 / 0.133) to 3 digits.
- Iray renders RGB (spectral rendering off), so its lobe is tint x the
  white lobe per channel.

## Octane side: the lobe alone

B3's backscatter node (metal on the retro normal, Beckmann, roughness
0.212) alone on every sphere, tint per variant; ratio of each variant to
the white tint x the tint, mean over the eight spheres (spread):

| tint | R | G | B |
|---|---|---|---|
| grey 0.5 | 1.000 | 1.000 | 0.999 |
| grey 0.0705 (R's) | 1.001 | 0.998 | 0.997 |
| grey 0.790 (G's) | 1.001 | 1.000 | 1.000 |
| grey 0.133 (B's) | 1.000 | 0.999 | 0.998 |
| green 0.0705 / 0.790 / 0.133 | **1.64** (0.18) | 0.98 | **0.90** (0.04) |

Octane is exactly linear in a grey tint, so neither the metal's Fresnel
(edgeTint = albedo) nor the level is in question. The saturated colour
alone moves light between channels. Green against its grey twins
(R from the R grey, ...), per sphere:

| sphere | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| metal lobe R | 1.31 | 1.47 | 1.56 | 1.63 | 1.67 | 1.74 | 1.84 | 1.89 |
| metal lobe B | 0.97 | 0.95 | 0.93 | 0.91 | 0.90 | 0.88 | 0.86 | 0.83 |
| dielectric lobe R (metallic 0, specular = tint, IOR 1000) | 1.31 | 1.46 | 1.56 | 1.63 | 1.67 | 1.73 | 1.83 | 1.89 |
| plain diffuse R (albedo = tint) | 0.10 | 0.09 | 0.08 | 0.08 | 0.08 | 0.07 | 0.07 | 0.11 |
| plain diffuse B | 1.05 | 1.05 | 1.05 | 1.06 | 1.06 | 1.06 | 1.06 | 1.06 |

(G within 0.98 - 1.00 everywhere.)

- **Not the metal**: a dielectric specular in the same tint gives the
  same numbers to 0.01, so the artistic metal's albedo / edgeTint
  conversion adds nothing.
- **Spectral**: the same colour as a diffuse *loses* its red (0.1 x),
  and the lobe's excess grows along the row (1.31 -> 1.89) with the
  light it returns: the retro-reflector sees the HDRI behind the camera,
  warm (white lobe R / B 1.22 -> 2.14 along the row), the diffuse the
  whole sky, blue (B > R). Octane upsamples the saturated green to a
  spectrum whose tail reaches into red; times a warm light spectrum that
  gives more red than the RGB product, times a blue one less. Iray (RGB)
  takes the RGB product. Same mechanism as #87's media and #163's W5
  transmission.

## Back to the scene

The B7 / B8 / B3 renders with the lobe's spectral part swapped for its
grey twins (Octane - share x (green - grey twin) per channel at the
sphere's own place), against Iray:

| sphere | as rendered R G B | grey-twin lobe R G B |
|---|---|---|
| B8 (mode 0, b 0.5) | 1.36 1.07 1.06 | 1.14 1.09 0.91 |
| B7 (mode 1, b 0.5) | 1.32 1.07 1.07 | 1.13 1.09 0.91 |
| B3 (lobe alone in the gloss part, share 1/3) | 1.15 1.13 1.09 | 1.10 1.15 0.67 |

B8 also splits exactly: 0.5 x the lobe alone + 0.5 x B8's surface alone
(both rendered at place 8) = 0.051 / 0.088 / 0.017, the render 0.051 /
0.088 / 0.017. With the spectral part out, the red sits at the lobe's own
level (#73: Calib-Backscatter 1.08 - 1.18, uniform); B3's blue 0.67 is on
an absolute 0.005 (lobe B 0.003).

## Decision

No change. The lobe's mapping (tint x `retro_albedo`, metal on the retro
normal) is linear and right; the red is Octane's spectral rendering of a
saturated colour, which depends on the light's spectrum, so no per-colour
constant can undo it (fitting one would match this HDRI only). As with
#87 / #163, kept and written into the spec. Luminance holds (Y 1.10).

## Process

- Grey twins per channel (#163's tip) split it in one batch: six renders
  of the lobe alone (green, three greys, white, 0.5), then the diffuse and
  dielectric twins to tell spectral from the metal's conversion. Twelve
  renders, ~6 min of Octane, no DAZ, no Iray.
- One node alone on every sphere gives the node's response at all eight
  places of the row at once; the place matters here (the HDRI behind the
  camera differs along the row), so twins must be compared sphere by
  sphere, not against another sphere's split (the #155 split used B5's
  diffuse from another place).
