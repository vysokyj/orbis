# 2026-11-14 — A translucent prop's volume as the ball's albedo (#87)

Resume of 2026-11-13-prop-volume.md with the Calib-PropVol-* Iray
references in (91b1388, 5000 samples). The question the batch was to
answer: does Iray stay DAZ's MDL on thin solids, and which Octane rule
follows it - the scattering medium, a bake of the ball's albedo
(`eye_volume`'s way) or a mix.

Octane 1000 samples, Enhanced + Final, raw beauty; per sphere the mean
over the square within 0.3 of its projected disc (`sphere_discs.py`),
linear. "Iray albedo" = Iray's sphere over its place's light (Octane's
plain diffuse 0.5 on the same sphere, `x87/octane/Calib-SSSColor__grey.png`,
same spheres, camera and HDRI in every scene). Work dir
`~/Library/Caches/orbis/calib/x87` (`octane-pv` the 2026-11-13 renders,
`octane2` the medium without fake shadows, `octane3` the shipped rule).

## Step 1: the .mi of the series (src/iray.rs)

The four new fixtures (Calib-PropVol-Chromatic / -Grey / -Mono /
-Parts-x100) disagreed with `src/iray.rs` on three spheres: "P8 no
volume" (chromatic, SSS Amount 0, SSS Color set) and "Q4 no scattering"
(the same in Parts-x100) have `sub_surface_enable` true and the SSS Color
as `sub_surface_scattering_amount`. DAZ switches chromatic scattering on
by the colour, not the amount; Calib-SSS-Solid's S7 (Amount 0, colour
black) stays off; mono goes by the amount. Last session's note (step 1)
had it the other way. So P8 chromatic / grey are a lossless scattering
ball, not an empty one, and Q4 is the full volume (= Q-dense). Fixed in
`iray.rs` (the test fails without it on exactly those six arguments),
`scattering()`'s fallback and `prop-volume-mc.py`.

## Step 2: Iray against the MDL over thickness

Iray albedo / Monte Carlo (`prop-volume-mc.py`, furnace, radius 15 cm),
red; k = both measurement distances × k (the volume in a ball 1 / k the
size):

| series | k 1 | 3 | 10 | 30 | 100 | 300 | 1000 |
|---|---|---|---|---|---|---|---|
| Chromatic | 0.96 | 0.96 | 0.97 | 0.99 | 1.08 | 1.19 | 1.31 |
| Grey | 0.95 | 0.95 | 0.95 | 0.95 | 1.03 | 1.13 | 1.27 |
| Mono | 0.96 | 0.95 | 0.96 | 0.97 | 1.05 | 1.16 | 1.30 |

G / B 0.95 – 0.97 up to k 100. Parts at k 1 (Tc white, base white, no
absorption, w 0.5 / 1, direction ±0.6): 0.87 – 0.97 (no absorption, a
lossless ball, 0.87: its light wanders the whole ball, so the lit front
gives less than the furnace). Iray is the MDL wherever the light comes
back out near where it went in. From k 100 on Iray is above the
furnace: the ball passes light from its far side, which a furnace
albedo cannot hold (Parts-x100 0.74 – 1.41 in red, by how much each
part lets through).

## Step 3: Octane's scattering medium, fake shadows off

#104 found fake shadows let shadow rays into a medium past a refracting
surface. Here (a diffuse wall) they change nothing measurable: red
0.56 / 0.60 / 0.56 on k 1 with or without them; only Q7 (g +0.6) moved,
0.76 → 0.54. With the new iray.rs P8 chromatic gets its medium and goes
1.50 → 0.80, Q4 0.21 → 0.57.

Octane / Iray, red, medium (no volume factor, fake shadows off):

| series | k 1 | 3 | 10 | 30 | 100 | 300 | 1000 |
|---|---|---|---|---|---|---|---|
| Chromatic | 0.56 | 0.55 | 0.48 | 0.42 | 0.52 | 0.67 | 0.77 |
| Grey | 0.60 | 0.60 | 0.60 | 0.60 | 0.59 | 0.68 | 0.78 |
| Mono | 0.56 | 0.56 | 0.56 | 0.55 | 0.56 | 0.68 | 0.77 |

G / B 0.92 – 1.13. The place's spread is gone, but red is 0.42 – 0.60
on every dense volume, grey σs and mono σs included: the leak is the
saturated σa (Transmitted Color 0.98 / 0.48 / 0.35), as 2026-11-13's
grey-σa render showed (0.94 – 1.02 of the Monte Carlo). No physical
correction exists without Octane's RGB → spectrum upsampling, and one
per colour would be a fit. Not shipped.

## Step 4: the ball's albedo (the eye's way)

The empty-ball components first, so the bake isn't credited with what
the wall does: an empty closed ball (Calib-SSS-Solid S2 / S3, PropVol-Mono
P8: no volume in Iray) as the export's hollow transmission is 0.92 /
0.96 / 0.83 × Iray in red, G / B 0.99 – 1.05 - that is #134's closed
solid, left as it is (no volume: no bake).

Where Iray has a volume, Universal gets one albedo, no transmission:
(1 − w) b + (w Tc)² P / (1 − (1 − w) b P) (`eye_volume::ball_albedo`),
P the share of the light let in that the volume returns to the wall,
traced in a ball of the material's surface area (`escape_ball`; every
exit counts, a lossless channel returns all). Octane renders a diffuse
albedo linearly at the place's light, so the prediction before any
render is Monte Carlo / Iray albedo:

| series | k 1 | 3 | 10 | 30 | 100 | 300 | 1000 |
|---|---|---|---|---|---|---|---|
| Chromatic R | 1.04 | 1.04 | 1.03 | 1.01 | 0.92 | 0.84 | 0.76 |
| Grey R | 1.06 | 1.05 | 1.06 | 1.05 | 0.97 | 0.88 | 0.79 |
| Mono R | 1.04 | 1.05 | 1.05 | 1.03 | 0.95 | 0.87 | 0.77 |

G / B 1.03 – 1.04 to k 100, 0.87 – 1.01 above. Calib-SSSColor V1 – V7
1.03 – 1.07 (its plain diffuse V8 1.04 – 1.06: Octane's diffuse over
Iray's at that place, the level the bake can reach). Parts k 1 1.03 –
1.05 (no absorption 1.12 – 1.15).

Rendered (`octane3`, the export as committed), Octane / Iray R / G / B:

| sphere | Chromatic | Grey | Mono |
|---|---|---|---|
| k 1 | 1.09 / 1.01 / 1.13 | 1.10 / 0.99 / 1.17 | 1.10 / 0.99 / 1.16 |
| k 3 | 1.09 / 1.00 / 1.14 | 1.11 / 0.99 / 1.18 | 1.10 / 0.98 / 1.18 |
| k 10 | 1.09 / 0.99 / 1.17 | 1.11 / 0.98 / 1.19 | 1.10 / 0.97 / 1.19 |
| k 30 | 1.06 / 0.97 / 1.21 | 1.11 / 0.97 / 1.21 | 1.09 / 0.97 / 1.21 |
| k 100 | 0.98 / 0.95 / 1.24 | 1.04 / 0.95 / 1.23 | 1.02 / 0.94 / 1.24 |
| k 300 | 0.90 / 0.90 / 1.21 | 0.95 / 0.90 / 1.21 | 0.93 / 0.89 / 1.21 |
| k 1000 | 0.82 / 0.81 / 1.10 | 0.85 / 0.81 / 1.11 | 0.83 / 0.80 / 1.10 |
| P8 | 1.23 / 1.07 / 1.31 (lossless) | 1.27 / 1.06 / 1.30 (lossless) | 0.83 / 1.03 / 0.97 (empty: hollow) |

Against the prediction red is +4 – 6 %, green −4 %, blue +10 – 17 %:
Octane renders a saturated diffuse colour (0.31 / 0.047 / 0.023 here)
spectrally too - the plain diffuse V8 (0.61 / 0.27 / 0.17) is 1.08 /
1.03 / 1.08 against a predicted 1.04 / 1.06 / 1.06. Blue is the weakest
channel by 13 ×; in luminance the dense spheres are 1.02 – 1.05.

The rest (Octane / Iray R / G / B; old = the export before, hollow ×
volume factor; med = the medium):

| sphere | bake | old | med |
|---|---|---|---|
| Calib-SSSColor V1 – V7 | 1.09 – 1.13 / 0.95 – 1.01 / 1.13 – 1.24 | 0.69 – 1.29 / 1.03 – 1.46 / 0.86 – 1.41 | 0.56 – 0.64 / 1.01 – 1.13 / 0.88 – 1.00 |
| Parts k 1, Q1 / Q2 / Q4 – Q8 | 1.06 – 1.12 / 0.97 – 1.05 / 1.06 – 1.21 (Q6 B 1.45, B 0.007) | 0.71 – 1.47 | 0.50 – 0.89 |
| Parts k 1, Q3 lossless | 1.21 / 1.09 / 1.25 | 0.82 / 1.03 / 1.02 | 0.79 / 1.00 / 0.96 |
| Parts k 100 | 0.76 – 1.40 / 0.92 – 1.03 / 1.06 – 1.59 | 0.33 – 0.81 | 0.46 – 0.87 |
| Calib-SSS-Solid S6 (lossless) / S7 (absorbing) | 1.15 / 1.08 | 0.99 / 1.33 | 0.92 / 1.08 |

The empty balls (S2, S3, S8, Mono P8) and the opaque S1 are unchanged
(no volume, no bake). Parts-x100's Q1 / Q2 (Tc or base white, thin) are
the worst, 1.40 / 1.28 in red: the furnace counts light that a thin ball
under one-sided light sends out of its back. Lossless volumes 1.15 –
1.27: their light wanders the whole ball, so the lit front gets less
than the furnace's share.

## Decision

The bake. On dense volumes it is the MDL at Octane's plain-diffuse level,
all channels; the medium loses 40 – 58 % of red there, the old hollow
transmitter × the fitted volume factor followed the place. On the thin
end (k ≥ 100) neither holds Iray's light through the ball; the bake's
red falls to 0.82 – 0.85 at k 1000 (the medium's 0.77), and the hollow
transmitter without the factor - the empty-ball limit - would light
the inside again. `volume_factor` and `PROP_VOLUME_LEVEL` are gone;
the skin keeps `volume_albedo` (its A(α), #19 / #57 untouched).

## Process

- A fixture before the thickness argument: two of the series' spheres
  weren't what their labels said (P8 / Q4 scatter in Iray). The .mi of a
  new batch goes first.
- The prediction from the Monte Carlo and the place probe (Monte Carlo /
  Iray albedo) is what a bake renders - checked by the render, not
  replaced by it.
- The previous worker's Octane batch had finished in Octane X after the
  worker died; its PNGs were still in the sandbox (`orbis_job_*.png`),
  `octane-render.sh` never moved them. Look there before re-rendering.
