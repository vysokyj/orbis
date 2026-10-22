# USD — the parts an Octane export needs

Reference excerpts from the OpenUSD specifications. How Octane imports USD:
[octane-x.md](octane-x.md#usd-import).

## USDZ container ([spec](https://openusd.org/release/spec_usdz.html))

- zip, **stored (no compression), unencrypted**;
- every file's data starts at a **multiple of 64 bytes** from the archive start
  (pad with the local header's extra field);
- the first file is the default layer (a USD file);
- allowed: `usda` / `usdc` / `usd`, `png`, `jpeg`, `exr`, `avif`, audio.

## Meshes

- `Mesh`: `points`, `faceVertexCounts`, `faceVertexIndices`,
  `subdivisionScheme` (`catmullClark` default, `none`), `normals`.
- `GeomSubset` with `familyName = "materialBind"` per material group, bound
  with `material:binding`.
- UVs: `primvars:st` as `texCoord2f[]`, interpolation `faceVarying`, optionally
  **indexed** (`primvars:st:indices`).
- `BasisCurves`: `type = "linear"`, `curveVertexCounts`, `points`,
  per-vertex `widths`.
- Stage metadata: `upAxis`, `metersPerUnit`, `defaultPrim`.

## UsdPreviewSurface ([spec](https://openusd.org/release/spec_usdpreviewsurface.html))

Inputs (defaults): `diffuseColor` (0.18), `emissiveColor` (0), `metallic` (0),
`roughness` (0.5), `opacity` (1), `opacityThreshold` (0 — `> 0` = cutout),
`ior` (1.5), `normal` (tangent space), `clearcoat`, `clearcoatRoughness`,
`useSpecularWorkflow`. Textures: `UsdUVTexture` (`file`, `st`, `wrapS/T`,
`scale`, `bias`, `sourceColorSpace` `raw|sRGB|auto`, outputs `r g b a rgb`),
`st` from `UsdPrimvarReader_float2` (`varname = "st"`). Normal maps:
`sourceColorSpace = "raw"`, `scale (2,2,2,1)`, `bias (-1,-1,-1,0)`.

## UsdLux

`SphereLight` (`radius`, `treatAsPoint`), `RectLight` (`width`, `height`),
`DiskLight` (`radius`); `intensity`, `normalize`, `colorTemperature` +
`enableColorTemperature`; `ShapingAPI` for cones (lost by Octane's import).
