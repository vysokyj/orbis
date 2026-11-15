# 2026-11-15 — Architectural's transmit roughness, g above r 0.4 (#117)

Resume of #117 with its second round rendered (Iray Server, 5000 samples,
c39209c): Calib-Furnace-MdlArch (`orbis_arch_v10` / `_v13`) and
-MdlLegacy (`orbis_legacy_v10` / `_v13`). Questions: why NVIDIA's
architectural glass reads ~3.7 × its formula's roughness (the `base::`
call, the layer structure, or neither), and the `mdl 1.0` lobe's per-event
loss g(r, cos) above r 0.4. Measure: `furnace-rings.py measure`; model:
`furnace-rings.py model` (`libbsdf`). Octane enhanced-final, denoised pass.

## 1. MdlArch: it is the call

| sphere | disk | centre → 0.8 – 0.9 | NVIDIA twin |
|---|---|---|---|
| A1 1.0 lobe, gtr(0.4) | 0.936 | 0.962 → 0.846 | G3 0.936, 0.962 → 0.846 |
| A2 1.0 lobe, gtr(0.2) | 0.609 | 0.717 → 0.432 | G4 0.609, 0.717 → 0.431 |
| A3 1.0 lobe, gtr(0) | 0.508 | 0.609 → 0.344 | G5 0.508, 0.609 → 0.345 |
| A4 1.3 lobe, gtr(0.2) | 0.773 | 0.834 → 0.652 | — |
| A5 1.0 structure, gtr(0.2) | 0.609 | 0.717 → 0.431 | = A2 |
| A6 1.0 structure, constant 0.13397 | 0.960 | 0.981 → 0.902 | — |
| A7 1.0 thin lobe, gtr(0.2) | 0.510 | 0.524 → 0.498 | H4 0.506, 0.521 → 0.497 |
| A8 1.0 thin structure, gtr(0.2) | 0.517 | 0.524 → 0.511 | = A7 |

- The bare lobe with `base::architectural_gloss_to_rough(refr_gloss)` is
  NVIDIA's architectural glass ring for ring (A1 – A3 = G3 – G5, A7 = H4).
- The module's layer structure adds nothing (A5 = A2, A8 = A7).
- The SDK formula's value written as a constant (0.134 at refr_gloss 0.2)
  in the same structure keeps 0.960, about the plain lobe at r 0.13 (T2
  0.963, T3 0.941); the call keeps 0.609. So Iray's compiled call returns
  more in a transmit lobe — and the formula in a reflection (#114: the
  same call straight into a reflect lobe, point-lit, reads r ≈ 0.33 at g 0,
  peak and width; #156: architectural's reflection at the formula's Phong
  pin within a pixel). Not the version either: A4 (1.3) sits among the
  1.3 / `libbsdf` closed spheres at r ≈ 0.6 as A2 does among the 1.0 ones.
- Why the call differs isn't documented. The release notes mention a
  removed "legacy mia material parameter matching" for thin-walled glossy
  transmission (an exponent × 7, Iray 2021.0) — the direction is the
  opposite (smoother) and A7 = A2's r, so not that. Measured, then.

**r_t read off the plain lobe at constant r** (closed, IOR 1.5: T1 – T4,
K1 – K4 disks 0.966 / 0.963 / 0.941 / 0.892 / 0.815 / 0.706 / 0.513 /
0.335 at r 0.05 / 0.1 / 0.2 / 0.3 / 0.4 / 0.5 / 0.7 / 1; thin K5, X5, X7,
K6, K7):

| refr_gloss | formula | r_t (closed) | r_t (thin) | former 3.7 × |
|---|---|---|---|---|
| 0 | 0.354 | 0.71 (A3 = K3's rings) | ~0.75 (H5 0.386) | 1.0 (capped) |
| 0.2 | 0.134 | 0.60 (A2) | 0.59 (A7) | 0.50 |
| 0.4 | 0.051 | 0.21 (A1) | ~0.2 (J4 = X5) | 0.19 |
| 0.6 | 0.020 | ≲ 0.13 (G2 0.964) | — | 0.07 |

The ratio to the formula isn't a constant (2.0 / 4.5 / 4.1), so no scale:
the rule is the table 0.72 / 0.60 / 0.21 at 0 / 0.2 / 0.4, linear between,
and above 0.4 the 0.4 point's ratio (4.13 ×) — there the lobe's own loss
(~2 % per event at any r > 0) can't resolve r, and Calib-NVIDIA3's look
had matched Octane at r 0.06 – 0.09 at 0.6 (4.13 × → 0.08). Both readings
are a lookup among twins of one lobe, not a fit to a render.

## 2. MdlLegacy: g above r 0.4

K8 (1.3, closed r 0.5): 0.836, 0.906 → 0.708 v `libbsdf` 0.843, 0.906 →
0.719 — 1.3 is `libbsdf` here too.

Per event from the thin walls, g = √(Iray 1.0 thin / `libbsdf` thin) per
ring (the ring's cosine at both walls):

| r | cos 0.999 | 0.937 | 0.835 | 0.661 | 0.527 |
|---|---|---|---|---|---|
| 0.1 (K5) | 0.990 | 0.981 | 0.980 | 0.981 | 0.977 |
| 0.2 (X5) | 0.972 | 0.972 | 0.974 | 0.968 | 0.965 |
| 0.4 (X7) | 0.949 | 0.935 | 0.929 | 0.920 | 0.912 |
| 0.6 (K6) | 0.836 | 0.834 | 0.823 | 0.803 | 0.795 |
| 1 (K7) | 0.671 | 0.665 | 0.646 | 0.629 | 0.618 |

**But g at every event doesn't predict the closed spheres above r 0.3**
(model `--loss` = this table in `ci`; in `co` the same to 0.005):

| closed r | Iray 1.0 disk | g every event | g outside events only | g on passing events only |
|---|---|---|---|---|
| 0.2 (T3) | 0.941 | 0.936 | 0.966 | 0.939 |
| 0.3 (K1) | 0.892 | 0.867 | — | — |
| 0.4 (T4) | 0.815 | 0.782 | 0.848 | 0.793 |
| 0.5 (K2) | 0.706 | 0.640 | 0.741 | 0.654 |
| 0.7 (K3) | 0.513 | 0.426 | 0.559 | 0.439 |
| 1 (K4) | 0.335 | 0.236 | 0.370 | 0.243 |

At r ≤ 0.2 (where the 2026-11-14 note checked it) all readings agree; at
r 1 every event gives 0.70 × Iray, the entry alone 1.10 ×. Also tried:
`libbsdf`'s masking as g1 · g2 at every event (`--mask prod`, the
separable form a legacy lobe might use) — changes nothing (thin r 1 0.465
v 0.474, closed 0.580 v 0.568). So above r 0.3 g is not one wall- and
IOR-independent function of (r, cos), or the `libbsdf` thin wall it was
read against is off at high r — Iray 1.3 thin was measured only to r 0.4
(X6 / X8). Note `libbsdf` itself keeps far less in a thin wall than in a
barely bent closed sphere (r 0.6: thin 0.739, closed IOR 1.05 0.983): the
thin wall's mirrored-reflection transmission loses to masking what a
refraction doesn't.

## 3. Octane at the new r_t

Calib-Furnace-ArchGlass / -2 / -3 re-exported (ca0f988 + this rule) and
rendered; Octane / Iray, disk (before: the previous renders at 3.7 ×,
2026-11-13 / -14):

| sphere | before | now | rings now (Octane centre → rim v Iray) |
|---|---|---|---|
| G3 refr_gloss 0.4 | 1.01 | 1.02 | 0.960 → 0.871 v 0.962 → 0.846 |
| G4 0.2 | 1.05 | 1.06 | 0.735 → 0.514 v 0.717 → 0.431 |
| G5 0 | 1.02 | 1.03 | 0.583 → 0.424 v 0.609 → 0.345 (was flat 0.527 → 0.436) |
| G7 0.2 under refl 0.6 | 1.03 | 1.03 | |
| G8 0.2, Fresnel term | 1.17 | 1.11 | |
| H6 0.2 IOR 1.2 | 1.01 | 1.06 | |
| H7 0.2 IOR 2 | 1.10 | 1.00 | |
| H4 / H5 thin | 1.02 / 1.01 | 1.02 / 1.01 | (Octane's thin wall doesn't blur) |
| J4 thin 0.4 | 0.99 | 0.99 | |

The disks stay where k puts them (it is solved per material); refr_gloss 0
now falls toward the rim like Iray instead of staying flat, the IOR pair
H6 / H7 goes 1.01 / 1.10 → 1.06 / 1.00, the Fresnel term drops 6 %.
The rims stay Octane's (G4 1.19 × at 0.8 – 0.9): the Falloff Mix solve
of the 2026-11-13 note still waits for g.

## 4. Third round (parked)

Calib-Furnace-MdlLegacy2 (`orbis_legacy2_v10` / `_v13`): 1.0 closed at
IOR 1.05 r 0.6 / 1 (L1 / L2: two barely bent events, no total
reflection — does a closed sphere then lose what a thin wall does?),
IOR 1.2 / 2 at r 0.6 (L3 / L4) and 2 at r 1 (L7) — g's IOR trend at one r
with K2 / K3 bracketing 1.5 — and 1.3 thin r 0.6 / 1 (L5 / L6) and closed
1.05 r 1 (L8): is `libbsdf`'s thin wall right there. `libbsdf` predicts
L1 0.983, L2 0.918, L3 0.912, L4 0.621, L5 0.739, L6 0.474, L7 0.378, L8
0.918 (disks). The `.mi` (headless) resolves all eight to
`mdl::orbis::orbis_legacy2_v10 / _v13::…`.

## Where it stands

Export changed: architectural's transmit roughness is the measured r_t
(`arch_glass_rough`, spec), `ARCH_GLASS_ROUGH` 3.7 gone as a fit. #117
stays open for g above r 0.3 (third round) and the Falloff Mix solve
after it. #168 (thin architectural under a Schlick layer) isn't settled
here: A8 has the structure at weight 0 only.

## Process lessons

- A constant-r twin of a call (A6) answered in one sphere what the
  roughness-sweep fits of #21 / #115 couldn't: when a value comes from a
  function, render the function's documented output as a literal beside
  it.
- The 2026-11-14 conclusion "g is IOR- and wall-independent" was checked
  only at r ≤ 0.2, where g ≈ 0.97 and every reading agrees. Check a
  derived rule where its effect is large before extending it.
- `calib-variants.sh` finds dumps by the scene's My Library path (the
  dump name hashes the path): pass scene names, not `calib/scenes` paths.
