# 2026-11-13 — Sphere discs from the scene, not the image (#146)

Goal: one sphere finder for every compare script, right on every sphere
row in `calib/scenes`, and how much the old finders bent the numbers.
No renders: everything below reads renders already in the cache
(`~/Library/Caches/orbis/calib/octane/<variant>/`) and the Iray
references in git.

## 1. What the old finders did

Both thresholded Calib-Glossy's Iray render for red (R > 1.6 G, R > 0.12)
and grouped the columns:

- `calib-compare-all.py` (sRGB values): **9 boxes**. G4 (Reflectivity 1)
  reflects enough white to break its red in two: a 4 × 11 px sliver at
  x 846 and the rest at x 893 (r 33). Every sphere row printed nine
  numbers; columns 5 - 9 are spheres 4 - 8, column 4 a sliver of sphere
  4's left rim. Applied to every scene of the prefix list, whatever its
  own spheres looked like.
- `highlight-spread.py` (linear): 8 discs of r 14 - 24 px centred ~20 px
  right of and above the true centres (#137): the sun's highlight.
- `furnace-rings.py`, `point-highlight.py`: hard-coded spans (right to
  ±1 px, but only for that framing). `refr-volume-mc.py --measure`:
  what differs from the dome (right on furnace scenes). `gloss-split.py`
  (#137): the matte `-bare` render (right, needs a bare scene).

## 2. `scripts/dev/sphere_discs.py`

Spheres are found in the scene's **dump** (every node whose world-space
vertices lie on a sphere: ≥ 50 vertices, distance to the centroid within
2 % of its mean, equal x / y / z extents; not around the camera, in front
of it, in frame), or without a dump in the `.duf`'s own primitives
(`geometry_library` placed by translation × scale). The camera is the
`.duf`'s current camera as `src/usd/camera.rs` reads it (Euler XYZ =
Rz Ry Rx, Focal Length, Frame Width fitted to the height, the render
size). Each disc is the bounding box of the projected vertices:
off-axis spheres are ellipses, the outer ones of the row 49.0 × 45.9 px
against 46.0 × 45.9 in the middle (8 % wider). Order: rows top to
bottom, left to right. A scene without geometry falls back to detection
in a matte render; nothing ever detects on a lit glossy render. Every
compare script prints the discs it used and their source.

Validation (centre / half-axes, px):

| check | result |
|---|---|
| dump vs `.duf` fallback, Calib-Glossy / -BumpRes / -MetalRefr | 0.002 |
| Calib-DualLobeGlossy-bare (all matte) detection | centres within 0.7, radii +0.5 - 1.5 (the threshold eats the darkened rim) |
| Calib-MetalRefr, known 424 + 153.3 i, 562, R 46 | 423.4 + 153.2 i, 560.7, R 46.0 - 49.0 × 45.9 |
| furnace-rings' measured spans (Calib-Furnace-*) | centres ±0.4, half-axes −0.6 - 0 |
| Calib-BumpRes's half-size sphere | found at (1493.5, 583.1) R 24.4 × 22.9; detection (bumped render) within 2.5 |

`calib/scenes`: 319 of 363 scenes have spheres (269 from the dump, 50
from the `.duf`): 236 rows of eight at Calib-Materials's framing (the
same centres in all of them), 83 single-sphere scenes (light and bump
scenes). The 44 without (G9 eye, strands, Smooth,
RoundCorners, Shell-G9) fall back to a matte render or the frame. No
two-row layout exists yet. Lens shift / distortion / stereo are flagged,
not modelled (Calib-Lens-*: nine scenes).

The first projection also took the Env-Mi scenes' floor plane for a
sphere (four vertices, all equidistant from their centroid): hence the
vertex count and extent tests.

## 3. The damage (same renders, old finder → projected discs)

`calib-compare-all.py` per sphere, current enhanced-final (and Oct 27's
preview renders): reading columns 5 - 9 as spheres 4 - 8 the old numbers
are within 1 - 3 % of the true ones on 22 of 24 scenes; Calib-SSSColor
8 %, and **Calib-Glass sphere 4: 2.80 / 4.11 for a true 2.42** (2.40 -
2.42 in all four variants). Reading the first eight columns as S1 - S8
shifts spheres 4 - 8 by one: up to 0.3 on Tone / DOF scenes, 2.8 × on
Glass.

`highlight-spread.py --vs` (upper disc, Octane / Iray), enhanced-final:

| scene | old discs | projected |
|---|---|---|
| Calib-DualLobeGlossy | 1.12 1.12 1.22 1.21 1.17 1.27 1.39 1.48 | 1.17 1.12 1.20 1.19 1.16 1.21 1.26 1.35 |
| Calib-Map-Metal | 1.10 1.17 1.22 1.23 1.26 1.25 1.19 1.06 | 1.14 1.14 1.14 1.13 1.13 1.15 1.13 1.06 |
| Calib-Glossy | 1.06 1.13 1.10 1.14 0.70 1.16 1.03 1.23 | 1.08 1.14 1.15 1.12 0.95 1.16 1.08 1.23 |
| Calib-NVIDIA | 1.28 1.20 1.05 1.00 1.06 1.04 1.37 1.06 | 1.21 1.12 1.04 0.97 1.09 1.04 1.09 1.06 |
| Calib-NVIDIA2 | 1.04 1.04 1.08 0.99 1.07 1.10 1.31 1.01 | 1.01 1.05 1.03 0.99 1.04 1.07 1.09 0.95 |
| Calib-DualLobe2 | 1.13 1.17 1.18 1.47 1.26 1.99 1.58 1.92 | 1.11 1.13 1.14 1.32 1.18 1.65 1.37 1.64 |
| Calib-DualLobe | 1.13 1.11 1.10 1.14 1.12 1.16 1.10 1.18 | 1.14 1.09 1.08 1.14 1.09 1.16 1.08 1.16 |
| Calib-DualLobeWeighted / -TL, -WeightedTL, -Mixing, -PBRSkin, -Aniso | | within 0.03 |

The old finder on today's Calib-DualLobeGlossy render reproduces #106's
spec numbers exactly (1.17 - 1.27, Top Coat 1.39, W 0.5 1.48): same
export, wrong discs. Spreads (p99 / median) move far more: the old disc
was the highlight, so its median was high (Calib-DualLobe Iray 3.0 / 1.9 /
1.5 … old vs 12.0 / 7.3 / 4.6 … projected). #95's spreads (13.4 / 8.4 /
5.6 / 11.1 / 5.0 / 9.2 / 14.0 / 5.7 for Iray) were measured on
sphere-sized discs in a scratch script: the projected r < 0.8 gives 13.3
/ 8.6 / 5.6 / 11.2 / 5.1 / 9.3 / 14.6 / 7.7 — unaffected but for the
outer sphere.

Layer split (`gloss-split.py`), Calib-DualLobeGlossy: the elliptic disc
moves only the outer spheres — sphere 8's glossy layer 1.32 (circle of
the vertical radius, #137's 1.31) → 1.40, as the gloss peaks at its
right rim. Stacking loss (1 − both / (gloss + dl)): Iray 2 1 4 3 27 2 %,
Octane 1 0 1 4 26 2 % on spheres 3 - 8 (old discs 5 3 6 5 26 4 / 3 1 3 3
24 2 %). `refr-volume-mc.py --measure` and `furnace-rings.py measure`:
within 0.001 - 0.01 of their own finders.

## 4. The spec, rule by rule

Corrected numbers (the rules stay; each was derived from the MDL / `.mi`,
not from these numbers):

- variant summary: Calib-Glass sphere 4 2.8 → 2.4 × (all four variants);
- Glossy Reflectivity (#100): the "after" gloss split r 0.25 / 1 / 0.5 —
  on the projected discs 1.09 / 1.04 / 1.07, the r 1 sphere 1.12 (the
  F0 → IOR rule is `metallic_layered_base`'s curve, head-on);
- the gloss by position (#137): 1.11 → 1.20 → **1.12 → 1.29** over the
  frame's 1.08, dual lobe 1.06 → 1.22 → 1.07 → 1.24 (the elliptic disc
  takes the outer spheres' rims). The sun-only model said 1.07 → 1.30
  (1.08 → 1.38): closer than #137 had it; the decision (keep the
  physical Fresnel, nothing fitted) stands;
- glossy layer over a dual lobe (#106): both layers 1.17 - 1.21 × (Top
  Coat 1.26, W 0.5 1.33), each alone 1.17 / 1.12; stacking Iray 1 - 4 %,
  Octane 0 - 4 % (Top Coat 27 / 26 %); the gloss 1.21 / 1.26 / 1.40 at
  spheres 1 / 3 / 8 (not 1.66). The stacking rule is DAZ's `.mi`;
- Metallic Weight (#59): the before / after table was on the highlight
  discs; today's export on the projected discs is 1.13 - 1.15 × on the
  seven metal spheres (specular mode 1.06), not 1.12 - 1.28. The w² / k
  rule is DAZ's `.mi` (w passed to both layers).

Checked, within 0.03, left as written: #81's Calib-DualLobe D6 (1.16;
neighbours 1.08 - 1.16 instead of 1.10 - 1.18), #121 (Calib-DualLobeWeighted),
#128 (-DualLobeWeightedTL, -WeightedTL), #95's spreads, the Tone / ISO /
PBRSkin ranges (columns 1 - 3 right, the rest read past the sliver).

A **fitted constant** that rests on the broken finder: `FITTED_GGX_CURVE`
(r + 1.85 r², #56) was fitted on highlight-spread's p99 / median over the
highlight-centred discs. It no longer drives the Uber gloss (#95 replaced
it with the Phong pin), but NVIDIA Architectural's reflection pins
(`fitted_pins`, `arch_aniso_pins`) still use it — filed as #156, not
changed here.

## Lab notes that used the broken finders

highlight-spread's discs (on the highlight): 2026-10-31 mdl-mapping
(#56, #59 tables), 2026-11-11 gloss-over-dual-lobe(-compare, -layers),
2026-11-11 dual-lobe-weighted-compare, 2026-11-11 weighted-translucency,
2026-11-11 translucent-weighted-dual-lobe, 2026-11-11 resin-layer's
"upper disk", 2026-11-13 universal-dual-lobe-reflectivity,
2026-11-13 universal-glossy-reflectivity. calib-compare-all's nine boxes:
every `report.md` / per-sphere row before this commit (2026-10-31
full-calibration-pass, 2026-11-03 diffuse-roughness, 2026-11-11
metal-diffuse-factors, 2026-11-11 scene-camera, 2026-11-13
denoised-pass-compare). Their numbers are history; this note says how
far they are off.

## Process

- Geometry beats image analysis for a generated scene: the dump is
  already there for every exported scene, and the projection is exact
  to the pixel without a single threshold.
- A finder that applies one scene's discs to another scene's render is
  wrong as soon as the layouts differ; each render now gets its own
  scene's discs.
- The old finder reproducing a spec number exactly on today's render
  tells which numbers were the finder's and not the export's.

Not covered: `skin-compare.py`'s face and sphere boxes (the private
Calib-Skin figures, #19 hold) and `calib-compare.py`'s primitives
regions (the user's own Calib-Base scenes) stay hand-placed.
