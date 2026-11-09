# 2026-11-09 — NVIDIA MDL presets' bump as a baked normal map (#89)

Goal: #85 baked every Uber height map into a tangent-space normal map
(`src/detail_normal.rs`), because Octane's bump reads the height per texel
and keeps every frequency at full slope. The NVIDIA MDL presets still put
their bump map on Octane's bump pin at `bump_power`, and Cast Iron's
"sparkle at 256 samples" looked like the same per-texel steps. Decompose
first: how does each preset's MDL build its bump?

## What the MDL says

DAZ's `shaders/iray/nvidia/*.mdl`:

| preset | bump | map |
|---|---|---|
| `metal_examples` (`metal_generic`, both presets) | `file_bump_texture(bump_texture, bump_amount, uvw: tiling)` on the metal layer and the base; `tangent_space_normal_texture` when `is_normaltexture` | Cast Iron `grunge_d.jpg` 2048 px, 1.5; Metal Brushed `brushed_a.jpg` 2048 px, 0.4, tiles 3 |
| `concrete`, `asphalt`, `ceramic_tiles_small` | `material_geometry(normal: file_bump_texture(...))` — the whole surface | 512 px PNGs, 6 / 1.5 / 4 |
| `concrete_polished` | `fresnel_layer(..., base: weighted_layer(normal: file_bump_texture(...), layer: diffuse))` — the diffuse only; the gloss keeps the plain normal | 512 px PNG, 2.5 |
| `architectural` | a `float3 bump = state::normal()` parameter; DAZ's .mi (Calib-NVIDIA, -Mi-Arch) wires nothing in | — |
| granite, porcelain, resin | none | — |

`file_bump_texture` with its defaults (`mono_average`, wrap repeat) is the
Uber's, so `detail_normal` applies as is: factor = Bump Strength (the
channel DAZ passes as `bump_amount`), the shader's tiling on the normal
map (a tangent-space normal tilts the same at any tiling). DAZ's bricks
have no `is_normaltexture` channel. Architectural keeps Octane's bump
for a Bump Strength map: no fixture shows what DAZ makes of it.

## Gamma of the height

The .mi hands these textures with `gamma "auto"` (the Uber's synthetic
maps had `gamma 1`). MDL's `gamma_default` leaves it to the file; NVIDIA's
programmer's manual says an 8-bit PNG's canvas has gamma 2.2. But the
bump PNGs carry a `gAMA` chunk of 1.0 (palette PNGs), the JPEGs none, and
the export already reads `grunge_d.jpg` linear for Cast Iron's roughness.
Two hypotheses, no constant: bake with heights decoded at 1 or at 2.2,
compare with Iray.

## Against Iray

Octane 1024 samples, enhanced-final; Iray references 5000 samples
(`calib/references/iray`). Calib-NVIDIA2 from the scratch copy with the
fitted camera (2026-11-05 note: the scene has no `current_camera`, the
reference shows the Perspective View).

Measures: disk luminance (r 0.75 of the sphere radius), Octane / Iray;
band-passed detail = std of (box blur r 2 − box blur r 8) in the
sphere's centre box over its mean, Octane / Iray, and the correlation of
that detail with Iray's (r 0 instead of 2 adds the pixel noise).

Calib-NVIDIA (floor 1.04), old (master) → linear bake; gamma 2.2 bake:

| sphere | disk | detail | corr | detail 2.2 | corr 2.2 |
|---|---|---|---|---|---|
| Metal Brushed | 1.04 → 1.04 | 1.07 → 1.07 | 0.96 → 0.96 | 1.07 | 0.96 |
| Cast Iron | 1.13 → 1.04 | 0.91 → 1.05 | 0.82 → 0.91 | 1.13 | 0.82 |
| concrete | 1.01 → 1.04 | 1.01 → 1.00 | 0.99 → 0.99 | 1.01 | 0.99 |
| polished | 0.97 → 0.99 | 0.98 → 0.98 | 0.94 → 0.95 | 0.98 | 0.94 |

Without the band-pass (r 0 – 6) Cast Iron: detail 1.01 → 1.18 (2.2:
1.29), correlation 0.70 → 0.83 (2.2: 0.67) — pixel noise included. The
four spheres without a bump within 0.01 (1.09 1.05 1.05 1.07).

Calib-NVIDIA2 (floor 1.05), old → linear (2.2 alike):

| sphere | disk | detail | corr |
|---|---|---|---|
| asphalt | 0.96 → 1.00 | 0.97 → 1.03 | 0.44 → 0.47 |
| ceramic tiles small | 1.06 → 1.06 | 1.02 → 1.04 | 0.87 → 0.90 |

Asphalt's detail is mostly the dark diffuse map's (low correlation either
way). The rest unchanged.

Sparkle: Cast Iron's pixel noise, std of (256 − 1024 samples) over the
mean in the centre box, 0.21 → 0.12 (concrete 0.034 → 0.031; granite,
no bump, 0.042 / 0.048 — run-to-run).

Linear wins at Cast Iron (the only map where the two differ much: a
contrasty JPEG at amount 1.5) and ties elsewhere: kept, so the NVIDIA
bump goes through `detail_normal` exactly as the Uber's.

## Dead end: polished concrete's smooth gloss

The MDL bumps only polished concrete's diffuse. Universal's `normal` tilts
both lobes, but its coating keeps the plain normal unless its own pins
are set (#61), so the gloss went on `coating` (luminance map × 0.8,
coatingIor, coatingRoughness on the GGX curve, specular 0): detail
1.58 × Iray, correlation 0.84 (bumped specular: 0.98 ×, 0.95), disk 0.98.
The coating isn't the same lobe as the specular the dielectrics were
calibrated with (#21); not pursued — the gloss stays on `specular` with
the bump.

## Uber scenes

Calib-CoatBump and Calib-BumpRes export byte-identical to 78e5944's
binary (the Uber path doesn't change), so no render.

## Process

The scene's camera trap again: an export of Calib-NVIDIA2 from the
library rendered the saved camera, not the reference's view — use the
fitted copy (or save the scene with `current_camera`). Octane caches a
texture by its package path: a re-export under the same file name while
a render waits for the lock may render the old texture (the coating
variant's Cast Iron matched the 2.2 bake's numbers); give each variant
its own directory.

**Superseded (#91):** the decode is 2.2 for an 8-bit map at `map_gamma`
0 (synthetic cards, `2026-11-09-bump-gamma.md`); Cast Iron's detail measure
turned out not to depend on the decode in Iray.
