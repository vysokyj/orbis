# 2026-11-10 — Metal Brushed's disk: the lobe's energy and Iray's rim (#98)

Question (#98, after #96): Calib-NVIDIA's Metal Brushed renders 1.16 ×
Iray on its disk (r < 0.75 of the sphere), Cast Iron 1.07, the floor
1.04, and #96 showed analytically that the anisotropic lobe keeps only
1.01 × what Iray's v-cavities Phong does there. Decompose before any
factor.

Measures: linear luminance (sRGB decode), rings of r / R; #92's
`detail.py` for disk / detail / correlation. Octane enhanced-final, 1024
samples (furnace probes 512); Iray Server 5000 samples.

## 1. Two components, radially

Calib-NVIDIA, Octane (#96's export) / Iray per ring:

| sphere | 0 – 0.2 | 0.2 – 0.4 | 0.4 – 0.6 | 0.6 – 0.75 | 0.75 – 0.9 | 0.9 – 1 |
|---|---|---|---|---|---|---|
| Metal Brushed | 1.12 | 1.14 | 1.12 | 1.24 | 1.45 | 1.78 |
| Cast Iron | 0.99 | 1.02 | 1.05 | 1.14 | 1.40 | 1.82 |
| concrete / polished / porcelain / granite / arch | 0.96 – 1.17 flat or falling | | | | | |

So a uniform 1.12 only on Metal Brushed and a rim excess both metals
share. Dielectrics are flat; so are the Uber metals (Calib-Aniso A6,
-Aniso2 K3, -Map-Metal M4) and the Uber weighted coat (Calib-TopCoat T3 /
T4: the same `simple_glossy_bsdf` under `weighted_layer`, no Fresnel).
Silhouettes coincide to the pixel (not geometry); the rim is the same in
all four quadrants (not the floor or the HDRI's content); Iray's and
Octane's reflections correlate 0.998 ring by ring — the same directions,
less light in Iray. Under the HDRI, Octane follows the analytic
Smith-Beckmann lobe at every radius (scratch `anarad.py`, after #95's
`hdrint3.py`: Octane / analytic 0.32 – 0.26), Iray's Cast Iron falls to
half its analytic v-cavities Phong at the rim; Iray's Uber metal M4 stays
on its analytic lobe (0.57 → 0.54, Schlick included). Cast Iron's rim
fits 1 / √cos θ_v (1.00 1.02 1.07 1.16 1.34 1.79 against 0.99 1.02 1.05
1.14 1.40 1.82).

Ruled out on paper: Octane's metal Fresnel (Gulbrandsen at edge tint =
albedo stays ≤ 1 to 72°); mip filtering of the maps in sRGB space (≤ 3 %
on these maps); Iray's own bump (CastFlat / Calib-NVIDIA: 0.97 flat).

## 2. Octane switches

Scratch hooks (not committed) on Calib-NVIDIA: disk / detail /
correlation of Metal Brushed.

| variant | disk | detail | corr |
|---|---|---|---|
| master | 1.160 | 0.89 | 0.989 |
| bump off | 1.158 | 0.88 | 0.991 |
| weight flat (mean) | 1.158 | 0.91 | 0.990 |
| lobe flat (roughness + anisotropy at the mean texel) | 1.176 | 1.02 | 0.946 |
| all flat | 1.180 | 1.02 | 0.948 |

Nothing on the Octane side moves the disk.

## 3. Iray switches: Calib-NVIDIA-Brushed

Metal Brushed on all eight spheres (one module, one DAZ session, #94),
flat maps from `calib-disp-*` (grey 128 = 0.218: weight 0.609, u 0.107 /
v 0.421). Built from Calib-NVIDIA's JSON (`make-calib-scenes.py`), its
geometry dump reused (the dump holds geometry only). Iray 289 s.

Octane / Iray at the centre (r < 0.2) and the disk:

| B1 as is | B2 bump 0 | B3 weight flat | B4 lobe flat | B5 all flat | B6 u 0.42 / v 0.35 | B7 weight 0.5 | B8 weight 1 |
|---|---|---|---|---|---|---|---|
| 1.12 | 1.12 | 1.11 | 1.12 | 1.11 | 1.12 | 1.10 | 1.11 |

All alike: not the maps, the bump or the weight. A constant
`metal_generic` keeps the whole excess.

## 4. Furnace (known input): Calib-Furnace-Metal / -Brushed

A uniform dome (our own constant RGBE map, 0.5), the floor hidden.
Calib-Furnace-Metal: white Uber metals (the metal's curve is 1 head-on
and grazing, so the reflection is `simple_glossy_bsdf` alone) at Iray r
0.05 – 1; Calib-Furnace-Brushed: Calib-NVIDIA-Brushed under the same
dome. Sphere / background per ring (Iray 60 s each):

| | 0 – 0.3 | 0.3 – 0.5 | 0.5 – 0.65 | 0.65 – 0.75 | 0.75 – 0.85 | 0.85 – 0.92 |
|---|---|---|---|---|---|---|
| Iray Uber r 0.05 | 1.000 | 1.000 | 0.999 | 0.999 | 0.999 | 1.000 |
| Iray Uber r 0.4 | 0.939 | 0.927 | 0.912 | 0.901 | 0.888 | 0.892 |
| Iray Uber r 1 | 0.520 | 0.540 | 0.573 | 0.618 | 0.678 | 0.745 |
| Octane Uber r 0.4 | 0.998 | 0.986 | 0.964 | 0.939 | 0.915 | 0.900 |
| Iray B5 (all flat) | 0.385 | 0.371 | 0.348 | 0.306 | 0.262 | 0.240 |
| Octane B5 | 0.415 | 0.412 | 0.408 | 0.385 | 0.362 | 0.377 |
| Iray B6 / Uber r 0.4 ÷ √cos θ_v | 0.965 | 0.975 | 0.978 | 0.968 | 0.969 | 0.978 |

- The furnace reproduces the disk: Octane / Iray B5 0.405 / 0.350 =
  **1.157**.
- **1.056**: the Uber r 0.4 disk, Octane / Iray (0.969 / 0.918) — the
  rough lobe's energy, Iray's Phong with v-cavities against Octane's
  single-scatter Smith-Beckmann (1.063 head-on, 1.137 at r 0.5, 1.45 at
  r 1; more than #96's analytic v-cavities loss, 1.04 / 1.09 / 1.23
  head-on). Brushed's centre 1.078 (its long axis v 0.42) is this.
- **1.10**: `metal_generic` in Iray darkens towards the rim as
  0.97 · √cos θ_v against the Uber metal with the same lobe, in every
  ring. The disk mean of 1 / √cos over r < 0.75 is 1.095. Cast Iron's
  sharp lobe has only this one (1.07 under the HDRI).
- Not in the MDL: `base::file_bump_texture` returns `normal` at factor 0
  (MDL SDK `base.mdl`), `weighted_layer` is w · layer + (1 − w) · base,
  `file_texture`'s mono is luminance(lookup · scale + offset) as the
  export bakes it, and B8 (weight 1, the base out of it) falls the same.

A dead end on the way: Octane's furnace metals read 1.02 – 1.07 × their
albedo head-on (0.2 → 0.229, 0.4 → 0.421, 0.65 → 0.666, 1 → 1.003), with
any edge tint and IOR — first read as Octane's metal Fresnel. Iray's
furnace has the same lift (weight 0.5 / 1: 0.324 / 0.614 against 0.325 /
0.65, ratio 0.53 for 0.5): the tone curves' toe, not the material.
Absolute furnace albedos at low values aren't readable through either
imager; ratios between the renderers at like levels are.

## Decision (the user's, 2026-11-10)

Both left as Octane over Iray, the export unchanged: the lobe's energy as
for the Uber lobes (#95, #96 left the v-cavities loss out); the rim
because it isn't in the MDL sources and the Uber metal with the same lobe
doesn't show it — matching it would take an OSL texture × √(n · v) on the
metal's albedo for an effect whose cause is unknown. The cause: #111 (a
hand-written MDL module with `metal_generic`'s pieces one at a time).
Regressions: none possible (no export change).

## Process

- The radial profile split the ratio into two components at once; a disk
  mean hid that one of them was shared with Cast Iron.
- Octane switches first (cheap): when none moves the ratio, go to the
  Iray side instead of tuning.
- A scene built in JSON from an existing one reuses that scene's dump
  (scene path and node labels changed) — no headless DAZ run needed.
- Iray's .png timings: Calib-NVIDIA-Brushed 289 s, the furnaces 60 s;
  Octane Calib-NVIDIA 90 s at 1024, furnaces ~30 s at 512.
