# 2026-11-05 — Diffuse Overlay maps against Iray (#71)

Follow-up of [2026-11-04](2026-11-04-diffuse-overlay-maps.md): the Iray
reference of Calib-Overlay arrived (third batch, 5000 samples,
`calib/references/iray/Calib-Overlay.png`). Variant enhanced-final,
Octane 1000 samples, scene exported from My Library.

## First render: the squaring never reached Octane

Whole-frame report 1.04 (the spheres are small), so measured per sphere:
a vertical strip through each centre (7 px wide, rows of 4 px) and the
sphere disc (r 38 px), linear (÷ gamma 2.2), Octane / Iray.

| sphere | belt w 0.5 (G) | belt w 0.25 (G) | belt 1 |
|---|---|---|---|
| V1 weight map | 1.02 | 1.09 | 1.03 |
| V2 squared | **4.5** | **14.5** | 1.04 |
| V3 × 0.5 squared | **4.2** | **7.0** | 1.02 |

V2 rendered exactly as V1. The export wrote the right node (the weight
map at gamma 4.4, power 1 / 0.25), but `image_pin` put every map with
gamma > 1 in colour space sRGB (1), where Octane ignores the `gamma` pin
— 4.4 read as sRGB ≈ 2.2. Only "linear sRGB + legacy gamma" (6) applies
it (`docs/octane/octane-x.md`).

Fix: `image_colour_space` — linear at gamma ≤ 1, sRGB at 2.2 (unchanged
for every colour and data map so far), legacy gamma (6) otherwise. Also
covers an image's own `map_gamma` other than 1 / 2.2, which was read as
sRGB too.

## After the fix

Sphere discs (overlay pixels only: Iray G > 0.03):

| sphere | disc Y | overlay Y | strip belts |
|---|---|---|---|
| V1 weight map, w 1 | 1.04 | 1.04 | 1.01–1.09 |
| V2 squared | 1.04 | 1.03 | 0.95–1.09 |
| V3 w 0.5 squared | 1.05 | 1.04 | 0.97–1.10 |
| V4 colour map, w 0.5 | 1.03 | 1.03 | |
| V5 colour map, squared | 1.03 | 1.04 | |
| V6 both maps | 1.04 | 1.03 | |
| V7 both, tinted 0.5 / 0.8 / 1 | 1.05 | 1.04 | |
| V8 both, Overlay Roughness 1 | **0.91** | **0.85** | |

Per pixel the luminance correlates 0.993–1.000. Calib-Diffuse's smooth
sphere is 1.08 × Iray, so V1–V7 sit where a plain diffuse does.

- Checker cells: blue B 1.01–1.02, yellow R / G 1.03–1.07. The yellow
  cells' blue (Iray 0.004–0.008 linear) reads 1.5–1.8 ×; the flat red base
  shows the same on its near-zero B (≈ 0.001), so it is the dark channels'
  noise / quantisation, not the map's decode (luminance unaffected).
- V8 is the overlay's Oren-Nayar flat term (A(σ₀), #32), map-independent:
  the flat white overlay at σ₀ 1 was 0.89 already. Split off as #76.

## Lessons

- A whole-frame ratio hides everything on small spheres; the strip
  profile found the unsquared belts in one look.
- An exported node that looks right can still be ignored by Octane:
  check which colour space reads which pin.
