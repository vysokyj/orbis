# Iray Uber in MDL — how the shader layers its channels

What DAZ Studio's Iray Uber material is, read from its MDL source, which
ships with DAZ Studio:
`DAZStudio.app/Contents/shaders/iray/daz_3d/irayubermaterial.mdl` (DAZ's
copyrighted code, not in this repo — read it there). The `df::` and
`base::` building blocks are defined in NVIDIA's MDL spec and `::base`
reference (local mirror: `docs/nvidia/`, `scripts/dev/fetch-nvidia-docs.py`).
Use this before fitting a channel by calibration: the structure is exact,
only DAZ's channel → parameter scaling is not (see [the mapping layer](#the-mapping-layer)).

Function names below are the MDL file's; line numbers drift between DAZ
versions, search by name.

## The mapping layer

The MDL entry point (`iray_uber`, wrapped by `iray_uber_unified`) takes
parameters such as `glossy_reflectivity` or `top_coat_curve_strength`,
not DAZ channels. DAZ Studio fills them in compiled code
(`PlugIns/libdsp_dzirayrenderer.dylib`). Its result can be read without
touching that code: DAZ's **.mi exporter** (`DzMIExporter`, Iray's scene
format) writes every material as its MDL call with the arguments that
differ from the defaults. `scripts/dev/daz-export-mi.dsa` exports scenes
headless, `scripts/dev/mi-args.py` tabulates the arguments; the
`Calib-Map-*` scenes (`make-calib-scenes.py`) sweep the channels for it.
The .mi also carries Iray's render options (environment, tone mapping,
path lengths).

**The rules are code now:** `src/iray.rs` reproduces every argument of the
Iray Uber materials in `calib/references/mi/` (DAZ's export of all
`calib/scenes`, as JSON from `scripts/dev/mi-oracle.py`), and its test
checks each one. The table below summarises; the module is the reference.
Found beyond it:

- **Base Mixing 2 (Weighted)** takes its roughness from Glossy Roughness,
  like Base Mixing 0 — only Base Mixing 1 reads Glossiness. Without gloss
  (Glossy Weight 0) the whole gloss part (colour, backscattering) isn't
  passed.
- **Diffuse** is switched off (`diffuse_enable` false) when Metallic,
  Refraction or Backscattering Weight is 1.
- **Metal hue:** `metallic_directional_color_enable` only for a coloured
  base; a grey metal has no directional tint.
- **Dual Lobe Specular Weight** goes to both `dual_lobe_specular_weight`
  and `dual_lobe_uniform_specular_weight`.
- **Refraction** without gloss (Glossy Layered Weight 0) doesn't reflect
  (`refraction_reflection_enable` false); otherwise DAZ asks for the full
  Fresnel reflection, except with Share Glossy Inputs off, which switches to
  the `iray_uber` entry with the refraction's own colour and roughness.
- **Volume** (transmitted colour and distance, scattering distance,
  direction) is passed only inside a closed surface that lets light in
  (Refraction, Translucency or SSS Amount > 0, not Thin Walled);
  `absorption_enable` follows a non-black Transmitted Color, also on a
  thin wall.
- **SSS:** mono mode passes SSS Amount (clamped to 1) as grey; chromatic
  mode passes the linear SSS Color and `logarithmic_scatter_enable`.
  **SSS Reflectance Tint is not passed at all.**
- **Base Color Effect** only where translucency mixes with the diffuse.
- **Lights** (`iray::light`): `dzpm_diffuse_area_light`, spots
  `dzpm_spot_light`; Flux as is when photometric, else 1000 π × Intensity
  with temperature 0; the colour, unlike a material's, rounded to 8 bits
  and through the sRGB curve; Spread Angle (the full angle) in radians,
  Beam Exponent as `spot_exponent`; rect / disc / sphere sizes in cm by
  Light Geometry, a spot's too (disc and sphere: Height as the diameter);
  Two Sided on rect, disc and sphere; a point light with a Light Profile
  `dzpm_ies_light` (`profile`: the IES file; any shape), an Emission
  Profile `emission_profile_enable` and `emission_profile` (Calib-L-IES-*). The camera headlamp
  (`iray::headlamp`): a 7 × 3.5 cm rect, colour 0.8, 10 × Depth of Field²
  × Headlamp Intensity lm, at the camera moved by its offsets (cm, camera
  axes). The .mi's instance transforms are world → object.
- **Maps** (`iray::Map`): `base::file_texture` scaled by the channel's
  value (`color_scale`: linear for colours, as is for floats), read as a
  colour (`texture_return.tint`) or grey (`.mono`); roughness maps then
  `square_float`, glossiness maps `gloss_to_rough` first; bump
  `file_bump_texture` at Bump Strength (`base_bump`; a Normal Map
  `tangent_space_normal_texture` there instead), Top Coat Bump as
  `top_coat_bump` — Mode 0 a `file_bump_texture`, 1 a
  `tangent_space_normal_texture`, the value as the factor (Calib-CoatBump); UV transform Horizontal / Vertical
  Tiles and Offset, the vertical offset negated; gamma the image's own, else
  "auto" (Iray decides). A metal, refraction or backscattering map at
  weight 1 leaves the diffuse on.
- **Emission units:** cd/m² and kcd/m² (× 1000) per m², cd/ft² per foot
  (`emission_scene_unit_size` 0.3048), kcd/ft² as cd per cm² (0.01, no ×
  1000 — DAZ's own conversion), lm and W × efficacy as a power.
- **Base Mixing 1 / 2:** every lobe's roughness follows the glossy one's
  (1 − Glossiness in mode 1, the Roughness channel in 0 and 2, squared);
  Weighted has no metal (Calib-Mi-Mix1 / -Mix2).
- **Camera** (`iray::camera`): focal length and film width in cm, the
  width = Frame Width (fitted to the image's height) × aspect; lens radius
  focal (mm) / (2 · f-stop) when DOF is on, the focus distance as is.
- **Tone mapping** (`iray::tone`): Film ISO, Shutter Speed, Aperture,
  burn, crush, gamma, saturation, vignetting, cm² factor as they are
  (Exposure Value isn't passed); white point × its scale.
- **Environment** (`iray::environment`): an HDRI dome at Environment
  Intensity × Environment Map × 10 000; Scene Only none; sun-sky's
  multiplier, haze, disk intensity / scale, saturation, horizon as they
  are, ground colour linear. The dome (`iray::dome_setup`): Dome Rotation
  in turns about Y, orientation Rz · Rx · Ry of Dome Orientation Z / X /
  Y, Environment Tint linear, the ground as set. The sun's direction: a steering node's
  position as is, else the solar position — ours (`usd::sun_direction`)
  within 0.1° of DAZ's.
- **PBRSkin** (`iray::pbrskin`; its channel defaults from DAZ's
  PBRSkin.dsf in `calib/references/daz-defaults/PBRSkin.json`): values
  raw where Uber converts — roughness unsquared (pbr_skin.mdl squares it),
  distances in cm; colours linear; `specularlevel` and the coat's
  `top_coat_specular_level` 0.08 × their Reflectivity (Top Coat
  Reflectivity, not Uber's Reflectivity); `translucency_weight` = 1 −
  Translucency Weight (inverted in the MDL); lobe 2's roughness the Mult.
  The "… Enable" switches reach the MDL as expressions, not values.
- **DAZ Default shader** (`iray::daz_default_as_uber`; the export
  converts it on load): DAZ renders it as Iray Uber — diffuse ×
  diffuse_strength; glossy weight = the specular colour's strongest
  channel (linear) × specular_strength, its hue the glossy colour; roughness
  (1 − glossiness)^1.7; refraction at refraction_strength, own colour and
  IOR (`iray_uber`, closed); transparency as Cutout Opacity. Ambient,
  reflection and the lighting model don't reach Iray. The old 3Delight
  presets (dzGlossy, dzMetal, …) are other shaders, not covered.
- **Blended Dual Lobe Hair** (`iray::blended_hair`): the Root colours only
  (hair, transmission, highlight — the Tip ones never reach Iray),
  roughnesses raw, the rest as is.
- **OmniHair Root-Tip Blend** (`iray::omni_hair`): root and tip blended
  along the strand (`blend_colors`, gain 0.15); Diffuse Strength the base
  colour's weight, Highlight Weight the diffuse reflection's.
- **NVIDIA MDL Examples** (Calib-NVIDIA): the channels go straight to the
  example's MDL function — colours linear, numbers and textures as they
  are, the rest at the function's defaults; `material::mdl_metal` /
  `mdl_dielectric` read them so (test, where DAZ's definitions are
  installed).
- **Brick graphs** (Shader Mixer, even on Uber's bricks) aren't these
  conversions: `iray::is_uber` leaves them out (#21).
- **Emission:** `emission_flux` and mode by Luminance Units (above);
  the temperature once the colour is non-black.

Observed (DAZ Studio 6, 2026-10-31):

| channel(s) | MDL argument |
|---|---|
| any colour (Base, Glossy, Top Coat, Translucency, Transmitted, Overlay, Flakes) | the value to the power 2.2 (linear) |
| Diffuse Strength | multiplies the linear `diffuse_color` |
| Glossy / Top Coat / Dual Lobe Reflectivity r | `glossy_reflectivity` / `top_coat_curve_strength` / `dual_lobe_specular_reflectivity` = 0.08 r |
| every roughness (Glossy, Top Coat, Lobe 1 / 2, Backscattering, Refraction, Flakes) | squared (with Roughness Squared on, the default) |
| Metallic Weight w, base colour c | `metallic_weight` = w; `metallic_custom_curve_normal` = max(c); `metallic_directional_normal_color` = c / max(c) (enabled); `diffuse_enable` off at w = 1. Glossy colour and reflectivity don't reach the metal |
| Base Mixing 1, Glossy Specular s | `blm_layered` with `glossy_reflectivity` = max(s^2.2), its hue as `glossy_directional_normal_color`; Glossiness → roughness; Metallic Weight dropped |
| Base Mixing 2 | `blm_normalized`, `diffuse_weight` / `glossy_weight` as set |
| Top Coat Layering Mode 0 / 1 / 2 / 3 | `tcw_principled_specular` / `tcw_weighted` / `tcw_fresnel` / `tcw_custom_curve` |
| **Top Coat IOR, Top Coat Curve Normal / Grazing / Exponent, Top Coat Specular** | **not passed**: Fresnel mode always reflects at IOR 1.5, Custom Curve at the MDL defaults (head-on 0, grazing 1, exponent 5). Calib-TopCoat's Iray spheres at IOR 1.5 and 2 render alike |
| Color Effect (Base, Glossy, Top Coat, Overlay) | `*_color_transmit_enable` = the mode (0 / 1 / 2); the colour unchanged |
| Diffuse Overlay Weight Squared | `diffuse_overlay_weight` squared |
| Translucency on | `translucency_enable`, and `invert_transmission_normal` always on |
| Backscattering with Share Glossy Inputs on (default) | Backscattering Color not passed (the MDL takes Glossy Color) |
| Share Glossy Inputs off | `refraction_glossy_unified_enable` false, `backscattering_color` / `refraction_color` passed |
| Transmitted / Scattering Measurement Distance, Flakes Size (cm) | × 0.01 (metres) |
| SSS Amount | as is (not linearised) |
| Abbe a > 0 | `dispersion_enable`, `abbe_number` = a |

Still empirical: anything a channel feeds through a texture (maps pass as
texture calls, not values) and the `*_enable` flags beyond these.

## The stack, bottom to top

`iray_uber` builds one material in this order; each step wraps the one
before:

1. **Base** — `metallic_layered_base` (Base Mixing 0, and 1 after DAZ's
   conversion) or `normalized_base` (Weighted, 2). Below.
2. **Base Thin Film** — `add_thin_film`: multiplies the whole base by a
   directional tint (head-on → grazing, (1 − cos θ)^1.1), from NVIDIA's
   `legacy_thin_film::compute_refl`. Not a layer, no weight.
3. **Metallic Flakes** — `add_metal_flakes`: `weighted_layer` of a glossy
   BSDF (flake colour, flake roughness) over the base, weight = Flakes
   Weight × `base::flake_noise_texture(size, density)`, normal from
   `base::flake_noise_bump_texture(size, strength)` (mixed with the base
   bump when there is one).
4. **Metallic Flakes Thin Film** — the same tint over everything so far.
5. **Top Coat** — `add_top_coat` over everything so far. Below.
6. **Top Coat Thin Film** — the tint over everything, coat included.
7. Backface copy (two-sided emitters), geometry (`Cutout Opacity`,
   displacement, bump), **volume** (`add_uber_volume`), emission.

## Base, layered (`metallic_layered_base`)

Bottom to top:

- **Diffuse / translucency** (`diffuse_base`): without translucency a
  `diffuse_reflection_bsdf` (Oren-Nayar, `Diffuse Roughness`) in the
  base colour. With it, a `weighted_layer` of the diffuse reflection at
  1 − Translucency Weight over a `diffuse_transmission_bsdf` in the
  translucency colour (its normal optionally inverted). **Diffuse
  Overlay** is a further `weighted_layer` of a second diffuse reflection
  (overlay colour, overlay roughness) at the overlay weight on top: it
  replaces that share of diffuse and translucency.
- **Dual Lobe Specular** (`add_dual_lobe_specular`, when its uniform
  weight > 0): a `custom_curve_layer` with the *diffuse* as the layer and
  the specular as the base — the diffuse keeps 1 − Reflectivity × Weight
  head-on and 1 − Weight at grazing. The specular is a white
  `weighted_layer` of lobe 1 (ratio) over lobe 2 (1 − ratio), at their
  own roughnesses.
- **Glossy** (`add_top_coat` in custom-curve mode, reused): a
  `custom_curve_layer` of a glossy BSDF (Glossy Color, Glossy Roughness)
  over the diffuse, head-on = `glossy_reflectivity`, grazing 1, exponent
  5 (Schlick), weight = Glossy Layered Weight. Glossy Specular /
  directional colours: a `directional_factor` on the glossy BSDF.
  Without diffuse (`diffuse_enable` off) the glossy goes too.
- **Backscattering**: `weighted_layer` of
  `backscattering_glossy_reflection_bsdf` at Backscattering Weight over
  diffuse + glossy. Its tint is **Glossy Color while Refraction Glossy
  Unified is on** (the default); Backscattering Color only when it is
  off.
- **Metal**: `weighted_layer` at Metallic Weight over all that. The metal
  lobe is itself a custom-curve coat over **black** at weight Metallic
  Weight (head-on = the base colour's strongest channel, grazing 1,
  exponent 5, its hue as the directional colour, glossy roughness) — DAZ
  passes the same weight to both, so the reflection scales with
  **Metallic Weight²** (#59).
- **Refraction**: `weighted_layer` at Refraction Weight over everything:
  one glossy (or specular) BSDF in reflect + transmit mode at
  `glossy_ior` — its reflection is the BSDF's own Fresnel. It is split
  into a `fresnel_layer` (Glossy Layered Weight on the reflection) plus
  transmission only when refraction and glossy aren't unified, a
  dispersion, a separate refraction reflection or Ward. **Dispersion**
  makes the transmission perfectly smooth (roughness 0) at
  `base::abbe_number_ior(ior, Abbe)`.

## Base, weighted (`normalized_base`, Base Mixing 2)

A `normalized_mix` of two parts: the diffuse (+ dual lobe) at
Diffuse Weight and the "gloss" at Glossy Weight, where the gloss is a
plain glossy BSDF (no Fresnel, no curve), backscattering over it and
refraction over that. `normalized_mix` scales the weights down only when
they sum past 1; below 1 the rest is black.

## Top Coat (`add_top_coat`)

A glossy BSDF in Top Coat Color and Top Coat Roughness (anisotropy
through `base::anisotropy_conversion`), optionally with
`directional_factor` (Top Coat Specular head-on → grazing colour,
exponent 5 in mode 0, else the curve exponent). Layered over everything
below by **Top Coat Layering Mode**:

| mode | MDL | head-on / grazing |
|---|---|---|
| 0 Reflectivity ("principled specular") | `custom_curve_layer` | Top Coat Reflectivity (DAZ-scaled) / 1, exponent 5 |
| 1 Weighted | `weighted_layer` | weight alone, no Fresnel |
| 2 Fresnel | `fresnel_layer` | Top Coat IOR |
| 3 Custom Curve | `custom_curve_layer` | Curve Normal / Curve Grazing, Curve Exponent |

The weight is Top Coat Weight (or its Color Effect version, below).

## Colour Effect (`transmit_color_calc`)

Base / Glossy / Top Coat / Diffuse Overlay / Flakes Color Effect, when
not 0 ("Scatter Only"), turn a layer's colour into two parts: the BSDF
tint becomes the colour's hue and saturation at full intensity
(`color_layer_color` over white), and the layer's weight becomes weight ×
the colour (its mono, the average); in mode 2 (Scatter & Transmit
Intensity) the weight's colour is greyed first. So a dark colour thins
the layer instead of darkening its tint. On the diffuse it applies only
when translucency mixes with it.

## Volume (`add_uber_volume`)

- absorption σa = −ln(Transmitted Color) / Transmitted Measurement
  Distance;
- scattering σs = SSS Amount / Scattering Measurement Distance (or
  −ln(SSS Amount) / distance in the logarithmic mode);
- phase: `anisotropic_vdf` with SSS Direction as the bias.

## BSDF model

Every glossy lobe goes through `choose_bsdf`; the default mode is
`simple_glossy_bsdf`: a Phong distribution with exponent 2 / roughness²
and a v-cavities shadowing term (MDL spec 21.1.3) — close to Beckmann,
not GGX. Octane's Universal uses GGX by default, with longer tails and a
sharper peak at the same roughness (#56).

## Other DAZ MDL files

In the same folder: `pbr_skin.mdl` (PBRSkin, [pbrskin.md](pbrskin.md)),
`dual_lobe_hair.mdl` (Blended Dual Lobe Hair), `fiber_chiang_hair_fur.mdl`,
`pbrmaterial.mdl`, `../OmniSurface/` (OmniSurface, OmniHair) and
`../nvidia/` (NVIDIA's MDL examples, `core_definitions`,
`legacy_thin_film`).
