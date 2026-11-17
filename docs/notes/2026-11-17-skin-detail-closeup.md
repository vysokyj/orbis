# 2026-11-17 Skin detail contrast at a close framing (#179)

#47 shipped Enhanced's skin detail contrast at s = 0.15 and measured it on
the Calib-Skin fronts at their 960 × 720 framing, where a face is ~200 px
wide and the pores average out under a pixel. #179 asks for the final s,
judged on real figures at a framing that resolves the skin. This session
renders that framing at s = 0, 0.15, 0.3 and 0.5 and leaves the choice to
the user (a look call, not a fit).

## Setup

- Three Calib-Skin fronts (private store skins, numbers only): Victoria
  (light), Twosret, Monique (dark).
- One Enhanced export per s: HEAD for 0.15, a scratch worktree of HEAD
  with only `ENHANCED_SKIN_DETAIL_CONTRAST` changed for 0, 0.3, 0.5
  (separate target dir; no switch in the code).
- The face close-up on the Octane side only: `orbx-variants.py` gained
  `shift=<x>,<y>` (the thin lens's `lensShift`); `fov=11.71&shift=0,0.4167`
  is a 2.5 × zoom centred 120 px above the 960 × 720 frame's centre (the
  export's fov 28.76°), the face ~500 px wide. No Iray at this framing:
  the figures are private (no Iray Server) and s is a look, not a match.
- 960 × 720, 400 samples, Enhanced final, denoised pass: 116 – 206 s a
  render, 12 renders ~30 min.

## Measured (denoised, log-luminance bands over s = 0)

Bands of `skin-texture.py` (fine = L − G(1.2 px), mid = G(1.2) − G(4));
regions of the close framing: forehead (400 – 560, 110 – 170), cheeks
(300 – 380 / 580 – 660, 360 – 430), chin (430 – 530, 570 – 610).
fine / mid, each region's σ over its s = 0 σ:

| | s 0.15 | s 0.3 | s 0.5 |
|---|---|---|---|
| Victoria forehead | 1.06 / 1.11 | 1.24 / 1.24 | 1.84 / 1.50 |
| Victoria cheeks | 1.11 – 1.26 / 1.06 – 1.13 | 1.29 – 2.04 / 1.10 – 1.49 | 1.73 – 2.45 / 1.36 – 1.85 |
| Victoria chin | 1.50 / 1.26 | 1.65 / 1.30 | 2.31 / 1.55 |
| Monique forehead | 1.59 / 1.38 | 6.83 / 2.53 | 11.5 / 3.42 |
| Monique cheeks | 1.30 – 1.68 / 1.24 – 1.43 | 2.28 – 3.00 / 1.89 – 2.26 | 2.87 – 3.65 / 2.14 – 2.70 |
| Monique chin | 1.01 / 1.10 | 1.26 / 1.36 | 1.46 / 1.59 |
| Twosret forehead | 1.03 / 1.04 | 1.15 / 1.12 | 1.31 / 1.28 |
| Twosret cheeks | 1.25 – 1.35 / 1.10 – 1.12 | 1.24 – 1.32 / 1.17 – 1.22 | 1.43 – 1.63 / 1.30 – 1.48 |
| Twosret chin | 1.10 / 1.09 | 1.14 / 1.17 | 1.18 / 1.26 |

Mean linear RGB per region stays within 1 – 3 % at every s (levels and
hue unchanged, as the bake intends).

## Decomposed: albedo detail or the walk's grain?

The rendered texture is the albedo's detail plus the random walk's grain
minus what the denoiser removes. The forehead's σ in the raw pass and the
denoised one (fine / mid / a coarse G(4) − G(8) band):

| forehead | s 0 | s 0.15 | s 0.3 | s 0.5 |
|---|---|---|---|---|
| Monique raw | 0.141 / 0.038 / 0.018 | 0.144 / 0.040 / 0.019 | 0.147 / 0.041 / 0.019 | 0.147 / 0.043 / 0.019 |
| Monique denoised | 0.009 / 0.011 / 0.015 | 0.015 / 0.015 / 0.016 | 0.064 / 0.027 / 0.017 | 0.109 / 0.036 / 0.018 |
| Victoria raw | 0.076 / 0.020 / 0.009 | 0.076 / 0.020 / 0.010 | 0.077 / 0.022 / 0.010 | 0.076 / 0.023 / 0.010 |
| Victoria denoised | 0.005 / 0.007 / 0.008 | 0.006 / 0.008 / 0.009 | 0.007 / 0.009 / 0.009 | 0.010 / 0.010 / 0.009 |
| Twosret raw | 0.118 / 0.035 / 0.012 | 0.119 / 0.036 / 0.012 | 0.122 / 0.036 / 0.014 | 0.128 / 0.040 / 0.014 |
| Twosret denoised | 0.089 / 0.030 / 0.011 | 0.092 / 0.031 / 0.012 | 0.102 / 0.033 / 0.013 | 0.116 / 0.038 / 0.014 |

- **The raw pass barely moves with s** (+0 – 8 %): at 400 samples the
  walk's grain is the texture on screen, 5 – 15 × the denoised σ.
- **The denoiser decides, and not linearly.** Monique's forehead: it wipes
  everything at s 0 – 0.15 (fine 0.009 – 0.015), then at s 0.3 it starts
  keeping texture (0.064) and at 0.5 nearly the raw grain (0.109 of
  0.147). Twosret's denoised forehead keeps most of the grain already at
  s = 0 (0.089 of 0.118). On screen at s ≥ 0.3 Monique's skin reads as a
  grey speckle, lighter and ashier, though its mean colour is unchanged.
- So the close-up's fine band at 400 samples is not the albedo's detail
  alone: part of what s "adds" is grain the denoiser now lets through.
  The 1600-sample pair below separates the two.

## 1600 samples: the denoiser's step was the sample count

Monique and Twosret again at s 0.15 and 0.3, 1600 samples (449 – 527 s a
render). fine / mid σ, raw → denoised:

| | s 0.15, 400 | s 0.15, 1600 | s 0.3, 400 | s 0.3, 1600 |
|---|---|---|---|---|
| Monique forehead | 0.144 / 0.040 → 0.015 / 0.015 | 0.089 / 0.030 → 0.072 / 0.028 | 0.147 / 0.041 → 0.064 / 0.027 | 0.092 / 0.032 → 0.084 / 0.031 |
| Monique cheek L | 0.165 / 0.040 → 0.028 / 0.014 | 0.095 / 0.029 → 0.066 / 0.024 | 0.168 / 0.045 → 0.050 / 0.022 | 0.098 / 0.032 → 0.058 / 0.026 |
| Monique cheek R | 0.164 / 0.041 → 0.025 / 0.014 | 0.096 / 0.029 → 0.060 / 0.025 | 0.166 / 0.045 → 0.043 / 0.021 | 0.097 / 0.031 → 0.081 / 0.028 |
| Monique chin | 0.150 / 0.038 → 0.009 / 0.012 | 0.087 / 0.028 → 0.017 / 0.017 | 0.153 / 0.041 → 0.011 / 0.015 | 0.087 / 0.027 → 0.024 / 0.019 |
| Twosret forehead | 0.119 / 0.036 → 0.092 / 0.031 | 0.090 / 0.031 → 0.089 / 0.031 | 0.122 / 0.036 → 0.102 / 0.033 | 0.095 / 0.033 → 0.093 / 0.032 |
| Twosret cheeks | 0.128 / 0.038 → 0.069 – 0.100 / 0.027 – 0.032 | 0.081 / 0.030 → 0.062 – 0.069 / 0.027 – 0.030 | 0.128 / 0.040 → 0.073 – 0.091 / 0.030 – 0.034 | 0.083 / 0.032 → 0.068 – 0.074 / 0.031 |

- At 1600 samples the raw pass drops toward the skin's own texture (grain
  would halve: Monique 0.144 → 0.089, not 0.072) and the denoiser keeps
  nearly all of it already at s 0.15 (Monique's forehead 0.015 at 400 →
  0.072 at 1600). Monique's s 0.15 at 1600 looks like her s 0.3 at 400.
- Converged, s 0.15 → 0.3 is a modest step: fine +17 % (Monique's
  forehead), −12 – +35 % (cheeks), Twosret +2 – 10 %. What made s 0.3 look
  like a jump at 400 samples was the denoiser crossing from smoothing to
  keeping, which more samples do as well.
- Even at 1600 Monique's s 0.3 reads a touch greyer (ashier) than 0.15:
  mean colour unchanged, so it is the speckle's own colour. Not checked:
  whether the per-channel ratio's blue / green detail is relatively larger
  on a dark map (a less saturated speckle) - a per-channel band σ of the
  baked albedo would tell, if a channel-coupled ratio (luminance D / D_low
  on all three) is ever considered.

## Left for the user

A look call: the side-by-side sheets (face crops at s 0 / 0.15 / 0.3 /
0.5 at 400 samples; s 0.15 / 0.3 at 1600) and the full frames are local
(private figures, not in git), paths in #179. What decides it is how
crisp skin should be at the samples a user renders with: the texture on
screen depends on the sample count (the denoiser) as much as on s.

## Process

- `orbx-variants.py fov=…&shift=…` gives a close-up of any export without
  a new DAZ scene or dump; zoom z = tan(fov / 2) / tan(fov' / 2), a point
  (dx, dy) px off centre lands at shift (z dx / W, z dy / H).
- Read the raw pass beside the denoised one before reading a texture
  change: a denoiser that flips from smoothing to keeping is a step, not
  a slope.
