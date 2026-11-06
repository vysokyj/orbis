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
<out.orbx|out.usdz|out.usda>` (options in the [README](../../README.md#command-line)).
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
  `.dsf`); `Visible` = false nodes are left out, and so is everything
  under a hidden **group** (`studio/node/group_node`). The same goes for
  lights. A hidden figure or bone doesn't hide its children. A node with
  `Renderable` = false is left out too (its own flag; its children stay).

  Why: in DAZ (`isVisible()`) the blonde in Calib-Hair-Teens went
  invisible with either of her two groups hidden, and Iray rendered
  nothing of her, but she stayed visible with the dolly or its bone hidden.
  intake's dForce hairs render under their hidden scalp caps (#53).
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
- **Cast Shadows** and **Render Priority** are **not mapped**: DAZ doesn't
  hand them to Iray. In its .mi every mesh instance has all render flags on
  (`shadow 3`, …) and `label` 0, whatever the node or an instance node of
  it says (Calib-Mi-Priority: five priorities from Lowest to Highest;
  Calib-CastShadows-Off: the sphere and an instance of it; test in
  `src/iray.rs`), and Iray Photoreal ignores `shadow_cast` anyway (Iray
  manual 4.3.5) — as for a light's Cast Shadows (#35). Why it matters: the
  first export (probes 100–102) took the sphere's shadow away, which Iray
  draws (Calib-CastShadows-Off's Iray render: the sphere and its instance,
  both off, cast it, as in Octane now). Octane's object layer input per prim (`/Scene/<prim>_object`,
  `objectLayerImport` 3) is still what shows a light to the camera.
- **Hidden surfaces and facet groups**: any node (not only a shell) drops
  the material groups its `material_group_<group>_vis` channels switch off
  and the facet groups (`polygon_groups`, a figure's bone regions) its
  `facet_group_<group>_vis` channels switch off — cut after refining, as
  for shells. A welded graft's facet groups are named `WG_<k>_<group>`
  (k: the graft's index among its host's grafts): NGV8's shells switch off
  the host's `Hip` and the Effect Layer `WG_0_Rectum` / `WG_0_Vagina`, yet
  show the graft's own Hip facets. The NGV8 shells are the only users of
  these channels in My Library. DAZ creates them only on a geometry shell
  (a figure has no such channels), and hands Iray the shell cut the same
  way: Calib-Shell-G9 (Genesis 9 with a red shell, Body and Arms shown,
  `l_forearm` and `l_pectoral` facets off) has 343 776 shell triangles in
  the .mi and in the export, and Iray and Octane show the same skin
  patches through it (the left pectoral, the left forearm; #36).
- **Visible grafts are welded into their host mesh**: graft points follow the
  host's, border vertices are remapped onto their paired host vertex, graft
  groups are named `<graft node>_<group>` and keep the graft's materials and
  UV set. Grafts inside a shell are welded too. (Two meshes subdivide apart
  and leave a seam or a floating outline.)
- **SubD**: `subdivision_surface` at `lod > 0` is refined by the export
  as many times as Iray does: SubD Level + Render SubD Level (the scene's
  geometry instance over the definition; DAZ's defaults 1 + 0; Genesis 8
  Female 1 + 1 = 2), at most 3 (Catmull-Clark, `mesh::subd_levels` — DAZ's
  .mi export of G8F at every pair 0 + 0 … 2 + 2 had the vertices of the sum;
  #62). Every mesh is written with
  `subdivisionScheme = "none"` and its own normals (below) — Octane has no
  per-mesh level, and "subdivide all" rounds hard props.
- **Normals (smoothing, #40)** as DAZ hands them to Iray: DAZ bakes its
  auto-smoothing (the material's `Smooth On`, `Smooth Angle`, default on,
  89.9°) into the trilists' vertex normals (Calib-Smooth .mi). An edge is
  hard when its two faces' normals lie further apart than the angle (the
  stricter of the two faces' materials; Smooth off makes all of a face's
  edges hard). Around a vertex, the faces joined through soft edges share
  the area-weighted sum of their normals (`smoothed_normals`). Material
  borders don't split.
  - Written as **`primvars:normals`**: per vertex, or `faceVarying` with
    `primvars:normals:indices` where a vertex carries more than one.
  - Why `primvars:`: Octane's USD import ignores a plain `normals`
    attribute. It then computes its own normals at the import's
    `maxSmoothAngle` (45° in our project). `primvars:normals` lands in the
    mesh node's `normals` / `polyNormalIndices` (probe). Before #40 every
    export rendered with Octane's 45° normals.
  - Why the edge rule: on Calib-Smooth .mi, Smooth off and 30° split an
    icosahedron (41.8° edges) into 60 corners, while 45° and the default
    keep 12 normals. With a rule measured against the face's own normal,
    45° would split it, since faces across a vertex lie 70° apart. At
    89.9°, a cube stays 24 corners and a 12-sided cylinder is smooth round
    its side and split at the caps (50 normals). At 91° the cube has 8
    corner normals. Two materials on an icosahedron kept 12. Our export
    has the same normal counts for all of them.
  - Octane (Calib-Smooth, enhanced-final): it renders the faceted, smooth
    and blobby (91°) shapes as DAZ means them. Calib-Hair and
    Calib-L-Distant-45 against their earlier renders: mean |Δ| 0.33 and
    0.15 / 255, so figures and plain props didn't change.
  - Against Iray (Calib-Smooth, 5000 samples; Octane enhanced-final 1024):
    the same faceted (S1, S2), smooth (S3–S5) and blobby (S7) shapes, the
    sharp cube (S6) and the cylinder smooth round its side with crisp caps
    (S8). Per object Octane is 1.02–1.06× Iray's level (the whole frame
    1.017), mean |Δ| 2.5–4.9 / 255.
  - Not known: how DAZ weights the faces in the sum (area, angle). Every
    test shape is symmetric, so we use area weights, as before.
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

`.orbx` with `assets/scene.usdc` — the scene as a **binary USD layer**
(Pixar's crate format 0.8.0, own writer `src/usd/usdc.rs`; layout in
[usd.md](../octane/usd.md#usdc-crate-layout)), textures `assets/tex/NNNN_<file>` (incl. flattened layered images), VDB
volumes `assets/vdb/NN_<file>` and
`project.ocs` from the template `assets/octane/project.ocs`.

- **Binary layer, streamed.** Each mesh's arrays go into the package as
  the layer is written and the mesh is dropped right after; the structure
  (tokens, paths, specs, fields) follows, then the header is patched in the
  package's first chunk. Integer arrays of 16+ use Pixar's integer coding
  (deltas, 2-bit codes) in LZ4 blocks of literals only (valid LZ4, no match
  search); floats as they are. A mesh's points are numbered in the order
  its faces first use them (normals with them): a refined mesh's face,
  edge and corner points come in three runs and its face indices took 4.0
  bytes each, 1.2 in use order. Why: the text layer of intake (60 M mesh
  points, 4.9 M strand points) was 9.6 GB — over the zip's 4 GiB, so no
  `.usdz` could hold it — and the export peaked at 22.7 GB beside the
  meshes. Octane X imports the bare `.usdc` and a `.usdz` holding one
  alike, rendering as the text layer did (Calib-Hair, #16).
- A `.usdz` export holds `scene.usdc` (stored, 64-byte aligned); `-o
  x.usda` writes the same layer as text (`UsdaWriter`, also what the tests
  read). Text keeps 5 decimals for points, 4 for normals, 6 for UVs and
  widths; binary keeps the floats whole.
- Library `.dsf` files are read without `modifier_library`, and a geometry
  or UV set entry leaves its file's JSON once parsed (intake's 300 MB of
  library files took 6.4 GB as JSON values). Notes:
  [2026-11-05](../notes/2026-11-05-export-memory.md).

Scene graph:

```
Scene
├─ "<scene> (DAZ 3D)"   USD import graph (type 2, filename assets/scene.usdc)
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

### Lens options (#39)

DAZ hands Iray every lens channel (`iray::Lens`, verified by the
Calib-Lens-* `.mi` fixtures): the mm channels (Lens Shift X / Y, Lens
Stereo Offset, Lens Thickness) as cm, Aperture Blade Rotation in radians,
K1–K3 as `mip_lens_distortion_params`, K4 / K5 as `…params2`, Lens
Distortion Type as Iray's name (none, spherical, cylindrical, poly3,
inv_poly3, poly5, inv_poly5, ptlens, inv_ptlens, brown, inv_brown,
equidistant), Radial Bias, Distortion Scale, Maximum FOV as they are.
Octane (`usd::lens`):

| Iray | Octane | Measured (Iray refs, 5000 samples, #39) |
|------|--------|--------|
| `mip_lens_shift_x/y` | the image moves **shift × F / 2** on the film (F the focal length, cm), with the shift's sign (x right, y up); thin lens `lensShift` = −that / film width, −that / film height (Octane's units are fractions of the image's width / height and move the *view*: 0.1 moves a 1920×1080 frame's content 192 px left, 108 px down) | Calib-Lens-Shift (0.5, −0.3 at 65 mm): Iray's content moved 349.5 px right, 209.7 px down; F / 2 predicts 351.0 / 210.6 — 0.4 % wide, fits within 1.7 px. One focal length referenced: F / 2 rather than a constant 3.24 cm is the reading (shift as half a tangent), not proven |
| `mip_aperture_num_blades` > 2, `…blade_rotation_angle` | `bokehSidecount` = blades, `bokehRoundedness` 0, `bokehRotation` = (90° − rotation) / 360°, mod 1; fewer blades → round (roundedness 1) | Iray's polygon has a corner at the rotation counter-clockwise from the image's +x (Calib-Lens-Blades5, 20°: a corner 92.5° ≡ 20.5°). Octane's `bokehRotation` is a fraction of a turn (pin range 0–1; degrees clamp to 1, a full turn), a corner up at 0, positive clockwise (0–60° in 12° steps). Verified on 5 blades |
| `mip_lens_radial_bias` b | thin lens **`aperture_edge` = 1 / (2b)**, within the pin's 0–3; the OSL camera samples the lens radius as ξ^b | Iray samples the radius as ξ^b (0.5 uniform; < 0.5 a ring, > 0.5 a peaked centre). Calib-Lens-Bias02: edge 2.5 best of 1, 1.5, 2.5, 4; -Bias08: 0.625 best of 1, 0.4, 0.625, 0.8. Bokeh RMSE vs Iray: 5.17 → 2.72 (thin lens) / 2.38 (OSL ξ^b) at 0.2, 4.15 → 2.76 / 2.41 at 0.8; Calib-DOF-f4's own 2.39 |
| `mip_lens_stereo_offset` | the camera moved that far to its right, parallel (Iray manual 13.3: the right eye, the left in a canvas's second layer) | not settled: the Iray Server's Calib-Lens-Stereo is a featureless beige gradient, no eye of the scene in it (Iray's stereo into a single-layer canvas misbehaves); kept as the manual reads |
| `mip_lens_distortion_type` ≠ none | an **OSL camera** (NT_CAM_OSL 126, `shaderCode`) in the thin lens's place, Iray's model written out (`lens::osl_camera`) | below |
| `mip_lens_thickness` | not mapped: it does nothing | Calib-Lens-Thick (50 mm on Calib-DOF-f4) is Calib-DOF-f4 pixel for pixel (mean \|Δ\| 0.00) |
| `mip_lens_max_fov` | the equidistant fisheye's cut-off at half of it (rays beyond: none, black) | from the manual |

**The OSL camera.** The image point on Iray's film (cm, the image shift
above subtracted), its radius normalised as r = scale · distance / film
width — the normalisation the manual's equidistant table implies (k2 = θ / s
for a cropped fisheye's horizontal FOV θ). Perspective models send the ray
through the undistorted film point (`ru / rd` × the film point, focal length
on the axis): forward models (rd = P(ru)) solved by 16 Newton steps, inverse
ones (ru = P(rd)) direct. Equidistant: exit angle θ = P(rd) at the point's
azimuth. Spherical: a full 360° × 180° lat-long. **Cylindrical: 360° across,
perspective up the cylinder** (the ray's height film y / F) — not square
pixels: fitted against Iray's own spherical render as an environment, the
vertical scale is 0.77 = film height / focal length (correlation 0.9997),
where square pixels (3.53) rendered the scene 4.6× too flat. Depth of field:
Octane's `camera:dofrandom` over Iray's lens (a disc of the lens radius, or
the blade polygon with a corner at the rotation from +x, the radius as ξ^b),
focus on the plane at the focus distance (perspective models) or the sphere
(the rest). Against Iray (Calib-Lens-Poly3, -InvPoly5, -Brown, -Equidistant,
-Spherical, -Cylindrical): every 320×270 tile within 1 px, edge correlation
0.972–0.979 as the undistorted Calib-Tone-Default's 0.975.

## Render modes

`ExportOptions::octane_mode`; the scene detail offers Final / Preview.

| | Preview | Final (default) |
|---|---|---|
| kernel | direct lighting (24), GI diffuse, depth 2 | path tracing (25): diffuse 8, glossy 24, **scatter 128**, **GI clamp 20** |
| ray epsilon | **1 µm** (`rayepsilon` 1e-6) | same |
| skin | Universal | Standard Surface + random-walk SSS |
| imager exposure | **1.4** × 2^(13 − EV) × cm² (Variants) | **1.4** × 2^(13 − EV) × cm² |
| imager response | **Gamma 2.2** (403) | same |
| highlight compression | 0; Burn < 1, Crush > 0, Gamma ≠ 2.2 or a coloured White Point → a tone LUT (below) | same |
| vignetting | Iray's cos(θ)^v fitted as Octane's frame-space cone per camera (below) | same |
| white balance / saturation | white (the White Point is in the tone LUT, below) / Saturation | same |
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

**Burn Highlights Per Component** off (#38): Iray burns the Rec. 709
luminance Y and keeps the chromaticity — c · burn(Y) / Y, then crush and
gamma per channel. A 1D LUT can't do that, so such a scene (with Burn < 1)
gets a **3D LUT** (`LUT_3D_SIZE` 33, the same input, not clipped at 1 — a
clip inside a cell bent the interpolation by up to 4 / 255; trilinear error
≤ 0.2 / 255). Why: Calib-Tone-BurnComp-off predicted from -on's Iray pixels
(the per-channel curve inverted) within 0.44 / 255 where the two differ by
3.1 (Rec. 601 weights as well; the channels' mean 0.7, maximum 2.7).
Octane reads the 3D LUT: its render is the curve on its Default render
within 0.55 / 255 (the 1D LUT 0.53), and Octane's off − on shift is
−2.7 / +0.2 / +5.9 per channel against Iray's −2.4 / +0.5 / +6.1.

**Vignetting** (#38): Iray darkens linear values by cos(θ)^v, θ the ray's
angle to the camera axis (Calib-Tone-Vignette05 / 1 over -Default: the
exponent 0.50 / 1.00 in every band of θ). Octane's imager `vignetting` v_o
darkens by 1 − v_o · e, e = √(x² + y²) / √2 in the frame's coordinates
(each −1 … 1; slope 1.00, power of e 1.00 at v_o 0.25 / 0.5 / 1, the same at
FOV 40° and 69°): a cone that ignores FOV and aspect, where Iray's falloff is
round and steeper for wide lenses. The export sets v_o per camera to the
least-squares fit of 1 − v_o · e to cos(θ)^v over the frame
(`ToneMapping::octane_vignetting`; Calib-Tone, 69° at 16:9: 0.5 → 0.086, 1 →
0.165). Rendered over Calib-Tone-Default, Octane / Iray:

| region | Iray 0.5 | Octane 0.5 | Iray 1 | Octane 1 |
|---|---|---|---|---|
| centre | 1.000 | 0.989 | 1.000 | 0.979 |
| left / right edge | 0.923 | 0.945 | 0.850 | 0.895 |
| top / bottom edge | 0.973 | 0.946 | 0.943 | 0.895 |
| corners | 0.904 | 0.925 | 0.819 | 0.854 |

Mean |Δ| 1.7 % / 2.7 % of the pixel, the frame's mean −0.3 % / −0.6 %: the
amount matches, the shape can't (the cone, and Octane darkens the top edge
as much as the side one).

**White point** (#70): DAZ hands Iray `mip_whitepoint` = White Point^2.2 ×
White Point Scale (`iray::white_point`, .mi fixtures Calib-Tone-WhiteWarm,
-Cool, -Green, -WhiteScale2: the colour linearised, the scale not). Iray
multiplies each linear channel by k / wp before the burn, k = 1 / Σ yᵢ / wpᵢ
with the CIE 1931 RGB luminance weights y = (0.17697, 0.81240, 0.01063) —
the white point's hue goes grey, a white pixel keeps that luminance; a grey
white point at any scale changes nothing. Octane's imager `white_balance` is a chromatic
adaptation, not that division (raw wp on the pin: R/G 0.761, B/G 1.319 where
the division gives 0.8 / 1.333; wp^2.2: R/G 0.471 for 0.612), so it stays
white and the gains go into the tone LUT (`ToneMapping::white_gains`; the 1D
LUT gets a column per channel, the 3D LUT takes them before the luminance
burn). Over -Default, per-pixel medians of the linear ratio (gamma 2.2,
pixels 0.15–0.9 in both):

| scene (White Point) | Iray R G B | Octane R G B | k: Iray / CIE RGB / mean / Rec. 709 |
|---|---|---|---|
| WhiteWarm [1, 0.8, 0.6] | 0.652 1.063 2.003 | 0.650 1.063 2.001 | 0.651 / 0.651 / 0.646 / 0.674 |
| WhiteCool [0.6, 0.8, 1] | 1.634 0.867 0.531 | 1.634 0.869 0.531 | 0.531 / 0.531 / 0.646 / 0.579 |
| WhiteGreen [0.9, 1, 0.7] | 1.193 0.944 2.069 | 1.190 0.944 2.069 | 0.945 / 0.944 / 0.750 / 0.917 |

Iray's ratios are k / wp on every channel to 0.5 % (a diagonal in RGB;
Bradford, CAT02 or von Kries adaptations miss by 3–25 %). Fitting k = 1 /
Σ wᵢ / wpᵢ with three free weights gives 0.177 0.813 0.010, summing to
1.000: CIE 1931 RGB's luminance row. Octane against Iray, mean |Δ| per
pixel: WhiteWarm 1.95, -Cool 1.83, -Green 2.05 / 255, Calib-Tone-Default's
own 1.92 (Octane's usual linear offset); before (white balance = raw wp)
WhiteWarm was 9.9.

### Render settings (#43)

DAZ's Iray render settings (`scene.extra[studio_render_settings]`, the
Photoreal options in a child element; `crate::usd::render_channels`) as
DAZ hands them Iray (`iray::render_options`, checked against every `.mi`
fixture's `options`) → the kernel and the render target's post
processing (`src/usd/render_settings.rs`). Scenes: **Calib-RS-*** (Calib-Glass
with one setting changed; Iray references at 5000 samples), Octane at
256 samples, Enhanced + Final.

| Iray (DAZ channel) | Octane | Why |
|---|---|---|
| Max Path Length N (−1 = unbounded) | every depth of the kernel capped at **N** (path tracing: diffuse, glossy; direct lighting: diffuse, glossy, specular) | Octane's depth counts path segments as Iray does: Calib-RS-PathLen2 at depth 2 renders the refracting spheres black as Iray, floor 1.04 × Iray (at depth 1 it lost half its light, 0.51 ×); -PathLen4 at depth 4 within 5 % per sphere (at 3 the glass spheres came out at half) |
| Max SSS Path Length N | `maxScatterDepth` min(128, N) | the same bound (volume vertices); not rendered |
| Firefly Filter off | `giClamp` 10⁶ (Octane's maximum: no clamp) instead of 20 | Iray's filter clamps outliers like Octane's GI clamp; Calib-RS-FireflyOff changes neither renderer's pixels in this scene |
| Pixel Filter, Radius r | `filtersize` 1.2 × σ / σ_default (Octane's range 1–8) | σ of Iray's filter: Gauss r / 3 (it ends at 3 σ), box r / √3, triangle r / √6 (width 2r, the .mi's `gauss 3`). Measured as the extra Gaussian blur that turns one render into another: Iray Gauss 1.5 → 3 +0.9 px, → box 1.5 +0.8 px (predicted 0.87 and 0.70 from σ 0.5); Octane `filtersize` 1.2 → 1.8 / 2.4 / 4 +0.6 / +0.8 / +1.5 px, i.e. σ ≈ 0.38 × size (1.2 ≈ Iray's default 0.5). Exported: Calib-RS-PixelRadius3 +0.8, -PixelBox +0.7 px over Octane's default |
| Bloom Filter (Threshold T, Radius r, Brightness Scale k) | render target's post processing on: `cutoff` T / 10⁴ × the imager's exposure, `bloom_power` k, `spreadStart` 0.0001, `spreadEnd` r, `glare_power` 0 | see Bloom below |

**Bloom.** DAZ passes all four channels as they are (`.mi`: Calib-RS-Bloom,
-Mi-Bloom). Iray's threshold is in its raw pixel values, before the tone
mapper: at T = 1 (Calib-RS-Bloom) every pixel blooms — the render is the
plain one blurred, a least-squares fit B = a·I + b·G_σ(I) (linear) gives
a 0.03, b 0.98 at σ 16 px = r × 1920 / 6, the mean unchanged (1.002):
Iray *replaces* what is over the threshold by its Gaussian blur. Octane's
`cutoff` is measured after the exposure, so T ÷ the dome scale (Iray's
10⁴ per unit of Octane's environment power, `DOME_INTENSITY_SCALE`) × the
imager's exposure is the same pixel (DAZ's default 2000 at EV 13: 0.28).
Iray's bloom, fitted on Calib-RS-Bloom2000 / -Bloom10000 against
Calib-Glass (5000 samples, display-linear) as B = I − w + k·G_σ(w),
w = max(I − t, 0): σ 16 px, k 0.98 / 0.97 (the brightness scale 1), the
threshold t 0.050 / 0.235 of the display's white — the export's cutoff for
this scene (exposure 0.2475) is 0.0495 / 0.247, within 5 %: the conversion
holds, and Octane's exposed value is the display-linear one. The fit
leaves 1/18 and 1/12 of the change unexplained; the mean stays (1.0001):
Iray moves the light over the threshold into a 16 px halo. At DAZ's
default 2000 that is nearly the whole frame (the floor blooms).

Octane's bloom *adds* a wide, faint glow and takes nothing from the bright
pixels: at that cutoff, power 1 adds 0.66 % / 0.09 % to the mean, power 4
2.6 % (linear in the power), mostly as a broad lift of the bright sky; in
Iray's halo ring it gives 1/28 (power 1) to 1/7 (power 4) of Iray's added
light, and over the blooming cores it brightens where Iray darkens
(correlation with Iray's change −0.1). Spread start 0.01, end 0.02 / 0.2
change it by less than the noise. Matching the halo would take power ≈ 28
and +18 % on the whole frame, against the light amount (Goal); the
export keeps `bloom_power` = k, Octane's subtle glow scaled as Iray's
brightness scale scales its halo (NVIDIA's Iray manual aims the filter
at subtle halos around emitters). Neither renderer can do the other's: Iray's
threshold 1 (Calib-RS-Bloom) replaces the frame by its blur. Octane X
rendered every post-processing variant
without trouble (the crash in #37 was with its post-volume fog).

**Not mapped**, with the reason:
- **Caustic Sampler**, **Guided Sampling**: sampling strategies; the
  converged image is the same (Calib-RS-Caustics vs Calib-Glass: mean
  |Δ| 0.0001). Octane's path tracer renders caustics by itself (its
  Caustic Blur is a bias, not Iray's sampler).
- **Spectral Rendering** (+ conversion colour space, intent, observer):
  Octane always renders spectrally; Calib-RS-Spectral equals Calib-Glass
  (|Δ| 0.0004).
- **Min / Max Samples, Max Time, Rendering Quality, Converged Ratio**:
  Iray's samples aren't Octane's; the export keeps its own sample count.
- **Post Denoiser** (Iray's): Octane's denoiser follows the render mode
  (Render modes) and `--no-denoise`, not DAZ's switch — Iray's denoiser is
  off in most scenes, which would turn Octane's off in Final.
- **Degrain**, **Nominal Luminance**, **Texture Compression**,
  **Instancing**, **Low Memory**, **Update Interval**: speed / memory /
  progressive display, no effect on the image Octane can match.
- **Alpha** (Default / Custom Alpha LPE): DAZ's alpha only matters for a
  PNG saved with transparency; Octane X saves its own alpha.
- **Canvases** (render to canvases: Beauty, Diffuse, Normal, Depth, LPE…):
  output passes, not the look. Octane has the matching render AOVs
  (diffuse 188, geometric normal 198, Z depth 255, …, in a render AOV
  group, 179), but the export doesn't fill them: DAZ's canvas list format
  isn't known (no scene with canvases; DAZ's plug-in names only the 23
  types) and Octane X crashed on a probe building an AOV group
  (2026-11-04). Enable the AOVs in Octane X.
- **White Mode**, **Section Caps**: DAZ preview aids.
- **Iray Interactive** options: another Iray render mode; the export
  stands for Photoreal.

## Materials

Every material input (20007) is filled — Octane doesn't convert
`UsdPreviewSurface`.

**DAZ channels → Iray parameters** first, verified: `src/iray.rs` computes
the MDL arguments DAZ Studio passes Iray (`iray_uber` / `iray_uber_unified`)
exactly as DAZ's `.mi` export of every calibration scene shows them
(`calib/references/mi/`, a test compares all of them). That step is never
tuned by calibration; only what the Octane side makes of the parameters is.
Where this spec states a channel conversion that `src/iray.rs` contradicts,
`src/iray.rs` wins (#62).

### Legacy shaders → Iray Uber (#41, #64)

DAZ Studio renders its old shaders in Iray as an Iray Uber material; the
export converts them the same way on load (`usd::mesh::scene_materials`,
`iray::as_uber`), so everything after sees Uber channels:

- **DAZ Default** (`daz_default_as_uber`, Calib-Mi-DzDefault2): diffuse ×
  diffuse_strength; glossy weight = the specular colour's strongest channel
  (linear) × specular_strength, its hue the glossy colour, roughness
  (1 − glossiness)^1.7; refraction at its strength, colour and IOR;
  transparency as Cutout Opacity. Ambient, reflection and the lighting
  model don't reach Iray.
- **Scripted 3Delight shaders** (`studio/material/daz_shader`: UberSurface,
  HumanSurface, dzGlossy / dzMetal / dzPlastic / dzSkin / dzMatte / toon,
  …; `legacy_shader_as_uber`, Calib-Mi-Legacy, -Legacy2). DAZ Studio 6 has
  no 3Delight and can't load their definition scripts: the material keeps
  only Diffuse Color and Opacity Strength, whatever the shader or what an
  older (DAZ Studio 4) save carries. Iray gets the diffuse colour and the
  opacity as Cutout Opacity **without their maps**, and Uber's MDL defaults
  otherwise: a white glossy layer at weight 1, roughness 1, thin-walled.
  The export takes that, but **keeps the diffuse and opacity maps** in
  every look (decided on #64, under Octane over Iray: what the scene means
  — a beard's colour and its alpha-mapped cards — which Iray loses and
  renders as flat, opaque cards).
- **AoA Subsurface** (Age of Armour's 3Delight brick graph, in DAZ's
  default resources; `aoa_subsurface_as_uber`, Calib-Mi-AoA … -AoA4):
  diffuse × Diffuse Strength, bump, normal map (its strength doesn't reach
  Iray: factor 1), displacement, tiling and opacity pass as Uber's
  same-named channels, maps included. Specular: glossy weight = Specular
  Color's strongest channel (linear) × Specular Strength, hue as the
  colour, roughness 1 − Glossiness (not squared). Specular 2 → the top coat
  by the same rule, roughness min(1.5 (1 − g), 1) — or the glossy layer
  itself when Specular Strength is 0. A Specular Strength map drives the
  glossy weight, a colour map the glossy / top-coat colour (Color Effect
  1); maps on the glossinesses or Specular 2 Strength don't reach Iray, nor
  do reflection, velvet or ambient. **Subsurface is not mapped**: with SSS
  Material Type Custom Iray gets a closed volume (absorption σa = 100 ×
  absorb / Shading Scale, deciphered) and a scattering amount that isn't
  deciphered, and no scene in My Library has it on — the export keeps the
  surface closed, as Iray, but adds no medium. The other SSS presets give Iray
  nothing.

Two Uber inputs these scenes showed first (any shader): a **Diffuse
Strength map** attenuates the colour (`attenuate_color`) — Universal's
albedo is a Multiply of the map and the colour (or the tinted colour map);
a **height and a normal map together** — the normal map is the base bump
and the height map a secondary one blended onto it (`add_detail_normal`,
`base::blend_normals`) — go on Universal's `normal` and `bump` pins, which
Octane combines the same way (the bump over the normal-mapped surface; a
scratch scene with dimples as the normal map, ridges as the bump showed
both).

DAZ keeps a `color`-typed channel (DAZ Default's, the scripted shaders') in
8 bits: a generated 0.3 reaches Iray as 77/255 (saved scenes are already
quantised).

**Not mapped: DAZ doesn't pass them** (their .mi equals the twin's without
them; the Uber test also fails on any argument DAZ passes that `src/iray.rs`
leaves out, #15):
- **Multiply Specular Through Opacity** off (a DAZ Default channel, also
  saved on brick materials such as eye moisture or LIE helpers): Iray gets
  the same `cutout_opacity` and gloss either way (Calib-Mi-DzDefault3
  S1 / S2, Calib-Mi-AoA5 S1 / S2, on Uber Calib-Map-Audit A8).
- **PBR Masked Threshold** (a user property some tear materials carry, no
  MDL parameter): Cutout Opacity passes as it is (Calib-Map-Audit A5 – A7:
  0.9989 under 0.999, 0.5 under 0.7 and over 0.3).
- **Top Coat Specular** (MDL `top_coat_directional_*`): in none of the four
  layering modes (Calib-Map-Audit A1 – A4, Calib-Map-TopCoat C6) — see
  Universal's coating below.

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
  are shared out by the weighted mix). A closed, non-thin solid with Iray's
  volume under it (Transmitted Color set; ~313 installed materials) has its
  `transmission` × the skin's volume factor (`volume_factor`, Rd(α) /
  Rd(α₀))^0.5 per channel) × 0.7 (`PROP_VOLUME_LEVEL`): Calib-SSSColor's
  spheres with SSS Color grey / blue / deep 0.84 – 1.05 × Iray, before 1.05 –
  2.68; mono 1.40 – 1.54 (before 1.40 – 1.69; Iray's mono spheres are darker
  than Rd says, its figures weren't), SSS Reflectance Tint 1.34 (unread,
  darkens there) (#28). Why: left out, Calib-SSS's spheres at
  w 0.5 / 1 rendered 1.6 – 1.7 / 3.2 × Iray and Calib-PBRSkin's Gladio cloth
  1.72; with it 0.91 – 0.96 and 1.23 (its neighbours 1.14 – 1.26), the two
  types alike on closed spheres. On P8 (metallic 0.75) the transmission
  swallowed the metal (0.23 × Iray). Thin-walled or not, figure or prop
  alike — Iray knows only the material: Calib-Thin's thin-walled cards (w
  0 / 0.3 / 0.5 / 0.65, Translucency Color white / the diffuse / dark, lit
  from the front with the dark behind them or from behind) 0.89 – 1.06 ×
  Iray; scaling the albedo (× 0.44 – 0.7) or the transmission (× 1.5 – 2)
  only did worse. **Base Color Effect** 1 / 2 (Scatter & Transmit, …
  Intensity; 3685 / 278 translucent materials installed) change it as
  DAZ's MDL does (`diffuse_base`, `transmit_color_calc`,
  `base_color_effect_shares`): the diffuse layer's weight becomes (1 − w) ×
  the diffuse colour's mono m — its average in effect 1, its strongest
  channel in effect 2 (the colour greyed at its own intensity first) — its
  tint the colour at full intensity (÷ its strongest channel), and the
  translucency takes the rest. So the albedo is (1 − w)·D × m / max(D),
  the transmission Tc × (1 − (1 − w)·m) in place of w; D is the diffuse
  colour × its map's mean, as DAZ hands it to Iray — with the SSS
  Reflectance Tint multiplied in when the effect's channel is saved shown
  (`iray.rs`, Calib-Mi-SSSTint; Morgan Hair's tint 0.68 / 0.67 / 0.62
  takes its diffuse to 0.43 / 0.41 / 0.34 ×, the 1.6 – 2.3 × its strands
  rendered too bright, #52). This replaces the shares once fitted (effect 1
  0.4 / 2 ×, effect 2 0.95 / 1.55 ×), which the rule reproduces:
  Calib-Thin's green cards (effect 1, m / max 0.41) measured a scalar 0.40
  – 0.41 and about twice the translucency; Calib-ColorEffect's blue C3
  (effect 2) 1.0 and 1.39 ×. Renders against the fitted shares (Octane /
  Iray, R G B): Calib-ColorEffect C2 (effect 1) 1.23 1.11 0.82 → 0.90 0.99
  0.98, C3 1.01 1.00 0.95 → 0.83 0.97 1.00 (C1, effect 0, is 0.87 1.00
  1.01); Calib-Map-Effect E1 0.81 1.12 1.19 → 0.74 1.04 1.07, E2 0.95 0.96
  0.93 → 0.97 1.04 1.06; Calib-Thin's -bce cards unchanged (#27, #45, #52).
  A **partial refraction** (Refraction Weight < 0.5) keeps the
  translucency: Iray's refraction is a weighted layer over diffuse and
  translucency, so both keep 1 − Refraction Weight; the translucency then
  holds Universal's one transmission (the refraction's own goes; its IOR
  and gloss stay). Morgan Hair (w 0.059, effect 2, refraction 0.106):
  transmission 0 → 0.30 × Tc. The Genesis EyeSocket is such a material: its
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
- **Dual lobe specular** (skin's, no glossy layer): Iray blends two GGX
  lobes by `Dual Lobe Specular Ratio` — on Iray Uber the ratio weighs lobe
  1 (default 0.85) and lobe 2 has `Specular Lobe 2 Roughness` (default 0, a
  mirror); on PBRSkin it weighs lobe 2 (default 0.15), whose roughness is
  lobe 1's × `Specular Lobe 2 Roughness Mult` (0.4) (DAZ's MDL). Lobe 1 is
  `specular` at the layer's share (weight × min(2r, 1)) × its part; lobe 2
  the `coating` at the rest, `coatingRoughness` **2 · r2²**, IOR 1.5. A
  reflectivity r above 0.5 sets the IOR from F0 = 0.08 r on both. Under a
  Top Coat (which takes the coating) one lobe at the full share and the
  ratio's blend of both roughnesses. The same on OpenPBR skin (without the
  IOR). Why: Calib-DualLobe rendered one sharp highlight whatever the ratio
  (p99 / median of the sphere 9 – 13, Iray 5.0 – 8.1 for lobe 2 at 0.6);
  now 6.4 / 5.2 / 7.6 / 9.3 against 8.1 / 5.5 / 8.1 / 9.0, lobe 2 at 0.1
  9.8 vs 10.8; r 1 0.79 → 1.17 × Iray. Octane's coating is sharper than
  Iray's lobe at the same squared roughness (r2 0.6: 9.5 at 0.36, 5.7 at
  1). Not reached: lobe 2 at 0.9 (Iray 3.3, Octane 6.6 at 1), and lobe 1
  above ~0.3 (#56). On a skin the ratio moves little: Victoria's forehead
  0.92 × with all of it on lobe 2, 1.00 on lobe 1 (Iray).
- **Roughness** by Base Mixing: 0 and 2 → `Glossy Roughness`, 1 →
  1 − `Glossiness` (both channels are always saved; Calib-Materials:
  Glossiness 0.8 beside Glossy Roughness 0.5 rendered Iray's sharp
  highlight; Weighted reads Glossy Roughness — DAZ's .mi export,
  `src/iray.rs`, #62), then **squared** when `Roughness Squared` / `Glossiness
  Squared` (default on) — Iray squares the slider, Octane doesn't:
  highlights went from 31 / 17 / 6 px to 48 / 24 / 22 against Iray's
  56 / 26 / 20 (#10).
- **Thin films tint what is under them.** DAZ's MDL (`add_thin_film`) does
  not layer a film: it multiplies the whole BSDF under it — diffuse,
  gloss, metal, refraction — by `df::directional_factor` between an RGB
  tint head-on and one at grazing angles, (1 − cos θ)^1.1 apart, the tints
  from NVIDIA's `legacy_thin_film::compute_refl` at cos 0.99 and 0.10.
  Base Thin Film is over the base, Top Coat Thin Film over everything,
  coat included (only when there is a coat); no layer weight scales them.
  The export ports `compute_refl` and multiplies Universal's `albedo`,
  `specular` and `transmission` (both films) and `coating` (the coat's)
  by a Mix texture of the two tints by a Falloff Map (normal vs. eye ray,
  skew 1.92: (θ / 90°)^1.92 fits (1 − cos θ)^1.1 within 0.015). The
  specular is then written even when it is Universal's white default.
  Octane's own film (`filmwidth`, `filmIor`) is not used. Metallic Flakes
  Thin Film waits for the flakes (#30).

  Why: Calib-ThinFilm. Iray's black glossy spheres are tinted warm
  (300 nm), olive (500 nm, also on a coat and a metal), purple (800 nm)
  and blue (500 nm at IOR 1.3), most at the rim — as the tints predict.
  Octane rendered them 1.08 – 1.82 × Iray untinted, and 1.08 – 1.12 now
  (800 nm 1.28; the plain sphere 1.08). Octane's film only dims the
  specular, and hard: 0.55 – 0.73 × Iray with it. Calib-FilmIOR (films
  over a matte grey): the head-on tint at IOR 1.91 is 0.77 – 0.83, the
  diffuse Iray leaves 0.77 – 0.81; 1.09 – 1.20 × Iray with the earlier
  fitted albedo dimming, 0.96 – 1.14 now (#31).
- **BSDF model and roughness.** Uber Universal materials (not rough glass
  — Beckmann, see Refraction — nor the skins' Standard Surface / OpenPBR) use GGX energy
  preserving (`brdf` 6; plain GGX lost light on rough lobes: Calib-Glossy
  roughness 0.6 1.17 → 1.01 × Iray, DualLobe2's r 0.36 1.25 → 1.08) at
  roughness min(r + 1.85 r², √r), r = Iray's roughness after its squaring; the
  coating too. Iray's lobes are Phong (2 / r², MDL `simple_glossy_bsdf`);
  Octane's default BSDF kept rough highlights sharp and bright.

  Why: Calib-DualLobe2's rough lobes went from 1.47 – 1.99 to 1.02 – 1.25
  × Iray with Iray's highlight spread, Calib-Glossy's spheres 1.06 – 1.21
  → 1.02 – 1.16 (roughness 0.6: 1.47 → 1.17); GGX at r stays sharp above
  r ≈ 0.3, at √r too broad below it; Beckmann stays sharp (#56). Above
  r 0.36 the curve passes √r, which caps it: Calib-Map-SpecMode's
  Glossiness 0.3 sphere (r 0.49) 0.53 → 0.86 × Iray (#52). A specular-
  mode IOR is at least 1.1: Iray's curve still rises to 1 at grazing over
  a black Glossy Specular (that sphere 0.91 → 1.00).
- **Anisotropy** (Glossy Anisotropy + Glossy Anisotropy Rotations on the
  gloss and the metal, Top Coat Anisotropy + Top Coat Rotations on the top
  coat; #23). Iray (`base::anisotropy_conversion`, MDL SDK's `base.mdl`):
  roughness_u = r, roughness_v = r / (1 − a) (r / 0.0001 at a = 1), the
  tangent turned by 2π · rotation. Octane: each axis on the GGX curve above
  (v capped at 1), **`roughness` = v** (the long axis),
  **`anisotropyTexture` = (v − u) / (v + u)** (1 stretches along the
  texture's v, like Iray), **`rotation` = 1 − Iray's rotation** (both in
  turns; Octane's slider 0 – 1, it turns the other way). A roughness map
  bakes v per texel. Where Iray's long axis runs past 1 (r / (1 − a) > 1)
  the lobe is **plain GGX** (`brdf` 2), not the energy-preserving one.
  Why: at the axes' geometric mean Calib-Aniso's streaks came out short
  and bright — highlight elongation (major / minor axis of the highlight)
  1.96 against Iray's 9.9 on roughness 0.1, anisotropy 1, mean 1.52 ×
  Iray; at v 10.3 and 1.16 ×, its other spheres 1.07 – 1.10 × (the
  isotropic one 1.11). Calib-Aniso2's K1 (rotation 0.125) and K2 (0.375)
  mirror each other: exported as Iray's, each looked like Iray's other
  one. Energy-preserving GGX at roughness 1 adds light Iray's flat lobe
  doesn't have: the sharp anisotropic metal K3 (r 0.04, a 1) 1.52 → 1.08
  × Iray, the weighted coat K4 1.31 → 0.96; below the cap plain GGX
  darkened Calib-Aniso's metal at v 0.8 (1.12 → 0.86).
  The **Top Coat** in Layering Modes 0 / 2 / 3 (Universal's `coating` has
  no anisotropy): an anisotropic one makes the material a **Layered
  material** (143, `pin_count` 1) of the surface without its coat
  (`material1`) and a **Specular Layer** (139, dynamic pin `Layer 1`,
  `dynamicType` 33) with the coating's colour × weight (`specular`), its
  roughness, IOR (`top_coat_ior`) and the anisotropy and rotation as
  above. Why: as an isotropic coating Calib-Aniso's A7 (mode 0, a 1) was
  1.22 × Iray with a sharp round highlight where Iray draws a streak,
  Calib-Aniso2's rotated K5 1.21; as a layer 1.16 / 1.08 with Iray's
  streaks. Modes 2 / 3 differ from 0 only in the IOR (no reference of
  them anisotropic). The weighted coat (mode 1) keeps its own metal node.
  **On strand hair** (an Uber material on `BasisCurves`) the rotation
  turns a quarter more, 1 − (rotation + 0.25), gloss and coat alike:
  DAZ hands Iray its strands as thin tubes whose UV u runs around the
  strand (its .mi export of Calib-Hair: a strand point's vertices step u
  by 1/6, v grows along the strand), so Iray's unrotated tangent lies
  across it, and Octane's curves take theirs along it. Why: Morgan Hair
  (Rotations 0.25, anisotropy 0.74 / 0.92) streaked across its strands as
  exported; Iray streaks along them. Turned, its highlights 0.69 – 0.77 →
  1.07 – 1.13 × Iray, the crown 2.98 → 2.24 (#52).
  Backscattering Anisotropy reaches Iray only from a channel saved
  shown; DAZ's default material (our generated scenes) hides it, so
  Calib-Backscatter B8 and Calib-Aniso2 K6 stayed isotropic
  (Calib-Mi-Visible2-Shown, see iray-uber-mdl.md, Channel visibility). The
  Octane side keeps the backscatter isotropic either way.
- **Top Coat** → Universal `coating` = Top Coat Color × weight,
  `coatingRoughness` (Top Coat Roughness, or 1 − Top Coat Glossiness in
  mode 1 — verified; mode 2 not yet, it keeps Glossiness — squared
  likewise), `coatingIor`: the top-coated sphere's
  highlight 0 → 19 px (Iray 17). `coatingIor` by Top Coat Layering Mode:
  0 Reflectivity (the cornea; 2128 materials in My Library) reflects F0 =
  0.08 × Reflectivity head-on (0.5 → 0.04, IOR 1.5) — Calib-TopCoat's
  Reflectivity 1 sphere 0.75 → 1.09 × Iray (0.5: 1.11); Fresnel and Custom
  Fresnel (2) always IOR 1.5, Custom Curve (3) IOR 1.07, whatever Top Coat
  IOR, the Curve Normal / Grazing / Exponent and Top Coat Specular say: DAZ
  Studio doesn't pass those channels to the MDL at all (its .mi export,
  `docs/daz/iray-uber-mdl.md`), so Iray's Fresnel coat is at the MDL's
  1.5 and its curve at the defaults, 0 head-on to 1 at grazing. Why:
  Calib-TopCoat's Iray spheres at IOR 1.5 and 2 render alike; the IOR 2
  sphere 2.06 → 1.11 × Iray, the curve spheres 2.5 → 1.23 (1.22, the IOR
  matching the curve's hemispherical average: 1.52; IOR 1, no coat: 1.18
  — what's left isn't the coat). 5561 materials in the installed library
  use Fresnel, 196 Custom Curve (#25).
- **Weighted top coat** (Top Coat Layering Mode 1; 2176 materials): DAZ's
  MDL lays the coat's glossy reflection over the rest by `weighted_layer`
  — the weight, no Fresnel. → a Mix material (19): `amount` = Top Coat
  Weight (or its map), `material1` a Universal metal in Top Coat Color at
  the top coat's roughness (`weighted_coat_material`), `material2` the
  surface without its coat; the mix carries the displacement. Calib-TopCoat
  weighted 1 / 0.4: 1.03 – 1.05 × Iray (as a coating 0.05 – 0.10; Iray's
  weight-1 sphere is a chrome mirror).
- **Top Coat Bump** (#61) → the coat's own surface detail: Universal
  `coatingBump` / `coatingNormal`, the weighted coat's metal and the
  anisotropic coat's Specular Layer on their `bump` / `normal`
  (`coat_bump_pins`). DAZ hands Iray Top Coat Bump as `top_coat_bump`:
  Mode 0 (Height Map) a `file_bump_texture`, Mode 1 (Normal Map) a
  `tangent_space_normal_texture`, the channel's value as the factor
  (Calib-CoatBump's .mi, `src/iray.rs`); the base's `base_bump` likewise
  from Bump Strength or Normal Map. The MDL's coat normal
  (`calc_mix_layer_normal`) is the coat's bump blended with the base's,
  and **the base's alone when the coat has none** — Octane's coating
  instead takes the unbumped shading normal unless its pins are set (the
  manual; Calib-CoatBump B2 / B6: the base's ridges stayed out of the
  coat's highlights). So the coat pins get the coat's height map, else
  the base's (Bump Strength's); with both, their sum — a Mix texture
  (38) at amount ½ of the two at twice their power (Iray's B7 shows the
  coat's dimples and the base's ridges at once; with the coat's alone,
  Octane's B7 was nearly smooth: RMS vs Iray 7.2 → 2.1). The normal pin
  gets the coat's normal map (Mode 1), else the base's Normal Map.
- **Bump strength** (#61, #75): every Uber height map — the base's Bump
  Strength map on `bump` (and on the coat), the Top Coat Bump — goes in
  at Octane image `power` = Iray's `file_bump_texture` factor (Bump
  Strength, Top Coat Bump; none at 0) × `BUMP_POWER` 2 × 512 / the map's
  width × the material's UV scale / 0.542 m / the UV tiles
  (`bump_power`; width and tiles as geometric means of both axes, the
  map's size as the scene has it, before the texture budget; UV scale =
  √(surface area / UV area) of the material's faces, `usd::uv_scales`,
  else 0.542). Why each term:
  - Iray: MDL's reference `base::file_bump_texture` (MDL SDK `base.mdl`)
    takes the slope as the difference of the texels one either side and
    applies it on unit tangents — the tilt goes with factor / the map's
    width, and neither UV tiling (`transform_coordinate` keeps the
    tangents unit) nor the object's size changes it.
  - Octane: the tilt goes with `power` × the height's slope per meter of
    surface, whatever the map's resolution. Calib-BumpRes, Info kernel
    shading normals with and without bump (`octane-probe-bumptilt.lua`,
    `bump-tilt.py`), RMS tilt at the same power: 256 / 1024 / 4096 px
    8.9° / 8.4° / 8.4°; tiles 2 and the sphere at half size 13.1° / 13.2°,
    as twice the power (13.7°).
  - The constant: Calib-CoatBump (512 px maps, untiled, spheres of UV
    scale 0.542 m) against Iray (5000 samples), RMS of the grey image
    (1 px blur) per sphere, power × 1 / 2 / 3 / 4: base ridges 7.7 /
    **2.9** / 8.1 / 10.8, coat dimples at 1 7.2 / **5.4** / 6.1 / 6.8, at
    3 14.7 / **9.2** / 12.1 / 13.2. Before, the base's map went in at
    power 1 whatever its Bump Strength, the coat's at Top Coat Bump: both
    rendered about half as bumpy as Iray.
  So a DAZ figure's 4096 px skin bump goes in at 1/8 of the 512 px fit.
  - Confirmed against Iray (Calib-BumpRes, 5000 samples): Octane renders
    at every power × 0 / 0.5 / 0.71 / 1 / 1.41 / 2, each sphere's
    band-passed pixels against Iray's (`bump-ladder-fit.py`), fit at × 1
    for all eight (256 / 1024 / 4096 px, factors 1 / 4 / 16, tiles 2 / 4,
    half size; R2 / R4 between 0.71 and 1, T2 between 1 and 1.41). Iray's
    native bump follows MDL's reference, and the constant holds. Normal
  maps stay at power 1 (B5, B6, B8 match Iray at Normal Map 1).
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
- **Metallic Weight w → `metallic` w² / k, albedo and specular × k,**
  k = 1 − w + w² (a map: squared by twice its gamma, k from the
  channel's value); the glossy excess coating × (1 − w). Iray's metal is a
  coat at weight w inside a layer at weight w (DAZ passes w to both, its
  .mi export): w² metal + (1 − w) base. Universal's metal takes the
  albedo's colour, so the scale has to be shared.

  Why: Calib-Map-Metal's red spheres at 0.25 / 0.5 / 0.75 went from 1.23
  / 1.38 / 1.39 to 1.12 / 1.16 / 1.22 × Iray (the full metal 1.19), the
  0.5 metal without gloss 1.51 → 1.06, a Reflectivity 1 metal 2.14 →
  1.28; Calib-DataGamma's metal 0.5 1.41 → 1.09; Calib-PBRSkin's 0.75 /
  0.9 metals 1.24 / 1.01 → 1.02 / 0.93. The metallic map at gamma 1
  (G5) 1.38 → 1.43 (#59).
- **Metallic Weight under 0.05** (no map) is left out: the GSOV4 jacket's
  0.0107 turned Octane's gloss off — black velvet (0.02× Iray) beside the
  same leather's trousers.
- Base Mixing 1 ignores Metallic Weight; a black base under a strong white
  gloss becomes a gloss-coloured metal (mirrors).
- **Base Mixing 1 (specular / glossiness): Glossy Specular is F0.** Iray
  reflects Glossy Specular head-on (a colour, per channel), with Fresnel to
  white at grazing, and the diffuse takes what is left. The export uses the
  dielectric layer's `ior` = (1 + √F0) / (1 − √F0), F0 Glossy Specular's
  strongest channel; the refraction's IOR wins when Refraction Weight ≥
  0.5. `specular` = Glossy Color × Glossy Specular's hue (the colour over
  that channel, so at most 1) × the layer's weight. A material without the channel takes
  F0 0.04 (IOR 1.5), and DAZ's default 0.2314 is that too. Black reflects
  nothing head-on (IOR 1).

  Why: on Calib-SpecMode, Glossy Specular 0 / 0.2 / 0.5 / 1, 0.5 at weight
  0.5, and Morgan Hair's gloss without and with its top coat rendered 1.26 /
  1.13 / 0.74 / 0.35 / 0.84 / 0.55 / 0.55 × Iray at the default IOR. With
  F0 as the IOR they render 1.01 – 1.10 (S1, mode 0: 1.08). At 1, Iray's
  sphere is a mirror. 289 materials in 41 of the user's scenes are in this
  mode, mostly hair and brows (#52). The hue over the luminance went past
  1 and was clipped: Calib-ColorEffect's C8 (Glossy Specular 0.9 / 0.3 /
  0.2 over a blue base) rendered a white reflection where Iray's is a red
  mirror (R 0.35 → 1.03 × Iray). Its green and blue stayed 2.6 / 4.4 ×:
  Iray leaves the diffuse 1 − w·F0 in every channel (DAZ passes F0 as the
  curve and the hue only as a directional tint on the reflection, its .mi
  export), Octane 1 − w·F0·t per channel of the specular tint t. So the
  albedo is × (1 − w·F0) / (1 − w·F0·t): C8 1.77 → 1.08 × Iray (R / G / B
  1.05 / 0.82 / 0.94), Morgan's beige gloss 1.09 → 1.03; a grey gloss
  keeps 1 (#27). 506 installed materials have Base Mixing 1 with a Glossy
  Specular above 0.06 or a map.
- **Color Effect (Glossy, Top Coat, Diffuse Overlay) 1 / 2** → the
  layer's colour as its hue (over the strongest channel) and its weight ×
  the colour's average (1) or its strongest channel (2: greyed at its own
  intensity first), as DAZ's MDL (`transmit_color_calc`) does; 0 leaves
  both. So effect 2 reflects tint × weight as effect 0 would; it differs
  only on a translucent diffuse (above). Calib-CoatEffect's effect 2
  spheres 1.08 – 1.14 × Iray, like effect 0's 1.05 – 1.16; Calib-Map-Effect
  E4 (red gloss, effect 2) 1.09 0.73 2.69 → 1.12 0.76 2.65 (#52). Weighted coats and weighted-mix gloss (Mix materials with a
  metal) get `edgeTint` = their colour: Iray's weighted layer has no
  Fresnel to white. **Diffuse Overlay** → flat colours over a flat base:
  albedo (1 − w) × base + w × overlay, w squared with "Weight Squared".
  With a Weight or Color map, or over a diffuse map: an albedo Mix
  texture (38), amount = w (the weight map × w, which DAZ squares as a
  whole with "Weight Squared": the map at twice its gamma × w²),
  texture1 = the base, texture2 = the overlay colour (× its map) × its
  Oren-Nayar flat term. Not under OOT's Overlay Alpha Mask (that one is
  the hair's own blend).

  Why: Calib-Map-Effect's blue overlay spheres had B 0.47 × Iray, now
  1.12 / 1.08; its red gloss with effect 1 / 2 B 1.75 / 1.67 → 1.43 /
  1.24; Calib-CoatEffect's red gloss with effect R 1.19 → 1.05;
  Calib-Map-TopCoat's red weighted coat B 4.29 → 1.81, its Fresnel coat
  with effect 1 1.09 / 1.09 / 1.16. Maps: DAZ's .mi export of
  Calib-Overlay passes the weight map as `square_float(map × w)` and the
  colour map with the linear colour as its `color_scale` (#71). Against
  Iray's render of Calib-Overlay (enhanced-final, sphere discs, Y): weight
  map 1.04, squared 1.04, × 0.5 squared 1.05, colour map 1.03 (squared
  1.03), both 1.04, tinted 1.05 — as Calib-Diffuse's smooth sphere (1.08).
  The squared map must be read at 4.4 in the legacy-gamma colour space
  (Textures, colour space): in sRGB it rendered unsquared, the 0.5 / 0.25
  belts G 4.5 / 14 × Iray. With Overlay Roughness 1 the overlay reads
  1.05 (0.85 under the classic A term, below).
- **Diffuse Roughness** σ (Iray's diffuse is Oren-Nayar at σ, DAZ's MDL;
  the .mi export hands σ over as is) → where Universal has no gloss,
  metal, refraction or transmission on its one `roughness` pin (Glossy
  Layered Weight 0, no dual lobe, or the diffuse part of a weighted mix):
  `diffuseBrdf` 2 (Oren-Nayar) and `roughness` σ. Elsewhere Lambertian,
  the albedo × F(σ), the head-on albedo of Iray's Oren-Nayar:
  F(σ) = π A = 1 / (1 + (1/2 − 2 / (3π)) σ) (0.874 at 0.5, 0.777 at 1).
  **Diffuse Overlay Roughness** σ₀: the overlay's share × F(σ₀); under
  Octane's Oren-Nayar, which stands in for Iray's at σ, × F(σ₀) / F(σ)
  (an overlay at the base's roughness mixes in as is). Maps of either
  are left out; the skin path (`standard_surface`) keeps its fitted
  albedo. **Diffuse Strength** scales the base colour (as DAZ does
  before the MDL).

  Why F: `diffuse_reflection_bsdf` is Fujii's improved qualitative
  Oren-Nayar since MDL 1.10 / Iray 2024.1 (the MDL spec and release
  notes; DAZ Studio 6 ships Iray 2025.0), A = 1 / (π + (π/2 − 2/3) σ),
  B = σ A, with σ = Diffuse Roughness as is (DAZ's MDL, the .mi export).
  Seen head-on (n = v) its B term vanishes under any light, so π A is
  what the lobe keeps of a Lambertian one there; elsewhere B shifts light
  toward or away from the light, which a flat factor can't follow. On
  Calib-Diffuse Iray's σ₀ 1 overlay kept 0.77 of the Lambertian at the
  sphere's centre (F 0.777), 0.74 / 0.70 on the middle 60 % / 90 % disc
  (the HDRI lights the spheres from behind: B darkens their rims). The
  classic A term 1 − 0.5 σ² / (σ² + 0.33) (0.624 at 1) left the overlay
  0.85 – 0.89 × Iray (#76). (#32 read Iray's σ 0.5 / 1 spheres at 0.78 /
  0.62 of the smooth one's as A; that ratio crosses sphere positions,
  where Octane's identical Lambertian spheres differ by 0.86.)

  Numbers (enhanced-final, luminance Octane / Iray; the smooth spheres
  1.04 – 1.08): Calib-Diffuse's white overlay at σ₀ 1, centre / middle
  60 % / 90 % disc, 0.85 / 0.88 / 0.93 → 1.05 / 1.09 / 1.14 (Lambertian
  1.40, #32); Calib-Overlay's V8 (both maps, σ₀ 1) overlay pixels 0.85 →
  1.05, blue cells B 0.82 → 1.03, yellow R / G 0.88 / 0.83 → 1.07 / 1.04
  — as its σ₀ 0 twin V6 (1.01, 1.05 / 1.03). The base at σ on Lambert × F
  (the gloss path, tried on the matte spheres): σ 0.5 / 1 1.08 / 1.10 at
  the centre, 1.10 / 1.15 on the middle disc (× the classic A: 0.99 /
  0.92). Octane's Oren-Nayar at σ 0.5 / 1: 0.98 / 0.89 at the centre,
  1.02 / 0.96 on the middle disc — the classic qualitative model (head-on
  ~0.80 / 0.65 of its Lambertian), darker head-on than Iray's, but its
  shape follows Iray's better (pixel correlation 0.979 vs 0.971 at σ 1),
  so it stays where the pin is free (enum 0 / 1 render Lambertian, 3
  brighter: 1.10 / 1.12; at roughness 0 Oren-Nayar is Lambert — the pin
  is shared). Strength 0.5 1.09, overlay squared 1.04, overlay 1 blue
  0.97 (#32).
- **Metallic flakes** → a Mix material: a metal in the flakes' colour
  (`edgeTint` the same, roughness 0.5: the flakes' scattered normals) at
  Flakes Weight × 0.4 × density^1.3 over the surface; the top coat lies
  over both. Size, strength, the flakes' roughness and thin film aren't
  mapped.

  Why: Iray's flakes (DAZ's MDL: a weighted glossy layer at weight × a
  flake noise, each flake with its own normal) take Calib-Flakes' spheres
  from 0.04 to 0.20; missing, Octane rendered them 0.12 – 0.36 × Iray, now
  0.88 – 1.12 (coverage 0.5: 1.05 – 1.35, 0.3: 0.72 – 0.88); Calib-Map-
  Misc's gold flakes 0.50 → ~1 (#30).
- **Backscattering** (Base Mixing 0, not refractive) → a Mix material
  at amount = Backscattering Weight: the surface and a retro-reflector —
  a metal in Glossy Color (or Backscattering Color with Share Glossy
  Inputs off) × (2 + r²) / (2 + 2 r²), Beckmann (brdf 1) at roughness
  √(r / √(4 + r²)) for Iray's (squared) roughness r, whose `normal` is an
  OSL texture giving the direction to the viewer in tangent space (T =
  dP/du made perpendicular to N, B = N × T: the frame Octane reads normal
  maps in). A mirror about that normal sends every ray back where it came
  from; the top coat lies over both.

  Why: MDL's `backscattering_glossy_reflection_bsdf` is the
  retro-reflective part of a halfway-vector disk model, exponent 2 / r²,
  no Fresnel, `1 / max(cos θi, cos θo)` for `1 / cos θi`, and DAZ leaves
  its `multiscatter_tint` at 0; DAZ layers it with `weighted_layer`
  (base × (1 − w), linear — Octane's Mix). Octane X has no
  retro-reflective lobe (the manual's Retro-Reflective option isn't in
  this version; Ray Direction's own "tangent" space is another frame).
  Integrating Calib-Backscatter's HDRI over a lobe of exponent n around
  the retro direction (with the max-cos term) reproduces Iray's spheres
  at n ≈ 125 – 235 for r 0.09 and ≈ 10 for r 0.49, × 1.00 – 1.02 —
  n = 2 / r² (247 / 8.3) — and Iray's weight-0.25 / 0.5 spheres as that
  lobe × the weight. Octane's mirror reflects about a Beckmann half
  vector (α = roughness²) and turns the ray twice as far: n = (2 / α² −
  2) / 4, fitted the same way on Octane renders (roughness 0.2 / 0.3 /
  0.4 → n ≈ 300 / 60 / 19), so α = r / √(4 + r²). The tint factor is
  the max-cos term averaged over the lobe (its mean cosine, (n + 1) /
  (n + 2)); Octane's lobe has none (its renders match the integrated
  lobe at unit albedo, × 1.00 – 1.05). The #29 stand-in, a dimmed metal
  mirror, showed the environment around: Calib-Aniso2's K6 0.56 × Iray,
  Calib-Backscatter 0.50 – 1.18. Now (Calib-Backscatter none / w0.25 /
  w0.5 / w1 / w1 r 0.49 / w1 ×3) 1.08 / 1.11 / 1.15 / 1.19 / 1.31 /
  1.07 – 1.13, K6 1.12, Calib-Map-Misc X1 / X2 (w0.5; X2 unshared, r 0)
  1.04 / 1.11 and X2 blue as in Iray (#73). A perfect retro-reflector
  (roughness 0) renders exactly what the HDRI holds behind the camera
  (sphere-to-sphere 1.60 / 1.26 / 1.10, predicted 1.60 / 1.26 / 1.09);
  Iray's spheres are flat because its lobe is wide, not because the
  environment is. Mean linear luminance in a 40 px disc per sphere.
  Limits: Beckmann at α 0.24 spreads wider than the small-angle relation
  (n ≈ 5 against 8.3), the rough sphere's 1.31; faint per-face stripes on
  3× crops (OSL's dP/du is per face).
- **Scattering in a refractive volume** (SSS Amount / SSS Color with
  refraction, not thin-walled) → a scattering medium (59) with Iray's σa
  and σs (as DAZ passes them: −ln(SSS Color) / d chromatic, SSS Amount /
  d mono, d in m) × 0.3 for σs; the phase stays isotropic. Why: missing,
  Calib-Map-Misc's milky sphere rendered 0.67 × Iray, at the full σs
  1.47, × 0.3 ~1.0 — one sphere's fit (#28). Translucent props keep
  `volume_factor`; SSS Reflectance Tint isn't passed to Iray by DAZ at
  all (the .mi export), so it has nothing to map.
- **Refraction** (glass, water) → `transmission` = Refraction Weight ×
  the refraction's colour, IOR on `ior` (Dielectric IOR — Universal has no
  `index4`, probe 82; `ior` 2.0 brightened Calib-Materials' gloss 1.08 →
  1.15), transparent shadows. Iray's refraction takes the glossy layer's
  colour and roughness while **Share Glossy Inputs** is on (the default),
  its own Refraction Color / Roughness (Glossiness in Base Mixing 1 / 2)
  when off: shared, a green Refraction Color and roughness 0.3 changed
  nothing in Iray. A mostly refractive surface (weight ≥ 0.5) with its own
  roughness gets it on Universal's one `roughness`, squared as usual.
  **Rough glass** (refraction ≥ 0.5, not thin-walled; shared or own
  roughness) → `brdf` 1 (Beckmann) at roughness √r, r = Iray's roughness
  after its squaring. Why (DAZ's MDL, #72): Uber refracts through
  `simple_glossy_bsdf` — Phong of exponent 2 / r², v-cavities shadowing —
  with no multiple-scattering compensation (`multiscatter_tint` 0, so its
  rough lobes lose light, MDL spec 21.1.3). Phong of exponent n is
  Beckmann at α = √(2 / (n + 2)) ≈ r, and Octane's roughness is √α: no
  fitted constant. GGX energy preserving (the gloss rule above) gives
  back the light Iray's lobe loses: Calib-Dispersion-Rough's spheres at
  r 0.36 1.19 (shared) / 1.14 (own, then at Octane's default BSDF × 0.3,
  a Calib-Glass R6 fit, too sharp) × Iray; plain GGX loses more than Iray
  (0.86 / 0.85, its long tail); Beckmann at √r 1.09 / 1.07 with Iray's
  blur (detail 1.4 / 1.6 vs Iray's 1.2 / 1.5; at r 0.09 8.3 / 5.4 vs
  8.3 / 5.4; Calib-Glass R6 6.4 = Iray, was 6.7), the clear spheres
  1.02 – 1.04 as before. What is left at 0.36 (~5 % over the scene's
  1.03) is Smith vs v-cavities shadowing; Octane has no v-cavities.
  **Thin Walled** → `transmissionType` 2 (Thin wall), no volume. Iray shows
  a thin-walled glass sphere without any reflection — invisible; Octane's
  thin wall keeps its Fresnel reflection, kept as the more plausible.
  **Dispersion** (`dispersion_enable`: refraction and Abbe > 0, the .mi
  export) → `dispersion_coefficient_B` = Iray's Abbe number as is, in
  Universal's default dispersion mode 1 (Abbe number), not on thin walls.
  The pin is a plain float: a float value node (6, float3), not a texture
  (31) — as one Octane dropped it on import (probe 112). With dispersion
  Iray's transmission is perfectly smooth whatever the roughness (DAZ's MDL,
  `refraction_base`): a mostly refractive (≥ 0.5), dispersive body gets
  `roughness` 0. Calib-Dispersion-Rough (Iray, 5000 samples) confirms it:
  every Abbe 20 sphere renders clear in Iray as in Octane at glossy
  roughness 0.3 / 0.6 and refraction roughness 0.3 / 0.6 (detail 8.0 –
  14.4 vs Octane 8.4 – 14.5, brightness 1.01 – 1.03 × Iray); the same
  spheres without Abbe are frosted at 0.6 in both.
  Calib-Dispersion (8 spheres, Abbe 0 – 60, IOR 1.33 – 2.4): brightness
  1.02 – 1.04 × Iray; Octane's spectral dispersion shows ~2× the colour
  fringes of Iray's (RGB: spectral rendering off) at Abbe 10 – 30 — kept
  (the number's physical meaning; question in #33).
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
- **UV tiles and offsets** (Calib-Map-Offset, #65): a material's maps get
  one Octane texture transform: scale `1 / tiles`, translation
  `(−offset_u / tiles_u, offset_v / tiles_v)`. The offset is in tile units
  (hence the division), and Octane's U runs against Iray's, while the V sign
  already matches `iray.rs`' negated vertical offset. Measured on six
  spheres (V +0.125, V −0.375, U, tiles 2 × 2 and 3 × 2 with offsets),
  correlation with the Iray render 0.87–0.98; raw offsets matched only the
  untiled V ones, `offset · tiles` none of the tiled ones.
- **Per-map image modifications** (`image_modification` on a channel,
  #77; DAZ's .mi of Calib-Mi-ImageMod and -ImageMod2, fixtures in
  `calib/references/mi`). DAZ hands each map's `file_texture`:
  - uvw = uv · (material tiles · map tiles) + (material offset_u · map
    tiles_u + map tiling offset_u, −(material offset_v · map tiles_v + map
    tiling offset_v)) — the map's tiles multiply the material's, its own
    offset adds in its tile units. Each Octane image node gets its own
    transform from that by the rule above: scale 1 / the product,
    translation (−T_u / S_u, −T_v / S_v) of Iray's translation T and scale
    S (`uv_transform`).
  - The value = the channel's value × (scale · t + offset), t the texel or
    1 − t with Invert (`color_scale`, `color_offset`); e.g. Glossy
    Roughness 0.7, invert, scale 1.5, offset −0.1 → 0.98 − 1.05 t. In
    Octane a Mix texture (38) with the image as amount between value ×
    lo and value × hi (t = 0 / 1); baked maps (roughness) take it in the
    bake.
  - Grey mode: average (`mono_average`, the default); red / green / blue
    channel zero the other channels' scale and offset and read the
    maximum; maximum (`mono_maximum`); alpha (`mono_alpha`, 1 without
    alpha). Octane reads a one-channel copy of the image (the bytes kept,
    so the image node decodes it as before; baked maps pick the channel in
    the bake).
  - Bump, normal and Top Coat Bump maps take the placement only: DAZ
    passes no scale, offset or invert for them (I7's inverted, scaled bump
    reached Iray as the plain factor). Their `bump_power` counts the map's
    tiles with the material's.
  How common (the user's 476 scenes, 110 with `image_modification`):
  invert in 80 scenes (Glossy Roughness 58, Top Coat Roughness 42 — mostly
  glossiness maps on a roughness channel), per-map tiles in 38, scale 29,
  grey mode 18 (overlay and make-up weights by alpha), tiling offsets 13
  (Calib-Skin's normal maps at 1: a whole period), offset 2.
- **Colour space.** Colour maps are sRGB; bump and normal maps are
  linear. A map read at any other gamma (an image's own `map_gamma`, a
  squared overlay weight map at 4.4) goes in "linear sRGB + legacy gamma"
  (6) with that gamma: Octane applies the `gamma` pin only there and
  ignores it in sRGB (Calib-Overlay's squared belts, #71). Every image
  node gets `initialColorSpaceSelected` = 1, and the HDRI stays linear. Why: without the attribute, data maps were decoded as
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
  Mixing 2 (`DzMdlIrayUberWeighted`); its terminal MDL is DAZ's own Uber
  stack (`add_top_coat` → `add_uber_volume` → `add_geometry` →
  `normalized_base`, DAZ's .mi of Calib-OOT), so the Uber path exports it.
  Its diffuse, translucency, glossy and top-coat colours are NVIDIA's
  `blend_colors(colour map, … Overlay Color map, normal, weight)`, and the
  weight DAZ passes is **the value of `Overlay Alpha Mask`**, not its map
  (`pass_float(0)` in the .mi; the alpha map is nowhere in it). The
  presets' black value turns the overlay off: the colour maps alone.
  Another value blends the overlay at that constant weight (the mean of
  the linear colour — DAZ's colour → float reduction is a guess)
  (`overlaid_pin`; the top coat's overlay is not exported). Calib-OOT (Rose Hair, local: store content) against
  its Iray render: blended by the alpha map the hair came out dark-rooted,
  crown 0.81 / fringe 0.64 × Iray (r/g 1.02 / 1.20 v 1.05 / 1.23); with
  the overlay off 0.96 / 0.79 (1.09 / 1.27), side 0.92. The rest (Iray's
  paler, softer strands) is the per-map tiling Iray gets — the diffuse
  map at 0.6 tiles on u, the gloss weight at 1.2 (`image_modification`,
  exported since [#77](https://github.com/vysokyj/orbis/issues/77)) — and the glints.
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
    albedo and edge tint = Metal Color (`diffuse`) × (offset + scale ·
    reflection map) clamped to 0 – 1 (`weighted_layer` clamps its weight;
    Cast Iron's 0.5 + 3 · map is mostly 1; the tinted `simple_glossy_bsdf`
    has no Fresnel). The MDL roughness is what the Uber's is after its
    squaring (the same Phong lobe), so each axis takes the Uber gloss's
    GGX curve: Octane's roughness = the longer axis per texel, anisotropy
    = (v − u) / (v + u) **per texel** (a derived map when one map drives
    both axes, as in both presets; else at a mid-grey texel), **plain GGX**
    (`brdf` 2), the bump at the Uber's bump power (`bump_power`: × 2 per
    unit of amount for a 512 px map, untiled), the shader's tiles
    (`mdl_metal`). Why: Metal Brushed's map runs u 0.04 – 0.18 against v
    0.36 – 0.48; its darkest texels draw Iray's thin vertical streak and
    brush bands, which a mid-grey ratio blurred into one broad highlight.
    Plain GGX because `metal_generic`'s lobe has no multiple-scattering
    compensation: at its broad long axis (0.6 – 0.7 on the curve) the
    energy-preserving GGX rendered it 1.16 × Iray, plain GGX 1.02 (the
    scene's floor 1.04) — measured, the MDL doesn't settle it. Cast Iron
    1.11 (centre 0.97) either way; its sparkle at 256 samples is noise of
    the 2048 px bump, gone at 1024.
  - Dielectrics (concrete, concrete polished, asphalt, ceramic tiles small,
    granite black, porcelain, resin polyurethane matte, architectural):
    `df::fresnel_layer(ior, weight, glossy, diffuse)` → Universal albedo,
    specular = the weight (a map where the MDL weights or tints the gloss
    with one: concrete's bump map tints it, asphalt's and the tiles' bump
    map weights it, polished concrete's colour map does — its diffuse is
    that map × the weight too), `ior` = the layer's IOR, the MDL roughness
    on the Uber gloss's GGX curve under energy-preserving GGX (`brdf` 6),
    bump × the Uber's bump power (`mdl_dielectric`). Granite's varnish is a
    mirror (roughness 0). Resin's base is a tinted glossy reflect +
    transmit (`simple_glossy_bsdf`, `scatter_reflect_transmit`): specular
    and transmission its colour, which Octane, like Iray, applies at every
    crossing — that keeps the hue — under the rough-glass rule (Beckmann at
    √r; the clear gloss as the coating at √r), both × the lobe's albedo
    **1 − 0.64 r** (`resin_lobe_albedo`). Why: Iray's lobe has no
    multiple-scattering compensation (MDL spec 21.1.3) and loses light
    with roughness. On Calib-NVIDIA2, at r 0.02 the full colour matched
    Iray (A8 0.91 ×, g/r 0.14 v 0.12; the earlier fitted × 0.8 gave 0.71
    and g/r 0.19). At r 0.25 the full colour was 1.23 – 1.46 × Iray under
    every BSDF tried (default, plain or energy-preserving GGX on the curve,
    Beckmann at √r); the albedo 0.84 brings the grey resin (A7) to 1.02 ×
    and Calib-NVIDIA's red one to 1.05 × with g/r 0.25 v 0.22 (floors
    1.04 – 1.05). The two roughnesses are measured; the line between them
    is assumed.
    Architectural (mental ray's mia_material): diffuse × weight,
    reflectivity × the reflection colour's maximum tinted by it, without
    "Use Fresnel Term" Schlick's curve from the 0° reflectivity as the IOR
    with that F0 (0.2 → 2.62), metal, transparency and cutout opacity;
    glossiness g → MDL roughness r by `base::architectural_gloss_to_rough`
    (MDL SDK `base.mdl`: √(2 · 2^(−4 − 14 g)), a mirror at g = 1, 0.35 at
    0) — what DAZ's .mi hands Iray; Iray's `base::` is compiled into
    libneuray, no source to read. **Iray renders the reflection at √(r ·
    r₀)** (`arch_reflection_rough`; r₀ = 0.35, the value at g = 0): the
    formula with half its slope, √(2 · 2^(−4 − 7 g)). Why: Calib-NVIDIA3's
    glossiness 0.15 / 0.3 / 0.5 / 0.7 matched Octane (sweep of the GGX
    pin, error of the disk's luminance pattern) at r 0.27 / 0.19 / 0.12 /
    ~0.07 — the formula gives 0.17 / 0.083 / 0.031 / 0.012, √(r · r₀)
    0.25 / 0.17 / 0.105 / 0.065. Anisotropy in mia's semantic
    (`anisotropy_conversion` with `mia_anisotropy_semantic`) on the MDL
    roughness — above 1 u = r · √a, v = r / √a, below v = r / a^¼, u = r ·
    a^¼ — then √(· r₀) and the GGX curve **per axis**, Octane's roughness
    the longer, `anisotropyTexture` (v − u) / (v + u) (negative: stretched
    along u), `rotation` 1 − Iray's. Applying √(· r₀) before the
    anisotropy stretched Calib-NVIDIA2's anisotropy-4 sphere far beyond
    Iray's (centre 1.69 ×); per axis 1.06 with Iray's look.
    Transparency t (`weighted_layer(t, transmit lobe, diffuse)`): diffuse
    × (1 − t), `transmission` = refr_color × t × **(1 − 0.36 r_t)**
    (`arch_glass_albedo`) through a transmit-only lobe — `specular` 0
    (Octane still refracts) — at **r_t = 3.7 × architectural_gloss_to_rough
    (refr_gloss)**, at most 1 (`arch_glass_rough`), and `ior` = Index of
    Refraction (refr_ior), under the rough-glass rule (Beckmann at √r);
    the reflection on the coating at √r of its own roughness and the
    reflection curve's IOR. Why: refr_gloss 0.6 and 0.2 (MDL 0.020 /
    0.134) matched Octane's Beckmann at r 0.06 – 0.09 and ~0.5; the
    reflection's curve would give 0.08 and 0.22, and at 0.2 Iray's
    interior is an even frosted grey that only r ~0.5 draws. The glass
    lobe also loses light with roughness, like resin's (Iray's lobe has
    no multiple-scattering compensation): at r_t 0.5 the full refr_color
    was 1.53 × Iray, 1 − 0.36 r_t brings it to 1.02. Both glass factors
    are measured at two refr_gloss values.
    **Enable Absorption** (`refr_falloff_on`, with transparency) → an
    absorption medium on the glass, σa = −ln(c) / d per metre
    (`arch_absorption`), as `nvidia/architectural.mdl` computes its
    `absorption_coefficient`: c the Absorption Color (linear) with
    Absorption Use Color on, else 0.3333; d the Absorption Distance (cm,
    in metres); d ≤ 0 lets nothing through; none when thin-walled. DAZ
    passes all four (Calib-Mi-Arch, the test checks σa); Octane darkened
    the 10 cm sphere and blacked out the 2 – 3 cm ones (sanity render, no
    Iray reference needed: the MDL states the medium).
    Calib-NVIDIA3 (Iray 5000, Octane 1024; floor 1.04), disk before →
    after: glossiness 0.15 0.97 → 1.05, 0.3 0.89 → 1.01, 0.5 1.01 → 1.05,
    0.7 1.04 → 1.05; glass refr_gloss 1 1.04, 0.6 1.06 → 1.01, 0.2 1.67 →
    1.02, 0.2 under reflectivity 0.6 1.42 → 1.05; hue within 0.04 (b/r).
    Calib-NVIDIA2 (floor 1.05): glossiness 0.3 0.90 → 1.02, 0 1.01,
    anisotropy 4 0.95 → 1.06, glass 1.46 → 0.98.
    Calib-NVIDIA (Iray 5000 samples, Octane 1024; sphere disk luminance,
    Octane / Iray; floor 1.04): Metal Brushed 1.02, Cast Iron 1.11,
    concrete 1.00, polished 0.95, porcelain 1.09, granite 1.09, resin 1.05,
    architectural 1.07; Calib-NVIDIA2: asphalt 1.05, ceramic tiles small
    1.04 (floor 1.05); hue within 0.02 (g/r) – 0.04 (b/r) but resin.
    Porcelain's and granite's excess (centre 1.14) is the diffuse under a
    full Fresnel gloss, as on the Uber's glossy spheres (Calib-Glossy
    1.02 – 1.16), not theirs alone.
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
  in hue (#8). D is the diffuse map's factor as DAZ hands it to Iray —
  with the SSS Reflectance Tint folded in under a shown Base Color Effect
  (the Genesis 8 Female base skin's 1 / 0.93 / 0.81 → × 1 / 0.85 / 0.63;
  `Material::base_color`, #52): G8F's skin, Octane / Iray median R / G / B,
  Calib-Eye 1.02 / 1.05 / 1.26 → 1.00 / 0.95 / 1.03, Calib-HDRI 1.00 /
  1.08 / 1.34 → 1.00 / 0.98 / 1.15 (#19). `SKIN_HUE` was fitted before
  the tint was known and may now count part of it twice on such skins;
- `radius` = per-channel diffusion length (see [iray.md](../daz/iray.md#translucency-and-subsurface-skin))
  normalised to its maximum and **square-rooted**; `scale` = that maximum in
  meters, clamped 0.5 mm – 2 cm. Why the root: Noemi's 1 / 0.18 / 0.15
  threw coloured fireflies, and Iray's look is softer.

**OpenPBR** (the Enhanced look, see Look; #19): OpenPBR Surface (505; its
`radius` is the distance and `scale` the per-channel colour — the other way
round from Standard Surface). On Iray's **chromatic SSS** (`SSS Mode` 1,
the modern skins) and on **mono SSS with Base Color Effect 0** its own
colour rule, fitted on Calib-Skin (eight Genesis 8 skins from pale to dark
on one body, Iray probes; see docs/notes/2026-10-28-skin-tones.md) and its
SSS variants (#28, docs/notes/2026-10-30-sss-volume.md). A mono-SSS skin
with Base Color Effect 1 (the Genesis 8 base skin) keeps the Standard
Surface tint × a lift by translucency w (`openpbr_skin_lift`): linear
between 1 at w 0, 1.3 at 0.5, 0.45 at 0.9 and 0.35 at 1. Up to 0.5 that is
1 / (1 − 0.46 · w), OpenPBR's darkening against Standard Surface (#8); past
it Iray darkens this skin far more (w 0.9 a ninth of w 0). Calib-Skin-Base
(docs/notes/2026-10-31-g8-base-skin.md): the base skin as it ships (w 0.5)
0.83 – 1.18 × Iray under the front light, 1.0 – 1.1 under the HDRI with
blue 1.2 – 1.4; w 0 0.94 – 1.08; w 0.9 luminance within ~0.8 – 1.3 but
bluer (B 1.2 – 1.9; one scalar lift can't fix hue). Under the baked rule
Calib-Eye came out 1.4 – 2.0 × Iray. SSS Direction changed nothing in
Iray's render of it (−0.65 → 0) and only enters the subsurface anisotropy.
Mono SSS with Base Color Effect 0 on this skin's volume is still too red
under the baked rule (G / B 0.52 – 0.89; the volume factor didn't move it:
its Translucency Color map carries the subsurface part), open in #28.

- **Colour, baked per texel** into one map for `baseColor` and
  `subsurfaceColor` (`Textures::derived_skin`, `skin_albedo`):
  `(1 − w)^1.3 · D + w · 0.55 · Tc · Tc_low` per channel (D the diffuse map
  × its colour, Tc the translucency map × its colour, `_low` low-passed).
  Why: Iray's probes put the diffuse layer at (1 − w)^1.3 (w 0.9 and 0.5
  within 0.94 – 1.03) and its subsurface part at ≈ 0.55 · Tc², not tied to
  D (a white diffuse map left it as it was); Tc · Tc_low keeps the map's
  texture at its contrast (Tc² doubled it).
- The subsurface part × the **volume factor** (`volume_factor`) per
  channel: (Rd(α) / Rd(α₀))^0.5, Rd Jensen's diffusion reflectance (dipole,
  η 1.4) at the volume's reduced single-scattering albedo α = σs(1 − g) /
  (σs(1 − g) + σa), α₀ the Genesis 8 skins' volume (Victoria 8's, which the
  rule above was fit on). DAZ's MDL: σa = −ln(Transmitted Color) / d_t;
  σs = −ln(SSS Color) / d_s (chromatic) or SSS Amount / d_s (mono); colours
  linear. Why: Iray's SSS variants on Victoria / Monique / Zurie (SSS
  Color grey, blue, deep; mono at Amount 2 and 0.5) moved the subsurface
  part as Rd's root predicts within ~10 % (Zurie ~20 %); Rd itself
  overshoots; Octane after it 0.81 – 1.34 × Iray relative to the base
  render (before 0.61 – 1.90). SSS Reflectance Tint did nothing on the
  figures and isn't read. The distances alone don't change α (Zurie's are
  half Victoria's, her factor 1).
- × **`SKIN_HUE_OPENPBR` (1.16, 0.83, 0.68)** to the power of the smooth
  colour's saturation ÷ 0.75, below 0.75 falling linearly to 0 at 0.45
  (`SKIN_HUE_SAT_FLOOR`): the eight figures came out 0.86 / 1.20 / 1.48 ×
  Iray per channel without it; at full strength the pale grey Zurie turned
  orange, and at saturation ÷ 0.75 she was still 0.89 / 0.75 / 0.64 (0.81 /
  0.83 / 0.80 with the floor; Angel's blue 1.06 → 1.24, the others within
  0.90 – 1.07 under the front light; #19). Open: Monique (dark) stays 1.03 /
  1.14 / 1.45 — not her darkness alone (Twosret, as dark, is 0.92 / 0.90 /
  1.00), and a stronger `SKIN_DARK_GAIN` (0.4) pulled the mid skins' green
  to 0.8. Under the HDRI every figure is ~0.2 bluer than under the front
  light: the light's colour, not the material.
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
  dual lobe's weight × min(2r, 1) of its reflectivity (lobe 2 the coating,
  see Universal) — with the
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
(`HAIR_COLOUR_SHARE`, `HAIR_TRANSMISSION_SHARE`; each its map's linear
mean × tint), at the root (Hair Root / Root Transmission Color) and at the
tip (Hair Tip / Tip Transmission Color), **blended along each strand**: a
Gradient (49) on a W Coordinate (104) from the root's albedo (W 0) to the
tip's (W 1); Root To Tip Bias / Gain other than 0.5 bend it — DAZ's
`get_line_root_to_tip_blend` (Schlick's bias, then gain; `basic.mdl`)
sampled at 15 control points spaced along the curve (≤ 1 % off at
0.2 – 0.8), linear between. Equal ends: a plain colour. Why: DAZ hands Iray
two `dual_lobe_hair` instances, root colours and tip colours, under a
`df::clamped_mix` weighted by that blend on the strand's line coordinate
(Calib-Mi-Hair / -Hair2's .mi: the tip colours and bias / gain do reach
Iray; bias / gain at 0.5 are left out as the MDL's default). Before #42 the
Blended shader took the root colours only. Roughness both ways from
`base_roughness`; specular = `Glossy Layer Weight` (with the highlight's
share, below). Why: Iray's Blended
Dual Lobe hair follows its transmission colour more than its hair colour —
Calib-Hair (blond) and two copies with dark brown hair or transmission
colours (render strands, 2× widths), fitted on the front-lit crown:
0.97 / 1.03 / 1.07 × Iray in luminance; Octane's Hair is linear in its
albedo (× 0.6 → × 0.60). Its `specular` barely shows (0.25 → 0.03: crown
−3 %). The hue is less saturated than Iray's on the blonds (crown r/g 1.16
vs 1.47) and more on the dark-transmission copy (1.18 vs 1.08) — no single
saturation curve fits both; open (#42). Backlit strands glow through
(Octane's TT lobe) where Iray's stay dark: kept, it is Octane's more
plausible take (the Enhanced look).

**Highlight** (#42): albedo = (1 − hw) × the above + hw × 0.15 × the
highlight colour (`HAIR_HIGHLIGHT_SHARE`; Highlight Root / Tip Highlight
Color at each end, hw = Highlight Weight); specular = (1 − hw) ×
`Glossy Layer Weight` + hw × the highlight colour (root / tip mean). Why:
`dual_lobe_hair` lays its highlight glossy (that colour, roughness²) over
the hair-coloured glossy and the base by a `weighted_layer` — no Fresnel,
so hw takes hw of the light and leaves 1 − hw; Octane's Hair has one R
lobe (Fresnel-weighted, too weak for 80 % of the light), so the share goes
into the albedo, the tint onto the R lobe. Calib-Hair-hl (Calib-Hair with
hw 0.8, highlight (0.2, 0.4, 1), separation 0.2; local), hair masked:
median 1.00 / 1.05 / 1.03 × Iray (RGB), mean luminance 0.84 × (Calib-Hair
0.85 ×); unmapped it was 2.17 × (median) and grey. 0.25 was 1.48 × in blue.
At Calib-Hair's hw 0.049 (pale highlight) the hair mean (same mask) is
0.85 × Iray, its dark copies 0.88 / 0.84 ×, unchanged within 1 %; on the
dark-rooted -tip copies the pale highlight adds 2 % (mean).
**highlight_roughness**: not mapped (Octane's Hair has one roughness for
all lobes; Calib-Hair-hl's equals its base_roughness, so it can't be fitted
there). **separation** tilts the two glossy lobes' normals ±separation
along the tangent, in opposite directions; Octane's `offset` (cuticle
tilt, float, slider 0 – 1) tilts its one R lobe: on Calib-Hair-hl offset
0.2 dimmed the hair 18 % (median) and lowered the log-luminance correlation
with Iray 0.950 → 0.932, 1.0 dimmed it 40 % — left at 0. **Anisotropy /
Anisotropy Rotations**: on strands the `anisotropy_conversion` tangent is
the strand's; Octane's fibre BSDF is anisotropic by construction — nothing
to map. **strength** reaches nothing in Iray (Calib-Mi-Hair2's .mi).

**Root → tip, checked against Iray** (Calib-Hair-tip: root dark brown, tip
blond; -tipbg: + bias 0.2 / gain 0.7; local): hair mean 0.94 / 0.92 ×
Iray in luminance, the dark crown 1.20 / 1.19 ×, the blond top tuft 1.03 /
1.03 × (linear medians); tipbg / tip in Octane 0.98 × Iray's ratio (hair
mean; crown 0.75 vs 0.80) — the bent blend lands where Iray's does.

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
An **Emission Profile** puts the IES profile on the emission's
`distribution` (see Lights, IES light profiles).

### Round Corners (#40)

DAZ wires `state::rounded_corner_normal(radius, across_materials,
roundness)` into every Uber material's `geometry_bump`. It passes the
channels `Round Corners Radius` / `Across Materials` / `Roundness` as they
are (Calib-RoundCorners .mi; radius 0 by default, which is off). The export
writes a radius > 0 to the material's `roundEdges` pin (on each material in
a mix) as Octane **Round Edges** (137):

- `roundEdgesMode` Accurate (2). Iray rounds convex and concave edges,
  and Accurate is the ray-traced mode, as Iray's is.
- `roundEdgesRadius` = Radius **in meters**. The MDL spec (19.3) gives the
  radius in meters in world space, and DAZ hands it over unconverted (as it
  does Matte Fog's visibility). So DAZ's 1 is 1 m, and 0.005 is 5 mm.
  Iray confirms it (Calib-RoundCorners, 5000 samples): R5 (radius 1)
  shades the whole 30 cm cube as a blob, which 1 cm could not. Octane caps
  the radius at 100.
- `roundEdgesCurvatureRoundness` = Roundness (1 round, 0 chamfer, in both
  renderers).
- 8 samples (Octane's default) and `roundEdgesConsiderOtherObjects` off.
  Iray rounds a mesh's own facets.
- **Across Materials off: the material's faces become a mesh of their
  own.** Iray then keeps that material's border sharp (R8: no highlight
  along the blue top's border; R7, across on, has one). Octane's Round
  Edges sees only geometry, but with other objects off it rounds an object
  against itself alone. So the export cuts each material with Across
  Materials off (and a radius > 0) out of its mesh into a prim of its own
  (`<mesh>_<group>`); the other materials stay one mesh, which keeps the
  name. The cut comes after the normals, so the shading across it doesn't
  change. Difference: Iray decides per shading point, so a neighbour with
  across on would still round towards it. After the cut, the neighbour
  doesn't round that border either.

Octane against Iray (Calib-RoundCorners, enhanced-final 1024 samples vs
Iray 5000): per cube, Octane is 1.02–1.06× Iray's level (the whole frame
1.017, the same as Calib-Smooth's 1.017, so this is not the rounding), and
mean |Δ| 2.7–5.2 / 255. The edge highlights widen alike from 0.005 to
0.02 m, and the chamfer (R6) is a flat band in both. Since the cut, R8's
border is sharp in both.
Not matched: at a radius beyond half the object (R5, 1 m on 30 cm), Iray
shades a smooth blob and Octane shows creases along the face diagonals.
That radius makes no sense on a real prop, so it stays as it is.

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
  and capped at 1 — on top of the export's own levels (usually 2; it was 2
  over one level before). Iray subdivides a SubD figure further for displacement
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
lm × Intensity, Intensity Scale ignored, no colour temperature — exactly
what DAZ hands Iray (`iray::light`, its .mi export: 3141.59 / 6283.19 lm);
renders had matched it (Calib-L-Point-legacy: Intensity 1 / 2 at scale
1000, 1 at scale 3000 → 3146 / 6281 / 3146 lm equivalent; the former 15 000 lm × Intensity × Scale was 4800× too bright
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
- **Spots** → analytic lights (402) — UsdLux shaping is lost: a disk of
  the emitter size (2 cm for a point source), for `Light Geometry`
  Rectangle a quad (`analyticLightType` 0, `quadAnalyticLightSize` width ×
  height in the light's XY plane, like Iray's `rectangle`; Calib-L-Mi-Spot-rect:
  the 20 × 40 cm emitter as in Iray, peak 1.02 ×), `normalize`, blackbody.
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

**Light options** (#35; Calib-L-Opt-* on Calib-L-Point-5k and Calib-L-Rect-50,
their .mi exports as fixtures, Iray references at 5000 samples):

| DAZ channel | what DAZ hands Iray | Octane |
|---|---|---|
| Render Emitter | the light's `visible` (camera rays only: still lights, still reflected — Iray manual, ghost lights) | an area light (rect, disc, sphere) the camera sees gets an object layer input `/Scene/Lights/<prim>_object` with `camera_visibility` on; off: left out |
| Temperature | `temperature` (photometric only) | blackbody `temperature`; Calib-L-Opt-Temp3000 floor R / G / B 1.03 / 1.01 / 0.98 × Iray |
| Cast Shadows, Shadow Type / Intensity / Softness / Colour / Bias, Decay, Illumination Diffuse Only / Specular Only | nothing — the .mi as without them | **not mapped** |
| Light Portal | nothing — portals were removed from Iray Photoreal in 2022.0 (release notes; DAZ Studio 6 runs build 387700) | **not mapped**: the rect stays a light |

**Octane hides a UsdLux light from the camera** unless its object layer
says otherwise: Calib-L-Opt-Rect-Facing (the rect turned to the camera, in
the frame) showed nothing where the rect is, without an object layer input
and with `camera_visibility` off alike; with it on, the rect's face (45 000
pixels); the rest of the frame and the floor stay as they were (0.999 ×;
Calib-L-Rect-50 / -2sided / Disc-50 / Sphere-d20 floor 1.00–1.01 × Iray
after the change). So Render Emitter **on** is what needs the input.
A point light has nothing to see (Octane's 1 cm sphere stays hidden), nor
does the camera headlamp (`visible` off in DAZ's .mi; the camera sits in
it). Spots are analytic lights, which Octane does show to the camera
(Calib-L-Opt-Spot-Facing: the 20 × 40 cm quad turned up to the camera,
22 000 pixels): Render Emitter off gives their `objectLayer` pin a layer
with `camera_visibility` off (-Spot-Facing-EmitterOff: none left).
Against Iray's references of the four Facing scenes: the faces in the same
place and size (to 1–5 px a side), white in both at the scene's exposure;
the rect's reflection in the sphere (unsaturated) 0.96 × Iray's energy, and
with Render Emitter off the face gone and the reflection kept in both
(0.91 ×, the rest of the frame 1.003 ×).

The options left unmapped are 3Delight's: Iray renders them identically
(every Calib-L-Opt-* reference but Temp3000 equals Calib-L-Point-5k to the
noise, mean ratio 1.000), so the export follows Iray, not the channel's
intent — a scene is lit as its author saw it in Iray. Octane could express
some (emission `castShadows`, `Visible on diffuse` / `Visible on specular`,
a portal material), which would light the scene differently from Iray.


**IES light profiles** (#34; Calib-L-IES-* — a point light level and
turned down with DAZ's own `light_profile_test.ies`, one turned down with
`calib/ies/Orbis-Asym.ies` (synthetic: a downlight to 60°, 1 / 0.6 / 0.1 /
0.3 at φ 0 / 90 / 180 / 270), a 50 cm rect, an emissive plane with each
profile; their .mi exports as fixtures):

- **What DAZ hands Iray.** A point light with a **Light Profile** (any
  Light Geometry; DAZ's spot and distant lights have no such channel)
  becomes `dzpm_ies_light` — NVIDIA's `lights_photometric::ies_light`:
  `measured_edf(profile, global_distribution, identity frame)` at
  intensity = flux / `light_profile_power` (`iray::light`): **the profile's
  shape at the light's Flux**, its candela values irrelevant. The file goes
  as a `lightprofile` (flags 2, counter-clockwise horizontal angles;
  hermite 1); a content-relative path resolves against the content
  directories, an absolute one (DAZ's own file in its app bundle) as is.
  The Uber **Emission Profile** sets `emission_profile_enable` and
  `emission_profile` (`iray::uber`): `measured_edf(global_distribution:
  true)` in the **mesh's object space**, the luminance divided by
  `light_profile_power`.
- **Orientation in Iray** (Iray references of Calib-L-IES-*, 5000
  samples — not what NVIDIA's comment in `ies_light` suggests): the
  profile's θ = 0 lies on the emitter's **local −Z**, φ = 0 on **+X**,
  φ = 90° on **−Y** — the light's own frame turned half round X, the way a
  spot's beam points. Point-Down (turned X −90°) showed a dark disc of
  35° under the light (the file's 0 – 35° zero band; +Z would have made
  it 19°); Asym-Down the downlight on the floor, its dim notch (φ 180)
  towards −X and the φ 90 side (0.6) towards world +Z = local −Y. An
  emissive surface likewise: Emit-Asym's plane (turned X 180°) throws its
  downlight forwards along world +Z = object −Z.
- **Octane**: the emission's `distribution` pin takes a greyscale image
  (36) reading the `.ies`, packed at `assets/ies/`, normalised to a peak of
  1 (`iesPhotometryMode` 1, as in Octane's hallway sample), through a
  Spherical projection (77; `positionType` 1 world, 3 object, 4 normal space
  (IES lights)). **Octane's IES frame** at rotation 0 (Orbis-Asym, probes):
  axis **−Y**, φ 0 **+Z**, φ 90 **−X**; the projection's `transform` (order
  XYZ) turns that sphere into the coordinate space (X +90° moved the axis
  from −Y to −Z). `ies_projection_rotation` turns it onto Iray's frame:
  lights in **object space** — a point / sphere prim is the light's own
  frame, so Iray's is (X, −Y, −Z) of it (rotation 90 90 0); a rect / disc
  prim is already turned half round X (`write_lights`), Iray's frame is
  its own — emissive surfaces in **world space**, (X, −Y, −Z) of the
  node's axes (the export's meshes are in world space; the dump's node
  transform). All six scenes now light the same places as Iray.
- **Power.** Octane's distribution only scales the emitter's radiance
  (peak 1), Iray keeps the flux:
  - **Point / sphere light**: × 1 / (4 × the peak-normalised profile's
    mean over the sphere) (`ies::Profile::mean_share` = ∫ I dΩ / (4π
    I_max); `IES_POINT_SHARE` 4). Iray lit Point, Point-Down and Asym-Down
    at exactly 1/4 of the flux-kept profile (Octane / Iray 4.00 – 4.04
    across the floor with × 1 / mean_share, either profile); with the 4:
    0.997 – 1.009, the shape identical. (A probe without Iray, 300 lm, had
    confirmed Octane's side: φ 0 / 90 / 270 / 180 at 0.99 / 1.00 / 1.02 /
    1.06 of I(ω) · 4π / ∫ I dΩ.) Likely Iray's `ies_light` on its small
    sphere: the flux spread over the whole surface, a direction seeing a
    quarter of it.
  - **Rect / disc light, emissive surface**: Iray emits the profile itself
    (its intensity ∝ I(ω), no cosine: on Emit the ratio to the plain
    emitter's Lambertian falls exactly with 1 / cos), Octane a Lambertian
    emitter × the distribution, I(ω) · cos θ_n — a shape Octane can't
    match. The power keeps the **flux through the front**: × 1 /
    `Profile::lambert_share` = π I_max / ∫ I(ω) max(0, n · ω) dΩ, n the
    surface normal in the profile's frame — the rect's / disc's axis, an
    emissive mesh's face normals by area (`face_normal_shares`, ~5° bins;
    a two-sided emitter averages front and back; without them the mean
    share). For a downlight on a ceiling panel (light near the normal)
    the shapes nearly agree; a profile throwing its light sideways lights
    the floor below brighter in Octane and the walls dimmer:

    | scene | Octane / Iray on the floor | ≈ / cos θ_n | flux, Octane / Iray |
    |---|---|---|---|
    | Rect (DAZ's batwing, axis = normal) | 1.44 – 1.66 | 2.02 | 0.93 |
    | Emit (batwing, axis ⟂ normal) | 1.21 – 1.57 | 1.75 | 0.87 |
    | Emit-Asym (downlight ⟂ normal, light at 53 – 60°) | 2.11 | 2.65 | 0.77 |

    (flux = the ratio's constant × ∫ D cos / ∫ D over the front.) The old
    rule (1 / mean_share for all) had the floor at 1.2 – 1.4, 1.2 – 1.6
    and 1.05 there, but kept only 0.78 / 0.87 / 0.38 of the flux and
    would make a downlight on a rect ~3× too bright.
- **Not mapped**: an IES file's TILT data (the ORBIS parser skips them; Octane
  reads the file itself).


**Camera headlamp** (#22, `lights::headlamp`): DAZ hands Iray a 7 × 3.5
cm rect at the camera, colour 0.8, of 10 × Depth of Field (cm)² ×
Headlamp Intensity lm (`iray::headlamp`, its .mi export: 1.77 Mlm at
Calib-Glossy's 420.5 cm focus, 0.4 Mlm at 200 cm). The export makes it a
point light at 0.96 of that flux (`HEADLAMP_POINT_SHARE`: Iray lit
Calib-Glossy without its dome 1.65 – 1.80 × as bright as a 1 Mlm point
light there; with the dome it adds 10.8 %), at the camera moved by X / Y /
Z Headlamp Offset (cm, camera axes: right, up, back) — where DAZ puts it
(its .mi export). When:

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
  0.97 with the same shadows; at the origin Iray rendered night).
  **`SS Horizon Blur`** b (Iray's `horizon_blur`) → `groundBlendAngle` =
  max(1°, 0.76 × asin(b / 10)) (`ground_blend_angle`): Iray blends the
  ground in by a smoothstep over asin(b / 10) below the horizon (NVIDIA's
  `hosek_wilkie_sky.mdl`, the same parameter as `base::sun_and_sky`),
  Octane's Daylight model by (angle / B)² — 0.76 fits it best
  (Calib-L-Mi-SunSky-Look, b 2: 11.5°, B 8.8°, the seen ground weight
  within rms 0.04 of Iray's curve, probe 120). Against Iray's render of
  Calib-L-Mi-SunSky-Look (both clipped above the ground, so only the
  ground side measures): the ramp's 10–90 % is ~60 px in Faithful, ~75 px
  in Iray (0.8 ×), Octane's horizon ~40 px higher (`Horizon Height` isn't
  mapped). Only Faithful's Octane
  model blends: Hosek-Wilkie (Enhanced) keeps a hard horizon whatever
  `groundBlendAngle` / `groundStartAngle` say. Left to Octane's sky, as
  looks rather than lighting (DAZ passes them, Calib-L-Mi-SunSky-Look):
  `SS Blue-Red Tint` (`redblueshift`), `Saturation`, `Sun Glow Intensity`,
  `Horizon Height`; `SS Night Color` (black by default, as Octane's night).
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
  measured against Blender at 0° and 110°, Octane2). With Dome Orientation
  X or Z ≠ 0 the image gets a Spherical projection (type 77, world space)
  rotated by (X, Y, Z) in order ZXY — Iray's Rz · Rx · Ry as it is — and the
  U shift drops orientation Y: `0.25 + Dome Rotation / 360`
  (Calib-L-Mi-Dome-turned, sky correlation 0.95; `docs/notes/2026-11-03-dome-tilt.md`).
- **Environment Tint** (linear, Iray's `environment_function_tint`)
  multiplies the map: a Multiply texture (39) of the image and the colour.
  Calib-Env-Tint (1, 0.5, 0.3): 1.04 × Iray everywhere, as untinted.
- No HDRI → Octane's white environment.
- **Dome Mode** (`crate::iray::dome_shape`, .mi: Calib-Env-DomeMode*,
  Calib-Env-Mi-Box / -SphereGround). The modes **with a ground**
  ("Infinite Sphere w/ Ground", "Finite Sphere / Box w/ Ground"): Iray
  projects the HDRI's lower half onto the ground, looked up from a virtual
  camera `Ground Texture Scale` (cm) above it. → a thin SDF box (296,
  2 km × 1 mm × 2 km, `DOME_GROUND_SIZE`) whose top is `DOME_GROUND_DROP`
  (2 mm) below Iray's ground height, under the shadow catcher; its
  material a Diffuse (17) of albedo 0 emitting the HDRI (texture emission,
  surface brightness on, power = the dome's × `DOME_GROUND_POWER` 1)
  through a world-space Spherical projection at the virtual camera — the
  same image node, U shift and tilt as the environment, the projection
  mirrored in X (`scale` −1 1 1: on a surface Octane's lookup is the
  environment's mirror image) (`projected_environment_texture`).
  Calib-Env-DomeMode1: below the horizon 1.03 – 1.04 × Iray, as the rest of
  the frame (without it 0.00 – 1.44). The infinite plane (110) renders an
  emitting material black, hence the box. Not with Draw Dome off. With
  Draw Ground on as well the shadow catcher sits above it (no reference).
  The **finite** dome itself (sphere or box: Dome Radius / Width / Height /
  Depth × Dome Scale Multiplier, Dome Origin) is **not mapped**: Octane's
  texture environment is at infinity, and a 100 m sphere around the
  Calib scene changed Iray's sky by 5 % and the scene by < 1 %
  (Calib-Env-DomeMode2). Visualize Finite Dome is a viewport aid.
- **Ground Fog** (`crate::iray::ground_fog`: densities = linear Density
  Start / End × their scales per meter, heights = Decay Start and Decay
  Start + Decay Height in m, absolute; Iray's extinction is `density0`
  below `height0`, decaying exponentially above it,
  `decay = (h0 − h1) / ln(d1 / d0)`) → Unit Volumes (295) of a Volume
  medium (98), 2 km wide (`FOG_RADIUS` 1 km) around the render camera
  (`ground_fog_slabs`): one 1 km deep below `height0` at `density0`, then
  slabs of `FOG_SLAB_DECAYS` = 2 decay lengths (at most 16) up to 1/1000 of
  the ground's density, each at the decay's mean over it per channel (its
  optical depth exact); equal densities → one uniform slab 1 km high.
  Scattering = albedo × extinction, absorption = (1 − albedo) × extinction
  (`VOLUME_COEFFICIENT_SCALE`), the phase Iray's Henyey-Greenstein
  anisotropy; ray-march step `UNIT_VOLUME_STEP` 0.1. Calib-Env-GroundFog:
  floor 1.06, below the plane 0.94, spheres 1.06, horizon 1.03 × Iray
  (the fog-free frame 1.04 – 1.06); at step 1.0 the floor 1.27 and below
  0.49, sixteen half-decay slabs were slower and further off. Costly: ~15 ×
  the fog-free render time in Octane (Iray 2.9 ×). Octane's **Post FX fog**
  (same base level / half-density shape) crashed the render engine at once
  (GPU memory full) and isn't used. The Approximate-Mie scattering type
  (droplet size) isn't mapped: Octane's phase is Henyey-Greenstein only,
  so Mie renders isotropic (DAZ hands an anisotropy of 0 then too). Not
  in Scene Only mode.
- **Matte Fog** (`crate::iray::matte_fog`): not mapped. Iray's matte fog
  attenuates and brightens the scene by distance (Koschmieder: visibility
  = the 2 % contrast distance) but leaves rays to the environment alone;
  an Octane volume fogs everything it holds. A uniform box around scene
  and camera (extinction ln 50 / visibility, albedo = brightness) at
  Calib-Env-MatteFog's 100 m brightened the spheres 1.24 × Iray (Iray's own
  fog +7 %) and the HDRI below the plane 1.90 ×, at 16 × the render time;
  without it the frame is 0.99 – 1.04. DAZ's default visibility, 10 km,
  is invisible at scene scale.
- **Environment Lighting Resolution / Blur**: not mapped — Iray Photoreal
  uses the baked environment only for importance sampling (NVIDIA's
  manual); Calib-Env-Res128 rendered identical to 512, Calib-Env-Blur
  within noise (< 0.3 %). Octane's importance sampling has no such knob.
- **Ground Reflectivity / Glossiness** (Iray's ground reflects the scene,
  Fresnel-weighted, roughness √(2 / glossiness)): not mapped — Octane's
  shadow catcher (145) has opacity only, and no reference shows Iray's
  ground reflection (the Calib scenes' plane covers it). **Ground Visible
  from Below** (only handed over with Draw Ground on): not mapped — the
  shadow catcher is transparent from below as well, i.e. Octane behaves
  like the option off.

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

- Materials: displacement [#24](https://github.com/vysokyj/orbis/issues/24), top coat
  modes [#25](https://github.com/vysokyj/orbis/issues/25), dual lobe
  [#26](https://github.com/vysokyj/orbis/issues/26), colour effects
  [#27](https://github.com/vysokyj/orbis/issues/27), volume / SSS
  [#28](https://github.com/vysokyj/orbis/issues/28), backscattering
  [#29](https://github.com/vysokyj/orbis/issues/29), flakes
  [#30](https://github.com/vysokyj/orbis/issues/30), thin film
  [#31](https://github.com/vysokyj/orbis/issues/31), diffuse overlay
  maps [#71](https://github.com/vysokyj/orbis/issues/71), dispersion
  [#33](https://github.com/vysokyj/orbis/issues/33), DAZ Default shader
  [#41](https://github.com/vysokyj/orbis/issues/41), legacy 3Delight shaders
  [#64](https://github.com/vysokyj/orbis/issues/64), brick / MDL shaders
  [#21](https://github.com/vysokyj/orbis/issues/21), skin hue
  [#19](https://github.com/vysokyj/orbis/issues/19), eyes
  [#20](https://github.com/vysokyj/orbis/issues/20), hair
  [#42](https://github.com/vysokyj/orbis/issues/42).
- Lights and camera: IES profiles [#34](https://github.com/vysokyj/orbis/issues/34), light
  options [#35](https://github.com/vysokyj/orbis/issues/35), lens
  [#39](https://github.com/vysokyj/orbis/issues/39), spots
  [#7](https://github.com/vysokyj/orbis/issues/7).
- Scene: hidden surfaces and render flags
  [#36](https://github.com/vysokyj/orbis/issues/36), fog and finite dome
  [#37](https://github.com/vysokyj/orbis/issues/37), Iray render
  settings [#43](https://github.com/vysokyj/orbis/issues/43), Sun-Sky
  channels [#18](https://github.com/vysokyj/orbis/issues/18), skin
  texture baking time [#78](https://github.com/vysokyj/orbis/issues/78).
