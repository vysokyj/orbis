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

## 6. Resume: Calib-PBRSkin-Metal against Iray (afternoon)

Iray reference in e668bac (Iray Server, 5000). Octane: the current
export (after #178's 6621d9b, #162's 521b921), enhanced-final 400,
denoised pass; it renders the same as the morning's to 0.01. Bins as
above (r / R: core < 0.5, mid 0.5 - 0.8, rim 0.8 - 0.93; rim R / B per
channel; spread p99 / median over Iray's).

| sphere | core | mid | rim | rim R | rim B | spread |
|---|---|---|---|---|---|---|
| N1 dual lobe (frame) | 1.10 | 1.19 | 1.27 | 1.17 | 1.38 | 1.13 |
| N2 metal 0.5 | 1.10 | 1.19 | 1.23 | 1.14 | 1.35 | 1.18 |
| N3 metal 1 | 1.06 | 1.16 | 1.18 | 1.12 | 1.30 | 1.16 |
| N4 metal 0.5 ratio 0 | 1.07 | 1.26 | 1.25 | 1.13 | 1.41 | 1.35 |
| N5 grey metal 0.5 | 1.05 | 1.11 | 1.07 | 1.06 | 1.09 | 1.06 |
| N6 metal 0.5, lobe weight 0.5 | 1.07 | 1.22 | 1.26 | 1.15 | 1.44 | 1.27 |
| N7 metal 0.5, dual lobe off | 1.09 | 1.08 | 1.09 | 1.09 | (2.26) | 0.97 |
| N8 metal 0.5, lobe 1 r 0.6 | 1.20 | 1.28 | 1.31 | 1.24 | 1.40 | 1.03 |

(N7's rim B: Iray's blue there is near 0, the ratio of two tiny numbers.)

Against the batch's resume lines:

- **N2 / N3 at or under the frame** (1.23 / 1.18 against N1's 1.27):
  the metal mapping (both lobes, no coating, beside the base) holds. N5
  (grey) has no edge excess either, so the red metal's edge to white is
  not a separate component.
- **N4 against N2**: lobe 2's share of the metal is right (rim 1.25 /
  1.23, core 1.07 / 1.10); only the spread differs (single lobe 1 at
  Beckmann: 1.35).
- **N6 = N2** (Iray halves N6's core, Octane follows: 1.07 against 1.10):
  the metal ignoring Dual Lobe Specular Weight stands.
- **N7** (#178): at N2's core ratio (1.09 / 1.10) with no highlight in
  either renderer: Iray's metal share without the dual lobe is black, as
  the fix writes it.
- **N8** (metal lobe 1 at α 0.36 on STD by `metal_brdf`): core 1.20, the
  row's highest. Per node (`brdf<k>=2` touches every node of a Mix): the
  metal's lobe 1 STD -> GGX 1.15 / 1.32, spread 1.10; the base's
  dielectric GGX EP -> GGX 1.19 / 1.38; lobe 2 Beckmann -> GGX nothing.
  The MDL's own GGX on the metal lobe takes 0.05 of the core: for #186.
- **K8** (Calib-PBRSkin-Makeup, same export): rim 1.36 against K1's 1.27
  with Beckmann, 1.26 against 1.22 with GGX on every node (K7 1.18,
  spread K8 1.48 -> 1.19). What was left of #167's rim follows the lobes'
  distribution and the rim component below, not the metal or the makeup
  Mix.

### The "contrary" GGX numbers (for #186)

The morning's table had GGX (`brdf=2`) lift Calib-PBRSkin's dual-lobe rim
1.19 -> 1.27 and P6's spread 1.08 -> 1.39. Those spheres' lobe 1 is at
r 0.6 (α 0.36, past `BECKMANN_DIELECTRIC_MAX_ALPHA` 0.25), so the export
already drew it at GGX EP (`brdf` 6, since before 449ddda), not Beckmann:
the comparison was GGX EP -> plain GGX. All three on the same export:

| Calib-PBRSkin | Beckmann (`brdf6=1`) | GGX EP (export) | GGX (`brdf6=2`) |
|---|---|---|---|
| P2 dual lobe 0.25: rim / spread | 1.47 / 1.39 | 1.19 / 0.97 | 1.26 / 1.10 |
| P3 dual lobe 0.5 | 1.56 / 1.31 | 1.23 / 0.99 | 1.31 / 1.10 |
| P4 dual lobe w0.25 | 1.34 / 1.44 | 1.14 / 1.00 | 1.18 / 1.10 |
| P6 Gladio cloth | 2.17 / 2.50 | 1.24 / 1.07 | 1.42 / 1.40 |

(P1 matte: core 1.07, rim 1.09 in every column - the frame's diffuse.)
Against Beckmann, plain GGX lowers rim and spread everywhere, on the
r 0.4 lobes too (N1 / K1 rim 1.27 -> 1.21 / 1.22, spread 1.13 -> 1.00).
Only GGX EP is lower still at α 0.36. P6's larger numbers are the same
excess on a darker base (translucency halves its diffuse; Iray core
0.0108 against P2's 0.0231), so the specular holds a bigger share of
both its rim and its p99.

What none of the models removes: every dual-lobe sphere's rim B (where a
red base leaves almost only the specular) is 1.30 - 1.45 × Iray, P2 with
GGX 1.44, N1 1.30, against the matte 1.09. That is the dielectric's
grazing level, not the distribution: `custom_curve_layer` (F0 to 1,
exponent 5, on the macro normal) against Universal's Fresnel. GGX EP's
lower rim at α 0.36 hides part of it; #186 should split it out before
choosing GGX vs GGX EP for the rough lobes (#188).

### Faithful

The faithful-final export (STD for the rough dielectric) on the same
scene: the row as in Enhanced but N8 core / rim 1.26 / 1.40.

Timings: 10 Octane renders 34 - 38 s each at 400 samples.
