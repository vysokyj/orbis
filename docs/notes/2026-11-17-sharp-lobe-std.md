# 2026-11-17 — Faithful's sharp dielectric lobes at STD (#196)

From [the dual lobe's base share](2026-11-16-dual-lobe-base-share.md)
(#193): Iray keeps the diffuse under Uber's dual lobe at a flat 1 − R W;
under a light near the view Octane's STD leaves the base that flat where
Beckmann (the export's model for every lobe at α ≤ 0.25) dims it to
0.84 – 0.87 at 0.9 R, and both draw the same sharp lobe (point 1.013 –
1.018 × Iray, furnace 1.29 – 1.36). The proposal: Faithful draws sharp
dielectric lobes at STD too. Contested, so judged on real scenes before
deciding. No new Iray render: every scene below had its reference.

## 1. What the change touches

`dielectric_brdf` / `RoughLobe::sharp_brdf`: a node with no metal part
(Metallic Weight 0 or Base Mixing 1, the layered Top Coat's Specular
Layer, an NVIDIA dielectric preset) takes the Look's sharp model,
Beckmann in Enhanced, STD in Faithful. Nodes with a metal part (all-metal
`metal_brdf`, the weighted metal gloss, partial metals) keep Beckmann
for sharp lobes: STD on sharp metals is unmeasured, and Cast Iron's
detail (#92) was fitted at Beckmann. Enhanced's exports are unchanged.

Faithful exports before / after, `brdf` pins that moved (`orbx_ocs get`,
a diff per material):

| scene | changed |
|---|---|
| Calib-Glossy | 8 of 9 (every dielectric sphere and the floor) |
| Calib-DualLobeGlossy (+ -bare, -dl, -gloss) | 8 – 9 of 9 – 10 (incl. G7's Top Coat Specular Layer) |
| Calib-Point-UberGloss | U1 only (r 0.25; the rest rough or metal) |
| Calib-Skin-{Victoria,Kayo,Gabriela,Nishi}-front | 9 – 11: floor, grey sphere, eyes (cornea, moisture, sclera, irises, pupils), teeth, mouth / lips Top Coat, eyelashes |

Matte surfaces (floor, grey sphere, G1) change model too - their specular
is 0, nothing to draw: the frame totals moved < 0.2 %.

## 2. Renders (faithful-final, 400 samples, denoised; ~60 s a scene)

`~/Library/Caches/orbis/calib/octane/c196-before` / `c196-after`.
Over the 0.9 disc (`highlight-spread.py --vs`), Octane / Iray, and the
highlight spread p99 / median (Iray's in brackets):

| scene | before | after | spreads that moved (Iray) |
|---|---|---|---|
| Calib-DualLobeGlossy | 1.12 – 1.35 | 1.12 – 1.25 | G7 17.3 → 16.1 (13.7), G8 19.2 → 15.9 (15.0), G6 14.0 → 13.1 (11.6) |
| Calib-DualLobeGlossy-gloss | 1.08 – 1.35 | 1.08 – 1.22 | G8 14.4 → 9.6 (9.3), G7 12.6 → 11.3 (10.7) |
| Calib-Glossy | 0.95 – 1.23 | 0.95 – 1.22 | G8 25.5 → 22.8 (19.5), G2 12.4 → 11.7 (13.0), G4 20.7 → 19.1 (20.2) |
| Calib-Point-UberGloss | U1 1.03 | U1 1.01 | |

Where it moved (relative difference image): on the HDRI scenes a crescent
at each glossy sphere's rim on the sun's side - the sharp lobe at grazing,
which #193's furnace found 1.4 – 1.6 × Iray at 0.85 – 0.9 R under
Beckmann. STD draws less of it there: over the pixels that moved,
Calib-DualLobeGlossy 1.33 → 1.18 × Iray, -gloss 1.31 → 1.15, Calib-Glossy
1.14 → 1.13. The sphere centres (`calib-compare-all.py`'s middle square)
moved ± 0.01.

Calib-Skin fronts, eye box (x 400 – 560, y 170 – 240, both eyes and the
brows), before → after × Iray: mean 1.031 → 1.031 / 0.951 → 0.950 /
0.970 → 0.969 / 0.987 → 0.987, p99 0.944 → 0.943 / 0.931 → 0.927 /
0.948 → 0.949 / 0.958 → 0.963 (Victoria / Kayo / Gabriela / Nishi); side
by side no visible change in the eyes, lips or teeth. The rest of the
frame differs by the denoiser's grain only.

## 3. Decision

Faithful at STD for sharp dielectric lobes: every sphere moved toward Iray
or stayed, the faces didn't move. Through the existing `rough_lobe` field
(as #175), `RoughLobe::sharp_brdf`, commented at the value. Enhanced keeps
Beckmann as the issue proposed; the HDRI scenes would favour STD there
too (Iray's level), a call to revisit if Enhanced should follow.

## Process

- A before / after diff of the export's `brdf` pins per material first:
  it says which materials of a real scene the rule touches (eyes, teeth,
  lips) before any render, and the moved-pixel mask after it.
- A moved-pixel mask over a denoised 400-sample face picks up grain;
  read a fixed region (the eye box) and look at the crops before quoting
  a p99 over the mask.
