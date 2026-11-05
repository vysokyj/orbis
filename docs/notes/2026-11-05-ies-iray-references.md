# 2026-11-05 — IES profiles against Iray (#34)

Follow-up of [2026-11-04](2026-11-04-ies-profiles.md). The six
Calib-L-IES-* Iray references (5000 samples, Iray Server, a676f4f) against
Octane, variant enhanced-final, 400 samples. Exports in
`~/Library/Caches/orbis/ies34b/`, three render rounds (v1 as built,
v2 frame fixed, v3 power fixed).

## Measuring

- Bare-floor scenes (camera 5.2 m above, X −90°): image right = world +X,
  image up = world −Z. Point-Down's dark disc in Iray ends at 480 px from
  the middle = 1.75 m = 35° at 2.5 m height → 274 px/m (the 4 m floor
  spans 1087 px). Radial profiles: rings of ±0.05 m, PNG linearised with
  gamma 2.2, saturated or black pixels left out.
- Emit scenes: the usual camera at (0, 120, 450) cm, X −10.45°, **yfov
  42.07°** (the .duf's `yfov` 0.734 rad — 30.96° put the floor's front
  edge 150 px off; 42.07° puts it at row 921, as rendered). Each floor
  pixel → its world point → the direction from the plane's middle, θ / φ
  in the profile frame, cos θ_n to the plane's normal (world −Y); the
  profile read by a small Python port of `ies::Profile::intensity`.
  Divided by Iray's plain Calib-L-Emit-cd (Lambertian) to get the shape.

## Orientation

v1 (as built, the frame from NVIDIA's MDL comment: axis on the light's
+Z):

| scene | Iray | Octane v1 |
|---|---|---|
| Point-Down | dark disc r 1.75 m (35°) | dark disc ~0.9 m (19°), bright ring |
| Asym-Down | lopsided downlight spot, notch to −X | black |
| Rect | dark disc ~35°, soft | small square hole, bright ring |
| Emit-Asym | the floor's front lit | almost dark |
| Point | brighter towards image top (−Z) | brighter towards the bottom |

So Iray's axis is the **light's −Z**. Asym-Down (Iray): 7×7 px samples
from the light's foot — left (−X) 0.26 → 0.20, up (−Z) 0.74 → 0.59, right
and down saturated: φ 180 (0.1) towards −X, so φ 0 = +X; down (+Z, local
−Y) brighter than up → φ 90 (0.6) = local −Y. Frame (X, −Y, −Z) =
the light turned half round X. The emissive plane the same in object
space: Calib-L-IES-Emit's plane is turned X 180°, object −Z = world +Z
(the camera side), and Iray's Emit-Asym lights the floor's front.

Fix: point / sphere frame (X, −Y, −Z) (projection rotation 90 90 0), the
rect / disc prim (already turned half round X) its own frame, emissive
surfaces (X, −Y, −Z) of the node's axes. v2: every scene lights the same
places as Iray (Point-Down's disc 35°, Asym-Down's notch to −X).

## Power

v2 (power × 1 / mean_share for all), Octane / Iray:

| scene | floor | note |
|---|---|---|
| Point | 3.96 – 4.04 | 18 rings 0.1 – 2.6 m, flat |
| Point-Down | 3.90 – 4.04 | |
| Asym-Down | 3.99 – 4.04 | |
| Rect | 1.18 – 1.39 | falls outwards |
| Emit | 1.21 – 1.57 | (ratio) / cos θ_n = 1.742 – 1.748 |
| Emit-Asym | 1.05 | / cos θ_n 1.31; lit only at θ 53 – 60° |

- Points: exactly 4 with both profiles, the shape identical — Iray
  renders an IES point light at 1/4 of the flux-kept profile. (The
  2026-11-04 probe had verified Octane's side against Octane's own plain
  point light, never Iray.) Likely `ies_light` on DAZ's small sphere: the
  intensity_power flux spread over the whole surface, a direction seeing
  its projected quarter. `IES_POINT_SHARE` = 4. v3: 0.997 – 1.009.
- Areas: Iray / plain Lambertian × cos θ_n / D is flat over cos θ_n
  (Emit: 2.327 / 2.334 at 0.7 – 0.85 / 0.85 – 1.0) → Iray emits I ∝ D(ω)
  with no cosine; Octane's emitter is Lambertian × the distribution, so
  Octane / Iray ∝ cos θ_n (Rect: 1.37 / 0.82 … 1.18 / 0.69 = 1.67 – 1.71).
  No Octane knob removes the cosine.
- Which constant? Profile integrals (peak-normalised, 180 × 360 grid):

  | profile, normal | mean share | F / S (front) | L / S (∫ D cos / ∫ D) |
  |---|---|---|---|
  | DAZ batwing, normal = axis (Rect) | 0.434 | 0.459 | 0.211 |
  | DAZ batwing, normal ⟂ axis (Emit) | 0.434 | 0.500 | 0.250 |
  | Orbis-Asym, normal = axis | 0.060 | 1.000 | 0.837 |
  | Orbis-Asym, normal ⟂ axis (Emit-Asym) | 0.060 | 0.425 | 0.124 |

  With v2's constant k (Octane / Iray / cos θ_n) the flux through the
  front was k · L / F: Rect 0.78, Emit 0.87, Emit-Asym 0.38. For a
  downlight on a rect, v2 would give 4 · L / S ≈ 3.3 × Iray's intensity
  if Iray normalises to the front (its batwing numbers fit that within
  0.87 – 0.93).
- Rule taken (v3): keep the **flux through the front** — power × π I_max /
  ∫ D max(0, n · ω) dΩ (`Profile::lambert_share`), n the rect's axis or an
  emissive mesh's face normals by area. v3:

  | scene | floor Octane / Iray | / cos θ_n | flux |
  |---|---|---|---|
  | Rect | 1.44 – 1.66 | 2.02 | 0.93 |
  | Emit | 1.21 – 1.57 (unchanged: lambert share = mean share here) | 1.75 | 0.87 |
  | Emit-Asym | 2.11 | 2.65 | 0.77 |

  The floor gets more than Iray where Octane's cosine concentrates the
  light below the emitter; Iray's sideways light (to walls, absent in the
  Calib scenes) is missing. A downlight facing along the normal keeps
  nearly Iray's shape. Emit-Asym's number sits on the profile's 60° cut-off
  edge (Iray interpolates the file with Hermite splines, the Python port
  linearly) — the least reliable of the three.

## Process

- The first export loop failed: `target/release` doesn't exist here —
  cargo builds where `scripts/dev/target-dir.sh` says.
- The Emit geometry first used 30.96° (from the spec's camera line); the
  floor edge check caught it — read `yfov` from the .duf.
- Renders: 6 scenes × 3 rounds, ~30 s each in Octane X, no crash.
