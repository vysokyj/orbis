# 2026-11-13 — Architectural's anisotropy: the ratio, and a point-lit batch (#114)

Question (#114, from #97): on Calib-NVIDIA2's re-rendered reference
(Camera-1), A5 (NVIDIA's architectural, glossiness 0.3, anisotropy 4)
matches in level but its highlight looks broader than Iray's. #112 had
just found that every NVIDIA example module is `mdl 1.0;` and that Iray
reads `simple_glossy_bsdf` with a legacy behaviour below 1.3 - a lobe
shape difference would fit too. Variant enhanced-final, 1024 samples,
judged on the denoised pass.

## 1. Reading

- `nvidia/architectural.mdl` (DAZ app bundle): the reflection is
  `simple_glossy_bsdf(roughness_u: aniso.roughness_u, roughness_v:
  aniso.roughness_v, tangent_u: aniso.tangent_u)` straight from
  `base::anisotropy_conversion(roughness: architectural_gloss_to_rough(g),
  anisotropy, rotation, mia_anisotropy_semantic: true)`. Nothing between
  the conversion and the lobe.
- The export: mia's u = r √a, v = r / √a, then `arch_reflection_rough`
  (√(· r₀), the isotropic glossiness fit) and `fitted_ggx_pin` **per
  axis**. At g 0.3, a 4: pins 0.350 / 0.148, Octane's α (pin², #96)
  5.6 : 1 - more than the MDL's own 4 : 1, because the fitted curve is
  nearly linear in pin (α ≈ r²) on each axis.
- History (`git log -S`): the per-axis rule was found when Octane's
  anisotropy was written as (v − u) / (v + u) of the pins - α ≈ 2.8 : 1
  at these pins. #96 measured Octane's real semantics (short pin = long
  · (1 − a)) and rewrote the pin for it; for architectural that turned
  2.8 : 1 into 5.6 : 1, and its A5 was then checked only Octane new vs
  old (the old reference showed another view).

## 2. Measuring the streak per axis

The issue's p99 / median over the disc doesn't say which axis; the sun's
highlight does. Scratch `sunspot.py`: luminance (gamma 2.2), the sun spot
(upper right of each sphere) from its peak, background the ring median,
the run through the peak along x and y above 15 / 30 / 50 % of
(peak − background). Peaks clip at 1 in both renderers, the contours
below the clip compare. A3 (same glossiness, isotropic) is the control.

| render | A3 w × h (0.15 / 0.3 / 0.5) | A5 w × h | A5 w / h |
|---|---|---|---|
| Iray | 14×15 / 9×13 / 7×11 | 15×13 / 11×10 / 9×8 | 1.15 / 1.10 / 1.12 |
| Octane master (α 5.6 : 1) | 16×14 / 8×11 / 6×9 | 20×10 / 14×8 / 11×6 | 2.00 / 1.75 / 1.83 |

Octane's A5 streak is longer and thinner than Iray's: too anisotropic,
not too rough. "Broader" in the issue was the long axis.

Variants at the same geometric-mean α (the isotropic pin, which A3
matches), α ratio k (a temporary env switch in `material.rs`, removed):

| k | A5 w × h | w / h |
|---|---|---|
| 1 | 14×15 / 8×12 / 5×10 | 0.93 / 0.67 / 0.50 |
| 2 | 16×13 / 9×10 / 7×8 | 1.23 / 0.90 / 0.88 |
| 3 | 18×11 / 11×9 / 8×8 | 1.64 / 1.22 / 1.00 |
| 4 | 18×11 / 11×9 / 8×7 | 1.64 / 1.22 / 1.14 |
| 5.6 (master) | 20×10 / 14×8 / 11×6 | 2.00 / 1.75 / 1.83 |
| 4, committed (`arch_aniso_pins`) | 19×11 / 12×8 / 8×8 | 1.73 / 1.50 / 1.00 |

5.6 is out. 2 – 4 all sit within a pixel of Iray at 0.3 / 0.5 (the spot is
~10 px; the 0.15 contour runs into GGX's longer tail vs Iray's Phong).
This scene can't tell 2 from 4.

## 3. The rule

**The isotropic fit belongs to the glossiness, not to each axis**:
Octane's pins g · √k, g / √k, g = `fitted_ggx_pin` of the isotropic
roughness (the geometric mean, what Calib-NVIDIA3's glossiness sweep was
fitted on), α ratio = the MDL's u / v (a above 1, √a below, by mia's
semantic) - a Phong lobe's α ≈ r per axis, the same reading the Uber's
anisotropy has (#96). Derived, not fitted; it is one of the ratios the
render allows. (The report's per-sphere column isn't read here: the
boxes split spheres, #146.)

## 4. What only Iray decides

Whether Iray's pre-1.3 lobe reads the roughness as 1.3 does (α ≈ r, ratio
4) or otherwise (e.g. ∝ √r: ratio 2) - and whether `base::` (compiled
into libneuray) converts as the MDL SDK's `base.mdl`. Two point-lit
scenes (`make-mdl-scenes.py`: Calib-Furnace-Metal's spheres, no dome, a
5000 lm point light 5 cm above the camera, so each highlight is its lobe
head-on):

- **Calib-Point-MdlAniso** (our modules `orbis_aniso_v10` / `_v13`, new
  names - none of #112's): S1/S2 isotropic r 0.2 at 1.0 / 1.3, S3/S4
  u 0.4 / v 0.1 (same geometric mean), S5/S6 `architectural_gloss_to_rough(0)`
  isotropic through `base::`, S7/S8 the same through
  `anisotropy_conversion` (mia, a 4). The `.mi` resolves all eight
  (`mdl::orbis::orbis_aniso_v1x::...`, `base.mdl` included).
- **Calib-Point-ArchAniso** (NVIDIA's Architectural preset, reduced to its
  reflection: diffuse black, reflectivity 1, Schlick 1 / 1): glossiness
  0 / 0.15 / 0.3 isotropic and anisotropy 4, glossiness 0 at anisotropy
  0.25 and at 4 rotated 0.25. Octane side rendered.

Exposure: the first ArchAniso render (EV 8.3) clipped glossiness 0.15 /
0.3 and left 0 at 0.19; regenerated at EV 11.5 (MdlAniso 9). Octane at
the committed rule, `scripts/dev/point-highlight.py` (peak, w × h at 0.3 /
0.5 of it):

| P1 g 0 | P2 g 0 a 4 | P3 g 0.15 | P4 g 0.15 a 4 | P5 g 0.3 | P6 g 0.3 a 4 | P7 g 0 a 0.25 | P8 g 0 a 4 rot |
|---|---|---|---|---|---|---|---|
| 0.020 33×32 23×22 | 0.025 74×14 55×10 | 0.138 11×11 7×8 | 0.141 23×6 17×4 | 0.772 5×5 3×4 | 0.664 9×2 7×2 | 0.022 21×50 15×34 | 0.023 15×74 11×54 |

## Process

- Look at what the issue's statistic measures before trusting a
  direction: "broader" by p99 / median was "longer and thinner" per axis.
- When a rule's input semantics change (#96: Octane's anisotropy), re-check
  every rule fitted on the old ones; architectural's per-axis curve was
  one.
