# Octane export — DAZ scene → `.orbx` for Octane X

The normative description of what ORBIS exports and why — every rule with
the measurement behind it. Code that disagrees is a bug, or this page is
wrong and changes in the same commit. Background on the other tools:
[DAZ files](../daz/files.md), [Iray](../daz/iray.md),
[Octane X](../octane/octane-x.md), [ORBX](../octane/orbx.md),
[USD](../octane/usd.md); how the numbers are measured:
[calibration](../calibration.md); all documents: [docs](../README.md).

**Contents** — [Goal](#goal) · [Look](#look) · [Inputs](#inputs) ·
[Plugin](#plugin) · [Geometry](#geometry) · [Package](#package) ·
[Render targets and cameras](#render-targets-and-cameras) ·
[Render modes](#render-modes) (tone mapping) · [Materials](#materials)
([Universal](#universal-130--default), [Skin](#skin--standard-surface-277-final-only),
[Hair](#dforce-hair--hair-material-147), [Emission](#emission)) ·
[Lights](#lights) · [Environment](#environment) (Sun-Sky,
[ground shadow](#ground-shadow)) · [Textures](#textures) ·
[Opening](#opening) · [Open work](#open-work)

Code: `src/`. Run: `cargo run --release -- export <scene.duf> -o
<out.orbx|out.usdz>` (options in the [README](../../README.md#command-line)).
Calibration notes cite their scenes (`Calib-*`, see
[calibration](../calibration.md#the-scenes); *Octane1*–*4*, *Fetroom*,
*Gladiator*: the developer's production scenes) and Lua probe numbers;
numbers are linear light unless marked sRGB.

## Goal

**The scene as Iray lights it, rendered the way Octane renders.** The
export matches what the DAZ scene *means* — the amount of light, exposure,
colours, which surfaces reflect, transmit or glow, the sun and sky balance
— measured against Iray. Where Octane does something more plausibly than
Iray (a low sun's warm colour, how a gloss layers over the diffuse, a
spot's soft edge), its behaviour stays: no Iray quirk is imitated at the
cost of realism.

## Look

`--look enhanced|faithful` (the dialog's Look; `Look` in the code): where
Iray and Octane differ in a way the export can choose. *Enhanced* (the
default) keeps Octane's take: skin on **OpenPBR Surface** (OTOY's material
for skin), the Hosek-Wilkie daylight that warms a low sun, thin-walled
glass with its Fresnel reflection. *Faithful* stays with Iray's: skin on
Standard Surface tinted to Iray's, Octane's daylight model with a white
sunset colour, dimmed by the sun's air mass as Iray dims it (Variants),
thin-walled glass at IOR 1 (Iray's showed none of its own reflection).
**Denoise** (`--no-denoise` off; the dialog's Denoise) is separate: Octane's
AI denoiser on the Final render — it changes no light or colour, it may
soften fine detail.

### Variants

An export is one of four variants: Look × render mode
([Render modes](#render-modes)). Everything that differs between them is a
field of `Tuning`, set per variant in one table (`Variant::tuning`,
`src/tuning.rs`) — this one. Everything not listed is shared and fitted on
**Enhanced + Final**; each variant gets the same calibration scenes in turn
(docs/calibration.md, [#44](https://github.com/vysokyj/orbis/issues/44)).

| `Tuning` field | Enhanced + Final | Faithful + Final | Enhanced + Preview | Faithful + Preview |
|---|---|---|---|---|
| `kernel` | path tracing | path tracing | direct lighting | direct lighting |
| `base_exposure` | 1.4 | 1.4 | 1.4 | 1.4 |
| `denoiser` | on | on | off | off |
| `skin` | OpenPBR Surface | Standard Surface | Universal (no SSS) | Universal (no SSS) |
| `thin_glass_ior_one` | no (Fresnel) | yes | no | yes |
| `daylight_model` | 3 Hosek-Wilkie | 1 Octane, white sunset | 3 Hosek-Wilkie | 1 Octane, white sunset |
| `daylight_power` | 2.7 | 2.864 | 2.7 | 2.864 |
| `daylight_extinction` | 0 | 0.200 | 0 | 0.200 |
| `max_transmission` | 1 | 1 | 0.75 | 0.75 |

Calibrated on the same 17 scenes in each variant (lights, Sun-Sky at
18:00, noon and steered, HDRI, tone, PBRSkin, Glossy, SSS, Glass, Mixing,
Leaf, Hair; `scripts/dev/calib-variants.sh all`), Octane / Iray:

- **Lights, HDRI, Sun-Sky** (floor): 0.98 – 1.04 in all four.
- **`base_exposure`**: Preview at the old 1 lit every floor 0.71 – 0.74 ×
  Iray; at 1.4, as Final, 0.99 – 1.04. Spheres under direct lighting stay
  ~10 % darker than under path tracing (Calib-Tone-Default 0.82 – 0.94 vs
  0.98 – 1.15): two bounces, not a light amount.
- **Faithful daylight**: Octane's model hardly dims a low sun — at one
  power (2.27) Calib-L-Mode-SunSky-off (18:00, 15.5°) came out 1.86 × and
  -noon (40°) 1.08 ×; a grey sunset colour (0.5 – 0.15) left 18:00 as it
  was. `power` × exp(−`daylight_extinction` / sin(elevation)) — the sun's
  air mass — through both: 18:00 0.95, noon 1.00, SunSky-dir (43°) 0.98.
  The floor isn't linear in Octane's power there (18:00 at 1.22 / 1.32 /
  1.34 / 1.39: 0.85 / 0.92 / 0.94 / 1.12 ×). Enhanced's Hosek-Wilkie does
  the dimming itself: 0.99 / 0.98.
- **`max_transmission`**: direct lighting renders a surface that passes
  all of its diffuse on as transmission black (Calib-SSS's Translucency
  Weight 1 sphere: 0.00); capped at 0.75 it is 0.66, with its neighbours
  (0.57 – 0.74; Final 0.93 – 1.04). Without transmission (albedo × (1 −
  0.67 w)) the weight-0.5 spheres fell to 0.38 – 0.49.
- **Skin** in Preview (Universal): Calib-PBRSkin 0.86 – 1.16, Final's
  OpenPBR 1.06 – 1.29 (before #19's rule). Faithful's Standard Surface skin
  keeps the older `skin_tint` rule, not refitted on Calib-Skin. **Hair** 1.04 (Final) / 1.06 (Preview).
- Shared misses, not variant-bound: Calib-Glass sphere 4 2.8 × in all
  four; Preview's Glass sphere 7 0.81 and Mixing sphere 8 0.76 (Final
  0.95 / 1.04).

## Inputs

- The `.duf` (nodes, materials, cameras, lights, render settings).
- Library `.dsf` assets for topology and UVs, resolved against the DAZ content
  dirs (`--content-dir`, else the dump's `content_dirs`, else DAZ Studio's
  preferences), then DAZ Studio's built-in resources (the dump's
  `builtin_mdl_dirs`, the installed app).
- The **scene dump** the DAZ Studio script writes (below) for final posed
  world-space vertices (matched by label, then unique name) and strand hair.

No Blender in the path.

### Scene dump

`daz/Scripts/ORBIS/Export to Octane.dsa` writes what DAZ Studio evaluated
as one JSON object (ASCII; anything else `\uXXXX`-escaped) and runs
`orbis export <scene.duf> --dump <file> --out-dir <dir>`:

```json
{ "format": "orbis-scene-dump", "format_version": 3,
  "daz_version": "6.x.y.z", "scene": "/…/scene.duf",
  "content_dirs": ["…"], "builtin_mdl_dirs": ["…"],
  "nodes": [ { "name": "Genesis9", "label": "Noemi", "class": "DzFigure",
               "lod": 1, "transform": [m11, m12, m13, m14, m21, …, m34],
               "material_groups": ["Torso", …],
               "vertices": [x, y, z, x, y, z, …],
               "polylines": [[0, 1, 2], …], "polyline_materials": [0, …] } ],
  "instances": [ { "name": "…", "label": "HFGG Tree 07_Ins_0",
                   "target_name": "HFGG_Tree_07", "target_label": "HFGG Tree 07",
                   "transforms": [[m11, …, m34], …] } ] }
```

- One entry per node with an object and a non-empty cached geometry (bones
  skipped). `vertices`: the **base mesh** from `getCachedGeom()` — every
  node's resolution level is switched to 0 and every cache rebuilt **before**
  any is read, then restored: a geometry shell copies its host's cached
  geometry, so a host restored early hands the shell its SubD mesh (72809
  instead of 18396 vertices for a G8F shell). With every cache rebuilt, a
  graft host's cache has its grafts' vertices appended like a shell's (G8F
  16556 + graft 1840 = 18396); the export takes the host's first vertices
  and accepts either count. The rebuild runs twice, each time after
  `processEvents()` and with `forceCacheUpdate(node, true)`. Before it, a
  freshly opened scene's wearables sit in a stale fit. teens: the man's
  boots were 11 cm off his feet, and the slip-ons left 39 % of their upper
  under the skin. After it the boots are 1.8 cm off and the slip-ons 1.2 cm,
  as fitted (#51). — world space, cm, Y up, morphs and pose applied; flat
  `x y z`. The export refines it itself
  (SubD above), so the dump stays small and topology matches the `.dsf`.
- `lod`: the resolution level DAZ renders at (> 0 = refined). A geometry
  shell reports 0 and renders at its host's level — the export refines it
  with the host (an unrefined shell over a refined body showed as dark
  patches).
- `transform`: the node's `getWSTransform()`, the rows of DAZ's 3×4
  `DzMatrix3` (p' = M · p + t, t = `m14 m24 m34` in cm).
- `instances`: one entry per `DzInstanceNode` that is visible (and visible
  in render) and has a target, with one world-space transform — or, for a
  `DzInstanceGroupNode` (UltraScatter), one per group item
  (`getGroupItem(i).getWSTransform()`). Instance nodes are not dumped as
  nodes. UltraScatter's `[S] … Proxies` groups have no target and drop out.
- `polylines` / `polyline_materials`: only for geometry without facets
  (strand hair) — the rendered strands as vertex indices, and each strand's
  index into `material_groups`. Format 3 writes `polyline_counts` instead
  (each strand's length) when the strands are runs of consecutive vertices,
  as DAZ's are; format 2 (every strand's indices) is still read.
- **dForce hair renders its render hair generator's strands** ("PR
  hairs"); the viewport — and the cached geometry — shows the guides
  unless that generator's *Preview Pre Render Hairs* is on. The dump
  switches it on for every generator where it's off and back after.
  Why: Cute Braids' base dumped 3374 guide strands where Iray renders
  51473 (3.1 M points), its braids 308 where it renders 12628; hairs with
  the preview already on (dForce Corporate, Military Cut) were complete.
- Coordinates are written to 0.001 cm, vertices in chunks: built as one
  string, a scene with 4 M strand points took DAZ 20 minutes and 10 GB.
- `format_version` changes with any incompatible change; `orbis` refuses
  other versions (the script and binary are installed together).
- `orbis` keeps the dump as `<cache>/orbis/dumps/<scene stem>-<FNV-1a of the
  scene path>.json`; `orbis export <scene.duf>` without `--dump` re-runs on
  it without DAZ Studio (calibration loop). The scene must still be saved:
  materials, cameras, lights and render settings come from the `.duf`.

## Plugin

- `orbis install [--library DIR]` copies itself to `~/Library/Application
  Support/ORBIS/bin/orbis` (executable, quarantine flag cleared) and writes
  `Scripts/ORBIS/Export to Octane.dsa` + `.png` thumbnail (an `.orbx`
  document — what the script makes) and the folder thumbnail
  `Scripts/ORBIS.png` (the globe) into the content directory named
  "My Library" (else the first one in DAZ Studio's preferences). Script and
  thumbnails are compiled into the binary; the release zip holds just the
  binary.
- The binary stays out of the content library: the Content Library lists
  every folder, a `bin/` included. The install writes the binary's path and
  the default output folder into the script (`"@ORBIS_BIN@"`,
  `"@DEFAULT_OUTPUT_DIR@"`, as ASCII-escaped string literals) — the script
  has no reliable way to find the user's home (`DzDir.homePath()` fails in
  DAZ Studio 6). An install over an older one removes its `Scripts/ORBIS/bin/`.
- On its first run the script adds a custom action *File → Export to
  Octane...* (after *File → Export...*), with the thumbnail as its icon; an
  existing one is repointed when the script moved.
- One progress bar for the whole export: reading the scene (by node) takes
  the first two thirds, `orbis` the rest — with `--progress` it prints
  `progress: <percent> <what>` lines (scene, meshes, USD, materials, each
  texture, `.orbx`), which the script turns into steps. Cancel works in both.
- The closing report gives the summary and the `.orbx` path; skipped nodes
  and missing textures go in a scrolling box below (left out when none).
- Options dialog, remembered in `DzAppSettings("ORBIS")`: where the `.orbx`
  goes (beside the scene, or a folder — default `~/Documents/ORBIS`), fast
  preview, shadow-catching ground.

## Geometry

- Stage `upAxis = "Y"`, `metersPerUnit = 1`, `defaultPrim = "Scene"`; points
  converted cm → **meters** (Octane ignores `metersPerUnit`).
- One `Mesh` per visible node (points from the dump, topology from the
  `.dsf`); `Visible` = false nodes are left out.
- `GeomSubset` per material group; `primvars:st` indexed `faceVarying`.
  **Fully transparent groups** (opacity 0, no map — the NGV8 pubic hair's
  "Hiding" strands) are left out: invisible, yet under random-walk skin the
  ones 0–2 mm under it showed as white specks (probes 71–72).
- **Geometry shells** are rebuilt as DAZ does (host + grafts), rendering only
  the groups their `material_group_<group>_vis` channels switch on — the
  rest (typically opaque white) would lay a white body over the figure.
  The surfaces are picked **after refining the shell whole**, as its host
  is: cut first, the patch's border refined by the boundary rules and sank
  up to 0.3 mm under the skin in creases (0.12–0.21 mm above once fixed),
  which random-walk skin lit as white lines along the NGV8 Color Layer's
  outline (Octane4, probes 62–71).
- **Visible grafts are welded into their host mesh**: graft points follow the
  host's, border vertices are remapped onto their paired host vertex, graft
  groups are named `<graft node>_<group>` and keep the graft's materials and
  UV set. Grafts inside a shell are welded too. (Two meshes subdivide apart
  and leave a seam or a floating outline.)
- **SubD**: `subdivision_surface` at `lod > 0` is refined once by the export
  (Catmull-Clark, as DAZ's SubD 1). Every mesh is written with
  `subdivisionScheme = "none"` and smooth vertex normals — Octane has no
  per-mesh level, and "subdivide all" rounds hard props.
- **Skin-tight lift**: an unrefined mesh conforming to a refined one (hair
  cap, brows) has its vertices within 5 mm of the figure lifted along the
  nearest refined vertex's normal to ≥ 0.5 mm above it (`LIFT_CLEARANCE`).
  Why: the cap sat up to 0.8 mm under the refined skin (dark dots along the
  hairline); after the lift it is 0.28 mm above, dots gone (probe 48).
  Clothing refines with the figure and isn't touched.
- **Strand hair** from the dump's `polylines` as one `BasisCurves` per
  material group, per-vertex `widths` tapering `Line Start Width` →
  `Line End Width` (DAZ labels them mm), at **2× that** (`LINE_WIDTH_SCALE`
  = 0.002 m per unit). Why: on Calib-Eye's close-up the strands cover the
  background as Iray's at 2× (0.47 vs Iray's 0.43; 1×: 0.28, 4×: 0.63), and
  Calib-Hair's fringe and sides match Iray's fullness (0.98 / 1.13 before
  the albedo refit below, 0.65–0.93 after it); at 1× the sky shows through
  (grey-blue hair). The earlier 4× was fitted on
  guide-only strands — it made the strands visibly thick close up (#16).
- A `Material` per (mesh, group) under `/Scene/Materials/<mesh>_<group>` (a
  constant `UsdPreviewSurface`) — overridden by the Octane material inputs
  below.

### Instances

UltraScatter's forests and meadows are instances: the visible trees and
grass are instance nodes and instance groups over a source mesh (Gladiator:
12 trees, three grass patches, tens of thousands of copies). Their positions
live only in DAZ Studio — UltraScatter keeps them in its own settings blob
in the `.duf` and rebuilds the groups on load — so they come from the dump.

- An instance's transform **replaces** its target's: a copy is drawn at
  instance · target⁻¹ applied to the target's world-space mesh. Why: placed
  so, Gladiator's `HFGG Tree 07_Ins_0` stands on the terrain (y −40 cm,
  ground −17 cm); on top of the target's transform it would be 1.9 m under it.
- The source mesh is written once, as the prototype of a `PointInstancer`
  `<mesh>_copies` (positions in m, orientations `quath`, per-axis scales;
  a mirror goes into a negative x scale, shear is lost). A visible source
  is one more copy at identity; a **hidden** one still serves its instances
  (UltraScatter hides its sources), else it is left out as before.
- Copy mode "target + children" isn't followed: only the target's mesh.

### VDB volumes

A node whose material has a **Volume File** is a VDB volume. DAZ's
"Simple OpenVDB MDL Volume" passes the file through
`base::lookup_volume_coefficients` into Iray Uber's inhomogeneous volume.
The node's mesh only carries the material; Iray fills the VDB's own extent,
placed by the node. Notes: [2026-10-29](../notes/2026-10-29-vdb-volumes.md).

- **The box is not drawn.** Drawn, Gladiator's haze cube was an opaque
  white wall.
- **Units and placement.** VDB world units are DAZ's cm. The volume is
  placed by the node's world transform from the dump, in meters, as a
  transform (67) with the whole matrix. The Volume node (91) is in meters
  (`scaleUnitType` 4). The `.vdb` is packed once as `assets/vdb/NN_<file>`.
- **Grid.** "Volume Channel", else the file's first grid, read from the
  header (`vdb.rs`).
- **Coefficients, per meter in both renderers:**
  - σs = Scattering Multiplier · D
  - σa = Absorption Multiplier · D
  - D = Density Multiplier, **÷ the node's scale** when "Density Relative
    To Size" is on.
  - The multipliers are colours, decoded as Iray decodes them.

  Why: Calib-VDB at DTAS's scale 0.1 renders density 1000 relative as
  thick as 10000 without it. Absorption only (Calib-VDB-abs) matches
  Iray at factor 1: sky 0.80 / horizon 0.36 / below 0.79 of the haze-free
  render, against Iray's 0.79 / 0.35 / 0.77 (σ × 100 was opaque). Left raw,
  the (1, 0.6, 0.3) haze came out too grey and DTAS's absorption 0.09 too
  dark.

  With all of it, DTAS's haze (Calib-VDB-base) comes out 1.20 / 1.30 / 1.12
  of the haze-free render, against Iray's 1.22 / 1.31 / 1.22. Its densities
  ×0.1, unscaled and isotropic agree within 0–10 %. Ten times denser
  (10⁴) stays darker than Iray's bright fog: 1.62 / 0.88 / 1.29 against
  2.14 / 1.53 / 2.48.
- **The Volume medium (98):**
  - density = min(peak σ, 10⁴) (the pin's maximum), and the grid scales
    (`volume*Scale`, applied as the file loads) take the rest;
  - colours σ / peak, `invertAbsorption` off;
  - phase (60) g = **SSS Direction** — Iray applies it (0 instead of
    DTAS's 0.85 renders the haze 8–30 % brighter).
- **Step = 10 placed voxels.** Octane's `rayMarchStepPercent` is a
  percentage of the VDB's *own* voxel (3000 units for DTAS), not of the
  placed one, so the value is 10 × the placement's scale (0.01 for DTAS).
  Why: at the default 100 % a step crossed the whole haze; it rendered
  thin, and longer steps rendered it thinner still. It converges at 10
  voxels (1 % and 0.1 % alike); 4 voxels took over 20 minutes for 400
  samples.
- **Not exported:** emission (reported), the offsets, and Clip To Geometry
  (the volume fills its whole extent). A `.usdz` reports the volumes as
  skipped.

## Package

`.orbx` with `assets/scene.usdz` (stored, 64-byte aligned, own writer),
textures `assets/tex/NNNN_<file>` (incl. flattened layered images), VDB
volumes `assets/vdb/NN_<file>` and
`project.ocs` from the template `assets/octane/project.ocs`.

Scene graph:

```
Scene
├─ "<scene> (DAZ 3D)"   USD import graph (type 2, filename assets/scene.usdz)
│    ├─ /Scene/Materials/<name>     material inputs (20007) → Octane materials
│    ├─ /Scene/Lights/<prim>_light  emitter inputs → emissive materials
│    └─ geometry (1439)              output linker
├─ <DAZ spot>           analytic light (402), one per DAZ spotlight
├─ ground               placement (4) → plane (110) + shadow catcher (145), optional
├─ <VDB node>           placement (4) → transform (67) + volume (91) → volume medium (98)
├─ "DAZ 3D lights"      geometry group (3): Input 1 = USD output, Input 2… = spots, ground, volumes
└─ "<scene>"            render target(s) (56) → the group (or 1439 when nothing else)
```

Group pins carry `dynamicType='12'`.

## Render targets and cameras

- **One render target per DAZ camera**, `current_camera` first, the rest in
  scene order; no camera at all → one front view fitting all meshes. One
  target is named after the scene, several `<scene> – <camera label>`.
- Copies share geometry, environment and kernel; each copy's node ids are
  moved up by 1 000 000 (`offset_node_ids`), environment texture included.
- Camera: `fov` = horizontal FOV from DAZ's vertical frame fit; DAZ Euler
  order mapped to Octane's (reverse naming). Render size from the scene.
- **DOF**: aperture radius = focal length in mm / (2 · f-stop) **as cm**
  when DAZ has DOF on, else 0 — 10× a real lens. Why: Gladiator (65 mm,
  f/6, focus 3.87 m) blurs the trunk at 5.3 m over 10–13 px (10–90 % edge,
  a ~15–20 px disc) and the trees at 10–20 m by ~40 px; a real lens gives
  1.5 / 3–5 px, 10× gives 15 / 34–45 px. **Calib-DOF-f4/f8/f16** (focus
  1.5 m, spheres at 4.6 m): a disc blur fitted to Iray's render has
  diameter 104 / 52 / 26 px against 107 / 54 / 27 predicted at 10× (a real
  lens: 10.7 / 5.4 / 2.7). DAZ renders the camera by its channels (`DOF`,
  `Depth of Field`, `Aperature` — sic); `perspective` mirrors them. The lens `fstop` is written to
  match (`lens_fstop`, the template's 65 mm; 1000 without DOF) — the
  template's fixed 12 disagreed with the aperture.

## Render modes

`ExportOptions::octane_mode`; the scene detail offers Final / Preview.

| | Preview | Final (default) |
|---|---|---|
| kernel | direct lighting (24), GI diffuse, depth 2 | path tracing (25): diffuse 8, glossy 24, **scatter 128**, **GI clamp 20** |
| ray epsilon | **1 µm** (`rayepsilon` 1e-6) | same |
| skin | Universal | Standard Surface + random-walk SSS |
| imager exposure | **1.4** × 2^(13 − EV) × cm² (Variants) | **1.4** × 2^(13 − EV) × cm² |
| imager response | **Gamma 2.2** (403) | same |
| highlight compression | 0; Burn < 1, Crush > 0 or Gamma ≠ 2.2 → a tone LUT (below) | same |
| white balance / saturation | DAZ White Point × Scale / Saturation | same |
| denoiser | off | **on**, during the render (`denoiserOnce` off, `maxDenoiserInterval` 20) |

Why path tracing for final: direct lighting renders random-walk SSS grey and
isn't photorealistic; Cycles (the Blender reference) path-traces too.

**Ray epsilon 1 µm** (Calib-Eye, #20): refined, a Genesis 8 EyeMoisture lies
0.05–0.19 mm in front of the sclera. At Octane's default 0.1 mm, rays leaving
the tear film skipped the sclera wherever the gap was smaller (median 0.08 mm
in the black patches, 0.11 mm in the lit parts): the whites rendered almost
black outside the cornea, 0.3 × Iray. At 1e-5 a hairline stayed at the inner
corner; at 1e-6 the sclera is 1.00–1.02 × Iray, and Calib-NVIDIA's ground
plane shows no self-shadowing.

**Imager calibration** (Octane2 vs Iray, 2026-10-21): a sweep of exposure
0.7–2 × highlight compression × ACES, compared on the HDRI background:
exposure 1.4, no highlight compression, no ACES fits best. As exported:
181 167 141 vs Iray's 188 161 132 (sRGB, RMSE 7.7 / 255; the earlier fixed
imager fitted to Blender gave 50.4). Only HDRI-lit scenes are calibrated.
**Response Gamma 2.2** (`responseCurveId.GAMMA2_2` = 403; LINEAR 400, sRGB
401, GAMMA1_8 402): DAZ's tone mapping writes a pure gamma 2.2, Octane's
default sRGB curve rendered equal light darker in the shadows (Calib-Point
floor 17 vs Iray's 24 / 255); with 403 every region matches Iray's pixels
within 1 / 255 (probe 76). The linear match is unchanged, so the exposure
stays.
**Exposure** comes from the Tonemapper's Shutter Speed S, Aperture N and
Film ISO as the ISO-100 EV log2(N² · S) − log2(ISO / 100) (Iray exposes ∝
ISO / (N² S)); its "Exposure Value" channel only when they're missing. Why:
DAZ derives the EV from them for show — a scene edited outside DAZ (the
calibration scripts) can carry a stale EV, and at ISO ≠ 100 DAZ shows
log2(N² S) + log2(ISO / 100), the opposite sign. The ISO term is the
photographic tone mapper's: Calib-Tone-ISO200 renders in Octane at the
same ratio to Iray as Calib-Tone-Default (1.02 – 1.13 per sphere).

**Tone LUT** (#14): Octane's imager has no curve for Iray's burn, crush or
another gamma — its highlight compression is far weaker (probe 91: 0.70×
at 0.84 where Iray's Burn 0 gives 0.54×). A scene off the calibrated
defaults gets Iray's curve as a custom LUT (`assets/tone.cube`, 1D, 4096
entries; LUT node 103 on the imager's `lut`, `order` 0 = response, gamma,
LUT): c · (1 + c · b) / (1 + c), then per channel x² / (x + 0.3 · crush),
then ^(1 / gamma), sampled on the Gamma 2.2 response of c / 16 — the
exposure is divided by 16 so highlights to 16× white survive the response's
clip. Why: **Calib-Tone-*** (Calib-Materials with one tone setting changed,
rendered at a sample cap so every variant has the same noise) mapped from
its Burn 1 / Crush 0 / Gamma 2.2 render: Burn 0.5 and 0, Gamma 1.8 and 2.6
within 0.5 / 255 of Iray's pixels — the photographic tone mapper's own
formula; the crush is a fit (`CRUSH_KNEE`): Crush 0.1 / 0.3 1.4 / 2.4 / 255,
Gladiator's mix (Burn 0, Crush 0.07, Gamma 1.857) 1.2 (Reinhard and gamma
alone 5.9; `^(1 + crush)` 2.3 / 4.2 / 1.7). Octane's LUT render matches the
curve applied to its linear render within +0.5 / 255 (+4 in the deepest
shadows); end to end, every Calib-Tone-* export renders in Octane at
Calib-Tone-Default's ratio to Iray (floor 1.01 – 1.05; the spheres differ
by material, not by tone setting).

## Materials

Every material input (20007) is filled — Octane doesn't convert
`UsdPreviewSurface`.

### Universal (130) — default

- **albedo** = diffuse map (sRGB) × the diffuse colour, or the colour; ×
  (1 − refraction). A grey colour is the image's power; a coloured one goes
  through a Multiply texture (39, `texture1` map × `texture2` RGB) — as a
  scalar a brown-tinted grey strand map rendered grey hair (probe 62). No diffuse map → a translucency colour map (hair, caps).
- **Weighted base mixing** (`Base Mixing` 2): a real diffuse map is scaled
  to the blend's brightness (`weighted_mix`; 2.8 for a mesh hair) and
  `Glossy Weight` counts as gloss. Not applied to a translucency map standing
  in as albedo (greyed a cap's crown, probe 60).
- roughness (dual-lobe skin: Specular Lobe 1 Roughness), metallic, cutout
  opacity = greyscale map × value or constant; normal / bump maps. Without
  `Cutout Opacity` (DAZ Default shader) opacity is `transparency` ("Opacity
  Strength"): the NGV8 pubic hair's "Hiding" cards at 0 rendered as a white
  layer over the skin (probes 62–63).
- **No glossy layer, dual lobe, refraction or mirror → `specular` 0** (Iray
  is matte there; Universal's white default glittered velvet, probe 53).
  A `Glossy Color` map tints `specular` (× glossy weight); else a `Glossy
  Layered Weight` map × the weight, else glossy colour × weight when below 1
  — Universal's full white default made the GSOV4 leather (gloss 0.33 through
  a seam mask) 3.4× Iray's sheen; now 1.13–1.16 (Fetroom).
- **Translucency** off the skin path (props, cloth, cards) → Universal's
  diffuse transmission: `albedo` × (1 − w), `transmission` = Translucency
  Color (map) × w, `transmissionType` 1 (Diffuse), 3 (Thin Wall (Diffuse))
  when thin-walled. Not on metal, refraction or Base Mixing 2 (its weights
  are shared out by the weighted mix). Why: left out, Calib-SSS's spheres at
  w 0.5 / 1 rendered 1.6 – 1.7 / 3.2 × Iray and Calib-PBRSkin's Gladio cloth
  1.72; with it 0.91 – 0.96 and 1.23 (its neighbours 1.14 – 1.26), the two
  types alike on closed spheres. On P8 (metallic 0.75) the transmission
  swallowed the metal (0.23 × Iray). Thin-walled or not, figure or prop
  alike — Iray knows only the material: Calib-Thin's thin-walled cards (w
  0 / 0.3 / 0.5 / 0.65, Translucency Color white / the diffuse / dark, lit
  from the front with the dark behind them or from behind) 0.89 – 1.06 ×
  Iray; scaling the albedo (× 0.44 – 0.7) or the transmission (× 1.5 – 2)
  only did worse. **Base Color Effect 1** (Scatter & Transmit) changes
  it: Iray reflects 0.4 of that diffuse and passes twice the translucency
  (`SCATTER_TRANSMIT_DIFFUSE` / `_TRANSMISSION`) — the same cards 0.98 –
  1.02 ×, at the default shares 1.8 – 2.5 ×. The share is a scalar (each
  channel 0.40 – 0.41), not the base colour again (× D on the
  transmission: 0.28 – 0.95). The Genesis EyeSocket is such a material: its
  caruncle rendered 4.6 × Iray, now 1.00 (Calib-Eye, #20). Found by
  switching the EyeSocket's other channels off one at a time in Iray on the
  cards (SSS Amount / Direction / Tint, Transmitted Color, its gloss, Top
  Coat, Invert Transmission Normal: no change). An **open sheet** that isn't
  thin-walled (a material whose faces have an edge no other face of the mesh
  shares: cards, leaves, a cloak) passes Tc × min(2w, 1): Iray lit
  Calib-Leaf's backlit card 1.9 × its thin-walled twin, Octane rendered them
  alike (0.48 / 0.90 × Iray at w; 0.81 at min(2w, 1)); doubled on the closed
  spheres it overshot (1.25 – 1.38). Opaque card 0.97.
- **Glossy Reflectivity** r (0.5 = Universal's full specular) scales the
  glossy weight by min(2r, 1) (PBRSkin's Dual Lobe Specular Reflectivity
  likewise); above 0.5 the excess 2r − 1 (× glossy weight, × glossy colour)
  becomes a clear `coating` at the gloss's roughness, unless a Top Coat
  takes that layer. Universal caps `specular` at 1 (2 renders as 1). Why:
  Calib-Glossy, r 0 / 0.25 at the full default 2.42 / 1.54 × Iray, scaled
  1.21 / 1.14; r 1 0.74 at specular 1 or 2, 1.08 with the coating — the
  neighbours sit at 1.11 – 1.16.
- **Roughness** by Base Mixing: 0 → `Glossy Roughness`, 1 and 2 →
  1 − `Glossiness` (both channels are always saved; Calib-Materials:
  Glossiness 0.8 beside Glossy Roughness 0.5 rendered Iray's sharp
  highlight), then **squared** when `Roughness Squared` / `Glossiness
  Squared` (default on) — Iray squares the slider, Octane doesn't:
  highlights went from 31 / 17 / 6 px to 48 / 24 / 22 against Iray's
  56 / 26 / 20 (#10).
- **Top Coat** → Universal `coating` = Top Coat Color × weight,
  `coatingRoughness` (Top Coat Roughness, or 1 − Top Coat Glossiness in
  modes 1 / 2, squared likewise), `coatingIor`: the top-coated sphere's
  highlight 0 → 19 px (Iray 17). `coatingIor` by Top Coat Layering Mode:
  0 Reflectivity (the cornea; 2128 materials in My Library) reflects F0 =
  0.08 × Reflectivity head-on (0.5 → 0.04, IOR 1.5) — Calib-TopCoat's
  Reflectivity 1 sphere 0.75 → 1.09 × Iray (0.5: 1.11); the other modes
  Top Coat IOR, which Iray doesn't follow (#25: Weighted renders a mirror
  blended by the weight, Octane 0.10 – 0.15; Fresnel 1.5 / 2 and both
  Custom Curves hardly show a coat in Iray, Octane 2.2 – 4.2).
- **Thin-walled refraction with its own inputs** (Share Glossy Inputs off:
  the cornea, the tear film) takes the glossy layer's roughness, not the
  refraction's — a thin wall's straight-through transmission doesn't blur
  — and its Top Coat at least that roughness. Why: at the refraction's 0
  and the coat's 0.0016 the cornea mirrored the room over the iris (a dark
  band across it, a hard highlight); at the gloss's 0.07 the iris shows
  Iray's veil and soft highlight — Calib-Eye pupil 0.43 → 0.89, iris
  0.57 – 0.74 → 0.79 – 0.83 × Iray, sclera 0.97 / 1.13 (#20).
- **Weighted mixing** (Base Mixing 2) without translucency → a **Mix
  material** (19): `material1` a metal (metallic 1) in the Glossy Color (or
  its map) at the surface's roughness, `material2` the surface without its
  gloss (`specular` 0, the diffuse map unbrightened), `amount` = Glossy
  Weight / (Diffuse Weight + Glossy Weight) (`weighted_gloss`); the Top Coat
  on both. Iray blends a full-strength glossy reflection by that share: as
  a metallic share of one Universal the reflection took the diffuse colour
  — Calib-Mixing (Fetroom's surfaces): weighted grey 0.63 → 1.05, ceiling
  0.85 → 1.06, door jamb (black, 0.8 coat) 1.09 once the coat covered the
  metal too (0.55 without). With translucency (mesh hair) the diffuse map
  is brightened by `weighted_mix` as before. Base Mixing 1 with the wall
  map as Diffuse and Glossy Color renders like a flat one (1.12 – 1.14).
- **Metallic Weight under 0.05** (no map) is left out: the GSOV4 jacket's
  0.0107 turned Octane's gloss off — black velvet (0.02× Iray) beside the
  same leather's trousers.
- Base Mixing 1 ignores Metallic Weight; a black base under a strong white
  gloss becomes a gloss-coloured metal (mirrors).
- **Refraction** (glass, water) → `transmission` = Refraction Weight ×
  the refraction's colour, IOR on `ior` (Dielectric IOR — Universal has no
  `index4`, probe 82; `ior` 2.0 brightened Calib-Materials' gloss 1.08 →
  1.15), transparent shadows. Iray's refraction takes the glossy layer's
  colour and roughness while **Share Glossy Inputs** is on (the default),
  its own Refraction Color / Roughness (Glossiness in Base Mixing 1 / 2)
  when off: shared, a green Refraction Color and roughness 0.3 changed
  nothing in Iray. A mostly refractive surface (weight ≥ 0.5) with its own
  roughness gets it on Universal's one `roughness`, squared as usual and
  × 0.3: Iray's 0.3 matched Octane's 0.02 – 0.03, Octane frosted at 0.09.
  **Thin Walled** → `transmissionType` 2 (Thin wall), no volume. Iray shows
  a thin-walled glass sphere without any reflection — invisible; Octane's
  thin wall keeps its Fresnel reflection, kept as the more plausible.
  **Absorption**: `Transmitted Color` T (linear) after `Transmitted
  Measurement Distance` d (cm) → an absorption medium (58) on `medium`,
  σ = −ln T / d per meter (`scale` = max σ, `absorption` = σ / scale,
  `invertAbsorption` 0); black T (Iray's default) and white mean none —
  the clear spheres stayed clear. Calib-Glass (Octane / Iray): clear,
  water, green, thin, rough, half, glossy 0.95 – 1.09; the absorbing blue
  sphere's core 0.59 in blue (Octane's spectral upsampling of a deep
  colour absorbs more), its surface keeps reflections Iray drops.
- **Custom (DAZ brick) shaders** name some inputs their own way; those
  stand in for the Uber channel and win when present (`CHANNEL_ALIASES`):
  `Diffuse Texture Map` → `diffuse`, `Map Horizontal / Vertical Tiles` →
  the tiles, `Bump Map` → bump, `Roughness Map` → roughness (a colour-typed
  strength read as its mean). HFGG's terrain keeps only a small untiled
  preview in `diffuse`: the Gladiator's sandy path rendered as a blurred
  stain where Iray showed sand and pebbles (its map tiled 20 × 20).
- Iray UV tiling → each texture's transform (scale 1/N); textures **wrap**
  (UDIM tiles).
- **Colour space.** Colour maps are sRGB; bump and normal maps are
  linear. Every image node gets `initialColorSpaceSelected` = 1, and the
  HDRI stays linear. Why: without the attribute, data maps were decoded as
  sRGB and sunglasses came out nearly clear (probes 55–57). With it the
  lenses tint like Iray's, brightness unchanged (Octane2 moved 0.28 / 255).
- **Float channels' maps are read at gamma 2.2.** This covers roughness,
  opacity, metallic, translucency, glossy weight and reflectivity, and
  displacement. DAZ reads a map with no gamma of its own (`map_gamma` 0,
  "automatic", which is every map in the user's scenes) at 2.2, though
  the Uber channels declare `default_image_gamma` 1. An image's own
  `map_gamma` (from `image_library`) wins.

  Why (#49): on Calib-DataGamma a grey 128 map renders as a value of 0.22
  in Iray. The same grey at `map_gamma` 1 renders as the plain value 0.5.
  The sphere-centre ratio of the map to the plain 0.5 is:
  - opacity: Iray 0.78, export now 0.78 (0.83 at gamma 1);
  - metallic: Iray 1.36, now 1.25 (1.14).

  The eight Calib-Skin figures (front light) moved by at most 2 % (ears
  5 %). Bump and normal maps are untested and stay linear.
- **Colour channels** (Base / Glossy / Translucency / Transmitted / Emission
  Color, light colours) are gamma-encoded as DAZ stores them and decoded to
  linear by a **pure gamma 2.2** (`colour_to_linear`), as Iray does — not
  the sRGB curve. Passed on raw, surfaces rendered 1.5× too bright
  (Calib-HDRI: floor 1.53 → 1.03, cubes 1.77 → 1.00 of Iray; the HDRI
  background 1.04 either way). Decoded as sRGB, dark channels came out too
  bright — the curve's linear toe: Calib-Glossy's red (0.45, 0.05, 0.04)
  rendered its green 2.77 × Iray, 0.99 × at gamma 2.2 (its matte sphere
  1.13 → 1.08 in luminance; mid greys move ≤ 2 %).
- **OOT Hairblending 2** (Outoftouch's hair, a brick graph on Iray Uber's
  weighted base): the brick without a "Base Mixing" channel reads as Base
  Mixing 2 (`DzMdlIrayUberWeighted`); its diffuse, translucency and glossy
  colours each blend in their `… Overlay Color` where `Overlay Alpha Mask`
  is light (Blend Colors, normal mode) — a Mix texture (`overlaid_pin`).
  Calib-OOT (Rose Hair, local: DAZ Store content): the crown's hue went
  from orange (r/g 1.25, b/g 0.59 × Iray: the base layer alone) to
  1.03 / 0.96; its brightness 2.24 → 0.62 × Iray — Iray's glints (glossy
  and Top Coat anisotropy 1, the Top Coat's colour maps) are not exported
  yet (#23, #25).
- **Howie Farkes's terrain shader** (Grassy Grove's HFGG Terrain, a brick
  graph): the albedo is its diffuse map × its AO map (both tiled with the
  material's Map Tiles) darkened toward `Damp Color` by `Damp Map` (Blend
  Colors, multiply) — the damp map's tiler has no frequency of its own, so
  it spans the terrain once (`damp_ao_albedo`). Its large wet patches are
  the ground's texture in Iray: left out, Gladiator's ground showed only the
  tiled detail (before the tiling was read, only the untiled diffuse map).
- **NVIDIA MDL examples** (DAZ's "NVIDIA MDL Examples" brick shaders; the
  MDL sources ship in DAZ's `shaders/iray/nvidia/`, their maps in
  `shaders/iray/resources/`). Parameters come from the shader's channels,
  else the MDL defaults; a map channel set empty is no texture.
  - Metals (`metal_examples`: Metal Brushed, Cast Iron): a Universal metal,
    albedo = Metal Color (`diffuse`) × (offset + scale · reflection map),
    roughness √(u · v) per texel, anisotropy from u / v, bump, the shader's
    tiles (`mdl_metal`). Calib-NVIDIA: Metal Brushed 1.12, Cast Iron 1.19
    × Iray, hue within 0.01 (g/r) – 0.04 (b/r).
  - Dielectrics (concrete, concrete polished, asphalt, ceramic tiles small,
    granite black, porcelain, resin polyurethane matte, architectural):
    `df::fresnel_layer(ior, weight, glossy, diffuse)` → Universal albedo,
    specular = the weight (a map where the MDL weights or tints the gloss
    with one: concrete's bump map tints it, asphalt's and the tiles' bump
    map weights it, polished concrete's colour map does — its diffuse is
    that map × the weight too), `ior` = the layer's IOR, the MDL roughness
    as is (not squared), bump × its amount (`mdl_dielectric`). Granite's
    varnish is a mirror (roughness 0). Resin's base is a tinted glossy
    reflect + transmit: specular and transmission its colour, the clear
    gloss as the coating; transmission × 0.8 (`RESIN_TRANSMISSION`) — at
    the full colour 1.36 × Iray and redder (g/r 0.30 v 0.37), × 0.85
    matched the hue (1.23), × 0.6 showed too much background (g/r 0.50).
    Architectural (mental ray's mia_material): diffuse × weight,
    reflectivity × the reflection colour's maximum tinted by it, without
    "Use Fresnel Term" Schlick's curve from the 0° reflectivity as the IOR
    with that F0 (0.2 → 2.62), metal, transparency and cutout opacity;
    glossiness → roughness (1 − g)² is a guess (Iray's
    `architectural_gloss_to_rough` is compiled in), only g = 1 calibrated.
    Calib-NVIDIA (Octane / Iray, sphere luminance): concrete 1.03,
    polished 0.98, porcelain 1.11, granite 1.12, architectural 1.07 — the
    level of the matte spheres elsewhere; before, all were plain diffuse
    without gloss, bump or tiling.
- **Layered images** are flattened into one PNG per stack.

### Skin — Standard Surface (277), Final only

A material with Iray `Translucency Weight` > 0 that is opaque (no cutout),
not `Thin Walled`, non-refractive, non-emissive, not metallic (`Metallic
Weight` under 0.05 and no map) and has a diffuse map. Why not metallic: the
Gladio outfit is PBRSkin with translucency 0.5 and a metallic map on every
surface — as skin its metal was lost and the open cloak rendered grey
instead of red (Gladiator). And on a figure (the dump's `class` names a figure): props'
translucency is grass blades and leaves, open cards — Gladiator's grass
rendered grey. And not the eye (material named …Irises / Pupils / Sclera,
`is_eye`): Genesis eyes are translucent too, and the skin's tint (half,
reddened) turned blue irises dark green (Calib-Hair, #20).

- `baseColor` and `subsurfaceColor` = the diffuse map (Iray's translucency
  map is pale and bleached skin and make-up) × **½ · ((1 − w) + w · Tc)**
  (`skin_tint`; w = translucency weight, Tc = `Translucency Color` × its
  map's mean over painted texels), `subsurface` = w,
  `subsurfaceAnisotropy` = `SSS Direction`. Why: Standard Surface shows the
  map as is — Calib-HDRI's Genesis 8 Female skin 3.2× Iray (effective
  albedo 0.48 vs 0.154); Iray's per channel matched ½ · ((1 − w) · D +
  w · D · Tc) within 4 %. Tinted: G8F 1.14–1.18 × Iray, Fleur (Octane4)
  0.9–1.0. × **`SKIN_HUE` (1, 0.80, 0.62)**: Octane's random walk came out
  bluer at the same tint on five figures (G 1.25 ×, B 1.6 × Iray relative
  to red); with it G8F 1.02 / 1.05 / 1.30, Fetroom's figures within ~0.2
  in hue (#8);
- `radius` = per-channel diffusion length (see [iray.md](../daz/iray.md#translucency-and-subsurface-skin))
  normalised to its maximum and **square-rooted**; `scale` = that maximum in
  meters, clamped 0.5 mm – 2 cm. Why the root: Noemi's 1 / 0.18 / 0.15
  threw coloured fireflies, and Iray's look is softer.

**OpenPBR** (the Enhanced look, see Look; #19): OpenPBR Surface (505; its
`radius` is the distance and `scale` the per-channel colour — the other way
round from Standard Surface). On Iray's **chromatic SSS** (`SSS Mode` 1,
the modern skins) its own colour rule, fitted on Calib-Skin (eight Genesis
8 skins from pale to dark on one body, Iray probes; see
docs/notes/2026-10-28-skin-tones.md). A mono-SSS skin (`SSS Mode` 0, like
the Genesis 8 base skin) keeps the Standard Surface tint lifted by
1 / (1 − 0.46 · w) (`OPENPBR_SKIN_DARKENING`; G8F 1.00 / 1.17 / 1.70 of
Iray): none was in the fit, and under the rule Calib-Eye came out 1.76 ×
Iray (1.17 so).

- **Colour, baked per texel** into one map for `baseColor` and
  `subsurfaceColor` (`Textures::derived_skin`, `skin_albedo`):
  `(1 − w)^1.3 · D + w · 0.55 · Tc · Tc_low` per channel (D the diffuse map
  × its colour, Tc the translucency map × its colour, `_low` low-passed).
  Why: Iray's probes put the diffuse layer at (1 − w)^1.3 (w 0.9 and 0.5
  within 0.94 – 1.03) and its subsurface part at ≈ 0.55 · Tc², not tied to
  D (a white diffuse map left it as it was); Tc · Tc_low keeps the map's
  texture at its contrast (Tc² doubled it).
- × **`SKIN_HUE_OPENPBR` (1.16, 0.83, 0.68)** to the power of the smooth
  colour's saturation ÷ 0.75: the eight figures came out 0.86 / 1.20 /
  1.48 × Iray per channel without it; at full strength the pale grey Zurie
  turned orange.
- × (smooth colour / 0.3)^**0.2** (`SKIN_DARK_GAIN`): the random walk
  brightens dark colours against mid ones (Monique 1.15 × her colour as a
  plain diffuse, Victoria 0.91). From the smooth colour so pores and
  wrinkles keep their contrast — per texel they turned grainy (#47 keeps
  that as an Enhanced idea).
- `subsurface` = the diffuse map's **detail mask**: D / D_low through a
  smoothstep 0.35 → 0.70 (1 on skin, 0 on brows and liner). Why: the random
  walk bled the skin's light under dark brows into blue-grey smudges;
  Universal at w 0 kept them dark. Elsewhere all of the colour goes through
  the random walk: at `subsurface` 0.5 Octane rendered 0.67 – 0.73 of the
  colour whatever it was (Calib-SSS's spheres).
- **Gloss** as Universal's (`skin_gloss`): Iray's squared roughness, the
  dual lobe's weight × min(2r, 1) of its reflectivity — with the
  reflectivity's **map** (dark under brows and make-up; a uniform gloss
  shone blue-grey there on dark skins), the Top Coat as the coating with
  its weight map and **`coatingDarkening` 0** (OpenPBR's coat took the base
  to 0.58 – 0.80 of Universal's; Gabriela's skin 0.66 → 1.01).
- Result, front light, face and chest medians: Angel 1.10, Victoria 1.00,
  Kayo 0.99, Gabriela 0.98, Nishi 0.88, Twosret 0.92, Monique 1.17,
  Zurie 0.80 × Iray (export before: 0.68 – 1.85); hue within ±15 % but
  Monique's blue (b/g 1.30). Dark skins show more pore texture than Iray's.
- Not a cause: Iray doesn't dim the gloss of translucent skin (highlight
  contrast on the dark foreheads is higher in Iray); the gloss alone ~0
  made faces flat.

### dForce hair — Hair material (147)

albedo = 0.046 × the hair colour + 0.110 × the transmission colour
(`HAIR_COLOUR_SHARE`, `HAIR_TRANSMISSION_SHARE`; each the mean of its root
and tip colour, each its map's linear mean × tint); roughness both ways from
`base_roughness`; specular = `Glossy Layer Weight`. Why: Iray's Blended
Dual Lobe hair follows its transmission colour more than its hair colour —
Calib-Hair (blond) and two copies with dark brown hair or transmission
colours (render strands, 2× widths), fitted on the front-lit crown:
0.97 / 1.03 / 1.07 × Iray in luminance; Octane's Hair is linear in its
albedo (× 0.6 → × 0.60). Its `specular` barely shows (0.25 → 0.03: crown
−3 %). The hue is less saturated than Iray's on the blonds (crown r/g 1.16
vs 1.47) and more on the dark-transmission copy (1.18 vs 1.08) — no single
saturation curve fits both; open in #42. Backlit strands glow through
(Octane's TT lobe) where Iray's stay dark: kept, it is Octane's more
plausible take (the Enhanced look).

**OmniHair** ("OmniHair Root-Tip Blend", a brick graph on
`OmniSurface::OmniHairBase`: Chiang's hair BSDF, as Octane's Hair is) →
Hair in **melanin mode** (`hairMode` 1): `hairMelanin` / `hairPheomelanin`
= OmniHair's eumelanin / pheomelanin concentrations, m² · 2.4 · (1 − r) and
· r (m = Melanin or its preset, r = Melanin Redness). A base colour other
than white at Diffuse Strength 1 adds its absorption (Chiang's colour
mapping of colour × strength × 2.4, clamped to 1) blended toward the
melanin's by √m — a white one at strength ≥ 0.42 absorbs nothing, leaving
the melanin × √m; a coloured one is fitted as the two pigments (least
squares, ≥ 0: `melanin_fit`). Roughness: OmniHair's mapped values
(`compute_v`, and `compute_s` for the azimuthal when Anisotropic
Roughness is on) — the unmapped 0.2 rendered the same crown. IOR as is;
Highlight (its diffuse lobe) as `diffuseAmount` / `diffuse`; root and tip
averaged. Why melanin mode: the absorption turned into Octane's albedo by
Chiang's colour mapping rendered the crown 0.65 (blonde) / 0.78 (brown) ×
Iray; the concentrations as is 0.99 / 1.00. Calib-OmniHair (Calib-Hair's
strands, local), crown: blonde 0.99, brown 1.00, black 0.88, red 0.85,
dyed 0.95, Wendy's settings 0.89, highlight 0.73. Octane's pheomelanin
reads yellower than Iray's: brown and red come out golden where Iray's are
reddish (red: g/r 0.85 v 0.72) — Chiang's coefficients (MDL's ÷ 1.21 /
÷ 1.83) and pheomelanin as a redness fraction were worse; open (#21).

### Emission

Iray emission → Universal `emission`: blackbody (53: power, temperature,
normalize) or, at temperature 0, texture emission (54) — **both × the
Emission Color** on `efficiency or texture`, as Iray multiplies (left out
on the blackbody, Fetroom's dark purple trims and door glowed white: door
95× → 3.6× Iray).
Power (W) = L × unit factor / 2 / **15 000** (`DAZ_LUMENS_PER_WATT`) ×
**1 / 3.4** (`EMISSION_POWER_FACTOR`) — total for lm / W, × the material's
surface area (m²) for luminance units. Calib-Emissive (one-sided 50 × 50 cm,
5000 cd/m²): 3.4× Iray without the factor, 0.98–1.02 with it (#6).

### Displacement

Iray Uber and PBRSkin displacement goes to the material's `displacement`
pin as an Octane **vertex displacement** (97), as Iray displaces vertices.
Only a map with a non-zero strength and range counts; Iray needs a map.
In a weighted mix the mix material (19) carries it. Notes:
[2026-10-29](../notes/2026-10-29-displacement.md).

- **Amount.** Iray moves a vertex along its normal by Strength ×
  lerp(Minimum, Maximum, map) cm. Octane moves it by `amount` × (map −
  `black_level`), so:
  - `amount` = Strength × (Maximum − Minimum) × 0.01 m;
  - `black_level` = −Minimum / (Maximum − Minimum).

  Why: on Calib-Disp, Octane's sphere silhouettes lie on Iray's to the
  pixel. White over 0..5 cm grew by 5 cm, at strength 0.5 by 2.5 cm and at
  strength 2 by 10 cm. Black over −5..5 cm shrank by 5 cm, and white over
  −5..5 cm grew by 5 cm.
- **Map gamma 2.2.** DAZ reads a map without a gamma of its own
  (`map_gamma` 0, as every displacement map in the user's scenes) at 2.2,
  though the channel declares `default_image_gamma` 1. The image's own
  `map_gamma` wins.

  Why: on Calib-Disp-Map, grey 128 moved the sphere by 22 % of its range
  and grey 64 by 6 %, for an L PNG, an RGB PNG and a JPEG alike. With
  `map_gamma` 1 the same grey 128 moved it by 50 %. The other float
  channels share this (Textures, colour space).
- **Subdivision.** `subdLevel` = SubD Displacement Level, on figures only
  and capped at 2. Iray subdivides a SubD figure further for displacement
  and ignores the level on a plain mesh; on a prop Octane would round its
  hard edges.

  Why: on a Genesis 8 Female in Comfy Shorts (teens, a close-up), the
  six displaced surfaces have 3–5 mm amounts at subdLevel 2. They kept
  their UV seams, hems and the belt closed, and their silhouettes matched
  the render without displacement.

## Lights

Watts at 15 000 lm/W × a factor per emitter shape, fitted to Iray on the
primitives scene (#6; Octane / Iray linear luminance on every surface):

| light | factor | before → after |
|---|---|---|
| point, sphere (`SphereLight`) | **0.58** (`SPHERE_POWER_FACTOR`) | 5.2 (×3) → 0.97–1.03 |
| rect, disc | **0.145** (`RECT_POWER_FACTOR`) | 6.9 → 0.97–1.03 |
| distant | 1 | 0.96–1.01 |

A sphere radiates from four times a one-sided rect's area (0.58 / 0.145 ≈ 4).
Discs take the rect's factor. **Sphere lights** take a quarter of the
point's (`IRAY_SPHERE_SHARE`): Iray lights the floor with 0.250 of a point
light of the same flux, at 20 and 50 cm alike (Calib-L-Sphere-d20/-d50) —
its flux as if from a disc of its radius. **Non-photometric lights**: 1000 π
lm × Intensity, Intensity Scale ignored as Iray does (Calib-L-Point-legacy:
Intensity 1 / 2 at scale 1000, 1 at scale 3000 → 3146 / 6281 / 3146 lm
equivalent; the former 15 000 lm × Intensity × Scale was 4800× too bright
at the default scale 1000).

**Iray-side checks** (the Calib-L-* scenes, exposure undone from shutter /
f-number / ISO, a floor patch): flux is linear (point 5k → 20k: 3.999×,
distant 1.5k → 5k: 3.337×); a rect or disc of the same flux lights the floor
alike at 25 / 50 / 100 cm (1.009 / 1 / 0.965, disc 1.003); a two-sided rect
sends its full flux down (1.000), as does a two-sided emissive plane — so a
two-sided rect / disc light (`Two Sided`) and emitter (`Two Sided Light`)
get Octane's `doubleSided` emission at **twice** the power
(`TWO_SIDED_POWER`), on the assumption that Octane spreads an emitter's power
over both faces — to confirm with Calib-L-Rect-50-2sided and
-Emit-cd-2sided; a distant light lights the floor by the sine of its
elevation (30° / 60°: 0.717 / 1.232 of 45°, sine 0.707 / 1.225); the
emission units keep their ratios (20 000 lm and 50 W against 5000 cd/m² on
50 × 50 cm: 16.000 and 0.600 in Iray, the export's formula gives 16 and
0.6).

- **Point / area** → UsdLux under `/Scene/Lights` (point → `SphereLight`
  r = 1 cm + `treatAsPoint`; rect, disc, sphere by `Light Geometry`; size in
  cm; `colorTemperature`; intensity = watts, `normalize`). Their emitter
  inputs get a black diffuse with blackbody emission (`light_inputs`) —
  Octane's import doesn't make them glow. **Rect and disc lights are turned
  half round their X axis**: Octane's import emits them along +Z, not
  UsdLux's and DAZ's −Z (Calib-Area lit the floor behind the light; turned,
  every surface 6.8–7.3 × Iray — power still to fit, #6).
- **Spots** → analytic disk lights (402) — UsdLux shaping is lost: disk of
  the emitter size (2 cm for a point source), `normalize`, blackbody.
  **Beam**: Iray lights cos(θ · 90°/half)^`Beam Exponent` (half = `Spread
  Angle` / 2; `Beam Distribution` changes nothing); the disk at
  `analyticLightSpreadCutoffHardness` 0 lights cos θ · cos²(θ · 90°/half′)
  (probe 74, three spreads). `octane_cone_half` fits half′ to the
  flux-normalised Iray profile (e = 4: 0.78 × half; e = 1: 1.26 ×).
  **Power** = watts × 0.039 (`SPOT_POWER_FACTOR`): DAZ's flux is the beam's
  whatever the spread and exponent (Iray: 0.255–0.257 over five variants),
  and the normalised disk keeps its flux; 0.0443 at equal flux, trimmed to
  the beam's middle. Calib-Spot: lit cubes / sphere 0.54–0.96 → ~1.0, floor
  2–8 → 0.3–1.0 at the edge where Octane's narrower cone fades first (#7).
  Transform: DAZ rotation order mapped, scale `1 1 -1` (the disk shines +Z,
  a DAZ spot −Z).


**Camera headlamp** (#22, `lights::headlamp`): the point light DAZ puts at
the render camera, moved by X / Y / Z Headlamp Offset (camera axes), at
1.7 Mlm × Headlamp Intensity (`HEADLAMP_LUMENS`: Iray lit Calib-Glossy
without its dome 1.65 – 1.80 × as bright as a 1 Mlm point light there;
with the dome it adds 10.8 %). When:

| HeadlampMode | Auto Headlamp (render setting) | lit |
|---|---|---|
| On | any | always (Calib-L-Point-5k with it 2.67 × as bright) |
| Auto | 1 or 2 | only in a scene without lights |
| Auto | 0 | never |
| Off | any | never |

Auto Headlamp 2 added nothing to Calib-L-Point-5k's point light either
(Calib-Headlamp-light-a2 = its render without one). Octane / Iray on
Calib-Headlamp-*: 1.04 – 1.05 with the dome, 1.05 without, 1.00 in the
scene with a light.

## Environment

- **Environment Mode** "Dome Only" (1) and "Sun-Sky Only" (2): Iray
  ignores the scene's lights — a 2 000 000 lm point light left the floor
  unchanged (Calib-L-Mode-*, 1.000), so the export leaves them out
  (`lights_iray_uses`); Octane renders both Dome Only scenes, the light
  on or off, at 1.03 – 1.04 × Iray (floor, background). Sun-Sky itself → below.
- **Sun-Sky** (Environment Mode 2) → Octane's **Daylight environment** (14)
  on the render target, Hosek-Wilkie (`model` 3) with its ground from the
  horizon down (`groundStartAngle` 0: the default 90 shows sky below the
  horizon; Preetham has no ground — 4.9 × Iray's grey there, though its
  sun and sky colours were closest). `sundir` is the sun's position for
  `SS Latitude` / `Longitude` / `Day` (Julian day number) / `Time` (s) /
  `UTC Offset` (`sun_direction`, the Astronomical Almanac's low-precision
  formula) in scene axes — north −Z, east +X: DAZ's default 18:00 in Salt
  Lake City casts the same shadows in both, and so does noon.
  `turbidity` = 2 + 2 × `SS Haze`, at most 10 (Hosek-Wilkie renders 14
  like 10); `power` = 2.7 × `SS Multiplier` / 0.10132 × exp(−0.128 · haze)
  (`haze_dimming`: Iray's haze dims, Octane's turbidity brightens — haze 4
  at turbidity 10 was 1.67 × Iray, now floor 1.00, shadow 1.13, sky 0.91,
  ground 0.61; one haze measured) and `sunIntensity` = 0.55 ×
  `SS Sun Disk Intensity` (`SUN_SKY_POWER`, `SUN_SKY_SUN_SHARE`): at 1 / 1
  the sky-lit shadow came out 0.37 × Iray, the sunlit floor 0.53 × — Octane's
  sun 2.2 × its sky where Iray's is 1.2 ×; at 2.7 / 0.55 floor 1.02, shadow
  0.96, ground 0.92 (Calib-L-Mode-SunSky-off; noon: floor 1.01, shadow
  1.14 — Calib-L-Mode-SunSky-noon). `sun_size` = `SS Sun Disk
  Scale` / 4, `groundColor` = `SS Ground Color`. The colour is Octane's, on
  purpose: Hosek-Wilkie warms a low sun as the atmosphere does, Iray's
  stays nearly white (sunlit floor R : B 1.12 : 0.91 of Iray's at 18:00,
  1.11 : 0.93 at noon) — the export matches the light's amount and the
  sun / sky balance, not Iray's hue. **`SS Sun Direction`**
  (a scene node, saved as the channel's `node`) overrides the date: the
  sun stands in the direction of the node's *position* from the origin —
  its rotation doesn't count (Calib-L-Mode-SunSky-dir: a light at (300,
  400, 300) cm turned to point back left lit from the front right, Octane
  0.97 with the same shadows; at the origin Iray rendered night). Left to
  Octane's sky, as looks rather than lighting: `SS Blue-Red Tint`,
  `Saturation`, `Sun Glow Intensity`, `Horizon Height` / `Blur`; `SS
  Night Color` (black by default, as Octane's night).
- **Draw Dome** off (the dome lights but isn't seen): Iray shows black
  behind the scene, the HDRI still lighting it (Calib-Dome-Off; the
  Calib-L scenes are "Scene Only", black anyway). The render target gets a
  black texture environment on its `cameraEnvironment` pin (probe 96; the
  manual calls it "Visible Environment") with `visibleEnvironmentBackplate`
  on — the camera sees it, lighting and reflections keep the HDRI. DAZ's backdrop (`backdrop_visible`) isn't
  followed; no scene in My Library shows one.

- The HDRI as a linear image texture environment, power = intensity (0 for
  scene-only mode). Dome rotation as a U shift
  `0.25 + (Dome Rotation − Dome Orientation Y) / 360` (offset and sign
  measured against Blender at 0° and 110°, Octane2). X / Z tilt left out.
- No HDRI → Octane's white environment.

### Ground shadow

Iray `Draw Ground` (with an HDRI, not scene-only) → infinite plane (110) with
a shadow catcher (145) in a placement 1 mm below Iray's ground height
(`GROUND_DROP`; Manual origin or the scene's lowest point), in the geometry
group. Why 1 mm under: exactly at a floor's height it z-fought (washed-out
planks, hard shadows); 1 mm under the floor covers it and outdoors the
shadow stays. Opacity = **0.89** × `Ground Shadow Intensity`
(`GROUND_SHADOW_OPACITY`): Octane darkens a full shadow to black, Iray keeps
the sky's fill (Octane2 umbra 11 % of lit sand). The scene detail's **Ground
shadow** checkbox (`no_ground`) leaves it out — the plane can't know about
holes in a floor.

## Textures

- Resolved against content dirs; layered images flattened.
- **Budget**: when the decoded RGBA total exceeds **8 GB** (`BUDGET_BYTES`),
  every texture is scaled to the largest of 2048 / 1024 / 512 px that fits,
  re-encoded in its own format (JPEG q90, PNG, TIFF), in parallel. yoga2
  (413 maps, ~17.8 GB decoded) failed in Octane on a 24 GB Mac; at 2048 px
  ~6.4 GB, 1.4 GB `.orbx`, export 25 s. Under the budget textures keep full
  size. HDR / EXR untouched.

## Opening

Octane X is sandboxed and declares no document types: macOS refuses to hand
it any file (`.usdz`, even its own `.orbx`), so ORBIS doesn't launch it. The
report ends with the `.orbx` path; the user opens it via File → Open and
selects the render target named after the scene.

## Open work

What the export doesn't do yet is tracked as issues, each with a test scene
to calibrate on ([calibration](../calibration.md#the-scenes)):

- Materials: anisotropy [#23](https://github.com/vysokyj/orbis/issues/23),
  displacement [#24](https://github.com/vysokyj/orbis/issues/24), top coat
  modes [#25](https://github.com/vysokyj/orbis/issues/25), dual lobe
  [#26](https://github.com/vysokyj/orbis/issues/26), colour effects
  [#27](https://github.com/vysokyj/orbis/issues/27), volume / SSS
  [#28](https://github.com/vysokyj/orbis/issues/28), backscattering
  [#29](https://github.com/vysokyj/orbis/issues/29), flakes
  [#30](https://github.com/vysokyj/orbis/issues/30), thin film
  [#31](https://github.com/vysokyj/orbis/issues/31), diffuse overlay
  [#32](https://github.com/vysokyj/orbis/issues/32), dispersion
  [#33](https://github.com/vysokyj/orbis/issues/33), DAZ Default shader
  [#41](https://github.com/vysokyj/orbis/issues/41), brick / MDL shaders
  [#21](https://github.com/vysokyj/orbis/issues/21), skin hue
  [#19](https://github.com/vysokyj/orbis/issues/19), eyes
  [#20](https://github.com/vysokyj/orbis/issues/20), hair
  [#16](https://github.com/vysokyj/orbis/issues/16),
  [#42](https://github.com/vysokyj/orbis/issues/42).
- Lights and camera: IES profiles [#34](https://github.com/vysokyj/orbis/issues/34), light
  options [#35](https://github.com/vysokyj/orbis/issues/35), lens
  [#39](https://github.com/vysokyj/orbis/issues/39), spots
  [#7](https://github.com/vysokyj/orbis/issues/7).
- Scene: hidden surfaces and render flags
  [#36](https://github.com/vysokyj/orbis/issues/36), fog and finite dome
  [#37](https://github.com/vysokyj/orbis/issues/37), vignetting
  [#38](https://github.com/vysokyj/orbis/issues/38), smoothing and round
  corners [#40](https://github.com/vysokyj/orbis/issues/40), Iray render
  settings [#43](https://github.com/vysokyj/orbis/issues/43), Sun-Sky
  channels [#18](https://github.com/vysokyj/orbis/issues/18).
