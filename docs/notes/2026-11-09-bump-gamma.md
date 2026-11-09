# 2026-11-09 — the height maps' decode gamma (#91)

Question: #85 / #89 bake height maps with the heights read linear; #90
showed Iray's loader gives an 8-bit canvas gamma 2.2 under "auto". The
linear choice rested on Cast Iron's band-passed detail (1.05 × linear
vs 1.13 × at 2.2). Decompose on a synthetic map instead.

## What DAZ hands Iray (.mi)

New fixtures (Calib-BumpGamma-Uber, -Uber2, -NVIDIA, Calib-NVIDIA-BumpG1):
an image at `map_gamma` 0 goes to Iray as gamma "auto" on the Uber's
`base_bump`, NVIDIA's `bump_texture` and the Uber's normal map alike; at
`map_gamma` 1 as gamma 1. Product content (133 maps sampled across the
library): bump maps at 0 and 1 alike (76 / 57), normal maps nearly
always 1. The eight Calib-Skin figures: face and body maps at 1, Zurie's
arms / nails and some eyes and mouths at 0.

## The scene

Calib-Leaf's three vertical cards, the distant light turned to the
front (45° up, in the plane of v), Scene Only, tone mapping linear
(burn 1, crush 0), EV 9.1. A synthetic 128 px height map, four bands of
32 texels across u: a sawtooth rising 8 levels a texel along v, the
same falling, 128 with a step to 160, flat 128. Bump factor 4.

- Calib-BumpGamma-Uber: Iray Uber diffuse 0.5 linear; the 8-bit PNG at
  `map_gamma` 0, at 1, a 16-bit PNG at 0.
- -Uber2: normal maps of a ±0.25 tilt at 0 and 1; the 8-bit height map
  at factor 2.
- -NVIDIA: NVIDIA's Concrete (`file_bump_texture` on the whole surface),
  colour a flat grey map, Overall Reflectivity 0; built headless by
  `daz-build-leaf-scene.dsa` with the Concrete preset (the script didn't
  select the card's surfaces: the preset never applied — fixed).
- Calib-NVIDIA-BumpG1: Calib-NVIDIA with its four bump maps at
  `map_gamma` 1 (Iray reads them linear).

First Octane renders gave a tilt contrast of 0.139 where 0.251 was due:
the floor's bounce lit the cards by ~45 % of the light's share. Hiding
the floor made DAZ's .mi move the dome ground's 1 mm drop (one case, not
in `iray.rs`), so the floor went black instead. The cards are then lit
by the light alone.

## Measure (`bump-gamma.py`)

Per texel row, each sawtooth band against the flat band of the same row:
R = (1 ∓ x) / √(1 + x²), x = f · s (Lambert, tan 45° = 1), solved for x.
A 1.0 decode gives x = f · 16 / 255 on every row; a 2.2 decode
f · ((8 (j + 1) / 255)^2.2 − (8 (j − 1) / 255)^2.2) at the band's own
height. No fitted scale: the export's linear bake gave x 0.252 per row
against 0.251.

**Dead end in the analysis:** the first version took r = (I_fall −
I_rise) / (I_fall + I_rise) = f s. That holds only when both bands have
the same slope, true for a linear decode, not at 2.2: the rising and the
falling band sit at heights h and 248 − h on the same row. Read that way
Iray's 8-bit card looked "in between" (best power 1.35, RMS 0.018 — a
fit I didn't keep); band by band it is plainly 2.2. Rows within 5 texels
of the sawtooth's 248-level drop are left out: Iray's texture filtering
spreads the drop over more texels than Octane's (its falling band read
30 % steep at j 3 – 4).

Cards located by their grey pixels' corners and a homography from UV, so
a skewed view works too: the Iray Server rendered Calib-BumpGamma-NVIDIA
from DAZ's Perspective View (the headless build saved no
`current_camera`, the trap the 11-05 note warned of).

## Results (RMS of f · s, rising and falling bands)

| card | Iray vs 1.0 | Iray vs 2.2 | export (new) vs 1.0 | vs 2.2 |
|---|---|---|---|---|
| Uber 8-bit, `map_gamma` 0 | 0.130 | **0.031** | 0.116 | **0.009** |
| Uber 8-bit, `map_gamma` 1 | **0.018** | 0.116 | **0.002** | 0.114 |
| Uber 16-bit, 0 | **0.018** | 0.116 | **0.002** | 0.114 |
| Uber 8-bit at 0, factor 2 | 0.059 | **0.005** | 0.058 | **0.010** |
| Concrete 8-bit, 0 | 0.130 | **0.032** | 0.117 | **0.009** |
| Concrete 8-bit, 1 | **0.018** | 0.116 | **0.003** | 0.114 |
| Concrete 16-bit, 0 | **0.018** | 0.116 | **0.003** | 0.114 |

Iray's 8-bit card, rising band x by row 5 / 9 / 13 / 17 / 21 / 25 / 28:
0.076 0.134 0.205 0.272 0.342 0.419 0.456; the 2.2 decode 0.060 0.121
0.188 0.260 0.335 0.413 0.473. Iray's linear cards come out 0.239 rising /
0.273 falling against 0.251 (±7 %, in every linear card, the export's
0.253 / 0.250): Iray's own, not the decode's; left as is.

Normal maps (8-bit RGB): the flat band against a flat height band, and
the contrast of the ±0.25 tilts:

| | `map_gamma` 0 | 1 |
|---|---|---|
| Iray | 0.386 / 0.313 | 1.004 / 0.248 |
| Lambert, 1.0 decode | 1.004 / 0.247 | |
| Lambert, 2.2 decode | 0.344 / 0.626 | |
| export, `gamma` pin 2.2 | 0.996 / 0.247 | |
| export, re-baked linear | 0.379 / 0.304 | 1.000 / 0.247 |

Iray decodes the normal map at 2.2 too. The tilts' contrast differs
from Lambert's at 2.2 the same way in Iray and Octane (the down-tilted
band shades at 0.09 of the light, near the terminator): both renderers'
own, not the decode. Octane ignores a normal texture's `gamma` pin
(legacy gamma space set, rendered as 1.0), so such a map goes in
re-baked linear.

**Rule:** heights and normals decode at the image's gamma, else "auto"
by the pixel type: 8 bits 2.2, deeper 1.0 (`detail_normal::auto_gamma`).

## An image without an `image_library` entry

The export took "no entry" as "auto". On Calib-Skin's spheres (the
Torso material with cropped maps, saved without entries) the normal map
went in at 2.2 and the skin sphere dropped to 0.47 / 0.51 × Iray (Zurie
/ Monique; 0.95 / 1.17 before). Calib-Skin-Zurie-front's .mi (local):
the cropped bump, normal and specular maps at gamma 1, the diffuse
"auto" — the channels' `default_image_gamma`. Calib-Mi-AoA's
calib-tiles.png, also unlisted, went "auto" on its Normal Map (default 1)
— the same file was on the diffuse first. Calib-Mi-ImageGamma (.mi only,
the maps unlisted) settles it: alone on the diffuse "auto"; on Bump
Strength, Normal Map, Glossy Roughness 1; one file on a diffuse then a
bump "auto" on both; bump then diffuse 1 on both. DAZ keeps one image
per file, its gamma from the first channel that loads it
(`daz_material::scene_image_gammas`). With it the skin spheres are back
at 0.95 / 1.17.

## Regressions

- Calib-CoatBump, Calib-BumpRes: their maps at `map_gamma` 1 (BumpRes 16
  bit) — exports byte-identical to the previous binary.
- Calib-Skin Zurie / Monique front (400 samples, region medians Y /
  r/g): within the run-to-run noise (cheeks ±0.03, ears ±0.05); face
  p99.5 1.004 → 1.004, 0.962 → 0.958.
- Calib-NVIDIA (Octane 1024; #89's measure, this session's windows:
  disk r 0.75 · 40 px, centre box ±20 px, box blur r 2 − r 8): Cast Iron
  disk 0.98 → 0.94, detail 1.21 → 1.39 ×, correlation 0.90 → 0.81;
  concrete 0.98 → 1.02, polished 1.03 → 1.07, brushed unchanged.
  Decomposed with Calib-NVIDIA-BumpG1 (Iray, the maps at 1, against
  Calib-NVIDIA): Cast Iron detail 0.94 ×, correlation 0.98 — Iray's
  detail hardly depends on the decode, while Octane's is above Iray's at
  either. Cast Iron's excess is something else (a 2048 px map ~25 ×
  minified; the same map as roughness): #92. Not fitted.
- Calib-NVIDIA2 (asphalt, tiles) exports differently (8-bit PNG bumps at
  2.2); not rendered — its fitted-camera copy is gone again.

## Also

- `iray.rs`: the volume's arguments (transmitted colour, measurement
  distances, SSS direction) on any closed surface, not only one that
  lets light in — Calib-BumpGamma-Uber's cards (Thin Walled off,
  translucency 0) got DAZ's 0.001 / 0 / black; every fixture passes.
- Follow-ups: #92 (Cast Iron's detail), #93 (data maps' "auto" ignores
  the pixel type: a 16-bit roughness or displacement map read at 2.2).

## Process

The Iray batch stalled the first time: the queue-side DAZ wasn't
running (`status.py` said STALLED); the user started it. ~100 s a
scene. Octane: the card scenes 26 – 40 s at 400 samples, Calib-NVIDIA
~60 s at 1024, the two skin figures 46 – 99 s.
