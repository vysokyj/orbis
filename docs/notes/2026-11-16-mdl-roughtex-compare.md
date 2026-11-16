# 2026-11-16 — The legacy rim needs a non-constant roughness at mdl 1.0 (#112)

Resume of #112 with its fourth batch rendered (Iray Server, 5000 samples,
denoiser on@8, e668bac): Calib-Furnace-MdlRoughTex (own modules
`orbis_roughtex_v10` / `_v13`, bare white lobe) and
Calib-Furnace-UberRoughMap (DAZ's Uber). Question: which property of the
roughness draws metal_generic's rim (2026-11-16 textured compare: a
roughness through `base::file_texture` = 0.93 √cos θ_v × its constant
twin), and does an Uber Glossy Roughness map do the same? Measure:
`furnace-rings.py measure`, sphere / dome per ring of the silhouette.

## 1. Calib-Furnace-UberRoughMap: the Uber is untouched

| Pair (constant / map) | 0 – 0.1 | 0.4 – 0.5 | 0.6 – 0.7 | 0.8 – 0.9 |
|---|---|---|---|---|
| metal r 0.1 (U1 / U2) | 1.000 / 1.000 | 1.000 / 0.999 | 0.998 / 0.993 | 0.998 / 0.994 |
| metal r 0.4 (U3 / U4) | 0.943 / 0.943 | 0.924 / 0.923 | 0.903 / 0.898 | 0.887 / 0.879 |
| metal r 0.7 (U5 / U6) | 0.685 / 0.685 | 0.701 / 0.699 | 0.734 / 0.724 | 0.815 / 0.791 |
| dielectric r 0.4 (U7 / U8) | 0.039 / 0.039 | 0.038 / 0.037 | 0.038 / 0.037 | 0.053 / 0.055 |

Map / constant 0.97 – 1.00 in every ring (U6's outer rings −2 to −3 %,
within what G6 showed at 1.3); the constants equal Calib-Furnace-Metal
F2 / F5 / F7. **A Glossy Roughness map doesn't rim the Uber (MDL 1.7)**:
store materials are not affected. Octane's twins were already equal
(2026-11-16 textured compare § 3), so nothing changes there.

## 2. Calib-Furnace-MdlRoughTex: what property, which version

Per ring, ratio to the constant twin (M3 = 1.0 constant r 0.4, M4 = 1.3,
MdlLobe L2 / L7 = 1.0 constant r 0.1 / 0.7), and ÷ √cos θ_v at the
ring's midpoint:

| Sphere | twin | 0.05 | 0.25 | 0.45 | 0.65 | 0.75 | 0.85 | ÷ √cos at 0.05 → 0.85 |
|---|---|---|---|---|---|---|---|---|
| J1 1.0, `file_texture` r 0.4 | M3 | 0.903 | 0.905 | 0.879 | 0.828 | 0.786 | 0.719 | 0.90 → 0.99 |
| J2 1.3, `file_texture` r 0.4 | M4 | 1.000 | 0.997 | 0.994 | 0.987 | 0.982 | 0.981 | — |
| J3 1.0, `tex::lookup_float` | M3 | 0.903 | 0.906 | 0.878 | 0.820 | 0.774 | 0.705 | 0.90 → 0.97 |
| J4 1.0, varying 1e-4, no texture | M3 | 0.903 | 0.906 | 0.880 | 0.824 | 0.777 | 0.704 | 0.90 → 0.97 |
| J5 1.3, varying 1e-4 | M4 | 1.000 | 0.999 | 0.998 | 0.990 | 0.987 | 0.985 | — |
| J6 1.0, tint via `file_texture`, r constant | M3 | 1.000 | 1.000 | 0.999 | 0.998 | 0.994 | 0.992 | — |
| J7 1.0, `file_texture` r 0.1 | L2 | 1.059 | 1.043 | 0.995 | 0.903 | 0.826 | 0.704 | 1.06 → 0.97 |
| J8 1.0, `file_texture` r 0.7 | L7 | 0.912 | 0.910 | 0.907 | 0.901 | 0.888 | 0.860 | 0.91 → 1.18 |

- **Any non-constant roughness at mdl 1.0**: a file texture, a plain
  lookup and a roughness varying by 1e-4 without any texture give the
  same rings (J1 = J3 = J4 within 0.015). Not the texture, not
  `base::file_texture`: a roughness the compiler can't fold.
- **Only at 1.0**: at 1.3 the textured and the varying roughness are the
  constant lobe (J2, J5: 0.98 – 1.00) — the legacy `simple_glossy_bsdf`
  path, which with a constant roughness stays close to 1.3 (MdlLobe, 0.98
  to r 0.3).
- **Only the roughness**: the tint through a texture at a constant
  roughness changes nothing (J6).
- **Its shape depends on the roughness**: "0.93 √cos θ_v" was r ≈ 0.4's.
  At r 0.1 the textured lobe *gains* 6 % head-on (1.039 of the dome, the
  1.3 lobe 1.000) and falls to 0.69 at the rim; at r 0.7 it loses 9 %
  head-on and stays flatter. J1's centre equals the constant r 0.5 lobe's
  (L6 0.834) but its rim is far darker (0.593 v 0.749): it isn't a wider
  lobe.

Against the 1.3 lobe (Calib-Furnace-Metal F2 / F7, -MdlVersion M4: what
Octane's metal already approximates), the textured legacy lobe is (θ_v
3° → 64°): r 0.1 1.04 → 0.69, r 0.4 0.88 → 0.67, r 0.7 0.83 → 0.66.

## 3. Scope among the presets the export maps

Of DAZ's NVIDIA modules (all `mdl 1.0` but `core_definitions`), only
`metal_examples` (Metal Brushed, Cast Iron) feeds `simple_glossy_bsdf` a
roughness through a texture; the mapped dielectrics (concrete …
architectural) take it as a parameter. So the rim concerns exactly the
two NVIDIA metals; resin's and architectural glass's roughness is a
constant, and #117's legacy table (constant roughness) holds for them.

## 4. Decision: a Look split

A texture lookup changing a BSDF's angular shape isn't physical, and
Octane's metal (flat, as Iray's own 1.3 / Uber lobe) is the plausible
one: **Enhanced keeps it; Faithful draws Iray's rim**
(`Tuning::legacy_metal_rim`). Octane: albedo and edge tint under a Mix
by a Falloff Map (normal vs. eye ray, (θ / 90°)^skew), k head-on, k ·
grazing at 90° — the construction #117 verified on the transmission. A
metal sphere in the furnace reflects once, so its rings are the per-hit
factor at the camera's θ_v and the shape is fitted to the ratio rings
directly (grid, r-weighted over each ring's width):

| r | k | grazing | skew | worst ring |
|---|---|---|---|---|
| 0.1 | 1.039 | 0.115 | 2.25 | 0.5 % |
| 0.4 | 0.894 | 0.325 | 2.30 | 1.0 % |
| 0.7 | 0.836 | 0.420 | 2.30 | 0.6 % |

(`LEGACY_METAL_RIM`, linear in r between, held at the ends), at the
mid-grey texel's mean roughness of the two axes.

## 5. Octane renders (enhanced-final / faithful-final, denoised)

Calib-Furnace-Brushed (NVIDIA's `metal_brushed`, B1 – B8, all with a
textured roughness), rings Octane / Iray:

| Sphere | Enhanced disk | Faithful 0 – 0.1 … 0.8 – 0.9 | Faithful disk |
|---|---|---|---|
| B1 brushed | 1.18 | 0.99 0.99 0.99 0.99 1.00 1.00 1.00 1.01 1.02 | 1.00 |
| B4 lobe flat | 1.18 | 0.99 0.99 0.99 0.99 1.00 1.00 1.00 1.01 1.03 | 1.00 |
| B6 near iso, flat | 1.18 | 0.98 1.00 1.01 1.00 1.00 1.00 0.99 0.98 0.98 | 1.00 |
| B7 weight black | 1.17 | 0.98 0.98 0.98 0.99 0.99 0.99 1.00 1.00 1.01 | 0.99 |
| B8 weight white | 1.17 | 0.98 0.98 0.98 0.99 0.99 1.00 1.00 1.01 1.02 | 1.00 |

Enhanced is the flat metal as before (B6's rings 0.366 → 0.322, Iray
0.342 → 0.221). Faithful is Iray's ring for ring (0.98 – 1.03): the
fitted 1.3-lobe ratio also takes the disk's former 1.056 "lobe energy"
(#98), which was the legacy lobe's head-on loss, not Octane's.

Lit (`calib-compare.py regions`, Octane / Iray): Calib-NVIDIA-Brushed's
metal cubes Enhanced 1.24 / 1.80 / 1.22, Faithful 1.02 / 1.08 / 0.98,
the rest of the scene unchanged (1.03 – 1.04). Calib-NVIDIA (no textured
NVIDIA metal on its cubes) equal in both Looks.

Real scenes: the Demo-* scenes' metals are Uber, and §1 shows an Uber
roughness map has no rim in Iray, so neither Look changes them.

## Where it stands

`Tuning::legacy_metal_rim`: Enhanced flat (Octane over Iray, #111's
reading of it as not physical), Faithful the legacy rim on NVIDIA's
`metal_examples` metals. #112 closes. Open: the 1.0 lobe at r < 0.1 with
a textured roughness (held at r 0.1's shape), an empty roughness map
(unmeasured; no falloff written), whether Octane clamps the 1.039 gain.

## Process lessons

- G5's "varying" roughness (`0.4 + 0.0 * u`) folded and hid the answer
  for a round: a "non-constant" variant must vary (J4's 1e-4).
- "0.93 √cos θ_v" was one roughness's curve; measure the law at two or
  three parameter values before naming it.
