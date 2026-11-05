# 2026-11-05 — channel audit: the last unexplained channels (#15)

Goal: every channel `scripts/dev/channel-audit.py` reports unread (486
scenes in My Library) explained — DAZ doesn't pass it, an open issue
covers it, or it is metadata. Four were left after the coordinator's
pass: `SS Horizon Blur`, Uber `Top Coat Specular`, `Multiply Specular
Through Opacity`, `PBR Masked Threshold`. A fifth turned up while the
audit was being fixed: Architectural's `refr_falloff_on`.

## Scenes (.mi only, DAZ built-ins and primitives)

| scene | from | what |
|---|---|---|
| Calib-L-Mi-SunSky-Look | Calib-L-Mi-SunSky-sydney | SS Horizon Blur 2, Blue-Red Tint 0.5, Sun Glow 2, Night Color (0.1, 0.1, 0.2) |
| Calib-Map-Audit | Calib-Materials (Uber) | A1 – A4 Top Coat Specular in modes 1 / 2 / 3 / 0; A5 – A7 PBR Masked Threshold (as the tear materials save it: a user property) under Cutout Opacity 0.9989 / 0.5; A8 Multiply Specular Through Opacity off |
| Calib-Mi-DzDefault3 | Calib-Mi-DzDefault | MSTO off on transparent spheres, S1 the twin with it on |
| Calib-Mi-AoA5 | Calib-Mi-AoA-base | the same on AoA Subsurface (a brick) |
| Calib-Mi-Arch | Calib-NVIDIA3 | Architectural Enable Absorption on an opaque sphere, on glass, with a distance, with a colour |

## What DAZ passes

- **SS Horizon Blur: passed** (`horizon_blur`), and so are `redblueshift`,
  `sun_glow_intensity`, `night_color`. DAZ leaves an argument out when it
  equals the MDL default (0.1 / 0 / 1 / black) — why Calib-L-Mi-SunSky-sydney's
  fixture never showed them. `iray::environment` carries all four now, the
  test reads a missing one as the MDL default.
- **Top Coat Specular: not passed** in any layering mode (no
  `top_coat_directional_*` in A1 – A4, as in Calib-Map-TopCoat's C6).
- **PBR Masked Threshold: not passed** — `cutout_opacity` as it is
  (0.9989 under 0.999; A6 and A7 identical).
- **Multiply Specular Through Opacity: not passed** — S1 = S2 on DAZ
  Default, on AoA and on Uber.
- **Architectural refr_falloff_on: passed**, with `refr_falloff_dist`
  (× meters per scene unit) and `refr_falloff_color` (linear: 0.8 →
  0.612). The MDL turns them into the glass's absorption coefficient.

To make "not passed" a test and not a reading, the Uber test now also
fails on any argument in the .mi that `iray::uber` doesn't produce
(minus five read elsewhere: rounded corners, cutout, displacement,
Share Glossy Inputs' unified flag). It passed over every fixture at once.

## SS Horizon Blur in Octane (probe 120)

Iray (NVIDIA's `hosek_wilkie_sky.mdl`, the same parameter as
`base::sun_and_sky`): ground weight smoothstep(−dir.z / (b / 10)) below
the horizon, full at asin(b / 10) — 0.57° at DAZ's 0.1, 11.5° at 2.

Octane Daylight: `groundStartAngle` (default 90, the export 0) and
`groundBlendAngle` (default 5°, minimum 1).

Attempts:
1. Probe on the Enhanced export (Hosek-Wilkie): blend 0 / 2 / 11.5 / 30
   — identical frames, the horizon a 1 px edge. The image was clipped
   (sky 255; the probe's import loses the tone LUT) — exposure 0.05 after.
2. Forced model 1 (Octane): `groundStartAngle` 10 moves the edge down;
   blend 1 – 30 changed nothing — but the pin had no input node and
   `setPinValueIx` left it at 5.0 (read back), connecting a new float
   node was refused. Dead end for measuring by Lua.
3. Import path: `importFromFile` + `copyFrom` reported model 3 for an
   ORBX that says 1; `octane.project.load` (File → Open) gives 1 (probe
   121). Probe artefact, not an export bug — the probe uses `project.load` now.
4. The export writes `groundBlendAngle` itself: Sydney (1°) a sharp
   edge, Look (11.5°) a ramp of ~300 px in Faithful (Octane model);
   Hosek-Wilkie still a hard edge. Octane's ramp, ground weight against
   angle / B: 0.42 → 0.18, 0.56 → 0.33, 0.70 → 0.50, 0.84 → 0.70,
   0.98 → 0.90 — (angle / B)², not a smoothstep.
5. B = k × asin(b / 10), k by least squares against the smoothstep:
   0.76 (rms 0.048; k 1 0.16, k 0.707 — matching the midpoints — 0.059).
   Rendered at B 8.8°: within rms 0.04 of Iray's curve.

So: `groundBlendAngle` = max(1, 0.76 asin(b / 10)) — Faithful only.

### Against Iray (2026-11-06)

Iray Server, 5000 samples (85e965f), against Octane Faithful final, a
strip at x 60–140 (no floor until y 840). The sky is clipped in both
renders (1, 1, 0.97), so only the ground side of the ramp measures: the
red channel's fall from 1 to the ground's value.

| row | 460 | 480 | 500 | 520 | 540 | 560 | 580 | 600 |
|---|---|---|---|---|---|---|---|---|
| Iray ground weight | 0 | 0 | 0.10 | 0.35 | 0.58 | 0.81 | 0.94 | 1 |
| Octane ground weight | 0.08 | 0.37 | 0.65 | 0.90 | 1 | 1 | 1 | 1 |

10–90 %: Iray ~75 px, Octane ~60 px (0.8 ×); Octane's horizon ~40 px
higher (Horizon Height 0.1 isn't mapped). The rule stands; #15 stays
closed. Enhanced (Hosek-Wilkie) keeps its hard edge, as noted. Ground
colour differs (Iray (0.69, 1, 0.54) clipped, Octane (0.37, 0.92, 0.55)):
the Sun-Sky's overall level, not the blur.

## Architectural absorption in Octane

`arch_absorption`: σa = −ln(c) / d, c = Absorption Color with Use Color
on, else 0.3333, d in metres, on the glass's medium. The first sanity
render showed clear spheres — `daz-dump.sh` exports with the *installed*
binary; re-exported with `target/release`: the 10 cm sphere darkened,
2 – 3 cm black. No Iray reference: the MDL states the coefficient.

## The audit

- `format!` templates in `src/` that start with a capitalised word
  ("Lens Distortion K{i}", "Dome Orientation {a}") count as read.
- Scene materials from a library `.dsf` (NVIDIA examples) now get that
  file's defaults, and scene channels overlay the definition field by
  field: Architectural's `brdf_curve`, `refr_trans_*` were at their
  defaults all along (the scene saves only `current_value`).
- `scripts/dev/channel-audit-known.tsv`: owner / channel regexes with the
  reason (spec section or issue); the report shows the reason and ends
  with an "Unexplained" list — 0 now (216 before the list).

## Process

- The probe's import route can change node values: load the ORBX as the
  user does.
- A pin without an input node can't be set from Lua here — write the
  value into the export instead and render as exported.
- After an export change, render what `target/release` exports, not what
  `daz-dump.sh` wrote.
