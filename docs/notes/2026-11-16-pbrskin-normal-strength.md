# 2026-11-16 PBRSkin normal map strength (#177)

Goal: #176 proved that DAZ hands Iray the Uber's Normal Map value as
`tangent_space_normal_texture`'s factor; PBRSkin's Normal Map stayed at 1
in the export because no PBRSkin `.mi` had a Normal Map other than 1.
Check it with a `.mi`, no Iray render.

## Scenes (`make-calib-scenes.py`, `pbr_spheres`, .mi only)

- **Calib-Mi-PBRSkin-NormalStrength**: a red dual-lobe base (reflectivity
  0.5, roughness 0.4), K1 - K4 the Normal Map (ridges) at 1 / 0.5 / 0 / 2,
  K5 0.5 under a height map (Bump Enable on), K6 0.5 under a top coat, K7
  the Detail Normal Map (dimples, 1 x 1) at 0.5, K8 normal 0.5 + detail 2.
- **Calib-Mi-PBRSkin-NormalStrength2**: the first `.mi` showed the Normal
  Map gated by Bump Enable (below), so J1 / J2 the Normal Map 1 / 0.5 with
  Bump Enable on, J3 a map at 0 beside a height map, J4 a Detail Normal Map
  at 0, J5 under translucency, J6 a height map at 0.5, J7 detail weight
  0.5, J8 normal + detail + top coat.

One headless DAZ `.mi` export each (`daz-export-mi.dsa`), no shader brick
lost (`mi-audit.py`: 286 checked, 0 disagree).

## What the .mi says (`pbr_detail_normal`'s arguments)

| Sphere | normal factor | bump factor | detail factor | normal_enable |
|--------|---------------|-------------|---------------|---------------|
| K1 normal 1 | 1 | - | - | false |
| K2 normal 0.5 | 0.5 | - | - | false |
| K3 normal 0 | (no map) | - | - | false |
| K4 normal 2 | 2 | - | - | false |
| K5 normal 0.5 + height | 0.5 | 1 | - | true |
| K7 detail 0.5 | - | - | 0.5 | false |
| K8 normal 0.5 + detail 2 | 0.5 | - | 2 | false |
| J1 normal 1, Bump Enable | 1 | - | - | true |
| J2 normal 0.5, Bump Enable | 0.5 | - | - | true |
| J3 normal 0 + height | (no map) | 1 | - | false |
| J4 normal 0.5 + detail 0 | 0.5 | - | (no map) | true |
| J6 normal 1 + height 0.5 | 1 | 0.5 | - | true |

- **The value is the factor**, as on the Uber: Normal Map 0.5 and 2 as
  saved (PBRSkin's range is 0 .. 2, so 2 is not clamped as the Uber's 1 is),
  Bump Strength 0.5, Detail Normal Map 0.5 / 2 (range 0 .. 50). A map at 0
  isn't passed (K3, J3, J4).
- **Bump Enable gates the Normal Map.** DAZ's `normal_enable` is
  `Bump Enable && (mapped && |value| > 1e-5)` - the same switch as
  `bump_enable`'s. K1 - K4 (no Bump Enable) pass the map but switch it off:
  `pbr_detail_normal` returns the plain normal. `gated_off` already had
  this pairing; the export's `normal_factor` returned 1 for every PBRSkin
  map and so ignored it.
- `detail_enable` is Detail Enable alone; a Detail Normal Map at 0 gives
  `blend_normals(base, 1, state::normal(), w)`, which is the base (RNM
  with a flat detail returns the base).
- The inverted normal (translucency's back side) takes the same maps with
  the factor negated (-0.5 for 0.5, J5): what `tangent_space_normal_texture`
  does with a negative factor is the transmitted side's business; not
  exported separately.

## Changes

- `iray::pbrskin` gives `pbr_detail_normal.normal` / `.bump` (map at its
  value) and `normal_enable` / `bump_enable`; the fixture test evaluates
  DAZ's constant switch expressions (`const_bool`) and reads height maps
  as well as normal maps. Mutation check: dropping the Bump Enable term
  fails K1 / K2 / K4.
- Export: `normal_factor` no longer special-cases PBRSkin (`Material::float`
  gates it by Bump Enable, its range clamps at 2); the Detail Normal Map is
  baked at its value (`pbr_detail`), and left out at 0
  (`Material::pbr_detail_normal`).
- Test `pbrskin_normal_map_strength_is_the_factor`.

No constant, no render: a direct conversion check (the #57 rule holds).

## Effect on scenes

Demo-Gladiator: 44 normal-mapped PBRSkin surfaces, all with Bump Enable
on; 31 at Normal Map 0.25, 4 at 0.5, 9 at 1 - those 35 went in fully
bumped and now go in at their strength. Not rendered here; the next
Gladiator compare will see it.

## Process

The first scene assumed Bump Enable would only gate the height map
(PBRSkin has no "Normal Enable"); the `.mi`'s switch expressions said
otherwise, and a second scene with the gate on cost one more headless
run. Evaluating the switch expressions in the fixture test is what makes
the gate checked rather than assumed.
