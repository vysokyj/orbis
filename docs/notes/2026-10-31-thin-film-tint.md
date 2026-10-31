# 2026-10-31 — Thin films: Iray's tint from DAZ's MDL (#31)

## Where it stood

The full calibration pass left Calib-ThinFilm at 1.00 – 1.82 × Iray:
Iray's black glossy spheres are coloured, Octane's plain. The export
dimmed the albedo under a film by a fit, 1 − (0.053 + 1.6 R)
(Calib-FilmIOR), with no hue.

## Measured tints

Iray / Octane per channel over each sphere, divided by the plain sphere's
ratio (Octane had no film, so this is the film's effect):

| sphere | R G B |
|---|---|
| H2 300 nm, IOR 1.5 | 0.87 0.79 0.72 |
| H3 500 nm | 0.86 0.90 0.66 |
| H4 800 nm | 0.93 0.73 0.85 |
| H5 500 nm, IOR 1.3 | 0.68 0.65 0.91 |
| H6 coat film 500 nm | 0.84 0.86 0.60 |
| H7 metal, 500 nm | 0.92 0.92 0.54 |
| H8 1200 nm | 0.70 0.77 0.78 |

The same film tints a gloss, a coat and a metal alike: a multiplier, not a
layer. A physical Airy film (air / film / substrate, the substrate's IOR
and a thickness scale free) fit none of the hues.

## DAZ's MDL

DAZ Studio ships the Iray Uber source:
`DAZStudio.app/Contents/shaders/iray/daz_3d/irayubermaterial.mdl`.
`add_thin_film` wraps the material under it in `df::directional_factor`
(normal_tint = `compute_refl(ior, d, 0.99)`, grazing_tint =
`compute_refl(ior, d, 0.10)`, exponent 1.1), with
`nvidia/legacy_thin_film.mdl`'s emulation: 16 wavelengths, CIE → linear
sRGB. Base Thin Film wraps the whole base (diffuse included), Metallic
Flakes Thin Film the flakes, Top Coat Thin Film everything with the coat.

Head-on / grazing tints: 300 nm 0.83 0.91 1.00 / 0.97 0.41 0.00; 500 nm
0.82 0.96 0.89 / 0.97 0.77 0.05; 800 nm 0.97 0.90 0.91 / 1.03 0.27 0.78;
500 nm at 1.3 0.99 0.94 0.99 / 0.06 0.11 0.92. Calib-FilmIOR at 1.91: head-on
0.77 – 0.83 — the diffuse dimming the earlier fit had found.

## Octane

Probe 103: Falloff Map is node 50 (`mode`, `applyBump`, `normal`,
`grazing`, `falloff index`, `direction`); Mix 38, Multiply 39. The colour
pins become Multiply(Mix(head-on, grazing, Falloff), value). (θ / 90°)^s
fits (1 − cos θ)^1.1 best at s = 1.92 (within 0.015, weighted by a
sphere's projected area).

First render: the rim's tint in the middle — Mix's amount 1 is `texture2`
(as `damp_ao_albedo` already used it). Swapped.

| scene | before | now |
|---|---|---|
| Calib-ThinFilm | 1.08 1.28 1.21 1.82 1.31 1.27 1.21 1.00 1.24 | 1.08 1.09 1.10 1.28 1.12 1.09 1.10 1.09 1.09 |
| Calib-FilmIOR | 1.09 1.16 1.09 1.15 1.12 1.11 1.17 1.20 1.12 | 1.09 1.10 1.11 0.96 1.12 1.12 1.14 1.14 1.14 |

(The first number is the plain sphere.) The hues match by eye; 800 nm
stays brightest (1.28). Not done: Metallic Flakes Thin Film (needs #30),
refraction-only materials that keep Universal's default specular.
