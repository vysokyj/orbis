# 2026-11-13 UV-covered map means (#82)

Goal: a diffuse map's mean colour (`Textures::mean_covered_colour`) feeds
Base Color Effect's shares (`base_color_effect_shares`) and, on skin,
`skin_albedo`, `skin_crossing` and `skin_walk_weight` (the Translucency
Color map's mean). It left out near-black texels only; Monique 8's maps
fill the space between their UV islands with white. No renders: exports
from cached dumps with a temporary log of the old and the new mean.

## Attempts

1. **Near-white texels out when the image's border ring is mostly white**
   (64 px thumbnail, a Python survey of the maps). Monique's torso
   0.53 / 0.50 / 0.49 → 0.17 / 0.12 / 0.10, but her mouth map loses its
   white teeth too. Dead end.
2. **Flood fill from the border over near-white texels.** Keeps interior
   whites, but Genesis 9's iris map (`Genesis9_Eyes_Iris_01`, the sclera
   reaches the border: 56 % "fill") and two synthetic calibration maps
   (`calib-bumpgamma-16`, `calib-overlay-weight`) would lose painted
   texels. Dead end: no image rule tells fill from paint.
3. **The faces' own UV coverage** (kept): `usd::uv_coverages` rasterises
   each material's UV triangles into a 128² grid (cell centre in the
   triangle, a sub-cell triangle marks its centroid's cell; each face in
   the UDIM tile of its UV centroid; v up = image row 0 at v 1); the mean
   is taken over the covered cells of the map resized to 128². Monique's
   torso 0.121 / 0.072 / 0.053 (the median of the whole map 0.16 / 0.10 /
   0.07). Tiled or offset maps, and meshes without UVs, keep the old rule.

## What moves

Every `calib/scenes` scene with a dump (280 exported): the non-skin path
(translucent Uber surfaces, `diffuse_share` / `transmit_share`) changes by
≤ 2e-5 (resampling) on Calib-WeightedTL*, nothing else reads it. No
recorded number moves.

The Calib-Skin fronts (skin path), old → covered:

| Skin | Base Color Effect | diffuse share (effect 1/2) | Translucency Color mean (torso, R) |
|---|---|---|---|
| G8F base | 1 | 0.61 – 0.64 → 0.60 – 0.63 (fingernails 0.78 → 0.84) | 0.51 → 0.56 |
| Angel | 1 | 0.81 – 0.88 → 0.72 – 0.74 | (none) |
| Nishi | 1 | 0.63 – 0.65 → 0.62 – 0.65 | (none) |
| Monique | 0 | - | 0.62 → 0.28 |
| Gabriela | 0 | - | 0.78 → 0.58 |
| Twosret | 0 | - | 0.61 → 0.35 |
| Victoria | 0 | - | 0.93 → 0.87 |

The torso `-patch` diffuse maps (fully painted) move < 0.3 %.

The Translucency Color mean sets the walk weight S on every skin. #86's
step 8 (docs/notes/2026-11-08-skin-walk-weight.md) tried exactly this
and left it out: a lower S took Monique's front to 0.89 / 0.83 / 0.94 ×
Iray against 1.00 / 0.96 / 1.07. So the skin path keeps the near-black
rule; the covered mean goes to the non-skin path only. Whether the skin
rules (held, #19 / #57) should read the covered mean is the question
left in #82: the old mean's white fill may stand in for a component the
walk-weight rule is missing (decompose before refitting).

## Process

- A dump is tied to the scene's path: the `calib/scenes` copies have no
  dumps of their own, export the "My Library/Scenes" copy.
- No `timeout` on macOS: a loop that wraps exports in it exports nothing.
