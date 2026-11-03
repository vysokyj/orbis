# 2026-11-03 — light options (#35)

Goal: DAZ's light options — Render Emitter, Light Portal, Cast Shadows,
Shadow Intensity / Softness / Colour, Decay, Illumination (diffuse /
specular only), Temperature — in Octane. Variant enhanced-final, 200
samples (the calibration default). Rule in the spec, Lights → Light options.

## What Iray does with them

The Iray references of the Calib-L-Opt-* scenes (5000 samples, rendered
2026-11-02), each against Calib-L-Point-5k, linear, whole frame:

| scene | mean R / G / B | mean abs diff | max diff |
|---|---|---|---|
| CastShadowsOff, Decay0, Decay1, EmitterOff, IllumDiffuse, IllumSpecular, ShadowColor, ShadowInt0, ShadowInt05, ShadowSoft05 | 1.000 / 1.000 / 1.000 | 0.0000 | 0.008 (noise) |
| Temp3000 | 1.696 / 0.861 / 0.267 | 0.057 | 0.746 |

Ten of eleven options change nothing. The scenes did carry them (the
channel values checked in the `.duf`). The `.mi` export (headless DAZ,
`daz-export-mi.dsa`) says why: with the object ids normalised, each `.mi`
equals the base's except

- Temp3000: `"temperature" 3000` in the light material;
- EmitterOff: `visible off` on the light (`on` otherwise);
- everything else: identical.

So DAZ hands Iray none of the 3Delight options. EmitterOff on a point
light changed nothing either: Iray's `visible` is for camera rays (Iray
manual, ghost lights), and a point has no face to see.

Light Portal needs an area light and a dome: new scenes on Calib-L-Rect-50
(which is Scene Only): Calib-L-Opt-Rect-Dome (Environment Mode 0) and
-Rect-Portal (the same, Light Portal on). Their `.mi` exports are identical
too: NVIDIA's release notes list portal lights as removed from Iray
Photoreal in 2022.0; DAZ Studio 6 runs Iray build 387700. No Iray render
needed — what Iray gets is the same.

The `.mi` exports became fixtures (`calib/references/mi/Calib-L-Opt-*`);
`iray::Light` gained `visible` (Render Emitter, off for the headlamp, whose
`.mi` light is `visible off` in every Calib-Headlamp-* fixture), tested
against the `.mi` `visible` field of every fixture light.

## Octane

1. **Render Emitter off as `camera_visibility` 0** on the light's object
   layer input `/Scene/Lights/<prim>_object` (the name probes 100–102
   found). Pin names from a pins probe (`octane-probe-pins.lua` on types
   65 and 402): `camera_visibility` on the object layer, `objectLayer` on
   the analytic light.
2. To see it, Calib-L-Rect-50's rect is above the frame (vertical field
   24.4°: Frame Width 50, Focal Length 65). First Facing try (y 150 cm,
   X −120°): still out of the frame (15.7° above the middle; I had taken
   the `.mi` aperture for the field). Second (y 100, X −150°): 6.6° above
   the middle, facing the camera — Calib-L-Opt-Rect-Facing and
   -Facing-EmitterOff.
3. Rendered: **both black where the rect is** (max 0 in rows 0–460). The
   Octane USD import hides a light's emitter from the camera by default.
   The sphere's reflection of it was there (Calib-L-Rect-50: highlight
   255, as in Iray), so the light is reflected, just not seen directly.
4. Hack: every light's layer with `camera_visibility` **1** — the rect
   showed (45 037 pixels, rows 271–459, columns 834–1084).
5. So the rule turned round: Render Emitter **on** writes the input with
   `camera_visibility` 1 for rect / disc / sphere lights; off writes nothing
   (the default hides it). Points (1 cm sphere) and the headlamp (the
   camera sits in it) stay hidden.

After the change:

| scene | Octane / Iray floor (front, left, right) |
|---|---|
| Calib-L-Rect-50 | 1.01 1.01 1.01 |
| Calib-L-Rect-50-2sided | 1.01 1.01 1.01 |
| Calib-L-Disc-50 | 1.01 1.01 1.01 |
| Calib-L-Sphere-d20 | 1.00 1.01 1.01 |
| Calib-L-Mi-Sphere-30x60 | 1.00 1.01 1.01 |
| Calib-L-Point-5k | 1.00 1.01 1.01 |
| Calib-L-Opt-* (ten unmapped) | 1.00 1.01 1.01 |
| Calib-L-Opt-Temp3000 | 1.01 1.02 1.02; R / G / B 1.03 / 1.01 / 0.98 |
| Calib-Headlamp-on-a0 | frame 1.04 (as before) |

Facing-EmitterOff against Facing (Octane): the rect gone (0 pixels > 30),
the rest of the frame 0.999 ×.

Spots (analytic lights, 402) the other way round: Calib-L-Opt-Spot-Facing
(Calib-L-Mi-Spot-rect's 20 × 40 cm rect spot turned up, X +90°, to the
camera above it) shows the quad without any object layer (22 306 pixels
> 30, rows 434–645); -Spot-Facing-EmitterOff with the `objectLayer`
`camera_visibility` off: 0 pixels, frame mean 0.0.

None of the other area-light references has the emitter in the frame, so
the change shows nowhere else. Rect-Facing / Spot-Facing and their EmitterOff twins have no Iray
reference yet (#35, "Iray batch: needs"): they would show the rect's face
brightness against Iray's and confirm the reflection stays with
Render Emitter off.

## Not mapped (Iray ignores them)

Cast Shadows, Shadow Type / Intensity / Softness / Colour / Bias, Decay,
Illumination 1 / 2, Light Portal. Octane has counterparts (emission
`castShadows`, `Visible on diffuse` / `Visible on specular`, the portal
material), but they would light the scene unlike the Iray render its
author saw. Illumination 0 (Off) was left out of the export already.

## Process

- Diff the `.mi` (ids normalised) before rendering anything: it settled
  ten of the eleven options in one headless DAZ run of about a minute.
- Check that a test scene puts what it tests in the frame — compute the
  field from Frame Width / Focal Length, not from the `.mi` camera.
