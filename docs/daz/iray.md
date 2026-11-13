# Iray in DAZ Studio — materials, lights, environment, tone mapping

Reference for the DAZ ecosystem: what the Iray settings stored in a `.duf`
mean, as far as reproducing a DAZ render in another renderer needs it. Where
the channels sit in the file: [files.md](files.md).
How the Uber shader layers them, from DAZ's MDL source:
[iray-uber-mdl.md](iray-uber-mdl.md). Iray's renderer side (environment
dome, tone mapping, firefly filter) is in NVIDIA's Iray manual
(`docs/nvidia/`, `scripts/dev/fetch-nvidia-docs.py`).

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
`Line End Width`. In MDL (`dual_lobe_hair.mdl`): a highlight glossy (Highlight
Weight) over a hair-coloured glossy (Glossy Layer Weight) over a
`diffuse_transmission_bsdf` in the Transmission Color, by plain weights. On
untessellated fibres (Iray `hair`) the transmission is lost (black, also
Uber's translucency); on tessellated strands (ribbons, prisms) it passes
(Calib-Strand-UberT / -T3, #79). The glossies (Anisotropy 1 by default)
are sharp across the strand and flat along it, on fibres and prisms
alike; a prism's flat faces return 0 - 0.24 of what a fibre does to a
front camera under a 45° sun, 0.63 - 1.09 under a dome (DAZ's prisms all
face one way; Calib-Strand-UberG / -T3 and their -Dome / -Side / -Back:
Uber's weighted gloss at Glossy Anisotropy 1, Rotations 0.25 is the same
lobe). DAZ sometimes fails to re-import the
module for a later scene of a session: the material renders black (#94).

### Strand geometry as Iray gets it

A polyline mesh (strand hair, dForce guides and their render hairs) reaches
Iray in one of three shapes, chosen by the figure's **Line Tessellation**
modifier (`studio/modifier/line_tessellation`; it runs on figures only).
Seen in DAZ's `.mi` export, which reads the *Viewport* sides (a render reads
the *Render* sides; product hair sets both alike):

| Line Tessellation | Iray object | cross-section |
|---|---|---|
| none, or sides 0 / 1 | `hair`: cubic B-spline fibres, per-vertex radius = Line Width / 20 cm (the width is a diameter in mm), the ends extended by one control point | Iray's own round fibre |
| sides 2 | `trilist`, 2 vertices per strand point (plus an unused one) | a **flat ribbon** one Line Width wide, one normal across it; UV u across the ribbon (± a few thousandths), v along |
| sides 3 | `trilist`, flat-shaded faces | a triangular **prism**, circumradius = width / 2; each face its own normal |

On a plain polyline (no generator) the 2-sided ribbon's normal is the
object's +z made perpendicular to the strand (a strand along z: −y). The
render hairs of a dForce generator carry their own frames: Morgan Hair's
ribbons (Calib-Hair-Teens, sides 2) follow no fixed axis. The free dForce
Mohawk at sides 2 (`.mi` of a scene written from its preset, #79) shows the
rule: each render hair's ribbon faces the scalp at its root (normal against
the root's direction from the head's centre, made perpendicular to the
strand: median dot 0.94) and keeps that frame along the strand without twist
(transported: median 0.00°, 95 % within 0.3° per segment) — the hairs lie
flat over the head. UV u = the hair's index + 0.25 / 0.75 across, v its
index + the position along. The `.mi` export writes fewer render hairs than
the scene's dump holds (1504 vs 29803 strands: the viewport's preview
density, as it reads the viewport sides). Morgan Hair
(sides 2) is ribbons, dForce Corporate Hair (Calib-Hair, sides 3) prisms,
the free dForce Mohawk ships sides 1 (fibres). Scenes:
`scripts/dev/make-strand-scenes.py` (#79).

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
- Spots: `Spread Angle` = full cone. Iray's beam is
  cos(θ · 90° / (Spread / 2))^`Beam Exponent` — zero at the cone's edge;
  `Beam Distribution` (0–2) doesn't change it. The `Flux` is the beam's: the
  same total whatever the spread and exponent (measured, #7).
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
else (Auto, default) the scene's lowest point: the world bounding box's
minimum y over the visible geometry (a hidden node doesn't count; strand
hair counts by its vertices), and the .mi's `environment_dome_ground_position`
is 0.001 cm below it (-0.001 over a floor at 0), but never above 0: with
the floor hidden and everything floating, the ground stays at 0, Draw Ground
on or off (Calib-Env-Mi-GroundAuto-High). `Ground Shadow Intensity`
scales the shadow; Iray keeps the sky's fill in a full shadow (an umbra at
~11 % of the lit ground in one HDRI scene).

## Tone mapping ("Tonemapper Options" node)

`Exposure Value` (EV, default 13), `cm2 Factor`, `White Point` ×
`White Point Scale` (the colour that maps to white; DAZ hands Iray
`White Point`^2.2 × scale as `mip_whitepoint`, and Iray divides each
channel by it, keeping the mean), `Saturation`,
`Burn Highlights` (1 = no burn), `Crush Blacks`. Switched off = DAZ defaults.
