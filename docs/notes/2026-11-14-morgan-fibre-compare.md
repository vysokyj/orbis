# 2026-11-14 — Morgan Hair: ribbons against fibres in Iray (#99)

Resume of [2026-11-13](2026-11-13-morgan-ribbon-gloss.md) with its Iray
batch in (91b1388): Calib-Strand-UberM-Edge-T2 / -Dome (in git) and
Calib-Hair-Teens-fib / -nogloss-fib (local: store hair, numbers only
here), 2000 samples each. Octane side as rendered on 2026-11-13 (256
samples, denoised pass), no new render.

Patches of the [2026-11-06 note](2026-11-06-morgan-hair-strands.md),
linear means over pixels opaque in both Iray renders (R / G / B). The
method reads the 2026-11-13 table within 0.01 – 0.04 (Octane / Iray
ribbons, enhanced: crown 1.20 / 1.52 / 1.47, middle 1.52 / 1.81 / 1.78).

## Iray's own fibres against its ribbons

| Iray fibres / Iray ribbons | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| whole | 1.99 / 2.20 / 2.33 | 1.35 / 1.57 / 1.68 | 2.23 / 3.19 / 3.46 | 1.71 / 1.87 / 1.89 | 1.26 / 1.29 / 1.28 |
| no gloss | 2.00 / 2.09 / 2.12 | 1.36 / 1.38 / 1.38 | 1.83 / 1.90 / 1.88 | 1.33 / 1.35 / 1.35 | 1.25 / 1.28 / 1.27 |

The face (no hair) reads 1.03 – 1.14: the same scene and exposure. Only
the strands' shape changed, and Iray's hair is 1.3 – 3.5 × brighter as
round fibres. As fibres Iray shows the even sheen across the middle that
Octane's curves have, not the ribbons' streaks with darker brown between.

Gloss component, G (base − no gloss):

| | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| Iray ribbons | −0.033 | +0.308 | +0.057 | −0.015 | −0.013 |
| Iray fibres | −0.061 | +0.524 | +0.388 | +0.073 | −0.016 |
| Octane enhanced (curves) | −0.015 | +0.274 | +0.213 | +0.060 | −0.011 |
| Octane faithful (curves) | −0.022 | +0.264 | +0.203 | +0.050 | −0.013 |

On fibres Iray's gloss lands on the middle as Octane's does (+0.39
against the ribbons' +0.06): #99's "3 – 5 × on the middle" was the
ribbons. Relative to each renderer's own no-gloss base (gloss / base, G):

| | crown | hilite | middle | lengths |
|---|---|---|---|---|
| Iray fibres | −0.30 | 1.67 | 1.28 | 0.28 |
| Octane enhanced | −0.13 | 1.53 | 1.19 | 0.38 |

Within 10 % on the hilite and middle; the crown darkens less, the lengths
gain a little more (the 2026-11-13 crown / lengths pattern, smaller). The
gloss lobe is not too bright.

## Octane against the fibres

| Octane / Iray fibres | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| enhanced-final | 0.60 / 0.69 / 0.63 | 0.70 / 0.54 / 0.48 | 0.68 / 0.57 / 0.51 | 0.60 / 0.65 / 0.62 | 0.50 / 0.70 / 0.46 |
| enhanced, no gloss | 0.51 / 0.56 / 0.53 | 0.52 / 0.57 / 0.57 | 0.53 / 0.59 / 0.59 | 0.54 / 0.60 / 0.59 | 0.58 / 0.70 / 0.58 |
| faithful-final | 0.42 / 0.51 / 0.48 | 0.63 / 0.50 / 0.44 | 0.60 / 0.51 / 0.46 | 0.49 / 0.55 / 0.53 | 0.33 / 0.48 / 0.36 |
| faithful, no gloss | 0.40 / 0.46 / 0.45 | 0.42 / 0.49 / 0.50 | 0.43 / 0.50 / 0.51 | 0.44 / 0.51 / 0.52 | 0.45 / 0.55 / 0.50 |

Not the resume's expected outcomes (within 15 %, or Iray's fibres below
Octane): Iray's fibres are far *above*, and evenly so - the no-gloss base
is 0.51 – 0.60 everywhere (Faithful lower, it drops Uber translucency on
fibres, #79). The gloss's share agrees, so the gap is the base under it.
The same material on synthetic fibre patches matched (#79,
Calib-Strand-UberM fibres 0.76 – 1.14 sun, 0.91 – 1.07 dome); what the
real hair adds (maps, top coat, many strands deep, the scene's dome,
tapered widths) is #161's to decompose.

## The ribbons turned in place (Calib-Strand-UberM-Edge-T2)

Cells found from the Iray alpha (rows G one thick strand, H sparse, I
dense, J sparse vertical; columns materials 5 whole / 14 no gloss / 15
no translucency / 16 no refraction), linear G means over each cell.

| Iray edge (65°) / face-on | 5 | 14 | 15 | 16 | gloss column (5 − 14) edge / face |
|---|---|---|---|---|---|
| sun G | 0.28 | 0.31 | 0.00 | 0.28 | 0.35 |
| sun H | 0.25 | 0.33 | 0.02 | 0.26 | 0.48 |
| sun I | 0.30 | 0.50 | 0.12 | 0.31 | 1.37 |
| sun J | 0.30 | 0.27 | 0.15 | 0.07 | 0.24 |
| dome G | 0.41 | 0.55 | 0.15 | 0.42 | 1.13 |
| dome H | 0.42 | 0.49 | 0.19 | 0.32 | 0.71 |
| dome I | 0.63 | 0.65 | 0.39 | 0.46 | 0.74 |
| dome J | 1.06 | 0.82 | 0.77 | 0.27 | 0.44 |

Turning the ribbons 65° about their strands darkens Iray's every column
to 0.25 – 0.5 under the sun, 0.4 – 1.1 under the dome - the no-gloss
column as much as the gloss (whose difference is negative face-on: Iray's
gloss takes more of the base than it sends to the camera there). So the
slant is not a gloss effect; it is the ribbons' projected width and their
facing to the light, which round curves don't have. Octane's renders of
the two scenes agree (edge / face 1.00 on columns 14 / 15, 1.05 – 1.10
on 5 / 16 - the node's turn reaches the gloss + translucency a little).
The no-translucency column under the sun goes black edge-on: a flat
ribbon turned away from a hard light, #79's "black under a hard light
from most views".

## Outcome

- #99 closes: the gloss on Morgan's strands is not too bright. Against
  the ribbon references the crown / middle excess is the ribbons' slant
  (Iray itself 1.3 – 3.5 × brighter as fibres, gloss and base alike), the
  price of #79's round curves; recorded in the spec (Uber anisotropy on
  strand hair). No lobe or share changed.
- #161: against Iray's fibres Octane's base is about 0.55 ×, with or
  without the gloss, on the real hair only. Which reference governs
  Morgan (ribbon hair) is for the user once that gap is understood.

## Process

- A decisive variant can land outside both outcomes a resume plans for
  (here the fibres were brighter than Octane, not close to or below it):
  read the component table first, then decide what it says.
- The 2026-11-06 patches on the fibre render need the opacity of both
  Iray renders as the mask (crown 81 %, lengths 83 %, tips 84 % hair).
- Strand-scene cells: row bands from the alpha's row maximum, then column
  bands inside each row, gaps under 15 px merged (no scipy here).
- No render this session; the compare took minutes from the cache.
