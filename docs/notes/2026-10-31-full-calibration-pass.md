# 2026-10-31 — Full calibration pass against the 5000-sample references

All 222 calibration scenes were re-exported with the current code
(39 skin-variant dumps made first) and rendered in Octane X at 400 samples
(01:41 – 06:11, about 50 s a scene, VDB up to 9 min). They were compared
with the new Iray Server references at 5000 samples
(`calib-compare-all.py`; skins by face region, `skin-compare.py`). The
report is in `~/Library/Caches/orbis/calib/octane-1031/report.md`, with
side-by-sides in `compare/`.

## Within ±5 %

- Lights: Point, Spot centres, Distant, Rect, Disc, Sphere, Emissive (all
  0.99 – 1.02); Mode SunSky / DomeOnly 0.97 – 1.04.
- Headlamp 1.00 – 1.05. `light-on` reads 0.62, but both renders clip to
  white there, so the number means nothing.
- Thin, Leaf, VDB, the hair scenes and HDRI. Their frames are 0.97 – 1.04,
  but a whole-frame number hides local differences.

## Off, by size

| Scene | Octane / Iray | What the renders show | Issue |
|---|---|---|---|
| Calib-TopCoat | T3 0.10, T4 0.05, T7/T8 2.1 – 2.6 | Top Coat Layering Mode weighted (1): Iray a clear mirror coat over the red, Octane plain red; curve (3) too bright | #25 |
| Calib-ColorEffect | C8 1.35, C3 0.76 | C8 (Base Mixing 1, coloured Glossy Specular): Iray pinkish red with a red reflection, Octane blue | #27 |
| Calib-SSSColor | 1.03 – 3.98 (sphere boxes are Calib-Glossy's; it has 8 spheres) | Octane's eight spheres look alike: SSS Color / Mode / Tint not mapped on props | #28 leftover |
| Calib-ThinFilm | 1.00 – 1.82 | Iray's iridescent rims missing | #31 |
| Calib-Glass | spheres 4 – 5 off | Iray's 5th sphere is invisible, Octane's faint; the blue 4th darker in Iray | — |
| Spot falloff | s60-e1 0.79 – 0.86 at 18 – 26°, edge 1.96; s60-e16 cut earlier | the cone's edge | — |
| SunSky noon haze 4 | 0.89 | haze | — |
| Tone Crush 0.3 | spheres up to 1.34 | crush | #38 |

## Skins (Octane / Iray, median over forehead, cheek, nose, neck, chest)

| Figure | front R G B | HDRI R G B | luminance range |
|---|---|---|---|
| Victoria | 1.04 0.98 0.96 | 1.11 1.08 1.24 | 0.94 – 1.13 |
| Gabriela | 1.01 0.90 0.88 | 1.12 0.92 1.00 | 0.79 – 1.03 |
| Nishi | 1.04 0.95 0.97 | 1.23 1.07 1.25 | 0.89 – 1.29 |
| Angel | 1.10 1.10 1.06 | 1.20 1.15 1.17 | 1.03 – 1.37 |
| Kayo | 1.06 0.97 0.93 | 1.21 1.00 1.04 | 0.77 – 1.52 |
| Twosret | 0.92 0.90 1.00 | 1.04 1.00 1.26 | 0.60 – 1.39 |
| Monique | 1.03 1.14 1.45 | 1.16 1.34 1.77 | 0.37 – 1.76 |
| Zurie | 0.89 0.75 0.64 | 1.00 0.84 0.83 | 0.70 – 1.02 |

Pale and mid skins are mostly within ±15 % under the front light. The
HDRI tips them blue (B 1.2 – 1.25). Monique (dark) comes out too blue and
Zurie (pale grey) too red, as in #19. The back light leaves the face too
dark for stable ratios.

## Top Coat layering modes (#25)

The installed library (26 814 files that carry a top coat channel)
counts materials with the coat on by mode: Reflectivity 6230, Fresnel 5561,
Weighted 2176 and Custom Curve 196. Base Mixing 0 dominates in every
mode.

In Iray (Calib-TopCoat; median R G B, linear):

| Sphere | Iray | Octane before |
|---|---|---|
| T1 Reflectivity 0.5 | 0.053 0.008 0.010 | 0.060 0.008 0.012 |
| T3 Weighted 1 | 0.157 0.187 0.255 (chrome mirror) | 0.055 0.008 0.012 |
| T4 Weighted 0.4 | 0.082 0.074 0.101 | 0.045 0.004 0.005 |
| T7 Custom Curve 0.04 / 1 / 5 | 0.036 0.008 0.011 | 0.045 0.008 0.012 |
| T8 Custom Curve 0.3 / 0.3 | 0.033 0.008 0.011 | 0.049 0.022 0.030 |
| T5 / T6 Fresnel 1.5 / 2 | 0.027 0 0 / 0.024 0 0 (no coat) | 0.037 / 0.034, coated |

Weighted, a Mix of a top-coat-coloured metal and the uncoated surface:
T3 1.03 – 1.04, T4 1.04 – 1.05 × Iray. Iray's Fresnel spheres show no
reflection at all, which doesn't fit DAZ's MDL (`fresnel_layer`). Since
5561 materials use it, it waits for a check on a real material in the next
Iray Server batch.

## Colour effects (#27)

Calib-ColorEffect per sphere (mean R G B, Octane / Iray):

| Sphere | before | after |
|---|---|---|
| C1 Base Color Effect 0 | 0.87 1.00 1.01 | same |
| C2 Base Color Effect 1 | 1.22 1.11 0.82 | same |
| C3 Base Color Effect 2 | 0.48 0.85 0.99 | 1.01 1.01 0.95 |
| C4 / C5 glass, Glossy Color Effect 0 / 1 | 0.86 1.28 0.16 | same |
| C6 / C7 top coat, Color Effect 0 / 1 | 0.98 – 1.13 | same |
| C8 Base Mixing 1, Glossy Specular 0.9 / 0.3 / 0.2 | 0.35 2.61 4.39 | 1.03 2.62 4.38 |

- C3 needed Base Color Effect 2's own diffuse / transmission shares.
  Tried 0.7 / 1.5 (0.94 0.82 0.70), 0.9 / 1.4 (0.84 0.91 0.90),
  0.95 / 1.7 (1.20 1.07 0.95); kept 0.95 / 1.55.
- C8: Glossy Specular's hue was divided by its luminance and clipped at 1,
  so red turned white. Over its strongest channel (IOR from that channel
  too) the reflection is Iray's red. The blue base still shows through
  2.6 – 4.4× too much in green and blue. Calib-SpecMode's greys didn't
  move (1.00 – 1.13).
- In Iray Glossy / Top Coat Color Effect 0 vs 1 change nothing visible on
  these spheres (C4 = C5, C6 ≈ C7). C4 / C5's blue 0.16 is the red glass's
  near-zero blue; their absolute B is 0 in both.
- One batch render of ColorEffect and SpecMode together came out with
  every ColorEffect sphere default grey (materials not applied). Rendered
  alone, the same export was right. Octane's import into its open project
  probably went wrong once; watch for it.

## Props SSS (#28's leftover)

Translucent props passed Translucency Color × w as diffuse transmission,
whatever their volume, so the eight Calib-SSSColor spheres rendered alike.
Their Iray medians ranged 0.064 – 0.216 in red. The transmission is now ×
`volume_factor` × `PROP_VOLUME_LEVEL` when the solid is closed, not thin
and has a Transmitted Color.

| Level | V1 chrom | V2 grey | V3 blue | V4 deep | V5 mono | V6 mono 0.5 | V7 tint |
|---|---|---|---|---|---|---|---|
| none (before) | 1.35 | 1.05 | 1.13 | 2.68 | 1.40 | 1.69 | 2.35 |
| 0.6 | 0.71 | 0.78 | 0.84 | 0.84 | 1.20 | 1.09 | 1.08 |
| 0.7 (kept) | 0.84 | 0.97 | 1.05 | 0.93 | 1.54 | 1.40 | 1.34 |
| 0.8 | 1.00 | 1.19 | 1.30 | 1.06 | 1.95 | 1.76 | 1.65 |

These are median luminance, Octane / Iray. Octane responds steeply to the
transmission colour. At 0.7 red sits 0.71 – 0.96 and green / blue 0.87 –
1.21 on the chromatic spheres. Calib-SSS (no volume) didn't change.
Calib-Eye, Calib-Hair-socket, Calib-Leaf and Victoria's front skin are
unchanged within noise. Calib-Eye's first try in a batch rendered empty
(#58); alone it was the same as before.

## Skin hue (#19)

Octane / Iray, median over face regions, R G B, front light. The rows are
before; dark gain 0.4 with a floor 0.45; floor 0.45 (kept); floor 0.3.

| Figure | before | gain 0.4 | floor 0.45 | floor 0.3 |
|---|---|---|---|---|
| Zurie | 0.89 0.75 0.64 | 0.77 0.75 0.69 | 0.81 0.83 0.80 | 0.86 0.78 0.72 |
| Victoria | 1.04 0.98 0.96 | 1.08 0.96 0.91 | 1.00 1.02 1.07 | 1.03 1.00 1.04 |
| Gabriela | 1.01 0.90 0.88 | 0.97 0.81 0.75 | 0.97 0.95 0.97 | 1.01 0.93 0.91 |
| Angel | 1.10 1.10 1.06 | 1.06 1.10 1.07 | 1.05 1.17 1.24 | 1.08 1.13 1.14 |
| Nishi | 1.04 0.95 0.97 | 1.04 0.79 0.79 | 1.01 0.95 1.00 | 1.03 0.95 0.97 |
| Kayo | 1.06 0.97 0.93 | 1.02 0.78 0.80 | 1.05 0.97 0.97 | 1.06 0.94 0.97 |
| Twosret | 0.92 0.90 1.00 | 0.80 0.73 0.79 | 0.92 0.90 1.00 | 0.91 0.90 1.00 |
| Monique | 1.03 1.14 1.45 | 0.82 0.91 1.18 | 1.03 1.14 1.45 | 1.03 1.14 1.45 |

The worst channel's |log| had a median of 0.10 / 0.24 / 0.085 / 0.102
across the columns. The baked colours (from the exports' `_skin.png`,
median) against Iray's rendered skin explain part of it.

| Figure | baked R G B | Iray R G B |
|---|---|---|
| Victoria | 0.542 0.198 0.092 | 0.41 0.142 0.090 |
| Zurie | 0.190 0.109 0.066 | 0.208 0.133 0.114 |
| Monique | 0.038 0.013 0.007 | 0.060 0.025 0.015 |
| Twosret | 0.078 0.013 0.005 | 0.109 0.038 0.022 |

Octane's render over its bake rises steeply as a channel gets darker
(roughly bake^0.6). Zurie's blue is wrong already in the bake. Monique and
Twosret have similar dark bakes, but only Monique comes out blue.
