# 2026-11-16 — metal_generic's rim is the textured roughness (#112)

Resume of #112 with Calib-Furnace-MdlTextured rendered (Iray Server, 5000
samples, denoiser on@8, 70453f2). Question (third round): what has Metal
Brushed B6 on top of Calib-Furnace-MdlGeneric's G7 (`metal_generic`'s
whole structure at B6's constants, `mdl 1.0`), which keeps the bare 1.0
lobe's rim (rim / centre 0.89) where B6 falls to 0.65? Measure:
`furnace-rings.py measure`, sphere / dome per ring of the silhouette.

## 1. Calib-Furnace-MdlTextured

| Sphere | 0 – 0.1 | 0.2 – 0.3 | 0.4 – 0.5 | 0.6 – 0.7 | 0.7 – 0.8 | 0.8 – 0.9 | rim / centre |
|---|---|---|---|---|---|---|---|
| H1 G7 + valid flat bump, factor 0 | 0.364 | 0.362 | 0.355 | 0.339 | 0.331 | 0.327 | 0.898 |
| H2 as H1, factor 0.4 | 0.364 | 0.363 | 0.356 | 0.338 | 0.326 | 0.316 | 0.868 |
| H3 G7, weight via `file_texture` | 0.364 | 0.364 | 0.357 | 0.337 | 0.324 | 0.313 | 0.860 |
| **H4 G7, roughness u / v via `file_texture`** | 0.342 | 0.331 | 0.316 | 0.280 | 0.253 | 0.220 | **0.643** |
| H5 H1 + H3 + H4 | 0.342 | 0.331 | 0.316 | 0.280 | 0.252 | 0.220 | 0.643 |
| H6 NVIDIA's `metal_brushed` at B6's inputs, our brick | 0.342 | 0.331 | 0.315 | 0.280 | 0.253 | 0.222 | 0.649 |
| H7 H1 at mdl 1.3 | 0.375 | 0.373 | 0.360 | 0.351 | 0.342 | 0.342 | 0.912 |
| H8 bare white r 0.4 under H1's bump | 0.924 | 0.907 | 0.883 | 0.842 | 0.823 | 0.810 | 0.877 |
| G7 (MdlGeneric) | 0.364 | 0.364 | 0.361 | 0.347 | 0.335 | 0.324 | 0.890 |
| B6 Metal Brushed, flat maps (Calib-Furnace-Brushed) | 0.342 | 0.331 | 0.315 | 0.279 | 0.253 | 0.221 | 0.646 |

- **H6 = B6** to 0.001 per ring: our two-brick graph hands Iray what
  DAZ's preset does; the rim is in the MDL call, not in the preset's
  texture settings (Resume (a)).
- **H4 names it**: the roughness u / v read through `base::file_texture`
  on the flat white / black maps — the same values as G7's constants
  (0.02 + 0.4 · 1 = 0.42, 0.35 + 0.325 · 0 = 0.35; the maps are exactly
  255 / 0) — is B6 ring for ring. H5 (all of B6's pieces) = H4.
- A valid bump texture adds nothing (H1 = G7 within 0.01, the rim even
  a hair higher), its factor neither (H2: −3 % at the outermost ring);
  the weight through a texture neither (H3, −3 % outermost). H8 / M3:
  1.00 head-on, 0.98 at the rim — the bump on the white lobe is flat too.

H4 / G7 per ring:

| ring | 0.05 | 0.15 | 0.25 | 0.35 | 0.45 | 0.55 | 0.65 | 0.75 | 0.85 |
|---|---|---|---|---|---|---|---|---|---|
| H4 / G7 | 0.940 | 0.920 | 0.909 | 0.896 | 0.875 | 0.855 | 0.807 | 0.755 | 0.679 |
| ÷ √cos θ_v | 0.940 | 0.926 | 0.924 | 0.925 | 0.926 | 0.936 | 0.926 | 0.929 | 0.936 |

**A textured roughness = 0.93 (± 0.01) · √cos θ_v × the constant one**,
#98's law exactly (B6 / G7 0.93 ± 0.01).

It is not a different roughness: no constant 1.0 lobe (Calib-Furnace-
MdlLobe L1 – L8, r 0.05 – 1) has rim / centre below 0.88 — at r ≥ 0.7 the
lobe flattens and then rises to the rim — while H4's centre (0.342 / (0.609
· 0.65) = 0.864) would put it near r 0.5 (0.834, rim / centre 0.90). The
textured roughness changes the lobe's angular shape, not its width.

## 2. What is open, fourth round

G5 (`0.4 + 0.0 * u`) was foldable, so "a varying roughness changes
nothing" (2026-11-15) only says a folded constant is a constant. Open: is
it `base::file_texture`, any texture lookup, or any non-constant
roughness; does the 1.3 lobe get it (H7 tested only the bump at 1.3); is
it the roughness only; and above all, **does the Uber (1.7) do it with a
roughness map**? The Uber metal with a constant roughness stays flat
(Calib-Furnace-Metal), and every Uber spheres scene so far had constant
roughness — while most store materials have a Glossy Roughness map. The
.mi of the new Uber scene shows a mapped Glossy Roughness reaches Iray as
a function call (texture lookup → `square_float`), a constant as a value.

- **Calib-Furnace-MdlRoughTex** (own modules `calib/mdl/orbis_roughtex_v10`
  / `_v13`, bare white lobe r 0.4 — MdlVersion M3 / M4 are the constant
  twins): J1 / J2 roughness via `base::file_texture` at 1.0 / 1.3, J3 via a
  plain `tex::lookup_float`, J4 / J5 a roughness varying by 1e-4 over the
  UV square without a texture at 1.0 / 1.3, J6 the tint via
  `file_texture` at a constant roughness, J7 / J8 J1 at r 0.1 / 0.7
  (MdlLobe L2 / L7 the constant twins). `.mi`: all eight resolve to
  `orbis_roughtex_v1x::...`, no Uber fallback.
- **Calib-Furnace-UberRoughMap** (`make-calib-scenes.py`, DAZ's Uber): the
  white metal at r 0.1 / 0.4 / 0.7 and the dielectric gloss (Glossy
  Layered Weight 1, Reflectivity 0.5, black base) at r 0.4, each as a
  constant and as the same value × the flat white map. Dumped, Octane
  side rendered (enhanced-final, faithful-final): the export reads a flat
  map like its value, so Octane's twins are equal (section 3).

`make-calib-scenes.py`: the furnace's dome-and-floor block, written out
three times, is now `furnace_dome` / `dome_file` (the existing helper
moved up).

## 3. Octane side of Calib-Furnace-UberRoughMap

Dumped (`daz-dump.sh`), exported and rendered in enhanced-final and
faithful-final (denoised pass), `furnace-rings.py measure`; the twins are
equal to 1 % in every ring, as the export reads a flat map like its value:

| Pair (constant / map) | disk | 0 – 0.1 | 0.8 – 0.9 |
|---|---|---|---|
| metal r 0.1 (U1 / U2) | 1.004 / 1.001 | 1.004 / 1.003 | 1.004 / 1.003 |
| metal r 0.4 (U3 / U4) | 0.945 / 0.945 | 0.980 / 0.980 | 0.894 / 0.886 |
| metal r 0.7 (U5 / U6) | 0.789 / 0.784 | 0.784 / 0.784 | 0.827 / 0.804 |
| dielectric r 0.4 (U7 / U8) | 0.037 / 0.036 | 0.034 / 0.034 | 0.052 / 0.054 |

(enhanced-final; faithful-final the same within 0.001 on the metals, the
dielectric 0.043 / 0.042.) The baseline for the Iray pairs.
