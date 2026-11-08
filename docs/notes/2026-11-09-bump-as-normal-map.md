# 2026-11-09 — Octane's bump against MDL's: height maps as normal maps (#85)

Goal: the skin's highlight peaks were 0.81 – 0.99 × Iray; Iray's bump
hardly widens a highlight, Octane's did (dual lobe with bump 0.77 – 0.86,
without 0.82 – 0.91). `BUMP_POWER` was fitted on Calib-CoatBump's diffuse
look. Decompose first: how does each renderer turn a height map into a
normal?

## Iray's side: MDL's reference

MDL SDK `base.mdl` (`file_bump_texture`, `apply_bump`, `blend_normals`):

- slope per axis = the smootherstep-weighted blend of central differences
  `h[i+1] − h[i−1]` of the neighbouring texels (texel i at u = i / W, no
  half-texel offset); a pattern of period 2 texels gives 0;
- `apply_bump`: n = normalize(t_u · du · (−f) + t_v · dv · (−f) + n), unit
  tangents — a tangent-space normal per texel;
- DAZ's Uber: a normal map is the base bump, the height map the detail
  blended onto it (`add_detail_normal` → `blend_normals`, reoriented normal
  mapping); the coat's normal (`calc_mix_layer_normal`) blends the coat's
  bump onto the base bump (the normal map, else the height map).

## Octane's side: probe on a flat plane

`octane-probe-bumpfreq.lua`: Calib-BumpRes's export with the ground plane's
material given a known height map (`bump-freq.py make`: sine stripes along
u, periods 2 / 3 / 4 / 6 / 8 / 12 / 24 / 48 texels in bands of 48, equal
slope amplitude, 16-bit) on `bump` (power 16), 3840 × 2160, Info kernel at
one sample a pixel without pixel filtering: shading normal without and
with bump, UVs. Image rows are lines of constant v, so u is fitted per row
from the 8-bit UV pass; slope = −n_x / n_y (exact at any tilt). ~2 s a
render.

First tries: on the spheres the Info kernel read the coat's
`coatingBump`, not the base's `bump` (the #75 note had it the other way),
and the UV seam sat in the middle of each sphere — moved to the plane.
Octane caches a texture by its package path: quit it between variants
that change the image under the same name.

Along a row at 10 px per texel the normal is **exactly flat inside each
texel and tilts hard (n_x ±0.4 – 0.6) in a one-pixel line at the texel
edges**: the height is read per texel (no interpolation), the slope is a
finite difference about a pixel wide.

| `bump-freq.py` | period 2 | 3 | 4 | 6 | 8 | 12 | 24 | 48 |
|---|---|---|---|---|---|---|---|---|
| height, 4 – 28 px / texel: RMS slope | 0.150 | 0.164 | 0.156 | 0.185 | 0.188 | 0.184 | 0.189 | 0.193 |
| share of samples untilted | 0.93 | 0.92 | 0.93 | 0.91 | 0.90 | 0.91 | 0.90 | 0.87 |
| height tiled 8 ×, 0.6 – 3.5 px / texel: RMS | 0.473 | 0.491 | 0.486 | 0.551 | 0.560 | 0.560 | 0.593 | 0.651 |
| height tiled 16 ×, 0.3 – 1.8 px / texel: RMS | 0.755 | 0.813 | 0.847 | 0.919 | 0.945 | 0.956 | 0.982 | 1.051 |
| normal map (MDL's bake, f 4): RMS | 0.041 | 0.098 | 0.174 | 0.251 | 0.285 | 0.318 | 0.345 | 0.354 |
| normal map: corr. with bilinear | 0.79 | 0.49 | 0.89 | 0.99 | 0.99 | 0.99 | 0.99 | 0.94 |

- Octane's bump keeps every frequency up to the texel's at full slope
  (period 2 as strong as 48); MDL's central difference rolls them off to 0
  at period 2. A skin's 4096 px pore map is mostly such detail: Octane
  scatters the normals where Iray's stay smooth — the widened highlight.
- No power fixes that: the spectrum differs, not the scale. `BUMP_POWER`
  fitted the low frequencies of Calib-CoatBump's dimples.
- Octane's normal map is bilinear (corr. 0.99) and, per #61's B5 / B6 /
  B8, read as Iray's at strength 1 — and a tangent-space normal is what
  `apply_bump` makes per texel.

## Rule: bake the height maps into the normal map

`src/detail_normal.rs`: per texel MDL's slope (luminance as read, v up the
image, wrap), n = normalize(−f du, −f dv, 1), a normal map (2c − 1, |z|)
as the base with the height map blended on by `blend_normals`, at the
larger map's size (a coarser height map keeps its slope per its own
texel), 8 bits. `base_detail_pins` (Universal, the skin's OpenPBR, the
metal layer) and `coat_bump_pins` (the coat's normal as the MDL's
`calc_mix_layer_normal`) write it on `normal`; maps of different
placements or unreadable ones fall back to Octane's bump at `bump_power`.
The export packs only the assets `project.ocs` names (the bake inputs had
gone in too: +138 MB on a skin scene; with the filter the baked PNGs add
~70 MB over the JPEG bump maps, the greyscale bump leaves Octane's memory).
The NVIDIA MDL presets' bumps (`mdl_metal`, `mdl_dielectric`) stay on
Octane's bump (#89).

## Against Iray

Octane 400 samples, old (master) and new binary, Iray references from the
cache.

Dual lobe (dlflat − gloss0, linear luminance, p99.5 in the face box
380 – 580 × 90 – 340), Octane / Iray, with bump / without:

| | Iray | old | new |
|---|---|---|---|
| Victoria | 0.116 / 0.131 | 0.76 / 0.83 | 0.83 / 0.83 |
| Gabriela | 0.131 / 0.135 | 0.85 / 0.89 | 0.88 / 0.90 |
| Nishi | 0.126 / 0.140 | 0.78 / 0.82 | 0.79 / 0.84 |
| Kayo | 0.054 / 0.077 | 0.88 / 0.94 | 0.97 / 0.94 |

The bump now costs the peak what Iray's does (ratio with ≈ without); the
remaining 0.83 – 0.94 is the lobe's own, present without bump too.

Full faces, p99.5 in the same box, Octane / Iray old → new: Angel 1.11 →
1.11, Victoria 0.95 → 0.96, Kayo 1.00 → 1.01, Gabriela 0.99 → 1.00,
Nishi 0.95 → 0.95, Twosret 0.89 → 0.91, Monique 0.93 → 0.95, Zurie 0.99
→ 1.01 — all within ±11 %. (The box differs from the 2026-11-07 note's,
and the walk / radius changes since moved the old numbers.) Skin colour
(`skin-compare.py`, 88 regions): median change 0.02; Monique's body Y
1.06 / 1.11 → 0.95 / 0.95 (chest, shoulder), neck 0.99 → 0.87.

Bump scenes against Iray:

- Calib-BumpRes, `bump-ladder-fit.py` residual per sphere (R1 R2 R3 R4 R5
  T1 T2 S1): old 1.18 1.22 0.23 0.77 0.80 0.34 0.56 0.17 → new 0.27 0.14
  0.09 0.09 0.25 0.18 0.16 0.10. R1 / R2's chaotic residuals (the #75
  note: "pixels don't line up") were Octane's per-texel steps.
- Calib-CoatBump, RMS of the grey image (1 px blur) per sphere vs Iray
  (B1 B2 B3 B4 B7 B8 B5 B6): old 2.1 3.5 6.5 9.9 2.5 1.8 3.2 1.6 → new
  2.0 2.5 2.2 3.3 2.4 1.7 1.9 1.4.

## Process

The probe answered in four 2-second renders what the power ladders of #61
/ #75 couldn't: they fitted a scale where the shape differed. Point
samples (one per pixel, no filter) and a plane (u linear per row) made the
per-texel structure visible; pixel-averaged renders hide it.
