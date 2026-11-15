# 2026-11-15 — libbsdf's Phong lobe is exponent e − 1: the derived α (#162)

Resume of #162 (the parked batch: Calib-Point-UberGloss, Iray Server 2000,
c39209c). The scene: plain Uber spheres under a point light at the camera,
a dielectric gloss (reflectivity 0.5, F0 0.04) over a black base at r 0.25
/ 0.354 / 0.5 / 0.7 / 1 (U1 – U5) and a dark grey metal (base 0.1) at r
0.354 / 0.5 / 0.7 (U6 – U8). Octane enhanced-final, 1024 samples, denoised
pass; `point-highlight.py` (peak, run through the peak above 0.3 / 0.5 of
it, px), `calib-compare-all.py` (sphere middle squares), `gloss-split.py
measure` (gloss alone, core / rim, bare = Calib-DualLobeGlossy-bare).

The export changes since the parked Octane side (#163 translucency, #140
dual-lobe coating normal, #117 architectural transmit roughness, #161
strand taper) touch translucency, dual lobes with a normal map,
transparency and hair: none on these spheres, so the parked renders
(`c162/oct3`) were compared as they were.

## 1. Iray against the parked Octane side

| sphere | Iray | GGX EP (export) | STD (`brdf6=7`) | Beckmann (`brdf6=1`) |
|---|---|---|---|---|
| U1 r 0.25 | 0.381 27×26 21×20 | 0.396 27×26 21×20 | 0.400 same | 0.396 same |
| U2 r 0.354 | 0.209 38×36 29×28 | 0.228 31×30 21×21 | 0.224 36×35 27×27 | 0.224 37×36 29×29 |
| U3 r 0.5 | 0.116 53×52 42×42 | 0.135 45×44 31×30 | 0.130 50×50 39×39 | 0.133 55×54 45×45 |
| U4 r 0.7 | 0.067 71×72 63×63 | 0.086 65×65 44×44 | 0.081 65×66 55×56 | 0.091 65×65 61×61 |
| U5 r 1 | 0.040 79×84 73×78 | 0.057 91×90 70×69 | 0.054 69×77 63×72 | 0.141 rim |
| U6 metal 0.354 | 0.035 37×36 29×28 | 0.037 37×37 29×28 | same | same |
| U7 metal 0.5 | 0.017 54×52 43×42 | 0.020 65×63 59×57 | same | same |
| U8 metal 0.7 | 0.009 74×70 67×64 | 0.036 rim brighter than core | same | same |

Iray follows STD / Beckmann's width, not GGX EP's, at every r: the issue's
first outcome. But the peak is too high in **all three** models, growing
with r: STD 1.05 / 1.07 / 1.12 / 1.21 / 1.35. Head-on (light at the
camera, h = v = l, θ_d = 0) the Fresnel is F0 everywhere in both
renderers, so the excess is not #137's Fresnel: it is the lobe's D(0).

## 2. The component: D(0) of `libbsdf`'s Phong

The MDL spec cites Ashikhmin & Shirley for `simple_glossy_bsdf`'s Phong.
`libbsdf` samples the half vector as they do, cos θ = (1 − u)^(1 / (e +
1)) (`hvd_phong_sample`, already modelled in `mdl_layer::sample_h`), so
p(h) = D (n · h) = (e + 1) / 2π cosᵉ: D = (e + 1) / 2π cos^(e − 1),
Walter's normalised Phong of exponent **e − 1**. `phong_pin` used
Walter's equivalence for exponent e (α = √(2 / (e + 2)) = r / √(1 + r²)),
so Octane's Beckmann had D(0) = (e + 2) / 2π: the peak ratio is
(e + 2) / (e + 1).

| r | predicted (e + 2) / (e + 1) | STD measured | Beckmann measured |
|---|---|---|---|
| 0.25 | 1.030 | 1.050 | 1.039 |
| 0.354 | 1.059 | 1.072 | 1.072 |
| 0.5 | 1.111 | 1.121 | 1.147 |
| 0.7 | 1.197 | 1.209 | 1.358 |
| 1 | 1.333 | 1.350 | (rim) |
| metal 0.354 | 1.059 | 1.057 | |

Widths, analytic (sphere radius 46 px, L ∝ D G / 4 cos θ head-on,
`libbsdf`'s v-cavities G = min(1, 2 cos² θ), Octane's Smith-Beckmann):

| r | Iray measured | A-S cosᵉ | cos^(e − 1) | Beckmann, old α | Beckmann, α √(2 / (e + 1)) |
|---|---|---|---|---|---|
| 0.25 | 27 21 | 25 19 | 26 20 | 25 20 | 26 20 |
| 0.354 | 38 29 | 35 27 | 37 28 | 36 28 | 37 29 |
| 0.5 | 53 42 | 50 39 | 53 42 | 50 41 | 53 44 |
| 0.7 | 71 63 | 67 55 | 72 64 | 64 57 | 69 63 |
| 1 | 81 75 | 78 71 | 85 80 | 75 71 | 80 77 |

cos^(e − 1) draws Iray to a pixel, cosᵉ (D normalised over solid angle
rather than projected area) doesn't. So the derived pin is Beckmann at
**α = √(2 / (e + 1)) = r √(2 / (2 + r²))** (equal D(0)), at most 1
(reached at r √2, e = 1, where D is flat: consistent with the
saturated-anisotropy rule's "GGX at α 1 = Phong at exponent → 0"). No
constant. The single-scatter sun model (`gloss-split.py sun 0.77 0.30`)
with the new α: disc 1.30 / 1.38 / 1.40 / 1.37 at r 0.09 / 0.36 / 0.6 /
0.81 (old α 1.30 / 1.39 / 1.48 / 1.53).

## 3. Octane at the derived pin

Calib-Point-UberGloss:

| sphere | Iray | GGX EP (export) | STD | Beckmann |
|---|---|---|---|---|
| U1 r 0.25 | 0.381 27×26 21×20 | 0.385 27×26 21×20 | same | same |
| U2 r 0.354 | 0.209 38×36 29×28 | 0.215 33×32 23×22 | **0.212 37×36 29×28** | 0.212 39×38 31×30 |
| U3 r 0.5 | 0.116 53×52 42×42 | 0.121 48×47 33×33 | **0.116 53×53 43×42** | 0.121 59×57 50×49 |
| U4 r 0.7 | 0.067 71×72 63×63 | 0.071 81×82 53×53 | **0.069 71×72 63×64** | rim |
| U5 r 1 | 0.040 79×84 73×78 | 0.044 91×91 90×90 | 0.048 59×82 53×78 | rim |
| U6 metal 0.354 | 0.035 37×36 29×28 | 0.035 39×39 29×30 | same | same |
| U7 metal 0.5 | 0.017 54×52 43×42 | 0.018 69×68 66×64 | same | same |
| U8 metal 0.7 | 0.009 74×70 67×64 | 0.056 rim | same | same |
| sphere means U1 – U8 | | 1.03 0.91 0.87 0.89 0.96 1.10 1.74 4.15 | 1.02 1.01 1.02 1.07 1.15 1.10 1.74 4.15 | 1.03 1.07 1.25 1.74 2.67 … |

STD at the derived pin is Iray's lobe head-on from r 0.25 to 0.7: peaks
1.01 – 1.03, widths ±1 px. GGX EP is still GGX's narrow core (0.76 –
0.84 × the width at half the peak). U5 (r 1, α 0.82) STD is lopsided.

Calib-Point-ArchAniso (architectural, r 0.354): P1 isotropic STD 0.017
39×36 29×27 v Iray 0.018 38×35 29×26. The anisotropic spheres (P2 / P7 /
P8, `arch_aniso_pins`, a rule read with the old pin in #156) come out 10 %
long on the long axis under STD (P2 57×26 45×19 v 52×25 40×18; the old pin
51×24 39×18), while under the export's GGX EP they improve (52×22 35×16,
was 48×22 33×15).

HDRI, gloss alone (core / rim, Octane / Iray):

| sphere (Iray r) | GGX EP old → new pin | STD old → new pin |
|---|---|---|
| Calib-Glossy G7 (0.36) | 1.03 / 1.07 → 1.02 / 1.05 | 1.22 / 1.26 → 1.22 / 1.25 |
| Calib-DualLobe2 E7 (0.36) | 1.08 / 1.24 → 1.07 / 1.22 | 1.28 / 1.47 → 1.28 / 1.45 |
| E8 (0.6) | 1.10 / 1.13 → 1.04 / 1.07 | 1.46 / 1.43 → 1.40 / 1.36 |
| E4 (0.81 + lobe 1) | 1.12 / 1.25 → 1.08 / 1.22 | 1.26 / 1.45 → 1.23 / 1.42 |
| E6 (0.81) | 1.16 / 1.11 → 1.07 / 1.02 | 1.58 / 1.54 → 1.47 / 1.45 |
| sharp lobes E1 – E3, E5 (Beckmann) | 1.09 – 1.14 / 1.37 – 1.41 | same |

Whole spheres at the new pin: the export unchanged (Calib-Glossy 1.08 –
1.15 as before, Calib-NVIDIA2 0.94 – 1.07 as before, Calib-DualLobe2 1.05 –
1.11); STD Calib-Glossy G7 1.20, Calib-DualLobe2 rough 1.15 – 1.24,
Calib-NVIDIA2 A4 1.09.

## 4. Where it stands

- `phong_pin` → √(r √(2 / (2 + r²))) (`phong_alpha`), every Phong lobe
  (gloss, dual lobe, Top Coat, rough glass, the NVIDIA presets, the skins'
  OpenPBR). Shipped: head-on peaks lose their (e + 2) / (e + 1), the
  export's HDRI levels stay or improve. Rough glass and the skins were not
  re-rendered (r ≤ 0.36 moves the pin ≤ 1.5 %).
- The `brdf` choice above α 0.25 is still a trade-off, now without the pin
  in it: STD has Iray's lobe head-on and the rim level of the sharp lobes
  (1.36 – 1.45 v 1.37 – 1.41), with a core 1.22 – 1.47 × (the sun model:
  1.38 – 1.40 on the disc, the physical Fresnel and Smith against
  v-cavities); GGX EP has 1.02 – 1.08 / 1.02 – 1.22 under the HDRI with
  the narrow lobe. Asked in #162. STD would also need `arch_aniso_pins`
  re-read.
- The rough metals (U7 / U8) are off before and after (rim brighter than
  the core under a coaxial light, where Fresnel is F0 everywhere): a
  separate component, #172.

Timings: 13 Octane renders at 1024 samples, ~35 – 70 s each; exports from
cached dumps < 1 s.

## Process

- A peak excess that every Octane model shares, and that grows with r,
  is in the mapping, not in the model choice: check D(0) before comparing
  BSDF models.
- The head-on point light is the cleanest decomposition there is: Fresnel
  constant, masking ≈ 1 at the core, so peak = D(0) and width = D's shape.
