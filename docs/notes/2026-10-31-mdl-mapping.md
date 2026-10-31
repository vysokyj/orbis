# 2026-10-31 — Iray Uber: DAZ's MDL and the channel mapping from .mi

## Sources

- DAZ ships the Uber shader as MDL source:
  `DAZStudio.app/Contents/shaders/iray/daz_3d/irayubermaterial.mdl`. Its
  layer stack is in `docs/daz/iray-uber-mdl.md`.
- NVIDIA's Iray and MDL docs, mirrored by `scripts/dev/fetch-nvidia-docs.py`
  (git-ignored `docs/nvidia/`). The site leaves out its intermediate
  certificate; only macOS's `/usr/bin/curl` fetches it.
- The channel → parameter code is compiled
  (`libdsp_dzirayrenderer.dylib`, stripped). We did not disassemble it.

## The .mi route

DAZ Studio has `DzMIExporter` in its export manager (".mi Mental Images
Scene Format"). `scripts/dev/daz-export-mi.dsa` exports scenes headless
(`App.getExportMgr()`, `writeFile`; a few seconds per scene). Each
material comes out as
`mdl::daz_3d::irayubermaterial::iray_uber_unified(...)` with the arguments
that differ from the MDL defaults. `scripts/dev/mi-args.py` tabulates
them. Exported: the 23 calibration scenes plus five new sweep scenes,
`Calib-Map-Metal / -TopCoat / -Effect / -SpecMode / -Misc` (no Iray
references, mapping only).

Shell pitfall: under zsh `$S` with spaces stays one argument; pass scene
lists as an array.

## Findings

The full table is in `docs/daz/iray-uber-mdl.md`. What changed or settled
an open question:

- **Top Coat IOR, Curve Normal / Grazing / Exponent and Top Coat Specular
  are never passed.** Fresnel mode is always IOR 1.5 (Calib-TopCoat's Iray
  T5 at 1.5 and T6 at 2: 0.102 and 0.104), Custom Curve at the MDL
  defaults (head-on 0). That explains the curve spheres Octane renders
  2.1 – 2.6 × Iray (#25). Not because the channels are hidden: hidden
  ones such as Reflectivity and Glossy Specular do pass.
- **Metal**: `metallic_weight` = w goes to both the layer weight and the
  metal coat's weight, so the reflection scales with w² (#59; Calib-DataGamma
  G6 1.37 × in Octane). Head-on = the base colour's strongest channel,
  its hue as a directional colour. Glossy colour and reflectivity don't
  reach the metal.
- **Base Mixing 1** is converted to layered: reflectivity = max(Glossy
  Specular^2.2), its hue a directional colour, Metallic Weight dropped.
- Reflectivities × 0.08, every roughness squared, colours ^2.2, Diffuse
  Strength scales the colour, overlay "squared" squares the weight,
  measurement distances and flake size cm → m. SSS Amount stays as set.
- Backscattering Color only with Share Glossy Inputs off (#29).

## Next

- #25: export Fresnel / Custom Curve coats as Iray gets them (IOR 1.5,
  curve head-on 0), then recheck Calib-TopCoat T5 – T8.
- #59: the w² metal.
- Converting .mi to Octane directly: an option to weigh (see #60).
