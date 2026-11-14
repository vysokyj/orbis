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
([Universal](#universal-130--default), [Skin](#skin--openpbr-surface-505-final-only),
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
OpenPBR without subsurface, in the colour Iray's skin shows, Octane's daylight model with a white
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
| `skin` | OpenPBR Surface, random walk | OpenPBR Surface, Iray's colour as a diffuse | Universal (no SSS) | Universal (no SSS) |
| `thin_glass_ior_one` | no (Fresnel) | yes (without a glossy layer) | no | yes (without a glossy layer) |
| `daylight_model` | 3 Hosek-Wilkie | 1 Octane, white sunset | 3 Hosek-Wilkie | 1 Octane, white sunset |
| `daylight_power` | 2.7 | 2.864 | 2.7 | 2.864 |
| `daylight_extinction` | 0 | 0.200 | 0 | 0.200 |
| `max_transmission` | 1 | 1 | 0.75 | 0.75 |
| `strand_hair` | Hair material, pile albedo | the MDL's layers | Hair material, pile albedo | Hair material, pile albedo |

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
  OpenPBR 1.06 – 1.29 (before #19's rule). Faithful's skin: Calib-Skin
  under the front lamp 0.80 – 1.04 in red (see Skin). **Hair** 1.04 (Final) / 1.06 (Preview).
- Shared misses, not variant-bound: Calib-Glass sphere 4 2.4 × in all
  four (the 2.8 read through the split boxes, #146); Preview's Glass sphere 7 0.81 and Mixing sphere 8 0.76 (Final
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
  `Line End Width` (DAZ labels them mm) × 0.001 m: the unit itself, in
  every variant. Why: Iray's own strands are 1× (DAZ's .mi: fibre radius,
  ribbon width and prism size all follow the width in mm; strand counts
  equal the dump's), and on sparse patches Iray's coverage follows 1×
  down to 0.15 px, Octane's curves against Iray's fibres 0.91 – 1.00 at
  equal width (#79, notes 2026-11-11 strand width); an Uber material on
  one thick fibre 0.99 – 1.04 × Iray at 1×, 1.55 – 2.0 at 2×. The earlier
  2× stood in for the Hair material's fitted albedo shares on thin hair
  (Calib-Eye's close-up "covered" the background as Iray's at it), the 4×
  before was fitted on guide-only strands (#16).
- **Line Tessellation**: the scene's modifier on the strand node (`Render
  Line Tessellation Sides`) says how DAZ hands the strands to Iray: fewer
  than 2 sides or no modifier → Iray's `hair` fibres (`Material::fibres`),
  else ribbons (2) or prisms. The export always writes round curves; the
  flag steers the hair shader's layers (dForce hair, Faithful) and an Uber
  translucency: on fibres Iray loses the diffuse transmission (black), so
  Faithful + Final writes none there; the other variants keep it (Look:
  a strand that passes light). Why: Morgan's Strands material on fibres
  (Calib-Strand-UberM, -Dome: whole / no gloss / no translucency / no
  refraction, a thick strand, a sparse and a dense patch) 0.44 - 0.47 x
  Iray under the sun with the transmission (Universal's highlight goes
  when a transmission joins the dielectric specular), 0.83 - 1.14 without;
  every part 0.76 - 1.14, under the dome 0.91 - 1.07 (#79). A
  value the scene leaves out counts as tessellated (library files are
  read without their modifiers).
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
  a mirror goes into a negative x scale, shear is lost); the source itself
  is the first copy, at identity.
- A **hidden** target (Visible or Visible in Render off; under a hidden
  group too, as for any node, unchecked in a .mi) is left out **with its
  copies**. Why: DAZ's .mi has neither the
  target nor any copy of it, for an instance node and an instance group
  alike (Calib-Mi-InstGroup-Hidden, -InstGroup-NoRender,
  Calib-Mi-Instance-NoRender, Calib-Env-Mi-GroundAuto-InstanceHidden; with
  the target shown, Calib-Mi-InstGroup has the sphere and both group
  items). UltraScatter doesn't hide its sources: Gladiator's are shown and
  in its .mi (the grass sources sit 2 m under the terrain) (#130).
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
| kernel | direct lighting (24), GI diffuse, depth 2 | path tracing (25): **diffuse 64**, glossy 24, **scatter 128**, **GI clamp 20** |
| ray epsilon | **1 µm** (`rayepsilon` 1e-6) | same |
| skin | Universal | OpenPBR: Enhanced random-walk SSS, Faithful Iray's colour as a diffuse |
| imager exposure | **1.4** × 2^(13 − EV) × cm² (Variants) | **1.4** × 2^(13 − EV) × cm² |
| imager response | **Gamma 2.2** (403) | same |
| highlight compression | 0; Burn < 1, Crush > 0, Gamma ≠ 2.2 or a coloured White Point → a tone LUT (below) | same |
| vignetting | Iray's cos(θ)^v fitted as Octane's frame-space cone per camera (below) | same |
| white balance / saturation | white (the White Point is in the tone LUT, below) / Saturation | same |
| denoiser | off | **on**, during the render (`denoiserOnce` off, `maxDenoiserInterval` 20) |

**Diffuse depth 64**, not Octane's 8: OpenPBR's random-walk subsurface spends
the diffuse depth on its scattering events (maxScatterDepth does not bound
it), so at 8 a long walk is cut and a bright subsurface colour loses most of
its light. Probe 64 (Monique's skin painted grey c, radius ×0.1, the albedo
as rendered against a plain diffuse of the same c):

| c | 0.1 | 0.3 | 0.6 | 0.8 | 0.95 |
|---|---|---|---|---|---|
| depth 8 | 0.97 | 0.93 | 0.76 | 0.63 | 0.54 |
| depth 32 | 0.97 | 0.99 | 0.98 | 0.90 | 0.80 |
| depth 64 | 0.97 | 0.98 | 0.98 | 0.97 | 0.87 |
| depth 128 | 0.99 | 0.98 | 0.99 | 0.99 | 0.93 |

Skin's red is its brightest channel, so depth 8 cut red most: the skin came
out bluer (Victoria Enhanced 0.89 / 0.98 / 0.98 × Iray, at 64 0.96 / 1.00 /
0.97). Iray bounds subsurface walks apart from its path length (Max SSS Path
Length), so this is Octane catching up with Iray, not a fit. Plain diffuse
scenes render the same at 8 and 64 (Russian roulette ends those paths); the
skin scenes take 10 – 25 % longer.

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

- **DAZ Default** (`daz_default_as_uber`, Calib-Mi-DzDefault2,
  -DzDefault4 … -DzDefault8): diffuse × diffuse_strength (both maps; a
  strength map attenuates the colour); glossy weight = the specular colour's
  strongest channel (linear) × specular_strength, its hue the glossy colour,
  a specular_strength map driving the weight; roughness
  (1 − glossiness)^1.7. A **specular colour map** makes the whole colour
  (map × value) the glossy colour (Glossy Color Effect 1) at weight
  specular_strength alone. A **glossiness map** switches DAZ to Uber's
  specular / glossiness mixing: Glossiness = map × value, squared
  `gloss_to_rough`, Glossy Specular 0.04 as an 8-bit DAZ colour (0.0399472).
  Lighting Model **Matte** drops the specular channels, maps included: a
  white glossy layer at weight 1, roughness 1; the other lighting models
  and the Subsurface group (shown or hidden) change nothing. Refraction at
  its strength, colour and IOR (maps on either dropped), closed; it shares
  the glossy inputs when its colour equals the glossy colour, else Share
  Glossy Inputs off; a refracting material passes no transparency.
  Transparency as Cutout Opacity, bump (factor = its value; bump_min / max
  don't reach Iray), normal (factor 1) and displacement (min / max) with
  their maps; u/v scale and offset as the tiling. Ambient, reflection and
  their maps don't reach Iray, nor bump or displacement without a map.
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
quantised). PBRSkin types four of its colours `color` - Makeup Base, Top
Coat, Metallic Flakes and SSS Color - and DAZ hands them to Iray rounded to
the nearest 1/255, then the gamma (Calib-PBRSkin-Colour8's `.mi`: 0.199 /
0.402 / 0.703 reach Iray as 51 / 103 / 179 ÷ 255, 0.123 / 0.456 / 0.789 as
31 / 116 / 201); the export reads them the same way on both sides
(`Material::colour_is_8bit`). Its `float_color` ones (`diffuse`,
Translucency and Transmitted Color) and all of Iray Uber's colours keep
their floats. The rounding moves an encoded value by at most 0.5/255, a
linear one by about 0.43 % / c relative (4 % at 0.1, under 1 % above 0.45).

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
- **Weighted base mixing** (`Base Mixing` 2): the diffuse part is
  `diffuse_base`, as in Base Mixing 0 — the diffuse map unscaled at
  1 − Translucency Weight, the translucency Universal's transmission
  (below) — and `Glossy Weight` is the gloss, the weighted Mix's share
  (below). Without a gloss DAZ's `normalized_base` takes the diffuse part
  alone: Diffuse Weight is unread (Calib-WeightedTL: W2 / W1 and W4 / W3
  0.97 in Iray, the step between neighbouring spheres in Octane too; #131).
  The map used to be scaled to the blend of its diffuse and translucency
  shares (`weighted_mix`, fitted ×3 by hand on a private mesh hair, probe
  59): the translucency counted as reflected light where Iray's sphere
  darkens and turns the translucency's colour (W3 – W5 2.1 – 2.5 × Iray).
- **The glossy layer's weight follows Base Mixing** (#127): `Glossy Weight`
  (and its map) under weighted mixing (2), `Glossy Layered Weight` under 0
  and 1 — the other channel gives no gloss. DAZ's `normalized_base` has no
  `glossy_layered_weight` (DAZ passes 1 whatever the channel: Calib-Mi-Mix2,
  -Visible2, the Calib-Tone base's .mi) and enables the gloss from Glossy
  Weight alone (Calib-DualLobeWeighted W2); `metallic_layered_base` has no
  `glossy_weight` (DAZ passes 0). The larger of the two had counted in every
  mode: a weighted material with Glossy Weight 0 (or a translucent weighted
  one, which skips the Mix) got a gloss on `specular` Iray hasn't, at
  roughness 0 since DAZ passes none without gloss (#121). The skin's gloss
  (`skin_gloss`, `skin_glossy_layer`) reads the same channel.
- roughness (dual-lobe skin: Specular Lobe 1 Roughness), metallic, cutout
  opacity = greyscale map × value or constant; normal / bump maps. Without
  `Cutout Opacity` (DAZ Default shader) opacity is `transparency` ("Opacity
  Strength"): the NGV8 pubic hair's "Hiding" cards at 0 rendered as a white
  layer over the skin (probes 62–63).
- **No glossy layer, dual lobe, refraction or mirror → `specular` 0** (Iray
  is matte there; Universal's white default glittered velvet, probe 53).
  `specular` = Glossy Color (× its map) × glossy weight (× the `Glossy
  Layered Weight` map), flat when neither has a map and only below 1 — the
  MDL tints the glossy BSDF by Glossy Color and weights the layer apart, so
  both maps apply, multiplied in a Multiply texture (39); a weight map alone
  dropped the colour and the Base Mixing 1 hue (#107). Universal's full
  white default made the GSOV4 leather (gloss 0.33 through a seam mask)
  3.4× Iray's sheen; now 1.13–1.16 (Fetroom).
- **Translucency** off the skin path (props, cloth, cards) → Universal's
  diffuse transmission: `albedo` × (1 − w), `transmission` = Translucency
  Color (map) × w, `transmissionType` 1 (Diffuse), 3 (Thin Wall (Diffuse))
  when thin-walled. Not on metal (a translucent metal's base transmits in
  a Mix of its own, below) or refraction. Base Mixing 2 too: its
  `normalized_base` takes the same `diffuse_base` and mixes it whole
  against the gloss (#131) — Calib-WeightedTL (thin-walled, enhanced-final)
  W1 – W8 0.92 – 1.06 × Iray, translucency 0.5 / 1 0.92 / 1.00, with the
  gloss 1.01 / 1.02, the hair maps 1.05 (all 1.06 – 2.5 before).
  **A closed, non-thin solid with Iray's volume** under it (Transmitted
  Color or SSS set: σa, σs as DAZ passes them; not metal, refraction,
  strands or the eye, which has its own) has **no transmission: one
  albedo, the ball's** (`prop_volume_escape`, `eye_volume::ball_albedo`,
  per texel under the diffuse / Translucency Color maps as the eye's):
  r + t² P / (1 − r P), r = (1 − w) b and t = w Tc (Base Color Effect's
  shares, `base_color_effect_shares`), P the share of the light let in
  that the volume returns to the wall, traced in a ball of the
  material's surface area (R = √(A / 4π), every exit counts, a channel
  that absorbs nothing returns all). Why: Iray's closed translucent
  sphere is exactly DAZ's MDL — a Lambertian wall ((1 − w) b reflected,
  w Tc transmitted, both ways) around a homogeneous σa / σs volume: a
  Monte Carlo of it (`scripts/dev/prop-volume-mc.py`) at each sphere's
  own light (Octane's plain diffuse 0.5 at the same place) predicts
  Calib-SSSColor's spheres at 0.93 – 0.98 of Iray and Calib-PropVol-*'s
  (the volume in a ball 1 / k the size) at 0.95 – 0.99 up to k 30, as
  Iray's own plain diffuse sphere — light that comes out near where it
  went in. Rendered (Octane / Iray, enhanced-final): Calib-SSSColor V1 –
  V7 1.09 – 1.13 / 0.95 – 1.01 / 1.13 – 1.24, Calib-PropVol-* k ≤ 30
  1.06 – 1.11 / 0.97 – 1.01 / 1.13 – 1.21, the Parts at k 1 (Tc / base
  white, w 0.5 / 1, g ±0.6) 1.06 – 1.12 in red, Calib-SSS-Solid S7 1.08;
  Octane's plain diffuse sphere at that place 1.08 / 1.03 / 1.08 (its
  spectral rendering of the saturated albedo, blue the weakest channel
  by 13 ×). Lossless volumes (no absorption) 1.15 – 1.27: the light
  wanders the whole ball. Before, the transmission × a fitted volume factor
  ((Rd(α) / Rd(α₀))^0.5 × 0.7) lit the hollow ball from inside, so its
  ratio followed the sphere's place in the row (red 0.45 – 1.17); Octane's
  scattering medium with Iray's σ (fake shadows off) is the same physics
  per channel (over a grey σ 0.94 – 1.02 of the Monte Carlo) but
  spectral: Victoria's Transmitted Color (σa 0.36 / 13 / 18 per cm) leaks
  into red, 0.42 – 0.60 × Iray on every dense volume
  (docs/notes/2026-11-14-prop-volume-bake.md). Limit: a volume thin for
  its ball (k ≥ 100) passes light from behind, which an albedo can't —
  red 0.82 – 1.04 (k 100 – 1000; the medium's 0.52 – 0.78), the thin
  Parts 0.76 – 1.40. A closed
  solid with no volume (Iray's empty ball) keeps the transmission: Iray
  renders it as its thin-walled twin to 1 / 255 (Calib-SSS-Solid S2 / S3 /
  S8, Calib-WeightedTL-Solid, Calib-DualLobeWeightedTL-Solid against their
  thin twins), Octane too (0.91 – 1.00 Y either way); the red deficit
  (0.84 – 0.96) is the same on the shells, not the wall's (#163). Where
  the solid holds a volume Iray darkens it (S5 / S7, Transmitted Color,
  0.80 / 0.76 × the shell, red 0.70 / 0.65; S6, SSS, 0.95) and the baked
  albedo follows: 1.05 / 1.08 / 1.05 × Iray, the shells (no volume in
  Iray: `add_uber_volume` needs a closed wall) 0.94 – 0.96 (#134). SSS Reflectance Tint isn't passed to Iray (the .mi;
  1.34 when it was read, #28).
  Why the transmission at all: left out, Calib-SSS's spheres at
  w 0.5 / 1 rendered 1.6 – 1.7 / 3.2 × Iray and Calib-PBRSkin's Gladio cloth
  1.72; with it 0.91 – 0.96 and 1.23 (its neighbours 1.14 – 1.26), the two
  types alike on Calib-SSS's spheres (thin-walled shells through the base
  material) and on Calib-SSS-Solid's closed ones (Diffuse, 1), both 0.92 –
  1.00 × Iray where no volume enters (#134). On P8 (metallic 0.75) the transmission
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
  colour × its map's mean (over the texels the material's faces cover:
  `UvCoverage`, a 128² grid of the map's UV square, each face in its own
  UDIM tile; a tiled or offset map, or no UVs, leaves out the near-black
  texels only — a white fill between UV islands, Monique 8's, read 2 – 4 ×
  her painted skin, #82; skin keeps the near-black rule, below), as DAZ
  hands it to Iray — with the SSS
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
  A **partial refraction** keeps the translucency: Iray's refraction is a
  weighted layer over diffuse and translucency (`metallic_layered_base`),
  so both keep 1 − Refraction Weight and both transmissions stay. In Base
  Mixing 0 (not PBRSkin) a translucent material with 0 < Refraction Weight
  < 1 is the same Mix as a refractive metal's (**Refraction on a
  metal**): the refraction (its own transmission, IOR and gloss) at
  Refraction Weight over the material without it (the translucency at its full share). Why: as one node
  Universal's one transmission held either the translucency's (refraction
  < 0.5) or the refraction's; Calib-MetalRefr-TL (thin-walled, w 0.5, Tc
  0.9 / 0.6 / 0.4) R1 (refraction 0.3) 0.51 × Iray (R G B 0.55 0.48 0.60),
  R2 (0.7) 0.90; as the Mix 1.04 (1.03 1.05 1.04) and 1.07, beside the same
  spheres without translucency (1.01 / 1.04) and its metal spheres (1.10 –
  1.13, unchanged) (#135). On closed solids (-TL-Solid) the same Mix
  renders R1 / R2 1.05 / 1.04, beside the solids without translucency
  (Calib-MetalRefr-Solid 1.05 / 1.03): the split holds (#134). Where the material stays one node (Base Mixing 1,
  PBRSkin, inside another Mix) a refraction below 0.5 leaves Universal's
  transmission to the translucency (the refraction's own goes; its IOR
  and gloss stay). Morgan Hair (Base Mixing 1, w 0.059, effect 2,
  refraction 0.106): transmission 0 → 0.30 × Tc. The Genesis EyeSocket's
  caruncle rendered 4.6 × Iray, now 1.00 (Calib-Eye, #20; the Mix leaves
  its export unchanged). Found by
  switching the EyeSocket's other channels off one at a time in Iray on the
  cards (SSS Amount / Direction / Tint, Transmitted Color, its gloss, Top
  Coat, Invert Transmission Normal: no change). An **open sheet** that isn't
  thin-walled (a material whose faces have an edge no other face of the mesh
  shares: cards, leaves, a cloak) passes Tc × min(2w, 1): Iray lit
  Calib-Leaf's backlit card 1.9 × its thin-walled twin, Octane rendered them
  alike (0.48 / 0.90 × Iray at w; 0.81 at min(2w, 1)); doubled on Calib-SSS's
  spheres it overshot (1.25 – 1.38). A closed solid stays at w: Iray
  renders Calib-SSS-Solid / -WeightedTL-Solid / -DualLobeWeightedTL-Solid
  as their thin-walled twins to 1 / 255 where no volume enters, and
  Octane at w within 0.91 – 1.08 of it (#134). Opaque card 0.97.
- **Glossy Reflectivity** r is the head-on reflectance of the glossy
  layer's custom curve, Glossy Layered Weight × (F0 + (1 − F0)(1 − cos)^5),
  F0 = 0.08 r × its map (`metallic_layered_base`; DAZ hands Iray the map ×
  0.08 r at gamma 1, a store outfit's `.mi`): the weight on `specular`
  (or on the `coating` over a dual lobe), **the IOR of that F0** on `ior`
  (`coatingIor`), per texel with a map — as on the skin and the dual lobe,
  and as Base Mixing 1's Glossy Specular (#100). Universal's Fresnel at
  that IOR is the physical one: equal head-on and over the hemisphere
  within 1 – 8 % from r 0.25 up (0.99 / 1.07 / 1.07 × at r 0.25 / 0.5 /
  1), but **not per angle**: 1.12 – 1.30 × Schlick's at 45 – 60°, 0.87 –
  0.96 past 80° (`scripts/dev/gloss-split.py fresnel`). No one IOR draws
  Schlick's shape, and Octane's is the physical dielectric — kept (#137).
  It is what made a gloss's excess over Iray depend on where the sphere
  stands: under Calib-DualLobeGlossy's low sun the highlight's half angle
  between view and light runs 35° → 62° from sphere 1 to 8, and the same
  glossy layer (r 0.5, Glossy Roughness 0.5) is 1.12 → 1.29 × Iray over the
  frame's 1.08 (the dual lobe alone 1.07 → 1.24; on the sphere's projected,
  elliptic disc, #146), against a single-scatter
  sun model of both lobes (`gloss-split.py sun`: `libbsdf`'s Phong /
  v-cavities / Schlick against Beckmann / Smith / Fresnel) of 1.07 → 1.30
  (1.08 → 1.38); the sky's share dilutes the measured trend. Below r 0.25
  it rises later — 0.79 × at r
  0.1, nothing at r 0 (IOR 1), where Iray's curve still reaches 1 at
  grazing. That residual is left, not fitted: Calib-Glossy's r 0 sphere
  shows no gloss in Octane (Iray's: 27 % of r 0.5's on the sphere's core).
  Until #100 r scaled the weight, min(2r, 1) at IOR 1.5 plus the excess
  2r − 1 as a coating, which dimmed (or doubled) the grazing reflection
  too: Calib-Glossy's gloss, split from the red diffuse by colour and over
  the matching diffuse ratio, r 0.25 0.86 → 1.03, r 1 1.35 → 1.07, r 0.5
  1.05 (unchanged); the r 1 sphere 1.25 → 1.17 × Iray. (Those discs sat
  on the highlight; on the projected discs, #146, the current export's
  gloss is r 0.25 1.09, r 1 1.04, r 0.5 1.07 over the diffuse ratio, the
  r 1 sphere 1.12.) **Not under weighted mixing** (Base
  Mixing 2), neither the value nor its map: DAZ's `normalized_base` has no
  `glossy_reflectivity` (its gloss is a plain glossy BSDF at Glossy
  Weight), and DAZ's `.mi` export hands Iray none —
  Calib-DualLobeWeightedTL's T8 (Reflectivity 0) and T3 (0.5) get
  identical arguments, so Iray renders them alike (#132).
- **Dual lobe specular** (skin's, no glossy layer): Iray blends two GGX
  lobes by `Dual Lobe Specular Ratio` — on Iray Uber the ratio weighs lobe
  1 (default 0.85) and lobe 2 has `Specular Lobe 2 Roughness` (default 0, a
  mirror); on PBRSkin it weighs lobe 2 (default 0.15), whose roughness is
  lobe 1's × `Specular Lobe 2 Roughness Mult` (0.4) (DAZ's MDL). Both
  lobes sit under one Schlick curve W · (F0 + (1 − F0)(1 − cos)^5), F0 =
  0.08 r × the Reflectivity map (`add_dual_lobe_specular`'s
  `custom_curve_layer`): the weight W on the pins, the reflectivity as the
  IOR of F0 on `ior` and `coatingIor`, per texel with a map (n / 2 baked at
  power 2; Universal's IOR pins take a texture, probe #81). Lobe 1 is
  `specular` at W × its part; lobe 2 the `coating` at the rest, `coatingRoughness` the **Phong pin of r2²**
  (both lobes are `simple_glossy_bsdf`; 2 · r2² until #95, a fit that made
  up for the GGX curve's sharpness — Calib-DualLobe2's lobe-2-only sphere
  then 0.42 × Iray's detail at the Phong pin). Under a
  Top Coat (which takes the coating) one lobe at the full W and the
  ratio's blend of both roughnesses. The same on OpenPBR skin. Until #81
  Universal took the reflectivity as a share of the specular (W · min(2r,
  1), its map dropped, the IOR only above 0.5): the share dims the grazing
  reflection that Iray keeps at W, and a dark map shone the full lobe —
  the Calib-Skin `-w0` fronts (Universal: no translucency) forehead
  Monique 1.19 / 1.39 / 1.63 → 0.96 / 0.97 / 1.00 × Iray, Twosret 1.05 /
  1.26 / 1.49 → 0.94 / 0.96 / 0.95, unmapped Victoria unchanged;
  Calib-DualLobe's r 0.25 sphere 1.05 → 1.16, at its neighbours' 1.10 –
  1.18 (#137). A `Dual Lobe Specular Weight` map multiplies both lobes' pins on
  Universal as on OpenPBR (DAZ hands Iray the map itself as
  `dual_lobe_specular_weight`; Calib-G9Eye's `.mi`, gamma 1): Genesis 9's
  eye (`G9_Eyes_SLW`, ≤ 0.17 over the iris, 0 on the sclera) had taken the
  full gloss, a grey sheen over the iris (#133). Why: Calib-DualLobe rendered one sharp highlight whatever the ratio
  (p99 / median of the sphere 9 – 13, Iray 5.0 – 8.1 for lobe 2 at 0.6);
  now 6.4 / 5.2 / 7.6 / 9.3 against 8.1 / 5.5 / 8.1 / 9.0, lobe 2 at 0.1
  9.8 vs 10.8; r 1 0.79 → 1.17 × Iray. At the Phong pins (#95) its eight
  spheres 13.5 / 6.8 / 4.8 / 11.5 / 4.9 / 8.3 / 13.0 / 6.1 against Iray's
  13.4 / 8.4 / 5.6 / 11.1 / 5.0 / 9.2 / 14.0 / 5.7, detail 0.8 – 1.06 ×
  Iray's at correlation 0.93 – 0.999 (on 2 · r2² and the GGX curve 1.1 –
  2.4, 0.55 – 0.99). On a skin the ratio moves little: Victoria's forehead
  0.92 × with all of it on lobe 2, 1.00 on lobe 1 (Iray).
- **PBRSkin Specular Occlusion and Detail roughness** (#54, from
  `pbr_skin.mdl`, no fit): under `Specular Occlusion Enable` the dual
  lobe's custom curve reflects n = F0 · lerp(1, Facing, w) head-on and
  g = lerp(1, Grazing, w) at grazing (w = `Ambient Occlusion Weight`;
  DAZ's channel is spelled `Specular Occusion Facing`) instead of F0 and
  1. W (n + (g − n)(1 − cos)^5) is a Schlick curve at weight W · g and
  F0 n / g, so both paths take that weight (the layer's whole share on
  Universal) and the IOR of n / g (`specular_occlusion`). An occlusion
  map counts by its value only (the library's mapped skins sit at
  Facing = Grazing = 1, a no-op). Under `Detail Enable` lobe 1's
  roughness is × lerp(1, `Detail Specular Roughness Mult`, `Detail
  Weight`) before Iray squares it, lobe 2 inheriting it
  (`Material::detail_roughness_mult`; a map on the multiplier counts by
  its value). DAZ hands all of these over as saved
  (Calib-PBRSkin-Occlusion's `.mi`, a map on the occlusion weight as the
  map). Checked against Iray (Calib-PBRSkin-Occlusion, enhanced-final,
  denoised): O2 - O5 mean within 0.92 - 1.02 of the unoccluded sphere's
  Octane / Iray, Detail roughness 0.5's peak 2.09 × vs Iray's 1.99 ×; the
  mapped occlusion weight is the known gap (#158).
- **PBRSkin Detail Normal Map** (#140, from `pbr_skin.mdl`'s
  `pbr_detail_normal`, no fit): under `Detail Enable`, in `Detail Normal
  Map Mode` 1 (a tangent-space normal map), the detail map — tiled by
  `Detail Horizontal / Vertical Tiles` (on its own per-map tiles, its
  offsets as the material's on a map's; DAZ's default 10 × 10) — is
  blended onto the base normal (the Normal Map, Bump Strength's height
  map, or both baked, **height maps as normal maps** below) by
  `base::blend_normals`: the detail at `Detail Weight` as a linear blend
  between the unperturbed normal and the detail, normalised, then
  reoriented normal mapping onto the base. It goes in **procedurally**:
  an OSL texture (117) on `normal` with the base and the detail as two
  image textures, each with its own UV transform, doing that blend
  (`detail_normal_osl`); not baked, because the tiles would need the base
  map at tiles × its resolution (a 4K map at 10 × 10 tiles: 40K), against
  the texture budget, and a capped bake would lose the detail it is for.
  The inputs are dynamic pins, written with `dynamicType='5'` (without
  it Octane compiles the shader on load and leaves them at their default
  colours: no detail). The Top Coat doesn't take it in the export, though
  DAZ wires `pbr_add_clearcoat`'s `top_coat_base_bump` to the same
  `pbr_detail_normal` (#166). DAZ hands Iray the map at
  its definition's `default_image_gamma` 1, its weight and the tiles as
  the transform's scale (Calib-PBRSkin-Occlusion O8's `.mi`). Known
  input: O8's shading normal (Info kernel, `octane-probe-bumptilt.lua`)
  through the OSL is the same map baked 4 × 4 by hand to 0.18° RMS
  (8.61° vs 8.62° tilt). Against Iray (Calib-PBRSkin-Occlusion O8):
  tiling and phase as Iray's, level within 4 % of the plain sphere's, the
  dimples ~25 % stronger in Octane although the formula is NVIDIA's
  `base.mdl` to the letter (open: #140's Calib-PBRSkin-Detail). Mode 0
  (the map as a height map) and a map on the weight aren't exported (the
  weight counts by its value).
- **PBRSkin Makeup** (#142, from `pbr_skin.mdl`'s `pbr_material`, no
  fit): under `Makeup Enable` (and Diffuse Enable) `diffuse_makeup_bsdf`
  lays a diffuse in `Makeup Base Color`, at the base's Diffuse Roughness,
  at `Makeup Weight` over the diffuse and its translucency (a
  `weighted_layer`, under the dual lobe, the metal and the Top Coat);
  lobe 1's roughness is × lerp(1, `Makeup Roughness Mult`, weight) before
  Iray squares it (beside Detail's factor, lobe 2 inheriting it); under
  `Makeup Metallicity Enable` the metal's tint is lerp(base colour,
  makeup, weight) (`blend_colors`, `color_layer_blend`), else the base
  colour. Every layer over the makeup is linear in its base, so it goes
  in as a **Mix material (19) at Makeup Weight** (its map at its gamma ×
  the value) of the whole material in the makeup's colour without
  translucency or subsurface (the layer covers both) and the whole
  material without makeup (`makeup_mix_material`): the stack over the
  blended base, exactly, also over the subsurface skin. The makeup side's
  metal takes the makeup's colour (so the Mix gives the lerp); without
  Makeup Metallicity Enable that side is a Mix by the Metallic Weight of
  the metal in the base colour and the makeup's diffuse. The roughness
  factor goes on both sides at a flat weight; under a weight map the
  makeup side takes the full mult and the base none (exact where the
  mask is 0 or 1; Iray's single blended roughness between). Weight 1
  without a map: the makeup side alone. Makeup Base Color is a `color`
  channel, kept in 8 bits: 0.1 / 0.3 / 0.8 reach Iray as (26 / 77 / 204 ÷
  255)^2.2 (Calib-PBRSkin-Makeup's `.mi`; `diffuse` is a `float_color`
  and keeps its float). DAZ writes `makeup_enable` /
  `metallic_makeup_enable` only where they differ from the MDL's
  defaults (true / false). Known limits: the makeup diffuse shades with
  the unbumped normal in Iray (`state::normal()`), on Universal with the
  material's one normal; the library has no material with Makeup Enable
  on (9 scenes set its channels).
- **Glossy layer over a dual lobe** (#106): DAZ's MDL has both in the
  stack at once — the dual lobe over the diffuse (`dl_diffuse_bsdf`), the
  glossy layer's custom curve over that (`metallic_layered_base`), each
  with its own roughness; DAZ hands both over (Calib-DualLobeGlossy's
  `.mi`). On a plain dielectric (Base Mixing 0, or Base Mixing 2 with
  translucency — below; no refraction, no metal) the export keeps both, in Iray's order: the dual lobe on `specular` as
  under a Top Coat (the full share, both lobes at the ratio's roughness
  blend, IOR above r 0.5), the gloss as the `coating` — Glossy Color
  (map) × weight (map), `coatingRoughness` the Phong pin of
  the glossy layer's own roughness (map included), `coatingIor` the IOR of
  F0 = 0.08 r × its map (per texel, as on `specular`, #100), the base's bump /
  normal on the coating's pins; no anisotropy (the coating has none). A
  Top Coat over both moves into a Layered material's Specular Layer
  (`layered_coat_material`), so the coating stays the gloss's. On OpenPBR
  skin the same (the gloss's weight and F0's IOR per texel on the coat,
  undarkened); a Top Coat over them (OpenPBR has `specular` and one coat)
  wraps the OpenPBR skin in the same Layered material, the coat its
  Specular Layer (#138: the fallback to the gloss alone dropped the dual
  lobe, the face 0.76 × Iray's rise over the uncoated skin; layered it
  rises 1.39 × against Iray's 1.35, as the uncoated 1.37 / 1.34);
  elsewhere (metal, refraction, Base Mixing 1) the gloss alone, as
  before; weighted mixing, translucent or not, keeps it in the Mix's
  diffuse part (#121, #128, below). Wherever the gloss is on `specular`, the roughness is the glossy
  layer's (`glossy_roughness_source`), not lobe 1's — the gloss took the
  dual lobe's roughness until #106 (a weighted mix's gloss too). Why:
  dropping the dual lobe left 15 of 348 installed dual-lobe skin
  materials without it (one also with a Top Coat). Against Iray
  (Calib-DualLobeGlossy) both layers together come out 1.17 – 1.21 ×
  (Top Coat 1.26, dual lobe W 0.5 1.33) where each alone is 1.17 / 1.12
  (on the projected discs, #146; the highlight-centred discs had read
  1.17 – 1.27, 1.39, 1.48); Octane's
  coating takes only 2 % of the layers under it. Split per sphere (-dl /
  -gloss / -bare, layer = scene − bare): Iray's stack takes 1 - 4 % of
  the two layers, Octane's 0 - 4 % (Top Coat: 27 / 26 %), so the
  stacking holds; the excess is each layer alone at that sphere's light
  (the same gloss 1.21 / 1.26 / 1.40 at spheres 1 / 3 / 8, the diffuse
  1.08 everywhere; #137). On the OpenPBR skin the coat over the dual lobe
  rises as Iray's (dual lobe added: face 1.37 vs 1.34); with a Top Coat
  the fallback loses the dual lobe (face 0.79 × Iray; #138).
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
- **BSDF model and roughness.** Every Uber lobe is `simple_glossy_bsdf`
  (`choose_bsdf` at `dfm_glossy`; DAZ never sets `df_mode`): the gloss,
  the dual lobe's two, the Top Coat, the refraction — and the NVIDIA
  dielectrics' gloss. Phong of exponent 2 / r² (r = Iray's roughness after
  its squaring) = Beckmann at α = r / √(1 + r²); Octane's roughness pin is
  √α. So every Phong lobe's pin is **√(r / √(1 + r²))** (`phong_pin`;
  derived, no constant) — on Universal's roughness and coating, the
  Layered material's Specular Layer, the weighted gloss and coat, the
  skins' OpenPBR. Universal's **`brdf`** (one per node, `phong_brdf`):
  **Beckmann** (1), Phong's own distribution; **GGX energy preserving**
  (6) at the same pin when a dielectric lobe on the node (the gloss unless
  the surface is all metal, lobe 2 of a dual lobe, a Top Coat) has α >
  **0.25** — Octane's limit for a dielectric Beckmann (below). Metals take
  Beckmann at any roughness, rough glass too (Refraction). Exceptions:
  architectural's glass keeps the pins its rule was swept on and its
  anisotropy has its own α ratio (NVIDIA MDL examples), a saturated anisotropic metal lobe plain GGX (Anisotropy), the flakes
  GGX energy preserving (their roughness stands for the flakes' normals).

  Why (#95; the 2026-10-31 GGX curve min(r + 1.85 r², √r) it replaces was
  fitted to highlight peaks, ≈ r for a sharp lobe, so α ≈ r²): shape —
  Octane / Iray band-passed detail and its correlation with Iray's, curve
  → Beckmann at the pin: Calib-Glossy's r 0.09 spheres 1.31 / 1.75 → 0.98
  / 0.99 at correlation 0.93 / 0.91 → 0.999 / 0.994; Calib-Aniso's
  isotropic r 0.16 3.34 / 0.69 → 1.02 / 0.996; Calib-DualLobe 1.6 – 2.4 /
  0.81 – 0.91 → 0.8 – 1.06 / 0.93 – 0.999 with Iray's p99 / median
  (13.5 vs 13.4); Calib-SpecMode, -Map-SpecMode, -Mixing, -Map-Metal,
  -Map-TopCoat to correlation 0.995 – 1.00 (the specular-mode sphere
  0.59 → 0.80). GGX at the same α has the same peak but its long tail
  blurs (detail 0.75 – 0.78). Brightness — furnace probes (a uniform white
  dome, a white Lambert sphere at 1.000): Octane's Beckmann is
  single-scatter Smith-Beckmann at α = pin² (pin 0.6 / 0.8: 0.966 / 0.807
  against 0.964 / 0.823 computed), plain GGX single-scatter Smith-GGX, GGX
  energy preserving 0.998 at every pin. Iray's v-cavities masking loses
  more than Smith: Beckmann / Iray albedo 1.00 to r 0.2, 1.03 at 0.4, 1.06
  at 0.5, 1.11 at 0.7, 1.17 at 1 — what is left over Iray on a rough metal.
  The limit: a black dielectric (specular 1, IOR 1.5) under the scene's
  HDRI renders its analytic Smith-Beckmann lobe (integrated over the same
  HDRI) at 1.03 – 1.08 up to pin 0.5 (α 0.25), 1.14 / 1.18 / 1.62 / 2.25
  at pin 0.6 / 0.7 / 0.8 / 1 — in a uniform furnace it matches at every
  pin, a white metal under the HDRI too (0.91 – 1.04): Octane's rough
  dielectric Beckmann is off under directional light. Calib-DualLobe2's
  rough lobes as Beckmann 1.24 – 1.54 × Iray (their gloss alone 1.36 –
  2.08), as GGX energy preserving at the same pin 1.06 – 1.09 (gloss 1.03
  – 1.10 on all eight spheres; the matte level 1.08). A specular-mode IOR
  is at least 1.1: Iray's curve still rises to 1 at grazing over a black
  Glossy Specular (that sphere 0.91 → 1.00).
- **Anisotropy** (Glossy Anisotropy + Glossy Anisotropy Rotations on the
  gloss and the metal, Top Coat Anisotropy + Top Coat Rotations on the top
  coat; #23). Iray (`base::anisotropy_conversion`, MDL SDK's `base.mdl`):
  roughness_u = r, roughness_v = r / (1 − a) (r / 0.0001 at a = 1), the
  tangent turned by 2π · rotation. Octane: each axis at the Phong pin
  above (not capped: r / 0.0001 gives pin → 1), **`roughness` = v** (the
  long axis), **`anisotropyTexture` = 1 − pin_u / pin_v** (1 stretches
  along the texture's v, like Iray; negative when u is the longer),
  **`rotation` = 1 − Iray's rotation** (both in turns; Octane's slider 0 –
  1, it turns the other way). A roughness map bakes v per texel.
  Octane's lobe at roughness p and anisotropy a is α = p² along its long
  axis and **p² (1 − |a|)²** across (#96, measured): an orthographic probe
  of white metal spheres lit from the camera by one small spot in a black
  environment, the highlight fitted per sphere with an anisotropic
  Beckmann (or GGX) lobe, Smith masking, against a Lambert sphere (scale
  0.98 – 1.00 on every resolved lobe): isotropic α = p² (0.361 at p 0.6,
  1.00 at 1); short / long α at a 0.25 / 0.5 / 0.75 = 0.571 / 0.259 /
  0.064 ((1 − a)² 0.5625 / 0.25 / 0.0625), the same at p 0.3 – 1 and
  under GGX, the long axis staying p². The rules before — (α_v − α_u) /
  (α_v + α_u) (#92), (v − u) / (v + u) of the pins (#23) — made the short
  axis narrower than Iray's (Metal Brushed's mid texel α_u 0.066 for
  Iray's 0.10).
  Where Iray's long axis runs past 1 (r / (1 − a) > 1, Phong's exponent
  below 2) a lobe that would be Beckmann (a metal) takes **plain GGX**
  (`brdf` 2) at the same pins: Phong of exponent n = 2 / r² goes flat as n
  → 0 (D = 1 / π along v at anisotropy 1), and GGX at α 1 is that flat
  distribution exactly — Beckmann at α 1 (Walter's √(2 / (n + 2))) isn't.
  Single scatter, as Iray's lobe. A dielectric node keeps GGX energy
  preserving (its long axis is past the Beckmann limit above). Why:
  Calib-Aniso2's sharp metal K3 (r 0.04, a 1) 1.53 × Iray under
  Beckmann, 1.14 under GGX; the weighted coat K4 1.32 / 1.05 (the
  scene's other spheres 1.09 – 1.12); analytically, under the scene's
  HDRI, Beckmann / GGX 1.37, Octane's two renders 1.34. Plain GGX on the
  saturated dielectric spheres too blurred their streaks (Calib-Aniso A3
  / A7 / A8 detail 1.09 / 1.35 / 1.59 → 1.54 / 2.16 / 2.37 × Iray).
  **Albedo (#96).** Iray's `simple_glossy_bsdf` has v-cavities masking
  and no multiple scattering, Octane's Beckmann Smith masking: Octane's
  lobe keeps more light, computed (single scatter, F = 1, the lobes at the
  same α) as **not applied** — "Octane over Iray": Iray's extra loss is
  the v-cavities model's, not the surface's. Beckmann / Iray, cosine-
  averaged over the hemisphere / head-on / a sphere under Calib-NVIDIA's
  HDRI: isotropic r 0.2 1.00 / 1.00, 0.4 1.01 / 1.04 / 1.02 (0.36), 0.7
  1.06 / 1.18, 1 1.10 / 1.23 / 1.17; anisotropic Metal Brushed (u 0.04 –
  0.18, v 0.36 – 0.48) 1.00 – 1.01 / 1.01 – 1.03 / 1.01, Calib-Aniso's
  metal A6 (u 0.16, v 0.8) 1.03 / 1.08 / 1.05 (rendered: A6 / the
  isotropic A1 1.064), saturated (u 0.04, v r / 0.0001) 1.07 / 1.09 /
  1.11. Resin's loss (`resin_lobe_albedo`, 19 % at r 0.25) isn't this
  effect: v-cavities against Smith on the whole resin sphere is 0.6 % at
  r 0.25; the loss is `fresnel_layer`'s weights and Iray's own rough
  transmission (Dielectrics below, #113, #115).
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
  `dynamicType` 33) with the coating's colour × weight, maps included
  (`specular`), its
  roughness, IOR (`top_coat_ior`) and the anisotropy and rotation as
  above. Why: as an isotropic coating Calib-Aniso's A7 (mode 0, a 1) was
  1.22 × Iray with a sharp round highlight where Iray draws a streak,
  Calib-Aniso2's rotated K5 1.21; as a layer 1.16 / 1.08 with Iray's
  streaks. Modes 2 / 3 differ from 0 only in the IOR (no reference of
  them anisotropic). The weighted coat (mode 1) keeps its own metal node.
  A coat over a glossy layer over a dual lobe takes the same Layered
  material, isotropic or not (#106, above), on an OpenPBR skin too (its
  base the OpenPBR skin without its coat, #138).
  **On strand hair** (an Uber material on `BasisCurves`) the rotation
  turns a quarter more, 1 − (rotation + 0.25), gloss and coat alike:
  DAZ hands Iray tessellated strands (Line Tessellation sides 2 / 3) as
  flat ribbons or flat-faced prisms whose UV u runs across the strand and
  v along it (`docs/daz/iray.md`, Strand geometry; Morgan Hair is
  ribbons, Calib-Hair prisms), so Iray's unrotated tangent lies across
  it, and Octane's curves take theirs along it. Octane's curves are round
  where Iray's ribbons are flat, and stay so (decided 2026-11-12, #79: no
  ribbon meshes - Iray's flat ribbons are a tessellation artefact, black
  under a hard light from most views; Morgan's material on ribbon patches
  renders 0.47 - 0.92 x Iray, the real hair 1.0 - 1.8). Untessellated strands reach Iray
  as its own `hair` fibres, and there the curves match per unit width
  (Calib-Strand row A, 2× wide: 1.97 – 2.05 × Iray, the highlight at the
  same place across the strand). Why: Morgan Hair
  (Rotations 0.25, anisotropy 0.74 / 0.92) streaked across its strands as
  exported; Iray streaks along them. Turned, its highlights 0.69 – 0.77 →
  1.07 – 1.13 × Iray, the crown 2.98 → 2.24 (#52). On the measured
  anisotropy (#96) its lengths rose 1.00 – 1.17 → 1.11 – 1.31 × Iray: the
  lobes as wide across the strand as Iray's show more of the strands'
  gloss. That excess (crown / middle 1.2 – 1.8 × the ribbon reference)
  is the ribbons' shape, not the lobe, and stays: Morgan as Iray fibres
  (Line Tessellation 1) renders 1.3 – 3.5 × its ribbons in Iray, gloss
  and base alike, and its gloss then lands where Octane's does (gloss /
  own base on the hilite and middle 1.67 / 1.28 in Iray, 1.53 / 1.19 in
  Octane); Morgan's ribbons turned 65° in place (Calib-Strand-UberM-Edge-T2)
  darken every Uber part in Iray, not the gloss alone (#99, notes
  2026-11-14). Against Iray's fibres the base is low instead (#161).
  Backscattering Anisotropy reaches Iray only from a channel saved
  shown; DAZ's default material (our generated scenes) hides it, so
  Calib-Backscatter B8 and Calib-Aniso2 K6 stayed isotropic
  (Calib-Mi-Visible2-Shown, see iray-uber-mdl.md, Channel visibility). The
  Octane side keeps the backscatter isotropic either way.
- **Top Coat** → Universal `coating` = Top Coat Color (× its map) ×
  weight (× its map; both maps multiplied as the glossy layer's, #107),
  `coatingRoughness` (Top Coat Roughness and its map, or 1 − Top Coat
  Glossiness in Base Mixing 1 — verified; Weighted (2) reads Roughness,
  Calib-Mi-Mix2's coat 0.3², #110; a brick graph's keeps Glossiness, as
  its gloss does — squared likewise), `coatingIor`: the top-coated sphere's
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
  surface without its coat; the mix carries the displacement (Displacement,
  below). It is the
  outermost Mix: a flake or backscattering Mix lies in its `material2`,
  and their nodes carry no Universal coating. Calib-TopCoat
  weighted 1 / 0.4: 1.03 – 1.05 × Iray (as a coating 0.05 – 0.10; Iray's
  weight-1 sphere is a chrome mirror).

  Why (outermost): DAZ's `add_top_coat` wraps everything below, metallic
  flakes and the base's backscattering included; nested inside them the
  coat covered only their surface's share, and the flake and backscatter
  metals got a Fresnel coating at IOR 1.5 instead of the weighted mirror
  (#102). Derived from the layer stack, not rendered: no calibration
  scene combines the two.
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
  coat's highlights). So the coat's `normal` pin gets the MDL's coat
  normal baked (**height maps as normal maps**, below): the coat's bump
  (a height or normal map) blended onto the base's (the Normal Map, else
  Bump Strength's height map; `add_detail_normal`, Iray's B7 shows the
  coat's dimples and the base's ridges at once), the coat's alone without
  a base, the base's whole surface detail without a coat bump. Maps that
  can't be baked together (different placements, unreadable) go on
  Octane's own pins as before #85: the coat's height map, else the
  base's; with both, their sum — a Mix texture (38) at amount ½ of the
  two at twice their power — and the coat's normal map, else the base's.
- **Height maps as normal maps** (#85): every Uber height map — Bump
  Strength's, the Top Coat Bump in Mode 0 — goes in **baked into a
  tangent-space normal map** on `normal` (`detail_normal`), with the
  Normal Map it lies on: per texel MDL's `file_bump_texture` (the height
  difference of the texels either side, `h[i+1] − h[i−1]`, v up the
  image, the map's luminance decoded at its gamma — below) on unit tangents
  (`apply_bump`: n = normalize(−f · du, −f · dv, 1), f the channel's
  value), the normal map (`tangent_space_normal_texture`: 2c − 1, |z|)
  the base and the height map blended onto it by `base::blend_normals`
  (reoriented normal mapping; DAZ's `add_detail_normal`), at the larger
  map's size (a coarser height map's slope per its own texel), 8 bits
  ((n + 1) / 2). The inputs leave the package (only what `project.ocs`
  names is packed). Why: Octane's bump reads the height map per texel
  without interpolation and takes its slope over about a pixel: the
  normal is flat inside a texel and tilts hard in a one-pixel line at
  each texel edge, and keeps every frequency up to the texel's (a flat
  plane, Info kernel point samples, `octane-probe-bumpfreq.lua`,
  `bump-freq.py`: 87 – 93 % of samples untilted, the RMS slope of sine
  stripes the same from a 48- to a 2-texel period, where MDL's central
  difference gives 0). Its normal map Octane interpolates bilinearly
  (correlation 0.99 with the bilinear normal) and reads as Iray at
  strength 1 (B5, B6, B8). So the mapping is exact and has no constant:
  Calib-BumpRes against Iray (`bump-ladder-fit.py`, band-passed residual
  per sphere) 1.18 / 1.22 / 0.23 / 0.77 / 0.80 / 0.34 / 0.56 / 0.17 →
  0.27 / 0.14 / 0.09 / 0.09 / 0.25 / 0.18 / 0.16 / 0.10, Calib-CoatBump
  (RMS of the grey image per sphere) B2 3.5 → 2.5, B3 6.5 → 2.2, B4 9.9
  → 3.3, the rest unchanged or lower; the skin's dual lobe narrows with
  the bump as Iray's does (below, Gloss).
- **Height and normal maps are decoded at the gamma Iray reads them
  with** (#91): the image's own gamma (Textures, image gamma below),
  else gamma "auto" by the file's pixel type (Map gamma, below: 2.2 for
  8 bits and 16-bit colour, 1.0 for 16-bit grey or float). The bake decodes each texel at that gamma
  before the central difference (a normal map before 2c − 1). A normal
  map Octane takes as it is (on `normal`, unbaked) goes in re-baked
  linear when its gamma isn't 1: Octane reads a normal texture linear
  whatever its `gamma` pin and colour space say (Calib-BumpGamma-Uber2:
  the 8-bit map at 2.2 in legacy gamma rendered as at 1.0). Octane's own
  bump (the fallback) gets the gamma on its pin. Why: Calib-BumpGamma
  (three vertical cards, a distant light 45° up in the plane of v and
  nothing else; `bump-gamma.py`): a 128-texel sawtooth rising and falling
  8 levels a texel against a flat band of the same row gives the slope
  f · s per texel row from the Lambert ratio (1 ∓ f s) / √(1 + f² s²)
  with no fitted scale. Iray, 8-bit PNG at `map_gamma` 0: RMS of f · s
  against a 2.2 decode 0.031, against 1.0 0.130 (f / 2: 0.005 / 0.059);
  the same map at `map_gamma` 1 and a 16-bit PNG at 0: 0.018 against
  1.0, 0.116 against 2.2. NVIDIA's Concrete alike (0.032 / 0.130; 1.0:
  0.018). The export now: 0.009 / 0.010 against 2.2, 0.002 / 0.003
  against 1.0. An 8-bit normal map at `map_gamma` 0: its flat band at
  0.386 of a flat height band in Iray (a 2.2 decode, Lambert: 0.344;
  1.0: 1.00), the export 0.379; contrast of the ±0.25 tilts 0.313 /
  0.304. Product content: bump maps at `map_gamma` 0 and 1 alike (76 /
  57 of 133 sampled), normal maps nearly always 1; the eight Calib-Skin
  figures' face and body maps 1 (unchanged), Zurie's arms and some
  eyes and mouths 0. The linear rule before rested on Cast Iron's
  band-passed detail (#89), which the decode hardly moves in Iray
  (Calib-NVIDIA-BumpG1, its maps at `map_gamma` 1: detail 0.94 ×,
  correlation 0.98 against Calib-NVIDIA) — that measure doesn't test
  the decode (NVIDIA presets, Bump, below).
- **Bump strength** (#61, #75): the fallback above and architectural's
  bump (`mdl_dielectric`; the NVIDIA presets below) still use Octane's own
  bump: a height map on `bump` goes in at Octane image `power` = Iray's `file_bump_texture` factor (Bump
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
  refraction's — Octane's Thin Wall transmission ignores roughness, so the
  pin shapes only the reflection, which in Iray is the glossy layer's (see
  **Thin Walled** under Refraction) —, its Top Coat its own. Why: at the refraction's 0 the cornea mirrored
  the room over the iris (a dark band across it, a hard highlight); at the
  gloss's 0.07 the iris shows Iray's veil — Calib-Eye pupil 0.43 → 0.89,
  iris 0.57 – 0.74 → 0.79 – 0.83 × Iray, sclera 0.97 / 1.13 (#20). Until
  #95 the coat took at least the gloss's roughness too: on the GGX curve
  the gloss itself was sharp (pin 0.079) and the blurred coat drew the
  veil. At the Phong pins (BSDF model and roughness) the gloss (pin 0.26)
  is the veil and the coat (0.0016, pin 0.04) Iray's sharp highlight inside
  its halo; the coat at the gloss's pin drew one broad glow instead. The
  eye's medians, Octane / Iray, pupil / iris / sclera (a disk r 40 px, a
  ring 70 – 115 px, a box): GGX curve 0.63 / 0.85 / 0.87, both at the
  gloss's pin 0.97 / 0.91 / 0.88 (the glow over the pupil), each its own
  0.83 / 0.88 / 0.92.
- **Weighted mixing** (Base Mixing 2), with or without translucency → a
  **Mix material** (19): `material1` a metal (metallic 1) in the Glossy Color (or
  its map) at the glossy layer's roughness, `material2` the surface without its
  gloss (`specular` 0, the diffuse map unbrightened; a dual lobe stays
  there, below), `amount` = Glossy
  Weight / (Diffuse Weight + Glossy Weight) (`weighted_gloss`); the Top Coat
  on both. DAZ's `df::normalized_mix` scales the two weights down only when
  they sum past 1; below 1 the rest is black (MDL spec 21.6.1; DAZ hands
  both weights as set, Calib-Mi-Mix2 / Calib-DualLobeWeighted's .mi). So
  under 1 `amount` = 1 − Diffuse Weight and the metal's colour (albedo,
  edge tint) × Glossy Weight / (1 − Diffuse Weight), the diffuse part and
  the coat untouched (`WeightedGloss`, #126): Diffuse 0.5 / Glossy 0.3
  had rendered 0.625 D + 0.375 G for Iray's 0.5 D + 0.3 G. Confirmed
  against Iray: Calib-DualLobeWeighted's W5 (weights summing to 0.8) is
  1.09 Octane / Iray, with W3 (1.08, the same parts summing past 1) and
  W1 (1.07) at their positions; W5 / W3 0.86 in both renderers. Iray blends a full-strength glossy reflection by that share: as
  a metallic share of one Universal the reflection took the diffuse colour
  — Calib-Mixing (Fetroom's surfaces): weighted grey 0.63 → 1.05, ceiling
  0.85 → 1.06, door jamb (black, 0.8 coat) 1.09 once the coat covered the
  metal too (0.55 without). **With translucency** the same Mix (#128):
  `normalized_base` mixes the same Fresnel-less gloss against the diffuse
  part, the translucency inside it (`diffuse_base`) and the dual lobe over
  it (`dl_diffuse_bsdf`); Glossy Reflectivity reaches neither (DAZ hands
  none under Base Mixing 2, #132). The translucent path had put the gloss
  on `specular` as a dielectric (with a dual lobe the coating over it):
  Calib-DualLobeWeightedTL's gloss spheres rendered 0.25 – 0.41 × Iray and
  red where Iray's are near white (the gloss lost), the lobe alone 1.00,
  the Mix twin without translucency 1.10. As the Mix every sphere is 0.99
  – 1.11 × Iray where it stands, highlight spread and colour Iray's (G / R
  ±0.04, translucency 1 0.07): gloss alone 1.03 (W1 1.05), both 1.05 (W3
  1.07), Diffuse 0.5 / Glossy 0.3 1.08 (W5 1.08), Top Coat 1.09 (W6 1.11),
  Glossy Reflectivity 0 1.08 = both. The diffuse part's translucency is
  its transmission (Translucency, above), the map unscaled (#131);
  counting the gloss in a map brightening too had put it there twice
  (Calib-WeightedTL W6 / W7's highlights 0.38 / 0.35 → 1.05 / 1.00 × Iray,
  #128). Base Mixing 1 with the wall
  map as Diffuse and Glossy Color renders like a flat one (1.12 – 1.14).
  **A dual lobe under weighted mixing** (#121) belongs to `material2`:
  DAZ's `normalized_base` mixes the diffuse with the dual lobe over it
  (`dl_diffuse_bsdf`, the same `custom_curve_layer` as in Base Mixing 0)
  at Diffuse Weight against the gloss at Glossy Weight, and DAZ hands the
  lobes over (Calib-Mi-Mix2's M3). So `material2` takes it as a dual lobe
  without a glossy layer (lobe 1 on `specular` at lobe 1's roughness,
  isotropic; lobe 2 the `coating`; under a Top Coat one lobe at the
  blended roughness), the gloss stays `material1`'s alone. It was dropped
  (`specular` 0) until #121. Calib-DualLobeWeighted (enhanced-final,
  Octane / Iray per sphere where it stands): gloss alone 1.05, dual lobe
  alone 1.12 (= Calib-DualLobeGlossy's G2, the same material there), both
  1.03 – 1.11 with any share, a Top Coat, lobe reflectivity 1 or dual lobe
  W 0.5, highlight spread and colour as Iray's (±0.2, G / R ±0.015).
  **Refraction under weighted mixing** (#154) belongs to `material1`, the
  gloss part: `normalized_base` lays the refraction (`refraction_base`
  with the reflection on and full: a plain glass) at Refraction Weight
  over the glossy BSDF, inside the share Glossy Weight takes, and mixes
  the diffuse part beside it whole (DAZ's `.mi`: the diffuse stays on at
  Refraction Weight 1, the glossy goes off; Calib-WeightedRefr, -Thin,
  Calib-Mi-WeightedRefr2, -3). So `material1` is the glass alone at
  Refraction Weight 1 - Universal at refraction 1 with the gloss at full
  weight: `transmission` and `specular` in the glossy colour (× the
  gloss scale), the Refraction Index, no albedo - or a Mix at the
  Refraction Weight (its map) of that glass and the metal below 1;
  `material2` is the diffuse part without refraction (diffuse whole,
  translucency, dual lobe). The export had put the refraction in
  `material2` without reflection (diffuse × (1 − r), `specular` 0) and the
  metal mirror at the full gloss share: at Diffuse 1 / Glossy 0.5 /
  Refraction 1 a third mirror over a reflection-less glass where Iray
  has 2/3 diffuse, 1/3 glass. **Diffuse Weight 0**: DAZ switches the
  diffuse off and the gloss part renders alone at full strength, so
  `amount` 1 and scale 1 whatever Glossy Weight (U1 / U2; it had taken
  the gloss at Glossy Weight). Not yet confirmed against Iray (parked
  Calib-WeightedRefr / -Thin, #154).
  **Backscattering under weighted mixing** (#155) belongs to the gloss
  part too: `normalized_base` lays `backscattering_glossy_reflection_bsdf`
  at Backscattering Weight over the glossy BSDF (`weighted_layer`), under
  the refraction, so it shares Glossy Weight's share and the diffuse part
  keeps none. The glossy metal becomes a Mix at the Backscattering Weight
  (its map) of the retro-reflector (Backscattering, below, its colour ×
  the gloss scale) and the metal; at weight 1 the retro-reflector alone
  (DAZ switches the glossy off); under a partial refraction the glass
  Mix holds that Mix as its `material2`; a full refraction or Glossy
  Weight 0 has none (DAZ passes no `backscattering_enable`,
  Calib-Mi-WeightedRefr2 V5 / V6). It was dropped until #155. Derived
  from the MDL, awaiting Iray: Calib-WeightedBack.
- **Metallic Weight w → `metallic` w² / k, albedo and specular × k,**
  k = 1 − w + w² (a map: squared by twice its gamma, k from the
  channel's value); the glossy excess coating × (1 − w). Iray's metal is a
  coat at weight w inside a layer at weight w (DAZ passes w to both, its
  .mi export): w² metal + (1 − w) base. Universal's metal takes the
  albedo's colour, so the scale has to be shared. The dual lobe's
  `specular` takes the scale as the glossy layer's does.
- **Diffuse-only factors under a flat metal** (#109): what only Iray's
  diffuse loses — the head-on Oren-Nayar F(σ) (Diffuse Roughness, below)
  and the translucent metal's dimming — is a factor d on the albedo,
  which Universal's metal would take too. With the metal blend (m, k)
  (M = m k metal, B = (1 − m) k base; PBRSkin: m = w, k = 1) and
  S = M + B d: `metallic` M / S, albedo C d × S / d and the gloss × S / d,
  so Universal holds M C metal, B d C diffuse and B gloss, as Iray's
  layer stack (the metal over the diffuse). Scaling the albedo by d alone
  dimmed the metal to d (0.78 at Diffuse Roughness 1). **Under a
  metallic map per texel** (#123): each texel's w(x) (the map at its
  gamma, its value range, × the value) through the same rule, baked into
  two maps — `metallic` M / S and S, which multiplies the albedo and the
  gloss (`specular`) at power 1 / d (Multiply textures). Taken at the
  channel's value, one albedo scale couldn't follow the map (its
  unmetalled texels × S / d, 1.75 × at t 0.5), so until #123 the map's
  metal was dimmed with the diffuse.
- **Translucency on a metal** (not refractive - a refractive one: the Mix below -, Base Mixing 0): a Mix
  material (19) by the Metallic Weight (its map at its gamma × the
  value) of the metal (the material at weight 1: `metallic` 1, its
  colour) and the base (at weight 0: the translucency Universal's diffuse
  transmission as on any non-metal); Iray Uber's metal side a second Mix
  at w of the metal and a black Universal (its metal is a coat at w over
  black, above), so w² metal + (1 − w) base. Iray's metal is a
  `weighted_layer` over the diffuse and its translucency (DAZ's MDL), a
  per-hit mix like Octane's. One Universal can't hold both: its
  transmission swallowed the metal (P8's 0.75 rendered 0.23), and with
  no transmission the translucency's light had to be taken as lost (the
  diffuse × (1 − t), #109) - wrong where the base dominates: the light
  it passes into the closed body comes back out. **Under a
  backscattering Mix** (below) the surface is the same two Mixes at the
  shares the backscatter leaves it, M = m k metal and B = (1 − m) k base
  (`backscatter_shares`): outer amount x = 1 − B, metal layer y = M / x
  (at b 0 the plain (w, w)), per texel under a metallic map (baked from
  the map as the per-texel metal blend is); no constant (#145). **Base
  Mixing 1 and 2 have no metal**: DAZ drops Metallic Weight there, a map
  too (Calib-MetalTL-Back K8's .mi), so such a surface is a translucent
  dielectric; a map used to count as a metal and took the translucency
  as lost. What stays one node: the metal part itself (metal 1: the
  dimming cancels in the scale).

  Why: Calib-PBRSkin-MetalTL (enhanced-final, 41 px boxes at nine
  offsets around each sphere's centre, Y), metal 0.75 at t 0.85 / 0.5,
  metal 0.3 at 0.85, a white and a grey map (w 0.16) at 0.85: with the
  diffuse × (1 − t) (flat) and the fitted 0.35 t (maps) 1.04 / 1.04 /
  0.83 / 0.93 / 1.42 × Iray; the same rule per texel under the maps
  (d = 1 − t): white 1.01, grey 0.68 - the less metal, the more light
  was missing; as the Mix 1.06 / 1.05 / 1.05 / 1.04 / 1.04, beside the
  opaque metal 1.07, the translucent non-metal 1.04 and the grey map
  without translucency 1.10. Calib-PBRSkin's P8 (Gladio armour, metal
  0.75, t 0.85) 1.08 → 1.09 beside the opaque metal's. The Oren-Nayar
  part is derived from the layer stack, not rendered (no calibration
  scene combines a metal with Diffuse Roughness).
- **Refraction on a metal** (Iray Uber, Base Mixing 0; checked before the
  translucent metal): a Mix material (19) by the Refraction Weight (its
  map at its gamma × the value) of the refraction (the material at
  Refraction Weight 1 and metal 0: albedo 0, `transmission`, its IOR and
  gloss) and the opaque rest (at Refraction Weight 0: the metal over the
  base, a translucent one as the Mix above). DAZ's MDL
  (`metallic_layered_base`) lays the refraction over everything, a
  `weighted_layer` at Refraction Weight over the metal-over-base, so
  Iray refracts r whatever the metal and its metal keeps 1 − r.
  Universal's `transmission` belongs to its dielectric, which the
  metallic blend scales away: as one node the metal swallowed the
  refraction. A translucent non-metal under a partial refraction takes
  the same Mix (the layer is the same; one node kept only one of the two
  transmissions, #135: see translucency).

  Why (#129): Calib-MetalRefr (enhanced-final, Y over each sphere's disc,
  red metal base, white refraction): metal 0.75 / 0.3 at refraction 0.3
  / 0.7 as one node 0.47 / 0.18 / 0.94 / 0.69 × Iray, green / blue (the
  background through) down to 0.15; as the Mix 1.14 / 1.12 / 1.13 / 0.90,
  beside the metals alone 1.16 and the refractions alone 1.02 / 1.03, G /
  R as Iray's (±0.06). The -g0 (no glossy layer) and -TL (translucency
  0.5) variants alike (1.10 – 1.14) except the last sphere, which sits
  in front of the floor's edge (refraction 0.7: 0.67 / 0.95).
  Those are thin-walled shells; as closed solids (Calib-MetalRefr-Solid,
  projected discs, #146) the Mix renders X1 – X4 1.05 / 1.03 / 1.05 /
  1.03 × Iray beside the metals 1.11 / 1.13 and the refractions 1.05 /
  1.03, where the shells now measure 1.20 / 1.22 / 1.17 / 0.82 (#134;
  the shells' excess #164, X4's blur #136).

  Why: Calib-Map-Metal's red spheres at 0.25 / 0.5 / 0.75 went from 1.23
  / 1.38 / 1.39 to 1.12 / 1.16 / 1.22 × Iray (the full metal 1.19), the
  0.5 metal without gloss 1.51 → 1.06, a Reflectivity 1 metal 2.14 →
  1.28; Calib-DataGamma's metal 0.5 1.41 → 1.09; Calib-PBRSkin's 0.75 /
  0.9 metals 1.24 / 1.01 → 1.02 / 0.93. The metallic map at gamma 1
  (G5) 1.38 → 1.43 (#59). (Measured on discs centred on the highlight; on
  the projected discs, #146, the current export renders Calib-Map-Metal's
  seven metal spheres 1.13 – 1.15 × Iray, the specular-mode one 1.06.)
- **Metallic Weight under 0.05** (no map) is left out: the GSOV4 jacket's
  0.0107 turned Octane's gloss off — black velvet (0.02× Iray) beside the
  same leather's trousers.
- Base Mixing 1 ignores Metallic Weight, also over a black base: no metal,
  the gloss is the dielectric below (F0 = Glossy Specular). Why: DAZ hands
  Iray no metal there (the .mi: `glossy_reflectivity` = F0, the hue as a
  directional tint), and the gloss-coloured metal the export made of a
  black base under a strong gloss reflected the full Glossy Color —
  Morgan Hair's gloss alone (Calib-Strand, F0 0.49) on a mesh tube under
  the dome 2.27 × Iray, B / R 1.17 where Iray's is 0.85; as the dielectric
  0.98 ×, 0.83 (#79).
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
  the albedo × F(σ) (under a flat metal its diffuse part only, see
  "Diffuse-only factors under a flat metal"), the head-on albedo of
  Iray's Oren-Nayar:
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
  mapped. The flake Mix is the outermost of the Mixes under the coat (a
  weighted coat's Mix lies around it): a
  backscattering Mix lies in its surface (`material2`), so the
  backscatter takes (1 − flake share) of its amount.

  Why (backscattering): DAZ's `add_metal_flakes` wraps the whole base
  material (metal, backscattering, diffuse and gloss); a backscattering
  Mix outside the flake Mix kept its full share where flakes cover the
  surface (#122). Derived from the layer stack, not rendered: no
  calibration scene combines the two.

  Why: Iray's flakes (DAZ's MDL: a weighted glossy layer at weight × a
  flake noise, each flake with its own normal) take Calib-Flakes' spheres
  from 0.04 to 0.20; missing, Octane rendered them 0.12 – 0.36 × Iray, now
  0.88 – 1.12 (coverage 0.5: 1.05 – 1.35, 0.3: 0.72 – 0.88); Calib-Map-
  Misc's gold flakes 0.50 → ~1 (#30).
- **Backscattering** (Base Mixing 0 and 1, not under a full refraction;
  weighted mixing above) → a Mix material
  at amount = Backscattering Weight: the surface and a retro-reflector —
  a metal in Glossy Color (or Backscattering Color with Share Glossy
  Inputs off) × (2 + r²) / (2 + 2 r²), Beckmann (brdf 1) at roughness
  √(r / √(4 + r²)) for Iray's (squared) roughness r, whose `normal` is an
  OSL texture giving the direction to the viewer in tangent space (T =
  dP/du made perpendicular to N, B = N × T: the frame Octane reads normal
  maps in). A mirror about that normal sends every ray back where it came
  from; the top coat lies over both. Under a metal (Metallic Weight w,
  as Universal takes it) the amount is b (1 − w) for Backscattering
  Weight b and the surface's metal is rescaled so that the sum stays
  Iray's w² metal + (1 − w) b backscatter + (1 − w)(1 − b) base: the
  surface holds M = w² / (1 − a) metal and B = (1 − w)(1 − b) / (1 − a)
  base, i.e. `metallic` M / (M + B), albedo and specular × (M + B). A
  full metal leaves no Mix. Under a metal map the amount is b (1 − w(x))
  per texel, baked from the map like the surface's per-texel shares (it
  used to take the channel's value); a translucent metal's surface is
  the translucent metal's Mix at those shares (above, #145). Awaiting
  Iray: Calib-MetalTL-Back. **Base Mixing 1** is the same construction
  (#155): DAZ converts glossiness mode to `metallic_layered_base` and
  passes the backscattering (Calib-MetalTL-Back K7's `.mi`), its
  roughness from 1 − Backscattering Glossiness, no metal (DAZ drops it);
  the export had left it out ("not calibrated there"). Awaiting Iray: K7,
  Calib-WeightedBack's tie spheres. **Under a partial refraction**
  (0 < Refraction Weight < 1, #157) the refraction's Mix (refractive
  metal, above: the refraction part at Refraction Weight, no metal, no
  backscattering) lies outside, and this Mix is its base part (at
  refraction 0), so the lobe keeps (1 − refraction) of its share: DAZ
  passes `backscattering_enable` there in both modes, closed or
  thin-walled, with a metal or translucency (Calib-Mi-RefrBack(-Shown),
  Calib-RefrBack `.mi`), and `metallic_layered_base` lays the refraction
  over the metal over the backscattering. A full refraction switches it
  off (#154). Until then the export dropped the lobe under any
  refraction. Awaiting Iray: Calib-RefrBack.

  Why (metal): DAZ's MDL lays the metal (`bsdf_w_metal`) over
  `bsdf_w_backscattering`, so a metal hides the backscattering as it
  hides the base; an outer Mix at b showed it at full weight over a
  metal (#108). Derived from the layer stack, not rendered: no
  calibration scene combines the two.

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
  and σs as DAZ passes them (−ln(SSS Color) / d chromatic, SSS Amount
  clamped to 0 – 1 / d mono, d in m), unscaled; the phase a Schlick node
  (60) at g = **SSS Direction** (`add_uber_volume`'s `anisotropic_vdf`
  bias); `fake_shadows` **off** on that material (glass with absorption
  alone keeps it). Why: with fake shadows every scattering vertex's
  shadow ray took the light in straight, past the refraction and the
  inner Fresnel / total reflection; in a white furnace the spheres
  rendered 1.2 – 1.5 × DAZ's MDL (a Monte Carlo of the sphere,
  `refr-volume-mc.py`), more the more they scatter. Without them Octane
  is the MDL to 1 – 2 % on a grey volume (Calib-Furnace-RefrVol-Grey,
  -SSSColor; g ±0.6 / 0.4 included); a saturated absorption loses
  ~10 % in its weak channels (Octane is spectral, as #87's props).
  Calib-Map-Misc's X7 0.94 × Iray (0.88 / 0.98 / 0.87). The σs × 0.3
  fitted on X7 before (#28) stood in for its missing g 0.4 (~1.35 ×)
  and the fake shadows (#104). Translucent props without refraction
  bake their volume into the albedo instead (**Translucency**, #87); SSS
  Reflectance Tint isn't passed to Iray by DAZ at all (the .mi export),
  so it has nothing to map. Chromatic SSS scatters whenever its SSS
  Color is set, SSS Amount 0 included (DAZ's `sub_surface_enable`, the
  Calib-PropVol-* .mi); mono by its amount.
- **Refraction** (glass, water) → `transmission` = Refraction Weight ×
  the refraction's colour, IOR on `ior` (Dielectric IOR — Universal has no
  `index4`, probe 82; `ior` 2.0 brightened Calib-Materials' gloss 1.08 →
  1.15), transparent shadows. Iray's refraction takes the glossy layer's
  colour and roughness while **Share Glossy Inputs** is on (the default),
  its own Refraction Color / Roughness (1 − Refraction Glossiness in Base
  Mixing 1 only: Weighted (2) reads Roughness, Calib-Mi-Mix2's M6 0.4²,
  #124; a brick graph keeps Glossiness, as its gloss and coat do) when off: shared, a green Refraction Color and roughness 0.3 changed
  nothing in Iray. Its reflection is the glossy layer's: under a partial
  layer (Glossy Layered Weight < 1) DAZ weights it by that weight
  (`full_refraction_reflection_enable` off, #120), as Universal's specular
  already does (× the glossy weight). Without a glossy layer it reflects
  nothing in Base Mixing 0 and 1 alike (`specular` 0): both go through
  `metallic_layered_base`, which hands `refraction_base` DAZ's
  `refraction_reflection_enable`, and DAZ passes it off at Glossy Layered
  Weight 0 - closed, thin-walled or partial (Calib-Mi-RefrNoGloss N1 - N4,
  #143). Glossy Specular black at layered weight 1 keeps the full Fresnel
  reflection at the Refraction Index (N8: the flag on, `glossy_enable` off
  under the full refraction). **Weighted mixing (2) without gloss has no
  refraction at all**: `normalized_base` would reflect whatever the flag
  (it calls `refraction_base` with the reflection fixed on), but it holds
  the refraction in its gloss part, and at Glossy Weight 0 DAZ passes no
  `refraction_enable` / `refraction_weight` and leaves the diffuse on - a
  diffuse surface in Iray, and so in the export (`refraction_weight`, from
  `crate::iray`; N5 - N7, Shown twins alike). A mostly refractive surface (weight ≥ 0.5) with its own
  roughness gets it on Universal's one `roughness`, squared as usual.
  **Rough glass** (refraction ≥ 0.5, not thin-walled; shared or own
  roughness) → `brdf` 1 (Beckmann) at the **Phong pin √(r / √(1 + r²))**
  (`phong_pin`, r = Iray's roughness after its squaring; √r until #95,
  0.6 against 0.58 at r 0.36), at any roughness. Why (DAZ's MDL, #72): Uber refracts through
  `simple_glossy_bsdf` — Phong of exponent 2 / r², v-cavities shadowing —
  with no multiple-scattering compensation (`multiscatter_tint` 0, so its
  rough lobes lose light, MDL spec 21.1.3). Phong of exponent n is
  Beckmann at α = √(2 / (n + 2)) = r / √(1 + r²), and Octane's roughness
  is √α: no fitted constant. GGX energy preserving (the gloss rule above) gives
  back the light Iray's lobe loses: Calib-Dispersion-Rough's spheres at
  r 0.36 1.19 (shared) / 1.14 (own, then at Octane's default BSDF × 0.3,
  a Calib-Glass R6 fit, too sharp) × Iray; plain GGX loses more than Iray
  (0.86 / 0.85, its long tail); Beckmann at √r 1.09 / 1.07 with Iray's
  blur (detail 1.4 / 1.6 vs Iray's 1.2 / 1.5; at r 0.09 8.3 / 5.4 vs
  8.3 / 5.4; Calib-Glass R6 6.4 = Iray, was 6.7), the clear spheres
  1.02 – 1.04 as before. What is left at 0.36 (~5 % over the scene's
  1.03) is Smith vs v-cavities shadowing; Octane has no v-cavities.
  **Its level is derived** (`uber_glass_level`, #118): the transmission
  colour × k per channel, k solved so a closed sphere of the material in
  a white furnace keeps under Octane's model what it keeps under
  `libbsdf`'s (`mdl_layer::tint_factor`, as resin's, no measured term).
  Iray: DAZ's `refraction_base` — without a glossy layer
  `scatter_transmit`; shared inputs with the full reflection one
  `scatter_reflect_transmit` lobe; split (Share Glossy Inputs off, or a
  partial glossy layer) `fresnel_layer` of weight 1 (or Glossy Layered
  Weight) at the IOR, its reflection at the glossy roughness, over the
  transmit lobe; Phong + v-cavities. Octane: as exported — Beckmann at the
  Phong pin, Smith, specular the layered weight (0 without one). Not under
  a roughness map, not on thin walls. Why: the Uber's rough transmission
  *is* `libbsdf`'s, ring for ring (Calib-Furnace-UberGlass: Iray /
  `libbsdf` disk 0.99 at transmit r 0.354 / 0.6, reflect_transmit r 0.25 /
  0.5; #116, #117) — unlike NVIDIA's `mdl 1.0` modules, whose lobes stay
  measured (`ARCH_GLASS_LOBE`). Decomposed on its disk (r < 0.75), Octane
  / Iray before → after: r 0.218 1.01 → 1.01, r 0.354 1.04 → 1.02, r 0.6
  1.13 → 1.05, reflect_transmit r 0.25 1.03 → 1.02, r 0.5 1.14 → 1.06 (k
  0.997 / 0.990 / 0.964 / 0.997 / 0.964); Smith over `libbsdf` was 1.02 /
  1.10 / 1.01 / 1.08 of it. What is left is Octane over its own Smith model
  (1.01 / 1.01 / 1.01 / 1.04, #148) and Iray 1 % under `libbsdf` — not
  folded into k. Calib-Dispersion-Rough's frosted spheres (no Abbe, r
  0.36, inner disk): shared 1.09 → 1.06, own 1.07 → 1.05 × Iray (k 0.985 /
  0.990; under the HDRI, the clear ones 1.01 – 1.04 unchanged);
  Calib-Glass R6 (r 0.09) k 1.
  **Thin Walled** → `transmissionType` 2 (Thin wall), no volume. Iray shows
  a thin-walled glass sphere without any reflection — invisible; Octane's
  thin wall keeps its Fresnel reflection, kept as the more plausible
  (Enhanced; Faithful writes IOR 1, `thin_glass_ior_one`). The IOR 1 takes
  the refraction's reflection only: under a partial refraction (< 0.5) in
  Base Mixing 1 the IOR is the glossy layer's F0 and stays. Why: Morgan
  Hair's strands (thin-walled, refraction 0.106) lost their whole gloss in
  Faithful - Calib-Strand-T2-Dome's C5 / D5 0.36 / 0.44 x Iray, with the
  gloss 0.62 / 0.56 as Enhanced (#79). Nor where Iray's refraction
  reflects (`refraction_reflection_enable`, i.e. a glossy layer): DAZ's
  `refraction_base` then reflects Fresnel at the Refraction Index,
  × Glossy Layered Weight when below 1, and a thin wall keeps the
  material's IOR for that (MDL spec 13.1: no bending, the IOR still
  counts) - Universal's `specular` weight at the Refraction Index, as in
  Enhanced. Only the glass without a glossy layer, which Iray shows
  without any reflection, gets IOR 1. Why: Genesis 9's eye moisture and
  tear (thin-walled, refraction 1, layered weight 0.4, IOR 1.38) lost
  their whole reflection in Faithful (#101).
  **The view through a rough thin wall stays sharp** — a known limit,
  not a rule: Iray blurs it, Octane can't (#136). DAZ's `refraction_base`
  transmits through `simple_glossy_bsdf` at the glossy (or its own)
  roughness whether thin-walled or not, and Iray 2021.0 dropped the × 7
  on the Phong exponent it used to give thin-walled transmission (release
  notes): Calib-MetalRefr's R2 (glossy roughness 0.2) shows the floor's
  back edge smeared away. Octane renders every thin-walled transmission
  unblurred (probe 136): Universal's Thin Wall at pin 0.2 and 0.5 (the
  manual: "no roughness for transmission"), Universal's Specular
  transmission at IOR 1 (microfacets between equal media don't scatter;
  at 1.05 it bends like a lens), the Specular material's `thinWall`,
  Standard Surface's `thinWall` with `roughnessExtra` 0.5, OpenPBR's
  `thinWall` at roughness 0.5. Calib-MetalRefr's X4 (0.88 × Iray, 0.67
  without the glossy layer, its neighbours 1.10 – 1.14) is that blur: it
  sits before the floor's corner, its deficit is in the quarter that
  sees the floor (0.78 / 0.55), and Octane's straight-through background
  Gaussian-blurred (σ 16 px, a check, not a rule) brings -g0's X4 to
  0.94 × Iray (pixel rms 17 → 5.5 ‰); R2's edge wants σ 24 – 32 px (its
  floor edge is farther behind: the blur is angular). Over an even
  background (R2, X2) the blur moves the mean by ≤ 4 %.
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
    reached Iray as the plain factor). Baked into one normal map (#85)
    they take that placement (they bake together only when theirs agree;
    a tangent-space normal tilts the same at any tiling, as MDL's unit
    tangents do); on Octane's own bump `bump_power` counts the map's tiles
    with the material's.
  How common (the user's 476 scenes, 110 with `image_modification`):
  invert in 80 scenes (Glossy Roughness 58, Top Coat Roughness 42 — mostly
  glossiness maps on a roughness channel), per-map tiles in 38, scale 29,
  grey mode 18 (overlay and make-up weights by alpha), tiling offsets 13
  (Calib-Skin's normal maps at 1: a whole period), offset 2.
- **Colour space.** Every map is read at the gamma Iray reads it with (Map
  gamma, below): 2.2 for an 8-bit map at "auto". Every gamma above 1 (2.2, an image's
  own `map_gamma`, a squared overlay weight map at 4.4) goes in "linear
  sRGB + legacy gamma" (6) with that gamma: Octane applies the `gamma` pin
  only there and ignores it in sRGB (Calib-Overlay's squared belts, #71).
  Why not Octane's sRGB space for 2.2: it decodes with the sRGB curve,
  not Iray's power — brighter below ≈ 0.1 linear (0.018 → 1.25 ×), 1 – 2 %
  darker above. Calib-Skin's dark skins (baked texels at 0.01 – 0.03):
  Monique 0.97 / 1.12 / 1.33 → 0.91 / 1.03 / 1.08, Twosret 0.94 / 1.16 /
  1.30 → 0.94 / 1.05 / 1.12 × Iray; Calib-Overlay's dark texels 1.06 →
  1.02; light maps (Calib-Map-*, -Mi-Maps, -Hair, -ColorEffect) move by
  1 – 2 % (#19). Every image
  node gets `initialColorSpaceSelected` = 1, and the HDRI stays linear. Why: without the attribute, data maps were decoded as
  sRGB and sunglasses came out nearly clear (probes 55–57). With it the
  lenses tint like Iray's, brightness unchanged (Octane2 moved 0.28 / 255).
  **A file's own colour metadata is ignored** (PNG gAMA / sRGB / iCCP),
  as Iray does: under `gamma "auto"` (MDL `gamma_default`) Iray's
  OpenImageIO reader gives a canvas its pixel type's gamma — 8 bit 2.2,
  half / float 1.0 — and reads `oiio:ColorSpace` only when the debug
  option `ignore_gamma_metadata_on_import` is 0 (default 1, MDL SDK
  2025 = DAZ Studio 6's Iray 2025.0.3; DAZ never sets it). Why (#90):
  the NVIDIA presets' PNGs all carry gAMA 1.0; read linear, asphalt's
  colour map would be 6 × brighter (Octane disk 5.4 × Iray), its gloss
  weight map at 1.0 put the disk at 1.17 × and the rim 1.26 × (at 2.2
  0.99 / 1.00; diffuse alone 0.83).
- **Map gamma: one rule for every map** (`Textures::map_gamma`, #93) —
  colour, float channel (roughness, opacity, metallic, translucency,
  glossy and top coat weight, reflectivity, displacement), bump and normal
  maps, the image pins and the bakes alike: the image's own gamma (below),
  else gamma "auto" by the file's pixel type
  (`daz_material::auto_gamma`), the rule of Iray's image loader (the MDL
  SDK's OpenImageIO plugin, `get_pixel_type_and_gamma`, 2025.0.3 as in
  DAZ Studio 6 and unchanged in 2026.0.2):

  | pixel type | "auto" |
  |---|---|
  | 8 bits a channel, any channels | 2.2 |
  | 16 bits, one channel | 1.0 |
  | 16 bits, grey + alpha, RGB, RGBA | 2.2 |
  | half / float (EXR, HDR, float TIFF) | 1.0 |

  The 16-bit grey's 1.0 is a loader debug option
  (`linear_gamma_for_single_uint16_channel`, default on). DAZ hands
  every image with `map_gamma` 0 to Iray as "auto", 16-bit and float
  files too (Calib-MapDepth, -MapDepth2 .mi: `g_Auto`; a 16-bit grey at
  `map_gamma` 2.2 goes at 2.2). The export reads the pixel type from the
  file's header; OpenEXR, Radiance HDR and a one-channel float TIFF,
  which the `image` crate doesn't decode, count as float. Octane gets the
  file as it is with that gamma (Colour space): it applies the legacy
  gamma pin to 16-bit PNGs as to 8-bit ones (Calib-MapDepth: a 16-bit
  grey at pin 2.2 displaced as the 8-bit map, within 0.4 px).
  Why (Calib-MapDepth, -MapDepth2 against Iray, 2026-11-10 note): a
  16-bit grey displacement map's edge at 500.2 px against Iray's 500.1
  (read at 2.2 before: 508.8); 16-bit RGB and 16-bit at `map_gamma` 2.2
  at Iray's edge (2.2). Diffuse maps Iray reads linear (16-bit grey,
  float TIFF, an 8-bit map at `map_gamma` 1) 0.45 → 1.03 – 1.04 × Iray;
  a weighted top coat's 8-bit weight map (a literal 1.0 before) 1.43 →
  1.04; 16-bit opacity 0.97 → 1.04.
  - Images the export writes carry the gamma they are written at (a
    derived roughness or IOR map 1, a skin or blend bake 2.2, a baked
    normal map 1); a grey channel pick (#77) keeps its source's depth
    and gamma.
  - The bakes decode their sources at the source's gamma: `derived`
    (roughness, IOR, the NVIDIA metals' maps) per channel value from a
    table at 8 or 16 bits; the colour bakes (skin, blends) take an 8-bit
    2.2 copy of a source Iray reads otherwise.
  - Layered images flatten to an 8-bit canvas (DAZ's composite): "auto"
    is 2.2 there.

  The float channels' maps follow it though the Uber channels declare
  `default_image_gamma` 1: DAZ hands an image with `map_gamma` 0 as "auto"
  (an image's own gamma wins):
- **An image's gamma** is what DAZ hands Iray for it
  (`daz_material::scene_image_gammas`, #91): its `image_library` entry's
  `map_gamma` (0 = "auto"); a file the scene doesn't list takes the
  `default_image_gamma` of the first channel that loads it — scene
  materials in order, a material's own channels before its
  definition's — and keeps it on every channel (DAZ has one image per
  file). Iray Uber's defaults: 1 for bump, normal and the float
  channels, 0 ("auto") for the colours. Why (Calib-Mi-ImageGamma's .mi,
  the maps unlisted): alone on the diffuse "auto", on Bump Strength,
  Normal Map or Glossy Roughness 1; one file on a diffuse and then a
  bump "auto" on both, the other way round 1 on both. The Calib-Skin
  spheres' cropped maps are saved without entries: read at 2.2 their
  normal map tilted the skin sphere to 0.47 – 0.51 × Iray (1.0: 0.95 /
  1.17, as before).

  Why (#49): on Calib-DataGamma a grey 128 map renders as a value of 0.22
  in Iray. The same grey at `map_gamma` 1 renders as the plain value 0.5.
  The sphere-centre ratio of the map to the plain 0.5 is:
  - opacity: Iray 0.78, export now 0.78 (0.83 at gamma 1);
  - metallic: Iray 1.36, now 1.25 (1.14).

  The eight Calib-Skin figures (front light) moved by at most 2 % (ears
  5 %). Bump and normal maps: their own rule (#91, above).
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
    has no Fresnel). The lobe is `simple_glossy_bsdf`, Phong of exponent
    2 / r² per axis = Beckmann at α = r / √(1 + r²), Octane's pin √α: the
    rough-glass rule, **Beckmann** (`brdf` 1) at **√(r / √(1 + r²))**
    (`phong_pin`) — no constant. Octane's roughness = the longer
    axis per texel, anisotropy = **1 − pin_u / pin_v** of the two axes'
    pins **per texel** (Octane's semantics, Anisotropy above) (a derived map when one map drives both axes, as
    in both presets; else at a mid-grey texel), the bump baked into a
    normal map (below), the shader's tiles (`mdl_metal`).
    Why (#92): on the Uber gloss's GGX curve (min(r + 1.85 r², √r), ≈ r
    for a sharp lobe, so α ≈ r²) Cast Iron (r ≈ 0.11) reflected the sky
    about 2.4 × too sharply. Calib-NVIDIA-CastFlat (Iray with Cast Iron's
    bump off) decomposed it: Iray's own bump moves the band-passed detail
    by 9 % (correlation 0.97 between the two references), while Octane
    without the bump was 1.52 × Iray's bump-off detail — the lobe, not the
    bump. Cast Iron's detail / correlation / disk, Octane / Iray: GGX curve
    1.30 / 0.76 / 0.95, Beckmann at the derived pin 1.04 / 0.997 / 1.07
    (bump off 1.00 / 0.996), plain GGX at the same pin 0.79 / 0.98 / 1.11
    (its long tail isn't Phong's). Metal Brushed (u 0.04 – 0.18, v 0.36 –
    0.48 on its map; its darkest texels draw Iray's thin vertical streak,
    which a mid-grey ratio blurred into one broad highlight): detail /
    correlation 1.10 / 0.95 → 0.95 / 0.99 (#92, then with (α_v − α_u) /
    (α_v + α_u); on Octane's own semantics (#96) 0.86 / 0.99). Its disk is
    1.12 × Iray (#92; 1.16 on Octane's semantics, #96: the short axis now
    as wide as Iray's; floor 1.04; 1.00 on the GGX curve, which had been
    tuned there). Its disk decomposes (#98) into 1.056, the rough lobe's
    energy (Iray's Phong with v-cavities keeps less than Smith-Beckmann at
    v ≈ 0.42, as on the white Uber metal at r 0.4, 1.063 head-on), and
    1.10, Iray darkening `metal_generic` towards the rim as about √cos θ_v
    (the disk mean of 1 / √cos over r < 0.75 is 1.095) — with every map
    flat and the bump off, under a uniform dome (Calib-Furnace-Brushed /
    -Metal); the Uber metal with the same lobe stays flat, and so does
    Octane. The same rim makes Cast Iron's 1.07. No Octane switch moves
    the disk (bump off, weight or lobe flat: 1.16 – 1.18). Both are left
    as Octane over Iray: the lobe's loss as for the Uber lobes (#95, #96),
    the rim because it isn't in the MDL (#111): the chain DAZ hands Iray
    (`pass_material` → preset → `metal_generic`) has no Fresnel, curve,
    directional factor or normalisation, and the differences from the
    flat Uber metal reachable through DAZ's channels don't darken the
    rim — a `file_bump_texture` layer normal (Calib-Furnace-MetalBump)
    nor the lobe's tint (Calib-Furnace-CoatTint). Nor the MDL version
    alone (#112, our own modules, Calib-Furnace-MdlVersion / -MdlLobe):
    Iray's legacy `simple_glossy_bsdf` of `mdl 1.0` modules (NVIDIA's)
    keeps 0.98 of the 1.3 / Uber lobe head-on and to the rim up to r 0.3,
    at r 0.4 0.98 → 0.93 at the rim, at r 0.7 / 1 0.91 / 0.83 head-on
    (disk 0.87 / 0.78) — about a fifth of the rim at Brushed's roughness,
    2 – 6 % on NVIDIA's metals' disks, left as Octane over Iray with the
    rough lobe's energy; the nested empty base adds nothing. Left: the
    1.0 lobe with the tint, the anisotropy or a varying roughness, and
    `metal_generic`'s structure at constants (Calib-Furnace-MdlGeneric,
    #112).
  - Dielectrics (concrete, concrete polished, asphalt, ceramic tiles small,
    granite black, porcelain, resin polyurethane matte, architectural):
    `df::fresnel_layer(ior, weight, glossy, diffuse)` → Universal albedo,
    specular = the weight (a map where the MDL weights or tints the gloss
    with one: concrete's bump map tints it, asphalt's and the tiles' bump
    map weights it — read at 2.2 like any 8-bit map, #90 —, polished concrete's colour map does — its diffuse is
    that map × the weight too), `ior` = the layer's IOR, the MDL roughness
    at the Phong pin under the Uber gloss's `brdf` rule (Beckmann, energy-
    preserving GGX past α 0.25; see BSDF model and roughness), the bump
    baked into a normal map (below; `mdl_dielectric`). Porcelain (r 0.1):
    band-passed detail 0.75 → 0.88 × Iray's, correlation 0.93 → 0.99 (on
    the GGX curve before #95). Granite's varnish is a
    mirror (roughness 0). Resin's base is a tinted glossy reflect +
    transmit (`simple_glossy_bsdf`, `scatter_reflect_transmit`): specular
    and transmission its colour, which Octane, like Iray, applies at every
    crossing — that keeps the hue — under the rough-glass rule (Beckmann at
    the Phong pin; the clear gloss as the coating at its Phong pin), both ×
    **k per channel** (`resin_lobe_albedo`, `mdl_layer::tint_factor_measured`):
    the factor that makes a closed sphere of the material, entered over its
    whole silhouette in a white furnace, keep in Octane (Beckmann α = pin²,
    Smith masking, coat w · F(k·h) over the base's 1 − w · F(k·h)) what it
    keeps in Iray, both traced at export (65 536 paths each, fixed seed; k
    solved on the same paths). Iray's model is the MDL SDK's `libbsdf`:
    `simple_glossy_bsdf` = Phong half-vector distribution (exponent 2 / r²)
    with v-cavities masking, min(G1, G2) on reflection, max(G1 + G2 − 1, 0)
    on refraction; `fresnel_layer` picks the coat with probability
    w · F(n·k1), weights it w · F(k·h), and weights the base **1 − w ·
    max(F(n·k1), F(n·k2_refl))** — Fresnel at the macro normal, k2_refl the
    reflection of k1 about the base's microfacet. Light leaving a rough
    base from inside meets k2_refl past the critical angle (F = 1): the
    base keeps 1 − w there, a loss that grows with roughness. **On top,
    Iray's base lobe loses more than `libbsdf`'s** — the module's `mdl
    1.0`: Iray reads `simple_glossy_bsdf` with a legacy behaviour in
    modules below 1.3 (our own 1.0 module's reflect_transmit lobe renders
    Calib-Furnace-Resin2's U3 – U6 to 0.003 per ring, its 1.3 twin is
    `libbsdf`'s, as the Uber's 1.7 lobe; Calib-Furnace-MdlResin, #117).
    The legacy loss is per surface event, grows with roughness and towards
    grazing, and isn't total internal reflection or a rougher lobe
    (note 2026-11-14-legacy-transmit-lobe); not documented, so what the lobe
    alone (no coat, white) keeps in the furnace over the sphere's inner
    disk is measured (`RESIN_LOBE`, by MDL roughness: 0.971 / 0.963 /
    0.920 / 0.678 / 0.462 / 0.324 at r 0.02 / 0.1 / 0.25 / 0.5 / 0.75 / 1,
    Calib-Furnace-Resin2; `libbsdf` 1.000 / 0.999 / 0.973 / 0.824 / 0.655 /
    0.531), and the `libbsdf` model meets it with an extra tint τ on every
    base event before the coated stack is traced. Grey 0.79, coat 0.95,
    IOR 1.49: k 0.98 at r 0.02, 0.92 at 0.1, 0.81 at 0.25, 0.59 at 0.5;
    uncoated white 0.99 / 0.98 / 0.97 / 0.89. Why: Calib-Furnace-Resin
    (uniform dome) — Iray's rough coated resin keeps far less than the
    physical sphere (white r 0.25 0.72, physics 0.98; grey r 0.5 0.26,
    physics 0.56); Octane keeps what physics does (0.99 / 0.60 at the full
    tint, uncoated r 0.5 / 1 0.894 / 0.740 v 0.88 / 0.71); `libbsdf` gives
    0.76 / 0.28 — the coat's weights are most of the loss, the lobe's
    masking ≤ 1 %. With the lobe's measured τ the `libbsdf` layering
    predicts the coated spheres: grey r 0.1 0.605 v Iray 0.605, r 0.25
    0.473 v 0.473, r 0.02 0.627 v 0.623, white r 0.25 0.722 v 0.722 /
    0.737, grey r 0.5 0.240 v 0.257 — the "3 – 6 % below the model" of
    #113 was the lobe. Octane / Iray disk (r < 0.75), floors 1.02 / 1.04,
    `libbsdf` alone (#113) → with the lobe: Calib-Resin grey r 0.25 1.05 →
    1.00, smooth 1.07 → 1.04, white r 0.25 1.03 → 0.97, uncoated grey /
    white r 0.25 1.15 / 1.10 → 1.09 / 1.04, r 0.5 1.21 → 1.07, r 0.1 1.00
    → 0.96; Calib-Furnace-Resin2 uncoated r 0.02 – 0.25 1.03 – 1.04, r 0.5
    / 0.75 / 1 1.08 / 1.15 / 1.17; Calib-NVIDIA2 A7 1.00, A8 0.93;
    Calib-NVIDIA's red resin 0.95 (centre 0.83), g/r 0.24 v 0.19. Left
    (#117): the whole sphere matches, its spread doesn't — Iray darkens a
    rough coated resin's centre and rim differently from Octane, the
    uncoated r ≥ 0.75 stays 1.15 ×; a constant tint can't draw it. The
    former fit 1 − 0.64 r (`RESIN_LOSS`, on a mis-framed reference, #97)
    gave nearly the same k for the coated resin but darkened the uncoated
    one to 0.80 ×.
    Architectural (mental ray's mia_material): diffuse × weight,
    reflectivity × the reflection colour's maximum tinted by it, without
    "Use Fresnel Term" Schlick's curve from the 0° reflectivity as the IOR
    with that F0 (0.2 → 2.62), metal, transparency and cutout opacity;
    glossiness g → MDL roughness r by `base::architectural_gloss_to_rough`
    (MDL SDK `base.mdl`: √(2 · 2^(−4 − 14 g)), a mirror at g = 1, 0.35 at
    0) — what DAZ's .mi hands Iray. **The reflection is the Phong lobe at
    that r** (`phong_pin` under `phong_brdf`, as every `simple_glossy_bsdf`,
    Gloss above; the module's reflection is that df straight from the
    conversion). Derived, nothing fitted (#156). Why: Calib-NVIDIA3's
    glossiness sweep (#21) had found Octane pins 0.4 / 0.25 / 0.15 / 0.05 –
    0.1 at g 0.15 / 0.3 / 0.5 / 0.7 on correctly placed discs; read back
    through the Uber's former GGX curve (min(r + 1.85 r², √r), #56) they
    looked like the formula with half its slope (√(r · r₀), a second fitted
    rule on the first), while `phong_pin` of the formula itself gives 0.41
    / 0.29 / 0.18 / 0.11 and 0.58 at g 0 — the sweep's pins. Sun spot, run
    through the peak above 0.15 / 0.3 / 0.5 of (peak − disc median), px,
    Octane enhanced-final denoised v Iray: g 0.15 at the fitted pins 18×22 /
    12×14 / 9×10 at peak 1.47 × v 23×27 / 18×20 / 14×15, at the Phong pin
    identical, peak 1.07 ×; g 0.3 – 0.7 and g 0 within a pixel either way
    (peaks clip); level-normalised pattern error (σ 1.5 px) 0.24 / 0.22 /
    0.10 / 0.07 → 0.02 / 0.05 / 0.06 / 0.07, Calib-NVIDIA2's g 0.3 0.21 →
    0.06. Disk (r < 0.75) Octane / Iray, Calib-NVIDIA3 (floor 1.04) 1.05 /
    1.02 / 1.05 / 1.05 → 1.07 / 1.06 / 1.05 / 1.05; Calib-NVIDIA2 (floor
    1.05) g 0.3 1.02 → 1.06, g 0 1.00. Anisotropy in mia's semantic
    (`anisotropy_conversion` with `mia_anisotropy_semantic`) on the MDL
    roughness **as Iray renders it: u = r · a^¼, v = r / a^¼** on either
    side of 1 — the MDL SDK's `base.mdl` has that below 1 but u = r · √a,
    v = r / √a above, which Iray's compiled `base::` doesn't do — and
    **each axis at its Phong pin**, as the Uber's (#96). Octane's
    roughness the longer pin, `anisotropyTexture` 1 − short / long of the
    two pins (negative: stretched along u; #96's semantics, Anisotropy
    above), `rotation` 1 − Iray's. Why (#114, point light at the camera,
    Calib-Point-MdlAniso / -ArchAniso, Iray 2000 samples; run through the
    peak above 0.3 / 0.5 of it, px): our own module at MDL 1.0 and 1.3
    renders alike (iso r 0.2 21×20 / 17×16 v 21×20 / 16×16, u 0.4 v 0.1
    40×10 / 31×8 v 41×10 / 32×8 — the lobe reads u / v per axis, linearly,
    in NVIDIA's `mdl 1.0` as in 1.3), while the same module through
    `base::anisotropy_conversion` (mia, a 4) is 51×25 / 40×19 (2.0 – 2.3 :
    1, not 4 : 1); Architectural at g 0.15 / a 4 is 25×12 / 19×10 against
    its isotropic twin's 17×17 / 13×13 — u 1.47 r, v 0.70 r (a^¼ = 1.41),
    the SDK's would be 2 r / 0.5 r — and a 0.25 is 25×48 / 19×38 (u / v
    0.5, as both readings). Octane at this rule: see the lab note
    2026-11-14-mia-anisotropy (Calib-NVIDIA2's A5 within a pixel as
    before: at small r the pins are the former g · (u / v)^⅛ ones).
    Transparency t (`weighted_layer(t, transmit lobe, diffuse)`): diffuse
    × (1 − t), `transmission` = refr_color × t × **k per channel**
    (`arch_glass_albedo`, `mdl_layer::tint_factor_measured`, as resin's)
    through a transmit-only lobe — `specular` 0 (Octane still refracts) —
    at **r_t = 3.7 × architectural_gloss_to_rough(refr_gloss)**, at most 1
    (`arch_glass_rough`), and `ior` = Index of Refraction (refr_ior), under
    the rough-glass rule (Beckmann at √r); the reflection on the coating
    at √r of its own roughness and the reflection curve's IOR. k makes a
    closed sphere keep in Octane what it keeps in Iray in a white furnace:
    Iray = `libbsdf`'s layering — `custom_curve_layer` (Schlick's r0, r90,
    exponent; its base weight 1 − w · max(c(n·k1), c(|n·k2|)), no IOR) or,
    with Use Fresnel Term, `fresnel_layer` at refr_ior (the critical angle
    in the base's weight, as resin's) of weight reflectivity × the
    reflection colour's peak, the reflection at its roughness, over the
    transmit lobe at r_t (`scatter_transmit` refracts every sample, a
    totally reflected one reflects) — with the lobe's own loss measured
    (`ARCH_GLASS_LOBE`, by refr_gloss 0 / 0.2 / 0.4 / 0.6 / 1: 0.512 /
    0.614 / 0.938 / 0.964 / 1.000, Calib-Furnace-ArchGlass G5 – G1,
    reflectivity 0, refr_ior 1.5) — τ solved on the lobe at the IOR the
    table was measured at (1.5; resin's `RESIN_LOBE` at its Coat IOR
    1.49), then the stack traced at the material's refr_ior with it (#148:
    `libbsdf`'s closed lobe itself loses more with the IOR — total
    reflection, longer bent paths — and τ solved at the material's IOR
    absorbed that: k 0.81 / 0.85 / 0.93 at IOR 1.2 / 1.5 / 2, Octane /
    Iray disk 0.89 / 1.05 / 1.32 on Calib-Furnace-ArchGlass2 H6 / H2 / H7;
    held at 1.5, k 0.86 / 0.85 / 0.85 and 1.01 / 1.05 / 1.10). The rest
    splits into Octane over its Smith model (1.03 / 1.04 / 1.04, flat in
    the IOR) and Iray under `libbsdf` × τ (0.98 / 1.02 / 1.06): the
    legacy lobe's per-event loss met on `libbsdf`'s longer bent paths, not
    an IOR term (Calib-Furnace-MdlTir, #117) — τ as one constant per event
    holds the disk, its angular spread is still open; Octane = the coating at its IOR, the lobe Beckmann at
    α r_t, Smith, physical layering. Reflectivity 0, refr_gloss 0.6 / 0.4
    / 0.2 / 0: k 0.98 / 0.97 / 0.85 / 0.93 (Octane caps α at 1 too); 0.2
    under reflectivity 0.6 0.85, with the Fresnel term 0.73. Why: the
    architectural lobe loses far more in Iray than `libbsdf`'s at the
    formula's roughness (refr_gloss 0.2: 0.614 v 0.99), and that isn't
    Iray's `simple_glossy_bsdf` (#116): the same df through DAZ's Uber
    (Calib-Furnace-UberGlass: transmit at r 0.019 – 0.6, reflect_transmit
    at 0.25 / 0.5) keeps what `libbsdf` predicts within 1 % per ring, at
    Max Path Length 3 too (-PL3), while NVIDIA's architectural and resin
    with that df lose at every surface event (Calib-Furnace-ArchGlass2:
    two tinted events per path; thin-walled, without refraction, a flat
    0.51 / 0.39 at refr_gloss 0.2 / 0; IOR 1.2 / 1.5 / 2 at 0.2 head-on
    0.82 / 0.72 / 0.52) — not the Firefly Filter (off: the same), not
    light trapped inside (Calib-Furnace-Resin3 at Max Path Length 3 loses
    the share `libbsdf` predicts), not a roughness (no `libbsdf` roughness
    draws the rings, rms 0.05). It is the module's `mdl 1.0` (#117):
    Iray's legacy `simple_glossy_bsdf` below MDL 1.3 is `libbsdf`'s lobe
    times a per-event loss g(r, cos), the same with or without refraction
    and at every IOR (Calib-Furnace-MdlTir / -MdlTransmit: thin walls
    predict the closed spheres at IOR 1.2 / 1.5 / 2 to 1 – 3 %; not total
    internal reflection — a thin wall loses too); architectural's lobe is
    that legacy lobe at ~r_t, not at the formula's roughness (G3 = our 1.0
    lobe at r 0.2). The legacy behaviour isn't documented, so the
    lobe alone stays measured; with it `libbsdf`'s layer weights
    predict the coated spheres — refr_gloss 0.2 under reflectivity 0.6
    Schlick 0.657 v Iray 0.655, Fresnel term 0.421 v 0.416 — while
    Octane's coating keeps all (its furnace 0.895 for both, the lobe alone
    0.886; Smith model 0.87). The former 1 − 0.36 r_t (`ARCH_GLASS_LOSS`,
    fitted on Calib-NVIDIA3's HDRI at two refr_gloss values) was the same
    level at 0.2 but 0.51 × Iray at refr_gloss 0 and missed the Fresnel
    term's weights (1.75 ×). r_t (3.7 ×) is still the look's fit: refr_gloss
    0.6 and 0.2 (MDL 0.020 / 0.134) matched Octane's Beckmann at r 0.06 –
    0.09 and ~0.5; the reflection's curve would give 0.08 and 0.22, and at
    0.2 Iray's interior is an even frosted grey that only r ~0.5 draws —
    part of #116. Octane / Iray disk, floor 1.04 (furnace 1.02): Calib-
    ArchGlass refr_gloss 1 / 0.6 / 0.4 / 0.2 / 0 1.03 / 1.02 / 1.03 /
    1.07 / 1.08 (was 0.51 at 0), reflectivity 0.6 smooth 1.02, at 0.2
    1.06, with the Fresnel term 1.75 → 1.40 (furnace 1.45 → 1.17: the
    spread, #117); Calib-NVIDIA3 refr_gloss 0.6 1.02, 0.2 1.10, 0.2 under
    reflectivity 0.6 1.10; Calib-NVIDIA2's glass 1.05.
    **Enable Absorption** (`refr_falloff_on`, with transparency) → an
    absorption medium on the glass, σa = −ln(c) / d per metre
    (`arch_absorption`), as `nvidia/architectural.mdl` computes its
    `absorption_coefficient`: c the Absorption Color (linear) with
    Absorption Use Color on, else 0.3333; d the Absorption Distance (cm,
    in metres); d ≤ 0 lets nothing through; none when thin-walled. DAZ
    passes all four (Calib-Mi-Arch, the test checks σa); Octane darkened
    the 10 cm sphere and blacked out the 2 – 3 cm ones (sanity render, no
    Iray reference needed: the MDL states the medium).
    **thin_walled** (with transparency) → Universal's Thin Wall
    (`transmissionType` 2): the MDL's `material.thin_walled` — no bending,
    no volume (no absorption), the IOR still the layer's Fresnel (the
    coating keeps it, MDL spec 13.1) — and k solved on a thin-walled
    sphere: Iray = `libbsdf`'s thin wall (the transmission the reflection
    mirrored through the surface, both faces the outside's) with the lobe's
    loss measured on thin spheres (`ARCH_GLASS_THIN_LOBE`, by refr_gloss 0
    / 0.2: 0.386 / 0.505, Calib-Furnace-ArchGlass2 H5 / H4, reflectivity 0;
    above 0.2 the closed spheres' values stand in until measured, #149),
    τ solved over the whole silhouette (Iray's thin profile is flat to the
    rim; `libbsdf`'s at refr_gloss 0 rises 0.43 → 0.58); Octane = the
    coating over a wall that passes the ray straight on, unweighted (no
    roughness for Thin Wall transmission, #136 — a rough thin wall stays
    sharp). Reflectivity 0: k = √ of the thin sphere, 0.72 / 0.62. Why:
    the glass went out as a refracting solid and H4 / H5 rendered as the
    closed H2 / H3 (0.729 → 0.507 centre to rim v Iray's flat 0.52 / 0.39);
    now Octane / Iray 1.02 / 1.01 on the disk, 0.99 – 1.03 per ring
    (enhanced-final, denoised; #147), unchanged by #148 (τ on a thin
    wall doesn't depend on the IOR: its lobe never refracts).
    Calib-NVIDIA3 (Iray 5000, Octane 1024; floor 1.04), disk before →
    after: glossiness 0.15 0.97 → 1.05, 0.3 0.89 → 1.01, 0.5 1.01 → 1.05,
    0.7 1.04 → 1.05; glass refr_gloss 1 1.04, 0.6 1.06 → 1.01, 0.2 1.67 →
    1.02, 0.2 under reflectivity 0.6 1.42 → 1.05 (the roughness rules,
    #21; with #115's furnace k 1.02 / 1.10 / 1.10); hue within 0.04 (b/r).
    Calib-NVIDIA2 (floor 1.05): glossiness 0.3 0.90 → 1.02, 0 1.01,
    anisotropy 4 0.95 → 1.06, glass 1.46 → 0.98 (#115: 1.05).
    Calib-NVIDIA (Iray 5000 samples, Octane 1024; sphere disk luminance,
    Octane / Iray; floor 1.04): Metal Brushed 1.02, Cast Iron 1.11 (1.04
    with the bump baked, #89),
    concrete 1.00, polished 0.95, porcelain 1.09, granite 1.09, resin 0.99 (#113),
    architectural 1.07; Calib-NVIDIA2: asphalt 1.05, ceramic tiles small
    1.04 (floor 1.05); hue within 0.02 (g/r) – 0.04 (b/r) but resin.
    Porcelain's and granite's excess (centre 1.14) is the diffuse under a
    full Fresnel gloss, as on the Uber's glossy spheres (Calib-Glossy
    1.02 – 1.16), not theirs alone.
  - **Bump** (#89): the metals (`metal_generic`), concrete, polished
    concrete, asphalt and the small tiles build it as
    `base::file_bump_texture(bump_texture, bump_amount, uvw)` on the
    shader's tiling — the Uber's rule — so the map goes in baked into a
    tangent-space normal map on `normal` (`detail_normal`, the Uber's
    height-map rule above: MDL's central difference, factor = Bump
    Strength), decoded at its gamma like the Uber's (#91: "auto", 2.2
    for the presets' 8-bit maps); unreadable, Octane's bump at
    `bump_power`. Architectural's `bump` is a ready normal (`float3`) and
    DAZ's .mi shows no map wired in, so a Bump Strength map there keeps
    Octane's bump. DAZ's bricks have no `is_normaltexture` channel (the
    metals' normal-map mode). Polished concrete's MDL bumps only the
    diffuse (`weighted_layer(normal: bump)` under the `fresnel_layer`) —
    Universal's bump tilts both; the gloss on the coating, which keeps
    the plain normal, came out worse (below), so both stay bumped.
    Why: Octane's bump reads the height per texel (above); Cast Iron's
    2048 px bump drew it as sparkle. Against Iray (Calib-NVIDIA, Octane
    1024, floor 1.04; disk luminance Octane / Iray, and band-passed
    luminance detail in the sphere's centre box, Octane / Iray, with its
    correlation with Iray's): Cast Iron disk 1.13 → 1.04, detail 0.91 →
    1.05, correlation 0.82 → 0.91, its noise at 256 samples (against
    1024) 0.21 → 0.12; concrete disk 1.01 → 1.04, polished 0.97 → 0.99,
    detail 1.01 / 0.98 → 1.00 / 0.98, correlation 0.99 / 0.94 → 0.99 /
    0.95; Calib-NVIDIA2 (floor 1.05) asphalt disk 0.96 → 1.00, tiles 1.06
    → 1.06, tiles detail 1.02 → 1.04, correlation 0.87 → 0.90; the
    spheres without a bump unchanged (all with the heights linear). At
    2.2 (#91) Cast Iron's detail goes up (1.21 → 1.39 ×, correlation
    0.90 → 0.81, disk 0.98 → 0.94 in #91's windows; concrete 0.98 →
    1.02, polished 1.03 → 1.07). That measure doesn't decide the decode:
    in Iray the decode moves Cast Iron's detail by 6 %
    (Calib-NVIDIA-BumpG1, the maps at `map_gamma` 1: 0.94 ×, correlation
    0.98), while Octane's exceeded Iray's at either decode: the metal's
    lobe was too sharp, not the bump (#92, Metals above) — with the
    derived lobe Cast Iron's detail is 1.04 ×, correlation 0.997 at 2.2.
    A bake prefiltered to the sphere's footprint (512 – 128 px) renders
    as the bump off: the bump's whole effect is sub-pixel (7 – 14 texels
    a pixel) and Octane doesn't mip-filter the normal map (Info kernel:
    tilt RMS per pixel 0.0586 = the full-resolution bake at each pixel's
    UV), nor does Iray differ at texel scale (Calib-BumpGamma-Uber's step
    band: same tilt profile, integral 1.169 / 1.171), so no prefilter.
    Polished concrete's gloss on the coating: detail 1.58 ×, correlation
    0.84.
- **Layered images** are flattened into one PNG per stack.

### Skin — OpenPBR Surface (505), Final only

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

**Genesis 9's eye** (`eye_volume`: `is_eye`, PBRSkin, translucency > 0,
not thin-walled; one surface per eye, a closed ball) gets no transmission
but one albedo, the diffuse layer plus what the eye's volume sends back
through the translucency (`eye_volume.rs`, `Textures::eye_bake`, per texel
in linear light; flat colours without maps):

  A = (1 − t) B + t² T² P / (1 − (1 − t) B P)

t = Translucency Weight, B = the base colour × its map, T = Translucency
Color × its map, P = the share of light entering the ball that comes back
out on its lit half. Why: `pbr_skin.mdl`'s `diffuse_translucency_bsdf` is
a `weighted_layer` of the diffuse (1 − t) over a diffuse transmission (t,
tint T); what goes through enters `pbr_add_volume` (absorption
−ln(Transmitted Color) / distance, scattering −ln(SSS Color) / distance,
Henyey-Greenstein at SSS Direction; DAZ's `.mi` hands the colours linear)
and meets the same surface from inside: out with t T, back in with
(1 − t) B for another round. Each re-entry is Lambertian like the first,
so the rounds are a geometric series. P is a Monte Carlo of that ball at
export (fixed seed, 20 000 paths per channel, radius 1.3 cm: Genesis 9's
`l_eye` bone), no fitted constant; on Genesis 9 base 0.61 / 0.43 / 0.32.
As Universal's transmission the whole eye glowed milky (Gladiator, Julius
9); with the maps alone as albedo (the old bake) Calib-G9Eye's eye
rendered 2.4 – 3.6 × Iray (sclera).

Calib-G9Eye's variants (#66), Octane / Iray, enhanced-final, sclera
(two boxes) / iris: translucency 0 (diffuse alone) 0.92 – 0.97 / 0.90 –
0.92; `-t1-white` 1.14 – 1.17, 0.97 – 1.04 G / B, iris R 1.34; `-t08-white`
1.09 – 1.11, 0.97 – 1.00 G / B; `-t1` (the map in both crossings: T²; T
alone predicted 2.6 – 2.9) 1.20 – 1.25 R, 0.96 – 1.04 G / B; the eye with
its maps, gloss off (`-nogloss`) 0.97 – 1.08 / 0.88 – 0.96; as loaded
sclera 1.03 – 1.14. Red runs 10 – 25 % high where the maps are white or T
is 1 (red travels farthest in the ball: the eye's inner geometry, not in
the model, takes some; the maps' T² hides it on a real eye). The dual
lobe over the iris at its weight map (#133): `-bare` iris 0.88 – 0.92,
as loaded 0.88 – 1.00 (before, the gloss at full weight: 1.25 – 1.96).

**Faithful** (`SkinShader::IrayAlbedo`, #19): OpenPBR Surface with
`subsurface` 0 — the colour Iray's skin shows, as a plain diffuse, the
gloss as Enhanced's (`skin_gloss`, below). No random walk; the colour
rule is Enhanced's, with no fitted hue in either.

- **Colour**, baked per texel into `baseColor` (`skin_albedo`,
  `Textures::derived_skin`), the same as Enhanced's (below), from DAZ's
  MDL layers with **no fitted factor**:
  `a · D + b · Tc · Tc_low / (1 − k · D)` per channel (D the diffuse map
  × its colour, Tc the translucency map × its colour, `_low` low-passed):
  - the diffuse layer `a` = (1 − w) (`diffuse_base`'s `weighted_layer`);
  - the translucency lets in t · Tc, the volume (`add_uber_volume`) gives
    back its **random-walk albedo A(α)** per channel, and the way out is
    t · Tc again: `b` = t² · A (`random_walk_albedo`: Chiang et al. 2016's
    fit α = 1 − (4.09712 + 4.20863 A − √(9.59217 + 41.6808 A + 17.7126
    A²))², inverted; α the volume's reduced single-scattering albedo,
    `volume_albedo`);
  - light that comes back but meets the diffuse layer from inside goes
    back in: the series 1 / (1 − (1 − w) · D · A), `k` = a · A;
  - Base Color Effect's **shares** (`base_color_effect_shares` with the
    diffuse map's mean colour): `a` × m / max, t = 1 − (1 − w)·m (effect 0:
    t = w). The skin's map means (this one and `skin_walk_weight`'s
    Translucency Color mean) leave out only near-black texels, not what
    the faces' UVs don't cover (#82): over the coverage Monique's
    Translucency Color mean halves and her front renders 0.89 / 0.83 /
    0.94 × Iray (#86's step 8), Angel's effect-1 diffuse share drops
    10 – 15 %; which mean the held skin rules want is open in #82.
  Why: Calib-Skin's Iray variants split the skin into its parts. A white
  diffuse map (dwhite) raised Monique's skin 2.4 / 4.3 / 7.1 ×, a white
  translucency map (tcwhite) 6.7 / 10.7 / 13.6 ×; with the Octane renders
  of the same variants (E, D and Tc² per region from w 0, tcwhite and an
  albedo-0 render) the rule predicts Iray's base, w 0.5, dwhite, tcwhite
  and w 0 of Victoria, Monique and Zurie within 0.91 – 1.01, but Monique's
  base (0.98 / 1.19 / 1.36) and w 0.5. The fit it replaced — (1 − w)^1.3 ·
  D + t · 0.55 · Tc² · A / A(Victoria's red) — had the diffuse layer at
  half and the subsurface part 1.2 ×; they cancel on Victoria and nowhere
  else (dwhite 0.31 – 0.52, tcwhite 1.04 – 1.22 × Iray). A grey
  subsurface part (`volume_factor`) had left G 1.10 – 1.46, B 0.97 – 1.96
  × Iray; A(α) is what Enhanced's fitted `SKIN_HUE_OPENPBR` had found.
- Result, Octane / Iray, face and body medians R / G / B, front lamp
  (before, with the fit): Angel 0.99 / 1.07 / 1.10 (1.01 / 1.09 / 1.08),
  Victoria 0.90 / 1.00 / 1.00 (0.97 / 1.02 / 0.97), Kayo 0.89 / 1.03 /
  1.05 (0.93 / 0.95 / 0.91), Gabriela 0.90 / 1.00 / 1.00 (0.90 / 0.96 /
  0.92), Nishi 0.92 / 1.00 / 1.00 (0.94 / 0.95 / 0.94), Zurie 0.94 / 0.98
  / 1.00 (0.89 / 0.87 / 0.83), Twosret 0.94 / 1.16 / 1.30 (0.93 / 1.11 /
  1.23), Monique 0.97 / 1.12 / 1.33 (1.04 / 1.24 / 1.34), G8F base 0.94 /
  0.99 / 1.03 (0.80 / 0.82 / 0.86). With the maps at a pure gamma 2.2
  (Colour space, above; the dark skins' baked texels sat in the sRGB
  curve's bright toe): Angel 1.00 / 1.09 / 1.10, Victoria 0.91 / 1.00 /
  1.00, Kayo 0.89 / 1.00 / 1.00, Gabriela 0.92 / 0.98 / 0.97, Nishi 0.93
  / 0.99 / 0.97, Zurie 0.96 / 1.00 / 0.99, Twosret 0.94 / 1.05 / 1.12,
  Monique 0.91 / 1.03 / 1.08, G8F base 0.95 / 0.99 / 1.00. With the
  mapped gloss at its share (`skin_gloss`, below): Angel 0.98 / 1.06 /
  1.07, Victoria 0.90 / 0.98 / 0.95, Kayo 0.90 / 1.00 / 0.93, Gabriela
  0.91 / 0.97 / 0.94, Nishi 0.92 / 0.97 / 0.97, Zurie 0.96 / 1.00 / 0.98,
  Twosret 0.91 / 1.00 / 0.89, Monique 0.89 / 0.96 / 0.94; HDRI Monique
  1.00 / 1.00 / 1.08, Victoria 1.00 / 1.04 / 1.05. With the dual lobe at W
  and the IOR of its F0 per texel (`skin_gloss`): Angel 0.98 / 1.06 /
  1.07, Victoria 0.91 / 0.98 / 0.95, Kayo 0.90 / 0.97 / 0.94, Gabriela
  0.92 / 0.98 / 0.94, Nishi 0.93 / 1.00 / 0.97, Zurie 0.96 / 0.98 / 0.97,
  Twosret 0.91 / 0.97 / 0.90, Monique 0.89 / 0.96 / 0.89; HDRI Monique
  1.00 / 1.00 / 1.00, Twosret 1.00 / 1.00 / 1.07, Zurie 1.00 / 1.03 /
  1.03, Victoria 1.00 / 1.04 / 1.07. With lobe 1 at the GGX curve (now
  derived, `phong_pin`: within 0.04 of this):
  Angel 0.98 / 1.06 / 1.07, Victoria 0.90 / 1.00 / 0.98, Kayo 0.90 / 1.00
  / 0.95, Gabriela 0.92 / 1.00 / 0.97, Nishi 0.93 / 1.00 / 0.97, Zurie
  0.95 / 0.98 / 0.97, Twosret 0.91 / 1.00 / 0.95, Monique 0.89 / 0.96 /
  0.94; HDRI Monique 1.00 / 1.00 / 1.06, Twosret 1.00 / 1.00 / 1.07.
  Open (#19): the highlights ~10 % under Iray's (Gloss, below); under
  environment light every skin 10 – 30 %
  bluer and brighter (HDRI Monique 1.11 / 1.32 / 1.55, Victoria 1.00 /
  1.07 / 1.18, Calib-Eye 1.14 / 1.31 / 1.57, Calib-HDRI 1.15 / 1.26 /
  1.46; the fit's half diffuse had hidden it on the G8F skin: 0.99 / 1.13
  / 1.41) — Iray's skin, not a renderer's subsurface (the plain diffuse
  shows it); backlit ears red 0.10 – 0.21 × Iray (no light passes
  through).

**Enhanced, the Genesis 8 base skin** (mono SSS, Base Color Effect 1,
SSS Direction −0.65; #83): baked like every other skin (`skin_albedo`,
below), no own rule. Until #83 it kept a fitted tint — the diffuse map ×
½ · ((1 − w) + w · Tc) (`skin_tint`) × `SKIN_HUE` (1, 0.80, 0.62) × a lift
by w (`openpbr_skin_lift`: 1 at w 0, 1.3 at 0.5, 0.45 at 0.9) — because
the bake had rendered it 1.4 – 2.0 × Iray (Calib-Eye, before #19's
fixes). Decomposed: Faithful's bake of this skin (a plain diffuse) is
right (front 0.96 / 1.00 / 1.00, Calib-Eye 1.06 / 1.07 / 1.10); its
subsurface part is next to nothing (Translucency Color 0.30 / 0.03 /
0.01), and Iray's render doesn't move with SSS Direction (−0.65 → 0: 0.99
/ 1.00 / 1.00). Enhanced's random walk on the same bake came out 8 – 14 %
over Faithful, all of it the walk's anisotropy: at 0 the two agree
within 1 %. Octane / Iray R / G / B, fitted → derived: Calib-Skin-Base
front 0.86 / 0.75 / 0.69 → 0.95 / 1.00 / 1.00, back 0.86 / 0.75 / 0.67 →
1.00 / 1.00 / 1.00, HDRI 1.00 / 0.90 / 1.00 → 1.05 / 1.05 / 1.11; Calib-Eye 1.01 /
0.94 / 1.07 → 1.05 / 1.07 / 1.11, Calib-HDRI 1.04 / 0.96 / 1.06 →
1.09 / 1.07 / 1.10. `SKIN_HUE` had made environment light right and a lamp 25 – 30 %
short in G / B; derived, both within ~10 %. Its backlit ears came out
2.21 / 1.61 / 1.30 × Iray with all of the colour through the walk (0.89 /
0.81 / 0.75 under the fit), 0.86 / 1.00 / 1.01 with the walk weighted as
Iray's translucency (#86, the split below): Iray's volume on this skin
sits behind w · Tc = 0.14 in red.
- **`subsurfaceAnisotropy` 0**: SSS Direction is in the baked colour
  already (the volume's reduced albedo, `volume_albedo`); OpenPBR's walk
  reproduces the colour it is given as an albedo only without its own
  anisotropy (g −0.65 made it 8 – 14 % brighter, above). Nishi (g −0.5,
the only Calib-Skin figure with one): front 0.96 / 1.04 / 1.07 → 0.92 /
0.98 / 0.95, HDRI 1.06 / 1.09 / 1.16 → 1.03 / 1.03 / 1.08, backlit ears
0.60 / 0.65 / 0.70 → 0.90 / 0.53 / 0.39; the other seven have g 0.

- The subsurface's distance per channel = Iray's volume's **transport
  mean free path** 1 / (σa + σs (1 − g)) (`sss_radius`; σa = −ln
  Transmitted Color / Transmitted Measurement Distance, σs = −ln SSS Color
  — or the mono SSS Amount, clamped to 1 — / Scattering Measurement
  Distance, g = SSS Direction,
  the `anisotropic_vdf`'s Henyey-Greenstein bias; DAZ's MDL
  `add_uber_volume`), OpenPBR's definition: the largest as the distance
  (m, clamped 0.1 mm – 5 cm), the channels over it as the colour; the walk
  itself isotropic (`subsurfaceAnisotropy` 0, above).
  Why the reduced σs (#88, docs/notes/2026-11-08-sss-direction.md): with
  anisotropy 0 and the reduced albedo in the colour, the walk must be the
  similar isotropic medium (σs (1 − g), g 0) as a whole — the mean free
  path of the unreduced σs left a medium of the right albedo 1 / (1 − g)
  too long. Nishi (g −0.5, red 1.46 × too long) backlit ears' red 1.24 →
  0.88 × Iray (front 0.94 / 0.98 / 0.98, HDRI 1.07 / 1.03 / 1.08,
  unchanged); the G8F base skin (g −0.65, the walk only w · Tc 0.14 of it)
  ears 0.96 → 0.86, front / HDRI, Calib-Eye 1.05 / 1.07 / 1.10, Calib-HDRI
  1.07 / 1.06 / 1.08 unchanged. Similarity checked against Iray's own
  physics: a slab Monte Carlo (η 1.4) that reproduces Iray's g 0
  Calib-SSSThick slabs within 1 – 7 % passes 3 – 10 % (1 – 9 mm) more
  light with the HG phase at g −0.5 / −0.65 than through the similar
  isotropic medium. Skins with g 0 export byte-identical.
  Why: it was a diffusion length without the SSS Color, normalised and
  square-rooted against fireflies — Victoria's 1.09 / 0.46 / 0.42 mm for
  Iray's 2.56 / 0.15 / 0.17: red too short to pass an ear, green and blue
  too long. Backlit ears (Calib-Skin-*-back, Enhanced) red 0.21 – 0.51 ×
  Iray → 0.58 – 1.07, glowing red like Iray's (G / B 0.26 – 0.78: Iray's
  ears pass more of them); no fireflies at 256 samples. Front skin within
  ±0.06 but the dark ones' red (Monique 1.09 → 1.19, HDRI 1.12 → 1.27).
- The distance × **1.52** (`OCTANE_WALK_RADIUS`, measured, #86): Octane's
  walk at radius r is a denser medium than one of mean free path r.
  Calib-SSSThick (backlit slabs 1 – 15 mm of one volume, w 1, white Tc):
  Iray's red decays over 4.69 mm (diffusion theory for our σ: 4.4 mm —
  Iray's volume is that medium), Octane's at the mean free path over 3.38
  mm, about as radius^0.8; × 1.52 matches, the same for a volume of
  another albedo (A 0.81 for 0.46). The manual doesn't define the radius
  beyond "the distance light can scatter below the surface". Fronts,
  Calib-Eye, Calib-HDRI moved < 1 %; backlit ears' red 0.32 – 0.86 →
  0.45 – 1.27 × Iray (G8F 0.86 → 1.00; Kayo, Monique, Twosret still 0.45 –
  0.66, Nishi 1.27: open in #86; Nishi 0.88 with the reduced σs, #88,
  above). Also: at S 1 (w · Tc 1) the base colour
  fallback was 0 / 0 (NaN); now 0.

**Enhanced** (`SkinShader::OpenPbr`, see Look; #19): OpenPBR Surface (505;
its `radius` is the distance and `scale` the per-channel colour — the other
way round from Standard Surface), random walk. On Iray's **chromatic SSS** (`SSS Mode` 1,
the modern skins) and mono SSS alike its own
colour rule (`skin_albedo`, shared with Faithful), from Calib-Skin (eight Genesis 8 skins from pale to dark
on one body, Iray probes; see docs/notes/2026-10-28-skin-tones.md) and its
SSS variants (#28, docs/notes/2026-10-30-sss-volume.md); the Genesis 8
base skin too since #83 (above, docs/notes/2026-11-08-skin-constants.md).

- **Split between the diffuse and the random walk as Iray splits it**
  (#86, `skin_walk_weight`, docs/notes/2026-11-08-skin-walk-weight.md):
  `subsurface` S = max over the channels of w · Tc (the translucency's
  share, Base Color Effect's, × Translucency Color × its map's mean),
  `subsurfaceColor` = the volume's part of the colour below / S²,
  `baseColor` = the diffuse layer's part / (1 − S). Why: Octane's OpenPBR
  doesn't blend linearly as OpenPBR's spec says — probe 68
  (`octane-probe-sssweight.lua`, greys on Victoria's skin) renders (1 − S)
  of the base and S² of the walk (S 0.25 / 0.5 / 0.75: 0.06 / 0.23 / 0.54
  of the walk at 1, within 4 %; an ear's transmission alike), and the
  whole base when the subsurface colour is black. That is Iray's
  translucency, (1 − w) diffuse and the volume behind w · Tc in and out:
  with S = w · Tc the walk's colour in the strongest channel is the
  volume's own A(α), so it scatters (and passes an ear) as Iray's volume
  does. All of the colour through the walk (S 1, #83) had the front lamp
  at R 0.91 – 1.16, G 0.95 – 1.24, B 0.95 – 1.44 over the eight
  Calib-Skin figures (Monique 1.16 / 1.24 / 1.44, Twosret 1.03 / 1.11 /
  1.30 — the "dark skins lifted by the walk" of #19), split 0.91 – 1.00 /
  0.95 – 1.03 / 0.95 – 1.07; G8F front 0.94 / 1.00 / 1.00, HDRI 1.04 /
  1.05 / 1.11, Calib-Eye 1.05 / 1.07 / 1.10, Calib-HDRI 1.07 / 1.06 /
  1.08; backlit ears R / G / B: G8F 2.21 / 1.61 / 1.30 → 0.86 / 1.00 /
  1.01. Open (#86): the eight figures' backlit ears' red came down from
  0.58 – 1.03 to 0.32 – 0.83 (G 0.29 – 0.77 → 0.60 – 0.88, B 0.00 – 0.87 → 0.31 – 0.86): Iray passes
  more red through them than (w · Tc)² · its volume — needs Iray's back
  renders at another w and a white translucency map.
- **Colour, baked per texel** into one map for `baseColor` and
  `subsurfaceColor` (`Textures::derived_skin`, `skin_albedo`; split
  between them as above): Faithful's rule above, `a · D + b · Tc · Tc_low / (1 − k · D)`. Tc · Tc_low keeps
  the map's texture at its contrast (Tc² doubled it). With it (diffuse
  depth 64), front lamp: Angel 0.98 / 1.07 / 1.13, Victoria 0.89 / 0.97 /
  0.97, Kayo 0.95 / 1.18 / 1.30, Gabriela 0.90 / 1.02 / 1.03, Nishi 0.92 /
  1.04 / 1.07, Zurie 0.93 / 0.97 / 0.98 (0.87 / 0.87 / 0.82 with the
  fit), Twosret 1.00 / 1.33 / 1.75, Monique 1.26 / 1.80 / 2.14; HDRI
  Victoria 1.00 / 1.07 / 1.21, Zurie 1.04 / 1.08 / 1.15; the G8F skin
  (mono, not baked) unchanged. With the maps at a pure gamma 2.2 (Colour
  space): Angel 1.00 / 1.07 / 1.11, Victoria 0.90 / 0.98 / 0.97, Kayo 0.97
  / 1.15 / 1.18, Gabriela 0.92 / 1.00 / 1.00, Nishi 0.93 / 1.04 / 1.03,
  Zurie 0.94 / 1.00 / 0.98, Twosret 1.00 / 1.22 / 1.49, Monique 1.22 /
  1.62 / 1.91 — the dark skins still over through the random walk (a grey
  under 0.07 renders brighter than its value, above). With the mapped gloss
  at its share: Monique 1.13 / 1.39 / 1.55, Twosret 1.00 / 1.16 / 1.37,
  Kayo 0.95 / 1.12 / 1.15, Victoria 0.91 / 1.00 / 1.00, HDRI 1.00 / 1.06
  / 1.10, the rest within 0.92 – 1.10.
  With the dual lobe at W and its IOR per texel: Monique 1.09 / 1.29 /
  1.44, Twosret 0.97 / 1.14 / 1.23, Kayo 0.97 / 1.12 / 1.13, the rest
  0.90 – 1.10; HDRI Monique 1.18 / 1.25 / 1.49, Twosret 1.08 / 1.13 /
  1.40, Zurie 1.06 / 1.06 / 1.09.
- **No fitted hue or dark gain** (#19, docs/notes/2026-11-07-skin-hue.md).
  Until then the subsurface part took the grey `volume_factor` and the bake
  × `SKIN_HUE_OPENPBR` (1.16 / 0.83 / 0.68) to the power of the smooth
  colour's saturation ÷ 0.75 (falling to 0 at 0.45) × (smooth colour /
  0.3)^0.2. The hue was the volume's A(α) found by fitting; the dark gain
  darkened dark skins. Front light, Octane / Iray R / G / B, before →
  after: Angel 0.95 / 1.04 / 1.06 → 0.93 / 1.04 / 1.04, Victoria 0.99 /
  1.06 / 1.15 → 0.89 / 0.97 / 0.98, Kayo 1.09 / 0.97 / 0.97 → 0.98 / 1.12
  / 1.15, Gabriela 0.97 / 0.90 / 0.88 → 0.89 / 0.98 / 0.92, Nishi 1.01 /
  0.82 / 0.72 → 0.93 / 1.02 / 1.00, Zurie 0.81 / 0.83 / 0.84 → 0.87 / 0.87
  / 0.81, Twosret 0.97 / 0.97 / 1.39 → 1.00 / 1.27 / 1.66, Monique 1.18 /
  1.48 / 1.91 → 1.34 / 1.80 / 2.14; the worst channel's |log| median 0.17
  → 0.13. HDRI: Victoria 1.09 / 1.15 / 1.30 → 0.97 / 1.07 / 1.17, Zurie
  0.89 / 0.91 / 0.97 → 0.96 / 0.97 / 1.00. The G8F skin with Base Color
  Effect 0 (Calib-Skin-Base-front-bce0) 0.79 / 0.82 / 0.84. Open (#19):
  the dark skins come out brighter and bluer through the random walk than
  as a plain diffuse (Monique 1.04 / 1.24 / 1.34 there) — Octane's albedo
  inversion on dark colours, measurable with a probe on known spheres;
  backlit ears red 0.17 – 0.24 × Iray.
  With the diffuse depth 64 (Render modes; it had cut the bright red
  channel): Angel 0.97 / 1.04 / 1.04, Victoria 0.96 / 1.00 / 0.97, Kayo 1.00
  / 1.12 / 1.11, Gabriela 0.90 / 0.97 / 0.92, Nishi 0.94 / 1.00 / 1.02,
  Zurie 0.87 / 0.87 / 0.82, Twosret and Monique unchanged; HDRI Victoria
  1.08 / 1.09 / 1.17, Zurie 1.00 / 0.97 / 1.00; the G8F skin (Calib-Eye,
  -HDRI, -Skin-Base) unchanged. The dark skins' excess is not the depth: a
  grey subsurface colour under 0.07 renders brighter than its value (0.01 →
  0.020, 0.02 → 0.028, 0.04 → 0.044 at any depth and radius), and they are
  1.04 / 1.24 / 1.34 already as a plain diffuse (Faithful).
  Since the walk's weight S (#86) the dark skins sit with the light ones
  (#84): front Monique 1.00 / 1.00 / 1.07, Twosret 0.94 / 0.96 / 1.00,
  Kayo 0.93 / 1.00 / 1.00, Gabriela 0.94 / 0.96 / 0.96; HDRI over the grey
  sphere (the environment itself 1.04 – 1.07) 1.03 / 0.94 / 1.12, 0.99 /
  0.94 / 1.09, 1.02 / 0.98 / 1.06, 1.02 / 0.90 / 1.01 (Calib-HDRI's G8F
  skin 1.07 / 1.06 / 1.08). The excess was the walk at S 1; at S² ≈ 0.2
  its +0.01 floor is a few thousandths. No compensation.
- The **volume's albedo** (`volume_albedo`, the skin's A(α)): the
  reduced single-scattering albedo α = σs(1 − g) / (σs(1 − g) + σa).
  DAZ's MDL: σa = −ln(Transmitted Color) / d_t; σs = −ln(SSS Color) / d_s
  (chromatic) or SSS Amount / d_s (mono); colours linear. (The props'
  volume factor (Rd(α) / Rd(α₀))^0.5 × 0.7 was a fit and is gone: a
  prop bakes its ball, **Translucency**, #87.) The **mono SSS Amount is
  clamped to 0 – 1** wherever σs is read (`mono_sss_amount`: the skin's
  A(α), the walk radius, a refractive volume's σs, a prop's ball):
  DAZ hands Iray `sub_surface_scattering_amount` clamped (`src/iray.rs`,
  Calib-SSSColor's .mi: Amount 2 → [1, 1, 1]). Unclamped (#105) Amount 2
  had twice Iray's σs: Calib-SSSColor's mono-2 sphere 1.26 / 1.53 / 1.59
  × Iray, clamped 1.13 / 1.29 / 1.31 beside mono 0.5's 1.11 / 1.29 /
  1.22 (enhanced-final); on Victoria 8 the skin's A(α) predicts Iray's
  mono-2 subsurface part 1.05 / 0.90 / 0.96 (unclamped 1.11 / 1.18 /
  1.31), mono 0.5 1.04 / 0.93 / 0.94. On the skins Iray's SSS
  variants (Victoria / Monique / Zurie: SSS Color grey, blue, deep; mono
  at Amount 2 and 0.5) moved the subsurface part as Rd's root predicts
  within ~10 % (Zurie ~20 %). SSS Reflectance Tint did nothing on the
  figures (their Base Color Effect is hidden). The distances alone don't
  change α.
- `subsurface` = S × the diffuse map's **detail mask** m: D / D_low
  through a smoothstep 0.35 → 0.70 (1 on skin, 0 on brows and liner); the
  base takes the volume's part the mask leaves, (diffuse + (1 − m²) ·
  volume) / (1 − S · m). Why: the random walk bled the skin's light under
  dark brows into blue-grey smudges; Universal at w 0 kept them dark.
  (Calib-SSS's spheres at `subsurface` 0.5 rendering 0.67 – 0.73 of the
  colour was the S² of the split above: 0.5 + 0.25.)
- The low-pass (D_low, Tc_low) stays on the **UV islands**
  (`fill_padding`, #86): a map whose padding is a flat colour (four
  equal corners, any colour) gets it replaced by the islands' mean before
  the 32 px blur. Why: Monique 8's and Kayo 8's face maps pad with white,
  Twosret 8's with a grey 238; the blur pulled it into the ears' small
  islands, the mask read their rims as dark detail and took them out of
  the walk — backlit ears 0.66 / 0.66 / 0.44 × Iray's red where the same
  body, volume and white Tc gave Victoria 1.07. Now 1.02 / 1.10 / 1.11;
  fronts unchanged. Not done: the walk's weight per texel
  (w · Tc of the texel) — the walk enters at one texel and leaves at
  another, so a varying S passes mean(S_in · S_out) < mean(S²): Monique's
  front 0.97 / 1.00 / 1.08 → 0.89 / 0.83 / 0.93. S stays one per material.
- **Gloss** (`skin_gloss`): **roughness derived, `phong_pin`** —
  Iray's lobes (and Top Coat) are `simple_glossy_bsdf`, a Phong microfacet
  lobe of exponent 2 / r² (r the MDL roughness, the slider squared), as
  Beckmann α = r / √(1 + r²); GGX has the same peak at that α, and OpenPBR's
  roughness is √α (α = roughness²). Lobe 1, lobe 2 (the coating, from
  Iray's r2², as Universal's since #95) and the Top Coat; under a Top Coat
  the lobes blend in Iray's roughness. Why: Calib-Skin's Iray variants
  without bump or normal map (dlflatnb / gloss0nb, dlflatnn / gloss0nn)
  showed neither map widens Iray's highlight (no bump 0.120 → 0.136
  Victoria, no normal map unchanged) — the width is the lobe's. The dual
  lobe's peak without bump, Octane / Iray (Victoria, Gabriela, Nishi,
  Kayo): roughness as is 1.3 – 2.6, Universal's GGX curve 1.09 / 1.47 /
  2.16 / 1.49, its root 0.62 / 0.77 / 0.58 / 0.81, derived **0.84 / 0.90 /
  0.82 / 0.91**; energy 1.00 – 1.17. Iray's **dual lobe**
  as DAZ's MDL has it (`add_dual_lobe_specular`: both white lobes under 1
  − a Schlick curve, W · (F0 + (1 − F0)(1 − cos θ)^5), F0 = 0.08 ×
  Reflectivity × its map) — OpenPBR's `specular` at **W** (× a Weight
  map: Zurie 8's face) and **`ior` the IOR of that F0 per texel** (baked
  from the reflectivity map as n / 2 at power 2, `ior_pin`); lobe 2 the
  coating at W × (1 − ratio) with the same IOR; a **glossy layer** alike
  (its MDL custom curve: F0 = 0.08 × Glossy Reflectivity × its map, 1 at
  grazing, weight Glossy Layered Weight × its map): `specular` the weight
  (× a weight map), `ior` the F0's. The Genesis 8 base skin maps its
  weight; the skin path looked for a reflectivity map only, so its face
  shone at full weight: under sky light the gloss alone was the whole
  excess (Calib-Skin-Base-hdri, (O − gloss) / I 0.94 – 1.04), Calib-Eye
  1.16 / 1.31 / 1.55 → 1.06 / 1.07 / 1.10, Calib-HDRI 1.17 / 1.25 / 1.43
  → 1.07 / 1.06 / 1.08, Calib-Skin-Base front 1.02 / 1.11 / 1.21 → 0.96 /
  1.00 / 1.00, HDRI 1.15 / 1.27 / 1.46 → 1.06 / 1.05 / 1.12 (its grey card
  1.04 / 1.07 / 1.06). A **Top Coat** takes the coating, the
  lobes then one at their blended roughness; its colour map tints it
  (Twosret 8, Kayo 8: their gloss map), its roughness map goes per texel
  (`roughness_pin_named`), a weight map is read at 2.2 like any float
  channel's; **`coatingDarkening` 0** (OpenPBR's coat took the base to
  0.58 – 0.80 of Universal's; Gabriela's skin 0.66 → 1.01).
  Why: three errors on top of each other, each a map. The map at 2 × W
  dropped the channel's value (twice Iray's at Reflectivity 0.5); as a
  share of the specular (map × min(2r, 1) · W) it dimmed the grazing
  reflection with the head-on one (Twosret's gloss map, 0.05 linear, left
  it a twentieth of Iray's); lobe 2 was unmapped. The face's highlight
  (p99.5), Octane / Iray before → after: Victoria 0.363 → 0.269 / 0.270,
  Monique 0.101 → 0.071 / 0.070, Zurie 0.301 → 0.207 / 0.206, Twosret
  0.078 → 0.098 / 0.099, Angel 0.252 / 0.256; HDRI Monique 1.05 / 1.15 /
  1.28 → 1.00 / 1.00 / 1.00, Twosret 1.00 / 1.00 / 1.07, Victoria 1.00 /
  1.04 / 1.07 — the skins' "environment light 10 – 30 % over" was mostly
  this gloss reflecting the sky. The Top Coat is not what the Top-Coated
  faces' highlights were off by: Iray's own split (Calib-Skin's gloss0 /
  coat0 / dlflat variants; Victoria, Gabriela, Nishi, Kayo) gives the coat
  1 – 2 % of the gloss (Octane alike); the dual lobe's energy matches Iray
  within 0.99 – 1.15, its peak didn't (the roughness above). The face's
  highlight (p99.5) with the derived roughness, Octane / Iray: Angel 0.99,
  Victoria 0.87, Kayo 0.84, Gabriela 0.91, Nishi 0.91, Twosret 0.81,
  Monique 0.87, Zurie 0.94 (−12 … +30 % with the roughness as is, −15 …
  +9 % at Universal's curve, which looked closer only where bump happened
  to make up for it). The bump's share is gone since the height maps
  bake into normal maps (#85, above): the dual lobe (dlflat − gloss0,
  p99.5 over the face) with bump / without, Octane / Iray, Victoria 0.76 /
  0.83 → 0.83 / 0.83, Gabriela 0.85 / 0.89 → 0.88 / 0.90, Nishi 0.78 /
  0.82 → 0.79 / 0.84, Kayo 0.88 / 0.94 → 0.97 / 0.94 — the bump now
  narrows it as Iray's does; the rest (0.83 – 0.94 without bump) is the
  lobe's own. The full faces (p99.5 over the face box): Angel 1.11,
  Victoria 0.96, Kayo 1.01, Gabriela 1.00, Nishi 0.95, Twosret 0.91,
  Monique 0.95, Zurie 1.01.
- Dark skins show more pore texture than Iray's.
- Not a cause: Iray doesn't dim the gloss of translucent skin (highlight
  contrast on the dark foreheads is higher in Iray); the gloss alone ~0
  made faces flat.

### dForce hair — Hair material (147)

(`Tuning::strand_hair`: the Hair material with the pile albedo in every
variant but Faithful + Final, which writes the MDL's layers — both
below.)

The shader's colours, each its map's linear mean × tint, are taken at the
root (Hair Root / Root Transmission / Highlight Root Color) and at the
tip (Hair Tip / Tip Transmission / Tip Highlight Color) and **blended
along each strand**: a Gradient (49) on a W Coordinate (104) from the
root's albedo (W 0) to the tip's (W 1); Root To Tip Bias / Gain other
than 0.5 bend it — DAZ's `get_line_root_to_tip_blend` (Schlick's bias,
then gain; `basic.mdl`) sampled at 15 control points spaced along the
curve (≤ 1 % off at 0.2 – 0.8), linear between. Equal ends: a plain
colour. Why: DAZ hands Iray two `dual_lobe_hair` instances, root colours
and tip colours, under a `df::clamped_mix` weighted by that blend on the
strand's line coordinate (Calib-Mi-Hair / -Hair2's .mi: the tip colours
and bias / gain do reach Iray; bias / gain at 0.5 are left out as the
MDL's default). Roughness both ways from `base_roughness`. Backlit
strands glow through (Octane's TT lobe) where Iray's stay dark: kept, it
is Octane's more plausible take.

Not mapped on the Hair material: **highlight_roughness** (one roughness
for all lobes); **separation** (it tilts the two glossy lobes' normals
±separation along the tangent; Octane's `offset` tilts its one R lobe and
only dimmed the hair: 18 % at 0.2 on Calib-Hair-hl); **Anisotropy /
Anisotropy Rotations** (the fibre BSDF is anisotropic by construction);
**strength** reaches nothing in Iray (Calib-Mi-Hair2's .mi).

Until #79 the albedo was two fitted shares of the colours (0.046 × hair +
0.110 × transmission, the highlight at 0.15 per unit weight, #42) at 2×
strand width, the specular the glossy weight; in the Preview variants
that rendered Calib-Hair's crown / top tuft / side 0.68 / 0.85 / 0.68 ×
Iray, r / g 0.96 / 0.76 / 1.00 against Iray's 1.23 / 0.85 / 1.21.

#### The pile albedo (#79)

Octane's Hair (a fibre BSDF: its own reflection, the light through the
strand, the backlit glow) stays; its inputs come from the shader's MDL
instead of fitted shares:

- **what one strand scatters** (`hair_strand_albedo`): the two glossies
  reflect ρ = hw · highlight + (1 − hw) · gw · hair colour; the rest
  enters through the diffuse transmission, w = (1 − hw)(1 − gw), tinted,
  and meets the strand's far side from within, where the same layers
  send it out (w · tint again) or back in (ρ): A = ρ + (w · tint)² / (1 −
  ρ). Why tint²: a single thick prism in Iray returns 0.25 – 0.27 of a
  white one at tint 0.5, 0.36 – 0.37 at 0.60, 0.56 – 0.59 at 0.76
  (Calib-Strand-UberT-T3). On Iray's fibres (no tessellation) the
  transmission is lost: A = ρ.
- **`albedo`** = the reflectance of a deep pile of scatterers of that
  albedo (`pile_reflectance`: van de Hulst's similarity form, isotropic,
  s = √(1 − A), (1 − s)(1 − 0.139 s) / (1 + 1.17 s)) — Octane's Hair
  albedo is the colour after many bounces. Why: Iray's dense patch returns
  0.094 / 0.142 / 0.280 of its white one at A 0.25 / 0.36 / 0.58, the
  form's 0.059 / 0.092 / 0.182 × 1.54 – 1.59 throughout (3000 strands are
  not a deep pile: the white patch lets light out of its back); Octane's
  Hair on the same patch returns 1.3 – 1.9 × its albedo of its white one.
  Root and tip each, blended along the strand as above.
- **`specular`** 1. Why: it scales every lobe of the Hair material, not
  its reflection (at 0 the hair is black whatever the albedo); as the
  glossy weight (0.29 on Calib-Hair) it was the hair's overall brightness.
- roughness both ways from `base_roughness`, as before; width 1×.
- No albedo matches Iray strand by strand: one white Iray strand equals
  Hair at albedo ≈ 0.2 on a sparse patch and ≈ 0.7 on a dense one — the
  fibre BSDF forwards light where Iray's diffuse transmission spreads it.
  The pile is what a head of hair shows.

**Calib-Hair** (local): crown 1.45, top tuft 1.22, side 1.65 × Iray; r / g
1.25 / 0.86 / 1.27 against Iray's 1.23 / 0.85 / 1.21 (the fitted shares
at 2×: 1.01 on the crown). The hue is the derivation's; the level is
Octane's own (its reflection on top of the albedo, light forwarded
through the sides) and is left: subtracting the material's reflection at
albedo 0 (0.025 in albedo units) lowered it to 1.31 / 1.18 / 1.54 and
reddened the crown to r / g 1.32. Calib-Eye's lashes and brows read as
Iray's. Both Preview variants (direct lighting): 1.17 / 1.15 / 1.19 ×
Iray, r / g 1.13 / 0.83 / 1.12; Calib-Eye's frame 0.95 (Enhanced + Final
1.08).

#### Faithful + Final: the MDL's layers (#79)

`dual_lobe_hair.mdl` is no hair BSDF: `weighted_layer`(Highlight Weight,
a glossy in the highlight colour, `weighted_layer`(Glossy Layer Weight, a
glossy in the hair colour, the base)), the base DAZ wires in a
`diffuse_transmission_bsdf` tinted by the Transmission Color — plain
weights, no Fresnel, no diffuse reflection. Written as that
(`Ocs::hair_layers`):

- two **Mix materials** (19), `amount` = Highlight Weight, then Glossy
  Layer Weight; `material1` each a Universal metal in the layer's colour to
  the edge (as weighted mixing's gloss), `material2` what lies under it;
- the base a Universal with albedo and specular 0, **`transmission`** =
  the Transmission Color, `transmissionType` 1 (diffuse);
- every colour root → tip along the strand (the Gradient on W above);
- roughness: the MDL's (channel squared, `conditional_square_roughness`)
  through `anisotropy_conversion` with Anisotropy (default 1) on the Phong
  curve, as Uber's gloss; Cutout Opacity on each node; `separation` has no
  pin.
- **Fibres** (no tessellation): no transmission, a black base. Why: Iray
  drops a surface BSDF's transmission on its `hair` primitive
  (Calib-Strand-UberT, -HairW: exactly black; prisms pass it).
- **The glossies are GGX energy preserving** (`brdf` 6), full colour.
  Why: the lobe is sharp across the strand and flat along it (Anisotropy
  1), and Octane applies no anisotropy on curves (`rotation`, a negative
  `anisotropyTexture`: the same picture) — only the long axis's roughness
  shows, 1 here, where plain GGX loses half the light. Under the dome,
  where the lobe's direction averages out, the energy-preserving metal
  returns what Iray's lobe does: white gloss alone 0.81 / 0.89 / 1.06 on
  fibres, 1.28 / 1.12 / 1.61 on prisms (thick / sparse / dense;
  Calib-Strand-HairW-Dome / -T3-Dome against Uber's weighted gloss,
  Calib-Strand-UberG-Dome / -T3-Dome — the same MDL lobe, equal to the
  hair shader's on fibres within 4 %). **No share of the colour**: Iray's
  strands over a plain-GGX metal on curves return 0.09 / 0.16 (prisms,
  sparse / dense) under a 45° front sun, 0.33 / 0.46 under a side sun,
  1.54 / 1.25 under the dome — DAZ's prisms all face one way, so each
  flat face's fan meets the camera or misses it with the light. A share
  fitted to the front sun (0.10) rendered the dome's gloss 0.13 × and
  Calib-Hair's crown 0.84 ×, browner (r / g 1.13 vs Iray's 1.23).

Checked, Octane / Iray (linear luminance per object; Calib-Strand rows:
thick, sparse, dense, sparse vertical):

| | thick | sparse | dense | vertical |
|---|---|---|---|---|
| prisms, white transmission (vs UberT-T3) | 1.12 | 1.23 | 1.25 | 0.99 |
| prisms, transmission 0.5 | 1.16 | 1.10 | 1.07 | 0.85 |
| dome, prisms: white gloss alone | 1.28 | 1.12 | 1.61 | 0.43 |
| dome, prisms: gloss 0.5 over white transmission | 1.47 | 1.39 | 1.65 | 1.16 |
| dome, fibres: white gloss alone | 0.81 | 0.89 | 1.06 | 0.46 |
| dome, fibres: 0.5 of it | 0.92 | 0.92 | 1.06 | 0.46 |
| front sun, prisms: white gloss alone | (Iray ≈ 0) | 16.7 | 14.1 | 12.0 |
| front sun, prisms: gloss 0.5 over white transmission | 3.33 | 2.87 | 2.67 | 2.92 |
| front sun, fibres: whole shader at Calib-Hair's values | 1.88 | 1.62 | 1.64 | 1.45 |
| front sun, fibres: white gloss alone | 2.06 | 2.51 | 3.43 | 1.41 |

**Calib-Hair** (dForce Corporate Hair, prisms, HDRI-lit; local): crown
1.22 (R / G / B 1.20 / 1.23 / 1.22), top tuft 1.13, side 1.23 × Iray; r /
g 1.21 / 0.85 / 1.20 against Iray's 1.23 / 0.85 / 1.21 (the Hair material
on the crown: 1.01). The level is the transmission's (1.1 – 1.25 above): a
round curve covers what a prism's 0.87 does. Open (#79): under a single
hard light the gloss lands where a round rough strand sends it, not
where Iray's flat faces do — sun-lit hair's gloss is too bright head-on
(the front-sun rows) and vertical strands under the dome too dark (0.43 –
0.46); prism meshes would draw it. The clean references of the hair
shader itself on prisms wait on #94 (Uber stood in).

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
Specular Reflection Shift (degrees, default 3) → `offset` in radians, the
same sign: Octane's offset is radians (0.5 and 3 turned the crown yellow /
grey, 0.0524 stayed next to 0) and OmniHairBase hands MDL −shift, MDL's
positive angle being the opposite tilt of Octane's — the crown's Iray −
Octane difference followed +3° (correlation 0.52), not −3° (0.21) (#54).

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
Notes: [2026-10-29](../notes/2026-10-29-displacement.md),
[2026-11-13](../notes/2026-11-13-displacement-in-mixes.md).

- **Once, on the outermost material** (`open_sub_materials`, #144):
  whatever the nesting (weighted mixing, flakes, backscattering, the
  weighted coat, translucent or refractive metal, PBRSkin's Makeup), the
  outermost Mix (19) carries the displacement and no node inside it
  does; a plain surface's Universal / OpenPBR node carries it itself.
  A displaced surface under a Layered material (the anisotropic Top
  Coat, a coat over a glossy layer over a dual lobe) gets a Mix at
  `amount` 1 around it, `material1` the Layered material, that carries
  it.

  Why: Octane takes a surface's displacement from its outermost
  material alone (orbx variants of Calib-Disp's +5 cm sphere, 2026-11-13):
  on a Mix's sub-materials only it rendered undisplaced (91 px wide
  against 124 px), 5 cm on the Mix and 10 cm on its sub-materials or on
  an inner Mix rendered 5 cm, never added. The Layered material has no
  `displacement` pin (`material1`, `customAov`, `customAovChannel` and
  its `Layer N`) and ignores its base's: undisplaced. In the Mix at
  amount 1 (all `material1`) it rendered 124 px, its shading within
  0.2 / 255 of the plain node. Before, an anisotropic coat and a full
  Makeup over a metal in the base colour lost their displacement; the
  other Mixes wrote it twice or more, ignored inside (no calibration
  scene changed).

- **Amount.** Iray moves a vertex along its normal by Strength ×
  lerp(Minimum, Maximum, map) cm. Octane moves it by `amount` × (map −
  `black_level`), so:
  - `amount` = Strength × (Maximum − Minimum) × 0.01 m;
  - `black_level` = −Minimum / (Maximum − Minimum).

  Why: on Calib-Disp, Octane's sphere silhouettes lie on Iray's to the
  pixel. White over 0..5 cm grew by 5 cm, at strength 0.5 by 2.5 cm and at
  strength 2 by 10 cm. Black over −5..5 cm shrank by 5 cm, and white over
  −5..5 cm grew by 5 cm.
- **Map gamma** as every map's (Textures, map gamma, #93): an 8-bit map
  without a gamma of its own (`map_gamma` 0, as every displacement map in
  the user's scenes) at 2.2, though the channel declares
  `default_image_gamma` 1; a 16-bit greyscale map (common for
  displacement) linear. The image's own `map_gamma` wins.

  Why: on Calib-Disp-Map, grey 128 moved the sphere by 22 % of its range
  and grey 64 by 6 %, for an L PNG, an RGB PNG and a JPEG alike. With
  `map_gamma` 1 the same grey 128 moved it by 50 %. Calib-MapDepth: the
  same grey as a 16-bit greyscale PNG moved it by 50 % in Iray and the
  export (the top edge 500.1 / 500.2 px), 16-bit RGB and 16-bit at
  `map_gamma` 2.2 by 22 %.
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
(`GROUND_DROP`; Manual: `Ground Origin Y`; Auto: the lowest point of the
visible geometry, strands included, which DAZ hands Iray 0.001 cm lower -
`iray::DomeSetup::ground_y`, Calib-Env-Mi-GroundAuto-Low / -LowHidden, #119;
never above 0: floating geometry keeps the ground at 0, Draw Ground on or
off - Calib-Env-Mi-GroundAuto-High / -High-Off, #150;
instances' copies don't count - Calib-Env-Mi-GroundAuto-Instance: a copy
down to -60 cm, DAZ's ground at -0.001, #125; a hidden target is left out
with its copies, -InstanceHidden, #130), in the geometry
group. Why 1 mm under: exactly at a floor's height it z-fought (washed-out
planks, hard shadows); 1 mm under the floor covers it and outdoors the
shadow stays. Opacity = **0.89** × `Ground Shadow Intensity`
(`GROUND_SHADOW_OPACITY`): Octane darkens a full shadow to black, Iray keeps
the sky's fill (Octane2 umbra 11 % of lit sand). The scene detail's **Ground
shadow** checkbox (`no_ground`) leaves it out — the plane can't know about
holes in a floor.

**Known difference** (#69): Iray's ground is a matte object, opaque — the
camera sees the environment in the ray's direction there, never what lies
below it (Calib-L-Mi-Dome-turned: a Manual ground at 12 cm hides the floor
and the sphere's lower half). Octane's shadow catcher is see-through, so
geometry under a Manual ground shows. With Ground Position Mode Auto
(DAZ's default) nothing is below the ground and the two agree.

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
  [#33](https://github.com/vysokyj/orbis/issues/33), legacy 3Delight shaders
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
