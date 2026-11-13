# 2026-11-14 PBRSkin Specular Occlusion / Detail roughness: Iray compare (#54)

Resume of #54 after its Iray batch (reference `calib/references/iray/
Calib-PBRSkin-Occlusion.png`, 5000 samples, denoiser on@8, rendered before
the queue's `pbr_skin` module clash of the same night, checked by eye: real
PBRSkin, no fallback). Checks the rule of 8d91967 (from `pbr_skin.mdl`, no
fit): occlusion = dual-lobe weight × g, F0 × n / g; Detail roughness × lerp(1,
mult, weight).

## Octane side

Re-rendered with the current export (enhanced-final, 400 samples, denoised
pass), because the 2026-11-13 render predated 9987673 (detail normal OSL),
the Makeup Mix and the 8-bit colour rounding. First run exported nothing:
`calib-variants.sh` with a relative `.duf` path finds no cached dump (the
dump is keyed by the absolute path) - #159. With the absolute path: 1 render,
under a minute.

## Numbers

Linear luminance (gamma 2.2), discs projected from the dump
(`sphere_discs.py`). Mean over 0.9 R (upper part, `highlight-spread.py --vs`)
and highlight spread p99 / median:

| sphere | mean Octane / Iray | ÷ O1's | spread Iray | spread Octane | Octane / Iray ÷ O1's |
|---|---|---|---|---|---|
| O1 dual lobe | 1.18 | 1.00 | 6.2 | 7.1 | 1.00 |
| O2 occlusion default | 1.09 | 0.92 | 5.3 | 6.2 | 1.02 |
| O3 grazing only | 1.08 | 0.92 | 7.4 | 8.5 | 1.00 |
| O4 facing only | 1.18 | 1.00 | 4.0 | 5.1 | 1.11 |
| O5 weight 0.5 | 1.20 | 1.02 | 7.5 | 9.7 | 1.12 |
| O6 grey map | 0.84 | 0.71 | 9.2 | 8.8 | 0.83 |
| O7 detail rough 0.5 | 1.15 | 0.97 | 14.7 | 15.2 | 0.90 |
| O8 detail normal w0.5 | 1.24 | 1.05 | 9.4 | 13.4 | 1.24 |

Radial bins (r / R: centre < 0.5, mid 0.5 - 0.8, rim 0.8 - 0.93), each
sphere / O1 in its own renderer:

| sphere | Iray centre / mid / rim | Octane centre / mid / rim |
|---|---|---|
| O2 | 0.74 / 0.81 / 0.61 | 0.71 / 0.75 / 0.52 |
| O3 | 0.82 / 1.08 / 0.75 | 0.78 / 1.00 / 0.60 |
| O4 | 0.44 / 0.61 / 0.78 | 0.46 / 0.62 / 0.67 |
| O5 | 0.58 / 0.95 / 0.88 | 0.56 / 0.97 / 0.85 |

The sphere / O1 ratios also carry each position's environment (the bright
sky sits left), so the centre bin drops along the row in both renderers
alike; only the Octane-vs-Iray difference of a ratio is meaningful.

Reading:

- **Occlusion holds within ~10 %** on O2 - O5 (mean 0.92 - 1.02 of O1's
  Octane / Iray). The rims look 15 - 20 % too dim relative to O1, but in
  absolute terms the occluded rims match Iray better than O1's: rim
  Octane / Iray O1 1.27, O2 1.07, O3 1.02, O4 1.09. O1's unoccluded dual
  lobe is too bright at grazing in Octane (the PBRSkin baseline, #57's
  ground), and occlusion removes most of exactly that excess. No sign of
  the Schlick-vs-real-Fresnel departure the resume feared for O3 / O4
  (F0 0.4 at weight 0.1 for O3).
- **O6** is off by the documented limit: the export takes the occlusion
  weight's channel value (1), Iray the grey map per texel; Octane renders
  O6 as O2, Iray as a much weaker occlusion (rim near O1's, which points at
  `gamma auto` decoding 128 as 0.22). Filed #158.
- **O7** (Detail roughness × 0.5): highlight p99.5 / O1 1.99 Iray, 2.09
  Octane; spread ratio 0.90 of O1's. Holds.
- **O8**: its roughness part is O7's rule; the detail normal (now exported,
  #140) leaves Octane's highlight sharper than Iray's - p99.5 / O1 1.47 vs
  1.11, spread 13.4 vs 9.4. That is #140's to judge.

## Where it stands

#54 closed: occlusion and Detail roughness verified; OmniHair shift checked
on 2026-11-13. Open: occlusion map (#158), detail normal (#140), relative
dump paths (#159).
