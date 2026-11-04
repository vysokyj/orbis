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
the change shows nowhere else.

### Against Iray (2026-11-04)

The four Facing scenes got Iray references in the 2026-11-04 batch (Iray
Server, 5000 samples, fe12e27). Octane enhanced-final, 1000 samples, the
export as of bdd887a, no change. Face = pixels > 30 in Facing and ≤ 30 in
its EmitterOff twin, eroded 2 px; linear = 8-bit decoded with gamma 2.2.

| | Iray | Octane |
|---|---|---|
| Rect-Facing face pixels | 47 144 | 45 947 |
| its box (rows, columns) | 270–471, 835–1083 | 273–469, 836–1083 |
| interior saturated (255) | 100 % | 100 % |
| rest of the frame, mean linear | 0.0880 | 0.0882 (1.002 ×) |
| -EmitterOff, whole frame | 0.0857 | 0.0860 (1.003 ×) |
| EmitterOff / Facing, rest of the frame | 0.996 | 0.998 |
| sphere highlight (the rect's reflection), integrated over background | 8.63 | 8.27 (0.96 ×) |
| the same, -EmitterOff | 8.62 | 7.83 (0.91 ×) |
| sphere highlight peak | 0.044 | 0.097 |
| Spot-Facing face pixels | 22 221 | 21 129 |
| its box | 433–645, 903–1014 | 436–643, 908–1011 |
| interior saturated | 100 % | 100 % |
| -Spot-Facing-EmitterOff, frame mean | 0.0 | 0.0 |

- Both faces are white at the scenes' exposure in both renderers: the 8-bit
  references cannot tell their radiance apart beyond that. The rect's
  reflection in the sphere is not saturated and stands in for it: the same
  energy to 4 % (Facing) / 9 % (EmitterOff, Octane noisier at 1000
  samples). The highlight is sharper in Octane (peak 2.2 ×, same energy):
  the sphere's gloss, not the light (not this issue).
- Render Emitter off: the face goes in both, the light and its reflection
  stay (Iray reflection unchanged to 0.02 %, the frame 0.996 ×).
- The spot quad's face matches in place and size to 1–5 px a side (Iray's
  pixel filter spreads a saturated edge a little further); off, both black
  (the spot points away from everything).
- No HDR references needed: what the author sees in Iray at the scene's
  exposure is what Octane shows.

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
