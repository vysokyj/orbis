# 2026-11-14 Refraction under weighted mixing, compare (#154)

Resume of #154 with Calib-WeightedRefr and -Thin's Iray references in
(91b1388, 5000 samples, DAZ Studio Iray, Post Denoiser on@8): does the
weighted Mix with the refraction in its gloss part (46de92e: the glass
alone at Refraction Weight 1, a Mix of glass and metal below 1, the
diffuse part without refraction, Diffuse Weight 0 the gloss part alone)
hold?

Octane: enhanced-final, 400 samples, denoised pass, re-exported from the
cached dumps with HEAD (after #87, #114, #134, #149) - identical to the
renders from 46de92e to 0.01 per sphere, so none of the later export
changes touches these materials. Numbers: linear RGB (gamma 2.2) per
projected disc (`sphere_discs.py`): centre r < 0.4 R, rim 0.75 - 0.95 R,
disc < 0.95 R. Floor 1.03 in both scenes. ~40 s per render.

## Octane / Iray, disc luminance (centre / rim)

| Sphere | Channels | Closed | Thin-walled |
|--------|----------|--------|-------------|
| W1 | r 1, g 0.5, d 1 | 1.05 (1.06 / 1.05) | 1.04 (1.05 / 1.03) |
| W2 | r 1, g 1 | 1.04 (1.04 / 1.04) | 1.04 (1.06 / 1.03) |
| W3 | r 1, d 0 | 1.02 (1.02 / 1.02) | 1.03 (1.07 / 1.02) |
| W4 | r 0.5, g 0.5 | 1.04 (1.04 / 1.04) | 1.04 (1.05 / 1.04) |
| W5 | r 0.5, g 1 | 1.03 (1.04 / 1.02) | 1.03 (1.04 / 1.03) |
| W6 | r 0.5, d 0 | 1.03 (1.03 / 1.03) | 1.03 (1.05 / 1.03) |
| W7 | r 1, tinted gloss | 1.06 (1.05 / 1.08) | 1.05 (1.07 / 1.06) |
| W8 | r 1, unshared | 1.02 (1.06 / 1.02) | 1.05 (1.07 / 1.04) |

Every sphere sits within 0.99 - 1.05 of the floor's 1.03. Decomposed as
the issue's Resume asked: W3 (the glass alone) 1.02, W6 (glass + mirror
Mix alone) 1.03, so the gloss part is right on its own; W1 / W2 / W4 / W5
add the diffuse part at its share and stay 1.03 - 1.05, so the share
(2/3 diffuse + 1/3 glass at weights 1 / 0.5, half / half at 1 / 1) is
right too. No component is left to fit.

Colour: W1 / W2 red 1.10 / 1.09 against green 1.02 (the red diffuse a
touch strong beside the glass, R / G 1.08 × Iray's, 1.13 at W2's rim);
W7's blue 1.11 - 1.15 is at 0.001 - 0.002 linear (noise). The look is
Iray's: W1 / W2 reddish half-glass, W3 / W6 clear glass, W7 the tinted
glass red, W8 the unshared Fresnel's purple rim.

Nothing here touches the open neighbours: no metal (#164), no
translucency (#163), no Base Mixing 1 or nested Mix (#165).

## Process

- The parked Octane side had been rendered before four export changes;
  re-rendering it with HEAD first (one `calib-variants.sh` call, two
  scenes, ~1.5 min) was cheaper than reasoning about whether they apply.
- With the parts split in the scene itself (W3 / W6 = the gloss part
  alone), the compare decomposes without a second batch.
