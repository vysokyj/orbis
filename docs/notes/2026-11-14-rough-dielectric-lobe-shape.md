# 2026-11-14 — The rough dielectric Phong lobe's shape under a point light (#162)

Question (#162, from #114): every glossiness-0 sphere of
Calib-Point-ArchAniso (MDL r 0.354, point light at the camera) renders
in Octane 0.82 × as wide and 1.17 × as bright at the peak as Iray,
the isotropic P1 included; at g 0.15 / 0.3 the Phong pin matches to a
pixel. Suspected: `phong_pin` saturating at large r. Octane
enhanced-final, 1024 samples, denoised pass; `point-highlight.py` (peak,
run through the peak above 0.3 / 0.5 of it, w × h px).

## 1. Not the pin: the `brdf` switch

The export writes P1 – P2 / P7 – P8 (r 0.354, pin 0.578, α 0.334) at
**`brdf` 6 (GGX energy preserving)**, P3 – P6 (α 0.163 / 0.062) at
Beckmann: `phong_brdf` leaves Beckmann above dielectric α 0.25
(`BECKMANN_DIELECTRIC_MAX_ALPHA`, #95). The narrow spheres are exactly
the GGX ones. Analytically, at the same α (same D(0)) GGX's core is
narrower than Beckmann's / Phong's: half maximum at tan²θ = 0.414 α²
(GGX) against 0.693 α² (Beckmann), width ratio 0.77; at 0.3 of the peak
0.83. Phong of exponent 2 / r² (n 16) vs Beckmann at α 0.334: Beckmann
4 % wider at half maximum.

Octane variants of the same export (`orbx-variants.py brdf=<n>`, every
sphere), Calib-Point-ArchAniso:

| sphere | Iray | GGX EP (6, export) | GGX (2) | Beckmann (1) | STD (7) |
|---|---|---|---|---|---|
| P1 g 0 iso | 0.018 38×35 29×26 | 0.021 33×30 23×21 | 0.018 33×31 23×22 | 0.018 39×36 31×28 | 0.018 **38×36 29×27** |
| P2 g 0 a 4 | 0.020 52×25 40×18 | 0.024 48×22 33×15 | 0.020 45×21 31×14 | 0.020 51×26 42×20 | 0.020 51×24 39×18 |
| P7 g 0 a 0.25 | 0.020 25×48 19×38 | 0.024 22×44 15×31 | 0.020 21×43 15×30 | 0.020 26×49 20×40 | 0.020 25×49 19×38 |
| P8 g 0 a 4 rot 0.25 | 0.019 27×48 21×38 | 0.022 23×44 16×31 | 0.018 23×44 17×31 | 0.018 27×50 20×41 | 0.018 27×49 19×38 |
| sphere mean P1 / P2 (Octane / Iray) | | 1.05 / 1.07 | 0.89 / 0.89 | 1.00 / 1.02 | 1.00 / 1.00 |

- The width is GGX's distribution (plain GGX as narrow), the peak's
  +17 % its energy compensation (plain GGX has Iray's peak).
- Beckmann and STD (default `spread`, not written) at the same pin draw
  Iray's lobe head-on; STD to a pixel, also anisotropic.

## 2. Why GGX EP was chosen: the same spheres under the HDRI

`orbx-variants.py brdf6=<n>` (new: only the nodes exported at `brdf` 6).
Gloss alone (`gloss-split.py measure`, bare = Calib-DualLobeGlossy-bare),
Octane / Iray, core / rim:

| sphere (Iray r) | GGX EP | Beckmann | STD |
|---|---|---|---|
| Calib-Glossy G7 (0.36) | 1.03 / 1.07 | 1.22 / 1.92 | 1.22 / 1.26 |
| Calib-DualLobe2 E7 lobe 1 (0.36) | 1.08 / 1.24 | 1.28 / 2.24 | 1.28 / 1.47 |
| E8 (0.6) | 1.10 / 1.13 | 1.61 / 2.82 | 1.46 / 1.43 |
| E4 lobe 2 (0.81) + lobe 1 | 1.12 / 1.25 | 1.42 / 2.14 | 1.26 / 1.45 |
| E6 lobe 2 only (0.81) | 1.16 / 1.11 | 1.97 / 3.21 | 1.58 / 1.54 |
| sharp lobes (G2 – G4, E1 – E3) | 1.08 – 1.15 | same | same |

Whole sphere, Calib-NVIDIA2 A4 (g 0, sun + dome): 1.00 / 1.11 / 1.09.

The single-scatter sun model (`gloss-split.py sun 0.77 0.30`, θ_d 56°,
Octane's Beckmann with Universal's physical Fresnel against `libbsdf`'s
Phong + v-cavities + Schlick) predicts 1.30 / 1.39 / 1.53 on the disc at
r 0.09 / 0.36 / 0.81, of which the Fresnel (#137: physical over Schlick,
kept) is 1.30 at the peak. The sharp lobes measure 1.08 – 1.15 against
the model's 1.30; STD's 1.22 / 1.28 at r 0.36 sits in the same
proportion to 1.39 (the Fresnel choice, nothing new), its 1.46 – 1.58 at
r 0.6 – 0.81 above it. GGX EP lands at 1.03 – 1.16 by its compensation,
with the wrong lobe.

So no single Octane model has both: GGX EP has the level under side
light and a 0.8 × core head-on; STD has Iray's core head-on and (at
r ≈ 0.36) the level the accepted Fresnel predicts, too bright at r ≥
0.6; Beckmann off at the rim from r 0.36 on (#95's rough-dielectric
excess).

## 3. Iray at r 0.5 – 1 (parked)

Calib-Point-ArchAniso holds only r 0.354. Calib-Point-UberGloss (new,
`make-mdl-scenes.py`): the same light on plain Uber spheres, a
dielectric gloss over a black base at r 0.25 / 0.354 / 0.5 / 0.7 / 1 and
a dark grey metal (base 0.1; Beckmann at any r, the control) at r 0.354
/ 0.5 / 0.7, EV 3.4. Octane side (1024 samples):

| sphere | GGX EP (export) | STD (`brdf6=7`) | Beckmann (`brdf6=1`) |
|---|---|---|---|
| U1 diel r 0.25 (Beckmann in all) | 0.396 27×26 21×20 | same | same |
| U2 diel r 0.354 | 0.228 31×30 21×21 | 0.224 36×35 27×27 | 0.224 37×36 29×29 |
| U3 diel r 0.5 | 0.135 45×44 31×30 | 0.130 50×50 39×39 | 0.133 55×54 45×45 |
| U4 diel r 0.7 | 0.086 65×65 44×44 | 0.081 65×66 55×56 | 0.091 65×65 61×61 |
| U5 diel r 1 | 0.057 91×90 70×69 | 0.054 69×77 63×72 | 0.141 47×74 43×29 (rim) |
| U6 metal r 0.354 | 0.037 37×37 29×28 | same | same |
| U7 metal r 0.5 | 0.020 65×63 59×57 | same | same |
| U8 metal r 0.7 | 0.036 45×74 41×19 (rim brighter than the core) | same | same |

(The disc is ~92 px across: widths above ~65 run into the rim.) Here
GGX EP's peak is not above STD's (F0 0.04: little to compensate);
Calib-Point-ArchAniso's white lobe (F 1) had +17 %.

Resume: if Iray's U2 – U4 widths follow STD (as P1 did), the choice is
between GGX EP (level) and STD up to some r (shape, level as the
Fresnel predicts) — U2 / U3 / U4 say up to where STD stays on Iray's
lobe; the metals check Beckmann's shape at large r with no switch.

Timings: ~60 s per Point scene, ~70 s per HDRI scene (1024 samples);
DAZ dump 6 s.

## Process

- The issue's hypothesis (the pin's large-r end) was one look at the
  exported `brdf` pins away from the cause: read the export's node
  values for the failing spheres before anything else.
- A point light at the camera measures only θ_d = 0; the HDRI scenes'
  gloss split (core / rim) the oblique half angles. A rule chosen on one
  hides its cost on the other — measure both before switching models.
