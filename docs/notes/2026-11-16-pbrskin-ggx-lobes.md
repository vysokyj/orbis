# 2026-11-16 — PBRSkin's lobes on GGX, Octane's plain GGX under an HDRI (#186)

#186: `pbr_skin.mdl` draws every PBRSkin lobe (both dual lobes, the Top
Coat, the metal's, the flakes) with `df::microfacet_ggx_smith_bsdf` at
α = roughness², `multiscatter_tint` 0; the export drew them as Uber's
Phong lobes, Beckmann at `phong_pin(α)` (GGX EP / STD past α 0.25). From
#188 (`2026-11-16-pbrskin-rim-curve.md`): with Schlick's curve against
Universal's Fresnel divided out, r 0.4 plain GGX is flat 0.97 – 1.00 ×
Iray, but at α 0.36 (Calib-PBRSkin P2 / P3) plain GGX sits 1.03 – 1.19,
GGX EP 1.00 – 1.08. This session: explain plain GGX's level at α 0.36
before choosing the model, then ship the mapping. No Iray: the
references in `calib/references/iray` serve.

## 1. Docs

- MDL spec (`microfacet_ggx_smith_bsdf`): GGX distribution, Smith
  shadowing; `multiscatter_tint` 0 = no energy compensation.
- Octane manual (BRDF models): GGX EP adds multiple scattering between
  microfacets so rough surfaces don't darken; plain GGX is the old model.
- #95's note (`2026-11-10-phong-lobes.md`): in a white furnace Octane's
  Beckmann / GGX metals are the analytic single-scatter Smith lobe at α =
  pin², GGX EP adds all of multiple scattering; under the HDRI Octane's
  rough *dielectric* Beckmann rose over its analytic lobe past pin 0.5
  (1.14 – 2.25 at pin 0.6 – 1) while metals stayed on theirs — the origin
  of `BECKMANN_DIELECTRIC_MAX_ALPHA`. GGX was not checked that way then.

## 2. Decomposition: Octane's GGX against the analytic lobe (known input)

Calib-Glossy's export with every sphere replaced (`scripts/dev/hdri-lobe.py
variants`, `~/Library/Caches/orbis/calib/c186`): G1 a Lambert 0.2 (the normaliser),
G2 a near-mirror (pin 0.05, fits the HDRI's orientation), G3 – G8 at pin
0.4 / 0.5 / 0.6 / 0.7 / 0.8 / 1; black dielectrics (specular 1, IOR 1.5)
or white metals (metallic 1), `brdf` 2 / 6 / 1; floor hidden; the scene's
HDRI or a white dome. 1024 samples, raw pass, ~65 – 115 s each.

Analytic side (`hdri-lobe.py measure`, nothing fitted): per pixel the camera
ray hits the sphere (dump + `.duf` camera, as `sphere_discs`), the lobe
is sampled by GGX's visible normals (Heitz) at α = pin², weight
F(v·h) · G1(l) (separable Smith; height-correlated moves pin 1 by
0.1, pin 0.8 by 0.03, pin ≤ 0.6 by < 0.01), the radiance read from the HDRI
(`DTHDR-RuinsB-500`, lat-long, orientation fitted on G2: corr(log) 0.93);
Lambert by cosine sampling. Ratio = (Octane / Octane's Lambert) /
(analytic / analytic Lambert). Disc means < 0.93 R:

| pin | GGX diel HDRI | furnace | HDRI / furnace | EP diel HDRI | furnace | HDRI / furnace | GGX metal HDRI | furnace | EP metal furnace | GGX / EP under HDRI |
|---|---|---|---|---|---|---|---|---|---|---|
| 0.05 | 0.945 | 0.976 | 0.97 | 0.945 | 0.976 | 0.97 | (0.65) | 0.986 | 0.988 | 1.00 |
| 0.4 | 1.125 | 0.975 | **1.15** | 1.065 | 0.988 | **1.08** | (0.84) | 1.002 | 1.062 | 1.06 |
| 0.5 | 1.200 | 0.974 | **1.23** | 1.084 | 1.000 | **1.08** | (0.93) | 0.998 | 1.136 | 1.11 |
| 0.6 | 1.266 | 0.975 | **1.30** | 1.114 | 1.023 | **1.09** | 1.015 | 0.995 | 1.262 | 1.14 |
| 0.7 | 1.298 | 0.980 | **1.32** | 1.155 | 1.062 | **1.09** | 1.071 | 0.994 | 1.458 | 1.12 |
| 0.8 | 1.310 | 0.985 | 1.33 | 1.236 | 1.119 | 1.10 | 1.082 | 0.990 | 1.750 | 1.06 |
| 1.0 | 1.305 | 1.000 | 1.31 | 1.460 | 1.321 | 1.11 | 1.083 | 0.991 | 2.807 | 0.89 |

(The sharp metals under the HDRI, in brackets, are the sun's glint
landing a few pixels off the model's; their mid rings dip to 0.2 – 0.6.
The rough ones are the integration's baseline: 1.02 – 1.09.)

- **In a white furnace every model is what it should be**: plain GGX
  metal = the analytic Smith-GGX to 1 % at every pin, the dielectric to
  ±2.5 % (the mirror's own pattern per ring), EP's metal 1.00 at every
  pin (all multiple scattering back), EP's dielectric +1 / 3 / 5 / 8 / 14
  / 32 % at pin 0.4 – 1 (a dielectric's multiple scattering, F0 0.04).
- **Under the HDRI plain GGX's dielectric is not**: 1.15 → 1.32 over its
  furnace self at pin 0.4 → 0.7, rising towards the rim (pin 0.6 per
  ring 1.15 core … 1.37 at 0.8 – 0.85 R), against the rough metal's
  baseline 1.02 – 1.09: +6 / 14 / 19 / 21 %. GGX EP's dielectric holds
  its furnace level (1.08 – 1.11 at every pin, the baseline) — and so,
  under the HDRI, EP draws a dielectric *darker* than plain GGX (0.88 –
  0.95 × at pin 0.4 – 0.8, integrator-free), where in a furnace it is
  brighter. Plain GGX on a rough dielectric is the importance-sampled
  environment defect #95 found in Octane's Beckmann (`brdf` 1 here, same
  probe: 1.08 / 1.17 / 1.40 / 1.71 / 1.98 / 2.39 at pin 0.4 – 1),
  milder.
- So plain GGX's level at α 0.36 on Calib-PBRSkin (1.03 – 1.19 over Iray
  once the curve is out) is Octane's, not the distribution's and not the
  pin (`phong_pin(0.36)` = 0.591 against √α = 0.6: 3 % in α). Per ring
  the probe's GGX / EP at pin 0.6 is 1.02 1.03 1.06 1.11 1.18 1.23 1.23
  1.20 (core → 0.93 R); Calib-PBRSkin P3's (#188's leftover rows) 1.03
  1.03 1.05 1.07 1.10 1.09 1.06 1.04 — the same sign and rise to 0.8 R,
  about half the size (lobe 1 is 0.85 of P3's lobes, lobe 2 a mirror; the
  scene's light is not Calib-Glossy's, and the defect depends on how
  directional the light is). P2, half P3's specular, 1.08 → 1.06.

## 3. The rule

PBRSkin lobes (`lobe_curve`, `ggx_brdf`, both Looks): the pin **√α**
(the roughness itself, `RoughCurve::Ggx`) on Universal and on the OpenPBR
skin (lobe 1, lobe 2's coating, the Top Coat); `brdf` **GGX (2)** unless a
dielectric lobe on the node is past α 0.25 (`GGX_DIELECTRIC_MAX_ALPHA`),
then **GGX EP (6)**; an all-metal node (the metal Mix's lobes) GGX at any
roughness (Octane's GGX metal is the MDL's single-scatter lobe under the
HDRI too). The limit is a judgment call inside the measured bracket
(Iray: α 0.16 GGX right, α 0.36 EP right; probe: plain GGX's excess
+6 % at pin 0.4, +14 % at 0.5), set at Beckmann's measured α for one
rule. Faithful no longer puts PBRSkin's rough lobes on STD: STD stood for
Phong's shape (#162), PBRSkin's shape is GGX.

## 4. Checked against Iray

enhanced-final 400, denoised pass, the export after the change against the
references in git (5000); "before" = the #167 / #188 renders of the same
scenes (`c167/r2`, the export at db2345b) or the old export with every GGX
node set back to Beckmann (`c186/tl`, MetalTL), ~40 – 50 s per scene.

Spread (p99 / median, `highlight-spread.py`) Octane / Iray's:

| scene | spheres | before | after |
|---|---|---|---|
| Calib-PBRSkin-Metal | N1 – N8 | 1.13 1.17 1.16 1.36 1.06 1.27 1.00 1.03 | 0.98 0.99 0.98 1.03 0.98 1.07 1.00 1.07 |
| Calib-PBRSkin-Makeup | K1 – K8 | 1.13 1.13 1.20 1.27 1.27 1.07 1.39 1.48 | 0.98 1.05 1.06 1.10 1.05 1.07 1.12 1.17 |
| Calib-PBRSkin | P2 / P3 / P6 | 0.96 / 1.00 / 1.09 | 0.96 / 0.97 / 1.07 |
| Calib-PBRSkin-MetalTL | M1 – M8 | 1.02 1.08 1.07 1.21 1.31 0.98 1.35 1.19 | 0.96 1.00 0.98 1.00 1.17 0.95 1.06 1.02 |
| Calib-PBRSkin-DetailLayers | E3 – E8 | 1.30 1.30 0.82 1.07 1.43 1.41 | 1.06 1.07 1.04 1.05 1.19 1.17 |
| Calib-PBRSkin-Detail | D1 – D8 | 1.17 1.15 1.22 1.28 1.14 1.10 1.25 1.66 | 0.94 1.02 1.05 1.13 1.12 1.15 1.22 1.37 |
| Calib-PBRSkin-Occlusion | O1 – O8 | 1.13 1.17 1.15 1.23 1.31 1.37 1.05 1.33 | 1.00 1.00 0.96 1.18 1.11 1.16 1.09 1.22 |

(DetailLayers / Detail / Occlusion "before": the enhanced-final renders of
2026-11-15 / -16 from before this change.)

Disc means Octane / Iray (`--vs`): Calib-PBRSkin-Metal 1.18 1.17 1.14
1.22 1.09 1.22 1.09 1.28 → 1.16 1.16 1.13 1.19 1.07 1.18 1.09 1.24 (N8,
metal lobe 1 at α 0.36 now plain GGX: 1.28 → 1.24); -Makeup K7 / K8 1.17 /
1.24 → 1.12 / 1.17; Calib-PBRSkin P7 / P8 1.07 / 1.09 → 1.04 / 1.05;
MetalTL 1.07 – 1.15 → 1.05 – 1.14. Everything moves down or stays.

Rim with Schlick's curve divided out (`rim-curve.py measure`, as #188):

| sphere | left over curve and frame, core → 0.93 R |
|---|---|
| N1 (α 0.16, now GGX) | 0.98 0.98 0.98 0.99 1.00 0.99 0.99 0.99 |
| K1 | 0.98 0.98 0.98 0.99 1.00 0.99 0.99 0.99 |
| O1 / O2 / O3 / O5 | 0.98 – 0.99 / 0.98 – 1.01 / 0.96 – 0.97 / 0.97 – 1.03 |
| P2 (α 0.36, GGX EP at √α) | 1.02 1.02 0.98 1.01 1.03 1.05 1.06 1.06 |
| P3 | 1.00 1.00 0.99 0.98 1.00 1.01 1.02 1.03 |

The r 0.4 lobes are #188's GGX rows to 0.01 (the pin moved 0.399 →
0.4); P2 / P3 at GGX EP as the export had them, a hair lower (pin 0.591
→ 0.6). O4 (n 0.01, the red base's blue on a quarter of the signal)
1.05 – 1.16 as before.

Faithful (faithful-final, Calib-PBRSkin): the rough lobes were STD
there (#162's Look split for Phong lobes); P2 / P3 left over curve and
frame at STD (the new export with `brdf6=7`, `c186/faith`) 1.11 – 1.32 /
1.12 – 1.19, now at GGX EP 1.00 – 1.07 / 0.98 – 1.03; spreads equal to
0.1.

Not checked: Demo-Gladiator (no Iray reference yet); the OpenPBR skin
path's pin moves `phong_pin(α)` → √α, ≤ 1.5 % in α on these roughnesses
(Calib-PBRSkin-Makeup-TL's skin spheres L6 – L8 unchanged to 0.01).

## Process

- A furnace and an HDRI render of the same probe split "Octane's model is
  off" from "Octane's model is off under directional light": the furnace
  alone (as in #95 for Beckmann) would have called plain GGX exact.
- EP < GGX under an HDRI, EP > GGX in a furnace: that inversion alone,
  integrator-free, says one of them is wrong under directional light; the
  HDRI integral says which.
- A metal at the same pins is the integration's baseline: its sharp pins
  are useless (the sun glint's position), its rough ones 1.02 – 1.09.
