# 2026-11-05 Backscattering as a retro-reflector (#73)

Goal: Iray Uber's Backscattering at weight 1 renders a flat disc in Iray
(the light from behind the camera) and a dark metal mirror in Octane (the
#29 stand-in). Find what the lobe is and build it in Octane instead of
dimming a mirror.
Scenes: Calib-Backscatter (Iray 2026-10-30), Calib-Aniso2 K6 (Iray
2026-11-04), Calib-Map-Misc X1 / X2; regression Calib-Glossy, Calib-Aniso.
Variant enhanced-final, Octane 256 samples (64 for the probe renders),
1920 × 1080, ~30 s a render.

## Metric

Mean linear luminance (PNG decoded at 2.2) in a 40 px disc around each
sphere (x = 424 + 153 k, y = 560), Octane / Iray; medians alongside to
rule out fireflies (they agree within 0.05 except where noted). Shapes by
eye on 3× crops. A scratch script, as in the anisotropy notes.

## What the lobe is

DAZ's MDL (`metallic_layered_base`): `weighted_layer(backscattering_weight,
backscattering_glossy_reflection_bsdf(tint: glossy_color while Share Glossy
Inputs is on, roughness_u/v from backscattering_roughness), base: the
diffuse + glossy)`, under metal and refraction. The top-level default
weight is 0.5 (Calib-Backscatter's B3 has none in its .mi).
MDL spec: the retro-reflective component of the halfway-vector disk
model, made symmetric (min of the disks around ωo and ωi, divided by the
larger cosine), exponent 2 / r², `tint` scales it, no Fresnel;
`multiscatter_tint` defaults to 0 and DAZ doesn't pass it — no energy
compensation. Under a distant environment it shows L(ωo) blurred by the
lobe: the same env direction (behind the camera) all over a sphere, the
flat disc.

## Octane: is there a retro lobe?

The manual lists a Retro-Reflective option on Glossy, Metallic, Specular
materials and the Metallic / Specular layers. Pin dump (probe, Octane X
here, compat 15050000) of Glossy (16), Metal (120), Specular (18),
Metallic Layer (141), Specular Layer (139), Sheen Layer (142), Universal
(130): no such pin, no BRDF enum entry for it. OSL materials (closures)
aren't supported in Octane. So no.

## A mirror whose normal is the view direction

A mirror about n = ωo reflects ωo onto itself: a retro-reflector, and its
GGX lobe at roughness α spreads around ωo — the halfway vector centred on
ωo, which is the disk model's retro component. Octane's Universal takes a
tangent-space normal map in `normal`; the job is ωo in Octane's normal
map frame.

| try (Calib-Backscatter B4, roughness 0 unless noted) | look |
|---|---|
| Ray Direction (326) tangent, view dir off, normalized (0..1) | a distorted mirror (sky stretched in the top half) |
| same, view dir on / not normalized / world | mirror, dark centre, or noise patterns |
| Channel Invert (174) of R, G, R+G | distorted mirrors |
| Channel Merge (172) of Channel Picks (171), x ↔ y, each sign | distorted mirrors |
| constant normal (0.5, 0.5, 1) | plain mirror (the pin reads procedurals) |
| **OSL texture (117): −I in (T = dP/du ⟂ N, B = N × T, N)** | **flat tan disc, as Iray** |
| OSL with B = dP/dv made perpendicular | identical to N × T |

Ray Direction as albedo showed its tangent space varies smoothly but in a
frame other than the normal map's (its colours don't match the OSL
frame's); no flip or swap fixed it. Probe pins: Ray Direction
`viewDirection` (bool), `coordinateSystem` (0 world, 1 camera, 2 object, 3
tangent), `normalize`; Channel Invert `invert`, `invert1`, `invert2`;
Channel Pick `colorChannel`; Channel Merge `texture1..3`; Add 106.

## Brightness and roughness

Colour = the tint as it is (no factor), roughness = the gloss curve of r.

| Calib-Backscatter (k0 – k7: B1 none, B2 w0.25, B3 w0.5, B4 w1, B5 w1 r 0.49, B6 / B7 / B8 w1) | B1 | B2 | B3 | B4 | B5 | B6 | B7 | B8 |
|---|---|---|---|---|---|---|---|---|
| before (#29 dimmed mirror) | 1.08 | 1.18 | 0.84 | 0.59 | 0.50 | 0.58 | 0.62 | 0.54 |
| Ray Direction normal, energy-pres. GGX | 1.08 | 2.57 | 2.29 | 1.95 | 3.42 | 1.94 | 2.08 | 1.91 |
| OSL normal, energy-pres. GGX (brdf 6), curve | 1.08 | 1.23 | 1.34 | 1.41 | 2.17 | 1.15 | 1.01 | 1.01 |
| OSL, plain GGX (2), roughness r | 1.08 | 1.24 | 1.36 | 1.44 | 1.58 | 1.15 | 1.00 | 1.00 |
| OSL, energy-pres. GGX, roughness r | 1.08 | 1.24 | 1.36 | 1.44 | 1.73 | 1.15 | 1.00 | 1.00 |
| OSL, Beckmann (1), roughness r | 1.08 | 1.26 | 1.39 | 1.48 | 1.46 | 1.16 | 0.99 | 0.99 |
| OSL, Octane BRDF (0), roughness r | 1.08 | 1.21 | 1.20 | 1.20 | 1.62 | 1.12 | 1.08 | 1.08 |
| **OSL, plain GGX, curve (kept)** | 1.08 | 1.23 | 1.34 | 1.41 | 1.41 | 1.15 | 1.01 | 1.01 |

Kept plain GGX on the curve: single scattering as DAZ's lobe (no
`multiscatter_tint`), the curve the other Phong-like Uber lobes use. Only
the rough sphere tells the BSDFs apart; Octane's energy-preserving GGX
there keeps the light Iray's lobe loses.

Open: B4 and B6 have equal Iray arguments (0.134 / 0.137 G) but Octane
gives 0.190 / 0.158: B2 – B4 (left of the frame centre) are brighter and
less flat in Octane (left / right edge of B4 0.18 / 0.21, Iray 0.13 /
0.14), B6 – B8 match. Not the lobe's tail (Beckmann and GGX alike) nor
fireflies (medians agree). The env behind the camera in Octane, or the
GGX lobe cut by the sphere near the rim; not chased further.

## Other scenes (as kept)

| | before | after |
|---|---|---|
| Calib-Aniso2 K6 (w1, r 0.09, glossy aniso 0.5) | 0.56, dark mirror | 1.15, flat tan disc |
| Calib-Map-Misc X1 (w0.5 shared, r 0.09) | 1.23 (median 0.93), mirror streaks | 0.94 (0.93), as Iray |
| Calib-Map-Misc X2 (w0.5 unshared blue, r 0) | 0.87, red | 1.11, purple as Iray |
| Calib-Aniso2 K1 – K5, K7, K8 | 0.95 – 1.10 | unchanged |
| Calib-Glossy (9 spheres) | 1.01 – 1.15 | unchanged (± 0.01) |
| Calib-Aniso (9 spheres) | 1.04 – 1.16 | unchanged |

At r 0 the mirror about ωo is an exact retro-reflector; X2 shows the
environment behind the camera in its blue tint, as Iray does (the #29 note
read Iray's smooth lobe as "returns almost nothing": it returns the dark
part of the HDRI behind the camera).

Artefact: 3× crops of K6 show faint meridian stripes — OSL's dP/du is per
face, Octane's normal-map frame looks smoother, so the tangential part of
ωo is off by a small rotation per face. Invisible at 1×.

## Process

- A pin dump first (`octane-probe-pins.lua` with another `which` list) —
  it settled "no retro option" in one probe.
- Debugging a procedural normal: feed the same texture to the albedo
  (diffuse only) to see its field, then to `normal`; a constant normal
  checks that the pin reads procedurals at all.
- The env vars used for the variants were scratch code, gone before the
  commit; exports went to the scratchpad, not the shared cache.
