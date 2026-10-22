# Octane export — DAZ scene → `.orbx` for Octane X

The normative description of what ORBIS exports and why; the project
overview is in [CLAUDE.md](../../CLAUDE.md). Background:
DAZ files [daz/files.md](../daz/files.md), Iray semantics
[daz/iray.md](../daz/iray.md), Octane behaviour
[octane/octane-x.md](../octane/octane-x.md), container
[octane/orbx.md](../octane/orbx.md), USD [octane/usd.md](../octane/usd.md).

Code: `src/`. Dev runner:
`cargo run --release -- export <scene.duf> -o <out.usdz|out.orbx>`
(`--preview`, `--no-ground`).

Calibration notes cite the reference scenes (*Octane1*: interior, spots +
HDRI; *Octane2*: outdoor, HDRI-lit) and Lua probe numbers; numbers are
linear-light unless marked sRGB.

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
{ "format": "orbis-scene-dump", "format_version": 1,
  "daz_version": "6.x.y.z", "scene": "/…/scene.duf",
  "content_dirs": ["…"], "builtin_mdl_dirs": ["…"],
  "nodes": [ { "name": "Genesis9", "label": "Noemi", "class": "DzFigure",
               "lod": 1, "material_groups": ["Torso", …],
               "vertices": [x, y, z, x, y, z, …],
               "polylines": [[0, 1, 2], …], "polyline_materials": [0, …] } ] }
```

- One entry per node with an object and a non-empty cached geometry (bones
  skipped). `vertices`: the **base mesh** from `getCachedGeom()` — every
  node's resolution level is switched to 0 and every cache rebuilt **before**
  any is read, then restored: a geometry shell copies its host's cached
  geometry, so a host restored early hands the shell its SubD mesh (72809
  instead of 18396 vertices for a G8F shell) — world space, cm,
  Y up, morphs and pose applied; flat `x y z`. The export refines it itself
  (SubD above), so the dump stays small and topology matches the `.dsf`.
- `lod`: the resolution level DAZ renders at (> 0 = refined).
- `polylines` / `polyline_materials`: only for geometry without facets
  (strand hair) — the rendered strands as vertex indices, and each strand's
  index into `material_groups`.
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
- Options dialog, remembered in `DzAppSettings("ORBIS")`: where the `.orbx`
  goes (beside the scene, or a folder — default `~/Documents/ORBIS`), fast
  preview, shadow-catching ground.

## Geometry

- Stage `upAxis = "Y"`, `metersPerUnit = 1`, `defaultPrim = "Scene"`; points
  converted cm → **meters** (Octane ignores `metersPerUnit`).
- One `Mesh` per visible node (points from the dump, topology from the
  `.dsf`); `Visible` = false nodes are left out.
- `GeomSubset` per material group; `primvars:st` indexed `faceVarying`.
- **Geometry shells** are rebuilt as DAZ does (host + grafts), rendering only
  the groups their `material_group_<group>_vis` channels switch on — the
  rest (typically opaque white) would lay a white body over the figure.
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
  `Line End Width`, at **0.4 mm per unit** (`LINE_WIDTH_SCALE` = 0.004 m).
  Why: read as mm the strands were sparse and see-through; a 1/2/4/10× sweep
  put 4× on Iray's fullness and colour (Octane2).
- A `Material` per (mesh, group) under `/Scene/Materials/<mesh>_<group>` (a
  constant `UsdPreviewSurface`) — overridden by the Octane material inputs
  below.

## Package

`.orbx` with `assets/scene.usdz` (stored, 64-byte aligned, own writer),
textures `assets/tex/NNNN_<file>` (incl. flattened layered images) and
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
├─ "DAZ 3D lights"      geometry group (3): Input 1 = USD output, Input 2… = spots, ground
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
  order mapped to Octane's (reverse naming); DOF aperture = focal /
  (2 · f-stop) when DAZ has DOF on, else 0. Render size from the scene.

## Render modes

`ExportOptions::octane_mode`; the scene detail offers Final / Preview.

| | Preview | Final (default) |
|---|---|---|
| kernel | direct lighting (24), GI diffuse, depth 2 | path tracing (25): diffuse 8, glossy 24, **scatter 128**, **GI clamp 20** |
| skin | Universal | Standard Surface + random-walk SSS |
| imager exposure | 1 × 2^(13 − EV) × cm² | **1.4** × 2^(13 − EV) × cm² |
| highlight compression | 1 − Burn Highlights | same |
| white balance / saturation | DAZ White Point × Scale / Saturation | same |
| denoiser | off | **on**, during the render (`denoiserOnce` off, `maxDenoiserInterval` 20) |

Why path tracing for final: direct lighting renders random-walk SSS grey and
isn't photorealistic; Cycles (the Blender reference) path-traces too.

**Imager calibration** (Octane2 vs Iray, 2026-10-21): a sweep of exposure
0.7–2 × highlight compression × ACES, compared on the HDRI background:
exposure 1.4, no highlight compression, no ACES fits best. As exported:
181 167 141 vs Iray's 188 161 132 (sRGB, RMSE 7.7 / 255; the earlier fixed
imager fitted to Blender gave 50.4). Only HDRI-lit scenes are calibrated.
`Crush Blacks` has no counterpart and is left out.

## Materials

Every material input (20007) is filled — Octane doesn't convert
`UsdPreviewSurface`.

### Universal (130) — default

- **albedo** = diffuse map (sRGB) with the diffuse colour's mean as the
  image's power (a coloured tint loses its hue), or the colour; × (1 −
  refraction). No diffuse map → a translucency colour map (hair, caps).
- **Weighted base mixing** (`Base Mixing` 2): a real diffuse map is scaled
  to the blend's brightness (`weighted_mix`; 2.8 for a mesh hair) and
  `Glossy Weight` counts as gloss. Not applied to a translucency map standing
  in as albedo (greyed a cap's crown, probe 60).
- roughness (dual-lobe skin: Specular Lobe 1 Roughness), metallic, cutout
  opacity = greyscale map × value or constant; normal / bump maps.
- **No glossy layer, dual lobe, refraction or mirror → `specular` 0** (Iray
  is matte there; Universal's white default glittered velvet, probe 53).
  A `Glossy Color` map tints `specular` (× glossy weight).
- Base Mixing 1 ignores Metallic Weight; a black base under a strong white
  gloss becomes a gloss-coloured metal (mirrors).
- refraction → `transmission` + IOR (`index4`), transparent shadows.
- Iray UV tiling → each texture's transform (scale 1/N); textures **wrap**
  (UDIM tiles).
- **Colour space**: colour maps sRGB, data maps (roughness, opacity,
  metallic, bump, normal) linear, every image node with
  `initialColorSpaceSelected` = 1; the HDRI keeps linear. Why: without the
  attribute data maps were decoded as sRGB — sunglasses nearly clear
  (probes 55–57); with it the lenses tint like Iray's, brightness unchanged
  (Octane2 moved 0.28 / 255).
- **Layered images** are flattened into one PNG per stack.

### Skin — Standard Surface (277), Final only

A material with Iray `Translucency Weight` > 0 that is opaque (no cutout),
not `Thin Walled`, non-refractive, non-emissive and has a diffuse map:

- `baseColor` and `subsurfaceColor` = the diffuse map (Iray's translucency
  map is pale and bleached skin and make-up), `subsurface` = translucency
  weight, `subsurfaceAnisotropy` = `SSS Direction`;
- `radius` = per-channel diffusion length (see [iray.md](../daz/iray.md#translucency-and-subsurface-skin))
  normalised to its maximum and **square-rooted**; `scale` = that maximum in
  meters, clamped 0.5 mm – 2 cm. Why the root: Noemi's 1 / 0.18 / 0.15
  threw coloured fireflies, and Iray's look is softer.

### dForce hair — Hair material (147)

albedo = (mean of root and tip colour, each its map's linear mean × tint)²;
roughness both ways from `base_roughness`; specular = `Glossy Layer Weight`.
Why squared: as-is a brunette rendered 3–5× too light; squared (with the 4×
widths) Octane2's hair matches Iray (0.16/0.14/0.06 vs 0.16/0.12/0.07).

### Emission

Iray emission → Universal `emission`: blackbody (53: power, temperature,
normalize) or, at temperature 0, texture emission (54) tinted by the colour.
Power (W) = L × unit factor / 2 / **15 000** (`DAZ_LUMENS_PER_WATT`) — total
for lm / W, × the material's surface area (m²) for luminance units.

## Lights

Watts at 15 000 lm/W, point lights ×3;
non-photometric lights 15 000 lm × Intensity (an assumption).

- **Point / area** → UsdLux under `/Scene/Lights` (point → `SphereLight`
  r = 1 cm + `treatAsPoint`; rect, disc, sphere by `Light Geometry`; size in
  cm; `colorTemperature`; intensity = watts, `normalize`). Their emitter
  inputs get a black diffuse with blackbody emission (`light_inputs`) —
  Octane's import doesn't make them glow.
- **Spots** → analytic disk lights (402) — UsdLux shaping is lost: disk of
  the emitter size (2 cm for a point source), spread = DAZ `Spread Angle`,
  `normalize`, blackbody. Power = watts × (1 − cos(half angle)) / 2 × 0.75
  (Octane puts all power into the cone; 0.75 = trim against the Blender
  reference). Transform: DAZ rotation order mapped, scale `1 1 -1` (the disk
  shines +Z, a DAZ spot −Z).

## Environment

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

## Calibration workflow

One numbered Lua probe per run (`orbis_probeN.txt` + `orbis_pN_*.png`),
results never overwritten; compare crops of the same pixel boxes against the
DAZ render in linear medians (`scripts/dev/octane-light-fit.py`); judge noise on
the denoised pass. The last probe lives in `scripts/dev/octane-probe.lua`,
light decomposition in `scripts/dev/octane-probe-decompose.lua`. Record each
confirmed finding here (behaviour) or in
[octane/octane-x.md](../octane/octane-x.md) (Octane facts) right away.

## Known gaps

Tracked as GitHub issues (light power, fidelity vs Iray).
