# 2026-11-04 — Iray render settings → Octane kernel and post (#43)

Goal: map DAZ's Iray render settings (Progressive Rendering, Optimization,
Filtering, Alpha, Spectral, Canvases) to Octane's kernel / imager / post
processing where the look changes; document the rest as not mapped.

Scenes: `Calib-RS-*` (Calib-Glass with one render setting changed, Iray
Server references at 5000 samples from 2026-11-02), `Calib-RS-Mi-*` (new,
.mi export only: triangle filter, both path lengths, bloom settings).
Octane: Enhanced + Final, 256 samples, 1920×1080.

## Which settings change Iray's image at all

Iray reference vs Calib-Glass (sRGB 0–1):

| Scene | mean ratio | mean abs Δ | p99 abs Δ |
|---|---|---|---|
| Bloom (threshold 1.0) | 1.009 | 0.0106 | 0.161 |
| Caustics | 1.000 | 0.0001 | 0.004 |
| FireflyOff | 1.000 | 0.0000 | 0.000 |
| PathLen2 | 0.973 | 0.0101 | 0.443 |
| PathLen4 | 0.999 | 0.0002 | 0.004 |
| PixelBox | 1.000 | 0.0003 | 0.008 |
| PixelRadius3 | 1.000 | 0.0004 | 0.012 |
| Spectral | 1.000 | 0.0004 | 0.004 |

Only path length 2, bloom and (at edges) the pixel filter change it.
Caustic sampler and spectral rendering: sampling only (not mapped).

## What DAZ hands Iray

`iray::render_options`, checked against the `options` of all 158 existing
.mi fixtures (all at DAZ's defaults): Pixel Filter 2 + radius 1.5 →
`filter "gauss 3"` (width = 2 × radius), `iray_firefly_filter` true,
`iray_caustic_sampler` / `iray_spectral_rendering` / `iray_bloom_filtering`
false, `iray_max_path_length` absent at −1. The Calib-RS / -RS-Mi .mi
fixtures (exported after the Iray Server queue emptied) check the rest.

## Max Path Length

Iray: the number of path segments, 2 = direct light (NVIDIA manual).
First guess: Octane depth = N − 1 (bounces). Wrong:

| Calib-RS-PathLen2 | glass spheres | floor (lin. Y) |
|---|---|---|
| Iray | 0.000 | 0.195 |
| Octane diffuse/glossy 1/1 | 0.000 | 0.099 |
| Octane 2/2 | 0.000 | 0.203 |
| Octane 2/1 | 0.000 | 0.203 |

PathLen4: at 3/3 the glass spheres came out at half (0.100 vs 0.189), at
4/4 every sphere within 5 % (0.197 vs 0.189 …). Octane's depth counts
segments like Iray: depth = N. Glossy 1 vs 2 made no difference at N = 2
(no glass path is that short).

## Firefly filter

Off → `giClamp` 10⁶ (pin maximum, Octane's "no clamp"); on keeps the
calibrated 20. Neither Iray nor Octane (giClamp 20 vs 10⁶) changes a pixel
in Calib-Glass; kept for the meaning (Iray unclamped = Octane unclamped).

## Pixel filter

Metric: the extra Gaussian σ (px) that turns render A into render B
(least RMS over the floor / spheres band; noise floor ≈ 2.2 / 1000).

| A → B | σ |
|---|---|
| Iray Gauss 1.5 → Gauss 3 | +0.9 |
| Iray Gauss 1.5 → Box 1.5 | +0.8 |
| Octane filtersize 1.2 → 1.0 | −0.4 (noisy) |
| Octane 1.2 → 1.8 | +0.6 |
| Octane 1.2 → 2.4 | +0.8 |
| Octane 1.2 → 4 | +1.5 |

Iray Gauss of radius r: σ = r / 3 (σ 0.5 at 1.5: √(1.0² − 0.5²) = 0.87
predicted for r 3, 0.9 measured); box of width 2r: r / √3 (0.70
predicted, 0.8). Octane σ ≈ 0.38 × filtersize (1.2 → 0.46, 2.4 → 0.91:
+0.79; 4 → 1.52: +1.45). So Octane's default 1.2 ≈ Iray's default and
filtersize scales with σ. Exported: Calib-RS-PixelRadius3 +0.8 px,
-PixelBox +0.7 px over Octane's default.

## Bloom

.mi: DAZ passes `iray_bloom_filtering_radius` / `_threshold` /
`_brightness_scale` as the channels are (0.05 / 1 / 1 and 0.02 / 3 / 2).

Iray Calib-RS-Bloom (threshold 1) against Calib-Glass, linear G, fit
B = a·I + b·G_σ(I) at quarter resolution:

| σ (full-res px) | a | b | rms |
|---|---|---|---|
| 8 | −0.34 | 1.33 | 0.0061 |
| **16** | **0.03** | **0.98** | **0.0016** |
| 32 | 0.34 | 0.66 | 0.0070 |
| 96 | 0.76 | 0.25 | 0.0138 |

The whole frame is replaced by its blur, σ = 0.05 × 1920 / 6, mean ×
1.002. So the threshold is in raw (pre-tone-mapping) values — at 1 every
pixel is over it — and Iray moves, not adds, the energy. (It also took
1433 s on the Iray Server, 13 × the usual: 2026-11-02 note.)

Octane post processing on Calib-RS-Bloom (cutoff 0, 128 samples):

| variant | mean / plain | best σ | a | b |
|---|---|---|---|---|
| power 1, spread end 0.05 | 1.010 | 4 | 0.98 | 0.03 |
| power 4, end 0.05 | 1.040 | 24 | 1.02 | 0.02 |
| power 1, end 0.02 | 1.010 | 4 | 0.98 | 0.03 |
| power 1, start 0.01, end 0.05 | 1.010 | 48 | 1.00 | 0.01 |

Octane adds a weak glow; it can't do Iray's threshold-1 look. Octane's
cutoff is "measured after the exposure" (pin description), so the export
writes T / 10⁴ × exposure. The power needs Iray at a realistic threshold:
Calib-RS-Bloom2000 / -Bloom10000 generated for the next Iray batch.

### Threshold 2000 / 10000 (Iray Server, 5000 samples, 18c3a65)

No headless DAZ needed: the two scenes are Calib-Glass with other render
settings, so their dumps are Calib-Glass's with `scene` changed (the cache
name hashes the scene path).

Iray against Calib-Glass, display-linear: the mean stays (1.00007 /
0.99999), mean |Δ| 0.0060 / 0.0015, 12.8 % / 2.1 % of pixels change by
> 0.01. Fit B = I − w + k·G_σ(w), w = max(I − t, 0) per channel
(rms of Δ 0.0194 / 0.0105):

| T | t | σ px | k | rms |
|---|---|---|---|---|
| 2000 | 0.00 | 16 | 0.93 | 0.0020 |
| 2000 | **0.05** | **16** | **0.98** | **0.0011** |
| 2000 | 0.07 | 16 | 1.04 | 0.0018 |
| 2000 | 0.20 | 16 | 1.29 | 0.0121 |
| 10000 | 0.20 | 16 | 0.87 | 0.0024 |
| 10000 | **0.23** | **16** | **0.97** | **0.0009** |
| 10000 | 0.26 | 16 | 1.04 | 0.0021 |

σ 14 / 18 fit worse. The export's cutoff (T / 10⁴ × exposure 0.2475):
0.0495 / 0.247 — matches t within 5 %. Brightness scale 1 → k ≈ 1: the
three-step composite (cut, blur, add back × scale) of NVIDIA's manual,
energy-conserving at scale 1. At 2000 the floor (0.2) and the whole sky
bloom: DAZ's default is a strong effect in a scene at EV 13.

Octane (Enhanced + Final, 400 samples, Calib-RS-Bloom2000's export with
pins changed; `off` = post processing off as baseline; Octane off / Iray
plain mean 1.035). Differences smoothed σ 3 px; halo = where Iray adds
> 0.003, core = where it takes > 0.003:

| variant | mean added | halo Iray / Octane | core Iray / Octane | corr. in halo |
|---|---|---|---|---|
| cutoff 0.0495, power 1 (= export, T 2000) | +0.66 % | 0.0183 / 0.0007 (×28) | −0.0202 / +0.0019 | −0.15 |
| power 4 | +2.6 % | 0.0183 / 0.0025 (×7) | −0.0202 / +0.0075 | −0.16 |
| spreadStart 0.01 | +0.63 % | ×22 | | −0.02 |
| spreadStart 0.01, End 0.02 | +0.65 % | ×25 | | −0.02 |
| spreadEnd 0.2 | +0.64 % | ×27 | | −0.14 |
| cutoff 0.2475, power 1 (= export, T 10000) | +0.09 % | 0.0077 / 0.0002 (×37) | −0.0145 / +0.0014 | −0.08 |

Octane's glow is a broad lift of the bright sky (σ of a single-Gaussian
fit 8–64 px, at the noise level), linear in the power, nearly blind to
the spread pins, and never darkens. Matching Iray's halo would take power
≈ 28 and +18 % on the frame. **Decision: `bloom_power` = brightness scale**
(Octane's subtle glow, scaled as Iray scales its halo), cutoff and
`spreadEnd` = r unchanged — now measured, not a placeholder.

## Octane pin ranges (probe 43)

filtersize 1–8, giClamp 0.001–10⁶, maxScatterDepth 1–256, diffuse /
glossy depth 1–2048; post processing `cutoff` 0–4096, `spreadStart`
≤ 0.01, `spreadEnd` 0.001–1 (proportion of the larger image side).

## Canvases

DAZ's plug-in lists 23 canvas types (Beauty, Diffuse, Specular, Glossy,
Emission, LightGroup, EnvironmentLighting, Alpha, LightPathExpression,
Normal, Depth, Distance, MaterialID, Shadow, AmbientOcclusion,
MaterialTag, Irradiance, ObjectID, TextureCoordinate, BSDFWeight,
ConvergenceHeatmap, PostToon, WorldPosition); a scene keeps them in
`studio/element_data/iray_settings` (`render_to_canvases`, `canvases`,
`node_lists`) — no scene here has any, so the entry format is unknown.
Octane's render AOVs exist for most (NT_AOV_DIFFUSE 188, GEOMETRIC_NORMAL
198, Z_DEPTH 255, …) in a render AOV group (179, `aovCount` dynamic
inputs). Probe 43 created loose AOV nodes and saved them fine; probe 43b
(project reset, AOV group with `A_AOV_COUNT` 3, connect, save) never
started — Octane X was crashing (the user saw it). Not repeated. Not
mapped (spec says why).

## Process

- Two `octane-render.sh` runs waiting for the lock at once (this one and
  #40's) overwrote each other's `orbis_jobs.txt`; both failed with
  "failed to import". Fixed on master (a07902a, #74): the lock is taken
  before the job list.

- `timeout` doesn't exist on macOS: the first probe-43 run silently didn't
  start and the probe log read was a crashed session's old file (dated
  07:34). Check the probe log's start time.
- `scripts/dev/orbx-variants.py` got `node=<id>=<value>` for kernel /
  post pins (filtersize and depth variants without re-exporting).
