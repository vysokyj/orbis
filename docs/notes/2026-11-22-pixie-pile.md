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
