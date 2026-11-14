# 2026-11-14 Closed solid compare (#134)

Resume of #134 with the `-Solid` twins' Iray references in (91b1388, a3f210a):
do the generated spheres' translucency and refraction rules, measured on
thin-walled shells, hold on closed solids?

Scenes: Calib-SSS / -SSS-Solid, Calib-WeightedTL / -Solid,
Calib-DualLobeWeightedTL / -Solid (Iray 5000), Calib-MetalRefr (400) /
-Solid (1000: its 400-sample render is under the queue's 15 s rejection
limit, #160), Calib-MetalRefr-TL / -TL-Solid (400). Octane: enhanced-final,
400, all ten re-exported from the cached dumps with the current export
(after #87's prop-volume bake and #114), denoised pass. Numbers: linear
luminance (Y) over 0.6 R of each projected disc (`sphere_discs.py`).
Octane batch ~10 min.

## Iray first: thin vs solid, no Octane

Iray's solid / thin, per sphere:

| scene | spheres | solid / thin |
|---|---|---|
| Calib-WeightedTL-Solid | W1 - W8 | 1.00 (max pixel difference 1 / 255) |
| Calib-DualLobeWeightedTL-Solid | T1 - T8 | 1.00 (1 / 255) |
| Calib-SSS-Solid | S1 - S4, S8 | 1.00 |
| | S5 red transmit (Transmitted Color) | 0.80 (R G B 0.70 0.86 0.91) |
| | S6 SSS amount | 0.95 |
| | S7 chromatic (Transmitted Color + SSS) | 0.76 (0.65 0.84 0.90) |
| Calib-MetalRefr-Solid | M1 / M2 (metal only) | 1.00 |
| | R1 / R2, X1 - X4 (refraction) | 0.96 - 1.12, X4 1.68 |
| Calib-MetalRefr-TL-Solid | R / X | 0.82 - 1.32 |

So without a volume a closed solid is its shell, exactly as DAZ's MDL
says (`thin_walled` enters only `add_uber_volume` and the two-sided
emission): diffuse transmission doesn't care about the wall. The wall
matters where the closed surface lets a volume in (S5 - S7) or a
refraction bends (the R / X spheres).

## Octane / Iray

| sphere | thin | solid |
|---|---|---|
| Calib-SSS S1 opaque | 1.04 | 1.04 |
| S2 / S3 / S4 / S8 | 0.97 / 1.00 / 0.96 / 0.92 | 0.97 / 1.00 / 0.96 / 0.93 |
| S5 / S6 / S7 (volume) | 0.96 / 0.95 / 0.94 | 1.05 / 1.08 / 1.05 |
| Calib-WeightedTL W1 - W8 | 0.91 - 1.06 | 0.91 - 1.06 (same) |
| Calib-DualLobeWeightedTL T1 - T8 | 0.98 - 1.09 | 0.95 - 1.08 |
| Calib-MetalRefr R1 / R2 | 1.01 / 1.09 | 1.05 / 1.03 |
| M1 / M2 | 1.10 / 1.12 | 1.11 / 1.13 |
| X1 / X2 / X3 / X4 | 1.20 / 1.22 / 1.17 / 0.82 | 1.05 / 1.03 / 1.05 / 1.03 |
| Calib-MetalRefr-TL R1 / R2 | 1.04 / 1.11 | 1.05 / 1.04 |
| M1 / M2 | 1.09 / 1.06 | 1.08 / 1.05 |
| X1 / X2 / X3 / X4 | 1.17 / 1.22 / 1.12 / 0.92 | 1.08 / 1.05 / 1.17 / 1.12 |

- **Translucency share on a closed solid: w.** Octane renders the thin and
  solid twins alike where Iray does (no volume); the open-sheet min(2w, 1)
  stays for open sheets only. `transmissionType` 1 (Diffuse) on the solids
  = 3 on the shells.
- **Volume spheres:** #87's baked albedo (closed, not thin-walled, with an
  Iray volume) follows Iray's darkening: 1.05 - 1.08 on the solids, while
  the shells (no volume in Iray, plain transmission) stay 0.94 - 0.96.
- **Red deficit** on the plain translucent spheres (S8 R 0.84, W3 / W4 R
  0.86 - 0.88, W5 B 0.73) is the same thin or solid: not the wall. Filed
  #163.
- **Refraction Mix (#129) on solids** fits (X 1.03 - 1.05, beside metals
  1.11 / 1.13 and refractions 1.03 / 1.05); on the shells it overshoots
  (1.17 - 1.22) more than either part alone. Filed #164. X4's thin 0.82 is
  #136's blur; on the solid 1.03.
- **#135** (partial refraction under translucency): -TL-Solid R1 / R2 1.05
  / 1.04 beside -Solid's 1.05 / 1.03, so the split holds on solids; the
  spec's "waits for -TL-Solid" is gone. -TL-Solid's X3 / X4 (1.17 / 1.12)
  are the metal side, not the split.

No export change; spec and the comments in `material.rs` say "measured"
where they said "unmeasured until Calib-SSS-Solid".

## Process

- Comparing Iray thin vs solid first (two references, no render) answered
  most of the question before Octane ran: four of five scenes split into
  "identical" and "volume / refraction" at once.
