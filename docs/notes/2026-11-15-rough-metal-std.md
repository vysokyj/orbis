# 2026-11-15 — Rough metals: Octane's Beckmann off under a small light, STD (#172)

Issue #172 (found in #162): on Calib-Point-UberGloss (point light at the
camera, Iray Server 2000, `calib/references/iray`) the dark grey Uber metals
(base 0.1, Metallic Weight 1, Beckmann at `phong_pin`) were wider than Iray
at r 0.5 and brighter at the rim than the core at r 0.7. Octane
enhanced-final, 1024 samples, denoised pass. Variants from the cached dump
with `orbx-variants.py`; profiles: mean luminance in rings of sin θ = ρ / R
(a scratch script on `sphere_discs`), widths `point-highlight.py`, means
`calib-compare-all.py`.

## 1. What Iray does

Uber's metal is `custom_curve_layer(normal = base colour's peak, grazing 1,
exponent 5)` over a glossy lobe at colour 1 (`metallic_layered_base`,
`iray.rs`). Head-on h = v = l, so if the curve is taken at k · h it is F0
everywhere. Iray's metal profile is its dielectric's at the same r, ring for
ring (U7 v U3: 1, 0.94, 0.83, 0.68, 0.51, 0.34, 0.19, 0.075 both), although
F0 is 0.0063 against 0.04 - a curve at n · v would bend them differently.
So F is at k · h, and the profile is the lobe alone. The analytic lobe
(Phong of exponent e − 1, v-cavities) draws Iray's profile to 0.01; the
analytic Smith-Beckmann at the pin's α falls too (no rise toward the rim).

## 2. Octane, ring profile / Iray (sin θ 0 – 0.95 in tenths)

| sphere | Beckmann (export) | STD (`brdf=7`) | GGX (2) | GGX EP (6) |
|---|---|---|---|---|
| U6 metal r 0.354 | 1.01 … 1.22 (0.5) | 1.00 … 1.09 (0.5) | 1.93 peak | 1.15 peak |
| U7 metal r 0.5 | 1.05 → 3.62 (0.6) | 1.00 – 1.03 to 0.8 | 2.87 | 1.38 |
| U8 metal r 0.7 | 1.15 → 13.1 (0.75) | 1.00 – 1.10 to 0.8 | 4.26 | 1.77 |
| U4 dielectric r 0.7, Beckmann (`brdf6=1`) | 1.10 → 2.80 (0.75) | 1.01 – 1.10 | | |

- The dielectric under Beckmann rises too, less: Octane's Beckmann past α
  0.25 is not Beckmann under a small light (the HDRI limit of #95, now
  seen head-on).
- Edge tint = the metal's colour (`et`): under Beckmann U8's ring 13 → 4.6,
  U7 3.5 → 0.94; under STD (`et7`) nothing moves (≤ 0.03). So Beckmann
  weighs the metal's Fresnel (its edge to white) off the half vector; STD
  takes it at k · h, as Iray. The edge tint stays as exported (Iray's curve
  does go to white at grazing).
- A white metal (albedo / edge tint 1) clipped at this exposure: no use.
- STD's widths: U6 – U8 37×36 27×28, 55×52 43×42, 73×72 65×64 against
  Iray's 37×36 29×28, 54×52 43×42, 74×70 67×64; peaks 0.035 / 0.017 /
  0.009 = Iray's.

## 3. Furnace (Calib-Furnace-Metal, white metals, Iray 5000)

| | F1 – F3 (r 0.05 – 0.2) | F4 0.3 | F5 0.4 | F6 0.5 | F7 0.7 | F8 1 |
|---|---|---|---|---|---|---|
| Beckmann | 1.02 1.02 1.03 | 1.05 | 1.08 | 1.11 | 1.19 | 1.27 |
| STD | same | 1.04 | 1.06 | 1.08 | 1.13 | 1.20 |
| GGX EP | same | 1.06 | 1.11 | 1.20 | 1.45 | 1.88 |

STD keeps less of the excess Smith masking has over Iray's v-cavities.

## 4. Rule (shipped)

`metal_brdf`: an isotropic all-metal node whose metal α (`phong_pin`²) is
past `BECKMANN_DIELECTRIC_MAX_ALPHA` takes STD (7) at the same pin, unless a
dielectric lobe on it (Top Coat, dual lobe 2) asks for GGX EP. Sharp metals
keep Beckmann; anisotropic metals keep their rule; the weighted gloss /
coat metals and the NVIDIA presets are untouched (not measured here).
Export as shipped: Calib-Point-UberGloss means U6 – U8 1.10 / 1.74 / 4.15
→ 1.00 / 1.02 / 1.07 (dielectrics unchanged); Calib-Furnace-Metal as STD
above. No constant.

## 5. Open: the HDRI

New scene Calib-Metal-Rough (`make-calib-scenes.py`): Uber metals under the
HDRI - dark grey 0.1 at r 0.354 / 0.5 / 0.7 / 1, light grey 0.8 at r 0.5 /
0.7 / 1, gold (0.9, 0.6, 0.2) at r 0.7. Octane, STD / Beckmann (centre
disc ρ < 0.5 R, rim 0.7 – 0.95 R):

| sphere | core | rim |
|---|---|---|
| R1 dark 0.354 | 0.95 | 0.68 |
| R2 dark 0.5 | 0.85 | 0.34 |
| R3 dark 0.7 | 0.64 | 0.18 |
| R4 dark 1 | 0.43 | 0.13 |
| R5 – R7 light 0.5 – 1 | 0.96 – 0.93 | 0.95 – 0.90 |
| R8 gold 0.7 | 0.94 | 0.86 |

The dark metals differ several times over at the rim: the same off-half-vector
Fresnel, which the earlier "metals fine under the HDRI" (#95) never saw,
being white. Parked for Iray (#172): the reference decides whether STD holds
under the environment too.

Timings: 13 Octane renders at 1024 samples, 35 – 106 s; one headless dump 9 s.

## Process

- A coaxial light makes Fresnel a constant only if the renderer takes it at
  k · h: compare a metal's profile with a dielectric's at the same r before
  blaming the distribution.
- An edge-tint variant under two BRDF models tells where each takes the
  Fresnel without reading any code.
- zsh doesn't split an unquoted `$VAR` in `for m in $M`: two variants came
  out unchanged until the list was spelt out.
