# 2026-11-05 — legacy 3Delight shaders as DAZ hands them to Iray (#64)

Goal: what DAZ Studio gives Iray for its old 3Delight shaders (UberSurface,
HumanSurface, AoA Subsurface, the DS Defaults shader-builder presets), read
from DAZ's `.mi` export as for DAZ Default (#41, #62), and the export doing
the same. No Iray render needed: the `.mi` decides.

## How often (My Library/Scenes, all `.duf`)

| shader | materials | scenes | in a calib scene? |
|---|---|---|---|
| Iray Uber | 28 105 | 465 | — |
| DAZ Default | 6 204 | 75 | Calib-Mi-DzDefault* (#41) |
| brick graphs (dForce, NVIDIA, OmniSurface, OOT, …) | 855 | 90 | #21 |
| omUberSurface (`support/DAZ/Uber/.../omUberSurfaceDef.dse`) | 53 | 15 | yes (S1 of Calib-Mi-Legacy) |
| omHumanSurface (`support/omnifreaker/...`) | 9 | 3 | no — third-party, not installed; same rule |
| dzPlastic | 1 | 1 | yes |
| AoA Subsurface (brick) | 2 | 1 | yes (Calib-Mi-AoA*) |

Of the 53 UberSurface materials 41 carry only Diffuse Color + Opacity
Strength (saved by DAZ Studio 6), 12 the whole UberSurface channel set
(saved by DAZ Studio 4); HumanSurface 4 / 5. UberSurface, the DS Defaults
presets and AoA Subsurface are all in "Default Resources for DAZ Studio"
(its DIM manifest), so they may go into calib scenes; HumanSurface is a
store product and isn't installed.

## Scenes

`scripts/dev/daz-build-shader-scene.dsa` applied the presets to
Calib-Mi-DzDefault's eight spheres (headless), `make-mi-shader-scenes.py`
varied the channels:

- Calib-Mi-Legacy-base / Calib-Mi-Legacy: S1–S7 UberSurface, dzGlossy,
  dzGlossyMetal, dzMatte, dzMetal, dzPlastic, dzSkin; S8 AoA Subsurface.
  Diffuse colours, opacity, a diffuse map (S3), an opacity map (S6).
- Calib-Mi-Legacy2: the whole DAZ Studio 4 UberSurface channel set (from
  its definition: groups, types, defaults) injected on S1–S7, varied —
  diffuse strength, both speculars, reflection + Fresnel, refraction,
  SSS + translucency, bump map + tiling, velvet + ambient.
- Calib-Mi-AoA-base / -AoA, -AoA2, -AoA3, -AoA4: AoA Subsurface on all
  eight spheres; glossiness sweep, Specular 2, reflection, SSS, velvet,
  bump / normal / displacement / tiling, maps on every channel the
  conversion reads, Specular 2 colours below 1.

## Findings

### DAZ Studio 6 has no 3Delight

The DS6 bundle has no 3Delight renderer and no `shaderDefinitions`; its log
says `Invalid render time file name passed to
DzShaderMaterial::setDefinitionFile()` for every preset. The material is a
bare DzShaderMaterial: Diffuse Color, Opacity Strength and the generic
properties (smoothing, lines, priority). Applied to a sphere that had DAZ
Default, it keeps the old diffuse colour.

What Iray gets, every scripted shader alike (7 definitions tried):

| arg | value |
|---|---|
| diffuse_color | Diffuse Color, linear (2.2) |
| cutout_opacity | Opacity Strength |
| glossy_layered_weight | not written → MDL default **1** |
| glossy_color | 1 1 1 |
| glossy_roughness | 1 |
| thin_walled | true |

Maps: a diffuse map (S3) and an opacity map (S6) **don't reach Iray** (the
.mi names no texture). Control: the same maps on DAZ Default spheres
(scratch copy of Calib-Mi-DzDefault2) did reach it. Legacy2: the DS4
channel set changes nothing — Diffuse Strength 0.6, speculars,
reflection, refraction, SSS, bump: all absent from the .mi; only the
diffuse colour and opacity (S4 0.3) pass.

The export (`legacy_shader_as_uber(mat, true)`) takes this but keeps the
diffuse and opacity maps: the user's 41 DS6-saved UberSurface materials
are mostly hair / beard cards with an alpha map, which Iray renders as
opaque, flat cards. Decision asked on #64.

### AoA Subsurface: a real conversion

A 3Delight brick graph (DzSpecularBrick, DzSubSurfaceBrick, … no MDL
bricks), yet DAZ hands Iray a full Uber. Its channel ids are DAZ Default's
labels and Uber's ids (`diffuse`, `Diffuse Strength`, `Specular Color`,
`Specular Strength`, `Glossiness`, `Bump Strength`, `Normal Map`,
`Displacement Strength`, `Minimum / Maximum Displacement`, `Horizontal /
Vertical Tiles`), plus `color2` / `value222` / `strength22` (Specular 2),
`value23` (SSS on), `sssmaterial`, `Ambient Color / Strength` (SSS colour /
strength), `color4` / `Ambient Strength2` (ambient), `color3` /
`strength3` (velvet).

Glossy roughness = 1 − Glossiness, linear (DAZ Default: (1 − g)^1.7):

| Glossiness | 0 | 0.3 | 0.5 | 0.6 | 0.75 | 0.9 | 0.95 | 1 |
|---|---|---|---|---|---|---|---|---|
| glossy_roughness | 1 | 0.7 | 0.5 | 0.4 | 0.25 | 0.1 | 0.05 | 0 |

Weight / colour: weight = max(Specular Color, linear) × Specular Strength,
colour = the hue (S3: [0.8, 0.6, 0.4] × 0.8 → 0.4897, (1, 0.531, 0.218)) —
DAZ Default's rule.

Specular 2: roughness min(1.5 (1 − strength22), 1):

| strength22 | 0.2 | 0.5 | 0.8 | 0.85 | 0.9 | 0.95 |
|---|---|---|---|---|---|---|
| roughness | 1 | 0.75 | 0.3 | 0.225 | 0.15 | 0.075 |

With Specular Strength > 0 it is the top coat (weight max(color2) ×
value222, hue colour; AoA4 S1 / S2 [0.6, 0.4, 0.2] × 0.5 → 0.1625,
(1, 0.410, 0.089)); with Specular Strength 0 it **is** the glossy layer —
the first puzzle: Specular Strength 0 gave roughness 0.225 at Glossiness
0.2, 0.5, 0.75 and 1 alike (the default strength22 0.85), weight 0, colour
0 0 0 (the scene's color2). Weight 0 leaves the colour unnormalised
(AoA4 S3: color2 as is).

Maps (AoA3): Specular Strength map → glossy_layered_weight map × (peak ×
strength); Specular Color map → glossy colour map × hue and
glossy_color_transmit_enable 1; color2 map → top coat colour map, its
transmit 1; Diffuse Strength map → `attenuate_color(map × value, colour)`;
Glossiness, value222, strength22 maps dropped (plain values). Bump + normal
map together: `base_bump` = the normal map, `base_bump2` = the bump. Bump
factor = Bump Strength (Positive / Negative Bump ignored); normal factor 1
whatever Normal Map Strength (`alpha`); displacement as Uber's (offset
min × strength, scale (max − min) × strength).

Reflection (S4), velvet (S7), ambient (`color4`, S6) reach nothing.

Subsurface: only SSS Material Type Custom gives Iray anything (Skin A, the
preset default, gives nothing): a closed volume, absorption σa = 100 ×
absorb / Shading Scale per channel written as transmitted distance 1 / max
σa and colour exp(−σa d) (AoA S6, AoA2 S5 fit exactly), scattering at
0.001 with an amount (1 1 1, then 0.4468 grey with scatter (0.5, 1, 1),
Scale 0.2, IOR 1.5) not worked out. Not reproduced: the fixture test skips
those spheres' SSS arguments; no user scene has it on (the one AoA material,
fingernails in Cows.duf, has SSS off).

### 8-bit colours

The first Legacy run's dzSkin [0.7, 0.5, 0.4] came back as 0.45908 =
(179/255)^2.2, not 0.7^2.2: DAZ keeps `color`-typed channels (DAZ Default's
`diffuse`, the scripted shaders') in 8 bits. Saved scenes are already
quantised, so the conversion doesn't round; the generator now uses
multiples of 0.2 for those channels. (`float_color` channels — Uber, hair —
keep their floats.)

## Code

- `Material.shader` (the `daz_shader` definition), `Material.url`.
- `iray::as_uber` → `daz_default_as_uber`, `legacy_shader_as_uber`,
  `aoa_subsurface_as_uber`; the export calls the three in
  `usd::mesh::scene_materials` (scripted shaders with their maps).
- `uber()`: Glossy Layered Weight map, Diffuse Strength map
  (`attenuate_color`).
- Fixtures: `calib/references/mi/Calib-Mi-{Legacy,Legacy2,AoA,AoA2,AoA3,AoA4}.json`.

## Octane sanity check

Calib-Mi-Legacy and Calib-Mi-AoA, enhanced-final, 256 samples, ~27 s each:
legacy spheres matte with a faint sheen (roughness-1 glossy), S3 textured,
S6 cut out; AoA spheres from matte to mirror-sharp by Glossiness, S3 at 0.6
opacity, S7 tiled diffuse + bump, S8 normal map + displacement. No errors.

## Process lessons

- Check the DS6 log first: the "Invalid render time file name" line told
  the whole story of the scripted shaders before any .mi.
- A control (the same map on a known shader) settles "dropped" vs "failed
  to load".
- Channel values carry over by name when a shader preset replaces another
  (AoA's Glossiness 1, Specular Color 0.6 came from DAZ Default): read the
  generated scene, not the preset's defaults.
- Pick 8-bit-exact colours for `color` channels in generated scenes.

## Open

On #64: whether the export should drop the scripted shaders' maps as Iray
does; AoA's subsurface; bump + normal map together on Uber (`base_bump2`,
seen here first) isn't in the export yet.
