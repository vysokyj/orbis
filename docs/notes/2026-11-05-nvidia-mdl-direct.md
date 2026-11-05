# 2026-11-05 — NVIDIA MDL examples, mapped from their MDL (#21)

Goal: every NVIDIA example on `calib/scenes/Calib-NVIDIA.duf` mapped from
its MDL source (DAZ's `shaders/iray/nvidia/*.mdl`, MDL SDK `base.mdl`)
rather than by fit, checked against the 5000-sample Iray reference
(fe12e27). Export from the library path, enhanced-final, Octane 256
samples for the variants, 1024 for the final; ~25 s per render.

Measure: mean linear luminance (gamma 2.2) over the sphere's disk (r 40
px), its centre (r 20 px), Octane / Iray; g/r and b/r of the disk. The
scene's floor is 1.04 × Iray, the level of matte surfaces. The script
lives in the session's scratchpad (centres x 423 + 153 k, y 560).

## What the MDL says that the export didn't do

- `metal_generic`: `weighted_layer(weight, simple_glossy_bsdf(tint:
  metal_color, roughness_u, roughness_v), …)` — a tinted Phong lobe, no
  Fresnel, weight clamped to 0..1 (the export clamped already).
- Its roughness, like every NVIDIA example's, goes straight into
  `simple_glossy_bsdf` — the same lobe and the same meaning as the Uber's
  roughness after its squaring. The Uber's Octane rule (GGX curve min(r +
  1.85 r², √r), energy-preserving GGX) was not applied: the metals used
  r as is, the dielectrics Octane's default BSDF at r as is.
- `file_bump_texture` is the Uber's bump function, but the MDL path set
  Octane's bump power = the amount, not `bump_power` (× 2 per unit for a
  512 px map, ÷ width, ÷ tiles).
- Metal Brushed's two roughness maps are the same texture: per texel u =
  0.02 + 0.4 l, v = 0.35 + 0.325 l (l 0.04 – 0.41 linear) — u 0.04 – 0.18,
  v 0.36 – 0.48. The export used one anisotropy, at a mid-grey texel.
- `base::architectural_gloss_to_rough` is in the open MDL SDK's
  `base.mdl` (github NVIDIA/MDL-SDK, `src/shaders/mdl/base/base.mdl`):
  √(2 · 2^(−4 − 14 g)), 0 at g = 1. The export's (1 − g)² was a guess;
  it differs badly below 1 (g 0.5: 0.25 v 0.03).

## Attempts (disk / centre)

| step | Brushed | Cast Iron | concrete | polished | porcelain | granite | resin | arch |
|---|---|---|---|---|---|---|---|---|
| before (HEAD) | 1.16 / 1.12 | 1.21 / 1.12 | 1.04 | 0.98 | 1.09 / 1.14 | 1.09 / 1.15 | 1.04 / 0.92 | 1.07 |
| curve + EP GGX + edge tint + bump_power, resin Beckmann √r | 1.26 / 1.25 | 1.11 / 0.98 | 1.00 | 0.95 | 1.09 | 1.09 | 1.02 / 0.83 | 1.06 |
| + per-texel anisotropy | 1.16 / 1.10 | 1.11 | | | | | | |
| + plain GGX on the metals (final) | 1.02 / 0.96 | 1.11 / 0.97 | 1.00 | 0.95 | 1.09 / 1.14 | 1.09 / 1.15 | 1.04 / 0.92 | 1.07 |

Metal Brushed variants (after the curve, mid-grey anisotropy): edge tint
white 1.27 (Octane ignores it here — kept as on the Uber's weighted gloss);
roughness × 0.64 (≈ r as is) 1.13; × 0.8 1.21; plain GGX 1.09;
anisotropy 0.9 1.09 with a thin streak. Only the per-texel anisotropy
drew Iray's thin vertical streak and the brush bands; then plain GGX
brought the level to the floor's. The energy-preserving GGX is the Uber's
rule (fitted there); `metal_generic`'s lobe has no multiscatter
compensation and loses light on its broad long axis (0.6 – 0.7 on the
curve), which plain GGX reproduces. This choice is measured, not derived.

Cast Iron: its sparkle at 256 samples was noise of the 2048 px bump
(bump off: a clean mirror; × 2: 1.20; 2048 samples: smooth, Iray's look).
The bump at `bump_power` (0.75 instead of 1.5) took it 1.21 → 1.11.
Plain or energy-preserving GGX alike (r 0.09 – 0.14).

Concrete / polished: half the bump (the old power) 1.03 / 0.98, the
`bump_power` 1.00 / 0.95 — but Iray's concrete shows more bump relief than
either, so the verified bump rule stays. Default BSDF v EP GGX on the
curve: ≤ 0.01 apart.

Porcelain / granite: unchanged by any of it (centre 1.14 – 1.15). The
diffuse under a full Fresnel gloss is brighter in Octane, as on the
Uber's glossy spheres (Calib-Glossy 1.02 – 1.16); not specific to the
MDL path, left.

Resin (coated rough glass, `scatter_reflect_transmit` tinted base):

| variant | disk | centre | p99 | g/r (Iray 0.22) |
|---|---|---|---|---|
| default BSDF, r, T × 0.8 (kept) | 1.04 | 0.92 | 1.27 | 0.24 |
| Beckmann √r, T × 0.8 | 1.02 | 0.83 | 1.26 | 0.28 |
| Beckmann √r, T × 0.9 | 1.12 | 0.96 | 1.28 | 0.24 |
| Beckmann √r, T = colour | 1.24 | 1.10 | 1.35 | 0.21 |
| Beckmann √r = 0.35 abs, T = colour | 1.26 | 1.08 | 1.52 | 0.21 |
| EP GGX curve, T × 0.8 | 0.99 | 0.82 | 1.43 | 0.26 |
| EP GGX curve, T = colour | 1.23 | 1.09 | 1.58 | 0.20 |

No variant gets Iray's level and hue together; Iray's sphere shows the
inverted horizon band sharper than any Octane one. Kept as it was.

Architectural: Calib-NVIDIA has glossiness 1 (mirror), unchanged by the
new curve; glossiness < 1 is now Iray's formula but not seen in a render.

## Final (Octane 1024 samples)

| sphere | disk | centre | g/r O v I | b/r O v I |
|---|---|---|---|---|
| Metal Brushed | 1.02 | 0.96 | 1.14 v 1.13 | 1.45 v 1.41 |
| Cast Iron | 1.11 | 0.97 | 1.12 v 1.12 | 1.47 v 1.45 |
| concrete | 1.00 | 1.00 | 1.04 v 1.04 | 1.20 v 1.18 |
| concrete polished | 0.95 | 0.93 | 1.13 v 1.13 | 1.38 v 1.37 |
| porcelain | 1.09 | 1.14 | 1.14 v 1.14 | 1.48 v 1.48 |
| granite black | 1.09 | 1.15 | 1.14 v 1.16 | 1.47 v 1.46 |
| resin | 1.04 | 0.92 | 0.24 v 0.22 | 0.34 v 0.32 |
| architectural | 1.07 | 1.06 | 0.55 v 0.57 | 0.74 v 0.76 |

## Lessons

- A metal sparkling at 256 samples is worth one 2048-sample render before
  blaming the bump or the roughness map.
- A map that drives two parameters at once (u and v) needs its derived
  quantity per texel; the mean of lobes is not the lobe of the mean.
- "Compiled into Iray" was wrong for `architectural_gloss_to_rough`:
  check the open MDL SDK's `base.mdl` first.

## Calib-NVIDIA2 (for the Iray batch)

Built from Calib-NVIDIA with `daz-apply-presets.dsa` (DAZ's NVIDIA MDL
Examples presets), the channel values edited in the .duf, then opened and
saved again by DAZ (that it opens and keeps the values), dumped, .mi
exported (`calib/references/mi/Calib-NVIDIA2.json`). Spheres, left to
right:

| node | material | why |
|---|---|---|
| A1 | asphalt (preset, 4 × 4 tiles) | not on Calib-NVIDIA |
| A2 | ceramic tiles small (preset) | not on Calib-NVIDIA |
| A3 | architectural, glossiness 0.3, diffuse 0.2 | r 0.083 by Iray's curve, 0.49 by the old guess |
| A4 | architectural, glossiness 0 | r 0.35 (the curve's top), guess 1.0 |
| A5 | architectural, glossiness 0.3, anisotropy 4 | mia semantic: u 0.165, v 0.041 |
| A6 | architectural, transparency 1, refr_gloss 0.2 | transmit lobe r 0.134, IOR 1.5 |
| A7 | resin, base colour grey 0.9 (0.79 linear) | resin's level without its hue |
| A8 | resin, roughness 0.02 | resin's colour without the blur |

A first build kept Calib-NVIDIA's concrete bump maps on A3 / A4 (the
preset left Bump Strength's image; DAZ passed it to Iray too): cleared.
The .mi confirms every mapped value (test
`nvidia_examples_match_daz_mi_export` now reads both scenes: 37 values).
Octane (512 samples) renders them plausibly: A6 refracts with `specular`
0. No Iray reference yet.

## OOT Hairblending (Calib-OOT, local, store content)

The brick graph ends in DAZ's own Uber stack (`add_top_coat` →
`add_uber_volume` → `add_geometry` → `normalized_base`, from DAZ's .mi of
Calib-OOT). The overlay is NVIDIA's `blend_colors(base map, overlay map,
normal, weight)` and the weight is `pass_float(0)`: the Overlay Alpha
Mask's value (black in the presets), its map not passed at all. The
export blended by the map.

| Calib-OOT, Octane / Iray | crown | side | fringe |
|---|---|---|---|
| blended by the alpha map (before) | 0.81, r/g 1.02 v 1.05 | 0.89 | 0.64, r/g 1.20 v 1.23 |
| overlay off (the .mi's weight 0; now) | 0.96, r/g 1.09 | 0.92 | 0.79, r/g 1.27 |

Also from the .mi: `color_scale` 0.505 on the diffuse map = the channel's
0.733 decoded; per-map tiling (`image_modification`: diffuse 0.6 on u,
gloss weight 1.2) that the export ignores everywhere: #77. Iray's strands
look paler and softer; the tiling and the glints are the likely rest.

## Calib-NVIDIA2 against Iray (evening)

The Iray reference (2835a5c, 5000 samples) came out from another view
than the export: DAZ had saved the scene without `current_camera` (it
was built and saved while looking through the Perspective View), so the
Iray Server rendered that view, which a .duf doesn't store, and `orbis`
took the first camera node. The view was recovered from the reference:
a pinhole fit (Gauss-Newton with damping, numpy) to the eight sphere
centres and radii, the back floor corners and the left floor edge;
checked first on an Octane render of Camera-1, whose known pose it
reproduced (centres x 424 + 153 k, y 561). Result: position (142.9,
188.1, 364.6) cm, X −13.96°, Y 21.99°, focal 89.1 mm, residual 1.3 px.
The scene stays as DAZ saved it: its .mi fixture checks Camera-1 (65 mm),
and a scene edited by hand would no longer be the one the fixture and
the reference came from. The comparison ran on a scratch copy whose
Camera-1 carries the fitted view (`current_camera` set, same dump);
the next Calib-NVIDIA* scene must be saved with its camera current
(check `current_camera` before queueing).

Measure as above (disk r 0.75 of the sphere's radius, centre 0.35),
Octane 512 samples for variants, 1024 for the final. Variants through a
temporary per-material pin override (an env var in the export, removed).

| sphere | disk | centre | g/r O v I | b/r O v I |
|---|---|---|---|---|
| A1 asphalt | 1.05 | 1.02 | 1.06 v 1.10 | 1.31 v 1.29 |
| A2 ceramic tiles small | 1.04 | 1.04 | 1.12 v 1.11 | 1.27 v 1.26 |
| A3 arch glossiness 0.3 | 0.90 | 1.01 | 1.16 v 1.13 | 1.50 v 1.40 |
| A4 arch glossiness 0 | 1.01 | 1.01 | 1.13 v 1.13 | 1.39 v 1.40 |
| A5 arch anisotropy 4 | 0.95 | 1.06 | 1.13 v 1.12 | 1.45 v 1.40 |
| A6 arch glass, refr_gloss 0.2 | 1.46 | 1.42 | 1.16 v 1.16 | 1.52 v 1.53 |
| A7 resin, grey base | 1.02 | 0.93 | 1.16 v 1.15 | 1.51 v 1.50 |
| A8 resin, r 0.02 | 0.91 | 0.92 | 0.14 v 0.12 | 0.23 v 0.19 |

(floor 1.05; final, with the resin rule below.)

### Resin

| variant | A7 grey (r 0.25) | A8 (r 0.02), g/r | Calib-NVIDIA red (r 0.25), g/r |
|---|---|---|---|
| default BSDF, r, T × 0.8 (before) | 1.00 / 0.86 | 0.71, 0.19 | 1.04 / 0.93, 0.24 |
| default BSDF, r, T = colour | 1.46 / 1.28 | 0.93, 0.14 | 1.31 / 1.25, 0.18 |
| plain GGX on the curve, T = colour | 1.32 / 1.23 | 0.93, 0.14 | 1.19 / 1.06, 0.21 |
| Beckmann √r, T = colour | 1.36 / 1.27 | 0.93, 0.14 | 1.23 / 1.10, 0.20 |
| Beckmann √r, T = colour, specular white | 1.38 / 1.27 | 1.14, 0.23 | 1.61 / 1.34, 0.35 |
| **Beckmann √r, T = colour × (1 − 0.64 r)** | 1.02 / 0.93 | 0.91, 0.14 | 1.05 / 0.88, 0.25 |

(Iray g/r: A8 0.12, Calib-NVIDIA 0.22.) A7's level against T: 1.00 at
0.8, 1.46 at 1 — Octane tints at each crossing (a + b·T², a ≈ 0.18), as
the MDL's tinted lobe does on the way in and out. With the full colour
the smooth resin is right in level and hue, so the hue rule is the MDL's;
the rough one is ~1.3 × too bright under every BSDF, so the excess is
the level of the rough lobe, not its shape. Iray's `simple_glossy_bsdf`
is Phong with a v-cavities shadowing term and no multiple-scattering
compensation (MDL spec 21.1.3), which loses light as roughness grows;
from A7 and the red resin the per-crossing albedo at r 0.25 is 0.82 –
0.87 (Beckmann), taken as 1 − 0.64 r. The specular (the base's own
reflection) carries the tint too: white made the red resin pink (g/r
0.35). Beckmann √r replaces the default BSDF as the rough-glass rule the
spec already has; at these levels the BSDFs differ by < 0.1.

### Architectural

- A4 (glossiness 0, r 0.35) and A5 (anisotropy) agree.
- A3 (glossiness 0.3, r 0.083 by `architectural_gloss_to_rough`): Iray's
  reflection is visibly blurrier. Octane's GGX roughness pin 0.096 (now)
  0.90 ×, 0.2 0.99 ×, 0.3 1.09 ×; the look sits between 0.2 and 0.3 —
  an MDL roughness of ~0.18, 2.2 × the formula's.
- A6 (refr_gloss 0.2, r 0.134; Beckmann pin 0.366): 1.46 × Iray in every
  third of the disk (top 1.54, middle 1.42, bottom 1.54). Iray's interior
  is an even grey, no inverted environment; Octane shows the inverted
  sky in the lower half. Coating off 1.38, specular white 1.50, IOR 1
  (no refraction) 1.53, roughness pin 0.5 1.46, 0.6 1.41, 0.7 1.33, 0.85
  1.15 — only the last looks like Iray (α 0.72, 5 × the formula's).
- `architectural_gloss_to_rough` in the MDL SDK's `base.mdl` is the
  formula in use (re-read). The two misses don't share a curve (A6 would
  need more roughness at refr_gloss 0.2 than glossiness 0 gives), so no
  rule from them. Iray batch: needs a series (reflection at glossiness
  0.15 / 0.3 / 0.5 / 0.7, glass at refr_gloss 1 / 0.6 / 0.2 with
  reflectivity 0) to see Iray's curve for each lobe.

## Calib-NVIDIA3 (for the Iray batch)

Built from Calib-NVIDIA2's .duf by a script (A3's architectural material
copied with its glossiness changed, A6's glass with its refr_gloss and
reflectivity changed, spheres relabelled, `current_camera` set), then
opened and saved by DAZ through `daz-apply-presets.dsa`. That script
now makes the scene's first camera the viewport's before saving. A
headless instance has no `MainWindow` and dropped `current_camera`
again; a normal instance (`HEADLESS=0`) kept it. Dumped, .mi exported
(`calib/references/mi/Calib-NVIDIA3.json`, Camera-1 at 65 mm; the test
`nvidia_examples_match_daz_mi_export` reads it too). Left to right
(x −175 … 175 as before):

| node | refl_gloss | refr_gloss | reflectivity | transparency |
|---|---|---|---|---|
| A1 | 0.15 | 1 | 0.6 | 0 |
| A2 | 0.3 | 1 | 0.6 | 0 |
| A3 | 0.5 | 1 | 0.6 | 0 |
| A4 | 0.7 | 1 | 0.6 | 0 |
| A5 | 1 | 1 | 0 | 1 |
| A6 | 1 | 0.6 | 0 | 1 |
| A7 | 1 | 0.2 | 0 | 1 |
| A8 | 1 | 0.2 | 0.6 | 1 |

A2 repeats Calib-NVIDIA2's A3 and A8 its A6 (anchors between the two
references); A5 – A7 are the glass without the reflection on top.
Iray batch: needs Calib-NVIDIA3, 5000 samples.

## Open (#21)

- Architectural glossiness < 1 (A3, blurrier in Iray) and glass (A6,
  1.46 ×): needs Calib-NVIDIA3's Iray reference.
- Resin's lobe albedo is measured at r 0.02 and 0.25 only.
- OOT hair: per-map tiling (#77); a non-black Overlay Alpha Mask value
  (the colour → float reduction is a guess, no scene has one).
