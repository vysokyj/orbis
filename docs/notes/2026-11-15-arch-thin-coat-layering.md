# 2026-11-15 — Thin architectural glass under a reflection layer (#168)

#149 left Calib-Furnace-ArchGlass3 J7 (NVIDIA architectural, thin-walled,
refr_gloss 0.2, reflectivity 0.6, Schlick 0.2 / 1 / 5, mirror reflection)
at Iray 0.553 disk against a `libbsdf` model of 0.487, Octane 0.525
(0.95 × Iray), and asked for an Iray round in architectural's frame. No
Iray was needed: the gap was in the offline model, the rest is Octane's.
Octane enhanced-final, 1024 samples, denoised pass; measure
`furnace-rings.py measure` (disk r < 0.75, rings), whole silhouette
r < 0.95 on the same discs.

## 1. The 0.487 was the offline tool's r0

`furnace-rings.py model --curve schlick` took Schlick's r0 from the IOR
(0.04 at 1.5), not architectural's `brdf_0_degree_refl` 0.2 — the
2026-11-13 / -14 predictions (0.503 all / 0.488 disk) were a layer of a
fifth the reflection. With `--r0 0.2` (new option) and τ solved so the
uncoated twin keeps `ARCH_GLASS_THIN_LOBE`'s 0.505:

| r_t | τ | r0 | all | disk |
|---|---|---|---|---|
| 0.5 (#149's 3.7 ×) | 0.779 | 0.04 | 0.503 | 0.483 |
| 0.5 | 0.779 | 0.2 | 0.552 | 0.535 |
| 0.6 (#117's r_t) | 0.812 | 0.2 | 0.549 | 0.522 |
| Iray J7 | | | 0.554 (r < 0.95) | 0.553 |

The export's own model (`mdl_layer::Stack`, thin, used by
`arch_glass_albedo`) always read r0 from the scene: 0.551 whole at r 0.5,
0.552 at 0.6 (a scratch test, not kept). So `libbsdf`'s layering
(base weight 1 − w · max(F(k1), F(k2))) holds on a thin wall; the closed
tie (ArchGlass2 G7, 0.657 v 0.655) was right for the same reason. The
issue's decomposing scenes (refr_gloss 0.4 / 0 under the layer,
reflectivity 0.3, the Fresnel term, the closed tie) aren't needed.

## 2. Octane renders below its own model

Re-exported at HEAD (r_t 0.6, k 0.692): J7 0.525 disk, 0.529 whole — the
Smith model it was solved on says 0.543 / 0.552. Octane-only variants
(`orbx-variants.py set=`, Universal Thin Wall, specular 0, coating
roughness 0), centre ring; models are a 1-D series (a straight thin wall
with a mirror coating meets every wall at the entry cosine):

| sphere | coating, IOR | k | Octane | both faces outside | always under | half under |
|---|---|---|---|---|---|---|
| J1 | 0.6, 2.62 | 1 | 1.003 | 1 | 1 | 1 |
| J2 | 0.6, 2.62 | 0 | 0.121 | 0.120 | 0.120 | 0.120 |
| J3 | 1, 2.62 | 0 | 0.205 | 0.200 | 0.200 | 0.200 |
| J4 | 1, 2.62 | 1 | 1.004 | 1 | 1 | 1 |
| J8 (J7 at r 0.46, IOR 2) | 0.6, 2.62 | 0.692 | 0.528 | 0.542 | 0.514 | 0.527 |
| J7 | 0.6, 2.62 | 0.692 | 0.528 | 0.542 | 0.514 | 0.527 |
| sweep J1 | 1, 6 | 0.7 | 0.703 | 0.750 | 0.667 | 0.700 |
| sweep J2 | 1, 6 | 0.5 | 0.597 | 0.633 | 0.579 | 0.598 |
| sweep J3 | 1, 6 | 0.85 | 0.819 | 0.864 | 0.785 | 0.819 |
| sweep J4 | 1, 2.62 | 0.7 | 0.569 | 0.592 | 0.548 | 0.569 |
| sweep J5 (no coating) | 0 | 0.7 | 0.491 | 0.490 | 0.490 | 0.490 |

- The coating reflects w · F at `coatingIor` and conserves energy (k 1:
  1.00; k 0: w · F) — so not a base weight of 1 − F, nor a loss inside.
- The base's roughness and IOR don't matter (J8 = J7).
- "Outside on both faces" (what `mdl_layer` assumed) is 3 – 6 % high,
  "the coating under the base when met from inside" (the manual's layer
  ordering by side, for Diffuse transmission) 3 – 6 % low; their mixture
  — under on half the inside hits: inner reflection R (1 + k²) / 2, inner
  transmission k (1 − R) — lands within 0.003 at the centre for every
  sphere, from r0 0.2 to 0.51 and k 0.5 to 0.85. Rings out to 0.5 too;
  at 0.6 – 0.9 Octane is 1 – 3 % under it in the coated spheres (J1 at
  k 1 dips 0.993 there as well).

Why one half isn't documented; it is the reading, not a fitted constant:
the 1/2 isn't tuned, the same rule holds across the sweep.

## 3. The change

`mdl_layer`'s Smith (Octane) path: a thin wall met from inside, a coat
reflection on half the hits passes the transmission twice (two tinted
events). k for J7 0.692 → 0.706. Octane / Iray, J7:

| | disk | whole (r < 0.95) | rings |
|---|---|---|---|
| Iray | 0.553 | 0.554 | 0.566 → 0.551 |
| before | 0.525 (0.95) | 0.529 (0.95) | 0.527 → 0.536 |
| after | 0.541 (0.98) | 0.546 (0.99) | 0.541 → 0.553 |

The uncoated spheres don't move (J1 – J5, J8 as before, J5 0.514).
Every thin architectural glass with reflectivity > 0 (the default 0.6)
gets a slightly larger k.

## Process lessons

- An offline model that disagrees with Iray gets checked against the
  export's own model first: the two had different inputs (r0), and the
  "Iray keeps more" was a CLI default.
- Before an Iray round for a gap between Octane and Iray, compare Octane
  with the model it was solved on — here that alone located the gap
  on the Octane side, and orbx variants with known inputs settled it in
  two renders.
