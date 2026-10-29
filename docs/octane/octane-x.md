# Octane X — behaviour, project format, scripting

Reference for OctaneRender as shipped in **Octane X** (macOS App Store build
`com.otoy.rndrviewer`, 2026.6 / Octane 15.5), learned by probing it. The
official manual is mirrored in [manual/](manual/README.md); the package
container is in [orbx.md](orbx.md); USD specs in [usd.md](usd.md).

## The app

- **Sandboxed, no command line, no document types.** It refuses every file
  opened from outside — `.usdz` and even its own `.orbx`. Files reach it
  through **File → Open** / its import panel (`fbx;obj;usd;usdz`), the
  gallery, or the Script menu. The sandbox grants access only to the picked
  file, so loose textures next to a `.usda` are unreadable — use one
  container (`.usdz`, `.orbx`).
- **Basic mode** renders a render target selected in the Scene outliner.
- Inside the sandbox `HOME` is
  `~/Library/Containers/com.otoy.rndrviewer/Data`.
- Gallery: see [orbx.md](orbx.md#gallery).
- Speed on an Apple M4 (10 GPU cores, 24 GB): direct lighting ~3.3 Ms/s,
  path tracing ~1.1–1.6 Ms/s on one test scene (2–3× slower per sample).
  User reports range from 1.2× to 8× (emissive-heavy scenes).

## USD import

`octanesdk` imports USD (`usda` / `usdc` / `usdz`), Alembic, FBX and OBJ.

- **Ignores `metersPerUnit`**: points are read as meters (geometry-import
  unit default). A centimeter stage comes in 100× too large.
- **No per-mesh subdivision level**: `subdivisionScheme = "catmullClark"`
  alone leaves meshes at level 0 (the cage). The only working switch is the
  import graph's "subdivide all meshes" (`A_USD_IMP_SUBDIV_ALL_MESHES`),
  which also rounds hard props. `A_SUBD_LEVEL` /
  `A_USD_IMP_OVERRIDE_SUBDIV_LEVEL` have no effect on their own.
- **`UsdPreviewSurface` is not converted** when a project opens: the import
  graph's material inputs (`/Scene/Materials/<name>`, type 20007) stay empty
  — the project must fill them.
- **UsdLux lights** become emitter meshes with a material input
  `/Scene/Lights/<prim>_light` carrying a plain diffuse material — **they
  don't glow** until that input gets an emissive material. **UsdLux shaping
  (spot cones) is lost**.
- `BasisCurves` import as hair geometry (`A_HAIR_THICKNESS` per vertex).
- `PointInstancer` imports as instances: `positions`, `orientations`
  (`quath`, w x y z) and per-axis `scales` all apply, the prototype under it
  isn't drawn on its own (Octane X 2026.6, a cube instanced three times).
- Imported mesh items are type 1; each prim gets a placement
  `<prim>_xform` (type 4).

## Project (`project.ocs`) rules

- Octane renders a render target only when it is **in the Scene graph,
  selected and wired to geometry**; the project-settings "Preview Render
  Target" has no geometry (grey render). Octane offers the render target's
  name when saving a render.
- A graph can't be connected as such: pins connect to its **output linker**
  node (geometry out 1439 / NT_OUT_GEOMETRY in the USD import).
- **Dynamic pins** (a geometry group's `Input N`) must carry their type,
  `dynamicType='12'` (geometry); without it Octane drops the pins and their
  connections and renders nothing.
- Node ids must be unique across copies of a subgraph.
- Lua can't wire a group to the USD import's inner output node
  (`Scene_xform`); the project file can.

## Node types and pins

Seen in samples ([orbx.md](orbx.md#scene-projectocs)) plus:

| Type | Node | Notes |
|---|---|---|
| 3 | geometry group | dynamic `Input N` pins |
| 4 | placement | pins `transform`, `geometry` |
| 53 / 54 | blackbody / texture emission | `power`, `temperature`, `normalize` |
| 56 | render target | |
| 67 | transform value | `rotationOrder`, `rotation` (degrees), `scale`, `translation` (m) |
| 110 | plane (infinite) | pin `groundColor` takes a material |
| 130 | Universal material | `specular` default = full white |
| 145 | shadow catcher | pin `opacity` |
| 147 | Hair material | `hairMode` 0 = albedo, `albedo`, `specular`, `roughness`, `roughnessV`, `ior`, `offset`, `opacity` |
| 163 | OCIO colour space | `namedColorSpace`: 1 sRGB, 2 linear sRGB, 6 linear sRGB + legacy gamma |
| 277 | Standard Surface | built-in random-walk SSS; pins `subsurface` 24, `radius` 26, `scale` 27, `roughness` 8 (= specular roughness) |
| 402 | analytic light | `analyticLightType` QUAD 0, DISK 1, DIRECTIONAL 2, SPHERE 3, TUBE 4; `analyticLightSpreadAngle` (full cone); `normalize` |
| 20007 | material input (USD import) | |

- **Rotation order** names the **matrix product left to right**: `XYZ` =
  Rx·Ry·Rz (Z applied first); enum XYZ 0, XZY 1, YXZ 2, YZX 3, ZXY 4, ZYX 5.
  The reverse of DAZ: DAZ `XYZ` is Octane `ZYX` (5).
- An analytic disk shines along its **+Z**.
- An analytic light puts **all** its power into its cone.
- **Cameras** follow the `fov` pin (horizontal), not `focalLength`.
- **Texture environment**: its image texture's `transform` is a **UV**
  transform there — scale X −1 mirrors, a 3D rotation does nothing; the dome
  turns as a U shift. Camera translation doesn't move it. No HDRI = white
  environment. The manual's way to tilt is a Spherical projection with a 3D
  rotation.

### Image texture colour space

The `colorSpace` pin (node 163) defaults to **linear sRGB**; the `gamma` pin
applies only in "linear sRGB + legacy gamma" ([RGB Image](manual/RGBImage.md)).
The pin alone is not enough: the image node must also say
**`initialColorSpaceSelected` = 1** (Octane's own samples do). Without it
Octane picks the colour space itself on load — sRGB for 8-bit files — and
overrides the pin, so data maps (roughness, opacity, bump, normal) get
decoded as sRGB.

### Materials

- Universal's default `specular` is a full white one: with roughness 0 it
  mirrors every bump of a normal map (velvet glitters).
- Octane's hair `albedo` is the colour after many bounces; it transmits
  light through fibres (TT lobe) — backlit strands glow.
- Standard Surface keeps the diffuse texture as base colour with its
  built-in random walk; a hand-wired random-walk medium (type 146) on a
  Universal rendered grey and grainy.
- **Diverging per-channel SSS radii throw coloured fireflies** (OTOY forum).
- Textures are held **decoded** (a 4096² map = 64 MB as RGBA8 however small
  its JPEG) in unified memory: ~17.8 GB of textures failed to render on a
  24 GB Mac ("(failed)" in the status bar).
- **Material types** ([manual](manual/MaterialsDatabase.md)): the uber
  shaders Universal (130), Standard Surface (277, Arnold's) and the newer
  **OpenPBR Surface** ([manual](manual/OpenPBRSurfaceMaterial.md) — "metals,
  glass, skin, fabrics"); specialised Diffuse, Glossy, Specular (glass,
  water), Metallic, Hair (147), Toon. Mediums for volumes: Absorption,
  Scattering, Random Walk (146), Standard Volume.
- **LiveDB** (OTOY's online library of ready materials, node groups and
  scenes; uploads closed) and **LocalDB** (one's own saved materials), in
  the Scene Outliner: copy a thumbnail, paste into the node graph. Generic
  presets with their own textures — a reference for how OTOY builds e.g.
  skin, not something a DAZ material maps onto.
- **Analytic light** (402): `analyticLightSpreadAngle` (full cone),
  `analyticLightSpreadCutoffHardness` (0: a soft cos θ · cos²(θ · 90°/half)
  falloff, 1: an even cone), `normalize` keeps the flux whatever the spread
  (probe 74). **Spotlight Distribution** (158: `orientation`, `target`,
  `coneAngle`, `hardness`, `normalize`) on a disk's emission `distribution`
  with the spread at 180° rendered black (probe 73).
- **Imager response** (`octane.responseCurveId`): LINEAR 400, sRGB 401,
  GAMMA1_8 402, GAMMA2_2 403, then the film curves (probe 76).
- **USD import emits rect and disk lights along +Z** (UsdLux: −Z).
- **Universal (130) pins** (probe 82): transmission, transmissionType,
  albedo, diffuseBrdf, metallic, edgeTint, specular, brdf, roughness,
  anisotropyTexture, rotation, spread, ior, metallicMode, index, index2,
  index3, hasCaustics, **coating, coatingRoughness, coatingIor**,
  coatingBump, coatingNormal, filmwidth, filmIor, sheen, sheenBrdf,
  sheenRoughness, sheenBump, sheenNormal, dispersion…, medium, opacity,
  fake_shadows, refractionAlpha, bump, bumpHeight, normal, displacement,
  smooth, smoothShadowTerminator, roundEdges, priority, emission, matte,
  customAov…, layer. No `index4` (the export's IOR pin for glass, #9).
- **Daylight environment (14)** (probe 97): `sundir` (float3, default
  node 30 = sun direction; direction *towards* the sun), `turbidity` (2 –
  15, 2.4), `power` (1), `sunIntensity` (1, sun only), `northoffset` (−1 …
  1), `model` (enum 57: 0 Preetham, 1 Octane (default), 2 Nishita,
  3 Hosek-Wilkie), `sky_color`, `sunset_color` (Octane model only),
  `sun_size` (× the real ~0.5°), `groundColor`, `groundStartAngle`,
  `groundBlendAngle` (ground: not Preetham), `texture`,
  `importance_sampling`, `medium`, `mediumRadius`, the visible-environment
  flags. **Sun direction (30)**: `latitude`, `longitude`, `month`, `day`,
  `hour` (local, float), `gmtoffset` (int); its attribute holds the
  normalised direction.
- **OpenPBR Surface (505)**: like Standard Surface but `radius` is a
  float (distance) and `scale` a colour (per channel) — the other way round.

## Kernels

- **Direct lighting** (24) is a fast preview — *"not unbiased and will not
  yield photorealistic results"*; its diffuse GI sits "in between Ambient
  Occlusion and Path Tracing"; no scatter depth. **Random-walk subsurface
  renders grey** in it whatever the radius or scale.
- **Path tracing** (25): random walk needs many scattering events in thin
  skin — the default scatter depth 8 loses colour (128 works, costing only on
  SSS paths). The default `giClamp` 1 000 000 lets single bright indirect
  paths through as fireflies; a low clamp (20) trades a little indirect
  energy for clean images. `giClamp` clamps indirect paths only.
- Pins: `giClamp`, `caustic_blur` (not `causticBlur`), `maxsamples`,
  `maxDiffuseDepth`, `maxGlossyDepth`, `maxScatterDepth`, `staticNoise`,
  `adaptiveSampling`.
- **Denoiser**: `denoiserOnce` on denoises only once the render reaches its
  maximum samples — anything viewed or saved earlier is raw (bright
  speckles). With it off and `maxDenoiserInterval` 20 it runs during the
  render.
- Sampling is deterministic: the same scene renders the same noise at the
  same pixels — a recurring speckle pattern is no evidence of a cause.
- Imager changes after a finished render are **not** applied by
  `octane.render.saveImage` — re-render to measure them.

Sources: docs.otoy.com/standaloneSE/DirectLightingKernel.html,
docs.otoy.com/BlenderH/Blender/TheRenderKernels.htm,
docs.otoy.com/blender/PathTracing.html,
render.otoy.com/forum/viewtopic.php?p=401464 and ?p=373238.

## Lua scripting

Octane X runs Lua scripts from its **Script** menu.

- Script folder: preferences
  (`~/Library/Containers/com.otoy.rndrviewer/Data/Library/OctaneRender/{…}/preferences_v2`,
  attribute `default_script_path`, written by Octane on quit — edit only while
  it is closed). **Script → Rescan script folder** lists the `.lua` files (a
  new file may need an app restart; replacing a listed file works at once).
- A script can write results to `~/Documents/…` (sandbox `HOME`).
- `octane.project.saveAs` fails in the sandbox; File → Save As works.
- Useful calls: `octane.node.create{type=…, graphOwner=…}`, `setAttribute`,
  `getPinInfoIx`, `getAttributeInfoIx`, `connectToIx` (nodes only — a graph via
  `getOutputNodes()`), `octane.project.getSceneGraph():getOwnedItems()`.
- **Values**: pins hold owned value nodes, so `node:setPinValueIx` silently
  does nothing — set `m:getInputNodeIx(ix):setAttribute(octane.A_VALUE, v, true)`
  and read it back.
- **Edits to USD-imported materials need `usdGraph:evaluate()`**, or renders
  ignore them. Material inputs: `usdGraph:getOwnedItems()`, each
  `:getInputNodeIx(1)`. Kernel and imager pins change renders directly.
- A pin the project didn't write holds no owned node (`getInputNodeIx` is
  nil) — such a pin can only be tested by writing it in the project.
- `connectToIx(ix, nil)` doesn't disconnect (needs a node); nodes can't be
  connected across graphs ("Can't connect node …").
- **Hide an object** by moving it: its `<prim>_xform` takes
  `setAttribute(octane.A_TRANSLATION, {x, y + 100, z})`; opacity 0 doesn't work.
- **Rendering**: `octane.render.start{renderTargetNode=rt}` blocks until
  done but ignores its `maxSamples` — set the kernel's
  (`rt:getInputNode(octane.P_KERNEL):setPinValue(octane.P_MAX_SAMPLES, n, true)`).
  Save with `octane.render.saveImage(path, octane.imageSaveFormat.PNG_8)` (raw
  beauty) or `octane.render.saveRenderPass(octane.renderPassId.BEAUTY_DENOISER_OUTPUT, …)`
  (denoised). `octane.render.imageType` doesn't exist.
- **Focus**: the first `octane.render.start` of a project opened with
  `octane.project.load` brings Octane X to the front (rendering it again
  doesn't). Nodes imported into the open project don't:
  `octane.nodegraph.createRootGraph(name):importFromFile(path)`, then
  `octane.project.getSceneGraph():copyFrom(root:getOwnedItems())` — the same
  pixels. Started with `open -g`, Octane X still has to come to the front once
  to leave its simple mode.
- Syntax-check with `luajit -bl`. No log at all = the script never started
  (not run, or Octane hung).
