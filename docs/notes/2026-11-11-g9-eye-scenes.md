# 2026-11-11 — Genesis 9 eye: scenes, the MDL and a prediction (#66)

Picks up [2026-11-01-g9-eye-wip.md](2026-11-01-g9-eye-wip.md). Goal: verify
`eye_volume` (the G9 eye's diffuse and Translucency Color maps baked into
one albedo, weights 1 − t / t, no transmission) against Iray. No Iray
render in this session (issue loop: the batch is parked in the issue).

## Scenes

The old builder replaced Genesis 8 in the user's private Calib-Eye with a
store figure (Julius 9): neither can go in git. Rebuilt from
Calib-L-Distant-45 with DAZ's free Genesis 9:

- `daz-build-eye-scene.dsa` (headless): the base scene without its cubes
  and sphere, Genesis 9 loaded (eyes, tear, eyelashes, mouth, card brows).
- `make-eye-scenes.py`: camera on the left eye (straight on, 100 mm, 9 cm
  off the cornea; at 15 cm both eyes filled a third of the frame), the
  distant light turned to −20° / 25° (at −45° the brow shadows the eye),
  SubD render level 1 (1.64 M faces at DAZ's 2, 0.54 M at 1), and eight
  variants that switch `pbr_skin.mdl`'s parts off one at a time: `-bare`
  (moisture and tear cut out), `-nogloss`, `-t0` (diffuse layer alone),
  `-t1` (transmission into the volume alone), `-t1-white` (no maps: the
  volume's own return), `-t08-white` (both layers, white maps),
  `-t1-white-nosss` (absorption only).

Octane, enhanced-final, 400 samples: ~190 s per scene, all rendered.
Regions for the compare (1920 × 1080): pupil centre ≈ (972, 607), the
iris a ring of r 70 – 115 px around it, the sclera left of the iris
(x 760 – 830, y 640 – 700).

## What DAZ hands Iray

Calib-G9Eye's `.mi` (local, `daz-export-mi.dsa`): `pbr_add_volume` with
`thin_walled` false, `transmitted_color` 0.949 / 0.201 / 0.101 at 0.1 cm,
`sub_surface_scattering_color` 0.949 / 0.448 / 0.548 at 0.015 cm,
direction −0.798 — the channels' sRGB colours (0.976 / 0.482 / 0.353 and
PBRSkin's default 0.976 / 0.694 / 0.761) decoded to linear. Absorption and
SSS enabled (defaults). Single-scatter albedo 0.87 / 0.77 / 0.64, mean free
path 2.5 / 0.15 / 0.16 mm.

## The MDL

`pbr_skin.mdl`'s `diffuse_translucency_bsdf` is a `weighted_layer` of the
diffuse (weight 1 − t, Translucency Weight) over
`diffuse_transmission_bsdf(tint: translucency_color)`: the bake's weights
are right. But what goes through is not a colour: it enters the volume,
scatters, and leaves through the same surface again — t × tint on the way
out, 1 − t reflected back in by the diffuse layer. On G9 base both maps
are the same layered image, so the bake gives the map itself.

## Prediction (no fit)

`scripts/dev/eye-volume-mc.py`: the ball (r 1.3 cm, Genesis 9's l_eye),
Lambertian entry, Henyey-Greenstein g −0.8, the interface as above, no
specular. Share of the entering light that comes out, per channel:

| | R | G | B |
|---|---|---|---|
| t 1 (= `-t1-white`'s albedo) | 0.61 | 0.43 | 0.32 |
| t 0.8 (escape) | 0.56 | 0.37 | 0.28 |
| t 0.8 albedo, 0.2 + 0.8 × escape (= `-t08-white`) | 0.65 | 0.50 | 0.42 |
| g 0 instead, t 1 | 0.70 | 0.32 | 0.22 |

The bake renders those white variants at albedo 1: expected Octane / Iray
≈ 1.6 / 2.3 / 3.1 (`-t1-white`) and 1.5 / 2.0 / 2.4 (`-t08-white`). The
first try used the channels' sRGB values (0.71 / 0.43 / 0.33 at t 1); the
`.mi` showed DAZ linearises them.

## Open

The Iray batch in #66 decides it. If the white variants match the
prediction (±15 %), the rule becomes albedo = (1 − t) · B + t · T · E(T, B)
from the volume's coefficients, computed at export (no constant); if not,
the variants say which part (diffuse layer, volume, light through the
ball) is off.
