# 2026-11-13 Thin-walled glass with a glossy layer in Faithful (#101)

Goal: Faithful's `thin_glass_ior_one` wrote IOR 1 on every thin-walled
refraction (unless the IOR held a gloss's F0, #79); the issue suspected
it removed a reflection Iray does show. Variant faithful-final, 400
samples, Calib-G9Eye against its 5000-sample Iray reference (no new Iray).

## What Iray does (docs first)

`irayubermaterial.mdl`, `metallic_layered_base`: the refraction is a
`weighted_layer` at Refraction Weight over everything else, so at
refraction 1 the glossy top coat (the Schlick curve, #100) is hidden
under it. `refraction_base` reflects through its own BSDF:
`scatter_reflect_transmit` when `refraction_reflection_enable` (DAZ sets
it with a glossy layer, `src/iray.rs`), Fresnel at `glossy_ior` = the
Refraction Index; with Glossy Layered Weight below 1 the full flag is off
and the reflection is a `fresnel_layer` of that weight at the same IOR.
On a thin wall the material's IOR still counts for the Fresnel, only the
bending goes (MDL spec 13.1's thin-walled table).

So with a glossy layer the reflection is the refraction's own Fresnel at
the Refraction Index x the layered weight: what Universal does with the
`specular` weight at that `ior`, i.e. Enhanced's export. IOR 1 is right
only without a glossy layer (Calib-Glass R5, the origin of the rule:
layered weight 0), where `specular` is 0 as well.

## The export before

Genesis 9's EyeMoisture and Tear: Universal, `specular` 0.40 (the layered
weight), transmission 1, Thin Wall, `ior` 1.38 in Enhanced and **1** in
Faithful: no reflection at all.

## Fix and measure

IOR 1 only where Iray's refraction doesn't reflect
(`refraction_reflection_enable` not true; materials `crate::iray` doesn't
cover keep the old rule). Faithful's moisture and tear now match
Enhanced's (1.38); nothing else in the scene moved.

Calib-G9Eye, faithful-final denoised, Octane / Iray (linear luminance,
median per region; eye = the opening as an ellipse, mean):

| region | before | after |
|---|---|---|
| iris | 0.95 | 0.95 |
| sclera left | 1.08 | 1.03 |
| sclera right | 1.14 | 1.09 |
| eye opening | 1.063 | 1.050 |
| frame | 0.992 | 0.992 |

0.13 % of the pixels move by more than 0.01, all on the eye, lashes and
brows (in the moisture's view). The reflection takes its share from the
transmission, so the sclera seen through the moisture darkens ~5 %
towards Iray; the scene's soft light gives no distinct highlight to judge
by eye. The private Calib-Eye was not rendered (Calib-G9Eye covers the
same material family).

## Found on the way

`normalized_base` (the other mixing entry) calls `refraction_base` with
the reflection flags hard-wired on: a Base Mixing 1 refraction without a
glossy layer may reflect in Iray where the export writes `specular` 0 -
#143, not measured.
