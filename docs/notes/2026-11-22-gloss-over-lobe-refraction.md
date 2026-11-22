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

## The Iray side (resume)

Iray references b3177d0 (server GPU, 2000 samples, denoised on@8). The
same cells as the Octane table above (rows 75-155 / 260-350 / 500-595 /
855-945, columns 180-380 / 650-820 / 1100-1270 / 1545-1740, sum of linear
luminance, PNG decoded with gamma 2.2; the Octane numbers above come out
exactly again). Iray's PNG carries alpha: weighting its edge pixels by
alpha moves the Iray ratios by 0.01 - 0.03 on fibres (ribbons H up to
0.12), so the RGB as `hair-set-compare.py` reads it stands.

| fibres (-HDRI), G / H / I / J | Iray | Faithful | Enhanced |
|---|---|---|---|
| lobe share 1 / 2 | 1.66 / 1.56 / 1.58 / 1.36 | 1.90 / 1.75 / 1.69 / 1.38 | 1.80 / 1.66 / 1.61 / 1.36 |
| 3 / 4 (Mix + lobe / one node) | 1.20 / 1.34 / 1.34 / 1.22 | 0.85 / 1.15 / 1.38 / 1.24 | 0.80 / 1.10 / 1.32 / 1.22 |
| 2 / 4 (Mix / one node, no lobe) | 0.82 / 0.86 / 0.86 / 0.75 | 0.69 / 0.81 / 0.85 / 0.77 | 0.74 / 0.88 / 0.90 / 0.82 |

| fibres, Octane / Iray per column | Faithful | Enhanced |
|---|---|---|
| 1 Pixie + gloss | 1.87 / 1.85 / 1.44 / 1.42 | 1.91 / 1.90 / 1.45 / 1.50 |
| 2 no lobe | 1.64 / 1.64 / 1.35 / 1.41 | 1.76 / 1.78 / 1.43 / 1.51 |
| 3 no translucency | 1.39 / 1.48 / 1.41 / 1.39 | 1.32 / 1.42 / 1.34 / 1.37 |
| 4 no transl., no lobe | 1.96 / 1.74 / 1.36 / 1.37 | 1.97 / 1.74 / 1.36 / 1.37 |

Ribbons (-T2) in Iray: 1 / 2 1.81 / 1.43 / 1.51 / 1.27 (Faithful 1.00 /
1.17 / 1.07 / 1.02 x that), 3 / 4 0.76 / 0.98 / 1.07 / 1.14; Octane /
Iray per column 0.65 - 1.08 (Octane draws -T2 as round curves, #79).

- **The fix holds.** The lobe's share under a glossy layer and a partial
  refraction is Faithful 1.14 / 1.12 / 1.07 / 1.01 x Iray's on fibres,
  Enhanced 1.08 / 1.06 / 1.02 / 1.00; on ribbons 1.00 - 1.17. The same
  small excess on the thick arc as -LobeArc's (#216: 1.04 x there), now
  under a refraction too. No constant; #220 closed.
- **Column 3 against 4 is not a #220 question.** In Iray the Mix with its
  lobe is brighter than one node without it on every fibre row (1.20 -
  1.34); Octane's 0.85 on G comes from column 4, the one node, at 1.96 x
  Iray (H 1.74), while the Mix without translucency sits at 1.39 - 1.48 x
  on every row. Every column with a partial refraction is 1.35 - 1.96 x
  Iray on fibres, where -LobeArc (no refraction) was 1.07 - 1.24 x
  (measured the same way). Iray manual: on fibre geometry a regular BSDF
  is single sided and opaque, its transmission discarded - the refraction's
  share as well as the translucency's. Faithful + Final drops only the
  translucency on fibres; the refraction part (transmission 1) and the one
  node's transmission still pass light. Filed separately (the refraction on
  Iray fibres, #135 / #165 shape): #222.

## Process (resume)

- Before a Mix-against-one-node question, look at each column against
  Iray, not only ratios inside one renderer: the "split" here was one
  column's transmission.
