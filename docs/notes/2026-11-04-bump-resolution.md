# 2026-11-04 — Bump vs the height map's resolution (#75)

Goal: #61 fitted Octane's bump `power` to 2 × Iray's `file_bump_texture`
factor on Calib-CoatBump's 512 px maps. Does Iray's bump depend on the
map's resolution (and on UV tiling, object size) where Octane's doesn't?

## Iray's side: MDL's reference code

NVIDIA's MDL docs (`docs/nvidia/txt/mdl-base-module.txt`) say only
"degree of bumpiness". The open-source MDL SDK carries the reference
implementation of the base module (`src/shaders/mdl/base/base.mdl`, BSD):

- `file_bump_texture` scales the UV position by the texture's size in
  texels and takes the height difference of the texels one to the
  right / above and one to the left / below (smootherstep-interpolated
  bilinear lookups), per axis — a central difference over two texels,
  not divided by anything;
- `apply_bump` adds `tangent_u × du × (−factor) + tangent_v × dv ×
  (−factor)` to the normal and normalises; the tangents are unit vectors
  (`transform_coordinate`, which DAZ uses for the UV tiles, re-normalises
  them).

So Iray's tilt is tan θ ≈ factor × 2 × ∂h/∂u / width: **4x the pixels =
4x flatter**, and neither the tiling nor the object's size changes it. It
fits #61's hint (per-texel normal maps rendered flatter than Iray, ×2
would fit). Iray's own `file_bump_texture` is native, so this is the
reference, not Iray's binary — the Iray render of Calib-BumpRes confirms
it.

DAZ's `.mi` of the new scene: `top_coat_bump = file_bump_texture(factor
= Top Coat Bump)`, the tiles as `base::transform_coordinate` (scale 2 / 4
on the diagonal) on its `uvw`; the textures named `…c_m512-h1024` (DAZ's
Iray texture compression thresholds — compression keeps the resolution,
left aside). Fixture `calib/references/mi/Calib-BumpRes.json`; `src/iray.rs`
passes it unchanged.

## Scene: Calib-BumpRes

`make-calib-scenes.py`: Calib-CoatBump's coat (Fresnel IOR 1.5, roughness
0.05, matte red base) with the same 8 × 8 sine dimples as a 16-bit height
map (8 bits would step at 4096 px: ~3 levels per texel pair) at 256,
1024 and 4096 px, `map_gamma` 1, on Top Coat Bump (height mode):

| sphere | map | factor | | Iray (rule) vs R2 |
|---|---|---|---|---|
| R1 | 256 | 4 | | 4x |
| R2 | 1024 | 4 | | 1 |
| R3 | 4096 | 4 | | 1/4 |
| R4 | 4096 | 16 | | 1 |
| R5 | 256 | 1 | | 1 |
| T1 | 1024 | 4 | tiles 2 | 1 (finer) |
| T2 | 256 | 1 | tiles 4 | 1 (finer) |
| S1 | 1024 | 4 | sphere at half size | 1 |

Opened and dumped headless (5 s), `.mi` exported.

## Octane's side

Renders (enhanced-final, 800 samples, ~60 s each) with every coat bump
power × 0 / 0.5 / 1 / 2: by eye R1 / R2 / R3 alike at each power; T1, T2
and S1 bumpier. The RMS of the grey image against the smooth render
doesn't rank bump strength (it saturates and isn't monotonic: R2 8.7 /
12.0 / 10.7), and comparing S1 with a downscaled R2 failed on alignment —
dead ends.

Instead, the normals themselves: `scripts/dev/octane-probe-bumptilt.lua`
swaps in the Info channel kernel (type 1, shading normal, unfiltered, 16
samples) and renders it without and with "Bump and normal mapping";
`scripts/dev/bump-tilt.py` takes the angle between the two per pixel, RMS
over each sphere's inner disc (radius 0.7). The info kernel reads the base
layer's `bump`, not `coatingBump`, so the height maps were moved there in
the exported `project.ocs` (a `sed`). RMS tilt (°), powers as exported
before (2 × factor) times k:

| k | R1 | R2 | R3 | R4 | R5 | T1 | T2 | S1 |
|---|---|---|---|---|---|---|---|---|
| 0.5 | 4.75 | 4.45 | 4.43 | 13.98 | 1.19 | 7.29 | 3.53 | 7.35 |
| 1 | 8.94 | 8.43 | 8.42 | 18.01 | 2.34 | 13.05 | 6.89 | 13.18 |
| 2 | 14.39 | 13.68 | 13.70 | 19.85 | 4.60 | 19.09 | 12.55 | 19.34 |

- Resolution: none (R1 / R2 / R3 at the same power alike) — as #61's
  512 vs 2048 px check.
- Tiles 2 (T1) and half size (S1): each like R2 at about twice the power
  (13.1 / 13.2 vs 13.7; 7.3 / 7.4 vs 8.4) — Octane's bump is a height per
  meter of surface. The small shortfall is the pattern being twice as
  fine on screen (both doublings show it alike, T2 at 4x finer more):
  pixel averaging, not the law.
- The tilt isn't linear in the power above ~8° (R2 4.5 / 8.4 / 13.7):
  compare at equal tilt, not by ratio.

The measure itself understates absolute tilt: the dimples' normal map
(`calib-coatbump-dimples-n`, RMS 34.5° in the file) on R1's `normal` pin
measured 16.4°. So only comparisons inside Octane; the absolute anchor
stays #61's fit against Iray.

## Rule

Octane power = factor × `BUMP_POWER` 2 × 512 / the map's width × the
material's UV scale / 0.542 m / the tiles (`bump_power` in
`src/material.rs`). UV scale = √(area / UV area) of the material's faces
(`usd::uv_scales`): Calib-CoatBump's spheres 0.282 m² over 0.958 →
0.542 m (the 4 m ground plane 4.0). Width and tiles are geometric means
of the two axes; the width is the map's as the scene has it (before the
texture budget, which Octane doesn't mind). Calib-CoatBump exports
unchanged (2.0007 / 6.002).

Calib-BumpRes exported with the rule, the same normals measure:

| | R1 | R2 | R3 | R4 | R5 | T1 | T2 | S1 |
|---|---|---|---|---|---|---|---|---|
| power | 16 | 4 | 1 | 4 | 4 | 2 | 1 | 2 |
| tilt (°) | 14.39 | 4.45 | 1.14 | 4.55 | 4.60 | 3.75 | 3.53 | 3.78 |

R2 / R4 / R5 equal, R1 / R3 the ±4x ends, T1 / T2 / S1 within the
screen-frequency shortfall above — Iray's rule, in Octane.

Effect on real scenes: a DAZ figure's 4096 px bump map now goes in at 1/8
of the power the 512 px fit gave it (times its UV scale term; a G9 body
tile is of the order of the calibration sphere's). #61's renders of the
figures were over-bumped by that.

## Open

Iray reference of Calib-BumpRes (5000 samples) — the rule is from MDL's
reference code, not Iray's binary; listed on #75 for the next Iray batch.
Done: [2026-11-05](2026-11-05-bump-iray-reference.md), confirmed, no change.
Not covered: the DAZ MDL preset materials' own bump amounts (`MdlMetal`,
`MdlDielectric`) keep their constants.

Process: the plain-render metrics were useless for bump strength; the Info
kernel's normals measure it directly and take 1 s a render.
