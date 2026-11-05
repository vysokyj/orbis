# 2026-11-05 — Diffuse Overlay Roughness: Iray's Oren-Nayar head-on (#76)

Goal: the Diffuse Overlay at Overlay Roughness 1 rendered 0.85 – 0.89 ×
Iray (Calib-Diffuse O8, Calib-Overlay V8) while the overlay at roughness
0 and every other sphere sat at the smooth level (1.04 – 1.08). The
export scaled the overlay's albedo share by the classic qualitative
Oren-Nayar A term, 1 − 0.5 σ² / (σ² + 0.33) (#32). Settle it from what
Iray's `diffuse_reflection_bsdf` is, not by fitting a constant.

Scenes: `Calib-Diffuse`, `Calib-Overlay` (exported from My Library,
cached dumps), Iray references at 5000 samples (2026-10-30 / 2026-11-05).
Variant enhanced-final, Octane 256 samples, 19 – 20 s per render. No new
Iray renders.

## What Iray's diffuse is

- DAZ's `irayubermaterial.mdl` (`diffuse_base`): base and overlay are
  both `df::diffuse_reflection_bsdf(roughness, tint)`, the overlay a
  `weighted_layer` at the overlay weight. DAZ's MDL is 1.7 and doesn't
  pass `multiscatter_tint` (0: no energy compensation).
- MDL spec (1.11): `diffuse_reflection_bsdf` is the "improved version of
  the qualitative Oren-Nayar model", citing Oren & Nayar and Fujii's
  "A tiny improvement of Oren-Nayar reflectance model". Iray release
  notes: MDL 1.10 / Iray 2024.1 switched it to Fujii's model ("reduced
  energy loss ... notable only for high roughness"). DAZ Studio 6's
  `libneuray` is 2025.0.3, so the references are Fujii's.
- Fujii's model: f = ρ (A + B s / t), s = l·v − (n·l)(n·v), t = 1 for
  s ≤ 0, max(n·l, n·v) above; A = 1 / (π + (π/2 − 2/3) σ), B = σ A.
- Seen head-on (n = v): s = 0, so the lobe returns π A of a Lambertian
  one whatever the lighting. π A = 1 / (1 + (1/2 − 2 / (3π)) σ): 0.874 at
  σ 0.5, 0.777 at 1. The classic A: 0.784 / 0.624.

Directional albedo under uniform light (numerical, ρ 1), centre / middle
60 % disc / whole disc of a sphere seen from afar:

| model | σ 0.5 | σ 1 |
|---|---|---|
| classic qualitative (max(0, cos φ)) | 0.784 / 0.816 / 0.855 | 0.624 / 0.664 / 0.712 |
| classic A, B with Fujii's s / t | 0.784 / 0.788 / 0.808 | 0.624 / 0.629 / 0.654 |
| Fujii (Iray) | 0.874 / 0.879 / 0.906 | 0.777 / 0.785 / 0.833 |

## Measuring Iray's overlay factor

Sphere centres from the edges along row 560 (radius 45 px), linear
(gamma 2.2; the scenes' tone mapping has burn 1, crush 0).

First try, ratios within one image (Iray O8 / O5 in G, the red base has
G ≈ 0): 0.555 on the middle disc — below A, nonsense. Octane's own O8 /
O5, which is exactly A (0.624) by construction, rendered 0.469: the
spheres sit at different places in a 6.5 mm wide-angle view under a
back-lighting HDRI, and identical Lambertian spheres differ by up to
25 % between positions. The same trap is in #32: Iray's σ 0.5 / 1
spheres at 0.78 / 0.62 of the smooth one's looked like A, but Octane's
Lambertian O3 / O1 (same albedo) was 0.86 there too.

Fix: a double ratio that cancels the position, Iray factor =
A × (O5 Octane / Iray) / (O8 Octane / Iray), per radial bin:

| bin (r / R) | 0–.3 | .3–.5 | .5–.6 | .6–.7 | .7–.8 | .8–.9 | disc 60 | disc 90 |
|---|---|---|---|---|---|---|---|---|
| O8 vs O5 (G) | 0.770 | 0.743 | 0.712 | 0.696 | 0.677 | 0.673 | 0.739 | 0.703 |
| O8 vs O7 (G) | 0.767 | 0.739 | 0.709 | 0.692 | 0.674 | 0.669 | 0.735 | 0.699 |
| O8 vs O5 (B) | 0.768 | 0.747 | 0.723 | 0.712 | 0.700 | 0.700 | 0.744 | 0.720 |

At the centre 0.77 = Fujii's π A (0.777), with σ as DAZ passes it.
Towards the rim it falls: the HDRI is behind the spheres, and Fujii's B
term (s < 0, t = 1) darkens forward scattering, where the classic
model's max(0, cos φ) never darkens.

Octane's Oren-Nayar (`diffuseBrdf` 2) at the centre, by the same double
ratio with Iray = π A: ~0.80 / 0.65 at σ 0.5 / 1 — the classic model's
A (0.784 / 0.624, a few % of B in the bin), rising toward the rim.

## Variants

Luminance Octane / Iray, centre (r < 0.3 R) / middle 60 % / 90 % disc.

| sphere | before (A) | **after (π A)** | O2 / O3 as Lambert × π A |
|---|---|---|---|
| O1 smooth | 1.08 / 1.08 / 1.09 | 1.08 / 1.08 / 1.09 | 1.08 / 1.08 / 1.08 |
| O2 σ 0.5 (Octane ON) | 0.99 / 1.02 / 1.03 | same | **1.08 / 1.10 / 1.10** |
| O3 σ 1 (Octane ON) | 0.91 / 0.96 / 0.98 | same | **1.11 / 1.15 / 1.14** |
| O5 overlay σ₀ 0 | 1.04 | 1.04 | 1.04 |
| O7 overlay squared | 1.04 | 1.04 | 1.04 |
| O8 overlay σ₀ 1 | 0.85 / 0.88 / 0.93 | **1.05 / 1.09 / 1.14** | |

The third column (orbx-variants `set=…diffuseBrdf=e1+…albedo=x<πA>`)
stands for the gloss path, where the base is Lambertian × the flat term:
1.08 – 1.15 against the classic A's 0.99 / 0.92 (1.26 / 1.49 × A).

Calib-Overlay (sphere disc r 38 px; overlay pixels Iray G > 0.03):

| | before | after |
|---|---|---|
| V8 disc Y | 0.90 | 1.05 |
| V8 overlay Y | 0.85 | 1.05 |
| V8 blue cells B | 0.82 | 1.03 (V6, σ₀ 0: 1.01) |
| V8 yellow cells R / G | 0.88 / 0.83 | 1.07 / 1.04 (V6: 1.05 / 1.03) |
| V1–V7 | 1.03 – 1.05 | 1.03 – 1.05 |

## Matte base: Octane's Oren-Nayar or Lambert × π A?

By the head-on argument Octane's Oren-Nayar is ~20 % dark at σ 1 (0.91
at the centre). Lambert × π A is brighter (1.11 centre, 1.15 middle
disc), Octane's Oren-Nayar with its albedo × π A / A_classic would be
1.13 / 1.20. Radial Octane / Iray profile of O3 (r / R 0 → 0.97):
Octane ON 0.89 → 1.00 → 0.93 (spread 0.11), Lambert 1.09 → 1.17 → 1.00
(0.17); pixel correlation 0.979 vs 0.971. Octane's Oren-Nayar keeps the
shape better and stays within ±10 % in light: kept for the matte base.
The overlay under it goes relative to the base: × π A(σ₀) / π A(σ), so an
overlay at the base's roughness mixes in as is (dividing by Octane's
classic A instead would make it 1.24 × the identical base at σ 1).

## Rule

- Lambertian path: base × π A(σ), overlay × π A(σ₀) (Fujii's head-on
  albedo, `oren_nayar_flat`).
- Octane's Oren-Nayar path: the base as is, the overlay × π A(σ₀) /
  π A(σ).

## Lessons

- Ratios between spheres of one image cross positions; in a wide-angle
  frame under an HDRI that is worth 25 %. Compare each sphere with
  itself across renderers and take a double ratio against a sphere
  whose Octane factor is known.
- The sphere's centre (n = v) isolates the A term of any Oren-Nayar:
  the cleanest single number to tell models apart.
- Check the renderer version against the docs' changelog: the MDL
  BSDF changed under the same name.

## Open

- The overlay at σ₀ ≠ σ over a matte rough base (× π A(σ₀) / π A(σ)) has
  no Iray reference; the rule follows from the base matching.
- The rim falloff under back-light (B's negative half) isn't in a flat
  factor; on Calib-Diffuse it leaves the 90 % disc 1.14.
