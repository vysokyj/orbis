# 2026-11-11 Diffuse-only factors under a metal (#109)

Goal: Universal's metal takes its colour from `albedo`, which also carried
two factors that belong to Iray's diffuse alone - the head-on Oren-Nayar
F(σ) (`base_flat`) and the translucent metal's dimming (`dimmed`,
`METAL_TRANSLUCENCY_LOSS` 0.35 × Translucency Weight). Both dimmed the
metal's reflection (to 0.78 at Diffuse Roughness 1). Found by code review.

Variant: enhanced-final. Measure: Octane / Iray linear luminance over the
middle of each sphere (±26 px at 1920 × 1080, the spheres found from the
dump's transforms, M1 … M8 left to right; `calib-compare-all.py`'s boxes
come from Calib-Glossy's red spheres and didn't sit on this scene's).

## Layer stack (no render)

`pbr_skin.mdl`: the metal is a `weighted_layer` at Metallicity of
`directional_factor(basecolor) × dual_specular` over the base; the
translucency is in the base only (`diffuse_translucency_bsdf`, the diffuse
at 1 − t over the diffuse transmission). Iray Uber's
`metallic_layered_base` likewise lies over the diffuse. So the metal keeps
its full colour and only the diffuse takes F(σ) and the translucency.

Universal: m' A metal, (1 − m') A diffuse, (1 − m') × specular gloss. For
Iray's M metal, B d diffuse, B gloss (M = m k, B = (1 − m) k from
`metal_shares`, d the diffuse-only factor): S = M + B d, m' = M / S,
albedo C d × S / d, specular × S / d - exact for a flat metal
(`metal_over_diffuse`). At d = 1 it is the old (m, k).

## Calib-PBRSkin-MetalTL

| sphere | before | metal kept, 0.35 t | metal kept, t | final |
|---|---|---|---|---|
| M1 metal 0.75 | 1.05 | 1.05 | 1.05 | 1.05 |
| M2 metal 0.75 tl 0.85 | 1.02 | 1.11 | 1.03 | 1.03 |
| M3 metal 0.75 tl 0.5 | 1.03 | 1.08 | 1.04 | 1.04 |
| M4 metal 0.3 tl 0.85 | 1.13 | 1.17 | 0.97 | 0.97 |
| M5 metal 0 tl 0.85 (transmission) | 1.03 | 1.03 | 1.03 | 1.03 |
| M6 white map tl 0.85 | 1.01 | 1.10 | 1.03 | 1.01 |
| M7 grey map tl 0.85 | 1.12 | 1.23 | 1.19 | 1.12 |
| M8 grey map | 1.07 | 1.07 | 1.07 | 1.07 |

- With the metal kept, the fitted 0.35 left the translucent metals 6 %
  over the opaque one (M1): it had been fitted while the dimming hit the
  metal too, and stood in for the metal's missing light.
- All of the translucency's share taken (d = 1 − t, what the MDL's weights
  say when nothing comes back out of the volume under the metal) brings the
  flat metals to M1's level. M4 (little metal) at 0.97: maybe a little
  light does come back through the 70 % base; within the scene's spread,
  not fitted.
- Maps: the metal follows the map per texel, the albedo scale doesn't. M7
  (a uniform grey map) rose to 1.19 - S / d is taken at the channel's
  0.75 while the map leaves about 0.16; on a mask map (armour) the
  unmetalled texels would come out × S / d (1.75 × at t 0.5). So under a
  metallic map the old rule stays: the whole albedo × (1 − 0.35 t), its
  metal dimmed with the diffuse (`METAL_MAP_TRANSLUCENCY_LOSS`). A
  per-texel albedo (a Mix by the map) would fix both: #123.

Export diff (`orbx_ocs get`, old vs new `project.ocs`, all 432 cached
dumps in My Library): only Calib-PBRSkin and -MetalTL change. Calib-PBRSkin's
P8 (Gladio armour: metal 0.75, translucency 0.85, dual lobe, top coat):
`metallic` 0.75 → 0.95, albedo 0.23 → 0.26, dual lobe `specular` 0.425 →
2.23; rendered 1.00 → 1.04 × Iray beside the opaque metal P7's 1.05, the
rest of the scene unchanged (1.04 – 1.06). Calib-Map-Metal unchanged.

## Also

The dual lobe's `specular` now takes the metal's base scale as the glossy
layer's always did (the spec's "albedo and specular × k"); without it the
translucent PBRSkin metals' dual lobe would have been dimmed by d / S.

## Process

- The diffuse-only factor only showed as a bug once the layer stack was
  read: the 0.35 fit had absorbed it. Fits made on a whole sphere hide
  exactly this; decomposing (M1 alone, M5 alone) would have shown the
  metal's share.
- An export diff (`orbx_ocs get` of old vs new `project.ocs` over every
  cached dump) is a cheap check that a fix touches only what it should.
