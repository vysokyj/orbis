# 2026-11-11 Translucent weighted dual lobe (#128)

Goal: a weighted material (Base Mixing 2) with translucency kept its gloss
on `specular` and dropped the dual lobe, although DAZ's `normalized_base`
has the lobe in `dl_diffuse_bsdf` there too. Found during #121, which kept
it only for the Mix path (no translucency).

## MDL (docs first)
`irayubermaterial.mdl`, `normalized_base`: `diffuse_material` is
`diffuse_base` with the translucency (a `weighted_layer` of the diffuse at
1 − Translucency Weight over the diffuse transmission); `dl_diffuse_bsdf`
lays the dual lobe over that whenever `dual_lobe_uniform_specular_weight`
> 0; `normalized_mix` of `dl_diffuse_bsdf` at Diffuse Weight and the plain
glossy at Glossy Weight. Translucency changes nothing in that structure:
the lobe is there with or without it.

## Choice
The translucent weighted path (mesh hair) doesn't take the Mix: its
diffuse map is brightened by `weighted_mix` (fitted on a private hair,
probe 59) and the gloss sits on `specular`. Routing it through the Mix
would change that hair look with no reference to check it, so the path
stays and only gains the lobe, in the layout #106 already has for a gloss
over a dual lobe: the dual lobe on `specular` (full share, both lobes at
the ratio's roughness blend), the gloss as the `coating` — now at Glossy
Weight (`glossy_coating` read Glossy Layered Weight; it follows
`gloss_weight_channel` now). A Top Coat moves into the Layered material's
Specular Layer as there (`layered_top_coat`). Universal can't mix the gloss
beside the lobe on one node; the coating over the lobe is the nearest it
has, and Iray decides.

Not done: a dual lobe on a translucent weighted material without gloss
already took the dual-lobe branch (lobe 1 `specular`, lobe 2 coating).

## Scene: Calib-DualLobeWeightedTL
Calib-DualLobeWeighted's spheres (red, DLW: Diffuse 1, Glossy 0.5 at
roughness 0.6, dual lobe W 1 ratio 0.5 r2 0.1) with Translucency Weight
0.5 in an orange: T1 gloss only, T2 dual lobe only, T3 both, T4
translucency 1, T5 Top Coat, T6 Diffuse 0.5 / Glossy 0.3, T7 no
translucency (the Mix, W3's twin), T8 Glossy Reflectivity 0.

First write added an Uber channel that doesn't exist ("Translucency
Enable", PBRSkin's) — removed and regenerated before anything ran on it.

`.mi` → fixture `calib/references/mi/Calib-DualLobeWeightedTL.json`;
`src/iray.rs` agrees unchanged. DAZ hands the dual lobe (uniform weight 1,
roughness 0.09 / 0.01, ratio 0.5), glossy_weight and the translucency on
every translucent sphere: the lobe is real in Iray. T8 and T3 get the same
arguments — no `glossy_reflectivity` under Base Mixing 2, while the Octane
side scales that gloss by it (#132).

Octane side (enhanced-final, 400 samples): rendered, 35 s. T7 (the Mix's
white metal share) stands far apart from T3; the Iray reference says
whether the translucent path's dielectric gloss or the Mix is the closer.

## Open
Iray batch in #128 (5000 samples). #132 (Glossy Reflectivity under
Base Mixing 2) is decided by T8 vs T3.
