# 2026-11-05 Per-map image modifications (#77)

Goal: DAZ's per-map texture-instance settings on a channel
(`image_modification`: tiles, tiling offsets, scale, offset, invert,
grayscale mode) were ignored; only the material's Horizontal / Vertical
Tiles and Offsets were read. Seen on OOT's Rose hair (Calib-OOT, #21):
the diffuse map at 0.6 tiles on u, the glossy weight map at 1.2.

## How common

All 476 scenes in My Library/Scenes (a throwaway scan in the style of
`scripts/dev/channel-audit.py`): 110 carry `image_modification` on some
channel (2893 channels); a non-default field in:

| Field | Scenes | Typical |
|---|---|---|
| invert | 80 | Glossy Roughness 58 scenes, Top Coat Roughness 42 (a glossiness map on a roughness channel), PreRender Reduce Length 8, Refraction Weight 4 |
| horizontal / vertical tiles | 38 / 15 | diffuse 5, 0.6, 1.5; Top Coat Bump 0.5, 1.75; Glossy Weight 1.2; Bump 20 × 20; Normal Map 3 × 3 |
| scale | 29 | Top Coat Weight 3, 1.35; Glossy Color 2; Cutout Opacity 0.7 – 1.25 |
| grayscale mode | 18 | alpha on Diffuse Overlay Weight (13) and Makeup Weight (5); red / green / blue on a packed map (Biker) |
| tiling offsets | 13 | 1 / 1 on the Calib-Skin figures' normal maps (a whole period: no effect) |
| offset | 2 | 0.5 on Translucency Weight |

The same image often sits on two channels of one material with different
settings (86 scenes: a spec map as Glossy Reflectivity plain and as Glossy
Roughness inverted), so the settings have to travel with the channel, not
the image.

## Scenes and the .mi

Two generated .mi sweeps (`make-calib-scenes.py`, DAZ built-in spheres,
`calib-tiles.png` and a new generated ramp `calib-imagemod-ramp.png`: red
along U, green along V, a blue checker):

- Calib-Mi-ImageMod: I1 map tiles 3 × 2; I2 map tiles 2 × 3 with tiling
  offsets 0.125 / 0.25 over material tiles 1.5 × 2 and offsets 0.1 / 0.2;
  I3 diffuse scale 0.5 offset 0.2; I4 diffuse invert; I5 Glossy Roughness
  invert, scale 1.5, offset −0.1, tiles 2; I6 Glossy Layered Weight red
  channel, scale 0.6, offset 0.3; I7 bump tiles 4 × 4, invert, scale 2;
  I8 diffuse tiles 0.5 + vertical tiling offset 0.375 and a roughness map
  at tiles 3 with grey mode alpha, material V tiles 2.
- Calib-Mi-ImageMod2: J1 – J3 roughness green / blue / maximum; J4 Top
  Coat Weight invert offset 0.25; J5 Cutout Opacity scale 0.8; J6 Glossy
  Color scale 2; J7 Translucency Color scale 1.5 offset 0.1; J8 Normal Map
  tiles 3 × 3, tiling offsets 1.

DAZ's export (headless, ~1 min each) gave, per `file_texture`:

| Sphere | uvw scale, translation | color_scale | color_offset | mono_source |
|---|---|---|---|---|
| I1 | 3, 2; 0, 0 | colour | – | average |
| I2 | 3, 6; 0.325, −0.85 | – | – | average |
| I3 | 1, 1 | 0.5 · colour | 0.2 · colour | average |
| I4 | 1, 1 | −colour | colour | average |
| I5 | 2, 1 | −1.05 | 0.98 | average |
| I6 | 1, 1 | 0.54, 0, 0 | 0.27, 0, 0 | maximum |
| I7 (bump) | 4, 4 | factor 0.6 only | – | – |
| I8 diffuse / rough | 0.5, 2; 0, −0.375 / 3, 2 | colour / 0.8 | – | average / (none: alpha) |
| J1 / J2 / J3 | 1, 1 | 0, 0.8, 0 / 0, 0, 0.8 / 0.8 | – | maximum |
| J4 | 1, 1 | −0.8 | 1 | average |
| J6 | 1, 1 | 2 · colour (1.59, no clamp) | – | average |
| J7 | 1, 1 | 1.5 · colour | 0.1 · colour | average |
| J8 (normal) | 3, 3; 1, −1 | – | – | – |

So: uvw = (uv · mt + (mo_u, −mo_v)) · pt + (po_u, −po_v) (I2: 0.1 · 2 +
0.125 = 0.325, −(0.2 · 3 + 0.25) = −0.85); value = V · (s · t' + o) with
t' = 1 − t when inverted (I5: 0.7 · (1.5 (1 − t) − 0.1) = 0.98 − 1.05 t;
inverting after the scale would give 0.77 − 1.05 t); a one-channel mode
masks the other channels and reads the maximum; bump and normal maps take
the placement only.

All of it passed the fixture test on the first run of the new `iray.rs`
code (Map now carries `offset` and `mono`; the test also compares them).
A deliberately broken variant (no invert, no grey mode) failed 8 of the
new arguments, so the fixtures do check it.

## Octane side

- Placement: `uv_transform` — Iray's S, T → Octane scale 1 / S,
  translation (−T_u / S_u, −T_v / S_v), which is #65's rule
  (`uv_translation`) with the map's offset counted in its tiles.
- Scale / offset / invert: a Mix texture (38) with the image as amount and
  value × lo / value × hi as `texture1` / `texture2` (amount 1 is
  `texture2`). Octane's own `invert` pin was not used: whether its `power`
  applies before or after the inversion isn't documented, and the Mix is
  exact for any scale and offset (also outside 0 – 1).
- Grey mode: a one-channel PNG copy of the image's bytes (decoded by the
  same gamma as the original); in a roughness bake the picked channel
  feeds the bake (alpha linear).
- The channel travels by `with_map` around each image write
  (`channel_map`), not by image: the same image on two channels keeps
  each channel's settings.

Octane render (256 samples, 22 s / 28 s), sanity only: I1 tiled 3 × 2, I2
dense, I3 / I4 the ramp dimmed and inverted (complementary hues), I8's
wide stripes, J8's normal pattern 3 × 3, J5 semi-transparent. The bump
sphere I7 is barely bumped: power 0.30 (1 / 4 of the untiled bump), the
grout lines thin at 4 × 4 — as `bump_power` intends. No Iray render was
needed: the .mi decides, and the Octane-side ops are linear.

## Open

- Squared maps (overlay weight, metallic) apply the scale / offset to t
  before the squaring (their gamma doubling), Iray to the value — not
  verified, rare.
- Brick shaders' (NVIDIA MDL) maps ignore `image_modification`.
