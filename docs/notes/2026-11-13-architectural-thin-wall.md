# 2026-11-13 — Architectural glass's thin_walled (#147)

Found in #117: Calib-Furnace-ArchGlass2's H4 / H5 (NVIDIA architectural,
reflectivity 0, refr_gloss 0.2 / 0, `thin_walled` on) exported exactly
like the closed H2 / H3 — Universal with `specular` 0, no Thin Wall, the
closed sphere's k. Octane drew them as refracting solids (0.729 → 0.507
centre to rim), Iray as thin walls (H4 0.52 → 0.50, H5 a flat 0.386).
Enhanced-final, 1024 samples, denoised pass; Iray references from
#116's batch (Iray Server 5000). No new Iray.

## What the MDL says

`nvidia/architectural.mdl` (DAZ app bundle) passes `thin_walled` straight
to `material.thin_walled`; the transmit lobe (`simple_glossy_bsdf`,
`scatter_transmit`, tint `refr_color`) and the `fresnel_layer` /
`custom_curve_layer` over it are the same for both; the volume is the
absorption only, which a thin wall doesn't have (the export already
dropped it). MDL spec 13.1: a thin wall doesn't bend, its IOR still sets
the Fresnel. So the colour reaches a thin wall only as `refr_color` per
crossing — two per pass through the sphere.

The `.mi` fixtures (Calib-NVIDIA2 / -NVIDIA3 / -Mi-Arch) don't carry the
argument (none of their spheres is thin-walled); the conversion lives in
`src/material.rs` (`mdl_dielectric`), not `src/iray.rs`, and the Iray
render shows that DAZ hands it on (flat profiles).

## Decomposed

The rough-transmission factor k is solved on two sphere models
(`mdl_layer.rs`); neither had a thin wall. Added `Stack::thin`:

- `libbsdf`: the transmission is the reflection mirrored through the
  surface (as the numpy port in `furnace-rings.py --thin`), masking
  min(G1, G2), no total reflection, the curve from outside on both faces.
- Octane: Thin Wall passes the ray straight on, unweighted ("no
  refraction and no roughness for transmission", #136).

`libbsdf` thin spheres (white, at the export's r_t = 3.7 × the formula):

| refr_gloss | r_t | model disk / all | rings | Iray | Iray / model |
|---|---|---|---|---|---|
| 0.2 | 0.497 | 0.823 / 0.830 | 0.81 … 0.83 flat | 0.505 flat | 0.61 at every ring |
| 0 | 1 (capped) | 0.474 / 0.534 | 0.43 → 0.58 rising | 0.386 flat | 0.90 → 0.67 |

At the MDL's own roughness (0.134 / 0.354) the model keeps 1.00 / 0.93 →
0.88; Iray / model 0.505 flat and 0.41 → 0.44. The residual is the same
kind as #117's component B (NVIDIA's 1.0 module loses more than `libbsdf`
at every event) — still measured, not derived.

Octane's side was checked by the render itself: H4 / H5 with Thin Wall
keep k² exactly (0.514 / 0.390 flat to the rim, transmission 0.717 /
0.624), so `specular` 0 + Thin Wall takes no Fresnel from the base IOR.

## The rule

`ARCH_GLASS_THIN_LOBE` = the thin spheres' measured albedo (0.386 / 0.505
at refr_gloss 0 / 0.2), the closed spheres' values above 0.2 until
measured (#149). τ is solved on the whole silhouette for a thin wall, not
the inner disk: Iray's thin profile is flat to the rim, and at refr_gloss
0 the model's rising rim would leave the whole sphere 1.13 × Iray with τ
solved on the disk (computed: 0.534 × 0.386 / 0.474 = 0.435 instead of
0.386; the first try).

## Result

Octane / Iray, enhanced-final, denoised (dome 0.1734 v 0.1701):

| sphere | before disk | after disk | after rings centre → 0.8 – 0.9 |
|---|---|---|---|
| H4 refr_gloss 0.2 thin | 1.27 (0.729 → 0.507) | 1.02 | 0.99 1.01 1.01 1.01 1.01 1.02 1.03 1.03 1.03 |
| H5 refr_gloss 0 thin | 1.34 (0.520 → 0.429) | 1.01 | 1.01 … 1.01 0.99 |
| H1 – H3, H6 – H8 (closed) | | unchanged | |

A rough thin wall stays sharp in Octane (#136); in the furnace that is
invisible, in a scene the view through it is unblurred.

## #148

The closed lobe's τ is solved at the material's IOR on tables measured at
1.5 (#148). A thin wall's lobe never refracts, so its τ doesn't depend on
the IOR; only the layer curve's Fresnel does, and that is `libbsdf`'s.

## Process lessons

- The Octane Smith model first killed thin-wall transmissions whose
  sampled facet faced away (0.92 instead of 1): a straight-through wall
  must not depend on the facet at all.
- A flat measured profile means the disk stands for the whole sphere;
  check what the solve's disk assumption does to a model whose profile
  isn't flat.
