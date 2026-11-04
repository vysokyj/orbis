# 2026-11-04: Blended Dual Lobe hair, root to tip (#42)

Enhanced + Final, 400 samples. Scenes: Calib-Mi-Hair, Calib-Mi-Hair2 (.mi
only; spheres with the Blended shader from dForce Starter Essentials),
Calib-Hair (local, a store hair) and three local copies: -tip, -tipbg, -hl.

## 1. What DAZ hands Iray

`dual_lobe_hair.mdl` (DAZ app bundle): a highlight glossy layer
(`highlight_weight`, `hightlight_color`, `highlight_roughness`²) over a
hair-coloured glossy layer (`hair_weight` = Glossy Layer Weight,
`hair_color`, `base_roughness`²) over the base material, here a
`diffuse_transmission_bsdf` tinted by the transmission colour. Each glossy
layer's normal is tilted ±`separation` along the tangent.

**The tip colours do reach Iray.** The earlier fixture read only the
`base` argument of `replace_scattering`. The graph's real scattering is its
`material_surface_scattering`: a `df::clamped_mix` of two `dual_lobe_hair`
instances. Component 1 holds the Tip colours (Hair Tip, Tip Highlight, Tip
Transmission), weighted by `get_line_root_to_tip_blend(bias, gain)` on the
strand's line coordinate. Component 2 holds the Root colours at weight 1,
clamped to the rest. `mi-oracle.py` now resolves arrays of references
(`"components" [ = "…", = "…" ]`). Calib-Mi-Hair's S2 (root 0.3 / 0.2 /
0.1, tip 0.8 / 0.7 / 0.5) shows both colours, linear.

Calib-Mi-Hair2 (`make-mi-shader-scenes.py`) settles the rest:

| sphere | set | in the .mi |
|---|---|---|
| S1 | bias 0.2, gain 0.7 | `get_line_root_to_tip_blend("bias" 0.2, "gain" 0.7)` |
| S2 | Highlight Weight 0, `strength` 0.5 | highlight_weight 0 in both lobes; `strength` nowhere |
| S3 | Highlight Weight 1, highlight_roughness 0.5, separation 0 | as is |
| S4 | bias 0.8, gain 0.3, Glossy Layer Weight 0 | as is |

At 0.5 / 0.5 the blend function has no arguments in the .mi: they are the
MDL's default. `basic.mdl`: Schlick's fast bias, then a gain built on it,
both clamped to (0, 1). At 0.5 / 0.5 the blend is linear.

`src/iray.rs` `blended_hair` now carries the tip lobe and the blend
(`root_to_tip_blend`). The fixture test runs both scenes.

## 2. Octane: a gradient along the strand

Octane's Hair has no root / tip of its own. The W Coordinate texture (104)
gives each strand 0 → 1 (Octane makes it when the geometry has none).
Probes (pins of 147 / 104 / 49 / 38; a Gradient saved as a project) showed
how the Gradient's control points are written: `controlpoints` = n, dynamic
pins `Position k` (float node 6, value type 9) and `Value k` (a texture).

Rule: albedo = Gradient(W) from the root's albedo to the tip's (the same
0.046 / 0.110 colour / transmission shares), linear. When bias / gain bend
the blend, 15 inner control points sample it.

- Even spacing in t was up to 9 % off the curve at bias / gain 0.2 – 0.8
  (16 % with 8 points), because bias bends one end steeply.
- Spacing along the curve's arc (t, w): ≤ 1 % at 0.2 – 0.8, 4 % at
  0.05 – 0.95. Kept.

**Direction.** In the dump, a polyline's first vertex sits closer to the
head (mean 8.6 cm from the hair's centroid, last vertex 9.6 cm), so W 0 is
the root. Calib-Hair-tip (root dark brown, tip blond) renders darker near
the scalp and lighter at the ends. Calib-Hair-tipbg (bias 0.2 keeps the root
colour further) renders darker still: crown 0.19 vs 0.22 (sRGB mean).

**Calib-Hair** (root and tip barely differ): crown 0.89 × Iray in linear
luminance (median over the crown, its own crop). r/g 1.07 vs Iray 1.44. The
hue gap from 2026-10-27 is unchanged. The colour shares were not refitted.

## 3. Not mapped, and why

- **Highlight** (weight, root / tip colour, roughness). Octane's Hair has one
  R lobe; its strength comes from the Fresnel (IOR) and the `specular` tint.
  Mapping Iray's separate highlight layer onto it needs strand references:
  Calib-Hair-hl (weight 0.8, blue highlight, roughness 0.3, separation 0.2)
  is dumped and waits for Iray.
- **separation**: Octane's `offset` (cuticle tilt) is the analogue, but its
  unit is unmeasured. It waits for the same reference.
- **Anisotropy / Rotations**: the strand is the tangent. Octane's fibre BSDF
  is anisotropic by construction.
- **strength**: doesn't reach Iray.

## Process

- `mi-oracle.py` had silently dropped the clamped mix's components. The old
  "tips don't reach Iray" conclusion came from that. Before reading a graph
  through the fixture, check the raw `.mi` for array arguments.
- Iray batch for #42: Calib-Hair-tip, -tipbg, -hl (local scenes, 5000
  samples).
