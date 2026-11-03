# 2026-11-03 Glossy anisotropy (#23)

Goal: Iray Uber's Glossy / Top Coat / Backscattering Anisotropy and their
rotations to Octane Universal's `anisotropyTexture` and `rotation`
([#23](https://github.com/vysokyj/orbis/issues/23)).
Scenes: Calib-Aniso (Iray reference of 2026-10-30, 5000 samples, Iray
Server), Calib-Aniso2 (generated this session, no Iray reference yet).
Variant enhanced-final, Octane at 400 samples, 1920 × 1080, ~30 s each.

## What Iray does

DAZ's `irayubermaterial.mdl` sends glossy, metal, refraction, top coat and
backscattering roughness through `base::anisotropy_conversion(roughness,
anisotropy, rotation)`. NVIDIA's base-module docs give only the
parameters; the body is in the open MDL SDK (`base.mdl`):

- roughness_u = r, roughness_v = r / (1 − a), r / 0.0001 at a = 1;
- the tangent turned by 2π · rotation about the normal
  (t cos φ − (n × t) sin φ).

The backscattering lobe takes `glossy_anisotropy_enable ?
backscattering_anisotropy : 0` — but DAZ's `.mi` export never writes
`backscattering_anisotropy`: not on Calib-Backscatter B8 (glossy
anisotropy off), not on Calib-Aniso2 K6 (glossy anisotropy 0.5). Iray's
backscattering stays isotropic, so nothing to export.

`top_coat_anisotropy_rotation` *is* handed over (Calib-Aniso2 K5: 0.25);
`src/iray.rs` now emits it, verified by the new fixture
`calib/references/mi/Calib-Aniso2.json`.

## What Octane does

Manual: anisotropy −1 horizontal … 1 vertical; rotation unexplained. The
pin dump (probe 104) gives `rotation` a 0 – 1 slider, so a turn like Iray's.

## Attempts

Metric (`highlight shape`, a scratch script): per sphere, the bright excess
over 0.4 of the way from median to p99.5; elongation = √ of its second
moments' eigenvalue ratio, plus mean luminance over the sphere vs Iray.
Each axis through the export's GGX curve (u = ggx(r), v = ggx(min(r/(1 − a), 1))).

| sphere | Iray elong. | H1 √(uv) | H2 (u+v)/2 | H3 v | H1 mean | H3 mean |
|---|---|---|---|---|---|---|
| A1 iso | 1.02 | 1.15 | 1.12 | 1.10 | 1.11 | 1.11 |
| A2 a 0.5 | 1.99 | 2.43 | 2.53 | 2.83 | 1.12 | 1.07 |
| A3 a 1 | 5.23 | 4.17 | 5.60 | 3.99 | 1.26 | 1.10 |
| A4 a 0.5 rot 0.25 | 1.69 | 1.41 | 1.55 | 2.03 | 1.18 | 1.10 |
| A5 a 1 rot 0.25 | 1.88 | 5.03 | 5.38 | 2.30 | 1.28 | 1.09 |
| A6 metal a 0.8 | 4.61 | 3.41 | 4.38 | 6.56 | 1.33 | 1.09 |
| A7 top coat a 1 | 7.03 | 1.67 | 1.67 | 1.67 | 1.69 | 1.69 |
| A8 r 0.1 a 1 | 9.91 | 1.96 | 6.53 | 9.85 | 1.52 | 1.15 |

(anisotropy (v − u)/(v + u) in all three.) H1 (the axes' geometric mean,
as `mdl_metal` does) left sharp bright spots where Iray draws long streaks
(A6, A8). H2 and H3 via `orbx-variants.py set=`; H3 — Octane's roughness
is the long axis, its anisotropy narrows the other — matches the streaks
by eye and the brightness of the isotropic sphere (1.07 – 1.16). The
elongation metric is noisy (the HDRI's sky lights the upper left, mid
anisotropies come out a bit long, A3 a bit short), so H3 wins on A8 and
the brightness, not a fit. The code's render: A8 10.3, means 1.07 – 1.16.

Rotation: A4 / A5 (0.25) turn the streak horizontal in Octane as in Iray,
but 90° can't tell the sign. Calib-Aniso2 K1 (0.125) and K2 (0.375)
mirror each other in Octane; which matches Iray needs its reference.

A7 (Top Coat Layering Mode 0) stays an isotropic sharp coat: Universal's
`coating` has no anisotropy. Only the weighted coat (mode 1, its own metal
node) gets it.

## Process

- The MDL body was not in DAZ's bundle nor the NVIDIA mirror; the MDL SDK
  on GitHub has it (`src/shaders/mdl/base/base.mdl`).
- `orbx-variants.py set=<material>.roughness=f…` was enough for the sweep:
  no code change per hypothesis.
- A rotation test needs an angle that isn't a multiple of 0.25.

## Open

Calib-Aniso2's Iray reference (rotation sign, K3 – K8), the Top Coat's
anisotropy outside the weighted mode, and `mdl_metal`'s anisotropy (still
the geometric mean) — in #23.
