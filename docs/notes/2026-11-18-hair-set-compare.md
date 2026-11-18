# 2026-11-18 — The hair set against Iray; translucency on ribbons (#161)

Resume of #161 with the eight hair-set references at 2000 samples
(Calib-Hair-Pixie, -Kevin, -Hudson, -Athena, each with -nogloss; the
store scenes in `calib/scenes-store`). Octane: the 2026-11-17 renders
(256 samples, denoised, both Looks), no export change since.

Patch table: `scripts/dev/hair-set-compare.py` (boxes inside the hair on
the 1280 x 720 frame, linear, R / G / B and luminance). Kevin and Hudson
are ribbons (Line Tessellation 2, Morgan's vendor), Pixie prisms (3),
Athena fibres (1).

## The hair set as it was (luminance, Octane / Iray)

| hair | patches | Faithful = Enhanced | -nogloss | Iray gloss share | Octane gloss share |
|---|---|---|---|---|---|
| Kevin (ribbons, w 0.10) | crown, front, curls, shaved side | 0.71 - 0.84 | 0.68 - 0.80 | 1.01 - 1.06 | 1.06 - 1.12 |
| Hudson (ribbons, w 0.48) | crown, front, side | 0.69 - 0.78 | 0.65 - 0.72 | 0.98 - 1.02 | 1.08 |
| Pixie (prisms, w 0.33, dark) | crown, fringe, side | 0.77 - 1.04 | 1.04 - 1.21 | 1.24 - 1.55 | 1.06 - 1.14 |
| Athena (fibres) | - | hair gone in Octane (#202) | | | |

- Morgan (2026-11-15) was 0.79 - 1.27 against its ribbons: **one level
  does not fit the ribbon hairs** - Kevin and Hudson are both darker.
  Hudson's B sits 0.1 - 0.2 above R / G (Octane bluer).
- Iray's gloss on Kevin's and Hudson's ribbons hardly shows in these
  patches (1.00 ± 0.02; a narrow anisotropic streak covering few pixels,
  the full-frame difference max 0.06 / 0.26 linear on 0.3 % / 0.2 % of
  the pixels); Octane's adds 6 - 12 % across them. Pixie's dual lobe in
  Iray is 1.24 - 1.55 (B 1.5 - 1.9: the dome's sky in the sheen), Octane's
  1.06 - 1.14.
- Athena: Octane's -op1 probe (Cutout Opacity 1, no map) against Iray's
  base: 0.63 - 1.00, B 0.68 - 0.92. Only a rough read (Iray keeps its
  cutout); the scene's -op1 reference is #204's.

## The cutout on Octane's side (Kevin, Hudson)

Octane only, Faithful, the -op1 scenes (`scene-channels.py ... "Cutout
Opacity=1" "Cutout Opacity=nomap"` on the hair materials, the base
scene's dump reused - the dump holds no materials; local, My Library,
not in git: new store scenes need the user's yes):

| | Kevin crown / front / curls / shaved | Hudson crown / front / side |
|---|---|---|
| op1 / base (Octane) | 1.08 / 1.12 / 1.09 / 1.14 | 1.01 / 0.92 / 1.02 |
| op1 / Iray base | 0.81 / 0.92 / 0.78 / 0.96 | 0.70 / 0.68 / 0.80 |

The maps: Hudson's TR2 varies across u only (one opacity per strand,
mean 0.53; Octane reads it near 1, so op1 changes little). Kevin's TR2
varies along v, and the hair's guide lines carry real UVs (u constant per
strand, v 0.012 → 0.501 on half of them and 0.501 → 0.990 on the other
half: means 0.82 and 0.26 along the strand). Kevin's render hairs are
grown from the guides (PreRender Hairs), so which v a render hair gets
is not in our dump. Iray's own cutout share needs Iray -op1 references
of these store scenes (CPU on the home server, ~50 min each): not now.

## Decomposition on synthetic strands (Calib-Strand-UberA-*)

`make-strand-scenes.py`: rows G (one thick strand), H (300 sparse), I
(3000 dense), J (300 vertical) of plain diffuse 0.2 / 0.5 / 0.85 and
0.85 at Translucency Weight 0.48 (Translucency Color 0.85: Hudson's
weight), under Calib-Aniso's dome; -Dome fibres, -T2-Dome ribbons. Iray
2000 samples on the home server (1145 s the ribbons), in git.

Octane Faithful / Iray, cell sums:

| | 0.2 | 0.5 | 0.85 | 0.85 tl |
|---|---|---|---|---|
| fibres G / H / I / J | 1.07 / 0.99 / 0.97 / 1.08 | 1.04 / 0.96 / 0.93 / 1.04 | 1.04 / 0.95 / 0.91 / 1.03 | 1.04 / 0.96 / 0.92 / 1.03 |
| ribbons G / H / I / J | 1.23 / 1.00 / 0.82 / 1.40 | 1.19 / 0.98 / 0.80 / 1.25 | 1.20 / 1.02 / 0.81 / 1.21 | 0.89 / 0.76 / 0.60 / 0.91 |

- **The albedo does nothing**: against fibres and against ribbons the
  ratio is the same at 0.2, 0.5 and 0.85. The hypothesis that blond hair
  lands lower than dark for its colour is refuted.
- Octane's curves are Iray's fibres (0.91 - 1.08). Iray's ribbons
  against its fibres: G 0.87, H 0.93 - 1.00, I 1.11 - 1.18, J 0.76 -
  0.85 - **the ribbons' own shape**: brighter in a dense patch, darker as
  one strand. That is the 0.80 Octane shows on dense ribbons.
- **Translucency**: Iray's ribbon gives it back (0.85 tl / 0.85: 1.08 /
  1.04 / 0.93 / 1.01), Octane's curves don't (0.80 / 0.78 / 0.69 /
  0.75). Iray's fibres drop it (Faithful already writes none there: 1.04
  / 0.96 / 0.92 / 1.03 x Iray).
- Octane probes on the ribbons' column 3 (`orbx-variants.py set=...`):
  albedo 0, transmission = the colour (Thin Wall (Diffuse) or Diffuse,
  identical) against albedo = the colour: **0.76 / 0.69 / 0.44 / 0.79**.
  Under a dome a thin sheet passing ρ looks as one reflecting ρ; Octane's
  curves pass a diffuse transmission at under half in a dense patch.
  Transmission x 2 overshot to 1.7 - 2.5 x Iray (not a fix: Universal
  passes more than it gets). `transmissionType` 1 = 3 on curves.

## Rule (Faithful + Final, ribbons only)

The translucency stays in the diffuse on ribbon strands (albedo without
the 1 − w, no transmission), `ribbon_reflects` in `src/material.rs`, a
`Material::ribbons` flag from the Line Tessellation sides (2). Enhanced
keeps the transmission (back-lit glow), prisms keep it in both.

- Calib-Strand-UberA-T2-Dome, the translucent column: 0.89 / 0.76 / 0.60
  / 0.91 → 1.11 / 0.99 / 0.90 / 1.13 (the opaque columns 1.19 - 1.23 /
  0.98 - 1.02 / 0.80 - 0.82 / 1.21 - 1.40: the ribbons' shape).
- Hudson Hair: 0.69 / 0.74 / 0.77 → 0.78 / 0.79 / 0.92; -nogloss 0.65 -
  0.72 → 0.76 - 0.89. What is left is about the dense-ribbon 0.80.
- Kevin Hair (w 0.10): crown / front / curls unchanged (0.72 - 0.82), the
  shaved side 0.84 → 1.03.
- First tried on every tessellated strand: Pixie Hair (prisms, w 0.33)
  went 0.77 - 1.04 → 1.06 - 1.46 (-nogloss 1.49 - 1.76). Light entering a
  prism leaves through a second face (the hair shader's tint² on prisms,
  Calib-Strand-UberT-T3), so the rule is for ribbons only; Pixie is back
  at its old numbers.

## What is left on ribbons

- The ribbons' shape in a pile: Octane 0.80 on a dense synthetic patch,
  Hudson 0.78 - 0.92, Kevin 0.72 - 0.82 (+ its cutout, 1.08 - 1.14 on
  Octane's side). Whether Faithful should carry it is the user's call
  (#79 decided round curves; a level for the pile would be a geometry
  factor, albedo-free, but it differs by depth: thick 0.87, dense 1.15).
- Octane's gloss on ribbons adds 6 - 12 % where Iray's shows ~0.
- The cutout along the strand (needs UVs on the curves; Kevin's render
  hairs carry the guides' v), with #202 for fibres.

## Process

- `wait-for.sh` with a 600 s tool timeout is moved to the background
  after 180 s; call it again until its `exited with code` line.
- A dump can serve a material-only variant scene: copy it under the new
  scene's cache key (`dump_file_name`, FNV-1a of the absolute path).
- Renders: Octane ~1 min per hair at 256 samples; Iray (home server, GPU)
  1145 s for the ribbon scene at 2000 samples.
