# 2026-11-03 — Iray environment options: fog, finite dome, lighting resolution (#37)

Goal: map the Environment Options #37 lists — Ground Fog, Matte Fog, Dome
Mode (finite dome, dome with ground), Environment Lighting Resolution /
Blur, Environment Tint, Ground Reflectivity / Glossiness, Ground Visible
from Below — to Octane, or record why not. Scenes: `Calib-Env-*` (Iray
references from 2026-11-02, 5000 samples, on Calib-Tone-Default: eight
spheres on a plane under DAZ's default HDRI), Octane enhanced-final.

## What DAZ hands Iray (.mi)

The Iray references alone don't say how DAZ converts the channels, so 13
.mi fixtures first (headless DAZ, `daz-export-mi.dsa`): the seven
`Calib-Env-*` with a reference plus seven .mi-only scenes
`Calib-Env-Mi-*` (`make-option-scenes.py`: every fog / dome channel off
its default). Now in `src/iray.rs` (`ground_fog`, `matte_fog`,
`dome_shape`, `DomeSetup::ground_visible_from_below`), tested:

| Iray option | DAZ channels |
|---|---|
| `iray_ground_fog_density0/1` | Density Start / End, linear, × their scales (0.5 × 2 → 0.435) |
| `iray_ground_fog_height0/1` (m) | Decay Start, Decay Start + Decay Height (cm / 100) — absolute: the ground at 30 cm, the fog from 50 cm |
| `iray_ground_fog_albedo` | Albedo, linear |
| `iray_ground_fog_scattering_type` | "hg" / "approx_mie"; anisotropy only under hg (0 under Mie), particle size as it is |
| `matte_fog_visibility` (m) | Matte Fog Visibility as it is; the tints linear, brightness as it is |
| `environment_dome_mode` | Dome Mode 0–5: infinite, ground, sphere, sphere_with_ground, box, box_with_ground |
| `environment_dome_radius`, `_width/_height/_depth` | × Dome Scale Multiplier (70 × 30 → 2100 cm); only for a finite dome |
| `environment_dome_position` | Dome Origin (cm) as it is; only for a finite dome |
| `environment_dome_ground_texturescale` | Ground Texture Scale (cm) as it is |
| `environment_dome_ground_visible_from_below` | the channel, only with Draw Ground on (else true) |
| `environment_lighting_resolution` / `_blur` | as they are |

## The Iray references against the base scene

Ratios to Calib-Tone-Default (Iray), linear:

| scene | sky | floor | below the plane | spheres | side |
|---|---|---|---|---|---|
| Blur | 1.003 | 1.000 | 0.998 | 1.002 | 0.999 |
| Res128 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 |
| MatteFog (100 m) | 1.000 | 1.032 | 1.000 | 1.073 | 1.015 |
| DomeMode1 (w/ ground) | 1.000 | 1.000 | 0.733 | 1.012 | 0.723 |
| DomeMode2 (sphere 100 m) | 1.049 | 1.006 | 0.973 | 1.001 | 1.037 |
| Ground (refl 0.5, gloss 0.6) | 1.000 | 1.000 | 1.000 | 1.000 | 0.991 |
| GroundFog (defaults) | 1.000 | 0.795 | 1.198 | 1.041 | 0.882 |

Lighting Resolution and Blur only steer Iray Photoreal's importance
sampling (NVIDIA's manual says so; Res128 renders identically, Blur within
noise): nothing to map. The ground reflection is hidden under the scene's
plane here.

## Ground fog

### Attempt 1: Octane's Post FX fog — crashed

Octane's post-processing volume fog has the same shape as Iray's (a base
level, a half-density height, an extinction distance). Variants patched
into the export (`postproc` on, bloom/glare 0, fog on, extinction 1 m,
base 0, half-density 0.069 m) failed at once: "render engine failure",
the status bar "Apple M4 (10 GPU cores) (RT) (failed)" with the GPU memory
at 24.0 of 24.2 GB; Octane X then hung at 100 % CPU and had to be killed.
Dead end: not used, not retried.

### Attempt 2: slabs of Unit Volume

Octane X has a Unit Volume geometry node (type 295: medium, object layer,
transform — found by a Lua probe of the `NT_` constants). Iray's fog is a
uniform density below `height0` and an exponential decay above it; the
export writes it as boxes 2 km wide around the camera: one 1 km deep below
`height0` at `density0`, then slabs up to 1/1000 of the ground's density
(first sixteen of half a decay length each, finally four of two), at the decay's mean over the slab
(so each slab's optical depth is exact). Each a Volume medium (98):
scattering = albedo × extinction, absorption = (1 − albedo) × extinction.

Ratios to Iray's Calib-Env-GroundFog, 128 samples (the export without
fog sits at sky / floor / below 1.04 / 1.04 / 1.04, spheres 1.06):

| slabs (half width, thickness), `rayMarchStepPercent` | floor | below the plane | spheres | horizon | s / 128 spp |
|---|---|---|---|---|---|
| 1 km, ½ decay, 1 % | — | — | — | — | > 660, stopped |
| 1 km, ½ decay, 100 % | 1.27 | 0.49 | 1.14 | 1.11 | 231 |
| 1 km, ½ decay, 100 %, density × 3 | 1.22 | 0.44 | 1.12 | 1.11 | 211 |
| 1 km, ½ decay, 10 % | 1.12 | 0.89 | 1.08 | 1.05 | 479 |
| 50 m, ½ decay, 100 % | 1.28 | 0.52 | 1.15 | 1.12 | 219 |
| 50 m, ½ decay, 30 % | 1.23 | 0.75 | 1.13 | 1.12 | 317 |
| 50 m, ½ decay, 10 % | 1.12 | 0.96 | 1.09 | 1.08 | 450 |
| 50 m, 2 decays, 30 % | 1.11 | 0.93 | 1.09 | 1.08 | 192 |
| **50 m, 2 decays, 10 %** | **1.06** | **1.00** | **1.07** | **1.05** | 321 |

- Too little fog at a long step was not too little density (× 3 changed
  the floor by 4 %): Octane's ray march at a step of the whole box
  under-counts the in-scatter.
- The box's width changes neither the result nor the time; the slabs'
  count does: four slabs of two decay lengths were both closer and faster
  than sixteen of half one, at the same step (the optical depth of each
  slab stays exact either way).
- 50 m slabs put the top of the fog wall at the horizon a little lower
  than Iray's unbounded fog; 1 km for the export.
- Cost: Octane needs ~15 × the fog-free render time (21 s → 321 s for
  128 samples); Iray needed 2.9 × (105 → 302 s for 5000).

Final rule (1 km, two-decay slabs, step 0.1, the export as it is):
floor 1.06, below the plane 0.94, spheres 1.06, side 1.03, horizon 1.03,
sky 1.04 — the fog-free frame's 1.04 – 1.06. Mie (droplet size) stays
isotropic: Octane's phase is Henyey-Greenstein only.

## Matte fog — not mapped

A uniform Unit Volume around the scene and the cameras (1 m margin),
extinction ln 50 / visibility (Koschmieder's 2 % contrast distance),
albedo = brightness × tint. Calib-Env-MatteFog (100 m), 128 samples:

| | sky | floor | below the plane | spheres | side | horizon | s |
|---|---|---|---|---|---|---|---|
| no matte fog | 1.04 | 1.00 | 1.04 | 0.99 | 1.02 | 1.04 | ~15 |
| fog box | 1.13 | 1.03 | 1.90 | 1.24 | 1.32 | 1.25 | 230 |

Iray's matte fog leaves rays to the environment alone (NVIDIA's manual);
the box fogs the HDRI it holds — the dark ground below the plane came out
1.90 × — and in-scatters more than Iray's "relative to environment"
brightness (spheres + 24 % against Iray's + 7 %). No Octane fog skips the
environment; with DAZ's default visibility of 10 km the effect is nil at
scene scale. Left out, the parse (`iray::matte_fog`) kept.

## Dome with ground

Iray's "Infinite Sphere w/ Ground" projects the HDRI's lower half onto
the ground from a virtual camera Ground Texture Scale above it.
Octane's texture environment can't, so the export puts the projection on
the ground: the HDRI through a world-space Spherical projection at the
virtual camera, as an emission (surface brightness, power = the dome's).

| try | below | side | horizon | note |
|---|---|---|---|---|
| no dome ground | 1.41 | 1.44 | 0.82 | the HDRI's own lower half |
| infinite plane (110), emitting Diffuse | 0.00 | 0.59 | 0.00 | black — also with constant RGB emission, double-sided, or surface brightness off |
| SDF box (296) 2 km × 1 mm | 1.04 | 0.91 | 0.19 | lit, but mirrored left-right |
| … UV transform scale −1 | 1.13 | 0.95 | 0.22 | mirrored, shifted |
| … projection transform scale −1 1 1 | **1.04** | **1.04** | **1.03** | matches Iray's streaks |
| … projection transform scale 1 1 −1 | 1.13 | 0.95 | 0.22 | |

Power 1 × the dome's is right (the frame sits at 1.03 – 1.06 throughout).
The finite dome (sphere / box) itself isn't mapped: 100 m changed Iray's
sky by 5 %, the scene < 1 %.

## Not mapped, and why

- Lighting Resolution / Blur: importance sampling only (above).
- Ground Reflectivity / Glossiness: Octane's shadow catcher has opacity
  only, and no reference shows Iray's ground reflection here.
- Ground Visible from Below: the shadow catcher is transparent from below
  anyway.
- Visualize Finite Dome: a viewport aid.

## Process lessons

- A failed Octane render can hang Octane X (100 % CPU): the render script
  then waits forever. Watch the probe log's time, not just its last line.
- `rayMarchStepPercent` on a Unit Volume: test slab thickness and step
  together; the count of volumes is what costs.
- The infinite plane primitive doesn't emit — use an SDF box for an
  emitting ground.
