# 2026-11-04 — Camera lens options (#39)

Goal: DAZ's camera lens channels (Lens Shift, Stereo Offset, Thickness,
Radial Bias, Distortion Type / K1–K5 / Scale / Maximum FOV, Aperture
Blades / Rotation) onto Octane's camera. Variant: Enhanced × Final.

## Scenes

`scripts/dev/make-option-scenes.py` → `calib/scenes/Calib-Lens-*`, one
option group each: on Calib-Tone-Default (no DOF) -Shift (5 / −3 mm),
-Stereo (100 mm), -Spherical, -Cylindrical, -Poly3 (K1 0.3), -InvPoly5
(0.2, 0.1), -Brown (1, 0.05, −0.3, 0.1, 0.2), -Equidistant (K2 2.0944 =
120° across), -Mi-PTLens (inv_ptlens, scale 0.8, max FOV 3; .mi only); on
Calib-DOF-f4 (f/4, focus 1.5 m) -Blades5 (rotation 20°), -Bias02, -Bias08,
-Thick (50 mm). All opened headless, dumped and exported to `.mi`.

## What DAZ hands Iray (.mi)

Every channel reaches the camera: mm → cm (shift 0.5 / −0.3, stereo 10,
thickness 5), the blade rotation in radians (20° → 0.349), K1–K3 →
`mip_lens_distortion_params`, K4, K5 → `…params2[0..1]`, the type enum →
Iray's string in the enum's order, bias / scale / max FOV unchanged. Now
fixtures; `iray::Lens` reproduces them (the camera test checks 15 more
values per scene).

## Octane

- **Probe 1** (Calib-Lens-Shift, 32 samples): thin lens `lensShift` (0.1, 0)
  moves the frame 192 px = 0.1 × 1920; (0, 0.1) 108 px = 0.1 × 1080 —
  fractions of width and height, positive = view right / up. A first
  attempt to wire an OSL camera failed in the probe itself
  (`getInputNode` takes a pin id, not an index); `octane.project.saveAs`
  returned false (the sandbox, known).
- **Probe 2**: the OSL camera's attributes by id → Lua names
  (`A_SHADER_CODE` …). A pinhole OSL camera (film 8.889 cm, 65 mm, shift
  0.1 in the shader) wired in rendered the same frame as the thin lens with
  `lensShift` 0.1 (mean |Δ| 0.22 / 255 against 17.9 for the unshifted one):
  `u`, `v`, `point("camera", …)` behave as assumed.
- The exports with an OSL camera written into `project.ocs` by hand (node
  126, attribute `shaderCode`, pins `pos` / `target` / `up`) load and
  render: Poly3 (the frame shrinks, edges bow), InvPoly5, Brown,
  Equidistant (a 120° fisheye), Spherical and Cylindrical (full panoramas,
  HDRI around). Calib-Lens-Blades5 (400 samples) shows pentagonal bokeh on
  the gloss highlights; Stereo moves the frame left (camera right).
  Renders: 64 samples, 4–8 s each.

## Not settled (Iray references needed)

The Iray manual leaves open: the units / sign of the lens shift, how the
distortion radius is normalised (the equidistant table fixes r = s·d / W,
assumed for every model), the panoramas' extents, which way the blade
polygon points, parallel vs converging stereo, what thickness does, and
the radial-bias distribution (Octane's `aperture_edge` can't be set
without it — not mapped). The scenes for each wait on #39 for the next
Iray batch.

## Lessons

- The .mi fixtures first: one DAZ run settled every channel conversion.
- Octane's OSL camera is the way to reproduce any of Iray's lens models
  exactly; `camera:dofrandom` keeps depth of field with it. The `.ocs`
  attribute name `shaderCode` was a guess from `A_SHADER_CODE` (as
  `filename` ↔ `A_FILENAME`) that loaded first time.
