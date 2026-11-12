# 2026-11-12 Partial refraction under translucency (#135)

Goal: #135 - on #129's Calib-MetalRefr-TL (translucency 0.5, Translucency
Color 0.9 / 0.6 / 0.4, red diffuse 0.45 / 0.05 / 0.04, white refraction,
gloss 0.5 at roughness 0.2, IOR 1.5) the two refracting spheres without a
metal rendered R1 (Refraction Weight 0.3) 0.51 × Iray and R2 (0.7) 0.89.
The same spheres without translucency (Calib-MetalRefr) 1.01 / 1.04.

Variant: enhanced-final, Octane 400 samples (~50 s a scene); Iray
reference 400 samples (calib/references/iray/Calib-MetalRefr-TL.png).
Measure: linear RGB (gamma 2.2) mean over each sphere's disc (r < 0.85 R;
the discs at x = 424 + 153.3 i, y 562, R 46 px in 1920 × 1080 - not
Calib-Glossy's, whose spheres sit elsewhere: those discs gave R1 0.76).

## Thin-walled first (MDL, no render)
#134 found the generated spheres thin-walled. What that does here, from
`irayubermaterial.mdl`:
- `metallic_layered_base` builds the surface the same way thin or not:
  `weighted_layer(refraction_weight, refraction_bsdf, bsdf_w_metal)`, the
  base `diffuse_base` = `weighted_layer(1 - w, diffuse, diffuse_transmission(Tc))`.
  So Iray has both transmissions in either case: r of the refraction, (1 -
  r) · w · Tc of the translucency.
- `thin_walled` enters only in `add_uber_volume` (the material's
  `thin_walled` flag, and with it no interior volume) and two-sided
  emission. On a thin wall MDL's specular transmission passes straight
  through (no bending); on a closed solid it refracts into the volume.
  The diffuse transmission is a diffuse transmission either way.

So the thin-walled reference is a legitimate case of its own for the
layer split; the bending through a closed solid is not decided by it.

## Octane side before (one Universal node)
`universal` gave the one `transmission` to the translucency below
refraction 0.5 (the refraction's own dropped), to the refraction from 0.5
(the translucency dropped, the diffuse at 1 - r only). R1 thus lost 0.3
of straight-through light, R2 lost the (1 - 0.7) · 0.5 · Tc of the
translucency. The measured shortfall fits: R1 dark in every channel (the
white background through it gone), R2 short in R (Tc's red).

## Change
The same Mix as #129's refractive metal (`refractive_metal_material`),
now also for a translucent non-metal with 0 < Refraction Weight < 1, Base
Mixing 0, not PBRSkin: material1 the material at refraction 1 (albedo 0,
its transmission - Thin Wall on a thin wall -, IOR, gloss), material2 at
refraction 0 (diffuse at 1 - w, the translucency at its full share,
Thin Wall (Diffuse) on a thin wall), amount = Refraction Weight. Nothing
fitted; it is DAZ's layer.

## Result (Octane / Iray)
| sphere | before Y | R | G | B | after Y | R | G | B |
|---|---|---|---|---|---|---|---|---|
| R1 r .3 | 0.51 | 0.55 | 0.48 | 0.60 | 1.04 | 1.03 | 1.05 | 1.04 |
| R2 r .7 | 0.90 | 0.81 | 0.91 | 1.05 | 1.07 | 1.04 | 1.07 | 1.14 |
| M1 | 1.13 | | | | 1.13 | | | |
| M2 | 1.10 | | | | 1.11 | | | |
| X1 | 1.12 | | | | 1.12 | | | |
| X2 | 1.12 | | | | 1.12 | | | |
| X3 | 1.10 | | | | 1.10 | | | |
| X4 | 0.93 | | | | 0.94 | | | |

R1 / R2 now sit beside the refractions without translucency (Calib-
MetalRefr R1 / R2 1.01 / 1.04, R2's B 1.12 there too) and the metal
spheres; the metal ones are unchanged (they already went through the
Mix). Calib-Eye's export (the Genesis EyeSocket, which the one-node rule
was checked on in #20) comes out byte-identical: the Mix doesn't reach it.

## Left
- Base Mixing 1 (Morgan Hair's strands, refraction 0.106) and PBRSkin
  keep the one-node rule - same layer in DAZ's MDL, but unmeasured here.
- The closed solid: Calib-MetalRefr-TL-Solid's Octane side re-rendered
  with the Mix (R1 / R2 1.09 / 1.12 of the thin Octane render, R1 B
  1.36: the solid bends the background), its Iray reference waits in
  #134's batch.
- Not #136: R2's thin-walled rough refraction looks alike in both now;
  #136's X4 (in front of the floor edge) is unchanged at 0.94.
