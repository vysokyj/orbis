# 2026-11-16 PBRSkin metal: both lobes, no coating over it (#167)

Issue #167 (found in #142): on Calib-PBRSkin-Makeup the metal spheres K7 /
K8 (metal 0.5, makeup w 0.5, tinted / not) sat at their row's core ratio
but 1.30 / 1.44 × Iray at the rim, and their highlight looked sharper.
Enhanced-final, 400 samples, Octane's denoised pass; Iray references as
in `calib/references/iray` (5000). Bins of r / R per sphere from
`sphere_discs` (core < 0.5, mid 0.5 - 0.8, rim 0.8 - 0.93), linear
luminance; spread = (p99 / median) Octane over Iray's, same mask as
`highlight-spread.py`.

## 1. What Iray does (`pbr_skin.mdl`)

The metal is `weighted_layer(metallic, directional_factor(normal_tint =
base colour, grazing 1, exponent 5, base: dual_specular_bsdf),
base: diffuse_specular_bsdf)`. `dual_specular_bsdf` is
`weighted_layer(roughness2_weight, GGX(r2²), GGX(r²))`: lobe 2 at the
ratio (0.15) over lobe 1, both metal. The dielectric `custom_curve_layer`
(the dual lobe at its weight and F0) is the metal's *base*, not over it.
The `.mi` (Calib-PBRSkin-Makeup K8) confirms the arguments:
`roughness2_weight` 0.15, `roughness` 0.4, `roughness2` 0.4,
`specularlevel` 0.04. The metal also never sees Dual Lobe Specular
Weight, and with the dual lobe off it has no lobes at all (#178).

## 2. What the export did

One Universal at metallic w: the metal at lobe 1's roughness only, lobe 2
as the dielectric `coating` (0.15, F0 0.04) over the dielectric *and* the
metal. (Makeup's Mix sides alike: K8's makeup side a metal-1 node with the
coating.)

## 3. Decomposition (Octane variants of the same export)

`var.py` (scratch) edits every Universal of a material, nested Mix parts
included - `orbx-variants.py`'s `brdf=` / `set=` touch only the first node
of a Mix (noted for whoever uses it on Mix materials).

Rim (Y), Octane / Iray:

| sphere | export | coating 0 on metal parts | metal holds both lobes | GGX (`brdf` 2) | both lobes + GGX |
|---|---|---|---|---|---|
| K1 dual lobe (frame) | 1.27 | 1.27 | 1.27 | 1.21 | 1.22 |
| K7 metal 0.5 tinted | 1.31 | 1.22 | 1.24 | 1.25 | 1.18 |
| K8 metal 0.5 | 1.44 | 1.35 | 1.36 | 1.34 | 1.26 |
| M1 metal 0.75 grey (MetalTL) | 1.08 | 1.07 | 1.07 | 1.06 | 1.05 |
| P7 metal 0.9 grey (PBRSkin) | 1.12 | 1.11 | 1.09 | 1.10 | 1.06 |
| P8 Gladio armour | 1.15 | 1.14 | 1.12 | 1.11 | 1.07 |

Core: K7 / K8 1.05 / 1.10 -> 1.02 / 1.06 with both lobes; M1 1.09 -> 1.06.
The non-metal spheres are the same to 0.01 in every column but GGX.

Spread, Octane / Iray:

| sphere | export | both lobes | GGX | both lobes + GGX |
|---|---|---|---|---|
| K1 | 1.14 | 1.14 | 1.00 | 1.00 |
| K7 | 1.42 | 1.38 | 1.15 | 1.15 |
| K8 | 1.55 | 1.52 | 1.24 | 1.16 |
| Calib-PBRSkin P2 / P6 | 1.00 / 1.08 | same | 1.10 / 1.39 | same |

Reading:

- The coating over the metal is ~0.09 of K7 / K8's rim; moving lobe 2 into
  the metal (the MDL's structure) takes that and a little more. Derived,
  no constant: shipped.
- The sharper highlight is the lobes' shape, not the metal: Beckmann at the
  Phong pin against PBRSkin's GGX. GGX brings the K row's spread to 1.00 -
  1.16, but Calib-PBRSkin's dual lobes' rim goes 1.19 -> 1.27 and P6's
  spread 1.08 -> 1.39. That is #175's question (GGX vs Iray's shape on the
  skin lobes) and #162's; not decided here.
- The opaque grey metals (M1, P7) have no rim excess at all (1.07 / 1.09).
  What is left on K8 (rim 1.36 against K1's 1.27; K7 1.24) is the red
  metal in the makeup's Mix; the K row has no red metal without makeup,
  so it can't be split further from the existing references.

## 4. Shipped

`pbr_metal_lobe2` / `pbr_metal_lobes` (src/material.rs): a PBRSkin metal
with lobe 2 on the coating (no Top Coat, no refraction, ratio and lobe
roughnesses unmapped) is a Mix by the Metallic Weight of its metal side and
its base (`translucent_metal_material`, now also opaque); the metal side a
Mix at the ratio of the metal at lobe 2's roughness and at lobe 1's, no
coating; all metal: the lobes' Mix alone. Renders after the change equal
the "both lobes" column (K7 / K8 rim 1.24 / 1.36, M1 1.07, P7 / P8 1.09 /
1.12).

## 5. Iray batch: Calib-PBRSkin-Metal

New (`make-calib-scenes.py`, dumped, Octane side rendered in
`~/Library/Caches/orbis/calib/c167/metal`, export and `__brdf=2`):
the Makeup row's red base without makeup - N1 dual lobe (frame), N2 metal
0.5, N3 metal 1, N4 metal 0.5 ratio 0 (lobe 2 off), N5 grey metal 0.5,
N6 dual lobe weight 0.5, N7 dual lobe off (#178), N8 lobe 1 r 0.6 (α 0.36,
the metal lobe on STD by `metal_brdf`).

## Process

- A Mix material's nested Universals need a variant tool that walks them
  all; `orbx-variants.py set=` stops at the first.
- Check the binary is rebuilt after `cargo test` before exporting: `cargo
  test --release` doesn't refresh `target/release/orbis` (one export came
  from the previous build).

Timings: 15 variant renders 30 - 60 s each at 400 samples; one headless
dump 8 s.
