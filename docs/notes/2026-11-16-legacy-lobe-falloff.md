# 2026-11-16 — The legacy lobe per closed event, Octane's transmission falloff (#117)

Resume of #117 with its third round rendered (Iray Server, 5000 samples,
70453f2): Calib-Furnace-MdlLegacy2 (`orbis_legacy2_v10` / `_v13`).
Questions (note 2026-11-15 § 4): is `libbsdf`'s thin wall right at high
r, and why do thin walls lose more per event than closed spheres above
r 0.3 — the bending (IOR) or the thin model. Measure: `furnace-rings.py
measure`; model: `furnace-rings.py model` (`libbsdf`), fits with its
`--loss` hook (numpy, scratch scripts). Octane enhanced-final, denoised.

## 1. MdlLegacy2

| sphere | Iray disk | centre → 0.8 – 0.9 | `libbsdf` disk | centre → rim |
|---|---|---|---|---|
| L1 1.0 closed IOR 1.05 r 0.6 | 0.761 | 0.834 → 0.625 | 0.983 | 1.000 → 0.865 |
| L2 1.0 closed IOR 1.05 r 1 | 0.529 | 0.638 → 0.366 | 0.918 | 0.967 → 0.670 |
| L3 1.0 closed IOR 1.2 r 0.6 | 0.710 | 0.817 → 0.520 | 0.912 | 0.981 → 0.756 |
| L4 1.0 closed IOR 2 r 0.6 | 0.456 | 0.524 → 0.340 | 0.621 | 0.658 → 0.545 |
| L5 1.3 thin r 0.6 | 0.708 | 0.701 → 0.755 | 0.739 | 0.732 → 0.778 |
| L6 1.3 thin r 1 | 0.449 | 0.407 → 0.559 | 0.474 | 0.429 → 0.581 |
| L7 1.0 closed IOR 2 r 1 | 0.211 | 0.244 → 0.148 | 0.378 | 0.392 → 0.332 |
| L8 1.3 closed IOR 1.05 r 1 | 0.906 | 0.962 → 0.668 | 0.918 | 0.967 → 0.670 |

- **`libbsdf`'s thin wall is 4 – 5 % high at r ≥ 0.6** (L5 0.958, L6
  0.947 × in every ring; closed L8 0.99, K8 0.99). The 2026-11-15 thin g
  table was read against it; re-read against Iray's own 1.3 thin wall,
  g = √(K6 / L5), √(K7 / L6) per ring: r 0.6 0.854 → 0.807, r 1 0.689 →
  0.630 (centre → cos 0.53), 2 – 3 % above the old rows. Below r 0.4
  `libbsdf` thin = 1.3 (X6 / X8), those rows stand.
- **Closed spheres don't lose g² of the thin wall, whatever the IOR.**
  Iray / `libbsdf` on the disk: r 0.6 0.774 / 0.778 / ~0.78 / 0.734 at IOR
  1.05 / 1.2 / 1.5 (A2) / 2; r 1 0.576 / 0.590 / 0.558 at 1.05 / 1.5 / 2,
  resin's reflect_transmit (U6) 0.56. Nearly IOR-independent — so not the
  bending. Even at IOR 1.05 (two barely bent events, no total reflection)
  the centre keeps 0.834 (r 0.6) / 0.660 (r 1), about one thin-wall event
  (0.854 / 0.689), where the thin wall loses two.
- The thin g at every closed event under-predicts (0.91 / 0.78 / 0.73 ×
  Iray at r 0.6 / 1), outside events only or inside only over-predict
  (1.08 – 1.20 ×); g^0.7 at every event gets the disks within 6 % but the
  rings tilt 0.86 → 1.11 at r 1: the thin wall's angular shape isn't the
  closed one's.

So the resume's second branch: the thin wall is the odd one (a thin wall
loses about g_closed² per wall head-on), g for closed solids is taken from
the closed spheres alone, and `ARCH_GLASS_THIN_LOBE` stays measured.

## 2. g for closed solids, per event

Per MDL roughness one curve g(θ) = head + (grazing − head)(θ / 90°)^skew
at the event's incoming angle, on every base event of `libbsdf`'s closed
sphere, fitted to the rings (centre to 0.8 – 0.9) of every 1.0 sphere at
that r (Nelder – Mead, 65 k paths):

| r | spheres | head | grazing | skew | worst ring |
|---|---|---|---|---|---|
| 0.05 | T1 | 1.009 | 0.971 | 0.30 | 0.99 |
| 0.1 | T2 | 0.995 | 0.970 | 0.46 | 0.99 |
| 0.2 | T3, X1 (IOR 1.2), X3 (IOR 2) | 0.986 | 0.967 | 0.45 | 0.98 |
| 0.25 | U3 (rt) | 0.977 | 0.956 | 1.13 | 0.99 |
| 0.3 | K1 | 0.970 | 0.942 | 1.72 | 0.99 |
| 0.4 | T4 | 0.957 | 0.860 | 2.84 | 0.98 |
| 0.5 | K2, U4 (rt) | 0.939 | 0.769 | 2.21 | 1.03 |
| 0.6 | L1, L3, A2, L4 (IOR 1.05 – 2) | 0.922 | 0.753 | 1.47 | 0.96 / 1.04 |
| 0.7 | K3 | 0.913 | 0.611 | 1.47 | 0.99 |
| 0.75 | U5 (rt) | 0.886 | 0.559 | 1.92 | 1.06 |
| 1 | L2, K4, L7, U6 (IOR 1.05 – 2, rt) | 0.840 | 0.609 | 1.11 | 0.88 (L2 rim) |

One curve holds IOR 1.05 to 2 and both scatter modes (transmit,
reflect_transmit) at r 0.2, 0.5, 0.6 and 1 within 1 – 5 % per ring (the
worst L2's rim, 0.88): the loss is the lobe's per event, as the 2026-11-14
note said, but **not the thin wall's g** — head-on it is about √ of it
(r 0.6 0.922 v √0.854 = 0.924, r 1 0.840 v √0.689 = 0.830). Rows by r are
single fits (skew is loosely held where the loss is small; grazing at r 1
lies above r 0.75's, at θ 60° the rows fall monotonically 0.746 / 0.736
/ 0.693 at r 0.7 / 0.75 / 1). Measured, not derived: `LEGACY_LOBE` in
`mdl_layer.rs`, interpolated linearly in r between the rows' curves.

## 3. Octane: a Falloff Mix on the transmission

The construction of the 2026-11-13 note § 3, solved now: Octane's
transmission = the tint × k head-on, × k · q at 90°, (θ / 90°)^skew
between (Falloff Map, normal vs. eye ray, per hit). Feasibility in numpy
first (Smith at the export's α against Iray's measured rings directly):
one k left U4 / U5 / U6 / A2's rims 1.13 / 1.25 / 1.32 / 1.17 × Iray, the
falloff (k, q, s) every ring within 0.97 – 1.03.

In the export (`mdl_layer::transmission_falloff`): Iray's model is
`libbsdf` × `legacy_loss` on every base event, τ on top so the lobe alone
keeps the measured disk (`RESIN_LOBE`, `ARCH_GLASS_LOBE`; τ ≈ 1 now);
Octane's Smith paths are traced once with each transmission event's
incoming cosine kept, so the falloff only reweights them; (k, q, skew)
solved by Nelder – Mead on the white sphere's rings (each by its share of
the silhouette), then k per channel for the whole sphere at the tint.
0.1 – 0.3 s per material. Uncoated resin, model rings Octane / Iray: r
0.25 / 0.5 / 0.75 / 1 within 1 / 1 / 5 / 5 % (MC noise at 65 k paths);
Iray's model meets U4 – U6's measured rings to 1 – 3 %. Thin walls keep
one flat k (Octane's Thin Wall).

## 4. Octane renders (enhanced-final, denoised)

Re-exported and rendered Calib-Furnace-ArchGlass, -ArchGlass2, -Resin,
-Resin2, Calib-Resin, Calib-ArchGlass. Rings centre → 0.8 – 0.9, Octane
before (one k, 2026-11-15) / after (falloff) v Iray:

| sphere | before | after | Iray |
|---|---|---|---|
| ArchGlass G4 refr_gloss 0.2 | 0.735 → 0.514 | 0.764 → 0.457 | 0.717 → 0.431 |
| ArchGlass G5 refr_gloss 0 | 0.583 → 0.424 | 0.634 → 0.345 | 0.609 → 0.345 |
| ArchGlass G7 0.2 under refl 0.6 | 0.747 → 0.551 | 0.771 → 0.507 | 0.749 → 0.484 |
| ArchGlass G8 0.2, Fresnel term | 0.514 → 0.389 | 0.436 → 0.368 | 0.459 → 0.374 |
| ArchGlass2 H6 IOR 1.2 | 0.853 → 0.615 | 0.892 → 0.548 | 0.817 → 0.526 |
| ArchGlass2 H7 IOR 2 | 0.508 → 0.386 | 0.514 → 0.341 | 0.524 → 0.344 |
| Resin2 U4 r 0.5 | 0.805 → 0.609 | 0.819 → 0.568 | 0.783 → 0.517 |
| Resin2 U5 r 0.75 | 0.568 → 0.447 | 0.578 → 0.380 | 0.538 → 0.312 |
| Resin2 U6 r 1 | 0.399 → 0.342 | 0.402 → 0.304 | 0.376 → 0.215 |
| Resin2 U8 | 0.671 → 0.625 | 0.817 → 0.528 | 0.834 → 0.514 |
| Resin sphere 1 (coated) | 0.449 → 0.441 | 0.541 → 0.378 | 0.538 → 0.357 |
| Resin sphere 3 (coated) | 0.672 → 0.622 | 0.818 → 0.523 | 0.834 → 0.508 |
| Resin sphere 7 (coated grey r 0.5) | 0.266 → 0.244 | 0.305 → 0.219 | 0.281 → 0.173 |

Disks (r < 0.75) Octane / Iray before → after: G4 1.06 → 1.05, G5 1.03 →
1.01, G7 1.02 → 1.01, G8 1.11 → 0.96, H6 1.06 → 1.06, H7 1.00 → 0.98, U4
1.06 → 1.04, U5 1.13 → 1.12, U6 1.16 → 1.17, coated resin 1 / 3 / 7 0.95
/ 0.93 / 0.97 → 1.05 / 1.03 / 1.05. The flat profiles are gone: every
closed sphere now falls to the rim as Iray does (G5's rim exact, G4 1.19 →
1.06 ×, coated resin 3 1.22 → 1.03 ×). What stays is Octane over its own
Smith model, largest at high α: centres 1.04 – 1.09 × and U5 / U6's rims
1.22 / 1.41 × (the model put them within 5 %; #148 measured Octane 3 – 4 %
over the model at α 0.36, more at α ≥ 0.5) — not folded in. Thin walls
(H4 / H5) unchanged by construction.

Lit scenes (`calib-compare.py regions`): Calib-Resin cube2 (the rough
resin) 1.32 → 1.29, the rest within 0.01; Calib-ArchGlass (after only)
cube2 0.87, cube1 / cube3 1.02 / 1.03, floor 1.03.

## Where it stands

Export changed: `LEGACY_LOBE` per closed event in Iray's model, Octane's
transmission a Falloff Mix solved per material (resin, closed
architectural glass). #117 closes: the angular loss is drawn. Open
elsewhere: Octane over its Smith model at α ≥ 0.5 (the residual U5 / U6
rims), thin walls stay one flat k.

## Process lessons

- A feasibility fit against the measured rings (numpy, minutes) before
  touching Rust said the three-parameter falloff can do it (0.97 – 1.03
  per ring) — and that the Iray model needed its own closed-sphere table,
  since the thin walls' g at every event tilted the rings 0.86 → 1.11.
- The falloff only reweights paths: trace each model once, keep each
  transmission event's cosine, and the solve costs 0.1 – 0.3 s.
