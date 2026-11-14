# 2026-11-14 PBRSkin occlusion map: Octane compare (#158)

Resume of #158 after c61d179 (the occlusion weight's map per texel, from
`pbr_skin.mdl`, no fit). Its Octane render had waited for Octane X after
#161's render engine failure; Octane X was restarted with the user's
approval. Iray's reference is the existing one
(`calib/references/iray/Calib-PBRSkin-Occlusion.png`, 5000 samples); no
new Iray.

## Run

`calib-variants.sh enhanced-final` with the absolute `.duf` path (the dump
is keyed by it, #159), the variant folder empty first. One render, 400
samples, denoised pass; the export bakes the two derived maps (weight pin,
IOR pins). Compared on the denoised pass, discs from the dump
(`sphere_discs.py`), linear luminance at gamma 2.2, mean over the upper
0.9 R as in `highlight-spread.py`; a scratch script reproduced the
earlier note's table on the old render to within 0.02 before reading the
new one.

## Numbers

Every sphere is compared only with itself across renderers (each position
sees a different sky; 2026-11-14-pbrskin-occlusion-compare).

| sphere | Octane / Iray | ÷ O1's | spread Iray | spread Octane | rim / O1 Iray | rim / O1 Octane |
|---|---|---|---|---|---|---|
| O1 dual lobe | 1.18 | 1.00 | 6.2 | 7.0 | 1.00 | 1.00 |
| O6 grey map, before (channel value) | 0.85 | 0.72 | 9.2 | 8.5 | 1.05 | 0.59 |
| O6 grey map, now (per texel) | 1.27 | 1.08 | 9.2 | 12.5 | 1.05 | 1.11 |
| O5 weight 0.5 (for scale) | 1.20 | 1.02 | 7.5 | 9.9 | 0.87 | 0.85 |

O1 - O5, O7, O8 unchanged from the earlier note (within 0.01).

Radial bins of O6 / O1 (centre / mid / rim): Iray 0.54 / 1.00 / 1.05,
Octane 0.53 / 1.09 / 1.11 (before 0.44 / 0.72 / 0.59).

## Reading

- The map now counts: O6 moved from O2's look (strong occlusion) to a
  weak one at w 0.22, as Iray renders it - mean 1.08 of O1's, rim within
  6 % of Iray's rim / O1. The gamma "auto" reading of grey 128 (w 0.22,
  #93's map-gamma rule) is confirmed by the rim.
- Octane's O6 highlight is sharper than Iray's (spread 12.5 vs 9.2, 1.20
  of O1's 1.13 ratio). O5 shows the same at weight 0.5 (9.9 vs 7.5, 1.17
  of O1's): a partial occlusion leaves more of O1's own Octane excess in
  the mid band, the PBRSkin dual-lobe baseline (#57), not the map. Not
  fitted.

## Where it stands

#158 closed: the occlusion map rule holds against Iray. Open elsewhere:
detail normal (#140 / #166), relative dump paths (#159), PBRSkin baseline
(#57).
