# 2026-11-11 Dual lobe under weighted mixing: compare (#121)

Resume of #121 with the Calib-DualLobeWeighted Iray reference (5000
samples, commit 4e8780f).

## Octane side re-rendered
The 15:54 render predated 900080c (Glossy Reflectivity left out under
weighted mixing) and 8efab8b. The export found no cached dump for the scene
as it is now (the scene changed after its dump), so `daz-dump.sh` again, then
enhanced-final at 400 samples; Calib-Mixing re-rendered too (its render
was from Oct 30).

## Compare (Octane / Iray, mean linear luminance over the upper disc)
Each sphere is compared with Iray at the same position (the #106 lesson:
each sphere has its own light, so W1 / W2 are no baseline for the values
of W3 - W8, only for the ratio band). Regions as in the #106 compare:
disc, centre (r < 0.45), rim (0.65 - 0.9), Iray's top 3 % ("hi").

| sphere | Iray | mean | centre | rim | hi | spread O / I |
|---|---|---|---|---|---|---|
| W1 gloss only | 0.229 | 1.05 | 1.07 | 1.03 | 1.10 | 3.0 / 2.7 |
| W2 dual lobe only | 0.059 | 1.12 | 1.12 | 1.10 | 0.98 | 2.2 / 2.7 |
| W3 both | 0.284 | 1.07 | 1.09 | 1.04 | 1.11 | 2.5 / 2.4 |
| W4 gloss 0.8 diffuse 0.2 | 0.666 | 1.03 | 1.03 | 1.03 | 1.00 | 1.5 / 1.5 |
| W5 diffuse 0.5 gloss 0.3 | 0.258 | 1.08 | 1.09 | 1.06 | 1.11 | 2.4 / 2.3 |
| W6 Top Coat | 0.333 | 1.11 | 1.13 | 1.09 | 1.12 | 2.4 / 2.4 |
| W7 lobe refl 1 | 0.574 | 1.10 | 1.10 | 1.09 | 1.00 | 1.6 / 1.8 |
| W8 dual lobe W 0.5 | 0.338 | 1.09 | 1.10 | 1.09 | 1.12 | 2.7 / 2.6 |

Colour: G / R within 0.015 of Iray's on W1, W3 - W8; W2 (a white lobe over
red) 0.281 vs 0.255, as Calib-DualLobe's lobes are.

Baselines: W1 sits with Calib-Mixing's weighted sphere X7 (1.09) and
Calib-Glossy's band; W2 with Calib-DualLobe (1.05 - 1.18) and is exactly
Calib-DualLobeGlossy's G2 (the same material at the same position: Iray
0.0594 vs 0.0593, Octane 1.12 both). W3 - W8 all inside 1.03 - 1.11, no
sphere above its position's single-layer ratios (Calib-DualLobe /
-Glossy at position 8: 1.18 / 1.24, W8 1.09), and the highlight shapes
match. Unlike #106's coating (1.22 - 1.48), the Mix stacks nothing, so
there is nothing for a per-sphere split to explain: no decomposition
scenes needed, nothing fitted.

W5 (weights summing to 0.8) is 1.08, with W3 (1.07) and W1 (1.05): the
export's "rest black" rule (d093523, #126) holds - before it W5 would
have rendered 1.25 x. That is #126's own Resume rule; its issue is left
to its own resume.

## Outcome
The mapping holds (spec, "A dual lobe under weighted mixing"): #121
closed.

## #126 resume: W5 against W3 and W1
The same Octane render (19:56, after 963e66d; no export change since) and
reference, measured again independently: mean linear luminance (gamma 2.2
decode) over each sphere's upper disc r < 0.9 R, sphere centres read off
the render (y 562, R 44 px), centre r < 0.45 R, rim 0.65 - 0.9 R.

| sphere | Iray | Octane / Iray | centre | rim | G / R Iray / Octane |
|---|---|---|---|---|---|
| W1 gloss only | 0.217 | 1.072 | 1.055 | 1.067 | 0.856 / 0.853 |
| W3 both (D 1, G 0.5) | 0.231 | 1.078 | 1.057 | 1.083 | 0.906 / 0.906 |
| W5 both (D 0.5, G 0.3) | 0.198 | 1.087 | 1.064 | 1.092 | 0.959 / 0.960 |

(The absolute values sit below the table above: a smaller disc, the
same ratios within 0.02.) W5 / W3: Iray 0.857, Octane 0.864.

#126's Resume asked: W5's ratio within W3's and W1's -> close; still
~1.25 x -> the rest is not black. W5 is 0.009 above W3, inside the
scene's 1.04 - 1.11 band and the 400-sample Octane noise, centre and rim
alike, colour equal. The export before d093523 rendered W5 1 / 0.78 x
brighter (the issue's own before / after Octane ratio, measured before
900080c / 8efab8b), far outside the band: that is what the rule removed. `normalized_mix` leaves
weights summing below 1 unscaled, as the MDL says; nothing fitted. #126
closed.
