# 2026-11-11 — Calib-Strand against Iray: fibres, ribbons, a false mirror (#79)

Resumes [strand shapes](2026-11-11-strand-shapes.md) once the Iray batch
was in (4e8780f: Calib-Strand 5000, -T2 5000, -T2-Dome 5000, -T3 2000,
-Dome 2000, -Side 2000 samples, Iray Server). Octane enhanced-final, 256
samples, 1920 × 1080. Rows: A thick strand (Line Width 20), B mesh tube
(radius 1 cm), C / D 3000 thin strands horizontal / vertical. Columns A / B:
diffuse, white gloss iso, white gloss aniso, Morgan's gloss; C / D: diffuse,
white aniso, Morgan's gloss, Morgan's base without coat.

Measure: per object, the radiance (linear) summed across the strand and
averaged along it (the caps cut off), so a strand twice as wide counts
twice; Iray's alpha finds the objects. B / R = blue over red of that sum.
Script in the session's scratch (`strand.py`), not kept.

## 1. Iray's fibre is a round tube one Line Width across

| Calib-Strand, Iray | A fibre | B mesh tube |
|---|---|---|
| diffuse, sun | 1.70 | 1.73 (same profile across) |
| white gloss iso, sun | 0.61 | 0.60 |
| diffuse, dome | 0.519 | 0.521 |
| white gloss iso, dome | 0.130 | 0.130 |
| Morgan gloss, dome | 0.70 (B / R 0.86) | 0.70 (0.85) |

15 px tall both. The fibre = a mesh tube of diameter = Line Width (mm), as
the `.mi`'s radius said. Anisotropic gloss differs (A3 0.93 vs B3 0.64):
the fibre's tangent runs along it, the tube's from its UV.

## 2. Octane's curve against Iray's fibre

The curve is 2× the width (`LINE_WIDTH_SCALE`), so 2.0 is a match:

| Calib-Strand row A, Octane / Iray | sun | dome |
|---|---|---|
| diffuse | 1.97 | 2.03 |
| white gloss iso | 1.70 | 2.05 |
| white gloss aniso | 1.79 | 2.09 |

The highlight sits at the same place across the strand (a third of the
radius above the axis in both). Per unit width the curve's BSDF is Iray's
fibre; the sun's highlight is 0.85 – 0.9 of it.

## 3. Ribbons (sides 2) and prisms (3) are not fibres

Under the sun Iray's ribbons (-T2) and prisms (-T3) with gloss only are
**black** (A2 – A4, C2, C3: 0.000): their flat faces mirror the sun away
from the camera; Octane's round curves (and Iray's fibres) show a band.
Under the dome the ribbons reflect another part of the sky, warmer:

| B / R, dome | fibre (-Dome) | ribbon (-T2-Dome) |
|---|---|---|
| A2 white gloss | 1.54 | 0.95 |
| A4 Morgan gloss | 0.86 | 0.47 |
| C3 Morgan gloss, patch | 0.81 | 0.44 |

Morgan's whole material without its coat (column 4 of C / D), Octane against
Iray:

| | fibre: O / I, B / R I vs O | ribbon (T2): O / I, B / R I vs O |
|---|---|---|
| C, sun | 0.51, 0.49 / 0.49 | 0.53, 0.36 / 0.49 |
| D, sun | 1.30, 0.53 / 0.49 | 0.54, 0.38 / 0.49 |
| C, dome | 1.24, 0.75 / 0.73 | 0.64, 0.49 / 0.73 |
| D, dome | 1.18, 0.57 / 0.61 | 0.51, 0.55 / 0.61 |

On fibres Octane's hue is Iray's; on ribbons Iray is warmer and twice as
bright. So the "warmer, placed differently" of Morgan Hair (ribbons) is the
strand's shape, as the `.mi` suggested, and no BSDF pin on a round curve
can reach it.

## 4. The false mirror (fixed)

Morgan's gloss alone (column 4 of A / B, C3, D3) rendered too bright and too
pale on every shape, mesh tubes included — so not the shape. The exported
material was a **metal** coloured by Glossy Color: the export's oldest rule
made a Base Mixing 1 material with a black base and a strong gloss a
"mirror". The `.mi` hands Iray no metal there: `glossy_reflectivity` 0.488
(F0 = Glossy Specular), the specular's hue as a directional tint — what
the F0 rule (`specular_ior`, #52) already does for any other base. Rule
removed:

| Octane / Iray (B / R Iray → Octane) | metal | dielectric (F0) |
|---|---|---|
| B4 tube, dome | 2.27 (0.85 → 1.17) | 0.98 (→ 0.83) |
| A4 fibre, dome (2× wide) | 4.63 (0.86 → 1.16) | 1.99 (→ 0.83) |
| C3 patch, dome | 2.61 (0.81 → 0.95) | 0.94 (→ 0.84) |
| D3 patch, dome | 2.06 | 0.81 |
| C3 patch, sun | 2.39 (0.48 → 0.72) | 0.88 (→ 0.47) |

Morgan Hair's own Strands material has a diffuse, so it never took the
metal; the fix matters for black-based glossiness-mode materials (glossy
hair layers, brows, lacquers).

## Still open

- **B4 under the sun**: Iray's mesh tube with Morgan's gloss is black
  (0.02) where its white aniso twin B3 has its highlight (0.64) and the
  fibre A4 a strong one (1.78); Octane draws one (0.86). Same roughness,
  anisotropy and UV as B3; the MDL's directional tint and Color Effect
  don't touch the lobe's direction. Unexplained; a mesh tube with a
  glossiness-mode gloss is no hair case, so left.
- **Ribbon frames of generated hair**: an `.mi` of the free dForce Mohawk
  at sides 2 (its preset rewritten as a scene with a camera — saving it
  from a headless DAZ segfaulted) shows them: each render hair's ribbon
  faces the scalp at its root and keeps that frame along the strand
  without twist (`docs/daz/iray.md`). Enough to export ribbons; whether to
  is the question on #79: Iray's flat ribbons are a tessellation artefact
  (black under a sun from most views, mutual reflections), Octane's round
  curves what a hair is.
- **The 2× width** is still a fit: Iray's fibre and ribbon are 1×. The
  `.mi` export writes the viewport's render-hair density (Mohawk: 1504
  strands, the dump 29803), so it can't say how many hairs Iray renders.

## Process

- Iray's references are RGBA on a transparent background: the alpha is the
  object mask; Octane's black background needs Iray's boxes.
- A scene preset (`scene_subset`) becomes a scene by type `scene`, a camera
  node from a calib scene and `current_camera`: `daz-export-mi.dsa` then
  writes its `.mi` headless. `Scene.saveScene()` with dForce hair crashed
  the headless DAZ.
- Timings: Octane 256 samples ≈ 30 s per scene (six scenes), Mohawk `.mi`
  export and dump ≈ 10 s each.
