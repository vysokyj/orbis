# 2026-10-30 — Skin volume: SSS Color, SSS Mode, SSS Amount (#28)

Follows the first look in [2026-10-30-dual-lobe.md](2026-10-30-dual-lobe.md):
the full-quality Iray references (Iray Server, 5000 samples) of the
Calib-Skin SSS variants are in for Victoria 8, Monique and Zurie.

## Iray: what each variant does

Median per channel over forehead, cheeks, neck, chest and shoulder, variant ÷
base render (front light):

| variant | Victoria | Monique | Zurie |
|---|---|---|---|
| SSS Color 0.7 grey | 1.58 1.00 1.10 | | |
| SSS Color (0.6, 0.75, 0.95) | 1.61 0.93 0.61 | 1.50 0.94 0.66 | 1.21 0.98 0.94 |
| SSS Color (0.995, 0.95, 0.95) | 0.60 0.53 0.61 | 0.64 0.59 0.66 | 0.87 0.89 0.94 |
| mono, Amount 2 | 1.63 1.18 1.29 | 1.53 1.17 1.22 | 1.21 1.04 1.05 |
| mono, Amount 0.5 | 1.52 0.86 0.95 | | |
| SSS Reflectance Tint blue / red | 1.00 1.00 1.00 | 1.00 | 1.00 |

Back light the same within ~0.1. All three are Iray Uber skins with the
same Transmitted Color and SSS Color; Zurie has w 0.65 and half the
distances (0.063 / 0.008 cm) where the others have w 0.9 (0.125 / 0.015).

Zurie's weaker response is her diffuse share: with the w 0 render as the
diffuse layer ((1 − w)^1.3 of it), it is 0.56 – 0.65 of her base render
against 0.11 – 0.20 for Victoria. The subsurface part alone, variant ÷
base: Victoria blue 1.69 / 0.92 / 0.52, deep 0.55 / 0.44 / 0.52, mono
1.71 / 1.21 / 1.35, mono 0.5 1.59 / 0.83 / 0.94; Monique within ~0.1 of
that; Zurie 1.48 / 0.95 / 0.82, 0.71 / 0.71 / 0.82, 1.48 / 1.09 / 1.13.

## Model

Single-scattering albedo α = σs / (σs + σa) from DAZ's MDL (σa =
−ln Transmitted Color / d_t; σs = −ln SSS Color / d_s chromatic, SSS
Amount / d_s mono). Chromatic α doesn't depend on whether the colours are
linearised (both σ scale alike); mono does — with linear colours σs =
Amount / d_s fits as is (sRGB needed a 0.4 fudge). Victoria's α: base
0.89 / 0.81 / 0.69, blue 0.99 / 0.77 / 0.29, deep 0.64 / 0.37 / 0.29,
mono 1.00 / 0.91 / 0.88. Zurie's distances give the same α.

Jensen's diffusion reflectance Rd(α) (dipole, η 1.4) ratio to the base,
to a power k, against the subsurface factors: k 0.4 16 % RMS, **0.5**
fits Victoria / Monique within ~10 % (Zurie off by up to 0.3 in blue), 1.0
(Rd itself) 56 %.

## Export (Enhanced)

`skin_albedo`'s subsurface term × (Rd(α) / Rd(α₀))^0.5 per channel, α₀
Victoria 8's volume. Octane renders of the variants (200 samples),
Octane / Iray relative to the figure's base render:

| scene | before | after |
|---|---|---|
| Monique mono | 1.00 1.64 1.54 | 1.02 1.00 0.91 |
| Monique SSS blue | 0.67 1.14 1.61 | 1.02 1.03 1.24 |
| Monique SSS deep | 1.60 1.82 1.56 | 1.19 1.34 1.28 |
| Victoria mono | 0.69 1.29 1.11 | 0.83 1.04 1.03 |
| Victoria mono 0.5 | 0.73 1.71 1.50 | 0.86 1.00 0.96 |
| Victoria SSS blue | 0.61 1.08 1.68 | 0.86 0.95 0.93 |
| Victoria SSS deep | 1.64 1.90 1.65 | 1.17 1.02 0.92 |
| Victoria SSS grey | 0.62 1.00 0.90 | 0.84 0.97 0.96 |
| Zurie mono | 0.61 0.83 0.84 | 1.04 1.04 1.01 |
| Zurie SSS blue | 0.81 1.00 1.04 | 1.07 0.95 0.81 |
| Zurie SSS deep | 1.12 1.12 1.06 | 0.96 0.89 0.88 |

The mono variants went through the baked rule (before: the older tint,
which ignores the volume). The Genesis 8 base skin on Calib-Eye (mono,
Base Color Effect 1, SSS Direction −0.65, w 0.5) did not: baked with the
factor 1.49 – 1.74 / 1.40 – 1.73 / 1.68 – 2.05 × Iray on brow, cheek,
temple and lid, the older rule 0.98 – 1.04 / 1.00 – 1.09 / 1.19 – 1.39. So
mono skins are baked only with Base Color Effect 0; which of the three
differences matters needs Iray variants of the G8 base skin.

Red stays 0.83 – 0.86 on Victoria's red-shifting variants: Octane's random
walk carries less of a red gain than the factor asks.

## Not done

- Calib-SSSColor (the same volume on Uber spheres, props → Universal):
  Tint darkened and mono differed there, unlike on the figures. Props
  don't take the skin path; left open in #28.
- SSS Direction isn't calibrated (all variants at g 0); the reduced albedo
  uses it as diffusion theory does.

## Tooling

- `orbis export --dump` moves the dump into the cache: the base scenes'
  dumps went to the first variant's name. Export variants from a copy.
