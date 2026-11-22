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
