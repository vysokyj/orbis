# 2026-11-22 — Gloss over a dual lobe under a partial refraction (#220)

Seen while fixing #218: `gloss_over_dual_lobe` read Refraction Weight off
the material, so wherever a partial refraction splits into the refraction
Mix (`refractive_metal_material`), the base part (refraction 0 there) with
a glossy layer took the gloss on `specular` alone and dropped the dual
lobe - the gloss-side twin of #216. The #218 scan of the installed content:
320 materials (strand hair over translucency, already a Mix) and 3 that
stay one node (a G8M beard's Hair1 - 3: refraction 0.5, no translucency).

## Docs first

`docs/daz/iray-uber-mdl.md` (`metallic_layered_base`, bottom to top):
diffuse / translucency, the dual lobe (`add_dual_lobe_specular`), the
glossy layer (`add_top_coat` in custom-curve mode) over that, backscatter,
metal, and the refraction as a `weighted_layer` at its weight over
everything. So the refraction's share holds the refraction alone and the
rest is the plain dielectric of #106: gloss over lobe over diffuse.

## The fix (no constant)

- `gloss_over_dual_lobe(mat, refraction)`: the refraction weight the node
  holds (`refraction_override` inside the Mix, else the channel). Its three
  users (`universal`, `skin_gloss`, `layered_top_coat`) pass it; the Top
  Coat case moves the base part into the Layered material as on a plain
  dielectric.
- The one-node case takes the Mix too (#218's `partial_over_lobe` extended
  to `gloss_over_dual_lobe(mat, 0)`, Base Mixing 0 only - Base Mixing 1
  keeps the gloss alone even without refraction, so a split would not
  bring the lobe back).
- Test `refraction_mix_base_keeps_gloss_over_its_dual_lobe` (over
  translucency and without).

The exported `project.ocs` of Calib-Strand-GlossLobeArc-HDRI, row G:

| Material | before (from the code path) | after |
|---|---|---|
| Pixie + gloss (Mix) base | specular 0.3 (gloss), no lobe | specular 0.526 (lobe, roughness 0.549, IOR 1.45), coating 0.3 (gloss, 0.30, IOR 1.5) |
| Pixie + gloss, no translucency | one node, specular 0.3, lobe lost | Mix 0.284: refraction part specular 0.3 + transmission 1; base as above |
| no lobe twins | unchanged | unchanged |

## New scenes

Calib-Strand-GlossLobeArc-HDRI / -T2-HDRI (`make-strand-scenes.py`,
materials 38 - 41): -LobeArc's arcs (THIN widths, #217) under DTHDR-RuinsB
and its sun; columns Pixie + a white glossy layer 0.3 (reflectivity 0.5,
roughness 0.3), the same without its dual lobe, without its translucency,
without both. 1 / 2 is the lobe's share under gloss + refraction in the
strand-hair case, 3 / 4 the beard's.

## Octane (400 samples, denoised, cell sums of linear luminance)

| Faithful fibres | G | H | I | J |
|---|---|---|---|---|
| 1 Pixie + gloss | 41.21 | 51.06 | 48.08 | 12.69 |
| 2 no lobe | 21.73 | 29.12 | 28.48 | 9.20 |
| 3 no translucency | 26.96 | 41.07 | 46.19 | 14.80 |
| 4 no transl., no lobe (one node) | 31.72 | 35.83 | 33.37 | 11.93 |
| lobe share 1 / 2 | 1.90 | 1.75 | 1.69 | 1.38 |
| 3 / 4 | 0.85 | 1.15 | 1.38 | 1.24 |

Ribbons (-T2, Faithful) 1 / 2: 1.81 / 1.68 / 1.61 / 1.30; 3 / 4 the same
as fibres (0.85 / 1.15 / 1.38 / 1.24). Enhanced fibres 1 / 2: 1.80 / 1.66
/ 1.61 / 1.36; 3 / 4: 0.80 / 1.10 / 1.32 / 1.22.

The 1 / 2 share is -LobeArc's (Pixie opaque, no gloss: 1.71 / 1.62 /
1.53 / 1.14) a little higher. Column 3 against 4 compares a Mix with a one
node: on the thick arc (G) the Mix is darker though it adds a lobe, so the
Mix's split itself (refraction part transmission 1 at 0.284 against one
node's transmission 0.284; gloss as coating against gloss on specular)
moves the result - column 2 (Mix, no lobe) against 4 (one node, no lobe)
differs by the translucency and the split: 0.69 / 0.81 / 0.85 / 0.77. Iray
decides which form is right; no tuning before.

## Iray

Not rendered: the home Iray Server is still on its CPU fallback (#217;
the last jobs, -LobeArc / -PixieArc, rendered on the CPU). Parked as a
`## Iray batch` on #220 with `needs-iray`.

## Process

- A predicate on the material (`gloss_over_dual_lobe`, like #216's flag)
  is wrong for every split part that overrides what it reads; pass the
  part's value in.
