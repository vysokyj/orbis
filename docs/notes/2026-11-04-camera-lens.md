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

## Against Iray (second batch, 5000 samples)

Octane renders at 400 samples (Enhanced × Final), compared with the Iray
references in `calib/references/iray` (phase correlation, per-tile offsets,
bokeh crops 1100–1700 × 430–640 px, box-blurred).

| Scene | Iray shows | Octane as exported (b0dafc4) | Now |
|---|---|---|---|
| Shift (0.5, −0.3, 65 mm) | content 349.5 px right, 209.7 px down | 108 px left, 65 px up (wrong sign, 3.2× short) | 351.0 / 210.6 (shift × F / 2), 1.7 / 0.9 px off |
| Stereo (10 cm) | a featureless beige gradient — no scene | content 31 px left | unchanged (manual's reading) |
| Blades5 (20°) | pentagon, a corner at 92.5° | the degrees clamped to a full turn: a corner up, 2.5° off by luck | (90° − 20°) / 360° |
| Bias02 | ring bokeh | uniform: RMSE 5.17 | edge 2.5: 2.72; OSL ξ^0.2: 2.38 |
| Bias08 | peaked centre | uniform: 4.15 | edge 0.625: 2.76; OSL ξ^0.8: 2.41 |
| Thick (50 mm) | Calib-DOF-f4 pixel for pixel | — | not mapped |
| Poly3, InvPoly5, Brown, Equidistant, Spherical | — | every tile within 1 px, edge corr. 0.973–0.979 (base scene 0.975) | unchanged |
| Cylindrical | 360° across, vertically zoomed | square pixels: mean \|Δ\| 45, edge corr. 0.17 | film y / F up the cylinder: 1.8, 0.972, tiles 0 px |

Calib-DOF-f4 itself: bokeh RMSE 2.39 — the noise floor of these crops.

- **Shift.** Subpixel phase correlation, Calib-Tone-Default → -Shift:
  0.3640 image widths per unit in x, 0.3641 (as widths) in y — isotropic.
  F / 2 = 3.25 cm per unit predicts 0.3656 (0.4 % wide); π (0.3534) and
  the film width are out. With one focal length a constant 3.24 cm can't be
  ruled out; F / 2 (the shift as half a tangent) is the reading kept. The
  image moves *with* the shift; Octane's `lensShift` moves the view, so it
  is negated (both axes).
- **Cylindrical.** Iray's own Calib-Lens-Spherical (a full 360° × 180°
  lat-long, confirmed) used as the environment seen from the camera:
  synthesising azimuth A·(u − 0.5), ray height B·(v − 0.5) and correlating
  with the reference gave A 6.28 (2π), B 0.770 (corr. 0.9997) — B = film
  height / focal length 0.769: perspective up the cylinder. The same fit on
  Octane's old render returned B 3.53 = 2π · H / W, the square pixels
  written — the method checks out.
- **Blades.** The first rotation sweep (`bokehRotation` 0–60 in degrees)
  rendered six identical frames (mean |Δ| 0.24, noise): a pin probe of
  NT_CAM_THINLENS showed `bokehRotation`'s range 0–1, `aperture_edge`'s
  0–3, `lensShift` "a proportion of the image width/height". Swept again as
  fractions (0–60° in 12° steps): Octane's corner is up at 0 and turns
  clockwise; the RMSE against Iray is lowest near 0 (parabola: −2 to −3°),
  so Iray's 20° puts a corner at 92.5° ≡ 20.5° — the blade rotation from
  +x, counter-clockwise (the background bokeh is the lens upright). Seen
  on 5 blades only: an even count could start flat-topped in Octane.
- **Radial bias.** The guess "Iray samples the lens radius as ξ^b, Octane
  as ξ^(1 / (2·edge))" picked 2.5 and 0.625 in advance; both won their
  sweeps. An OSL camera forced onto the scene (inverse poly3, k 0 — the
  identity) with ξ^b sampled lands on the noise floor (2.38 / 2.41), so ξ^b
  is Iray's model; the thin lens's `aperture_edge` gets within 0.35.
  Octane's edge stops at 3 (b < 1/6).
- **Thickness** does nothing in Iray (mean |Δ| 0.00 vs Calib-DOF-f4).
- **Stereo.** The Iray Server rendered no scene at all into the
  single-layer canvas; nothing to compare. The parallel right-eye move
  stays as the manual reads; a DAZ-side stereo render would be the only
  check, not worth a batch.

Timings: 14 scenes at 400 samples ≈ 36 s each; the variant runs waited on
the render lock behind the #43 bloom batch.

## Lessons

- The .mi fixtures first: one DAZ run settled every channel conversion.
- Octane's OSL camera is the way to reproduce any of Iray's lens models
  exactly; `camera:dofrandom` keeps depth of field with it. The `.ocs`
  attribute name `shaderCode` was a guess from `A_SHADER_CODE` (as
  `filename` ↔ `A_FILENAME`) that loaded first time.
- `bokehRotation` in degrees went unnoticed because a clamped value still
  renders a polygon: when an Octane pin "does nothing", probe its pin info
  (range, units) before fitting.
- With other sessions running `clean-cache.sh`, variant `.orbx` files in
  `~/Library/Caches/orbis` vanished between export and render; keep
  experiment exports in the session's scratch directory.
- Iray's own spherical render is a free environment probe: any other
  projection from the same camera can be synthesised from it and fitted.
