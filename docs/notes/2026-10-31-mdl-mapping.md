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

## #25: Fresnel and Custom Curve coats as Iray gets them

`top_coat_ior`: Fresnel → 1.5 always, Custom Curve → a fixed IOR.
Calib-TopCoat, Octane / Iray (spheres T1 – T8, compare-all):

| | T1 | T2 | T3 | T4 | T5 | T6 | T7 | T8 |
|---|---|---|---|---|---|---|---|---|
| before (full pass) | 1.09 | 0.10 | 0.05 | 0.15 | 1.10 | 2.06 | 2.49 | 2.55 |
| Fresnel 1.5, curve 1.22 | 1.10 | 1.04 | 1.01 | 1.04 | 1.11 | 1.11 | 1.52 | 1.54 |
| curve 1.07 | 1.10 | 1.04 | 1.01 | 1.04 | 1.11 | 1.11 | 1.23 | 1.23 |
| curve 1.0 | 1.10 | 1.04 | 1.04 | 1.05 | 1.11 | 1.11 | 1.18 | 1.19 |

(The "before" row is the 2026-10-31 full pass, before the weighted coat
fix made T3 / T4 right.) 1.22 matches Schlick's curve from 0 on the
hemispherical average (1/21); Iray's curve spheres show no head-on
reflection at all, so it was too much. 1 removes the coat and leaves
1.18: the rest is not the coat. Kept 1.07 for the rim Iray shows.

Measuring pitfall: a scratch script's whole-sphere means disagreed with
compare-all, which averages the middle 60 % of each sphere and leaves out
clipped pixels (both decode at gamma 2.2). Quote compare-all.

## #56: GGX and a roughness curve

Probe 104: Universal's `brdf` enum is 0 Octane (the default), 1 Beckmann,
2 GGX, 6 GGX energy preserving, 7 STD; `diffuseBrdf` defaults to
Lambertian (Iray: Oren-Nayar, #32). Octane was never on GGX.

Variants (`orbx-variants.py brdf=<n>`, `rough-pow=<p>`, `a&b`), measured
with `highlight-spread.py` (p99 / median per sphere, `--vs` luminance):

| Calib-DualLobe2 | E4 | E6 r .81 | E7 r .36 | E8 r .6 unsq. |
|---|---|---|---|---|
| Iray spread | 1.3 | 1.5 | 1.3 | 1.7 |
| default BSDF, r: spread / lum | 1.5 / 1.47 | 2.6 / 1.99 | 2.6 / 1.58 | 3.3 / 1.92 |
| Beckmann, r | 1.9 / – | 2.4 / – | 4.9 / – | 4.8 / – |
| GGX, r | 1.6 / – | 1.5 / – | 3.6 / – | 2.3 / – |
| GGX, √r | 1.4 / 1.07 | 1.5 / 0.83 | 1.4 / 1.25 | 1.8 / 1.05 |
| GGX, r + 1.85 r² | 1.5 / 1.02 | 1.5 / 0.83 | 1.5 / 1.25 | 1.5 / 0.69 |

Beckmann sharpens (the opposite of the hypothesis from Phong ≈ Beckmann);
the roughness scale is what differed. √r fixed the rough spheres but made
the sharp ones broad and bright (Calib-Glossy Reflectivity 1: 1.21 →
1.62, SpecMode 1.06 → 1.14). The curve r + 1.85 r² keeps GGX at about r
for sharp lobes and √r for rough ones: Calib-Glossy 1.02 – 1.16 (G7 1.47
→ 1.17), SpecMode 0.95 – 1.10, Mixing 1.03 – 1.20, TopCoat unchanged.
Worse: E8 (Roughness Squared off) 0.69 and Calib-ColorEffect C8 1.35 →
1.77 (#27). Calib-Materials has no cached dump, not checked.

## #27: C8, Base Mixing 1 with a red Glossy Specular

The .mi: `glossy_reflectivity` 0.793 (the red channel, linear),
`glossy_color` white, the hue (1, 0.089, 0.037) as
`glossy_directional_normal_color` only. Iray's diffuse under it is
1 − 0.793 head-on in every channel. Octane rendered C8's R right (0.366 /
0.366) and G / B 1.5 / 2.9 ×: under a tinted specular Universal's diffuse
keeps 1 − F·t per channel, so the blue base showed through. Albedo ×
(1 − w·F0) / (1 − w·F0·t) (variant `set=C8_mode1_specular.albedo=x1,0.2226,0.2131`,
then `specular_diffuse_share`): C8 1.77 → 1.08 (R / G / B 1.05 / 0.82 /
0.94), Calib-SpecMode's Morgan spheres 1.09 → 1.03, the rest unchanged.
