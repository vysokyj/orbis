# Iray in DAZ Studio — materials, lights, environment, tone mapping

Reference for the DAZ ecosystem: what the Iray settings stored in a `.duf`
mean, as far as reproducing a DAZ render in another renderer needs it. Where
the channels sit in the file: [files.md](files.md).

## Iray Uber material

Channels are keyed by label in `studio_material_channels`; a channel has a
value and optionally an `image` (`image_file`, or `#id` into
`image_library`). Definition and scene material merge: the scene overlays the
material definition.

- **Base colour**: Iray multiplies `Diffuse Color` and the diffuse map (a grey
  tint darkens a map; a coloured one tints it). Weighted by
  (1 − `Refraction Weight`).
- **Base Mixing** — 0 metallicity/roughness (PBR), 1 specular/glossiness
  (ignores `Metallic Weight`), 2 **Weighted** (diffuse, translucency and
  glossy weights blend; e.g. diffuse 1, translucency 0.5, `Glossy Weight` 1).
- **Gloss**: a dielectric gloss comes only from the glossy layer
  (`Glossy Layered Weight`, older `Glossy Weight`) or skin's dual-lobe
  specular (`Dual Lobe Specular Weight`, `Specular Lobe 1 Roughness`…); at
  weight 0 a surface is matte. `Glossy Color` (and its map) tints the gloss.
  Roughness depends on the shading mode (roughness vs glossiness).
- **Cutout opacity**: greyscale map × value. Hair, lashes and brows are
  single-sided cut-out cards.
- **Thin Walled**: the surface is a sheet (fabrics), not a volume.
- **Refraction**: `Refraction Weight`, `Refraction Index`.
- **UV tiling**: `Horizontal/Vertical Tiles` + offsets.
- **Normal / bump maps**: data maps, linear (gamma 1); colour maps are sRGB
  (gamma 2.2). DAZ normal maps are often TIFF.
- **UDIM tiles**: figures spread UVs over tiles (torso U 1–2, arms 2–3, legs
  3–4, genital grafts 8–9…), one texture per tile.

### Translucency and subsurface (skin)

`Translucency Weight` > 0 with `Translucency Color` (map often pale/pinkish),
plus the volume: `Transmitted Color`, `Transmitted Measurement Distance`,
`Scattering Measurement Distance`, `SSS Direction` (anisotropy g). Physical
coefficients (DAZ cm):

- σa = −ln(`Transmitted Color`) / `Transmitted Measurement Distance`
- σs′ = (1 − g) / `Scattering Measurement Distance`
- diffusion length per channel = 1 / √(3 · σa · σs′)

Measured: Zelie ~1.73 cm, Noemi ~1.3 mm (max channel).

### dForce hair (Blended Dual Lobe Hair)

Recognised by `Hair Root Color`. Root / tip colour (each × its map), base
roughness, `Glossy Layer Weight`, strand width `Line Start Width` →
`Line End Width`. Each lobe is tinted by the colour; no transmission through
the fibres.

### Emission

`Emission Color` (non-black), `Luminance` in `Luminance Units` (cd/m²,
kcd/m², cd/ft², cd/cm², lm, W × `Luminous Efficacy`), `Emission
Temperature` (0 = use the colour), `Two Sided Light`. Luminance units are per
surface area — total power needs the emitting faces' area.

### Layered images

DAZ's Layered Image Editor stores a stack in `image_library[].map[]`, bottom
first: image or colour layers, `transparency` = opacity, operation
(`alpha_blend` / `blend_source_over` = normal, multiply, add, subtract,
screen), offset / scale in pixels, mirror. Used for tattoos and make-up over
skin; the first layer alone drops them.

## Lights

- **Photometric** lights emit `Flux` (lm) only; `Intensity` /
  `Intensity Scale` apply to non-photometric lights.
- `Light Geometry`: 0 point, 1 rect, 2 disc, 3 sphere; `Width`, `Height` in
  cm; `Temperature`.
- Spots: `Spread Angle` = full cone. DAZ (and Blender) spot power is a point
  source's power that the cone cuts from.
- **Watts ↔ lumens**: converting at **15 000 lm/W** (point lights
  ×3), which reproduces Iray's EV 13 exposure in watt-based renderers
  (overall luminance within ~5 % of Iray in Blender). Nits (L = Φ/(π·A)) or
  100 lm/W over-light by orders of magnitude.

## Environment ("Environment Options" node)

- `Environment Map` (HDRI image), `Environment Intensity` × the map value,
  `Dome Rotation`, `Dome Orientation X/Y/Z`, `Environment Mode` (3 = scene
  only — no dome light).
- `/resources/…` paths are DAZ Studio built-ins (`Contents/shaders/iray`).
- Iray lights from a downsampled HDRI (`Environment Lighting Resolution`,
  512 px) — shadows are softer than the HDRI's sun.

### Ground

`Draw Ground` (any mode but Scene Only, and only with an HDRI — otherwise
there is no backdrop for its shadow) catches shadows on a ground plane that
is **part of the environment**: geometry at its height (a room's floor)
hides it. Height: `Ground Origin Y` when `Ground Position Mode` is Manual,
else (Auto, default) the scene's lowest point. `Ground Shadow Intensity`
scales the shadow; Iray keeps the sky's fill in a full shadow (an umbra at
~11 % of the lit ground in one HDRI scene).

## Tone mapping ("Tonemapper Options" node)

`Exposure Value` (EV, default 13), `cm2 Factor`, `White Point` ×
`White Point Scale` (the colour that maps to white), `Saturation`,
`Burn Highlights` (1 = no burn), `Crush Blacks`. Switched off = DAZ defaults.
