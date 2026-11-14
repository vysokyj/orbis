# 2026-11-14 PBRSkin Detail Normal Map: Iray compare (#140)

Resume of #140 after its Iray batch (reference `calib/references/iray/
Calib-PBRSkin-Occlusion.png`, 5000 samples, denoiser on@8, rendered at
23:33 before the queue's `pbr_skin` clash of 00:00 - a real PBRSkin, as
#54's note checked). Sphere O8: the dimples normal map
(`calib-coatbump-dimples-n.png`, 512², 8 × 8 sine periods a tile) at 4 × 4
tiles, Detail Weight 0.5, Detail Specular Roughness Mult 0.25.

## Octane side

Re-rendered with the current export (after 8350679 #87 and 5791263 #114,
neither touching skin; enhanced-final, 400 samples): the numbers are the
2026-11-14 00:48 render's to the decimal. At 2000 samples (the -NoDN run
re-rendered the whole variant folder) O8's detail is 0.98 (1.30 × Iray's),
O1 / O7 unchanged: the excess isn't grain.

## Tiling, phase, level

Band-pass detail (luminance minus its Gaussian blur at σ 2 px, std / mean
over 0.85 R) and its correlation with Iray's:

| sphere | detail Iray | detail Octane | Octane / Iray | correlation (best shift) |
|---|---|---|---|---|
| O1 dual lobe | 0.206 | 0.230 | 1.12 | 0.98 (0, 0) |
| O7 detail rough 0.5 | 0.841 | 0.842 | 1.00 | 0.93 (0, 0) |
| O8 detail normal | 0.757 | 0.959 | 1.27 | 0.89 (0, 0) |

- Tiling and phase match: the dimples correlate 0.89 at zero shift (no
  mirror, no half-period offset; a ±3 px search finds nothing better).
- Level: O8's mean / Iray 1.23 against O1's 1.18 (1.04 relative).
- The dimples are **~25 % stronger in Octane**, the same in R, G and B
  (1.25 / 1.25 / 1.23), most in the highlight: by Iray luminance bin
  1.32 / 1.20 on the dim half, 1.68 / 1.45 in the top 20 %.
- Spread (p99 / median, upper disc) relative to O1, after a Gaussian blur
  of the whole image: σ 0 1.24, σ 1 1.12, σ 3 1.13, σ 5 1.08. Octane's raw
  pass equals its denoised one (13.4 / 13.1), so Octane's denoiser isn't
  in it. The occlusion spheres O4 / O5 scatter by 1.11 - 1.13 at σ 3 too,
  so the envelope alone wouldn't separate O8; the dimple detail does.

## What the formula leaves (read, no fit)

The resume said to check the factor and the weight blend first.
NVIDIA's open `base.mdl` (MDL SDK, `src/shaders/mdl/base/base.mdl`):

- `tangent_space_normal_texture`: (lookup - 0.5) × 2 factor, then
  normalize(tu x + tv y + n (|z| + 1 - factor)); O8's `.mi` factor is 1
  (the -1 in the fixture is `inverted_normal`, the transmission side).
- `transform_coordinate` with scale 4 re-orthonormalises the uvw tangents:
  the tiles don't scale the tilt.
- `blend_normals`: each normal projected on `texture_tangent_u / v(0)` and
  `state::normal()`, weighted (w x, w y, w z + 1 - w), normalised, then
  reoriented normal mapping - exactly `detail_normal_osl` (and
  `detail_normal::blend_normals`).

So Octane's input is Iray's formula, and Octane renders it as intended
(2026-11-13: the OSL's shading normal = the same map baked by hand, 8.61°
vs 8.62°).

## Controls

- **Calib-CoatBump B5** (the same dimples map as an Uber Top Coat normal
  map, 1 × 1 tile, re-rendered now): detail 0.946 vs Iray 0.897 (1.05),
  correlation 0.97; B6 (ridges, base normal) 1.00. The plain normal-map
  path agrees with Iray at 1 × 1.
- O8 differs from B5 in the tiles (dimples 4 × smaller: ~5 px a half
  period at the disc's centre, 1 - 2 px at the rim; ~7 texels a pixel), the
  weight blend, the PBRSkin dual lobe at roughness 0.25, and Iray's AI
  denoiser on a pattern near pixel scale. Mip filtering can't make 25 %: a
  64-texel period under an 8-texel box keeps 0.97.

## New scenes (parked for Iray)

`Calib-PBRSkin-Detail` (`make-calib-scenes.py`, O8's red dual lobe):
D1 no map at O8's roughness (O8's own baseline, which O7 isn't: O7 is mult
0.5 at weight 1 = roughness 0.2, O8 0.25), D2 = O8, D3 / D4 / D5 tiles 1 / 2
/ 8, D6 / D7 weight 1 at 4 × 4 / 1 × 1 with mult 0.625 (lobe roughness as
D2's), D8 the normal alone (mult 1). `Calib-PBRSkin-Detail-NoDN`: the same
with Iray's Post Denoiser off. Octane side rendered (enhanced-final, 400
samples, then both at 2000; raw pass for the -NoDN compare). Octane
alone: detail D3 0.91, D4 0.83, D2 0.62, D5 0.43 - the amplitude falls with
the tiles as the dimples go below a few pixels. D2 (= O8's material) reads
0.62 where O8 reads 0.98: each position sees its own sky, so only
Octane / Iray at the same position counts.

What each decides:
- D2 vs D2-NoDN in Iray: the denoiser's share of the 25 %.
- D3 / D4 / D2 / D5 Octane / Iray: does the gap grow with the tiles
  (footprint / sampling) or stay (mapping)?
- D6 / D7 against D2 / D3: the weight blend (Octane's tilt halves at w 0.5).
- D8 against D2: the roughness detail's part.

## Also found

- DAZ wires PBRSkin's `pbr_add_clearcoat.top_coat_base_bump` to the same
  `pbr_detail_normal` call as the base normal: the Top Coat takes the
  base normal map and the detail in Iray, against the spec's sentence.
  No fixture has a top coat over a normal map: #166.
- Calib-CoatBump's cached dump is keyed by the library path: the
  `calib/scenes` copy finds none (#159's case).

Timings: Octane 400 samples ~1 min a scene, 2000 samples ~3 min; DAZ dump
5 - 6 s a scene.
