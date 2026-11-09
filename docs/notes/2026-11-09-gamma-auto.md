# 2026-11-09 — gamma "auto" and a PNG's gAMA chunk (#90)

Question: the NVIDIA presets' bump PNGs (concrete_plain_bump,
asphalt_bump, ceramic_tiles_small_bump) carry a `gAMA` chunk of 1.0 and
DAZ's .mi hands them to Iray with `gamma "auto"`. The same maps weight or
tint the gloss (`MdlGloss::Map`), which the export decodes at 2.2. If
Iray honoured the chunk, the gloss weight would be off (grey 0.5: 0.22
vs 0.5). Docs first, then decompose.

## What the docs and Iray's loader say

- MDL spec 20.3: `gamma_default` "leaves this decision up to the
  texture itself". The Iray programmer's manual (example_mdl): a PNG's
  canvas "has already a gamma value of 2.2". Neither says where the
  canvas gamma comes from.
- DAZ Studio 6 runs Iray 2025.0.3 (build 387700, DAZ's log) and reads
  images through `nv_openimageio.so` (OpenImageIO, per the Iray release
  notes). The MDL SDK publishes that plugin's source (BSD); the 2025
  release (387700) and later: `get_pixel_type_and_gamma()` gives the
  canvas a gamma by pixel type — 8-bit (1 or 3 – 4 channels) 2.2, half
  / float 1.0, a 16-bit single channel 1.0 (another debug option,
  default on) — and reads the file's colour metadata (`oiio:ColorSpace`:
  linear → 1, sRGB → 2.2, GammaX → X; OIIO sets it from PNG gAMA / sRGB)
  only when the debug option `ignore_gamma_metadata_on_import` is 0. Its
  default is 1 (`g_ignore_gamma_metadata_on_import = true`). The
  2024.1 plugin and older have no `get_gamma_from_metadata` at all.
- The option string occurs only in `nv_openimageio.so` in DAZ Studio's
  bundle — DAZ never sets it. So under "auto" an 8-bit PNG is 2.2
  whatever its gAMA, sRGB or iCCP chunk says.
- The export never reads gAMA either (the `image` crate ignores it; the
  gamma comes from the .mi / DAZ channel: 2.2 for "auto" 8-bit maps,
  `map_gamma` when an image sets one, #49), for the Uber and the presets
  alike — consistent with Iray. Octane gets the original PNG with the
  gamma pin in "linear sRGB + legacy gamma".

A cross-check before any render: the presets' **colour** PNGs carry the
same gAMA 1.0 (asphalt.png, concrete_plain.png, ceramic_tiles_small.png,
concrete_fine_polished.png — RGB, the bump maps palette). Mean
luminance at 1.0 / 2.2: asphalt 0.217 / 0.035 (6.1 ×), concrete 0.442 /
0.168 (2.6 ×), tiles 0.776 / 0.583 (1.3 ×); bump maps asphalt 0.518 /
0.264, concrete 0.611 / 0.340, tiles 0.652 / 0.489. The export reads the
colour maps at 2.2 and the disks match Iray (asphalt 1.00).

## Decompose: the gloss map at 2.2, at 1.0, off

Octane 1024 samples, enhanced-final; temporary env switch in the export
(removed). Disk = linear luminance r ≤ 0.75 of the sphere's radius, rim
0.75 – 0.95 (where Fresnel lifts the gloss), Octane / Iray. "all 1" =
the hypothesis "Iray honours gAMA": colour and gloss maps at 1.0.

| sphere | 2.2 (export) | gloss map 1.0 | all 1.0 | gloss off |
|---|---|---|---|---|
| concrete (Calib-NVIDIA) | 1.04 / 1.05 | 1.05 / 1.07 | 2.75 / 2.76 | 1.03 / 1.03 |
| asphalt (Calib-NVIDIA2) | 0.99 / 1.00 | 1.17 / 1.26 | 5.42 / 5.16 | 0.83 / 0.77 |
| ceramic tiles small | 1.05 / 1.06 | 1.05 / 1.06 | 1.39 / 1.40 | 1.05 / 1.05 |

Granite (no map) 1.10 / 1.08 in every column. Asphalt is the only sphere
where the gloss is a large share (its diffuse is 0.035): diffuse alone
0.83 of Iray's disk, so Iray's gloss is ≈ 0.17 of it; Octane's gloss at
2.2 adds 0.16, at 1.0 0.34 (the map's 1.96 × mean ratio). No Iray
glossy_weight 0 variant needed — the diffuse-only Octane disk plus the
docs settle it. Concrete's and the tiles' gloss is a few percent of
their disk (concrete +0.014 at 2.2, +0.025 at 1.0; tiles' bright map
differs by 1.3 × only), so they can't tell.

Result: the export's 2.2 is right; no code change.

## Open: the bump

The same loader means Iray reads the presets' (and any "auto") 8-bit
height maps at 2.2 too, while #89 bakes them linear on Cast Iron's
render (detail 1.05 × linear vs 1.13 × at 2.2). The Uber's height maps
are linear by an untested rule as well. Not changed here: #91, to be
decomposed on a synthetic height map (slope against a 1.0 / 2.2 decode)
first.

## Process

The fitted-camera scratch copy of Calib-NVIDIA2 from 2026-11-05 was gone
(the n21 copies have no `current_camera`): an export of them renders the
saved camera — asphalt came out 10 × Iray. Rebuilt from the 11-05 note's
fit (Camera-1 at (142.9, 188.1, 364.6) cm, X −13.96°, Y 21.99°, 89.1 mm,
`current_camera` set) in the cache; it reproduced #89's numbers (asphalt
0.99, tiles 1.05). `orbis export --dump` moves the dump into the cache
under the new scene's key — copy a library dump before passing it, or
the library scene loses its cached dump.
