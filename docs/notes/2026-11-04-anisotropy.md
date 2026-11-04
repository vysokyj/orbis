# 2026-11-04 Anisotropy, second round (#23)

Goal: settle the rotation's sign, check Calib-Aniso2's K3 – K8, export the
Top Coat's anisotropy in Layering Modes 0 / 2 / 3 (the user's decision on
[#23](https://github.com/vysokyj/orbis/issues/23): a Layered material),
and NVIDIA Metal Brushed's roughness.
Scenes: Calib-Aniso2 (Iray reference of 2026-11-04, 5000 samples, Iray
Server), Calib-Aniso (2026-10-30), Calib-NVIDIA (refreshed 2026-11-04).
Variant enhanced-final, Octane 400 samples, 1920 × 1080, ~30 s each.
First round: [2026-11-03](2026-11-03-anisotropy.md).

## Metric

Mean linear luminance (PNG decoded at 2.2, like `calib-compare.py`) in a
disc of radius 40 px around each sphere's centre (x = 424 + 153 k,
y = 560), Octane / Iray; a scratch script. Shapes by eye on 3× crops.

## Rotation sign

As exported (Octane `rotation` = Iray's), K1 (0.125) looked like Iray's
K2 (0.375) and K2 like Iray's K1. `set=` K1 0.875, K2 0.625 (1 − r): both
match Iray's streaks. Rule: Octane's rotation = 1 − Iray's (mod 1; 0
stays unwritten). Calib-Aniso's 0.25 spheres go to 0.75, the same
streak (a highlight is symmetric under half a turn).

## K3 – K8 as exported (before the fixes below)

| sphere | ratio | shape |
|---|---|---|
| K1 rot 0.125 | 1.07 | mirrored |
| K2 rot 0.375 | 1.09 | mirrored |
| K3 metal r 0.2 a 1 | 1.52 | streak right, body too light |
| K4 weighted coat a 1 | 1.31 | streak right, body too light |
| K5 top coat (mode 0) a 1 rot 0.25 | 1.21 | sharp round coat, Iray a horizontal streak |
| K6 backscatter 1, gloss a 0.5 | 0.66 | dark mirror; Iray a flat tan disc |
| K7 a 0.25 | 1.09 | right |
| K8 a 0.75 | 1.08 | right |

## K3 / K4: energy-preserving GGX at roughness 1

K3: Iray r 0.04, v = r / 0.0001 → Octane roughness 1, anisotropy 0.92.

| try on K3 | K3 |
|---|---|
| as exported (brdf 6) | 1.52 |
| roughness 0.7 / 0.5 / 0.3 | 1.69 / 1.65 / 1.53 |
| brdf 2 (plain GGX) | 1.08, shape as Iray |

Then brdf 2 on every material of both scenes: K4 1.31 → 0.96, the
dielectric spheres within 0.01, but Calib-Aniso's metal A6 (a 0.8, v 0.8)
1.12 → 0.86. Octane's energy-preserving GGX adds the light of multiple
scattering, which at roughness 1 is a lot; Iray's lobe at r / 0.0001 is
flat along v and doesn't have it. Rule: plain GGX only where Iray's long
axis passes 1 (r / (1 − a) > 1, `Anisotropy::saturates`). As coded:
K3 1.08, K4 0.96, Calib-Aniso unchanged (1.08 – 1.12, A7 1.22).

## Top Coat anisotropy as a Layered material

Manual: Layered material = base material + up to eight material layers;
the Specular Layer has anisotropy and rotation (and lists a
Retro-Reflective option the probe doesn't show in this Octane X).
Probe (scratch Lua, `octane-run.sh`): `NT_MAT_LAYER` = 143 (pins
`material1`, `customAov`, `customAovChannel`; attribute `pin_count`, the
layer count, adds dynamic pins `Layer N`), `NT_MAT_SPECULAR_LAYER` = 139
(pins `enabled`, `specular`, `transmission`, `brdf` default 2,
`roughness`, `affectRoughness`, `anisotropyTexture`, `rotation`,
`spread`, `ior` 1.5, `hasCaustics`, `filmwidth`, `filmIor`, `thinLayer`
true, `bump`, `bumpHeight`, `normal`, dispersion, `opacity`). Saved .ocs:
`<attr name='pin_count' type='2'>1</attr>` and `<pin name='Layer 1'
dynamicType='33' connect='…'/>`; the export nests the layer node inside
the pin, which Octane loads.

Export: the Universal (or Mix) as `material1` without its coating, the
layer = coating colour × weight, `top_coat_ior`, the coat's roughness /
anisotropy / rotation via `gloss_roughness_pins`, brdf 6 / 2 as above.

| sphere | coating | Layered |
|---|---|---|
| K5 (mode 0, rot 0.25) | 1.21, round | 1.08, Iray's horizontal streak |
| A7 (mode 0) | 1.22, round | 1.16, Iray's vertical streak (Octane's a bit sharper) |

Modes 2 and 3 take the same layer at their IOR; no anisotropic reference
of them (not needed: the modes differ only in the IOR, calibrated on
Calib-TopCoat).

## NVIDIA Metal Brushed

MDL roughness u = 0.02 + 0.4 l, v = 0.35 + 0.325 l straight into
`simple_glossy_bsdf`. N1 against the new reference:

| try | N1 | shape |
|---|---|---|
| √(u v), Octane's default BSDF (before) | — | (not rendered this session) |
| long axis, default BSDF | 1.26 | round highlight, no streak |
| long axis, brdf 6 | 1.17 | Iray's vertical streak |
| long axis, brdf 2 | 1.14 | streak, alike |
| GGX curve on both axes, brdf 6 / 2 | 1.27 / 1.10 | too broad |

Kept: long axis, as is, brdf 6 (as the Uber's unsaturated lobes). Cast
Iron (N2) 1.21 either way.

## K6: not anisotropy

DAZ hands no backscattering anisotropy (first round). K6 at
Backscattering Weight 1 renders the backscatter stand-in (a dark metal
mirror, #29) where Iray shows a flat tan disc (light from behind the
camera): 0.64 ×. A backscatter question, not #23's: [#73](https://github.com/vysokyj/orbis/issues/73).

## Process

- `orbx-variants.py set=…brdf=e2` per material, `brdf=2` for all: one
  batch decided the K3 question.
- The cache's .orbx were cleaned by another session mid-run: export to
  the scratchpad instead.
