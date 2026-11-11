# 2026-11-11 — The 2× strand width: what it was fitted on, what it hides (#79)

Follows [strand compare](2026-11-11-strand-compare.md). Question: why does
the export draw strands at 2× DAZ's Line Width (`LINE_WIDTH_SCALE`) when
Iray's fibres, ribbons and prisms are all 1×?

## 1. Not the strand count

Strands counted in DAZ's `.mi` (connected components of the hair's
`trilist`, vertices welded by position) against the dump the export reads:

| | Morgan Hair (sides 2, local) | dForce Corporate (Calib-Hair, sides 3) |
|---|---|---|
| strands, `.mi` / dump | 68 773 / 68 773 | 36 451 / 36 449 |
| total length, `.mi` / dump | ~9 050 / 8 840 m | 1 566 / 1 566 m |
| Line Start → End Width | 0.2 → 0.1 | 0.12 → 0.01 |
| width in the `.mi` (shortest triangle edge) | 0.10 – 0.20 mm | prism side 0.0086 – 0.104 mm = 0.866 × 0.01 – 0.12 |

Iray gets the dump's strands, one Line Width (mm) wide, tapered as the
channels say. (The Mohawk `.mi` with 1504 of 29 803 strands was that scene's
viewport density, not a rule.)

## 2. Where the 2× came from

`6b5dbc9` (2026-10-27, #16, [note](2026-10-27-hair-variants-eyes-thin-walls.md)):
three renders (k = 1, 2, 4) of Calib-Eye's close-up (dForce Corporate Hair,
Blended Dual Lobe Hair shader), "coverage" = the share of pixels more than
15 % off the background: Iray 0.43, k = 1 0.28, k = 2 0.47. The strands
are sub-pixel there, so that threshold counts a pixel by its contrast, not
by its geometry - width × the strand's colour - and the colour was off
(grey-blue at k = 1). The crown's brightness fell with the width too (2.20
/ 1.56 / 0.93 × Iray), so the width tuned the material; `HAIR_COLOUR_SHARE`
0.046 and `HAIR_TRANSMISSION_SHARE` 0.110 were then fitted on top of 2×.

## 3. Sparse patches: Octane's curve is as wide as it says

The patches of Calib-Strand (rows C / D, 3000 strands) are several strands
deep: Iray's alpha there is 0.93 - 0.99, saturated - which is why they
read 0.94 × Iray at twice the width. New scenes
(`make-strand-scenes.py`): Calib-Strand-Sparse / -T2 / -T3, rows E (300
horizontal strands, Line Width 0.1 / 0.2 / 0.4 / 0.8) and F (40 vertical
strands of 0.75 / 1.5 / 3, and 300 × 0.2), diffuse grey 0.5, one width
along the strand, over a 12 cm wide patch (0.15 px per 0.1 mm). Each width
doubles the one before, so the export's 2× on one patch is 1× on the next.
Iray Server 2000 samples (121 / 28 / 28 s); Octane enhanced-final, 256
samples (18 s).

Iray's alpha summed over the patch, over 1 − exp(−τ) (τ = strands × width /
12 cm, random overlap) - constant if the alpha is the 1× coverage:

| patch | E1 | E2 | E3 | E4 | F1 | F2 | F3 | F4 |
|---|---|---|---|---|---|---|---|---|
| τ at 1× | 0.25 | 0.5 | 1 | 2 | 0.25 | 0.5 | 1 | 0.5 |
| fibre, alpha / coverage (k px) | 22.2 | 22.4 | 23.1 | 23.6 | 16.8 | 17.7 | 17.5 | 16.5 |
| ribbon / fibre alpha | 0.99 | 0.99 | 0.99 | 0.99 | 0.97 | 1.00 | 1.00 | 0.98 |
| prism / fibre alpha | 0.87 | 0.88 | 0.91 | 0.94 | 0.86 | 0.90 | 0.90 | 0.88 |

So Iray's coverage follows 1× the width down to 0.15 px; ribbons (facing
the camera here) cover as fibres, prisms 0.87 × (a prism's projected width
is 0.75 - 0.87 of its circumscribed diameter).

Octane X saved no alpha (kernel `alphachannel` 1 through
`orbx-variants.py node=75=1`: the PNG stays RGB), so Octane is compared by
the patch's summed linear luminance (black void, the same sun; Iray's RGB
is premultiplied: L / alpha is constant across F3's edge pixels). Octane at
2× against Iray's fibres on the patch of twice the width (the same strand
width in both):

| width drawn | 0.2 mm (E) | 0.4 (E) | 0.8 (E) | 1.5 (F) | 3 (F) |
|---|---|---|---|---|---|
| Octane 2× / Iray 1× of the same width | 0.91 | 0.92 | 1.00 | 0.75 | 1.00 |

and on the same patch (Octane 2× over Iray 1×): E 1.40 / 1.25 / 1.13 /
1.16, F 1.47 / 1.60 / 1.40, F4 1.35 - the excess the 2× adds wherever the
hair is thin enough to see through. The 0.75 is two different random
patches of 40 strands a pixel wide (F1's strands against F2's); the rows of
300 are the measure. Ribbons are 1.07 - 1.17 × the fibres' light (E), prisms
0.87 - 0.94 × (E), 0.62 - 0.65 × (F).

**Finding.** Octane's curve covers what Iray's fibre of the same width
covers, sub-pixel included (within 10 %, Octane the lower). Geometry gives
no reason for 2×: it stands in for the hair *material* on thin hair (the
Hair material's low albedo shares and its transmission, fitted at 2×),
measured through a contrast threshold. On dense hair the width hardly
shows (Morgan at 1×, [11-06](2026-11-06-morgan-hair-strands.md): crown
2.98 → 2.78), on fringes, lashes and hairlines it draws 1.1 - 1.6 × Iray's
light.

## Left

- `LINE_WIDTH_SCALE` is still 0.002: 1× needs the Blended Dual Lobe Hair
  mapping (`HAIR_COLOUR_SHARE`, `HAIR_TRANSMISSION_SHARE`) taken apart
  first - both were fitted at 2× - and Calib-Hair / Calib-Eye re-checked.
- Ribbons or curves (the question on #79) is untouched by this.

## Process

- Count before theorising: the strand count took one script over two
  `.mi` files and closed the main suspect.
- A patch meant to show coverage must be thin (τ ≤ 1); rows C / D were
  built for inter-reflection and could not show the width.
- The scripts (`micount.py`, the patch sums) were session scratch, not kept.
