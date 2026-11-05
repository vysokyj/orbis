# USD — the parts an Octane export needs

Reference excerpts from the OpenUSD specifications. How Octane imports USD:
[octane-x.md](octane-x.md#usd-import).

## USDZ container ([spec](https://openusd.org/release/spec_usdz.html))

- zip, **stored (no compression), unencrypted**;
- every file's data starts at a **multiple of 64 bytes** from the archive start
  (pad with the local header's extra field);
- the first file is the default layer (a USD file);
- allowed: `usda` / `usdc` / `usd`, `png`, `jpeg`, `exr`, `avif`, audio.

## usdc (crate) layout

From OpenUSD's `pxr/usd/sdf/crateFile.cpp` (version 0.8.0, what Pixar
writes by default; readers take every older version). All little-endian.

- **Header** (88 bytes at 0): `PXR-USDC`, version bytes `0 8 0 0 0 0 0 0`,
  int64 offset of the table of contents, 64 reserved bytes.
- **Values** anywhere after it, each referenced by a 64-bit *ValueRep*:
  bit 63 array, 62 inlined, 61 compressed, bits 48–55 the type
  (`crateDataTypes.h`: Bool 1, Int 3, Float 8, Double 9, String 10, Token
  11, Matrix4d 15, Quath 18, Vec2f 20, Vec3f 24, TokenListOp 32,
  PathListOp 34, TokenVector 41, Specifier 42, Variability 44), low 48 bits
  the value (inlined: bools, ints, floats, a double that is exact as float,
  token / string indices, enums) or its file offset.
  - Arrays: 8-byte aligned, uint64 count, the elements. Int arrays of 16+
    are compressed instead (not aligned): count, uint64 size, the
    compressed ints. Quath stores i, j, k, then the real part; Matrix4d
    row by row.
  - Token vector: uint64 count + uint32 token indices. List op: a flag
    byte (1 explicit, 2 has explicit items, 0x20 has prepended items, …),
    then per present list uint64 count + uint32 indices.
- **Sections** (each listed in the table of contents as a 16-byte name,
  int64 start, int64 size): `TOKENS` (count, raw size, compressed size,
  then the null-terminated tokens compressed), `STRINGS` (uint64 count +
  token index each), `FIELDS` (count, compressed token indices, then the
  ValueReps compressed as bytes), `FIELDSETS` (compressed field indices,
  each set ended by −1), `PATHS` (path count, then three compressed arrays
  of a depth-first walk: path index, element token — negative for a
  property name — and jump: −2 leaf, −1 child only, 0 sibling only, else
  the sibling's distance), `SPECS` (three compressed arrays: path, field
  set, spec type — Attribute 1, Prim 6, PseudoRoot 7, Relationship 8).
- **Compression**: *integer coding* = int32 most common delta, 2-bit code
  per int (common / int8 / int16 / int32 delta from the previous), the
  non-common deltas; then *TfFastCompression* = a byte 0 and one LZ4 block
  (or a chunk count and int32-sized blocks of at most 0x7E000000 bytes).
- **Fields** Sdf writes (as `usdcat -o x.usdc` does): pseudo-root
  `defaultPrim`, `metersPerUnit`, `upAxis`, `documentation`,
  `primChildren`; prim `specifier`, `typeName`, metadata (`displayName`,
  `kind`, `apiSchemas`), `primChildren`, `properties`; attribute `custom`,
  `typeName`, `variability`, `default`, `interpolation`,
  `connectionPaths`; relationship `variability` (uniform) and
  `targetPaths`. No target or connection specs.

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
