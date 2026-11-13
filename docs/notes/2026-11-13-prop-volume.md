# 2026-11-13 — A translucent prop's volume, decomposed (#87)

`volume_factor` ((Rd(α) / Rd(α₀))^0.5) × `PROP_VOLUME_LEVEL` 0.7 is a fit
on Calib-SSSColor (Victoria 8's volume on closed 15 cm Uber spheres, w
0.9; Iray 5000 in git). #83 read its leftover spread as a thickness: mono
SSS Amount 2 (α ≥ grey's in every channel) rendered darker than chromatic
grey. Decompose before fitting: the MDL first, then each renderer against
a known model.

Octane 1000 samples, Enhanced + Final, raw beauty; Iray the reference in
git. Per sphere the mean over a 44 px square at its centre, linear. Work
dir `~/Library/Caches/orbis/calib/x87`; the medium variants came from a
temporary `ORBIS_X87` flag (removed before the commit) and
`orbx-variants.py set=`.

## Step 1: what Iray computes (the MDL)

`irayubermaterial.mdl`: `diffuse_trans_mix` is a `weighted_layer` of a
Lambertian reflection (weight 1 − w, tint the base colour b) over a
Lambertian transmission (tint Translucency Color Tc) — no Fresnel, no IOR.
Closed and not thin-walled, `add_uber_volume` puts a homogeneous volume
under it: σa = −ln(Transmitted Color) / d_t, σs = −ln(SSS Color) / d_s
(chromatic, `logarithmic_scatter_enable`) or SSS Amount / d_s (mono, the
amount clamped to 1), Henyey-Greenstein at SSS Direction. The .mi of
Calib-SSSColor carries exactly these (d in m). Calib-SSS-Solid's .mi: DAZ
sets `sub_surface_enable` only with SSS Amount > 0, in chromatic mode too.

`scripts/dev/prop-volume-mc.py` is that model as a Monte Carlo: light
enters through the wall (w Tc), walks the volume, and at every return to
the wall leaves (w Tc) or is reflected back in ((1 − w) b). Its output is
the sphere's albedo under uniform light. On Calib-SSSColor the spheres
are 400 – 1000 mean free paths across in red (more in G / B): a
semi-infinite medium, thickness can't act.

## Step 2: where each sphere stands (a plain diffuse probe)

The spheres sit in a row under the HDRI; each sees other light. Octane's
render of the scene with every sphere a plain diffuse 0.5 (no
transmission) gives each place's light E_k = O_k / 0.5. Then Iray's
sphere albedo = I_k / E_k, against the Monte Carlo:

| sphere | MC albedo R / G / B | Iray / MC |
|---|---|---|
| V1 chromatic | 0.313 / 0.048 / 0.023 | 0.96 / 0.96 / 0.95 |
| V2 grey | 0.503 / 0.047 / 0.024 | 0.95 / 0.97 / 0.96 |
| V3 blue | 0.522 / 0.045 / 0.019 | 0.93 / 0.96 / 0.96 |
| V4 deep | 0.193 / 0.033 / 0.019 | 0.98 / 0.96 / 0.96 |
| V5 mono 2 | 0.518 / 0.049 / 0.025 | 0.93 / 0.96 / 0.95 |
| V6 mono 0.5 | 0.483 / 0.043 / 0.023 | 0.96 / 0.96 / 0.95 |
| V7 tint blue (= V1 in the .mi) | 0.313 / 0.048 / 0.023 | 0.96 / 0.95 / 0.95 |
| V8 no translucency (diffuse) | 0.612 / 0.268 / 0.173 | 0.96 / 0.95 / 0.95 |

Iray is DAZ's MDL, to the level of its own plain diffuse sphere. No
thickness, nothing left over. The "mono darker than grey" was the place:
V5 / V6 / V7 stand right of the middle, and V7 — V1's material — is 0.63
/ 0.65 / 0.72 × V1 in Iray.

## Step 3: why the export's ratio followed the place

The export gives the prop Universal's diffuse transmission with no
medium: light that enters crosses the empty ball and lights its whole
inside, so the ball glows by the total light it takes in, not where it is
lit. Iray's volume returns the light within a few mm of where it entered
— a diffuser. Octane / Iray by place, red: V1 0.71, V2 0.88, V3 0.97, V4
0.78, V5 1.17, V6 1.16, V7 1.07, V8 1.08 — V1 and V7 are one material.
`volume_factor`'s fit was a fit to the row.

## Step 4: Octane's scattering medium with Iray's σ

Universal's diffuse transmission (w Tc, no volume factor) over a
scattering medium (59) with σa, σs (1/m, as `scattering_pin`; isotropic):

| Octane / Iray | V1 | V2 | V3 | V4 | V5 | V6 | V7 | V8 |
|---|---|---|---|---|---|---|---|---|
| hollow (export) R | 0.71 | 0.88 | 0.97 | 0.78 | 1.17 | 1.16 | 1.07 | 1.08 |
| medium R | 0.56 | 0.60 | 0.62 | 0.56 | 0.63 | 0.56 | 0.57 | 1.08 |
| medium G | 1.13 | 1.02 | 1.04 | 1.03 | 1.02 | 1.01 | 1.12 | 1.03 |
| medium B | 0.96 | 0.95 | 0.87 | 1.03 | 0.96 | 0.97 | 0.99 | 1.07 |

The place's spread is gone; red is uniformly short. Split further against
the Monte Carlo at each place's light (Octane albedo / MC, red):

| variant | V1 | V2 | V3 | V4 | V5 | V6 |
|---|---|---|---|---|---|---|
| medium as exported | 0.54 | 0.57 | 0.58 | 0.55 | 0.59 | 0.54 |
| albedo 0 (the volume's part alone) | 0.47 | 0.58 | 0.60 | 0.36 | 0.61 | 0.53 |
| albedo 0, transmission 1 (a lossless wall) | 0.55 | 0.75 | 0.81 | 0.43 | 0.78 | 0.69 |
| the same, σa grey (red's in all channels) | 1.19 | 0.96 | 0.94 | 1.05 | 0.94 | 0.97 |
| the same, σa and σs grey (red's) | 1.00 | 0.95 | 0.94 | 1.02 | 0.94 | 0.97 |

Over a grey medium Octane is the Monte Carlo (0.94 – 1.02, Iray's level).
The deficit is spectral: Octane upsamples the σ colours to spectra, and
Victoria's Transmitted Color (0.98 / 0.48 / 0.35: σa 0.36 / 13 / 18 per
cm) leaks green's and blue's absorption into red — its effective red σa
3 – 6.5 × Iray's. The worst losses are on the thinnest red volumes (V4,
V1); chromatic σs leaks too (V1 1.19 with σa alone grey). G / B are
mostly the wall's (1 − w) b, ~1.

Not shipped: the medium is the right physics but its red is 0.56 – 0.63
on these volumes; the hollow transmitter is right on average only by the
fit. A bake (the eye's way, `eye_volume`: albedo = (1 − w) b + w Tc E
from the Monte Carlo, no transmission) would be exact for thick solids
but blind to thin ones, where the volume passes light through.

## Step 5: the thickness series (Iray batch)

Both measurement distances × k is the same volume in a ball 1 / k the
size, so `make-calib-scenes.py` builds the series on the same spheres:

- Calib-PropVol-Chromatic / -Grey / -Mono: k 1, 3, 10, 30, 100, 300,
  1000 and an empty closed ball (no volume), on V1's, V2's and V6's
  volumes. k 1 repeats Calib-SSSColor's sphere at another place.
- Calib-PropVol-Parts / -Parts-x100 (k 1, dense; k 100, thin): Tc white,
  base white, no absorption, no scattering, w 0.5, w 1, SSS Direction ±0.6.

The Monte Carlo's albedo for each (radius 15 cm), red: Chromatic 0.32 /
0.32 / 0.38 / 0.49 / 0.56 / 0.59 / 0.60 (k 1 … 1000), empty 0.60; Grey
0.51 → 0.60; Mono 0.48 → 0.60. Parts k 1: Tc white 0.46, base white 0.35,
no absorption 0.60, no scattering 0.07, direction +0.6 0.23 / −0.6 0.36,
w 0.5 0.39, w 1 0.30; k 100: 0.86, 0.62, 0.60, 0.56, 0.56, 0.56, 0.51,
0.59. A furnace albedo: a thin ball's light also comes through from
behind, so on the thin end the comparison is the renders'.

Octane side rendered (export as is and the medium variant, 1000 samples):
`~/Library/Caches/orbis/calib/x87/octane-pv/`; place probe
`x87/octane/Calib-SSSColor__grey.png` serves every scene of the series
(same spheres, camera, HDRI).

## Process

- The place in a row of spheres is a component: a scene where two
  spheres carry one material (V1 / V7) shows it at once. Compare O / I
  only for a model that responds to light like Iray's; otherwise probe
  each place's light with a plain diffuse.
- The Monte Carlo of the MDL answered "does Iray do something unknown?"
  before any Iray batch: no.
- A known input on the Octane side (a grey medium) split "Octane's medium
  is wrong" from "Octane's colours are spectral" in two renders.
