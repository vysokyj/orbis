# 2026-10-30 — Node render flags and hidden surfaces (#36)

## What the scenes use

A scan of every scene in My Library (node channels, scene nodes and the
scene's own node definitions): `Renderable`, `Cast Shadows` and `Render
Priority` sit at their defaults (true, true, 3) on all 2788 nodes that list
them. `material_group_<g>_vis` and `facet_group_<g>_vis` appear only on the
NGV8 shells (Effect Layer, Color Layer) in 7 scenes: the facet channels all
false (61 / 63 of them), the surface ones true for the two graft surfaces
(`new_gens_V8_1840_Genitalia`, `_Anus`), false for the 16 others.

## Facet groups

`polylist` entries are `[facet group, material group, v…]`; the facet
groups are the geometry's `polygon_groups` (a figure's bone regions). The
NGV8 graft's are `Hip`, `Rectum`, `Vagina` (800 / 272 / 728 polygons).
Both shells switch off the host's `Hip`; the Effect Layer also
`WG_0_Rectum` and `WG_0_Vagina`.

First try: a graft facet group the host also has keeps its name (`Hip`),
the others become `WG_0_<g>`. Calib-Feet-Teens' Color Layer then lost the
graft's 800 Hip polygons (7198 → 4000 faces refined) and the Effect Layer
would have shown nothing of the graft — not what the shells are for. So
every graft facet group is `WG_<k>_<g>`, the graft's Hip polygons stay
visible (the outside) and only the Effect Layer drops the inside. Exports
of Calib-Feet-Teens and Calib-Hair-Teens came out as before (the Effect
Layer is hidden in all the scenes); not checked against Iray.

## Cast Shadows in Octane

Probes 100–102 on Calib-L-Distant-45 (plane, three cubes, a sphere, a
distant light):

- The USD graph's `A_GEOIMP_OBJECT_LAYER_IMPORT` (42, `objectLayerImport`
  in the project) is 1 (hidden: no object layer inputs). At 3 the import
  creates an input per prim, `/Scene/<prim>_object` (type 20017; lights
  `/Scene/Lights/<light>_object`), each through an object layer map (64)
  onto its mesh; at 2 a single "Object layer" input for all.
- The object layer (65) has `shadow_visibility` among its pins; saved from
  Octane, an input is a 20017 node with the layer under its `input` pin.
- The export writes `objectLayerImport` 3 and only the inputs of the prims
  with Cast Shadows off, with only `shadow_visibility` 0. Rendered (200
  samples): the sphere casts no shadow, the cubes' shadows and the light
  are unchanged — the inputs left out get a default layer.

The scene for it was a copy in `_local/` with a `Cast Shadows` false channel
added to the sphere and the cached dump copied under the copy's name.
