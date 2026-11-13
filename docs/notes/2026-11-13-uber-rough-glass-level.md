# 2026-11-13 — The Uber rough glass level from `libbsdf` (#118)

Question: the export rendered Uber rough glass (refraction ≥ 0.5, closed)
as Beckmann at the Phong pin with no level factor; Calib-Dispersion-Rough's
frosted spheres were 1.07 – 1.09 × Iray, put down to Smith vs v-cavities.
#116 / #117 showed the Uber's rough transmission is `libbsdf`'s ring for
ring, so a k per material from the two furnace-sphere models
(`mdl_layer::tint_factor`, as resin's) should remove it with nothing
measured. Octane enhanced-final, denoised pass; no Iray this session (the
Calib-Furnace-UberGlass and Calib-Dispersion-Rough references in git).

## Premise check

`git log -S phong_pin` / the code: Uber glass had no level term
(`GLASS_BRDF`, `phong_pin`); only the NVIDIA modules had `tint_factor_*`.
Calib-Furnace-UberGlass's spheres are closed (`'Thin Walled': False` in its
generator, #134). The export writes T1 – T6 (no glossy layer) with
specular 0 (Octane transmit-only, as Iray's `scatter_transmit`), T7 / T8
(shared, full reflection) with the default specular 1 (Octane reflect +
transmit by Fresnel, as Iray's `scatter_reflect_transmit`).

DAZ's `refraction_base` (irayubermaterial.mdl): not split → one
`choose_bsdf` lobe, reflect_transmit or transmit; split (own inputs, a
partial reflection, dispersion) with reflection → `fresnel_layer` of weight
1 or Glossy Layered Weight at the IOR, reflection at the glossy roughness,
over a transmit lobe; split without → the transmit lobe alone. All three
are `mdl_layer::Stack`s.

## Decomposed (disk r < 0.75)

Calib-Furnace-UberGlass, before the change (fresh Octane render, 1024
samples) against the two models (`furnace-rings.py model`, 2^18 paths):

| sphere | Iray | `libbsdf` | Smith model | Octane | Iray / libbsdf | Smith / libbsdf | Octane / Smith | Octane / Iray |
|---|---|---|---|---|---|---|---|---|
| T4 transmit r 0.218 | 0.982 | 0.990 | 0.992 | 0.996 | 0.99 | 1.00 | 1.00 | 1.01 |
| T5 r 0.354 | 0.927 | 0.938 | 0.956 | 0.967 | 0.99 | 1.02 | 1.01 | 1.04 |
| T6 r 0.6 | 0.769 | 0.780 | 0.857 | 0.869 | 0.99 | 1.10 | 1.01 | 1.13 |
| T7 reflect_transmit r 0.25 | 0.963 | 0.974 | 0.981 | 0.990 | 0.99 | 1.01 | 1.01 | 1.03 |
| T8 r 0.5 | 0.808 | 0.819 | 0.881 | 0.920 | 0.99 | 1.08 | 1.04 | 1.14 |

Nearly all of Octane's excess is the model difference (v-cavities against
Smith masking, both derived); the rest is Octane over its own model (1 – 4
%, #148's gap) and Iray 1 % under `libbsdf`. Neither goes into k.

## The rule and the re-render

`uber_glass_level` (src/material.rs): the stacks above, Octane as exported
(Beckmann at α = pin², Smith, specular = the layered weight), `tint_factor`
per channel on the transmission colour. k: T3 0.9997, T4 0.997, T5 0.990,
T6 0.964, T7 0.997, T8 0.964.

| sphere | Octane / Iray disk before → after | rings after, centre → 0.8 – 0.9 |
|---|---|---|
| T4 | 1.014 → 1.011 | 1.00 … 1.02 |
| T5 | 1.043 → 1.023 | 1.01 … 1.03 |
| T6 | 1.130 → 1.051 | 1.06 … 1.05 |
| T7 | 1.028 → 1.023 | 1.02 … 1.04 |
| T8 | 1.139 → 1.062 | 1.05 … 1.10 |

Regression: Calib-Dispersion-Rough (400 samples, HDRI, the inner disk;
baseline = the same export with the transmission reset to 1 through
`orbx-variants.py set=`): R5 shared r 0.36 1.093 → 1.062 (k 0.985), R7 own
1.072 → 1.052 (k 0.990), the six other spheres (dispersive, or r 0.09:
k ≥ 0.9998) unchanged at 1.01 – 1.04. Calib-Glass: R6 rough (r 0.09) k 1,
the rest unchanged (exports checked, no re-render). Calib-Eye / -Eye-G9 /
Calib-MetalRefr: thin walls, partial refraction or translucency — untouched.

Fake shadows (#104): left on — the furnace's spheres have no diffuse or
light vertex that would send a shadow ray through a second wall, and the
remaining Octane / Smith gap is the same on the transmit-only and the
reflect_transmit spheres' centres; not tried as a variant.

## Where it stands

Done (#118). Open, not this issue: Octane over its Smith model (~4 % at α
0.5, larger with the reflection lobe, T8); Octane's coating flat where
Iray falls to the rim for layered stacks (#117); rough thin walls stay
sharp (#136); a roughness map keeps k 1 (one value per material).

## Process lessons

- The model columns (`furnace-rings.py model`) took seconds and split the
  whole render ratio before any code change; the render then only
  confirmed the predicted factor.
- A roughness ≤ 0.1 needs no k (≤ 0.03 %); only frosted glass moves.
