# 2026-11-08 — The remaining fitted skin constants, decomposed (#83)

#19 replaced the skin's fitted hue with rules from DAZ's MDL; each fit
had hidden a missing component. Left: `SKIN_HUE`, `SKIN_ALBEDO_FACTOR`
and `OPENPBR_SKIN_LIFT` (Enhanced's path for the Genesis 8 base skin:
mono SSS, Base Color Effect 1, SSS Direction −0.65, w 0.5) and the square
root in `volume_factor`. The rule: split before fitting.

Octane 256 samples, Enhanced + Final. Octane / Iray, linear, R / G / B:
Calib-Skin-* the median of the face and body regions, Calib-Eye /
Calib-HDRI the median over the G8F skin's 20 px blocks (the #19 cmp.py,
work dir `~/Library/Caches/orbis/calib/s83`). Store figures: numbers only.

## The Genesis 8 base skin

| Enhanced | fitted (`skin_tint` × `SKIN_HUE` × lift) | baked (`skin_albedo`) | baked, anisotropy 0 |
|---|---|---|---|
| Calib-Eye | 1.01 / 0.95 / 1.07 | 1.13 / 1.17 / 1.22 | 1.05 / 1.07 / 1.11 |
| Calib-HDRI | 1.04 / 0.96 / 1.06 | 1.16 / 1.16 / 1.22 | 1.09 / 1.07 / 1.10 |
| Skin-Base front | 0.86 / 0.75 / 0.69 | 1.04 / 1.11 / 1.14 | 0.95 / 1.00 / 1.00 |
| Skin-Base back | 0.86 / 0.75 / 0.67 | 1.08 / 1.14 / 1.14 | 1.00 / 1.00 / 1.00 |
| Skin-Base HDRI | 1.00 / 0.90 / 1.00 | 1.14 / 1.14 / 1.22 | 1.05 / 1.05 / 1.11 |

Faithful (the same bake as a plain diffuse): front 0.96 / 1.00 / 1.00,
Calib-Eye 1.06 / 1.07 / 1.10, HDRI 1.06 / 1.05 / 1.12.

Steps:
1. The bake as is (column 2): the hue is right — R, G, B level — where
   the fit was 25 – 30 % short in G / B under a lamp. #28's "1.4 – 2.0 ×
   Iray baked" predates #19's diffuse depth, mapped gloss weight, mean
   free path and gamma; it is ~10 – 20 % over now, evenly.
2. Enhanced / Faithful on the same bake: 1.08 / 1.11 / 1.14 front, 1.08 /
   1.09 / 1.09 HDRI. Red over as much as blue, so not #84's dark-albedo
   floor (that is additive and spares a 0.2 albedo).
3. The skin's parts (a debug print of `skin_albedo`): the diffuse layer a
   ≈ 0.31 / 0.26 / 0.19, the subsurface b 0.04 / 8e-5 / 2e-5
   (Translucency Color 0.30 / 0.03 / 0.01). The skin is a diffuse with a
   little red volume. Iray's -sssdir0 variant (−0.65 → 0): 0.99 / 1.00 /
   1.00 of the base — Iray doesn't see g on this skin.
4. Octane does: all of the colour goes through the walk, with
   `subsurfaceAnisotropy` −0.65. At 0 (column 3) Enhanced = Faithful
   within 1 %. The bake already has g (the volume's reduced albedo
   `σs (1 − g)`); the walk's own anisotropy changed the albedo it was
   asked to reproduce. Now `subsurfaceAnisotropy` 0 on every skin.

Nishi is the only Calib-Skin figure with g (−0.5 / −0.35): front 0.96 /
1.04 / 1.07 → 0.92 / 0.98 / 0.95, HDRI 1.06 / 1.09 / 1.16 → 1.03 / 1.03 /
1.08, back (body) 1.00 / 1.00 / 1.00 → 1.00 / 0.88 / 0.77, backlit ears
0.60 / 0.65 / 0.70 → 0.90 / 0.53 / 0.39. Front and sky better, backlight
mixed — g shapes where the light leaves, not only how much.

Shipped: every skin baked (`skin_tint`, `SKIN_ALBEDO_FACTOR`, `SKIN_HUE`,
`openpbr_skin_lift` / `OPENPBR_SKIN_LIFT` removed), anisotropy 0.

Open (#86): the G8F skin's backlit ears 2.29 / 1.61 / 1.30 × Iray
(fitted: 0.89 / 0.81 / 0.75; baked with g 1.43 / 1.44 / 1.52). Iray's
subsurface part is a few percent, the walk takes all of the colour at the
volume's long red mean free path. Candidate: the walk's weight = b / (a +
b) — not tried.

## The square root in `volume_factor`

On skin it is gone since #19 (A(α) in `skin_albedo`); it is left on props
(Universal's diffuse transmission × (Rd(α) / Rd(α₀))^0.5 ×
`PROP_VOLUME_LEVEL` 0.7). Calib-SSSColor (Uber spheres on Victoria 8's
volume, w 0.9; Iray in git):

| sphere | α | Iray | √Rd (now) O / I | A(α) / A(α₀) O / I |
|---|---|---|---|---|
| V1 chromatic | 0.89 / 0.81 / 0.69 | 0.145 / 0.025 / 0.015 | 0.71 / 1.05 / 1.16 | 0.71 / 1.05 / 1.16 |
| V2 grey | 0.99 / 0.80 / 0.74 | 0.218 / 0.023 / 0.015 | 0.89 / 1.13 / 1.17 | 1.11 / 1.15 / 1.22 |
| V3 blue | 1.00 / 0.77 / 0.30 | 0.203 / 0.020 / 0.011 | 0.97 / 1.21 / 0.87 | 1.22 / 1.21 / 0.64 |
| V4 deep | 0.67 / 0.37 / 0.30 | 0.072 / 0.013 / 0.010 | 0.79 / 1.07 / 1.10 | 0.58 / 0.93 / 1.00 |
| V5 mono 2 | 1.00 / 0.91 / 0.88 | 0.165 / 0.019 / 0.012 | 1.30 / 1.56 / 1.61 | 1.67 / 1.88 / 2.26 |
| V6 mono 0.5 | 0.99 / 0.72 / 0.65 | 0.143 / 0.015 / 0.010 | 1.17 / 1.33 / 1.23 | 1.47 / 1.26 / 1.15 |

The skin's own model (`skin_albedo` with irradiance from V8, w 0) under-
predicts Iray's spheres 1.6 – 2.0 × on chromatic, 1.1 – 1.3 × on mono.
And no function of α alone fits: V5 (α ≥ V2's in every channel, σs 133
/ cm) is darker in red than V2 (σs 53 / cm). A denser volume passes less
through the sphere — a thickness, which a figure's semi-infinite skin
doesn't have and α doesn't carry. A(α) spread the spheres wider (0.58 –
2.26) than the root (0.71 – 1.61). Kept, with the why in the code; the
derived rule needs the volume itself (#87: a random walk on props).

## Process

- Two variants first (fit vs bake) before touching any constant: the bake's
  even excess said "one component", not "a hue".
- The skin's parts printed from `skin_albedo` on the real material settled
  where g can act (only b) in a minute.
- `ORBIS_X83` env flags for variants during the session, removed before the
  commit.
