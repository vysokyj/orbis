# 2026-11-13 — Architectural's reflection on the Phong pin (#156)

Question (#156, from #146): NVIDIA Architectural's reflection still ran on
the #56 GGX curve min(r + 1.85 r², √r) (`fitted_ggx_pin`), fitted on
highlight-centred discs, with `arch_reflection_rough` (√(r · r₀), #21) on
top of it. How wrong is it on correct discs, and what replaces it?
Variant enhanced-final, Octane 1024 samples, denoised pass; Iray
references in git (Calib-NVIDIA3, -NVIDIA2, 5000 samples). No new Iray.

## 1. Reading

- `nvidia/architectural.mdl` (DAZ app bundle, `mdl 1.0`): the reflection
  is `df::simple_glossy_bsdf(roughness_u / _v: aniso.roughness_u / _v)`
  straight from `base::anisotropy_conversion(roughness:
  base::architectural_gloss_to_rough(refl_gloss), ...)` - the same df as
  every Uber lobe, so `phong_pin` (#95) applies unless Iray's pre-1.3 lobe
  (#112) or its compiled `base::` differs. DAZ's `.mi` only `$include`s
  `base.mdl` (no source, cached a114 `.mi`).
- The chain in the export: formula r → √(r · r₀) → `fitted_ggx_pin`
  under GGX energy preserving. #21's lab note (2026-11-05 nvidia-mdl-direct)
  shows how √(r · r₀) came about: a sweep of the GGX **pin** on Calib-
  NVIDIA3 (sphere centres checked by circle fit, ≤ 1 px - correct discs,
  not #146's highlight discs) found pins 0.4 / 0.25 / 0.15 / 0.05 - 0.1 at
  glossiness 0.15 / 0.3 / 0.5 / 0.7, then converted them to "MDL r"
  through the fitted GGX curve: half the formula's slope.
- `phong_pin` of the formula's own r: 0.410 / 0.287 / 0.177 / 0.109, and
  0.578 at g 0 (the old chain 0.577). That is the sweep's pins without any
  fitted step: the "half slope" was the GGX curve read backwards.

## 2. Measuring (sun spot per axis, projected discs)

Scratch `sunspot.py` (#114's method on `sphere_discs`): luminance (gamma
2.2), 1 px blur, the peak inside 0.75 of the disc, background the disc
median, the run through the peak along x / y above 0.15 / 0.3 / 0.5 of
(peak − background); level-normalised pattern error (σ 1.5 px, #21's).
Peaks of g ≥ 0.3 clip at 1 in both renderers; their contours compare.

Calib-NVIDIA3, at HEAD (fitted chain) and with the Phong pin (Beckmann /
GGX EP by `phong_brdf`):

| sphere | Iray w×h 15/30/50 % | fitted | Phong pin | peak O/I fitted → Phong | pattern err | disk |
|---|---|---|---|---|---|---|
| A1 g 0.15 | 23×27 18×20 14×15 | 18×22 12×14 9×10 | 23×27 18×20 14×15 | 1.47 → 1.07 | 0.237 → 0.017 | 1.05 → 1.07 |
| A2 g 0.3 | 14×15 11×12 9×10 | same | 14×16 11×13 9×11 | clip | 0.217 → 0.052 | 1.02 → 1.06 |
| A3 g 0.5 | 11×13 8×10 7×8 | 11×13 9×10 7×8 | same | clip | 0.096 → 0.062 | 1.05 |
| A4 g 0.7 | 10×12 7×9 7×7 | 10×12 8×10 7×8 | same | clip | 0.068 → 0.066 | 1.05 |

(floor 1.04; the glass spheres A5 - A8 unchanged, A8's coat 1.10 → 1.09.)
Calib-NVIDIA2 (floor 1.05): A3 g 0.3 14×15 11×13 10×11 Iray, fitted
14×16 10×12 8×10, Phong 14×16 11×13 10×11, error 0.213 → 0.055, disk
1.02 → 1.06; A4 g 0 the same pin, 1.00; A6 frosted glass unchanged.

So the fitted curve was materially off only at g 0.15 (a highlight half
as wide, 1.47 × as bright - between g 0 and 0.3, where the two fitted
curves bend), and within a pixel elsewhere; the derived pin is right
everywhere it can be measured.

## 3. Anisotropy

Phong pin per axis (α ≈ r per axis, ratio u / v = 4, MDL 1.3's reading):
NVIDIA2's A5 (g 0.3, a 4) 20×12 16×9 13×8 v Iray 15×14 13×11 11×9
(w / h 1.6 - 1.8 v 1.1 - 1.2), error 0.287 - worse than the old rule
(18×13 14×10 11×8). At the Phong pin of the geometric mean with the α's
in ratio √(u / v) (pins g · (u / v)^⅛, g / (u / v)^⅛): 16×15 13×12 11×10,
error 0.059, disk 1.06. #114's two candidate readings were exactly these
(α ∝ r: 4, ∝ √r: 2); this render picks 2. Committed as that, not fitted;
Calib-Point-ArchAniso / -MdlAniso (parked, #114) decide it.

## 4. The change

- `arch_reflection_rough`, `fitted_ggx_pin`, `FITTED_GGX_CURVE` gone: the
  reflection is `phong_pin` of the formula's r under `phong_brdf`; the
  glass keeps √r under Beckmann and `arch_glass_rough` (`arch_glass_pins`,
  swept on in #21, untouched here; its coat now at √ of the formula r:
  NVIDIA3 A8 / NVIDIA2 A6 unchanged).
- `arch_aniso_pins`: `phong_pin(√(u v))`, pin ratio (u / v)^⅛.
- Exports that change: Calib-NVIDIA (the mirror's `brdf` only),
  -NVIDIA2, -NVIDIA3, -Point-ArchAniso, -Mi-Arch; ArchGlass and the
  Furnace-ArchGlass scenes byte-identical.
- Calib-Point-ArchAniso's Octane side re-rendered at the new rule
  (enhanced-final; peak, w×h at 0.3 / 0.5): P1 g 0 0.021 33×30 23×21, P2 a 4
  0.023 50×20 35×14, P3 g 0.15 0.079 17×17 13×13, P4 0.084 21×10 15×8, P5
  g 0.3 0.334 9×8 7×6, P6 0.319 12×6 9×4, P7 a 0.25 0.022 27×38 19×27, P8
  rot 0.25 0.022 22×47 15×32 (was P3 0.138 11×11, P5 0.785 5×4, P2 74×14).

Timings: ~110 s per Calib-NVIDIA2 / -3 render, 60 s Point-ArchAniso.

## Process

- A rule fitted by reading measured pins back through another fitted
  curve inherits that curve: re-derive from the measured pins, not from
  the intermediate "roughness".
- Test the fully derived mapping (docs → formula → pin) before anything
  else; #95 tested `phong_pin` only on top of the fitted √(r · r₀), which
  made the derived lobe look worse.
