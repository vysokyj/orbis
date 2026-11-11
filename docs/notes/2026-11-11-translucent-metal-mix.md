# 2026-11-11 Metallic maps and translucent metals (#123)

Goal: under a Metallic Weight map the albedo was dimmed as a whole by the
diffuse-only factors (F(σ), and a translucency loss fitted at 0.35 × t,
`METAL_MAP_TRANSLUCENCY_LOSS`), which dimmed the map's metal too (#109
left maps on the old rule). Drop the fitted 0.35.

Scene: Calib-PBRSkin-MetalTL (its Iray reference was already in git),
variant enhanced-final, 400 samples. Measure: Octane / Iray linear
luminance, 41 px boxes at nine offsets (±8 px) around each sphere's
centre (1920 × 1080, centres at x 424 + 153.5 i, y 560), mean ± spread.
(The #109 note's boxes came from the dump's transforms; its numbers aren't
comparable one to one with these.)

## Attempt 1: the #109 rule per texel (bake)

Per texel w(x) (the map at its gamma, its value range, × the value),
Iray's (M, B) by the metal law and S = M + B d: `metallic` M / S and a
scale S that multiplies the albedo and `specular` at 1 / d - exactly
`metal_over_diffuse` at every texel, two baked grey maps and two
Multiply textures (`metal_map_bake`).

| sphere | HEAD (flat 1 − t, maps 0.35 t) | bake (d = 1 − t) |
|---|---|---|
| M1 metal 0.75 | 1.07 ± 0.04 | 1.07 |
| M2 metal 0.75 tl 0.85 | 1.04 ± 0.06 | 1.04 |
| M3 metal 0.75 tl 0.5 | 1.04 ± 0.04 | 1.04 |
| M4 metal 0.3 tl 0.85 | 0.83 ± 0.07 | 0.83 |
| M5 metal 0 tl 0.85 (transmission) | 1.04 ± 0.01 | 1.05 |
| M6 white map tl 0.85 (w 0.75) | 0.93 ± 0.01 | 1.01 ± 0.03 |
| M7 grey map tl 0.85 (w 0.16) | 1.42 ± 0.06 | 0.68 ± 0.03 |
| M8 grey map | 1.10 ± 0.00 | 1.09 |

The bake does what it should (M6, a white map at w 0.75, now renders as
the flat M2; the baked texels checked: `metallic` 145 / 255 = 0.165 /
0.290, S 74 / 255), but M7 fell to 0.68: the decomposition's own premise,
that the translucency's share of the base is lost (d = 1 − t), fails as the
metal thins - M4 (w 0.3, the flat rule) 0.83, M7 (w 0.16) 0.68, while M5
(w 0, Octane's transmission) is right at 1.04. The light the base passes
into the closed sphere comes back out; at w 0.75 the metal dominates and
hid it (#109's M2 / M3). The fitted 0.35 had stood in for exactly this on
the map spheres. A dead end as the translucency rule; kept for the
F(σ) part (opaque rough metal maps), where d is exact.

A cross-sphere check was a mistake: solving "metal" and "opaque base"
from M1 and M8 and predicting M2 / M4 / M6 / M7 as linear mixes with M5
gave M6 ≠ M2 in Iray (0.0945 vs 0.1324) - the spheres sit in different
parts of the HDRI, so identical materials differ by position. Only
per-sphere Octane / Iray ratios compare.

## Attempt 2: a Mix of the metal and the translucent base

Iray's metal is a `weighted_layer` at w over the base (`pbr_skin.mdl`; Uber
`metallic_layered_base` with its coat at w over black), the base holding
the diffuse at 1 − t over `translucency_bsdf`. A weighted layer is a
per-hit mix, as Octane's Mix material (19) is. So: Mix by the Metallic
Weight (map × value) of the material at weight 1 (Universal metal, its
colour) and at weight 0 (the translucent base, transmission as on any
non-metal - M5's rule, 1.04); Uber's metal side a second Mix at w with a
black Universal (w² metal + (1 − w) base). No constant.

| sphere | Mix |
|---|---|
| M1 | 1.07 ± 0.04 |
| M2 | 1.06 ± 0.05 |
| M3 | 1.05 ± 0.04 |
| M4 | 1.05 ± 0.03 |
| M5 | 1.04 ± 0.01 |
| M6 | 1.04 ± 0.03 |
| M7 | 1.04 ± 0.00 |
| M8 | 1.10 ± 0.00 |

Every translucent metal, flat or mapped, now sits with the opaque metal
and the translucent non-metal. Calib-PBRSkin's P8 (Gladio armour,
PBRSkin, metal 0.75, t 0.85, dual lobe, top coat) 1.08 → 1.09 (53 px box),
the rest of the scene unchanged.

## Export diff

Old (HEAD) vs new `project.ocs` (`orbx_ocs get`) over the cached dumps
with a scene in My Library (453; dumps of an older format skipped): only
Calib-PBRSkin, Calib-PBRSkin-MetalTL and private scenes (the teens
family and two others): store materials with a metallic map at Diffuse
Roughness > 0, the F(σ) bake only, no new Mix; not rendered (the teens
scenes froze the Mac).

## Process

- Decomposing a fitted constant can show the decomposition's premise is
  wrong: the per-texel rule was exact for its model, and its render
  exposed the model (lost translucency). The variants that vary the
  dominant part (w 0.3, 0.16 vs 0.75) decided it; #109 only had w 0.75
  translucent metals, where the error hides under the metal.
- When Iray's structure is a weighted layer and Octane can't hold both
  parts in one node, a Mix material reproduces the structure itself -
  no factor to derive.
