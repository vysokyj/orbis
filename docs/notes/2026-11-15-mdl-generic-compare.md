# 2026-11-15 — metal_generic's pieces at mdl 1.0: compare, third round parked (#112)

Resume of #112 with Calib-Furnace-MdlGeneric rendered (Iray, 5000
samples, c39209c). Question: which piece of NVIDIA's `metal_generic`,
combined with the `mdl 1.0` (legacy) lobe, draws Metal Brushed B6's rim
(rim / centre 0.65 against the bare 1.0 lobe's 0.89)? Measure:
`furnace-rings.py measure`, sphere / dome per ring of the silhouette,
denoised references as in git. Compared against Calib-Furnace-MdlVersion
M3 / M4 (bare white lobe r 0.4, 1.0 / 1.3) and Calib-Furnace-Brushed B6.

## 1. Calib-Furnace-MdlGeneric

| Sphere | 0 – 0.1 | 0.2 – 0.3 | 0.4 – 0.5 | 0.6 – 0.7 | 0.7 – 0.8 | 0.8 – 0.9 | rim / centre |
|---|---|---|---|---|---|---|---|
| G1 1.0 tint 0.65 r 0.4 | 0.594 | 0.593 | 0.575 | 0.554 | 0.541 | 0.531 | 0.894 |
| G2 1.3 tint 0.65 r 0.4 | 0.609 | 0.609 | 0.595 | 0.584 | 0.572 | 0.567 | 0.931 |
| G3 1.0 u 0.42 / v 0.35 | 0.924 | 0.924 | 0.906 | 0.875 | 0.851 | 0.828 | 0.896 |
| G4 1.3 u 0.42 / v 0.35 | 0.943 | 0.943 | 0.926 | 0.905 | 0.897 | 0.890 | 0.944 |
| G5 1.0 varying r 0.4 | 0.924 | 0.914 | 0.899 | 0.866 | 0.844 | 0.822 | 0.890 |
| G6 1.3 varying r 0.4 | 0.943 | 0.931 | 0.922 | 0.893 | 0.873 | 0.860 | 0.912 |
| G7 1.0 metal_generic at B6 | 0.364 | 0.364 | 0.361 | 0.347 | 0.335 | 0.324 | 0.890 |
| G8 1.3 metal_generic at B6 | 0.375 | 0.371 | 0.359 | 0.352 | 0.345 | 0.346 | 0.923 |
| M3 1.0 white r 0.4 | 0.924 | 0.914 | 0.899 | 0.867 | 0.846 | 0.825 | 0.893 |
| M4 1.3 white r 0.4 | 0.943 | 0.932 | 0.924 | 0.903 | 0.892 | 0.885 | 0.939 |
| B6 Metal Brushed, flat maps | 0.342 | 0.331 | 0.315 | 0.279 | 0.253 | 0.221 | 0.646 |

- **The tint scales**: G1 / M3 = 0.643 head-on and 0.644 at the rim (0.65
  tint), G2 / M4 0.646 / 0.641 — no shape.
- **The anisotropy is the isotropic lobe**: G3 = M3 within 0.003 per ring
  (u 0.42 / v 0.35 against r 0.4), G4 = M4.
- **A varying roughness changes nothing at 1.0**: G5 = M3 to 0.003. At
  1.3, G6 sits 2 – 3 % below M4 in the outer rings (0.860 v 0.885) — small,
  not the rim, and at the version NVIDIA doesn't use.
- **metal_generic's whole structure at B6's constants keeps the bare
  lobe's shape**: G7's rim / centre 0.890 = M3's 0.893; its centre 0.364 =
  0.609 · 0.65 · 0.924 (0.366): weight · tint · lobe, nothing else.
  G8 = 0.609 · 0.65 · M4 likewise.

So the Resume's outcome (3): the rim is outside `metal_generic` at
constants. B6 / G7 per ring:

| ring | 0.05 | 0.15 | 0.25 | 0.35 | 0.45 | 0.55 | 0.65 | 0.75 | 0.85 |
|---|---|---|---|---|---|---|---|---|---|
| B6 / G7 | 0.940 | 0.923 | 0.909 | 0.896 | 0.873 | 0.852 | 0.804 | 0.755 | 0.682 |
| ÷ √cos θ_v | 0.940 | 0.928 | 0.924 | 0.925 | 0.923 | 0.933 | 0.922 | 0.929 | 0.940 |

**B6 = 0.93 (± 0.01) · √cos θ_v × G7** in every ring — the same
√cos θ_v #98 found against the Uber metal (0.97 there: the Uber lobe is
the 1.3 one, ~2 % above 1.0), now with the version, the tint, the
anisotropy, the layers and the constants all accounted for.

## 2. What is left, and the third round

Ruled out by now: the sphere (same primitive, `pSphere 48_24`, in both
scenes), DAZ's brick wrapping (`make-mdl-scenes.py` writes NVIDIA's
two-brick graph, `DzCustomMDLBrick` → `DzMDLSurfaceBrick`; both `.mi`
wrap the call in `daz_3d::basic::pass_material`). What B6 has and G7
hasn't:

- the `base::file_texture` lookups (weight, roughness u / v) on the flat
  maps — values that are constants, read through a texture;
- a **valid bump texture**: G7's `file_bump_texture` had `texture_2d()`
  (invalid; Iray can fold it to the normal), B6's is a real image at
  Bump Strength 0. Iray has a builtin `base::file_bump_texture`, and its
  release notes speak of "normal clamping if bump/normal mapping is
  triggered" against darkening at grazing angles — a path a valid
  texture may open whatever the factor. (Calib-Furnace-MetalBump ruled
  out a bump on the Uber metal, 1.3-style lobe, not with the legacy one.)

Calib-Furnace-MdlTextured, own modules `calib/mdl/orbis_textured_v10.mdl`
/ `_v13`, G7 plus one piece at a time (all 1.0 unless said):

| Sphere | Material |
|---|---|
| H1 | G7 + valid flat bump texture (calib-disp-grey) at factor 0 |
| H2 | as H1 at factor 0.4 (metal_brushed's default; a flat map has no slope) |
| H3 | G7, weight through `file_texture` (grey, 0.5 + 0.5 L) |
| H4 | G7, roughness u / v through `file_texture` (white 0.02 + 0.4 L, black 0.35 + 0.325 L) |
| H5 | H1 + H3 + H4: metal_brushed's path written out |
| H6 | `nvidia::metal_examples::metal_brushed` itself at B6's inputs, through our brick |
| H7 | H1 at mdl 1.3 |
| H8 | bare white r 0.4 lobe (M3) under H1's bump |

The flat maps are B6's own (`Runtime/Textures/ORBIS/calib-disp-*.png`,
make-calib-scenes.py's), copied next to the modules. Its `.mi` (headless,
`daz-export-mi.dsa`) resolves all eight to
`mdl::orbis::orbis_textured_v10/_v13::...`. H6 imports
`nvidia::metal_examples` (the module of Calib-NVIDIA*'s metals: #94).

Reading: H6 at B6's 0.65 confirms the setup (our brick = DAZ's preset);
H6 at 0.89 says the rim is in what DAZ's preset hands Iray beyond the
call (the `.mi` arguments: textures with `gamma auto`, `compression 2`),
not in the MDL. With H6 at 0.65: the first of H1 – H4 that falls towards
0.65 names the piece (H8 gives its shape on a white lobe, H7 whether it
needs the legacy lobe); only H5 = combination. Then the rim is a
documented Iray mechanism, not an unknown, and whether NVIDIA's metals
get it on the Octane side (a √cos θ_v factor on the metal's albedo; today
Octane over Iray, #111) is the user's call.

## Process

- First MDL compile failure of this series: `mdl 1.0` functions need a
  compound body (`{ return ...; }`, C115), and no user function may
  return a `bsdf` — write the shared structure as a parameterised
  material and the variants as `let { ... } in generic(...)`, as NVIDIA's
  presets do. `daz_log.py <scene>` shows the import errors; the
  orbisrender instance's own log has the compiler lines.
- A per-ring ratio against the nearest like-built control (B6 / G7)
  gave a cleaner law (0.93 · √cos θ_v, ± 0.01) than rim / centre alone.
