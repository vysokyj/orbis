# 2026-11-14 — mia's anisotropy as Iray renders it (#114 resume)

Resume of #114 with its batch in (91b1388): Calib-Point-MdlAniso and
Calib-Point-ArchAniso, Iray 2000 samples, a point light 5 cm above the
camera (each highlight its lobe head-on). Octane enhanced-final, denoised
pass. Measure: `scripts/dev/point-highlight.py` (peak, run through the
peak above 0.3 / 0.5 of it, w × h px, projected discs).

## 1. Decomposition: lobe vs conversion

Our own modules (`orbis_aniso_v10` / `_v13`), Iray:

| sphere | MDL 1.0 | MDL 1.3 |
|---|---|---|
| iso r 0.2 (S1/S2) | 0.307 21×20 17×16 | 0.322 21×20 16×16 |
| u 0.4 v 0.1 written straight (S3/S4) | 0.351 40×10 31×8 | 0.351 41×10 32×8 |
| `architectural_gloss_to_rough(0)` iso (S5/S6) | 0.121 35×35 27×27 | 0.113 37×36 29×28 |
| same through `base::anisotropy_conversion` (mia, a 4) (S7/S8) | 0.114 51×25 40×19 | 0.103 57×25 45×20 |

- The version doesn't touch the shape (S1 = S2, S3 = S4): NVIDIA's `mdl 1.0`
  lobe reads u / v per axis as 1.3 does.
- Per axis the width is linear in r: u 0.4 = 2 × the iso 0.2 → 40 (2 × 20),
  v 0.1 → 10. So the lobe is the Uber's per-axis Phong lobe (#96).
- Through `base::` at a 4 the ratio is 2.0 – 2.3, not 4. The MDL SDK's
  `base.mdl` (mia branch, read on GitHub): above 1 iso = √(1/a), u = r /
  iso, v = r · iso (ratio a); below 1 iso = a^¼, u = r · iso, v = r / iso
  (ratio √a). The export had copied that. Iray's compiled `base::`
  renders ratio √a above 1 too.

The preset confirms it at a small roughness, where the sphere doesn't
saturate the long axis (Calib-Point-ArchAniso, Iray): P3 g 0.15 iso
17×17 13×13, P4 g 0.15 a 4 25×12 19×10 — u 1.47 r, v 0.70 r (a^¼ = 1.41;
the SDK's 2 r / 0.5 r would be ~34×8). P7 a 0.25: 25×48 19×38, u / v 0.5
(both readings agree below 1).

## 2. The rule

u = r · a^¼, v = r / a^¼ on both sides of 1, each axis at its Phong pin
(as the Uber's anisotropy); `arch_aniso_pins` (pins of the geometric
mean in ratio (u / v)^⅛, #156) gone. Nothing fitted: the choice between
the SDK's a and Iray's √a is discrete, read off the decomposition.

## 3. Octane at the rule (Calib-Point-ArchAniso)

| sphere | Iray | #156 rule | this rule |
|---|---|---|---|
| P1 g 0 iso | 0.018 38×35 29×26 | 0.021 33×30 23×21 | 0.021 32×30 22×21 |
| P2 g 0 a 4 | 0.020 52×25 40×18 | 0.023 50×20 35×14 | 0.024 44×20 31×14 |
| P3 g 0.15 iso | 0.082 17×17 13×13 | 0.079 17×17 13×13 | 0.077 17×18 13×14 |
| P4 g 0.15 a 4 | 0.084 25×12 19×10 | 0.084 21×10 15×8 | 0.081 **25×12 19×10** |
| P5 g 0.3 iso | 0.346 9×8 7×6 | 0.334 9×8 7×6 | 0.335 9×8 7×6 |
| P6 g 0.3 a 4 | 0.331 12×6 9×4 | 0.319 12×6 9×4 | 0.325 11×6 9×5 |
| P7 g 0 a 0.25 | 0.020 25×48 19×38 | 0.022 27×38 19×27 | 0.023 22×44 15×31 |
| P8 g 0 a 4 rot 0.25 | 0.019 27×48 21×38 | 0.022 22×47 15×32 | 0.022 23×44 17×31 |

- P4 now exactly Iray's; P6 within a pixel.
- The g 0 spheres (r 0.354) are all narrower in Octane, and so is their
  isotropic twin P1 (0.82 × at 0.3 of the peak, 1.17 × the peak): that is
  the isotropic Phong pin at large r, not the anisotropy. Normalised by
  P1, P2 is 1.37 × / 0.67 × (Iray 1.37 × / 0.71 ×; #156's 1.52 × / 0.67 ×)
  and P7 0.69 × / 1.47 × (Iray 0.66 × / 1.37 ×; #156's 0.82 × / 1.27 ×).
  Filed separately (#162).
- P8 (rotation 0.25) turns the streak upright in both: the rotation's
  sign is moot at a quarter turn; the export's `1 − rotation` stands.
- Calib-NVIDIA2 (sun + dome), sun spot per axis above 0.15 / 0.3 / 0.5
  of (peak − disc median), 1 px blur (scratch script, not #156's exact
  one): A5 g 0.3 a 4 Iray 12×13 10×10 9×8, Octane 13×13 10×11 9×8; A3
  14×16 11×13 8×10 v 14×16 11×13 9×11. Highlight spread / Iray mean
  A5 1.06 (#156 1.06).

Timings: ~60 s Point-ArchAniso, ~110 s NVIDIA2 (Octane 1024 samples).

## Process

- Own MDL modules that run one piece through Iray (a lobe written
  straight, the same through `base::`) split "the lobe" from "the
  conversion" where the preset alone couldn't: the #156 reading (√ in
  the lobe) gave the right answer at a 4 for the wrong reason and showed
  it at a 0.25.
- Read ratios at a roughness where the sphere doesn't clip the long axis
  (g 0.15 here); at g 0 the streak runs over half the disc.
- The SDK's `base.mdl` is not what Iray's libneuray computes: a
  measured `base::` beats the source.
