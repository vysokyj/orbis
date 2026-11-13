# 2026-11-13 — Architectural thin wall above refr_gloss 0.2 (#149)

`ARCH_GLASS_THIN_LOBE` (#147) has Iray points only at refr_gloss 0 / 0.2
(Calib-Furnace-ArchGlass2 H5 / H4: 0.386 / 0.505, flat per ring); at 0.4 /
0.6 / 1 the closed spheres' values (0.938 / 0.964 / 1) stand in. This
session prepares the measurement; the Iray side is parked.

## The scene

Calib-Furnace-ArchGlass3 (`make-calib-scenes.py`): ArchGlass2's frame —
Calib-NVIDIA3's spheres, NVIDIA architectural, reflectivity 0, uniform
dome, floor hidden, Firefly Filter off:

| sphere | refr_gloss | what | MDL r | export r_t (3.7 ×) |
|---|---|---|---|---|
| J1 | 1 | thin | 0 (mirror) | 0 |
| J2 | 0.8 | thin | 0.0073 | 0.027 |
| J3 | 0.6 | thin | 0.0192 | 0.071 |
| J4 | 0.4 | thin | 0.0508 | 0.188 |
| J5 | 0.2 | thin (H4's tie) | 0.134 | 0.496 |
| J6 | 0.4 | closed (G3's tie, thin / closed in one frame) | 0.0508 | 0.188 |
| J7 | 0.2 | thin under reflectivity 0.6 (Schlick 0.2 / 1 / 5, smooth) | 0.134 | 0.496 |
| J8 | 0.4 | thin at IOR 2 | 0.0508 | 0.188 |

The `.mi` (headless export) hands `thin_walled true` to J1 – J5, J7, J8,
none to J6 (default false), `refr_ior` 2 to J8 — the channels arrive as
set. Not kept as a fixture: the fixture test then compares the ground
height of a scene whose ground is off (DAZ writes 0, #150).

## What the models say

`libbsdf` thin spheres (`furnace-rings.py model --thin`), white, reflectivity 0:

| r | all | rings |
|---|---|---|
| 0.007 / 0.019 / 0.027 / 0.051 / 0.071 | 1.000 – 0.996 | flat 1.000 |
| 0.134 | 0.987 | 1.000 → 0.987 at 0.8 – 0.9 |
| 0.188 | 0.975 | 1.000 → 0.962 |
| 0.188 at IOR 2 | 0.975 | the same: no layer, no IOR |
| 0.496 | 0.829 | flat 0.82 |

So below refr_gloss 0.2 `libbsdf` loses nothing on a thin wall; whatever
Iray keeps under 1 at J1 – J4 is NVIDIA's own per-event loss (#117's
component B), measured directly by those spheres. With a per-wall τ that
reproduces H4 (0.78 per event, the flat 0.61), the model predicts J7 at
0.503 (all) / 0.488 (disk), flat 0.50 → 0.49 — the reflection layer
barely helps a thin sphere in the furnace, as the light it keeps inside
meets the lossy lobe again.

## Octane side (enhanced-final, 1024 samples, denoised pass, dome 0.1737)

Rendered at f3391a6's code (the stand-ins in place):

| sphere | disk | rings centre → 0.8 – 0.9 |
|---|---|---|
| J1 thin 1 | 1.000 | flat 1.000 |
| J2 thin 0.8 | 0.985 | flat 0.985 |
| J3 thin 0.6 | 0.971 | flat 0.971 |
| J4 thin 0.4 | 0.942 | flat 0.942 |
| J5 thin 0.2 | 0.514 | flat 0.514 (H4 0.514: the frames agree) |
| J6 closed 0.4 | 0.941 | 0.943 … 0.940 0.918 0.875 |
| J7 thin 0.2 refl 0.6 | 0.525 | 0.527 … 0.524 0.537 |
| J8 thin 0.4 IOR 2 | 0.942 | flat 0.942 (= J4) |

The thin spheres keep k² of the table exactly (Thin Wall takes no Fresnel
from the base IOR, #147), so each J1 – J4 Iray value replaces its stand-in
directly; J2 sits on the interpolated 0.982.

## Open

Iray batch in #149 (5000 samples, NVIDIA architectural — a queue-side DAZ
restart first, alone in its session). #117's Calib-Furnace-MdlTir covers
the thin lobe only at MDL r 0.2 / 0.4 (rougher than refr_gloss 0.2's
0.134) in our own modules: it can say whether the thin loss is the legacy
lobe's, not what NVIDIA's module keeps at r 0.007 – 0.05.

## Process lessons

- A furnace scene's `.mi` fails the ground-height fixture check (#150):
  check a new fixture with `cargo test` before deciding to keep it.
