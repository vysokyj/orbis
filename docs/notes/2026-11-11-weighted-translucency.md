# 2026-11-11 Weighted mixing's translucency (#131)

Goal: `weighted_mix` brightened a Base Mixing 2 diffuse map by a blend of
its diffuse and translucency shares (fitted ×3 by hand on a private mesh
hair, probe 59). #128 left the translucency term open: Calib-WeightedTL's
W3 - W5 (no gloss) 2.4 - 2.9 × Iray, the gloss spheres' rims 1.3.

Scenes: Calib-WeightedTL and -Thin (Iray 5000 samples, 4e8780f),
Calib-DualLobeWeightedTL (#128's) as a check. Octane enhanced-final, 400
samples, ~35 s each. Per sphere where it stands: mean linear luminance
(gamma 2.2) over the disc r < 0.9 R above 0.3 R below the centre, centre
r < 0.45 R, rim 0.65 - 0.9 R, "hi" each render's top 3 %; G / R and B / R
over the disc. (A slightly different disc from #128's note, so its
absolute ratios differ by up to 0.3 on W3 - W5; the conclusions don't.)

## MDL (docs first)
`irayubermaterial.mdl`, `diffuse_base`: with translucency the diffuse is
`df::weighted_layer(weight: 1 - Translucency Weight, layer:
diffuse_reflection(Diffuse Color), base: diffuse_transmission(Translucency
Color))` - the same in every Base Mixing. `normalized_base` (Base Mixing 2)
takes that whole material (with the dual lobe over it, `dl_diffuse_bsdf`)
into `df::normalized_mix` at Diffuse Weight against the gloss at Glossy
Weight only when `gloss_mix_used`; otherwise `dl_diffuse_bsdf` alone,
Diffuse Weight unread. So the translucency is never a third share of the
weighted mix: it is transmission inside the diffuse part, exactly what
Base Mixing 0's path already exports as Universal's diffuse transmission
(Calib-SSS 0.91 - 0.96).

## Decomposition (before the change)
| sphere | Iray | Octane / Iray | centre | rim | hi | G / R I / O |
|---|---|---|---|---|---|---|
| W1 diffuse map | 0.0654 | 1.06 | 1.06 | 1.06 | 1.06 | 0.100 / 0.090 |
| W2 Dw 0.5 | 0.0633 | 1.06 | 1.06 | 1.07 | 1.07 | 0.100 / 0.090 |
| W3 tl 0.5 | 0.0521 | 2.10 | 2.01 | 2.14 | 2.58 | 0.120 / 0.089 |
| W4 tl 0.5 Dw 0.5 | 0.0504 | 2.47 | 2.29 | 2.57 | 3.10 | 0.121 / 0.089 |
| W5 tl 1 | 0.0737 | 1.59 | 1.28 | 1.80 | 3.03 | 0.191 / 0.089 |
| W6 tl 0.5 Gw 0.5 | 0.2006 | 1.16 | 1.20 | 1.15 | 1.00 | 0.735 / 0.539 |
| W7 tl 0.5 Dw 0.5 Gw 0.3 | 0.1726 | 1.20 | 1.28 | 1.16 | 1.00 | 0.827 / 0.564 |
| W8 hair maps | 0.0768 | 1.34 | 1.40 | 1.29 | 1.13 | 0.561 / 0.573 |

1. W1 is the baseline, 1.06.
2. W2 / W1 = 0.97 in Iray and 0.97 in Octane (which gives both the same
   factor 1): the 3 % is the step between neighbouring spheres, so Iray
   reads no Diffuse Weight without gloss - the MDL's 1.0. W4 / W3 0.97
   in Iray too: unread with translucency as well; the export's 1.14 there
   (Dw read as a share against t) was wrong.
3. W3 / W1 in Iray 0.80, W5 / W1 1.13 at a G / R 0.19 (the
   translucency's orange): Iray's sphere gets darker and takes the
   translucency colour as t grows - the reflection at (1 - t)·D plus light
   transmitted from the dim surroundings. The export's factor brightened
   the red map instead (G / R stayed 0.09): no factor on the map can give
   Iray's colour shift, so it isn't a constant to refit.
4. W6 / W7: highlights at 1.00 since #128, the rims 1.15 - 1.16 are the
   diffuse part's translucency term again - nothing gloss-specific left.

Solid vs -Thin: both scenes are thin-walled. The generator copies the base
sphere material, whose `Thin Walled` is true, and only -Thin sets it; the
`.mi` fixtures say `thin_walled: true` on every sphere of both, and the
two Iray references agree to 4 digits. The closed case was never in this
batch - #134 (it also puts Calib-SSS's "closed spheres" in doubt).

## Change
The translucent weighted material transmits like Base Mixing 0: `transmit`
no longer zeroed under weighted mixing, `weighted_mix` gone (the map
unscaled, the albedo at 1 - t via the existing `diffuse_weight`). Nothing
fitted.

## After
| sphere | Octane / Iray | centre | rim | hi | G / R I / O |
|---|---|---|---|---|---|
| W1 | 1.06 | 1.06 | 1.06 | 1.06 | 0.100 / 0.090 |
| W2 | 1.06 | 1.06 | 1.06 | 1.06 | 0.100 / 0.090 |
| W3 | 0.93 | 0.92 | 0.93 | 0.97 | 0.120 / 0.135 |
| W4 | 0.92 | 0.91 | 0.93 | 0.97 | 0.121 / 0.137 |
| W5 | 1.00 | 1.00 | 1.00 | 1.04 | 0.191 / 0.220 |
| W6 | 1.01 | 1.00 | 1.01 | 1.00 | 0.735 / 0.781 |
| W7 | 1.02 | 1.01 | 1.02 | 1.00 | 0.827 / 0.854 |
| W8 | 1.05 | 1.02 | 1.06 | 1.07 | 0.561 / 0.572 |

-Thin within 0.01 of these (the same scene, above). W8, the hair maps,
1.05 inside the others' 0.92 - 1.06 (1.34 before, 1.02 before #128).
W3 / W4 at 0.92 sit where Calib-SSS's translucent spheres do (0.91 -
0.96) - the transmission rule's own residue, not the weighted mix's.

Calib-DualLobeWeightedTL (#128's, translucency 0.5 / 1 under the Mix),
same discs, before → after: T1 1.03 → 1.05, T2 (dual lobe alone, no
gloss) 0.87 → 1.02, T3 1.03 → 1.06, T4 (translucency 1) 0.91 → 1.05 with
G / R 0.898 → 0.779 (Iray 0.725), T5 1.07 → 1.09, T6 1.06 → 1.08, T7 (no
translucency) 1.09 → 1.08, T8 1.02 → 1.07: spread 0.87 - 1.09 → 1.02 -
1.09, the colour nearer Iray's.

## Process
- The decomposition the issue named settled it before any formula: W2 /
  W4 against their neighbours, and the colour of W5, which no map factor
  can reproduce.
- Compare twin scenes' `.mi` before reading their split: an identical
  pair of references (4 digits) was the hint that -Thin and the solid
  were the same scene.

## Open
- #134: generated spheres are thin-walled unless they say otherwise; the
  closed weighted / Calib-SSS case needs explicit closed variants.
- The private mesh hair (the old ×3 fit's source) loses its brightening
  and gains transmission; it has no reference in git, so it is checked by
  eye on its next export.
