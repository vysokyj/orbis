# ORBX container format (reverse-engineered)

OTOY publishes no specification. This layout was derived from the
12 sample scenes bundled with Octane X 2026.6
(`/Applications/Octane X.app/Contents/Resources/*.orbx`, 87 KB – 62 MB,
Octane versions `12000020` and `12000101`). A writer following it reproduces
**all 12 samples byte for byte** from their extracted files.

## Why

Octane X (App Store, sandboxed, no CLI, no document types) refuses every file
opened from outside — `.usdz` and even its own `.orbx`. Scenes reach it only
through its own import panel (`fbx;obj;usd;usdz`) or its gallery, which lists
`.orbx` scenes (see "Gallery" below). A native ORBX also carries Octane's own
materials (Universal material, SSS, …) instead of `UsdPreviewSurface`.

## Layout

```
header   0e 02 "OX"  12 01  1a 1d 61  5e 0e "octane_version"  66 <ulen> "<version>"  00
chunks   ( 3f <ulen> <bytes> )*                     ← file contents, in write order
index    see below
footer   <u32 LE: index length>  04 11 5a 9f
index    (again, byte-identical)
footer   (again)
```

- `version` is the Octane version as decimal text, e.g. `12000101`.
- Each file is split into chunks of at most 1 MiB (`project.ocs` is written as
  a single chunk, even above 1 MiB). Chunks of one file are consecutive.

### Tags

A tag byte is `field << 4 | type`. Types seen: `e`/`6` string (`<ulen>` +
bytes), `f` blob (`<ulen>` + bytes), `3`/`b` signed integer (`<sint>`),
`9`/`1` container start, `0` end, `2`/`a` fixed constants in the header. The
writer only needs the exact sequences below.

### Integers

Big-endian, length from the high bits of the first byte (like UTF-8):

| first byte | bytes | payload bits |
|---|---|---|
| `0xxxxxxx` | 1 | 7 |
| `10xxxxxx` | 2 | 14 |
| `110xxxxx` | 3 | 21 |
| `1110xxxx` | 4 | 28 |
| `11110xxx` | 5 | 35 |

- **Unsigned** (`<ulen>`: blob/string lengths) use the full payload width:
  `2^20` → `d0 00 00`.
- **Signed** (`<sint>`: every `x3`/`xb` integer in the index) keep a sign
  bit, so a form holds `< 2^(bits-1)`: `64` → `80 40`, `2^20` → `e0 10 00 00`.
- 5-byte form `11110xxx` + 4 bytes (35 bits) for larger values — not in the
  samples, but Octane X opened a 258 MB package written with it (verified
  2026-10-20).

### Index

```
79 79  1e 04 "file"
  3b <sint chunk count>
  43 <sint total bytes of all chunks>
  00 00
  71 71
    ( 33 <sint offset delta>  53 <sint chunk size> )*   ← per chunk
  00 00
  69 69
    ( 23 <sint first chunk>  33 <sint file size>  3e <ulen> "<path>" )*   ← per file, sorted by path
  00 00
```

- Offset delta: `0` for the first chunk, then the previous chunk's
  `size + 1 + len(<ulen size>)` — the distance between consecutive `3f` tags.
  Offsets are relative to the first chunk tag, which directly follows the
  header.
- Paths: `project.ocs` (the scene) and `assets/<file>` (OBJ, JPG/PNG, VDB, IES,
  Lua, …), referenced from the OCS as `assets/<file>`.

## Scene (`project.ocs`)

XML, `<OCS2 version='<octane version>'>`: nested `<graph>`/`<node id type
name position>` with `<attr name type>` values and `<pin name>` connections
(inline nodes or links). Node types seen in the samples:

| type | node |
|---|---|
| 1 | mesh (`filename` → `assets/x.obj`; one material pin per `usemtl`) |
| 3 | geometry group |
| 4 | placement |
| 13 | thin lens camera |
| 14 / 37 | daylight / texture environment |
| 16 / 17 / 18 / 120 / 130 | glossy / diffuse / specular / metallic / **universal** material |
| 31 / 33 | float texture / RGB spectrum |
| 34 / 36 | RGB / float (greyscale) image texture |
| 56 | render target |
| 61 / 100 | post processing / film settings |
| 25 / 24 | path tracing / direct lighting kernel |
| 70 | project settings |
| 30007 / 30012 | graph output linkers (material / geometry out) |

Dynamic pins (e.g. a geometry group's `Input N`) carry their type:
`<pin name='Input 1' dynamicType='12' connect='…'/>` (12 = geometry). Without
`dynamicType` Octane 15 drops the pin and its connection.

## Gallery

The gallery lists `OtoyDemoScene` objects from the `demoScenes` preference
(`~/Library/Containers/com.otoy.rndrviewer/Data/Library/Preferences/com.otoy.rndrviewer.plist`,
an `NSKeyedArchiver` array): `caption`, `url` (e.g. `cornell.orbx`),
`renderTarget` (render target node name, e.g. `PT kernel`), `thumbnail`
(e.g. `011.png`). The bundled entries resolve against the app's Resources.
