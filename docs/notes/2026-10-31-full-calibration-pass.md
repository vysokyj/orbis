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
