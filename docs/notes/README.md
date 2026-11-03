# Notes — the calibration lab notebook

Dated, detailed notes of calibration sessions: what was tried, in what
order, the numbers, the dead ends and why they were dead, and the process
lessons. **Not normative** — the [spec](../spec/octane-export.md) decides;
a note only records how a rule was found, so it can be re-checked or
re-opened later. Open work stays in
[GitHub issues](https://github.com/vysokyj/orbis/issues).

All ratios are Octane / Iray in linear luminance (sRGB PNG, gamma 2.2,
median over a region) unless a note says otherwise; 1.00 = equal.

| Note | Topics |
|---|---|
| [2026-10-27 – 28](2026-10-27-hair-variants-eyes-thin-walls.md) | strand-hair width and colour, the four variants, the cornea, the caruncle and Base Color Effect 1, Calib-Thin |
| [2026-10-28 skin tones](2026-10-28-skin-tones.md) | Calib-Skin (eight skins), Iray's skin model, Octane's gloss and transfer, Iray Server |
| [2026-10-28 NVIDIA MDL](2026-10-28-nvidia-mdl.md) | NVIDIA's dielectric MDL examples (concrete, porcelain, granite, resin, architectural), the stale Metal Brushed reference |
| [2026-10-29 OmniHair](2026-10-29-omnihair.md) | DAZ's OmniHair → Octane Hair in melanin mode, Calib-OmniHair, Octane's pheomelanin |
| [2026-10-29 VDB volumes](2026-10-29-vdb-volumes.md) | DAZ's Simple OpenVDB MDL Volume → Octane Volume + Volume medium, Calib-VDB, Octane's ray-march step, per-meter coefficients |
| [2026-10-29 displacement](2026-10-29-displacement.md) | Iray displacement (strength × lerp(min, max, map)), DAZ's automatic gamma 2.2 for maps, Octane vertex displacement, Calib-Disp / Calib-Disp-Map, a figure's clothes (teens), Octane's "kernel failed" (#50) |
| [2026-10-29 wardrobe fit](2026-10-29-wardrobe-fit.md) | Wearables in a stale fit in the dump (boots off the feet, toes through shoes): processEvents + forced cache rebuild (#51) |
| [2026-10-29 Base Mixing 1 and Morgan Hair](2026-10-29-spec-mode-hair.md) | Glossy Specular as F0 (Octane IOR), Calib-SpecMode, Calib-Hair-Teens, the hair's top coat and colour effects (#52) |
| [2026-10-30 node flags](2026-10-30-node-flags.md) | Renderable, hidden surfaces and facet groups (NGV8 shells, `WG_<k>_` graft facets), Cast Shadows as an Octane object layer (#36) |
| [2026-10-30 skin volume](2026-10-30-sss-volume.md) | SSS Color / Mode / Amount on Calib-Skin's variants (Victoria, Monique, Zurie): diffusion reflectance factor on the baked skin, mono skins with Base Color Effect 0 baked (#28) |
| [2026-11-01 G9 eye (WIP)](2026-11-01-g9-eye-wip.md) | Genesis 9 eye: PBRSkin translucency as colour, layered-image layers at their own size, where calibration stood (#66) |
| [2026-11-02 slow Iray render](2026-11-02-option-scenes.md) | Calib-RS-Bloom took 1433 s at 5000 samples (usual 105–112 s), other slow option scenes |
| [2026-11-03 map offsets](2026-11-03-map-offset.md) | UV offset sweep (Calib-Map-Offset): translation = (−offset_u / tiles_u, offset_v / tiles_v) (#65) |
| [2026-11-03 dome tilt, spot shapes](2026-11-03-dome-tilt.md) | Dome Orientation X / Z as a Spherical projection (type 77) with Iray's Rz · Rx · Ry, order ZXY; a rect spot as an Octane quad (#65) |
| [2026-11-03 vignetting, per-component burn](2026-11-03-vignetting-burn.md) | Iray cos(θ)^v vs Octane's frame-space cone (least-squares fit per camera); luminance burn as a 3D tone LUT (#38) |
| [2026-11-03 anisotropy](2026-11-03-anisotropy.md) | Iray `anisotropy_conversion` (u = r, v = r / (1 − a)) → Octane roughness = the long axis, anisotropy (v − u)/(v + u); backscattering anisotropy never reaches Iray; Calib-Aniso2 (#23) |
| [2026-11-03 diffuse roughness](2026-11-03-diffuse-roughness.md) | Iray Oren-Nayar σ → Universal `diffuseBrdf` 2 where the roughness pin is free, else albedo × A(σ); overlay roughness (#32) |
| [2026-11-03 dispersion](2026-11-03-dispersion.md) | Iray Abbe number → Universal `dispersion_coefficient_B` (Abbe mode) as a float value node; smooth transmission with dispersion; Calib-Dispersion-Rough (#33) |
| [2026-11-03 light options](2026-11-03-light-options.md) | DAZ hands Iray only Render Emitter (`visible`) and Temperature of the light options; Octane hides USD lights from the camera unless an object layer shows them; portals gone from Iray (#35) |
| [2026-11-03 environment options](2026-11-03-environment-options.md) | .mi of ground fog / matte fog / dome modes; ground fog as Unit Volume slabs (Post FX fog crashed); dome ground as an emitting SDF box, projection mirrored in X; matte fog, finite dome, lighting resolution / blur not mapped (#37) |
