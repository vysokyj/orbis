# DAZ files — `.duf`, `.dsf`, `.dbz`, headless DAZ Studio

Reference for the DAZ ecosystem: the scene and asset files (DSON) and the
`.dbz` posed-scene export, as far as reading a scene outside DAZ Studio
needs them. Content layout and types: [content.md](content.md). Iray
materials, lights and render settings: [iray.md](iray.md).

## DSON basics

`.duf` (user file: scene, preset) and `.dsf` (support file: `data/` asset) are
JSON, **optionally gzipped** — gzip magic `1F 8B` → inflate, else plain UTF-8.
URLs inside are **percent-encoded and content-relative**
(`/data/DAZ%203D/…`), resolved against the DAZ content directories, may carry
a `scope:` prefix and point at an object with `#fragment`
(`…/Genesis8Female.dsf#Genesis8Female`). Materials can also reference shader
definitions compiled into DAZ Studio (`dzPlasticDef`, `omUberSurfaceDef`,
`omHumanSurfaceDef`…) — no package ships those as files.

DAZ Studio's content directories: `ContentDir0..N` in its macOS preferences.

## `.duf` — the scene

- `scene.nodes[]`: `id`, `name`, `label`, `parent` (`#id`), `conform_target`
  (`#id`, fitted clothing / grafts), `geometries[0]` = geometry instance:
  `id`, `url` (`/data/…/X.dsf#geometry` or `#local-id` into the duf's own
  `geometry_library`), `type` (`polygon_mesh` | `subdivision_surface`),
  `extra[]` (`studio/geometry/shell` marks a geometry shell).
- Node channels (`extra[].channels`, `studio_node_channels`): `Visible`
  false = not rendered (an "Effect Layer" shell often is).
- `scene.materials[]`: `geometry` (`#<geometry instance id>`), `groups`
  (material group names), `uv_set` (URL), `diffuse` + `extra[]`
  (`studio/material/uber_iray`, `studio_material_channels` with Iray Uber
  channels keyed by label, e.g. `"Dual Lobe Specular Reflectivity"`). `url`
  points into the duf's `material_library`.
- `image_library[]`: layered / remapped images (`map[]`, see
  [iray.md](iray.md#layered-images)).
- `scene.current_camera`: `#id` of the active camera node; a scene saved
  through the Perspective View has none.
- Render settings: `studio_render_settings.render_options.imageSize`.
- Saved scene thumbnail: `<scene>.duf.png` next to it.

## `.dsf` — library assets

- `geometry_library[]`: `vertices` (`count` + `values`), `polygon_groups`,
  `polygon_material_groups` (`values`), `polylist` (`values`), `default_uv_set`,
  `graft`.
- polygon: `[face_group, material_group, v0, v1, v2(, v3)]` — triangle or quad.
- `uv_set_library[]`: `vertex_count`, `uvs.values` (`uvs[i]` is vertex `i`'s
  UV; entries past `vertex_count` are per-polygon extras),
  `polygon_vertex_indices`: `[polygon, vertex, uv_index]` overrides — `vertex`
  is a geometry vertex index, not a corner position.
- `graft`: `vertex_count` / `poly_count` of the target (host),
  `vertex_pairs` `[graft_vertex, host_vertex]`, `hidden_polys` = **host**
  polygons hidden while the graft is attached. Only geometry with non-empty
  `vertex_pairs` is a graft.

## How DAZ Studio builds what it renders

### Subdivision
A `subdivision_surface` at resolution level > 0 renders as **one
Catmull-Clark refinement** at SubD 1 — a refined mesh, not the limit surface;
boundaries "edges and corners", UVs split linearly so seams stay sharp.
Smoothing modifiers fit skin-tight unrefined items (hair caps, brows) onto
the **subdivided** figure surface; their faces are straight chords over it.

### Geografts are welded
DAZ welds a graft's border onto its host (`vertex_pairs`) before subdividing.
In a `.dbz` the graft's own border vertices sit up to ~0.85 cm off the host's.

### Geometry shells
A shell's geometry is empty in the duf (`studio/geometry/shell`); DAZ builds
it from its parent's cached geometry **including attached grafts**. Verified
(G8F 16556 verts + genital graft 1840 → shell 18396):

- vertices: host vertices, then each graft's vertices (offset by the shell's
  `Offset`, here 0.005–0.02 cm);
- material groups: host groups, then `<graft node name>_<graft group>`;
- polygons: host polygons without the grafts' `hidden_polys`, then each
  graft's polygons with vertex indices shifted by the host vertex count;
- **visibility**: the shell node's channels `material_group_<group>_vis`
  switch surfaces on (e.g. only `new_gens_V8_1840_Genitalia` / `_Anus`); every
  other group is not rendered. Its other materials are typically opaque white.

### Units and axes
Centimeters, Y up. Euler order `XYZ` = `Rz·Ry·Rx` (X applied first); per node
`rotation_order`.

### Cameras
DAZ fits the frame width (channel `Frame Width`, 36 mm) to the image's
**vertical** extent (DSON `yfov`): vertical FOV v = 2·atan(frame / 2 / focal),
horizontal = 2·atan(tan(v/2) · w/h) — 65 mm in 16:9 is 52.4° horizontally,
not 31°. Depth of field: aperture radius = focal / (2 · f-stop).

### Lights
DSON light nodes (`type: light`; `spot` / `point` / `directional`) are off
when `on` is false, `Visible` is off or `Illumination` is 0. Units and power:
[iray.md](iray.md#lights).

## `.dbz` — posed-scene export

A DAZ Studio export script writes the posed scene as (gzip or plain) JSON next to it:

```json
{ "application": "export_to_blender" | "export_highdef_to_blender" | "export_basic_data",
  "version": "4.5.0.2668",
  "filepath": "<scene>.duf",
  "rootpaths": { "content": ["/…/My DAZ 3D Library", …], "builtin_mdl": …, … },
  "figures": [ {
      "name": "Genesis8Female", "label": "Noemi", "class": "DzFigure",
      "center_point": [x,y,z], "end_point": [x,y,z], "orientation": [x,y,z], "rotation_order": "YXZ",
      "ws_transform": [9 rotation/scale + 3 translation],
      "lod": 1,                       // resolution level (1 = high = subdivided)
      "material groups": ["Torso", …],
      "num verts": 16556,
      "vertices": [[x,y,z], …],       // final mesh: morphs + pose, world space
      "bones": [ … ]                   // per-bone center/end/ws_transform
  }, … ] }
```

- `vertices` come from `obj.getCachedGeom()` at LOD 0 (the base mesh) — world
  space, cm, Y up, every morph and the pose applied. No faces or UVs for
  regular meshes.
- dForce strand hair carries the rendered strands: `polylines`,
  `polyline_materials` per `material groups`. The library mesh is only their
  guide, so its vertex count never matches.
- Entries match scene nodes by `label` (unique per scene), then `name`. A
  `{"name": "dummy", "num verts": 0}` entry closes the list.
- HD export (`num hd verts`, `hd vertices`, `hd faces`, `hd uvs`) is optional.
- `rootpaths.builtin_mdl` points at DAZ Studio's built-in resources
  (`Contents/shaders/iray`, where `/resources/…` paths such as the default HDRI
  `DTHDR-RuinsB-500.hdr` live).

The script ends with `createDialog()` (an options dialog) and a closing
message box; `exportToBlender(path)` does the work.

## Headless DAZ Studio

```
DAZStudio.app/Contents/MacOS/DAZStudio -headless -noPrompt -noDefaultScene \
    <script.dsa> -scriptArg <arg0> -scriptArg <arg1>
```

- In the script: `Scene.loadScene(path, Scene.OpenNew)` loads a scene,
  `App.delayedExit()` quits when done.
- A second default instance clashes with a running DAZ Studio
  (`-instanceName` + copied settings is untested).
- A headless load logs missing content, e.g. morphs a character's formulas
  drive that the scene doesn't reference (`Formula target property not found`).
- A scene load + `.dbz` export took 23–27 s for a single-figure scene.
