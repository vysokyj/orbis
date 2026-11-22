# 2026-11-22 — Top coat on fibre hair (#215)

#203's compare: on Calib-Hair-Pixie-T1 (fibres) Iray's white top coat
(weight 1, reflectivity mode 0.5, roughness 0.2) adds 1.18 - 1.39 x in the
crown / fringe / side patches, Octane Faithful's 1.92 - 2.99 x. Goal:
split the coat's share into parts before touching anything (no factor on
the Pixie render). #216 (the dual lobe's share) kept apart. Faithful +
Final, 400 samples unless said; linear luminance.

## What MDL and Iray say

- Uber's Top Coat in mode 0 is `custom_curve_layer` (head-on 0.08 x
  Reflectivity = 0.04, grazing 1, exponent 5) of a glossy BSDF over
  everything below - the same layer the Glossy uses in PBR metallicity
  mode (`docs/daz/iray-uber-mdl.md`). The export writes Universal's
  `coating` (white, IOR 1.5, `coatingRoughness` 0.2); with a dual lobe
  the lobes collapse onto `specular` (spec, Dual Lobe).
- Iray programmer's manual 8.1.4: on fibres a regular BSDF is single
  sided and its transmission is discarded; nothing about the normal.
  Iray's thick fibre with a white custom-curve gloss over black renders
  as a 24-sided mesh tube (Calib-Strand-Dome A2 / B2 cells 37.85 / 37.92):
  Iray's fibres shade as round tubes.

## Octane side first (existing references)

| Calib-Strand-Dome (white gloss over black, fibres) | A2 thick | B2 tube | C3 dense (aniso) | D3 vertical |
|---|---|---|---|---|
| Octane / Iray, gloss on `specular` (as exported) | 1.06 | 1.12 | 0.92 | 0.86 |
| the same on `coating` | 1.06 | 1.12 | 0.96 | 1.03 |

Coating = specular on curves. Under the sun (Calib-Strand) both lose the
highlight at roughness 0.05 alike (C3 608 -> 12): thin curves follow the
roughness.

Pixie-T1-coat taken apart in Octane (patch means x 1000, crown / fringe /
side; Iray base 6.02 / 4.97 / 6.39, coat 7.93 / 6.90 / 7.51):

| Octane variant | crown | fringe | side |
|---|---|---|---|
| base | 3.88 | 3.00 | 4.58 |
| coat (export) | 8.30 | 5.75 | 13.70 |
| coat, coating 0 (the dual lobe collapsed, no coat) | 3.88 | 3.01 | 4.58 |
| coat on the base Universal only / on the refraction only | 7.10 / 5.01 | 5.08 / 3.62 | 11.50 / 6.64 |
| the coat alone (albedo, specular, transmission 0) | 4.29 | 2.73 | 8.92 |

The collapse costs nothing, the coat is additive (alone = what it adds),
split by the refraction Mix's 0.28. Not the layering. Also tried, no
change to the coat's patches: `coatingRoughness` 0.1 / 0.05 (4.36 / 2.71
/ 8.9), the polylines refined into Iray's cubic B-spline (ends
extrapolated, 3 samples per span: 4.45 / 2.74 / 9.19), 4000 samples
(4.36 / 2.74 / 9.00), Octane's raw pass instead of the denoised one (the
Iray references are CPU renders without the denoiser, #217: same means).
Ray epsilon 1e-7 (kernel 1e-6) moved base and coat a little alike, 1e-8
darkens everything (self-hits) - not it.

## The coat over the whole hair

Coat addition (coat - base) summed over the hair's box, and the share of
it in glints (pixels where the coat adds > 0.02):

| | hair box | top half | lower half |
|---|---|---|---|
| Iray T1 | 519 (glints 0.49) | 281 (0.66) | 238 (0.29) |
| Octane T1 (400 / 4000 samples) | 606 (0.05 - 0.11) | 281 - 286 | 323 - 325 |
| Iray T2 (ribbons) | 892 (0.50) | 549 | 343 |

The amount matches (top half 1.00, lower 1.36, whole 1.17); Iray puts
half of it into coherent sun glints at the crown, Octane spreads it as a
blue sky sheen over every strand. The patches sit in the sheen, so they
read 1.6 - 2.5 x (side 8 x).

## Calib-Strand-Coat-Dome / -T2-Dome (new, Iray 2000 samples)

The coat on straight strands under Calib-Aniso's dome (materials 27 -
30: coat over black, over grey 0.5, over dark 0.2, a coat-like glossy
layer over grey; bases bare = Calib-Strand-UberA-Dome columns 2 / 1).
Rendered first on the CPU (the server's GPU had crashed, #217; raw) and
again on the GPU (141 s): cells within 1 - 3 %. Octane / Iray, fibres
(G thick, H sparse, I dense, J vertical):

| | G | H | I | J |
|---|---|---|---|---|
| coat over black | 1.07 | 1.07 | 1.04 | 1.20 |
| coat over grey | 1.16 | 1.08 | 1.03 | 1.12 |
| coat over dark | 1.15 | 1.08 | 1.04 | 1.16 |
| coat-like gloss over grey | 1.15 | 1.08 | 1.02 | 1.11 |

Coat share over dark (coat / bare): Iray 2.43 / 2.44 / 1.74 / 1.23,
Octane 2.60 / 2.66 / 1.87 / 1.33. On straight fibres the coat matches.
Iray's ribbons: the coat alone is a third of the fibres' (14.6 / 14.8 /
18.3 / 5.3 vs 43 / 56 / 40 / 15), over grey it darkens the ribbon (147
vs 170 bare) - on plain strands Iray's coat adds *less* on ribbons, the
opposite of Pixie.

## Calib-Strand-CoatArc-HDRI / -T2-HDRI (new)

The same materials (dark bare, dark + coat, coat over black, over grey)
on bent strands - half circles of 18 points, over the top toward / away
from the camera (rows G - I), facing it (J) - under DAZ's default dome
DTHDR-RuinsB with its sun, at the hair-set exposure (Pixie's lighting).
Octane / Iray fibres:

| | G | H | I | J |
|---|---|---|---|---|
| dark bare | 1.07 | 1.10 | 1.09 | 1.12 |
| dark + coat | 1.16 | 1.17 | 1.16 | 1.17 |
| coat over black | 1.15 | 1.16 | 1.17 | 1.20 |
| coat over grey | 1.15 | 1.16 | 1.15 | 1.15 |

Coat share over dark: Iray 2.63 / 2.98 / 2.48 / 1.80, Octane 2.85 /
3.17 / 2.65 / 1.87 (1.04 - 1.08 x). Ribbons again below fibres in Iray
(2.07 / 2.28 / 2.14 / 1.18). Bent strands under Pixie's HDRI and sun do
not reproduce the gap either; the coat looks alike in both images.

## Where it stands

Not the coat mapping, not its roughness, not the curve's shape, not the
HDRI or the layering: on plain strands, straight or bent, Octane's coat
on curves is 1.03 - 1.2 x Iray's fibres, its share 1.04 - 1.08 x. What
Pixie has and the synthetic strands don't: its own material (dark base
0.31 / 0.24 / 0.20, translucency 0.33 and refraction 0.28 that Iray drops
on fibres, the dual lobe) and its strands' width (0.05 -> 0.02 mm, a
quarter of THIN) in a deep pile over a head. Calib-Strand-PixieArc-HDRI /
-T2-HDRI put Pixie's Hair01 values on the arcs at Pixie's widths (whole,
+ coat, without translucency / refraction, that + coat); Octane rendered
(coat share 2.94 / 3.29 / 3.12 / 2.13 on fibres), Iray failed on the
server (internal rendering error at 763 / 2000 iterations, the GPU
crash of #217 again) - parked for Iray. Nothing shipped; no constant.

## Process

- Patch ratios on a hair hide where the light went: sum the addition
  over the hair before reading a share.
- On the server, strand scenes ran 110 - 141 s on the GPU and ~22 min on
  the CPU fallback; a job stuck at "Internal rendering error" is the GPU
  gone - cancel it rather than wait for the 30 min stall guard.

## Resume: Pixie's material on arcs and on a pile (after #216, #222)

The Octane side re-rendered with the current export (#216: Hair01's dual
lobe kept; #222: refraction transmission dropped on fibres in Faithful).
Iray references on the server's GPU, 2000 samples, denoised. Cells by
`strand-cells.py`'s boxes, masked by Iray alpha > 0.1 (row H of
-PixieArc is thinner than alpha 0.5 everywhere: the strands, 0.05 ->
0.02 mm, are sub-pixel). Columns: Pixie whole, + coat, opaque (no
translucency / refraction), opaque + coat.

Calib-Strand-PixieArc-HDRI (fibres), Faithful:

| row | Octane / Iray col 1 / 2 / 3 / 4 | coat share 2/1 Iray / Octane | 4/3 Iray / Octane |
|---|---|---|---|
| G | 1.12 / 1.17 / 1.15 / 1.19 | 2.42 / 2.54 | 1.64 / 1.69 |
| H | 1.10 / 1.13 / 1.11 / 1.16 | 1.73 / 1.78 | 1.52 / 1.58 |
| I | 1.09 / 1.13 / 1.11 / 1.16 | 1.89 / 1.96 | 1.53 / 1.60 |
| J | 1.08 / 1.10 / 1.07 / 1.12 | 1.38 / 1.41 | 1.26 / 1.32 |

Pixie's own material on single fibres at Pixie's widths takes the coat
alike in both (share 1.02 - 1.05 x); whole / opaque 0.69 - 0.79 in both.
How it lands also matches: in the coat cells the top 10 % of pixels hold
the same share of the light (Iray 0.16 - 0.34, Octane 0.16 - 0.34) and
p99 is 1.04 - 1.17 x. Not the material.

The ribbons (-PixieArc-T2) differ on the sparse rows: Iray's whole
ribbons are brighter than its opaque ones on G / H (1.59 / 1.32, the sun
through the translucency), Octane's darker (0.88 / 0.94), and Iray's coat
adds only 1.08 - 1.10 there (Octane 1.99 / 1.58); I / J agree within
10 %. Pixie-T2 itself is not off that way (share 1.63 / 1.54 / 2.06 vs
2.11 / 1.59 / 1.88); noted, not followed (a ribbon's facing on sparse
arcs).

New Calib-Strand-CoatPile-HDRI: -Pile's rows (G THIN 750, H Pixie's
widths 3000 at Pixie's density, I = H over the cap, J = G over the cap)
with -PixieArc's columns. Iray 121 s on the GPU; cells at alpha > 0.5
(0.1 moves them < 2 %).

| row | Octane / Iray col 1 / 2 / 3 / 4 | coat share 2/1 Iray / Faithful / Enhanced |
|---|---|---|
| G thin | 1.10 / 1.15 / 1.12 / 1.17 | 2.32 / 2.42 / 2.31 |
| H Pixie widths | 1.10 / 1.13 / 1.12 / 1.17 | 2.11 / 2.18 / 2.12 |
| I + cap | 1.09 / 1.11 / 1.10 / 1.15 | 1.65 / 1.69 / 1.69 |
| J thin + cap | 1.08 / 1.13 / 1.10 / 1.15 | 1.58 / 1.64 / 1.63 |

Width does nothing in either; the cap lowers the share alike (Iray 2.11
-> 1.65, Octane 2.18 -> 1.69). The pile does not reproduce the gap.

## What DAZ hands Iray for Pixie

- `.mi` of Calib-Hair-Pixie-T1-coat against Calib-Strand-CoatPile-HDRI
  (local, store content stays out of git): Hair01's MDL arguments are
  identical to the pile's (top coat roughness 0.04, the same diffuse,
  translucency, refraction, dual lobe; only the bump node's name
  differs); dome, sun, intensity, orientation and tone mapping identical.
- Strand count: the `.mi` holds 29 404 Hair01 fibres of 10 points, our
  dump 117 616 of 18 - exactly 4 x. Pixie's hair generator adds `PR
  Hairs (Per Guide)` 3 at render time; the `.mi` export has only the
  simulated guides (as it reads the Viewport tessellation sides). Which
  one Iray renders: the Pixie-T1-coat reference's alpha against our
  dump's strands rasterized through the camera (width x length per
  pixel, coverage 1 - e^-c): binned by predicted coverage 0.1 - 0.95,
  Iray's alpha follows all strands (0.19 / 0.33 / 0.54 / 0.74 / 0.91
  against 0.15 / 0.30 / 0.50 / 0.70 / 0.89), not a quarter (0.51 - 1.0).
  Iray renders the render hairs, as the dump has them; the export's
  widths (0.05 mm, tapering) match the `.mi` radii.

## Where it stands (2)

Material (the `.mi` proves it identical), widths, density, depth, a cap,
the dome, sun and tone mapping, the strand count and the glint
concentration all agree between the synthetic scenes and Pixie, and on
all of them Octane's coat share is 1.02 - 1.08 x Iray's. Pixie's gap
(after #216: 1.84 / 1.73 / 2.38 vs 1.32 / 1.39 / 1.18) is only in
Pixie's scene: its strands' shape (clumping, frizz, scraggle over a
head), the figure and hair cap under them, the floor. Taking that apart
needs Iray renders of Pixie variants (store content: the user's call).
No constant; nothing shipped in the export.
