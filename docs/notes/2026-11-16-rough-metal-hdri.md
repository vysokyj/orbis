# 2026-11-16 — Rough metals under the HDRI: STD holds (#172)

Resume of #172 (`2026-11-15-rough-metal-std.md`). The open question:
`metal_brdf` (0e9b2b8) gives isotropic all-metal Uber nodes past α 0.25
Octane's STD instead of Beckmann, decided under a coaxial point light. Does
it hold under the environment, where Beckmann's dark-metal rim was 1.5 – 8 ×
STD's? If the HDRI and the point light disagreed, that would be a Look split.

Scene: Calib-Metal-Rough (`make-calib-scenes.py`; HDRI; R1 – R4 dark grey
0.1 at r 0.354 / 0.5 / 0.7 / 1, R5 – R7 light grey 0.8 at r 0.5 / 0.7 / 1,
R8 gold (0.9, 0.6, 0.2) at r 0.7). Iray Server 2000 samples, Post Denoiser
on@8 (70453f2). Octane enhanced-final, 1024 samples, denoised pass, from
2026-11-15 (`c172/oct2`): the export (STD) and its Beckmann twin (`brdf=1`).
The commits since (521b921 `rough_lobe`, 8bc61dd `legacy_metal_rim`) don't
reach these nodes (no dielectric lobe, no textured roughness), so no
re-render.

## Sphere means (`calib-compare-all.py`, Octane / Iray, floor 1.04)

| | R1 | R2 | R3 | R4 | R5 | R6 | R7 | R8 |
|---|---|---|---|---|---|---|---|---|
| STD (export) | 1.05 | 1.08 | 1.13 | 1.17 | 1.09 | 1.14 | 1.18 | 1.16 |
| Beckmann | 1.28 | 1.82 | 2.90 | 4.89 | 1.13 | 1.21 | 1.28 | 1.25 |

## Rings (ρ / R, a scratch script on `sphere_discs`)

| sphere | STD 0 – 0.5 | 0.5 – 0.7 | 0.7 – 0.85 | 0.85 – 0.95 | Beckmann 0 – 0.5 | 0.5 – 0.7 | 0.7 – 0.85 | 0.85 – 0.95 |
|---|---|---|---|---|---|---|---|---|
| R1 dark 0.354 | 1.05 | 1.05 | 1.03 | 0.93 | 1.10 | 1.48 | 1.74 | 1.26 |
| R2 dark 0.5 | 1.10 | 1.07 | 1.01 | 0.87 | 1.31 | 2.26 | 3.43 | 2.30 |
| R3 dark 0.7 | 1.17 | 1.10 | 1.00 | 0.82 | 1.83 | 4.07 | 6.08 | 4.09 |
| R4 dark 1 | 1.22 | 1.14 | 1.01 | 0.79 | 2.83 | 6.97 | 8.83 | 5.57 |
| R5 light 0.5 | 1.12 | 1.06 | 1.04 | 1.03 | 1.16 | 1.10 | 1.09 | 1.10 |
| R6 light 0.7 | 1.18 | 1.10 | 1.06 | 1.03 | 1.26 | 1.16 | 1.14 | 1.13 |
| R7 light 1 | 1.22 | 1.14 | 1.09 | 1.03 | 1.32 | 1.24 | 1.20 | 1.15 |
| R8 gold 0.7 | 1.21 | 1.12 | 1.07 | 1.02 | 1.29 | 1.20 | 1.20 | 1.22 |

- Beckmann's off-half-vector Fresnel (the point-light finding) is there
  under the environment too: a grey rim Iray's dark metals don't have, 1.7 –
  8.8 × at 0.7 – 0.85 R. Visually the Beckmann row is the odd one out; STD
  and Iray look alike.
- The two lights agree, so no Look split: `metal_brdf` stays in both Looks.
- STD's residual is one component, not the shape: its core over the floor
  (1.01 / 1.06 / 1.13 / 1.17 dark, 1.08 / 1.13 / 1.17 light, gold 1.16)
  follows Calib-Furnace-Metal's white-furnace excess (STD 1.06 / 1.08 / 1.13
  / 1.20 at r 0.4 / 0.5 / 0.7 / 1): Iray's `simple_glossy_bsdf` (v-cavities,
  no multiscatter compensation) loses more of a rough lobe's energy than
  Octane's STD. It is directional albedo, so derivable from `libbsdf` rather
  than fitted, and a Look question (Enhanced may keep Octane's lobe). Opened
  as #189; no constant here.
- The dark metal's outermost ring (0.85 – 0.95 R) is 0.93 – 0.79 × Iray:
  Iray's metal curve going to white at grazing shows a little more there.
  Small, at the silhouette; left.

## Real scenes

No current Iray reference of a real scene with rough all-metals exists. The
Truck scene (private) exports about ten all-metal nodes with roughness maps
(brushed metal, `sprRoughness01`) on STD; noted in #189 for its check.
The other cached real-scene dumps are an older dump format or their scene
moved, so they weren't exported.

## Process

- The Octane side rendered before parking made the resume a 2-minute
  compare; the Beckmann twin rendered with it gave the decision in one
  table.
- A residual that scales with roughness and is the same for dark, light
  and gold metals is the lobe's energy, not Fresnel: hold it against the
  white furnace before reading it as a shape error.
