# 2026-11-12 — The hair shader's glossy lobe on strands (#79)

Follows [hair shader parts](2026-11-11-hair-shader-parts.md). A first
Faithful mapping (the MDL's layers as two Mix materials: a metal per
glossy over a Universal with the diffuse transmission, strands at 1×
width) rendered against Iray.

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

## 5. The measured share (the user's choice of three)

Prism meshes, a measured share, or no gloss lobes: the share. Iray's
strands over the export's rough metal on curves, white gloss alone: fibres
0.49 / 0.52 / 0.53 / 0.71 (G / H / I / J), prisms 0 / 0.08 / 0.13 / 0.08 →
`HAIR_FIBRE_GLOSS_SHARE` 0.55, `HAIR_PRISM_GLOSS_SHARE` 0.10, on the
metals' colours (the Mix weights stay: scaled weights would hand the
difference to the transmission). Octane / Iray after it:

- fibres: Calib-Strand-Hair whole 1.11 / 1.01 / 0.99 / 0.91, gloss alone
  1.24 / 1.05 / 1.06 / 0.93, highlight 1.02 / 0.93 / 0.88 / 0.85; -HairW
  white gloss 1.12 / 1.00 / 0.88 / 0.81, 0.5 of it 1.10 / 0.96 / 0.92 /
  0.86;
- prisms (-HairW-T3 against the Uber stand-ins): white gloss - / 1.53 /
  0.75 / 1.65, gloss 0.5 over white transmission 0.94 / 0.70 / 0.43 / 0.67.
  The dense patch shows the price: a dimmed metal absorbs what Iray's
  gloss sends on into the hair. With the full metal that column was 3.0 /
  2.3 / 1.7 / 2.5.
- Calib-Hair (faithful-final, 256 samples, 1× width): crown 0.84 (RGB
  0.78 / 0.85 / 0.91), top tuft 1.01, side 0.95; the crown's r / g 1.13
  (Iray 1.23, the Hair material at 2× 1.01). Calib-Eye: lashes and brows
  read as Iray's, the brows a little lighter.

## 6. The share does not survive another light

Calib-Strand-UberG / -T3 under the dome (Calib-Aniso's HDRI), the sun
turned 60° (-Side) and 180° (-Back); Iray Server 2000 samples (51 - 66 s).
Iray over the export's plain-GGX metal on curves, the hair lobe (column 1),
G / H / I / J:

| | front sun | side sun | back sun | dome |
|---|---|---|---|---|
| fibres | 0.52 / 0.60 / 0.66 / 0.72 | 0.69 / 1.01 / 1.14 / 0.95 | 3.78 / 2.27 / 1.70 / 8.80 | 2.09 / 1.95 / 1.89 / 3.51 |
| prisms | 0.00 / 0.09 / 0.16 / 0.09 | 0.00 / 0.33 / 0.46 / 0.18 | 1.12 / 0.31 / 0.67 / 2.93 | 1.33 / 1.54 / 1.25 / 3.84 |

The isotropic column stays at 0.95 - 1.07 on fibres under every light
(the curve is the fibre). The lobe's share is the light's direction, not a
property: 0.10 was the front sun's. Under the dome Iray returns *more*
than the metal - plain GGX at roughness 1 loses about half.

Probes (Calib-Strand-HairW-Dome / -T3-Dome, new, Octane only; the Uber
references above): GGX energy preserving, full colour, against the 0.10 /
0.55 shares -

| Octane / Iray | energy preserving | shares |
|---|---|---|
| dome, fibres, gloss alone | 0.81 / 0.89 / 1.06 / 0.46 | 0.33 / 0.33 / 0.31 / 0.18 |
| dome, prisms, gloss alone | 1.28 / 1.12 / 1.61 / 0.43 | 0.13 / 0.13 / 0.14 / 0.06 |
| dome, prisms, 0.5 over transmission | 1.47 / 1.39 / 1.65 / 1.16 | 0.62 / 0.49 / 0.33 / 0.50 |
| front sun, prisms, gloss alone | - / 16.7 / 14.1 / 12.0 | - / 1.53 / 0.75 / 1.65 |
| front sun, prisms, 0.5 over transmission | 3.33 / 2.87 / 2.67 / 2.92 | 0.94 / 0.70 / 0.43 / 0.66 |

Calib-Hair (HDRI-lit): shares crown 0.84 (r / g 1.13), top 1.01, side
0.95; energy preserving 1.22 (r / g 1.21, Iray 1.23), 1.13, 1.23 - the
hue exact, the level uniform and the transmission's own (a round curve
against a prism's 0.87 coverage). **Taken: energy preserving, no share**
(the shares of section 5 are out of the code). Front sun on fibres with
it: whole shader 1.88 / 1.62 / 1.64 / 1.45, white gloss 2.06 / 2.51 / 3.43
/ 1.41.

## Left

- Direction: under one hard light the gloss lands where a round rough
  strand sends it (front sun 12 - 17 × on prisms' gloss alone, vertical
  strands under the dome 0.43 - 0.46). Prism meshes would draw it.
- The export's Uber weighted anisotropic gloss on strands (section 4).
- Clean references of the hair shader itself on prisms (#94).

## Process

- Look at the thick strand's picture before trusting sums: the tube
  against the line named the cause in one crop.
- When a shader's module fails in Iray, the same MDL building block in a
  shader that loads (Uber's weighted mixing) is a reference without
  restarts; equal sums on the shared columns prove the stand-in.
- A constant measured on one light is a fit to that light: the dome is
  the reference for an amount, a single sun for a direction. The share
  looked explained (flat faces) and still failed the second light.
