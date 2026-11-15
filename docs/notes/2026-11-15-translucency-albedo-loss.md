# 2026-11-15 Universal's transmission loses the albedo's share (#163)

#134 left plain translucent spheres low in red: Calib-SSS S2 / S8 0.92 /
0.84, Calib-WeightedTL W3 / W4 0.86 – 0.88, W5 blue 0.73 (Octane / Iray,
enhanced-final), the same thin-walled or closed. Goal: split the
translucent sphere into parts before touching any number.

Variant: enhanced-final, 400 samples, denoised pass; numbers are linear
means over 0.6 R of each projected disc (`sphere_discs.py`), R G B.
Exports from the cached dumps; hypotheses as `orbx-variants.py set=`.
Octane batches: 16 variants ~9 min, 2 × 16 scenes before / after ~25 min.
No Iray render, no new scene: the Iray references of #134 and #45 serve.

## Reading the row first

Calib-SSS's S2 and S4 – S8 are one material in Iray (thin-walled shells,
so Transmitted Color / SSS / scattering distance never enter). Their Iray
red falls along the row (0.180 → 0.126, the HDRI's bright side is left),
and Octane's red ratio falls with it, 0.92 → 0.84, green / blue flat. The
same material at another place: the deficit follows the share of the
sphere's light that came through the walls, not the material.

## Decomposition in Octane (known inputs)

DAZ's MDL (`diffuse_base`: diffuse reflection at 1 − w over
`diffuse_transmission_bsdf`; thin-walled with no `backface`, which MDL
takes as the surface on both sides) makes a translucent sphere a cavity:
light through one wall bounces off the inner side's albedo a before
leaving, a gain of 1 / (1 − a). Octane renders, S2 / S8 changed only:

| variant | S2 R G B | S8 R G B |
|---|---|---|
| as exported (a = t = 0.306 / 0.134 / 0.086) | 0.1656 0.0741 0.0587 | 0.1054 0.0459 0.0402 |
| albedo 0 | 0.0302 0.0073 0.0037 | 0.0336 0.0083 0.0041 |
| transmission 0 | 0.1434 0.0678 0.0552 | 0.0805 0.0388 0.0364 |
| grey a = t = 0.3 | 0.1582 0.1779 0.2208 | 0.0999 0.1159 0.1584 |
| grey, albedo 0 | 0.0314 0.0356 0.0468 | 0.0344 0.0399 0.0508 |
| grey, transmission 0 | 0.1364 0.1528 0.1880 | 0.0755 0.0879 0.1227 |
| transmissionType 1 | = as exported | = as exported |

(full − transmission 0) / albedo 0, the light through the walls with the
albedo on over without it: grey 0.69 / 0.71 / 0.70 — one number in every
channel, so nothing spectral — and 0.73 / 0.86 / 0.95 at the exported
colour. That is 1 − a (0.70; 0.69 / 0.87 / 0.91), not Iray's 1 / (1 − a)
(1.43). Universal's diffuse transmission passes transmission × (1 − albedo)
per wall, the albedo first; with the inner reflection on, two walls give
t²(1 − a)² / (1 − a) = t²(1 − a). Not in the manual (it only says the
transmission is "mixed with" the diffuse colour). W3 (albedo a map, a =
0.23 red) the same: 0.82 / 0.97 (red / green; blue too dark to read).

An alternative (no inner reflection, each wall at t·√(1 − a)) fits the
two-wall number too; a single pass separates them, and the backlit cards
below (one wall) follow t(1 − a).

Prediction from these parts, with the pin ÷ (1 − a): S2 1.04 1.03 1.05,
S8 1.03 1.03 1.06, W3 1.02 1.02 1.05 × Iray — the opaque neighbour's
ratio (S1 1.06 1.03 1.06, W1 1.08 0.98 1.14).

## W5: a second, separate part

W5 has no albedo (translucency 1), so the loss can't be it. Each channel
of its transmission (linear 0.79 / 0.33 / 0.13) rendered as a grey: the
coloured sphere's channel over the grey sphere's same channel 0.91 / 1.05
/ 0.73 — exactly W5's ratio to Iray (0.94 1.09 0.73); the greys alone are
1.03 / 1.04 / 1.01 × Iray. Calib-SSS S3 the same (0.93 / 1.01 / 0.98;
greys 1.03 / 1.04 / 1.02). Octane's spectral rendering of a saturated
colour, squared by the two walls, as #87's medium. Left as it is (no
fitted counter-tint); noted on #169, which may share it.

## The change and the renders

`material.rs`: the transmission pin ÷ (1 − albedo mean), at most 1
(`albedo_mean`, the albedo as exported, its map's mean over the covered
texels). Before = HEAD's export, after = the change, Octane / Iray:

| scene | sphere / card | before | after |
|---|---|---|---|
| Calib-SSS | S2 / S4 – S8 | 0.84 – 0.92, 0.98 – 1.00, 1.04 – 1.05 | 1.04 – 1.05, 1.03 – 1.04, 1.05 – 1.06 |
| Calib-SSS-Solid | S2 / S4 / S8 | 0.84 – 0.92 ... | 1.05 1.03 1.05 ... |
| | S5 – S7 (baked volume, no transmission) | unchanged | |
| Calib-WeightedTL / -Solid | W3 / W4 | 0.86 – 0.88 1.00 1.06 | 1.03 1.05 1.04 |
| | W6 / W7 | 0.94 / 0.97 red | 1.03 / 1.04 |
| | W5 | 0.94 1.09 0.73 | same (spectral, above) |
| Calib-DualLobeWeightedTL | T2 | 0.91 1.04 1.04 | 0.98 1.05 1.04 |
| Calib-ColorEffect | C1 / C2 / C3 / C7 | 0.87 / 0.90 / 0.83 / 0.89 red | 0.93 / 0.93 / 0.90 / 0.95 |
| | C6 | 0.98 1.13 1.13 | 1.05 1.18 1.14 |
| Calib-MetalTL-Back | K8 | 0.93 1.07 1.03 | 1.03 1.08 1.03 |
| Calib-Thin-back-A | card 1 / 2 (backlit) | 0.96 0.86 0.98 / 1.15 0.88 1.18 | 0.99 1.00 0.98 / 1.20 1.00 1.24 |
| Calib-Leaf | card 2 (open, 2w) / 3 (thin) | green 0.78 / 0.88 | 0.90 / 1.00 |
| Calib-Thin-front-*, -back-B, -bce, Calib-Backscatter, Calib-PBRSkin | | within 0.01 – 0.02 | |

The cards' red / blue 1.20 – 1.24 are the green diffuse's near-black
channels (a ≈ 0.01, so unchanged by this; 1.15 – 1.18 before).

## Process

- Reading one material across a row of positions (the thin S2 / S4 – S8)
  told "light share, not material" before any render.
- Three Octane variants per sphere (albedo 0, transmission 0, both) plus a
  grey twin split it fully; the analytic cavity gave the expected number,
  so the fix is a division, not a fitted factor.
- Per-channel grey twins of a coloured transmission isolate Octane's
  spectral part in one render each.

Open: W5 / S3's spectral tint (#87's mechanism; #169 may share it).
