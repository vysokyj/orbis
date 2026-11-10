# 2026-11-10 — Octane's anisotropy, measured; the saturated lobe; v-cavities (#96)

Question (#96, after #92 and #95): Metal Brushed under Beckmann at the
derived pins had Iray's pattern but a disk 1.12 × Iray; Calib-Aniso's
metal A6 went 1.12 → 1.16; the saturated anisotropy (Iray's long axis
past 1, K3 / K4) stayed on #23's GGX curve because Beckmann at the Phong
pins blew it up (K3 1.13 → 1.73). Two components, separated before any
factor: what Octane's `anisotropyTexture` does to the lobe's axes, and
how much more light Octane's Smith masking keeps than Iray's v-cavities.

## 1. Octane's anisotropy: probe

The Octane manual says only "−1 is horizontal and 1 is vertical". So a
probe with known input (scratch `p96/build.py`, `fit.py`, `fit2.py`):
the Calib-Aniso export with its project.ocs rewritten — an orthographic
camera along the sphere row (an orthographic view's width is 2 · |pos −
target| · tan(fov / 2); the `scale` pin did nothing), the environment a
black lat-long map with one small spot (Gaussian, σ 1.5 px of 2048) near
the camera's direction, the floor at opacity 0, every other sphere hidden
(the neighbours' reflections showed at the limbs), the rest Universal
white metals (albedo 1, `metallic` 1) at chosen `brdf` / roughness /
anisotropy, rotation 0, one sphere a white Lambert. Imager as exported
(response 403, gamma 1 — gamma 2.2 on top of that doubled the curve: the
Lambert sphere came out 0.76 instead of cos at 0.82 R), denoiser off, 512
samples, ~20 s a render.

The light's direction from the Lambert sphere (least squares of I on n),
then each metal sphere fitted, per pixel, with an anisotropic Beckmann
(or GGX) lobe D · G₂ / (4 n·v), height-correlated Smith, F = 1, α along
the sphere's longitude t and latitude b free, the scale k free (should be
1 against the Lambert irradiance). First at 44 px a radius; the sharp
axes (α < 0.03) were unresolved and their peaks clipped the 8-bit PNG, so
those again at 251 px a radius with exposure 0.005 – 0.05 (divided out).

| brdf, pin, a | α_t | α_b | k | pin² | short / long | (1 − a)² |
|---|---|---|---|---|---|---|
| Beckmann 0.6, 0 | 0.361 | 0.361 | 0.994 | 0.36 | 1 | 1 |
| 0.6, 0.25 | 0.204 | 0.358 | 0.999 | | 0.571 | 0.5625 |
| 0.6, 0.5 | 0.091 | 0.358 | 0.992 | | 0.254 | 0.25 |
| 0.6, 0.75 | 0.0228 | 0.358 | 0.980 | | 0.064 | 0.0625 |
| 0.6, 0.9 | 0.0056 | 0.351 | 0.972 | | (0.016: ~1 px wide) | 0.01 |
| 0.6, −0.5 | 0.361 | 0.091 | 0.990 | | 0.25 (along t) | 0.25 |
| 0.2, 0.5 | 0.0116 | 0.0429 | 0.975 | 0.04 | 0.27 | 0.25 |
| 0.3, 0.5 | 0.0228 | 0.0881 | 0.989 | 0.09 | 0.259 | 0.25 |
| 0.45, 0.75 | 0.0133 | 0.198 | 0.977 | 0.2025 | 0.067 | 0.0625 |
| 0.8, 0.5 | 0.162 | 0.646 | 1.001 | 0.64 | 0.251 | 0.25 |
| 0.9, 0.25 | 0.459 | 0.805 | 1.000 | 0.81 | 0.571 | 0.5625 |
| 1, 0 | 0.983 | 1.003 | 1.001 | 1 | | |
| 1, 0.5 | 0.252 | 1.003 | 0.997 | 1 | 0.251 | 0.25 |
| GGX 0.6, 0 | 0.361 | 0.361 | 0.995 | 0.36 | | |
| GGX 0.6, 0.5 | 0.093 | 0.361 | 0.997 | | 0.257 | 0.25 |

Octane's lobe at roughness p and anisotropy a: **α = p² along the long
axis, p² (1 − |a|)² across**, Beckmann and GGX alike; a > 0 stretches it
along b (the texture's v on these spheres). In pins: the short axis's
pin is p (1 − |a|), so for Iray's two axes **a = 1 − pin_u / pin_v**. The
rules before: (α_v − α_u) / (α_v + α_u) (#92) and (v − u) / (v + u) of
the pins (#23) — both too narrow across (Metal Brushed's mid texel α_u
0.066 for Iray's 0.10). The rest of the table: the scale k 0.97 – 1.00 on
every resolved lobe (Octane's lobes are normalised Beckmann / GGX with
Smith), the isotropic α = p² again (#95's furnace).

Pins at 1 under Beckmann (1, 0.9 / 1, 1) didn't fit well (residual 0.3 –
0.46): the lobe covers the sphere to the limb.

## 2. The saturated lobe

Iray's anisotropy 1 gives roughness_v = r / 0.0001: Phong exponent n_v =
2 / r_v² → 0, the distribution flat along v. Walter's √(2 / (n + 2)) gives
Beckmann α → 1, which isn't flat. **GGX at α = 1 is exactly flat** (D =
α² / (π cos⁴θ (α² + tan²θ)²) = 1 / π), so is Phong at n = 0 ((n + 2) /
2π). So a saturated lobe is GGX on its long axis, by identity; on the
short axis GGX at a sharp Phong's α has its peak (#95).

Analytically (scratch `hdraniso.py`: #95's `hdrint3.py` with the lobe
per axis, Ashikhmin-Shirley Phong normalised by quadrature, v-cavities;
Octane's Beckmann / GGX with anisotropic Smith; the Ruins HDRI, floor
ignored), Calib-Aniso2's K3 (u 0.04, v 0.04 / 0.0001), Octane / Iray:
Beckmann 1.110, GGX 0.812 — Beckmann / GGX 1.37; Octane's two renders
1.53 / 1.14, ratio 1.34. Octane renders its lobes as the analytic ones;
the absolute level differs (Iray's K3 sits ~0.77 under its analytic
lobe; not chased).

Renders, Octane / Iray disk / detail / correlation (enhanced-final, 1024
samples):

| sphere | master (#95) | semantics only | + plain GGX for all saturated | committed |
|---|---|---|---|---|
| Aniso2 K3 (metal r 0.04, a 1) | 1.13 / 1.28 / 0.98 | 1.53 / 1.10 / 0.996 | 1.14 / 1.12 / 0.99 | **1.14 / 1.12 / 0.99** |
| Aniso2 K4 (weighted coat, a 1) | 0.97 / 1.00 / 0.98 | 1.32 / 0.66 / 0.98 | 1.05 / 0.89 / 0.997 | **1.05 / 0.89 / 0.997** |
| Aniso A3 (gloss a 1) | 1.13 / 1.96 / 0.85 | 1.10 / 1.09 / 0.96 | 1.13 / 1.54 / 0.92 | **1.10 / 1.09 / 0.96** |
| Aniso A5 (a 1, rot 0.25) | 1.09 / 1.29 / 0.82 | 1.09 / 0.93 / 0.91 | 1.09 / 1.00 / 0.90 | **1.09 / 0.93 / 0.91** |
| Aniso A7 (top coat a 1) | 1.11 / 2.98 / 0.95 | 1.12 / 1.35 / 0.99 | 1.13 / 2.16 / 0.97 | **1.12 / 1.35 / 0.99** |
| Aniso A8 (r 0.1, a 1) | 1.10 / 1.24 / 0.99 | 1.11 / 1.59 / 0.99 | 1.10 / 2.37 / 0.98 | **1.11 / 1.59 / 0.99** |

So plain GGX for a saturated lobe on a node that would be Beckmann (a
metal: K3, K4); a dielectric node stays on GGX energy preserving (its
long axis past `BECKMANN_DIELECTRIC_MAX_ALPHA`) — there plain GGX blurred
the streaks. A8's detail rose (1.24 → 1.59), the only sphere that got
worse in shape. The scene's level: the isotropic A1 1.09, Aniso2's K1 /
K2 1.09 / 1.09.

The unsaturated anisotropic lobes on the measured semantics:

| sphere | master | committed |
|---|---|---|
| Aniso A2 (a 0.5) | 1.08 / 0.98 / 0.97 | 1.08 / 0.88 / 0.98 |
| Aniso A4 (a 0.5, rot 0.25) | 1.09 / 1.00 / 0.95 | 1.09 / 0.91 / 0.96 |
| Aniso A6 (metal, v 0.8) | 1.16 / 1.64 / 0.97 | 1.16 / 1.08 / 0.997 |
| Aniso2 K1 / K2 (rotated) | 1.10 / 1.52 – 1.59 / 0.85 | 1.09 / 1.00 – 1.02 / 0.95 |
| Aniso2 K5 (top coat, rot) | 1.09 / 1.64 / 0.90 | 1.10 / 0.95 / 0.96 |
| Aniso2 K7 / K8 | 1.12 / 1.24 / 0.98, 1.09 / 1.54 / 0.92 | 1.12 / 1.21 / 0.98, 1.10 / 1.09 / 0.97 |
| NVIDIA Metal Brushed | 1.12 / 0.95 / 0.985 | 1.16 / 0.86 / 0.990 |

## 3. Albedo: v-cavities against Smith

Scratch `aalbedo.py`: directional albedo, single scatter, F = 1, by
quadrature over the microfacet normal; Iray's Ashikhmin-Shirley Phong
(n = 2 / r² per axis, normalised numerically) with v-cavities G = min(1,
2 n·h n·v / v·h, 2 n·h n·l / v·h) against Beckmann-Smith at α = r / √(1 +
r²) per axis. Beckmann / Iray, cosine-weighted hemispherical average /
head-on / on the sphere under the HDRI (`hdraniso.py`):

| lobe (u, v) | hemi | head-on | HDRI sphere |
|---|---|---|---|
| 0.2 iso | 1.002 | 1.000 | |
| 0.36 iso | | | 1.017 |
| 0.4 iso | 1.014 | 1.041 | 1.024 |
| 0.5 iso | 1.026 | 1.092 | |
| 0.6 iso | | | 1.072 |
| 0.7 iso | 1.057 | 1.175 | |
| 1 iso | 1.101 | 1.227 | 1.167 |
| Metal Brushed 0.04 / 0.36 | 1.004 | 1.007 | 1.006 |
| 0.1 / 0.4 | 1.006 | 1.012 | 1.008 |
| 0.18 / 0.48 | 1.010 | 1.029 | 1.013 |
| A6 0.16 / 0.8 | 1.029 | 1.079 | 1.054 |
| saturated 0.04 / 400 | 1.071 | 1.089 | 1.110 |

- A6 / A1 rendered 1.160 / 1.093 = 1.061; the analytic 1.054 / 1.001
  (A1 r 0.16) = 1.053. The effect is real and computable.
- **Metal Brushed's excess isn't it**: 1.01 on that sphere, where the
  render is 1.16 × Iray (Cast Iron 1.08, floor 1.04). The guess in #92 /
  #95 (Beckmann keeping more of a rough lobe) is wrong for this lobe —
  v 0.36 – 0.48 isn't rough enough. A Beckmann sweep of the short axis
  (α_u 0.03 / 0.066 / 0.10) under the HDRI moved this sphere 0.963 /
  0.965 / 0.962: the semantics' change (1.12 → 1.16 rendered) isn't
  explained there either; the analytic grid (128 × 256 of the HDRI) is
  coarse for α 0.03. Open, with Iray variants to take apart: #98.
- Not applied (the decision): Iray's extra loss is the v-cavities
  model's, not the surface's; Octane's Smith lobe is the more plausible
  one ("Octane over Iray"), and for the lobes in the calibration scenes
  it's ≤ 5 % (A6), ≤ 2 % for r ≤ 0.4. Only isotropic r ≥ 0.7 and the
  saturated lobes reach 1.06 – 1.17, and the factor depends on the view
  (head-on vs hemisphere up to 1.12 apart at r 1), so one weight would be
  a choice of viewing, not a derivation.
- Resin (`resin_lobe_albedo`, 0.84 at r 0.25, the "same effect, fitted"):
  the reflection's v-cavities share at r 0.25 is ≤ 1 %; the transmission
  through the rough interface wasn't computed. Not replaced.

## Regressions

`project.ocs`, installed (#95) binary against this one: byte-identical for
Calib-Glossy, -NVIDIA3, -SpecMode, -Hair, -OmniHair-blonde, -PBRSkin,
-Skin Zurie / Victoria / Monique front, -Map-TopCoat, -Map-Metal,
-DualLobe, -Eye, -Mixing, -TopCoat. Calib-NVIDIA: only the derived
anisotropy map's texels. Calib-NVIDIA2: the architectural anisotropy-4
sphere's `anisotropyTexture` (its Iray reference shows another camera,
#97): Octane new / old disk 0.975, detail 1.065. Calib-Hair-Teens (Morgan
Hair on strands, Glossy / Top Coat anisotropy 0.74 / 0.92): the gloss's
anisotropy 0.53 → 0.45, the saturated top coat roughness 1 → 0.992,
anisotropy 0.19 → 0.34, `brdf` plain GGX → GGX energy preserving
(dielectric). Rendered at 512 samples, 1280 × 720 (patches of the
2026-11-06 note, R / G / B, Octane / Iray):

| | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| master | 1.26 / 1.60 / 1.56 | 1.02 / 0.95 / 0.90 | 1.55 / 1.96 / 1.91 | 1.00 / 1.17 / 1.14 | 0.64 / 0.94 / 0.62 |
| #96 | 1.30 / 1.64 / 1.58 | 0.98 / 0.88 / 0.83 | 1.57 / 1.89 / 1.84 | 1.11 / 1.31 / 1.26 | 0.65 / 0.94 / 0.62 |
| #96, the coat plain GGX | 1.29 / 1.64 / 1.58 | 0.98 / 0.88 / 0.84 | 1.57 / 1.89 / 1.84 | 1.11 / 1.31 / 1.27 | 0.65 / 0.94 / 0.62 |

The coat's `brdf` doesn't matter; the lobes as wide across the strand as
Iray's do: the lengths rose 11 – 15 %. #52 had left the strands' gloss /
top coat too strong on the lit crown and middle; the narrower lobes had
hidden part of it. Not fitted: #99. (The first try of the hair renders
ended in Octane's "render engine failure" on the master export after a
day of renders; after a restart both rendered, 244 / 248 s.)

## Process

- A probe "with a known input" again needed its input checked first: the
  imager's gamma (2.2 on top of the export's response) and the
  orthographic width (the `scale` pin is not it) each cost a render.
- The sharp axes clip an 8-bit render long before they're resolved: set
  the exposure per lobe and zoom in.
- Rendering times: probe scenes 12 – 27 s at 128 – 512 samples; Calib-
  Aniso / -Aniso2 74 – 78 s, Calib-NVIDIA 89 – 94 s at 1024.
