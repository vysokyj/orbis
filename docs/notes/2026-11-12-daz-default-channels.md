# 2026-11-12 DAZ Default channels (#41)

Goal: the DAZ Default (3Delight "standard material") channels the first
.mi sweep (Calib-Mi-DzDefault2) left open - maps, bump / normal /
displacement, tiling, the lighting models, the Subsurface group - read
from DAZ's own `.mi` export, not from renders. No Iray or Octane render:
the conversion lands on Iray Uber channels whose Octane side is already
calibrated.

## Scenes

All from Calib-Mi-DzDefault (DAZ primitives, DS Defaults presets), by
`scripts/dev/make-mi-shader-scenes.py`, map = `calib-tiles.png`:

| Scene | What it sweeps |
|-------|----------------|
| Calib-Mi-DzDefault4 | one map per sphere: diffuse (+ u/v scale and offset), diffuse_strength, specular, specular_strength, glossiness, transparency, bump, normal |
| Calib-Mi-DzDefault5 | displacement, refraction_strength / refraction maps, ambient, reflection maps, bump / displacement without a map, several maps with tiling, normal + bump + transparency |
| Calib-Mi-DzDefault6 (-Shown) | Lighting Model Metallic / Skin / Glossy (Plastic) / Matte / Glossy (Metallic), the hidden Subsurface group under Skin and Plastic; -Shown the same with every `visible` key dropped |
| Calib-Mi-DzDefault7 | specular map at strength 0.5, refraction white without a map / coloured with a map, Matte with values and with maps, glossiness map with a coloured specular / at strength 0, tiling on bump + normal |
| Calib-Mi-DzDefault8 | refraction white under a coloured specular, refraction colour = specular colour, refraction + glossiness map, Matte + refraction + transparency, Matte + transparency, refraction + transparency |

Each round's open question decided the next scene (4 -> 5/6 -> 7 -> 8);
each export took ~1 min headless.

## Findings

- Maps that pass: diffuse, diffuse_strength (attenuate_color), specular,
  specular_strength, glossiness, transparency, bump, normal, displacement.
  Dropped: refraction_strength, refraction, ambient, reflection(_strength).
- **Specular colour map**: glossy colour = map x the whole linear colour
  (not its hue), `glossy_color_transmit_enable` 1, weight = specular_strength
  alone (DzDefault7 S1: 0.5 with a colour peak 0.6, not 0.5 x 0.325).
- **specular_strength map**: weight = map x (peak x strength), as the value.
- **Glossiness map**: `glossy_roughness = square_float(gloss_to_rough(map x g))`
  and `glossy_reflectivity = 0.0399472` = (59/255)^2.2, i.e. 0.04 through an
  8-bit gamma-encoded DAZ colour: Uber's specular / glossiness mixing with
  Glossy Specular 0.04 (base_mixing stays `blm_layered`, as for mode 1).
  At strength 0 the reflectivity stays at the MDL default, as Uber mode 1.
- **Lighting Model Matte**: glossy weight 1 (the MDL default), white,
  roughness 1, whatever specular, its strength, glossiness or their maps;
  diffuse x strength unchanged. Metallic, Skin, Glossy (Plastic),
  Glossy (Metallic): nothing changes. Subsurface Scatter / Sheen / Thickness:
  nothing reaches Iray, shown or hidden, under Skin or Plastic.
- **Refraction sharing**: `iray_uber_unified` when the refraction colour
  equals the glossy colour (white under a grey specular - the hue is white
  -, [1, .6, .4] under the same specular), else `iray_uber` with its own
  colour (white under a coloured specular, DzDefault8 S1).
- **A refracting material passes no transparency** (DzDefault8 S4, S6: no
  `cutout_opacity`; S5, Matte without refraction: 0.6). The old conversion
  passed it; the export now doesn't.
- bump: factor = the bump value, bump_min / max not passed; normal:
  factor 1; bump + normal: normal on `base_bump`, bump on `base_bump2`, as
  Uber. Displacement: scale strength x (max - min), offset strength x min, as
  Uber's channels. u/v scale and offset are Uber's tiles and offsets.
- Map gamma: the generic `resolve_gammas` already gives what DAZ hands
  (auto alone, 1 when one image serves several channels of a material).

## Bug found on the way

The old conversion kept only the diffuse map: every other DAZ Default map
(transparency included - `opacity_channel` read the map-less Cutout
Opacity) was lost in the export.

## Where it stands

`iray::daz_default_as_uber` reproduces all six new fixtures
(`uber_matches_daz_mi_export`), plus a new test for what `material` reads
(Cutout Opacity, displacement). #41 closed.
