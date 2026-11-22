# 2026-11-22 — generic figure scenes against Iray (#203)

Resume of #203: the ten Iray references of the 2026-11-19 scenes are in
git (aa3ad44, 2462899; headless submit on the home Iray Server, 1000
samples for the figure scenes, 2000 for the hair, ~38 min per hair scene;
Calib-Fit-G9 failed once with "Internal rendering error" and rendered
after a server restart). Octane: today's export (after #202's fibre
cutout and the resin / arch-glass work #205 - #214), both Looks final,
400 samples, denoised, re-exported from the cached dumps.
Calib-Hair-Pixie-T2-nogloss had no cached dump (dumped headless, 44 s).
Calib-Hair-Pixie (sides 3) and -nogloss were rendered again beside them.
Calib-Big-G9 stays an export measurement (#50), not rendered.

## Fit (Calib-Fit-G9) - the fit holds on Genesis 9

Edge alignment: gradient-magnitude images, the best integer shift by
normalised cross-correlation within ±12 px, per box (scratch script, not
kept). Every box at shift 0 in both Looks:

| box | Enhanced ncc | Faithful ncc |
|---|---|---|
| shorts' hem | 0.99 | 0.99 |
| thighs | 0.99 | - |
| left / right foot | 0.99 / 0.99 | 0.99 / 0.99 |
| hand, legs | 1.00 / 0.99 | - |

No stale fit (#51's double `baseResolution()` is not back). Frame
luminance 1.04 (report.md).

## Displacement (Calib-Disp-G9, -off, -Skin) - both renderers displace alike

Displacement alone = the scene minus `-off`, per renderer (linear
luminance, Enhanced):

| scene | pixels changed > 0.01 (Iray / Octane / both) | corr. of the two difference maps | mean abs. difference (Iray / Octane) |
|---|---|---|---|
| Calib-Disp-G9 (clothes) | 8072 / 8406 / 7748 | 0.98 | 0.0010 / 0.0013 |
| Calib-Disp-G9-Skin (PBRSkin) | 40208 / 42413 / 37441 | 0.99 | 0.0029 / 0.0032 |

Edge shifts 0 on the hem, shirt, arm and legs in all three scenes (ncc
0.96 - 1.00; -Skin hem 0.89, legs 0.78, below). Patch ratios Octane /
Iray: shirt 0.93 - 0.95, skin leg 1.05 - 1.08, arm 1.04, the same with
and without displacement, so they are the materials', not displacement's.

- **Skin displaces in Iray**: up to 5 mm on the PBRSkin surfaces pushes
  the legs through the shorts, in Iray and in Octane alike. The export
  keeps displacement on skin (the question in the batch's Resume).
- **The one difference, the -Skin seams**: the ridge map is a sine in v,
  so the two sides of a UV seam get different heights. Iray opens the
  seam (a sharp dark crack along the hip seam and down the thigh), Octane
  closes it (a soft step, no gap) - the lower ncc on -Skin's hem and legs.
  Octane's is the plausible one (a real skin map matches across seams);
  left as it is (Octane over Iray).
- The ridges themselves hardly show on the dark shirt at this exposure;
  the scene measures silhouettes and seams more than relief.

## Hair: Pixie as prisms (T3), ribbons (T2), fibres (T1)

`hair-set-compare.py Pixie Pixie-T2 Pixie-T1` (now with the Pixie-T*
boxes = Pixie's, same camera, and a -coat column). Luminance, Octane /
Iray, crown / fringe / side:

| shape | Iray lin (crown) | Faithful | Enhanced | F -nogloss | Iray gloss share | Octane F gloss share |
|---|---|---|---|---|---|---|
| prisms (T3) | 0.004 | 0.98 / 1.04 / 0.76 | 0.98 / 1.04 / 0.75 | 1.14 / 1.21 / 1.04 | 1.24 / 1.28 / 1.55 | 1.06 / 1.10 / 1.14 |
| ribbons (T2) | 0.012 | 0.48 / 0.89 / 0.85 | 0.34 / 0.63 / 0.61 | 0.62 / 1.08 / 1.13 | 1.35 / 1.29 / 1.45 | 1.04 / 1.06 / 1.10 |
| fibres (T1) | 0.006 | 0.64 / 0.60 / 0.72 | 0.67 / 0.62 / 0.73 | 0.75 / 0.70 / 0.76 | 1.23 / 1.26 / 1.22 | 1.06 / 1.10 / 1.14 |

Top coat (white, weight 1, reflectivity 0.5, roughness 0.2):

| shape | F -coat / Iray -coat | Iray coat share (coat / base) | Octane F coat share |
|---|---|---|---|
| ribbons (T2) | 0.40 / 0.92 / 1.08 | 2.11 / 1.59 / 1.88 | 1.78 / 1.64 / 2.39 |
| fibres (T1) | 1.05 / 0.83 / 1.83 | 1.32 / 1.39 / 1.18 | 2.14 / 1.92 / 2.99 |

What the split says (for #161; its decided split, docs/tuning.md, is not
changed here):

- Prisms: unchanged from 2026-11-18 (0.77 - 1.04 then), so #202 and the
  resin / arch-glass work did not move strands.
- **Ribbons** (Iray draws the same hair 3 × brighter at the crown, the
  ribbons' slant to the sky): Faithful 0.48 - 0.89, Enhanced 0.34 - 0.63.
  Faithful's ribbon rule lifts the hair 1.4 × over Enhanced, and on the
  fringe / side it reaches the ribbons (0.85 - 0.89, base 1.08 - 1.13);
  the crown, where the ribbons face the dome, stays at 0.48 (base 0.62):
  a round curve cannot show a flat ribbon's slant. Same picture as
  Morgan's crown (#99).
- **Fibres**: Faithful 0.60 - 0.72, the base 0.70 - 0.76 - the fibre gap
  #161 found on Morgan (base ~0.55) repeats on free content, smaller.
  Both renderers draw fibres without a cutout since #202, so it is not
  the cutout.
- **The gloss share is short on every shape**: Iray's dual lobe adds 1.22
  - 1.55, Octane's 1.04 - 1.14, prisms, ribbons and fibres alike (B
  highest in Iray: the sky in the sheen). The 2026-11-18 note saw it on
  prisms only; it is not a tessellation effect (#216).
- **The coat on fibres**: Iray's white coat adds 1.18 - 1.39 on fibres but
  1.59 - 2.11 on ribbons; Octane's adds 1.92 - 2.99 on its round curves
  (Faithful T1). The first measurement of Iray's coat on fibres: it is
  much weaker there than on flat ribbons, and Octane's coat on curves
  overshoots it ~1.6 - 2.5 ×. On ribbons Octane's coat share is near
  Iray's (0.8 - 1.3 ×). Filed as #215 (decompose first: the coat
  alone on a plain fibre, Calib-Strand rows).

## Process

- `calib-variants.sh` re-renders every `.orbx` in its variant's folder;
  a scene whose export failed (no dump) is simply missing. One export +
  `octane-render.sh` on that file filled the gap (~40 s).
- Octane at 1280 × 720, 400 samples: ~40 - 60 s per figure scene, both
  Looks of twelve scenes in ~20 min.
