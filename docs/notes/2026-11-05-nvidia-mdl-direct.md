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

## Open (#21)

- Resin's level vs hue (fitted × 0.8).
- Calib-NVIDIA2's Iray reference (5000 samples): architectural
  glossiness / anisotropy / glass, asphalt, ceramic tiles, resin variants.
- OOT hair: per-map tiling (#77); a non-black Overlay Alpha Mask value
  (the colour → float reduction is a guess, no scene has one).
