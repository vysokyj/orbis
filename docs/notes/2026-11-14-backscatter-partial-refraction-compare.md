# 2026-11-14 Backscattering under a partial refraction, compare (#157)

Resume of #157 with Calib-RefrBack's Iray reference in (91b1388, 5000
samples, DAZ Studio Iray, Post Denoiser on@8): does the lobe under a
partial refraction (565a007: the refraction's Mix outside, the
backscattering Mix as its base part, no constant) hold?

Octane: enhanced-final, 400 samples, denoised pass, re-exported from the
cached dump with HEAD (3fbeec6, after #87, #114, #149; the 2026-11-13
render predated them). Numbers: linear RGB means over 0 - 0.95 R of each
projected disc (`sphere_discs.py`), luminance Y from them; centre
r < 0.4, rim 0.75 - 0.95. Floor 1.04.

## Octane / Iray

| sphere | Iray disc RGB | Y | R G B | centre Y | rim Y |
|---|---|---|---|---|---|
| R1 r0.5 b0 (no lobe) | 0.106 0.082 0.104 | 1.03 | 0.97 1.05 1.05 | 1.01 | 1.04 |
| R2 r0 b0.5 (no refraction) | 0.046 0.051 0.018 | 1.11 | 1.16 1.10 1.10 | 1.09 | 1.13 |
| R3 r0.5 b0.5 | 0.092 0.124 0.118 | **1.06** | 1.08 1.06 1.04 | 1.06 | 1.07 |
| R4 r0.5 b1 (diffuse off) | 0.077 0.188 0.120 | **1.08** | 1.04 1.09 1.03 | 1.09 | 1.09 |
| R5 mode 1 r0.5 b0.5 | 0.089 0.136 0.118 | **1.06** | 1.09 1.06 1.03 | 1.07 | 1.06 |
| R6 r0.5 b0.5 thin | 0.079 0.117 0.095 | **1.06** | 1.10 1.06 1.04 | 1.04 | 1.07 |
| R7 mode 1 r0.5 b0.5 thin | 0.079 0.114 0.095 | **1.05** | 1.10 1.04 1.04 | 1.02 | 1.07 |
| R8 r0.5 b0.5 metal 0.6 | 0.076 0.085 0.095 | **1.05** | 1.08 1.04 1.05 | 1.02 | 1.08 |

## Decomposition (sphere minus R1, the same glass without the lobe)

| difference | Iray RGB | Octane RGB | G ratio |
|---|---|---|---|
| R3 - R1 (lobe at b 0.5 under r 0.5) | -0.014 0.042 0.014 | -0.003 0.045 0.013 | 1.07 |
| R4 - R1 (lobe alone under r 0.5) | -0.028 0.105 0.016 | -0.022 0.118 0.014 | 1.12 |
| R5 - R1 (mode 1) | -0.017 0.054 0.014 | -0.005 0.058 0.012 | 1.07 |
| R6 - R1 (thin) | -0.026 0.035 -0.008 | -0.015 0.037 -0.010 | 1.07 |
| R7 - R1 (mode 1 thin) | -0.027 0.031 -0.008 | -0.015 0.032 -0.010 | 1.02 |
| R8 - R1 (metal 0.6) | -0.030 0.003 -0.009 | -0.021 0.002 -0.009 | (0.84, 0.003 absolute) |

- **Ties hold first.** R1 (refraction Mix, no lobe) 1.03 = #135 / #129's
  refraction; R2 (no refraction) 1.11, B8's twin on Calib-WeightedBack
  (1.10), its R 1.16 the lobe's red (#169).
- **Nesting.** The lobe's green under the glass relative to the lobe
  alone (R3 - R1) / (R4 - R1): Iray 0.40, Octane 0.38. Same share in
  both renderers, so the lobe sits where the MDL puts it, under the
  refraction at (1 - r) b; a lobe at b outside the refraction's Mix
  would have shown as a larger R3 than R4 allows. Nothing to refit.
- **R4** 1.12 on the lobe's green: the lobe's own level (#73, B3 1.14),
  not this issue's.
- **Base Mixing 1** (R5 / R7) = mode 0 (R3 / R6), thin-walled = closed
  within 0.01 - 0.05 on the disc.
- **Metal 0.6** (R8) hides the lobe in both: 0.003 green in Iray, 0.002
  in Octane, below the noise; Y 1.05.
- **Red.** Every lobe sphere loses less red than Iray when the lobe takes
  its share (R3 - R1: Iray -0.014, Octane -0.003): the lobe's red under a
  saturated unshared colour, the same excess as #169. Left there.

## Where it stands

#157 closed: backscattering under a partial refraction in Base Mixing 0
and 1, closed / thin / metal, Octane / Iray 1.03 - 1.08 on the disc, no
constant. No further Iray round, so #165 (Base Mixing 1 translucent
partial refraction) needs its own `.mi` and scene round.
