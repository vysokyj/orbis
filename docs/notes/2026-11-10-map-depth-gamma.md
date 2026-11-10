# 2026-11-10 — gamma "auto" by pixel type on every map (#93)

Goal: #91 left the height and normal maps decoding "auto" by the file's
pixel type, while the float channels' maps (`data_gamma`) and the colour
maps took 2.2 for any "auto" map, so a 16-bit roughness or displacement
map was read too dark. One rule for every map, verified against Iray.
Variant enhanced-final, 400 samples, half the work on Octane only.

## The loader's rule, read again

#90's note summed up the MDL SDK's OpenImageIO plugin as "8-bit 2.2,
half / float 1.0, a 16-bit single channel 1.0" and #91 coded it as
"deeper than 8 bits → 1.0". The source (`openimageio_utilities.cpp`,
`get_pixel_type_and_gamma_internal`, identical in tag 2025.0.3 = DAZ
Studio 6's Iray and in 2026.0.2) says more:

| channels | uint8 | uint16 / int16 | half / float |
|---|---|---|---|
| 1 | 2.2 | **1.0** (`g_linear_gamma_for_single_uint16_channel`, default true) | 1.0 |
| 2 with alpha, 4 | 2.2 | **2.2** | 1.0 |
| 3 (RGB) | 2.2 | **2.2** | 1.0 |
| 2 without alpha | undefined | undefined | 1.0 |

So a 16-bit **RGB** (or grey + alpha) map is 2.2, not 1.0 — #91's
`auto_gamma` took it linear (its doc said "unverified"). Fixed:
`daz_material::auto_gamma` maps the decoded `image::ColorType` (L16,
Rgb32F, Rgba32F → 1.0; everything else, La16 / Rgb16 / Rgba16 included,
2.2). Metadata stays ignored (`g_ignore_gamma_metadata_on_import`, #90).

## What DAZ hands Iray (.mi)

Calib-MapDepth's and -MapDepth2's `.mi` (`daz-export-mi.dsa`, fixtures
`calib/references/mi/Calib-MapDepth*.json`): every map at `map_gamma` 0 —
8-bit, 16-bit grey, 16-bit RGB, the float TIFF — goes as `g_Auto`, the
16-bit grey at `map_gamma` 2.2 as `gamma 2.20000005`, the 8-bit grey at
`map_gamma` 1 as `gamma 1` (on the diffuse too, as Calib-Mi-ImageGamma
showed). Single-channel files go as `scalar texture` on any channel, the
16-bit RGB as `color texture`. `src/iray.rs` passes both fixtures
unchanged.

## The export: one place

- `Textures::map_gamma(asset, own)`: the channel's own gamma, else the
  asset's recorded gamma, else "auto" by the header (`file_gamma`).
  `own_gammas` comes from the materials' channels (one gamma per file, as
  `scene_image_gammas`), so a colour pin, which never knew its channel,
  now reads an image's own gamma too.
- Every `image_pin` takes it: the float channels (was `data_gamma`, 2.2
  for any "auto"), the colour pins (were a literal 2.2), displacement, the
  overlay weight (× 2 when squared).
- Generated images record the gamma they are written at (derived 1,
  bakes 2.2, baked normal maps 1); a grey channel pick (#77) keeps 16 bits
  and its source's gamma.
- `derived_mono` decodes its source at the source's gamma — 8 bits by the
  old table at 2.2 (byte-identical), else a 256- or 65536-entry table;
  roughness and IOR lost their `g / 2.2` rescale. The colour bakes (skin,
  blend) go through `open_colour`: the file as before when Iray reads it
  at 2.2, else decoded at its gamma and re-encoded to 8-bit 2.2.
- The `image` crate is built without OpenEXR / HDR and can't read a
  one-channel float TIFF: those count as float (by extension, or the
  TIFF's SampleFormat 3) instead of falling back to 8 bits.

Found on the way: the weighted (Layering Mode 1) and anisotropic
(layered) top coats read their weight map at a literal 1.0 since #25 /
#23 — older than #49's 2.2 for float maps, which the plain coat had. Now
the same rule.

## Byte-identity

`hashall.sh` (scratch): export every library scene with a cached dump
(250, 31 without one fail both ways) with the old and the new binary and
compare the `.orbx`. Different:

- Calib-NVIDIA-BumpG1, Calib-BumpGamma-NVIDIA: the NVIDIA Concrete's
  gloss map (`concrete_plain_bump.png`, `MdlGloss::Map`) has its own
  `map_gamma` 1 in these scenes; the gloss pin took a literal 2.2, now 1.
- Truck (`lashes diffuse.jpg`, 3 colour pins), intake
  (`M3DSHSColor04.jpg`, `Sara_Sclera.jpg`): colour maps at 1 instead of
  2.2 — the scenes' `image_library` lists them with `map_gamma` 1, which
  DAZ hands Iray on every channel (the .mi of Calib-MapDepth2's C5 shows
  it on a diffuse); the colour pins ignored an image's own gamma.
- teens (private): 11 pins of specular / coat colour maps (the shoes'
  `-spec`, the skin's `…S_100x` maps) at 1 — listed with `map_gamma` 1
  too.
- Calib-BumpGamma-NVIDIA also: its 16-bit grey bump map, which the
  Concrete reads as its gloss weight too, now at 1 there.

Octane, the changed calibration scenes against their Iray references
(enhanced-final, 400 samples): Calib-NVIDIA-BumpG1 spheres 0.99 0.96
0.52 1.01 0.99 1.11 1.07 0.94 1.06 — the old binary's export rendered
the same to 0.01 (the concrete's gloss is a few percent of its disk,
#90); the concrete's 0.52 is there before and after (not #93).

Every other scene, Demo-Gladiator and the skin / hair / overlay / bump
calibration scenes included, exports byte-identical.

## Octane

Octane reads the float TIFF (C4) and both 16-bit PNGs. Calib-MapDepth (enhanced-final, 400 samples), the spheres' top edge (px,
sub-pixel, `silhouette.py` in scratch; no displacement 514.6):

| sphere | gamma | top | Δ px | Δ cm |
|---|---|---|---|---|
| D1 8-bit | 2.2 | 508.3 | 6.3 | 2.1 |
| D2 16-bit grey | 1 | 499.9 | 14.7 | 4.8 |
| D3 16-bit RGB | 2.2 | 508.2 | 6.4 | 2.1 |
| D4 16-bit grey, `map_gamma` 2.2 | 2.2 | 507.9 | 6.7 | 2.2 |

(3.06 px / cm from Iray's Calib-Disp-Map white sphere, 30.6 px for
10 cm; expected 2.2 / 5.0 cm.) Octane applies the legacy gamma pin to a
16-bit PNG as to an 8-bit one (D4 = D1), and reads a 16-bit PNG at pin 1
as its linear value (D2): the export passes 16-bit files as they are.

## Iray

Iray Server, 5000 samples (129 s / 106 s); Octane enhanced-final 400
samples, the export now and the previous binary's.

Calib-MapDepth, the top edge (px):

| sphere | Iray | Octane now | before |
|---|---|---|---|
| D1 8-bit | 508.3 | 508.3 | 508.3 |
| D2 16-bit grey | **500.1** | **500.2** | 508.8 |
| D3 16-bit RGB | 508.1 | 508.1 | 508.1 |
| D4 16-bit grey, `map_gamma` 2.2 | 507.9 | 507.9 | 507.9 |

Iray reads the 16-bit grey linear and the 16-bit RGB at 2.2, as the
loader's source says; the export now agrees to 0.1 px.

Sphere-centre linear luminance (median of a 24 px box), Octane / Iray:

| scene | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| MapDepth now | 1.03 | 1.03 | 1.03 | 1.03 | 1.16 | 1.12 | 1.03 | 1.04 |
| MapDepth before | 1.05 | 1.02 | 1.03 | 1.05 | 1.16 | 1.12 | 1.04 | 0.97 |
| MapDepth2 now | 1.04 | 1.03 | 1.03 | 1.03 | 1.04 | 1.04 | 1.04 | 1.04 |
| MapDepth2 before | 1.04 | **0.45** | 1.03 | **0.45** | **0.45** | **1.43** | 1.04 | 1.07 |

(MapDepth: D1–D4, R1 R2 roughness 8 / 16 bit, O1 O2 opacity 8 / 16 bit;
MapDepth2: C1 8-bit, C2 16-bit grey, C3 16-bit RGB, C4 float TIFF, C5
8-bit at `map_gamma` 1 diffuse; T1 T2 weighted coat weight 8 / 16 bit,
T3 anisotropic coat 8 bit. The scene's matte level is 1.03 – 1.04.)

- The diffuse colour maps Iray reads linear (16-bit grey, float, own
  gamma 1) came out at 0.45 × read at 2.2; now 1.03 – 1.04.
- The weighted coat's 8-bit weight map at the literal 1.0: 1.43 ×; at 2.2
  1.04. The 16-bit one was right before by chance (1.0 = its "auto").
- Opacity 16 bit 0.97 → 1.04 (beside the 8-bit 1.03).
- Roughness: the centre box doesn't tell 0.22 from 0.5 on these spheres
  (1.16 / 1.12 before and after, the scene's gloss offset as Calib-
  DataGamma's G7); the displacement and opacity spheres carry the
  float-channel case, the code path (`map_gamma`) is the same.

Result: the rule holds for every map kind tested. No fitted constants.

## Process

- Byte-identity check by exporting every library scene with a cached
  dump with both binaries (sha of the `.orbx`) before rendering
  anything: it found the own-gamma colour pins (Truck, intake, teens)
  before any render did.
- `make-calib-scenes.py` writes a new scene's maps once; a 16-bit RGB
  PNG needs its own writer (`png16`, PIL can't write one).
