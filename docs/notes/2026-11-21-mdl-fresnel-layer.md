# 2026-11-21 fresnel_layer's reflected cosine: |n·k2_refl|

Issue #207, resumed from the 2026-11-19 note (§ 5). Calib-Furnace-MdlFresnel
rendered on the home Iray Server (5000 samples, headless submit, 0d43a45;
the first render was a bare dome, the shared MDL Rim file was written before
FRESNEL was defined, fixed in 041daa4). `furnace-rings.py measure`,
projected discs; model `cargo test --release --lib
diag207::print_mdl_fresnel_vs_iray -- --ignored --nocapture`.

## 1. Factor or cut: the factor

| Sphere | Iray disk | factor (model) | cut | Iray / factor |
|---|---|---|---|---|
| F1 1.0 w 0.6 r 0.6 | 0.404 | 0.396 | 0.416 | 1.020 |
| F2 1.3 w 0.6 r 0.6 | 0.492 | 0.484 | 0.484 | 1.017 |
| F3 1.0 w 1 r 0.6 | 0.277 | 0.268 | 0.300 | 1.034 |
| F4 1.3 w 1 r 0.6 | 0.313 | 0.301 | 0.301 | 1.040 |
| F5 1.0 w 0.6 r 0.3 | 0.723 | 0.729 | 0.739 | 0.992 |
| F6 1.0 w 0.6 r 1 | 0.216 | 0.205 | 0.224 | 1.054 |
| F7 1.0 lobe alone r 0.6 | 0.600 | 0.607 | 0.607 | 0.988 |
| F8 1.0 Schlick 0.04 w 0.6 r 0.6 | 0.602 | 0.613 | 0.612 | 0.982 |

F3 sits 3 % over the factor and 8 % under the cut: `legacy_loss` is a
factor, the cut is dead. But F2 / F4 (mdl 1.3, no legacy lobe) are off
the model as much as their 1.0 twins (F1 / F2 1.020 / 1.017, F3 / F4 1.034
/ 1.040): the residual is in the layer, not the lobe (the resume's "look at
the layer"). The controls without a critical angle (F7 lobe alone, F8
Schlick) sit 1 - 2 % under the model - the frame's scale (the dome is one
8-bit level, 114, ±1 %; flat interiors like F4's 0.321 are one level too,
±1.6 %). Against them the Fresnel spheres are 3 % (w 0.6) - 5 % (w 1)
over, growing with roughness (r 0.3 0, r 0.6 3 %, r 1 7 %), deepest at the
centre (head-on entries).

## 2. The layer weight, read again

`libbsdf` master (`curve_layer_sample` / `_evaluate`, read, not copied):
both take nk2_refl = 2 |k1·h| (n·h) − n·k1 on a refraction, the same in
sample and evaluate (MIS can't split them), base weight 1 − w ·
max(c(nk1), c(nk2_refl)). `ior_fresnel` doesn't mirror a negative cosine:
for k2_refl below the macro surface it returns ≥ 1, saturated to 1. The
2020.1.2 SDK has nk2 = |n·k2| (no reflected term). Variants (model / Iray
disk; controls 1.012 / 1.018):

| Base weight | F1 | F2 | F3 | F4 | F5 | F6 |
|---|---|---|---|---|---|---|
| max(c(nk1), c(nk2_refl)), master (model) | 0.979 | 0.984 | 0.967 | 0.962 | 1.009 | 0.948 |
| c(\|n·k2\|) at k1's side (2020 SDK) | 1.263 | 1.302 | 1.562 | 1.673 | 1.023 | 1.403 |
| c(nk1) only | 1.524 | 1.603 | 2.128 | 2.307 | 1.243 | 1.644 |
| no nk2 term when k2_refl is below the surface | 1.033 | 1.054 | 1.054 | 1.077 | 1.014 | 1.080 |
| **c(\|nk2_refl\|): mirrored below the surface** | **1.009** | **1.023** | **1.015** | **1.024** | **1.011** | **1.019** |

Only the mirrored cosine puts every Fresnel sphere on the controls' level,
rings flat (F6 0.97 – 1.08, was 0.88 – 1.03). No constant: a rule from
reading the code's one degree of freedom (how a negative cosine is taken).
DEPTH 32 → 1024 moves nothing (F4 0.301 → 0.306, noise).

## 3. Held against the earlier spheres

| Sphere | Iray | master | \|nk2_refl\| |
|---|---|---|---|
| G8 (ArchGlass, Fresnel term) | 0.412 | 0.396 | 0.408 |
| G4 / G7 | 0.609 / 0.648 | 0.608 / 0.651 | same |
| R7 grey resin r 0.5 (coat loss on) | 0.256 | 0.249 (0.972) | 0.254 (0.993) |
| R1 / R3 / R8, U4 – U7 | | | unchanged (±0.001) |

Strictly better everywhere it changes anything. Shipped in `trace_n`
(`src/mdl_layer.rs`, commented at the value) and `furnace-rings.py`; spec
§ resin / architectural layer weights.

## 4. Octane

Calib-Furnace-ArchGlass, enhanced-final, after the change (denoised pass,
projected discs): G8 0.409 v Iray 0.412 (0.993; rings 1.01 0.99 1.00 0.99
0.99 1.00 0.99 0.98 0.95), was ~3 % low with k solved against the master
model's 0.396. G1 – G7 unchanged (G4 0.611 v 0.609, G7 0.648 v 0.648).

## Lesson

The resume's two hypotheses (factor v cut) were both about the lobe; the 1.3
twins said at once it was neither. Always put a "no legacy" twin next to a
legacy sphere: it separates lobe and layer for the price of one sphere.
