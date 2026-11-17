# 2026-11-17 — Rough metal energy: STD v Iray's v-cavities (#189)

Goal (#189): the rough all-metal lobe (Octane STD at `phong_pin`, since
0e9b2b8) keeps 1.06 - 1.20 x Iray's energy in the white furnace. Take the
excess apart before any change, and see what Octane offers against it.
Variant enhanced-final (metals are STD in both Looks), 512 samples furnace /
point light, 1024 under the HDRI, denoised pass. No new Iray renders: the
references of Calib-Furnace-Metal (Iray Server 5000), Calib-Point-UberGloss
and Calib-Metal-Rough (2000) were current. Octane only, `orbx-variants.py
set=` on one export per scene; renders in `~/Library/Caches/orbis/calib/c189/`.

## 1. Both sides against their models (furnace rings)

`furnace-rings.py measure` on the renders, `model --mode reflect` (white
metal, one event per path on a convex sphere) for the models: `libbsdf`
(Phong of exponent 2 / r², v-cavities) at r, Smith-Beckmann at
`phong_alpha(r)`. Disk (rho < 0.75 R), over the dome:

| r | Iray | libbsdf model | Octane Beckmann | Smith model | Octane STD (export) |
|---|---|---|---|---|---|
| 0.3 | 0.963 | 0.974 | 0.991 | 0.988 | 0.983 |
| 0.4 | 0.915 | 0.926 | 0.963 | 0.959 | 0.945 |
| 0.5 | 0.850 | 0.862 | 0.920 | 0.918 | 0.896 |
| 0.7 | 0.714 | 0.725 | 0.824 | 0.822 | 0.789 |
| 1   | 0.556 | 0.566 | 0.690 | 0.690 | 0.650 |

Rings at r 1, centre -> 0.8 - 0.9 R: Iray 0.497 -> 0.727, libbsdf 0.502 ->
0.738; Octane Beckmann 0.626 -> 0.801, Smith 0.625 -> 0.797; STD 0.587 ->
0.774.

- Iray is the single-scatter v-cavities model, ring by ring, 0.98 - 0.99 x
  it (the 1 - 2 % left is Iray's own; it is the same at every r).
- Octane's Beckmann is single-scatter Smith-Beckmann at the export's alpha,
  exactly (as #95's furnace probe found).
- STD lies between: 1.02 / 1.03 / 1.05 / 1.10 / 1.17 x Iray on the disk,
  1.18 head-on and 1.06 at the rim for r 1. It is single-scatter too (GGX
  energy preserving at the same pins is 1.004 - 1.007 everywhere, octf
  from #172) - just a lobe that masks less than v-cavities.

So the whole excess is v-cavities against Smith masking: nothing else in
the white furnace (no Fresnel, no layering, no multiple scattering).

## 2. STD's `spread`

Octane's `spread` pin (the Universal pin dump, `orbis_probe_univ81.txt`: "the spread of the tail of the specular
BSDF model (STD only)", a plain float 0 - 1, default 0.5; not written by the
export). First try: the pin as a float *texture* node (type 31, as `set=`'s
`f` writes) - every variant 0.01 - 1 rendered identical to the export to
the third decimal: Octane ignores a texture node on that pin. `set=`'s new
`v` kind writes a float value node (type 6) and the pin takes.

Furnace (sphere means / Iray, the dome 1.02; F1 - F3, r <= 0.2, are
Beckmann and don't move):

| spread | r 0.3 | 0.4 | 0.5 | 0.7 | 1 |
|---|---|---|---|---|---|
| 0 | 1.04 | 1.07 | 1.10 | 1.18 | 1.26 |
| 0.1 | 1.04 | 1.07 | 1.10 | 1.17 | 1.26 |
| 0.5 (export) | 1.04 | 1.06 | 1.08 | 1.13 | 1.20 |
| 0.6 | 1.04 | 1.05 | 1.07 | 1.11 | 1.17 |
| 0.7 | 1.03 | 1.03 | 1.05 | 1.08 | 1.12 |
| 0.8 | 1.01 | 1.01 | 1.01 | 1.03 | 1.05 |
| 1 | 0.78 | 0.70 | 0.65 | 0.59 | 0.54 |

Spread 0 is Smith-Beckmann's level (0.684 v 0.690 at r 1); the loss grows
steeply past 0.8. Interpolating the disk, Iray's level is at spread 0.74 /
0.74 / 0.76 / 0.80 / 0.81 for r 0.3 / 0.4 / 0.5 / 0.7 / 1 - close to one
value. Rings at 0.8: r 1 0.517 -> 0.685 (Iray 0.497 -> 0.727: 1.04 head-on,
0.94 at the rim), r 0.7 0.695 -> 0.765 (1.01 / 0.94), r 0.5 0.851 -> 0.825
(1.00 / 0.96) - STD's profile is a little flatter than v-cavities', the
level is Iray's.

The other lights, default spread v 0.8 (Octane / Iray):

| scene | spheres | default | spread 0.8 |
|---|---|---|---|
| Calib-Metal-Rough (HDRI, floor 1.04 / 1.03) | R1 - R4 dark r 0.354 - 1 | 1.05 1.08 1.13 1.17 | 1.01 1.00 1.01 1.01 |
| | R5 - R7 light r 0.5 - 1, R8 gold 0.7 | 1.09 1.14 1.18 1.16 | 1.01 1.01 1.02 1.03 |
| Calib-Point-UberGloss (point light at the camera) | U6 - U8 dark r 0.354 / 0.5 / 0.7 | 1.00 1.02 1.07 | 0.97 0.95 0.98 |

Point-light widths (0.3 / 0.5 of the peak, px; Iray U6 37x36 / 29x28, U7
54x52 / 43x42, U8 74x70 / 67x64): default 37x36 / 27x28, 55x52 / 43x42,
72x72 / 64x64; spread 0.7 35x36 / 27x26, 54x52 / 41x40, 76x74 / 65x64;
spread 0.8 35x34 / 25x25, 53x52 / 39x38, 78x75 / 64x62. A longer tail and a
narrower core by 2 - 4 px; the head-on peaks agree to the third decimal.

## Where it stands

- Explained: the rough metal's excess is Iray's v-cavities masking (single
  scatter, no multiscatter compensation) against STD's; derivable in
  principle, but Octane has no v-cavities lobe.
- An Octane-side knob reaches Iray's level: STD at spread 0.8 (one value,
  measured on the furnace's directional albedo, not on a whole render)
  holds in the furnace, under the HDRI and under a point light (within
  5 %), at the cost of 2 - 4 px of highlight shape.
- Not applied. A lossless rough metal keeps 1 in the white furnace, so
  Octane's default is not less plausible than Iray's; whether rough metals
  take spread 0.8 in both Looks, in Faithful only, or not at all is asked
  in #189 (`question`). The same kind of decision as #185 (Octane keeps
  more than a single-scatter model).
- Only `metal_brdf`'s STD nodes were tried; the Faithful dielectric STD
  lobes (`rough_lobe`) keep Iray's level differently (#162) and weren't
  touched.

## Process

- A pin that changes nothing at all across 0.01 - 1 is not read: check the
  pin's type in the probe dump (`orbis_probe_univ81.txt`: `type=2`, `defaultNodeType=6`)
  before concluding the parameter has no effect.
- `furnace-rings.py model --mode reflect` gives a convex metal sphere's
  directional albedo per ring for both models in seconds; with the furnace
  render that splits Iray, Octane and the model gap without any Iray run.
