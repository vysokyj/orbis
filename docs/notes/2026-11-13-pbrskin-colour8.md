# 2026-11-13 PBRSkin's 8-bit colour channels (#152)

Goal: does DAZ round PBRSkin's other `color`-typed channels to 8 bits the
way #142 found for Makeup Base Color, and does Iray Uber do it too?
`.mi` only: no Iray, no Octane render.

## Types
`PBRSkin.dsf` types its colours:

| `color` | `float_color` |
|---------|---------------|
| Makeup Base Color, Top Coat Color, Metallic Flakes Color, SSS Color (+ Line Preview Color, Material ID: not in the `.mi`) | Base Color (`diffuse`), Translucency Color, Transmitted Color |

Iray Uber (Calib-Glossy's scene channels): every material colour is
`float_color`, Top Coat, SSS and Metallic Flakes Color included - its
`color` ones are UI only (Line Preview, Node Color, Material ID, DOF / FOV
overlays). The Uber fixtures already prove floats there: Calib-Glossy's
red 0.45 / 0.05 / 0.04 reaches Iray as 0.45^2.2 = 0.172611, not
(115/255)^2.2 = 0.17344. #41's "Glossy Specular 0.04 as an 8-bit colour"
is DAZ Default's conversion (`daz_default_as_uber`), not Uber's channel.

## Scene
Calib-PBRSkin-Colour8 (`make-calib-scenes.py`): eight PBRSkin spheres, two
off-grid triples - LO 0.123 / 0.456 / 0.789 (31.4, 116.3, 201.2 of 255,
all fractions < 0.5) and HI 0.199 / 0.402 / 0.703 (50.7, 102.5, 179.3) -
so the `.mi` tells rounding from truncation. C1/C2 Top Coat Color, C3/C4
Metallic Flakes Color, C5/C6 SSS Color with Translucency and Transmitted
Color on the other triple, C7/C8 Makeup Base Color; the base colour the
other triple each time. Exported alone in its DAZ session (#153); the
`.mi` holds `pbr_skin` (48 references).

## Result
| Channel | LO in the `.mi` | HI in the `.mi` | reads |
|---------|-----------------|-----------------|-------|
| Top Coat Color | 0.009696 0.176774 0.592438 | 0.028991 0.136099 0.459080 | 8 bits, rounded |
| Metallic Flakes Color | same | same | 8 bits, rounded |
| SSS Color | same | same | 8 bits, rounded |
| Makeup Base Color | same | same | 8 bits, rounded |
| Base Color | 0.009949 0.177714 0.593703 | 0.028673 0.134678 0.460576 | float |
| Translucency Color | float | float | float |
| Transmitted Color | float | float | float |

(31/255)^2.2 = 0.009696, (51/255)^2.2 = 0.028991: the nearest step (HI's
50.7 -> 51, truncation would give 50), then the gamma. The rule is the
definition's type, not the scene's: the generated scene saves every colour
as `float_color` and DAZ still rounds.

Before the fix the fixture test failed on exactly the six Top Coat /
Flakes / SSS arguments; Makeup, Base, Translucency and Transmitted passed.

## Fix
`Material::colour_is_8bit` (PBRSkin and one of `PBRSKIN_8BIT_COLOURS`):
`Material::color` (the Octane side) and `iray::Channels::color` round
those channels first, so the separate Makeup `color_8bit` path is gone.
How much it moves: at most 0.5/255 encoded; linear relative 2.2 x
0.5/255 / c = 0.43 % / c (4 % at 0.1, under 1 % above 0.45), invisible
except near black; no render needed.

## Lessons
- `mi-args.py --param` lists Uber columns only; for PBRSkin read the
  `mi-oracle.py` JSON (pass_color nodes, `sub_surface_scattering_color`
  and `transmitted_color` as plain arrays).
- #153 did not bite: one scene per DAZ run, then `grep -a pbr_skin` on the
  binary-looking `.mi` (plain `grep` says nothing on it).
