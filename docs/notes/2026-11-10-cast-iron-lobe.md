# 2026-11-10 — Cast Iron's detail: the metal lobe, not the bump (#92)

Question: since #91 (heights decoded at 2.2) Calib-NVIDIA's Cast Iron
(`grunge_d.jpg`, 2048 px, bump 1.5, an ~94 px sphere) had a band-passed
detail 1.39 × Iray (1.21 × with the heights linear), while in Iray the
decode moved it 6 %. Candidates in #92: the map minified ~7 – 14 texels a
pixel (Iray filtering, our full-resolution bake), the same map as
roughness, the weight map `grunge_b`. Decompose before any fit.

Measure (scratch `detail.py`, #89's): linear luminance; disk mean (r 0.75
of the sphere); centre box ±20 px, box blur r 2 − r 8, std over the mean,
Octane / Iray, and the correlation with Iray's band. This session's
reproduction of #91: Iray BumpG1 / Calib-NVIDIA detail 0.942, correlation
0.984 (#91: 0.94 / 0.98). Octane enhanced-final, 1024 samples; probe
switches as environment variables in a scratch build (not committed).

## Octane: what each part does

Cast Iron, Octane / Iray (Calib-NVIDIA):

| variant | disk | detail | corr |
|---|---|---|---|
| master | 0.954 | 1.304 | 0.761 |
| bump off | 0.922 | 1.659 | 0.625 |
| bake low-passed to 512 / 256 / 128 px | 0.915 | 1.65 | 0.63 |
| roughness map flat (l 0.132, its mean at 2.2) | 0.955 | 1.297 | 0.772 |
| weight map flat (l 0.090) | 1.001 | 1.211 | 0.856 |
| both flat | 1.003 | 1.185 | 0.863 |
| both flat, bump off | 0.964 | 1.503 | 0.698 |
| bump × 1.5 / × 2 / × 3 | 0.996 / 1.041 / 1.120 | 1.138 / 1.068 / 0.970 | 0.875 / 0.916 / 0.880 |

- The bump **lowers** the detail: its normal spread is sub-pixel and
  blurs the reflection. All of it lies above 512 px — a bake low-passed
  to the sphere's footprint renders as the bump off. Prefiltering would
  move away from Iray.
- × 2 the bump came closest (1.07, 0.92) — read as "Iray spreads the
  normals twice as much" at first. That was the lobe (below).
- The roughness map's variation doesn't matter; the weight map adds some.

## Is anything filtered?

- Octane doesn't mip-filter the normal map: `octane-probe-bumpfreq.lua`
  (Info kernel, one sample a pixel, no pixel filter) on the export, the
  shading normal with / without bump at 7.5 × 14 texels a pixel: tilt RMS
  0.0586 per pixel = the baked map sampled bilinearly at each pixel's UV
  (16-bit UV pass) 0.0586; one mip level down would give 0.034, two 0.019.
  (The kernel's `lodBias` attribute exists, but the normal map isn't
  LOD-filtered here.) The 16-bit passes need a real 16-bit reader — PIL
  reads them as 8 bits, 1/256 of the UV.
- Iray's slope at texel scale is the bake's: Calib-BumpGamma-Uber's step
  band (128 → 160, f 4), the tilt profile across the step, Iray vs Octane:
  integral of |R − 1| 1.169 / 1.171 (16-bit card 1.059 / 1.067), minimum
  0.425 / 0.431; Iray's profile sits ~½ texel lower in v (MDL's texel i at
  u = i / W, no half-texel offset), shape and width alike. So Iray uses
  MDL's blended central differences, not the interpolant's gradient
  (that would have been ~2.4 × the slope on white noise).
- Iray's texture compression: DAZ hands every map above the scene's
  1024 px threshold at `compression 2` ("high"; grunge_d, brushed_a, the
  4096 px dimples), 512 – 1024 px at 1. A simulated 4-level 4 × 4 block
  code on grunge_d adds +6 % to the slope RMS — too little.

## Iray: Calib-NVIDIA-CastFlat

Calib-NVIDIA with Cast Iron's Bump Strength 0 (`.mi`: `bump_amount 0`),
Iray Server 5000 samples, 131 s, a freshly started queue DAZ (#94).

- Iray's own bump moves Cast Iron's detail by **9 %** (CastFlat /
  Calib-NVIDIA: disk 0.970, detail 1.088, correlation 0.973).
- Octane bump-off against Iray bump-off: disk 0.950, detail **1.524**,
  correlation 0.703. The excess is in the lobe without any bump.

## The lobe

MDL: `metal_generic` is `simple_glossy_bsdf`, Phong of exponent 2 / r²
(MDL spec 21.1.3) = Beckmann at α = r / √(1 + r²); Octane's pin is √α
(rough-glass note 2026-11-05, backscatter probe, `openpbr_roughness`).
The export put it on the Uber gloss's GGX curve min(r + 1.85 r², √r) —
fitted to highlight peaks (2026-10-31), ≈ r for a sharp lobe, α ≈ r².
Cast Iron's r ≈ 0.06 + 0.4 · 0.13 = 0.11: pin 0.137 against the derived
0.335.

Octane, bump off, against CastFlat; the pin × k (diagnostic only):

| | disk | detail | corr |
|---|---|---|---|
| k 1 | 0.950 | 1.524 | 0.703 |
| k 1.17 | 0.965 | 1.438 | 0.741 |
| k 1.5 | 0.999 | 1.253 | 0.823 |
| k 2 | 1.059 | 0.989 | 0.927 |
| k 3 | 1.122 | 0.671 | 0.886 |
| **Beckmann at √(r / √(1 + r²))** | 1.080 | **1.007** | **0.996** |
| Beckmann at √r | 1.081 | 1.003 | 0.995 |
| plain GGX at √(r / √(1 + r²)) | 1.121 | 0.768 | 0.976 |

Metal Brushed got worse at every k (its long axis already at the curve's
√r cap): the error depends on r. Derived, not swept: Beckmann at the
derived pin; GGX at the same α blurs (its long tail isn't Phong's), as
the rough-glass note found at r 0.09.

Anisotropy: Metal Brushed's u 0.04 – 0.18 / v 0.36 – 0.48 on the derived
pins, (v − u) / (v + u) of the pins (#23's rule): disk 1.216, detail 0.630,
correlation 0.867 — the streak broadened (the pins' ratio is the root of
the α's). Of the α: 1.116 / 0.951 / **0.990** (plain GGX, same: 1.032 /
1.010 / 0.982). Octane's anisotropy semantics weren't probed (#96).

## Result (Calib-NVIDIA, the committed rule)

| sphere | disk | detail | corr |
|---|---|---|---|
| Metal Brushed | 0.997 → 1.116 | 1.099 → 0.949 | 0.949 → 0.991 |
| Cast Iron | 0.954 → 1.073 | 1.304 → 1.040 | 0.761 → 0.997 |
| concrete, polished, porcelain, granite, resin, arch | unchanged (±0.01) | | |

Floor 1.04. Brushed's disk 1.12 is the open part: Beckmann keeps more of
a rough lobe than Iray's v-cavities Phong without multiple scattering
(resin's `resin_lobe_albedo` is the same effect, fitted); the GGX curve
had been tuned on this sphere. Not fitted: #96.

Regressions: Calib-CoatBump, -BumpRes, -BumpGamma-Uber / -Uber2 /
-NVIDIA, Calib-Skin Zurie / Monique / Victoria front, Calib-NVIDIA2 / 3 —
`project.ocs` byte-identical to the installed binary's (only `mdl_metal`
changed; #85's skin numbers stand). Calib-NVIDIA's `.ocs` differs only in
the metals' `brdf`.

## Not used

Calib-NVIDIA-NoComp (texture compression off, `.mi` verified: no
`compression` on any map) was prepared for the batch and not rendered:
with the lobe derived, Cast Iron's bump matches Iray, so compression
needs no answer here.

## Follow-ups

- #95: the Uber's gloss and coat and the NVIDIA dielectrics are the same
  Phong lobe on the same fitted curve.
- #96: Metal Brushed's brightness under Beckmann; Octane's anisotropy on α.

## Process

- The "× 2 bump" match was a decoy: a stronger bump blurs the reflection
  as a broader lobe does. One Iray reference with the bump off split it.
  The first table already showed the bump *lowering* the detail — that
  should have pointed at the reflection's sharpness at once.
- Rendering times: Calib-NVIDIA at 1024 samples 80 – 90 s; the Info probe
  ~5 s; Iray CastFlat 131 s.
