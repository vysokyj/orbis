# 2026-11-13 — The gloss "by position": wrong discs and Schlick's shape (#137)

#137 (from #106): on Calib-DualLobeGlossy the glossy layer alone came out
1.21 / 1.33 / 1.66 × Iray on spheres 1 / 3 / 8 that carry the same layer,
the dual lobe alone 1.14 – 1.37 — "depending on the sphere's position".
Re-measured on the current export (after #81, #100, #133: both layers on
the MDL's Schlick curve, F0 as the IOR). Octane enhanced-final, 400
samples, denoised pass; Iray references in git (Calib-DualLobeGlossy,
-gloss, -dl, -bare 5000; Calib-DualLobe, Calib-Glossy). No new Iray.

## 1. The discs were wrong

The per-sphere numbers of #106 / #100 / #81 came from the disc finder of
`highlight-spread.py` (red pixels of the Calib-Glossy Iray render, R > 1.6
G and R > 0.12): it finds only the brightest red part of each sphere —
discs of radius 14 – 24 px around the lit upper right, where the spheres
are 45 px in radius. Those "discs" sat on the sun's highlight. From the
-bare render itself (all eight spheres matte, R > 2.5 G and R > 0.02):
eight discs of r 45 – 46, centres 20 px left and 20 px lower. #146 (the
same finder's boxes) gets the details.

`scripts/dev/gloss-split.py measure` (new): the discs from the bare
render, and per pixel scene = a · bare + g · white (the gloss is white,
the diffuse red), a = (R − G) / (Rb − Gb), g = R − a Rb.

## 2. Re-measured (gloss g, Octane / Iray; the frame's bare spheres 1.08 – 1.09)

| sphere | -gloss disc | core | -dl disc | core | both disc | Calib-DualLobe disc | Calib-Glossy disc |
|---|---|---|---|---|---|---|---|
| 1 | 1.20 | 1.12 | - | - | 1.20 | 1.16 | - (matte) |
| 2 | - | - | 1.15 | 1.07 | 1.15 | 1.12 | 1.16 |
| 3 | 1.25 | 1.15 | 1.16 | 1.08 | 1.22 | 1.11 | 1.18 |
| 4 | 1.30 | 1.16 | 1.18 | 1.10 | 1.23 | 1.18 | 1.12 |
| 5 | 1.16 | 1.09 | 1.12 | 1.05 | 1.18 | 1.13 | - (r 0) |
| 6 | 1.22 | 1.10 | 1.20 | 1.10 | 1.21 | 1.27 | 1.17 |
| 7 | 1.27 | 1.13 | 1.19 | 1.09 | 1.24 | 1.09 | 1.07 |
| 8 | 1.31 | 1.11 | 1.32 | 1.10 | 1.31 | 1.23 | 1.27 |

Over the frame's 1.08: the layers alone are 1.06 – 1.22 on the disc and
0.97 – 1.07 on the core. The diffuse kept under each layer (a) agrees
within 1 – 5 % (Octane's slightly higher under the glossy layer: 0.91 v
0.88 on sphere 1). "1.2 – 1.7" was the highlight region alone, measured
against a disc in the wrong place. What is left still grows from
sphere 1 to 8, on the disc's rim, not its core.

## 3. Where it grows: the sun's half angle

The Ruins HDRI (DAZ's built-in, 500 px) has a sun 16° above the horizon:
its top 0.1 % of texels carry 22 % of the light. Its highlight on each
sphere peaks at offset (dx, dy) of the radius; there n = h, so θ_d (the
half angle between view and sun) = asin |d|: 35 / 39 / 42 / 49 / 61° on
spheres 1 / 3 / 4 / 6 / 8 (-gloss, Iray's peaks; Octane's within 1 – 3°
except sphere 8's at the very rim). The sun sits right of and slightly
behind the camera, so the further right the sphere, the more grazing the
reflection.

Universal's Fresnel at the IOR of F0 against the MDL's Schlick curve
(`gloss-split.py fresnel`):

| r (F0) | IOR | 0° | 30° | 45° | 55° | 60° | 70° | 80° | 85° | hemisphere |
|---|---|---|---|---|---|---|---|---|---|---|
| 0.1 (0.008) | 1.196 | 1.00 | 1.06 | 1.20 | 0.99 | 0.84 | 0.68 | 0.73 | 0.83 | 0.79 |
| 0.25 (0.02) | 1.329 | 1.00 | 1.05 | 1.24 | 1.27 | 1.17 | 0.94 | 0.87 | 0.91 | 0.99 |
| 0.5 (0.04) | 1.500 | 1.00 | 1.04 | 1.19 | 1.30 | 1.27 | 1.08 | 0.95 | 0.94 | 1.07 |
| 1 (0.08) | 1.789 | 1.00 | 1.02 | 1.12 | 1.21 | 1.22 | 1.10 | 0.96 | 0.95 | 1.07 |

The spec said "per angle within 1 – 8 % from r 0.25 up" (#100 checked
80°); at 45 – 60° the physical Fresnel is 1.12 – 1.30 × Schlick's. Schlick
under-reads a dielectric there; the sun's highlights sit exactly there.

Single-scatter sun model of each sphere (`gloss-split.py sun`): `libbsdf`
(Phong half-vector D, exponent 2 / r², v-cavities; the glossy layer w ·
Schlick(k · h), the dual lobe W · Schlick(max(n · k1, n · k2)) — the
`custom_curve_layer`'s base weight) against Octane as exported (Beckmann at
α = r / √(1 + r²), Smith, Fresnel at the IOR of F0). Octane / Iray:

| sphere | θ_d | glossy layer: model disc | measured / 1.08 | dual lobe (r 0.09 / 0.01, ratio 0.5): model disc | measured -dl / 1.08 |
|---|---|---|---|---|---|
| 1 / 2 | 35 / 36 | 1.07 | 1.11 | 1.08 | 1.06 |
| 3 | 39 | 1.11 | 1.16 | | 1.07 |
| 4 | 42 | 1.15 | 1.20 | | 1.09 |
| 6 | 47 – 49 | 1.17 (r 1) | 1.13 | 1.25 | 1.11 |
| 8 | 56 – 61 | 1.30 | 1.21 | 1.38 | 1.22 |

At the peak the model's ratio is the Fresnel / Schlick ratio within 1 –
3 % (D's peaks match by `phong_pin`, both maskings ≈ 1 at n = h): the
growth is Schlick's shape, not the lobes or the light. The measured trend
is flatter than the sun-only model — the sky lights the same disc at
other angles (where the two curves agree or turn) — and follows it in
order and size. Nothing unexplained beyond that dilution; no factor is
fitted for it.

## 4. Decision

Kept as it is. No IOR draws Schlick's curve: the head-on match (#81,
#100) or the hemispherical one (an IOR ~1.47 at r 0.5) still leave
1.2 – 1.3 × at 45 – 60° and turn below it past 75°, and Octane's curve is
the physical dielectric, Schlick's the approximation (the project's
"Octane over Iray" where it is more plausible). The spec now says so,
with these numbers. A Schlick-shaped Octane lobe (perhaps a layered
material's Metallic Layer in Artistic mode — untested what curve it
draws) would be a separate question with no probe yet — not this issue.

## Process

- Check a shared region finder against the image before reading its
  numbers: a disc of r 20 on a 45 px sphere turned a highlight into
  "the sphere".
- The bare scene of a decomposition is also the best disc finder and the
  diffuse's colour per pixel.
- A highlight's position on the sphere gives θ_d for free (n = h): the
  angular dependence of a lobe under one strong light can be read without
  knowing the camera or the HDRI's orientation.
