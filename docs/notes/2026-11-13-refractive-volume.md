# 2026-11-13 — Scattering in a refracting solid, decomposed (#104)

`REFRACTIVE_SCATTERING_SCALE` 0.3 (σs × 0.3 in Octane's scattering
medium behind a refracting surface) was fitted on one sphere,
Calib-Map-Misc's X7 (#28: Iray's σs rendered 1.47 × Iray, × 0.3 ~1.0),
with an isotropic medium: SSS Direction was not read. `git log -S`: the
constant is still the one from a2702c2 (#28), moved into the `iray.rs`
reading by 2bc17f0; nothing else touched it. Decompose before keeping it.

Octane 1000 samples, Enhanced + Final, raw beauty. Work dir
`~/Library/Caches/orbis/calib/x104` (orbx/, octane/, dump/); the
variants came from a temporary `ORBIS_X104` flag (`s=<σs scale>`, `iso`
phase off, `nofs` fake shadows off; removed before the commit) and
`orbx-variants.py ior=1`.

## Step 1: what Iray computes (the MDL)

`add_uber_volume` (`irayubermaterial.mdl`): `material_volume(scattering:
df::anisotropic_vdf(directional_bias: SSS Direction), absorption_coefficient:
−ln(Transmitted Color) / d_t, scattering_coefficient: SSS Amount / d_s (mono)
or −ln(SSS Color) / d_s (logarithmic))`. No scale on σs; the phase is
Henyey-Greenstein at SSS Direction. X7's `.mi` (Calib-Map-Misc.json):
σa from Transmitted Color 0.218 / 0.456 / 0.793 (linear) at 0.02 m, SSS
Amount 0.3 at 0.005 m, `sub_surface_scattering_direction` 0.4, glossy
roughness 0.09, `full_refraction_reflection_enable`. Octane's scattering
medium has a `phase` input; the Volume medium already takes Iray's g
through a Schlick node (60, `scattering_direction`; the spec's fog).
So the phase is a direct read: SSS Direction → node 60 on the
refractive medium too.

## Step 2: a model to compare against (white furnace)

`scripts/dev/refr-volume-mc.py`: the MDL's sphere (smooth dielectric at
Refraction Index, Fresnel both ways, total internal reflection, the
homogeneous HG volume inside; 15 cm radius) in a uniform white dome. By
reciprocity a pixel shows the share of a beam entering there along the
view that leaves again; averaged over the middle square (±0.6 of the
radius). Checked against the exact semi-infinite isotropic albedo
(Chandrasekhar's H function, index-matched, normal incidence, R 150 cm):
α 0.5 0.1158 vs 0.1152, α 0.9 0.4198 vs 0.4149.

What the MC says about X7's own material in the dome (out, R / G / B):

| | out |
|---|---|
| MDL: σs, g 0.4 | 0.060 / 0.081 / 0.180 |
| σs, isotropic | 0.082 / 0.115 / 0.232 (1.37 / 1.42 / 1.29) |
| σs × 0.3, isotropic (the export) | 0.056 / 0.069 / 0.140 (0.93 / 0.85 / 0.78) |

The phase alone is ~1.35 × of #28's 1.47: the 0.3 mostly stood in for
the missing g 0.4 (forward scattering sends less light back out).

New scenes (`make-calib-scenes.py`, in `calib/scenes`): closed, refracting
(IOR 1.5, shared smooth gloss), 15 cm spheres in Calib-Furnace-UberGlass's
dome — Calib-Furnace-RefrVol-Grey (X7's red absorption, 0.5 at 2 cm, in
all channels), -Chroma (X7's own Transmitted Color 0.5 / 0.7 / 0.9: its
red the same in RGB), each: absorption only, SSS Amount 0.1 / 0.3 / 1 at
0.5 cm, 0.3 at SSS Direction −0.6 / 0.6 / 0.4, absorption at 10 cm;
-SSSColor (chromatic SSS Color grey / blue / orange, ±0.6, the
scattering's density); Calib-RefrVol: X7's material under the HDRI, one
part changed per sphere (direction 0 / −0.6, grey / no absorption,
absorption only, SSS Amount 1 / 0.1).

## Step 3: Octane against the Monte Carlo (Calib-Furnace-RefrVol-Grey)

Out (sphere / dome, linear, the middle square; `refr-volume-mc.py
--measure`), grey so one channel:

| sphere | MC | export (σs × 0.3, g) | σs, g | σs, g, no fake shadows | MC at IOR 1 | the same, `ior=1` |
|---|---|---|---|---|---|---|
| G1 absorption only | 0.043 | 0.043 | 0.043 | 0.043 | 0.000 | 0.000 |
| G2 amount 0.1 | 0.057 | 0.051 | 0.069 | 0.058 | 0.044 | 0.043 |
| G3 amount 0.3 | 0.082 | 0.066 | 0.112 | 0.082 | 0.110 | 0.108 |
| G4 amount 1 | 0.146 | 0.112 | 0.221 | 0.146 | 0.247 | 0.244 |
| G5 g −0.6 | 0.150 | 0.095 | 0.173 | 0.152 | 0.180 | 0.182 |
| G6 g 0.6 | 0.053 | 0.050 | 0.063 | 0.052 | 0.039 | 0.034 |
| G7 g 0.4 | 0.061 | 0.055 | 0.080 | 0.060 | 0.061 | 0.059 |
| G8 absorption 10 cm | 0.199 | 0.165 | 0.306 | 0.196 | 0.337 | 0.330 |

- Octane reads the phase (G5 / G6 / G7 differ from G3 in the right
  directions).
- At Iray's σs Octane was 1.2 – 1.5 × the MC, rising with the scattering
  (G2 → G4 → G8), only 1.15 – 1.2 on G5 / G6. Not the medium: the
  material's `fake_shadows` (on since the cornea, "light through for
  shadows") lets every scattering vertex's shadow ray see the dome
  straight through the surface — no refraction, no Fresnel, no total
  internal reflection. Off: 0.98 – 1.02 on every sphere. Index-matched
  (`ior=1` against the MC at IOR 1) the same, so the dielectric
  boundary is right too.
- Absorption alone (G1) is the same either way (no scattering vertex
  inside): glass keeps its fake shadows.

The MC itself, checked against the exact half-space albedo (step 2), and
the variants without fake shadows agreeing to 1 – 2 % on eight different
volumes: no constant left to fit.

## Step 4: chromatic absorption and scattering

Calib-Furnace-RefrVol-Chroma (X7's absorption), no fake shadows, Octane /
MC per channel: C1 1.02 / 0.98 / 0.79, C2 0.95 / 1.00 / 0.87, C3 0.90 /
0.99 / 0.91, C4 0.90 / 1.01 / 0.94, C5 0.93 / 1.01 / 0.96, C6 0.91 / 0.95 /
0.84, C7 0.92 / 0.98 / 0.87, C8 0.89 / 0.99 / 0.92. Red (the same σa as
Grey's, where Octane was 1.00) is ~0.9: the spectral leak #87 found on
props (Octane upsamples the σ colours to spectra; here milder, 0.10 /
0.04 / 0.01 per mm against Victoria's 0.04 / 1.3 / 1.8). Not compensated:
it is Octane's, and #87 decides the rule for it.

Calib-Furnace-RefrVol-SSSColor (chromatic SSS Color, grey absorption),
no fake shadows / MC: S1 1.00, S2 1.01 / 0.99 / 1.03, S3 1.07 / 0.99 /
1.00, S4 1.00, S5 0.95, S6 1.00, S7 1.00, S8 1.00. A saturated σs barely
leaks.

## Step 5: X7 against Iray (Calib-Map-Misc)

Octane / Iray on X7 (RGB; luminance of the other spheres unchanged
within 0.02):

| | X7 R / G / B |
|---|---|
| σs × 0.3, g 0.4 (the phase added to the old rule) | 0.83 / 0.83 / 0.77 |
| σs, g 0.4 | 1.15 / 1.47 / 1.32 |
| σs, isotropic | 1.57 / 2.02 / 1.66 |
| σs, g 0.4, no fake shadows (kept) | **0.88 / 0.98 / 0.87** |

Green — X7's least saturated channel relative to its σ — at 0.98; red /
blue ~0.88 as Chroma's leak in the furnace. #28's 1.47 at the full σs was
the phase (MC: isotropic 1.37 × g 0.4) and the fake shadows.

## Result

`REFRACTIVE_SCATTERING_SCALE` is gone: the medium takes Iray's σa / σs
unscaled, the phase at SSS Direction, and the material drops
`fake_shadows` when its medium scatters. No Iray batch: the MDL settled
the phase, the furnace MC settled Octane's medium, and X7's existing Iray
reference confirms the whole at 0.94 (luminance). The four new scenes are
in `calib/scenes` for a later check of Iray against the same MC.

## Process

- `git log -S` first: the constant was untouched since #28; re-measure
  on the current code before keeping any old fit (not rendered here:
  × 0.3 isotropic on today's code; the MC puts it at 0.93 / 0.85 / 0.78
  of the MDL in the furnace).
- A white furnace plus a Monte Carlo of the MDL is a known input for a
  medium inside a refracting shell; a constant HDR dome and the sphere /
  dome ratio need no Iray at all.
- Read every pin the material writes, not only the one under test: the
  excess was `fake_shadows`, two lines below the medium.
- The cached dump is found by the scene's path, not its content: a
  `calib/scenes` copy of a library scene needs its own dump (Map-Misc
  was exported from the library copy, byte-identical).
