# 2026-11-05 — Bump resolution against Iray (#75)

Follow-up of [2026-11-04](2026-11-04-bump-resolution.md): the rule
`bump_power` (factor × 2 × 512 / width × UV scale / 0.542 m / tiles) came
from MDL's reference `base::file_bump_texture`, not from Iray's binary.
Now the Iray reference of Calib-BumpRes (5000 samples, Iray Server,
a676f4f) against Octane, variant enhanced-final, 1000 samples, ~60 s a
render.

## By eye

Iray: R1 (256 px) clearly the bumpiest, R3 (4096 px) almost smooth,
R2 / R4 / R5 alike, T1 / T2 / S1 finer but of R2's depth — the MDL law
(R1 4x, R3 ¼ of R2). Octane with the rule: the same picture, sphere by
sphere.

## Measured: a power ladder

Octane renders with every bump power × k (`BUMP_POWER` scaled, rebuilt,
re-exported; k = 0 / 0.5 / 0.71 / 1 / 1.41 / 2), each against Iray's same
sphere: band-passed images (blur 1.5 px − blur 6 px, RGB), residual over
the inner disc relative to Iray's band signal
(`scripts/dev/bump-ladder-fit.py`):

| k | R1 | R2 | R3 | R4 | R5 | T1 | T2 | S1 |
|---|---|---|---|---|---|---|---|---|
| 0 | 1.59 | 1.69 | 0.46 | 1.30 | 1.29 | 1.16 | 1.01 | 0.45 |
| 0.5 | 1.27 | 1.23 | 0.31 | 0.88 | 0.91 | 0.90 | 0.78 | 0.34 |
| 0.71 | 1.35 | **1.13** | 0.27 | **0.75** | 0.81 | 0.68 | 0.65 | 0.27 |
| 1 | **1.16** | 1.22 | **0.23** | 0.77 | **0.80** | **0.35** | 0.54 | **0.17** |
| 1.41 | 1.21 | 1.44 | 0.29 | 0.87 | 0.90 | 0.52 | **0.45** | 0.27 |
| 2 | 1.28 | 1.67 | 0.41 | 0.93 | 0.98 | 1.26 | 0.71 | 0.37 |

Other bands: 1 / 4 px gives the same minima (R5 0.71); 2 / 10 px puts
R2 and T2 at 1 too. So every sphere fits at k ≈ 1 (R2 / R4 between 0.71
and 1, T2 between 1 and 1.41 — the finest pattern on screen, Octane's
pixel-averaging shortfall from the 2026-11-04 note). Had Iray's bump not
followed the width, R1 and R3 would fit at k ¼ and 4 — off this ladder's
ends on opposite sides; both sit at 1. Likewise T1 / T2 / S1 at 1 rules
out a per-meter bump in Iray (they'd fit at ½).

- **Iray's native `file_bump_texture` = MDL's reference**: tilt ∝
  factor / map width, tiling- and size-free.
- **The absolute strength holds**: `BUMP_POWER` 2 at 512 px and UV scale
  0.542 m (#61's fit on Calib-CoatBump) carries over to 256–4096 px,
  tiles 2 / 4 and a half-size sphere. No change.
- R1 / R2's residuals stay above 1 at every k: their big dimples throw
  the highlights around chaotically, so the pixels don't line up; the
  minimum is still where it should be.

Calib-CoatBump with the rule: exports unchanged (powers 2.0007 / 4.0015
/ 6.0022, normal maps 1); its render against Iray by eye as in #61.

## Dead end

Band energy per sphere alone (no pairing with Iray's pixels; blur 1.5 /
3 / 6 px): not monotonic in k and the spheres don't agree among
themselves even in Octane (R2 / R4 / R5 at the same tilt 6.9 / 5.0 / 5.8)
— each sphere reflects a different part of the environment. As the
2026-11-04 note found for RMS against the smooth render.

## Process

The ladder took 6 renders × ~60 s plus exports; the source change
(`BUMP_POWER`) reverted afterwards. Fitting per sphere against Iray's own
pixels is what made the plain renders usable for bump strength.
