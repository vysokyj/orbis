# 2026-11-22 — Pixie Hair's dual lobe (#216)

#203's compare: on G9 Base dForce Pixie Hair (prisms / ribbons / fibres:
Calib-Hair-Pixie, -T2, -T1, each with a -nogloss twin) Iray's dual lobe
adds 1.22 - 1.55 x in the crown / fringe / side patches, Octane
Faithful's 1.04 - 1.14 x, on every tessellation. Goal: decompose before
fitting. Faithful + Enhanced, Final, 400 samples; linear luminance;
`hair-set-compare.py`.

## Docs and the material first

- Pixie's strands carry two Uber materials. Hair01: Base Mixing 0, base
  0.31 / 0.24 / 0.20, Translucency 0.33, **Refraction 0.28**, no glossy
  layer, Dual Lobe Weight 0.526, Reflectivity 0.42 (F0 0.034), lobe
  roughnesses 0.55 / 0.57 (both rough), ratio 0.85. Hair02 the same but
  Refraction 0.
- `irayubermaterial.mdl`: `add_dual_lobe_specular` puts the diffuse as the
  *layer* of a `custom_curve_layer` (normal 1 − R W, grazing 1 − W) whose
  *base* is the white pair of lobes; `metallic_layered_base` lays the
  refraction at its weight over all of it. DAZ's `refraction_reflection_enable`
  (off without a glossy layer, #143) concerns the refraction's own
  reflection only.
- Iray manual 8.1.4: on fibres a regular BSDF is single sided and its
  transmission discarded.

## The export dropped Hair01's lobe

The Faithful Pixie-T1 `project.ocs`: Hair01 is the refraction Mix (#135:
a partial refraction over translucency), amount 0.284 - and **both parts
at `specular` 0**. Hair02 (no refraction) has its lobe (specular 0.447,
coating 0.079, IOR 1.45). `universal` read `refraction_reflection_enable`
off the material and zeroed the specular on the Mix's base part as well
(refraction 0 there), so Hair01 drew no dual lobe on any tessellation;
Octane's share came from Hair02 and the hairlines alone. No factor
involved: the flag now applies only to a node that holds a refraction
(`refraction > 0`), the base part keeps W × ratio on `specular` and lobe 2
on the coating (test `refraction_mix_base_keeps_its_dual_lobe`).

| Faithful, crown / fringe / side | Octane / Iray before | after | Octane lobe share before | after | Iray share |
|---|---|---|---|---|---|
| prisms (Pixie) | 0.98 / 1.04 / 0.76 | 1.33 / 1.29 / 1.11 | 1.06 / 1.10 / 1.14 | 1.43 / 1.37 / 1.67 | 1.24 / 1.28 / 1.55 |
| fibres (T1) | 0.64 / 0.60 / 0.72 | 0.87 / 0.75 / 1.04 | 1.06 / 1.10 / 1.14 | 1.42 / 1.36 / 1.66 | 1.23 / 1.26 / 1.22 |
| ribbons (T2) | 0.48 / 0.89 / 0.85 | 0.59 / 1.04 / 1.12 | 1.04 / 1.06 / 1.10 | 1.28 / 1.24 / 1.45 | 1.35 / 1.29 / 1.45 |

Enhanced / Iray: prisms 0.98 / 1.04 / 0.75 → 1.30 / 1.28 / 1.06, fibres
0.67 / 0.62 / 0.73 → 0.89 / 0.76 / 1.02, ribbons 0.34 / 0.63 / 0.61 →
0.45 / 0.78 / 0.85.

The -coat twins (#215) re-rendered in Faithful: T1-coat 1.05 / 0.83 /
1.83 → 1.21 / 0.93 / 2.11 x Iray; Octane's coat share over the base that
now has its lobe 1.84 / 1.73 / 2.38 (was 2.14 / 1.92 / 2.99 over a
lobe-less one) against Iray's 1.32 / 1.39 / 1.18; ribbons 1.63 / 1.54 /
2.06 against 2.11 / 1.59 / 1.88. #215's gap shrinks, stays.

The -nogloss twins don't change (no lobe either way). The share's colour
now follows Iray's (B highest: the sky in the sheen, R 1.2 / B 1.5 - 2.1
vs Iray's 1.1 - 1.3 / 1.4 - 1.9). Ribbons match (0.95 - 1.00 x Iray's
share); prisms 1.08 - 1.15 x over, on a base that was already 1.04 - 1.21
x Iray's (-nogloss); fibres 1.08 / 1.08 / 1.36 x over.

## What is left: the lobe on fibres

Iray weights its dual lobe by the curve at the **macro normal** (the base
of a curve layer: 1 − w · max(c(n·k1), c(n·k2)), `libbsdf`, 2026-11-16
note), its glossy layer and Top Coat by the curve at the half vector;
Octane's specular is Fresnel at the half vector for all of them. On a
sphere the two agree within the known rim excess; what Iray's fibres take
as the macro normal is not documented. Calib-Strand-LobeArc-HDRI / -T2
(new, #215's arcs at THIN widths under DTHDR-RuinsB and its sun, rows G
one thick arc, H 300, I 3000 over the top, J 3000 facing the camera):
Pixie opaque (its lobe), the same without the lobe, the lobe alone over
black, its curve as a glossy layer over black. Octane (Faithful, cells x
1000):

| | G | H | I | J |
|---|---|---|---|---|
| Pixie opaque | 1.89 | 2.16 | 2.61 | 0.87 |
| no lobe | 1.11 | 1.34 | 1.71 | 0.76 |
| lobe alone | 0.76 | 0.92 | 0.91 | 0.22 |
| curve as glossy layer | 0.77 | 0.85 | 0.86 | 0.22 |
| lobe share | 1.71 | 1.62 | 1.53 | 1.14 |

Additive (lobe alone = what it adds, 0.79 / 0.82 / 0.91 / 0.11) and the
lobe = the glossy layer within 0.98 - 1.09, as written; ribbons (-T2)
alike in Octane. Enhanced 1.49 - 1.65 (J 1.13). The Iray side decides: its
lobe against its glossy layer on fibres and ribbons. Submitted on the
server: it rendered at ~80 iterations / min (477 → 954 in 6 min, the CPU
fallback of #217, GPU 2000 in ~110 s), cancelled at ~950 / 2000; parked.

## Process

- Read the exported `project.ocs` of the material in question before any
  theory: the whole gap was a lobe the export never wrote.
- A flag DAZ hands Iray for one layer (`refraction_reflection_enable`) read
  off the material leaks into every part the export splits it into.
