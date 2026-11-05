# 2026-11-05: Blended Dual Lobe hair against Iray — gradient, highlight, separation (#42)

Follows [2026-11-04](2026-11-04-blended-hair-root-tip.md). Iray references
(Iray Server, 5000 samples, rendered overnight) of three local copies of
Calib-Hair (a store hair: scenes, dumps, renders stay out of git):

| scene | differs from Calib-Hair |
|---|---|
| Calib-Hair-tip | root dark brown, tip blond |
| Calib-Hair-tipbg | as -tip + Root To Tip Bias 0.2 / Gain 0.7 |
| Calib-Hair-hl | Highlight Weight 0.8 (0.049), Highlight Root / Tip (0.2, 0.4, 1) as entered, highlight_roughness 0.3 (= base_roughness), separation 0.2 (0) |

Octane: Enhanced + Final, 400 samples, the scenes' 1920 × 1080;
renders: three single-scene tries plus the Calib-Hair check took 5 min in all (~1 min a render; no crash, no freeze).

Measures (linear, Octane / Iray): a **hair mask** (pixels where Iray's -hl
differs from Calib-Hair's by > 0.02 in summed RGB, head box 820 – 1100 ×
200 – 480: only the hair changed between the two), median and mean; and
four small regions — crown (910 – 1020 × 310 – 370), top tuft, the two
sides. The regions catch edges and skin, so the mask's mean is the energy
measure; the earlier notes' "crown" was a different crop (not comparable).

## 1. Root → tip (the be96caa rule, unchanged)

| scene | mask mean L | crown L | tuft L |
|---|---|---|---|
| Calib-Hair | 0.85 | 0.78 | 0.85 |
| Calib-Hair-tip | 0.92 | 1.09 | 1.00 |
| Calib-Hair-tipbg | 0.89 | 1.05 | 0.96 |

tipbg / tip (mask mean): Octane's ratio 0.97 × Iray's (0.98 with the
highlight rule below); crown 0.75 vs Iray 0.80, tuft 0.72 vs 0.75 (linear
luminance, rounded medians) — the bias holds the dark root a little
further in Octane, within the regions' noise. The
direction (W 0 = root) and the bent blend land where Iray's do. Nothing to
change.

## 2. Highlight

`dual_lobe_hair.mdl`: `weighted_layer(highlight_weight, simple_glossy(tint
highlight colour), base: weighted_layer(hair_weight, simple_glossy(tint
hair colour), base))` — no Fresnel: Highlight Weight hw takes hw of the
light, the rest gets 1 − hw. In Iray, -hl's whole hair turns vivid blue:
mask crown (0.009, 0.011, 0.041) against Calib-Hair's (0.050, 0.034,
0.027). (hl − 0.2 × base) / 0.8 ≈ 0.043 × the linear tint (blue 0.044,
green 0.040 / 0.133) at the crown: a linear tinted layer.

Octane's Hair has one R lobe, Fresnel-weighted (`specular` 0.25 → 0.03
moved the crown 3 %, 2026-10-27): it can't carry 80 % of the light. So:
albedo = (1 − hw) × (the colour / transmission shares) + hw × S × the
highlight colour, and the R lobe tinted the same way.

| try | S | specular | mask median R / G / B | median L | mean L | mean b/g (Iray 2.29) |
|---|---|---|---|---|---|---|
| unmapped | – | gloss | 3.68 / 2.52 / 0.61 | 2.17 | 1.25 | 1.18 |
| a | 0.15 | gloss | 1.18 / 1.10 / 0.95 | 1.07 | 0.85 | 2.38 |
| **b** | **0.15** | (1 − hw) gloss + hw × hl | **1.00 / 1.05 / 1.03** | **1.04** | **0.84** | 2.56 |
| c | 0.25 | as b | 1.06 / 1.26 / 1.48 | 1.29 | 0.94 | 2.99 |

S = 0.15 came from the arithmetic first (Octane's crown response ≈ 0.3 ×
albedo in blue: 0.036 / 0.3 / 0.8) and landed. Kept b
(`HAIR_HIGHLIGHT_SHARE` 0.15): balanced median, mean on par with
Calib-Hair's 0.84 – 0.88. Octane's highlights are a little greyer and
less saturated than Iray's vivid blue sheen (seen side by side); mean b/g
is still higher.

Side effect at Calib-Hair's own hw 0.049 (pale yellow highlight): mask mean
Calib-Hair 0.85 → 0.85, -dark 0.88, -darktr 0.84 (the colour-share fit
copies, now all within 0.84 – 0.88); the dark-rooted -tip / -tipbg gain
2 – 3 % (mean 0.94 / 0.92; their small dark crown region 1.20 / 1.19).

**highlight_roughness** can't be fitted here: -hl has it equal to
base_roughness (0.3). Octane's Hair has one roughness for all lobes; left
on base_roughness.

## 3. separation → `offset`

Octane's `offset` (probe of 147's pins, 2026-11-04: float texture,
default 0, slider 0 – 1, "shifts the specular highlight away from the
perfectly reflective direction") tilts the one R lobe. Iray's separation
tilts the highlight's normal −separation and the hair glossy's
+separation along the tangent (opposite ways; atan 0.2 = 11°).

| offset | mask median L | mean L | log-L correlation with Iray (blur 1 px) |
|---|---|---|---|
| 0 | 1.04 | 0.84 | 0.950 |
| 0.2 (= separation) | 0.82 | 0.79 | 0.932 |
| 1.0 (5 ×) | 0.60 | 0.73 | 0.951 |

Offset only dimmed the hair and didn't bring the pattern closer: left at
0. Its unit stays unidentified — on this messy, HDRI-lit hairstyle the
highlight shift isn't measurable; a straight-strand scene under one light
would be needed, and nothing in DAZ's scenes seems to depend on it.

## Process

- Env knobs in the exporter (H42_S / _SP / _OFF, removed before the
  commit) let one build render every try; one batch per try, Calib-Hair-hl
  only, then a confirming batch of all six scenes with the final code.
- The region medians disagree with the mask means by up to 0.3 on
  Calib-Hair (crown 0.78, mean 0.85): small regions on a strand hair catch
  background between the strands. Compare tries on the same mask.
