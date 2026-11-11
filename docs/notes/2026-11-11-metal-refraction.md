# 2026-11-11 Refraction over a metal (#129)

Goal: #129 said a partial Refraction Weight scales a metal's colour by
1 − refraction. Checked against DAZ's MDL first, then against Iray.

Scenes: Calib-MetalRefr, -g0 (Glossy Layered Weight 0), -TL
(translucency 0.5, colour 0.9 / 0.6 / 0.4); red base 0.45 / 0.05 / 0.04
(the metal's colour), white Glossy Color (the refraction's), gloss 0.5,
roughness 0.2, IOR 1.5. Spheres left to right: R1 / R2 refraction 0.3 /
0.7, M1 / M2 metal 0.75 / 0.3, X1 - X4 metal 0.75 / 0.75 / 0.3 / 0.3 at
refraction 0.3 / 0.7 / 0.3 / 0.7. Iray 400 samples (Iray Server,
bee5dd0), Octane enhanced-final 400 samples, ~40 - 50 s each. Measure:
linear RGB (gamma 2.2) mean over each sphere's disc (r < 0.85 R), Y;
centre r < 0.45 R, rim 0.6 - 0.85 R, "hi" the disc's top 3 %.

## MDL (no render)
`irayubermaterial.mdl`, `metallic_layered_base`: `bsdf_w_metal =
weighted_layer(metallic_weight, metallic_bsdf, <diffuse / backscatter>)`,
then `base_bsdf = weighted_layer(refraction_weight, refraction_bsdf,
bsdf_w_metal)`. The refraction lies over the metal: Iray's metal keeps
1 − r and the refraction is r whatever the metal. So the issue's premise
was wrong (the metal at 1 − r is right); what could be off is
Universal's side: its `transmission` belongs to the dielectric, which
the metallic blend scales by 1 − m'.

## Before (one Universal node, HEAD f347646)
| sphere | Calib-MetalRefr Y | R | G | B | -g0 Y | -TL Y |
|---|---|---|---|---|---|---|
| R1 r .3 | 1.02 | 1.07 | 0.99 | 0.99 | 1.00 | 0.52 |
| R2 r .7 | 1.03 | 0.98 | 1.04 | 1.12 | 1.04 | 0.89 |
| M1 m .75 | 1.15 | 1.11 | 1.24 | 1.35 | 1.13 | 1.13 |
| M2 m .3 | 1.15 | 1.14 | 1.16 | 1.19 | 1.10 | 1.10 |
| X1 m .75 r .3 | 0.47 | 0.74 | 0.32 | 0.32 | 0.43 | 0.41 |
| X2 m .75 r .7 | 0.18 | 0.27 | 0.16 | 0.15 | 0.16 | 0.17 |
| X3 m .3 r .3 | 0.94 | 0.98 | 0.92 | 0.94 | 0.92 | 0.61 |
| X4 m .3 r .7 | 0.69 | 0.77 | 0.66 | 0.67 | 0.56 | 0.50 |

The decomposition holds: refraction alone (R) and metal alone (M) sit
with the rest of the set (1.02 - 1.15). Every combination is dark, worst
at refraction 0.7 and the stronger metal, and darkest in G / B - the
white background seen through the sphere: outcome (a) of the batch, the
transmission lost under Universal's metallic blend. (The renders from
16:15 and the re-render at HEAD agree within 0.02.)

## Change
A Mix material (19) at Refraction Weight: material1 the refraction (the
material at refraction 1, metal 0 - albedo 0, transmission, IOR, gloss),
material2 the opaque metal + base (refraction 0; a translucent one goes
on into `translucent_metal_material`). The same split as #123's
translucent metal; it is DAZ's layer, nothing fitted.

## After
| sphere | Calib-MetalRefr Y | R | G | B | G/R Iray / Octane | -g0 Y | -TL Y |
|---|---|---|---|---|---|---|---|
| R1 | 1.02 | 1.07 | 0.99 | 0.98 | 0.40 / 0.37 | 0.99 | 0.51 |
| R2 | 1.03 | 0.98 | 1.04 | 1.12 | 0.91 / 0.97 | 1.04 | 0.89 |
| M1 | 1.16 | 1.11 | 1.25 | 1.35 | 0.11 / 0.12 | 1.13 | 1.14 |
| M2 | 1.16 | 1.15 | 1.17 | 1.20 | 0.25 / 0.25 | 1.10 | 1.11 |
| X1 | 1.14 | 1.11 | 1.15 | 1.18 | 0.48 / 0.49 | 1.14 | 1.12 |
| X2 | 1.12 | 1.10 | 1.12 | 1.14 | 0.98 / 0.99 | 1.13 | 1.12 |
| X3 | 1.13 | 1.13 | 1.12 | 1.15 | 0.61 / 0.60 | 1.12 | 1.10 |
| X4 | 0.90 | 0.94 | 0.88 | 0.90 | 0.83 / 0.77 | 0.67 | 0.95 |

X1 - X3 now sit with the metals alone (1.12 - 1.14 vs 1.16) in all three
scenes, their colour as Iray's. -g0 says the refraction's own reflection
needs nothing more (X1 - X3 1.12 - 1.14 without the glossy layer); -TL
says the translucent opaque part works through the existing
translucent-metal Mix (X1 - X4 1.10 - 1.12, 0.95).

X4 is the exception: it sits in front of the floor's back edge. Through
R2-like refraction Octane shows that edge sharp and straight, Iray blurred
(a white smear over the sphere's lower half) - a thin-walled rough
refraction question, not the metal (#136; the spheres are thin-walled,
#134). The -g0 X4 0.67 is the same sphere with less of its own gloss to
hide it.

## Left open
- R1-TL 0.51, R2-TL 0.89 (no metal): a partial refraction with
  translucency keeps only one of the two transmissions (#135).
- M1 / M2 at 1.15 are the set's metal baseline, unchanged here.
