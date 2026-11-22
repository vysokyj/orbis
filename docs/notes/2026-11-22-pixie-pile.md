# 2026-11-22 — Pixie's pile on the arcs (#221)

#216 / #222 left on Calib-Hair-Pixie-T1 (Pixie Hair as fibres): the side
patch's dual-lobe share 1.67 in Faithful against Iray's 1.22 (crown /
fringe 1.42 / 1.36 vs 1.23 / 1.26), over a base (-nogloss) 0.69 - 0.75 x
Iray's - while on Calib-Strand-LobeArc-HDRI (Pixie's material opaque, THIN
widths 0.2 / 0.1) the share matches (1.02 - 1.04 x). Goal: split what the
real hair adds over the arcs before anything is fitted. Faithful +
Enhanced, Final, 400 samples, denoised.

## What separates the arcs from Pixie

From Calib-Hair-Pixie-T1's dump: 174 040 strands of 18 (28) points, 4.4 cm
on average, 765 000 cm of hair in about 2100 cm3 (0.3 cm voxels): about
360 strands per cm2 across the layer. At Pixie's widths (0.05 -> 0.02 mm,
0.035 mean) that is 1.3 strand widths per cm of path - the same coverage
as -LobeArc's 3000 THIN arcs (83 per cm2 at 0.15 mm, 1.25 per cm). So the
pile's optical density is not new; new are (a) strands 4 x thinner and 4 x
as many, (b) a thin layer (1 - 2 cm) on an opaque cap / head instead of
black behind, (c) the material: -LobeArc drew Pixie *opaque*, the real
hair carries Hair01's translucency 0.33 and its refraction 0.28 (no glossy
layer, so `refraction_reflection_enable` off: the refraction's share is
black on a fibre, #222). Pixie-T1-nogloss keeps them (only the lobe off).

## Calib-Strand-Pile-HDRI

New in `make-strand-scenes.py`: -LobeArc's arcs over the top under the
same dome and sun, each row a small layer (6 cm across, 1.5 cm deep, radius
8): G THIN at -LobeArc's density (750), H Pixie's widths at Pixie's density
(3000; same coverage per cm), I = H over a cap (a half cylinder just under
the arcs, plain diffuse in Pixie's colour), J = G over the cap. Columns:
Pixie opaque with / without its lobe (materials 33 / 35), Pixie whole
with / without it (31 / 42). 30 000 strands in all (the 12 000-strand
-PixieArc rows crashed the server's GPU, #217). Cells measured with the new
`scripts/dev/strand-cells.py` (mean linear luminance x 1000 over the
silhouette; on the arcs it gives -LobeArc's Faithful share 1.02 - 1.05 x
Iray's, as #216 found with whole-cell sums).

Octane (Faithful, Final, denoised), cells and shares:

| row | 1 opaque | 2 opaque, no lobe | 3 whole | 4 whole, no lobe | share 1/2 | share 3/4 | 4/2 |
|---|---|---|---|---|---|---|---|
| G thin | 10.76 | 7.83 | 6.91 | 3.98 | 1.38 | 1.74 | 0.51 |
| H Pixie widths | 10.11 | 7.54 | 6.71 | 3.92 | 1.34 | 1.71 | 0.52 |
| I Pixie widths, cap | 10.49 | 8.24 | 7.42 | 5.51 | 1.27 | 1.35 | 0.67 |
| J thin, cap | 10.92 | 8.63 | 7.66 | 5.67 | 1.27 | 1.35 | 0.66 |

Enhanced: shares 1.34 / 1.31 / 1.25 / 1.25 (opaque) and 1.52 / 1.55 /
1.31 / 1.32 (whole). The raw pass gives the same to 0.01.

Inside Octane:
- **Width alone does next to nothing**: G -> H (4 x thinner, 4 x as many)
  moves the base 0.96 x, the share 1.38 -> 1.34.
- **The cap** adds 9 % to the opaque base and lowers the share
  (1.34 -> 1.27); under the whole material it adds 40 % (the cap's light
  counts more where the strands reflect less).
- **The whole material halves the base**: its exported base part has
  albedo 0.67 x the opaque one (the translucency's share dropped on fibres,
  #79) and the refraction part (0.284) is black (no reflection, #222), so
  0.72 x 0.67 = 0.48; measured 0.51 - 0.52. With the lobe unchanged the
  share rises to 1.71 - 1.74 - the size of Pixie-T1's side patch (1.67).

So the share Octane draws on Pixie follows its base, and the base on
fibres follows what the export does with Hair01's translucency and its
reflection-less refraction. Whether Iray loses as much there is not
measured: every Iray fibre reference with Pixie's refraction so far had a
glossy layer (-GlossLobeArc, where the refraction keeps its reflection),
and -PixieArc (the whole material, no gloss) never rendered (#217). If
Iray's 4/2 is near Octane's 0.5, the base gap is the pile (rows); if it is
higher (0.7 would make Octane's base 0.71 x, as Pixie-T1-nogloss's 0.69 -
0.75 x), the gap is the material rule on fibres, not the geometry.

Iray: the home server is on its CPU fallback (the last job, -PixieArc-T2,
hit a CUDA illegal memory access in post-processing), so the scene is
parked as an `## Iray batch` in #221.

## Process

- Count the real hair from its dump before building a stand-in: the
  "4 x thinner, much denser" pile turned out to have the arcs' coverage,
  which takes density off the list and leaves width, the cap and the
  material.
- A material twin of what the real hair carries (here Pixie whole, not
  opaque) belongs in every decomposition scene; the arcs had only the
  opaque one, so they could not see the material's loss on fibres.

## The Iray side (resume)

Iray reference: GPU, 2000 samples, denoised (0a2e8a2). Cells over Iray's
own silhouette (alpha > 0.5; the Octane table above used Octane's, so its
numbers differ a little):

| row | Iray 1 / 2 / 3 / 4 | Iray 1/2, 3/4, 4/2 | Faithful 1/2, 3/4, 4/2 | Enhanced 1/2, 3/4, 4/2 |
|---|---|---|---|---|
| G thin | 10.57 / 8.09 / 6.61 / 4.00 | 1.31, 1.65, 0.50 | 1.35, 1.77, 0.50 | 1.31, 1.56, 0.56 |
| H Pixie widths | 10.15 / 7.77 / 6.42 / 3.95 | 1.31, 1.62, 0.51 | 1.34, 1.73, 0.51 | 1.31, 1.56, 0.57 |
| I Pixie widths, cap | 10.49 / 8.56 / 7.29 / 5.56 | 1.23, 1.31, 0.65 | 1.25, 1.36, 0.66 | 1.23, 1.32, 0.67 |
| J thin, cap | 10.93 / 8.79 / 7.49 / 5.77 | 1.24, 1.30, 0.66 | 1.27, 1.35, 0.66 | 1.25, 1.31, 0.67 |

Faithful / Iray per cell 1.05 - 1.09 (col 2: 1.06 everywhere).

- **The material rule holds on fibres**: Iray's whole / opaque base (4/2)
  is 0.50 / 0.51 / 0.65 / 0.66, Faithful's 0.50 / 0.51 / 0.66 / 0.66. Iray
  loses Hair01's translucency share and its reflection-less refraction on
  a fibre exactly as the export drops them (`Tuning::opaque_fibres`, #79).
- **Width and the cap move both alike**: G -> H base 0.96 x in both, the
  cap lowers the opaque share 1.31 -> 1.23 in Iray, 1.34 -> 1.25 in
  Faithful, the whole one 1.62 -> 1.31 / 1.73 -> 1.36.
- **The lobe share matches**: Faithful 1.02 - 1.03 x Iray's (opaque),
  1.04 - 1.07 x (whole); Enhanced 0.94 - 1.01 x. The opaque rows repeat
  -LobeArc (the control).

So nothing of the arcs-to-Pixie list - width, density, a cap, Hair01's
whole material - reproduces Pixie-T1's base 0.70 - 0.75 x or its side
share 1.36 x. Like #215's coat, the gap is only in Pixie's own scene.

## Pixie's scene taken apart

Two facts from the scene itself (the `.mi` of Pixie-T1-coat, #215):
Pixie Cut has four materials - Hair01 (67 % of the fibre data in the
`.mi`), Hair02 (26 %), Hairline01 / 02 (6 %) - all with Hair01's
values, except that Hair02 has no refraction (it is the opaque material
of the pile's columns 1 - 2); and the hair cap under the strands is a
cutout-mapped Uber with Hair01's diffuse colour and a thin-walled
refraction 0.59 tinted by its glossy colour (= the diffuse, 0.076): about
as dark as the export writes it (albedo 0.031, transmission 0.045).

New variants in `make-hair-set-scenes.py` (all Pixie, fibres):
Calib-Hair-Pixie-T1-nofig(-nogloss) - every mesh but the strands hidden
(figure, eyes, brows, hair cap, floor: the hair over the bare dome, as the
arcs), Calib-Hair-Pixie-T1-nocap-nogloss - the hair cap alone hidden.

Octane (Final, 400 samples, denoised), patch means (hair-set-compare.py's
boxes) against Pixie-T1 / -T1-nogloss:

| patch | Faithful share T1 / -nofig | -nofig-nogloss / -nogloss | -nocap-nogloss / -nogloss |
|---|---|---|---|
| crown | 1.42 / 1.43 | 1.00 | 1.00 |
| fringe | 1.36 / 1.27 | 1.11 | 1.00 |
| side | 1.67 / 1.69 | 0.96 | 1.08 |

(Enhanced the same within 0.02.) In Octane the patches are the hair alone:
what lies under it moves the base by at most 11 % and the side share not
at all. So if Iray's -nofig-nogloss lands near its -nogloss too, figure
and cap are out and the strands' own shape over the head is what is left;
if Iray's base drops to Octane's (0.7 x of now), Iray's patches see the
head through the strands where Octane's don't - a coverage difference
(width / opacity of a fibre in the pile), to be measured from Iray's
alpha on -nofig against our dump's strands (#215's coverage check).

Iray: the server's GPU failed on -T1-nofig-nogloss after ~200 of 2000
iterations ("Kernel [1] (SSS & RT) failed", error -4; #217's kind), the
submit stopped the server queue; the three scenes are parked as an `##
Iray batch` in #221 until the Iray Server is restarted.

## Figure and cap out (resume 2)

Iray references of -T1-nofig, -T1-nofig-nogloss and -T1-nocap-nogloss
(server GPU, 2000 samples, denoised; b9c5c1b). The Octane side was
rendered before #161 / #223, which touch ribbons only (fibres return
before the cutout pin and the fold), so it stands.

Patch means (hair-set-compare.py's boxes), Octane / Iray:

| variant | crown F / E | fringe F / E | side F / E |
|---|---|---|---|
| -T1-nogloss | 0.73 / 0.77 | 0.69 / 0.71 | 0.75 / 0.78 |
| -T1-nofig-nogloss | 0.73 / 0.77 | 0.73 / 0.75 | 0.77 / 0.80 |
| -T1-nocap-nogloss | 0.72 / 0.76 | 0.69 / 0.71 | 0.65 / 0.67 |
| -T1 | 0.85 / 0.89 | 0.74 / 0.76 | 1.03 / 1.02 |
| -T1-nofig | 0.86 / 0.89 | 0.78 / 0.80 | 1.08 / 1.07 |

Within each renderer:

| ratio | crown I / F | fringe I / F | side I / F |
|---|---|---|---|
| -nofig-nogloss / -nogloss | 1.00 / 1.00 | 1.04 / 1.11 | 0.94 / 0.96 |
| -nocap-nogloss / -nogloss | 1.02 / 1.00 | 1.00 / 1.00 | 1.26 / 1.08 |
| lobe share, with head (T1 / -nogloss) | 1.23 / 1.42 | 1.26 / 1.36 | 1.22 / 1.67 |
| lobe share, strands alone (-nofig / -nofig-nogloss) | 1.22 / 1.43 | 1.19 / 1.27 | 1.20 / 1.69 |

- **Figure, cap and floor are out** (outcome 1 of the batch): Iray's base
  does not move without them either, and Octane's stays 0.73 - 0.77 x.
  The lobe share without the head is Iray 1.20 - 1.22, Octane 1.27 -
  1.69, as with it. Only the side patch over the bare head (-nocap) moves
  more in Iray (1.26 vs 1.08): skin shows through there; it is not the
  base gap.
- **Not coverage**: Iray's alpha is 1.00 in every patch of -nofig (0.99
  on the fringe), so Iray's patches see no more of what lies behind than
  Octane's. Over the whole hair the ratio is 0.70 wherever Iray's alpha
  is 1 and rises only toward the silhouette (0.76 at alpha 0.7 - 0.98).
- Pixie's four materials sit on the strands as in the `.mi`: Hair01 65 %
  of the polyline data, Hair02 26 %, Hairline 6 + 3 %; the export writes
  Hair02 as the opaque base (albedo 0.051), Hair01 as the Mix over the
  same base. Not a material assignment.

### What is left: how the render hairs lie

The arcs cross every orientation to the camera (|cos(tangent, view)|
0.1 - 0.97) and every patch of Pixie lies inside that (median 0.21 -
0.33), and Pixie's crown (strands along x) and side (along y) have the
same 0.73, so the orientation in the image is out too. What differs is
the packing. A 0.6 cm cube of the dump, strand surfaces sampled every
0.02 mm (radius 0.025 -> 0.01 mm), the nearest other strand over the sum
of radii:

| geometry | strands in the cube | median | touching (< 1) | within 2 |
|---|---|---|---|---|
| Pixie-T1 (two spots) | 557 / 384 | 4.6 / 4.0 | 3.8 / 3.6 % | 13 / 17 % |
| -Pile row H | 96 | > 9 | 0 | 3 % |
| Pixie -pr0 | 142 / 99 | > 9 | 0.5 / 0.7 % | 2 / 3 % |
| -DensePile H (0.3 cm deep) | 312 | 3.4 | 5 % | 21 % |
| -DensePile I (0.1 cm deep) | 297 | 1.8 | 10 % | 54 % |

The pile note's "same coverage" was an average over the hair layer;
locally Pixie's render hairs lie about 4 x denser than -Pile's arcs.

New Pixie variants (`make-hair-set-scenes.py`, strands alone, no gloss):
- -T1-nofig-nogloss-noclump (`PR Clumpiness 1` 0): its dump is
  identical to -nofig-nogloss, vertex for vertex - clumpiness does not
  move Pixie's render hairs. Kept, not for Iray.
- -T1-nofig-nogloss-tipsep (`PR Hair Tip Separation` 0.2): points move
  by up to 0.5 mm, the packing stays (table: the same). Kept, not for
  Iray.
- -T1-nofig-nogloss-pr0 (`PR Hairs (Per Guide)` 0): the guides only,
  29 404 / 11 458 / 1 684 / 964 strands (a quarter), the packing of
  -Pile. Octane 2.31 / 3.23 / 1.75 x its -nofig-nogloss (the dome shows
  through a quarter of the strands); the Iray side says whether
  Octane / Iray goes back to the arcs' 1.05 without the packing.

New `Calib-Strand-DensePile-HDRI` (`make-strand-scenes.py`): -Pile's
row H (3000 arcs at Pixie's widths) and columns, the layer 1.5 / 0.3 /
0.1 cm deep (rows G / H / I; J = H over the cap). Octane (cells x 1000
over its own silhouette, share 1/2, 3/4, 4/2):

| row | Faithful 1 / 2 / 3 / 4 | Faithful shares | Enhanced shares |
|---|---|---|---|
| G 1.5 cm | 10.31 / 7.62 / 6.82 / 3.87 | 1.35, 1.76, 0.51 | 1.32, 1.55, 0.57 |
| H 0.3 cm | 9.19 / 7.20 / 6.34 / 3.80 | 1.28, 1.67, 0.53 | 1.26, 1.52, 0.58 |
| I 0.1 cm | 9.75 / 7.79 / 6.61 / 4.09 | 1.25, 1.62, 0.53 | 1.23, 1.53, 0.55 |
| J 0.3 cm + cap | 10.52 / 8.75 / 7.82 / 6.29 | 1.20, 1.24, 0.72 | 1.19, 1.23, 0.73 |

In Octane the packing moves the base by at most 5 % and the lobe share
by -0.1. If Iray's base rises ~1.35 x from G to H, the packing is Pixie's
gap and the question becomes what Iray does with fibres that nearly
touch (docs say only: regular BSDF on fibres single sided, opaque); if it
stays, the packing is out as well and what is left is per strand (the
18-point render hairs' kinks under Iray's cubic B-spline). No constant
either way. `Calib-Strand-CoatDensePile-HDRI` carries #215's coat
columns on the same rows.

`Calib-Strand-CoatDensePile-HDRI` (#215; columns Pixie whole, + coat,
opaque, opaque + coat), Octane coat share 2/1, 4/3: Faithful G 2.31,
1.60 / H 2.15, 1.53 / I 2.07, 1.46 / J 1.51, 1.33; Enhanced 2.21, 1.63 /
2.07, 1.54 / 2.04, 1.48 / 1.51, 1.33. Octane's share falls a little with
the packing (2.31 -> 2.15 whole); on -CoatPile Iray's was 2.32 / 2.11 /
1.65 / 1.58 and Faithful's 1.02 - 1.04 x that. Pixie-T1-coat: Faithful
1.84 / 1.73 / 2.38 against Iray's 1.32 / 1.39 / 1.18 (crown / fringe /
side). If Iray's share drops toward Pixie's with the packing while
Octane's barely moves, the coat gap and the base gap are the same
packing effect.

Process: count the hair locally (a small cube of the dump), not as an
average over its layer - the average hid a 4 x denser packing. And check
a generator parameter's effect in the dump before rendering it:
clumpiness did nothing to Pixie's render hairs.

## The packing is out too (resume 3)

Iray references of -DensePile, -CoatDensePile and -T1-nofig-nogloss-pr0
(server GPU, 2000 samples, denoised; 378b30f). The Octane side stands (no
export change since 11f8a80; #161 / #223 touch ribbons only).

`strand-cells.py`, cells x 1000 over Iray's silhouette (share 1/2, 3/4,
base 4/2):

| row | Iray col 2 | Faithful col 2 | F / I col 2 | Iray shares | Faithful shares |
|---|---|---|---|---|---|
| G 1.5 cm | 7.87 | 8.30 | 1.05 | 1.31, 1.67, 0.50 | 1.35, 1.77, 0.50 |
| H 0.3 cm (Pixie's packing) | 7.35 | 7.75 | 1.05 | 1.23, 1.59, 0.52 | 1.27, 1.67, 0.52 |
| I 0.1 cm | 7.98 | 8.41 | 1.05 | 1.22, 1.55, 0.51 | 1.25, 1.64, 0.52 |
| J 0.3 cm + cap | 8.93 | 9.41 | 1.05 | 1.18, 1.22, 0.71 | 1.19, 1.25, 0.71 |

- Iray's base does **not** rise from G to H (col 2: 0.93 x in both
  renderers); Faithful / Iray is 1.05 in every row and column, the lobe
  share moves alike (-0.08 in both). Outcome (2) of the batch: Pixie's
  packing on arcs reproduces nothing of the 0.73.
- -CoatDensePile (#215), coat share 2/1, 4/3: Iray 2.24, 1.53 / 2.13,
  1.43 / 2.05, 1.38 / 1.49, 1.27; Faithful 2.32, 1.60 / 2.19, 1.50 /
  2.13, 1.45 / 1.53, 1.33 (1.03 - 1.05 x). The coat share falls with the
  packing alike in both: the packing is out for the coat as well.
- -T1-nofig-nogloss-pr0 (guides only, the arcs' packing), Pixie's boxes,
  Octane / Iray: crown 0.74 / 0.77 (F / E), fringe 0.84 / 0.85, side
  0.75 / 0.78; on the pixels where Iray's alpha is 1: 0.64 / 0.62 / 0.72.
  pr0 / full is 2.3 x in both renderers (the dome through a quarter of the
  strands), the gap stays: it does not go with the strand count either.

### Geometry checked against Iray's own

- **Coverage**: the dump's -pr0 strands rasterised through the scene's
  camera at the export's widths (0.05 -> 0.02 mm, quadratic taper) predict
  Iray's alpha within 1 %: patches 0.957 / 0.917 / 0.931 vs Iray 0.963 /
  0.924 / 0.938, partial pixels binned 0.12 - 0.89 vs 0.14 - 0.90, alpha
  sum 90 242 vs 91 529. Iray renders the dump's strands at the dump's
  widths (DAZ's `.mi` of Pixie-T1-coat: radius 0.025 -> 0.010 mm per
  control point, the same quadratic taper). The camera optical depth of
  -pr0's patches is 3.5 - 4.8, of -DensePile's cells 2.5 - 3.5 (the same
  raster): optical depth is not what separates them.
- **Curve basis**: Iray draws a polyline as a uniform cubic B-spline with
  phantom end points (Iray manual 7.6.2); Pixie's points turn 3.3 deg
  (median), 13.5 (p90), 90 (p99), the B-spline lies up to 0.29 mm (p90)
  off them. Octane probe: the dumps resampled to that B-spline (2 points
  per span, repeated points dropped), exported and rendered: Pixie
  -nofig-nogloss 1.00 / 1.00 / 1.00 x the linear export, -DensePile
  1.00 - 1.01. Octane draws Iray's curve as it draws ours - the kinks
  are out without an Iray render.
- **Environment**: Pixie's scene draws the dome, the -HDRI scenes don't
  (a black backplate in Octane). Pixie -nofig-nogloss with -DensePile's
  black backplate: crown / side unchanged (0.73 / 0.77; the fringe drops
  to 0.61 only where the dome showed through). Dome mode (infinite),
  lighting resolution, tone mapper and render settings are identical in
  the two `.duf`s; the export's kernel pins are identical.
- **Material**: Pixie's Hair01 in DAZ's `.mi` has the same MDL arguments
  as -CoatPile's Pixie column (`rounded_corner_normal` bump, instance
  flags, shadow / visibility attributes included); the export writes the
  same Octane nodes for Pixie's Hair01 and -DensePile's column 4.
- The ratio is grey (R / G / B 0.74 / 0.73 / 0.75 on the crown), so no
  multiple scattering in the hair's colour either.

What is left between Pixie's render hairs and the arcs is their shape on
the head: 3 cm strands (vs 25 cm), bent to a ~3 cm radius along a closed
shell in every direction. New `Calib-Strand-Shell-HDRI`
(`make-strand-scenes.py`, SHELL): G 3 cm strands over a 4 cm shell in
random directions (900 cm of strand per cm2, -pr0's camera optical depth),
H the same shell as 12 cm half circles, I -DensePile's row H arcs cut into
3 cm pieces; columns Pixie without its lobe, whole / opaque.

Its Iray reference: server GPU, 2000 samples, 71 s. Faithful / Iray (cells x 1000, col 1
whole / col 2 opaque, share 2/1):

| row | Iray | Faithful | Enhanced | F / I |
|---|---|---|---|---|
| G 3 cm on the shell | 5.12 / 7.56, 1.48 | 5.44 / 8.00, 1.47 | 5.69 / 8.01, 1.41 | 1.06 / 1.06 |
| H 12 cm half circles | 4.78 / 7.05, 1.48 | 5.09 / 7.47, 1.47 | 5.39 / 7.47, 1.39 | 1.07 / 1.06 |
| I -DensePile H cut to 3 cm | 4.68 / 7.53, 1.61 | 4.98 / 7.97, 1.60 | 5.28 / 7.96, 1.51 | 1.06 / 1.06 |

Short strands, a bent shell in every direction, the arcs cut short: all
match like every pile (1.06). The strand shape is out.

### Pixie's own strands on a plain node

`make-pixie-poly-scene.py`: -pr0's dumped strands (43 510, world space)
as one polyline node - no figure, no dForce generator - on -DensePile's
column-4 material (Pixie without its lobe), through Pixie's camera; the
scene carries geometry of Pixie Hair (free DAZ content), so it stays in My
Library and only the generator is in git. Iray: server GPU, 2000 samples.

| scene | Octane / Iray (crown / fringe / side) |
|---|---|
| -pr0 (the generated hair) | 0.74 / 0.84 / 0.75 |
| -pr0-poly (one material, dome not drawn) | 1.10 / 1.11 / 1.09 |
| -pr0-poly-split (Hair02's strands on their own node, refraction off) | 1.10 / 1.10 / 1.09 |
| -pr0-poly-split-drawn (the dome drawn, as in -pr0) | 1.05 / 1.06 / 1.07 |

- Same strands, same alpha (Iray 91 433 vs 91 529), the same MDL
  arguments per strand (the split's `.mi`: H4r = Hair02's arguments,
  `full_refraction_reflection_enable` included; H4 = Hair01's): Octane /
  Iray goes from 0.74 to 1.05 - 1.10 as soon as the strands are a plain
  polyline node.
- Octane renders -pr0 and -pr0-poly-split-drawn alike (1.00 / 1.00 /
  1.00): the export of the generated hair is the polyline export.
- Iray renders the generated hair 1.41 x brighter (poly-split-drawn /
  -pr0: 0.71 / 0.79 / 0.71) than the identical strands on a plain node.
- Hair02 is not it (split / single material 1.06 - 1.11 in both
  renderers alike), nor the drawn dome (1.10 -> 1.06).

### What DAZ hands Iray for the generated hair

DAZ's `.mi` of -pr0 (the figure shown, else the export refuses the
hidden PBRSkin, see below) against -pr0-poly-split's:
- material arguments and instance attributes identical (shadow /
  visibility flags, matte, transform);
- the generated hair's `hair` objects carry **per-fibre texture
  coordinates** (`hair t 2`), the plain node's none;
- **8 control points per fibre** (+ 2 phantoms) where the dump and the
  plain node have 18: DAZ hands Iray a coarser curve, ~0.2 mm (median,
  per-strand max 0.5 mm) off the dump's points (the `.mi` reads the
  viewport). Octane fed those 8-point B-splines (dump rebuilt from the
  `.mi`, scratch export): 0.64 -> 0.66 on full-alpha pixels - not the
  1.41.

So the gap is in DAZ's render path of a dForce-generated hair into Iray:
something the viewport `.mi` doesn't show, or the per-fibre texture
coordinates through the Uber MDL, makes Iray light those fibres 1.4 x
more than the same fibres from a polyline. Octane has no such path - its
render of Pixie is the render of the strands. Nothing in the export
changed; any Faithful factor for "generated fibres" would be a constant
fitted to Pixie with no component behind it (the decompose rule).

Process: build the same strands outside DAZ's generator early - one
plain-node copy split the renderer path from the geometry in two
renders, after four generic stand-ins (piles, shells, cut arcs, a
B-spline probe) each matched.

Tooling: `daz-export-mi.dsa` refuses a scene whose *hidden* nodes carry
a shader brick (PBRSkin on -nofig's hidden figure: "bricks exported as
Uber", .mi removed) - workaround here: a scratch copy with every node
shown.

## The component: DAZ renders sides-1 hair as triangles (resume 4)

The user's answer (2026-11-22): find what DAZ sends Iray for the
generated hair before any rule. Docs first: the Uber reads no texture
coordinate without a map (`irayubermaterial.mdl`: only `texture_tangent_*`
through bumps; `hair: base.hair`, no hair BSDF), and Pixie's materials have
none, so the `.mi`'s per-fibre coordinates (`hair t 2`, MDL's
`texture_coordinate(1)` on fibres, Iray manual 8.1.4 / MDL spec 19.1) feed
nothing. Left: the transmission Iray discards on fibres (manual 8.1.4) and
the geometry.

**Decomposition** - Pixie's translucency (0.33) and refraction (0.28) off:
new `Calib-Hair-Pixie-T1-nofig-nogloss-pr0-opaque` (`make-hair-set-scenes.py`)
and its plain-node twin (`make-pixie-poly-scene.py --scene <it> --material 2
--draw-dome`, My Library only). Iray: server GPU, 2000 samples, 198 / 136 s.
Patch means x 1000, crown / fringe / side:

| scene | Iray | Faithful | F / I |
|---|---|---|---|
| -pr0 (generated, whole) | 11.20 / 11.53 / 8.78 | 8.33 / 9.66 / 6.62 | 0.74 / 0.84 / 0.75 |
| -pr0-poly-split-drawn (plain node, whole) | 7.95 / 9.13 / 6.20 | 8.33 / 9.64 / 6.61 | 1.05 / 1.06 / 1.07 |
| -pr0-opaque (generated) | 11.64 / 13.12 / 10.52 | 11.85 / 12.26 / 10.20 | 1.02 / 0.93 / 0.97 |
| -pr0-opaque-poly-drawn (plain node) | 11.25 / 11.58 / 9.60 | 11.80 / 12.28 / 10.17 | 1.05 / 1.06 / 1.06 |

- Opaque, generated / plain node in Iray: 1.03 / 1.13 / 1.10 (whole: 1.41 /
  1.26 / 1.42). The 1.41 rides on the transmission.
- Iray whole / opaque: generated 0.96 / 0.88 / 0.83, plain node 0.71 / 0.79
  / 0.65 (= Octane's 0.70 / 0.79 / 0.65 with `opaque_fibres`). On the
  generated hair Iray keeps nearly all a transmission returns in a pile.
- **The geometry**: the Iray Server's log, `Geometry import`: the generated
  hair "1 triangle object with 2 M triangles", the plain node "1 fiber
  object with 43510 fibers (766 k segments)". DAZ tessellates a Line
  Tessellation sides-1 hair into triangles at render (about 2 per segment:
  a flat strip) although its `.mi` writes `hair` fibres; on triangles Iray
  keeps the transmission. Coverage says the strip shows the strand's full
  width from every view (camera-facing, presumably). Recorded in
  `docs/daz/iray.md`.

So `Material::fibres` (`src/usd/mesh.rs`: sides < 2 -> fibres) is wrong for
sides 1: those strands are tessellated in Iray. What to treat them as is a
choice. Trial exports (an environment switch, not committed), Faithful +
Final, Octane / Iray:

| scene | now (fibres) | as prisms (transmission kept) | as ribbons (#161's rule: translucency into the diffuse) |
|---|---|---|---|
| -pr0 | 0.74 / 0.84 / 0.75 | 0.77 / 0.85 / 0.78 | 0.85 / 0.91 / 0.93 |
| Pixie-T1-nogloss | 0.73 / 0.69 / 0.75 | 0.77 / 0.71 / 0.79 | 1.00 / 0.90 / 1.07 |
| Pixie-T1 | 0.85 / 0.74 / 1.03 | 0.90 / 0.77 / 1.07 | 1.08 / 0.92 / 1.29 |
| Pixie-T1-coat | 1.20 / 0.93 / 2.08 | 1.24 / 0.95 / 2.12 | 1.37 / 1.06 / 2.31 |

Lobe share (T1 / -nogloss) Iray 1.23 / 1.26 / 1.22: now 1.42 / 1.36 / 1.67,
ribbons 1.32 / 1.28 / 1.47. Coat share (T1-coat / T1) Iray 1.32 / 1.39 /
1.18: now 1.85 / 1.74 / 2.37, ribbons 1.68 / 1.60 / 2.10. Enhanced is
unchanged by either (it keeps the transmission anyway).

- Octane's round curves lose most of a transmission (prisms: +4 %); the
  ribbon rule's "translucency stays in the diffuse" closes Pixie's base
  (-nogloss 1.00 / 0.90 / 1.07). The lobe and coat excess shrink but stay:
  Iray's flat strips catch the sun as coherent glints, Octane's round
  curves as a sheen (#215) - the curves stay round (#79's decision, no
  ribbon meshes).
- Not shipped: sides-1 strips as #161's ribbons is a judgment call (the
  ribbon rule was set on scalp-facing ribbons; these face the camera),
  parked as a question on #221.

Process: the Iray Server's log names the geometry of every job (`iray-server.py`
`server_log`, "Geometry import"); it settled in one read what three `.mi`
comparisons could not - the `.mi` export is no witness of the render's
tessellation.

## The rule (both, by Look)

The user's call (2026-11-22, #221 / #215): both, by Look. New
`Tuning::flat_strips_as_ribbons` with `Material::flat_strips` (Line
Tessellation sides 1, set beside `fibres`): Faithful + Final writes such
strands as #161's ribbons (fold with #223's short-strand exception, the
ribbons' cutout rule); Enhanced and the Preview rows keep the fibre
treatment. Sides 0 stays fibres (unverified at render).

Checks, Faithful + Final 400 samples, Octane / Iray crown / fringe / side
(hair-set-compare.py):

| scene | Faithful | Enhanced |
|---|---|---|
| Pixie-T1-nogloss | 1.00 / 0.90 / 1.07 | unchanged (export byte-identical) |
| Pixie-T1 | 1.08 / 0.92 / 1.29 | 0.89 / 0.76 / 1.02 (unchanged) |
| Pixie-T1-coat | 1.37 / 1.06 / 2.31 | unchanged |

The trial's numbers exactly. Lobe share Faithful 1.32 / 1.28 / 1.48 (Iray
1.23 / 1.26 / 1.22), coat share 1.68 / 1.60 / 2.10 (Iray 1.32 / 1.39 /
1.18). Ribbon scenes that must not move: Calib-Hair-Kevin, -Hudson and
Calib-Strand-LobeArc-T2-HDRI export byte-identical in both Looks before
and after (installed binary vs the new one), so no render was needed.

What stays: the side patch with gloss (1.29) and the coat (2.31) - Iray's
flat strips catch the sun as glints, Octane's round curves a sheen (#79:
curves stay round). Left for its own issue.
