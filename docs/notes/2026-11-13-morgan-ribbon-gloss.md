# 2026-11-13 — Morgan Hair's gloss: lobe or ribbon? (#99)

#99 (from #96): with the measured anisotropy Morgan Hair's strands
(Calib-Hair-Teens, local: store hair, numbers only here) render the crown
and middle 1.3 – 1.9 × Iray. #79 closed in between; re-measured first.

Patches of the [2026-11-06 note](2026-11-06-morgan-hair-strands.md)
(1280 × 720, linear patch means, R / G / B, Octane / Iray), 256 samples,
denoised pass (the raw beauty reads within 0.01); the 5000-sample Iray
references of 2026-10-29. The method reads #96's renders within 0.01 –
0.06 of its table.

## On the current code

| export | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| enhanced-final | 1.21 / 1.50 / 1.43 | 0.95 / 0.84 / 0.80 | 1.53 / 1.81 / 1.75 | 1.07 / 1.26 / 1.21 | 0.64 / 0.92 / 0.63 |
| faithful-final | 1.21 / 1.50 / 1.43 | 0.95 / 0.84 / 0.80 | 1.53 / 1.80 / 1.75 | 1.07 / 1.25 / 1.21 | 0.64 / 0.92 / 0.63 |
| -nocoat, enhanced | 0.94 / 1.10 / 1.39 | 0.97 / 0.92 / 0.81 | 1.35 / 1.52 / 1.70 | 0.94 / 1.05 / 1.19 | 0.63 / 0.87 / 0.66 |
| -nogloss, enhanced | 1.02 / 1.16 / 1.11 | 0.70 / 0.79 / 0.79 | 0.98 / 1.12 / 1.10 | 0.75 / 0.84 / 0.84 | 0.76 / 0.92 / 0.78 |

Not gone: #79 left Morgan's material alone (its ribbons are not fibres, so
the dropped translucency doesn't reach it), and both Looks write the same
three Morgan materials. As #79's section 10 measured.

## The parts, per renderer

Each part as base − variant (linear, crown / hilite / middle / lengths):

| | Iray | Octane | Octane / Iray |
|---|---|---|---|
| gloss, G | −0.028 / +0.309 / +0.056 / −0.014 | −0.012 / +0.271 / +0.210 / +0.053 | 0.42 / 0.88 / 3.73 / − |
| top coat, G | −0.058 / −0.147 / −0.141 / −0.102 | −0.040 / −0.176 / −0.153 / −0.075 | 0.69 / 1.20 / 1.08 / 0.74 |

(R and B alike: gloss on the middle 3.20 / 3.73 / 4.74.) The top coat
does what Iray's does within its noise. The gloss does not, and not by a
level: on the hilite streak, where Iray's gloss lands, Octane's adds the
same (1.26 / 0.88 / 0.82); on the middle it adds 3 – 5 × Iray's; on the
crown and lengths Iray's gloss *darkens* (it takes from the base by its
Fresnel and sends it away from the camera), Octane's darkens half as much
or brightens. Octane's hair is one even sheen; Iray's has streaks and
darker brown between them. A highlight that lands in other places, not a
lobe that is too strong.

## Probes on the lobe (project.ocs edited, enhanced, 256 samples)

| variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| as exported (gloss roughness 0.72, anisotropy 0.45) | 1.21 / 1.50 / 1.43 | 0.95 / 0.84 / 0.80 | 1.53 / 1.81 / 1.75 | 1.07 / 1.26 / 1.21 | 0.64 / 0.92 / 0.63 |
| gloss isotropic at 0.72 | 1.83 / 2.30 / 2.13 | 0.63 / 0.56 / 0.54 | 1.16 / 1.36 / 1.33 | 1.20 / 1.42 / 1.36 | 0.83 / 1.23 / 0.81 |
| gloss isotropic at the short axis (0.40) | 1.28 / 1.57 / 1.48 | 0.86 / 0.76 / 0.73 | 1.43 / 1.66 / 1.63 | 1.06 / 1.23 / 1.19 | 0.66 / 0.94 / 0.64 |
| top coat isotropic | 1.23 / 1.54 / 1.47 | 0.94 / 0.83 / 0.79 | 1.52 / 1.79 / 1.74 | 1.06 / 1.25 / 1.20 | 0.65 / 0.95 / 0.64 |

- Octane **does** apply a Universal's anisotropy on these curves: the
  isotropic lobe at the long axis moves every patch by up to 50 %. (#79's
  section 2 found a roughness-1 metal on dForce strands unchanged by
  rotation and sign; that case is not this one - a roughness-1 long axis
  can't widen.)
- No lobe width moves the middle without wrecking another patch (the
  isotropic short axis: hilite down to 0.73, middle still 1.43 – 1.66);
  #52's pin sweeps found the same. The lobe's width and orientation are
  #96's and #52's, measured on their own; the excess is not in them.

## The shape: ribbons seen at a slant

DAZ hands Morgan Hair to Iray as flat ribbons (Line Tessellation 2), each
ribbon's normal the hair object's +z made perpendicular to its strand
(make-strand-scenes.py, the .mi). In Calib-Hair-Teens the hair's +z
points to world (0.91, 0.34, 0.06) - the figure looks to the right of the
frame - and the camera looks along about (−0.19, −0.51, −0.84): the
ribbons face the camera at about 65° (|cos| ≈ 0.4). Octane's round curves
always turn a face to the camera and to the light. That is a reading of
every column above:

- Fresnel at a slant is high, so Iray's gloss and coat take more of the
  base where they don't send light to the camera (the crown: Iray's gloss
  −0.028, Octane's −0.012);
- a fixed normal puts the gloss highlight only where the ribbons' facing
  matches the light (Iray's streaks); a round curve has that facing
  everywhere (Octane's even sheen, the middle 3 – 5 ×);
- Calib-Strand-UberM-T2 (#79, the same material on straight ribbons that
  face the camera) has no such excess: the gloss's share is Octane's as
  Iray's there (whole 0.97 / 0.83 / 0.51 / 0.74 against no gloss 1.01 /
  0.82 / 0.55 / 0.77 under the sun).

The user decided (#79) that strands stay round curves. If the excess is
the ribbons' slant, it is that decision's price and no material constant
may stand in for it (a share fitted on one view fails the next, #79's
section 6). Two renders decide it; both need Iray, so #99 is parked with
a batch:

- **Calib-Strand-UberM-Edge-T2 / -Dome** (generated, in git): the
  -UberM-T2 scenes with every node turned 65° about its strands and the
  geometry turned back, so the strands stay where they were (the dumps'
  points agree to 0.001 cm) and only the ribbons' normal turns as in
  Calib-Hair-Teens. Octane's curves are the same as -UberM-T2's. If Iray's
  gloss column drops against -UberM-T2 while its no-gloss column stays,
  the slant is the cause, reproducible from git.
- **Calib-Hair-Teens-fib / -nogloss-fib** (local, `strand-tess.py`):
  Morgan Hair at Line Tessellation 1, which DAZ hands Iray as round `hair`
  fibres. If Iray's fibres land within ~15 % of Octane on the middle and
  crown, the excess is the ribbon shape and #99 closes as the curves'
  known limit (spec); if not, the gloss on round strands is off and the
  decomposition goes on from these renders (fibres take the Faithful +
  Final rule of #79 for Uber translucency).

## Left

See #99's `## Iray batch`.

## Process

- The parts as base − variant per renderer (a component table) named the
  gloss on the middle at once, where the ratio table had the crown, the
  middle and the lengths all "too bright".
- An Octane probe whose result contradicts a recorded "Octane ignores X"
  needs the earlier probe's input checked: it was a roughness-1 metal on
  another hair.
- Turning a ribbon without moving it: node rotation +θ about the strand,
  geometry pre-rotated −θ (DAZ: R_x(θ) takes y to z); check the dump's
  world points against the untouched scene.
- Timings: Calib-Hair-Teens 123 – 154 s at 256 samples in Octane (6
  renders back to back, no freeze); headless dumps 50 s each.
