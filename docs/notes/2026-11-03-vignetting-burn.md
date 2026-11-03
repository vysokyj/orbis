# 2026-11-03 Tonemapper vignetting and per-component burn (#38)

Goal: export DAZ's Tonemapper Vignetting and Burn Highlights Per Component
to Octane ([#38](https://github.com/vysokyj/orbis/issues/38)).
Scenes: Calib-Tone-Default, -Vignette05, -Vignette1, -BurnComp-on,
-BurnComp-off (`make-option-scenes.py`; Iray references from 2026-11-02, 5000
samples on the Iray Server). Variant enhanced-final, Octane at 256 samples,
1920 × 1080, ~20 s each.

## What Iray does

NVIDIA's Iray manual: `mip_vignetting` darkens by the cosine of the ray's
angle to the film (depends on the FOV); `mip_burn_highlights_per_component`
off compresses the luminance and keeps the chromaticity.

**Vignetting**, from the references alone: linear luminance of
Vignette / Default against cos θ (θ from DAZ's `.mi` camera: focal 6.5 cm,
film 8.89 cm, 16:9), median per band of cos θ:

| cos θ | 0.806 | 0.854 | 0.907 | 0.960 |
|---|---|---|---|---|
| ratio, v 0.5 | 0.898 | 0.924 | 0.952 | 0.980 |
| ln ratio / ln cos θ | 0.499 | 0.498 | 0.502 | 0.496 |
| ratio, v 1 | 0.806 | 0.855 | 0.908 | 0.960 |
| ln ratio / ln cos θ | 1.003 | 0.994 | 0.999 | 0.990 |

So Iray: × cos(θ)^v on linear values, exactly.

**Per-component burn**, offline from the references: invert the Burn 0.5
curve per channel on BurnComp-on (gamma 2.2, then c(1 + 0.5c)/(1 + c)
solved for c), predict BurnComp-off as c · burn(Y) / Y, compare with Iray's
BurnComp-off. Where on and off differ by > 4 / 255 (142 574 px; there the
mean difference is 3.13):

| luminance | mean abs error / 255 | bias R G B |
|---|---|---|
| Rec. 709 | 0.44 | −0.18 −0.23 −0.32 |
| Rec. 601 | 0.43 | −0.15 −0.18 −0.26 |
| channel mean | 0.67 (all bright saturated px) | −0.57 −0.57 −0.79 |
| channel max | 2.69 (same) | −2.35 −2.57 −3.13 |

709 and 601 can't be told apart on this scene; 709 kept (Iray's linear
sRGB primaries). Crush and gamma per channel after the burn (not separable
here: Crush 0, Gamma 2.2).

`mip_burn_highlights_per_component` is in every Calib-Tone `.mi` fixture
(true at DAZ's default); `iray::burn_per_component` reads the channel and the
fixture test checks it. No fixture has it off yet — the mapping of off is
the channel's plain boolean.

## Octane

The manual says the imager has Vignetting (no formula) and a custom LUT
(3D LUTs mentioned); nothing per channel for highlight compression.

**Imager vignetting**, Calib-Tone-Default with the imager pin set
(`orbx-variants.py vignetting=`, `fov=`): 1 − Octane / Default on a 16 × 9
grid at v_o 1 is a cone, round in the frame's own coordinates (top-mid edge
0.63, left-mid 0.67, corners 0.92, centre 0.06 block average). Fit
1 − v_o · e, e = √(x² + y²) / √2, x and y each −1 … 1:

| v_o | slope / v_o | power of e | median residual |
|---|---|---|---|
| 0.25 | 1.003 | 1.036 | 0.0044 |
| 0.5 | 1.004 | 1.006 | 0.0039 |
| 1 | 1.004 | 1.001 | 0.0025 |

FOV 40° at v_o 0.5 gave the same ratios as 69° (to 0.003 per band): Octane's
vignetting ignores the lens. Its shape can't match Iray's — linear in
radius from the centre (Iray quadratic), elliptical (Iray round), no FOV.
Kept: v_o per camera = least squares of 1 − v_o · e against cos(θ)^v over the
frame. Predicted, Calib-Tone: v 0.5 → 0.086 (mean |Δ| 0.013, max 0.031),
1 → 0.165 (0.025, 0.058), 3 (a compact camera) → 0.436 (0.059, 0.130).

End to end (the exports' Vignette05 / 1 over Octane's Default vs Iray's):

| region | Iray 0.5 | Octane 0.5 | Iray 1 | Octane 1 |
|---|---|---|---|---|
| centre | 1.000 | 0.989 | 1.000 | 0.979 |
| left / right edge | 0.923 | 0.945 | 0.850 | 0.895 |
| top / bottom edge | 0.973 | 0.946 | 0.943 | 0.895 |
| corners | 0.904 | 0.925 | 0.819 | 0.854 |

Mean |Δ| 0.017 / 0.027 (p95 0.037 / 0.055), the frame's mean −0.3 % / −0.6 %.

Not tried: Octane's Universal camera optical vignetting (a lens-barrel
aperture effect, not a cosine law) and the Add Vignette output AOV
(compositing; the manual page has no parameters).

**3D tone LUT** for the luminance burn. Offline, trilinear reads of the LUT
against the curve (20³ colours up to 1.5, clipped to white after the read):

| size | clipped at 1 in the LUT | not clipped |
|---|---|---|
| 33 | 6.0 / 255 | 0.15 |
| 48 | 4.3 | 0.07 |
| 65 | 2.8 | 0.04 |
| 96 | 1.8 | — |

The error clipped is the clip's kink inside a cell near white; unclipped
values (up to 3.06) fix it. Kept 33³ (35 937 rows, ~1 MB in the `.orbx`).

Octane loads it (the render job's `.cube` beside the `.orbx`): BurnComp-off
vs the curve applied to Octane's own Default render 0.55 / 255 mean abs
(p95 1.65; BurnComp-on through the 1D LUT 0.53 / 1.62 — noise). Where Iray's
on / off differ by > 4 / 255: Iray off − on −2.36 / +0.48 / +6.05, Octane
−2.66 / +0.19 / +5.89 (|Δ| 0.44). Linear mean Octane / Iray: Default 1.037,
on 1.035, off 1.033.

## Lessons

- The Iray references alone settled both Iray rules (vignetting exponent,
  luminance burn) before any Octane render: same scene and sample cap, so
  ratio images are clean.
- One batch of 8 renders (3 min) measured Octane's vignetting completely;
  `orbx-variants.py` gained `vignetting=` and `fov=`.
