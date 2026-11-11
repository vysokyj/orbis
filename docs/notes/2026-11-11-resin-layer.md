# 2026-11-11 — Resin's level: Iray's layer weights, not the lobe (#113)

Goal: replace `RESIN_LOSS` (1 − 0.64 r, fitted on 2026-11-05 against a
view reconstructed from Calib-NVIDIA2's mis-framed reference, #97) with a
derived rule. The issue: on the re-rendered reference A7 (grey resin, r
0.25) read 1.28 × Iray. Variant enhanced-final, Octane 1024 samples, Iray
Server 5000.

## The MDL

`nvidia/resin_polyurethane_matte.mdl`: `fresnel_layer(coat_ior 1.49,
glossy_weight 0.95, simple_glossy_bsdf(reflect, white), simple_glossy_bsdf
(reflect_transmit, tint base_color))`, both at the same roughness (default
0.25), material IOR = coat_ior, no volume, not thin-walled — a clear coat
over a tinted rough glass, a closed solid. DAZ hands A7's base 0.793
(0.9 sRGB); the export reads the same.

## 1. Lobe energy, analytically (dead end for the cause)

Scratch MC of the sphere in a white furnace (disk r < 0.75), Iray's Phong
+ v-cavities (`min(G1, G2)` on both events, as first assumed) against
Beckmann-Smith at α = pin²: r 0.02 / 0.1 / 0.25 / 0.5 → 1.000 / 1.000 /
1.006 / 1.05. The lobe's energy explains 0.6 % at r 0.25, not 16 %. So the
fit was hiding something else.

## 2. The measurement

A7's "1.28" depends on the region: on the same pair of renders (Octane
enhanced-final 00:55, Iray c76e35c), disk r < 0.75 1.07 (sRGB or 2.2 decode
alike), centre r < 0.35 0.94, whole sphere (r < 0.92) 1.18,
`highlight-spread.py`'s upper disk (r < 0.9, above +0.3 r) 1.36. The ratio
rises from the centre to the rim (rings 0.93 → 1.51; with the full tint
1.26 → 1.90): an angular difference, not only a level. Calib-NVIDIA's red
resin (the issue's 1.30, 2026-11-05 render) is 0.99 on the disk.

## 3. Scenes: one part at a time

`make-calib-scenes.py`: Calib-NVIDIA2's spheres all resin (one module: one
DAZ session each, #94) — R1 grey r 0.25 (= A7), R2 grey 0.02, R3 white
0.25, R4 white 0.02, R5 grey 0.25 coat 0, R6 white 0.25 coat 0, R7 grey
0.5, R8 grey 0.1. **Calib-Resin** (HDRI + floor) and **Calib-Furnace-Resin**
(uniform dome 0.5, floor hidden). Geometry from Calib-NVIDIA2's dump (scene
path and sphere labels changed). Iray 141 s / 82 s, DAZ restarted between.

Furnace, sphere / background, disk:

| sphere | Iray | Octane full tint | physics (MC, Smith) | `libbsdf` model (MC) |
|---|---|---|---|---|
| R1 grey .25 | 0.473 | 0.653 | 0.629 | 0.501 |
| R2 grey .02 | 0.623 | 0.656 | 0.645 | 0.645 |
| R3 white .25 | 0.722 | 0.995 | 0.976 | 0.762 |
| R4 white .02 | 0.967 | 1.000 | 1.000 | 0.999 |
| R5 grey .25 c0 | 0.564 | 0.638 | 0.604 | 0.601 |
| R6 white .25 c0 | 0.913 | 0.994 | 0.981 | 0.975 |
| R7 grey .5 | 0.257 | 0.603 | 0.560 | 0.279 |
| R8 grey .1 | 0.605 | 0.656 | 0.644 | 0.626 |

- Octane keeps what physics does (its few % over are the imager's toe,
  as in the metal furnaces, 2026-11-10).
- Iray's rough *coated* resin loses a lot (white r 0.25: 0.72), the
  uncoated one little (0.91): the coat — white, lossless in physics — is
  where the light goes.

## 4. `libbsdf` (MDL SDK, `src/mdl/jit/libbsdf`, read, not copied)

- `simple_glossy_bsdf`: Phong half-vector distribution, exponent 2 / r²;
  v-cavities masking **max(G1 + G2 − 1, 0) on refraction** (min on
  reflection); reflect / transmit split by Fresnel at the microfacet.
- `fresnel_layer` → `curve_layer_sample`: coat picked with w · F(n·k1),
  weighted w · F(k·h); the base weighted **1 − w · max(F(n·k1),
  F(n·k2_refl))**, Fresnel at the macro normal, k2_refl the reflection of
  k1 about the base's microfacet. Not energy-conserving: leaving a rough
  base from inside, k2_refl lies past the critical angle (F = 1) for many
  microfacets and the base keeps 1 − w = 0.05.

The MC with these rules (last column) gives the coat's loss: R1 0.50 v
Iray 0.47, R7 0.28 v 0.26. Per interface, cos-weighted (white, coat 0.95,
r 0.25): entering 0.94 (physics 0.99), leaving 0.82 (0.97, head-on 0.89).
Iray stays 3 – 6 % below the model on every sphere (smooth too: 0.966 /
0.967 for R2 / R4) — a constant of this scene's furnace; the uncoated R6
6 % (0.913 v 0.975) is more than that and not explained (#115).

## 5. The rule

Tried first: per-crossing k = √(k_enter · k_leave), cos-weighted entry,
leaving at the refracted cosines — 1.01 on the r < 0.75 disk at r 0.25 but
the whole sphere 1.11 and r 0.5 1.25 (light trapped inside loses at each
internal event). Kept: **k such that the whole closed sphere in the white
furnace keeps in Octane what it keeps under `libbsdf`**, per channel, both
traced at export (`src/resin.rs`, 65 536 paths each, fixed seed; the paths'
weights and tinted-event counts kept so k solves exactly on them, ~70 ms
per resin material). Grey 0.79: 0.99 / 0.94 / 0.83 / 0.64 at r 0.02 / 0.1
/ 0.25 / 0.5; uncoated ≥ 0.99; per channel for a colour (red resin
0.83 / 0.88 / 0.88). Nearly the old fit for the coated resin (0.84 at
0.25) — it hit the level without the cause, and darkened the uncoated
resin to 0.80 ×.

## Results (Octane / Iray, disk r < 0.75; full tint → `RESIN_LOSS` → k)

| sphere | furnace | Calib-Resin (HDRI) |
|---|---|---|
| R1 grey .25 | 1.41 → 1.04 → 1.01 | 1.43 → 1.08 → 1.05 |
| R2 grey .02 | 1.08 → 1.05 → 1.06 | 1.09 → 1.06 → 1.07 |
| R3 white .25 | 1.40 → 1.02 → 0.99 | 1.42 → 1.05 → 1.03 |
| R4 white .02 | 1.06 → 1.03 → 1.03 | 1.07 → 1.04 → 1.05 |
| R5 grey .25 c0 | 1.15 → 0.83 → 1.15 | 1.16 → 0.84 → 1.15 |
| R6 white .25 c0 | 1.11 → 0.79 → 1.10 | 1.11 → 0.80 → 1.10 |
| R7 grey .5 | 2.37 → 1.23 → 1.13 | 2.37 → 1.31 → 1.21 |
| R8 grey .1 | 1.11 → 0.98 → 1.00 | 1.11 → 0.99 → 1.00 |

Hue unchanged (g/r within 0.01). Calib-NVIDIA2 (new): A7 1.05 / centre
0.92 / whole 1.16, A8 0.95, the rest as before (0.99 – 1.06). Calib-NVIDIA
(fresh render): red resin N7 0.99 (centre 0.86, g/r 0.23 v 0.20); Metal
Brushed reads 1.16 against the 2026-11-05 render's 0.98 (another binary;
its known #98 level), granite 1.05.

Regressions: `project.ocs` against the installed binary byte-identical for
Calib-Glossy, -PBRSkin, -SSS, -Glass, -Mixing, -TopCoat, -NVIDIA3; only
the resin transmission / specular values change in Calib-NVIDIA / -NVIDIA2.
Skin, glossy and translucent props don't go through this path (#87 is
untouched).

## Open

- Iray darkens the rough resin towards the rim more than Octane (rings at
  r 0.25 0.92 → ~1.4 with k): a constant tint can't draw it.
- Uncoated rough resin 1.10 – 1.15 × (Iray's transmission ~6 % below the
  `libbsdf` model), architectural glass's fitted loss and roughness: #115.

## Process lessons

- State the measuring region with a ratio: the same render pair read 1.07
  (disk), 1.18 (whole), 1.28 (the note), 1.36 (upper disk). A ring profile
  shows whether the gap is a level or an angle.
- A fit that matches the level can still be the wrong cause: the
  uncoated variant (one switch) showed it at once (0.80 ×).
- When the MDL spec is silent (layer weights), the MDL SDK's `libbsdf` is
  readable source for what `df::` does; a furnace scene then checks the
  model against Iray before any render-fitting.
