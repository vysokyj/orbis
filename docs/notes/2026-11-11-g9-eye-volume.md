# 2026-11-11 — Genesis 9 eye: the volume's return verified (#66)

Follows [2026-11-11-g9-eye-scenes.md](2026-11-11-g9-eye-scenes.md). The
Iray batch (Calib-G9Eye 5000 samples, seven variants at 1000) came back
(4e8780f); the Octane side was rendered before it (b6a8d77, 400 samples,
enhanced-final). Medians in linear light (gamma 2.2), Octane / Iray, per
region at 1920 × 1080: iris ring r 70 – 115 px around (972, 607), sclera
left box x 760 – 830 / y 640 – 700, sclera right box x 1100 – 1180 (added:
the left box sits in the eyelid's shade). The pupil is black in both.

## Decompose first (old export: the maps as albedo)

| variant | iris | sclera L | sclera R | reading |
|---|---|---|---|---|
| -t0 | 0.90 / 0.92 / 0.91 | 0.92 / 0.95 / 0.95 | 0.95 / 0.97 / 0.97 | the diffuse path is right |
| -t1-white-nosss | 12 / 1600 / 4700 | 6.5 / 710 / 1800 | 10 / 650 / 1300 | Iray: absorption only, the ball nearly black (R 0.07); Octane white — the bake ignores the volume entirely |
| -t1-white | 2.27 / 2.23 / 2.95 | 1.95 / 2.47 / 3.28 | 1.89 / 2.30 / 3.02 | MC predicted 1.6 / 2.3 / 3.1 |
| -t08-white | 1.84 / 1.93 / 2.28 | 1.75 / 2.03 / 2.37 | 1.70 / 1.96 / 2.32 | MC predicted 1.5 / 2.0 / 2.4 |
| -t1 | 11.7 / 18 / 39 | 4.5 / 6.1 / 8.6 | 4.5 / 6.4 / 9.0 | |
| -nogloss | 3.6 / 3.9 / 4.2 | 2.8 / 3.2 / 3.5 | 2.8 / 3.3 / 3.6 | |
| Calib-G9Eye | 3.8 / 4.3 / 4.9 | 2.4 / 2.8 / 3.0 | 2.6 / 3.1 / 3.4 | |

So the white variants' effective albedo (Iray / Octane-white): sclera R
0.53 / 0.43 / 0.33 against the MC's 0.61 / 0.43 / 0.32; t 0.8 0.59 / 0.51
/ 0.43 against 0.65 / 0.50 / 0.42. G / B within 5 %, red 10 – 16 % low.

## The closed form

Each time light inside the ball meets the surface it leaves with t·T or is
reflected back in with (1 − t)·B, and the re-launched walk is Lambertian
like the first entry (a ball looks the same from every point). With P the
escape on the lit half at a white interface (`-t1-white`'s albedo):

A = (1 − t) B + t² T² P / (1 − (1 − t) B P)

At t 0.8, white: 0.645 / 0.501 / 0.419 — the MC's own t 0.8 run gave
0.65 / 0.50 / 0.42, so the series is exact enough. The MC's front-half
escape equals its total (0.608 vs 0.610 red): with g −0.8 light comes
out near where it went in, the back of the ball doesn't matter.

T or T² for the map? Projected from the old renders (Octane × A / old
albedo) on -t1: T alone 2.6 – 2.9 × Iray, T² 0.96 – 1.03 G / B. T²: the
tint acts on the way in and on the way out, as the layer is crossed twice.

## Implemented and re-rendered

`src/eye_volume.rs`: P by a deterministic Monte Carlo at export (20 000
paths per channel, splitmix64, a few ms), from the material's channels
(the `.mi`'s coefficients: absorption −ln(Transmitted Color) / 0.1 cm,
scattering −ln(SSS Color) / 0.015 cm, HG at SSS Direction), radius 1.3 cm
(the `l_eye` bone). `Textures::eye_bake` applies A per texel.

First re-render: the mapped variants came out like the white ones — the
G9 eye's maps are one *layered image* (`#Eye Color-3`), and the old
`Textures::blend` read only plain files: it had silently returned None
all along, so the old export never baked anything, it rendered the
diffuse map through the plain path. `open_colour_or_layered` flattens
the stack first.

| variant (new export) | iris | sclera L | sclera R |
|---|---|---|---|
| -t1-white | 1.34 / 0.93 / 0.92 | 1.17 / 1.04 / 1.04 | 1.14 / 0.99 / 0.97 |
| -t08-white | 1.15 / 0.94 / 0.93 | 1.11 / 1.00 / 0.98 | 1.09 / 0.97 / 0.97 |
| -t1 | 1.20 / 0.81 / 1.00 | 1.25 / 1.04 / 1.00 | 1.20 / 1.00 / 0.96 |
| -nogloss | 0.96 / 0.90 / 0.88 | 1.08 / 1.00 / 0.97 | 1.07 / 1.00 / 0.97 |
| -bare | 1.55 / 1.88 / 2.26 | 1.06 / 1.00 / 1.00 | 1.09 / 1.08 / 1.09 |
| Calib-G9Eye | 1.50 / 1.73 / 2.17 | 1.08 / 1.03 / 1.03 | 1.14 / 1.08 / 1.09 |

The volume part is within ±15 % with the maps (-nogloss). Red stays 10 –
25 % high where T is 1 or white: red's path in the ball is longest
(absorption length 19 mm), so it is the channel that meets what the model
leaves out (the eye mesh's inner geometry); not fitted.

## What is left: the gloss over the iris

-nogloss → -bare switches only the dual lobe on: Iray's iris goes 0.0385 →
0.0399, Octane's 0.037 → 0.062 — a grey sheen of about +0.02 that the old
bright albedo hid (+13 % then). The sclera hardly moves in either. The
Octane render also shows the iris's bump relief much harder. A separate
component, #133.

## Process

- My waiting loops used `pgrep -f calib-variants.sh`, which matched the
  loops' own command lines: they never ended. Wait on the render script's
  own background task instead.
- Iray ~2 min per 1000-sample variant, Octane ~190 s per scene at 400.
