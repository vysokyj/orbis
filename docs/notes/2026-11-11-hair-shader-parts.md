# 2026-11-11 — Blended Dual Lobe Hair in its parts, on strands (#79)

Follows [strand width](2026-11-11-strand-width.md): the 2× width stands
in for the Hair material, so what is that material in Iray?

## 1. What Iray gets (MDL + `.mi`, no render)

`dual_lobe_hair.mdl` and Calib-Mi-Hair's `.mi`: no hair BSDF. Three parts
under `df::weighted_layer` (plain weights, no Fresnel):

1. a glossy in the highlight colour, weight hw = Highlight Weight;
2. a glossy in the hair colour, weight (1 − hw) × Glossy Layer Weight;
3. the base DAZ wires in: `df::diffuse_transmission_bsdf` tinted by the
   Transmission Color - a diffuse *transmission*, no diffuse reflection.

Calib-Hair (gloss 0.25, highlight 0.049): 71 % of what meets a strand goes
into the diffuse transmission. The export instead writes an Octane Hair
material: albedo 0.046 × hair + 0.110 × transmission colour, specular =
the glossy weight - two fitted shares on another model (and at 2× width).

## 2. Scenes

`make-strand-scenes.py`, rows G one thick strand (Line Width 20), H 300
sparse strands of 0.2, I 3000 of 0.15, J row H vertical; four materials
across. Calib-Strand-Hair (Calib-Hair's values: whole, hair gloss alone,
transmission alone, highlight alone at weight 1), -HairW (white
transmission, 0.5, white gloss, gloss 0.5 over white transmission), -Hair-Back
(the sun behind), -UberT (Iray Uber, Translucency Weight 1 white / 0.5 /
blond, glossy layer 0.25 over the blond), each also as prisms (-T3).
Iray Server 2000 samples (28 - 62 s each); Octane enhanced-final 256
samples (49 - 78 s), exported **at 1× width** (a temporary build with
`LINE_WIDTH_SCALE` 0.001, not committed). Measure: linear luminance summed
over each object (Iray's premultiplied RGB, Octane on black).

## 3. Iray's untessellated fibres drop the transmission

Calib-Strand-UberT (Uber, reliable): the three translucency-only columns
are exactly black on fibres (alpha present, RGB 0) in every row; the same
materials on prisms (-T3) are the brightest objects of the batch. So on
Iray's `hair` primitive a surface BSDF's transmission is lost; on
tessellated strands (ribbons, prisms: real surfaces) it passes. The hair
shader's fibres agree: transmission alone black; white gloss 0.5 over white
transmission = 0.52 / 0.50 / 0.45 / 0.48 × the white gloss alone (G / H / I
/ J), Calib-Hair's gloss alone (0.25 × the hair colour, luminance 0.59) =
115 against 0.25 × 0.59 × 743 = 110: the layers are plain weights.

## 4. A Universal on curves matches Iray's prisms, no constant

Calib-Strand-UberT in Octane (the export's Universal with a diffuse
transmission, round curves, 1×) over Iray's prisms:

| | white | 0.5 | blond | gloss 0.25 over blond |
|---|---|---|---|---|
| G thick | 1.12 | 1.16 | 1.14 | 1.18 |
| H sparse | 1.23 | 1.10 | 1.10 | 1.17 |
| I dense | 1.25 | 1.07 | 1.05 | 1.14 |
| J sparse vertical | 0.98 | 0.85 | 0.86 | 0.93 |

R / G / B within 0.06 of each other everywhere but the dense blond (0.98 /
1.09 / 1.04). A prism covers 0.87 × its fibre
([strand width](2026-11-11-strand-width.md)), which is most of the 1.1 -
1.2. Octane's curves do carry a diffuse transmission through the strand,
single strand to many deep.

## 5. Octane's Hair material

Probes on Calib-Strand-HairW's curves (1×), albedo / specular set per column:

| albedo, specular | G thick | H sparse | I dense | J vertical |
|---|---|---|---|---|
| 1 / 0.5 / 0.25 / 0.8 / 0.1, specular 0 | 0.1 - 0.4 | 1 - 3 | 4 - 16 | 0 |
| 0, 1 | 150 | 283 | 400 | 14 |
| 0.046, 1 (the export's white gloss) | 280 | 603 | 1074 | 31 |
| 0.5, 1 | 506 | 1625 | 6429 | 79 |

- **`specular` scales every lobe**, not the reflection alone: at specular
  0 the hair is black whatever its albedo. The export sets specular = the
  glossy weight (0.29 on Calib-Hair), so Iray's split between gloss and
  transmission became the hair's overall brightness in Octane.
- The lobes are a hair's: vertical strands under a sun in their plane
  return 5 % of the horizontal ones' light (J against H); Iray's parts on
  the same strands return about as much either way (J / H 0.6 - 0.9).

The export as it is (Hair material, at 1×) against Iray's fibres,
Calib-Hair's values: whole 0.66 / 1.06 / 1.74 / 0.08 (G / H / I / J),
gloss alone 0.68 / 0.77 / 0.89 / 0.05, highlight alone 0.36 / 0.56 / 0.81
/ 0.04, transmission alone black in both (Iray: fibres drop it; Octane:
specular 0).

## 6. The hair shader's Iray renders are not all usable

DAZ re-imports `::daz_3d::dual_lobe_hair` for each scene of a session and
now and then fails (#94's MDL Import Error 4004, here on DAZ's own module):
the material arrives empty and renders black with its alpha. Seen in the
`.mi` exports too (the second scene of a headless session: its first 11,
then 8 materials were a bare `replace_scattering(replace_surface_enable:
false)`), so it is DAZ, not the queue. Of this batch: -HairW-T3 and
-Hair-Back logged the error; -Hair-T3 and -Hair-T3-Back are black all over
**without** a logged error; -Hair and -HairW (fibres) are consistent with
the Uber set and with each other (section 3) but stay out of git with the
rest until each is rendered as the first hair scene of a DAZ session.
Kept: -UberT, -UberT-T3.

## Finding

On tessellated hair (what dForce Corporate and Morgan are) Iray's Blended
Dual Lobe Hair is two weighted glossies over a diffuse transmission, and
Octane's Universal on 1× curves reproduces a diffuse transmission through
strands within 0.85 - 1.25, colour included. The Hair material's fitted
shares, and the 2× width under them, are not needed to reach Iray. What
each Look should write is the user's call (Enhanced keeps Octane's
hair model and its backlit glow).

## Left

- Clean Iray references of the hair shader on prisms (one scene per DAZ
  session), then the gloss lobes on prisms against Octane.
- Enhanced: the Hair material's inputs from the parts above instead of
  the fitted shares (specular must not carry the glossy weight alone).
- `LINE_WIDTH_SCALE` 0.002 and the shares are unchanged.

## Process

- `.mi` before the batch caught the empty materials; a render that is
  black where physics says light is a broken material until shown
  otherwise - check each object's sum, not the log alone.
- A pair that differs in one thing (Uber against the brick shader, fibres
  against prisms) separated the renderer's rule from DAZ's failure.
