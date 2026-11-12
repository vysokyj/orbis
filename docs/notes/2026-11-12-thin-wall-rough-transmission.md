# 2026-11-12 Rough thin-walled transmission (#136)

Goal: #136 - on #129's Calib-MetalRefr (thin-walled spheres, white
refraction, glossy roughness 0.2 shared, IOR 1.5) Iray smears the floor's
back edge seen through R2 / X2 / X4, Octane draws it as a sharp line; X4
(metal 0.3, refraction 0.7) is the one sphere left low, 0.88 × Iray (0.67
without the glossy layer, -g0), its neighbours 1.10 – 1.14. The spec kept
a thin-walled refraction at the glossy layer's roughness on the premise
that a thin wall's transmission isn't blurred (the cornea, #20).

Variant: enhanced-final, Octane 400 samples (~36 s a render); Iray
references 400 samples (calib/references/iray/Calib-MetalRefr{,-g0,-TL}.png).
Measure: linear luminance (gamma 2.2) over each sphere's disc (r < 0.85 R,
x = 424 + 153.3 i, y 562, R 46 px). No Iray render needed: the existing
thin-walled references are this issue's subject; the closed twin
(Calib-MetalRefr-Solid) waits in #134's batch and isn't needed here.

## Docs first
- DAZ's `irayubermaterial.mdl`, `refraction_base`: the transmission is
  `choose_bsdf(df_mode, ..., mode: scatter_transmit / reflect_transmit)`,
  i.e. `simple_glossy_bsdf` at the glossy roughness (shared) or the
  refraction's (own), with nothing that depends on `thin_walled`; only
  dispersion forces roughness 0. `thin_walled` enters only in
  `add_uber_volume` (#135's reading). So Iray's thin-walled transmission
  is a glossy lobe as wide as the reflection's.
- Iray release notes, 2021.0.0 beta: the legacy multiplication of the
  Phong exponent by 7 for thin-walled transmission of `simple_glossy_bsdf`
  was removed - since then a thin-walled transmission blurs at the full
  roughness. The premise in the spec was wrong for Iray.
- Octane manual, Universal: Thin Wall transmission is Specular "with no
  refraction and no roughness for transmission". The Specular, Standard
  Surface and OpenPBR materials have a `thinWall` bool; their pages say
  nothing about roughness there.

## Probes
orbx variants of the exported scene (`orbx-variants.py set=`, R1 / R2 the
plain Universal spheres):

| variant | what | view through R2 |
|---|---|---|
| twr5 | Thin Wall, roughness pin 0.2 → 0.5 | sharp edge, only the reflection blurs |
| s1 | `transmissionType` 0 (Specular), IOR 1 | sharp, no reflection at all |
| s1r5 | the same at pin 0.5 | sharp |
| s105 | Specular, IOR 1.05 | bends: a lens, the floor upside down at the rim |

Microfacets between equal media don't scatter, so Specular at IOR 1 can't
blur; any IOR above 1 bends the view through the closed sphere like a
solid. Probe 136 (`scripts/dev/octane-probe-thinrough.lua`) put other
materials in R1 / R2's place:

| variant | material | view through R2 |
|---|---|---|
| spec_tw / spec_tw_r5 | Specular, `thinWall`, pin 0.2 / 0.5 (IOR left at its default) | sharp |
| spec_tw_r5_ior | the same, IOR 1.5 | sharp |
| std_tw_r5_extra | Standard Surface, `thinWall`, roughness 0.5, `roughnessExtra` 0.5 | sharp |
| openpbr_tw_r5 | OpenPBR, `thinWall`, transmission 0.7, roughness 0.5 | sharp |

(Run 1 had OpenPBR's and Standard Surface's pins misnamed - `transmission`
and `roughness`, not `transmissionWeight` / `specularRoughness` - so it
proved nothing for them; run 2 set them.) No Octane material blurs a
thin-walled transmission: the limit is Octane's, not the export's.

## X4's deficit is the blur
A decomposition, not a fit. The probe's `bg` variant made R1 / R2 / X4
clear (Thin Wall, transmission 1, specular 0, IOR 1): the background as
a straight ray through a thin wall sees it. Octane's sphere over its disc
regressed on that background (`a · bg + c`), then the background
Gaussian-blurred in its place, against Iray (-g0, no glossy layer, so the
transmission dominates):

| sphere | Octane / Iray | σ 0 | 8 | 16 | 24 | 32 px |
|---|---|---|---|---|---|---|
| R2 | 1.05 | 1.05 / 25 | 1.05 / 17 | 1.02 / 9.9 | 0.99 / 5.8 | 0.96 / 5.8 |
| X4 | 0.67 | 0.67 / 17 | 0.76 / 11 | 0.94 / 5.5 | 1.14 / 9.4 | 1.36 / 15 |

(mean ratio / pixel rms in ‰ of linear luminance.) One blur explains each
sphere's pixels four times better than the sharp background; X4's mean
goes 0.67 → 0.94 at σ 16 px. R2 wants a wider σ because the floor's edge
behind it is farther away (an angular blur). X4 sits before the floor's
corner, and its deficit lives in the quarter that sees the floor
(Octane / Iray, full scene: top 0.96, bottom 0.79, left 0.78, right 0.97;
-g0: 0.83 / 0.55 / 0.55 / 0.85); R2 and X2, before an even background,
stay within 4 % of their mean either way. The absolute gap (Iray −
Octane over X4's disc) is 0.011 in -g0, 0.007 with the glossy layer (it
reflects part of what would pass), 0.005 with translucency (-TL, 0.94).

## Outcome
- No export change: Octane can't render the blur, and faking it (a share
  of Thin Wall (Diffuse) by roughness) would be a constant fitted to a
  look. The spec now records it as a known limit under Thin Walled, and
  the cornea rule's reason is corrected: the roughness pin shapes only
  the reflection on a thin wall, so it takes the glossy layer's - the
  numbers of #20 stand, the code is unchanged.
- `docs/octane/octane-x.md`: thin walls transmit unblurred in every
  material, with the pin names.

## Process
- Look at the picture first: the quadrant split (where the deficit is)
  tied X4's ratio to the background structure before any probe.
- A probe that sets pins by name must log every match - run 1's OpenPBR
  and Standard Surface variants silently kept their defaults.
- A probe variant that leaves a material changed carries over to the
  next variant unless reset (run 2's X4 stayed clear after `bg`).
