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

## 7. Octane's Hair material against its albedo (for Enhanced)

Probe (a temporary build, not committed): Calib-Strand-HairW / -Dome's
curves at 1× width as Hair (147), `specular` 1, roughness 0.3 both ways,
grey `albedo` per column, two renders of four columns; 256 samples. Sums:

| albedo | 0 | 0.1 | 0.25 | 0.5 | 0.7 | 0.85 | 0.95 | 1 |
|---|---|---|---|---|---|---|---|---|
| dome, G thick | 86 | 449 | 590 | 682 | 821 | 761 | 719 | 718 |
| dome, H sparse | 154 | 773 | 1090 | 1326 | 1552 | 1481 | 1426 | 1429 |
| dome, I dense | 122 | 647 | 1502 | 2990 | 3245 | 3457 | 3628 | 3936 |
| dome, J vertical | 97 | 351 | 509 | 623 | 725 | 681 | 669 | 673 |
| front sun, G | 150 | 346 | 443 | 411 | 428 | 532 | 534 | 433 |
| front sun, H | 281 | 831 | 1279 | 1293 | 1407 | 1733 | 1769 | 1409 |
| front sun, I | 389 | 1824 | 3786 | 5784 | 6692 | 8079 | 8119 | 7402 |
| front sun, J | 14 | 43 | 65 | 80 | 83 | 86 | 86 | 87 |

(0.7 - 1 are the second render; the steps at 0.5 → 0.7 and 0.95 → 1 on
the single-scattering rows are between renders and columns, a few %.)
A single strand's return saturates by albedo 0.25 - 0.5 (the rest goes
through it); many strands deep it keeps rising. Iray's white strand for
scale (prisms): under the dome a white gloss returns 375 / 810 / 870 /
929, a white transmission about 1080 on the sparse row; front sun, white
transmission 902 / 1888 / 6674 / 1174. So one white Iray strand equals
Hair at albedo about 0.2 on the sparse row and about 0.7 on the dense one:
no albedo maps Iray's strand onto Hair's at every depth - the fibre BSDF
forwards light where Iray's diffuse transmission spreads it.

## 8. Enhanced: the pile albedo

From section 7, no albedo equals Iray's strand at every depth, so the
albedo takes the meaning Octane gives it: the colour after many bounces.

- Iray's strand, from the transmission columns of Calib-Strand-UberT-T3
  per channel (white, 0.5, blond = 0.76 / 0.60 / 0.49 linear), R / R white:
  thick prism 0.27 / 0.37 / 0.59 at tint 0.5 / 0.60 / 0.76 - tint², in and
  out; sparse 0.24 / 0.33 / 0.53; dense 0.094 / 0.142 / 0.280; vertical
  0.26 / 0.36 / 0.56.
- Strand albedo A = ρ + (w · tint)² / (1 − ρ) (ρ the glossies, w the
  transmission's weight; the far side reflects back in by ρ).
- Dense against van de Hulst's isotropic pile at A = tint²: 0.059 / 0.092
  / 0.182, measured 1.59 / 1.54 / 1.54 × that - one factor, the white
  patch's finite depth. Octane's Hair on the patch: albedo 0.1 / 0.25 /
  0.5 return 0.16 - 0.25 / 0.38 - 0.51 / 0.76 - 0.78 of its white.
- So `albedo` = pile(A), `specular` 1, width 1×. Calib-Hair (enhanced-
  final, 256 samples): crown 1.45, top tuft 1.22, side 1.65 × Iray, r / g
  1.25 / 0.86 / 1.27 (Iray 1.23 / 0.85 / 1.21).
- Tried and dropped: the Hair material's own reflection subtracted from
  the albedo (albedo 0 returns 0.031 / 0.053 of the white dense patch
  under the dome / front sun, 0.025 in albedo units): 1.31 / 1.18 / 1.54,
  crown r / g 1.32 - redder than Iray for 10 % of level.

## 9. Uber strands at 1x, and Morgan's lost gloss

The user saw a scene where Faithful came out darker than Iray and Enhanced
about like it. Suspect: Uber strand hair now at 1x width (its rules were
checked at 2x). Calib-Strand, -Dome, -Side, -T2, -T3, -T2-Dome in Faithful
Final (1x) against Faithful Preview (2x), Octane / Iray:

| | 1x | 2x |
|---|---|---|
| A, one thick fibre: diffuse / gloss iso / aniso | 0.99 / 1.04 / 0.89 | 1.97 / 1.55 / 1.81 |
| A under the dome | 1.01 / 1.05 / 1.06 | 2.01 / 2.04 / 2.08 |
| C diffuse patch, fibres sun / dome | 0.86 / 0.92 | 0.95 / 0.99 |
| C diffuse patch, ribbons sun / dome | 0.72 / 0.81 | 0.79 / 0.87 |
| C5 Morgan's Strands, ribbons sun / dome | 0.54 / 0.36 | 0.46 / 0.29 |

The width is no regression: a fibre is right at 1x, twice Iray's at 2x.
Morgan's material is dark at either width.

Its export: albedo 0.23 / 0.18 / 0.10, a thin-wall diffuse transmission
0.39 / 0.35 / 0.28, specular 0.86 / 0.76 / 0.45 - and **`ior` 1** in
Faithful (5.63 in Enhanced, the gloss's F0 0.49). Faithful's
`thin_glass_ior_one` (a thin-walled glass shows none of its own reflection
in Iray) caught every thin-walled material with a refraction, also a
partial one whose IOR pin holds the *gloss's* F0: the strands had no gloss
at all. Fixed: IOR 1 only where the pin is the refraction's own.

| C5 / D5, Octane / Iray | before | fixed | Enhanced |
|---|---|---|---|
| fibres, sun | 0.51 / 1.40 | 0.47 / 1.25 | 0.47 / 1.25 |
| ribbons, sun | 0.54 / 0.57 | 0.50 / 0.51 | 0.50 / 0.51 |
| ribbons, dome | 0.36 / 0.44 | 0.62 / 0.56 | 0.62 / 0.56 |

(Under the sun the gloss takes light from the base and mirrors it away
from the camera: a little darker, as Iray's ribbons do.)

Variants of the export on the way (project.ocs edited, `orbx_ocs`):
`transmissionType` 1 (Diffuse) in place of 3 (Thin Wall (Diffuse)) renders
the same on curves to the last digit; without the transmission C5 loses a
fifth to a quarter (0.54 -> 0.42, 0.36 -> 0.27). So the transmission
passes, and half of Iray's light is still missing with every part
written. Iray's own parts are unknown: Calib-Strand-UberM{,-T2}{,-Dome}
(materials 5, 14 - 16: whole, no gloss, no translucency, no refraction;
rows G - J) are generated for an Iray batch.

## 10. Morgan's material part by part (Calib-Strand-UberM)

Iray batch (4 scenes, 2000 samples): materials 5 / 14 / 15 / 16 (whole, no
gloss, no translucency, no refraction) on rows G (one thick strand), H
(300 sparse), I (3000 dense), J (sparse, vertical); fibres and ribbons
(-T2), sun and dome. Octane / Iray, Faithful + Final after section 9:

| | whole | no gloss | no transl. | no refr. |
|---|---|---|---|---|
| fibres, sun G / H / I / J | 0.47 / 0.48 / 0.48 / 1.15 | 1.17 / 1.13 / 1.12 / 1.11 | 0.88 / 0.82 / 0.79 / 0.92 | 0.50 / 0.48 / 0.49 / 1.17 |
| fibres, dome | 1.27 / 1.20 / 1.19 / 1.27 | 1.49 / 1.33 / 1.21 / 1.44 | 1.07 / 0.97 / 0.91 / 0.98 | 1.23 / 1.16 / 1.16 / 1.31 |
| ribbons, sun | 0.97 / 0.83 / 0.51 / 0.74 | 1.01 / 0.82 / 0.55 / 0.77 | 1.25 / 2.61 / 2.07 / 0.74 | 0.99 / 0.87 / 0.57 / 0.77 |
| ribbons, dome | 0.92 / 0.85 / 0.62 / 0.90 | 0.65 / 0.53 / 0.42 / 0.65 | 1.43 / 1.01 / 0.78 / 1.26 | 1.08 / 0.79 / 0.64 / 0.88 |

**Fibres.** Without the translucency Octane matches; with it the gloss
and the translucency together fail under the sun (0.47), each alone not.
Iray's fibres lose the transmission (section 1), Octane's curves pass it -
and a Universal with a transmission under its dielectric specular (IOR
5.63) loses the highlight: the same export without the transmission pin
renders the whole material 1.14 / 0.92 / 0.83 / 0.96 x Iray. Rule:
Faithful + Final writes no Uber translucency on fibres. After it every
cell of the fibre scenes is 0.76 - 1.14 (sun) and 0.91 - 1.07 (dome);
Calib-Strand-UberT's translucent fibres are black as Iray's, its gloss
over them 1.00 - 1.22. Enhanced keeps the transmission (0.44 - 0.47 under
the sun, 1.10 - 1.21 under the dome): the highlight it loses is Octane's
own, open.

**Ribbons** (Morgan's shape) keep their transmission in Iray. Tried on
the export (project.ocs edited), each cell against Iray's same cell:

| whole, G / H / I / J | sun | dome | B / R sun, dome (Iray 0.59, 0.80) |
|---|---|---|---|
| as exported | 0.92 / 0.77 / 0.47 / 0.70 | 0.88 / 0.80 / 0.59 / 0.84 | 0.62, 0.93 |
| a' = a / (1 + t), t' = sqrt(t (1 - a'^2)) | 1.00 / 0.93 / 0.62 / 0.78 | 1.07 / 0.99 / 0.73 / 1.10 | 0.81, 1.18 |
| the gloss as a coating | 2.23 / 1.81 / 1.09 / 0.70 | 0.87 / 0.80 / 0.58 / 0.87 | 0.66, 0.94 |
| both | 2.32 / 1.96 / 1.24 / 0.78 | 1.07 / 0.99 / 0.73 / 1.14 | 0.74, 1.18 |

The second row is "a ribbon is one sheet, a curve a closed tube": light
crosses a curve's surface twice (a thick prism returns tint^2, section 1 of
[hair-shader-parts](2026-11-11-hair-shader-parts.md)), so the tube gets
the transmission whose two crossings pass what one sheet does. It lifts
the level to Iray's on the single strand and the sparse patches, but the
hue goes cooler than Iray's (0.81 against 0.59) where the gloss is on,
while without gloss it is right (dome 1.09 against 1.05, as exported 0.85).
A coating in place of the specular brings back the highlight a round
strand has under the sun - which Iray's flat ribbons mirror away (2.2 x) -
and changes nothing under the dome. Neither went into the export: the hue
says a part is still missing, and the real Morgan Hair (Calib-Hair-Teens,
local, Faithful + Final after section 9) is not dark: crown 1.21 / 1.49 /
1.43, hilite 0.95 / 0.84 / 0.80, middle 1.53 / 1.81 / 1.76, lengths 1.06 /
1.25 / 1.21, tips 0.64 / 0.92 / 0.63 x Iray (R / G / B) - more
transmission would add to it. `transmissionType` 1 against 3 there: 7 %
darker, every patch.

## 11. Curves stay; Preview on the pile albedo

The user's decision on the ribbons of section 10: no ribbon meshes, the
round curves stay in both looks.

The two Preview variants still had the fitted albedo shares at 2x width.
Calib-Hair (local, 400 samples, direct lighting), crown / top tuft / side
against Iray, r / g (Iray 1.23 / 0.85 / 1.21):

| | level | r / g |
|---|---|---|
| fitted shares, 2x (Enhanced = Faithful Preview) | 0.68 / 0.85 / 0.68 | 0.96 / 0.76 / 1.00 |
| pile albedo, 1x | 1.17 / 1.15 / 1.19 | 1.13 / 0.83 / 1.12 |

Calib-Eye's frame 0.95 with it (Enhanced + Final 1.08). So every variant
but Faithful + Final now takes the pile albedo at 1x; the fitted shares
(`HAIR_COLOUR_SHARE`, `HAIR_TRANSMISSION_SHARE`, `HAIR_HIGHLIGHT_SHARE`),
`StrandHair::OctaneHair` and `Tuning::line_width_scale` are gone.

## Left

- Direction: under one hard light the gloss lands where a round rough
  strand sends it (front sun 12 - 17 x on prisms' gloss alone, vertical
  strands under the dome 0.43 - 0.46); ribbons' flat faces (section 10).
  Curves stay (decided).
- Enhanced with an Uber translucency on fibres loses its highlight under
  a hard light (0.44 - 0.47, section 10): Universal's own, with a
  transmission under a dielectric specular.
- Enhanced's level on Calib-Hair (1.22 - 1.65), left as Octane's.
- The real Morgan Hair's crown and middle (1.2 - 1.8 x Iray): its gloss
  and top coat on round curves.
- Clean references of the hair shader itself on prisms (#94).

## Process

- **Columns are places.** A horizontal strand's highlight depends on where
  along the frame it sits, so one material moved to another column is
  another measurement: a probe that put "the whole material, variant X" in
  column 2 and read it against Iray's column 1 showed a highlight coming
  back (0.97 against 0.42) that the export then did not have (0.40). Only
  the same cell compares; a variant goes into every column, or the cell
  keeps its own material.
- Look at the thick strand's picture before trusting sums: the tube
  against the line named the cause in one crop.
- When a shader's module fails in Iray, the same MDL building block in a
  shader that loads (Uber's weighted mixing) is a reference without
  restarts; equal sums on the shared columns prove the stand-in.
- A constant measured on one light is a fit to that light: the dome is
  the reference for an amount, a single sun for a direction. The share
  looked explained (flat faces) and still failed the second light.
