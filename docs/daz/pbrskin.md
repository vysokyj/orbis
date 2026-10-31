# PBRSkin (Iray)

DAZ's skin shader from 2020, on every Genesis 9 and Genesis 8.1 figure.
DAZ Studio 6 ships it as `shaders/iray/daz_3d/pbr_skin.mdl` (in the app
bundle). The channels and their defaults are in
`data/DAZ 3D/Built-in Content/Daz Iray PBRSkin/PBRSkin.dsf`. Its parts are
stacked as below (summarised from the MDL, read 2026-10-31; #57).

## Surface, bottom up

1. **Diffuse ⊕ translucency.** With `Translucency Enable` the surface is a
   weighted layer: the diffuse reflection (`Base Color`, `Diffuse
   Roughness`) at weight 1 − `Translucency Weight`, over a diffuse
   *transmission* tinted by `Translucency Color`. Like Iray Uber's
   translucency: light that goes in enters the volume below. No Base
   Color Effect, no SSS Mode, no Base Mixing.
2. **Makeup.** A second diffuse reflection (`Makeup Base Color`) weighted
   by `Makeup Weight` on top of 1. Its roughness multiplier also scales
   the specular roughness (lerp 1 → `Makeup Roughness Mult` by the weight).
3. **Dual lobe specular.** Two GGX lobes:
   - roughness r₁ = `Specular Lobe 1 Roughness`, times the makeup and
     detail multipliers;
   - r₂ = r₁ × `Specular Lobe 2 Roughness Mult`;
   - both squared.

   `Dual Lobe Specular Ratio` weighs lobe 2 over lobe 1 (#26). They sit
   on 1–2 as a Fresnel-curve layer: normal reflectivity ×
   `Specular Occlusion Facing` (lerped by `Ambient Occlusion Weight`),
   grazing 1 × `Specular Occlusion Grazing`, weight `Dual Lobe Specular
   Weight`. The normal reflectivity comes from `Dual Lobe Specular
   Reflectivity` (the MDL's default `specularlevel` is 0.08, F0 of
   IOR ≈ 1.8; the slider's mapping to it is DAZ's).
4. **Metallic.** `Metallic Weight` blends the lobes tinted by `Base Color`
   (the makeup colour blended in with `Makeup Metallicity Enable`) over
   all of the above.
5. Separate materials in the same file add clear coat (`Top Coat *`) and
   metallic flakes on top, as Iray Uber's.

`Detail *` adds a tiled detail normal and a roughness multiplier.
`Specular Occlusion *` dims the lobes by an occlusion map.

## Volume

As Iray Uber's chromatic SSS, always logarithmic:

- σa = −ln(`Transmitted Color`) / `Transmitted Measurement Distance`;
- σs = −ln(`SSS Color`) / `Scattering Measurement Distance`;
- distances in DAZ cm, ×0.01 to meters;
- `SSS Direction` the phase function's bias;
- each part off with `Transmission Enable` / `Sub Surface Enable`.

Defaults: `SSS Color` (0.976, 0.694, 0.761) over 0.015, `Transmitted
Measurement Distance` 0.12 — Victoria 8's Uber volume. Genesis 9 skins
(Amelia 9) leave `SSS Color` and the scattering distance at these
defaults and set Transmitted Color / Distance.

## For the Octane mapping

PBRSkin isn't a new skin model. It is Iray Uber's skin (diffuse over
diffuse transmission, volume below) with fewer options: chromatic SSS
only, no Base Color Effect, a makeup layer, and the dual lobe as the
only specular. OpenPBR's subsurface is one random walk with colour and
radius, so the mapping still needs Iray's look of the layered diffuse
plus volume. That is what `skin_albedo` models for chromatic Uber skins
(fitted on eight Genesis 8 skins).
