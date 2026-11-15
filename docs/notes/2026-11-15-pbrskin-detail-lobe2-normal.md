# 2026-11-15 PBRSkin Detail Normal Map: lobe 2's normal (#140)

Resume of #140 after its second Iray batch: `Calib-PBRSkin-Detail` (Post
Denoiser on@8) and `Calib-PBRSkin-Detail-NoDN` (off, fresh DAZ session),
both 5000 samples on the Iray Server (c39209c, dff82c9). The 2026-11-14
compare had O8's dimples tiling and phasing as Iray's but ~25 - 30 %
stronger in Octane; the candidates were the tiles (dimples a few px
across), the weight blend, the PBRSkin lobe and Iray's denoiser. Variant
enhanced-final, 2000 samples; the -NoDN scene against Octane's raw pass,
the other against its denoised pass.

## Octane side re-rendered first

The export moved since the Octane side was rendered (#166 Top Coat normal,
#158 occlusion weight map, #163 translucency, #117 architectural). None
touches these spheres (no top coat, no occlusion, no translucency): the
re-render's numbers equal the 2026-11-14 ones to ±0.02.

## Measure

Per sphere (discs projected from the dump, `sphere_discs.py`), inside
0.85 R: band-pass detail = std / mean of luminance minus its Gaussian
blur at σ 2 px; its correlation with Iray's at zero shift; mean level;
spread p99 / median; and the detail inside Iray's top 20 % (the
highlight, by Iray's σ 3 luminance).

## The denoiser: out

Iray's two references are the same image: at most 4 / 255 apart on 1.6 %
of the pixels, every per-sphere number equal to the third decimal. Either
the Iray Server doesn't apply the Post Denoiser or at 5000 samples it
moves nothing (#171 to decide on a low-sample pair). Either way Iray's
denoiser isn't in the gap, and Octane's raw and denoised passes give the
same ratios (±0.03, D5 ±0.13).

## Current export against Iray (before the fix)

-NoDN, Octane raw (the denoised pair gives the same to ±0.03):

| sphere | detail Iray | detail Octane | O / I | corr | spread I / O | highlight detail O / I |
|---|---|---|---|---|---|---|
| D1 rough 0.25 no map | 0.513 | 0.519 | 1.01 | 0.95 | 7.66 / 8.46 | 1.02 |
| D2 4 × 4 w 0.5 (= O8) | 0.478 | 0.552 | 1.15 | 0.88 | 5.36 / 6.14 | 1.15 |
| D3 1 × 1 w 0.5 | 0.853 | 0.834 | 0.98 | 0.95 | 9.82 / 10.57 | 0.97 |
| D4 2 × 2 w 0.5 | 0.642 | 0.756 | 1.18 | 0.93 | 7.98 / 9.48 | 1.14 |
| D5 8 × 8 w 0.5 | 0.195 | 0.389 | 2.00 | 0.65 | 4.37 / 5.32 | 1.95 |
| D6 4 × 4 w 1 | 0.448 | 0.549 | 1.23 | 0.81 | 3.99 / 4.75 | 1.07 |
| D7 1 × 1 w 1 | 0.650 | 0.787 | 1.21 | 0.86 | 7.89 / 9.73 | 1.15 |
| D8 4 × 4 w 0.5 mult 1 | 0.472 | 0.698 | 1.48 | 0.85 | 6.47 / 10.13 | 1.32 |

None of the resume's outcomes fits cleanly: the ratio grows with the
tiles (b) but D3 → D7 also grows with the weight (c), and D8 (d) is the
largest. The correlation, not the ratio, gave it away: it falls with the
tiles (0.95 → 0.65), i.e. Octane has something Iray hasn't, not the same
thing stronger.

## Looked at, not measured: one glint too many

Side by side at 8 × (highlight region of D2, D6, D8) the dimples match
pixel for pixel, but Octane has one extra sharp glint, 2 - 3 px, exactly
where the smooth sphere D1 has its sun highlight - on every mapped
sphere, none in Iray. A component that doesn't see the normal map.

`project.ocs`: the PBRSkin dual lobe goes on Universal with lobe 1 on
`specular` / `roughness` and lobe 2 on `coating` / `coatingRoughness`
(0.15, roughness 0.1); the OSL detail normal is on `normal` only. The
manual: without its own Coating Normal / Bump the coating uses the plain
shading normal. So lobe 2 was smooth.

What Iray does (DAZ's MDL, no fit):

- PBRSkin `pbr_material`: `dual_specular_bsdf` = lobe 2 `weighted_layer`
  over lobe 1 with `normal: state::normal()`, and that pair is the
  `layer` of the `custom_curve_layer` with `normal: normal` (the
  `pbr_detail_normal` output). The MDL spec only says a layerer's normal
  applies to its top layer; the render settles the nested default: no
  smooth glint in Iray, so lobe 2 sees the detail normal.
- Uber `add_dual_lobe_specular`: both lobes in one `weighted_layer` on
  `layer_bump` = `combined_bump`. Same normal for both.

## Fix

Universal's lobe 2 coating takes the base's normal pins
(`base_detail_pins` on `coatingNormal` / `coatingBump`: the OSL detail
normal on PBRSkin, the baked or plain normal map / height map on Uber),
in both writers (`universal` and `skin_gloss`). Test
`dual_lobe_coating_takes_the_base_normal`.

## After the fix

-NoDN, Octane raw (denoised within ±0.03, D5 1.35):

| sphere | detail O / I | corr | spread I / O | highlight detail O / I |
|---|---|---|---|---|
| D1 | 1.01 | 0.95 | 7.66 / 8.47 | 1.01 |
| D2 | 1.25 | **0.98** | 5.36 / 6.01 | 1.26 |
| D3 | 1.14 | 0.97 | 9.82 / 11.06 | 1.13 |
| D4 | 1.32 | **0.98** | 7.98 / 9.60 | 1.31 |
| D5 | 1.63 | **0.97** | 4.37 / 5.29 | 1.59 |
| D6 | 1.24 | 0.96 | 3.99 / 4.93 | 1.23 |
| D7 | 1.34 | 0.95 | 7.89 / 9.83 | 1.30 |
| D8 | 1.59 | **0.98** | 6.47 / 10.33 | 1.44 |

The glint is gone and the pattern is Iray's: correlation 0.95 - 0.98 on
every sphere (D5 0.65 → 0.97). What's left is amplitude, not shape: the
dimples read 1.14 - 1.6 × Iray's contrast. That was hidden before - the
smooth lobe 2 diluted the mapped part. The smooth D1 is already 10 %
sharper in Octane (spread 8.47 vs 7.66), the dual lobe at roughness 0.4
(Calib-PBRSkin-Occlusion O1) 15 %; a sharper lobe turns the same tilt into
more contrast. Not fitted (#57): split by layer next.

Calib-PBRSkin-Occlusion (re-dumped: its cached dump no longer matched
the library scene), denoised, same measure: O8 correlation 0.87 → 0.98,
contrast 1.32 → 1.43 × Iray; O1 - O7 unchanged (no normal map; O1 1.14,
spread 6.83 vs 5.86).

## Next: Calib-PBRSkin-DetailLayers (parked for Iray)

`make-calib-scenes.py`: the dimples at 1 × 1, weight 1, mult 1 (no
footprint, no blend, the lobe's roughness as without the map), each
layer alone beside its no-map twin: E1 / E2 the diffuse alone (the tilt
itself, no lobe), E3 / E4 lobe 1 alone (ratio 0), E5 / E6 lobe 2 alone
(ratio 1), E7 / E8 the dual lobe under a Top Coat (weight 1, roughness
0.05; #166's check that the coat carries the dimples). Octane side
rendered (enhanced-final, 2000 samples): detail E1 0.010 / E2 0.099, E3
0.084 / E4 0.477, E5 0.771 / E6 0.829, E7 0.366 / E8 0.895.

What it decides: E2 ≈ Iray -> the tilt is right and the excess is the
lobes' response (then E4 / E6 say which lobe, against the Phong pin's
docs, no fit); E2 already > Iray -> the normal itself is stronger in
Octane than Iray's `tangent_space_normal_texture` makes it, back to the
map decode / tangent frame. E8 vs E7: the coat's highlight broken by the
dimples as in Iray.

Timings: Octane 2000 samples ~2.5 min a scene; DAZ dump 3 s.
