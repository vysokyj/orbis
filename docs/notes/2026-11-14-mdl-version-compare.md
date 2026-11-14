# 2026-11-14 — The MDL version against metal_generic's rim: compare, second round parked (#112)

Resume of #112 with Calib-Furnace-MdlVersion / -MdlLobe / -MdlTransmit
rendered (Iray, 5000 samples, 91b1388). Question: does Iray's legacy
`simple_glossy_bsdf` (modules below `mdl 1.3`, NVIDIA's examples are 1.0)
make `metal_generic`'s rim fall (Metal Brushed B5 / B6: about 0.97 ·
√cos θ_v against the Uber metal with the same lobe, ≈ 0.65 of the centre
at r 0.85)? Measure: `furnace-rings.py measure`, sphere / dome per ring of
the silhouette (0 – 0.1 ... 0.8 – 0.9), denoised reference as in git.

## 1. Calib-Furnace-MdlVersion: the version alone

| Sphere | 0 – 0.1 | 0.2 – 0.3 | 0.4 – 0.5 | 0.6 – 0.7 | 0.7 – 0.8 | 0.8 – 0.9 |
|---|---|---|---|---|---|---|
| M1 1.0 reflect r 0.1 | 0.981 | 0.981 | 0.981 | 0.981 | 0.980 | 0.978 |
| M2 1.3 reflect r 0.1 | 1.000 | 1.000 | 0.999 | 0.989 | 0.988 | 0.992 |
| M3 1.0 reflect r 0.4 | 0.924 | 0.914 | 0.899 | 0.867 | 0.846 | 0.825 |
| M4 1.3 reflect r 0.4 | 0.943 | 0.932 | 0.924 | 0.903 | 0.892 | 0.885 |
| M5 1.0 layered r 0.1 | 0.981 | 0.981 | 0.981 | 0.978 | 0.973 | 0.971 |
| M6 1.3 layered r 0.1 | 1.000 | 1.000 | 1.000 | 0.997 | 0.991 | 0.985 |
| M7 1.0 transmit r 0.2 | 0.969 | 0.962 | 0.943 | 0.932 | 0.905 | 0.856 |
| M8 1.3 transmit r 0.2 | 1.000 | 1.000 | 0.999 | 0.980 | 0.956 | 0.917 |

- **1.3 is the Uber's lobe**: M4 = Calib-Furnace-Metal F5 (Uber r 0.4,
  1.7) to 0.001 in every ring, M2 = F2 within 1 %.
- **1.0 is a different lobe, but a small one**: 0.98 of 1.3 head-on at
  r 0.1 and flat to the rim; at r 0.4 0.98 head-on → 0.93 at 0.8 – 0.9.
  Rim / centre at r 0.4: 0.89 (1.0) v 0.94 (1.3) — Metal Brushed B6 (u
  0.42 / v 0.35) 0.65. The legacy lobe carries about a fifth of the rim
  (ln 0.93 v ln 0.70 against the Uber), not the rim.
- M5 = M1 (0.979 v 0.981 disk): `metal_generic`'s two weighted layers with
  an explicit layer normal add nothing — the nested empty base is out.

## 2. Calib-Furnace-MdlLobe: the legacy reflect lobe per roughness

1.0 lobe (L) / Uber 1.7 lobe (Calib-Furnace-Metal F) at the same r:

| r | 0.05 | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 | 0.7 | 1.0 |
|---|---|---|---|---|---|---|---|---|
| centre (0 – 0.1) | 0.987 | 0.981 | 0.981 | 0.981 | 0.980 | 0.979 | 0.911 | 0.825 |
| rim (0.8 – 0.9) | 0.981 | 0.978 | 0.979 | 0.957 | 0.924 | 0.868 | 0.764 | 0.652 |
| disk (< 0.75) | 0.981 | 0.980 | 0.973 | 0.976 | 0.967 | 0.941 | 0.870 | 0.775 |

A flat ~2 % below 1.3 up to r 0.3, then a loss that grows with roughness
and towards the rim. At r 1 the 1.7 lobe rises to the rim (0.497 → 0.727,
Phong with v-cavities), the 1.0 lobe barely (0.410 → 0.474). For NVIDIA's
metals (Brushed v 0.35 – 0.48, Cast Iron sharper) this is 2 – 6 % on the
disk — inside the Octane over Iray margin already accepted for the rough
lobe's energy (#98's 1.056). No export change from it.

## 3. Calib-Furnace-MdlTransmit: the closed refractor (for #117)

| r | 1.0 disk | 1.3 disk | 1.0 rim | 1.3 rim |
|---|---|---|---|---|
| 0.05 | 0.966 | 1.000 | 0.959 | 0.994 |
| 0.1 | 0.963 | 1.000 | 0.926 | 0.969 |
| 0.2 | 0.941 | 0.987 | 0.856 | 0.916 |
| 0.4 | 0.815 | 0.901 | 0.652 | 0.776 |

T3 / T7 equal M7 / M8 (same module material, two scenes). The 1.0
transmit lobe loses 3 – 10 % on the disk and more towards the rim than the
1.3 one: the version is part of #116's NVIDIA loss. Comparing the 1.3
rings with `libbsdf` (`furnace-rings.py model`) and the TIR hypothesis is
#117's resume (it shares these two scenes and adds Calib-Furnace-MdlTir /
-MdlResin); not done here.

## 4. What is left of metal_generic, and the second round

The Resume's outcomes were (1) the version is the rim, (3) M1 = M2 and
nothing left to test. Neither: the version is real and small, and three
pieces of `metal_generic` were never combined with a 1.0 lobe — the tint
(#111's Calib-Furnace-CoatTint tested a tinted lobe only at 1.7, flat),
the anisotropy with the default `tangent_u` (B6 is near-isotropic, the
rim's there; Cast Iron is isotropic), and a roughness that is an
expression (NVIDIA's comes from `base::file_texture`) rather than a
constant. One more scene splits them, each at 1.0 / 1.3:

| Sphere | Material (`calib/mdl/orbis_generic_v10.mdl` / `_v13`) |
|---|---|
| G1 / G2 | tint 0.65 (metal_brushed's metal_color), isotropic r 0.4 |
| G3 / G4 | white, u 0.42 / v 0.35 (B6), default tangent_u |
| G5 / G6 | white, r 0.4 as `0.4 + 0 · texture_coordinate` |
| G7 / G8 | `metal_generic`'s whole structure at B6's constants: weight 0.609, tint 0.65, u 0.42 / v 0.35, base weight 0, `file_bump_texture` normal (factor 0, tiling 3) |

Its `.mi` (headless, `daz-export-mi.dsa`) resolves all eight to
`mdl::orbis::orbis_generic_v10/_v13::...` under `pass_material`.
Reading the result against M3 / M4 and B6: a 1.0 sphere whose rim / centre
drops towards B6's 0.65 while its 1.3 twin stays at M4's 0.94 names the
piece. G7 = B6's shape (up to the ratio weight · tint) with G1 – G6 near
M3: the combination, not a single piece. G7 near M3 (0.89): the rim is
outside `metal_generic` at constants — the maps (`file_texture`'s lookups),
DAZ's brick wrapping or the sphere's UV; then the rim stays Octane over
Iray with that recorded.

## Process

- A rim that's a fifth explained isn't "ruled out": the outcome table in
  the batch comment had no row for "real but small" — write one next time.
- Twin spheres in one scene (M3 / M4 v F5 in another) agree to 0.001;
  cross-scene comparisons of the same furnace layout are safe.
