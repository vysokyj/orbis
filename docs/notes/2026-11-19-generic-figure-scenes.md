# 2026-11-19 — generic figure scenes for the deleted teens scenes (#203)

The private teens scene and the scenes cut from it (Calib-Hair-Teens-*,
Calib-Disp-Teens, Calib-Feet-Teens) were deleted on 2026-11-18. This
session built generic stand-ins from free content only (Genesis 9 Starter
Essentials: Genesis 9, G9 Base Shirt, G9 Base Shorts, G9 Base dForce Pixie
Hair, the G9 Base Poses). No renders yet: the Iray Server did not answer
and the screen was locked (Octane X needs an unlocked screen for its Script
menu). The lab notes that cite teens numbers stay as they are.

## What each teens scene measured, and its stand-in

| teens scene | measured | stand-in |
|---|---|---|
| Calib-Hair-Teens-* | Morgan Hair (store, ribbons, sides 2) whole, without gloss, with / without its coat; the ribbon / fibre split of #161 | `Calib-Hair-Pixie-T2`, `-T1`, each `-nogloss`, `-coat` (beside `Calib-Hair-Pixie`, sides 3, and `-nogloss`) |
| Calib-Disp-Teens | displacement maps on a figure's clothes (3 – 5 mm, SubD Displacement Level 2) at about 1 mm per pixel: seams closed, the relief | `Calib-Disp-G9` (ridges on shirt and shorts), `-off` (strength 0), `-Skin` (the ridges on the PBRSkin surfaces) |
| Calib-Feet-Teens | wearables in a stale fit after opening (#51): shoes off the feet | `Calib-Fit-G9` (shorts on the stepping thighs, bare feet on the floor) |
| teens (export memory / timing) | a 1.2 M-face scene with strand hair | `Calib-Big-G9` (eight figures, 13.7 M faces) |

- **Hair.** The Pixie hair is Uber in PBR metallicity mode with a dual lobe
  specular and no glossy layer, so `-nogloss` takes its dual lobe off (as
  `Calib-Hair-Pixie-nogloss`). Sides are set on the hair's own Line
  Tessellation modifier, Render and Viewport alike (DAZ's `.mi` export reads
  the Viewport value, the render the Render one). `-coat` puts a white top
  coat on its four strand materials (weight 1, reflectivity mode 0.5,
  roughness 0.2): Iray's coat on fibres was never measured (2026-11-14
  note). The Enhanced exports of -T2 and -T1 are the same size (Enhanced
  keeps round curves on both; the ribbon rule is Faithful's, #161).
- **Displacement.** The map is ours: `calib-disp-ridges.png`, 1024², 16
  horizontal sine ridges over the UV square, 8-bit grey with no gamma of
  its own (DAZ decodes it at 2.2), 0 – 0.5 cm at strength 1. The export
  writes 7 displacement nodes for `Calib-Disp-G9` (the shirt's and the
  shorts' surfaces), none for `-off`, 4 for `-Skin`. Camera: 65 mm, 66 cm
  from the shorts, about 0.6 mm per pixel; the shorts take rows 196 – 580
  of 720 (dumped points projected through the scene camera).
- **Fit.** No free Genesis 9 shoes exist, so the teens shoe check has no
  free twin; the shorts on Base Pose 13 Walking (one thigh forward) and the
  bare feet on the floor are what a stale fit would move. The frame runs
  from the floor to the shirt's hem (shorts rows 114 – 265, the feet near
  the bottom).
- **Big.** Eight figures in a 4 × 2 grid, each in another base pose, with
  shirt, shorts and hair, at DAZ's SubD render level 2. Too big for the
  Iray Server (4 GB VRAM) and for an Octane series (#50), so it is never
  queued: it is an export measurement.

## Builder

`scripts/dev/daz-build-figure-scene.dsa` takes the base scene, the out file
and a list of `figure:<x>,<z>,<yaw>:<duf>`, `pose:<duf>`, `wear:<duf>`
items; poses and wearables go on the last figure loaded, and the figures
are placed after posing (a pose preset may carry the root's transform).
One figure with three wearables builds headless in 16 s, eight in about
45 s. `scripts/dev/make-figure-scenes.py` aims the camera at a box (65 mm,
the box's height or width filling the frame), sets Calib-Hair's exposure
(EV 15.7), the dome drawn, no ground, 1280 × 720, SubD render level 1 on
the one-figure scenes, and writes the displacement variants. Both
generators now blank the contributor DAZ saves into a scene, as `demo.py`
does for Demo-*.

## Timings

| scene | dump + export | export from the cached dump | meshes, faces | peak footprint | `.orbx` |
|---|---|---|---|---|---|
| Calib-Fit-G9, -Disp-G9* | 31 s each | 4.0 s | 14, 0.81 M | — | 243 MB |
| Calib-Hair-Pixie-T* | 29 – 32 s each | 3.5 s | 12, 0.55 M | — | 205 MB |
| Calib-Big-G9 | 171 s | 22.1 s (23.8 s user) | 105, 13.7 M (39.7 M points) | 2.58 GB (max RSS 2.82 GB) | 1.30 GB |

The big scene's dump is 852 MB. It shares its 35 textures between the
eight figures, so it measures the mesh and layer side of the export (#16),
not the texture baking (#78: intake had 956 textures).

## Open

The Iray batch and the Octane side are parked in #203.
