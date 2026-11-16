# 2026-11-16 — Partial refraction over translucency in Base Mixing 1 (#165)

Resume of #165 with its Iray batch in (70453f2: Calib-RefrTL-BM1, -Solid,
400 samples, Iray Server, denoised). The question: Base Mixing 1 kept the
one-node rule for a translucent material under a partial refraction,
unmeasured, while Base Mixing 0 takes #135's Mix (the refraction at
Refraction Weight over the material without it).

Conversion side, no new `.mi`: Calib-Mi-RefrBack's Q3 / Q4 / Q7 show DAZ
handing Iray `refraction_weight` and the translucency in mode 1 as in mode
0; `irayubermaterial.mdl` builds both through `metallic_layered_base`
(`weighted_layer(refraction_weight, refraction_bsdf, ...)` over
`diffuse_base`). Only the gloss inputs differ.

## Spheres (enhanced-final, 400 samples, denoised; linear mean within 0.6 R)

Octane both rules rendered before the batch (165-node = HEAD, 165-mix =
the Mix extended to Base Mixing 1, scratch build). Octane / Iray, R G B:

| Calib-RefrTL-BM1 (thin) | one node | Mix |
|---|---|---|
| T1 r 0.3, tl 0.5 | 0.50 0.17 0.10 | 1.11 1.06 1.04 |
| T2 r 0.7, tl 0.5 | 0.86 1.01 1.19 | 1.09 1.12 1.23 |
| T3 r 0.106, tl 0.5 | 0.80 0.47 0.25 | 1.11 1.15 1.17 |
| T4 r 0.45, tl 0.5 | 0.27 0.06 0.03 | 1.16 1.16 1.23 |
| T5 tl 0.5 (control) | 1.09 1.24 1.19 | 1.10 1.25 1.18 |
| T6 r 0.3 (control) | 1.08 1.13 1.16 | 1.09 1.14 1.17 |
| T7 r 0.7 (control) | 1.11 1.25 1.30 | 1.11 1.25 1.30 |
| T8 r 0.106, tl 0.059 (Morgan's) | 1.04 0.28 0.47 | 1.20 1.08 1.28 |

| -Solid (closed) | one node | Mix |
|---|---|---|
| T1 | 0.51 0.16 0.08 | 1.14 1.10 1.08 |
| T2 | 0.85 0.98 1.05 | 1.08 1.04 1.04 |
| T3 | 0.83 0.51 0.26 | 1.13 1.18 1.15 |
| T4 | 0.28 0.05 0.02 | 1.14 1.08 1.07 |
| T5 / T6 / T7 | 1.09 1.22 1.17 / 1.08 1.11 1.14 / 1.00 1.08 1.09 | same |
| T8 | 1.08 0.28 0.39 | 1.24 1.26 1.30 |

The controls fit (1.00 – 1.30, the Base Mixing 1 gloss and the
translucency alone as they were), so the split decides T1 – T4 / T8 alone:
the Mix lands within the controls' band, the one node loses the white
refraction's light through the red base (green / blue 0.03 – 0.5), as Base
Mixing 0's R1 did before #135. -Solid agrees, so it is not thin-wall blur
(#164). Decided at first sight, as the batch comment expected.

## Morgan Hair's own material (Calib-Strand-UberM*, enhanced-final, default samples, denoised)

Morgan's Strands material (Base Mixing 1, w 0.059, r 0.106, thin) now
exports as the Mix (in Calib-Hair-Teens: `<mat> base` = refraction / base
under the top coat Mix). Both rules rendered on the strand scenes (cells
from the Iray alpha; columns 5 whole / 14 no gloss / 15 no translucency /
16 no refraction; 15 and 16 have one part only and stay one node).
Octane / Iray, G, rows top to bottom:

| scene | 5 node → Mix | 14 node → Mix | 15 | 16 |
|---|---|---|---|---|
| UberM (fibres, sun) | 0.49 → 0.54, 0.51 → 0.59, 0.53 → 0.64, 1.37 → 1.53 | 1.30 → 1.37, 1.35 → 1.50, 1.31 → 1.53, 1.29 → 1.40 | 0.89, 0.85, 0.81, 0.95 | 0.54, 0.53, 0.57, 1.41 |
| UberM-Dome | 1.45 → 1.64, 1.45 → 1.67, 1.37 → 1.57, 1.61 → 1.97 | 1.91 → 2.33, 1.89 → 2.41, 1.50 → 1.86, 2.04 → 2.58 | 1.08, 0.99, 0.90, 1.02 | 1.40, 1.39, 1.35, 1.75 |
| UberM-T2 (ribbons, sun) | 1.08 → 1.19, 0.94 → 1.08, 0.58 → 0.70, 0.83 → 0.94 | 1.13 → 1.19, 0.91 → 1.02, 0.62 → 0.72, 0.85 → 0.93 | 1.28, 2.79, 2.16, 0.74 | 1.13, 1.05, 0.68, 0.87 |
| UberM-T2-Dome | 1.01 → 1.13, 0.93 → 1.06, 0.66 → 0.75, 1.10 → 1.33 | 0.76 → 0.93, 0.59 → 0.75, 0.44 → 0.54, 0.75 → 0.96 | 1.49, 1.03, 0.76, 1.44 | 1.21, 0.94, 0.71, 1.07 |

The Mix adds Morgan's 0.106 refraction transmission: +10 – 30 % on the
columns with both parts, the controls untouched. On ribbons (Morgan's
shape in the real hair, #99) it brings the whole material level with its
no-refraction twin (column 16) on most rows, where Iray has them alike;
on fibres it lifts cells that were already above Iray (dome 1.4 – 2.0).
The strand gap itself (curves against Iray's ribbons / fibres, #79, #99,
#161) is far larger than this step and not the layering; the spheres
decide the rule. Calib-Hair-Teens (private, 1.2 M faces) not rendered: its
hair goes up by about this step.

## Code

`universal`'s dispatch: the partial-refraction-over-translucency split now
for Base Mixing < 2 (a metal still only in mode 0); Weighted (2) stays one
node, unmeasured. The test card checks the Mix in modes 0 and 1 and the
one node in mode 2. Spec: "Translucency" (the Why with these numbers),
"Refraction on a metal", thin-wall IOR (the base part keeps the gloss's
F0; the refraction part takes the Refraction Index, as a whole refraction).

## Process

- Both Octane rules rendered before the Iray batch made the resume one
  compare; the controls in the same scene (T5 – T7) told at once that the
  mode 1 gloss mapping was not in question.
- `sphere_discs.Disc` is an object (attributes, not a tuple).
