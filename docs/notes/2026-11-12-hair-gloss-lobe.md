# 2026-11-12 — The hair shader's glossy lobe on strands (#79)

Follows [hair shader parts](2026-11-11-hair-shader-parts.md). A first
Faithful mapping (the MDL's layers as two Mix materials: a metal per
glossy over a Universal with the diffuse transmission, strands at 1×
width; not committed) rendered against Iray.

## 1. The layers on fibres

Calib-Strand-Hair / -HairW (fibres; the mapping writes no transmission
there, as Iray drops it), Octane / Iray, linear luminance per object:

| | G thick | H sparse | I dense | J vertical |
|---|---|---|---|---|
| whole shader (Calib-Hair's values) | 1.88 | 1.61 | 1.61 | 1.45 |
| hair gloss alone | 2.10 | 1.66 | 1.72 | 1.48 |
| highlight alone | 1.82 | 1.60 | 1.56 | 1.46 |
| white gloss, weight 1 | 2.03 | 1.91 | 1.90 | 1.41 |
| white gloss 0.5 over transmission | 1.95 | 1.64 | 1.61 | 1.46 |

R / G / B within 0.03, transmission columns black in both: the weights and
colours are right, the glossy lobe's level is not.

## 2. Octane draws no anisotropy on curves

The shader's Anisotropy defaults to 1 (`anisotropy_conversion`: one axis
at r, the other at r / 0.0001). On the thick strand Iray draws a thin line
along it; the export (roughness 1, `anisotropyTexture` 0.70) an evenly
lit tube. Probes on -HairW: `rotation` 0.75, and `anisotropyTexture`
−0.70, render the same picture and sums as the unrotated one (within
1 %, the dense patch 16 %); isotropic at the lobe's r draws Iray's line,
wider. On `BasisCurves` a Universal's anisotropy is not applied - the
long axis's roughness is all that shows.

## 3. The lobe in Iray, through Uber (no #94)

Calib-Strand-UberG / -T3: Uber's weighted mixing, a white gloss alone at
the shader's roughness - the same `weighted_layer` of a
`simple_glossy_bsdf`. Columns: anisotropy 1 at rotation 0.25 (the hair
shader's `texture_tangent_v`), at 0 (Uber's own tangent), isotropic, and
0.5 of the first over a white translucency. Iray Server, 2000 samples
(62 / 56 s). Sums:

| Iray | rot 0.25 | rot 0 | isotropic | 0.5 over transl. |
|---|---|---|---|---|
| fibres G / H / I / J | 773 / 1349 / 2405 / 1290 | 4058 / 10790 / 18140 / 14 | 1439 / 15188 / 22982 / 11 | 383 / 686 / 1069 / 647 |
| prisms G / H / I / J | 1 / 206 / 574 / 160 | 2685 / 11691 / 19704 / 70 | 5 / 2382 / 6038 / 3 | 381 / 843 / 2105 / 543 |

- Fibres, rot 0.25 and the 0.5 column equal Calib-Strand-HairW's white
  gloss and gloss-over-transmission columns (743 / 1371 / 2361 / 1349 and
  384 / 687 / 1070 / 647): Uber stands in for the hair shader exactly.
- The hair shader's lobe is **sharp across the strand, flat along it**, on
  fibres and prisms alike (the thin line on the thick fibre; rot 0, the
  other way round, lights the whole tube and goes dark on vertical
  strands).
- On **prisms** - product hair - that lobe returns little: three flat
  faces each send a fan, where a round fibre's normals sweep it over every
  direction. White gloss, prisms / fibres: 0.00 / 0.15 / 0.24 / 0.12. Per
  unit weight the gloss returns 13 - 17 % of what the transmission does
  (sparse, dense, vertical; the 0.5 column minus half the gloss), nothing
  on the thick strand. At Calib-Hair's weights (gloss and highlight 0.29,
  transmission 0.71) about 6 % of the hair's light.

## 4. Octane against that

Faithful exports, Octane / Iray:

- Isotropic gloss, curves against Iray's fibres: 1.01 / 0.85 / 0.95 /
  0.97 - a round curve is Iray's fibre.
- Transmission on prisms (hair shader's export against UberT-T3, and
  UberT-T3 itself): 0.85 - 1.25, as at 1× before.
- The hair lobe on prisms (roughness-1 metal on curves): thick strand
  1504 against 1, sparse 12.7, dense 7.8, vertical 12.0; 0.5 over the
  transmission 3.0 / 2.3 / 1.7 / 2.5.
- The export's own Uber weighted anisotropic gloss on strands is off the
  same way (UberG rot 0: 0.37 / 0.21 / 0.21 on fibres, vertical lit where
  Iray's is dark): existing code, same cause.

## Left

- How Faithful renders the glossy lobes of tessellated hair: round curves
  cannot draw a flat-faced, flat-along lobe. Prism meshes instead of
  curves, a measured prism share on the metal, or leaving the lobes out
  (6 % on Calib-Hair) - undecided.
- Clean references of the hair shader itself on prisms (#94).

## Process

- Look at the thick strand's picture before trusting sums: the tube
  against the line named the cause in one crop.
- When a shader's module fails in Iray, the same MDL building block in a
  shader that loads (Uber's weighted mixing) is a reference without
  restarts; equal sums on the shared columns prove the stand-in.
