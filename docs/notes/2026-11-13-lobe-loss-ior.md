# 2026-11-13 — The measured lobe loss at the IOR it was measured at (#148)

Question (#148, from #117): `mdl_layer::tint_factor_measured` solved the
per-base-event tint τ so that the `libbsdf` lobe alone keeps the measured
`ARCH_GLASS_LOBE` / `RESIN_LOBE` value **at the material's IOR** — but the
tables were measured at refr_ior 1.5 (Calib-Furnace-ArchGlass G1 – G5) and
at resin's Coat IOR 1.49 (the preset's value; the MDL default too,
Calib-Furnace-Resin2 U1 – U6). `libbsdf`'s closed lobe itself loses more
with the IOR, so τ absorbed that and moved the factor the wrong way.

Scene: Calib-Furnace-ArchGlass2 (reflectivity 0, refr_gloss 0.2, r_t
0.496), Octane enhanced-final 1024 samples, denoised pass, disk r < 0.75
(`furnace-rings.py measure`). Iray reference in git (Iray Server 5000,
#116). No Iray this session.

## Fix

τ is solved on the lobe at the table's IOR (`lobe_ior`:
`ARCH_GLASS_LOBE_IOR` 1.5, `RESIN_LOBE_IOR` 1.49), then the material's
stack is traced with it. Thin walls keep τ at the material's IOR — their
lobe never bends, τ doesn't depend on it (#147).

Export's factor k (white, reflectivity 0):

| IOR | k, solved at the material's IOR | k, solved at 1.5 |
|---|---|---|
| 1.2 (H6) | 0.808 | 0.860 |
| 1.5 (H2) | 0.853 | 0.853 |
| 2.0 (H7) | 0.929 | 0.850 |

## Measured (disk, Octane / Iray)

| sphere | Iray | before | after |
|---|---|---|---|
| H2 IOR 1.5 | 0.607 | 0.639 (1.05) | 0.639 (1.05) |
| H6 IOR 1.2 | 0.713 | 0.633 (0.89) | 0.718 (1.01) |
| H7 IOR 2.0 | 0.461 | 0.610 (1.32) | 0.509 (1.10) |
| H4 thin 0.2 | 0.505 | 0.514 (1.02) | 0.514 (1.02) |
| H5 thin 0 | 0.386 | 0.390 (1.01) | 0.390 (1.01) |

## Decomposed: what is left

Octane / Iray = (Octane render / the Smith model at k) × (the `libbsdf`
model at τ 0.861 / Iray), `furnace-rings.py model`:

| sphere | Smith model | Octane / Smith | `libbsdf` × τ | model / Iray |
|---|---|---|---|---|
| H6 1.2 | 0.700 | 1.03 | 0.696 | 0.98 |
| H2 1.5 | 0.616 | 1.04 | 0.616 | 1.02 |
| H7 2.0 | 0.491 | 1.04 | 0.489 | 1.06 |

- Octane over its Smith model: flat in the IOR (the known ~4 % at α 0.5,
  #117's note) — not part of this issue.
- Iray under `libbsdf` × τ: 0.98 / 1.02 / 1.06 — Iray's own loss grows a
  little with the IOR beyond `libbsdf`'s. Two IORs (three points with
  1.5) don't say whether that is #117's total-reflection guess or another
  per-event law; no IOR curve fitted. Calib-Furnace-MdlTir (#117's batch:
  closed transmit r 0.2 at IOR 1.2 / 2.0, `mdl 1.0` v `1.3`) decides it.
- Resin: every reference is at 1.49, so its spheres don't change.

## Process lessons

- Before trusting a measured table, write down the conditions it was
  measured in (IOR, thin or closed) next to it — the table now carries its
  IOR as a constant.
