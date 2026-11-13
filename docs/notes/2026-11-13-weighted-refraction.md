# 2026-11-13 Refraction under weighted mixing (#154)

Goal: #143 left Base Mixing 2 (Weighted) with gloss and refraction: the
export routed it through the weighted Mix with the refraction in the
diffuse part (no reflection) and a mirror-like metal at the full gloss
share. The issue guessed that at Refraction Weight 1 the diffuse goes off
and Iray shows plain glass. MDL first, then DAZ's `.mi`, then the Octane
side; Iray parked.

## Docs first (irayubermaterial.mdl, `normalized_base`)

- `refraction_bsdf = refraction_base(..., true, true, 1, combined_bump)`:
  reflection on and full, glossy layered weight 1. With shared inputs and
  no dispersion `split_bsdf` is false, so it is one
  `scatter_reflect_transmit` BSDF in the glossy colour at the glossy
  roughness - a plain glass with its Fresnel split at the material IOR
  (the Refraction Index). Unshared: `fresnel_layer` of the glossy reflection
  over the transmission in Refraction Color.
- `glossy_base_bsdf = weighted_layer(refraction_weight, refraction,
  backscatter-over-glossy)`.
- `base_bsdf = diffuse_enable && gloss_mix_used ? normalized_mix(Diffuse
  Weight x dl_diffuse, Glossy Weight x glossy_base) : (gloss_mix_used ?
  glossy_base : dl_diffuse)`.

So the whole question is which `*_enable` flags DAZ sets.

## The `.mi` (Calib-WeightedRefr, -Shown, -Thin, Calib-Mi-WeightedRefr2, -Shown, -3)

Red base, IOR 1.5, Glossy Roughness 0.1, closed unless -Thin. One headless
DAZ run for the first five scenes, one for -3; `export-mi.done` clean, the
-Shown twins match their scenes (node names aside).

| Sphere | Channels | DAZ hands Iray |
|--------|----------|----------------|
| W1 | r 1, g 0.5, d 1 | diffuse **on**, glossy off, refraction 1 |
| W2 | r 1, g 1 | the same |
| W3 | r 1, g 0.5, d 0 | diffuse off, glossy off |
| W4 / W5 | r 0.5, g 0.5 / 1 | diffuse on, glossy on |
| W6 | r 0.5, d 0 | diffuse off, glossy on |
| W7 | r 1, tinted gloss | as W1, `glossy_color` the tint |
| W8 | r 1, unshared | `iray_uber`, `refraction_color`, own roughness |
| V1 / V2 | dual lobe | passed, as W1 / W4 |
| V3 / V4 | metal 0.7 | no metal (Base Mixing 2) |
| V5 | r 1, backscatter 0.5 | backscattering **off** |
| V7 | r 0.5, d 0.3, g 0.4 | weights as set |
| U1 / U2 | r 0, d 0, g 0.5 / 1 | diffuse off |
| U4 | r 0, backscatter 1 | diffuse on, glossy off |
| U5 / U6 | Base Mixing 0 / 1, r 1, backscatter 0.5 | backscattering off |
| U8 | r 1, g 0, d 0 | no refraction (#143), diffuse off |

`src/iray.rs` disagreed in 23 arguments (`diffuse_enable` on W1 / W2 / W6 /
W7 / W8 / V1 / V3 / V5 per scene, `backscattering_enable` on V5). Fixed:
under weighted mixing `diffuse_enable` = Diffuse Weight > 0, `glossy_enable`
off under a full refraction or backscattering (the `weighted_layer` at 1
covers it); `backscattering_enable` off under a full refraction in every
mode. `mi-audit.py` 271 checked, 0 disagree.

So the issue's guess was wrong: at Refraction Weight 1 the diffuse stays,
and W1 is 2/3 diffuse + 1/3 glass in Iray; only Diffuse Weight 0 gives
plain glass. And U1: at Diffuse Weight 0 the gloss part renders alone at
full strength - `WeightedGloss::of(0, g)` had scaled it by g.

## The Octane side

`weighted_mix_material` now follows the MDL's tree, no constant:
- `material1` (the gloss part, at the same share and scale as before): the
  metal mirror (no refraction), the glass alone (refraction 1: Universal at
  refraction 1 with the gloss at full weight - `transmission` and
  `specular` in the glossy colour x the gloss scale, the Refraction Index,
  no albedo, no dual lobe, no Glossy Weight map; `weighted_refraction`),
  or a Mix at the Refraction Weight (its map) of the glass and the metal.
- `material2` (the diffuse part): `refraction_override` 0 - diffuse whole,
  its translucency (a full refraction had dropped it), its dual lobe.
- Diffuse Weight 0: `amount` 1, scale 1.

Unit tests: the three shapes, displacement once on the outermost node
(two new cases).

Octane side rendered (enhanced-final) for Calib-WeightedRefr and -Thin;
Iray parked (Uber only, no preset): see #154's `## Iray batch`.

## Process

- The `.mi` contradicted the MDL-based guess in the issue at once
  (diffuse stays on at a full refraction): DAZ's enable flags decide the
  MDL's branch, so the `.mi` comes before any structure is built.
- A second small `.mi` scene (-3) answered the general rule (Diffuse
  Weight 0 without refraction, backscattering in modes 0 / 1) for one more
  minute of headless DAZ - cheaper than generalising from refraction
  spheres only.
