# 2026-10-28 — NVIDIA MDL dielectrics (#21)

Ratios: Octane / Iray, mean linear luminance (gamma 2.2) over a box at the
sphere's centre (Calib-NVIDIA renders at 1920 × 1080, Octane 1024 samples).

## What the scenes use

Outside calibration, the user's library uses NVIDIA concrete 26 ×,
porcelain 8, architectural 7, concrete polished 4, granite black 3,
asphalt 3, resin polyurethane matte 2, ceramic tiles small 1. Before this
note each one exported as a plain Universal diffuse: no gloss, no bump,
no tiling (the brick graph's MDL was not read).

## Reading the MDL

The sources are in `DAZStudio.app/Contents/shaders/iray/nvidia/*.mdl`, the
maps in `shaders/iray/resources/` (scenes write them as
`/resources/<file>`). All but architectural and resin are one
`df::fresnel_layer(ior, weight, simple_glossy_bsdf, diffuse_reflection_bsdf)`.
The details differ per module:

| module | gloss weight | gloss tint | diffuse | bump |
|---|---|---|---|---|
| concrete | `glossy_weight` | bump map (colour) | colour map | bump map × amount |
| concrete polished | colour map's `mono_average` × weight | white | colour map × weight (sic) | on the base only |
| asphalt, ceramic tiles | bump map mono × weight | white | colour map | bump map |
| granite black | `specular_weight` | white, `specular_bsdf` (mirror) | colour map | — |
| porcelain | `glossy_weight` | white | `base_color` (`diffuse`) | — |
| resin matte | `glossy_weight` (clear coat) | — | base: glossy reflect + transmit tinted `base_color` | — |

DAZ channel ids: the colour map is `Diffuse Color`, the bump `Bump
Strength`, the base colour `diffuse`, the IOR `Index of Refraction`
(asphalt `coating_ior`), the rest by the MDL parameter name.

Architectural is mental ray's mia_material: diffuse × weight; reflection
weight = reflectivity × max(refl_color), tinted by it; with "Use Fresnel
Term" off a `custom_curve_layer` (0°, 90°, exponent) = Schlick at the
defaults (0.2, 1, 5), which is Fresnel at the IOR whose F0 is 0.2 (2.62).
`base::architectural_gloss_to_rough` is not in any shipped file (compiled
into Iray); (1 − g)² is a guess. Calib-NVIDIA has glossiness 1 (mirror)
only.

## Result

| sphere | Octane / Iray |
|---|---|
| concrete | 1.04 |
| concrete polished | 0.98 |
| porcelain | 1.11 |
| granite black | 1.12 |
| architectural | 1.07 |
| resin (full colour as transmission) | 1.36 (box 1.27), g/r 0.30 v 0.37 |

Resin variants (`orbx-variants.py set=N7_resin_matte_Default.…`):

| variant | lum | r : g : b |
|---|---|---|
| Iray | 1.00 | 1 : 0.371 : 0.514 |
| export (T = colour) | 1.36 | 1 : 0.304 : 0.430 |
| specular 0 | 1.27 | 1 : 0.317 : 0.437 |
| T × 0.85 | 1.23 | 1 : 0.365 : 0.508 |
| T × 0.75 | 1.16 | 1 : 0.414 : 0.572 |
| T × 0.6 | 1.06 | 1 : 0.500 : 0.685 |
| T × 0.4 | 0.97 | 1 : 0.634 : 0.859 |

Less transmission lets more of the grey background through (hue to blue)
before the level drops; × 0.85 is the hue, × 0.6 the level. Took 0.8.
Visually Iray's resin shows more of the background's structure through
the sphere; Octane's is smoother.

## Not resolved

- **Metal Brushed renders black in Iray** on Calib-NVIDIA although the
  scene sets its three brush maps (the same `/resources/…` paths concrete
  loads fine) and its Metal Color is 0.82. Octane shows brushed silver.
  Cast Iron is darker and sharper in Iray (3.6 × in Octane at the centre).
  Both were like this since the metals were added (octane35, 27 Oct).
  An Iray probe (the preset's channels removed → MDL defaults; or the
  maps re-pointed) would tell whether Iray fails to load them.
- Real scenes with concrete (Sport.duf) have no dump yet; not checked
  there.
