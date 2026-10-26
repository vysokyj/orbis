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
  and accepts either count. — world space, cm,
  Y up, morphs and pose applied; flat `x y z`. The export refines it itself
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
  `Line End Width`, at **0.4 mm per unit** (`LINE_WIDTH_SCALE` = 0.004 m).
  Why: read as mm the strands were sparse and see-through; a 1/2/4/10× sweep
  put 4× on Iray's fullness and colour (Octane2).
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
| skin | Universal | Standard Surface + random-walk SSS |
| imager exposure | 1 × 2^(13 − EV) × cm² | **1.4** × 2^(13 − EV) × cm² |
| imager response | **Gamma 2.2** (403) | same |
| highlight compression | 0; Burn < 1, Crush > 0 or Gamma ≠ 2.2 → a tone LUT (below) | same |
| white balance / saturation | DAZ White Point × Scale / Saturation | same |
| denoiser | off | **on**, during the render (`denoiserOnce` off, `maxDenoiserInterval` 20) |

Why path tracing for final: direct lighting renders random-walk SSS grey and
isn't photorealistic; Cycles (the Blender reference) path-traces too.

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
photographic tone mapper's, to be confirmed by Calib-Tone-ISO200 (one stop
brighter than Calib-Tone-Default if right).

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
shadows).

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
- **Roughness** by Base Mixing: 0 → `Glossy Roughness`, 1 and 2 →
  1 − `Glossiness` (both channels are always saved; Calib-Materials:
  Glossiness 0.8 beside Glossy Roughness 0.5 rendered Iray's sharp
  highlight), then **squared** when `Roughness Squared` / `Glossiness
  Squared` (default on) — Iray squares the slider, Octane doesn't:
  highlights went from 31 / 17 / 6 px to 48 / 24 / 22 against Iray's
  56 / 26 / 20 (#10).
- **Top Coat** → Universal `coating` = Top Coat Color × weight,
  `coatingRoughness` (Top Coat Roughness, or 1 − Top Coat Glossiness in
  modes 1 / 2, squared likewise), `coatingIor` = Top Coat IOR: the
  top-coated sphere's highlight 0 → 19 px (Iray 17).
- **Weighted mixing** (Base Mixing 2) without translucency → `metallic` =
  Glossy Weight / (Diffuse Weight + Glossy Weight), `specular` 0, the
  diffuse map unbrightened (`weighted_gloss`): Iray blends a full-strength
  glossy reflection by its share. Highlight 19 → 68 px (Iray 215), overall
  0.81 of Iray either way — open in #10. With translucency (mesh hair) the
  diffuse map is brightened by `weighted_mix` as before.
- **Metallic Weight under 0.05** (no map) is left out: the GSOV4 jacket's
  0.0107 turned Octane's gloss off — black velvet (0.02× Iray) beside the
  same leather's trousers.
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
- **Colour channels** (Base / Glossy / Translucency / Transmitted / Emission
  Color, light colours) are **sRGB as DAZ stores them** and decoded to
  linear, as Iray does. Passed on raw, surfaces rendered 1.5× too bright
  (Calib-HDRI: floor 1.53 → 1.03, cubes 1.77 → 1.00 of Iray; the HDRI
  background 1.04 either way).
- **Layered images** are flattened into one PNG per stack.

### Skin — Standard Surface (277), Final only

A material with Iray `Translucency Weight` > 0 that is opaque (no cutout),
not `Thin Walled`, non-refractive, non-emissive, not metallic (`Metallic
Weight` under 0.05 and no map) and has a diffuse map. Why not metallic: the
Gladio outfit is PBRSkin with translucency 0.5 and a metallic map on every
surface — as skin its metal was lost and the open cloak rendered grey
instead of red (Gladiator). And on a figure (the dump's `class` names a figure): props'
translucency is grass blades and leaves, open cards — Gladiator's grass
rendered grey.

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

**OpenPBR option** (`--skin-openpbr`, the dialog's "Skin as OpenPBR"):
the same skin on OpenPBR Surface (505; its `radius` is the distance and
`scale` the per-channel colour — the other way round from Standard
Surface). It renders the colour darker, evenly per channel and more so with
the translucency weight (G8F 0.78 at w 0.5, Fleur 0.70 at 0.62 of Standard
Surface), so the tint is lifted by 1 / (1 − 0.46 · w)
(`OPENPBR_SKIN_DARKENING`): G8F 1.00 / 1.17 / 1.70 of Iray, Fleur 0.81–0.92
in red. Looks grittier; the hue offset is the same as Standard Surface's.

### dForce hair — Hair material (147)

albedo = (mean of root and tip colour, each its map's linear mean × tint)²;
roughness both ways from `base_roughness`; specular = `Glossy Layer Weight`.
Why squared: as-is a brunette rendered 3–5× too light; squared (with the 4×
widths) Octane2's hair matches Iray (0.16/0.14/0.06 vs 0.16/0.12/0.07).

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
sends its full flux down (1.000) — the export's one-sided light matches the
side it faces; a distant light lights the floor by the sine of its
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

## Environment

- **Environment Mode** "Dome Only" (1) and "Sun-Sky Only" (2): Iray
  ignores the scene's lights — a 2 000 000 lm point light left the floor
  unchanged (Calib-L-Mode-*, 1.000), so the export leaves them out
  (`lights_iray_uses`). Sun-Sky itself (DAZ's procedural sun and sky) has
  no Octane counterpart in the export yet: such a scene keeps its HDRI.
- **Draw Dome** off (the dome lights but isn't seen): Iray shows black
  behind the scene, the HDRI still lighting it (Calib-Dome-Off; the
  Calib-L scenes are "Scene Only", black anyway). The render target gets a
  black texture environment on its `visibleEnvironment` pin with
  `visibleEnvironmentBackplate` on — the camera sees it, lighting and
  reflections keep the HDRI. *Pin name to confirm in Octane* (the batch
  renders Calib-Dome-Off). DAZ's backdrop (`backdrop_visible`) isn't
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
