# 2026-11-23 — PBRSkin mapped directly, verified on Genesis 9 (#57)

Goal (#57, the user's decision 2026-10-31, started 2026-11-22): map DAZ's
PBRSkin straight from `pbr_skin.mdl`, no fitted constants, verify on 2 - 3
Genesis 9 figures within ±15 %. Plan and mapping table: the issue's plan
comment.

## What the MDL says

`pbr_material`'s surface is Uber's skin without Base Color Effect:
`weighted_layer(weight 1 - w: diffuse(basecolor), base: diffuse_transmission(Tc))`
(DAZ passes `translucency_weight_inverted` with 1 - w), Makeup over it,
the GGX dual lobe on a `custom_curve_layer`, metal over all;
`pbr_add_volume` is `add_uber_volume` with the enables exposed. So the
skin colour rule derived from the layers in #19 (`skin_albedo`) holds as
it is; what changed is its input: PBRSkin's own arguments.

## .mi of a real Genesis 9 skin

Fixtures `Calib-Mi-SkinG9`, `-nosss`, `-w0` (F02's front with every mesh
but the figure hidden, figure lifted 1 cm, #228). What a G9 base skin
hands Iray, and what `iray::pbrskin` lacked:

- `transmitted_distance_of_measurement` 0.1, `sub_surface_scattering_direction`
  -0.7 (written only off the MDL's 0.12 / 0);
- `absorption_enable` = Transmission Enable, `sub_surface_enable` = Sub
  Surface Enable, `translucency_enable` = Translucency Enable (written only
  off the MDL default);
- Dual Lobe Specular Weight and Lobe 1 Roughness maps pass as maps
  (`Skin_SLW`, each skin's `_R`), not floats;
- lobe 1 roughness × **Dual Lobe Specular Roughness Mult** (mouth / teeth
  0.55 × their map) - a channel the export never read;
- `top_coat_bump_weight` 0.5 (#166 had it always absent; #231 for the
  Octane side);
- the eyes' Layered Image Editor maps arrive as DAZ temp files under the
  user's home: mi-oracle now writes them `<daz-temp>/<file>`.

Material side: `pbr_volume` builds σa / σs / g from those arguments for
`volume_albedo` and `sss_radius`. On Genesis 9's base skins the numbers
are the same as the Uber reads gave (all switches on); it changes PBRSkin
skins with Transmission or Sub Surface Enable off (before: the default
black Transmitted Color absorbed all).

## Scenes

`make-skin-g9-scenes.py`: Genesis 9 in its free base skins F02 (pale,
head map mean red 0.79), F04 (mid, 0.53), M03 (dark, 0.29), G9 Base Shirt,
head and shoulders at 130 mm, Calib-Skin's lamp positions, an 18 % grey
sphere; front / back / hdri, front-w0 / -gloss0 / -nosss, back-w0. 1.68 M
faces at SubD 2. Iray Server: 2000 samples for the nine bases (~340 s
each), 1000 for the twelve variants (100 - 140 s). Octane: 400 samples,
denoised, both Looks, 20 - 90 s each (one 545 s outlier, F04-front-w0,
unexplained, the render completed).

## Results (Octane / Iray, linear luminance, region medians)

| | front lamp | HDRI | backlit ears |
|---|---|---|---|
| Enhanced | 0.83 - 1.06 | 0.86 - 1.20 | 0.90 - 1.12 |
| Faithful | 0.85 - 1.03 | 0.89 - 1.19 | 0.60 - 0.68 (no walk) |

Decomposition, face mean of the region medians, R / G / B:

| skin | Look | -w0 (diffuse + lobes) | -gloss0 (diffuse + volume) | front |
|---|---|---|---|---|
| F02 | Enhanced | 0.96 / 1.01 / 1.01 | 0.98 / 1.05 / 1.02 | 1.01 / 1.05 / 1.04 |
| F02 | Faithful | 0.96 / 1.01 / 1.01 | 0.93 / 1.02 / 1.01 | 0.97 / 1.04 / 1.03 |
| F04 | Enhanced | 0.96 / 1.00 / 1.01 | 0.94 / 1.03 / 1.07 | 0.97 / 1.02 / 0.99 |
| F04 | Faithful | 0.96 / 1.00 / 1.01 | 0.90 / 1.00 / 1.03 | 0.94 / 1.00 / 0.97 |
| M03 | Enhanced | 0.96 / 0.98 / 1.01 | 0.83 / 1.01 / 1.04 | 0.85 / 0.94 / 0.97 |
| M03 | Faithful | 0.96 / 0.98 / 1.01 | 0.88 / 0.96 / 1.02 | 0.90 / 0.92 / 0.94 |

- The diffuse layer and the lobes are right on every skin (w0).
- The volume's part is right in Faithful within 7 - 12 % red, short in red
  the darker the skin; Enhanced's walk loses more red on M03 (0.83). M03's
  shortfall sits on the nose (gloss0 nose 0.81 both Looks, w0 nose 0.98):
  Iray's nose is redder than the forehead, light through a thin feature
  the semi-infinite rule can't show (#230, like the ears' red in #86).
- `-nosss` is not a clean "volume off": with σs 0 Iray's volume is nearly
  clear in red (mean free path 2 cm), so light entering the nose or
  cheeks leaves elsewhere; Iray's nosss red sits 11 - 17 % over (1 - w) D.
  Read front and gloss0, not front - nosss.
- The gloss alone (front - gloss0) is within the noise under this lamp.
- F02 under the dome 1.16 - 1.20 in both Looks: the skins'
  environment-light excess open since #19, now #229.

No constant fitted, nothing split by Look.

## Process

- mi-oracle crashes on dForce hair with per-fibre texture coords (#227):
  the fixture scenes keep the figure alone.
- A session ended mid-batch (disk space); the resubmit one scene per
  `submit` picked up the eleven missing variants in ~22 min.
- Back-light face regions sit at 8-bit levels of 1 - 3 in Iray: only the
  ears read.
