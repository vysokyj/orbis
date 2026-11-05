# 2026-11-05 — Tone mapper white point (#70)

Goal: Iray's White Point in Octane ([#70](https://github.com/vysokyj/orbis/issues/70),
found in #38). Scenes: Calib-Tone-Default, -WhiteWarm ([1, 0.8, 0.6]),
-WhiteScale2 (scale 2), new -WhiteCool ([0.6, 0.8, 1]) and -WhiteGreen
([0.9, 1, 0.7]). Variant enhanced-final, 256 samples, Octane X; Iray
references at 5000 samples (Iray Server).

## What Iray does

- NVIDIA's manual (`mip_whitepoint`): a colour of the white point's hue maps
  to grey, its intensity unchanged; the custom-curve tonemapper applies
  exposure and white point first, then the curve. It doesn't say what
  "intensity" is. Iray's default `mip_whitepoint` (1.04287, 0.983863,
  1.03358) has a Rec. 709 luminance of 1.000 and a mean of 1.020 — a hint at
  luminance, but the default could be chosen that way for any reason.
- **DAZ linearises the white point before Iray sees it** (.mi export, new
  fixtures): WhiteWarm `mip_whitepoint` [1, 0.612066, 0.325037] = wp^2.2,
  Cool and Green likewise, WhiteScale2 [2, 2, 2] — the scale multiplies after
  the ^2.2. `iray::white_point` had passed the raw value: the fixtures until
  now all had a white White Point, so the test could not tell. Fixed in
  `iray.rs` from these fixtures (the #62 rule: .mi proves it).
- Iray over its Default (linear means of the unclipped pixels):
  WhiteWarm 0.651 1.062 2.001 = k / wp per channel with k 0.650 on all three.
  Candidates: mean of wp 0.646, Rec. 709 luminance of wp 0.674,
  (luminance of the raw wp)^2.2 0.660. The mean is closest (−0.6 %), but one
  hue cannot decide; White Point Scale 2 changes nothing in Iray (any k that
  scales with wp does that).

## Attempts

| try | WhiteWarm / Default R G B | Octane / Iray WhiteWarm, mean Δ sRGB R G B |
|---|---|---|
| Iray | 0.651 1.062 2.001 | — |
| white_balance = raw wp (the export before) | 0.785 1.031 1.360 | +9.1 +0.4 −20.2 (mean \|Δ\| 9.9 / 255) |
| white_balance = wp^2.2 (#38, reverted) | 0.510 1.082 1.949 | R too low |
| white_balance white, k / wp in the tone LUT (k = mean) | 0.646 1.055 1.987 | +1.2 +1.6 +1.9 (mean \|Δ\| 1.56) |

For scale: Calib-Tone-Default itself is +1.8 +1.9 +2.0 (mean |Δ| 1.92)
against Iray — Octane's usual 1.037 linear offset. Octane / Iray on WhiteWarm
is 1.029 / 1.030 / 1.025 per channel now (Default 1.037 / 1.038 / 1.038): the
channel balance matches, the remaining 0.7 % is k (0.646 vs 0.650).
WhiteScale2 over Default in Octane: 1.000 1.000 1.000 (plain export, no LUT).

Octane's white balance is some chromatic adaptation: raw wp gives R/G 0.761,
B/G 1.319 where a division would give 0.8 / 1.333; wp^2.2 gives R/G 0.471
for 0.612. Not worth inverting — the LUT already exists for #14 / #38 and
takes per-channel gains for free (the 1D LUT gets a column per channel, the
3D LUT of a luminance burn takes them before the burn, as Iray's order
says). The LUT's input headroom (16× white) leaves ≥ 8× white after the
largest gain here (2.0).

## Octane predictions for the new scenes (mean k)

Octane over its Default, unclipped pixels; Iray should give these if the
mean is right, the second line if it is the luminance:

| scene | Octane (mean k) | luminance k would give |
|---|---|---|
| WhiteCool [0.6, 0.8, 1] | 1.986 1.055 0.646 | 1.78 0.946 0.579 |
| WhiteGreen [0.9, 1, 0.7] | 0.945 0.750 1.643 | 1.156 0.917 2.01 |

Both scenes open and dump in headless DAZ (4 s each); their Iray references
are queued for the next batch (noted on #70).

## k from -WhiteCool and -WhiteGreen (Iray references, 5000 samples)

The ratio of means is mask-sensitive (WhiteWarm R 0.651 in the first pass,
0.663 with a 0.01–0.98 mask). Per-pixel medians of (S / D)^2.2 over pixels
0.15–0.9 in both images are tight (interquartile ±0.5 %) and were used from
here on.

| scene | Iray / Default R G B | k / wp → k per channel |
|---|---|---|
| WhiteWarm | 0.652 1.063 2.003 | 0.652 0.650 0.651 |
| WhiteCool | 1.634 0.867 0.531 | 0.531 0.531 0.531 |
| WhiteGreen | 1.193 0.944 2.069 | 0.946 0.944 0.944 |
| WhiteScale2 | 1.000 1.000 1.000 | — |

So the diagonal k / wp holds exactly; neither prediction did (Cool: mean
1.986 / luminance 1.78 for R, Iray 1.634). Candidates for k (Warm / Cool /
Green, measured 0.651 / 0.531 / 0.945):

| k | Warm | Cool | Green |
|---|---|---|---|
| mean of wp | 0.646 | 0.646 | 0.750 |
| Rec. 709 luminance | 0.674 | 0.579 | 0.917 |
| Rec. 601 luminance | 0.695 | 0.570 | 0.876 |
| (Rec. 709 luminance of raw wp)^2.2 | 0.660 | 0.566 | 0.908 |
| 1 / Σ Rec. 709 yᵢ / wpᵢ | 0.624 | 0.528 | 0.876 |
| **1 / Σ CIE RGB yᵢ / wpᵢ** | **0.651** | **0.531** | **0.944** |

Also tried a chromatic adaptation applied to Iray's own Default pixels
(XYZ scaling, von Kries, Bradford, CAT02 from wp to grey): the per-channel
ratios disagree with Iray by 3–25 % where the RGB diagonal is within 1 %.

The fit that settled it: three free weights in k = Σ wᵢ wpᵢ gave
0.143 0.846 −0.034 (sum 0.956, a negative weight — wrong family), in
k = Π wpᵢ^wᵢ 0.196 0.842 0.014 (sum 1.05), in k = 1 / Σ wᵢ / wpᵢ
0.177 0.813 0.010, sum 1.0001 — the Y row of CIE 1931 RGB → XYZ (0.17697,
0.81240, 0.01063). So Iray scales a white pixel's channels by k / wp so its
CIE RGB luminance stays 1. Iray's default `mip_whitepoint` gives k 0.994 by
this rule (Rec. 709 luminance 1.000), so that default was not chosen for it.

Octane (enhanced-final, 256 samples) with k on this rule, over its Default:

| scene | Octane R G B | Iray R G B | mean \|Δ\| / 255 per pixel |
|---|---|---|---|
| Default | — | — | 1.92 |
| WhiteWarm | 0.650 1.063 2.001 | 0.652 1.063 2.003 | 1.95 |
| WhiteCool | 1.634 0.869 0.531 | 1.634 0.867 0.531 | 1.83 |
| WhiteGreen | 1.190 0.944 2.069 | 1.193 0.944 2.069 | 2.05 |

Every scene is now as close as Default itself (Octane's usual +1.9 offset).
WhiteWarm's earlier 1.56 likely came from the mean's slightly low k partly cancelling
that offset.

## Lessons

- A fixture set with only default values verifies nothing about a
  conversion: the "verified" raw white point was never tested off white.
  Every option scene should get its .mi fixture at once.
- One hue fits too many formulas: WhiteWarm had the mean 0.6 % off and
  "closest"; the third scene showed it 21 % off. Fit with as many free
  weights as scenes and look at what comes out (here a known constant).
- Per-pixel medians beat the ratio of means (mask-dependent by 2 %).
- The ratio-of-means method needs the clipped pixels out (B "all pixels"
  1.805 vs 1.987 unclipped).

