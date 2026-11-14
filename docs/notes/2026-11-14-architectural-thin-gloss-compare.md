# 2026-11-14 — Architectural thin wall above refr_gloss 0.2: compare (#149)

Resume of #149 (prepared in
[2026-11-13 architectural thin gloss](2026-11-13-architectural-thin-gloss.md)):
Calib-Furnace-ArchGlass3's Iray reference (5000 samples, denoiser on@8,
d226a45) against the Octane render and the `libbsdf` models. Measure:
`furnace-rings.py measure` (sphere / dome per ring, disk r < 0.75), plus
the whole silhouette (r < 0.9 / 0.95, a scratch script on the same discs).
Iray dome 0.1701, Octane 0.1737.

## Iray

| sphere | disk | r < 0.9 / 0.95 | rings centre → 0.8 – 0.9 |
|---|---|---|---|
| J1 thin 1 | 1.000 | 1.000 / 1.000 | flat 1.000 |
| J2 thin 0.8 | 0.968 | 0.966 / 0.965 | 0.993 0.981 … 0.962 |
| J3 thin 0.6 | 0.963 | 0.962 / 0.960 | 0.981 … 0.962 0.960 |
| J4 thin 0.4 | 0.936 | 0.923 / 0.915 | 0.943 … 0.925 0.915 0.885 |
| J5 thin 0.2 | 0.516 | 0.513 / 0.513 | 0.524 … 0.505 |
| J6 closed 0.4 | 0.936 | 0.913 / 0.900 | 0.962 … 0.896 0.846 |
| J7 thin 0.2, refl 0.6 Schlick | 0.553 | 0.552 / 0.554 | 0.566 … 0.544 0.551 |
| J8 thin 0.4, IOR 2 | 0.937 | 0.923 / 0.915 | = J4 ring for ring |

The denoised 8-bit reference sits on plateaus ~2 % apart near white
(0.943 / 0.962 / 0.981 / 1.000): differences below one step don't count.

1. **The frame ties.** J5 v H4 (ArchGlass2) 0.516 v 0.505, J6 v G3 0.936
   v 0.938 — within a step. The 0.2 point stays H4's.
2. **J1 – J4 replace the stand-ins**, over the whole silhouette (r <
   0.95, #147's rule, as Octane's Thin Wall draws one flat k²): 1 / 0.965
   / 0.960 / 0.915 at refr_gloss 1 / 0.8 / 0.6 / 0.4 (stand-ins 1 / 0.982
   interpolated / 0.964 / 0.938). The 0.4 profile isn't flat (0.943 →
   0.885), so its disk (0.936) and whole (0.915) differ; the table holds
   the whole.
3. **J8 = J4** in every ring: the thin loss doesn't depend on the IOR (the
   code's assumption; no `THIN_LOBE_IOR`).
4. **J7 is 10 % over the `libbsdf` layering** — see below.
5. **It is #117's g, not a curve of its own.** `libbsdf`'s thin wall
   keeps 1.000 / 0.996 / 0.975 at J2 – J4's roughness (3.7 × the formula:
   0.027 / 0.071 / 0.188), so nearly everything Iray loses there is
   NVIDIA's lobe: J4 (0.936, 0.943 → 0.885) is #117's plain `mdl 1.0`
   thin sphere at r 0.2 (X5: 0.939, 0.943 → 0.893), and `libbsdf` × a
   flat τ 0.969 per event at r 0.188 gives 0.916 whole, 0.939 → 0.906 —
   #117's g(0.2) ≈ 0.975, flat; the rim fall is mostly `libbsdf`'s own
   (1.000 → 0.962 at τ 1). J2 / J3 lose
   ~2 % per wall at r 0.03 – 0.07, falling to the rim (#117's closed r
   0.05 / 0.1 1.0 / 1.3 ratios 0.998 → 0.965, 0.989 → 0.956, within a
   step); the mirror (J1) loses nothing. So the table is g², read at
   architectural's roughness; nothing is fitted to it. When #117 tabulates
   g(r, cos), `ARCH_GLASS_THIN_LOBE` and `ARCH_GLASS_LOBE` can both be
   derived from it.

## J7: the reflection layer on a thin wall

Iray J7 / J5 = 1.07: the reflectivity-0.6 Schlick layer (r0 0.2, mirror)
lifts the thin sphere from 0.516 to 0.553, flat (0.566 → 0.551). The
`libbsdf` model (`furnace-rings.py model --thin --weight 0.6 --curve
schlick --coat-rough 0`, τ 0.78 per event so the uncoated one keeps H4's
0.505) predicts the opposite, 0.503 all / 0.487 disk, and Octane (k solved
to that) renders 0.525. Tries, offline:

| model | disk | rings |
|---|---|---|
| as `mdl_layer.rs`: base weight 1 − w · max(F(k1), F(k2)) | 0.487 | 0.506 → 0.491 |
| base weight 1 − w · F(k1) (energy-conserving) | 0.513 | 0.529 → 0.521 |
| base weight 1 − w · F(k2) | 0.496 | 0.513 → 0.510 |
| Iray | 0.553 | 0.566 → 0.551 |

Even an energy-conserving layer reaches only 0.513: each wall's base
takes (1 − wF) ≈ 0.88 on the way in and out, and the inner reflections
meet the lossy lobe again. Iray behaves nearer to the front reflection
plus the uncoated sphere behind it (centre 0.12 + 0.88 · 0.524 = 0.581,
less a little) — as if the layer's weight didn't cost the lobe both walls
of transmission, or g weren't applied under the layer. The release notes
change the custom-curve half vector and energy for *non*-thin-walled
transmissive materials only, so the thin path is a different code path in
`libbsdf`; which one, the docs don't say. Not settled from one sphere:
filed as #168 with the variants that split it (the closed coated tie, a
near-lossless lobe under the same layer, the Fresnel term, a second
weight). Octane is 0.95 × Iray on J7 until then.

## Octane after the change (enhanced-final, 1024, denoised)

Octane / Iray, whole silhouette (r < 0.95) and disk, before (stand-ins,
f3391a6) → after:

| sphere | whole | disk | Octane, flat |
|---|---|---|---|
| J1 thin 1 | 1.00 → 1.00 | 1.00 → 1.00 | 1.001 |
| J2 thin 0.8 | 1.02 → 1.01 | 1.02 → 1.01 | 0.985 → 0.979 |
| J3 thin 0.6 | 1.01 → 1.00 | 1.01 → 1.00 | 0.971 → 0.961 |
| J4 thin 0.4 | 1.03 → 1.01 | 1.01 → 0.99 | 0.942 → 0.924 |
| J8 thin 0.4 IOR 2 | 1.03 → 1.01 | 1.01 → 0.99 | 0.942 → 0.924 |
| J5 / J6 / J7 | unchanged (1.00 / 1.02 / 0.95) | | |

Octane keeps the table to 0.001 – 0.014 (Smith model v render); per ring
J4 now 0.98 at the centre, 1.04 at 0.8 – 0.9 (Iray falls, Thin Wall
can't).

## Where it stands

`ARCH_GLASS_THIN_LOBE` holds measured points at every refr_gloss
architectural spheres were rendered at (0 / 0.2 / 0.4 / 0.6 / 0.8 / 1),
the spec's thin-wall rule says what they are. The thin layering (J7) is
#168; deriving both architectural tables from g stays with #117.

## Process lessons

- The thin spheres' disk and whole differ once the profile falls to the
  rim (J4 0.936 v 0.915): name which one a table holds; for a Thin Wall
  (one flat k² in Octane) it has to be the whole.
- A check variant (J7) packed into a measurement scene paid off: it
  showed the layering wrong at once, but one sphere can't say why — next
  time put its decomposing twins (a near-lossless lobe under the same
  layer) beside it.
