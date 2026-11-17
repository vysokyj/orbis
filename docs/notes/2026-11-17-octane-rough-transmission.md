# 2026-11-17 — Octane over its Smith model in rough transmission (#185)

Question (#185, from #117 / #191): with the transmission Falloff Mix,
Octane's Smith model meets Iray's rings, but Octane itself renders more —
uncoated resin (Calib-Furnace-Resin2) U5 / U6 (r 0.75 / 1, α 0.6 / 0.71)
disks 1.12 / 1.17 × Iray, rims 1.22 / 1.41; #148 had Octane 3 – 4 % over
the model at α 0.36. Decompose first: plain closed spheres at a known α,
Octane against its own model, no Iray.

## Probe

Calib-Furnace-UberGlass's export (enhanced-final, `--no-denoise`, raw
pass), its eight closed spheres re-pinned with `orbx-variants.py set=`:
Beckmann (`brdf` 1, as exported), roughness pin √α for α 0.25 / 0.36 /
0.45 / 0.5 / 0.6 / 0.7 / 0.85 / 1, IOR 1.5, transmission white, no
falloff, five variants:

| variant | pins | what it isolates |
|---|---|---|
| `tr` | specular 0 | the transmission lobe alone (inside, total reflection) |
| `rt` | specular 1 | reflect + transmit by Fresnel: resin's base, Uber's shared glass |
| `refl` | specular 1, transmission 0, albedo 0 | one outside reflection event (η 1.5) |
| `inv` | as `refl`, IOR 0.6667 | one reflection event of the *inside* kind (η 1 / 1.5, total reflection past 41.8°), seen from outside |
| `ior50` | as `refl`, IOR 50 | the lobe's masking with F ≈ 0.92 flat |

e.g. `set=T1_.roughness=f0.5+T1_.transmission=c1+T1_.specular=c0+…`
for all eight; Octane 1024 samples (`tr`, `rt`: 388 / 359 s), 512 the
rest (41 – 82 s). Rings: `furnace-rings.py measure`; model:
`furnace-rings.py model --model smith` (2^18 paths), the single events by
a scratch integral of F · D · G over the same rings. The dome
normalisation reads 1.01 at α 0.25's centre (Octane's 1 %, all variants).

## 1. The full spheres

Disk (r < 0.75) Octane / Smith, rings centre → 0.8 – 0.9:

| α | `tr` disk | `tr` rings | `rt` disk | `rt` Octane | `rt` model | `rt` rings |
|---|---|---|---|---|---|---|
| 0.25 | 1.01 | 1.01 → 1.01 | 1.01 | 1.009 → 0.911 | 1.000 → 0.895 | 1.01 → 1.02 |
| 0.36 | 1.01 | 1.02 → 1.02 | 1.03 | 1.007 → 0.848 | 0.977 → 0.803 | 1.03 → 1.06 |
| 0.45 | 1.02 | 1.02 → 1.02 | 1.05 | 0.988 → 0.815 | 0.952 → 0.742 | 1.04 → 1.10 |
| 0.5 | 1.02 | 1.02 → 1.03 | 1.07 | 0.965 → 0.803 | 0.920 → 0.714 | 1.05 → 1.12 |
| 0.6 | 1.02 | 1.01 → 1.06 | 1.10 | 0.899 → 0.798 | 0.867 → 0.643 | 1.04 → 1.24 |
| 0.7 | 1.03 | 1.02 → 1.07 | 1.16 | 0.827 → 0.807 | 0.749 → 0.601 | 1.10 → 1.34 |
| 0.85 | 1.05 | 1.04 → 1.13 | 1.26 | 0.742 → 0.833 | 0.629 → 0.548 | 1.18 → 1.52 |
| 1 | 1.07 | 1.10 → 1.22 | 1.37 | 0.685 → 0.858 | 0.528 → 0.501 | 1.30 → 1.71 |

- **The residue is all Octane's rt lobe.** The plain rt sphere at U5 /
  U6's α (0.6 / 0.7) is 1.10 / 1.16 × its model on the disk, 1.24 / 1.34 at
  the rim — U5 / U6's 1.12 / 1.17 and 1.22 / 1.41 × Iray with the falloff
  that puts the model on Iray. Neither the falloff nor the Iray side holds
  any of it.
- Transmission alone (specular 0) follows the model within 1 – 3 % up to α
  0.7 (rim 1.07), 1.05 / 1.07 at 0.85 / 1.
- With the reflection on, Octane keeps *more* than without it (α 1: 0.758
  v 0.640; the model has rt below transmit, 0.553 v 0.597) and its rim
  *rises* (0.685 → 0.858) where the model's falls.

## 2. One reflection event

Rings Octane (model), centre → 0.8 – 0.9:

| α | `refl` (η 1.5) | `inv` (η 1 / 1.5) | `ior50` |
|---|---|---|---|
| 0.25 | 0.041 → 0.081 (0.040 → 0.079) | 0.041 → 0.911 (0.041 → 0.895) | 0.925 → 0.868 (0.923 → 0.864) |
| 0.5 | 0.038 → 0.073 (0.039 → 0.059) | 0.061 → 0.649 (0.060 → 0.616) | 0.872 → 0.780 (0.868 → 0.796) |
| 0.7 | 0.032 → 0.083 (0.031 → 0.049) | 0.097 → 0.548 (0.073 → 0.473) | 0.696 → 0.737 (0.690 → 0.757) |
| 1 | 0.021 → 0.091 (0.020 → 0.039) | 0.125 → 0.449 (0.051 → 0.306) | 0.426 → 0.657 (0.422 → 0.683) |

- **Masking is the model's**: at F ≈ 0.92 flat (IOR 50) Octane is the
  single-scatter Smith-Beckmann lobe within 1 – 4 % at every α. No
  multiple-scattering energy, no height-correlated G (which changes the
  sphere model by < 1 % anyway, `--smith-g corr`).
- The dielectric's Fresnel is where it parts: outside (η 1.5) the centre is
  the model's, the rim up to 2.3 × (small in absolute terms, +0.05); of the
  inside kind (η 1 / 1.5) 1.05 – 1.09 at α 0.5, 1.2 – 1.35 at 0.7, 2.4 ×
  head-on at α 1. Octane's rough dielectric reflection keeps more than
  F(v · h) D G, the more the more facets reflect totally.
- Not a Fresnel at another cosine: F at (v·h)^p (n·v)^q (n·l)^s, p, q, s in
  0 – 1, searched over `refl` / `inv` / `ior50` — v·h (the model) scores
  as the best (error 4.7 v 4.4 for (v·h)(n·v)^0.25, the rest 7 – 12+);
  n·l, n·v, their mean or max are far off.
- Totally reflected samples that keep G1(v) but lose the shadowing G1(l)
  draw `inv` at α ≥ 0.6 within ~5 % but overshoot below (α 0.36 rim 0.808
  v 0.770); on the full spheres it puts `tr` within 3 % but leaves `rt` 0.78
  × Octane at α 1. A hint, not the mechanism.

## 3. Folding the measured event back in

Scratch model: the Smith sphere with every reflection event scaled by
Octane / model of the single-event probe at its cosine (`inv` inside,
`refl` outside, clamped past 0.9 R). Model / Octane, centre → 0.8 – 0.9:

| α | `tr` | `rt` |
|---|---|---|
| 0.25 | 0.99 → 1.00 | 0.99 → 0.99 |
| 0.5 | 0.98 → 1.02 | 0.96 → 0.95 |
| 0.7 | 0.99 → 1.03 | 0.94 → 0.88 |
| 1 | 0.95 → 0.93 | 0.86 → 0.75 |

The transmission sphere is explained by Octane's inside reflection; the rt
sphere only half: about half of its excess at α ≥ 0.6 sits in the
transmitting part of an rt event or in inside events past 0.9 R's cosine,
which no single-event probe here reaches.

## Where it stands

Measured, nothing in the export changed. The U5 / U6 residue is Octane's
plain rough rt lobe over single-scatter Smith (§ 1); masking is Smith's,
the Fresnel weighting of rough dielectric reflection isn't (§ 2); half of
the rt excess is explained by the measured reflection event (§ 3). Octane's
rt keeps more than any single-scatter model — a physically lossless rough
dielectric would keep 1 in the white furnace, so it is not less plausible
than Iray's. Whether to fold a measured Octane table (rt sphere rings or
the single events by α) into the export's Octane model, Faithful only or
both Looks, is the user's call (#185, `question`).

## Process lessons

- Five Octane variants of one existing furnace export (`set=` re-pins, no
  DAZ, no Iray) split the question into lobe / masking / Fresnel in under
  20 minutes of rendering.
- An IOR below 1 on an opaque dielectric draws the inside events of a
  closed sphere from outside, one event at a time — the probe for what a
  sphere does past total reflection.
