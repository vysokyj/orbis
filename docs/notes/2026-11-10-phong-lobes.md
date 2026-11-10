# 2026-11-10 — The Phong lobes off the fitted GGX curve (#95)

Question (#95, after #92): every Uber glossy lobe (gloss, dual lobe, Top
Coat) and the NVIDIA dielectrics' gloss is `simple_glossy_bsdf`, Phong of
exponent 2 / r² = Beckmann at α = r / √(1 + r²), Octane's pin √α. The
export still put them on min(r + 1.85 r², √r) under GGX energy preserving
— fitted to highlight peaks (2026-10-31 note), ≈ r for a sharp lobe (α ≈
r²). Decompose shape and brightness before changing anything.

Checked first: `choose_bsdf`'s `df_mode` is never set (no `.mi` fixture
has it): every Uber lobe is `dfm_glossy` = `simple_glossy_bsdf`, the dual
lobe's two included. The eight mapped NVIDIA dielectrics are
`simple_glossy_bsdf` too (only `core_definitions` uses GGX); PBRSkin is
`microfacet_ggx_smith_bsdf` (its own rule, untouched).

Measure (scratch `shape.py`, #92's `detail.py` per sphere): linear
luminance (gamma 2.2), disk mean (r < 0.75 of the sphere), band-passed
detail over r < 0.8 (box r 2 − r 8, std / mean) and its correlation with
Iray's band, p99 / median. Octane enhanced-final, 1024 samples. Variants
from a scratch build: `ORBIS_X_GLOSS=<brdf>` put every gloss on the
derived pin under that `brdf` (not committed).

## Shape: Beckmann at the derived pin

Calib-Glossy (G2 – G4 r 0.09, G6 0.01, G7 0.36), Octane / Iray, detail /
correlation:

| | G2 | G3 | G4 | G6 | G7 |
|---|---|---|---|---|---|
| GGX curve (master) | 1.31 / 0.93 | 1.75 / 0.91 | 0.92 / 0.95 | 0.91 / 1.00 | 1.06 / 0.91 |
| Beckmann, derived pin | 0.98 / 0.999 | 0.99 / 0.994 | 0.95 / 0.997 | 0.92 / 1.00 | 0.85 / **−0.25** |
| GGX EP, derived pin | 0.75 / 0.998 | 0.75 / 0.993 | 0.78 / 0.997 | 0.94 / 1.00 | 1.23 / 0.90 |
| plain GGX, derived pin | 0.77 / 0.997 | 0.77 / 0.992 | 0.78 / 0.996 | 0.94 / 1.00 | 1.14 / 0.80 |

Calib-DualLobe2 (E1 – E3 lobe 1 r 0.09 + a smooth lobe 2, E4 lobe 2 r 0.81,
E6 lobe 2 only 0.81, E7 lobe 1 0.36, E8 unsquared 0.6), disk / detail /
correlation:

| | E1 | E3 | E4 | E6 | E7 | E8 |
|---|---|---|---|---|---|---|
| GGX curve | 1.02 / 1.19 / 0.97 | 1.07 / 1.25 / 0.96 | 1.01 / 1.92 / 0.92 | 0.90 / 0.83 / 0.88 | 1.08 / 1.06 / 0.94 | 1.02 / 0.74 / 0.90 |
| Beckmann | 1.10 / 0.99 / 1.00 | 1.11 / 0.65 / 0.98 | **1.29** / 0.84 / 1.00 | **1.54** / 2.13 / 0.26 | **1.24** / 0.86 / −0.27 | **1.39** / 4.0 / 0.28 |
| GGX EP | 1.08 / 0.82 / 1.00 | 1.07 / 0.53 / 0.99 | 1.08 / 0.82 / 1.00 | 1.06 / 0.73 / 0.92 | 1.09 / 1.24 / 0.92 | 1.09 / 0.81 / 0.79 |

(The scene's matte level: Calib-Glossy's G1 1.08.) Sharp lobes: Beckmann
has Iray's detail and correlation; GGX at the same α (same peak) blurs
them by its long tail. Rough lobes: Beckmann glows at the rim (E6, E7:
a white sheen on the lit side, grainy at 1024 samples), 1.24 – 1.54 ×.
Calib-NVIDIA porcelain (r 0.1): detail 0.75 → 0.88, correlation 0.93 →
0.99. Calib-CoatBump (coat r 0.0025): every variant within 0.01.

## Brightness: furnace probes (known input)

Calib-Glossy's export with the dome a uniform white (`gw.ocs`: the HDRI
texture → RGB 1), the floor's opacity 0, G1 a white Lambert sphere (the
sanity check: **1.000** of the background), G2 – G8 white metals at
`brdf` 1 / 2 / 6 and pins 0.05 – 1 (two renders per model, 256 samples).
Disk mean / background:

| pin | 0.4 | 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 1.0 |
|---|---|---|---|---|---|---|---|
| Beckmann (1) | 0.999 | 0.993 | 0.966 | 0.905 | 0.807 | 0.688 | 0.561 |
| GGX (2) | 0.956 | 0.896 | 0.803 | 0.685 | 0.557 | 0.436 | 0.331 |
| GGX EP (6) | 0.999 | 0.999 | 0.999 | 0.998 | 0.998 | 0.998 | 0.998 |

Single-scatter albedo, numerically (scratch `albedo.py`, F = 1, the same
disk average): Smith-Beckmann at α = pin² 0.964 / 0.823 at pin 0.6 / 0.8
— Octane's 0.966 / 0.807; per ring (ρ 0 – 0.96) at pin 0.8 Octane 0.810
0.805 0.810 0.831 0.857 against Smith 0.814 0.810 0.819 0.843 0.875. So
Octane's Beckmann is single-scatter Smith-Beckmann at α = pin² (the pin is
√α, confirmed for a rough lobe), GGX single-scatter Smith-GGX, GGX EP adds
all of multiple scattering. Iray's Phong with v-cavities masking loses
more than Smith: Beckmann / Iray albedo 1.00 to r 0.2, 1.01 at 0.3, 1.03
at 0.4, 1.06 at 0.5, 1.11 at 0.7, 1.17 at 1 — the rough metal excess
(Metal Brushed's 1.12, #96) is the shadowing term, which Octane doesn't
offer.

The same with a black dielectric (specular 1, IOR 1.5) and as Universal's
coating (identical numbers), and with a white Lambert base under it (never
above 1): Octane = the analytic Smith-Beckmann · F(v·h) per ring at every
pin (pin 0.8: 0.034 … 0.062 against 0.034 … 0.067). Nothing in a uniform
furnace explains E6's 1.54.

## Brightness: the dielectric under directional light

Decomposition of Calib-DualLobe2 (albedo black → gloss only; specular and
coating 0 → diffuse only): the diffuse is the same under every `brdf`
(1.000), the gloss alone differs — E6 0.0212 (Beckmann) against 0.0106
(GGX EP). Iray's gloss by subtraction (Iray − Octane's diffuse / 1.08),
Octane / Iray gloss: Beckmann 1.11 – 1.13 on the sharp spheres, 1.36 / 1.48
/ 1.68 / 2.08 on E7 / E4 / E8 / E6; GGX EP **1.03 – 1.10 on all eight**.

Then the analytic lobe under the HDRI itself (scratch `hdrint3.py`: the
Ruins HDRI read from the export, its orientation fitted on the mirror
sphere — correlation 0.94 in log —, BRDF × HDRI on a 64 × 128 grid,
normalised by the Lambert sphere), floor hidden, black dielectric:
Octane `brdf` 1 / analytic Smith-Beckmann · F(v·h) = 1.03 / 1.08 / 1.08 /
1.07 at pin 0.15 / 0.2 / 0.4 / 0.5, then **1.14 / 1.18 / 1.62 / 2.25 at
pin 0.6 / 0.7 / 0.8 / 1**. The white metals under the same HDRI stay on
their analytic lobe (0.91 – 1.04 at pin 0.6 – 1). So Octane's rough
*dielectric* Beckmann is off under directional (importance-sampled) light
and only there; where it begins lies between pin 0.5 and 0.6. The Octane
manual says as much in passing (Beckmann not energy-bounded, GGX the
better choice for very rough surfaces).

STD (`brdf` 7) on the derived pin, tried as a Beckmann-like but energy
preserving model: sharp lobes as Beckmann (detail 0.92, correlation
0.999), E7's correlation the best (0.985), but rough lobes 1.21 – 1.28;
`spread` 0.001 / 0.5 / 1 changed nothing. Dropped.

## The rule

- Every Phong lobe at **`phong_pin`** = √(r / √(1 + r²)) — the gloss,
  the dual lobe's two, the Top Coat (Universal's coating, the Layered
  material's Specular Layer, the weighted coat), the weighted gloss, the
  NVIDIA dielectrics, rough glass (was √r: 0.6 against 0.58 at r 0.36).
  `ggx_roughness` is gone from these.
- `brdf` (`phong_brdf`, one per node): **Beckmann**, unless a dielectric
  lobe on the node has α > **0.25** (pin 0.5, the last pin the HDRI probe
  found faithful: `BECKMANN_DIELECTRIC_MAX_ALPHA`, a measured limit of
  Octane's model, not a fit to Iray) — then GGX energy preserving at the
  same pin. Metals Beckmann at any roughness. Glass Beckmann at any
  roughness (#72's evidence; the limit is about reflection).
- Lobe 2 of the Uber dual lobe at Iray's r2² — `DUAL_LOBE_2_ROUGHNESS`
  (2 · r2², #26) had compensated the curve's sharpness: with it the
  lobe-2-only E5 (r2 0.3) came out at detail 0.42, correlation 0.78.
- Anisotropy on the α's (`anisotropic_roughness`, as `mdl_anisotropy` in
  #92) — except a lobe whose long axis runs past 1 (`Anisotropy::
  saturates`): Iray's lobe at r / 0.0001 is degenerate; on the derived pin
  under Beckmann Calib-Aniso2's sharp metal K3 went 1.13 → 1.73 × Iray, the
  weighted coat K4 0.97 → 1.41, Calib-Aniso's A3 detail 1.97 → 2.35. It
  keeps #23's GGX curve under plain GGX (`saturated_aniso_pin`, the only
  place the curve stays; Octane's anisotropy semantics are #96).
- Flakes keep GGX EP: `FLAKE_ROUGHNESS` stands for the flakes' normals and
  was fitted under it.
- Architectural keeps its pins (`MdlDielectric::fitted_pins`):
  `arch_reflection_rough` (√(r · r₀)) was found by sweeping the GGX
  curve's pin, `arch_glass_rough` on Beckmann at √r. At the Phong pins
  Calib-NVIDIA3's glossiness spheres went from detail / correlation 0.90 –
  1.35 / 0.92 – 0.999 to 0.46 – 1.08 / 0.85 – 0.95, disk up to 1.26.
- The thin-walled refraction's Top Coat at its own pin (#20 had it at least
  the gloss's): see the eye below.

## Result

Octane / Iray, master → this commit, disk / detail / correlation (the
committed rule unless marked):

| scene, sphere | master | #95 |
|---|---|---|
| Glossy G2 (r 0.09) | 1.05 / 1.31 / 0.93 | 1.13 / 0.98 / 0.999 |
| Glossy G3 | 1.12 / 1.75 / 0.91 | 1.14 / 0.99 / 0.994 |
| Glossy G7 (r 0.36, GGX EP) | 1.07 / 1.06 / 0.91 | 1.09 / 1.23 / 0.90 |
| Aniso A1 (iso, r 0.16) | 1.09 / 3.34 / 0.69 | 1.09 / 1.02 / 0.996 |
| Aniso A2 (a 0.5) | 1.09 / 1.56 / 0.87 | 1.08 / 0.98 / 0.97 |
| Aniso2 K7 (a 0.25) | 1.12 / 2.70 / 0.86 | 1.12 / 1.24 / 0.98 |
| SpecMode S1 | 1.07 / 1.22 / 0.93 | 1.09 / 1.01 / 0.999 |
| Map-SpecMode S2 | 1.10 / 1.50 / 0.92 | 1.14 / 0.99 / 0.999 |
| Mixing M4 | 1.16 / 2.16 / 0.56 | 1.16 / 1.01 / 0.97 |
| Map-Metal (8 spheres) | 0.97 – 1.08 / 0.96 – 1.41 / 0.82 – 0.95 | 1.07 – 1.13 / 0.94 – 0.99 / 0.995 – 1.00 |
| Map-TopCoat (8) | 1.03 – 1.17 / 0.83 – 1.11 / 0.96 – 0.98 | 1.07 – 1.21 / 0.91 – 1.06 / 0.996 – 0.999 |
| NVIDIA porcelain | 1.10 / 0.75 / 0.93 | 1.11 / 0.88 / 0.99 |

Calib-DualLobe (lobe 1 r 0.09, lobe 2 at r2 0.6 / 0.1 / …), master → #95:
disk 0.84 – 1.06 → 1.06 – 1.12, detail 1.09 – 2.39 (D5 0.39) → 0.80 –
1.06, correlation 0.55 – 0.99 → 0.93 – 0.999, p99 / median 6.2 / 3.9 /
2.7 / 7.4 / 2.9 / 5.4 / 7.6 / 4.0 → 13.5 / 6.8 / 4.8 / 11.5 / 4.9 / 8.3 /
13.0 / 6.1 against Iray's 13.4 / 8.4 / 5.6 / 11.1 / 5.0 / 9.2 / 14.0 / 5.7.
Calib-DualLobe2 E1 – E5 correlation 0.92 – 0.99 → 0.997 – 1.00, detail
1.16 – 1.92 → 0.80 – 1.09; E6 – E8 (rough, GGX EP) 1.10 / 1.09 / 1.09
disk, as the variant above.

Unchanged (±0.01): Calib-CoatBump, Calib-TopCoat, Calib-NVIDIA's metals,
concrete, granite, resin, arch. The sharp spheres' disks rose 2 – 9 %
(Map-Metal 1.00 → 1.11, Map-TopCoat 1.03 → 1.12): the old curve's narrow
lobe reflected less of the bright sky; the matte floor of these scenes is
1.04 – 1.08.

Checked for regressions, `project.ocs` against the installed (#92) binary:
Calib-Skin Zurie / Monique / Victoria front, Calib-PBRSkin — every OpenPBR
skin node byte-identical (`phong_pin` is `openpbr_roughness` renamed; the
dual lobe's 2 · r2² was divided out on the skin), only the Uber props
around them (floor, eye socket, tear film, lashes) change. Calib-Hair,
-OmniHair-blonde: the strands (Hair 147) unchanged. The Uber card hair
(#52's Morgan): Calib-SpecMode's Morgan spheres disk 1.01 / 1.02 → 1.02 /
1.03, detail 1.03 / 1.08 → 1.00 / 0.99, correlation 0.985 / 0.986 → 0.999
/ 0.998. The
NVIDIA metals: unchanged (#92's rule). Calib-NVIDIA2: its Iray reference
shows another camera (#97), Octane before / after alike by eye.
Calib-Aniso's metal at v 0.8 (A6, not saturated, Beckmann now): disk
1.12 → 1.16, detail 1.38 → 1.64 — #96.

The eye (Calib-Eye; 1221 s at 1024 samples, so at 256 for the variants):
on the derived pins with #20's rule (the cornea's coat at least the
gloss's pin, 0.26) the highlight became one broad glow over the pupil;
Iray shows a sharp highlight inside a soft halo — its sharp coat (0.0016)
over the glossy veil (r 0.07). With the coat at its own pin (0.04): the
same, as Iray. Medians Octane / Iray, pupil (disk r 40 px) / iris (ring
70 – 115 px) / sclera (a box): GGX curve 0.63 / 0.85 / 0.87, coat at the
gloss's pin 0.97 / 0.91 / 0.88, at its own **0.83 / 0.88 / 0.92**. The
old rule had made up for the curve: its sharp gloss (pin 0.079) left the
veil to the blurred coat.

## Follow-ups

- #96: the rough metal's excess is Smith vs v-cavities shadowing
  (albedo 1.03 – 1.17 at r 0.4 – 1, above), not the distribution; the
  saturated anisotropy's curve.

## Process

- The first furnace run came out on the export's HDRI, not white: the
  dome's texture is an image node; check the background before reading a
  furnace. A probe "with a known input" needs that input verified (the
  Lambert sphere at 1.000 did it).
- A uniform furnace can't see a bias that only shows under importance-
  sampled light; the HDRI integration could.
- Rendering times: the sphere scenes 71 – 80 s at 1024 samples, furnace
  renders ~40 s at 256, Calib-Eye 1221 s at 1024 (don't batch it).
