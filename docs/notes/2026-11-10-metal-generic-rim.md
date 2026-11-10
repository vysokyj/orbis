# 2026-11-10 — Iray's rim on NVIDIA's `metal_generic`: read, two variants (#111)

Question (#111, after #98): Iray darkens NVIDIA's `metal_generic` (Metal
Brushed, Cast Iron) towards the rim as about √cos θ_v — every map flat,
bump off, under a uniform dome — while the Uber metal with the same
`simple_glossy_bsdf` stays flat. Find the term in the MDL by reading;
Iray variants only for what the reading leaves open.

Measures: sphere / background in linear luminance per ring of r / R
(rings 0 – 0.3, 0.3 – 0.5, 0.5 – 0.65, 0.65 – 0.75, 0.75 – 0.85,
0.85 – 0.92), the #98 furnace layout. Iray Server 5000 samples.

## 1. Reading

The chain DAZ hands Iray (Calib-NVIDIA's `.mi`): `daz_3d::basic::
pass_material(v)` (returns `v` unchanged) → `metal_brushed` / `cast_iron`
→ `metal_textured_anisotropic_uv` / `metal_textured` (maps → `file_texture`
mono, `base_weight` 0) → `metal_generic`:
`weighted_layer(metal_weight, simple_glossy_bsdf(tint, u, v), normal: bump,
base: weighted_layer(base_weight, diffuse, normal: bump))`, with `bump` a
`base::file_bump_texture` (or `tangent_space_normal_texture`) call.

None of the candidates is there: no Fresnel, `custom_curve_layer`,
`measured_curve_factor`, `directional_factor`, thin film, clearcoat or
normalisation; `weighted_layer` is linear over a black base;
`file_bump_texture` returns `normal` at factor 0 (MDL SDK `base.mdl`).

Against the flat Uber metal (`custom_curve_layer(1, 1, w,
simple_glossy_bsdf(white, tangent_u: anisotropy_conversion's), normal:
state::normal())`) and the flat Uber Weighted coat (Calib-TopCoat T3 / T4),
what differs:

1. the glossy's layer normal is a `file_bump_texture` call (`noinline`;
   Iray swaps in a builtin implementation, release notes), the Uber's is
   `state::normal()`. NVIDIA's dielectrics (concrete polished, ...) bump
   only their diffuse base, their glossy sits on `fresnel_layer` without
   a normal — and they are flat. Of Calib-NVIDIA's materials only
   `metal_generic` has its glossy under such a normal.
2. the lobe's tint: 0.65 / 0.543 on NVIDIA's metals, white on the Uber
   metals (their colour goes through the curve). #98's B7 / B8 varied
   only the weight.
3. the glossy's `tangent_u`: default `state::texture_tangent_u(0)` against
   the Uber's — unlikely, Cast Iron is isotropic.
4. the nested empty `weighted_layer` base, the coordinate transform.

1 and 2 are reachable through DAZ's channels, so two variants.

## 2. Calib-Furnace-MetalBump (bumped layer normal)

Calib-Furnace-Metal's white Uber metals; F5 – F8 with a flat grey bump
map (every texel 128) at Bump Strength 1: a zero gradient, the normal
unchanged, but the layers' normal a `file_bump_texture` call. Iray 62 s.

| | 0 – 0.3 | 0.3 – 0.5 | 0.5 – 0.65 | 0.65 – 0.75 | 0.75 – 0.85 | 0.85 – 0.92 |
|---|---|---|---|---|---|---|
| r 0.2 | 1.000 | 0.998 | 0.983 | 0.977 | 0.969 | 0.950 |
| r 0.2 flat bump | 1.000 | 1.000 | 0.984 | 0.981 | 0.972 | 0.950 |
| r 0.4 | 0.939 | 0.927 | 0.913 | 0.907 | 0.892 | 0.898 |
| r 0.4 flat bump | 0.939 | 0.927 | 0.915 | 0.908 | 0.895 | 0.894 |

r 0.05 / 0.1 alike (≥ 0.99). The bumped layer normal is not the rim. (Not
checked through a `.mi` that DAZ hands the flat map on.)

## 3. Calib-Furnace-CoatTint (the lobe's tint)

The Uber's Weighted top coat, `weighted_layer(w, simple_glossy_bsdf(tint:
Top Coat Color))`, over a black base (diffuse 0, Glossy Layered Weight 0).
Iray 96 s.

| | r 0.1 | r 0.4 |
|---|---|---|
| colour 1, w 1 | 1.000 0.997 0.964 0.947 0.979 0.991 | 0.938 0.921 0.890 0.859 0.834 0.834 |
| colour 0.65, w 1 | 0.407 0.407 0.401 0.394 0.401 0.404 | 0.385 0.380 0.373 0.368 0.360 0.360 |
| colour 1, w 0.65 | 0.651 0.651 0.641 0.605 0.591 0.613 | 0.621 0.609 0.594 0.575 0.563 0.557 |
| colour 0.4, w 1 | 0.156 0.156 0.156 0.154 0.151 0.153 | 0.150 0.149 0.147 0.144 0.142 0.142 |

- A tinted lobe stays flat: colour 0.65 / colour 1 at r 0.4 goes 0.410 →
  0.432; √cos θ_v would be 0.66 at r 0.9.
- The mid-ring dips (colour 1 at r 0.1: 0.947) are the neighbours'
  reflections — dark spheres beside bright ones. A mixed furnace row
  isn't uniform for a sharp lobe; compare like neighbours (#98's rows
  were uniform: all-white or all-Brushed).
- Top Coat Color reaches the lobe as about colour² (0.65 → 0.407, 0.4 →
  0.156; the weight is linear: 0.651) — DAZ's colour decode, not this
  question.

## Result

Excluded: every map, the bump, the weight (#98), the bumped layer normal,
the tint. Left: the default `tangent_u`, the nested empty base, the
coordinate transform, the module itself — none reachable through DAZ's
channels; a hand-written MDL module is #112.

## Decision (the user's, 2026-11-10)

The rim stays Octane over Iray, the export unchanged (as #98 recorded);
the hand-written module goes to #112. Regressions: none possible.

## Process

- Read the full chain DAZ hands Iray (the `.mi` names `pass_material`
  first), not only the preset's module.
- One variant per reachable difference, each with its control in the same
  scene: one Iray minute each.
- A furnace row with mixed albedos isn't a furnace for its neighbours.
