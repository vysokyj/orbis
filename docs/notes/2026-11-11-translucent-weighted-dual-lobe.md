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

## Resume: compare with the Iray reference (4e8780f)
The Octane render (16:08) predated 900080c and 8efab8b; the scene's dump
no longer matched (export refused), so `daz-dump.sh` again (library scene
= `calib/scenes` copy, sha1 equal), enhanced-final at 400 samples, 33 s.
Measured per sphere where it stands (the #106 / #121 lesson), mean linear
luminance (gamma 2.2) over the upper disc r < 0.9 R, centre r < 0.45 R,
rim 0.65 - 0.9 R, "hi" Iray's top 3 %, discs from Calib-Glossy's Iray.

| sphere | Iray | 5c24564 | centre | hi | G / R I / O | as the Mix | centre | rim | hi | spread O / I | G / R O |
|---|---|---|---|---|---|---|---|---|---|---|---|
| T1 gloss only | 0.232 | 0.27 | 0.15 | 0.12 | 0.860 / 0.261 | 1.03 | 1.06 | 1.01 | 1.10 | 3.0 / 2.7 | 0.817 |
| T2 dual lobe only | 0.067 | 1.00 | 1.00 | 0.95 | 0.321 / 0.282 | 1.00 | 1.01 | 0.99 | 0.94 | 2.2 / 2.5 | 0.281 |
| T3 both | 0.287 | 0.29 | 0.23 | 0.19 | 0.915 / 0.389 | 1.05 | 1.08 | 1.02 | 1.11 | 2.5 / 2.3 | 0.902 |
| T4 translucency 1 | 0.360 | 0.25 | 0.22 | 0.20 | 0.855 / 0.416 | 0.99 | 1.04 | 0.93 | 1.06 | 2.3 / 2.0 | 0.927 |
| T5 Top Coat | 0.305 | 0.41 | 0.36 | 0.30 | 0.981 / 0.623 | 1.09 | 1.11 | 1.07 | 1.12 | 2.3 / 2.2 | 0.972 |
| T6 D 0.5 / G 0.3 | 0.287 | 0.30 | 0.25 | 0.23 | 0.999 / 0.447 | 1.08 | 1.10 | 1.07 | 1.12 | 2.5 / 2.4 | 0.978 |
| T7 no translucency | 0.545 | 1.10 | 1.11 | 1.07 | 0.976 / 0.988 | 1.11 | 1.12 | 1.10 | 1.07 | 1.7 / 1.8 | 0.988 |
| T8 Glossy Refl 0 | 0.359 | 0.30 | 0.25 | 0.33 | 0.966 / 0.595 | 1.08 | 1.09 | 1.06 | 1.12 | 2.8 / 2.6 | 0.987 |

Decomposition (Resume rule: T3 far off, T7 matching -> T1 / T2 first):
T2, the lobe alone on the unchanged path, is 1.00 - with Calib-DualLobe
(1.05 - 1.18) and Calib-DualLobeWeighted W2 (1.12): the lobe was fine.
T1, the gloss alone with no lobe (the translucent path as it was before
5c24564, untouched by it), is 0.27 and red: the gloss itself is what is
lost, and every sphere carrying it (T3 - T6, T8) follows T1, whatever the
lobe. The MDL says why (`normalized_base`, irayubermaterial.mdl): the
gloss is `basic_glossy_bsdf`, a full-strength glossy reflection tinted
Glossy Color with no Fresnel, in `normalized_mix` against `dl_diffuse_bsdf`
at Diffuse / Glossy Weight; translucency only changes `diffuse_material`
(`diffuse_base`'s weighted layer), not that mix. As a 4 % dielectric on
`specular` the export kept about a tenth of it - the same miss #10 found
for the opaque weighted sphere, which is why the Mix exists. Nothing to
fit: the translucent weighted path takes the Mix too.

Change: `weighted_gloss` no longer refuses translucency; the lobe goes
into the Mix's diffuse part (#121's layout), `gloss_over_dual_lobe` is
Base Mixing 0 only again. A translucent diffuse map inside the Mix is
brightened by `weighted_mix` from the diffuse and translucency shares
alone: with the gloss's share in the map too, the gloss would count twice
(the hair test's factor 2.8 -> 2.0). The translucency term itself is
#131's (left as it was).

After: every sphere 0.99 - 1.11, the W twins of Calib-DualLobeWeighted
alongside (T1 1.03 / W1 1.05, T3 1.05 / W3 1.07, T6 1.08 / W5 1.08, T5
1.09 / W6 1.11), spreads within 0.3, colour within 0.04 except T4
(translucency 1: Iray's 0.855 vs 0.927 - Octane has no transmission on
this path, `transmit` 0 under Base Mixing 2; #131). #132: T8 1.08 = T3
1.05 within the noise, and the export hands them the same now.

## Calib-WeightedTL / -Thin (#131's scenes) with the change
Rendered beside it to see the gloss there (refs 4e8780f, enhanced-final,
the same regions):

| sphere | Iray | before: mean | hi | spread | after: mean | centre | rim | hi | spread O / I |
|---|---|---|---|---|---|---|---|---|---|
| W1 diffuse map | 0.105 | 1.06 | 1.04 | 1.6 | 1.06 | 1.06 | 1.05 | 1.04 | 1.6 / 1.6 |
| W3 tl 0.5 | 0.067 | 2.41 | 2.60 | 1.2 | 2.42 | 2.48 | 2.31 | 2.61 | 1.2 / 1.1 |
| W5 tl 1 | 0.069 | 2.58 | 2.54 | 1.3 | 2.58 | 2.66 | 2.40 | 2.53 | 1.3 / 1.0 |
| W6 tl 0.5 gloss 0.5 | 0.288 | 0.90 | 0.38 | 1.3 | 1.22 | 1.17 | 1.31 | 1.05 | 3.2 / 4.0 |
| W7 tl 0.5 D 0.5 G 0.3 | 0.472 | 0.62 | 0.35 | 1.3 | 1.14 | 1.07 | 1.28 | 1.00 | 2.3 / 2.7 |
| W8 hair maps | 0.125 | 1.02 | 0.39 | 1.6 | 1.41 | 1.31 | 1.57 | 1.19 | 4.0 / 5.4 |

(-Thin within 0.01 of the solid.) The highlights are Iray's now; what is
left is the diffuse part's translucency brightening (W3 - W5 2.4 - 2.6 x
with no gloss at all, the rims 1.3 - 1.6), #131's own question. W8's
old 1.02 was two errors cancelling (highlight 0.39, rim 1.48); it is 1.41
until #131 settles the translucency term - the mesh hair (private, the
x3 fit's source) will look brighter until then.

## Outcome
#128 closed: the dual lobe and the gloss both reach Octane on translucent
weighted materials, nothing fitted. #131 resumes on this export.
